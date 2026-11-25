#![forbid(unsafe_code)]

//! Capability policy — read-mostly copy-on-write policy table.
//!
//! Per architecture §4.6: "Read-mostly; copy-on-write for policy updates."

pub mod decision;

use std::collections::HashMap;
use std::sync::Arc;

use arc_swap::ArcSwap;

use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i9::SandboxTier;

use decision::{ApprovalClass, Capability, Intent, PolicyDecision};

/// Operator policy configuration.
#[derive(Debug, Clone, Default)]
#[maos_attrs::i9_exempt(reason = "operator policy config; part of PolicyTableInner CoW snapshot")]
pub struct OperatorPolicyConfig {
    /// Per-Spirit sandbox tier floor from operator policy.
    pub spirit_tier_floor: HashMap<u32, SandboxTier>,
    /// Global sandbox floor applied to ALL Spirits regardless of manifest.
    pub global_sandbox_floor: SandboxTier,
    /// Per-capability approval class overrides.
    pub per_capability_approval: HashMap<String, decision::ApprovalClass>,
    /// Resource cap floor from operator policy (Story 1b.3).
    pub resource_cap_floor: Option<crate::security::manifest::ResourceCaps>,
    /// Story 11.4a (F2, FLAG-Winston) — org-level deny (forbid) set, keyed
    /// by stable enterprise-PDP action keys (for example `fs.read`), not Rust
    /// discriminant debug output. An absolute deny: the `evaluate` deny-wins
    /// arm returns `Deny` before any grant (Cedar `forbid`-beats-`permit`).
    /// Populated out-of-kernel by the `PolicyDecisionPort` Cedar adapter at
    /// the composition root; the kernel keeps the ceiling (I1) — subtract-only.
    /// Empty (default) ⇒ no-op (AC1).
    pub per_capability_deny: std::collections::HashSet<String>,
    /// Subject-scoped org forbids keyed by `spirit_pid` then stable action key.
    /// This preserves Cedar's principal-specific policy semantics without
    /// letting PDP permits raise the manifest ceiling.
    pub per_spirit_capability_deny: HashMap<u32, std::collections::HashSet<String>>,
    /// Story 16-6 (§16a R3) — the operator's ceiling on a Spirit's runtime
    /// posture, and the fifth axis of the leash to get one.
    ///
    /// `admit_spirit` stored `[posture] default` and `[posture] allowed_max`
    /// VERBATIM from the manifest, so a bearer holder who could put a
    /// manifest into a running daemon chose their own ceiling and then
    /// passed `shift_posture`'s check against it. Every other axis already
    /// had an operator floor (`global_sandbox_floor`, `spirit_tier_floor`,
    /// `resource_cap_floor`, and the two subtract-only deny sets).
    ///
    /// ⚠ NAMED `ceiling`, NOT `floor`, because it is implemented as `min()`.
    /// `Posture` derives `Ord` least-privilege-first
    /// (`Cautious < Assistive < AutonomousWithHalt < Autonomous`), so `min()`
    /// is monotone-DOWN and is a real clamp. A later "fix" to `max()` would
    /// brick every Spirit at `Posture::Autonomous`, which `shift_posture`
    /// refuses as `NonRuntimePosture`.
    ///
    /// `None` (the forced default — `Posture` derives no `Default` while
    /// this struct derives it) ⇒ no clamp, so `maos run` is provably
    /// unchanged unless an operator sets one. Legal values are bounded to
    /// `{cautious, assistive, autonomous-with-halt}` by
    /// [`crate::security::operator_config::PostureSection::resolve_from_env_and_disk`];
    /// `Autonomous` is a no-op ceiling by construction.
    pub operator_posture_ceiling: Option<crate::security::manifest::Posture>,
}

/// Manifest capability scope per Spirit.
#[derive(Debug, Clone, Default)]
#[maos_attrs::i9_exempt(reason = "manifest scope; part of PolicyTableInner CoW snapshot")]
pub struct ManifestCapabilityScope {
    pub scopes: Vec<Scope>,
    pub declared_tier: SandboxTier,
    pub trust_tier: decision::TrustTier,
}

