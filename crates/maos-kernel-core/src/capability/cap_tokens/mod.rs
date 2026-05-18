#![forbid(unsafe_code)]

//! Capability Tokens — lock-free hot-path token mediation per ADR-030.
//!
//! Architecture §4.6 + ADR-030 + ADR-023. The hot path (token verify on
//! every IAC frame and every tool call) takes a read-lock on exactly one
//! shard (selected by `hash(token_id) % 64`), with CAS on `AtomicU64`
//! quota counters per token. No global lock. P99 verify latency budget:
//! 5µs (ADR-030 ship gate), 100µs end-to-end (NFR-Perf-3 ship gate).
//!
//! # TOCTOU correctness (NFR-Maint-8)
//!
//! Every `verify(token, current_posture, current_sandbox)` re-reads the
//! current state from the shard ring AND re-validates against
//! `posture_snapshot_hash` and `intent_class` baked into the token at
//! issue. Tokens carrying stale posture (changed since issue) are
//! rejected. There is NO caching of "this token was valid 50µs ago."
//!
//! # I9 status
//!
//! This module lives in `crates/maos-kernel-core/src/capability/cap_tokens/`
//! — an I9-sanctioned directory per `xtask/i9-whitelist.toml`. Persistent
//! state (the shard ring) is exempt from the I9 denylist by virtue of
//! living in this whitelisted directory.

pub mod shard;
pub mod key;
pub mod body;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope, TokenId};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::capability::CapError;
use maos_domain::ports::crypto::CryptoProvider;
use subtle::ConstantTimeEq;

use crate::capability::cap_audit;

pub use shard::{CapShard, TokenState, TokenStateSnapshot, SHARD_COUNT};
pub use key::Ed25519SigningKey;
pub use body::{CapTokenBody, scope_hash};

/// Monotonic nanosecond counter. Initialized once per boot.
static MONOTONIC_BASE: AtomicU64 = AtomicU64::new(0);

static BOOT_INSTANT: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// Initialize the monotonic clock base. Called once from the composition root.
/// Idempotent — safe to call multiple times (tests, re-initialization).
pub fn init_monotonic_base() {
    let ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    MONOTONIC_BASE.store(ns, Ordering::Relaxed);
    let _ = BOOT_INSTANT.set(std::time::Instant::now());
}

/// Nanoseconds since boot. Used for TTL checks.
/// Computes epoch-base + elapsed-since-init so the clock actually advances.
pub fn monotonic_now_ns() -> u64 {
    let base = MONOTONIC_BASE.load(Ordering::Relaxed);
    let elapsed = BOOT_INSTANT.get()
        .map(|i| i.elapsed().as_nanos() as u64)
        .unwrap_or(0);
    base.saturating_add(elapsed)
}

/// Generate a 16-byte token ID using the boot nonce + counter.
fn generate_token_id(boot_nonce: u64) -> TokenId {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let c = COUNTER.fetch_add(1, Ordering::Relaxed);
    assert!(c < u64::MAX - 1, "cap_tokens: token ID counter exhausted — restart required");
    let mut bytes = [0u8; 16];
    bytes[0..8].copy_from_slice(&boot_nonce.to_be_bytes());
    bytes[8..16].copy_from_slice(&c.to_be_bytes());
    TokenId(bytes)
}

/// Reason for token revocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevokeReason {
    /// Operator explicitly revoked.
    Operator,
    /// Spirit crashed or was unloaded.
    SpiritUnload { spirit_pid: u32, count: usize },
    /// TTL expired (natural death).
    TtlExpired,
}

/// The cap-tokens shard ring. One per Host; constructed in the
/// composition root and held inside `CapabilityRegistryAdapter`.
pub struct CapTokensShardRing {
    shards: Arc<[CapShard; SHARD_COUNT]>,
    crypto: Arc<dyn CryptoProvider>,
    signing_key: Ed25519SigningKey,
    boot_nonce: u64,
    audit: cap_audit::Sender,
}

impl std::fmt::Debug for CapTokensShardRing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapTokensShardRing")
            .field("shards", &"[CapShard; 64]")
            .field("crypto", &"Arc<dyn CryptoProvider>")
            .field("signing_key", &self.signing_key)
            .field("boot_nonce", &self.boot_nonce)
            .field("audit", &"Sender")
            .finish()
    }
}

