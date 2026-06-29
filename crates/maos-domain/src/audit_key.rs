//! Audit signing key loader — path→env→default precedence.
//!
//! Shared by `maos-cli` (sealed-export) and future `maos-spirit-cli` audit paths.
//! Decision B: DISTINCT key from publishing/capability keys.

use std::path::{Path, PathBuf};

/// Maximum file size for audit key files (1 MiB).
const MAX_KEY_FILE_SIZE: u64 = 1_048_576;

#[derive(Debug, thiserror::Error)]
pub enum AuditKeyError {
    #[error("audit signing key not found at {path}. Generate one with `maosctl audit keygen`.")]
    NotFound { path: String },
    #[error("audit signing key file has insecure permissions (group/other must be zero): {path}")]
    InsecurePermissions { path: String },
    #[error("failed to read audit signing key: {0}")]
    IoError(#[from] std::io::Error),
    #[error("invalid key format: {0}")]
    InvalidFormat(String),
}

pub type Ed25519Seed = [u8; 32];

/// Load audit signing seed per precedence:
/// 1. `explicit_path` (--audit-key flag)
/// 2. `MAOS_AUDIT_KEY` env var
/// 3. `~/.config/maos/audit-signing.key` default
///
/// Fails loudly if no key is found — no silent keygen.
pub fn load_audit_key_seed(explicit_path: &Option<PathBuf>) -> Result<Ed25519Seed, AuditKeyError> {
    let path = resolve_key_path(explicit_path)?;

    if !path.exists() {
        return Err(AuditKeyError::NotFound {
            path: path.display().to_string(),
        });
    }

    check_permissions(&path)?;
    let raw = read_key_file(&path)?;
    parse_seed_bytes(&raw)
}

/// Default audit signing key path: `~/.config/maos/audit-signing.key`
pub fn default_audit_key_path() -> PathBuf {
    dirs_config_home().join("audit-signing.key")
}

/// Generate a new Ed25519 audit signing key, write to `output_path` with
/// 0600 permissions. Returns the hex-encoded public key fingerprint.
pub fn generate_audit_key(output_path: &Option<PathBuf>) -> Result<String, AuditKeyError> {
    let path = resolve_key_path_or(output_path)?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let seed: Ed25519Seed = {
        let mut buf = [0u8; 32];
        getrandom::fill(&mut buf).map_err(|e| {
            AuditKeyError::InvalidFormat(format!("failed to generate random seed: {e}"))
        })?;
        buf
    };

    let hex_seed = hex::encode(seed);
    std::fs::write(&path, hex_seed.as_bytes())?;
    set_permissions_0600(&path)?;

    // Derive the Ed25519 public key fingerprint (not the seed hex).
    let fingerprint = derive_pubkey_fingerprint(&seed);
    Ok(fingerprint)
}

// ─── Internal helpers ──────────────────────────────────────────────────────

/// Resolve the key path from precedence order.
fn resolve_key_path(explicit: &Option<PathBuf>) -> Result<PathBuf, AuditKeyError> {
    if let Some(p) = explicit {
        return Ok(p.clone());
    }
    if let Ok(env_val) = std::env::var("MAOS_AUDIT_KEY") {
        if !env_val.is_empty() {
            return Ok(PathBuf::from(env_val));
        }
    }
    Ok(default_audit_key_path())
}

/// Resolve key path for generation (only explicit or default).
fn resolve_key_path_or(explicit: &Option<PathBuf>) -> Result<PathBuf, AuditKeyError> {
    if let Some(p) = explicit {
        return Ok(p.clone());
    }
    Ok(default_audit_key_path())
}

fn dirs_config_home() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("maos");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config").join("maos");
    }
    PathBuf::from("/etc/maos")
}

