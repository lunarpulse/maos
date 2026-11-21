//! Story 21-12 AC3 — an interrupted forget converges by forward completion from a
//! durable `principal.forget.intent` frame. Convergence, not atomicity.

#![forbid(unsafe_code)]
#![cfg(debug_assertions)]

use std::path::PathBuf;
use std::sync::Arc;

use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::memory::{ForgetOutcome, MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use maos_iac::adapter::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::memory::spill_test_faults::{arm_failure, disarm, FailurePoint};
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use tempfile::TempDir;

const PRINCIPAL: &str = "interrupted-forget@example.org";
const BYSTANDER: &str = "forget-bystander@example.org";

struct Fixture {
    _dir: TempDir,
    fs_root: PathBuf,
    db_path: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().expect("create fixture directory");
        let fs_root = dir.path().join("memory");
        std::fs::create_dir_all(&fs_root).expect("create memory root");
        let db_path = dir.path().join("audit.sqlite");
        Self {
            _dir: dir,
            fs_root,
            db_path,
        }
    }

    /// (Re)open every store on the same directories and audit DB: a restart.
    fn open(&self) -> (MemoryManagerAdapter, Arc<TransparencyLogAdapter>) {
        let log = Arc::new(
            TransparencyLogAdapter::open_with_global_legal_holds(&self.db_path, &self.db_path, 1)
                .expect("open transparency log"),
        );
        let memory = MemoryManagerAdapter::new(
            Arc::new(PrivateMemoryStore::new(self.fs_root.clone(), 4 * 1024)),
            Arc::new(SharedMemoryStore::open(&self.db_path).expect("open shared store")),
            Arc::new(PrincipalNamespaceIndex::open(&self.db_path).expect("open index")),
            Arc::clone(&log),
        );
        (memory, log)
    }

    fn interrupted(&self) {
        self.interrupt(PRINCIPAL, None);
    }

    /// Seed `target` and a bystander, then interrupt a forget of `target` right after
    /// the private-tier delete (before the index delete and the outcome frame).
    fn interrupt(&self, target: &str, reason: Option<&str>) {
        let (memory, log) = self.open();
        for (pid, who, keys) in [(7, target, ["a", "b"]), (8, BYSTANDER, ["c", "d"])] {
            for key in keys {
                memory
                    .write(
                        pid,
                        MemoryTier::Private,
                        &namespace(who),
                        key,
                        MemoryValue::Text(format!("value-{key}")),
                    )
                    .expect("seed private value");
            }
        }
        arm_failure(FailurePoint::ForgetAfterPrivateDelete);
        let failed = memory.forget_with_reason(target, reason);
        disarm();
        assert!(
            failed.is_err(),
            "the injected failure interrupts the forget"
        );
        assert_eq!(frames(&log, "principal.forget.intent").len(), 1);
        assert!(frames(&log, "principal.forget").is_empty());
        assert_eq!(
            memory.subject_access(target).unwrap().len(),
            2,
            "index rows linger"
        );
    }
}

fn namespace(principal_id: &str) -> MemoryNamespace {
    MemoryNamespace::Principal {
        principal_id: principal_id.into(),
        schema: "profile".into(),
    }
}

fn frames(log: &TransparencyLogAdapter, intent: &str) -> Vec<([u8; 16], serde_json::Value)> {
    log.query_frames(FrameFilter {
        intent: Some(intent.into()),
        ..Default::default()
    })
    .expect("query frames")
    .into_iter()
    .map(|f| {
        (
            f.frame_id,
            serde_json::from_slice(&f.payload_redacted).expect("json"),
        )
    })
    .collect()
}

#[test]
fn forget_interrupted_after_private_delete_completes_after_restart() {
    let fixture = Fixture::new();
    fixture.interrupted();

    let (memory, log) = fixture.open();
    assert_eq!(memory.complete_outstanding_forgets().unwrap(), (1, vec![]));
    assert!(memory.subject_access(PRINCIPAL).unwrap().is_empty());
    let intent = frames(&log, "principal.forget.intent")[0].0;
    let done = frames(&log, "principal.forget");
    assert_eq!(
        done.len(),
        1,
        "exactly one outcome frame references the intent"
    );
    let grouped = TransparencyLogAdapter::grouped_hex(&intent);
    assert_eq!(done[0].1["completed_from_intent"], grouped);
    assert!(log.outstanding_forget_intents(None).unwrap().is_empty());
}

