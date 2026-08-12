#![forbid(unsafe_code)]

//! Integration test: Cross-major migration path (AC2).
//!
//! Covers:
//! - migrates_from with matching version pattern.
//! - EMigratorMissing when no migrator declared.
//! - Version pattern mismatch.
//! - Migrator matches_version_pattern test.

use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex, RwLock,
};

use maos_domain::hot_swap::SchemaCompat;
use maos_kernel_core::hot_swap::{migrator, HotSwapCoordinator};
use maos_kernel_core::scheduler::control_block::{
    make_spirit_obj, ScbLifecycleState, SpiritControlBlock, SpiritManifestBundle,
};
use maos_kernel_core::scheduler::hook_dispatch::HookDispatcher;
use maos_kernel_core::security::manifest::{
    ClassSection, HotSwapManifestSection, LifecycleSection, MigratesFromSection, SchedulingSection,
};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_spirit_abi::ctx::Ctx;
use maos_spirit_abi::lifecycle::{MigratorError, Spirit, SwapInPayload};

struct MajorOnePredecessor;

impl Spirit for MajorOnePredecessor {
    fn snapshot(&self, _ctx: &mut Ctx) -> Vec<u8> {
        b"hello".to_vec()
    }
}

struct MajorTwoSuccessor {
    migrate_calls: Arc<AtomicU32>,
    migrate_input: Arc<Mutex<Vec<u8>>>,
    swap_in_payload: Arc<Mutex<Vec<u8>>>,
}

impl Spirit for MajorTwoSuccessor {
    fn migrate(&self, _ctx: &mut Ctx, predecessor_state: &[u8]) -> Result<Vec<u8>, MigratorError> {
        self.migrate_calls.fetch_add(1, Ordering::SeqCst);
        *self
            .migrate_input
            .lock()
            .expect("migrate input lock poisoned") = predecessor_state.to_vec();
        Ok(b"major-two-state".to_vec())
    }

    fn on_swap_in<'a>(&self, _ctx: &mut Ctx, payload: &SwapInPayload<'a>) {
        *self
            .swap_in_payload
            .lock()
            .expect("swap-in payload lock poisoned") = payload.predecessor_state.to_vec();
    }
}

fn class_section(version: &str) -> ClassSection {
    ClassSection {
        name: "migration-spirit".into(),
        version: version.into(),
        abi: "1.0".into(),
        manifest_schema_version: 1,
        min_substrate_version: "0.1.0".into(),
        forms: vec!["rust-inproc".into()],
        trust_tier: "local".into(),
        description: "adjacent-major migration regression".into(),
    }
}

#[tokio::test]
async fn adjacent_upward_major_reaches_migrator_and_delivers_migrated_state() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();

    let tmp = tempfile::TempDir::new().expect("tempdir");
    let tl = Arc::new(
        maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open_in_memory(0xC0DE),
    );
    let telemetry = Arc::new(IacRtMetrics::new());
    let capability = Arc::new(
        maos_kernel_core::capability::CapabilityRegistryAdapter::new(
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0u8; 32]),
            0xC0DE,
            Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new()),
            maos_kernel_core::capability::cap_audit::channel().0,
            maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
            Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
            Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
        ),
    );
    let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
        Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&telemetry))),
        Arc::clone(&tl),
    ));
    let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
    let dispatcher = Arc::new(HookDispatcher::new(Arc::clone(&tl), Arc::clone(&telemetry)));
    let journal = Arc::new(
        maos_kernel_core::journal::JournalAdapter::open(&tmp.path().join("journal.ndjson"))
            .expect("journal open"),
    );

    let predecessor_manifest = SpiritManifestBundle {
        scheduling: SchedulingSection::default(),
        lifecycle: LifecycleSection {
            enabled_hooks: vec!["snapshot".into()],
        },
        class: Some(class_section("1.0.0")),
        hot_swap: Some(HotSwapManifestSection {
            state_schema_uri: "urn:maos:test:state:v1".into(),
            state_schema_version: 0x0001_0001,
        }),
        ..Default::default()
    };
    let predecessor_pid = 7;
    let predecessor_scb = Arc::new(SpiritControlBlock::new(
        predecessor_pid,
        "migration-spirit".into(),
        predecessor_manifest,
        make_spirit_obj(MajorOnePredecessor),
        0xC0DE,
    ));
    predecessor_scb
        .state
        .store(ScbLifecycleState::Running as u8, Ordering::Release);
    let spirits = Arc::new(RwLock::new(BTreeMap::from([(
        predecessor_pid,
        predecessor_scb,
    )])));

    let coordinator = HotSwapCoordinator::new(
        Arc::clone(&spirits),
        journal,
        tl,
        halt_registry,
        capability,
        iac,
        dispatcher,
        telemetry,
        tmp.path().join("archives"),
    );

    let migrate_calls = Arc::new(AtomicU32::new(0));
    let migrate_input = Arc::new(Mutex::new(Vec::new()));
    let swap_in_payload = Arc::new(Mutex::new(Vec::new()));
    let successor = MajorTwoSuccessor {
        migrate_calls: Arc::clone(&migrate_calls),
        migrate_input: Arc::clone(&migrate_input),
        swap_in_payload: Arc::clone(&swap_in_payload),
    };
    let successor_manifest = SpiritManifestBundle {
        scheduling: SchedulingSection::default(),
        lifecycle: LifecycleSection {
            enabled_hooks: vec!["migrate".into(), "on_swap_in".into()],
        },
        class: Some(class_section("2.0.0")),
        hot_swap: Some(HotSwapManifestSection {
            state_schema_uri: "urn:maos:test:state:v2".into(),
            state_schema_version: 0x0002_0001,
        }),
        migrates_from: Some(MigratesFromSection {
            versions: vec!["1.0.0".into()],
        }),
        ..Default::default()
    };

    let result = coordinator
        .initiate_swap(
            "migration-spirit",
            &successor_manifest,
            make_spirit_obj(successor),
        )
        .await
        .expect("adjacent upward major must reach the migrator");

    assert_eq!(result.schema_compat, SchemaCompat::CrossMajor);
    assert_eq!(migrate_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        migrate_input
            .lock()
            .expect("migrate input lock poisoned")
            .as_slice(),
        b"hello"
    );
    assert_eq!(
        swap_in_payload
            .lock()
            .expect("swap-in payload lock poisoned")
            .as_slice(),
        b"major-two-state"
    );
}

