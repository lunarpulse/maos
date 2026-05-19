//! Integration test: I5 cross-Spirit private-tier isolation.
//! AC1 — Story 4.3.

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
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xBEE2));
    let adapter = Arc::new(MemoryManagerAdapter::new(private, shared, principal, tl));
    (adapter, tmp)
}

#[test]
fn spirit_a_private_not_readable_by_spirit_b() {
    let (adapter, _tmp) = make_adapter();
    adapter
        .write(1, MemoryTier::Private, &MemoryNamespace::Default, "secret", MemoryValue::Text("spirit-1".into()))
        .unwrap();

    // Spirit 2 attempts to read Spirit 1's private key.
    let got = adapter
        .read(2, MemoryTier::Private, &MemoryNamespace::Default, "secret")
        .unwrap();
    assert!(got.is_none(), "cross-Spirit private read must return None (I5)");
}

#[test]
fn spirit_b_cannot_write_as_spirit_a() {
    let (adapter, _tmp) = make_adapter();
    // Spirit 2 writes to its own namespace.
    adapter
        .write(2, MemoryTier::Private, &MemoryNamespace::Default, "k", MemoryValue::Text("b".into()))
        .unwrap();

    // Spirit 1's key should not exist (Spirit 2 wrote under pid=2).
    let got = adapter
        .read(1, MemoryTier::Private, &MemoryNamespace::Default, "k")
        .unwrap();
    assert!(got.is_none());
}
