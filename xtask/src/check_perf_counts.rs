#![forbid(unsafe_code)]

//! Story 20-7 AC5 — Tier C: instruction and allocation counts per pull request
//! (ADR-068 §1, D-20-7-07 A). `check-perf-counts --dir <out>` compares what the
//! `tier-c-harness` binary measured with the baselines committed in
//! `xtask/perf-budgets.toml`.
//!
//! The directory holds, per harness path `P` (including the empty path `empty`):
//! `P.json` — `{"path","iterations","allocations"}` printed by the harness's
//! counting allocator — and `P.callgrind` — the `valgrind --tool=callgrind`
//! output of the same run. Instructions per iteration are
//! `(Ir(P) − Ir(empty)) / iterations(P)`, so process start-up cost cancels;
//! allocations per iteration are counted inside the measured loop only.
//!
//! A row without a committed baseline reads `ABSENT` (never `PASS`); a count
//! over `baseline × (1 + tolerance)` is `FAIL`; a row blocks only where its
//! per-phase disposition is `blocking`. A deliberate change re-baselines in the
//! same commit, with the measured figure and the reason.

use std::path::Path;

use crate::check_perf_verdict::Verdict;
use crate::perf_budgets::{self, Metric};

const GATE: &str = "check-perf-counts";
pub const EMPTY: &str = "empty";

/// The total instruction count of a callgrind output (`totals:` else `summary:`).
pub fn callgrind_total(text: &str) -> Option<u64> {
    ["totals:", "summary:"].iter().find_map(|prefix| {
        text.lines()
            .rev()
            .find_map(|line| line.strip_prefix(prefix))
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|count| count.parse().ok())
    })
}

#[derive(Debug, Clone, Copy)]
pub struct Measured {
    pub iterations: u64,
    pub allocations: u64,
    pub instructions: u64,
}

fn measure(dir: &Path, path: &str, need_instructions: bool) -> Result<Measured, String> {
    let json: serde_json::Value = std::fs::read_to_string(dir.join(format!("{path}.json")))
        .map_err(|error| format!("{path}.json: {error}"))
        .and_then(|text| {
            serde_json::from_str(&text).map_err(|error| format!("{path}.json: {error}"))
        })?;
    let field = |name: &str| {
        json[name]
            .as_u64()
            .ok_or_else(|| format!("{path}.json: `{name}` missing"))
    };
    let instructions = if need_instructions {
        let text = std::fs::read_to_string(dir.join(format!("{path}.callgrind")))
            .map_err(|error| format!("{path}.callgrind: {error}"))?;
        callgrind_total(&text).ok_or_else(|| format!("{path}.callgrind: no totals/summary line"))?
    } else {
        0
    };
    if json["path"].as_str() != Some(path)
        || (path != EMPTY && field("iterations")? == 0)
        || (path == EMPTY && (field("iterations")? != 0 || field("allocations")? != 0))
    {
        return Err(format!(
            "{path}.json: invalid path/iterations/empty baseline"
        ));
    }
    Ok(Measured {
        iterations: field("iterations")?,
        allocations: field("allocations")?,
        instructions,
    })
}

/// Per-iteration figure for one Tier C row.
pub fn figure(metric: &Metric, path: &Measured, empty: &Measured) -> Result<f64, String> {
    if path.iterations == 0
        || (metric.unit == "instructions" && path.instructions < empty.instructions)
    {
        return Err("invalid measured iterations or instruction total below empty baseline".into());
    }
    Ok(match metric.unit.as_str() {
        "allocations" => path.allocations as f64 / path.iterations as f64,
        _ => (path.instructions - empty.instructions) as f64 / path.iterations as f64,
    })
}

pub fn judge(metric: &Metric, measured: f64) -> (Verdict, String) {
    if !measured.is_finite() || measured < 0.0 {
        return (
            Verdict::Fail,
            "invalid non-finite or negative measurement".into(),
        );
    }
    let Some(baseline) = metric.baseline else {
        return (
            Verdict::Absent,
            format!(
                "measured {measured:.1} {}; no committed baseline",
                metric.unit
            ),
        );
    };
    if measured > baseline * (1.0 + metric.regression_tolerance) {
        (
            Verdict::Fail,
            format!(
                "measured {measured:.1} {} over baseline {baseline} (+{:.1}% tolerance {}%)",
                metric.unit,
                (measured / baseline - 1.0) * 100.0,
                metric.regression_tolerance * 100.0
            ),
        )
    } else {
        (
            Verdict::Pass,
            format!(
                "measured {measured:.1} {} (baseline {baseline})",
                metric.unit
            ),
        )
    }
}

pub fn run(dir: &str, json: bool) -> Result<(), String> {
    let root = std::env::current_dir().map_err(|error| format!("{GATE}: no cwd: {error}"))?;
    let budgets = perf_budgets::load(&root).map_err(|error| format!("{GATE}: {error}"))?;
    let dir = Path::new(dir);
    let rows: Vec<&Metric> = budgets.metric.iter().filter(|m| m.tier == "C").collect();
    let needs_instructions = rows.iter().any(|m| m.unit == "instructions");
    let empty = measure(dir, EMPTY, needs_instructions).map_err(|e| format!("{GATE}: {e}"))?;
    let mut blocked = Vec::new();
    let mut report = Vec::new();
    for metric in rows {
        let path = metric.path.as_deref().unwrap_or(&metric.id);
        let measured = measure(dir, path, metric.unit == "instructions")
            .map_err(|error| format!("{GATE}: {error}"))?;
        let (verdict, reason) = judge(metric, figure(metric, &measured, &empty)?);
        let binding = metric.binding_at(&budgets.protocol.phase);
        println!("{} {} — {reason}", verdict.as_str(), metric.id);
        if verdict != Verdict::Pass && !binding {
            eprintln!(
                "{GATE}: WOULD BLOCK once binding: {} is {}",
                metric.id,
                verdict.as_str()
            );
        }
        if verdict != Verdict::Pass && binding {
            blocked.push(format!("{}={}", metric.id, verdict.as_str()));
        }
        report.push(serde_json::json!({"metric": metric.id, "verdict": verdict.as_str(), "reason": reason, "binding": binding}));
    }
    if json {
        println!(
            "{}",
            serde_json::json!({ "gate": GATE, "verdicts": report })
        );
    }
    if blocked.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{GATE}: binding count(s) not PASS: {}",
            blocked.join(", ")
        ))
    }
}
