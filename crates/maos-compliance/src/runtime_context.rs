//! [`RuntimeExecutionContext`] — the **runtime** execution-context the v1.0
//! evaluator compares a ComplianceClaim against (the FR38 §8.5 upgrade from
//! manifest-only to actual-runtime-context comparison).
//!
//! The v0.5-α structural floor recomputed the fingerprint from the MANIFEST
//! alone. The v1.0 binding sources the load-bearing fields from RUNTIME:
//!
//! * `effective_trust_tier` — the admission strictest-of result (NOT the
//!   manifest-declared tier), so a claim attesting `trust_tier=local` admitted
//!   under an operator policy forcing `public_untrusted` is rejected.
//! * `effective_sandbox_tier` — the admission sandbox-tier floor.
//! * `runtime_provider_endpoint` — the operator's resolved provider config.
//! * `runtime_crypto_provider` — the composition-root crypto identity
//!   (`"ring"` for `RingCryptoProvider`).
//!
//! `manifest_hash`, `spirit_version`, and `capability_scope` remain
//! manifest-derived; per §8.5 defense-in-depth the capability scope is trusted
//! only because the `manifest_hash` field is compared conjunctively (a drifted
//! manifest fails the hash check before the scope is relied upon).

use std::collections::BTreeSet;

use maos_domain::ports::registry::SignedPackage;
use maos_spirit_abi::compliance::{
    CapabilityId, CryptoProviderId, ExecutionContextFingerprint, ProviderEndpointPin, SandboxTier,
    TrustTier,
};

use crate::canonical_cbor::sha256;

/// The seven runtime-sourced fingerprint inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeExecutionContext {
    /// SHA-256 of the admitted `manifest.toml`.
    pub manifest_hash: [u8; 32],
    /// Semantic version of the admitted Spirit.
    pub spirit_version: String,
    /// Strictest-of effective trust tier from admission (NOT manifest-declared).
    pub effective_trust_tier: TrustTier,
    /// Admission sandbox-tier floor.
    pub effective_sandbox_tier: SandboxTier,
    /// Operator-resolved runtime provider endpoint.
    pub runtime_provider_endpoint: ProviderEndpointPin,
    /// Composition-root crypto provider identity.
    pub runtime_crypto_provider: CryptoProviderId,
    /// Manifest-derived capability scope (gated behind `manifest_hash` equality
    /// per §8.5 defense-in-depth).
    pub capability_scope: BTreeSet<CapabilityId>,
}

impl RuntimeExecutionContext {
    /// Project this runtime context onto the frozen ABI fingerprint shape so
    /// it can be hashed with the SAME canonical encoder the producer + corpus
    /// generator use.
    pub fn to_fingerprint(&self) -> ExecutionContextFingerprint {
        ExecutionContextFingerprint {
            manifest_hash: self.manifest_hash,
            spirit_version: self.spirit_version.clone(),
            trust_tier: self.effective_trust_tier,
            sandbox_tier: self.effective_sandbox_tier,
            capability_scope: self.capability_scope.clone(),
            provider_endpoint: self.runtime_provider_endpoint.clone(),
            crypto_provider: self.runtime_crypto_provider.clone(),
        }
    }

    /// Build a runtime context from the admission result + composition-root
    /// identities.
    ///
    /// `effective_trust_tier` and `effective_sandbox_tier` are the admission
    /// strictest-of / sandbox-floor results; `provider_cfg` and `crypto_id`
    /// come from the operator's resolved provider config and the composition
    /// root. `manifest_hash`, `spirit_version`, and `capability_scope` are
    /// derived from `pkg`.
    ///
    /// (Takes the resolved tiers rather than a `&AdmissionDecision` to avoid a
    /// dependency cycle — `maos-registry` depends on `maos-compliance`, not the
    /// other way around.)
    pub fn from_admission(
        effective_trust_tier: TrustTier,
        effective_sandbox_tier: SandboxTier,
        pkg: &SignedPackage,
        provider_cfg: &ProviderEndpointPin,
        crypto_id: &CryptoProviderId,
    ) -> RuntimeExecutionContext {
        let fields = extract_manifest_fingerprint_fields(&pkg.manifest_toml);
        RuntimeExecutionContext {
            manifest_hash: sha256(&pkg.manifest_toml),
            spirit_version: pkg.version.clone(),
            effective_trust_tier,
            effective_sandbox_tier,
            runtime_provider_endpoint: provider_cfg.clone(),
            runtime_crypto_provider: crypto_id.clone(),
            capability_scope: fields.capability_scope,
        }
    }
}

/// Manifest-derived fingerprint fields used during verification and during
/// `maos-spirit publish --compliance-claim` auto-population (Story 7.2 AC2).
///
/// LIFTED from `maos-registry::compliance_verify` per Story 7.3 (single home).
pub struct ManifestFingerprintFields {
    pub trust_tier: TrustTier,
    pub sandbox_tier: SandboxTier,
    pub capability_scope: BTreeSet<CapabilityId>,
    pub provider_endpoint: ProviderEndpointPin,
    pub crypto_provider: CryptoProviderId,
}

