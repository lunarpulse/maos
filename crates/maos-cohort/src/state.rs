#![forbid(unsafe_code)]

use maos_a2a_core::identity::PeerCertFingerprint;
use maos_a2a_core::{
    CohortConsentDenial, CohortConsentSeam, CohortConsentVerdict, CohortManifestGate,
    CohortReissueDisposition, CohortReissueRejection, DigestFrameClass, DigestReadPort,
    DigestReplyObservation, HaltReceiptObserver, COHORT_INTENT_DIGEST_READ,
};
use maos_domain::frame::IacFrame;
use maos_spirit_abi::identity::HostId;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::audit::{CohortAuditEvent, CohortAuditSink};
use crate::consent::{accept_admits, send_context, OutboundConsentContext};
use crate::control::CohortManifestControl;
use crate::digest::{DigestReadControl, DigestSummary, DIGEST_DAILY_SCOPE};
use crate::error::{CohortError, CohortManifestForkReason};
use crate::halt_receipt::{AbsenceKind, HaltReceiptControl};
use crate::manifest::CohortManifest;
use crate::pin::PinnedAuthorityKeys;
use crate::rotation::{PeerCertRotation, RotationGraceTimer, RotationWindow};

pub trait CohortClock: Send + Sync {
    fn now_secs(&self) -> u64;
}

/// The pull interval a manifest signs: half its stale lease, rounded up. Also the lag between
/// two members applying one reissue, which is why Story 23-7 / D-23-7-03 A adds it to the
/// rotation grace.
fn confirmation_of(manifest: &CohortManifest) -> Duration {
    Duration::from_secs((manifest.t_stale_secs + 1) / 2)
}

/// The lag the rotation grace must cover (review B5/B6). Two members apply one reissue up to one
/// pull interval apart, and this host's own-leaf swap then trails the last peer's apply by up to
/// one more (it waits for that peer's next `Pull` to confirm) — hence twice the interval. The
/// interval is the LARGER of the manifest being applied and the one in force: the pull cadence
/// that actually delivers the reissue is the one already in force, while the grace is armed from
/// the new one (a reissue that lowers `t_stale_secs` must not shrink the cover).
fn window_lag(candidate: &CohortManifest, in_force: &CohortManifest) -> Duration {
    confirmation_of(candidate).max(confirmation_of(in_force)) * 2
}

struct SystemCohortClock {
    base: Instant,
}

impl SystemCohortClock {
    fn new() -> Self {
        Self {
            base: Instant::now(),
        }
    }
}

impl CohortClock for SystemCohortClock {
    fn now_secs(&self) -> u64 {
        self.base.elapsed().as_secs()
    }
}

struct CachedManifest {
    manifest: CohortManifest,
    canonical_hash: [u8; 32],
    /// Exact signed bytes that passed pinned-key verification. Distribution
    /// reuses this artifact rather than reconstructing a body/signature pair.
    signed_toml: String,
    confirmed_at_secs: u64,
    /// Cohort-clock seconds at which this manifest VERSION was applied (or loaded). Unlike
    /// `confirmed_at_secs`, a same-version confirmation never refreshes it.
    applied_at_secs: u64,
}

#[derive(Debug, Clone)]
struct DigestGrant {
    scope: String,
    manifest_version: u64,
}

const MAX_PENDING_DIGEST_READS: usize = 256;

fn schema_is_downgrade(current_schema: u64, candidate_schema: u64) -> bool {
    candidate_schema < current_schema
}

#[cfg(test)]
mod schema_floor_tests {
    use super::schema_is_downgrade;

    #[test]
    fn schema_downgrade_is_an_ordered_floor() {
        assert!(schema_is_downgrade(2, 1));
        assert!(schema_is_downgrade(3, 2));
        assert!(schema_is_downgrade(3, 1));
        // Story 13.6a — the floor is ordinal, so V4 inherits it with no new
        // branch: once a node has accepted the authenticated-team schema it can
        // never be walked back to a schema that cannot express the edge.
        assert!(schema_is_downgrade(4, 3));
        assert!(schema_is_downgrade(4, 1));
        assert!(!schema_is_downgrade(4, 4));
        assert!(!schema_is_downgrade(2, 2));
        assert!(!schema_is_downgrade(2, 3));
        assert!(!schema_is_downgrade(3, 4));
    }
}

/// Story 14-2b / AC1 — a convergence observation that is still trustworthy.
pub const CONVERGENCE_OBSERVED: &str = "observed";
/// Story 14-2b / AC2a — the peer's pin generation changed or was invalidated
/// after the observation, so the recorded version predates a restart and is no
/// longer a claim about anything.
pub const CONVERGENCE_RESTARTED: &str = "restarted";
/// Story 14-2b / AC2b — the observation is older than the derived bound.
pub const CONVERGENCE_STALE: &str = "stale";

/// One cohort peer's last SELF-DECLARED manifest version, as an operator reads
/// it.
///
/// ⚠ **Authenticated as coming from that peer, NEVER verified as true.** Every
/// field but [`Self::state`] is a value the peer put on the wire in its own
/// `Pull`; the TLS peer binding (`router.rs:1755-1777`) makes the attribution
/// trustworthy and says nothing about the content. A peer lying HIGH is bounded
/// by the fact that this host only ever displays it; a peer lying LOW is the
/// exposure `14-2d` inherits and must rule on, because that is the story whose
/// swap deadline is evaluated against this table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerConvergence {
    /// The TLS-verified peer the declaration came from.
    pub peer: String,
    /// The version that peer declared it holds.
    pub declared_version: u64,
    /// The canonical hash it declared at that version, hex-encoded exactly as
    /// `CohortDistributor::pull_from` sends it (`distribution.rs:67`).
    pub declared_hash: String,
    /// [`CohortClock`] seconds at which the declaration was received — never
    /// `SystemTime::now()`, so a test clock moves it and the staleness bound is
    /// deterministic.
    pub observed_at_secs: u64,
    /// Exact signed-manifest lease deadline captured with this observation.
    /// Later reissues cannot extend or revive an already-expired declaration.
    pub expires_at_secs: u64,
    /// The boot nonce authenticated for this exact request. A zero nonce is
    /// never retained because it cannot support restart invalidation.
    pub pin_generation: u64,
    /// [`CONVERGENCE_OBSERVED`], [`CONVERGENCE_RESTARTED`] or
    /// [`CONVERGENCE_STALE`]. Derived on every read, never stored, so it cannot
    /// go stale the way a cached disposition would.
    pub state: &'static str,
}

impl PeerConvergence {
    /// Whether this record is currently a convergence claim at all (AC2).
    ///
    /// The predicate lives HERE and not in the read surface because `14-2d`
    /// evaluates its swap deadline against this table: two copies of "valid"
    /// would let the operator's view and the rotation gate's view disagree.
    pub fn is_valid(&self) -> bool {
        self.state == CONVERGENCE_OBSERVED
    }
}

/// What [`CohortManifestState::commit_own_leaf`] did (Story 23-7 review B2 + B5).
#[derive(Debug, PartialEq, Eq)]
pub enum OwnLeafCommit<R> {
    /// A newer reissue replaced or removed this host's signed declaration after the leaf was
    /// prepared: nothing was committed.
    Superseded,
    /// Peers have not yet confirmed the manifest version in force and the stale-lease bound has not
    /// elapsed: nothing was committed, the retiring leaf stays served.
    Held { unconfirmed: Vec<String> },
    /// The commit ran. `unconfirmed` is non-empty only when the stale-lease bound elapsed before
    /// every peer confirmed.
    Committed { result: R, unconfirmed: Vec<String> },
}

/// The verified, local authority for a cohort manifest cache.
///
/// All state transitions verify a candidate under the operator-pinned genesis
/// key set before inspecting its version. This prevents unauthenticated input
/// from advancing or regressing the cached version.
pub struct CohortManifestState {
    pinned: PinnedAuthorityKeys,
    audit: Arc<dyn CohortAuditSink>,
    cached: Mutex<CachedManifest>,
    pull_requests: Mutex<Vec<HostId>>,
    clock: Arc<dyn CohortClock>,
    local_host: HostId,
    /// Story 12.3 — per-member received-receipt presence table: member
    /// `host_id` → the set of distinct `HaltReceipt.halt_id`s observed from
    /// that authenticated member. Dedup is by `halt_id` (P4), so a re-shipped
    /// receipt never inflates the count. Interior-mutable like `pull_requests`.
    halt_presence: Mutex<HashMap<String, HashSet<String>>>,
    /// Story 12.3 — per-member explicit transport-level absence marker recorded
    /// after a classified probe (P2a/P3). Observability only.
    halt_absence: Mutex<HashMap<String, AbsenceKind>>,
    /// Story 12.4a — reader-side live capabilities keyed by authenticated peer
    /// and globally unique request id. Each capability is bound to the admitted
    /// scope and manifest version, then consumed by the first accepted reply.
    outstanding_digest_reads: Mutex<HashMap<(String, String), DigestGrant>>,
    /// Target-side reply capabilities with the same immutable binding.
    admitted_digest_reads: Mutex<HashMap<(String, String), DigestGrant>>,
    /// Target-side bounded reply obligations.
    pending_digest_replies: Mutex<Vec<(String, String, String)>>,
    /// Reader-side immutable summaries keyed by `(member, request_id)`.
    received_digest_summaries: Mutex<HashMap<(String, String), DigestSummary>>,
    /// Story 14-2b / AC1 — per-peer last-declared manifest version, keyed by
    /// the TLS-verified peer. Interior-mutable like `pull_requests`, and
    /// written from the SAME receive arm: the `Pull` this host already queues a
    /// push for is the frame that carries the value, so retention adds no wire
    /// field, no intent and no second code path.
    ///
    /// A `BTreeMap` and not a `HashMap` on purpose: the read surface is an
    /// operator-facing list, so peer order must be stable across reads without
    /// a sort at every call.
    peer_convergence: Mutex<BTreeMap<String, PeerConvergence>>,
    /// Story 14-2a / AC1.2 — the SET-ONCE production rotation seam.
    ///
    /// This closes an ORDERING INVERSION, and it is the same shape and the same
    /// reason as `Mailbox::install_a2a_router`
    /// (`crates/maos-iac/src/adapter/mailbox.rs:131`), whose doc says it exists
    /// *"because the production composition root wraps the mailbox in an `Arc`
    /// before the router's peer configs and TOFU store exist"* and whose
    /// set-once refusal is *"a security property, not an ergonomic."* Here the
    /// inversion is: this state is loaded at `main.rs:2038`, the only concrete
    /// `Arc<TcpA2ATransport>` in the workspace is created ~7,900 lines later at
    /// `main.rs:10000`, and the pin store and router core that the reload must
    /// move live inside it.
    ///
    /// Set-once is load-bearing for the same reason it is on the mailbox: a
    /// SECOND install would point the signed-manifest trigger at a different
    /// mesh's trust planes, which is a trust-set redirection dressed as a
    /// configuration call. `None` until installed means "no rotation trigger in
    /// this process" — every non-daemon `MAOS_ONE_SHOT` arm, byte-for-byte
    /// unchanged.
    cert_rotation: std::sync::OnceLock<Arc<PeerCertRotation>>,
    /// Story 23-7 / D-23-7-02 C — called, OFF the `cached` lock, after a reissue left this host's
    /// signed declaration different from the leaf the transport serves. The composition root's
    /// own-leaf reconciler wakes on it; implementations MUST NOT block (they run on the caller of
    /// `apply_reissue`) and may call back into this state (the lock is released).
    local_leaf_observer: std::sync::OnceLock<Arc<dyn Fn() + Send + Sync>>,
    /// Story 23-7 / D-23-7-09 A (review B1) — the highest signed-manifest version this host
    /// applied or loaded as a durable floor. Held HERE, not only in the pin store, so a floor
    /// loaded at boot is enforced before `install_cert_rotation` runs (the listener and the pull
    /// service are live before it), and raised before a reissue becomes visible.
    manifest_floor: std::sync::atomic::AtomicU64,
}

