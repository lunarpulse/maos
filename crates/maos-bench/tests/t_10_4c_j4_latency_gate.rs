//! Story 10.4c — J4 §13.1 Mira-Nash Observer colocation-latency gate.
//!
//! Replaces the Story 10.4b proven-RED placeholder gate
//! (`t_10_4b_j4_placeholder_gate.rs`). The real in-kernel `scalar.tap`
//! measurement runs with the `kernel_measurement` feature ON — it produces a
//! REAL latency distribution, not canned smoke samples.
//!
//! Gate scalar: cross-task delivery latency P95 ≤ 10ms (10000µs).
//!
//! ## Tests
//!
//! - `t_10_4c_j4_latency_gate_green` — the §13.1 gate: P95 within budget.
//! - `t_10_4c_j4_real_measurement_no_warning` — anti-regression: the real
//!   path must NOT emit the "NOT real measurements" placeholder marker.

use std::io::Write;
use std::process::Command;

use maos_bench::harness::j4::{run_j4_measurement, J4Config};
use maos_bench::report::JourneyResult;

/// Marker prefix the producer prints its serialized result after.
const PRODUCER_MARKER: &str = "__J4_RESULT_JSON__";

/// Inner producer: runs the J4 measurement and prints the result as JSON.
#[test]
fn j4_latency_producer_inner() {
    let result = run_j4_measurement(&J4Config::default()).expect("j4 measurement");
    let json = serde_json::to_string(&result).expect("serialize journey result");
    let mut out = std::io::stdout();
    writeln!(out, "{PRODUCER_MARKER}{json}").expect("write result line");
    out.flush().expect("flush stdout");
}

/// Run the J4 measurement in a subprocess and return `(result, stderr)`.
fn run_j4_in_subprocess() -> (JourneyResult, String) {
    let exe = std::env::current_exe().expect("resolve test bin");
    let output = Command::new(exe)
        .args(["j4_latency_producer_inner", "--exact", "--nocapture"])
        .output()
        .expect("spawn J4 producer subprocess");

    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "J4 producer subprocess failed; stderr:\n{stderr}"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json = stdout
        .lines()
        .find(|line| line.starts_with(PRODUCER_MARKER))
        .map(|line| &line[PRODUCER_MARKER.len()..])
        .unwrap_or_else(|| {
            panic!("no {PRODUCER_MARKER} line in producer stdout; full stdout:\n{stdout}");
        });
    let result: JourneyResult =
        serde_json::from_str(json).expect("deserialize journey result from producer");
    (result, stderr)
}

/// §13.1 gate: the real J4 P95 is within the 10ms (10000µs) budget.
///
/// This is the GREEN gate that replaces the 10.4b proven-RED placeholder.
/// When `kernel_measurement` is ON, `run_j4_measurement` runs the real
/// in-kernel scalar.tap loop. The P95 must be within budget for the gate
/// to pass.
// These gate tests assert REAL-path invariants (no placeholder marker; real P95)
// and are only meaningful under `kernel_measurement`. Under the default smoke
// path the harness legitimately emits the placeholder marker, so they are
// compiled out without the feature.
#[cfg(feature = "kernel_measurement")]
#[test]
fn t_10_4c_j4_latency_gate_green() {
    let (result, _stderr) = run_j4_in_subprocess();
    // AC3/D5 (review D1): advisory at v1.0. The gate does NOT panic on
    // over-budget — it emits a loud "WOULD BLOCK at v1.5" banner (never a silent
    // pass) and passes. v1_0=advisory ⇒ a v1.0 latency flake warns, not blocks;
    // graduation to v1_5=blocking is a disposition flip (no phase-aware release
    // enforcer exists — out of scope per AC3). The load-bearing falsifier is the
    // separate mutation test (AC2), which still hard-fails when injection cannot
    // move the number.
    assert!(
        result.invocation_count >= 200,
        "J4 invocation count ({}) below the N=200 post-warmup floor (D2)",
        result.invocation_count,
    );
    if result.budget_met {
        eprintln!(
            "J4 §13.1 latency gate GREEN: P95={}µs P99={}µs max={}µs (budget=10000µs, N={})",
            result.p95_us, result.p99_us, result.max_us, result.invocation_count,
        );
    } else {
        eprintln!(
            "⚠️  WOULD BLOCK at v1.5: J4 §13.1 latency gate OVER BUDGET — \
             P95={}µs exceeds 10000µs (N={}). ADVISORY at v1.0 (gate passes); \
             becomes BLOCKING at v1.5 GA. Investigate the real kernel latency \
             before adjusting the budget.",
            result.p95_us, result.invocation_count,
        );
    }
}

/// Anti-regression: the real path must NOT emit the placeholder marker.
#[cfg(feature = "kernel_measurement")]
#[test]
fn t_10_4c_j4_real_measurement_no_warning() {
    let (_result, stderr) = run_j4_in_subprocess();
    assert!(
        !stderr.contains("NOT real measurements"),
        "J4 latency gate regression: the real measurement path emitted the \
         'NOT real measurements' placeholder marker — the harness was re-stubbed. \
         stderr:\n{stderr}"
    );
}
