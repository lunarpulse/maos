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

/// The committed config must pass on the committed tree: the kernel row is honestly meetable.
#[test]
fn kloc_check_runs_on_workspace() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let config_path = std::path::Path::new(manifest_dir).join("kloc.toml");
    let report = kloc_check(config_path.to_str().unwrap()).unwrap();
    assert!(report.passed, "expected to pass: {:?}", report.over_budget);
}

/// The report enumerates every measured crate, not only the gated one.
#[test]
fn kloc_check_produces_report_on_workspace() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let config_path = std::path::Path::new(manifest_dir).join("kloc.toml");
    let report = kloc_check(config_path.to_str().unwrap()).unwrap();
    assert!(
        report.per_crate.contains_key("maos-kernel-core"),
        "report should enumerate maos-kernel-core"
    );
    assert!(
        report.per_crate.contains_key("xtask"),
        "report should enumerate non-kernel crates too"
    );
    assert_eq!(report.aggregate, report.per_crate.values().sum::<u64>());
}

#[test]
fn tokei_version_parser_names_both_versions() {
    let error = validate_tokei_version("tokei 13.0.0").unwrap_err();
    assert!(error.contains("tokei version mismatch"));
    assert!(error.contains("expected 14.0.0"));
    assert!(error.contains("found 13.0.0"));
}

/// The version pin at its CALL SITE, not just in the parser: `cargo` is guaranteed
/// present under `cargo test` and reports a version that is never `14.0.0`, so
/// deleting the version probe from `kloc_check_with_tokei` reds this test.
#[test]
fn tokei_version_pin_is_enforced_at_the_call_site() {
    let (_workspace, config) = write_kloc_fixture("maos-kernel-core = 100\n", &[]);
    let error = kloc_check_with_tokei(config.to_str().unwrap(), "cargo").unwrap_err();
    assert!(
        error.contains("tokei version mismatch") && error.contains("expected 14.0.0"),
        "the pin must reject a mismatched binary at the point of use: {error}"
    );
}

#[test]
fn absent_tokei_binary_is_named_not_skipped() {
    let (_workspace, config) = write_kloc_fixture("maos-kernel-core = 100\n", &[]);
    let error = kloc_check_with_tokei(config.to_str().unwrap(), "maos-no-such-binary-wp23")
        .unwrap_err();
    assert!(error.contains("failed to read tokei version"), "{error}");
}

/// Lays out `xtask/kloc.toml` plus `crates/<name>/src/lib.rs` holding `lines` one-line
/// functions for each `(name, lines)` pair, so tokei measures exactly those figures.
fn write_kloc_fixture(
    config: &str,
    crates: &[(&str, usize)],
) -> (tempfile::TempDir, std::path::PathBuf) {
    let workspace = tempfile::tempdir().unwrap();
    let xtask = workspace.path().join("xtask");
    std::fs::create_dir_all(&xtask).unwrap();
    let config_path = xtask.join("kloc.toml");
    std::fs::write(&config_path, config).unwrap();
    for (crate_name, lines) in crates {
        let source = workspace.path().join("crates").join(crate_name).join("src");
        std::fs::create_dir_all(&source).unwrap();
        let body: String = (0..*lines).map(|i| format!("pub fn f{i}() {{}}\n")).collect();
        std::fs::write(source.join("lib.rs"), body).unwrap();
    }
    (workspace, config_path)
}

/// Decision 11: growth outside the kernel is measured, never gated. A crate far larger than
/// the kernel passes, and the verdict stays green.
#[test]
fn non_kernel_growth_never_fails_the_gate() {
    let (_workspace, config) = write_kloc_fixture(
        "maos-kernel-core = 5\n",
        &[("maos-kernel-core", 2), ("maos-big", 500)],
    );
    let report = kloc_check(config.to_str().unwrap()).unwrap();
    assert_eq!(report.per_crate["maos-big"], 500);
    assert_eq!(report.aggregate, 502);
    assert!(report.passed, "{:?}", report.over_budget);
    assert!(report.over_budget.is_empty());
}

/// The kernel envelope is the one verdict: one line over the row fails, at the row passes.
#[test]
fn kernel_above_its_row_fails_and_at_its_row_passes() {
    let (_over, over) = write_kloc_fixture(
        "maos-kernel-core = 2\n",
        &[("maos-kernel-core", 3), ("maos-other", 1)],
    );
    let report = kloc_check(over.to_str().unwrap()).unwrap();
    assert!(!report.passed);
    assert_eq!(report.over_budget, vec!["maos-kernel-core 3 > 2".to_string()]);

    let out = render_operator_output(&report);
    assert!(out.passed.is_none());
    let failure = out.failure.expect("a kernel breach must render a failure line");
    assert!(
        failure.starts_with("kernel envelope breach: maos-kernel-core 3 > 2"),
        "{failure}"
    );
    assert!(out.table.contains("| maos-kernel-core | 3 | 2 OVER |"));

    let (_at, at) = write_kloc_fixture("maos-kernel-core = 3\n", &[("maos-kernel-core", 3)]);
    let report = kloc_check(at.to_str().unwrap()).unwrap();
    assert!(report.passed, "a kernel exactly at its row is inside the envelope");
}

