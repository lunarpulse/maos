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
fn body_only_claim_and_removed_or_invalid_cadence_cells_are_refused() {
    let doc = |claim: &str, cells: &str, body: &str| {
        format!("---\nid: I1\nclaim: {claim}\nenforcement_cadence:\n{cells}---\n{body}")
    };
    let old = doc("asserted", "  v1.0: fuzz\n  v1.5: fuzz\n", "");
    // Planted red: `claim: guaranteed` only in the body is no flip.
    let body_only = doc("asserted", "  v1.0: fuzz\n  v1.5: fuzz\n", "\nclaim: guaranteed\n");
    assert!(!phase_commitment(&old, &body_only).unwrap());
    let removed = doc("asserted", "  v1.0: fuzz\n", "");
    assert!(cadence_problem("I1", &old, &removed).unwrap().unwrap().contains("removed"));
    let invalid = doc("asserted", "  v1.0: fuzz\n  v1.5: bogus\n", "");
    assert!(cadence_problem("I1", &old, &invalid).unwrap().unwrap().contains("invalid"));
    let regressed = doc("asserted", "  v1.0: CI\n  v1.5: fuzz\n", "");
    assert!(cadence_problem("I1", &old, &regressed).unwrap().unwrap().contains("regressed"));
    assert!(cadence_problem("I1", &old, &old).unwrap().is_none());
}

