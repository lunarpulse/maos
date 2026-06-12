#![forbid(unsafe_code)]

//! Sandbox domain types — T3 container isolation attestation artifacts,
//! image-pin chain, and operator-facing error catalog.
//!
//! Per architecture §4.0.9 dependency-triangle rule, this module lives in
//! `maos-domain::sandbox` so operator HTTP API (Story 5.5a/9.4), CLI
//! (Story 5.5a `maosctl spirit inspect --sandbox`), and future MCP
//! registry (Story 5.5d) can consume the surface without depending on
//! `maos-kernel-core`.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// T3ImageAttestation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct T3ImageAttestation {
    #[doc = "Construct via [`T3ImageAttestation::new`] to enforce id/signature/sha validation; struct literals bypass schema checks."]
    pub id: ImageAttestationId,
    #[doc = "Construct via [`T3ImageAttestation::new`] to enforce id/signature/sha validation; struct literals bypass schema checks."]
    pub schema_version: u32,
    #[doc = "Construct via [`T3ImageAttestation::new`] to enforce id/signature/sha validation; struct literals bypass schema checks."]
    pub signed_at_ns: u64,
    #[doc = "Construct via [`T3ImageAttestation::new`] to enforce id/signature/sha validation; struct literals bypass schema checks."]
    pub entries: Vec<T3ImageEntry>,
    #[serde(with = "serde_sig64")]
    #[doc = "Construct via [`T3ImageAttestation::new`] to enforce id/signature/sha validation; struct literals bypass schema checks."]
    pub signature: [u8; 64],
    #[serde(with = "serde_pubkey32")]
    #[doc = "Construct via [`T3ImageAttestation::new`] to enforce id/signature/sha validation; struct literals bypass schema checks."]
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

mod serde_sha256 {
    use serde::{Deserializer, Serializer};
    pub fn serialize<S: Serializer>(sha: &[u8; 32], ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_bytes(sha)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<[u8; 32], D::Error> {
        struct ShaVisitor;
        impl<'de> serde::de::Visitor<'de> for ShaVisitor {
            type Value = [u8; 32];
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a 32-byte SHA-256 digest")
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<[u8; 32], A::Error> {
                let mut arr = [0u8; 32];
                for (i, slot) in arr.iter_mut().enumerate() {
                    *slot = seq.next_element()?.ok_or_else(|| {
                        serde::de::Error::custom(format!("expected 32-byte SHA-256, got {i} bytes"))
                    })?;
                }
                Ok(arr)
            }
        }
        de.deserialize_tuple(32, ShaVisitor)
    }
}

impl T3ImageAttestation {
    /// Construct a `T3ImageAttestation` with validation.
    ///
    /// Enforces:
    /// - `entries` is non-empty.
    /// - Exactly zero or one entry has `default_for_v05 = true`.
    /// - `signature` and `signer_pub_key` are non-zero.
    /// - `schema_version` is 1.
    ///
    /// The `id` is a SHA-256 digest computed by the caller from
    /// `serde_json::to_vec(&entries)` — matching the canonicalization
    /// pattern from Story 5.4's `CrlId`.
    pub fn new(
        id: ImageAttestationId,
        schema_version: u32,
        signed_at_ns: u64,
        entries: Vec<T3ImageEntry>,
        signature: [u8; 64],
        signer_pub_key: [u8; 32],
    ) -> Result<Self, T3Error> {
        if entries.is_empty() {
            return Err(T3Error::SignatureInvalid);
        }

        let default_count = entries.iter().filter(|e| e.default_for_v05).count();
        if default_count > 1 {
            return Err(T3Error::SignatureInvalid);
        }

        if signature == [0u8; 64] {
            return Err(T3Error::SignatureInvalid);
        }
        if signer_pub_key == [0u8; 32] {
            return Err(T3Error::SignatureInvalid);
        }
        if schema_version != 1 {
            return Err(T3Error::UnsupportedSchemaVersion {
                version: schema_version,
            });
        }

        Ok(Self {
            id,
            schema_version,
            signed_at_ns,
            entries,
            signature,
            signer_pub_key,
        })
    }
}

// ---------------------------------------------------------------------------
// T3ImageEntry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct T3ImageEntry {
    #[doc = "Construct via [`T3ImageEntry::new`] to enforce non-empty image_uri and well-formed image_sha256."]
    pub image_uri: String,
    #[serde(with = "serde_sha256")]
    #[doc = "Construct via [`T3ImageEntry::new`] to enforce non-empty image_uri and well-formed image_sha256."]
    pub image_sha256: [u8; 32],
    #[doc = "Construct via [`T3ImageEntry::new`] to enforce non-empty image_uri and well-formed image_sha256."]
    pub description: String,
    #[doc = "Construct via [`T3ImageEntry::new`] to enforce non-empty image_uri and well-formed image_sha256."]
    pub default_for_v05: bool,
}

