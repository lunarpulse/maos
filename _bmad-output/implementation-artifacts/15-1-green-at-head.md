---
baseline_commit: "**`2fd492c0`** (\"15-3-single-phase-source-and-exit-command-check\"). Every number below was RE-MEASURED at this commit, not inherited. ⚠⚠ **THE WORKING TREE MUTATED DURING THIS PREFLIGHT.** `git status --porcelain` was empty at 13:34 and dirty by 13:44 — `xtask/src/check_exit_commands.rs` (+183/−69), `15-3-…md` and `sprint-status.yaml` — written by a CONCURRENT AGENT closing 15-3 review findings, not by this preflight. That in-progress edit currently leaves `cargo build -p xtask` at **EXIT 101** (`E0425: cannot find value COMMANDLESS_MAOS_BINARIES`, `check_exit_commands.rs:1590`). **This is NOT a HEAD defect** — `git archive HEAD` carries the symbol (2 occurrences) and every gate run below used a binary built from HEAD. Every figure in this file was therefore re-pinned against a pristine `git archive HEAD` copy: `xtask` 43411, `maos-bin` 17339, `check_exit_commands.rs` 1344. **Apply the 14-2d rule: cite with `git show HEAD:<path>` when another agent may be writing, and RE-MEASURE at T0 before writing a line.**"
depends_on: "**15-2 (`done`) and 15-3 (`done`) — and 15-3 is what makes this story necessary in its current shape.** 15-2 supplied the ceilings; 15-3 spent the entire `xtask` grant and then overspent it by 226 (§7). The epic's dependency line orders 15-2 → 15-3 → 15-1 and that order held, but the ground moved twice underneath this section: **AC4 was discharged by 15-2 and re-opened by 15-3 in a different crate**, and **AC6 acquired an ordering constraint from a gate 15-3 did not touch** (§4). Nothing else blocks."
blocks: "**15-4 AC3 directly** (`epic-15…` Dependencies: *\"15-1 (aggregate green) before 15-4 AC3 can be true\"*), and the epic's exit-block line 1 for every story after it. Until this lands, six of the workspace's Blocking controls are red and the discipline `aggregate` cannot go green, so no later story in Epics 16–21 can demonstrate an exit."
spec_alignment: "⚠ **THE EPIC IS AMENDED IN THE SAME COMMIT — fourteen places, listed in §11.** Operator directive in force (`feedback_pay_effort_early_deferring_drifts`, confidence rule 9): *no drift in spec; the story and the epic say the same thing, or the epic is amended in the same commit; and never let a story be merely BETTER than its epic.* Four adversarial scouts disproved premises in this story's own section. The largest: **AC1's stated J1 fix cannot be implemented and contradicts itself** (§3), **AC1's CI citations are false in kind, not just in line** (§2), **AC3's central 9/8 split is 18/1 and its two halves annihilate each other** (§6), **AC4 is discharged and vacuous while the live kloc red is in a crate it does not name** (§7), and **AC6 can turn a currently-GREEN Blocking gate RED** (§4). ⚠ The epic amendment is NOT applied in this spec commit — see `spec_landing_note`."
spec_landing_note: "✅ **§11 APPLIED 2026-09-08 against `2fd492c0`** (R-14), after confirming the epic's 15-1 section was byte-identical across the `bce5ac68 → 2fd492c0` amend. Eighteen OLD→NEW edits landed; `check-exit-commands`, `check-epic-close-coherence` and `check-ship-gate-completeness` re-run green over the edited epic. T26 is now a VERIFICATION, and R-7 survives for anything that drifts before the landing commit. ⚠ HISTORICAL, retained so the sequencing is legible: **THE EPIC AMENDMENT AND THE `sprint-status.yaml` FLIP WERE HELD, DELIBERATELY.** A concurrent agent is writing `_bmad-output/implementation-artifacts/sprint-status.yaml` (it moved 15-3 `done` → `review` at 13:37) and may also be amending the epic's 15-3 section. Writing either file now would clobber that work. §11 carries the complete OLD→NEW edit list, ready to apply verbatim. That agent's work is now committed as `2fd492c0` and the tree is clean, so §11 was applied rather than deferred. Rule 9 is discharged, not waived."
split_from: "Not a split. Authored from `epics/epic-15-foundations-w0.md` (the `### 15-1-green-at-head` section) under Round 3, re-derived at `2fd492c0`. ⚠ **SIZING FLAGGED TO THE OPERATOR.** The epic prices this story as five gate fixes plus one test. Measured, it is **six gate fixes, four workspace test failures, one Blocking CI job that cannot pass, and one exit-block clause CI never executes** (§1). It is delivered WHOLE under the no-deferral directive. **RULED WHOLE at the 2026-09-07 round-table (§12, R-8), and the reason is NOT the one 15-3 used.** John declined to hide behind arithmetic: unlike 15-3, the budget does **not** forbid this split — tests are uncharged and AC7 is YAML plus a registry row plus one const, so `AC6+AC7 → 15-1b-suite-green` is genuinely affordable. It loses on **D-6 coupling**: AC6 changes a *gate's binding flag* (`--test-threads=1` out of `check_cert_rotation_trigger.rs:613`) and AC1 depends on the fix that flag was masking. Split them and 15-1 ships a control whose owner has already shipped — the 13.6 shape, named and written down and still four passes to see. Secondary: AC1's oracle is the whole exit line, so a story that makes five of six clauses green has a **false AC1**. **Dana's dissent recorded verbatim: she wanted AC6+AC7 split off and shipped a week later, and was outvoted on capability, not on cost — eight controls stay red for every day this story takes.**"
kernel_grant: "**+2 physical lines, AUTHORIZED, and it BREACHES a zero-headroom ceiling on contact — both instruments move in this commit.** AC2 adds one single-line `#[maos_attrs::i9_exempt(reason = \"…\")]` to `ScbRuntimeSnapshot` (`scheduler/control_block.rs:244`) and one to `VerifiedImageLock` (`security/sandbox/t3/image_lock.rs:27`); the third fix is a MOVE and is net-0. **Proven by execution, not argued** (§8): `check-empty-kernel` → `PASSED (0 violations)`, pin 24472 → **24474**, tokei 18933 → **18935**, `rustfmt --check` clean. ⚠ `maos-kernel-core` is **18933/18933, ZERO headroom by deliberate design (D13(a))**, so `+2` reds `kloc-check` the moment it lands: `xtask/kloc.toml`'s `maos-kernel-core` row goes to **18935** in the SAME commit as `xtask/kernel-core-baseline.toml`. The authorization is D-F's standing permission, recorded in `kloc.toml`'s own ledger as `\"15-1 +2\"` against a Σ-202 forecast funded in the aggregate and pre-granted to nobody. ⚠⚠ **HARD CONSTRAINT THE EPIC OMITS: the reason string must be ≤ 63 characters.** There is no `rustfmt.toml`, so `max_width = 100`; the attribute's fixed overhead is 37 chars; at 64 the line reflows to three and the delta becomes +6 while `check-fmt` (Blocking, job `check-fmt`, `discipline.yml:140`, in `aggregate.needs`) goes red."
kloc_grant: "⚠⚠ **THIS STORY INHERITS A LIVE BREACH IT DID NOT CAUSE, AND IT CANNOT LAND WITHOUT RESOLVING IT.** `kloc-check` is **RED at HEAD**: `xtask 43411 > 43095` (+316). `maos-domain` (8695/9192 ✅), `maos-bin` (17339/20109 ✅) and the aggregate (157186/170884 ✅, alarm 158608) are all green — 15-2 closed those. The breach reconciles EXACTLY against 15-3's own `kloc.toml` ledger: `check_exit_commands.rs` booked +1143, measured **+1344** (+201); `check_third_party_trial` booked −5, measured **+18** (+23); `check_escape_detector` booked +52, measured **+54** (+2). **201+23+2 = 226.** 15-3 measured mid-story, took the operator grant on that measurement, wrote 226 more `xtask` lines (and +30 `maos-bin`) and never re-measured; its completion note's *\"kloc-check PASS\"*, *\"maos-bin 17109→17309\"* and *\"Aggregate 156840\"* are all false at its own commit (measured 17339 / 157186). ⚠ **REJECTED FUNDING PATH, named so it is not re-proposed:** moving inline `#[cfg(test)] mod tests` out of charged `xtask/src/*.rs` into uncharged `xtask/src/tests/` (a **5653**-line pool across 42 files, measured). It is NOT the `j1-crosshost-1a` precedent `kloc.toml:232` invokes — that precedent is *\"let the COMPILER prove there are no callers… **Deleted**… Reclaimed 108 tokei lines\"*. Moving live test code re-denominates ONE side of a comparison whose ceiling was set in the other unit, and would silently create 5653 lines of headroom nobody granted. **RULED (§7, D-1) and then RE-RULED at the round-table (§12, R-1): the reclaim is EVIDENCE, not funding — 14-0 already spent that pool and three of the 16 surviving warnings are the exact functions it named as un-deletable. Attempt it, record the ≈15–25 line yield against a 316 breach, then take ONE named measured grant at the landing commit. Precedent, same file: `maos-a2a-core` 14-2a, \"RECLAIM WAS ATTEMPTED FIRST AND CANNOT FUND IT.\"** ⚠ `kloc.toml`'s 14-0 block already rules the SHAPE: *\"`xtask` is the gate crate: it grows whenever a control is added, so the zero-headroom shape used for `maos-kernel-core` (18933/18933) and `maos-bin` (16870/16870) is the wrong instrument here — those ceilings exist to STOP growth, this one exists to PRICE it.\"* 15-3 set `xtask` zero-headroom against that ratified note; this story restores the pricing shape. **Tests are free**: `-e tests` is a bare depth-agnostic pattern — `xtask/src/tests/` (2904, 16 files) and `xtask/tests/` (8652, 68 files) are both uncharged; 43411 = 99 files, and 99+16+68 = 183 = the crate's full Rust file count."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs:45-48`) is the machine-checked set; `equiv` is prose, NOT a match token. Keep the literal `allowlist {`  (`check_dev_model_used_populated.rs:302`)."
review: "§A6 full-layer net (Blind + Edge Case + Acceptance + Test-Infra + non-author runtime). **Test-Infra is load-bearing and this story is the reason.** Four separate null controls were found in the very gates this story turns green (§10), and three of this story's four test failures are FLAKY — a single green run proves nothing. **The reviewer MUST: (1) re-run `cargo test --workspace --no-fail-fast` at the landing commit and paste the totals; (2) run each repaired flake ≥40× and paste the failure count; (3) re-derive the `xtask` arithmetic by hand from `tokei` at the landing commit, not from the story; (4) verify `crates/maos-kernel-core/src` is byte-identical in the diff apart from the two attribute lines.** A green suite here proves nothing about whether CI would ever have run it — see §5."
---

# 15-1 — Green at HEAD

Status: **done.**

> **The capability:** *Every hermetic control in this repository is green at HEAD and says so
> for a reason a machine can re-derive — so that every story after this one can prove an exit
> instead of asserting one.*

**Closes:** the epic's S4 (part) · seven red controls, not four · four workspace test failures,
not one · two Blocking CI jobs that could not pass at baseline · kernel-Δ **+2**
(re-pin 24472 → 24474)

---

## What this story actually is

The epic says *"4 red gates + 1 test red"*. Measured at `2fd492c0`, with the command that
produced each number, it is **eight**:

| # | Red | Instrument | Owned by the epic? |
|---|---|---|---|
| 1 | `check-empty-kernel` EXIT 1, 6 violations | `aggregate.needs` | AC2 ✅ |
| 2 | `check-service-boundary` EXIT 1, 19 removed + 17 `other` + 4 P3 | `aggregate.needs` | AC3 ✅ |
| 3 | `kloc-check` EXIT 1 — **`xtask 43411 > 43095`** | `aggregate.needs` | ❌ AC4 names the wrong crates |
| 4 | `check-env-contract` EXIT 1, 2 unregistered vars | `v1-0-ship-gate.needs` | AC5 ✅ (cite drifted 34 lines) |
| 5 | `check-j1-loopback-delegation` EXIT 1, 1 finding | `v1-0-ship-gate.needs` | ❌ AC1's fix is unimplementable |
| 6 | `t_14_2a_a_promoted_generation_…` FAILS | `cargo test --workspace` | AC6 ✅ (cause now proven) |
| 7 | `drr_backpressure_emitted_when_backlog_exceeds_threshold` FAILS | `cargo test --workspace` | ❌ **unlisted anywhere** |
| 8 | `stdio_transport_malformed_output_returns_transport_error` FAILS | `cargo test --workspace` | ❌ **unlisted anywhere** |
| 8b | `kloc_check::tests::kloc_check_runs_on_workspace` FAILS | `cargo test --workspace` | ❌ downstream of #3 |

plus **`a2a-tcp-tests-8-6`** — a Blocking job in `aggregate.needs` that **cannot pass at HEAD**
(§4) — and one structural fact that makes AC1's oracle weaker than it reads: **no CI job runs
`cargo test --workspace`** (§5).

```
$ for g in check-empty-kernel check-service-boundary kloc-check check-env-contract \
           check-j1-loopback-delegation; do ./target/debug/xtask $g; echo "EXIT=$?"; done
EXIT=1  EXIT=1  EXIT=1  EXIT=1  EXIT=1

$ cargo test --workspace --no-fail-fast
EXIT 101 — 4168 passed / 4 failed / 118 ignored, across 497 test-result lines
```

Everything below is what four adversarial scouts DISPROVED in this story's own epic section.

---

## 1. 🔴 THE RED INVENTORY IS WRONG IN BOTH DIRECTIONS — AND THREE OF THE FOUR TEST FAILURES ARE FLAKES

The epic's AC6 names exactly one test and concedes *"other reds not measured — the story
records them."* An unmeasured obligation inside an AC is unbounded scope. It is now measured.

`cargo test --workspace --no-fail-fast` at `2fd492c0`: **EXIT 101, 4168 passed / 4 failed /
118 ignored.**

| Test | Crate | Deterministic? | Measured |
|---|---|---|---|
| `t_14_2a_a_promoted_generation_makes_the_retired_leaf_a_queryable_refusal` | `maos-a2a-tcp` | **NO — flaky** | 29/40 fail @default, **0/40 @2 threads**, 0/40 alone |
| `drr_backpressure_emitted_when_backlog_exceeds_threshold` | `maos-kernel-core` | **NO — flaky** | 3/20 @default, 1/20 @1 thread, **0/20 alone** |
| `stdio_transport_malformed_output_returns_transport_error` | `maos-mcp` | **NO — flaky** | 7/20 @default, **0/20 alone** |
| `kloc_check::tests::kloc_check_runs_on_workspace` | `xtask` | YES | fails on `["xtask 43411 > 43095"]` — downstream of §7 |

⚠ **THE ORACLE CONSEQUENCE, AND IT IS THE MOST IMPORTANT SENTENCE IN THIS FILE.** Three of the
four are races. **A single green run of a repaired flake is a null control.** Every fix in AC6
ships with a repeat count and a measured failure rate, in both directions — the pre-fix rate
recorded, the post-fix rate proven at ≥40 runs. This is the `14-1` *"the derivation was
repaired, the input left canned"* shape one layer over: repair the test, leave the oracle
single-shot, and the story is green by luck.

⚠ **The fourth is not a test failure at all — it is §7 wearing a test's clothes.**
`kloc_check_runs_on_workspace` (`xtask/src/tests/kloc_check_tests.rs:40`) was `#[ignore]`d
since Epic 5 and **un-ignored by 15-2 AC6(f)** on the argument that the re-base finally made it
pass. It did — for one commit. 15-3 reddened it immediately and shipped anyway, claiming
`cargo test -p xtask` → *"850 passed / 0 failed"*. It is the only real control over the
ceilings (§10) and it did its job on the first commit after it was armed.

---

## 2. 🔴 AC1's CI CITATIONS ARE FALSE IN KIND, NOT MERELY IN LINE — AND IT IS THE SAME ERROR 15-3 ALREADY FILED

AC1 says the J1 gate is *"Blocking: job `discipline.yml:1904`, in `aggregate.needs` at `:3410`"*.

| AC1 claim | Measured @`2fd492c0` | Verdict |
|---|---|---|
| `aggregate` at `:3510`, 123 `needs:` | job `aggregate:` at **`:3540`**; **123 needs VERIFIED** (`:3543`–`:3665`) | line DRIFTED, count TRUE |
| J1 job at `:1904` | **`:1939`** (block ends `:2032`) | DRIFTED |
| *"in `aggregate.needs` at `:3410`"* | **NOT IN `aggregate.needs` AT ALL.** It is in **`v1-0-ship-gate.needs` at `:3439`** (that job at `:3414`, 46 needs). `v1-0-ship-gate` is itself in `aggregate.needs`, so the dependency is **transitive** | **FALSE IN KIND** |
| Blocking | **VERIFIED** — `gate-registry.toml:327-328` `{v1_0="blocking", v1_5="blocking"}`; `BindingClass::Blocking` at `check_j1_loopback_delegation.rs:1110` | TRUE |

**This is the exact error 15-3's own refinement round caught and recorded** — the epic header
already says *"the new gate belongs in `v1-0-ship-gate.needs`, not `aggregate.needs`"* — and
AC1 re-commits it two sections above that sentence. Which list a gate sits in is not
bookkeeping: `extract_ship_gate_needs` reads only the ship-gate job, and enrolling in the wrong
one reds `check-ship-gate-completeness` on run 1.

**Corrected map of AC1's own five gates:**

| Gate | `aggregate.needs` | `v1-0-ship-gate.needs` |
|---|---|---|
| `check-empty-kernel`, `check-service-boundary`, `kloc-check` | **YES** | no |
| `check-env-contract`, `check-j1-loopback-delegation`, `check-exit-commands` | no | **YES** |

**Advisory audit (AC1: "no gate demoted to advisory") — DISCHARGED, with one flag.** 38
`[[ship_gate]]` rows; effective at `CURRENT_PHASE = "v1_5"` (`gate_common.rs:166`): **21
blocking, 14 advisory, 3 blocking-when-present.** Every advisory row is phase-ladder-scheduled
(→ v2_0 or v2_2) or substrate/engagement-gated. **None is silently demoted**, and
`check-epic-close-green` confirms *"11 workflow files scanned, 0 `if: false`-disabled jobs"*.
D20 landed as 15-3 claimed: `check_cohort_mesh.rs:18` whole-gate `Blocking`;
`check_escape_detector.rs:160/:180` mixed per-leg. ⚠ **ONE FLAG, filed not fixed:**
`check-cross-form-equiv` is `{v1_0="advisory", v1_5="advisory"}` with **no v2_0/v2_2 row**, so
it inherits advisory forever — the only demotion in the registry with no end date.

---

## 3. 🔴 AC1's STATED J1 FIX CANNOT BE IMPLEMENTED, AND CONTRADICTS ITSELF IN THE SAME SENTENCE

AC1 prescribes: *"the suffix derivation is narrowed to J1 stems (e.g. `j1_*_{1a,1b,2a,2b}.rs`
or an explicit stem list) … the gate's proven-red vector (a planted un-enrolled J1 file) is
kept."*

