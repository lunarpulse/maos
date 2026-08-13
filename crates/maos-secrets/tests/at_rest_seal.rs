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
