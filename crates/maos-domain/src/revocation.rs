#![forbid(unsafe_code)]

//! Revocation domain types — CRL artifact, propagation pipeline, and registry client seam.
//!
//! Per architecture §4.0.9 dependency-triangle rule, this module lives in
//! `maos-domain::revocation` so operator HTTP API (Story 5.4/9.4), CLI
//! (Story 5.4 `maosctl revocations import`), and future MCP registry
//! (Story 5.5d) can consume the surface without depending on
//! `maos-kernel-core`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// SignedRevocationList
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedRevocationList {
    #[doc = "Construct via [`SignedRevocationList::new`] to enforce id/signature/origin validation; struct literals bypass schema checks."]
    pub id: CrlId,
    #[doc = "Construct via [`SignedRevocationList::new`] to enforce id/signature/origin validation; struct literals bypass schema checks."]
    pub schema_version: u32,
    #[doc = "Construct via [`SignedRevocationList::new`] to enforce id/signature/origin validation; struct literals bypass schema checks."]
    pub issued_at_ns: u64,
    #[doc = "Construct via [`SignedRevocationList::new`] to enforce id/signature/origin validation; struct literals bypass schema checks."]
    pub origin: RevocationOrigin,
    #[doc = "Construct via [`SignedRevocationList::new`] to enforce id/signature/origin validation; struct literals bypass schema checks."]
    pub entries: Vec<RevocationEntry>,
    #[serde(with = "serde_sig64")]
    #[doc = "Construct via [`SignedRevocationList::new`] to enforce id/signature/origin validation; struct literals bypass schema checks."]
    pub signature: [u8; 64],
    #[serde(with = "serde_pubkey32")]
    #[doc = "Construct via [`SignedRevocationList::new`] to enforce id/signature/origin validation; struct literals bypass schema checks."]
    pub signer_pub_key: [u8; 32],
}