/// Inner policy table — the actual data.
#[derive(Debug, Clone, Default)]
#[maos_attrs::i9_exempt(
    reason = "inner policy data behind ArcSwap; structural-state caching per I9"
)]
pub struct PolicyTableInner {
    pub manifest_scopes: HashMap<u32, ManifestCapabilityScope>,
    pub trust_tier_floor: HashMap<decision::TrustTier, SandboxTier>,
    pub operator_policy: OperatorPolicyConfig,
    /// Story 3.2 — per-Spirit runtime posture state. Updated atomically
    /// via CoW swap (same shape as manifest_scopes).
    pub spirit_postures: HashMap<u32, crate::security::posture::PostureState>,
}

/// What [`PolicyTable::narrow_scopes`] changed, kept so the change can be undone.
#[derive(Debug, Clone, PartialEq, Eq)]
#[maos_attrs::i9_exempt(
    reason = "exact per-swap undo record; bounded by the predecessor and installed scope sets, released with the rollback window"
)]
pub(crate) struct ScopeInstall {
    previous: Vec<Scope>,
    installed: Vec<Scope>,
}

/// Why a successor scope set cannot be installed on a PID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScopeInstallError {
    /// The PID has no admitted policy entry; a swap never recreates one.
    UnknownPid,
    /// The successor declares a scope its predecessor neither holds nor entitles.
    NotEntitled(Scope),
}

fn scopes_within(held: &[Scope], successor: &[Scope]) -> Result<(), ScopeInstallError> {
    match successor
        .iter()
        .find(|s| !held.iter().any(|d| d == *s || d.entitles(s)))
    {
        Some(extra) => Err(ScopeInstallError::NotEntitled(extra.clone())),
        None => Ok(()),
    }
}

fn same_scope_set(a: &[Scope], b: &[Scope]) -> bool {
    a.iter().all(|s| b.contains(s)) && b.iter().all(|s| a.contains(s))
}

/// Policy table — read-mostly copy-on-write.
///
/// Readers take a single atomic load (free); writers swap the entire
/// `Arc<PolicyTableInner>` at runtime.
#[derive(Debug)]
#[maos_attrs::i9_exempt(reason = "operator policy table; structural-state caching per I9")]
pub struct PolicyTable {
    inner: Arc<ArcSwap<PolicyTableInner>>,
}