impl CohortManifestState {
    /// Loads a manifest only after strict schema validation and cryptographic
    /// verification under the operator-provisioned authority pin set.
    pub fn load(
        local_host: HostId,
        manifest_toml: &str,
        pinned: PinnedAuthorityKeys,
        audit: Arc<dyn CohortAuditSink>,
    ) -> Result<Self, CohortError> {
        Self::load_with_clock(
            local_host,
            manifest_toml,
            pinned,
            audit,
            Arc::new(SystemCohortClock::new()),
        )
    }

    #[doc(hidden)]
    pub fn load_with_clock(
        local_host: HostId,
        manifest_toml: &str,
        pinned: PinnedAuthorityKeys,
        audit: Arc<dyn CohortAuditSink>,
        clock: Arc<dyn CohortClock>,
    ) -> Result<Self, CohortError> {
        let manifest = CohortManifest::parse_and_validate(manifest_toml, &pinned)?;
        manifest.verify_signature(&pinned)?;
        Ok(Self {
            local_host,
            pinned,
            audit,
            cached: Mutex::new(CachedManifest {
                canonical_hash: manifest.canonical_hash(),
                manifest,
                signed_toml: manifest_toml.to_string(),
                confirmed_at_secs: clock.now_secs(),
                applied_at_secs: clock.now_secs(),
            }),
            pull_requests: Mutex::new(Vec::new()),
            halt_presence: Mutex::new(HashMap::new()),
            halt_absence: Mutex::new(HashMap::new()),
            outstanding_digest_reads: Mutex::new(HashMap::new()),
            admitted_digest_reads: Mutex::new(HashMap::new()),
            pending_digest_replies: Mutex::new(Vec::new()),
            received_digest_summaries: Mutex::new(HashMap::new()),
            peer_convergence: Mutex::new(BTreeMap::new()),
            cert_rotation: std::sync::OnceLock::new(),
            local_leaf_observer: std::sync::OnceLock::new(),
            manifest_floor: std::sync::atomic::AtomicU64::new(0),
            clock,
        })
    }

    /// Story 14-2a / AC1.1+AC1.2 — install the live trust planes this state's
    /// signed reissues must move. SET-ONCE: a second call is REFUSED.
    ///
    /// `pins` and `core` MUST be the running mesh's own handles
    /// (`TcpA2ATransport::pins()` / `::core()`, which return clones of the SAME
    /// `Arc`s the accept loop, both verifiers and the router core hold), or the
    /// reload silently reloads a copy — which is precisely the failure mode of
    /// the `MAOS_ONE_SHOT` design this story measured out: a fresh process
    /// mutating its own in-memory store, passing its own test, and changing
    /// nothing in the live daemon.
    ///
    /// The audit sink is NOT a parameter: the rotation rows are written through
    /// the SAME `Arc<dyn CohortAuditSink>` this state already journals reissue
    /// acceptance and rejection through, so the rotation timeline and the
    /// manifest timeline can never land in different logs.
    pub fn install_cert_rotation(
        &self,
        pins: Arc<maos_a2a_core::InMemoryTofuPinStore>,
        core: Arc<maos_a2a_core::A2ARouterCore>,
        timer: Arc<dyn RotationGraceTimer>,
        grace: std::time::Duration,
    ) -> Result<(), CohortError> {
        self.cert_rotation
            .set(Arc::new(PeerCertRotation::new(
                pins,
                core,
                Arc::clone(&self.audit),
                timer,
                grace,
            )))
            .map_err(|_| {
                CohortError::EAuditAppendFailed(
                    "cohort cert-rotation control is already installed; a second install would \
                     redirect the signed-manifest trigger at a different mesh's trust planes"
                        .into(),
                )
            })?;
        // Review B1 — a version applied before this install raised only the floor held here;
        // carry it into the durable pin store (a no-op when the store already holds it).
        if let Some(rotation) = self.cert_rotation.get() {
            rotation.raise_manifest_floor(
                self.manifest_floor
                    .load(std::sync::atomic::Ordering::Acquire),
            );
        }
        // ⚠ RECONCILE IMMEDIATELY, and this is not belt-and-braces.
        //
        // The transport's accept loop and the manifest pull service are BOTH
        // live before this install runs (`bind_with_intake_sink` starts the
        // listener; `build_cohort_a2a_daemon_runtime` spawns the pull service;
        // the install happens after the builder returns). A signed push or a
        // pull response landing in that interval would advance the cached
        // manifest with `cert_rotation` still unset — and redelivery at the SAME
        // version returns `Confirmed`, which does no work, so the divergence
        // would persist silently until some LATER version arrived.
        //
        // Reconciling the CURRENT cached manifest against the live planes closes
        // that interval whenever it opened, and costs nothing when it did not:
        // an unchanged fingerprint takes no action at all, so on the ordinary
        // boot path this is a diff that finds nothing.
        let cached = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        let Some(rotation) = self.cert_rotation.get() else {
            return Ok(());
        };
        match cached.manifest.peer_configs_for(self.local_host.as_str()) {
            Ok(peers) => {
                rotation.reload(
                    &peers,
                    cached.manifest.version,
                    self.clock.now_secs(),
                    None,
                    self.local_leaf_declaration_event(&cached.manifest, None)
                        .as_ref(),
                    window_lag(&cached.manifest, &cached.manifest),
                )?;
            }
            // The SAME named row the reissue path writes for the identical
            // failure. Silently skipping here would make "this node cannot
            // derive a peer set for itself" observable on one path and invisible
            // on the other.
            Err(error) => {
                self.audit.append(&CohortAuditEvent::CertRotationRefused {
                    peer: self.local_host.as_str().to_string(),
                    reason: error.to_string(),
                    version: cached.manifest.version,
                })?;
            }
        }
        Ok(())
    }

    /// Story 14-2a / AC1.5 — the rotation status an operator can read, or
    /// `None` when NO rotation control is installed in this process.
    ///
    /// `None` and `Some(vec![])` are DIFFERENT facts and an operator acts
    /// differently on each: "this process does not rotate at all" versus "this
    /// process rotates and nothing is in flight". Every `MAOS_ONE_SHOT` arm
    /// except `cohort-a2a-daemon` reads a cohort config without ever installing
    /// the control (the `maos run` cross-host arm type-erases its transport, so
    /// the planes are unreachable there by construction), so collapsing the two
    /// would tell an operator who pushed a signed reissue at such a process
    /// that the rotation completed cleanly.
    ///
    /// This is the READ half of the one seam [`Self::install_cert_rotation`]
    /// builds; the reissue path is the WRITE half. A verb and a noun, never two
    /// mechanisms.
    pub fn rotation_status(&self) -> Result<Option<Vec<RotationWindow>>, CohortError> {
        let Some(rotation) = self.cert_rotation.get() else {
            return Ok(None);
        };
        let peers = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .manifest
            .peer_configs_for(self.local_host.as_str())?;
        Ok(Some(rotation.status(&peers)))
    }

    pub fn manifest(&self) -> Result<CohortManifest, CohortError> {
        Ok(self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .manifest
            .clone())
    }

    pub fn version(&self) -> Result<u64, CohortError> {
        Ok(self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .manifest
            .version)
    }

    pub(crate) fn accept_consent(
        &self,
        sender_peer: &str,
        acting_role: Option<&str>,
        intent: &str,
        sender_version: Option<u64>,
    ) -> Result<(), CohortError> {
        let cached = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        accept_admits(
            &cached.manifest,
            sender_peer,
            acting_role,
            intent,
            sender_version,
        )
    }

    pub(crate) fn outbound_consent_context(
        &self,
        receiver_peer: &str,
        intent: &str,
    ) -> Result<OutboundConsentContext, CohortError> {
        let cached = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        send_context(
            &cached.manifest,
            self.local_host.as_str(),
            receiver_peer,
            intent,
        )
    }

    pub fn canonical_hash(&self) -> Result<[u8; 32], CohortError> {
        Ok(self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .canonical_hash)
    }

    /// Host identity bound when this verified state was loaded.
    pub fn local_host(&self) -> &HostId {
        &self.local_host
    }

    /// The exact verified, signed manifest artifact suitable for a push.
    pub fn signed_toml(&self) -> Result<String, CohortError> {
        Ok(self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .signed_toml
            .clone())
    }

    /// Drain verified peers awaiting a signed push. Router code only enqueues;
    /// composition owns outbound delivery and its cancellation lifecycle.
    pub fn take_pull_requests(&self) -> Result<Vec<HostId>, CohortError> {
        let mut requests = self
            .pull_requests
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        Ok(std::mem::take(&mut *requests))
    }

    /// Story 14-2b / AC1 — retain what an authenticated peer declared on its
    /// own `Pull`.
    ///
    /// Last-write-wins per peer, deliberately NOT a high-water mark: a
    /// `max()`-style record would survive the peer reverting to its on-disk
    /// manifest after a restart (`apply_reissue` writes only the in-memory
    /// cache — `state.rs:585-590` — while boot re-reads `file.manifest_path`),
    /// which is precisely the false convergence claim this story exists to
    /// prevent.
    fn record_peer_convergence(
        &self,
        peer: &HostId,
        peer_boot_nonce: u64,
        declared_version: u64,
        declared_hash: String,
    ) {
        // A zero nonce is the wire's legacy "generation unavailable" sentinel.
        // Such a declaration cannot meet AC2's restart-invalidatable contract.
        if peer_boot_nonce == 0 {
            return;
        }
        let observed_at_secs = self.clock.now_secs();
        let cached = match self.cached.lock() {
            Ok(cached) => cached,
            Err(_) => return,
        };
        // `peer_configs` are boot-static, so the reserved Pull path may still
        // admit a host removed by a hot reissue. Only current signed members
        // may refresh this current-cohort observation table.
        if !cached
            .manifest
            .members
            .iter()
            .any(|member| member.host_id == peer.as_str())
        {
            return;
        }
        let record = PeerConvergence {
            peer: peer.as_str().to_string(),
            declared_version,
            declared_hash,
            observed_at_secs,
            expires_at_secs: observed_at_secs.saturating_add(cached.manifest.t_stale_secs),
            pin_generation: peer_boot_nonce,
            state: CONVERGENCE_OBSERVED,
        };
        // Hold the manifest guard through insertion. A concurrent reissue
        // therefore either sees this old-member row and removes it, or wins
        // first and makes the membership check above reject it.
        if let Ok(mut table) = self.peer_convergence.lock() {
            table.insert(record.peer.clone(), record);
        }
        drop(cached);
    }

