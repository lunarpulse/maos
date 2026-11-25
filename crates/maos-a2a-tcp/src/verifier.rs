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

use crate::error::{TcpTransportError, RETIRED_KEY_CANDIDATE};
use crate::san;
use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::{EPinMismatch, InMemoryTofuPinStore};
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
    /// Story 23-7 / D-23-7-06 A — `Some(trust_domain)` only on the listening side under
    /// `peer_san = "required"`: the client leaf's SPIFFE URI SAN selects the pin record the
    /// fingerprint is verified against. `None` keeps the flat listen-side lookup.
    san_trust_domain: Option<Arc<str>>,
    sig_algs: WebPkiSupportedAlgorithms,
}

impl std::fmt::Debug for TofuPinningVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TofuPinningVerifier")
            .field("direction", &(self.direction == VerifyDirection::Server))
            .field(
                "chain_mode",
                &matches!(self.posture, TrustPosture::ChainToRoots(_)),
            )
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
        let sig_algs = rustls::crypto::ring::default_provider().signature_verification_algorithms;
        Self {
            pins,
            posture,
            direction,
            expected_peer,
            validation_time,
            san_trust_domain: None,
            sig_algs,
        }
    }

    /// Story 23-7 — scope the LISTEN side to the peer the client leaf's SAN names (`required`).
    #[must_use]
    pub fn with_peer_san(mut self, trust_domain: Option<Arc<str>>) -> Self {
        self.san_trust_domain = trust_domain;
        self
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
        webpki_prelude(
            &self.posture,
            &self.sig_algs,
            end_entity,
            intermediates,
            now,
            usage,
        )?;

        // ---- Step 2: TOFU pin (runs ONLY on WebPKI success) ----
        let observed = PeerCertFingerprint::from_cert_der(end_entity.as_ref());
        match &self.expected_peer {
            // Dial side: the target peer is known — scope the pin check to THAT
            // peer (frozen `verify_pinned` contract) so a leaf pinned for a
            // different peer cannot impersonate the one we dialed.
            Some(peer) => self
                .pins
                .verify_pinned_sync(peer, &observed)
                .map_err(|e| self.pin_refusal(peer, &e)),
            // Listen side, `peer_san = "off"`: peer identity is learned from the cert —
            // accept any active pin (TOFU first-contact). The receiver-side allowlist/consent
            // checks in `handle_intake` bind the wire identity downstream.
            None => match &self.san_trust_domain {
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
                // Listen side, `required`: the SAN SELECTS the record, the fingerprint
                // DECIDES. A missing, doubled or foreign SAN names no subject.
                Some(domain) => {
                    let subject =
                        san::peer_from_leaf(end_entity.as_ref(), domain).map_err(|refusal| {
                            reject("PIN_MISMATCH", format!("SAN_REFUSED: {refusal}"))
                        })?;
                    self.pins
                        .verify_pinned_sync(&subject, &observed)
                        .map_err(|e| self.pin_refusal(&subject, &e))
                }
            },
        }
    }

    /// The refusal for a failed pin check against `subject`. Story 23-7 (review A3): it carries
    /// [`RETIRED_KEY_CANDIDATE`] only when ONE read of the pin state shows a plain fingerprint
    /// `Mismatch` on a promoted subject AND the leaf chained to the configured roots. An
    /// invalidated pin (`Invalidated`: a restart awaiting re-pin approval, where the peer may be
    /// presenting its current leaf) and every `ca_roots`-unset handshake (the leaf is its own
    /// anchor, so anyone can present one whose SAN names the subject) stay untyped.
    fn pin_refusal(&self, subject: &PeerId, error: &EPinMismatch) -> RustlsError {
        let retired_key_candidate = matches!(error, EPinMismatch::Mismatch { .. })
            && matches!(self.posture, TrustPosture::ChainToRoots(_))
            && self.pins.is_promoted(subject);
        let detail = if retired_key_candidate {
            format!("{error} [{RETIRED_KEY_CANDIDATE}]")
        } else {
            error.to_string()
        };
        reject_with(Some(subject.clone()), "PIN_MISMATCH", detail)
    }
}

/// Step 1 of every handshake verification, as a function of the posture alone: validity,
/// structure, `usage` (EKU) and — under `ChainToRoots` — the chain to a configured root, using
/// `intermediates` as the presented chain. `LeafSelfAnchor` anchors the leaf to itself, so the
/// leaf's signature, structure, validity and EKU are judged and no chain exists to judge.
fn webpki_prelude(
    posture: &TrustPosture,
    sig_algs: &WebPkiSupportedAlgorithms,
    end_entity: &CertificateDer<'_>,
    intermediates: &[CertificateDer<'_>],
    now: UnixTime,
    usage: KeyUsage,
) -> Result<(), RustlsError> {
    let ee = EndEntityCert::try_from(end_entity)
        .map_err(|e| reject("BAD_CERTIFICATE", format!("malformed leaf: {e:?}")))?;

    // `LeafSelfAnchor` (None mode) anchors the leaf to ITSELF — webpki then checks the
    // self-signature, structure, and validity window (notBefore/notAfter) against `now`, with no
    // chain. NOT a noop: an expired/malformed leaf is rejected here.
    let anchors: Vec<TrustAnchor<'_>> = match posture {
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
        sig_algs.all,
        &anchors,
        intermediates,
        now,
        usage,
        None,
        None,
    )
    .map(|_| ())
    .map_err(map_webpki_error)
}

