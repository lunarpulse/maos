#![forbid(unsafe_code)]

//! Story 20-14 L2 review round 1 (GPT P2-3, GLM S1) — the root-bound cost scope is part of its
//! token's own state: one lifecycle, no second collection, nothing to scan or prune. Binding is one
//! hash lookup in the token's own shard; the scope survives revocation (a paid in-flight completion
//! still reports its task) and leaves with the token state when the shard evicts it.

use std::sync::atomic::AtomicBool;
use std::sync::OnceLock;

use maos_capability::cap_tokens::{CapShard, TokenState};
use maos_domain::cost::CostScope;
use maos_domain::invariants::i1::{IntentClass, Scope, TokenId};

fn id(n: u32) -> TokenId {
    let mut raw = [0u8; 16];
    raw[..4].copy_from_slice(&n.to_be_bytes());
    TokenId(raw)
}

fn state(expiry_ns: u64) -> TokenState {
    TokenState {
        signature: [0; 64],
        expiry_ns,
        posture_hash: [0; 32],
        intent_class: IntentClass::Standard,
        scope: Scope::ProviderInfer {
            provider: "replay".into(),
        },
        spirit_pid: 1,
        revoked: AtomicBool::new(false),
        cost_scope: OnceLock::new(),
    }
}

fn scope(n: u32) -> CostScope {
    CostScope {
        task_ref: Some(format!("task:{n}")),
        session_principal: Some(format!("p{}", n % 7)),
    }
}

#[test]
fn a_scope_lives_and_dies_with_its_token_state_across_sustained_mint_revoke_and_expiry() {
    const N: u32 = 3_000;
    let shard = CapShard::new();
    for n in 0..N {
        // Even tokens expire at 10 ns, odd ones at 1000 ns.
        shard.insert(id(n), state(if n % 2 == 0 { 10 } else { 1_000 }));
        assert!(shard.bind_cost_scope(&id(n), scope(n)), "bind {n}");
        // Binding never disturbs a neighbour: the one before still reads its own scope.
        if n > 0 {
            assert_eq!(shard.cost_scope(&id(n - 1)), Some(scope(n - 1)));
        }
        if n % 3 == 0 {
            assert_eq!(shard.set_revoked(&id(n)), Some(false));
        }
    }
    // Revoked tokens keep their scope: the paid in-flight completion is still attributable.
    for n in (0..N).filter(|n| n % 3 == 0) {
        assert_eq!(shard.cost_scope(&id(n)), Some(scope(n)), "revoked {n}");
    }
    // One binding per token, and only for a token the shard holds.
    assert!(!shard.bind_cost_scope(&id(1), scope(9_999)));
    assert_eq!(shard.cost_scope(&id(1)), Some(scope(1)));
    assert!(!shard.bind_cost_scope(&id(N + 5), scope(5)));
    assert_eq!(shard.cost_scope(&id(N + 5)), None);
    // Expiry sweep: the scope leaves with the token state; survivors keep theirs.
    assert_eq!(shard.evict_expired(500), (N as usize).div_ceil(2));
    for n in 0..N {
        let expected = (n % 2 == 1).then(|| scope(n));
        assert_eq!(shard.cost_scope(&id(n)), expected, "after eviction {n}");
    }
}
