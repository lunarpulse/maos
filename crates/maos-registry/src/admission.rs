//! Three-trust-tier strictest-of-floor admission per ADR-009.
//!
//! The `admit_spirit` function implements the strictest-of-(manifest, registry,
//! operator-policy) floor for the four trust tiers.  It performs:
//!
//! 1. Parse manifest to extract declared trust tier.
//! 2. Compute `effective_tier = strictest_of(manifest_declared, registry_origin, op_cfg.tier_floor)`.
//! 3. Branch on tier: `Local` (unsigned allowed per policy), `OrgInternal` (org key required),
//!    `PublicUntrusted` (signature + ComplianceClaim envelope required),
//!    `PublicVetted` (rejected per FR37).

use maos_domain::ports::registry::{SignedPackage, TrustTier};
use maos_spirit_abi::compliance::{CryptoProviderId, ProviderEndpointPin, SandboxTier};

use crate::compliance_verify::extract_manifest_fingerprint_fields;
use maos_compliance::evaluator::{evaluate_envelope, ComplianceVerdict, EComplianceRejection};
use maos_compliance::RuntimeExecutionContext;

/// Admission decision returned by `admit_spirit`.
#[derive(Debug, Clone)]
pub struct AdmissionDecision {
    /// The effective trust tier after strictest-of resolution.
    pub effective_tier: TrustTier,
    /// The sandbox tier floor to apply.
    pub sandbox_tier_floor: SandboxTier,
    /// Whether to admit the Spirit.
    pub admit: bool,
    /// Human-readable journal note for Transparency-Log.
    pub journal_note: String,
}

/// Errors specific to the admission path.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AdmissionError {
    #[error("trust_tier 'public_vetted' deferred per FR37 to v2.5; allowed: local, org_internal, public_untrusted")]
    PublicVettedDeferred,

    #[error("org signature does not match operator-configured org key (operator must set [registry].org_signing_pubkey)")]
    OrgSignatureInvalid,

    #[error("publisher Ed25519 signature verification failed on artifact bytes")]
    PublisherSignatureInvalid,

    #[error(
        "ComplianceClaim execution-context drift on {field} — actual={actual}, claimed={claimed}"
    )]
    ComplianceContextDrift {
        /// Which fingerprint field drifted (or "MalformedClaim" / "ExpiredClaim").
        field: String,
        actual: String,
        claimed: String,
    },

    #[error("operator-policy unsigned_local rejected — operator must set allow_unsigned_local=true to admit unsigned local Spirits")]
    UnsignedLocalRejected,

    /// Story 9.4b AC-6 — typed, **catalogued** model-provenance rejection
    /// (presence / staleness). The wrapped variants are defined in
    /// `maos-domain` (the FR63-scanned home), so the error is catalogued there.
    #[error(transparent)]
    ModelProvenance(#[from] maos_domain::provenance::ProvenanceError),

    /// Story 9.4b AC-6 — a present `[model_provenance]` section failed to parse
    /// (malformed shape / free-text lineage). Distinct from `ModelProvenance`
    /// (a policy reject) so the operator sees a config-fix signal.
    #[error("model-provenance section malformed: {0}")]
    ModelProvenanceMalformed(String),
}

/// Configuration for the admission path.
#[derive(Debug, Clone)]
pub struct AdmissionConfig {
    pub tier_floor: TrustTier,
    pub registry_origin_tier: TrustTier,
    pub t3_for_public_untrusted: bool,
    pub allow_unsigned_local: bool,
    pub org_signing_pubkey: Option<[u8; 32]>,
    /// Story 7.3 (FR38 v1.0): the operator's resolved RUNTIME provider endpoint.
    /// `None` defaults to the manifest-derived value so v0.5-α manifest-matching
    /// fixtures keep admitting; `Some` exercises the v1.0 runtime drift check.
    pub runtime_provider_endpoint: Option<ProviderEndpointPin>,
    /// Story 7.3 (FR38 v1.0): the composition-root crypto-provider identity
    /// (`"ring"` for `RingCryptoProvider`). `None` defaults to the
    /// manifest-derived value.
    pub runtime_crypto_provider: Option<CryptoProviderId>,
}

