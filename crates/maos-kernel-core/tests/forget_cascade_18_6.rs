#![forbid(unsafe_code)]

use maos_domain::distillation::{DigestPayload, DistillationRequest};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::memory::{ForgetOutcome, MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::{DistillationPort, MemoryManagerPort};
use maos_iac::adapter::distillate::DistillateWriter;
use maos_iac::adapter::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use std::sync::Arc;

struct Scene {
    dir: tempfile::TempDir,
    log: Arc<TransparencyLogAdapter>,
    memory: MemoryManagerAdapter,
}
impl Scene {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("audit.sqlite");
        let log =
            Arc::new(TransparencyLogAdapter::open_with_global_legal_holds(&db, &db, 42).unwrap());
        let memory = MemoryManagerAdapter::new(
            Arc::new(PrivateMemoryStore::new(dir.path().join("private"), 4096)),
            Arc::new(SharedMemoryStore::open(&db).unwrap()),
            Arc::new(PrincipalNamespaceIndex::open(&db).unwrap()),
            log.clone(),
        );
        Self { dir, log, memory }
    }
    fn raw(&self, kind: FrameKind, pid: u32, body: &str) -> [u8; 16] {
        self.log.insert_frame_event(
            kind,
            pid,
            None,
            "forget.test",
            body.as_bytes(),
            FrameOrigin::Kernel,
        );
        self.log.last_frame_id()
    }
    fn digest(&self, pid: u32, refs: Vec<[u8; 16]>, body: &str) -> [u8; 16] {
        DistillateWriter::new(self.log.clone(), Arc::new(()))
            .write_distillate(
                pid,
                DistillationRequest::new(refs, 1, DigestPayload::Text(body.into()), None).unwrap(),
            )
            .unwrap()
            .digest_frame_id
    }
    fn body(&self, id: [u8; 16]) -> serde_json::Value {
        serde_json::from_slice(
            &self
                .log
                .query_frame_by_id(id)
                .unwrap()
                .unwrap()
                .payload_redacted,
        )
        .unwrap()
    }
    fn events(&self, intent: &str) -> usize {
        self.log
            .query_frames(FrameFilter {
                intent: Some(intent.into()),
                ..Default::default()
            })
            .unwrap()
            .len()
    }
}

#[test]
fn any_kind_any_unindexed_writer_is_scrubbed_with_nonpersonal_markers() {
    let s = Scene::new();
    let ids: Vec<_> = [
        FrameKind::McpInvocation,
        FrameKind::TaskComplete,
        FrameKind::InferenceCall,
    ]
    .into_iter()
    .map(|kind| s.raw(kind, 999, "director-ada CANARY-18-6"))
    .collect();
    let control = s.raw(FrameKind::McpInvocation, 1000, "director-grace CONTROL");
    let before = s.log.query_frame_by_id(control).unwrap().unwrap();
    let ForgetOutcome::Erased {
        redacted_principal_frame_ids,
        redistill,
        receipt,
        ..
    } = s.memory.forget_with_reason("director-ada", None).unwrap()
    else {
        panic!("held")
    };
    assert_eq!(
        receipt.deleted_index_rows, 0,
        "writer is deliberately unindexed"
    );
    assert_eq!(redacted_principal_frame_ids.len(), 3);
    assert!(redistill.is_empty());
    for id in ids {
        assert_eq!(s.body(id)["redacted"], true);
    }
    assert_eq!(s.events("frame.redacted"), 3);
    assert_eq!(s.log.query_frame_by_id(control).unwrap().unwrap(), before);
    for row in s
        .log
        .query_frames(FrameFilter {
            intent: Some("frame.redacted".into()),
            ..Default::default()
        })
        .unwrap()
    {
        let text = String::from_utf8(row.payload_redacted).unwrap();
        assert!(!text.contains("director-ada") && !text.contains("CANARY-18-6"));
    }
}

