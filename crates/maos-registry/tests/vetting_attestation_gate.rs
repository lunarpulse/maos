//! Story 13.4 (FR37 / ADR-056) — `check-vetting-attestation` gate backing tests.
//!
//! Each test is an independently-named, hermetic leg that reds on its own
//! defect (the anti-null discipline of AC6 — a gate that verifies a signature it
//! also computed is a null control with a compliance badge). The `xtask`
//! `check-vetting-attestation` gate runs each by `--exact` name. Leg 6 (inverted
//! `e2e_public_vetted_always_rejected`) lives in `end_to_end_test.rs`.

use maos_compliance::vetting::keyring::issue_event;
use maos_compliance::{
    issue_attestation, observe_running_spirit, RevocationSemantics, RunningSpiritObservation,
    TerminalInputs, TerminalObservationSink, VetterKeyEventClaim, VetterKeyEventKind,
    VetterKeyring, VettingAttestation, VettingClaim, VettingTerminalCause,
};
use maos_domain::ports::registry::{SignedPackage, SpiritId, TrustTier};
use maos_spirit_abi::compliance::{
    ComplianceClaimEnvelope, CryptoProviderId, ExecutionContextFingerprint, ProviderEndpointPin,
    SandboxTier, SigningAlg,
};

use maos_registry::admission::{admit_spirit_with_attestation, AdmissionConfig, AdmissionError};

const VETTER_SEED: [u8; 32] = [0x33; 32];
const OP_SEED: [u8; 32] = [0x44; 32];

#[derive(Default)]
struct RecordingTerminalSink(parking_lot::Mutex<Vec<RunningSpiritObservation>>);

impl TerminalObservationSink for RecordingTerminalSink {
    fn journal_terminal_observation(&self, observation: &RunningSpiritObservation) {
        self.0.lock().push(observation.clone());
    }
}

fn ed_pub(seed: &[u8; 32]) -> [u8; 32] {
    use ring::signature::{Ed25519KeyPair, KeyPair};
    let kp = Ed25519KeyPair::from_seed_unchecked(seed).unwrap();
    kp.public_key().as_ref().try_into().unwrap()
}

fn signed_vetted_pkg(spirit_id: &str, version: &str) -> SignedPackage {
    use ring::signature::Ed25519KeyPair;
    use sha2::Digest;

    // Deterministic publisher keypair.
    let pub_seed = [0x55u8; 32];
    let kp = Ed25519KeyPair::from_seed_unchecked(&pub_seed).unwrap();
    let pubkey: [u8; 32] = {
        use ring::signature::KeyPair;
        kp.public_key().as_ref().try_into().unwrap()
    };

    let manifest = format!(
        "[spirit]\nname = \"{spirit_id}\"\nversion = \"{version}\"\ntrust_tier = \"public-vetted\"\n"
    );
    let artifact = b"vetted-binary".to_vec();

    let manifest_bytes = manifest.as_bytes();
    let mut hasher = sha2::Sha256::new();
    hasher.update((manifest_bytes.len() as u64).to_le_bytes());
    hasher.update(manifest_bytes);
    hasher.update((artifact.len() as u64).to_le_bytes());
    hasher.update(&artifact);
    let msg = hasher.finalize();
    let sig: [u8; 64] = kp.sign(&msg).as_ref().try_into().unwrap();

    let manifest_hash: [u8; 32] = sha2::Sha256::digest(manifest_bytes).into();
    let fingerprint = ExecutionContextFingerprint {
        manifest_hash,
        spirit_version: version.into(),
        trust_tier: TrustTier::PublicUntrusted,
        sandbox_tier: SandboxTier::T0,
        capability_scope: std::collections::BTreeSet::new(),
        provider_endpoint: ProviderEndpointPin {
            provider_id: String::new(),
            endpoint_url: String::new(),
            model_id: None,
        },
        crypto_provider: CryptoProviderId(String::new()),
    };
    let claim_bytes = serde_json::to_vec(&serde_json::json!({
        "fingerprint_hash": hex::encode(
            maos_registry::compliance_verify::compute_fingerprint_hash(&fingerprint)
        ),
        "trust_tier": "public_untrusted",
        "sandbox_tier": "t0",
        "capability_scope": [],
        "provider_endpoint": {"provider_id": "", "endpoint_url": ""},
        "crypto_provider": ""
    }))
    .unwrap();
    let compliance_envelope = ComplianceClaimEnvelope {
        signature: kp.sign(&claim_bytes).as_ref().try_into().unwrap(),
        attester_pubkey: pubkey,
        claim_bytes,
        signing_alg: SigningAlg::Ed25519,
    };

    SignedPackage::new(
        SpiritId::from(spirit_id),
        version.into(),
        manifest.into_bytes(),
        artifact,
        sig,
        pubkey,
        compliance_envelope,
    )
}

