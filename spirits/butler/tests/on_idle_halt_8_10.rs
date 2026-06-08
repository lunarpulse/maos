#![forbid(unsafe_code)]

//! Story 8.10 AC1 — the anti-8.1 test.
//!
//! Drives Butler's `on_idle` through the SDK vtable harness with a real
//! kernel-orchestrator-backed [`EpistemicScalarPort`] and asserts the FULL
//! honest set: the produced `HaltReceipt`'s trigger scalar **== Butler's own
//! assessed `primary_scalar()` value** (not test-supplied); exactly one
//! `EpistemicHalt` frame via `maos_audit::query`; a `LifecycleEvent::Halt` on
//! the real lifecycle journal; and the anti-bypass — the port is invoked
//! **exactly once with Butler's assessed value**, proving the link
//! `assessment → on_idle → port → halt` that Story 8.1 never tested.
//!
//! The adapter wraps the REAL `WorkingMemoryOrchestrator::process_scalar_write`
//! (a dev-dep newtype per the orphan rule) and carries **zero halt logic of its
//! own** — no canned `HaltReceipt`. The halt DECISION is computed by production
//! kernel logic; only the INJECTION (calling `on_idle` from the test) is
//! test-side. A mock-returning adapter would fail this AC.
//!
//! ## Revert-to-red (AC1 f)
//!
//! Reverting `Butler::on_idle` to store-only (deleting the `scalar_port` call)
//! turns the `on_idle_*` assertions below RED: with no port invocation there is
//! no `HaltReceipt`, no `EpistemicHalt` frame, and no `LifecycleEvent::Halt`.
//! The named reviewer must run that revert and sign off (a green-by-inspection
//! sign-off — the Story 8.1 failure mode — does NOT close AC1).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use butler::{Butler, CalendarEvent, EventStatus, ScenarioInput, BUTLER_SPIRIT_ID, BUTLER_SPIRIT_PID};
use butler::__maos_spirit_vtable_Butler;

use maos_domain::halt::HaltReceipt;
use maos_domain::ports::crypto::CryptoProvider;
use maos_domain::ports::{EpistemicScalarPort, ScalarPortError};

