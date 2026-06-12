#![forbid(unsafe_code)]

//! section_13_1_run — Operator-facing orchestrator binary.
//!
//! Runs the §13.1 J1 + J4 + J6 measurement journeys (real subprocess mode)
//! and writes a JSON report to `tests/reports/section-13-1-<sha>.json`.
//!
//! Environment variables:
//! - `MAOS_BENCH_INVOCATIONS`: number of invocations per journey (default 1000).
//! - `MAOS_BENCH_SPIRIT_BINARY`: path to the spirit bench fixture binary (default `hello-spirit-bench`).

use std::env;
use std::fs;
use std::path::Path;
use std::process;

use maos_bench::decision::decide;
use maos_bench::harness;
use maos_bench::harness::j1::{self, J1Config};
use maos_bench::report::BenchReport;

fn main() {
    if let Err(e) = run() {
        eprintln!("section_13_1_run: FAILED — {}", e);
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let invocation_count: u64 = env::var("MAOS_BENCH_INVOCATIONS")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .map_err(|e| format!("invalid MAOS_BENCH_INVOCATIONS: {}", e))?;

    let spirit_binary =
        env::var("MAOS_BENCH_SPIRIT_BINARY").unwrap_or_else(|_| "hello-spirit-bench".to_string());

    let mut harness = harness::BenchHarness::new();
    let git_sha = harness.git_sha.clone();

    // J1 measurement
    eprintln!(
        "section_13_1_run: starting J1 measurement (N={})...",
        invocation_count
    );
    let j1_config = J1Config {
        invocation_count,
        spirit_binary: spirit_binary.clone(),
    };
    let j1 =
        j1::run_j1_measurement(&j1_config).map_err(|e| format!("J1 measurement failed: {}", e))?;
    eprintln!(
        "section_13_1_run: J1 complete — P50={}us P95={}us budget_met={}",
        j1.p50_us, j1.p95_us, j1.budget_met
    );
    harness.add_journey(j1.clone());

    // J4 measurement
    eprintln!(
        "section_13_1_run: starting J4 measurement (N={})...",
        invocation_count
    );
    let j4_config = harness::j4::J4Config { invocation_count };
    let j4 = harness::j4::run_j4_measurement(&j4_config)
        .map_err(|e| format!("J4 measurement failed: {}", e))?;
    eprintln!(
        "section_13_1_run: J4 complete — P50={}us P95={}us budget_met={}",
        j4.p50_us, j4.p95_us, j4.budget_met
    );
    harness.add_journey(j4.clone());

    // J6 measurement (Story 8.5 — Diego cold-start; reported per release alongside
    // J1/J4. Not part of the J1/J4 `decide` gate — it is recorded, breach or not,
    // per the §13.1 "fix our code first; do not mask" semantics).
    eprintln!(
        "section_13_1_run: starting J6 cold-start measurement (N={})...",
        invocation_count
    );
    let j6_config = harness::j6::J6Config { invocation_count };
    let j6 = harness::j6::run_j6_measurement(&j6_config)
        .map_err(|e| format!("J6 measurement failed: {}", e))?;
    eprintln!(
        "section_13_1_run: J6 complete — P50={}us P95={}us budget_met={}",
        j6.p50_us, j6.p95_us, j6.budget_met
    );
    harness.add_journey(j6.clone());

    // Decision
    let decision = decide(&j1, &j4, Some(&j6));
    let report = BenchReport::new(
        harness.run_id,
        harness.started_at_ns,
        harness.git_sha.clone(),
        harness.journey_results,
        decision.clone(),
    );

    // Write report
    let reports_dir = Path::new("tests/reports");
    fs::create_dir_all(reports_dir).map_err(|e| format!("cannot create tests/reports: {}", e))?;

    let report_path = reports_dir.join(format!("section-13-1-{}.json", git_sha));
    let json = serde_json::to_vec_pretty(&report).map_err(|e| format!("serialization: {}", e))?;
    fs::write(&report_path, &json)
        .map_err(|e| format!("write {}: {}", report_path.display(), e))?;

    // Print summary
    println!(
        "bench-section-13-1 complete: J1 P95={}us (budget 25000us, met={}); J4 P95={}us (budget 10000us, met={}); J6 P95={}us (budget 500000us, met={}); decision={}; report={}",
        j1.p95_us, j1.budget_met,
        j4.p95_us, j4.budget_met,
        j6.p95_us, j6.budget_met,
        decision.outcome,
        report_path.display(),
    );

    Ok(())
}