impl PolicyTable {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(ArcSwap::from_pointee(PolicyTableInner::default())),
        }
    }

    /// Access the inner ArcSwap (for SecurityManagerAdapter admission).
    pub fn inner(&self) -> &Arc<ArcSwap<PolicyTableInner>> {
        &self.inner
    }

    pub fn evaluate(
        &self,
        spirit_pid: u32,
        capability: &Capability,
        intent: &Intent,
    ) -> PolicyDecision {
        // Story 4.3 — FR56 self-telemetry always-allow rule.
        // Self-telemetry is a positive always-allow — the Spirit reads
        // its own data without per-read operator admission.  The rule is
        // enumerable so operators can audit the policy table and see the
        // self-telemetry cap-class explicitly (NFR-Aud-1).
        if matches!(
            capability.scope,
            maos_domain::invariants::i1::Scope::SelfTelemetryRead
        ) || matches!(intent, Intent::SelfTelemetryRead)
        {
            return PolicyDecision::Allow;
        }

        let inner = self.inner.load_full();

        // Story 11.4a (F2, FLAG-Winston) — org forbid beats every grant
        // (Cedar forbid-beats-permit). Populated out-of-kernel by the PDP
        // adapter at the composition root; subtract-only (I1, kernel ceiling).
        let deny_scope_key = maos_domain::ports::scope_action_key(&capability.scope);
        let deny_intent_key = intent_action_key(intent);
        let global_deny = &inner.operator_policy.per_capability_deny;
        let spirit_deny = inner
            .operator_policy
            .per_spirit_capability_deny
            .get(&spirit_pid);
        if global_deny.contains(deny_scope_key)
            || global_deny.contains(deny_intent_key)
            || spirit_deny
                .is_some_and(|deny| deny.contains(deny_scope_key) || deny.contains(deny_intent_key))
        {
            return PolicyDecision::Deny;
        }

        // Fail-closed: unknown Spirits are denied.
        let manifest = match inner.manifest_scopes.get(&spirit_pid) {
            Some(m) => m,
            None => return PolicyDecision::Deny,
        };

        // Scope-match check (Story 17-11): a declared manifest scope must
        // entitle the requested scope, selector included (`Scope::entitles`).
        let scope_match = manifest
            .scopes
            .iter()
            .any(|s| s.entitles(&capability.scope));
        if !scope_match {
            return PolicyDecision::Deny;
        }

        // Effective sandbox tier: strictest of manifest, trust-tier-floor, operator.
        let effective = self.effective_sandbox_tier(spirit_pid, manifest.trust_tier, &inner);
        if effective.0 > 3 {
            return PolicyDecision::Deny;
        }

        // Story 4.4 — default approval-class rules for new capabilities.
        // LogRecall / LogFetch → AutonomousWithHalt (normal admission under halt protocol).
        // DistillateWrite → Assistive (prompt-on-write per architecture §F.4).
        use maos_domain::invariants::i1::Scope;
        match &capability.scope {
            Scope::DistillateWrite => {
                return PolicyDecision::RequireApproval {
                    class: ApprovalClass::Interactive,
                };
            }
            Scope::LogRecall | Scope::LogFetch => {
                // Normal admission — halt protocol is enforced at the caller layer.
                // Falls through to Allow below.
            }
            Scope::SkillAuthorSelf => {
                // Story 7.4 — FR39/FR57 `skill.author.self`. This arm is reached
                // ONLY after the scope-match above confirmed the Spirit DECLARES
                // `skill.author.self` in its manifest scopes — i.e. UNLIKE
                // `SelfTelemetryRead` (the top-of-function always-allow), this
                // capability is NOT always-allow: a Spirit that does not declare
                // it is denied at the scope-match gate. The capability grants ONLY
                // the write-to-queue (a Spirit may author a skill into the PENDING
                // operator-admission queue); it does NOT auto-admit the skill.
                // Activation still requires the FR39 operator-admission path
                // (`SkillAdmissionQueue::approve`). Falls through to Allow below
                // (the write is permitted; the queue gates activation).
            }
            _ => {}
        }

        // Approval-class lookup from operator policy (by stable scope key
        // and by stable intent key).
        let scope_key = maos_domain::ports::scope_action_key(&capability.scope);
        if let Some(class) = inner.operator_policy.per_capability_approval.get(scope_key) {
            return PolicyDecision::RequireApproval { class: *class };
        }
        let intent_key = intent_action_key(intent);
        if let Some(class) = inner
            .operator_policy
            .per_capability_approval
            .get(intent_key)
        {
            return PolicyDecision::RequireApproval { class: *class };
        }

        PolicyDecision::Allow
    }

    /// Compute the effective sandbox tier: strictest of (manifest,
    /// trust-tier-floor for the Spirit's trust tier, operator-policy).
    pub fn effective_sandbox_tier(
        &self,
        spirit_pid: u32,
        trust_tier: decision::TrustTier,
        inner: &PolicyTableInner,
    ) -> SandboxTier {
        let manifest = inner
            .manifest_scopes
            .get(&spirit_pid)
            .map(|m| m.declared_tier)
            .unwrap_or(SandboxTier::DEFAULT_FLOOR);
        let trust = inner
            .trust_tier_floor
            .get(&trust_tier)
            .copied()
            .unwrap_or(SandboxTier::DEFAULT_FLOOR);
        let operator = inner
            .operator_policy
            .spirit_tier_floor
            .get(&spirit_pid)
            .copied()
            .unwrap_or(inner.operator_policy.global_sandbox_floor);
        strictest_of(manifest, trust, operator)
    }

    /// Update the policy table atomically (CoW swap).
    pub fn update(&self, new_policy: PolicyTableInner) {
        self.inner.store(Arc::new(new_policy));
    }

    /// Retry-safe read-modify-write of the policy table (ArcSwap RCU).
    ///
    /// `f` receives a private clone of the CURRENT snapshot and may run more
    /// than once if another writer lands first, so it must be pure (no I/O,
    /// no audit emission) and recompute everything from its argument. Use
    /// this instead of `inner().load_full()` + `update()`, which can undo a
    /// concurrent change such as a same-PID scope narrowing.
    pub fn mutate<R>(&self, mut f: impl FnMut(&mut PolicyTableInner) -> R) -> R {
        let mut result = None;
        self.inner.rcu(|current| {
            let mut next = (**current).clone();
            result = Some(f(&mut next));
            Arc::new(next)
        });
        // rcu always runs the closure at least once.
        result.expect("rcu closure executes at least once")
    }

    /// Whether the Spirit's CURRENT manifest scopes still entitle `scope`.
    ///
    /// Defensive re-check for holders of a capability minted before a
    /// same-PID scope swap. Uses the same `Scope::entitles` match as
    /// [`Self::evaluate`] (selector included) plus the implicit
    /// always-allowed `SelfTelemetryRead`. Lock-free read; an unadmitted PID
    /// entitles nothing but the implicit scope. Operator deny/approval
    /// policy is deliberately not consulted.
    pub fn still_entitles(&self, spirit_pid: u32, scope: &Scope) -> bool {
        if matches!(scope, Scope::SelfTelemetryRead) {
            return true;
        }
        self.inner
            .load()
            .manifest_scopes
            .get(&spirit_pid)
            .is_some_and(|m| m.scopes.iter().any(|s| s.entitles(scope)))
    }

    /// Preflight of [`Self::narrow_scopes`]: `Ok(())` exactly when the PID is
    /// admitted and every successor scope is held (or entitled) by its current
    /// effective scopes. Pure; mutates nothing.
    pub(crate) fn check_scope_narrowing(
        &self,
        spirit_pid: u32,
        successor: &[Scope],
    ) -> Result<(), ScopeInstallError> {
        let inner = self.inner.load();
        let entry = inner
            .manifest_scopes
            .get(&spirit_pid)
            .ok_or(ScopeInstallError::UnknownPid)?;
        scopes_within(&entry.scopes, successor)
    }

    /// Hot-swap commit (Story 21-9, D-21-PF-14): replace an ALREADY-admitted
    /// PID's effective scopes with the successor's declared set, atomically,
    /// only if it never widens them. An unknown PID is refused (never
    /// recreated) and a scope neither held nor entitled by the current set is
    /// refused. `Ok(None)` ⇒ the set is unchanged and nothing was written
    /// (tier, trust and posture are never touched either way). `Ok(Some(_))`
    /// carries what is needed to undo it with [`Self::restore_scopes`].
    pub(crate) fn narrow_scopes(
        &self,
        spirit_pid: u32,
        successor: &[Scope],
    ) -> Result<Option<ScopeInstall>, ScopeInstallError> {
        let mut outcome = Err(ScopeInstallError::UnknownPid);
        self.inner.rcu(|current| {
            let Some(entry) = current.manifest_scopes.get(&spirit_pid) else {
                outcome = Err(ScopeInstallError::UnknownPid);
                return Arc::clone(current);
            };
            if let Err(refused) = scopes_within(&entry.scopes, successor) {
                outcome = Err(refused);
                return Arc::clone(current);
            }
            if same_scope_set(&entry.scopes, successor) {
                outcome = Ok(None);
                return Arc::clone(current);
            }
            outcome = Ok(Some(ScopeInstall {
                previous: entry.scopes.clone(),
                installed: successor.to_vec(),
            }));
            let mut next = (**current).clone();
            if let Some(slot) = next.manifest_scopes.get_mut(&spirit_pid) {
                slot.scopes = successor.to_vec();
            }
            Arc::new(next)
        });
        outcome
    }

    /// Undo [`Self::narrow_scopes`] (matching auto-revert or a failed publish):
    /// restores the predecessor's scopes only while the PID is still admitted
    /// and still holds exactly the set that install wrote. Returns whether it
    /// restored; an unloaded PID is never recreated.
    pub(crate) fn restore_scopes(&self, spirit_pid: u32, install: &ScopeInstall) -> bool {
        let mut restored = false;
        self.inner.rcu(|current| {
            restored = false;
            let held_exactly = current
                .manifest_scopes
                .get(&spirit_pid)
                .is_some_and(|entry| same_scope_set(&entry.scopes, &install.installed));
            if !held_exactly {
                return Arc::clone(current);
            }
            restored = true;
            let mut next = (**current).clone();
            if let Some(slot) = next.manifest_scopes.get_mut(&spirit_pid) {
                slot.scopes = install.previous.clone();
            }
            Arc::new(next)
        });
        restored
    }

    /// Replace the operator posture ceiling and immediately re-clamp every
    /// admitted Spirit so a live policy change cannot leave stale authority.
    /// Story 16-6 review 2026-09-20.
    pub fn set_operator_posture_ceiling(
        &self,
        ceiling: Option<crate::security::manifest::Posture>,
    ) {
        self.mutate(|next| {
            next.operator_policy.operator_posture_ceiling = ceiling;
            for state in next.spirit_postures.values_mut() {
                clamp_posture_state(state, ceiling);
            }
        });
    }

    /// Atomically shift a Spirit's posture under the ceiling constraint
    /// (Story 3.2, AC4).
    pub fn shift_posture(
        &self,
        spirit_pid: u32,
        new_posture: crate::security::manifest::Posture,
    ) -> Result<[u8; 32], crate::security::posture::PostureError> {
        use crate::security::manifest::Posture;
        use crate::security::posture::PostureError;

        // Reject non-runtime postures
        if new_posture == Posture::Autonomous {
            return Err(PostureError::NonRuntimePosture(Posture::Autonomous));
        }

        // Retry-safe RCU: validation and the swap both read the same current
        // snapshot, so a concurrent writer (e.g. scope narrowing) is never
        // overwritten by a stale clone. The closure is pure and recomputed
        // on every retry.
        let mut outcome = Err(PostureError::UnknownSpirit(spirit_pid));
        self.inner.rcu(|current| {
            let Some(state) = current.spirit_postures.get(&spirit_pid) else {
                outcome = Err(PostureError::UnknownSpirit(spirit_pid));
                return Arc::clone(current);
            };
            if new_posture > state.allowed_max {
                outcome = Err(PostureError::AboveCeiling {
                    requested: new_posture,
                    allowed: state.allowed_max,
                });
                return Arc::clone(current);
            }
            let mut new_inner = (**current).clone();
            let state = new_inner
                .spirit_postures
                .get_mut(&spirit_pid)
                .expect("the cloned snapshot contains the checked posture");
            state.current = new_posture;
            outcome = Ok(state.posture_hash());
            Arc::new(new_inner)
        });

        outcome
    }

    /// Posture-aware evaluation (Story 3.2, AC5).
    pub fn evaluate_with_posture(
        &self,
        spirit_pid: u32,
        base_class: maos_domain::notification::ApprovalClass,
    ) -> PolicyDecision {
        let inner = self.inner.load_full();

        // Step 1: Fail-closed — unknown spirit
        let state = match inner.spirit_postures.get(&spirit_pid) {
            Some(s) => s,
            None => return PolicyDecision::Deny,
        };

        // Step 2: ControlPlane always requires approval
        if base_class == maos_domain::notification::ApprovalClass::ControlPlane {
            return PolicyDecision::RequireApproval {
                class: domain_class_to_decision(base_class),
            };
        }

        // Step 3: Posture × class matrix lookup
        let requires =
            crate::security::posture::posture_requires_approval(state.current, base_class);
        let matrix_decision = if requires {
            PolicyDecision::RequireApproval {
                class: domain_class_to_decision(base_class),
            }
        } else {
            PolicyDecision::Allow
        };

        // Step 4: Operator policy overrides take precedence (existing evaluate semantics)
        let scope_key = format!("{:?}", std::mem::discriminant(&base_class));
        if let Some(class) = inner
            .operator_policy
            .per_capability_approval
            .get(&scope_key)
        {
            return PolicyDecision::RequireApproval { class: *class };
        }

        matrix_decision
    }
}

