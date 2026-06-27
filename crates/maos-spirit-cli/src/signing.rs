//! Ed25519 signing-key loading + keypair derivation for `maos-spirit publish`.
//!
//! Two on-disk formats supported per Story 7.2 AC2 §3:
//!   * PEM-encoded `BEGIN ED25519 PRIVATE KEY` block (extracts 32-byte seed
//!     from PKCS#8 v1 or v2 envelopes via the trailing 32 bytes of the inner
//!     OCTET STRING — sufficient for the openssl-genpkey path called out in
//!     the README).
//!   * Raw 32-byte hex (64 hex chars optionally with a `0x` prefix or trailing
//!     newline).
//!
//! The CLI accepts:
//!   1. `--signing-key <path>` (explicit; highest precedence)
//!   2. `--signing-key-env <var>` (env var holds the same content as the file)
//!   3. `~/.config/maos/spirit-signing.key` (default fallback)

use std::path::{Path, PathBuf};

use base64::Engine;
use ring::signature::{Ed25519KeyPair, KeyPair};

use crate::errors::CliError;

/// Maximum file size for signing key files (1 MiB).
const MAX_KEY_FILE_SIZE: u64 = 1_048_576;

/// Maximum file size for manifest files (10 MiB).
const MAX_MANIFEST_FILE_SIZE: u64 = 10_485_760;

/// Resolved signing material: 32-byte Ed25519 seed.
pub type Ed25519Seed = [u8; 32];

/// Load the signing seed per the precedence order.
pub fn load_signing_seed(
    explicit_path: &Option<PathBuf>,
    env_var: &Option<String>,
) -> Result<Ed25519Seed, CliError> {
    if let Some(path) = explicit_path {
        let bytes = read_key_file(path)?;
        return parse_seed_bytes(&bytes);
    }
    if let Some(var_name) = env_var {
        let value = std::env::var(var_name)
            .map_err(|e| CliError::SigningKeyLoad(format!("env var '{var_name}' not set: {e}")))?;
        return parse_seed_bytes(value.as_bytes());
    }
    // Default fallback
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| {
            CliError::SigningKeyLoad(
                "no --signing-key, no --signing-key-env, and $HOME/$USERPROFILE unset".into(),
            )
        })?;
    let default = Path::new(&home).join(".config/maos/spirit-signing.key");
    let bytes = read_key_file(&default)?;
    parse_seed_bytes(&bytes)
}

/// Read a key file with size limits and permission checks.
fn read_key_file(path: &Path) -> Result<Vec<u8>, CliError> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| CliError::SigningKeyLoad(format!("read {:?}: {e}", path)))?;

    let size = metadata.len();
    if size > MAX_KEY_FILE_SIZE {
        return Err(CliError::SigningKeyLoad(format!(
            "key file too large: {} bytes (max {})",
            size, MAX_KEY_FILE_SIZE
        )));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        if mode & 0o077 != 0 {
            return Err(CliError::SigningKeyLoad(format!(
                "key file {:?} has overly permissive permissions ({:04o}); \
                 expected owner-only (0o600 or tighter)",
                path,
                mode & 0o777
            )));
        }
    }

    std::fs::read(path).map_err(|e| CliError::SigningKeyLoad(format!("read {:?}: {e}", path)))
}

/// Parse the seed from either raw 32-byte hex or PEM PKCS#8 envelope.
pub fn parse_seed_bytes(raw: &[u8]) -> Result<Ed25519Seed, CliError> {
    let s = std::str::from_utf8(raw).unwrap_or("").trim();
    // PEM path
    if s.starts_with("-----BEGIN") {
        return parse_pem_seed(s);
    }
    // Raw 32-byte hex path
    let hex_input = s.strip_prefix("0x").unwrap_or(s);
    let hex_input: String = hex_input.chars().filter(|c| !c.is_whitespace()).collect();
    if hex_input.len() == 64 {
        let bytes = hex::decode(&hex_input)
            .map_err(|e| CliError::SigningKeyLoad(format!("hex decode failure: {e}")))?;
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&bytes);
        return Ok(seed);
    }
    // Try interpreting the raw bytes as 32 raw bytes (binary key on disk).
    if raw.len() == 32 {
        let mut seed = [0u8; 32];
        seed.copy_from_slice(raw);
        return Ok(seed);
    }
    Err(CliError::SigningKeyLoad(format!(
        "signing key is not 64-char hex, not PEM, and not 32 raw bytes (got {} bytes)",
        raw.len()
    )))
}

