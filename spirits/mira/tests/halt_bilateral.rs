//! AC4 — a halt fires on Mira → routes to the **mobile-push surface** (test-double
//! channel) + **Nash informed via A2A typed-intent consent** + the director
//! **resolves via the existing three-tap `HaltFlow` + `KernelHaltResolver`**, all
//! over REAL adapters as dev-dependencies (Decision D/E).
//!
//! - The halt fires through the real `invoke_halt` (TL `EpistemicHalt` row +
//!   lifecycle journal + pending-registry insert). Its payload is the one Mira
//!   raises at its diagnostic-confidence boundary (`mira::Mira::halt_payload`).
//! - The real `NotificationDispatcher` fans `NotificationEvent::Halt` to a
//!   test-double `NotificationChannel` whose `surface() == MobilePush` (Decision
//!   D — the real `MobilePushChannel` is the §6.5 `unimplemented!` stub; only the
//!   terminal push transport is fixture-replaced, the dispatch path is real).
//! - Nash (Host B) is informed via an A2A `readonly` advisory over the real
//!   `LoopbackA2ARouter` (positive); a non-allowlisted intent is denied (negative).
//! - The director resolves via `HaltFlow::resolve_flow` (Tap1→Tap2→Tap3→Done) then
//!   `submit_resolution(halt_id, Resolution::AcceptedHalt, "mira")` →
//!   `KernelHaltResolver::resolve`, journaled against the real `TransparencyLogAdapter`
//!   (which implements `HaltJournal`).

use std::sync::{Arc, Mutex};

use maos_a2a::{
    A2APeerConfig, A2APeerRouter as LocalRouter, A2AProfile, ConsentAllowlists,
    InMemoryTofuPinStore, LoopbackA2ARouter, PeerCertFingerprint, PeerId, TofuPinStore,
};
use maos_a2a::error::A2AError;
use maos_director_surface::halt_ui::{FlowState, HaltFlow, TapEvent};
use maos_director_surface::notification::{
    NotificationChannel, NotificationDispatcher, NotificationError,
};
use maos_domain::frame::{
    EpistemicHaltPayload, FrameAddress, FramePayload, IacFrame, PosturePreferences,
    TaskAssignPayload,
};
use maos_domain::halt::{HaltId, Resolution};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::notification::{NotificationEvent, NotificationLevel, NotificationSurface};
use maos_kernel_core::halt::KernelHaltResolver;
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId, SpiritRole};
use mira::{AnomalySignal, Mira, ADVISORY_CONSENT_INTENT};
use nash::Nash;
use smallvec::smallvec;

const SCENARIOS: &str = include_str!("fixtures/diagnostic-scenarios.json");
const BOOT_NONCE: u64 = 0xB0_07;
const MIRA_PID: u32 = 4242;

/// A test-double mobile-push channel (Decision D): its `surface()` is `MobilePush`
/// and it CAPTURES the dispatched events. The real `MobilePushChannel` is the §6.5
/// `unimplemented!` stub — this proves the halt notification ROUTES to the
/// mobile-push surface without the live gateway transport.
struct MobilePushCapture {
    captured: Arc<Mutex<Vec<NotificationEvent>>>,
}

impl NotificationChannel for MobilePushCapture {
    fn surface(&self) -> NotificationSurface {
        NotificationSurface::MobilePush
    }
    fn dispatch(
        &self,
        event: &NotificationEvent,
        _level: NotificationLevel,
    ) -> Result<(), NotificationError> {
        self.captured
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(event.clone());
        Ok(())
    }
}

/// A fully-wired halt loop over REAL kernel + director-surface adapters.
struct HaltHarness {
    flow: HaltFlow<KernelHaltResolver>,
    captured: Arc<Mutex<Vec<NotificationEvent>>>,
    registry: Arc<maos_kernel_core::halt::HaltRegistry>,
    tl: Arc<maos_kernel_core::iac::TransparencyLogAdapter>,
    journal: maos_kernel_core::journal::JournalAdapter,
    _tmp: tempfile::TempDir,
}

