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
use std::sync::Arc;

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
    TimedOut,
}

/// TOFU pin store trait — the persistence-backed implementation lives in
/// `maos-persistence` (existing crate boundary). For v0.5 the in-memory
/// impl below is the default; the persistence-backed impl can ship in a
/// follow-up without changing the trait surface.
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

/// Default in-memory TOFU pin store. Per architecture I9 ("Empty kernel") the
/// production impl is persistence-backed at `maos-persistence`; this default
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
    /// Optional hook to drive `await_repin_consent` deterministically from
    /// tests; production path defers to the Approval Decision Log (which is
    /// out-of-band and not modeled in this crate's surface).
    test_repin_hook: Arc<dyn Fn(&PeerId, &PeerCertFingerprint, u64) -> RePinDecision + Send + Sync>,
}

impl Default for InMemoryTofuPinStore {
    fn default() -> Self {
        Self {
            pins: Arc::new(DashMap::new()),
            rotation_next: Arc::new(DashMap::new()),
            window_lock: Arc::new(std::sync::Mutex::new(())),
            test_repin_hook: Arc::new(|_, _, _| RePinDecision::TimedOut),
        }
    }
}

impl InMemoryTofuPinStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Install a re-pin hook for deterministic tests of NFR-Rel-6 AC4.
    pub fn with_repin_hook<F>(mut self, hook: F) -> Self
    where
        F: Fn(&PeerId, &PeerCertFingerprint, u64) -> RePinDecision + Send + Sync + 'static,
    {
        self.test_repin_hook = Arc::new(hook);
        self
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
        })?;
        self.rotation_next.remove(peer.as_str());
        Some(retired)
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
        self.pins.insert(peer.as_str().to_string(), pin.clone());
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
        Ok(())
    }

    async fn await_repin_consent(
        &self,
        peer: &PeerId,
        new_observed: &PeerCertFingerprint,
        new_boot_nonce: u64,
    ) -> RePinDecision {
        match (self.test_repin_hook)(peer, new_observed, new_boot_nonce) {
            RePinDecision::AcceptedByOperator { approval_id } => {
                let _guard = self.lock_window();
                if let Some(colliding_peer) = self.colliding_peer(peer, new_observed) {
                    return RePinDecision::RejectedByStore {
                        error: EPinMismatch::FingerprintCollision {
                            peer: peer.as_str().to_string(),
                            colliding_peer,
                            fingerprint: new_observed.wire(),
                        },
                    };
                }
                self.rotation_next.remove(peer.as_str());
                self.pins.insert(
                    peer.as_str().to_string(),
                    TofuPin {
                        peer: peer.clone(),
                        fingerprint: new_observed.clone(),
                        boot_nonce: new_boot_nonce,
                        pinned_at_ns: now_ns(),
                        invalidated: None,
                        repin_approval_id: Some(approval_id),
                    },
                );
                RePinDecision::AcceptedByOperator { approval_id }
            }
            decision => decision,
        }
    }

    async fn get_pin(&self, peer: &PeerId) -> Option<TofuPin> {
        self.pins.get(peer.as_str()).map(|r| r.value().clone())
    }

    /// Story 8.9 / AC6.3 (G5b) — truly-atomic override: the boot-nonce read and
    /// the invalidation happen under a single `DashMap` `get_mut` entry lock, so
    /// a concurrent intake racing the same peer cannot interleave between the
    /// check and the act (the TOCTOU the router's prior `get_pin` + separate
    /// `invalidate_for_restart` exposed).
    async fn invalidate_if_boot_nonce_differs(
        &self,
        peer: &PeerId,
        observed_boot_nonce: u64,
    ) -> Result<Option<u64>, A2AError> {
        let _guard = self.lock_window();
        if let Some(mut entry) = self.pins.get_mut(peer.as_str()) {
            let prior = entry.boot_nonce;
            if prior != observed_boot_nonce {
                if entry.invalidated.is_none() {
                    entry.invalidated = Some(Invalidated::SpiritRestarted {
                        prior_boot_nonce: prior,
                    });
                    drop(entry);
                    self.rotation_next.remove(peer.as_str());
                    return Ok(Some(prior));
                }
                if let Some(Invalidated::SpiritRestarted { prior_boot_nonce }) = entry.invalidated {
                    return Ok(Some(prior_boot_nonce));
                }
            }
        }
        Ok(None)
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

    #[tokio::test]
    async fn await_repin_consent_accept_path() {
        let approval_id = [42u8; 16];
        let new_boot = 5u64;
        let store = InMemoryTofuPinStore::new()
            .with_repin_hook(move |_, _, _| RePinDecision::AcceptedByOperator { approval_id });
        let peer = PeerId::new("p");
        let new_fp = fp("cert-B");
        let decision = store.await_repin_consent(&peer, &new_fp, new_boot).await;
        assert!(matches!(decision, RePinDecision::AcceptedByOperator { .. }));
        // Re-pin materialized — verify_pinned should now pass against new_fp.
        let pin = store.get_pin(&peer).await.expect("pin exists");
        assert_eq!(pin.boot_nonce, new_boot);
        store.verify_pinned(&peer, &new_fp).await.expect("verify");
    }

    #[tokio::test]
    async fn await_repin_consent_reject_path() {
        let store = InMemoryTofuPinStore::new().with_repin_hook(|_, _, _| {
            RePinDecision::RejectedByOperator {
                reason: "no thanks".into(),
            }
        });
        let peer = PeerId::new("p");
        let new_fp = fp("cert-B");
        let decision = store.await_repin_consent(&peer, &new_fp, 0).await;
        assert!(matches!(decision, RePinDecision::RejectedByOperator { .. }));
        // No pin materialized.
        let err = store
            .verify_pinned(&peer, &new_fp)
            .await
            .expect_err("must be not pinned");
        assert!(matches!(err, EPinMismatch::NotPinned(_)));
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
}
