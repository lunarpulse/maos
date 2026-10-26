//! Story 18.7: skills use the authenticated durable door, never a parallel writer.
#[path = "support/fixture_door.rs"]
mod fixture_door;

use fixture_door::{FixtureDoor, TOKEN};
use maos_skill::{LocalFsSkillQueueStore, QueueEntry, SkillAdmissionState, SkillQueueStore};

#[test]
fn list_show_approve_and_reject_use_the_same_authenticated_door() {
    let door = FixtureDoor::spawn();
    for args in [
        vec!["skills", "list"],
        vec!["skills", "show", "0123456789abcdef"],
        vec!["skills", "approve", "0123456789abcdef", "--actor", "alice"],
        vec!["skills", "reject", "fedcba9876543210", "--actor", "bob"],
    ] {
        let output = door.run(&args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let requests = door.requests();
    assert_eq!(requests.len(), 4);
    assert_eq!(
        (&*requests[0].method, &*requests[0].path),
        ("GET", "/v1/skills")
    );
    assert_eq!(
        (&*requests[1].method, &*requests[1].path),
        ("GET", "/v1/skills/0123456789abcdef")
    );
    for request in &requests {
        assert_eq!(
            request.authorization.as_deref(),
            Some(format!("Bearer {TOKEN}").as_str())
        );
    }
    for (index, approve, actor, entry) in [
        (2, true, "alice", "0123456789abcdef"),
        (3, false, "bob", "fedcba9876543210"),
    ] {
        assert_eq!(
            (&*requests[index].method, &*requests[index].path),
            ("POST", "/v1/skills/decide")
        );
        let body = requests[index].json_body();
        assert_eq!(body["entry"], entry);
        assert_eq!(body["approve"], approve);
        assert_eq!(body["actor"], actor);
    }
    assert!(!door
        .home_dir()
        .join(".local/share/maos/skills/queue.json")
        .exists());
}

#[test]
fn list_preserves_custom_discovery_roots_over_the_door() {
    let door = FixtureDoor::spawn();
    assert!(door
        .run(&[
            "skills",
            "list",
            "--root",
            "/scratch/a",
            "--root",
            "/scratch/b"
        ])
        .status
        .success());
    let requests = door.requests();
    assert_eq!(requests[0].method, "GET");
    assert_eq!(
        requests[0].json_body()["roots"],
        serde_json::json!(["/scratch/a", "/scratch/b"])
    );
}

#[test]
fn rejected_bearer_never_falls_back_to_offline_queue_mutation() {
    let door = FixtureDoor::spawn_replying(401, r#"{"error":"unauthorized"}"#);
    let store = LocalFsSkillQueueStore::at_path(
        door.home_dir().join(".local/share/maos/skills/queue.json"),
    );
    let entry = QueueEntry::package_shipped("skill.auth".into(), "1.0.0".into()).unwrap();
    store.save(&[entry]).unwrap();
    let before = std::fs::read(store.path()).unwrap();
    let output = door.run(&["skills", "approve", "skill.auth", "--actor", "alice"]);
    assert_eq!(output.status.code(), Some(77));
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(!door.maos_home().join("audit/transparency.sqlite").exists());
}

#[cfg(unix)]
#[test]
fn refused_connection_uses_offline_only_while_the_home_lock_is_free() {
    let door = FixtureDoor::spawn_dead();
    let store = LocalFsSkillQueueStore::at_path(
        door.home_dir().join(".local/share/maos/skills/queue.json"),
    );
    let entry = QueueEntry::package_shipped("skill.offline".into(), "1.0.0".into()).unwrap();
    store.save(&[entry]).unwrap();
    let home = std::fs::File::open(door.maos_home()).unwrap();
    rustix::fs::flock(&home, rustix::fs::FlockOperation::NonBlockingLockShared).unwrap();
    let before = std::fs::read(store.path()).unwrap();
    let blocked = door.run(&["skills", "reject", "skill.offline", "--actor", "alice"]);
    assert_eq!(
        blocked.status.code(),
        Some(69),
        "{}",
        String::from_utf8_lossy(&blocked.stderr)
    );
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    drop(home);
    let output = door.run(&["skills", "reject", "skill.offline", "--actor", "alice"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        store.load().unwrap()[0].state,
        SkillAdmissionState::Rejected
    );
    let log = maos_iac::adapter::transparency_log::TransparencyLogAdapter::open(
        &door.maos_home().join("audit/transparency.sqlite"),
        0,
    )
    .unwrap();
    assert!(log
        .query_approvals(None)
        .unwrap()
        .iter()
        .any(
            |row| row.target == "skill.offline@1.0.0" && row.capability == "skill.admission.reject"
        ));
}

#[test]
fn corrupt_v2_offline_show_and_decide_leave_payload_bytes_untouched() {
    let door = FixtureDoor::spawn_dead();
    let path = door.home_dir().join(".local/share/maos/skills/queue.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let bytes = br#"{"schema_version":"maos.skill-queue.v2","pending":[{"payload":null}]}"#;
    std::fs::write(&path, bytes).unwrap();
    for args in [
        vec!["skills", "show", "entry"],
        vec!["skills", "approve", "entry"],
    ] {
        let output = door.run(&args);
        assert!(!output.status.success());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}

#[cfg(unix)]
#[test]
fn offline_decision_retains_exclusive_liveness_lock_until_journal_and_queue_finish() {
    use rustix::fs::{flock, FlockOperation};
    let door = FixtureDoor::spawn_dead();
    let store = LocalFsSkillQueueStore::at_path(
        door.home_dir().join(".local/share/maos/skills/queue.json"),
    );
    let entry = QueueEntry::package_shipped("skill.retained".into(), "1.0.0".into()).unwrap();
    store.save(&[entry]).unwrap();
    let path = door.maos_home().join("audit/transparency.sqlite");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let log = maos_iac::adapter::transparency_log::TransparencyLogAdapter::open(&path, 0).unwrap();
    let writer = rusqlite::Connection::open(&path).unwrap();
    writer.execute_batch("BEGIN IMMEDIATE").unwrap();
    let child = std::process::Command::new(fixture_door::maosctl())
        .env_clear()
        .env("HOME", door.home_dir())
        .env("MAOS_HOME", door.maos_home())
        .env("XDG_DATA_HOME", door.xdg_dir())
        .args(["skills", "reject", "skill.retained", "--actor", "alice"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let root_probe = std::fs::File::open(door.maos_home()).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let retained = loop {
        match flock(&root_probe, FlockOperation::NonBlockingLockShared) {
            Err(rustix::io::Errno::WOULDBLOCK) => break true,
            Ok(()) => flock(&root_probe, FlockOperation::Unlock).unwrap(),
            Err(error) => panic!("liveness probe failed: {error}"),
        }
        if std::time::Instant::now() >= deadline {
            break false;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    };
    assert!(
        retained,
        "offline skills released the home guard before its durable work"
    );
    assert_eq!(store.load().unwrap()[0].state, SkillAdmissionState::Pending);
    assert_eq!(
        flock(&root_probe, FlockOperation::NonBlockingLockShared),
        Err(rustix::io::Errno::WOULDBLOCK)
    );
    writer.execute_batch("COMMIT").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    flock(&root_probe, FlockOperation::NonBlockingLockShared).unwrap();
    assert_eq!(
        store.load().unwrap()[0].state,
        SkillAdmissionState::Rejected
    );
    assert_eq!(log.query_approvals(None).unwrap().len(), 1);
}
