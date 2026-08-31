#![forbid(unsafe_code)]

//! Story 14-2a — the production peer-certificate rotation trigger.
//!
//! # What this closes
//!
//! `crates/maos-bin/src/main.rs:9844-9853` (Story 13.6a) wrote this story's bug
//! report a year of commits ago, verbatim:
//!
//! > *"Two independent config surfaces name certificates — `tcp.peer_pins` (the
//! > handshake verifier's oracle) and `file.peers` (the frame-level TOFU
//! > records) — and neither is derived from the manifest. … **a signed reissue
//! > that rotates a member's fingerprint would not revoke the stale
//! > certificate.** Disagreement is a boot error, never a warning."*
//!
//! `reconcile_transport_identity_with_manifest` enforces that agreement exactly
//! ONCE, at boot, before the transport binds. [`CohortManifestState::apply_reissue`]
//! can move the signed truth underneath it at any moment afterwards, and until
//! this module nothing re-checked. This is the RUNTIME half of that boot check;
//! the boot check remains its precondition, never its competitor.
//!
//! # The authority is the signed manifest, and the operator's act is signing it
//!
//! The trigger is not a new control channel. `cohort:manifest-reissue` is
//! already live, signed, version-monotonic, cohort-id-pinned, fail-closed and
//! audited (`state.rs:260`), and `CohortManifest::peer_configs_for`
//! (`manifest.rs:568`) already projects `members[].fingerprint` into
//! `Vec<A2APeerConfig>` with the §7.2 pin *straight from the manifest*. Story
//! 12.1 built that projection and nothing ever consumed it. This module is its
//! first consumer.
//!
//! ⚠ **THE COHORT AUTHORITY KEY NOW CONTROLS LIVE TLS TRUST.** Before this
//! module a signed reissue changed *who is in the cohort*; now the same frame
//! changes *which certificates every node accepts on a live handshake,
//! mid-session, with no restart*. That is not "no second trust path" — it is the
//! FIRST trust path getting wider, and the blast radius of a compromised cohort
//! authority key grows from membership to live transport identity. 14-2's
//! collision guard (`tofu.rs:221-233`) stops an attacker STEALING an existing
//! peer's fingerprint; it does NOT stop one being ADDED. The escalation is
//! recorded as a threat-model delta in `RELEASE-HOLDS.md`, every transition is
//! journaled under one stable greppable intent
//! ([`CERT_ROTATION_INTENT`](crate::audit::CERT_ROTATION_INTENT)), and there is
//! no bypass, no `force` flag and no unsigned hatch: the ONLY way in is the same
//! signed path.
//!
//! # Two planes, and why ORDER replaces the lock nobody can take
//!
//! A peer's fingerprint is declared twice at runtime:
//!
//! * **plane A** — `A2ARouterCore.peers[p].cert_fingerprint` (`router.rs:138`),
//!   the *declaration* sites 5/6 verify on every frame;
//! * **plane B** — `InMemoryTofuPinStore` `pins` + `rotation_next`
//!   (`tofu.rs:156`, `:168`), the *pin* they verify it against.
//!
//! Sites 5 and 6 — `prepare_outbound` (`router.rs:991-994` → `PinMismatch`) and
//! `handle_intake_inner` (`router.rs:1309-1315` → NACK
//! `CODE_PIN_MISMATCH_NOT_PINNED`) — read A and B with **no shared lock and no
//! snapshot**, and no primitive in the tree can make them do otherwise. So the
//! reload is made observably atomic by ORDER instead:
//!
//! **The invariant, asserted by name:** for every peer `p` at every observable
//! instant, `peers[p].cert_fingerprint ∈ {pins[p].fingerprint, rotation_next[p]}`.
//!
//! 1. plane B FIRST: [`InMemoryTofuPinStore::open_rotation_window`] makes the
//!    accepted set the SUPERSET `{old, next}` for the whole transition;
//! 2. plane A SECOND: [`A2ARouterCore::set_peer_cert_fingerprint`] writes a
//!    value the accepted set already contains;
//! 3. rollback runs the same order REVERSED — plane A restored to `old` BEFORE
//!    [`InMemoryTofuPinStore::abort_rotation_window`] narrows the set again.
//!
//! At no point is plane A observable against a set that excludes it, so a torn
//! reload — which would refuse **every frame in both directions** for its
//! duration — cannot be observed. The transaction is PER PEER, which is the
//! granularity the invariant has: one peer's refusal never un-rotates another
//! peer that already succeeded.
//!
//! ## The ten torn-state hazards, each closed or declared
//!
//! * **(a) sites 5/6 read A and B with no shared lock** — CLOSED by the
//!   superset ordering above; no lock is needed because the accepted set is
//!   never narrower than plane A's value.
//! * **(b) `window_lock` serializes writers only, no reader takes it** —
//!   CLOSED for the same reason: readers need no lock when the set only widens
//!   and then narrows to a value plane A already holds.
//! * **(c) plane A has no lock at all** — CLOSED: plane A is written with a
//!   single `DashMap` entry write of one already-accepted value. There is no
//!   multi-field plane-A transaction, so there is nothing to serialize.
//! * **(d) a cross-peer fingerprint SWAP is refused mid-way** — DECLARED and
//!   fail-closed: `colliding_peer` (`tofu.rs:221-233`) refuses a `next` another
//!   peer still holds, so a reissue that swaps two members' fingerprints is
//!   REFUSED for both, both keep their current certs, and both refusals are
//!   journaled. Clearing first to "make room" is exactly the impersonation
//!   window 14-2's guard exists to close, so the operator rotates through a
//!   fresh fingerprint in two reissues instead. A swap is indistinguishable
//!   from an impersonation attempt at the store level, and that is the correct
//!   reading.
//! * **(e) a pin-less peer fails site 2 and is closed before intake** — CLOSED
//!   by construction: an unpinned peer has NO plane-B state to overlap, so this
//!   module moves plane A ONLY (there is no window to open or close) and first
//!   contact pins the manifest-declared value. It was already unreachable until
//!   first contact; the reload does not change that, it only makes the manifest
//!   authoritative when contact happens.
//! * **(f) `pin_first_contact` is not idempotent** — CLOSED: this module never
//!   calls it. Re-running `build_pin_store` against a live store is not a reload
//!   strategy and is not used.
//! * **(g) `verified_peer` is resolved once per connection and reused** —
//!   DECLARED: a long-lived connection keeps its pre-reload binding until it is
//!   re-established. The overlap window is what makes that safe: both
//!   generations are accepted for `T_grace`, and after promotion the retired
//!   leaf is refused on the NEXT handshake. A rotation therefore takes effect
//!   per connection, not per frame.
//! * **(h) `peer_cfg` is snapshotted at `router.rs:829` and carried through the
//!   dial** — DECLARED, and harmless for the same reason: a snapshot taken
//!   before the reload holds `old`, which the open window still pins.
//! * **(i) `lock_window` poison recovery clears ALL of `rotation_next`** —
//!   DECLARED and fail-closed: every open window disappears, so the mesh
//!   narrows to the serving pins. Plane A may then hold a promoted-but-unpinned
//!   value; the closer's terminal row records `no open window at expiry` rather
//!   than silently reporting success, and [`PeerCertRotation::open_windows`]
//!   stops reporting the window because it INTERSECTS its ledger with
//!   `rotation_next` — the read surface can never over-report live trust.
//! * **(j) a dropped peer makes site 6 return `CODE_INTERNAL`** — DECLARED:
//!   this module never removes a peer from plane A. Membership REMOVAL is not
//!   rotation and is out of scope (the roster gate already refuses a
//!   non-member); the projection only moves fingerprints of peers plane A
//!   already declares, and a peer the manifest adds but plane A never declared
//!   is refused with a named row rather than inserted.
//!
//! ## Lock order, in one place
//!
//! `CohortManifestState.cached` → `InMemoryTofuPinStore.window_lock` → a `pins`
//! entry → a plane-A `peers` entry → [`PeerCertRotation::ledger`]. The first two
//! are the only real locks and the tree already documents the second
//! (`tofu.rs:171-172`). `TcpA2ATransport::swap_lock` is NOT in this order
//! because this story never takes it: the local leaf is `14-2b`'s
//! (`swap_serving_cert` still has no production caller after this story).
//! ⚠ Implementations of [`RotationGraceTimer`] MUST NOT call back into
//! `CohortManifestState`: the manifest lock is held across [`PeerCertRotation::reload`].

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use maos_a2a_core::{
    A2APeerConfig, A2ARouterCore, EPinMismatch, InMemoryTofuPinStore, PeerCertFingerprint, PeerId,
};
use maos_spirit_abi::identity::HostId;