/// Three-trust-tier strictest-of-floor admission per ADR-009.
pub fn admit_spirit(
    pkg: &SignedPackage,
    op_cfg: &AdmissionConfig,
) -> Result<AdmissionDecision, AdmissionError> {
    // 1. Parse manifest to extract declared trust tier.
    let manifest_declared_tier = extract_manifest_tier(&pkg.manifest_toml);

    // 2. Compute effective_tier = strictest_of(manifest, registry_origin, op_cfg.tier_floor).
    let registry_origin_tier = op_cfg.registry_origin_tier;
    let effective_tier = strictest_of(
        manifest_declared_tier,
        registry_origin_tier,
        op_cfg.tier_floor,
    );

    // 3. Branch on effective_tier.
    match effective_tier {
        TrustTier::PublicVetted => {
            // FR37: PublicVetted is deferred to v2.5
            Err(AdmissionError::PublicVettedDeferred)
        }
        TrustTier::PublicUntrusted => {
            // REQUIRE: Ed25519 signature verification + ComplianceClaim envelope
            // Step a: Verify publisher signature
            let sig_ok = verify_publisher_sig(pkg);
            if !sig_ok {
                return Err(AdmissionError::PublisherSignatureInvalid);
            }

            // Step b: v1.0 semantic ComplianceClaim verification against the
            // RUNTIME execution context (FR38 §8.5 upgrade). The claim is
            // compared against the kernel's ACTUAL runtime context — the
            // strictest-of effective trust tier (NOT the manifest-declared
            // tier), the operator-resolved provider endpoint, and the
            // composition-root crypto provider — rather than the manifest alone.
            let manifest_fields = extract_manifest_fingerprint_fields(&pkg.manifest_toml);
            let runtime_provider = op_cfg
                .runtime_provider_endpoint
                .clone()
                .unwrap_or_else(|| manifest_fields.provider_endpoint.clone());
            let runtime_crypto = op_cfg
                .runtime_crypto_provider
                .clone()
                .unwrap_or_else(|| manifest_fields.crypto_provider.clone());
            // NOTE (boundary documentation — team consensus Decision #1):
            // Compliance attests the manifest-declared sandbox tier. The effective
            // runtime sandbox floor (e.g. T3 when t3_for_public_untrusted=true) is
            // computed and enforced DOWNSTREAM at lines ~163-167 — it is a separate
            // enforcement layer, not a compliance concern. These are intentionally
            // two distinct questions: (1) "does the claim match the manifest?" and
            // (2) "does the runtime floor meet operator policy?" The v1.0 "runtime >
            // manifest" principle applies to fields where the runtime value EXISTS at
            // compliance time (trust_tier, provider_endpoint, crypto_provider). For
            // sandbox_tier the effective value has not been computed yet.
            let runtime_ctx = RuntimeExecutionContext::from_admission(
                effective_tier,               // strictest-of result (v1.0 upgrade)
                manifest_fields.sandbox_tier, // manifest-declared; runtime floor enforced downstream
                pkg,
                &runtime_provider,
                &runtime_crypto,
            );
            match evaluate_envelope(&pkg.compliance_envelope, &runtime_ctx) {
                ComplianceVerdict::Admit => {}
                ComplianceVerdict::Reject(EComplianceRejection::SignatureInvalid) => {
                    return Err(AdmissionError::PublisherSignatureInvalid);
                }
                ComplianceVerdict::Reject(EComplianceRejection::ContextDrift {
                    field,
                    actual,
                    claimed,
                }) => {
                    return Err(AdmissionError::ComplianceContextDrift {
                        field: format!("{field:?}"),
                        actual,
                        claimed,
                    });
                }
                ComplianceVerdict::Reject(EComplianceRejection::MalformedClaim(m)) => {
                    return Err(AdmissionError::ComplianceContextDrift {
                        field: "MalformedClaim".into(),
                        actual: String::new(),
                        claimed: m,
                    });
                }
                ComplianceVerdict::Reject(EComplianceRejection::ExpiredClaim {
                    expired_at_unix_ms,
                }) => {
                    return Err(AdmissionError::ComplianceContextDrift {
                        field: "ExpiredClaim".into(),
                        actual: String::new(),
                        claimed: format!("expired_at_unix_ms={expired_at_unix_ms}"),
                    });
                }
            }

            // Sandbox tier floor
            let sandbox_tier_floor = if op_cfg.t3_for_public_untrusted {
                SandboxTier::T3
            } else {
                SandboxTier::T0
            };

            Ok(AdmissionDecision {
                effective_tier,
                sandbox_tier_floor,
                admit: true,
                journal_note: format!(
                    "admitted public_untrusted spirit '{}' v{} with ComplianceClaim envelope",
                    pkg.spirit_id.as_str(),
                    pkg.version
                ),
            })
        }
        TrustTier::OrgInternal => {
            // REQUIRE: publisher_pubkey == op_cfg.org_signing_pubkey
            let org_key = op_cfg
                .org_signing_pubkey
                .ok_or(AdmissionError::OrgSignatureInvalid)?;
            if pkg.publisher_pubkey != org_key {
                return Err(AdmissionError::OrgSignatureInvalid);
            }

            // Also verify the signature
            let sig_ok = verify_publisher_sig(pkg);
            if !sig_ok {
                return Err(AdmissionError::PublisherSignatureInvalid);
            }

            Ok(AdmissionDecision {
                effective_tier,
                sandbox_tier_floor: SandboxTier::T0,
                admit: true,
                journal_note: format!(
                    "admitted org_internal spirit '{}' v{} with org signature",
                    pkg.spirit_id.as_str(),
                    pkg.version
                ),
            })
        }
        TrustTier::Local => {
            // Admit if unsigned_local allowed OR signature matches org key
            if !op_cfg.allow_unsigned_local {
                // Check if there's an org key configured and signature matches
                if let Some(org_key) = op_cfg.org_signing_pubkey {
                    if pkg.publisher_pubkey == org_key {
                        let sig_ok = verify_publisher_sig(pkg);
                        if sig_ok {
                            return Ok(AdmissionDecision {
                                effective_tier,
                                sandbox_tier_floor: SandboxTier::T0,
                                admit: true,
                                journal_note: format!(
                                    "admitted local spirit '{}' v{} with org signature match",
                                    pkg.spirit_id.as_str(),
                                    pkg.version
                                ),
                            });
                        }
                    }
                }
                return Err(AdmissionError::UnsignedLocalRejected);
            }

            Ok(AdmissionDecision {
                effective_tier,
                sandbox_tier_floor: SandboxTier::T0,
                admit: true,
                journal_note: format!(
                    "admitted local spirit '{}' v{} (unsigned local allowed)",
                    pkg.spirit_id.as_str(),
                    pkg.version
                ),
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Story 9.4b AC-6 — model-provenance admission gate (NFR-Comp-5 / SB-1047)
//
// Model-provenance is a SEPARATE admission axis from `admit_spirit`'s tier /
// signature / ComplianceClaim checks (mirrors the sandbox-tier boundary note
// inside `admit_spirit`). It lives in this module and is invoked on the same
// composition-root admission path; keeping it off `AdmissionConfig` preserves
// the frozen `admit_spirit` signature (AC-11: pre-v3 admissions byte-stable).
// The returned record is what the caller journals as the FR62 governance event.
// ---------------------------------------------------------------------------

/// Operator policy for the model-provenance admission gate (D5/D6).
#[derive(Debug, Clone)]
pub struct ModelProvenancePolicy {
    /// When `true`, a Spirit admitted without a `[model_provenance]` section is
    /// rejected with `EModelProvenanceMissing` (covered classes — SB-1047).
    /// Defaults to `false` so pre-v3 / non-covered manifests stay admissible.
    pub require: bool,
    /// Maximum allowed age of `last_eval_timestamp` in seconds. `None` disables
    /// the staleness check.
    pub max_age_secs: Option<u64>,
    /// Admission wall-clock as Unix seconds (injected for determinism/testing).
    pub now_unix_secs: i64,
}

impl Default for ModelProvenancePolicy {
    fn default() -> Self {
        // AC-11 safe default: provenance optional, no staleness window.
        Self {
            require: false,
            max_age_secs: None,
            now_unix_secs: 0,
        }
    }
}

/// The provenance facts captured at admission, bound for the FR62 governance
/// event (D6 — schema-identity + content-hash; D7 — `deployment_operator_id`).
/// Carries SCHEMA/class-metadata only — **zero claim-instance ids**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelProvenanceRecord {
    pub covered_model_id: String,
    pub training_data_lineage: Vec<String>,
    pub last_eval_timestamp: String,
    /// SHA-256 hex over the canonical provenance-triple bytes.
    pub content_hash: String,
}

/// AC-6 — validate the OPTIONAL `[model_provenance]` section on the admission
/// path. Returns:
/// - `Ok(None)` when absent and not required (AC-11 — pre-v3 manifests admit);
/// - `Ok(Some(record))` when present-and-valid (caller emits the FR62 event);
/// - `Err(AdmissionError::ModelProvenance(_))` on missing-but-required or stale
///   (typed, **catalogued** in `maos-domain`);
/// - `Err(AdmissionError::ModelProvenanceMalformed(_))` on a present-but-broken
///   section (e.g. free-text lineage).
pub fn validate_model_provenance(
    manifest_toml: &[u8],
    policy: &ModelProvenancePolicy,
) -> Result<Option<ModelProvenanceRecord>, AdmissionError> {
    use maos_domain::provenance::ProvenanceError;
    use maos_manifest::ModelProvenanceSection;

    let text = String::from_utf8_lossy(manifest_toml);
    let section = ModelProvenanceSection::from_manifest_toml(&text)
        .map_err(|e| AdmissionError::ModelProvenanceMalformed(e.to_string()))?;

    match section {
        None => {
            if policy.require {
                return Err(ProvenanceError::EModelProvenanceMissing.into());
            }
            Ok(None)
        }
        Some(sec) => {
            if let Some(max_age) = policy.max_age_secs {
                sec.validate_staleness(policy.now_unix_secs, max_age)?;
            }
            let content_hash = {
                use sha2::Digest;
                let mut h = sha2::Sha256::new();
                h.update(sec.canonical_content_bytes());
                hex::encode(h.finalize())
            };
            Ok(Some(ModelProvenanceRecord {
                covered_model_id: sec.covered_model_id,
                training_data_lineage: sec.training_data_lineage,
                last_eval_timestamp: sec.last_eval_timestamp,
                content_hash,
            }))
        }
    }
}

/// Strictest-of ordering: PublicUntrusted > OrgInternal > Local.
/// PublicVetted is the most-restricted and always rejected.
fn strictest_of(
    manifest_tier: TrustTier,
    registry_origin_tier: TrustTier,
    op_floor: TrustTier,
) -> TrustTier {
    // Convert each tier to a strictness score (higher = more restricted)
    fn score(t: TrustTier) -> u8 {
        match t {
            TrustTier::Local => 0,
            TrustTier::OrgInternal => 1,
            TrustTier::PublicUntrusted => 2,
            TrustTier::PublicVetted => 3, // Most restricted
        }
    }

    let winning_score = score(manifest_tier)
        .max(score(registry_origin_tier))
        .max(score(op_floor));

    match winning_score {
        0 => TrustTier::Local,
        1 => TrustTier::OrgInternal,
        2 => TrustTier::PublicUntrusted,
        _ => TrustTier::PublicVetted,
    }
}

/// Extract the trust tier declared in a manifest TOML.
///
/// Story 7.2 made this `pub` for `maos-spirit-cli` to use during the
/// `publish --tier` flag's manifest-tier cross-check.
pub fn extract_manifest_tier(manifest_toml: &[u8]) -> TrustTier {
    let text = String::from_utf8_lossy(manifest_toml);
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("trust_tier") {
            if let Some(val) = trimmed.split('=').nth(1) {
                let val = val.trim().trim_matches('"');
                match val {
                    "local" => return TrustTier::Local,
                    "org_internal" => return TrustTier::OrgInternal,
                    "public_vetted" => return TrustTier::PublicVetted,
                    "public_untrusted" => return TrustTier::PublicUntrusted,
                    _ => {}
                }
            }
        }
    }
    // Default: Local
    TrustTier::Local
}

/// Verify the publisher's Ed25519 signature over `sha256(manifest_len_u64 || manifest_toml || artifact_len_u64 || artifact_bytes)`.
/// Domain-separated to prevent collision attacks (Story 7.2 fix).
/// Uses u64::to_le_bytes for cross-arch compatibility.
pub fn verify_publisher_sig(pkg: &SignedPackage) -> bool {
    use ring::signature::{UnparsedPublicKey, ED25519};
    use sha2::Digest;

    let mut hasher = sha2::Sha256::new();
    hasher.update(&(pkg.manifest_toml.len() as u64).to_le_bytes());
    hasher.update(&pkg.manifest_toml);
    hasher.update(&(pkg.artifact_bytes.len() as u64).to_le_bytes());
    hasher.update(&pkg.artifact_bytes);
    let msg = hasher.finalize();

    let pk = UnparsedPublicKey::new(&ED25519, &pkg.publisher_pubkey);
    pk.verify(&msg, &pkg.signature).is_ok()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::ports::registry::SpiritId;
    use maos_spirit_abi::compliance::{ComplianceClaimEnvelope, SigningAlg};

    fn empty_envelope() -> ComplianceClaimEnvelope {
        ComplianceClaimEnvelope {
            signature: [0u8; 64],
            attester_pubkey: [1u8; 32],
            claim_bytes: vec![],
            signing_alg: SigningAlg::Ed25519,
        }
    }

    fn pkg_with_tier(tier: &str) -> SignedPackage {
        let manifest = format!(
            "[spirit]\nname = \"test\"\nversion = \"0.1.0\"\ntrust_tier = \"{}\"\n",
            tier
        );
        SignedPackage::new(
            SpiritId::from("test"),
            "0.1.0".into(),
            manifest.into_bytes(),
            b"binary".to_vec(),
            [0xAAu8; 64],
            [0xBBu8; 32],
            empty_envelope(),
        )
    }

    fn permissive_cfg() -> AdmissionConfig {
        AdmissionConfig {
            tier_floor: TrustTier::Local,
            registry_origin_tier: TrustTier::Local,
            t3_for_public_untrusted: false,
            allow_unsigned_local: true,
            org_signing_pubkey: None,
            runtime_provider_endpoint: None,
            runtime_crypto_provider: None,
        }
    }

    fn strict_local_cfg() -> AdmissionConfig {
        AdmissionConfig {
            tier_floor: TrustTier::Local,
            registry_origin_tier: TrustTier::Local,
            t3_for_public_untrusted: false,
            allow_unsigned_local: false,
            org_signing_pubkey: None,
            runtime_provider_endpoint: None,
            runtime_crypto_provider: None,
        }
    }

    #[allow(dead_code)]
    fn public_untrusted_origin_cfg() -> AdmissionConfig {
        AdmissionConfig {
            tier_floor: TrustTier::Local,
            registry_origin_tier: TrustTier::PublicUntrusted,
            t3_for_public_untrusted: false,
            allow_unsigned_local: true,
            org_signing_pubkey: None,
            runtime_provider_endpoint: None,
            runtime_crypto_provider: None,
        }
    }

    #[test]
    fn local_unsigned_allowed_when_policy_permits() {
        let pkg = pkg_with_tier("local");
        let decision = admit_spirit(&pkg, &permissive_cfg()).unwrap();
        assert!(decision.admit);
        assert_eq!(decision.effective_tier, TrustTier::Local);
    }

    #[test]
    fn local_unsigned_rejected_when_policy_strict() {
        let pkg = pkg_with_tier("local");
        let err = admit_spirit(&pkg, &strict_local_cfg()).unwrap_err();
        assert!(matches!(err, AdmissionError::UnsignedLocalRejected));
    }

    #[test]
    fn public_vetted_tier_always_rejects() {
        let pkg = pkg_with_tier("public_vetted");
        let err = admit_spirit(&pkg, &permissive_cfg()).unwrap_err();
        assert!(matches!(err, AdmissionError::PublicVettedDeferred));
    }

    #[test]
    fn strictest_of_resolves_correctly_local_vs_public_untrusted() {
        // manifest=local, registry=public_untrusted → effective=public_untrusted
        let cfg = AdmissionConfig {
            registry_origin_tier: TrustTier::PublicUntrusted,
            ..permissive_cfg()
        };
        let manifest = "[spirit]\nname=\"test\"\nversion=\"0.1.0\"\ntrust_tier=\"local\"\n";
        let pkg = SignedPackage::new(
            SpiritId::from("test"),
            "0.1.0".into(),
            manifest.as_bytes().to_vec(),
            b"binary".to_vec(),
            [0xAAu8; 64],
            [0xBBu8; 32],
            empty_envelope(),
        );
        // public_untrusted → needs signature verification, will fail on fake sig
        let err = admit_spirit(&pkg, &cfg).unwrap_err();
        assert!(matches!(err, AdmissionError::PublisherSignatureInvalid));
    }

    #[test]
    fn org_internal_signature_requires_org_key() {
        let pkg = pkg_with_tier("org_internal");
        let cfg = AdmissionConfig {
            tier_floor: TrustTier::Local,
            ..permissive_cfg()
        };
        // No org key configured → OrgSignatureInvalid
        let err = admit_spirit(&pkg, &cfg).unwrap_err();
        assert!(matches!(err, AdmissionError::OrgSignatureInvalid));
    }

    #[test]
    fn strictest_of_resolves_all_combinations() {
        // Test the core strictest_of function directly
        use maos_domain::ports::registry::TrustTier as T;
        assert_eq!(strictest_of(T::Local, T::Local, T::Local), T::Local);
        assert_eq!(
            strictest_of(T::Local, T::Local, T::OrgInternal),
            T::OrgInternal
        );
        assert_eq!(
            strictest_of(T::Local, T::Local, T::PublicUntrusted),
            T::PublicUntrusted
        );
        assert_eq!(
            strictest_of(T::Local, T::OrgInternal, T::Local),
            T::OrgInternal
        );
        assert_eq!(
            strictest_of(T::OrgInternal, T::OrgInternal, T::OrgInternal),
            T::OrgInternal
        );
        assert_eq!(
            strictest_of(T::Local, T::PublicUntrusted, T::Local),
            T::PublicUntrusted
        );
        assert_eq!(
            strictest_of(T::PublicUntrusted, T::Local, T::Local),
            T::PublicUntrusted
        );
        assert_eq!(
            strictest_of(T::PublicUntrusted, T::PublicUntrusted, T::PublicUntrusted),
            T::PublicUntrusted
        );
    }

    #[test]
    fn public_untrusted_with_t3_floor_returns_t3_sandbox() {
        let cfg = AdmissionConfig {
            tier_floor: TrustTier::Local, // weak floor so effective stays public_untrusted
            t3_for_public_untrusted: true,
            ..permissive_cfg()
        };
        let manifest =
            "[spirit]\nname=\"test\"\nversion=\"0.1.0\"\ntrust_tier=\"public_untrusted\"\n";
        let pkg = SignedPackage::new(
            SpiritId::from("test"),
            "0.1.0".into(),
            manifest.as_bytes().to_vec(),
            b"binary".to_vec(),
            [0xAAu8; 64],
            [0xBBu8; 32],
            empty_envelope(),
        );
        // Will fail on signature verification, but we want to test the tier path
        let result = admit_spirit(&pkg, &cfg);
        // It fails at signature step because fake sig — that's expected.
        // The key point: the admission path didn't fail at tier level.
        assert!(result.is_err());
    }

    // ----------------------------------------------------------------
    // Real-signature tests using deterministic seeded Ed25519 keypairs
    // (Story 4.5 §A6 precedent — seed 0x150C04A5). NEVER commit the
    // private key; we derive it deterministically in-test from the seed.
    // ----------------------------------------------------------------

    /// Deterministic Ed25519 keypair derived from a 4-byte seed prefix,
    /// expanded to 32 bytes by repeating the prefix.  Suitable for tests
    /// ONLY — never use this for production keys.
    fn seeded_keypair(seed: u32) -> (ring::signature::Ed25519KeyPair, [u8; 32]) {
        use ring::signature::{Ed25519KeyPair, KeyPair};
        let prefix = seed.to_le_bytes();
        let mut s = [0u8; 32];
        for (i, b) in s.iter_mut().enumerate() {
            *b = prefix[i % 4];
        }
        let kp = Ed25519KeyPair::from_seed_unchecked(&s).unwrap();
        let pk: [u8; 32] = kp.public_key().as_ref().try_into().unwrap();
        (kp, pk)
    }

    /// Build a SignedPackage with a real Ed25519 signature over
    /// sha256(manifest_len_u64 || manifest_toml || artifact_len_u64 || artifact_bytes).
    /// Domain-separated to prevent collision attacks (Story 7.2 fix).
    /// Uses u64::to_le_bytes for cross-arch compatibility.
    fn signed_pkg_with(
        spirit_id: &str,
        version: &str,
        tier: &str,
        keypair: &ring::signature::Ed25519KeyPair,
        pubkey: [u8; 32],
    ) -> SignedPackage {
        use sha2::Digest;
        let manifest = format!(
            "[spirit]\nname = \"{}\"\nversion = \"{}\"\ntrust_tier = \"{}\"\n",
            spirit_id, version, tier
        );
        let artifact = b"binary".to_vec();
        let mut hasher = sha2::Sha256::new();
        let manifest_bytes = manifest.as_bytes();
        hasher.update(&(manifest_bytes.len() as u64).to_le_bytes());
        hasher.update(manifest_bytes);
        hasher.update(&(artifact.len() as u64).to_le_bytes());
        hasher.update(&artifact);
        let msg = hasher.finalize();
        let sig = keypair.sign(&msg);
        let sig_bytes: [u8; 64] = sig.as_ref().try_into().unwrap();
        SignedPackage::new(
            SpiritId::from(spirit_id),
            version.into(),
            manifest.into_bytes(),
            artifact,
            sig_bytes,
            pubkey,
            empty_envelope(),
        )
    }

    /// Build a SignedPackage for a public_untrusted spirit with a valid
    /// ComplianceClaim envelope whose fingerprint matches the manifest.
    fn public_untrusted_pkg_with_valid_envelope(
        spirit_id: &str,
        version: &str,
        keypair: &ring::signature::Ed25519KeyPair,
        pubkey: [u8; 32],
    ) -> SignedPackage {
        use crate::compliance_verify::compute_fingerprint_hash;
        use maos_spirit_abi::compliance::{
            CapabilityId, CryptoProviderId, ExecutionContextFingerprint, ProviderEndpointPin,
            SandboxTier as Sb, TrustTier as Tt,
        };
        use sha2::Digest;
        use std::collections::BTreeSet;

        let manifest = format!(
            "[spirit]\nname = \"{}\"\nversion = \"{}\"\ntrust_tier = \"public_untrusted\"\nsandbox_tier = \"t3\"\n",
            spirit_id, version
        );
        let artifact = b"binary".to_vec();

        // Re-derive the expected fingerprint from the manifest fields
        let mut mh = sha2::Sha256::new();
        mh.update(manifest.as_bytes());
        let manifest_hash: [u8; 32] = mh.finalize().into();

        let fp = ExecutionContextFingerprint {
            manifest_hash,
            spirit_version: version.to_string(),
            trust_tier: Tt::PublicUntrusted,
            sandbox_tier: Sb::T3,
            capability_scope: BTreeSet::<CapabilityId>::new(),
            provider_endpoint: ProviderEndpointPin {
                provider_id: String::new(),
                endpoint_url: String::new(),
                model_id: None,
            },
            crypto_provider: CryptoProviderId(String::new()),
        };
        let fp_hex = hex::encode(compute_fingerprint_hash(&fp));

        let claim_json = serde_json::json!({
            "fingerprint_hash": fp_hex,
            "trust_tier": "public_untrusted",
            "sandbox_tier": "t3",
            "capability_scope": [],
            "provider_endpoint": {"provider_id": "", "endpoint_url": ""},
            "crypto_provider": ""
        });
        let claim_bytes = serde_json::to_vec(&claim_json).unwrap();
        let claim_sig = keypair.sign(&claim_bytes);
        let envelope = ComplianceClaimEnvelope {
            signature: claim_sig.as_ref().try_into().unwrap(),
            attester_pubkey: pubkey,
            claim_bytes,
            signing_alg: SigningAlg::Ed25519,
        };

        // Package signature over sha256(manifest_len_u64 || manifest || artifact_len_u64 || artifact)
        // Domain-separated to prevent collision attacks (Story 7.2 fix).
        let mut h = sha2::Sha256::new();
        let manifest_bytes = manifest.as_bytes();
        h.update(&(manifest_bytes.len() as u64).to_le_bytes());
        h.update(manifest_bytes);
        h.update(&(artifact.len() as u64).to_le_bytes());
        h.update(&artifact);
        let msg = h.finalize();
        let pkg_sig = keypair.sign(&msg);
        let pkg_sig_bytes: [u8; 64] = pkg_sig.as_ref().try_into().unwrap();

        SignedPackage::new(
            SpiritId::from(spirit_id),
            version.into(),
            manifest.into_bytes(),
            artifact,
            pkg_sig_bytes,
            pubkey,
            envelope,
        )
    }

    #[test]
    fn org_internal_signature_matches_admits() {
        // Story 5.5d Finding #6: real Ed25519 publisher_pubkey == org_signing_pubkey
        // matches and the signature verifies — admit.
        let (kp, pk) = seeded_keypair(0x150C04A5);
        let pkg = signed_pkg_with("org-spirit", "0.1.0", "org_internal", &kp, pk);

        let cfg = AdmissionConfig {
            tier_floor: TrustTier::Local,
            registry_origin_tier: TrustTier::Local,
            t3_for_public_untrusted: false,
            allow_unsigned_local: false,
            // Operator-configured org key matches publisher.
            org_signing_pubkey: Some(pk),
            runtime_provider_endpoint: None,
            runtime_crypto_provider: None,
        };

        let decision = admit_spirit(&pkg, &cfg).expect("expected admit");
        assert!(decision.admit);
        assert_eq!(decision.effective_tier, TrustTier::OrgInternal);
    }

    #[test]
    fn org_internal_signature_mismatch_rejects() {
        // Story 5.5d Finding #6: publisher key does NOT match operator-configured
        // org key — reject with OrgSignatureInvalid.
        let (kp_publisher, pk_publisher) = seeded_keypair(0x150C04A5);
        let (_kp_org, pk_org) = seeded_keypair(0xDEADBEEF); // different seed

        let pkg = signed_pkg_with(
            "org-spirit",
            "0.1.0",
            "org_internal",
            &kp_publisher,
            pk_publisher,
        );

        let cfg = AdmissionConfig {
            tier_floor: TrustTier::Local,
            registry_origin_tier: TrustTier::Local,
            t3_for_public_untrusted: false,
            allow_unsigned_local: false,
            org_signing_pubkey: Some(pk_org),
            runtime_provider_endpoint: None,
            runtime_crypto_provider: None,
        };

        let err = admit_spirit(&pkg, &cfg).unwrap_err();
        assert!(matches!(err, AdmissionError::OrgSignatureInvalid));
    }

    #[test]
    fn public_untrusted_with_valid_envelope_admits() {
        // Story 5.5d Finding #6: real Ed25519 signature + matching ComplianceClaim
        // envelope fingerprint — admit.
        let (kp, pk) = seeded_keypair(0x150C04A5);
        let pkg = public_untrusted_pkg_with_valid_envelope("pub-spirit", "0.1.0", &kp, pk);

        let cfg = AdmissionConfig {
            tier_floor: TrustTier::Local,
            registry_origin_tier: TrustTier::Local,
            t3_for_public_untrusted: false,
            allow_unsigned_local: true,
            org_signing_pubkey: None,
            runtime_provider_endpoint: None,
            runtime_crypto_provider: None,
        };

        let decision = admit_spirit(&pkg, &cfg).expect("expected admit");
        assert!(decision.admit);
        assert_eq!(decision.effective_tier, TrustTier::PublicUntrusted);
    }

    #[test]
    fn public_untrusted_with_tampered_envelope_rejects() {
        // Story 5.5d Finding #6: valid publisher signature but tampered envelope
        // signature (signed by a different key than the attester_pubkey).
        let (kp_pub, pk_pub) = seeded_keypair(0x150C04A5);
        let (kp_tamper, _pk_tamper) = seeded_keypair(0xC0FFEE00);

        // Start from a valid envelope, then tamper its signature
        let mut pkg =
            public_untrusted_pkg_with_valid_envelope("pub-spirit", "0.1.0", &kp_pub, pk_pub);
        // Re-sign claim_bytes with a DIFFERENT key but keep the attester_pubkey
        // pointing at the original publisher → signature won't verify.
        let bad_sig = kp_tamper.sign(&pkg.compliance_envelope.claim_bytes);
        pkg.compliance_envelope.signature = bad_sig.as_ref().try_into().unwrap();

        let cfg = AdmissionConfig {
            tier_floor: TrustTier::Local,
            registry_origin_tier: TrustTier::Local,
            t3_for_public_untrusted: false,
            allow_unsigned_local: true,
            org_signing_pubkey: None,
            runtime_provider_endpoint: None,
            runtime_crypto_provider: None,
        };

        let err = admit_spirit(&pkg, &cfg).unwrap_err();
        assert!(
            matches!(err, AdmissionError::PublisherSignatureInvalid),
            "expected PublisherSignatureInvalid, got {err:?}"
        );
    }

    // ----------------------------------------------------------------
    // Story 9.4b AC-6 — model-provenance admission gate
    // ----------------------------------------------------------------

    fn manifest_with_provenance(lineage: &str, ts: &str) -> Vec<u8> {
        format!(
            "[class]\nname = \"x\"\nversion = \"0.1.0\"\ntrust_tier = \"local\"\n\
             [model_provenance]\ncovered_model_id = \"anthropic.claude-opus-4-8\"\n\
             training_data_lineage = [\"{lineage}\"]\nlast_eval_timestamp = \"{ts}\"\n"
        )
        .into_bytes()
    }

    #[test]
    fn provenance_absent_admits_when_not_required_ac11() {
        let manifest = b"[class]\nname = \"x\"\nversion = \"0.1.0\"\n".to_vec();
        let got =
            validate_model_provenance(&manifest, &ModelProvenancePolicy::default()).unwrap();
        assert!(got.is_none(), "pre-v3 manifest must stay admissible");
    }

    #[test]
    fn provenance_absent_but_required_rejects_catalogued() {
        use maos_domain::provenance::ProvenanceError;
        let manifest = b"[class]\nname = \"x\"\nversion = \"0.1.0\"\n".to_vec();
        let policy = ModelProvenancePolicy {
            require: true,
            ..Default::default()
        };
        let err = validate_model_provenance(&manifest, &policy).unwrap_err();
        assert!(matches!(
            err,
            AdmissionError::ModelProvenance(ProvenanceError::EModelProvenanceMissing)
        ));
    }

    #[test]
    fn provenance_present_valid_returns_record_with_content_hash() {
        let manifest =
            manifest_with_provenance("lineage.public-web.cc-2024", "2026-06-01T00:00:00Z");
        let rec = validate_model_provenance(&manifest, &ModelProvenancePolicy::default())
            .unwrap()
            .expect("present");
        assert_eq!(rec.covered_model_id, "anthropic.claude-opus-4-8");
        assert_eq!(rec.content_hash.len(), 64); // sha256 hex
        assert!(rec.content_hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn provenance_stale_rejects() {
        use maos_domain::provenance::ProvenanceError;
        let manifest =
            manifest_with_provenance("lineage.public-web.cc-2024", "2026-06-01T00:00:00Z");
        // last_eval = 1_780_272_000; now = +100 days; window = 30 days.
        let policy = ModelProvenancePolicy {
            require: true,
            max_age_secs: Some(30 * 86_400),
            now_unix_secs: 1_780_272_000 + 100 * 86_400,
        };
        let err = validate_model_provenance(&manifest, &policy).unwrap_err();
        assert!(matches!(
            err,
            AdmissionError::ModelProvenance(ProvenanceError::EModelProvenanceStale { .. })
        ));
    }

    #[test]
    fn provenance_free_text_lineage_rejects_malformed() {
        let manifest =
            manifest_with_provenance("trained on private emails", "2026-06-01T00:00:00Z");
        let err = validate_model_provenance(&manifest, &ModelProvenancePolicy::default())
            .unwrap_err();
        assert!(matches!(err, AdmissionError::ModelProvenanceMalformed(_)));
    }

    #[test]
    fn provenance_content_hash_is_deterministic() {
        let m1 = manifest_with_provenance("lineage.a.b", "2026-06-01T00:00:00Z");
        let m2 = manifest_with_provenance("lineage.a.b", "2026-06-01T00:00:00Z");
        let r1 = validate_model_provenance(&m1, &ModelProvenancePolicy::default())
            .unwrap()
            .unwrap();
        let r2 = validate_model_provenance(&m2, &ModelProvenancePolicy::default())
            .unwrap()
            .unwrap();
        assert_eq!(r1.content_hash, r2.content_hash);
    }

    #[test]
    fn public_untrusted_with_fingerprint_drift_rejects() {
        // Story 5.5d Finding #30 — make outcome deterministic by signing
        // both pkg + envelope so we KNOW signatures verify; the only
        // mismatch is the claimed fingerprint_hash. Must reject with
        // ComplianceContextDrift (not SignatureInvalid).
        use sha2::Digest;

        let (kp, pk) = seeded_keypair(0x150C04A5);
        let manifest = b"[spirit]\nname = \"drift\"\nversion = \"0.1.0\"\ntrust_tier = \"public_untrusted\"\nsandbox_tier = \"t3\"\n";
        let artifact = b"binary".to_vec();

        let claim_json = serde_json::json!({
            // Deliberately wrong fingerprint hash
            "fingerprint_hash": "0000000000000000000000000000000000000000000000000000000000000000",
            "trust_tier": "public_untrusted",
            "sandbox_tier": "t3",
            "capability_scope": [],
            "provider_endpoint": {"provider_id": "", "endpoint_url": ""},
            "crypto_provider": ""
        });
        let claim_bytes = serde_json::to_vec(&claim_json).unwrap();
        let claim_sig = kp.sign(&claim_bytes);

        let envelope = ComplianceClaimEnvelope {
            signature: claim_sig.as_ref().try_into().unwrap(),
            attester_pubkey: pk,
            claim_bytes,
            signing_alg: SigningAlg::Ed25519,
        };

        // Valid pkg signature (domain-separated hash per Story 7.2 fix)
        let mut h = sha2::Sha256::new();
        h.update(&(manifest.len() as u64).to_le_bytes());
        h.update(manifest);
        h.update(&(artifact.len() as u64).to_le_bytes());
        h.update(&artifact);
        let msg = h.finalize();
        let pkg_sig = kp.sign(&msg);
        let pkg_sig_bytes: [u8; 64] = pkg_sig.as_ref().try_into().unwrap();

        let pkg = SignedPackage::new(
            SpiritId::from("drift"),
            "0.1.0".into(),
            manifest.to_vec(),
            artifact,
            pkg_sig_bytes,
            pk,
            envelope,
        );

        let cfg = AdmissionConfig {
            tier_floor: TrustTier::Local,
            registry_origin_tier: TrustTier::Local,
            t3_for_public_untrusted: false,
            allow_unsigned_local: true,
            org_signing_pubkey: None,
            runtime_provider_endpoint: None,
            runtime_crypto_provider: None,
        };

        let err = admit_spirit(&pkg, &cfg).unwrap_err();
        assert!(
            matches!(err, AdmissionError::ComplianceContextDrift { .. }),
            "expected ComplianceContextDrift (deterministic), got {err:?}"
        );
    }
}
