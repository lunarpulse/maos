#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use maos_domain::revocation::{
    CrlId, RevocationAction, RevocationEntry, RevocationOrigin, SignedRevocationList,
};
use maos_kernel_core::capability::{
    cap_audit, cap_policy::PolicyTable, cap_quota::CapQuotaTracker, cap_tokens::Ed25519SigningKey,
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
use maos_kernel_core::revocation::RevocationApplier;
use maos_kernel_core::scheduler::{control_block::SpiritManifestBundle, SpiritSchedulerAdapter};
use maos_kernel_core::security::{
    manifest::{ClassSection, OnRevocationSection, SupervisionSection},
    RingCryptoProvider,
};
use maos_kernel_core::telemetry::{iac_rt::IacRtMetrics, TelemetryStreamAdapter};

struct TestSpirit;
impl maos_spirit_abi::lifecycle::Spirit for TestSpirit {}

struct Fixture {
    scheduler: Arc<SpiritSchedulerAdapter>,
    applier: RevocationApplier,
    tl: Arc<TransparencyLogAdapter>,
    _tmp: tempfile::TempDir,
}

fn fixture() -> Fixture {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let tmp = tempfile::TempDir::new().expect("revocation tempdir");
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xAC71));
    let telemetry = Arc::new(IacRtMetrics::new());
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(RingCryptoProvider),
        Ed25519SigningKey::new([0u8; 32]),
        0xAC71,
        Arc::new(PolicyTable::new()),
        cap_audit::channel().0,
        CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ));
    let memory = Arc::new(MemoryManagerAdapter::new(
        Arc::new(PrivateMemoryStore::new(tmp.path().join("memory"), 4)),
        Arc::new(
            SharedMemoryStore::open(&tmp.path().join("memory.db")).expect("shared memory store"),
        ),
        Arc::new(
            PrincipalNamespaceIndex::open(&tmp.path().join("memory.db"))
                .expect("principal namespace store"),
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
        None,
    ));
    let journal = Arc::new(
        JournalAdapter::open(&tmp.path().join("journal.ndjson")).expect("revocation journal"),
    );
    let applier = RevocationApplier::new(
        scheduler.scbs(),
        capability,
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
        _tmp: tmp,
    }
}

fn manifest(name: &str, action: RevocationAction) -> SpiritManifestBundle {
    SpiritManifestBundle {
        class: Some(ClassSection {
            name: name.into(),
            version: "1.2.3".into(),
            abi: "1.0".into(),
            manifest_schema_version: 1,
            min_substrate_version: "0.1.0".into(),
            forms: vec!["rust-inproc".into()],
            trust_tier: "local".into(),
            description: "revocation action fixture".into(),
        }),
        on_revocation: Some(OnRevocationSection { action }),
        supervision: Some(SupervisionSection {
            heartbeat_interval_ms: 5_000,
            progress_threshold_ms: 5_000,
            silent_failure_threshold_ms: 5_000,
        }),
        ..SpiritManifestBundle::default()
    }
}

fn signed_crl() -> SignedRevocationList {
    let entries = ["terminate-worker", "drain-worker", "quarantine-worker"]
        .into_iter()
        .map(|class| {
            RevocationEntry::new(class, ">=1.0.0,<2.0.0", "action integration test", None)
                .expect("valid revocation entry")
        })
        .collect::<Vec<_>>();
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

#[tokio::test(start_paused = true)]
async fn applier_executes_terminate_drain_and_quarantine_actions() {
    let fixture = fixture();
    let mut pids = Vec::new();
    for (name, action) in [
        ("terminate-worker", RevocationAction::TerminateImmediately),
        ("drain-worker", RevocationAction::DrainThenTerminate),
        ("quarantine-worker", RevocationAction::Quarantine),
    ] {
        let pid = fixture
            .scheduler
            .load(
                name,
                manifest(name, action),
                TestSpirit,
                u64::from(pids.len() as u32 + 1),
            )
            .await
            .expect("load action fixture");
        fixture
            .scheduler
            .start(pid)
            .await
            .expect("start action fixture");
        pids.push(pid);
    }

    let report = fixture
        .applier
        .apply_crl(signed_crl())
        .await
        .expect("apply CRL");
    assert_eq!(report.matched_count, 3);
    assert_eq!(report.revoked_count, 3);
    assert_eq!(report.per_spirit.len(), 3);
    assert_eq!(report.halt_receipts_produced, 1);
    for action in [
        RevocationAction::TerminateImmediately,
        RevocationAction::DrainThenTerminate,
        RevocationAction::Quarantine,
    ] {
        assert!(report.per_spirit.iter().any(|entry| entry.action == action));
    }

    {
        let scbs = fixture.scheduler.scbs();
        let map = scbs.read().expect("spirits lock poisoned");
        assert!(
            !map.contains_key(&pids[0]),
            "terminate must unload immediately"
        );
        assert!(
            map.contains_key(&pids[1]),
            "drain must remain until deadline"
        );
        assert!(
            map.contains_key(&pids[2]),
            "quarantine must remain until deadline"
        );
    }
    assert_eq!(
        fixture
            .tl
            .query_frames(FrameFilter {
                kind: Some(FrameKind::CapabilityInvocation),
                spirit_pid: Some(pids[2]),
                ..Default::default()
            })
            .expect("query quarantine marker")
            .into_iter()
            .filter(|frame| frame.intent == "spirit.quarantine_requested")
            .count(),
        1
    );

    // Let both deferred-action tasks register their timers before advancing
    // the paused clock.
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_millis(10_001)).await;
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    {
        let scbs = fixture.scheduler.scbs();
        let map = scbs.read().expect("spirits lock poisoned");
        assert!(!map.contains_key(&pids[1]), "drain must unload at deadline");
        assert!(
            !map.contains_key(&pids[2]),
            "quarantine must unload at deadline"
        );
    }
    for pid in pids {
        assert!(
            fixture
                .tl
                .query_frames(FrameFilter {
                    kind: Some(FrameKind::EpistemicHalt),
                    spirit_pid: Some(pid),
                    ..Default::default()
                })
                .expect("query termination receipts")
                .len()
                >= 2,
            "each action must produce a termination receipt"
        );
    }
}
