//! TOFU (Trust On First Use) pin store + re-pin protocol.
//!
//! Per architecture §7.2: "First-contact TOFU pinning verifies the configured
//! fingerprint; subsequent connections re-verify against the pinned cert."
//!
//! Per NFR-Rel-6 (Story 6.3 AC4): "Spirit-restart invalidates prior A2A TOFU
//! pins; re-pin protocol with consent confirmation."

use crate::error::A2AError;
use crate::identity::{PeerCertFingerprint, PeerId};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant, SystemTime};

/// A TOFU pin record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TofuPin {
    pub peer: PeerId,
    pub fingerprint: PeerCertFingerprint,
    /// Spirit boot_nonce captured at pin time. NFR-Rel-6: when a new
    /// boot_nonce arrives for the same `(peer, spirit_id)`, the pin is
    /// invalidated and re-pin consent is required.
    pub boot_nonce: u64,
    /// Monotonic time at pin (used for staleness diagnostics; NOT a TTL).
    pub pinned_at_ns: u64,
    /// When the pin was invalidated; `None` = active.
    pub invalidated: Option<Invalidated>,
    /// Approval id of the operator consent that re-pinned this record.
    /// `None` for the first-contact pin; populated by re-pin approval.
    pub repin_approval_id: Option<[u8; 16]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Invalidated {
    SpiritRestarted { prior_boot_nonce: u64 },
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EPinMismatch {
    #[error("TOFU pin mismatch for peer {peer}: pinned {pinned} observed {observed}")]
    Mismatch {
        peer: String,
        pinned: String,
        observed: String,
    },
    #[error("no TOFU pin recorded for peer {0} — first-contact not yet attempted")]
    NotPinned(String),
    #[error("TOFU pin invalidated for peer {peer}: {reason}")]
    Invalidated { peer: String, reason: String },
    /// Story 14-2 / AC2.2.a.i — `open_rotation_window` refused a `next`
    /// fingerprint already held by ANOTHER peer's current pin or open
    /// window. Founder-ratified 2026-08-28 as a NEW variant (never a reuse
    /// of `Mismatch`): a collision is an impersonation attempt in progress,
    /// a different security event than "observed cert does not match the
    /// pin", and conflating them makes the two indistinguishable in logs
    /// and SIEM precisely when someone is investigating one. `EPinMismatch`
    /// is `#[non_exhaustive]`, so the variant is non-breaking downstream.
    #[error("rotation window refused for peer {peer}: fingerprint {fingerprint} is already held by {colliding_peer}")]
    FingerprintCollision {
        peer: String,
        colliding_peer: String,
        fingerprint: String,
    },
}

/// Operator decision on a re-pin request.
///
/// Story 6.3 AC4: re-pin requires explicit consent confirmation via the
/// Approval Decision Log (the existing `interactive` prompt-class surface
/// from architecture §8.3). The decision is recorded by `approval_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RePinDecision {
    AcceptedByOperator {
        approval_id: [u8; 16],
    },
    RejectedByOperator {
        reason: String,
    },
    /// Operator approved the re-pin, but the store refused to materialize it
    /// because doing so would violate a security invariant.
    RejectedByStore {
        error: EPinMismatch,
    },
    /// Story 21-3 — the approval was not applied because the pin file could
    /// not be written; the pin stays invalidated and the request stays pending.
    PersistFailed {
        detail: String,
    },
    TimedOut,
}

/// Story 19-7 / D-19-7-06 A — lifetime of a pending re-pin request
/// (Story 6.3 AC4 4.5).
pub const REPIN_REQUEST_TTL: Duration = Duration::from_secs(300);

/// Story 19-7 — one NFR-Rel-6 restart detection awaiting operator consent.
/// `pinned_fingerprint` is the invalidated pin's identity, the one a
/// consented re-pin keeps (D-19-7-04 A). `expired` is computed from the
/// store's monotonic deadline by [`InMemoryTofuPinStore::pending_repins`];
/// it is `false` in the value handed to the sink and in the one returned by
/// [`InMemoryTofuPinStore::record_repin_decision`] (which refuses expired
/// requests).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepinRequest {
    pub peer: PeerId,
    pub pinned_fingerprint: PeerCertFingerprint,
    pub prior_boot_nonce: u64,
    pub observed_boot_nonce: u64,
    pub detected_at: SystemTime,
    pub expires_at: SystemTime,
    pub expired: bool,
}

/// Story 19-7 / D-19-7-05 A — the operator verdict recorded on a pending
/// request for [`TofuPinStore::await_repin_consent`] to consume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepinVerdict {
    Approve { approval_id: [u8; 16] },
    Reject { reason: String },
}

/// Story 19-7 — why a verdict was not recorded; nothing changed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RepinDecisionError {
    #[error("no re-pin request pending for peer {peer}")]
    NotPending { peer: String },
    #[error("re-pin request for peer {peer} observed boot_nonce {observed}, not {supplied}")]
    NonceMismatch {
        peer: String,
        observed: u64,
        supplied: u64,
    },
    #[error("re-pin request for peer {peer} expired")]
    Expired { peer: String },
}

/// Story 19-7 / D-19-7-08 A — receives every new restart detection, in
/// detection order: it is invoked while the store's `window_lock` is held.
/// It MUST therefore only hand the request off without blocking (e.g. a
/// bounded channel `try_send`) and MUST NOT call back into the store — a
/// blocking sink stalls every pin-lifecycle write, a re-entrant one deadlocks.
pub type RepinSink = Arc<dyn Fn(&RepinRequest) + Send + Sync>;

struct PendingRepin {
    request: RepinRequest,
    deadline: Instant,
    verdict: Option<RepinVerdict>,
    /// Edge F4 — a rotation window was open for the peer at (any) detection
    /// of this request; an approval is then refused by the store.
    rotating: bool,
}

/// Story 21-3 — one peer's durable pin row. `pinned_at_ns`, rotation windows
/// and recorded verdicts are never part of it: a restart closes every window
/// and a verdict is not consent that outlives the process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinRecord {
    pub pin: TofuPin,
    /// The `[[tcp.peer_pins]]` entry `(fingerprint, boot_nonce)` this peer was
    /// seeded from (D-21-3-02).
    pub anchor: (PeerCertFingerprint, u64),
    /// Count of pinned-fingerprint changes since first seed (D-21-3-05).
    pub generation: u64,
    /// Observed boot nonce of the pending re-pin request (D-21-3-03 A). A request
    /// armed while a rotation window was open is not recorded: it is not approvable.
    pub pending_observed: Option<u64>,
    /// Story 23-7 / D-23-7-07 A — a rotation promotion closed on this peer's current
    /// fingerprint (its signed rotation retired the previous one). Cleared by a re-pair or
    /// a re-seed; a pin file written before schema 2 reads as `false`.
    pub promoted: bool,
}