use crate::audit::{CohortAuditEvent, CohortAuditSink};
use crate::error::CohortError;

/// §7.2.1.a's own cold-deployment floor, in milliseconds.
///
/// The steady-state branch — `T_grace = max(2 × p99_handshake_rtt, 5 s)` over
/// the trailing 30-day p99 of `iac_handshake_duration_us` — is NOT
/// constructible: that metric has **no producer anywhere in the workspace**
/// (its sole in-code occurrence is a doc comment at
/// `crates/maos-a2a-core/src/chaos/rotation.rs:8`). The cold branch is: *"If
/// <30 days of data exist … use the maximum observed handshake duration over
/// available history, floored at 500 ms."* Zero days of history exist, so the
/// honest conservative substitute for "maximum observed" is the spec's own
/// floor, and `compute_t_grace(500, 0)` resolves to the §7.2.1.a hard floor of
/// **5 s** — the minimum the spec permits on either branch, i.e. the narrowest
/// widened-trust window that is still spec-faithful.
///
/// When the metric ships, the real p99 goes in at this one call site and
/// nothing else changes. This is deliberately **not** an operator knob: §7.2.1.a
/// defines `T_grace` as DERIVED, not chosen, and the sprint row's phrase
/// "operator-defined grace" diverges from the ratified spec (settled at AC3.1).
/// A knob would also need an env-contract registry row (`check-env-contract` is
/// blind outside `maos-bin/src`) or a `TcpA2AConfig` schema delta, and neither
/// buys anything the derivation does not already give.
pub const COLD_DEPLOYMENT_HANDSHAKE_MS: u64 = 500;

/// Days of trailing history available for the §7.2.1.a p99. Zero, measurably:
/// no producer of `iac_handshake_duration_us` exists.
pub const COLD_DEPLOYMENT_DAYS_OF_HISTORY: u32 = 0;

/// `T_grace` on §7.2.1.a's cold-deployment branch — 5 s at HEAD.
///
/// Derived through the shipped [`maos_a2a_core::compute_t_grace`], never
/// restated as a literal, so the floor and this call site cannot drift.
pub fn cold_deployment_t_grace() -> Duration {
    maos_a2a_core::compute_t_grace(
        COLD_DEPLOYMENT_HANDSHAKE_MS,
        COLD_DEPLOYMENT_DAYS_OF_HISTORY,
    )
}

