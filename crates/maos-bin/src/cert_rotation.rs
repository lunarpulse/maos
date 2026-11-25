//! Story 14-2a — the production wiring of the peer-certificate rotation
//! trigger: the real deadline, and the operator's read seam.
//!
//! Both live in the LIBRARY rather than in `main.rs`, on this crate's own
//! doctrine (see `lib.rs`): a private item of the binary crate cannot be named
//! from `crates/maos-bin/tests/`, and an in-`src` test module is budget-charged
//! and CI-invisible. The runtime gate leg for this story has to drive these two
//! types, not a re-implementation of them.
//!
//! ⚠ **`MAOS_ONE_SHOT` CANNOT CARRY A ROTATION VERB, AND A DEV WHO DOES NOT
//! READ THIS WILL WRITE ONE AND ITS TEST WILL PASS.** Every `maosctl` verb that
//! performs a runtime action — `posture`, `halt resolve`, `pause`, `resume`,
//! `revoke-token`, `forget`, `legal-hold`, `spirit upgrade`, `governance admit`
//! — spawns a FRESH `maos` child process configured by `MAOS_ONE_SHOT`
//! (`crates/maos-cli/src/subcommands.rs:1434-1460`, `:1522-1536`, `:1592`,
//! `:1615-1641`, `:1698-1715`, `:1842`; arm table `main.rs:4896`). Exactly one
//! subcommand reaches a RUNNING daemon — `maosctl spirit inspect --sandbox`
//! (`subcommands.rs:1886` → `fetch_live_sandbox_report` `:1930`) — and it is a
//! GET. A fresh process cannot mutate the running daemon's in-memory
//! `InMemoryTofuPinStore`: it would build its own, rotate that, exit, and leave
//! the live mesh untouched while its own test asserted on the child's state.
//! That is why the WRITE path here is a signed cohort-manifest reissue over the
//! live A2A wire, and why the only thing this operator surface adds is a GET.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use maos_cohort::rotation::RotationGraceTimer;
use maos_cohort::CohortManifestState;
use maos_control::{
    CohortConvergenceSource, CohortSelfIdentitySource, PeerVersionRow, PeerVersionStatus,
    RotationWindowRow, RotationWindowSource, RotationWindowStatus, SelfIdentityRow,
    SelfIdentityStatus,
};

/// The real `T_grace` deadline: a spawned task that sleeps and then performs
/// the terminal transition.
///
/// Modelled on `PostSwapMonitor::spawn`
/// (`crates/maos-kernel-core/src/hot_swap/post_swap_monitor.rs:38-105`) — the
/// shipped deadline-then-terminal-action shape — and on
/// `FailClosedReconciler`'s semantics (`crates/maos-pdp/src/reconcile.rs:230-300`),
/// the only other operator-visible TTL in the tree that REVERTS a widened trust
/// set rather than merely reporting on it. The window closes on ELAPSED TIME,
/// never on the next frame: a widened trust set that waits for traffic to narrow
/// it is a widened trust set with no expiry.
///
/// The task is deliberately detached. The transition it performs is a
/// promote-and-retire on a process-local in-memory store, so a process that
/// exits before the deadline has already narrowed its trust set to whatever the
/// operator TOML says — a restart is the involuntary revert documented in
/// `RELEASE-HOLDS.md`, not a leak of the window.
pub struct TokioGraceTimer;

impl RotationGraceTimer for TokioGraceTimer {
    fn schedule(&self, grace: Duration, on_expiry: Box<dyn FnOnce() + Send>) {
        tokio::spawn(async move {
            tokio::time::sleep(grace).await;
            on_expiry();
        });
    }
}

/// The operator READ seam over the live open-window set (AC1.5).
///
/// Holds the SAME `Arc<CohortManifestState>` the signed reissue path writes
/// through, so the read and the write are one seam with two consumers rather
/// than two mechanisms: `install_cert_rotation` fills the state's set-once
/// control, and this type reads it. It is bound into
/// `maos_control::OperatorHttpServer` ~7,900 lines BEFORE the transport exists,
/// which is exactly why the control is a `OnceLock` and not a constructor
/// argument.
pub struct CohortRotationWindows {
    state: Arc<CohortManifestState>,
}

