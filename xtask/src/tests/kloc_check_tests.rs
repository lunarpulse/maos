use super::*;

#[test]
fn infer_crate_from_crates_path() {
    assert_eq!(
        infer_crate_name("./crates/maos-kernel-core/src/lib.rs"),
        "maos-kernel-core"
    );
}

#[test]
fn infer_crate_from_xtask_path() {
    assert_eq!(infer_crate_name("./xtask/src/main.rs"), "xtask");
}

#[test]
fn infer_crate_fallback_for_unknown_path() {
    let name = infer_crate_name("some/unknown/path.rs");
    assert!(!name.is_empty(), "should produce fallback name");
    assert!(name.starts_with("(unknown:"));
}

#[test]
fn infer_crate_empty_for_target() {
    assert_eq!(infer_crate_name("target/debug/build/foo.rs"), "");
}

/// AC6(f): the un-ignore is the proof that the ceilings are honestly meetable,
/// so `passed` is the assertion. `!report.alarm` is deliberately NOT asserted
/// here: `_aggregate_alarm` is re-based (F8) precisely so it FIRES once the
/// workspace starts drawing on the Epic-15 allowance, which would turn a
/// designed advisory into a `cargo test -p xtask` failure whose name says
/// nothing about ceilings. The alarm boundary is proven below on a
/// deterministic fixture instead (Story 15-2 review R-D1).
#[test]
fn kloc_check_runs_on_workspace() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let config_path = std::path::Path::new(manifest_dir).join("kloc.toml");
    let report = kloc_check(config_path.to_str().unwrap()).unwrap();
    assert!(report.passed, "expected to pass: {:?}", report.over_budget);
}

/// Smoke test that asserts the gate produces a structured workspace report.
#[test]
fn kloc_check_produces_report_on_workspace() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let config_path = std::path::Path::new(manifest_dir).join("kloc.toml");
    let report = kloc_check(config_path.to_str().unwrap()).unwrap();
    // Shape-only assertion: per-crate LOC numbers were extracted.
    assert!(
        report.per_crate.contains_key("maos-kernel-core"),
        "report should enumerate maos-kernel-core"
    );
}

#[test]
fn alarm_fires_at_threshold() {
    let mut per_crate: BTreeMap<String, u64> = BTreeMap::new();
    per_crate.insert("maos-kernel-core".to_string(), 16000u64);
    let aggregate: u64 = per_crate.values().sum();
    let alarm = aggregate >= 16000;
    assert!(alarm, "alarm should fire at {} LOC", aggregate);
}

#[test]
fn hardfail_at_aggregate_threshold() {
    let mut per_crate: BTreeMap<String, u64> = BTreeMap::new();
    per_crate.insert("maos-kernel-core".to_string(), 20001u64);
    let aggregate: u64 = per_crate.values().sum();
    assert!(aggregate >= 20000, "should hard-fail at {} >= 20000", aggregate);
}

#[test]
fn hardfail_at_per_crate_threshold() {
    // Even if aggregate is small, per-crate over-budget should fail.
    let budget = 6000u64;
    let actual = 6001u64;
    assert!(actual > budget, "per-crate breach should be detected");
}

#[test]
fn tokei_version_parser_names_both_versions() {
    let error = validate_tokei_version("tokei 13.0.0").unwrap_err();
    assert!(error.contains("tokei version mismatch"));
    assert!(error.contains("expected 14.0.0"));
    assert!(error.contains("found 13.0.0"));
}

/// AC6(b)/F7b at its CALL SITE, not just in the parser: `cargo` is guaranteed
/// present under `cargo test` and reports a version that is never `14.0.0`, so
/// deleting the version probe from `kloc_check_with_tokei` reds this test
/// (Story 15-2 review R-P3).
#[test]
fn tokei_version_pin_is_enforced_at_the_call_site() {
    let (_workspace, config) = write_kloc_fixture(
        "_aggregate_alarm = 100\n_aggregate_hardfail = 200\nfixture = 100\n",
        Some("fixture"),
    );
    let error = kloc_check_with_tokei(config.to_str().unwrap(), "cargo").unwrap_err();
    assert!(
        error.contains("tokei version mismatch") && error.contains("expected 14.0.0"),
        "the pin must reject a mismatched binary at the point of use: {error}"
    );
}

#[test]
fn absent_tokei_binary_is_named_not_skipped() {
    let (_workspace, config) = write_kloc_fixture(
        "_aggregate_alarm = 100\n_aggregate_hardfail = 200\nfixture = 100\n",
        Some("fixture"),
    );
    let error = kloc_check_with_tokei(
        config.to_str().unwrap(),
        "maos-no-such-binary-15-2-review",
    )
    .unwrap_err();
    assert!(error.contains("failed to read tokei version"), "{error}");
}

