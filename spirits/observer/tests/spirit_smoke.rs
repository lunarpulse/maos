//! AC2 / AC6 — Observer ships with an `on_idle` watchdog pass within a budgeted
//! envelope, and its manifest declares the read-only `cautious` autonomy +
//! anomaly-surface output shape + sandbox envelope and validates against the
//! AUTHORITATIVE `maos-manifest` validators (no invented sections — AC6).
//!
//! Decision G: Observer declares NO `[capabilities.required]` (no inference at
//! v0.5; the scalar.tap subscription scope is kernel-granted) and NO
//! `[epistemic_policy]` (Observer emits no claims and never halts itself). The
//! live drift / structural / scalar.tap paths are PROVEN against the real
//! adapters in the other test files; this harness exercises the firing-side hook
//! + the manifest envelope.

use maos_domain::invariants::i7::ScalarTapEvent;
use maos_spirit_sdk::spirit_test::{assert_no_deprecations, manifest_self_check, SpiritTest};
use observer::__maos_spirit_vtable_Observer;
use observer::{
    DivergenceKind, DriftDirection, Observer, PrincipalScope, StructuralSignal, WatchThreshold,
    SANDBOX_BLOCK_FRAME_KIND,
};

const MANIFEST: &str = include_str!("../manifest.toml");

#[test]
fn on_idle_fires_once_and_surfaces_within_budget() {
    // Seed one drifting scalar + one structural suspect; the on_idle pass should
    // surface both (production-visible effect), well within time_cap_seconds=10.
    let spirit = Observer::watching(
        PrincipalScope::all(),
        vec![WatchThreshold::new("belief_variance", 0.7, DriftDirection::Above, 0.15).unwrap()],
    )
    .with_pending_scalars(vec![ScalarTapEvent {
        spirit_id: "mira".into(),
        tag: "belief_variance".into(),
        value: 0.66,
        timestamp: 1,
    }])
    .with_pending_signals(vec![StructuralSignal {
        frame_kind: SANDBOX_BLOCK_FRAME_KIND,
        subject: "mira".into(),
        kind: DivergenceKind::FdTableGrowth,
        magnitude: 0.82,
        detail: "fd count 412 vs declared 64".into(),
    }]);
    let vtable = __maos_spirit_vtable_Observer();
    let mut harness = SpiritTest::new(&spirit, &vtable);
    harness.fixture_mut().invoke_on_idle = true;
    let report = harness.run();

    assert_eq!(
        report.base.hooks_fired.get("on_idle").copied().unwrap_or(0),
        1,
        "on_idle should fire exactly once"
    );
    let elapsed = report
        .base
        .elapsed_per_hook
        .get("on_idle")
        .copied()
        .unwrap_or_default();
    assert!(
        elapsed.as_millis() < 10_000,
        "on_idle must return before time_cap_seconds=10; took {elapsed:?}"
    );
    let surfaces = spirit.last_surfaces();
    assert_eq!(
        surfaces.len(),
        2,
        "one drift early-warning + one structural suspect"
    );
    assert_no_deprecations!(report);
}

#[test]
fn on_idle_with_no_pending_is_no_op() {
    let spirit = Observer::default();
    let vtable = __maos_spirit_vtable_Observer();
    let mut harness = SpiritTest::new(&spirit, &vtable);
    harness.fixture_mut().invoke_on_idle = true;
    let report = harness.run();
    assert_eq!(
        report.base.hooks_fired.get("on_idle").copied().unwrap_or(0),
        1,
        "on_idle fires once even with nothing pending"
    );
    assert!(
        spirit.last_surfaces().is_empty(),
        "nothing pending ⇒ no surfaces"
    );
    assert_no_deprecations!(report);
}

#[test]
fn manifest_self_check_is_well_formed() {
    let report = manifest_self_check(MANIFEST.as_bytes()).expect("Observer manifest must parse");
    assert_eq!(report.class_name, "observer");
    assert!(report.forms.iter().any(|f| f == "rust-inproc"));
    assert_eq!(report.trust_tier, "local");
    assert_eq!(report.sandbox_tier, "T2");
    // Decision G — read-only `cautious` autonomy.
    assert_eq!(report.posture_default, "cautious");
    assert_eq!(report.posture_allowed_max, "cautious");
    assert_eq!(report.budget_time_cap_seconds, Some(10));
    assert!(report.resources_cpu_max_pct.is_some());
    assert!(report.resources_memory_max_mb.is_some());
    // The anomaly-surface output shape.
    for f in ["subject", "summary", "confidence", "anomaly_kind"] {
        assert!(
            report.output_shape_required_fields.iter().any(|x| x == f),
            "output_shape must require '{f}'"
        );
    }
    // Decision G — Observer declares no capabilities (no inference / no MCP).
    assert_eq!(
        report.capabilities_required_count, 0,
        "Observer declares no [capabilities.required] (Decision G)"
    );
    assert!(
        report.warnings.is_empty(),
        "no manifest warnings: {:?}",
        report.warnings
    );
}

#[test]
fn manifest_sections_parse_with_authoritative_validators() {
    use maos_manifest::manifest::{ClassSection, PostureSection, SandboxConfig};

    let class = ClassSection::from_toml_str(&section(MANIFEST, "class")).expect("[class] valid");
    let _ = class;
    let _ = PostureSection::from_toml_str(&section(MANIFEST, "posture")).expect("[posture] valid");
    let _ = SandboxConfig::from_toml_str(&section(MANIFEST, "sandbox")).expect("[sandbox] valid");

    // Decision G — these sections are deliberately ABSENT.
    let v = value(MANIFEST);
    assert!(
        v.get("capabilities").is_none(),
        "Observer declares no [capabilities] (Decision G)"
    );
    assert!(
        v.get("epistemic_policy").is_none(),
        "Observer declares no [epistemic_policy] — it emits no claims and never halts itself (Decision G)"
    );
}

// ── TOML section extraction helpers ──────────────────────────────────────────

fn value(manifest: &str) -> toml::Value {
    toml::from_str(manifest).expect("manifest is valid TOML")
}

fn section(manifest: &str, key: &str) -> String {
    let v = value(manifest);
    toml::to_string(v.get(key).unwrap_or_else(|| panic!("[{key}] present"))).unwrap()
}