/// Clamp both persisted runtime posture fields under the operator's ceiling.
///
/// Admission and a live operator-policy update must share this rule: clamping
/// only `allowed_max` can leave `current` above its own ceiling.
pub(crate) fn clamp_posture_state(
    state: &mut crate::security::posture::PostureState,
    ceiling: Option<crate::security::manifest::Posture>,
) {
    if let Some(limit) = ceiling {
        state.current = state.current.min(limit);
        state.allowed_max = state.allowed_max.min(limit);
    }
}

/// Strictest of three sandbox tiers.
pub fn strictest_of(a: SandboxTier, b: SandboxTier, c: SandboxTier) -> SandboxTier {
    SandboxTier(a.0.max(b.0).max(c.0))
}

/// Map domain ApprovalClass → kernel decision::ApprovalClass.
fn domain_class_to_decision(
    c: maos_domain::notification::ApprovalClass,
) -> decision::ApprovalClass {
    match c {
        maos_domain::notification::ApprovalClass::ReadonlyScoped => {
            decision::ApprovalClass::ReadonlyScoped
        }
        maos_domain::notification::ApprovalClass::ReadonlySearch => {
            decision::ApprovalClass::ReadonlySearch
        }
        maos_domain::notification::ApprovalClass::Mutating => decision::ApprovalClass::Mutating,
        maos_domain::notification::ApprovalClass::ExecCapable => {
            decision::ApprovalClass::ExecCapable
        }
        maos_domain::notification::ApprovalClass::ControlPlane => {
            decision::ApprovalClass::ControlPlane
        }
        maos_domain::notification::ApprovalClass::Interactive => {
            decision::ApprovalClass::Interactive
        }
    }
}