#[test]
fn digest_of_digest_closes_for_unindexed_writer_and_captures_only_surviving_sources() {
    let s = Scene::new();
    let raw = s.raw(FrameKind::McpInvocation, 7, "director-ada CANARY-18-6");
    let clean = s.raw(FrameKind::TaskComplete, 7, "surviving context");
    let first = s.digest(7, vec![raw, clean], "director-ada CANARY-18-6");
    let second = s.digest(7, vec![first], "neutral digest of digest");
    let ForgetOutcome::Erased {
        redistill,
        redacted_distillate_frame_ids,
        ..
    } = s.memory.forget_with_reason("director-ada", None).unwrap()
    else {
        panic!("held")
    };
    assert_eq!(redacted_distillate_frame_ids.len(), 2);
    assert_eq!(redistill.len(), 2);
    for (id, pid) in [(first, 7), (second, 7)] {
        assert_eq!(s.body(id)["original_kind"], "Distillate");
        let request = redistill
            .iter()
            .find(|r| r.redacted_distillate_frame_id == hex::encode(id))
            .unwrap();
        assert_eq!(request.writer_spirit_pid, pid);
        assert_eq!(request.writer_boot_nonce, 42_u128.to_le_bytes());
        assert_eq!(request.surviving_source_log_ref, vec![hex::encode(clean)]);
    }
    assert_eq!(s.events("distillate.redacted"), 2);
    assert_eq!(s.events("distillate.redistill.requested"), 2);
}

#[test]
fn historical_persisted_cross_writer_distillate_is_closed_without_new_authoring_permission() {
    let s = Scene::new();
    let raw = s.raw(FrameKind::McpInvocation, 7, "director-ada CANARY");
    let first = s.digest(7, vec![raw], "director-ada CANARY");
    let historical = s.digest(7, vec![first], "neutral historical digest");
    // Real writer/adapter generate the complete persisted receipt. Model an
    // older log's cross-writer attribution explicitly in storage: current I11
    // correctly forbids newly authoring this citation at pid 8.
    let conn = rusqlite::Connection::open(s.dir.path().join("audit.sqlite")).unwrap();
    conn.execute(
        "UPDATE transparency_log SET spirit_pid = 8 WHERE frame_id = ?1",
        rusqlite::params![&historical[..]],
    )
    .unwrap();
    let ForgetOutcome::Erased { redistill, .. } =
        s.memory.forget_with_reason("director-ada", None).unwrap()
    else {
        panic!("held")
    };
    assert_eq!(s.body(historical)["redacted"], true);
    let request = redistill
        .iter()
        .find(|r| r.redacted_distillate_frame_id == hex::encode(historical))
        .unwrap();
    assert_eq!(request.writer_spirit_pid, 8);
    assert!(request.surviving_source_log_ref.is_empty());
}

