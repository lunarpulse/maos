//! AC2 / AC8 — Orchestrator ships with an `on_idle` coordination pass within a
//! budgeted envelope, and its manifest declares the `autonomous-with-halt`
//! autonomy + dispatch output shape + sandbox envelope + a recall-preferring
//! halt policy, all validating against the AUTHORITATIVE `maos-manifest`
//! validators (no invented sections — AC8).
//!
//! Decision E: the Orchestrator declares NO `[capabilities.required]` — its
//! dispatch logic is deterministic and the halt path needs no inference. The
//! LIVE FR20 buffer-drain + FR21 dispatch paths are proven against the real
//! adapters in the other test files; this harness exercises the firing-side
//! hook + the manifest envelope.

use maos_spirit_sdk::spirit_test::{assert_no_deprecations, manifest_self_check, SpiritTest};
use orchestrator::__maos_spirit_vtable_Orchestrator;
use orchestrator::Orchestrator;

const MANIFEST: &str = include_str!("../manifest.toml");

#[test]
fn on_idle_fires_once_within_budget() {
    let spirit = Orchestrator::new("orchestrator");
    let vtable = __maos_spirit_vtable_Orchestrator();
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
        elapsed.as_millis() < 30_000,
        "on_idle must return before time_cap_seconds=30; took {elapsed:?}"
    );
    assert_no_deprecations!(report);
}

#[test]
fn manifest_self_check_is_well_formed() {
    let report =
        manifest_self_check(MANIFEST.as_bytes()).expect("Orchestrator manifest must parse");
    assert_eq!(report.class_name, "orchestrator");
    assert!(report.forms.iter().any(|f| f == "rust-inproc"));
    assert_eq!(report.trust_tier, "local");
    assert_eq!(report.sandbox_tier, "T2");
    // §6.7 — the founder-loop coordinator runs autonomous-with-halt.
    assert_eq!(report.posture_default, "autonomous-with-halt");
    assert_eq!(report.posture_allowed_max, "autonomous-with-halt");
    assert_eq!(report.budget_time_cap_seconds, Some(30));
    assert!(report.resources_cpu_max_pct.is_some());
    assert!(report.resources_memory_max_mb.is_some());
    // The dispatch output shape.
    for f in [
        "target_role",
        "goal",
        "has_distillate_ref",
        "distillation_depth",
    ] {
        assert!(
            report.output_shape_required_fields.iter().any(|x| x == f),
            "output_shape must require '{f}'"
        );
    }
    // Decision E — no inference / no MCP.
    assert_eq!(
        report.capabilities_required_count, 0,
        "Orchestrator declares no [capabilities.required] (Decision E)"
    );
    assert!(
        report.warnings.is_empty(),
        "no manifest warnings: {:?}",
        report.warnings
    );
}

#[test]
fn manifest_sections_parse_with_authoritative_validators() {
    use maos_manifest::manifest::{
        ClassSection, EpistemicPolicySection, PostureSection, SandboxConfig,
    };

    let _ = ClassSection::from_toml_str(&section(MANIFEST, "class")).expect("[class] valid");
    let posture =
        PostureSection::from_toml_str(&section(MANIFEST, "posture")).expect("[posture] valid");
    let _ = SandboxConfig::from_toml_str(&section(MANIFEST, "sandbox")).expect("[sandbox] valid");

    // §6.7 — autonomous-with-halt is a valid runtime-shift posture.
    use maos_manifest::manifest::Posture;
    assert_eq!(posture.default, Posture::AutonomousWithHalt);
    assert_eq!(posture.allowed_max, Posture::AutonomousWithHalt);

    // The recall-preferring halt policy parses against the authoritative validator.
    let ep = EpistemicPolicySection::from_toml_str(&section(MANIFEST, "epistemic_policy"))
        .expect("[epistemic_policy] valid");
    assert_eq!(ep.rules.len(), 2, "two recall-preferring halt rules");

    // Decision E — Orchestrator declares no capabilities.
    let v = value(MANIFEST);
    assert!(
        v.get("capabilities").is_none(),
        "Orchestrator declares no [capabilities] (Decision E)"
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
