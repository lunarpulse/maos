#![forbid(unsafe_code)]

//! Story 6.2 AC5 / AC6 — CliWrapperSpirit runtime stdio bridge.
//!
//! ## Story 6.2 AC6 — FR52 capability-token-authority subprocess invocation
//!
//! The invocation flow:
//!
//! 1. Invoking Spirit obtains a `Scope::CliSubprocessSpawn` cap-token via the
//!    existing CapabilityRegistryPort (`verify` is the 5µs P99 hot path).
//! 2. Runtime re-derives the manifest's `argv_prefix_hash` and asserts equality
//!    with the cap-token's bound hash (TOCTOU correctness per ADR-023).
//! 3. Spawns the subprocess via the EXISTING `spawn_t3()` path
//!    (`security/sandbox/t3/spawn.rs:54`). Story 6.2 does NOT bypass T3.
//! 4. Subprocess child's stdout/stderr is captured line-by-line via the
//!    existing `cap_audit_bridge.rs` from Story 5.5a, with each line written
//!    to the Transparency Log as a `FrameKind::CliSubprocessOutput` row.
//! 5. Every captured row carries `intent_lineage` inherited from the invoking
//!    Spirit's session-originating intent (cross-references AC4 corpus class
//!    `lineage_via_cli_subprocess`).
//! 6. On subprocess exit a `FrameKind::CapabilityInvocation` audit row is
//!    written and the cap-token is revoked with
//!    `RevokeReason::CliSubprocessExit`.
//!
//! ## v0.5-α scope
//!
//! The full ndjson/json-rpc/raw stdio bridge + control-channel + recovery
//! policy state machine ships as scaffolding at v0.5-α; the integration
//! tests `cli_subprocess_invocation_fr52.rs` exercise the surface end-to-end
//! using `echo` as the CLI stand-in for the Founder-Loop wedge demo per
//! `[[feedback_lunarpulse_observability_preference]]`.

use sha2::Digest;

/// Story 6.2 AC6 — recompute the manifest's `argv_prefix_hash` for cap-token
/// binding verification. Re-derived at runtime; asserted equal to the
/// hash bound into the issued cap-token at issue-time per ADR-023 TOCTOU
/// correctness.
pub fn argv_prefix_hash(argv_prefix: &[String]) -> [u8; 32] {
    let mut hasher = sha2::Sha256::new();
    for arg in argv_prefix {
        hasher.update((arg.len() as u32).to_le_bytes());
        hasher.update(arg.as_bytes());
    }
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argv_prefix_hash_empty_is_stable() {
        let h1 = argv_prefix_hash(&[]);
        let h2 = argv_prefix_hash(&[]);
        assert_eq!(h1, h2);
    }

    #[test]
    fn argv_prefix_hash_differs_with_args() {
        let h1 = argv_prefix_hash(&["code".to_string()]);
        let h2 = argv_prefix_hash(&["chat".to_string()]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn argv_prefix_hash_deterministic() {
        let args = vec!["code".to_string(), "--verbose".to_string()];
        let h1 = argv_prefix_hash(&args);
        let h2 = argv_prefix_hash(&args);
        assert_eq!(h1, h2);
    }
}
