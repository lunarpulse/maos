//! Spirit Registry port per ADR-008 + ADR-010.
//!
//! Kernel-internal admission consumer + operator CLI consumer.
//!
//! Per ADR-010 sync-only port semantics.  Adapter impls live in
//! `maos-registry` (production `McpSpiritRegistryClient` + test
//! `FixtureReplaySpiritRegistryClient`).  Async callers wrap in
//! `spawn_blocking`.
//!
//! # Reused types
//!
//! - `SpiritId` — re-exported from `maos_spirit_abi::identity`.
//! - `TrustTier` — re-exported from `maos_spirit_abi::compliance`.
//! - `ComplianceClaimEnvelope` — re-exported from `maos_spirit_abi::compliance`.

pub use maos_spirit_abi::compliance::{ComplianceClaimEnvelope, TrustTier};
pub use maos_spirit_abi::identity::SpiritId;

/// Spirit Registry client port — kernel adapter contract.
///
/// Five operations per ADR-008 binding-v0.5.  Per ADR-010 sync-only
/// port semantics.  The production adapter (`McpSpiritRegistryClient`
/// in `maos-registry`) routes through Story 5.5c's `McpClient::call`.
pub trait SpiritRegistryClient: Send + Sync {
    /// Class: data-movement
    ///
    /// Searches the registry for Spirits matching `q`.
    fn search(&self, q: &SearchQuery) -> Result<SearchResults, RegistryError>;

    /// Class: data-movement
    ///
    /// Fetches a signed manifest for the (spirit_id, version) tuple.
    fn manifest(
        &self,
        spirit_id: &SpiritId,
        version: &str,
    ) -> Result<SignedManifest, RegistryError>;

    /// Class: data-movement
    ///
    /// Fetches a signed binary artifact.
    fn artifact(
        &self,
        spirit_id: &SpiritId,
        version: &str,
    ) -> Result<SignedArtifact, RegistryError>;

    /// Class: data-movement
    ///
    /// Publishes a signed package.  Publisher-side op.
    fn publish(&self, pkg: &SignedPackage) -> Result<PublishReceipt, RegistryError>;

    /// Class: data-movement
    ///
    /// Yanks a version.  Publisher- OR operator-side op.
    fn deprecate(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        reason: &YankReason,
    ) -> Result<YankReceipt, RegistryError>;
}

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

/// Search query for the registry.
///
/// `text` is required (non-empty).  `include_yanked` defaults to `false`.
/// `limit` defaults to 50 and is capped at 200.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SearchQuery {
    #[doc = "Construct via [`SearchQuery::new`] to enforce non-empty text."]
    pub text: String,
    #[doc = "Construct via [`SearchQuery::new`].  Default false."]
    #[serde(default)]
    pub include_yanked: bool,
    #[doc = "Construct via [`SearchQuery::new`].  Default 50; max 200."]
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    50
}

impl SearchQuery {
    /// Construct a `SearchQuery`, applying `limit` clamping.
    /// Enforces non-empty text per doc contract.
    pub fn new(text: String, include_yanked: bool, limit: u32) -> Self {
        assert!(!text.trim().is_empty(), "SearchQuery::new: text must be non-empty");
        Self {
            text,
            include_yanked,
            limit: limit.min(200).max(1),
        }
    }
}

/// A single item in search results.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SearchResultItem {
    #[doc = "Construct via [`SearchResultItem::new`]."]
    pub spirit_id: SpiritId,
    #[doc = "Construct via [`SearchResultItem::new`]."]
    pub version: String,
    #[doc = "Construct via [`SearchResultItem::new`]."]
    pub summary: String,
}

impl SearchResultItem {
    pub fn new(spirit_id: SpiritId, version: String, summary: String) -> Self {
        Self {
            spirit_id,
            version,
            summary,
        }
    }
}

/// Search results from the registry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SearchResults {
    #[doc = "Construct via [`SearchResults::new`]."]
    pub items: Vec<SearchResultItem>,
}

