#![forbid(unsafe_code)]

//! Story 6.1 — DRR scheduler integration tests.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;

use maos_domain::frame::{
    FrameAddress, FramePayload, IacFrame, PosturePreferences, TaskAssignPayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::{
    drr_scheduler::{BudgetWarningEvent, DrrScheduler},
    IacBusAdapter, Mailbox, TransparencyLogAdapter,
};
use maos_spirit_abi::identity::{FrameKind, SpiritId};
use smallvec::smallvec;
use tokio::sync::mpsc;

static FRAME_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn make_frame(from: &str, to: &str, payload_size: usize) -> IacFrame {
    let goal = "x".repeat(payload_size);
    let counter = FRAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let mut frame_id = [0u8; 16];
    frame_id[0..8].copy_from_slice(&counter.to_le_bytes());
    IacFrame {
        frame_id,
        timestamp_ns: 0,
        logical_clock: 0,
        from: FrameAddress {
            spirit_id: SpiritId::from(from),
            host_id: None,
            role: None,
        },
        to: smallvec![FrameAddress {
            spirit_id: SpiritId::from(to),
            host_id: None,
            role: None,
        }],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal,
            scope: vec![],
            success_criteria: "done".into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn drr_basic_two_spirits_fair() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let mailbox = Arc::new(Mailbox::new(Arc::new(
        maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new(),
    )));

    let (bw_tx, mut bw_rx) = mpsc::unbounded_channel::<BudgetWarningEvent>();
    let drr = DrrScheduler::new(tl.clone(), bw_tx);
    let adapter = IacBusAdapter::new(mailbox.clone(), tl.clone()).with_drr_scheduler(drr);

    let _h1 = adapter.register_spirit_typed(&SpiritId::from("a")).unwrap();
    let _h2 = adapter.register_spirit_typed(&SpiritId::from("b")).unwrap();

    // Spirit "a" sends a large frame (5 KiB > 4 KiB quantum)
    let big = make_frame("a", "b", 5 * 1024);
    adapter.deliver_typed(big).await.unwrap();

    // Spirit "b" sends a small frame (1 KiB < 4 KiB quantum)
    let small = make_frame("b", "a", 1 * 1024);
    adapter.deliver_typed(small).await.unwrap();

    // Both should be logged
    let entries = tl.query_frames(Default::default()).unwrap();
    assert_eq!(entries.len(), 2, "both frames should be logged");

    // No backpressure expected for just two frames
    assert!(bw_rx.try_recv().is_err(), "no backpressure expected");
}

// ONE worker thread: the DRR processor (spawned by `DrrScheduler::new`) and
// the submitter task below share it, so neither runs while the other is being
// polled. `block_in_place` in `flush_batch` rules out `current_thread`.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn drr_backpressure_emitted_when_backlog_exceeds_threshold() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let mailbox = Arc::new(Mailbox::new(Arc::new(
        maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new(),
    )));

    let (bw_tx, mut bw_rx) = mpsc::unbounded_channel::<BudgetWarningEvent>();
    let drr = DrrScheduler::new(tl.clone(), bw_tx);
    let adapter = IacBusAdapter::new(mailbox.clone(), tl.clone()).with_drr_scheduler(drr);

    let _h1 = adapter.register_spirit_typed(&SpiritId::from("a")).unwrap();
    let _h2 = adapter.register_spirit_typed(&SpiritId::from("b")).unwrap();

    // Spirit "a" floods with 10 × 1 KiB frames = 10 KiB backlog.
    // Threshold is 2 × 4 KiB = 8 KiB.
    //
    // DETERMINISTIC by construction (2026-09-24). The warning is emitted only
    // at ENQUEUE, when a's queue exceeds 8 KiB — and the processor drains
    // queues on a 100 ms ticker. The old test spawned ten submit tasks and
    // hoped they all landed before a tick: on a loaded runner a tick fell
    // mid-arrival, drained ~4 KiB, the backlog never crossed 8 KiB, and the
    // warning was (correctly) never emitted — CI 79610248 waited the full
    // 10 s bound and failed. No wait can fix that; ordering can.
    //
    // `submit` SENDS synchronously, then awaits the persist ack. Polling all
    // ten submit futures inside ONE task poll puts all ten in the channel
    // before the processor — on the same single worker — can run; its
    // `biased` select then enqueues all ten before it ever considers the tick.
    let drr = adapter.drr_scheduler().unwrap().clone();
    let submitter = tokio::spawn(async move {
        let mut pending: Vec<Pin<Box<dyn Future<Output = ()> + Send>>> = (0..10)
            .map(|_| {
                let f = make_frame("a", "b", 1 * 1024);
                let payload = serde_json::to_vec(&f.payload).unwrap();
                let drr = drr.clone();
                Box::pin(async move {
                    let _ = drr
                        .submit(
                            f,
                            payload,
                            maos_kernel_core::iac::FrameKind::TaskAssign,
                            0,
                            "standard".into(),
                            FrameOrigin::HumanAuthored,
                            vec![],
                        )
                        .await;
                }) as Pin<Box<dyn Future<Output = ()> + Send>>
            })
            .collect();
        std::future::poll_fn(|cx| {
            pending.retain_mut(|submit| submit.as_mut().poll(cx).is_pending());
            if pending.is_empty() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
    });
    submitter.await.expect("submitter task");

    // Bounded wait on the condition (Story 15-1 AC6(b)): the original fixed
    // `sleep(50ms)` was a synchronisation primitive — under load the DRR
    // processor has not necessarily emitted within 50 ms (measured 3/20 at
    // default threads, 1/20 even at 1). Wait for the warning itself, bounded.
    let mut found = false;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if found {
            break;
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, bw_rx.recv()).await {
            Ok(Some(evt)) => {
                if evt.spirit_id == "a" && evt.backlog_bytes >= 8 * 1024 {
                    found = true;
                }
            }
            Ok(None) => break,      // channel closed — no further events possible
            Err(_elapsed) => break, // deadline exhausted without the condition
        }
    }
    assert!(
        found,
        "expected budget warning for spirit a with backlog >= 8 KiB"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn drr_batch_flush_on_interval() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let mailbox = Arc::new(Mailbox::new(Arc::new(
        maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new(),
    )));

    let (bw_tx, _bw_rx) = mpsc::unbounded_channel::<BudgetWarningEvent>();
    let drr = DrrScheduler::new(tl.clone(), bw_tx);
    let adapter = IacBusAdapter::new(mailbox.clone(), tl.clone()).with_drr_scheduler(drr);

    let _h1 = adapter.register_spirit_typed(&SpiritId::from("a")).unwrap();

    // Submit a single small frame — because the batch is not full,
    // deliver_typed blocks until the interval flushes it.
    let f = make_frame("a", "a", 100);
    adapter.deliver_typed(f).await.unwrap();

    // Frame should now be in the TL
    let entries = tl.query_frames(Default::default()).unwrap();
    assert_eq!(entries.len(), 1, "frame should be flushed");
}
