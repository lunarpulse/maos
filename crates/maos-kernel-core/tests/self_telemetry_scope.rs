//! Integration test: Self-telemetry scope — only calling Spirit's data.
//! AC4 — Story 4.3.

use std::sync::Arc;

use maos_domain::ports::SelfTelemetryPort;
use maos_domain::self_telemetry::SelfTelemetryError;
use maos_kernel_core::memory::self_telemetry::SelfTelemetryAggregator;
use maos_kernel_core::halt::HaltRegistry;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_domain::invariants::i3::FrameOrigin;

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
