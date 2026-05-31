//! The v0.9-binding ComplianceClaim semantic evaluator.
//!
//! [`evaluate_envelope`] runs the four-step pipeline (signature → strict
//! decode → runtime fingerprint → conjunctive field comparison) and returns a
//! [`ComplianceVerdict`].
//!
//! # Drift-field naming order (binding decision)
//!
//! The story pseudocode lists `fingerprint_hash` first, but naming the hash
//! first would label *every* structural drift `FingerprintHash` (any structural
//! change also perturbs the hash), which violates AC5's requirement that the
//! 100 context-drift envelopes reject with `expected_rejection_field` matching
//! the *drifted field*. We therefore compare the five claim-carried structural
//! fields FIRST (naming the specific field) and fall back to `FingerprintHash`
//! only when all structural fields agree but the hash still differs (i.e., the
//! drift is in `manifest_hash` / `spirit_version`, which the claim does not
//! carry as discrete fields).

use std::collections::BTreeSet;

use maos_spirit_abi::compliance::{
    CapabilityId, ComplianceClaimEnvelope, CryptoProviderId, ProviderEndpointPin, SandboxTier,
    SigningAlg, TrustTier,
};

use crate::canonical_cbor;
use crate::runtime_context::RuntimeExecutionContext;

/// The evaluator's verdict for a single envelope.
#[derive(Debug, Clone, PartialEq)]
pub enum ComplianceVerdict {
    /// Envelope is valid under the runtime execution context.
    Admit,
    /// Envelope is rejected with a typed reason.
    Reject(EComplianceRejection),
}

/// The typed rejection taxonomy (the §8.5 `EComplianceContextDrift` maps to
/// [`EComplianceRejection::ContextDrift`]).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EComplianceRejection {
    /// Ed25519 signature verification failed (or signing_alg unsupported).
    #[error("ComplianceClaim Ed25519 signature verification failed")]
    SignatureInvalid,
    /// `claim_bytes` could not be decoded, was missing a required field, or
    /// carried an unknown enum value. NEVER a silent default.
    #[error("ComplianceClaim is malformed: {0}")]
    MalformedClaim(String),
    /// The runtime execution-context drifted from the attested claim on
    /// `field`.
    #[error("execution-context drift on field {field:?}: actual={actual} claimed={claimed}")]
    ContextDrift {
        field: DriftField,
        actual: String,
        claimed: String,
    },
    /// The claim carried an `expires_at_unix_ms` in the past.
    #[error("ComplianceClaim expired at {expired_at_unix_ms}")]
    ExpiredClaim { expired_at_unix_ms: u64 },
}

/// Which of the seven fingerprint fields (or the hash) diverged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftField {
    ManifestHash,
    SpiritVersion,
    TrustTier,
    SandboxTier,
    CapabilityScope,
    ProviderEndpoint,
    CryptoProvider,
    FingerprintHash,
}

/// Evaluate an envelope against a runtime execution context (wall-clock `now`
/// for the expiry check).
pub fn evaluate_envelope(
    envelope: &ComplianceClaimEnvelope,
    ctx: &RuntimeExecutionContext,
) -> ComplianceVerdict {
    let now_unix_ms = unix_now_ms();
    evaluate_envelope_at(envelope, ctx, now_unix_ms)
}

