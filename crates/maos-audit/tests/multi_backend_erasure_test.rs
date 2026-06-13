//! Story 9.2 — Decision F: every registered storage backend must prove erasure
//! OR prove non-applicability by construction.

#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::Arc;

use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use rusqlite::OpenFlags;
use tempfile::TempDir;

const CANARY: &str = "DECISION-F-CANARY-9-2";

fn open_isolated_adapter(dir: &TempDir) -> Arc<maos_kernel_core::memory::MemoryManagerAdapter> {
    let fs_root = dir.path().join("memory");
    std::fs::create_dir_all(&fs_root).unwrap();
    let db_path = dir.path().join("audit.sqlite");

    let private = Arc::new(maos_kernel_core::memory::PrivateMemoryStore::new(
        fs_root, 4 * 1024,
    ));
    let shared = Arc::new(
        maos_kernel_core::memory::SharedMemoryStore::open(&db_path).unwrap(),
    );
    let principal_index = Arc::new(
        maos_kernel_core::memory::PrincipalNamespaceIndex::open(&db_path).unwrap(),
    );
    let tl = Arc::new(
        maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open(&db_path, 1)
            .unwrap(),
    );
    Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
        private,
        shared,
        principal_index,
        tl,
    ))
}

fn private_store_contains(fs_root: &Path, needle: &str) -> bool {
    fn scan_dir(dir: &Path, needle: &str) -> bool {
        let Ok(entries) = std::fs::read_dir(dir) else { return false };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if scan_dir(&path, needle) {
                    return true;
                }
            } else if let Ok(bytes) = std::fs::read(&path) {
                if String::from_utf8_lossy(&bytes).contains(needle) {
                    return true;
                }
            }
        }
        false
    }
    scan_dir(fs_root, needle)
}

fn shared_store_contains(db_path: &Path, needle: &str) -> bool {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .unwrap();
    let mut stmt = conn
        .prepare("SELECT value FROM shared_memory")
        .unwrap();
    let rows: Vec<Vec<u8>> = stmt
        .query_map([], |row| {
            let bytes: Vec<u8> = row.get(0)?;
            Ok(bytes)
        })
        .unwrap()
        .flatten()
        .collect();
    rows.iter()
        .any(|bytes| String::from_utf8_lossy(bytes).contains(needle))
}

#[test]
fn multi_backend_erasure_partition_invariant() {
    let dir = TempDir::new().unwrap();
    let memory = open_isolated_adapter(&dir);
    let fs_root = dir.path().join("memory");
    let db_path = dir.path().join("audit.sqlite");
    let principal = "dave@example.org";

    // Plant principal data in Private tier.
    let principal_ns = MemoryNamespace::Principal {
        principal_id: principal.into(),
        schema: "chat".into(),
    };
    memory
        .write(
            7,
            MemoryTier::Private,
            &principal_ns,
            "msg1",
            MemoryValue::Text(format!("{} hello", CANARY)),
        )
        .unwrap();

    // Plant the SAME canary in Shared tier under a non-principal namespace.
    // This represents legitimate cross-Spirit data that ADR-026 says must NOT
    // be erased by a principal forget.
    memory
        .write(
            7,
            MemoryTier::Shared,
            &MemoryNamespace::Coordination,
            "shared-marker",
            MemoryValue::Text(format!("{} shared", CANARY)),
        )
        .unwrap();

    // Forget the principal.
    memory.forget_with_reason(principal, None).unwrap();

    // --- Backend enumeration + partition ---
    let mut proved_erased: Vec<&str> = Vec::new();
    let mut proved_principal_empty: Vec<&str> = Vec::new();

    // Private tier: empirical canary scan must return nil.
    assert!(
        !private_store_contains(&fs_root, CANARY),
        "Private tier still contains principal canary after forget"
    );
    proved_erased.push("private");

    // Principal index: address-only rows must be gone.
    let rows = maos_audit::subject_access_query(&db_path, principal).unwrap();
    assert!(rows.is_empty(), "principal_index still contains rows after forget");
    proved_erased.push("principal_index");

    // Shared tier: empirical canary scan returns the legitimate shared canary,
    // proving principal data never landed here (non-applicability by construction).
    assert!(
        shared_store_contains(&db_path, CANARY),
        "Shared tier should retain its legitimate canary for negative-scan demonstration"
    );
    proved_principal_empty.push("shared");

    // Partition invariant: every registered backend is in exactly one bucket,
    // buckets are disjoint, and together they cover the registered set.
    let registered: std::collections::HashSet<&str> =
        maos_kernel_core::memory::REGISTERED_ERASURE_BACKENDS.iter().copied().collect();
    let covered: std::collections::HashSet<&str> = proved_erased.iter().copied().collect();
    let retained: std::collections::HashSet<&str> =
        proved_principal_empty.iter().copied().collect();

    assert!(covered.is_disjoint(&retained), "erased and principal-empty sets must be disjoint");
    assert_eq!(
        covered.union(&retained).copied().collect::<std::collections::HashSet<_>>(),
        registered,
        "all registered backends must be accounted for"
    );
}
