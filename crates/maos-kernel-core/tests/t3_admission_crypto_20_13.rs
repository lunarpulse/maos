//! Story 20-13 AC7 — kernel T3 admission verifies the image lock with the
//! composition root's selected `CryptoProvider`, never an inline `ring`.
//!
//! A genuinely signed lock and trust anchor are valid under `ring`; an
//! injected provider that rejects every signature must therefore turn the
//! admission into `T3AdmissionFailed`, and a recording provider must observe
//! the verification on the very instance the adapter was built with. Both the
//! explicit-lock branch (`with_t3_image_verification`) and the environment
//! branch (`MAOS_T3_IMAGE_LOCK_PATH` + `MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX`)
//! are covered.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use maos_domain::invariants::i10::LifecycleEvent;
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::CryptoError;
use maos_domain::ports::CryptoProvider;
use maos_domain::sandbox::{ImageAttestationId, T3ImageAttestation, T3ImageEntry};
use maos_kernel_core::capability::cap_policy::{
    decision::TrustTier, ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_tokens;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::security::{
    CapabilitiesRequired, ClassSection, EpistemicPolicySection, PostureSection, ResourceCaps,
    RingCryptoProvider, SandboxConfig, SecurityError, SecurityManagerAdapter,
};
use ring::signature::{Ed25519KeyPair, KeyPair};

const SPIRIT_PID: u32 = 99;
const SPIRIT_ID: &str = "spirit-99";

/// The one construction site of the adapter under test.
fn adapter_with(
    policy: Arc<PolicyTable>,
    crypto: Arc<dyn CryptoProvider>,
) -> SecurityManagerAdapter {
    SecurityManagerAdapter::new(policy, crypto)
}

/// Accepts nothing: every signature verification is refused.
struct RejectingProvider;

impl CryptoProvider for RejectingProvider {
    fn verify_signature(&self, _: &[u8], _: &[u8], _: &[u8]) -> Result<(), CryptoError> {
        Err(CryptoError::SignatureInvalid)
    }
    fn seal_for_export(
        &self,
        key: &[u8],
        nonce: &[u8],
        aad: &[u8],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        RingCryptoProvider.seal_for_export(key, nonce, aad, plaintext)
    }
    fn sign_capability_token(&self, key: &[u8], token: &[u8]) -> Result<Vec<u8>, CryptoError> {
        RingCryptoProvider.sign_capability_token(key, token)
    }
}

/// Delegates to `ring` and counts `verify_signature` calls on this instance.
#[derive(Default)]
struct CountingProvider {
    verifies: AtomicUsize,
}

impl CryptoProvider for CountingProvider {
    fn verify_signature(&self, pk: &[u8], msg: &[u8], sig: &[u8]) -> Result<(), CryptoError> {
        self.verifies.fetch_add(1, Ordering::SeqCst);
        RingCryptoProvider.verify_signature(pk, msg, sig)
    }
    fn seal_for_export(
        &self,
        key: &[u8],
        nonce: &[u8],
        aad: &[u8],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        RingCryptoProvider.seal_for_export(key, nonce, aad, plaintext)
    }
    fn sign_capability_token(&self, key: &[u8], token: &[u8]) -> Result<Vec<u8>, CryptoError> {
        RingCryptoProvider.sign_capability_token(key, token)
    }
}

fn class() -> ClassSection {
    ClassSection {
        name: "test-spirit".into(),
        version: "0.1.0".into(),
        abi: "1.0".into(),
        manifest_schema_version: maos_spirit_abi::MANIFEST_SCHEMA_VERSION,
        min_substrate_version: "0.0.1".into(),
        forms: vec!["rust-inproc".into()],
        trust_tier: "local".into(),
        description: "test".into(),
        artifact: None,
    }
}

fn caps_required() -> CapabilitiesRequired {
    CapabilitiesRequired {
        provider: maos_kernel_core::security::ProviderCapabilities { complete: vec![] },
        mcp: maos_kernel_core::security::manifest::McpCapabilities { servers: vec![] },
        loom: maos_kernel_core::security::manifest::LoomCapabilities::default(),
        iac: maos_kernel_core::security::manifest::IacCapabilities::default(),
        log: maos_manifest::LogCapabilities::default(),
        skill: maos_manifest::SkillCapabilities::default(),
    }
}

/// Write a lock signed by a fresh genuine Ed25519 key; returns the path and the
/// matching trust anchor. `tamper` flips one signature bit after signing.
fn write_signed_lock(dir: &tempfile::TempDir, tamper: bool) -> (std::path::PathBuf, [u8; 32]) {
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
    let mut signature: [u8; 64] = RingCryptoProvider
        .sign_capability_token(
            &seed,
            &serde_json::to_vec(&entries).expect("serialize T3 entries"),
        )
        .expect("sign T3 entries")
        .try_into()
        .expect("Ed25519 signature");
    if tamper {
        signature[0] ^= 0x01;
    }
    let attestation = T3ImageAttestation::new(
        ImageAttestationId([0x3A; 32]),
        1,
        1_700_000_000_000_000_000,
        entries,
        signature,
        signer_pub_key,
    )
    .expect("T3 attestation");
    let path = dir.path().join("t3-image.lock");
    std::fs::write(
        &path,
        serde_json::to_vec(&vec![attestation]).expect("serialize T3 lock"),
    )
    .expect("write T3 lock");
    (path, signer_pub_key)
}

/// A policy whose operator floor forces `SPIRIT_PID` onto T3.
fn t3_policy() -> Arc<PolicyTable> {
    cap_tokens::init_monotonic_base();
    let policy = Arc::new(PolicyTable::new());
    let mut inner = PolicyTableInner::default();
    inner
        .trust_tier_floor
        .insert(TrustTier::Verified, SandboxTier::T0);
    inner.manifest_scopes.insert(
        SPIRIT_PID,
        ManifestCapabilityScope {
            scopes: vec![],
            declared_tier: SandboxTier::T0,
            trust_tier: TrustTier::Verified,
        },
    );
    inner
        .operator_policy
        .spirit_tier_floor
        .insert(SPIRIT_PID, SandboxTier::T3);
    policy.update(inner);
    policy
}

fn admit(
    adapter: &SecurityManagerAdapter,
    journal: &JournalAdapter,
) -> Result<maos_kernel_core::security::SandboxSpec, SecurityError> {
    adapter.admit_spirit(
        SPIRIT_PID,
        SPIRIT_ID,
        &SandboxConfig {
            tier: SandboxTier::T0,
            image_pin: None,
        },
        &ResourceCaps::default(),
        &caps_required(),
        None,
        journal,
        &PostureSection::from_toml_str("default = \"assistive\"\nallowed_max = \"assistive\"")
            .expect("posture"),
        Some(&EpistemicPolicySection::default_open_fail()),
        None,
        None,
        None,
        None,
        None,
        Some(&class()),
    )
}

fn open_journal(dir: &tempfile::TempDir) -> JournalAdapter {
    JournalAdapter::open(&dir.path().join("journal.ndjson")).expect("journal")
}

fn assert_refused(result: Result<impl Sized, SecurityError>, journal: &JournalAdapter) {
    match result {
        Err(SecurityError::T3AdmissionFailed(_)) => {}
        Err(other) => panic!("expected T3AdmissionFailed, got {other}"),
        Ok(_) => panic!("T3 admission accepted a lock the selected provider rejects"),
    }
    assert!(
        journal.last_event(SPIRIT_ID).is_none(),
        "a refused T3 admission must not journal Load"
    );
}

fn assert_admitted(
    result: Result<maos_kernel_core::security::SandboxSpec, SecurityError>,
    journal: &JournalAdapter,
) {
    let spec = result.expect("a validly signed lock must admit");
    assert_eq!(spec.tier, SandboxTier::T3);
    assert!(matches!(
        journal.last_event(SPIRIT_ID),
        Some(LifecycleEvent::Load)
    ));
}

/// Restores an environment variable on drop (also on panic).
struct EnvGuard(&'static str, Option<std::ffi::OsString>);

impl EnvGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self(key, previous)
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.1.take() {
            Some(value) => std::env::set_var(self.0, value),
            None => std::env::remove_var(self.0),
        }
    }
}

