#![forbid(unsafe_code)]

//! At-rest seal hook for the Loom-lite collective store (Story 11.4c AC3).
//!
//! When org-KMS is configured, adapter-store rows are sealed (ciphertext) at
//! the adapter write layer; with no KMS, writes stay byte-identical Option-A
//! plaintext. The seal itself lives in
//! `maos-secrets::seal_at_rest_opt(Option<&dyn KeyManagementPort>, &dyn
//! CryptoProvider, &[u8])`. To keep `maos-loom-lite` decoupled from
//! `maos-secrets` (L7 closure hygiene), the seal is injected here as a
//! **closure boundary** — an [`AtRestSeal`] the daemon composition root
//! builds from a `KeyManagementPort` + `CryptoProvider` and hands to the
//! store via [`LoomLiteStore::with_at_rest_seal`].
//!
//! Default posture (`None`): writes are byte-identical Option-A plaintext —
//! the pre-11.4c behavior, preserved exactly.
//!
//! [`LoomLiteStore::with_at_rest_seal`]: crate::store::LoomLiteStore::with_at_rest_seal

use std::sync::Arc;

use crate::store::StoreError;

/// At-rest seal closure boundary (Story 11.4c AC3).
///
/// Transforms plaintext payload bytes (`&[u8]`) into sealed ciphertext
/// (`Vec<u8>`), or returns a `String` error. The daemon composition root
/// constructs this from `maos_secrets::seal_at_rest_opt` bound to the
/// configured KMS + crypto provider; `maos-loom-lite` itself carries NO
/// dependency on `maos-secrets` (L7 closure hygiene — the seal is a trait
/// boundary, not a hard dep).
pub type AtRestSeal = Arc<dyn Fn(&[u8]) -> Result<Vec<u8>, String> + Send + Sync>;

/// Holder for the optional at-rest seal hook.
///
/// The single point where collective-store value payload bytes are
/// transformed before persistence. Composed by
/// [`LoomLiteStore`](crate::store::LoomLiteStore); also independently
/// constructable so the seal transform is unit-testable without a live
/// Postgres (the store write path delegates to [`AtRestSealer::seal`]).
///
/// # Posture
///
/// - `Some` hook → the plaintext payload is transformed through the hook
///   (ciphertext on disk). On hook error the write **fails closed** — the
///   error propagates and NO plaintext is persisted under a configured seal
///   posture.
/// - `None` (default) → bytes pass through unchanged — the byte-identical
///   Option-A plaintext posture is preserved exactly.
#[derive(Clone, Default)]
pub struct AtRestSealer {
    seal: Option<AtRestSeal>,
}

impl AtRestSealer {
    /// Construct with an optional seal hook.
    pub fn new(seal: Option<AtRestSeal>) -> Self {
        Self { seal }
    }

    /// Seal plaintext payload bytes for at-rest persistence.
    ///
    /// - Configured hook (`Some`) → transforms `data` through the hook
    ///   (ciphertext). On hook error, returns [`StoreError::AtRestSeal`];
    ///   the caller MUST NOT persist (fail-closed: never write plaintext
    ///   under a configured seal posture).
    /// - No hook (`None`, default) → returns `data` unchanged (byte-
    ///   identical Option-A plaintext).
    pub fn seal(&self, data: &[u8]) -> Result<Vec<u8>, StoreError> {
        match &self.seal {
            Some(f) => f(data).map_err(StoreError::AtRestSeal),
            None => Ok(data.to_vec()),
        }
    }

