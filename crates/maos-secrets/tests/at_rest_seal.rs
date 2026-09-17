//! Story 11.4c Task 3 (AC3) — the opt-in at-rest envelope seal.
//!
//! Pins `maos_secrets::seal_at_rest` / `open_at_rest`: a `KeyManagementPort`-
//! wrapped data key combined with `CryptoProvider::seal_for_export`
//! AES-256-GCM (the existing crypto port — no new kernel crypto, L8). The
//! AEAD routes through `CryptoProvider`; the KMS only wraps the data key.
//!
//! Three falsifiers, each hunting a distinct canned-green fake (the reason
//! this story is opus-4-8 — L4):
//!
//! 1. `ciphertext_differs_from_plaintext` — a passthrough "seal" that returns
//!    the plaintext untransformed reds here.
//! 2. `right_key_opens_sealed_payload` — the seal is reversible under the
//!    right key (not a one-way hash or a corrupting transform).
//! 3. `wrong_key_open_fails` — **the CARRIED falsifier** ("wrong-key-fails"):
//!    a key-ignoring seal (encrypts the payload but wraps the data key under a
//!    constant / ignores the KMS master key) would pass #1 and #2 yet open
//!    under ANY key. This reds that exact fake — a naive ciphertext!=plaintext
//!    check alone cannot.
//!
//! The reference `LocalMasterKeyKms` is dev/CI-ONLY (ADR-051 / NFR-Sec-19);
//! production = OS keyring / cloud-KMS, additive-per-port, deferred (F3).

#![cfg(not(feature = "kms-fault-inject"))]

use maos_domain::ports::{CryptoProvider, KeyManagementPort};
use maos_kernel_core::security::RingCryptoProvider;
use maos_secrets::{open_at_rest, seal_at_rest, LocalMasterKeyKms};

/// A 32-byte reference master key (AES-256-sized) for the dev/CI KMS.
const MASTER_KEY_A: [u8; 32] = [0xA1; 32];
/// A DIFFERENT 32-byte master key — proves wrong-key open fails.
const MASTER_KEY_B: [u8; 32] = [0xB2; 32];

/// A row-like plaintext payload (mirrors a loom-lite Collective / audit TL row).
const PAYLOAD: &[u8] = b"{\"region\":\"default\",\"spirit_pid\":42,\"payload\":\"secret-canary\"}";

fn crypto() -> RingCryptoProvider {
    RingCryptoProvider
}

fn kms_from(key: &[u8]) -> LocalMasterKeyKms {
    LocalMasterKeyKms::from_master_key(key).expect("a 32-byte master key -> reference KMS")
}

#[test]
fn ciphertext_differs_from_plaintext() {
    let crypto = crypto();
    let kms = kms_from(&MASTER_KEY_A);
    let sealed = seal_at_rest(&kms, &crypto, PAYLOAD).expect("seal under a configured KMS");

    assert_ne!(
        sealed.as_slice(),
        PAYLOAD,
        "sealed output MUST be ciphertext, not the plaintext — a passthrough seal reds here"
    );
    // The plaintext must not appear verbatim anywhere in the sealed blob —
    // defends against a "prepend-the-plaintext" fake too.
    assert!(
        !sealed
            .as_slice()
            .windows(PAYLOAD.len())
            .any(|w| w == PAYLOAD),
        "the plaintext MUST NOT appear as a contiguous substring of the sealed blob"
    );
}

#[test]
fn right_key_opens_sealed_payload() {
    let crypto = crypto();
    let kms = kms_from(&MASTER_KEY_A);

    // Bind through the trait object once: this both proves the port is
    // object-safe (composition-root shape: `Arc<dyn KeyManagementPort>`) and
    // that `seal_at_rest`/`open_at_rest` accept the port dynamically.
    let kms: &dyn KeyManagementPort = &kms;

    let sealed = seal_at_rest(kms, &crypto, PAYLOAD).expect("seal under a configured KMS");
    let opened = open_at_rest(kms, &sealed).expect("open under the SAME key round-trips");

    assert_eq!(
        opened.as_slice(),
        PAYLOAD,
        "open_at_rest(seal_at_rest(x)) == x under the right key — the seal is reversible, \
         not a destructive one-way transform"
    );
}

#[test]
fn wrong_key_open_fails() {
    let crypto = crypto();

    // Sealed under master key A.
    let sealed =
        seal_at_rest(&kms_from(&MASTER_KEY_A), &crypto, PAYLOAD).expect("seal under master key A");

    // Opened under master key B -> MUST fail. A key-ignoring seal (encrypts the
    // payload but wraps the data key under a constant / ignores the KMS master
    // key) would open under any key AND pass the ciphertext!=plaintext check
    // above — this is the exact fake the story CARRIES as the must-hunt
    // falsifier ("wrong-key-fails", AC3 / CARRIED non-negotiable).
    let result = open_at_rest(&kms_from(&MASTER_KEY_B), &sealed);
    assert!(
        result.is_err(),
        "open_at_rest under the WRONG master key MUST fail (got Ok); a key-ignoring seal \
         would succeed — the carried wrong-key-fails falsifier"
    );
}

