//! Story 20-7 — proven-red vectors for the performance gates, driven through the
//! REAL library entry points (`check_perf_verdict`, `check_perf_runner_policy`,
//! `check_perf_counts`, `perf_budgets`) over fixture ledgers and planted trees.
//!
//! * AC1 — one honest verdict per metric, and the two loud failures;
//! * AC2 — the runner policy: four named refusals, one control each;
//! * AC3 — no budget literal outside `xtask/perf-budgets.toml`;
//! * AC4 — every NFR-Perf-1…5,7,8 and success-criteria hot-path id has a metric
//!   with an existing, registered instrument;
//! * AC5 — Tier C counts against committed baselines.
//!
//! `MAOS_20_7_TREE` points the tree-scanning vectors (AC3/AC4) at another
//! checkout, which is how their failing-before witness runs at `a4c00897`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use xtask::check_perf_counts::{callgrind_total, judge};
use xtask::check_perf_runner_policy::audit;
use xtask::check_perf_verdict::{parse_ledger, staleness, verdicts, Verdict};
use xtask::perf_budgets::{self, Budgets};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives under the repo root")
        .to_path_buf()
}

fn tree() -> PathBuf {
    std::env::var_os("MAOS_20_7_TREE").map_or_else(repo_root, PathBuf::from)
}

fn fixture(name: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/perf-ledger")
            .join(name),
    )
    .unwrap_or_else(|error| panic!("fixture {name}: {error}"))
}

fn verdict_of(sha: &str, metric: &str) -> (Verdict, String) {
    let budgets = perf_budgets::parse(&fixture("budgets.toml")).expect("fixture budgets");
    let ledger = parse_ledger(&fixture("ledger.jsonl")).expect("fixture ledger");
    let found = verdicts(&budgets, sha, &ledger)
        .into_iter()
        .find(|v| v.metric == metric)
        .unwrap_or_else(|| panic!("no verdict for {metric}"));
    (found.verdict, found.reason)
}

// ── AC1 ─────────────────────────────────────────────────────────────────────

#[test]
fn a_run_inside_budget_calibration_spread_and_baseline_passes() {
    assert_eq!(verdict_of("s-pass", "fixture-latency").0, Verdict::Pass);
    // Inside the regression tolerance (1080 < 1000 × 1.10): not a regression.
    assert_eq!(
        verdict_of("s-within-tolerance", "fixture-latency").0,
        Verdict::Pass
    );
    // A `min` row: throughput over its floor passes, under it fails.
    assert_eq!(
        verdict_of("s-tp-pass", "fixture-throughput").0,
        Verdict::Pass
    );
    assert_eq!(
        verdict_of("s-tp-low", "fixture-throughput").0,
        Verdict::Fail
    );
}

#[test]
fn a_noisy_five_run_spread_is_indeterminate_never_pass() {
    let (verdict, reason) = verdict_of("s-noisy", "fixture-latency");
    assert_eq!(verdict, Verdict::Indeterminate, "{reason}");
}

#[test]
fn the_median_decides_a_budget_breach_not_the_best_of_five() {
    // Best run 4000 (< 5000), median 6000 (> 5000), spread 0.35 (< 0.4): the
    // best-of-five plant reads PASS here.
    let (verdict, reason) = verdict_of("s-best-of-five", "fixture-latency");
    assert_eq!(verdict, Verdict::Fail, "{reason}");
}

#[test]
fn a_regression_beyond_tolerance_and_three_mads_fails() {
    let (verdict, reason) = verdict_of("s-regress", "fixture-latency");
    assert_eq!(verdict, Verdict::Fail, "{reason}");
    assert!(reason.contains("regressed"), "{reason}");
}

#[test]
fn a_noisy_calibration_makes_every_verdict_indeterminate() {
    assert_eq!(
        verdict_of("s-calib-cv", "fixture-latency").0,
        Verdict::Indeterminate
    );
    assert_eq!(
        verdict_of("s-calib-drift", "fixture-latency").0,
        Verdict::Indeterminate
    );
    assert_eq!(
        verdict_of("s-nocalib", "fixture-latency").0,
        Verdict::Indeterminate
    );
}

#[test]
fn a_new_fingerprint_is_indeterminate_until_a_rebaseline_entry_exists() {
    let (verdict, reason) = verdict_of("s-newfp", "fixture-latency");
    assert_eq!(verdict, Verdict::Indeterminate, "{reason}");
    assert_eq!(
        verdict_of("s-rebaselined", "fixture-latency").0,
        Verdict::Pass
    );
}