/// Story 21-3 / D-21-3-01 A — the synchronous durability port. `store` runs
/// under the store's `window_lock` (never on a verifier read path), so it must
/// not call back into the store.
pub trait PinPersistence: Send + Sync {
    /// The records read and validated when the backing file was opened.
    fn load(&self) -> Vec<PinRecord>;
    /// Story 23-7 / D-23-7-09 A — the highest signed-manifest version this host durably applied
    /// (`0` when none was recorded).
    fn load_floor(&self) -> u64 {
        0
    }
    /// Durably replace the whole record set together with the manifest version `floor`. A
    /// success also lifts any fence.
    fn store(&self, records: &[PinRecord], floor: u64) -> Result<(), String>;
    /// Story 23-7 / D-23-7-08 A — [`Self::store`] for a transition that NARROWS trust
    /// (retirement, invalidation, a raised floor). An implementation with a fence marks the file
    /// untrustworthy BEFORE the write and lifts the mark only once the write is durable, so a
    /// crash at any point between leaves the next open refusing. The default has no fence.
    fn store_narrowing(&self, records: &[PinRecord], floor: u64) -> Result<(), String> {
        self.store(records, floor)
    }
    /// A narrowing transition (retirement, invalidation) could not be written:
    /// durably mark the backing file untrustworthy so the next open refuses
    /// instead of reviving the older snapshot. It must not need a fresh
    /// directory entry, since the failing write may be exactly that.
    fn fence(&self) -> Result<(), String>;
}

struct PinMeta {
    anchor: (PeerCertFingerprint, u64),
    generation: u64,
    promoted: bool,
}

/// TOFU pin store trait. [`InMemoryTofuPinStore`] is the only implementation;
/// durability is a [`PinPersistence`] port on it (implemented by
/// `maos-persistence`'s `FilePinStore`), so this surface stays unchanged.
#[async_trait]
pub trait TofuPinStore: Send + Sync {
    /// First-contact pin recording. The operator config's declared fingerprint
    /// is matched against the observed cert fingerprint; mismatch fires
    /// `EPinMismatch::Mismatch` at first contact (the strict-binding semantic
    /// from architecture §7.2).
    async fn pin_first_contact(
        &self,
        peer: &PeerId,
        observed: &PeerCertFingerprint,
        declared: &PeerCertFingerprint,
        boot_nonce: u64,
    ) -> Result<TofuPin, EPinMismatch>;

    /// Re-verify: every subsequent connection compares the observed cert to
    /// the pinned fingerprint. NFR-Sec-12: 100% pin-mismatch detected.
    async fn verify_pinned(
        &self,
        peer: &PeerId,
        observed: &PeerCertFingerprint,
    ) -> Result<(), EPinMismatch>;

    /// NFR-Rel-6: Spirit-restart invalidates prior A2A TOFU pins. Called
    /// from `LifecycleHooks::on_restart_observed_at_peer`; the pin is
    /// marked `Invalidated::SpiritRestarted` and re-pin consent is required.
    async fn invalidate_for_restart(
        &self,
        peer: &PeerId,
        prior_boot_nonce: u64,
    ) -> Result<(), A2AError>;

    /// Await operator re-pin consent decision. Returns the decision recorded
    /// in the Approval Decision Log. On acceptance, materializes a new pin
    /// with the supplied `new_boot_nonce`.
    async fn await_repin_consent(
        &self,
        peer: &PeerId,
        new_observed: &PeerCertFingerprint,
        new_boot_nonce: u64,
    ) -> RePinDecision;

    /// Read the current pin (if any) — used by the chaos harness + diagnostics.
    async fn get_pin(&self, peer: &PeerId) -> Option<TofuPin>;

    /// Story 8.9 / AC6.3 (G5b) — ATOMIC compare-and-invalidate for Spirit-restart
    /// detection. Reads the stored `boot_nonce` and, iff it differs from
    /// `observed_boot_nonce`, marks the pin `Invalidated::SpiritRestarted` —
    /// under ONE critical section so a concurrent intake cannot interleave
    /// between the read and the invalidation (the prior `get_pin` + separate
    /// `invalidate_for_restart` was a check-then-act TOCTOU). Returns
    /// `Ok(Some(prior_boot_nonce))` when the pin was invalidated, `Ok(None)` when
    /// the nonce matched (no-op) or no pin exists.
    async fn invalidate_if_boot_nonce_differs(
        &self,
        peer: &PeerId,
        observed_boot_nonce: u64,
    ) -> Result<Option<u64>, A2AError>;
}

/// Default in-memory TOFU pin store, and the only [`TofuPinStore`]. It is made
/// durable through an optional [`PinPersistence`] port, never a second store:
/// the 14-2/19-7 lock-ordering invariants live in this one implementation.
pub struct InMemoryTofuPinStore {
    pins: Arc<DashMap<String, TofuPin>>,
    /// Story 14-2 / AC2.1 — the rotation window side-map: peer → NEXT
    /// fingerprint. A SIDE-MAP, not a `TofuPin` field, on four measured
    /// grounds: zero serde delta (`TofuPin` stays byte-identical, so the
    /// `PinnedFingerprint` / `TcpA2AConfig` strict-schema blast radius is
    /// untouched); fail-closed on restart by construction (a process-local
    /// window cannot be persisted, so a reboot closes it — a `TofuPin` field
    /// would RE-OPEN a widened trust set on reboot, the wrong default);
    /// `PinnedFingerprint` deliberately gains no `next` (a boot-time window
    /// nobody closes is the permanently widened trust set this story exists
    /// to prevent); and the window becomes a state you read rather than a
    /// field you interpret.
    rotation_next: Arc<DashMap<String, PeerCertFingerprint>>,
    /// §A6 mid-story (Blind-3/Edge-2): serializes open's scan+insert,
    /// close's remove+promote, and the invalidation/re-pin lifecycle — no
    /// half-applied window can interleave. Lock ORDER: `window_lock` BEFORE
    /// any `pins` entry lock.
    window_lock: Arc<std::sync::Mutex<()>>,
    /// Story 19-7 / D-19-7-06 A — peer → restart detection awaiting consent,
    /// written only under `window_lock`. A detection with a different
    /// observed nonce (or after expiry) supersedes the entry, dropping any
    /// recorded verdict; a repeat of the same nonce leaves it untouched.
    /// No intake path re-detects an invalidated pin: router intake refuses it in
    /// `verify_pinned` (`-32002`) before `invalidate_if_boot_nonce_differs`
    /// runs, and over TCP the LISTEN-side TLS verifier
    /// ([`InMemoryTofuPinStore::find_active_pin_by_fingerprint`]) refuses its
    /// certificate even earlier. The supersede, repeat and post-expiry branches
    /// are reachable only by direct store calls (tests); in production no
    /// further detection occurs for that peer until consent re-activates the
    /// pin or this host restarts. There is no public pin-removal or
    /// manual-invalidation API; were one added, it should drop the peer's entry
    /// here (consent re-checks the pin anyway).
    pending_repins: DashMap<String, PendingRepin>,
    repin_ttl: Duration,
    repin_sink: OnceLock<RepinSink>,
    /// Story 21-3 — durable-only per-peer fields, written under `window_lock`.
    meta: DashMap<String, PinMeta>,
    persistence: Option<Arc<dyn PinPersistence>>,
    /// A narrowing transition could not be persisted; cleared by the next
    /// successful write.
    degraded: AtomicBool,
    /// Story 23-7 / D-23-7-09 A — highest applied signed-manifest version; durable through the
    /// port, written under `window_lock`.
    manifest_floor: AtomicU64,
}

