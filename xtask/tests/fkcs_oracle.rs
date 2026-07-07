use xtask::check_fkcs::{FkcsBaseline, FkcsOracle, FkcsSurfaceSnapshot, ForgedSelfReport, is_blocking_at, parse_inline_disposition, phase_disposition, read_disposition, read_nonempty_lines};

#[test]
fn frozen_baseline_reconciles_live_triple_and_rejects_src_line_drift() {
    let baseline =
        FkcsBaseline::load_from_file("xtask/fkcs-baseline.toml").expect("baseline loads");

    assert_eq!(baseline.src_lines, 23_081);
    assert_eq!(baseline.abi_baseline, "abi-baseline/v1-pre-bump.txt");
    assert_eq!(baseline.host_baseline, "abi-baseline/maos-host-v1.txt");
    baseline
        .validate_files_exist()
        .expect("surface files exist");

    let mut drifted = baseline.clone();
    drifted.src_lines = 23_082;
    let err = drifted
        .reconcile_src_lines(23_081)
        .expect_err("src_lines drift must red the frozen-tag-consistency leg");
    assert!(err.contains("src_lines"));
}

#[test]
fn diff_oracle_derives_kernel_unchanged_and_ignores_self_reported_flags() {
    let before = FkcsSurfaceSnapshot::synthetic(
        23_081,
        ["pub struct StableAbi;"],
        ["pub trait SpiritHostPort;"],
    );
    let after = FkcsSurfaceSnapshot::synthetic(
        23_082,
        ["pub struct StableAbi;"],
        ["pub trait SpiritHostPort;"],
    );

    let report = FkcsOracle::derive(
        &before,
        &after,
        Some(ForgedSelfReport {
            kernel_unchanged: true,
            abi_unchanged: true,
        }),
    );

    assert!(
        !report.kernel_unchanged,
        "real line drift must beat forged flags"
    );
    assert_eq!(report.lines_before, 23_081);
    assert_eq!(report.lines_after, 23_082);
    assert!(
        report.ignored_self_report,
        "self-reported flags are not oracle inputs"
    );
}


#[test]
fn diff_oracle_reports_kernel_unchanged_for_identical_surfaces() {
    // Positive green path: before == after must derive kernel_unchanged == true.
    // The existing test only covers the RED/drift path; without this leg the
    // oracle's positive derivation is unproven and a synthetic-only regression
    // could pass.
    let surface = FkcsSurfaceSnapshot::synthetic(
        23_081,
        ["pub struct StableAbi;"],
        ["pub trait SpiritHostPort;"],
    );
    let report = FkcsOracle::derive(&surface, &surface, None);

    assert!(
        report.kernel_unchanged,
        "before == after must derive kernel_unchanged == true"
    );
    assert_eq!(report.lines_before, report.lines_after);
    assert!(report.abi_additive_only);
    assert!(report.host_closed_allowlist_holds);
    assert!(
        !report.ignored_self_report,
        "absent self-report means nothing to ignore"
    );
}

#[test]
fn diff_oracle_ignores_forged_self_report_even_when_it_claims_red() {
    // The oracle must ignore self-report ENTIRELY: a forged `kernel_unchanged =
    // false` cannot red a genuinely-unchanged surface (and a forged `true`
    // cannot green a drifted one — already covered by the drift test above).
    let surface = FkcsSurfaceSnapshot::synthetic(23_081, ["abi"], ["host"]);
    let forged_red = ForgedSelfReport {
        kernel_unchanged: false,
        abi_unchanged: false,
    };
    let report = FkcsOracle::derive(&surface, &surface, Some(forged_red));

    assert!(
        report.kernel_unchanged,
        "forged self-reported RED must not override real surface equality"
    );
    assert!(
        report.ignored_self_report,
        "self-report is recorded-as-ignored, never consulted as an oracle input"
    );
}

