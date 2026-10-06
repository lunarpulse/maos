#![forbid(unsafe_code)]

//! Story 17-3b AC4 — registry admission is form-aware before every tier path.

use std::collections::BTreeSet;

use maos_compliance::vetting::keyring::issue_event;
use maos_compliance::{
    issue_attestation, RevocationSemantics, VetterKeyEventClaim, VetterKeyEventKind, VetterKeyring,
    VettingAttestation, VettingClaim,
};
use maos_domain::ports::registry::{SignedPackage, SpiritId, TrustTier};
use maos_registry::admission::{
    admit_spirit, admit_spirit_with_attestation, AdmissionConfig, AdmissionError,
};
use maos_registry::compliance_verify::compute_fingerprint_hash;
use maos_spirit_abi::compliance::{
    ComplianceClaimEnvelope, CryptoProviderId, ExecutionContextFingerprint, ProviderEndpointPin,
    SandboxTier, SigningAlg,
};
use ring::signature::{Ed25519KeyPair, KeyPair};
use sha2::Digest;

const VETTER_SEED: [u8; 32] = [0x33; 32];
const OP_SEED: [u8; 32] = [0x44; 32];

fn empty_envelope() -> ComplianceClaimEnvelope {
    ComplianceClaimEnvelope {
        signature: [0; 64],
        attester_pubkey: [1; 32],
        claim_bytes: Vec::new(),
        signing_alg: SigningAlg::Ed25519,
    }
}

fn keypair() -> (Ed25519KeyPair, [u8; 32]) {
    let seed = [0x55; 32];
    let keypair = Ed25519KeyPair::from_seed_unchecked(&seed).expect("deterministic test key");
    let public_key = keypair
        .public_key()
        .as_ref()
        .try_into()
        .expect("Ed25519 public key is 32 bytes");
    (keypair, public_key)
}

fn ed_pub(seed: &[u8; 32]) -> [u8; 32] {
    let keypair = Ed25519KeyPair::from_seed_unchecked(seed).expect("deterministic test key");
    keypair
        .public_key()
        .as_ref()
        .try_into()
        .expect("Ed25519 public key is 32 bytes")
}

fn class_manifest(form: &str, trust_tier: &str, schema: u32) -> String {
    let artifact = if form == "wasm-component" {
        "artifact = \"dist/spirit.wasm\"\n"
    } else {
        ""
    };
    format!(
        "[class]\nname = \"form-test\"\nversion = \"0.1.0\"\nabi = \"1.0\"\nmanifest_schema_version = {schema}\nmin_substrate_version = \"0.1.0\"\nforms = [\"{form}\"]\ntrust_tier = \"{trust_tier}\"\ndescription = \"form-admission fixture\"\n{artifact}"
    )
}

fn valid_envelope(
    manifest: &[u8],
    version: &str,
    keypair: &Ed25519KeyPair,
    public_key: [u8; 32],
) -> ComplianceClaimEnvelope {
    let manifest_hash: [u8; 32] = sha2::Sha256::digest(manifest).into();
    let fingerprint = ExecutionContextFingerprint {
        manifest_hash,
        spirit_version: version.into(),
        trust_tier: TrustTier::PublicUntrusted,
        sandbox_tier: SandboxTier::T0,
        capability_scope: BTreeSet::new(),
        provider_endpoint: ProviderEndpointPin {
            provider_id: String::new(),
            endpoint_url: String::new(),
            model_id: None,
        },
        crypto_provider: CryptoProviderId(String::new()),
    };
    let claim_bytes = serde_json::to_vec(&serde_json::json!({
        "fingerprint_hash": hex::encode(compute_fingerprint_hash(&fingerprint)),
        "trust_tier": "public_untrusted",
        "sandbox_tier": "t0",
        "capability_scope": [],
        "provider_endpoint": {"provider_id": "", "endpoint_url": ""},
        "crypto_provider": ""
    }))
    .expect("serialize deterministic claim");
    ComplianceClaimEnvelope {
        signature: keypair
            .sign(&claim_bytes)
            .as_ref()
            .try_into()
            .expect("Ed25519 signature is 64 bytes"),
        attester_pubkey: public_key,
        claim_bytes,
        signing_alg: SigningAlg::Ed25519,
    }
}

