---
baseline_commit: af788c3e
depends_on: j1-crosshost-1a-frame-borne-delegation (the wire must exist before it can be proven to refuse)
blocked_by: j1-crosshost-1a-frame-borne-delegation
blocks: j1-crosshost-2-cross-host-signed-run
split_from: j1-crosshost-1-loopback-developer-remote-delegation (SCP 2026-07-16 §4.1; split ratified by Lunarpulse 2026-08-14 at preflight)
kernel_grant: NONE — no `maos-kernel-core` line Δ and no ABI Δ. The ABI movement is 1a's (`Mailbox::install_a2a_router`).
kloc_grant: **NONE EXPECTED.** `1a` reclaimed 133 dead `xtask` lines and stood the gate skeleton up inside that; this story adds legs. Re-measure before assuming — if legs overrun the reclaimed margin, request the grant WITH the measurement attached, never on an estimate (`kloc.toml:60-65`).
model: frontier-class {opus-4-8, gpt-5.5, glm-5.2, opus-5, equiv}
review: §A6 full-layer net (Blind + Edge + Acceptance + Test-Infra + runtime) — NON-DEGRADABLE (consent/A2A surface)
---

# j1-crosshost-1b — ADR-012 consent proofs + `check-j1-loopback-delegation` legs

Status: blocked (on `j1-crosshost-1a`) — **round-table consensus applied 2026-08-14**

**Kernel-Δ: ZERO on both axes.** No `maos-kernel-core/src` lines (pin 24472), no ABI surface change —
that movement belongs to 1a. This story's budget risk is entirely `xtask`.

<!-- Model/review: frontier-class dev allowlist (E11 retro A1 / E12-B3). §A6 full-layer net
     NON-DEGRADABLE per E12-B6 — consent/A2A is the row epic-10-process-agreements.md:15 marks
     mandatory. NOTE: five story-file gates skip this filename (digit-prefix scoping) — §A6 is
     cultural, not mechanical, here. See AC3.4. -->

> **Why this story is separate.** `1a` builds a wire that carries a canonical
> `development-task:write-workspace` intent and delivers. This story proves the wire **refuses** —
> with the right error codes — and fills out the gate that keeps it refusing.
>
> **Split hazard, and how the round-table closed it (2026-08-14).** This story judges machinery `1a`
> shipped — the shape that forced the 13.6 → 13.6a/13.6e and 12.4 → 12.4a/12.4b corrections. The
> original mitigation was an in-crate test in `1a`, which the room rejected: *a test that isn't
> behind a gate is a suggestion* — it rots, someone relaxes a red assertion, and the regression walks
> through, exactly as `smoke_cli_wrapper_8_12` did. **So `1a` now lands the gate skeleton itself** —
> registered, `Blocking`, enrolled in all seven surfaces, with one leg that IS the proven-red — and
> **this story adds legs to a gate that already blocks.** The seam stops being "the student built the
> grader" and becomes "1a built the frame, 1b built the exam."
>
> The budget objection that originally justified the split dissolved on measurement: `1a` funds the
> skeleton by deleting `xtask/src/example_spirit_regen.rs` (133 code lines, zero callers), so no
> grant was ever requested on an estimate and `kloc.toml:60-65` stands unbent.

---

## Corrected facts — read this block, then the ACs

1. **A disallowed intent is `-32001 CODE_INTENT_DENIED` / `IntentDeniedAtPeer`.** The ratified card
   said to reuse the "smoke-8-8 shape" — **smoke-8-8 asserts `-32009` and explicitly FAILS on
   `-32001`** (`main.rs:11166-11169`: *"got -32001 (classified-denied) — conflation defect"*). The
   correct precedent is **`smoke-a2a-consent-vocab-8-7`** (`main.rs:10968-11007`).
2. The separation of the two codes is deliberate and pinned by
   `crates/maos-a2a-core/tests/fail_closed_8_8.rs:215` — *"classified-but-not-allowlisted must be
   -32001, never -32009."*
