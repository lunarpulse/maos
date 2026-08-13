#![forbid(unsafe_code)]

//! Behavioral CRL propagation: parser → applier → token denial → lifecycle
//! evidence, plus the concurrent idempotency/admission boundary.

use std::sync::{Arc, Barrier};

use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::lifecycle::LifecycleError;
use maos_domain::ports::CryptoProvider;
use maos_domain::revocation::{
    canonical_entries_bytes, CrlId, RevocationEntry, RevocationError, RevocationOrigin,
    SignedRevocationList,
};
use maos_kernel_core::capability::{
    cap_audit,
    cap_policy::{decision::TrustTier, ManifestCapabilityScope, PolicyTable, PolicyTableInner},
    cap_quota::CapQuotaTracker,
    cap_tokens::Ed25519SigningKey,
    CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::iac::{
    transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter},
    IacBusAdapter, Mailbox,
};
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::memory::{
    principal::PrincipalNamespaceIndex, private::PrivateMemoryStore, shared::SharedMemoryStore,
    MemoryManagerAdapter,
};
use maos_kernel_core::revocation::{parse_signed_crl, RevocationApplier};
use maos_kernel_core::scheduler::{
    control_block::SpiritManifestBundle, scheduler_loop::SpiritSchedulerAdapter,
};
use maos_kernel_core::security::{manifest::ClassSection, RingCryptoProvider};
use maos_kernel_core::telemetry::{iac_rt::IacRtMetrics, TelemetryStreamAdapter};
use ring::signature::{Ed25519KeyPair, KeyPair};

struct TestSpirit;
impl maos_spirit_abi::lifecycle::Spirit for TestSpirit {}

struct Fixture {
    scheduler: Arc<SpiritSchedulerAdapter>,
    policy: Arc<PolicyTable>,
    capability: Arc<CapabilityRegistryAdapter>,
    applier: Arc<RevocationApplier>,
    tl: Arc<TransparencyLogAdapter>,
    journal: Arc<JournalAdapter>,
    _tmp: tempfile::TempDir,
}

fn manifest() -> SpiritManifestBundle {
    let mut manifest = SpiritManifestBundle::default();
    manifest.class = Some(ClassSection {
        name: "revoked-spirit".into(),
        version: "1.2.3".into(),
        abi: "1.0".into(),
        manifest_schema_version: 1,
        min_substrate_version: "0.1.0".into(),
        forms: vec!["rust-inproc".into()],
        trust_tier: "local".into(),
        description: "revocation integration fixture".into(),
    });
    manifest
}

fn fixture() -> Fixture {
    let tmp = tempfile::TempDir::new().unwrap();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xC0DE));
    let policy = Arc::new(PolicyTable::new());
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(RingCryptoProvider),
        Ed25519SigningKey::new([0u8; 32]),
        0xC0DE,
        Arc::clone(&policy),
        cap_audit::channel().0,
        CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ));
    let memory = Arc::new(MemoryManagerAdapter::new(
        Arc::new(PrivateMemoryStore::new(tmp.path().join("memory"), 4)),
        Arc::new(SharedMemoryStore::open(&tmp.path().join("memory.db")).unwrap()),
        Arc::new(PrincipalNamespaceIndex::open(&tmp.path().join("memory.db")).unwrap()),
        Arc::clone(&tl),
    ));
    let telemetry = Arc::new(IacRtMetrics::new());
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
        None,
    ));
    let journal = Arc::new(JournalAdapter::open(&tmp.path().join("journal.ndjson")).unwrap());
    let applier = Arc::new(RevocationApplier::new(
        scheduler.scbs(),
        Arc::clone(&capability),
        Arc::clone(&scheduler),
        iac,
        halt_registry,
        Arc::clone(&tl),
        Arc::clone(&journal),
        telemetry,
    ));
    Fixture {
        scheduler,
        policy,
        capability,
        applier,
        tl,
        journal,
        _tmp: tmp,
    }
}

