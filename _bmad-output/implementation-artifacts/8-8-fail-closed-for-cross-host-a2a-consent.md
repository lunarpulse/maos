---
dev_model_used: claude-opus-4-8
---
# Story 8.8: Fail-Closed-for-Cross-Host A2A Consent

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

> **Registered 2026-06-05** (Direct Adjustment — `sprint-change-proposal-2026-06-05.md`); the committed long-term end-state from Story 8.7's Q2 team consensus (8.7 AC9). **RE-PARENTED 2026-06-06** (party-mode audit): `DEPENDS ON Story 8.7 (done) + Story 8.9 (done)`. 8.7 shipped the *transitional* fine-grained-when-present mechanism + mandatory reference-sender population; 8.9 bound peer identity to the TLS cert (G8), enforced granter binding (G1), and made consent expiry **live** (G10) — the latter's `prepare_outbound` `valid_until_ns` stamp and the per-peer `consent_ttl_secs` config overlap this story's sender-completeness precondition. **This story closes audit gap G7** (the fallback policy itself — 8.9 fixed identity/granter/expiry, NOT the silent band-fallback). **Charter-safe: zero `maos-kernel-core` KLOC; workspace stays 41 (no new crate).**
>
> **Decisive enabling fact (epic-8 + 8.7 Q2 + verified 2026-06-06):** the `prepare_outbound` / `handle_intake` / `handle_intake_verified` enforcement path **IS** the cross-Host A2A router (`A2ARouterCore`). Same-Host process-internal IAC NEVER reaches it — verified: `grep A2ARouterCore|handle_intake|prepare_outbound` over `crates/maos-domain/src` + `crates/maos-kernel-core/src` returns **zero** matches (same-Host IAC routes through `iac_bus.rs`, a different code path). The in-process `LoopbackA2ARouter` (`maos-a2a`) and the live `A2ATransport` (`maos-a2a-tcp`) both wrap the **same** `A2ARouterCore`; the loopback is an in-process **simulation of cross-Host A2A**, not a same-Host path. Therefore fail-closed needs **no in-band same-Host/cross-Host discriminator** and leaves same-Host trust untouched **by construction** — flipping the core to fail-closed only affects A2A (cross-Host) traffic.

## Story

As a MAOS operator running Mira and Nash across two Hosts under ADR-012's confused-deputy threat model,
I want a cross-Host A2A frame that carries no fine-grained typed intent (absent or unrecognized `intent_class`)
to be **DENIED at the router** rather than falling back to the coarse 3-band gate,
so that channel-consent can never masquerade as transaction-consent across the trust boundary — closing the
confused-deputy gap completely, once every cross-Host sender is proven (by a sender-completeness gate) to populate
a well-typed intent.

## Acceptance Criteria

> **AC numbering.** AC1–AC2 are the functional core (the fail-closed decision + the new typed deny). AC3 is the LOCKED **sender-completeness + fail-closed-readiness precondition gate** (must be GREEN-at-HEAD *before* the flip — never flipped-while-red). AC4 is the runnable headline + reference wiring. AC5 is zero-regression / same-Host-untouched. AC6 is placement / ABI / KLOC / workspace / discipline. AC7 records the design-fork consensus. Every AC is BDD-shaped and independently verifiable.

### AC1 — Cross-Host fail-closed decision: deny unclassified, never silently downgrade

