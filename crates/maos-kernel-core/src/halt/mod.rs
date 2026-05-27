#![forbid(unsafe_code)]

//! Halt protocol mechanism scaffold.
//!
//! Story 3.3 + 4.1 LANDED: the `HaltResolver` trait + `MockHaltResolver` +
//! resolution-receiving glue, plus `invoke_halt`, halt-receipt production,
//! `HaltState` lifecycle, I14 halt-continuity validation.
//!
//! See `crates/maos-director-surface/src/halt_ui.rs` for the
//! director-side resolution submission path.
//!
//! ## Architecture references
//! - ADR-019 (I14 halt continuity) — `validate_halt_set`
//! - ADR-022 (tagged-scalar slot) — referenced from `invoke_halt`
//! - §4.0.7 (kernel does NOT interpret tag semantics) — `invoke_halt` doc-comment

pub mod output_markers;
pub mod resolver;
pub mod termination;
pub use maos_domain::halt::{HaltResolver, ResolveError};
pub use output_markers::OutputMarkerRegistry;
pub use resolver::{FailingHaltResolver, KernelHaltResolver, MockHaltResolver};
pub use termination::terminate_spirit;

use crate::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use crate::journal::JournalAdapter;
use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::halt::{
    HaltContinuityError, HaltId, HaltJournal, HaltJournalError, HaltReceipt, HaltState,
    InvokeHaltError, Resolution,
};
use maos_domain::invariants::i10::{JournalEntry, LifecycleEntry, LifecycleEvent};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i4::ApprovalDecision;
use std::collections::HashMap;
use std::sync::RwLock;

/// Journal a halt resolution to the Approval Decision Log (Story 3.3, AC4).
///
/// Mirrors `crate::security::posture::journal_posture_shift` (Story 3.2
/// AC4) — one canonical surface for director-action audit rows. The
/// `actor` is the director identity (`"director"` at v0.3-β; Story 9.x
/// wires identity propagation from the control-plane session).
///
/// Returns `Err(AuditError::SqliteWriteFatal)` if the SQLite write
/// fails AT THE LOG ADAPTER LEVEL — note that `insert_frame_event`
/// PANICS on log write failure per I2 (`transparency_log.rs:296-307`)
/// but `insert_approval_decision` returns a `Result` per the existing
/// signature at `transparency_log.rs:322-345`. The caller (`halt_ui`)
/// MUST surface the error to the director rather than silently dropping.
pub fn journal_halt_resolution(
    log: &TransparencyLogAdapter,
    actor: &str,
    spirit_id: &str,
    halt_id: &HaltId,
    resolution: &Resolution,
) -> Result<(), HaltJournalError> {
    let reasoning = match resolution {
        Resolution::ProvidedContext { text } => Some(format!(
            "halt={}: provided_context: {text}",
            halt_id.as_str()
        )),
        Resolution::AcceptedHalt => Some(format!("halt={}: accepted_halt", halt_id.as_str())),
        Resolution::AuthorizedOverride {
            operator_policy_ref,
        } => Some(format!(
            "halt={}: authorized_override: operator_policy_ref={operator_policy_ref}",
            halt_id.as_str()
        )),
    };
    log.insert_approval_decision(ApprovalDecision {
        actor: actor.into(),
        target: spirit_id.into(),
        capability: "halt.resolve".into(),
        intent: resolution.kind_label().into(),
        decision: true,
        reasoning,
    })
    .map_err(|e| HaltJournalError::WriteFailed(e.to_string()))
}

// Story 6.5 — HaltJournal impl moved to maos-iac/src/adapter/transparency_log.rs
// per orphan rules (TransparencyLogAdapter is now in maos-iac).

// ----- Story 4.1 — kernel-side halt mechanism (additive) -----

