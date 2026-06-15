//! Story 9.4 AC-1 — release artifact verification.
//!
//! Shared by `maosctl install` (CLI consumer) and `xtask release-verify`
//! (CI gate). Verifies release artifacts via SHA256 integrity + Ed25519
//! signature over the `SHA256SUMS` file. The signing pattern mirrors
//! `sealed_export::sign_bundle` / `verify_bundle` (same `ed25519-dalek`
//! + `sha2` deps, same `sha256(content)` → Ed25519 sign idiom).
//!
//! The release-signing key is **distinct** from the audit-signing key
//! (`crates/maos-domain/src/audit_key.rs`) and the capability-token
//! signing key. Provenance: CI secret `RELEASE_SIGNING_KEY`; rotation
//! documented in `docs/runbooks/release-signing.md`.

#![forbid(unsafe_code)]

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

/// Ed25519 release-signing **public** key, bundled into every binary.
///
/// Override at build time via the `MAOS_RELEASE_PUBKEY` environment variable
/// (64 lowercase hex chars). If unset, a documented development key is used so
/// offline verification works in local/test builds. CI must set the production
/// key and assert it differs from `dev_seed()`.
pub const RELEASE_PUBKEY: [u8; 32] = {
    const DEFAULT: [u8; 32] = [
        0xbe, 0xdd, 0x2b, 0xa6, 0x34, 0xda, 0x72, 0x40, 0x27, 0x98, 0x3f, 0x36, 0x91, 0x49, 0xf1,
        0x08, 0x54, 0x1f, 0x43, 0xe6, 0x24, 0xa8, 0x46, 0x43, 0x8c, 0x01, 0x45, 0x2c, 0xa7, 0xf4,
        0x69, 0xe7,
    ];

    const fn parse_hex_32(s: &str) -> [u8; 32] {
        let bytes = s.as_bytes();
        let mut out = [0u8; 32];
        let mut i = 0;
        while i < 32 {
            let hi = match bytes[i * 2] {
                b'0'..=b'9' => bytes[i * 2] - b'0',
                b'a'..=b'f' => bytes[i * 2] - b'a' + 10,
                b'A'..=b'F' => bytes[i * 2] - b'A' + 10,
                _ => panic!("MAOS_RELEASE_PUBKEY must be 64 lowercase hex chars"),
            };
            let lo = match bytes[i * 2 + 1] {
                b'0'..=b'9' => bytes[i * 2 + 1] - b'0',
                b'a'..=b'f' => bytes[i * 2 + 1] - b'a' + 10,
                b'A'..=b'F' => bytes[i * 2 + 1] - b'A' + 10,
                _ => panic!("MAOS_RELEASE_PUBKEY must be 64 lowercase hex chars"),
            };
            out[i] = (hi << 4) | lo;
            i += 1;
        }
        out
    }

    match option_env!("MAOS_RELEASE_PUBKEY") {
        Some(hex) => parse_hex_32(hex),
        None => DEFAULT,
    }
};

#[derive(Debug, thiserror::Error)]
pub enum ReleaseVerifyError {
    #[error("SHA256 mismatch for {file}: expected {expected}, got {actual}")]
    Sha256Mismatch {
        file: String,
        expected: String,
        actual: String,
    },
    #[error("Ed25519 signature verification failed")]
    SignatureVerificationFailed,
    #[error("invalid public key: {0}")]
    InvalidPubkey(String),
    #[error("invalid signature format: {0}")]
    InvalidSignature(String),
    #[error("malformed SHA256SUMS line {line_num}: {line}")]
    MalformedSha256sumsLine { line_num: usize, line: String },
    #[error("file not found in SHA256SUMS: {0}")]
    FileNotInSha256sums(String),
    #[error("manifest entry not provided for verification: {0}")]
    MissingArtifact(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// A parsed `(sha256_hex, filename)` entry from a `SHA256SUMS` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sha256Entry {
    pub hash: String,
    pub filename: String,
}

/// Parse a `SHA256SUMS` file into entries.
///
/// Format: `<64-hex-chars>  <filename>\n` (two spaces between hash and name,
/// matching the GNU coreutils `sha256sum -b` output format). Filenames may
/// contain single spaces because the separator is exactly two spaces.
pub fn parse_sha256sums(content: &str) -> Result<Vec<Sha256Entry>, ReleaseVerifyError> {
    let mut entries = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // GNU coreutils format: hash + two spaces + filename.
        let (hash, filename) =
            line.split_once("  ")
                .ok_or_else(|| ReleaseVerifyError::MalformedSha256sumsLine {
                    line_num: i + 1,
                    line: line.to_string(),
                })?;
        let hash = hash.trim();
        let filename = filename.trim();
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ReleaseVerifyError::MalformedSha256sumsLine {
                line_num: i + 1,
                line: line.to_string(),
            });
        }
        entries.push(Sha256Entry {
            hash: hash.to_lowercase(),
            filename: filename.to_string(),
        });
    }
    Ok(entries)
}

