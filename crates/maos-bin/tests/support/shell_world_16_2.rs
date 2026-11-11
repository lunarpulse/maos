//! Shared real stores/scheduler/door for shell and large erasure vectors.
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use maos_bin::operator_door::{BinPrivateOps, OperatorDoor, SuccessorManifestSlot};
use maos_director_surface::notification::NotificationDispatcher;
use maos_iac::adapter::{IacBusAdapter, Mailbox};
use maos_kernel_core::capability::cap_policy::{
    ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_quota::CapQuotaTracker;
use maos_kernel_core::capability::cap_tokens::Ed25519SigningKey;
use maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator;
use maos_kernel_core::capability::{CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::halt::{HaltRegistry, OutputMarkerRegistry};
use maos_kernel_core::hot_swap::HotSwapCoordinator;
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use maos_kernel_core::orchestrator::OrchestratorBufferRegistry;
use maos_kernel_core::revocation::RevocationApplier;
use maos_kernel_core::scheduler::{HookDispatcher, SpiritSchedulerAdapter};
use maos_kernel_core::security::crypto::RingCryptoProvider;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
pub(super) struct KernelWorld {
    pub(super) dir: tempfile::TempDir,
    pub(super) transparency_log: Arc<TransparencyLogAdapter>,
    pub(super) journal: Arc<JournalAdapter>,
    pub(super) halt_registry: Arc<HaltRegistry>,
    pub(super) memory: Arc<MemoryManagerAdapter>,
    pub(super) capability: Arc<CapabilityRegistryAdapter>,
    /// Live (ArcSwap) — re-seeded with the scheduler's ACTUAL pid below:
    /// `allocate_pid` is a PROCESS-GLOBAL counter, so under the parallel
    /// workspace suite the loaded Spirit is not always pid 1.
    pub(super) policy: Arc<PolicyTable>,
}

pub(super) fn kernel_world(boot_nonce: u64) -> KernelWorld {
    kernel_world_with_crypto(boot_nonce, Arc::new(RingCryptoProvider))
}

pub(super) fn kernel_world_with_crypto(
    boot_nonce: u64,
    crypto: Arc<dyn maos_domain::ports::CryptoProvider>,
) -> KernelWorld {
    let dir = tempfile::TempDir::new().unwrap();
    let audit_db = dir.path().join("transparency.sqlite");
    let journal_path = dir.path().join("journal.sqlite");
    let memory_root = dir.path().join("memory");
    std::fs::create_dir_all(&memory_root).unwrap();
    let principal_index =
        Arc::new(PrincipalNamespaceIndex::open(&audit_db).expect("open principal index"));
    let transparency_log = Arc::new(
        TransparencyLogAdapter::open_with_global_legal_holds(&audit_db, &audit_db, boot_nonce)
            .expect("open transparency log"),
    );
    let memory = MemoryManagerAdapter::new(
        Arc::new(PrivateMemoryStore::new(memory_root, 4 * 1024)),
        Arc::new(SharedMemoryStore::open(&audit_db).expect("open shared store")),
        principal_index,
        Arc::clone(&transparency_log),
    );
    // Seed the policy table with hello-spirit's manifest scope so turn-token
    // issuance succeeds exactly as it does against the real composition root
    // (the manifest-derived policy the daemon builds).
    let policy = Arc::new(PolicyTable::new());
    let mut policy_inner = PolicyTableInner::default();
    policy_inner.manifest_scopes.insert(
        1,
        ManifestCapabilityScope {
            scopes: vec![maos_domain::invariants::i1::Scope::ProviderInfer {
                provider: "anthropic".into(),
            }],
            declared_tier: maos_domain::invariants::i9::SandboxTier(0),
            trust_tier: maos_kernel_core::capability::cap_policy::decision::TrustTier::Verified,
        },
    );
    policy.update(policy_inner);
    let (audit_tx, mut audit_rx) = maos_kernel_core::capability::cap_audit::channel();
    std::thread::spawn(move || while audit_rx.blocking_recv().is_some() {});
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        crypto,
        Ed25519SigningKey::new([0x16; 32]),
        boot_nonce,
        Arc::clone(&policy),
        audit_tx,
        CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ));
    let journal = Arc::new(JournalAdapter::open(&journal_path).expect("open journal"));
    let halt_registry = Arc::new(HaltRegistry::new());
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    KernelWorld {
        dir,
        transparency_log,
        journal,
        halt_registry,
        memory: Arc::new(memory),
        capability,
        policy,
    }
}

