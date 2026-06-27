//! Story 10.5 AC1 (NFR-Test-10) — proven-red vectors for the skill-format
//! conformance gate (`check-skill-conformance`).
//!
//! Per Epic 9 §A1: proven-red as a dev-pass gate. This is the external
//! proven-red vector the Epic-10 retro §A2 re-review (finding **H2**) found
//! missing: the gate's failure branches — false self-reported booleans in the
//! journaled results, malformed results TOML, a corrupt "valid" fixture, and a
//! defeated proven-red guard — had no test, so *inverting the pass condition
//! would have stayed green*. Each vector below forces one malformation and
//! asserts the gate actually goes red (or, for the absent-results case, that it
//! stays advisory-green).
//!
//! Mirrors `story_10_2_proven_red.rs`: every vector runs the real `xtask`
//! binary against a self-contained fixture tree in a tempdir.
//!
//! The gate (`xtask/src/check_skill_conformance.rs`) resolves three relative
//! paths from the current dir:
//!   - `tests/fixtures/anthropic-skill/SKILL.md`          (valid — must parse + execute)
//!   - `tests/fixtures/anthropic-skill-invalid/SKILL.md`  (invalid — adapter must reject)
//!   - `docs/skill-conformance/results/skill-conformance-results.toml` (optional journaled results)
//! so each test lays down its own copy of that tree before invoking the gate.

use std::io::Write;

// ═══════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════

