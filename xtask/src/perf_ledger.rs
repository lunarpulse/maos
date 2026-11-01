#![forbid(unsafe_code)]

//! Story 20-7 AC1/AC12 — the two halves of the measurement protocol around
//! `check-perf-verdict` (ADR-068 §5–§6): `perf-calibrate` (a fixed CPU
//! workload and a fixed fsync workload, timed 30 times each) and
//! `perf-ledger-append` (the GitHub-hosted follow-up job that turns the
//! runner's bench logs plus calibration into ledger lines). Neither fakes a
//! host: the runner supplies the logs, this only arithmetic and file append.

use std::io::Write;
use std::path::Path;
use std::time::Instant;

use crate::check_perf_verdict::{parse_ledger, verdicts, Record};
use crate::perf_budgets;

const RUNS: usize = 30;
const SPIN_ITERATIONS: u64 = 20_000_000;
/// Prefix every Tier W bench prints per measured metric.
pub const METRIC_PREFIX: &str = "MAOS-PERF ";

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    values[values.len() / 2]
}

fn cv_pct(values: &[f64]) -> f64 {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    var.sqrt() / mean * 100.0
}

fn spin() -> u64 {
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    for _ in 0..SPIN_ITERATIONS {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
    }
    std::hint::black_box(state)
}

fn fsync_once(file: &mut std::fs::File) -> std::io::Result<()> {
    file.write_all(&[0x5A; 4096])?;
    file.sync_all()
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn calibrate(sha: &str, fingerprint: &str, reference: Option<&str>) -> Result<(), String> {
    let mut cpu = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let start = Instant::now();
        spin();
        cpu.push(start.elapsed().as_nanos() as f64);
    }
    let mut file =
        tempfile::tempfile_in(".").map_err(|error| format!("perf-calibrate: {error}"))?;
    let mut fsync = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let start = Instant::now();
        fsync_once(&mut file).map_err(|error| format!("perf-calibrate: fsync: {error}"))?;
        fsync.push(start.elapsed().as_nanos() as f64);
    }
    let (cpu_cv, fsync_cv) = (cv_pct(&cpu), cv_pct(&fsync));
    let (cpu_median, fsync_median) = (median(&mut cpu), median(&mut fsync));
    let (cpu_ref, fsync_ref) = match reference {
        Some(path) => reference_medians(path, fingerprint)?,
        None => (cpu_median, fsync_median),
    };
    let drift = |now: f64, base: f64| (now - base) / base * 100.0;
    println!(
        "{}",
        serde_json::json!({
            "kind": "calibration", "sha": sha, "ts": now_rfc3339(), "fingerprint": fingerprint,
            "cpu_cv_pct": cpu_cv, "fsync_cv_pct": fsync_cv,
            "cpu_drift_pct": drift(cpu_median, cpu_ref), "fsync_drift_pct": drift(fsync_median, fsync_ref),
            "cpu_median_ns": cpu_median, "fsync_median_ns": fsync_median,
        })
    );
    Ok(())
}

/// The machine's recorded calibration: its earliest calibration record.
fn reference_medians(ledger: &str, fingerprint: &str) -> Result<(f64, f64), String> {
    let text = std::fs::read_to_string(ledger)
        .map_err(|error| format!("perf-calibrate: reference ledger {ledger}: {error}"))?;
    parse_ledger(&text)?
        .into_iter()
        .find_map(|record| match record {
            Record::Calibration {
                fingerprint: seen,
                cpu_median_ns,
                fsync_median_ns,
                ..
            } if seen == fingerprint && cpu_median_ns > 0.0 => {
                Some((cpu_median_ns, fsync_median_ns))
            }
            _ => None,
        })
        .ok_or_else(|| {
            format!(
                "perf-calibrate: no calibration record for fingerprint {fingerprint} in {ledger}"
            )
        })
}

/// `(metric, value)` pairs from one bench log.
fn metrics_in(log: &str) -> Vec<(String, f64)> {
    log.lines()
        .filter_map(|line| line.strip_prefix(METRIC_PREFIX))
        .filter_map(|json| serde_json::from_str::<serde_json::Value>(json).ok())
        .filter_map(|v| Some((v["metric"].as_str()?.to_owned(), v["value"].as_f64()?)))
        .collect()
}

pub fn append(
    ledger: &str,
    logs: &str,
    calibration: &str,
    sha: &str,
    branch: &str,
    fingerprint: &str,
) -> Result<(), String> {
    let root = std::env::current_dir().map_err(|error| format!("perf-ledger-append: {error}"))?;
    let budgets = perf_budgets::load(&root)?;
    let existing = std::fs::read_to_string(ledger).map_err(|error| {
        format!("perf-ledger-append: the perf-ledger is missing at {ledger}: {error}")
    })?;
    let mut records = parse_ledger(&existing)?;
    let calibration_line = std::fs::read_to_string(calibration)
        .map_err(|error| format!("perf-ledger-append: {calibration}: {error}"))?;
    let calibration_record: Record = serde_json::from_str(calibration_line.trim())
        .map_err(|error| format!("perf-ledger-append: {calibration}: {error}"))?;
    let ts = now_rfc3339();
    let mut by_run: Vec<_> = std::fs::read_dir(logs)
        .map_err(|error| format!("perf-ledger-append: {logs}: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    by_run.sort();
    let mut runs: std::collections::BTreeMap<String, Vec<f64>> = std::collections::BTreeMap::new();
    for path in by_run {
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        for (metric, value) in metrics_in(&text) {
            runs.entry(metric).or_default().push(value);
        }
    }
    records.push(calibration_record);
    let mut lines = vec![calibration_line.trim().to_owned()];
    for (metric, values) in runs {
        records.push(Record::Metric {
            sha: sha.to_owned(),
            ts: ts.clone(),
            fingerprint: fingerprint.to_owned(),
            metric: metric.clone(),
            runs: values.clone(),
            branch: branch.to_owned(),
            verdict: String::new(),
        });
        let verdict = verdicts(&budgets, sha, &records)
            .into_iter()
            .find(|v| v.metric == metric)
            .map_or("ABSENT", |v| v.verdict.as_str());
        lines.push(
            serde_json::json!({
                "kind": "metric", "sha": sha, "ts": ts, "fingerprint": fingerprint,
                "metric": metric, "runs": values, "branch": branch, "verdict": verdict,
            })
            .to_string(),
        );
    }
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(Path::new(ledger))
        .map_err(|error| format!("perf-ledger-append: {ledger}: {error}"))?;
    for line in lines {
        writeln!(file, "{line}").map_err(|error| format!("perf-ledger-append: {error}"))?;
    }
    Ok(())
}
