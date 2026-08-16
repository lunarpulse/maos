---
baseline_commit: 5a921c0c
depends_on: j1-crosshost-1a-frame-borne-delegation (done, `6827dc87` — the wire must exist before it can be proven to refuse)
blocks: j1-crosshost-2-cross-host-signed-run
split_from: j1-crosshost-1-loopback-developer-remote-delegation (SCP 2026-07-16 §4.1; split ratified by Lunarpulse 2026-08-14 at preflight)
kernel_grant: "NONE — `check-kernel-baseline` GREEN at 24472. **But do not cite `abi-diff` as evidence:** it scopes to `crates/maos-spirit-abi/Cargo.toml` ONLY (`xtask/src/abi_diff.rs:8`), so it is structurally blind to `maos-kernel-core`'s re-exported surface — that is open **FLAG-E4**. This story expects zero ABI movement anyway (`1a` owns `Mailbox::install_a2a_router`); the honest statement is 'no ABI change was made', not 'the gates confirm none'."
kloc_grant: "**RATIFIED AND ALREADY APPLIED — do not re-take it.** Decision **D15** is RESOLVED: Lunarpulse ratified **`maos-bin = 16219`** (exact measured, **ZERO headroom**) on 2026-08-15, over the formula's 16544; it is live at `xtask/kloc.toml:264` and `maos-bin` is GREEN at HEAD. `xtask` needs no grant (headroom **+534**) and this story adds **+0** to `maos-bin/src`. **The `_aggregate_hardfail` grant is REFUSED deliberately** (filed **D17**). Your job is to keep the number true, not to request it — see AC3.2."
model: frontier-class {opus-4-8, gpt-5.5, glm-5.2, opus-5, equiv}
review: §A6 full-layer net (Blind + Edge + Acceptance + Test-Infra + runtime) — NON-DEGRADABLE (consent/A2A surface)
---

# j1-crosshost-1b — ADR-012 consent refusal proofs + `check-j1-loopback-delegation` legs

Status: **ready-for-dev**

`j1-crosshost-1a` is `done` (`6827dc87`). The wire this story judges exists, and the gate it adds
legs to is registered, `Blocking`, and **PASSING at HEAD** with two legs.

**Kernel-Δ: ZERO expected on both axes.** `check-kernel-baseline` is GREEN at 24472 — that one is
measured evidence. The ABI axis is **not** evidenced by a gate: `abi-diff` and
`check-abi-ratification` scope to `crates/maos-spirit-abi` only and are blind to `maos-kernel-core`'s
re-exported surface (open **FLAG-E4**, see F11). This story makes no ABI change; report that as a
fact about the diff, not as a gate result. Any kernel seam is FLAG-Winston, never an implicit re-pin.

> **What this story is.** `1a` built a wire that carries a canonical
> `development-task:write-workspace` intent and delivers. This story proves the wire **refuses** —
> with the *right* error codes, kept distinct — and adds the legs that keep it refusing.
>
> **The split hazard, and how it was closed.** This story judges machinery `1a` shipped — the shape
> that forced the 13.6 → 13.6a/13.6e and 12.4 → 12.4a/12.4b corrections. `1a` therefore landed the
> gate **skeleton** itself, so this story adds an exam to a grader that already blocks rather than
> building both. That is confirmed true at HEAD (AC2 preamble).

---

## ⚠ Read this block before the ACs — the draft this replaces was wrong in thirteen places

Every line number below was re-derived at `5a921c0c` by reading the file. The previous draft was
written against `af788c3e`, and `246660f9` reformatted the tree, so **inherit no line number from
memory or from an older story.**

### The five findings that change what you build

**F1 — The gate is a STATIC SOURCE-TEXT ORACLE. It runs no tests.**
There is no `Command`, no `cargo`, no `--test`, no test name anywhere in
`xtask/src/check_j1_loopback_delegation.rs` (219 tokei CODE lines). Both legs read six committed
files (consts at `:55-60`) and match structural needles against them. The behavioural `cargo test`
commands live in **CI**, at `.github/workflows/discipline.yml:1819-1821`.
*Consequence:* "add consent legs to the gate" means **add source-structural checks**, plus wire the
behavioural test file into the gate's CI job. It does **not** mean make the gate shell out.

**F2 — There is NO vacuous-green guard in this gate — and no shared one to reach for.**
The previous draft told you to copy `check_vetting_attestation.rs:224-235`. That guard is in a
*different* gate. This gate's entire aggregation is `let oracle_green = findings.is_empty();`
(`:306`) — a leg that reads nothing and pushes nothing is indistinguishable from a leg that passed.
There is no per-leg `ran`/`passed`/`failed` record to guard.
**Measured 2026-08-15: 16 of 68 `check_*.rs` gates carry a vacuity guard; `gate_common.rs` carries
none; all 16 are hand-rolled separately.** Writing a 17th bespoke guard here is how that number
reached 16 — so AC2.2 lands the ~20-line primitive in `gate_common` and makes this gate its first
consumer. Migrating the other 15 is 14-6's, not yours.

**F3 — SHIP-BLOCKER: a `cargo`-invoking leg silently destroys the proven-red harness.**
`xtask/tests/j1_crosshost_1a_proven_red.rs` runs the xtask **binary** (`:92`,
`env!("CARGO_BIN_EXE_xtask")`) with `.current_dir(dir.path())` (`:94`) where `dir` is a **tempdir
containing six stub source files and no `Cargo.toml`**. `check_vetting_attestation::invoke_leg`
(`:59-66`) builds `Command::new("cargo")` and **never sets `current_dir`**, so it would inherit that
tempdir. If you copy that template: `baseline_fixture_tree_is_green` (`:131-133`) goes red, and all
planted vectors keep "passing" **for the wrong reason** — the gate is red no matter what is planted.
The suite would still report green in CI while proving nothing. **Every new leg stays root-relative
and source-static, honouring `run_with_root(json, root)` (`:299`).**

**F4 — The story's headline assertion is UNREACHABLE through the production `DelegationLeg`.**
`DelegationLeg::install(mailbox, intent)` (`crates/maos-bin/src/delegation.rs:102-131`) builds *both*
endpoints from the **same single intent**:
`LoopbackEndpoint::sender_of(TO_HOST, 7451, intent)` and `acceptor_of(FROM_HOST, 7452, intent)`.
So **any** intent mismatch is caught by `send_admits` (`router.rs:749`) *before the frame reaches the
wire*, yielding `A2AError::IntentDenied{direction: Send}` — **not** `-32001` / `IntentDeniedAtPeer`.
To produce `-32001` the intent must be **in the destination's `send_allowlist` but absent from the
source's `accept_allowlist`**. `LoopbackEndpoint`'s four fields are all `pub`, so you build that
asymmetry directly — no production change needed. See AC1.2.

**F5 — The two deny codes are CONFLATED above the router, so the non-conflation assertion must be
made at the router seam.**
`map_a2a_error_to_iac_bus` (`crates/maos-a2a-core/src/router.rs:1671-1783`) loses the vocabulary,
though **not uniformly — be precise about which half** (measured 2026-08-15 for D18):
- **The `-32001` half survives, partially.** `IntentDenied{Send}` (`:1673-1683`) and
  `IntentDeniedAtPeer` (`:1684-1690`) both produce `CrossHostIntentDenied`, but with
  `direction: Send` vs `Accept`, so the two seams *are* still tellable apart by field. The defect
  there is that `IntentDeniedAtPeer` stuffs the NACK **message** into a field named `intent`.
- **The `-32009` half is destroyed.** Both `ConsentUnclassified` variants collapse into stringly
  `IacBusError::CrossHostRouteFailure` (`:1773-1782`), discarding the typed `UnclassifiedReason`
  (`Absent`/`NonCanonical`/`Oversized`) and the direction outright.
`DelegationLeg::delegate` (`delegation.rs:149-171`) then stringifies even that into a `String`.
*Consequence, unchanged and load-bearing:* a typed `-32001`-vs-`-32009` assertion is **impossible
above `A2ARouterCore`** — one side keeps a variant, the other becomes a sentence, so there is nothing
to compare. Assert at the router seam. The lossy mapping is itself a finding — this story **files it as D18** with an
owner and a deadline (AC1.5b), rather than addressing it to `j1-crosshost-2`, which has no story
file to receive it. It does not fix it.

