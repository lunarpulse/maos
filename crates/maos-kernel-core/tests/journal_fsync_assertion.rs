//! Journal fsync P99 assertion — NFR-Rel-8 binding.
//!
//! Measures 10,000 `append_transition` calls and enforces P99 < 1ms.
//! The NFR-Rel-8 budget is binding at v0.1 and must pass unconditionally
//! on every `cargo test --workspace` run.

use std::time::Instant;

use maos_domain::invariants::i10::{JournalEntry, LifecycleEvent};
use maos_kernel_core::journal::JournalAdapter;

#[test]
fn journal_append_p99_measurement() {
    let tmpdir = tempfile::TempDir::new().unwrap();
    let path = tmpdir.path().join("journal.ndjson");
    let journal = JournalAdapter::open(&path).expect("journal open");

    let n = 10_000;
    let mut samples = Vec::with_capacity(n);
    for i in 0..n {
        let entry = JournalEntry {
            timestamp: i as u64,
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
        "journal_fsync P50 = {p50_us}µs, P99 = {p99_us}µs (NFR-Rel-8 budget: 1000µs = 1ms)"
    );

    assert!(
        p99 < 1_000_000, // 1ms in nanoseconds
        "NFR-Rel-8 binding broken: journal_fsync P99 = {p99_us}µs, budget = 1000µs"
    );
}
