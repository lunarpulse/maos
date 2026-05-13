//! I3: Auto-responses are always marked `[auto-sent]` on both sides.
//!
//! The "no puppeting" rule. Every message carries an `origin` stamp:
//! `human-authored`, `spirit-auto`, or `spirit-drafted-human-approved`.
//!
//! # Enforcement
//!
//! - **v0.1**: `CI` — structural lint over IAC frame origin stamps.
//! - **v0.3 / v0.5 / v0.9 / v1.0 / v1.5**: `CI` (unchanged; no runtime
//!   upgrade path adds value for a structural lint).
//!
//! # Invariant statement (doctest)
//!
//! ```
//! use maos_domain::invariants::i3::{InvariantI3, FrameOrigin};
//!
//! let _marker: InvariantI3 = InvariantI3;
//! assert_eq!(FrameOrigin::HumanAuthored as u8, 0);
//! assert_eq!(FrameOrigin::SpiritAuto as u8, 1);
//! assert_eq!(FrameOrigin::SpiritDraftedHumanApproved as u8, 2);
//! ```

/// I3 marker type — Auto-responses are always marked `[auto-sent]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantI3;

/// Origin stamp for every IAC frame — the "no puppeting" rule at the
/// type level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum FrameOrigin {
    /// Human typed or explicitly approved every byte.
    HumanAuthored = 0,
    /// Spirit generated without human in the loop.
    SpiritAuto = 1,
    /// Spirit drafted; human approved the draft (one-click or explicit).
    SpiritDraftedHumanApproved = 2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_origin_discriminants_stable() {
        assert_eq!(FrameOrigin::HumanAuthored as u8, 0);
        assert_eq!(FrameOrigin::SpiritAuto as u8, 1);
        assert_eq!(FrameOrigin::SpiritDraftedHumanApproved as u8, 2);
    }
}
