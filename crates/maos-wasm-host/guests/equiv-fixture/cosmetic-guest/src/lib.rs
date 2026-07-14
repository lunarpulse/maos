//! `equiv-cosmetic-spirit` — WASM fixture for the Story 11.1b cross-form
//! equivalence gate (COSMETIC-DELAY mode).
//!
//! This guest adds latency but preserves EVERY invariant-bearing field — it
//! delegates to [`equiv_fixture_logic`] with [`FixtureMode::CosmeticDelay`],
//! which leaves `logical_clock` (and every other field) unchanged. It is the
//! gate's cosmetic-control case: the oracle MUST still classify this form as
//! EQUIVALENT to the identity form, because the only observable difference is
//! wall-clock latency, which is not an invariant.
//!
//! wasm32-unknown-unknown has no WASI clock/sleep import in this guest, so the
//! cosmetic latency is realized as a fuel-billing spin (the native twin sleeps
//! for the wall-clock equivalent). Both forms preserve all invariant fields.

wit_bindgen::generate!({
    path: "../../../../../wit/spirit.wit",
    world: "spirit",
});

use equiv_fixture_logic::{should_delay, transform_logical_clock, FixtureMode};

/// Compile-time-fixed behavioral mode. The native twin selects its mode via
/// `--mode`; each WASM component hardcodes exactly one.
const MODE: FixtureMode = FixtureMode::CosmeticDelay;

/// Bounded spin that consumes wasmtime fuel (and therefore wall-clock time)
/// without touching any frame field. `std::hint::black_box` prevents the
/// optimizer from eliding it.
///
/// Sized to stay well UNDER the spec's nominal `--fuel 1000000` budget: each
/// iteration bills ~10 fuel (measured: 100k iters exhausted 1M), so 10k iters
/// ≈ 100k fuel — ~10% of the nominal budget, leaving ample headroom for the
/// frame round-trip itself. The latency it adds is small (microseconds); the
/// real, human-observable cosmetic latency lives on the native twin's 5 ms
/// `thread::sleep`. Both forms preserve every invariant field, so the gate
/// classifies this form as EQUIVALENT to identity regardless of timing.
const COSMETIC_SPIN_ITERS: u64 = 10_000;

fn burn_cosmetic_latency() {
    let mut acc: u64 = 0;
    let bound = COSMETIC_SPIN_ITERS;
    while acc < bound {
        acc = std::hint::black_box(acc.wrapping_add(1));
    }
}

struct CosmeticSpirit;

impl Guest for CosmeticSpirit {
    fn handle_frame(frame: IacFrame) -> Result<Vec<IacFrame>, Halt> {
        if should_delay(&MODE) {
            burn_cosmetic_latency();
        }
        // Invariant fields are untouched — `transform_logical_clock` is a
        // no-op under `CosmeticDelay`. The call still routes through the
        // shared logic so the WASM and native forms cannot drift.
        let mut out = frame;
        out.logical_clock = transform_logical_clock(out.logical_clock, &MODE);
        Ok(vec![out])
    }

    fn on_start() -> Result<(), Halt> {
        Ok(())
    }

    fn on_shutdown() {}
}

export!(CosmeticSpirit);