#[test]
fn no_record_for_the_sha_is_absent() {
    assert_eq!(
        verdict_of("s-never-ran", "fixture-latency").0,
        Verdict::Absent
    );
}

#[test]
fn only_pass_is_green_so_a_binding_non_pass_exits_red_through_the_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(repo_root())
        .args(["check-perf-verdict", "--sha", "s-pass", "--ledger"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/perf-ledger"))
        .output()
        .expect("run check-perf-verdict");
    // The REAL budget file has no `fixture-*` rows, so every real metric reads
    // ABSENT; every real Tier W row is advisory at v1_5, so the run is green
    // but says so loudly instead of printing PASS.
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("ABSENT nfr-perf-1-iac-routing-p50"), "{text}");
    assert!(!text.contains("PASS "), "nothing is PASS: {text}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("WOULD BLOCK once binding"),
        "advisory non-PASS rows announce themselves"
    );
    assert!(output.status.success());
}

fn in_scratch_repo(args: &[&str]) -> (bool, String) {
    let scratch = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(scratch.path().join("xtask")).expect("xtask dir");
    fs::copy(
        repo_root().join("xtask/perf-budgets.toml"),
        scratch.path().join("xtask/perf-budgets.toml"),
    )
    .expect("copy budgets");
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(scratch.path())
        .args(args)
        .output()
        .expect("run xtask");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn a_missing_ledger_is_a_loud_error_never_absent() {
    let (ok, stderr) = in_scratch_repo(&[
        "check-perf-verdict",
        "--sha",
        "x",
        "--ledger",
        "/nonexistent/ledger",
    ]);
    assert!(!ok);
    assert!(stderr.contains("perf-ledger missing"), "{stderr}");
    // No `--ledger`: the `perf-ledger` branch, absent in a scratch tree.
    let (ok, stderr) = in_scratch_repo(&["check-perf-verdict", "--sha", "x"]);
    assert!(!ok);
    assert!(stderr.contains("perf-ledger branch is missing"), "{stderr}");
}

#[test]
fn a_malformed_ledger_line_is_a_loud_error_naming_the_line() {
    let error = parse_ledger("{\"kind\":\"calibration\"}\nnot json\n").expect_err("loud");
    assert!(error.contains("perf-ledger line 1"), "{error}");
}

#[test]
fn a_ledger_older_than_the_stale_limit_is_red() {
    let budgets: Budgets = perf_budgets::parse(&fixture("budgets.toml")).expect("budgets");
    let ledger = parse_ledger(&fixture("ledger.jsonl")).expect("ledger");
    let at = |text: &str| {
        chrono::DateTime::parse_from_rfc3339(text)
            .expect("time")
            .to_utc()
    };
    // Newest entry 2026-11-01T00:00:00Z.
    assert_eq!(
        staleness(&budgets, &ledger, at("2026-11-02T23:00:00Z")).expect("ok"),
        None
    );
    assert_eq!(
        staleness(&budgets, &ledger, at("2026-11-03T01:00:00Z")).expect("ok"),
        Some(49)
    );
}

// ── AC2 ─────────────────────────────────────────────────────────────────────

const PERF_YML: &str = "\
name: perf
on:
  push:
    branches: [main]
  schedule:
    - cron: '0 3 * * *'
  workflow_dispatch:
permissions:
  contents: read
jobs:
  tier-w:
    environment: perf-host
    if: github.ref == 'refs/heads/main'
    runs-on: [self-hosted, maos-perf]
    steps:
      - uses: actions/checkout@v5
  append:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - run: echo hosted follow-up
";

const OTHER_YML: &str = "name: other\non: push\njobs:\n  a:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo ok\n";

fn workflows(perf: &str, other: &str) -> tempfile::TempDir {
    let scratch = tempfile::tempdir().expect("tempdir");
    let dir = scratch.path().join(".github/workflows");
    fs::create_dir_all(&dir).expect("workflows dir");
    fs::write(dir.join("perf.yml"), perf).expect("perf.yml");
    fs::write(dir.join("other.yml"), other).expect("other.yml");
    scratch
}

fn findings(perf: &str, other: &str) -> Vec<String> {
    audit(workflows(perf, other).path()).expect("audit")
}

#[test]
fn the_policy_holds_on_a_compliant_tree_and_on_the_real_tree() {
    assert_eq!(findings(PERF_YML, OTHER_YML), Vec::<String>::new());
    assert_eq!(
        audit(&repo_root()).expect("real tree"),
        Vec::<String>::new()
    );
}

#[test]
fn refusal_1_a_second_workflow_targeting_the_perf_runner() {
    let other = OTHER_YML.replace("ubuntu-latest", "[self-hosted, maos-perf]");
    let found = findings(PERF_YML, &other);
    assert!(
        found
            .iter()
            .any(|f| f.contains("other.yml") && f.contains("only perf.yml may")),
        "{found:?}"
    );
}

#[test]
fn refusal_2_perf_yml_without_its_fork_guard() {
    let perf = PERF_YML.replace("    if: github.ref == 'refs/heads/main'\n", "");
    let found = findings(&perf, OTHER_YML);
    assert!(found.iter().any(|f| f.contains("fork guard")), "{found:?}");
}

#[test]
fn refusal_3_pull_request_target_added() {
    let perf = PERF_YML.replace(
        "  workflow_dispatch:\n",
        "  workflow_dispatch:\n  pull_request_target:\n",
    );
    let found = findings(&perf, OTHER_YML);
    assert!(
        found.iter().any(|f| f.contains("pull_request_target")),
        "{found:?}"
    );
}

#[test]
fn refusal_4_permissions_wider_than_contents_read_on_a_perf_job() {
    let perf = PERF_YML.replace(
        "permissions:\n  contents: read\n",
        "permissions:\n  contents: write\n",
    );
    let found = findings(&perf, OTHER_YML);
    assert!(
        found.iter().any(|f| f.contains("contents: read")),
        "{found:?}"
    );
    // A job-level override widening only the perf job reds too.
    let perf = PERF_YML.replace(
        "    runs-on: [self-hosted, maos-perf]\n",
        "    runs-on: [self-hosted, maos-perf]\n    permissions:\n      contents: read\n      id-token: write\n",
    );
    let found = findings(&perf, OTHER_YML);
    assert!(
        found.iter().any(|f| f.contains("contents: read")),
        "{found:?}"
    );
}

// ── AC3 / AC4 ───────────────────────────────────────────────────────────────

#[test]
fn the_budget_file_is_the_only_home_of_budget_literals() {
    let found = perf_budgets::budget_literals(&tree());
    assert_eq!(
        found,
        Vec::<String>::new(),
        "budget literals outside xtask/perf-budgets.toml"
    );
}

#[test]
fn a_re_added_literal_reds_the_scan() {
    let scratch = tempfile::tempdir().expect("tempdir");
    let decision = scratch.path().join("crates/maos-bench/src/decision.rs");
    fs::create_dir_all(decision.parent().expect("parent")).expect("dirs");
    fs::write(&decision, "const J4_P95_BUDGET_US: u64 = 10_000;\n").expect("write");
    let found = perf_budgets::budget_literals(scratch.path());
    assert!(
        found.iter().any(|f| f.contains("decision.rs:1")),
        "{found:?}"
    );
}

/// Each requirement id the epic names, and the substring its budget row carries.
const REQUIRED: &[(&str, &str)] = &[
    ("NFR-Perf-1", "NFR-Perf-1"),
    ("NFR-Perf-2", "NFR-Perf-2"),
    ("NFR-Perf-3", "NFR-Perf-3"),
    ("NFR-Perf-4", "NFR-Perf-4"),
    ("NFR-Perf-5", "NFR-Perf-5"),
    ("NFR-Perf-7", "NFR-Perf-7"),
    ("NFR-Perf-8", "NFR-Perf-8"),
    ("hot path iac/send", "iac/send"),
    ("hot path token issuance", "token issuance"),
    ("hot path invocation dispatch", "invocation dispatch"),
    ("hot path memory/read cached", "memory/read (cached)"),
    ("hot path memory/read uncached", "memory/read (uncached"),
    ("hot path hot-swap", "hot-swap (rust-inproc)"),
    ("hot path telemetry broadcast", "telemetry broadcast"),
];

/// An instrument exists, and a bench file is registered as a `[[bench]]` target.
fn instrument_problem(root: &Path, instrument: &str) -> Option<String> {
    let path = root.join(instrument);
    if !path.is_file() {
        return Some(format!("{instrument} does not exist"));
    }
    let (crate_dir, rest) = instrument.split_once("/benches/")?;
    let name = rest.strip_suffix(".rs")?;
    let manifest = fs::read_to_string(root.join(crate_dir).join("Cargo.toml")).ok()?;
    (!manifest.contains(&format!("name = \"{name}\"")))
        .then(|| format!("{instrument} is not a registered [[bench]]"))
}

fn completeness_problems(root: &Path) -> Vec<String> {
    let budgets = perf_budgets::load(root).expect("budget file");
    let mut problems = Vec::new();
    for (label, needle) in REQUIRED {
        let rows: Vec<_> = budgets
            .metric
            .iter()
            .filter(|m| m.tier == "W" && m.requirement.contains(needle))
            .collect();
        if rows.is_empty() {
            problems.push(format!("{label}: no Tier W metric row"));
        }
        problems.extend(rows.iter().filter_map(|m| {
            instrument_problem(root, &m.instrument).map(|p| format!("{label} ({}): {p}", m.id))
        }));
    }
    problems
}

#[test]
fn every_nfr_perf_and_hot_path_budget_has_a_metric_with_an_existing_instrument() {
    assert_eq!(completeness_problems(&tree()), Vec::<String>::new());
}

#[test]
fn deleting_the_nfr_perf_4_bench_reds_the_completeness_vector() {
    let scratch = tempfile::tempdir().expect("tempdir");
    for (rel, text) in [
        (
            "xtask/perf-budgets.toml",
            fs::read_to_string(repo_root().join("xtask/perf-budgets.toml")).expect("budgets"),
        ),
        (
            "crates/maos-bench/Cargo.toml",
            fs::read_to_string(repo_root().join("crates/maos-bench/Cargo.toml")).expect("manifest"),
        ),
    ] {
        let target = scratch.path().join(rel);
        fs::create_dir_all(target.parent().expect("parent")).expect("dirs");
        fs::write(target, text).expect("copy");
    }
    // Every instrument present except the NFR-Perf-4 bench.
    let budgets = perf_budgets::load(scratch.path()).expect("budgets");
    for metric in budgets
        .metric
        .iter()
        .filter(|m| m.instrument != "crates/maos-bench/benches/posture_shift_nfr_perf_4.rs")
    {
        let target = scratch.path().join(&metric.instrument);
        fs::create_dir_all(target.parent().expect("parent")).expect("dirs");
        fs::write(target, "// instrument\n").expect("instrument");
    }
    let problems = completeness_problems(scratch.path());
    assert!(
        problems
            .iter()
            .any(|p| p.contains("NFR-Perf-4") && p.contains("does not exist")),
        "{problems:?}"
    );
    assert!(
        !problems.iter().any(|p| p.contains("NFR-Perf-3")),
        "only NFR-Perf-4 is missing: {problems:?}"
    );
}

#[test]
fn the_budget_file_phase_is_the_gate_phase_and_every_tier_c_path_is_a_harness_path() {
    let budgets = perf_budgets::load(&repo_root()).expect("budgets");
    assert_eq!(budgets.protocol.phase, xtask::gate_common::CURRENT_PHASE);
    let harness =
        fs::read_to_string(repo_root().join("crates/maos-bench/src/bin/tier_c_harness.rs"))
            .expect("harness");
    for metric in budgets.metric.iter().filter(|m| m.tier == "C") {
        let path = metric.path.as_deref().expect("Tier C rows name a path");
        assert!(
            harness.contains(&format!("\"{path}\" =>")),
            "{}: harness has no `{path}` path",
            metric.id
        );
    }
}

// ── AC5 ─────────────────────────────────────────────────────────────────────

#[test]
fn callgrind_totals_come_from_the_totals_line() {
    let out = "events: Ir\nsummary: 10\nfn=(1) f\n7 3\ntotals: 1234567\n";
    assert_eq!(callgrind_total(out), Some(1_234_567));
    assert_eq!(callgrind_total("events: Ir\nsummary: 42\n"), Some(42));
    assert_eq!(callgrind_total("garbage"), None);
}

#[test]
fn a_count_over_tolerance_fails_and_a_missing_baseline_is_absent() {
    let budgets = perf_budgets::load(&repo_root()).expect("budgets");
    let mut row = budgets
        .metric
        .iter()
        .find(|m| m.tier == "C")
        .expect("a Tier C row")
        .clone();
    row.baseline = Some(100.0);
    row.regression_tolerance = 0.02;
    assert_eq!(judge(&row, 101.0).0, Verdict::Pass);
    assert_eq!(judge(&row, 102.5).0, Verdict::Fail);
    row.baseline = None;
    assert_eq!(judge(&row, 50.0).0, Verdict::Absent);
}

fn counts_dir(extra_allocations: f64, extra_instructions: f64) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let budgets = perf_budgets::load(&repo_root()).expect("budgets");
    let write = |path: &str, iterations: u64, allocations: u64, instructions: u64| {
        fs::write(
            dir.path().join(format!("{path}.json")),
            format!(
                "{{\"path\":\"{path}\",\"iterations\":{iterations},\"allocations\":{allocations}}}"
            ),
        )
        .expect("count json");
        fs::write(
            dir.path().join(format!("{path}.callgrind")),
            format!("events: Ir\ntotals: {instructions}\n"),
        )
        .expect("callgrind");
    };
    let iterations = 10_000;
    let empty_instructions = 1000;
    write("empty", 0, 0, empty_instructions);
    for metric in budgets
        .metric
        .iter()
        .filter(|m| m.tier == "C" && m.unit == "allocations")
    {
        let baseline = metric.baseline.expect("allocation rows are baselined");
        let path = metric.path.as_deref().expect("path");
        let instruction_baseline = budgets
            .metric
            .iter()
            .find(|m| m.tier == "C" && m.unit == "instructions" && m.path == metric.path)
            .and_then(|m| m.baseline)
            .expect("hosted instruction baseline");
        write(
            path,
            iterations,
            (baseline * iterations as f64 * (1.0 + extra_allocations)).round() as u64,
            empty_instructions
                + (instruction_baseline * iterations as f64 * (1.0 + extra_instructions)).round()
                    as u64,
        );
    }
    dir
}