impl CohortRotationWindows {
    pub fn new(state: Arc<CohortManifestState>) -> Self {
        Self { state }
    }
}

impl RotationWindowSource for CohortRotationWindows {
    fn open_rotation_windows(&self) -> Option<RotationWindowStatus> {
        match self.state.rotation_status() {
            Ok(None) => None,
            Ok(Some(windows)) => Some(RotationWindowStatus::Healthy(
                windows
                    .into_iter()
                    .map(|window| RotationWindowRow {
                        peer: window.peer,
                        retiring: window.retiring,
                        next: window.next,
                        declared: window.declared,
                        state: window.state.to_string(),
                        manifest_version: window.manifest_version,
                        opened_at_secs: window.opened_at_secs,
                    })
                    .collect(),
            )),
            Err(error) => Some(RotationWindowStatus::Unhealthy {
                detail: error.to_string(),
            }),
        }
    }
}

/// Story 14-2b / AC3 — the operator READ seam over retained peer manifest
/// versions.
///
/// Holds the SAME `Arc<CohortManifestState>` the `Pull` receive arm writes
/// through, for the same reason [`CohortRotationWindows`] does: one seam with
/// two consumers, never two mechanisms. It lives in the LIBRARY and not in
/// `main.rs` so the gate leg can drive the real type.
///
/// It carries NO cohort type across the `maos-control` boundary: the mapping
/// from `PeerConvergence` to [`PeerVersionRow`] happens HERE, in the crate that
/// already depends on both.
pub struct CohortPeerVersions {
    state: Arc<CohortManifestState>,
}

impl CohortPeerVersions {
    pub fn new(state: Arc<CohortManifestState>) -> Self {
        Self { state }
    }
}

impl CohortConvergenceSource for CohortPeerVersions {
    fn peer_manifest_versions(&self) -> Option<PeerVersionStatus> {
        if !self.state.convergence_observer_ready() {
            return None;
        }
        // NEVER `unwrap_or_default()` here. A poisoned or unreadable state
        // would then render as an empty list, and an empty list is exactly the
        // shape an operator must be able to trust means "nothing declared".
        match self.state.peer_convergence() {
            Ok(records) => Some(PeerVersionStatus::Healthy(
                records
                    .into_iter()
                    .map(|record| PeerVersionRow {
                        valid: record.is_valid(),
                        peer: record.peer,
                        declared_version: record.declared_version,
                        declared_hash: record.declared_hash,
                        observed_at_secs: record.observed_at_secs,
                        state: record.state.to_string(),
                    })
                    .collect(),
            )),
            Err(error) => Some(PeerVersionStatus::Unhealthy {
                detail: error.to_string(),
            }),
        }
    }
}

/// Story 14-2c — live local certificate identity plus mesh reachability.
pub struct CohortSelfIdentity {
    state: Arc<CohortManifestState>,
    pull_health: Arc<Mutex<BTreeMap<String, String>>>,
}

impl CohortSelfIdentity {
    pub fn new(
        state: Arc<CohortManifestState>,
        pull_health: Arc<Mutex<BTreeMap<String, String>>>,
    ) -> Self {
        Self { state, pull_health }
    }
}