impl Default for InMemoryTofuPinStore {
    fn default() -> Self {
        Self {
            pins: Arc::new(DashMap::new()),
            rotation_next: Arc::new(DashMap::new()),
            window_lock: Arc::new(std::sync::Mutex::new(())),
            pending_repins: DashMap::new(),
            repin_ttl: REPIN_REQUEST_TTL,
            repin_sink: OnceLock::new(),
            meta: DashMap::new(),
            persistence: None,
            degraded: AtomicBool::new(false),
            manifest_floor: AtomicU64::new(0),
        }
    }
}

impl InMemoryTofuPinStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Story 19-7 — override the pending-request lifetime (default
    /// [`REPIN_REQUEST_TTL`]).
    pub fn with_repin_ttl(mut self, ttl: Duration) -> Self {
        self.repin_ttl = ttl;
        self
    }

    /// Story 21-3 / D-21-3-01 A — make every pin-lifecycle write durable
    /// through `port`.
    pub fn with_persistence(mut self, port: Arc<dyn PinPersistence>) -> Self {
        self.manifest_floor
            .store(port.load_floor(), Ordering::Release);
        self.persistence = Some(port);
        self
    }

    /// Story 19-7 / D-19-7-08 A — set-once detection sink, installable on a
    /// bound (shared) store. Called under `window_lock` (see [`RepinSink`] for
    /// the non-blocking requirement), once per new or superseding detection,
    /// so sink rows follow detection order. Requests already pending (re-armed
    /// at boot, D-21-3-03 A) are replayed through it on install. `false`: one
    /// is already installed, `sink` dropped.
    #[must_use]
    pub fn install_repin_sink(&self, sink: RepinSink) -> bool {
        let _guard = self.lock_window();
        if self.repin_sink.set(sink).is_err() {
            return false;
        }
        if let Some(sink) = self.repin_sink.get() {
            self.pending_repins
                .iter()
                .for_each(|entry| sink(&entry.request));
        }
        true
    }

    /// Story 19-7 — every pending request, `expired` stamped from the
    /// monotonic deadline. Expired ones stay listed until superseded so a
    /// decision on them reads `Expired`, not `NotPending`.
    pub fn pending_repins(&self) -> Vec<RepinRequest> {
        let now = Instant::now();
        self.pending_repins
            .iter()
            .map(|entry| RepinRequest {
                expired: now >= entry.deadline,
                ..entry.request.clone()
            })
            .collect()
    }

    /// Story 21-3 — install a record loaded from the durable file without
    /// writing it back (boot reconciliation calls [`Self::flush`] once after).
    /// An invalidated pin with a recorded request is re-armed with a fresh TTL
    /// (D-21-3-03 A); rotation windows are never restored.
    pub fn restore(&self, record: PinRecord) -> Result<(), EPinMismatch> {
        let _guard = self.lock_window();
        let PinRecord {
            pin,
            anchor,
            generation,
            pending_observed,
            promoted,
        } = record;
        if let Some(colliding_peer) = self.colliding_peer(&pin.peer, &pin.fingerprint) {
            return Err(EPinMismatch::FingerprintCollision {
                peer: pin.peer.as_str().to_string(),
                colliding_peer,
                fingerprint: pin.fingerprint.wire(),
            });
        }
        if self.pins.contains_key(pin.peer.as_str()) {
            return Err(EPinMismatch::Invalidated {
                peer: pin.peer.as_str().to_string(),
                reason: "pin already exists — duplicate peer".into(),
            });
        }
        let key = pin.peer.as_str().to_string();
        if let (Some(Invalidated::SpiritRestarted { prior_boot_nonce }), Some(observed)) =
            (&pin.invalidated, pending_observed)
        {
            let detected_at = SystemTime::now();
            let request = RepinRequest {
                peer: pin.peer.clone(),
                pinned_fingerprint: pin.fingerprint.clone(),
                prior_boot_nonce: *prior_boot_nonce,
                observed_boot_nonce: observed,
                detected_at,
                expires_at: detected_at + self.repin_ttl,
                expired: false,
            };
            let deadline = Instant::now() + self.repin_ttl;
            self.pending_repins.insert(
                key.clone(),
                PendingRepin {
                    request,
                    deadline,
                    verdict: None,
                    rotating: false,
                },
            );
        }
        self.meta.insert(
            key.clone(),
            PinMeta {
                anchor,
                generation,
                promoted,
            },
        );
        self.pins.insert(
            key,
            TofuPin {
                pinned_at_ns: now_ns(),
                ..pin
            },
        );
        Ok(())
    }

    /// Story 21-3 — durably write the current pin set (boot reconciliation).
    pub fn flush(&self) -> Result<(), String> {
        let _guard = self.lock_window();
        self.persist(None)
    }

    /// Story 21-3 — every pin as its durable record, sorted by peer.
    pub fn records(&self) -> Vec<PinRecord> {
        let _guard = self.lock_window();
        self.records_with(None)
    }

    /// Story 21-3 — the records, or `None` while degraded, read together under
    /// `window_lock` so a concurrent failing narrowing write cannot split them.
    pub fn healthy_records(&self) -> Option<Vec<PinRecord>> {
        let _guard = self.lock_window();
        (!self.is_degraded()).then(|| self.records_with(None))
    }

    /// Story 21-3 / D-21-3-04 A — a narrowing transition could not be
    /// persisted; stays set until a later write succeeds.
    pub fn is_degraded(&self) -> bool {
        self.degraded.load(Ordering::Acquire)
    }

    /// Story 23-7 / D-23-7-07 A — `peer`'s current pin is the product of a rotation promotion
    /// (the durable join that types a post-grace refusal).
    pub fn is_promoted(&self, peer: &PeerId) -> bool {
        self.meta
            .get(peer.as_str())
            .is_some_and(|meta| meta.promoted)
    }

    /// Story 23-7 / D-23-7-09 A — the highest signed-manifest version durably applied.
    pub fn manifest_floor(&self) -> u64 {
        self.manifest_floor.load(Ordering::Acquire)
    }

    /// Raise the durable manifest-version floor (never lowers it). A raise narrows what a replayed
    /// signed manifest may do, so it is written like any narrowing transition.
    pub fn raise_manifest_floor(&self, version: u64) {
        let _guard = self.lock_window();
        if version > self.manifest_floor.load(Ordering::Acquire) {
            self.manifest_floor.store(version, Ordering::Release);
            self.persist_narrowed();
        }
    }

    /// The pin set as records; `change` replaces (or adds) its peer's record.
    fn records_with(&self, change: Option<PinRecord>) -> Vec<PinRecord> {
        let mut records: Vec<PinRecord> = self
            .pins
            .iter()
            .filter(|entry| {
                change
                    .as_ref()
                    .is_none_or(|changed| changed.pin.peer.as_str() != entry.key())
            })
            .map(|entry| self.record_of(entry.value().clone()))
            .collect();
        records.extend(change);
        records.sort_by(|left, right| left.pin.peer.as_str().cmp(right.pin.peer.as_str()));
        records
    }

    fn record_of(&self, pin: TofuPin) -> PinRecord {
        let key = pin.peer.as_str();
        let (anchor, generation, promoted) = self.meta.get(key).map_or_else(
            || ((pin.fingerprint.clone(), pin.boot_nonce), 0, false),
            |meta| (meta.anchor.clone(), meta.generation, meta.promoted),
        );
        // A request armed while a rotation window was open can never be approved
        // (`rotating`); it is not recorded, so a restart cannot re-arm it as approvable.
        let pending_observed = self
            .pending_repins
            .get(key)
            .filter(|pending| !pending.rotating)
            .map(|pending| pending.request.observed_boot_nonce);
        PinRecord {
            pin,
            anchor,
            generation,
            pending_observed,
            promoted,
        }
    }

    /// Write the pin set (with `change` applied); a success clears `degraded`.
    /// Caller holds `window_lock`; never called with a `pins` entry guard held.
    fn persist(&self, change: Option<PinRecord>) -> Result<(), String> {
        self.persist_with(change, false)
    }

    fn persist_with(&self, change: Option<PinRecord>, narrowing: bool) -> Result<(), String> {
        let Some(port) = &self.persistence else {
            return Ok(());
        };
        let records = self.records_with(change);
        let floor = self.manifest_floor.load(Ordering::Acquire);
        if narrowing {
            port.store_narrowing(&records, floor)?;
        } else {
            port.store(&records, floor)?;
        }
        self.degraded.store(false, Ordering::Release);
        Ok(())
    }

    /// Narrowing transitions apply in memory; a refused write marks the store
    /// degraded (D-21-3-04 A) and FENCES the durable file, so a restart refuses
    /// to revive the older, wider snapshot the failed write left behind.
    fn persist_narrowed(&self) {
        if let Err(detail) = self.persist_with(None, true) {
            self.degraded.store(true, Ordering::Release);
            tracing::error!(%detail, "TOFU pin file not written; pins degraded");
            if let Some(Err(fence)) = self.persistence.as_ref().map(|port| port.fence()) {
                tracing::error!(%fence, "TOFU pin file not fenced; a restart may revive retired pins");
            }
        }
    }

    /// Persist the pin set with `pending` already removed. A refused write puts the
    /// request back pending with no verdict (nothing was decided) and reports
    /// `PersistFailed`; the in-memory and durable states then agree.
    fn commit_consumed(&self, peer: &PeerId, pending: PendingRepin) -> Result<(), RePinDecision> {
        self.persist(None).map_err(|detail| {
            self.pending_repins
                .insert(peer.as_str().to_string(), pending);
            RePinDecision::PersistFailed { detail }
        })
    }

    /// A promotion changed `peer`'s fingerprint: bump its durable generation and mark the
    /// current pin as promoted (the durable join behind `CertPostGraceReject`).
    fn persist_promotion(&self, peer: &PeerId) {
        if let Some(mut meta) = self.meta.get_mut(peer.as_str()) {
            meta.generation += 1;
            meta.promoted = true;
        }
        self.persist_narrowed();
    }

    /// Story 19-7 / D-19-7-03 A — record the operator verdict on `peer`'s
    /// pending request; `boot_nonce` must echo its observed nonce. Changes no
    /// pin: [`TofuPinStore::await_repin_consent`] consumes the verdict, called
    /// with the returned `pinned_fingerprint` and `observed_boot_nonce`.
    pub fn record_repin_decision(
        &self,
        peer: &PeerId,
        boot_nonce: u64,
        verdict: RepinVerdict,
    ) -> Result<RepinRequest, RepinDecisionError> {
        let _guard = self.lock_window();
        let peer_name = || peer.as_str().to_string();
        let mut entry = self
            .pending_repins
            .get_mut(peer.as_str())
            .ok_or_else(|| RepinDecisionError::NotPending { peer: peer_name() })?;
        if Instant::now() >= entry.deadline {
            return Err(RepinDecisionError::Expired { peer: peer_name() });
        }
        if entry.request.observed_boot_nonce != boot_nonce {
            return Err(RepinDecisionError::NonceMismatch {
                peer: peer_name(),
                observed: entry.request.observed_boot_nonce,
                supplied: boot_nonce,
            });
        }
        entry.verdict = Some(verdict);
        Ok(entry.request.clone())
    }

    /// Story 19-7 / CR-2 — drop a recorded verdict whose audit row could not
    /// be written; the request stays pending. `true` if one was dropped.
    pub fn withdraw_repin_decision(&self, peer: &PeerId) -> bool {
        let _guard = self.lock_window();
        self.pending_repins
            .get_mut(peer.as_str())
            .and_then(|mut entry| entry.verdict.take())
            .is_some()
    }

    /// Serialize rotation-window and pin-lifecycle writes. If an earlier
    /// writer panicked, discard every transient `next` before recovering the
    /// guard: current pins remain fail-closed, while no widened trust survives.
    fn lock_window(&self) -> std::sync::MutexGuard<'_, ()> {
        match self.window_lock.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                let guard = poisoned.into_inner();
                self.rotation_next.clear();
                guard
            }
        }
    }

    /// Return the other peer already holding `fingerprint` as either a current
    /// pin or an open rotation generation.
    fn colliding_peer(&self, peer: &PeerId, fingerprint: &PeerCertFingerprint) -> Option<String> {
        self.pins
            .iter()
            .find_map(|entry| {
                let pin = entry.value();
                (pin.peer.as_str() != peer.as_str() && &pin.fingerprint == fingerprint)
                    .then(|| pin.peer.as_str().to_string())
            })
            .or_else(|| {
                self.rotation_next.iter().find_map(|entry| {
                    (entry.key() != peer.as_str() && entry.value() == fingerprint)
                        .then(|| entry.key().clone())
                })
            })
    }

    /// Story 8.6 — synchronous pin lookup for the rustls verifier callback.
    ///
    /// rustls `ServerCertVerifier`/`ClientCertVerifier` callbacks are **sync**,
    /// but [`TofuPinStore::verify_pinned`] is `async`. Rather than change that
    /// frozen async signature (epic AC-A6), this additive inherent helper does
    /// the pin comparison synchronously against the in-memory `DashMap`
    /// (`verify_pinned`'s in-memory body is itself non-blocking; the `async` is
    /// only the trait indirection — see Dev Notes "async verify_pinned inside a
    /// sync rustls callback", bridge option (a)).
    ///
    /// Returns the [`PeerId`] whose **active** pin equals `observed`, or `None`
    /// (unpinned / invalidated). This is the TOFU trust oracle the
    /// `TofuPinningVerifier` consults after WebPKI succeeds.
    pub fn find_active_pin_by_fingerprint(&self, observed: &PeerCertFingerprint) -> Option<PeerId> {
        if let Some(peer) = self.pins.iter().find_map(|entry| {
            let pin = entry.value();
            if pin.invalidated.is_none() && &pin.fingerprint == observed {
                Some(pin.peer.clone())
            } else {
                None
            }
        }) {
            return Some(peer);
        }
        self.rotation_next.iter().find_map(|entry| {
            if entry.value() == observed {
                // §A6 mid-story Edge-1: a window is an ACTIVE identity only
                // while the peer's CURRENT pin is active — an invalidated
                // pin (Spirit restart) must not keep the replacement
                // trusted at the listen side.
                let pin_active = self
                    .pins
                    .get(entry.key())
                    .map(|p| p.value().invalidated.is_none())
                    .unwrap_or(false);
                if pin_active {
                    Some(PeerId::new(entry.key().clone()))
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    /// Story 14-2 / AC2.1 — open the one-generation overlap window for
    /// `peer` (§7.2.1.a: agents accept either cert on inbound handshakes
    /// during `[t_provision, t_revoke + T_grace]`; the close is an explicit
    /// state transition, never a clock — AC2.1.c). The store additionally
    /// accepts `next` as this peer's active identity until the window
    /// closes.
    ///
    /// ⚠ **THE UNIQUENESS GUARD IS THE SECURITY CONTROL, NOT A RIDER
    /// (AC2.2.a).** Without it this method OPENS a cross-peer
    /// impersonation vector: `find_active_pin_by_fingerprint` is
    /// first-match-wins over a per-process randomized `DashMap` iteration,
    /// so a `next` colliding with another peer's held fingerprint would
    /// resolve non-deterministically to EITHER peer at sites 2/3 — the
    /// attacker's key-holder dials in, site 2 accepts, site 3 may resolve
    /// the victim, and the binding check passes. Refused here, at open,
    /// with a dedicated variant the catalog can enumerate.
    pub fn open_rotation_window(
        &self,
        peer: &PeerId,
        next: &PeerCertFingerprint,
    ) -> Result<(), EPinMismatch> {
        let _guard = self.lock_window();
        let pin = self
            .pins
            .get(peer.as_str())
            .map(|r| r.value().clone())
            .ok_or_else(|| EPinMismatch::NotPinned(peer.as_str().to_string()))?;
        if pin.invalidated.is_some() {
            return Err(EPinMismatch::Invalidated {
                peer: peer.as_str().to_string(),
                reason: "cannot open a rotation window on an invalidated pin".into(),
            });
        }
        if &pin.fingerprint == next {
            return Err(EPinMismatch::FingerprintCollision {
                peer: peer.as_str().to_string(),
                colliding_peer: peer.as_str().to_string(),
                fingerprint: next.wire(),
            });
        }
        if let Some(existing) = self.rotation_next.get(peer.as_str()) {
            if existing.value() == next {
                return Ok(());
            }
            return Err(EPinMismatch::Invalidated {
                peer: peer.as_str().to_string(),
                reason: format!(
                    "rotation window already open for {}; close it before opening {}",
                    existing.value().wire(),
                    next.wire()
                ),
            });
        }
        if let Some(colliding_peer) = self.colliding_peer(peer, next) {
            return Err(EPinMismatch::FingerprintCollision {
                peer: peer.as_str().to_string(),
                colliding_peer,
                fingerprint: next.wire(),
            });
        }
        self.rotation_next
            .insert(peer.as_str().to_string(), next.clone());
        Ok(())
    }

    /// Story 14-2 / AC2.1.a — close the window PROMOTE-and-retire, never
    /// discard: the store must end up holding NEW, not OLD. Promotes
    /// `next → current` under the pin's entry lock, clears the side-map
    /// entry, and returns the RETIRED OLD fingerprint. A close that
    /// merely dropped `next` would leave the mesh pinned to a cert
    /// nobody serves. Returns `None` if no window was open. Serialized
    /// against open/re-pin by `window_lock` (§A6 Blind-3): no transition
    /// can interleave inside the remove→promote pair.
    pub fn close_rotation_window(&self, peer: &PeerId) -> Option<PeerCertFingerprint> {
        let _guard = self.lock_window();
        // Promote first: lock-free readers always accept the serving
        // generation throughout close. A recovered poisoned lock cleared all
        // transient windows, so `None` is fail-closed rather than stale trust.
        let next = self.rotation_next.get(peer.as_str())?.value().clone();
        let retired = self.pins.get_mut(peer.as_str()).map(|mut entry| {
            let retired = entry.fingerprint.clone();
            entry.fingerprint = next;
            entry.pinned_at_ns = now_ns();
            retired
        });
        // The window is cleared even when the pin has vanished (an invalidation
        // or a re-pin removed it between the two map reads). Leaving it would
        // ORPHAN a widened trust set that no later close can reach and that the
        // operator read surface would keep reporting — narrower is the only
        // fail-closed answer when the promotion target is gone.
        self.rotation_next.remove(peer.as_str());
        if retired.is_some() {
            self.persist_promotion(peer);
        }
        retired
    }

    /// Story 14-2a / AC5.1 — ABORT the window: DISCARD `next` without
    /// promoting, leaving `pins[peer]` exactly as it was. Returns the
    /// discarded generation so the caller can audit what it refused.
    ///
    /// This is a DIFFERENT transition from [`Self::close_rotation_window`],
    /// not a variant of it: close is promote-and-retire (14-2 AC2.1.a, which
    /// this does not reopen), abort is discard-and-keep. Two mechanisms
    /// wearing one name was 14-2's whole pathology, so they are two methods
    /// with two names.
    ///
    /// It is the ONLY reversible half of a rotation: before promotion the
    /// retiring fingerprint is still the serving pin, so a reload that fails
    /// part-way can put the mesh back exactly as it found it. After promotion
    /// there is no revert — a "rollback" is a NEW rotation back to the old
    /// fingerprint, subject to the same window, grace and uniqueness guard.
    ///
    /// ⚠ Callers restoring a torn reload MUST move plane A
    /// (`A2ARouterCore.peers[p].cert_fingerprint`) back to the serving pin
    /// BEFORE calling this: aborting first leaves plane A holding a
    /// fingerprint plane B no longer accepts, which sites 5/6 report as a
    /// `PinMismatch` on every frame in both directions.
    pub fn abort_rotation_window(&self, peer: &PeerId) -> Option<PeerCertFingerprint> {
        let _guard = self.lock_window();
        self.rotation_next
            .remove(peer.as_str())
            .map(|(_, next)| next)
    }

    /// Story 14-2a — promote-and-retire ONLY IF the open window is still the
    /// generation the caller opened. Returns the retired fingerprint, or
    /// `None` when no window is open, the open one is a DIFFERENT generation,
    /// or the pin entry has vanished — in which case the window is still
    /// CLEARED (fail-closed, mirror of [`Self::close_rotation_window`]) and
    /// nothing was promoted.
    ///
    /// ⚠ **A grace closer that closes by peer alone promotes the wrong
    /// generation.** A window can be cancelled underneath its own timer —
    /// `invalidate_if_boot_nonce_differs` and `lock_window` poison recovery
    /// both clear `rotation_next` — and a later reissue can then open a NEW
    /// window before the old timer fires. An unconditional close would promote
    /// that newer generation EARLY, before its own grace elapsed, and journal
    /// the terminal row against the wrong fingerprint. The comparison and the
    /// promotion happen under one `window_lock` acquisition, so nothing can
    /// interleave between them.
    pub fn close_rotation_window_if(
        &self,
        peer: &PeerId,
        expected: &PeerCertFingerprint,
    ) -> Option<PeerCertFingerprint> {
        let _guard = self.lock_window();
        let next = self.rotation_next.get(peer.as_str())?.value().clone();
        if &next != expected {
            return None;
        }
        // Once the expected generation matches, this closer owns that exact
        // window instance and must close it even when the promotion target
        // has vanished — the same fail-closed clear as close_rotation_window.
        // An early `?` here would ORPHAN a widened trust set no later close
        // could reach; narrower is the only fail-closed answer.
        let retired = self.pins.get_mut(peer.as_str()).map(|mut entry| {
            let retired = entry.fingerprint.clone();
            entry.fingerprint = next;
            entry.pinned_at_ns = now_ns();
            retired
        });
        self.rotation_next.remove(peer.as_str());
        if retired.is_some() {
            self.persist_promotion(peer);
        }
        retired
    }

    /// Story 14-2 / AC2.1 — the window oracle: this peer's open `next`,
    /// if a rotation window is currently open.
    pub fn rotation_next(&self, peer: &PeerId) -> Option<PeerCertFingerprint> {
        self.rotation_next
            .get(peer.as_str())
            .map(|e| e.value().clone())
    }

    /// Story 8.6 — synchronous mirror of [`TofuPinStore::verify_pinned`] for the
    /// sync rustls verifier callback (same semantics: `NotPinned` / `Invalidated`
    /// / `Mismatch`). Additive; does NOT change the async trait surface.
    pub fn verify_pinned_sync(
        &self,
        peer: &PeerId,
        observed: &PeerCertFingerprint,
    ) -> Result<(), EPinMismatch> {
        let pin = self
            .pins
            .get(peer.as_str())
            .map(|r| r.value().clone())
            .ok_or_else(|| EPinMismatch::NotPinned(peer.as_str().to_string()))?;
        if pin.invalidated.is_some() {
            let reason = match &pin.invalidated {
                Some(Invalidated::SpiritRestarted { prior_boot_nonce }) => {
                    format!("Spirit restarted (prior boot_nonce={prior_boot_nonce})")
                }
                Some(Invalidated::Manual) => "manually invalidated".to_string(),
                None => unreachable!(),
            };
            return Err(EPinMismatch::Invalidated {
                peer: peer.as_str().to_string(),
                reason,
            });
        }
        if &pin.fingerprint == observed {
            Ok(())
        } else if self
            .rotation_next
            .get(peer.as_str())
            .is_some_and(|next| next.value() == observed)
        {
            // Story 14-2 / AC2.2 (site 1, dial side + the sync mirror of
            // site 4): during an open rotation window the peer's `next` is
            // equally pinned — one-generation overlap, §7.2.1.a.
            Ok(())
        } else {
            Err(EPinMismatch::Mismatch {
                peer: peer.as_str().to_string(),
                pinned: pin.fingerprint.wire(),
                observed: observed.wire(),
            })
        }
    }

    /// Read the current pin (if any) synchronously — sync mirror of
    /// [`TofuPinStore::get_pin`] for the verifier callback / teardown asserts.
    pub fn get_pin_sync(&self, peer: &PeerId) -> Option<TofuPin> {
        self.pins.get(peer.as_str()).map(|r| r.value().clone())
    }
}

#[async_trait]
impl TofuPinStore for InMemoryTofuPinStore {
    async fn pin_first_contact(
        &self,
        peer: &PeerId,
        observed: &PeerCertFingerprint,
        declared: &PeerCertFingerprint,
        boot_nonce: u64,
    ) -> Result<TofuPin, EPinMismatch> {
        if observed != declared {
            return Err(EPinMismatch::Mismatch {
                peer: peer.as_str().to_string(),
                pinned: declared.wire(),
                observed: observed.wire(),
            });
        }
        let _guard = self.lock_window();
        if self.pins.contains_key(peer.as_str()) {
            return Err(EPinMismatch::Invalidated {
                peer: peer.as_str().to_string(),
                reason: "pin already exists — use re-pin consent path".into(),
            });
        }
        if let Some(colliding_peer) = self.colliding_peer(peer, observed) {
            return Err(EPinMismatch::FingerprintCollision {
                peer: peer.as_str().to_string(),
                colliding_peer,
                fingerprint: observed.wire(),
            });
        }
        let pin = TofuPin {
            peer: peer.clone(),
            fingerprint: observed.clone(),
            boot_nonce,
            pinned_at_ns: now_ns(),
            invalidated: None,
            repin_approval_id: None,
        };
        let key = peer.as_str().to_string();
        let anchor = (observed.clone(), boot_nonce);
        // Widening: persist FIRST; a refused write leaves nothing pinned (D-21-3-04 A).
        self.persist(Some(PinRecord {
            pin: pin.clone(),
            anchor: anchor.clone(),
            generation: 0,
            pending_observed: None,
            promoted: false,
        }))
        .map_err(|detail| EPinMismatch::Invalidated {
            peer: key.clone(),
            reason: format!("pin not persisted, nothing pinned: {detail}"),
        })?;
        self.meta.insert(
            key.clone(),
            PinMeta {
                anchor,
                generation: 0,
                promoted: false,
            },
        );
        self.pins.insert(key, pin.clone());
        Ok(pin)
    }

    async fn verify_pinned(
        &self,
        peer: &PeerId,
        observed: &PeerCertFingerprint,
    ) -> Result<(), EPinMismatch> {
        let pin = self
            .pins
            .get(peer.as_str())
            .map(|r| r.value().clone())
            .ok_or_else(|| EPinMismatch::NotPinned(peer.as_str().to_string()))?;
        if pin.invalidated.is_some() {
            let reason = match &pin.invalidated {
                Some(Invalidated::SpiritRestarted { prior_boot_nonce }) => {
                    format!("Spirit restarted (prior boot_nonce={prior_boot_nonce})")
                }
                Some(Invalidated::Manual) => "manually invalidated".to_string(),
                None => unreachable!(),
            };
            return Err(EPinMismatch::Invalidated {
                peer: peer.as_str().to_string(),
                reason,
            });
        }
        if &pin.fingerprint == observed {
            Ok(())
        } else if self
            .rotation_next
            .get(peer.as_str())
            .is_some_and(|next| next.value() == observed)
        {
            // Story 14-2 / AC2.2 (site 4 — async trait IMPL BODY; the six
            // trait SIGNATURES are untouched, per the house additive-mirror
            // convention): during an open rotation window the peer's `next`
            // is equally pinned. Sites 5/6 (`router.rs`) call this and pass
            // because of AC2.0's provisioning ordering — they need no edit.
            Ok(())
        } else {
            Err(EPinMismatch::Mismatch {
                peer: peer.as_str().to_string(),
                pinned: pin.fingerprint.wire(),
                observed: observed.wire(),
            })
        }
    }

    async fn invalidate_for_restart(
        &self,
        peer: &PeerId,
        prior_boot_nonce: u64,
    ) -> Result<(), A2AError> {
        let _guard = self.lock_window();
        self.rotation_next.remove(peer.as_str());
        let mut entry =
            self.pins
                .get_mut(peer.as_str())
                .ok_or_else(|| A2AError::PinInvalidated {
                    peer: peer.as_str().to_string(),
                    awaiting_repin: false,
                })?;
        entry.invalidated = Some(Invalidated::SpiritRestarted { prior_boot_nonce });
        drop(entry);
        self.persist_narrowed();
        Ok(())
    }

    /// Story 19-7 / D-19-7-05 A — consumes the verdict recorded by
    /// [`InMemoryTofuPinStore::record_repin_decision`]. No verdict, an
    /// expired or superseded request, or `new_boot_nonce` other than the
    /// pending observed nonce ⇒ `TimedOut` with nothing changed. A reject
    /// consumes the request and leaves the pin invalidated. An approval is
    /// consumed but refused (`RejectedByStore`, pin untouched) when the pin
    /// is absent, no longer `Invalidated::SpiritRestarted`, holds a
    /// fingerprint other than the request's `pinned_fingerprint`, or a
    /// rotation window was open at detection — a consent never resurrects a
    /// removed or manually invalidated pin.
    async fn await_repin_consent(
        &self,
        peer: &PeerId,
        new_observed: &PeerCertFingerprint,
        new_boot_nonce: u64,
    ) -> RePinDecision {
        let _guard = self.lock_window();
        let Some((_, mut pending)) = self.pending_repins.remove_if(peer.as_str(), |_, pending| {
            Instant::now() < pending.deadline
                && pending.request.observed_boot_nonce == new_boot_nonce
                && pending.verdict.is_some()
        }) else {
            return RePinDecision::TimedOut;
        };
        match pending.verdict.take() {
            None => RePinDecision::TimedOut,
            Some(RepinVerdict::Reject { reason }) => {
                // The removal is committed BEFORE the verdict is reported: a reboot must
                // not re-arm a request the operator was told is consumed.
                if let Err(failed) = self.commit_consumed(peer, pending) {
                    return failed;
                }
                RePinDecision::RejectedByOperator { reason }
            }
            Some(RepinVerdict::Approve { approval_id }) => {
                let awaits_consent = self.pins.get(peer.as_str()).is_some_and(|pin| {
                    pin.fingerprint == pending.request.pinned_fingerprint
                        && matches!(pin.invalidated, Some(Invalidated::SpiritRestarted { .. }))
                });
                if pending.rotating || !awaits_consent {
                    if let Err(failed) = self.commit_consumed(peer, pending) {
                        return failed;
                    }
                    return RePinDecision::RejectedByStore {
                        error: EPinMismatch::Invalidated {
                            peer: peer.as_str().to_string(),
                            reason: "re-pin refused: pin changed or rotating at detection".into(),
                        },
                    };
                }
                if let Some(colliding_peer) = self.colliding_peer(peer, new_observed) {
                    if let Err(failed) = self.commit_consumed(peer, pending) {
                        return failed;
                    }
                    return RePinDecision::RejectedByStore {
                        error: EPinMismatch::FingerprintCollision {
                            peer: peer.as_str().to_string(),
                            colliding_peer,
                            fingerprint: new_observed.wire(),
                        },
                    };
                }
                let new_pin = TofuPin {
                    peer: peer.clone(),
                    fingerprint: new_observed.clone(),
                    boot_nonce: new_boot_nonce,
                    pinned_at_ns: now_ns(),
                    invalidated: None,
                    repin_approval_id: Some(approval_id),
                };
                let mut record = self.record_of(new_pin.clone());
                let changed = &pending.request.pinned_fingerprint != new_observed;
                record.generation += u64::from(changed);
                // A consented re-pin to a different fingerprint is a re-pair, not a promotion.
                record.promoted &= !changed;
                // Widening: persist FIRST. A refused write applies nothing and the
                // request stays pending, verdict dropped (D-21-3-04 A).
                if let Err(detail) = self.persist(Some(record.clone())) {
                    self.pending_repins
                        .insert(peer.as_str().to_string(), pending);
                    return RePinDecision::PersistFailed { detail };
                }
                self.meta.insert(
                    peer.as_str().to_string(),
                    PinMeta {
                        anchor: record.anchor,
                        generation: record.generation,
                        promoted: record.promoted,
                    },
                );
                self.rotation_next.remove(peer.as_str());
                self.pins.insert(peer.as_str().to_string(), new_pin);
                RePinDecision::AcceptedByOperator { approval_id }
            }
        }
    }

    async fn get_pin(&self, peer: &PeerId) -> Option<TofuPin> {
        self.pins.get(peer.as_str()).map(|r| r.value().clone())
    }

    /// Story 8.9 / AC6.3 (G5b) — truly-atomic override: the boot-nonce read and
    /// the invalidation happen under a single `DashMap` `get_mut` entry lock, so
    /// a concurrent intake racing the same peer cannot interleave between the
    /// check and the act (the TOCTOU the router's prior `get_pin` + separate
    /// `invalidate_for_restart` exposed). Story 19-7: the pending re-pin
    /// request is recorded, and the sink invoked, under the same
    /// `window_lock` acquisition; whether a rotation window was open is read
    /// before the window is cleared.
    async fn invalidate_if_boot_nonce_differs(
        &self,
        peer: &PeerId,
        observed_boot_nonce: u64,
    ) -> Result<Option<u64>, A2AError> {
        let _guard = self.lock_window();
        let Some(mut entry) = self.pins.get_mut(peer.as_str()) else {
            return Ok(None);
        };
        if entry.boot_nonce == observed_boot_nonce {
            return Ok(None);
        }
        let prior = match entry.invalidated {
            Some(Invalidated::SpiritRestarted { prior_boot_nonce }) => prior_boot_nonce,
            Some(Invalidated::Manual) => return Ok(None),
            None => entry.boot_nonce,
        };
        let newly = entry.invalidated.is_none();
        if newly {
            entry.invalidated = Some(Invalidated::SpiritRestarted {
                prior_boot_nonce: prior,
            });
        }
        let pinned_fingerprint = entry.fingerprint.clone();
        drop(entry);
        let rotating = self.rotation_next.remove(peer.as_str()).is_some();
        let now = Instant::now();
        let (fresh, rotating) = match self.pending_repins.get(peer.as_str()) {
            Some(pending) => (
                pending.request.observed_boot_nonce != observed_boot_nonce
                    || now >= pending.deadline,
                rotating || pending.rotating,
            ),
            None => (true, rotating),
        };
        let detected = fresh.then(|| {
            let detected_at = SystemTime::now();
            let request = RepinRequest {
                peer: peer.clone(),
                pinned_fingerprint,
                prior_boot_nonce: prior,
                observed_boot_nonce,
                detected_at,
                expires_at: detected_at + self.repin_ttl,
                expired: false,
            };
            let pending = PendingRepin {
                request: request.clone(),
                deadline: now + self.repin_ttl,
                verdict: None,
                rotating,
            };
            self.pending_repins
                .insert(peer.as_str().to_string(), pending);
            request
        });
        if newly || detected.is_some() {
            self.persist_narrowed();
        }
        if let (Some(request), Some(sink)) = (detected, self.repin_sink.get()) {
            sink(&request);
        }
        Ok(Some(prior))
    }
}

/// Monotonic counter for diagnostic timestamps (NOT a TTL).
/// Uses a simple atomic increment — wall-time is not needed for the
/// pin staleness diagnostic timeline, and monotonicity (never going
/// backwards) is more important than correlation with real clocks.
fn now_ns() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(s: &str) -> PeerCertFingerprint {
        PeerCertFingerprint::from_cert_der(s.as_bytes())
    }

    #[tokio::test]
    async fn first_contact_mismatch_fires() {
        let store = InMemoryTofuPinStore::new();
        let peer = PeerId::new("p");
        let observed = fp("cert-A");
        let declared = fp("cert-B");
        let err = store
            .pin_first_contact(&peer, &observed, &declared, 1)
            .await
            .expect_err("must mismatch");
        assert!(matches!(err, EPinMismatch::Mismatch { .. }));
    }

    #[tokio::test]
    async fn verify_pinned_mismatch_fires() {
        let store = InMemoryTofuPinStore::new();
        let peer = PeerId::new("p");
        let observed = fp("cert-A");
        store
            .pin_first_contact(&peer, &observed, &observed, 1)
            .await
            .expect("pin");
        let other = fp("cert-B");
        let err = store
            .verify_pinned(&peer, &other)
            .await
            .expect_err("must mismatch");
        assert!(matches!(err, EPinMismatch::Mismatch { .. }));
    }

    #[tokio::test]
    async fn verify_pinned_returns_not_pinned_when_absent() {
        let store = InMemoryTofuPinStore::new();
        let peer = PeerId::new("p");
        let observed = fp("cert-A");
        let err = store
            .verify_pinned(&peer, &observed)
            .await
            .expect_err("must be not pinned");
        assert!(matches!(err, EPinMismatch::NotPinned(_)));
    }

    #[tokio::test]
    async fn invalidate_for_restart_marks_pin() {
        let store = InMemoryTofuPinStore::new();
        let peer = PeerId::new("p");
        let observed = fp("cert-A");
        store
            .pin_first_contact(&peer, &observed, &observed, 1)
            .await
            .expect("pin");
        store
            .invalidate_for_restart(&peer, 1)
            .await
            .expect("invalidate");
        let err = store
            .verify_pinned(&peer, &observed)
            .await
            .expect_err("must be invalidated post-invalidate");
        assert!(matches!(err, EPinMismatch::Invalidated { .. }));
    }

    /// Story 8.9 / AC6.3 (G5b) — concurrent intakes racing the same peer MUST
    /// NOT interleave between the boot-nonce read and the invalidation.
    /// 50× stress: the pin is invalidated exactly once; no task panics;
    /// all tasks that observed a mismatch can report the prior nonce.
    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn invalidate_if_boot_nonce_differs_is_atomic_under_race() {
        let store = Arc::new(InMemoryTofuPinStore::new());
        let peer = PeerId::new("p");
        let observed = fp("cert-A");
        store
            .pin_first_contact(&peer, &observed, &observed, 1)
            .await
            .expect("pin");

        let mut handles = vec![];
        for _ in 0..50 {
            let s = store.clone();
            let p = peer.clone();
            handles.push(tokio::spawn(async move {
                s.invalidate_if_boot_nonce_differs(&p, 2).await
            }));
        }
        let mut results: Vec<Result<Option<u64>, A2AError>> = vec![];
        for h in handles {
            results.push(h.await.expect("join"));
        }

        let ok_results: Vec<Option<u64>> = results.into_iter().map(|r| r.expect("ok")).collect();
        // Every task that saw the mismatch must be able to report the prior nonce.
        assert!(
            ok_results.iter().all(|r| r.is_some()),
            "all 50 tasks observed a mismatch and must report the prior nonce"
        );

        // The pin is invalidated exactly once (no TOCTOU race).
        let pin = store.get_pin(&peer).await.expect("pin exists");
        assert!(
            pin.invalidated.is_some(),
            "pin must be invalidated after the race"
        );
        assert_eq!(
            pin.boot_nonce, 1,
            "boot_nonce must be unchanged (the original value)"
        );
        if let Some(Invalidated::SpiritRestarted { prior_boot_nonce }) = pin.invalidated {
            assert_eq!(prior_boot_nonce, 1, "prior_boot_nonce must match original");
        } else {
            panic!("invalidation must be SpiritRestarted");
        }
    }

    /// §A6 close pass (F6) — the fail-closed half. The "window open, pin
    /// entry gone" state is unreachable through the public API (`open_rotation_window`
    /// requires a live pin; no public mutator removes a pin), so this unit
    /// test reaches it through the module-private maps — NOT through a public
    /// test-only API — and pins the contract: once the expected generation
    /// matches, the closer owns that window instance and must clear it even
    /// when it cannot promote. Leaving it would orphan a widened trust set no
    /// later close could reach.
    #[tokio::test]
    async fn close_rotation_window_if_clears_the_window_even_when_the_pin_has_vanished() {
        let store = InMemoryTofuPinStore::new();
        let peer = PeerId::new("p");
        let old = fp("cert-A");
        let next = fp("cert-B");
        store
            .pin_first_contact(&peer, &old, &old, 1)
            .await
            .expect("pin");
        store
            .open_rotation_window(&peer, &next)
            .expect("window opens");

        // Manufacture the promotion-target-gone state directly.
        store.pins.remove(peer.as_str());
        assert!(store.pins.get(peer.as_str()).is_none());

        let retired = store.close_rotation_window_if(&peer, &next);
        assert!(retired.is_none(), "nothing to promote, nothing retired");
        assert!(
            store.rotation_next(&peer).is_none(),
            "no window may survive a close whose promotion target is gone"
        );
        assert!(
            store.verify_pinned(&peer, &next).await.is_err(),
            "the incoming generation is not accepted by a store that never promoted it"
        );

        // Symmetry check: the identical manufactured state through the
        // UNCONDITIONAL close must produce the same fail-closed outcome.
        store
            .rotation_next
            .insert(peer.as_str().to_string(), next.clone());
        assert!(
            store.close_rotation_window(&peer).is_none(),
            "the unconditional close also retires nothing when the pin is gone"
        );
        assert_eq!(store.rotation_next(&peer), None);
    }
}