#[test]
fn tier_c_binds_hosted_counts_and_detects_instruction_and_allocation_regressions() {
    let run = |dir: &Path| {
        Command::new(env!("CARGO_BIN_EXE_xtask"))
            .current_dir(repo_root())
            .args(["check-perf-counts", "--dir"])
            .arg(dir)
            .output()
            .expect("run check-perf-counts")
    };
    let at_baseline = run(counts_dir(0.0, 0.0).path());
    assert!(
        at_baseline.status.success(),
        "{}",
        String::from_utf8_lossy(&at_baseline.stderr)
    );
    let text = String::from_utf8_lossy(&at_baseline.stdout);
    assert!(text.contains("PASS cap-token-verify-allocations"), "{text}");
    assert!(!text.contains("ABSENT"), "{text}");
    let budgets = perf_budgets::load(&repo_root()).expect("budgets");
    for metric in budgets.metric.iter().filter(|m| m.tier == "C") {
        assert!(metric.binding_at(&budgets.protocol.phase), "{}", metric.id);
        assert!(text.contains(&format!("PASS {} ", metric.id)), "{text}");
    }
    let over = run(counts_dir(0.10, 0.0).path());
    assert!(
        !over.status.success(),
        "a 10% allocation regression must block"
    );
    assert!(String::from_utf8_lossy(&over.stdout).contains("FAIL cap-token-verify-allocations"));
    let instructions = run(counts_dir(0.0, 0.10).path());
    assert!(
        !instructions.status.success(),
        "a 10% instruction regression must block"
    );
    let text = String::from_utf8_lossy(&instructions.stdout);
    assert!(
        text.contains("FAIL cap-token-verify-instructions"),
        "{text}"
    );
    assert!(text.contains("PASS cap-token-verify-allocations"), "{text}");
}

