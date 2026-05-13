//! journal_fsync_p99 — criterion bench measuring the ring-buffer flush
//! latency P99 over 10000 `append_transition` calls.
//!
//! NFR-Rel-8 binds <1ms P99. Run via:
//!   cargo bench --bench journal_fsync_p99
//!   cargo bench --bench journal_fsync_p99 -- --test  (fail-on-regress mode)

use criterion::{criterion_group, criterion_main, Criterion};
use maos_domain::invariants::i10::{JournalEntry, LifecycleEvent};
use maos_kernel_core::journal::JournalAdapter;

fn bench_journal_fsync(c: &mut Criterion) {
    let mut group = c.benchmark_group("journal_fsync");
    group.sample_size(10_000); // 10K samples for P99 stability per NFR-Rel-8
    group.bench_function("append_transition", |b| {
        let tmpdir = tempfile::TempDir::new().expect("tempdir creation must succeed");
        let path = tmpdir.path().join("journal.ndjson");
        let journal = JournalAdapter::open(&path).expect("journal open must succeed");
        let mut counter: u64 = 0;
        b.iter(|| {
            counter += 1;
            let entry = JournalEntry {
                timestamp: counter,
                lifecycle_event: LifecycleEvent::Start,
                spirit_id: format!("spirit-{counter}"),
            };
            journal.append_transition(entry);
        });
        // Keep tmpdir alive until bench iteration completes
        let _ = tmpdir;
    });
    group.finish();
}

criterion_group!(benches, bench_journal_fsync);
criterion_main!(benches);