/// Per-process pending-halt registry. Held in the composition root and
/// passed into both `invoke_halt` (the kernel-side invocation path) and
/// `KernelHaltResolver` (the director-surface resolution sink). One
/// authoritative source of truth for "is this halt_id still pending".
///
/// Capacity: unbounded HashMap — halts are O(per-Spirit-session) in
/// volume; the practical ceiling is the number of in-flight Spirits ×
/// per-Spirit halt-set size (typically < 100 entries). If this grows
/// unbounded in production, Story 5.x adds an eviction policy; v0.3-β
/// trusts the lifecycle to drain.
#[maos_attrs::i9_exempt(
    reason = "halt mechanism — per-process pending-resolution state for SINGLE-HALT-OWNER protocol; parallel to capability-token ledger, not pattern-learning"
)]
#[derive(Default)]
pub struct HaltRegistry {
    pending: RwLock<HashMap<HaltId, HaltState>>,
    /// Story 4.3 — per-halt metadata for resolution-side lookups.
    metadata: RwLock<HashMap<HaltId, PendingHaltMetadata>>,
    /// Story 4.5 — AC5 isolation hook for corpus runner observation.
    #[cfg(feature = "spirit_test")]
    isolation_hook: Option<
        std::sync::Arc<
            parking_lot::Mutex<dyn maos_spirit_sdk::spirit_test::IsolationHookPoint + Send>,
        >,
    >,
}

/// Story 4.3 — metadata stored alongside a pending halt so the
/// `ProvidedContext` resolution arm can recover the originating
/// `spirit_pid` and `spirit_id` without rescanning the Transparency Log.
#[doc = "Construct via `KernelHaltResolver::resolve`; struct literals bypass non-empty checks."]
#[derive(Debug, Clone, PartialEq)]
pub struct PendingHaltMetadata {
    pub spirit_pid: u32,
    pub spirit_id: String,
    pub payload: EpistemicHaltPayload,
    /// Timestamp when the halt was fired (ns since UNIX_EPOCH).
    pub fired_ns: u64,
}

impl std::fmt::Debug for HaltRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("HaltRegistry");
        d.field("pending", &self.pending)
            .field("metadata", &self.metadata);
        #[cfg(feature = "spirit_test")]
        {
            let hook_label = if self.isolation_hook.is_some() {
                "Some(<IsolationHookPoint>)"
            } else {
                "None"
            };
            d.field("isolation_hook", &hook_label);
        }
        d.finish()
    }
}

