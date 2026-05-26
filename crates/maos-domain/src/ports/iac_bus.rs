//! IAC Bus port trait per architecture §4.5.
//!
//! Routes frames between Spirits and the kernel. At v0.1-α this trait
//! declares the data-movement surface; Story 6.1 lands the full IAC
//! Bus with retract primitive and DRR fairness scheduler.

use crate::frame::IacFrame;
use crate::iac_bus_types::{IacBusError, RetractOutcome};
use crate::invariants::i2::LogBeforeDeliver;
use crate::invariants::i3::FrameOrigin;
use maos_spirit_abi::identity::SpiritId;

/// IAC Bus — inter-agent communication frame routing.
///
/// Per §4.5: "Every IAC frame is logged before delivery (I2) and
/// carries an origin stamp (I3)."
///
/// # Associated types (Story 3.1)
///
/// `MailboxHandle` has a default of `()` so existing implementors
/// (tests, mocks) don't break. The kernel adapter overrides it.
pub trait IacBusPort {
    /// Handle returned by `register_spirit`.
    type MailboxHandle: std::fmt::Debug;

    /// Class: data-movement
    ///
    /// Enqueue a single frame onto the IAC Bus for delivery to its
    /// destination Spirit. At v0.1-α this is a structural placeholder;
    /// Story 1b.2 wires the actual mailbox mechanics.
    fn enqueue_frame(&self, frame_bytes: &[u8], origin: FrameOrigin) -> LogBeforeDeliver<()>;

    /// Class: data-movement
    ///
    /// Broadcast a frame to all subscribed Spirits on the given topic.
    /// Used for telemetry and global halt signals.
    fn broadcast_frame(&self, frame_bytes: &[u8], origin: FrameOrigin) -> LogBeforeDeliver<()>;

    /// Class: data-movement
    ///
    /// Deliver a typed `IacFrame` through the I2 log-before-deliver
    /// pipeline. Story 3.1 wires the Mailbox; Story 6.1 adds DRR
    /// fairness scheduling.
    async fn deliver(&self, frame: IacFrame) -> Result<LogBeforeDeliver<()>, IacBusError>;

    /// Class: data-movement
    ///
    /// Register a Spirit on the IAC Bus, creating per-kind bounded
    /// channels with §7.1.1 capacity floors. Story 3.1 wires the
    /// real Mailbox; Story 6.1 adds persistence.
    fn register_spirit(&self, spirit_id: &SpiritId) -> Result<Self::MailboxHandle, IacBusError>;

    /// Class: data-movement
    ///
    /// Retract a previously-delivered frame. Idempotent: re-retracting the same
    /// `original_frame_id` returns `Ok(Already)` rather than a duplicate-emission
    /// error. Story 6.1 (FR22 full features + ADR-022 retract semantics).
    async fn retract(
        &self,
        original_frame_id: [u8; 16],
        reason: String,
        retracting_spirit: &SpiritId,
    ) -> Result<RetractOutcome, IacBusError>;
}
