//! Story 10.4c — Gate 1 mutation test (AC2 / D4).
//!
//! Requires `bench-fault-inject` feature. Verifies that injecting a ≥15ms delay
//! INSIDE the measured `scalar.tap` colocation region causes the P95 to cross
//! the 10000µs budget → RED. This is the "stranger's falsifier and mechanical
//! no-re-can clause" (D4): if anyone re-stubs the harness to constants, the
//! injection cannot move the number → this test goes green-when-it-must-be-red.
//!
//! Run via: `cargo test -p maos-bench --features bench-fault-inject --test t_10_4c_j4_mutation`

// These tests only make sense when bench-fault-inject is active — otherwise
// the injection hook is compiled out and the normal GREEN path runs.
#[cfg(feature = "bench-fault-inject")]
use maos_bench::harness::j4::{run_j4_measurement, J4Config};
/// AC2 / D4: With fault injection, the measured P95 must CROSS the 10000µs budget.
///
/// The `bench-fault-inject` feature injects ≥15ms inside the measured span, so
/// the P95 should be ≥15000µs (injection ≈15ms + real latency ≈0-1ms). We gate
/// on P95 > 10000µs (the budget threshold) with tolerance.
#[cfg(feature = "bench-fault-inject")]
#[test]
fn t_10_4c_mutation_injection_causes_red() {
    let config = J4Config {
        invocation_count: 250,
        warmup_count: 10,
    };
    let result = run_j4_measurement(&config).expect("j4 measurement");

    // The injected ≥15ms delay must push P95 above the 10000µs budget.
    assert!(
        !result.budget_met,
        "J4 mutation test: budget_met must be FALSE with fault injection — \
         the P95 ({p95}µs) did not cross the 10000µs budget. If the injection \
         cannot move the number, the harness has been re-stubbed to constants (D4).",
        p95 = result.p95_us,
    );

    // Verify the injection moved the number by the expected amount (±tolerance).
    // Injection is ≥15ms = 15000µs; with real latency the P95 should be ≥14000µs.
    assert!(
        result.p95_us >= 14_000,
        "J4 mutation test: P95 ({p95}µs) is below 14000µs — the injection \
         (≥15ms) should add ≥14000µs to each sample. If the number didn't move, \
         the harness may not be measuring the injected region.",
        p95 = result.p95_us,
    );

    eprintln!(
        "J4 mutation test PASSED: P95={}µs (>10000µs budget, injection moves the number)",
        result.p95_us,
    );
}

/// AC2 / D4: sample count and non-degeneracy hold under fault injection.
#[cfg(feature = "bench-fault-inject")]
#[test]
fn t_10_4c_mutation_sample_integrity() {
    let config = J4Config {
        invocation_count: 250,
        warmup_count: 10,
    };
    let result = run_j4_measurement(&config).expect("j4 measurement");

    // samples_received == invocation_count (tap-arrival-count).
    assert_eq!(
        result.invocation_count, 250,
        "sample count must equal invocation_count under fault injection"
    );

    // Non-degeneracy: variance > 0 (not all samples identical).
    let has_variation = result.std_dev_us > 0 || result.max_us > result.p50_us;
    assert!(
        has_variation,
        "non-degeneracy failed under fault injection: std_dev={}, max={}, p50={}",
        result.std_dev_us, result.max_us, result.p50_us,
    );
}
