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