fn signed_package(manifest: String, envelope_is_valid: bool) -> SignedPackage {
    let (keypair, public_key) = keypair();
    let artifact = b"form-admission artifact".to_vec();
    let envelope = if envelope_is_valid {
        valid_envelope(manifest.as_bytes(), "0.1.0", &keypair, public_key)
    } else {
        empty_envelope()
    };
    let mut hasher = sha2::Sha256::new();
    hasher.update((manifest.len() as u64).to_le_bytes());
    hasher.update(manifest.as_bytes());
    hasher.update((artifact.len() as u64).to_le_bytes());
    hasher.update(&artifact);
    let signature = keypair.sign(&hasher.finalize());
    SignedPackage::new(
        SpiritId::from("form-test"),
        "0.1.0".into(),
        manifest.into_bytes(),
        artifact,
        signature
            .as_ref()
            .try_into()
            .expect("Ed25519 signature is 64 bytes"),
        public_key,
        envelope,
    )
}

fn cfg() -> AdmissionConfig {
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

fn enrolled_keyring() -> VetterKeyring {
    let mut keyring = VetterKeyring::new(ed_pub(&OP_SEED));
    keyring.push(issue_event(
        &OP_SEED,
        &VetterKeyEventClaim {
            kind: VetterKeyEventKind::Enroll,
            vetter_key_id: "vetter-01".into(),
            vetter_pubkey: ed_pub(&VETTER_SEED),
            predecessor_pubkey: None,
            effective_at_unix_ms: 100,
            journal_sequence: 1,
            journaled_at_unix_ms: 100,
            note: "form-admission fixture enrollment".into(),
        },
    ));
    keyring
}

fn attestation_for(pkg: &SignedPackage) -> VettingAttestation {
    let manifest_hash: [u8; 32] = sha2::Sha256::digest(&pkg.manifest_toml).into();
    issue_attestation(
        &VETTER_SEED,
        &VettingClaim {
            manifest_hash,
            spirit_id: pkg.spirit_id.as_str().into(),
            spirit_version: pkg.version.clone(),
            from_tier: TrustTier::PublicUntrusted,
            to_tier: TrustTier::PublicVetted,
            vetter_key_id: "vetter-01".into(),
            issued_at_unix_ms: 500,
            expires_at_unix_ms: 2_000,
            revocation_semantics: RevocationSemantics::RefuseAtNextLoad,
            successor_policy: None,
        },
    )
}

#[test]
fn rust_inproc_is_refused_before_every_admission_tier() {
    for tier in ["local", "org-internal", "public-untrusted"] {
        let err = admit_spirit(
            &signed_package(class_manifest("rust-inproc", tier, 4), true),
            &cfg(),
        )
        .expect_err("rust-inproc must be refused before tier-specific work");
        assert!(matches!(err, AdmissionError::FirstPartyFormFromRegistry));
    }

    let package = signed_package(class_manifest("rust-inproc", "public-vetted", 4), true);
    let attestation = attestation_for(&package);
    let err = admit_spirit_with_attestation(
        &package,
        &cfg(),
        Some(&attestation),
        &enrolled_keyring(),
        &ed_pub(&OP_SEED),
        1_000,
    )
    .expect_err("the valid attestation path must also refuse rust-inproc");
    assert!(matches!(err, AdmissionError::FirstPartyFormFromRegistry));
}

#[test]
fn wasm_component_uses_t2_at_each_admission_tier() {
    let local = signed_package(class_manifest("wasm-component", "local", 5), false);
    assert_eq!(
        admit_spirit(&local, &cfg())
            .expect("local wasm component admits")
            .sandbox_tier_floor,
        SandboxTier::T2
    );

    let org = signed_package(class_manifest("wasm-component", "org-internal", 5), false);
    let mut org_cfg = cfg();
    org_cfg.allow_unsigned_local = false;
    org_cfg.org_signing_pubkey = Some(org.publisher_pubkey);
    assert_eq!(
        admit_spirit(&org, &org_cfg)
            .expect("org wasm component admits with its configured key")
            .sandbox_tier_floor,
        SandboxTier::T2
    );

    let public = signed_package(
        class_manifest("wasm-component", "public-untrusted", 5),
        true,
    );
    assert_eq!(
        admit_spirit(&public, &cfg())
            .expect("public wasm component admits with a valid envelope")
            .sandbox_tier_floor,
        SandboxTier::T2
    );
    let mut t3_cfg = cfg();
    t3_cfg.t3_for_public_untrusted = true;
    assert_eq!(
        admit_spirit(&public, &t3_cfg)
            .expect("the public T3 flag does not change the wasm boundary")
            .sandbox_tier_floor,
        SandboxTier::T2
    );

    let vetted = signed_package(class_manifest("wasm-component", "public-vetted", 5), true);
    let attestation = attestation_for(&vetted);
    assert_eq!(
        admit_spirit_with_attestation(
            &vetted,
            &t3_cfg,
            Some(&attestation),
            &enrolled_keyring(),
            &ed_pub(&OP_SEED),
            1_000,
        )
        .expect("vetted wasm component admits with a valid attestation")
        .sandbox_tier_floor,
        SandboxTier::T2
    );
}

#[test]
fn structural_class_reads_have_the_named_typed_outcomes() {
    let invalid_tier = signed_package(class_manifest("subprocess", "public_untrusted", 4), false);
    assert!(matches!(
        admit_spirit(&invalid_tier, &cfg()),
        Err(AdmissionError::ClassSectionInvalid(_))
    ));

    let legacy = signed_package(
        "[spirit]\nname = \"form-test\"\nversion = \"0.1.0\"\ntrust_tier = \"local\"\n".into(),
        false,
    );
    assert_eq!(
        admit_spirit(&legacy, &cfg())
            .expect("legacy package behavior is unchanged")
            .sandbox_tier_floor,
        SandboxTier::T0
    );

    let class_in_string = signed_package(
        "note = '''[class]\nforms = [\"rust-inproc\"]\n'''\ntrust_tier = \"local\"\n".into(),
        false,
    );
    assert!(
        admit_spirit(&class_in_string, &cfg()).is_ok(),
        "a class-looking string is not a class table"
    );

    let dotted_class = signed_package(
        "class.name = \"form-test\"\nclass.version = \"0.1.0\"\nclass.abi = \"1.0\"\nclass.manifest_schema_version = 4\nclass.min_substrate_version = \"0.1.0\"\nclass.forms = [\"rust-inproc\"]\nclass.trust_tier = \"local\"\nclass.description = \"dotted form fixture\"\n".into(),
        false,
    );
    assert!(matches!(
        admit_spirit(&dotted_class, &cfg()),
        Err(AdmissionError::FirstPartyFormFromRegistry)
    ));

    let duplicate_class = signed_package("[class]\n[class]\n".into(), false);
    assert!(matches!(
        admit_spirit(&duplicate_class, &cfg()),
        Err(AdmissionError::ManifestTrustTierInvalid(_))
    ));
}

#[test]
fn structural_fkcs_reads_refuse_every_evasion_shape() {
    let manifest_prefix = "name = \"form-test\"\nversion = \"0.1.0\"\ntrust_tier = \"local\"\n";
    for suffix in [
        "[fkcs]\ninternal_references = [\n  \"maos_kernel_core::scheduler::pick_next_spirit_from_slice\",\n]\n",
        "[fkcs] # a comment must not hide this table\ninternal_references = [\"maos_kernel_core::scheduler::pick_next_spirit_from_slice\"]\n",
        "fkcs.internal_references = [\"maos_kernel_core::scheduler::pick_next_spirit_from_slice\"]\n",
        "[fkcs]\ninternal_references = [\"maos_kernel_core::scheduler::pick_next_spirit_from_slice\"]\n",
    ] {
        let package = signed_package(format!("{manifest_prefix}{suffix}"), false);
        assert!(matches!(
            admit_spirit(&package, &cfg()),
            Err(AdmissionError::OffFrozenSurface { .. })
        ));
    }

    let malformed_fkcs = signed_package(
        format!("{manifest_prefix}[fkcs]\ninternal_references = \"not an array\"\n"),
        false,
    );
    assert!(matches!(
        admit_spirit(&malformed_fkcs, &cfg()),
        Err(AdmissionError::ManifestTrustTierInvalid(_))
    ));

    let non_utf8 = SignedPackage::new(
        SpiritId::from("form-test"),
        "0.1.0".into(),
        vec![0xff],
        b"form-admission artifact".to_vec(),
        [0; 64],
        [0; 32],
        empty_envelope(),
    );
    assert!(matches!(
        admit_spirit(&non_utf8, &cfg()),
        Err(AdmissionError::ManifestTrustTierInvalid(_))
    ));
}
