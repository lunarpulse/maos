//! AC2 / AC8 — Mira ships with an `on_idle` diagnostic pass within a budgeted
//! envelope, and its manifest declares the `cautious` autonomy + diagnosis output
//! shape + sandbox envelope + the `[epistemic_policy]` halt rule, and validates
//! against the AUTHORITATIVE `maos-manifest` validators (no invented sections).
//!
//! Decision I: Mira declares NO `[capabilities.required]` (no inference at v1.5;
//! diagnosis is deterministic, Spirit-side). Decision C: Mira maps to
//! `SpiritRole::Worker` (set at registration, not a manifest field). The live A2A
//! / halt paths are PROVEN against the real adapters in `a2a_pairing.rs` +
//! `halt_bilateral.rs`; this harness exercises the firing-side hook + the
//! manifest envelope (incl. the diagnostic-confidence halt rule Decision C needs).

use maos_spirit_sdk::spirit_test::{assert_no_deprecations, manifest_self_check, SpiritTest};
use mira::__maos_spirit_vtable_Mira;
use mira::{AnomalySignal, Mira};

const MANIFEST: &str = include_str!("../manifest.toml");

const SCENARIOS: &str = include_str!("fixtures/diagnostic-scenarios.json");

#[test]
fn on_idle_fires_once_and_diagnoses_within_budget() {
    let signals: Vec<AnomalySignal> =
        serde_json::from_str(SCENARIOS).expect("diagnostic scenarios parse");
    assert_eq!(signals.len(), 2, "two seeded scenarios (known + unknown)");
    let spirit = Mira::default().with_pending_signals(signals);
    let vtable = __maos_spirit_vtable_Mira();
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
    let diagnoses = spirit.last_diagnoses();
    assert_eq!(diagnoses.len(), 2, "both scenarios diagnosed");
    // The known scenario is confidently diagnosed; the unknown-severe one reaches
    // Mira's halt boundary.
    assert!(
        diagnoses.iter().any(|d| !d.requires_halt),
        "known → no halt"
    );
    assert!(
        diagnoses.iter().any(|d| d.requires_halt),
        "unknown-severe → halt boundary"
    );
    assert_no_deprecations!(report);
}

#[test]
fn on_idle_with_no_pending_is_no_op() {
    let spirit = Mira::default();
    let vtable = __maos_spirit_vtable_Mira();
    let mut harness = SpiritTest::new(&spirit, &vtable);
    harness.fixture_mut().invoke_on_idle = true;
    let report = harness.run();
    assert_eq!(
        report.base.hooks_fired.get("on_idle").copied().unwrap_or(0),
        1,
        "on_idle fires once even with nothing pending"
    );
    assert!(
        spirit.last_diagnoses().is_empty(),
        "nothing pending ⇒ no diagnoses"
    );
    assert_no_deprecations!(report);
}

#[test]
fn manifest_self_check_is_well_formed() {
    let report = manifest_self_check(MANIFEST.as_bytes()).expect("Mira manifest must parse");
    assert_eq!(report.class_name, "mira");
    assert!(report.forms.iter().any(|f| f == "rust-inproc"));
    assert_eq!(report.trust_tier, "local");
    assert_eq!(report.sandbox_tier, "T2");
    assert_eq!(report.posture_default, "cautious");
    assert_eq!(report.posture_allowed_max, "cautious");
    assert_eq!(report.budget_time_cap_seconds, Some(10));
    assert!(report.resources_cpu_max_pct.is_some());
    assert!(report.resources_memory_max_mb.is_some());
    for f in ["subject", "finding", "severity", "confidence"] {
        assert!(
            report.output_shape_required_fields.iter().any(|x| x == f),
            "output_shape must require '{f}'"
        );
    }
    // Decision I — Mira declares no capabilities (no inference / no MCP).
    assert_eq!(
        report.capabilities_required_count, 0,
        "Mira declares no [capabilities.required] (Decision I)"
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
    let _ = PostureSection::from_toml_str(&section(MANIFEST, "posture")).expect("[posture] valid");
    let _ = SandboxConfig::from_toml_str(&section(MANIFEST, "sandbox")).expect("[sandbox] valid");

    // [epistemic_policy] — the diagnostic-confidence halt rule (Decision C: Mira
    // is a halt-capable Worker, unlike read-only Observer).
    let ep = EpistemicPolicySection::from_toml_str(&section(MANIFEST, "epistemic_policy"))
        .expect("[epistemic_policy] valid");
    assert_eq!(ep.rules.len(), 1, "one diagnostic-confidence halt rule");
    assert_eq!(ep.rules[0].tag, mira::DIAGNOSTIC_CONFIDENCE_TAG);

    // Decision I — [capabilities] deliberately ABSENT (no inference / no MCP).
    let v = value(MANIFEST);
    assert!(
        v.get("capabilities").is_none(),
        "Mira declares no [capabilities] (Decision I)"
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
