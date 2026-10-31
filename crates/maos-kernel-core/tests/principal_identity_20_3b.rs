#![cfg(unix)]
#![forbid(unsafe_code)]
//! FR31 / story 20-3b: fresh adapters model separate boots over the same disk.
//! Replacing `owner_dirname` with the pid must break the disclosure witness.

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use maos_domain::memory::{ExportPayload, MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};

type TestResult = Result<(), Box<dyn Error>>;
const PRINCIPAL: &str = "director-ada";

fn boot(root: &Path) -> Result<MemoryManagerAdapter, Box<dyn Error>> {
    let db = root.join("memory.sqlite");
    Ok(MemoryManagerAdapter::new(
        Arc::new(PrivateMemoryStore::new(root.join("private"), 4096)),
        Arc::new(SharedMemoryStore::open(&db)?),
        Arc::new(PrincipalNamespaceIndex::open(&db)?),
        Arc::new(TransparencyLogAdapter::open_in_memory(0x203b)),
    ))
}

#[test]
fn recycled_pid_cannot_disclose_another_spirits_principal_spill() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ns = MemoryNamespace::principal(PRINCIPAL, "private/v1")?;
    {
        let memory = boot(dir.path())?;
        memory.bind_storage_identity(1, "spirit-a")?;
        memory.write(
            1,
            MemoryTier::Private,
            &ns,
            "secret",
            MemoryValue::Markdown("A only".into()),
        )?;
    }
    let memory = boot(dir.path())?;
    memory.bind_storage_identity(1, "spirit-b")?;
    assert_eq!(memory.read(1, MemoryTier::Private, &ns, "secret")?, None);
    assert!(memory.scan(1, MemoryTier::Private, &ns, "", 10)?.is_empty());
    assert!(dir
        .path()
        .join(format!("private/id-{}", hex::encode("spirit-a")))
        .is_dir());
    assert!(!dir.path().join("private/1").exists());
    Ok(())
}

#[test]
fn small_non_markdown_principal_values_survive_restart_and_pid_change() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ns = MemoryNamespace::principal(PRINCIPAL, "durability/v1")?;
    let values = [
        MemoryValue::Text("small".into()),
        MemoryValue::Json(serde_json::json!({"accepted": true})),
        MemoryValue::Blob(vec![0, 1, 255]),
    ];
    {
        let memory = boot(dir.path())?;
        memory.bind_storage_identity(1, "stable-spirit")?;
        for (i, value) in values.iter().enumerate() {
            memory.write(1, MemoryTier::Private, &ns, &i.to_string(), value.clone())?;
        }
    }
    let memory = boot(dir.path())?;
    memory.bind_storage_identity(77, "stable-spirit")?;
    for (i, value) in values.iter().enumerate() {
        assert_eq!(
            memory
                .read(77, MemoryTier::Private, &ns, &i.to_string())?
                .as_ref(),
            Some(value)
        );
    }
    Ok(())
}

#[test]
fn subject_access_export_preserves_both_owners_when_pid_and_key_are_reused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ns = MemoryNamespace::principal(PRINCIPAL, "export/v1")?;
    {
        let memory = boot(dir.path())?;
        memory.bind_storage_identity(1, "spirit-a")?;
        memory.write(
            1,
            MemoryTier::Private,
            &ns,
            "same-key",
            MemoryValue::Text("A".into()),
        )?;
    }
    let memory = boot(dir.path())?;
    memory.bind_storage_identity(1, "spirit-b")?;
    memory.write(
        1,
        MemoryTier::Private,
        &ns,
        "same-key",
        MemoryValue::Text("B".into()),
    )?;
    let rows = memory.subject_access(PRINCIPAL)?;
    assert_eq!(
        rows.len(),
        2,
        "the index key must include stable owner, not pid"
    );
    for identity in ["spirit-a", "spirit-b"] {
        assert!(rows
            .iter()
            .any(|row| row.writer_owner == format!("id-{}", hex::encode(identity))));
    }
    let entries = memory.export_redactable(PRINCIPAL, true)?;
    assert_eq!(entries.len(), 2);
    for expected in ["A", "B"] {
        assert!(entries.iter().any(|entry| matches!(&entry.payload, ExportPayload::Raw(MemoryValue::Text(text)) if text == expected)));
    }
    Ok(())
}

