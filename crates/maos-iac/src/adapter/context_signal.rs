#![forbid(unsafe_code)]

//! Story 18-5 (ADR-016, D-18-5-04 A) — the root-built [`ContextBudgetPort`].
//!
//! One adapter per Spirit, fixed by the root to a resolver of the Spirit's
//! admitted incarnation: its pid, id and the window and thresholds of its
//! admitted manifest bundle — never chosen by the Spirit. A report moves the
//! kernel registry's ledger and returns a typed [`ContextStatus`]; each upward
//! threshold crossing becomes one kernel-written `ContextPressure` (32) /
//! `ContextLimit` (33) frame. Reports journal synchronously, even after the
//! bus is gone; a single ordered task per adapter handles mailbox delivery.

use std::sync::{Arc, Mutex, Weak};

use maos_capability::context_budget::{ContextBudget, ContextWindow, Crossing};
use maos_domain::frame::{ContextEnvelope, FrameAddress, FramePayload, IacFrame};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::context_budget::{
    ContextBudgetPort, ContextLevel, ContextReportError, ContextStatus,
};
use maos_spirit_abi::identity::{FrameKind, SpiritId};

use super::mailbox::PreparedDelivery;
use super::transparency_log::{FrameKind as TlKind, TransparencyLogAdapter};
use super::IacBusAdapter;

/// The Spirit's admitted incarnation, as the root resolves it.
#[derive(Debug, Clone)]
pub struct AdmittedSpirit {
    pub pid: u32,
    pub spirit_id: SpiritId,
    pub window: ContextWindow,
}

/// Resolves the admitted incarnation; `None` when not (or no longer) admitted.
pub type Admitted = Arc<dyn Fn() -> Option<AdmittedSpirit> + Send + Sync>;

/// The root-fixed context reporter for one Spirit. `bus` is held weakly: a
/// Spirit never keeps the host's bus alive.
pub struct ContextSignalAdapter {
    ledger: Arc<ContextBudget>,
    bus: Weak<IacBusAdapter>,
    admitted: Admitted,
    journal: Arc<TransparencyLogAdapter>,
    delivery: Mutex<Option<tokio::sync::mpsc::UnboundedSender<(IacFrame, PreparedDelivery)>>>,
}

fn frame(
    tl: &TransparencyLogAdapter,
    spirit: &AdmittedSpirit,
    used: u32,
    crossing: Crossing,
) -> IacFrame {
    let envelope = ContextEnvelope {
        spirit_pid: spirit.pid,
        context_used: used,
        context_window_size: spirit.window.size,
        threshold_permille: crossing.threshold_permille,
    };
    let (kind, payload) = match crossing.level {
        ContextLevel::Pressure => (
            FrameKind::ContextPressure,
            FramePayload::ContextPressure(envelope),
        ),
        _ => (
            FrameKind::ContextLimit,
            FramePayload::ContextLimit(envelope),
        ),
    };
    let address = |spirit_id: SpiritId| FrameAddress {
        spirit_id,
        host_id: None,
        role: None,
    };
    IacFrame {
        frame_id: tl.mint_frame_id(),
        timestamp_ns: maos_capability::cap_tokens::monotonic_now_ns(),
        logical_clock: 0,
        from: address(super::mailbox::KERNEL_SENDER_SPIRIT_ID.into()),
        to: vec![address(spirit.spirit_id.clone())].into(),
        kind,
        intent: maos_domain::invariants::i1::IntentClass::Standard,
        payload,
        auto_marker: FrameOrigin::Kernel,
        consent_envelope: None,
        intent_lineage: Default::default(),
    }
}

/// Journal before enqueueing; mailbox errors can never create a second row.
fn journal(
    tl: &TransparencyLogAdapter,
    bus: Option<&IacBusAdapter>,
    pid: u32,
    frame: &IacFrame,
) -> PreparedDelivery {
    let (kind, intent) = match frame.kind {
        FrameKind::ContextPressure => (TlKind::ContextPressure, "context.pressure"),
        _ => (TlKind::ContextLimit, "context.limit"),
    };
    let prepared = bus.map_or_else(PreparedDelivery::default, |bus| {
        bus.mailbox.prepare(frame, tl.recipient_resolver())
    });
    if let Ok(payload) = serde_json::to_vec(&frame.payload) {
        if bus.is_some() {
            let _ = tl.insert_routed_frame(
                frame,
                kind,
                pid,
                None,
                "standard",
                &payload,
                prepared.recipients(),
            );
        } else {
            let _ = tl.insert_frame_event(kind, pid, None, intent, &payload, FrameOrigin::Kernel);
        }
    }
    prepared
}

impl ContextSignalAdapter {
    pub fn new(ledger: Arc<ContextBudget>, bus: &Arc<IacBusAdapter>, admitted: Admitted) -> Self {
        Self {
            ledger,
            bus: Arc::downgrade(bus),
            admitted,
            journal: Arc::clone(bus.transparency_log()),
            delivery: Mutex::new(None),
        }
    }
}

impl ContextBudgetPort for ContextSignalAdapter {
    fn report_context_used(&self, used_tokens: u32) -> Result<ContextStatus, ContextReportError> {
        // Serialize ledger transitions, synchronous writes and enqueue order.
        let mut delivery = self.delivery.lock().unwrap_or_else(|e| e.into_inner());
        let spirit = (self.admitted)().ok_or(ContextReportError::NotAdmitted)?;
        let (status, crossings) = self.ledger.report(spirit.pid, spirit.window, used_tokens);
        if crossings.is_empty() {
            return Ok(status);
        }
        let bus = self.bus.upgrade();
        if delivery.is_none() {
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                let (sender, mut receiver) =
                    tokio::sync::mpsc::unbounded_channel::<(IacFrame, PreparedDelivery)>();
                let bus = self.bus.clone();
                drop(runtime.spawn(async move {
                    while let Some((frame, prepared)) = receiver.recv().await {
                        let Some(bus) = bus.upgrade() else { break };
                        let _ = bus.mailbox.deliver_prepared(frame, prepared).await;
                    }
                }));
                *delivery = Some(sender);
            }
        }
        for crossing in crossings {
            let frame = frame(&self.journal, &spirit, used_tokens, crossing);
            let prepared = journal(&self.journal, bus.as_deref(), spirit.pid, &frame);
            if let Some(sender) = delivery.as_ref() {
                let _ = sender.send((frame, prepared));
            }
        }
        Ok(status)
    }
}
