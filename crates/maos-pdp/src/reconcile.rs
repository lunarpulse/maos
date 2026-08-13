//! Story 11.4a composition-root reconciler — bridges the `PolicyDecisionPort`
//! verdicts to the kernel's bounded `OperatorPolicyConfig.per_capability_deny`
//! forbid layer (F2).
//!
//! The reconciler runs OFF-HOT-PATH at daemon startup and on the daemon
//! refresh cadence (ADR-030: the PDP is NEVER called from the token-verify hot
//! path). It submits the governed capability set to the engine, collects the
//! `Deny` verdicts, and returns their stable action keys for the composition
//! root to materialize into `per_capability_deny` via the public CoW
//! `PolicyTable::update()`.
//!
//! # Fail-closed (F4)
//!
//! A configured-but-unreachable PDP MUST fail closed. At startup the caller
//! materializes [`all_governed_deny_keys`]. After a fresh reconciliation, a
//! runtime drop freezes the last-known-good global and subject-scoped denies
//! until the TTL expires; after TTL expiry the reconciler returns all governed
//! deny keys to close the stale-permit/revocation window.

use std::collections::{HashMap, HashSet};

use maos_domain::invariants::i1::Scope;
use maos_domain::ports::{
    scope_action_key, PolicyDecisionError, PolicyDecisionPort, PolicyDecisionRequest, PolicyVerdict,
};

/// The stable action key the kernel uses for `per_capability_deny` /
/// `per_capability_approval` and the Cedar adapter uses as `Action` id.
pub fn scope_deny_key(scope: &Scope) -> String {
    scope_action_key(scope).to_string()
}

/// The representative kernel `Scope` variants the PDP reconciler submits to
/// the engine. One per variant (fields are placeholders — the PDP keys on the
/// stable action vocabulary only). `SelfTelemetryRead` is EXCLUDED: it is a
/// non-overridable FR56 kernel invariant (the Spirit reads its own telemetry),
/// so the operator cannot govern it via the PDP.
pub fn representative_governed_scopes() -> Vec<Scope> {
    use Scope::*;
    vec![
        FsRead {
            subtree: String::new(),
        },
        FsWrite {
            subtree: String::new(),
        },
        NetHttps {
            domain: String::new(),
        },
        ProcExec {
            binary: String::new(),
        },
        SubSpiritSpawn {
            class: String::new(),
        },
        ProviderInfer {
            provider: String::new(),
        },
        IacSend {
            peer_class: String::new(),
        },
        MemRead {
            scope: String::new(),
        },
        MemWrite {
            scope: String::new(),
        },
        LogRecall,
        LogFetch,
        DistillateWrite,
        McpCall {
            server: String::new(),
            tool: String::new(),
        },
        CliSubprocessSpawn {
            cli_binary_path: String::new(),
            argv_prefix_hash: [0u8; 32],
            output_shape_version: String::new(),
        },
        GatewaySend {
            gateway_id: String::new(),
            recipient: String::new(),
        },
        SkillAuthorSelf,
        LoomRead,
        LoomWrite,
        LoomScan,
    ]
}

/// Stable action keys for EVERY governed capability. Used by the composition
/// root's fail-closed path (F4): a configured-but-unreachable PDP materializes
/// ALL of these into `per_capability_deny` (deny every governed capability),
/// never relaxing to permissive defaults.
pub fn all_governed_deny_keys() -> Vec<String> {
    representative_governed_scopes()
        .iter()
        .map(scope_deny_key)
        .collect()
}

/// Reconcile the operator policy via `adapter` over the governed capability
/// set, returning the stable action keys the PDP forbids (to materialize into
/// `per_capability_deny`). The verdict count is derived per-request from
/// actual engine calls and reconciled against the governed-set cardinality
/// (derive-and-reconcile — never a committed literal).
///
/// Returns `Err` if the engine is unreachable; the caller fail-closes
/// (materializes [`all_governed_deny_keys`]).
pub fn reconcile_org_denies(
    adapter: &dyn PolicyDecisionPort,
) -> Result<Vec<String>, PolicyDecisionError> {
    let scopes = representative_governed_scopes();
    let requests: Vec<PolicyDecisionRequest> = scopes
        .iter()
        .map(|s| PolicyDecisionRequest {
            // spirit_pid is irrelevant for an org-wide (ceiling) forbid — the
            // Cedar `forbid(principal, action, resource)` rule applies to all
            // principals; the materialized key is global in `per_capability_deny`.
            spirit_pid: 0,
            capability_key: scope_deny_key(s),
            principal_attributes: None,
        })
        .collect();
    let cardinality = requests.len();
    let verdicts = adapter.evaluate(&requests)?;
    if verdicts.len() != cardinality {
        return Err(PolicyDecisionError::Transport(format!(
            "PDP returned {} verdicts for {} requests",
            verdicts.len(),
            cardinality
        )));
    }
    // Derive-and-reconcile: one verdict per governed capability.
    let denies: Vec<String> = scopes
        .iter()
        .zip(verdicts.iter())
        .filter(|(_, v)| **v == PolicyVerdict::Deny)
        .map(|(s, _)| scope_deny_key(s))
        .collect();
    Ok(denies)
}

