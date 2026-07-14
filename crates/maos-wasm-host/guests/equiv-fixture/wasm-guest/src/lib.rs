//! `equiv-identity-spirit` — WASM fixture for the Story 11.1b cross-form
//! equivalence gate (IDENTITY mode).
//!
//! Crypto-free (D9) identity guest compiled against `maos:spirit@1.0`: it
//! delegates every field transform to [`equiv_fixture_logic`] with
//! [`FixtureMode::Identity`], so `handle-frame` echoes the inbound frame
//! unchanged. This is the WASM half of the gate's PASS case — its native twin
//! (`equiv-native-twin --mode identity`) MUST produce a byte-identical
//! invariant footprint.

wit_bindgen::generate!({
    path: "../../../../../wit/spirit.wit",
    world: "spirit",
});

use equiv_fixture_logic::{transform_logical_clock, FixtureMode};

/// Compile-time-fixed behavioral mode. The native twin selects its mode via
/// `--mode`; each WASM component hardcodes exactly one.
const MODE: FixtureMode = FixtureMode::Identity;

struct IdentitySpirit;

impl Guest for IdentitySpirit {
    fn handle_frame(frame: IacFrame) -> Result<Vec<IacFrame>, Halt> {
        // Delegate to the SHARED logic so the WASM and native forms cannot
        // drift in how they transform invariant fields.
        let mut out = frame;
        out.logical_clock = transform_logical_clock(out.logical_clock, &MODE);
        Ok(vec![out])
    }

    fn on_start() -> Result<(), Halt> {
        Ok(())
    }

    fn on_shutdown() {}
}

export!(IdentitySpirit);