#[test]
fn refusal_vectors_for_runner_policy_bypasses() {
    for selector in [
        "self-hosted",
        "[self-hosted]",
        "${{ vars.HOST }}",
        "\"maos\\x2dperf\"",
        "{group: perf}",
        "*host",
    ] {
        let perf = PERF_YML.replace("[self-hosted, maos-perf]", selector);
        assert!(!findings(&perf, OTHER_YML).is_empty(), "{selector}");
        let other = OTHER_YML.replace("ubuntu-latest", selector);
        assert!(!findings(PERF_YML, &other).is_empty(), "{selector}");
    }
    for condition in [
        "true || github.ref == 'refs/heads/main'",
        "a || github.event.pull_request.head.repo.full_name == github.repository",
        "true||github.event.pull_request.head.repo.full_name == github.repository",
        "github.ref == 'refs/heads/main' || true",
    ] {
        assert!(!findings(
            &PERF_YML.replace("github.ref == 'refs/heads/main'", condition),
            OTHER_YML
        )
        .is_empty());
    }
    for event in ["pull_request", "pull_request_target", "issue_comment"] {
        assert!(!findings(
            &PERF_YML.replace(
                "  workflow_dispatch:",
                &format!("  workflow_dispatch:\n  {event}:")
            ),
            OTHER_YML
        )
        .is_empty());
    }
    for expression in [
        "${{ secrets.X }}",
        "${{ secrets['X'] }}",
        "${{ toJSON(secrets) }}",
    ] {
        let perf = format!("env:\n  LEAK: \"{expression}\"\n{PERF_YML}");
        assert!(findings(&perf, OTHER_YML)
            .iter()
            .any(|f| f.contains("secrets")));
    }
    for extra in [
        "    uses: owner/repo/.github/workflows/a.yml@main\n",
        "    env: &host {X: ok}\n",
    ] {
        assert!(!findings(
            &PERF_YML.replace("  tier-w:\n", &format!("  tier-w:\n{extra}")),
            OTHER_YML
        )
        .is_empty());
    }
}