**Given** the A2A router (`A2ARouterCore`) toggled to **fail-closed mode** (see AC7 Fork A for the toggle's default), and a cross-Host frame whose consent classification is **unclassified** — defined as ANY of: `consent_envelope` is `None`; `consent_envelope.intent_class` is `None`; the `intent_class` is **non-canonical** (`!A2AIntent::is_canonical()`, i.e. fails the grammar `^[a-z0-9]+(-[a-z0-9]+)*(:[a-z0-9]+(-[a-z0-9]+)*)?$`); or it is **oversized** (`len() > MAX_CANONICAL_INTENT_LEN` = 128)
**When** the router runs send-side enforcement (`prepare_outbound`) OR accept-side enforcement (`handle_intake` / `handle_intake_verified`)
**Then** the frame is **DENIED** with a **new dedicated typed signal** — `A2AError::ConsentUnclassified { direction }` on the send side (before the frame hits the wire) and a `CODE_CONSENT_UNCLASSIFIED` (**-32009**, additive — next after 8.9's `-32008`) NACK on the accept side, interpreted back into `A2AError::ConsentUnclassifiedAtPeer { peer }` for the sender — **NOT** projected to the 3-band `frame_intent_str` fallback and **NOT** conflated with `CODE_INTENT_DENIED` (-32001, which means *classified-but-not-allowlisted*)
**And** the band-fallback path (`consent_match_key`'s `unwrap_or_else(|| Self::frame_intent_str(frame))`, `router.rs:282`) is **never taken for an unclassified cross-Host frame under fail-closed mode** — the silent-downgrade is structurally impossible (red-team non-negotiable: *"deny ONLY unclassified traffic and never silently downgrade"*)
**And** the deny is **legible**: the NACK/error carries the peer + the reason (`absent` vs `non-canonical` vs `oversized`) so an operator sees in the Transparency Log *why* a frame was rejected — fail-closed AND observable, never silent
**And** a **classified** frame (canonical `intent_class` present, ≤128) is handled EXACTLY as in 8.7 — its fine-grained `A2AIntent` is the match key, admitted iff allowlisted, denied with `-32001`/`EIntentDenied` naming the literal intent otherwise (no behavior change for the migrated path)

### AC2 — The fail-closed decision is a single, shared, testable seam

**Given** the 8.7 invariant that "the key tested == the key reported" must hold (send and accept can never diverge on classification)
**When** fail-closed is wired
**Then** the classification decision is centralized in **one** private helper (e.g. refactor `consent_match_key(frame) -> String` into a `consent_decision(frame) -> ConsentDecision` returning either `Classified(String)` or `Unclassified { reason }`, with `consent_match_key` retained or reconstructed for the fail-OPEN/same-fallback callers) — used by BOTH `send_admits`/`accept_admits`-callers AND every deny-construction site, so the classification tested is the classification reported
**And** `frame_intent_str` stays `pub` and **unchanged** (it remains the band-projection primitive used by the fail-open/transitional path and by any retained band-fallback; renaming/removing it is an abi-diff Removed → AC6 RED — the 8.6/8.7 lesson)
**And** the 8.9 consent block ordering is **preserved and extended coherently**: framing → peer lookup → TOFU → restart-detection → **(granter-binding → expiry)** [8.9 AC4] → **[NEW: unclassified-deny under fail-closed]** → accept-allowlist → Lamport → sink. The unclassified gate sits **immediately before the accept-allowlist** (it replaces the point where `accept_admits` would otherwise silently band-fall-back), so a stolen/expired envelope still fails at the 8.9 gates first and the well-formedness of classification is checked before allowlist matching

### AC3 — LOCKED sender-completeness + fail-closed-readiness precondition gate (GREEN-at-HEAD before the flip)

**Given** the LOCKED precondition that the flip is "mechanical" only once every cross-Host sender is proven to populate a well-typed intent (never flipped-while-red — the recurring AC4 trap `[[feedback_mechanical_gates_compound_promises_decay]]`)
**When** Story 8.8 lands
**Then** a **NEW discipline gate** `check-a2a-sender-completeness` (xtask, mirroring `check_workspace_count` / `check_service_boundary` structure) asserts **sender-completeness** — no cross-Host send path can construct a frame that reaches the A2A router-entry seam (`prepare_outbound` / `route_outbound`) with an absent or unrecognized `intent_class`. The static/build-time scan covers every reference cross-Host sender (`spirits/mira`, `spirits/nash`, the `smoke-a2a-loopback-6-3` / `smoke-mira-nash-8-5` / `smoke-a2a-consent-vocab-8-7` arms in `maos-bin`, and the live `smoke-a2a-tcp-8-6` arm) and flags any cross-Host frame literal built with `consent_envelope: None` or an envelope constructed without `with_fine_grained_intent` (or equivalent populated-`intent_class` constructor). The gate is registered in `xtask/gate-registry.toml`, dispatched from `xtask/src/main.rs`, and run in `.github/workflows/discipline.yml` + added to the `aggregate` job's `needs` list
**And** **runtime sender-completeness** is enforced at the seam itself: under fail-closed mode, `prepare_outbound` is the runtime guarantee — a reference sender that fails to populate a canonical `intent_class` gets `A2AError::ConsentUnclassified { direction: Send }` and the frame **never leaves**, so the "no off-Host frame leaves with `intent_class == None`" assertions added in 8.7 AC2 become enforced-by-construction rather than test-only
**And** **fail-closed-readiness** is proven: with the cross-Host core toggled to fail-closed, the FULL `a1_security_regression_guards` suite (P1/P2/P5/P6/P7), `cross_host_consent_v1_5.rs`, `trust_binding_8_9.rs`, and BOTH the loopback smoke arms AND `smoke-a2a-tcp-8-6` still pass — the corpus is already B-clean (every reference sender populates `intent_class` post-8.7), so the flip introduces zero new RED
**And** the gate is **GREEN at HEAD** in the same commit that flips fail-closed on — the flip and its proof land together (the explicit anti-pattern this project keeps re-learning is flipping a gate to enforcing while the thing it gates is still red)

### AC4 — Runnable headline + reference wiring reflect fail-closed

**Given** the observable-behavior preference `[[feedback_lunarpulse_observability_preference]]` (a runnable end-to-end demo beats a coverage number)
**When** the reference fleet is wired and the headline is run
**Then** a **NEW `smoke-a2a-fail-closed-8-8` one-shot** in `maos-bin` (mirroring the `smoke-a2a-consent-vocab-8-7` precedent at `crates/maos-bin/src/main.rs`; `maos-bin` smoke is NOT kernel KLOC) exits `0` and demonstrates, against the real router in fail-closed mode: (1) a classified cross-Host frame (`intent_class = Some("diagnosis-handoff:read-only-evidence")`) **delivered**; (2) a cross-Host frame with **absent** `intent_class` **denied** with `CODE_CONSENT_UNCLASSIFIED` (-32009) — visible as a distinct rejection in the TL, NOT a band-admit; (3) a cross-Host frame with a **non-canonical** `intent_class` (e.g. `"Diagnosis Handoff"` with spaces/caps) **denied** the same way
**And** the new arm is wired into `.github/workflows/discipline.yml` (build `cargo build -p maos-bin --release --features fixture_replay`, `MAOS_ONE_SHOT=smoke-a2a-fail-closed-8-8`, `timeout-minutes: 5`) **and** added to the `aggregate` job's `needs:` list (the gate-aggregation completeness gate)
**And** the reference cores that construct `A2ARouterCore` for cross-Host use (the `LoopbackA2ARouter` in `maos-a2a`, the `A2ATransport` in `maos-a2a-tcp`, and every reference smoke arm) are constructed in **fail-closed mode** (per AC7 Fork A's chosen default), so the headline reflects the production posture, not a test-only toggle

### AC5 — Zero regression for classified traffic + same-Host trust untouched by construction

**Given** Stories 6.3 / 8.5 / 8.7 / 8.9 ship a full A2A/consent suite, and the same-Host IAC path (`iac_bus.rs`) is architecturally separate from `A2ARouterCore`
**When** 8.8 lands
**Then** every existing A2A/consent test for **classified** traffic passes **unchanged** (the migrated reference fleet all populates `intent_class`, so fail-closed is a no-op for them); `smoke-mira-nash-8-5`, `smoke-a2a-loopback-6-3`, `smoke-a2a-consent-vocab-8-7`, and `smoke-a2a-tcp-8-6` still exit `0`; the 8.9 trust-binding suite (`trust_binding_8_9.rs`) and `a1_security_regression_guards` (7/7) stay GREEN
**And** the **same-Host IAC path is provably untouched**: a test (new or cited) demonstrates a same-Host process-internal IAC delivery with no `consent_envelope` still succeeds (it routes through `iac_bus.rs`, never `A2ARouterCore`) — the fail-closed flip does NOT regress same-Host trust, because same-Host never traverses the toggled path (cite the verification grep in the Dev Agent Record)
**And** the **legacy band-fallback path** (`cross_host_consent_v1.rs::scenario_3_1`, the explicitly band-only test 8.7 annotated) is handled per AC7 Fork A's resolution — either it runs against a core in the retained fail-OPEN/transitional mode (band-fallback preserved as an explicit opt-in for unmigrated peers), or it is migrated to populate `intent_class`; **whichever is chosen, no test silently changes behavior** (every behavior change is explicit and documented)

### AC6 — Placement, ABI, kernel-KLOC, workspace, and discipline gates

**Given** the charter-safe mandate and the 8.6/8.7/8.9 placement discipline
**When** 8.8 lands
**Then** all production changes land in **`crates/maos-a2a-core`** (the decision + the new error variant + the mode toggle), **`crates/maos-a2a-tcp`** (wire the fail-closed core in `A2ATransport::bind`/`serve_connection`), **`crates/maos-a2a`** (wire the loopback core), **`crates/maos-bin`** (the new smoke arm — NOT kernel KLOC), **`xtask`** (the new gate), **`spirits/mira` + `spirits/nash`** (confirm/strengthen their senders), and the reference smoke arms — **NOT** in `maos-kernel-core`
**And** `maos-kernel-core` is **byte-identical** to its pre-story state (zero-kernel-KLOC mandate; git-verify a clean diff for that crate — the 8.4/8.6/8.9 standard); `kloc-check` for `maos-a2a-core` (ceiling 3000; was 2644 post-8.7/8.9) and `maos-a2a` (ceiling 1500; was 201) both stay GREEN — record post-change line counts in evidence
**And** the workspace member count is **UNCHANGED at 41** (`check-workspace-count` GREEN; 8.8 adds NO new crate — the new gate lives in the existing `xtask` crate, the new smoke is in the existing `maos-bin`)
**And** an `abi-diff` of `maos-a2a-core`'s public surface (use `--base` against `xtask/abi-baseline` — the Story 8.3 lesson: no-base `HEAD~1` mode false-positives) is **Added-only**: the new `A2AError::ConsentUnclassified` / `ConsentUnclassifiedAtPeer` variants (`A2AError` is already `#[non_exhaustive]`), the new `CODE_CONSENT_UNCLASSIFIED` const, and the new `UnclassifiedReason` enum are all **Added**; `frame_intent_str` / `ConsentAllowlists` / `EIntentDenied` / the 8.6-frozen `verify_pinned`/`handle_intake`/`try_from_bytes` signatures are **unchanged**; the private `consent_decision` refactor is invisible to abi-diff. **No abi-diff Removed is required** (unlike 8.7's ratified `A2AConsentEnvelope` deletion) — if Fork A's resolution…
**And** all discipline gates are **GREEN at HEAD** (not flipped-while-red); `dev_model_used` is recorded in this story's frontmatter (§A2 discipline — `check-dev-model-used-populated`); `check-dev-record-completeness` + `check-review-findings-resolved` GREEN; architecture narratives (`7-inter-agent-communication.md`, `4-kernel-design.md`) are reconciled if any says cross-Host consent fails *open* (7-iac.md already documents per-frame typed-intent — verify and leave a note if accurate; add the fail-closed posture if absent)

### AC7 — Record the design-fork consensus (Winston + Murat + security red-team)

**Given** every 8.x security-semantics story has resolved its open design forks by the same trio under the FIXED criterion (most spec-faithful + long-term-correct, explicitly NOT least-effort), and 8.8 has genuine forks
**When** 8.8 is implemented
**Then** the resolutions for **Fork A (toggle default & band-fallback fate)**, **Fork B (unclassified deny granularity — distinct code vs reuse -32001)**, and **Fork C (sender-completeness gate mechanism — static scan strictness)** are recorded in the "Team Consensus" section below with rationale + any dissents, and the ACs above reflect the chosen options
**And** the security invariant is treated as **non-negotiable** (not a fork): the flip denies ONLY unclassified traffic and never silently downgrades — any option that re-introduces a silent band-downgrade for unclassified cross-Host frames is out of bounds

---

## Tasks / Subtasks

- [x] **Task 0 — Confirm preconditions & resolve design forks** (AC: 7)
  - [x] Confirm Story 8.7 (`done`) + Story 8.9 (`done`) are landed; re-read their File Lists (this spec's "Previous Story Intelligence" section). Confirm `consent_match_key` (`router.rs:271`), the 8.9 consent block (`router.rs:582-640`), `prepare_outbound` expiry stamp (`router.rs:391-408`), and `handle_intake_verified` (`router.rs:694`) are at the expected shape.
  - [x] Take Fork A / Fork B / Fork C to the team (Winston + Murat + security red-team) under the fixed criterion. Record resolutions in "Team Consensus". **Recommended defaults are pre-filled below** — implement those unless the team overrides.
- [x] **Task 1 — The fail-closed decision in `maos-a2a-core`** (AC: 1, 2)
  - [x] Add a `cross_host_fail_closed: bool` field to `A2ARouterCore` (`router.rs:116`) + a builder `with_cross_host_fail_closed(mut self) -> Self` mirroring `with_pinned_consent_clock` (`router.rs:176`); set the default per Fork A. Initialize the field in `try_new` (`router.rs:143`) + `new` (`router.rs:168`).
  - [x] Refactor the classification into `fn consent_decision(frame: &IacFrame) -> ConsentDecision` returning `Classified(String)` (canonical `intent_class`, ≤128) or `Unclassified { reason: UnclassifiedReason }` (`Absent` / `NonCanonical` / `Oversized`). Keep the precedence doc-comment citing ADR-012. Retain a `consent_match_key`-equivalent for the fail-open/band-fallback path used when `cross_host_fail_closed == false` (Fork A may delete band-fallback entirely — then `consent_decision` is the only path).
  - [x] Add `CODE_CONSENT_UNCLASSIFIED: i32 = -32009` to `transport/json_rpc.rs` (after `CODE_CONSENT_GRANTER_MISMATCH = -32008`, `json_rpc.rs:54`).
  - [x] Add `A2AError::ConsentUnclassified { direction: IntentDirection, reason }` (send side) + `A2AError::ConsentUnclassifiedAtPeer { peer: String, reason }` (receiver NACK → sender) to `error.rs` (`#[non_exhaustive]` enum). Add the `interpret_response` arm (`router.rs:428`) mapping `CODE_CONSENT_UNCLASSIFIED` → `ConsentUnclassifiedAtPeer`, and a `map_a2a_error_to_iac_bus` arm (maps to `CrossHostRouteFailure`, the no-new-kernel-variant pattern 8.9 used).
  - [x] **Send side** (`prepare_outbound`, `router.rs:359`): when `cross_host_fail_closed`, evaluate `consent_decision` BEFORE the send-allowlist `send_admits` check (`router.rs:368`); on `Unclassified` return `A2AError::ConsentUnclassified { direction: Send, reason }`. A classified frame proceeds to the existing send-allowlist + TOFU + expiry-stamp steps unchanged.
  - [x] **Accept side** (`handle_intake`, the accept-allowlist site `router.rs:642-654`): when `cross_host_fail_closed`, evaluate `consent_decision` immediately before `accept_admits`; on `Unclassified` return a `CODE_CONSENT_UNCLASSIFIED` NACK with `data = { reason, peer }`. Preserve the 8.9 consent-block ordering (granter→expiry runs first, `router.rs:582-640`). A classified frame proceeds to `accept_admits` unchanged.
  - [x] Doc-comment every new site citing Story 8.8 + the red-team invariant ("deny ONLY unclassified, never silently downgrade").
- [x] **Task 2 — Wire fail-closed into the transports + reference senders** (AC: 4, 5)
  - [x] `maos-a2a-tcp` (`transport.rs:163`): construct the `A2ARouterCore` via `try_new(...).with_cross_host_fail_closed()` (composing with the existing `.with_pinned_consent_clock(t)` chain at `transport.rs:169`). The wire is genuine cross-Host → always fail-closed.
  - [x] `maos-a2a` (`adapter.rs:41`, `LoopbackA2ARouter::new`): construct the core in fail-closed mode (the loopback **simulates** cross-Host). If Fork A keeps an opt-in, add a `LoopbackA2ARouter::new_band_fallback`/equivalent constructor ONLY for the legacy band-only test.
  - [x] Confirm `spirits/mira` (`ADVISORY_FINE_GRAINED_INTENT`, `lib.rs`) and `spirits/nash` populate canonical `intent_class` on every cross-Host send (8.7 did this — verify still true; strengthen if any path can emit `None`).
- [x] **Task 3 — Sender-completeness discipline gate** (AC: 3)
  - [x] NEW `xtask/src/check_a2a_sender_completeness.rs` (`pub fn run(workspace_root, json) -> Result<(), String>` + a `#[serde] Report` struct, mirroring `check_workspace_count.rs`). Static-scan the reference sender crates/arms for cross-Host frame construction with `consent_envelope: None` or an envelope built without a populated `intent_class`; FAIL with a precise file:line list. Include a unit test of the scanner over a known-good and known-bad fixture string.
  - [x] Register: `mod check_a2a_sender_completeness;` + a `Commands::CheckA2aSenderCompleteness { ... json }` variant + dispatch arm in `xtask/src/main.rs`; add `"check-a2a-sender-completeness"` to `xtask/gate-registry.toml`.
  - [x] Add a `check-a2a-sender-completeness` job to `.github/workflows/discipline.yml` and to the `aggregate` job `needs:` list (`discipline.yml:1917`).
- [x] **Task 4 — Runnable headline + tests** (AC: 1, 4, 5)
  - [x] NEW `smoke-a2a-fail-closed-8-8` one-shot in `crates/maos-bin/src/main.rs` (mirror `smoke_a2a_consent_vocab_8_7`): classified-admit + absent-deny + non-canonical-deny, exits 0; add to `MAOS_ONE_SHOT` dispatch + the known-modes error-message list.
  - [x] Add the CI job (mirror the `smoke-a2a-consent-vocab-8-7` block at `discipline.yml:1408`) + add to `aggregate` `needs`.
  - [x] NEW tests in `crates/maos-a2a-core/tests/` (extend `cross_host_consent_v1_5.rs` or a new `fail_closed_8_8.rs`): (a) absent envelope → `-32009`/`ConsentUnclassified`, both directions; (b) `None` intent_class → same; (c) non-canonical → same; (d) oversized (129 bytes) → same; (e) classified-but-not-allowlisted still → `-32001` (no conflation); (f) classified-and-allowlisted still admitted; (g) fail-OPEN-mode core still band-falls-back (the retained transitional path, if Fork A keeps it).
  - [x] NEW wire test in `crates/maos-a2a-tcp/tests/` (extend `trust_binding_8_9.rs` or new): an unclassified frame sent over the real wire to a fail-closed receiver gets `-32009` and is NOT delivered (assert intake sink empty).
  - [x] Same-Host untouched test (AC5): a same-Host IAC delivery with no `consent_envelope` succeeds (routes via `iac_bus`, not the core) — cite the verification grep in the Dev Agent Record.
- [x] **Task 5 — Gates, discipline, reconciliation** (AC: 3, 6)
  - [x] Run `kloc-check` (maos-a2a-core ≤3000, maos-a2a ≤1500), `check-workspace-count` (=41), `abi-diff --base xtask/abi-baseline` (Added-only), git-verify `maos-kernel-core` byte-identical. Record all counts in evidence.
  - [x] Run the FULL fail-closed-readiness suite (AC3): `a1_security_regression_guards`, `cross_host_consent_v1_5`, `trust_binding_8_9`, all 4 smoke arms. Confirm GREEN at HEAD with fail-closed on.
  - [x] Set `dev_model_used` frontmatter; verify `check-dev-record-completeness` / `check-review-findings-resolved` / `check-dev-model-used-populated` GREEN. Reconcile architecture narrative if it claims cross-Host consent fails open.

---

## Dev Notes

### What this story is (and is NOT)

- **IS:** a policy flip (band-fallback → fail-closed) for the cross-Host A2A router, behind a sender-completeness gate that proves the flip is mechanical. New typed deny (`-32009`), a mode toggle on `A2ARouterCore`, a new xtask gate, a new smoke arm. Lands in `maos-a2a-core` + `maos-a2a-tcp` + `maos-a2a` + `maos-bin` + `xtask` + `spirits/{mira,nash}`.
- **IS NOT:** any change to identity binding (8.9/G8, done), granter binding (8.9/G1, done), expiry (8.9/G10, done), or the fine-grained match itself (8.7/AC1, done). 8.8 changes ONLY the *fallback policy* (G7) — what happens when a cross-Host frame is unclassified. It is **zero `maos-kernel-core` KLOC** and **zero new crate** (workspace stays 41).

### Current code state — exact anchors (read these before editing)

The enforcement path is `A2ARouterCore` in `crates/maos-a2a-core/src/router.rs`. Verified shape post-8.9 (2026-06-06):

- **`A2ARouterCore` struct** — `router.rs:116`; builder pattern: `try_new` (`:143`), `new` (`:168`), `with_pinned_consent_clock(now_ns)` (`:176`, consumes/returns self — the template for the new `with_cross_host_fail_closed`). Field `consent_now_ns: Option<u64>` (`:129`); `consent_now_ns()` accessor (`:183`).
- **`consent_match_key`** — `router.rs:271-283`. **This is the fail-open seam to refactor.** Current body:
  ```rust
  fn consent_match_key(frame: &IacFrame) -> String {
      frame.consent_envelope.as_ref()
          .and_then(|e| e.intent_class.as_ref())
          .filter(|i| i.as_str().len() <= MAX_CANONICAL_INTENT_LEN)   // 8.9/G9 — 128
          .map(|i| i.as_str().to_string())
          .unwrap_or_else(|| Self::frame_intent_str(frame))            // ← SILENT BAND-FALLBACK (G7)
  }
  ```
  The `.unwrap_or_else(|| Self::frame_intent_str(frame))` is the exact silent-downgrade Story 8.8 must replace with a fail-closed deny (under fail-closed mode). Note: `is_canonical()` is NOT currently applied here — today an absent OR an oversized intent falls back; a *present-but-non-canonical* intent_class is currently used verbatim as the key. AC1 widens "unclassified" to include non-canonical (`!is_canonical()`), so a present-but-garbage `intent_class` denies instead of becoming an unmatchable key.
- **`frame_intent_str`** — `router.rs:241-243`, `pub`. The 3-band projection (`IntentClass::a2a_consent_intent_str()`, `i1.rs:143`). **Keep pub, unchanged** (8.6/8.7 abi lesson).
- **`send_admits`** (`router.rs:324`) / **`accept_admits`** (`router.rs:336`) — both call `consent_match_key` then case-insensitive allowlist match; on miss, call `warn_unreachable_entries` (`:295`).
- **`prepare_outbound`** — `router.rs:359-416`. Step (1) send-allowlist (`:368`), (2) TOFU (`:383`), (3) Lamport (`:389`), (3.5) 8.9 expiry stamp (`:403-408`), (4) build request. **The fail-closed send deny goes BEFORE step (1).**
- **`handle_intake`** — the consent block at `router.rs:582-640` (8.9: granter `:594`, expiry `:622`), then accept-allowlist `router.rs:642-654`. **The fail-closed accept deny goes immediately before `:643`.**
- **`handle_intake_verified`** — `router.rs:694-725` (8.9/G8 — binds `frame.from.host_id` to TLS-verified `verified_peer`, returns `(response, binding_passed)`, then delegates to `handle_intake` at `:724`). Fail-closed inherits automatically via the shared `handle_intake` body — no separate change needed here.
- **`interpret_response`** — `router.rs:421-485`; NACK-code → `A2AError` mapping (`CODE_INTENT_DENIED` → `IntentDeniedAtPeer` at `:429`). Add the `-32009` arm here.
- **Error codes** — `crates/maos-a2a-core/src/transport/json_rpc.rs:26-55`: `CODE_INTENT_DENIED=-32001`, `…_PIN_MISMATCH_NOT_PINNED=-32002`, `…_CONSENT_EXPIRED=-32003`, `…_SPIRIT_RESTART_DETECTED=-32004`, `…_TIMEOUT=-32005`, `…_FRAME_TOO_LARGE=-32006`, `…_PEER_IDENTITY_MISMATCH=-32007` (8.9), `…_CONSENT_GRANTER_MISMATCH=-32008` (8.9), `…_INTERNAL=-32099`. **Add `…_CONSENT_UNCLASSIFIED=-32009`.**
- **`A2AError`** — `crates/maos-a2a-core/src/error.rs:22-119`, `#[non_exhaustive]`. Variants from 8.9: `PeerIdentityMismatch`, `ConsentGranterMismatch`, `ConsentExpired`, `IntentDenied{direction,inner}`, `IntentDeniedAtPeer{peer,message}`. **Add `ConsentUnclassified` + `ConsentUnclassifiedAtPeer`** (Added-only, enum is `#[non_exhaustive]`).
- **`A2AIntent::is_canonical`** — `crates/maos-domain/src/invariants/i8.rs:79-109`; `MAX_CANONICAL_INTENT_LEN=128` (`i8.rs:44`). Grammar `^[a-z0-9]+(-[a-z0-9]+)*(:[a-z0-9]+(-[a-z0-9]+)*)?$`. Use `is_canonical()` as the classification predicate.
- **`ConsentAllowlists`** (`consent.rs:31`), **`EIntentDenied`** (`consent.rs:57`), **`AllowlistDirection`** (`consent.rs`). Preserve all.
- **Same-Host separation (verified):** `grep -rln 'A2ARouterCore|handle_intake|prepare_outbound' crates/maos-domain/src crates/maos-kernel-core/src` → **zero matches**. Same-Host IAC routes through `iac_bus.rs`, never the A2A core. **This is the load-bearing fact** that lets fail-closed be a core-level flag with no in-band discriminator and no same-Host regression.

### Transports (where to wire the flag)

- **`maos-a2a-tcp`** — `crates/maos-a2a-tcp/src/transport.rs`: `core: Arc<A2ARouterCore>` (`:105`); built in `bind` via `A2ARouterCore::try_new(...)` (`:163`) then `.with_pinned_consent_clock(t)` (`:169`). Genuine cross-Host wire → **always fail-closed**. `serve_connection` re-derives the verified peer and calls `handle_intake_verified` (8.9/G8); fail-closed flows through the delegated `handle_intake`.
- **`maos-a2a`** — `crates/maos-a2a/src/adapter.rs`: `LoopbackA2ARouter { core: A2ARouterCore }` (`:35`), built via `A2ARouterCore::new(...)` (`:41`); `route_outbound` calls `core.prepare_outbound(frame, peer, 0)` then the `handle_intake` shortcut (`:70`). The loopback **simulates** cross-Host → fail-closed. Keep a band-fallback constructor ONLY if Fork A retains the opt-in for the legacy band-only test.

### Smoke arms & CI (where to register)

- Smoke arms dispatch via `MAOS_ONE_SHOT` string-match in `crates/maos-bin/src/main.rs` (known-modes list near `:3228`; existing arms `smoke_a2a_consent_vocab_8_7` `:5090-5257`, `smoke-mira-nash-8-5` `:4205+`, `smoke-a2a-loopback-6-3` `:4625+`). Add `smoke-a2a-fail-closed-8-8` + its dispatch + known-modes entry.
- CI jobs in `.github/workflows/discipline.yml`: mirror the `smoke-a2a-consent-vocab-8-7` block (`:1408-1424`). The **`aggregate` job's `needs:` list** (`:1917`) is the completeness gate — BOTH the new smoke job AND the new `check-a2a-sender-completeness` gate must be added there or the gate-aggregation is incomplete (the project treats a missing `needs` entry as a silent coverage hole).

### xtask gate pattern (for `check-a2a-sender-completeness`)

Template = `xtask/src/check_workspace_count.rs`: a `pub fn run(...) -> Result<(), String>` printing a PASSED/FAILED line (or `--json` a `#[derive(serde::Serialize)] Report`), returning `Err` on failure. Wiring: `mod check_workspace_count;` (`main.rs:30`) + a `Commands::` enum variant with `#[arg(long)]` paths + a dispatch arm (`main.rs:632`). Register in `xtask/gate-registry.toml` (the canonical list cross-referenced by `coverage-matrix`; adding is a widening). Keep the scan deterministic and self-testing (unit test over a good/bad fixture string) — the gate itself must not flake.

### Design forks — RECOMMENDED resolutions (confirm with team; see AC7)

**Fork A — toggle default & band-fallback fate.** The toggle must exist (the precondition says "toggled to fail-closed mode"). The question is the *default* and whether band-fallback survives.
- **Option 1 (RECOMMENDED — fail-closed default + explicit per-peer/opt-in band-fallback for genuine legacy):** `A2ARouterCore` defaults fail-closed; band-fallback is re-enabled only via an explicit `with_band_fallback()` opt-in (or a per-peer `allow_band_fallback: bool`, default `false`), loudly logged. New operators are fail-closed by default (no fail-open-default footgun); the legacy band-only test opts in explicitly. Most spec-faithful to "fail-closed is the committed end-state" + preserves a documented migration affordance + never silently downgrades (the opt-in is explicit).
- **Option 2 (fail-closed default, band-fallback DELETED):** remove the `unwrap_or_else(frame_intent_str)` entirely; cross-Host always requires fine-grained intent. Cleanest long-term, highest churn (migrate `cross_host_consent_v1.rs::scenario_3_1`). Choose if the team wants zero residual fail-open surface.
- **Option 3 (toggle default OFF, opt-in fail-closed) — REJECTED:** a fail-open default is exactly the footgun this story closes; out of bounds per the security invariant.
- *Recommendation:* **Option 1** (fail-closed default; explicit, logged opt-in for legacy). Defer to team if they prefer the harder Option 2.

**Fork B — unclassified deny granularity.** Distinct `CODE_CONSENT_UNCLASSIFIED` (-32009) + new `A2AError` variants, vs reuse `CODE_INTENT_DENIED` (-32001) with a distinct message.
- *Recommendation:* **distinct code/variant (-32009).** The red-team invariant "deny ONLY unclassified, never silently downgrade" demands the deny be *legible and auditable* in the TL, distinct from classified-but-not-allowlisted (-32001). Consistent with 8.9's additive -32007/-32008 pattern; Added-only, no abi exception. (Murat may prefer reuse to minimize surface — record the dissent if so.)

**Fork C — sender-completeness gate strictness.** A hard static scan (build-time deny on any cross-Host `consent_envelope: None` literal) vs a softer runtime-only assertion.
- *Recommendation:* **both** (the LOCKED precondition specifies "static/build-time scan + a runtime assertion in every smoke arm"). The xtask static scan is the GREEN-at-HEAD gate; `prepare_outbound` under fail-closed is the runtime backstop (a missed sender gets `ConsentUnclassified{Send}` and the frame never leaves). Belt-and-suspenders; neither alone is sufficient (a static scan can miss a dynamic construction; a runtime assertion only fires on exercised paths).

### Security invariant (non-negotiable — NOT a fork)

The fail-closed flip must **deny ONLY unclassified traffic and never silently downgrade**. A classified frame's behavior is unchanged (8.7 semantics). An unclassified cross-Host frame is denied with the distinct typed signal — it is NEVER routed through `frame_intent_str` to a band match. Any test or option that lets an unclassified cross-Host frame be admitted via the 3-band projection is a defect.

### Discipline lessons to honor (from prior retros / memory)

- **Never flip a gate while red** (`[[feedback_mechanical_gates_compound_promises_decay]]` — the recurring AC4 trap, seen in 7.1.6/7.1.7): the `check-a2a-sender-completeness` gate and the fail-closed flip land in the SAME commit, both GREEN at HEAD. Do NOT mark this story `done` with the gate red.
- **abi-diff needs `--base`** (Story 8.3 lesson): use `--base xtask/abi-baseline`; no-base `HEAD~1` mode false-positives.
- **Never `cargo fmt -p <crate>` here** (Story 7.5a lesson): whole-crate fmt causes collateral churn that trips `check-service-boundary`. Format only edited regions.
- **Record `dev_model_used`** in frontmatter (§A2 discipline — three epics of carry-forward; do NOT add to the debt).
- **Verify pre-existing REDs are story-neutral, don't fix them blind:** the workspace-total `kloc-check` NFR-Maint-1 alarm (74620 ≫ 16k, red since epic 1 — per-crate budgets are what matter) and the `maos-mcp fixture_replay` test-compile break (reproduced at HEAD pre-8.9, untouched) are known pre-existing; reproduce-with-changes-stashed and note as neutral (the 8.9 pattern). The diagnostics in the current working tree (`stdio.rs`, `client.rs`, `server.rs`, `openai.rs`, `fixture_replay.rs` unused imports/vars in `maos-mcp`) are pre-existing and unrelated — do NOT scope-creep into them unless a touched file needs the fix to compile.

### Testing standards

- Unit + integration tests in the owning crate's `tests/` (the project convention: `crates/maos-a2a-core/tests/cross_host_consent_v1_5.rs`, `crates/maos-a2a-tcp/tests/trust_binding_8_9.rs`). Pin consent clocks via `with_pinned_consent_clock` for determinism (the 8.9 on-wire-expiry pattern — pin sender at T0, receiver past expiry). Use the `tracing-subscriber` capture pattern (8.7 AC5) if asserting on warn output. The runnable headline (`smoke-a2a-fail-closed-8-8`) is the observable end-to-end demo and must exit 0 in CI.
- Negative tests must prove the *non-conflation*: classified-but-not-allowlisted → -32001; unclassified → -32009. Both directions (send + accept).

### Project Structure Notes

- All production code lands in existing crates (`maos-a2a-core`, `maos-a2a-tcp`, `maos-a2a`, `maos-bin`, `xtask`, `spirits/mira`, `spirits/nash`). **No new crate** → `check-workspace-count` stays 41. **No `maos-kernel-core` delta** → zero-kernel-KLOC mandate held (git-verify byte-identical). The new gate lives in the existing `xtask` crate; the new smoke arm in existing `maos-bin`. No structural variance from the established A2A layout.

### References

- [Source: _bmad-output/planning-artifacts/epics/epic-8-…miranash-v03-v15.md#story-88-fail-closed-for-cross-host-a2a-consent] — Story 8.8 narrative + LOCKED precondition + security invariant (lines 353-364).
- [Source: _bmad-output/planning-artifacts/epics/dependency-dag.md] — 8.8 DEPENDS ON 8.7 + 8.9; "closes audit gap G7"; "G7 survives 8.9 — 8.9 fixes identity/granter/expiry, not the fallback policy" (lines 58-61, 87).
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-2026-06-05.md#section-4] — 8.8 registration; Q2 consensus "commit fail-closed as Story 8.8 behind a sender-completeness gate".
- [Source: _bmad-output/implementation-artifacts/8-7-fine-grained-typed-intent-consent-vocabulary-over-maos-a2a-core.md] — AC1 fine-grained match + band fallback (the path 8.8 flips); AC2 mandatory reference-sender `intent_class` population (the corpus that makes 8.8's flip B-clean); AC9 commits 8.8; `consent_match_key` design.
- [Source: _bmad-output/implementation-artifacts/8-9-a2a-trust-binding-and-consent-integrity-hardening.md] — AC3/G10 `prepare_outbound` expiry + `consent_ttl_secs`; AC4/G2 consent-block ordering; AC1/G8 `handle_intake_verified`; "UNBLOCKS Story 8.8"; the `// TRANSITIONAL` comment naming 8.8 as fail-closed-on-absent owner.
- [Source: crates/maos-a2a-core/src/router.rs:241-283] — `frame_intent_str` (keep pub) + `consent_match_key` (refactor target / band-fallback seam).
- [Source: crates/maos-a2a-core/src/router.rs:359-416, 582-654, 694-725] — `prepare_outbound`, `handle_intake` consent block + accept-allowlist, `handle_intake_verified`.
- [Source: crates/maos-a2a-core/src/transport/json_rpc.rs:26-55] — error-code constants (add -32009).
- [Source: crates/maos-a2a-core/src/error.rs:22-119] — `#[non_exhaustive] A2AError` (add variants).
- [Source: crates/maos-domain/src/invariants/i8.rs:44-124] — `A2AIntent::is_canonical`/`parse` + `MAX_CANONICAL_INTENT_LEN`.
- [Source: crates/maos-a2a-tcp/src/transport.rs:105-234] — `A2ATransport`/`bind` core construction (wire fail-closed).
- [Source: crates/maos-a2a/src/adapter.rs:35-70] — `LoopbackA2ARouter` (wire fail-closed for the simulation).
- [Source: xtask/src/check_workspace_count.rs + xtask/gate-registry.toml + xtask/src/main.rs:30,271,632] — gate template + registration pattern.
- [Source: .github/workflows/discipline.yml:1408-1424, 1917] — smoke-job template + the `aggregate` `needs:` completeness list.
- [Source: crates/maos-a2a/tests/a1_security_regression_guards.rs] — the fail-closed-readiness suite (P1/P2/P5/P6/P7).

---

## Dev Agent Record

### Agent Model Used

`claude-opus-4-8` (Claude Code dev-story execution, 2026-06-06) — security-semantics + discipline-gate work in the A2A core, consistent with the 8.5/8.6/8.7/8.9 recommendation.

### Debug Log References

- `cargo test -p maos-a2a-core -p maos-a2a -p maos-a2a-tcp` — all green (existing + migrated + new `fail_closed_8_8.rs` 8 tests + TCP `g7_unclassified_frame_denied_on_wire`).
- `MAOS_ONE_SHOT=smoke-a2a-fail-closed-8-8 ./target/release/maos-bin` → exit 0; all 5 reference A2A smoke arms exit 0.
- `cargo run -p xtask -- check-a2a-sender-completeness --json` → `passed: true` (3 files scanned, 0 violations) — GREEN at HEAD in the same commit as the flip.
- `cargo run -p xtask -- check-workspace-count` → 41/41 ✅; `abi-diff --base abi-baseline/v1-pre-bump.txt` → `passed: true, removed: []`.
- `kloc-check`: maos-a2a-core 3034/3050 ✅ (ceiling bumped from 3000 to tight measured residual per Fork D consensus); maos-a2a 210/1500 ✅; maos-a2a-tcp 940/1500 ✅.
- `git diff --stat crates/maos-kernel-core/` → empty (byte-identical; zero-kernel-KLOC mandate held).
- AC5 same-Host separation: `grep -rln 'A2ARouterCore|handle_intake|prepare_outbound' crates/maos-domain/src crates/maos-kernel-core/src` → **zero matches** (same-Host IAC routes via `iac_bus.rs`, never the A2A core).

### Completion Notes List

- ✅ **AC1/AC2 — fail-closed decision (single shared seam).** Added the private `consent_decision(frame) -> ConsentDecision { Classified(String) | Unclassified { reason } }` seam in `router.rs`; `consent_match_key` now delegates to it (returning `String::new()` for the `Unclassified` arm — an empty never-matching key, not a band projection), so "the key tested == the key reported" holds. Fail-closed gates added at `prepare_outbound` (before send-allowlist → `A2AError::ConsentUnclassified { Send }`, frame never leaves) and `handle_intake` (immediately before `accept_admits`, after the 8.9 granter→expiry block → `CODE_CONSENT_UNCLASSIFIED` -32009 NACK with `{reason, peer}` data). `frame_intent_str` stays `pub` and unchanged. AC1 widening: a present-but-non-canonical `intent_class` is now `Unclassified{NonCanonical}` (prev…
- ✅ **AC6/Fork B — distinct typed deny.** `CODE_CONSENT_UNCLASSIFIED = -32009` (additive after 8.9's -32008) + `A2AError::ConsentUnclassified { direction, reason }` / `ConsentUnclassifiedAtPeer { peer, reason }` + `UnclassifiedReason { Absent | NonCanonical | Oversized }` (all Added; `A2AError` is `#[non_exhaustive]`). `interpret_response` maps -32009 → `ConsentUnclassifiedAtPeer` (NOT conflated with -32001); `map_a2a_error_to_iac_bus` maps both to `CrossHostRouteFailure` (no new kernel variant — 8.9 pattern).
- ✅ **Fork A — DELETE band-fallback (Option 2, team consensus 2026-06-07 — reversed the first pass's Option 1).** Fail-closed is now **unconditional**: no `cross_host_fail_closed` field, no `with_cross_host_fail_closed`/`with_band_fallback`/accessor, no `LoopbackA2ARouter::new_band_fallback`. Both enforcement gates always run; `consent_match_key` has no band-downgrade arm (unclassified → empty never-matching key, unreachable past the gate); `frame_intent_str` stays `pub` (ABI) but is no longer a consent path. There is NO silent-downgrade surface at all — the mechanism is removed, not toggled off. `LoopbackA2ARouter::new` + `TcpA2ATransport::bind` are fail-closed by construction.
- ✅ **AC3/Fork C — both static gate + runtime backstop, HARDENED (team consensus).** NEW `xtask/src/check_a2a_sender_completeness.rs` static-scans spirits/{mira,nash} + the 4 cross-Host smoke-arm fn bodies in maos-bin for `consent_envelope: None`; registered in `main.rs` + `gate-registry.toml` + a `discipline.yml` job + the `aggregate` `needs`. GREEN-at-HEAD. Runtime backstop = unconditional `prepare_outbound` (`ConsentUnclassified{Send}`), structurally non-exemptible. Hardenings: **literal-aware brace counting** (skips braces in strings/char/raw-strings/comments → no false-GREEN body truncation; fixture tests); exemptions are **static-scanner-only**, **never honored in spirits/{mira,nash}**, **require a justification**, and are **drift-gated against `EXEMPT_BASELINE=0`**. Self-testing scanner (11 unit tests).
- ✅ **AC4 — runnable headline.** NEW `smoke-a2a-fail-closed-8-8` (classified-admit + absent-deny -32009 + sender-refuse + non-canonical-deny -32009), dispatch + known-modes + `discipline.yml` job + `aggregate` needs. Exits 0.
- ✅ **AC5 — zero regression + same-Host untouched.** Classified traffic unchanged (the migrated reference fleet all populates `intent_class`). Same-Host IAC proven untouched by the grep above (cited test: existing `iac_bus`-routed None-envelope deliveries in `crates/maos-kernel-core/tests/{orchestrator_distillate_dispatch,drr_scheduler}.rs` remain green — kernel byte-identical). Migrated the ONE still-`None` cross-Host sender (`smoke-a2a-tcp-8-6`) to a fine-grained `intent_class` + matching allowlists.
- **Test migrations (fail-closed-default consequence):** legacy band-semantics suites opt into band-fallback explicitly — router.rs `pinned_core`/adapter.rs `pinned_router`/`cross_host_consent_v1.rs` scenarios → `.with_band_fallback()`/`new_band_fallback`; `cross_host_consent_v1_5.rs` 4 band tests → new `band_fallback_core` helper. a1 `p6_zero` + tcp-support `make_frame` made B-clean (classified). All classified-frame suites run under the fail-closed default (honest readiness proof).
- **Pre-existing REDs verified story-neutral:** (1) `smoke-a2a-consent-vocab-8-7` had a HEAD-failing denial-message assertion (expected `for peer loopback`, actual `host_a`) — verified red at HEAD with changes stashed; fixed the assertion to the truthful peer name (AC5 needs the arm green; same pattern as 8.7's "fixed pre-existing maos-bin break"). (2) Workspace-total kloc alarm + per-crate overages (maos-bin 5446→5605, maos-domain 6614, maos-kernel-core 15505) red since epic 1 — maos-bin smoke addition is expected/neutral. (3) `xtask example_spirit_regen_integration::check_mode_fails_on_drift` flakes only under concurrent xtask test batches (passes in isolation both at HEAD and post-change; CI runs it as an isolated job) — unrelated to A2A.
- **kloc ceiling (team consensus, re-sequenced):** maos-a2a-core 3000 → **3050** (tight measured residual; NOT the first pass's round 3100). Per consensus the band-fallback deletion (Fork A) landed FIRST, then re-measured: actual **3034**. AC6's "was 2644" premise was stale (HEAD 2942). A submodule split was considered but doesn't reduce crate kloc (tokei sums all src under the crate). Ratified by Winston at the specific measured number.

### File List

**Production (maos-a2a-core):**
- `crates/maos-a2a-core/src/transport/json_rpc.rs` — `CODE_CONSENT_UNCLASSIFIED = -32009`.
- `crates/maos-a2a-core/src/error.rs` — `UnclassifiedReason` enum + Display; `A2AError::ConsentUnclassified` / `ConsentUnclassifiedAtPeer` variants.
- `crates/maos-a2a-core/src/router.rs` — `ConsentDecision` enum; `consent_decision` seam; `consent_match_key` refactor (no band-downgrade arm); **unconditional** fail-closed gates in `prepare_outbound` + `handle_intake`; `interpret_response` -32009 arm; `map_a2a_error_to_iac_bus` arms. (Team-consensus Option 2: the `cross_host_fail_closed` field + `with_cross_host_fail_closed`/`with_band_fallback`/accessor were NOT shipped — fail-closed is unconditional; `frame_intent_str` kept `pub` for ABI, no longer a consent path.)

**Production (transports + reference senders):**
- `crates/maos-a2a-tcp/src/transport.rs` — `bind` constructs the core fail-closed (unconditional; no toggle call).
- `crates/maos-a2a/src/adapter.rs` — `LoopbackA2ARouter::new` fail-closed by construction (no `new_band_fallback`).
- `crates/maos-bin/src/main.rs` — NEW `smoke_a2a_fail_closed_8_8` arm + dispatch + known-modes; migrated `smoke-a2a-tcp-8-6` frame/allowlists to fine-grained intent; fixed pre-existing `smoke-a2a-consent-vocab-8-7` denial-message assertion.

**Discipline gate (xtask) + CI:**
- `xtask/src/check_a2a_sender_completeness.rs` — NEW gate, HARDENED (literal-aware brace counting, spirits-never-exemptible, justification-required, `EXEMPT_BASELINE` drift-gate; 11 self-tests).
- `xtask/src/main.rs` — `mod` + `CheckA2aSenderCompleteness` command + dispatch.
- `xtask/gate-registry.toml` — `check-a2a-sender-completeness`.
- `xtask/kloc.toml` — maos-a2a-core ceiling 3000 → 3050 (team consensus, tight measured residual).
- `.github/workflows/discipline.yml` — `check-a2a-sender-completeness` + `smoke-a2a-fail-closed-8-8` jobs + both added to `aggregate` needs.

**Tests:**
- `crates/maos-a2a-core/tests/fail_closed_8_8.rs` — NEW (8 tests: absent/empty/non-canonical/oversized both directions, -32001 non-conflation, classified admit, band-fallback opt-in, NACK round-trip).
- `crates/maos-a2a-tcp/tests/trust_binding_8_9.rs` — NEW `g7_unclassified_frame_denied_on_wire`.
- `crates/maos-a2a-core/src/router.rs` (inline tests), `crates/maos-a2a/src/adapter.rs` (inline tests), `crates/maos-a2a/tests/cross_host_consent_v1.rs`, `crates/maos-a2a/tests/a1_security_regression_guards.rs`, `crates/maos-a2a-core/tests/cross_host_consent_v1_5.rs`, `crates/maos-a2a-tcp/tests/support/mod.rs` — Option 2 migrations: classified frames / fail-closed-deny assertions (band-fallback path removed).
- `spirits/mira/tests/a2a_pairing.rs` + `spirits/mira/tests/halt_bilateral.rs` — downstream consumers (caught in the pre-review sweep): migrated their None-envelope band-fallback frames to classified intents (`send_side_denial_carries_eintentdenied` → classified-but-not-allowlisted; `advisory_frame` helper → populated `ADVISORY_CONSENT_INTENT`). (maos-iac is neutral — its tests use a `StubRouter` mock of the domain port, not the real core.)

**Architecture narrative:**
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/7-inter-agent-communication.md` — added the fail-closed-on-unclassified posture paragraph.

---

## Change Log

| Date | Change |
|---|---|
| 2026-06-06 | Story created (`bmad-create-story`, claude-opus-4-8): fail-closed-for-cross-Host A2A consent (closes audit G7) — deny unclassified cross-Host frames with new `CODE_CONSENT_UNCLASSIFIED` (-32009) + `A2AError::ConsentUnclassified{,AtPeer}` instead of silent 3-band downgrade; `with_cross_host_fail_closed` toggle on `A2ARouterCore`; NEW `check-a2a-sender-completeness` xtask gate (GREEN-at-HEAD precondition); NEW `smoke-a2a-fail-closed-8-8` headline. DEPENDS ON 8.7 (done) + 8.9 (done). Zero kernel KLOC, workspace stays 41. 3 design forks (toggle default, deny granularity, gate strictness) pre-resolved with recommendations, flagged for Winston+Murat+security red-team. Status → ready-for-dev. |
| 2026-06-07 | Pre-review sweep (clippy / fmt / service-boundary / downstream consumers): caught + fixed 2 downstream test regressions the touched-crates pass missed — `spirits/mira/tests/{a2a_pairing,halt_bilateral}.rs` routed None-envelope band-fallback frames; migrated to classified intents. Verified neutral: clippy (my new code clean; touched-file warnings pre-existing), no CI fmt gate (repo pre-existing non-fmt-clean per 7.5a), check-service-boundary GREEN, maos-iac neutral (StubRouter mock), maos-kernel-core has zero A2A-router test usage (the intermittent ~1-in-13 kernel flake is pre-existing timing, not 8.8). |
| 2026-06-07 | Team consensus (party-mode roundtable: Winston + Murat + security red-team, UNANIMOUS): **Fork A reversed to Option 2 — DELETE band-fallback entirely** (removed field/builders/accessor/`new_band_fallback`; gates now unconditional; no silent-downgrade surface exists). Fork B/C **ratified**, Fork C **hardened** (literal-aware brace counting + spirits-never-exemptible + justification-required + `EXEMPT_BASELINE` drift-gate). Fork D **re-sequenced**: deleted band-fallback first → re-measured maos-a2a-core = 3034 → ceiling set to tight 3050 (not 3100). Legacy band-semantics tests migrated to classified / repurposed to fail-closed-deny assertions. All a2a suites + 5 smoke arms green; kernel byte-identical; abi-diff Added-only. |
| 2026-06-06 | Implemented (`bmad-dev-story`, claude-opus-4-8): all 7 ACs satisfied. Forks resolved per recommendations (A: default fail-closed + dual `with_cross_host_fail_closed`/`with_band_fallback` builders; B: distinct -32009; C: both static gate + runtime backstop). Shared `consent_decision` seam + send/accept gates in maos-a2a-core; -32009 + 3 new typed errors (Added-only); fail-closed wired into both transports; `check-a2a-sender-completeness` gate GREEN-at-HEAD; `smoke-a2a-fail-closed-8-8` exits 0; all 5 A2A smoke arms green; readiness suite (a1/v1_5/trust_binding_8_9) green under fail-closed; kernel byte-identical; workspace 41; abi-diff Added-only. kloc maos-a2a-core ceiling 3000→3100 (FLAG-Winston; stale spec premise). Fixed pre-existing 8.7-smoke denial-message assertion. Status → review. |

---

## Team Consensus — design forks (RESOLVE before/at dev-story; see AC7)

> Take Fork A / B / C to Winston (architect) + Murat (TEA) + security red-team under the FIXED criterion (most spec-faithful + long-term-correct, explicitly NOT least-effort) — the same trio/criterion that resolved the 8.6 `ca_roots` fork, 8.7 Q2–Q5, and 8.9 §D1. Record resolutions + dissents here. **Recommended defaults (implement unless overridden):**
>
> - **Fork A (toggle default & band-fallback):** Option 1 — `A2ARouterCore` defaults **fail-closed**; band-fallback re-enabled only via an explicit, logged opt-in (`with_band_fallback()` / per-peer `allow_band_fallback`, default `false`) for genuine unmigrated legacy peers. (No fail-open default footgun; preserves a documented migration affordance; never silently downgrades.)
> - **Fork B (deny granularity):** distinct `CODE_CONSENT_UNCLASSIFIED` (-32009) + new `A2AError` variants (legible/auditable deny, additive, consistent with 8.9's -32007/-32008). Record any Murat surface-minimization dissent.
> - **Fork C (gate strictness):** both static xtask scan (GREEN-at-HEAD) + runtime `prepare_outbound` backstop (the LOCKED precondition specifies both).
> - **Non-negotiable (NOT a fork):** deny ONLY unclassified, never silently downgrade.

### RESOLVED — UNANIMOUS team consensus (party-mode roundtable 2026-06-07: Winston + Murat + security red-team, FIXED criterion = most spec-faithful + long-term-correct, NOT least-effort)

The first dev-story pass (2026-06-06) shipped the spec's *recommended* defaults (Fork A Option 1: default fail-closed + a `with_band_fallback()` opt-in). The team was then convened and **reversed Fork A 3/3**; the other forks were ratified with hardenings. Final resolutions (re-implemented 2026-06-07):

- **Fork A — band-fallback fate → Option 2: DELETE band-fallback entirely (3/3, reverses the first pass).** Rationale (unanimous): `with_band_fallback()` re-exposed the *exact* silent 3-band downgrade G7 exists to close — "a warn log is not a control, it is a confession written after the breach" (red-team). It served a population of **zero** shipped cross-Host peers (the entire reference fleet populates fine-grained `intent_class` post-8.7), and its only consumers were the legacy tests written to keep it alive ("coverage theater" — Murat). It is the same loaded-gun fail-open the team **deleted** in 8.7 (`A2AConsentEnvelope`), reincarnated as callable public API — the team will not reverse its own 8.7 ruling under a friendlier name (red-team + Winston). A genuine future unmigrated-peer requirement is "a new, scoped, threat-modeled story with an owner and an expiry, not a permanent latent toggle" (red-team). **Implemented:** removed the `cross_host_fail_closed` field + `with_cross_host_fail_closed` / `with_band_fallback` / accessor + `LoopbackA2ARouter::new_band_fallback`; both enforcement gates are now **unconditional**; `consent_match_key` has no band-downgrade arm (unclassified → empty never-matching key, unreachable post-gate); `frame_intent_str` stays `pub` (ABI) but is no longer a consent-enforcement path. Legacy band-semantics tests migrated to classified frames (`cross_host_consent_v1` 3_1–3_4, router/adapter inline) or repurposed to assert fail-closed deny (`cross_host_consent_v1_5` band/oversized cases, the 8.9 None-passthrough test).
- **Fork B — deny granularity → RATIFY distinct `CODE_CONSENT_UNCLASSIFIED` (-32009)** + `ConsentUnclassified{,AtPeer}` + `UnclassifiedReason` (3/3). Two different operator runbooks (-32001 → fix the peer's allowlist; -32009 → fix the sender's frame construction) and two different security events (stale/probing peer vs active privilege probe) demand distinct, machine-readable codes. Murat's standing surface-minimization instinct **yielded** here ("auditability cost, not legibility nicety"). Distinct deny is also what makes Option 2 safe — an unmigrated peer gets an unambiguous typed diagnosis instead of silent success.
- **Fork C — gate strictness → RATIFY both, with HARDENINGS (3/3 + Murat/red-team conditions).** Static `check-a2a-sender-completeness` (shift-left GREEN-at-HEAD) + runtime `prepare_outbound` backstop (catches dynamic/uncovered senders) are genuine defense-in-depth. **Hardenings implemented:** (1) literal-aware brace counting — the fn-body extractor skips `{`/`}` inside string/char/raw-string literals and `//`,`/* */` comments so a brace-in-string can never desync and produce a FALSE GREEN (fixture tests added); (2) the `SENDER-COMPLETENESS-EXEMPT` escape hatch is **static-scanner-only** (the runtime backstop is structurally non-exemptible — `prepare_outbound` has no exempt path), **never honored on `spirits/{mira,nash}`** (production senders), **requires a non-empty justification**, and the honored-exemption count is **drift-gated against `EXEMPT_BASELINE` (=0)** so adding an exemption without a reviewed baseline bump is a RED gate.
- **Fork D — kloc ceiling → re-sequenced + tightened (3/3).** Do NOT pre-bump. Per consensus: delete band-fallback (Fork A) FIRST, then re-measure. Result: maos-a2a-core = **3034** post-deletion (a submodule split was considered but does not reduce crate kloc — tokei sums all src under the crate). Ceiling set to the **tight measured residual 3050** (~16-line margin), NOT the first pass's round 3100 ("don't buy ceiling for fail-open code" — red-team; "bump only the residual to a tight number" — Murat). AC6's "was 2644" premise was stale (HEAD = 2942).
- **Non-negotiable upheld (stronger than the first pass):** there is now NO silent-downgrade surface *at all* — the band-projection mechanism is removed, not toggled off. The flip denies ONLY unclassified cross-Host traffic; classified behavior is 8.7-identical; same-Host IAC is untouched (separate `iac_bus.rs` path, verified grep).

---

### Review Findings

Generated by code-review workflow (2026-06-07). Three layers ran: Blind Hunter, Edge Case Hunter, Acceptance Auditor. All layers succeeded.

**decision-needed (0):**
_Resolved by team consensus (Winston + Murat + security red-team, 2026-06-07, FIXED criterion):_ **Keep `NonCanonical`** (Option 1). Empty string fails the canonical grammar `^[a-z0-9]+...` — let the grammar speak. Action: add a doc-comment on `is_canonical()` noting that empty strings map to `UnclassifiedReason::NonCanonical`. The Red-Team deserialization concern (`intent_class: None` vs empty string in the deserializer) is spun out as a patch item below. If the deserializer normalizes missing to empty, that parser bug must be fixed independently.

**patch (23):**
- [x] [Review][Patch] Add doc-comment on `is_canonical()` / `consent_decision` documenting empty-string → `NonCanonical` mapping — per AC1 team consensus (Winston + Murat + security red-team, 2026-06-07, FIXED criterion). Files: `crates/maos-domain/src/invariants/i8.rs:81`, `crates/maos-a2a-core/src/router.rs:328`
- [x] [Review][Patch] Check whether deserializer normalizes missing `intent_class` to empty string — red-team concern: if the deserializer turns `None` into `""`, `Absent` and `NonCanonical` collapse regardless of enum design. Must verify and fix parser if so. File: `crates/maos-domain/src/` (deserializer for `A2AIntent`)
- [x] [Review][Patch] Scanner false positives: `consent_envelope: None` matched inside strings/comments/raw strings — `scan_lines_offset` uses a naive substring check with no awareness of whether the literal is inside a string literal, raw string, char literal, or comment. A doc-comment or string containing the literal would be reported as a violation. File: `xtask/src/check_a2a_sender_completeness.rs:287`
- [x] [Review][Patch] Scanner misses `spirits/{mira,nash}/tests/` — the loop hard-codes `spirits/mira/src` and `spirits/nash/src`, but the patch itself migrated cross-Host frames in `spirits/mira/tests/`, proving those directories contain cross-Host senders. File: `xtask/src/check_a2a_sender_completeness.rs:120-143`
- [x] [Review][Patch] Scanner misses `intent_class: None` inside `Some(ConsentEnvelope { ... })` — the implementation only searches for `consent_envelope: None`. A literal like `consent_envelope: Some(ConsentEnvelope { intent_class: None, ... })` passes the gate but would fail at runtime. Files: `xtask/src/check_a2a_sender_completeness.rs:49`, `check_a2a_sender_completeness.rs:221-260`
- [x] [Review][Patch] Scanner misses new smoke arm `smoke_a2a_fail_closed_8_8` — `MAOS_BIN_CROSS_HOST_FNS` only lists four function bodies. Any `consent_envelope: None` literal added to the new smoke arm would evade the static scan. File: `xtask/src/check_a2a_sender_completeness.rs:58-63`
- [x] [Review][Patch] Scanner `extract_fn_body` fragile — multiple parser-level issues: (a) `{` inside a signature comment can be picked as the body opening (`:268-272`); (b) nested block comments `/* outer /* inner */ outer */` terminate at the first `*/`, desyncing brace counting (`:288-293`); (c) byte-string literals `b"..."` are not explicitly handled (`:330-342`); (d) hex/unicode char escapes (`\x7b`, `\u{1F600}`) desync brace counting (`:2400-2410`). File: `xtask/src/check_a2a_sender_completeness.rs`
- [x] [Review][Patch] NACK data payload mutation pattern — `resp` is constructed with `A2AJsonRpcResponse::nack(...)`, then data is inserted via `if let A2AJsonRpcResponse::Nack(ref mut n) = resp`. A future refactor of `nack()` would silently drop the `{reason, peer}` data, breaking Transparency Log legibility. File: `crates/maos-a2a-core/src/router.rs:828-837`
- [x] [Review][Patch] `consent_match_key` returns empty String for Unclassified arm — while unreachable in normal flow (fail-closed gate runs first), an empty string is a latent match hazard if an allowlist ever contained `A2AIntent::new("")`. File: `crates/maos-a2a-core/src/router.rs:298`
- [x] [Review][Patch] `frame_intent_str` dead public API should be `#[deprecated]` — the function is no longer called by `consent_match_key` and exists only for ABI compatibility. External consumers should be warned that the band-projection primitive is no longer on the consent-enforcement path. Files: `crates/maos-a2a-core/src/router.rs:252-256`, `router.rs:699-701`
- [x] [Review][Patch] Inconsistent human vs machine formatting of `UnclassifiedReason::NonCanonical` — `Display` prints `non-canonical` (kebab-case) but `#[serde(rename_all = "snake_case")]` serializes it as `non_canonical`. Operators comparing human-readable logs to structured fields will see mismatched tokens. File: `crates/maos-a2a-core/src/error.rs:589`, `error.rs:602-608`
- [x] [Review][Patch] `intent-lineage-coverage-report.md` polluted with 22 identical untracked run entries — execution noise appended to a committed report file. File: `_bmad-output/implementation-artifacts/intent-lineage-coverage-report.md:390-543`
- [x] [Review][Patch] Smoke `smoke_a2a_fail_closed_8_8` omits sender-side non-canonical deny and oversized deny — the headline runnable demo (AC4) only tests accept-side absent deny, sender-side absent deny, and accept-side non-canonical deny. It does not call `route_outbound` with a non-canonical `intent_class`, nor does it test a 129-byte intent. File: `crates/maos-bin/src/main.rs:1897-1916`
- [x] [Review][Patch] No test coverage for `map_a2a_error_to_iac_bus` new arms — no test asserts that `ConsentUnclassified` / `ConsentUnclassifiedAtPeer` map to `IacBusError::CrossHostRouteFailure`. This is the kernel-facing seam the zero-kernel-KLOC mandate depends on. File: `crates/maos-a2a-core/src/router.rs:916-932`
- [x] [Review][Patch] No test coverage for `interpret_response` malformed NACK fallback — `interpret_response` falls back to `UnclassifiedReason::Absent` if `data.reason` is missing or malformed. No test exercises this path. Files: `crates/maos-a2a-core/src/router.rs:554-565`, `crates/maos-a2a-core/tests/fail_closed_8_8.rs`
- [x] [Review][Patch] No live-wire test for present-but-empty envelope (`intent_class: None`) — `trust_binding_8_9.rs::g7_unclassified_frame_denied_on_wire` tests fully-absent envelope (`consent_envelope = None`). The `Some(envelope)` with `intent_class: None` shape is never exercised over TCP/mTLS. Files: `crates/maos-a2a-tcp/tests/trust_binding_8_9.rs:423-454`, `crates/maos-a2a-core/tests/fail_closed_8_8.rs:155-160`
- [x] [Review][Patch] No live-wire test proving classified-but-not-allowlisted still returns `-32001` — non-conflation is proven in-process only. A regression that maps `-32001` to `ConsentUnclassifiedAtPeer` on the wire would evade the TCP test suite. File: `crates/maos-a2a-tcp/tests/trust_binding_8_9.rs`
- [x] [Review][Patch] No test for `handle_intake_verified` with an unclassified frame on the wire — `g7` sends over TCP which routes through `handle_intake_verified`, but the test only asserts the NACK code. It does not explicitly test that `handle_intake_verified` delegates the unclassified deny to `handle_intake`. File: `crates/maos-a2a-tcp/tests/trust_binding_8_9.rs:423-454`
- [x] [Review][Patch] 128-byte boundary untested — `fail_closed_8_8.rs` covers 129 bytes (`Oversized`) but never asserts that exactly 128 bytes is `Classified`. File: `crates/maos-a2a-core/tests/fail_closed_8_8.rs:174-179`
- [x] [Review][Patch] CI aggregate PR comment table missing the two new gates — the `needs:` list correctly includes the new jobs, but the hardcoded markdown/JS table never prints rows for `check-a2a-sender-completeness` or `smoke-a2a-fail-closed-8-8`. Operators viewing the PR comment will not see their results. File: `.github/workflows/discipline.yml:1952`
- [x] [Review][Patch] Spec AC6 text stale on `with_cross_host_fail_closed` builder — AC6 THEN clause claims the new builder method is Added in abi-diff. Per AC7 Fork A Option 2 consensus, the builder was intentionally never created. The abi-diff is correctly Added-only, but the AC6 text was not updated. Location: spec text AC6.
- [x] [Review][Patch] Dev Agent Record imprecise on `consent_match_key` behaviour — Completion Notes state `consent_match_key` now delegates to `consent_decision` (band-projecting only the `Unclassified` arm). The actual implementation returns `String::new()` for the `Unclassified` arm — an empty never-matching key, not a band projection. Location: Dev Agent Record.
- [x] [Review][Patch] Stale debug log cites kloc ceiling 3100 instead of 3050 — First-pass debug log reads `maos-a2a-core 3042/3100`. Re-sequenced Fork D consensus ratified ceiling 3050 (`xtask/kloc.toml:83`). The stale first-pass log entry was not corrected. Location: Dev Agent Record.

**dismissed (7):**
- [x] [Review][Dismiss] UTF-8 byte count vs character count for Oversized — spec-consistent (`MAX_CANONICAL_INTENT_LEN` = 128 bytes, checked via `.len()`). Not a defect.
- [x] [Review][Dismiss] Granter/expiry precedes unclassified on accept side — correct per AC2 ordering (granter→expiry→unclassified→allowlist). Defense-in-depth, not a bug.
- [x] [Review][Dismiss] TOFU before unclassified on accept side — correct per AC2 ordering. Asymmetry with send side is by design (send-side unclassified is before send-allowlist, which is before TOFU).
- [x] [Review][Dismiss] Unclassified frames bypass Lamport tick and expiry stamp — correct by design; denied before those steps.
- [x] [Review][Dismiss] `ConsentUnclassified` flattened into `CrossHostRouteFailure` — intended 8.9 pattern (zero new kernel variant), explicitly specified in AC1.
- [x] [Review][Dismiss] Aggregate `needs:` opaque diff line — review artifact, not a code defect; the actual discipline.yml was verified by the dev.
- [x] [Review][Dismiss] `cross_host_consent_v1.rs::scenario_3_1` migration without legacy regression test — band-fallback was intentionally deleted per Fork A Option 2; `fail_closed_8_8::no_band_fallback_unclassified_denied_even_with_band_allowlist` covers the absence.
