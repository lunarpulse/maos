use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use maos_domain::ports::PolicyDecisionPort;
use maos_kernel_core::capability::cap_policy::PolicyTable;
use maos_pdp::{FailClosedOutcome, FailClosedPosture, FailClosedReconciler};
use tokio_util::sync::CancellationToken;

pub const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(30);
pub const DEFAULT_STALENESS_TTL: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicySource {
    Inline(String),
    File(PathBuf),
}

impl PolicySource {
    pub fn load(self) -> Result<String, String> {
        match self {
            PolicySource::Inline(policy) => Ok(policy),
            PolicySource::File(path) => read_policy_file(&path),
        }
    }
}

pub fn load_policy_text_from_env() -> Result<Option<String>, String> {
    let explicit_file = std::env::var_os("MAOS_PDP_POLICY_FILE").map(PathBuf::from);
    let explicit_inline = std::env::var("MAOS_PDP_POLICY_INLINE").ok();
    let legacy = std::env::var("MAOS_PDP_POLICY").ok();
    resolve_policy_source(explicit_file, explicit_inline, legacy)?
        .map_or(Ok(None), |source| source.load().map(Some))
}

pub fn resolve_policy_source(
    explicit_file: Option<PathBuf>,
    explicit_inline: Option<String>,
    legacy: Option<String>,
) -> Result<Option<PolicySource>, String> {
    match (explicit_file, explicit_inline) {
        (Some(_), Some(_)) => Err(
            "maos: set only one PDP policy source: MAOS_PDP_POLICY_FILE or MAOS_PDP_POLICY_INLINE"
                .into(),
        ),
        (Some(path), None) => Ok(Some(PolicySource::File(path))),
        (None, Some(policy)) => Ok(Some(PolicySource::Inline(policy))),
        (None, None) => Ok(legacy.map(resolve_legacy_policy_source)),
    }
}

fn resolve_legacy_policy_source(raw: String) -> PolicySource {
    if let Some(path) = raw.strip_prefix("file:") {
        PolicySource::File(PathBuf::from(path))
    } else if let Some(policy) = raw.strip_prefix("inline:") {
        PolicySource::Inline(policy.to_owned())
    } else {
        let path = Path::new(&raw);
        if path.is_file() {
            PolicySource::File(path.to_path_buf())
        } else {
            PolicySource::Inline(raw)
        }
    }
}

fn read_policy_file(path: &Path) -> Result<String, String> {
    if !path.is_file() {
        return Err(format!(
            "maos: PDP policy file '{}' does not exist or is not a file",
            path.display()
        ));
    }
    std::fs::read_to_string(path).map_err(|e| {
        format!(
            "maos: PDP policy file '{}' is unreadable: {e}",
            path.display()
        )
    })
}

pub fn refresh_interval_from_env() -> Result<Duration, String> {
    duration_ms_from_env("MAOS_PDP_REFRESH_INTERVAL_MS", DEFAULT_REFRESH_INTERVAL)
}

pub fn staleness_ttl_from_env() -> Result<Duration, String> {
    duration_ms_from_env("MAOS_PDP_STALENESS_TTL_MS", DEFAULT_STALENESS_TTL)
}

fn duration_ms_from_env(name: &str, default: Duration) -> Result<Duration, String> {
    let Some(raw) = std::env::var(name).ok() else {
        return Ok(default);
    };
    let millis = raw
        .parse::<u64>()
        .map_err(|_| format!("maos: {name} must be an integer millisecond value, got '{raw}'"))?;
    if millis == 0 {
        return Err(format!("maos: {name} must be > 0 ms"));
    }
    Ok(Duration::from_millis(millis))
}

pub fn validate_refresh_ttl(refresh_interval: Duration, ttl: Duration) -> Result<(), String> {
    if ttl < refresh_interval {
        return Err(format!(
            "maos: MAOS_PDP_STALENESS_TTL_MS ({}) must be >= MAOS_PDP_REFRESH_INTERVAL_MS ({})",
            ttl.as_millis(),
            refresh_interval.as_millis()
        ));
    }
    Ok(())
}

#[derive(Clone)]
pub struct EnterprisePdpRuntime {
    adapter: Arc<dyn PolicyDecisionPort>,
    policy: Arc<PolicyTable>,
    reconciler: FailClosedReconciler,
    refresh_interval: Duration,
    started_at: Instant,
    last_posture: Option<FailClosedPosture>,
}