/// Materialized PDP denies split into global and per-spirit restrictions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MaterializedDenies {
    pub global: HashSet<String>,
    pub per_spirit: HashMap<u32, HashSet<String>>,
}

/// Reconcile a Cedar policy for a concrete set of known Spirit subjects.
///
/// Returns an empty materialization without calling the adapter when
/// `spirit_pids` is empty; that lets the composition root report "0 subjects"
/// honestly without manufacturing a successful subject-policy evaluation.
/// Global Cedar denies that apply to every supplied Spirit are also surfaced in
/// [`MaterializedDenies::global`] so callers can intentionally fold them into
/// the kernel's global deny set instead of carrying a dead field.
pub fn reconcile_subject_denies(
    adapter: &dyn PolicyDecisionPort,
    spirit_pids: &[u32],
) -> Result<MaterializedDenies, PolicyDecisionError> {
    if spirit_pids.is_empty() {
        return Ok(MaterializedDenies::default());
    }

    let scopes = representative_governed_scopes();
    let mut requests = Vec::with_capacity(scopes.len() * spirit_pids.len());
    let mut positions = Vec::with_capacity(scopes.len() * spirit_pids.len());
    for pid in spirit_pids {
        for scope in &scopes {
            let key = scope_deny_key(scope);
            positions.push((*pid, key.clone()));
            requests.push(PolicyDecisionRequest {
                spirit_pid: *pid,
                capability_key: key,
                principal_attributes: None,
            });
        }
    }
    let verdicts = adapter.evaluate(&requests)?;
    if verdicts.len() != requests.len() {
        return Err(PolicyDecisionError::Transport(format!(
            "PDP returned {} verdicts for {} subject requests",
            verdicts.len(),
            requests.len()
        )));
    }
    let mut materialized = MaterializedDenies::default();
    for ((pid, key), verdict) in positions.into_iter().zip(verdicts.into_iter()) {
        if verdict == PolicyVerdict::Deny {
            materialized.per_spirit.entry(pid).or_default().insert(key);
        }
    }
    for key in all_governed_deny_keys() {
        if spirit_pids.iter().all(|pid| {
            materialized
                .per_spirit
                .get(pid)
                .is_some_and(|denies| denies.contains(&key))
        }) {
            materialized.global.insert(key);
        }
    }
    Ok(materialized)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailClosedPosture {
    Fresh,
    StartupClosed,
    RuntimeFreeze,
    TtlExpiredRevert,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailClosedOutcome {
    pub deny_keys: Vec<String>,
    pub subject_denies: MaterializedDenies,
    pub posture: FailClosedPosture,
}

#[derive(Debug, Clone)]
pub struct FailClosedReconciler {
    ttl_nanos: u64,
    last_good: Option<Vec<String>>,
    last_good_subject_denies: Option<MaterializedDenies>,
    last_success_nanos: Option<u64>,
}

impl FailClosedReconciler {
    pub fn new(ttl_nanos: u64) -> Self {
        Self {
            ttl_nanos,
            last_good: None,
            last_good_subject_denies: None,
            last_success_nanos: None,
        }
    }

    pub fn reconcile_at(
        &mut self,
        adapter: &dyn PolicyDecisionPort,
        now_nanos: u64,
    ) -> FailClosedOutcome {
        self.reconcile_with_subjects_at(adapter, &[], now_nanos)
    }

    pub fn reconcile_with_subjects_at(
        &mut self,
        adapter: &dyn PolicyDecisionPort,
        spirit_pids: &[u32],
        now_nanos: u64,
    ) -> FailClosedOutcome {
        match reconcile_org_denies(adapter).and_then(|deny_keys| {
            reconcile_subject_denies(adapter, spirit_pids).map(|s| (deny_keys, s))
        }) {
            Ok((deny_keys, subject_denies)) => {
                self.last_good = Some(deny_keys.clone());
                self.last_good_subject_denies = Some(subject_denies.clone());
                self.last_success_nanos = Some(now_nanos);
                FailClosedOutcome {
                    deny_keys,
                    subject_denies,
                    posture: FailClosedPosture::Fresh,
                }
            }
            Err(_) => match (
                &self.last_good,
                &self.last_good_subject_denies,
                self.last_success_nanos,
            ) {
                (Some(keys), Some(subject_denies), Some(last))
                    if now_nanos.saturating_sub(last) <= self.ttl_nanos =>
                {
                    FailClosedOutcome {
                        deny_keys: keys.clone(),
                        subject_denies: subject_denies.clone(),
                        posture: FailClosedPosture::RuntimeFreeze,
                    }
                }
                (Some(_), _, Some(_)) => FailClosedOutcome {
                    deny_keys: all_governed_deny_keys(),
                    subject_denies: MaterializedDenies::default(),
                    posture: FailClosedPosture::TtlExpiredRevert,
                },
                _ => FailClosedOutcome {
                    deny_keys: all_governed_deny_keys(),
                    subject_denies: MaterializedDenies::default(),
                    posture: FailClosedPosture::StartupClosed,
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CedarPolicyAdapter;

    #[test]
    fn governed_scopes_exclude_self_telemetry() {
        // FR56 SelfTelemetryRead is a non-overridable kernel invariant — the
        // PDP reconciler must NOT submit it (the operator cannot govern it).
        let scopes = representative_governed_scopes();
        assert!(
            !scopes.iter().any(|s| matches!(s, Scope::SelfTelemetryRead)),
            "SelfTelemetryRead must be excluded from the PDP-governed set"
        );
        assert!(
            scopes.len() >= 16,
            "governed set covers the kernel Scope variants: {}",
            scopes.len()
        );
    }
    #[cfg(not(feature = "pdp-fault-inject"))]
    #[test]
    fn reconcile_collects_only_forbids() {
        // Operator pattern: PERMIT all, FORBID specific (Cedar `forbid` beats
        // `permit`). ProcExec + LoomWrite are forbidden; the rest are permitted
        // → the reconciler materializes exactly those two deny keys.
        let adapter = CedarPolicyAdapter::new();
        let procexec_key = scope_deny_key(&Scope::ProcExec {
            binary: String::new(),
        });
        let loomwrite_key = scope_deny_key(&Scope::LoomWrite);
        let policy = format!(
            r#"
            permit(principal, action, resource);
            forbid(principal, action == Action::"{procexec_key}", resource);
            forbid(principal, action == Action::"{loomwrite_key}", resource);
            "#
        );
        adapter.load_policy(&policy).unwrap();
        let denies = reconcile_org_denies(&adapter).unwrap();
        assert_eq!(denies.len(), 2, "exactly the two forbids materialize");
        assert!(denies.contains(&procexec_key));
        assert!(denies.contains(&loomwrite_key));
    }

    #[cfg(not(feature = "pdp-fault-inject"))]
    #[test]
    fn empty_policy_denies_all_governed_cedar_default_deny() {
        // Cedar default-deny: an empty policy (no permits) denies everything.
        // This is the "I forgot to permit anything" lockdown — maximally
        // restrictive (fail-closed-ish), NOT a silent permissive pass. The
        // operator MUST write explicit `permit` rules to allow capabilities.
        let adapter = CedarPolicyAdapter::new();
        adapter.load_policy("").unwrap();
        let denies = reconcile_org_denies(&adapter).unwrap();
        assert_eq!(
            denies.len(),
            representative_governed_scopes().len(),
            "empty policy → Cedar default-deny → ALL governed capabilities denied"
        );
    }

    struct ShortPort;

    impl PolicyDecisionPort for ShortPort {
        fn load_policy(&self, _policy_text: &str) -> Result<(), PolicyDecisionError> {
            Ok(())
        }

        fn evaluate(
            &self,
            _requests: &[PolicyDecisionRequest],
        ) -> Result<Vec<PolicyVerdict>, PolicyDecisionError> {
            Ok(Vec::new())
        }

        fn is_healthy(&self) -> bool {
            true
        }
    }

    #[test]
    fn stable_action_keys_are_human_readable() {
        assert_eq!(
            scope_deny_key(&Scope::FsRead {
                subtree: "/tmp".into()
            }),
            "fs.read"
        );
        assert_eq!(
            scope_deny_key(&Scope::ProcExec {
                binary: "sh".into()
            }),
            "proc.exec"
        );
        assert!(!scope_deny_key(&Scope::LoomScan).contains("Discriminant"));
    }

    #[test]
    fn cardinality_mismatch_is_rejected() {
        let err = reconcile_org_denies(&ShortPort).unwrap_err();
        assert!(
            matches!(&err, PolicyDecisionError::Transport(msg) if msg.contains("verdicts")),
            "short PDP verdict vector must hard-error, got {err:?}"
        );
    }
}
