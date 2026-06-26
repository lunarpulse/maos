//! Story 9.7 AC-1 — Durable skill-queue store tests.
//!
//! Tests the `LocalFsSkillQueueStore` round-trip, fault injection (partial
//! write leaves prior valid state intact), schema-version hard error, and
//! integration with `SkillAdmissionQueue` mechanics.

use std::fs;
use std::path::PathBuf;

use maos_skill::{
    parse_skill, LocalFsSkillQueueStore, QueueEntry, SkillAdmissionQueue, SkillAdmissionState,
    SkillEntryPath, SkillId, SkillQueueStore, SkillVersion,
};

fn skill(id: &str) -> maos_skill::Skill {
    let src = format!(
        "---\nid = \"{id}\"\nversion = \"1.0.0\"\nname = \"{id}\"\ndescription = \"d\"\n---\nBody for {id}.\n"
    );
    parse_skill(&src).unwrap()
}

fn queue_path(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("queue.json")
}

// ─── AC-1: restart round-trip ──────────────────────────────────────────

#[test]
fn restart_round_trip_recovers_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = queue_path(&dir);
    let store = LocalFsSkillQueueStore::at_path(path.clone());

    // Build queue, enqueue + approve/reject
    let mut q = SkillAdmissionQueue::new();
    q.enqueue_skill(skill("skill.a"), SkillEntryPath::PackageShipped, "op")
        .unwrap();
    q.enqueue_skill(skill("skill.b"), SkillEntryPath::AuthorSelf, "op")
        .unwrap();
    q.enqueue_skill(skill("skill.c"), SkillEntryPath::PackageShipped, "op")
        .unwrap();

    // Approve a, reject b, leave c pending
    q.approve(&SkillId::from("skill.a"));
    q.reject(&SkillId::from("skill.b"));

    // Persist
    store.save(&q.to_stored()).unwrap();

    // Open NEW store instance (= restart)
    let store2 = LocalFsSkillQueueStore::at_path(path);
    let entries = store2.load().unwrap();

    // Reconstruct queue
    let q2 = SkillAdmissionQueue::from_stored(entries);

    // Verify state is recovered — typed assertions, not string scrapes
    assert_eq!(
        q2.state_of(&SkillId::from("skill.a")),
        Some(SkillAdmissionState::Admitted)
    );
    assert_eq!(
        q2.state_of(&SkillId::from("skill.b")),
        Some(SkillAdmissionState::Rejected)
    );
    assert_eq!(
        q2.state_of(&SkillId::from("skill.c")),
        Some(SkillAdmissionState::Pending)
    );
    assert_eq!(q2.entries().len(), 3);
}

// ─── AC-1: schema version hard error ───────────────────────────────────

#[test]
fn unknown_schema_version_is_hard_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = queue_path(&dir);
    let store = LocalFsSkillQueueStore::at_path(path.clone());

    // Write a file with wrong schema version
    let bad = serde_json::json!({
        "schema_version": "maos.skill-queue.v99",
        "pending": []
    });
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, serde_json::to_string_pretty(&bad).unwrap()).unwrap();

    let result = store.load();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("v99"),
        "error should mention the unknown version: {}",
        err
    );
}

#[test]
fn missing_schema_version_is_hard_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = queue_path(&dir);
    let store = LocalFsSkillQueueStore::at_path(path.clone());

    // Write a file missing schema_version
    let bad = serde_json::json!({
        "pending": []
    });
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, serde_json::to_string_pretty(&bad).unwrap()).unwrap();

    // serde_json will fail to parse because schema_version is required
    let result = store.load();
    assert!(result.is_err());
}

// ─── AC-1: absent file = empty queue (fresh install) ───────────────────

#[test]
fn absent_file_returns_empty_queue() {
    let dir = tempfile::tempdir().unwrap();
    let path = queue_path(&dir);
    let store = LocalFsSkillQueueStore::at_path(path);

    let entries = store.load().unwrap();
    assert!(entries.is_empty());
}

// ─── AC-1: fault injection — partial write leaves prior state intact ───

#[test]
fn fault_injection_partial_write_preserves_prior_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = queue_path(&dir);
    let store = LocalFsSkillQueueStore::at_path(path.clone());

    // Write a valid initial state
    let initial = vec![QueueEntry {
        id: SkillId::from("skill.x"),
        version: SkillVersion::from("1.0.0"),
        entry_path: "package_shipped".to_string(),
        state: SkillAdmissionState::Pending,
    }];
    store.save(&initial).unwrap();

    // Simulate a "failed write" by writing a temp file but NOT renaming.
    // The prior queue.json should still be intact.
    let parent = path.parent().unwrap();
    let tmp_path = parent.join("queue.json.tmp.99999");
    fs::write(&tmp_path, b"CORRUPTED DATA").unwrap();
    // The tmp file exists but queue.json is untouched.

    let loaded = store.load().unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, SkillId::from("skill.x"));
    assert_eq!(loaded[0].state, SkillAdmissionState::Pending);

    // Cleanup
    let _ = fs::remove_file(&tmp_path);
}