fn manifest_hash_of(pkg: &SignedPackage) -> [u8; 32] {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(&pkg.manifest_toml);
    h.finalize().into()
}

fn enrolled_keyring(enrolled_at: u64) -> VetterKeyring {
    let mut kr = VetterKeyring::new(ed_pub(&OP_SEED));
    kr.push(issue_event(
        &OP_SEED,
        &VetterKeyEventClaim {
            kind: VetterKeyEventKind::Enroll,
            vetter_key_id: "vetter-01".into(),
            vetter_pubkey: ed_pub(&VETTER_SEED),
            predecessor_pubkey: None,
            effective_at_unix_ms: enrolled_at,
            journal_sequence: 1,
            journaled_at_unix_ms: enrolled_at,
            note: "enrolled".into(),
        },
    ));
    kr
}

fn revoke_attestation_in(kr: &mut VetterKeyring, pkg: &SignedPackage, effective_at: u64) {
    use maos_domain::revocation::{CrlId, RevocationEntry, RevocationOrigin, SignedRevocationList};
    use ring::signature::Ed25519KeyPair;

    let entries = vec![RevocationEntry::new(
        pkg.spirit_id.as_str(),
        pkg.version.clone(),
        "vetting withdrawn",
        None,
    )
    .unwrap()];
    let entries_bytes = serde_json::to_vec(&entries).unwrap();
    let operator = Ed25519KeyPair::from_seed_unchecked(&OP_SEED).unwrap();
    let signature = operator.sign(&entries_bytes).as_ref().try_into().unwrap();
    kr.push_attestation_revocation(
        SignedRevocationList::new(
            CrlId([0xA5; 32]),
            1,
            effective_at * 1_000_000,
            RevocationOrigin::Operator,
            entries,
            signature,
            ed_pub(&OP_SEED),
        )
        .unwrap(),
    );
}

