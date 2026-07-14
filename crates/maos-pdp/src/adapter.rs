#![forbid(unsafe_code)]

//! `PolicyDecisionPort` adapter — in-process Cedar reference engine (F3).
//!
//! Story 11.4a, ADR-050 / NFR-Sec-17. This is the **reference** adapter that
//! proves capability-authorization decisions come from a REAL policy engine,
//! not a hardcoded stand-in. Cedar (`cedar-policy`, pure-Rust, Apache-2.0) is
//! in-process — so the deny tripwire is a real per-commit gate, not an
//! advisory-skipped live leg like an external OPA/Vault server would be (the
//! trap `check-multi-region-slo` / `check-scale-churn` Postgres legs fall
//! into). OPA / Vault adapters are additive-per-port (ADR-010) and would
//! carry a `services:` block (F5) — out of scope for 11.4a.
//!
//! # Fail-closed posture
//!
//! The adapter is sync (the engine is in-process — no async boundary, unlike
//! a remote adapter which would mirror the `maos-loom-lite` `spawn_blocking`
//! recipe). A Cedar panic is caught by a panic firewall and mapped to a typed
//! `PolicyDecisionError::Unreachable` — no panic ever crosses into the kernel
//! (mirrors the `maos-loom-lite` `block_on_or_typed` idiom). The reconciler
//! treats a configured-but-unreachable PDP as fail-closed (F4): freeze the
//! last-known-good CoW snapshot, never relax to permissive defaults.
//!
//! # Anti-canned discipline (the story's central thesis)
//!
//! Every `evaluate` call forwards to the real Cedar engine. The
//! `pdp-fault-inject` feature (AC3) stubs the engine to a canned `Allow` and
//! asserts the deny test goes RED — proving the deny is engine-derived, not a
//! constant.

use std::sync::RwLock;

use cedar_policy::{Authorizer, Context, Decision, Entities, EntityUid, PolicySet, Request};
use maos_domain::ports::{
    PolicyDecisionError, PolicyDecisionPort, PolicyDecisionRequest, PolicyVerdict,
};

/// Out-of-kernel Cedar reference adapter implementing `PolicyDecisionPort`.
///
/// Holds a real in-process Cedar `Authorizer` + the loaded operator policy
/// (`PolicySet`, behind a `RwLock` so `load_policy` can swap it). The adapter
/// is constructed at the composition root and injected as
/// `Option<Arc<dyn PolicyDecisionPort>>`.
pub struct CedarPolicyAdapter {
    authorizer: Authorizer,
    policy: RwLock<Option<PolicySet>>,
}

impl CedarPolicyAdapter {
    /// Construct with no policy loaded. The reconciler MUST call
    /// `load_policy` before evaluation is meaningful; `is_healthy` returns
    /// `false` until a policy is loaded.
    pub fn new() -> Self {
        Self {
            authorizer: Authorizer::new(),
            policy: RwLock::new(None),
        }
    }

    /// Construct and immediately load `policy_text`. Convenience for the
    /// composition root + tests.
    pub fn with_policy(policy_text: &str) -> Result<Self, PolicyDecisionError> {
        let adapter = Self::new();
        adapter.load_policy(policy_text)?;
        Ok(adapter)
    }

    /// Build a schemaless Cedar `Request` for one PDP request.
    ///
    /// - principal: `Spirit::"<pid>"` (the authorization subject in 11.4a).
    /// - action: `Action::"<capability_key>"` (the kernel's stable enterprise
    ///   PDP action key — identical keying to `per_capability_deny`).
    /// - resource: a fixed placeholder (the PDP layer is capability-scoped,
    ///   not resource-scoped, in 11.4a).
    /// - context: empty (the optional `principal_attributes`, F7, are for
    ///   11.4c's SSO layer and are not modeled into Cedar yet).
    fn build_cedar_request(req: &PolicyDecisionRequest) -> Result<Request, PolicyDecisionError> {
        let principal: EntityUid = format!("Spirit::\"{}\"", req.spirit_pid)
            .parse()
            .map_err(|e| PolicyDecisionError::Transport(format!("principal uid parse: {e}")))?;
        let action: EntityUid = format!("Action::\"{}\"", req.capability_key)
            .parse()
            .map_err(|e| PolicyDecisionError::Transport(format!("action uid parse: {e}")))?;
        let resource: EntityUid = "Resource::\"spirit\""
            .parse()
            .map_err(|e| PolicyDecisionError::Transport(format!("resource uid parse: {e}")))?;
        Request::new(principal, action, resource, Context::empty(), None)
            .map_err(|e| PolicyDecisionError::Transport(format!("cedar request build: {e}")))
    }