/// Minimal PEM parser — extracts the trailing 32 bytes of the inner OCTET STRING
/// from a PKCS#8 v1 or v2 Ed25519 private-key envelope. Matches the output
/// shape of `openssl genpkey -algorithm Ed25519`.
fn parse_pem_seed(pem: &str) -> Result<Ed25519Seed, CliError> {
    let mut b64_lines = Vec::new();
    let mut in_body = false;
    let mut block_count = 0u32;
    for line in pem.lines() {
        let line = line.trim();
        if line.starts_with("-----BEGIN") {
            if block_count > 0 {
                return Err(CliError::SigningKeyLoad(
                    "PEM file contains multiple blocks; expected a single Ed25519 private key"
                        .into(),
                ));
            }
            let label = line
                .trim_start_matches("-----BEGIN ")
                .trim_end_matches("-----");
            if !label.contains("PRIVATE KEY") {
                return Err(CliError::SigningKeyLoad(format!(
                    "expected a PRIVATE KEY PEM block, got: {label}"
                )));
            }
            in_body = true;
            continue;
        }
        if line.starts_with("-----END") {
            in_body = false;
            block_count += 1;
            continue;
        }
        if in_body && !line.is_empty() {
            b64_lines.push(line);
        }
    }
    if b64_lines.is_empty() {
        return Err(CliError::SigningKeyLoad("empty PEM body".into()));
    }
    let b64 = b64_lines.concat();
    let der = base64::engine::general_purpose::STANDARD
        .decode(&b64)
        .map_err(|e| CliError::SigningKeyLoad(format!("base64 decode: {e}")))?;
    if der.len() < 32 {
        return Err(CliError::SigningKeyLoad(format!(
            "PKCS#8 envelope too short: {} bytes",
            der.len()
        )));
    }
    if der.len() < 48 {
        return Err(CliError::SigningKeyLoad(
            "DER envelope too short to contain an Ed25519 key — expected PKCS#8 v1 (48+ bytes) or v2 (49+ bytes)".into(),
        ));
    }
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&der[der.len() - 32..]);
    Ok(seed)
}

/// Derive an Ed25519 keypair from the seed.
///
/// Returns (`publisher_pubkey`, the keypair).
pub fn derive_keypair(seed: &Ed25519Seed) -> Result<([u8; 32], Ed25519KeyPair), CliError> {
    let pair = Ed25519KeyPair::from_seed_unchecked(seed)
        .map_err(|e| CliError::SigningKeyDerive(format!("ring from_seed: {e}")))?;
    let pub_bytes = pair.public_key().as_ref();
    if pub_bytes.len() != 32 {
        return Err(CliError::SigningKeyDerive(format!(
            "expected 32-byte pubkey, got {}",
            pub_bytes.len()
        )));
    }
    let mut pubkey = [0u8; 32];
    pubkey.copy_from_slice(pub_bytes);
    Ok((pubkey, pair))
}

/// Extract `(spirit_id, version)` from manifest TOML using proper parsing.
pub fn extract_spirit_id_and_version(manifest_toml: &[u8]) -> Result<(String, String), CliError> {
    let text = std::str::from_utf8(manifest_toml)
        .map_err(|e| CliError::ManifestParse(format!("manifest not UTF-8: {e}")))?;

    let manifest: toml::Value = text
        .parse()
        .map_err(|e| CliError::ManifestParse(format!("TOML parse error: {e}")))?;

    let spirit_id = manifest
        .get("spirit_id")
        .or_else(|| manifest.get("name"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::ManifestParse("manifest missing `spirit_id` or `name`".into()))?;

    let version = manifest
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::ManifestParse("manifest missing `version`".into()))?;

    Ok((spirit_id.to_string(), version.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_seed_loads() {
        let hex = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let seed = parse_seed_bytes(hex.as_bytes()).unwrap();
        assert_eq!(hex::encode(seed), hex);
    }

    #[test]
    fn hex_seed_with_prefix_and_whitespace_loads() {
        let hex = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef\n";
        let seed = parse_seed_bytes(hex.as_bytes()).unwrap();
        assert_eq!(hex::encode(seed)[..8], *"12345678");
    }

    #[test]
    fn derive_keypair_round_trips_signature() {
        let seed = [42u8; 32];
        let (pubkey, pair) = derive_keypair(&seed).unwrap();
        let msg = b"hello, story 7.2";
        let sig = pair.sign(msg);
        use ring::signature::{UnparsedPublicKey, ED25519};
        let pk = UnparsedPublicKey::new(&ED25519, &pubkey);
        assert!(pk.verify(msg, sig.as_ref()).is_ok());
    }

    #[test]
    fn extract_spirit_id_and_version_parses_minimal_manifest() {
        let manifest = b"name = \"hello-spirit\"\nversion = \"0.1.0\"\n";
        let (id, ver) = extract_spirit_id_and_version(manifest).unwrap();
        assert_eq!(id, "hello-spirit");
        assert_eq!(ver, "0.1.0");
    }

    #[test]
    fn extract_spirit_id_prefers_spirit_id_over_name() {
        let manifest = b"spirit_id = \"real-id\"\nname = \"display-name\"\nversion = \"1.0.0\"\n";
        let (id, ver) = extract_spirit_id_and_version(manifest).unwrap();
        assert_eq!(id, "real-id");
        assert_eq!(ver, "1.0.0");
    }

    #[test]
    fn base64_with_padding_decodes_correctly() {
        // Standard base64 with padding
        let b64 = "SGVsbG8gV29ybGQh";
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), "Hello World!");
    }

    #[test]
    fn base64_without_padding_fails() {
        // Standard base64 decoder requires padding
        let b64 = "SGVsbG8gV29ybGQ";
        let result = base64::engine::general_purpose::STANDARD.decode(b64);
        assert!(result.is_err());
    }
}
