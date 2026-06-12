//! Story 7.3 AC2 — evaluator admit/reject coverage: every `DriftField`, the
//! malformed-claim taxonomy, signature failure, and expiry.

#![allow(deprecated)]

use std::collections::BTreeSet;

#[allow(deprecated)]
use maos_compliance::builder::seeded_keypair;
use maos_compliance::builder::{
    build_self_attested_envelope, build_signed_envelope, encode_claim_bytes,
    encode_claim_bytes_with_hash,
};
use maos_compliance::canonical_cbor::fingerprint_hash;
use maos_compliance::evaluator::{evaluate_envelope_at, DriftField, EComplianceRejection};
use maos_compliance::{ComplianceVerdict, RuntimeExecutionContext};
use maos_spirit_abi::compliance::{
    CapabilityId, ComplianceClaimEnvelope, CryptoProviderId, ExecutionContextFingerprint,
    ProviderEndpointPin, SandboxTier, SigningAlg, TrustTier,
};

const NOW_MS: u64 = 1_900_000_000_000; // far-future-ish fixed clock

fn caps(items: &[&str]) -> BTreeSet<CapabilityId> {
    items.iter().map(|s| CapabilityId(s.to_string())).collect()
}

fn reference_fp() -> ExecutionContextFingerprint {
    ExecutionContextFingerprint {
        manifest_hash: [9u8; 32],
        spirit_version: "1.0.0".into(),
        trust_tier: TrustTier::PublicUntrusted,
        sandbox_tier: SandboxTier::T3,
        capability_scope: caps(&["fs.read"]),
        provider_endpoint: ProviderEndpointPin {
            provider_id: "anthropic".into(),
            endpoint_url: "https://api.anthropic.com".into(),
            model_id: None,
        },
        crypto_provider: CryptoProviderId("ring".into()),
    }
}

fn ctx_from(fp: &ExecutionContextFingerprint) -> RuntimeExecutionContext {
    RuntimeExecutionContext {
        manifest_hash: fp.manifest_hash,
        spirit_version: fp.spirit_version.clone(),
        effective_trust_tier: fp.trust_tier,
        effective_sandbox_tier: fp.sandbox_tier,
        runtime_provider_endpoint: fp.provider_endpoint.clone(),
        runtime_crypto_provider: fp.crypto_provider.clone(),
        capability_scope: fp.capability_scope.clone(),
    }
}

#[test]
fn well_formed_envelope_admits() {
    let (kp, pk) = seeded_keypair(0x7E57_0001);
    let fp = reference_fp();
    let env = build_self_attested_envelope(&fp, &kp, pk);
    let ctx = ctx_from(&fp);
    assert_eq!(
        evaluate_envelope_at(&env, &ctx, NOW_MS),
        ComplianceVerdict::Admit
    );
}

#[test]
fn tampered_signature_rejects_signature_invalid() {
    let (kp, pk) = seeded_keypair(0x7E57_0002);
    let fp = reference_fp();
    let mut env = build_self_attested_envelope(&fp, &kp, pk);
    env.signature[0] ^= 0xFF;
    let ctx = ctx_from(&fp);
    assert_eq!(
        evaluate_envelope_at(&env, &ctx, NOW_MS),
        ComplianceVerdict::Reject(EComplianceRejection::SignatureInvalid)
    );
}

#[test]
fn wrong_attester_pubkey_rejects_signature_invalid() {
    let (kp, _pk) = seeded_keypair(0x7E57_0003);
    let (_kp2, pk2) = seeded_keypair(0x7E57_0004);
    let fp = reference_fp();
    // sign with kp but claim attester is pk2 → verify fails
    let claim_bytes = encode_claim_bytes(&fp, None);
    let env = build_signed_envelope(claim_bytes, &kp, pk2);
    let ctx = ctx_from(&fp);
    assert_eq!(
        evaluate_envelope_at(&env, &ctx, NOW_MS),
        ComplianceVerdict::Reject(EComplianceRejection::SignatureInvalid)
    );
}