impl T3ImageEntry {
    /// Construct a `T3ImageEntry` with validation.
    ///
    /// Enforces:
    /// - `image_uri` is non-empty.
    /// - `image_sha256` is non-zero.
    pub fn new(
        image_uri: impl Into<String>,
        image_sha256: [u8; 32],
        description: impl Into<String>,
        default_for_v05: bool,
    ) -> Result<Self, T3Error> {
        let image_uri = image_uri.into();
        if image_uri.is_empty() {
            return Err(T3Error::SignatureInvalid);
        }
        if image_sha256 == [0u8; 32] {
            return Err(T3Error::SignatureInvalid);
        }
        Ok(Self {
            image_uri,
            image_sha256,
            description: description.into(),
            default_for_v05,
        })
    }
}

// ---------------------------------------------------------------------------
// ImageAttestationId
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct ImageAttestationId(#[serde(with = "serde_sha256")] pub [u8; 32]);

impl std::fmt::Display for ImageAttestationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

// ---------------------------------------------------------------------------
// ContainerRuntimeKind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ContainerRuntimeKind {
    Podman,
    Docker,
}

impl ContainerRuntimeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Podman => "podman",
            Self::Docker => "docker",
        }
    }
}

// ---------------------------------------------------------------------------
// T3Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum T3Error {
    #[error(
        "no container runtime found (tried podman, docker); install one or set MAOS_T3_RUNTIME=none to disable T3"
    )]
    RuntimeUnavailable,
    #[error("image SHA mismatch: expected {expected}, observed {observed}")]
    ImageMismatch { expected: String, observed: String },
    #[error("image attestation signature invalid")]
    SignatureInvalid,
    #[error("image attestation trust anchor mismatch")]
    TrustAnchorMismatch,
    #[error("image attestation unsupported schema version {version}")]
    UnsupportedSchemaVersion { version: u32 },
    #[error("image pin '{name}' not found in t3-image.lock")]
    ImagePinMissing { name: String },
    #[error("no default T3 image in t3-image.lock")]
    NoDefaultImage,
    #[error("image trust anchor not configured (set MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX)")]
    TrustAnchorMissing(String),
    #[error("container spawn failed: {0}")]
    Spawn(String),
    #[error("container runtime inspect failed: {0}")]
    Inspect(String),
    #[error(
        "quarantine requested but Spirit form does not support container re-spawn; subprocess form arrives at Epic 6"
    )]
    QuarantineRequiresSubprocessForm,
    #[error("io error: {0}")]
    Io(String),
}