impl SearchResults {
    pub fn new(items: Vec<SearchResultItem>) -> Self {
        Self { items }
    }
}

/// Signed manifest fetched from the registry.
///
/// Story 7.2 (closes 5.5d High [edge] `defer`) added the optional
/// `server_reported_tier` + `server_signature_on_tier` fields so the
/// kernel-side caller can cross-verify the manifest-declared trust tier
/// against the server's reported tier. Both fields are `Option` and
/// `#[serde(default)]` so the v0.5→v1.0 migration window tolerates servers
/// that have not yet been upgraded — when operators flip
/// `[registry].require_server_tier_signature = true` the caller treats
/// missing fields as `RegistryError::ServerTierSignatureRequired`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedManifest {
    #[doc = "Construct via [`SignedManifest::new`]."]
    pub spirit_id: SpiritId,
    #[doc = "Construct via [`SignedManifest::new`]."]
    pub version: String,
    #[doc = "Construct via [`SignedManifest::new`] — raw TOML bytes of the manifest."]
    pub manifest_toml: Vec<u8>,
    #[doc = "Construct via [`SignedManifest::new`] — Ed25519 signature (64 bytes)."]
    pub signature: [u8; 64],
    #[doc = "Construct via [`SignedManifest::new`] — signer's Ed25519 public key (32 bytes)."]
    pub signer_pubkey: [u8; 32],
    #[doc = "Story 7.2 (additive) — server-reported trust tier for consumer-side cross-verification."]
    pub server_reported_tier: Option<TrustTier>,
    #[doc = "Story 7.2 (additive) — server's Ed25519 signature over `(spirit_id || version || tier_byte)`."]
    pub server_signature_on_tier: Option<[u8; 64]>,
}

impl SignedManifest {
    pub fn new(
        spirit_id: SpiritId,
        version: String,
        manifest_toml: Vec<u8>,
        signature: [u8; 64],
        signer_pubkey: [u8; 32],
    ) -> Self {
        Self {
            spirit_id,
            version,
            manifest_toml,
            signature,
            signer_pubkey,
            server_reported_tier: None,
            server_signature_on_tier: None,
        }
    }

    /// Story 7.2 — construct a SignedManifest WITH the server-reported tier
    /// metadata populated. Server-side handlers (registry.manifest) call
    /// this; client-side cross-verification consumes the populated fields.
    pub fn with_server_tier(
        mut self,
        server_reported_tier: TrustTier,
        server_signature_on_tier: [u8; 64],
    ) -> Self {
        self.server_reported_tier = Some(server_reported_tier);
        self.server_signature_on_tier = Some(server_signature_on_tier);
        self
    }
}

/// Signed artifact fetched from the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedArtifact {
    #[doc = "Construct via [`SignedArtifact::new`]."]
    pub spirit_id: SpiritId,
    #[doc = "Construct via [`SignedArtifact::new`]."]
    pub version: String,
    #[doc = "Construct via [`SignedArtifact::new`] — binary artifact bytes."]
    pub artifact_bytes: Vec<u8>,
    #[doc = "Construct via [`SignedArtifact::new`] — Ed25519 signature (64 bytes)."]
    pub signature: [u8; 64],
    #[doc = "Construct via [`SignedArtifact::new`] — signer's Ed25519 public key (32 bytes)."]
    pub signer_pubkey: [u8; 32],
}

impl SignedArtifact {
    pub fn new(
        spirit_id: SpiritId,
        version: String,
        artifact_bytes: Vec<u8>,
        signature: [u8; 64],
        signer_pubkey: [u8; 32],
    ) -> Self {
        Self {
            spirit_id,
            version,
            artifact_bytes,
            signature,
            signer_pubkey,
        }
    }
}