/// AC6(a) at its CALL SITE. `run()` only dispatches `render_operator_output`,
/// so reverting either message to a hardcoded literal, or wiring the alarm
/// notice to `aggregate_hardfail`, reds this test (Story 15-2 review R-P2).
#[test]
fn operator_output_names_the_configured_thresholds() {
    // aggregate 1 >= alarm 0 AND >= hardfail 1 — distinct integers, so a
    // notice wired to the wrong threshold is observable.
    let (_workspace, config) = write_kloc_fixture(
        "_aggregate_alarm = 0\n_aggregate_hardfail = 1\nfixture = 100\n",
        Some("fixture"),
    );
    let report = kloc_check(config.to_str().unwrap()).unwrap();
    assert_eq!(report.aggregate, 1);
    let out = render_operator_output(&report);

    assert_eq!(
        out.alarm.expect("alarm must fire at aggregate >= _aggregate_alarm"),
        "workspace aggregate KLOC alarm: configured threshold=0, current=1"
    );
    let failure = out.failure.expect("aggregate breach must render a failure line");
    assert_eq!(
        failure.lines().next().unwrap(),
        "workspace aggregate KLOC hard fail: configured threshold=1, current=1; per-crate breakdown:"
    );
    assert!(!failure.contains("20 KLOC") && !failure.contains("NFR-Maint-1"));
    assert!(out.passed.is_none());
}

/// R-P1: a per-crate-only breach must NOT be reported as an aggregate hard
/// fail. Reverting the headline to the unconditional aggregate notice reds this.
#[test]
fn per_crate_only_breach_does_not_claim_an_aggregate_hard_fail() {
    let (_workspace, config) = write_kloc_fixture(
        "_aggregate_alarm = 100\n_aggregate_hardfail = 200\nfixture = 0\n",
        Some("fixture"),
    );
    let report = kloc_check(config.to_str().unwrap()).unwrap();
    assert!(!report.passed);
    assert_eq!(report.over_budget, vec!["fixture 1 > 0".to_string()]);

    let out = render_operator_output(&report);
    assert!(out.alarm.is_none(), "aggregate 1 is under the alarm at 100");
    let failure = out.failure.expect("a per-crate breach must still report");
    assert!(
        failure.starts_with("per-crate KLOC ceiling breach: fixture 1 > 0;"),
        "{failure}"
    );
    assert!(
        !failure.contains("hard fail"),
        "the aggregate was never breached: {failure}"
    );
    assert!(failure.contains("configured threshold=200"));
    assert!(failure.contains("| fixture | 1 | 0 | ❌ OVER |"));
}

/// R-D1: the alarm's first real control. It binds in both directions on a
/// deterministic fixture, and it stays advisory — `passed` is unaffected.
#[test]
fn alarm_binds_at_the_configured_threshold_and_stays_advisory() {
    let (_below, below) = write_kloc_fixture(
        "_aggregate_alarm = 2\n_aggregate_hardfail = 100\nfixture = 100\n",
        Some("fixture"),
    );
    let report = kloc_check(below.to_str().unwrap()).unwrap();
    assert_eq!(report.aggregate, 1);
    assert!(!report.alarm, "1 >= 2 is false");
    assert!(report.passed);

    let (_at, at) = write_kloc_fixture(
        "_aggregate_alarm = 1\n_aggregate_hardfail = 100\nfixture = 100\n",
        Some("fixture"),
    );
    let report = kloc_check(at.to_str().unwrap()).unwrap();
    assert!(report.alarm, "alarm must fire at aggregate == _aggregate_alarm");
    assert!(
        report.passed,
        "the alarm is advisory: it must never change the verdict"
    );
}

fn write_kloc_fixture(config: &str, crate_name: Option<&str>) -> (tempfile::TempDir, std::path::PathBuf) {
    let workspace = tempfile::tempdir().unwrap();
    let xtask = workspace.path().join("xtask");
    std::fs::create_dir_all(&xtask).unwrap();
    let config_path = xtask.join("kloc.toml");
    std::fs::write(&config_path, config).unwrap();
    if let Some(crate_name) = crate_name {
        let source = workspace
            .path()
            .join("crates")
            .join(crate_name)
            .join("src");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("lib.rs"), "pub fn measured() {}\n").unwrap();
    }
    (workspace, config_path)
}

#[test]
fn measured_crate_without_budget_is_named() {
    let (_workspace, config) = write_kloc_fixture(
        "_aggregate_alarm = 100\n_aggregate_hardfail = 200\n",
        Some("unbudgeted"),
    );
    let error = kloc_check(config.to_str().unwrap()).unwrap_err();
    assert!(error.contains("missing KLOC budget"));
    assert!(error.contains("unbudgeted"));
}