3. **On loopback the allowlist is keyed by the SOURCE `host`, not the destination.** Intake resolves
   the peer by `frame.from.host_id`, falling back to `HostId("loopback")` when `None`
   (`router.rs:1087-1090`). Documented in shipped code at `main.rs:10860-10864` and
   `spirits/mira/tests/a2a_pairing.rs:8-10`.
4. **The gate exists as a skeleton when this story opens** — `1a` lands it registered, `Blocking`,
   enrolled in all seven surfaces, with one leg (the proven-red) plus a boundary leg. **This story
   adds legs; it does not stand a gate up.** Verify that before writing: `grep -rn
   "check-j1-loopback-delegation" xtask/ .github/` should return the module, the dispatch arm, the
   registry rows and the job — if it returns only the sprint-status line, `1a` did not finish.
5. **`EXPECTED_GATES` is hand-maintained** — 36 against 67 `check_*.rs` pre-`1a`, **37 / 68** once
   the J1 gate is enrolled. An unenrolled gate reds nothing. `1a` did the enrollment; you verify it,
   and D11's row is being corrected to those numbers (AC3.6).
6. **`xtask` slack is whatever `1a` left after the reclaim.** Pre-`1a` it was ~29 tokei-code lines
   (37287 − 37023 = 264 raw, and `check_vetting_attestation.rs` is **235 code** lines, not its 273
   raw); `1a` then deleted 133 dead lines and spent ~52-69 on the skeleton. **Re-measure at your
   HEAD — do not inherit either number.** kloc excludes `tests/`, so `xtask/tests/` legs are free.
7. **`smoke_cli_wrapper_8_12.rs` has no CI invocation.** CI runs `-p worker`
   (`discipline.yml:815`) and `-p maos-journey-test` (`:1869`) on this path.
8. Baseline is **24472**. `kloc-check` is **RED at HEAD** on `maos-kernel-core` — pre-existing, not
   yours.

---

## Story

**As** Lunarpulse, relying on the founder loop's delegation path,
**I want** the loopback A2A wire to provably refuse a disallowed intent with the correct error code,
guarded by a blocking CI gate that reds when the routing is bypassed,
**so that** the `developer-remote` leg cannot silently regress into local delivery, and rung 2
inherits a `PROVEN_BLOCKING` foundation rather than a claim.

---

## Acceptance Criteria (3)

### AC1 — ADR-012 refusal proofs, with the two deny codes kept distinct

1. **Positive control first.** The allowlisted `development-task:write-workspace` frame is delivered and the
   delivered frame's `intent_class` is asserted equal (the smoke-8-8 structure,
   `main.rs:11121-11146`). Without a working positive the negatives prove nothing.
2. **Allowlist keying.** The admitting entry lives in the peer config keyed by the **SOURCE** `host`
   (`router.rs:1087-1090`). **Name the two expected `peer_id` strings in the test** so the assertion
   is checkable. An AC phrased *"Host B's accept_allowlist admits X"* would be wrong on loopback.
3. **STATED NON-COVERAGE — rung 1 does not exercise peer authentication.** *(Consensus 2026-08-14;
   this must appear in the story record, not only here.)* On the TCP path
   `handle_intake_verified` binds `frame.from.host_id` to the **TLS-verified** peer. On loopback
   there is nothing to bind it to — `router.rs:1478-1479` says so outright: the loopback router calls
   `handle_intake` directly because there is *"no wire identity to bind."* Therefore **the field that
   selects which `accept_allowlist` applies is written by the sender and never verified.** A frame
   picks its own judge.
   In-process this is survivable — one address space, no attacker. What is **not** acceptable is the
   inherited claim that rung 1 "proves the wire so rung 2 only adds network." It does not: rung 1
   proves the wire **with authentication stubbed out**, and every refusal proved in AC1.4/AC1.5 is
   one string assignment away from selecting a different allowlist. Write this sentence into the
   story record and into the `j1-crosshost-2` sprint-status row as an explicit inheritance, so rung
   2's preflight cannot mistake a partial proof for a whole one. `1a`'s AC4.4 lands the matching
   four-line gate leg that makes the gap visible in CI rather than only in prose.