#[test]
fn explicit_lock_valid_under_ring_is_admitted() {
    let dir = tempfile::TempDir::new().unwrap();
    let (path, anchor) = write_signed_lock(&dir, false);
    let adapter = adapter_with(t3_policy(), Arc::new(RingCryptoProvider))
        .with_t3_image_verification(path, anchor);
    let journal = open_journal(&dir);
    assert_admitted(admit(&adapter, &journal), &journal);
}

#[test]
fn explicit_lock_is_refused_when_the_selected_provider_rejects() {
    let dir = tempfile::TempDir::new().unwrap();
    let (path, anchor) = write_signed_lock(&dir, false);
    let adapter = adapter_with(t3_policy(), Arc::new(RejectingProvider))
        .with_t3_image_verification(path, anchor);
    let journal = open_journal(&dir);
    assert_refused(admit(&adapter, &journal), &journal);
}

#[test]
fn explicit_lock_verification_runs_on_the_selected_instance() {
    let dir = tempfile::TempDir::new().unwrap();
    let (path, anchor) = write_signed_lock(&dir, false);
    let selected = Arc::new(CountingProvider::default());
    let adapter = adapter_with(
        t3_policy(),
        Arc::clone(&selected) as Arc<dyn CryptoProvider>,
    )
    .with_t3_image_verification(path, anchor);
    let journal = open_journal(&dir);
    assert_admitted(admit(&adapter, &journal), &journal);
    assert_eq!(
        selected.verifies.load(Ordering::SeqCst),
        1,
        "the lock signature must be verified exactly once, on the selected provider"
    );
}