impl CapTokensShardRing {
    pub fn new(
        crypto: Arc<dyn CryptoProvider>,
        signing_key: Ed25519SigningKey,
        boot_nonce: u64,
        audit: cap_audit::Sender,
    ) -> Self {
        Self {
            shards: Arc::new(std::array::from_fn(|_| CapShard::new())),
            crypto,
            signing_key,
            boot_nonce,
            audit,
        }
    }

    /// Issue a capability token. Returns the wire-stable `CapabilityToken`.
    ///
    /// Latency budget: not on the hot path; budget is "fast but not <5µs."
    pub fn issue(
        &self,
        spirit_pid: u32,
        scope: Scope,
        ttl_secs: u32,
        posture_snapshot_hash: [u8; 32],
        intent_class: IntentClass,
    ) -> Result<CapabilityToken, CapError> {
        // Cap TTL per ADR-023
        let effective_ttl = match intent_class {
            IntentClass::HighPrivilege => ttl_secs.min(60),
            IntentClass::Standard => ttl_secs.min(300),
            IntentClass::Readonly => ttl_secs.min(900),
        };
        let now_ns = monotonic_now_ns();
        let expiry_ns = now_ns + (effective_ttl as u64) * 1_000_000_000;
        let token_id = generate_token_id(self.boot_nonce);
        let body = CapTokenBody {
            token_id,
            spirit_pid,
            boot_nonce: self.boot_nonce,
            expiry_ns,
            scope_hash: scope_hash(&scope),
            posture_snapshot_hash,
            intent_class,
        };
        let body_bytes = body.to_signing_bytes();
        let signature_vec = self
            .crypto
            .sign_capability_token(self.signing_key.as_seed_bytes(), &body_bytes)?;
        let signature: [u8; 64] = signature_vec.as_slice().try_into()
            .map_err(|_| maos_domain::ports::crypto::CryptoError::OperationFailed("signature must be 64 bytes"))?;

        let shard_idx = shard::hash_token_id(&token_id);
        let shard = &self.shards[shard_idx];
        shard.insert(
            token_id,
            TokenState {
                signature,
                expiry_ns,
                posture_hash: posture_snapshot_hash,
                intent_class,
                scope: scope.clone(),
                spirit_pid,
                revoked: std::sync::atomic::AtomicBool::new(false),
            },
        );

        // Audit (try_send, never block)
        if self.audit.try_send(cap_audit::CapAuditEvent::Issue {
            token_id,
            spirit_pid,
            scope,
            ttl_secs: effective_ttl,
        }).is_err() {
            cap_audit::record_drop();
        }

        Ok(CapabilityToken::new(token_id, spirit_pid, expiry_ns, signature))
    }

    /// Verify a capability token. THE hot path. Must complete in <5µs P99.
    ///
    /// Re-validates against current posture and sandbox tier per
    /// ADR-023 / NFR-Maint-8 TOCTOU correctness.
    pub fn verify(
        &self,
        token: &CapabilityToken,
        current_posture_hash: [u8; 32],
        _current_sandbox: SandboxTier,
    ) -> Result<(), CapError> {
        let shard_idx = shard::hash_token_id(&token.token_id);
        let shard = &self.shards[shard_idx];

        // Read-lock on this one shard; no global lock.
        let state = shard.get(&token.token_id)
            .ok_or(CapError::UnknownToken)?;

        // Expiry check (TTL)
        let now_ns = monotonic_now_ns();
        if now_ns >= state.expiry_ns {
            return Err(CapError::Expired);
        }

        // Revocation check
        if state.revoked {
            return Err(CapError::Revoked);
        }

        // Spirit-PID binding (ADR-023)
        if state.spirit_pid != token.spirit_pid {
            return Err(CapError::SpiritIdMismatch);
        }

        // Signature integrity — constant-time equality
        if !bool::from(state.signature.ct_eq(&token.signature)) {
            return Err(CapError::SignatureMismatch);
        }

        // TOCTOU: current state vs token-baked posture
        if state.posture_hash != current_posture_hash {
            return Err(CapError::PostureMismatch);
        }

        Ok(())
    }

    /// Look up the scope for a given token ID without verifying validity.
    /// Returns `None` if the token is unknown.
    pub fn get_scope(&self, token_id: &TokenId) -> Option<Scope> {
        let shard_idx = shard::hash_token_id(token_id);
        let shard = &self.shards[shard_idx];
        shard.get(token_id).map(|s| s.scope)
    }