### Eight corrections you would otherwise carry forward as facts

**F6 — Envelope expiry is NOT a gap. The previous AC1.7 rested on a dead premise.**
`ConsentEnvelope::with_fine_grained_intent` (`crates/maos-domain/src/frame.rs:447-465`) does build
`timestamp_ns: 0, valid_until_ns: None` — but `prepare_outbound` **stamps a TTL** on any envelope
carrying `None` (`router.rs:866-871`, Story 8.9/G10: *"expiry was dead code on every real frame"*),
and `handle_intake_inner` **enforces it** (`router.rs:1208-1222`) with a **third** code,
`CODE_CONSENT_EXPIRED` (`-32003`). There is nothing to "either set or record". What there IS: a
third code that must also stay distinct. See AC1.4.

**F7 — Both `smoke-*` precedents are `#[cfg(feature = "network")]` binary-only arms with ZERO CI
invocation.** `smoke_a2a_consent_vocab_8_7` is `main.rs:10963-11155` (dispatch `:8183`);
`smoke_a2a_fail_closed_8_8` is `:11157-11420` (dispatch `:8189`). Neither is `cargo test`-reachable
and no workflow invokes either. **Copying "the smoke-8-7 shape" as a smoke reproduces a null
control.** Copy its *assertions* into a real integration test.
Also: the previous draft's "positive control at `main.rs:11121-11146`" points **inside the deny
arm**. The real positive control is `:11081-11108`. And `smoke-8-8`'s `-32001`-is-a-failure assertion
is at `:11292-11322`, not `:11166-11169` (that range is doc prose).

**F8 — The budget block was stale by a full grant.** Ceiling is **38609** (`kloc.toml:203`, raised by
`j1-demo-one-command-scene` — the **eighth** consecutive `xtask` re-base), measured **38075**,
headroom **+534** — not "~29 lines". The entire "budget is the reason for the split" argument is
dead. Conversely the story understated the reds: **four** breaches at HEAD, not one (AC4).

**F9 — `xtask/src/lib.rs` is not a seventh enrollment surface.** It exports 8 modules and this gate
is correctly absent: the proven-red suite drives the **binary**, not the library. There are **six**
real surfaces, all correct at HEAD. Adding a `pub mod` would grow the xtask public surface for no
control.

**F10 — `deferred-work.md` contains ZERO rows naming this story.** A case-insensitive sweep of all
807 lines finds no `j1-crosshost-1`, `-1a`, `-1b`, or `j1-demo` owner. `check-dev-record-completeness`
is GREEN (`violation_count: 0` over 24 owner assertions). The stale-owner-on-close hazard is
**unfounded here** — do not plan around it.

**F11 — The ABI gates cannot see the surface this lane moves (FLAG-E4).** `abi-diff` and
`check-abi-ratification` scope to `crates/maos-spirit-abi` **only** (`xtask/src/abi_diff.rs:8`
`MANIFEST`). `1a` recorded this after discovering *"**None can see the new `Mailbox` method**"*.
`maos-kernel-core/src/iac.rs:13` is `pub use maos_iac::*`, so kernel-core's public API can grow at a
flat `src_lines` with every gate green. **Their passing is not evidence.** This story makes no ABI
change; say that, and do not launder it through a blind gate.

**F12 — `cargo test -p maos-bin` is RED at HEAD under default parallel flags (D16).** A suite-wide
test-isolation defect: `std::env::set_var("MAOS_HOME", …)` is process-global and locking is
inconsistent across `cross_team_consent_13_3.rs`, `cross_team_crossing_13_6b.rs` and
`cross_wall_log_read_13_6d.rs`. `cross_wall_recall_live_path_uses_verified_state_and_home_team`
fails 5/5 parallel, passes 3/3 with `--test-threads=1`. **This will bite you the moment you run the
crate's suite.** Run your file scoped (`--test consent_refusal_1b`) and do not "fix" the flake — D16
routes the whole-suite decision to 14-0, and `--test-threads=1` is recorded as a masking workaround,
never a resolution.

**F13 — Line numbers and two structural claims corrected.** `accept_admits` is `router.rs:648-665`
(not `:650`). The `story_10_5_proven_red` idiom is `discipline.yml:2598-2599` (not `:2567`, which is
a Windows-sandbox comment). `-p maos-journey-test` is `:1901` (not `:1869`). The "documented at
`main.rs:10860-10864`" citation for allowlist keying is wrong — that is `smoke-6-3`; the real
documentation is `crates/maos-a2a/src/pairing.rs:9-17` and `crates/maos-bin/src/delegation.rs:58-65`.
Two claims about *shape*, not just position, were also wrong:
- The demo suite asserts **`beats.len() == 9`** at `xtask/src/tests/demo_j1_tests.rs:218` — not 7.
- `1a`'s boundary leg is **28 lines (`:266-293`) that push a `Finding` and RED the gate when the
  boundary moves** (`:282-291`) — not "a 4-line leg that merely records". Do not describe it as
  passive; it has teeth, and rung 2 flips it from *documented gap* to *enforced*.

### What is already true — verify, do not rebuild

| Claim | State at HEAD |
|---|---|
| Gate registered, `Blocking`, hard-blocks regardless of `CURRENT_PHASE` | TRUE — `:308` `dev_enforced_red_blocks(BindingClass::Blocking, true)`; `gate_common.rs:97-102` returns `true` unconditionally |
| Gate PASSES at HEAD | TRUE — exit 0, `legs:["frame-borne-route-intact","loopback-from-host-unverified"]` |
| Proven-red runs as its OWN CI step | TRUE — `discipline.yml:1816-1817`, 11 tests pass locally |
| All enrollment surfaces correct | TRUE — six surfaces, verified individually (AC2.6) |
| `EXPECTED_GATES` = 37, `check_*.rs` = 68 | TRUE — but the ratio **asserts nothing**; see Trap 12 |
| FR23a coverage row already names the gate | TRUE — `tests/coverage-matrix.yaml:472-480`, and it already pre-announces this story's legs |
| A CI positive control already exists | TRUE — `journey_j1.rs:107` asserts `delegation_routed.intent`, run under `-p maos-journey-test` |
| `smoke_cli_wrapper_8_12` null control | TRUE and OPEN — zero CI invocation across all 11 workflows |

---

## Story

**As** Lunarpulse, relying on the founder loop's delegation path,
**I want** the loopback A2A wire to provably refuse a disallowed intent with the correct, unconflated
error code, guarded by a blocking gate whose legs cannot pass vacuously,
**so that** the `developer-remote` leg cannot silently regress into local delivery or into a
fail-open admit, and rung 2 inherits a `PROVEN_BLOCKING` foundation rather than a claim.

---

## Acceptance Criteria (4)

### AC1 — Refusal proofs, asserted where the codes are actually distinguishable

Lands as **one new integration test file**, `crates/maos-bin/tests/consent_refusal_1b.rs`.
*Why there:* it is the only location that can drive the **real production peer configuration** —
`maos_a2a::pairing::{paired_loopback_router, LoopbackEndpoint}` plus the
`maos_bin::delegation::{FROM_HOST, TO_HOST, RECIPIENT_SPIRIT, FROM_SPIRIT}` constants — instead of
hand-copied identity strings that drift. `crates/*/tests/` costs **zero kloc** (verified: the
measured set contains 0 files under `/tests/`), and `maos-bin`'s `network` feature is `default`, so
it compiles under a bare `cargo test`.

1. **Positive control, in the same test binary as the negatives.** An allowlisted
   `development-task:write-workspace` frame is admitted and its delivered `intent_class` asserted
   equal. Without a working positive in the same binary, the negatives can pass vacuously.
   *(The end-to-end production positive already exists and runs in CI —
   `crates/maos-journey-test/tests/journey_j1.rs:107` and
   `crates/maos-bin/tests/delegation_leg_1a.rs:75-87`. Do not rebuild either; this is the local
   control for this file.)*