    /// Whether a seal hook is configured (non-default posture).
    pub fn is_configured(&self) -> bool {
        self.seal.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC3: WITH a configured seal hook, the persisted bytes (the output of
    /// the write-layer seal transform) DIFFER from the plaintext input, and
    /// the transform succeeds (the write path proceeds, not fails-closed).
    ///
    /// The hook is a deterministic XOR stand-in for AEAD — same shape
    /// (`Fn(&[u8]) -> Result<Vec<u8>, String>`), and (crucially) adds NO
    /// `maos-secrets` dependency to this crate.
    #[test]
    fn at_rest_seal_with_hook_transforms_bytes_and_succeeds() {
        let xor_seal: AtRestSeal = Arc::new(|data: &[u8]| {
            Ok(data.iter().map(|b| b ^ 0xA5).collect())
        });
        let sealer = AtRestSealer::new(Some(xor_seal));

        let plaintext: Vec<u8> = b"collective-memory-payload".to_vec();
        let sealed = sealer.seal(&plaintext).expect(
            "configured seal must succeed (write path proceeds; a failure here \
             would fail the write closed, but the XOR stand-in never errors)",
        );

        // Persisted bytes differ from the plaintext input (ciphertext on disk).
        assert_ne!(
            sealed, plaintext,
            "configured seal MUST transform the bytes (ciphertext on disk)"
        );
        assert!(sealer.is_configured());

        // Deterministic stand-in preserves length (sanity).
        assert_eq!(sealed.len(), plaintext.len());

        // Round-trip: XOR is self-inverse, so applying the seal twice yields the
        // original — proves the transform is real and reversible, not a no-op
        // or corruption. (A real AEAD seal would not be self-inverse; the
        // paired unseal lives at the composition root, out of scope for AC3's
        // write-path sealing.)
        let roundtrip = sealer.seal(&sealed).expect("seal is deterministic");
        assert_eq!(roundtrip, plaintext, "XOR stand-in must be self-inverse");
    }

    /// AC3: WITHOUT a seal hook (the Option-A default), the persisted bytes
    /// are byte-identical to the plaintext input — the pre-11.4c behavior
    /// preserved exactly.
    #[test]
    fn at_rest_seal_without_hook_is_byte_identical() {
        let sealer = AtRestSealer::default();

        let plaintext: Vec<u8> = b"collective-memory-payload".to_vec();
        let persisted = sealer
            .seal(&plaintext)
            .expect("identity seal (None) never errors");

        assert_eq!(
            persisted, plaintext,
            "None hook MUST be byte-identical (Option-A default preserved)"
        );
        assert!(!sealer.is_configured());
    }

    /// AC3 (fail-closed): on hook error, `seal()` returns an error so the
    /// write path (`write_with_source`) MUST NOT persist — never write
    /// plaintext under a configured seal posture.
    #[test]
    fn at_rest_seal_hook_error_fails_closed() {
        let failing_seal: AtRestSeal =
            Arc::new(|_data: &[u8]| Err("kms unavailable (stand-in)".to_string()));
        let sealer = AtRestSealer::new(Some(failing_seal));

        let plaintext = b"never-persisted".to_vec();
        let err = sealer
            .seal(&plaintext)
            .expect_err("hook error MUST propagate (fail-closed)");
        assert!(
            matches!(err, StoreError::AtRestSeal(_)),
            "hook error must surface as StoreError::AtRestSeal, got {err:?}"
        );
    }

    /// AC3 (empty / boundary inputs): the seal transform is total over its
    /// input — empty and max-byte payloads flow through the hook without
    /// special-casing.
    #[test]
    fn at_rest_seal_handles_boundary_inputs() {
        let prefix_seal: AtRestSeal = Arc::new(|data: &[u8]| {
            // Deterministic stand-in: prepend a marker (mimics AEAD nonce/tag
            // overhead) so output is observably distinct from input.
            let mut out = Vec::with_capacity(4 + data.len());
            out.extend_from_slice(b"SEAL");
            out.extend_from_slice(data);
            Ok(out)
        });
        let sealer = AtRestSealer::new(Some(prefix_seal));

        // Empty payload.
        let empty_sealed = sealer.seal(&[]).expect("empty input must seal");
        assert!(empty_sealed.len() > 0, "sealed empty must carry the marker");
        assert!(empty_sealed.starts_with(b"SEAL"));

        // Non-empty payload — persisted bytes differ from input.
        let payload = b"\x00\xFF\x10\x20blob-bytes".to_vec();
        let sealed = sealer.seal(&payload).expect("seal must succeed");
        assert_ne!(sealed, payload);
        assert!(sealed.starts_with(b"SEAL"));
        assert_eq!(&sealed[4..], &payload[..], "payload preserved after marker");
    }
}
