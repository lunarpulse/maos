#![forbid(unsafe_code)]

//! Story 20-1b AC11 (D-20-1-02 A, KQ1) — FR13 quarantine is a capability-mediated
//! fence and never kills. It is NOT isolation (operator ruling `q`, 2026-11-12).
//!
//! A fixture Spirit declaring `[on_revocation] action = "quarantine"` receives a
//! CRL. From then on every capability-mediated action of its pid (MCP, inference,
//! `MemWrite` token issuance, token verification) fails closed with
//! `CapError::Revoked`, the Spirit stays loaded and Running past the old drain
//! deadline, a director pause -> resume cannot lift the fence, and unload ends it.
//! A bystander the CRL does not name is untouched. Private/shared memory writes,
//! scalar mutation, DRR/native IAC and IO, hooks and telemetry are not fenced here.
//!
//! Run: `cargo nextest run -p maos-kernel-core --test quarantine_fr13_20_1`

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::capability::CapError;
use maos_domain::ports::inference::{
    InferenceError, InferenceOptions, InferenceRequest, InferenceResponse, ProviderAttribution,
    StopReason, TokenUsage,
};
use maos_domain::ports::mcp::{
    McpAttribution, McpCallResponse, McpClientPort, McpError, McpTransportId,
};
use maos_domain::ports::InferencePort;
use maos_domain::revocation::{
    CrlId, RevocationAction, RevocationEntry, RevocationOrigin, SignedRevocationList,
};
use maos_kernel_core::capability::cap_policy::decision::TrustTier;
use maos_kernel_core::capability::cap_policy::{
    ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::{
    cap_audit, cap_quota::CapQuotaTracker, cap_tokens::Ed25519SigningKey,
    CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::iac::{
    transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter},
    IacBusAdapter, Mailbox,
};
use maos_kernel_core::inference::router::MultiProviderRouter;
use maos_kernel_core::inference::InferencePortAdapter;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::mcp::McpClientAdapter;
use maos_kernel_core::memory::{
    principal::PrincipalNamespaceIndex, private::PrivateMemoryStore, shared::SharedMemoryStore,
    MemoryManagerAdapter,
};
use maos_kernel_core::revocation::RevocationApplier;
use maos_kernel_core::scheduler::{
    control_block::{ScbLifecycleState, SpiritManifestBundle},
    SpiritSchedulerAdapter,
};
use maos_kernel_core::security::{
    manifest::{ClassSection, OnRevocationSection, SupervisionSection},
    RingCryptoProvider,
};
use maos_kernel_core::telemetry::{iac_rt::IacRtMetrics, TelemetryStreamAdapter};
use maos_providers::provider::{Provider, ProviderError};

const PROVIDER: &str = "count-stub";
const SERVER: &str = "count-stub";
const TOOL: &str = "write";

struct TestSpirit;
impl maos_spirit_abi::lifecycle::Spirit for TestSpirit {}

/// Counts the calls that actually reach the effect.
#[derive(Default)]
struct Effects {
    inference: AtomicUsize,
    mcp: AtomicUsize,
}

struct CountingProvider(Arc<Effects>);
impl Provider for CountingProvider {
    fn complete(&self, _req: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
        self.0.inference.fetch_add(1, Ordering::SeqCst);
        Ok(InferenceResponse {
            text: "effect".into(),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 1,
                output_tokens: 1,
            },
            provider_attribution: ProviderAttribution {
                provider_id: PROVIDER.into(),
                endpoint_url: "http://count-stub".into(),
                model_id: None,
            },
        })
    }
}

struct CountingMcp(Arc<Effects>);
impl maos_mcp::McpClient for CountingMcp {
    fn call(
        &self,
        server_name: &str,
        tool: &str,
        _args: serde_json::Value,
    ) -> Result<McpCallResponse, McpError> {
        self.0.mcp.fetch_add(1, Ordering::SeqCst);
        Ok(McpCallResponse::new(
            serde_json::json!({"written": true}),
            false,
            McpAttribution::new(server_name.into(), McpTransportId::Stdio, tool.into()),
        ))
    }
}

