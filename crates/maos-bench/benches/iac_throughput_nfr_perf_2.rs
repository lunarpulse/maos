#![forbid(unsafe_code)]

//! Story 20-7 AC4 — NFR-Perf-2: sustained IAC frame throughput, single host
//! (5,000–10,000 frames/s before the log writer becomes the bottleneck;
//! `non-functional-requirements.md:10`).
//!
//! `PRODUCERS` concurrent senders each deliver `TaskAssign` frames to their own
//! drained Worker mailbox through the real `IacBusAdapter` (real mailbox, real
//! transparency-log writer) for a fixed wall window, and the delivered count
//! over the elapsed time is the metric `nfr-perf-2-iac-throughput`
//! (`xtask/perf-budgets.toml`, direction `min`). `MAOS_BENCH_WINDOW_SECS`
//! sizes the window (default 3 s smoke; the runner's protocol sets its own).

use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use maos_bench::budgets;
use maos_domain::frame::{
    FrameAddress, FramePayload, IacFrame, PosturePreferences, TaskAssignPayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::{IacBusAdapter, Mailbox, TransparencyLogAdapter};
use maos_spirit_abi::identity::{FrameKind, SpiritId};
use smallvec::smallvec;

const PRODUCERS: usize = 4;

fn make_frame(seq: u64, worker: usize) -> IacFrame {
    let mut frame_id = [0u8; 16];
    frame_id[0..8].copy_from_slice(&seq.to_le_bytes());
    frame_id[8..16].copy_from_slice(&(worker as u64).to_le_bytes());
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
            spirit_id: SpiritId::from(format!("worker-{worker}").as_str()),
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

fn window() -> Duration {
    let secs = std::env::var("MAOS_BENCH_WINDOW_SECS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(3);
    Duration::from_secs(secs)
}

fn main() -> ExitCode {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(PRODUCERS + 1)
        .build()
        .expect("tokio runtime");
    let delivered = Arc::new(AtomicU64::new(0));
    let window = window();
    let elapsed = rt.block_on(async {
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0));
        let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
        let adapter = Arc::new(IacBusAdapter::new(Arc::new(Mailbox::new(metrics)), tl));
        let mut producers = Vec::with_capacity(PRODUCERS);
        let start = Instant::now();
        for worker in 0..PRODUCERS {
            let mut handle = adapter
                .register_spirit_typed(&SpiritId::from(format!("worker-{worker}").as_str()))
                .expect("register worker");
            tokio::spawn(async move { while handle.recv().await.is_some() {} });
            let adapter = Arc::clone(&adapter);
            let delivered = Arc::clone(&delivered);
            producers.push(tokio::spawn(async move {
                let mut seq = 0u64;
                while start.elapsed() < window {
                    seq += 1;
                    if adapter
                        .deliver_typed(make_frame(seq, worker), 0, None)
                        .await
                        .is_ok()
                    {
                        delivered.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }));
        }
        for producer in producers {
            producer.await.expect("producer task");
        }
        start.elapsed()
    });
    let frames_per_s = delivered.load(Ordering::Relaxed) as f64 / elapsed.as_secs_f64();
    eprintln!(
        "iac-throughput: {frames_per_s:.0} frames/s over {elapsed:?} ({PRODUCERS} producers)"
    );
    budgets::finish(budgets::report("nfr-perf-2-iac-throughput", frames_per_s))
}