/// On-wire publishable unit per ADR-008.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedPackage {
    #[doc = "Construct via [`SignedPackage::new`] to enforce well-formed signature shape."]
    pub spirit_id: SpiritId,
    #[doc = "Construct via [`SignedPackage::new`]."]
    pub version: String,
    #[doc = "Construct via [`SignedPackage::new`] — raw TOML bytes of the Spirit manifest."]
    pub manifest_toml: Vec<u8>,
    #[doc = "Construct via [`SignedPackage::new`] — Spirit binary blob."]
    pub artifact_bytes: Vec<u8>,
    #[doc = "Construct via [`SignedPackage::new`] — Ed25519 signature over sha256(manifest_toml || artifact_bytes)."]
    pub signature: [u8; 64],
    #[doc = "Construct via [`SignedPackage::new`] — publisher's Ed25519 public key."]
    pub publisher_pubkey: [u8; 32],
    #[doc = "Construct via [`SignedPackage::new`] — frozen-schema ComplianceClaim per §8.5."]
    pub compliance_envelope: ComplianceClaimEnvelope,
}

impl SignedPackage {
    /// Construct a `SignedPackage`, validating signature length.
    pub fn new(
        spirit_id: SpiritId,
        version: String,
        manifest_toml: Vec<u8>,
        artifact_bytes: Vec<u8>,
        signature: [u8; 64],
        publisher_pubkey: [u8; 32],
        compliance_envelope: ComplianceClaimEnvelope,
    ) -> Self {
        Self {
            spirit_id,
            version,
            manifest_toml,
            artifact_bytes,
            signature,
            publisher_pubkey,
            compliance_envelope,
        }
    }
}

/// Receipt returned after a successful publish.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PublishReceipt {
    #[doc = "Construct via [`PublishReceipt::new`]."]
    pub publish_id: String,
    #[doc = "Construct via [`PublishReceipt::new`]."]
    pub spirit_id: SpiritId,
    #[doc = "Construct via [`PublishReceipt::new`]."]
    pub version: String,
}

impl PublishReceipt {
    pub fn new(publish_id: String, spirit_id: SpiritId, version: String) -> Self {
        Self {
            publish_id,
            spirit_id,
            version,
        }
    }
}

/// Reason a version was yanked.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct YankReason {
    #[doc = "Construct via [`YankReason::new`]."]
    pub summary: String,
}

impl YankReason {
    pub fn new(summary: String) -> Self {
        Self { summary }
    }
}

/// Receipt returned after a successful yank.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct YankReceipt {
    #[doc = "Construct via [`YankReceipt::new`]."]
    pub yank_id: String,
    #[doc = "Construct via [`YankReceipt::new`]."]
    pub spirit_id: SpiritId,
    #[doc = "Construct via [`YankReceipt::new`]."]
    pub version: String,
}

impl YankReceipt {
    pub fn new(yank_id: String, spirit_id: SpiritId, version: String) -> Self {
        Self {
            yank_id,
            spirit_id,
            version,
        }
    }
}

/// A single yank entry in the registry's yank list.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct YankEntry {
    #[doc = "Construct via [`YankEntry::new`]."]
    pub spirit_id: SpiritId,
    #[doc = "Construct via [`YankEntry::new`]."]
    pub version: String,
    #[doc = "Construct via [`YankEntry::new`] — monotonic nanosecond timestamp."]
    pub yanked_at_ns: u64,
    #[doc = "Construct via [`YankEntry::new`]."]
    pub reason: String,
}

impl YankEntry {
    pub fn new(
        spirit_id: SpiritId,
        version: String,
        yanked_at_ns: u64,
        reason: String,
    ) -> Self {
        Self {
            spirit_id,
            version,
            yanked_at_ns,
            reason,
        }
    }
}

/// List of yank entries returned by `registry.yanks_since`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct YankList {
    #[doc = "Construct via [`YankList::new`]."]
    pub entries: Vec<YankEntry>,
}

impl YankList {
    pub fn new(entries: Vec<YankEntry>) -> Self {
        Self { entries }
    }
}