#[test]
fn environment_store_materializes_and_deletes_known_names_only() {
    use maos_domain::ports::{SecretDeleteStatus, SecretKey, SecretStore};

    let store = maos_secrets::EnvSecretStore::new([(
        SecretKey::AnthropicApiKey,
        "sk-ant-canary".to_string(),
    )]);
    assert_eq!(
        store.get(SecretKey::AnthropicApiKey).unwrap().as_deref(),
        Some("sk-ant-canary")
    );
    assert_eq!(
        store.delete(SecretKey::AnthropicApiKey).unwrap(),
        SecretDeleteStatus::Removed
    );
    assert_eq!(store.get(SecretKey::AnthropicApiKey).unwrap(), None);
    store
        .put(SecretKey::OpenAiApiKey, "sk-openai-canary")
        .expect("store environment credential");
    assert_eq!(
        store.get(SecretKey::OpenAiApiKey).unwrap().as_deref(),
        Some("sk-openai-canary")
    );
    assert_eq!(
        store.delete(SecretKey::OpenAiApiKey).unwrap(),
        SecretDeleteStatus::Removed
    );
}

#[test]
fn fallback_store_reports_downgrade_without_secret_material() {
    use maos_domain::ports::{SecretKey, SecretStore};
    use std::sync::{Arc, Mutex};

    let primary = Arc::new(maos_secrets::EnvSecretStore::new([(
        SecretKey::OpenAiApiKey,
        "   ".to_string(),
    )]));
    let fallback = Arc::new(maos_secrets::EnvSecretStore::new([(
        SecretKey::OpenAiApiKey,
        "sk-proj-never-log-this".to_string(),
    )]));
    let events = Arc::new(Mutex::new(Vec::new()));
    let observed = Arc::clone(&events);
    let store = maos_secrets::FallbackSecretStore::new(
        primary,
        fallback,
        Arc::new(move |key, reason| {
            observed
                .lock()
                .unwrap()
                .push(format!("{}:{reason}", key.as_str()));
        }),
    );

    assert_eq!(
        store.get(SecretKey::OpenAiApiKey).unwrap().as_deref(),
        Some("sk-proj-never-log-this")
    );
    let journal = events.lock().unwrap().join("\n");
    assert!(journal.contains("openai-api-key:primary entry present but blank"));
    assert!(!journal.contains("sk-proj-"));
}

#[test]
fn fallback_delete_attempts_the_fallback_after_a_primary_failure() {
    use maos_domain::ports::{SecretDeleteStatus, SecretKey, SecretStore, SecretStoreError};
    use std::sync::Arc;

    struct FailingPrimary;
    impl SecretStore for FailingPrimary {
        fn get(&self, _key: SecretKey) -> Result<Option<String>, SecretStoreError> {
            Ok(None)
        }

        fn put(&self, _key: SecretKey, _value: &str) -> Result<(), SecretStoreError> {
            Err(SecretStoreError::Unavailable("offline".to_string()))
        }

        fn delete(&self, _key: SecretKey) -> Result<SecretDeleteStatus, SecretStoreError> {
            Err(SecretStoreError::Unavailable("offline".to_string()))
        }

        fn is_healthy(&self) -> bool {
            false
        }
    }

    let fallback = Arc::new(maos_secrets::EnvSecretStore::new([(
        SecretKey::AnthropicApiKey,
        "captured-environment-key".to_string(),
    )]));
    let store = maos_secrets::FallbackSecretStore::new(
        Arc::new(FailingPrimary),
        Arc::clone(&fallback) as Arc<dyn SecretStore>,
        Arc::new(|_, _| {}),
    );
    assert!(store.delete(SecretKey::AnthropicApiKey).is_err());
    assert_eq!(
        fallback.get(SecretKey::AnthropicApiKey).unwrap(),
        None,
        "fallback deletion must still run after the primary fails"
    );
}

#[cfg(feature = "encrypted-file")]
#[test]
fn encrypted_file_store_round_trips_ciphertext_and_deletes() {
    use maos_domain::ports::{SecretDeleteStatus, SecretKey, SecretStore};
    use std::sync::Arc;

    let directory = tempfile::tempdir().expect("encrypted secret fixture");
    let root = directory.path().join("vault");
    let store = maos_secrets::EncryptedFileSecretStore::new(
        root.clone(),
        Arc::new(kms_from(&MASTER_KEY_A)),
        Arc::new(crypto()),
    );
    store
        .store(SecretKey::AnthropicApiKey, "sk-ant-sealed")
        .expect("seal credential");
    let on_disk =
        std::fs::read(root.join("anthropic-api-key.sealed")).expect("read sealed credential");
    assert!(
        !on_disk
            .windows(b"sk-ant-sealed".len())
            .any(|window| window == b"sk-ant-sealed"),
        "encrypted backend must not persist plaintext"
    );
    assert_eq!(
        store.get(SecretKey::AnthropicApiKey).unwrap().as_deref(),
        Some("sk-ant-sealed")
    );
    store
        .store(SecretKey::AnthropicApiKey, "sk-ant-replaced")
        .expect("replace sealed credential");
    assert_eq!(
        store.get(SecretKey::AnthropicApiKey).unwrap().as_deref(),
        Some("sk-ant-replaced")
    );
    assert_eq!(
        store.delete(SecretKey::AnthropicApiKey).unwrap(),
        SecretDeleteStatus::Removed
    );
    assert_eq!(store.get(SecretKey::AnthropicApiKey).unwrap(), None);
}
