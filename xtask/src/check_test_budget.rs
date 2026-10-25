#![forbid(unsafe_code)]

//! Story 20-12 AC1 — `check-test-budget --junit <path>`: every test binary in
//! the nextest junit the workspace suite produced finishes under its budget in
//! `xtask/test-budgets.toml` (`default_seconds`, or an `[[exception]]` naming
//! `binary`, `seconds`, `reason` and `owner`), and the suite's total test time
//! stays under `total_seconds`.
//!
//! **What "a binary's time" is (review 2026-10-23).** Σ of its tests' junit
//! `time` — each test's own run, independent of how nextest scheduled it. The
//! span (first start → last end) is reported alongside but not budgeted: it
//! includes queue time behind other binaries' tests, so the same code measures
//! differently on a busier host. The total is Σ over every binary.
//!
//! Every budget must be finite and positive, every junit `time` finite and
//! non-negative, and every `[[exception]]` must name a binary present in the
//! junit (a stale row is a finding). Disposition: BLOCKING at v1_0 since
//! 18-0-green-main-ci-baseline (operator ruling R3 2026-10-24; calibrated on
//! five CI junits, `total_seconds` 800).
//!
//! The file is also the register of expensive tests (`[[expensive]]`), read by
//! `check-test-cost`.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

const GATE: &str = "check-test-budget";
/// The budget register, relative to the workspace root.
pub const BUDGETS_PATH: &str = "xtask/test-budgets.toml";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budgets {
    pub default_seconds: f64,
    pub total_seconds: f64,
    #[serde(default, rename = "exception")]
    pub exceptions: Vec<Exception>,
    #[serde(default)]
    pub expensive: Vec<Expensive>,
}

/// A binary allowed more than `default_seconds`, with why and who owns it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exception {
    pub binary: String,
    pub seconds: f64,
    pub reason: String,
    pub owner: String,
}

/// A binary whose `#[ignore = "expensive: …"]` tests run only in `job`
/// (`expensive-tests`, or the dedicated CI job that provisions their substrate).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expensive {
    pub binary: String,
    pub job: String,
    pub reason: String,
    pub owner: String,
}

/// Parse and validate the register: finite positive budgets, a reason and an
/// owner on every row, no binary listed twice in one table.
pub fn parse_budgets(text: &str) -> Result<Budgets, String> {
    let budgets: Budgets =
        toml::from_str(text).map_err(|e| format!("{BUDGETS_PATH} does not parse: {e}"))?;
    let mut problems = Vec::new();
    let positive = |seconds: f64| seconds.is_finite() && seconds > 0.0;
    if !positive(budgets.default_seconds) || !positive(budgets.total_seconds) {
        problems.push("default_seconds and total_seconds must be finite and positive".to_owned());
    }
    let mut seen = std::collections::BTreeSet::new();
    for row in &budgets.exceptions {
        if !seen.insert(("exception", row.binary.as_str())) {
            problems.push(format!("[[exception]] {} is listed twice", row.binary));
        }
        if !positive(row.seconds) {
            problems.push(format!(
                "[[exception]] {} grants {} s — must be finite and positive",
                row.binary, row.seconds
            ));
        } else if row.seconds <= budgets.default_seconds {
            problems.push(format!(
                "[[exception]] {} grants {} s, not above default_seconds {} — delete the row",
                row.binary, row.seconds, budgets.default_seconds
            ));
        }
        if row.reason.trim().is_empty() || row.owner.trim().is_empty() {
            problems.push(format!(
                "[[exception]] {} needs a reason and an owner",
                row.binary
            ));
        }
    }
    for row in &budgets.expensive {
        if !seen.insert(("expensive", row.binary.as_str())) {
            problems.push(format!("[[expensive]] {} is listed twice", row.binary));
        }
        if row.job.trim().is_empty() || row.reason.trim().is_empty() || row.owner.trim().is_empty()
        {
            problems.push(format!(
                "[[expensive]] {} needs a job, a reason and an owner",
                row.binary
            ));
        }
    }
    if problems.is_empty() {
        Ok(budgets)
    } else {
        Err(format!("{BUDGETS_PATH}: {}", problems.join("; ")))
    }
}

pub fn load_budgets(root: &Path) -> Result<Budgets, String> {
    let path = root.join(BUDGETS_PATH);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_budgets(&text)
}

/// One test binary's measured time in a junit report.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BinaryTime {
    pub binary: String,
    pub tests: usize,
    /// Σ per-test `time` — the budgeted metric.
    pub sum_seconds: f64,
    /// First test start → last test end (reported, not budgeted: it includes
    /// queue time).
    pub span_seconds: f64,
}

/// `key="value"` from one XML start tag (` key="` so `name` never matches
/// `classname`). nextest writes these attributes unescaped-safe: binary ids,
/// RFC 3339 timestamps and decimal seconds.
fn attribute<'a>(tag: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!(" {key}=\"");
    let start = tag.find(&needle)? + needle.len();
    tag[start..].split('"').next()
}

