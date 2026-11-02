#![forbid(unsafe_code)]

//! Story 20-7 AC1 — `check-perf-verdict`: one honest verdict per metric
//! (ADR-068 §5–§6). `PASS`, `FAIL`, `INDETERMINATE` or `ABSENT`; only `PASS` is
//! green (20-3a's EvidenceState vocabulary).
//!
//! Read from the `perf-ledger` JSON-lines ledger. Per metric, in order:
//!
//! 1. no metric record for the SHA ⇒ `ABSENT`;
//! 2. no calibration record for the SHA, a calibration coefficient of variation
//!    over `calibration_cv_max_pct` or a median drift over
//!    `calibration_drift_max_pct` ⇒ `INDETERMINATE`;
//! 3. five-run spread `(max − min) / median` over the metric's noise tolerance ⇒
//!    `INDETERMINATE`;
//! 4. the median against the budget ⇒ `FAIL`;
//! 5. a fingerprint with neither `PASS` history nor a re-baseline entry ⇒
//!    `INDETERMINATE` (a new machine state has no baseline);
//! 6. median beyond baseline × (1 + regression tolerance) **and** beyond
//!    `mad_multiplier` × the baseline's MAD ⇒ `FAIL`; otherwise `PASS`.
//!
//! A missing ledger is a loud error, never `ABSENT`. `--stale-check` is the
//! scheduled check: the newest entry older than `stale_hours` is red.

use std::cmp::Ordering;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::perf_budgets::{self, Budgets, Direction, Metric};

const GATE: &str = "check-perf-verdict";

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Record {
    Calibration {
        run_id: String,
        sha: String,
        ts: String,
        fingerprint: String,
        cpu_cv_pct: f64,
        fsync_cv_pct: f64,
        cpu_drift_pct: f64,
        fsync_drift_pct: f64,
        #[serde(default)]
        cpu_median_ns: f64,
        #[serde(default)]
        fsync_median_ns: f64,
    },
    Metric {
        run_id: String,
        process_runs: Vec<String>,
        sha: String,
        ts: String,
        fingerprint: String,
        metric: String,
        runs: Vec<f64>,
        #[serde(default = "main_branch")]
        branch: String,
        /// The verdict the runner recorded; a baseline run must have been `PASS`.
        verdict: String,
    },
    Rebaseline {
        sha: String,
        run_id: String,
        actor: String,
        review: String,
        writer: String,
        ts: String,
        fingerprint: String,
        metric: String,
        median: f64,
        mad: f64,
        #[allow(dead_code)]
        reason: String,
    },
}

fn main_branch() -> String {
    "main".to_owned()
}

impl Record {
    fn ts(&self) -> &str {
        match self {
            Self::Calibration { ts, .. }
            | Self::Metric { ts, .. }
            | Self::Rebaseline { ts, .. } => ts,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail,
    Indeterminate,
    Absent,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Indeterminate => "INDETERMINATE",
            Self::Absent => "ABSENT",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MetricVerdict {
    pub metric: String,
    pub verdict: Verdict,
    pub reason: String,
    pub binding: bool,
}

/// Parse a JSON-lines ledger; a malformed line is a loud error naming it.
pub fn parse_ledger(text: &str) -> Result<Vec<Record>, String> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            let record: Record = serde_json::from_str(line)
                .map_err(|error| format!("perf-ledger line {}: {error}", index + 1))?;
            validate_record(&record, Utc::now())
                .map_err(|e| format!("perf-ledger line {}: {e}", index + 1))?;
            Ok(record)
        })
        .collect()
}

