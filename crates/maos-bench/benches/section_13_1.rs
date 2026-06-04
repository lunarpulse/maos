#![forbid(unsafe_code)]

//! section_13_1 — Criterion benchmark for §13.1 J0 + J1 + J4 + J6 + J-Researcher
//! measurement journeys.
//!
//! Run via:
//!   cargo bench -p maos-bench --bench section_13_1
//!   cargo bench -p maos-bench --bench section_13_1 -- --test  (fail-on-regress mode)
//!
//! AC1: completes within 5 minutes wall-clock on a 4-core developer workstation.
//!
//! NOTE: This criterion bench uses small invocation counts (smoke-scale) per
//! iteration. Real ≥1000-invocation measurements run via the `bench-section-13-1`
//! arm (`MAOS_ONE_SHOT=bench-section-13-1`) or `section_13_1_run` binary.

use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;

fn bench_j0(c: &mut Criterion) {
    let mut group = c.benchmark_group("section_13_1_j0");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("j0_butler_conversational_and_inproc", |b| {
        b.iter(|| {
            let _ = maos_bench::harness::j0::run_j0_measurement();
        });
    });

    group.finish();
}

fn bench_j1(c: &mut Criterion) {
    let mut group = c.benchmark_group("section_13_1_j1");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("j1_founder_loop_ipc", |b| {
        b.iter(|| {
            let config = maos_bench::harness::j1::J1Config {
                invocation_count: 10,
                ..Default::default()
            };
            let _ = maos_bench::harness::j1::run_j1_measurement(&config);
        });
    });

    group.finish();
}

fn bench_j4(c: &mut Criterion) {
    let mut group = c.benchmark_group("section_13_1_j4");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("j4_mira_nash_colocation", |b| {
        b.iter(|| {
            let _ = maos_bench::harness::j4::run_j4_smoke();
        });
    });

    group.finish();
}

fn bench_j6(c: &mut Criterion) {
    let mut group = c.benchmark_group("section_13_1_j6");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("j6_diego_cold_start", |b| {
        b.iter(|| {
            let _ = maos_bench::harness::j6::run_j6_smoke();
        });
    });

    group.finish();
}

fn bench_j_researcher(c: &mut Criterion) {
    let mut group = c.benchmark_group("section_13_1_j_researcher");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("j_researcher_distillation_step", |b| {
        b.iter(|| {
            let _ = maos_bench::harness::j_researcher::run_j_researcher_measurement_smoke();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_j0, bench_j1, bench_j4, bench_j6, bench_j_researcher);
criterion_main!(benches);