4. **Disallowed-intent negative → `-32001 CODE_INTENT_DENIED` / `A2AError::IntentDeniedAtPeer`**,
   following `smoke-a2a-consent-vocab-8-7` (`main.rs:10968-11007`). An `Ack` is an explicit failure.
   **Do not copy smoke-8-8** — it fails on this code.
5. **Unclassified negative, kept distinct → `-32009 CODE_CONSENT_UNCLASSIFIED`** at intake and
   `ConsentUnclassified{Send}` at the sender (`router.rs:696-701`). Assert the two codes are **not**
   conflated; that separation is the point of `fail_closed_8_8.rs:215`.
6. **Send-side and accept-side are separate seams** and both are asserted: `send_admits`
   (`router.rs:748-760`) and `accept_admits` (`router.rs:1312-1338`).
7. **Envelope expiry is a known gap, not a defect to file:**
   `ConsentEnvelope::with_fine_grained_intent` hard-codes `timestamp_ns = 0`, `valid_until_ns = None`
   (`maos-domain/src/frame.rs:447-450`) — the envelope never expires. Either set a TTL here or state
   the gap explicitly in the record.

### AC2 — Fill out `check-j1-loopback-delegation`: consent legs on a gate that already blocks

*(Consensus 2026-08-14: `1a` stands the gate up; this story adds legs. The AC below was originally
written as "create the gate" — it is not, and a dev who creates a second one will collide with the
skeleton's enrollment rows.)*

1. **Extend** `xtask/src/check_j1_loopback_delegation.rs` — created by `1a`, structured on
   `xtask/src/check_vetting_attestation.rs` (**235 tokei-code lines** — see AC3.1). It already
   carries `BindingClass::Blocking` with
   `dev_enforced_red_blocks(BindingClass::Blocking, true)`, so a RED reds CI **at HEAD**, independent
   of `CURRENT_PHASE = "v1_5"` (`gate_common.rs:34`, `:97-102`). **Do not re-declare the module,
   the dispatch arm, or the registry rows.** Add legs for AC1's positive control and both negatives.
2. **Vacuous-green guard**, verbatim shape from `check_vetting_attestation.rs:224-235`: any leg with
   `!ran || (passed == 0 && failed == 0)` is a hard FAIL.
3. **Anti-canned, the 8.12 Tier-1 shape:** per-run nonce
   `format!("maos-nonce-{}-{}", std::process::id(), nanos)` (`cli_wrapper_bridge_8_12.rs:20-27`),
   echoed *through* the child, asserted present in the journaled row alongside
   `"child_pid":{child_pid}` and `assert_ne!(child_pid, std::process::id())` (`:53-98`).
4. **Proven-red, extended to the consent legs.** `1a`'s skeleton already proves "route locally
   anyway" reds the gate. This story adds the consent counterparts: a planted regression that
   **admits a disallowed intent**, and one that **collapses `-32001` into `-32009`**, must each RED.
   Asserted by the `xtask/tests/` suite `1a` created, invoked as **its own CI step** following the
   `story_10_5_proven_red` idiom (`discipline.yml:2567`). A proven-red only a human ran is not a
   control. (`xtask/tests/` costs **zero** kloc — put as much here as possible.)
5. **Close the null control.** Add `crates/maos-bin/tests/smoke_cli_wrapper_8_12.rs` to a CI job so
   `ci_local_split_refuses_a_granted_real_agent_without_the_live_flag` actually executes. Today the
   "CI physically cannot spawn a paid agent" claim holds **by env-var omission, not by an executed
   assertion**. **Any new test not invoked by the gate and not in `-p worker` / `-p maos-journey-test`
   is dead in CI** — the same hole this AC exists to close.
6. **Live-`codex` leg is `MAOS_LIVE_AGENT=1`, local-only, NEVER CI.**
7. **Enrollment — VERIFY, do not repeat.** `1a` enrolled the gate in all seven surfaces. This story
   confirms each is present and correct, and touches one only if `1a` left it wrong.
   (`EXPECTED_GATES` is hand-maintained at **36** against **67** `check_*.rs` pre-`1a`, **37 / 68**
   after — and D11's row is being corrected to match, see AC3.6.) The seven:
   (a) `xtask/src/main.rs` — `mod`, `#[command(name = …)]`, dispatch arm;
   (b) `.github/workflows/discipline.yml` — job + `v1-0-ship-gate` `needs:` + both echo tables;
   (c) `xtask/gate-registry.toml` — flat `gates` list **and** a `[[ship_gate]]` disposition block;
   (d) `xtask/src/check_ship_gate_completeness.rs` — `EXPECTED_GATES`;
   (e) `tests/coverage-matrix.yaml` — add the gate to the **existing `FR23a` row at `:472-477`**;
   there is **no ADR-012 row** — do not invent one. **(c) must land before (e)**:
   `coverage_matrix.rs:140-144` reds if a row names a gate absent from the flat list;
   (f) `crates/maos-bin/src/env_contract.rs` — any new env var;
   (g) `xtask/src/lib.rs` — **only if** the proven-red suite calls the gate as a library.
8. **Two CI tripwires the job must not trip:** `check-loom-substrate-drift` (`discipline.yml:698-711`)
   reds any job declaring `services.postgres` not in `CONTRACTS` — keep the job hermetic; and
   `check-epic-close-green` (`:69-100`) forbids job-level `if: false`.

### AC3 — Budget honesty, measured rather than estimated

1. **No grant is expected, and none may be requested on an estimate.** kloc counts tokei **`code`**
   lines and **excludes** `tests/`, `benches/`, `examples/`, `spirits/`
   (`xtask/src/kloc_check.rs:167-190`). Pre-`1a` slack was **264 − 235 ≈ 29 code lines**; `1a` then
   reclaimed **133** dead lines (`example_spirit_regen.rs`) and spent ~52-69 on the skeleton, so this
   story should open with real margin. **Re-measure at your HEAD — inherit no number from this
   paragraph.** Per `kloc.toml:60-65` a grant is authorized after measurement, never on an estimate;
   `kloc.toml:204-210` records the **seventh** consecutive `xtask` re-base with the growth rate
   itself unratified (D11), so an eighth needs a reason, not a reflex. **If the legs do not fit:
   move them to `xtask/tests/` (free), shrink them, or request a named grant WITH the measurement
   attached — never silently re-base.**
2. **Both kernel axes stay flat.** `check-kernel-baseline` green at 24472, and `abi-diff` /
   `check-abi-ratification` show no delta from this story (1a owns the `Mailbox` method).
3. **Do not absorb the pre-existing red.** `kloc-check` is RED at HEAD on `maos-kernel-core`
   (18933 > 18248) — the Epic-5 closure's unrebased ceiling, **not in scope**. Measure before and
   after; attribute honestly.
4. **Five story-file gates skip this filename** (digit-prefix scoping), so the dev record, Review
   Findings table, File List **and** model attribution are mechanically unenforced here:
   `check-dev-model-tier` (`:103`), `check-dev-model-used-populated` (`:136-138`),
   `check-bare-review-findings` (`:35-37`), `check-dev-record-completeness` (`:240-250`),
   `check-review-findings-resolved` (`:50-63`). (`check-epic-close-coherence:213-214` treats
   `j1-crosshost-*` as epic-less by design — that one is correct.) Operator decision 2026-08-14:
   **keep the `j1-` prefix and state the gap** rather than rename. Record the model and review
   artifact anyway; a green CI does not mean the net ran.
5. **On completion:** set this story's `sprint-status.yaml` row to `done`, and write AC1.3's
   peer-authentication non-coverage into the `j1-crosshost-2` row as an explicit inheritance. Note
   `check_dev_record_completeness.rs:534-552` — once a story flips to `done`, any `deferred-work.md`
   row naming it as owner becomes `Stale` and reds. Rung 2's evidence contract expects this rung to
   read **`PROVEN_BLOCKING`** (`gate_common.rs:139-146`).
6. **Confirm `1a` filed the D11 amendment; do not re-file and do not close it.** `1a`'s AC5.6 owes
   `epic-14-preflight-decisions.md` three things: D11(b) widened to *"budget-charged code with no
   execution path"*, the 36/66 → 37/68 correction, and the clippy-never-invoked note. **The 37/68
   figure only becomes true once this story's legs land and the gate is fully enrolled** — verify the
   row matches reality at close. D11 stays **open**; its owners (Winston + Murat) settle substance at
   14-6's preflight. Binding rule 1: shipping adjacent work does not close a row.

---

## Traps

1. **`cargo run -q -p xtask -- <cmd>`.** There is no `cargo xtask` alias.
2. **Do not copy smoke-8-8 for the disallowed-intent negative** — it asserts `-32009` and fails on
   `-32001`. Copy **smoke-8-7**.
3. **`-32001` and `-32009` must not be conflated.** That separation is a shipped invariant.
4. **The loopback allowlist is keyed by the SOURCE host**, not the destination.
5. **`task.assign` is not a legal consent intent** — the vocabulary is
   **`development-task:write-workspace`** (defined in 1a; ADR-012 names effect authority, not verbs).
5b. **The gate already exists when you start.** `1a` landed the skeleton and all seven enrollment
    rows. Creating a second module or a second registry entry collides.
6. **kloc is tokei `code` lines, not raw lines.** 273-line files are 235 code lines. Budget in the
   right unit or the grant request will be wrong.
7. **`xtask/tests/` costs zero kloc** — put as much of the proven-red harness there as possible.
8. **A gate that exists only in `discipline.yml` is invisible to every meta-gate.** Seven surfaces.
9. **Registry (c) before coverage-matrix (e)** or `coverage_matrix.rs:140-144` reds.
10. **There is no ADR-012 row in `tests/coverage-matrix.yaml`** — use the existing FR23a row at
    `:472-477`. Note the matrix is `mode: warning` (`:3`) so it cannot fail today.
11. **Keep the CI job hermetic** — a `services.postgres` block reds `check-loom-substrate-drift`.
12. **No job-level `if: false`** — `check-epic-close-green` forbids it.
13. **A test not run by CI is not a control.** CI runs `-p worker` and `-p maos-journey-test` here.
14. **The working tree may be dirty.** Establish a clean baseline measurement before attributing
    kloc movement.

---

## Tasks

- [ ] **T1 (AC1.1-1.2)** — Positive control + named `peer_id`s for the source-keyed allowlist.
- [ ] **T2 (AC1.3-1.5)** — `-32001` disallowed negative (smoke-8-7 shape); `-32009` unclassified
      negative at both send and accept seams; assert non-conflation.
- [ ] **T3 (AC1.6)** — Decide envelope TTL: set one, or record the never-expires gap.
- [ ] **T4 (AC1.3)** — Write the peer-authentication non-coverage into the story record; confirm
      `1a`'s boundary leg exists in the gate.
- [ ] **T5 (AC2.1-2.3)** — **Extend** `1a`'s `check_j1_loopback_delegation.rs` with the consent legs
      (do NOT re-create the module/dispatch/registry). **Measure code lines on completion.**
- [ ] **T6 (AC2.4)** — Add consent proven-reds to `1a`'s `xtask/tests/` suite: admitting a disallowed
      intent must RED, and collapsing `-32001` into `-32009` must RED.
- [ ] **T7 (AC2.5)** — Add `smoke_cli_wrapper_8_12` to a CI job; verify the live-split blind executes.
- [ ] **T8 (AC2.7-2.8)** — **Verify** the seven enrollment surfaces `1a` landed; fix only what is
      wrong. Hermetic job, no `if: false`.
- [ ] **T9 (AC3.1)** — Re-measure `xtask` at HEAD; only if the legs overrun, request a grant **with
      the measurement attached** or move legs to `xtask/tests/` (free).
- [ ] **T10 (AC3.2-3.3)** — Before/after `check-kernel-baseline`, `abi-diff`,
      `check-abi-ratification`, `kloc-check`; attribute movement honestly.
- [ ] **T11 (AC3.5-3.6)** — Sprint-status row → `done`; write the peer-auth inheritance into the
      `j1-crosshost-2` row; verify `1a`'s D11 amendment reads 37/68 and that **D11 is still open**;
      check `deferred-work.md` for rows naming this story as owner.

### Review Findings

_(populated by §A6 review)_

---

## Dev Notes

### Budget — the whole reason this story is separate

| Instrument | Ceiling / pin | Measured at HEAD | State |
|---|---|---|---|
| kloc `xtask` | 37287 | pre-`1a` 37023 | `1a` reclaimed −133, spent ~52-69 on the skeleton — **re-measure at HEAD** |
| `xtask/tests/` | — | — | **zero kloc cost — prefer it** |
| `check-kernel-baseline` | 24472 | 24472 | GREEN — no change from this story |
| `abi-diff` / `check-abi-ratification` | — | — | No delta from this story (1a owns it) |
| kloc `maos-kernel-core` | 18248 | 18933 | **RED, pre-existing — not this story's** |
| kloc `maos-domain` | 8644 | 8694 | RED; inside the uncommitted Story-3.3 lines |

`kloc.toml:60-65`: recalculated **only** at an epic retrospective or under an explicitly authorized
measured grant. *"Slack is operating capacity, NOT authorization."*

### Gate anatomy — the template to copy

`xtask/src/check_vetting_attestation.rs` is the closest hermetic-only match:
`GATE_NAME` const → `struct LegResult` → `invoke_leg()` building
`cargo test --locked -p <pkg> --test <file> -- <exact_name> --exact --nocapture` →
`green = status.success() && ran && passed >= 1 && failed == 0` (`:87`) → `run(json)` doing
`read_disposition` → `is_blocking_at` → `dev_enforced_red_blocks` → legs → **vacuous guard
(`:224-235`)** → JSON payload → `Ok(())`. Gates return `Result<(), String>`; `xtask/src/main.rs:1301-1305`
converts `Err` to `exit(1)`. There is no third exit code.

### Where the code goes

| Concern | File | Anchor |
|---|---|---|
| Allowlist enforcement (send) | `crates/maos-a2a-core/src/router.rs` | `send_admits` :633-648, call :748-760 |
| Allowlist enforcement (accept) | `crates/maos-a2a-core/src/router.rs` | `accept_admits` :650-665, call :1312-1338 |
| Deny codes | `crates/maos-a2a-core/src/transport/json_rpc.rs` | `CODE_INTENT_DENIED` :30 |
| Refusal precedent to copy | `crates/maos-bin/src/main.rs` | `smoke_a2a_consent_vocab_8_7` :10968-11007 |
| Non-conflation invariant | `crates/maos-a2a-core/tests/fail_closed_8_8.rs` | :215-240 |
| Anti-canned nonce/PID | `crates/maos-kernel-core/tests/cli_wrapper_bridge_8_12.rs` | :20-27, :53-98 |
| New gate | `xtask/src/check_j1_loopback_delegation.rs` | NEW |
| Gate template | `xtask/src/check_vetting_attestation.rs` | whole file (235 code lines) |
| Binding classes | `xtask/src/gate_common.rs` | :34, :76-102, :139-146 |
| Enrollment (c) | `xtask/gate-registry.toml` | flat list + `[[ship_gate]]` |
| Enrollment (d) | `xtask/src/check_ship_gate_completeness.rs` | `EXPECTED_GATES` :20-83 |
| Enrollment (e) | `tests/coverage-matrix.yaml` | existing FR23a row :472-477 |
| Proven-red idiom | `.github/workflows/discipline.yml` | :2567 (`story_10_5_proven_red`) |

### What this story does NOT do

- **No mechanism** — the wire, the router install, the pump, and the env deletion are `1a`.
- **No new protocol**, no mTLS, no second host — `j1-crosshost-2`.
- **No enforced egress** — stays `declared-not-enforced`.
- **No fix for the pre-existing `maos-kernel-core` kloc red.**
- **No widening of the digit-prefix gate scoping** — that is Epic 14's instrument work (D11).

### References

- [Source: `_bmad-output/planning-artifacts/sprint-change-proposal-2026-07-16.md#4.1`] — the ratified card (its AC4 refusal shape is corrected here).
- [Source: `_bmad-output/implementation-artifacts/j1-crosshost-1a-frame-borne-delegation.md#appendix-a`] — the shared 12-premise refutation.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/7-inter-agent-communication.md#7.2`] — fail-closed-on-unclassified; `-32009` vs `-32001`; no auto-retry.
- [Source: `docs/adr/ADR-012-typed-intent-a2a-consent.md`] — `(peer-identity, intent-class)`; the ADR names no allowlist keys.
- [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md#FR23a`] — loopback A2A corpus floors (30 consent scenarios, 100% disallowed blocked).
- [Source: `xtask/kloc.toml#60-65`] — grants are measured, never estimated.
- [Source: `xtask/src/gate_common.rs#139-146`] — `PROVEN_BLOCKING` / `ABSENT` / `INDETERMINATE` wire spellings rung 2 consumes.

---

## Dev Agent Record

### Agent Model Used

_(record `vendor/model` + harness + date — required by policy even though five gates skip this
filename; see AC3.4)_

### Debug Log References

### Completion Notes List

### File List

---

## Change Log

| Date | Change |
|---|---|
| 2026-08-14 | Created by splitting `j1-crosshost-1-loopback-developer-remote-delegation` (split ratified by Lunarpulse at preflight). Carries the judge: consent refusal proofs + the blocking gate + the deferred, measured `xtask` grant. Corrects the ratified AC4's refusal shape (`-32001` via smoke-8-7, **not** smoke-8-8's `-32009`) and its allowlist keying (SOURCE host on loopback). Absorbs the D-10 null control (`smoke_cli_wrapper_8_12` never runs in CI). |
| 2026-08-14 | **Round-table consensus applied.** (1) **The gate skeleton moved to `1a`** — a test not behind a gate is a suggestion, so this story now *adds legs* to a gate that already blocks rather than standing one up; AC2 and its tasks rewritten accordingly, and re-creating the module/dispatch/registry is now a named trap. (2) The budget rationale for the split **dissolved on measurement** — `1a` funds the skeleton by deleting 133 dead `xtask` lines, so no grant is requested on an estimate and AC3.1 drops from "request the grant" to "re-measure, and only ask with the measurement attached." (3) **NEW AC1.3 — stated non-coverage: rung 1 does not exercise peer authentication.** `handle_intake` binds no wire identity on loopback (`router.rs:1478-1479`), so `frame.from.host_id` — the field that *selects which `accept_allowlist` applies* — is sender-written and unverified. Every refusal here is one string assignment from selecting a different allowlist. Must be written into the `j1-crosshost-2` row as an explicit inheritance; `1a` lands the matching CI boundary leg. (4) Intent is now `development-task:write-workspace`. (5) NEW AC3.6 — verify `1a`'s D11 amendment reads 37/68 and that **D11 remains open**. |