    /// Story 14-2b / AC3 — every retained declaration with its CURRENT validity
    /// (AC2) derived on read.
    ///
    /// ⚠ An empty vector is NOT agreement. It means no peer has ever declared a
    /// version to this host — the same distinction
    /// [`Self::rotation_status`]'s `Some(Healthy(vec![]))` draws, and the read
    /// surface must preserve it.
    pub fn peer_convergence(&self) -> Result<Vec<PeerConvergence>, CohortError> {
        let now = self.clock.now_secs();
        let table = self
            .peer_convergence
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        Ok(table
            .values()
            .map(|record| {
                let restarted = self.pin_generation(&record.peer) != record.pin_generation;
                let state = if restarted {
                    CONVERGENCE_RESTARTED
                } else if now > record.expires_at_secs {
                    CONVERGENCE_STALE
                } else {
                    CONVERGENCE_OBSERVED
                };
                PeerConvergence {
                    state,
                    ..record.clone()
                }
            })
            .collect())
    }

    /// Whether the live daemon installed the pin source needed to evaluate
    /// restart validity. A loaded manifest alone is not a live observer.
    pub fn convergence_observer_ready(&self) -> bool {
        self.cert_rotation.get().is_some()
    }

    /// The TLS leaf fingerprint configured on the live router, if rotation is installed.
    pub fn local_leaf_fingerprint(&self) -> Option<PeerCertFingerprint> {
        self.cert_rotation
            .get()
            .and_then(|rotation| rotation.local_leaf_fingerprint())
    }

    fn local_declared_fingerprint(&self, manifest: &CohortManifest) -> Option<PeerCertFingerprint> {
        manifest
            .members
            .iter()
            .find(|member| member.host_id == self.local_host.as_str())
            .and_then(|member| PeerCertFingerprint::parse(&member.fingerprint))
    }

    /// Story 23-7 / D-23-7-02 C — the fingerprint the signed manifest declares for this host and the
    /// manifest version it came from.
    pub fn local_declaration(&self) -> Result<Option<(PeerCertFingerprint, u64)>, CohortError> {
        let cached = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        Ok(self
            .local_declared_fingerprint(&cached.manifest)
            .map(|declared| (declared, cached.manifest.version)))
    }

    /// Story 23-7 — install the observer woken after a reissue diverges this host's declaration
    /// from its serving leaf. Set-once; `false` when one is already installed.
    #[must_use]
    pub fn install_local_leaf_observer(&self, observer: Arc<dyn Fn() + Send + Sync>) -> bool {
        self.local_leaf_observer.set(observer).is_ok()
    }

    /// Story 23-7 — journal what the own-leaf reconciler did, through the same sink as every other
    /// rotation row. `detail` names the peers that had not confirmed the declaring version
    /// (comma separated; empty when there were none).
    pub fn journal_self_leaf(
        &self,
        outcome: crate::audit::SelfLeafOutcome,
        on_disk: &str,
        declared: &str,
        version: u64,
        detail: &str,
    ) -> Result<(), CohortError> {
        self.audit.append(&CohortAuditEvent::SelfLeafReconciled {
            host: self.local_host.as_str().to_string(),
            outcome,
            on_disk: on_disk.to_string(),
            declared: declared.to_string(),
            version,
            detail: detail.to_string(),
        })
    }

    /// Stories 23-7 review B2 + B5 — commit this host's own-leaf swap ATOMICALLY with respect to
    /// manifest replacement, and only once the peers can accept it.
    ///
    /// Under the `cached` lock (so no reissue can replace the declaration between the check and
    /// the commit) it (1) re-reads this host's signed declaration and refuses to run `commit` if it
    /// is no longer `prepared_for` ([`OwnLeafCommit::Superseded`]), and (2) holds the swap
    /// ([`OwnLeafCommit::Held`]) until EVERY peer this host has an edge with has told this host,
    /// in a `Pull` it sent, that it holds the manifest version in force (14-2b's convergence table,
    /// restart- and lease-aware): a peer that has not applied the reissue still pins only the
    /// retiring leaf and would refuse this host in both directions, and its pulls over mTLS could
    /// not even fetch the reissue from us. An unreachable or silent peer must not hold the rotation
    /// forever: once the manifest's own stale lease has elapsed since this host applied it, the
    /// commit runs anyway (the peer is stale by that lease) and reports the unconfirmed peers.
    ///
    /// `commit` runs with the `cached` lock held: it MUST be quick and MUST NOT call back into this
    /// state (the transport's two in-memory write guards qualify).
    pub fn commit_own_leaf<R>(
        &self,
        prepared_for: &PeerCertFingerprint,
        commit: impl FnOnce() -> R,
    ) -> Result<OwnLeafCommit<R>, CohortError> {
        let cached = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        if self.local_declared_fingerprint(&cached.manifest).as_ref() != Some(prepared_for) {
            return Ok(OwnLeafCommit::Superseded);
        }
        let unconfirmed = self.unconfirmed_peers(&cached)?;
        let held_for = self.clock.now_secs().saturating_sub(cached.applied_at_secs);
        if !unconfirmed.is_empty() && held_for < cached.manifest.t_stale_secs {
            return Ok(OwnLeafCommit::Held { unconfirmed });
        }
        Ok(OwnLeafCommit::Committed {
            result: commit(),
            unconfirmed,
        })
    }

    /// The peers (this host's edges in the manifest in force) with no CURRENT convergence claim at
    /// a version `>=` the one in force (same hash at that version). Caller holds `cached`.
    fn unconfirmed_peers(&self, cached: &CachedManifest) -> Result<Vec<String>, CohortError> {
        let required = cached.manifest.version;
        let required_hash = hex::encode(cached.canonical_hash);
        let peers = cached.manifest.peer_configs_for(self.local_host.as_str())?;
        let now = self.clock.now_secs();
        let table = self
            .peer_convergence
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        Ok(peers
            .iter()
            .map(|peer| peer.peer_id.as_str())
            .filter(|peer| {
                !table.get(*peer).is_some_and(|record| {
                    now <= record.expires_at_secs
                        && self.pin_generation(peer) == record.pin_generation
                        && (record.declared_version > required
                            || (record.declared_version == required
                                && record.declared_hash == required_hash))
                })
            })
            .map(str::to_string)
            .collect())
    }

    /// A peer's `Pull` just told this host which version it holds: if this host's own leaf swap is
    /// pending, wake the reconciler now instead of leaving the swap to the next tick (the swap has
    /// to trail the last peer's apply by as little as possible: the rotation grace is sized for
    /// it).
    fn wake_own_leaf_reconciler(&self) {
        let Some(observer) = self.local_leaf_observer.get() else {
            return;
        };
        let pending = self
            .local_leaf_fingerprint()
            .zip(self.local_declaration().ok().flatten())
            .is_some_and(|(serving, (declared, _))| serving != declared);
        if pending {
            observer();
        }
    }

    /// Story 14-2c — `Some` only when this manifest MOVES the local host's
    /// declared fingerprint off the identity the transport still serves, and
    /// the local row actually changed against the manifest being replaced
    /// (`previous` is `None` at the install site, which has no predecessor).
    /// Re-firing on every later reissue while a divergence persists would
    /// date the move to versions that never touched the local row.
    fn local_leaf_declaration_event(
        &self,
        manifest: &CohortManifest,
        previous: Option<&PeerCertFingerprint>,
    ) -> Option<CohortAuditEvent> {
        let serving = self.local_leaf_fingerprint()?;
        let declared = self.local_declared_fingerprint(manifest)?;
        (serving != declared && previous != Some(&declared)).then(|| {
            CohortAuditEvent::LocalLeafDeclarationMoved {
                host: self.local_host.as_str().to_string(),
                serving: serving.wire(),
                declared: declared.wire(),
                version: manifest.version,
            }
        })
    }

    /// The peer's active pin generation, or `0` when no rotation control is
    /// installed in this process or no active pin exists for that peer.
    ///
    /// `0` mirrors the wire sentinel (`router.rs:1344`): an unavailable
    /// generation cannot be compared, so it is never reported as a restart.
    fn pin_generation(&self, peer: &str) -> u64 {
        self.cert_rotation
            .get()
            .and_then(|rotation| rotation.pin_generation(peer))
            .unwrap_or(0)
    }

    /// Review B1 — adopt the durable floor the pin file recorded at boot, before any manifest
    /// traffic can reach [`Self::apply_reissue`]. Never lowers the floor.
    pub fn install_manifest_floor(&self, floor: u64) {
        self.manifest_floor
            .fetch_max(floor, std::sync::atomic::Ordering::AcqRel);
    }

    /// The larger of the floor held here and the one the installed pin store carries.
    fn effective_floor(&self) -> u64 {
        let held = self
            .manifest_floor
            .load(std::sync::atomic::Ordering::Acquire);
        self.cert_rotation
            .get()
            .map_or(held, |rotation| held.max(rotation.manifest_floor()))
    }

    /// Raise the floor here and, when the pin store is installed, durably there.
    fn raise_manifest_floor(&self, version: u64) {
        self.manifest_floor
            .fetch_max(version, std::sync::atomic::Ordering::AcqRel);
        if let Some(rotation) = self.cert_rotation.get() {
            rotation.raise_manifest_floor(version);
        }
    }