mod serde_sig64 {
    use serde::{Deserializer, Serializer};
    pub fn serialize<S: Serializer>(sig: &[u8; 64], ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_bytes(sig)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<[u8; 64], D::Error> {
        struct SigVisitor;
        impl<'de> serde::de::Visitor<'de> for SigVisitor {
            type Value = [u8; 64];
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a 64-byte array")
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<[u8; 64], A::Error> {
                let mut arr = [0u8; 64];
                for (i, slot) in arr.iter_mut().enumerate() {
                    *slot = seq.next_element()?.ok_or_else(|| {
                        serde::de::Error::custom(format!(
                            "expected 64-byte signature, got {i} bytes"
                        ))
                    })?;
                }
                Ok(arr)
            }
        }
        de.deserialize_tuple(64, SigVisitor)
    }
}

mod serde_pubkey32 {
    use serde::{Deserializer, Serializer};
    pub fn serialize<S: Serializer>(key: &[u8; 32], ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_bytes(key)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<[u8; 32], D::Error> {
        struct PubkeyVisitor;
        impl<'de> serde::de::Visitor<'de> for PubkeyVisitor {
            type Value = [u8; 32];
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a 32-byte array")
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<[u8; 32], A::Error> {
                let mut arr = [0u8; 32];
                for (i, slot) in arr.iter_mut().enumerate() {
                    *slot = seq.next_element()?.ok_or_else(|| {
                        serde::de::Error::custom(format!("expected 32-byte pubkey, got {i} bytes"))
                    })?;
                }
                Ok(arr)
            }
        }
        de.deserialize_tuple(32, PubkeyVisitor)
    }
}

/// Compact JSON encoding with recursively sorted object keys.
///
/// This is the sole CRL byte representation: signatures and `CrlId`s both
/// derive from it. Arrays retain their declared order; object keys are sorted
/// at every depth, so producer struct-field order cannot affect verification.
pub fn canonical_entries_bytes(entries: &[RevocationEntry]) -> Result<Vec<u8>, RevocationError> {
    fn write(value: &serde_json::Value, output: &mut Vec<u8>) -> Result<(), RevocationError> {
        match value {
            serde_json::Value::Null => output.extend_from_slice(b"null"),
            serde_json::Value::Bool(value) => {
                output.extend_from_slice(if *value { b"true" } else { b"false" })
            }
            serde_json::Value::Number(value) => {
                output.extend_from_slice(value.to_string().as_bytes())
            }
            serde_json::Value::String(value) => {
                serde_json::to_writer(&mut *output, value).map_err(|error| {
                    RevocationError::Deserialize(format!("JSON string serialize: {error}"))
                })?;
            }
            serde_json::Value::Array(values) => {
                output.push(b'[');
                for (index, value) in values.iter().enumerate() {
                    if index != 0 {
                        output.push(b',');
                    }
                    write(value, output)?;
                }
                output.push(b']');
            }
            serde_json::Value::Object(values) => {
                output.push(b'{');
                let mut keys: Vec<_> = values.keys().collect();
                keys.sort_unstable();
                for (index, key) in keys.iter().enumerate() {
                    if index != 0 {
                        output.push(b',');
                    }
                    serde_json::to_writer(&mut *output, *key).map_err(|error| {
                        RevocationError::Deserialize(format!("JSON object key serialize: {error}"))
                    })?;
                    output.push(b':');
                    write(&values[*key], output)?;
                }
                output.push(b'}');
            }
        }
        Ok(())
    }

    let value = serde_json::to_value(entries)
        .map_err(|error| RevocationError::Deserialize(format!("entries serialize: {error}")))?;
    let mut output = Vec::new();
    write(&value, &mut output)?;
    Ok(output)
}

impl SignedRevocationList {
    /// Construct a `SignedRevocationList` with validation.
    ///
    /// Enforces:
    /// - every entry satisfies `RevocationEntry::new` validation.
    /// - `id` is SHA-256 of the canonical entries bytes.
    /// - `schema_version` is 1 (v0.3-β only accepts 1).
    /// - `signature` and `signer_pub_key` are non-zero (basic sanity).
    pub fn new(
        id: CrlId,
        schema_version: u32,
        issued_at_ns: u64,
        origin: RevocationOrigin,
        entries: Vec<RevocationEntry>,
        signature: [u8; 64],
        signer_pub_key: [u8; 32],
    ) -> Result<Self, RevocationError> {
        if entries.is_empty() {
            return Err(RevocationError::Deserialize(
                "CRL entries must be non-empty".into(),
            ));
        }
        if schema_version != 1 {
            return Err(RevocationError::UnsupportedSchemaVersion {
                actual: schema_version,
            });
        }
        if signature == [0u8; 64] {
            return Err(RevocationError::SignatureInvalid);
        }
        if signer_pub_key == [0u8; 32] {
            return Err(RevocationError::SignatureInvalid);
        }
        let entries = entries
            .into_iter()
            .map(|entry| {
                RevocationEntry::new(
                    entry.spirit_class,
                    entry.version_range,
                    entry.reason,
                    entry.recommended_action,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        let expected_id = CrlId::from_entries(&entries)?;
        if id != expected_id {
            return Err(RevocationError::CrlIdMismatch {
                expected: expected_id.to_string(),
                actual: id.to_string(),
            });
        }
        Ok(Self {
            id,
            schema_version,
            issued_at_ns,
            origin,
            entries,
            signature,
            signer_pub_key,
        })
    }
}

// ---------------------------------------------------------------------------
// RevocationEntry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevocationEntry {
    #[doc = "Construct via [`RevocationEntry::new`] to enforce non-empty spirit_class and well-formed version_range."]
    pub spirit_class: String,
    #[doc = "Construct via [`RevocationEntry::new`] to enforce non-empty spirit_class and well-formed version_range."]
    pub version_range: String,
    #[doc = "Construct via [`RevocationEntry::new`] to enforce non-empty spirit_class and well-formed version_range."]
    pub reason: String,
    #[doc = "Construct via [`RevocationEntry::new`] to enforce non-empty spirit_class and well-formed version_range."]
    pub recommended_action: Option<RevocationAction>,
}

impl RevocationEntry {
    /// Construct a `RevocationEntry` with validation.
    ///
    /// Enforces:
    /// - `spirit_class` is non-empty and matches `[a-z0-9-]+`.
    /// - `version_range` is non-empty and parses as a valid semver range.
    pub fn new(
        spirit_class: impl Into<String>,
        version_range: impl Into<String>,
        reason: impl Into<String>,
        recommended_action: Option<RevocationAction>,
    ) -> Result<Self, RevocationError> {
        let spirit_class = spirit_class.into();
        let version_range = version_range.into();
        let reason = reason.into();

        if spirit_class.is_empty() {
            return Err(RevocationError::MalformedVersionRange {
                range: String::new(),
                reason: "spirit_class must be non-empty".into(),
            });
        }
        if !spirit_class
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(RevocationError::MalformedVersionRange {
                range: spirit_class.clone(),
                reason: "spirit_class must match [a-z0-9-]+".into(),
            });
        }
        if version_range.is_empty() {
            return Err(RevocationError::MalformedVersionRange {
                range: String::new(),
                reason: "version_range must be non-empty".into(),
            });
        }
        // Validate the range parses
        let _ =
            parse_range(&version_range).map_err(|e| RevocationError::MalformedVersionRange {
                range: version_range.clone(),
                reason: e.to_string(),
            })?;

        Ok(Self {
            spirit_class,
            version_range,
            reason,
            recommended_action,
        })
    }
}

// ---------------------------------------------------------------------------
// RevocationOrigin
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RevocationOrigin {
    Operator,
    Publisher,
    RegistryYank,
}

impl RevocationOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Operator => "operator",
            Self::Publisher => "publisher",
            Self::RegistryYank => "registry_yank",
        }
    }
}

