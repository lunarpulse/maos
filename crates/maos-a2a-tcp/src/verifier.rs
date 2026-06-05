//! `TofuPinningVerifier` — the named deliverable that bridges WebPKI validation
//! into `maos_a2a_core`'s TOFU `verify_pinned` on the REAL mTLS handshake
//! (Story 8.6 AC-A3).
//!
//! It implements BOTH rustls verifier directions — `ServerCertVerifier` (dialing
//! side, validating the server's leaf) and `ClientCertVerifier` (listening side,
//! validating the client's leaf) — and BOTH pin. Each `verify_*_cert`:
//!
//!   1. **WebPKI FIRST** — well-formed + in-validity (+ chain-to-root in
//!      `ca_roots = Some` mode); an expired / malformed / untrusted-CA cert is
//!      rejected here, BEFORE the pin is ever consulted (preserves the
//!      `CERTIFICATE_EXPIRED` retry path, AC-T5, and makes AC-T4's
//!      "untrusted-CA rejected even if its fingerprint were pinned" oracle
//!      constructible).
//!   2. **THEN TOFU pin** — SHA-256 of the leaf is looked up against the
//!      `InMemoryTofuPinStore` via the synchronous bridge
//!      `find_active_pin_by_fingerprint` (the `async verify_pinned` body is
//!      non-blocking in-memory; we read the same `DashMap` synchronously rather
//!      than change the frozen signature — Dev Notes bridge option (a)).
//!
//! `ca_roots` (LOCKED Option A): `Some(roots)` ⇒ chain-to-root then pin
//! (defense-in-depth, the default); `None` ⇒ validity/structure-only (leaf as
//! its own trust anchor) then pin (the FR23a self-signed posture). In BOTH
//! branches the WebPKI step runs FIRST and the pin runs ONLY on its success —
//! `None` is NOT a `danger_accept_any` noop (AC-A3 HARD CONDITION 1).

use crate::error::TcpTransportError;
use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::InMemoryTofuPinStore;
use rustls::crypto::{verify_tls12_signature, verify_tls13_signature, WebPkiSupportedAlgorithms};
use rustls::pki_types::{CertificateDer, ServerName, TrustAnchor, UnixTime};
use rustls::{DigitallySignedStruct, Error as RustlsError, SignatureScheme};
use std::sync::Arc;
use webpki::{anchor_from_trusted_cert, EndEntityCert, KeyUsage};

/// Which trust posture the verifier enforces in its WebPKI prelude
/// (AC-A5 `ca_roots`). Both run validity/structure; `ChainToRoots` additionally
/// requires the leaf to chain to one of the configured roots.
#[derive(Clone)]
pub enum TrustPosture {
    /// `ca_roots = Some(bundle)` — DEFAULT: full chain-to-root, defense-in-depth.
    ChainToRoots(Arc<Vec<CertificateDer<'static>>>),
    /// `ca_roots = None` — pin-only (FR23a self-signed): validity + structure
    /// only, leaf treated as its own trust anchor. Still NOT fail-open.
    LeafSelfAnchor,
}

/// The connection direction this verifier instance guards — only affects the
/// WebPKI `KeyUsage` (server_auth when validating a server cert from the dialer,
/// client_auth when validating a client cert on the listener).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VerifyDirection {
    /// Dialing side, verifying the SERVER's leaf (server_auth).
    Server,
    /// Listening side, verifying the CLIENT's leaf (client_auth).
    Client,
}

/// The shared verifier. One instance is wrapped per `ClientConfig` /
/// `ServerConfig` via `.dangerous().with_custom_certificate_verifier(...)`.
#[derive(Clone)]
pub struct TofuPinningVerifier {
    pins: Arc<InMemoryTofuPinStore>,
    posture: TrustPosture,
    direction: VerifyDirection,
    /// The peer this connection is EXPECTED to be. `Some(peer)` on the dialing
    /// side (the `route_outbound` target is known up front) scopes the pin check
    /// to THAT peer via `verify_pinned_sync(peer, observed)` — matching the
    /// frozen `verify_pinned(peer, observed)` contract — so a leaf pinned for a
    /// DIFFERENT peer cannot satisfy this connection (cross-peer impersonation in
    /// the 3-host mesh, Story 8.6 review). `None` on the listening side, where
    /// the peer identity is LEARNED from the presented cert (flat TOFU lookup is
    /// the correct first-contact behavior there).
    expected_peer: Option<PeerId>,
    /// H2 — pinned validation clock. `Some(T0)` in tests (rustls's `now`
    /// param is ignored so cert validity is judged against the SAME `T0` as the
    /// rotation drill); `None` in production (use rustls's supplied `now`).
    validation_time: Option<UnixTime>,
    sig_algs: WebPkiSupportedAlgorithms,
}

impl std::fmt::Debug for TofuPinningVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TofuPinningVerifier")
            .field("direction", &(self.direction == VerifyDirection::Server))
            .field("chain_mode", &matches!(self.posture, TrustPosture::ChainToRoots(_)))
            .finish()
    }
}

impl TofuPinningVerifier {
    pub fn new(
        pins: Arc<InMemoryTofuPinStore>,
        posture: TrustPosture,
        direction: VerifyDirection,
        expected_peer: Option<PeerId>,
        validation_time: Option<UnixTime>,
    ) -> Self {
        let sig_algs = rustls::crypto::ring::default_provider()
            .signature_verification_algorithms;
        Self {
            pins,
            posture,
            direction,
            expected_peer,
            validation_time,
            sig_algs,
        }
    }

    fn now(&self, rustls_now: UnixTime) -> UnixTime {
        self.validation_time.unwrap_or(rustls_now)
    }

