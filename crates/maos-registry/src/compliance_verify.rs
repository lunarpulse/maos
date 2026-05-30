//! ComplianceClaim structural verification per §8.5.
//!
//! v0.5-α scope: Ed25519 signature verification + canonical CBOR decode +
//! fingerprint hash match.  The FULL semantic evaluator (principle engine +
//! N=600 corpus) arrives at Story 7.3 + App-E v0.9.

use maos_domain::ports::registry::SignedPackage;
use maos_spirit_abi::compliance::{
    CapabilityId, ComplianceClaimEnvelope, CryptoProviderId, ExecutionContextFingerprint,
    ProviderEndpointPin, SandboxTier, SigningAlg, TrustTier,
};

/// Result of structural envelope verification.
#[derive(Debug)]
pub enum VerificationResult {
    /// Envelope is structurally valid.
    Ok,
    /// Envelope signature does not verify.
    SignatureInvalid,
    /// Envelope claim_bytes cannot be decoded as canonical CBOR.
    ClaimDecodeFailure(String),
    /// Fingerprint hash mismatch between claimed and actual context.
    Drift {
        actual_hex: String,
        claimed_hex: String,
    },
}

/// Manifest-derived fingerprint fields used during both verification and
/// `maos-spirit publish --compliance-claim` auto-population (Story 7.2 AC2).
pub struct ManifestFingerprintFields {
    pub trust_tier: TrustTier,
    pub sandbox_tier: SandboxTier,
    pub capability_scope: std::collections::BTreeSet<CapabilityId>,
    pub provider_endpoint: ProviderEndpointPin,
    pub crypto_provider: CryptoProviderId,
}

/// Story 7.2 — made `pub` for `maos-spirit-cli` auto-population.
pub fn extract_manifest_fingerprint_fields(manifest_toml: &[u8]) -> ManifestFingerprintFields {
    let text = String::from_utf8_lossy(manifest_toml);
    let mut trust_tier = TrustTier::Local;
    let mut sandbox_tier = SandboxTier::T0;
    let mut capability_scope = std::collections::BTreeSet::new();
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

/// Verify a ComplianceClaim envelope structurally.
///
/// Steps:
/// 1. Verify the Ed25519 signature over `claim_bytes`.
/// 2. CBOR-decode `claim_bytes` into a minimal `ClaimParsed` struct.
/// 3. Re-compute the actual execution-context fingerprint from the package manifest.
/// 4. Compare `claimed.fingerprint_hash` against `cbor_hash(actual_fingerprint)`.
pub fn verify_envelope_structural(
    envelope: &ComplianceClaimEnvelope,
    pkg: &SignedPackage,
) -> VerificationResult {
    // Step 1: Ed25519 signature verification
    if envelope.signing_alg != SigningAlg::Ed25519 {
        return VerificationResult::SignatureInvalid;
    }

    let sig_ok = verify_ed25519(
        &envelope.attester_pubkey,
        &envelope.claim_bytes,
        &envelope.signature,
    );

    if !sig_ok {
        return VerificationResult::SignatureInvalid;
    }

    // Step 2: CBOR-decode claim_bytes (with JSON fallback)
    let claimed = match parse_claim(&envelope.claim_bytes) {
        Ok(c) => c,
        Err(e) => return VerificationResult::ClaimDecodeFailure(e),
    };

    // Step 3: Re-compute actual fingerprint from manifest TOML
    let manifest_fields = extract_manifest_fingerprint_fields(&pkg.manifest_toml);

    let actual = ExecutionContextFingerprint {
        manifest_hash: sha256_hash(&pkg.manifest_toml),
        spirit_version: pkg.version.clone(),
        trust_tier: manifest_fields.trust_tier,
        sandbox_tier: manifest_fields.sandbox_tier,
        capability_scope: manifest_fields.capability_scope,
        provider_endpoint: manifest_fields.provider_endpoint,
        crypto_provider: manifest_fields.crypto_provider,
    };

    let actual_hash = compute_fingerprint_hash(&actual);

    // Step 4: Compare hashes
    if actual_hash != claimed.fingerprint_hash {
        return VerificationResult::Drift {
            actual_hex: hex::encode(actual_hash),
            claimed_hex: hex::encode(claimed.fingerprint_hash),
        };
    }

    // Step 4b (Story 5.5d Finding #1 defense-in-depth): cross-check the
    // CLAIMED structural fields against the MANIFEST-derived fields. The
    // fingerprint_hash comparison above already catches drift via the CBOR
    // hash, but the structural check makes the drift cause explicit and
    // catches the case where a publisher tries to misrepresent the claim
    // shape (claimed_trust_tier="local" while manifest declares
    // public_untrusted, for instance).
    if claimed.trust_tier != actual.trust_tier
        || claimed.sandbox_tier != actual.sandbox_tier
        || claimed.capability_scope != actual.capability_scope
        || claimed.provider_endpoint != actual.provider_endpoint
        || claimed.crypto_provider != actual.crypto_provider
    {
        return VerificationResult::Drift {
            actual_hex: hex::encode(actual_hash),
            claimed_hex: format!(
                "structural-mismatch:claim_tier={:?}/manifest_tier={:?}",
                claimed.trust_tier, actual.trust_tier
            ),
        };
    }

    VerificationResult::Ok
}

/// Minimal parsed claim — only the fields needed for fingerprint verification.
#[derive(Debug, Clone)]
struct ParsedClaim {
    fingerprint_hash: [u8; 32],
    trust_tier: TrustTier,
    sandbox_tier: SandboxTier,
    capability_scope: std::collections::BTreeSet<CapabilityId>,
    provider_endpoint: ProviderEndpointPin,
    crypto_provider: CryptoProviderId,
}

/// Parse a `ParsedClaim` from canonical CBOR bytes with JSON fallback.
fn parse_claim(bytes: &[u8]) -> Result<ParsedClaim, String> {
    #[derive(serde::Deserialize)]
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
    }

    #[derive(serde::Deserialize)]
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
            <[u8; 32]>::try_from(v)
                .map_err(|v: Vec<u8>| format!("fingerprint_hash expected 32 bytes, got {}", v.len()))
        })?;

    let trust_tier = match helper.trust_tier.as_deref() {
        Some("local") => TrustTier::Local,
        Some("org_internal") => TrustTier::OrgInternal,
        Some("public_vetted") => TrustTier::PublicVetted,
        Some("public_untrusted") => TrustTier::PublicUntrusted,
        _ => TrustTier::PublicUntrusted,
    };

    let sandbox_tier = match helper.sandbox_tier.as_deref() {
        Some("t0") => SandboxTier::T0,
        Some("t1") => SandboxTier::T1,
        Some("t2") => SandboxTier::T2,
        Some("t3") => SandboxTier::T3,
        Some("t4") => SandboxTier::T4,
        _ => SandboxTier::T0,
    };

    let capability_scope = helper
        .capability_scope
        .ok_or_else(|| "claim missing capability_scope field".to_string())
        .into_iter()
        .flatten()
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
    })
}