// ---------------------------------------------------------------------------
// RevocationAction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RevocationAction {
    #[default]
    TerminateImmediately,
    DrainThenTerminate,
    Quarantine,
}

impl RevocationAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TerminateImmediately => "terminate-immediately",
            Self::DrainThenTerminate => "drain-then-terminate",
            Self::Quarantine => "quarantine",
        }
    }
}

// ---------------------------------------------------------------------------
// CrlId
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct CrlId(pub [u8; 32]);

impl CrlId {
    /// Derive the CRL identity from the same canonical bytes that are signed.
    pub fn from_entries(entries: &[RevocationEntry]) -> Result<Self, RevocationError> {
        let bytes = canonical_entries_bytes(entries)?;
        let digest: [u8; 32] = Sha256::digest(bytes).into();
        Ok(Self(digest))
    }
}

impl std::fmt::Display for CrlId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

// ---------------------------------------------------------------------------
// ApplyReport + ApplyEntry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ApplyReport {
    pub crl_id: CrlId,
    pub origin: RevocationOrigin,
    pub matched_count: usize,
    pub revoked_count: usize,
    pub halt_receipts_produced: usize,
    pub tokens_revoked_total: usize,
    pub apply_latency_ns: u64,
    pub per_spirit: Vec<ApplyEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ApplyEntry {
    pub spirit_id: String,
    pub spirit_pid: u32,
    pub spirit_class: String,
    pub spirit_version: String,
    pub action: RevocationAction,
    pub tokens_revoked: usize,
    pub halt_receipts_produced: usize,
    pub in_flight_token_count: usize,
}

// ---------------------------------------------------------------------------
// RevocationError
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum RevocationError {
    #[error("CRL signature verification failed")]
    SignatureInvalid,
    #[error("CRL schema_version {actual} unsupported (v0.3-β only accepts 1)")]
    UnsupportedSchemaVersion { actual: u32 },
    #[error("CRL entry version_range '{range}' is malformed: {reason}")]
    MalformedVersionRange { range: String, reason: String },
    #[error("CRL trust anchor not configured (set MAOS_CRL_TRUST_ANCHOR_PUB_HEX)")]
    TrustAnchorMissing,
    #[error("CRL trust anchor public key mismatch (expected one of N pinned, got {observed})")]
    TrustAnchorMismatch { observed: String },
    #[error("CRL deserialization failed: {0}")]
    Deserialize(String),
    #[error("CRL already applied (id={id})")]
    AlreadyApplied { id: String },
    #[error("Registry client returned error: {0}")]
    RegistryClient(String),
    #[error("I/O error reading offline CRL: {0}")]
    Io(String),
    #[error("quarantine failed: {0}")]
    QuarantineFailed(String),
    #[error("unsupported revocation action: {0}")]
    UnsupportedAction(String),
    #[error("CRL id does not match canonical entries digest (expected {expected}, got {actual})")]
    CrlIdMismatch { expected: String, actual: String },
}

