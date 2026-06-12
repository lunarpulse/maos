//! `TcpA2AConfig` — operator cert/PKI provisioning + config schema for the live
//! cross-Host transport (Story 8.6 AC-A5).
//!
//! Fields are EXACTLY those the epic enumerates; `#[serde(deny_unknown_fields)]`
//! is consistent with `maos_a2a_core::config::A2AConfig` (no silent acceptance
//! of operator typos).

use crate::error::TcpTransportError;
use crate::verifier::TrustPosture;
use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::{InMemoryTofuPinStore, TofuPinStore};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// A pre-paired peer leaf-cert fingerprint (ADR-012 "pre-paired fingerprints"
/// made real). Loaded into the `InMemoryTofuPinStore` at startup so the TOFU
/// verifier has a trust anchor before the first handshake. Reuses
/// `PeerCertFingerprint` (`maos_a2a_core::identity`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedFingerprint {
    /// Operator-declared peer identity (e.g. `host_a` / `host_b`). Maps to the
    /// `HostId`↔fingerprint keying used by the loopback router.
    pub peer_id: PeerId,
    /// The peer's pinned SHA-256 leaf-cert fingerprint.
    pub fingerprint: PeerCertFingerprint,
    /// The peer Spirit's boot-nonce at pin time (NFR-Rel-6 restart detection).
    /// REQUIRED (review patch P2): a `#[serde(default)] = 0` here was a footgun —
    /// a peer pinned with `0` triggers a spurious `CODE_SPIRIT_RESTART_DETECTED`
    /// on the first real non-zero-nonce frame (self-DoS), and a mutual `0`
    /// silently disables restart detection. Operators MUST pre-pair the nonce.
    pub boot_nonce: u64,
}

/// Serde helper: (de)serialize a `Duration` as whole seconds, so operator TOML
/// reads `handshake_timeout = 30`.
mod duration_secs {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(d.as_secs())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let secs = u64::deserialize(d)?;
        Ok(Duration::from_secs(secs))
    }
}

fn default_handshake_timeout() -> Duration {
    Duration::from_secs(30)
}

/// Operator config for one live cross-Host TCP endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TcpA2AConfig {
    /// Listen address. `127.0.0.1:0` in tests (ephemeral port, readback via
    /// `local_addr` — H3).
    pub listen_addr: SocketAddr,
    /// This Host's own mTLS cert chain (PEM).
    pub own_cert_chain: PathBuf,
    /// This Host's own private key (PKCS#8 PEM).
    pub own_private_key: PathBuf,
    /// Pre-paired peer leaf-cert fingerprints, loaded into the TOFU pin store.
    pub peer_pins: Vec<PinnedFingerprint>,
    /// Handshake timeout (default 30s; injectable for tests — H5).
    #[serde(default = "default_handshake_timeout", with = "duration_secs")]
    pub handshake_timeout: Duration,
    /// WebPKI trust bundle (LOCKED Option A). `Some` ⇒ CA-chain-to-root THEN pin
    /// (defense-in-depth, the test/prod default); `None` ⇒ pin-only
    /// (leaf validity/structure THEN pin, the FR23a self-signed posture).
    #[serde(default)]
    pub ca_roots: Option<PathBuf>,
}

impl TcpA2AConfig {
    /// Build the [`TrustPosture`] this config selects (AC-A5 LOCKED Option A).
    /// `Some(bundle)` loads the roots and produces `ChainToRoots`; `None`
    /// produces `LeafSelfAnchor` (validity/structure-only — NOT fail-open).
    pub fn trust_posture(&self) -> Result<TrustPosture, TcpTransportError> {
        match &self.ca_roots {
            Some(path) => {
                let roots = load_certs(path)?;
                if roots.is_empty() {
                    return Err(TcpTransportError::Config(format!(
                        "ca_roots {} contained no certificates",
                        path.display()
                    )));
                }
                Ok(TrustPosture::ChainToRoots(Arc::new(roots)))
            }
            None => Ok(TrustPosture::LeafSelfAnchor),
        }
    }