#[test]
fn bad_signature_and_wrong_trust_anchor_are_refused_by_ring() {
    let dir = tempfile::TempDir::new().unwrap();
    let (bad_path, anchor) = write_signed_lock(&dir, true);
    let adapter = adapter_with(t3_policy(), Arc::new(RingCryptoProvider))
        .with_t3_image_verification(bad_path, anchor);
    let journal = open_journal(&dir);
    assert_refused(admit(&adapter, &journal), &journal);

    let dir = tempfile::TempDir::new().unwrap();
    let (good_path, _) = write_signed_lock(&dir, false);
    let adapter = adapter_with(t3_policy(), Arc::new(RingCryptoProvider))
        .with_t3_image_verification(good_path, [0x11; 32]);
    let journal = open_journal(&dir);
    assert_refused(admit(&adapter, &journal), &journal);
}

/// One test owns the process environment: the env-backed branch reads
/// `MAOS_T3_IMAGE_LOCK_PATH` / `MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX`.
#[test]
fn environment_lock_uses_the_selected_provider() {
    let dir = tempfile::TempDir::new().unwrap();
    let (path, anchor) = write_signed_lock(&dir, false);
    let _lock = EnvGuard::set("MAOS_T3_IMAGE_LOCK_PATH", &path);
    let _anchor = EnvGuard::set("MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX", hex::encode(anchor));

    // Positive control: ring admits the env-backed lock.
    let adapter = adapter_with(t3_policy(), Arc::new(RingCryptoProvider));
    let journal_ok = open_journal(&dir);
    assert_admitted(admit(&adapter, &journal_ok), &journal_ok);

    // The selected provider rejects what ring accepts.
    let adapter = adapter_with(t3_policy(), Arc::new(RejectingProvider));
    let rejecting_dir = tempfile::TempDir::new().unwrap();
    let journal_rejected = open_journal(&rejecting_dir);
    assert_refused(admit(&adapter, &journal_rejected), &journal_rejected);

    // The verification ran on the selected instance.
    let selected = Arc::new(CountingProvider::default());
    let adapter = adapter_with(
        t3_policy(),
        Arc::clone(&selected) as Arc<dyn CryptoProvider>,
    );
    let counting_dir = tempfile::TempDir::new().unwrap();
    let journal_counted = open_journal(&counting_dir);
    assert_admitted(admit(&adapter, &journal_counted), &journal_counted);
    assert_eq!(selected.verifies.load(Ordering::SeqCst), 1);
}
