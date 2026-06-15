//! Story 9.2 — GDPR Article 17 cascade integration tests.
//!
//! Uses the real `MemoryManagerAdapter` + `TransparencyLogAdapter` against an
//! isolated temp directory so the tests never touch the developer's home store.

#![forbid(unsafe_code)]

use std::sync::Arc;

use maos_domain::distillation::{DigestPayload, DistillationRequest};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::memory::{ForgetOutcome, MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::{DistillationPort, MemoryManagerPort};
use tempfile::TempDir;

fn open_isolated_stores(
    dir: &TempDir,
) -> (
    Arc<maos_kernel_core::memory::PrivateMemoryStore>,
    Arc<maos_kernel_core::memory::SharedMemoryStore>,
    Arc<maos_kernel_core::memory::PrincipalNamespaceIndex>,
    Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
    std::path::PathBuf,
) {
    let fs_root = dir.path().join("memory");
    std::fs::create_dir_all(&fs_root).unwrap();
    let db_path = dir.path().join("audit.sqlite");

    let private = Arc::new(maos_kernel_core::memory::PrivateMemoryStore::new(
        fs_root,
        4 * 1024,
    ));
    let shared = Arc::new(maos_kernel_core::memory::SharedMemoryStore::open(&db_path).unwrap());
    let principal_index =
        Arc::new(maos_kernel_core::memory::PrincipalNamespaceIndex::open(&db_path).unwrap());
    let tl = Arc::new(
        maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open(&db_path, 1).unwrap(),
    );
    (private, shared, principal_index, tl, db_path)
}

fn make_memory_adapter(
    private: Arc<maos_kernel_core::memory::PrivateMemoryStore>,
    shared: Arc<maos_kernel_core::memory::SharedMemoryStore>,
    principal_index: Arc<maos_kernel_core::memory::PrincipalNamespaceIndex>,
    tl: Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
) -> Arc<maos_kernel_core::memory::MemoryManagerAdapter> {
    Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
        private,
        shared,
        principal_index,
        tl,
    ))
}

fn write_principal_data(
    memory: &Arc<maos_kernel_core::memory::MemoryManagerAdapter>,
    spirit_pid: u32,
    principal: &str,
    schema: &str,
    key: &str,
    value: &str,
) {
    let ns = MemoryNamespace::Principal {
        principal_id: principal.into(),
        schema: schema.into(),
    };
    memory
        .write(
            spirit_pid,
            MemoryTier::Private,
            &ns,
            key,
            MemoryValue::Text(value.into()),
        )
        .unwrap();
}

fn insert_source_frame(
    tl: &Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
    spirit_pid: u32,
    intent: &str,
    payload: &str,
) -> [u8; 16] {
    tl.insert_frame_event(
        maos_kernel_core::iac::transparency_log::FrameKind::TaskComplete,
        spirit_pid,
        None,
        intent,
        payload.as_bytes(),
        FrameOrigin::Kernel,
    );
    tl.last_frame_id()
}

fn write_distillate_with_canary(
    tl: &Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
    spirit_pid: u32,
    source_frame_id: [u8; 16],
    principal: &str,
    canary: &str,
) -> [u8; 16] {
    let writer = maos_kernel_core::iac::distillate::DistillateWriter::new(
        Arc::clone(tl),
        Arc::new(()) as Arc<dyn std::any::Any + Send + Sync>,
    );
    // Embed the principal_id in the distillate body so the forget cascade's
    // content-based filter (P3) links this distillate to its subject.
    let body = format!("{canary} principal={principal}");
    let request =
        DistillationRequest::new(vec![source_frame_id], 1, DigestPayload::Text(body), None)
            .unwrap();
    let receipt = writer.write_distillate(spirit_pid, request).unwrap();
    receipt.digest_frame_id
}

fn count_redaction_markers(
    tl: &Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
    principal: &str,
) -> usize {
    let entries = tl
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
            kind: Some(maos_kernel_core::iac::transparency_log::FrameKind::TaskComplete),
            ..Default::default()
        })
        .unwrap();
    entries
        .into_iter()
        .filter(|e| {
            e.intent == "distillate.redacted"
                && String::from_utf8_lossy(&e.payload_redacted)
                    .contains(&format!("\"principal_id\":\"{}\"", principal))
        })
        .count()
}

