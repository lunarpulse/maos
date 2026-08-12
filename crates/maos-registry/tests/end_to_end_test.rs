//! End-to-end integration test for Spirit Registry (Story 5-5d, AC6).
//!
//! Exercises the full path: FixtureReplay client → registry operations →
//! admission decisions for all three active trust tiers.
//!
//! Run with: `cargo test -p maos-registry --features fixture_replay`

#![cfg(feature = "fixture_replay")]

use maos_compliance::vetting::keyring::issue_event;
use maos_compliance::{
    issue_attestation, RevocationSemantics, VetterKeyEventClaim, VetterKeyEventKind, VetterKeyring,
    VettingClaim,
};
use maos_domain::ports::registry::{
    PublishReceipt, SearchQuery, SignedPackage, SpiritId, SpiritRegistryClient, YankReason,
};
use maos_registry::admission::{
    admit_spirit, admit_spirit_with_attestation, AdmissionConfig, AdmissionDecision, AdmissionError,
};
use maos_registry::compliance_verify;
use maos_registry::fixture_replay::FixtureReplaySpiritRegistryClient;
use maos_registry::storage::{LocalFsRegistryStorage, RegistryStorage};
use maos_spirit_abi::compliance::{
    ComplianceClaimEnvelope, CryptoProviderId, ExecutionContextFingerprint, ProviderEndpointPin,
    SandboxTier, SigningAlg, TrustTier,
};
use std::collections::BTreeSet;

fn make_keypair() -> (ring::signature::Ed25519KeyPair, [u8; 32]) {
    use ring::rand::SystemRandom;
    use ring::signature::{Ed25519KeyPair, KeyPair};
    let rng = SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    let kp = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
    let pubkey: [u8; 32] = kp.public_key().as_ref().try_into().unwrap();
    (kp, pubkey)
}

fn make_signed_package(
    spirit_id: &str,
    version: &str,
    manifest: &[u8],
    artifact: &[u8],
    tier: TrustTier,
    keypair: &ring::signature::Ed25519KeyPair,
    pubkey: &[u8; 32],
) -> SignedPackage {
    use sha2::Digest;

    // Domain-separated to match `admission::verify_publisher_sig`
    // (sha256(manifest_len_u64 || manifest || artifact_len_u64 || artifact)).
    let mut hasher = sha2::Sha256::new();
    hasher.update((manifest.len() as u64).to_le_bytes());
    hasher.update(manifest);
    hasher.update((artifact.len() as u64).to_le_bytes());
    hasher.update(artifact);
    let msg = hasher.finalize();
    let signature = keypair.sign(&msg);

    let manifest_hash: [u8; 32] = {
        let mut h = sha2::Sha256::new();
        h.update(manifest);
        let r = h.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&r);
        out
    };

    let fp = ExecutionContextFingerprint {
        manifest_hash,
        spirit_version: version.to_string(),
        trust_tier: tier,
        sandbox_tier: SandboxTier::T3,
        capability_scope: BTreeSet::new(),
        provider_endpoint: ProviderEndpointPin {
            provider_id: String::new(),
            endpoint_url: String::new(),
            model_id: None,
        },
        crypto_provider: CryptoProviderId(String::new()),
    };
    let fp_hash = compliance_verify::compute_fingerprint_hash(&fp);
    let fp_hex = hex::encode(fp_hash);

    let tier_str = match tier {
        TrustTier::Local => "local",
        TrustTier::OrgInternal => "org_internal",
        TrustTier::PublicVetted => "public_vetted",
        TrustTier::PublicUntrusted => "public_untrusted",
    };

    let claim_json = serde_json::json!({
        "fingerprint_hash": fp_hex,
        "trust_tier": tier_str,
        "sandbox_tier": "t3",
        "capability_scope": [],
        "provider_endpoint": {"provider_id": "", "endpoint_url": ""},
        "crypto_provider": ""
    });
    let claim_bytes = serde_json::to_vec(&claim_json).unwrap();
    let claim_sig = keypair.sign(&claim_bytes);

    let envelope = ComplianceClaimEnvelope {
        signature: claim_sig.as_ref().try_into().unwrap(),
        attester_pubkey: *pubkey,
        claim_bytes,
        signing_alg: SigningAlg::Ed25519,
    };

    SignedPackage::new(
        SpiritId::from(spirit_id),
        version.to_string(),
        manifest.to_vec(),
        artifact.to_vec(),
        signature.as_ref().try_into().unwrap(),
        *pubkey,
        envelope,
    )
}