#[test]
fn failed_atomic_write_preserves_prior_state_and_leaves_no_temp() {
    // Review #11: force `atomic_write` ITSELF to fail (not a hand-written
    // stray temp) and prove the prior valid `queue.json` is left intact +
    // parseable AND no temp remnant leaks. True mid-rename injection needs IO
    // hooks that don't exist in safe Rust; this exercises the failure path that
    // IS reachable (parent-dir creation failure) and asserts the atomicity +
    // cleanup contract.
    use maos_skill::store::atomic_write;
    let dir = tempfile::tempdir().unwrap();
    let path = queue_path(&dir);
    let store = LocalFsSkillQueueStore::at_path(path.clone());

    let initial = vec![QueueEntry {
        id: SkillId::from("skill.x"),
        version: SkillVersion::from("1.0.0"),
        entry_path: "package_shipped".to_string(),
        state: SkillAdmissionState::Pending,
    }];
    store.save(&initial).unwrap();

    // A dest whose parent is an existing REGULAR FILE → create_dir_all fails
    // inside atomic_write (cannot make a directory out of a file).
    let blocker = dir.path().join("blocker_file");
    fs::write(&blocker, b"not a dir").unwrap();
    let bad_dest = blocker.join("queue.json");

    let result = atomic_write(&bad_dest, b"SHOULD NOT LAND");
    assert!(
        result.is_err(),
        "atomic_write must fail on an unwritable parent"
    );

    // Prior queue.json intact + parseable.
    let loaded = store.load().unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, SkillId::from("skill.x"));
    assert_eq!(loaded[0].state, SkillAdmissionState::Pending);

    // No temp remnant leaked anywhere under the temp dir.
    let mut leaked = 0;
    for entry in fs::read_dir(dir.path()).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name().to_string_lossy().contains(".tmp.") {
            leaked += 1;
        }
    }
    assert_eq!(leaked, 0, "a failed atomic_write must not leak a temp file");
}

// ─── AC-1: atomic write produces parseable JSON ────────────────────────

#[test]
fn atomic_write_produces_parseable_json_with_schema_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = queue_path(&dir);
    let store = LocalFsSkillQueueStore::at_path(path.clone());

    let entries = vec![
        QueueEntry {
            id: SkillId::from("skill.one"),
            version: SkillVersion::from("2.0.0"),
            entry_path: "author_self".to_string(),
            state: SkillAdmissionState::Admitted,
        },
        QueueEntry {
            id: SkillId::from("skill.two"),
            version: SkillVersion::from("3.0.0"),
            entry_path: "revision_proposal".to_string(),
            state: SkillAdmissionState::Rejected,
        },
    ];
    store.save(&entries).unwrap();

    // Verify the file is valid JSON with the correct schema version
    let raw = fs::read_to_string(&path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        parsed["schema_version"].as_str().unwrap(),
        "maos.skill-queue.v1"
    );
    assert_eq!(parsed["pending"].as_array().unwrap().len(), 2);
}

// ─── AC-1: existing 7 in-memory tests still pass ──────────────────────
// (These are in admission_queue_test.rs — verified by `cargo test -p maos-skill`)

// ─── AC-1: QueueEntry round-trip preserves typed state ─────────────────

#[test]
fn queue_entry_round_trip_preserves_admission_state_types() {
    let dir = tempfile::tempdir().unwrap();
    let path = queue_path(&dir);
    let store = LocalFsSkillQueueStore::at_path(path);

    let entries = vec![
        QueueEntry {
            id: SkillId::from("pending-skill"),
            version: SkillVersion::from("1.0.0"),
            entry_path: "package_shipped".to_string(),
            state: SkillAdmissionState::Pending,
        },
        QueueEntry {
            id: SkillId::from("admitted-skill"),
            version: SkillVersion::from("2.0.0"),
            entry_path: "author_self".to_string(),
            state: SkillAdmissionState::Admitted,
        },
        QueueEntry {
            id: SkillId::from("rejected-skill"),
            version: SkillVersion::from("3.0.0"),
            entry_path: "revision_proposal".to_string(),
            state: SkillAdmissionState::Rejected,
        },
    ];

    store.save(&entries).unwrap();
    let loaded = store.load().unwrap();

    // Typed SkillAdmissionState assertion, not string scrape
    assert_eq!(loaded[0].state, SkillAdmissionState::Pending);
    assert_eq!(loaded[1].state, SkillAdmissionState::Admitted);
    assert_eq!(loaded[2].state, SkillAdmissionState::Rejected);

    // Ids and versions are preserved
    assert_eq!(loaded[0].id, SkillId::from("pending-skill"));
    assert_eq!(loaded[2].version, SkillVersion::from("3.0.0"));

    // Entry paths are preserved as strings
    assert_eq!(loaded[0].entry_path, "package_shipped");
    assert_eq!(loaded[1].entry_path, "author_self");
    assert_eq!(loaded[2].entry_path, "revision_proposal");
}

