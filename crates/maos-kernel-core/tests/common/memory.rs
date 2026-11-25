#![allow(dead_code)]

//! Memory-manager harness shared by the I12 digest-provider tests and the I12 ledger proof.

use std::sync::Arc;

use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};

pub fn make_memory() -> (Arc<MemoryManagerAdapter>, tempfile::TempDir) {
    let tmp = tempfile::TempDir::new().unwrap();
    let memory_root = tmp.path().join("memory");
    let db_path = tmp.path().join("audit.db");
    let private = Arc::new(PrivateMemoryStore::new(memory_root, 4 * 1024));
    let shared = Arc::new(SharedMemoryStore::open(&db_path).unwrap());
    let principal = Arc::new(PrincipalNamespaceIndex::open(&db_path).unwrap());
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xD1235));
    let adapter = Arc::new(MemoryManagerAdapter::new(private, shared, principal, tl));
    (adapter, tmp)
}