/// Story 23-7 review B3 — judge this host's OWN `chain` exactly as a peer's verifier will judge
/// it when it is presented: for BOTH usages (`serverAuth` when a peer dials us, `clientAuth` when
/// we dial a peer), with `chain[1..]` as the presented intermediates, against the same posture.
/// A leaf whose intermediate has not been written yet fails here, not in every peer's handshake.
pub(crate) fn check_own_chain(
    posture: &TrustPosture,
    chain: &[CertificateDer<'_>],
    now: UnixTime,
) -> Result<(), TcpTransportError> {
    let sig_algs = rustls::crypto::ring::default_provider().signature_verification_algorithms;
    let (Some(leaf), intermediates) = (chain.first(), chain.get(1..).unwrap_or_default()) else {
        return Err(TcpTransportError::OwnIdentityRefused(
            "the chain holds no certificate".into(),
        ));
    };
    for (name, usage) in [
        ("serverAuth", KeyUsage::server_auth()),
        ("clientAuth", KeyUsage::client_auth()),
    ] {
        webpki_prelude(posture, &sig_algs, leaf, intermediates, now, usage).map_err(|error| {
            TcpTransportError::OwnIdentityRefused(format!(
                "the own chain does not verify for {name}: {error}"
            ))
        })?;
    }
    Ok(())
}

/// Story 23-7 / D-23-7-07 A — the typed carrier a handshake refusal travels in: the classified
/// error plus the subject peer the verifier scoped the check to (the dial target, or the peer a
/// listen-side SAN named). `Display` is the classified error's, so every string classification
/// stays byte for byte.
#[derive(Debug)]
pub struct SubjectRefusal {
    pub subject: Option<PeerId>,
    pub error: TcpTransportError,
}

impl std::fmt::Display for SubjectRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.error, f)
    }
}

impl std::error::Error for SubjectRefusal {}

/// The subject a verifier refusal carried, read back from the I/O error a failed handshake
/// surfaces (`tokio-rustls` wraps the `rustls::Error` in an `io::Error`).
pub fn refusal_subject(error: &std::io::Error) -> Option<PeerId> {
    let rustls_error = error.get_ref()?.downcast_ref::<RustlsError>()?;
    let RustlsError::InvalidCertificate(rustls::CertificateError::Other(other)) = rustls_error
    else {
        return None;
    };
    other.0.downcast_ref::<SubjectRefusal>()?.subject.clone()
}

/// Build a tagged `rustls::Error` whose Display carries a stable sentinel so the
/// dialer can classify it via [`TcpTransportError::classify_handshake`]
/// regardless of rustls's own wording (AC-A3 shared taxonomy).
fn reject(tag: &str, detail: String) -> RustlsError {
    reject_with(None, tag, detail)
}

fn reject_with(subject: Option<PeerId>, tag: &str, detail: String) -> RustlsError {
    RustlsError::InvalidCertificate(rustls::CertificateError::Other(rustls::OtherError(
        Arc::new(SubjectRefusal {
            subject,
            error: TcpTransportError::classify_handshake(&format!("{tag}: {detail}")),
        }),
    )))
}

/// Map a webpki error into a sentinel-tagged rustls error.
///
/// Story 8.9 / AC6.1 (G4): classify on the `webpki::Error` TYPED variants, not a
/// `format!("{e:?}").contains(...)` substring of the Debug rendering. The
/// sentinel taxonomy (`CERT_EXPIRED` / `NOT_YET_VALID` / `UNTRUSTED_ISSUER` /
/// `BAD_CERTIFICATE`) is preserved byte-for-byte so `classify_handshake`
/// downstream — and the AC-T4/AC-T5 oracles — see the same classes. `Error` is
/// `#[non_exhaustive]`; the wildcard maps any future/unhandled variant to the
/// conservative `BAD_CERTIFICATE`.
fn map_webpki_error(e: webpki::Error) -> RustlsError {
    use webpki::Error as WE;
    let tag = match e {
        WE::CertExpired { .. } => "CERT_EXPIRED",
        WE::CertNotValidYet { .. } => "NOT_YET_VALID",
        // Untrusted/unknown issuer and a bad issuer signature are both chain-layer
        // "this leaf does not chain to a trusted root" rejections (AC-T4).
        WE::UnknownIssuer | WE::InvalidSignatureForPublicKey => "UNTRUSTED_ISSUER",
        _ => "BAD_CERTIFICATE",
    };
    reject(tag, format!("{e:?}"))
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
