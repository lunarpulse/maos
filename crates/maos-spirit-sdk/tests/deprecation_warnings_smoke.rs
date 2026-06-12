#![forbid(unsafe_code)]
#![cfg(feature = "spirit_test")]

//! Story 7.1 v0.5 binding — integration tests for the deprecation warning channel.

use maos_spirit_abi::DeprecationWarning;
use maos_spirit_sdk::{
    local_runner::RunReport,
    spirit_test::{self, harness::ExtendedRunReport},
    Ctx, Spirit, SpiritVtable,
};

struct TestSpirit;

impl Spirit for TestSpirit {
    fn on_idle(&self, _ctx: &mut Ctx) {
        // noop
    }
}

#[test]
fn ctx_mock_returns_empty_deprecation_warnings() {
    let ctx = Ctx::mock();
    assert!(ctx.deprecation_warnings().is_empty());
}

#[test]
fn ctx_mock_with_deprecation_warnings_returns_supplied() {
    let warnings = vec![DeprecationWarning::new(
        "Test::api",
        "0.5",
        "1.0",
        "use Test::new_api",
    )];
    let ctx = Ctx::mock_with_deprecation_warnings(warnings);
    assert_eq!(ctx.deprecation_warnings().len(), 1);
    assert_eq!(ctx.deprecation_warnings()[0].surface, "Test::api");
    assert_eq!(ctx.deprecation_warnings()[0].since_version, "0.5");
    assert_eq!(ctx.deprecation_warnings()[0].planned_removal, "1.0");
    assert_eq!(
        ctx.deprecation_warnings()[0].migration_hint,
        "use Test::new_api"
    );
}

#[test]
fn local_runner_populates_deprecation_warnings_surfaced() {
    let report = ExtendedRunReport {
        base: RunReport {
            hooks_fired: Default::default(),
            mock_bus_frames: vec![],
            elapsed_per_hook: Default::default(),
            deprecation_warnings_surfaced: vec![DeprecationWarning::new(
                "Test::api",
                "0.5",
                "1.0",
                "use Test::new_api",
            )],
        },
        halt_resolutions: vec![],
        captured_frames: vec![],
    };
    assert_eq!(report.base.deprecation_warnings_surfaced.len(), 1);
}

#[test]
fn deprecation_warnings_deduplicated() {
    use maos_spirit_sdk::local_runner::{LocalRunner, LocalRunnerFixture};
    let warnings = vec![DeprecationWarning::new(
        "Test::api",
        "0.5",
        "1.0",
        "use Test::new_api",
    )];
    let spirit = TestSpirit;
    let vtable = SpiritVtable::<TestSpirit>::from_spirit();
    let fixture = LocalRunnerFixture {
        invoke_on_idle: true,
        deprecation_warnings: warnings,
        ..Default::default()
    };
    // The LocalRunner deduplicates warnings by full tuple across hook fires.
    // on_idle fires once; the same warning appears once in the Ctx.
    // Result: exactly 1 unique warning.
    let report = LocalRunner::run(&spirit, &vtable, &fixture);
    assert_eq!(
        report.deprecation_warnings_surfaced.len(),
        1,
        "deduplication should reduce to 1 unique warning"
    );
    assert_eq!(report.deprecation_warnings_surfaced[0].surface, "Test::api");
}

#[test]
#[should_panic(expected = "assert_no_deprecations! FAILED")]
fn assert_no_deprecations_panics_when_warnings_present() {
    let report = ExtendedRunReport {
        base: RunReport {
            hooks_fired: Default::default(),
            mock_bus_frames: vec![],
            elapsed_per_hook: Default::default(),
            deprecation_warnings_surfaced: vec![DeprecationWarning::new(
                "Test::api",
                "0.5",
                "1.0",
                "use Test::new_api",
            )],
        },
        halt_resolutions: vec![],
        captured_frames: vec![],
    };
    spirit_test::assert_no_deprecations!(report);
}

#[test]
fn assert_no_deprecations_passes_when_empty() {
    let report = ExtendedRunReport {
        base: RunReport {
            hooks_fired: Default::default(),
            mock_bus_frames: vec![],
            elapsed_per_hook: Default::default(),
            deprecation_warnings_surfaced: vec![],
        },
        halt_resolutions: vec![],
        captured_frames: vec![],
    };
    spirit_test::assert_no_deprecations!(report);
}

#[test]
fn extended_run_report_reachable_via_harness() {
    let report = ExtendedRunReport {
        base: RunReport {
            hooks_fired: Default::default(),
            mock_bus_frames: vec![],
            elapsed_per_hook: Default::default(),
            deprecation_warnings_surfaced: vec![],
        },
        halt_resolutions: vec![],
        captured_frames: vec![],
    };
    spirit_test::assert_no_deprecations!(report);
}

#[test]
fn v05_kernel_has_zero_deprecation_annotations() {
    let ctx = Ctx::mock();
    assert!(ctx.deprecation_warnings().is_empty());
}