#[test]
fn forget_counts_identity_and_legacy_dirs_once_and_preserves_other_principals() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ns = MemoryNamespace::principal(PRINCIPAL, "forget/v1")?;
    let other = MemoryNamespace::principal("someone-else", "forget/v1")?;
    {
        let memory = boot(dir.path())?;
        memory.write(
            9,
            MemoryTier::Private,
            &ns,
            "legacy",
            MemoryValue::Text("legacy".into()),
        )?;
        memory.bind_storage_identity(1, "spirit-a")?;
        memory.write(
            1,
            MemoryTier::Private,
            &ns,
            "owned",
            MemoryValue::Text("owned".into()),
        )?;
        memory.bind_storage_identity(2, "spirit-b")?;
        memory.write(
            2,
            MemoryTier::Private,
            &ns,
            "owned",
            MemoryValue::Text("owned".into()),
        )?;
        memory.write(
            2,
            MemoryTier::Private,
            &other,
            "keep",
            MemoryValue::Text("keep".into()),
        )?;
        let receipt = memory.forget(PRINCIPAL)?;
        assert_eq!(
            receipt.deleted_entries, 3,
            "cached and spilled principal values count once"
        );
        assert_eq!(receipt.deleted_index_rows, 3);
        assert!(!dir.path().join("private/9").exists());
        assert!(!dir
            .path()
            .join(format!("private/id-{}", hex::encode("spirit-a")))
            .exists());
    }
    let memory = boot(dir.path())?;
    memory.bind_storage_identity(99, "spirit-b")?;
    assert_eq!(memory.read(99, MemoryTier::Private, &ns, "owned")?, None);
    assert_eq!(
        memory.read(99, MemoryTier::Private, &other, "keep")?,
        Some(MemoryValue::Text("keep".into()))
    );
    assert_eq!(memory.forget(PRINCIPAL)?.deleted_entries, 0);
    Ok(())
}

#[test]
fn legacy_index_migrates_idempotently_and_export_ignores_new_pid_binding() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ns = MemoryNamespace::principal(PRINCIPAL, "legacy/v1")?;
    {
        let memory = boot(dir.path())?;
        memory.write(
            1,
            MemoryTier::Private,
            &ns,
            "legacy",
            MemoryValue::Markdown("old owner".into()),
        )?;
    }
    {
        let conn = rusqlite::Connection::open(dir.path().join("memory.sqlite"))?;
        conn.execute_batch("DROP TABLE principal_index;
            CREATE TABLE principal_index (principal_id TEXT NOT NULL, writer_spirit_pid INTEGER NOT NULL,
            schema TEXT NOT NULL, key TEXT NOT NULL, timestamp_ns INTEGER NOT NULL,
            PRIMARY KEY(principal_id, writer_spirit_pid, schema, key));
            INSERT INTO principal_index VALUES ('director-ada', 1, 'legacy/v1', 'legacy', 42);")?;
    }
    for _ in 0..2 {
        let memory = boot(dir.path())?;
        memory.bind_storage_identity(1, "new-spirit")?;
        assert_eq!(
            memory.read(1, MemoryTier::Private, &ns, "legacy")?,
            None,
            "bound Spirits never inherit legacy pid spills"
        );
        let rows = memory.subject_access(PRINCIPAL)?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].writer_owner, "1");
        let exported = memory.export_redactable(PRINCIPAL, true)?;
        assert_eq!(exported.len(), 1);
        assert!(
            matches!(&exported[0].payload, ExportPayload::Raw(MemoryValue::Markdown(text)) if text == "old owner")
        );
    }
    Ok(())
}

#[test]
fn storage_identity_binding_cannot_be_reassigned() -> TestResult {
    let dir = tempfile::tempdir()?;
    let memory = boot(dir.path())?;
    memory.bind_storage_identity(1, "spirit-a")?;
    memory.bind_storage_identity(1, "spirit-a")?;
    assert!(memory.bind_storage_identity(1, "spirit-b").is_err());
    assert!(memory.bind_storage_identity(2, "").is_err());
    Ok(())
}

