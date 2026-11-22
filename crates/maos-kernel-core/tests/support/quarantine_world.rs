//! Shared kernel fixture of the FR13 quarantine tests (`quarantine_fr13_20_1.rs`,
//! `quarantine_closure_lrqc.rs`): a scheduler, applier, memory adapter with the capability
//! registry wired (so the memory fence is live) and the IAC bus the applier fences.
#![allow(dead_code)]

use std::sync::Arc;

use maos_domain::revocation::{
    CrlId, RevocationAction, RevocationEntry, RevocationOrigin, SignedRevocationList,
};
use maos_kernel_core::capability::cap_policy::PolicyTable;
use maos_kernel_core::capability::{
    cap_audit, cap_quota::CapQuotaTracker, cap_tokens::Ed25519SigningKey,
    CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::iac::{transparency_log::TransparencyLogAdapter, IacBusAdapter, Mailbox};
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

pub struct TestSpirit;
impl maos_spirit_abi::lifecycle::Spirit for TestSpirit {}

pub struct Fixture {
    pub scheduler: Arc<SpiritSchedulerAdapter>,
    pub applier: RevocationApplier,
    pub tl: Arc<TransparencyLogAdapter>,
    pub capability: Arc<CapabilityRegistryAdapter>,
    pub policy: Arc<PolicyTable>,
    pub iac: Arc<IacBusAdapter>,
    pub memory: Arc<MemoryManagerAdapter>,
    pub _tmp: tempfile::TempDir,
}

pub fn fixture() -> Fixture {
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
    let memory = Arc::new(
        MemoryManagerAdapter::new(
            Arc::new(PrivateMemoryStore::new(tmp.path().join("memory"), 4)),
            Arc::new(SharedMemoryStore::open(&tmp.path().join("memory.db")).expect("shared store")),
            Arc::new(
                PrincipalNamespaceIndex::open(&tmp.path().join("memory.db"))
                    .expect("principal index"),
            ),
            Arc::clone(&tl),
        )
        .with_capabilities(Some(Arc::clone(&capability))),
    );
    let iac = Arc::new(IacBusAdapter::new(
        Arc::new(Mailbox::new(Arc::clone(&telemetry))),
        Arc::clone(&tl),
    ));
    let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
    let scheduler = Arc::new(SpiritSchedulerAdapter::new(
        Arc::clone(&tl),
        Arc::clone(&capability),
        Arc::clone(&memory),
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
        Arc::clone(&iac),
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
        iac,
        memory,
        _tmp: tmp,
    }
}

pub fn manifest(name: &str) -> SpiritManifestBundle {
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

pub fn crl_naming(class: &str) -> SignedRevocationList {
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