2. **`-32001` / `IntentDeniedAtPeer`, via a deliberately ASYMMETRIC pairing.** Per F4 this is the
   only way to reach the code. Build endpoints directly (all `LoopbackEndpoint` fields are `pub`):
   the destination's `send_allowlist` carries the disallowed intent, the source's `accept_allowlist`
   does **not**. Assert `A2AError::IntentDeniedAtPeer { peer, message }` where `peer == TO_HOST`, and
   that `message` names **`founder-loop-host`** — the SOURCE host, produced at `router.rs:1331-1336`.
   That tail is the source-keying asymmetry made observable; naming it is the assertion.
   **Both `peer_id` strings must appear literally in the test.** An AC phrased *"Host B's
   accept_allowlist admits X"* would be wrong on loopback.
3. **`-32009` / `ConsentUnclassified` at BOTH seams, with the reason typed.** Send seam:
   `A2AError::ConsentUnclassified { direction: Send, reason }` (`router.rs:696-701`). Accept seam:
   the `-32009` NACK from `handle_intake` (`router.rs:1235-1244`), which must be driven directly
   because `prepare_outbound` denies first. Cover **every reachable** `UnclassifiedReason` — the
   architecture names four triggers and the enum ships three:
   `Absent` (no `consent_envelope` / no `intent_class`), `NonCanonical`, `Oversized`
   (`> MAX_CANONICAL_INTENT_LEN` = 128, `maos-domain/src/invariants/i8.rs:44`). Assert the NACK's
   `data.reason` deserializes to the expected variant — a numeric-only assertion makes the deny
   illegible, which is the defect `fail_closed_8_8.rs:128-135` exists to prevent.
4. **Non-conflation, asserted in BOTH directions, across all THREE codes.** The `-32001` leg must
   reject `-32009`, **and** the `-32009` leg must reject `-32001` (the both-ways shape of
   `fail_closed_8_8.rs:216-240`; the `-32001`-is-a-failure arm of `main.rs:11308-11311`). Per F6 a
   third code is live — `CODE_CONSENT_EXPIRED` (`-32003`) — so also assert an expired envelope
   yields `-32003` and **not** either deny code. Three codes, three meanings, no collapse.
5. **STATED NON-COVERAGE — two gaps, both written into the record and into `j1-crosshost-2`.**
   (a) **Rung 1 does not exercise peer authentication.** On the TCP path `handle_intake_verified`
   binds `frame.from.host_id` to the TLS-verified peer (`router.rs:1494-1521`). On loopback there is
   nothing to bind it to — `router.rs:1477-1479` says so outright (*"no wire identity to bind"*), and
   `LoopbackA2ARouter` calls `handle_intake` directly (`crates/maos-a2a/src/adapter.rs:82`, `:97`).
   **The field that selects which `accept_allowlist` applies is written by the sender and never
   verified: a frame picks its own judge.** Survivable in-process; **not** acceptable as the
   inherited claim that rung 1 "proves the wire so rung 2 only adds network." Every refusal proved
   above is one string assignment from selecting a different allowlist.
   (b) **NEW — the production error path conflates these codes** (F5). Refusals are proven at the
   router seam; a cross-host operator consuming `IacBusError` cannot today tell `-32001` from
   `-32009`, and cannot see an unclassified reason at all. One of those is policy working; the other
   is policy being unreadable. **Do not fix it here** — it is a `maos-a2a-core` surface at zero kloc
   headroom, and widening it is a scoped decision, not a side effect.
   **D18 is FILED and RESOLVED** (2026-08-15): the fix costs `maos-a2a-core` ≈ zero net lines (two
   5-line `format!` arms become typed constructions) with ~+6 in `maos-domain` riding on D14, so
   D10's zero-headroom wall never applied; owner John + Vex at 14-4, deadline *"before
   `j1-crosshost-2b` writes its first line"*. **Verify the row, do not re-file, do not close it.** **Do not file it "against rung 2":**
   `j1-crosshost-2` has a sprint-status row and **no story file**, so a deferral addressed there has
   no document to land in. Scout the story you defer *into*; this one does not exist yet.

### AC2 — Gate legs on a static oracle, with the vacuity hole closed

1. **Add the consent leg(s) to `xtask/src/check_j1_loopback_delegation.rs` as source-structural
   checks.** Per F1/F3: no `Command`, no `cargo`, root-relative only, honouring
   `run_with_root(json, root)` (`:299`). The leg asserts the AC1 proofs **exist and are correctly
   shaped** — i.e. that `consent_refusal_1b.rs` still asserts `IntentDeniedAtPeer`, still asserts
   both `-32009` seams, and still contains the both-ways non-conflation assertion. Deleting or
   weakening an assertion must RED the gate.
   **Use the composed idiom `structural(production_before_tests(src))` — the exact spelling at
   `:137` — for every multi-token needle. Never bare `contains_live()` (`:104-106`).** Both halves
   are load-bearing and each was learned from a live escape:
   - `contains_live` matches raw line content and is layout-sensitive. That is what made `cargo fmt`
     and this gate **mutually exclusive** at `246660f9`, whose commit message ends: *"j1-crosshost-1b
     should reuse the same normalization for its refusal legs."*
   - `structural()` alone normalizes the **whole file**, so at `5a921c0c` the demo's review found the
     repair was itself bypassable: relocating the production fail-closed closure into a
     `#[cfg(test)]` module kept the `Blocking` leg **green**. `production_before_tests` (`:108`) is
     the fix, and it is now pinned by an **11th** planted vector.
   Your consent legs must not be satisfiable by an assertion that lives in a test module.
   Add sites in this order: const near `:55-60` → `fn leg_*` after `:293` → call in `run_with_root`
   `:303-304` → name in `ledger_leg_names()` `:64-66` → published boolean in the JSON `:313-326` →
   the `## Legs` module doc `:22-33`.
2a. **FIRST — repair `1a`'s boundary leg. It is a null control that can never fire.**
   *(Found by the `j1-crosshost-2` preflight, 2026-08-15. This is the vacuity class AC2.2 exists to
   close, sitting inside the very gate you are extending — fix it before adding legs beside it.)*
   `leg_loopback_from_host_unverified` (`:266-293`) computes
   `unverified = contains("frame.from.host_id") && contains("pub async fn handle_intake_verified")`
   over **one shared file**, `crates/maos-a2a-core/src/router.rs`. **Both needles are permanent
   features of that file** — the loopback path will always need the self-asserted resolution at
   `:1087-1090`, and the verified entry point already exists at `:1494` for the TCP path. The leg
   therefore publishes `loopback_from_host_unverified: true` **forever, in every possible future**.
   It cannot change state, so it cannot fail, so it is decoration.
   *Sharpened 2026-08-16 by the `j1-crosshost-2a` preflight — the mechanism is worse than "both
   needles are permanent", in two ways you need before you rewrite it:*
   - **The "self-asserted" needle is satisfied by the VERIFIED path's own error message.**
     `contains_live` (`:104-106`) filters comment-prefixed lines but **not string literals**, and
     `crates/maos-a2a-core/src/router.rs:1514` is the `format!` literal
     `"frame.from.host_id {} does not match TLS-verified peer {}"` — **inside
     `handle_intake_verified`'s own TLS-mismatch NACK**. So even deleting the loopback resolution at
     `:1087-1090` would leave the needle green, pinned by the code that proves verification *is*
     enforced.
   - **The leg cannot observe its own declared trigger.** "Rung 2 turns verification on" is a change
     in *which* router entry the composition root calls — in `maos-a2a-tcp` and `maos-bin` — **not a
     text change in `router.rs`, the only file this leg reads.** Any rewrite must therefore point the
     leg at the file where the trigger actually lives, or it reproduces the same defect at a new
     address.
   That makes three shipped documents wrong: this gate's own doc (`:30-32`), `1a`'s story record
   (`j1-crosshost-1a-frame-borne-delegation.md:288`) and the `j1-crosshost-2` sprint-status row all
   claim *"when rung 2 turns verification on, this leg flips."* It will not.
   **Fix it to observe the thing it names:** the leg must key on whether the **J1 delegation path**
   binds wire identity — i.e. on `crates/maos-bin/src/delegation.rs` / the composition root using
   `LoopbackA2ARouter` (which calls `handle_intake` directly, `crates/maos-a2a/src/adapter.rs:82`,
   `:97`) versus a verified transport. Then it genuinely flips when rung 2 lands, and a planted
   regression can red it. **Add a proven-red vector for the flipped state** — otherwise you have
   replaced one unfalsifiable leg with another. Correct the three documents in the same change.
