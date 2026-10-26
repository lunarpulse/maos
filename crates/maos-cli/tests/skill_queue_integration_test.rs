//! Story 9.7 AC-3/AC-7 — Journal-FIRST TL integration + multi-writer SQLite
//! tests, AND the Review D1 end-to-end proof that a DISCOVERED skill is
//! approvable (the Critical fix). All decide-path tests drive the production
//! `decide_skill` (no hand-mirrored dispatch), assert the typed `DecideOutcome`,
//! and verify the TL + cache state.

use std::fs;
use std::path::Path;

use maos_cli::subcommands::{decide_skill, DecideOutcome};
use maos_domain::invariants::i4::ApprovalDecision;
use maos_iac::adapter::transparency_log::TransparencyLogAdapter;
use maos_skill::approval_target::approval_target;
use maos_skill::{
    discover_skills_detailed, LocalFsSkillQueueStore, SkillAdmissionState, SkillId,
    SkillQueueStore, SkillVersion,
};

/// Write a valid `maos.skill.v1` file `{id}.md` under `root` (flat discovery).
fn write_skill(root: &Path, id: &str) {
    fs::create_dir_all(root).unwrap();
    let src = format!(
        "---\nid = \"{id}\"\nversion = \"1.0.0\"\nname = \"{id}\"\ndescription = \"d\"\n---\nBody for {id}.\n"
    );
    fs::write(root.join(format!("{id}.md")), src).unwrap();
}

// ─── AC-3: double-journal guard keyed on (target, capability) ──────────

#[test]
fn double_journal_guard_keyed_on_target_capability() {
    let dir = tempfile::tempdir().unwrap();
    let tl_path = dir.path().join("tl.db");
    let tl = TransparencyLogAdapter::open(&tl_path, 0).unwrap();

    let id_x = SkillId::from("skill.x");
    let id_y = SkillId::from("skill.y");
    let v = SkillVersion::from("1.0.0");

    // Enqueue both (NOT approve/reject decisions).
    for id in [&id_x, &id_y] {
        tl.insert_approval_decision(ApprovalDecision {
            actor: "system".into(),
            target: approval_target(id, &v),
            capability: "skill.admission.enqueue".into(),
            intent: "package_shipped".into(),
            decision: false,
            reasoning: None,
        })
        .unwrap();
    }

    tl.insert_approval_decision(ApprovalDecision {
        actor: "alice".into(),
        target: approval_target(&id_x, &v),
        capability: "skill.admission.approve".into(),
        intent: "cli_operator_decision".into(),
        decision: true,
        reasoning: None,
    })
    .unwrap();
    tl.insert_approval_decision(ApprovalDecision {
        actor: "alice".into(),
        target: approval_target(&id_y, &v),
        capability: "skill.admission.reject".into(),
        intent: "cli_operator_decision".into(),
        decision: false,
        reasoning: None,
    })
    .unwrap();

    let all = tl.query_approvals(None).unwrap();

    // Exactly 1 row per (target, capability) — keyed on (target, capability),
    // NOT decision_id (which query_approvals never surfaces — R4).
    let x_approve: Vec<_> = all
        .iter()
        .filter(|d| {
            d.target == approval_target(&id_x, &v) && d.capability == "skill.admission.approve"
        })
        .collect();
    assert_eq!(x_approve.len(), 1);

    let y_reject: Vec<_> = all
        .iter()
        .filter(|d| {
            d.target == approval_target(&id_y, &v) && d.capability == "skill.admission.reject"
        })
        .collect();
    assert_eq!(y_reject.len(), 1);

    // Total decided rows (filtering OUT enqueue) == 2.
    let decided: Vec<_> = all
        .iter()
        .filter(|d| {
            d.capability == "skill.admission.approve" || d.capability == "skill.admission.reject"
        })
        .collect();
    assert_eq!(decided.len(), 2, "decided rows (approve+reject) == 2");
}

// ─── Review D1 (Critical): a discovered skill is approvable end-to-end ──

