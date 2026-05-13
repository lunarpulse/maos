//! IAC Bus port trait per architecture §4.5.
//!
//! Routes frames between Spirits and the kernel. At v0.1-α this trait
//! declares the data-movement surface; Story 6.1 lands the full IAC
//! Bus with retract primitive and DRR fairness scheduler.

use crate::invariants::i2::LogBeforeDeliver;
use crate::invariants::i3::FrameOrigin;

/// IAC Bus — inter-agent communication frame routing.
///
/// Per §4.5: "Every IAC frame is logged before delivery (I2) and
/// carries an origin stamp (I3)."
pub trait IacBusPort {
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
}
