#![forbid(unsafe_code)]

//! Story 18-5 AC2.1 / AC3.1 — `ContextSignalAdapter` (D-18-5-04 A).
//!
//! One typed frame per upward crossing, delivered to a registered mailbox and
//! journaled; with no mailbox the journaled row is the record; with no runtime
//! the crossing is written as a direct TL row; an unadmitted pid is refused.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use maos_capability::context_budget::{ContextBudget, ContextWindow};
use maos_domain::frame::FramePayload;
use maos_domain::ports::context_budget::{ContextBudgetPort, ContextLevel, ContextReportError};
use maos_iac::adapter::context_signal::{AdmittedSpirit, ContextSignalAdapter};
use maos_iac::adapter::transparency_log::{FrameFilter, FrameKind as TlKind};
use maos_iac::adapter::{IacBusAdapter, IacRtMetrics, Mailbox, TransparencyLogAdapter};
use maos_spirit_abi::identity::{FrameKind, SpiritId};

const BUTLER: ContextWindow = ContextWindow {
    size: 8192,
    pressure_permille: 800,
    limit_permille: 950,
};

fn bus() -> (Arc<IacBusAdapter>, Arc<TransparencyLogAdapter>) {
    maos_capability::cap_tokens::init_monotonic_base();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x18_05));
    let bus = Arc::new(IacBusAdapter::new(
        Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new()))),
        Arc::clone(&tl),
    ));
    (bus, tl)
}

fn adapter(bus: &Arc<IacBusAdapter>, pid: Arc<AtomicU32>) -> ContextSignalAdapter {
    ContextSignalAdapter {
        ledger: Arc::new(ContextBudget::default()),
        bus: Arc::downgrade(bus),
        admitted: Arc::new(move || {
            let pid = pid.load(Ordering::SeqCst);
            (pid != 0).then(|| AdmittedSpirit {
                pid,
                spirit_id: SpiritId::from("butler"),
                window: BUTLER,
            })
        }),
    }
}

fn rows(tl: &TransparencyLogAdapter, kind: TlKind) -> Vec<u32> {
    tl.query_frames(FrameFilter {
        kind: Some(kind),
        ..Default::default()
    })
    .expect("query")
    .into_iter()
    .map(|row| row.spirit_pid)
    .collect()
}

async fn settle(tl: &TransparencyLogAdapter, kind: TlKind, expected: usize) {
    for _ in 0..200 {
        if rows(tl, kind).len() >= expected {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_frame_per_upward_crossing_reaches_a_registered_mailbox() {
    let (bus, tl) = bus();
    let mut mailbox = bus
        .register_spirit_typed(&SpiritId::from("butler"))
        .expect("register");
    let port = adapter(&bus, Arc::new(AtomicU32::new(41)));

    assert_eq!(port.report_context_used(4000).unwrap().crossed, None);
    let status = port.report_context_used(6600).unwrap();
    assert_eq!(
        (status.level, status.crossed),
        (ContextLevel::Pressure, Some(ContextLevel::Pressure))
    );
    let (kind, frame) = tokio::time::timeout(Duration::from_secs(5), mailbox.recv())
        .await
        .expect("pressure frame delivered")
        .expect("open");
    assert_eq!(kind, FrameKind::ContextPressure);
    match frame.payload {
        FramePayload::ContextPressure(envelope) => {
            assert_eq!(
                (
                    envelope.spirit_pid,
                    envelope.context_used,
                    envelope.context_window_size,
                    envelope.threshold_permille
                ),
                (41, 6600, 8192, 800)
            );
        }
        other => panic!("unexpected payload {other:?}"),
    }
    // Still above pressure: no second frame.
    assert_eq!(port.report_context_used(6700).unwrap().crossed, None);
    port.report_context_used(7800).unwrap();
    let (kind, _) = tokio::time::timeout(Duration::from_secs(5), mailbox.recv())
        .await
        .expect("limit frame delivered")
        .expect("open");
    assert_eq!(kind, FrameKind::ContextLimit);
    settle(&tl, TlKind::ContextLimit, 1).await;
    assert_eq!(rows(&tl, TlKind::ContextPressure), [41]);
    assert_eq!(rows(&tl, TlKind::ContextLimit), [41]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_a_mailbox_the_journaled_row_is_the_record() {
    let (bus, tl) = bus();
    let port = adapter(&bus, Arc::new(AtomicU32::new(42)));
    let status = port.report_context_used(9000).unwrap();
    assert_eq!(status.level, ContextLevel::Exhausted);
    settle(&tl, TlKind::ContextLimit, 1).await;
    assert_eq!(rows(&tl, TlKind::ContextPressure), [42]);
    assert_eq!(rows(&tl, TlKind::ContextLimit), [42]);
}

#[test]
fn without_a_runtime_the_crossing_is_written_directly() {
    let (bus, tl) = bus();
    let port = adapter(&bus, Arc::new(AtomicU32::new(43)));
    port.report_context_used(6600).unwrap();
    assert_eq!(rows(&tl, TlKind::ContextPressure), [43]);
}

#[test]
fn an_unadmitted_pid_is_refused_and_moves_nothing() {
    let (bus, tl) = bus();
    let pid = Arc::new(AtomicU32::new(0));
    let port = adapter(&bus, Arc::clone(&pid));
    assert_eq!(
        port.report_context_used(9000),
        Err(ContextReportError::NotAdmitted)
    );
    assert!(rows(&tl, TlKind::ContextPressure).is_empty());
    pid.store(44, Ordering::SeqCst);
    assert_eq!(
        port.report_context_used(100).unwrap().level,
        ContextLevel::Within,
        "the refused report left no slot behind"
    );
}