/// The seam that closes a rotation window when `T_grace` has ELAPSED — a real
/// deadline, never "on the next frame".
///
/// `maos-cohort` has no `tokio` dependency and must not grow one, so the timer
/// is injected: production installs a `tokio::spawn` + `sleep` implementation
/// from `maos-bin` (modelled on `PostSwapMonitor::spawn`, which is the shipped
/// deadline-then-terminal-action shape), and tests install a manual timer that
/// fires on demand. Both drive the same terminal action —
/// [`InMemoryTofuPinStore::close_rotation_window`] — so the test and the
/// production path differ in WHEN, never in WHAT.
///
/// ⚠ The store gets no clock and `TofuPin` gets no TTL field: `pinned_at_ns` is
/// a process-global `AtomicU64::fetch_add` counter documented *"NOT a TTL"*
/// (`tofu.rs:609-617`), and 14-2's AC2.1.c decision that the close is an
/// explicit state transition is not reopened here. The deadline lives in the
/// closer, which is exactly what makes it observable and cancellable.
pub trait RotationGraceTimer: Send + Sync {
    /// Run `on_expiry` exactly once, after `grace` has elapsed, off the
    /// caller's thread. The implementation MUST NOT run it inline: the caller
    /// holds `CohortManifestState.cached`.
    fn schedule(&self, grace: Duration, on_expiry: Box<dyn FnOnce() + Send>);
}

/// [`RotationWindow::state`] — a live one-generation overlap.
pub const OPEN: &str = "open";
/// [`RotationWindow::state`] — the peer holds NO pin yet, so plane A's
/// declaration is authoritative and first contact will pin it. Reported so an
/// operator can see it, and deliberately NOT called a divergence: a manifest
/// member with no configured `tcp.peer_pins` row is a boot-passing deployment
/// (`reconcile_transport_identity_with_manifest` validates pins that EXIST and
/// never requires one per member), and the write path reports the same peer as a
/// SUCCESS in `RotationOutcome::declared_only`. Flagging it would be a false
/// alarm an operator would act on.
pub const AWAITING_FIRST_CONTACT: &str = "awaiting-first-contact";
/// [`RotationWindow::state`] — the committed manifest names a fingerprint the
/// live planes do NOT implement, because this peer's rotation was refused.
/// Derived on every read, so it can never go stale and it clears itself the
/// moment the planes converge.
pub const DIVERGED: &str = "diverged";

/// One open one-generation overlap window, as an operator can read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RotationWindow {
    /// The cohort member whose certificate is rotating.
    pub peer: String,
    /// The fingerprint that is still serving and will be retired at promotion.
    pub retiring: String,
    /// The incoming fingerprint the signed manifest now declares.
    pub next: String,
    /// The fingerprint plane A (`A2ARouterCore.peers[p].cert_fingerprint`)
    /// ACTUALLY declares right now — the live router's own value, not the
    /// manifest's intent.
    ///
    /// It is reported beside `next` on purpose: a rotation wired to a DETACHED
    /// router core would still open a window and still report it, and only this
    /// field makes that wrong-core wiring visible. During the window it equals
    /// `next`; before plane A moves it equals `retiring`.
    pub declared: String,
    /// The manifest version that opened this window.
    pub manifest_version: u64,
    /// Cohort-clock seconds at which the window opened.
    pub opened_at_secs: u64,
    /// [`OPEN`], [`DIVERGED`] or [`AWAITING_FIRST_CONTACT`].
    pub state: &'static str,
    /// A per-process monotonic INSTANCE id, not a generation.
    ///
    /// ⚠ Load-bearing: a grace closer is fire-and-forget (the injected timer
    /// returns no handle, so an armed closer can never be cancelled), and
    /// binding it to the fingerprint VALUE alone is not enough — a window
    /// aborted and later RE-OPENED on the same generation would be promoted by
    /// the stale timer, cutting the new window's §7.2.1.a grace to nothing, and
    /// the stale closer would delete the live window's ledger row by peer key,
    /// hiding a genuinely widened trust set from the read surface. Every closer
    /// therefore checks that the ledger row it is about to end is still ITS OWN
    /// instance.
    pub instance: u64,
}

/// A per-peer refusal. The reload continues for the other peers; the refusal is
/// journaled, never swallowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RotationRefusal {
    pub peer: String,
    pub reason: String,
}

/// What one signed reissue actually did to the live trust set.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RotationOutcome {
    /// Peers whose window is now open (plane B widened, plane A moved).
    pub opened: Vec<RotationWindow>,
    /// Peers the manifest declares with the fingerprint already in force — no
    /// window, no audit row, no action at all (AC2.1).
    pub unchanged: Vec<String>,
    /// Peers with no pin yet: plane A moved, no window (hazard (e)).
    pub declared_only: Vec<String>,
    /// Peers this reload refused, with the reason an operator will read.
    pub refused: Vec<RotationRefusal>,
    /// Peers whose IN-FLIGHT window this newer manifest superseded: it
    /// re-declared the serving generation, so the window was aborted before its
    /// grace rather than promoted into a generation the manifest no longer
    /// names.
    pub superseded: Vec<String>,
}

impl RotationOutcome {
    /// True when the reissue moved no trust at all — the common case, and the
    /// one that must cost nothing.
    pub fn is_noop(&self) -> bool {
        self.opened.is_empty()
            && self.declared_only.is_empty()
            && self.refused.is_empty()
            && self.superseded.is_empty()
    }
}

enum AppliedChange {
    WindowOpened {
        peer: PeerId,
        previous: PeerCertFingerprint,
    },
    DeclarationMoved {
        peer: PeerId,
        previous: PeerCertFingerprint,
    },
    WindowSuperseded {
        peer: PeerId,
        previous_declaration: PeerCertFingerprint,
        prior_window: RotationWindow,
        prior_next: PeerCertFingerprint,
    },
}

/// The two-plane reload and its grace closer.
///
/// Holds the SAME `Arc`s the accept loop, both verifiers and the router core
/// hold (`TcpA2ATransport::pins()` / `::core()` return clones of the live
/// handles, `transport.rs:492`/`:503`), so a reload here is a reload of the
/// running mesh and not of a copy.
pub struct PeerCertRotation {
    pins: Arc<InMemoryTofuPinStore>,
    core: Arc<A2ARouterCore>,
    audit: Arc<dyn CohortAuditSink>,
    timer: Arc<dyn RotationGraceTimer>,
    grace: Duration,
    /// Serializes every plane/ledger transition with grace closers. Readers do
    /// not take the store's writer lock, so ordering still preserves their
    /// invariant; this lock prevents two writers from validating one window
    /// instance and then mutating different instances.
    transition_lock: Arc<Mutex<()>>,
    /// Observability ledger for [`Self::open_windows`]. NOT an authority: the
    /// read surface intersects it with plane B's `rotation_next`, so a stale row
    /// can never make the API report trust the store does not hold.
    ledger: Arc<Mutex<BTreeMap<String, RotationWindow>>>,
    /// Monotonic source of [`RotationWindow::instance`] ids.
    instances: Arc<std::sync::atomic::AtomicU64>,
}