impl HaltRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Story 4.5 — attach an isolation hook for cross-Spirit corpus observation.
    #[cfg(feature = "spirit_test")]
    pub fn with_isolation_hook(
        mut self,
        hook: std::sync::Arc<
            parking_lot::Mutex<dyn maos_spirit_sdk::spirit_test::IsolationHookPoint + Send>,
        >,
    ) -> Self {
        self.isolation_hook = Some(hook);
        self
    }

    /// Story 4.5 — fire isolation hooks for cross-Spirit observation.
    #[cfg(feature = "spirit_test")]
    fn fire_isolation_hooks(
        &self,
        case_id: &str,
        _surface: &str,
        outcome: maos_spirit_sdk::spirit_test::IsolationHookOutcome,
    ) {
        if let Some(ref hook) = self.isolation_hook {
            let mut h = hook.lock();
            let _ = h.before_spirit_a_attempt(case_id);
            let attempt = maos_spirit_sdk::spirit_test::AttemptResult {
                hooks_fired_during_attempt: vec![case_id.into()],
                frames_emitted: 1,
            };
            let _ = h.after_spirit_a_attempt(case_id, &attempt);
            let _ = h.before_spirit_b_observe(case_id);
            let observation = maos_spirit_sdk::spirit_test::ObservationResult {
                hooks_fired_during_observation: vec![],
                frames_emitted: 0,
                leaked_bytes: None,
            };
            let _ = h.after_spirit_b_observe(case_id, &observation);
        }
    }

    /// Insert a fresh halt entering `PendingResolution`. Called by
    /// `invoke_halt` after the TL + Journal rows commit; idempotency on
    /// duplicate `halt_id` is `Err(InvokeHaltError::DuplicateHaltId)`.
    ///
    /// Story 4.3: also accepts metadata for resolution-side lookups.
    pub fn insert_pending_with_metadata(
        &self,
        halt_id: HaltId,
        state: HaltState,
        metadata: PendingHaltMetadata,
    ) -> Result<(), InvokeHaltError> {
        let mut map = self.pending.write().expect("HaltRegistry lock poisoned");
        if map.contains_key(&halt_id) {
            return Err(InvokeHaltError::DuplicateHaltId(
                halt_id.as_str().to_string(),
            ));
        }
        map.insert(halt_id.clone(), state);
        drop(map);
        let mut meta = self
            .metadata
            .write()
            .expect("HaltRegistry metadata lock poisoned");
        meta.insert(halt_id, metadata);
        Ok(())
    }

    /// Legacy insert — for callers that don't have metadata (v0.3-β compat).
    pub fn insert_pending(&self, halt_id: HaltId, state: HaltState) -> Result<(), InvokeHaltError> {
        let mut map = self.pending.write().expect("HaltRegistry lock poisoned");
        if map.contains_key(&halt_id) {
            return Err(InvokeHaltError::DuplicateHaltId(
                halt_id.as_str().to_string(),
            ));
        }
        map.insert(halt_id, state);
        Ok(())
    }

    /// Story 4.3 — Lookup pending-halt metadata for the
    /// `ProvidedContext` resolution arm.  Returns `None` if the
    /// halt_id was inserted via the legacy `insert_pending` path
    /// without metadata.
    pub fn lookup_pending_metadata(&self, halt_id: &HaltId) -> Option<PendingHaltMetadata> {
        let meta = self
            .metadata
            .read()
            .expect("HaltRegistry metadata lock poisoned");
        meta.get(halt_id).cloned()
    }

    /// Lookup + atomic transition. Used by `KernelHaltResolver::resolve`
    /// to confirm the halt exists, transition the state. The entry stays
    /// in the map in its terminal state so double-resolve returns
    /// `AlreadyResolved`.
    pub fn resolve(
        &self,
        halt_id: &HaltId,
        terminal: HaltState,
    ) -> Result<HaltState, ResolveStateError> {
        let mut map = self.pending.write().expect("HaltRegistry lock poisoned");
        match map.get(halt_id) {
            Some(HaltState::PendingResolution) => {
                let prev = map.insert(halt_id.clone(), terminal).unwrap();
                drop(map);
                // Story 4.3 — clean up metadata once the halt reaches a
                // terminal state so the map doesn't grow unbounded.
                let mut meta = self
                    .metadata
                    .write()
                    .expect("HaltRegistry metadata lock poisoned");
                meta.remove(halt_id);
                Ok(prev)
            }
            Some(_) => Err(ResolveStateError::AlreadyTerminal(
                halt_id.as_str().to_string(),
            )),
            None => Err(ResolveStateError::NotPending(halt_id.as_str().to_string())),
        }
    }

    /// Read-only inspection — used by `validate_halt_set` (AC5) and
    /// by `maosctl halt-list` (Story 3.3 AC7 already wired).
    pub fn pending_halt_ids(&self) -> Vec<HaltId> {
        #[cfg(feature = "spirit_test")]
        self.fire_isolation_hooks(
            "halt.pending_halt_ids:unknown",
            "HaltRegistry::pending_halt_ids",
            maos_spirit_sdk::spirit_test::IsolationHookOutcome::Continue,
        );
        let map = self.pending.read().expect("HaltRegistry lock poisoned");
        map.iter()
            .filter(|(_, s)| matches!(s, HaltState::PendingResolution))
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Atomically clear all entries for a given spirit_pid — used by
    /// the termination paths (planned `unload`, crash) to drain
    /// before emitting the HaltReceipt. spirit_pid match is by the
    /// halt_id convention (halt_ids carry the spirit context) —
    /// v0.3-β uses the `drain_for_spirit` on all halts for an in-flight
    /// spirit; Story 5.3 wires per-pid filtering.
    pub fn drain_all(&self) -> Vec<(HaltId, HaltState)> {
        let mut map = self.pending.write().expect("HaltRegistry lock poisoned");
        let pending_keys: Vec<HaltId> = map
            .iter()
            .filter(|(_, s)| matches!(s, HaltState::PendingResolution))
            .map(|(k, _)| k.clone())
            .collect();
        let mut drained = Vec::with_capacity(pending_keys.len());
        for k in &pending_keys {
            if let Some(v) = map.remove(k) {
                drained.push((k.clone(), v));
            }
        }
        drop(map);
        // Clean up metadata for drained halts.
        let mut meta = self
            .metadata
            .write()
            .expect("HaltRegistry metadata lock poisoned");
        for (k, _) in &drained {
            meta.remove(k);
        }
        drained
    }

    /// Drain pending halts that belong to a specific `spirit_pid`.
    ///
    /// Uses the metadata map (Story 4.3) to identify ownership.
    /// Halts inserted via the legacy `insert_pending` path (no metadata)
    /// are NOT matched by this filter — they remain in the registry.
    pub fn drain_for_spirit(&self, spirit_pid: u32) -> Vec<(HaltId, HaltState)> {
        let meta = self
            .metadata
            .read()
            .expect("HaltRegistry metadata lock poisoned");
        let owned: Vec<HaltId> = meta
            .iter()
            .filter(|(_, m)| m.spirit_pid == spirit_pid)
            .map(|(id, _)| id.clone())
            .collect();
        drop(meta);
        let mut map = self.pending.write().expect("HaltRegistry lock poisoned");
        // Transition to terminal before drain so concurrent resolve() sees AlreadyTerminal
        for id in &owned {
            if let Some(state) = map.get_mut(id) {
                if matches!(state, HaltState::PendingResolution) {
                    *state = HaltState::Terminated;
                }
            }
        }
        let mut drained = Vec::with_capacity(owned.len());
        for id in &owned {
            if let Some(state) = map.remove(id) {
                drained.push((id.clone(), state));
            }
        }
        drop(map);
        let mut meta = self
            .metadata
            .write()
            .expect("HaltRegistry metadata lock poisoned");
        for (id, _) in &drained {
            meta.remove(id);
        }
        drained
    }

    /// Dry-run variant — returns the halts that *would* be drained for
    /// `spirit_pid` without mutating the registry.
    ///
    /// Used by `validate_swap_halt_continuity_dry_run` (Story 5.2 review
    /// patch closure) to preview drain impact before committing a swap.
    pub fn drain_for_spirit_dry_run(&self, spirit_pid: u32) -> Vec<(HaltId, HaltState)> {
        let meta = self
            .metadata
            .read()
            .expect("HaltRegistry metadata lock poisoned");
        let owned: Vec<HaltId> = meta
            .iter()
            .filter(|(_, m)| m.spirit_pid == spirit_pid)
            .map(|(id, _)| id.clone())
            .collect();
        drop(meta);
        let map = self.pending.read().expect("HaltRegistry lock poisoned");
        let mut preview = Vec::with_capacity(owned.len());
        for id in &owned {
            if let Some(state) = map.get(id) {
                preview.push((id.clone(), state.clone()));
            }
        }
        preview
    }

    /// Story 4.3 — Return halt metadata entries for the given `spirit_pid`
    /// whose `fired_ns` is >= `since_ns`.
    /// Includes both pending and terminal halts whose metadata is still
    /// resident.  Returns an empty vector if no metadata matches.
    pub fn halt_metadata_for_spirit(
        &self,
        spirit_pid: u32,
        since_ns: u64,
    ) -> Vec<(HaltId, PendingHaltMetadata)> {
        #[cfg(feature = "spirit_test")]
        self.fire_isolation_hooks(
            &format!("halt.metadata_for_spirit:{spirit_pid}"),
            "HaltRegistry::halt_metadata_for_spirit",
            maos_spirit_sdk::spirit_test::IsolationHookOutcome::Continue,
        );
        let meta = self
            .metadata
            .read()
            .expect("HaltRegistry metadata lock poisoned");
        meta.iter()
            .filter(|(_, m)| m.spirit_pid == spirit_pid && m.fired_ns >= since_ns)
            .map(|(id, m)| (id.clone(), m.clone()))
            .collect()
    }

    /// Story 4.3 — Read-only lookup of the current `HaltState` for a given
    /// `halt_id`. Returns `None` if the halt_id is not in the registry.
    pub fn lookup_state(&self, halt_id: &HaltId) -> Option<HaltState> {
        let map = self.pending.read().expect("HaltRegistry lock poisoned");
        map.get(halt_id).cloned()
    }
}