2. **Land the vacuous-green guard as a SHARED PRIMITIVE in `gate_common.rs`, and be its first
   consumer** (F2). A leg that reads nothing must not be able to look like a leg that passed.
   **Measured, and the reason this is not a local fix:** 16 of 68 `check_*.rs` gates carry a vacuity
   guard; `gate_common.rs` carries **none**; all 16 are hand-rolled separately. Writing a 17th bespoke
   guard here is how that number got to 16.
   - Add the minimum to `xtask/src/gate_common.rs`: a per-leg outcome record (`ran`, plus a count of
     checks actually performed) and one assertion that hard-FAILs when any leg reports
     `!ran || checks == 0`. Semantics from `check_vetting_attestation.rs:225-235`. **~20 lines. Not a
     framework** — no plugin points, no trait hierarchy.
   - `check_j1_loopback_delegation` consumes it for **every** leg, `1a`'s two included.
   - **The primitive gets its own falsifier** (proven-red): a leg rigged to check nothing must RED.
     A vacuity guard that can itself be vacuous is the defect it exists to prevent.
   - **Migrating the other 15 is explicitly OUT OF SCOPE** — that is 14-6's instrument work. This
     story lands the shared home and one consumer, nothing more.
   *Net effect on this story is negative lines:* a shared 20-line primitive replaces the bespoke
   guard that would otherwise be written inline here.
3. **Close the leg-omission null control.** Add a `#[cfg(test)]` derivation test asserting
   `ledger_leg_names()` matches the legs `run_with_root` actually invokes. This gate is the **only**
   `ledger_leg_names()` owner with no such test — `check_reza_production_path.rs:1162-1164`,
   `check_cross_region_consensus.rs:313-315`, `check_multi_tenant_loom.rs:1930-1933` and
   `check_multi_region_slo.rs:751-752` all have one. Today a leg added and forgotten in the accessor
   reds nothing.
   *Budget note:* an in-`src` `#[cfg(test)]` module is kloc-charged and CI-invisible — that is
   D11-E3. Prefer `xtask/tests/`; if it must be in-`src`, count it in AC3.1.
4. **Extend the proven-red vectors** in `xtask/tests/j1_crosshost_1a_proven_red.rs` (kloc-free): a
   planted regression that **admits a disallowed intent**, and one that **collapses `-32001` into
   `-32009`**, must each RED. **You must also extend `lay_green` (`:72`) and its `GOOD_*` consts
   (`:44`, `:48`, `:60`) with any file or needle your new leg reads** — otherwise
   `baseline_fixture_tree_is_green` (`:131-133`) reds and every planted vector becomes vacuous
   (F3). The suite already runs as its own CI step at `discipline.yml:1816-1817`; keep
   `--test-threads=1` (load-bearing — each vector sets `current_dir`).
5. **Enroll the new test file in CI — this line IS the control, and it is the load-bearing one.**
   Add `--test consent_refusal_1b` to `discipline.yml:1821`. `maos-bin` is never tested bare in CI;
   all three invocations (`:1821`, `:2576`, `:2943`) name explicit `--test` targets, so an unenrolled
   file in `crates/maos-bin/tests/` is dead. **That omission is exactly how `smoke_cli_wrapper_8_12`
   became a null control.**
   **Why this is not a footnote.** The gate is a static oracle: it has never observed a frame being
   refused and cannot — it greps source text. The behaviour is proven by the CI step, a *different
   mechanism in a different file*. **The enrollment line is the only thing connecting the linter to
   the judge.** Delete it and the gate still finds the right words in the test file and goes green
   while the test never runs.
   So: (a) a gate leg asserts that every `crates/maos-bin/tests/*_1a.rs` / `*_1b.rs` delegation test
   file is named in the J1 job, and (b) **proven-red vector #12** — remove `--test consent_refusal_1b`
   from the workflow, the gate MUST red. Not "should". The most important leg in this story currently
   has no falsifier; this is it.
6. **Close the `smoke_cli_wrapper_8_12` null control — by exact test name.** Today
   `ci_local_split_refuses_a_granted_real_agent_without_the_live_flag` has **zero** CI invocation, so
   *"CI physically cannot spawn a paid agent"* holds by env-var omission rather than by an executed
   assertion. It is safe to run: it needs no secret and no network, spawns a 2-line `/bin/sh` fake
   `codex` it writes itself, and asserts the **refusal** path.
   **Enroll the single test, not the file** — following the `:2578` idiom:
   `cargo test -p maos-bin --test smoke_cli_wrapper_8_12 ci_local_split_refuses_a_granted_real_agent_without_the_live_flag -- --exact`.
   Enrolling the whole file drags in `maos_run_cli_wrapper_worker_spawns_real_subprocess`, which
   needs `worker-cli-fixture` on `PATH` — not a `maos-bin` dev-dependency, so it would red on a fresh
   runner (the failure `discipline.yml:1887-1898` documents).
7. **Live-`codex` leg stays `MAOS_LIVE_AGENT=1`, local-only, NEVER CI.** Already registered at
   `crates/maos-bin/src/env_contract.rs:420`; no new env var is expected. If you add one, register it
   there or `check-env-contract` reds.
8. **Enrollment is VERIFY-only.** All six surfaces are correct at HEAD (see AC2 preamble table and
   Dev Notes). Touch one only if it is wrong. **Do not create a second gate module, dispatch arm, or
   registry entry** — it will collide.
9. **Keep the job hermetic and unconditional.** No `services.postgres` (the J1 job already satisfies
   `runs_gate`, so adding one fires `check-loom-substrate-drift`,
   `check_loom_substrate_drift.rs:701-706`); no `if: false` at job **or step** level
   (`check_epic_close_green.rs:83-90` trims leading whitespace, so it catches both).

10. **Expose per-leg outcomes, so a consent red is not reported under the wrong name.** The per-leg
    record AC2.2 introduces must be readable by callers, not collapsed to `Result<(), String>`.
    Today `demo_j1::run_delegation_gate()` (`:763-781`) matches on that one boolean and emits a
    single beat named `frame-borne-route-intact` — so after this story a red *consent* leg would
    print `FAIL frame-borne-route-intact`, naming the wrong failure in the narrated artifact. Emit
    one beat per leg.
11. **Flip the ABSENT beat this story owns — the artifact must not print a false claim about its own
    work.** `xtask/src/demo_j1.rs:785-790` declares
    `Beat::absent("disallowed-intent-refused-blocking", "a disallowed intent must be REFUSED (-32001
    CODE_INTENT_DENIED, distinct from -32009)", "j1-crosshost-1b")`. If the legs land and that beat
    still renders `ABSENT`, the demo states this work was never done.
    **Flip it in code:** remove the entry from `unlanded_beats()` (extended into the vector at
    `:238`) and emit it as an **executed** beat from the gate-judging path.
    **Do NOT attempt the published-ledger route** — it is structurally unreachable: `ledger_gates()`
    (`xtask/src/evidence_ledger.rs:148-150`) derives from `check_loom_substrate_drift::contract_jobs()`
    (the four Postgres substrate gates), `expected_ledger_legs` (`:154-166`) has no J1 arm, and
    `validate_against` (`:1246-1249`) rejects an unknown gate up front. A hand-written J1 ledger
    would fail validation and suppress published-ledger application for the **entire** demo
    (`demo_j1.rs:817-826`). Extending `ledger_gates()`/`CONTRACTS` is **out of scope**.
12. **Do not break the demo's own suite, and do not rename `1a`'s leg.**
    `xtask/src/tests/demo_j1_tests.rs:218` asserts `beats.len() == 9` over `evaluate_beats`'s
    **event-derived** beats — a new *gate leg* does not trip it, but flipping your beat out of
    `unlanded_beats()` into the executed set **will**. Update that constant deliberately; `:15-24`
    asserts every `unlanded_beats()` entry is `ABSENT` and names an owner. The beat name
    `frame-borne-route-intact` is hard-coded at `demo_j1.rs:229`, `:766` and `:775` — renaming `1a`'s
    leg silently breaks the matcher at `:844`.

### AC3 — Budget, measured at HEAD and attributed honestly

1. **`xtask` needs no grant. Measured: 38075 / 38609 = +534 headroom.** That is ~2.4× the whole gate
   module. `kloc.toml:64-65` still governs: *"Slack is operating capacity, NOT authorization."*
   Keep the proven-red harness in `xtask/tests/` (free) and prefer it for anything that fits.