use maos_kernel_core::capability::cap_policy::PolicyTable;
use maos_kernel_core::capability::cap_tokens::Ed25519SigningKey;
use maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator;
use maos_kernel_core::capability::{CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::halt::HaltRegistry;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::security::manifest::EpistemicPolicySection;
use maos_kernel_core::security::RingCryptoProvider;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;

use maos_spirit_sdk::spirit_test::SpiritTest;

const MANIFEST: &str = include_str!("../manifest.toml");
const BOOT_NONCE: u64 = 0x_B17_1E5;

// ── Butler's manifest epistemic policy → kernel EpistemicPolicySection ───────

fn butler_policy() -> EpistemicPolicySection {
    let v: toml::Value = toml::from_str(MANIFEST).unwrap();
    let ep = v.get("epistemic_policy").expect("[epistemic_policy] present");
    let ep_str = toml::to_string(ep).unwrap();
    EpistemicPolicySection::from_toml_str(&ep_str).expect("Butler [epistemic_policy] must parse")
}

// ── the REAL-orchestrator-backed adapter (AC1 c) — zero halt logic ───────────

/// A dev-dep newtype delegating to the REAL `WorkingMemoryOrchestrator`. It
/// carries no halt logic and no canned receipt: the halt DECISION is the
/// kernel's; the adapter only forwards Butler's assessed scalar and records the
/// call so the anti-bypass assertion can verify Butler's OWN value reached it.
struct ButlerOrchestratorAdapter {
    orchestrator: Arc<WorkingMemoryOrchestrator>,
    tl: Arc<TransparencyLogAdapter>,
    journal: Arc<JournalAdapter>,
    policy: EpistemicPolicySection,
    boot_nonce: u64,
    /// (tag, value) recorded per `write_scalar` call (anti-bypass evidence).
    calls: Arc<Mutex<Vec<(String, f64)>>>,
}

impl EpistemicScalarPort for ButlerOrchestratorAdapter {
    fn write_scalar(
        &self,
        spirit_pid: u32,
        spirit_id: &str,
        tag: &str,
        value: f64,
        derived_from: &str,
    ) -> Result<Option<HaltReceipt>, ScalarPortError> {
        self.calls
            .lock()
            .unwrap()
            .push((tag.to_string(), value));
        self.orchestrator
            .process_scalar_write(
                &self.tl,
                &self.journal,
                spirit_pid,
                spirit_id,
                self.boot_nonce,
                tag,
                value,
                derived_from,
                &self.policy,
            )
            .map_err(|e| ScalarPortError::Backend(e.to_string()))
    }
}

struct World {
    tl: Arc<TransparencyLogAdapter>,
    journal: Arc<JournalAdapter>,
    orchestrator: Arc<WorkingMemoryOrchestrator>,
    audit_db: PathBuf,
    _tmp: tempfile::TempDir,
}

/// File-backed world (so `maos_audit::query` can read the Transparency Log),
/// otherwise the same construction as `corpus_halt.rs:188-202`.
fn make_world() -> World {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(RingCryptoProvider);
    let signing_key = Ed25519SigningKey::new([0u8; 32]);
    let policy_table = Arc::new(PolicyTable::new());
    let (audit_tx, _audit_rx) = maos_kernel_core::capability::cap_audit::channel();
    let quota = maos_kernel_core::capability::cap_quota::CapQuotaTracker::new();
    let telemetry = Arc::new(TelemetryStreamAdapter::default());
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        crypto,
        signing_key,
        BOOT_NONCE,
        policy_table,
        audit_tx,
        quota,
        Arc::new(WorkingMemoryStore::new()),
        telemetry,
    ));
    let orchestrator = Arc::new(WorkingMemoryOrchestrator::new(
        Arc::clone(&capability),
        Arc::new(HaltRegistry::new()),
    ));

    let tmp = tempfile::TempDir::new().unwrap();
    let audit_db = tmp.path().join("audit.db");
    let journal = Arc::new(JournalAdapter::open(&tmp.path().join("journal.ndjson")).unwrap());
    let tl = Arc::new(TransparencyLogAdapter::open(&audit_db, BOOT_NONCE).unwrap());
    World {
        tl,
        journal,
        orchestrator,
        audit_db,
        _tmp: tmp,
    }
}

/// A calendar-conflict scenario (one confirmed overlap ⇒ belief_variance ≥ 0.75).
fn calendar_conflict_scenario() -> ScenarioInput {
    ScenarioInput {
        calendar: vec![
            CalendarEvent {
                id: "a".into(),
                title: "Standup".into(),
                start_min: 540,
                end_min: 600,
                status: EventStatus::Confirmed,
            },
            CalendarEvent {
                id: "b".into(),
                title: "Board call".into(),
                start_min: 570,
                end_min: 630,
                status: EventStatus::Confirmed,
            },
        ],
        ..Default::default()
    }
}

fn drive_on_idle(spirit: &Butler) {
    let vtable = __maos_spirit_vtable_Butler();
    let mut harness = SpiritTest::new(spirit, &vtable);
    harness.fixture_mut().invoke_on_idle = true;
    let report = harness.run();
    assert_eq!(
        report.base.hooks_fired.get("on_idle").copied().unwrap_or(0),
        1,
        "on_idle must fire exactly once"
    );
}