#[test]
fn malformed_counts_never_pass() {
    use xtask::check_perf_counts::{figure, Measured};
    let budgets = perf_budgets::load(&repo_root()).unwrap();
    let row = budgets
        .metric
        .iter()
        .find(|m| m.unit == "instructions")
        .unwrap();
    let empty = Measured {
        iterations: 0,
        allocations: 0,
        instructions: 100,
    };
    assert!(figure(
        row,
        &Measured {
            iterations: 0,
            allocations: 0,
            instructions: 100
        },
        &empty
    )
    .is_err());
    assert!(figure(
        row,
        &Measured {
            iterations: 1,
            allocations: 0,
            instructions: 99
        },
        &empty
    )
    .is_err());
    for value in [f64::NAN, f64::INFINITY, -1.0] {
        assert_eq!(judge(row, value).0, Verdict::Fail);
    }
}

#[test]
fn malformed_budget_semantics_are_refused() {
    let text = fixture("budgets.toml");
    for (from, to) in [
        ("tier = \"W\"", "tier = \"V\""),
        ("budget = 5000", "budgt = 5000"),
        ("unit = \"us\"", "unit = \"seconds\""),
        ("\"blocking\"", "\"blokcing\""),
        ("noise_tolerance = 0.4", "noise_tolerance = nan"),
        ("budget = 5000", "budget = -1"),
        ("fixture-throughput", "fixture-latency"),
        ("phase = \"v1_5\"", "phase = \"unknown\""),
    ] {
        assert!(
            perf_budgets::parse(&text.replace(from, to)).is_err(),
            "{to}"
        );
    }
}