2. **D15 is RESOLVED — the grant is ratified and applied. Do not re-request it; keep it true.**
   `maos-bin = 16219` (exact measured, **zero headroom**) was ratified by Lunarpulse 2026-08-15 and
   is live at `xtask/kloc.toml:264`; the crate is **GREEN** at HEAD. Attribution, measured per commit:
   `af788c3e` 16027 → `6827dc87` (1a) **16211** (+184, landing +33 over) → `296aa2ce` (j1-demo drain
   fix) **16219** (+8).
   **What this means for you, concretely:**
   - Your additions to `maos-bin/src` must be **+0**. All new code lands in `crates/maos-bin/tests/`
     (kloc-excluded) and `xtask/` (534 spare). **One production line in `maos-bin/src` reds CI** —
     that is the deliberate design, not an accident.
   - If you genuinely need a `maos-bin/src` line for a **correctness or compliance repair**, the
     valve is `kloc.toml:87` (a ceiling *"must never block a correctness or compliance repair"*,
     cited by name by Story 13.5g). State the repair and take a named grant. **Silent growth is what
     the zero stops; declared repair is not blocked.**
   - Rationale is recorded in the `kloc.toml` annotation and in D15 — read it before arguing with
     the wall. Summary: the formula's 16544 would grant 325 lines to a crate that is 73% `main.rs`
     with no decomposition scheduled; tighter-than-formula is house style (`kloc.toml:203`); zero
     headroom is precedented and deliberate (`maos-a2a-core` 4654/4654).
3. **REFUSE the aggregate grant, deliberately, and say why in the record.** All three breaches below
   are pre-existing and owned elsewhere:

   | Breach | Size | Owner |
   |---|---:|---|
   | `maos-kernel-core` 18933/18248 | +685 | **D13** — `spec-epic-5-review-finding-closure` (repair) / 14-6 (instrument) |
   | `maos-domain` 8694/8644 | +50 | **D14** — 14-7, via explicit AC expansion |
   | `_aggregate_hardfail` 147549/147057 | +492 | **D17 (file it)** — see below |

   **This story *could* take the aggregate grant, and must not.** `kloc.toml:61` permits recalculation
   *"at an epic retrospective, **or** under an explicitly authorized measured grant"* — two doors, and
   Stories 13.6d and 13.6e both went through the second one, as did the epic-orphaned
   `j1-demo-one-command-scene`. So neither "bridge stories have no retrospective" nor "the aggregate
   is unowned" is a reason. The reason is narrower and stronger:
   - **1b's contribution to the aggregate is ZERO** — every line it writes is kloc-excluded. 13.6e's
     grant is annotated *"authorized formula applies **because aggregate actually breached**"*: the
     story that caused it paid for it.
   - The +492 is **arithmetic downstream of D13 (+685), D14 (+50) and D15 (+41)**. Re-basing it here
     turns the CI signal that holds those three to account **green**, leaving only prose behind them.
     D13 forbids precisely this of 14-6 — *"may not erase this red with a grant it has no measured
     delta to justify"* — and 1b is further from the delta than 14-6 is.
   - **It would not stay fixed anyway.** Re-basing a *crate* ceiling does not change the *measured*
     aggregate; measured stays 147549. D13/D14/D15 can all land in full and this key is still red.
     It is independent by design — the only instrument that catches distributed growth no per-crate
     reserve can see.
4. **D17 is already FILED — verify, do not re-file.** `_aggregate_hardfail` is a **standing red with
   three named debtors**, it does **not** clear when they re-base, and it clears at an epic
   retrospective or under a grant taken by someone with a measured delta to justify it. Owner: the
   ceiling instrument (**14-6**) or the Epic-14 retrospective, deadline at the v2.2 close. Do
   **not** record it as "unowned" — that framing was disproved at the round-table.
5. **Close with `kloc-check` RED, attributed.** Measure before and after; state plainly which four
   keys are red and whose they are. This story cannot and should not make that gate green.
6. **`maos-a2a-core` is at 4654/4654 — exactly zero headroom, deliberately** (`kloc.toml:407`; D10
   forbids a third unscoped grant). One added **production** line there hard-fails on contact.
   `crates/maos-a2a-core/tests/` remains free, but AC1.5(b) is explicit that the error-mapping fix is
   not taken here — it goes to **D18**.
7. **Kernel axes: re-run the instruments, but report them honestly.** `check-kernel-baseline` GREEN
   at 24472 is real evidence. `abi-diff` / `check-abi-ratification` green is **not** — per F11 they
   scope to `crates/maos-spirit-abi` only and are structurally blind to `maos-kernel-core`'s
   re-exported surface (`iac.rs:13` is `pub use maos_iac::*`), which is open **FLAG-E4**. A flat
   `src_lines` does not imply a flat ABI. This story makes no ABI change; state that as a fact about
   the diff, not as a gate result. Any kernel seam is FLAG-Winston, never an implicit re-pin.

### AC4 — Close the lane honestly

1. **Set this story's `sprint-status.yaml` row to `done`**, and record the model + review artifact in
   the Dev Agent Record. Note that **five story-file gates skip this filename** (digit-prefix
   scoping), so none of that is mechanically enforced — verified individually:
   `check_dev_model_tier.rs:103`, `check_dev_model_used_populated.rs:136`,
   `check_bare_review_findings.rs:35`, `check_dev_record_completeness.rs:240-249`,
   `check_review_findings_resolved.rs:50-63` — each requires the filename's first character to be an
   ASCII digit. Operator decision 2026-08-14: **keep the `j1-` prefix and state the gap.** A green CI
   does not mean the §A6 net ran.
2. **Write AC1.5's two non-coverage statements into the `j1-crosshost-2` sprint-status row** as
   explicit inheritance, so rung 2's preflight cannot mistake a partial proof for a whole one.
   Note what that row *is*: `j1-crosshost-2` has a sprint-status row and **no story file**. Prose in
   a row is the weakest form of hand-off this project has, which is why D18 (AC1.5b) is a decision
   row with an owner rather than a sentence addressed to a document nobody has written.
3. **Verify D17 and D18 — both are already FILED** (`epic-14-preflight-decisions.md`, 2026-08-15).
   D17 = the standing aggregate red (owner 14-6 / Epic-14 retro). D18 = the deny-code conflation
   (owner John + Vex, target 14-4, deadline *before `j1-crosshost-2` writes its first line*).
   Confirm both still read as OPEN at close; do not re-file and do not close them. Binding rule 1:
   shipping adjacent work does not close a row.
4. **Rung 2's evidence contract expects this rung to read `PROVEN_BLOCKING`**
   (`xtask/src/gate_common.rs:139-146`).
5. **Verify D11 and leave it OPEN.** Its row already reads *"36 entries vs 67 `check_*.rs` at
   `af788c3e`; 37 / 68 once the J1 loopback gate lands"* and the counts are **correct at HEAD**.
   Binding rule 1: shipping adjacent work does not close a row — D11's owners (Winston + Murat)
   settle substance at 14-6's preflight. **Do not cite the 37/68 ratio as evidence of anything**
   (Trap 12).
6. **Do not claim FR23a closure.** The coverage row carries `corpora: []` while FR23a's PRD floor is
   *"cross-Spirit consent 30 scenarios with 100% disallowed blocked"*. This story adds a handful of
   refusal legs on the J1 path — it does not deliver that corpus. Update the row's `notes`; do not
   mark the FR satisfied.

---

## Traps

1. **`cargo run -q -p xtask -- <cmd>`.** There is no `cargo xtask` alias.
2. **The gate runs no tests.** It is a static source oracle. Do not make it shell out (F1/F3).
3. **Never copy `check_vetting_attestation::invoke_leg`.** It shells `cargo` with no `current_dir`
   and would silently vacuum the proven-red suite (F3).
4. **`-32001` is unreachable through `DelegationLeg::install`** — one intent configures both
   allowlists, so the send seam denies first (F4).
5. **`-32001` / `-32009` / `-32003` must not be conflated** — and above `A2ARouterCore` they already
   are (F5). Assert at the router seam.
6. **The loopback allowlist is keyed by the SOURCE host** (`router.rs:1087-1090`, fallback
   `HostId("loopback")` when `None`, `lookup_peer` at `:1093` with no first-peer fallback).