// ---------------------------------------------------------------------------
// SandboxInspectReport
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxInspectReport {
    pub spirit_id: String,
    pub pid: u32,
    pub runtime: String,
    pub image_sha: String,
    pub applied_t2_protections: T2ProtectionSummary,
    pub strictest_of_reasoning: StrictestOfReasoning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct T2ProtectionSummary {
    pub landlock_rules: usize,
    pub seccomp_allow_count: usize,
    pub seccomp_kill_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrictestOfReasoning {
    pub manifest_tier: String,
    pub trust_tier_floor: String,
    pub operator_policy_floor: String,
    pub effective_tier: String,
    pub dominant_axis: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t3_image_attestation_new_rejects_empty_entries() {
        let err = T3ImageAttestation::new(
            ImageAttestationId([1u8; 32]),
            1,
            0,
            vec![],
            [1u8; 64],
            [1u8; 32],
        )
        .unwrap_err();
        assert!(matches!(err, T3Error::SignatureInvalid));
    }

    #[test]
    fn t3_image_attestation_new_rejects_zero_signature() {
        let entry =
            T3ImageEntry::new("ghcr.io/maos/spirit-runtime", [1u8; 32], "test", false).unwrap();
        let err = T3ImageAttestation::new(
            ImageAttestationId([1u8; 32]),
            1,
            0,
            vec![entry],
            [0u8; 64],
            [1u8; 32],
        )
        .unwrap_err();
        assert!(matches!(err, T3Error::SignatureInvalid));
    }

    #[test]
    fn t3_image_attestation_new_rejects_zero_pubkey() {
        let entry =
            T3ImageEntry::new("ghcr.io/maos/spirit-runtime", [1u8; 32], "test", false).unwrap();
        let err = T3ImageAttestation::new(
            ImageAttestationId([1u8; 32]),
            1,
            0,
            vec![entry],
            [1u8; 64],
            [0u8; 32],
        )
        .unwrap_err();
        assert!(matches!(err, T3Error::SignatureInvalid));
    }

    #[test]
    fn t3_image_attestation_new_rejects_multiple_defaults() {
        let e1 = T3ImageEntry::new("ghcr.io/maos/spirit-runtime", [1u8; 32], "one", true).unwrap();
        let e2 =
            T3ImageEntry::new("ghcr.io/maos/spirit-runtime-2", [2u8; 32], "two", true).unwrap();
        let err = T3ImageAttestation::new(
            ImageAttestationId([1u8; 32]),
            1,
            0,
            vec![e1, e2],
            [1u8; 64],
            [1u8; 32],
        )
        .unwrap_err();
        assert!(matches!(err, T3Error::SignatureInvalid));
    }

    #[test]
    fn t3_image_attestation_new_rejects_wrong_schema_version() {
        let entry =
            T3ImageEntry::new("ghcr.io/maos/spirit-runtime", [1u8; 32], "test", false).unwrap();
        let err = T3ImageAttestation::new(
            ImageAttestationId([1u8; 32]),
            2,
            0,
            vec![entry],
            [1u8; 64],
            [1u8; 32],
        )
        .unwrap_err();
        assert!(matches!(
            err,
            T3Error::UnsupportedSchemaVersion { version: 2 }
        ));
    }

    #[test]
    fn t3_image_attestation_new_happy_path() {
        let entry = T3ImageEntry::new("ghcr.io/maos/spirit-runtime", [0xAB; 32], "test desc", true)
            .unwrap();
        let attestation = T3ImageAttestation::new(
            ImageAttestationId([0xCD; 32]),
            1,
            42,
            vec![entry],
            [1u8; 64],
            [2u8; 32],
        )
        .unwrap();
        assert_eq!(attestation.id.0, [0xCD; 32]);
        assert_eq!(attestation.schema_version, 1);
        assert_eq!(attestation.signed_at_ns, 42);
        assert_eq!(attestation.signature, [1u8; 64]);
        assert_eq!(attestation.signer_pub_key, [2u8; 32]);
    }

    #[test]
    fn t3_image_entry_new_rejects_empty_uri() {
        let err = T3ImageEntry::new("", [1u8; 32], "test", false).unwrap_err();
        assert!(matches!(err, T3Error::SignatureInvalid));
    }

    #[test]
    fn t3_image_entry_new_rejects_zero_sha() {
        let err =
            T3ImageEntry::new("ghcr.io/maos/spirit-runtime", [0u8; 32], "test", false).unwrap_err();
        assert!(matches!(err, T3Error::SignatureInvalid));
    }

    #[test]
    fn t3_image_entry_new_happy_path() {
        let e =
            T3ImageEntry::new("ghcr.io/maos/spirit-runtime", [1u8; 32], "test desc", true).unwrap();
        assert_eq!(e.image_uri, "ghcr.io/maos/spirit-runtime");
        assert_eq!(e.image_sha256, [1u8; 32]);
        assert_eq!(e.description, "test desc");
        assert!(e.default_for_v05);
    }

    #[test]
    fn image_attestation_id_display_hex() {
        let id = ImageAttestationId([0xAB; 32]);
        let s = id.to_string();
        assert_eq!(s.len(), 64);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn container_runtime_kind_as_str() {
        assert_eq!(ContainerRuntimeKind::Podman.as_str(), "podman");
        assert_eq!(ContainerRuntimeKind::Docker.as_str(), "docker");
    }

    #[test]
    fn container_runtime_kind_serde_roundtrip() {
        for kind in [ContainerRuntimeKind::Podman, ContainerRuntimeKind::Docker] {
            let json = serde_json::to_string(&kind).unwrap();
            let de: ContainerRuntimeKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, de);
        }
    }

    #[test]
    fn t3_error_display() {
        let e = T3Error::ImagePinMissing {
            name: "foobar".into(),
        };
        assert!(e.to_string().contains("foobar"));
    }

    #[test]
    fn sandbox_inspect_report_serde_roundtrip() {
        let report = SandboxInspectReport {
            spirit_id: "test-spirit".into(),
            pid: 42,
            runtime: "podman".into(),
            image_sha: "abc123".into(),
            applied_t2_protections: T2ProtectionSummary {
                landlock_rules: 7,
                seccomp_allow_count: 58,
                seccomp_kill_count: 14,
            },
            strictest_of_reasoning: StrictestOfReasoning {
                manifest_tier: "T0".into(),
                trust_tier_floor: "T2".into(),
                operator_policy_floor: "T3".into(),
                effective_tier: "T3".into(),
                dominant_axis: "operator".into(),
            },
        };
        let json = serde_json::to_string(&report).unwrap();
        let de: SandboxInspectReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, de);
    }
}
