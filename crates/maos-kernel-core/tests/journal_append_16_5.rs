use std::sync::Arc;

use maos_domain::invariants::i10::{JournalEntry, LifecycleEntry, LifecycleEvent};
use maos_kernel_core::journal::JournalAdapter;

#[test]
fn independent_adapters_append_complete_non_overlapping_records() {
    const RECORDS_PER_WRITER: usize = 200;
    let temp = tempfile::tempdir().expect("temp journal");
    let path = temp.path().join("lifecycle.ndjson");
    let first = Arc::new(JournalAdapter::open(&path).expect("first writer"));
    let second = Arc::new(JournalAdapter::open(&path).expect("second writer"));

    // Review 2026-09-17: hold both writers on a barrier so their write
    // lifetimes genuinely OVERLAP — without it a regression to the two-syscall
    // `write!` shape could pass with non-overlapping writers (the AC4
    // cursor-overwrite window is only exercised by concurrent writers).
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let write =
        |journal: Arc<JournalAdapter>, prefix: &'static str, barrier: Arc<std::sync::Barrier>| {
            std::thread::spawn(move || {
                barrier.wait();
                for index in 0..RECORDS_PER_WRITER {
                    journal.append_transition(JournalEntry::Lifecycle(LifecycleEntry {
                        timestamp: index as u64,
                        lifecycle_event: LifecycleEvent::Load,
                        spirit_id: format!("{prefix}-{index}"),
                        payload: None,
                        effective_sandbox_tier: None,
                    }));
                }
            })
        };
    let first_writer = write(Arc::clone(&first), "a", Arc::clone(&barrier));
    let second_writer = write(Arc::clone(&second), "b", barrier);
    first_writer.join().unwrap();
    second_writer.join().unwrap();
    let report = first.recover_in_flight_with_tasks();
    assert_eq!(report.lifecycle.len(), RECORDS_PER_WRITER * 2);
}
