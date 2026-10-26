//! Story 18.7: the real door service shares the live author's serialization lock.
use maos_bin::operator_door::skill_operation;
use maos_control::OperatorOutcome;
use maos_iac::adapter::transparency_log::TransparencyLogAdapter;
use maos_skill::{
    AuthorIdentity, LocalFsSkillQueueStore, SkillAdmissionQueue, SkillAdmissionState,
    SkillQueueStore, SubmissionContext,
};
use std::path::Path;
use std::sync::Arc;

fn enqueue(queue: &mut SkillAdmissionQueue, name: &str) -> String {
    let author = AuthorIdentity {
        class: "researcher".into(),
        pid: 7,
        boot_nonce: "boot".into(),
    };
    let actions = std::collections::BTreeSet::new();
    let context = SubmissionContext {
        author: &author,
        declared_actions: &actions,
        principal_ids: &[],
    };
    let document = format!("---\nid='researcher.{name}'\nversion='1.0.0'\nname='Skill'\ndescription='d'\n---\nBody for {name}\n");
    let (entry, _) = queue.prepare_author(document, &context, &[]).unwrap();
    let id = entry.entry_id.clone();
    queue.commit_prepared(entry).unwrap();
    id
}

fn isolated(name: &str, run: impl FnOnce(&Path)) {
    const CHILD: &str = "MAOS_SKILL_DOOR_TEST_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let root = std::path::PathBuf::from(std::env::var_os("MAOS_SKILL_DOOR_TEST_ROOT").unwrap());
        run(&root);
        return;
    }
    let scratch = tempfile::tempdir().unwrap();
    let root = scratch.path();
    std::fs::create_dir_all(root.join("home")).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env_clear()
        .env(CHILD, "1")
        .env("MAOS_SKILL_DOOR_TEST_ROOT", root)
        .env("HOME", root.join("home"))
        .env("MAOS_HOME", root.join("maos"))
        .env("XDG_DATA_HOME", root.join("xdg"))
        .env("MAOS_MEMORY_ROOT", root.join("memory"))
        .env("MAOS_ARCHIVE_DIR", root.join("archives"))
        .env("MAOS_ERASURE_PROOFS_DIR", root.join("proofs"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn live_submission_and_door_decision_share_one_lock_without_lost_updates() {
    isolated(
        "live_submission_and_door_decision_share_one_lock_without_lost_updates",
        |root| {
            let log = Arc::new(TransparencyLogAdapter::open(&root.join("tl.db"), 0).unwrap());
            let store = LocalFsSkillQueueStore::new().unwrap();
            let mut queue = SkillAdmissionQueue::new();
            let first = enqueue(&mut queue, "first");
            store.save(queue.entries()).unwrap();
            let serialization = maos_bin::operator_door::skill_serialization();
            assert!(Arc::ptr_eq(
                &serialization,
                &maos_bin::operator_door::skill_serialization()
            ));
            let held = serialization.lock().unwrap();
            let (started_tx, started_rx) = std::sync::mpsc::channel();
            let (done_tx, done_rx) = std::sync::mpsc::channel();
            let worker_log = Arc::clone(&log);
            let selected = first.clone();
            let worker = std::thread::spawn(move || {
                started_tx.send(()).unwrap();
                let outcome =
                    skill_operation(&worker_log, Some(&selected), Some((true, "alice")), &[]);
                done_tx.send(outcome).unwrap();
            });
            started_rx.recv().unwrap();
            assert!(done_rx
                .recv_timeout(std::time::Duration::from_millis(50))
                .is_err());
            let second = enqueue(&mut queue, "second");
            store.save(queue.entries()).unwrap();
            drop(held);
            let outcome = done_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap();
            assert!(
                matches!(outcome, OperatorOutcome::Completed(_)),
                "{outcome:?}"
            );
            worker.join().unwrap();
            let restored = SkillAdmissionQueue::from_stored(store.load().unwrap()).unwrap();
            assert_eq!(
                restored.show(&first).unwrap().state,
                SkillAdmissionState::Admitted
            );
            assert_eq!(
                restored.show(&second).unwrap().state,
                SkillAdmissionState::Pending
            );
            assert!(root.join("home/.maos/skills/researcher.first.md").exists());
            let shown = skill_operation(&log, Some(&first), None, &[]);
            match shown {
                OperatorOutcome::Completed(value) => {
                    let entry: maos_skill::QueueEntry = serde_json::from_value(value).unwrap();
                    assert_eq!(&entry, restored.show(&first).unwrap());
                }
                other => panic!("show refused: {other:?}"),
            }
            let proposal = {
                let _guard = serialization.lock().unwrap();
                let mut queue = SkillAdmissionQueue::from_stored(store.load().unwrap()).unwrap();
                let author = AuthorIdentity {
                    class: "researcher".into(),
                    pid: 8,
                    boot_nonce: "second-boot".into(),
                };
                let actions = std::collections::BTreeSet::new();
                let context = SubmissionContext {
                    author: &author,
                    declared_actions: &actions,
                    principal_ids: &[],
                };
                let evidence = maos_domain::self_telemetry::SelfTelemetryReport::new(
                    8,
                    10,
                    20,
                    3,
                    1,
                    0,
                    0,
                    0,
                    vec![],
                    vec![],
                    21,
                )
                .unwrap();
                let payload = maos_skill::build_proposal(
                    "researcher.first".into(),
                    "1.0.0".into(),
                    "-old\n+evidence-guided revision".into(),
                    evidence,
                )
                .unwrap();
                let (entry, _) = queue.prepare_proposal(payload, &context).unwrap();
                queue.commit_prepared(entry.clone()).unwrap();
                store.save(queue.entries()).unwrap();
                entry
            };
            match skill_operation(&log, Some(&proposal.entry_id), None, &[]) {
                OperatorOutcome::Completed(value) => {
                    let shown: maos_skill::QueueEntry = serde_json::from_value(value).unwrap();
                    assert_eq!(
                        shown, proposal,
                        "show must preserve diff, evidence and sealed identity"
                    );
                }
                other => panic!("proposal show refused: {other:?}"),
            }
            let outcome =
                skill_operation(&log, Some(&proposal.entry_id), Some((false, "alice")), &[]);
            assert!(matches!(outcome, OperatorOutcome::Completed(_)));
            let restored = SkillAdmissionQueue::from_stored(store.load().unwrap()).unwrap();
            assert_eq!(
                restored.show(&first).unwrap().state,
                SkillAdmissionState::Admitted
            );
            assert_eq!(
                restored.show(&proposal.entry_id).unwrap().state,
                SkillAdmissionState::Rejected
            );
            assert!(log
                .query_approvals(None)
                .unwrap()
                .iter()
                .any(|row| row.target == proposal.approval_target()
                    && row.capability == "skill.admission.reject"));
            let corrupt =
                br#"{"schema_version":"maos.skill-queue.v2","pending":[{"payload":null}]}"#;
            std::fs::write(store.path(), corrupt).unwrap();
            for decision in [None, Some((false, "alice"))] {
                let outcome = skill_operation(&log, Some(&second), decision, &[]);
                assert!(matches!(outcome, OperatorOutcome::Failed { .. }));
                assert_eq!(std::fs::read(store.path()).unwrap(), corrupt);
            }
            assert_eq!(log.query_approvals(None).unwrap().len(), 2);
        },
    );
}

#[cfg(unix)]
#[test]
fn root_liveness_set_creates_and_locks_an_initially_missing_home() {
    isolated(
        "root_liveness_set_creates_and_locks_an_initially_missing_home",
        |root| {
            let home = root.join("maos");
            assert!(!home.exists());
            let locks = maos_bin::operator_door::acquire_store_lock_set(
                &root.join("audit/tl.db"),
                maos_bin::operator_door::StoreLockRole::RootShared,
            )
            .unwrap();
            assert!(locks
                .locked_paths()
                .contains(&std::fs::canonicalize(&home).unwrap()));
            let exclusive = std::fs::File::open(&home).unwrap();
            assert_eq!(
                rustix::fs::flock(
                    &exclusive,
                    rustix::fs::FlockOperation::NonBlockingLockExclusive
                ),
                Err(rustix::io::Errno::WOULDBLOCK)
            );
            drop(locks);
            rustix::fs::flock(
                &exclusive,
                rustix::fs::FlockOperation::NonBlockingLockExclusive,
            )
            .unwrap();
        },
    );
}