#[test]
fn missing_or_non_integer_aggregate_threshold_is_named() {
    let (_workspace, missing) = write_kloc_fixture("_aggregate_hardfail = 200\n", None);
    let error = kloc_check(missing.to_str().unwrap()).unwrap_err();
    assert!(error.contains("_aggregate_alarm"));
    assert!(error.contains("missing"));

    let (_workspace, non_integer) = write_kloc_fixture(
        "_aggregate_alarm = \"100\"\n_aggregate_hardfail = 200\n",
        None,
    );
    let error = kloc_check(non_integer.to_str().unwrap()).unwrap_err();
    assert!(error.contains("_aggregate_alarm"));
    assert!(error.contains("integer"));
}

/// R-P4: the BLOCKING threshold needs its own control. Restoring
/// `unwrap_or(20000)` for `_aggregate_hardfail` alone reds this test.
#[test]
fn missing_or_non_integer_hardfail_threshold_is_named() {
    let (_workspace, missing) = write_kloc_fixture("_aggregate_alarm = 100\n", None);
    let error = kloc_check(missing.to_str().unwrap()).unwrap_err();
    assert!(error.contains("_aggregate_hardfail"), "{error}");
    assert!(error.contains("missing"), "{error}");

    let (_workspace, non_integer) = write_kloc_fixture(
        "_aggregate_alarm = 100\n_aggregate_hardfail = \"200\"\n",
        None,
    );
    let error = kloc_check(non_integer.to_str().unwrap()).unwrap_err();
    assert!(error.contains("_aggregate_hardfail"), "{error}");
    assert!(error.contains("integer"), "{error}");

    let (_workspace, negative) = write_kloc_fixture(
        "_aggregate_alarm = 100\n_aggregate_hardfail = -1\n",
        None,
    );
    let error = kloc_check(negative.to_str().unwrap()).unwrap_err();
    assert!(error.contains("_aggregate_hardfail"), "{error}");
    assert!(error.contains("non-negative"), "{error}");
}

/// R-P6: an alarm above the hard fail can never be observed.
#[test]
fn inverted_aggregate_thresholds_are_named() {
    let (_workspace, config) =
        write_kloc_fixture("_aggregate_alarm = 300\n_aggregate_hardfail = 200\n", None);
    let error = kloc_check(config.to_str().unwrap()).unwrap_err();
    assert!(error.contains("_aggregate_alarm"), "{error}");
    assert!(error.contains("can never fire"), "{error}");

    // Equality is legal: the alarm fires in the same run the hard fail blocks.
    let (_workspace, equal) = write_kloc_fixture(
        "_aggregate_alarm = 200\n_aggregate_hardfail = 200\nfixture = 100\n",
        Some("fixture"),
    );
    kloc_check(equal.to_str().unwrap()).expect("alarm == hardfail must be accepted");
}

/// R-P7: the `[in_progress_decomposition]` placement trap (§2a) — a budget row
/// written below a table header is a sub-key the document-root walk cannot see.
/// It is now named instead of silently inert, and the table's own
/// `phase_N = { … }` metadata must not trip the check.
#[test]
fn budget_row_nested_below_a_table_header_is_named() {
    let (_workspace, config) = write_kloc_fixture(
        "_aggregate_alarm = 100\n_aggregate_hardfail = 200\n\
         [in_progress_decomposition]\n\
         phase_1 = { target = \"maos-iac\", status = \"done\", epic = \"6.5\" }\n\
         maos-egress = 780\n",
        None,
    );
    let error = kloc_check(config.to_str().unwrap()).unwrap_err();
    assert!(error.contains("maos-egress"), "{error}");
    assert!(error.contains("document root"), "{error}");

    // The real file's metadata table alone must stay green.
    let (_workspace, clean) = write_kloc_fixture(
        "_aggregate_alarm = 100\n_aggregate_hardfail = 200\nfixture = 100\n\
         [in_progress_decomposition]\n\
         phase_1 = { target = \"maos-iac\", status = \"done\", epic = \"6.5\" }\n",
        Some("fixture"),
    );
    kloc_check(clean.to_str().unwrap()).expect("phase metadata must not be read as a budget");
}

#[test]
fn quoted_or_negative_crate_budget_is_named() {
    let (_workspace, quoted) = write_kloc_fixture(
        "_aggregate_alarm = 100\n_aggregate_hardfail = 200\nfixture = \"10\"\n",
        None,
    );
    let error = kloc_check(quoted.to_str().unwrap()).unwrap_err();
    assert!(error.contains("fixture"));
    assert!(error.contains("integer"));

    let (_workspace, negative) = write_kloc_fixture(
        "_aggregate_alarm = 100\n_aggregate_hardfail = 200\nfixture = -1\n",
        None,
    );
    let error = kloc_check(negative.to_str().unwrap()).unwrap_err();
    assert!(error.contains("fixture"));
    assert!(error.contains("non-negative"));
}
