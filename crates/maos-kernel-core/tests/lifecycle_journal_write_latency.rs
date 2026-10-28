#![forbid(unsafe_code)]

//! Lifecycle journal-write latency (Story 3.4 AC5; renamed by Story 19-2 AC9).
//!
//! Times what it runs: 1000 pairs of director `lifecycle.pause` /
//! `lifecycle.resume` Approval-Decision rows written to an in-memory
//! Transparency Log, the resume leg also recalling an (empty) Orchestrator
//! buffer. No Spirit is loaded, no hook runs and nothing is paused, so this is
//! NOT an FR51 measurement — was `nfr_perf_4_pause_resume_latency.rs`, whose
//! "FR51 (a/b) pause P99 <=2s" label had no mechanism behind it (rule 11).
//! FR51(a)(d) are measured in `fr51_pause_fence_19_2.rs` (D-19-2-19 A).
//!
//! The 2 s / 5 s ceilings are a regression tripwire on the journal path only.

use std::time::Instant;

use maos_domain::invariants::i4::ApprovalDecision;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::orchestrator::OrchestratorBufferRegistry;
use maos_spirit_abi::identity::SpiritId;

/// P99 ceiling of one lifecycle journal write, in microseconds (2 s).
const JOURNAL_WRITE_P99_BUDGET_US: u64 = 2_000_000;
/// P99.9 ceiling of one lifecycle journal write, in microseconds (5 s).
const JOURNAL_WRITE_P999_BUDGET_US: u64 = 5_000_000;

fn setup_in_memory_log() -> TransparencyLogAdapter {
    TransparencyLogAdapter::open_in_memory(0)
}

fn write_lifecycle_row(log: &TransparencyLogAdapter, spirit_id: &str, verb: &str) {
    let _ = log.insert_approval_decision(ApprovalDecision {
        actor: "director".into(),
        target: spirit_id.into(),
        capability: format!("lifecycle.{verb}"),
        intent: verb.into(),
        decision: true,
        reasoning: None,
    });
}

#[tokio::test]
async fn lifecycle_journal_write_1000_corpus() {
    const N: usize = 1000;
    let mut pause_row_us = Vec::with_capacity(N);
    let mut resume_row_us = Vec::with_capacity(N);

    let log = setup_in_memory_log();
    let registry = OrchestratorBufferRegistry::new();

    for _ in 0..N {
        let t0 = Instant::now();
        write_lifecycle_row(&log, "hello-spirit", "pause");
        pause_row_us.push(t0.elapsed().as_micros() as u64);

        let t1 = Instant::now();
        write_lifecycle_row(&log, "hello-spirit", "resume");
        if let Some(buf) = registry.get(&SpiritId::from("hello-spirit")) {
            let _ = buf.recall_all_pending();
        }
        resume_row_us.push(t1.elapsed().as_micros() as u64);
    }

    pause_row_us.sort();
    resume_row_us.sort();

    for (label, sample) in [
        ("lifecycle.pause row", &pause_row_us),
        ("lifecycle.resume row + buffer recall", &resume_row_us),
    ] {
        let p99 = sample[(N * 99) / 100];
        let p999 = sample[(N * 999) / 1000];
        assert!(
            p99 < JOURNAL_WRITE_P99_BUDGET_US,
            "{label}: P99 = {p99}us exceeds {JOURNAL_WRITE_P99_BUDGET_US}us"
        );
        assert!(
            p999 < JOURNAL_WRITE_P999_BUDGET_US,
            "{label}: P99.9 = {p999}us exceeds {JOURNAL_WRITE_P999_BUDGET_US}us"
        );
    }
}
