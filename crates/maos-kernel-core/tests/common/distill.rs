#![allow(dead_code)]

//! Distillate-writer harness shared by the I11 audit-chain tests and the I11 ledger proof.

use std::sync::Arc;

use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};

pub fn make_writer(
    nonce: u64,
) -> (
    DistillateWriter,
    Arc<TransparencyLogAdapter>,
    tempfile::TempDir,
) {
    let tmp = tempfile::TempDir::new().unwrap();
    let memory_root = tmp.path().join("memory");
    let db_path = tmp.path().join("audit.db");
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(nonce));
    let private = Arc::new(PrivateMemoryStore::new(memory_root, 4 * 1024));
    let shared = Arc::new(SharedMemoryStore::open(&db_path).unwrap());
    let principal_index = Arc::new(PrincipalNamespaceIndex::open(&db_path).unwrap());
    let memory = Arc::new(MemoryManagerAdapter::new(
        private,
        shared,
        principal_index,
        Arc::clone(&tl),
    ));
    let writer = DistillateWriter::new(Arc::clone(&tl), memory);
    (writer, tl, tmp)
}
