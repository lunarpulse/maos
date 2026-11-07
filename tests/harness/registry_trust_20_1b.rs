//! Deterministic test-only signing inputs; never spawns cargo.
#![allow(dead_code)]
use maos_compliance::{VetterKeyEventClaim, VetterKeyEventKind, VetterKeyring, VettingClaim};
use maos_domain::ports::registry::{SignedPackage, SpiritId, TrustTier, YankOrigin};
use ring::signature::{Ed25519KeyPair, KeyPair};
use sha2::{Digest, Sha256};
pub const PUBLISHER: [u8; 32] = [0x53; 32];
pub const OPERATOR: [u8; 32] = [0x44; 32];
pub const VETTER: [u8; 32] = [0x33; 32];
pub fn public(seed: &[u8; 32]) -> [u8; 32] {
    Ed25519KeyPair::from_seed_unchecked(seed)
        .unwrap()
        .public_key()
        .as_ref()
        .try_into()
        .unwrap()
}
pub fn package(name: &str, version: &str, seed: &[u8; 32]) -> SignedPackage {
    let envelope = maos_spirit_abi::compliance::ComplianceClaimEnvelope {
        signature: [0; 64],
        attester_pubkey: [0; 32],
        claim_bytes: vec![],
        signing_alg: maos_spirit_abi::compliance::SigningAlg::Ed25519,
    };
    let mut pkg = SignedPackage::new(
        SpiritId::from(name),
        version.into(),
        format!(
            "[spirit]\nname = {name:?}\nversion = {version:?}\ntrust_tier = \"public-untrusted\"\n"
        )
        .into_bytes(),
        b"test-component".to_vec(),
        [0; 64],
        public(seed),
        envelope,
    );
    sign(&mut pkg, seed);
    pkg
}
pub fn sign(pkg: &mut SignedPackage, seed: &[u8; 32]) {
    let key = Ed25519KeyPair::from_seed_unchecked(seed).unwrap();
    pkg.publisher_pubkey = public(seed);
    let fields =
        maos_compliance::runtime_context::extract_manifest_fingerprint_fields(&pkg.manifest_toml);
    let tier = if fields.trust_tier == TrustTier::PublicVetted {
        TrustTier::PublicUntrusted
    } else {
        fields.trust_tier
    };
    let context = maos_compliance::RuntimeExecutionContext::from_admission(
        tier,
        fields.sandbox_tier,
        pkg,
        &fields.provider_endpoint,
        &fields.crypto_provider,
    );
    pkg.compliance_envelope = maos_compliance::builder::build_self_attested_envelope(
        &context.to_fingerprint(),
        &key,
        pkg.publisher_pubkey,
    );
    let mut hash = Sha256::new();
    hash.update((pkg.manifest_toml.len() as u64).to_le_bytes());
    hash.update(&pkg.manifest_toml);
    hash.update((pkg.artifact_bytes.len() as u64).to_le_bytes());
    hash.update(&pkg.artifact_bytes);
    pkg.signature = key.sign(&hash.finalize()).as_ref().try_into().unwrap();
}
pub fn keyring() -> VetterKeyring {
    let mut keyring = VetterKeyring::new(public(&OPERATOR));
    keyring.push(maos_compliance::vetting::keyring::issue_event(
        &OPERATOR,
        &VetterKeyEventClaim {
            kind: VetterKeyEventKind::Enroll,
            vetter_key_id: "vetter".into(),
            vetter_pubkey: public(&VETTER),
            predecessor_pubkey: None,
            effective_at_unix_ms: 1,
            journal_sequence: 1,
            journaled_at_unix_ms: 1,
            note: "test-only enrollment".into(),
        },
    ));
    keyring
}
pub fn attestation(
    pkg: &SignedPackage,
    issued: u64,
    expires: u64,
) -> maos_compliance::VettingAttestation {
    maos_compliance::issue_attestation(
        &VETTER,
        &VettingClaim {
            manifest_hash: Sha256::digest(&pkg.manifest_toml).into(),
            artifact_sha256: Sha256::digest(&pkg.artifact_bytes).into(),
            publisher_pubkey: pkg.publisher_pubkey,
            spirit_id: pkg.spirit_id.as_str().into(),
            spirit_version: pkg.version.clone(),
            from_tier: TrustTier::PublicUntrusted,
            to_tier: TrustTier::PublicVetted,
            vetter_key_id: "vetter".into(),
            issued_at_unix_ms: issued,
            expires_at_unix_ms: expires,
            revocation_semantics: maos_compliance::RevocationSemantics::RefuseAtNextLoad,
            successor_policy: None,
        },
    )
}
pub fn yank(
    addr: &str,
    name: &str,
    version: &str,
    seed: &[u8; 32],
    origin: YankOrigin,
) -> serde_json::Value {
    let reason = "withdrawn";
    let auth = maos_registry::yank_auth::YankAuthorization::sign(
        seed,
        maos_registry::yank_auth::YankScope {
            name,
            version,
            reason,
            registry_uri: &format!("http://{addr}/mcp"),
        },
        origin,
        "vetter".into(),
        maos_registry::yank_auth::now_unix_ms().unwrap(),
    )
    .unwrap();
    serde_json::json!({"spirit_id":name,"version":version,"reason":reason,"authorization":auth})
}
