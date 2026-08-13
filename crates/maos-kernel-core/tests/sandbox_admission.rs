//! Sandbox admission tests — strictest-of, T3 rejection, T1 rejection, tier journaling.

use std::sync::Arc;

use maos_domain::invariants::i10::LifecycleEvent;
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::CryptoProvider;
use maos_domain::sandbox::{ImageAttestationId, T3ImageAttestation, T3ImageEntry};
use maos_kernel_core::capability::cap_policy::{
    decision::TrustTier, ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_tokens;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::security::{
    CapabilitiesRequired, ClassSection, EpistemicPolicySection, OutputShape, PostureSection,
    ResourceCaps, RingCryptoProvider, SandboxConfig, SecurityError, SecurityManagerAdapter,
};
use ring::signature::{Ed25519KeyPair, KeyPair};

fn test_class() -> ClassSection {
    ClassSection {
        name: "test-spirit".into(),
        version: "0.1.0".into(),
        abi: "1.0".into(),
        manifest_schema_version: maos_spirit_abi::MANIFEST_SCHEMA_VERSION,
        min_substrate_version: "0.0.1".into(),
        forms: vec!["rust-inproc".into()],
        trust_tier: "local".into(),
        description: "test".into(),
    }
}

fn make_adapter_with_trust_floors() -> (SecurityManagerAdapter, JournalAdapter, tempfile::TempDir) {
    cap_tokens::init_monotonic_base();
    let policy = Arc::new(PolicyTable::new());
    let mut inner = PolicyTableInner::default();
    inner
        .trust_tier_floor
        .insert(TrustTier::PublicUntrusted, SandboxTier::T2);
    inner
        .trust_tier_floor
        .insert(TrustTier::Known, SandboxTier::T1);
    inner
        .trust_tier_floor
        .insert(TrustTier::Verified, SandboxTier::T0);
    inner
        .trust_tier_floor
        .insert(TrustTier::Internal, SandboxTier::T0);
    policy.update(inner);

    let adapter = SecurityManagerAdapter::new(policy);
    let tmpdir = tempfile::TempDir::new().unwrap();
    let journal = JournalAdapter::open(&tmpdir.path().join("journal.ndjson")).unwrap();
    (adapter, journal, tmpdir)
}

fn empty_caps_required() -> CapabilitiesRequired {
    CapabilitiesRequired {
        provider: maos_kernel_core::security::ProviderCapabilities { complete: vec![] },
        mcp: maos_kernel_core::security::manifest::McpCapabilities { servers: vec![] },
        loom: maos_kernel_core::security::manifest::LoomCapabilities::default(),
    }
}

fn hello_spirit_output_shape() -> OutputShape {
    OutputShape {
        required_fields: vec![
            "introduction".into(),
            "capability_scope".into(),
            "halt_tags".into(),
            "transparency_log".into(),
        ],
    }
}

fn default_posture_section() -> PostureSection {
    PostureSection::from_toml_str(
        r#"default = "assistive"
allowed_max = "assistive""#,
    )
    .unwrap()
}

fn default_epistemic_policy() -> EpistemicPolicySection {
    EpistemicPolicySection::default_open_fail()
}

fn write_signed_t3_lock(tmpdir: &tempfile::TempDir) -> (std::path::PathBuf, [u8; 32]) {
    let seed = [0x5A; 32];
    let keypair = Ed25519KeyPair::from_seed_unchecked(&seed).expect("T3 test keypair");
    let signer_pub_key: [u8; 32] = keypair
        .public_key()
        .as_ref()
        .try_into()
        .expect("Ed25519 public key");
    let entries = vec![T3ImageEntry::new(
        "registry.example/maos/t3-runtime",
        [0xA5; 32],
        "signed T3 admission fixture",
        true,
    )
    .expect("T3 image entry")];
    let signature: [u8; 64] = RingCryptoProvider
        .sign_capability_token(
            &seed,
            &serde_json::to_vec(&entries).expect("serialize T3 entries"),
        )
        .expect("sign T3 entries")
        .try_into()
        .expect("Ed25519 signature");
    let attestation = T3ImageAttestation::new(
        ImageAttestationId([0x3A; 32]),
        1,
        1_700_000_000_000_000_000,
        entries,
        signature,
        signer_pub_key,
    )
    .expect("T3 attestation");
    let path = tmpdir.path().join("t3-image.lock");
    std::fs::write(
        &path,
        serde_json::to_vec(&vec![attestation]).expect("serialize T3 lock"),
    )
    .expect("write T3 lock");
    (path, signer_pub_key)
}

#[test]
fn strictest_of_manifest_trust_operator() {
    let (adapter, journal, _tmpdir) = make_adapter_with_trust_floors();

    let mut inner = (*adapter.policy().inner().load_full()).clone();
    inner.manifest_scopes.insert(
        42,
        ManifestCapabilityScope {
            scopes: vec![],
            declared_tier: SandboxTier::T0,
            trust_tier: TrustTier::PublicUntrusted,
        },
    );
    adapter.policy().update(inner);

    let spec = adapter
        .admit_spirit(
            42,
            "spirit-42",
            &SandboxConfig {
                tier: SandboxTier::T0,
                image_pin: None,
            },
            &ResourceCaps::default(),
            &empty_caps_required(),
            None,
            &journal,
            &default_posture_section(),
            Some(&default_epistemic_policy()),
            None,
            None,
            None,
            None,
            None,
            Some(&test_class()), // Story 7.5a — class required at admit (EClassRequired)
        )
        .unwrap();

    assert_eq!(
        spec.tier,
        SandboxTier::T2,
        "PublicUntrusted must force T0→T2"
    );
    assert_eq!(spec.spirit_id, "spirit-42");
}

#[test]
fn t3_effective_tier_admitted() {
    let (adapter, journal, tmpdir) = make_adapter_with_trust_floors();
    let (lock_path, trust_anchor_pub) = write_signed_t3_lock(&tmpdir);
    let adapter = adapter.with_t3_image_verification(lock_path, trust_anchor_pub);

    let mut inner = (*adapter.policy().inner().load_full()).clone();
    inner.manifest_scopes.insert(
        99,
        ManifestCapabilityScope {
            scopes: vec![],
            declared_tier: SandboxTier::T0,
            trust_tier: TrustTier::Verified,
        },
    );
    inner
        .operator_policy
        .spirit_tier_floor
        .insert(99, SandboxTier::T3);
    adapter.policy().update(inner);

    let spec = adapter
        .admit_spirit(
            99,
            "spirit-99",
            &SandboxConfig {
                tier: SandboxTier::T0,
                image_pin: None,
            },
            &ResourceCaps::default(),
            &empty_caps_required(),
            None,
            &journal,
            &default_posture_section(),
            Some(&default_epistemic_policy()),
            None,
            None,
            None,
            None,
            None,
            Some(&test_class()), // Story 7.5a — class required at admit (EClassRequired)
        )
        .unwrap();

    assert_eq!(
        spec.tier,
        SandboxTier::T3,
        "T3 effective tier must be admitted (Story 5.5a)"
    );
    assert_eq!(spec.spirit_id, "spirit-99");
}

#[test]
fn t4_effective_tier_rejected() {
    let (adapter, journal, _tmpdir) = make_adapter_with_trust_floors();

    let mut inner = (*adapter.policy().inner().load_full()).clone();
    inner.manifest_scopes.insert(
        100,
        ManifestCapabilityScope {
            scopes: vec![],
            declared_tier: SandboxTier::T4,
            trust_tier: TrustTier::Verified,
        },
    );
    inner
        .trust_tier_floor
        .insert(TrustTier::Verified, SandboxTier::T0);
    inner.operator_policy.global_sandbox_floor = SandboxTier::T0;
    adapter.policy().update(inner);

    let err = adapter
        .admit_spirit(
            100,
            "spirit-100",
            &SandboxConfig {
                tier: SandboxTier::T4,
                image_pin: None,
            },
            &ResourceCaps::default(),
            &empty_caps_required(),
            None,
            &journal,
            &default_posture_section(),
            Some(&default_epistemic_policy()),
            None,
            None,
            None,
            None,
            None,
            Some(&test_class()), // Story 7.5a — class required at admit (EClassRequired)
        )
        .unwrap_err();

    assert!(
        matches!(err, SecurityError::SandboxTierUnsupported(SandboxTier::T4)),
        "T4 effective tier must still be rejected: got {err}"
    );
}

#[test]
fn t1_effective_tier_rejected() {
    let (adapter, journal, _tmpdir) = make_adapter_with_trust_floors();

    let mut inner = (*adapter.policy().inner().load_full()).clone();
    inner.manifest_scopes.insert(
        55,
        ManifestCapabilityScope {
            scopes: vec![],
            declared_tier: SandboxTier::T1,
            trust_tier: TrustTier::Verified,
        },
    );
    inner
        .trust_tier_floor
        .insert(TrustTier::Verified, SandboxTier::T1);
    inner.operator_policy.global_sandbox_floor = SandboxTier::T0;
    adapter.policy().update(inner);

    let err = adapter
        .admit_spirit(
            55,
            "spirit-55",
            &SandboxConfig {
                tier: SandboxTier::T1,
                image_pin: None,
            },
            &ResourceCaps::default(),
            &empty_caps_required(),
            None,
            &journal,
            &default_posture_section(),
            Some(&default_epistemic_policy()),
            None,
            None,
            None,
            None,
            None,
            Some(&test_class()), // Story 7.5a — class required at admit (EClassRequired)
        )
        .unwrap_err();

    assert!(
        matches!(err, SecurityError::SandboxTierUnsupported(SandboxTier::T1)),
        "T1 enforcement not implemented — must fail-closed: got {err}"
    );
}

#[test]
fn effective_tier_is_journaled() {
    let (adapter, journal, _tmpdir) = make_adapter_with_trust_floors();

    let mut inner = (*adapter.policy().inner().load_full()).clone();
    inner.manifest_scopes.insert(
        7,
        ManifestCapabilityScope {
            scopes: vec![],
            declared_tier: SandboxTier::T2,
            trust_tier: TrustTier::Verified,
        },
    );
    adapter.policy().update(inner);

    let spec = adapter
        .admit_spirit(
            7,
            "spirit-seven",
            &SandboxConfig {
                tier: SandboxTier::T2,
                image_pin: None,
            },
            &ResourceCaps::default(),
            &empty_caps_required(),
            None,
            &journal,
            &default_posture_section(),
            Some(&default_epistemic_policy()),
            None,
            None,
            None,
            None,
            None,
            Some(&test_class()), // Story 7.5a — class required at admit (EClassRequired)
        )
        .unwrap();

    assert_eq!(spec.tier, SandboxTier::T2);

    let last = journal.last_event("spirit-seven").unwrap();
    assert!(
        matches!(last, LifecycleEvent::Load),
        "must journal Load event"
    );

    let recovered = journal.recover_in_flight();
    let entry_for_spirit = recovered.iter().find(|(id, _)| id == "spirit-seven");
    assert!(
        entry_for_spirit.is_some(),
        "spirit-seven must appear in recovered journal"
    );
    assert!(matches!(entry_for_spirit.unwrap().1, LifecycleEvent::Load));
}