impl EnterprisePdpRuntime {
    pub fn new(
        adapter: Arc<dyn PolicyDecisionPort>,
        policy: Arc<PolicyTable>,
        refresh_interval: Duration,
        staleness_ttl: Duration,
    ) -> Result<Self, String> {
        validate_refresh_ttl(refresh_interval, staleness_ttl)?;
        Ok(Self {
            adapter,
            policy,
            reconciler: FailClosedReconciler::new(duration_nanos_u64(staleness_ttl)),
            refresh_interval,
            started_at: Instant::now(),
            last_posture: None,
        })
    }

    pub fn refresh_once(&mut self) -> FailClosedPosture {
        self.refresh_once_at(duration_nanos_u64(self.started_at.elapsed()))
    }

    pub fn refresh_once_at(&mut self, now_nanos: u64) -> FailClosedPosture {
        let known_spirits = known_spirit_pids(&self.policy);
        let outcome =
            self.reconciler
                .reconcile_with_subjects_at(&*self.adapter, &known_spirits, now_nanos);
        apply_fail_closed_outcome(&self.policy, &outcome);
        self.log_posture_transition(outcome.posture, known_spirits.len(), &outcome);
        outcome.posture
    }

    pub fn spawn(mut self, cancel: CancellationToken) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = tokio::time::sleep(self.refresh_interval) => {
                        self.refresh_once();
                    }
                }
            }
        })
    }

    fn log_posture_transition(
        &mut self,
        posture: FailClosedPosture,
        subject_count: usize,
        outcome: &FailClosedOutcome,
    ) {
        if self.last_posture == Some(posture) {
            return;
        }
        self.last_posture = Some(posture);
        match posture {
            FailClosedPosture::Fresh => eprintln!(
                "maos: enterprise PDP fresh — {} org forbid(s), {} subject(s), {} subject-scoped deny set(s)",
                outcome.deny_keys.len(),
                subject_count,
                outcome.subject_denies.per_spirit.len()
            ),
            FailClosedPosture::StartupClosed
            | FailClosedPosture::RuntimeFreeze
            | FailClosedPosture::TtlExpiredRevert => eprintln!(
                "maos: warn: enterprise PDP fail-closed ({posture:?}) — {} global deny key(s), {} frozen subject deny set(s)",
                outcome.deny_keys.len(),
                outcome.subject_denies.per_spirit.len()
            ),
        }
    }
}

pub fn known_spirit_pids(policy: &PolicyTable) -> Vec<u32> {
    let mut pids: Vec<u32> = policy
        .inner()
        .load_full()
        .manifest_scopes
        .keys()
        .copied()
        .collect();
    pids.sort_unstable();
    pids
}

pub fn apply_fail_closed_outcome(policy: &PolicyTable, outcome: &FailClosedOutcome) {
    let mut global: HashSet<String> = outcome.deny_keys.iter().cloned().collect();
    global.extend(outcome.subject_denies.global.iter().cloned());

    let mut inner = (*policy.inner().load_full()).clone();
    inner.operator_policy.per_capability_deny = global;
    inner.operator_policy.per_spirit_capability_deny = outcome.subject_denies.per_spirit.clone();
    policy.update(inner);
}

