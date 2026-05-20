//! Integration test: Self-telemetry scope — only calling Spirit's data.
//! AC4 — Story 4.3 + Story 4.4 (Distillate frame counting).

use std::sync::Arc;

use maos_domain::ports::SelfTelemetryPort;
use maos_domain::ports::DistillationPort;
use maos_domain::self_telemetry::SelfTelemetryError;
use maos_kernel_core::memory::self_telemetry::SelfTelemetryAggregator;
use maos_kernel_core::halt::HaltRegistry;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::distillation::{DigestPayload, DistillationRequest};

fn make_aggregator() -> (SelfTelemetryAggregator, Arc<TransparencyLogAdapter>) {
    let metrics = Arc::new(IacRtMetrics::new());
    let registry = Arc::new(HaltRegistry::new());
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xBEE5));
    let agg = SelfTelemetryAggregator::new(metrics, registry, Arc::clone(&tl));
    (agg, tl)
}

#[test]
fn empty_window_returns_zeros() {
    let (agg, _tl) = make_aggregator();
    let report = agg.self_telemetry(1, None).unwrap();
    assert_eq!(report.spirit_pid, 1);
    assert_eq!(report.success_count, 0);
    assert_eq!(report.failure_count, 0);
    assert!(report.halt_events.is_empty());
    assert!(report.distillation_outcomes.is_empty());
}

#[test]
fn since_ns_zero_includes_all_frames() {
    let (agg, tl) = make_aggregator();

    // Seed a frame.
    tl.insert_frame_event(
        FrameKind::TaskComplete,
        1,
        None,
        "test",
        b"payload",
        FrameOrigin::SpiritAuto,
    );

    // Query with since_ns = 0 — the frame should be included.
    let report = agg.self_telemetry(1, Some(0)).unwrap();
    assert_eq!(report.success_count, 1);
}

#[test]
fn mixed_pid_frames_filtered_by_spirit_pid() {
    let (agg, tl) = make_aggregator();

    // Seed frames for pid 1 and pid 2.
    tl.insert_frame_event(
        FrameKind::TaskComplete,
        1,
        None,
        "test",
        b"pid1",
        FrameOrigin::SpiritAuto,
    );
    tl.insert_frame_event(
        FrameKind::TaskComplete,
        2,
        None,
        "test",
        b"pid2",
        FrameOrigin::SpiritAuto,
    );

    let r1 = agg.self_telemetry(1, None).unwrap();
    assert_eq!(r1.success_count, 1);
    assert_eq!(r1.spirit_pid, 1);

    let r2 = agg.self_telemetry(2, None).unwrap();
    assert_eq!(r2.success_count, 1);
    assert_eq!(r2.spirit_pid, 2);
}

#[test]
fn audit_row_written_per_call() {
    let (agg, tl) = make_aggregator();

    let before = tl.query_frames(Default::default()).unwrap();
    let before_count = before.len();

    agg.self_telemetry(1, None).unwrap();

    let after = tl.query_frames(Default::default()).unwrap();
    let after_count = after.len();

    assert_eq!(after_count, before_count + 1, "CapabilityInvocation audit row should be written");
}

#[test]
fn self_telemetry_counts_distillate_frames_precisely() {
    let (agg, tl) = make_aggregator();

    // Seed 3 raw frames for pid=1
    for _ in 0..3 {
        tl.insert_frame_event(
            FrameKind::TaskAssign,
            1,
            None,
            "delegate",
            b"raw",
            FrameOrigin::HumanAuthored,
        );
    }

    // Create a DistillateWriter to produce Distillate frames via production path.
    let tmp = tempfile::TempDir::new().unwrap();
    let memory_root = tmp.path().join("memory");
    let db_path = tmp.path().join("audit.db");
    let private = Arc::new(maos_kernel_core::memory::PrivateMemoryStore::new(memory_root, 4 * 1024));
    let shared = Arc::new(maos_kernel_core::memory::SharedMemoryStore::open(&db_path).unwrap());
    let principal_index = Arc::new(maos_kernel_core::memory::PrincipalNamespaceIndex::open(&db_path).unwrap());
    let memory = Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
        private, shared, principal_index, Arc::clone(&tl),
    ));
    let writer = DistillateWriter::new(Arc::clone(&tl), memory);

    // Write 3 Distillate frames for pid=1
    for _ in 0..3 {
        let raw_id = tl.last_frame_id();
        let req = DistillationRequest::new(
            vec![raw_id],
            1,
            DigestPayload::Text("digest".into()),
            None,
        ).unwrap();
        writer.write_distillate(1, req).unwrap();
    }

    // Write 2 Distillate frames for pid=2
    for _ in 0..2 {
        tl.insert_frame_event(
            FrameKind::TaskAssign,
            2,
            None,
            "consult",
            b"raw",
            FrameOrigin::HumanAuthored,
        );
        let raw_id = tl.last_frame_id();
        let req = DistillationRequest::new(
            vec![raw_id],
            1,
            DigestPayload::Text("digest".into()),
            None,
        ).unwrap();
        writer.write_distillate(2, req).unwrap();
    }

    let r1 = agg.self_telemetry(1, None).unwrap();
    assert_eq!(r1.distillation_outcomes.len(), 3, "pid=1 should see exactly 3 distillate frames");

    let r2 = agg.self_telemetry(2, None).unwrap();
    assert_eq!(r2.distillation_outcomes.len(), 2, "pid=2 should see exactly 2 distillate frames");
}