#[test]
fn frozen_baseline_files_carry_real_public_surfaces_not_synthetic_tokens() {
    // The oracle's green path consumes the committed abi/host baseline files
    // (the live cargo-public-api capture must match them). Assert they carry a
    // REAL multi-symbol public surface — not the synthetic "abi"/"host"
    // placeholder tokens a synthetic-only regression would substitute. This
    // uses the fast file-read path; the live capture itself is exercised by
    // the gate's diff-oracle leg.
    let baseline =
        FkcsBaseline::load_from_file("xtask/fkcs-baseline.toml").expect("baseline loads");

    let abi = read_nonempty_lines(&baseline.abi_baseline).expect("abi baseline reads");
    let host = read_nonempty_lines(&baseline.host_baseline).expect("host baseline reads");

    assert!(
        abi.contains("pub struct maos_spirit_abi::compliance::ComplianceClaimEnvelope"),
        "abi baseline must carry the real frozen public-API surface"
    );
    assert!(
        !abi.contains("abi"),
        "the synthetic 'abi' placeholder token must not stand in for the real surface"
    );
    assert!(abi.len() > 1, "a real abi surface is multi-symbol");
    assert!(!host.is_empty(), "host baseline must carry a non-empty real surface");
}

#[test]
fn live_triple_reconciles_real_kernel_core_line_count_not_a_literal() {
    // AC1: the frozen baseline src_lines is reconciled against the REAL
    // crates/maos-kernel-core/src tree — an independent line count of the live
    // sources, NOT the reconcile_src_lines(23_081) literal. A frozen-kernel
    // source change reds this. (The production `FkcsBaseline::validate_live_triple`
    // performs this same count and is exercised by the gate's frozen-tag leg from
    // the workspace root; this test re-counts independently so it is robust to the
    // cargo-test CWD and directly proves the baseline is grounded in reality.)
    let baseline =
        FkcsBaseline::load_from_file("xtask/fkcs-baseline.toml").expect("baseline loads");
    let real = real_kernel_core_src_lines();

    baseline
        .reconcile_src_lines(real)
        .expect("the frozen baseline src_lines must match the real kernel-core src count");
    assert_eq!(
        real, baseline.src_lines,
        "the baseline is grounded in the real kernel-core src line count"
    );
}

/// Independently count every `.rs` line under `crates/maos-kernel-core/src`,
/// matching the production `count_rs_lines` walk. `cargo test` may run with the
/// package directory as CWD (not the workspace root), so the src tree is located
/// by walking up from CWD rather than assuming a fixed working directory.
fn real_kernel_core_src_lines() -> usize {
    let cwd = std::env::current_dir().expect("current_dir is readable");
    for ancestor in cwd.ancestors() {
        let src = ancestor
            .join("crates")
            .join("maos-kernel-core")
            .join("src");
        if src.is_dir() {
            return count_rs_lines_recursive(&src);
        }
    }
    panic!(
        "crates/maos-kernel-core/src not found from {}",
        cwd.display()
    );
}

fn count_rs_lines_recursive(dir: &std::path::Path) -> usize {
    let mut total = 0;
    for entry in std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
    {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            total += count_rs_lines_recursive(&path);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            total += std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
                .lines()
                .count();
        }
    }
    total
}


#[test]
fn oracle_fault_injections_red_kernel_unchanged_around_a_stable_base() {
    // Pure (no subprocess) fault injection around a stable synthetic base: each
    // mutation MUST red the oracle. This proves the positive derivation is not
    // trivially green and that a synthetic-only gate leg could not hide a real
    // drift. The real gate leg applies these same mutators to a LIVE-captured
    // baseline.
    let base = FkcsSurfaceSnapshot::synthetic(
        23_081,
        ["pub struct StableAbi;"],
        ["pub trait SpiritHostPort;"],
    );

    let kernel_drift = base.with_src_lines(23_082);
    let abi_removed = base.without_abi_item("pub struct StableAbi;");
    let host_grew = base.with_extra_host_item("maos_host::UnauthorizedSurface");

    assert!(
        !FkcsOracle::derive(&base, &kernel_drift, None).kernel_unchanged,
        "a kernel line drift must red the oracle"
    );
    assert!(
        !FkcsOracle::derive(&base, &abi_removed, None).kernel_unchanged,
        "an ABI removal (breaks additive-only) must red the oracle"
    );
    assert!(
        !FkcsOracle::derive(&base, &host_grew, None).kernel_unchanged,
        "a host-surface growth (breaks closed-allowlist) must red the oracle"
    );
    assert!(
        FkcsOracle::derive_positive(&base).kernel_unchanged,
        "derive_positive agrees: an unmutated base stays green"
    );
}

