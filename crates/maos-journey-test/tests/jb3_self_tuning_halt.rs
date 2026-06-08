#![forbid(unsafe_code)]

//! JB-3 (P0) — the self-tuning epistemic halt fires in PRODUCTION on
//! `belief_variance` (the 8.10·AC1 → 8.11 regression guard).
//!
//! **Authored by Story 8.11 to "the-right-RED" (John's pen/seal ruling):**
//! - It COMPILES against the frozen `maos_journey_test` import surface and the
//!   harness RUNS (RED-because-won't-compile is a disqualifier, not the bar).
//! - It is RED at **exactly one line** — the halt-screen assertion — for **one
//!   reason**: the 8.11 harness skeleton's `Pty::screen()` returns an empty
//!   screen, so the production halt render-string is absent until Story 8.15
//!   wires the real PTY/vt100 drive. Everything upstream is GREEN.
//! - It is **NOT `#[ignore]`** — it ships running-RED.
//! - It binds to the **shared constants** [`butler::SCALAR_TAG_BELIEF_VARIANCE`]
//!   and [`butler::halt_screen_line`] (AC5 f), reconciling the original sketch's
//!   erroneous `self.belief_variance` tag. Any drift between the production
//!   daemon and this test is now a COMPILE error, not a silent body edit.
//!
//! **8.11 does NOT bless JB-3.** The certifying revert-to-red (remove the daemon
//! halt wiring → JB-3 RED at the halt-screen assert → restore → GREEN) is signed
//! at Story 8.15 by a non-author reviewer — the only signature that certifies
//! JB-3's redness is load-bearing (the 8.1 self-certification trap is the seal's
//! whole reason for existing). 8.11 demonstrated single-reason redness with a
//! throwaway stub (hard-coding the render-string into `Pty::screen()` flips this
//! GREEN; removing it returns RED) and then deleted the stub so JB-3 ships RED.

use std::time::Duration;

use maos_journey_test::{world_llm, AuditDb, JourneyWorld, MockMcp, Pty, ReplayProvider, TestClock};

#[tokio::test(start_paused = true)]
async fn jb3_self_tunes_via_belief_variance_halt() {
    let world = JourneyWorld::builder()
        .clock(TestClock::tuesday_1pm())
        .mcp(
            "google-calendar",
            MockMcp::calendar("fixtures/butler/calendar.json"),
        )
        .llm(ReplayProvider::cassette("cassettes/butler/j_butler.json"))
        .audit(AuditDb::temp())
        .build();

    // The frozen PTY command surface JB-3 binds to (NOT a Rust constructor —
    // see the corrected 2026-06-08 framing): the daemon's production run surface.
    let pty = Pty::spawn("maos run butler --live --replay-llm", &world);

    // The Spirit computes its own uncertainty proxy above the 0.7 threshold.
    // Bind the SCALAR TAG to the shared constant (reconciles the sketch's
    // erroneous `self.belief_variance` → production `belief_variance`).
    world_llm(&world).queue_scalar(butler::SCALAR_TAG_BELIEF_VARIANCE, 0.78);
    tokio::time::advance(Duration::from_secs(13 * 60)).await;

    // The single load-bearing assertion: the production halt screen-string. The
    // expected string is the SHARED constant (compile-error on drift), NOT a
    // literal. RED in 8.11 (skeleton screen is empty); 8.15 wires the real PTY
    // drive and signs the revert-to-red seal.
    let screen = pty.screen();
    let expected = butler::halt_screen_line(butler::SCALAR_TAG_BELIEF_VARIANCE);
    assert!(
        screen.contains(&expected),
        "REGRESSION: production on_idle stored the assessment but never rendered \
         the halt screen-string {expected:?} (the 8.1 bug). Screen was: {:?}",
        screen.text()
    );
}