/// The size table lists every measured crate, largest first, with the kernel's ceiling and
/// the total; the passing verdict line carries the kernel figure.
#[test]
fn size_table_lists_every_measured_crate_largest_first() {
    let (_workspace, config) = write_kloc_fixture(
        "maos-kernel-core = 10\n",
        &[
            ("maos-kernel-core", 4),
            ("maos-small", 1),
            ("maos-large", 7),
        ],
    );
    let report = kloc_check(config.to_str().unwrap()).unwrap();
    assert_eq!(report.kernel_ceiling, 10);
    let out = render_operator_output(&report);
    let rows: Vec<&str> = out.table.lines().skip(2).collect();
    assert_eq!(
        rows,
        vec![
            "| maos-large | 7 | - |",
            "| maos-kernel-core | 4 | 10 |",
            "| maos-small | 1 | - |",
            "| (total) | 12 | - |",
        ]
    );
    assert!(out.failure.is_none());
    assert_eq!(
        out.passed.as_deref(),
        Some("kloc-check: PASSED (maos-kernel-core 4 <= 10; aggregate=12 LOC, measured, not gated)")
    );
}

/// A kernel that measures nothing still has a table row and a verdict.
#[test]
fn absent_kernel_crate_is_reported_as_zero() {
    let (_workspace, config) =
        write_kloc_fixture("maos-kernel-core = 1\n", &[("maos-other", 2)]);
    let report = kloc_check(config.to_str().unwrap()).unwrap();
    assert_eq!(report.per_crate["maos-kernel-core"], 0);
    assert!(report.passed);
}

/// A per-crate row is refused, naming the key and the decision, so a removed ceiling cannot
/// be reintroduced as a number nothing reads.
#[test]
fn a_per_crate_ceiling_row_is_refused() {
    let (_workspace, config) = write_kloc_fixture(
        "maos-kernel-core = 10\nmaos-egress = 2000\n",
        &[("maos-kernel-core", 1)],
    );
    let error = kloc_check(config.to_str().unwrap()).unwrap_err();
    assert!(error.contains("`maos-egress`"), "{error}");
    assert!(error.contains("decision 11"), "{error}");
}

/// The retired aggregate thresholds are refused whatever their value, and before any
/// measurement.
#[test]
fn retired_aggregate_keys_are_refused() {
    for (key, config) in [
        (
            "_aggregate_alarm",
            "maos-kernel-core = 10\n_aggregate_alarm = 100\n",
        ),
        (
            "_aggregate_alarm",
            "maos-kernel-core = 10\n_aggregate_alarm = \"100\"\n",
        ),
        (
            "_aggregate_hardfail",
            "maos-kernel-core = 10\n_aggregate_hardfail = 200\n",
        ),
    ] {
        let (_workspace, path) = write_kloc_fixture(config, &[("maos-kernel-core", 1)]);
        let error = kloc_check(path.to_str().unwrap()).unwrap_err();
        assert!(error.contains(&format!("`{key}`")), "{error}");
        assert!(error.contains("retired"), "{error}");
    }
}

#[test]
fn missing_kernel_row_is_named() {
    let (_workspace, config) = write_kloc_fixture("", &[("maos-kernel-core", 1)]);
    let error = kloc_check(config.to_str().unwrap()).unwrap_err();
    assert!(error.contains("missing required `maos-kernel-core` row"), "{error}");
}

/// The `[in_progress_decomposition]` placement trap: a row written below a table header is a
/// sub-key the document-root walk cannot see. It is named instead of silently inert, and the
/// table's own `phase_N = { ... }` metadata must not trip the check.
#[test]
fn row_nested_below_a_table_header_is_named() {
    let (_workspace, config) = write_kloc_fixture(
        "[in_progress_decomposition]\n\
         phase_1 = { target = \"maos-iac\", status = \"done\", epic = \"6.5\" }\n\
         maos-kernel-core = 780\n",
        &[],
    );
    let error = kloc_check(config.to_str().unwrap()).unwrap_err();
    assert!(error.contains("maos-kernel-core"), "{error}");
    assert!(error.contains("document root"), "{error}");

    // The real file's metadata table alone must stay green.
    let (_workspace, clean) = write_kloc_fixture(
        "maos-kernel-core = 100\n\
         [in_progress_decomposition]\n\
         phase_1 = { target = \"maos-iac\", status = \"done\", epic = \"6.5\" }\n",
        &[("maos-kernel-core", 1)],
    );
    kloc_check(clean.to_str().unwrap()).expect("phase metadata must not be read as a budget");
}

#[test]
fn quoted_or_negative_kernel_row_is_named() {
    let (_workspace, quoted) = write_kloc_fixture("maos-kernel-core = \"10\"\n", &[]);
    let error = kloc_check(quoted.to_str().unwrap()).unwrap_err();
    assert!(error.contains("maos-kernel-core"));
    assert!(error.contains("integer"));

    let (_workspace, negative) = write_kloc_fixture("maos-kernel-core = -1\n", &[]);
    let error = kloc_check(negative.to_str().unwrap()).unwrap_err();
    assert!(error.contains("maos-kernel-core"));
    assert!(error.contains("non-negative"));
}