impl CohortSelfIdentitySource for CohortSelfIdentity {
    fn self_identity(&self) -> Option<SelfIdentityStatus> {
        let manifest = match self.state.manifest() {
            Ok(manifest) => manifest,
            Err(error) => {
                return Some(SelfIdentityStatus::Unhealthy {
                    detail: error.to_string(),
                });
            }
        };
        let convergence = match self.state.peer_convergence() {
            Ok(convergence) => convergence,
            Err(error) => {
                return Some(SelfIdentityStatus::Unhealthy {
                    detail: error.to_string(),
                });
            }
        };
        let last_pull_errors = match self.pull_health.lock() {
            Ok(errors) => errors.clone(),
            Err(_) => {
                return Some(SelfIdentityStatus::Unhealthy {
                    detail: "cohort pull-health lock poisoned".into(),
                });
            }
        };
        let local_host = self.state.local_host().as_str();
        let declared = manifest
            .members
            .iter()
            .find(|member| member.host_id == local_host)
            .and_then(|member| maos_a2a_core::PeerCertFingerprint::parse(&member.fingerprint));
        let serving = self.state.local_leaf_fingerprint();
        let peers_observed = convergence
            .into_iter()
            .filter(|record| record.is_valid())
            .count();
        let peers_total = manifest
            .members
            .iter()
            .filter(|member| member.host_id != local_host)
            .count();
        let verdict = match (&declared, &serving) {
            (Some(declared), Some(serving)) if declared != serving => "diverged",
            // Agreement additionally requires no CURRENT pull failure: a
            // convergence record stays valid until its signed lease expires,
            // so during a partition the table can still read "fully observed"
            // while every renewal pull fails. "agree" must not be claimable
            // from stale observations (14-2c review finding).
            (Some(_), Some(_)) if peers_observed == peers_total && last_pull_errors.is_empty() => {
                "agree"
            }
            _ => "unconfirmable",
        };
        Some(SelfIdentityStatus::Healthy(SelfIdentityRow {
            declared: declared.map(|fingerprint| fingerprint.short()),
            serving: serving.map(|fingerprint| fingerprint.short()),
            verdict: verdict.into(),
            peers_observed,
            peers_total,
            last_pull_errors,
        }))
    }
}

/// Story 23-7 / D-23-7-02 C — the production caller of the own-leaf swap
/// ([`TcpA2ATransport::prepare_own_identity`] / [`TcpA2ATransport::commit_own_identity`]).
///
/// The signed cohort manifest is the ONLY authority for replacing this host's serving leaf
/// (ADR-062 keeps serving-leaf replacement off the operator POST surface). So the trigger is not
/// a verb: this reconciler wakes when a reissue moved this host's declaration off the served leaf
/// ([`CohortManifestState::install_local_leaf_observer`], delivered after the `cached` lock is
/// released), when a peer's `Pull` arrives while the swap is pending, and on every confirmation
/// tick, and ONLY while the declaration and the served leaf differ it re-reads the configured PEM
/// paths, validates the identity (exact host SAN, chain, both TLS usages) and commits it inside
/// [`CohortManifestState::commit_own_leaf`] if the leaf there IS the declared one, the declaration
/// is still current at the commit, and every peer confirmed the manifest version (or the stale
/// lease elapsed). A leaf the manifest does not declare is never served; a declared leaf that is
/// not on disk yet leaves the host partitioned from every peer that already promoted
/// (fail-closed), and a later tick heals it with no restart.
///
/// It runs on its own task: never inside a `RotationGraceTimer` callback, never under `cached`.
pub struct OwnLeafReconciler {
    transport: Arc<maos_a2a_tcp::TcpA2ATransport>,
    state: Arc<CohortManifestState>,
    /// The last row journaled, so a standing condition (`swap_pending`) writes ONE row, not one
    /// per tick.
    last_row: Mutex<Option<(maos_cohort::SelfLeafOutcome, String, String)>>,
}

impl OwnLeafReconciler {
    pub fn new(
        transport: Arc<maos_a2a_tcp::TcpA2ATransport>,
        state: Arc<CohortManifestState>,
    ) -> Self {
        Self {
            transport,
            state,
            last_row: Mutex::new(None),
        }
    }

