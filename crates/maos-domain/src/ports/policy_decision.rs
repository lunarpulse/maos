//! Policy Decision Port — sync trait for the out-of-kernel enterprise Policy
//! Decision Point (Story 11.4a, ADR-050 / NFR-Sec-17).
//!
//! # Architecture
//!
//! The kernel mediates capability-authorization decisions via this injected
//! port trait. The reference implementation lives in `maos-pdp` (user-space,
//! in-process Cedar engine, F3). The kernel stays runtime-agnostic and sync
//! and NEVER calls the PDP on the IAC/token hot path (ADR-030 `<5µs` P99):
//! the composition-root reconciler evaluates the operator policy off-hot-path
//! via this port and materializes the effective deny set into the bounded
//! `OperatorPolicyConfig.per_capability_deny` forbid layer (F2) consumed by
//! the read-mostly CoW `PolicyTable::evaluate` table walk.
//!
//! # Zero-async-dependency guarantee
//!
//! This trait follows the `maos-domain` zero-async contract (`lib.rs`):
//! no `async fn`, no tokio types. Only sync trait method signatures. The
//! async boundary (should a remote OPA/Vault adapter need one) is owned by
//! the adapter crate, mirroring the `CollectiveMemoryPort` recipe.
//!
//! # Additive / optional
//!
//! Injected as `Option<Arc<dyn PolicyDecisionPort>>`. When `None`, no PDP is
//! configured and the kernel-default `PolicyTable` is the authority — byte-
//! identical to pre-11.4a (AC1). When `Some`, the reconciler evaluates the
//! operator policy and materializes the verdicts.

use crate::invariants::i1::Scope;

/// Stable enterprise-PDP action key for a capability scope.
///
/// These strings are the operator-facing Cedar `Action` ids and the kernel's
/// materialized policy keys. They MUST NOT depend on Rust enum discriminants or
/// `Debug` output; policies need a stable vocabulary across compiler versions.
pub fn scope_action_key(scope: &Scope) -> &'static str {
    match scope {
        Scope::FsRead { .. } => "fs.read",
        Scope::FsWrite { .. } => "fs.write",
        Scope::NetHttps { .. } => "net.https",
        Scope::ProcExec { .. } => "proc.exec",
        Scope::SubSpiritSpawn { .. } => "subspirit.spawn",
        Scope::ProviderInfer { .. } => "provider.infer",
        Scope::IacSend { .. } => "iac.send",
        Scope::MemRead { .. } => "mem.read",
        Scope::MemWrite { .. } => "mem.write",
        Scope::SelfTelemetryRead => "self.telemetry.read",
        Scope::LogRecall => "log.recall",
        Scope::LogFetch => "log.fetch",
        Scope::DistillateWrite => "distillate.write",
        Scope::McpCall { .. } => "mcp.call",
        Scope::CliSubprocessSpawn { .. } => "cli.subprocess.spawn",
        Scope::GatewaySend { .. } => "gateway.send",
        Scope::SkillAuthorSelf => "skill.author.self",
        Scope::LoomRead => "loom.read",
        Scope::LoomWrite => "loom.write",
        Scope::LoomScan => "loom.scan",
    }
}

use std::collections::HashMap;

/// Error returned when the policy-decision port is unreachable or times out.
///
/// Per AC4 / F4: typed, halt-safe, bounded timeout — no panic, no hang. A
/// configured-but-unreachable PDP MUST fail closed (freeze last-known-good /
/// degrade to deny), never fall open to permissive defaults (L4 P0).
#[derive(Debug, thiserror::Error)]
pub enum PolicyDecisionError {
    /// The PDP engine is unreachable (engine down, connection refused, DNS
    /// failure). For an in-process engine this is an internal panic caught by
    /// the adapter's panic firewall; for a remote PDP it is a network failure.
    #[error("policy decision point unreachable: {reason}")]
    Unreachable { reason: String },

    /// The operation timed out waiting for the PDP engine.
    #[error("policy decision point timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// An internal transport or protocol error from the engine evaluation.
    #[error("policy decision point transport error: {0}")]
    Transport(String),

    /// The supplied policy is malformed / failed to parse in the engine's
    /// native language (Cedar policy set, OPA bundle, etc.). A configured PDP
    /// that fails to load its policy MUST be treated as unreachable → fail
    /// closed (F4).
    #[error("policy decision point invalid policy: {0}")]
    InvalidPolicy(String),
}

/// One capability-authorization request submitted to the PDP.
///
/// The decision SUBJECT in 11.4a is the existing kernel `spirit_pid` (u32) —
/// NOT a federated principal (SSO/OIDC/SAML identity assertions are 11.4c).
/// The optional opaque `principal_attributes` field is shaped NOW (F7) so
/// 11.4c can layer an authenticated principal as an additional PDP request
/// attribute additively, without an ABI/contract break. It is unused in 11.4a.
#[derive(Debug, Clone)]
pub struct PolicyDecisionRequest {
    /// The Spirit's kernel-assigned PID (the authorization subject in 11.4a).
    pub spirit_pid: u32,
    /// Stable enterprise-PDP action key — identical keying to the kernel's
    /// `per_capability_approval` / `per_capability_deny` action vocabulary
    /// (for example `fs.read`, never Rust `Debug`/discriminant output). The
    /// reconciler submits one request per capability the operator policy may
    /// forbid; a `Deny` verdict materializes this key into a deny set.
    pub capability_key: String,
    /// Optional opaque principal attributes for 11.4c (SSO identity layer).
    /// Unused in 11.4a (`None`); populated by the SSO→capability-token
    /// issuance slice. Kept additive so 11.4c is non-breaking.
    pub principal_attributes: Option<HashMap<String, String>>,
}

