//! Deterministic expansion of CCAC seeds into N=600 real envelopes.
//!
//! Determinism: every envelope is produced from a seeded keypair derived from
//! the running item index (no RNG), and every fingerprint hash uses the shared
//! `maos_compliance::canonical_cbor`. Re-running `expand` on any host yields
//! byte-identical output.

#[allow(deprecated)]
use maos_compliance::builder::seeded_keypair;
use maos_compliance::builder::{
    build_self_attested_envelope, build_signed_envelope, encode_claim_bytes,
    encode_claim_bytes_with_hash,
};
use maos_compliance::canonical_cbor::fingerprint_hash;
use maos_spirit_abi::compliance::{
    CapabilityId, ComplianceClaimEnvelope, CryptoProviderId, ExecutionContextFingerprint,
    SandboxTier, TrustTier,
};

use super::{reference_context, CcacItem, CcacSeed, REFERENCES, VARIATIONS_PER_SEED};

/// Expand seeds into items. `n` is honored as the target count; each seed
/// emits [`VARIATIONS_PER_SEED`] variations (60 × 10 = 600).
pub fn expand_deterministic(seeds: &[CcacSeed], n: usize) -> Vec<CcacItem> {
    let mut items = Vec::with_capacity(n);
    let mut global_idx: usize = 0;
    for (ord, seed) in seeds.iter().enumerate() {
        for v in 0..VARIATIONS_PER_SEED {
            if items.len() >= n {
                return items;
            }
            let reference = REFERENCES[(ord + v) % REFERENCES.len()];
            items.push(build_item(seed, ord, v, global_idx, reference));
            global_idx += 1;
        }
    }
    items
}

#[allow(deprecated)]
fn build_item(
    seed: &CcacSeed,
    ord: usize,
    variation: usize,
    global_idx: usize,
    reference: &str,
) -> CcacItem {
    let (manifest, ctx) =
        reference_context(reference).expect("CCAC expansion uses only valid references");
    let ref_fp = ctx.to_fingerprint();
    let (kp, pk) = seeded_keypair(0xCCAC_0000u32.wrapping_add(global_idx as u32));

    let id = format!("ccac-{global_idx:03}");

    let (envelope, expected_verdict, kind, field) = if seed.kind == "well_formed" {
        let env = build_self_attested_envelope(&ref_fp, &kp, pk);
        (env, "admit", None, None)
    } else {
        build_malformed(seed, &ref_fp, &kp, pk, global_idx)
            .unwrap_or_else(|e| panic!("CCAC seed {}: {e}", seed.id))
    };

    let envelope_cbor_hex =
        hex::encode(serde_cbor::to_vec(&envelope).expect("envelope is CBOR-serializable"));

    let _ = (ord, variation); // (kept for clarity; reference already derived)

    CcacItem {
        id,
        class: seed.class.clone(),
        expected_verdict: expected_verdict.to_string(),
        expected_rejection_kind: kind.map(|s: &str| s.to_string()),
        expected_rejection_field: field.map(|s: String| s),
        reference_spirit: reference.to_string(),
        envelope_cbor_hex,
        manifest_toml: manifest,
        rationale: seed.rationale.clone(),
    }
}

/// Build a malformed envelope per the seed's `malform` op. Returns
/// `Ok((envelope, "reject", Some(kind), Some(field)?))` or an error for
/// unknown malform ops.
#[allow(deprecated)]
fn build_malformed(
    seed: &CcacSeed,
    ref_fp: &ExecutionContextFingerprint,
    kp: &ring::signature::Ed25519KeyPair,
    pk: [u8; 32],
    global_idx: usize,
) -> Result<
    (
        ComplianceClaimEnvelope,
        &'static str,
        Option<&'static str>,
        Option<String>,
    ),
    String,