/// I14 enforcement entry-point — used by Story 5.2's Hot-Swap Coordinator
/// before initiating a swap.
///
/// Drain-OR-migrate semantics per ADR-019 + architecture §3.2 I14:
///   1. Snapshot predecessor's pending halt set.
///   2. Attempt drain via `drain_for_spirit(predecessor_spirit_pid)`.
///   3. After drain attempt, recompute the snapshot — if empty, swap is safe
///      (`Ok(SwapVerdict::SafeDrained { drained_count })`).
///   4. If drain failed, fall through to schema-compatible migration check
///      via `validate_halt_set`.
///   5. If `validate_halt_set` returns `Ok(())`, swap proceeds as
///      `SwapVerdict::SafeMigrated { ... }`.
///   6. Otherwise propagate `HaltContinuityError::*`.
pub fn validate_swap_halt_continuity(
    registry: &HaltRegistry,
    predecessor_spirit_pid: u32,
    predecessor_halt_protocol_version: u32,
    successor_accepted_versions: Option<&[u32]>,
) -> Result<SwapVerdict, HaltContinuityError> {
    // 1. Snapshot BEFORE drain
    let before_count = registry.pending_halt_ids().len();

    // 2. Attempt drain for predecessor
    let _drained = registry.drain_for_spirit(predecessor_spirit_pid);

    // 3. Snapshot AFTER drain
    let after_count = registry.pending_halt_ids().len();
    let drained_count = before_count.saturating_sub(after_count);

    // 4. If nothing left pending, swap is safe
    if after_count == 0 {
        return Ok(SwapVerdict::SafeDrained { drained_count });
    }

    // 5. Fall through to migration check
    let remaining = registry.pending_halt_ids();
    validate_halt_set(
        &remaining,
        predecessor_halt_protocol_version,
        successor_accepted_versions,
    )?;

    Ok(SwapVerdict::SafeMigrated {
        migrated_count: remaining.len(),
        predecessor_version: predecessor_halt_protocol_version,
        successor_versions: successor_accepted_versions.unwrap_or(&[]).to_vec(),
    })
}

