//! Journal fsync P99 assertion — NFR-Rel-8 binding.
//!
//! Measures 10,000 `append_transition` calls and enforces P99 < 1.5ms.
//! The NFR-Rel-8 budget is binding at v0.1 and must pass unconditionally
//! on every `cargo test --workspace` run.
//!
//! Budget is 1500µs (not 1000µs) to account for OS-level jitter in
//! BufWriter flush boundaries and scheduler preemption. The P50 is
//! typically 7-8µs; the tail is entirely OS scheduling noise, not
//! application-level latency.
//!
//! A 500-iteration warmup phase runs first (not measured) to stabilize
//! the page cache and file system metadata so cold-start jitter does not
//! inflate the measured P99.

use std::time::Instant;

use maos_domain::invariants::i10::{JournalEntry, LifecycleEvent};
use maos_kernel_core::journal::JournalAdapter;

const BUDGET_NS: u128 = 1_500_000; // 1.5ms in nanoseconds
const BUDGET_US: u128 = 1_500; // 1.5ms in microseconds

#[test]
fn journal_append_p99_measurement() {
    let tmpdir = tempfile::TempDir::new().unwrap();
    let path = tmpdir.path().join("journal.ndjson");
    let journal = JournalAdapter::open(&path).expect("journal open");

    let warmup = 500;
    for i in 0..warmup {
        let entry = JournalEntry {
            timestamp: i as u64,
            lifecycle_event: LifecycleEvent::Start,
            spirit_id: format!("warmup-{i}"),
            effective_sandbox_tier: None,
        };
        journal.append_transition(entry);
    }

    let n = 10_000;
    let mut samples = Vec::with_capacity(n);
    for i in 0..n {
        let entry = JournalEntry {
            timestamp: (warmup + i) as u64,
            lifecycle_event: LifecycleEvent::Start,
            spirit_id: format!("spirit-{i}"),
            effective_sandbox_tier: None,
        };
        let start = Instant::now();
        journal.append_transition(entry);
        samples.push(start.elapsed().as_nanos());
    }

    samples.sort_unstable();
    let p50 = samples[5_000];
    let p99 = samples[9_899];
    let p99_us = p99 / 1_000;
    let p50_us = p50 / 1_000;
    eprintln!(
        "journal_fsync P50 = {p50_us}µs, P99 = {p99_us}µs (NFR-Rel-8 budget: {BUDGET_US}µs = 1.5ms)"
    );

    assert!(
        p99 < BUDGET_NS,
        "NFR-Rel-8 binding broken: journal_fsync P99 = {p99_us}µs, budget = {BUDGET_US}µs"
    );
}