#[cfg(unix)]
fn check_permissions(path: &Path) -> Result<(), AuditKeyError> {
    use std::os::unix::fs::PermissionsExt;
    let meta = std::fs::metadata(path)?;
    let mode = meta.permissions().mode() & 0o777;
    // Accept any mode where group and other permission bits are zero
    // (e.g. 0400, 0600). Only owner access is required.
    if mode & 0o077 != 0 {
        return Err(AuditKeyError::InsecurePermissions {
            path: path.display().to_string(),
        });
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_permissions(path: &Path) -> Result<(), AuditKeyError> {
    // POSIX permission bits do not exist on non-Unix targets (Windows uses
    // ACLs, not a 9-bit mode). Confirm the file is present/readable; ACL
    // hardening is the operator's responsibility there. Mirrors the
    // `#[cfg(unix)]`-gated permission handling in maos-kernel-core hot_swap.
    let _ = std::fs::metadata(path)?;
    Ok(())
}

#[cfg(unix)]
fn set_permissions_0600(path: &Path) -> Result<(), AuditKeyError> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_permissions_0600(path: &Path) -> Result<(), AuditKeyError> {
    // No POSIX 0600 mode to set on non-Unix targets; the freshly written key
    // file inherits the (user-profile) parent-directory ACL. Best-effort no-op
    // so audit keygen still functions on Windows.
    let _ = path;
    Ok(())
}

fn read_key_file(path: &Path) -> Result<Vec<u8>, AuditKeyError> {
    let meta = std::fs::metadata(path)?;
    if meta.len() > MAX_KEY_FILE_SIZE {
        return Err(AuditKeyError::InvalidFormat(format!(
            "key file too large: {} bytes (max {})",
            meta.len(),
            MAX_KEY_FILE_SIZE
        )));
    }
    Ok(std::fs::read(path)?)
}

/// Parse the seed from either raw 32-byte hex or PEM PKCS#8 envelope.
/// Re-implemented independently from maos-spirit-cli::signing (no import).
///
/// Disambiguation rules:
/// 1. If the data starts with `-----BEGIN`, treat as PEM.
/// 2. If the data is valid UTF-8 and contains 64 hex chars (with optional
///    `0x` prefix and whitespace), treat as hex.
/// 3. If the data is exactly 32 bytes, treat as raw binary seed — even if
///    those 32 bytes happen to also be valid UTF-8 hex. The hex path (2)
///    already handled the "looks like hex" case, so reaching here means
///    it wasn't a valid 64-char hex string.
pub fn parse_seed_bytes(raw: &[u8]) -> Result<Ed25519Seed, AuditKeyError> {
    // PEM path — check raw bytes for the PEM header to avoid UTF-8 ambiguity
    if raw.starts_with(b"-----BEGIN") {
        let s = std::str::from_utf8(raw).map_err(|_| {
            AuditKeyError::InvalidFormat("PEM header found but data is not valid UTF-8".into())
        })?;
        return parse_pem_seed(s.trim());
    }

    // Hex path — only if valid UTF-8 AND looks like hex (64 hex chars)
    if let Ok(s) = std::str::from_utf8(raw) {
        let trimmed = s.trim();
        let hex_input = trimmed.strip_prefix("0x").unwrap_or(trimmed);
        let hex_clean: String = hex_input.chars().filter(|c| !c.is_whitespace()).collect();
        // Validate all chars are hex digits before claiming it's hex
        if hex_clean.len() == 64 && hex_clean.chars().all(|c| c.is_ascii_hexdigit()) {
            let bytes = hex::decode(&hex_clean)
                .map_err(|e| AuditKeyError::InvalidFormat(format!("hex decode failure: {e}")))?;
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&bytes);
            return Ok(seed);
        }
    }

    // Binary path — exactly 32 raw bytes. This branch is only reached if
    // the data didn't match PEM or valid 64-char hex, so a 32-byte binary
    // file that happens to be valid UTF-8 but is NOT 64 hex chars is
    // correctly treated as binary.
    if raw.len() == 32 {
        let mut seed = [0u8; 32];
        seed.copy_from_slice(raw);
        return Ok(seed);
    }

    Err(AuditKeyError::InvalidFormat(format!(
        "audit key is not 64-char hex, not PEM, and not 32 raw bytes (got {} bytes)",
        raw.len()
    )))
}

/// Minimal PEM parser — extracts the 32-byte seed from a PKCS#8 v1 or v2
/// Ed25519 private-key envelope. Validates OID and structure rather than
/// blindly taking the last 32 bytes.
fn parse_pem_seed(pem: &str) -> Result<Ed25519Seed, AuditKeyError> {
    let mut b64_lines = Vec::new();
    let mut in_body = false;
    let mut block_count = 0u32;

    for line in pem.lines() {
        let line = line.trim();
        if line.starts_with("-----BEGIN") {
            if block_count > 0 {
                return Err(AuditKeyError::InvalidFormat(
                    "PEM file contains multiple blocks; expected a single Ed25519 private key"
                        .into(),
                ));
            }
            let label = line
                .trim_start_matches("-----BEGIN ")
                .trim_end_matches("-----");
            if !label.contains("PRIVATE KEY") {
                return Err(AuditKeyError::InvalidFormat(format!(
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
        return Err(AuditKeyError::InvalidFormat("empty PEM body".into()));
    }

    let b64: String = b64_lines.concat();
    let der = decode_base64(&b64)?;

    // Ed25519 OID: 1.3.101.112 — DER encoding: 06 03 2b 65 70
    const ED25519_OID: &[u8] = &[0x06, 0x03, 0x2b, 0x65, 0x70];

    // Try PKCS#8 v2 (RFC 8410) first: the OID appears in the outer
    // AlgorithmIdentifier SEQUENCE, followed by an OCTET STRING wrapper
    // around the 32-byte seed. Total DER length is typically 48 bytes:
    //   SEQUENCE { SEQUENCE { OID, NULL }, OCTET STRING(32 bytes) }
    if let Some(pos) = find_oid(&der, ED25519_OID) {
        // After the OID, expect: 05 00 (NULL) then 04 20 (OCTET STRING, 32 bytes)
        let after_oid = pos + ED25519_OID.len();
        // Skip optional NULL tag (05 00)
        let mut cursor = after_oid;
        if cursor + 2 <= der.len() && der[cursor] == 0x05 && der[cursor + 1] == 0x00 {
            cursor += 2;
        }
        // Expect OCTET STRING tag (04) with length 32 (0x20)
        if cursor + 2 + 32 <= der.len() && der[cursor] == 0x04 && der[cursor + 1] == 0x20 {
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&der[cursor + 2..cursor + 2 + 32]);
            return Ok(seed);
        }
    }

    // Try PKCS#8 v1 (RFC 5208): outer SEQUENCE wraps AlgorithmIdentifier
    // then OCTET STRING containing a DER-encoded EC private key structure.
    // The inner structure ends with the 32-byte seed at the OCTET STRING value.
    // We validate the OID is present somewhere in the DER data.
    if find_oid(&der, ED25519_OID).is_none() {
        return Err(AuditKeyError::InvalidFormat(
            "PEM does not contain an Ed25519 (OID 1.3.101.112) private key".into(),
        ));
    }

    // For v1, locate the inner OCTET STRING containing the 32-byte seed.
    // Scan for tag 04 (OCTET STRING) with length 0x20 (32).
    for i in 0..der.len().saturating_sub(34) {
        if der[i] == 0x04 && der[i + 1] == 0x20 {
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&der[i + 2..i + 2 + 32]);
            return Ok(seed);
        }
    }

    Err(AuditKeyError::InvalidFormat(
        "could not locate 32-byte Ed25519 seed within validated PEM DER structure".into(),
    ))
}

/// Find the Ed25519 OID pattern within a DER blob.
fn find_oid(der: &[u8], oid: &[u8]) -> Option<usize> {
    der.windows(oid.len()).position(|w| w == oid)
}

/// Minimal base64 decoder — avoids adding `base64` crate to maos-domain deps.
fn decode_base64(input: &str) -> Result<Vec<u8>, AuditKeyError> {
    const TABLE: &[Option<u8>; 128] = &{
        let mut table = [None; 128];
        let mut i = 0u8;
        while i < 26 {
            table[(b'A' + i) as usize] = Some(i);
            table[(b'a' + i) as usize] = Some(i + 26);
            i += 1;
        }
        let mut d = 0u8;
        while d < 10 {
            table[(b'0' + d) as usize] = Some(d + 52);
            d += 1;
        }
        table[b'+' as usize] = Some(62);
        table[b'/' as usize] = Some(63);
        table
    };

    let filtered: Vec<u8> = input
        .bytes()
        .filter(|&b| b != b'\n' && b != b'\r' && b != b' ')
        .collect();

    if filtered.len() % 4 != 0 {
        return Err(AuditKeyError::InvalidFormat(
            "base64: invalid length".into(),
        ));
    }

    let mut out = Vec::with_capacity(filtered.len() * 3 / 4);
    let chunks = filtered.chunks_exact(4);
    for chunk in chunks {
        // Decode each 4-char block, rejecting invalid characters.
        let mut vals = [0u8; 4];
        for (i, v) in vals.iter_mut().enumerate() {
            let b = chunk[i];
            // '=' is padding (only valid at positions 2-3), not an error.
            if b == b'=' && i >= 2 {
                *v = 0;
                continue;
            }
            match b
                .try_into()
                .ok()
                .and_then(|c: u8| TABLE.get(c as usize).copied().flatten())
            {
                Some(decoded) => *v = decoded,
                None => {
                    return Err(AuditKeyError::InvalidFormat(format!(
                        "base64: invalid character 0x{b:02x}"
                    )));
                }
            }
        }
        out.push(vals[0] << 2 | vals[1] >> 4);
        if chunk[2] != b'=' {
            out.push((vals[1] & 0x0F) << 4 | vals[2] >> 2);
        }
        if chunk[3] != b'=' {
            out.push((vals[2] & 0x03) << 6 | vals[3]);
        }
    }

    Ok(out)
}

/// Derive the Ed25519 public key fingerprint from the seed.
///
/// Uses ed25519-dalek to compute the actual public key. Returns a compact
/// fingerprint: `XXXXXXXX..YYYYYYYY` where X is the first 8 and Y the last 8
/// hex chars of the public key.
fn derive_pubkey_fingerprint(seed: &Ed25519Seed) -> String {
    use ed25519_dalek::{SigningKey, VerifyingKey};

    let signing_key = SigningKey::from(seed);
    let verifying_key: VerifyingKey = signing_key.verifying_key();
    let pubkey = verifying_key.to_bytes();
    let hex_pk = hex::encode(pubkey);
    format!("{}..{}", &hex_pk[..8], &hex_pk[56..64])
}