/// Verdict from `validate_swap_halt_continuity`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SwapVerdict {
    /// All predecessor halts drained before swap; swap is safe regardless of schema.
    SafeDrained { drained_count: usize },
    /// Halts migrated; schema compatibility was verified.
    SafeMigrated {
        migrated_count: usize,
        predecessor_version: u32,
        successor_versions: Vec<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResolveStateError {
    #[error("halt_id {0} not found in pending registry")]
    NotPending(String),
    #[error("halt_id {0} already in terminal state")]
    AlreadyTerminal(String),
}

/// The halt-invocation primitive — the SINGLE owner of: TL row +
/// Lifecycle Journal entry + pending-registry insert + HaltReceipt
/// production. Spirits call this from their `[epistemic_policy]`
/// predicate-firing handlers (Story 4.2 wires that path).
///
/// Atomicity: TL row commits BEFORE Lifecycle Journal entry BEFORE
/// registry insert. If any step fails the function returns
/// `Err(InvokeHaltError::*)` and `HaltReceipt` is NOT produced — the
/// Spirit decides whether to retry or escalate.
///
/// Returns the `HaltReceipt` carrying `(halt_id, timestamp_ns,
/// spirit_pid, boot_nonce, frame_id)` — proof the halt entered the audit chain.
///
/// ## Architecture: §4.0.7
/// The kernel is the receiver of the payload; tag semantics belong to Spirit.
/// The kernel does NOT interpret `tag`, `value`, `threshold` beyond
/// what is needed for the TL row + journal entry.
pub fn invoke_halt(
    tl: &TransparencyLogAdapter,
    journal: &JournalAdapter,
    registry: &HaltRegistry,
    payload: maos_domain::frame::EpistemicHaltPayload,
    spirit_pid: u32,
    spirit_id: &str,
    boot_nonce: u64,
) -> Result<HaltReceipt, InvokeHaltError> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let halt_id = maos_domain::halt::HaltId::new(payload.halt_id.clone())
        .map_err(|_| InvokeHaltError::RegistryInsertFailed("invalid halt_id".into()))?;

    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    // Step 1: Write Transparency Log row (FrameKind::EpistemicHalt = 3).
    // I2 guarantees: insert_frame_event panics on write failure, so the
    // audit chain is never incomplete at this point.
    let payload_bytes = serde_json::to_vec(&payload)
        .map_err(|e| InvokeHaltError::TransparencyLogWriteFailed(e.to_string()))?;
    tl.insert_frame_event(
        FrameKind::EpistemicHalt,
        spirit_pid,
        None,
        payload.tag.as_str(),
        &payload_bytes,
        FrameOrigin::SpiritAuto,
    );
    let frame_id = tl.last_frame_id();

    // Step 2: Write Lifecycle Journal entry (LifecycleEvent::Halt = 6)
    journal.append_transition(JournalEntry::Lifecycle(LifecycleEntry {
        timestamp: timestamp_ns,
        lifecycle_event: LifecycleEvent::Halt,
        spirit_id: spirit_id.to_string(),
        payload: None,
        effective_sandbox_tier: None,
    }));

    // Step 3: Insert into pending registry with metadata (Story 4.3).
    registry.insert_pending_with_metadata(
        halt_id.clone(),
        HaltState::PendingResolution,
        PendingHaltMetadata {
            spirit_pid,
            spirit_id: spirit_id.to_string(),
            payload: payload.clone(),
            fired_ns: timestamp_ns,
        },
    )?;

    Ok(HaltReceipt::new(
        halt_id,
        timestamp_ns,
        spirit_pid,
        boot_nonce,
        frame_id,
    ))
}

