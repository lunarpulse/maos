#![forbid(unsafe_code)]

//! D-16-1-V — reason-keyed re-revoke semantics of `CapTokensShardRing`.
//!
//! A repeated `RevokeReason::Operator` revoke of one id must be typed
//! `Err(CapError::Revoked)` and write NO second `cap.revoke` audit row (the
//! operator has to distinguish "I revoked it" from "it was already revoked",
//! and a no-op state change must not duplicate the row). The falsifier half:
//! a repeated `RevokeReason::CliSubprocessExit` revoke of the same id must
//! stay `Ok(())` WITH its row — its caller (`worker_spawn.rs`
//! `revoke_cli_subprocess_exit`) discards the result, and an already-revoked
//! token is the NORMAL case there (unload and CRL application revoke first),
//! so losing the row would drop the Worker's exit-provenance record for
//! every such token (§17 V-35). Absent ids stay `UnknownToken`, distinct
//! from `Revoked`.
//!
//! Audit rows are counted from the ring's actual audit channel, never from a
//! value the test recorded itself.

use std::sync::Arc;

use maos_capability::cap_audit::{self, CapAuditEvent};
use maos_capability::cap_tokens::{
    init_monotonic_base, CapTokensShardRing, Ed25519SigningKey, RevokeReason,
};
use maos_domain::invariants::i1::{IntentClass, Scope, TokenId};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::capability::CapError;
use maos_domain::ports::crypto::{CryptoError, CryptoProvider};

struct MockCryptoProvider;

impl CryptoProvider for MockCryptoProvider {
    fn verify_signature(
        &self,
        _public_key: &[u8],
        _message: &[u8],
        signature: &[u8],
    ) -> Result<(), CryptoError> {
        if signature.iter().all(|&b| b == 0) {
            Ok(())
        } else {
            Err(CryptoError::SignatureInvalid)
        }
    }
    fn seal_for_export(
        &self,
        _key: &[u8],
        _nonce: &[u8],
        _aad: &[u8],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        Ok(plaintext.to_vec())
    }
    fn sign_capability_token(
        &self,
        _signing_key: &[u8],
        token_bytes: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let mut sig = [0u8; 64];
        for (i, b) in token_bytes.iter().enumerate() {
            sig[i % 64] ^= *b;
        }
        Ok(sig.to_vec())
    }
}

fn test_ring(audit: cap_audit::Sender) -> CapTokensShardRing {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(MockCryptoProvider);
    CapTokensShardRing::new(
        crypto,
        Ed25519SigningKey::new([0u8; 32]),
        0xDEAD_BEEF,
        audit,
    )
}

fn issue_fs_read(
    ring: &CapTokensShardRing,
    subtree: &str,
) -> maos_domain::invariants::i1::CapabilityToken {
    ring.issue(
        7,
        Scope::FsRead {
            subtree: subtree.into(),
        },
        60,
        [1u8; 32],
        IntentClass::Standard,
    )
    .unwrap()
}

/// Every `cap.revoke` row the ring actually sent to the audit channel.
fn drain_revoke_rows(
    receiver: &mut tokio::sync::mpsc::Receiver<CapAuditEvent>,
) -> Vec<CapAuditEvent> {
    let mut revokes = Vec::new();
    while let Ok(event) = receiver.try_recv() {
        if matches!(event, CapAuditEvent::Revoke { .. }) {
            revokes.push(event);
        }
    }
    revokes
}

#[test]
fn operator_re_revoke_is_typed_revoked_and_writes_no_second_audit_row() {
    init_monotonic_base();
    let (audit_tx, mut audit_rx) = cap_audit::channel();
    let ring = test_ring(audit_tx);
    let token = issue_fs_read(&ring, "/tmp/operator-re-revoke-16-1");

    assert!(ring.revoke(token.token_id, RevokeReason::Operator).is_ok());
    assert_eq!(
        ring.revoke(token.token_id, RevokeReason::Operator),
        Err(CapError::Revoked)
    );
    // The end state the operator observes through the hot path is unchanged.
    assert_eq!(
        ring.verify(&token, [1u8; 32], SandboxTier(2)),
        Err(CapError::Revoked)
    );

    let revokes = drain_revoke_rows(&mut audit_rx);
    assert_eq!(
        revokes.len(),
        1,
        "a repeated operator revoke must not add a second cap.revoke row"
    );
    assert_eq!(
        revokes[0],
        CapAuditEvent::Revoke {
            token_id: token.token_id,
            reason: RevokeReason::Operator,
        }
    );
}

#[test]
fn cli_subprocess_exit_re_revoke_stays_ok_and_keeps_its_audit_row() {
    init_monotonic_base();
    let (audit_tx, mut audit_rx) = cap_audit::channel();
    let ring = test_ring(audit_tx);
    let token = ring
        .issue(
            42,
            Scope::CliSubprocessSpawn {
                cli_binary_path: "/usr/local/bin/claude".into(),
                argv_prefix_hash: [9u8; 32],
                output_shape_version: "1.0.0".into(),
            },
            300,
            [1u8; 32],
            IntentClass::Standard,
        )
        .unwrap();
    let exit_reason = || RevokeReason::CliSubprocessExit {
        spirit_pid: 42,
        exit_code: Some(0),
    };

    assert!(ring.revoke(token.token_id, exit_reason()).is_ok());
    // Already-revoked is the NORMAL case on this path — the Worker's caller
    // discards the result, but the row must still be written.
    assert_eq!(ring.revoke(token.token_id, exit_reason()), Ok(()));

    let revokes = drain_revoke_rows(&mut audit_rx);
    assert_eq!(
        revokes.len(),
        2,
        "losing the second row would drop exit provenance for every already-revoked token"
    );
    assert_eq!(
        revokes[1],
        CapAuditEvent::Revoke {
            token_id: token.token_id,
            reason: exit_reason(),
        }
    );
}

#[test]
fn absent_token_id_stays_unknown_token_and_writes_no_row() {
    init_monotonic_base();
    let (audit_tx, mut audit_rx) = cap_audit::channel();
    let ring = test_ring(audit_tx);
    let _token = issue_fs_read(&ring, "/tmp/absent-id-16-1");
    let absent = TokenId([0xEE; 16]);

    assert_eq!(
        ring.revoke(absent, RevokeReason::Operator),
        Err(CapError::UnknownToken),
        "never-issued is a different fact from already-revoked"
    );
    assert!(drain_revoke_rows(&mut audit_rx).is_empty());
}