fn attestation_for(pkg: &SignedPackage, issued: u64, expires: u64) -> VettingAttestation {
    let claim = VettingClaim {
        manifest_hash: manifest_hash_of(pkg),
        spirit_id: pkg.spirit_id.as_str().to_string(),
        spirit_version: pkg.version.clone(),
        from_tier: TrustTier::PublicUntrusted,
        to_tier: TrustTier::PublicVetted,
        vetter_key_id: "vetter-01".into(),
        issued_at_unix_ms: issued,
        expires_at_unix_ms: expires,
        revocation_semantics: RevocationSemantics::RefuseAtNextLoad,
        successor_policy: None,
    };
    issue_attestation(&VETTER_SEED, &claim)
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

/// Leg 1 — full lifecycle issue → install/promote → revoke, with the verifier
/// (`verify_attestation`, an independent CBOR decode) NOT the issue codec.
#[test]
fn leg1_round_trip_issue_promote_revoke() {
    let pkg = signed_vetted_pkg("vetted", "0.1.0");
    let att = attestation_for(&pkg, 500, 2_000);

    // issue → install/promote: admits.
    let mut kr = enrolled_keyring(100);
    let decision =
        admit_spirit_with_attestation(&pkg, &cfg(), Some(&att), &kr, &ed_pub(&OP_SEED), 1_000)
            .unwrap();
    assert!(decision.admit);
    assert_eq!(decision.effective_tier, TrustTier::PublicVetted);

    // revoke: the SAME attestation is now refused at the next load.
    revoke_attestation_in(&mut kr, &pkg, 900);
    let err =
        admit_spirit_with_attestation(&pkg, &cfg(), Some(&att), &kr, &ed_pub(&OP_SEED), 1_000)
            .unwrap_err();
    assert!(matches!(
        err,
        AdmissionError::VettingAttestationRejected(cause)
            if cause.contains("revoked by signed revocation list")
    ));
}

/// Leg 2 — a forged/tampered attestation signature is refused. Reds if the
/// signature check is removed.
#[test]
fn leg2_forged_signature_refused() {
    let pkg = signed_vetted_pkg("vetted", "0.1.0");
    let mut att = attestation_for(&pkg, 500, 2_000);
    att.signature[0] ^= 0xFF;
    let kr = enrolled_keyring(100);
    let err =
        admit_spirit_with_attestation(&pkg, &cfg(), Some(&att), &kr, &ed_pub(&OP_SEED), 1_000)
            .unwrap_err();
    assert!(matches!(err, AdmissionError::VettingAttestationRejected(_)));
}

/// Leg 3 — an expired attestation is refused. Reds if the expiry check is removed.
#[test]
fn leg3_expired_attestation_refused() {
    let pkg = signed_vetted_pkg("vetted", "0.1.0");
    let att = attestation_for(&pkg, 500, 900); // expires before now=1000
    let kr = enrolled_keyring(100);
    let err =
        admit_spirit_with_attestation(&pkg, &cfg(), Some(&att), &kr, &ed_pub(&OP_SEED), 1_000)
            .unwrap_err();
    assert!(matches!(err, AdmissionError::VettingAttestationRejected(_)));
}

/// Leg 4 — the 3am case: a structurally VALID signature from an un-enrolled
/// vetter key is refused (the enrollment-predating-issuance walk fires). Reds if
/// the chain walk is skipped.
#[test]
fn leg4_forged_vetter_key_refused() {
    let pkg = signed_vetted_pkg("vetted", "0.1.0");
    // Signature is valid (issued with a real key) but the key is never enrolled.
    let att = attestation_for(&pkg, 500, 2_000);
    let empty_kr = VetterKeyring::new(ed_pub(&OP_SEED));
    let err = admit_spirit_with_attestation(
        &pkg,
        &cfg(),
        Some(&att),
        &empty_kr,
        &ed_pub(&OP_SEED),
        1_000,
    )
    .unwrap_err();
    assert!(matches!(err, AdmissionError::VettingAttestationRejected(_)));
}

/// Leg 5 — upgrade-flap control (negative + positive). A new manifest version
/// without its own attestation is refused at the floor; the same version WITH a
/// valid attestation is admitted (so the leg isn't reject-everything).
#[test]
fn leg5_upgrade_flap_control() {
    let kr = enrolled_keyring(100);
    let v2 = signed_vetted_pkg("vetted", "0.2.0");

    // Negative isolates exact-hash binding: identity/version match v0.2.0, but
    // the signed hash still binds the old v0.1.0 manifest.
    let v1 = signed_vetted_pkg("vetted", "0.1.0");
    let old_att = issue_attestation(
        &VETTER_SEED,
        &VettingClaim {
            manifest_hash: manifest_hash_of(&v1),
            spirit_id: v2.spirit_id.as_str().into(),
            spirit_version: v2.version.clone(),
            from_tier: TrustTier::PublicUntrusted,
            to_tier: TrustTier::PublicVetted,
            vetter_key_id: "vetter-01".into(),
            issued_at_unix_ms: 500,
            expires_at_unix_ms: 2_000,
            revocation_semantics: RevocationSemantics::RefuseAtNextLoad,
            successor_policy: None,
        },
    );
    let err =
        admit_spirit_with_attestation(&v2, &cfg(), Some(&old_att), &kr, &ed_pub(&OP_SEED), 1_000)
            .unwrap_err();
    assert!(matches!(
        err,
        AdmissionError::VettingAttestationRejected(cause)
            if cause.contains("exact-hash mismatch")
    ));

    // Negative: no attestation at all → deferred.
    let err_none = admit_spirit_with_attestation(&v2, &cfg(), None, &kr, &ed_pub(&OP_SEED), 1_000)
        .unwrap_err();
    assert!(matches!(err_none, AdmissionError::PublicVettedDeferred));

    // Positive: v0.2.0 WITH its own attestation is admitted.
    let new_att = attestation_for(&v2, 500, 2_000);
    let decision =
        admit_spirit_with_attestation(&v2, &cfg(), Some(&new_att), &kr, &ed_pub(&OP_SEED), 1_000)
            .unwrap();
    assert!(decision.admit);
    assert_eq!(decision.effective_tier, TrustTier::PublicVetted);
}

/// Leg 7 — the four terminal causes are provably distinct; a planted mislabel
/// (a revocation classified as expiry, a yank as operator-local) reds.
#[test]
fn leg7_four_cause_distinguishability() {
    let sink = RecordingTerminalSink::default();
    let inputs = [
        TerminalInputs {
            vetting_revoked: true,
            ..Default::default()
        },
        TerminalInputs {
            attestation_expired: true,
            ..Default::default()
        },
        TerminalInputs {
            registry_yanked: true,
            ..Default::default()
        },
        TerminalInputs {
            operator_local_disable: true,
            ..Default::default()
        },
    ];
    for input in &inputs {
        observe_running_spirit("vetted", "0.1.0", Some(&[0xAB; 32]), input, 1_000, &sink)
            .expect("terminal input must produce an audit observation");
    }

    let observations = sink.0.lock();
    let labels: Vec<_> = observations
        .iter()
        .map(|observation| observation.cause.audit_label())
        .collect();
    assert_eq!(
        labels,
        [
            "vetting-revocation",
            "expiry-lapse",
            "registry-yank",
            "operator-local",
        ]
    );
    assert!(observations.iter().all(|observation| observation
        .journal_note()
        .contains(observation.cause.audit_label())));
    assert_eq!(
        observations[0].cause,
        VettingTerminalCause::VettingRevocation
    );
    assert_ne!(observations[2].cause.audit_label(), "operator-local");
}
