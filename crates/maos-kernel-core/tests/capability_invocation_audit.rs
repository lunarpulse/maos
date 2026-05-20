//! Integration test: CapabilityInvocation audit rows for recall, fetch, and distillate.

use std::sync::Arc;

use maos_domain::distillation::{DigestPayload, DistillationRequest};
use maos_domain::log_recall::LogRecallFilter;
use maos_domain::ports::{DistillationPort, LogRecallPort};

use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_kernel_core::iac::log_recall::LogRecallAdapter;
use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};

fn seed_frames(tl: &Arc<TransparencyLogAdapter>, pid: u32, count: usize) {
    for i in 0..count {
        let payload = format!("payload-{pid}-{i}");
        let _token = tl.insert_frame_event(
            FrameKind::TaskAssign,
            pid,
            None,
            "delegate",
            payload.as_bytes(),
            FrameOrigin::HumanAuthored,
        );
    }
}

#[test]
fn recall_emits_capability_invocation() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xE421));
    seed_frames(&tl, 10, 3);

    let adapter = LogRecallAdapter::new(Arc::clone(&tl));
    let _page = adapter.recall(10, LogRecallFilter::default()).unwrap();

    let audit_filter = FrameFilter {
        kind: Some(FrameKind::CapabilityInvocation),
        ..Default::default()
    };
    let audit_rows = tl.query_frames(audit_filter).unwrap();
    assert_eq!(
        audit_rows.iter().filter(|r| r.intent == "log.recall").count(),
        1,
        "expected exactly one CapabilityInvocation row with intent log.recall"
    );
}

#[test]
fn fetch_emits_capability_invocation() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xE422));
    seed_frames(&tl, 10, 1);

    let adapter = LogRecallAdapter::new(Arc::clone(&tl));
    let page = adapter.recall(10, LogRecallFilter::default()).unwrap();
    let frame_id = page.entries[0].frame_id;
    adapter.fetch(10, frame_id).unwrap();

    let audit_filter = FrameFilter {
        kind: Some(FrameKind::CapabilityInvocation),
        ..Default::default()
    };
    let audit_rows = tl.query_frames(audit_filter).unwrap();
    assert_eq!(
        audit_rows.iter().filter(|r| r.intent == "log.fetch").count(),
        1,
        "expected exactly one CapabilityInvocation row with intent log.fetch"
    );
}

#[test]
fn write_distillate_emits_capability_invocation() {
    let tmp = tempfile::TempDir::new().unwrap();
    let memory_root = tmp.path().join("memory");
    let db_path = tmp.path().join("audit.db");
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xE423));
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

    let _token = tl.insert_frame_event(
        FrameKind::TaskAssign,
        1,
        None,
        "delegate",
        b"raw-payload",
        FrameOrigin::HumanAuthored,
    );
    let raw_id = tl.last_frame_id();

    let request = DistillationRequest::new(
        vec![raw_id],
        1,
        DigestPayload::Text("digest".into()),
        None,
    )
    .unwrap();
    writer.write_distillate(1, request).unwrap();

    let audit_filter = FrameFilter {
        kind: Some(FrameKind::CapabilityInvocation),
        ..Default::default()
    };
    let audit_rows = tl.query_frames(audit_filter).unwrap();
    assert_eq!(
        audit_rows
            .iter()
            .filter(|r| r.intent == "distillate.write")
            .count(),
        1,
        "expected exactly one CapabilityInvocation row with intent distillate.write"
    );
}