pub fn instant(ts: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(ts)
        .map(|t| t.to_utc())
        .map_err(|e| format!("invalid timestamp {ts}: {e}"))
}
pub fn validate_record(record: &Record, now: DateTime<Utc>) -> Result<(), String> {
    if instant(record.ts())? > now {
        return Err("future record refused".into());
    }
    let positive = |v: f64| v.is_finite() && v >= 0.0;
    let valid = match record {
        Record::Calibration {
            sha,
            run_id,
            fingerprint,
            cpu_cv_pct,
            fsync_cv_pct,
            cpu_drift_pct,
            fsync_drift_pct,
            cpu_median_ns,
            fsync_median_ns,
            ..
        } => {
            !sha.is_empty()
                && !run_id.is_empty()
                && !fingerprint.is_empty()
                && [*cpu_cv_pct, *fsync_cv_pct, *cpu_median_ns, *fsync_median_ns]
                    .into_iter()
                    .all(positive)
                && cpu_drift_pct.is_finite()
                && fsync_drift_pct.is_finite()
        }
        Record::Metric {
            sha,
            run_id,
            process_runs,
            fingerprint,
            runs,
            ..
        } => {
            !sha.is_empty()
                && !run_id.is_empty()
                && !fingerprint.is_empty()
                && runs.len() == 5
                && runs.iter().copied().all(positive)
                && process_runs.len() == 5
                && process_runs.iter().all(|r| !r.is_empty())
                && process_runs
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    == 5
        }
        Record::Rebaseline {
            sha,
            run_id,
            actor,
            review,
            writer,
            median,
            mad,
            reason,
            fingerprint,
            metric,
            ..
        } => {
            !sha.is_empty()
                && !run_id.is_empty()
                && !actor.is_empty()
                && review.starts_with("https://github.com/")
                && review.contains("/pull/")
                && writer == "perf-ledger-append"
                && !reason.trim().is_empty()
                && !fingerprint.is_empty()
                && !metric.is_empty()
                && positive(*median)
                && positive(*mad)
        }
    };
    if valid {
        Ok(())
    } else {
        Err("invalid record identity, observations, numbers or rebaseline provenance".into())
    }
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        sorted[mid]
    } else {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    }
}

fn mad(values: &[f64], center: f64) -> f64 {
    median(
        &values
            .iter()
            .map(|v| (v - center).abs())
            .collect::<Vec<_>>(),
    )
}

/// The metric's value for one record: the median of its per-run quantiles.
fn representative(runs: &[f64]) -> f64 {
    median(runs)
}

fn spread(runs: &[f64]) -> f64 {
    let max = runs.iter().copied().fold(f64::MIN, f64::max);
    let min = runs.iter().copied().fold(f64::MAX, f64::min);
    let center = representative(runs);
    if center == 0.0 {
        if max == min {
            0.0
        } else {
            f64::INFINITY
        }
    } else {
        (max - min) / center
    }
}

/// How much worse than `baseline` `value` is, direction-aware (negative: better).
fn worse_by(direction: Direction, value: f64, baseline: f64) -> f64 {
    match direction {
        Direction::Max => value - baseline,
        Direction::Min => baseline - value,
    }
}

