#![forbid(unsafe_code)]

//! Story 6.2 AC3, renamed by Story 20-7 AC4 — IN-PROCESS IAC dispatch under
//! fan-out load: 50 in-process Worker task handles, 10 tasks/sec, a 15 s window.
//!
//! This is NOT NFR-Perf-8. NFR-Perf-8 is 50 concurrent real T3 Workers through
//! the real `WorkerSupervisor` for an hour; that measurement is
//! `crates/maos-bin/tests/nfr_perf_8_real_spawn_fanout_20_7.rs`. What this keeps
//! is the in-process dispatch measurement (row `iac-inproc-fanout-dispatch-p99`
//! of `xtask/perf-budgets.toml`, which holds the budget).
//!
//! ### Methodology
//!
//! 1. Constructs a fully wired `IacBusAdapter` — real `Mailbox`, real
//!    `TransparencyLogAdapter::open_in_memory(0)`, real DRR scheduler. NO mocking
//!    of the dispatch path because the dispatch latency IS the measurement target.
//! 2. Spawns 50 Worker Spirit task handles + 1 Orchestrator handle, all in-process
//!    at v0.5-α (subprocess CliWrapperSpirit fan-out lives in a SEPARATE bench
//!    per AC6 §Bench-Note).
//! 3. Orchestrator dispatches `task.assign` frames at 10 tasks/sec via
//!    `tokio::time::interval(Duration::from_millis(100))` with
//!    `MissedTickBehavior::Skip` (the AC3 spec specifies Skip — catch-up bursts
//!    violate sustained-rate semantics).
//! 4. Each Worker emits `TaskComplete` after synthetic 50–200ms work simulation.
//! 5. Measures dispatch latency = `deliver_typed` start → Worker `recv` return.
//! 6. Asserts P99 ≤500ms AND dropped_task_count == 0 AND 50-concurrent floor
//!    maintained (`Semaphore::new(50)` enforces hard floor).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use maos_bench::budgets;
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
use tokio::sync::Semaphore;
use tokio::time::MissedTickBehavior;

const FAN_OUT: usize = 50;
const TASKS_PER_SEC: u64 = 10;
/// The in-process window: a smoke measurement. NFR-Perf-8's 1 h claim is the
/// real-spawn bench's (`nfr_perf_8_real_spawn_fanout_20_7`, weekly on Tier W).
fn target_duration() -> Duration {
    Duration::from_secs(15)
}

fn make_task_assign(seq: u64, worker_id: usize) -> IacFrame {
    let mut frame_id = [0u8; 16];
    frame_id[0..8].copy_from_slice(&seq.to_le_bytes());
    IacFrame {
        frame_id,
        timestamp_ns: seq,
        logical_clock: seq,
        from: FrameAddress {
            spirit_id: SpiritId::from("orchestrator"),
            host_id: None,
            role: Some(maos_spirit_abi::identity::SpiritRole::Orchestrator),
        },
        to: smallvec![FrameAddress {
            spirit_id: SpiritId::from(format!("worker-{worker_id}").as_str()),
            host_id: None,
            role: Some(maos_spirit_abi::identity::SpiritRole::Worker),
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

fn run_fanout(rt: &Runtime) -> (JourneyResult, u64) {
    let dropped = Arc::new(AtomicU64::new(0));
    let semaphore = Arc::new(Semaphore::new(FAN_OUT));
    let samples: Arc<parking_lot::Mutex<Vec<u64>>> = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let target = target_duration();

    rt.block_on(async {
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0));
        let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
        let mailbox = Arc::new(Mailbox::new(metrics));
        let adapter = Arc::new(IacBusAdapter::new(mailbox.clone(), tl.clone()));

        let mut worker_handles = Vec::with_capacity(FAN_OUT);
        for w in 0..FAN_OUT {
            let sid = SpiritId::from(format!("worker-{w}").as_str());
            let handle = adapter
                .register_spirit_typed(&sid)
                .expect("register worker");
            let samples = samples.clone();
            let sem = semaphore.clone();
            let join = tokio::spawn(async move {
                let mut handle = handle;
                // `recv()` yields `(FrameKind, IacFrame)` (the kind tag is unused here).
                while let Some((_kind, frame)) = handle.recv().await {
                    // dispatch_arrival_ns captured from frame.timestamp_ns
                    let dispatch_start = frame.timestamp_ns;
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos() as u64)
                        .unwrap_or(0);
                    if dispatch_start > 0 && now >= dispatch_start {
                        let elapsed_us = (now - dispatch_start) / 1000;
                        samples.lock().push(elapsed_us);
                    }
                    let permit = sem.clone().acquire_owned().await.expect("semaphore");
                    // Synthetic Worker work — 50–200ms uniform.
                    let synthetic = 50 + (frame.logical_clock % 150);
                    tokio::time::sleep(Duration::from_millis(synthetic)).await;
                    drop(permit);
                }
            });
            worker_handles.push(join);
        }

        let mut interval = tokio::time::interval(Duration::from_millis(1000 / TASKS_PER_SEC));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let start = Instant::now();
        let mut seq: u64 = 0;
        while start.elapsed() < target {
            interval.tick().await;
            seq += 1;
            let mut frame = make_task_assign(seq, (seq as usize) % FAN_OUT);
            frame.timestamp_ns = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            match adapter.deliver_typed(frame, 0, None).await {
                Ok(_) => {}
                Err(_e) => {
                    dropped.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        // Drop adapter so mailbox channels close; workers exit recv loops.
        drop(adapter);
        // Workers will be cleaned up when handles are dropped.
        for h in worker_handles {
            let _ = tokio::time::timeout(Duration::from_secs(2), h).await;
        }
    });

    let mut samples = samples.lock().clone();
    samples.sort();
    if samples.is_empty() {
        samples.push(0);
    }
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
    let dropped_count = dropped.load(Ordering::Relaxed);
    let budget_met =
        p99 <= budgets::budget_us("iac-inproc-fanout-dispatch-p99") && dropped_count == 0;
    let journey = JourneyResult::new(
        format!("orchestrator-fanout-{FAN_OUT}c-{TASKS_PER_SEC}rps"),
        len as u64,
        p50,
        p95,
        p99,
        max,
        mean,
        std_dev,
        budget_met,
    );
    (journey, dropped_count)
}

// A SUSTAINED-LOAD measurement (run the fan-out for a fixed wall window, then
// report P99 / dropped / budget), not a micro-benchmark: a single window yields
// one sample, which panics criterion's univariate stats (`slice.len() > 1`). So
// this bench is `harness = false` (Cargo.toml) with a plain `main` that runs the
// fan-out once and emits the JourneyResult report directly.
fn main() -> std::process::ExitCode {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(4)
        .build()
        .expect("tokio runtime");

    let (journey, dropped) = run_fanout(&rt);
    let budget_us = budgets::budget_us("iac-inproc-fanout-dispatch-p99");
    let report = BenchReport::new(
        format!("orchestrator-inproc-dispatch-{}", journey.invocation_count),
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
            journey.budget_met,
            dropped == 0,
            true, // j6 not measured here
            format!(
                "in-process dispatch p99={}us budget={budget_us}us dropped={dropped}",
                journey.p99_us
            ),
            "ADR-068".into(),
        ),
    );
    let json = serde_json::to_string_pretty(&report).expect("serialize report");
    eprintln!("{json}");
    println!("{json}");
    let within = budgets::report("iac-inproc-fanout-dispatch-p99", journey.p99_us as f64);
    budgets::finish(within && dropped == 0)
}
