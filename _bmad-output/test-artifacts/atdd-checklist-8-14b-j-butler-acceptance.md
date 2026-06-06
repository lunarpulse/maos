---
stepsCompleted: ['step-01-preflight-and-context', 'step-02-generation-mode', 'step-03-test-strategy', 'step-04-generate-tests', 'step-05-validate-and-complete']
lastStep: 'step-05-validate-and-complete'
lastSaved: '2026-06-06'
storyId: '8.14b'
storyKey: '8-14b-j-butler-acceptance'
storyFile: '_bmad-output/planning-artifacts/epics/epic-8-...miranash-v03-v15.md (Story 8.14b)'
atddChecklistPath: '_bmad-output/test-artifacts/atdd-checklist-8-14b-j-butler-acceptance.md'
generatedTestFiles:
  - 'crates/maos-journey-test/src/lib.rs            (harness — 8.11·AC5)'
  - 'crates/maos-journey-test/tests/journey_butler.rs (red-phase acceptance — 8.14b)'
  - 'crates/maos-journey-test/cassettes/butler/j_butler.json (replay cassette)'
  - 'crates/maos-journey-test/fixtures/butler/*.json (MCP seeds)'
inputDocuments:
  - 'PRD: _bmad-output/planning-artifacts/prd/user-journeys.md (Journey B — J-Butler)'
  - 'Epic 8 Story 8.14b + 8.11·AC5 + 8.10·AC1 (epic-8 md)'
  - 'Reuse: crates/maos-a2a-tcp/tests/h_guards.rs + support/ (H1–H6 determinism guards)'
  - 'knowledge: test-levels-framework.md, test-priorities-matrix.md, ci-burn-in.md, test-quality.md'
detectedStack: backend (Rust / Cargo)
testFramework: 'cargo test + nextest; tokio test-util; portable-pty; vt100; insta'
ciPlatform: 'nextest burn-in (repo uses GitHub Actions discipline.yml — tea config says gitlab-ci; reconcile at wire time)'
phase: RED (all acceptance tests intentionally failing until 8.10·AC1 + 8.11 + 8.14a + 8.14b land)
---

# ATDD Red-Phase Checklist — J-Butler Journey (Story 8.14b)

> **Purpose:** Failing acceptance tests + implementation checklist that, when green, prove a user can *watch Sandra's Butler journey work* end-to-end — anticipatory notification, real tool write, audited morning digest, self-tuning halt — in **< 2 s wall-clock**, hermetically.
>
> **Authored by:** Murat (Test Architect), 2026-06-06. Backend (Rust) profile.
> **Seam:** live = everything MAOS owns; virtualized = the 4 nondeterministic externalities (clock, model, SaaS, terminal).

---

## 1. Story Context & Acceptance Source

J-Butler ("Sandra reclaims dinner", PRD `user-journeys.md` Journey B) is reproducible only when the completion-delivery chain lands: **8.10·AC1 (production halt) → 8.11 (daemon + Inference Port + AC5 harness) → 8.14a (CLI/shell) → 8.14b (real MCP drivers)**. This checklist drives that chain from the journey backward.

| Journey beat (PRD) | Acceptance source | Enabling story |
|---|---|---|
| `on_idle` fires after idle window; anticipatory notification with `{pattern, confidence, evidence, options[]}` | 8.1 AC2 + 8.14b·AC2 | 8.11 (on_idle dispatch) + 8.14b (real calendar) |
| Notification carries 3 options; kernel rejects emit missing `output_shape` fields | J-Butler "Output_shape predicate" | existing (8.1) + 8.11 |
| User picks (a) → **real** Linear note written via MCP | 8.14b·AC1/AC2 | 8.14b |
| Every notification/response in Transparency Log | J-Butler "Transparency Log" | existing |
| Morning digest cites `source_log_ref` for all completions (I11) | 8.1 AC5 + 8.14b·AC2 | existing + 8.11 |
| **Self-tuning halt on `self.belief_variance`** (Resolution scene) | **8.10·AC1** | **8.10 (the regression — production `on_idle` must fire the halt)** |
| Posture-shift "be more cautious for the next hour" | J-Butler "Posture-shift command" | 8.14a (shell) |

