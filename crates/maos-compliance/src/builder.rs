//! Envelope + claim construction shared by the CCAC generator
//! (`maos-corpus-gen::ccac`), the evaluator tests, and the smoke arm.
//!
//! [`encode_claim_bytes`] produces the SAME CBOR map shape the v0.5-α producer
//! (`maos-spirit-cli::compliance_claim::auto_populate`) emits, so the evaluator
//! decodes both byte-shapes identically and the generator→evaluator round-trip
//! holds. The fingerprint hash inside the claim is computed with the shared
//! [`crate::canonical_cbor::fingerprint_hash`], so a well-formed envelope's
//! claimed hash equals the evaluator's recomputed runtime hash byte-for-byte.

use maos_spirit_abi::compliance::{
    ComplianceClaimEnvelope, ExecutionContextFingerprint, SigningAlg,
};

use crate::canonical_cbor;

/// Encode the producer-shaped claim map (CBOR) for a fingerprint.
///
/// `fingerprint_hash` is computed from `fp` via the shared canonical encoder.
/// The five structural fields are emitted verbatim from `fp`. `expires_at_unix_ms`
/// is appended only when `Some` (mirroring the producer's "omit when absent").
pub fn encode_claim_bytes(
    fp: &ExecutionContextFingerprint,
    expires_at_unix_ms: Option<u64>,
) -> Vec<u8> {
    encode_claim_bytes_with_hash(fp, canonical_cbor::fingerprint_hash(fp), expires_at_unix_ms)
}

/// Like [`encode_claim_bytes`] but with an explicit `fingerprint_hash` — used to
/// author internally-consistent *drifted* claims (the claim attests its own
/// mutated fields + a hash over those mutated fields, then drifts from the
/// runtime context it is bound to).
pub fn encode_claim_bytes_with_hash(
    fp: &ExecutionContextFingerprint,
    fingerprint_hash: [u8; 32],
    expires_at_unix_ms: Option<u64>,
) -> Vec<u8> {
    use serde_cbor::value::Value;
    use std::collections::BTreeMap;

    let mut m: BTreeMap<Value, Value> = BTreeMap::new();
    m.insert(
        Value::Text("fingerprint_hash".into()),
        Value::Text(hex::encode(fingerprint_hash)),
    );
    m.insert(
        Value::Text("trust_tier".into()),
        Value::Text(trust_tier_str(fp.trust_tier).into()),
    );
    m.insert(
        Value::Text("sandbox_tier".into()),
        Value::Text(sandbox_tier_str(fp.sandbox_tier).into()),
    );
    let caps: Vec<Value> = fp
        .capability_scope
        .iter()
        .map(|c| Value::Text(c.0.clone()))
        .collect();
    m.insert(Value::Text("capability_scope".into()), Value::Array(caps));

    let mut pe: BTreeMap<Value, Value> = BTreeMap::new();
    pe.insert(
        Value::Text("provider_id".into()),
        Value::Text(fp.provider_endpoint.provider_id.clone()),
    );
    pe.insert(
        Value::Text("endpoint_url".into()),
        Value::Text(fp.provider_endpoint.endpoint_url.clone()),
    );
    if let Some(mid) = &fp.provider_endpoint.model_id {
        pe.insert(Value::Text("model_id".into()), Value::Text(mid.clone()));
    }
    m.insert(Value::Text("provider_endpoint".into()), Value::Map(pe));
    m.insert(
        Value::Text("crypto_provider".into()),
        Value::Text(fp.crypto_provider.0.clone()),
    );
    if let Some(exp) = expires_at_unix_ms {
        m.insert(
            Value::Text("expires_at_unix_ms".into()),
            Value::Integer(exp as i128),
        );
    }

    serde_cbor::to_vec(&Value::Map(m)).expect("claim map is always CBOR-serializable")
}

/// Sign `claim_bytes` directly (matching the producer + the evaluator's step 1)
/// and wrap into an envelope.
pub fn build_signed_envelope(
    claim_bytes: Vec<u8>,
    keypair: &ring::signature::Ed25519KeyPair,
    attester_pubkey: [u8; 32],
) -> ComplianceClaimEnvelope {
    let sig = keypair.sign(&claim_bytes);
    let signature: [u8; 64] = sig
        .as_ref()
        .try_into()
        .expect("Ed25519 signature is always 64 bytes");
    ComplianceClaimEnvelope {
        signature,
        attester_pubkey,
        claim_bytes,
        signing_alg: SigningAlg::Ed25519,
    }
}

/// Build a well-formed self-attested envelope for `fp` signed by `keypair`.
pub fn build_self_attested_envelope(
    fp: &ExecutionContextFingerprint,
    keypair: &ring::signature::Ed25519KeyPair,
    attester_pubkey: [u8; 32],
) -> ComplianceClaimEnvelope {
    let claim_bytes = encode_claim_bytes(fp, None);
    build_signed_envelope(claim_bytes, keypair, attester_pubkey)
}

/// Deterministic Ed25519 keypair from a 4-byte seed prefix.
///
/// # WARNING — NOT FOR PRODUCTION ADMISSION
///
/// This function produces keys with only 32 bits of entropy (the seed is
/// repeated across 32 bytes). It is safe ONLY for tests, corpus generation,
/// and smoke arms. Any use in the production admission path is a forgery
/// vulnerability.
#[deprecated(note = "TEST/CORPUS/SMOKE ONLY — never use in production admission")]
pub fn seeded_keypair(seed: u32) -> (ring::signature::Ed25519KeyPair, [u8; 32]) {
    use ring::signature::{Ed25519KeyPair, KeyPair};
    let prefix = seed.to_le_bytes();
    let mut s = [0u8; 32];
    for (i, b) in s.iter_mut().enumerate() {
        *b = prefix[i % 4];
    }
    let kp = Ed25519KeyPair::from_seed_unchecked(&s).expect("valid 32-byte seed");
    let pk: [u8; 32] = kp
        .public_key()
        .as_ref()
        .try_into()
        .expect("Ed25519 public key is 32 bytes");
    (kp, pk)
}

fn trust_tier_str(t: maos_spirit_abi::compliance::TrustTier) -> &'static str {
    use maos_spirit_abi::compliance::TrustTier::*;
    match t {
        Local => "local",
        OrgInternal => "org_internal",
        PublicVetted => "public_vetted",
        PublicUntrusted => "public_untrusted",
    }
}

fn sandbox_tier_str(t: maos_spirit_abi::compliance::SandboxTier) -> &'static str {
    use maos_spirit_abi::compliance::SandboxTier::*;
    match t {
        T0 => "t0",
        T1 => "t1",
        T2 => "t2",
        T3 => "t3",
        T4 => "t4",
    }
}
