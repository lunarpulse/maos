//! Hot-Swap port trait per architecture §4.5 (ADR-017 / ADR-019 / ADR-036).
//!
//! Authored under ADR-041 (Epic 6 retro §A3 — 2026-05-28) as the third of
//! the three Phase-4 extraction port traits. Story 7.4 (pre-Story-7.5a)
//! fills the method surface against the existing `maos-kernel-core::hot_swap`
//! machinery (`coordinator`, `saga`, `state_codec`, `precheck`,
//! `post_swap_monitor`, `archive`, `migrator` — ~1,700 LOC measured
//! pre-extraction); the trait surface declared here is the v0.7 hexagonal
//! port that survives unchanged across the `maos-kernel-core` →
//! `maos-hot-swap` extraction boundary.
//!
//! Per ADR-010's sync-trait rule (see `crates/maos-domain/src/ports/mod.rs`
//! preamble), the methods are declared SYNCHRONOUS. Async behavior is
//! mediated at the adapter layer; the saga is driven by a `Pin<Box<dyn
//! Future>>` returning concrete `HotSwapOutcome` rather than `async fn` on
//! the trait itself.
//!
//! ## Method surface at v0.7
//!
//! The v0.7 surface is intentionally minimal — only the four invariant
//! checkpoints Story 7.4 must move to the new crate. Method bodies + the
//! full saga / migrator / state_codec wiring are Story 7.4 work; this stub
//! declares the contract that survives the extraction boundary.

use crate::halt::HaltId;
use crate::invariants::i10::JournalEntry;
use crate::invariants::i6::HotSwapState;

/// Hot-Swap Coordinator — saga orchestrator preserving I6 (token preservation)
/// and I14 (halt continuity) across Spirit version transitions.
///
/// Per §4.5: "The hot-swap saga drains a quiescence window, validates halt
/// continuity, snapshots state, decodes against the successor's manifest,
/// hands control to the successor, and rolls back on any invariant breach."
pub trait HotSwapPort {
    /// Class: supervision
    ///
    /// Read the current hot-swap state for a Spirit. `None` = no swap in
    /// flight. Used by Director surface + Story 7.5b 30-Min Gate
    /// telemetry to verify substrate quiescence before kicking off
    /// onboarding flows.
    fn current_swap_state(&self, spirit_pid: u32) -> Option<HotSwapState>;

    /// Class: supervision
    ///
    /// Append a hot-swap saga lifecycle transition to the kernel's per-Host
    /// journal. Mirrors `SpiritSchedulerPort::journal_lifecycle` but
    /// carries hot-swap-specific saga phases (`SnapshotTaken`,
    /// `SwapInFired`, `PostSwapMonitorRunning`, `AutoReverted`,
    /// `DiscardSuccessor`, `RollbackComplete`).
    fn journal_swap_phase(&self, entry: JournalEntry);

    /// Class: supervision
    ///
    /// I14 halt continuity validation — invoked at saga step 3 (pre-swap)
    /// and saga step 11 (post-swap-monitor baseline). Returns the set of
    /// `HaltId`s the receiver MUST observe to remain post-swap; an empty
    /// set is the only legal "halt-set-loss = false" outcome. Story 7.4
    /// preserves the validation logic from
    /// `crates/maos-kernel-core/src/hot_swap/precheck.rs::validate_swap_halt_continuity`.
    fn validate_swap_halt_continuity(&self, spirit_pid: u32) -> Vec<HaltId>;

    /// Class: supervision
    ///
    /// I6 token-preservation post-swap check — invoked after saga step 9
    /// (`on_swap_in` completes). Returns `true` if every cap-token held by
    /// the predecessor was inherited by the successor under the same
    /// `(Spirit-PID + boot-nonce + expiry)` triple. `false` triggers
    /// `DiscardSuccessor` saga compensation. Story 7.4 preserves the
    /// validation logic from
    /// `crates/maos-kernel-core/src/hot_swap/post_swap_monitor.rs`.
    fn validate_token_preservation(&self, spirit_pid: u32) -> bool;
}
