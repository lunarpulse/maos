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

pub mod resolver;
pub mod output_markers;
pub mod termination;
pub use resolver::{MockHaltResolver, FailingHaltResolver, KernelHaltResolver};
pub use maos_domain::halt::{HaltResolver, ResolveError};
pub use output_markers::OutputMarkerRegistry;
pub use termination::terminate_spirit;

use std::sync::RwLock;
use std::collections::HashMap;
use maos_domain::halt::{
    HaltId, HaltJournal, HaltJournalError, HaltReceipt, HaltState, HaltContinuityError,
    InvokeHaltError, Resolution,
};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i4::ApprovalDecision;
use maos_domain::invariants::i10::{JournalEntry, LifecycleEvent};
use maos_domain::frame::EpistemicHaltPayload;
use crate::iac::transparency_log::{TransparencyLogAdapter, FrameKind};
use crate::journal::JournalAdapter;

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
        Resolution::ProvidedContext { text } => Some(format!("halt={}: provided_context: {text}", halt_id.as_str())),
        Resolution::AcceptedHalt => Some(format!("halt={}: accepted_halt", halt_id.as_str())),
        Resolution::AuthorizedOverride { operator_policy_ref } => {
            Some(format!("halt={}: authorized_override: operator_policy_ref={operator_policy_ref}", halt_id.as_str()))
        }
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

impl HaltJournal for TransparencyLogAdapter {
    fn journal_halt_resolution(
        &self,
        actor: &str,
        spirit_id: &str,
        halt_id: &HaltId,
        resolution: &Resolution,
    ) -> Result<(), HaltJournalError> {
        journal_halt_resolution(self, actor, spirit_id, halt_id, resolution)
    }
}

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
#[maos_attrs::i9_exempt(reason = "halt mechanism — per-process pending-resolution state for SINGLE-HALT-OWNER protocol; parallel to capability-token ledger, not pattern-learning")]
#[derive(Debug, Default)]
pub struct HaltRegistry {
    pending: RwLock<HashMap<HaltId, HaltState>>,
    /// Story 4.3 — per-halt metadata for resolution-side lookups.
    metadata: RwLock<HashMap<HaltId, PendingHaltMetadata>>,
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

impl HaltRegistry {
    pub fn new() -> Self {
        Self::default()
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
            return Err(InvokeHaltError::DuplicateHaltId(halt_id.as_str().to_string()));
        }
        map.insert(halt_id.clone(), state);
        drop(map);
        let mut meta = self.metadata.write().expect("HaltRegistry metadata lock poisoned");
        meta.insert(halt_id, metadata);
        Ok(())
    }

    /// Legacy insert — for callers that don't have metadata (v0.3-β compat).
    pub fn insert_pending(&self, halt_id: HaltId, state: HaltState) -> Result<(), InvokeHaltError> {
        let mut map = self.pending.write().expect("HaltRegistry lock poisoned");
        if map.contains_key(&halt_id) {
            return Err(InvokeHaltError::DuplicateHaltId(halt_id.as_str().to_string()));
        }
        map.insert(halt_id, state);
        Ok(())
    }

    /// Story 4.3 — Lookup pending-halt metadata for the
    /// `ProvidedContext` resolution arm.  Returns `None` if the
    /// halt_id was inserted via the legacy `insert_pending` path
    /// without metadata.
    pub fn lookup_pending_metadata(&self, halt_id: &HaltId) -> Option<PendingHaltMetadata> {
        let meta = self.metadata.read().expect("HaltRegistry metadata lock poisoned");
        meta.get(halt_id).cloned()
    }

    /// Lookup + atomic transition. Used by `KernelHaltResolver::resolve`
    /// to confirm the halt exists, transition the state. The entry stays
    /// in the map in its terminal state so double-resolve returns
    /// `AlreadyResolved`.
    pub fn resolve(&self, halt_id: &HaltId, terminal: HaltState) -> Result<HaltState, ResolveStateError> {
        let mut map = self.pending.write().expect("HaltRegistry lock poisoned");
        match map.get(halt_id) {
            Some(HaltState::PendingResolution) => {
                let prev = map.insert(halt_id.clone(), terminal).unwrap();
                drop(map);
                // Story 4.3 — clean up metadata once the halt reaches a
                // terminal state so the map doesn't grow unbounded.
                let mut meta = self.metadata.write().expect("HaltRegistry metadata lock poisoned");
                meta.remove(halt_id);
                Ok(prev)
            }
            Some(_) => Err(ResolveStateError::AlreadyTerminal(halt_id.as_str().to_string())),
            None => Err(ResolveStateError::NotPending(halt_id.as_str().to_string())),
        }
    }

    /// Read-only inspection — used by `validate_halt_set` (AC5) and
    /// by `maosctl halt-list` (Story 3.3 AC7 already wired).
    pub fn pending_halt_ids(&self) -> Vec<HaltId> {
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
        let mut meta = self.metadata.write().expect("HaltRegistry metadata lock poisoned");
        for (k, _) in &drained {
            meta.remove(k);
        }
        drained
    }

    /// Drain for a specific spirit. v0.3-β drains all;
    /// Story 5.3 refines with real spirit_pid filtering.
    pub fn drain_for_spirit(&self, _spirit_pid: u32) -> Vec<(HaltId, HaltState)> {
        self.drain_all()
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
        let meta = self.metadata.read().expect("HaltRegistry metadata lock poisoned");
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
    journal.append_transition(JournalEntry {
        timestamp: timestamp_ns,
        lifecycle_event: LifecycleEvent::Halt,
        spirit_id: spirit_id.to_string(),
        effective_sandbox_tier: None,
    });

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

    Ok(HaltReceipt::new(halt_id, timestamp_ns, spirit_pid, boot_nonce, frame_id))
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
    let accepted = successor_accepted_versions
        .ok_or(HaltContinuityError::MissingHaltProtocolCompatibility)?;
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
