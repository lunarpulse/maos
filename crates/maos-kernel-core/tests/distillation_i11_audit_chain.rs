//! Integration test: DistillateWriter I11 audit chain invariants.

use std::sync::Arc;

use maos_domain::distillation::{DigestPayload, DistillationError, DistillationRequest};
use maos_domain::invariants::i13::AllowedPromotionSet;
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::ports::DistillationPort;

use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};

fn make_writer(nonce: u64) -> (DistillateWriter, Arc<TransparencyLogAdapter>, tempfile::TempDir) {
    let tmp = tempfile::TempDir::new().unwrap();
    let memory_root = tmp.path().join("memory");
    let db_path = tmp.path().join("audit.db");
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(nonce));
    let private = Arc::new(PrivateMemoryStore::new(memory_root, 4 * 1024));
    let shared = Arc::new(SharedMemoryStore::open(&db_path).unwrap());
    let principal_index = Arc::new(PrincipalNamespaceIndex::open(&db_path).unwrap());
    let memory = Arc::new(MemoryManagerAdapter::new(
        private,
        shared,
        principal_index,
        Arc::clone(&tl),
    ));
    let writer = DistillateWriter::new(Arc::clone(&tl), memory);
    (writer, tl, tmp)
}

fn insert_raw_frame(tl: &Arc<TransparencyLogAdapter>, pid: u32, intent: &str) -> [u8; 16] {
    let _token = tl.insert_frame_event(
        FrameKind::TaskAssign,
        pid,
        None,
        intent,
        format!("raw-payload-{pid}").as_bytes(),
        FrameOrigin::HumanAuthored,
    );
    tl.last_frame_id()
}

/// Format a frame_id as colon-separated hex pairs (mirrors distillate.rs).
fn format_frame_id_hex(frame_id: &[u8; 16]) -> String {
    frame_id
        .chunks(2)
        .map(|chunk| format!("{:02x}{:02x}", chunk[0], chunk[1]))
        .collect::<Vec<_>>()
        .join(":")
}

#[test]
fn write_single_hop() {
    let (writer, tl, _tmp) = make_writer(0xE411);
    let raw_id = insert_raw_frame(&tl, 1, "delegate");

    let request = DistillationRequest::new(
        vec![raw_id],
        1,
        DigestPayload::Text("digest of raw".into()),
        None,
    )
    .unwrap();

    let receipt = writer.write_distillate(1, request).unwrap();
    assert_eq!(receipt.effective_distillation_depth, 1);
    assert!(!receipt.intent_lineage.as_slice().is_empty());
    assert!(!receipt.digest_frame_id.iter().all(|b| *b == 0));
}

#[test]
fn two_hop_flattening() {
    let (writer, tl, _tmp) = make_writer(0xE412);
    let raw_id = insert_raw_frame(&tl, 1, "consult");

    // First hop: digest of raw
    let req1 = DistillationRequest::new(
        vec![raw_id],
        1,
        DigestPayload::Text("first digest".into()),
        None,
    )
    .unwrap();
    let receipt1 = writer.write_distillate(1, req1).unwrap();
    let digest_id = receipt1.digest_frame_id;

    // Second hop: digest-of-digest
    let req2 = DistillationRequest::new(
        vec![digest_id],
        2,
        DigestPayload::Text("second digest".into()),
        None,
    )
    .unwrap();
    let receipt2 = writer.write_distillate(1, req2).unwrap();

    // effective_source_log_ref should contain ONLY the original raw frame
    assert_eq!(receipt2.effective_source_log_ref.len(), 1);
    assert_eq!(receipt2.effective_source_log_ref[0], raw_id);
    assert_eq!(receipt2.effective_distillation_depth, 2);
}

#[test]
fn rejects_empty_source() {
    let (writer, _tl, _tmp) = make_writer(0xE413);

    let request = DistillationRequest {
        source_log_ref: vec![],
        distillation_depth: 1,
        digest_payload: DigestPayload::Text("test".into()),
        segment_hint: None,
    };

    let err = writer.write_distillate(1, request).unwrap_err();
    assert!(
        matches!(err, DistillationError::AuditChainMissing { reason } if reason == "empty source_log_ref")
    );
}

#[test]
fn rejects_depth_zero() {
    let (writer, _tl, _tmp) = make_writer(0xE414);

    let request = DistillationRequest {
        source_log_ref: vec![[1u8; 16]],
        distillation_depth: 0,
        digest_payload: DigestPayload::Text("test".into()),
        segment_hint: None,
    };

    let err = writer.write_distillate(1, request).unwrap_err();
    assert!(
        matches!(err, DistillationError::AuditChainMissing { reason } if reason == "distillation_depth < 1")
    );
}