/// Per-binary Σ per-test time and span from a nextest junit report.
pub fn parse_junit(text: &str) -> Result<Vec<BinaryTime>, String> {
    // binary -> (tests, first start, last end, Σ time)
    let mut acc: BTreeMap<String, (usize, f64, f64, f64)> = BTreeMap::new();
    for chunk in text.split("<testcase").skip(1) {
        let tag = chunk.split('>').next().unwrap_or_default();
        let field = |key| {
            attribute(tag, key).ok_or_else(|| format!("junit <testcase> without `{key}`: {tag}"))
        };
        let binary = field("classname")?;
        let time: f64 = field("time")?
            .parse()
            .map_err(|e| format!("junit time in {tag}: {e}"))?;
        if !time.is_finite() || time < 0.0 {
            return Err(format!(
                "junit time {time} in {tag} is not a finite non-negative number"
            ));
        }
        let start = chrono::DateTime::parse_from_rfc3339(field("timestamp")?)
            .map_err(|e| format!("junit timestamp in {tag}: {e}"))?;
        let start = start.timestamp_millis() as f64 / 1000.0;
        let entry = acc
            .entry(binary.to_owned())
            .or_insert((0, f64::MAX, f64::MIN, 0.0));
        entry.0 += 1;
        entry.1 = entry.1.min(start);
        entry.2 = entry.2.max(start + time);
        entry.3 += time;
    }
    if acc.is_empty() {
        return Err("junit report has no <testcase> — a vacuous run cannot pass".to_owned());
    }
    Ok(acc
        .into_iter()
        .map(|(binary, (tests, first, last, sum))| BinaryTime {
            binary,
            tests,
            span_seconds: last - first,
            sum_seconds: sum,
        })
        .collect())
}

#[derive(Debug, serde::Serialize)]
pub struct Report {
    pub binaries: Vec<BinaryTime>,
    /// Σ per-test time over every binary — judged against `total_seconds`.
    pub total_sum_seconds: f64,
    /// Σ per-binary span (reported only).
    pub total_span_seconds: f64,
    pub findings: Vec<String>,
}

/// Judge measured binaries against the register.
pub fn audit(budgets: &Budgets, binaries: Vec<BinaryTime>) -> Report {
    let mut findings = Vec::new();
    for binary in &binaries {
        let budget = budgets
            .exceptions
            .iter()
            .find(|row| row.binary == binary.binary)
            .map_or(budgets.default_seconds, |row| row.seconds);
        if binary.sum_seconds > budget {
            findings.push(format!(
                "{} took {:.3} s (Σ per-test time) over {} test(s), budget {budget:.3} s — make \
                 it cheaper, or add an [[exception]] with reason + owner to {BUDGETS_PATH}",
                binary.binary, binary.sum_seconds, binary.tests
            ));
        }
    }
    for row in &budgets.exceptions {
        if !binaries.iter().any(|b| b.binary == row.binary) {
            findings.push(format!(
                "[[exception]] {} names a binary absent from the junit — stale row, delete it \
                 from {BUDGETS_PATH}",
                row.binary
            ));
        }
    }
    let total_sum_seconds: f64 = binaries.iter().map(|b| b.sum_seconds).sum();
    let total_span_seconds: f64 = binaries.iter().map(|b| b.span_seconds).sum();
    if total_sum_seconds > budgets.total_seconds {
        findings.push(format!(
            "suite test time {total_sum_seconds:.3} s (Σ per-test time) exceeds total_seconds \
             {:.3}",
            budgets.total_seconds
        ));
    }
    Report {
        binaries,
        total_sum_seconds,
        total_span_seconds,
        findings,
    }
}

pub fn run(junit: &Path, json: bool) -> Result<(), String> {
    let root = std::env::current_dir().map_err(|e| format!("{GATE}: no cwd: {e}"))?;
    let budgets = load_budgets(&root).map_err(|e| format!("{GATE}: {e}"))?;
    let text = std::fs::read_to_string(junit)
        .map_err(|e| format!("{GATE}: cannot read junit {}: {e}", junit.display()))?;
    let report = audit(
        &budgets,
        parse_junit(&text).map_err(|e| format!("{GATE}: {e}"))?,
    );
    let passed = report.findings.is_empty();
    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": GATE,
                "passed": passed,
                "junit": junit,
                "binaries": report.binaries.len(),
                "total_span_seconds": report.total_span_seconds,
                "total_sum_seconds": report.total_sum_seconds,
                "findings": report.findings,
            })
        );
    }
    if passed {
        if !json {
            println!(
                "{GATE}: PASS ({} binaries, {:.3} s test time, budget {:.3} s)",
                report.binaries.len(),
                report.total_sum_seconds,
                budgets.total_seconds
            );
        }
        return Ok(());
    }
    let detail: String = report.findings.iter().map(|f| format!("- {f}\n")).collect();
    let msg = format!(
        "{GATE}: FAIL — {} finding(s):\n{detail}",
        report.findings.len()
    );
    crate::gate_common::emit_command(json, "error", &msg);
    if !json {
        eprintln!("{msg}");
    }
    Err(msg)
}
