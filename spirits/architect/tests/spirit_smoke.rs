//! AC5 / AC8 — the Architect ships an `on_idle` design pass within a budgeted
//! envelope, and its manifest declares the `assistive` autonomy + design output
//! shape + sandbox envelope, validating against the AUTHORITATIVE
//! `maos-manifest` validators. Decision E: NO `[capabilities.required]` (no live
//! LLM at v0.8).

use architect::__maos_spirit_vtable_Architect;
use architect::Architect;
use maos_spirit_sdk::spirit_test::{assert_no_deprecations, manifest_self_check, SpiritTest};

const MANIFEST: &str = include_str!("../manifest.toml");

#[test]
fn on_idle_fires_once_within_budget() {
    let spirit = Architect::new("architect").with_pending_spec("parse; validate; persist");
    let vtable = __maos_spirit_vtable_Architect();
    let mut harness = SpiritTest::new(&spirit, &vtable);
    harness.fixture_mut().invoke_on_idle = true;
    let report = harness.run();
    assert_eq!(
        report.base.hooks_fired.get("on_idle").copied().unwrap_or(0),
        1,
        "on_idle should fire exactly once"
    );
    assert!(
        spirit.last_proposal().is_some(),
        "on_idle produced a proposal"
    );
    assert_no_deprecations!(report);
}

#[test]
fn manifest_self_check_is_well_formed() {
    let report = manifest_self_check(MANIFEST.as_bytes()).expect("Architect manifest must parse");
    assert_eq!(report.class_name, "architect");
    assert!(report.forms.iter().any(|f| f == "rust-inproc"));
    assert_eq!(report.trust_tier, "local");
    assert_eq!(report.sandbox_tier, "T2");
    assert_eq!(report.posture_default, "assistive");
    for f in ["components", "rationale", "interfaces", "risks"] {
        assert!(
            report.output_shape_required_fields.iter().any(|x| x == f),
            "output_shape must require '{f}'"
        );
    }
    // Decision E — no live LLM at v0.8.
    assert_eq!(
        report.capabilities_required_count, 0,
        "Architect declares no [capabilities.required] (Decision E)"
    );
    assert!(
        report.warnings.is_empty(),
        "no manifest warnings: {:?}",
        report.warnings
    );
}
