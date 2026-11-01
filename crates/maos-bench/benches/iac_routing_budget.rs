#![forbid(unsafe_code)]

//! NFR-Perf-1 — per-frame IAC routing latency through `IacBusAdapter::deliver_typed`
//! (Story 6.2 AC1; reworked by Story 20-7 AC4).
//!
//! Reports and checks NFR-Perf-1's own quantiles — P50 < 5 ms and P99 < 50 ms
//! (`non-functional-requirements.md:9`) — and the PRD hot-path row `iac/send`
//! (same-Host) P99 < 10 µs (`success-criteria.md:41`): one measurement, three
//! rows of `xtask/perf-budgets.toml`, which holds every budget. The 1 ms P95
//! "soft-fail calibration floor" this bench used to check is gone.
//!
//! Each row prints `MAOS-PERF {..}`; the process exits non-zero only when a row
//! whose disposition at the file's phase is `blocking` breaches (`-- --test` and
//! `-- --quick` run the same smoke measurement; the claim is the stable runner's).

use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

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

const INVOCATIONS: usize = 2_000;
const WARMUP: usize = 100;

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

/// Nanosecond latency of one `deliver_typed` per invocation, sorted.
fn measure() -> Vec<u64> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
    let mailbox = Arc::new(Mailbox::new(metrics));
    let adapter = IacBusAdapter::new(mailbox, tl);
    // Drain the worker mailbox so deliver_typed never backpressures.
    let mut worker = rt.block_on(async {
        adapter
            .register_spirit_typed(&SpiritId::from("worker"))
            .expect("register worker")
    });
    let _drain = rt.spawn(async move { while worker.recv().await.is_some() {} });

    let mut samples = Vec::with_capacity(INVOCATIONS);
    for i in 0..WARMUP + INVOCATIONS {
        let frame = make_frame(i as u64);
        let start = Instant::now();
        rt.block_on(adapter.deliver_typed(frame, 0, None))
            .expect("deliver should succeed");
        if i >= WARMUP {
            samples.push(start.elapsed().as_nanos() as u64);
        }
    }
    samples.sort_unstable();
    samples
}

fn quantile_us(sorted_ns: &[u64], quantile: f64) -> f64 {
    let rank = ((sorted_ns.len() as f64 * quantile).ceil() as usize).clamp(1, sorted_ns.len());
    sorted_ns[rank - 1] as f64 / 1_000.0
}

fn main() -> ExitCode {
    let samples = measure();
    let (p50, p99) = (quantile_us(&samples, 0.50), quantile_us(&samples, 0.99));
    eprintln!(
        "iac-routing: N={} P50={p50:.1}us P99={p99:.1}us",
        samples.len()
    );
    let mut within = true;
    within &= budgets::report("nfr-perf-1-iac-routing-p50", p50);
    within &= budgets::report("nfr-perf-1-iac-routing-p99", p99);
    within &= budgets::report("hot-path-iac-send-p99", p99);
    budgets::finish(within)
}