#[test]
fn completion_is_idempotent_across_a_second_restart() {
    let fixture = Fixture::new();
    fixture.interrupted();
    {
        let (memory, _log) = fixture.open();
        assert_eq!(memory.complete_outstanding_forgets().unwrap().0, 1);
    }
    let (memory, log) = fixture.open();
    assert_eq!(memory.complete_outstanding_forgets().unwrap(), (0, vec![]));
    assert_eq!(
        frames(&log, "principal.forget").len(),
        1,
        "a second restart appends nothing"
    );
}

#[test]
fn completion_frame_counts_only_what_it_deleted_and_names_its_intent() {
    let fixture = Fixture::new();
    fixture.interrupted();
    let (memory, log) = fixture.open();
    memory.complete_outstanding_forgets().unwrap();
    let done = &frames(&log, "principal.forget")[0].1;
    assert_eq!(done["principal_id"], PRINCIPAL);
    assert_eq!(
        done["deleted_entries"], 0,
        "the failed attempt had already deleted them"
    );
    assert_eq!(
        done["deleted_index_rows"], 2,
        "only the index rows this run deleted"
    );
    assert!(done["completed_from_intent"].is_string());
}

#[test]
fn intent_frame_survives_the_principal_scrub() {
    let fixture = Fixture::new();
    fixture.interrupted();
    let (memory, log) = fixture.open();
    memory.complete_outstanding_forgets().unwrap();
    let intent = &frames(&log, "principal.forget.intent");
    assert_eq!(intent.len(), 1);
    assert_eq!(
        intent[0].1["principal_hex"],
        TransparencyLogAdapter::grouped_hex(PRINCIPAL.as_bytes()),
        "not tombstoned by the scrub"
    );
}

#[test]
fn suspended_forget_writes_no_intent() {
    let fixture = Fixture::new();
    let (memory, log) = fixture.open();
    let held = memory
        .forget_with_reason(PRINCIPAL, Some("legal-hold:case-21-12"))
        .unwrap();
    assert!(matches!(held, ForgetOutcome::Suspended { .. }));
    let prior = memory.forget_with_reason(PRINCIPAL, None).unwrap();
    assert!(matches!(prior, ForgetOutcome::Suspended { .. }));
    assert!(frames(&log, "principal.forget.intent").is_empty());
}

#[test]
fn pending_scrub_leaves_the_intent_outstanding_and_reported() {
    let fixture = Fixture::new();
    fixture.interrupted();
    let (memory, log) = fixture.open();
    log.place_legal_hold(PRINCIPAL, "legal-hold", None, 1)
        .expect("place hold");
    let (done, pending) = memory.complete_outstanding_forgets().unwrap();
    assert_eq!(
        (done, pending),
        (0, vec![format!("{PRINCIPAL}: erasure pending")])
    );
    assert_eq!(log.outstanding_forget_intents(None).unwrap().len(), 1);
    assert!(frames(&log, "principal.forget").is_empty());
    assert_eq!(
        memory.subject_access(PRINCIPAL).unwrap().len(),
        2,
        "nothing was deleted"
    );

    assert!(log.release_legal_hold(PRINCIPAL).unwrap());
    assert_eq!(memory.complete_outstanding_forgets().unwrap(), (1, vec![]));
}

#[test]
fn hold_after_interrupted_forget_follows_the_ruled_order() {
    let fixture = Fixture::new();
    fixture.interrupted();
    let (memory, log) = fixture.open();
    let ForgetOutcome::Suspended { hold } = memory
        .forget_with_reason(PRINCIPAL, Some("legal-hold:case-21-12"))
        .unwrap()
    else {
        panic!("a legal-hold request suspends");
    };
    assert_eq!(
        hold.status, "PRINCIPAL NOT PRESENT — NOTHING TO ERASE; HOLD RECORDED",
        "the earlier erasure completed before the hold was recorded"
    );
    assert_eq!(frames(&log, "principal.forget").len(), 1);
    assert!(memory.subject_access(PRINCIPAL).unwrap().is_empty());
    assert!(
        log.is_under_legal_hold(PRINCIPAL).unwrap(),
        "the hold is still recorded"
    );
}

#[test]
fn bystander_principal_is_untouched_by_completion() {
    let fixture = Fixture::new();
    fixture.interrupted();
    let (memory, log) = fixture.open();
    log.insert_frame_event(
        FrameKind::TaskComplete,
        9,
        None,
        "bystander.test",
        BYSTANDER.as_bytes(),
        FrameOrigin::Kernel,
    );
    let frame = log.last_frame_id();
    let rows = memory.subject_access(BYSTANDER).unwrap();
    memory.complete_outstanding_forgets().unwrap();
    assert_eq!(memory.subject_access(BYSTANDER).unwrap(), rows);
    assert_eq!(rows.len(), 2);
    let value = memory
        .read(8, MemoryTier::Private, &namespace(BYSTANDER), "c")
        .unwrap();
    assert_eq!(value, Some(MemoryValue::Text("value-c".into())));
    let kept = log.query_frame_by_id(frame).unwrap().unwrap();
    assert_eq!(kept.payload_redacted, BYSTANDER.as_bytes());
}