/// Compute SHA-256 hash of data.
fn sha256_hash(data: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

/// Compute the fingerprint hash: `sha256(cbor_canonical(ser(&fp)))`.
pub fn compute_fingerprint_hash(fp: &ExecutionContextFingerprint) -> [u8; 32] {
    match serde_cbor::to_vec(fp) {
        Ok(cbor_bytes) => sha256_hash(&cbor_bytes),
        Err(e) => {
            eprintln!("CBOR serialization error in fingerprint_hash: {e}");
            sha256_hash(&[])
        }
    }
}

/// Verify an Ed25519 signature using `ring`.
fn verify_ed25519(public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    use ring::signature::{UnparsedPublicKey, ED25519};
    let pk = UnparsedPublicKey::new(&ED25519, public_key);
    pk.verify(message, signature).is_ok()
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
            claim_bytes: vec![],
            signing_alg: SigningAlg::Ed25519,
        }
    }

    fn test_pkg() -> SignedPackage {
        SignedPackage::new(
            maos_domain::ports::registry::SpiritId::from("test"),
            "0.1.0".into(),
            b"[manifest]".to_vec(),
            b"binary".to_vec(),
            [0xAAu8; 64],
            [0xBBu8; 32],
            empty_envelope(),
        )
    }

    fn generate_ed25519_keypair() -> (ring::signature::Ed25519KeyPair, [u8; 32]) {
        use ring::signature::KeyPair;
        let rng = ring::rand::SystemRandom::new();
        let pkcs8 = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let keypair = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
        let pubkey: [u8; 32] = keypair.public_key().as_ref().try_into().unwrap();
        (keypair, pubkey)
    }

    #[test]
    fn tampered_signature_is_rejected() {
        let pkg = test_pkg();
        let mut envelope = empty_envelope();
        envelope.signature = [0u8; 64];
        envelope.claim_bytes = b"hello".to_vec();
        let result = verify_envelope_structural(&envelope, &pkg);
        assert!(matches!(result, VerificationResult::SignatureInvalid));
    }

    #[test]
    fn undecodable_claim_bytes_is_rejected() {
        let pkg = test_pkg();
        let envelope = ComplianceClaimEnvelope {
            signature: [0u8; 64],
            attester_pubkey: [1u8; 32],
            claim_bytes: vec![0xFF, 0xFE, 0xFD],
            signing_alg: SigningAlg::Ed25519,
        };
        let result = verify_envelope_structural(&envelope, &pkg);
        assert!(matches!(
            result,
            VerificationResult::SignatureInvalid | VerificationResult::ClaimDecodeFailure(_)
        ));
    }

    #[test]
    fn fingerprint_mismatch_is_rejected() {
        let (keypair, pubkey) = generate_ed25519_keypair();

        let manifest = format!(
            "[spirit]\nname = \"test\"\nversion = \"0.1.0\"\ntrust_tier = \"local\"\nsandbox_tier = \"t0\"\n"
        );
        let pkg = SignedPackage::new(
            maos_domain::ports::registry::SpiritId::from("test"),
            "0.1.0".into(),
            manifest.into_bytes(),
            b"binary".to_vec(),
            [0xAAu8; 64],
            [0xBBu8; 32],
            empty_envelope(),
        );

        let claim_bytes = serde_json::json!({
            "fingerprint_hash": "0000000000000000000000000000000000000000000000000000000000000000",
            "trust_tier": "public_untrusted",
            "sandbox_tier": "t3",
            "capability_scope": [],
            "provider_endpoint": {"provider_id": "test", "endpoint_url": "http://localhost"},
            "crypto_provider": "ring"
        });
        let claim_vec = serde_json::to_vec(&claim_bytes).unwrap();

        let signature = keypair.sign(&claim_vec);
        let sig_bytes: [u8; 64] = signature.as_ref().try_into().unwrap();

        let envelope = ComplianceClaimEnvelope {
            signature: sig_bytes,
            attester_pubkey: pubkey,
            claim_bytes: claim_vec,
            signing_alg: SigningAlg::Ed25519,
        };
        let result = verify_envelope_structural(&envelope, &pkg);
        match result {
            VerificationResult::Drift {
                actual_hex,
                claimed_hex,
            } => {
                assert!(!actual_hex.is_empty());
                assert!(!claimed_hex.is_empty());
            }
            other => panic!("expected Drift, got {other:?}"),
        }
    }
}
