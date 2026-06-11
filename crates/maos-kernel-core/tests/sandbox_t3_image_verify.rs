//! Story 5.5a — T3 image-attestation signature-verification tests.
//!
//! Wired into CI as `.github/workflows/discipline.yml::nfr-sec-1-t3-image-signature`
//! (`cargo test -p maos-kernel-core --test sandbox_t3_image_verify --release`).
//!
//! The Story 5.5a spec (§AC "And integration test … covers") mandated this file;
//! it was specified but never authored when the story landed, so the CI job
//! referenced a non-existent test target. This restores the five mandated cases
//! against the real `verify_image_attestation` / `parse_signed_image_attestation`
//! pipeline at `security::sandbox::t3::image_verify`, using a controllable mock
//! `CryptoProvider` (mirrors the `revocation::parser` test pattern — the real
//! `ring` round-trip is covered in `security::crypto#tests`).

use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
use maos_domain::sandbox::{ImageAttestationId, T3Error, T3ImageAttestation, T3ImageEntry};
use maos_kernel_core::security::sandbox::t3::image_verify::{
    parse_signed_image_attestation, verify_image_attestation,
};

/// Controllable crypto mock: `verify_signature` returns `Ok` iff `verify_ok`.
/// The other trait methods are never exercised by the image-verify path.
struct MockCrypto {
    verify_ok: bool,
}

impl CryptoProvider for MockCrypto {
    fn verify_signature(&self, _pk: &[u8], _msg: &[u8], _sig: &[u8]) -> Result<(), CryptoError> {
        if self.verify_ok {
            Ok(())
        } else {
            Err(CryptoError::SignatureInvalid)
        }
    }

    fn seal_for_export(
        &self,
        _key: &[u8],
        _nonce: &[u8],
        _aad: &[u8],
        _plaintext: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::OperationFailed("unused in image-verify tests"))
    }

    fn sign_capability_token(
        &self,
        _signing_key: &[u8],
        _token_bytes: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::OperationFailed("unused in image-verify tests"))
    }
}

const SIGNER_PUB: [u8; 32] = [7u8; 32];

/// A well-formed, validly-constructed attestation (schema 1, one entry,
/// non-zero signature + signer pubkey).
fn valid_attestation() -> T3ImageAttestation {
    let entry = T3ImageEntry::new(
        "registry.example/distroless@sha256:deadbeef",
        [0x11; 32],
        "default distroless image",
        true,
    )
    .expect("well-formed entry");
    T3ImageAttestation::new(
        ImageAttestationId([0xAB; 32]),
        1,
        1_700_000_000_000_000_000,
        vec![entry],
        [1u8; 64],
        SIGNER_PUB,
    )
    .expect("well-formed attestation")
}

#[test]
fn verify_well_formed_attestation_succeeds() {
    let att = valid_attestation();
    let crypto = MockCrypto { verify_ok: true };
    verify_image_attestation(&att, &SIGNER_PUB, &crypto).expect("happy path must verify");
}

#[test]
fn verify_signature_mismatch_returns_signature_invalid() {
    let att = valid_attestation();
    // Crypto provider reports the signature does not verify.
    let crypto = MockCrypto { verify_ok: false };
    let err = verify_image_attestation(&att, &SIGNER_PUB, &crypto).unwrap_err();
    assert!(
        matches!(err, T3Error::SignatureInvalid),
        "expected SignatureInvalid, got {err:?}"
    );
}

#[test]
fn verify_trust_anchor_mismatch_returns_trust_anchor_mismatch() {
    let att = valid_attestation();
    let crypto = MockCrypto { verify_ok: true };
    // Trust anchor differs from the attestation's signer_pub_key.
    let wrong_anchor = [9u8; 32];
    let err = verify_image_attestation(&att, &wrong_anchor, &crypto).unwrap_err();
    assert!(
        matches!(err, T3Error::TrustAnchorMismatch),
        "expected TrustAnchorMismatch, got {err:?}"
    );
}

#[test]
fn verify_unsupported_schema_version_returns_unsupported() {
    let mut att = valid_attestation();
    att.schema_version = 2;
    let crypto = MockCrypto { verify_ok: true };
    let err = verify_image_attestation(&att, &SIGNER_PUB, &crypto).unwrap_err();
    assert!(
        matches!(err, T3Error::UnsupportedSchemaVersion { version: 2 }),
        "expected UnsupportedSchemaVersion {{ version: 2 }}, got {err:?}"
    );
}

#[test]
fn verify_empty_entries_returns_signature_invalid() {
    let mut att = valid_attestation();
    att.entries.clear();
    let crypto = MockCrypto { verify_ok: true };
    let err = verify_image_attestation(&att, &SIGNER_PUB, &crypto).unwrap_err();
    assert!(
        matches!(err, T3Error::SignatureInvalid),
        "expected SignatureInvalid for empty entries, got {err:?}"
    );
}

#[test]
fn parse_signed_attestation_round_trips_from_bytes() {
    // The bytes path (`parse_signed_image_attestation`) decodes JSON, applies the
    // same guards, then verifies via the crypto provider.
    let att = valid_attestation();
    let bytes = serde_json::to_vec(&att).expect("serialize attestation");
    let crypto = MockCrypto { verify_ok: true };
    let parsed =
        parse_signed_image_attestation(&bytes, &SIGNER_PUB, &crypto).expect("round-trip parse");
    assert_eq!(parsed.signer_pub_key, SIGNER_PUB);
    assert_eq!(parsed.entries.len(), 1);
}

#[test]
fn parse_signed_attestation_rejects_unsupported_schema_from_bytes() {
    let mut att = valid_attestation();
    att.schema_version = 2;
    let bytes = serde_json::to_vec(&att).expect("serialize attestation");
    let crypto = MockCrypto { verify_ok: true };
    let err = parse_signed_image_attestation(&bytes, &SIGNER_PUB, &crypto).unwrap_err();
    assert!(
        matches!(err, T3Error::UnsupportedSchemaVersion { version: 2 }),
        "expected UnsupportedSchemaVersion {{ version: 2 }}, got {err:?}"
    );
}