/// I14 enforcement — verify the successor manifest accepts the
/// predecessor's halt-protocol version before allowing a hot-swap that
/// would carry pending halts across. Returns `Ok(())` when:
///   (a) `predecessor_halt_set` is empty (no halts to migrate; swap is safe), OR
///   (b) `successor_accepted_versions` contains `predecessor_version`.
///
/// Returns `Err(HaltContinuityError::EHaltContinuityViolation { ... })`
/// when the predecessor has pending halts that the successor's manifest
/// does NOT declare compatibility for. Returns
/// `Err(MissingHaltProtocolCompatibility)` when `successor_accepted_versions`
/// is `None` (manifest missing the field entirely).
///
/// **Story 5.2 owns the end-to-end integration** that calls this from
/// the Hot-Swap Coordinator with a real `spirit_manifest`; Story 4.1
/// only ships the typed-error path + unit test.
///
/// ## Architecture references
/// - ADR-019 (I14) — `architecture-maos-minimal-opus/12-architecture-decision-records.md`
/// - I14 marker — `crates/maos-domain/src/invariants/i14.rs`
pub fn validate_halt_set(
    predecessor_halt_set: &[HaltId],
    predecessor_version: u32,
    successor_accepted_versions: Option<&[u32]>,
) -> Result<(), HaltContinuityError> {
    if predecessor_halt_set.is_empty() {
        return Ok(());
    }
    let accepted =
        successor_accepted_versions.ok_or(HaltContinuityError::MissingHaltProtocolCompatibility)?;
    if accepted.contains(&predecessor_version) {
        Ok(())
    } else {
        Err(HaltContinuityError::EHaltContinuityViolation {
            predecessor: predecessor_version,
            successor: *accepted.iter().max().unwrap_or(&0),
            orphan_count: predecessor_halt_set.len(),
        })
    }
}

#[cfg(test)]
mod swap_continuity_tests {
    // v0.3-β limitation: these tests use `insert_pending` (direct registry
    // mutation) instead of `invoke_halt` (the production path through
    // TransparencyLogAdapter + JournalAdapter) because `invoke_halt` requires
    // a full TL journal setup that is disproportionate for unit-level
    // drain/migrate verdict testing.  The v0.3-β hot-swap integration test
    // (`hot_swap_halt_continuity_corpus_integration.rs`) uses `invoke_halt`
    // for the end-to-end corpus-driven path per spec AC3.
    use super::*;
    use maos_domain::halt::HaltId;

