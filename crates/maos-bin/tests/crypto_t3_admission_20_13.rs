#![cfg(feature = "crypto-recording")]
//! Story 20-13 AC7.2, bin leg: the provider the daemon's own selector returns
//! (`maos_bin::crypto_provider::select`, the value `main.rs` hands to
//! `SecurityManagerAdapter::new`) must be the instance that verifies the T3
//! image lock inside the kernel's admission arm. The recording provider's
//! identity log names `pid:instance` per call, so "a recording" is not enough:
//! every kernel-arm `verify_signature` must carry the selected instance's id.
//!
//! Not a daemon run: nothing in the shipped composition root raises a Spirit's
//! effective tier above T2 (no writer of the operator or trust-tier floor), so a
//! booted daemon never reaches the kernel T3 arm. This test therefore raises the
//! per-Spirit floor itself, exactly as the kernel witness does.

use std::sync::Arc;

use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::CryptoProvider;
use maos_domain::sandbox::{ImageAttestationId, T3ImageAttestation, T3ImageEntry};
use maos_kernel_core::capability::cap_policy::{
    decision::TrustTier, ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::security::{
    CapabilitiesRequired, ClassSection, EpistemicPolicySection, PostureSection, ResourceCaps,
    RingCryptoProvider, SandboxConfig, SecurityError, SecurityManagerAdapter,
};
use ring::signature::{Ed25519KeyPair, KeyPair};

const PID: u32 = 99;

fn write_lock(dir: &std::path::Path, name: &str, tamper: bool) -> ([u8; 32], std::path::PathBuf) {
    let seed = [0x5A; 32];
    let public: [u8; 32] = Ed25519KeyPair::from_seed_unchecked(&seed)
        .unwrap()
        .public_key()
        .as_ref()
        .try_into()
        .unwrap();
    let entries =
        vec![T3ImageEntry::new("registry.example/maos/t3", [0xA5; 32], "fixture", true).unwrap()];
    let mut signature: [u8; 64] = RingCryptoProvider
        .sign_capability_token(&seed, &serde_json::to_vec(&entries).unwrap())
        .unwrap()
        .try_into()
        .unwrap();
    signature[0] ^= u8::from(tamper);
    let lock = T3ImageAttestation::new(
        ImageAttestationId([0x3A; 32]),
        1,
        1,
        entries,
        signature,
        public,
    )
    .unwrap();
    let path = dir.join(name);
    std::fs::write(&path, serde_json::to_vec(&vec![lock]).unwrap()).unwrap();
    (public, path)
}

fn t3_policy() -> Arc<PolicyTable> {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let policy = Arc::new(PolicyTable::new());
    let mut inner = PolicyTableInner::default();
    inner
        .trust_tier_floor
        .insert(TrustTier::Verified, SandboxTier::T0);
    inner.manifest_scopes.insert(
        PID,
        ManifestCapabilityScope {
            scopes: vec![],
            declared_tier: SandboxTier::T0,
            trust_tier: TrustTier::Verified,
        },
    );
    inner
        .operator_policy
        .spirit_tier_floor
        .insert(PID, SandboxTier::T3);
    policy.update(inner);
    policy
}

fn admit(
    adapter: &SecurityManagerAdapter,
    dir: &std::path::Path,
) -> Result<SandboxTier, SecurityError> {
    let caps = CapabilitiesRequired {
        provider: maos_kernel_core::security::ProviderCapabilities { complete: vec![] },
        mcp: maos_kernel_core::security::manifest::McpCapabilities { servers: vec![] },
        loom: Default::default(),
        iac: Default::default(),
        log: Default::default(),
        skill: Default::default(),
    };
    let class = ClassSection {
        name: "t3-spirit".into(),
        version: "0.1.0".into(),
        abi: "1.0".into(),
        manifest_schema_version: maos_spirit_abi::MANIFEST_SCHEMA_VERSION,
        min_substrate_version: "0.0.1".into(),
        forms: vec!["rust-inproc".into()],
        trust_tier: "local".into(),
        description: "t3".into(),
        artifact: None,
    };
    let posture =
        PostureSection::from_toml_str("default = \"assistive\"\nallowed_max = \"assistive\"")
            .unwrap();
    let journal =
        JournalAdapter::open(&dir.join(format!("journal-{}.ndjson", std::process::id()))).unwrap();
    adapter
        .admit_spirit(
            PID,
            "spirit-99",
            &SandboxConfig {
                tier: SandboxTier::T0,
                image_pin: None,
            },
            &ResourceCaps::default(),
            &caps,
            None,
            &journal,
            &posture,
            Some(&EpistemicPolicySection::default_open_fail()),
            None,
            None,
            None,
            None,
            None,
            Some(&class),
        )
        .map(|spec| spec.tier)
}

/// `pid:instance` of every `verify_signature` row, in order.
fn verify_rows(home: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(home.join("crypto-recording-identity.log"))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.strip_suffix(" verify_signature"))
        .map(str::to_owned)
        .collect()
}

#[test]
fn selected_recording_instance_verifies_the_kernel_t3_lock() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("maos");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        home.join("config.toml"),
        "[crypto]\nprovider = \"recording\"\n",
    )
    .unwrap();
    let selected = maos_bin::crypto_provider::select(Some(&home)).unwrap();
    assert_eq!(selected.name, "recording");
    // Probe: one call on the selected instance names its `pid:instance` identity.
    let _ = selected
        .provider
        .verify_signature(&[0; 32], b"probe", &[0; 64]);
    let probe = verify_rows(&home);
    assert_eq!(probe.len(), 1, "{probe:?}");
    // A second `select` yields a different instance; it must never be the verifier.
    let decoy = maos_bin::crypto_provider::select(Some(&home)).unwrap();
    let _ = decoy
        .provider
        .verify_signature(&[0; 32], b"decoy", &[0; 64]);
    assert_ne!(
        verify_rows(&home)[1],
        probe[0],
        "decoy must be a distinct instance"
    );

    // The daemon's no-config branch: lock path and anchor come from the environment.
    let (anchor, lock) = write_lock(dir.path(), "good.lock", false);
    std::env::set_var("MAOS_T3_IMAGE_LOCK_PATH", &lock);
    std::env::set_var("MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX", hex::encode(anchor));
    // Exactly the wiring of `main.rs`: policy and the selected provider, nothing else.
    let adapter = SecurityManagerAdapter::new(t3_policy(), Arc::clone(&selected.provider));
    assert_eq!(admit(&adapter, dir.path()).unwrap(), SandboxTier::T3);
    let rows = verify_rows(&home);
    assert_eq!(
        rows.len(),
        3,
        "kernel T3 admission must verify exactly once: {rows:?}"
    );
    assert_eq!(
        rows[2], probe[0],
        "the kernel arm verified on a different instance than the selected one"
    );

    // Negative control: a tampered lock is refused, and the refusal also ran on the selected instance.
    let (_, tampered) = write_lock(dir.path(), "bad.lock", true);
    std::env::set_var("MAOS_T3_IMAGE_LOCK_PATH", &tampered);
    let adapter = SecurityManagerAdapter::new(t3_policy(), Arc::clone(&selected.provider));
    assert!(matches!(
        admit(&adapter, dir.path()),
        Err(SecurityError::T3AdmissionFailed(_))
    ));
    let rows = verify_rows(&home);
    assert_eq!(
        (rows.len(), rows[3].as_str()),
        (4, probe[0].as_str()),
        "{rows:?}"
    );
}