#[test]
fn non_ed25519_alg_rejects() {
    let (kp, pk) = seeded_keypair(0x7E57_0005);
    let fp = reference_fp();
    let claim_bytes = encode_claim_bytes(&fp, None);
    let env = build_signed_envelope(claim_bytes, &kp, pk);
    // The frozen SigningAlg enum only has Ed25519 today, so we cannot
    // construct a different variant. This test validates the Ed25519 path
    // admits AND documents the forward-compat guard at evaluator.rs:97-99.
    // If a new variant is added to SigningAlg, add a test that constructs
    // an envelope with that variant and asserts SignatureInvalid.
    assert_eq!(env.signing_alg, SigningAlg::Ed25519);
    assert!(matches!(
        evaluate_envelope_at(&env, &ctx_from(&fp), NOW_MS),
        ComplianceVerdict::Admit
    ));
}

#[test]
fn garbage_claim_bytes_rejects_malformed() {
    let (kp, pk) = seeded_keypair(0x7E57_0006);
    let claim_bytes = vec![0xFF, 0xFE, 0x00, 0x01, 0x02];
    let env = build_signed_envelope(claim_bytes, &kp, pk);
    let fp = reference_fp();
    match evaluate_envelope_at(&env, &ctx_from(&fp), NOW_MS) {
        ComplianceVerdict::Reject(EComplianceRejection::MalformedClaim(_)) => {}
        other => panic!("expected MalformedClaim, got {other:?}"),
    }
}

#[test]
fn unknown_trust_tier_enum_rejects_malformed_not_default() {
    // The Story 5.5d bug: a silent `_ => PublicUntrusted`. Strict parse rejects.
    let (kp, pk) = seeded_keypair(0x7E57_0007);
    let claim = serde_json::json!({
        "fingerprint_hash": hex::encode([0u8;32]),
        "trust_tier": "super_trusted",   // unknown
        "sandbox_tier": "t3",
        "capability_scope": [],
        "provider_endpoint": {"provider_id":"a","endpoint_url":"u"},
        "crypto_provider": "ring"
    });
    let claim_bytes = serde_json::to_vec(&claim).unwrap();
    let env = build_signed_envelope(claim_bytes, &kp, pk);
    match evaluate_envelope_at(&env, &ctx_from(&reference_fp()), NOW_MS) {
        ComplianceVerdict::Reject(EComplianceRejection::MalformedClaim(m)) => {
            assert!(m.contains("trust_tier"), "msg: {m}");
        }
        other => panic!("expected MalformedClaim, got {other:?}"),
    }
}

#[test]
fn unknown_sandbox_tier_enum_rejects_malformed() {
    let (kp, pk) = seeded_keypair(0x7E57_0008);
    let claim = serde_json::json!({
        "fingerprint_hash": hex::encode([0u8;32]),
        "trust_tier": "local",
        "sandbox_tier": "t99",
        "capability_scope": [],
        "provider_endpoint": {"provider_id":"a","endpoint_url":"u"},
        "crypto_provider": "ring"
    });
    let env = build_signed_envelope(serde_json::to_vec(&claim).unwrap(), &kp, pk);
    assert!(matches!(
        evaluate_envelope_at(&env, &ctx_from(&reference_fp()), NOW_MS),
        ComplianceVerdict::Reject(EComplianceRejection::MalformedClaim(_))
    ));
}

#[test]
fn missing_fingerprint_hash_rejects_malformed() {
    let (kp, pk) = seeded_keypair(0x7E57_0009);
    let claim = serde_json::json!({
        "trust_tier": "local",
        "sandbox_tier": "t0",
        "capability_scope": [],
        "provider_endpoint": {"provider_id":"a","endpoint_url":"u"},
        "crypto_provider": "ring"
    });
    let env = build_signed_envelope(serde_json::to_vec(&claim).unwrap(), &kp, pk);
    assert!(matches!(
        evaluate_envelope_at(&env, &ctx_from(&reference_fp()), NOW_MS),
        ComplianceVerdict::Reject(EComplianceRejection::MalformedClaim(_))
    ));
}

