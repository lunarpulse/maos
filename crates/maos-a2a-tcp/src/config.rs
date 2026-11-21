//! `TcpA2AConfig` — operator cert/PKI provisioning + config schema for the live
//! cross-Host transport (Story 8.6 AC-A5).
//!
//! Fields are EXACTLY those the epic enumerates; `#[serde(deny_unknown_fields)]`
//! is consistent with `maos_a2a_core::config::A2AConfig` (no silent acceptance
//! of operator typos).

use crate::error::TcpTransportError;
use crate::verifier::TrustPosture;
use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::tofu::{PinPersistence, PinRecord, TofuPin};
use maos_a2a_core::InMemoryTofuPinStore;
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
    /// silently disables restart detection. Operators pre-pair the nonce here;
    /// when the peer's nonce differs (it restarted, or it booted after this
    /// host), the receiver invalidates the pin and the recovery is either a
    /// corrected value plus a restart, or operator consent on the running
    /// `cohort-a2a-daemon` via `maosctl a2a repin approve <peer> --boot-nonce <N>`
    /// — the latter keeps this fingerprint and re-arms only the nonce. The
    /// daemon persists that consent (and any promoted rotation) in
    /// `$MAOS_HOME/a2a-pins.json`; at boot this entry is the anchor the file is
    /// reconciled against, so a changed entry wins over the file and an entry
    /// the file has promoted past refuses the boot (see [`TcpA2AConfig::build_pin_store`]).
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
    /// "pre-paired fingerprints" real), reconciled with the durable record when
    /// a `persistence` port is given (Story 21-3 / D-21-3-02 anchor rule). Per
    /// configured peer: no record → seed from config; config equals the record's
    /// anchor and its fingerprint → restore the record; config equals the anchor
    /// but the record was promoted at runtime → typed boot refusal; config
    /// differs from the anchor (the operator re-paired) → config wins. Records
    /// for peers the config omits are dropped: the file never adds a
    /// fingerprint the config does not name. The result is written once.
    pub async fn build_pin_store(
        &self,
        persistence: Option<Arc<dyn PinPersistence>>,
    ) -> Result<Arc<InMemoryTofuPinStore>, TcpTransportError> {
        let mut records = persistence
            .as_ref()
            .map(|port| port.load())
            .unwrap_or_default();
        let mut store = InMemoryTofuPinStore::new();
        if let Some(port) = persistence {
            store = store.with_persistence(port);
        }
        let store = Arc::new(store);
        for pin in &self.peer_pins {
            let anchor = (pin.fingerprint.clone(), pin.boot_nonce);
            let seeded = |generation| PinRecord {
                pin: TofuPin {
                    peer: pin.peer_id.clone(),
                    fingerprint: pin.fingerprint.clone(),
                    boot_nonce: pin.boot_nonce,
                    pinned_at_ns: 0,
                    invalidated: None,
                    repin_approval_id: None,
                },
                anchor: anchor.clone(),
                generation,
                pending_observed: None,
            };
            let found = records
                .iter()
                .position(|record| record.pin.peer.as_str() == pin.peer_id.as_str());
            let record = match found.map(|index| records.swap_remove(index)) {
                Some(old) if old.anchor == anchor && old.pin.fingerprint == pin.fingerprint => old,
                Some(old) if old.anchor == anchor => {
                    return Err(TcpTransportError::PinPromotionNotCarried {
                        peer: pin.peer_id.as_str().to_string(),
                        configured: pin.fingerprint.wire(),
                        promoted: old.pin.fingerprint.wire(),
                    });
                }
                Some(old) => {
                    seeded(old.generation + u64::from(old.pin.fingerprint != pin.fingerprint))
                }
                None => seeded(0),
            };
            store.restore(record).map_err(|e| {
                TcpTransportError::Config(format!(
                    "failed to pin peer {}: {e}",
                    pin.peer_id.as_str()
                ))
            })?;
        }
        store.flush().map_err(|e| {
            TcpTransportError::Config(format!("cannot write the durable pin file: {e}"))
        })?;
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
