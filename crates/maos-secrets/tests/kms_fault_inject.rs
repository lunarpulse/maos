//! Story 11.4c review patch P2 — exercises the `kms-fault-inject` passthrough
//! branch of `seal_at_rest`.
//!
//! `seal_at_rest` carries a real `#[cfg(feature = "kms-fault-inject")]` arm
//! that returns the plaintext byte-for-byte (the dev/CI shortcut that bypasses
//! the real envelope seal so the gate can exercise the sealed-write path
//! without a live KMS). Nothing exercised it; this is the **inversion** of
//! `at_rest_seal::ciphertext_differs_from_plaintext`: under the feature, seal
//! MUST return the plaintext unchanged.
//!
//! Gated `#![ignore]` because it requires the non-default `kms-fault-inject`
//! feature; the `check-enterprise-identity` gate runs it via
//! `cargo test -p maos-secrets --features kms-fault-inject -- --ignored`. The
//! release `compile_error!` guard in `src/lib.rs` keeps the feature out of
//! release builds (AC6 / NFR-Sec-19).

#![cfg(feature = "kms-fault-inject")]

use maos_domain::ports::{CryptoProvider, KeyManagementPort};
use maos_kernel_core::security::RingCryptoProvider;
use maos_secrets::{seal_at_rest, LocalMasterKeyKms};

/// A 32-byte reference master key (matches `at_rest_seal::MASTER_KEY_A`).
const MASTER_KEY: [u8; 32] = [0xA1; 32];

/// A row-like plaintext payload.
const PAYLOAD: &[u8] = b"fault-inject passthrough canary";

#[test]
#[ignore = "requires --features kms-fault-inject; gate-controlled via check-enterprise-identity"]
fn kms_fault_inject_seal_is_passthrough() {
    // Bind through the trait objects once — mirrors the composition-root shape
    // (`Arc<dyn KeyManagementPort>`) and proves the fault arm accepts the port
    // dynamically.
    let crypto = RingCryptoProvider;
    let kms = LocalMasterKeyKms::from_master_key(&MASTER_KEY).expect("32-byte master key");
    let kms: &dyn KeyManagementPort = &kms;

    let sealed = seal_at_rest(kms, &crypto, PAYLOAD).expect("fault-inject passthrough seals Ok");

    // Inversion of `ciphertext_differs_from_plaintext`: under the fault feature
    // seal is a passthrough, so the output is byte-identical to the plaintext
    // input (the exact opposite of the default-feature ciphertext assertion).
    assert_eq!(
        sealed.as_slice(),
        PAYLOAD,
        "under kms-fault-inject, seal_at_rest MUST return the plaintext unchanged (passthrough)"
    );
}