fn run_in_tempdir(
    subcommand: &str,
    fixture_setup: impl FnOnce(&std::path::Path),
) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    fixture_setup(dir.path());
    std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([subcommand, "--json"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run xtask")
}

fn write_file(root: &std::path::Path, rel: &str, content: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
}

const VALID_FIXTURE: &str = "tests/fixtures/anthropic-skill/SKILL.md";
const INVALID_FIXTURE: &str = "tests/fixtures/anthropic-skill-invalid/SKILL.md";
const RESULTS_PATH: &str = "docs/skill-conformance/results/skill-conformance-results.toml";

/// A minimal Anthropic-format skill the adapter accepts (non-empty `name`,
/// non-empty body) — stands in for `tests/fixtures/anthropic-skill/SKILL.md`.
const GOOD_VALID_SKILL: &str = "---\nname: proven-red-skill\ndescription: Minimal valid skill for the H2 proven-red harness.\n---\n\n# Proven Red Skill\n\nBody content sufficient to pass the non-empty-body check.\n";

/// A skill the adapter MUST reject (no `name` field → `ESkillSchema::EmptyName`)
/// — stands in for `tests/fixtures/anthropic-skill-invalid/SKILL.md`.
const GOOD_INVALID_SKILL: &str = "---\ndescription: Missing required 'name' field — must fail adapter validation.\n---\n\n# Invalid Skill\n\nIntentionally malformed (no name) for proven-red.\n";

/// Lay down both required fixtures (good valid + good invalid). Tests that want
/// to corrupt one of them overwrite it afterward.
fn lay_fixtures(root: &std::path::Path) {
    write_file(root, VALID_FIXTURE, GOOD_VALID_SKILL);
    write_file(root, INVALID_FIXTURE, GOOD_INVALID_SKILL);
}

/// Build a journaled `skill-conformance-results.toml` body with controllable
/// self-reported booleans. All fields the `ConformanceResults` deserializer
/// requires are present, so the TOML is well-formed by construction.
fn make_results(executed_without_kernel_modification: bool, abi_unchanged: bool) -> String {
    format!(
        r#"[conformance]
adapter_name = "anthropic_adapter"
source_format = "anthropic-skills-yaml-frontmatter"
target_format = "maos.skill.v1"
executed_without_kernel_modification = {executed_without_kernel_modification}
abi_unchanged = {abi_unchanged}
fixture_path = "tests/fixtures/anthropic-skill/SKILL.md"
conformance_date = "2026-06-27"
"#
    )
}

// ═══════════════════════════════════════════════════════════════════
// Vector (a): false boolean — executed_without_kernel_modification = false → hard-fail.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn skill_conformance_fails_on_false_executed_without_kernel_modification() {
    let out = run_in_tempdir("check-skill-conformance", |root| {
        lay_fixtures(root);
        write_file(root, RESULTS_PATH, &make_results(false, true));
    });
    assert!(
        !out.status.success(),
        "gate must hard-fail when executed_without_kernel_modification is false: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ═══════════════════════════════════════════════════════════════════
// Vector (b): false boolean — abi_unchanged = false → hard-fail.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn skill_conformance_fails_on_false_abi_unchanged() {
    let out = run_in_tempdir("check-skill-conformance", |root| {
        lay_fixtures(root);
        write_file(root, RESULTS_PATH, &make_results(true, false));
    });
    assert!(
        !out.status.success(),
        "gate must hard-fail when abi_unchanged is false: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ═══════════════════════════════════════════════════════════════════
// Vector (c): malformed results TOML → hard-fail.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn skill_conformance_fails_on_malformed_toml() {
    let out = run_in_tempdir("check-skill-conformance", |root| {
        lay_fixtures(root);
        write_file(root, RESULTS_PATH, "this is not valid TOML {{{{");
    });
    assert!(
        !out.status.success(),
        "gate must hard-fail on malformed results TOML"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Vector (d): corrupt "valid" fixture → hard-fail.
//
// The fixture the gate trusts as the valid conformance input is structurally
// corrupt (no YAML frontmatter fence → `ESkillSchema::MissingFence`). The
// adapter must reject it and the gate must error before it ever reaches the
// journaled-results branch.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn skill_conformance_fails_on_corrupt_valid_fixture() {
    let out = run_in_tempdir("check-skill-conformance", |root| {
        lay_fixtures(root);
        // Overwrite the valid fixture with a fence-less (corrupt) document.
        write_file(
            root,
            VALID_FIXTURE,
            "name: not-fenced\ndescription: no frontmatter fence\n\nBody.\n",
        );
    });
    assert!(
        !out.status.success(),
        "gate must hard-fail when the valid fixture is corrupt and the adapter rejects it"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Vector (e): defeated proven-red guard — the "invalid" fixture parses fine.
//
// If the supposedly-invalid fixture is replaced with a well-formed skill, the
// adapter accepts it and the gate's internal proven-red assertion ("invalid
// fixture MUST be rejected") must fire and hard-fail. This proves that guard is
// load-bearing, not decorative.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn skill_conformance_fails_when_invalid_fixture_is_accepted() {
    let out = run_in_tempdir("check-skill-conformance", |root| {
        lay_fixtures(root);
        // Overwrite the invalid fixture with a fully-valid skill.
        write_file(
            root,
            INVALID_FIXTURE,
            "---\nname: should-have-been-invalid\ndescription: This parses fine, so the proven-red guard must reject it.\n---\n\n# Not Actually Invalid\n\nBody.\n",
        );
    });
    assert!(
        !out.status.success(),
        "gate must hard-fail when the invalid fixture is accepted by the adapter (proven-red guard defeated)"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("PROVEN-RED"),
        "failure should be attributed to the proven-red guard: {combined}"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Vector (f): absent journaled results → advisory PASS.
//
// Fixtures present and well-formed, but no results TOML → the gate passes with
// `advisory: true` (the conformance run may still be pending).
// ═══════════════════════════════════════════════════════════════════

#[test]
fn skill_conformance_passes_advisory_when_results_absent() {
    let out = run_in_tempdir("check-skill-conformance", |root| {
        lay_fixtures(root);
        // No results TOML written.
    });
    assert!(
        out.status.success(),
        "gate should advisory-PASS when results are absent: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("gate must emit JSON on --json: {e}; stdout was: {stdout}"));
    assert_eq!(json["advisory"], true, "absent results should be advisory");
}

// ═══════════════════════════════════════════════════════════════════
// Vector (g): all fixtures good + results present with true booleans → PASS clean.
//
// The positive control: a well-formed valid fixture, a correctly-rejected
// invalid fixture, and journaled results with both booleans true → the gate
// passes non-advisory. Without this, a gate that fails unconditionally would
// also satisfy every red vector above.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn skill_conformance_passes_clean_with_valid_results() {
    let out = run_in_tempdir("check-skill-conformance", |root| {
        lay_fixtures(root);
        write_file(root, RESULTS_PATH, &make_results(true, true));
    });
    assert!(
        out.status.success(),
        "gate should PASS with good fixtures + true booleans: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("gate must emit JSON on --json: {e}; stdout was: {stdout}"));
    assert_eq!(json["passed"], true, "gate should report passed");
    assert_eq!(
        json["advisory"], false,
        "journaled-results pass should be non-advisory"
    );
}
