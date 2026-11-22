#![forbid(unsafe_code)]

//! later-registry-quarantine-closure AC1/AC8 (KQ2) — a quarantined sender's native IAC frame is
//! refused BEFORE the log append (no TL row, nothing delivered) and its distillate write is
//! refused typed. A fence is a refusal of effects, not isolation (ruling `q`).
//!
//! Planted reds (orchestrator runs them): delete the `is_sender_fenced` check at the top of
//! `IacBusAdapter::deliver_typed` -> the first test fails; delete the `sender_fence` check in
//! `DistillateWriter::write_distillate` -> the second test fails.

use std::sync::Arc;

use maos_domain::distillation::{DigestPayload, DistillationError, DistillationRequest};
use maos_domain::frame::{BudgetEnvelope, FrameAddress, FramePayload, IacFrame};
use maos_domain::iac_bus_types::IacBusError;
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::DistillationPort;
use maos_iac::adapter::distillate::DistillateWriter;
use maos_iac::adapter::transparency_log::{FrameFilter, TransparencyLogAdapter};
use maos_iac::adapter::{IacBusAdapter, IacRtMetrics, Mailbox};
use maos_spirit_abi::identity::{FrameKind, SpiritId};

fn bus() -> (Arc<TransparencyLogAdapter>, IacBusAdapter) {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x1fe0));
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let iac = IacBusAdapter::new(mailbox, Arc::clone(&tl));
    (tl, iac)
}

fn rows(tl: &TransparencyLogAdapter, pid: u32) -> usize {
    tl.query_frames(FrameFilter {
        spirit_pid: Some(pid),
        ..Default::default()
    })
    .unwrap()
    .len()
}

fn frame() -> IacFrame {
    let address = |name: &str| FrameAddress {
        spirit_id: SpiritId::from(name),
        host_id: None,
        role: None,
    };
    IacFrame {
        frame_id: [9; 16],
        timestamp_ns: 1,
        logical_clock: 1,
        from: address("sender"),
        to: std::iter::once(address("peer")).collect(),
        kind: FrameKind::BudgetWarning,
        intent: IntentClass::Standard,
        payload: FramePayload::BudgetWarning(BudgetEnvelope {
            spirit_pid: 5,
            hook_name: "on_idle".into(),
            wall_ns: 1,
            cap_seconds: 1,
        }),
        auto_marker: FrameOrigin::Kernel,
        consent_envelope: None,
        intent_lineage: Default::default(),
    }
}

#[tokio::test]
async fn a_quarantined_sender_frame_is_neither_logged_nor_delivered() {
    let (tl, iac) = bus();
    iac.set_sender_fenced(5, true);
    let refused = iac.deliver_typed(frame(), 5, None).await;
    assert!(
        matches!(
            &refused,
            Err(IacBusError::SenderQuarantined { spirit_pid: 5 })
        ),
        "{:?}",
        refused.err()
    );
    assert_eq!(rows(&tl, 5), 0, "a refused frame is never logged");
    // A different pid is not fenced by this one's quarantine.
    let other = iac.deliver_typed(frame(), 6, None).await;
    assert!(!matches!(
        &other,
        Err(IacBusError::SenderQuarantined { .. })
    ));
    // Release restores the sender.
    iac.set_sender_fenced(5, false);
    let released = iac.deliver_typed(frame(), 5, None).await;
    assert!(!matches!(
        &released,
        Err(IacBusError::SenderQuarantined { .. })
    ));
}

#[test]
fn a_quarantined_pid_distillate_write_is_refused_typed() {
    let (tl, iac) = bus();
    let writer = DistillateWriter::new(Arc::clone(&tl), Arc::new(()));
    writer.set_sender_fence(iac.sender_fence());
    iac.set_sender_fenced(5, true);
    let request =
        DistillationRequest::new(vec![[1u8; 16]], 1, DigestPayload::Text("x".into()), None)
            .unwrap();
    let refused = writer.write_distillate(5, request);
    assert!(
        matches!(
            &refused,
            Err(DistillationError::Quarantined { spirit_pid: 5 })
        ),
        "{:?}",
        refused.err()
    );
    assert_eq!(
        rows(&tl, 5),
        0,
        "a refused distillate writes no row, not even the audit row"
    );
}
