#![forbid(unsafe_code)]

//! Story 10.4c AC3 / Task-3.1 (review D3) — J4 §13.1 latency regression bench.
//!
//! Gate 2a "decay/regression-vs-baseline" track. Mirrors the established
//! `iac_routing_budget` / `orchestrator_fanout_nfr_perf_8` NFR-Perf-* idiom
//! (`discipline.yml` nfr-perf-1/8): a criterion bench that reuses the real
//! `run_j4_measurement` and reports the J4 P95 (the §13.1 metric) for trendable
//! regression tracking. The absolute `<10ms` gate is enforced by
//! `t_10_4c_j4_latency_gate`; this bench catches SILENT DECAY below budget that
//! the absolute threshold cannot (e.g. 1µs → 9000µs still reads GREEN).
//!
//! On a CI runner without a committed criterion baseline this runs in soft-fail
//! (advisory) mode — `continue-on-error: true` on the `nfr-perf-j4-latency` job,
//! identical to the sibling NFR-Perf benches. Hard blocking-regression-vs-
//! committed-baseline is the v1.5 hardening (the sibling benches share this
//! advisory posture; per AC3 "do NOT invent a bespoke ledger format").

use criterion::{criterion_group, criterion_main, Criterion};
use maos_bench::harness::j4::{run_j4_measurement, J4Config};
use maos_bench::report::{BenchReport, DecisionRecord};

/// §13.1 Mira-Nash Observer colocation-latency P95 budget (10ms) — matches
/// `harness/j4.rs::J4_P95_BUDGET_US`.
const J4_P95_BUDGET_US: u64 = 10_000;

/// Run the real J4 measurement journey and return its `JourneyResult`.
fn run_j4_journey() -> maos_bench::report::JourneyResult {
    let config = J4Config {
        invocation_count: 250,
        warmup_count: 10,
    };
    run_j4_measurement(&config).expect("j4 measurement")
}

fn j4_latency_regression_bench(c: &mut Criterion) {
    c.bench_function("j4_latency_regression", |b| {
        b.iter(|| {
            let journey = run_j4_journey();
            criterion::black_box(journey)
        });
    });

    // Emit a single BenchReport for the §13.1 continuity / regression record.
    let journey = run_j4_journey();
    eprintln!(
        "J4 regression track: P95={}µs P99={}µs max={}µs mean={}µs (budget={}µs, N={})",
        journey.p95_us,
        journey.p99_us,
        journey.max_us,
        journey.mean_us,
        J4_P95_BUDGET_US,
        journey.invocation_count,
    );
    let report = BenchReport::new(
        format!("j4-latency-regression-{}", journey.invocation_count),
        0,
        std::env::var("GITHUB_SHA").unwrap_or_else(|_| "untracked".into()),
        vec![journey.clone()],
        DecisionRecord::new(
            if journey.budget_met {
                "pass"
            } else {
                "soft-fail"
            }
            .into(),
            true, // j1 not measured here
            journey.budget_met,
            true, // j6 not measured here
            format!(
                "j4-latency-regression p95={}us budget={}us",
                journey.p95_us, J4_P95_BUDGET_US
            ),
            "ADR-040".into(),
        ),
    );
    let json = serde_json::to_string_pretty(&report).expect("serialize report");
    eprintln!("{json}");
}

criterion_group!(benches, j4_latency_regression_bench);
criterion_main!(benches);
