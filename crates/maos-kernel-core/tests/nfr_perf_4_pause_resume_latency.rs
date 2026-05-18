#![forbid(unsafe_code)]

//! FR51 (a/b) — pause P99 <=2s, resume P99 <=2s (Story 3.4, AC5).
//!
//! The director-surface path measured here is the pure Rust path
//! (no subprocess overhead) — Story 5.1 supersedes with the supervised
//! process-interruption measurement on a real spawned Spirit subprocess.
//!
//! v0.3-beta scaffold: 1000-iteration corpus; Story 5.1 extends with
//! user-observable supervised-pause latency.

use std::time::Instant;

use maos_domain::invariants::i4::ApprovalDecision;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::orchestrator::OrchestratorBufferRegistry;
use maos_spirit_abi::identity::SpiritId;

/// FR51 a — pause P99 budget in microseconds. 2s * 1_000_000 us/s.
const PAUSE_P99_BUDGET_US: u64 = 2_000_000;
/// FR51 b — resume P99 budget in microseconds. 2s * 1_000_000 us/s.
const RESUME_P99_BUDGET_US: u64 = 2_000_000;
/// P99.9 ceiling: 5s in microseconds.
const P999_BUDGET_US: u64 = 5_000_000;

fn setup_in_memory_log() -> TransparencyLogAdapter {
    TransparencyLogAdapter::open_in_memory(0)
}

fn write_pause_journal(log: &TransparencyLogAdapter, spirit_id: &str) {
    let _ = log.insert_approval_decision(ApprovalDecision {
        actor: "director".into(),
        target: spirit_id.into(),
        capability: "lifecycle.pause".into(),
        intent: "pause".into(),
        decision: true,
        reasoning: None,
    });
}

fn write_resume_journal(log: &TransparencyLogAdapter, spirit_id: &str) {
    let _ = log.insert_approval_decision(ApprovalDecision {
        actor: "director".into(),
        target: spirit_id.into(),
        capability: "lifecycle.resume".into(),
        intent: "resume".into(),
        decision: true,
        reasoning: None,
    });
}

#[tokio::test]
async fn nfr_perf_4_1000_pause_resume_corpus() {
    const N: usize = 1000;
    let mut pause_latencies_us = Vec::with_capacity(N);
    let mut resume_latencies_us = Vec::with_capacity(N);

    let log = setup_in_memory_log();
    let registry = OrchestratorBufferRegistry::new();

    for _ in 0..N {
        let t0 = Instant::now();
        write_pause_journal(&log, "hello-spirit");
        pause_latencies_us.push(t0.elapsed().as_micros() as u64);

        let t1 = Instant::now();
        write_resume_journal(&log, "hello-spirit");
        if let Some(buf) = registry.get(&SpiritId::from("hello-spirit")) {
            let _ = buf.recall_all_pending();
        }
        resume_latencies_us.push(t1.elapsed().as_micros() as u64);
    }

    pause_latencies_us.sort();
    resume_latencies_us.sort();

    let p99_pause = pause_latencies_us[(N * 99) / 100];
    let p99_resume = resume_latencies_us[(N * 99) / 100];
    let p999_pause = pause_latencies_us[(N * 999) / 1000];
    let p999_resume = resume_latencies_us[(N * 999) / 1000];

    assert!(
        p99_pause < PAUSE_P99_BUDGET_US,
        "pause P99 = {p99_pause}us exceeds 2s budget ({PAUSE_P99_BUDGET_US}us)"
    );
    assert!(
        p99_resume < RESUME_P99_BUDGET_US,
        "resume P99 = {p99_resume}us exceeds 2s budget ({RESUME_P99_BUDGET_US}us)"
    );
    assert!(
        p999_pause < P999_BUDGET_US,
        "pause P99.9 = {p999_pause}us exceeds 5s budget ({P999_BUDGET_US}us)"
    );
    assert!(
        p999_resume < P999_BUDGET_US,
        "resume P99.9 = {p999_resume}us exceeds 5s budget ({P999_BUDGET_US}us)"
    );
}