    fn make_registry() -> HaltRegistry {
        HaltRegistry::new()
    }

    #[test]
    fn empty_predecessor_returns_safe_drained_zero() {
        let registry = make_registry();
        let verdict = validate_swap_halt_continuity(&registry, 1, 1, Some(&[1, 2])).unwrap();
        assert_eq!(verdict, SwapVerdict::SafeDrained { drained_count: 0 });
    }

    #[test]
    fn drain_completes_returns_safe_drained() {
        let registry = make_registry();
        // Insert a pending halt WITH metadata (Story 5.3 per-PID filter)
        let hid = HaltId::new("test-halt-001").unwrap();
        registry
            .insert_pending_with_metadata(
                hid.clone(),
                HaltState::PendingResolution,
                PendingHaltMetadata {
                    spirit_pid: 1,
                    spirit_id: "spirit-001".into(),
                    payload: maos_domain::frame::EpistemicHaltPayload {
                        halt_id: "test-halt-001".into(),
                        tag: "test".into(),
                        value: 0.0,
                        threshold: None,
                        policy_id: "".into(),
                        derived_from: "".into(),
                    },
                    fired_ns: 0,
                },
            )
            .unwrap();
        assert_eq!(registry.pending_halt_ids().len(), 1);

        let verdict = validate_swap_halt_continuity(&registry, 1, 1, Some(&[1])).unwrap();
        // per-PID drain clears the halt for spirit_pid=1
        assert_eq!(verdict, SwapVerdict::SafeDrained { drained_count: 1 });
    }

    #[test]
    fn drain_fails_migrate_succeeds() {
        let registry = make_registry();
        let hid = HaltId::new("test-halt-002").unwrap();
        registry.insert_pending(hid.clone(), HaltState::PendingResolution);
        // Don't drain — simulate scenario where drain doesn't clear everything.
        // Since v0.3-β drain_for_spirit drains ALL, this test uses a pre-seeded
        // registry with a pending halt that we manually check migrate path.
        let remaining = registry.pending_halt_ids();
        assert!(!remaining.is_empty());

        let result = validate_halt_set(&remaining, 1, Some(&[1]));
        assert!(
            result.is_ok(),
            "migration should succeed for matching version"
        );
    }

    #[test]
    fn drain_fails_migrate_rejects() {
        let registry = make_registry();
        let hid = HaltId::new("test-halt-003").unwrap();
        registry.insert_pending(hid.clone(), HaltState::PendingResolution);
        let remaining = registry.pending_halt_ids();

        let result = validate_halt_set(&remaining, 1, Some(&[2]));
        assert!(
            result.is_err(),
            "migration should reject for mismatched version"
        );
    }

    #[test]
    fn missing_halt_protocol_compatibility_via_validate_halt_set() {
        // drain_for_spirit drains all at v0.3-β, so test validate_halt_set
        // directly for the migration reject path.
        let hid = HaltId::new("test-halt-004").unwrap();
        let result = validate_halt_set(&[hid], 1, None);
        assert!(result.is_err());
        match result.unwrap_err() {
            HaltContinuityError::MissingHaltProtocolCompatibility => {}
            e => panic!("expected MissingHaltProtocolCompatibility, got {e:?}"),
        }
    }

    #[test]
    fn empty_successor_accepted_versions_returns_violation_via_validate_halt_set() {
        let hid = HaltId::new("test-halt-005").unwrap();
        let result = validate_halt_set(&[hid], 1, Some(&[]));
        assert!(result.is_err());
        match result.unwrap_err() {
            HaltContinuityError::EHaltContinuityViolation { .. } => {}
            e => panic!("expected EHaltContinuityViolation, got {e:?}"),
        }
    }

    #[test]
    fn drain_fails_then_migrate_rejects_via_validate_halt_set() {
        let hid = HaltId::new("test-halt-006").unwrap();
        let result = validate_halt_set(&[hid], 1, Some(&[2]));
        assert!(
            result.is_err(),
            "migration should reject for mismatched version"
        );
    }

    // ---- Story 5.3 — drain_for_spirit per-PID filter tests ----