// ---------------------------------------------------------------------------
// RegistryClient trait
// ---------------------------------------------------------------------------

pub trait RegistryClient: Send + Sync + 'static {
    /// Fetch the latest signed CRL from the registry. Production impl
    /// (Story 5.5d) calls the MCP-Streamable-HTTP `registry.crl` op;
    /// v0.3-β default `LocalFileRegistryClient` reads
    /// `~/.local/share/maos/crl/latest.signed.json`.
    fn fetch_signed_crl(&self) -> Result<Vec<u8>, RevocationError>;

    /// Return the trust anchor injected into this registry instance.
    fn trust_anchor_pub(&self) -> Result<Vec<u8>, RevocationError>;
}

// ---------------------------------------------------------------------------
// LocalFileRegistryClient
// ---------------------------------------------------------------------------

/// v0.3-β default `RegistryClient` that reads CRL from local filesystem.
pub struct LocalFileRegistryClient {
    crl_dir: PathBuf,
    trust_anchor_pub: Option<Vec<u8>>,
}

impl LocalFileRegistryClient {
    /// Construct a local registry with the trust anchor supplied by the
    /// composition root.  The client never reads mutable process environment.
    pub fn new(crl_dir: impl Into<PathBuf>, trust_anchor_pub: Option<Vec<u8>>) -> Self {
        Self {
            crl_dir: crl_dir.into(),
            trust_anchor_pub,
        }
    }
}

impl RegistryClient for LocalFileRegistryClient {
    fn fetch_signed_crl(&self) -> Result<Vec<u8>, RevocationError> {
        let path = self.crl_dir.join("latest.signed.json");
        std::fs::read(&path).map_err(|error| {
            RevocationError::Io(format!("read CRL file {}: {error}", path.display()))
        })
    }

    fn trust_anchor_pub(&self) -> Result<Vec<u8>, RevocationError> {
        self.trust_anchor_pub
            .clone()
            .ok_or(RevocationError::TrustAnchorMissing)
    }
}

// ---------------------------------------------------------------------------
// Semver range matching (minimal hand-rolled parser; no semver dep)
// ---------------------------------------------------------------------------

/// Parse a minimal semver range string.
///
/// Supported shapes:
/// - `"*"` — matches any version.
/// - `"0.1.0"` — exact version match.
/// - `">=0.1.0,<0.2.0"` — comma-separated comparators (AND semantics).
///
/// Returns a `VersionReq` that can be tested with `matches`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionReq {
    comparators: Vec<Comparator>,
    wildcard: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Comparator {
    Gte(String),
    Gt(String),
    Lte(String),
    Lt(String),
    Eq(String),
}

/// Parse a range string. Returns `Err` on unsupported syntax.
pub fn parse_range(range: &str) -> Result<VersionReq, String> {
    let range = range.trim();
    if range.is_empty() {
        return Err("empty version range".into());
    }
    if range == "*" {
        return Ok(VersionReq {
            comparators: vec![],
            wildcard: true,
        });
    }

    // Exact version: "0.1.0" or "0.1.0-alpha" (no comparator prefix)
    if !range.starts_with('>') && !range.starts_with('<') && !range.starts_with('=') {
        // Basic sanity: exact versions must look like semver (digits + dots + optional alpha)
        let valid = range
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c.is_ascii_alphabetic());
        if !valid {
            return Err(format!(
                "exact version '{range}' contains invalid characters"
            ));
        }
        return Ok(VersionReq {
            comparators: vec![Comparator::Eq(range.to_string())],
            wildcard: false,
        });
    }

    let mut comparators = Vec::new();
    for part in range.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(rest) = part.strip_prefix(">=") {
            let v = rest.trim();
            if v.is_empty() {
                return Err(format!(
                    "empty version operand in comparator part: '{part}'"
                ));
            }
            comparators.push(Comparator::Gte(v.to_string()));
        } else if let Some(rest) = part.strip_prefix(">") {
            let v = rest.trim();
            if v.is_empty() {
                return Err(format!(
                    "empty version operand in comparator part: '{part}'"
                ));
            }
            comparators.push(Comparator::Gt(v.to_string()));
        } else if let Some(rest) = part.strip_prefix("<=") {
            let v = rest.trim();
            if v.is_empty() {
                return Err(format!(
                    "empty version operand in comparator part: '{part}'"
                ));
            }
            comparators.push(Comparator::Lte(v.to_string()));
        } else if let Some(rest) = part.strip_prefix("<") {
            let v = rest.trim();
            if v.is_empty() {
                return Err(format!(
                    "empty version operand in comparator part: '{part}'"
                ));
            }
            comparators.push(Comparator::Lt(v.to_string()));
        } else if let Some(rest) = part.strip_prefix("=") {
            let v = rest.trim();
            if v.is_empty() {
                return Err(format!(
                    "empty version operand in comparator part: '{part}'"
                ));
            }
            comparators.push(Comparator::Eq(v.to_string()));
        } else {
            return Err(format!("unsupported comparator in range part: '{part}'"));
        }
    }

    if comparators.is_empty() {
        return Err("empty comparator list after parsing".into());
    }

    Ok(VersionReq {
        comparators,
        wildcard: false,
    })
}

