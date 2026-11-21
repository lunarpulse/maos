//! In-process daemon pieces of the FR13 closure tests: a scheduler with a real security manager,
//! the capability registry, the IAC bus the sender fence lives on, and admission deps wired to a
//! `QuarantineGate`. (Same composition as `yank_policy_20_1b.rs`'s `World`, which can adopt this.)
#![allow(dead_code)]

use maos_bin::admission::{
    gate_manifest, load_admit_start, AdmissionDeps, GatedManifest, StartPolicy,
};
use maos_bin::quarantine::QuarantineGate;
use maos_kernel_core::capability::{
    cap_audit, cap_policy::PolicyTable, cap_quota::CapQuotaTracker, cap_tokens::Ed25519SigningKey,
    CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::iac::{IacBusAdapter, Mailbox, TransparencyLogAdapter};
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use maos_kernel_core::scheduler::{make_spirit_obj, SpiritSchedulerAdapter};
use maos_kernel_core::security::RingCryptoProvider;
use maos_kernel_core::telemetry::{iac_rt::IacRtMetrics, TelemetryStreamAdapter};
use std::sync::Arc;

pub struct Spirit;
impl maos_spirit_abi::lifecycle::Spirit for Spirit {}

pub struct World {
    pub cap: Arc<CapabilityRegistryAdapter>,
    pub scheduler: Arc<SpiritSchedulerAdapter>,
    pub tl: Arc<TransparencyLogAdapter>,
    pub iac: Arc<IacBusAdapter>,
    memory: Arc<MemoryManagerAdapter>,
    security: Arc<maos_kernel_core::security::SecurityManagerAdapter>,
    journal: Arc<maos_kernel_core::journal::JournalAdapter>,
    pids: Arc<std::sync::RwLock<std::collections::BTreeMap<String, u32>>>,
}

pub fn world(path: &std::path::Path) -> World {
    static CLOCK: std::sync::Once = std::sync::Once::new();
    CLOCK.call_once(maos_kernel_core::capability::cap_tokens::init_monotonic_base);
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x1fe1));
    let telemetry = Arc::new(IacRtMetrics::new());
    let (tx, mut rx) = cap_audit::channel();
    std::thread::spawn(move || while rx.blocking_recv().is_some() {});
    let policy = Arc::new(PolicyTable::new());
    let security = Arc::new(maos_kernel_core::security::SecurityManagerAdapter::new(
        policy.clone(),
        Arc::new(RingCryptoProvider),
    ));
    let cap = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(RingCryptoProvider),
        Ed25519SigningKey::new([0x21; 32]),
        0x1fe1,
        policy,
        tx,
        CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ));
    std::fs::create_dir_all(path).unwrap();
    let memory = Arc::new(
        MemoryManagerAdapter::new(
            Arc::new(PrivateMemoryStore::new(path.join("private"), 4)),
            Arc::new(SharedMemoryStore::open(&path.join("memory.db")).unwrap()),
            Arc::new(PrincipalNamespaceIndex::open(&path.join("memory.db")).unwrap()),
            tl.clone(),
        )
        .with_capabilities(Some(cap.clone())),
    );
    let iac = Arc::new(IacBusAdapter::new(
        Arc::new(Mailbox::new(telemetry.clone())),
        tl.clone(),
    ));
    let scheduler = Arc::new(SpiritSchedulerAdapter::new(
        tl.clone(),
        cap.clone(),
        memory.clone(),
        iac.clone(),
        Arc::new(maos_kernel_core::halt::HaltRegistry::new()),
        telemetry,
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    let journal = Arc::new(
        maos_kernel_core::journal::JournalAdapter::open(&path.join("journal.ndjson")).unwrap(),
    );
    World {
        cap,
        scheduler,
        tl,
        iac,
        memory,
        security,
        journal,
        pids: Arc::new(std::sync::RwLock::new(std::collections::BTreeMap::new())),
    }
}

impl World {
    pub fn gate(&self, store: &std::path::Path) -> QuarantineGate {
        self.open_gate(Some(store.to_path_buf()))
    }

    /// A gate with no durable path (no home resolved): holds live in memory only.
    pub fn open_gate(&self, path: Option<std::path::PathBuf>) -> QuarantineGate {
        QuarantineGate::open(path, self.cap.clone(), self.iac.clone(), self.tl.clone()).unwrap()
    }

    /// The shipped hello Spirit's gated manifest (its `[class]` name and version).
    pub fn manifest() -> GatedManifest {
        gate_manifest(maos_spirit_hello::MANIFEST_TOML, &|_| true).unwrap()
    }

    /// Admit through the real triple, leaving the Spirit `Loaded` (never `Running`).
    pub async fn admit_loaded(&self, gate: Option<&QuarantineGate>) -> u32 {
        load_admit_start(
            &AdmissionDeps {
                providers: None,
                iac: Some(&self.iac),
                quarantine: gate,
                scheduler: &self.scheduler,
                security: &self.security,
                transparency_log: &self.tl,
                journal: &self.journal,
                memory: &self.memory,
                pid_by_spirit_id: &self.pids,
                hooks: None,
            },
            &Self::manifest(),
            make_spirit_obj(Spirit),
            1,
            StartPolicy::LeaveLoaded,
            &mut |_| {},
        )
        .await
        .unwrap()
        .pid
    }
}
