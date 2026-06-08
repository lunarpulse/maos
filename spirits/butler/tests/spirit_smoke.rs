//! AC2 — Butler ships with `on_idle` anticipatory reasoning within a budgeted
//! envelope, and its manifest declares the on_idle hook with the required
//! resource/posture/sandbox/epistemic_policy envelope.
//!
//! Halt PRODUCTION (the `EpistemicHaltPayload` + `HaltReceipt`) is proven in
//! `tests/corpus_halt.rs` against the real kernel orchestrator — the SDK
//! spirit-test harness only SIMULATES halts, so the firing-side assertions
//! live here and the kernel-side halt assertions live there.

use butler::{Butler, ScenarioInput, CalendarEvent, EventStatus};
use butler::__maos_spirit_vtable_Butler;
use maos_spirit_sdk::spirit_test::{assert_no_deprecations, manifest_self_check, SpiritTest};

const MANIFEST: &str = include_str!("../manifest.toml");

#[test]
fn on_idle_fires_once_within_budget() {
    // A scenario with a confirmed overlap — on_idle runs the anticipatory pass.
    let scenario = ScenarioInput {
        calendar: vec![
            CalendarEvent { id: "a".into(), title: "Standup".into(), start_min: 540, end_min: 600, status: EventStatus::Confirmed },
            CalendarEvent { id: "b".into(), title: "Board call".into(), start_min: 570, end_min: 630, status: EventStatus::Confirmed },
        ],
        ..Default::default()
    };
    let spirit = Butler::with_scenario(scenario);
    let vtable = __maos_spirit_vtable_Butler();
    let mut harness = SpiritTest::new(&spirit, &vtable);
    harness.fixture_mut().invoke_on_idle = true;
    let report = harness.run();

    assert_eq!(
        report.base.hooks_fired.get("on_idle").copied().unwrap_or(0),
        1,
        "on_idle should fire exactly once"
    );
    // Bounded well within the manifest time_cap_seconds (30s). The hook is a
    // single linear pass; assert it returned in << the cap.
    let elapsed = report
        .base
        .elapsed_per_hook
        .get("on_idle")
        .copied()
        .unwrap_or_default();
    assert!(
        elapsed.as_millis() < 30_000,
        "on_idle must return before time_cap_seconds=30 (no BudgetExceeded); took {elapsed:?}"
    );
    assert_no_deprecations!(report);
}

#[test]
fn on_idle_with_no_pending_is_no_op() {
    // The production default: Butler::new() has pending: None, so on_idle
    // should fire but do nothing (no panic, no infinite loop, no side effects).
    let spirit = Butler::new();
    let vtable = __maos_spirit_vtable_Butler();
    let mut harness = SpiritTest::new(&spirit, &vtable);
    harness.fixture_mut().invoke_on_idle = true;
    let report = harness.run();
    assert_eq!(
        report.base.hooks_fired.get("on_idle").copied().unwrap_or(0),
        1,
        "on_idle should fire exactly once even with no pending scenario"
    );
    assert_no_deprecations!(report);
}

#[test]
fn manifest_self_check_is_well_formed() {
    let report = manifest_self_check(MANIFEST.as_bytes()).expect("Butler manifest must parse");
    assert_eq!(report.class_name, "butler");
    assert!(report.forms.iter().any(|f| f == "rust-inproc"));
    assert_eq!(report.trust_tier, "local");
    assert_eq!(report.sandbox_tier, "T2");
    assert_eq!(report.posture_default, "assistive");
    // Story 8.11 / AC6 FORK D — Butler's autonomy ceiling is `autonomous-with-halt`
    // (it self-halts on belief_variance); this is the manifest-derived signal the
    // daemon's boot-loud check keys on. See manifest.toml [posture].
    assert_eq!(report.posture_allowed_max, "autonomous-with-halt");
    assert_eq!(report.budget_time_cap_seconds, Some(30));
    assert!(report.budget_context_window_size.is_some());
    assert!(report.resources_cpu_max_pct.is_some());
    assert!(report.resources_memory_max_mb.is_some());
    // §6.1 output shape.
    for f in ["pattern", "confidence", "evidence", "options"] {
        assert!(
            report.output_shape_required_fields.iter().any(|x| x == f),
            "output_shape must require '{f}'"
        );
    }
    assert!(report.warnings.is_empty(), "no manifest warnings: {:?}", report.warnings);
}

#[test]
fn manifest_sections_parse_with_authoritative_validators() {
    use maos_manifest::manifest::{
        CapabilitiesRequired, ClassSection, EpistemicPolicySection, PostureSection, SandboxConfig,
    };

    // [class]
    let class = section(MANIFEST, "class");
    let class = ClassSection::from_toml_str(&class).expect("[class] valid");
    let _ = class; // parsed == valid

    // [capabilities.required] — produces the McpCall scopes (Decision B) +
    // provider.complete.
    let caps_str = capabilities_required_section(MANIFEST);
    let caps = CapabilitiesRequired::from_toml_str(&caps_str).expect("[capabilities.required] valid");
    assert!(!caps.provider.complete.is_empty());
    let servers: Vec<&str> = caps.mcp.servers.iter().map(|s| s.name.as_str()).collect();
    assert!(servers.contains(&"calendar") && servers.contains(&"slack"), "MCP scopes declared: {servers:?}");

    // [posture]
    let _ = PostureSection::from_toml_str(&section(MANIFEST, "posture")).expect("[posture] valid");
    // [sandbox]
    let _ = SandboxConfig::from_toml_str(&section(MANIFEST, "sandbox")).expect("[sandbox] valid");

    // [epistemic_policy] — the 2 §6.1 halt rules with exact predicate keys.
    let ep = EpistemicPolicySection::from_toml_str(&epistemic_policy_section(MANIFEST))
        .expect("[epistemic_policy] valid");
    assert_eq!(ep.rules.len(), 2, "two halt rules");
    let tags: Vec<&str> = ep.rules.iter().map(|r| r.tag.as_str()).collect();
    assert!(tags.contains(&"belief_variance"));
    assert!(tags.contains(&"user_preference_drift"));
}

// ── TOML section extraction helpers (round-trip a sub-table through `toml`) ──

fn value(manifest: &str) -> toml::Value {
    toml::from_str(manifest).expect("manifest is valid TOML")
}

/// Re-serialize a top-level `[section]` so a section-level `from_toml_str`
/// (which expects the section body at the document root) can parse it.
fn section(manifest: &str, key: &str) -> String {
    let v = value(manifest);
    toml::to_string(v.get(key).unwrap_or_else(|| panic!("[{key}] present"))).unwrap()
}

/// `[capabilities.required]` lives under `capabilities`.
fn capabilities_required_section(manifest: &str) -> String {
    let v = value(manifest);
    let req = v
        .get("capabilities")
        .and_then(|c| c.get("required"))
        .expect("[capabilities.required] present");
    toml::to_string(req).unwrap()
}

/// `[epistemic_policy]` with its `[[epistemic_policy.rules]]` array-of-tables.
fn epistemic_policy_section(manifest: &str) -> String {
    section(manifest, "epistemic_policy")
}