// ---------------------------------------------------------------------------
// RegistryError — typed error enum
// ---------------------------------------------------------------------------

/// Typed registry error per ADR-008 + FR63.
///
/// `#[non_exhaustive]` preserves forward-compat for additive variants.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum RegistryError {
    #[error("unknown spirit '{0}'")]
    UnknownSpirit(String),

    #[error("version '{requested}' not found for spirit '{spirit_id}'")]
    VersionNotFound {
        spirit_id: String,
        requested: String,
    },

    #[error("Ed25519 signature verification failed")]
    SignatureInvalid,

    #[error("trust-tier floor violated: manifest='{manifest_tier:?}', floor='{floor:?}'")]
    TrustTierFloorViolated {
        manifest_tier: TrustTier,
        floor: TrustTier,
    },

    #[error("ComplianceClaim execution-context fingerprint drift: actual={actual_hex}, claimed={claimed_hex}")]
    ComplianceContextDrift {
        actual_hex: String,
        claimed_hex: String,
    },

    #[error("registry version '{spirit_id}@{version}' yanked: {reason}")]
    Yanked {
        spirit_id: String,
        version: String,
        reason: String,
    },

    #[error("registry transport error: {0}")]
    Transport(String),

    #[error(
        "registry not configured (set MAOS_REGISTRY_URI or [registry].uri in operator.toml)"
    )]
    Unconfigured,

    #[error("org signature does not match operator-configured org key")]
    OrgSignatureInvalid,

    #[error("public_vetted trust tier deferred per FR37 to v2.5")]
    PublicVettedDeferred,

    #[error(
        "consumer-side trust-tier server-side cross-check failed: manifest='{manifest_tier:?}', server-reported='{server_reported_tier:?}'"
    )]
    TrustTierServerMismatch {
        manifest_tier: TrustTier,
        server_reported_tier: TrustTier,
    },

    #[error(
        "consumer-side trust-tier server-side signature missing or invalid: {reason} (operator requires `[registry].require_server_tier_signature=true`)"
    )]
    ServerTierSignatureRequired {
        reason: String,
    },

    #[error(
        "server-reported tier '{server_reported:?}' exceeds operator floor '{operator_floor:?}'"
    )]
    TierFloorViolation {
        server_reported: TrustTier,
        operator_floor: TrustTier,
    },

    #[error("server tier signature invalid — possible tampering or wrong org key")]
    ServerTierSignatureInvalid,
}

// ---------------------------------------------------------------------------
// Serde impls for [u8; N] types
// ---------------------------------------------------------------------------
// to properly handle [u8; N] arrays as sequences.