fn local_config() -> AdmissionConfig {
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
fn e2e_local_tier_admit_unsigned() {
    let manifest =
        b"[spirit]\nname = \"local-spirit\"\nversion = \"0.1.0\"\ntrust_tier = \"local\"\n";
    let artifact = b"local-binary".to_vec();

    let (kp, pubkey) = make_keypair();
    let pkg = make_signed_package(
        "local-spirit",
        "0.1.0",
        manifest,
        &artifact,
        TrustTier::Local,
        &kp,
        &pubkey,
    );

    let decision = admit_spirit(&pkg, &local_config()).unwrap();
    assert!(decision.admit);
    assert_eq!(decision.effective_tier, TrustTier::Local);
}

#[test]
fn e2e_public_untrusted_admit_with_valid_compliance() {
    let manifest = b"[spirit]\nname = \"pub-spirit\"\nversion = \"0.1.0\"\ntrust_tier = \"public_untrusted\"\nsandbox_tier = \"t3\"\n";
    let artifact = b"pub-binary".to_vec();

    let (kp, pubkey) = make_keypair();
    let pkg = make_signed_package(
        "pub-spirit",
        "0.1.0",
        manifest,
        &artifact,
        TrustTier::PublicUntrusted,
        &kp,
        &pubkey,
    );

    let decision = admit_spirit(&pkg, &local_config()).unwrap();
    assert!(decision.admit);
    assert_eq!(decision.effective_tier, TrustTier::PublicUntrusted);
}

/// Story 13.4 (FR37 / ADR-056) leg 6 — the pre-existing "always rejected"
/// negative is REPURPOSED as the anti-null control for the un-defer: rejected
/// WITHOUT a valid attestation, admitted WITH one.
#[test]
fn e2e_public_vetted_always_rejected() {
    let manifest = b"[spirit]\nname = \"vetted-spirit\"\nversion = \"0.1.0\"\ntrust_tier = \"public-vetted\"\nsandbox_tier = \"t3\"\n";
    let artifact = b"vetted-binary".to_vec();

    let (kp, pubkey) = make_keypair();
    let pkg = make_signed_package(
        "vetted-spirit",
        "0.1.0",
        manifest,
        &artifact,
        TrustTier::PublicUntrusted,
        &kp,
        &pubkey,
    );

    // Operator vetter keyring: an operator-root-signed enrollment predating issuance.
    let op_seed = [0x44u8; 32];
    let vetter_seed = [0x33u8; 32];
    let ed_pub = |seed: &[u8; 32]| -> [u8; 32] {
        use ring::signature::{Ed25519KeyPair, KeyPair};
        Ed25519KeyPair::from_seed_unchecked(seed)
            .unwrap()
            .public_key()
            .as_ref()
            .try_into()
            .unwrap()
    };
    let mut keyring = VetterKeyring::new(ed_pub(&op_seed));
    keyring.push(issue_event(
        &op_seed,
        &VetterKeyEventClaim {
            kind: VetterKeyEventKind::Enroll,
            vetter_key_id: "vetter-01".into(),
            vetter_pubkey: ed_pub(&vetter_seed),
            predecessor_pubkey: None,
            effective_at_unix_ms: 100,
            journal_sequence: 1,
            journaled_at_unix_ms: 100,
            note: "enrolled".into(),
        },
    ));

    let manifest_hash: [u8; 32] = {
        use sha2::Digest;
        let mut h = sha2::Sha256::new();
        h.update(manifest);
        h.finalize().into()
    };
    let att = issue_attestation(
        &vetter_seed,
        &VettingClaim {
            manifest_hash,
            spirit_id: "vetted-spirit".into(),
            spirit_version: "0.1.0".into(),
            from_tier: TrustTier::PublicUntrusted,
            to_tier: TrustTier::PublicVetted,
            vetter_key_id: "vetter-01".into(),
            issued_at_unix_ms: 500,
            expires_at_unix_ms: 2_000,
            revocation_semantics: RevocationSemantics::RefuseAtNextLoad,
            successor_policy: None,
        },
    );

    // WITHOUT a valid attestation → deferred (rejected).
    let deferred = admit_spirit_with_attestation(
        &pkg,
        &local_config(),
        None,
        &keyring,
        &ed_pub(&op_seed),
        1_000,
    );
    assert!(matches!(
        deferred,
        Err(AdmissionError::PublicVettedDeferred)
    ));

    // WITH a valid attestation → admitted as public-vetted.
    let decision = admit_spirit_with_attestation(
        &pkg,
        &local_config(),
        Some(&att),
        &keyring,
        &ed_pub(&op_seed),
        1_000,
    )
    .unwrap();
    assert!(decision.admit);
    assert_eq!(decision.effective_tier, TrustTier::PublicVetted);
}

#[test]
fn e2e_strictest_of_three_sources() {
    let manifest = b"[spirit]\nname = \"test\"\nversion = \"0.1.0\"\ntrust_tier = \"local\"\n";
    let artifact = b"bin".to_vec();
    let (kp, pubkey) = make_keypair();
    let pkg = make_signed_package(
        "test",
        "0.1.0",
        manifest,
        &artifact,
        TrustTier::Local,
        &kp,
        &pubkey,
    );

    let cfg = AdmissionConfig {
        tier_floor: TrustTier::Local,
        registry_origin_tier: TrustTier::PublicUntrusted,
        t3_for_public_untrusted: false,
        allow_unsigned_local: true,
        org_signing_pubkey: None,
        runtime_provider_endpoint: None,
        runtime_crypto_provider: None,
    };

    let result = admit_spirit(&pkg, &cfg);
    assert!(result.is_err());
    let err = format!("{:?}", result.unwrap_err());
    assert!(
        err.contains("PublisherSignatureInvalid") || err.contains("ComplianceContextDrift"),
        "expected signature or drift error for strictest-of escalation, got: {err}"
    );
}

#[test]
fn e2e_storage_publish_and_retrieve() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = LocalFsRegistryStorage::at_path(tmp.path().to_path_buf()).unwrap();

    let manifest = b"[spirit]\nname = \"stored\"\nversion = \"1.0.0\"\ntrust_tier = \"local\"\n";
    let artifact = b"stored-binary".to_vec();
    let (kp, pubkey) = make_keypair();
    let pkg = make_signed_package(
        "stored-spirit",
        "1.0.0",
        manifest,
        &artifact,
        TrustTier::Local,
        &kp,
        &pubkey,
    );

    storage
        .put(&SpiritId::from("stored-spirit"), "1.0.0", &pkg)
        .unwrap();

    let retrieved = storage
        .get_manifest(&SpiritId::from("stored-spirit"), "1.0.0")
        .unwrap();
    assert_eq!(retrieved.manifest_toml, manifest.to_vec());

    let artifact_out = storage
        .get_artifact(&SpiritId::from("stored-spirit"), "1.0.0")
        .unwrap();
    assert_eq!(artifact_out.artifact_bytes, artifact);
}