fn judge(budgets: &Budgets, metric: &Metric, sha: &str, ledger: &[Record]) -> MetricVerdict {
    let protocol = &budgets.protocol;
    let out = |verdict, reason: String| MetricVerdict {
        metric: metric.id.clone(),
        verdict,
        reason,
        binding: metric.binding_at(&protocol.phase),
    };
    let own = ledger.iter().rev().find_map(|record| match record {
        Record::Metric {
            sha: record_sha,
            metric: id,
            run_id,
            ts,
            fingerprint,
            runs,
            ..
        } if record_sha == sha && *id == metric.id => Some((ts, fingerprint, runs, run_id)),
        _ => None,
    });
    let Some((ts, fingerprint, runs, run_id)) = own else {
        return out(Verdict::Absent, format!("no record for SHA {sha}"));
    };
    if runs.len() != 5 || runs.iter().any(|v| !v.is_finite() || *v < 0.0) {
        return out(
            Verdict::Indeterminate,
            "exactly five finite process runs required".into(),
        );
    }
    if ledger
        .iter()
        .filter(
            |r| matches!(r, Record::Metric { sha: s, metric: id, .. } if s==sha && id==&metric.id),
        )
        .count()
        != 1
    {
        return out(Verdict::Fail, "duplicate SHA/metric verdict refused; preserve original evidence and use an explicitly selected new run".into());
    }
    let calibration = ledger.iter().rev().find_map(|record| match record {
        Record::Calibration {
            sha: record_sha,
            run_id: calibration_run,
            fingerprint: calibration_fingerprint,
            cpu_cv_pct,
            fsync_cv_pct,
            cpu_drift_pct,
            fsync_drift_pct,
            ..
        } if record_sha == sha
            && calibration_run == run_id
            && calibration_fingerprint == fingerprint =>
        {
            Some((*cpu_cv_pct, *fsync_cv_pct, *cpu_drift_pct, *fsync_drift_pct))
        }
        _ => None,
    });
    let Some((cpu_cv, fsync_cv, cpu_drift, fsync_drift)) = calibration else {
        return out(
            Verdict::Indeterminate,
            "no calibration record for this SHA".into(),
        );
    };
    if cpu_cv.max(fsync_cv) > protocol.calibration_cv_max_pct
        || cpu_drift.abs().max(fsync_drift.abs()) > protocol.calibration_drift_max_pct
    {
        return out(
            Verdict::Indeterminate,
            format!(
                "calibration noisy: cv cpu {cpu_cv}% fsync {fsync_cv}% (max {}%), drift cpu {cpu_drift}% fsync {fsync_drift}% (max {}%)",
                protocol.calibration_cv_max_pct, protocol.calibration_drift_max_pct
            ),
        );
    }
    let value = representative(runs);
    let observed_spread = spread(runs);
    if observed_spread > metric.noise_tolerance {
        return out(
            Verdict::Indeterminate,
            format!(
                "five-run spread {observed_spread:.3} over noise tolerance {}",
                metric.noise_tolerance
            ),
        );
    }
    if let Some(budget) = metric.budget {
        let breached = match metric.direction {
            Direction::Max => value > budget,
            Direction::Min => value < budget,
        };
        if breached {
            return out(
                Verdict::Fail,
                format!("median {value} {} breaches budget {budget}", metric.unit),
            );
        }
    }
    let mut history: Vec<(&String, f64)> = ledger
        .iter()
        .filter_map(|record| match record {
            Record::Metric {
                sha: record_sha,
                ts: record_ts,
                fingerprint: record_fingerprint,
                metric: id,
                runs,
                branch,
                verdict,
                ..
            } if record_sha != sha
                && *id == metric.id
                && record_fingerprint == fingerprint
                && branch == "main"
                && verdict == "PASS"
                && instant(record_ts).ok() < instant(ts).ok() =>
            {
                Some((record_ts, representative(runs)))
            }
            _ => None,
        })
        .collect();
    history.sort_by_key(|a| instant(a.0).ok());
    let recent: Vec<f64> = history
        .iter()
        .rev()
        .take(protocol.baseline_runs)
        .map(|(_, v)| *v)
        .collect();
    let rebaseline = ledger
        .iter()
        .filter_map(|record| match record {
            Record::Rebaseline {
                ts: record_ts,
                fingerprint: record_fingerprint,
                metric: id,
                median,
                mad,
                ..
            } if *id == metric.id
                && record_fingerprint == fingerprint
                && instant(record_ts).ok() <= instant(ts).ok()
                && history
                    .last()
                    .is_none_or(|(last, _)| instant(record_ts).ok() >= instant(last).ok()) =>
            {
                Some((instant(record_ts).ok(), (*median, *mad)))
            }
            _ => None,
        })
        .max_by_key(|(ts, _)| *ts)
        .map(|(_, pair)| pair);
    let (baseline, baseline_mad) = match (rebaseline, recent.is_empty()) {
        (Some(pair), _) => pair,
        (None, false) => {
            let base = median(&recent);
            (base, mad(&recent, base))
        }
        (None, true) => {
            return out(
                Verdict::Indeterminate,
                format!("fingerprint {fingerprint} has no baseline or re-baseline entry"),
            )
        }
    };
    let change = worse_by(metric.direction, value, baseline);
    if change > baseline * metric.regression_tolerance
        && change > protocol.mad_multiplier * baseline_mad
    {
        return out(
            Verdict::Fail,
            format!(
                "median {value} regressed from baseline {baseline} (MAD {baseline_mad}) beyond {}",
                metric.regression_tolerance
            ),
        );
    }
    out(Verdict::Pass, format!("median {value} {}", metric.unit))
}