7. **`task.assign` is not a legal consent intent.** The vocabulary is
   `development-task:write-workspace`, defined **once** at `spirits/orchestrator/src/lib.rs:73`.
8. **The gate already exists.** Creating a second module/dispatch/registry entry collides.
9. **Use `structural()`, not `contains_live()`, for multi-token needles** — or you re-create the
   `cargo fmt` vs gate mutual exclusion that `246660f9` fixed.
10. **kloc is tokei `code` lines, not raw.** `check_vetting_attestation.rs` is 235 code / 273 raw;
    this gate is 219 code / 353 raw. Budget in the right unit.
11. **`tests/` costs zero kloc in EVERY crate** (verified: 0 measured files under `/tests/`), but
    `xtask/src/tests/` is **charged** (it is under `src/`) — that is D11-E3.
12. **The 37/68 ratio asserts nothing.** `check_ship_gate_completeness` iterates `EXPECTED_GATES`
    one-directionally (`:182`, `:203`); at HEAD it reports `expected 37 / found 43 / missing []`.
    Six ship-gate jobs are absent from `EXPECTED_GATES` and it is silent. Quoting the ratio as
    evidence is a claim standing in for a control.
13. **`coverage-matrix` cannot fail.** `mode: warning` (`tests/coverage-matrix.yaml:3`) and
    `coverage_matrix.rs:65` fails only on `"hard"`; at HEAD it reports `passed:false` with **31
    violations** and exits **0**. So a misspelled gate name in the FR23a row reds nothing.
    `check-coverage-matrix-completeness` only checks for *empty* gate lists.
14. **A `maos-bin` test not named in a `--test` flag is dead in CI.** `maos-a2a-core` is the opposite
    — `discipline.yml:1536` runs it bare, so anything in `crates/maos-a2a-core/tests/` auto-runs.
15. **Do not enroll the whole `smoke_cli_wrapper_8_12` file** — enroll the one test by exact name
    (AC2.6).
16. **`journey_j1` and `delegation_leg_1a` already prove the positive path.** Do not rebuild them.
17. **Establish a clean baseline measurement before attributing kloc movement.** Four crates arrive
    red.
18. **`cargo test -p maos-bin` is RED at HEAD under default parallel flags** (D16, F12). Run your
    file scoped: `cargo test -p maos-bin --test consent_refusal_1b`. The `MAOS_HOME` isolation
    defect is not yours to fix.
19. **Two local `maos run … --once` on the same `XDG_DATA_HOME` within 60s: run 2 exits 1** with
    `EOrchestratorDispatchRawOutput` — FR21's dispatch gate is chronological, not causal (open
    **FLAG-E5**, owned by Story 6.2). Use a fresh data home between runs; do not "fix" it here.
20. **`abi-diff` green proves nothing about this surface** (F11 / FLAG-E4).
21. **Do not rename `1a`'s leg `frame-borne-route-intact`** — three hard-codings in `demo_j1.rs`.

---

## Tasks

- [ ] **T1 (AC1.1-1.2)** — Create `crates/maos-bin/tests/consent_refusal_1b.rs`. Local positive
      control + the asymmetric-pairing `-32001` leg asserting `IntentDeniedAtPeer` and the
      `founder-loop-host` message tail. Name both `peer_id`s literally.
- [ ] **T2 (AC1.3-1.4)** — `-32009` at send **and** accept seams, every reachable
      `UnclassifiedReason` (`Absent` / `NonCanonical` / `Oversized`), typed `data.reason` assertion;
      both-ways non-conflation; `-32003` expiry kept distinct.
- [ ] **T3 (AC1.5)** — Write both non-coverage statements (peer-auth unverified; production error
      mapping conflates) into the story record. Confirm `1a`'s `loopback-from-host-unverified`
      boundary leg still reports `true`. **File D18** for the conflation — owner + deadline "before
      `j1-crosshost-2` writes its first line" — **already filed 2026-08-15; verify only**.
- [ ] **T4 (AC2.5)** — Add `--test consent_refusal_1b` at `discipline.yml:1821`; add the gate leg
      that asserts the enrollment line exists; add **proven-red vector #12** (delete the flag → gate
      MUST red). This is the only link between the static oracle and the behaviour it judges.
- [ ] **T5 (AC2.1)** — Add the consent leg(s) to the gate, `structural()`-normalized, root-relative,
      in the six-site order. **Measure code lines on completion.**
- [ ] **T6 (AC2.2-2.3)** — Land the per-leg `ran`/`checks` record + vacuous-green hard-FAIL as a
      **shared primitive in `gate_common.rs`** (~20 lines, no framework); make this gate its first
      consumer across **all** legs incl. `1a`'s two; give the primitive its own proven-red (a leg
      that checks nothing must RED). Do **not** migrate the other 15 gates — that is 14-6. Add the
      `ledger_leg_names()` derivation test.
- [ ] **T7 (AC2.4)** — Add the two consent proven-red vectors **and** extend `lay_green` + `GOOD_*`
      consts so `baseline_fixture_tree_is_green` stays green.
- [ ] **T8 (AC2.6)** — Enroll `ci_local_split_refuses_a_granted_real_agent_without_the_live_flag` by
      exact name; confirm it executes and that CI needs no secret.
- [ ] **T9 (AC2.8-2.9)** — Verify the six enrollment surfaces; fix only what is wrong. Job stays
      hermetic, no `if:`.
- [ ] **T10 (AC2.10-2.12)** — Per-leg beats in `demo_j1`; flip `disallowed-intent-refused-blocking`
      out of `unlanded_beats()` in code (never via a ledger); update
      `demo_j1_tests.rs:218`'s `beats.len() == 9` deliberately; do not rename `1a`'s leg.
- [ ] **T11 (AC3)** — Re-measure `kloc-check` before/after. Take the D15 grant at **`maos-bin =
      16219` (zero headroom)** — **ALREADY RATIFIED AND APPLIED at `kloc.toml:264`; verify it is
      still true and that your `maos-bin/src` delta is +0.** **REFUSE the aggregate grant** (D17 filed).
      Close with `kloc-check` red, attributed to four keys. Re-run `check-kernel-baseline` (real
      evidence) and the ABI gates (report as blind, per F11).
- [ ] **T12 (AC4)** — Sprint-status → `done`; write the two inheritances into the `j1-crosshost-2`
      row; confirm **D17 and D18 still read OPEN**; verify D11 reads 37/68 and stays **open**; update the
      FR23a `notes` without claiming corpus closure; record model + §A6 artifact (all four layers —
      Test-Infra is the one that matters here).

### Review Findings

_(populated by §A6 review)_

---

## Dev Notes

### Measured at HEAD (`5a921c0c`, clean tree) — inherit no number from an older story

| Instrument | Ceiling / pin | Measured | Verdict |
|---|---:|---:|---|
| kloc `xtask` | 38609 (`kloc.toml:203`) | **38075** | GREEN, **+534 headroom** |
| kloc `maos-bin` | 16178 | **16219** | **RED +41 — D15, THIS STORY'S** |
| kloc `maos-kernel-core` | 18248 | 18933 | RED +685 — D13, not yours |
| kloc `maos-domain` | 8644 | 8694 | RED +50 — D14, not yours |
| kloc `maos-a2a-core` | 4654 | 4654 | GREEN — **zero headroom by design** |
| kloc `_aggregate_hardfail` | 147057 | 147549 | **RED +492 — standing red, 3 named debtors (D17)** |
| `check-kernel-baseline` | 24472 | 24472 | GREEN |
| `abi-diff` / `check-abi-ratification` | — | 0 changes / 3 ratified / 0 uncovered | GREEN |
| `check-j1-loopback-delegation` | — | 2 legs, `oracle_green: true` | GREEN, Blocking |
| `check-ship-gate-completeness` | — | expected 37 / found 43 / missing [] | GREEN |
| `coverage-matrix` | — | `passed:false`, 31 violations | **exits 0 — cannot fail** |

Per-file: `check_j1_loopback_delegation.rs` **219** code / 353 raw · `check_vetting_attestation.rs`
**235** / 273 · `demo_j1.rs` **938** / 1207. `xtask/tests/j1_crosshost_1a_proven_red.rs` = 239 code,
**excluded** from the measurement.