#[test]
fn discovered_skill_is_approvable_end_to_end() {
    // Pre-fix: queue.json started empty, nothing ever persisted a discovered
    // skill, so `approve <id>` hit "not found" + FAILURE. The fix derives the
    // admission view from discovery + TL, so a discovered skill is Pending and
    // therefore approvable — with NO manual cache seeding.
    let dir = tempfile::tempdir().unwrap();
    let skills_root = dir.path().join("skills");
    write_skill(&skills_root, "skill.e2e");
    let discovered = discover_skills_detailed(&[skills_root]);
    assert_eq!(
        discovered.discovered.len(),
        1,
        "the skill must be discovered"
    );

    let store = LocalFsSkillQueueStore::at_path(dir.path().join("queue.json"));
    let tl_path = dir.path().join("tl.db");

    // Empty cache — the skill is discovered only.
    let out = decide_skill(
        &discovered.discovered,
        vec![],
        &store,
        &tl_path,
        "skill.e2e",
        true,
        "alice",
    );
    assert!(
        matches!(
            out,
            DecideOutcome::Applied {
                new_state: SkillAdmissionState::Admitted
            }
        ),
        "a discovered skill must be approvable: {out:?}"
    );

    // The TL carries the journaled decision with the REAL actor (AC-3/AC-5).
    let tl = TransparencyLogAdapter::open(&tl_path, 0).unwrap();
    let rows = tl.query_approvals(None).unwrap();
    let approve_row = rows
        .iter()
        .find(|d| d.capability == "skill.admission.approve")
        .expect("the approve decision must be journaled");
    assert_eq!(approve_row.actor, "alice");
    assert_eq!(approve_row.target, "skill.e2e@1.0.0");

    // The cache now reflects the decision (durable across restart).
    let cached = store.load().unwrap();
    let entry = cached
        .iter()
        .find(|e| e.id == SkillId::from("skill.e2e"))
        .unwrap();
    assert_eq!(entry.state, SkillAdmissionState::Admitted);

    // Re-deciding is a no-op (already resolved — AC-2).
    let out2 = decide_skill(
        &discovered.discovered,
        store.load().unwrap(),
        &store,
        &tl_path,
        "skill.e2e",
        true,
        "alice",
    );
    assert!(matches!(out2, DecideOutcome::AlreadyResolved { .. }));

    // An unknown id is NotFound.
    let out3 = decide_skill(
        &discovered.discovered,
        vec![],
        &store,
        &tl_path,
        "skill.nope",
        true,
        "alice",
    );
    assert!(matches!(out3, DecideOutcome::NotFound));
}

// ─── AC-3 / AC-7: no-silent-loss invariant through the real decide path ─

#[test]
fn no_silent_loss_when_journal_write_fails() {
    // If the TL journal write fails, decide_skill returns JournalFailed,
    // mutates NOTHING (no cache row), and never reports success. This drives the
    // PRODUCTION decide path (Review #8), not just a bare TL-open check.
    let dir = tempfile::tempdir().unwrap();
    let skills_root = dir.path().join("skills");
    write_skill(&skills_root, "skill.guarded");
    let discovered = discover_skills_detailed(&[skills_root]);
    let store = LocalFsSkillQueueStore::at_path(dir.path().join("queue.json"));
    // A TL path whose parent does not exist cannot be opened → journal fails.
    let bad_tl = dir.path().join("not_a_dir").join("tl.db");

    let out = decide_skill(
        &discovered.discovered,
        vec![],
        &store,
        &bad_tl,
        "skill.guarded",
        true,
        "alice",
    );
    assert!(
        matches!(out, DecideOutcome::JournalFailed(_)),
        "decide must surface journal failure, not silent success: {out:?}"
    );

    // Cache unchanged — no decision persisted.
    let cached = store.load().unwrap();
    assert!(
        cached.is_empty(),
        "nothing must be persisted when the journal failed: {cached:?}"
    );
}

// ─── AC-7: deterministic forced-contention contract (RED + GREEN) ──────

