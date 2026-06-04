//! AC5 / AC8 — the Reviewer ships an `on_idle` review pass within a budgeted
//! envelope, and its manifest declares the `assistive` autonomy + critique
//! output shape + sandbox envelope, validating against the AUTHORITATIVE
//! `maos-manifest` validators. Decision E: NO `[capabilities.required]`.

use maos_spirit_sdk::spirit_test::{assert_no_deprecations, manifest_self_check, SpiritTest};
use reviewer::__maos_spirit_vtable_Reviewer;
use reviewer::{DesignUnderReview, Reviewer};

const MANIFEST: &str = include_str!("../manifest.toml");

#[test]
fn on_idle_fires_once_within_budget() {
    let design = DesignUnderReview {
        components: vec!["a-module".into()],
        interfaces: vec!["a_port".into()],
        risks: vec![],
    };
    let spirit = Reviewer::new("reviewer").with_pending_design(design);
    let vtable = __maos_spirit_vtable_Reviewer();
    let mut harness = SpiritTest::new(&spirit, &vtable);
    harness.fixture_mut().invoke_on_idle = true;
    let report = harness.run();
    assert_eq!(
        report.base.hooks_fired.get("on_idle").copied().unwrap_or(0),
        1,
        "on_idle should fire exactly once"
    );
    assert!(
        spirit.last_critique().is_some(),
        "on_idle produced a critique"
    );
    assert_no_deprecations!(report);
}

#[test]
fn manifest_self_check_is_well_formed() {
    let report = manifest_self_check(MANIFEST.as_bytes()).expect("Reviewer manifest must parse");
    assert_eq!(report.class_name, "reviewer");
    assert!(report.forms.iter().any(|f| f == "rust-inproc"));
    assert_eq!(report.trust_tier, "local");
    assert_eq!(report.sandbox_tier, "T2");
    assert_eq!(report.posture_default, "assistive");
    for f in ["findings", "verdict", "severity", "summary"] {
        assert!(
            report.output_shape_required_fields.iter().any(|x| x == f),
            "output_shape must require '{f}'"
        );
    }
    assert_eq!(
        report.capabilities_required_count, 0,
        "Reviewer declares no [capabilities.required] (Decision E)"
    );
    assert!(
        report.warnings.is_empty(),
        "no manifest warnings: {:?}",
        report.warnings
    );
}
