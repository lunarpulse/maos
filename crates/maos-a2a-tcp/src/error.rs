//! Concrete error taxonomy for the live TCP/mTLS transport (Story 8.6 AC-A3
//! HARD CONDITION 2: a SHARED taxonomy that distinguishes
//! "TOFU-pin-mismatch" vs "bad-cert/untrusted-issuer" vs "expired/not-yet-valid"
//! identically in BOTH `ca_roots` postures, so the AC-T3 / AC-T4 / AC-T4b /
//! AC-T5 oracles never alias).

use maos_a2a_core::A2AError;
use std::fmt;

/// Classified transport-layer failure. The variants are the oracle classes the
/// security tests read; `classify` recovers them from a dial error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TcpTransportError {
    /// WebPKI succeeded (cert is well-formed, in-validity, and — in `Some` mode
    /// — chains to a trusted root) but the leaf fingerprint is NOT pinned, or is
    /// pinned to a different identity. The TOFU trust oracle (AC-T3/T4b/T6).
    TofuPinMismatch(String),
    /// WebPKI rejected the cert at the CHAIN layer: untrusted issuer / bad
    /// signature / malformed. Distinct from a pin mismatch (AC-T4 — only
    /// constructible in `ca_roots = Some` chain mode).
    BadCertificate(String),
    /// WebPKI rejected the cert at the VALIDITY layer: expired or not-yet-valid
    /// against the pinned `T0` clock. Retry-eligible (AC-T5).
    CertExpired(String),
    /// Generic mTLS handshake failure not otherwise classified.
    Handshake(String),
    /// A bounded timeout fired (handshake / intake / idle) — AC-T7.
    Timeout(String),
    /// An inbound frame exceeded the codec cap (AC-T8).
    FrameTooLarge(String),
    /// Socket / IO error (connection refused, reset, EOF mid-frame).
    Io(String),
    /// Operator config / provisioning error (bad PEM, missing peer endpoint).
    Config(String),
    /// JSON-RPC protocol error after a successful handshake (e.g. parse error).
    Protocol(String),
}

impl TcpTransportError {
    /// Is this a TOFU pin mismatch (the security model's primary negative)?
    pub fn is_tofu_mismatch(&self) -> bool {
        matches!(self, TcpTransportError::TofuPinMismatch(_))
    }

    /// Is this a chain/issuer/structure rejection (NOT a pin mismatch)?
    pub fn is_bad_cert(&self) -> bool {
        matches!(self, TcpTransportError::BadCertificate(_))
    }

    /// Is this an expiry / not-yet-valid rejection?
    pub fn is_cert_validity(&self) -> bool {
        matches!(self, TcpTransportError::CertExpired(_))
    }

    pub fn is_timeout(&self) -> bool {
        matches!(self, TcpTransportError::Timeout(_))
    }

    /// Map to the frozen `A2AError` surface (AC-A6: consumed unchanged). The
    /// message is shaped so `HandshakeRetryPolicy::is_retryable` fires ONLY on
    /// the cert-class errors (`BAD_CERTIFICATE` / `CERTIFICATE_EXPIRED`),
    /// preserving the AC-T5 retry path.
    pub fn to_a2a_error(&self) -> A2AError {
        match self {
            // A TOFU pin mismatch is a VALID cert with the wrong identity —
            // retrying cannot change the pinned identity, so it is deliberately
            // NOT cert-class-retryable (no `BAD_CERTIFICATE`/`CERTIFICATE_EXPIRED`
            // keyword): `HandshakeRetryPolicy::is_retryable` returns false here.
            TcpTransportError::TofuPinMismatch(m) => {
                A2AError::HandshakeFailed(format!("PIN_MISMATCH (TOFU): {m}"))
            }
            TcpTransportError::BadCertificate(m) => {
                A2AError::HandshakeFailed(format!("BAD_CERTIFICATE: {m}"))
            }
            TcpTransportError::CertExpired(m) => {
                A2AError::HandshakeFailed(format!("CERTIFICATE_EXPIRED: {m}"))
            }
            TcpTransportError::Handshake(m) => A2AError::HandshakeFailed(m.clone()),
            TcpTransportError::Timeout(m) => A2AError::TransportFailed(format!("timeout: {m}")),
            TcpTransportError::FrameTooLarge(m) => {
                A2AError::TransportFailed(format!("frame too large: {m}"))
            }
            TcpTransportError::Io(m) => A2AError::Io(m.clone()),
            TcpTransportError::Config(m) => A2AError::ConfigInvalid(m.clone()),
            TcpTransportError::Protocol(m) => A2AError::TransportFailed(m.clone()),
        }
    }

    /// Classify a rustls/io handshake error string into the oracle taxonomy.
    /// The `TofuPinningVerifier` tags its rejections with stable sentinels
    /// (`PIN_MISMATCH` / `UNTRUSTED_ISSUER` / `CERT_EXPIRED`) so this recovers
    /// the class deterministically from the surfaced error.
    pub fn classify_handshake(msg: &str) -> Self {
        let lower = msg.to_lowercase();
        if lower.contains("pin_mismatch") || lower.contains("pin mismatch") {
            TcpTransportError::TofuPinMismatch(msg.to_string())
        } else if lower.contains("cert_expired")
            || lower.contains("certificate_expired")
            || lower.contains("expired")
            || lower.contains("not_yet_valid")
            || lower.contains("notvalidyet")
            || lower.contains("not yet valid")
        {
            TcpTransportError::CertExpired(msg.to_string())
        } else if lower.contains("untrusted_issuer")
            || lower.contains("unknownissuer")
            || lower.contains("unknown issuer")
            || lower.contains("bad_signature")
            || lower.contains("badsignature")
            || lower.contains("invalid peer certificate")
            || lower.contains("invalidcertificate")
            || lower.contains("bad certificate")
        {
            TcpTransportError::BadCertificate(msg.to_string())
        } else {
            TcpTransportError::Handshake(msg.to_string())
        }
    }
}

impl fmt::Display for TcpTransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TcpTransportError::TofuPinMismatch(m) => write!(f, "TOFU pin mismatch: {m}"),
            TcpTransportError::BadCertificate(m) => write!(f, "bad certificate: {m}"),
            TcpTransportError::CertExpired(m) => write!(f, "certificate validity: {m}"),
            TcpTransportError::Handshake(m) => write!(f, "handshake failed: {m}"),
            TcpTransportError::Timeout(m) => write!(f, "timeout: {m}"),
            TcpTransportError::FrameTooLarge(m) => write!(f, "frame too large: {m}"),
            TcpTransportError::Io(m) => write!(f, "io error: {m}"),
            TcpTransportError::Config(m) => write!(f, "config error: {m}"),
            TcpTransportError::Protocol(m) => write!(f, "protocol error: {m}"),
        }
    }
}

impl std::error::Error for TcpTransportError {}

impl From<TcpTransportError> for A2AError {
    fn from(e: TcpTransportError) -> Self {
        e.to_a2a_error()
    }
}