#[test]
fn cost_scrub_keeps_figures_and_removes_nested_principal() {
    let s = Scene::new();
    let cost = s.raw(FrameKind::CostAttribution, 999, r#"{"schema_version":1,"principal":{"kind":"Resolved","principal_id":"a@b.org"},"dimensions":{"tokens_in":42,"tokens_out":7},"model":"fixture"}"#);
    s.memory.forget_with_reason("a@b.org", None).unwrap();
    let body = s.body(cost);
    assert_eq!(body["dimensions"]["tokens_in"], 42);
    assert_eq!(body["dimensions"]["tokens_out"], 7);
    assert_eq!(body["principal"]["principal_id"], "[REDACTED]");
    assert_eq!(body["redacted"], true);
    assert!(!body.to_string().contains("a@b.org"));
}

#[test]
fn token_boundary_prevents_overerasure_and_all_scrubbed_sources_stay_empty() {
    let s = Scene::new();
    let target = s.raw(FrameKind::McpInvocation, 7, "a@b.org CANARY");
    let control = s.raw(FrameKind::McpInvocation, 7, "aa@b.org CONTROL");
    let control_before = s.log.query_frame_by_id(control).unwrap().unwrap();
    let digest = s.digest(7, vec![target], "a@b.org CANARY");
    let control_digest = s.digest(7, vec![control], "aa@b.org CONTROL");
    let before = s.log.query_frame_by_id(control_digest).unwrap().unwrap();
    let ForgetOutcome::Erased { redistill, .. } =
        s.memory.forget_with_reason("a@b.org", None).unwrap()
    else {
        panic!("held")
    };
    assert_eq!(redistill.len(), 1);
    assert_eq!(
        redistill[0].redacted_distillate_frame_id,
        hex::encode(digest)
    );
    assert!(redistill[0].surviving_source_log_ref.is_empty());
    assert_eq!(
        s.log.query_frame_by_id(control).unwrap().unwrap(),
        control_before
    );
    assert_eq!(
        s.log.query_frame_by_id(control_digest).unwrap().unwrap(),
        before
    );
}

#[test]
fn rerun_after_scrub_failure_closes_from_an_already_scrubbed_source() {
    let s = Scene::new();
    s.memory
        .write(
            7,
            MemoryTier::Private,
            &MemoryNamespace::principal("director-ada", "fixture").unwrap(),
            "k",
            MemoryValue::Text("CANARY".into()),
        )
        .unwrap();
    let raw = s.raw(FrameKind::McpInvocation, 999, "director-ada CANARY");
    let digest = s.digest(999, vec![raw], "neutral derived body");
    let conn = rusqlite::Connection::open(s.dir.path().join("audit.sqlite")).unwrap();
    conn.execute_batch(&format!("CREATE TRIGGER fail_scrub BEFORE UPDATE OF payload_redacted ON transparency_log WHEN OLD.frame_id = X'{}' BEGIN SELECT RAISE(FAIL, 'injected scrub failure'); END;", hex::encode(digest))).unwrap();
    assert!(s.memory.forget_with_reason("director-ada", None).is_err());
    assert_eq!(s.body(raw)["redacted"], true);
    assert!(!s.body(digest).to_string().contains("redacted"));
    conn.execute_batch("DROP TRIGGER fail_scrub;").unwrap();
    let remaining = s.log.distillates_citing(&Default::default()).unwrap();
    assert!(
        remaining.iter().any(|frame| frame.frame_id == digest),
        "retry closure must recover the digest from its persisted source tombstone"
    );
    let ForgetOutcome::Erased {
        receipt, redistill, ..
    } = s.memory.forget_with_reason("director-ada", None).unwrap()
    else {
        panic!("held")
    };
    assert_eq!(receipt.deleted_index_rows, 1);
    assert_eq!(s.body(digest)["redacted"], true);
    assert_eq!(redistill.len(), 1);
    assert!(redistill[0].surviving_source_log_ref.is_empty());
}

#[test]
fn legal_hold_preserves_every_body_and_journals_no_request() {
    let s = Scene::new();
    let raw = s.raw(FrameKind::McpInvocation, 999, "director-ada CANARY");
    let digest = s.digest(999, vec![raw], "director-ada CANARY");
    let before = s.log.query_frame_by_id(digest).unwrap().unwrap();
    assert!(matches!(
        s.memory
            .forget_with_reason("director-ada", Some("legal-hold:case")),
        Ok(ForgetOutcome::Suspended { .. })
    ));
    assert_eq!(s.log.query_frame_by_id(digest).unwrap().unwrap(), before);
    assert_eq!(s.events("distillate.redistill.requested"), 0);
    assert_eq!(s.events("distillate.redacted"), 0);
    assert_eq!(s.events("frame.redacted"), 0);
}

#[test]
fn clean_raw_ancestry_does_not_hide_a_personal_intermediate_digest() {
    let s = Scene::new();
    let raw = s.raw(FrameKind::TaskComplete, 7, "clean raw");
    let first = s.digest(7, vec![raw], "director-ada private conclusion");
    let second = s.digest(7, vec![first], "neutral conclusion");
    let third = s.digest(7, vec![second], "another neutral conclusion");
    s.memory.forget_with_reason("director-ada", None).unwrap();
    assert_eq!(
        s.log
            .query_frame_by_id(raw)
            .unwrap()
            .unwrap()
            .payload_redacted,
        b"clean raw"
    );
    for id in [first, second, third] {
        assert_eq!(s.body(id)["redacted"], true);
    }
    assert!(DistillateWriter::new(s.log.clone(), Arc::new(()))
        .write_distillate(
            7,
            DistillationRequest::new(vec![first], 1, DigestPayload::Text("revival".into()), None)
                .unwrap()
        )
        .is_err());
}

#[test]
fn held_coprincipal_protects_frames_and_neutral_derivatives() {
    let s = Scene::new();
    s.log
        .place_legal_hold("director-grace", "legal-hold", None, 1)
        .unwrap();
    let raw = s.raw(
        FrameKind::TaskComplete,
        7,
        "director-ada and director-grace",
    );
    let digest = s.digest(7, vec![raw], "neutral derivative");
    let ForgetOutcome::Erased { receipt, .. } =
        s.memory.forget_with_reason("director-ada", None).unwrap()
    else {
        panic!("unexpected hold");
    };
    assert_eq!(receipt.held_frame_ids.len(), 2);
    assert_ne!(s.body(digest)["redacted"], true);
    assert_eq!(
        s.log
            .query_frame_by_id(raw)
            .unwrap()
            .unwrap()
            .payload_redacted,
        b"director-ada and director-grace"
    );
}

#[test]
fn partial_redaction_is_not_a_tombstone_and_uninstall_receipts_are_exempt() {
    let s = Scene::new();
    let partial = s.raw(
        FrameKind::TaskComplete,
        7,
        r#"{"redacted":true,"redaction_reason":"partial","principal":"director-ada"}"#,
    );
    let digest = s.digest(7, vec![partial], "neutral");
    s.log.insert_frame_event(
        FrameKind::TaskComplete,
        7,
        None,
        "principal.uninstall.completed",
        b"director-ada",
        FrameOrigin::Kernel,
    );
    let uninstall = s.log.last_frame_id();
    s.memory.forget_with_reason("director-ada", None).unwrap();
    assert_eq!(s.body(digest)["redacted"], true);
    assert_eq!(
        s.log
            .query_frame_by_id(uninstall)
            .unwrap()
            .unwrap()
            .payload_redacted,
        b"director-ada"
    );
    assert!(s.memory.forget_with_reason("", Some("legal-hold")).is_err());
    assert!(!s.log.is_under_legal_hold("").unwrap());
}

#[test]
fn malformed_distillates_are_scrubbed_or_reported_without_aborting_and_requests_recover() {
    let s = Scene::new();
    let raw = s.raw(FrameKind::TaskComplete, 7, "clean");
    let matched = s.digest(7, vec![raw], "director-ada");
    let gap = s.digest(7, vec![raw], "neutral");
    let conn = rusqlite::Connection::open(s.dir.path().join("audit.sqlite")).unwrap();
    for (id, body) in [
        (matched, "director-ada malformed"),
        (gap, "malformed neutral"),
    ] {
        conn.execute(
            "UPDATE transparency_log SET payload_redacted = ?1 WHERE frame_id = ?2",
            rusqlite::params![body.as_bytes(), &id[..]],
        )
        .unwrap();
    }
    let ForgetOutcome::Erased {
        receipt, redistill, ..
    } = s.memory.forget_with_reason("director-ada", None).unwrap()
    else {
        panic!("unexpected hold");
    };
    assert_eq!(s.body(matched)["redacted"], true);
    assert_eq!(
        receipt.malformed_distillate_frame_ids,
        vec![hex::encode(gap)]
    );
    assert_eq!(s.log.outstanding_redistill_requests().unwrap(), redistill);
    s.log
        .insert_redistill_outcome(
            &redistill[0],
            &maos_domain::memory::RedistillOutcome::Pending {
                reason: maos_domain::memory::RedistillPendingReason::WriterNotLive,
            },
        )
        .unwrap();
    assert!(s.log.outstanding_redistill_requests().unwrap().is_empty());
}

#[test]
fn an_erased_raw_source_cannot_be_cited_by_a_late_writer() {
    let s = Scene::new();
    let raw = s.raw(FrameKind::McpInvocation, 7, "director-ada private source");
    s.memory.forget_with_reason("director-ada", None).unwrap();
    let result = DistillateWriter::new(s.log.clone(), Arc::new(())).write_distillate(
        7,
        DistillationRequest::new(
            vec![raw],
            1,
            DigestPayload::Text("late revival".into()),
            None,
        )
        .unwrap(),
    );
    assert!(
        result.is_err(),
        "a raw tombstone is not an authoritative source"
    );
}

#[test]
fn an_unrelated_forget_does_not_treat_partial_redaction_as_prior_erasure() {
    let s = Scene::new();
    let partial = s.raw(
        FrameKind::CostAttribution,
        7,
        r#"{"redacted":true,"redaction_reason":"gdpr-forget","tokens":42}"#,
    );
    let digest = s.digest(7, vec![partial], "neutral numerical summary");
    let ForgetOutcome::Erased {
        redacted_distillate_frame_ids,
        ..
    } = s
        .memory
        .forget_with_reason("unrelated-principal", None)
        .unwrap()
    else {
        panic!("unexpected hold");
    };
    assert!(
        redacted_distillate_frame_ids.is_empty(),
        "partial cost redactions stay citable"
    );
    assert_ne!(s.body(digest)["redacted"], true);
    assert_eq!(s.body(partial)["tokens"], 42);
}
