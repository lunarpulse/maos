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

use std::sync::Arc;
use std::time::Duration;

use maos_cohort::rotation::RotationGraceTimer;
use maos_cohort::CohortManifestState;
use maos_control::{
    CohortConvergenceSource, PeerVersionRow, PeerVersionStatus, RotationWindowRow,
    RotationWindowSource, RotationWindowStatus,
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