fn signed_crl() -> (SignedRevocationList, [u8; 32]) {
    let entries = vec![RevocationEntry::new(
        "revoked-spirit",
        ">=1.0.0,<2.0.0",
        "compromised build",
        None,
    )
    .unwrap()];
    let seed = [0x42; 32];
    let keypair = Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
    let public: [u8; 32] = keypair.public_key().as_ref().try_into().unwrap();
    let signature: [u8; 64] = RingCryptoProvider
        .sign_capability_token(&seed, &canonical_entries_bytes(&entries).unwrap())
        .unwrap()
        .try_into()
        .unwrap();
    (
        SignedRevocationList::new(
            CrlId::from_entries(&entries).unwrap(),
            1,
            0,
            RevocationOrigin::Operator,
            entries,
            signature,
            public,
        )
        .unwrap(),
        public,
    )
}

#[tokio::test]
async fn signed_crl_propagates_to_token_denial_and_lifecycle_evidence() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let fixture = fixture();
    let pid = fixture
        .scheduler
        .load("revoked-spirit", manifest(), TestSpirit, 7)
        .await
        .unwrap();
    fixture.scheduler.start(pid).await.unwrap();
    let mut policy = PolicyTableInner::default();
    policy.manifest_scopes.insert(
        pid,
        ManifestCapabilityScope {
            scopes: vec![Scope::FsRead {
                subtree: "/tmp/revocation".into(),
            }],
            declared_tier: SandboxTier(0),
            trust_tier: TrustTier::Verified,
        },
    );
    fixture.policy.update(policy);
    let token = fixture
        .capability
        .issue_with_mediation(
            pid,
            Scope::FsRead {
                subtree: "/tmp/revocation".into(),
            },
            60,
            [0u8; 32],
            IntentClass::Standard,
        )
        .unwrap();
    assert!(fixture
        .capability
        .verify_and_audit(&token, [0u8; 32], SandboxTier::T2)
        .is_ok());

    let (crl, anchor) = signed_crl();
    let parsed = parse_signed_crl(
        &serde_json::to_vec(&crl).unwrap(),
        &anchor,
        &RingCryptoProvider,
    )
    .unwrap();
    let report = fixture.applier.apply_crl(parsed.clone()).await.unwrap();

    assert_eq!(report.matched_count, 1);
    assert_eq!(report.tokens_revoked_total, 1);
    assert!(matches!(
        fixture
            .capability
            .verify_and_audit(&token, [0u8; 32], SandboxTier::T2),
        Err(maos_domain::ports::capability::CapError::Revoked)
    ));
    assert_eq!(
        fixture.journal.last_event("revoked-spirit"),
        Some(maos_domain::invariants::i10::LifecycleEvent::Revoked)
    );
    assert_eq!(
        fixture
            .tl
            .query_frames(FrameFilter {
                kind: Some(FrameKind::SpiritRevoked),
                spirit_pid: Some(pid),
                ..Default::default()
            })
            .unwrap()
            .len(),
        1
    );
    assert!(matches!(
        fixture.applier.apply_crl(parsed).await,
        Err(RevocationError::AlreadyApplied { .. })
    ));

    let admission = fixture
        .scheduler
        .load("blocked-future-spirit", manifest(), TestSpirit, 8)
        .await;
    assert!(matches!(admission, Err(LifecycleError::Admission(_))));
}

#[test]
fn concurrent_apply_reserves_exactly_one_crl_commit() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let fixture = fixture();
    let (crl, _) = signed_crl();
    let barrier = Arc::new(Barrier::new(9));
    let mut workers = Vec::new();
    for _ in 0..8 {
        let applier = Arc::clone(&fixture.applier);
        let crl = crl.clone();
        let barrier = Arc::clone(&barrier);
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .unwrap()
                .block_on(applier.apply_crl(crl))
        }));
    }
    barrier.wait();
    let outcomes: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Err(RevocationError::AlreadyApplied { .. })))
            .count(),
        7
    );
}
