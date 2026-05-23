//! Integration test: Cross-Spirit isolation hook wiring under `spirit_test` feature.
//! AC6 — Story 4.3.

#![cfg(feature = "spirit_test")]

use std::sync::Arc;

use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use maos_spirit_sdk::spirit_test::{
    AttemptResult, DefaultIsolationHook, IsolationHookPoint, ObservationResult,
};
use parking_lot::Mutex;
use tempfile::TempDir;

#[test]
fn isolation_hooks_fire_on_memory_write_and_read() {
    let tmp = TempDir::new().unwrap();
    let memory_root = tmp.path().join("memory");
    let db_path = tmp.path().join("audit.db");

    let private = Arc::new(PrivateMemoryStore::new(memory_root, 4 * 1024));
    let shared = Arc::new(SharedMemoryStore::open(&db_path).unwrap());
    let principal = Arc::new(PrincipalNamespaceIndex::open(&db_path).unwrap());
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xBEE7));
    let concrete_hook = Arc::new(Mutex::new(DefaultIsolationHook::default()));
    let hook: Arc<Mutex<dyn IsolationHookPoint + Send>> = concrete_hook.clone();
    let adapter = Arc::new(
        MemoryManagerAdapter::new(private, shared, principal, tl).with_isolation_hook(hook),
    );

    // Spirit A writes.
    adapter
        .write(
            1,
            MemoryTier::Private,
            &MemoryNamespace::Default,
            "k",
            MemoryValue::Text("a".into()),
        )
        .unwrap();

    // Spirit B reads (returns None due to I5).
    let _ = adapter.read(2, MemoryTier::Private, &MemoryNamespace::Default, "k");

    // Hooks should have fired for both operations.
    let records = concrete_hook.lock().records.clone();
    assert!(
        records
            .iter()
            .any(|r| r.hook_name == "before_spirit_a_attempt"),
        "before_spirit_a_attempt should fire"
    );
    assert!(
        records
            .iter()
            .any(|r| r.hook_name == "after_spirit_a_attempt"),
        "after_spirit_a_attempt should fire"
    );
    assert!(
        records
            .iter()
            .any(|r| r.hook_name == "before_spirit_b_observe"),
        "before_spirit_b_observe should fire"
    );
    assert!(
        records
            .iter()
            .any(|r| r.hook_name == "after_spirit_b_observe"),
        "after_spirit_b_observe should fire"
    );
}