#[test]
fn write_and_lookup() -> TestResult {
    let dir = tempfile::tempdir()?;
    let memory = boot(dir.path())?;
    for (pid, schema, key) in [
        (10, "calendar", "event-001"),
        (10, "calendar", "event-002"),
        (20, "tasks", "task-001"),
    ] {
        memory.write(
            pid,
            MemoryTier::Private,
            &MemoryNamespace::principal("alice@example.org", schema)?,
            key,
            MemoryValue::Text(key.into()),
        )?;
    }
    let rows = memory.subject_access("alice@example.org")?;
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].writer_spirit_pid, 10);
    assert_eq!(rows[1].writer_spirit_pid, 10);
    assert_eq!(rows[2].writer_spirit_pid, 20);
    Ok(())
}

#[test]
fn lookup_nonexistent_returns_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    assert!(boot(dir.path())?
        .subject_access("bob@example.org")?
        .is_empty());
    Ok(())
}

#[test]
fn forget_then_lookup_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let memory = boot(dir.path())?;
    for (pid, schema, key) in [
        (10, "calendar", "e1"),
        (10, "calendar", "e2"),
        (20, "tasks", "t1"),
    ] {
        memory.write(
            pid,
            MemoryTier::Private,
            &MemoryNamespace::principal("alice@example.org", schema)?,
            key,
            MemoryValue::Text(key.into()),
        )?;
    }
    assert_eq!(memory.forget("alice@example.org")?.deleted_index_rows, 3);
    assert!(memory.subject_access("alice@example.org")?.is_empty());
    Ok(())
}

#[test]
fn second_forget_returns_zero() -> TestResult {
    let dir = tempfile::tempdir()?;
    let memory = boot(dir.path())?;
    memory.write(
        10,
        MemoryTier::Private,
        &MemoryNamespace::principal("alice@example.org", "calendar")?,
        "e1",
        MemoryValue::Text("value".into()),
    )?;
    assert_eq!(memory.forget("alice@example.org")?.deleted_index_rows, 1);
    assert_eq!(memory.forget("alice@example.org")?.deleted_index_rows, 0);
    Ok(())
}

#[test]
fn long_identity_namespace_and_key_round_trip_and_forget() -> TestResult {
    let dir = tempfile::tempdir()?;
    let identity = "a".repeat(128);
    let principal = "p".repeat(1024);
    let namespace = MemoryNamespace::principal(&principal, "s".repeat(1024))?;
    let key = "k".repeat(1024);
    {
        let memory = boot(dir.path())?;
        memory.bind_storage_identity(1, &identity)?;
        memory.write(
            1,
            MemoryTier::Private,
            &namespace,
            &key,
            MemoryValue::Text("long".into()),
        )?;
    }
    let owner = std::fs::read_dir(dir.path().join("private"))?
        .next()
        .unwrap()?
        .path();
    assert!(owner.file_name().unwrap().len() <= 200);
    assert_eq!(
        std::fs::read_to_string(owner.join(".identity"))?,
        format!("id-{}", hex::encode(&identity))
    );
    let memory = boot(dir.path())?;
    memory.bind_storage_identity(2, &identity)?;
    assert_eq!(
        memory.read(2, MemoryTier::Private, &namespace, &key)?,
        Some(MemoryValue::Text("long".into()))
    );
    let rows = memory.scan(2, MemoryTier::Private, &namespace, "", 10)?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key, key);
    assert!(
        matches!(&memory.export_redactable(&principal, true)?[0].payload, ExportPayload::Raw(MemoryValue::Text(value)) if value == "long")
    );
    assert_eq!(
        memory.principal_ids_for_spirit(&identity)?,
        vec![principal.clone()]
    );
    assert_eq!(memory.forget(&principal)?.deleted_entries, 1);
    assert!(!owner.exists());
    Ok(())
}

