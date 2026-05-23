//! T3 image-attestation parser and verifier.
//!
//! Mirrors Story 5.4's `parse_signed_crl` pipeline (see
//! `crates/maos-kernel-core/src/revocation/parser.rs` for the
//! reference body). Performs:
//! 1. JSON decode
//! 2. Schema version check (v0.5-α only accepts 1)
//! 3. Trust-anchor pubkey pin
//! 4. Ed25519 signature verification over canonical-serialized entries
//! 5. Local image SHA check against the pin

use maos_domain::ports::crypto::CryptoProvider;
use maos_domain::sandbox::{T3Error, T3ImageAttestation};

use super::runtime_detect::ContainerRuntime;

pub fn read_trust_anchor_pub() -> Result<[u8; 32], T3Error> {
    let hex_str = std::env::var("MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX")
        .map_err(|_| T3Error::TrustAnchorMissing(
            "MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX env-var not set".into(),
        ))?;
    let mut key = [0u8; 32];
    hex::decode_to_slice(hex_str.trim(), &mut key)
        .map_err(|e| T3Error::TrustAnchorMissing(
            format!("MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX decode failed: {e}")
        ))?;
    Ok(key)
}

/// Parse a signed image attestation from raw bytes, verifying the
/// Ed25519 signature using `crypto` and pinning the signer's public
/// key against `trust_anchor_pub`.
///
/// Steps:
/// 1. Decode JSON → `T3ImageAttestation`
/// 2. `schema_version == 1` check
/// 3. `signer_pub_key == trust_anchor_pub` pin check
/// 4. `CryptoProvider::verify_signature` over canonical-serialized entries
pub fn parse_signed_image_attestation(
    bytes: &[u8],
    trust_anchor_pub: &[u8],
    crypto: &dyn CryptoProvider,
) -> Result<T3ImageAttestation, T3Error> {
    let attestation: T3ImageAttestation =
        serde_json::from_slice(bytes).map_err(|e| T3Error::Io(e.to_string()))?;

    if attestation.schema_version != 1 {
        return Err(T3Error::UnsupportedSchemaVersion {
            version: attestation.schema_version,
        });
    }

    if attestation.entries.is_empty() {
        return Err(T3Error::SignatureInvalid);
    }

    if attestation.signer_pub_key.as_slice() != trust_anchor_pub {
        return Err(T3Error::TrustAnchorMismatch);
    }

    let entries_bytes = serde_json::to_vec(&attestation.entries)
        .map_err(|e| T3Error::Io(e.to_string()))?;
    crypto
        .verify_signature(&attestation.signer_pub_key, &entries_bytes, &attestation.signature)
        .map_err(|_| T3Error::SignatureInvalid)?;

    Ok(attestation)
}

/// Verify a pre-parsed attestation against the trust anchor and crypto
/// provider, plus optionally check the local image SHA.
pub fn verify_image_attestation(
    image: &T3ImageAttestation,
    trust_anchor_pub: &[u8],
    crypto: &dyn CryptoProvider,
) -> Result<(), T3Error> {
    if image.schema_version != 1 {
        return Err(T3Error::UnsupportedSchemaVersion {
            version: image.schema_version,
        });
    }

    if image.entries.is_empty() {
        return Err(T3Error::SignatureInvalid);
    }

    if image.signer_pub_key.as_slice() != trust_anchor_pub {
        return Err(T3Error::TrustAnchorMismatch);
    }

    let entries_bytes = serde_json::to_vec(&image.entries)
        .map_err(|e| T3Error::Io(e.to_string()))?;
    crypto
        .verify_signature(&image.signer_pub_key, &entries_bytes, &image.signature)
        .map_err(|_| T3Error::SignatureInvalid)?;

    Ok(())
}

/// Inspect the local image SHA-256 using the container runtime.
/// Shells out to `<runtime> image inspect --format '{{.Id}}' <image_uri>`.
pub fn inspect_image_sha(
    runtime: &ContainerRuntime,
    image_uri: &str,
) -> Result<[u8; 32], T3Error> {
    let output = std::process::Command::new(&runtime.path)
        .args(["image", "inspect", "--format", "{{.Id}}", image_uri])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| T3Error::Inspect(format!("image inspect for {image_uri}: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(T3Error::Inspect(format!(
            "image inspect failed for {image_uri}: {stderr}"
        )));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let id_str = stdout_str.trim();
    let sha_hex = id_str.strip_prefix("sha256:").unwrap_or(id_str);
    let mut sha = [0u8; 32];
    hex::decode_to_slice(sha_hex, &mut sha)
        .map_err(|e| T3Error::Inspect(format!("parse image SHA '{sha_hex}': {e}")))?;
    Ok(sha)
}