### Gate anatomy — what is ACTUALLY there

`xtask/src/check_j1_loopback_delegation.rs`, a static source-text oracle:

```
consts :55-60 (TOPOLOGY, DELEGATION_RS, MAILBOX_RS, MAIN_RS, ORCHESTRATOR_RS, A2A_ROUTER_RS)
ledger_leg_names() :62-66            → ["frame-borne-route-intact","loopback-from-host-unverified"]
struct Finding :68-                  → { check, detail }  ← no ran/passed/failed (F2)
contains_live() :104-106             → layout-SENSITIVE, single-token needles only
production_before_tests() :108-      → strips #[cfg(test)] so test asserts can't keep it green
structural() :125-130                → strip comment lines, strip ALL whitespace
   :137  structural(production_before_tests(src))   ← THE COMPOSED IDIOM. USE THIS.
leg_frame_borne_route_intact() :147-261   → 6 sub-checks
leg_loopback_from_host_unverified() :266-293 → 28 lines, NOT "a 4-line record". Returns `true` as a
   boundary observation, but PUSHES A FINDING at :282-291 if the boundary MOVES — so it reds a
   Blocking gate. Rung 2 flips it from "documented gap" to "enforced".
run() :295-297 → run_with_root(json, Path::new("."))
run_with_root() :299-355 → legs → `oracle_green = findings.is_empty()` :306
                         → dev_enforced_red_blocks(Blocking, true) :308
                         → JSON :313-326 → Ok(()) | Err(...) :344-352
```

`main.rs:1348` is the `process::exit(1)`; the dispatch arm is `main.rs:1221`. Gates return
`Result<(), String>`; there is no third exit code.

### Where the code goes

| Concern | File | Anchor |
|---|---|---|
| **NEW** consent proofs | `crates/maos-bin/tests/consent_refusal_1b.rs` | NEW — free, must be `--test`-enrolled |
| Production pairing helper (use it) | `crates/maos-a2a/src/pairing.rs` | `LoopbackEndpoint` (all fields `pub`), `paired_loopback_router` |
| Production identities (use them) | `crates/maos-bin/src/delegation.rs` | `RECIPIENT_SPIRIT`/`FROM_SPIRIT`/`TO_HOST`/`FROM_HOST` :57-65 |
| Canonical intent (single definition) | `spirits/orchestrator/src/lib.rs` | `DELEGATION_CONSENT_INTENT` :73 |
| Test harness to copy | `crates/maos-bin/tests/delegation_leg_1a.rs` | `harness()` :31-47, `delegation_frame()` :49-69 |
| Allowlist enforcement (send) | `crates/maos-a2a-core/src/router.rs` | `send_admits` :633-646, call :749, deny :750-759 |
| Allowlist enforcement (accept) | `crates/maos-a2a-core/src/router.rs` | `accept_admits` :648-665, call :1313, NACK :1328-1337 |
| Source-keyed peer resolution | `crates/maos-a2a-core/src/router.rs` | :1087-1090, `lookup_peer` :1093 |
| Unclassified (send / accept) | `crates/maos-a2a-core/src/router.rs` | :696-701 / :1235-1244 |
| Expiry stamp / check | `crates/maos-a2a-core/src/router.rs` | :866-871 / :1208-1222 |
| Classification (private) | `crates/maos-a2a-core/src/router.rs` | `consent_decision` :534-552, `ConsentDecision` :119-126 |
| **Conflation to file, not fix** | `crates/maos-a2a-core/src/router.rs` | `map_a2a_error_to_iac_bus` :1671-1783 |
| Deny codes | `crates/maos-a2a-core/src/transport/json_rpc.rs` | `-32001` :30, `-32009` :62 |
| Error variants | `crates/maos-a2a-core/src/error.rs` | `IntentDenied` :70-75, `IntentDeniedAtPeer` :136-138, `ConsentUnclassified` :166-169 |
| Non-conflation precedent | `crates/maos-a2a-core/tests/fail_closed_8_8.rs` | :213-240, reason assert :128-135 |
| Assertion source (not shape) | `crates/maos-bin/src/main.rs` | positive :11081-11108, `-32001`-is-failure :11308-11311 |
| Gate (EXTEND, not create) | `xtask/src/check_j1_loopback_delegation.rs` | see anatomy above |
| Proven-red (EXTEND `lay_green`) | `xtask/tests/j1_crosshost_1a_proven_red.rs` | `lay_green` :72, `GOOD_*` :44/:48/:60, baseline :131-133 |
| Demo coupling | `xtask/src/demo_j1.rs` | `run_delegation_gate` :763-781, `unlanded_beats` :785-800 |
| CI job (add one `--test`) | `.github/workflows/discipline.yml` | job :1804, gate :1815, proven-red :1817, legs :1819-1821 |

### The six enrollment surfaces — all correct at HEAD, VERIFY only

(a) `xtask/src/main.rs` :111 `mod`, :557 `#[command(name=…)]`, :1221 dispatch ·
(b) `discipline.yml` :1804 job, :3177 `v1-0-ship-gate` needs, :3236 + :3265 echo tables ·
(c) `xtask/gate-registry.toml` :100 flat list, :274-282 `[[ship_gate]]`
(`disposition = { v1_0 = "blocking", v1_5 = "blocking" }`) ·
(d) `xtask/src/check_ship_gate_completeness.rs:56` ·
(e) `tests/coverage-matrix.yaml:472-480` (FR23a row; already names the gate and this story) ·
(f) `crates/maos-bin/src/env_contract.rs:420` (`MAOS_LIVE_AGENT`), :425 (`MAOS_HOST_GRANTS`).
**(g) `xtask/src/lib.rs` is NOT a surface** (F9) — the registry declares only the GA ladder; the
dev-time binding lives in code at `check_j1_loopback_delegation.rs:308`.

### What this story does NOT do

- **No mechanism** — the wire, router install, pump, and env deletion are `1a`'s.
- **No fix to `map_a2a_error_to_iac_bus`** — filed for rung 2 (AC1.5b); `maos-a2a-core` is at zero
  headroom and D10 forbids a third unscoped grant.
- **No new protocol, no mTLS, no second host** — `j1-crosshost-2`.
- **No enforced egress** — stays `declared-not-enforced`.
- **No repair of the `maos-kernel-core` / `maos-domain` / aggregate reds** — D13 / D14 / unowned.
- **No FR23a corpus closure** — 30 scenarios is not this story.
- **No widening of digit-prefix gate scoping** — Epic 14's instrument work (D11).
- **No extension of `ledger_gates()` / `CONTRACTS`** to make a J1 evidence ledger publishable.

### References

