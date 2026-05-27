//! Typed errors for the A2A surface.
//!
//! Per architecture §7.2: cross-Host frame intake rejects with one of the
//! variants below; the JSON-RPC NACK encodes the variant via the
//! `error.code` field (see `transport::json_rpc::NackError` for the wire
//! mapping).

use crate::consent::EIntentDenied;
use crate::tofu::EPinMismatch;

pub type A2AResult<T> = Result<T, A2AError>;

/// Direction context for `IntentDenied` — distinguishes the SENDER-side
/// outbound rejection (`Send`) from the RECEIVER-side intake rejection
/// (`Accept`). Defense in depth — both ends MUST validate per ADR-012.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IntentDirection {
    Send,
    Accept,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum A2AError {
    /// Operator config invalid (e.g., `partition_timeout_secs` out of range).
    #[error("a2a config invalid: {0}")]
    ConfigInvalid(String),

    /// TOFU pin mismatch — see `EPinMismatch` for the full discriminator.
    #[error("tofu pin mismatch: {0}")]
    PinMismatch(#[from] EPinMismatch),

    /// Pin invalidated by Spirit restart (NFR-Rel-6) — re-pin consent
    /// is required before outbound frames may resume.
    #[error("pin invalidated for peer {peer} — awaiting_repin={awaiting_repin}")]
    PinInvalidated { peer: String, awaiting_repin: bool },

    /// ADR-012 typed-intent consent denied — defense-in-depth on send + accept.
    #[error("a2a intent denied ({direction:?}): {inner}")]
    IntentDenied {
        direction: IntentDirection,
        inner: EIntentDenied,
    },

    /// Receiver returned a JSON-RPC NACK with `-32001 EIntentDenied`.
    #[error("a2a intent denied at peer {peer}: {message}")]
    IntentDeniedAtPeer { peer: String, message: String },

    /// Consent envelope's `valid_until_ns` is in the past.
    #[error("consent envelope expired at {expired_at_ns}; now is {now_ns}")]
    ConsentExpired { expired_at_ns: u64, now_ns: u64 },

    /// Outbound `route_outbound` timed out awaiting receiver ACK — partition
    /// behavior per architecture §7.2 "A2A in-flight frames during partition
    /// are NACKed after a configurable timeout (default 30s); the kernel does
    /// NOT auto-retry."
    #[error("a2a partition timeout for peer {peer}: frame_id {frame_id:?} after {timeout_secs}s")]
    PartitionTimeout {
        peer: String,
        frame_id: [u8; 16],
        timeout_secs: u64,
    },

    /// JSON-RPC framing or transport-level failure.
    #[error("a2a transport failure: {0}")]
    TransportFailed(String),

    /// mTLS handshake failed.
    #[error("a2a handshake failed: {0}")]
    HandshakeFailed(String),

    /// Spirit-restart-pin invalidation: caller attempted to use a pin record
    /// for a peer whose Spirit boot_nonce has rolled.
    #[error("spirit restart detected on peer {peer}: prior_boot_nonce={prior_boot_nonce} observed_boot_nonce={observed_boot_nonce}")]
    SpiritRestartDetected {
        peer: String,
        prior_boot_nonce: u64,
        observed_boot_nonce: u64,
    },

    /// Frame deserialization failed.
    #[error("a2a frame deserialization failed: {0}")]
    DeserializationFailed(String),

    /// Catch-all I/O error.
    #[error("a2a io error: {0}")]
    Io(String),
}