impl serde::Serialize for SignedManifest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SignedManifest", 7)?;
        state.serialize_field("spirit_id", &self.spirit_id)?;
        state.serialize_field("version", &self.version)?;
        state.serialize_field("manifest_toml", &self.manifest_toml)?;
        state.serialize_field("signature", &hex::encode(&self.signature[..]))?;
        state.serialize_field("signer_pubkey", &hex::encode(&self.signer_pubkey[..]))?;
        state.serialize_field("server_reported_tier", &self.server_reported_tier)?;
        let server_sig_hex = self
            .server_signature_on_tier
            .as_ref()
            .map(|s| hex::encode(&s[..]));
        state.serialize_field("server_signature_on_tier", &server_sig_hex)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for SignedManifest {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Helper {
            spirit_id: SpiritId,
            version: String,
            manifest_toml: Vec<u8>,
            signature: String,
            signer_pubkey: String,
            #[serde(default)]
            server_reported_tier: Option<TrustTier>,
            #[serde(default)]
            server_signature_on_tier: Option<String>,
        }
        let h = Helper::deserialize(deserializer)?;
        let signature = hex::decode(&h.signature)
            .map_err(|e| serde::de::Error::custom(format!("signature hex decode: {e}")))?
            .try_into()
            .map_err(|v: Vec<u8>| {
                serde::de::Error::custom(format!(
                    "expected 64-byte signature, got {} bytes",
                    v.len()
                ))
            })?;
        let signer_pubkey = hex::decode(&h.signer_pubkey)
            .map_err(|e| serde::de::Error::custom(format!("pubkey hex decode: {e}")))?
            .try_into()
            .map_err(|v: Vec<u8>| {
                serde::de::Error::custom(format!(
                    "expected 32-byte pubkey, got {} bytes",
                    v.len()
                ))
            })?;
        // Enforce invariant: both server_reported_tier and server_signature_on_tier
        // must be present together, or both absent.
        let server_signature_on_tier = match (h.server_reported_tier, h.server_signature_on_tier) {
            (None, None) => None,
            (Some(_), None) => {
                return Err(serde::de::Error::custom(
                    "server_reported_tier present but server_signature_on_tier absent — pair must be provided together"
                ));
            }
            (None, Some(_)) => {
                return Err(serde::de::Error::custom(
                    "server_signature_on_tier present but server_reported_tier absent — pair must be provided together"
                ));
            }
            (Some(_), Some(s)) => {
                let bytes = hex::decode(&s).map_err(|e| {
                    serde::de::Error::custom(format!("server_signature_on_tier hex decode: {e}"))
                })?;
                let arr: [u8; 64] = bytes.try_into().map_err(|v: Vec<u8>| {
                    serde::de::Error::custom(format!(
                        "expected 64-byte server_signature_on_tier, got {} bytes",
                        v.len()
                    ))
                })?;
                Some(arr)
            }
        };
        Ok(SignedManifest {
            spirit_id: h.spirit_id,
            version: h.version,
            manifest_toml: h.manifest_toml,
            signature,
            signer_pubkey,
            server_reported_tier: h.server_reported_tier,
            server_signature_on_tier,
        })
    }
}

impl serde::Serialize for SignedArtifact {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SignedArtifact", 5)?;
        state.serialize_field("spirit_id", &self.spirit_id)?;
        state.serialize_field("version", &self.version)?;
        state.serialize_field("artifact_bytes", &self.artifact_bytes)?;
        state.serialize_field("signature", &hex::encode(&self.signature[..]))?;
        state.serialize_field("signer_pubkey", &hex::encode(&self.signer_pubkey[..]))?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for SignedArtifact {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        struct Helper {
            spirit_id: SpiritId,
            version: String,
            artifact_bytes: Vec<u8>,
            signature: String,
            signer_pubkey: String,
        }
        let h = Helper::deserialize(deserializer)?;
        let signature = hex::decode(&h.signature)
            .map_err(|e| serde::de::Error::custom(format!("signature hex decode: {e}")))?
            .try_into()
            .map_err(|v: Vec<u8>| {
                serde::de::Error::custom(format!(
                    "expected 64-byte signature, got {} bytes",
                    v.len()
                ))
            })?;
        let signer_pubkey = hex::decode(&h.signer_pubkey)
            .map_err(|e| serde::de::Error::custom(format!("pubkey hex decode: {e}")))?
            .try_into()
            .map_err(|v: Vec<u8>| {
                serde::de::Error::custom(format!(
                    "expected 32-byte pubkey, got {} bytes",
                    v.len()
                ))
            })?;
        Ok(SignedArtifact {
            spirit_id: h.spirit_id,
            version: h.version,
            artifact_bytes: h.artifact_bytes,
            signature,
            signer_pubkey,
        })
    }
}