/// Test whether `version` satisfies `req`.
///
/// Versions are compared lexicographically by numeric dot-separated
/// components (e.g. `0.1.0` < `0.2.0` < `1.0.0`). Pre-release suffixes
/// are compared as strings.
pub fn semver_range_contains(version: &str, range: &str) -> Result<bool, RevocationError> {
    let req = parse_range(range).map_err(|reason| RevocationError::MalformedVersionRange {
        range: range.to_string(),
        reason,
    })?;
    Ok(matches_version(version, &req))
}

fn matches_version(version: &str, req: &VersionReq) -> bool {
    if req.wildcard {
        return true;
    }
    req.comparators.iter().all(|c| match c {
        Comparator::Eq(v) => version == v,
        Comparator::Gte(v) => compare_versions(version, v) != std::cmp::Ordering::Less,
        Comparator::Gt(v) => compare_versions(version, v) == std::cmp::Ordering::Greater,
        Comparator::Lte(v) => compare_versions(version, v) != std::cmp::Ordering::Greater,
        Comparator::Lt(v) => compare_versions(version, v) == std::cmp::Ordering::Less,
    })
}

/// Lexicographic comparison of dot-separated numeric version strings.
/// Follows semver §11.4: a version without a pre-release suffix has
/// HIGHER precedence than a version with one.
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();
    let max_len = a_parts.len().max(b_parts.len());
    for i in 0..max_len {
        let a_raw = a_parts.get(i).copied().unwrap_or("0");
        let b_raw = b_parts.get(i).copied().unwrap_or("0");

        let (a_num_part, a_has_pre) = split_pre(a_raw);
        let (b_num_part, b_has_pre) = split_pre(b_raw);

        match (a_num_part.parse::<u64>(), b_num_part.parse::<u64>()) {
            (Ok(a_num), Ok(b_num)) => match a_num.cmp(&b_num) {
                std::cmp::Ordering::Equal => {
                    // Same numeric — pre-release check
                    match (a_has_pre, b_has_pre) {
                        (false, false) => continue,
                        (false, true) => return std::cmp::Ordering::Greater,
                        (true, false) => return std::cmp::Ordering::Less,
                        (true, true) => match a_raw.cmp(b_raw) {
                            std::cmp::Ordering::Equal => continue,
                            other => return other,
                        },
                    }
                }
                other => return other,
            },
            _ => match a_raw.cmp(b_raw) {
                std::cmp::Ordering::Equal => continue,
                other => return other,
            },
        }
    }
    std::cmp::Ordering::Equal
}