#[test]
fn hashed_owner_namespace_and_key_mismatches_refuse_reads_and_writes() -> TestResult {
    for target in ["owner", "namespace", "key"] {
        let dir = tempfile::tempdir()?;
        let identity = "a".repeat(128);
        let namespace = MemoryNamespace::principal(PRINCIPAL, "s".repeat(200))?;
        let key = "k".repeat(1024);
        {
            let memory = boot(dir.path())?;
            memory.bind_storage_identity(1, &identity)?;
            memory.write(
                1,
                MemoryTier::Private,
                &namespace,
                &key,
                MemoryValue::Text("secret".into()),
            )?;
        }
        let owner = std::fs::read_dir(dir.path().join("private"))?
            .next()
            .unwrap()?
            .path();
        let ns = std::fs::read_dir(&owner)?
            .filter_map(Result::ok)
            .find(|e| e.file_type().unwrap().is_dir())
            .unwrap()
            .path();
        let metadata = match target {
            "owner" => owner.join(".identity"),
            "namespace" => ns.join(".identity"),
            _ => std::fs::read_dir(&ns)?
                .filter_map(Result::ok)
                .find(|e| e.file_name().to_string_lossy().starts_with(".key-"))
                .unwrap()
                .path(),
        };
        std::fs::write(metadata, "mismatched identity")?;
        let memory = boot(dir.path())?;
        memory.bind_storage_identity(2, &identity)?;
        assert!(
            memory
                .read(2, MemoryTier::Private, &namespace, &key)
                .is_err(),
            "{target}"
        );
        assert!(
            memory.export_redactable(PRINCIPAL, true).is_err(),
            "{target}"
        );
        assert!(
            memory
                .write(
                    2,
                    MemoryTier::Private,
                    &namespace,
                    &key,
                    MemoryValue::Text("other".into())
                )
                .is_err(),
            "{target}"
        );
    }
    Ok(())
}

#[test]
fn case_distinct_keys_have_case_fold_distinct_filenames() -> TestResult {
    let dir = tempfile::tempdir()?;
    let namespace = MemoryNamespace::principal(PRINCIPAL, "case")?;
    {
        let memory = boot(dir.path())?;
        memory.bind_storage_identity(1, "case-spirit")?;
        for key in ["Foo", "foo"] {
            memory.write(
                1,
                MemoryTier::Private,
                &namespace,
                key,
                MemoryValue::Text(key.into()),
            )?;
        }
    }
    let ns = dir.path().join(format!(
        "private/id-{}/{}",
        hex::encode("case-spirit"),
        hex::encode(serde_json::to_vec(&namespace)?)
    ));
    let names: std::collections::BTreeSet<_> = std::fs::read_dir(&ns)?
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_lowercase())
        .collect();
    assert_eq!(
        names,
        ["466f6f.txt".into(), "666f6f.txt".into()]
            .into_iter()
            .collect()
    );
    let memory = boot(dir.path())?;
    memory.bind_storage_identity(2, "case-spirit")?;
    for key in ["Foo", "foo"] {
        assert_eq!(
            memory.read(2, MemoryTier::Private, &namespace, key)?,
            Some(MemoryValue::Text(key.into()))
        );
    }
    assert_eq!(
        memory
            .scan(2, MemoryTier::Private, &namespace, "", 10)?
            .len(),
        2
    );
    Ok(())
}

