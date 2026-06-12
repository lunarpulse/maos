//! Story 7.3 AC3 — admission rewired to the RUNTIME execution-context
//! fingerprint (FR38 v1.0). A claim whose attested context drifts from the
//! kernel's actual runtime context is rejected with a field-named
//! `ComplianceContextDrift`. The v0.5-α manifest-matching fixture still admits.

use std::collections::BTreeSet;

#[allow(deprecated)]
use maos_compliance::builder::seeded_keypair;
use maos_compliance::builder::{build_signed_envelope, encode_claim_bytes_with_hash};
use maos_compliance::canonical_cbor::fingerprint_hash;
use maos_domain::ports::registry::{SignedPackage, SpiritId, TrustTier};
use maos_registry::admission::{admit_spirit, AdmissionConfig, AdmissionError};
use maos_spirit_abi::compliance::{
    CryptoProviderId, ExecutionContextFingerprint, ProviderEndpointPin, SandboxTier,
};
use sha2::Digest;

const MANIFEST: &[u8] = b"[spirit]\nname = \"drift-spirit\"\nversion = \"0.1.0\"\ntrust_tier = \"public_untrusted\"\nsandbox_tier = \"t3\"\nprovider_id = \"anthropic\"\nendpoint_url = \"https://api.anthropic.com\"\ncrypto_provider = \"ring\"\n";

fn manifest_hash() -> [u8; 32] {
    let mut h = sha2::Sha256::new();
    h.update(MANIFEST);
    h.finalize().into()
}

/// The honest manifest-derived fingerprint (what a well-formed claim attests).
fn honest_fp() -> ExecutionContextFingerprint {
    ExecutionContextFingerprint {
        manifest_hash: manifest_hash(),
        spirit_version: "0.1.0".into(),
        trust_tier: TrustTier::PublicUntrusted,
        sandbox_tier: SandboxTier::T3,
        capability_scope: BTreeSet::new(),
        provider_endpoint: ProviderEndpointPin {
            provider_id: "anthropic".into(),
            endpoint_url: "https://api.anthropic.com".into(),
            model_id: None,
        },
        crypto_provider: CryptoProviderId("ring".into()),
    }
}

/// Build a public_untrusted package whose ComplianceClaim attests `claimed_fp`,
/// with a valid publisher signature + valid self-attestation signature.
fn pkg_attesting(claimed_fp: &ExecutionContextFingerprint) -> SignedPackage {
    let (kp, pk) = seeded_keypair(0x150C_04A5);
    let artifact = b"binary".to_vec();

    let claim_bytes = encode_claim_bytes_with_hash(claimed_fp, fingerprint_hash(claimed_fp), None);
    let envelope = build_signed_envelope(claim_bytes, &kp, pk);

    // Publisher sig over sha256(manifest_len||manifest||artifact_len||artifact).
    let mut h = sha2::Sha256::new();
    h.update(&(MANIFEST.len() as u64).to_le_bytes());
    h.update(MANIFEST);
    h.update(&(artifact.len() as u64).to_le_bytes());
    h.update(&artifact);
    let msg = h.finalize();
    let pkg_sig: [u8; 64] = kp.sign(&msg).as_ref().try_into().unwrap();

    SignedPackage::new(
        SpiritId::from("drift-spirit"),
        "0.1.0".into(),
        MANIFEST.to_vec(),
        artifact,
        pkg_sig,
        pk,
        envelope,
    )
}

fn base_cfg() -> AdmissionConfig {
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

#[test]
fn manifest_matching_envelope_still_admits() {
    // v0.5-α backward compatibility: honest claim + no runtime override → admit.
    let pkg = pkg_attesting(&honest_fp());
    let decision = admit_spirit(&pkg, &base_cfg()).expect("expected admit");
    assert!(decision.admit);
    assert_eq!(decision.effective_tier, TrustTier::PublicUntrusted);
}

#[test]
fn trust_tier_drift_rejects_naming_field() {
    // Claim attests trust_tier=local; admitted as public_untrusted (manifest
    // declares public_untrusted → strictest-of effective tier is public_untrusted).
    let mut claimed = honest_fp();
    claimed.trust_tier = TrustTier::Local;
    let pkg = pkg_attesting(&claimed);
    let err = admit_spirit(&pkg, &base_cfg()).unwrap_err();
    match err {
        AdmissionError::ComplianceContextDrift { field, .. } => {
            assert_eq!(field, "TrustTier", "expected TrustTier drift, got {field}");
        }
        other => panic!("expected ComplianceContextDrift, got {other:?}"),
    }
}

#[test]
fn crypto_provider_drift_rejects_naming_field() {
    // Claim attests crypto_provider="ring"; operator runtime forces "fips-module".
    let pkg = pkg_attesting(&honest_fp());
    let cfg = AdmissionConfig {
        runtime_crypto_provider: Some(CryptoProviderId("fips-module".into())),
        ..base_cfg()
    };
    let err = admit_spirit(&pkg, &cfg).unwrap_err();
    match err {
        AdmissionError::ComplianceContextDrift {
            field,
            actual,
            claimed,
        } => {
            assert_eq!(field, "CryptoProvider");
            assert_eq!(actual, "fips-module");
            assert_eq!(claimed, "ring");
        }
        other => panic!("expected CryptoProvider drift, got {other:?}"),
    }
}

#[test]
fn provider_endpoint_drift_rejects_naming_field() {
    // Claim attests the manifest endpoint; operator runtime resolves a different one.
    let pkg = pkg_attesting(&honest_fp());
    let cfg = AdmissionConfig {
        runtime_provider_endpoint: Some(ProviderEndpointPin {
            provider_id: "anthropic".into(),
            endpoint_url: "https://evil.proxy.example".into(),
            model_id: None,
        }),
        ..base_cfg()
    };
    let err = admit_spirit(&pkg, &cfg).unwrap_err();
    match err {
        AdmissionError::ComplianceContextDrift { field, .. } => {
            assert_eq!(field, "ProviderEndpoint");
        }
        other => panic!("expected ProviderEndpoint drift, got {other:?}"),
    }
}

#[test]
fn honest_claim_with_matching_runtime_override_admits() {
    // Operator runtime override that AGREES with the claim → still admits.
    let pkg = pkg_attesting(&honest_fp());
    let cfg = AdmissionConfig {
        runtime_crypto_provider: Some(CryptoProviderId("ring".into())),
        runtime_provider_endpoint: Some(ProviderEndpointPin {
            provider_id: "anthropic".into(),
            endpoint_url: "https://api.anthropic.com".into(),
            model_id: None,
        }),
        ..base_cfg()
    };
    let decision = admit_spirit(&pkg, &cfg).expect("expected admit");
    assert!(decision.admit);
}