    /// One pass. Returns the outcome it journaled, or `None` when the served leaf already is the
    /// declared one (no file is read), the row was already written for this exact condition, or a
    /// newer reissue replaced the declaration before the commit (that reissue wakes the next pass).
    ///
    /// The files are read and validated OFF every lock ([`maos_a2a_tcp::TcpA2ATransport::prepare_own_identity`]);
    /// the commit then runs inside [`CohortManifestState::commit_own_leaf`], which re-checks the
    /// declaration and the peers' confirmation under the manifest lock, so a reissue cannot slip
    /// between the check and the swap (review B2) and the swap waits for the peers (review B5).
    pub fn reconcile(&self) -> Option<maos_cohort::SelfLeafOutcome> {
        use maos_a2a_tcp::OwnLeafPrepare;
        use maos_cohort::{OwnLeafCommit, SelfLeafOutcome};

        let (declared, version) = self.state.local_declaration().ok().flatten()?;
        if self.state.local_leaf_fingerprint().as_ref() == Some(&declared) {
            // Consistent again: a later divergence is a new condition and gets its own row.
            *self
                .last_row
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
            return None;
        }
        let unusable = |error: &dyn std::fmt::Display| {
            eprintln!("cohort own-leaf reconcile: the configured identity is unusable: {error}");
            (
                SelfLeafOutcome::RefusedUnusable,
                "unusable".to_string(),
                String::new(),
            )
        };
        let (outcome, on_disk, detail) = match self.transport.prepare_own_identity(&declared) {
            Ok(OwnLeafPrepare::Current) => return None,
            Ok(OwnLeafPrepare::Pending { on_disk }) => {
                (SelfLeafOutcome::SwapPending, on_disk.wire(), String::new())
            }
            Ok(OwnLeafPrepare::Undeclared { on_disk }) => (
                SelfLeafOutcome::RefusedUndeclared,
                on_disk.wire(),
                String::new(),
            ),
            Ok(OwnLeafPrepare::Ready(prepared)) => {
                let on_disk = prepared.fingerprint().wire();
                let transport = &self.transport;
                match self
                    .state
                    .commit_own_leaf(&declared, || transport.commit_own_identity(prepared))
                {
                    Ok(OwnLeafCommit::Superseded) => return None,
                    Ok(OwnLeafCommit::Held { unconfirmed }) => (
                        SelfLeafOutcome::SwapHeldForPeers,
                        on_disk,
                        unconfirmed.join(","),
                    ),
                    Ok(OwnLeafCommit::Committed {
                        result: Ok((_retired, serving)),
                        unconfirmed,
                    }) => (
                        if unconfirmed.is_empty() {
                            SelfLeafOutcome::Swapped
                        } else {
                            SelfLeafOutcome::SwappedUnconfirmed
                        },
                        serving.wire(),
                        unconfirmed.join(","),
                    ),
                    Ok(OwnLeafCommit::Committed {
                        result: Err(error), ..
                    }) => unusable(&error),
                    Err(error) => {
                        eprintln!(
                            "cohort own-leaf reconcile: the manifest state is unavailable: {error}"
                        );
                        return None;
                    }
                }
            }
            Err(error) => unusable(&error),
        };
        let key = (outcome, on_disk.clone(), declared.wire());
        let mut last = self
            .last_row
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if last.as_ref() == Some(&key) {
            return None;
        }
        match self
            .state
            .journal_self_leaf(outcome, &on_disk, &declared.wire(), version, &detail)
        {
            Ok(()) => {
                *last = Some(key);
                Some(outcome)
            }
            Err(error) => {
                eprintln!("cohort own-leaf reconcile: the journal row was not written: {error}");
                None
            }
        }
    }

    /// Run [`Self::reconcile`] now, on every `wake`, and on every confirmation interval of the
    /// manifest in force, until `cancel`.
    pub fn spawn(
        self: Arc<Self>,
        wake: Arc<tokio::sync::Notify>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                // A pass reads files and fsyncs nothing, but it is blocking I/O: keep it off the
                // reactor.
                let this = Arc::clone(&self);
                if let Err(error) = tokio::task::spawn_blocking(move || this.reconcile()).await {
                    eprintln!("cohort own-leaf reconcile task failed: {error}");
                }
                let interval = self
                    .state
                    .confirmation_interval()
                    .unwrap_or(Duration::from_secs(15));
                tokio::select! {
                    () = cancel.cancelled() => return,
                    () = wake.notified() => {}
                    () = tokio::time::sleep(interval) => {}
                }
            }
        })
    }
}
