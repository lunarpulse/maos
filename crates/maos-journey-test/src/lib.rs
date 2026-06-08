#![forbid(unsafe_code)]

//! `maos-journey-test` — the MAOS journey-acceptance test **harness skeleton**
//! (Story 8.11 · AC5).
//!
//! This crate exposes the **frozen import surface** the journey regression tests
//! (JB-1..JB-8, the J-Butler / J-Researcher / J1 / J4 journeys) bind to. Story
//! 8.11 ships ONLY the API *shape* — most bodies are `todo!()` placeholders.
//! **Story 8.15 fills the bodies** (real `portable-pty` drive, `vt100` screen
//! rendering, cassette I/O), lands the Tier-1/Tier-2 suites, and — critically —
//! **signs the revert-to-red seal** that certifies each regression test is a
//! real guard (a non-author reviewer removes the daemon wiring → the test goes
//! RED → restore → GREEN).
//!
//! ## Why a standalone crate (Option A, party-mode 2026-06-08)
//! Hosting the harness here — NOT in `maos-bin/tests/` — freezes JB-3's
//! `use maos_journey_test::*;` header across the 8.11→8.15 hand-off and lets
//! `maos_journey_test::guards::` be importable cross-crate. Option B
//! (`maos-bin/tests/` then migrate) would edit JB-3's import header on the move
//! and cannot expose the `guards` module — destroying revert-to-red.
//!
//! ## What 8.11 makes real vs. stubs
//! - [`Screen::contains`] is REAL (a byte search) — it is the assertion surface.
//! - [`Pty::screen`] returns an **empty** [`Screen`] in 8.11 (the load-bearing
//!   stub): so JB-3 reaches its halt-screen assertion and finds the string
//!   ABSENT → ships RED at exactly that line (8.15's real PTY drive flips it
//!   GREEN, then the revert-to-red seal proves the redness was load-bearing).
//! - Everything 8.15 must genuinely implement (real PTY spawn, vt100 parsing,
//!   cassette replay, the `guards` source-scans) is `todo!()`.

use std::collections::BTreeMap;

/// The world a journey test drives: a pinned clock, mock MCP endpoints, a replay
/// LLM provider, and a temp audit DB. Construct via [`JourneyWorld::builder`].
pub struct JourneyWorld {
    _clock: TestClock,
    _mcp: BTreeMap<String, MockMcp>,
    llm: ReplayProvider,
    _audit: AuditDb,
}

impl JourneyWorld {
    pub fn builder() -> JourneyWorldBuilder {
        JourneyWorldBuilder::default()
    }
}

/// Fluent builder for [`JourneyWorld`] (the JB-* shared construction surface).
#[derive(Default)]
pub struct JourneyWorldBuilder {
    clock: Option<TestClock>,
    mcp: BTreeMap<String, MockMcp>,
    llm: Option<ReplayProvider>,
    audit: Option<AuditDb>,
}

impl JourneyWorldBuilder {
    pub fn clock(mut self, clock: TestClock) -> Self {
        self.clock = Some(clock);
        self
    }

    pub fn mcp(mut self, server: &str, mock: MockMcp) -> Self {
        self.mcp.insert(server.to_string(), mock);
        self
    }

    pub fn llm(mut self, provider: ReplayProvider) -> Self {
        self.llm = Some(provider);
        self
    }

    pub fn audit(mut self, audit: AuditDb) -> Self {
        self.audit = Some(audit);
        self
    }

    pub fn build(self) -> JourneyWorld {
        JourneyWorld {
            _clock: self.clock.unwrap_or_default(),
            _mcp: self.mcp,
            llm: self.llm.unwrap_or_default(),
            _audit: self.audit.unwrap_or_default(),
        }
    }
}

/// A pinned virtual clock (H2 guard: one T0 governs the whole world).
#[derive(Default)]
pub struct TestClock {
    _t0_min_of_day: u32,
}

impl TestClock {
    /// Tuesday 1:00pm — the J-Butler scenario T0.
    pub fn tuesday_1pm() -> Self {
        Self {
            _t0_min_of_day: 13 * 60,
        }
    }
}

/// A mock MCP endpoint seeded from a fixture file (real driver lands in 8.15).
#[derive(Default)]
pub struct MockMcp {
    _fixture: String,
}

impl MockMcp {
    pub fn calendar(fixture_path: &str) -> Self {
        Self {
            _fixture: fixture_path.to_string(),
        }
    }
}

/// A replay LLM provider keyed by a cassette file, with the ability to queue
/// computed scalars the Spirit's reasoning would emit (e.g. `belief_variance`).
#[derive(Default)]
pub struct ReplayProvider {
    _cassette: String,
}

impl ReplayProvider {
    pub fn cassette(path: &str) -> Self {
        Self {
            _cassette: path.to_string(),
        }
    }

    /// Queue a computed scalar the Spirit emits this turn (e.g.
    /// `(butler::SCALAR_TAG_BELIEF_VARIANCE, 0.78)`). Real cassette wiring is
    /// Story 8.15's; the 8.11 skeleton records nothing.
    pub fn queue_scalar(&self, _tag: &str, _value: f64) {
        // Story 8.15 — wire the queued scalar into the replay cassette so the
        // PTY-driven `maos run` consumes it. 8.11 skeleton: no-op.
    }
}

/// A temp-dir audit DB (real `TransparencyLogAdapter` on a tempdir in 8.15).
#[derive(Default)]
pub struct AuditDb {
    _temp: (),
}

impl AuditDb {
    pub fn temp() -> Self {
        Self { _temp: () }
    }
}

/// A pseudo-terminal driving a `maos run ...` subprocess against a [`JourneyWorld`].
/// Story 8.15 wires the real `portable-pty` spawn + `vt100` parser.
pub struct Pty {
    _command: String,
}

impl Pty {
    /// Spawn the given `maos run ...` command against the world's seams. The
    /// 8.11 skeleton records the command only; 8.15 wires the real PTY.
    pub fn spawn(command: &str, _world: &JourneyWorld) -> Self {
        Self {
            _command: command.to_string(),
        }
    }

    /// The current rendered screen. **8.11 skeleton: returns an EMPTY screen**
    /// (the load-bearing stub — see crate docs). 8.15 wires the real `vt100`
    /// render of the PTY output.
    pub fn screen(&self) -> Screen {
        Screen(String::new())
    }
}

/// A rendered terminal screen. [`Screen::contains`] is REAL (the assertion
/// surface); the rendering that fills it is Story 8.15's.
pub struct Screen(String);

impl Screen {
    pub fn contains(&self, needle: &str) -> bool {
        self.0.contains(needle)
    }

    /// The raw rendered text (for richer 8.15 assertions).
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// The world's replay LLM provider (the sketch's `world_llm` free-fn — bound
/// here so JB-3's `world_llm(&world).queue_scalar(...)` resolves).
pub fn world_llm(world: &JourneyWorld) -> &ReplayProvider {
    &world.llm
}

/// H1–H6 hermeticity guards (lifted surface for JB-7). Story 8.15 fills the
/// real source-scans (currently in `maos-a2a-tcp/tests/h_guards.rs`, which are
/// path-relative and not cross-crate importable).
pub mod guards {
    /// JB-7 — assert a test reads no wall-clock and uses no fixed `sleep`
    /// (determinism guard). Real implementation lands in Story 8.15.
    pub fn assert_no_wallclock_or_fixed_sleep(_test_source_path: &str) {
        todo!("Story 8.15 — source-scan for Instant::now/SystemTime::now/sleep")
    }
}