/// A real scheduler with a real loaded hello-spirit — the object `main`
/// drives, so `finish_shell_session` runs exactly the path production runs.
pub(super) fn scheduler_with_hello(
    world: &KernelWorld,
    boot_nonce: u64,
) -> Arc<SpiritSchedulerAdapter> {
    scheduler_with_bundle(world, boot_nonce, Default::default())
}

pub(super) fn scheduler_with_bundle(
    world: &KernelWorld,
    boot_nonce: u64,
    bundle: maos_kernel_core::scheduler::SpiritManifestBundle,
) -> Arc<SpiritSchedulerAdapter> {
    let iac = IacBusAdapter::new(
        Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new()))),
        Arc::clone(&world.transparency_log),
    );
    let scheduler = Arc::new(SpiritSchedulerAdapter::new(
        Arc::clone(&world.transparency_log),
        Arc::clone(&world.capability),
        Arc::clone(&world.memory),
        Arc::new(iac),
        Arc::clone(&world.halt_registry),
        Arc::new(IacRtMetrics::new()),
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime");
    let pid = rt.block_on(async {
        let pid = scheduler
            .load(
                "hello-spirit",
                bundle,
                maos_spirit_hello::HelloSpirit,
                boot_nonce,
            )
            .await
            .expect("load hello-spirit");
        // `allocate_pid` is PROCESS-GLOBAL: under the parallel suite the
        // real pid is not always 1. Re-seed the LIVE policy table (ArcSwap —
        // the adapter sees the update) with the ACTUAL pid so turn-token
        // issuance succeeds.
        let mut policy_inner = PolicyTableInner::default();
        policy_inner.manifest_scopes.insert(
            pid,
            ManifestCapabilityScope {
                scopes: vec![maos_domain::invariants::i1::Scope::ProviderInfer {
                    provider: "anthropic".into(),
                }],
                declared_tier: maos_domain::invariants::i9::SandboxTier(0),
                trust_tier: maos_kernel_core::capability::cap_policy::decision::TrustTier::Verified,
            },
        );
        world.policy.update(policy_inner);
        // The shell block's exact sequence: load → admit (security, not
        // wired in the kernel-object world) → start.
        //
        // ⚠ Story 16-6 — this used to add "without start the transition
        // table refuses Unload from Loaded, the residual the epic-16 retro
        // row names". That arm now exists, so the refusal is gone and the
        // residual is closed; `start` stays because this fixture is
        // reproducing the shell's sequence, not working around a defect.
        scheduler.start(pid).await.expect("start hello-spirit");
        pid
    });
    assert_ne!(pid, 0, "the scheduler assigns a real pid");
    scheduler
}

/// The bin-private ops, unwired — they back uninstall/upgrade/migration, none
/// of which the halt-resolve race below drives, so every method is
/// unreachable by construction of that vector.
#[derive(Default)]
pub(super) struct UnwiredBinOps {
    pub(super) schedule_capture:
        Option<Arc<std::sync::Mutex<Option<maos_bin::operator_door::DoorUninstall>>>>,
}

#[async_trait]
impl BinPrivateOps for UnwiredBinOps {
    async fn append_lifecycle_journal(
        &self,
        _event: maos_domain::invariants::i10::LifecycleEvent,
        _spirit_id: &str,
    ) -> Result<PathBuf, String> {
        unreachable!("the halt-resolve race never touches the bin-private ops");
    }
    async fn load_spirit(
        &self,
        _manifest: &std::path::Path,
        _pread: Option<std::sync::Arc<String>>,
    ) -> Result<maos_bin::admission::Admitted, maos_bin::admission::AdmissionRefusal> {
        unreachable!("the halt-resolve race never touches the bin-private ops");
    }

    async fn run_uninstall_cascade(
        &self,
        _spirit_id: &str,
        door: maos_bin::operator_door::DoorUninstall,
    ) -> maos_bin::operator_door::UninstallOutcome {
        let capture = self
            .schedule_capture
            .as_ref()
            .expect("only the snapshot oracle reaches this fixture spy");
        *capture.lock().expect("snapshot") = Some(door);
        maos_bin::operator_door::UninstallOutcome {
            body: serde_json::json!({"fixture_snapshot_only":true}),
            terminal_code: 0,
        }
    }

    async fn enforce_vetted_upgrade_precondition(
        &self,
        _spirit_id: &str,
        _target_manifest: &Path,
        _evidence: maos_bin::operator_door::UpgradeVetting<'_>,
    ) -> Result<(), String> {
        unreachable!("the halt-resolve race never touches the bin-private ops");
    }

    async fn upgrade_with_plan_guard(
        &self,
        _spirit_id: &str,
        _target_manifest: &Path,
        _original_target: &Path,
        _policy: maos_kernel_core::lifecycle::UpgradePolicy,
        _evidence: maos_bin::operator_door::UpgradeVetting<'_>,
    ) -> Result<serde_json::Value, maos_bin::operator_door::UpgradeFailure> {
        unreachable!("the halt-resolve race never touches the bin-private ops");
    }

    async fn create_migration_plan(
        &self,
        _spirit_id: &str,
        _from_version: &str,
        _target_manifest: &Path,
        _candidates: &[String],
    ) -> Result<serde_json::Value, String> {
        unreachable!("the halt-resolve race never touches the bin-private ops");
    }

    fn admitted_manifest(&self, _spirit_id: &str) -> Option<Arc<String>> {
        unreachable!("the halt-resolve race never touches the bin-private ops");
    }

    fn record_upgraded_manifest(&self, _spirit_id: &str, _text: &str) {
        unreachable!("the halt-resolve race never touches the bin-private ops");
    }

    fn migration_plan_pending(&self, _spirit_id: &str) -> bool {
        unreachable!("the halt-resolve race never touches the bin-private ops");
    }
}

pub(super) fn regression_door(
    world: &KernelWorld,
    scheduler: &Arc<maos_kernel_core::scheduler::SpiritSchedulerAdapter>,
    rt: &tokio::runtime::Runtime,
    boot_nonce: u64,
) -> Arc<OperatorDoor> {
    regression_door_with_ops(
        world,
        scheduler,
        rt,
        boot_nonce,
        Arc::new(UnwiredBinOps::default()),
    )
}

pub(super) fn regression_door_with_ops(
    world: &KernelWorld,
    scheduler: &Arc<SpiritSchedulerAdapter>,
    rt: &tokio::runtime::Runtime,
    boot_nonce: u64,
    ops: Arc<dyn BinPrivateOps>,
) -> Arc<OperatorDoor> {
    regression_door_with_crypto(
        world,
        scheduler,
        rt.handle().clone(),
        boot_nonce,
        ops,
        Arc::new(RingCryptoProvider),
        None,
    )
}

pub(super) fn regression_door_with_crypto(
    world: &KernelWorld,
    scheduler: &Arc<SpiritSchedulerAdapter>,
    rt: tokio::runtime::Handle,
    boot_nonce: u64,
    ops: Arc<dyn BinPrivateOps>,
    crypto: Arc<dyn maos_domain::ports::CryptoProvider>,
    anchor: Option<Vec<u8>>,
) -> Arc<OperatorDoor> {
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let iac = Arc::new(IacBusAdapter::new(
        Arc::clone(&mailbox),
        Arc::clone(&world.transparency_log),
    ));
    let door = Arc::new(OperatorDoor::new(
        Arc::clone(scheduler),
        Arc::clone(&world.policy),
        Arc::clone(&world.halt_registry),
        Arc::new(OutputMarkerRegistry::new()),
        Arc::new(OrchestratorBufferRegistry::new()),
        Arc::new(RevocationApplier::new(
            scheduler.scbs(),
            Arc::clone(&world.capability),
            Arc::clone(scheduler),
            Arc::clone(&iac),
            Arc::clone(&world.halt_registry),
            Arc::clone(&world.transparency_log),
            Arc::clone(&world.journal),
            Arc::new(IacRtMetrics::new()),
        )),
        Arc::new(HotSwapCoordinator::new(
            scheduler.scbs(),
            Arc::clone(&world.journal),
            Arc::clone(&world.transparency_log),
            Arc::clone(&world.halt_registry),
            Arc::clone(&world.capability),
            Arc::clone(&iac),
            Arc::new(HookDispatcher::new(
                Arc::clone(&world.transparency_log),
                Arc::new(IacRtMetrics::new()),
            )),
            Arc::new(IacRtMetrics::new()),
            world.dir.path().join("hot-swap-archive"),
        )),
        Arc::clone(&world.capability),
        Arc::clone(&world.memory),
        Arc::new(WorkingMemoryOrchestrator::new(
            Arc::clone(&world.capability),
            Arc::clone(&world.halt_registry),
        )),
        mailbox,
        Arc::new(NotificationDispatcher::new()),
        Arc::clone(&world.transparency_log),
        crypto,
        anchor,
        boot_nonce,
        rt,
        ops,
        SuccessorManifestSlot::default(),
    ));
    door
}
