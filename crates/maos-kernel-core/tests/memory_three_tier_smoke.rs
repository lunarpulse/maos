//! Integration test: three-tier memory write/read/scan + collective typed error.
//! AC1 — Story 4.3.

use std::sync::Arc;

use maos_domain::memory::{MemoryError, MemoryNamespace, MemoryTier, MemoryValue};
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
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xBEE1));
    let adapter = Arc::new(MemoryManagerAdapter::new(private, shared, principal, tl));
    (adapter, tmp)
}

#[test]
fn write_read_private() {
    let (adapter, _tmp) = make_adapter();
    let val = MemoryValue::Text("hello private".into());
    adapter
        .write(1, MemoryTier::Private, &MemoryNamespace::Default, "k1", val.clone())
        .unwrap();
    let got = adapter
        .read(1, MemoryTier::Private, &MemoryNamespace::Default, "k1")
        .unwrap();
    assert_eq!(got, Some(val));
}

#[test]
fn write_read_shared() {
    let (adapter, _tmp) = make_adapter();
    let val = MemoryValue::Text("hello shared".into());
    adapter
        .write(2, MemoryTier::Shared, &MemoryNamespace::Coordination, "s1", val.clone())
        .unwrap();
    let got = adapter
        .read(2, MemoryTier::Shared, &MemoryNamespace::Coordination, "s1")
        .unwrap();
    assert_eq!(got, Some(val));
}

#[test]
fn collective_returns_typed_error() {
    let (adapter, _tmp) = make_adapter();
    let err = adapter
        .write(1, MemoryTier::Collective, &MemoryNamespace::Default, "k", MemoryValue::Text("x".into()))
        .unwrap_err();
    match err {
        MemoryError::CollectiveNotYetAvailable {
            ship_target,
            landing_story,
        } => {
            assert_eq!(ship_target, "v1.5");
            assert_eq!(landing_story, "E10 Story 10.4");
        }
        _ => panic!("expected CollectiveNotYetAvailable, got {err:?}"),
    }
}

#[test]
fn scan_private_returns_entries() {
    let (adapter, _tmp) = make_adapter();
    adapter
        .write(1, MemoryTier::Private, &MemoryNamespace::Default, "alpha", MemoryValue::Text("a".into()))
        .unwrap();
    adapter
        .write(1, MemoryTier::Private, &MemoryNamespace::Default, "beta", MemoryValue::Text("b".into()))
        .unwrap();
    let results = adapter
        .scan(1, MemoryTier::Private, &MemoryNamespace::Default, "", 10)
        .unwrap();
    assert_eq!(results.len(), 2);
}