    fn make_meta(spirit_pid: u32, spirit_id: &str, halt_id: &str) -> PendingHaltMetadata {
        PendingHaltMetadata {
            spirit_pid,
            spirit_id: spirit_id.into(),
            payload: maos_domain::frame::EpistemicHaltPayload {
                halt_id: halt_id.into(),
                tag: "test".into(),
                value: 0.0,
                threshold: None,
                policy_id: "".into(),
                derived_from: "".into(),
            },
            fired_ns: 0,
        }
    }

    #[test]
    fn drain_for_spirit_empty_returns_empty() {
        let registry = make_registry();
        let drained = registry.drain_for_spirit(42);
        assert!(drained.is_empty());
    }

    #[test]
    fn drain_for_spirit_finds_only_owned_halts() {
        let registry = make_registry();
        let h1 = HaltId::new("owned-001").unwrap();
        let h2 = HaltId::new("other-001").unwrap();
        registry
            .insert_pending_with_metadata(
                h1.clone(),
                HaltState::PendingResolution,
                make_meta(1, "spirit-1", "owned-001"),
            )
            .unwrap();
        registry
            .insert_pending_with_metadata(
                h2.clone(),
                HaltState::PendingResolution,
                make_meta(2, "spirit-2", "other-001"),
            )
            .unwrap();

        let drained = registry.drain_for_spirit(1);
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].0, h1);
        // spirit-2's halt remains
        assert_eq!(registry.pending_halt_ids().len(), 1);
        assert!(registry.lookup_state(&h2).is_some());
    }

    #[test]
    fn drain_for_spirit_leaves_legacy_no_meta_untouched() {
        let registry = make_registry();
        let h1 = HaltId::new("legacy-001").unwrap();
        registry
            .insert_pending(h1.clone(), HaltState::PendingResolution)
            .unwrap();

        let drained = registry.drain_for_spirit(1);
        assert!(drained.is_empty());
        assert_eq!(registry.pending_halt_ids().len(), 1);
    }

    #[test]
    fn drain_for_spirit_idempotent_second_call_empty() {
        let registry = make_registry();
        let h1 = HaltId::new("owned-002").unwrap();
        registry
            .insert_pending_with_metadata(
                h1.clone(),
                HaltState::PendingResolution,
                make_meta(1, "spirit-1", "owned-002"),
            )
            .unwrap();

        let first = registry.drain_for_spirit(1);
        assert_eq!(first.len(), 1);

        let second = registry.drain_for_spirit(1);
        assert!(second.is_empty());
    }

    #[test]
    fn drain_for_spirit_dry_run_does_not_mutate() {
        let registry = make_registry();
        let h1 = HaltId::new("dry-001").unwrap();
        registry
            .insert_pending_with_metadata(
                h1.clone(),
                HaltState::PendingResolution,
                make_meta(7, "spirit-7", "dry-001"),
            )
            .unwrap();

        let preview = registry.drain_for_spirit_dry_run(7);
        assert_eq!(preview.len(), 1);
        assert_eq!(preview[0].0, h1);

        // Registry untouched
        assert_eq!(registry.pending_halt_ids().len(), 1);
        assert!(registry.lookup_pending_metadata(&h1).is_some());
    }

    #[test]
    fn drain_for_spirit_multi_spirit_filtering_correct() {
        let registry = make_registry();
        for pid in 10..=14 {
            let hid = HaltId::new(&format!("multi-{pid}")).unwrap();
            registry
                .insert_pending_with_metadata(
                    hid,
                    HaltState::PendingResolution,
                    make_meta(pid, &format!("spirit-{pid}"), &format!("multi-{pid}")),
                )
                .unwrap();
        }

        let drained_12 = registry.drain_for_spirit(12);
        assert_eq!(drained_12.len(), 1);
        assert_eq!(drained_12[0].0, HaltId::new("multi-12").unwrap());

        let remaining: Vec<_> = registry.pending_halt_ids();
        assert_eq!(remaining.len(), 4);
        for pid in [10, 11, 13, 14] {
            assert!(registry
                .lookup_state(&HaltId::new(&format!("multi-{pid}")).unwrap())
                .is_some());
        }
    }
}