fn canary_survives_in_distillate_bodies(
    tl: &Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
    canary: &str,
) -> bool {
    let entries = tl
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
            kind: Some(maos_kernel_core::iac::transparency_log::FrameKind::Distillate),
            ..Default::default()
        })
        .unwrap();
    entries
        .iter()
        .any(|e| String::from_utf8_lossy(&e.payload_redacted).contains(canary))
}

#[test]
fn forget_removes_principal_and_marks_distillate() {
    let dir = TempDir::new().unwrap();
    let (private, shared, principal_index, tl, db_path) = open_isolated_stores(&dir);
    let memory = make_memory_adapter(private, shared, principal_index, tl.clone());

    let principal = "alice@example.org";
    let canary = "CANARY-alice-9-2";

    // Spirit A writes principal data and distills it.
    write_principal_data(&memory, 1, principal, "chat", "msg1", "hello alice");
    let source = insert_source_frame(&tl, 1, "task.complete", "task for alice");
    write_distillate_with_canary(&tl, 1, source, principal, canary);

    // Pre-forget: subject access finds the row.
    let before = maos_audit::subject_access_query(&db_path, principal).unwrap();
    assert_eq!(before.len(), 1);

    // Forget.
    let outcome = memory.forget_with_reason(principal, None).unwrap();
    match outcome {
        ForgetOutcome::Erased {
            redacted_distillate_frame_ids,
            ..
        } => {
            assert_eq!(redacted_distillate_frame_ids.len(), 1);
        }
        _ => panic!("expected Erased, got {:?}", outcome),
    }

    // Post-forget: subject access empty.
    let after = maos_audit::subject_access_query(&db_path, principal).unwrap();
    assert!(
        after.is_empty(),
        "principal data must be removed from queryable surface"
    );

    // Redaction marker present.
    assert_eq!(count_redaction_markers(&tl, principal), 1);

    // Canary must not survive in any distillate body bytes.
    assert!(
        !canary_survives_in_distillate_bodies(&tl, canary),
        "canary token survived body scrub"
    );
}

#[test]
fn legal_hold_blocks_erasure_and_journals_request() {
    let dir = TempDir::new().unwrap();
    let (private, shared, principal_index, tl, db_path) = open_isolated_stores(&dir);
    let memory = make_memory_adapter(private, shared, principal_index, tl.clone());

    let principal = "bob@example.org";
    write_principal_data(&memory, 2, principal, "chat", "msg1", "hello bob");

    let outcome = memory
        .forget_with_reason(principal, Some("legal-hold:case-42"))
        .unwrap();

    match outcome {
        ForgetOutcome::Suspended { hold } => {
            assert_eq!(hold.principal_id, principal);
            assert!(hold.reason.starts_with("legal-hold"));
            assert_eq!(hold.case_ref.as_deref(), Some("case-42"));
            assert!(hold.status.contains("SUSPENDED"));
        }
        _ => panic!("expected Suspended, got {:?}", outcome),
    }

    // Data retained.
    let rows = maos_audit::subject_access_query(&db_path, principal).unwrap();
    assert_eq!(rows.len(), 1, "legal-hold must retain data");

    // Request journaled.
    let entries = tl
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
            kind: Some(maos_kernel_core::iac::transparency_log::FrameKind::TaskComplete),
            ..Default::default()
        })
        .unwrap();
    assert!(entries.iter().any(|e| e.intent == "principal.forget.held"));
}

#[test]
fn forget_receipt_matches_transparency_log_frame_id() {
    let dir = TempDir::new().unwrap();
    let (private, shared, principal_index, tl, _db_path) = open_isolated_stores(&dir);
    let memory = make_memory_adapter(private, shared, principal_index, tl.clone());

    write_principal_data(&memory, 3, "carol@example.org", "chat", "msg1", "hi");
    let outcome = memory
        .forget_with_reason("carol@example.org", None)
        .unwrap();

    let receipt = match outcome {
        ForgetOutcome::Erased { receipt, .. } => receipt,
        _ => panic!("expected Erased"),
    };

    // The receipt's frame_id must equal the last frame written to the TL.
    assert_eq!(receipt.frame_id, tl.last_frame_id());
    assert_eq!(receipt.deleted_entries, 1);
}