    /// Applies a signed reissue. A same-version, same-body manifest is an
    /// idempotent confirmation. A lower verified version or a divergent verified
    /// body at the same version is rejected with a specific fork discriminant.
    pub fn apply_reissue(&self, manifest_toml: &str) -> Result<ReissueOutcome, CohortError> {
        let candidate = match CohortManifest::parse_and_validate(manifest_toml, &self.pinned) {
            Ok(candidate) => candidate,
            Err(CohortError::ECohortAuthorityUnpinned { .. }) => {
                return self
                    .reject_non_authority(reissue_version(manifest_toml).unwrap_or_default());
            }
            Err(error) => return Err(error),
        };
        if candidate.verify_signature(&self.pinned).is_err() {
            return self.reject_non_authority(candidate.version);
        }
        let candidate_hash = candidate.canonical_hash();

        let mut cached = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        let seen_version = cached.manifest.version;
        let rejected_version = candidate.version;

        if candidate.cohort_id != cached.manifest.cohort_id {
            let expected_cohort_id = cached.manifest.cohort_id.clone();
            drop(cached);
            let error = CohortError::ECohortIdMismatch {
                expected_cohort_id: expected_cohort_id.clone(),
                rejected_cohort_id: candidate.cohort_id,
                seen_version,
                rejected_version,
            };
            self.audit.append(&CohortAuditEvent::ReissueRejected {
                cohort_id: expected_cohort_id,
                seen_version,
                rejected_version,
                reason: error.to_string(),
            })?;
            return Err(error);
        }

        let rejection =
            if schema_is_downgrade(cached.manifest.schema_version, candidate.schema_version) {
                Some(CohortManifestForkReason::SchemaDowngrade)
            } else if candidate.version < seen_version {
                Some(CohortManifestForkReason::VersionRegression)
            } else if candidate.version == seen_version && candidate_hash != cached.canonical_hash {
                Some(CohortManifestForkReason::ConcurrentFork)
            } else if candidate.version < self.effective_floor() {
                // Story 23-7 / D-23-7-09 A — below the highest version this host ever applied,
                // which survives a restart (the in-memory `seen_version` does not).
                Some(CohortManifestForkReason::VersionBelowDurableFloor)
            } else {
                None
            };

        if let Some(reason) = rejection {
            let error = CohortError::ECohortManifestFork {
                reason,
                seen_version,
                rejected_version,
            };
            self.audit.append(&CohortAuditEvent::ReissueRejected {
                cohort_id: cached.manifest.cohort_id.clone(),
                seen_version,
                rejected_version,
                reason: error.to_string(),
            })?;
            return Err(error);
        }

        if candidate.version == seen_version {
            cached.confirmed_at_secs = self.clock.now_secs();
            return Ok(ReissueOutcome::Confirmed {
                version: seen_version,
            });
        }

        // ── Story 14-2a / AC2.1 — THE RUNTIME HALF OF `main.rs:9855` ────────
        //
        // `reconcile_transport_identity_with_manifest` enforces
        // config-vs-manifest certificate agreement exactly ONCE, at boot,
        // before the transport binds. This line is where that agreement is
        // re-established when the signed truth moves underneath it.
        //
        // Placement is load-bearing on three counts:
        //   * AFTER every authentication, cohort-id, schema and monotonicity
        //     check — an unsigned, forked or regressed manifest can never reach
        //     the trust planes;
        //   * BEFORE `*cached` is replaced, while the accepted row is appended
        //     only after every peer transition succeeds. A failed transition
        //     therefore rolls back without leaving an accepted row for a
        //     manifest this process never committed;
        //   * INSIDE the `cached` guard — two concurrent reissues cannot
        //     interleave a projection with a commit. `RotationGraceTimer`
        //     implementations therefore MUST NOT call back into this state.
        //
        // The projection is 12.1's `peer_configs_for`, which validates every
        // fingerprint through `PeerCertFingerprint::parse` (AC2.4). Nothing
        // here accepts a fingerprint from any other source.
        // Review B1 — the durable floor is committed BEFORE the signed artifact, the trust
        // windows or the cached manifest become visible, and under the `cached` guard, so a
        // crash anywhere after this line finds the floor already at this version and no apply
        // can interleave with the raise. A failed apply leaves the floor raised, which only
        // narrows (a retry of the same version is still accepted; a lower one is not).
        self.raise_manifest_floor(candidate.version);
        let accepted_event = CohortAuditEvent::MemberReissueAccepted {
            cohort_id: candidate.cohort_id.clone(),
            version: candidate.version,
            canonical_hash: candidate_hash,
        };
        let accepted_by_rotation = if let Some(rotation) = self.cert_rotation.get() {
            match candidate.peer_configs_for(self.local_host.as_str()) {
                Ok(peers) => {
                    let local_leaf_event = self.local_leaf_declaration_event(
                        &candidate,
                        self.local_declared_fingerprint(&cached.manifest).as_ref(),
                    );
                    rotation.reload(
                        &peers,
                        candidate.version,
                        self.clock.now_secs(),
                        Some(&accepted_event),
                        local_leaf_event.as_ref(),
                        window_lag(&candidate, &cached.manifest),
                    )?;
                    true
                }
                // A reissue that REMOVES this host has no position from which
                // to project edges. Refusing the manifest would let a removed
                // member ignore its own removal, so the manifest applies and
                // the roster gate refuses the traffic — but the operator gets a
                // named row rather than a silent no-op.
                Err(error) => {
                    self.audit.append(&CohortAuditEvent::CertRotationRefused {
                        peer: self.local_host.as_str().to_string(),
                        reason: error.to_string(),
                        version: candidate.version,
                    })?;
                    false
                }
            }
        } else {
            false
        };
        if !accepted_by_rotation {
            self.audit.append(&accepted_event)?;
        }
        let version = candidate.version;
        let local_leaf_diverged = self
            .local_leaf_fingerprint()
            .zip(self.local_declared_fingerprint(&candidate))
            .is_some_and(|(serving, declared)| serving != declared);
        *cached = CachedManifest {
            manifest: candidate,
            canonical_hash: candidate_hash,
            signed_toml: manifest_toml.to_string(),
            confirmed_at_secs: self.clock.now_secs(),
            applied_at_secs: self.clock.now_secs(),
        };
        // The observation surface represents the CURRENT signed cohort. The
        // cached-manifest guard serializes this purge with record insertion.
        if let Ok(mut table) = self.peer_convergence.lock() {
            table.retain(|peer, _| {
                cached
                    .manifest
                    .members
                    .iter()
                    .any(|member| member.host_id == peer.as_str())
            });
        }
        // Correlation capabilities are grants under one signed manifest
        // snapshot. A reissue invalidates every in-flight exemption; callers
        // must mint a fresh request under the new matrix.
        self.outstanding_digest_reads
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .clear();
        self.admitted_digest_reads
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .clear();
        self.pending_digest_replies
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .clear();
        // The observer may read this state: it runs with the `cached` lock RELEASED.
        drop(cached);
        if local_leaf_diverged {
            if let Some(observer) = self.local_leaf_observer.get() {
                observer();
            }
        }
        Ok(ReissueOutcome::Applied { version })
    }

    /// Records an authority-originated signed reissue before applying it to the
    /// authority's own verified cache. The caller retains key custody and must
    /// supply a fully signed TOML artifact; this state never stores a signing
    /// key. Both origin and member-adoption events are therefore observable.
    pub fn issue_reissue(&self, manifest_toml: &str) -> Result<ReissueOutcome, CohortError> {
        let candidate = match CohortManifest::parse_and_validate(manifest_toml, &self.pinned) {
            Ok(candidate) => candidate,
            Err(CohortError::ECohortAuthorityUnpinned { .. }) => {
                return self
                    .reject_non_authority(reissue_version(manifest_toml).unwrap_or_default());
            }
            Err(error) => return Err(error),
        };
        if candidate.verify_signature(&self.pinned).is_err() {
            return self.reject_non_authority(candidate.version);
        }
        if candidate.cohort_id != self.manifest()?.cohort_id {
            return self.apply_reissue(manifest_toml);
        }
        let hash = candidate.canonical_hash();
        let seen_version = self.version()?;
        if candidate.version <= seen_version {
            return Err(CohortError::ECohortManifestFork {
                reason: CohortManifestForkReason::VersionRegression,
                seen_version,
                rejected_version: candidate.version,
            });
        }
        self.audit
            .append(&CohortAuditEvent::AuthorityReissueIssued {
                cohort_id: candidate.cohort_id,
                version: candidate.version,
                canonical_hash: hash,
            })?;
        self.apply_reissue(manifest_toml)
    }

    /// Staleness is evaluated against the signed, code-clamped lease. At the
    /// precise lease boundary the state remains current; it becomes stale only
    /// after the elapsed duration exceeds the lease.
    pub fn is_fresh(&self) -> bool {
        let Ok(cached) = self.cached.lock() else {
            return false;
        };
        self.clock
            .now_secs()
            .saturating_sub(cached.confirmed_at_secs)
            <= cached.manifest.t_stale_secs
    }

    /// Freshness and the manifest from ONE locked snapshot: the lease check
    /// and the returned manifest are evaluated against the same
    /// `confirmed_at_secs`, so a lease turning stale between two separate
    /// lock acquisitions can no longer admit a grant on a stale snapshot
    /// (13.3 review). `Ok(None)` = the lease is stale (fail-closed,
    /// distinguishable from an unavailable state, which is `Err`).
    pub fn manifest_if_fresh(&self) -> Result<Option<CohortManifest>, CohortError> {
        let cached = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        let fresh = self
            .clock
            .now_secs()
            .saturating_sub(cached.confirmed_at_secs)
            <= cached.manifest.t_stale_secs;
        Ok(fresh.then(|| cached.manifest.clone()))
    }

    /// Refresh before half of the signed stale lease elapses, leaving the other
    /// half to receive and verify the authority's signed confirmation.
    pub fn confirmation_interval(&self) -> Result<Duration, CohortError> {
        let cached = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        Ok(confirmation_of(&cached.manifest))
    }

    fn reject_non_authority(&self, rejected_version: u64) -> Result<ReissueOutcome, CohortError> {
        let (cohort_id, seen_version) = {
            let cached = self
                .cached
                .lock()
                .map_err(|_| CohortError::EStatePoisoned)?;
            (cached.manifest.cohort_id.clone(), cached.manifest.version)
        };
        let error = CohortError::ECohortManifestFork {
            reason: CohortManifestForkReason::NonAuthoritySigner,
            seen_version,
            rejected_version,
        };
        self.audit.append(&CohortAuditEvent::ReissueRejected {
            cohort_id,
            seen_version,
            rejected_version,
            reason: error.to_string(),
        })?;
        Err(error)
    }

    fn control_rejection(&self, error: CohortError) -> CohortReissueRejection {
        let (seen_version, rejected_version) = match &error {
            CohortError::ECohortManifestFork {
                seen_version,
                rejected_version,
                ..
            }
            | CohortError::ECohortIdMismatch {
                seen_version,
                rejected_version,
                ..
            } => (Some(*seen_version), Some(*rejected_version)),
            _ => (self.version().ok(), None),
        };
        CohortReissueRejection {
            reason: error.to_string(),
            seen_version,
            rejected_version,
        }
    }

    /// Story 12.3 — record an explicit transport-level absence marker for a
    /// probed member (P2a/P3). Observability only; the caller classifies the
    /// probe result (`HaltReceiptDistributor::classify_presence`) and records
    /// the induced-loss outcome here. Poisoned lock is a best-effort drop —
    /// absence is never a trust decision.
    pub fn record_absence(&self, member: &HostId, kind: AbsenceKind) {
        if let Ok(mut table) = self.halt_absence.lock() {
            table.insert(member.as_str().to_string(), kind);
        }
    }

