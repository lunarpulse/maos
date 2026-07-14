//! Story 11.4c Task 3 — the **Option-A default-posture pin** (AC3 / AC5 / L2-F2).
//!
//! 9.4b / ADR-049 ratified plaintext memory rows region-bound by governance
//! only. 11.4c's at-rest AEAD is an **opt-in layer enabled only when an
//! org-KMS is configured**; the default (no `MAOS_KMS_*`) MUST stay
//! **byte-identical Option-A plaintext** (AC3 "And" clause; AC5 zero-config
//! byte-identity leg; L2 "the Option-A reversal trap").
//!
//! `maos-secrets` today exposes ONLY the opt-in envelope — `seal_at_rest` /
//! `open_at_rest` both REQUIRE a `&dyn KeyManagementPort` and produce
//! ciphertext. There is no surface that expresses the *absence* of a KMS
//! (the composition-root `Option<Arc<dyn KeyManagementPort>> == None` arm).
//! This file defines that surface as a **minimal expected API** and pins it
//! RED until implemented.
//!
//! # Minimal expected API (RED — not yet implemented in `maos-secrets`)
//!
//! ```ignore
//! pub fn seal_at_rest_opt(
//!     kms: Option<&dyn KeyManagementPort>,
//!     crypto: &dyn CryptoProvider,
//!     plaintext: &[u8],
//! ) -> Result<Vec<u8>, KmsError>
//! ```
//!
//! Behavior contract:
//! - `None`  -> `Ok(plaintext.to_vec())` — **byte-identical Option-A plaintext**;
//!   the default posture is "disabled", which is `Ok` (not a failure), and the
//!   bytes equal the input exactly (no header, no tag, no transform).
//! - `Some(k)` -> delegates to [`maos_secrets::seal_at_rest`] (real AEAD
//!   ciphertext); encryption is **opt-in via KMS presence**.
//!
//! # Two falsifiers
//!
//! 1. `no_kms_default_returns_byte_identical_plaintext` — a silent default
//!    flip (encrypting when no org-KMS is configured, or returning an error
//!    instead of plaintext) reds here. This is THE L2/F2/AC5 trap.
//! 2. `configured_kms_seals_to_ciphertext_not_passthrough` — a passthrough-
//!    always fake (ignores `Some`/`None`, always hands back plaintext) reds
//!    here. Encryption must be opt-in: `Some` produces real ciphertext.

use maos_domain::ports::{CryptoProvider, KeyManagementPort};
use maos_kernel_core::security::RingCryptoProvider;
use maos_secrets::{seal_at_rest_opt, LocalMasterKeyKms};

/// A 32-byte reference master key (AES-256-sized) for the dev/CI KMS.
const MASTER_KEY_A: [u8; 32] = [0xA1; 32];

/// A row-like plaintext payload — mirrors a loom-lite Collective / audit TL row
/// that, under Option-A, is stored byte-for-byte on disk.
const PAYLOAD: &[u8] =
    b"{\"region\":\"default\",\"spirit_pid\":7,\"row\":\"option-a-plaintext-default\"}";

fn crypto() -> RingCryptoProvider {
    RingCryptoProvider
}

fn kms_from(key: &[u8]) -> LocalMasterKeyKms {
    LocalMasterKeyKms::from_master_key(key).expect("a 32-byte master key -> reference KMS")
}

#[test]
fn no_kms_default_returns_byte_identical_plaintext() {
    // No org-KMS configured -> the composition-root arm is `None`.
    let crypto = crypto();
    let sealed = seal_at_rest_opt(None, &crypto, PAYLOAD).expect(
        "no KMS -> Option-A plaintext; the disabled posture is Ok, never an error \
         (a configured-but-unhealthy KMS failing closed is the OTHER arm, not this one)",
    );

    // THE carried falsifier for the Option-A reversal trap (L2/F2/AC5):
    // a silent default flip — encrypting when no KMS is configured — reds here.
    assert_eq!(
        sealed.as_slice(),
        PAYLOAD,
        "no-KMS default posture MUST be byte-identical Option-A plaintext; a silent default \
         flip (encrypting under the default, or prepending any header/tag) reds here"
    );
}

#[test]
fn configured_kms_seals_to_ciphertext_not_passthrough() {
    // A configured, healthy KMS -> the posture is "encrypted", opt-in.
    let crypto = crypto();
    let kms = kms_from(&MASTER_KEY_A);
    let sealed = seal_at_rest_opt(Some(&kms), &crypto, PAYLOAD)
        .expect("a configured healthy KMS seals (real AEAD), not an error");

    // Encryption is opt-in via KMS presence: a passthrough-always fake
    // (returns plaintext regardless of Some/None) reds here.
    assert_ne!(
        sealed.as_slice(),
        PAYLOAD,
        "with a configured KMS the posture MUST be ciphertext, not plaintext — a \
         passthrough-always fake (ignores Some/None) reds here"
    );
    // The plaintext must not appear verbatim anywhere in the sealed blob —
    // defends against a "prepend-the-plaintext" fake too.
    assert!(
        !sealed.windows(PAYLOAD.len()).any(|w| w == PAYLOAD),
        "under a configured KMS the plaintext MUST NOT appear as a contiguous substring \
         of the sealed blob"
    );
}
