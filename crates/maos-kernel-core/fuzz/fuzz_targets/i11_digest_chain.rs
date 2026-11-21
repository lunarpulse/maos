#![no_main]

//! I11 fuzz leg: arbitrary digest requests against the real `DistillateWriter`. A persisted
//! digest must have a non-empty, raw-only `source_log_ref` and depth >= 1; a request without them
//! must be refused `AuditChainMissing`.

use libfuzzer_sys::fuzz_target;
use maos_domain::distillation::{DigestPayload, DistillationError, DistillationRequest};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::DistillationPort;
use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use std::sync::Arc;

fuzz_target!(|data: &[u8]| {
    let Some((&depth, rest)) = data.split_first() else { return };
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("audit.db");
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(1));
    let memory = Arc::new(MemoryManagerAdapter::new(
        Arc::new(PrivateMemoryStore::new(tmp.path().join("memory"), 4096)),
        Arc::new(SharedMemoryStore::open(&db).unwrap()),
        Arc::new(PrincipalNamespaceIndex::open(&db).unwrap()),
        Arc::clone(&tl),
    ));
    let writer = DistillateWriter::new(Arc::clone(&tl), memory);
    tl.insert_frame_event(FrameKind::TaskAssign, 1, None, "consult", b"raw", FrameOrigin::HumanAuthored);
    let raw = tl.last_frame_id();
    let digest = DistillationRequest::new(vec![raw], 1, DigestPayload::Text("d".into()), None)
        .ok()
        .and_then(|r| writer.write_distillate(1, r).ok())
        .map(|r| r.digest_frame_id);
    let pool = [raw, digest.unwrap_or([0xAA; 16]), [0xDE; 16]];
    let sources: Vec<[u8; 16]> = rest.iter().take(4).map(|b| pool[*b as usize % 3]).collect();
    let depth = u32::from(depth % 4);
    let request = DistillationRequest {
        source_log_ref: sources.clone(),
        distillation_depth: depth,
        digest_payload: DigestPayload::Text("x".into()),
        segment_hint: None,
    };
    match writer.write_distillate(1, request) {
        Ok(receipt) => {
            assert!(!sources.is_empty() && depth >= 1, "chainless digest persisted");
            assert!(!receipt.effective_source_log_ref.is_empty());
            for id in &receipt.effective_source_log_ref {
                let row = tl.query_frame_by_id(*id).unwrap().expect("source row");
                assert!(row.kind != FrameKind::Distillate, "source_log_ref not flattened");
            }
        }
        Err(err) => {
            if sources.is_empty() || depth == 0 {
                assert!(matches!(err, DistillationError::AuditChainMissing { .. }));
            }
        }
    }
});
