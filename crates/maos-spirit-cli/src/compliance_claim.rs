//! ComplianceClaim envelope load + auto-population for `maos-spirit publish`.
//!
//! Story 7.2 AC2 §4: when `--compliance-claim` is absent, the CLI builds a
//! self-attested envelope structurally compatible with Story 5.5d's
//! `compliance_verify::verify_envelope_structural` so that the fingerprint
//! hash + per-field defense-in-depth checks both pass.

use ring::signature::Ed25519KeyPair;
use sha2::Digest;

use maos_registry::compliance_verify::{
    compute_fingerprint_hash, extract_manifest_fingerprint_fields,
};
use maos_spirit_abi::compliance::{
    ComplianceClaimEnvelope, ExecutionContextFingerprint, SigningAlg,
};

use crate::errors::CliError;

/// Load an externally-baked ComplianceClaim envelope from a file.
///
/// Accepts canonical CBOR (preferred) with a JSON-fallback for
/// fixture-author convenience — same posture as Story 5.5d's
/// `compliance_verify::parse_claim`.
pub fn load_envelope(path: &std::path::Path) -> Result<ComplianceClaimEnvelope, CliError> {
    let bytes = std::fs::read(path)
        .map_err(|e| CliError::ComplianceClaimLoad(format!("read {:?}: {e}", path)))?;
    // Try CBOR first
    if let Ok(env) = serde_cbor::from_slice::<ComplianceClaimEnvelope>(&bytes) {
        return Ok(env);
    }
    // Fall back to JSON
    serde_json::from_slice::<ComplianceClaimEnvelope>(&bytes)
        .map_err(|e| CliError::ComplianceClaimLoad(format!("CBOR+JSON decode failed: {e}")))
}

/// Auto-populate a self-attested envelope from the manifest.
///
/// The envelope's `attester_pubkey == publisher_pubkey` (self-attested per
/// §8.5 v0.5 binding posture). The `claim_bytes` payload carries the same
/// fields Story 5.5d's `parse_claim` expects, ensuring the structural
/// verifier will accept it at admission time.
pub fn auto_populate(
    manifest_toml: &[u8],
    spirit_version: &str,
    publisher_pubkey: &[u8; 32],
    signing_pair: &Ed25519KeyPair,
) -> Result<ComplianceClaimEnvelope, CliError> {
    let manifest_fields = extract_manifest_fingerprint_fields(manifest_toml);

    let actual = ExecutionContextFingerprint {
        manifest_hash: sha256(manifest_toml),
        spirit_version: spirit_version.to_string(),
        trust_tier: manifest_fields.trust_tier,
        sandbox_tier: manifest_fields.sandbox_tier,
        capability_scope: manifest_fields.capability_scope.clone(),
        provider_endpoint: manifest_fields.provider_endpoint.clone(),
        crypto_provider: manifest_fields.crypto_provider.clone(),
    };
    let fingerprint_hash = compute_fingerprint_hash(&actual);

    // Build the minimal JSON-shaped claim Story 5.5d's parse_claim expects.
    // We encode through serde_cbor for the on-wire shape.
    let claim_value = serde_cbor::value::Value::Map({
        let mut m = std::collections::BTreeMap::new();
        m.insert(
            serde_cbor::value::Value::Text("fingerprint_hash".into()),
            serde_cbor::value::Value::Text(hex::encode(fingerprint_hash)),
        );
        m.insert(
            serde_cbor::value::Value::Text("trust_tier".into()),
            serde_cbor::value::Value::Text(trust_tier_str(manifest_fields.trust_tier).into()),
        );
        m.insert(
            serde_cbor::value::Value::Text("sandbox_tier".into()),
            serde_cbor::value::Value::Text(sandbox_tier_str(manifest_fields.sandbox_tier).into()),
        );
        let caps: Vec<serde_cbor::value::Value> = manifest_fields
            .capability_scope
            .iter()
            .map(|c| serde_cbor::value::Value::Text(c.0.clone()))
            .collect();
        m.insert(
            serde_cbor::value::Value::Text("capability_scope".into()),
            serde_cbor::value::Value::Array(caps),
        );
        let mut pe = std::collections::BTreeMap::new();
        pe.insert(
            serde_cbor::value::Value::Text("provider_id".into()),
            serde_cbor::value::Value::Text(manifest_fields.provider_endpoint.provider_id.clone()),
        );
        pe.insert(
            serde_cbor::value::Value::Text("endpoint_url".into()),
            serde_cbor::value::Value::Text(manifest_fields.provider_endpoint.endpoint_url.clone()),
        );
        if let Some(mid) = &manifest_fields.provider_endpoint.model_id {
            pe.insert(
                serde_cbor::value::Value::Text("model_id".into()),
                serde_cbor::value::Value::Text(mid.clone()),
            );
        }
        m.insert(
            serde_cbor::value::Value::Text("provider_endpoint".into()),
            serde_cbor::value::Value::Map(pe),
        );
        m.insert(
            serde_cbor::value::Value::Text("crypto_provider".into()),
            serde_cbor::value::Value::Text(manifest_fields.crypto_provider.0.clone()),
        );
        m
    });
    let claim_bytes = serde_cbor::to_vec(&claim_value)
        .map_err(|e| CliError::Other(format!("CBOR encode claim: {e}")))?;

    // Sign claim_bytes directly per Story 5.5d's compliance_verify::verify_ed25519
    // (which calls `pk.verify(message=&claim_bytes, signature)`). The class-level
    // doc on ComplianceClaimEnvelope mentions `sign_bytes = sha256(claim_bytes)` but
    // the actual verifier signs over raw claim_bytes; Story 7.2 matches the
    // verifier shape so the envelope round-trips through admission.
    let signature_arr: [u8; 64] = {
        let sig = signing_pair.sign(&claim_bytes);
        let sig_bytes = sig.as_ref();
        if sig_bytes.len() != 64 {
            return Err(CliError::Other(format!(
                "expected 64-byte Ed25519 signature, got {} bytes",
                sig_bytes.len()
            )));
        }
        let mut out = [0u8; 64];
        out.copy_from_slice(sig_bytes);
        out
    };

    Ok(ComplianceClaimEnvelope {
        signature: signature_arr,
        attester_pubkey: *publisher_pubkey,
        claim_bytes,
        signing_alg: SigningAlg::Ed25519,
    })
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    let r = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&r);
    out
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