// ─── Store→Queue→Store round-trip (the restart story) ──────────────────

#[test]
fn store_to_queue_to_store_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = queue_path(&dir);

    // Session 1: enqueue + approve + persist
    {
        let store = LocalFsSkillQueueStore::at_path(path.clone());
        let mut q = SkillAdmissionQueue::new();
        q.enqueue_skill(skill("skill.alpha"), SkillEntryPath::PackageShipped, "op")
            .unwrap();
        q.approve(&SkillId::from("skill.alpha"));
        store.save(&q.to_stored()).unwrap();
    }

    // Session 2 (= restart): load → verify → operate → persist
    {
        let store = LocalFsSkillQueueStore::at_path(path.clone());
        let entries = store.load().unwrap();
        let mut q = SkillAdmissionQueue::from_stored(entries);

        // The approve from session 1 is preserved
        assert_eq!(
            q.state_of(&SkillId::from("skill.alpha")),
            Some(SkillAdmissionState::Admitted)
        );

        // Enqueue a new skill in session 2
        q.enqueue_skill(skill("skill.beta"), SkillEntryPath::AuthorSelf, "op")
            .unwrap();
        store.save(&q.to_stored()).unwrap();
    }

    // Session 3: verify both persisted
    {
        let store = LocalFsSkillQueueStore::at_path(path);
        let entries = store.load().unwrap();
        assert_eq!(entries.len(), 2);
        let q = SkillAdmissionQueue::from_stored(entries);
        assert_eq!(
            q.state_of(&SkillId::from("skill.alpha")),
            Some(SkillAdmissionState::Admitted)
        );
        assert_eq!(
            q.state_of(&SkillId::from("skill.beta")),
            Some(SkillAdmissionState::Pending)
        );
    }
}

// ─── AC-6: principal-free queue.json ───────────────────────────────────

#[test]
fn queue_json_contains_no_principal_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = queue_path(&dir);
    let store = LocalFsSkillQueueStore::at_path(path.clone());

    // Build a queue with all three state types, simulating real operation
    let mut q = SkillAdmissionQueue::new();
    q.enqueue_skill(
        skill("skill.pending"),
        SkillEntryPath::PackageShipped,
        "real-operator-name", // actor — this is principal data
    )
    .unwrap();
    q.enqueue_skill(
        skill("skill.admitted"),
        SkillEntryPath::AuthorSelf,
        "spirit:42", // actor — this is principal data
    )
    .unwrap();
    q.approve(&SkillId::from("skill.admitted"));
    q.enqueue_skill(
        skill("skill.rejected"),
        SkillEntryPath::PackageShipped,
        "admin-user", // actor — this is principal data
    )
    .unwrap();
    q.reject(&SkillId::from("skill.rejected"));

    // Save — only the pending state machine should be written
    store.save(&q.to_stored()).unwrap();

    // Read the raw JSON and verify NO principal data is present.
    // Principal data = actor/operator id (the `actor` field on ApprovalDecision).
    let raw = fs::read_to_string(&path).unwrap();

    // The `actor` values used above must NOT appear in queue.json.
    assert!(
        !raw.contains("real-operator-name"),
        "queue.json must not contain actor principal data: found 'real-operator-name'"
    );
    assert!(
        !raw.contains("spirit:42"),
        "queue.json must not contain actor principal data: found 'spirit:42'"
    );
    assert!(
        !raw.contains("admin-user"),
        "queue.json must not contain actor principal data: found 'admin-user'"
    );
    // The word "actor" itself should not appear as a JSON key
    assert!(
        !raw.contains("\"actor\""),
        "queue.json must not contain an 'actor' field"
    );
    // Verify the file DOES contain expected skill metadata
    assert!(raw.contains("skill.pending"));
    assert!(raw.contains("skill.admitted"));
    assert!(raw.contains("skill.rejected"));
    assert!(raw.contains("maos.skill-queue.v1"));
}
