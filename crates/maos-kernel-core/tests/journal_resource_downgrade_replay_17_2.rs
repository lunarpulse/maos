use std::fs;

use maos_domain::invariants::i10::{
    JournalEntry, LifecycleEntry, LifecycleEvent, ResourceCeilingDowngradedPayload,
};
use maos_kernel_core::journal::JournalAdapter;

#[test]
fn legacy_lifecycle_and_typed_resource_downgrade_replay_from_journal() {
    let temp = tempfile::tempdir().expect("temp journal");
    let path = temp.path().join("lifecycle.ndjson");
    fs::write(
        &path,
        format!(
            "{}\n",
            r#"{"timestamp":1,"lifecycle_event":"Load","spirit_id":"legacy-spirit"}"#
        ),
    )
    .expect("write legacy lifecycle record");

    let payload = ResourceCeilingDowngradedPayload::new(
        "cgroup v2 memory.max unavailable".into(),
        Some(384),
        None,
    );
    let payload_bytes = serde_json::to_vec(&payload).expect("serialize typed payload");
    let journal = JournalAdapter::open(&path).expect("read legacy journal");
    assert_eq!(
        journal.last_event("legacy-spirit"),
        Some(LifecycleEvent::Load),
        "the untagged consumer-era lifecycle record must load through JournalAdapter"
    );
    journal.append_transition(JournalEntry::Lifecycle(LifecycleEntry {
        timestamp: 2,
        lifecycle_event: LifecycleEvent::ResourceCeilingDowngraded,
        spirit_id: "downgraded-spirit".into(),
        payload: Some(payload_bytes),
        effective_sandbox_tier: None,
    }));
    drop(journal);

    let replayed = JournalAdapter::open(&path).expect("reopen persisted journal");
    assert_eq!(
        replayed.last_event("legacy-spirit"),
        Some(LifecycleEvent::Load)
    );
    assert_eq!(
        replayed.last_event("downgraded-spirit"),
        Some(LifecycleEvent::ResourceCeilingDowngraded),
        "the event 20 index must come from replaying the raw lifecycle record"
    );
    let recovery = replayed.recover_in_flight_with_tasks();
    assert!(recovery
        .lifecycle
        .iter()
        .any(|(spirit, event)| spirit == "downgraded-spirit"
            && *event == LifecycleEvent::ResourceCeilingDowngraded));

    let contents = fs::read_to_string(&path).expect("read persisted records");
    let persisted_entry = contents
        .lines()
        .filter_map(|line| serde_json::from_str::<JournalEntry>(line).ok())
        .find_map(|entry| match entry {
            JournalEntry::Lifecycle(entry) if entry.spirit_id == "downgraded-spirit" => Some(entry),
            _ => None,
        })
        .expect("persisted typed downgrade entry");
    assert_eq!(
        persisted_entry.lifecycle_event,
        LifecycleEvent::ResourceCeilingDowngraded
    );
    let replayed_payload: ResourceCeilingDowngradedPayload = serde_json::from_slice(
        persisted_entry
            .payload
            .as_deref()
            .expect("typed downgrade payload bytes"),
    )
    .expect("deserialize typed payload from persisted lifecycle entry");
    assert_eq!(
        replayed_payload.missing_mechanism,
        payload.missing_mechanism
    );
    assert_eq!(replayed_payload.memory_max_mb, payload.memory_max_mb);
    assert_eq!(replayed_payload.fd_max, payload.fd_max);
    assert_eq!(replayed_payload.rlimit_as, payload.rlimit_as);
    assert_eq!(replayed_payload.rlimit_nofile, payload.rlimit_nofile);
}