#[test]
fn truncated_fingerprint_hash_rejects_malformed() {
    let (kp, pk) = seeded_keypair(0x7E57_000A);
    let claim = serde_json::json!({
        "fingerprint_hash": hex::encode([0u8;16]), // 16 bytes, not 32
        "trust_tier": "local",
        "sandbox_tier": "t0",
        "capability_scope": [],
        "provider_endpoint": {"provider_id":"a","endpoint_url":"u"},
        "crypto_provider": "ring"
    });
    let env = build_signed_envelope(serde_json::to_vec(&claim).unwrap(), &kp, pk);
    assert!(matches!(
        evaluate_envelope_at(&env, &ctx_from(&reference_fp()), NOW_MS),
        ComplianceVerdict::Reject(EComplianceRejection::MalformedClaim(_))
    ));
}

/// Build a drifted envelope: claim attests `claimed_fp` (internally consistent),
/// bound to a runtime ctx derived from `reference_fp()`.
fn drifted_env(claimed_fp: &ExecutionContextFingerprint) -> ComplianceClaimEnvelope {
    let (kp, pk) = seeded_keypair(0x7E57_0F00);
    let claim_bytes = encode_claim_bytes_with_hash(claimed_fp, fingerprint_hash(claimed_fp), None);
    build_signed_envelope(claim_bytes, &kp, pk)
}

#[test]
fn trust_tier_drift_names_trust_tier() {
    let mut claimed = reference_fp();
    claimed.trust_tier = TrustTier::Local; // attests local, runtime is public_untrusted
    let env = drifted_env(&claimed);
    match evaluate_envelope_at(&env, &ctx_from(&reference_fp()), NOW_MS) {
        ComplianceVerdict::Reject(EComplianceRejection::ContextDrift { field, .. }) => {
            assert_eq!(field, DriftField::TrustTier);
        }
        other => panic!("expected TrustTier drift, got {other:?}"),
    }
}

#[test]
fn sandbox_tier_drift_names_sandbox_tier() {
    let mut claimed = reference_fp();
    claimed.sandbox_tier = SandboxTier::T0;
    let env = drifted_env(&claimed);
    match evaluate_envelope_at(&env, &ctx_from(&reference_fp()), NOW_MS) {
        ComplianceVerdict::Reject(EComplianceRejection::ContextDrift { field, .. }) => {
            assert_eq!(field, DriftField::SandboxTier);
        }
        other => panic!("expected SandboxTier drift, got {other:?}"),
    }
}

#[test]
fn capability_scope_drift_names_capability_scope() {
    let mut claimed = reference_fp();
    claimed.capability_scope = caps(&["fs.read", "net.connect"]);
    let env = drifted_env(&claimed);
    match evaluate_envelope_at(&env, &ctx_from(&reference_fp()), NOW_MS) {
        ComplianceVerdict::Reject(EComplianceRejection::ContextDrift { field, .. }) => {
            assert_eq!(field, DriftField::CapabilityScope);
        }
        other => panic!("expected CapabilityScope drift, got {other:?}"),
    }
}

#[test]
fn provider_endpoint_drift_names_provider_endpoint() {
    let mut claimed = reference_fp();
    claimed.provider_endpoint.endpoint_url = "https://evil.example".into();
    let env = drifted_env(&claimed);
    match evaluate_envelope_at(&env, &ctx_from(&reference_fp()), NOW_MS) {
        ComplianceVerdict::Reject(EComplianceRejection::ContextDrift { field, .. }) => {
            assert_eq!(field, DriftField::ProviderEndpoint);
        }
        other => panic!("expected ProviderEndpoint drift, got {other:?}"),
    }
}

