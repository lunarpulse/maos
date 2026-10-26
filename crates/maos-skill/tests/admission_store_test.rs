//! V1 migration, fail-closed v2 persistence and atomic-write witnesses.
use maos_skill::{
    ESkillQueue, ESkillStore, LocalFsSkillQueueStore, QueueEntry, SkillAdmissionQueue,
    SkillAdmissionState, SkillId, SkillQueueStore, SkillVersion,
};
use std::{fs, path::PathBuf};
fn package(id: &str, state: SkillAdmissionState) -> QueueEntry {
    let mut e =
        QueueEntry::package_shipped(SkillId::from(id), SkillVersion::from("1.0.0")).unwrap();
    e.state = state;
    e
}
fn store(dir: &tempfile::TempDir) -> LocalFsSkillQueueStore {
    LocalFsSkillQueueStore::at_path(dir.path().join("queue.json"))
}
#[test]
fn restart_round_trip_recovers_typed_states_and_identity() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);
    let entries = vec![
        package("pkg.a", SkillAdmissionState::Pending),
        package("pkg.b", SkillAdmissionState::Admitted),
        package("pkg.c", SkillAdmissionState::Rejected),
    ];
    store.save(&entries).unwrap();
    let restored = SkillAdmissionQueue::from_stored(store.load().unwrap()).unwrap();
    assert_eq!(restored.entries(), entries);
    assert_eq!(restored.pending_count(), 1);
    assert!(restored.audit_trail().is_empty());
    let raw: serde_json::Value = serde_json::from_slice(&fs::read(store.path()).unwrap()).unwrap();
    assert_eq!(raw["schema_version"], "maos.skill-queue.v2");
}
#[test]
fn v1_label_only_entries_migrate_without_label_loss_and_write_v2() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);
    let pending: Vec<_> = ["package_shipped", "author_self", "revision_proposal"].into_iter().enumerate().map(|(i,label)| serde_json::json!({"id":format!("old.{i}"),"version":"1.0.0","entry_path":label,"state":"Pending"})).collect();
    fs::write(
        store.path(),
        serde_json::to_vec(
            &serde_json::json!({"schema_version":"maos.skill-queue.v1","pending":pending}),
        )
        .unwrap(),
    )
    .unwrap();
    let entries = store.load().unwrap();
    assert!(entries
        .iter()
        .all(|e| e.legacy && e.payload.is_none() && e.author.is_none()));
    assert_eq!(entries[2].entry_path, "revision_proposal");
    assert!(entries[2].approval_target().contains('#'));
    store.save(&entries).unwrap();
    assert_eq!(store.load().unwrap(), entries);
    assert!(fs::read_to_string(store.path())
        .unwrap()
        .contains("maos.skill-queue.v2"));
}
#[test]
fn unknown_and_missing_versions_are_hard_errors_and_never_overwritten() {
    for version in [Some("maos.skill-queue.v99"), None] {
        let dir = tempfile::tempdir().unwrap();
        let store = store(&dir);
        let mut value = serde_json::json!({"pending":[]});
        if let Some(version) = version {
            value["schema_version"] = version.into();
        }
        let original = serde_json::to_vec(&value).unwrap();
        fs::write(store.path(), &original).unwrap();
        assert!(matches!(
            store.load(),
            Err(ESkillStore::UnknownSchemaVersion(_))
        ));
        assert!(store.save(&[]).is_err());
        assert_eq!(fs::read(store.path()).unwrap(), original);
    }
}
#[test]
fn unreadable_or_corrupt_v2_refuses_load_and_save() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);
    for original in [
        b"{broken".as_slice(),
        br#"{"schema_version":"maos.skill-queue.v2","pending":[{"id":"missing-payload"}]}"#
            .as_slice(),
    ] {
        fs::write(store.path(), original).unwrap();
        assert!(matches!(store.load(), Err(ESkillStore::Json(_))));
        assert!(store.save(&[]).is_err());
        assert_eq!(fs::read(store.path()).unwrap(), original);
    }
    fs::remove_file(store.path()).unwrap();
    fs::create_dir(store.path()).unwrap();
    assert!(matches!(store.load(), Err(ESkillStore::Io(_))));
}
#[test]
fn unknown_labels_refuse_without_package_fallback() {
    let mut entry = package("pkg.core", SkillAdmissionState::Pending);
    entry.entry_path = "unknown-path".into();
    entry.seal().unwrap();
    assert!(matches!(
        SkillAdmissionQueue::from_stored(vec![entry]),
        Err(ESkillStore::InvalidEntry(ESkillQueue::UnknownEntryPath(_)))
    ));
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);
    fs::write(store.path(), br#"{"schema_version":"maos.skill-queue.v1","pending":[{"id":"pkg.core","version":"1.0.0","entry_path":"unknown","state":"Pending"}]}"#).unwrap();
    assert!(matches!(
        store.load(),
        Err(ESkillStore::InvalidEntry(ESkillQueue::UnknownEntryPath(_)))
    ));
}
#[test]
fn absent_file_is_empty_but_not_other_io_errors() {
    let dir = tempfile::tempdir().unwrap();
    assert!(store(&dir).load().unwrap().is_empty());
    let blocker = dir.path().join("blocker");
    fs::write(&blocker, "file").unwrap();
    assert!(matches!(
        LocalFsSkillQueueStore::at_path(blocker.join("queue.json")).load(),
        Err(ESkillStore::Io(_))
    ));
}
#[test]
fn no_data_home_is_typed_without_mutating_shared_environment() {
    const MARKER: &str = "MAOS_SKILL_NO_HOME_PROBE";
    if std::env::var_os(MARKER).is_some() {
        assert!(matches!(
            LocalFsSkillQueueStore::new(),
            Err(ESkillStore::NoDataHome)
        ));
        return;
    }
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "no_data_home_is_typed_without_mutating_shared_environment",
        ])
        .env(MARKER, "1")
        .env_remove("HOME")
        .status()
        .unwrap();
    assert!(status.success());
}
#[test]
fn atomic_write_failure_preserves_prior_valid_queue_and_leaks_no_temp() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);
    let entries = vec![package("pkg.core", SkillAdmissionState::Pending)];
    store.save(&entries).unwrap();
    let blocker = dir.path().join("blocker");
    fs::write(&blocker, "not a directory").unwrap();
    assert!(maos_skill::store::atomic_write(&blocker.join("queue.json"), b"bad").is_err());
    assert_eq!(store.load().unwrap(), entries);
    assert!(fs::read_dir(dir.path()).unwrap().all(|e| !e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .contains(".tmp.")));
}
#[test]
fn stray_partial_temp_does_not_replace_valid_queue() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);
    let entries = vec![package("pkg.core", SkillAdmissionState::Pending)];
    store.save(&entries).unwrap();
    fs::write(dir.path().join("queue.json.tmp.99999"), b"CORRUPTED").unwrap();
    assert_eq!(store.load().unwrap(), entries);
}
#[test]
fn invalid_digest_refuses_and_preserves_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);
    let mut entry = package("pkg.core", SkillAdmissionState::Pending);
    store.save(&[entry.clone()]).unwrap();
    let original = fs::read(store.path()).unwrap();
    entry.payload_sha256 = "tampered".into();
    assert!(matches!(
        store.save(&[entry]),
        Err(ESkillStore::InvalidEntry(ESkillQueue::InvalidStoredEntry(
            _
        )))
    ));
    assert_eq!(fs::read(store.path()).unwrap(), original);
}
#[test]
fn actors_are_not_persisted_with_package_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);
    let skill = maos_skill::parse_skill(
        "---\nid='pkg.core'\nversion='1.0.0'\nname='Core'\ndescription='d'\n---\nBody",
    )
    .unwrap();
    let mut q = SkillAdmissionQueue::new();
    q.enqueue_skill(
        skill,
        maos_skill::SkillEntryPath::PackageShipped,
        "personal-operator-name",
    )
    .unwrap();
    store.save(&q.to_stored()).unwrap();
    let raw = fs::read_to_string(store.path()).unwrap();
    assert!(!raw.contains("personal-operator-name"));
    assert!(!raw.contains("\"actor\""));
    assert!(raw.contains("pkg.core"));
}
#[test]
fn explicit_path_does_not_need_a_home_resolver() {
    let path = PathBuf::from("private/queue.json");
    assert_eq!(LocalFsSkillQueueStore::at_path(path.clone()).path(), path);
}
