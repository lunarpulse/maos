#![forbid(unsafe_code)]

//! Story 6.2 AC1 Task 0.1 — Story 6.1 D-4.* carry-forward closure.
//!
//! Measures per-frame IAC routing latency through `IacBusAdapter::deliver_typed`.
//! The bench reports P50/P95/P99 over `BUDGET_INVOCATIONS` invocations and writes
//! a `BenchReport` JSON shaped per `crates/maos-bench/src/report.rs` for §13.1
//! continuity.
//!
//! NFR-Perf-1 baseline: per-frame routing P95 ≤ 1000us at v0.5-α (soft-fail
//! calibration; refined to NFR-Perf-8's 500ms P99 fan-out floor in Story 6.2 AC3).
//!
//! ### Soft-fail calibration mode
//!
//! On a CI runner without dedicated baseline measurement, this bench runs in
//! `--quick` mode (invocation_count=200, no panic on breach). The CI job has
//! `continue-on-error: true` per Epic 5 §13.1 calibration-phase precedent;
//! flip to hard-fail in a follow-up PR before Epic 6 closes.

use std::sync::Arc;
use std::time::Instant;

use criterion::{criterion_group, criterion_main, Criterion};
use maos_bench::report::{BenchReport, DecisionRecord, JourneyResult};
use maos_domain::frame::{
    FrameAddress, FramePayload, IacFrame, PosturePreferences, TaskAssignPayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::{IacBusAdapter, Mailbox, TransparencyLogAdapter};
use maos_spirit_abi::identity::{FrameKind, SpiritId};
use smallvec::smallvec;
use tokio::runtime::Runtime;

const BUDGET_INVOCATIONS: usize = 200;
const P95_BUDGET_US: u64 = 1000; // v0.5-α soft-fail calibration floor

fn make_frame(seq: u64) -> IacFrame {
    let mut frame_id = [0u8; 16];
    frame_id[0..8].copy_from_slice(&seq.to_le_bytes());
    IacFrame {
        frame_id,
        timestamp_ns: 0,
        logical_clock: seq,
        from: FrameAddress {
            spirit_id: SpiritId::from("orchestrator"),
            host_id: None,
            role: None,
        },
        to: smallvec![FrameAddress {
            spirit_id: SpiritId::from("worker"),
            host_id: None,
            role: None,
        }],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: format!("synthetic-{seq}"),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}

fn run_routing_journey(rt: &Runtime) -> JourneyResult {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
    let mailbox = Arc::new(Mailbox::new(metrics));
    let adapter = IacBusAdapter::new(mailbox.clone(), tl.clone());

    // Drain the worker mailbox so deliver_typed never backpressures.
    let worker_handle = rt.block_on(async {
        adapter
            .register_spirit_typed(&SpiritId::from("worker"))
            .expect("register worker")
    });
    let _drain = rt.spawn({
        let mut handle = worker_handle;
        async move {
            loop {
                match handle.recv().await {
                    Some(_frame) => continue,
                    None => break,
                }
            }
        }
    });

    let mut samples = Vec::with_capacity(BUDGET_INVOCATIONS);
    for i in 0..BUDGET_INVOCATIONS {
        let frame = make_frame(i as u64);
        let start = Instant::now();
        rt.block_on(adapter.deliver_typed(frame))
            .expect("deliver should succeed");
        samples.push(start.elapsed().as_micros() as u64);
    }

    samples.sort();
    let len = samples.len();
    let p50 = samples[len / 2];
    let p95 = samples[(len * 95) / 100];
    let p99 = samples[(len * 99) / 100];
    let max = *samples.last().unwrap();
    let mean = samples.iter().copied().sum::<u64>() / len as u64;
    let std_dev = {
        let var: f64 = samples
            .iter()
            .map(|&v| {
                let d = v as f64 - mean as f64;
                d * d
            })
            .sum::<f64>()
            / len as f64;
        var.sqrt() as u64
    };
    let budget_met = p95 <= P95_BUDGET_US;
    JourneyResult::new(
        "iac-routing-budget".to_string(),
        len as u64,
        p50,
        p95,
        p99,
        max,
        mean,
        std_dev,
        budget_met,
    )
}

fn iac_routing_bench(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    c.bench_function("iac_routing_budget", |b| {
        b.iter(|| {
            let journey = run_routing_journey(&rt);
            criterion::black_box(journey)
        });
    });

    // Emit a single BenchReport for the §13.1 continuity record.
    let journey = run_routing_journey(&rt);
    let report = BenchReport::new(
        format!("iac-routing-{}", journey.invocation_count),
        0,
        std::env::var("GITHUB_SHA").unwrap_or_else(|_| "untracked".into()),
        vec![journey.clone()],
        DecisionRecord::new(
            if journey.budget_met { "pass" } else { "soft-fail" }.into(),
            journey.budget_met,
            true, // not relevant for this bench
            format!(
                "iac-routing p95={}us budget={}us",
                journey.p95_us, P95_BUDGET_US
            ),
            "ADR-040".into(),
        ),
    );
    let json = serde_json::to_string_pretty(&report).expect("serialize report");
    eprintln!("{json}");
}

criterion_group!(benches, iac_routing_bench);
criterion_main!(benches);