    /// Real Cedar evaluation (behind the panic firewall + fault-inject gate).
    fn evaluate_real(
        &self,
        requests: &[PolicyDecisionRequest],
    ) -> Result<Vec<PolicyVerdict>, PolicyDecisionError> {
        let guard = self
            .policy
            .read()
            .map_err(|e| PolicyDecisionError::Transport(format!("policy lock poisoned: {e}")))?;
        let policies = guard
            .as_ref()
            .ok_or_else(|| PolicyDecisionError::Unreachable {
                reason: "no operator policy loaded".into(),
            })?;
        let entities = Entities::empty();
        let mut out = Vec::with_capacity(requests.len());
        for req in requests {
            let cedar_req = Self::build_cedar_request(req)?;
            // The real engine call — `is_authorized` is Cedar's authorization
            // entry point. Each iteration is a DISTINCT evaluation (AC2: the
            // policy swap forces a fresh load, so two calls under two policies
            // are two real evaluations, NOT a memoized cache).
            let resp = self
                .authorizer
                .is_authorized(&cedar_req, policies, &entities);
            let verdict = match resp.decision() {
                Decision::Allow => PolicyVerdict::Allow,
                Decision::Deny => PolicyVerdict::Deny,
            };
            out.push(verdict);
        }
        Ok(out)
    }
}

impl Default for CedarPolicyAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyDecisionPort for CedarPolicyAdapter {
    fn load_policy(&self, policy_text: &str) -> Result<(), PolicyDecisionError> {
        // Parse the operator policy (Cedar policy-set source) into a real
        // `PolicySet`. A malformed policy → InvalidPolicy (a configured PDP
        // that can't load its policy is treated as unreachable → fail-closed,
        // F4). A successful swap REPLACES the held PolicySet, so the next
        // `evaluate` re-evaluates against the new policy (AC2 — NOT a cache).
        let policies = policy_text
            .parse::<PolicySet>()
            .map_err(|e| PolicyDecisionError::InvalidPolicy(e.to_string()))?;
        let mut guard = self
            .policy
            .write()
            .map_err(|e| PolicyDecisionError::Transport(format!("policy lock poisoned: {e}")))?;
        *guard = Some(policies);
        Ok(())
    }

    #[allow(unreachable_code)]
    fn evaluate(
        &self,
        requests: &[PolicyDecisionRequest],
    ) -> Result<Vec<PolicyVerdict>, PolicyDecisionError> {
        // AC3 FALSIFIER — `pdp-fault-inject` stubs the real engine to a canned
        // `Allow`. With it ON, a real `forbid` rule returns `Allow` (not
        // `Deny`) → the deny test goes RED, proving the deny is engine-derived,
        // not a constant (§A7.3: the flag REMOVES the real engine, the verdict
        // flips). Dev/CI-only; `compile_error!` in `lib.rs` blocks release.
        #[cfg(feature = "pdp-fault-inject")]
        {
            return Ok(requests.iter().map(|_| PolicyVerdict::Allow).collect());
        }

        // Panic firewall — a Cedar panic must never cross into the kernel.
        // (In-process Cedar is pure-Rust and shouldn't panic on well-formed
        // input, but the firewall is the fail-closed belt-and-braces.)
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.evaluate_real(requests)
        }));
        match res {
            Ok(inner) => inner,
            Err(_) => Err(PolicyDecisionError::Unreachable {
                reason: "cedar engine panicked during evaluation".into(),
            }),
        }
    }

    fn is_healthy(&self) -> bool {
        // A policy must be loaded. (For a future remote adapter this would
        // also probe connectivity — the F4 runtime-drop hook.)
        self.policy.read().map(|g| g.is_some()).unwrap_or(false)
    }
}
