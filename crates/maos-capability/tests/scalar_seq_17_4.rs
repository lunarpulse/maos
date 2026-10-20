#![forbid(unsafe_code)]

//! Story 17-4 (D-17-4-FW-01) — `ScalarTapEvent.seq` is the store-write
//! order: unique within one store under concurrent writers, and strictly
//! increasing across one thread's writes whatever the tags.

use std::collections::BTreeSet;
use std::sync::Arc;

use maos_capability::working_memory::store::WorkingMemoryStore;

const THREADS: usize = 8;
const WRITES: usize = 2_000;

#[test]
fn seq_is_unique_and_gap_free_under_concurrent_writers() {
    let store = Arc::new(WorkingMemoryStore::new());
    let handles: Vec<_> = (0..THREADS)
        .map(|thread| {
            let store = Arc::clone(&store);
            std::thread::spawn(move || {
                (0..WRITES)
                    .map(|i| {
                        let tag = format!("tag_{}", i % 3);
                        store
                            .set_scalar(thread as u32, &format!("s{thread}"), &tag, i as f64, "t")
                            .unwrap()
                            .seq
                    })
                    .collect::<Vec<u64>>()
            })
        })
        .collect();

    let mut all = BTreeSet::new();
    for handle in handles {
        let seqs = handle.join().unwrap();
        // One thread's writes, across three tags, are strictly increasing.
        assert!(
            seqs.windows(2).all(|pair| pair[0] < pair[1]),
            "a thread's own writes must carry strictly increasing seq"
        );
        for seq in seqs {
            assert!(all.insert(seq), "seq {seq} issued twice");
        }
    }
    let total = (THREADS * WRITES) as u64;
    assert_eq!(all.len() as u64, total);
    assert_eq!(
        all.into_iter().collect::<Vec<_>>(),
        (0..total).collect::<Vec<_>>()
    );
}

#[test]
fn a_refused_write_consumes_no_seq() {
    let store = WorkingMemoryStore::new();
    assert_eq!(store.set_scalar(1, "s", "a", 1.0, "t").unwrap().seq, 0);
    assert!(store.set_scalar(1, "s", "a", f64::NAN, "t").is_err());
    assert!(store.set_scalar(1, "s", "", 1.0, "t").is_err());
    assert_eq!(store.set_scalar(1, "s", "b", 2.0, "t").unwrap().seq, 1);
}

#[test]
fn separate_stores_number_independently() {
    let a = WorkingMemoryStore::new();
    let b = WorkingMemoryStore::new();
    assert_eq!(a.set_scalar(1, "s", "x", 1.0, "t").unwrap().seq, 0);
    assert_eq!(b.set_scalar(1, "s", "x", 1.0, "t").unwrap().seq, 0);
}