fn build_halt_harness() -> HaltHarness {
    // NOTE: init_monotonic_base() mutates global process state. This is a
    // pre-existing pattern across the entire test suite (110+ call sites).
    // Isolation would require a test-process-per-test or a resettable base.
    // See deferred-work.md: "Global monotonic base initializer called without
    // test isolation" for tracking.
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();

    let tmp = tempfile::TempDir::new().expect("tempdir");
    let db_path = tmp.path().join("audit.db");
    let memory_root = tmp.path().join("memory");
    let journal_path = tmp.path().join("journal");

    let tl = Arc::new(maos_kernel_core::iac::TransparencyLogAdapter::open_in_memory(
        BOOT_NONCE,
    ));
    let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
    let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());

    let capability = Arc::new(maos_kernel_core::capability::CapabilityRegistryAdapter::new(
        Arc::new(maos_kernel_core::api::RingCryptoProvider),
        maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0u8; 32]),
        BOOT_NONCE,
        Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new()),
        maos_kernel_core::capability::cap_audit::channel().0,
        maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
        Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
        Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
    ));
    let orchestrator = Arc::new(
        maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator::new(
            Arc::clone(&capability),
            Arc::clone(&halt_registry),
        ),
    );
    let mailbox = Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&metrics)));
    let memory = Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
        Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
            memory_root,
            4,
        )),
        Arc::new(maos_kernel_core::memory::shared::SharedMemoryStore::open(&db_path).unwrap()),
        Arc::new(
            maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&db_path).unwrap(),
        ),
        Arc::clone(&tl),
    ));
    let output_markers = Arc::new(maos_kernel_core::halt::OutputMarkerRegistry::new());
    let resolver = Arc::new(KernelHaltResolver::new(
        Arc::clone(&halt_registry),
        Arc::clone(&tl),
        output_markers,
        mailbox,
        BOOT_NONCE,
        memory,
        orchestrator,
    ));

    let captured = Arc::new(Mutex::new(Vec::new()));
    let mut dispatcher = NotificationDispatcher::new();
    dispatcher.register(Box::new(MobilePushCapture {
        captured: Arc::clone(&captured),
    }));

    let journal =
        maos_kernel_core::journal::JournalAdapter::open(&journal_path).expect("journal opens");

    let flow = HaltFlow::new(
        resolver,
        Arc::new(dispatcher),
        Arc::clone(&tl) as Arc<dyn maos_domain::halt::HaltJournal>,
    );

    HaltHarness {
        flow,
        captured,
        registry: halt_registry,
        tl,
        journal,
        _tmp: tmp,
    }
}

fn mira_halt_payload() -> EpistemicHaltPayload {
    let signals: Vec<AnomalySignal> =
        serde_json::from_str(SCENARIOS).expect("diagnostic scenarios parse");
    let mira = Mira::default();
    let diag = mira.diagnose(&signals[1]); // unknown-severe → halt boundary
    assert!(diag.requires_halt, "unknown-severe scenario must halt");
    mira.halt_payload(&diag).expect("halt payload at boundary")
}

#[test]
fn halt_on_mira_routes_to_mobile_push_and_resolves_via_three_tap() {
    let h = build_halt_harness();
    let payload = mira_halt_payload();
    let halt_id = HaltId::new(payload.halt_id.clone()).expect("halt id");

    // ── A halt fires on Mira (real invoke_halt: TL row + journal + registry) ──
    let receipt = maos_kernel_core::halt::invoke_halt(
        &h.tl,
        &h.journal,
        &h.registry,
        payload.clone(),
        MIRA_PID,
        "mira",
        BOOT_NONCE,
    )
    .expect("halt fires on Mira");
    assert_eq!(receipt.halt_id, halt_id);

    // ── The halt notification ROUTES to the mobile-push surface ──
    let report = h
        .flow
        .dispatch_halt(halt_id.clone(), payload.clone())
        .expect("halt dispatched");
    assert_eq!(report.delivered, 1, "the mobile-push channel received it");
    assert_eq!(report.errors, 0);
    let captured = h.captured.lock().unwrap();
    assert_eq!(captured.len(), 1, "exactly one Halt event captured");
    assert!(
        matches!(&captured[0], NotificationEvent::Halt { payload: p } if p.tag == payload.tag),
        "captured a Halt event on the MobilePush surface"
    );
    drop(captured);

    // ── Director three-tap (pure total state machine) ──
    let s0 = FlowState::Tap1Acknowledge;
    let s1 = HaltFlow::<KernelHaltResolver>::resolve_flow(s0, TapEvent::Acknowledge);
    let s2 = HaltFlow::<KernelHaltResolver>::resolve_flow(s1, TapEvent::SelectKind);
    let s3 = HaltFlow::<KernelHaltResolver>::resolve_flow(s2, TapEvent::Submit);
    assert_eq!(s3, FlowState::Done, "three taps reach Done");

    // ── Resolve via KernelHaltResolver + journal (no new mechanism) ──
    h.flow
        .submit_resolution(halt_id.clone(), Resolution::AcceptedHalt, "mira")
        .expect("resolution submitted + journaled");

    // The halt is now terminal — re-resolving fails (registry transitioned).
    let again = h
        .flow
        .submit_resolution(halt_id, Resolution::AcceptedHalt, "mira");
    assert!(
        again.is_err(),
        "an already-resolved halt cannot be resolved twice"
    );
}

