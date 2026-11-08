use maos_domain::ports::registry::YankEntry;
use maos_registry::storage::{LocalFsRegistryStorage, RegistryStorage};
use std::sync::Arc;
fn journal() -> (tempfile::TempDir, Arc<dyn RegistryStorage>) {
    let root = tempfile::tempdir().unwrap();
    let entries: Vec<_> = (1..=4097)
        .map(|sequence| {
            let mut entry = YankEntry::new("sample".into(), "1.0.0".into(), 0, "withdrawn".into());
            entry.sequence = sequence;
            entry
        })
        .collect();
    std::fs::write(
        root.path().join("yanks.json"),
        serde_json::to_vec(&entries).unwrap(),
    )
    .unwrap();
    let storage = Arc::new(LocalFsRegistryStorage::at_path(root.path().to_owned()).unwrap());
    (root, storage)
}
#[test]
fn large_delta_is_refused_instead_of_returning_an_incomplete_signed_head_f6() {
    let (_root, storage) = journal();
    let result = storage.yanks_since(0);
    assert!(result.is_err(), "uncapped yank history was returned");
}
#[test]
fn filtered_snapshot_keeps_its_authenticated_complete_head_f6() {
    let (_root, storage) = journal();
    let list = storage.yanks_since(4097).unwrap();
    assert!(list.entries.is_empty());
    assert_eq!(list.head, Some(4097));
}
#[test]
fn large_history_small_delta_uses_storage_lower_bound_f6() {
    let (_root, storage) = journal();
    let response = maos_registry::handlers::yanks_since::handle_yanks_since(
        &storage,
        &maos_registry::operations::YanksSinceArgs { since_ns: 4096 },
    )
    .unwrap();
    assert_eq!(response["head"], 4097);
    assert_eq!(response["entries"].as_array().unwrap().len(), 1);
    assert_eq!(response["entries"][0]["sequence"], 4097);
}