/// One verdict per Tier W metric in the budget file.
pub fn verdicts(budgets: &Budgets, sha: &str, ledger: &[Record]) -> Vec<MetricVerdict> {
    budgets
        .metric
        .iter()
        .filter(|metric| metric.tier == "W")
        .map(|metric| judge(budgets, metric, sha, ledger))
        .collect()
}

/// `Some(age_hours)` when the newest ledger entry is older than `stale_hours`.
pub fn staleness(
    budgets: &Budgets,
    ledger: &[Record],
    now: DateTime<Utc>,
) -> Result<Option<i64>, String> {
    let times = ledger
        .iter()
        .map(|r| {
            let t = instant(r.ts())?;
            if t > now {
                return Err("perf-ledger future timestamp refused".into());
            }
            Ok(t)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let newest = times
        .into_iter()
        .max()
        .ok_or("perf-ledger holds no entries")?;
    let age = now - newest;
    Ok((age > chrono::Duration::hours(budgets.protocol.stale_hours)).then_some(age.num_hours()))
}

/// The ledger text: a file, a directory holding `ledger.jsonl`, or the
/// `origin/perf-ledger` branch. Every absence is a loud error.
fn read_ledger(path: Option<&str>) -> Result<String, String> {
    if let Some(path) = path {
        let path = Path::new(path);
        let file = if path.is_dir() {
            path.join("ledger.jsonl")
        } else {
            path.to_path_buf()
        };
        return std::fs::read_to_string(&file).map_err(|error| {
            format!("{GATE}: perf-ledger missing at {}: {error}", file.display())
        });
    }
    let output = std::process::Command::new("git")
        .args(["show", "origin/perf-ledger:ledger.jsonl"])
        .output()
        .map_err(|error| format!("{GATE}: cannot run git: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{GATE}: the perf-ledger branch is missing (git show origin/perf-ledger:ledger.jsonl): {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn run(
    sha: Option<&str>,
    ledger_path: Option<&str>,
    stale_check: bool,
    now: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let root = std::env::current_dir().map_err(|error| format!("{GATE}: no cwd: {error}"))?;
    let budgets = perf_budgets::load(&root).map_err(|error| format!("{GATE}: {error}"))?;
    let ledger =
        parse_ledger(&read_ledger(ledger_path)?).map_err(|error| format!("{GATE}: {error}"))?;
    let now = match now {
        Some(text) => DateTime::parse_from_rfc3339(text)
            .map_err(|error| format!("{GATE}: --now {text:?}: {error}"))?
            .with_timezone(&Utc),
        None => Utc::now(),
    };
    if stale_check {
        let stale =
            staleness(&budgets, &ledger, now).map_err(|error| format!("{GATE}: {error}"))?;
        if let Some(age) = stale {
            return Err(format!(
                "{GATE}: RED — newest perf-ledger entry is {age} h old (limit {} h): the stable runner is dead or silent",
                budgets.protocol.stale_hours
            ));
        }
        println!("{GATE}: PASS — ledger fresh");
        return Ok(());
    }
    let sha = sha.ok_or_else(|| format!("{GATE}: --sha is required"))?;
    let verdicts = verdicts(&budgets, sha, &ledger);
    if json {
        let rows: Vec<_> = verdicts
            .iter()
            .map(|v| {
                serde_json::json!({
                    "metric": v.metric,
                    "verdict": v.verdict.as_str(),
                    "reason": v.reason,
                    "binding": v.binding,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({ "gate": GATE, "sha": sha, "verdicts": rows })
        );
    } else {
        for v in &verdicts {
            println!("{} {} — {}", v.verdict.as_str(), v.metric, v.reason);
        }
    }
    let blocked: Vec<&MetricVerdict> = verdicts
        .iter()
        .filter(|v| v.verdict != Verdict::Pass && v.binding)
        .collect();
    for v in verdicts
        .iter()
        .filter(|v| v.verdict != Verdict::Pass && !v.binding)
    {
        eprintln!(
            "{GATE}: WOULD BLOCK once binding: {} is {}",
            v.metric,
            v.verdict.as_str()
        );
    }
    if blocked.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{GATE}: {} binding metric(s) not PASS: {}",
            blocked.len(),
            blocked
                .iter()
                .map(|v| format!("{}={}", v.metric, v.verdict.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}