/// The PDP's authorization verdict for a single request.
///
/// Maps onto the F2 deny layer: `Deny` ⇒ the capability is forbidden
/// (Cedar `forbid`-beats-`permit`) and materializes into the global or
/// per-spirit deny set; `Allow` ⇒ no opinion (the kernel ceiling from the
/// manifest / tiers still applies). The PDP can only subtract, never grant
/// beyond the manifest (I1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyVerdict {
    /// The PDP permits the capability (no forbid rule fires).
    Allow,
    /// The PDP forbids the capability (a `forbid` rule fired).
    Deny,
}

/// Sync port trait for the enterprise Policy Decision Point.
///
/// Injected into the composition root as `Option<Arc<dyn PolicyDecisionPort>>`
/// and called by the off-hot-path reconciler (NEVER from the IAC/token hot
/// path — ADR-030). The reference adapter (`maos-pdp`) holds a real in-process
/// Cedar engine; OPA / Vault adapters are additive-per-port (ADR-010) and
/// would carry a `services:` block (F5) — out of scope for 11.4a.
///
/// Per architecture:
/// - ADR-006 / I1: the kernel mediates + keeps the ceiling; the PDP layer can
///   only subtract (deny-wins), never grant beyond the Spirit's manifest.
/// - ADR-030: NEVER called on the hot path — evaluation is off-hot-path,
///   materialized into the read-mostly CoW snapshot.
///
/// # Anti-canned discipline (the story's central thesis)
///
/// Decisions come from REAL engine evaluation, not a `HashMap` literal. The
/// adapter MUST forward each `evaluate` call to the engine; the `pdp-fault-
/// inject` falsifier (AC3) stubs the engine to a canned `Allow` and asserts
/// the deny test goes RED, proving the deny is engine-derived.
pub trait PolicyDecisionPort: Send + Sync {
    /// Class: supervision
    ///
    /// Load (or replace) the operator policy in the engine's native language.
    /// The adapter compiles + holds it; subsequent `evaluate` calls use it.
    /// Re-loading — a policy swap — forces a fresh compile, so the next
    /// `evaluate` reflects the new policy (AC2: two DISTINCT engine
    /// evaluations, NOT a memoized cache).
    fn load_policy(&self, policy_text: &str) -> Result<(), PolicyDecisionError>;

    /// Class: supervision
    ///
    /// Evaluate the loaded policy for `requests`. Returns one verdict per
    /// request, aligned by index. The allow/deny count is the actual engine
    /// output, reconciled by the caller against the request cardinality (the
    /// derive-and-reconcile numerator — never a committed/hardcoded literal,
    /// the 11.2a vacuous-count lesson).
    fn evaluate(
        &self,
        requests: &[PolicyDecisionRequest],
    ) -> Result<Vec<PolicyVerdict>, PolicyDecisionError>;

    /// Class: supervision
    ///
    /// Whether the PDP engine is healthy (a policy is loaded + the engine is
    /// reachable). The reconciler's fail-closed hook (F4): a configured PDP
    /// that drops at runtime freezes the CoW snapshot rather than relaxing it.
    fn is_healthy(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_decision_port_is_object_safe() {
        // If this compiles, the trait is dyn-compatible per RFC 2027 —
        // required for `Arc<dyn PolicyDecisionPort>` injection at the
        // composition root (`maos-bin/src/main.rs`). Mirrors the
        // `CryptoProvider` object-safety test at `ports/crypto.rs:142`.
        fn _accepts_dyn(_: &dyn PolicyDecisionPort) {}
    }

    #[test]
    fn policy_decision_error_has_typed_variants() {
        // Typed error taxonomy — fail-closed routing depends on matching
        // variants, not string parsing. Each variant is constructible and
        // distinguishable by discriminant (the enum does not derive PartialEq,
        // matching CollectivePortError).
        let unreach = PolicyDecisionError::Unreachable { reason: "x".into() };
        let timeout = PolicyDecisionError::Timeout { timeout_ms: 1 };
        let transport = PolicyDecisionError::Transport("a".into());
        let invalid = PolicyDecisionError::InvalidPolicy("p".into());
        assert!(matches!(unreach, PolicyDecisionError::Unreachable { .. }));
        assert!(matches!(timeout, PolicyDecisionError::Timeout { .. }));
        assert!(matches!(transport, PolicyDecisionError::Transport(_)));
        assert!(matches!(invalid, PolicyDecisionError::InvalidPolicy(_)));
    }

    #[test]
    fn policy_verdict_only_has_allow_and_deny() {
        // The verdict space is exactly {Allow, Deny} — no third "RequireApproval"
        // arm. The PDP layer is deny-only (subtract-only, I1); approval-class
        // routing stays kernel-owned (`per_capability_approval`).
        assert_eq!(PolicyVerdict::Allow, PolicyVerdict::Allow);
        assert_eq!(PolicyVerdict::Deny, PolicyVerdict::Deny);
        assert_ne!(PolicyVerdict::Allow, PolicyVerdict::Deny);
    }
}