#[test]
fn changed_pid_discovers_recorded_owner_without_rewriting_then_forgets() -> TestResult {
    let dir = tempfile::tempdir()?;
    let namespace = MemoryNamespace::principal(PRINCIPAL, "owner")?;
    {
        let memory = boot(dir.path())?;
        memory.bind_storage_identity(1, "stable")?;
        memory.write(
            1,
            MemoryTier::Private,
            &namespace,
            "key",
            MemoryValue::Text("value".into()),
        )?;
    }
    let memory = boot(dir.path())?;
    memory.bind_storage_identity(2, "stable")?;
    let ids = memory.principal_ids_for_spirit("stable")?;
    assert_eq!(ids, vec![PRINCIPAL]);
    assert_eq!(memory.subject_access(PRINCIPAL)?[0].writer_spirit_pid, 1);
    assert_eq!(memory.forget(&ids[0])?.deleted_entries, 1);
    assert!(memory.principal_ids_for_spirit("stable")?.is_empty());
    assert_eq!(
        memory.read(2, MemoryTier::Private, &namespace, "key")?,
        None
    );
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn spill_failure_removes_new_intent_but_preserves_existing_value_index() -> TestResult {
    use maos_kernel_core::memory::spill_test_faults::{arm_failure, disarm, FailurePoint};
    let dir = tempfile::tempdir()?;
    let memory = boot(dir.path())?;
    let namespace = MemoryNamespace::principal(PRINCIPAL, "intent")?;
    arm_failure(FailurePoint::TempFileSync);
    let failed = memory.write(
        1,
        MemoryTier::Private,
        &namespace,
        "new",
        MemoryValue::Text("new".into()),
    );
    disarm();
    assert!(failed.is_err());
    assert!(memory.subject_access(PRINCIPAL)?.is_empty());
    assert_eq!(
        memory.read(1, MemoryTier::Private, &namespace, "new")?,
        None
    );
    memory.write(
        1,
        MemoryTier::Private,
        &namespace,
        "old",
        MemoryValue::Text("old".into()),
    )?;
    arm_failure(FailurePoint::TempFileSync);
    let failed = memory.write(
        1,
        MemoryTier::Private,
        &namespace,
        "old",
        MemoryValue::Text("replacement".into()),
    );
    disarm();
    assert!(failed.is_err());
    assert_eq!(memory.subject_access(PRINCIPAL)?.len(), 1);
    assert!(
        matches!(&memory.export_redactable(PRINCIPAL, true)?[0].payload, ExportPayload::Raw(MemoryValue::Text(value)) if value == "old")
    );
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn committed_index_intent_precedes_spill_and_missing_value_exports_redacted() -> TestResult {
    use maos_kernel_core::memory::spill_test_faults::{arm_pause, disarm, PausePoint};
    use std::sync::mpsc;
    use std::time::Duration;
    let dir = tempfile::tempdir()?;
    let memory = Arc::new(boot(dir.path())?);
    let namespace = MemoryNamespace::principal(PRINCIPAL, "intent")?;
    let (arrived_tx, arrived_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        arm_pause(PausePoint::BeforeRename, arrived_tx, release_rx);
        let result = memory.write(
            1,
            MemoryTier::Private,
            &namespace,
            "key",
            MemoryValue::Text("value".into()),
        );
        disarm();
        result
    });
    arrived_rx.recv_timeout(Duration::from_secs(10))?;
    let conn = rusqlite::Connection::open(dir.path().join("memory.sqlite"))?;
    let count: u64 =
        conn.query_row("SELECT COUNT(*) FROM principal_index", [], |row| row.get(0))?;
    release_tx.send(())?;
    worker.join().unwrap()?;
    assert_eq!(count, 1, "intent must already be independently committed");
    conn.execute(
        "INSERT INTO principal_index (principal_id, writer_spirit_pid, schema, key, timestamp_ns, writer_owner) VALUES ('missing', 1, 'intent', 'absent', 1, '1')",
        [],
    )?;
    let exported = boot(dir.path())?.export_redactable("missing", true)?;
    assert_eq!(exported.len(), 1);
    assert!(matches!(
        &exported[0].payload,
        ExportPayload::Redacted { .. }
    ));
    Ok(())
}

#[test]
fn concurrent_legacy_index_open_serializes_owner_migration() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("legacy.sqlite");
    {
        let conn = rusqlite::Connection::open(&path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;
            CREATE TABLE principal_index (principal_id TEXT NOT NULL, writer_spirit_pid INTEGER NOT NULL,
            schema TEXT NOT NULL, key TEXT NOT NULL, timestamp_ns INTEGER NOT NULL,
            PRIMARY KEY(principal_id, writer_spirit_pid, schema, key));
            INSERT INTO principal_index VALUES ('legacy', 1, 'schema', 'key', 42);")?;
    }
    let barrier = Arc::new(std::sync::Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                PrincipalNamespaceIndex::open(&path)
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap()?;
    }
    let conn = rusqlite::Connection::open(&path)?;
    assert_eq!(
        conn.query_row("SELECT writer_owner FROM principal_index", [], |row| row
            .get::<_, String>(
            0
        ))?,
        "1"
    );
    Ok(())
}

#[test]
fn index_failure_happens_before_any_durable_value_is_created() -> TestResult {
    let dir = tempfile::tempdir()?;
    let memory = boot(dir.path())?;
    let conn = rusqlite::Connection::open(dir.path().join("memory.sqlite"))?;
    conn.execute_batch(
        "CREATE TRIGGER reject_intent BEFORE INSERT ON principal_index
        BEGIN SELECT RAISE(FAIL, 'injected intent failure'); END;",
    )?;
    let namespace = MemoryNamespace::principal(PRINCIPAL, "index-failure")?;
    assert!(memory
        .write(
            1,
            MemoryTier::Private,
            &namespace,
            "new",
            MemoryValue::Text("new".into())
        )
        .is_err());
    assert!(!dir.path().join("private").exists());
    assert!(memory.subject_access(PRINCIPAL)?.is_empty());
    Ok(())
}

#[test]
fn absent_long_key_is_absent_not_an_identity_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let memory = boot(dir.path())?;
    let namespace = MemoryNamespace::principal(PRINCIPAL, "absent")?;
    memory.write(
        1,
        MemoryTier::Private,
        &namespace,
        "existing",
        MemoryValue::Text("value".into()),
    )?;
    assert_eq!(
        memory.read(1, MemoryTier::Private, &namespace, &"k".repeat(1024))?,
        None
    );
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn failed_inserting_writer_preserves_a_second_index_instances_completed_spill() -> TestResult {
    use maos_kernel_core::memory::spill_test_faults::{arm_pause, disarm, PausePoint};
    use std::sync::mpsc;
    let dir = tempfile::tempdir()?;
    let first = boot(dir.path())?;
    let second = boot(dir.path())?;
    first.bind_storage_identity(1, "shared-owner")?;
    second.bind_storage_identity(1, "shared-owner")?;
    let ns = MemoryNamespace::principal(PRINCIPAL, "two-indexes")?;
    let worker_ns = ns.clone();
    let (arrived_tx, arrived_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        arm_pause(PausePoint::BeforeRename, arrived_tx, release_rx);
        let result = first.write(
            1,
            MemoryTier::Private,
            &worker_ns,
            "key",
            MemoryValue::Text("first".into()),
        );
        disarm();
        result
    });
    arrived_rx.recv_timeout(std::time::Duration::from_secs(10))?;
    second.write(
        1,
        MemoryTier::Private,
        &ns,
        "key",
        MemoryValue::Text("second".into()),
    )?;
    let namespace = dir
        .path()
        .join(format!("private/id-{}", hex::encode("shared-owner")))
        .join(hex::encode(serde_json::to_vec(&ns)?));
    let temp = std::fs::read_dir(namespace)?
        .filter_map(Result::ok)
        .find(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
        .unwrap();
    std::fs::remove_file(temp.path())?;
    release_tx.send(())?;
    assert!(worker.join().unwrap().is_err());
    let restarted = boot(dir.path())?;
    assert_eq!(restarted.subject_access(PRINCIPAL)?.len(), 1);
    assert_eq!(
        restarted.principal_ids_for_spirit("shared-owner")?,
        vec![PRINCIPAL]
    );
    assert!(
        matches!(&restarted.export_redactable(PRINCIPAL, true)?[0].payload, ExportPayload::Raw(MemoryValue::Text(value)) if value == "second")
    );
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn interrupted_identity_publication_never_leaves_a_partial_final_sidecar() -> TestResult {
    use maos_kernel_core::memory::spill_test_faults::{arm_failure, disarm, FailurePoint};
    for target in ["owner", "namespace", "key"] {
        for fault in [
            FailurePoint::IdentityTempSync,
            FailurePoint::IdentityPublish,
        ] {
            let dir = tempfile::tempdir()?;
            let identity = if target == "owner" {
                "o".repeat(128)
            } else {
                "owner".into()
            };
            let ns = MemoryNamespace::principal(
                PRINCIPAL,
                if target == "namespace" {
                    "s".repeat(200)
                } else {
                    "profile".into()
                },
            )?;
            let key = if target == "key" {
                "k".repeat(200)
            } else {
                "key".into()
            };
            let memory = boot(dir.path())?;
            memory.bind_storage_identity(1, &identity)?;
            arm_failure(fault);
            let result = memory.write(
                1,
                MemoryTier::Private,
                &ns,
                &key,
                MemoryValue::Text("value".into()),
            );
            disarm();
            assert!(result.is_err(), "{target} {fault:?}");
            let metadata = sidecar_path(dir.path(), &identity, &ns, &key, target)?;
            assert!(!metadata.exists(), "{target} {fault:?}");
            drop(memory);
            let restarted = boot(dir.path())?;
            restarted.bind_storage_identity(2, &identity)?;
            restarted.write(
                2,
                MemoryTier::Private,
                &ns,
                &key,
                MemoryValue::Text("value".into()),
            )?;
            assert!(!std::fs::read(metadata)?.is_empty());
            assert_eq!(
                restarted.read(2, MemoryTier::Private, &ns, &key)?,
                Some(MemoryValue::Text("value".into()))
            );
        }
    }
    Ok(())
}

fn sidecar_path(
    root: &Path,
    identity: &str,
    ns: &MemoryNamespace,
    key: &str,
    target: &str,
) -> Result<std::path::PathBuf, Box<dyn Error>> {
    use sha2::{Digest, Sha256};
    let component = |name: String| {
        if name.len() <= 200 {
            name
        } else {
            format!("h-{}", hex::encode(Sha256::digest(name)))
        }
    };
    let owner = root
        .join("private")
        .join(component(format!("id-{}", hex::encode(identity))));
    let namespace = owner.join(component(hex::encode(serde_json::to_vec(ns)?)));
    Ok(match target {
        "owner" => owner.join(".identity"),
        "namespace" => namespace.join(".identity"),
        _ => namespace.join(format!(".key-{}", component(hex::encode(key)))),
    })
}

#[test]
fn empty_legacy_sidecars_recover_only_without_ambiguous_values() -> TestResult {
    for target in ["owner", "namespace", "key"] {
        for occupied in [false, true] {
            let dir = tempfile::tempdir()?;
            let identity = "o".repeat(128);
            let ns = MemoryNamespace::principal(PRINCIPAL, "s".repeat(200))?;
            let key = "k".repeat(200);
            let metadata = sidecar_path(dir.path(), &identity, &ns, &key, target)?;
            std::fs::create_dir_all(metadata.parent().unwrap())?;
            // Complete ancestor identities first; only the target is incomplete.
            for ancestor in ["owner", "namespace"] {
                let path = sidecar_path(dir.path(), &identity, &ns, &key, ancestor)?;
                if path != metadata && metadata.starts_with(path.parent().unwrap()) {
                    let text = if ancestor == "owner" {
                        format!("id-{}", hex::encode(&identity))
                    } else {
                        hex::encode(serde_json::to_vec(&ns)?)
                    };
                    std::fs::write(path, text)?;
                }
            }
            std::fs::write(&metadata, "")?;
            let residue = if target == "key" {
                metadata.with_file_name(format!(
                    "{}.txt",
                    metadata
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .strip_prefix(".key-")
                        .unwrap()
                ))
            } else {
                metadata.with_file_name("other.txt")
            };
            if occupied {
                std::fs::write(&residue, "unattributed value")?;
            }
            let memory = boot(dir.path())?;
            memory.bind_storage_identity(1, &identity)?;
            let result = memory.write(
                1,
                MemoryTier::Private,
                &ns,
                &key,
                MemoryValue::Text("value".into()),
            );
            assert_eq!(result.is_err(), occupied, "{target}");
            if occupied {
                assert_eq!(std::fs::read_to_string(metadata)?, "");
                assert_eq!(std::fs::read_to_string(residue)?, "unattributed value");
            } else {
                assert!(!std::fs::read(metadata)?.is_empty());
            }
        }
    }
    Ok(())
}