#[test]
fn on_idle_fires_the_real_halt_through_the_scalar_port() {
    let scenario = calendar_conflict_scenario();

    // Butler's OWN assessed scalar (recomputed deterministically — NOT supplied
    // to the port by the test).
    let assessed = Butler::new().assess(&scenario).primary_scalar();
    let (assessed_tag, assessed_value, _) = assessed;
    assert_eq!(assessed_tag, "belief_variance");
    assert!(assessed_value >= 0.75, "one conflict crosses the halt floor");

    let world = make_world();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let adapter = ButlerOrchestratorAdapter {
        orchestrator: Arc::clone(&world.orchestrator),
        tl: Arc::clone(&world.tl),
        journal: Arc::clone(&world.journal),
        policy: butler_policy(),
        boot_nonce: BOOT_NONCE,
        calls: Arc::clone(&calls),
    };

    let butler = Butler::with_scenario(scenario.clone())
        .with_scalar_port(Arc::new(adapter) as Arc<dyn EpistemicScalarPort>);

    drive_on_idle(&butler);

    // (i) — a real HaltReceipt was produced, and its trigger scalar is Butler's
    // OWN assessed value (the port received exactly that value).
    let receipt = butler
        .last_halt_receipt()
        .expect("on_idle must fire the real halt and surface a HaltReceipt");
    assert_eq!(receipt.spirit_pid, BUTLER_SPIRIT_PID);
    assert!(receipt.terminal_state.is_none(), "invocation-time receipt");

    // (iv) anti-bypass — the port was invoked EXACTLY once, with Butler's
    // assessed value (proves assessment → on_idle → port, not a smuggled value).
    let recorded = calls.lock().unwrap().clone();
    assert_eq!(recorded.len(), 1, "port invoked exactly once");
    assert_eq!(recorded[0].0, "belief_variance");
    assert_eq!(
        recorded[0].1, assessed_value,
        "the port received Butler's OWN assessed scalar, not a test-supplied one"
    );

    // (i, cont.) — the journaled EpistemicHalt frame carries that same value.
    let frames = world
        .tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::EpistemicHalt),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(frames.len(), 1, "exactly one EpistemicHalt frame journaled");
    let payload: serde_json::Value =
        serde_json::from_slice(&frames[0].payload_redacted).expect("halt payload is JSON");
    let frame_value = payload
        .get("value")
        .and_then(|v| v.as_f64())
        .expect("EpistemicHalt payload carries the trigger value");
    assert!(
        (frame_value - assessed_value).abs() < 1e-6,
        "journaled trigger scalar {frame_value} == Butler's assessed value {assessed_value}"
    );

    // (ii) — maos_audit::query shows exactly one EpistemicHalt frame for Butler.
    let entries = maos_audit::query(
        &world.audit_db,
        maos_audit::AuditFilter {
            spirit_pid: Some(BUTLER_SPIRIT_PID),
            kind: Some("epistemic.halt".to_string()),
            ..Default::default()
        },
    )
    .expect("audit query");
    assert_eq!(
        entries.len(),
        1,
        "maos_audit::query: exactly one EpistemicHalt frame for Butler's id"
    );

    // (iii) — a LifecycleEvent::Halt is recorded on the REAL lifecycle journal.
    let last = world
        .journal
        .last_event(butler::BUTLER_SPIRIT_ID)
        .expect("a lifecycle event for butler");
    assert_eq!(last, maos_domain::invariants::i10::LifecycleEvent::Halt);
}

#[test]
fn negative_control_no_port_stores_assessment_but_fires_nothing() {
    // AC1(e) — with port = None, on_idle stores the SAME assessment and produces
    // ZERO receipts/frames/Halt events. Proves the firing path is real, not
    // incidental to the assessment store. Creates a kernel World (TL + journal)
    // but does NOT wire it to Butler — so any side effects must be absent.
    let scenario = calendar_conflict_scenario();
    let butler = Butler::with_scenario(scenario.clone()); // no scalar_port
    let world = make_world(); // kernel world exists but is NOT wired to Butler

    drive_on_idle(&butler);

    // Same assessment is stored…
    let assessment = butler
        .last_assessment()
        .expect("on_idle stores the assessment even with no port");
    let (tag, value, _) = assessment.primary_scalar();
    let (etag, evalue, _) = Butler::new().assess(&scenario).primary_scalar();
    assert_eq!(tag, etag);
    assert_eq!(value, evalue);

    // (i) — no HaltReceipt surfaced.
    assert!(
        butler.last_halt_receipt().is_none(),
        "no HaltReceipt without a wired scalar port"
    );

    // (ii) — zero EpistemicHalt frames in the Transparency Log.
    let frames = world
        .tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::EpistemicHalt),
            ..Default::default()
        })
        .expect("TL query succeeds");
    assert!(
        frames.is_empty(),
        "zero EpistemicHalt frames in TL without a port — got {}",
        frames.len()
    );

    // (iii) — no LifecycleEvent::Halt on the journal.
    let journal_result = world.journal.last_event(BUTLER_SPIRIT_ID);
    assert!(
        journal_result.is_none(),
        "no LifecycleEvent::Halt on the journal without a port"
    );
}