#[test]
fn rejects_missing_source_frame() {
    let (writer, _tl, _tmp) = make_writer(0xE415);

    let request = DistillationRequest::new(
        vec![[0xDE; 16]], // non-existent frame
        1,
        DigestPayload::Text("test".into()),
        None,
    )
    .unwrap();

    let err = writer.write_distillate(1, request).unwrap_err();
    assert!(matches!(err, DistillationError::SourceFrameNotFound { .. }));
}

#[test]
fn admit_allows_matching_lineage() {
    let (writer, tl, _tmp) = make_writer(0xE416);
    let raw_id = insert_raw_frame(&tl, 1, "consult");

    let request = DistillationRequest::new(
        vec![raw_id],
        1,
        DigestPayload::Text("digest".into()),
        None,
    )
    .unwrap();
    let receipt = writer.write_distillate(1, request).unwrap();

    let mut allowed = AllowedPromotionSet::new();
    allowed.insert(A2AIntent::new("consult"));
    assert!(writer
        .admit_for_consumer(receipt.digest_frame_id, &allowed)
        .is_ok());
}

#[test]
fn admit_denies_non_matching_lineage() {
    let (writer, tl, _tmp) = make_writer(0xE417);
    let raw_id = insert_raw_frame(&tl, 1, "delegate");

    let request = DistillationRequest::new(
        vec![raw_id],
        1,
        DigestPayload::Text("digest".into()),
        None,
    )
    .unwrap();
    let receipt = writer.write_distillate(1, request).unwrap();

    let mut allowed = AllowedPromotionSet::new();
    allowed.insert(A2AIntent::new("consult")); // NOT delegate
    let err = writer
        .admit_for_consumer(receipt.digest_frame_id, &allowed)
        .unwrap_err();
    assert!(matches!(err, DistillationError::IntentPromotionDenied { .. }));
}

#[test]
fn cycle_detection() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("audit.db");
    let tl = Arc::new(TransparencyLogAdapter::open(&db_path, 0xE418).unwrap());

    // Insert a raw frame
    let _token = tl.insert_frame_event(
        FrameKind::TaskAssign,
        1,
        None,
        "consult",
        b"raw-payload",
        FrameOrigin::HumanAuthored,
    );
    let _raw_id = tl.last_frame_id();

    // Insert a Distillate frame with a placeholder source_log_ref
    let placeholder = [0u8; 16];
    let placeholder_hex = format_frame_id_hex(&placeholder);
    let poison_payload = serde_json::json!({
        "kind": "distillate",
        "source_log_ref": [placeholder_hex],
        "distillation_depth": 1,
        "intent_lineage": ["consult"],
        "digest_frame_id": placeholder_hex,
    });
    let _token = tl.insert_frame_event(
        FrameKind::Distillate,
        99,
        None,
        "test.poison",
        &serde_json::to_vec(&poison_payload).unwrap(),
        FrameOrigin::SpiritAuto,
    );
    let poison_id = tl.last_frame_id();

    // Update the poison row so its source_log_ref points to itself
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let self_hex = format_frame_id_hex(&poison_id);
        let new_payload = serde_json::json!({
            "kind": "distillate",
            "source_log_ref": [self_hex],
            "distillation_depth": 1,
            "intent_lineage": ["consult"],
            "digest_frame_id": self_hex,
        });
        conn.execute(
            "UPDATE transparency_log SET payload_redacted = ?1 WHERE frame_id = ?2",
            rusqlite::params![serde_json::to_vec(&new_payload).unwrap(), &poison_id[..]],
        )
        .unwrap();
    }

    // Build writer
    let memory_root = tmp.path().join("memory");
    let private = Arc::new(PrivateMemoryStore::new(memory_root, 4 * 1024));
    let shared = Arc::new(SharedMemoryStore::open(&db_path).unwrap());
    let principal_index = Arc::new(PrincipalNamespaceIndex::open(&db_path).unwrap());
    let memory = Arc::new(MemoryManagerAdapter::new(
        private,
        shared,
        principal_index,
        Arc::clone(&tl),
    ));
    let writer = DistillateWriter::new(Arc::clone(&tl), memory);

    let req = DistillationRequest::new(
        vec![poison_id],
        1,
        DigestPayload::Text("B".into()),
        None,
    )
    .unwrap();

    let err = writer.write_distillate(1, req).unwrap_err();
    assert!(
        matches!(err, DistillationError::Storage(ref s) if s.contains("cycle in distillation chain")),
        "expected cycle detection error, got: {err:?}"
    );
}