/// Extract manifest-declared fingerprint fields from a `manifest.toml`.
///
/// LIFTED verbatim from `maos-registry::compliance_verify` — `maos-spirit-cli`
/// and the admission path import it from here (re-exported by
/// `maos-registry::compliance_verify` for backward compatibility).
pub fn extract_manifest_fingerprint_fields(manifest_toml: &[u8]) -> ManifestFingerprintFields {
    let text = String::from_utf8_lossy(manifest_toml);
    let mut trust_tier = TrustTier::Local;
    let mut sandbox_tier = SandboxTier::T0;
    let mut capability_scope = BTreeSet::new();
    let mut provider_id = String::new();
    let mut endpoint_url = String::new();
    let mut model_id: Option<String> = None;
    let mut crypto_provider = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(val) = extract_toml_kv(trimmed, "trust_tier") {
            trust_tier = match val {
                "local" => TrustTier::Local,
                "org_internal" => TrustTier::OrgInternal,
                "public_vetted" => TrustTier::PublicVetted,
                "public_untrusted" => TrustTier::PublicUntrusted,
                _ => TrustTier::Local,
            };
        } else if let Some(val) = extract_toml_kv(trimmed, "sandbox_tier") {
            sandbox_tier = match val {
                "t0" => SandboxTier::T0,
                "t1" => SandboxTier::T1,
                "t2" => SandboxTier::T2,
                "t3" => SandboxTier::T3,
                "t4" => SandboxTier::T4,
                _ => SandboxTier::T0,
            };
        } else if let Some(val) = extract_toml_kv(trimmed, "capability_scope") {
            for cap in parse_toml_string_array(val) {
                capability_scope.insert(CapabilityId(cap));
            }
        } else if let Some(val) = extract_toml_kv(trimmed, "provider_id") {
            provider_id = val.to_string();
        } else if let Some(val) = extract_toml_kv(trimmed, "endpoint_url") {
            endpoint_url = val.to_string();
        } else if let Some(val) = extract_toml_kv(trimmed, "model_id") {
            model_id = Some(val.to_string());
        } else if let Some(val) = extract_toml_kv(trimmed, "crypto_provider") {
            crypto_provider = val.to_string();
        }
    }

    ManifestFingerprintFields {
        trust_tier,
        sandbox_tier,
        capability_scope,
        provider_endpoint: ProviderEndpointPin {
            provider_id,
            endpoint_url,
            model_id,
        },
        crypto_provider: CryptoProviderId(crypto_provider),
    }
}

fn extract_toml_kv<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    if !line.starts_with(key) {
        return None;
    }
    let rest = &line[key.len()..];
    let rest = rest.trim_start();
    if !rest.starts_with('=') {
        return None;
    }
    let val = rest.split('=').nth(1)?;
    let val = val.trim();
    Some(val.trim_matches('"'))
}

fn parse_toml_string_array(s: &str) -> Vec<String> {
    let s = s.trim();
    let s = s.strip_prefix('[').unwrap_or(s);
    let s = s.strip_suffix(']').unwrap_or(s);
    s.split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_all_fields() {
        let manifest = b"trust_tier = \"public_untrusted\"\nsandbox_tier = \"t3\"\ncapability_scope = [\"fs.read\", \"net.connect\"]\nprovider_id = \"anthropic\"\nendpoint_url = \"https://x\"\ncrypto_provider = \"ring\"\n";
        let f = extract_manifest_fingerprint_fields(manifest);
        assert_eq!(f.trust_tier, TrustTier::PublicUntrusted);
        assert_eq!(f.sandbox_tier, SandboxTier::T3);
        assert_eq!(f.capability_scope.len(), 2);
        assert_eq!(f.provider_endpoint.provider_id, "anthropic");
        assert_eq!(f.crypto_provider.0, "ring");
    }

    #[test]
    fn to_fingerprint_uses_runtime_tiers() {
        let pkg = SignedPackage::new(
            maos_domain::ports::registry::SpiritId::from("t"),
            "0.1.0".into(),
            b"trust_tier = \"local\"\n".to_vec(),
            b"bin".to_vec(),
            [0u8; 64],
            [0u8; 32],
            maos_spirit_abi::compliance::ComplianceClaimEnvelope {
                signature: [0u8; 64],
                attester_pubkey: [0u8; 32],
                claim_bytes: vec![],
                signing_alg: maos_spirit_abi::compliance::SigningAlg::Ed25519,
            },
        );
        let ctx = RuntimeExecutionContext::from_admission(
            TrustTier::PublicUntrusted, // runtime forces stricter than manifest's "local"
            SandboxTier::T3,
            &pkg,
            &ProviderEndpointPin {
                provider_id: "anthropic".into(),
                endpoint_url: "https://x".into(),
                model_id: None,
            },
            &CryptoProviderId("ring".into()),
        );
        let fp = ctx.to_fingerprint();
        assert_eq!(fp.trust_tier, TrustTier::PublicUntrusted);
        assert_eq!(fp.crypto_provider.0, "ring");
    }
}