#[test]
fn malformed_metric_lines_are_not_filtered() {
    use xtask::perf_ledger::metrics_in;
    for text in [
        "MAOS-PERF not-json",
        "MAOS-PERF {\"metric\":\"x\"}",
        "MAOS-PERF {\"metric\":\"x\",\"value\":-1}",
        "MAOS-PERF {\"metric\":\"x\",\"value\":1}\nMAOS-PERF {\"metric\":\"x\",\"value\":2}",
    ] {
        assert!(metrics_in(text).is_err(), "{text}");
    }
}

#[test]
fn ledger_observation_identity_and_provenance_vectors() {
    let text = fixture("ledger.jsonl");
    let rows: Vec<serde_json::Value> = text
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let metric = rows
        .iter()
        .find(|r| r["sha"] == "s-pass" && r["kind"] == "metric")
        .unwrap();
    for (field, value) in [
        ("runs", serde_json::json!([1000])),
        ("process_runs", serde_json::json!(["1", "1", "2", "3", "4"])),
        ("ts", serde_json::json!("2099-01-01T00:00:00Z")),
    ] {
        let mut row = metric.clone();
        row[field] = value;
        assert!(parse_ledger(&row.to_string()).is_err());
    }
    let budgets = perf_budgets::parse(&fixture("budgets.toml")).unwrap();
    let mut ledger = parse_ledger(&text).unwrap();
    ledger.push(serde_json::from_value(metric.clone()).unwrap());
    assert_eq!(
        verdicts(&budgets, "s-pass", &ledger)[0].verdict,
        Verdict::Fail
    );
    for field in ["run_id", "fingerprint"] {
        let changed = rows
            .iter()
            .map(|r| {
                let mut r = r.clone();
                if r["sha"] == "s-pass" && r["kind"] == "calibration" {
                    r[field] = "other".into();
                }
                r.to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            verdicts(&budgets, "s-pass", &parse_ledger(&changed).unwrap())[0].verdict,
            Verdict::Indeterminate
        );
    }
    let rebaseline = rows.iter().find(|r| r["kind"] == "rebaseline").unwrap();
    for field in ["writer", "actor", "review", "run_id", "reason"] {
        let mut row = rebaseline.clone();
        row[field] = "".into();
        assert!(parse_ledger(&row.to_string()).is_err(), "{field}");
    }
    let mut later = rebaseline.clone();
    later["ts"] = "2026-11-01T00:00:01Z".into();
    later["fingerprint"] = "fp-a".into();
    later["median"] = 999999.into();
    let mut ledger = parse_ledger(&text).unwrap();
    ledger.push(serde_json::from_value(later).unwrap());
    assert_eq!(
        verdicts(&budgets, "s-regress", &ledger)[0].verdict,
        Verdict::Fail
    );
    let now = chrono::DateTime::parse_from_rfc3339("2026-11-03T00:00:01Z")
        .unwrap()
        .to_utc();
    assert!(staleness(&budgets, &parse_ledger(&text).unwrap(), now)
        .unwrap()
        .is_some());
}

#[test]
fn issuance_instrument_uses_the_production_signing_provider() {
    let source =
        fs::read_to_string(repo_root().join("crates/maos-kernel-core/benches/hot_path_budgets.rs"))
            .unwrap();
    assert!(source.contains("Arc::new(maos_kernel_core::api::RingCryptoProvider)"));
    assert!(!source.contains("impl CryptoProvider for Mock"));
}

#[test]
fn append_refuses_missing_duplicate_malformed_or_mismatched_observations() {
    let root = repo_root();
    let budgets = perf_budgets::load(&root).unwrap();
    for variant in [
        "valid",
        "missing-run",
        "missing-metric",
        "duplicate",
        "malformed",
        "kind",
        "sha",
        "fingerprint",
        "run_id",
        "second-sha",
    ] {
        let scratch = tempfile::tempdir().unwrap();
        let logs = scratch.path().join("logs");
        fs::create_dir(&logs).unwrap();
        let ledger = scratch.path().join("ledger.jsonl");
        let calibration = scratch.path().join("calibration.json");
        let mut cal: serde_json::Value =
            serde_json::from_str(fixture("ledger.jsonl").lines().next().unwrap()).unwrap();
        cal["sha"] = "new-sha".into();
        cal["run_id"] = "123/1".into();
        if ["kind", "sha", "fingerprint", "run_id"].contains(&variant) {
            cal[variant] = "wrong".into();
        }
        fs::write(&calibration, cal.to_string()).unwrap();
        fs::write(
            &ledger,
            if variant == "second-sha" {
                fixture("ledger.jsonl").replace("s-pass", "new-sha")
            } else {
                fixture("ledger.jsonl")
            },
        )
        .unwrap();
        for run in 1..=5 {
            if variant == "missing-run" && run == 5 {
                continue;
            }
            let mut lines = Vec::new();
            for metric in budgets
                .metric
                .iter()
                .filter(|m| m.tier == "W" && m.instrument.contains("/benches/"))
            {
                if variant == "missing-metric"
                    && metric.id == "hot-path-token-issuance-p99"
                    && run == 5
                {
                    continue;
                }
                lines.push(format!(
                    "MAOS-PERF {}",
                    serde_json::json!({"metric":metric.id,"value":1})
                ));
            }
            if run == 1 && variant == "duplicate" {
                lines.push(lines[0].clone());
            }
            if run == 1 && variant == "malformed" {
                lines.push("MAOS-PERF broken".into());
            }
            fs::write(logs.join(format!("run-{run}-all.log")), lines.join("\n")).unwrap();
        }
        let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
            .current_dir(&root)
            .env("GITHUB_RUN_ID", "123")
            .env("GITHUB_RUN_ATTEMPT", "1")
            .args(["perf-ledger-append", "--ledger"])
            .arg(&ledger)
            .arg("--logs")
            .arg(&logs)
            .arg("--calibration")
            .arg(&calibration)
            .args([
                "--sha",
                "new-sha",
                "--branch",
                "main",
                "--fingerprint",
                "fp-a",
            ])
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            variant == "valid",
            "{variant}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn workflow_trust_and_diagnostic_status_contracts() {
    let perf: serde_yaml::Value = serde_yaml::from_str(
        &fs::read_to_string(repo_root().join(".github/workflows/perf.yml")).unwrap(),
    )
    .unwrap();
    assert!(perf["on"].get("pull_request").is_none());
    let append = &perf["jobs"]["append"];
    assert_eq!(
        append["if"].as_str(),
        Some("${{ github.ref == 'refs/heads/main' && needs.tier-w.result == 'success' }}")
    );
    assert_eq!(
        append["steps"][0]["with"]["persist-credentials"].as_bool(),
        Some(false)
    );
    assert_eq!(
        append["steps"][0]["with"]["ref"].as_str(),
        Some("${{ github.sha }}")
    );
    let source=fs::read_to_string(repo_root().join("_bmad-output/implementation-artifacts/20-7-stable-runner-perf-gates-evidence/ci/ci-20-7-verify.yml")).unwrap();
    let workflow: serde_yaml::Value = serde_yaml::from_str(&source).unwrap();
    let script = workflow["jobs"]["diag-ac4"]["steps"]
        .as_sequence()
        .unwrap()
        .iter()
        .filter_map(|s| s["run"].as_str())
        .find(|s| s.contains("diag/summary.txt"))
        .unwrap();
    for required in [
        "set -o pipefail",
        "PIPESTATUS",
        "INCOMPLETE",
        "SUCCESS",
        "FAILED",
    ] {
        assert!(script.contains(required), "{required}");
    }
}

#[test]
fn escaped_host_and_matrix_expansion_cannot_bypass_admission() {
    let escaped = PERF_YML.replace(
        "[self-hosted, maos-perf]",
        "[self-hosted, \"maos\\x2dperf\"]",
    );
    assert!(findings(&escaped, OTHER_YML)
        .iter()
        .any(|f| f.contains("escaped")));
    let matrix=OTHER_YML.replace("runs-on: ubuntu-latest","runs-on: ${{ matrix.os }}\n    strategy:\n      matrix:\n        os: [self-hosted]\n        include:\n          - os: ubuntu-latest");
    assert!(!findings(PERF_YML, &matrix).is_empty());
}

#[test]
fn ledger_timestamps_compare_instants_not_offsets_or_truncated_hours() {
    let budgets = perf_budgets::parse(&fixture("budgets.toml")).unwrap();
    let mut row: serde_json::Value =
        serde_json::from_str(fixture("ledger.jsonl").lines().next().unwrap()).unwrap();
    row["ts"] = "2026-11-01T00:00:00+01:00".into();
    let mut later = row.clone();
    later["ts"] = "2026-10-31T23:30:00Z".into();
    let ledger = parse_ledger(&format!("{row}\n{later}")).unwrap();
    let at = chrono::DateTime::parse_from_rfc3339("2026-11-02T23:15:00Z")
        .unwrap()
        .to_utc();
    assert_eq!(staleness(&budgets, &ledger, at).unwrap(), None);
    let future = chrono::DateTime::parse_from_rfc3339("2026-10-31T22:00:00Z")
        .unwrap()
        .to_utc();
    assert!(staleness(&budgets, &ledger, future).is_err());
}

#[test]
fn rebaseline_writer_generates_provenance_and_refuses_forged_fields() {
    for variant in [
        "valid",
        "writer",
        "negative-mad",
        "negative-median",
        "missing-review",
        "branch",
    ] {
        let scratch = tempfile::tempdir().unwrap();
        let ledger = scratch.path().join("ledger.jsonl");
        let request = scratch.path().join("request.json");
        fs::write(&ledger, "").unwrap();
        let mut row = serde_json::json!({"metric":"hot-path-token-issuance-p99","median":5.0,"mad":0.0,"reason":"reviewed measurement","review":"https://github.com/lunarpulse/maos/pull/1"});
        match variant {
            "writer" => row["writer"] = "forged".into(),
            "negative-mad" => row["mad"] = (-1).into(),
            "negative-median" => row["median"] = (-1).into(),
            "missing-review" => row["review"] = "".into(),
            _ => {}
        }
        fs::write(&request, row.to_string()).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_xtask"))
            .current_dir(repo_root())
            .env(
                "GITHUB_REF",
                if variant == "branch" {
                    "refs/heads/topic"
                } else {
                    "refs/heads/main"
                },
            )
            .env("GITHUB_SHA", "trusted-sha")
            .env("GITHUB_RUN_ID", "123")
            .env("GITHUB_RUN_ATTEMPT", "1")
            .env("GITHUB_ACTOR", "operator")
            .args(["perf-ledger-append", "--ledger"])
            .arg(&ledger)
            .arg("--rebaseline")
            .arg(&request)
            .args([
                "--sha",
                "trusted-sha",
                "--branch",
                "main",
                "--fingerprint",
                "fp",
            ])
            .output()
            .unwrap();
        assert_eq!(
            result.status.success(),
            variant == "valid",
            "{variant}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        if variant == "valid" {
            let text = fs::read_to_string(&ledger).unwrap();
            let rows = parse_ledger(&text).unwrap();
            assert!(
                matches!(&rows[0],xtask::check_perf_verdict::Record::Rebaseline {writer,actor,run_id,..} if writer=="perf-ledger-append" && actor=="operator" && run_id=="123/1")
            );
        } else {
            assert!(fs::read_to_string(&ledger).unwrap().is_empty());
        }
    }
}