fn duration_nanos_u64(duration: Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    //! Story 11.4a — focused tests for the composition-root PDP runtime seam.
    //! These exercise the BEHAVIOR the runtime guarantees to the kernel
    //! (policy-source resolution, manifest-driven subject denies, the
    //! freeze/expiry/recover timeline, and the subject-global fold), NOT the
    //! reconciler internals. The reconciler-level subject freeze has its own
    //! sibling in `maos-pdp/tests/fail_closed.rs`.
    use super::*;
    use maos_domain::invariants::i1::Scope;
    use maos_domain::ports::{PolicyDecisionError, PolicyDecisionRequest, PolicyVerdict};
    use maos_kernel_core::capability::cap_policy::{ManifestCapabilityScope, PolicyTableInner};
    use maos_pdp::{all_governed_deny_keys, scope_deny_key};
    use parking_lot::Mutex;
    use std::collections::HashMap;

    // ----- Policy source resolution: explicit config beats the legacy heuristic

    #[test]
    fn resolve_policy_source_picks_inline_without_slash_heuristic() {
        // The regression-prone boundary: an earlier shape sniffed the inline
        // text for `/` and silently re-routed it to a File. The explicit
        // inline / explicit file / `file:` / `inline:` / conflict contracts
        // are pinned here so a reintroduced heuristic reddens the right case.
        let cases: Vec<(
            &str,
            Option<PathBuf>,
            Option<String>,
            Option<String>,
            Option<PolicySource>,
        )> = vec![
            // Explicit MAOS_PDP_POLICY_INLINE containing `/` is Inline verbatim.
            (
                "explicit_inline_with_slash",
                None,
                Some("forbid(principal, action == Action::\"fs/read\");".into()),
                None,
                Some(PolicySource::Inline(
                    "forbid(principal, action == Action::\"fs/read\");".into(),
                )),
            ),
            (
                "explicit_inline_plain",
                None,
                Some("permit(...)".into()),
                None,
                Some(PolicySource::Inline("permit(...)".into())),
            ),
            // Explicit MAOS_PDP_POLICY_FILE is a File source — resolution does
            // NOT touch the filesystem (load() reads it later).
            (
                "explicit_file",
                Some(PathBuf::from("/opt/maos/policy.cedar")),
                None,
                None,
                Some(PolicySource::File(PathBuf::from("/opt/maos/policy.cedar"))),
            ),
            // Legacy `file:` prefix → File with the prefix stripped.
            (
                "legacy_file_prefix",
                None,
                None,
                Some("file:/etc/maos/policy".into()),
                Some(PolicySource::File(PathBuf::from("/etc/maos/policy"))),
            ),
            // Legacy `inline:` prefix → Inline with the prefix stripped.
            (
                "legacy_inline_prefix",
                None,
                None,
                Some("inline:permit(principal, action, resource);".into()),
                Some(PolicySource::Inline(
                    "permit(principal, action, resource);".into(),
                )),
            ),
            // Legacy bare text containing `/` that is NOT an existing file →
            // Inline (existence-check fallback, NOT a pure slash sniff).
            (
                "legacy_bare_nonfile_with_slash",
                None,
                None,
                Some("/this/path/does/not/exist.cedar".into()),
                Some(PolicySource::Inline(
                    "/this/path/does/not/exist.cedar".into(),
                )),
            ),
            ("nothing_configured", None, None, None, None),
        ];
        for (name, explicit_file, explicit_inline, legacy, expected) in cases {
            let got =
                resolve_policy_source(explicit_file, explicit_inline, legacy).unwrap_or_else(|e| {
                    panic!("resolve_policy_source({name}) unexpectedly errored: {e}")
                });
            assert_eq!(got, expected, "policy-source case `{name}`");
        }
    }

    #[test]
    fn resolve_policy_source_rejects_file_and_inline_conflict() {
        // Setting BOTH an explicit file and explicit inline is a hard
        // configuration error — the operator must disambiguate. This is the
        // one resolution leg that errors; it must stay a typed rejection
        // rather than silently preferring one source.
        let err = resolve_policy_source(
            Some(PathBuf::from("/opt/maos/policy.cedar")),
            Some("permit(principal, action, resource);".into()),
            None,
        )
        .expect_err("conflicting file+inline must error");
        assert!(
            err.contains("only one PDP policy source"),
            "conflict must be a disambiguating error, got: {err}"
        );
    }

    // ----- Fake PDP port: verdicts keyed by (spirit_pid, capability_key).
    //
    // Mirrors how the real Cedar adapter behaves under the subject-aware
    // reconcile calls: each (spirit, capability) request gets an independent
    // verdict. A single canned verdict vector would trip the reconciler's
    // cardinality-mismatch guard on the subject call (org call = N requests,
    // subject call = N * spirits requests), so the fake keys per request.
    // `spirit_pid == 0` models the org-wide (ceiling) request the reconciler
    // submits with `reconcile_org_denies`.

    #[derive(Clone)]
    struct ScriptedPort {
        state: Arc<Mutex<ScriptedState>>,
    }

    struct ScriptedState {
        denies: HashMap<u32, HashSet<String>>,
        failing: bool,
    }

    impl ScriptedPort {
        fn new(denies: HashMap<u32, HashSet<String>>) -> Self {
            Self {
                state: Arc::new(Mutex::new(ScriptedState {
                    denies,
                    failing: false,
                })),
            }
        }
        fn set_failing(&self, failing: bool) {
            self.state.lock().failing = failing;
        }
    }

    impl PolicyDecisionPort for ScriptedPort {
        fn load_policy(&self, _policy_text: &str) -> Result<(), PolicyDecisionError> {
            Ok(())
        }
        fn evaluate(
            &self,
            requests: &[PolicyDecisionRequest],
        ) -> Result<Vec<PolicyVerdict>, PolicyDecisionError> {
            let state = self.state.lock();
            if state.failing {
                return Err(PolicyDecisionError::Unreachable {
                    reason: "injected PDP failure".into(),
                });
            }
            Ok(requests
                .iter()
                .map(|r| {
                    if state
                        .denies
                        .get(&r.spirit_pid)
                        .is_some_and(|denied| denied.contains(&r.capability_key))
                    {
                        PolicyVerdict::Deny
                    } else {
                        PolicyVerdict::Allow
                    }
                })
                .collect())
        }
        fn is_healthy(&self) -> bool {
            !self.state.lock().failing
        }
    }

    /// TTL (nanoseconds) for the runtime timeline tests. `refresh_once_at`
    /// takes an explicit clock, so the tests are wall-clock-free: each
    /// timeline step passes a deterministic `now_nanos`.
    const TTL_NANOS: u64 = 1_000;

    fn build_runtime(port: Arc<ScriptedPort>, policy: Arc<PolicyTable>) -> EnterprisePdpRuntime {
        EnterprisePdpRuntime::new(
            Arc::clone(&port) as Arc<dyn PolicyDecisionPort>,
            policy,
            Duration::from_nanos(1),
            Duration::from_nanos(TTL_NANOS),
        )
        .expect("refresh_interval < staleness_ttl is a valid pair")
    }

    fn insert_spirit(policy: &PolicyTable, pid: u32) {
        let mut inner: PolicyTableInner = (*policy.inner().load_full()).clone();
        inner
            .manifest_scopes
            .insert(pid, ManifestCapabilityScope::default());
        policy.update(inner);
    }

    fn global_denies(policy: &PolicyTable) -> HashSet<String> {
        policy
            .inner()
            .load_full()
            .operator_policy
            .per_capability_deny
            .clone()
    }

    fn subject_denies(policy: &PolicyTable, pid: u32) -> HashSet<String> {
        policy
            .inner()
            .load_full()
            .operator_policy
            .per_spirit_capability_deny
            .get(&pid)
            .cloned()
            .unwrap_or_default()
    }

    #[test]
    fn refresh_picks_up_subject_deny_for_newly_inserted_spirit() {
        // Contract: refresh_once_at reads the CURRENT PolicyTable.manifest_scopes
        // each call, so a Spirit admitted AFTER construction gets its
        // subject-scoped deny on the next refresh. A runtime that cached the
        // spirit list at construction would never deny the new Spirit.
        let fs_read = scope_deny_key(&Scope::FsRead {
            subtree: String::new(),
        });
        let port = Arc::new(ScriptedPort::new(HashMap::from([(
            7,
            HashSet::from([fs_read.clone()]),
        )])));
        let policy = Arc::new(PolicyTable::new());
        let mut runtime = build_runtime(Arc::clone(&port), Arc::clone(&policy));

        // No spirits in the manifest yet → the subject-scoped deny cannot land.
        let posture = runtime.refresh_once_at(100);
        assert_eq!(posture, FailClosedPosture::Fresh);
        assert!(
            subject_denies(&policy, 7).is_empty(),
            "spirit 7 not yet in manifest → no subject deny"
        );

        // Admit spirit 7 into the live table.
        insert_spirit(&policy, 7);

        // Next refresh MUST re-read manifest_scopes and apply spirit 7's deny.
        let posture = runtime.refresh_once_at(200);
        assert_eq!(posture, FailClosedPosture::Fresh);
        assert!(
            subject_denies(&policy, 7).contains(&fs_read),
            "spirit 7 admitted → its fs.read subject deny materialized on the next refresh"
        );
    }

    #[test]
    fn runtime_drop_before_ttl_freezes_global_and_subject_denies() {
        // Contract: a PDP drop WITHIN the staleness TTL freezes the
        // last-known-good global AND subject deny sets — it never relaxes
        // toward permissive. Two spirits so the spirit-7 deny stays genuinely
        // subject-scoped (denied for 7 only, so it does not fold to global).
        let fs_read = scope_deny_key(&Scope::FsRead {
            subtree: String::new(),
        });
        let proc_exec = scope_deny_key(&Scope::ProcExec {
            binary: String::new(),
        });
        let port = Arc::new(ScriptedPort::new(HashMap::from([
            (0, HashSet::from([fs_read.clone()])), // spirit 0 ⇒ org-wide global deny
            (7, HashSet::from([proc_exec.clone()])), // spirit-7 subject deny
        ])));
        let policy = Arc::new(PolicyTable::new());
        insert_spirit(&policy, 7);
        insert_spirit(&policy, 8);
        let mut runtime = build_runtime(Arc::clone(&port), Arc::clone(&policy));

        // Fresh: establish the deny sets.
        runtime.refresh_once_at(100);
        assert!(global_denies(&policy).contains(&fs_read));
        assert!(subject_denies(&policy, 7).contains(&proc_exec));
        let frozen_global = global_denies(&policy);
        let frozen_subject = subject_denies(&policy, 7);

        // PDP drops.
        port.set_failing(true);

        // Within TTL → RuntimeFreeze; both deny sets must be byte-for-byte
        // unchanged (no relaxation toward the empty/permit side).
        let posture = runtime.refresh_once_at(200);
        assert_eq!(posture, FailClosedPosture::RuntimeFreeze);
        assert_eq!(
            global_denies(&policy),
            frozen_global,
            "global denies frozen verbatim on a within-TTL drop"
        );
        assert_eq!(
            subject_denies(&policy, 7),
            frozen_subject,
            "subject denies frozen verbatim on a within-TTL drop"
        );
    }

    #[test]
    fn ttl_expiry_then_fresh_evaluation_relaxes_to_current_result() {
        // Contract: past TTL the runtime reverts to ALL governed global denies
        // (closing the stale-permit / revocation window), and a later fresh
        // evaluation relaxes back down to the actual current result. The
        // relaxation is driven ONLY by a real evaluation — neither the freeze
        // nor the expiry path ever relaxes toward permissive on its own.
        let port = Arc::new(ScriptedPort::new(HashMap::new())); // allow everything
        let policy = Arc::new(PolicyTable::new());
        insert_spirit(&policy, 7);
        let mut runtime = build_runtime(Arc::clone(&port), Arc::clone(&policy));

        // Fresh: nothing denied.
        runtime.refresh_once_at(100);
        assert!(global_denies(&policy).is_empty());

        // PDP drops — within TTL, RuntimeFreeze keeps the (empty) last-good.
        port.set_failing(true);
        let posture = runtime.refresh_once_at(200);
        assert_eq!(posture, FailClosedPosture::RuntimeFreeze);
        assert!(global_denies(&policy).is_empty());

        // Past TTL — revert to ALL governed denies (maximally restrictive).
        let posture = runtime.refresh_once_at(2_000);
        assert_eq!(posture, FailClosedPosture::TtlExpiredRevert);
        let governed = all_governed_deny_keys();
        assert_eq!(
            global_denies(&policy).len(),
            governed.len(),
            "past TTL the global deny set is every governed key"
        );
        for key in &governed {
            assert!(
                global_denies(&policy).contains(key),
                "governed key `{key}` materialized after TTL expiry"
            );
        }
        assert!(
            subject_denies(&policy, 7).is_empty(),
            "past TTL subject denies are cleared (global-only revert)"
        );

        // PDP recovers — a fresh evaluation relaxes from all-governed back to
        // the actual (empty) result. If the reconciler stuck at TtlExpiredRevert
        // or a fresh eval failed to overwrite last_good, this stays maximal.
        port.set_failing(false);
        let posture = runtime.refresh_once_at(2_100);
        assert_eq!(posture, FailClosedPosture::Fresh);
        assert!(
            global_denies(&policy).is_empty(),
            "fresh evaluation relaxes the global deny set to the current result"
        );
    }

    #[test]
    fn subject_global_denies_fold_into_global_deny_set() {
        // Contract: apply_fail_closed_outcome folds MaterializedDenies.global
        // (capabilities denied for EVERY known Spirit) into the kernel's single
        // global deny set. fs.read is denied for spirits 7 AND 8 but NOT
        // org-wide (spirit 0), so it reaches the global set ONLY via the
        // subject-global fold — a bug dropping that fold reddens here.
        let fs_read = scope_deny_key(&Scope::FsRead {
            subtree: String::new(),
        });
        let port = Arc::new(ScriptedPort::new(HashMap::from([
            (7, HashSet::from([fs_read.clone()])),
            (8, HashSet::from([fs_read.clone()])),
        ])));
        let policy = Arc::new(PolicyTable::new());
        insert_spirit(&policy, 7);
        insert_spirit(&policy, 8);
        let mut runtime = build_runtime(Arc::clone(&port), Arc::clone(&policy));

        runtime.refresh_once_at(100);

        assert!(
            global_denies(&policy).contains(&fs_read),
            "fs.read denied for ALL known Spirits folded into the global deny set"
        );
        // The fold is additive, not a move: the per-spirit entries remain.
        assert!(subject_denies(&policy, 7).contains(&fs_read));
        assert!(subject_denies(&policy, 8).contains(&fs_read));
    }
}