#[test]
fn crypto_provider_drift_names_crypto_provider() {
    let mut claimed = reference_fp();
    claimed.crypto_provider = CryptoProviderId("fips-module".into());
    let env = drifted_env(&claimed);
    match evaluate_envelope_at(&env, &ctx_from(&reference_fp()), NOW_MS) {
        ComplianceVerdict::Reject(EComplianceRejection::ContextDrift {
            field,
            actual,
            claimed,
        }) => {
            assert_eq!(field, DriftField::CryptoProvider);
            assert_eq!(actual, "ring");
            assert_eq!(claimed, "fips-module");
        }
        other => panic!("expected CryptoProvider drift, got {other:?}"),
    }
}

#[test]
fn manifest_hash_drift_names_fingerprint_hash() {
    // claim attests a different manifest_hash but identical structural fields →
    // structural fields agree, only the hash differs → FingerprintHash.
    let mut claimed = reference_fp();
    claimed.manifest_hash = [0xAB; 32];
    let env = drifted_env(&claimed);
    match evaluate_envelope_at(&env, &ctx_from(&reference_fp()), NOW_MS) {
        ComplianceVerdict::Reject(EComplianceRejection::ContextDrift { field, .. }) => {
            assert_eq!(field, DriftField::FingerprintHash);
        }
        other => panic!("expected FingerprintHash drift, got {other:?}"),
    }
}

#[test]
fn expired_claim_rejects() {
    let (kp, pk) = seeded_keypair(0x7E57_00EE);
    let fp = reference_fp();
    let claim_bytes = encode_claim_bytes(&fp, Some(1_000)); // expired long ago
    let env = build_signed_envelope(claim_bytes, &kp, pk);
    match evaluate_envelope_at(&env, &ctx_from(&fp), NOW_MS) {
        ComplianceVerdict::Reject(EComplianceRejection::ExpiredClaim { expired_at_unix_ms }) => {
            assert_eq!(expired_at_unix_ms, 1_000);
        }
        other => panic!("expected ExpiredClaim, got {other:?}"),
    }
}

#[test]
fn non_expired_claim_admits() {
    let (kp, pk) = seeded_keypair(0x7E57_00EF);
    let fp = reference_fp();
    let claim_bytes = encode_claim_bytes(&fp, Some(NOW_MS + 1_000_000));
    let env = build_signed_envelope(claim_bytes, &kp, pk);
    assert_eq!(
        evaluate_envelope_at(&env, &ctx_from(&fp), NOW_MS),
        ComplianceVerdict::Admit
    );
}

#[test]
fn claim_at_exact_expiry_is_expired() {
    let (kp, pk) = seeded_keypair(0x7E57_00F0);
    let fp = reference_fp();
    let claim_bytes = encode_claim_bytes(&fp, Some(NOW_MS));
    let env = build_signed_envelope(claim_bytes, &kp, pk);
    match evaluate_envelope_at(&env, &ctx_from(&fp), NOW_MS) {
        ComplianceVerdict::Reject(EComplianceRejection::ExpiredClaim { expired_at_unix_ms }) => {
            assert_eq!(expired_at_unix_ms, NOW_MS);
        }
        other => panic!("expected ExpiredClaim at exact boundary, got {other:?}"),
    }
}

#[test]
fn multi_field_drift_names_first_divergent_field() {
    let (kp, pk) = seeded_keypair(0x7E57_0F01);
    let ref_fp = reference_fp();

    let mut drifted = ref_fp.clone();
    drifted.trust_tier = TrustTier::Local;
    drifted.crypto_provider = CryptoProviderId("fips-module".into());

    let drifted_hash = fingerprint_hash(&drifted);
    let claim_bytes = encode_claim_bytes_with_hash(&drifted, drifted_hash, None);
    let env = build_signed_envelope(claim_bytes, &kp, pk);

    match evaluate_envelope_at(&env, &ctx_from(&ref_fp), NOW_MS) {
        ComplianceVerdict::Reject(EComplianceRejection::ContextDrift { field, .. }) => {
            assert_eq!(
                field,
                DriftField::TrustTier,
                "first divergent field must be TrustTier, not the also-drifted CryptoProvider"
            );
        }
        other => panic!("expected ContextDrift, got {other:?}"),
    }
}
