#![cfg(feature = "spirit_test")]

//! Smoke test for spirit_test SDK seed — exercises the harness +
//! halt resolution + manifest self-check + assertion macros.

use maos_spirit_sdk::{spirit, Ctx, Spirit};
use maos_spirit_sdk::spirit_test::{
    SpiritTest, HaltResolutionKind, manifest_self_check, ManifestSelfCheckViolation,
};
use maos_spirit_sdk::{assert_hook_fired, assert_halts_with, assert_manifest_well_formed};

pub struct TestSpirit;

#[spirit]
impl TestSpirit {
    fn on_idle(&self, _ctx: &mut Ctx) {}
}

#[test]
fn harness_runs_on_idle_and_records_resolution() {
    let s = TestSpirit;
    let v = __maos_spirit_vtable_TestSpirit();
    let mut h = SpiritTest::new(&s, v);
    h.fixture_mut().invoke_on_idle = true;
    h.resolve_halt("halt-001".to_string(), HaltResolutionKind::AcceptedHalt);
    let report = h.run();
    assert_hook_fired!(report, "on_idle", 1);
    assert_halts_with!(report, |k| matches!(k, &HaltResolutionKind::AcceptedHalt));
}

#[test]
fn provided_context_resolution_carries_bytes() {
    let s = TestSpirit;
    let v = __maos_spirit_vtable_TestSpirit();
    let mut h = SpiritTest::new(&s, v);
    h.resolve_halt(
        "halt-002".to_string(),
        HaltResolutionKind::ProvidedContext { context_bytes: b"clarification text".to_vec() },
    );
    let report = h.run();
    assert_eq!(report.halt_resolutions.len(), 1);
    match &report.halt_resolutions[0].kind {
        HaltResolutionKind::ProvidedContext { context_bytes } => {
            assert_eq!(context_bytes.as_slice(), b"clarification text");
        }
        other => panic!("unexpected kind: {other:?}"),
    }
}

#[test]
fn authorized_override_resolution_carries_marker() {
    let s = TestSpirit;
    let v = __maos_spirit_vtable_TestSpirit();
    let mut h = SpiritTest::new(&s, v);
    h.resolve_halt(
        "halt-003".to_string(),
        HaltResolutionKind::AuthorizedOverride { override_marker: b"OPS-OVERRIDE-42".to_vec() },
    );
    let report = h.run();
    assert_eq!(report.halt_resolutions.len(), 1);
    match &report.halt_resolutions[0].kind {
        HaltResolutionKind::AuthorizedOverride { override_marker } => {
            assert_eq!(override_marker.as_slice(), b"OPS-OVERRIDE-42");
        }
        other => panic!("unexpected kind: {other:?}"),
    }
}

#[test]
fn manifest_self_check_accepts_hello_spirit_shape() {
    let manifest = br#"
        [class]
        name = "hello"
        version = "0.1.0"
        forms = ["rust-inproc"]
        trust_tier = "local"

        [posture]
        default = "assistive"
        allowed_max = "assistive"

        [output_shape]
        required_fields = ["introduction"]

        [sandbox]
        tier = "T0"
    "#;
    let report = manifest_self_check(manifest).expect("should parse");
    assert_manifest_well_formed!(report);
    assert_eq!(report.class_name, "hello");
    assert_eq!(report.sandbox_tier, "T0");
}

#[test]
fn manifest_self_check_rejects_whitespace_in_required_field() {
    let manifest = br#"
        [class]
        name = "hello"
        version = "0.1.0"
        forms = ["rust-inproc"]
        trust_tier = "local"

        [posture]
        default = "assistive"
        allowed_max = "assistive"

        [output_shape]
        required_fields = ["with space"]

        [sandbox]
        tier = "T0"
    "#;
    let result = manifest_self_check(manifest);
    assert!(matches!(
        result,
        Err(ManifestSelfCheckViolation::InvalidValue { field: "output_shape.required_fields", .. })
    ));
}

#[test]
fn manifest_self_check_rejects_invalid_sandbox_tier() {
    let manifest = br#"
        [class]
        name = "hello"
        version = "0.1.0"
        forms = ["rust-inproc"]
        trust_tier = "local"

        [posture]
        default = "assistive"
        allowed_max = "assistive"

        [sandbox]
        tier = "T99"
    "#;
    let result = manifest_self_check(manifest);
    assert!(matches!(
        result,
        Err(ManifestSelfCheckViolation::InvalidValue { field: "sandbox.tier", .. })
    ));
}
