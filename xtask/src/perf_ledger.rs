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

fn run_identity() -> Result<String, String> {
    let id = std::env::var("GITHUB_RUN_ID").map_err(|_| "GITHUB_RUN_ID required")?;
    let attempt = std::env::var("GITHUB_RUN_ATTEMPT").map_err(|_| "GITHUB_RUN_ATTEMPT required")?;
    if id.is_empty() || attempt.is_empty() {
        return Err("empty run identity".into());
    }
    Ok(format!("{id}/{attempt}"))
}
pub fn calibrate(sha: &str, fingerprint: &str, reference: Option<&str>) -> Result<(), String> {
    let run_id = run_identity()?;
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
            "run_id": run_id,
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
pub fn metrics_in(log: &str) -> Result<Vec<(String, f64)>, String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for line in log.lines().filter(|l| l.starts_with("MAOS-PERF")) {
        let json = line
            .strip_prefix(METRIC_PREFIX)
            .ok_or("malformed metric prefix")?;
        let v: serde_json::Value =
            serde_json::from_str(json).map_err(|e| format!("malformed metric: {e}"))?;
        let id = v["metric"].as_str().ok_or("missing metric id")?;
        let value = v["value"].as_f64().ok_or("missing finite metric value")?;
        if !value.is_finite() || value < 0.0 || !seen.insert(id.to_owned()) {
            return Err(format!("invalid or duplicate metric {id}"));
        }
        out.push((id.to_owned(), value));
    }
    Ok(out)
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
    let run_id = run_identity()?;
    let existing = std::fs::read_to_string(ledger).map_err(|error| {
        format!("perf-ledger-append: the perf-ledger is missing at {ledger}: {error}")
    })?;
    let mut records = parse_ledger(&existing)?;
    let calibration_line = std::fs::read_to_string(calibration)
        .map_err(|error| format!("perf-ledger-append: {calibration}: {error}"))?;
    let calibration_record: Record = serde_json::from_str(calibration_line.trim())
        .map_err(|error| format!("perf-ledger-append: {calibration}: {error}"))?;
    crate::check_perf_verdict::validate_record(&calibration_record, chrono::Utc::now())?;
    if !matches!(&calibration_record, Record::Calibration { sha: s, fingerprint: f, run_id: r, .. } if s==sha && f==fingerprint && r==&run_id)
    {
        return Err(
            "calibration kind/sha/fingerprint/run_id disagrees with append arguments".into(),
        );
    }
    if records
        .iter()
        .any(|r| matches!(r, Record::Metric { sha: s, .. } if s==sha))
    {
        return Err("SHA already has observations; cannot overwrite its verdict".into());
    }
    let ts = now_rfc3339();
    let mut by_run: Vec<_> = std::fs::read_dir(logs)
        .map_err(|error| format!("perf-ledger-append: {logs}: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    by_run.sort();
    let expected: std::collections::BTreeSet<_> = budgets
        .metric
        .iter()
        .filter(|m| m.tier == "W" && m.instrument.contains("/benches/"))
        .map(|m| m.id.clone())
        .collect();
    let mut observations =
        std::collections::BTreeMap::<String, std::collections::BTreeMap<String, f64>>::new();
    for path in by_run {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("invalid log filename")?;
        let process = name
            .strip_prefix("run-")
            .and_then(|n| n.split('-').next())
            .filter(|n| ["1", "2", "3", "4", "5"].contains(n))
            .ok_or("log must identify process run-1 through run-5")?;
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        for (metric, value) in metrics_in(&text)? {
            if !expected.contains(&metric) {
                return Err(format!("unregistered metric {metric}"));
            }
            if observations
                .entry(metric.clone())
                .or_default()
                .insert(process.to_owned(), value)
                .is_some()
            {
                return Err(format!("duplicate {metric} process {process}"));
            }
        }
    }
    if expected
        .iter()
        .any(|m| observations.get(m).is_none_or(|v| v.len() != 5))
    {
        return Err("every metric requires exactly five distinct process runs".into());
    }
    records.push(calibration_record);
    let mut lines = vec![calibration_line.trim().to_owned()];
    for (metric, observations) in observations {
        let process_runs: Vec<_> = observations.keys().cloned().collect();
        let values: Vec<_> = observations.values().copied().collect();
        records.push(Record::Metric {
            run_id: run_id.clone(),
            process_runs: process_runs.clone(),
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
                "run_id": run_id, "process_runs": process_runs,
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

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct RebaselineRequest {
    metric: String,
    median: f64,
    mad: f64,
    reason: String,
    review: String,
}

pub fn rebaseline(
    ledger: &str,
    request: &str,
    sha: &str,
    branch: &str,
    fingerprint: &str,
) -> Result<(), String> {
    if branch != "main"
        || std::env::var("GITHUB_REF").as_deref() != Ok("refs/heads/main")
        || std::env::var("GITHUB_SHA").as_deref() != Ok(sha)
    {
        return Err("rebaseline requires trusted main ref and SHA".into());
    }
    let request: RebaselineRequest =
        serde_json::from_str(&std::fs::read_to_string(request).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let root = std::env::current_dir().map_err(|e| e.to_string())?;
    if !perf_budgets::load(&root)?
        .metric
        .iter()
        .any(|m| m.id == request.metric && m.tier == "W")
    {
        return Err("unknown Tier W rebaseline metric".into());
    }
    let existing = std::fs::read_to_string(ledger).map_err(|e| e.to_string())?;
    parse_ledger(&existing)?;
    let mut value = serde_json::to_value(request).map_err(|e| e.to_string())?;
    let fields = value.as_object_mut().ok_or("request object required")?;
    for (key, val) in [
        ("kind", "rebaseline".to_owned()),
        ("ts", now_rfc3339()),
        ("sha", sha.to_owned()),
        ("fingerprint", fingerprint.to_owned()),
        ("run_id", run_identity()?),
        (
            "actor",
            std::env::var("GITHUB_ACTOR").map_err(|e| e.to_string())?,
        ),
        ("writer", "perf-ledger-append".to_owned()),
    ] {
        fields.insert(key.into(), val.into());
    }
    let line = value.to_string();
    parse_ledger(&line)?;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(ledger)
        .map_err(|e| e.to_string())?;
    writeln!(file, "{line}").map_err(|e| e.to_string())
}
