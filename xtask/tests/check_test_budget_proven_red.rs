//! Story `20-12` AC1 — proven-red vectors for `check-test-budget`.
//!
//! The junit reports are nextest-shaped (one `<testcase>` per test with
//! `classname` = binary id, `timestamp`, `time`) and go through the SAME
//! `parse_junit` + `audit` the CI job runs. Register vectors are written to a
//! tempdir tree and read back through the SAME `load_budgets`. The in-place
//! plant of a real 31 s sleep is recorded once in `20-12-evidence/`; these
//! vectors keep the verdict logic honest on every run in milliseconds.
//!
//! Review 2026-10-23: the budgeted metric is Σ per-test `time` (the span is
//! reported only); non-finite budgets and junit times, empty `[[expensive]]`
//! fields, duplicate binaries and stale `[[exception]]` rows each red.

use std::fs;
use std::path::Path;
use xtask::check_test_budget::{audit, load_budgets, parse_budgets, parse_junit, Budgets};

const PLAIN: &str = "default_seconds = 30\ntotal_seconds = 300\n";

const REGISTER: &str = "\
default_seconds = 30
total_seconds = 300

[[exception]]
binary = \"slow::daemon\"
seconds = 45
reason = \"real daemon boot\"
owner = \"20-12\"
";

const EXPENSIVE: &str = "\
default_seconds = 30
total_seconds = 300

[[expensive]]
binary = \"foo::bar\"
job = \"expensive-tests\"
reason = \"spawns cargo\"
owner = \"20-12\"
";

fn plain() -> Budgets {
    parse_budgets(PLAIN).expect("register parses")
}

fn register() -> Budgets {
    parse_budgets(REGISTER).expect("register parses")
}

/// `text` as `xtask/test-budgets.toml` in a tempdir tree, read back through
/// `load_budgets`.
fn load(text: &str) -> Result<Budgets, String> {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(dir.path().join("xtask")).expect("xtask dir");
    fs::write(dir.path().join("xtask/test-budgets.toml"), text).expect("register");
    load_budgets(dir.path())
}

fn assert_refused(text: &str, needle: &str) {
    let error = load(text).expect_err("register must be refused");
    assert!(error.contains(needle), "{error:?} lacks {needle:?}");
}

/// One binary whose tests start together; test `i` lasts `seconds[i]`.
fn junit(binary: &str, seconds: &[f64]) -> String {
    let cases: String = seconds
        .iter()
        .enumerate()
        .map(|(i, s)| {
            format!(
                "<testcase name=\"t{i}\" classname=\"{binary}\" \
                 timestamp=\"2026-10-23T10:00:00.000+00:00\" time=\"{s:.3}\"/>\n"
            )
        })
        .collect();
    format!("<testsuites><testsuite name=\"{binary}\">\n{cases}</testsuite></testsuites>\n")
}

fn testcase(time: &str) -> String {
    format!(
        "<testcase name=\"a\" classname=\"x::y\" timestamp=\"2026-10-23T10:00:00.000+00:00\" \
         time=\"{time}\"/>"
    )
}

#[test]
fn a_binary_under_its_budget_is_green() {
    let report = audit(
        &plain(),
        parse_junit(&junit("fast::suite", &[0.2, 29.0])).unwrap(),
    );
    assert!(report.findings.is_empty(), "{:#?}", report.findings);
    assert!((report.total_sum_seconds - 29.2).abs() < 1e-6);
}

#[test]
fn a_planted_31_second_test_in_an_unbudgeted_binary_reds() {
    let report = audit(
        &plain(),
        parse_junit(&junit("fast::suite", &[0.2, 31.0])).unwrap(),
    );
    assert_eq!(report.findings.len(), 1, "{:#?}", report.findings);
    assert!(
        report.findings[0].starts_with("fast::suite took 31.200 s"),
        "{:#?}",
        report.findings
    );
}

#[test]
fn an_exception_with_reason_and_owner_raises_only_its_binary() {
    let report = audit(
        &register(),
        parse_junit(&junit("slow::daemon", &[40.0])).unwrap(),
    );
    assert!(report.findings.is_empty(), "{:#?}", report.findings);
    let report = audit(
        &register(),
        parse_junit(&junit("slow::daemon", &[46.0])).unwrap(),
    );
    assert!(
        report.findings[0].contains("budget 45.000 s"),
        "{:#?}",
        report.findings
    );
}

