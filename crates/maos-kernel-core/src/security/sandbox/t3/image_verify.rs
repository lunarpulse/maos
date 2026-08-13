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
    let hex_str = std::env::var("MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX").map_err(|_| {
        T3Error::TrustAnchorMissing("MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX env-var not set".into())
    })?;
    let mut key = [0u8; 32];
    hex::decode_to_slice(hex_str.trim(), &mut key).map_err(|e| {
        T3Error::TrustAnchorMissing(format!(
            "MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX decode failed: {e}"
        ))
    })?;
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

    let entries_bytes =
        serde_json::to_vec(&attestation.entries).map_err(|e| T3Error::Io(e.to_string()))?;
    crypto
        .verify_signature(
            &attestation.signer_pub_key,
            &entries_bytes,
            &attestation.signature,
        )
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

    let entries_bytes =
        serde_json::to_vec(&image.entries).map_err(|e| T3Error::Io(e.to_string()))?;
    crypto
        .verify_signature(&image.signer_pub_key, &entries_bytes, &image.signature)
        .map_err(|_| T3Error::SignatureInvalid)?;

    Ok(())
}

/// Inspect the registry manifest digest recorded by the runtime's
/// `RepoDigests` metadata.  Local `.Id` is a config/image identifier and must
/// never be compared with an attested registry manifest digest.
pub fn inspect_image_sha(runtime: &ContainerRuntime, image_uri: &str) -> Result<[u8; 32], T3Error> {
    let output = std::process::Command::new(&runtime.path)
        .args([
            "image",
            "inspect",
            "--format",
            "{{json .RepoDigests}}",
            image_uri,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|error| T3Error::Inspect(format!("image inspect for {image_uri}: {error}")))?;

    if !output.status.success() {
        return Err(T3Error::Inspect(format!(
            "image inspect failed for {image_uri}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    parse_repo_digest(&output.stdout, image_uri)
}

fn parse_repo_digest(stdout: &[u8], image_uri: &str) -> Result<[u8; 32], T3Error> {
    let digests: Vec<String> = serde_json::from_slice(stdout).map_err(|error| {
        T3Error::Inspect(format!(
            "malformed RepoDigests output for {image_uri}: {error}"
        ))
    })?;
    let expected_repo = normalize_repository(image_uri)?;
    let digest = digests
        .iter()
        .find(|digest| {
            digest.split_once("@sha256:").is_some_and(|(repo, _)| {
                normalize_repository(repo).ok().as_deref() == Some(expected_repo.as_str())
            })
        })
        .ok_or_else(|| {
            T3Error::Inspect(format!(
                "RepoDigests contains no manifest digest for repository {expected_repo}"
            ))
        })?;
    let (_, hex_digest) = digest
        .split_once("@sha256:")
        .ok_or_else(|| T3Error::Inspect(format!("malformed repository digest '{digest}'")))?;
    let mut sha = [0u8; 32];
    hex::decode_to_slice(hex_digest, &mut sha).map_err(|error| {
        T3Error::Inspect(format!("malformed manifest digest '{digest}': {error}"))
    })?;
    Ok(sha)
}

/// Remove a tag or digest suffix while preserving registry ports in the
/// repository name.  This gives argv pinning and runtime inspection the same
/// repository identity without constructing `@sha256:@sha256:` URIs.
pub fn normalize_repository(image_uri: &str) -> Result<String, T3Error> {
    let without_digest = image_uri
        .split_once('@')
        .map_or(image_uri, |(repository, _)| repository);
    let last_slash = without_digest.rfind('/');
    let tag_separator = without_digest.rfind(':');
    let repository = match (last_slash, tag_separator) {
        (Some(slash), Some(colon)) if colon > slash => &without_digest[..colon],
        (None, Some(colon)) => &without_digest[..colon],
        _ => without_digest,
    };
    if repository.is_empty() {
        return Err(T3Error::Inspect("empty image repository".into()));
    }
    Ok(repository.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{normalize_repository, parse_repo_digest, T3Error};

    #[test]
    fn normalizer_removes_one_tag_or_digest_without_losing_registry_port() {
        assert_eq!(
            normalize_repository("registry.example:5000/team/spirit:stable").unwrap(),
            "registry.example:5000/team/spirit"
        );
        assert_eq!(
            normalize_repository(
                "registry.example:5000/team/spirit@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            )
            .unwrap(),
            "registry.example:5000/team/spirit"
        );
    }

    #[test]
    fn repo_digest_parser_selects_the_matching_repository() {
        let expected = [0xAB; 32];
        let stdout = serde_json::to_vec(&vec![
            format!("registry.example/other@sha256:{}", "cd".repeat(32)),
            format!(
                "registry.example:5000/team/spirit@sha256:{}",
                hex::encode(expected)
            ),
        ])
        .unwrap();
        assert_eq!(
            parse_repo_digest(&stdout, "registry.example:5000/team/spirit:stable").unwrap(),
            expected
        );
    }

    #[test]
    fn repo_digest_parser_preserves_malformed_and_missing_diagnostics() {
        assert!(matches!(
            parse_repo_digest(b"not-json", "registry.example/team/spirit"),
            Err(T3Error::Inspect(reason)) if reason.contains("malformed RepoDigests")
        ));
        let unrelated = serde_json::to_vec(&vec![format!(
            "registry.example/other@sha256:{}",
            "ab".repeat(32)
        )])
        .unwrap();
        assert!(matches!(
            parse_repo_digest(&unrelated, "registry.example/team/spirit"),
            Err(T3Error::Inspect(reason)) if reason.contains("contains no manifest digest")
        ));
    }

    struct RejectCrypto;

    impl maos_domain::ports::crypto::CryptoProvider for RejectCrypto {
        fn verify_signature(
            &self,
            _: &[u8],
            _: &[u8],
            _: &[u8],
        ) -> Result<(), maos_domain::ports::crypto::CryptoError> {
            Err(maos_domain::ports::crypto::CryptoError::SignatureInvalid)
        }

        fn seal_for_export(
            &self,
            _: &[u8],
            _: &[u8],
            _: &[u8],
            _: &[u8],
        ) -> Result<Vec<u8>, maos_domain::ports::crypto::CryptoError> {
            Err(maos_domain::ports::crypto::CryptoError::OperationFailed(
                "unused",
            ))
        }

        fn sign_capability_token(
            &self,
            _: &[u8],
            _: &[u8],
        ) -> Result<Vec<u8>, maos_domain::ports::crypto::CryptoError> {
            Err(maos_domain::ports::crypto::CryptoError::OperationFailed(
                "unused",
            ))
        }
    }

    #[test]
    fn signed_lock_parser_rejects_an_invalid_signature_after_anchor_match() {
        let json = serde_json::json!({
            "id": vec![9; 32],
            "schema_version": 1,
            "signed_at_ns": 1,
            "entries": [{
                "image_uri": "registry.example/spirit",
                "image_sha256": vec![8; 32],
                "description": "test",
                "default_for_v05": true
            }],
            "signature": vec![7; 64],
            "signer_pub_key": vec![6; 32]
        });
        assert!(matches!(
            super::parse_signed_image_attestation(
                serde_json::to_string(&json).unwrap().as_bytes(),
                &[6; 32],
                &RejectCrypto,
            ),
            Err(maos_domain::sandbox::T3Error::SignatureInvalid)
        ));
    }
}