**Both halves of that sentence cannot be true at once.**

`derive_enrolled_targets` (`check_j1_loopback_delegation.rs:765`, filter at `:788`) reads
`crates/maos-bin/tests` and keeps every file ending in one of
`J1_TEST_SUFFIXES = ["_1a.rs","_1b.rs","_2a.rs","_2b.rs"]` (`:114`, verified verbatim). Applied
by hand to the 40 real entries, **9 files match and exactly ONE is a false positive**:

| File | Enrolled in the j1 job? | Genuinely J1? |
|---|---|---|
| `bounded_postures_2b.rs`, `consent_refusal_1b.rs`, `delegation_leg_1a.rs`, `host_grants_2b.rs`, `topology_delegation_1a.rs`, `two_host_delegation_2b.rs`, `worker_completion_2a.rs`, `worker_manifests_2a.rs` | YES (8/8) | YES |
| **`cert_rotation_trigger_14_2a.rs`** | **NO** | **NO — story 14-2a** |

**Why the epic's form is fatal — three independent reasons:**

1. **`j1_*` derives ZERO targets.** *None* of the eight genuine J1 files carries a `j1_`
   prefix. `leg_completion_vectors_enrolled` (`:809`) calls `audit.checked()` once per target,
   so `checks == 0` → `is_vacuous()` → the synthetic `leg-vacuity` finding (`:1081`;
   `gate_common.rs:297-299`) → **the gate stays EXIT 1, in a new way.**
2. **It breaks the vector AC1 promises to keep.**
   `xtask/tests/j1_crosshost_1b_proven_red.rs:621`
   `planting_an_unenrolled_j1_test_file_must_red` plants **`crates/maos-bin/tests/xyz_1b.rs`**
   and requires `passed:false` plus the substring `--test xyz_1b`. `xyz_1b.rs` has no `j1_`
   prefix and can never be in a hand-list. Under either epic form the gate goes GREEN and the
   vector fails with *"PROVEN-RED FAILURE — the gate stayed GREEN"*.
3. **The "explicit stem list" re-creates the defect `1b` deleted.** The gate's own doc
   (`:101-108`) condemns it by name: *"`2a` shipped this leg against a hand-maintained
   `ENROLLED_TEST_TARGETS` const… a const list is one forgotten line away from a dead test
   behind a green gate."*

**And option (B) — the fix the gate's own message asks for — is worse.** Enrolling
`cargo test -p maos-bin --test cert_rotation_trigger_14_2a` in the j1 job looks right, because
`grep -rn "cert_rotation_trigger_14_2a" .github/` returns **zero** hits and no job runs
`cargo test -p maos-bin` unscoped. **But the file is not dead.**
`xtask/src/check_cert_rotation_trigger.rs` shells `cargo test` itself — `invoke_cargo_test`
(`:595`), `Command::new("cargo")` (`:596`), `--test-threads=1` (`:613`), driven by `TEST_FILES`
(`:92`, which lists `("maos-bin", "cert_rotation_trigger_14_2a", "crates/maos-bin/tests")`).
Measured: `check-cert-rotation-trigger` **EXIT 0**, *"oracle green (4 legs, 39 enrolled rotation
tests)"*. So (B) buys **zero** coverage and pays: double execution of a real-daemon test, on a
job that has **no `timeout-minutes`** (`:1939`, versus 20 on its cert-rotation sibling), while
permanently entrenching the claim that a 14-2a file is a J1 vector.

> **The epic's own words are also misleading here.** *"the test itself stays enrolled where it
> is (`check-cert-rotation-trigger`)"* is TRUE IN SUBSTANCE but invites a reader to grep
> `discipline.yml` and conclude the opposite: **zero `--test cert_rotation_trigger_14_2a`
> tokens exist in any of the 11 workflow files.** It is executed by the gate's subprocess.

⚠ **Latent blind spot, filed not fixed:** `_2c.rs` is not in `J1_TEST_SUFFIXES`, and all six
`j1-crosshost-2c` tests live outside `crates/maos-bin/tests`. The derivation cannot see the 2c
lane at all. **Do not let the narrowing be sold as making the derivation complete.**

---

## 4. 🔴 AC6 CAN TURN A CURRENTLY-GREEN BLOCKING GATE RED — AND A SECOND BLOCKING JOB HAS BEEN UNPASSABLE ALL ALONG

### 4a. The root cause, PROVEN by isolation (the epic says it is unmeasured)

The epic's AC6 says only *"inter-test interference, not the subscriber arrangement …; the root
cause is measured and recorded before the fix (candidates: tracing callsite-interest cache;
handshake-arm timing; no `static`/`OnceLock` in `maos-a2a-tcp/src`)."* The third candidate is
why nobody found it: **the shared mutable state is not in MAOS source. It is the global
callsite-interest cache inside `tracing_core` (`tracing` 0.1.44).**

`t_14_2a_post_grace_journal.rs` holds three tests. **Only one installs a subscriber** —
`tracing::subscriber::with_default(LogCapture(...), …)` at `:267`, a *thread-local* default.
`LogCapture` (`:376-401`) overrides `enabled`/`event` but **not `register_callsite`**. Two
tests traverse the same `router.rs` refusal `warn!` callsite; the other has **no subscriber at
all**. Whichever thread reaches the callsite first fixes its `Interest` process-globally — and
if that thread is the one without a subscriber, the capturing test records `[]`.

**Isolated by pairing (the discriminating experiment):**

| Combination | Result |
|---|---|
| C (`…promoted_generation…`) + A (`…retired_leaf_presented_after_the_grace…`) | **21/30 FAIL** |
| C + B (`…current_generation_handshake_journals_no_post_grace_row`) | **0/30** |
| C alone | **0/30** |

A is the poisoner: it is the other test that reaches the refusal path, and it installs nothing.
That is also why `--test-threads=2` is clean — with two threads, A is scheduled after C
completes.

**Failure rate by thread count** (`target/debug/deps/t_14_2a_post_grace_journal-*`, 32-core box):

| `--test-threads` | 1 | 2 | 3 | 4 | 8 | default (32) |
|---|---|---|---|---|---|---|
| failures | 0/40 | 0/40 | 18/30 | 23/30 | **24/30** | 29/40 |

### 4b. ⚠ THE ORDERING CONSTRAINT AC6 DOES NOT STATE

AC6 says *"`check-cert-rotation-trigger` drops `--test-threads=1` (`check_cert_rotation_trigger.rs:613`)
so gate-green equals suite-green."* **That gate is GREEN today precisely because of that flag,
and it EXECUTES this test.** Dropping the flag before the race is fixed moves a green Blocking
gate onto the 24/30-failure path. **The flag comes out AFTER the fix is proven, in the same
commit, and the gate is re-run to prove it — never before.**

### 4c. ⚠ A SIXTH RED THE EPIC DOES NOT LIST: A BLOCKING JOB THAT CANNOT PASS

`discipline.yml:1642`, job **`a2a-tcp-tests-8-6`** — in **`aggregate.needs` at `:3623`**, no
`continue-on-error`, no `if:`:

```yaml
- name: Run a2a-tcp suite 50× (determinism — AC-T13)
  run: |
    for i in $(seq 1 50); do
      cargo test -p maos-a2a-tcp --locked -- --test-threads=8 || { echo "FLAKE on run $i"; exit 1; }
    done
```

At a measured 24/30 per-run failure rate at exactly 8 threads, the probability this job passes
50 consecutive runs is ≈ 0.2^50. **It has been red by construction since the flake was
introduced, and no story owns it.** Its own comment names the failure mode it was built to
stop: *"this is the gate that stops this security story becoming §A2-style CI-only-flake debt."*
AC6's fix closes it — but only if AC6's oracle runs the suite the way that job does.

### 4d. The other two failures — different causes, both measured

**`drr_backpressure_emitted_when_backlog_exceeds_threshold`**
(`crates/maos-kernel-core/tests/drr_scheduler.rs:140`) has no tracing at all. It spawns submit
tasks, `tokio::time::sleep(50ms)`, then drains `bw_rx` with `try_recv()` and asserts at least
one budget warning arrived. Under load the DRR processor has not necessarily emitted within
50 ms. **A fixed sleep used as a synchronisation primitive.** 3/20 default, 1/20 even at
`--test-threads=1`, 0/20 alone.

**`stdio_transport_malformed_output_returns_transport_error`**
(`crates/maos-mcp/tests/stdio_transport_test.rs:45`) is **not a parse problem — it is EPIPE.**
The fixture is `sh -c "echo 'not json at all'"`, which **never reads stdin**.
`StdioTransport::spawn_and_invoke` (`crates/maos-mcp/src/transport/stdio.rs`) writes the JSON
request to the child's stdin *before* reading stdout. When the child wins the race the write
returns `EPIPE` and the error is `"write: Broken pipe (os error 32)"`, which does not contain
`"parse"`. **Mechanism reproduced 300/300 outside the repo** with a 2 ms scheduling delay
standing in for load. 7/20 default, 0/20 alone.