#[test]
fn json_output_round_trip() {
    let report = LockReport {
        passed: false,
        claims: BTreeMap::new(),
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

#[test]
fn domain_invariant_modules_are_lock_gated() {
    let list = "/tmp/i21_10_domain_changed.txt";
    std::fs::write(list, "crates/maos-domain/src/invariants/i1.rs\n").unwrap();
    let report = invariant_lock(Some(list), None, None).unwrap();
    assert_eq!(report.touched_invariants, vec!["maos-domain-invariants".to_string()]);
    assert!(!report.passed, "a domain edit without corpus delta or commitment must fail");
    std::fs::remove_file(list).ok();
}

#[test]
fn phase_commitment_ignores_context_and_accepts_cadence_or_validated_flip() {
    let doc = |claim: &str, v10: &str| {
        format!("---\nid: I1\nclaim: {claim}\nowner: x\nenforcement_cadence:\n  v1.0: {v10}\n---\nbody\n")
    };
    // Planted red: only the claim basis / body moved, so `enforcement_cadence` can appear
    // as diff context but no cell changed and nothing flipped.
    let (a, b) = (doc("asserted", "fuzz"), doc("asserted", "fuzz").replace("body", "other body"));
    assert!(!phase_commitment(&a, &b).unwrap());
    assert!(!phase_commitment(&doc("guaranteed", "fuzz"), &doc("guaranteed", "fuzz")).unwrap());
    assert!(phase_commitment(&doc("asserted", "runtime"), &doc("asserted", "fuzz")).unwrap());
    assert!(phase_commitment(&doc("asserted", "fuzz"), &doc("guaranteed", "fuzz")).unwrap());
    assert!(!phase_commitment(&doc("guaranteed", "fuzz"), &doc("asserted", "fuzz")).unwrap());
}

fn review(login: &str, assoc: &str, state: &str, at: &str, oid: &str) -> serde_json::Value {
    serde_json::json!({
        "author": {"login": login}, "authorAssociation": assoc, "state": state,
        "submittedAt": at, "commit": {"oid": oid},
    })
}

#[test]
fn two_approvals_need_two_distinct_eligible_maintainers_on_the_current_head() {
    let pr = |reviews: Vec<serde_json::Value>| {
        serde_json::json!({"headRefOid": "head2", "reviews": reviews})
    };
    // Planted mutation: count every APPROVED entry (the pre-fix gate).
    // Same account twice is one identity.
    let twice = pr(vec![
        review("alice", "MEMBER", "APPROVED", "2026-11-22T01:00:00Z", "head2"),
        review("Alice", "MEMBER", "APPROVED", "2026-11-22T02:00:00Z", "head2"),
    ]);
    assert_eq!(eligible_approvals(&twice), 1);
    // An approval of an older revision is stale.
    let stale = pr(vec![
        review("alice", "MEMBER", "APPROVED", "2026-11-22T01:00:00Z", "head2"),
        review("bob", "OWNER", "APPROVED", "2026-11-22T01:30:00Z", "head1"),
    ]);
    assert_eq!(eligible_approvals(&stale), 1);
    // Approval then changes-requested: the later review wins.
    let superseded = pr(vec![
        review("alice", "MEMBER", "APPROVED", "2026-11-22T01:00:00Z", "head2"),
        review("bob", "OWNER", "APPROVED", "2026-11-22T01:30:00Z", "head2"),
        review("bob", "OWNER", "CHANGES_REQUESTED", "2026-11-22T02:00:00Z", "head2"),
    ]);
    assert_eq!(eligible_approvals(&superseded), 1);
    // A later plain comment does not supersede an approval; a non-maintainer never counts.
    let ok = pr(vec![
        review("alice", "MEMBER", "APPROVED", "2026-11-22T01:00:00Z", "head2"),
        review("bob", "COLLABORATOR", "APPROVED", "2026-11-22T01:30:00Z", "head2"),
        review("bob", "COLLABORATOR", "COMMENTED", "2026-11-22T02:00:00Z", "head2"),
        review("eve", "NONE", "APPROVED", "2026-11-22T02:00:00Z", "head2"),
    ]);
    assert_eq!(eligible_approvals(&ok), 2);
    assert_eq!(eligible_approvals(&serde_json::json!({"reviews": []})), 0);
}

#[test]
fn witness_files_and_ci_job_of_a_guaranteed_row_are_lock_scope() {
    let row = |claim: &str| {
        format!("---\nid: I1\nclaim: {claim}\nowner: x\nproperty_test: crates/a/tests/t.rs#t\nadversarial_corpus: crates/a/c.jsonl\nci_job: proofs\n---\n")
    };
    let wf = |step: &str| format!("jobs:\n  proofs:\n    steps:\n    - run: {step}\n  other:\n    steps:\n    - run: x\n");
    let set = |paths: &[&str]| paths.iter().map(|p| p.to_string()).collect::<BTreeSet<_>>();
    let (g, a) = (row("guaranteed"), row("asserted"));
    // Planted mutation: gate only register/predicate paths (the pre-fix scope).
    assert!(witness_touched(&set(&["crates/a/tests/t.rs"]), (&g, &g), (&wf("a"), &wf("a"))).unwrap());
    assert!(witness_touched(&set(&["crates/a/c.jsonl"]), (&g, &g), (&wf("a"), &wf("a"))).unwrap());
    assert!(witness_touched(&set(&[]), (&g, &g), (&wf("a"), &wf("b"))).unwrap());
    // The BASE row counts too: downgrading and weakening the witness in one step is scoped.
    assert!(witness_touched(&set(&["crates/a/tests/t.rs"]), (&g, &a), (&wf("a"), &wf("a"))).unwrap());
    // Untouched witness, other job edits, and asserted rows are out of scope.
    assert!(!witness_touched(&set(&["README.md"]), (&g, &g), (&wf("a"), &wf("a"))).unwrap());
    assert!(!witness_touched(&set(&["crates/a/tests/t.rs"]), (&a, &a), (&wf("a"), &wf("b"))).unwrap());
    let other = wf("a").replace("run: x", "run: y");
    assert!(!witness_touched(&set(&[]), (&g, &g), (&wf("a"), &other)).unwrap());
}

#[test]
fn candidate_cadence_cells_with_new_unrecognized_phase_keys_are_refused() {
    let doc = |cells: &str| format!("---\nid: I1\nclaim: asserted\nenforcement_cadence:\n{cells}---\n");
    let old = doc("  v1.0: fuzz\n  v1.5: fuzz\n");
    // Planted mutation: compare only the base's keys (new candidate keys skip validation).
    let new_key = doc("  v1.0: fuzz\n  v1.5: fuzz\n  v9.9: bogus\n");
    assert!(cadence_problem("I1", &old, &new_key).unwrap().unwrap().contains("unrecognized"));
    let new_phase_valid_value = doc("  v1.0: fuzz\n  v1.5: fuzz\n  v2.0: CI\n");
    assert!(cadence_problem("I1", &old, &new_phase_valid_value).unwrap().unwrap().contains("unrecognized"));
    let new_bad_value = doc("  v1.0: fuzz\n  v1.5: fuzz\n  v0.9: bogus\n");
    assert!(cadence_problem("I1", &old, &new_bad_value).unwrap().unwrap().contains("invalid"));
    // A legacy key already at the base stays valid; canonical phases may be added.
    let legacy = doc("  v0.1-alpha: CI\n  v1.0: fuzz\n  v1.5: fuzz\n");
    assert!(cadence_problem("I1", &legacy, &legacy).unwrap().is_none());
    let added = doc("  v0.9: runtime\n  v1.0: fuzz\n  v1.5: fuzz\n");
    assert!(cadence_problem("I1", &old, &added).unwrap().is_none());
}
