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

/// Story 8.8 / AC1 (G7) — why a cross-Host frame is treated as **unclassified**
/// (and therefore denied under fail-closed mode rather than silently downgraded
/// to the coarse 3-band projection). Serialized into the `CODE_CONSENT_UNCLASSIFIED`
/// NACK `data.reason` so an operator sees in the Transparency Log *why* a frame
/// was rejected — fail-closed AND legible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnclassifiedReason {
    /// No `consent_envelope`, or an envelope whose `intent_class` is `None`.
    Absent,
    /// An `intent_class` present but failing the canonical grammar
    /// `^[a-z0-9]+(-[a-z0-9]+)*(:[a-z0-9]+(-[a-z0-9]+)*)?$` (`!A2AIntent::is_canonical`).
    NonCanonical,
    /// An `intent_class` longer than `MAX_CANONICAL_INTENT_LEN` (128 bytes).
    Oversized,
}

impl std::fmt::Display for UnclassifiedReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Story 8.8 — Display uses the SAME tokens as serde (snake_case) so
        // operators comparing human-readable logs to structured log fields see
        // identical tokens (was kebab-case, which caused a mismatch).
        let s = match self {
            UnclassifiedReason::Absent => "absent",
            UnclassifiedReason::NonCanonical => "non_canonical",
            UnclassifiedReason::Oversized => "oversized",
        };
        f.write_str(s)
    }
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

    /// Story 8.9 / G8 — the frame's self-asserted `from.host_id` does not match
    /// the TLS-verified peer identity. The receiver binds intake to the
    /// certificate-derived peer (NOT the attacker-controlled frame field), so a
    /// peer holding any one validly-pinned leaf cannot act as a confused deputy
    /// for another Host. `expected` is the TLS-verified peer; `asserted` is the
    /// forged wire value.
    #[error("peer identity mismatch: TLS-verified peer {expected}, frame asserted {asserted}")]
    PeerIdentityMismatch { expected: String, asserted: String },

    /// Story 8.9 / G1 — the consent envelope's `granter` does not match the
    /// frame's own `from` address. Closes stolen-envelope replay: an envelope
    /// granted by Host X, replayed inside a frame `from` Host Y, is denied.
    #[error("consent granter mismatch: envelope granter {granter}, frame from {frame_from}")]
    ConsentGranterMismatch { granter: String, frame_from: String },

    /// Story 8.8 / G7 — a cross-Host frame carries NO well-typed fine-grained
    /// `intent_class` (the [`UnclassifiedReason`]) and the router is fail-closed,
    /// so the send is REFUSED before the frame hits the wire. Distinct from
    /// [`A2AError::IntentDenied`] (-32001, classified-but-not-allowlisted): this
    /// is the never-silently-downgrade signal (the frame would otherwise have
    /// collapsed to the coarse 3-band projection). `direction` is always `Send`.
    #[error("cross-Host consent unclassified ({reason}) on {direction:?} — fail-closed deny")]
    ConsentUnclassified {
        direction: IntentDirection,
        reason: UnclassifiedReason,
    },

    /// Story 8.8 / G7 — the receiver fail-closed-denied an unclassified frame
    /// with `CODE_CONSENT_UNCLASSIFIED` (-32009); the sender reconstructs this
    /// typed mirror from the NACK (NOT conflated with `IntentDeniedAtPeer`).
    #[error("cross-Host consent unclassified ({reason}) at peer {peer} — fail-closed deny")]
    ConsentUnclassifiedAtPeer { peer: String, reason: UnclassifiedReason },

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
    #[error("a2a handshake failed: {class:?}: {message}")]
    HandshakeFailed {
        class: HandshakeFailureClass,
        message: String,
    },

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

/// Typed classification for handshake failures — replaces stringly-matched
/// sentinel tags with a compile-time discriminant (Story 8.9 / AC6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeFailureClass {
    CertExpired,
    CertNotValidYet,
    UnknownIssuer,
    BadCertificate,
    PinMismatch,
    Other,
}