> ⚠ **The fix is the fixture, not the transport.** `sh -c "cat >/dev/null; echo …"` makes the
> child read its request first and removes the race deterministically while preserving the
> test's intent. **File, do not fix here:** `spawn_and_invoke` treats a write-side EPIPE as
> fatal even when the child has already produced a complete answer — a real robustness question
> for a one-shot NDJSON server, and out of scope for a green-at-HEAD story.
> ⚠ **R-5: the filed item carries the reproduction, not a summary.** The 300/300 measurement and
> the exact call ordering in `spawn_and_invoke` go INTO the filed item, so the next owner does
> not have to rediscover the mechanism. A filed item reading *"consider reviewing EPIPE
> handling"* is a to-do, and to-dos decay.

---

## 5. 🔴 THE EXIT LINE ENDS IN A COMMAND CI NEVER RUNS

AC1's oracle is *"the first exit line exits 0 … and the discipline `aggregate` is green"*, and
its last clause is `cargo test --workspace --no-fail-fast`.

```
$ grep -rniE "cargo (test|nextest)" .github/workflows/*.yml | grep -viE "\-\-test |--bin |--lib |-p "
   (no unscoped invocation)
```

**Every `cargo test` in `discipline.yml` is `-p`/`--test`-scoped, and `journey-nightly.yml`'s
two `cargo nextest run` invocations are `-p maos-journey-test`-scoped.** A green `aggregate`
therefore proves nothing about the exit line's final clause, and the two halves of AC1 are not
one oracle but two — one machine-checked, one performed by hand on a developer's box.

That is the project's own documented signature failure: *a claim standing in for a control*
(Epic-12 retro, 11 instances). A story whose AC1 is *"this command exits 0"* where the command
is run by nobody is green the day it lands and unverifiable the day after.

**RULED (D-3, §Decisions): the suite becomes a control in this story.** The cost is real and is
stated rather than hidden — one workspace test run per CI cycle. The alternative that was
rejected: labelling the clause "local acceptance step". It was rejected because AC1 already
says *"the aggregate is green with no gate demoted to advisory"*, and a clause CI cannot see is
a demotion in everything but name.

---

## 6. 🔴 AC3's 9/8 SPLIT IS 18/1, AND THE AC's TWO HALVES ANNIHILATE EACH OTHER

AC3 says the 19 `removed` rows split into *"signature-changed (~9 …)"* and *"genuinely removed
(`SpawnError`, `UpgradeError`, `UpgradeOrchestrator`, `UpgradeOutcome`, `UpgradeReport`,
`UpgradePolicy`, `RevocationApplier`, `get_default_image`)"*.

**Measured, by testing whether the same `(kind, path)` also appears in `current − baseline`:
18 of 19 removed triples are signature-hash refreshes of symbols that still exist. Exactly ONE
is a genuine removal.** Seven of the epic's eight "genuinely removed" symbols are alive at HEAD.

| Epic said | Measured |
|---|---|
| 8 signature-changed, listed | all 8 correct — but **incomplete**: `SecurityManagerAdapter` is also signature-changed and is listed nowhere |
| 8 genuinely removed | **7 of 8 FALSE.** `SpawnError` (+2 variants, purely additive), `UpgradeError`, `UpgradeOrchestrator`, `UpgradeOutcome`, `UpgradeReport`, `UpgradePolicy`, `RevocationApplier` are all alive and all **already classified** in `kernel-api-classes.toml` (`:122`, `:541-546`) |
| — | **`get_default_image` is the only genuine removal**, deleted by `af788c3e`, replaced in-place by `VerifiedImageLock::resolve_pin`. It was an *unverified fallback image URI*; its deletion is a security repair |

**How the error was made, and it is instructive:** the epic inferred "genuinely removed" from
*the absence of an `other` violation row*. Those seven emit no `other` row **because they are
already classified**. Silence was read as absence.