const TOKEN_SHAPED: &str = "user-0123456789abcdef0123456789abcdef@example.org";

#[test]
fn token_shaped_principal_id_recovers_its_real_subject() {
    let fixture = Fixture::new();
    fixture.interrupt(TOKEN_SHAPED, None);
    let (memory, log) = fixture.open();
    let intent = frames(&log, "principal.forget.intent");
    assert_eq!(
        intent[0].1["principal_hex"],
        TransparencyLogAdapter::grouped_hex(TOKEN_SHAPED.as_bytes()),
        "the recovery key survives journal redaction"
    );
    assert_eq!(memory.complete_outstanding_forgets().unwrap(), (1, vec![]));
    assert!(memory.subject_access(TOKEN_SHAPED).unwrap().is_empty());
    assert_eq!(memory.subject_access(BYSTANDER).unwrap().len(), 2);
}

#[test]
fn hold_is_not_recorded_while_the_earlier_erasure_is_unfinished() {
    let fixture = Fixture::new();
    fixture.interrupted();
    let (memory, log) = fixture.open();
    arm_failure(FailurePoint::ForgetAfterPrivateDelete);
    let refused = memory.forget_with_reason(PRINCIPAL, Some("legal-hold:case-21-12"));
    disarm();
    let message = refused
        .expect_err("an unfinished erasure refuses the hold")
        .to_string();
    assert!(
        message.contains("hold not recorded") && message.contains(PRINCIPAL),
        "{message}"
    );
    assert!(message.contains("injected"), "the cause is kept: {message}");
    assert!(!log.is_under_legal_hold(PRINCIPAL).unwrap());
    assert!(frames(&log, "principal.forget.held").is_empty());
    assert_eq!(
        log.outstanding_forget_intents(Some(PRINCIPAL))
            .unwrap()
            .len(),
        1
    );

    let again = memory.forget_with_reason(PRINCIPAL, Some("legal-hold:case-21-12"));
    assert!(matches!(again, Ok(ForgetOutcome::Suspended { .. })));
    assert!(log.is_under_legal_hold(PRINCIPAL).unwrap());
}

#[test]
fn a_malformed_intent_does_not_block_valid_recovery() {
    let fixture = Fixture::new();
    fixture.interrupted();
    let (memory, log) = fixture.open();
    for body in [&b"not json"[..], &br#"{"principal_hex": 5}"#[..]] {
        log.insert_kernel_event_returning_id(
            0,
            FrameKind::Decision,
            None,
            "principal.forget.intent",
            body,
        );
    }
    let (done, unfinished) = memory.complete_outstanding_forgets().unwrap();
    assert_eq!(done, 1);
    assert_eq!(unfinished.len(), 2);
    assert!(
        unfinished
            .iter()
            .all(|u| u.starts_with("malformed intent ")),
        "{unfinished:?}"
    );
    assert!(memory.subject_access(PRINCIPAL).unwrap().is_empty());
    let other = memory.forget_with_reason(BYSTANDER, None).unwrap();
    assert!(matches!(other, ForgetOutcome::Erased { .. }));
}

#[test]
fn a_live_retry_reuses_its_open_intent_and_keeps_its_reason() {
    let fixture = Fixture::new();
    fixture.interrupt(PRINCIPAL, Some("gdpr-request-21-12"));
    let (memory, log) = fixture.open();
    let ForgetOutcome::Erased { receipt, .. } = memory.forget_with_reason(PRINCIPAL, None).unwrap()
    else {
        panic!("the retry erases");
    };
    assert_eq!(receipt.deleted_index_rows, 2);
    let intents = frames(&log, "principal.forget.intent");
    assert_eq!(intents.len(), 1, "no second intent is appended");
    let done = frames(&log, "principal.forget");
    assert_eq!(done.len(), 1);
    assert_eq!(
        done[0].1["completed_from_intent"],
        TransparencyLogAdapter::grouped_hex(&intents[0].0)
    );
    assert_eq!(
        done[0].1["reason"], "gdpr-request-21-12",
        "the original reason is kept"
    );
    assert!(log.outstanding_forget_intents(None).unwrap().is_empty());
}