/// Split a version component at the first `-`. Returns (numeric_prefix, has_pre_release).
/// e.g. `"0-alpha"` → `("0", true)`, `"0"` → `("0", false)`.
fn split_pre(s: &str) -> (&str, bool) {
    match s.split_once('-') {
        Some((num, _pre)) => (num, true),
        None => (s, false),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revocation_action_default_is_terminate_immediately() {
        assert_eq!(
            RevocationAction::default(),
            RevocationAction::TerminateImmediately
        );
    }

    #[test]
    fn revocation_origin_serde_roundtrip() {
        for origin in [
            RevocationOrigin::Operator,
            RevocationOrigin::Publisher,
            RevocationOrigin::RegistryYank,
        ] {
            let json = serde_json::to_string(&origin).unwrap();
            let de: RevocationOrigin = serde_json::from_str(&json).unwrap();
            assert_eq!(origin, de);
        }
    }

    #[test]
    fn signed_revocation_list_new_rejects_empty_entries() {
        let err = SignedRevocationList::new(
            CrlId([0u8; 32]),
            1,
            0,
            RevocationOrigin::Operator,
            vec![],
            [1u8; 64],
            [1u8; 32],
        )
        .unwrap_err();
        assert!(matches!(err, RevocationError::Deserialize(..)));
    }

    #[test]
    fn signed_revocation_list_new_rejects_wrong_schema_version() {
        let err = SignedRevocationList::new(
            CrlId([0u8; 32]),
            2,
            0,
            RevocationOrigin::Operator,
            vec![RevocationEntry::new("hello", "*", "test", None).unwrap()],
            [1u8; 64],
            [1u8; 32],
        )
        .unwrap_err();
        assert!(matches!(
            err,
            RevocationError::UnsupportedSchemaVersion { actual: 2 }
        ));
    }

    #[test]
    fn revocation_entry_new_rejects_empty_spirit_class() {
        let err = RevocationEntry::new("", "*", "test", None).unwrap_err();
        assert!(matches!(err, RevocationError::MalformedVersionRange { .. }));
    }

    #[test]
    fn revocation_entry_new_rejects_invalid_spirit_class() {
        let err = RevocationEntry::new("Hello_World", "*", "test", None).unwrap_err();
        assert!(matches!(err, RevocationError::MalformedVersionRange { .. }));
    }

    #[test]
    fn semver_range_contains_in_range() {
        assert_eq!(
            semver_range_contains("0.1.5", ">=0.1.0,<0.2.0").unwrap(),
            true
        );
    }

    #[test]
    fn semver_range_contains_out_of_range() {
        assert_eq!(
            semver_range_contains("0.2.0", ">=0.1.0,<0.2.0").unwrap(),
            false
        );
    }

    #[test]
    fn semver_range_exact_match() {
        assert_eq!(semver_range_contains("0.1.0", "0.1.0").unwrap(), true);
        assert_eq!(semver_range_contains("0.1.1", "0.1.0").unwrap(), false);
    }

    #[test]
    fn semver_range_wildcard() {
        assert_eq!(semver_range_contains("99.99.99", "*").unwrap(), true);
    }

    #[test]
    fn semver_range_malformed_returns_err() {
        let err = semver_range_contains("0.1.0", "~1.0.0").unwrap_err();
        assert!(matches!(err, RevocationError::MalformedVersionRange { .. }));
    }

    #[test]
    fn registry_client_trait_object_safe() {
        fn _accepts_dyn(_: &dyn RegistryClient) {}
        let _: std::sync::Arc<dyn RegistryClient> =
            std::sync::Arc::new(LocalFileRegistryClient::new("/tmp", None));
    }

    #[test]
    fn crl_id_display_hex() {
        let id = CrlId([0xAB; 32]);
        let s = id.to_string();
        assert_eq!(s.len(), 64);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn crl_id_is_the_sha256_of_recursively_key_sorted_entries() {
        let entry = RevocationEntry::new("worker", "1.0.0", "test", None).unwrap();
        let id = CrlId::from_entries(&[entry]).unwrap();
        assert_eq!(
            id.to_string(),
            "420b7542a679d0d85503662c43980f5313ac55528e42032a19461058f17f4063"
        );
    }

    #[test]
    fn revocation_action_as_str_roundtrip() {
        assert_eq!(
            RevocationAction::TerminateImmediately.as_str(),
            "terminate-immediately"
        );
        assert_eq!(
            RevocationAction::DrainThenTerminate.as_str(),
            "drain-then-terminate"
        );
        assert_eq!(RevocationAction::Quarantine.as_str(), "quarantine");
    }

    #[test]
    fn revocation_origin_as_str_roundtrip() {
        assert_eq!(RevocationOrigin::Operator.as_str(), "operator");
        assert_eq!(RevocationOrigin::Publisher.as_str(), "publisher");
        assert_eq!(RevocationOrigin::RegistryYank.as_str(), "registry_yank");
    }
}