---

## 2. Risk & Priority Matrix (risk_threshold = p1)

| ID | Scenario | Risk if broken | Pri | Level |
|---|---|---|---|---|
| JB-1 | on_idle renders anticipatory notification | Journey invisible — nothing to watch | **P0** | journey (E2E) |
| JB-2 | pick (a) → real Linear write + audit row | Core value (acting on the user's behalf) silently no-ops | **P0** | journey (E2E) |
| JB-3 | self-tuning halt fires in **production** on belief_variance | The exact 8.1 "done"-but-not regression; safety-of-self-tuning | **P0** | journey (E2E) |
| JB-4 | morning digest cites source_log_ref (I11) | Audit claim ("0 hallucination") unfounded on real data | **P1** | integration |
| JB-5 | output_shape predicate rejects malformed emit | Unstructured notifications reach the user | **P1** | integration |
| JB-6 | capability scope enforced (no write outside grant) | Confused-deputy at the MCP boundary | **P1** | integration |
| JB-7 | hermetic determinism (50× burn-in, virtual time only) | Becomes the next §A2 CI-flake debt | **P0** | meta/CI |
| JB-8 | posture-shift narrows subsequent capability prompts | Supervision knob non-functional | P2 | journey (E2E) |

**Coverage decision:** P0+P1 are red-phase mandatory (JB-1..7). JB-8 (P2) is scaffolded `#[ignore]` for the green sprint. Per "prefer lower levels": JB-4/5/6 are written at **integration** level against the real adapters (no PTY) so a failure localizes without screen-scraping; JB-1/2/3 require the **journey/E2E** PTY+vt100 level because "what the user sees" is the assertion.

---

## 3. Harness Architecture (8.11·AC5 deliverable)

New dev-only crate `crates/maos-journey-test/` (workspace member delta +1 — pin at dev time). It owns the reusable harness; per-journey tests live in its `tests/`.

```
crates/maos-journey-test/
├── src/lib.rs                 # JourneyWorld builder, Pty, vt100 helpers, ReplayProvider, MockMcp
├── cassettes/butler/j_butler.json     # recorded LLM completions (insta-managed)
├── fixtures/butler/{calendar,slack,belief_log}.json   # MCP + scalar seeds
└── tests/journey_butler.rs    # JB-1..JB-8 (this checklist's red tests)
```

**Reuse, don't reinvent:** import the H1–H6 guards from `crates/maos-a2a-tcp/tests/h_guards.rs` (single pinned clock, ephemeral port, readiness oneshot, `kill_on_drop`, injectable timeouts, teardown-leak). Lift them into `maos-journey-test::guards` so both suites share one determinism contract.

**Virtualized boundaries (the only 4):**
1. **Clock** → `#[tokio::test(start_paused = true)]` + `tokio::time::advance` (collapses 12-min idle, 15-hour overnight, budget timers).
2. **LLM** → `ReplayInferenceProvider`: impl the frozen Inference Port trait; returns cassette completions keyed by a stable prompt hash. `record` mode (Tier-2) writes the cassette; `replay` mode (Tier-1) reads it; mismatch ⇒ hard fail (cassette drift signal).
3. **External SaaS** → `MockMcp`: real MCP wire (reuse the 5.5c/5.5d server scaffold), seeded per scenario. Records inbound tool-calls for assertion (`writes()`).
4. **Terminal** → `Pty` (`portable-pty`) drives `maos run`; `vt100::Parser` renders the screen for `contains`/snapshot assertions.

---

## 4. RED-PHASE TEST SCAFFOLDS

> All tests below are intentionally **failing/ignored** until the implementation checklist (§6) is complete. They compile against the harness API once `maos-journey-test::src/lib.rs` exists with `todo!()` bodies; mark each `#[ignore = "RED: 8.14b not impl"]` and flip as stories land.

### 4a. Harness surface — `crates/maos-journey-test/src/lib.rs` (scaffold)

```rust
//! Journey-acceptance harness (Story 8.11·AC5). Hermetic "watch-it-work" rig:
//! real MAOS daemon under test; only clock/LLM/SaaS/terminal are virtualized.
use std::time::Duration;

pub mod guards { /* lift H1–H6 from maos-a2a-tcp/tests/h_guards.rs */ }

/// Single pinned wall-clock for the whole world (H2). All expiry/idle decisions read THIS.
pub struct TestClock(/* injected Arc<Clock> shared with the daemon */);
impl TestClock { pub fn tuesday_1pm() -> Self { todo!("H2 pinned clock seed") } }

/// Deterministic Inference Port impl. `replay` reads cassette; `record` (Tier-2) writes it.
pub struct ReplayProvider;
impl ReplayProvider {
    pub fn cassette(path: &str) -> Self { todo!("load JSON cassette keyed by prompt-hash") }
    /// Inject a Spirit-computed scalar the next on_idle should surface (JB-3).
    pub fn queue_scalar(&self, tag: &str, value: f64) { todo!() }
}
// impl maos_<inference_port_trait> for ReplayProvider { ... }  // frozen 1b.4 trait

/// Mock MCP server speaking the REAL MCP wire, seeded from a fixture.
pub struct MockMcp;
impl MockMcp {
    pub fn calendar(seed: &str) -> Self { todo!() }
    pub fn slack(seed: &str) -> Self { todo!() }
    pub fn writable(kind: &str) -> Self { todo!("e.g. linear, figma") }
    /// Inbound tool-calls observed (assertion oracle for JB-2).
    pub fn writes(&self) -> Vec<McpCall> { todo!() }
}
pub struct McpCall { pub tool: String, pub args: serde_json::Value }

/// Real temp SQLite Transparency Log (NOT mocked — this is under test).
pub struct AuditDb;
impl AuditDb {
    pub fn temp() -> Self { todo!("open a real TransparencyLogAdapter on a tempdir") }
    pub fn frames(&self) -> Vec<FrameRow> { todo!() }
}
pub struct FrameRow { pub kind: String, pub source_log_ref: Vec<String> }

/// PTY-driven view of `maos run` (portable-pty) + vt100 screen parsing.
pub struct Pty;
impl Pty {
    pub fn spawn(cmd: &str, world: &JourneyWorld) -> Self { todo!("portable-pty + env wiring") }
    pub fn send_line(&mut self, s: &str) { todo!() }
    /// Drain output until the screen is quiescent (no fixed sleep — H4 readiness style).
    pub fn screen(&mut self) -> Screen { todo!("vt100::Parser over drained bytes") }
    pub fn run(&mut self, cmd: &str) -> String { todo!("subcommand inside the shell, e.g. audit query") }
}
pub struct Screen(String);
impl Screen { pub fn contains(&self, needle: &str) -> bool { self.0.contains(needle) } }

pub struct JourneyWorld { /* clock, providers, mcp registry, audit */ }
pub struct JourneyWorldBuilder;
impl JourneyWorld {
    pub fn builder() -> JourneyWorldBuilder { todo!() }
}
impl JourneyWorldBuilder {
    pub fn clock(self, c: TestClock) -> Self { todo!() }
    pub fn mcp(self, name: &str, m: MockMcp) -> Self { todo!() }
    pub fn llm(self, p: ReplayProvider) -> Self { todo!() }
    pub fn audit(self, a: AuditDb) -> Self { todo!() }
    pub fn build(self) -> JourneyWorld { todo!() }
}
```

### 4b. Acceptance tests — `crates/maos-journey-test/tests/journey_butler.rs`

```rust
use maos_journey_test::*;
use std::time::Duration;

// ── JB-1 (P0): on_idle renders the anticipatory notification ──
#[tokio::test(start_paused = true)]
#[ignore = "RED: 8.11 on_idle + 8.14b calendar MCP not impl"]
async fn jb1_on_idle_renders_anticipatory_notification() {
    let world = JourneyWorld::builder()
        .clock(TestClock::tuesday_1pm())
        .mcp("google-calendar", MockMcp::calendar("fixtures/butler/calendar.json")) // conflict week
        .mcp("slack", MockMcp::slack("fixtures/butler/slack.json"))                 // unanswered partner msg
        .llm(ReplayProvider::cassette("cassettes/butler/j_butler.json"))
        .audit(AuditDb::temp())
        .build();
    let mut pty = Pty::spawn("maos run butler --live --replay-llm", &world);

    tokio::time::advance(Duration::from_secs(13 * 60)).await; // cross 12-min idle → on_idle

    let s = pty.screen();
    assert!(s.contains("pattern noticed"), "anticipatory notification not rendered");
    assert!(s.contains("(a)") && s.contains("(b)") && s.contains("(c)"), "3 options missing");
}

// ── JB-2 (P0): pick (a) → REAL Linear write + audit row ──
#[tokio::test(start_paused = true)]
#[ignore = "RED: 8.14b Linear MCP write not impl"]
async fn jb2_option_a_writes_real_linear_note_and_audits() {
    let world = JourneyWorld::builder()
        .clock(TestClock::tuesday_1pm())
        .mcp("google-calendar", MockMcp::calendar("fixtures/butler/calendar.json"))
        .mcp("slack", MockMcp::slack("fixtures/butler/slack.json"))
        .mcp("linear", MockMcp::writable("linear"))
        .llm(ReplayProvider::cassette("cassettes/butler/j_butler.json"))
        .audit(AuditDb::temp())
        .build();
    let mut pty = Pty::spawn("maos run butler --live --replay-llm", &world);
    tokio::time::advance(Duration::from_secs(13 * 60)).await;
    let _ = pty.screen();

    pty.send_line("a");
    tokio::time::advance(Duration::from_secs(1)).await;

    let linear = /* world.mcp("linear") */ unimplemented!("expose mcp handle");
    assert_eq!(linear.writes().len(), 1, "Butler did not write the real Linear note");
    assert_eq!(linear.writes()[0].tool, "linear.create_issue");
    // Real audit row exists (Transparency Log under test, not mocked)
    let audit = /* world.audit() */ unimplemented!();
    assert!(audit.frames().iter().any(|f| f.kind == "NotificationEmitted"));
}

// ── JB-3 (P0): self-tuning halt fires in PRODUCTION on belief_variance — the 8.10·AC1 regression ──
#[tokio::test(start_paused = true)]
#[ignore = "RED: 8.10·AC1 — production on_idle must write scalar + fire halt"]
async fn jb3_self_tunes_via_belief_variance_halt() {
    let world = JourneyWorld::builder()
        .clock(TestClock::tuesday_1pm())
        .mcp("google-calendar", MockMcp::calendar("fixtures/butler/calendar.json"))
        .llm(ReplayProvider::cassette("cassettes/butler/j_butler.json"))
        .audit(AuditDb::temp())
        .build();
    let mut pty = Pty::spawn("maos run butler --live --replay-llm", &world);

    // Spirit computes its own uncertainty proxy above the 0.7 threshold
    world_llm(&world).queue_scalar("self.belief_variance", 0.78);
    tokio::time::advance(Duration::from_secs(13 * 60)).await;

    let s = pty.screen();
    assert!(s.contains("halted on self.belief_variance"),
        "REGRESSION: production on_idle stored the assessment but never fired the halt (8.1 bug)");
}

// ── JB-4 (P1, integration): morning digest cites source_log_ref (I11) ──
#[tokio::test(start_paused = true)]
#[ignore = "RED: 8.14b real-MCP-seeded 24h window not impl"]
async fn jb4_morning_digest_cites_source_log_ref() {
    let world = /* ...as JB-2... */ unimplemented!();
    let mut pty = Pty::spawn("maos run butler --live --replay-llm", &world);
    tokio::time::advance(Duration::from_secs(13 * 60)).await;
    pty.send_line("a");
    tokio::time::advance(Duration::from_secs(15 * 60 * 60)).await; // jump to "morning"

    let digest = pty.run("maos audit query --since 24h");
    // every claimed completion resolves to a real Transparency Log frame
    assert!(digest.contains("source_log_ref"));
    assert!(!digest.contains("source_log_ref: []"), "I11 chain empty — uncited completion");
}

// ── JB-5 (P1, integration): output_shape predicate rejects malformed emit ──
#[tokio::test]
#[ignore = "RED: output_shape enforcement on live emit not wired to daemon"]
async fn jb5_output_shape_rejects_malformed_notification() {
    // Drive the Butler emit path directly with a notification missing `options[]`;
    // assert the kernel rejects it (EOutputShapeViolation), nothing reaches the screen.
    todo!("integration-level: call the emit seam, expect rejection");
}

// ── JB-6 (P1, integration): capability scope enforced at the MCP boundary ──
#[tokio::test]
#[ignore = "RED: 8.14b scope enforcement not impl"]
async fn jb6_capability_scope_denies_out_of_grant_write() {
    // Butler granted linear:write but NOT figma:write; attempt a figma write → denied + journaled.
    todo!("integration-level: assert ECapabilityDenied + audit row");
}

// ── JB-7 (P0, meta): hermetic determinism — no real clock/port/sleep ──
#[test]
fn jb7_harness_is_hermetic() {
    // Static guard mirroring AC-T13/H-guards: no SystemTime::now in expiry paths,
    // no hardcoded ports, no fixed sleeps in setup. Reuse maos-a2a-tcp h_guards.
    maos_journey_test::guards::assert_no_wallclock_or_fixed_sleep("tests/journey_butler.rs");
}

// ── JB-8 (P2): posture-shift narrows subsequent prompts ──
#[tokio::test(start_paused = true)]
#[ignore = "RED + P2: green-sprint scope"]
async fn jb8_posture_shift_narrows_capability_prompts() { todo!() }

fn world_llm(_w: &JourneyWorld) -> &ReplayProvider { unimplemented!("expose provider handle") }
```

### 4c. Cassette + fixtures (record/replay anchors)

`cassettes/butler/j_butler.json` (Tier-1 replay; Tier-2 re-records from a real `--live` run):
```json
{
  "schema": "maos.journey.cassette/v1",
  "recorded_against": "anthropic:claude-…, MAOS <git-sha>",
  "entries": [
    { "prompt_sha256": "<hash of the on_idle reasoning prompt>",
      "completion": "Sandra — pattern noticed: you've worked past 7 PM the last 3 Tuesdays…\n(a)…(b)…(c)…",
      "scalars": { "user.calendar_conflict.confidence": 0.85 } },
    { "prompt_sha256": "<hash of the belief-variance reflection prompt>",
      "completion": "My posterior over your preferred sensitivity is bimodal…",
      "scalars": { "self.belief_variance": 0.78 } }
  ]
}
```
`fixtures/butler/calendar.json` — Sandra's conflict week (recurring 7 PM dinner T/Th, 2 attended-late, 2 missed). `slack.json` — "Heads down" status + unanswered 4:15 partner message. `belief_log.json` — 14-day acceptance log (0.42 shallow-work / 0.81 deep-work) feeding the bimodal posterior.

---

## 5. Tier-2 Live Re-record (drift guard, off PR path)

Same tests, env `MAOS_JOURNEY_MODE=record --live` with real keys + real MCP, nightly. Re-writes cassettes, asserts journey invariants still hold. **Cassette-age gate:** CI fails if any cassette older than 14 days without a successful Tier-2 run. This is what keeps a green Tier-1 from masquerading as "the live journey works."

---

## 6. IMPLEMENTATION CHECKLIST (red → green)

Ordered along the shortest Butler path. Each item flips one or more JB-tests from `#[ignore]` to green.

**Harness (Story 8.11·AC5) — unblocks JB-1..JB-7 to compile**
- [ ] `crates/maos-journey-test` crate created (dev-only; pin workspace +1).
- [ ] Lift H1–H6 guards from `maos-a2a-tcp/tests/h_guards.rs` into `guards` (JB-7).
- [ ] `Pty` over `portable-pty`; `Screen` over `vt100::Parser`; `screen()` drains to quiescence (no fixed sleep).
- [ ] `ReplayProvider` impl of the frozen Inference Port trait (1b.4); cassette load + prompt-hash keying + `queue_scalar`.
- [ ] `MockMcp` over the 5.5c/5.5d MCP server scaffold; `writes()` recorder.
- [ ] `AuditDb::temp()` opens a real `TransparencyLogAdapter` on a tempdir.
- [ ] `JourneyWorld` builder wires all four into a `maos run` invocation + shared pinned clock.

**Production — Story 8.10·AC1 (P0, the regression) — flips JB-3**
- [ ] `spirits/butler/src/lib.rs:250-270` production `on_idle` WRITES the calendar-conflict scalar AND fires the halt via the policy path (today it only stores the assessment).

**Production — Story 8.11 (keystone) — flips JB-1, enables JB-4**
- [ ] `maos run butler` serving loop drives `on_idle`/intake against the (virtual) clock.
- [ ] Inference Port call path reaches the provider (ReplayProvider in test, real behind `--live`).
- [ ] `--replay-llm` / `--live` flag wiring; budget timer threaded (not the hard-coded 30s).

**Production — Story 8.14a (CLI/shell) — enables JB-1/JB-4 to be driven**
- [ ] `maos run` + kernel-rendered shell render the notification surface to stdout.
- [ ] `maos audit query --since 24h` subcommand (via Epic 9.1) returns digest with `source_log_ref`.
- [ ] `maos spirit add butler --scope … --posture …`.

**Production — Story 8.14b (real MCP) — flips JB-2, JB-4, JB-6**
- [ ] `maos-mcp` Calendar(read)/Slack(read+draft)/Linear(write)/Figma(read) drivers replace `butler/src/lib.rs:76-77` fixture-replay.
- [ ] Capability scope enforced at the driver boundary (JB-6).
- [ ] `output_shape` predicate enforced on the live emit path (JB-5).

**CI**
- [ ] nextest burn-in job: `--retries 0 --test-threads=8`, 50× loop on `-p maos-journey-test` (JB-7 gate). Repo CI is GitHub Actions `discipline.yml` (tea config says gitlab — reconcile).
- [ ] Tier-2 nightly `--live` re-record workflow (separate secrets, off PR path) + cassette-age gate.

---

## 7. Definition of Done & Handoff

- [ ] JB-1, JB-2, JB-3, JB-4, JB-5, JB-6, JB-7 green (P0+P1). JB-8 remains `#[ignore]` (P2).
- [ ] Whole `journey_butler.rs` suite < 2 s wall-clock; 50× burn-in 100% green.
- [ ] Tier-2 nightly recorded once; cassettes committed; age-gate active.
- [ ] **Coverage honesty note in the test module doc:** "Proves MAOS orchestration/audit/halt/budget/MCP/render are correct given recorded inputs. Does NOT prove LLM reasoning quality (→ eval corpora) or live-API non-drift (→ Tier-2)."
- **Handoff:** feed this checklist to `bmad-dev-story` for Story 8.14b, after predecessors 8.10·AC1 → 8.11 → 8.14a. The harness item (8.11·AC5) is the first dev task; JB-3 (the 8.10·AC1 regression) is the first production task and the cheapest high-value flip.

**Murat's risk call:** JB-3 is the single most important red test — it pins the exact "marked-applied-but-wasn't" regression so it can never silently reopen. Write it first; it fails today against the real code and that failure is the proof the gap is real.