// ── Nash informed via A2A consent during the halt journey (Decision E) ──

fn advisory_frame(intent: IntentClass) -> IacFrame {
    let signals: Vec<AnomalySignal> = serde_json::from_str(SCENARIOS).unwrap();
    let mira = Mira::default();
    let diag = mira.diagnose(&signals[1]);
    let advisory_json = serde_json::to_string(&mira.advisory(&diag)).unwrap();
    IacFrame {
        frame_id: [9u8; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: FrameAddress {
            spirit_id: SpiritId::from("mira"),
            host_id: Some(HostId("host_a".into())),
            role: Some(SpiritRole::Worker),
        },
        to: smallvec![FrameAddress {
            spirit_id: SpiritId::from("nash"),
            host_id: Some(HostId("host_b".into())),
            role: Some(SpiritRole::Worker),
        }],
        kind: FrameKind::TaskAssign,
        intent,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: advisory_json,
            scope: vec![],
            success_criteria: "architect a mitigation".into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}

async fn pinned_router(accept_a: &[&str]) -> Arc<LoopbackA2ARouter> {
    let fa = PeerCertFingerprint::from_cert_der(b"mira-host-a-cert-v1");
    let fb = PeerCertFingerprint::from_cert_der(b"nash-host-b-cert-v1");
    let cfg_a = A2APeerConfig {
        peer_id: PeerId::new("host_a"),
        endpoint: "tls://127.0.0.1:7443".into(),
        cert_fingerprint: fa.clone(),
        profile: A2AProfile::Loopback,
        allowlists: ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new(ADVISORY_CONSENT_INTENT)],
            accept_allowlist: accept_a.iter().map(|s| A2AIntent::new(*s)).collect(),
        },
        partition_timeout_secs: 30,
    };
    let cfg_b = A2APeerConfig {
        peer_id: PeerId::new("host_b"),
        endpoint: "tls://127.0.0.1:7444".into(),
        cert_fingerprint: fb.clone(),
        profile: A2AProfile::Loopback,
        allowlists: ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new(ADVISORY_CONSENT_INTENT)],
            accept_allowlist: vec![A2AIntent::new(ADVISORY_CONSENT_INTENT)],
        },
        partition_timeout_secs: 30,
    };
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    tofu.pin_first_contact(&PeerId::new("host_a"), &fa, &fa, 1)
        .await
        .unwrap();
    tofu.pin_first_contact(&PeerId::new("host_b"), &fb, &fb, 1)
        .await
        .unwrap();
    Arc::new(LoopbackA2ARouter::new(vec![cfg_a, cfg_b], tofu))
}

#[tokio::test]
async fn nash_informed_via_consent_and_non_allowlisted_intent_denied() {
    // Positive — Nash accepts the readonly advisory and architects.
    let router = pinned_router(&[ADVISORY_CONSENT_INTENT]).await;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    router.install_intake_sink(tx).await;
    LocalRouter::route_outbound(&*router, advisory_frame(IntentClass::Readonly), &HostId("host_b".into()))
        .await
        .expect("Nash informed via consent");
    let delivered = rx.recv().await.expect("advisory delivered to Nash");
    let goal = match &delivered.payload {
        FramePayload::TaskAssign(t) => t.goal.clone(),
        other => panic!("unexpected payload {other:?}"),
    };
    let proposal = Nash::default()
        .architect(&Nash::from_wire(&goal).expect("advisory off wire"));
    assert_eq!(proposal.subject, "edge-cache");

    // Negative — Nash refuses an intent not in its accept_allowlist (EIntentDenied).
    let deny_router = pinned_router(&[]).await; // host_a accepts nothing
    let err = LocalRouter::route_outbound(
        &*deny_router,
        advisory_frame(IntentClass::Readonly),
        &HostId("host_b".into()),
    )
    .await
    .expect_err("non-allowlisted intent denied");
    assert!(
        matches!(err, A2AError::IntentDeniedAtPeer { .. }),
        "expected IntentDeniedAtPeer, got {err:?}"
    );
}