#[test]
fn version_pattern_wildcard_matches_same_major_minor() {
    assert!(migrator::matches_version_pattern("0.3.x", "0.3.1"));
    assert!(migrator::matches_version_pattern("0.3.x", "0.3.99"));
}

#[test]
fn version_pattern_exact_match() {
    assert!(migrator::matches_version_pattern("0.3.1", "0.3.1"));
}

#[test]
fn version_pattern_rejects_different_major() {
    assert!(!migrator::matches_version_pattern("0.3.x", "1.3.1"));
}

#[test]
fn version_pattern_rejects_different_minor() {
    assert!(!migrator::matches_version_pattern("0.3.x", "0.4.1"));
}

#[test]
fn version_pattern_rejects_exact_mismatch() {
    assert!(!migrator::matches_version_pattern("0.3.1", "0.3.2"));
}

// Story 12.5 §A7 missing-hop reflex: distinct from the cohort resolver's
// plan-time `ECohortNoMigrationPath`, the kernel's `run_migrator` refuses a
// RESOLVED hop whose migrator is absent at RUN time, naming the specific
// predecessor -> successor hop. This is the runtime leg the check-cohort-mesh
// gate owns for AC4(b) — the resolver validates only the declared candidate
// graph and cannot catch a hop whose migrator implementation is missing.
#[tokio::test]
async fn run_migrator_names_the_specific_absent_hop_at_run_time() {
    use std::sync::Arc;

    use maos_domain::hot_swap::HotSwapError;
    use maos_kernel_core::hot_swap::run_migrator;
    use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
    use maos_kernel_core::scheduler::control_block::{
        make_spirit_obj, SpiritControlBlock, SpiritManifestBundle,
    };
    use maos_kernel_core::scheduler::hook_dispatch::HookDispatcher;
    use maos_kernel_core::security::manifest::{ClassSection, MigratesFromSection};
    use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
    use maos_spirit_abi::lifecycle::Spirit;

    struct NoMigratorSpirit;
    impl Spirit for NoMigratorSpirit {}

    fn class_section(version: &str) -> ClassSection {
        ClassSection {
            name: "marcus-agent".into(),
            version: version.into(),
            abi: "1.0".into(),
            manifest_schema_version: 1,
            min_substrate_version: "0.1.0".into(),
            forms: vec!["rust-inproc".into()],
            trust_tier: "local".into(),
            description: "migration hop probe".into(),
        }
    }

    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xC0DE));
    let dispatcher = HookDispatcher::new(tl, Arc::new(IacRtMetrics::new()));

    // The walk has reached the intermediate 2.0 -> 3.0 hop (predecessor now 2.0).
    let mut predecessor_manifest = SpiritManifestBundle::default();
    predecessor_manifest.class = Some(class_section("2.0"));
    let scb = SpiritControlBlock::new(
        7,
        "marcus-agent".into(),
        predecessor_manifest,
        make_spirit_obj(NoMigratorSpirit),
        0xC0DE,
    );

    // Successor 3.0 declares a migrator only for source 1.0 — the migrator for
    // the resolved 2.0 -> 3.0 hop is absent from the candidate set.
    let mut successor_manifest = SpiritManifestBundle::default();
    successor_manifest.class = Some(class_section("3.0"));
    successor_manifest.migrates_from = Some(MigratesFromSection {
        versions: vec!["1.0".into()],
    });

    let error = run_migrator(
        &dispatcher,
        &scb,
        &make_spirit_obj(NoMigratorSpirit),
        b"predecessor-state",
        &successor_manifest,
        "2.0",
    )
    .await
    .expect_err("a resolved hop whose migrator is absent must refuse at run time");

    match error {
        HotSwapError::EMigratorMissing {
            predecessor_version,
            successor_version,
            ..
        } => {
            assert_eq!(
                predecessor_version, "2.0",
                "EMigratorMissing must name the specific absent hop's predecessor"
            );
            assert_eq!(
                successor_version, "3.0",
                "EMigratorMissing must name the specific absent hop's successor"
            );
        }
        other => panic!("expected EMigratorMissing naming the 2.0 -> 3.0 hop, got {other:?}"),
    }
}
