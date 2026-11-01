#![forbid(unsafe_code)]

//! Story 20-7 AC3/AC4 — J1 (ADR-040 §13.1) measured through the REAL
//! `runtime.rs` bridge (`run_j1_bridge_measurement`, Story 8.12 AC4): the strict
//! 25 ms P95 row `j1-bridge-p95` and the 50 ms shared-runner catastrophe guard
//! row `j1-bridge-ci-ceiling-p95` of `xtask/perf-budgets.toml`, which holds both.
//! On the stable runner the strict row is the verdict; on a shared runner only
//! the ceiling is believed (ADR-068 table).

use maos_bench::budgets;
use maos_bench::harness::j1::{run_j1_bridge_measurement, J1BridgeConfig};

fn main() -> std::process::ExitCode {
    let journey =
        run_j1_bridge_measurement(&J1BridgeConfig::default()).expect("J1 bridge measurement");
    eprintln!(
        "J1-bridge: P50={}us P95={}us P99={}us max={}us (N={})",
        journey.p50_us, journey.p95_us, journey.p99_us, journey.max_us, journey.invocation_count
    );
    let mut within = budgets::report("j1-bridge-p95", journey.p95_us as f64);
    within &= budgets::report("j1-bridge-ci-ceiling-p95", journey.p95_us as f64);
    budgets::finish(within)
}