impl serde::Serialize for SignedPackage {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SignedPackage", 7)?;
        state.serialize_field("spirit_id", &self.spirit_id)?;
        state.serialize_field("version", &self.version)?;
        state.serialize_field("manifest_toml", &self.manifest_toml)?;
        state.serialize_field("artifact_bytes", &self.artifact_bytes)?;
        state.serialize_field("signature", &hex::encode(&self.signature[..]))?;
        state.serialize_field("publisher_pubkey", &hex::encode(&self.publisher_pubkey[..]))?;
        state.serialize_field("compliance_envelope", &self.compliance_envelope)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for SignedPackage {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        struct Helper {
            spirit_id: SpiritId,
            version: String,
            manifest_toml: Vec<u8>,
            artifact_bytes: Vec<u8>,
            signature: String,
            publisher_pubkey: String,
            compliance_envelope: ComplianceClaimEnvelope,
        }
        let h = Helper::deserialize(deserializer)?;
        let signature = hex::decode(&h.signature)
            .map_err(|e| serde::de::Error::custom(format!("signature hex decode: {e}")))?
            .try_into()
            .map_err(|v: Vec<u8>| {
                serde::de::Error::custom(format!(
                    "expected 64-byte signature, got {} bytes",
                    v.len()
                ))
            })?;
        let publisher_pubkey = hex::decode(&h.publisher_pubkey)
            .map_err(|e| serde::de::Error::custom(format!("pubkey hex decode: {e}")))?
            .try_into()
            .map_err(|v: Vec<u8>| {
                serde::de::Error::custom(format!(
                    "expected 32-byte pubkey, got {} bytes",
                    v.len()
                ))
            })?;
        Ok(SignedPackage {
            spirit_id: h.spirit_id,
            version: h.version,
            manifest_toml: h.manifest_toml,
            artifact_bytes: h.artifact_bytes,
            signature,
            publisher_pubkey,
            compliance_envelope: h.compliance_envelope,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use maos_spirit_abi::compliance::{ComplianceClaimEnvelope, SigningAlg};

    fn empty_envelope() -> ComplianceClaimEnvelope {
        ComplianceClaimEnvelope {
            signature: [0u8; 64],
            attester_pubkey: [1u8; 32],
            claim_bytes: vec![0xA1, 0x01],
            signing_alg: SigningAlg::Ed25519,
        }
    }

    #[test]
    fn search_query_defaults() {
        let q = SearchQuery::new("hello".into(), false, 50);
        assert_eq!(q.text, "hello");
        assert!(!q.include_yanked);
        assert_eq!(q.limit, 50);
    }

    #[test]
    fn search_query_limit_clamped_to_200() {
        let q = SearchQuery::new("test".into(), false, 500);
        assert_eq!(q.limit, 200);
    }

    #[test]
    fn search_query_limit_minimum_1() {
        let q = SearchQuery::new("test".into(), false, 0);
        assert_eq!(q.limit, 1);
    }

    #[test]
    fn search_query_serde_roundtrip() {
        let q = SearchQuery::new("hello-spirit".into(), true, 25);
        let json = serde_json::to_string(&q).unwrap();
        let q2: SearchQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(q, q2);
    }

    #[test]
    fn search_query_deser_defaults() {
        let json = r#"{"text":"hello"}"#;
        let q: SearchQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.text, "hello");
        assert!(!q.include_yanked);
        assert_eq!(q.limit, 50);
    }

    #[test]
    fn signed_package_serde_roundtrip() {
        let pkg = SignedPackage::new(
            SpiritId::from("hello-spirit"),
            "0.1.0".into(),
            b"[manifest]".to_vec(),
            b"binary".to_vec(),
            [0xABu8; 64],
            [0xCDu8; 32],
            empty_envelope(),
        );
        let json = serde_json::to_string(&pkg).unwrap();
        let pkg2: SignedPackage = serde_json::from_str(&json).unwrap();
        assert_eq!(pkg.spirit_id, pkg2.spirit_id);
        assert_eq!(pkg.version, pkg2.version);
        assert_eq!(pkg.manifest_toml, pkg2.manifest_toml);
        assert_eq!(pkg.artifact_bytes, pkg2.artifact_bytes);
        assert_eq!(pkg.signature, pkg2.signature);
        assert_eq!(pkg.publisher_pubkey, pkg2.publisher_pubkey);
        assert_eq!(
            pkg.compliance_envelope.attester_pubkey,
            pkg2.compliance_envelope.attester_pubkey
        );
    }

    #[test]
    fn signed_manifest_serde_roundtrip() {
        let m = SignedManifest::new(
            SpiritId::from("test"),
            "1.0.0".into(),
            b"manifest".to_vec(),
            [0xAAu8; 64],
            [0xBBu8; 32],
        );
        let json = serde_json::to_string(&m).unwrap();
        let m2: SignedManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(m.spirit_id, m2.spirit_id);
        assert_eq!(m.version, m2.version);
        assert_eq!(m.manifest_toml, m2.manifest_toml);
        assert_eq!(m.signature, m2.signature);
        assert_eq!(m.signer_pubkey, m2.signer_pubkey);
    }

    #[test]
    fn signed_artifact_serde_roundtrip() {
        let a = SignedArtifact::new(
            SpiritId::from("test"),
            "1.0.0".into(),
            b"binary".to_vec(),
            [0xAAu8; 64],
            [0xBBu8; 32],
        );
        let json = serde_json::to_string(&a).unwrap();
        let a2: SignedArtifact = serde_json::from_str(&json).unwrap();
        assert_eq!(a.spirit_id, a2.spirit_id);
        assert_eq!(a.version, a2.version);
        assert_eq!(a.artifact_bytes, a2.artifact_bytes);
        assert_eq!(a.signature, a2.signature);
        assert_eq!(a.signer_pubkey, a2.signer_pubkey);
    }

    #[test]
    fn yank_entry_serde_roundtrip() {
        let e = YankEntry::new(
            SpiritId::from("hello-spirit"),
            "0.1.0".into(),
            42,
            "buggy".into(),
        );
        let json = serde_json::to_string(&e).unwrap();
        let e2: YankEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(e, e2);
    }

    #[test]
    fn yank_list_serde_roundtrip() {
        let list = YankList::new(vec![
            YankEntry::new(SpiritId::from("s1"), "1.0.0".into(), 10, "reason".into()),
        ]);
        let json = serde_json::to_string(&list).unwrap();
        let list2: YankList = serde_json::from_str(&json).unwrap();
        assert_eq!(list.entries.len(), list2.entries.len());
    }

    #[test]
    fn publish_receipt_serde_roundtrip() {
        let r = PublishReceipt::new("pub-1".into(), SpiritId::from("test"), "0.1.0".into());
        let json = serde_json::to_string(&r).unwrap();
        let r2: PublishReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(r, r2);
    }

    #[test]
    fn yank_receipt_serde_roundtrip() {
        let r = YankReceipt::new("yank-1".into(), SpiritId::from("test"), "0.1.0".into());
        let json = serde_json::to_string(&r).unwrap();
        let r2: YankReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(r, r2);
    }

    #[test]
    fn yank_reason_serde_roundtrip() {
        let r = YankReason::new("buggy release".into());
        let json = serde_json::to_string(&r).unwrap();
        let r2: YankReason = serde_json::from_str(&json).unwrap();
        assert_eq!(r, r2);
    }

    #[test]
    fn registry_error_display() {
        let e = RegistryError::UnknownSpirit("test-spirit".into());
        let msg = e.to_string();
        assert!(msg.contains("test-spirit"));

        let e2 = RegistryError::VersionNotFound {
            spirit_id: "s".into(),
            requested: "2.0".into(),
        };
        assert!(e2.to_string().contains("2.0"));
        assert!(e2.to_string().contains("s"));

        let e3 = RegistryError::ComplianceContextDrift {
            actual_hex: "abcdef".into(),
            claimed_hex: "123456".into(),
        };
        assert!(e3.to_string().contains("abcdef"));
        assert!(e3.to_string().contains("123456"));
    }

    #[test]
    #[allow(unreachable_patterns)]
    fn registry_error_is_non_exhaustive() {
        let e = RegistryError::Unconfigured;
        match e {
            RegistryError::Unconfigured => {}
            _ => {}
        }
    }
}