/// Deterministic core — `now_unix_ms` is injected so the CCAC ship gate and
/// latency bench are reproducible.
pub fn evaluate_envelope_at(
    envelope: &ComplianceClaimEnvelope,
    ctx: &RuntimeExecutionContext,
    now_unix_ms: u64,
) -> ComplianceVerdict {
    use ComplianceVerdict::Reject;
    use EComplianceRejection::*;

    // Step 1: Ed25519 signature verification over claim_bytes (matches the
    // producer, which signs claim_bytes directly).
    if envelope.signing_alg != SigningAlg::Ed25519 {
        return Reject(SignatureInvalid);
    }
    if !verify_ed25519(
        &envelope.attester_pubkey,
        &envelope.claim_bytes,
        &envelope.signature,
    ) {
        return Reject(SignatureInvalid);
    }

    // Step 2: strict canonical-CBOR (then JSON) decode — unknown enum value or
    // missing field is MalformedClaim, NEVER a silent default.
    let claimed = match parse_claim_strict(&envelope.claim_bytes) {
        Ok(c) => c,
        Err(e) => return Reject(MalformedClaim(e)),
    };

    // Step 2b: expiry.
    if let Some(exp) = claimed.expires_at_unix_ms {
        if now_unix_ms >= exp {
            return Reject(ExpiredClaim {
                expired_at_unix_ms: exp,
            });
        }
    }

    // Step 3: recompute the RUNTIME fingerprint + its canonical-CBOR hash.
    let actual_fp = ctx.to_fingerprint();
    let actual_hash = canonical_cbor::fingerprint_hash(&actual_fp);

    // Step 4: conjunctive comparison. Name the FIRST divergent structural
    // field; fall back to FingerprintHash for manifest_hash/spirit_version
    // drift (which the claim does not carry as discrete fields).
    if claimed.trust_tier != actual_fp.trust_tier {
        return drift(
            DriftField::TrustTier,
            tier_str(actual_fp.trust_tier),
            tier_str(claimed.trust_tier),
        );
    }
    if claimed.sandbox_tier != actual_fp.sandbox_tier {
        return drift(
            DriftField::SandboxTier,
            sandbox_str(actual_fp.sandbox_tier),
            sandbox_str(claimed.sandbox_tier),
        );
    }
    if claimed.capability_scope != actual_fp.capability_scope {
        return drift(
            DriftField::CapabilityScope,
            caps_str(&actual_fp.capability_scope),
            caps_str(&claimed.capability_scope),
        );
    }
    if claimed.provider_endpoint != actual_fp.provider_endpoint {
        return drift(
            DriftField::ProviderEndpoint,
            pe_str(&actual_fp.provider_endpoint),
            pe_str(&claimed.provider_endpoint),
        );
    }
    if claimed.crypto_provider != actual_fp.crypto_provider {
        return drift(
            DriftField::CryptoProvider,
            actual_fp.crypto_provider.0.clone(),
            claimed.crypto_provider.0.clone(),
        );
    }
    // Structural fields agree — if the hash still differs the drift is in
    // manifest_hash / spirit_version (encoded in the hash only).
    if claimed.fingerprint_hash != actual_hash {
        return drift(
            DriftField::FingerprintHash,
            hex::encode(actual_hash),
            hex::encode(claimed.fingerprint_hash),
        );
    }

    ComplianceVerdict::Admit
}

fn drift(field: DriftField, actual: String, claimed: String) -> ComplianceVerdict {
    ComplianceVerdict::Reject(EComplianceRejection::ContextDrift {
        field,
        actual,
        claimed,
    })
}

/// Minimal parsed claim — the producer's self-attested fingerprint map.
#[derive(Debug, Clone)]
pub struct ParsedClaim {
    pub fingerprint_hash: [u8; 32],
    pub trust_tier: TrustTier,
    pub sandbox_tier: SandboxTier,
    pub capability_scope: BTreeSet<CapabilityId>,
    pub provider_endpoint: ProviderEndpointPin,
    pub crypto_provider: CryptoProviderId,
    pub expires_at_unix_ms: Option<u64>,
}