impl PeerCertRotation {
    pub fn new(
        pins: Arc<InMemoryTofuPinStore>,
        core: Arc<A2ARouterCore>,
        audit: Arc<dyn CohortAuditSink>,
        timer: Arc<dyn RotationGraceTimer>,
        grace: Duration,
    ) -> Self {
        Self {
            pins,
            core,
            audit,
            timer,
            grace,
            transition_lock: Arc::new(Mutex::new(())),
            ledger: Arc::new(Mutex::new(BTreeMap::new())),
            instances: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    /// The rotation status an operator can read: every window this process still
    /// holds open, INTERSECTED with plane B's `rotation_next`, PLUS every peer
    /// whose committed manifest fingerprint the live planes do not implement.
    ///
    /// Two things make this trustworthy rather than decorative:
    ///
    /// * **The intersection.** A ledger row whose window was promoted, aborted,
    ///   or cleared by poison recovery is dropped, so the surface cannot
    ///   over-report a widened trust set — the one thing a rotation
    ///   observability surface must never do.
    /// * **The divergence set, computed ON READ.** A per-peer rotation can be
    ///   REFUSED (a collision, an invalidated pin, a member plane A never
    ///   declared) while the signed manifest still commits, because refusing
    ///   the whole manifest would let one restarted peer block every future
    ///   reissue. That leaves a peer whose committed manifest fingerprint the
    ///   planes do not implement, and `main.rs:9917` is explicit that such a
    ///   disagreement "is a boot error, never a warning". At runtime it cannot
    ///   be a boot error, so it MUST at least be visible: derived on every read
    ///   from the manifest projection against the live planes, it can never go
    ///   stale and it clears itself the moment the planes converge.
    pub fn status(&self, manifest_peers: &[A2APeerConfig]) -> Vec<RotationWindow> {
        let _transition = match self.transition_lock.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let mut rows: Vec<RotationWindow> = self
            .ledger_write(|ledger| ledger.values().cloned().collect::<Vec<_>>())
            .into_iter()
            .filter(|window| {
                self.pins
                    .rotation_next(&PeerId::new(&window.peer))
                    .is_some()
            })
            .collect();
        for peer_config in manifest_peers {
            let peer = &peer_config.peer_id;
            if rows.iter().any(|row| row.peer == peer.as_str()) {
                continue;
            }
            let state = match self
                .pins
                .verify_pinned_sync(peer, &peer_config.cert_fingerprint)
            {
                Ok(()) => continue,
                Err(EPinMismatch::NotPinned(_)) => AWAITING_FIRST_CONTACT,
                Err(_) => DIVERGED,
            };
            let serving = self
                .pins
                .get_pin_sync(peer)
                .map(|pin| pin.fingerprint.wire())
                .unwrap_or_else(|| "none".to_string());
            let declared = self
                .core
                .lookup_peer(&HostId(peer.as_str().to_string()))
                .map(|config| config.cert_fingerprint.wire())
                .unwrap_or_else(|_| "undeclared".to_string());
            rows.push(RotationWindow {
                peer: peer.as_str().to_string(),
                retiring: serving,
                next: peer_config.cert_fingerprint.wire(),
                declared,
                manifest_version: 0,
                opened_at_secs: 0,
                state,
                instance: 0,
            });
        }
        rows.sort_by(|left, right| left.peer.cmp(&right.peer));
        rows
    }

    /// Reload the live peer trust set from a signed manifest projection.
    ///
    /// `peers` MUST come from [`crate::CohortManifest::peer_configs_for`], which
    /// validates every fingerprint through `PeerCertFingerprint::parse` —
    /// `sha256`, length 64, ASCII-hex, LOWERCASED (`manifest.rs:588-594`). That
    /// is load-bearing (AC2.4): `PeerCertFingerprint`'s derived `Deserialize`
    /// has no validator, so `hex = "ZZ"`, `hex = ""` and a
    /// correct-but-UPPERCASE fingerprint all deserialize cleanly from operator
    /// TOML and then fail as an opaque per-frame `PinMismatch`. Equality is the
    /// derived case-sensitive `PartialEq`, so a fingerprint that did NOT go
    /// through `parse` can silently never match. This module therefore accepts
    /// fingerprints from the signed manifest projection and from nowhere else.
    ///
    /// Returns `Err` only when evidence could not be written; every plane this
    /// call moved is then rolled back, so a caller that refuses to commit the
    /// manifest is left with a coherent mesh rather than a half-applied one.
    ///
    /// ⚠ **THAT `Err` PATH IS UNREACHABLE THROUGH THE PRODUCTION SINK, AND THE
    /// DESIGN DOES NOT DEPEND ON IT.** `CohortTransparencyLogSink::append`
    /// always returns `Ok(())`: the underlying
    /// `TransparencyLogAdapter::insert_frame_event` PANICS on a write failure by
    /// architecture §7.3 I2 ("if the log write fails, the kernel panics rather
    /// than silently dropping the frame"). A fail-closed contract built on a
    /// sink that cannot report failure would be a contract in name only, so the
    /// safety here does NOT come from the rollback — it comes from ARMING THE
    /// GRACE DEADLINE BEFORE THE AUDIT WRITE (see
    /// [`Self::rotate_pinned_peer`]): an unwind at any later point leaves a
    /// window that still narrows itself within `T_grace`, with plane A
    /// untouched, rather than a trust set widened for the life of the process.
    /// The rollback remains, exercised by an injected sink, because a
    /// transient-failure sink is a legitimate deployment and because the
    /// property it enforces — planes and manifest agree or neither moves — is
    /// worth asserting.
    pub fn reload(
        &self,
        peers: &[A2APeerConfig],
        manifest_version: u64,
        now_secs: u64,
        commit_event: Option<&CohortAuditEvent>,
    ) -> Result<RotationOutcome, CohortError> {
        let _transition = match self.transition_lock.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let mut outcome = RotationOutcome::default();
        // Every plane this call moved, newest first, for the rollback path.
        let mut applied = Vec::new();

        for peer_config in peers {
            let peer = &peer_config.peer_id;
            let declared = &peer_config.cert_fingerprint;
            let superseded = match self.pins.rotation_next(peer) {
                Some(open) if &open != declared => {
                    match self.supersede_open_window(
                        peer,
                        &open,
                        manifest_version,
                        &mut outcome,
                        &mut applied,
                    ) {
                        Ok(superseded) => superseded,
                        Err(error) => {
                            self.rollback(&applied, manifest_version);
                            return Err(error);
                        }
                    }
                }
                _ => false,
            };
            let result = match self.pins.verify_pinned_sync(peer, declared) {
                Ok(()) => {
                    if !superseded {
                        outcome.unchanged.push(peer.as_str().to_string());
                    }
                    Ok(())
                }
                Err(EPinMismatch::Mismatch { pinned, .. }) => self.rotate_pinned_peer(
                    peer,
                    declared,
                    &pinned,
                    manifest_version,
                    now_secs,
                    &mut outcome,
                    &mut applied,
                ),
                // Hazard (e): no pin exists yet. There is no generation to
                // overlap, so plane A alone becomes authoritative and first
                // contact pins the manifest's value.
                Err(EPinMismatch::NotPinned(_)) => self.move_declaration_only(
                    peer,
                    declared,
                    manifest_version,
                    now_secs,
                    &mut outcome,
                    &mut applied,
                ),
                // An invalidated pin is already refusing every frame and cannot
                // hold a window (`open_rotation_window` refuses it by name). A
                // rotation cannot repair it; the re-pin consent path can. Named,
                // journaled, and never silently skipped.
                Err(other) => self.refuse(&mut outcome, peer, &other.to_string(), manifest_version),
            };
            if let Err(error) = result {
                self.rollback(&applied, manifest_version);
                return Err(error);
            }
        }
        if let Some(event) = commit_event {
            if let Err(error) = self.audit.append(event) {
                self.rollback(&applied, manifest_version);
                return Err(error);
            }
        }
        Ok(outcome)
    }

    /// End an in-flight generation before evaluating the newer declaration.
    /// The transition lock held by `reload` makes the ledger removal, plane
    /// realignment, and pin-window abort indivisible from every grace closer.
    /// The full prior state is retained so a later peer failure can restore the
    /// exact window and let its already-armed timer finish it.
    fn supersede_open_window(
        &self,
        peer: &PeerId,
        open: &PeerCertFingerprint,
        manifest_version: u64,
        outcome: &mut RotationOutcome,
        applied: &mut Vec<AppliedChange>,
    ) -> Result<bool, CohortError> {
        let prior_window = self
            .ledger_write(|ledger| ledger.get(peer.as_str()).cloned())
            .ok_or(CohortError::EStatePoisoned)?;
        let serving = self
            .pins
            .get_pin_sync(peer)
            .map(|pin| pin.fingerprint)
            .ok_or(CohortError::EStatePoisoned)?;
        let previous_declaration = self
            .core
            .set_peer_cert_fingerprint(&HostId(peer.as_str().to_string()), serving)
            .ok_or(CohortError::EStatePoisoned)?;
        let Some(aborted) = self.pins.abort_rotation_window(peer) else {
            self.core.set_peer_cert_fingerprint(
                &HostId(peer.as_str().to_string()),
                previous_declaration,
            );
            return Err(CohortError::EStatePoisoned);
        };
        if aborted != *open {
            let _ = self.pins.open_rotation_window(peer, &aborted);
            self.core.set_peer_cert_fingerprint(
                &HostId(peer.as_str().to_string()),
                previous_declaration,
            );
            return Err(CohortError::EStatePoisoned);
        }
        self.ledger_write(|ledger| {
            ledger.remove(peer.as_str());
        });
        applied.push(AppliedChange::WindowSuperseded {
            peer: peer.clone(),
            previous_declaration,
            prior_window,
            prior_next: open.clone(),
        });
        self.audit.append(&CohortAuditEvent::CertRotationRefused {
            peer: peer.as_str().to_string(),
            reason: format!(
                "superseded: manifest v{manifest_version} replaced the open window on {}",
                open.wire()
            ),
            version: manifest_version,
        })?;
        outcome.superseded.push(peer.as_str().to_string());
        Ok(true)
    }

    /// Plane B, then plane A, for one peer whose signed fingerprint moved.
    ///
    /// ⚠ **THE ORDER OF THE FOUR STEPS IS THE WHOLE SAFETY ARGUMENT.**
    ///
    /// 1. publish the ledger row — so an operator GET can never observe a
    ///    widened trust set the read surface does not report (the intersection
    ///    with `rotation_next` keeps it from over-reporting before step 2);
    /// 2. open plane B's window — the accepted set becomes `{retiring,
    ///    declared}`, a SUPERSET, which no reader can distinguish from the
    ///    pre-reload state for a peer still serving `retiring`;
    /// 3. **ARM THE GRACE DEADLINE — before the audit write and before plane
    ///    A.** The production audit sink cannot return an error; it PANICS on a
    ///    log-write failure (§7.3 I2). If the deadline were armed after the
    ///    loop, such an unwind would leave `{retiring, declared}` accepted for
    ///    the life of the process with no closer and a poisoned manifest lock —
    ///    a permanently widened trust set from a logging fault. Armed here, the
    ///    worst case is a window that narrows itself in `T_grace` with plane A
    ///    never moved, i.e. a coherent mesh throughout.
    /// 4. journal, then move plane A into a set that already contains its new
    ///    value.
    ///
    /// Step 3 is also the ORDERING CONTROL: an injected
    /// [`RotationGraceTimer`] observes the world exactly once per rotation, at
    /// the only instant where the contract is `rotation_next[p] == declared`
    /// AND `peers[p].cert_fingerprint == retiring`. A test timer asserts that,
    /// which is what turns "plane B first" from a comment into something a
    /// mutation reds deterministically instead of a nanosecond race.
    #[allow(clippy::too_many_arguments)]
    fn rotate_pinned_peer(
        &self,
        peer: &PeerId,
        declared: &PeerCertFingerprint,
        retiring: &str,
        manifest_version: u64,
        now_secs: u64,
        outcome: &mut RotationOutcome,
        applied: &mut Vec<AppliedChange>,
    ) -> Result<(), CohortError> {
        let mut window = RotationWindow {
            peer: peer.as_str().to_string(),
            retiring: retiring.to_string(),
            next: declared.wire(),
            declared: retiring.to_string(),
            manifest_version,
            opened_at_secs: now_secs,
            state: OPEN,
            instance: self
                .instances
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        };
        self.ledger_write(|ledger| {
            ledger.insert(window.peer.clone(), window.clone());
        });
        if let Err(refusal) = self.pins.open_rotation_window(peer, declared) {
            self.ledger_write(|ledger| {
                ledger.remove(peer.as_str());
            });
            return self.refuse(outcome, peer, &refusal.to_string(), manifest_version);
        }
        // `retiring` was read BEFORE `window_lock` was taken, so a grace closer
        // promoting concurrently could have retired it one generation earlier.
        // Re-read the serving pin now that the window is open: the reported
        // value is then the one an operator can still observe.
        if let Some(serving) = self.pins.get_pin_sync(peer) {
            window.retiring = serving.fingerprint.wire();
            window.declared = window.retiring.clone();
        }
        self.ledger_write(|ledger| {
            ledger.insert(window.peer.clone(), window.clone());
        });
        self.schedule_close(&window, manifest_version);
        if let Err(error) = self
            .audit
            .append(&CohortAuditEvent::CertRotationWindowOpened {
                peer: window.peer.clone(),
                retiring: window.retiring.clone(),
                next: window.next.clone(),
                version: manifest_version,
            })
        {
            self.pins.abort_rotation_window(peer);
            self.ledger_write(|ledger| {
                ledger.remove(peer.as_str());
            });
            return Err(error);
        }
        let Some(previous) = self
            .core
            .set_peer_cert_fingerprint(&HostId(peer.as_str().to_string()), declared.clone())
        else {
            // Hazard (j): plane A never declared this peer. Narrow the trust set
            // back — an open window with no declaration is a widened trust set
            // nobody asked for.
            self.pins.abort_rotation_window(peer);
            self.ledger_write(|ledger| {
                ledger.remove(peer.as_str());
            });
            return self.refuse(
                outcome,
                peer,
                "peer is not declared in the live transport peer set; a member the manifest adds \
                 is reachable only after a restart (membership addition is not rotation)",
                manifest_version,
            );
        };
        // The published row now names BOTH planes: `next` is what plane B
        // accepts, `declared` is what plane A actually declares. A rotation
        // wired to a DETACHED router core reports a window whose `declared`
        // never moves, which is the wrong-core mutation made visible.
        window.declared = declared.wire();
        self.ledger_write(|ledger| {
            ledger.insert(window.peer.clone(), window.clone());
        });
        applied.push(AppliedChange::WindowOpened {
            peer: peer.clone(),
            previous,
        });
        outcome.opened.push(window);
        Ok(())
    }

    /// Plane A alone, for a peer plane B has never pinned.
    fn move_declaration_only(
        &self,
        peer: &PeerId,
        declared: &PeerCertFingerprint,
        manifest_version: u64,
        now_secs: u64,
        outcome: &mut RotationOutcome,
        applied: &mut Vec<AppliedChange>,
    ) -> Result<(), CohortError> {
        let Some(previous) = self
            .core
            .set_peer_cert_fingerprint(&HostId(peer.as_str().to_string()), declared.clone())
        else {
            return self.refuse(
                outcome,
                peer,
                "peer is not declared in the live transport peer set; a member the manifest adds \
                 is reachable only after a restart (membership addition is not rotation)",
                manifest_version,
            );
        };
        // ⚠ FIRST-CONTACT RACE, closed by RE-CHECKING rather than by a lock this
        // crate cannot take. Between the `NotPinned` read and this write, a
        // concurrent handshake can run `pin_first_contact` and pin the PREVIOUS
        // plane-A declaration. Plane A would then hold `declared` with plane B
        // pinned to `previous` and NO window — the torn state, refusing every
        // frame in both directions. Re-reading plane B after the write detects
        // exactly that interleaving (a pin cannot un-appear), and the ordinary
        // rotation path repairs it by opening the overlap window the race
        // skipped.
        //
        // ⚠ The transient between the write and the re-check is REAL and is
        // stated rather than implied away: for the microseconds in between,
        // plane A declares `declared` while plane B may already pin `previous`.
        // It is fail-closed twice over — an unpinned peer was already
        // unreachable until first contact, and the interleaving requires that
        // first contact to land inside that window — and the re-check below
        // converts it into an ordinary overlap window instead of leaving it.
        if let Err(EPinMismatch::Mismatch { pinned, .. }) =
            self.pins.verify_pinned_sync(peer, declared)
        {
            self.core
                .set_peer_cert_fingerprint(&HostId(peer.as_str().to_string()), previous);
            return self.rotate_pinned_peer(
                peer,
                declared,
                &pinned,
                manifest_version,
                now_secs,
                outcome,
                applied,
            );
        }
        if let Err(error) = self
            .audit
            .append(&CohortAuditEvent::CertRotationDeclarationMoved {
                peer: peer.as_str().to_string(),
                previous: previous.wire(),
                declared: declared.wire(),
                version: manifest_version,
            })
        {
            self.core
                .set_peer_cert_fingerprint(&HostId(peer.as_str().to_string()), previous);
            return Err(error);
        }
        applied.push(AppliedChange::DeclarationMoved {
            peer: peer.clone(),
            previous,
        });
        outcome.declared_only.push(peer.as_str().to_string());
        Ok(())
    }

    fn refuse(
        &self,
        outcome: &mut RotationOutcome,
        peer: &PeerId,
        reason: &str,
        manifest_version: u64,
    ) -> Result<(), CohortError> {
        self.audit.append(&CohortAuditEvent::CertRotationRefused {
            peer: peer.as_str().to_string(),
            reason: reason.to_string(),
            version: manifest_version,
        })?;
        outcome.refused.push(RotationRefusal {
            peer: peer.as_str().to_string(),
            reason: reason.to_string(),
        });
        Ok(())
    }

    /// Ledger writes recover a poisoned mutex instead of silently skipping.
    /// `if let Ok(...)` on a poisoned lock would hide EVERY later window from
    /// the operator read surface, which is the failure mode an observability
    /// surface must not have.
    fn ledger_write<R>(&self, write: impl FnOnce(&mut BTreeMap<String, RotationWindow>) -> R) -> R {
        let mut ledger = match self.ledger.lock() {
            Ok(ledger) => ledger,
            Err(poisoned) => poisoned.into_inner(),
        };
        write(&mut ledger)
    }

    /// Undo everything this call moved, in the ONLY order that keeps the
    /// invariant: plane A back to the fingerprint plane B still serves, THEN
    /// the window narrowed.
    ///
    /// A rolled-back peer already has a durable `CertRotationWindowOpened` (or
    /// `CertRotationDeclarationMoved`) row, so the rollback appends a
    /// COMPENSATING terminal row: without it the timeline would carry an
    /// unterminated transition for a manifest version that was never committed,
    /// contradicting the exactly-one-terminal-row contract. Best-effort by
    /// necessity — this path exists BECAUSE the sink refused, and the refusal
    /// can be transient (SQLite BUSY) — and loud on failure rather than silent.
    fn rollback(&self, applied: &[AppliedChange], manifest_version: u64) {
        for change in applied.iter().rev() {
            match change {
                AppliedChange::WindowOpened { peer, previous } => {
                    // Plane A FIRST, back to the fingerprint plane B still
                    // serves; only then narrow the accepted set.
                    self.core.set_peer_cert_fingerprint(
                        &HostId(peer.as_str().to_string()),
                        previous.clone(),
                    );
                    self.compensate_rollback(peer, previous, manifest_version);
                    self.pins.abort_rotation_window(peer);
                    self.ledger_write(|ledger| {
                        ledger.remove(peer.as_str());
                    });
                }
                AppliedChange::DeclarationMoved { peer, previous } => {
                    self.core.set_peer_cert_fingerprint(
                        &HostId(peer.as_str().to_string()),
                        previous.clone(),
                    );
                    self.compensate_rollback(peer, previous, manifest_version);
                    self.ledger_write(|ledger| {
                        ledger.remove(peer.as_str());
                    });
                }
                AppliedChange::WindowSuperseded {
                    peer,
                    previous_declaration,
                    prior_window,
                    prior_next,
                } => {
                    if let Err(error) = self.pins.open_rotation_window(peer, prior_next) {
                        eprintln!(
                            "cohort cert-rotation: could not restore superseded window for {}: \
                             {error}",
                            peer.as_str()
                        );
                    }
                    self.core.set_peer_cert_fingerprint(
                        &HostId(peer.as_str().to_string()),
                        previous_declaration.clone(),
                    );
                    self.ledger_write(|ledger| {
                        ledger.insert(peer.as_str().to_string(), prior_window.clone());
                    });
                    if let Err(error) =
                        self.audit
                            .append(&CohortAuditEvent::CertRotationWindowOpened {
                                peer: prior_window.peer.clone(),
                                retiring: prior_window.retiring.clone(),
                                next: prior_window.next.clone(),
                                version: prior_window.manifest_version,
                            })
                    {
                        eprintln!(
                            "cohort cert-rotation: restored-window row for {} was not written: \
                             {error}",
                            peer.as_str()
                        );
                    }
                }
            }
        }
    }

    fn compensate_rollback(
        &self,
        peer: &PeerId,
        previous: &PeerCertFingerprint,
        manifest_version: u64,
    ) {
        if let Err(error) = self.audit.append(&CohortAuditEvent::CertRotationRefused {
            peer: peer.as_str().to_string(),
            reason: format!(
                "reload rolled back before commit; both planes restored to {}",
                previous.wire()
            ),
            version: manifest_version,
        }) {
            eprintln!(
                "cohort cert-rotation: compensating row for {} was not written: {error}",
                peer.as_str()
            );
        }
    }

    /// Arm the real deadline for ONE window. Terminal action: promote-and-retire
    /// the generation THIS window opened, then a row that says how it ENDED — a
    /// rotation that reports its start and never its end is the failure AC4.3
    /// exists to prevent.
    /// Arm the real deadline for ONE window instance.
    ///
    /// The terminal action is promote-and-retire, but it is **conditional on
    /// three things**, and each condition closes a defect the §A6 close pass
    /// found in the unconditional version:
    ///
    /// 1. **This closer must still own the window.** The timer seam is
    ///    fire-and-forget (no cancellation handle exists), so every path that
    ///    ends a window early — rollback, supersede, the hazard-(j) refusal, an
    ///    evidence failure — leaves a timer armed. Binding by fingerprint VALUE
    ///    alone is not enough: a window aborted and RE-OPENED on the same
    ///    generation would be promoted by the stale timer, cutting the new
    ///    window's §7.2.1.a grace to nothing, and the stale closer would delete
    ///    the live window's ledger row by peer key, hiding a genuinely widened
    ///    trust set from the read surface. So the ledger row's `instance` is
    ///    checked first and a stale closer does NOTHING AT ALL.
    /// 2. **Plane A must have committed.** The production audit sink cannot
    ///    return `Err` — it PANICS (§7.3 I2) — and an unwind between arming and
    ///    plane A leaves the window open with plane A still on `retiring`.
    ///    PROMOTING then would move the pin to a generation plane A does not
    ///    declare and tear the mesh permanently. When plane A has not moved, the
    ///    window is ABORTED instead: the accepted set narrows to the generation
    ///    the mesh is actually serving, which is the coherent worst case the
    ///    arm-early ordering claims.
    /// 3. **A realignment must not undo a live rotation.** Plane A is realigned
    ///    to the serving pin ONLY when no window is open for the peer;
    ///    otherwise a closer whose own window vanished would drag plane A off a
    ///    DIFFERENT, in-flight rotation.
    fn schedule_close(&self, window: &RotationWindow, manifest_version: u64) {
        let pins = Arc::clone(&self.pins);
        let core = Arc::clone(&self.core);
        let audit = Arc::clone(&self.audit);
        let ledger = Arc::clone(&self.ledger);
        let transition_lock = Arc::clone(&self.transition_lock);
        let window = window.clone();
        self.timer.schedule(
            self.grace,
            Box::new(move || {
                let _transition = match transition_lock.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                let peer = PeerId::new(&window.peer);
                // (1) Still ours? A stale closer touches nothing — not the
                // store, not the ledger, not the journal.
                let owns = {
                    let ledger = match ledger.lock() {
                        Ok(ledger) => ledger,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    ledger
                        .get(&window.peer)
                        .is_some_and(|row| row.instance == window.instance)
                };
                if !owns {
                    return;
                }
                // (2) Did plane A commit? If not, narrowing is the only
                // coherent ending.
                let declared_now = core
                    .lookup_peer(&HostId(window.peer.clone()))
                    .ok()
                    .map(|config| config.cert_fingerprint.wire());
                let plane_a_committed = declared_now.as_deref() == Some(window.next.as_str());
                let expected = PeerCertFingerprint::parse(&window.next);
                let event = if plane_a_committed {
                    match expected
                        .as_ref()
                        .and_then(|expected| pins.close_rotation_window_if(&peer, expected))
                    {
                        Some(retired) => CohortAuditEvent::CertRotationWindowClosed {
                            peer: window.peer.clone(),
                            retired: retired.wire(),
                            promoted: window.next.clone(),
                            version: manifest_version,
                        },
                        None => {
                            let realigned = realign_plane_a(&pins, &core, &peer, &window.peer);
                            CohortAuditEvent::CertRotationRefused {
                                peer: window.peer.clone(),
                                reason: format!(
                                    "grace elapsed with no open window for generation {}; nothing \
                                     was promoted{realigned}",
                                    window.next
                                ),
                                version: manifest_version,
                            }
                        }
                    }
                } else {
                    // The window opened but plane A never declared it — an
                    // unwind between the two writes, or a refusal. Narrow, do
                    // not promote.
                    let discarded = pins.abort_rotation_window(&peer);
                    CohortAuditEvent::CertRotationRefused {
                        peer: window.peer.clone(),
                        reason: format!(
                            "grace elapsed with the router declaration still on {} rather than \
                             {}; the window on {} was DISCARDED, never promoted — promoting a \
                             generation this node does not declare would refuse every frame in \
                             both directions",
                            declared_now.unwrap_or_else(|| "an undeclared peer".to_string()),
                            window.next,
                            discarded
                                .map(|fingerprint| fingerprint.wire())
                                .unwrap_or_else(|| "nothing".to_string()),
                        ),
                        version: manifest_version,
                    }
                };
                {
                    let mut ledger = match ledger.lock() {
                        Ok(ledger) => ledger,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    // Remove only if it is still ours (re-checked: the
                    // ownership read above released the lock).
                    if ledger
                        .get(&window.peer)
                        .is_some_and(|row| row.instance == window.instance)
                    {
                        ledger.remove(&window.peer);
                    }
                }
                if let Err(error) = audit.append(&event) {
                    // No caller to return to: fail LOUD instead of losing the
                    // terminal row silently.
                    eprintln!(
                        "cohort cert-rotation: terminal audit row for {} was not written: {error}",
                        window.peer
                    );
                }
            }),
        );
    }
}

/// Put plane A back on the generation plane B actually serves — but ONLY when no
/// window is open for the peer. A closer whose own window vanished must not drag
/// plane A off a DIFFERENT, in-flight rotation, which is what an unconditional
/// realignment does.
fn realign_plane_a(
    pins: &InMemoryTofuPinStore,
    core: &A2ARouterCore,
    peer: &PeerId,
    peer_name: &str,
) -> String {
    if pins.rotation_next(peer).is_some() {
        return "; plane A left alone — another window is open for this peer".to_string();
    }
    match pins.get_pin_sync(peer).map(|pin| pin.fingerprint) {
        Some(serving) => {
            match core.set_peer_cert_fingerprint(&HostId(peer_name.to_string()), serving.clone()) {
                Some(previous) if previous != serving => format!(
                    "; plane A realigned from {} to the serving pin {}",
                    previous.wire(),
                    serving.wire()
                ),
                _ => String::new(),
            }
        }
        None => "; the peer holds no pin at all".to_string(),
    }
}