    /// Step 1 + 2: WebPKI-first, then TOFU pin. Returns `Ok(())` only if BOTH
    /// pass, in that order. Shared by both verifier directions so the ordering
    /// invariant (AC-A3 HARD CONDITION 3) holds identically on each side.
    fn verify_webpki_then_pin(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        now: UnixTime,
        usage: KeyUsage,
    ) -> Result<(), RustlsError> {
        // ---- Step 1: WebPKI (validity/structure [+ chain in Some mode]) ----
        let ee = EndEntityCert::try_from(end_entity).map_err(|e| {
            reject("BAD_CERTIFICATE", format!("malformed leaf: {e:?}"))
        })?;

        // Build the trust anchors for this posture. `LeafSelfAnchor` (None mode)
        // anchors the leaf to ITSELF — webpki then checks the self-signature,
        // structure, and validity window (notBefore/notAfter) against `now`,
        // with no chain. NOT a noop: an expired/malformed leaf is rejected here.
        let anchors: Vec<TrustAnchor<'_>> = match &self.posture {
            TrustPosture::ChainToRoots(roots) => roots
                .iter()
                .filter_map(|der| anchor_from_trusted_cert(der).ok())
                .collect(),
            TrustPosture::LeafSelfAnchor => {
                let anchor = anchor_from_trusted_cert(end_entity).map_err(|e| {
                    reject("BAD_CERTIFICATE", format!("leaf not a valid anchor: {e:?}"))
                })?;
                vec![anchor]
            }
        };
        if anchors.is_empty() {
            return Err(reject(
                "UNTRUSTED_ISSUER",
                "no trust anchors configured for chain validation".into(),
            ));
        }

        ee.verify_for_usage(
            self.sig_algs.all,
            &anchors,
            intermediates,
            now,
            usage,
            None,
            None,
        )
        .map_err(map_webpki_error)?;

        // ---- Step 2: TOFU pin (runs ONLY on WebPKI success) ----
        let observed = PeerCertFingerprint::from_cert_der(end_entity.as_ref());
        match &self.expected_peer {
            // Dial side: the target peer is known — scope the pin check to THAT
            // peer (frozen `verify_pinned` contract) so a leaf pinned for a
            // different peer cannot impersonate the one we dialed.
            Some(peer) => self
                .pins
                .verify_pinned_sync(peer, &observed)
                .map_err(|e| reject("PIN_MISMATCH", e.to_string())),
            // Listen side: peer identity is learned from the cert — accept any
            // active pin (TOFU first-contact). The receiver-side allowlist/consent
            // checks in `handle_intake` bind the wire identity downstream.
            None => match self.pins.find_active_pin_by_fingerprint(&observed) {
                Some(_peer) => Ok(()),
                None => Err(reject(
                    "PIN_MISMATCH",
                    format!(
                        "no active TOFU pin matches observed leaf fingerprint {}",
                        observed.wire()
                    ),
                )),
            },
        }
    }
}

/// Build a tagged `rustls::Error` whose Display carries a stable sentinel so the
/// dialer can classify it via [`TcpTransportError::classify_handshake`]
/// regardless of rustls's own wording (AC-A3 shared taxonomy).
fn reject(tag: &str, detail: String) -> RustlsError {
    RustlsError::InvalidCertificate(rustls::CertificateError::Other(rustls::OtherError(
        Arc::new(TcpTransportError::classify_handshake(&format!("{tag}: {detail}"))),
    )))
}

/// Map a webpki error into a sentinel-tagged rustls error. Uses a `Debug`
/// substring match so it is robust across rustls-webpki minor variant shapes.
fn map_webpki_error(e: webpki::Error) -> RustlsError {
    let dbg = format!("{e:?}");
    if dbg.contains("Expired") {
        reject("CERT_EXPIRED", dbg)
    } else if dbg.contains("NotValidYet") || dbg.contains("NotYetValid") {
        reject("NOT_YET_VALID", dbg)
    } else if dbg.contains("UnknownIssuer") {
        reject("UNTRUSTED_ISSUER", dbg)
    } else {
        reject("BAD_CERTIFICATE", dbg)
    }
}

impl rustls::client::danger::ServerCertVerifier for TofuPinningVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, RustlsError> {
        // Identity is anchored on the TOFU pin (the operator-named fingerprint),
        // not the SAN — `_server_name` is intentionally unused.
        self.verify_webpki_then_pin(
            end_entity,
            intermediates,
            self.now(now),
            KeyUsage::server_auth(),
        )?;
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, RustlsError> {
        verify_tls12_signature(message, cert, dss, &self.sig_algs)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, RustlsError> {
        verify_tls13_signature(message, cert, dss, &self.sig_algs)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.sig_algs.supported_schemes()
    }
}

impl rustls::server::danger::ClientCertVerifier for TofuPinningVerifier {
    fn root_hint_subjects(&self) -> &[rustls::DistinguishedName] {
        // No hint subjects — the client presents its pre-paired leaf; the pin is
        // the trust anchor, not a CA-derived hint.
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        now: UnixTime,
    ) -> Result<rustls::server::danger::ClientCertVerified, RustlsError> {
        self.verify_webpki_then_pin(
            end_entity,
            intermediates,
            self.now(now),
            KeyUsage::client_auth(),
        )?;
        Ok(rustls::server::danger::ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, RustlsError> {
        verify_tls12_signature(message, cert, dss, &self.sig_algs)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, RustlsError> {
        verify_tls13_signature(message, cert, dss, &self.sig_algs)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.sig_algs.supported_schemes()
    }
}
