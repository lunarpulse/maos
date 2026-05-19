//! Integration test: Principal Namespace lifecycle (write → subject_access → forget).
//! AC2 — Story 4.3.

use std::sync::Arc;

use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use tempfile::TempDir;

fn make_adapter() -> (Arc<MemoryManagerAdapter>, TempDir) {
    let tmp = TempDir::new().unwrap();
    let memory_root = tmp.path().join("memory");
    let db_path = tmp.path().join("audit.db");

    let private = Arc::new(PrivateMemoryStore::new(memory_root, 4 * 1024));
    let shared = Arc::new(SharedMemoryStore::open(&db_path).unwrap());
    let principal = Arc::new(PrincipalNamespaceIndex::open(&db_path).unwrap());
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xBEE3));
    let adapter = Arc::new(MemoryManagerAdapter::new(private, shared, principal, tl));
    (adapter, tmp)
}

#[test]
fn principal_namespace_full_lifecycle() {
    let (adapter, _tmp) = make_adapter();
    let ns_cal = MemoryNamespace::principal("alice@example.org", "calendar").unwrap();
    let ns_task = MemoryNamespace::principal("alice@example.org", "tasks").unwrap();

    // Spirit 10 writes 3 calendar entries.
    adapter
        .write(10, MemoryTier::Private, &ns_cal, "e1", MemoryValue::Text("cal-1".into()))
        .unwrap();
    adapter
        .write(10, MemoryTier::Private, &ns_cal, "e2", MemoryValue::Text("cal-2".into()))
        .unwrap();
    adapter
        .write(10, MemoryTier::Private, &ns_cal, "e3", MemoryValue::Text("cal-3".into()))
        .unwrap();
    // Spirit 20 writes 2 task entries.
    adapter
        .write(20, MemoryTier::Private, &ns_task, "t1", MemoryValue::Text("task-1".into()))
        .unwrap();
    adapter
        .write(20, MemoryTier::Private, &ns_task, "t2", MemoryValue::Text("task-2".into()))
        .unwrap();

    // subject_access should return 5 rows.
    let rows = adapter.subject_access("alice@example.org").unwrap();
    assert_eq!(rows.len(), 5, "expected 5 rows across 2 spirits");

    // forget cascade.
    let receipt = adapter.forget("alice@example.org").unwrap();
    assert_eq!(receipt.deleted_entries, 5);
    assert_eq!(receipt.deleted_index_rows, 5);

    // Re-query should be empty.
    let rows_after = adapter.subject_access("alice@example.org").unwrap();
    assert!(rows_after.is_empty());

    // Original keys should be gone.
    let got = adapter.read(10, MemoryTier::Private, &ns_cal, "e1").unwrap();
    assert!(got.is_none());
}