/// Compute the SHA256 hash of `data` and return the lowercase hex string.
pub fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    hex::encode(digest)
}

/// Verify a file's contents against an expected SHA256 hex string.
pub fn verify_sha256(
    file_bytes: &[u8],
    expected_hex: &str,
    filename: &str,
) -> Result<(), ReleaseVerifyError> {
    let actual = sha256_hex(file_bytes);
    if actual != expected_hex.to_lowercase() {
        return Err(ReleaseVerifyError::Sha256Mismatch {
            file: filename.to_string(),
            expected: expected_hex.to_string(),
            actual,
        });
    }
    Ok(())
}

/// Verify the Ed25519 signature over `SHA256SUMS` content.
///
/// The signing convention mirrors `sealed_export::sign_bundle`:
/// `Ed25519(sha256(sha256sums_bytes))`.
pub fn verify_release_signature(
    sha256sums_bytes: &[u8],
    sig_bytes: &[u8; 64],
    pubkey: &[u8; 32],
) -> Result<(), ReleaseVerifyError> {
    let verifying_key = VerifyingKey::from_bytes(pubkey)
        .map_err(|e| ReleaseVerifyError::InvalidPubkey(format!("{e}")))?;

    let digest = Sha256::digest(sha256sums_bytes);
    let signature = ed25519_dalek::Signature::from_bytes(sig_bytes);

    verifying_key
        .verify(&digest, &signature)
        .map_err(|_| ReleaseVerifyError::SignatureVerificationFailed)
}

/// Sign `SHA256SUMS` content with an Ed25519 private key seed.
///
/// Returns the 64-byte raw signature. Used by the release pipeline
/// and `xtask release-verify --sign`.
pub fn sign_sha256sums(sha256sums_bytes: &[u8], seed: &[u8; 32]) -> [u8; 64] {
    let signing_key = SigningKey::from_bytes(seed);
    let digest = Sha256::digest(sha256sums_bytes);
    let signature = signing_key.sign(&digest);
    signature.to_bytes()
}

/// Generate a `SHA256SUMS` file from a list of `(filename, sha256_hex)` pairs.
///
/// Output matches the GNU coreutils `sha256sum` format: `<hash>  <filename>\n`.
pub fn generate_sha256sums(entries: &[(String, String)]) -> String {
    let mut out = String::new();
    for (filename, hash) in entries {
        out.push_str(hash);
        out.push_str("  ");
        out.push_str(filename);
        out.push('\n');
    }
    out
}