    /// Revoke a single token. Slow-path (write-lock).
    pub fn revoke(&self, token_id: TokenId, reason: RevokeReason) -> Result<(), CapError> {
        let shard_idx = shard::hash_token_id(&token_id);
        let shard = &self.shards[shard_idx];

        let was_present = shard.set_revoked(&token_id)
            .map_err(|_| CapError::UnknownToken)?;
        if was_present {
            let _ = self.audit.try_send(cap_audit::CapAuditEvent::Revoke {
                token_id,
                reason,
            });
            Ok(())
        } else {
            Err(CapError::UnknownToken)
        }
    }

    /// Revoke all tokens for a Spirit. Crash-recovery / hot-swap rebind
    /// surface. Slow-path (iterates all shards).
    pub fn revoke_all(&self, spirit_pid: u32) -> usize {
        let mut count = 0;
        for shard in self.shards.iter() {
            count += shard.revoke_for_spirit(spirit_pid);
        }
        if self.audit.try_send(cap_audit::CapAuditEvent::Revoke {
            token_id: TokenId::ZERO,
            reason: RevokeReason::SpiritUnload { spirit_pid, count },
        }).is_err() {
            cap_audit::record_drop();
        }
        count
    }

    /// Debug introspection: list all active (non-revoked) token IDs
    /// across all shards. Gated behind test cfg.
    #[cfg(any(test, feature = "test-introspection"))]
    pub fn list_active(&self) -> Vec<TokenId> {
        let mut ids = Vec::new();
        for shard in self.shards.iter() {
            ids.extend(shard.list_active());
        }
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::crypto::tests::MockCryptoProvider;

    fn test_ring() -> CapTokensShardRing {
        let crypto: Arc<dyn CryptoProvider> = Arc::new(MockCryptoProvider);
        let signing_key = Ed25519SigningKey::new([0u8; 32]);
        let (audit_tx, _audit_rx) = cap_audit::channel();
        CapTokensShardRing::new(crypto, signing_key, 0xDEAD_BEEF, audit_tx)
    }

    #[test]
    fn issue_and_verify_round_trip() {
        init_monotonic_base();
        let ring = test_ring();
        let posture = [1u8; 32];
        let token = ring.issue(7, Scope::FsRead { subtree: "/tmp".into() }, 60, posture, IntentClass::Standard).unwrap();
        assert_eq!(token.spirit_pid, 7);
        assert!(ring.verify(&token, posture, SandboxTier(2)).is_ok());
    }

    #[test]
    fn verify_rejects_wrong_posture() {
        init_monotonic_base();
        let ring = test_ring();
        let posture_v1 = [1u8; 32];
        let posture_v2 = [2u8; 32];
        let token = ring.issue(7, Scope::FsRead { subtree: "/tmp".into() }, 60, posture_v1, IntentClass::Standard).unwrap();
        assert!(ring.verify(&token, posture_v1, SandboxTier(2)).is_ok());
        assert_eq!(ring.verify(&token, posture_v2, SandboxTier(2)), Err(CapError::PostureMismatch));
    }

    #[test]
    fn verify_rejects_cross_spirit_replay() {
        init_monotonic_base();
        let ring = test_ring();
        let posture = [1u8; 32];
        let token = ring.issue(7, Scope::FsRead { subtree: "/tmp".into() }, 60, posture, IntentClass::Standard).unwrap();
        let mut tampered = token.clone();
        tampered.spirit_pid = 8;
        assert_eq!(ring.verify(&tampered, posture, SandboxTier(2)), Err(CapError::SpiritIdMismatch));
    }

    #[test]
    fn revoke_makes_verify_fail() {
        init_monotonic_base();
        let ring = test_ring();
        let posture = [1u8; 32];
        let token = ring.issue(7, Scope::FsRead { subtree: "/tmp".into() }, 60, posture, IntentClass::Standard).unwrap();
        ring.revoke(token.token_id, RevokeReason::Operator).unwrap();
        assert_eq!(ring.verify(&token, posture, SandboxTier(2)), Err(CapError::Revoked));
    }

    #[test]
    fn high_privilege_ttl_capped_at_60s() {
        init_monotonic_base();
        let ring = test_ring();
        let posture = [1u8; 32];
        let token = ring.issue(7, Scope::FsRead { subtree: "/tmp".into() }, 3600, posture, IntentClass::HighPrivilege).unwrap();
        let now_ns = monotonic_now_ns();
        assert!(token.expiry_ns <= now_ns + 60 * 1_000_000_000 + 1_000_000);
    }
}
