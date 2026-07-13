#![forbid(unsafe_code)]

//! Integration test: Cross-major migration path (AC2).
//!
//! Covers:
//! - migrates_from with matching version pattern.
//! - EMigratorMissing when no migrator declared.
//! - Version pattern mismatch.
//! - Migrator matches_version_pattern test.

use maos_kernel_core::hot_swap::migrator;

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