fn intent_action_key(intent: &Intent) -> &'static str {
    match intent {
        Intent::FsRead { .. } => "fs.read",
        Intent::FsWrite { .. } => "fs.write",
        Intent::NetHttps { .. } => "net.https",
        Intent::ProcExec { .. } => "proc.exec",
        Intent::SubSpiritSpawn { .. } => "subspirit.spawn",
        Intent::ProviderInfer { .. } => "provider.infer",
        Intent::IacSend { .. } => "iac.send",
        Intent::MemRead { .. } => "mem.read",
        Intent::MemWrite { .. } => "mem.write",
        Intent::SelfTelemetryRead => "self.telemetry.read",
        Intent::LogRecall => "log.recall",
        Intent::LogFetch => "log.fetch",
        Intent::DistillateWrite => "distillate.write",
        Intent::SkillAuthorSelf => "skill.author.self",
        Intent::McpCall { .. } => "mcp.call",
        Intent::LoomRead => "loom.read",
        Intent::LoomWrite => "loom.write",
        Intent::LoomScan => "loom.scan",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Production code no longer names `TrustTier` directly (the shared
    // posture-clamp helper takes `Posture`), but these tests assert tier
    // plumbing end to end. `decision` re-exports the registry-side tier type
    // these tests have always used — NOT `maos_spirit_abi::compliance`.
    use super::decision::TrustTier;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn strictest_of_three() {
        assert_eq!(
            strictest_of(SandboxTier(0), SandboxTier(1), SandboxTier(2)),
            SandboxTier(2)
        );
        assert_eq!(
            strictest_of(SandboxTier(2), SandboxTier(0), SandboxTier(0)),
            SandboxTier(2)
        );
    }

    #[test]
    fn policy_table_update_is_atomic() {
        let table = PolicyTable::new();
        let mut new_inner = PolicyTableInner::default();
        new_inner
            .operator_policy
            .spirit_tier_floor
            .insert(7, SandboxTier(2));
        table.update(new_inner);
        let loaded = table.inner.load_full();
        assert_eq!(
            loaded.operator_policy.spirit_tier_floor.get(&7),
            Some(&SandboxTier(2))
        );
    }

    #[test]
    fn strictest_of_floor_forces_t0_to_t2_for_public_untrusted() {
        let table = PolicyTable::new();
        let mut inner = PolicyTableInner::default();
        inner.manifest_scopes.insert(
            42,
            ManifestCapabilityScope {
                scopes: vec![Scope::FsRead {
                    subtree: "/tmp".into(),
                }],
                declared_tier: SandboxTier(0),
                trust_tier: TrustTier::PublicUntrusted,
            },
        );
        inner
            .trust_tier_floor
            .insert(TrustTier::PublicUntrusted, SandboxTier(2));
        table.update(inner);

        let loaded = table.inner.load_full();
        let effective = table.effective_sandbox_tier(42, TrustTier::PublicUntrusted, &loaded);
        assert_eq!(
            effective,
            SandboxTier(2),
            "trust tier floor must force T0→T2 for PublicUntrusted"
        );
    }

    #[test]
    fn operator_floor_can_force_above_manifest_and_trust_tier() {
        let table = PolicyTable::new();
        let mut inner = PolicyTableInner::default();
        inner.manifest_scopes.insert(
            42,
            ManifestCapabilityScope {
                scopes: vec![Scope::FsRead {
                    subtree: "/tmp".into(),
                }],
                declared_tier: SandboxTier(0),
                trust_tier: TrustTier::Verified,
            },
        );
        inner
            .trust_tier_floor
            .insert(TrustTier::Verified, SandboxTier(0));
        inner
            .operator_policy
            .spirit_tier_floor
            .insert(42, SandboxTier(3));
        table.update(inner);

        let loaded = table.inner.load_full();
        let effective = table.effective_sandbox_tier(42, TrustTier::Verified, &loaded);
        assert_eq!(
            effective,
            SandboxTier(3),
            "operator floor must override manifest+trust"
        );
    }

    #[test]
    fn concurrent_readers_never_block_writer() {
        let table = Arc::new(PolicyTable::new());
        let mut inner = PolicyTableInner::default();
        inner.manifest_scopes.insert(
            1,
            ManifestCapabilityScope {
                scopes: vec![Scope::FsRead {
                    subtree: "/tmp".into(),
                }],
                declared_tier: SandboxTier(0),
                trust_tier: TrustTier::Verified,
            },
        );
        table.update(inner);

        let writer_table = table.clone();
        let writer = thread::spawn(move || {
            for i in 0..100 {
                let mut new_inner = PolicyTableInner::default();
                new_inner.manifest_scopes.insert(
                    1,
                    ManifestCapabilityScope {
                        scopes: vec![Scope::FsRead {
                            subtree: format!("/tmp/{i}"),
                        }],
                        declared_tier: SandboxTier(0),
                        trust_tier: TrustTier::Verified,
                    },
                );
                writer_table.update(new_inner);
            }
        });

        let reader_table = table.clone();
        let reader = thread::spawn(move || {
            for _ in 0..10_000 {
                let inner = reader_table.inner.load_full();
                assert!(inner.manifest_scopes.contains_key(&1));
            }
        });

        writer.join().unwrap();
        reader.join().unwrap();
    }

    #[test]
    fn skill_author_self_requires_manifest_declaration_not_always_allow() {
        use maos_domain::invariants::i1::Scope;
        let table = PolicyTable::new();
        let mut inner = PolicyTableInner::default();
        // Spirit 7 DECLARES skill.author.self; Spirit 8 declares only FsRead.
        inner.manifest_scopes.insert(
            7,
            ManifestCapabilityScope {
                scopes: vec![Scope::SkillAuthorSelf],
                declared_tier: SandboxTier(0),
                trust_tier: TrustTier::Verified,
            },
        );
        inner.manifest_scopes.insert(
            8,
            ManifestCapabilityScope {
                scopes: vec![Scope::FsRead {
                    subtree: "/tmp".into(),
                }],
                declared_tier: SandboxTier(0),
                trust_tier: TrustTier::Verified,
            },
        );
        table.update(inner);

        let cap = Capability {
            scope: Scope::SkillAuthorSelf,
        };
        let intent = Intent::FsRead {
            subtree: "n/a".into(),
        };

        // Declaring Spirit → Allow: the write-to-queue is authorized.
        assert_eq!(table.evaluate(7, &cap, &intent), PolicyDecision::Allow);
        // Non-declaring Spirit → Deny: scope-match fails.
        assert_eq!(table.evaluate(8, &cap, &intent), PolicyDecision::Deny);
        // Unknown Spirit → Deny. CRITICAL contrast with SelfTelemetryRead, which
        // is always-allow EVEN for an unknown Spirit: SkillAuthorSelf is NOT
        // always-allow — it is fail-closed and requires manifest declaration.
        assert_eq!(table.evaluate(999, &cap, &intent), PolicyDecision::Deny);
        let self_tel = Capability {
            scope: Scope::SelfTelemetryRead,
        };
        assert_eq!(
            table.evaluate(999, &self_tel, &intent),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn evaluate_with_posture_returns_deny_for_unknown_spirit() {
        use crate::security::manifest::{EpistemicAction, EpistemicPolicySection, Posture};
        use crate::security::posture::PostureState;

        let table = PolicyTable::new();
        let decision =
            table.evaluate_with_posture(999, maos_domain::notification::ApprovalClass::Mutating);
        assert!(matches!(decision, PolicyDecision::Deny));
    }

    #[test]
    fn evaluate_with_posture_operator_override_takes_precedence() {
        use crate::security::manifest::{EpistemicAction, EpistemicPolicySection, Posture};
        use crate::security::posture::PostureState;

        let table = PolicyTable::new();
        let mut inner = PolicyTableInner::default();
        inner.spirit_postures.insert(
            0,
            PostureState {
                current: Posture::AutonomousWithHalt,
                allowed_max: Posture::AutonomousWithHalt,
                epistemic_policy: EpistemicPolicySection {
                    rules: vec![],
                    default_action: EpistemicAction::VerbalizeOnly,
                },
            },
        );
        // AutonomousWithHalt + Mutating = Allow per matrix, but operator
        // override forces RequireApproval
        inner.operator_policy.per_capability_approval.insert(
            format!(
                "{:?}",
                std::mem::discriminant(&maos_domain::notification::ApprovalClass::Mutating)
            ),
            decision::ApprovalClass::Mutating,
        );
        table.update(inner);

        let decision =
            table.evaluate_with_posture(0, maos_domain::notification::ApprovalClass::Mutating);
        assert!(
            matches!(decision, PolicyDecision::RequireApproval { .. }),
            "operator override must take precedence over matrix Allow"
        );
    }

    #[test]
    fn concurrent_policy_write_preserves_the_committed_narrowing() {
        let table = PolicyTable::new();
        let fs = Scope::FsRead {
            subtree: "/tmp".into(),
        };
        table.mutate(|next| {
            next.manifest_scopes.insert(
                9,
                ManifestCapabilityScope {
                    scopes: vec![fs.clone(), Scope::LogRecall],
                    declared_tier: SandboxTier(0),
                    trust_tier: TrustTier::Verified,
                },
            );
        });
        assert!(table.still_entitles(9, &fs));
        assert!(table.still_entitles(9, &Scope::LogRecall));

        // Hold the writer after it has read the wider snapshot, commit the narrowing,
        // then let the writer finish. Its policy change and the narrowing must both land.
        std::thread::scope(|scope| {
            let (ready, pending) = std::sync::mpsc::sync_channel(1);
            let (release, resume) = std::sync::mpsc::sync_channel(1);
            let watchdog = std::time::Duration::from_secs(10);
            let writer_table = &table;
            let writer = scope.spawn(move || {
                let mut first = true;
                writer_table.mutate(|next| {
                    if std::mem::replace(&mut first, false) {
                        ready.send(()).unwrap();
                        resume.recv_timeout(watchdog).unwrap();
                    }
                    next.operator_policy.global_sandbox_floor = SandboxTier(1);
                });
            });
            pending.recv_timeout(watchdog).unwrap();
            let result = table.narrow_scopes(9, std::slice::from_ref(&fs));
            release.send(()).unwrap();
            writer.join().unwrap();
            assert!(result.unwrap().is_some());
        });
        assert_eq!(
            table.inner.load().operator_policy.global_sandbox_floor,
            SandboxTier(1)
        );
        assert!(!table.still_entitles(9, &Scope::LogRecall));

        // Implicit scope needs no manifest entry; unknown PIDs get nothing else.
        assert!(table.still_entitles(9, &Scope::SelfTelemetryRead));
        assert!(table.still_entitles(404, &Scope::SelfTelemetryRead));
        assert!(!table.still_entitles(404, &fs));
    }
}
