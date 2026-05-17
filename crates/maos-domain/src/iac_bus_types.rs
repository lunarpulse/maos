//! Shared types for the IAC Bus port trait.
//!
//! These types are referenced by `IacBusPort` methods and live in
//! `maos-domain` so the port trait can reference them without a
//! dependency on `maos-kernel-core`.

use maos_spirit_abi::identity::FrameKind;

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
}
