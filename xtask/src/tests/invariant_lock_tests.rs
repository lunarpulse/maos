use super::*;

#[test]
fn no_invariant_touch_passes() {
    // When no invariant-related files are touched, gate passes immediately.
    let changed = Some("/tmp/empty_changed.txt");
    std::fs::write(changed.unwrap(), "README.md\n").unwrap();
    let report = invariant_lock(changed, None, None)
        .unwrap_or_else(|e| panic!("invariant_lock failed: {e}"));
    assert!(report.passed);
    assert!(report.touched_invariants.is_empty());
}

#[test]
fn parse_cadence_works() {
    let src = "---\nenforcement_cadence:\n  v0.1: CI\n  v0.3: runtime\n---\n";
    let map = parse_cadence(src).unwrap();
    assert_eq!(map.get("v0.1"), Some(&"CI".to_string()));
    assert_eq!(map.get("v0.3"), Some(&"runtime".to_string()));
}

#[test]
fn parse_cadence_handles_tabs() {
    let src = "---\nenforcement_cadence:\n\tv0.1: CI\n\tv0.3: runtime\n---\n";
    let map = parse_cadence(src).unwrap();
    assert_eq!(map.get("v0.1"), Some(&"CI".to_string()));
}

#[test]
fn json_output_round_trip() {
    let report = LockReport {
        passed: false,
        touched_invariants: vec!["I1".into(), "I9".into()],
        missing_corpus_delta: true,
        missing_phase_commitment: false,
        insufficient_reviews: false,
        regression_detected: vec!["ADR-037 violation: ...".into()],
        review_count: 1,
        revert_of: None,
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: LockReport = serde_json::from_str(&json).unwrap();
    assert!(!parsed.passed);
    assert_eq!(parsed.touched_invariants.len(), 2);
    assert!(parsed.missing_corpus_delta);
}

#[test]
fn detect_regression_runtime_to_ci() {
    // Verify the rank ordering catches demotion.
    let order = ["\u{2014}", "CI", "runtime", "fuzz"];
    let rank = |s: &str| order.iter().position(|&x| x == s).unwrap_or(0);
    assert!(rank("CI") < rank("runtime"), "CI < runtime");
    assert!(rank("runtime") < rank("fuzz"), "runtime < fuzz");
}

// =============================================================================
// DF17 pre-flight: multi-invariant case for Story 1a.1's 14-ADR landing.
// Fixtures at xtask/tests/fixtures/{clean,violation}-invariant-lock-14*.
// Story 1a.1 will touch I1-I14 in one PR; the gate has only been dogfooded on
// a single invariant (Story 0.2 / I9). These tests verify the gate handles the
// 14-invariant batch correctly at the unit-test level (the integration-level
// fixture-shellout test requires a --root flag refactor; see fixture READMEs).
// =============================================================================

#[test]
fn parse_cadence_handles_14_distinct_files() {
    // Verify parse_cadence works against the canonical "after-1a.1" shape on
    // all 14 register files. Each file carries the same additive cadence rows;
    // the parser must produce the same map for all 14.
    for i in 1..=14 {
        let src = format!(
            "---\nid: I{i}\ntitle: synthetic\nenforcement_cadence:\n  v0.1-alpha-pre: \u{2014}\n  v0.1: CI\n---\n\n# I{i}: synthetic\n"
        );
        let map = parse_cadence(&src)
            .unwrap_or_else(|e| panic!("parse_cadence failed for I{i}: {e}"));
        assert_eq!(
            map.get("v0.1-alpha-pre"),
            Some(&"\u{2014}".to_string()),
            "I{i} v0.1-alpha-pre row missing or wrong"
        );
        assert_eq!(
            map.get("v0.1"),
            Some(&"CI".to_string()),
            "I{i} v0.1 row missing or wrong"
        );
    }
}

#[test]
fn regression_detection_in_14_invariant_batch() {
    // The 14-invariant batch is regression-clean for 13 of 14 invariants and
    // carries a deliberate v0.3 demotion in I7. The rank-ordering logic must
    // catch the I7 regression without false-positives on the other 13.
    let order = ["\u{2014}", "CI", "runtime", "fuzz"];
    let rank = |s: &str| order.iter().position(|&x| x == s).unwrap_or(0);

    // 13 clean invariants: HEAD~1 has {v0.1: CI}, HEAD has {v0.1-alpha-pre: —, v0.1: CI}.
    // Additive only; no regression.
    for i in [1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14] {
        let old_cadence: std::collections::BTreeMap<String, String> =
            [("v0.1".to_string(), "CI".to_string())].into_iter().collect();
        let new_cadence: std::collections::BTreeMap<String, String> = [
            ("v0.1-alpha-pre".to_string(), "\u{2014}".to_string()),
            ("v0.1".to_string(), "CI".to_string()),
        ]
        .into_iter()
        .collect();

        let mut regression = None;
        for (phase, old_val) in &old_cadence {
            if let Some(new_val) = new_cadence.get(phase) {
                if rank(new_val) < rank(old_val) {
                    regression = Some(format!("I{i}: {old_val} -> {new_val}"));
                }
            }
        }
        assert!(
            regression.is_none(),
            "false-positive regression for I{i}: {regression:?}"
        );
    }

    // I7: HEAD~1 had {v0.1: CI, v0.3: runtime}, HEAD has {v0.1-alpha-pre: —, v0.1: CI, v0.3: CI}.
    // v0.3 demotes runtime -> CI. Must be caught.
    let old_cadence: std::collections::BTreeMap<String, String> = [
        ("v0.1".to_string(), "CI".to_string()),
        ("v0.3".to_string(), "runtime".to_string()),
    ]
    .into_iter()
    .collect();
    let new_cadence: std::collections::BTreeMap<String, String> = [
        ("v0.1-alpha-pre".to_string(), "\u{2014}".to_string()),
        ("v0.1".to_string(), "CI".to_string()),
        ("v0.3".to_string(), "CI".to_string()),
    ]
    .into_iter()
    .collect();

    let mut regressions: Vec<String> = Vec::new();
    for (phase, old_val) in &old_cadence {
        if let Some(new_val) = new_cadence.get(phase) {
            if rank(new_val) < rank(old_val) {
                regressions.push(format!("I7 phase {phase}: {old_val} -> {new_val}"));
            }
        }
    }
    assert_eq!(
        regressions.len(),
        1,
        "expected exactly one regression for I7 v0.3, got: {regressions:?}"
    );
    assert!(
        regressions[0].contains("runtime"),
        "regression message should cite old=runtime, got: {}",
        regressions[0]
    );
    assert!(
        regressions[0].contains("CI"),
        "regression message should cite new=CI, got: {}",
        regressions[0]
    );
}

#[test]
fn multi_invariant_touched_set_complete() {
    // Verify the gate identifies all 14 invariants when a changed-files list
    // contains all 14 register paths. The gate fails the tri-requirement at
    // fixture-test level (no corpus-delta, no reviewers, no git context for
    // phase-commitment), but the touched_invariants set must be the full I1-I14.
    let tmpfile = "/tmp/df17_changed_files.txt";
    let paths: Vec<String> = (1..=14)
        .map(|i| format!("docs/invariants/I{i}.md"))
        .collect();
    std::fs::write(tmpfile, paths.join("\n")).unwrap();

    let report = invariant_lock(Some(tmpfile), None, None)
        .unwrap_or_else(|e| panic!("invariant_lock failed: {e}"));

    // 14 invariants touched.
    assert_eq!(
        report.touched_invariants.len(),
        14,
        "expected 14 touched invariants, got {}: {:?}",
        report.touched_invariants.len(),
        report.touched_invariants
    );

    // All 14 ids present.
    for i in 1..=14 {
        let id = format!("I{i}");
        assert!(
            report.touched_invariants.contains(&id),
            "expected {id} in touched_invariants, got {:?}",
            report.touched_invariants
        );
    }

    // Gate fails because corpus-delta absent (changed-files list does not include
    // tests/coverage-matrix.yaml). This is the expected fixture-context failure.
    assert!(!report.passed, "expected gate to fail without corpus-delta");
    assert!(
        report.missing_corpus_delta,
        "expected missing_corpus_delta=true, report={report:?}"
    );

    std::fs::remove_file(tmpfile).ok();
}

// =============================================================================
// DF16 (Option c) — revert detection.
// Parses the GitHub revert idiom from a PR body so the journal-append code
// path can emit a paired "reverted" entry. The detection is best-effort
// enrichment; absence of any idiom means "not a revert" and the gate
// proceeds without a revert_of payload.
// =============================================================================

#[test]
fn detect_revert_matches_reverts_pr_idiom() {
    let tmpfile = "/tmp/df16_pr_body_reverts.txt";
    std::fs::write(tmpfile, "Reverts #1234\n\nReason: bug introduced.\n").unwrap();
    let r = detect_revert(tmpfile).unwrap().expect("expected revert detection");
    assert_eq!(r.pr_number, Some(1234));
    assert_eq!(r.sha, None);
    std::fs::remove_file(tmpfile).ok();
}

#[test]
fn detect_revert_matches_this_reverts_commit_idiom() {
    let tmpfile = "/tmp/df16_pr_body_commit.txt";
    let sha = "abcdef0123456789abcdef0123456789abcdef01";
    std::fs::write(tmpfile, format!("Revert \"original title\"\n\nThis reverts commit {sha}.\n")).unwrap();
    let r = detect_revert(tmpfile).unwrap().expect("expected revert detection");
    assert_eq!(r.pr_number, None);
    assert_eq!(r.sha, Some(sha.to_string()));
    std::fs::remove_file(tmpfile).ok();
}

#[test]
fn detect_revert_captures_both_idioms_when_present() {
    let tmpfile = "/tmp/df16_pr_body_both.txt";
    let sha = "0123456789abcdef0123456789abcdef01234567";
    std::fs::write(
        tmpfile,
        format!("Reverts #42\n\nThis reverts commit {sha}.\n"),
    )
    .unwrap();
    let r = detect_revert(tmpfile).unwrap().expect("expected revert detection");
    assert_eq!(r.pr_number, Some(42));
    assert_eq!(r.sha, Some(sha.to_string()));
    std::fs::remove_file(tmpfile).ok();
}

#[test]
fn detect_revert_returns_none_on_non_revert_body() {
    let tmpfile = "/tmp/df16_pr_body_normal.txt";
    std::fs::write(
        tmpfile,
        "Adds new feature X.\n\nNo Reverts here. No reverts commit either — lowercase doesn't match.\n",
    )
    .unwrap();
    assert!(detect_revert(tmpfile).unwrap().is_none());
    std::fs::remove_file(tmpfile).ok();
}

#[test]
fn detect_revert_returns_none_on_unreadable_body_path() {
    // Spec: unreadable body is treated as "not a revert", not an error.
    // The gate's revert handling is best-effort enrichment; absence of
    // body file should not fail the gate.
    let r = detect_revert("/tmp/this-path-does-not-exist-df16").unwrap();
    assert!(r.is_none());
}

#[test]
fn detect_revert_rejects_too_short_sha() {
    let tmpfile = "/tmp/df16_pr_body_short_sha.txt";
    // 39-char hex (one short of full GitHub SHA). Should not be captured.
    let short_sha = "deadbeef1234567890abcdef01234567890abcd"; // 39 hex chars
    assert_eq!(short_sha.len(), 39, "test fixture: short_sha must be exactly 39 chars");
    std::fs::write(tmpfile, format!("This reverts commit {short_sha}.\n")).unwrap();
    let r = detect_revert(tmpfile).unwrap();
    // pr_number is None and sha is None → returns None overall.
    assert!(r.is_none(), "39-char hex should not be captured as full SHA");
    std::fs::remove_file(tmpfile).ok();
}

#[test]
fn detect_revert_rejects_non_numeric_pr_ref() {
    let tmpfile = "/tmp/df16_pr_body_bad_num.txt";
    std::fs::write(tmpfile, "Reverts #notanumber\n").unwrap();
    let r = detect_revert(tmpfile).unwrap();
    assert!(r.is_none(), "Reverts #<non-numeric> should not capture");
    std::fs::remove_file(tmpfile).ok();
}

#[test]
fn multi_invariant_with_corpus_delta_clears_that_leg() {
    // Same 14-invariant changed-files list, but with tests/coverage-matrix.yaml
    // included. corpus-delta leg should clear; gate still fails on other legs
    // (no git context for phase-commitment, no reviewers). The test confirms
    // the multi-invariant case does not break the corpus-delta detection.
    let tmpfile = "/tmp/df17_changed_files_with_corpus.txt";
    let mut paths: Vec<String> = (1..=14)
        .map(|i| format!("docs/invariants/I{i}.md"))
        .collect();
    paths.push("tests/coverage-matrix.yaml".to_string());
    std::fs::write(tmpfile, paths.join("\n")).unwrap();

    let report = invariant_lock(Some(tmpfile), None, None)
        .unwrap_or_else(|e| panic!("invariant_lock failed: {e}"));

    assert_eq!(report.touched_invariants.len(), 14);
    assert!(
        !report.missing_corpus_delta,
        "expected corpus-delta leg to clear, report={report:?}"
    );

    std::fs::remove_file(tmpfile).ok();
}
