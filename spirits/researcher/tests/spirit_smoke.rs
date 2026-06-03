//! AC2 / AC8 — Researcher ships with `on_idle` survey cognition within a
//! budgeted envelope, and its manifest declares the survey/output/posture/
//! sandbox/epistemic_policy envelope and validates against the AUTHORITATIVE
//! `maos-manifest` validators (no invented sections — AC8).
//!
//! The participant-scoped recall, the I11 chain, and the scalar.tap
//! subscription are PROVEN against the real kernel adapters in the other test
//! files (`recall_walker`, `distillation_i11`, `scalar_tap`); the SDK
//! spirit-test harness only exercises the firing-side hook + manifest envelope.

use researcher::{ClaimPayload, RecalledFrame, Researcher};
use researcher::__maos_spirit_vtable_Researcher;
use maos_spirit_sdk::spirit_test::{assert_no_deprecations, manifest_self_check, SpiritTest};

const MANIFEST: &str = include_str!("../manifest.toml");

fn claim_frame(id_byte: u8, claim_id: &str, conf: f32) -> RecalledFrame {
    let claim = ClaimPayload {
        claim_id: claim_id.into(),
        statement: "the effect is likely present".into(),
        topic: "t".into(),
        methodology_strength: 0.9,
        confidence: conf,
        load_bearing: true,
        polarity: true,
        hedges: vec!["likely".into()],
    };
    RecalledFrame {
        frame_id: [id_byte; 16],
        intent: "inform".into(),
        payload: serde_json::to_vec(&claim).unwrap(),
    }
}

#[test]
fn on_idle_fires_once_and_surveys_within_budget() {
    let spirit = Researcher::with_frames(vec![claim_frame(0x11, "c1", 0.95)]);
    let vtable = __maos_spirit_vtable_Researcher();
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
        elapsed.as_millis() < 60_000,
        "on_idle must return before time_cap_seconds=60; took {elapsed:?}"
    );
    // The survey produced a production-visible result.
    let out = spirit.last_output().expect("on_idle stored a survey output");
    assert_eq!(out.findings.len(), 1);
    assert_no_deprecations!(report);
}

#[test]
fn on_idle_with_no_pending_is_no_op() {
    let spirit = Researcher::new();
    let vtable = __maos_spirit_vtable_Researcher();
    let mut harness = SpiritTest::new(&spirit, &vtable);
    harness.fixture_mut().invoke_on_idle = true;
    let report = harness.run();
    assert_eq!(
        report.base.hooks_fired.get("on_idle").copied().unwrap_or(0),
        1,
        "on_idle should fire exactly once even with no pending frames"
    );
    assert!(spirit.last_output().is_none(), "no frames ⇒ no survey");
    assert_no_deprecations!(report);
}

#[test]
fn manifest_self_check_is_well_formed() {
    let report = manifest_self_check(MANIFEST.as_bytes()).expect("Researcher manifest must parse");
    assert_eq!(report.class_name, "researcher");
    assert!(report.forms.iter().any(|f| f == "rust-inproc"));
    assert_eq!(report.trust_tier, "local");
    assert_eq!(report.sandbox_tier, "T2");
    assert_eq!(report.posture_default, "assistive");
    assert_eq!(report.posture_allowed_max, "assistive");
    assert_eq!(report.budget_time_cap_seconds, Some(60));
    assert!(report.budget_context_window_size.is_some());
    assert!(report.resources_cpu_max_pct.is_some());
    assert!(report.resources_memory_max_mb.is_some());
    // §6.2 output shape.
    for f in ["findings", "open_questions", "confidence_map", "bibliography"] {
        assert!(
            report.output_shape_required_fields.iter().any(|x| x == f),
            "output_shape must require '{f}'"
        );
    }
    assert!(
        report.warnings.is_empty(),
        "no manifest warnings: {:?}",
        report.warnings
    );
}

#[test]
fn manifest_sections_parse_with_authoritative_validators() {
    use maos_manifest::manifest::{
        CapabilitiesRequired, ClassSection, EpistemicPolicySection, PostureSection, SandboxConfig,
    };

    // [class]
    let class = ClassSection::from_toml_str(&section(MANIFEST, "class")).expect("[class] valid");
    let _ = class;

    // [capabilities.required] — provider.complete + the 4 MCP servers (Decision B).
    let caps =
        CapabilitiesRequired::from_toml_str(&capabilities_required_section(MANIFEST)).expect(
            "[capabilities.required] valid",
        );
    assert!(!caps.provider.complete.is_empty());
    let servers: Vec<&str> = caps.mcp.servers.iter().map(|s| s.name.as_str()).collect();
    for expected in ["web", "arxiv", "github", "citation-graph"] {
        assert!(
            servers.contains(&expected),
            "MCP scope '{expected}' must be declared: {servers:?}"
        );
    }

    // [posture] — autonomy spectrum (NOT survey/hypothesize; see manifest comment).
    let _ = PostureSection::from_toml_str(&section(MANIFEST, "posture")).expect("[posture] valid");
    // [sandbox]
    let _ = SandboxConfig::from_toml_str(&section(MANIFEST, "sandbox")).expect("[sandbox] valid");

    // [epistemic_policy] — the 2 §6.2 halt rules with exact predicate keys.
    let ep = EpistemicPolicySection::from_toml_str(&section(MANIFEST, "epistemic_policy"))
        .expect("[epistemic_policy] valid");
    assert_eq!(ep.rules.len(), 2, "two halt rules");
    let tags: Vec<&str> = ep.rules.iter().map(|r| r.tag.as_str()).collect();
    assert!(tags.contains(&"methodology_conflict"));
    assert!(tags.contains(&"load_bearing_confidence"));
}

// ── TOML section extraction helpers ──────────────────────────────────────────

fn value(manifest: &str) -> toml::Value {
    toml::from_str(manifest).expect("manifest is valid TOML")
}

fn section(manifest: &str, key: &str) -> String {
    let v = value(manifest);
    toml::to_string(v.get(key).unwrap_or_else(|| panic!("[{key}] present"))).unwrap()
}

fn capabilities_required_section(manifest: &str) -> String {
    let v = value(manifest);
    let req = v
        .get("capabilities")
        .and_then(|c| c.get("required"))
        .expect("[capabilities.required] present");
    toml::to_string(req).unwrap()
}