/// Full release-artifact verification pipeline.
/// 1. Verify the Ed25519 signature over `sha256sums_content` (fail-closed).
/// 2. Parse `SHA256SUMS` entries.
/// 3. Verify each file's SHA256 against the signed manifest.
///
/// `files` is an iterator of `(filename, file_bytes)`. When `allow_subset` is
/// `false` (the default for CI gates), every manifest entry must be present in
/// `files`; missing entries fail with `MissingArtifact`. When `allow_subset` is
/// `true`, only the provided files are verified (used by `maosctl install` for a
/// single-platform artifact).
pub fn verify_release(
    sha256sums_content: &[u8],
    sig_bytes: &[u8; 64],
    pubkey: &[u8; 32],
    files: &[(&str, &[u8])],
    allow_subset: bool,
) -> Result<Vec<Sha256Entry>, ReleaseVerifyError> {
    // Step 1: signature verification (fail-closed — always first)
    verify_release_signature(sha256sums_content, sig_bytes, pubkey)?;

    // Step 2: parse the trusted SHA256SUMS
    let content = std::str::from_utf8(sha256sums_content).map_err(|e| {
        ReleaseVerifyError::InvalidSignature(format!("SHA256SUMS is not valid UTF-8: {e}"))
    })?;
    let entries = parse_sha256sums(content)?;

    // Step 3: validate that the caller supplied all manifest entries unless
    // subset verification was explicitly requested.
    if !allow_subset {
        if files.is_empty() {
            return Err(ReleaseVerifyError::MissingArtifact(
                "no artifacts provided for verification".to_string(),
            ));
        }
        let provided: std::collections::HashSet<&str> = files.iter().map(|(n, _)| *n).collect();
        for entry in &entries {
            if !provided.contains(entry.filename.as_str()) {
                return Err(ReleaseVerifyError::MissingArtifact(entry.filename.clone()));
            }
        }
    }

    // Step 4: verify each provided file against the manifest
    for (filename, file_bytes) in files {
        let entry = entries
            .iter()
            .find(|e| e.filename == *filename)
            .ok_or_else(|| ReleaseVerifyError::FileNotInSha256sums(filename.to_string()))?;
        verify_sha256(file_bytes, &entry.hash, filename)?;
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev_seed() -> [u8; 32] {
        let hex_str = "794959d4c4dc813f968cd95eb4a45c4a02583a7c5211126e7b4583e4776d1c8d";
        let bytes: Vec<u8> = (0..hex_str.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16).unwrap())
            .collect();
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&bytes);
        seed
    }

    #[test]
    fn bundled_pubkey_matches_dev_seed_when_not_overridden() {
        // If MAOS_RELEASE_PUBKEY is set, the bundled key is intentionally
        // different from the dev seed. In local/test builds (no env override),
        // the bundled key must equal the dev seed so tests can sign/verify.
        if option_env!("MAOS_RELEASE_PUBKEY").is_some() {
            return;
        }
        let pubkey = crate::sealed_export::derive_pubkey(&dev_seed());
        assert_eq!(
            pubkey, RELEASE_PUBKEY,
            "bundled RELEASE_PUBKEY must match dev seed"
        );
    }

    #[test]
    fn production_pubkey_must_differ_from_dev_seed() {
        // This test is a compile-time guard: if MAOS_RELEASE_PUBKEY is set,
        // the resulting bundled key MUST NOT be the dev seed. When the env var
        // is unset, this assertion is trivially true because RELEASE_PUBKEY
        // still equals the dev seed; CI overrides the env var to enforce the
        // real production-key invariant.
        let dev_pubkey = crate::sealed_export::derive_pubkey(&dev_seed());
        if option_env!("MAOS_RELEASE_PUBKEY").is_some() {
            assert_ne!(
                dev_pubkey, RELEASE_PUBKEY,
                "production RELEASE_PUBKEY must differ from dev_seed-derived key"
            );
        }
    }

    #[test]
    fn parse_sha256sums_valid() {
        let content = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789  maos-linux-amd64.tar.gz\n\
                        fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210  maos-linux-arm64.tar.gz\n";
        let entries = parse_sha256sums(content).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].filename, "maos-linux-amd64.tar.gz");
        assert_eq!(entries[1].filename, "maos-linux-arm64.tar.gz");
    }

    #[test]
    fn parse_sha256sums_rejects_bad_hash() {
        let content = "tooshort  file.tar.gz\n";
        assert!(parse_sha256sums(content).is_err());
    }

    #[test]
    fn parse_sha256sums_skips_blank_lines() {
        let content =
            "\n\nabcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789  file.tar.gz\n\n";
        let entries = parse_sha256sums(content).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn sha256_hex_deterministic() {
        let data = b"hello world";
        let h1 = sha256_hex(data);
        let h2 = sha256_hex(data);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn verify_sha256_pass() {
        let data = b"test data";
        let hash = sha256_hex(data);
        verify_sha256(data, &hash, "test.bin").unwrap();
    }

    #[test]
    fn verify_sha256_fail() {
        let data = b"test data";
        let err = verify_sha256(
            data,
            "0000000000000000000000000000000000000000000000000000000000000000",
            "test.bin",
        );
        assert!(err.is_err());
    }

    #[test]
    fn sign_and_verify_round_trip() {
        let seed = dev_seed();
        let content = b"abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789  maos-linux-amd64.tar.gz\n";
        let sig = sign_sha256sums(content, &seed);
        verify_release_signature(content, &sig, &RELEASE_PUBKEY).unwrap();
    }

    #[test]
    fn verify_rejects_tampered_content() {
        let seed = dev_seed();
        let content = b"abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789  maos-linux-amd64.tar.gz\n";
        let sig = sign_sha256sums(content, &seed);
        let tampered = b"abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456780  maos-linux-amd64.tar.gz\n";
        let err = verify_release_signature(tampered, &sig, &RELEASE_PUBKEY);
        assert!(matches!(
            err,
            Err(ReleaseVerifyError::SignatureVerificationFailed)
        ));
    }

    #[test]
    fn verify_rejects_wrong_key() {
        let seed = dev_seed();
        let content = b"abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789  maos-linux-amd64.tar.gz\n";
        let sig = sign_sha256sums(content, &seed);
        let wrong_key = [0u8; 32]; // zeroed key
        let err = verify_release_signature(content, &sig, &wrong_key);
        assert!(err.is_err());
    }

    #[test]
    fn full_verify_release_pipeline() {
        let seed = dev_seed();
        let file_a = b"binary content A";
        let file_b = b"binary content B";
        let hash_a = sha256_hex(file_a);
        let hash_b = sha256_hex(file_b);

        let sums_content = generate_sha256sums(&[
            ("maos-linux-amd64".to_string(), hash_a),
            ("maos-linux-arm64".to_string(), hash_b),
        ]);
        let sig = sign_sha256sums(sums_content.as_bytes(), &seed);

        let files: Vec<(&str, &[u8])> =
            vec![("maos-linux-amd64", file_a), ("maos-linux-arm64", file_b)];

        let entries = verify_release(
            sums_content.as_bytes(),
            &sig,
            &RELEASE_PUBKEY,
            &files,
            false,
        )
        .unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn full_verify_release_tampered_binary_rejected() {
        let seed = dev_seed();
        let file_a = b"binary content A";
        let hash_a = sha256_hex(file_a);

        let sums_content = generate_sha256sums(&[("maos-linux-amd64".to_string(), hash_a)]);
        let sig = sign_sha256sums(sums_content.as_bytes(), &seed);

        let tampered_file = b"tampered binary content";
        let files: Vec<(&str, &[u8])> = vec![("maos-linux-amd64", tampered_file)];

        let err = verify_release(
            sums_content.as_bytes(),
            &sig,
            &RELEASE_PUBKEY,
            &files,
            false,
        );
        assert!(matches!(
            err,
            Err(ReleaseVerifyError::Sha256Mismatch { .. })
        ));
    }

    #[test]
    fn full_verify_release_rejects_missing_artifacts_in_strict_mode() {
        let seed = dev_seed();
        let file_a = b"binary content A";
        let hash_a = sha256_hex(file_a);

        let sums_content = generate_sha256sums(&[
            ("maos-linux-amd64".to_string(), hash_a.clone()),
            ("maos-linux-arm64".to_string(), hash_a),
        ]);
        let sig = sign_sha256sums(sums_content.as_bytes(), &seed);

        // Only one artifact provided, but manifest lists two → strict mode fails.
        let files: Vec<(&str, &[u8])> = vec![("maos-linux-amd64", file_a)];
        let err = verify_release(
            sums_content.as_bytes(),
            &sig,
            &RELEASE_PUBKEY,
            &files,
            false,
        );
        assert!(matches!(err, Err(ReleaseVerifyError::MissingArtifact(_))));
    }

    #[test]
    fn full_verify_release_allows_subset_when_requested() {
        let seed = dev_seed();
        let file_a = b"binary content A";
        let hash_a = sha256_hex(file_a);

        let sums_content = generate_sha256sums(&[
            ("maos-linux-amd64".to_string(), hash_a.clone()),
            ("maos-linux-arm64".to_string(), hash_a),
        ]);
        let sig = sign_sha256sums(sums_content.as_bytes(), &seed);

        let files: Vec<(&str, &[u8])> = vec![("maos-linux-amd64", file_a)];
        let entries =
            verify_release(sums_content.as_bytes(), &sig, &RELEASE_PUBKEY, &files, true).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn full_verify_release_rejects_empty_files_in_strict_mode() {
        let seed = dev_seed();
        let content =
            b"abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789  maos-linux-amd64\n";
        let sig = sign_sha256sums(content, &seed);

        let err = verify_release(content, &sig, &RELEASE_PUBKEY, &[], false);
        assert!(matches!(err, Err(ReleaseVerifyError::MissingArtifact(_))));
    }

    #[test]
    fn signature_must_be_verified_before_sha256() {
        // Even if the SHA256 entries parse fine, a missing/invalid signature
        // must be rejected FIRST (fail-closed).
        let content =
            b"abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789  maos-linux-amd64\n";
        let bad_sig = [0u8; 64];
        let err = verify_release(content, &bad_sig, &RELEASE_PUBKEY, &[], false);
        assert!(matches!(
            err,
            Err(ReleaseVerifyError::SignatureVerificationFailed)
        ));
    }

    #[test]
    fn parse_sha256sums_rejects_single_space_separator() {
        // Single space is ambiguous for filenames containing spaces.
        let content = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789 file with spaces.tar.gz\n";
        assert!(parse_sha256sums(content).is_err());
    }

    #[test]
    fn parse_sha256sums_accepts_filename_with_single_space() {
        let content = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789  file with spaces.tar.gz\n";
        let entries = parse_sha256sums(content).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].filename, "file with spaces.tar.gz");
    }
}