**Root cause of the whole red, measured not titled:** the baseline was last blessed at
`accf763c` (2026-08-07). Only three kernel-core commits since. **`af788c3e`** (*"fix(epic-5):
close review findings and harden release gates"*) touched **30 kernel-core files** and updated
**neither** `docs/ci-baselines/kernel-surface-v0.1-beta.json` **nor** `kernel-api-classes.toml`.
`kloc.toml:199` already indicts the same commit for the same shape — *"the paired CEILING grant
that `af788c3e` owed and never took … so the two instruments disagreed for 13 days."* **Same
commit, second instrument.**

⚠ **THE TWO HALVES OF AC3 CANCEL.** `removed` has exactly one input (the baseline JSON), and
`new` is `current − baseline`. **Re-emitting the baseline closes all 19 removed rows AND all 17
`other` rows at once** — proven by executing a full re-emit with `kernel-api-classes.toml`
untouched: **`NFR-Test-2 rows remaining: 0`**. Under a full re-emit the epic's 16 classification
rows are **unreachable by construction** — 16 rows of "invariant-lock review" judgement
committed as decoration. **Classify OR re-emit; the AC asks for both and gets one.**

⚠ **`--write-baseline` is unaffordable and its funding source does not exist.** Measured cost:
**15 charged xtask lines**, onto a ceiling already **316 over** (§7). AC3 charges it *"to the
xtask row of 15-2"* — but that row is a **15-3** grant, dated 2026-09-07, declared ZERO
HEADROOM. The named funding source is not there. The hand-edit costs **0** xtask lines and has
**five commits of precedent**, the gold standard being `1dda03eb` — a 1-insertion/1-deletion
re-bless carrying its symmetric-diff proof in the commit message.

⚠ **And the note cannot live where AC3 puts it.** `KernelSurface` derives plain `Deserialize`
(no `deny_unknown_fields`), so an added `"_invariant_lock_review"` key parses — but `Serialize`
emits only the three declared fields, so **any future re-emit silently destroys it.**

⚠ **Latent gate defect, filed not fixed:** five of the nineteen removed rows come from ONE
identifier. `lifecycle/mod.rs:16-19`'s `pub use upgrade::{…}` group is hashed once and the hash
attributed to every path in it, so adding one name to a re-export emitted five
`removed public kernel symbol` alarms. **"19 rows" wildly overstates the change.**

---

## 7. 🔴 AC4 IS DISCHARGED AND VACUOUS, AND THE LIVE kloc RED IS IN A CRATE IT DOES NOT NAME

| AC4 clause | Measured @HEAD |
|---|---|
| *"`maos-domain` +51 (`kloc-check`: 8695/8644 OVER)"* | **8695 / 9192 ✅ ok** — closed by 15-2 |
| *"the two `EnvVar` blocks … in `maos-bin` (17109/17109)"* | **17339 / 20109 ✅ ok**, headroom 2770 |
| *"D17 closes when `_aggregate_hardfail` is re-derived there"* | **D17 and D14 are already CLOSED against `15-2-kloc-ceiling-rebase`**, by 15-2 AC7. `check-decision-register` → *"23 rows, 12 open"*, exit 0; neither names 15-1 |

**Every clause of AC4 is discharged, drifted, or someone else's. As written it is a no-op AC a
dev could claim credit for by doing nothing** — the exact "claim standing in for a control"
failure mode the Epic-12 retro named as this project's signature.

**But the kloc red did not go away. It moved crates.**

```
$ ./target/debug/xtask kloc-check          # EXIT 1
per-crate KLOC ceiling breach: xtask 43411 > 43095;
workspace aggregate=157186 is UNDER its configured threshold=170884
```

**Attribution, reconciled to the line against 15-3's own ledger:**

| file | 15-3 BOOKED | MEASURED @HEAD | unbooked |
|---|---|---|---|
| `check_exit_commands.rs` | +1143 | **+1344** | **+201** |
| `check_third_party_trial.rs` | −5 | **+18** | **+23** |
| `check_escape_detector.rs` | +52 | **+54** | **+2** |
| | | | **= 226 EXACT at `bce5ac68`** |
| `check_exit_commands.rs` again, in the **amend** to `2fd492c0` | not booked | **+90** | **+90** |
| | | | **= 316 at HEAD** |

Every other booked figure reproduces exactly. **15-3 measured mid-story, took the operator
grant on that measurement, then wrote 226 more `xtask` lines and never re-measured.** The same
pass mis-booked `maos-bin` (+30) and the aggregate (+256 = 226 + 30). Its ledger does not even
sum to the row it justifies — 42048 + its own per-file list = 43090, not 43095.

**RULED (D-1): fund it the way 14-0 did, in that order.** `kloc.toml:232` — *"1. FUND BY
RECLAIM FIRST, the `j1-crosshost-1a` precedent, and let the COMPILER prove there are no callers
rather than trusting a grep … **Deleted** … Reclaimed 108 tokei lines. 2. THEN MEASURE, THEN
ASK … Ceiling = final measured + 35, authorized only after the patched tree existed."*

And the same block already rules the SHAPE, against 15-3: *"`xtask` is the gate crate: it grows
whenever a control is added, so the zero-headroom shape used for `maos-kernel-core`
(18933/18933) and `maos-bin` (16870/16870) is the **wrong instrument here** — those ceilings
exist to STOP growth, this one exists to **PRICE** it."*

⚠ **REJECTED, and named so it is not re-proposed:** funding the breach by moving inline
`#[cfg(test)] mod tests` out of charged `xtask/src/*.rs` into uncharged `xtask/src/tests/`.
The pool is real — **5653 tokei lines across 42 files**, measured, led by `evidence_ledger.rs`
(582), `check_dev_record_completeness.rs` (430), `check_error_catalog.rs` (406). But it is **not
the `j1-crosshost-1a` precedent**, which is *deletion of compiler-proven dead code*. Moving live
test code re-denominates one side of a comparison whose ceiling was set in the other unit; the
breach survives the unit change identically, and the operation would silently create 5653 lines
of headroom nobody granted. **It launders the overspend rather than paying it.** (The
observation that inline tests are charged at all is nonetheless true and worth a filed note —
`kloc.toml`'s own header says *"production Rust code only"* — but re-denominating both sides is
an epic-retrospective decision, not a green-at-HEAD story's.)

---

## 8. ✅ AC2 IS SOUND — AND WAS PROVEN END-TO-END, WITH ONE CONSTRAINT THE EPIC OMITS

Every mechanism claim in AC2 verified, and the fix was executed against a scratch copy of
`crates/maos-kernel-core` with the real binary:

| Instrument | Before | After |
|---|---|---|
| `check-empty-kernel` | EXIT 1, 6 violations | **`PASSED (0 violations)`** |
| pin (`content.lines().count()`, `check_kernel_baseline.rs:99-112`) | 24472 | **24474 (+2)** |
| tokei `code` (`maos-kernel-core` kloc row) | 18933 | **18935 (+2)** |
| `rustfmt --edition 2021 --check` on all three edited files | — | **clean** |

**The misattachment is real and its provenance is the same commit as §6.** `git log -S` puts it
at **`af788c3e`**, which inserted `T3ImageVerificationConfig` *between* the
`SecurityManagerAdapter` doc comment + exemption and its own struct, silently re-parenting both.
The orphaned attribute's `reason` string still literally reads *"security manager adapter; holds
Arc<PolicyTable> …"* while sitting on a struct with a `PathBuf` and a `[u8; 32]`. Moving it back
is net-0 and removes the `:124` undocumented-site violation; `T3ImageVerificationConfig`'s
fields are not denylisted, so it needs no entry (**verified by execution**, not by reading).

⚠ **The constraint the epic does not state, and it changes the arithmetic.** No `rustfmt.toml`
exists → `max_width = 100`. Attribute overhead `#[maos_attrs::i9_exempt(reason = "` (34) + `")]`
(3) = 37. **Binary-searched: 100 chars KEEP, 101 REFLOW.** So **the reason string must be ≤ 63
characters** or the line becomes three and the delta becomes +6 while `check-fmt` (Blocking,
job `check-fmt` at `discipline.yml:140`, step `:151-152`, `aggregate.needs :3545`) goes red. The
epic's cited precedent `maos-capability/src/cap_quota/mod.rs:37` measures 95 chars / reason 58.

⚠ **The whitelist is not merely "semantically wrong" — it is mechanically blocked.**
`xtask/src/tests/check_empty_kernel_tests.rs::whitelist_has_exactly_three_paths` **hard-asserts
`paths.len() == 3`**. A fourth entry reds a unit test. Stronger than the epic's reason; use it.

⚠ **AC2 and AC3 must land in ONE commit.** The four surviving `P3 violation` rows in
`check-service-boundary` are `ScbRuntimeSnapshot.spirit_obj`, `SecurityManagerAdapter.policy`,
`SecurityManagerAdapter.provider_history`, `VerifiedImageLock.attestations` — the *same* symbols
AC3 classifies, surfaced through AC2's gate. Split them and neither AC can prove itself.

⚠ **`maos-kernel-core` is 18933/18933 with ZERO headroom by design.** The `+2` breaches on
contact; the kloc row goes to 18935 in the same commit as the pin (§`kernel_grant`).

---

## 9. 🔴 AC5's CITE DRIFTED 34 LINES, AND THE GATE POINTS OPERATORS AT A STORY THAT DOES NOT EXIST

```
$ ./target/debug/xtask check-env-contract     # EXIT 1
  VIOLATION: crates/maos-bin/src/main.rs:2703: env::var("MAOS_OPERATOR_BEARER_TOKEN") not in env_contract.rs registry
  VIOLATION: crates/maos-bin/src/main.rs:2704: env::var("MAOS_OPERATOR_HTTP_BIND") not in env_contract.rs registry
check-env-contract: FAIL — 2 unregistered maos-bin/src MAOS_* env reads (workspace coverage tracked in Story 12.7)
```

AC5 cites `main.rs:2669-2670`; measured **`:2703-2704`**. The `env_contract.rs:13` registry cite
and the `:14-18` row-format cite are **VERIFIED exactly** — a rare clean pair in this section.

**The surface is LIVE, not a stub.** `main.rs:2702-2725` validates the pair at the composition
root (`Ok(token)` *enables* the surface; bind defaults to `127.0.0.1:8787`; `(Err, Ok)` is a
typed refusal — *"MAOS_OPERATOR_BEARER_TOKEN is required when MAOS_OPERATOR_HTTP_BIND is
configured"*), and `main.rs:2824-2841` binds a real listener via
`maos_control::OperatorHttpServer::bind`. It is the surface `maosctl spirit inspect --sandbox`
depends on, and `crates/maos-bin/tests/cert_rotation_trigger_14_2a.rs:245-246` sets both vars.

**Classification: both `UserFacing`.** `EnvStability` has exactly two variants — there is no
`Secret`. The precedent is decisive: `MAOS_KMS_MASTER_KEY`, `MAOS_SSO_JWKS`,
`MAOS_VETTER_KEYRING`, `MAOS_CROSS_TEAM_BASE_SEED` are all `UserFacing`; all 11 `HarnessOnly`
rows are test/bench/replay knobs. A live production HTTP surface is not a harness.

⚠ **The gate's own operator message names a dead key.** *"workspace coverage tracked in Story
12.7"* appears on **both** its PASS and FAIL paths. `sprint-status.yaml:347` records that no
`12-7` key survives and that the single home became Epic 14's `14-7`/`14-8`/`14-9` — **and none
of those exists as a sprint key either** (Epic 14 closed by re-scope). A Blocking gate is
telling operators to consult three phantoms. Repointing costs 0 tokei lines (D-I / 21-4 /
`xtask/env-contract.toml` is the surviving home).

⚠ **Scope honesty — AC5 must not overclaim.** `MAOS_OPERATOR_BEARER_TOKEN` is **also** read at
`crates/maos-cli/src/subcommands.rs:1940`, alongside a **third** variable
`MAOS_OPERATOR_HTTP_ENDPOINT` at `:1938`. Both are outside the gate's scope
(`check_env_contract.rs:119`: `let src_dir = Path::new(maos_bin_dir).join("src");`) and both stay
permanently unregistered. AC5 registers **the two reads the gate can see**, not "the operator
surface's env contract".

---

## 10. NULL CONTROLS FOUND IN THE GATES THIS STORY TURNS GREEN

Every one of these is a place where the *only* live oracle is the red this story is about to
remove.

| Control | Status |
|---|---|
| `check_empty_kernel_tests.rs::missing_exemption_doc_fires` | **NULL.** It never calls `run_silent` — it **reimplements** the matcher inline and asserts on its own copy, which has **already diverged** (`exemptions_src.contains(..)` vs production's `.lines().any(..)`). Delete the production cross-check and this test still passes |
| `empty_kernel_integration.rs::clean_i9_passes` | **NULL for the exemption leg** — its fixture has no exempt struct and no denylisted field |
| The whole exemption-documentation leg | **No test anywhere exercises "an `#[i9_exempt]` site is undocumented" through `run_silent`.** The only executable proof at HEAD is the live red AC2 removes |
| `xtask/tests/service_boundary_integration.rs` | **9 of 10 pass `--baseline /dev/null`** → empty → the entire baseline-diff branch is skipped. A live saboteur's door |
| Any test over the shipped baseline JSON | **NONE.** `grep` for `kernel-surface-v0.1-beta.json` across `xtask/tests/` + `xtask/src/tests/` → **zero hits**. Deleting, emptying or fully re-emitting it breaks no test |
| `kloc_check::tests::kloc_check_runs_on_workspace` | **REAL, and the only one** — un-ignored by 15-2, red at HEAD, and it caught 15-3 on the first commit after it was armed (§1) |
| `check_env_contract` unit tests | **REAL** — two proven-red, two proven-green over the tricky shapes |

**Consequence, and it is an AC not a note:** three of this story's ACs remove the only live
oracle for the property they close. Each must leave a replacement behind (AC2 → an
exempt-but-undocumented fixture; AC3 → a baseline-absent-symbol vector that closes the
`/dev/null` hole; AC6 → repeat-count oracles). All are free — `xtask/tests/` and
`xtask/src/tests/` are uncharged.

---

## Decisions ratified in this story (rule 7 — one decision per fork)

| # | Fork | Ruling | Loser, and why |
|---|---|---|---|
| **D-1** | How to close `xtask 43411 > 43095` | ⚠ **RE-RULED at the 2026-09-07 round-table (§12, R-1): reclaim is EVIDENCE, not funding — 14-0 already spent that pool.** Measured at HEAD: 16 `xtask/src` warnings, of which **three are the exact functions 14-0 named as NOT deletable** (`evidence_ledger::detail`, `check_fkcs::synthetic`, `check_fkcs::read_nonempty_lines` — live callees of `xtask/tests/` targets); the rest are ~10 unused struct fields, 2 dead assignments, 1 unused import. **Realistic yield ≈15–25 lines against a 316 breach.** So: attempt the reclaim, **record that it cannot fund**, then take ONE named measured grant at 15-1's landing commit — `xtask = final measured + 35`, restoring the PRICING shape `kloc.toml`'s 14-0 block ratifies for the gate crate (*"this one exists to PRICE it"*). Precedent verbatim, same file, `maos-a2a-core` 14-2a: *"RECLAIM WAS ATTEMPTED FIRST AND CANNOT FUND IT."* **R-2: the 226 is booked as 15-3's, itemised +201/+23/+2, *filed not overwritten* (D13(a)); 15-1's own spend is a separate line in the same comment.** | (i) *Move inline tests to `xtask/src/tests/`* — not the cited precedent, re-denominates one side, silently grants 5653 lines. (ii) *Reopen 15-3 to pay its own overspend* — two grants on one row across a number that moves in between = the drift D13 spent thirteen days demonstrating; 15-1 must re-set that row anyway (AC2 +2 kernel, AC1/AC3 xtask). (iii) *Bare grant with no reclaim attempt* — skips the evidence the precedent requires. (iv) *Retire `check_epic_6_bridge.rs`* (4025 lines) — **steals 20-3's funding** |
| **D-2** | The J1 gate's false positive | **Keep the filesystem derivation and all four suffixes; exclude any file whose suffix is immediately preceded by a digit.** An epic-numbered stem (`…_14_2a.rs`) is a foreign-lane test; a J1 stem (`…_2a.rs`, `xyz_1b.rs`) is not | (i) *`j1_*` prefix* — derives **0** targets → vacuity red, and breaks the gate's own `xyz_1b.rs` vector. (ii) *Explicit stem list* — resurrects `ENROLLED_TEST_TARGETS`, condemned by name in the gate's own doc. (iii) *Enroll the 14-2a test* — buys zero coverage (its own gate already shells it), double-runs a real-daemon test on an untimed job, entrenches a category error |
| **D-3** | `cargo test --workspace` is run by no CI job | **Enroll it as a Blocking job in `aggregate.needs`.** The exit line's last clause becomes machine-checked | *"Local acceptance step"* — AC1 already promises "no gate demoted to advisory", and a clause CI cannot see is a demotion in everything but name |
| **D-4** | AC3: classify, re-emit, or both | **Surgical hand-edit of the baseline (18 hash refreshes + 1 deletion) + classify only the 8 genuinely-new paths.** The baseline stays a pure removal/mutation anchor; `kernel-api-classes.toml` stays the artifact a human must reason about | (i) *Full re-emit* — proven one-button green (0 rows with classes untouched) and makes the classification rows unreachable. (ii) *`--write-baseline`* — +15 xtask lines onto a 316-over ceiling, funding source does not exist, and it institutionalises the one-button green as a CLI verb |
| **D-5** | Where the sanctioned-removal note lives | **A new ADR (next free number, `ADR-066`).** `invariant-lock` maps `I1..I14 → docs/invariants/I*.md` only and never fires on `kernel-api-classes.toml` or the baseline JSON — the "invariant-lock review" line in both is a **prose convention**, exactly as `ADR-065:87-92` (landed by 15-3) already adjudicates for `gate-registry.toml`. ADR-043 → ADR-065 is the discharging chain | *A key inside the baseline JSON* — `Serialize` emits only three fields, so any future re-emit silently destroys it |
| **D-7** | AC6(a): WHICH fix (the story proved the cause and left the fix unruled — a rule-7 gap found after the round-table) | **Default: process isolation.** Move `t_14_2a_a_promoted_generation_makes_the_retired_leaf_a_queryable_refusal` into its own test binary (`crates/maos-a2a-tcp/tests/t_14_2a_post_grace_token.rs`). A separate binary is a separate **process**, so no co-resident test can register the `warn!` callsite before it — the defect is removed **by construction**, with no dependency on `tracing_core` internals. Cost: one file in an uncharged directory. **Permitted alternative, only if measured:** give `LogCapture` an explicit `register_callsite` returning `Interest::sometimes()` (3 lines, keeps one file) — allowed **only** if it measures 0/40 at default threads AND 0/50 at `--test-threads=8` in the 3-test binary. It is not ruled in because whether a thread-local subscriber's interest survives a foreign dispatcher registering the callsite first is a `tracing_core` behaviour this preflight did **not** measure | (i) *Install `LogCapture` in the co-resident test too* — changes what that test proves to fix a different one. (ii) *`rebuild_interest_cache()` after `with_default`* — still order-dependent under concurrency, so it narrows the window rather than closing it. (iii) *Assert on the durable rupture row instead of tracing* — the test's own doc records that the frame has no detail field, which is why the tracing surface was chosen |
| **D-6** | AC6 ordering | **Fix the race first, prove it at ≥40 runs, and drop `--test-threads=1` from `check_cert_rotation_trigger.rs:613` in the SAME commit, re-running the gate to prove it stayed green.** | *Drop the flag as written in AC6* — moves a currently-GREEN Blocking gate onto a 24/30-failure path |

---

## Acceptance Criteria (7)

- **AC1 (command).** The epic's exit-block line 1 exits **0**, and the discipline `aggregate`
  (job `aggregate:`, `discipline.yml:3540`, **123** `needs:` at `:3543-3665`) is green with no
  gate demoted to advisory. The seven controls it closes, each re-measured at the landing commit
  and each with its true CI home stated: `check-empty-kernel` (AC2, `aggregate.needs`);
  `check-service-boundary` (AC3, `aggregate.needs`); `kloc-check` (AC4, `aggregate.needs`);
  `check-env-contract` (AC5, **`v1-0-ship-gate.needs`**); `check-j1-loopback-delegation` (this
  AC, **`v1-0-ship-gate.needs:3439`** — *not* `aggregate.needs`; the aggregate depends on it
  transitively via `v1-0-ship-gate`, job `:3414`); `cargo test --workspace --no-fail-fast`
  (AC6 + AC7); and `onb-nfr2-timing` (review rebaseline, `aggregate.needs`). **The J1 fix is
  D-2**: `derive_enrolled_targets` excludes an underscore-delimited decimal segment immediately
  before a J1 suffix, while numeric J1 stems remain governed. Validated against all 9 original
  matches, the existing `xyz_1b.rs` red vector, the `foo_14_2a.rs` exclusion, and a new
  `j1_2a.rs` red vector.
  The advisory audit is recorded as a table (21 blocking / 14 advisory / 3
  blocking-when-present at `CURRENT_PHASE = "v1_5"`), with `check-cross-form-equiv`'s
  never-scheduled lift filed as the single exception.
  ⚠ Falsifier: re-running the gate after reverting only the filter change must red it again.
  ⚠⚠ **SCOPE HONESTY — THE LARGEST UNMEASURED THING IN THIS STORY, AND IT IS AC1's OWN.** This
  preflight measured the seven controls, the full workspace suite, and ~41 hermetic `xtask` verbs.
  It did **NOT** measure the aggregate job-by-job. Of its **123** `needs:`, **66 build or test and
  are NOT `cargo run -p xtask --` gate verbs** (benches, `MAOS_ONE_SHOT` smoke jobs, corpus jobs,
  `reproducible-build`, the per-Spirit suites, the NFR jobs). AC6 makes `cargo test --workspace`
  green, which covers the large `-p <crate> --test <name>` share of them — but **not** the benches,
  the one-shot smoke arms, or the corpus/build jobs. **Two unlisted unpassable jobs were found:
  `a2a-tcp-tests-8-6` and `onb-nfr2-timing`.** Both now close in this story: AC6 removes the
  a2a flake, and the 2026-09-08 review rebaselines the measured 26,473,608-byte binary under a
  28MiB limit. T0 measures the aggregate's non-gate jobs before code is written so any further
  red is scoped and resolved before closure. **AC1 must not claim a green aggregate on the
  strength of its named gates alone.**

- **AC2 (I9 / D-H).** `check-empty-kernel` exits 0 with `PASSED (0 violations)`. The 3-line
  `#[maos_attrs::i9_exempt(...)]` at `security/mod.rs:120-122` — misattached by `af788c3e`, its
  `reason` still describing `SecurityManagerAdapter` — is **moved back** onto
  `SecurityManagerAdapter` (`:130`, already registered at `i9-exemptions.md:91`), net-0;
  `T3ImageVerificationConfig` (`:124`) then carries no attribute and needs no entry (its fields
  are not denylisted — verify by running the gate, not by reading). `ScbRuntimeSnapshot`
  (`scheduler/control_block.rs:244`) and `VerifiedImageLock`
  (`security/sandbox/t3/image_lock.rs:27`) each gain a **single-line**
  `#[maos_attrs::i9_exempt(reason = "…")]` with **reason ≤ 63 characters** (line ≤ 100;
  `check-fmt` is Blocking); `ValidatedRevocationRules` (`revocation/rules.rs:19`) gains a
  register entry only. Three entries appended to `docs/invariants/i9-exemptions.md` in its
  `### \`Name\` — \`path\`` + `**Reason:**` format (§Entries exemplar `:13-17`, **not** `:12-16`).
  **+2 kernel physical lines: `kernel-core-baseline.toml` 24472 → 24474 AND the
  `maos-kernel-core` kloc row 18933 → 18935, in the SAME commit** (D-F standing permission;
  the ledger's `"15-1 +2"`). The whitelist path is NOT taken —
  `whitelist_has_exactly_three_paths` hard-fails on a fourth entry.
  ⚠ **Residual control (§10):** an integration fixture with an exempt-but-undocumented struct,
  proven red, in uncharged `xtask/tests/`.

- **AC3 (NFR-Test-2).** `check-service-boundary` reports **0** NFR-Test-2 rows (the 4 `P3` rows
  are AC2's and clear with it — **land AC2 and AC3 in one commit**). Per D-4: the baseline
  `docs/ci-baselines/kernel-surface-v0.1-beta.json` is **hand-edited surgically** — 18
  in-place `signature_hash` refreshes plus the deletion of the single genuinely-removed symbol
  `security::sandbox::t3::image_lock::get_default_image`, taking it 370 → **369** items — with
  the symmetric-diff proof in the commit message (`1dda03eb` precedent). **Only the 8
  genuinely-new paths** are classified in `xtask/kernel-api-classes.toml`, each with a
  one-line justification grounded in what the symbol does. The 19/17 and 9/8 figures in the
  epic are corrected to **19 rows / 17 unique / 18 signature-changed / 1 genuinely removed**.
  `--write-baseline` is **not** added (D-4).
  ⚠ **R-6 — the symmetric diff is AC text, not a task.** The 18/1 split is the largest single
  correction in this story and is the one big number the author did **not** personally re-derive
  (it is scout-measured via `git log -S`, the diff, and a symbol-presence check). It becomes a
  control only when the dev re-runs it and states the result in the commit message: **paths
  added 0 · paths removed 1 · hashes changed 18 · items 370 → 369.** If those four numbers do
  not reproduce, the AC is not met and the story is amended before proceeding.
  ⚠ **Residual control (§10):** one proven-red test in uncharged `xtask/tests/` asserting that a
  symbol present in the shipped baseline but absent from the current surface reds the gate —
  closing the `--baseline /dev/null` hole that lets 9 of 10 integration tests pass a broken diff.

- **AC4 (kloc).** `cargo run -p xtask -- kloc-check` exits **0**. Four arms, none dischargeable
  by moving a number:
  **(a)** the live breach `xtask 43411 > 43095` is closed per **D-1 as re-ruled (§12, R-1)**:
  the reclaim is attempted and **its failure is recorded as evidence** — 14-0 already spent that
  pool, three of the 16 surviving warnings are the exact functions 14-0 named as un-deletable,
  and the realistic yield is ≈15–25 lines against a 316 breach. Then, and only then, **one named
  measured grant at the landing commit**: `xtask = final measured + 35`, pricing shape restored,
  operator authorization recorded in the row comment, mirroring the `maos-a2a-core` 14-2a row
  wording included.
  ⚠ **R-11 — the number is a FORMULA, never a literal, and the landing commit is what freezes
  it.** The literal was wrong twice in twenty-four hours: 226 over at `bce5ac68`, **316** over at
  `2fd492c0` after 15-3 amended a `done` commit with 90 more charged lines and did not touch
  `kloc.toml`. Do not copy any number from this file into `kloc.toml`. Compute it at the commit
  that lands. Drift after that reds `kloc-check` and belongs to whoever wrote it — which is the
  rule that was supposed to exist all along (§13).
  ⚠ **R-12 — AUTHORIZED.** The grant is ratified under Lunarpulse's *"resolve all before we
  develop the story"* plus the standing directive *"we pay it early as planned — deferring
  drifts."* Recorded as a ruling, not an assumption, so it can be corrected rather than
  discovered.
  **(b)** 15-3's booking is **corrected in place, filed not overwritten** (D13(a) wording): the
  `xtask` row's *"EXACT MEASURED … ZERO HEADROOM"* and *"check_exit_commands.rs +1143"*
  (measured **1344** at `bce5ac68`, **1434** at HEAD; unbooked **+201 / +23 / +2 = 226**, plus
  **+90** added by the `2fd492c0` amend = **316**), and `sprint-status.yaml:235`'s
  *"kloc-check PASS"*, *"17109→17309"* (**17339**) and *"Aggregate 156840"* (**157186**).
  Comments are not tokei `code`, so (b) is free.
  **(c)** the falsifier: adding enough charged code to exceed the authorized 35-line reserve
  must red `kloc-check`; the executed +43-line probe proves both sides of that boundary.
  **(d)** what 15-2 already discharged is recorded as **measured-verified inheritance, not
  claimed**: `maos-domain` 8695/**9192** ✅, `maos-bin` 17339/**20109** ✅, aggregate
  157186/170884 ✅ (alarm 158608), **D14 and D17 CLOSED against `15-2-kloc-ceiling-rebase`**,
  `check-decision-register` exit 0 at 23 rows / 12 open.
  ⚠ AC2's `maos-kernel-core` 18933 → 18935 lands here too, or `kloc-check` reds on AC2's `+2`.

- **AC5 (env contract).** `check-env-contract` exits 0. `MAOS_OPERATOR_BEARER_TOKEN` and
  `MAOS_OPERATOR_HTTP_BIND` (read at **`main.rs:2703-2704`** — the epic's `:2669-2670` has
  drifted 34 lines) are registered in `crates/maos-bin/src/env_contract.rs` (registry opens
  `:13`, row format `:14-18`), both **`EnvStability::UserFacing`** — the surface is live
  (`main.rs:2702-2725` validates, `:2824-2841` binds a real listener), and every secret-shaped
  var in the registry is already `UserFacing` while all 11 `HarnessOnly` rows are test knobs.
  Measured cost +10…+14 `maos-bin` lines against 2770 headroom. The gate's *"Story 12.7"*
  message is repointed on **both** its PASS and FAIL paths (12.7, 14-7, 14-8 and 14-9 are all
  dead keys) to the surviving home, D-I / 21-4 / `xtask/env-contract.toml`. **AC5 registers the
  two reads the gate can see** — `crates/maos-cli/src/subcommands.rs:1938,:1940` hold a third
  variable and a second read of the first, both permanently out of the gate's
  `crates/maos-bin/src` scope (`check_env_contract.rs:119`); this is stated, not fixed.
  D-I: the registry stays where it is in W0.

- **AC6 (the four workspace test failures).** `cargo test --workspace --no-fail-fast` exits
  **0**. Four failures, three of them races; **each fix ships a repeat-count oracle and a
  measured before/after failure rate — a single green run does not discharge any of them**:
  **(a)** `t_14_2a_a_promoted_generation_…` — root cause is the process-global `tracing_core`
  callsite-interest cache, proven by isolation (C+A **21/30**, C+B **0/30**, C alone **0/30**);
  only `:267` installs a subscriber while the co-resident test at `:68` traverses the same
  `warn!` callsite with none. Pre-fix rate at 8 threads **24/30**; post-fix **0/40 required at
  default threads and 0/50 at `--test-threads=8`**. **The fix shape is ruled by D-7: process
  isolation by default (its own test binary, defect removed by construction); the 3-line
  `register_callsite` alternative is permitted only if it MEASURES to the same two numbers.**
  **(b)** `drr_backpressure_emitted_when_backlog_exceeds_threshold`
  (`maos-kernel-core/tests/drr_scheduler.rs:140`) — a fixed `sleep(50ms)` used as a
  synchronisation primitive; replaced by a bounded wait on the condition. Pre 3/20, post 0/40.
  **(c)** `stdio_transport_malformed_output_returns_transport_error`
  (`maos-mcp/tests/stdio_transport_test.rs:45`) — **EPIPE, not parse**: the fixture never reads
  stdin, so `spawn_and_invoke`'s write loses the race and yields `"write: Broken pipe"`
  (mechanism reproduced 300/300). The **fixture** is fixed (`sh -c "cat >/dev/null; echo …"`),
  not the transport; the transport's fatal-EPIPE behaviour is **filed, not fixed**. Pre 7/20,
  post 0/40.
  **(d)** `kloc_check::tests::kloc_check_runs_on_workspace` clears with AC4 and stays
  un-`#[ignore]`d.
  **Per D-6, `--test-threads=1` comes out of `check_cert_rotation_trigger.rs:613` only AFTER
  (a) is proven, in the same commit, with the gate re-run to show it stayed green.**
  ⚠ **This AC also closes the sixth red the epic does not list:** `a2a-tcp-tests-8-6`
  (`discipline.yml:1642`, `aggregate.needs:3623`, no `continue-on-error`) runs the suite 50× at
  `--test-threads=8` and fails on any flake — unpassable at HEAD. Its 50× loop is the
  acceptance run for (a).

- **AC7 (the suite becomes a control) — NEW, per D-3.** `cargo test --workspace --no-fail-fast`
  is enrolled as a Blocking CI job in `aggregate.needs`, because at HEAD **no job runs it**:
  every `cargo test` in `discipline.yml` is `-p`/`--test`-scoped and both `cargo nextest run`
  invocations in `journey-nightly.yml` are `-p maos-journey-test`-scoped. Enrolled at all the
  sites 15-3 established for a new Blocking gate (discipline job + the correct `needs:` list +
  `gate-registry.toml` + `check_ship_gate_completeness.rs` `EXPECTED_GATES`), with
  `check-ship-gate-completeness` re-run green and its count moved deliberately. The job carries
  an explicit `timeout-minutes` and its wall-clock cost is measured and recorded in the story.
  ⚠ Falsifier: planting a failing test anywhere in the workspace must red the aggregate.
  ⚠ **Framing corrected at the round-table (§12, R-3): this is a GAP, not a regression.** CI
  never ran the suite; the exit block was written assuming a cargo built-in "belonged" beside
  five verbs that do exist. Nothing was demoted — the clause was never built.
  ⚠ **R-4 (downgraded on measurement — the escalation did not survive its own check).** Making
  the suite Blocking makes `#[ignore]` the escape hatch. Audited: **173 sites, 171 carry a
  reason.** The two bare ones were escalated in the room and **both measured benign**:
  `maos-siem/tests/fault_inject.rs:67` is gate-driven (`check_enterprise_identity.rs:273`, the
  whole file is `#![cfg(feature = "siem-fault-inject")]`) and **PASSES** under
  `cargo test -p maos-siem --features siem-fault-inject -- --ignored`;
  `maos-kernel-core/tests/cgroup_ceiling_smoke.rs:15` carries an internal `MAOS_CGROUP_TEST`
  guard and self-skips regardless. **So: add the two reason strings (2 lines, in `crates/`, free)
  and FILE the enforcing gate to 20-3** — building a new xtask gate, funded by the grant of
  AC4(a), to police a hatch with zero live defects is the gold-plating this room exists to stop.
  The rule is right; the urgency was not.
  ⚠ **R-13 — AUTHORIZED.** This adds one full workspace test run per CI cycle (measured locally:
  497 test binaries, 4168 tests). Ratified under Lunarpulse's *"resolve all before we develop the
  story"*; no new argument against it was offered at the second round-table, Dana's cost objection
  having died on the gap-not-regression reframing. Recorded as a ruling so it can be corrected
  rather than discovered. **If it is ever reversed, AC1's final clause must be re-worded to say
  plainly that it is a local acceptance step no machine checks, and the epic amended to match; it
  must not stay worded as though CI enforces it.**

---

## Declared cut lines

| Not in this story | Why | Owner |
|---|---|---|
| `spawn_and_invoke`'s fatal write-side EPIPE | A real robustness question for one-shot NDJSON servers; the fixture race is what makes the suite red | filed → 21-4 |
| `pub use` group-hash defect (5 removed rows from 1 identifier) | Latent `check_service_boundary` defect, not a HEAD red | filed → 20-3 |
| `_2c.rs` J1 blind spot | The derivation cannot see the 2c lane; unrelated to the false positive | filed → 21-4 |
| `check-cross-form-equiv`'s never-scheduled advisory lift | The only demotion with no end date | filed → 20-3 (D-G) |
| Re-denominating the kloc instrument to exclude inline tests (5653 lines) | True observation; an epic-retrospective decision, not a green-at-HEAD story's | filed → epic-15 retro |
| **A `done` story can add charged lines and no gate reads the ceiling** (R-10) | `check-epic-close-coherence` never opens `kloc.toml`; 3 occurrences, same crate, same story. Building the control that would have prevented this story's own debt is the scope creep this story exists to refuse | **filed → 20-3** |
| `check_epic_6_bridge.rs` retirement (4025 lines) | Load-bearing funding for 20-3 AC7 | 20-3 |
| `EnvStability::Secret`, workspace-wide env scope | Registry becomes a data file at 21-4 (D-I) | 21-4 |

---

## Dev notes

**T0 IS NOT OPTIONAL AND IT IS NOT A FORMALITY.** Two independent reasons:
1. A concurrent agent was writing this repo during the preflight (see `baseline_commit`) and
   left `cargo build -p xtask` at EXIT 101 in the working tree. **Confirm `git status
   --porcelain` is empty and `cargo build -p xtask` exits 0 before writing a line.** If the tree
   is dirty, cite with `git show HEAD:<path>` (the 14-2d rule).
2. **This story exists because 15-3 measured mid-story and never re-measured.** Repeating that
   is the one failure this story cannot afford. Every kloc figure is re-derived at the landing
   commit, not taken from this file.

**Instrument discipline — do not conflate the two.** The PIN counts **raw physical lines**
(`content.lines().count()`, `check_kernel_baseline.rs:99-112`) over every `.rs` under
`maos-kernel-core/src`, blanks and comments included → 24472. The kloc CEILING counts **tokei
`code`** with `target tests benches examples fuzz spirits` excluded → 18933. They differ by
5539 and the architecture is explicit that they are *"each cited only in its own units, never
compared"*. AC2 moves **both**, by +2 each, in one commit.

**Tests are free; production lines are not.** `-e tests` is a bare depth-agnostic directory
pattern: `xtask/src/tests/` (2904 code, 16 files) and `xtask/tests/` (8652, 68 files) are both
uncharged, and 99 + 16 + 68 = 183 = the crate's full Rust file count. **Every proven-red vector
and residual control this story owes goes in one of those two directories.** A test written
in-`src` is charged *and*, per `crates/maos-bin/src/topology.rs`'s module doc, never executed by
CI.

**No CI job runs `cargo test -p xtask` or `cargo test -p maos-bin` unscoped**
(`discipline.yml:424`, `:1966`, `:1902-1903` say so in their own comments). **26 of the 38 files
in `crates/maos-bin/tests/` execute nowhere.** A control added by this story is not a control
until it is enrolled **by name** — that is what AC7 is for, and it is why AC7 is not optional
polish.

**Citation hygiene — this section is the reason.** Four scouts found **eleven** drifted or false
citations in a section written eight days ago: `aggregate` `:3510`→`:3540`; the J1 job
`:1904`→`:1939`; *"`aggregate.needs:3410`"*→**wrong list entirely**; env vars
`:2669-2670`→`:2703-2704`; the verb dispatch `:577-578`→`:588`; `i9-exemptions.md` §Entries
`:12-16`→`:13-17`; `check-fmt` `:136-139`→job at `:140`; `check_empty_kernel.rs:243-244`→the
path-scoping is at `:136-141`; `main.rs:204-208`→`:203-222`. **Cite SYMBOLS, quote text, and
re-run every number you print.** `xtask/kloc.toml` is quote-only by house rule since 15-2 F2.

**Two independent phase axes, do not confuse them** (`gate_common.rs`). `CURRENT_PHASE = "v1_5"`
(`:166`) is the **GA ship ladder** and feeds `phase_disposition`/`is_blocking_at` over
`gate-registry.toml`. `enum BindingClass { Blocking, AdvisorySubstrate }` (`:210`) is
**dev-time** enforcement, decoupled from phase (`dev_enforced_red_blocks`, `:229`). Several
gates are GA-advisory at v1_5 yet dev-Blocking in code — that is Epic-12-retro B1 Option C
working as designed, not a demotion.

**Prior-story intelligence that applies directly here.**
- *A grant is a global, not a reservation* — re-measure before citing any prior authorization.
- *A correct measurement on the wrong axis* — check which way the arrow points before retiring
  a mechanism (§3's option (B) is exactly this shape: the gate's own message asks for the wrong
  fix).
- *"a claim standing in for a control"* — the Epic-12 retro's named signature failure. It
  appears **four** times in this one section: AC4 (§7), AC1's suite clause (§5), AC3's
  classification rows under a full re-emit (§6), and the epic's *"stays enrolled where it is"*
  (§3).
- *Mechanical gates compound; promises decay* — AC7 and the three residual controls are the
  mechanical half. Ship them in this commit or they decay.

---

## Tasks / Subtasks

- [x] **T0 — Re-measure at HEAD before writing a line.** (AC: 1,4)
  - [x] `git status --porcelain` empty; `git rev-parse --short HEAD` recorded in the story
  - [x] `cargo build -p xtask` exits 0; `cargo fmt --all --check` clean
  - [x] `tokei --version` == the CI pin (`discipline.yml:11`, `TOKEI_VERSION: "14.0.0"`)
  - [x] Run and record: the five gate exits, `kloc-check --json`, `cargo test --workspace --no-fail-fast` totals
  - [x] **Measure the aggregate's 66 non-gate jobs** (the ones that build/test and are not `cargo run -p xtask --`). Reds this story does not own are FILED with a named owner and AC1 is narrowed in writing — not discovered at T28
  - [x] If any figure differs from this file, **the story is amended before dev proceeds** (rule 9)
- [x] **T1 — AC4(a): attempt the reclaim, and record that it cannot fund.** Delete what is genuinely dead (unused struct fields, dead assignments, the unused `CURRENT_PHASE` import), each evidenced by a clean `cargo build -p xtask`. **Do NOT delete `evidence_ledger::detail`, `check_fkcs::synthetic`, `check_fkcs::read_nonempty_lines`** — 14-0 named all three as live callees of `xtask/tests/` targets, and the warning comes from the BIN compilation. Do **not** delete `check_epic_6_bridge.rs` (20-3's funding). Record the measured yield (expected ≈15–25) against the 316 breach, in the `maos-a2a-core` 14-2a wording. (AC: 4)
- [x] **T2 — AC2: the I9 attribute move.** `security/mod.rs` — move the 3-line attribute (and, free and correct, the misattached doc comment) from `T3ImageVerificationConfig` back onto `SecurityManagerAdapter`. Net 0 lines. (AC: 2)
- [x] **T3 — AC2: two single-line attributes.** `ScbRuntimeSnapshot`, `VerifiedImageLock`. **reason ≤ 63 chars each**; verify with `cargo fmt --all --check` immediately. +2 physical. (AC: 2)
- [x] **T4 — AC2: three register entries** in `docs/invariants/i9-exemptions.md` (`### \`Name\` — \`path\`` + `**Reason:**`). Write real rationales per `I9.md:34`, not gate-satisfying strings. (AC: 2)
- [x] **T5 — AC2: both instruments, same commit.** `kernel-core-baseline.toml` 24472 → 24474 with a HISTORY row; `kloc.toml` `maos-kernel-core` 18933 → **18935** with the D-F/`"15-1 +2"` citation. Run `check-kernel-baseline` and `check-empty-kernel`. (AC: 2,4)
- [x] **T6 — AC2 residual control.** Integration fixture with an exempt-but-undocumented struct, **proven red**, in `xtask/tests/` (uncharged). Verify it reds by reverting T4 alone. (AC: 2)
- [x] **T7 — AC3: surgical baseline edit.** 18 `signature_hash` refreshes + delete the `get_default_image` object; 370 → 369 items. Attach the symmetric diff (paths added: 0; paths removed: 1; hashes changed: 18) to the commit message, `1dda03eb` form. (AC: 3)
- [x] **T8 — AC3: classify the 8 genuinely-new paths** in `kernel-api-classes.toml` under a `# Story 15-1 AC3` banner, one justification line each. **Do not classify the signature-changed symbols** — they are unreachable after T7. (AC: 3)
- [x] **T9 — AC3: ADR-066** recording (a) the sanctioned removal of `get_default_image` and its security rationale, (b) the **three breaking** signature changes (`spawn_t3`, `build_runtime_argv`, `load_and_verify_lock` — all now returning/consuming verified types), which the epic omits entirely, (c) that `af788c3e` moved the surface without its paired instrument, mirroring `kloc.toml:199` against the same commit, and (d) why the note is not in the JSON. Update `docs/adr/index.md`. (AC: 3)
- [x] **T10 — AC3 residual control.** Proven-red test in `xtask/tests/`: a symbol in the shipped baseline but absent from the current surface must red. Closes the `--baseline /dev/null` hole. (AC: 3)
- [x] **T11 — AC1/D-2: the J1 filter.** Add the digit-exclusion to `derive_enrolled_targets`' filter; update the `J1_TEST_SUFFIXES` doc comment to record *why* (the 14-2a file is executed by `check_cert_rotation_trigger`'s own `invoke_cargo_test`, so enrolling here would double-run it). (AC: 1)
- [x] **T12 — AC1/D-2: pin the exclusion.** Add an `assert_green` twin beside `planting_an_unenrolled_j1_test_file_must_red` (`xtask/tests/j1_crosshost_1b_proven_red.rs:661`): plant `crates/maos-bin/tests/foo_14_2a.rs` un-enrolled, assert the gate stays GREEN. **Re-run the existing `xyz_1b.rs` vector and confirm it still REDS.** (AC: 1)
- [x] **T13 — AC5: two `EnvVar` rows**, both `UserFacing`, in `env_contract.rs`. Run `check-env-contract`. (AC: 5)
- [x] **T14 — AC5: repoint the dead-key message** on both the PASS and FAIL paths of `check_env_contract.rs`. (AC: 5)
- [x] **T15 — AC6(a): fix the tracing race, per D-7.** Record the pre-fix rate first (≥30 runs at 8 threads). Then apply **process isolation** (own test binary) unless the 3-line `register_callsite` alternative is measured to the same bar. Then prove: **0/40 at default threads and 0/50 at `--test-threads=8`.** (AC: 6)
- [x] **T16 — AC6(b): `drr_scheduler`.** Replace the fixed `sleep(50ms)` + `try_recv()` drain with a bounded wait on the condition. Pre-rate recorded, post-rate 0/40. (AC: 6)
- [x] **T17 — AC6(c): the stdio fixture.** Make the child read stdin before echoing. Pre-rate recorded, post-rate 0/40. **File** the transport's fatal-EPIPE behaviour; do not change `stdio.rs`. (AC: 6)
- [x] **T18 — AC6/D-6: drop `--test-threads=1`** from `check_cert_rotation_trigger.rs:613` **only after T15 is proven**, in the same commit; re-run `check-cert-rotation-trigger` and confirm it stayed green. (AC: 6)
- [x] **T19 — AC6: prove the sixth red closes.** Run `a2a-tcp-tests-8-6`'s exact loop locally: 50× `cargo test -p maos-a2a-tcp --locked -- --test-threads=8`, zero flakes. (AC: 6)
- [x] **T20 — AC7: enroll the workspace suite.** New Blocking job with an explicit `timeout-minutes`, in `aggregate.needs`; `gate-registry.toml` row; `EXPECTED_GATES` in `check_ship_gate_completeness.rs`; re-run `check-ship-gate-completeness` green with its count moved deliberately. Record the measured wall-clock. (AC: 7)
- [x] **T21 — AC7 falsifier.** Plant a failing test, confirm the new job reds the aggregate, revert. (AC: 7)
- [x] **T22 — AC4(a): measure, then take ONE grant.** After T1–T21, re-run `tokei` at the landing commit; set `xtask = final measured + 35` with the pricing-shape rationale, the operator authorization, and **R-2's attribution** (the 226 itemised as 15-3's +201/+23/+2; 15-1's own spend on its own line) in the row comment. One grant, one commit — never a second grant on this row. (AC: 4)
- [x] **T23 — AC4(b): file the corrections.** `kloc.toml`'s `xtask` row comment and `sprint-status.yaml:235`, *filed not overwritten*. (AC: 4)
- [x] **T24 — AC4(c): the falsifier.** Add more than the authorized 35 charged lines; confirm `kloc-check` reds; revert. (AC: 4)
- [x] **T25 — AC1: the advisory audit table**, recorded in the story: 38 rows, effective disposition at `v1_5`, with `check-cross-form-equiv` flagged. (AC: 1)
- [x] **T26 — §11: VERIFY the epic amendment** (rule 9). **R-14: §11 was APPLIED at spec time (2026-09-08), against `2fd492c0`, after confirming the epic's 15-1 section was byte-identical across the `bce5ac68 → 2fd492c0` amend.** This task is now a *verification*: confirm the fourteen edits are present and still true at the landing commit. **R-7 survives for anything that has drifted since: re-read `epic-15-foundations-w0.md` and RE-DERIVE every OLD anchor before re-applying.** The NEW text is settled; the OLD text was captured at `2fd492c0` while another agent was editing that file, so a verbatim apply may not match. If an OLD anchor has moved, the epic's current wording wins as the anchor and the NEW text is re-fitted to it. (AC: all)
- [x] **T27 — AC7/R-4: two reason strings.** `maos-siem/tests/fault_inject.rs:67` → `#[ignore = "requires --features siem-fault-inject; gate-driven via check-enterprise-identity"]`; `maos-kernel-core/tests/cgroup_ceiling_smoke.rs:15` → a reason naming its `MAOS_CGROUP_TEST` guard. File the enforcing gate to 20-3. 2 lines, both in `crates/`, uncharged to `xtask`. (AC: 7)

---
- [x] **T28 — Full sweep.** `cargo fmt --all --check`; the five gates; `check-kernel-baseline`; `check-decision-register`; `check-epic-close-coherence`; `check-ship-gate-completeness`; `check-exit-commands`; `cargo test --workspace --no-fail-fast`; `git diff --stat crates/maos-kernel-core/src` shows **exactly two added lines**. (AC: all)

### Review Findings

- [x] [Review][Patch] HIGH — Rebaseline `onb-nfr2-timing` in 15-1 so the aggregate can become green — Lunarpulse selected an in-story measured rebaseline on 2026-09-08. Preserve AC1's aggregate-green contract; update the size gate and its rationale from the measured 26,473,608-byte binary, remove the stale 15-4 filing, and verify the Blocking job's script exits 0.
- [x] [Review][Patch] HIGH — Install the ABI test toolchain in the new Blocking workspace-suite job [.github/workflows/discipline.yml:3423]
- [x] [Review][Patch] HIGH — Drain stdin in the successful stdio fixture to remove the same EPIPE race [crates/maos-mcp/tests/stdio_transport_test.rs:6]
- [x] [Review][Patch] MEDIUM — Preserve typed validation of the optional red-team `notes` field [xtask/src/check_red_team_gate.rs:52]
- [x] [Review][Patch] MEDIUM — Exclude actual epic-number segments without dropping numeric J1 stems [xtask/src/check_j1_loopback_delegation.rs:803]
- [x] [Review][Patch] MEDIUM — Align AC4's falsifier with the authorized 35-line headroom [_bmad-output/implementation-artifacts/15-1-green-at-head.md:633]
- [x] [Review][Patch] MEDIUM — Keep the 15-3 grant history distinct from the 15-1 pricing grant [xtask/kloc.toml:251]

## 11. Epic OLD→NEW edits (rule 9 — land in this commit)

✅ **APPLIED 2026-09-08 against `2fd492c0`** (R-14). All eighteen landed in
`_bmad-output/planning-artifacts/epics/epic-15-foundations-w0.md`; the three gates that read epic
files were re-run green afterwards. Retained below as the record of what changed and why — and as
the checklist T26 verifies at the landing commit.

1. **Header, "Closes:"** — OLD *"5 red gates + 1 test red … `t_14_2a_post_grace_journal`"* → NEW: six red controls (`kloc-check`'s breach is now `xtask`), **four** workspace test failures, and the Blocking job `a2a-tcp-tests-8-6`, which is unpassable at HEAD.
2. **Header, exit-block provenance line 1** — OLD *"`kloc-check` … red: aggregate 155498 > `_aggregate_hardfail` 147057 and `maos-domain` 8695/8644 (15-2)"* → NEW *"`kloc-check` red: **`xtask` 43411 > 43095** (15-1 AC4); aggregate 157186/170884 ✅ and `maos-domain` 8695/9192 ✅, both closed by 15-2"*.
3. **Header, exit-block provenance line 1** — add: *"`cargo test --workspace --no-fail-fast` — EXIT 101 at `2fd492c0`: 4168 passed / **4 failed** / 118 ignored. **No CI job runs this command**; 15-1 AC7 enrolls it."*
4. **15-1 AC1** — `discipline.yml:3510` → **`:3540`**; job `:1904` → **`:1939`**.
5. **15-1 AC1** — OLD *"in `aggregate.needs` at `:3410`"* → NEW *"in **`v1-0-ship-gate.needs` at `:3439`** (job `:3414`); the aggregate depends on it transitively"*. (Same error the epic header already records against 15-3.)
6. **15-1 AC1** — OLD *"narrowed to J1 stems (e.g. `j1_*_{1a,1b,2a,2b}.rs` or an explicit stem list)"* → NEW *"the suffix match additionally requires that the preceding character is not an ASCII digit"*, plus the reason both epic forms are rejected: `j1_*` derives **0** targets (vacuity red) and both break the gate's own planted `xyz_1b.rs` vector, which the same sentence promises to keep.
7. **15-1 AC1** — OLD *"the test itself stays enrolled where it is (`check-cert-rotation-trigger`)"* → NEW *"…is **executed by `check-cert-rotation-trigger`'s own `invoke_cargo_test` (`:595-613`)**, not by any `--test` line in `discipline.yml` — zero such tokens exist in all 11 workflow files"*.
8. **15-1 AC2** — OLD *"6 violations (five structs + two undocumented-register rows)"* (= 7) → NEW *"**4 struct-field rows** (`ScbRuntimeSnapshot`; `SecurityManagerAdapter` ×2 fields; `VerifiedImageLock`) **+ 2 undocumented `#[i9_exempt]`** (`revocation/rules.rs:19`, `security/mod.rs:124`)"*.
9. **15-1 AC2** — add the missing hard constraint: *"the reason string must be **≤ 63 characters** (line ≤ 100; no `rustfmt.toml`; overhead 37) — at 64 it reflows to three lines, the delta becomes +6 and `check-fmt` reds"*. Correct §Entries `:12-16` → **`:13-17`**; `check-fmt` `:136-139` → job `check-fmt` at `:140`; `check_empty_kernel.rs:243-244` → the path-scoping is at `:136-141`. Add the stronger whitelist reason: `whitelist_has_exactly_three_paths` hard-fails on a fourth entry.
10. **15-1 AC3** — OLD *"signature-changed (~9 …) and genuinely removed (`SpawnError`, `UpgradeError`, `UpgradeOrchestrator`, `UpgradeOutcome`, `UpgradeReport`, `UpgradePolicy`, `RevocationApplier`, `get_default_image`)"* → NEW *"**18 signature-changed and exactly ONE genuinely removed (`get_default_image`)**. Seven of the eight formerly listed as removed are alive and already classified; the absence of an `other` row was read as absence of the symbol."*
11. **15-1 AC3** — OLD *"the baseline JSON is re-emitted for both … the story adds `--write-baseline` to xtask (charged to the xtask row of 15-2) or hand-edits"* → NEW *"the baseline is **hand-edited surgically** (18 hash refreshes + 1 deletion, 370→369) and **only the 8 genuinely-new paths are classified**. A full re-emit is a proven one-button green (0 rows with the classes file untouched) that makes the classification rows unreachable. `--write-baseline` is **not** added: +15 charged xtask lines onto a ceiling already 226 over, and the named funding source — 'the xtask row of 15-2' — **is a 15-3 grant, dated 2026-09-07, declared ZERO HEADROOM**."*
12. **15-1 AC3** — OLD *"under an invariant-lock review note [in the baseline JSON]"* → NEW *"recorded in **ADR-066**; `invariant-lock` maps `I1..I14 → docs/invariants/I*.md` only and never fires on either file — the 'invariant-lock review' line in both is a prose convention (ADR-065 §87-92, ADR-043 precedent). A key added to the JSON is destroyed by the next re-emit."*
13. **15-1 AC4** — replaced wholesale by the four arms of this story's AC4: the live red is **`xtask` 43411 > 43095**; `maos-domain`, `maos-bin`, D14 and D17 are **discharged by 15-2** and are recorded as measured-verified inheritance, not claimed.
14. **15-1 AC5** — `main.rs:2669-2670` → **`:2703-2704`**; `maos-bin (17109/17109)` → **(17339/20109)**. Add the scope-honesty clause (`maos-cli/src/subcommands.rs:1938,:1940` hold a third var and a second read, permanently out of the gate's scope) and the dead-key repoint (12.7 / 14-7 / 14-8 / 14-9 all absent).
15. **15-1 AC6** — grow from one test to four, with per-test measured flake rates; add **D-6's ordering constraint** (`--test-threads=1` comes out only after the fix is proven, or a green Blocking gate goes red); add `a2a-tcp-tests-8-6` as the sixth red and its 50× loop as the acceptance run.
16. **15-1 — NEW AC7** (D-3): enroll `cargo test --workspace --no-fail-fast` as a Blocking CI job, because no job runs it today. Flagged to the operator as a cost decision.
17. **15-1 Closes·Δ row in the Stories table** — add *"7 ACs; refined 2026-09-07 by a mid-epic refinement round (rule 9) — this section was amended to match"*, matching the 15-2 and 15-3 rows.
18. **Header** — add a third refinement-round paragraph in the established form, naming the four scouts' largest disproofs and pointing at `implementation-artifacts/15-1-green-at-head.md`.

---

## 12. Round-table 2026-09-07 — eight rulings folded in

Convened on the story as written. Winston · Amelia · Murat · Mary · John · Paige · Sally ·
Grumbal · Vex · Boundary · Yui · Dana · Level · Splinter · Killjoy · Wildcard.
**The room opened by convicting this story's own D-1 ruling, and closed by correcting one of
its own escalations on measurement.**

| # | Ruling | Applied at |
|---|---|---|
| **R-1** | **D-1's reclaim step re-ruled: evidence, not funding.** 14-0 already spent the pool; three of the 16 surviving `xtask/src` warnings are the exact functions 14-0 named as un-deletable. Yield ≈15–25 vs a 316 breach → attempt, record the failure, take ONE named measured grant. `maos-a2a-core` 14-2a is the precedent, wording included | D-1 · AC4(a) · T1 · T22 · `kloc_grant` |
| **R-2** | Grant attribution: the breach booked as **15-3's, itemised +201/+23/+2 at `bce5ac68` plus +90 from the `2fd492c0` amend = 316**, *filed not overwritten*; 15-1's own spend on its own line | D-1 · AC4(b) · T22 |
| **R-3** | AC7 **enrolled**, and re-framed — a **gap, not a regression**. The clause was never built | AC7 |
| **R-4** | **Downgraded on measurement.** The `#[ignore]` hatch audit: 173 sites, 171 with reasons, and **both bare ones measured benign**. Add the two reason strings (free, in `crates/`); **file the enforcing gate to 20-3** rather than fund a new xtask gate for a hatch with zero live defects | AC7 · **new T28** |
| **R-5** | The EPIPE item is filed **with its 300/300 reproduction inside it**, not summarised | §4d |
| **R-6** | AC3's 18/1 split: the **symmetric diff is promoted from task to AC text** — 0 added / 1 removed / 18 changed / 370→369, or the AC is not met | AC3 |
| **R-7** | §11 is applied by **re-deriving the OLD anchors at the landing commit**; the epic file is being edited concurrently. NEW text settled, OLD text not | T26 |
| **R-8** | **WHOLE**, on D-6 coupling — not on budget, which does *not* forbid this split. Dana's dissent recorded | `split_from` |

**Beats worth keeping.** Level opened by asking who had actually looked at the dead-code pool,
and the answer indicted the story's own funding plan: *"those are the three 14-0 wrote down by
name."* — Grumbal. Sally's naive question (her tenth) dissolved the AC7 fight by re-framing it:
*"what does the aggregate actually promise right now?"* — it never promised the suite, so this
is a gap nobody built, not a control anyone demoted; Dana conceded it was a better argument than
Murat's. Vex escalated the bare `#[ignore]` on a SIEM leak-inversion test, the room agreed with
him, **and the measurement disagreed** — it is gate-driven and passes — which is why R-4 ships as
two reason strings instead of a gate. John refused to reuse 15-3's budget argument for the split
and won on D-6 instead. And Mary's, which nobody enjoyed: **this story indicts 15-3 for measuring
once and never looking again, and its own preflight went stale in twenty minutes because another
agent was writing the tree** — which is why T0 is about every number in this document, not only
the kloc ones. Paige took her line for the sixth consecutive session.

## 13. Round-table round two, 2026-09-08 — the baseline moved underneath the story

Reconvened to close the open items before dev. **It opened with the tree correcting the room for
the third time.**

`HEAD` was `bce5ac68` when this story was written. It is now **`2fd492c0`** — the same story
title, a **rewritten commit** (`git merge-base --is-ancestor bce5ac68 HEAD` → false, so
`git show bce5ac68:<path>` no longer resolves through HEAD's history). 15-3 amended a commit that
`sprint-status.yaml` records as **`done`**, adding 90 charged `xtask` lines, and **did not touch
`kloc.toml`**.

| Instrument | at `bce5ac68` | at `2fd492c0` |
|---|---|---|
| `xtask` tokei `code` | 43321 | **43411** |
| breach vs the 43095 ceiling | 226 | **316** |
| `check_exit_commands.rs` | 1344 | **1434** |
| workspace aggregate | 157096 | **157186** (still under 170884) |
| five exit-line gates | all EXIT 1 | **all EXIT 1, unchanged** |
| `check-exit-commands` | PASS 7/7/31/6 | **PASS, unchanged** |
| `cargo test -p xtask` | 1 failed | **519 passed / 1 failed — still only `kloc_check_runs_on_workspace`** |
| epic's 15-1 section | — | **byte-identical across the amend** |

### R-10 — this is a missing control, not a mistake

Three occurrences now, all the same crate, all the same story. **Twice is a mistake; three times
is an absent instrument.** And the instrument is absent for a reason 15-2 already established and
nobody joined up: **`check-epic-close-coherence` never opens `kloc.toml`.** It reads
`sprint-status.yaml`, `epics/index.md`, `epics/` and `kernel-core-baseline.toml`. So a story can
be marked `done`, amend its commit, add charged lines, and **no gate in this repository is
looking**. `kloc-check` would catch it on the next run — but nothing ties that run to the story
that caused it, which is why the debt landed on 15-1 instead of on 15-3.

**Filed to 20-3** (which is already restructuring xtask) as a *coherence-gate check*, not a new
gate: an epic cannot close while any crate it touched is over its ceiling, and a `done` story's
commit is the measurement point for its own row. **Deliberately not fixed here** — 15-1 is a
green-at-HEAD story, and building the control that would have prevented its own debt is the
scope creep this room exists to stop. But it is named, with its owner, rather than absorbed.

### The consequence for this story, and it is the whole of R-11

Dana put it plainly: *"if the crate next door can add ninety lines after it's done, 15-1 doesn't
have a grant — it has a moving target."* Wildcard offered two freezes, and Yui killed the first:
freezing the *writer* is a promise about someone else's behaviour, and this room has spent a
month on what promises are worth. **So the measurement is what freezes: the grant is computed at
15-1's landing commit, and any drift after that reds `kloc-check` and belongs to whoever wrote
it.** Which, as Murat noted, is the rule that was supposed to exist all along.

**Grumbal's closing, and it is the fairest thing said in two sessions:** *"You spent two sessions
building a story whose entire thesis is 'measure at the commit that lands, not the commit you
started on.' In the twenty hours between them the ground moved twice — once from an agent typing,
once from a rebase — and both times your own numbers were wrong until somebody went and looked.
I'm not complaining. I'm saying the story is right about the thing it's right about, and you
should stop being surprised by it."*

## Dev Agent Record

### Agent Model Used

zai/glm-5.3 (frontier-class allowlist, `FRONTIER_FAMILIES` member).

### Debug Log References

- **T0 @ `2fd492c0` (2026-09-08):** tree dirty in `_bmad-output/` only (story file staged-new,
  sprint-status, epic-15 §11 amendment, party-mode memlog) — **zero source dirty**, so the
  14-2d `git show HEAD:` fallback was not needed. `cargo build -p xtask` EXIT 0 (16 warnings,
  the R-1 pool); `cargo fmt --all --check` clean; `tokei 14.0.0` == CI pin.
- Five gates: all EXIT 1 with the story's exact findings (6 I9 rows; 36 NFR-Test-2 rows = 19
  removed + 17 `other`; 2 env vars at `main.rs:2703-2704`; 1 J1 finding; `xtask 43411 > 43095`).
- `cargo test --workspace --no-fail-fast`, TWO full runs: run 1 = 497 binaries / 4173 / **1** /
  118; run 2 = 497 / 4172 / **2** / 118. Run 2's failures: `t_14_2a_a_promoted_generation…`
  (flake) + `kloc_check::tests::kloc_check_runs_on_workspace` on `["xtask 43411 > 43095"]`
  (deterministic). `drr_backpressure` and `stdio_transport_malformed` passed both runs —
  consistent with their 3/20 and 7/20 rates. Union across runs = the story's 4-test inventory. ✅
- **R-6 re-derived by driving the gate's own `--json`:** removed-by-triple **19** = **18**
  hash-changed + **1** genuinely-absent (`get_default_image`); unique removed paths **17**;
  baseline items **370**; genuinely-new = **9 `(kind,path)` pairs over 8 unique paths**
  (`SuccessorSpiritFactory` = trait + use re-export). ALL FOUR R-6 numbers reproduce. ✅
- **T25 audit re-derived:** 38 `[[ship_gate]]` rows; at `v1_5` = **21 blocking / 14 advisory /
  3 blocking-when-present**; `check-cross-form-equiv` is the only advisory with no v2_0/v2_2
  lift. ✅
- **Aggregate measured:** job at `discipline.yml:3540`, **123 needs** (`:3543`–`:3665`), J1 job
  `:1939`, `v1-0-ship-gate` `:3414` with J1 need at `:3439`. 78 non-gate jobs (story's 66 =
  build/test subset; all 78 measured): **43 directly-measured legs GREEN** (8 feature-flag test
  arms, 8 release-mode tests, 7 benches, 12 one-shot smokes, 7 integration scripts, npm TS job,
  fmt/audit-grep/owasp-hash/nightly-grep). **REDS:** (1) `a2a-tcp-tests-8-6` — owned, AC6/T19;
  (2) **`onb-nfr2-timing` AC4 binary-size: 26,473,608 > 25,165,824 B** (release `maos`, local
  rustc 1.96.0-stable = the repo's pinned channel). The review found that deferring this direct
  aggregate dependency contradicted AC1; Lunarpulse authorized an in-story measured rebaseline
  to **28MiB / 29,360,128 B**, restoring 2.75MiB of headroom. `cargo deny check` advisories
  remain RED for RUSTSEC-2026-0268/-0269 in wasmtime 46.0.2, but the containing
  `reproducible-build` job is `continue-on-error: true` and cannot red the aggregate; filed to
  the 15-5 decision lane.
  Never-red-by-construction verified: `reproducible-build`, `nfr-perf-1/8` (toe=True),
  `determinism-tests` (advisory wrapper), `t3-escape-corpus`/`t3-smoke-busybox` (`|| echo skipped`).

### Completion Notes List

- **T0 complete — every figure in this story reproduced at HEAD; one NEW red found and resolved in review.**
  The only figure that could not be reproduced is one that does not exist: the story's
  "16 xtask/src warnings … 1 unused import" measures 16 warnings with **no unused import**
  (15-3 evidently took it); the R-1 census otherwise matches (3 un-deletable functions, 9
  unused-field rows, 2 dead assignments, 2 unused vars). Review restored the optional typed
  red-team `notes` field because deleting it weakened artifact validation; the measured
  `onb-nfr2-timing` red is resolved by the operator-authorized 28MiB rebaseline.
- **T1 (reclaim, evidence not funding):** deleted the 2 dead assignments + dead initializer
  (`consistency_ok`/`u_recomputed`), 2 unused locals (`cap_ref_patterns`, `_pass`), 3 optional
  unread serde fields + the orphaned `DerivationProvenance` struct, and the unused
  `CURRENT_PHASE` import — 16 warnings → 7. **Measured yield: 43411 → 43399 = 12 tokei lines
  against the 316 breach.** The 7 survivors: 3 are 14-0's named un-deletables; 4 rows are
  schema-REQUIRED `Deserialize` fields whose deletion would weaken artifact-file validation
  (behavior change, not dead code) — recorded rather than deleted.
- **T2–T6 (AC2):** attribute + doc moved back onto `SecurityManagerAdapter` (net-0);
  `ScbRuntimeSnapshot` (reason 56 chars, line 93) and `VerifiedImageLock` (57/94) got
  single-line attributes; 3 register entries written; pin 24472→24474 + kloc 18933→18935 in
  one commit; `check-empty-kernel` **PASSED (0 violations)**, `check-kernel-baseline` PASSED
  (24474==24474). Residual control: `exempt_but_undocumented_struct_reds` (proven red) +
  `exempt_and_documented_struct_passes` — **verified to red by reverting the T4
  `VerifiedImageLock` entry alone**, then restored green.
- **T7–T10 (AC3):** surgical baseline edit via the gate's own `--json` surface — 18 hash
  refreshes + 1 deletion (get_default_image), 370→369; +1 post-T2 refresh
  (`SecurityManagerAdapter`'s hash moved again when the attribute returned). R-6 numbers
  reproduce: **0 added / 1 removed / 18 changed / 370→369**. 8 genuinely-new paths classified
  under `# Story 15-1 AC3`; ADR-066 written + indexed. `check-service-boundary` **PASSED (0
  violations)**. Residual controls: `shipped_baseline_symbol_absent_from_surface_reds` +
  `shipped_baseline_matches_live_kernel_surface` (12/12 in the suite).
- **T11–T12 (AC1/D-2):** digit-exclusion filter + doc; gate **PASS**. New twin
  `planting_an_epic_numbered_foreign_lane_test_file_stays_green` green; the `xyz_1b.rs`
  vector still reds (28/28). **AC1's falsifier executed**: reverting ONLY the filter change
  reds the gate again (EXIT 1 with the 14-2a finding); restored → EXIT 0.
- **T13–T14 (AC5):** 2 `UserFacing` rows registered (88 vars total); dead-key message
  repointed on both paths to `21-4 / xtask env-contract.toml — D-I`. `check-env-contract`
  **PASS, EXIT 0**.
- **T15 (AC6(a), D-7 process isolation):** pre-fix measured **26/30** at 8 threads (full
  binary; story preflight: 24/30; `--exact`-filtered alone: 0/30, matching the isolation
  experiment). Test C + LogCapture moved verbatim to `t_14_2a_post_grace_token.rs` (own
  process; byte-identity of the test body against HEAD verified after fixing a
  transposed-arg transcription). Post-fix: **0/40 default, 0/50 @ 8 threads**. The
  `register_callsite` alternative was not needed.
- **T16 (AC6(b)):** fixed sleep → bounded wait (10 s deadline on `bw_rx.recv()`). Pre-fix
  alone 0/20 (matches "0/20 alone"); story's under-load 3/20 stands as the recorded pre-rate.
  Post-fix **0/40**.
- **T17 (AC6(c)):** fixture now drains stdin before echoing (`cat >/dev/null;`). Pre-fix
  alone 0/20 (story: 7/20 under load). Post-fix **0/40**. Transport EPIPE filed → 21-4 with
  the 300/300 reproduction inside the filing.
- **T18 (D-6):** `--test-threads=1` removed from `check_cert_rotation_trigger.rs` AFTER T15
  was proven; `TEST_FILES` grown 5→6 and the leg retargeted to the new binary (the gate
  shells the moved test by filter). `check-cert-rotation-trigger` **PASSED — oracle green (4
  legs, 39 enrolled rotation tests)** with the flag out.
- **T19 (sixth red):** the exact `a2a-tcp-tests-8-6` loop — 50× `cargo test -p maos-a2a-tcp
- **T20–T21 (AC7):** `workspace-test-suite` job (explicit `timeout-minutes: 45`; local
  wall-clock ~3 min for 497 binaries/4174 tests on 32 cores) enrolled in BOTH
  `v1-0-ship-gate.needs` (the registry-enrolled home; `EXPECTED_GATES` 40→41,
  `check-ship-gate-completeness` re-run **PASSED — all 41 expected gates**) and directly in
  `aggregate.needs` (123→124, D-3's ruling text). Falsifier: planted failing test → the job's
  exact command **EXIT 101** with the planted test FAILED → reverted (the fail-check step's
  `contains(needs.*.result,'failure')` propagates to the aggregate by construction).
- **T22–T24 (AC4):** final measured xtask tokei **43409** (43411 − 12 reclaim + 10 net new:
  D-2 filter + docs, AC5 repoint, D-18 retarget + sixth TEST_FILES entry, AC7 const row).
  **Grant: `xtask = 43444` = 43409 + 35**, one grant, one commit, R-2 attribution + R-11
  formula in the row comment. `kloc-check` **PASSED (aggregate=157191)**. T23: 15-3's row
  comment and sprint-status:235 corrected *filed-not-overwritten*. T24 falsifier: +43
  physical lines → `xtask 43452 > 43444` **EXIT 1**; reverted → EXIT 0 (a 1-line probe cannot
  red a +35 pricing ceiling — the both-directions proof is green-at-measured / red-past-grant).
- **T25 (advisory audit):** 38 `[[ship_gate]]` rows; effective at `CURRENT_PHASE = "v1_5"`:
  **21 blocking / 14 advisory / 3 blocking-when-present**. Every advisory row is
  phase-ladder-scheduled (→v2_0/v2_2) or substrate/engagement-gated. **One exception,
  flagged and filed → 20-3 (D-G): `check-cross-form-equiv`** — `{v1_0="advisory",
  v1_5="advisory"}` with **no v2_0/v2_2 row**, so it inherits advisory forever; the only
  demotion-shaped row in the registry with no end date.
- **T26 (§11 verification):** all eighteen landed edits verified present in
  `epic-15-foundations-w0.md` (header paragraph, :3540/:1939 corrections, ship-gate home,
  digit rule, invoke_cargo_test execution, AC2 budget constraint, 18/1 disproof, ADR-066
  home, AC4 replacement, AC5 cites, AC6 four-tests + D-6 + sixth red, AC7, Stories-table
  row). One R-7 re-fit applied: AC7's literal "in `aggregate.needs`" — the direct entry was
  added alongside the ship-gate one (124 needs at landing; the 123 figure was the preflight
  measurement).
- **T27 (R-4):** both reason strings added verbatim; enforcing gate filed → 20-3.
- **T0/AC1 scope honesty:** `onb-nfr2-timing` measured 26,473,608 B against the former
  25,165,824 B limit and is closed here by the review-authorized 28MiB rebaseline. The two
  wasmtime RustSec advisories remain filed to 15-5 inside `reproducible-build`, whose
  `continue-on-error: true` posture cannot red the aggregate. All other non-gate aggregate
  jobs measured green (43 legs) or never-red by construction.

### File List

- crates/maos-kernel-core/src/security/mod.rs (T2: attribute+doc move, net-0)
- crates/maos-kernel-core/src/scheduler/control_block.rs (T3: +1 attribute line)
- crates/maos-kernel-core/src/security/sandbox/t3/image_lock.rs (T3: +1 attribute line)
- docs/invariants/i9-exemptions.md (T4: 3 register entries)
- xtask/kernel-core-baseline.toml (T5: pin 24472→24474 + HISTORY row)
- xtask/kloc.toml (T5: kernel row 18935; T22: xtask grant 43444 + R-2/R-11 comment)
- xtask/tests/empty_kernel_integration.rs + fixtures/exempt-undocumented/ + fixtures/exempt-documented/ (T6)
- docs/ci-baselines/kernel-surface-v0.1-beta.json (T7: 18+1 refreshes, 370→369)
- xtask/kernel-api-classes.toml (T8: 8 new-path rows)
- docs/adr/ADR-066-sanctioned-kernel-surface-removal-and-verified-type-signatures.md (T9, new)
- docs/adr/index.md (T9: ADR-066 row)
- xtask/tests/service_boundary_integration.rs (T10: 2 controls)
- xtask/src/check_j1_loopback_delegation.rs (T11: D-2 filter + doc)
- xtask/tests/j1_crosshost_1b_proven_red.rs (T12: assert_green twin)
- crates/maos-bin/src/env_contract.rs (T13: 2 UserFacing rows)
- xtask/src/check_env_contract.rs (T14: message repoint)
- crates/maos-a2a-tcp/tests/t_14_2a_post_grace_token.rs (T15, NEW binary)
- crates/maos-a2a-tcp/tests/t_14_2a_post_grace_journal.rs (T15: test C removed, pointer comment)
- crates/maos-kernel-core/tests/drr_scheduler.rs (T16: bounded wait)
- crates/maos-mcp/tests/stdio_transport_test.rs (T17: fixture drains stdin)
- xtask/src/check_cert_rotation_trigger.rs (T18: flag dropped; TEST_FILES 5→6; leg retarget)
- .github/workflows/discipline.yml (T20: job + v1-0-ship-gate.needs + aggregate.needs + summary/fail rows)
- tests/integration/onb_nfr2_timing.sh (review: measured 28MiB binary-size rebaseline)
- xtask/gate-registry.toml (T20: gates list + [[ship_gate]] row)
- xtask/src/check_ship_gate_completeness.rs (T20: EXPECTED_GATES 40→41)
- crates/maos-siem/tests/fault_inject.rs + crates/maos-kernel-core/tests/cgroup_ceiling_smoke.rs (T27: ignore reasons)
- xtask/src/check_cross_form_equiv.rs, check_red_team_gate.rs, check_third_party_trial.rs, coverage_matrix_nfr_test_3.rs, check_epic_6_bridge.rs (T1 reclaim)
- _bmad-output/implementation-artifacts/sprint-status.yaml (in-progress flip; 15-3 correction; filings to 15-4/15-5/20-3/21-4/epic-15-retro)
- _bmad-output/planning-artifacts/epics/epic-15-foundations-w0.md, epic-18-spirits-that-think-w3.md, epic-20-ship-it-w5.md (T26 verification + pin re-cites 24474)
- crates/maos-telemetry/src/otel_sink.rs (review verification: synchronous exporter-pause acknowledgement)
- crates/maos-kernel-core/tests/on_revocation_three_actions.rs (review verification: semantic deferred-unload wait)
- _bmad-output/implementation-artifacts/15-1-green-at-head.md (this record)

### Change Log

- 2026-09-08: T0 re-measurement at `2fd492c0` — all story figures reproduce; workspace suite
  run twice (4173/1 and 4172/2 of 4174); non-gate aggregate jobs measured (43 green legs, 2
  reds: owned `a2a-tcp-tests-8-6`; `onb-nfr2-timing`, closed by the review rebaseline).
- 2026-09-08: AC2+AC3 landed (I9 repair +2 kernel lines, both instruments moved in one
  commit; surgical baseline 370→369 with the R-6 symmetric diff reproduced; 8
  classifications; ADR-066; three residual controls, each proven red).
- 2026-09-08: AC1's J1 fix (D-2 digit exclusion) + revert-falsifier; AC5 env rows + repoint.
- 2026-09-08: AC6 — three race fixes with repeat-count oracles (pre 26/30 · 3/20 · 7/20 →
  post 0/40+0/50 · 0/40 · 0/40), `--test-threads=1` dropped post-proof with the gate
  re-run green, `a2a-tcp-tests-8-6`'s exact 50× loop 0/50.
- 2026-09-08: AC7 — `workspace-test-suite` Blocking job enrolled at all four sites (41
  expected gates; aggregate needs 123→124); planted-test falsifier EXIT 101.
- 2026-09-08: AC4 — reclaim attempted first and cannot fund it (12 lines); one measured
  grant `xtask = 43444` = final 43409 + 35; 15-3 corrections filed not overwritten; ceiling
  falsifier `43452 > 43444` reds, reverted green.
- 2026-09-08: Code review — seven findings patched; binary-size limit rebaselined from 24MiB
  to 28MiB from the measured 26,473,608-byte artifact; workspace CI installs ABI tooling;
  stdio fixtures consume stdin; red-team notes remain typed; J1 numeric stems stay governed;
  KLOC falsifier and grant-history contracts corrected.
- 2026-09-08: Review verification — removed two aggregate-only scheduling flakes by
  acknowledging exporter pause before saturation assertions and polling deferred revocation
  state instead of assuming four yields; exact CI setup and all focused gates passed;
  `cargo test --workspace --locked --no-fail-fast`: 4181 passed, 0 failed, 118 ignored.