    /// Story 12.3 — the count of DISTINCT halt receipts observed from `member`
    /// (dedup by `halt_id`, P4). Feeds the 12.4 digest. A poisoned lock reads 0.
    pub fn present_receipt_count(&self, member: &HostId) -> usize {
        self.halt_presence
            .lock()
            .ok()
            .and_then(|table| table.get(member.as_str()).map(HashSet::len))
            .unwrap_or(0)
    }

    /// Story 12.3 — whether a specific receipt (by `halt_id`) is present for a
    /// member.
    pub fn is_receipt_present(&self, member: &HostId, halt_id: &str) -> bool {
        self.halt_presence
            .lock()
            .ok()
            .and_then(|table| table.get(member.as_str()).map(|ids| ids.contains(halt_id)))
            .unwrap_or(false)
    }

    /// Story 12.3 — the recorded absence marker for a member, if any.
    pub fn absence_of(&self, member: &HostId) -> Option<AbsenceKind> {
        self.halt_absence
            .lock()
            .ok()
            .and_then(|table| table.get(member.as_str()).copied())
    }

    /// Story 13.6a — the operator-signed team THIS host speaks for under the
    /// verified cache, or `None` when no signed declaration exists (fail-closed).
    pub fn team_of_local_host(&self) -> Option<String> {
        let cached = self.cached.lock().ok()?;
        cached
            .manifest
            .team_of_host(self.local_host.as_str())
            .map(|team| team.as_str().to_string())
    }

    /// The consent verdict body, evaluated against an already-locked manifest
    /// snapshot so [`CohortManifestGate::consent_and_team`] can pair it with the
    /// team declaration from the SAME snapshot (Story 13.6a review P2).
    #[allow(clippy::too_many_arguments)]
    fn consent_decision_at(
        &self,
        manifest: &CohortManifest,
        confirmed_at_secs: u64,
        seam: CohortConsentSeam,
        counterparty: &HostId,
        acting_role: Option<&str>,
        intent: &str,
        sender_manifest_version: Option<u64>,
    ) -> CohortConsentVerdict {
        // A peer outside the roster is a mixed-deployment bilateral path, not a
        // cohort denial. This preserves the legacy defer behavior.
        if !manifest
            .members
            .iter()
            .any(|member| member.host_id == counterparty.as_str())
        {
            return CohortConsentVerdict::Defer;
        }
        let current = self.clock.now_secs().saturating_sub(confirmed_at_secs)
            <= manifest.t_stale_secs
            && manifest
                .members
                .iter()
                .any(|member| member.host_id == self.local_host.as_str());
        if !current {
            return CohortConsentVerdict::NotCurrent;
        }

        let decision = match seam {
            CohortConsentSeam::Send => send_context(
                manifest,
                self.local_host.as_str(),
                counterparty.as_str(),
                intent,
            )
            .map(|context| CohortConsentVerdict::AdmitOutbound {
                acting_role: context.acting_role,
                manifest_version: context.manifest_version,
            }),
            CohortConsentSeam::Accept => accept_admits(
                manifest,
                counterparty.as_str(),
                acting_role,
                intent,
                sender_manifest_version,
            )
            .map(|()| CohortConsentVerdict::Admit),
        };
        match decision {
            Ok(verdict) => verdict,
            Err(CohortError::EConsentPeerNotMember { ref peer, .. })
                if peer == counterparty.as_str() =>
            {
                CohortConsentVerdict::Defer
            }
            Err(CohortError::EConsentActingRoleAbsent) => {
                CohortConsentVerdict::Deny(CohortConsentDenial::ActingRoleAbsent)
            }
            Err(CohortError::EConsentManifestVersionAbsent) => {
                CohortConsentVerdict::Deny(CohortConsentDenial::ManifestVersionAbsent)
            }
            Err(CohortError::EConsentRoleNotEntitled { .. }) => {
                CohortConsentVerdict::Deny(CohortConsentDenial::RoleNotEntitled)
            }
            Err(CohortError::EConsentTupleDenied { .. }) => {
                CohortConsentVerdict::Deny(CohortConsentDenial::NoGrant)
            }
            Err(CohortError::ECohortManifestSkew {
                sender_version,
                receiver_version,
                delta,
            }) => CohortConsentVerdict::Deny(CohortConsentDenial::ManifestSkew {
                sender_version,
                receiver_version,
                delta,
            }),
            Err(_) => CohortConsentVerdict::Deny(CohortConsentDenial::StateUnavailable),
        }
    }

    /// The seam-relevant team declaration read from an already-locked snapshot,
    /// gated on the presented leaf fingerprint EQUALING the signed
    /// [`CohortMember::fingerprint`] — the cert-bound, non-seed-derived axis
    /// D-3 designates (Story 13.6a review P1). Fail-closed in four independent
    /// ways: no presented fingerprint, no such member, a certificate the signed
    /// manifest does not name (including a stale cert after rotation), and a
    /// pre-V4 / team-less member all return `None`.
    fn team_declaration_at(
        &self,
        manifest: &CohortManifest,
        seam: CohortConsentSeam,
        counterparty: &HostId,
        endpoint_fingerprint: Option<&PeerCertFingerprint>,
    ) -> Option<String> {
        let host = match seam {
            CohortConsentSeam::Send => self.local_host.as_str(),
            CohortConsentSeam::Accept => counterparty.as_str(),
        };
        let presented = endpoint_fingerprint?;
        let member = manifest.members.iter().find(|m| m.host_id == host)?;
        let signed = PeerCertFingerprint::parse(&member.fingerprint)?;
        if presented != &signed {
            return None;
        }
        manifest
            .team_of_host(host)
            .map(|team| team.as_str().to_string())
    }
}