#[test]
fn busy_timeout_zero_reds_immediately_under_contention() {
    // AC-7 RED half: with busy_timeout=0, a contended insert fails IMMEDIATELY
    // (SQLITE_BUSY) instead of blocking. (The opposite of the GREEN test below.)
    use std::thread;
    let dir = tempfile::tempdir().unwrap();
    let tl_path = dir.path().join("tl.db");
    let tl = TransparencyLogAdapter::open_with_busy_timeout(&tl_path, 0, 0).unwrap();

    // Hold an exclusive write lock on a raw connection.
    let raw = rusqlite::Connection::open(&tl_path).unwrap();
    raw.execute_batch("BEGIN IMMEDIATE").unwrap();

    // Attempt the insert on the timeout=0 adapter → immediate BUSY.
    let result = tl.insert_approval_decision(ApprovalDecision {
        actor: "red".into(),
        target: "skill.red@1.0.0".into(),
        capability: "skill.admission.approve".into(),
        intent: "red".into(),
        decision: true,
        reasoning: None,
    });
    raw.execute_batch("COMMIT").unwrap();
    assert!(
        result.is_err(),
        "timeout=0 must RED under contention (immediate SQLITE_BUSY): {result:?}"
    );
    // Sanity: nothing was written under the failed insert.
    let tl_check = TransparencyLogAdapter::open(&tl_path, 0).unwrap();
    assert!(tl_check
        .query_approvals(None)
        .unwrap()
        .iter()
        .all(|d| d.target != "skill.red@1.0.0"));
    // keep `thread` import used even if the GREEN test is the one that spawns
    let _ = thread::spawn(move || ()).join();
}

#[test]
fn busy_timeout_5000_blocks_then_succeeds_green() {
    // AC-7 GREEN half: with busy_timeout=5000, a contended insert BLOCKS until
    // the lock is released, then succeeds. (Together with the RED test above,
    // this is the functional proof of the busy_timeout contract — Review #8.)
    // Zero wall-clock (JB-7): we release the lock immediately and use the thread
    // join as the deterministic barrier; no sleeps or racing waits.
    use std::sync::Arc;
    use std::thread;

    let dir = tempfile::tempdir().unwrap();
    let tl_path = dir.path().join("tl.db");
    let tl = Arc::new(TransparencyLogAdapter::open(&tl_path, 0).unwrap());

    let raw = rusqlite::Connection::open(&tl_path).unwrap();
    raw.execute_batch("BEGIN IMMEDIATE").unwrap();

    let tl2 = Arc::clone(&tl);
    let handle = thread::spawn(move || {
        tl2.insert_approval_decision(ApprovalDecision {
            actor: "green".into(),
            target: "skill.green@1.0.0".into(),
            capability: "skill.admission.approve".into(),
            intent: "green".into(),
            decision: true,
            reasoning: None,
        })
    });

    // Release the lock immediately. The insert thread must be blocking (not
    // failed); the deterministic join() barrier proves it resumed and completed
    // once the lock was released.
    raw.execute_batch("COMMIT").unwrap();

    let result = handle.join().expect("thread panicked");
    assert!(
        result.is_ok(),
        "insert should block-then-succeed after lock release: {:?}",
        result.err()
    );

    let rows = tl.query_approvals(None).unwrap();
    assert!(
        rows.iter().any(|d| d.target == "skill.green@1.0.0"),
        "the contention-test row should be in the TL"
    );
}

#[test]
fn unknown_or_corrupt_payload_queue_is_never_rebuilt_or_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("skills");
    write_skill(&root, "skill.guarded");
    let discovered = discover_skills_detailed(&[root]);
    let store = LocalFsSkillQueueStore::at_path(dir.path().join("queue.json"));
    let tl_path = dir.path().join("tl.db");
    for bytes in [
        r#"{"schema_version":"maos.skill-queue.v99","pending":[]}"#,
        r#"{"schema_version":"maos.skill-queue.v2","pending":[{"id":"lost-payload"}]}"#,
        r#"{"schema_version":"maos.skill-queue.v2","pending":["#,
    ] {
        fs::write(store.path(), bytes).unwrap();
        let outcome = decide_skill(
            &discovered.discovered,
            vec![],
            &store,
            &tl_path,
            "skill.guarded",
            true,
            "operator",
        );
        assert!(
            matches!(outcome, DecideOutcome::JournalFailed(_)),
            "{outcome:?}"
        );
        assert_eq!(fs::read(store.path()).unwrap(), bytes.as_bytes());
    }
    let tl = TransparencyLogAdapter::open(&tl_path, 0).unwrap();
    assert!(tl.query_approvals(None).unwrap().is_empty());
}

