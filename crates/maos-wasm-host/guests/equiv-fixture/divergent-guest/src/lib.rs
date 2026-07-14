//! `equiv-divergent-spirit` — WASM fixture for the Story 11.1b cross-form
//! equivalence gate (DIVERGENT mode).
//!
//! This guest mutates exactly ONE invariant-bearing field — `logical_clock` —
//! by delegating to [`equiv_fixture_logic`] with
//! [`FixtureMode::DivergentLogicalClock`] (`logical_clock += 1`). It is the
//! gate's FAIL case: the equivalence oracle MUST flag this form as divergent
//! from the identity form, because the divergence is on an invariant field
//! (not a cosmetic one).

wit_bindgen::generate!({
    path: "../../../../../wit/spirit.wit",
    world: "spirit",
});

use equiv_fixture_logic::{transform_logical_clock, FixtureMode};

/// Compile-time-fixed behavioral mode. The native twin selects its mode via
/// `--mode`; each WASM component hardcodes exactly one.
const MODE: FixtureMode = FixtureMode::DivergentLogicalClock;

struct DivergentSpirit;

impl Guest for DivergentSpirit {
    fn handle_frame(frame: IacFrame) -> Result<Vec<IacFrame>, Halt> {
        // The shared transform is the SINGLE divergence point — incrementing
        // `logical_clock` by exactly 1. No other field is touched, so the
        // gate can attribute any flagged divergence to this field alone.
        let mut out = frame;
        out.logical_clock = transform_logical_clock(out.logical_clock, &MODE);
        Ok(vec![out])
    }

    fn on_start() -> Result<(), Halt> {
        Ok(())
    }

    fn on_shutdown() {}
}

export!(DivergentSpirit);