fn reissue_version(manifest_toml: &str) -> Option<u64> {
    toml::from_str::<toml::Value>(manifest_toml)
        .ok()?
        .get("version")?
        .as_integer()
        .and_then(|version| u64::try_from(version).ok())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReissueOutcome {
    Applied { version: u64 },
    Confirmed { version: u64 },
}

impl CohortManifestGate for CohortManifestState {
    fn consent_decision(
        &self,
        seam: CohortConsentSeam,
        counterparty: &HostId,
        acting_role: Option<&str>,
        intent: &str,
        sender_manifest_version: Option<u64>,
    ) -> CohortConsentVerdict {
        let cached = match self.cached.lock() {
            Ok(cached) => cached,
            Err(_) => return CohortConsentVerdict::Deny(CohortConsentDenial::StateUnavailable),
        };
        self.consent_decision_at(
            &cached.manifest,
            cached.confirmed_at_secs,
            seam,
            counterparty,
            acting_role,
            intent,
            sender_manifest_version,
        )
    }

    fn apply_reissue(
        &self,
        verified_peer: &HostId,
        peer_boot_nonce: u64,
        frame: &maos_domain::frame::IacFrame,
    ) -> Result<CohortReissueDisposition, CohortReissueRejection> {
        match CohortManifestControl::from_frame(frame) {
            Ok(CohortManifestControl::Push { manifest_toml }) => self
                .apply_reissue(&manifest_toml)
                .map(|outcome| match outcome {
                    ReissueOutcome::Applied { version } => {
                        CohortReissueDisposition::Applied { version }
                    }
                    ReissueOutcome::Confirmed { version } => {
                        CohortReissueDisposition::Confirmed { version }
                    }
                })
                .map_err(|error| self.control_rejection(error)),
            Ok(CohortManifestControl::Pull {
                known_version,
                known_hash,
            }) => {
                // Story 14-2b / AC1 — the ONLY receive site for these two
                // values, and until this line the only one that discarded them.
                // `verified_peer` is the TLS-authenticated identity
                // (`router.rs:1755-1777` refuses the frame unless
                // `frame.from.host_id` equals the verified peer), so the
                // attribution is trustworthy even though the content is a
                // self-report.
                self.record_peer_convergence(
                    verified_peer,
                    peer_boot_nonce,
                    known_version,
                    known_hash,
                );
                self.wake_own_leaf_reconciler();
                self.pull_requests
                    .lock()
                    .map_err(|_| CohortReissueRejection {
                        reason: CohortError::EStatePoisoned.to_string(),
                        seen_version: self.version().ok(),
                        rejected_version: None,
                    })?
                    .push(verified_peer.clone());
                Ok(CohortReissueDisposition::PullRequested)
            }
            Err(error) => Err(CohortReissueRejection {
                reason: error.to_string(),
                seen_version: self.version().ok(),
                rejected_version: None,
            }),
        }
    }

    /// Story 13.6a (AC2, review P1/P2) — consent verdict AND team declaration
    /// from ONE locked snapshot: a hot reissue cannot slip a team change
    /// between the identity check and the admission decision, and the
    /// declaration is returned only when the presented leaf fingerprint equals
    /// the signed [`CohortMember::fingerprint`] (the D-3 axis). Fail-closed:
    /// a poisoned lock denies and declares nothing.
    fn consent_and_team(
        &self,
        seam: CohortConsentSeam,
        counterparty: &HostId,
        endpoint_fingerprint: Option<&PeerCertFingerprint>,
        acting_role: Option<&str>,
        intent: &str,
        sender_manifest_version: Option<u64>,
    ) -> (CohortConsentVerdict, Option<String>) {
        let cached = match self.cached.lock() {
            Ok(cached) => cached,
            Err(_) => {
                return (
                    CohortConsentVerdict::Deny(CohortConsentDenial::StateUnavailable),
                    None,
                );
            }
        };
        let verdict = self.consent_decision_at(
            &cached.manifest,
            cached.confirmed_at_secs,
            seam,
            counterparty,
            acting_role,
            intent,
            sender_manifest_version,
        );
        let team =
            self.team_declaration_at(&cached.manifest, seam, counterparty, endpoint_fingerprint);
        (verdict, team)
    }
}

impl HaltReceiptObserver for CohortManifestState {
    /// Record receipt-presence for the authenticated `member` (P5r — the caller
    /// [`crate::A2ARouterCore::handle_intake_verified`] has proven
    /// `frame.from.host_id == verified_peer`, and the receipt payload carries no
    /// host identity, so `member` is the sole trustworthy emitter). Dedup is by
    /// the receipt's stable `halt_id` (P4). A frame that is not a well-formed
    /// halt-receipt control envelope is silently ignored (it never reaches this
    /// arm on the production path). Observability ONLY — this type has no method
    /// that resolves, resumes, or overrides a halt, and `maos-cohort` does not
    /// depend on `maos-kernel-core`, so the arbitration sink is unreachable (AC3).
    fn observe_receipt(&self, member: &HostId, frame: &IacFrame) {
        let Ok(control) = HaltReceiptControl::from_frame(frame) else {
            return;
        };
        let halt_id = control.halt_id().to_string();
        if let Ok(mut table) = self.halt_presence.lock() {
            table
                .entry(member.as_str().to_string())
                .or_default()
                .insert(halt_id);
        }
    }
}

impl CohortManifestState {
    pub fn note_digest_request_sent(
        &self,
        peer: &HostId,
        request_id: &str,
        scope: &str,
    ) -> Result<(), CohortError> {
        let version = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .manifest
            .version;
        let mut outstanding = self
            .outstanding_digest_reads
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        let received = self
            .received_digest_summaries
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        if outstanding.keys().any(|(_, id)| id == request_id)
            || received.keys().any(|(_, id)| id == request_id)
        {
            return Err(CohortError::EInvalidDigestRequest(
                "request_id must be globally unique for this reader".into(),
            ));
        }
        if outstanding.len() >= MAX_PENDING_DIGEST_READS {
            return Err(CohortError::EDigestCapacityExceeded(
                "reader has too many outstanding requests".into(),
            ));
        }
        outstanding.insert(
            (peer.as_str().to_string(), request_id.to_string()),
            DigestGrant {
                scope: scope.to_string(),
                manifest_version: version,
            },
        );
        Ok(())
    }

    pub fn cancel_digest_request(
        &self,
        peer: &HostId,
        request_id: &str,
    ) -> Result<(), CohortError> {
        self.outstanding_digest_reads
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .remove(&(peer.as_str().to_string(), request_id.to_string()));
        Ok(())
    }

    pub fn complete_admitted_digest_reply(
        &self,
        peer: &HostId,
        request_id: &str,
    ) -> Result<(), CohortError> {
        self.admitted_digest_reads
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .remove(&(peer.as_str().to_string(), request_id.to_string()));
        Ok(())
    }

    pub fn drain_pending_digest_replies(
        &self,
    ) -> Result<Vec<(String, String, String)>, CohortError> {
        Ok(std::mem::take(
            &mut *self
                .pending_digest_replies
                .lock()
                .map_err(|_| CohortError::EStatePoisoned)?,
        ))
    }

    pub fn requeue_pending_digest_reply(
        &self,
        requester: String,
        request_id: String,
        scope: String,
    ) -> Result<(), CohortError> {
        let mut queue = self
            .pending_digest_replies
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        if queue.len() >= MAX_PENDING_DIGEST_READS {
            return Err(CohortError::EDigestCapacityExceeded(
                "target has too many pending replies".into(),
            ));
        }
        if !queue
            .iter()
            .any(|(peer, id, _)| peer == &requester && id == &request_id)
        {
            queue.push((requester, request_id, scope));
        }
        Ok(())
    }

    pub fn digest_summary(&self, peer: &HostId, request_id: &str) -> Option<DigestSummary> {
        self.received_digest_summaries.lock().ok().and_then(|map| {
            map.get(&(peer.as_str().to_string(), request_id.to_string()))
                .cloned()
        })
    }

    pub fn digest_summary_count(&self) -> usize {
        self.received_digest_summaries
            .lock()
            .map(|map| map.len())
            .unwrap_or(0)
    }
}

impl DigestReadPort for CohortManifestState {
    /// Parse a `cohort:digest-read` frame into its request/reply class. A frame
    /// that is not a well-formed digest-read envelope is `NotDigest` (the router
    /// then treats it as an ordinary consent-gated frame).
    fn classify(&self, frame: &IacFrame) -> DigestFrameClass {
        match DigestReadControl::from_frame(frame) {
            Ok(DigestReadControl::Request { request_id, .. }) => {
                DigestFrameClass::Request { request_id }
            }
            Ok(DigestReadControl::Reply { request_id, .. }) => {
                DigestFrameClass::Reply { request_id }
            }
            Err(_)
                if frame
                    .consent_envelope
                    .as_ref()
                    .and_then(|envelope| envelope.intent_class.as_ref())
                    .is_some_and(|intent| {
                        intent
                            .as_str()
                            .eq_ignore_ascii_case(COHORT_INTENT_DIGEST_READ)
                    }) =>
            {
                DigestFrameClass::Invalid
            }
            Err(_) => DigestFrameClass::NotDigest,
        }
    }

    fn note_admitted_request_guarded(
        &self,
        requester: &HostId,
        request_id: &str,
        frame: &IacFrame,
        before_commit: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<(), String> {
        let DigestReadControl::Request {
            request_id: parsed_id,
            scope,
        } = DigestReadControl::from_frame(frame).map_err(|error| error.to_string())?
        else {
            return Err("admitted digest frame is not a request".into());
        };
        if parsed_id != request_id {
            return Err("classified request id changed before admission".into());
        }
        let version = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned.to_string())?
            .manifest
            .version;
        let key = (requester.as_str().to_string(), request_id.to_string());
        let mut admitted = self
            .admitted_digest_reads
            .lock()
            .map_err(|_| CohortError::EStatePoisoned.to_string())?;
        if admitted.contains_key(&key) {
            return Ok(());
        }
        let mut queue = self
            .pending_digest_replies
            .lock()
            .map_err(|_| CohortError::EStatePoisoned.to_string())?;
        if admitted.len() >= MAX_PENDING_DIGEST_READS || queue.len() >= MAX_PENDING_DIGEST_READS {
            return Err(CohortError::EDigestCapacityExceeded(
                "target has too many admitted digest reads".into(),
            )
            .to_string());
        }
        before_commit()?;
        self.audit
            .append(&CohortAuditEvent::DigestReadRequested {
                requester: requester.as_str().to_string(),
                request_id: request_id.to_string(),
                scope: scope.clone(),
            })
            .map_err(|error| error.to_string())?;
        admitted.insert(
            key.clone(),
            DigestGrant {
                scope: scope.clone(),
                manifest_version: version,
            },
        );
        queue.push((key.0, key.1, scope));
        Ok(())
    }

    fn authorize_reply_send(&self, peer: &HostId, request_id: &str) -> bool {
        let Ok(cached) = self.cached.lock() else {
            return false;
        };
        self.admitted_digest_reads
            .lock()
            .map(|grants| {
                grants
                    .get(&(peer.as_str().to_string(), request_id.to_string()))
                    .is_some_and(|grant| {
                        grant.manifest_version == cached.manifest.version
                            && grant.scope == DIGEST_DAILY_SCOPE
                    })
            })
            .unwrap_or(false)
    }

    fn observe_reply_guarded(
        &self,
        peer: &HostId,
        frame: &IacFrame,
        before_commit: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<DigestReplyObservation, String> {
        let DigestReadControl::Reply {
            request_id,
            summary,
        } = DigestReadControl::from_frame(frame).map_err(|error| error.to_string())?
        else {
            return Ok(DigestReplyObservation::Unauthorized);
        };
        let version = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned.to_string())?
            .manifest
            .version;
        let key = (peer.as_str().to_string(), request_id.clone());
        let mut outstanding = self
            .outstanding_digest_reads
            .lock()
            .map_err(|_| CohortError::EStatePoisoned.to_string())?;
        let mut received = self
            .received_digest_summaries
            .lock()
            .map_err(|_| CohortError::EStatePoisoned.to_string())?;
        if let Some(existing) = received.get(&key) {
            return if existing == &summary {
                Ok(DigestReplyObservation::Duplicate)
            } else {
                Err("conflicting replay attempted to replace a recorded digest summary".into())
            };
        }
        let Some(grant) = outstanding.get(&key) else {
            return Ok(DigestReplyObservation::Unauthorized);
        };
        if grant.manifest_version != version || grant.scope != DIGEST_DAILY_SCOPE {
            return Ok(DigestReplyObservation::Unauthorized);
        }
        // `j1-crosshost-2c` AC3.5 (`deferred-work.md:819`) — INVARIANT: nothing is
        // `Duplicate` until something is durable. Every authorization and conflict
        // check above has passed; nothing below this line has been published yet.
        // If the caller's commit fails, we publish NO dedup state and NO receipt
        // audit row, so the grant stays outstanding and the reply stays RETRYABLE.
        // Ordering matters: `DigestReplyReceived` asserts the reply "consumed a
        // live capability and was durably recorded", which is exactly what did not
        // happen when the commit fails.
        before_commit()?;
        self.audit
            .append(&CohortAuditEvent::DigestReplyReceived {
                member: peer.as_str().to_string(),
                request_id: request_id.clone(),
            })
            .map_err(|error| error.to_string())?;
        outstanding.remove(&key);
        received.insert(key, summary);
        Ok(DigestReplyObservation::Accepted)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ed25519_dalek::SigningKey;

    use super::*;
    use crate::audit::InMemoryCohortAuditSink;
    use crate::manifest::{
        CohortAuthority, CohortMember, ConsentMatrix, ConsentTuple, ManifestSignature, TeamEntry,
        COHORT_SCHEMA_V1, COHORT_SCHEMA_V2, COHORT_SCHEMA_V3, COHORT_SCHEMA_V4,
        RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
    };
    use maos_domain::frame::{ConsentEnvelope, FrameAddress, FramePayload, TelemetryEventPayload};
    use maos_domain::invariants::i1::IntentClass;
    use maos_domain::invariants::i13::IntentLineage;
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_domain::invariants::i8::A2AIntent;
    use maos_domain::region::Region;
    use maos_domain::team::TeamId;
    use maos_spirit_abi::identity::{FrameKind, SpiritId, SpiritRole};
    use smallvec::smallvec;

    fn signing_key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn signed_toml_for(
        version: u64,
        signer: &SigningKey,
        authority: &SigningKey,
        cohort_id: &str,
    ) -> String {
        let manifest = CohortManifest {
            schema_version: COHORT_SCHEMA_V1,
            cohort_id: cohort_id.to_string(),
            version,
            authority: CohortAuthority {
                threshold: 1,
                keys: vec![hex::encode(authority.verifying_key().to_bytes())],
            },
            members: vec![CohortMember {
                host_id: "host-a".into(),
                fingerprint: format!("sha256:{}", "ab".repeat(32)),
                roles: vec!["worker".into()],
                team: None,
            }],
            consent: ConsentMatrix::default(),
            reserved_intents: vec![
                RESERVED_INTENT_REISSUE.into(),
                RESERVED_INTENT_HALT_RECEIPT.into(),
            ],
            t_stale_secs: 120,
            teams: None,
            signature: ManifestSignature { sig: String::new() },
            cross_team_consent: Vec::new(),
        }
        .signed_with(signer);
        toml::to_string(&manifest).expect("serializable test manifest")
    }

    fn signed_tenant_toml(schema_version: u64, version: u64, signer: &SigningKey) -> String {
        let mut manifest: CohortManifest =
            toml::from_str(&signed_toml(version, signer, signer)).unwrap();
        manifest.schema_version = schema_version;
        manifest.teams = Some(vec![
            TeamEntry {
                team_id: TeamId::new("team-a").unwrap(),
                region: Region::canonicalize("region-a").unwrap(),
                datname: "maos_team_a".to_string(),
                members: vec![SpiritId::from("spirit-a")],
            },
            TeamEntry {
                team_id: TeamId::new("team-b").unwrap(),
                region: Region::canonicalize("region-b").unwrap(),
                datname: "maos_team_b".to_string(),
                members: vec![SpiritId::from("spirit-b")],
            },
        ]);
        // Story 13.6a — at V4 the roster also declares which team each host
        // speaks for. `host-a` → `team-a`, every other declared member →
        // `team-b`, so both the match and the mismatch axis are reachable.
        if schema_version == crate::manifest::COHORT_SCHEMA_V4 {
            for member in manifest.members.iter_mut() {
                member.team = Some(if member.host_id == "host-a" {
                    TeamId::new("team-a").unwrap()
                } else {
                    TeamId::new("team-b").unwrap()
                });
            }
        }
        toml::to_string(&manifest.signed_with(signer)).unwrap()
    }

    fn digest_frame(from: &str, to: &str, control: DigestReadControl) -> IacFrame {
        let from_address = FrameAddress {
            spirit_id: SpiritId::from("digest"),
            host_id: Some(HostId(from.into())),
            role: Some(SpiritRole::Worker),
        };
        IacFrame {
            frame_id: [7; 16],
            timestamp_ns: 0,
            logical_clock: 1,
            from: from_address.clone(),
            to: smallvec![FrameAddress {
                spirit_id: SpiritId::from("digest"),
                host_id: Some(HostId(to.into())),
                role: None,
            }],
            kind: FrameKind::TelemetryEvent,
            intent: IntentClass::Readonly,
            payload: FramePayload::TelemetryEvent(
                control.telemetry_payload().expect("valid digest control"),
            ),
            auto_marker: FrameOrigin::SpiritAuto,
            consent_envelope: Some(ConsentEnvelope::with_fine_grained_intent(
                from_address,
                A2AIntent::new(COHORT_INTENT_DIGEST_READ),
            )),
            intent_lineage: IntentLineage::default(),
        }
    }

    fn signed_toml_with_stale_secs(
        version: u64,
        signer: &SigningKey,
        authority: &SigningKey,
        t_stale_secs: u64,
    ) -> String {
        let mut manifest: CohortManifest =
            toml::from_str(&signed_toml_for(version, signer, authority, "cohort-test"))
                .expect("fixture manifest parses");
        manifest.t_stale_secs = t_stale_secs;
        toml::to_string(&manifest.signed_with(signer)).expect("resigned fixture serializes")
    }

    fn signed_toml(version: u64, signer: &SigningKey, authority: &SigningKey) -> String {
        signed_toml_for(version, signer, authority, "cohort-test")
    }

    fn state(
        version: u64,
        signer: &SigningKey,
        audit: Arc<dyn CohortAuditSink>,
    ) -> CohortManifestState {
        let pins = PinnedAuthorityKeys::from_keys(vec![signer.verifying_key()]).unwrap();
        CohortManifestState::load(
            HostId("host-a".into()),
            &signed_toml(version, signer, signer),
            pins,
            audit,
        )
        .unwrap()
    }

    fn consent_state(version: u64, signer: &SigningKey) -> CohortManifestState {
        let manifest = CohortManifest {
            schema_version: COHORT_SCHEMA_V1,
            cohort_id: "consent-state".into(),
            version,
            authority: CohortAuthority {
                threshold: 1,
                keys: vec![hex::encode(signer.verifying_key().to_bytes())],
            },
            members: vec![
                CohortMember {
                    host_id: "host-a".into(),
                    fingerprint: format!("sha256:{}", "aa".repeat(32)),
                    roles: vec!["architect".into()],
                    team: None,
                },
                CohortMember {
                    host_id: "host-b".into(),
                    fingerprint: format!("sha256:{}", "bb".repeat(32)),
                    roles: vec!["receiver".into()],
                    team: None,
                },
            ],
            consent: ConsentMatrix {
                send: vec![ConsentTuple {
                    peer: "host-b".into(),
                    role: "receiver".into(),
                    intent: "cohort-work:write".into(),
                }],
                accept: vec![ConsentTuple {
                    peer: "host-a".into(),
                    role: "architect".into(),
                    intent: "cohort-work:write".into(),
                }],
            },
            reserved_intents: vec![
                RESERVED_INTENT_REISSUE.into(),
                RESERVED_INTENT_HALT_RECEIPT.into(),
            ],
            t_stale_secs: 120,
            teams: None,
            signature: ManifestSignature { sig: String::new() },
            cross_team_consent: Vec::new(),
        }
        .signed_with(signer);
        let pins = PinnedAuthorityKeys::from_keys(vec![signer.verifying_key()]).unwrap();
        CohortManifestState::load(
            HostId("host-b".into()),
            &toml::to_string(&manifest).unwrap(),
            pins,
            Arc::new(InMemoryCohortAuditSink::default()),
        )
        .unwrap()
    }

    #[test]
    fn consent_reads_the_live_manifest_version_for_each_frame() {
        let signer = signing_key(14);
        let state = consent_state(4, &signer);
        state
            .accept_consent("host-a", Some("architect"), "cohort-work:write", Some(4))
            .expect("same-version frame admits");

        let replacement = consent_state(6, &signer).signed_toml().unwrap();
        state.apply_reissue(&replacement).unwrap();
        assert!(matches!(
            state.accept_consent("host-a", Some("architect"), "cohort-work:write", Some(4),),
            Err(CohortError::ECohortManifestSkew {
                sender_version: 4,
                receiver_version: 6,
                delta: 2,
            })
        ));
    }

    #[test]
    fn applies_higher_verified_reissue_after_auditing() {
        let signer = signing_key(7);
        let audit = Arc::new(InMemoryCohortAuditSink::default());
        let state = state(1, &signer, audit.clone());

        assert_eq!(
            state
                .apply_reissue(&signed_toml(2, &signer, &signer))
                .unwrap(),
            ReissueOutcome::Applied { version: 2 }
        );
        assert_eq!(state.version().unwrap(), 2);
        assert!(matches!(
            audit.events().as_slice(),
            [CohortAuditEvent::MemberReissueAccepted { version: 2, .. }]
        ));
    }

    #[test]
    fn peer_absent_from_current_manifest_defers_to_bilateral_path() {
        let signer = signing_key(13);
        let state = state(1, &signer, Arc::new(InMemoryCohortAuditSink::default()));
        assert_eq!(
            CohortManifestGate::consent_decision(
                &state,
                CohortConsentSeam::Send,
                &HostId("host-b".into()),
                None,
                "cohort-work:write",
                None,
            ),
            CohortConsentVerdict::Defer,
            "an unknown peer remains eligible for the legacy bilateral path"
        );
    }

    #[test]
    fn authority_issue_is_journaled_before_member_adoption() {
        let signer = signing_key(6);
        let audit = Arc::new(InMemoryCohortAuditSink::default());
        let state = state(1, &signer, audit.clone());

        assert_eq!(
            state
                .issue_reissue(&signed_toml(2, &signer, &signer))
                .unwrap(),
            ReissueOutcome::Applied { version: 2 }
        );
        assert!(matches!(
            audit.events().as_slice(),
            [
                CohortAuditEvent::AuthorityReissueIssued { version: 2, .. },
                CohortAuditEvent::MemberReissueAccepted { version: 2, .. },
            ]
        ));
    }

    #[test]
    fn same_verified_body_confirms_but_divergent_body_forks() {
        let signer = signing_key(8);
        let audit = Arc::new(InMemoryCohortAuditSink::default());
        let state = state(2, &signer, audit);
        let same = signed_toml(2, &signer, &signer);

        assert_eq!(
            state.apply_reissue(&same).unwrap(),
            ReissueOutcome::Confirmed { version: 2 }
        );

        let divergent = signed_toml_with_stale_secs(2, &signer, &signer, 121);
        let error = state.apply_reissue(&divergent).unwrap_err();
        assert!(matches!(
            error,
            CohortError::ECohortManifestFork {
                reason: CohortManifestForkReason::ConcurrentFork,
                seen_version: 2,
                rejected_version: 2,
            }
        ));
    }

    #[test]
    fn lower_verified_version_is_a_version_regression() {
        let signer = signing_key(9);
        let state = state(2, &signer, Arc::new(InMemoryCohortAuditSink::default()));
        let error = state
            .apply_reissue(&signed_toml(1, &signer, &signer))
            .unwrap_err();

        assert!(matches!(
            error,
            CohortError::ECohortManifestFork {
                reason: CohortManifestForkReason::VersionRegression,
                seen_version: 2,
                rejected_version: 1,
            }
        ));
    }

    #[test]
    fn wrong_signer_is_journaled_with_actual_versions() {
        let authority = signing_key(10);
        let intruder = signing_key(11);
        let audit = Arc::new(InMemoryCohortAuditSink::default());
        let state = state(1, &authority, audit.clone());
        let error = state
            .apply_reissue(&signed_toml(2, &intruder, &authority))
            .unwrap_err();

        assert!(matches!(
            error,
            CohortError::ECohortManifestFork {
                reason: CohortManifestForkReason::NonAuthoritySigner,
                seen_version: 1,
                rejected_version: 2,
            }
        ));
        assert!(matches!(
            audit.events().as_slice(),
            [CohortAuditEvent::ReissueRejected {
                seen_version: 1,
                rejected_version: 2,
                ..
            }]
        ));
        assert_eq!(state.version().unwrap(), 1);
    }

    #[test]
    fn higher_version_cannot_replace_the_cohort_identity() {
        let authority = signing_key(11);
        let audit = Arc::new(InMemoryCohortAuditSink::default());
        let state = state(1, &authority, audit.clone());

        let error = state
            .apply_reissue(&signed_toml_for(2, &authority, &authority, "other-cohort"))
            .unwrap_err();

        assert!(matches!(
            error,
            CohortError::ECohortIdMismatch {
                seen_version: 1,
                rejected_version: 2,
                ..
            }
        ));
        assert!(matches!(
            audit.events().as_slice(),
            [CohortAuditEvent::ReissueRejected {
                seen_version: 1,
                rejected_version: 2,
                ..
            }]
        ));
        assert_eq!(state.manifest().unwrap().cohort_id, "cohort-test");
    }

    #[test]
    fn confirmation_interval_leaves_half_the_stale_lease_for_delivery() {
        let signer = signing_key(12);
        let state = state(1, &signer, Arc::new(InMemoryCohortAuditSink::default()));

        assert_eq!(
            state.confirmation_interval().unwrap(),
            Duration::from_secs(60),
            "a 120-second stale lease must pull at its halfway point"
        );
    }

    struct RejectingAudit;

    impl CohortAuditSink for RejectingAudit {
        fn append(&self, _event: &CohortAuditEvent) -> Result<(), CohortError> {
            Err(CohortError::EAuditAppendFailed(
                "simulated transparency-log failure".into(),
            ))
        }
    }

    #[test]
    fn audit_failure_prevents_reissue_publication() {
        let signer = signing_key(12);
        let state = state(1, &signer, Arc::new(RejectingAudit));

        assert!(matches!(
            state.apply_reissue(&signed_toml(2, &signer, &signer)),
            Err(CohortError::EAuditAppendFailed(_))
        ));
        assert_eq!(state.version().unwrap(), 1);
    }

    #[test]
    fn digest_audit_failure_publishes_no_reply_capability() {
        let signer = signing_key(13);
        let state = state(1, &signer, Arc::new(RejectingAudit));
        let request = digest_frame(
            "host-b",
            "host-a",
            DigestReadControl::Request {
                request_id: "host-b:0001".into(),
                scope: DIGEST_DAILY_SCOPE.into(),
            },
        );

        assert!(DigestReadPort::note_admitted_request(
            &state,
            &HostId("host-b".into()),
            "host-b:0001",
            &request,
        )
        .is_err());
        assert!(!DigestReadPort::authorize_reply_send(
            &state,
            &HostId("host-b".into()),
            "host-b:0001",
        ));
        assert!(state
            .drain_pending_digest_replies()
            .expect("queue remains readable")
            .is_empty());
    }

    #[test]
    fn guarded_digest_admission_skips_side_effects_for_duplicates_and_capacity_rejections() {
        let signer = signing_key(14);
        let state = state(1, &signer, Arc::new(InMemoryCohortAuditSink::default()));
        let requester = HostId("host-b".into());

        let first_id = "host-b:guarded-0000";
        let first = digest_frame(
            "host-b",
            "host-a",
            DigestReadControl::Request {
                request_id: first_id.into(),
                scope: DIGEST_DAILY_SCOPE.into(),
            },
        );
        let mut guard_calls = 0usize;
        DigestReadPort::note_admitted_request_guarded(
            &state,
            &requester,
            first_id,
            &first,
            &mut || {
                guard_calls += 1;
                Ok(())
            },
        )
        .expect("first request admits");
        DigestReadPort::note_admitted_request_guarded(
            &state,
            &requester,
            first_id,
            &first,
            &mut || {
                guard_calls += 1;
                Ok(())
            },
        )
        .expect("duplicate remains idempotent");
        assert_eq!(
            guard_calls, 1,
            "duplicate request must not repeat irreversible governance"
        );

        for index in 1..MAX_PENDING_DIGEST_READS {
            let request_id = format!("host-b:guarded-{index:04}");
            let frame = digest_frame(
                "host-b",
                "host-a",
                DigestReadControl::Request {
                    request_id: request_id.clone(),
                    scope: DIGEST_DAILY_SCOPE.into(),
                },
            );
            DigestReadPort::note_admitted_request(&state, &requester, &request_id, &frame)
                .expect("fill pending digest capacity");
        }
        let overflow_id = "host-b:guarded-overflow";
        let overflow = digest_frame(
            "host-b",
            "host-a",
            DigestReadControl::Request {
                request_id: overflow_id.into(),
                scope: DIGEST_DAILY_SCOPE.into(),
            },
        );
        let mut overflow_guard_calls = 0usize;
        let error = DigestReadPort::note_admitted_request_guarded(
            &state,
            &requester,
            overflow_id,
            &overflow,
            &mut || {
                overflow_guard_calls += 1;
                Ok(())
            },
        )
        .expect_err("over-capacity request must fail closed");
        assert!(error.contains("too many admitted digest reads"));
        assert_eq!(
            overflow_guard_calls, 0,
            "capacity rejection must occur before irreversible governance"
        );
    }

    #[test]
    fn digest_reply_is_immutable_and_manifest_scoped() {
        let signer = signing_key(14);
        let state = state(1, &signer, Arc::new(InMemoryCohortAuditSink::default()));
        let peer = HostId("host-b".into());
        state
            .note_digest_request_sent(&peer, "host-a:0001", DIGEST_DAILY_SCOPE)
            .unwrap();
        let first = DigestSummary {
            frames: 1,
            halts: 2,
            conflicts: 3,
        };
        let reply = digest_frame(
            "host-b",
            "host-a",
            DigestReadControl::Reply {
                request_id: "host-a:0001".into(),
                summary: first.clone(),
            },
        );
        assert_eq!(
            DigestReadPort::observe_reply(&state, &peer, &reply).unwrap(),
            DigestReplyObservation::Accepted
        );
        assert_eq!(
            DigestReadPort::observe_reply(&state, &peer, &reply).unwrap(),
            DigestReplyObservation::Duplicate
        );
        let conflicting = digest_frame(
            "host-b",
            "host-a",
            DigestReadControl::Reply {
                request_id: "host-a:0001".into(),
                summary: DigestSummary {
                    frames: 99,
                    ..first.clone()
                },
            },
        );
        assert!(DigestReadPort::observe_reply(&state, &peer, &conflicting).is_err());
        assert_eq!(state.digest_summary(&peer, "host-a:0001"), Some(first));

        state
            .note_digest_request_sent(&peer, "host-a:0002", DIGEST_DAILY_SCOPE)
            .unwrap();
        state
            .apply_reissue(&signed_toml(2, &signer, &signer))
            .unwrap();
        let revoked = digest_frame(
            "host-b",
            "host-a",
            DigestReadControl::Reply {
                request_id: "host-a:0002".into(),
                summary: DigestSummary::default(),
            },
        );
        assert_eq!(
            DigestReadPort::observe_reply(&state, &peer, &revoked).unwrap(),
            DigestReplyObservation::Unauthorized
        );
    }

    /// `j1-crosshost-2c` AC3.5 (`deferred-work.md:819`) — **nothing is `Duplicate`
    /// until something is durable.**
    ///
    /// The router hands the intake-sink push in as `before_commit`. When that push
    /// fails, the reply must publish NO dedup state: the grant stays outstanding,
    /// no summary is recorded, no receipt is audited, and a retry is answered
    /// `Accepted` rather than `Duplicate`. Before this fix a retry after a
    /// dropped-receiver NACK was told `Duplicate` — an ACK claiming
    /// `delivered: true` — while the frame was still gone.
    #[test]
    fn a_failed_commit_leaves_the_digest_reply_retryable_never_duplicate() {
        let signer = signing_key(21);
        let audit = Arc::new(InMemoryCohortAuditSink::default());
        let state = state(1, &signer, audit.clone());
        let peer = HostId("host-b".into());
        state
            .note_digest_request_sent(&peer, "host-a:0001", DIGEST_DAILY_SCOPE)
            .unwrap();
        let summary = DigestSummary {
            frames: 7,
            halts: 0,
            conflicts: 0,
        };
        let reply = digest_frame(
            "host-b",
            "host-a",
            DigestReadControl::Reply {
                request_id: "host-a:0001".into(),
                summary: summary.clone(),
            },
        );

        let audited_before = audit.events().len();
        let error = DigestReadPort::observe_reply_guarded(&state, &peer, &reply, &mut || {
            Err("intake sink full or receiver dropped — digest reply NOT delivered".to_string())
        })
        .expect_err("a failed commit must surface, not be swallowed");
        assert!(error.contains("NOT delivered"), "{error}");

        // NOTHING was published.
        assert_eq!(
            state.digest_summary(&peer, "host-a:0001"),
            None,
            "a reply that was never handed over must not be recorded"
        );
        assert_eq!(
            audit.events().len(),
            audited_before,
            "no DigestReplyReceived receipt may be audited for an undelivered reply"
        );

        // And the retry is RETRYABLE — this is the assertion `:819` was about.
        let observation = DigestReadPort::observe_reply(&state, &peer, &reply)
            .expect("the grant must still be outstanding");
        assert_eq!(
            observation,
            DigestReplyObservation::Accepted,
            "a retry after a failed commit must be accepted, never reported Duplicate"
        );
        assert_eq!(state.digest_summary(&peer, "host-a:0001"), Some(summary));

        // Only NOW is a second attempt a duplicate.
        assert_eq!(
            DigestReadPort::observe_reply(&state, &peer, &reply).unwrap(),
            DigestReplyObservation::Duplicate
        );
    }
    #[test]
    fn signed_v3_cache_refuses_higher_version_v2_reissue() {
        let signer = signing_key(15);
        let pins = PinnedAuthorityKeys::from_keys(vec![signer.verifying_key()]).unwrap();
        let state = CohortManifestState::load(
            HostId("host-a".into()),
            &signed_tenant_toml(COHORT_SCHEMA_V3, 1, &signer),
            pins,
            Arc::new(InMemoryCohortAuditSink::default()),
        )
        .unwrap();
        let error = state
            .apply_reissue(&signed_tenant_toml(COHORT_SCHEMA_V2, 2, &signer))
            .unwrap_err();
        assert!(matches!(
            error,
            CohortError::ECohortManifestFork {
                reason: CohortManifestForkReason::SchemaDowngrade,
                seen_version: 1,
                rejected_version: 2,
            }
        ));
    }

    /// Story 13.6a (AC1) — once V4 is cached, a V4→V3 reissue is refused on the
    /// SHIPPED `SchemaDowngrade` path and locally audited, even though its
    /// manifest revision is higher. Without this an operator could revoke every
    /// host→team edge in the cohort by re-issuing the predecessor schema.
    #[test]
    fn signed_v4_cache_refuses_higher_version_v3_reissue_and_audits_it() {
        let signer = signing_key(16);
        let pins = PinnedAuthorityKeys::from_keys(vec![signer.verifying_key()]).unwrap();
        let audit = Arc::new(InMemoryCohortAuditSink::default());
        let state = CohortManifestState::load(
            HostId("host-a".into()),
            &signed_tenant_toml(COHORT_SCHEMA_V4, 1, &signer),
            pins,
            audit.clone(),
        )
        .unwrap();
        // Positive control first: a higher revision AT V4 is accepted, so the
        // refusal below is about the schema floor and not about revisions.
        state
            .apply_reissue(&signed_tenant_toml(COHORT_SCHEMA_V4, 2, &signer))
            .expect("a same-schema higher revision must still apply");

        let error = state
            .apply_reissue(&signed_tenant_toml(COHORT_SCHEMA_V3, 3, &signer))
            .unwrap_err();
        assert!(matches!(
            error,
            CohortError::ECohortManifestFork {
                reason: CohortManifestForkReason::SchemaDowngrade,
                seen_version: 2,
                rejected_version: 3,
            }
        ));
        assert!(audit.events().iter().any(|event| matches!(
            event,
            CohortAuditEvent::ReissueRejected { reason, .. } if reason.contains("schema_downgrade")
        )));
        // The edge survives the refused downgrade.
        assert_eq!(state.team_of_local_host().as_deref(), Some("team-a"));
    }
}