- [Source: `_bmad-output/planning-artifacts/sprint-change-proposal-2026-07-16.md#4.1`] — the ratified card (its AC4 refusal shape is corrected here: smoke-8-8 was the wrong precedent).
- [Source: `_bmad-output/implementation-artifacts/j1-crosshost-1a-frame-borne-delegation.md`] — predecessor; `DelegationLeg`'s `intent` parameter was added explicitly for this story.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/7-inter-agent-communication.md#7.2`] — normative: unconditional fail-closed on unclassified at BOTH seams; `-32009` distinct from `-32001`; no band-fallback opt-in; no auto-retry.
- [Source: `docs/adr/ADR-012-typed-intent-a2a-consent.md`] — consent is `(peer-identity, intent-class)`; the ADR names no allowlist keys.
- [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md#FR23a`] — 30 consent scenarios, 100% disallowed blocked (**not** closed by this story).
- [Source: `_bmad-output/planning-artifacts/prd/user-journeys.md#Journey-1`] — J1; note its "remote Host's allowlist" phrasing is wrong for loopback (source-keyed).
- [Source: `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md`] — D10 (`maos-a2a-core` zero headroom), D11 + D11-E1/E2/E3, D13, D14, **D15 (this story's grant)**.
- [Source: `xtask/kloc.toml#58-65`] — grants are measured, never estimated; slack is not authorization.
- [Source: `xtask/src/gate_common.rs#139-146`] — `PROVEN_BLOCKING` / `ABSENT` / `INDETERMINATE` spellings rung 2 consumes.

---

## Dev Agent Record

### Agent Model Used

_(record `vendor/model` + harness + date — required by policy even though five gates skip this
filename; see AC4.1)_

> **§A6 is non-degradable here, and the Test-Infra layer is the one that matters.**
> `epic-10-process-agreements.md:26-30`: *"A degraded review is not a review. Main-session
> self-review does not satisfy §A6."* Four layers — Blind Hunter · Edge Case Hunter · Acceptance
> Auditor · **Test-Infra Auditor** — plus, for security surfaces, a **runtime-execution check
> confirming the code is actually exercised in CI, not merely compiled.**
> This is not boilerplate on this story. `1a`'s §A6 pass **omitted Test-Infra** (its record,
> `:16-17`); `j1-demo` ran it voluntarily and **it is what caught the gate bypass** that a
> `#[cfg(test)]` relocation could walk through. Given F1–F3 (a static oracle, a missing vacuity
> guard, and a harness that silently vacuums when a leg shells out), Test-Infra is the layer with
> the highest yield here. Run all four.

### Debug Log References

### Completion Notes List

### File List

---

## Open Questions for Lunarpulse

**None blocking. Both are closed.**

- **Q1 — D15 grant number: RATIFIED 2026-08-15.** `maos-bin = 16219`, exact measured, zero headroom,
  over the formula's 16544. Applied at `xtask/kloc.toml:264`; `maos-bin` is GREEN at HEAD; D15 is
  marked RESOLVED in the register. The dev does not request this — see AC3.2 for what it constrains.
- **Q2 — `j1-crosshost-2`: being authored.** It is no longer a row with no document, so AC4.2's
  inheritance lands in a real story file, and D18's deadline (*before rung 2 writes its first line*)
  is addressable.

Also settled by the round-table and written into the ACs: the aggregate breach is a standing red with
three named debtors and is **not** this story's to clear (**D17** filed); the per-leg gate surface is
in scope and carries the shared vacuity primitive; AC3 folded into AC2/AC4, leaving four ACs.

---

## Change Log

| Date | Change |
|---|---|
| 2026-08-14 | Created by splitting `j1-crosshost-1-loopback-developer-remote-delegation` (split ratified by Lunarpulse at preflight). Carries the judge: consent refusal proofs + the blocking gate + the deferred, measured `xtask` grant. |
| 2026-08-14 | **Round-table consensus applied.** Gate skeleton moved to `1a`; budget rationale dissolved on measurement; NEW stated non-coverage on peer authentication; intent widened to `development-task:write-workspace`; D11 verification added. |
| 2026-08-15 | **Re-grounded at `5a921c0c` by a five-scout preflight; thirteen premises disproved or corrected.** (F1) The gate is a **static source-text oracle** — it runs no tests; AC2 rewritten from "cargo legs" to source-structural legs + CI enrollment. (F2) **No vacuous-green guard exists** in this gate — AC2.2 changed from "extend" to "build", and it now supplies the per-leg record. (F3) **Ship-blocker:** copying `check_vetting_attestation::invoke_leg` would inherit the proven-red tempdir and silently vacuum every planted vector; new legs must stay root-relative. (F4) `-32001` is **unreachable through `DelegationLeg::install`** (one intent configures both allowlists, so the send seam denies first) — AC1.2 now builds an asymmetric pairing. (F5) `map_a2a_error_to_iac_bus` **conflates both deny pairs**, so the non-conflation assertion moves to the router seam and the lossy mapping is filed for rung 2 as new non-coverage AC1.5(b). (F6) **Envelope expiry is NOT a gap** — TTL is stamped at `router.rs:866-871` and enforced at `:1208-1222` via a third code `-32003`; the old AC1.7 is deleted and replaced by a three-code distinctness assertion. (F7) Both `smoke-*` precedents are **network-gated binary arms with zero CI invocation**; copying their *shape* reproduces a null control. (F8) Budget was stale by a full grant: `xtask` headroom is **+534**, not ~29; four crates are red, not one. (F9) `xtask/src/lib.rs` is **not** an enrollment surface — six, not seven. (F10) `deferred-work.md` names this story **nowhere**; the stale-owner hazard is unfounded. **NEW AC3** — `demo_j1` carries an `ABSENT` beat named `disallowed-intent-refused-blocking` **owned by this story**, and collapses all gate legs into one beat, so a consent red would be reported under the wrong name; the evidence-ledger route to flip it is structurally unreachable. **AC4** now carries the **D15 `maos-bin` +41 grant** (measured per-commit) and names the **unowned aggregate breach**. **AC5.5** forbids claiming FR23a corpus closure (`corpora: []` vs a 30-scenario floor). (F11) **`abi-diff` / `check-abi-ratification` are structurally blind to this surface** — they scope to `crates/maos-spirit-abi` only (`abi_diff.rs:8`), open **FLAG-E4** — so AC4.5 no longer cites them as evidence, and `kernel_grant` was rewritten from "verified flat" to "no change was made". (F12) **`cargo test -p maos-bin` is RED at HEAD under default parallel flags** (D16 `MAOS_HOME` isolation defect); added as a trap with the scoped-invocation workaround, plus FLAG-E5's 60s FR21 window for repeated local runs. (F13) Line/shape corrections re-derived throughout (`accept_admits` :648, proven-red idiom :2598, journey :1901, allowlist doc citation -> `pairing.rs`): the demo suite asserts `beats.len() == 9` at `:218` (not 7/`:199`), and `1a`'s boundary leg is **28 lines that RED when the boundary moves**, not "a 4-line record". **AC2.1 now mandates the composed idiom `structural(production_before_tests(src))`** (`:137`) — `structural()` alone was itself bypassable by relocating the production closure into a `#[cfg(test)]` module, fixed at `5a921c0c` and pinned by an 11th vector. **AC3.1 names the only viable flip mechanism** (remove from `unlanded_beats()` + emit an executed beat; the ledger path is unreachable). §A6 note added: `1a`'s pass omitted the Test-Infra layer, which is the layer that caught the demo's gate bypass. 3 ACs → 5. |
| 2026-08-15 | **Round-table consensus (Winston · Murat · Amelia · John · Mary · Sally · Paige · Vex · Dana · Grumbal); five decisions folded, 5 ACs → 4.** (1) **AC3 collapsed into AC2.10-2.12 and AC4** — the demo beat is a *correctness constraint on the close*, not a capability ("every fix a constraint, never a new AC"): if the legs land and `disallowed-intent-refused-blocking` still renders `ABSENT`, the narrated artifact prints a false claim about its own work. (2) **The vacuity guard becomes a SHARED PRIMITIVE in `gate_common.rs`** with this gate as first consumer — measured 16/68 gates carry one, `gate_common` carries none, all 16 hand-rolled; a 17th bespoke guard is how that number grew. ~20 lines, no framework, its own falsifier, and migrating the other 15 stays 14-6's. Net lines negative. (3) **NEW proven-red vector #12** — delete `--test consent_refusal_1b` from the workflow and the gate MUST red. Sally's question exposed why: the gate has *never observed a frame being refused* and cannot — it greps source text. The enrollment line is the only thing connecting the linter to the judge, and it had no falsifier. (4) **D15 grant set to `maos-bin = 16219` — exact measured, ZERO headroom**, over the formula's 16544: `main.rs` is 73% of the crate with no decomposition scheduled, tighter-than-formula is house style (the `j1-demo` grant records landing 4 lines tighter as the point), zero headroom is precedented and deliberate (`maos-a2a-core` 4654/4654 "hard-fail on contact"), and `kloc.toml:87` exempts correctness/compliance repairs so it traps no genuine fix. Dana's trap objection recorded and answered. (5) **The aggregate grant is REFUSED deliberately → D17.** Two claims from story-creation were disproved in the room: `kloc.toml:61` permits recalculation at a retrospective **or** under an authorized measured grant — two doors, both used by Stories 13.6d/13.6e and by the epic-orphaned `j1-demo` — so "bridge stories have no vehicle" was wrong; and the aggregate is not "unowned" but a standing red with three named debtors that would **not** clear even if all three re-based, because a ceiling move does not change the measured aggregate. 1b's contribution is **zero**, and granting it would green the CI signal holding D13's +685 to account — which D13 forbids of 14-6, and 1b is further from the delta. (6) **F5 conflation → D18** with an owner and the deadline *before rung 2 writes its first line*, not deferred "against `j1-crosshost-2`", which has a sprint-status row and **no story file**: you cannot defer into a document that does not exist. |