struct Fixture {
    scheduler: Arc<SpiritSchedulerAdapter>,
    applier: RevocationApplier,
    tl: Arc<TransparencyLogAdapter>,
    capability: Arc<CapabilityRegistryAdapter>,
    policy: Arc<PolicyTable>,
    _tmp: tempfile::TempDir,
}

fn fixture() -> Fixture {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x2011));
    let telemetry = Arc::new(IacRtMetrics::new());
    let (audit_tx, mut audit_rx) = cap_audit::channel();
    std::thread::spawn(move || while audit_rx.blocking_recv().is_some() {});
    let policy = Arc::new(PolicyTable::new());
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(RingCryptoProvider),
        Ed25519SigningKey::new([0u8; 32]),
        0x2011,
        Arc::clone(&policy),
        audit_tx,
        CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ));
    let memory = Arc::new(MemoryManagerAdapter::new(
        Arc::new(PrivateMemoryStore::new(tmp.path().join("memory"), 4)),
        Arc::new(SharedMemoryStore::open(&tmp.path().join("memory.db")).expect("shared store")),
        Arc::new(
            PrincipalNamespaceIndex::open(&tmp.path().join("memory.db")).expect("principal index"),
        ),
        Arc::clone(&tl),
    ));
    let iac = Arc::new(IacBusAdapter::new(
        Arc::new(Mailbox::new(Arc::clone(&telemetry))),
        Arc::clone(&tl),
    ));
    let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
    let scheduler = Arc::new(SpiritSchedulerAdapter::new(
        Arc::clone(&tl),
        Arc::clone(&capability),
        memory,
        Arc::clone(&iac),
        Arc::clone(&halt_registry),
        Arc::clone(&telemetry),
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    let journal =
        Arc::new(JournalAdapter::open(&tmp.path().join("journal.ndjson")).expect("journal"));
    let applier = RevocationApplier::new(
        scheduler.scbs(),
        Arc::clone(&capability),
        Arc::clone(&scheduler),
        iac,
        halt_registry,
        Arc::clone(&tl),
        journal,
        telemetry,
    );
    Fixture {
        scheduler,
        applier,
        tl,
        capability,
        policy,
        _tmp: tmp,
    }
}

fn manifest(name: &str) -> SpiritManifestBundle {
    SpiritManifestBundle {
        class: Some(ClassSection {
            name: name.into(),
            version: "1.2.3".into(),
            abi: "1.0".into(),
            manifest_schema_version: 1,
            min_substrate_version: "0.1.0".into(),
            forms: vec!["rust-inproc".into()],
            trust_tier: "local".into(),
            description: "FR13 quarantine fixture".into(),
            artifact: None,
        }),
        on_revocation: Some(OnRevocationSection {
            action: RevocationAction::Quarantine,
        }),
        supervision: Some(SupervisionSection {
            heartbeat_interval_ms: 5_000,
            progress_threshold_ms: 5_000,
            silent_failure_threshold_ms: 5_000,
        }),
        ..SpiritManifestBundle::default()
    }
}

fn crl_naming(class: &str) -> SignedRevocationList {
    let entries =
        vec![
            RevocationEntry::new(class, ">=1.0.0,<2.0.0", "FR13 quarantine fixture", None)
                .expect("valid revocation entry"),
        ];
    SignedRevocationList::new(
        CrlId::from_entries(&entries).expect("CRL id"),
        1,
        0,
        RevocationOrigin::Operator,
        entries,
        [1; 64],
        [2; 32],
    )
    .expect("valid CRL")
}

fn scopes() -> Vec<Scope> {
    vec![
        Scope::McpCall {
            server: SERVER.into(),
            tool: TOOL.into(),
        },
        Scope::ProviderInfer {
            provider: PROVIDER.into(),
        },
        Scope::MemWrite {
            scope: "private".into(),
        },
    ]
}

fn mint(
    cap: &CapabilityRegistryAdapter,
    pid: u32,
    scope: Scope,
) -> Result<CapabilityToken, CapError> {
    cap.issue_with_mediation(pid, scope, 60, [0u8; 32], IntentClass::Standard)
}

fn frames(tl: &TransparencyLogAdapter, pid: u32, kind: FrameKind, intent: &str) -> usize {
    tl.query_frames(FrameFilter {
        kind: Some(kind),
        spirit_pid: Some(pid),
        ..Default::default()
    })
    .expect("query frames")
    .into_iter()
    .filter(|frame| frame.intent == intent)
    .count()
}

fn running(fixture: &Fixture, pid: u32) -> bool {
    fixture
        .scheduler
        .scbs()
        .read()
        .expect("spirits lock")
        .get(&pid)
        .is_some_and(|scb| scb.current_state() == ScbLifecycleState::Running)
}

/// A quarantine-declaring `target` and a `bystander` of another class, both
/// Running, `target`'s tokens held from before a CRL naming only its class.
struct Quarantined {
    fixture: Fixture,
    target: u32,
    bystander: u32,
    held: Vec<CapabilityToken>,
    report: maos_domain::revocation::ApplyReport,
}

async fn quarantined() -> Quarantined {
    let fixture = fixture();
    let mut pids = Vec::new();
    for (name, boot) in [("quarantine-worker", 1u64), ("bystander-worker", 2)] {
        let pid = fixture
            .scheduler
            .load(name, manifest(name), TestSpirit, boot)
            .await
            .expect("load fixture");
        fixture.scheduler.start(pid).await.expect("start fixture");
        pids.push(pid);
    }
    let (target, bystander) = (pids[0], pids[1]);
    let mut table = PolicyTableInner::default();
    for pid in &pids {
        table.manifest_scopes.insert(
            *pid,
            ManifestCapabilityScope {
                scopes: scopes(),
                declared_tier: SandboxTier(0),
                trust_tier: TrustTier::Verified,
            },
        );
    }
    fixture.policy.update(table);
    let held: Vec<CapabilityToken> = scopes()
        .into_iter()
        .map(|scope| mint(&fixture.capability, target, scope).expect("a Running Spirit mints"))
        .collect();
    assert!(
        !fixture.capability.is_paused(target),
        "no fence before the CRL"
    );
    let report = fixture
        .applier
        .apply_crl(crl_naming("quarantine-worker"))
        .await
        .expect("apply CRL");
    Quarantined {
        fixture,
        target,
        bystander,
        held,
        report,
    }
}

#[tokio::test(start_paused = true)]
async fn crl_quarantine_journals_one_typed_receipt_and_no_termination() {
    let q = quarantined().await;
    assert_eq!(q.report.matched_count, 1, "only the named class matches");
    assert_eq!(q.report.per_spirit.len(), 1);
    assert_eq!(q.report.per_spirit[0].action, RevocationAction::Quarantine);
    assert_eq!(
        q.report.halt_receipts_produced, 0,
        "quarantine terminates nothing, so it produces no halt receipt"
    );
    let tl = &q.fixture.tl;
    assert_eq!(
        frames(
            tl,
            q.target,
            FrameKind::CapabilityInvocation,
            "spirit.quarantined"
        ),
        1,
        "exactly one typed quarantine receipt"
    );
    assert_eq!(
        frames(
            tl,
            q.target,
            FrameKind::CapabilityInvocation,
            "spirit.quarantine_requested"
        ),
        0,
        "the old request-only marker is gone"
    );
    assert_eq!(
        frames(
            tl,
            q.bystander,
            FrameKind::CapabilityInvocation,
            "spirit.quarantined"
        ),
        0,
        "the unnamed class gets no receipt"
    );
}

#[tokio::test(start_paused = true)]
async fn crl_quarantine_refuses_every_mediated_action_and_reaches_no_effect() {
    let q = quarantined().await;
    let cap = &q.fixture.capability;
    // Issuance for MCP, inference and a memory write, plus verification of the
    // tokens held from before the CRL.
    for scope in scopes() {
        assert_eq!(
            mint(cap, q.target, scope.clone()),
            Err(CapError::Revoked),
            "fenced issuance for {scope:?}"
        );
    }
    for token in &q.held {
        assert_eq!(
            cap.verify_and_audit(token, [0u8; 32], SandboxTier(0)),
            Err(CapError::Revoked),
            "fenced verification"
        );
    }
    // Nothing reaches the effect: the real MCP and inference adapters refuse
    // before the stub is called.
    let effects = Arc::new(Effects::default());
    let mcp = McpClientAdapter::new(
        Arc::new(CountingMcp(Arc::clone(&effects))),
        Arc::clone(cap),
        Arc::clone(&q.fixture.tl),
        Arc::new(IacRtMetrics::new()),
    );
    let provider: Arc<dyn Provider> = Arc::new(CountingProvider(Arc::clone(&effects)));
    let router = Arc::new(MultiProviderRouter::new(
        [(PROVIDER.to_owned(), provider)].into_iter().collect(),
        Some(PROVIDER.into()),
    ));
    let inference = InferencePortAdapter::new(
        router,
        Arc::clone(cap),
        Arc::clone(&q.fixture.tl),
        Arc::new(IacRtMetrics::new()),
    );
    let (mcp_token, infer_token, target) = (q.held[0].clone(), q.held[1].clone(), q.target);
    let (mcp_result, infer_result) = std::thread::spawn(move || {
        let mcp_result = mcp.call(&mcp_token, SERVER, TOOL, serde_json::json!({"k": "v"}));
        let infer_result = inference.complete(InferenceRequest::new(
            target,
            infer_token,
            "hello".into(),
            InferenceOptions::default(),
            Some(PROVIDER.into()),
            Vec::new(),
        ));
        (mcp_result, infer_result)
    })
    .join()
    .expect("effect probe joins");
    assert!(
        matches!(mcp_result, Err(McpError::CapabilityDenied { .. })),
        "{mcp_result:?}"
    );
    assert!(
        matches!(infer_result, Err(InferenceError::CapabilityDenied)),
        "{infer_result:?}"
    );
    assert_eq!(effects.mcp.load(Ordering::SeqCst), 0, "no MCP effect");
    assert_eq!(
        effects.inference.load(Ordering::SeqCst),
        0,
        "no inference effect"
    );

    // The CRL named one class: the bystander still mints and verifies.
    let ok = mint(cap, q.bystander, scopes()[1].clone()).expect("an unnamed Spirit still mints");
    cap.verify_and_audit(&ok, [0u8; 32], SandboxTier(0))
        .expect("and verifies");
}

#[tokio::test(start_paused = true)]
async fn crl_quarantine_keeps_the_spirit_loaded_and_running_past_the_drain_deadline() {
    let q = quarantined().await;
    // The old drain deadline is 2 x progress_threshold = 10 s.
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_millis(10_001)).await;
    for _ in 0..100 {
        tokio::task::yield_now().await;
    }
    assert!(running(&q.fixture, q.target), "quarantine never terminates");
    assert!(
        q.fixture.capability.is_paused(q.target),
        "the fence outlives the old deadline"
    );
    assert_eq!(
        mint(&q.fixture.capability, q.target, scopes()[1].clone()),
        Err(CapError::Revoked)
    );
    assert!(
        q.fixture
            .tl
            .query_frames(FrameFilter {
                kind: Some(FrameKind::EpistemicHalt),
                spirit_pid: Some(q.target),
                ..Default::default()
            })
            .expect("query termination receipts")
            .is_empty(),
        "no termination receipt for a never-terminated Spirit"
    );
    assert!(
        running(&q.fixture, q.bystander),
        "the bystander is untouched"
    );
}

#[tokio::test(start_paused = true)]
async fn crl_quarantine_survives_director_resume_and_ends_only_at_unload() {
    let q = quarantined().await;
    let cap = &q.fixture.capability;
    q.fixture
        .scheduler
        .pause(q.target)
        .await
        .expect("director pause");
    q.fixture
        .scheduler
        .resume(q.target)
        .await
        .expect("director resume");
    assert!(running(&q.fixture, q.target));
    assert!(
        cap.is_paused(q.target),
        "resume leaves the quarantine fence"
    );
    assert_eq!(
        mint(cap, q.target, scopes()[0].clone()),
        Err(CapError::Revoked),
        "still fenced after pause -> resume"
    );
    q.fixture.scheduler.unload(q.target).await.expect("unload");
    assert!(!cap.is_paused(q.target), "unload clears the quarantine");
    assert!(running(&q.fixture, q.bystander));
}
