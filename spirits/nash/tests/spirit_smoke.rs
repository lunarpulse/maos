//! AC2 / AC8 — Nash ships with an `on_idle` architecture pass within a budgeted
//! envelope, and its manifest declares the `assistive` autonomy + proposal output
//! shape + sandbox envelope, and validates against the AUTHORITATIVE
//! `maos-manifest` validators (no invented sections).
//!
//! Decision I: Nash declares NO `[capabilities.required]` (no inference at v1.5).
//! Unlike Mira, Nash declares NO `[epistemic_policy]` — it architects, it does not
//! halt. Decision C: Nash maps to `SpiritRole::Worker`. The live A2A receive path
//! is PROVEN in `spirits/mira/tests/a2a_pairing.rs`; this harness exercises the
//! firing-side hook + the manifest envelope.

use maos_spirit_sdk::spirit_test::{assert_no_deprecations, manifest_self_check, SpiritTest};
use nash::__maos_spirit_vtable_Nash;
use nash::{AdvisoryInput, Nash};

const MANIFEST: &str = include_str!("../manifest.toml");

const SCENARIOS: &str = include_str!("fixtures/architect-scenarios.json");

#[test]
fn on_idle_fires_once_and_architects_within_budget() {
    let advisories: Vec<AdvisoryInput> =
        serde_json::from_str(SCENARIOS).expect("architect scenarios parse");
    assert_eq!(advisories.len(), 2, "two seeded advisories");
    let spirit = Nash::default().with_pending_advisories(advisories);
    let vtable = __maos_spirit_vtable_Nash();
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
        elapsed.as_millis() < 15_000,
        "on_idle must return before time_cap_seconds=15; took {elapsed:?}"
    );
    let proposals = spirit.last_proposals();
    assert_eq!(proposals.len(), 2, "both advisories architected");
    assert!(proposals.iter().all(|p| !p.proposed_fix.is_empty()));
    assert_no_deprecations!(report);
}

#[test]
fn on_idle_with_no_pending_is_no_op() {
    let spirit = Nash::default();
    let vtable = __maos_spirit_vtable_Nash();
    let mut harness = SpiritTest::new(&spirit, &vtable);
    harness.fixture_mut().invoke_on_idle = true;
    let report = harness.run();
    assert_eq!(
        report.base.hooks_fired.get("on_idle").copied().unwrap_or(0),
        1,
        "on_idle fires once even with nothing pending"
    );
    assert!(
        spirit.last_proposals().is_empty(),
        "nothing pending ⇒ no proposals"
    );
    assert_no_deprecations!(report);
}

#[test]
fn manifest_self_check_is_well_formed() {
    let report = manifest_self_check(MANIFEST.as_bytes()).expect("Nash manifest must parse");
    assert_eq!(report.class_name, "nash");
    assert!(report.forms.iter().any(|f| f == "rust-inproc"));
    assert_eq!(report.trust_tier, "local");
    assert_eq!(report.sandbox_tier, "T2");
    assert_eq!(report.posture_default, "assistive");
    assert_eq!(report.posture_allowed_max, "assistive");
    assert_eq!(report.budget_time_cap_seconds, Some(15));
    assert!(report.resources_cpu_max_pct.is_some());
    assert!(report.resources_memory_max_mb.is_some());
    for f in ["subject", "proposed_fix", "components", "confidence"] {
        assert!(
            report.output_shape_required_fields.iter().any(|x| x == f),
            "output_shape must require '{f}'"
        );
    }
    assert_eq!(
        report.capabilities_required_count, 0,
        "Nash declares no [capabilities.required] (Decision I)"
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

    let _ = ClassSection::from_toml_str(&section(MANIFEST, "class")).expect("[class] valid");
    let _ = PostureSection::from_toml_str(&section(MANIFEST, "posture")).expect("[posture] valid");
    let _ = SandboxConfig::from_toml_str(&section(MANIFEST, "sandbox")).expect("[sandbox] valid");

    // Decision I / Decision C — these sections are deliberately ABSENT: Nash does
    // no inference (no capabilities) and never halts (no epistemic_policy).
    let v = value(MANIFEST);
    assert!(
        v.get("capabilities").is_none(),
        "Nash declares no [capabilities] (Decision I)"
    );
    assert!(
        v.get("epistemic_policy").is_none(),
        "Nash declares no [epistemic_policy] — it architects, it does not halt (Decision C)"
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