> {
    let op = seed.malform.as_deref().unwrap_or("");
    match op {
        "truncated_signature" => {
            let mut env = build_self_attested_envelope(ref_fp, kp, pk);
            for b in env.signature.iter_mut().take(8) {
                *b ^= 0xFF;
            }
            Ok((env, "reject", Some("SignatureInvalid"), None))
        }
        "wrong_attester_pubkey" => {
            let (_other_kp, other_pk) =
                seeded_keypair(0xDEAD_0000u32.wrapping_add(global_idx as u32));
            let claim_bytes = encode_claim_bytes(ref_fp, None);
            let env = build_signed_envelope(claim_bytes, kp, other_pk);
            Ok((env, "reject", Some("SignatureInvalid"), None))
        }
        "garbage_cbor" => {
            let garbage: Vec<u8> = (0..24u8).map(|i| 0xFFu8 ^ i).collect();
            let env = build_signed_envelope(garbage, kp, pk);
            Ok((env, "reject", Some("MalformedClaim"), None))
        }
        "empty_claim_bytes" => {
            let env = build_signed_envelope(Vec::new(), kp, pk);
            Ok((env, "reject", Some("MalformedClaim"), None))
        }
        "missing_trust_tier" => {
            let json = serde_json::json!({
                "fingerprint_hash": hex::encode(fingerprint_hash(ref_fp)),
                "sandbox_tier": "t1",
                "capability_scope": [],
                "provider_endpoint": {"provider_id": "p", "endpoint_url": "u"},
                "crypto_provider": "ring"
            });
            Ok(sign_json(json, kp, pk))
        }
        "missing_crypto_provider" => {
            let json = serde_json::json!({
                "fingerprint_hash": hex::encode(fingerprint_hash(ref_fp)),
                "trust_tier": "local",
                "sandbox_tier": "t1",
                "capability_scope": [],
                "provider_endpoint": {"provider_id": "p", "endpoint_url": "u"}
            });
            Ok(sign_json(json, kp, pk))
        }
        "truncated_fingerprint_hash" => {
            let json = serde_json::json!({
                "fingerprint_hash": hex::encode([0u8; 16]),
                "trust_tier": "local",
                "sandbox_tier": "t1",
                "capability_scope": [],
                "provider_endpoint": {"provider_id": "p", "endpoint_url": "u"},
                "crypto_provider": "ring"
            });
            Ok(sign_json(json, kp, pk))
        }
        "unknown_trust_tier" => {
            let json = serde_json::json!({
                "fingerprint_hash": hex::encode([0u8; 32]),
                "trust_tier": "super_trusted",
                "sandbox_tier": "t1",
                "capability_scope": [],
                "provider_endpoint": {"provider_id": "p", "endpoint_url": "u"},
                "crypto_provider": "ring"
            });
            Ok(sign_json(json, kp, pk))
        }
        "unknown_sandbox_tier" => {
            let json = serde_json::json!({
                "fingerprint_hash": hex::encode([0u8; 32]),
                "trust_tier": "local",
                "sandbox_tier": "t9",
                "capability_scope": [],
                "provider_endpoint": {"provider_id": "p", "endpoint_url": "u"},
                "crypto_provider": "ring"
            });
            Ok(sign_json(json, kp, pk))
        }
        "expired_claim" => {
            let claim_bytes = encode_claim_bytes(ref_fp, Some(1_000));
            let env = build_signed_envelope(claim_bytes, kp, pk);
            Ok((env, "reject", Some("ExpiredClaim"), None))
        }
        "context_drift" => {
            let field = seed
                .drift_field
                .clone()
                .unwrap_or_else(|| "TrustTier".into());
            let mutated = mutate_field(ref_fp, &field)
                .ok_or_else(|| format!("unknown CCAC drift field '{field}'"))?;
            let claim_bytes =
                encode_claim_bytes_with_hash(&mutated, fingerprint_hash(&mutated), None);
            let env = build_signed_envelope(claim_bytes, kp, pk);
            let expected_field = match field.as_str() {
                "ManifestHash" | "SpiritVersion" => "FingerprintHash".to_string(),
                f => f.to_string(),
            };
            Ok((env, "reject", Some("ContextDrift"), Some(expected_field)))
        }
        other => Err(format!("unknown CCAC malform op '{other}'")),
    }
}

fn sign_json(
    json: serde_json::Value,
    kp: &ring::signature::Ed25519KeyPair,
    pk: [u8; 32],
) -> (
    ComplianceClaimEnvelope,
    &'static str,
    Option<&'static str>,
    Option<String>,
) {
    let claim_bytes = serde_json::to_vec(&json).expect("json serializes");
    let env = build_signed_envelope(claim_bytes, kp, pk);
    (env, "reject", Some("MalformedClaim"), None)
}

/// Mutate exactly one structural field of `ref_fp` to a value that differs from
/// the reference (so the evaluator names that field as the drift).
/// Returns `None` for unknown field names.
fn mutate_field(
    ref_fp: &ExecutionContextFingerprint,
    field: &str,
) -> Option<ExecutionContextFingerprint> {
    let mut fp = ref_fp.clone();
    match field {
        "TrustTier" => fp.trust_tier = other_tier(ref_fp.trust_tier),
        "SandboxTier" => fp.sandbox_tier = other_sandbox(ref_fp.sandbox_tier),
        "CapabilityScope" => {
            fp.capability_scope
                .insert(CapabilityId("net.exfiltrate.drift".into()));
        }
        "ProviderEndpoint" => {
            fp.provider_endpoint.endpoint_url =
                format!("https://drift.{}", ref_fp.provider_endpoint.endpoint_url);
        }
        "CryptoProvider" => fp.crypto_provider = CryptoProviderId("fips-module-drift".into()),
        "ManifestHash" => {
            let mut bytes = fp.manifest_hash;
            bytes[0] ^= 0xFF;
            bytes[1] ^= 0xAA;
            fp.manifest_hash = bytes;
        }
        "SpiritVersion" => {
            fp.spirit_version = format!("{}-drifted", fp.spirit_version);
        }
        _ => return None,
    }
    Some(fp)
}

fn other_tier(t: TrustTier) -> TrustTier {
    match t {
        TrustTier::Local => TrustTier::PublicUntrusted,
        TrustTier::OrgInternal => TrustTier::Local,
        TrustTier::PublicVetted => TrustTier::Local,
        TrustTier::PublicUntrusted => TrustTier::Local,
    }
}

fn other_sandbox(t: SandboxTier) -> SandboxTier {
    match t {
        SandboxTier::T0 => SandboxTier::T4,
        SandboxTier::T1 => SandboxTier::T4,
        SandboxTier::T2 => SandboxTier::T0,
        SandboxTier::T3 => SandboxTier::T0,
        SandboxTier::T4 => SandboxTier::T0,
    }
}
