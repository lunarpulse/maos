//! Shared types for the IAC Bus port trait.
//!
//! These types are referenced by `IacBusPort` methods and live in
//! `maos-domain` so the port trait can reference them without a
//! dependency on `maos-kernel-core`.

use maos_spirit_abi::identity::FrameKind;

use crate::invariants::i3::FrameOrigin;

/// Typed error for IAC bus operations.
#[derive(Debug, thiserror::Error)]
pub enum IacBusError {
    #[error("spirit {0} is not registered — call register_spirit first")]
    UnknownSpirit(String),
    #[error("epistemic halt queue overflow for spirit {0} — kernel MUST raise watchdog (Story 3.3)")]
    HaltQueueOverflow(String),
    #[error("channel closed for spirit {0} kind {1:?}")]
    ChannelClosed(String, FrameKind),
    #[error("frame serialization failed: {0}")]
    SerializationFailed(String),
    #[error("cross-host routing unsupported at v0.3-β (Story 6.3)")]
    CrossHostUnsupported,
    #[error("channel full for spirit {0} kind {1:?} — backpressure")]
    QueueFull(String, FrameKind),
    #[error("spirit {0} is already registered — deregister before re-registering")]
    AlreadyRegistered(String),
    /// Story 4.5 — NFR-Aud-14: cross-Spirit frame arrived with no
    /// intent_lineage AND non-human origin. The kernel auto-computes
    /// lineage for `FrameOrigin::HumanAuthored` originating frames
    /// (single-class lineage from `frame.intent`), so this variant
    /// fires for Spirit-emitted cross-Spirit frames missing lineage —
    /// the structural sign of consent-laundering through re-emission.
    #[error("intent_lineage chain broken on cross-Spirit frame from {from} to {to}: empty lineage on non-human origin {origin:?}")]
    EIntentLineageBroken {
        from: String,
        to: String,
        origin: FrameOrigin,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eintent_lineage_broken_display() {
        let err = IacBusError::EIntentLineageBroken {
            from: "spirit-a".into(),
            to: "spirit-b".into(),
            origin: FrameOrigin::SpiritAuto,
        };
        let msg = format!("{err}");
        assert!(msg.contains("spirit-a"));
        assert!(msg.contains("spirit-b"));
        assert!(msg.contains("SpiritAuto"));
    }

    #[test]
    fn eintent_lineage_broken_spirit_drafted() {
        let err = IacBusError::EIntentLineageBroken {
            from: "s1".into(),
            to: "s2".into(),
            origin: FrameOrigin::SpiritDraftedHumanApproved,
        };
        let msg = format!("{err}");
        assert!(msg.contains("SpiritDraftedHumanApproved"));
    }
}