#[test]
fn e2e_storage_search() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = LocalFsRegistryStorage::at_path(tmp.path().to_path_buf()).unwrap();

    let manifest =
        b"[spirit]\nname = \"searchable\"\nversion = \"0.1.0\"\ntrust_tier = \"local\"\n";
    let artifact = b"bin".to_vec();
    let (kp, pubkey) = make_keypair();
    let pkg = make_signed_package(
        "searchable",
        "0.1.0",
        manifest,
        &artifact,
        TrustTier::Local,
        &kp,
        &pubkey,
    );
    storage
        .put(&SpiritId::from("searchable"), "0.1.0", &pkg)
        .unwrap();

    let q = SearchQuery::new("search".into(), false, 10);
    let results = storage.search(&q).unwrap();
    assert_eq!(results.items.len(), 1);
    assert_eq!(results.items[0].spirit_id.as_str(), "searchable");
}

#[test]
fn e2e_compliance_fingerprint_mismatch_rejected() {
    let manifest = b"[spirit]\nname = \"drift-test\"\nversion = \"0.1.0\"\ntrust_tier = \"public_untrusted\"\nsandbox_tier = \"t3\"\n";
    let artifact = b"drift-bin".to_vec();

    let (kp, pubkey) = make_keypair();

    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update((manifest.len() as u64).to_le_bytes());
    hasher.update(manifest);
    hasher.update((artifact.len() as u64).to_le_bytes());
    hasher.update(&artifact);
    let msg = hasher.finalize();
    let signature = kp.sign(&msg);

    let claim_json = serde_json::json!({
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
        attester_pubkey: pubkey,
        claim_bytes,
        signing_alg: SigningAlg::Ed25519,
    };

    let pkg = SignedPackage::new(
        SpiritId::from("drift-test"),
        "0.1.0".into(),
        manifest.to_vec(),
        artifact,
        signature.as_ref().try_into().unwrap(),
        pubkey,
        envelope,
    );

    let result = admit_spirit(&pkg, &local_config());
    assert!(result.is_err());
    let err = format!("{:?}", result.unwrap_err());
    assert!(
        err.contains("ComplianceContextDrift"),
        "expected drift error, got: {err}"
    );
}

#[test]
fn e2e_t3_sandbox_for_public_untrusted() {
    let manifest = b"[spirit]\nname = \"t3-test\"\nversion = \"0.1.0\"\ntrust_tier = \"public_untrusted\"\nsandbox_tier = \"t3\"\n";
    let artifact = b"t3-bin".to_vec();

    let (kp, pubkey) = make_keypair();
    let pkg = make_signed_package(
        "t3-test",
        "0.1.0",
        manifest,
        &artifact,
        TrustTier::PublicUntrusted,
        &kp,
        &pubkey,
    );

    let cfg = AdmissionConfig {
        tier_floor: TrustTier::Local,
        registry_origin_tier: TrustTier::Local,
        t3_for_public_untrusted: true,
        allow_unsigned_local: true,
        org_signing_pubkey: None,
        runtime_provider_endpoint: None,
        runtime_crypto_provider: None,
    };

    let decision = admit_spirit(&pkg, &cfg).unwrap();
    assert!(decision.admit);
    assert_eq!(decision.sandbox_tier_floor, SandboxTier::T3);
}