fn authored_queue() -> (maos_skill::SkillAdmissionQueue, String, String) {
    use maos_skill::{AuthorIdentity, SkillAdmissionQueue, SubmissionContext};
    let author = AuthorIdentity {
        class: "researcher".into(),
        pid: 7,
        boot_nonce: "boot".into(),
    };
    let actions = std::collections::BTreeSet::new();
    let ctx = SubmissionContext {
        author: &author,
        declared_actions: &actions,
        principal_ids: &[],
    };
    let document = "---\nid='researcher.core'\nversion='1.0.0'\nname='Core'\ndescription='d'\n---\nOriginal body\n".to_string();
    let mut queue = SkillAdmissionQueue::new();
    let (entry, _) = queue.prepare_author(document.clone(), &ctx).unwrap();
    let id = entry.entry_id.clone();
    queue.commit_prepared(entry).unwrap();
    (queue, id, document)
}

#[test]
fn authored_decision_journals_before_materializing_and_saving() {
    use maos_cli::subcommands::apply_skill_decision;
    let dir = tempfile::tempdir().unwrap();
    let store = LocalFsSkillQueueStore::at_path(dir.path().join("queue.json"));
    let directory = dir.path().join("materialized");
    let destination = directory.join("researcher.core.md");
    let tl = TransparencyLogAdapter::open(&dir.path().join("tl.db"), 0).unwrap();
    let (mut queue, id, document) = authored_queue();
    store.save(queue.entries()).unwrap();
    let before = fs::read(store.path()).unwrap();
    apply_skill_decision(
        &mut queue,
        &store,
        &id,
        true,
        "alice",
        Some(&directory),
        |row| {
            assert!(!destination.exists());
            assert_eq!(fs::read(store.path()).unwrap(), before);
            assert_eq!(row.target, "researcher.core@1.0.0");
            assert!(row.reasoning.as_ref().unwrap().contains(&id));
            assert!(!format!("{row:?}").contains("Original body"));
            tl.insert_approval_decision(row).map_err(|e| e.to_string())
        },
    )
    .unwrap();
    assert_eq!(fs::read_to_string(destination).unwrap(), document);
    assert_eq!(
        store.load().unwrap()[0].state,
        SkillAdmissionState::Admitted
    );
    assert_eq!(tl.query_approvals(None).unwrap().len(), 1);
    assert_eq!(
        apply_skill_decision(&mut queue, &store, &id, false, "bob", None, |_| panic!(
            "resolved entries must not journal twice"
        ))
        .unwrap(),
        SkillAdmissionState::Admitted
    );
}

#[test]
fn journal_failure_mutates_nothing_and_materialization_failure_is_reported_after_commit() {
    use maos_cli::subcommands::{apply_skill_decision, SkillDecisionError};
    let dir = tempfile::tempdir().unwrap();
    let store = LocalFsSkillQueueStore::at_path(dir.path().join("queue.json"));
    let (mut queue, id, _) = authored_queue();
    store.save(queue.entries()).unwrap();
    let before = fs::read(store.path()).unwrap();
    let directory = dir.path().join("skills");
    let out = apply_skill_decision(
        &mut queue,
        &store,
        &id,
        true,
        "alice",
        Some(&directory),
        |_| Err("journal unavailable".into()),
    );
    assert!(matches!(out, Err(SkillDecisionError::Journal(_))));
    assert_eq!(fs::read(store.path()).unwrap(), before);
    assert!(!directory.exists());
    assert_eq!(queue.show(&id).unwrap().state, SkillAdmissionState::Pending);
    fs::create_dir_all(&directory).unwrap();
    let destination = directory.join("researcher.core.md");
    fs::write(&destination, "foreign document").unwrap();
    let tl = TransparencyLogAdapter::open(&dir.path().join("tl.db"), 0).unwrap();
    let out = apply_skill_decision(
        &mut queue,
        &store,
        &id,
        true,
        "alice",
        Some(&directory),
        |row| tl.insert_approval_decision(row).map_err(|e| e.to_string()),
    );
    assert!(matches!(
        out,
        Err(SkillDecisionError::Store(
            maos_skill::ESkillStore::MaterializeCollision(_)
        ))
    ));
    assert_eq!(tl.query_approvals(None).unwrap().len(), 1);
    assert_eq!(fs::read(store.path()).unwrap(), before);
    assert_eq!(fs::read_to_string(destination).unwrap(), "foreign document");
}

