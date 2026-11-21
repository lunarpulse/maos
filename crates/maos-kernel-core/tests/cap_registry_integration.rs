//! Integration test for the Capability Registry.
//!
//! Covers issue/verify/revoke/expire/TOCTOU/cross-Spirit isolation.

use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::capability::CapError;
use maos_kernel_core::capability::{cap_tokens, CapabilityRegistryAdapter, CapabilityRegistryPort};

mod common;

fn make_adapter() -> CapabilityRegistryAdapter {
    common::registry::adapter(&[(
        7,
        vec![Scope::FsRead {
            subtree: "/tmp".into(),
        }],
    )])
}

#[test]
fn integration_issue_verify_revoke() {
    cap_tokens::init_monotonic_base();
    let adapter = make_adapter();
    let posture = [1u8; 32];

    let token = adapter
        .issue(
            7,
            Scope::FsRead {
                subtree: "/tmp".into(),
            },
            60,
            posture,
            IntentClass::Standard,
        )
        .unwrap();
    assert_eq!(token.spirit_pid, 7);
    assert!(adapter.verify(&token, posture, SandboxTier(2)).is_ok());

    adapter.revoke(token.token_id).unwrap();
    assert!(matches!(
        adapter.verify(&token, posture, SandboxTier(2)),
        Err(CapError::Revoked)
    ));
}

#[test]
fn integration_toctu_rejects_stale_posture() {
    cap_tokens::init_monotonic_base();
    let adapter = make_adapter();
    let posture_v1 = [1u8; 32];
    let posture_v2 = [2u8; 32];

    let token = adapter
        .issue(
            7,
            Scope::FsRead {
                subtree: "/tmp".into(),
            },
            60,
            posture_v1,
            IntentClass::Standard,
        )
        .unwrap();
    assert!(adapter.verify(&token, posture_v1, SandboxTier(2)).is_ok());
    assert!(matches!(
        adapter.verify(&token, posture_v2, SandboxTier(2)),
        Err(CapError::PostureMismatch)
    ));
}

#[test]
fn integration_cross_spirit_replay_rejected() {
    cap_tokens::init_monotonic_base();
    let adapter = make_adapter();
    let posture = [1u8; 32];

    let token = adapter
        .issue(
            7,
            Scope::FsRead {
                subtree: "/tmp".into(),
            },
            60,
            posture,
            IntentClass::Standard,
        )
        .unwrap();
    let mut tampered = token.clone();
    tampered.spirit_pid = 8;
    assert!(matches!(
        adapter.verify(&tampered, posture, SandboxTier(2)),
        Err(CapError::SpiritIdMismatch)
    ));
}

#[test]
fn integration_high_privilege_ttl_capped() {
    cap_tokens::init_monotonic_base();
    let adapter = make_adapter();
    let posture = [1u8; 32];

    let token = adapter
        .issue(
            7,
            Scope::FsRead {
                subtree: "/tmp".into(),
            },
            3600,
            posture,
            IntentClass::HighPrivilege,
        )
        .unwrap();
    assert!(token.expiry_ns <= cap_tokens::monotonic_now_ns() + 60 * 1_000_000_000 + 1_000_000);
}