    /// Load `own_cert_chain` + `own_private_key` from disk.
    pub fn load_identity(
        &self,
    ) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), TcpTransportError> {
        let chain = load_certs(&self.own_cert_chain)?;
        if chain.is_empty() {
            return Err(TcpTransportError::Config(format!(
                "own_cert_chain {} contained no certificates",
                self.own_cert_chain.display()
            )));
        }
        let key = load_private_key(&self.own_private_key)?;
        Ok((chain, key))
    }

    /// Materialize the TOFU pin store from `peer_pins` (makes ADR-012's
    /// "pre-paired fingerprints" real). Each pin is recorded as a first-contact
    /// pin so the synchronous verifier lookup succeeds before the handshake.
    pub async fn build_pin_store(&self) -> Result<Arc<InMemoryTofuPinStore>, TcpTransportError> {
        let store = Arc::new(InMemoryTofuPinStore::new());
        for pin in &self.peer_pins {
            store
                .pin_first_contact(
                    &pin.peer_id,
                    &pin.fingerprint,
                    &pin.fingerprint,
                    pin.boot_nonce,
                )
                .await
                .map_err(|e| {
                    TcpTransportError::Config(format!(
                        "failed to pin peer {}: {e}",
                        pin.peer_id.as_str()
                    ))
                })?;
        }
        Ok(store)
    }
}

/// Load a PEM file of one-or-more X.509 certificates into owned DER.
pub fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, TcpTransportError> {
    let bytes = std::fs::read(path)
        .map_err(|e| TcpTransportError::Config(format!("cannot read {}: {e}", path.display())))?;
    let mut reader = std::io::BufReader::new(&bytes[..]);
    let certs: Result<Vec<_>, _> = rustls_pemfile::certs(&mut reader).collect();
    certs.map_err(|e| TcpTransportError::Config(format!("bad PEM cert in {}: {e}", path.display())))
}

/// Clone a `PrivateKeyDer` (which does not impl `Clone`) via a secret-bytes
/// round-trip — needed because the key feeds BOTH the server config
/// (`with_single_cert`) and the client auth config (`with_client_auth_cert`).
pub fn clone_key(k: &PrivateKeyDer<'static>) -> PrivateKeyDer<'static> {
    match k {
        PrivateKeyDer::Pkcs1(d) => PrivateKeyDer::Pkcs1(d.secret_pkcs1_der().to_vec().into()),
        PrivateKeyDer::Sec1(d) => PrivateKeyDer::Sec1(d.secret_sec1_der().to_vec().into()),
        PrivateKeyDer::Pkcs8(d) => PrivateKeyDer::Pkcs8(d.secret_pkcs8_der().to_vec().into()),
        // `PrivateKeyDer` is `#[non_exhaustive]`. Fail CLOSED rather than silently
        // substitute an EMPTY key (which would build a degenerate TLS config that
        // appears to come up but cannot authenticate) — review patch P3. Today
        // `load_private_key` only yields Pkcs1/Sec1/Pkcs8, so this is unreachable;
        // a future rustls key type must be wired explicitly here.
        other => panic!(
            "clone_key: unsupported PrivateKeyDer variant {other:?}; refusing to substitute an empty key (fail-closed — wire this key type explicitly)"
        ),
    }
}

/// Load a PKCS#8 (or otherwise) PEM private key into an owned key.
pub fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, TcpTransportError> {
    let bytes = std::fs::read(path)
        .map_err(|e| TcpTransportError::Config(format!("cannot read {}: {e}", path.display())))?;
    let mut reader = std::io::BufReader::new(&bytes[..]);
    match rustls_pemfile::private_key(&mut reader) {
        Ok(Some(key)) => Ok(key),
        Ok(None) => Err(TcpTransportError::Config(format!(
            "no private key found in {}",
            path.display()
        ))),
        Err(e) => Err(TcpTransportError::Config(format!(
            "bad PEM key in {}: {e}",
            path.display()
        ))),
    }
}