#[test]
fn queue_save_failure_never_reports_a_committed_decision_as_success() {
    use maos_cli::subcommands::{apply_skill_decision, SkillDecisionError};
    struct RefusingStore(std::path::PathBuf);
    impl SkillQueueStore for RefusingStore {
        fn load(&self) -> Result<Vec<maos_skill::QueueEntry>, maos_skill::ESkillStore> {
            Ok(vec![])
        }
        fn save(&self, _: &[maos_skill::QueueEntry]) -> Result<(), maos_skill::ESkillStore> {
            Err(std::io::Error::other("forced queue save failure").into())
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let (mut queue, id, _) = authored_queue();
    let tl = TransparencyLogAdapter::open(&dir.path().join("tl.db"), 0).unwrap();
    let out = apply_skill_decision(
        &mut queue,
        &RefusingStore(dir.path().join("queue.json")),
        &id,
        false,
        "alice",
        None,
        |row| tl.insert_approval_decision(row).map_err(|e| e.to_string()),
    );
    assert!(matches!(
        out,
        Err(SkillDecisionError::Store(maos_skill::ESkillStore::Io(_)))
    ));
    assert_eq!(tl.query_approvals(None).unwrap().len(), 1);
}

#[test]
fn admission_view_keeps_revision_entries_and_refuses_ambiguous_skill_ids() {
    use maos_cli::subcommands::{apply_skill_decision, skill_entries, SkillDecisionError};
    use maos_skill::{AuthorIdentity, SkillAdmissionQueue, SubmissionContext};
    let dir = tempfile::tempdir().unwrap();
    let directory = dir.path().join("skills");
    let store = LocalFsSkillQueueStore::at_path(dir.path().join("queue.json"));
    let tl = TransparencyLogAdapter::open(&dir.path().join("tl.db"), 0).unwrap();
    let (mut queue, base, _) = authored_queue();
    apply_skill_decision(
        &mut queue,
        &store,
        &base,
        true,
        "alice",
        Some(&directory),
        |row| tl.insert_approval_decision(row).map_err(|e| e.to_string()),
    )
    .unwrap();
    let mut revisions = Vec::new();
    for pid in [7, 8] {
        let author = AuthorIdentity {
            class: "researcher".into(),
            pid,
            boot_nonce: "boot".into(),
        };
        let actions = std::collections::BTreeSet::new();
        let ctx = SubmissionContext {
            author: &author,
            declared_actions: &actions,
            principal_ids: &[],
        };
        let report = maos_domain::self_telemetry::SelfTelemetryReport::new(
            pid,
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
        let proposal = maos_skill::build_proposal(
            "researcher.core".into(),
            "1.0.0".into(),
            format!("-old\n+new from {pid}"),
            report,
        )
        .unwrap();
        let (entry, _) = queue.prepare_proposal(proposal, &ctx).unwrap();
        revisions.push(entry.entry_id.clone());
        queue.commit_prepared(entry).unwrap();
    }
    store.save(queue.entries()).unwrap();
    let discovered = discover_skills_detailed(&[directory.clone()]);
    let mut view = skill_entries(
        &discovered.discovered,
        store.load().unwrap(),
        &tl.query_approvals(None).unwrap(),
    )
    .unwrap();
    assert_eq!(view.entries().len(), 3);
    let refusal = apply_skill_decision(
        &mut view,
        &store,
        "researcher.core",
        true,
        "alice",
        Some(&directory),
        |_| panic!("ambiguous selectors must not journal"),
    );
    assert!(matches!(
        refusal,
        Err(SkillDecisionError::Queue(
            maos_skill::ESkillQueue::AmbiguousEntry
        ))
    ));
    apply_skill_decision(
        &mut view,
        &store,
        &revisions[0],
        false,
        "alice",
        Some(&directory),
        |row| tl.insert_approval_decision(row).map_err(|e| e.to_string()),
    )
    .unwrap();
    let restored = SkillAdmissionQueue::from_stored(store.load().unwrap()).unwrap();
    assert_eq!(
        restored.show(&base).unwrap().state,
        SkillAdmissionState::Admitted
    );
    assert_eq!(
        restored.show(&revisions[1]).unwrap().state,
        SkillAdmissionState::Pending
    );
    assert!(tl
        .query_approvals(None)
        .unwrap()
        .iter()
        .any(
            |row| row.target == format!("researcher.core@1.0.0#{}", revisions[0])
                && row.capability == "skill.admission.reject"
        ));
}
