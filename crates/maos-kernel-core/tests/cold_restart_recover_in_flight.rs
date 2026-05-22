#![forbid(unsafe_code)]

//! Integration test: InFlightEntry survives journal close + reopen.
//!
//! Story 5.3 — AC6.

use std::path::PathBuf;

#[test]
fn in_flight_entry_survives_journal_reopen() {
    let tmp = tempfile::TempDir::new().unwrap();
    let journal_path = tmp.path().join("cold-restart-test.ndjson");

    let adapter = maos_kernel_core::journal::JournalAdapter::open(&journal_path).unwrap();
    adapter.append_in_flight(maos_domain::invariants::i10::InFlightEntry {
        timestamp_ns: 1_000_000,
        spirit_id: "cold-restart-spirit".into(),
        task_id: "cold-task-001".into(),
        capability_token: maos_domain::invariants::i1::TokenId([7u8; 16]),
        ttl_deadline_ns: 2_000_000,
        intent_class: "HighPrivilege".into(),
        originator_spirit_id: "originator-1".into(),
    });
    drop(adapter);

    let recovered = maos_kernel_core::journal::JournalAdapter::open(&journal_path).unwrap();
    let report = recovered.recover_in_flight_with_tasks();

    assert_eq!(report.in_flight.len(), 1, "expected exactly 1 in-flight entry");
    let entry = &report.in_flight[0];
    assert_eq!(entry.spirit_id, "cold-restart-spirit");
    assert_eq!(entry.task_id, "cold-task-001");
    assert_eq!(entry.capability_token, maos_domain::invariants::i1::TokenId([7u8; 16]));
}