/// Strict claim parse — CBOR first, JSON fallback (fixture convenience).
///
/// Unlike Story 5.5d's `parse_claim`, an unknown `trust_tier` / `sandbox_tier`
/// value is `Err(MalformedClaim)` — NOT a silent `PublicUntrusted` / `T0`
/// default. A truncated `fingerprint_hash` (≠32 bytes) and any missing
/// required field are likewise errors. This precision is what the 400-malformed
/// CCAC corpus validates.
pub fn parse_claim_strict(bytes: &[u8]) -> Result<ParsedClaim, String> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ClaimHelper {
        #[serde(default)]
        fingerprint_hash: Option<String>,
        #[serde(default)]
        trust_tier: Option<String>,
        #[serde(default)]
        sandbox_tier: Option<String>,
        #[serde(default)]
        capability_scope: Option<Vec<String>>,
        #[serde(default)]
        provider_endpoint: Option<ProviderEndpointHelper>,
        #[serde(default)]
        crypto_provider: Option<String>,
        #[serde(default)]
        expires_at_unix_ms: Option<u64>,
    }

    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ProviderEndpointHelper {
        #[serde(default)]
        provider_id: Option<String>,
        #[serde(default)]
        endpoint_url: Option<String>,
        #[serde(default)]
        model_id: Option<String>,
    }

    let helper = if let Ok(h) = serde_cbor::from_slice::<ClaimHelper>(bytes) {
        h
    } else {
        serde_json::from_slice::<ClaimHelper>(bytes)
            .map_err(|e| format!("claim parse error (tried CBOR then JSON): {e}"))?
    };

    let fingerprint_hash = helper
        .fingerprint_hash
        .ok_or_else(|| "claim missing fingerprint_hash field".to_string())
        .and_then(|s| {
            hex::decode(&s).map_err(|e| format!("fingerprint_hash hex decode error: {e}"))
        })
        .and_then(|v| {
            <[u8; 32]>::try_from(v).map_err(|v: Vec<u8>| {
                format!("fingerprint_hash expected 32 bytes, got {}", v.len())
            })
        })?;

    // STRICT: unknown enum value → MalformedClaim, never a default.
    let trust_tier = match helper.trust_tier.as_deref() {
        Some("local") => TrustTier::Local,
        Some("org_internal") => TrustTier::OrgInternal,
        Some("public_vetted") => TrustTier::PublicVetted,
        Some("public_untrusted") => TrustTier::PublicUntrusted,
        Some(other) => return Err(format!("unknown trust_tier '{other}'")),
        None => return Err("claim missing trust_tier field".to_string()),
    };

    let sandbox_tier = match helper.sandbox_tier.as_deref() {
        Some("t0") => SandboxTier::T0,
        Some("t1") => SandboxTier::T1,
        Some("t2") => SandboxTier::T2,
        Some("t3") => SandboxTier::T3,
        Some("t4") => SandboxTier::T4,
        Some(other) => return Err(format!("unknown sandbox_tier '{other}'")),
        None => return Err("claim missing sandbox_tier field".to_string()),
    };

    let capability_scope: BTreeSet<CapabilityId> = helper
        .capability_scope
        .ok_or_else(|| "claim missing capability_scope field".to_string())?
        .into_iter()
        .map(CapabilityId)
        .collect();

    let provider_endpoint = match helper.provider_endpoint {
        Some(pe) => {
            let pid = pe
                .provider_id
                .ok_or_else(|| "provider_endpoint missing provider_id".to_string())?;
            let eurl = pe
                .endpoint_url
                .ok_or_else(|| "provider_endpoint missing endpoint_url".to_string())?;
            ProviderEndpointPin {
                provider_id: pid,
                endpoint_url: eurl,
                model_id: pe.model_id,
            }
        }
        None => return Err("claim missing provider_endpoint field".to_string()),
    };

    let crypto_provider = helper
        .crypto_provider
        .map(CryptoProviderId)
        .ok_or_else(|| "claim missing crypto_provider field".to_string())?;

    Ok(ParsedClaim {
        fingerprint_hash,
        trust_tier,
        sandbox_tier,
        capability_scope,
        provider_endpoint,
        crypto_provider,
        expires_at_unix_ms: helper.expires_at_unix_ms,
    })
}

/// Verify an Ed25519 signature using `ring`.
pub fn verify_ed25519(public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    use ring::signature::{UnparsedPublicKey, ED25519};
    let pk = UnparsedPublicKey::new(&ED25519, public_key);
    pk.verify(message, signature).is_ok()
}

fn unix_now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_millis() as u64,
        Err(_) => u64::MAX,
    }
}

fn tier_str(t: TrustTier) -> String {
    match t {
        TrustTier::Local => "local",
        TrustTier::OrgInternal => "org_internal",
        TrustTier::PublicVetted => "public_vetted",
        TrustTier::PublicUntrusted => "public_untrusted",
    }
    .to_string()
}

fn sandbox_str(t: SandboxTier) -> String {
    match t {
        SandboxTier::T0 => "t0",
        SandboxTier::T1 => "t1",
        SandboxTier::T2 => "t2",
        SandboxTier::T3 => "t3",
        SandboxTier::T4 => "t4",
    }
    .to_string()
}

fn caps_str(caps: &BTreeSet<CapabilityId>) -> String {
    caps.iter()
        .map(|c| c.0.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

fn pe_str(pe: &ProviderEndpointPin) -> String {
    format!(
        "{}|{}|{}",
        pe.provider_id,
        pe.endpoint_url,
        pe.model_id.as_deref().unwrap_or("")
    )
}
