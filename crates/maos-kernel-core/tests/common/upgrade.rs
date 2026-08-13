use std::sync::Arc;

use maos_kernel_core::capability::{
    cap_audit, cap_policy::PolicyTable, cap_quota::CapQuotaTracker, cap_tokens::Ed25519SigningKey,
    CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::hot_swap::HotSwapCoordinator;
use maos_kernel_core::iac::{transparency_log::TransparencyLogAdapter, IacBusAdapter, Mailbox};
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::lifecycle::{SuccessorSpiritFactory, UpgradeOrchestrator};
use maos_kernel_core::memory::{
    principal::PrincipalNamespaceIndex, private::PrivateMemoryStore, shared::SharedMemoryStore,
    MemoryManagerAdapter,
};
use maos_kernel_core::scheduler::{
    control_block::SpiritManifestBundle, hook_dispatch::HookDispatcher, SpiritSchedulerAdapter,
};
use maos_kernel_core::security::{
    manifest::{ClassSection, HotSwapManifestSection, LifecycleSection, MigratesFromSection},
    RingCryptoProvider,
};
use maos_kernel_core::telemetry::{iac_rt::IacRtMetrics, TelemetryStreamAdapter};

pub struct CorpusSpirit;

impl maos_spirit_abi::lifecycle::Spirit for CorpusSpirit {
    fn snapshot(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) -> Vec<u8> {
        b"corpus-state".to_vec()
    }

    fn migrate(
        &self,
        _ctx: &mut maos_spirit_abi::ctx::Ctx,
        predecessor_state: &[u8],
    ) -> Result<Vec<u8>, maos_spirit_abi::lifecycle::MigratorError> {
        Ok(predecessor_state.to_vec())
    }
}

pub struct UpgradeHarness {
    pub scheduler: Arc<SpiritSchedulerAdapter>,
    pub orchestrator: Arc<UpgradeOrchestrator>,
    pub journal: Arc<JournalAdapter>,
    pub transparency_log: Arc<TransparencyLogAdapter>,
    _tmp: tempfile::TempDir,
}

impl UpgradeHarness {
    pub fn new(successor_factory: Arc<dyn SuccessorSpiritFactory>) -> Self {
        maos_kernel_core::capability::cap_tokens::init_monotonic_base();
        let tmp = tempfile::TempDir::new().expect("upgrade harness tempdir");
        let transparency_log = Arc::new(TransparencyLogAdapter::open_in_memory(0x5A54));
        let telemetry = Arc::new(IacRtMetrics::new());
        let capability = Arc::new(CapabilityRegistryAdapter::new(
            Arc::new(RingCryptoProvider),
            Ed25519SigningKey::new([0u8; 32]),
            0x5A54,
            Arc::new(PolicyTable::new()),
            cap_audit::channel().0,
            CapQuotaTracker::new(),
            Arc::new(WorkingMemoryStore::new()),
            Arc::new(TelemetryStreamAdapter::default()),
        ));
        let memory = Arc::new(MemoryManagerAdapter::new(
            Arc::new(PrivateMemoryStore::new(tmp.path().join("memory"), 4)),
            Arc::new(
                SharedMemoryStore::open(&tmp.path().join("memory.db"))
                    .expect("shared memory store"),
            ),
            Arc::new(
                PrincipalNamespaceIndex::open(&tmp.path().join("memory.db"))
                    .expect("principal namespace store"),
            ),
            Arc::clone(&transparency_log),
        ));
        let iac = Arc::new(IacBusAdapter::new(
            Arc::new(Mailbox::new(Arc::clone(&telemetry))),
            Arc::clone(&transparency_log),
        ));
        let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
        let scheduler = Arc::new(SpiritSchedulerAdapter::new(
            Arc::clone(&transparency_log),
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
            JournalAdapter::open(&tmp.path().join("journal.ndjson")).expect("upgrade journal"),
        );
        let dispatcher = Arc::new(HookDispatcher::new(
            Arc::clone(&transparency_log),
            Arc::clone(&telemetry),
        ));
        let hot_swap = Arc::new(HotSwapCoordinator::new(
            scheduler.scbs(),
            Arc::clone(&journal),
            Arc::clone(&transparency_log),
            halt_registry,
            capability,
            iac,
            dispatcher,
            Arc::clone(&telemetry),
            tmp.path().join("archives"),
        ));
        let orchestrator = Arc::new(UpgradeOrchestrator::new(
            Arc::clone(&scheduler),
            hot_swap,
            Arc::clone(&transparency_log),
            Arc::clone(&journal),
            telemetry,
            successor_factory,
        ));
        Self {
            scheduler,
            orchestrator,
            journal,
            transparency_log,
            _tmp: tmp,
        }
    }

    pub fn write_manifest(&self, name: &str, contents: &str) -> std::path::PathBuf {
        let path = self._tmp.path().join(name);
        std::fs::write(&path, contents).expect("write upgrade manifest");
        path
    }
}

pub fn parse_manifest_bundle(manifest: &str) -> Result<SpiritManifestBundle, String> {
    let root: toml::Value = toml::from_str(manifest).map_err(|error| error.to_string())?;
    let section = |name: &str| -> Result<Option<String>, String> {
        root.get(name)
            .map(|value| toml::to_string(value).map_err(|error| error.to_string()))
            .transpose()
    };
    let class = ClassSection::from_toml_str(
        &section("class")?.ok_or_else(|| "manifest is missing [class]".to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let lifecycle = section("lifecycle")?
        .map(|value| LifecycleSection::from_toml_str(&value).map_err(|error| error.to_string()))
        .transpose()?
        .unwrap_or_default();
    let hot_swap = section("hot_swap")?
        .map(|value| {
            HotSwapManifestSection::from_toml_str(&value).map_err(|error| error.to_string())
        })
        .transpose()?;
    let migrates_from = section("migrates_from")?
        .map(|value| MigratesFromSection::from_toml_str(&value).map_err(|error| error.to_string()))
        .transpose()?;

    Ok(SpiritManifestBundle {
        class: Some(class),
        lifecycle,
        hot_swap,
        migrates_from,
        ..SpiritManifestBundle::default()
    })
}