#[test]
fn the_budget_is_the_sum_of_test_times_not_the_span() {
    // Two 20 s tests that ran together: span 20 s, Σ 40 s — red on Σ, whatever
    // the scheduler did.
    let parallel = parse_junit(&junit("x::y", &[20.0, 20.0])).unwrap();
    assert!((parallel[0].span_seconds - 20.0).abs() < 1e-6);
    assert!((parallel[0].sum_seconds - 40.0).abs() < 1e-6);
    let report = audit(&plain(), parallel);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.starts_with("x::y took 40.000 s")),
        "{:#?}",
        report.findings
    );
    // The same tests back to back: span 40 s, Σ unchanged.
    let serial = format!(
        "{}<testcase name=\"b\" classname=\"x::y\" \
         timestamp=\"2026-10-23T10:00:20.000+00:00\" time=\"20\"/>",
        testcase("20")
    );
    let times = parse_junit(&serial).unwrap();
    assert!((times[0].span_seconds - 40.0).abs() < 1e-6);
    assert!((times[0].sum_seconds - 40.0).abs() < 1e-6);
}

#[test]
fn the_suite_total_is_budgeted() {
    let mut tight = plain();
    tight.total_seconds = 10.0;
    let report = audit(&tight, parse_junit(&junit("fast::suite", &[11.0])).unwrap());
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.contains("exceeds total_seconds 10.000")),
        "{:#?}",
        report.findings
    );
}

#[test]
fn an_empty_report_cannot_pass() {
    assert!(parse_junit("<testsuites></testsuites>").is_err());
}

#[test]
fn a_non_finite_or_negative_junit_time_is_refused() {
    for time in ["NaN", "inf", "-1.0"] {
        let error = parse_junit(&testcase(time)).expect_err(time);
        assert!(error.contains("not a finite non-negative"), "{error}");
    }
    assert!(parse_junit(&testcase("0.000")).is_ok());
}

#[test]
fn a_non_finite_or_non_positive_budget_is_refused() {
    assert_refused(
        "default_seconds = nan\ntotal_seconds = 300\n",
        "must be finite and positive",
    );
    assert_refused(
        "default_seconds = 30\ntotal_seconds = inf\n",
        "must be finite and positive",
    );
    assert_refused(
        "default_seconds = 0\ntotal_seconds = 300\n",
        "must be finite and positive",
    );
    assert_refused(
        &REGISTER.replace("seconds = 45", "seconds = nan"),
        "[[exception]] slow::daemon grants NaN s — must be finite and positive",
    );
    assert_refused(
        &REGISTER.replace("seconds = 45", "seconds = inf"),
        "must be finite and positive",
    );
}

#[test]
fn an_exception_without_owner_or_not_above_default_is_refused() {
    let no_owner = REGISTER.replace("owner = \"20-12\"", "owner = \"\"");
    assert_refused(&no_owner, "needs a reason and an owner");
    let pointless = REGISTER.replace("seconds = 45", "seconds = 30");
    assert_refused(&pointless, "not above default_seconds");
}

#[test]
fn an_expensive_row_with_an_empty_job_reason_or_owner_is_refused() {
    assert!(load(EXPENSIVE).is_ok());
    for field in [
        "job = \"expensive-tests\"",
        "reason = \"spawns cargo\"",
        "owner = \"20-12\"",
    ] {
        let key = field.split(' ').next().unwrap();
        let blank = EXPENSIVE.replace(field, &format!("{key} = \"  \""));
        assert_refused(
            &blank,
            "[[expensive]] foo::bar needs a job, a reason and an owner",
        );
    }
}

#[test]
fn a_binary_listed_twice_is_refused() {
    let exception = REGISTER.split("[[exception]]").nth(1).unwrap();
    assert_refused(
        &format!("{REGISTER}\n[[exception]]{exception}"),
        "[[exception]] slow::daemon is listed twice",
    );
    let expensive = EXPENSIVE.split("[[expensive]]").nth(1).unwrap();
    assert_refused(
        &format!("{EXPENSIVE}\n[[expensive]]{expensive}"),
        "[[expensive]] foo::bar is listed twice",
    );
}

#[test]
fn a_stale_exception_reds() {
    // `slow::daemon` is excepted but absent from the junit.
    let report = audit(
        &register(),
        parse_junit(&junit("fast::suite", &[1.0])).unwrap(),
    );
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.contains("[[exception]] slow::daemon names a binary absent from the junit")),
        "{:#?}",
        report.findings
    );
}

#[test]
fn the_committed_register_parses() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let budgets = load_budgets(root).expect("xtask/test-budgets.toml parses");
    assert_eq!(budgets.default_seconds, 30.0);
    // 18-0 AC5 (ruling R3): calibrated on five CI junits, 300 -> 800.
    assert_eq!(budgets.total_seconds, 800.0);
}
