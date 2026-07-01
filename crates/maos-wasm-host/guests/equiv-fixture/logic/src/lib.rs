#![forbid(unsafe_code)]

//! Shared fixture-Spirit logic for the Story 11.1b cross-form equivalence gate.
//!
//! This crate is the SINGLE source of behavior shared by:
//!
//! - the WASM `equiv-identity-spirit` / `equiv-divergent-spirit` /
//!   `equiv-cosmetic-spirit` components (compiled via `wit-bindgen` against
//!   `maos:spirit@1.0`), and
//! - the native `equiv-native-twin` subprocess (linked against
//!   `maos_wasm_host::codec` + `maos_domain::frame`).
//!
//! It is crypto-free (D9) and dependency-free (D11): the transforms operate on
//! the logical primitive fields that BOTH forms carry through the WIT↔domain
//! bridge — the invariant-bearing fields (`frame_id`, `timestamp_ns`,
//! `logical_clock`, `from`, `to`, `kind`, `payload`, `auto_marker`). The two
//! forms differ only in HOW they move these fields (WIT records vs domain
//! `IacFrame`); the transform logic is identical because it lives here.
//!
//! # Modes (the three behavioral variants the gate distinguishes)
//!
//! - [`FixtureMode::Identity`] — echo: every field preserved. The PASS case
//!   (WASM ≡ native).
//! - [`FixtureMode::DivergentLogicalClock`] — mutates exactly ONE invariant
//!   field (`logical_clock += 1`). The FAIL case: the gate must detect the
//!   cross-form divergence.
//! - [`FixtureMode::CosmeticDelay`] — adds wall-clock latency but preserves
//!   every invariant field. The cosmetic-control case: the gate must still
//!   PASS (latency is not an invariant).

/// Behavioral mode of a fixture Spirit.
///
/// See the crate-level docs for the gate semantics of each variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureMode {
    /// Echo the frame unchanged. The cross-form-equivalence PASS case.
    Identity,
    /// Flip exactly one invariant field: `logical_clock += 1`.
    ///
    /// The cross-form-equivalence FAIL case — a detectable divergence on an
    /// invariant-bearing field.
    DivergentLogicalClock,
    /// Add latency but change no invariant field. The cosmetic-control case:
    /// the gate must classify this as EQUIVALENT to [`Identity`](Self::Identity)
    /// (latency is observable but not an invariant).
    CosmeticDelay,
}

/// Transform the `logical_clock` field under `mode`.
///
/// [`FixtureMode::DivergentLogicalClock`] returns `clock + 1` (saturating);
/// every other mode returns `clock` unchanged. This is the single shared
/// divergence point the equivalence gate asserts on.
///
/// # Examples
///
/// ```
/// use equiv_fixture_logic::{transform_logical_clock, FixtureMode};
///
/// assert_eq!(transform_logical_clock(42, &FixtureMode::Identity), 42);
/// assert_eq!(transform_logical_clock(42, &FixtureMode::DivergentLogicalClock), 43);
/// assert_eq!(transform_logical_clock(42, &FixtureMode::CosmeticDelay), 42);
/// // saturating at the u64 ceiling, never wrapping:
/// assert_eq!(transform_logical_clock(u64::MAX, &FixtureMode::DivergentLogicalClock), u64::MAX);
/// ```
pub fn transform_logical_clock(clock: u64, mode: &FixtureMode) -> u64 {
    match mode {
        FixtureMode::DivergentLogicalClock => clock.saturating_add(1),
        _ => clock,
    }
}

/// Whether `mode` requests a cosmetic (non-invariant) delay.
///
/// Only [`FixtureMode::CosmeticDelay`] delays. [`FixtureMode::Identity`] and
/// [`FixtureMode::DivergentLogicalClock`] do not — they complete as fast as the
/// form allows.
///
/// # Examples
///
/// ```
/// use equiv_fixture_logic::{should_delay, FixtureMode};
///
/// assert!(!should_delay(&FixtureMode::Identity));
/// assert!(!should_delay(&FixtureMode::DivergentLogicalClock));
/// assert!(should_delay(&FixtureMode::CosmeticDelay));
/// ```
pub fn should_delay(mode: &FixtureMode) -> bool {
    matches!(mode, FixtureMode::CosmeticDelay)
}
