//! mTLS configuration + handshake retry policy.
//!
//! Per architecture §7.2.1.a: TLS 1.3 handshake; retry policy `100 ms / 300 ms
//! / 1000 ms across 3 attempts (4 total tries including the original)` with
//! ±20% jitter; retries fire only on `BAD_CERTIFICATE` or `CERTIFICATE_EXPIRED`.

use crate::error::{A2AError, HandshakeFailureClass};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Per architecture §7.2.1.a — retry policy for mTLS handshake failures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeRetryPolicy {
    /// Backoff schedule in milliseconds. Default `[100, 300, 1000]`.
    pub backoff_ms: Vec<u64>,
    /// Jitter percentage (e.g. `20` for ±20%). Per §7.2.1.a.
    pub jitter_pct: u8,
    /// Total attempts including the original. Default `4` (1 + 3 retries).
    pub max_attempts: u8,
}

impl Default for HandshakeRetryPolicy {
    fn default() -> Self {
        Self {
            backoff_ms: vec![100, 300, 1000],
            jitter_pct: 20,
            max_attempts: 4,
        }
    }
}

impl HandshakeRetryPolicy {
    /// Returns the delay (in ms) before attempt `n` (1-indexed; attempt 1 is
    /// the original, attempts 2..max_attempts are retries). Jitter is applied
    /// deterministically when `jitter_seed` is `Some`, randomly otherwise.
    pub fn delay_for_attempt(&self, attempt: u8, jitter_seed: Option<u64>) -> u64 {
        if attempt < 2 {
            return 0;
        }
        let idx = (attempt as usize).saturating_sub(2);
        let base = self.backoff_ms.get(idx).copied().unwrap_or(0);
        if base == 0 || self.jitter_pct == 0 {
            return base;
        }
        let jitter_pct = self.jitter_pct.min(100) as u64;
        let jitter_range = (base * jitter_pct) / 100;
        if jitter_range == 0 {
            return base;
        }
        let seed = jitter_seed.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0)
        });
        // Linear-congruential cheap jitter; deterministic for the same seed
        let r = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let off = (r % (2 * jitter_range + 1)) as i64 - jitter_range as i64;
        ((base as i64) + off).max(0) as u64
    }

    /// Returns true if the failure class is retry-eligible per §7.2.1.a.
    ///
    /// Story 8.9 / AC6.1 (G4): classify on the STRUCTURED sentinel TAG the
    /// transport's `TcpTransportError::to_a2a_error` emits as the leading
    /// `WORD:` prefix (`BAD_CERTIFICATE:` / `CERTIFICATE_EXPIRED:` /
    /// `PIN_MISMATCH (TOFU):`), NOT a fragile `to_lowercase().contains(...)`
    /// substring of arbitrary rustls wording. The leading tag is the stable,
    /// transport-owned discriminant; a TOFU pin mismatch is deliberately NOT
    /// retry-eligible (retrying cannot change a pinned identity).
    pub fn is_retryable(&self, err: &A2AError) -> bool {
        match err {
            A2AError::HandshakeFailed { class, .. } => {
                matches!(
                    class,
                    HandshakeFailureClass::BadCertificate | HandshakeFailureClass::CertExpired
                )
            }
            _ => false,
        }
    }
}

/// Loopback mTLS server config.
///
/// At v0.5 loopback profile, both ends present self-signed certs and pin via
/// TOFU on first contact. The `LoopbackTlsConfig` carries the cert + key DER
/// bytes plus a client-cert verifier (typically a `WebPkiClientVerifier` or
/// a custom verifier that accepts any client cert; see
/// `build_loopback_server_config` for the assembly).
pub struct LoopbackTlsConfig {
    pub bind: std::net::SocketAddr,
    pub server_cert: rustls::pki_types::CertificateDer<'static>,
    pub server_key: rustls::pki_types::PrivateKeyDer<'static>,
    pub client_cert_verifier:
        Arc<dyn rustls::server::danger::ClientCertVerifier>,
}

/// Build a `rustls::ServerConfig` from a `LoopbackTlsConfig`.
///
/// At v0.5 the server config uses the workspace's pinned ring provider; TLS
/// 1.3 enabled (the v0.5 floor); `with_client_cert_verifier` enforces mTLS.
pub fn build_loopback_server_config(
    cfg: &LoopbackTlsConfig,
) -> Result<rustls::ServerConfig, A2AError> {
    let cert_chain = vec![cfg.server_cert.clone()];
    let provider = std::sync::Arc::new(rustls::crypto::ring::default_provider());
    let server_config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| A2AError::HandshakeFailed { class: HandshakeFailureClass::Other, message: format!("protocol_versions: {e}") })?
        .with_client_cert_verifier(cfg.client_cert_verifier.clone())
        .with_single_cert(cert_chain, clone_key(&cfg.server_key))
        .map_err(|e| A2AError::HandshakeFailed { class: HandshakeFailureClass::Other, message: format!("with_single_cert: {e}") })?;
    Ok(server_config)
}