#[test]
fn disposition_inline_parser_extracts_phase_keys_and_rejects_malformed_stanzas() {
    let line = r#"disposition = { v1_0 = "advisory", v1_5 = "advisory", v2_0 = "blocking" }"#;
    let map = parse_inline_disposition(line).expect("a well-formed disposition parses");
    assert_eq!(map.get("v1_0").map(String::as_str), Some("advisory"));
    assert_eq!(map.get("v1_5").map(String::as_str), Some("advisory"));
    assert_eq!(map.get("v2_0").map(String::as_str), Some("blocking"));

    // A braceless/malformed stanza must error — never silently empty-pass into a
    // vacuous disposition.
    parse_inline_disposition(r#"disposition = broken"#)
        .expect_err("a braceless disposition must be rejected, not silently dropped");
}

#[test]
fn phase_disposition_inherits_downward_and_resolves_the_blocking_window() {
    let line = r#"disposition = { v1_0 = "advisory", v1_5 = "advisory", v2_0 = "blocking" }"#;
    let map = parse_inline_disposition(line).expect("parses");

    // v2_0 is the blocking window; v1_0/v1_5 are advisory (non-blocking).
    assert_eq!(phase_disposition(&map, "v2_0"), Some("blocking"));
    assert!(!is_blocking_at(&map, "v1_0"));
    assert!(!is_blocking_at(&map, "v1_5"));
    assert!(is_blocking_at(&map, "v2_0"));

    // Phase inheritance: a phase with no explicit key inherits the nearest lower
    // declared phase (a v1_5 gap resolves up to v1_0). This is what makes
    // "advisory at v1.0/v1.5, blocking at v2.0" machine-readable, not prose.
    let sparse =
        parse_inline_disposition(r#"disposition = { v1_0 = "blocking", v2_0 = "advisory" }"#)
            .expect("parses");
    assert_eq!(
        phase_disposition(&sparse, "v1_5"),
        Some("blocking"),
        "v1_5 inherits v1_0's disposition when v1_5 is unset"
    );
    assert!(
        is_blocking_at(&sparse, "v1_5"),
        "the inherited blocking disposition is enforced at the gap phase"
    );
}

#[test]
fn read_disposition_extracts_exactly_the_check_fkcs_stanza() {
    // read_disposition must return EXACTLY check-fkcs's phase keys, bounded to
    // its [[ship_gate]] stanza (the parser stops at the next table, so it cannot
    // leak a sibling gate's disposition). Leakage would surface as stray or
    // missing keys.
    let map = read_disposition().expect("check-fkcs stanza parses from the registry");
    assert_eq!(map.get("v1_0").map(String::as_str), Some("advisory"));
    assert_eq!(map.get("v1_5").map(String::as_str), Some("advisory"));
    assert_eq!(map.get("v2_0").map(String::as_str), Some("blocking"));
    assert_eq!(
        map.len(),
        3,
        "the stanza boundary keeps exactly check-fkcs's three phase keys"
    );
    // Current phase (v1_5) is advisory; the gate graduates to blocking at v2_0.
    assert!(
        !is_blocking_at(&map, "v1_5"),
        "check-fkcs is advisory (non-blocking) at the current v1_5 phase"
    );
    assert!(
        is_blocking_at(&map, "v2_0"),
        "check-fkcs graduates to blocking at v2_0"
    );
}
