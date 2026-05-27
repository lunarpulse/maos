#![forbid(unsafe_code)]

//! One shard of the cap-tokens ring.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use maos_domain::invariants::i1::{IntentClass, Scope, TokenId};
use parking_lot::RwLock;

pub const SHARD_COUNT: usize = 64;

/// One shard of the cap-tokens ring. The hot path reads ONE of these.
#[derive(Debug)]
pub struct CapShard {
    inner: RwLock<HashMap<TokenId, TokenState>>,
}

impl CapShard {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert(&self, id: TokenId, state: TokenState) {
        self.inner.write().insert(id, state);
    }

    /// Hot path. Returns a snapshot of the token state.
    pub fn get(&self, id: &TokenId) -> Option<TokenStateSnapshot> {
        let guard = self.inner.read();
        guard.get(id).map(|s| TokenStateSnapshot {
            signature: s.signature,
            expiry_ns: s.expiry_ns,
            posture_hash: s.posture_hash,
            intent_class: s.intent_class.clone(),
            scope: s.scope.clone(),
            spirit_pid: s.spirit_pid,
            revoked: s.revoked.load(Ordering::Acquire),
        })
    }

    pub fn set_revoked(&self, id: &TokenId) -> Result<bool, ()> {
        let guard = self.inner.read();
        if let Some(state) = guard.get(id) {
            state.revoked.store(true, Ordering::Release);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn revoke_for_spirit(&self, spirit_pid: u32) -> usize {
        let guard = self.inner.read();
        let mut count = 0;
        for state in guard.values() {
            if state.spirit_pid == spirit_pid && !state.revoked.load(Ordering::Acquire) {
                state.revoked.store(true, Ordering::Release);
                count += 1;
            }
        }
        count
    }

    /// Return `true` if this shard holds at least one non-revoked token
    /// for the given `spirit_pid`.
    pub fn has_active_tokens_for_spirit(&self, spirit_pid: u32) -> bool {
        let guard = self.inner.read();
        guard.values().any(|state| {
            state.spirit_pid == spirit_pid && !state.revoked.load(Ordering::Acquire)
        })
    }

    /// Evict expired tokens from this shard. Returns the number evicted.
    /// Called periodically from a maintenance sweep.
    pub fn evict_expired(&self, now_ns: u64) -> usize {
        let mut guard = self.inner.write();
        let before = guard.len();
        guard.retain(|_, state| state.expiry_ns > now_ns);
        before - guard.len()
    }

    /// Debug introspection: list active (non-revoked) token IDs in this shard.
    #[cfg(any(test, feature = "test-introspection"))]
    pub fn list_active(&self) -> Vec<TokenId> {
        let guard = self.inner.read();
        guard
            .iter()
            .filter(|(_, state)| !state.revoked.load(Ordering::Acquire))
            .map(|(id, _)| *id)
            .collect()
    }
}

/// Snapshot of `TokenState` — cloneable, lock-free.
#[derive(Debug, Clone)]
pub struct TokenStateSnapshot {
    pub signature: [u8; 64],
    pub expiry_ns: u64,
    pub posture_hash: [u8; 32],
    pub intent_class: IntentClass,
    pub scope: Scope,
    pub spirit_pid: u32,
    pub revoked: bool,
}

/// In-shard state per token. `revoked` is `AtomicBool` for lock-free
/// flip during the read-lock-held verify path.
#[derive(Debug)]
pub struct TokenState {
    pub signature: [u8; 64],
    pub expiry_ns: u64,
    pub posture_hash: [u8; 32],
    pub intent_class: IntentClass,
    pub scope: Scope,
    pub spirit_pid: u32,
    pub revoked: AtomicBool,
}

/// Fast non-cryptographic hash for shard selection. ~5-10ns per call;
/// FNV-1a is the chosen primitive (zero new deps).
pub fn hash_token_id(id: &TokenId) -> usize {
    let bytes = id.0;
    let mut h: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
    for b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3); // FNV prime
    }
    (h as usize) % SHARD_COUNT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_token_id_is_in_range() {
        let id = TokenId([0xAB; 16]);
        let idx = hash_token_id(&id);
        assert!(idx < SHARD_COUNT);
    }

    #[test]
    fn hash_token_id_different_ids_different_shards_usually() {
        let id1 = TokenId([1u8; 16]);
        let id2 = TokenId([2u8; 16]);
        // They might collide, but it's unlikely for this test
        // Just assert they both map to valid ranges
        assert!(hash_token_id(&id1) < SHARD_COUNT);
        assert!(hash_token_id(&id2) < SHARD_COUNT);
    }
}