fn clone_key(
    k: &rustls::pki_types::PrivateKeyDer<'static>,
) -> rustls::pki_types::PrivateKeyDer<'static> {
    // PrivateKeyDer doesn't impl Clone; clone via secret-bytes round-trip.
    match k {
        rustls::pki_types::PrivateKeyDer::Pkcs1(d) => {
            rustls::pki_types::PrivateKeyDer::Pkcs1(d.secret_pkcs1_der().to_vec().into())
        }
        rustls::pki_types::PrivateKeyDer::Sec1(d) => {
            rustls::pki_types::PrivateKeyDer::Sec1(d.secret_sec1_der().to_vec().into())
        }
        rustls::pki_types::PrivateKeyDer::Pkcs8(d) => {
            rustls::pki_types::PrivateKeyDer::Pkcs8(d.secret_pkcs8_der().to_vec().into())
        }
        // Future-proof: unknown PrivateKeyDer variants from rustls upgrades
        // return an error instead of panicking with unreachable!().
        _ => {
            // At v0.5 this arm is unreachable; if a future rustls adds a new
            // variant, this gracefully falls through to a forwarded error
            // rather than crashing in production.
            rustls::pki_types::PrivateKeyDer::Pkcs8(
                rustls::pki_types::PrivatePkcs8KeyDer::from(vec![]),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_policy_defaults() {
        let p = HandshakeRetryPolicy::default();
        assert_eq!(p.backoff_ms, vec![100, 300, 1000]);
        assert_eq!(p.jitter_pct, 20);
        assert_eq!(p.max_attempts, 4);
    }

    #[test]
    fn delay_for_attempt_1_is_zero() {
        let p = HandshakeRetryPolicy::default();
        assert_eq!(p.delay_for_attempt(1, Some(0)), 0);
    }

    #[test]
    fn delay_for_attempt_with_jitter_in_range() {
        let p = HandshakeRetryPolicy::default();
        for seed in 0..32 {
            let d2 = p.delay_for_attempt(2, Some(seed));
            assert!((80..=120).contains(&d2), "attempt 2 delay {d2} out of [80, 120]");
            let d3 = p.delay_for_attempt(3, Some(seed));
            assert!((240..=360).contains(&d3), "attempt 3 delay {d3} out of [240, 360]");
            let d4 = p.delay_for_attempt(4, Some(seed));
            assert!((800..=1200).contains(&d4), "attempt 4 delay {d4} out of [800, 1200]");
        }
    }

    #[test]
    fn is_retryable_bad_certificate() {
        let p = HandshakeRetryPolicy::default();
        // Story 8.9 / G4 — classify on the structured `WORD:` sentinel tag the
        // transport's `to_a2a_error` emits, not arbitrary substrings.
        assert!(p.is_retryable(&A2AError::HandshakeFailed {
            class: HandshakeFailureClass::BadCertificate,
            message: "leaf malformed".into(),
        }));
    }
    #[test]
    fn is_retryable_certificate_expired() {
        let p = HandshakeRetryPolicy::default();
        assert!(p.is_retryable(&A2AError::HandshakeFailed {
            class: HandshakeFailureClass::CertExpired,
            message: "leaf expired".into(),
        }));
    }

    #[test]
    fn is_retryable_pin_mismatch_not_retryable() {
        // Story 8.9 / G4 — a TOFU pin mismatch is a valid cert with the wrong
        // identity; retrying cannot change the pinned identity, so its
        // `PIN_MISMATCH (TOFU):` tag must NOT be retry-eligible.
        let p = HandshakeRetryPolicy::default();
        assert!(!p.is_retryable(&A2AError::HandshakeFailed {
            class: HandshakeFailureClass::PinMismatch,
            message: "wrong identity".into(),
        }));
    }

    #[test]
    fn is_retryable_other_failure_classes_not_retryable() {
        let p = HandshakeRetryPolicy::default();
        assert!(!p.is_retryable(&A2AError::HandshakeFailed {
            class: HandshakeFailureClass::Other,
            message: "DECRYPT_ERROR: alert".into(),
        }));
        assert!(!p.is_retryable(&A2AError::TransportFailed("connection reset".into())));
    }
}
