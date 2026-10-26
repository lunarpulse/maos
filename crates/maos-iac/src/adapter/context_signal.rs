#![forbid(unsafe_code)]

//! Story 18-5 (ADR-016, D-18-5-04 A) — the root-built [`ContextBudgetPort`].
//!
//! One adapter per Spirit, fixed by the root to a resolver of the Spirit's
//! admitted incarnation: its pid, id and the window and thresholds of its
//! admitted manifest bundle — never chosen by the Spirit. A report moves the
//! kernel registry's ledger and returns a typed [`ContextStatus`]; each upward
//! threshold crossing becomes one kernel-written `ContextPressure` (32) /
//! `ContextLimit` (33) frame. Delivery is spawned on the current runtime (the
//! port is sync); the bus journals the frame before delivery, and a frame the
//! bus could not journal is written as a direct TL row, so no crossing goes
//! unrecorded.

use std::sync::{Arc, Weak};

use maos_capability::context_budget::{ContextBudget, ContextWindow, Crossing};
use maos_domain::frame::{ContextEnvelope, FrameAddress, FramePayload, IacFrame};
use maos_domain::iac_bus_types::IacBusError;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::context_budget::{
    ContextBudgetPort, ContextLevel, ContextReportError, ContextStatus,
};
use maos_spirit_abi::identity::{FrameKind, SpiritId};

use super::transparency_log::FrameKind as TlKind;
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
    pub ledger: Arc<ContextBudget>,
    pub bus: Weak<IacBusAdapter>,
    pub admitted: Admitted,
}

fn frame(bus: &IacBusAdapter, spirit: &AdmittedSpirit, used: u32, crossing: Crossing) -> IacFrame {
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
        frame_id: bus.transparency_log().mint_frame_id(),
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

/// The direct TL row for a crossing the bus could not journal.
fn journal(bus: &IacBusAdapter, pid: u32, frame: &IacFrame) {
    let (kind, intent) = match frame.kind {
        FrameKind::ContextPressure => (TlKind::ContextPressure, "context.pressure"),
        _ => (TlKind::ContextLimit, "context.limit"),
    };
    if let Ok(payload) = serde_json::to_vec(&frame.payload) {
        let tl = bus.transparency_log();
        let _ = tl.insert_frame_event(kind, pid, None, intent, &payload, FrameOrigin::Kernel);
    }
}

/// Deliver the crossings in order (pressure before limit) on one task.
async fn deliver(bus: Arc<IacBusAdapter>, pid: u32, frames: Vec<IacFrame>) {
    for frame in frames {
        match bus.deliver_typed(frame.clone(), pid, None).await {
            // Journaled; `UnknownSpirit` = no mailbox registered (native
            // in-proc Spirits): the row is the record.
            Ok(_) | Err(IacBusError::UnknownSpirit(_)) => {}
            Err(_) => journal(&bus, pid, &frame),
        }
    }
}

impl ContextBudgetPort for ContextSignalAdapter {
    fn report_context_used(&self, used_tokens: u32) -> Result<ContextStatus, ContextReportError> {
        let spirit = (self.admitted)().ok_or(ContextReportError::NotAdmitted)?;
        let (status, crossings) = self.ledger.report(spirit.pid, spirit.window, used_tokens);
        let Some(bus) = self.bus.upgrade().filter(|_| !crossings.is_empty()) else {
            return Ok(status);
        };
        let frames: Vec<_> = crossings
            .into_iter()
            .map(|crossing| frame(&bus, &spirit, used_tokens, crossing))
            .collect();
        match tokio::runtime::Handle::try_current() {
            Ok(runtime) => drop(runtime.spawn(deliver(bus, spirit.pid, frames))),
            Err(_) => frames.iter().for_each(|f| journal(&bus, spirit.pid, f)),
        }
        Ok(status)
    }
}
