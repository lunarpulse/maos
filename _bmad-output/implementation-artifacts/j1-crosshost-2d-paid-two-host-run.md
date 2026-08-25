---
baseline_commit: "`dd4cf959` — HEAD, working tree clean apart from this story and its sprint-status row. Every number and every `file:line` below was measured against a committed HEAD and is reproducible by `git checkout dd4cf959`."
depends_on: "**`j1-crosshost-2e-two-host-run-enablement`** for AC8 ONLY (the six code blockers F1-F5, F7 — ratified at the 2026-08-21 round-table). Everything else — AC1-AC7 — is startable today. · `j1-crosshost-2c`: `done`, §A6 CLOSED 2026-08-18, committed `dd4cf959`; **DISCHARGED** (the sprint-status clause *'its §A6 review is not yet run'* expired the day it was written)."
blocks: "The J1 lane's v1.0 rung. Last story in the lane — no successor to hand the real thing to."
kernel_grant: "NONE and none needed. `check-kernel-baseline` GREEN at **24472 == 24472**, executed against HEAD. Zero Rust."
kloc_grant: "**NONE, and taking one would be a violation.** `kloc_check` counts `--types Rust` only (`xtask/src/kloc_check.rs:167-190`); markdown and `_bmad-output/` are not counted. ⚠ `kloc-check` is **BLOCKING and RED at HEAD** on four keys, executed 2026-08-19: `aggregate 151124 >= 147057` (D17), `maos-kernel-core 18933 > 18248` (D13), `maos-domain 8695 > 8644` (D14), and **`xtask 39966 > 39960` — undisclosed at authoring, ROUTED at the round-table to `2e` (AC5.1)**. This story must NOT absorb the `+6`: D17's ruling is that a grant requires *a measured delta to justify it*, and this story's Rust delta is zero. See AC6.2 — **`all gates green` is not available as a done criterion.**"
model: "frontier-class allowlist {opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, opus-5, equiv}. The literal token `allowlist {` is deliberate: `check_dev_model_used_populated.rs:302` uses it as the boilerplate guard, and without it `:337-344` would extract a model from this POLICY LIST and satisfy `check-dev-model-tier` VACUOUSLY with no dev ever recording what actually ran."
review: "§A6 full-layer net (Blind + Edge + Acceptance + Test-Infra + runtime) — NON-DEGRADABLE, and **re-targeted from the diff to the artifacts** (AC7). The three skill-defined layers consume `{diff_output}` (`.claude/skills/bmad-code-review/steps/step-02-review.md:18-32`); this story's diff is markdown and JSON, so run as-specified Blind Hunter and Test-Infra Auditor have nothing to consume — a degraded review arriving BY CONSTRUCTION, which §A6 forbids by name."
---

# j1-crosshost-2d — the paid two-host run

Status: **in-progress** — AC1-AC7 COMPLETE and verified by execution (dev pass 2026-08-22,
`anthropic/claude-opus-5`, baseline `dd4cf959`). **AC8 is the only open AC**, gated on
`j1-crosshost-2e` (`backlog`, no story file). Not `review`: one of eight ACs is untouched, and
two of its blockers (F5, F7) bill money before they fail.

> **What a dev can do today: AC1-AC7, all of it, code-free.** AC8 — the paid run itself — waits on
> `j1-crosshost-2e`, which owns the six code defects that make the run impossible or worthless. That
> dependency is a **named row with an owner**, not an open question, which is the difference between
> this story being `ready-for-dev` and `blocked`.
>
> **Why this row keeps the paid run rather than shedding it.** The round-table tried to take it away —
> a row named `paid-two-host-run` that cannot run is a name making a claim it can't support. It
> cannot be taken away. `j1-crosshost-2d-paid-two-host-run` is pinned in three places:
> `xtask/src/demo_j1.rs:911` (the beat's owner string), `check_j1_two_host_signed_run.rs:879-889` (a
> **Blocking** gate that REDS if `demo_j1.rs` stops naming it), and `xtask/src/tests/demo_j1_tests.rs:55`
> (an enrolled pin). **Renaming this row, or moving the run to a successor, costs the exact Rust this
> row is forbidden to write.** The machine assigned the paid run to this key. That is the ruling.

---

## 📌 READ THIS FIRST — every open call is already decided

*Ten calls closed at the **2026-08-21 round-table** (Mary · Paige · John · Sally · Winston · Amelia ·
Murat, with Grumbal, Dana, Level, Vex and Killjoy walking on), per spec + long-term correctness.
**Two of them are defects in THIS FILE's first draft** and are marked ⛔ — if you read the ACs without
this table you will do two things wrong.*

| # | Call | **Ratified answer** | Trap |
|---|---|---|---|
| R1 | Q1 — what is this lane's evidence, now that `PROVEN_LIVE_SIGNED` is unreachable (F2)? | **The two bundle signatures**, verified by the third-party `verify.py`, plus a `reconcile-hosts` that actually executes. The beat lands `INDETERMINATE` and `two_host_signed_run_claimed: false` is **published as a true fact**, not hidden. | **T6 — the only signed run this project has ever done — was signed exactly this way and never touched `MAOS-EVIDENCE-V1`.** That mechanism arrived later, in 13.6e, for the Postgres gates. The target was mis-specified, not merely unbuilt. |
| R2 | Q2 — does "code-free by construction" survive? | **YES, intact.** A new row **`j1-crosshost-2e`** takes all six code blockers. This row writes zero Rust. | Grumbal's condition is unamended. Do not "just fix the one boolean." |
| R3 | Q3 — two processes on one box, or two machines? | **Two processes on one box.** `"shape": "two real OS processes on one box"` — the shipped template string, and the one `good_capture()` uses. | **The reason is not cost.** `CLAIM_SCOPE` is byte-pinned and reads *"…**not** two machines…"*. Buying a second machine buys a property **the artifact is contractually forbidden to assert**. Vex's addendum: physical separation is not the threat model — one seed attesting two identities is, and two boxes sharing a key file is still one root. |
| R4 | Q4 — is codex-on-host-A still in scope? | **NO.** Drop it. Prove the second adapter **unbilled** via the fake-CLI harness (AC2.2). | The cross-host arm sets `local_worker_spawned: false` and hands the worker no goal; the only goal the system can mint is rejected by codex's oracle *after billing* (F7). **Stop calling the run "heterogeneous" in the row text** — restate the claim to match what happens. |
| R5 | ⛔ **Defect in this file's AC1.2** | **Prove release-ness via a LOOPBACK `maos run … --once`, never the daemon.** | The first draft said "boot the daemon twice." **The daemon cannot boot (F1)** — the story's own first task was blocked by the story's own first blocker, and was marked *"free."* `cohort_daemon` is an `Option` (`main.rs:2452-2455`), the boot nonce is minted on every path (`main.rs:1865`), and a loopback run needs no cohort manifest. Same falsifier, no manifest — and **more** correct, since the question is whether `cfg!(debug_assertions)` is off in that binary, which has nothing to do with cohort membership. |
| R6 | ⛔ **Defect in this file's AC4.4** | **There are TWO capture documents. Cite them separately.** | The first draft justified extra fields by citing `subcommands.rs:2409-2412` — which is **`CaptureDoc`**, the `record-capture` document, whose doc comment explicitly permits extra fields. `two-host-capture.json` is a **different file read by a different validator**; it tolerates unknown keys by *silence*, and every extra top-level string it holds is **fed to the overclaim tripwire**. Two documents, two rulesets. New shape, minuted: **a preflight inherits the defect it diagnosed** — this is the same two-contracts-one-concept bug we filed against the README. |
| R7 | The undisclosed `xtask +6` | **Routed to `2e`**, which will edit `demo_j1.rs` and therefore has a measured delta. One grant covering `2c`'s six and its own. | **This row must not take it.** D17: a grant needs a measured delta to justify it; this row's is zero. Absorbing it is precisely what 14-6 was forbidden from doing to D13. |
| R8 | `deferred-work.md:826` (F8) | **Routed to 14-4**, the v2.0 operational-surface sweep that already owns D7 and D18. Fault typing on a cross-host operator surface is that family. | Re-own it in that file's standing vocabulary (AC5.3) or the stale-owner red fires the moment this row reaches `done`. **Routing a lane is not the same as having a story** — say so in the entry. |
| R9 | Sequencing of the capture | **Gated on `2e` BY MECHANISM, not by promise.** | You cannot land the capture before the demo fix: F3 fires the instant `two-host-capture.json` appears. John's note: *promises decay; mechanisms don't.* |
| R10 | Does this row keep AC8? | **YES — it cannot do otherwise.** See the header: three machine pins own the name. | Grumbal's "the name lies" objection was **withdrawn on measurement**, not overruled. First time in this lane the enforcement pointed at us rather than for us. |

---

## The findings — F1-F9

### F1 — ⛔ 2e: nothing in the workspace can sign a cohort manifest

`CohortManifest::signed_with` (`crates/maos-cohort/src/manifest.rs:546`) has **zero non-test callers**.
Every call site is under `tests/` except `crates/maos-bin/src/main.rs:12997`, which sits inside
`#[cfg(all(test, feature = "network"))] mod story_13_5a_enterprise_daemon_seam` (`:12710-12711`) —
verified by reading the enclosing attribute, not by grep alone.

The daemon refuses to boot without a `manifest_path` verified against `authority_keys`
(`main.rs:8853-8854`, checked `:8949`), and `reconcile_transport_identity_with_manifest`
(`:9656-9740`) requires every pin and peer fingerprint — **including the host's own leaf**
(`:9711-9726`) — to byte-match it. **Host B cannot start.** Blocks before spend, but absolutely.

### F2 — ⛔ 2e (or re-scope per R1): `PROVEN_LIVE_SIGNED` is structurally unreachable

`two_host_signed_run_claimed = capture_present && oracle_green && capture_signature_verified`
(`check_j1_two_host_signed_run.rs:1322-1326`). The third term needs a `MAOS-EVIDENCE-V1` record whose
`nonce` matches a binding **recomputed at gate-run time**. On the operator lane that nonce is
`format!("{gate}.{:x}.{nanos:x}", std::process::id())` (`evidence_ledger.rs:415-431`) — fresh per
process, per nanosecond. No file written beforehand can carry it. Compounding: `commit` is
`local_worktree_commit()`, a hash over HEAD *plus every untracked file's bytes*, so writing the
transcript changes the value it must contain.

**And nothing produces the file.** The sole signer is `tests/harness/evidence_record.rs`, which emits
only when a gate exports `MAOS_EVIDENCE_{GATE,COMMIT,NONCE}` via `harness_env()`
(`evidence_ledger.rs:1072-1079`) — called from exactly two places, neither this gate. The four sibling
ledger gates produce their transcript **in the same process**; this one reads a static file. J1 is not
in `ledger_gates()` (`:148-150`). The design intent is explicit
(`tests/harness/evidence_record.rs:6-10`): *"A gate that signed a transcript after reading it would
attest 'the gate saw this text', not 'the test produced it' — the judge grading its own code."*

### F3 — ⛔ 2e: landing the capture breaks the founder demo

`apply_two_host_signed_run` sets `beat.executed = true` in **every** present-capture branch, including
`Indeterminate` (`demo_j1.rs:971`). `Beat::failed() = executed && !state.is_proven()` (`:118-121`) and
`is_proven()` admits only `ProvenBlocking | ProvenLiveSigned` (`gate_common.rs:357-364`); any failed
beat returns `Err` (`:352-363`). So the capture landing without a verifying transcript — the only
state reachable per F2 — makes `demo-j1` exit nonzero. The same file's doc comment at `:936-938` says
this state *"is not a failure."* The gate honours that; the demo does not.

**Blast radius, measured:** CI stays green — the binary has no CI invocation, and the enrolled tests
assert against `unlanded_beats()` (`:901-919`), the static declaration. The damage is to the
founder-facing demo, which for this lane is worse.

### F4 — ⛔ 2e: the documented pairing procedure is impossible

`cohort:daemon-started` is written only in daemon mode (`main.rs:9381`, emit `:9548-9555`), but **host
A is `maos run --once`**, which takes the cross-host arm *precisely because*
`MAOS_ONE_SHOT != "cohort-a2a-daemon"` (`:2455`) — **host A never emits the row the procedure says to
read.** The nonce does not exist before the dial (created `:1865`, transport binds `:2457`, delegation
emitted `:3250-3270`, same process); connect is not retryable (`mtls.rs:73-83`, ~1.4 s budget); and
re-running yields a different nonce. The file named (`a2a-peers.toml`) exists nowhere — the real
surface is `tcp.peer_pins[].boot_nonce` (`crates/maos-a2a-tcp/src/config.rs:25-37`) under
`deny_unknown_fields`. The read-back flag is `--intent-contains`, not `--intent`
(`crates/maos-cli/src/cli.rs:376-378`). The only route that works today is holding host A under a
debugger. That is not an operator procedure.

### F5 — ⛔ 2e, and it burns both agents: the stranger's path is broken today

`tools/verify-audit-bundle/verify.py:93` omits `ensure_ascii=False`, so Python escapes non-ASCII to
`\uXXXX` while Rust's `canonicalize_value` (`crates/maos-audit/src/sealed_export.rs:632-639`) emits raw
UTF-8. **Reproduced free at HEAD against the real T6 artifact:**

```
$ python3 tools/verify-audit-bundle/verify.py \
    _bmad-output/test-artifacts/j1-tier2-evidence/j1-tier2-bundle.json \
    61f4f495dba703e74aff7d42b4286a1a914a89b592a98bf76ed3656c81107766
FAIL — signature verification failed          # exit 1
```

That bundle carries 12 non-ASCII bytes; the signature is valid. `README.md:54-59`'s OpenSSL fallback
has the identical bug. CI installs `cryptography` "for the AC2.1 stranger's path"
(`discipline.yml:1919-1920`) but leg 9 never executes `verify.py`. **Runbook Phase 7.4 is marked NOT
optional and a rejection is an abort condition — so today the run dies after both agents are billed.**

### F6 — the judge cannot tell a real run from a forgery, and a committed test proves it

`xtask/tests/j1_crosshost_2c_proven_red.rs:386-414` writes the real published template plus two bundle
halves from `good_bundle()` (`:283-294`) carrying **`"attester_pubkey": "aa".repeat(32)` — identical in
both halves, one root** — and `"signature": "bb".repeat(64)`, with **no transcript**, then asserts
`verdict.passed && verdict.success`.

It passes because the two-root property is never checked against the artifacts: leg 3
(`:395-478`) is a **source-text grep** of `sealed_export.rs` and executes nothing; leg 9 (`:1006-1034`)
compares only each `bundle["host"]` string to the capture; and leg 3 at `:458-470` **forbids**
reconciliation from reading `attester_pubkey` (R-RG1), so the artifacts structurally cannot carry the
discriminator. The gate has **zero `Command::new`**. The only evidence of independence is a boolean
that ships **pre-filled `true`** in the template the operator is told to copy
(`two-host-capture.example.json:7`).

**This is not routed to 2e.** It is resolved *here*, code-free, by AC4: the discriminators become
operator-performed and pasted, because the gate cannot check them.

### F7 — ⛔ 2e: the delegated goal cannot satisfy codex's oracle

The goal is a hardcoded constant, `"founder-loop: execute the delegated assignment from {FROM_HOST}"`
(`main.rs:3247-3251`), with no env override. codex's oracle requires `item.completed` / `file_change` /
`status:"completed"` (`worker_cli.rs:437-457`); it returns `NoEffectEvidence` → `NotCompleted` →
`main.rs:3386-3392` errors the run, **after billing**. claude is asymmetric: its oracle only proves no
tool permission was *denied*, so a model that declines scores `Completed` — named in-source at
`worker_cli.rs:512-519`. **codex fails loudly; claude passes weakly.** Hence R4.

### F8 — this row is assigned a Rust task it may not do, and it reds a gate at `done`

`deferred-work.md:826` names this row owner of a fault-typing fix in
`crates/maos-a2a-tcp/src/transport.rs:602-603`. `check_dev_record_completeness.rs:182-183` marks an
owner `Stale` iff its sprint status is `done`, and stale owners are unconditional violations
(`:546-555`) — reproduced in a sandbox. **Resolution is forced: re-own it** (R8, AC5.3).

### F9 — the row's name is machine-pinned, which is why AC8 stays here

`xtask/src/demo_j1.rs:911` holds the literal; `check_j1_two_host_signed_run.rs:879-889` is a
**Blocking** leg that files a finding if `demo_j1.rs` stops naming it; `xtask/src/tests/demo_j1_tests.rs:55`
pins it in an enrolled test. Renaming this row or moving the paid run to a successor requires editing
all three — the exact Rust this row is forbidden to write. **The enforcement assigned the run to this
key.**

---

## Blocking conditions

**Gating AC8 only — all owned by `j1-crosshost-2e`:** F1 (manifest signer), F2 (evidence target — or
re-scope per R1), F3 (demo tolerates a validated-but-unsigned capture), F4 (a pairing path without a
debugger), F5 (`verify.py` canonicalization), F7 (an expressible goal, or codex formally out per R4).

**Discharged by this story, before `done`:** F8 via AC5.3, and the `xtask +6` disclosure via AC5.1.

**Free and mechanically checkable right now** — run it before anything else, because if it passes,
F5 has been fixed since authoring and this story must be re-measured:

```bash
python3 tools/verify-audit-bundle/verify.py \
  _bmad-output/test-artifacts/j1-tier2-evidence/j1-tier2-bundle.json \
  61f4f495dba703e74aff7d42b4286a1a914a89b592a98bf76ed3656c81107766
```

---

## Story

**As** the founder who has spent five stories building a judge for a two-host signed run,
**I want** the run's preconditions proven unbilled, its evidence made checkable by a stranger rather
than sworn by me, and every reason it cannot yet happen written down with a measurement attached —
**so that** when the paid run does happen it produces something a third party can verify, instead of
an artifact that would look identical if I had never run it.

---

## Acceptance Criteria (8)

### AC1 — Prove the release build is real, because nothing else does

`MAOS_TEST_BOOT_NONCE` is gated by a **runtime `cfg!(debug_assertions)`** (`main.rs:1865-1878`), not
`#[cfg]` and not a feature. `debug_assertions` is a *codegen flag*, not the profile:
`RUSTFLAGS="-C debug-assertions=yes" cargo build --release` re-enables the shortcut silently, and
`check-mock-not-in-release` cannot see it — it greps the symbol table for
`MockHaltResolver`/`FailingHaltResolver` only (`xtask/src/check_mock_not_in_release.rs:31`). **No
artifact-level proof exists that a binary is a genuine release binary.**

1. Build **both**: `cargo build --release -p maos-bin -p maos-cli`. ⚠ `release.yml:44` builds
   `maos-bin` only — release artifacts ship **no `maosctl`**, and `maosctl` is the only read-back tool.
2. **Via a LOOPBACK `maos run <topology> --once`, never the daemon** (R5 — the daemon cannot boot,
   F1). Run it **twice** with the **same** `MAOS_TEST_BOOT_NONCE` set, and read both nonces back from
   the TL. **Two distinct values, neither equal to the override ⇒ genuine release build. One value
   equal to the override ⇒ debug-assertions are on; STOP.** Costs nothing, needs no cohort manifest,
   and has never been done.
3. Record the release binary's `sha256`. It is the anchor for AC4.4.

### AC2 — Rehearse everything reachable with zero spend

1. **Fixture route.** `FixtureCli` is exempt from `live_agent_gate` **by name** (`worker_cli.rs:857-869`)
   and its built-in grant already covers it (`worker_spawn.rs:157-166`). Note honestly what this does
   and does not prove: `cargo test -p maos-bin --test two_host_delegation_2b` is **already enrolled in
   CI** and uses pre-chosen nonces on a debug build — **re-running it is not a rehearsal of anything
   new.** Run it as a baseline, and say so.
2. **Fake-adapter route — this is the one that earns its keep.** Plant `#!/bin/sh` scripts named
   `codex` and `claude` on a prepended `PATH` that print oracle-shaped JSON and exit 0 (template:
   `crates/maos-bin/tests/worker_completion_2a.rs:767-880`, `plant_fake_cli` `:797-806`). Because
   `select_worker_cli` dispatches on the **basename** of the resolved binary (`worker_cli.rs:833-848`),
   this drives the *entire* live path — argv-flag refusals, ambient-auth check, cap-token mint, TL
   journaling, completion oracle — with `MAOS_LIVE_AGENT=1` and zero spend. **This is also how R4's
   dropped codex leg gets proven.**
3. **Dry-run the exact intended `CaptureDoc` through `maosctl audit record-capture` against a scratch
   TL.** Use the **real strings** that will be submitted, never a paraphrase — this is the falsifier
   for the `sk-` substring landmine (`contains_prefix`, `crates/maos-iac/src/adapter/redaction.rs:291-297`;
   refusal at `crates/maos-cli/src/subcommands.rs:2598-2606`, which journals nothing, **after** the paid
   run). `codex exec --task-file …` contains `task-`, which contains `sk-`; so do `risk-accepted` and
   `disk-backed`. **T6 escaped only by luck** — its `command_metadata` reads `<c2 task>`, `task>` not
   `task-`. ⚠ **Set `two_host_shape` in the dry run**, or `validate_two_host` is skipped entirely
   (`subcommands.rs:2528-2574`) and the block you most need to exercise never runs.
4. `cargo run -p xtask -- check-j1-two-host-signed-run --json` and
   `cargo test -p xtask --test j1_crosshost_2c_proven_red -- --test-threads=1`.

### AC3 — Repair the operator documentation, which would refuse the operator after billing

All markdown. Every defect verified against source at HEAD.

**`_bmad-output/test-artifacts/j1-two-host-evidence/README.md`**

1. **`:35` instructs the operator to write a string the gate refuses.** It says *"say which it actually
   was: two processes on one box, or two machines"*; the tripwire (`check_j1_two_host_signed_run.rs:975-1002`,
   phrases `:985`, normalization `:984`) fires on `two machines` unless the literal `not` immediately
   precedes it (`preceded_by_not` `:1039-1045`). Publish a sanctioned honest string for a genuine
   two-machine run — **this stays in scope even though R3 picked two processes**, because the defect
   outlives this story. The file's own §"Overclaim tripwires" four lines later contradicts this row.
2. `:39` overstates `stranger_verification` as *"the result of verify.py"*; the validator checks
   **non-emptiness only** (`:910-926`). The template even ships the acceptable string (`:8`).
3. `:34` omits that `host_a`/`host_b` are compared **trimmed** (`:949`); `:49` says "each
   operator-authored string field" when the scan is **top-level strings only**, never recursed
   (`:976-983`). Add the type contract: four JSON **strings**, two JSON **booleans** — the string
   `"true"` fails at `:932`.
4. Document the trap with no documentation at all: **any operator-added field repeating the claim
   scope REDs on `two operators`**, because that phrase is preceded by `or `, not `not `. The
   `claim_scope` exemption (`:978-980`) is load-bearing and its bytes are un-repeatable elsewhere.
5. `:14-24` misattributes ownership: the bundle halves are validated by **leg 4** (`:605-625`)
   unconditionally, and `two-host-evidence.txt` is read by **no leg at all**. Per F2, state that the
   transcript has no producer and that R1 re-scoped the evidence to the bundle signatures.

**`_bmad-output/test-artifacts/runbook-j1-tier-2-signed-live-run.md`**

6. `:165-177` — the **Phase 4 capture template is invalid**: it omits `fs_jail` and `fs_jail_followup`,
   both required (`subcommands.rs:2479`, `:2504-2514`). Following Phase 4 verbatim is refused at Phase
   5a **after billing**.
7. `:220` — `<FPR>` is **not** the pubkey from `keygen`, which prints a truncated fingerprint
   (`crates/maos-domain/src/audit_key.rs:397-405`) and **ignores `MAOS_AUDIT_KEY`** (`:101-106`). The
   64-hex pubkey comes only from `sealed-export`'s stderr (`subcommands.rs:2302-2307`);
   `verify.py:135-142` rejects the truncated form.
8. Phase 7.0 — replace the literal `...` with the real launch
   (`MAOS_ONE_SHOT=cohort-a2a-daemon MAOS_COHORT_DAEMON_CONFIG=<abs>`; mode is env-selected, `maos` has
   no daemon subcommand); `--intent` → `--intent-contains`; delete `a2a-peers.toml`, point at
   `[[tcp.peer_pins]]` with all three required keys under `deny_unknown_fields`. **Mark Phase 7.0 as
   superseded pending `2e` (F4) rather than leaving a procedure that cannot be executed.**
9. **Warn against `--format plain`.** `crates/maos-audit/src/lib.rs:808` renders `boot_nonce` as
   `{:016x}` under a column header literally named `boot_nonce`, feeding a TOML field parsed as
   **decimal**. Most hex contains `a`-`f` and TOML rejects it loudly; an all-decimal-digit hex **parses
   silently** and fails only at the first inbound frame. Name the shape: *a human-readable format that
   is only readable by a machine that knows the base.* Use `--format ndjson`.
10. Phase 7 names none of the cross-host substrate. Add
    `spirits/topologies/j1-founder-loop-crosshost.toml`, host B's
    `worker_manifest = spirits/worker/manifest-claude.toml`, the `claude` grant
    (`signing_key_id = "Anthropic"`), `ANTHROPIC_API_KEY`, and a definition for `$OPERATOR_KEY` (used
    at `:355`, never defined). Document `MAOS_AUDIT_KEY` — **zero occurrences** across all three docs,
    yet required by `verify_capture_signature` and the default for `--receipt-key`.
11. `:431-433` — the `cp` block contradicts its own `cd` at `:417` and omits `two-host-evidence.txt`.
12. State the two asymmetries an operator will otherwise misdiagnose: a `-32004` nonce refusal writes
    **no TL row on host B** (only the sender learns — `transport.rs:715/750/1018` covers TLS-handshake
    mismatch only, so `:311-314`'s "both sides journal" is wrong here); and the refusal is **permanent
    and cascading** — `invalidate_if_boot_nonce_differs` (`crates/maos-a2a-core/src/tofu.rs:351-372`)
    invalidates the pin, so the **second** attempt fails with a *different, misleading* error and
    recovery needs a host B restart.
13. **`runbook-j1-demo.md` is stale in five places** — `:31-32` says "eight … PROVEN_BLOCKING, four
    ABSENT" (actual **19 / 3**); `:38-51` lists 12 of 22 beats; `:48` marks
    `disallowed-intent-refused-blocking` ABSENT (actual `PROVEN_BLOCKING`); `:50` names the owner
    `j1-crosshost-2` (actual `j1-crosshost-2d-paid-two-host-run`); `:84-86` claims the codex topology
    and manifests must be authored, but all three ship at HEAD.

### AC4 — Make the evidence checkable by a stranger, because the gate cannot check it

The story's spine. F6 established the gate accepts a forged, single-root, unsigned pair, so the
discriminators must be **operator-performed and pasted**, and the gap recorded rather than papered over.

1. **Pre-publish `FPR_A` and `FPR_B` in the repo *before* the run.** Runbook `:311-329` says "publish
   FPR_A/FPR_B" and names no file. A pubkey read out of our own `sealed-export` after the fact proves
   internal consistency, not identity — **a self-check wearing a stranger's coat**. Commit them to a
   named file and have the evidence reference it.
2. **Run `maosctl audit reconcile-hosts` with `--seed-a`/`--seed-b`, never `--pubkey-a`/`--pubkey-b`.**
   Not style: `sign_bundle` derives per-region (`sealed_export.rs:305-313`), so one base seed under two
   `MAOS_REGION_HOME` values yields two distinct pubkeys and `key_a == key_b` never fires. The CLI's
   cross-derivation guard (`subcommands.rs:2900-2937`) runs **only** on the seed form; with `--pubkey-*`
   both `base_seed_*` are `None` (`:3020-3026`) and the check is **skipped entirely** — the source says
   so at `:2905-2908`. Simplest safe posture: leave `MAOS_REGION_HOME` unset on both hosts.
3. **Assert `attester_pubkey_A != attester_pubkey_B`** by reading both landed bundles, and paste both
   values. This is the check leg 9 structurally cannot make.
4. **Record what the gate's capture cannot — and mind that these are TWO DIFFERENT DOCUMENTS (R6).**
   - **`CaptureDoc`** (`maosctl audit record-capture`) **explicitly permits extra fields and preserves
     them verbatim in the journaled row** — its own doc comment says so (`subcommands.rs:2409-2412`).
     Put the release-binary `sha256` and build provenance from AC1.3 here. This is the durable home,
     because the row is covered by the bundle signature.
   - **`two-host-capture.json`** (the gate's capture) is a *different file, different validator*. It
     tolerates unknown keys only by **silence** — there is no `additionalProperties` rule for it — and
     **every extra top-level string is fed to the overclaim tripwire.** If you add anything here, keep
     it clear of `sk-` / `AKIA` / `ASIA` substrings and of the three tripwire phrases.
   Without this, a debug-build run produces a byte-identical, gate-admissible capture (F1/AC1).
5. **Paste `cargo run -p xtask -- demo-j1`'s rendered beat line.** No test verifies the beat flips:
   `demo_j1_tests.rs:48-63` reads `unlanded_beats()`, the static declaration, while its own comment
   claims to test the render. A gate verdict is not a substitute for the rendered ledger.
6. **Key the claim on the JSON fields, never the exit code.** `paid_run_capture_present`,
   `two_host_signed_run_claimed` and `capture_signature_verified` are the discriminators;
   `passed`/`oracle_green` are green whether the capture is absent, valid, **or fabricated**. Per R1,
   `two_host_signed_run_claimed: false` is **published as a true fact**, not treated as a failure.

### AC5 — Close the lane's record honestly

1. **State the pre-existing red set** and correct `sprint-status.yaml:271`, which claims reds are
   "only the standing D13/D14 keys" — false on two counts at HEAD. **Route the `xtask +6` to `2e`
   (R7); do not absorb it here** — D17 forbids a grant without a measured delta, and this row's is zero.
2. **Add the J1 two-host rows to `_bmad-output/test-artifacts/traceability-matrix.md`**, including the
   rows that record absence: the transcript has no producer (F2), and leg 9 accepts a single-root
   forgery (F6). A traceability matrix that lists only what passed is a marketing document.
3. **Re-own `deferred-work.md:826` to 14-4** (R8), in that file's standing vocabulary, stating that
   routing a lane is not the same as having a story. Add to the runbook: if the paid run stalls,
   `TransportFailed("awaiting response")` with **no frame id** is the expected untyped rendering and
   does **not** distinguish a partition from a slow agent.
4. **Record the claim boundaries in `RELEASE-HOLDS.md`**: leg 9 validates a *sworn* two-root claim, not
   a verified one; and `PROVEN_LIVE_SIGNED` is unreachable for this gate, with R1's re-scope named.

### AC6 — Define `done` against what is actually machine-checked

1. Machine-checked and required: the story file exists (`gate_common.rs:89-100` — `ready-for-dev` is an
   ACTIVE status, so file and status must land together or five Blocking gates `Err`); an extractable
   frontier model and a §A6 marker; non-empty `### Completion Notes List` and `### File List`; no
   `_No review findings.` placeholder and no `**open**` rows at `done`;
   `check-j1-two-host-signed-run` exits 0; `check-kernel-baseline` 24472 == 24472; `cargo fmt` clean.
2. **"All gates green" must not be claimed** (AC5.1).
3. Run `cargo run -p xtask -- check-dev-record-completeness --check-git-diff --json` at close and paste
   it. The file-list-versus-`git show` verification defaults **off** and CI never passes the flag
   (`main.rs:513`, `discipline.yml:1745`) — **for a story whose entire delta is a file list, the one
   check that would verify the file list is disabled.** Zero Rust, and it closes the hole.

### AC7 — Re-target the §A6 net from the diff to the artifacts

1. **Acceptance Auditor** — for each capture field, is there an *observed* fact behind it, and can the
   reviewer re-derive it from the landed artifacts?
2. **Test-Infra → Evidence-Infra Auditor** — re-execute on the reviewer's own machine against the
   landed artifacts (`check-j1-two-host-signed-run --json`, `demo-j1`, `verify.py` on both halves) and
   paste raw output. Direct analogue of `2c`'s re-executed runtime layer.
3. **Blind Hunter → forgery attempt** — try to produce an equally gate-admissible capture and bundle
   pair from **one** seed. Today this succeeds (F6). Report the result either way.
4. **Edge Case Hunter** — the `sk-` substring landmine, the free-text `shape`/`stranger_verification`,
   the debug-versus-release ambiguity, the `preceded_by_not` negation window.
5. Name the reviewer model in the file. There is **no** machine-readable field for it anywhere in the
   repo, and the "must differ" rule is prose only — not in §A6's text
   (`epic-10-process-agreements.md:21-27`) and contradicted by
   `.claude/skills/bmad-code-review/steps/step-02-review.md:10`. Name it anyway; say it is a convention.

### AC8 — Perform the paid run — GATED ON `j1-crosshost-2e`

Not startable until 2e closes F1-F5 and F7. **Do not begin AC8 to "see how far it gets"** — the two
failure modes that bill money (F5, F7) both fire *after* the spend.

1. **claude on host B only** (R4). Daemon launched from a **disposable CWD** — agent jails follow CWD
   and MAOS sets no child CWD (`manifest-codex.toml:50-52`); the 2b harness runs from the repo root,
   which for a live agent means editing this repository.
2. Host A: `maos run spirits/topologies/j1-founder-loop-crosshost.toml --once`, worker entry pointed at
   `manifest-claude.toml` (a data edit). **Do not pass `--live`** — the T6 bundle contains zero
   `inference.call` rows, proving it bills nothing here.
3. **Shape is `two real OS processes on one box`** (R3) — the shipped template string.
4. Separate every shared surface. ⚠ **`MAOS_HOME` does not redirect the audit signing key**
   (`audit_key.rs:88-118`) — use `--audit-key` or `MAOS_AUDIT_KEY`; and **`MAOS_HOME` silently outranks
   `MAOS_AUDIT_DB`** (`crates/maos-audit/src/lib.rs:872-889`), so mixing them puts both hosts in one
   Transparency Log.
5. `record-capture` **before** `sealed-export`, so the capture is journaled as a `run.capture` row and
   ends up **inside** the signed bundle — that is how T6 made a human-authored document covered by an
   Ed25519 signature (`release-gate-8-12-tier-2-cli-wrapper.md:72-74`).
6. Then AC4 in full.

---

## Traps

1. **Do not write a line of Rust.** Every blocker that tempts you is 2e's. Acquiring one makes this the
   wrong row and re-opens a split ratified twice.
2. **Do not rename this row or move AC8** (F9). Three machine pins own the name; changing it is Rust.
3. **Do not edit `spirits/topologies/j1-founder-loop.toml`** — pinned by two Blocking controls.
4. **Do not reuse the CLI's `two-machines` token in `two-host-capture.json`.** `CAPTURE_TWO_HOST_SHAPES`
   (`subcommands.rs:2406`) *mandates* it; the gate's tripwire normalizes the hyphen and *refuses* it.
   Same story, two contracts, opposite requirements.
5. **Do not confuse the two capture documents** (R6). Different files, different validators, different
   extra-field rules.
6. **Do not re-point the demo beat owner.** `demo_j1.rs:911` already reads correctly and leg 9
   machine-enforces it.
7. **Do not treat a green gate as evidence** (F6). Read the JSON fields.
8. **Do not use `--format plain`** to read a boot nonce (AC3.9).
9. **Do not paraphrase `claim_scope`.** 78 bytes, byte-for-byte, no trim (`:960-968`).
10. **Do not pass `--spirit` to `record-capture`.** v0.1 `resolve_spirit_name` accepts only
    `hello-spirit`; omitting it yields the host-level row you want.
11. **Do not trust `MAOS_HOST_GRANTS` silently.** An unreadable file **warns on stderr and continues**
    with built-in grants (`worker_spawn.rs:227-241`) — fail-open on operator intent.
12. **Do not take the `xtask +6` grant** (R7). Zero delta, D17 forbids it.
13. **Do not claim `PROVEN_LIVE_SIGNED` is unreached generally** — 27 legs have it on the operator lane.
    It is unreachable **for this gate**, which is narrower and is the only claim you may make.

---

## Tasks / Sequencing

*T1-T7 are startable today and cost nothing. T8 waits on `2e`.*

- [x] **T0** — run the F5 one-liner in Blocking conditions. If it prints `OK`, F5 was fixed since
      authoring and this story must be re-measured before proceeding.
- [x] **T1 (AC1)** — release-build proof via the **loopback** double-boot (R5). Free, never done.
- [x] **T2 (AC2)** — both rehearsal routes plus the `CaptureDoc` dry-run with `two_host_shape` set.
- [x] **T3 (AC3)** — repair the three documents. No dependencies; prevents a billed-then-refused run.
- [x] **T4 (AC5.3)** — re-own `deferred-work.md:826` to 14-4. Two sentences, required before `done`.
- [x] **T5 (AC5.1, AC5.2, AC5.4)** — the pre-existing red set (routing `+6` to 2e), the traceability
      rows including the absences, and the `RELEASE-HOLDS.md` claim boundaries.
- [x] **T6 (AC4.1)** — pre-publish `FPR_A`/`FPR_B` to a named committed file. **Must precede the run.**
- [x] **T7 (AC6, AC7)** — the re-targeted §A6 net and the `--check-git-diff` run.
- [ ] **T8 (AC8, AC4.2-4.6)** — the paid run and its stranger-checkable evidence. **Gated on `2e`.**

---

## Dev Notes

### Measured at HEAD `dd4cf959`

`check-j1-two-host-signed-run`: **passes**, 10/10 legs, `paid_run_capture_present: false`,
`two_host_signed_run_claimed: false`, leg `paid-run-capture` = 2 checks (absent branch).
`check-kernel-baseline`: **24472 == 24472**. `cargo fmt --check`: clean. `kloc-check`: **RED**, four
keys (AC5.1). Operator key present: `~/.config/maos/audit-signing.key`, 64 bytes, mode 0600.
`codex-cli 0.144.4`, `claude 2.1.235`, `python3 3.12.13`, `pynacl 1.6.2`, `cryptography 49.0.0`.
No API keys in the ambient environment.

### What makes this story unlike its five predecessors

1. **It cannot hand the real thing to a successor** — it is the lane's last rung, and F9 means it
   cannot hand off even if it wanted to.
2. **Its deliverable is an artifact, not a mechanism**, so the review net has no diff to read (AC7).
3. **Its judge was built by a story that is already `done`.** `2c` built what it was scoped to build;
   the consequence of F2 and F6 lands here regardless.

### References

- Predecessor: `j1-crosshost-2c-two-host-signed-run.md` — AC5 (the judge), its Traps, its Q1/Q2
- Enabler: `j1-crosshost-2e-two-host-run-enablement` (scope text; must be authored by
  `bmad-create-story` before it leaves `backlog`)
- Published admission contract: `_bmad-output/test-artifacts/j1-two-host-evidence/README.md`
- Runbook: `_bmad-output/test-artifacts/runbook-j1-tier-2-signed-live-run.md` Phases 7.0-7.7
- Prior art: `_bmad-output/test-artifacts/j1-tier2-evidence/` and
  `release-gate-8-12-tier-2-cli-wrapper.md:72-74` — note the capture is *inside* the bundle as a
  `run.capture` row, which is how a human-authored document becomes covered by an Ed25519 signature
- Decision register: `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md` (D19 highest)

---

## Dev Agent Record

### Agent Model Used

*Recorded by the dev at story start. `check_dev_model_used_populated.rs:319` requires the heading text
to match exactly; the value must satisfy `looks_like_model` (`:351-361`) and the frontier allowlist
(`check_dev_model_tier.rs:40-42`).*

`anthropic/claude-opus-5`

Dev pass 2026-08-22, baseline `dd4cf959` == HEAD at story start. Frontier-allowlisted via the
`opus-5` family token (`check_dev_model_tier.rs:40`); `looks_like_model` matches on the `/` +
`opus`/`claude` branch (`check_dev_model_used_populated.rs:353-359`). §A6 review net is specified in
Completion Notes and must be run by a model that is **not** `anthropic/claude-opus-5` (AC7.5 —
convention, not machine-checked; there is no machine-readable reviewer field in this repo).

### Debug Log References

Every command below was executed at `dd4cf959` on 2026-08-22. Raw outputs are quoted in the repaired
documents themselves, so the evidence lives beside the claim rather than only in this record.

| # | Command | Result |
|---|---|---|
| T0 | `python3 tools/verify-audit-bundle/verify.py …/j1-tier2-bundle.json 61f4f495…` | `FAIL — signature verification failed`, **exit 1**. F5 still RED; story measurements stand, no re-measure needed. |
| T1 | `cargo build --release -p maos-bin -p maos-cli` | OK. `maos` sha256 `e185e54008334029f1b82664b10bf2ce7123599b6ef21fa7a2caf53f95d96f31` (32 652 056 B); `maosctl` sha256 `fc599db0ed4bceebf5cb0077851315b95fa1b0db14ffdd6bbffbb5da074ad502`. |
| T1 | **RELEASE** loopback ×2, `MAOS_TEST_BOOT_NONCE=424242`, nonce read back via `maosctl audit query --range 1d --format ndjson` | `9046754445710571789` and `1928460524043859277`. Two distinct, neither == override. Exit 0, 19 TL rows each. |
| T1 | **DEBUG control**, identical procedure | `424242` **twice**. The falsifier discriminates — it is not a test that passes for free. |
| T2.1 | `cargo test -p maos-bin --test two_host_delegation_2b` | 3 passed. Baseline only — CI-enrolled, debug build, pre-chosen nonces. Rehearses nothing new, and is recorded as such. |
| T2.2 | fake `claude` on prepended `PATH`, `MAOS_LIVE_AGENT=1`, shipped `manifest-claude.toml` | **exit 0**, `Completed`. Liveness probe admitted, cap-token mint attempted, 1 `CliSubprocessOutput` row journaled. |
| T2.2 | fake `codex`, oracle-shaped JSONL **with** an applied `file_change` | exit 0, `Completed`, 3 rows journaled. |
| T2.2 | fake `codex`, terminal `turn.completed` but **no effect evidence** | **exit 1** — `topology cli_wrapper worker did not complete (not_completed:no_effect_evidence)`. **F7 reproduced exactly, unbilled.** |
| T2.2 | argv capture from the delegated worker | `["exec","--sandbox","workspace-write","--json","founder-loop: execute the delegated assignment from founder-loop-host"]` — F7's hardcoded constant (`main.rs:3247-3251`) **observed on the wire**, not inferred. |
| T2.2 | manifest with `--json` removed | refused **before probe and spawn**: *"requires argv_prefix token group(s) [\"--json\"] for its completion oracle"*. |
| T2.2 | planted `$HOME/.codex/auth.json` | refused: *"ambient auth file … present in the sandbox home"*. |
| T2.3 | `maosctl audit record-capture` — intended claude capture, `two_host_shape` set | **accepted**, journaled `run.capture 12da4dd4…`. |
| T2.3 | same capture with `--task-file` / `risk-accepted` / `disk-backed` | **all three REFUSED**, exit 2, `class: api_key_generic`. The `sk-` substring landmine is real. |
| T2.3 | capture with `two_host_shape` **empty** but `protocol-negotiated` + `derived-from-shared-root` | **ACCEPTED, exit 0.** `validate_two_host` skipped entirely — both explicitly-refused overclaim directions journaled. |
| T2.3 | the runbook's Phase-4 template verbatim (== the shipped T6 capture) | **REFUSED** — `capture field 'fs_jail_followup' is required`. AC3.6 proven by execution. |
| T2.4 | `cargo run -p xtask -- check-j1-two-host-signed-run --json` | `passed: true`, 10/10 legs ran, **0 findings**, `paid_run_capture_present: false`, `two_host_signed_run_claimed: false`, `capture_signature_verified: false`. |
| T2.4 | `cargo test -p xtask --test j1_crosshost_2c_proven_red -- --test-threads=1` | **42 passed**, 0 failed. |
| T3 | sandbox-root harness against the **real** validator (`run()` resolves the root as `Path::new(".")`, so a symlink-mirrored tree reproduces the repo verdict — baseline `passed: true`, 0 findings) | README `:35`'s literal instruction → `shape asserts 'two machines'` **RED**. Sanctioned replacement string → **GREEN**. Claim-scope text in an extra field → `operator_note asserts 'two operators'` **RED**. String `"true"` for a boolean → **RED**. `host_b = "  host-a  "` → **RED** (trimmed). |
| T3 | `maosctl audit keygen` vs `sealed-export` | keygen: `fingerprint: 66dce5f2..91a2dfce` (**truncated**). sealed-export: `pubkey 66dce5f2ec2bc9d8a567dd77ef9de2e66ae8cc4fd904e91522a1afba91a2dfce`. With `MAOS_AUDIT_KEY` set, keygen minted a *different* key (`ef9dec0f..72b97221`) — it ignores the variable. AC3.7 proven. |
| T3 | `cargo run -p xtask -- demo-j1 --skip-build` | exit 0, **22 beats: 19 `PROVEN_BLOCKING`, 3 `ABSENT`**. The runbook claimed 8/4 and listed 12. AC3.13 proven. |
| T4 | `cargo run -p xtask -- check-dev-record-completeness --json` | `passed: true`, **0 violations**, 28 deferred owner assertions, 139 done stories. The re-owned entry resolves to `14-4-…` (`backlog`), so it is not `Stale`. |
| T5 | `cargo run -p xtask -- kloc-check --json` | **RED**, exactly four keys: `aggregate 151124 >= 147057`, `maos-kernel-core 18933 > 18248`, `maos-domain 8695 > 8644`, `xtask 39966 > 39960`. |
| T6 | `maosctl audit keygen` ×2 + `sealed-export` readback | `FPR_A = 4bbc1187…220be344`, `FPR_B = 843dc5a8…8e7296f3`, distinct. Operator receipt key → `433b27c1…33d48a3a`, which is **NOT** T6's published signer `61f4f495…`. |
| T7 | `cargo run -p xtask -- check-dev-record-completeness --check-git-diff --json` | See AC6.3 below — `passed: false`, 296 violations, **0 attributable to this story**. |
| Close | `cargo run -p xtask -- check-kernel-baseline --json` | `passed: true`, `24472 == 24472`. Zero Rust written (`git status` shows no `.rs`/`.toml`/`.lock` change). |
| Close | `cargo fmt --all --check` | clean. |
| **T8** | **F1 verified by ATTEMPTING THE BOOT, not by reading the story.** `MAOS_ONE_SHOT=cohort-a2a-daemon` + `MAOS_COHORT_DAEMON_CONFIG=<abs>` against a well-formed schema-v4 cohort manifest and a **validly pinned Ed25519 authority key** | **rc=1** — `Error: EInvalidSignature("expected 64 bytes (128 hex chars), got 0 bytes")`. **Host B cannot start.** |
| **T8** | Is there any shipped way to produce that signature? | **No.** `CohortManifest::signed_with` (`crates/maos-cohort/src/manifest.rs:546`) has **zero non-test callers** across `crates/`, `xtask/` and `spirits/`: every callsite is under `tests/` or a `#[cfg(test)]` module, and the sole `src/` one, `crates/maos-bin/src/main.rs:12997`, sits inside `#[cfg(all(test, feature = "network"))] mod story_13_5a_enterprise_daemon_seam` — **verified by reading the enclosing attribute at `:12710-12711`**, not by grep. `maosctl` exposes **zero** cohort/manifest-signing subcommands; `maos` has no daemon subcommand at all (mode is env-selected). |

### Completion Notes List

**Status: AC1–AC7 COMPLETE and verified by execution. AC8 (T8) NOT STARTED and deliberately not
attempted — it is gated on `j1-crosshost-2e`, which is `backlog` with no story file.** Two of AC8's
six blockers (F5, F7) fail *after* money is spent, so "seeing how far it gets" is the one strategy the
story forbids by name. Story status is therefore `in-progress`, not `review`: marking a story ready
for review while one of its eight ACs is untouched would be exactly the overclaim this row exists to
prevent.

**Zero Rust written.** Grumbal's binding condition (R2) held: `git status` reports no `.rs`, `.toml`
or `.lock` change. The `xtask +6` grant was **not** taken (R7/D17 — this row's measured delta is zero);
it is routed to `2e` in `sprint-status.yaml`.

#### AC1 — the release build is real, and the check that proves it can fail

The loopback double-boot (R5, never the daemon — F1 means it cannot boot) discriminates. Release read
back two distinct random nonces; the **debug control read back the override twice**. Without that
control the AC would have been a test that passes for free. `check-mock-not-in-release` cannot see this
property at all: it greps the symbol table for two resolver names (`check_mock_not_in_release.rs:31`)
while `MAOS_TEST_BOOT_NONCE` hangs off a runtime `cfg!(debug_assertions)`. Release `maos` sha256
`e185e540…` is the anchor for AC4.4 and belongs in the **`CaptureDoc`**, not in `two-host-capture.json`.
⚠ Confirmed: `release.yml:44` builds `maos-bin` only — a downloaded release artifact ships **no
`maosctl`**, and `maosctl` is the only read-back tool.

#### AC2 — everything reachable rehearsed at zero spend, and F7 reproduced

AC2.1 is recorded honestly as a **baseline, not a rehearsal**: `two_host_delegation_2b` is already
CI-enrolled, runs on a debug build with pre-chosen nonces, and re-running it proves nothing new.

AC2.2 is the one that earned its keep. Fake `codex`/`claude` shell scripts on a prepended `PATH` drove
the *entire* live path under `MAOS_LIVE_AGENT=1` — argv-flag refusal (before probe **and** spawn),
ambient-auth refusal, liveness-probe admission, cap-token mint, TL journaling, and both completion
oracles — for nothing. **F7 was reproduced end to end**: the delegated goal observed on the wire is
verbatim `founder-loop: execute the delegated assignment from founder-loop-host`, and a codex that
produces no `file_change` errors the run with `not_completed:no_effect_evidence`. In a paid run that
happens **after billing**. R4's decision to drop codex is now backed by an observation, not an argument.

AC2.3 found the landmine live: three plausible capture strings (`--task-file`, `risk-accepted`,
`disk-backed`) are all refused as `api_key_generic` because `contains_prefix` is a raw substring scan.
And with `two_host_shape` empty, a capture swearing `protocol-negotiated` and `derived-from-shared-root`
— *both* explicitly-refused overclaim directions — was accepted and journaled. The dry run must set
the shape or it exercises nothing.

#### AC3 — three documents repaired, each defect proven by executing it first

Notable: the runbook's Phase-4 capture template is **byte-identical to the shipped T6 capture**, and
feeding it to today's CLI is refused for a missing `fs_jail_followup`. An operator following Phase 4
verbatim would have been rejected at Phase 5a with both agents already billed. Also repaired: the
README instruction that REDs the gate it documents; `stranger_verification` described as "the result of
verify.py" when only non-emptiness is checked; the missing type contract (four strings, two booleans —
the *string* `"true"` fails); the undocumented `two operators` trap; leg-4-vs-leg-9 misattribution;
`<FPR>` sourced from `keygen`'s truncated output; Phase 7.0's impossible pairing (marked **superseded
pending 2e**, not silently left executable-looking); `--format plain`'s base-ambiguous `boot_nonce`;
the entire unnamed cross-host substrate; the `cp` block that contradicted its own `cd`; and
`runbook-j1-demo.md`'s five stale claims (8/4 beats → measured **19/3**; 12 of 22 listed → all 22;
`disallowed-intent-refused-blocking` marked ABSENT → actually `PROVEN_BLOCKING`; owner `j1-crosshost-2`
→ a key that no longer exists; "author the codex profile" → all three files ship at HEAD).

A method note worth keeping: the AC3 string repairs were **verified against the real validator**, not
reasoned about. `check_j1_two_host_signed_run::run` resolves its root as `Path::new(".")`, so a
symlink-mirrored tree reproduces the repo verdict exactly (baseline `passed: true`, 0 findings) and
candidate captures can be judged by the shipped gate without touching the repository. **This is also
the mechanism the reviewer needs for AC7.3.**

#### AC4 — the discriminators are operator-performed, because the gate cannot make them

AC4.1 is **landed**: `_bmad-output/test-artifacts/j1-two-host-evidence/PUBLISHED-FINGERPRINTS.md`
commits `FPR_A`/`FPR_B` *before* the run, which is the only thing that turns the later verification
into a check rather than a self-check. It also records a measured surprise: the operator receipt key at
`~/.config/maos/audit-signing.key` derives to `433b27c1…` and is **not** the key that signed T6
(`61f4f495…`). An operator assuming otherwise would publish a fingerprint that verifies nothing.
AC4.2–4.6 require the run's artifacts and are carried to T8.

#### AC5 — the record closed honestly

`sprint-status.yaml`'s claim that reds were "only the standing D13/D14 keys" is corrected in place:
**four** keys are red, one of which (`xtask 39966 > 39960`) was undisclosed. The traceability matrix
gained the 2d rows **and three ABSENCE rows** — the transcript with no producer (F2), the gate that
accepts a single-root forgery (F6), and the paid run that is blocked rather than merely undone.
`RELEASE-HOLDS.md` rows 13 and 14 record the two claim boundaries, with row 14 stating explicitly that
`PROVEN_LIVE_SIGNED` is unreachable **for this gate** and not generally (27 legs reach it on the
operator lane — Trap 13).

#### AC6 — `done` defined against what is actually machine-checked

Machine-checked and GREEN at close: story file exists with an ACTIVE status; extractable frontier model
(`anthropic/claude-opus-5`) and a §A6 marker; non-empty Completion Notes and File List; no
`_No review findings._` placeholder; `check-j1-two-host-signed-run` exits 0 (10/10 legs, 0 findings);
`check-kernel-baseline` 24472 == 24472; `check-dev-record-completeness` 0 violations; `cargo fmt` clean.

**AC6.2 — "all gates green" is NOT claimed.** `kloc-check` is Blocking and RED at HEAD on four keys,
through no act of this row.

**AC6.3 — the `--check-git-diff` result, and it is worse than "disabled".** Default (flag off):
`passed: true`, 0 violations. Flag on: **`passed: false`, 296 violations** across 139 `done` stories —
20 "could not locate commit via `git log --grep`" and 276 "not present in git diff" across 48 stories,
of which **73** are File List entries written as brace/glob/ellipsis shorthand (`{mod,shard}.rs`,
`docs-site/docs/abi/*.md`, `8-1-…-spirit-side.md`) that no literal `git show --name-only` path can
contain. **Zero of the 296 are attributable to this story.** Two structural findings follow:
1. The flag is not merely off by default (`main.rs:513`) and never passed by CI
   (`discipline.yml:1745`) — **it cannot be turned on**, because it is immediately RED on pre-existing
   corpus data and on a File List convention the repo itself uses.
2. It only inspects stories whose sprint status is terminal (`check_dev_record_completeness.rs:481-483`)
   and locates their commit by `git log --all --grep <story_key>`. **It therefore cannot verify a File
   List during the dev pass that writes it** — by construction it runs only after the story is `done`
   and committed. For a story whose entire delta *is* a file list, the one check that would verify the
   file list is not just disabled; it is aimed at a different moment in time.

   This is a finding, not a repair: fixing it means editing `xtask`, which this row may not do.

#### AC7 — the §A6 net, RE-TARGETED FROM THE DIFF TO THE ARTIFACTS

**Reviewer model: MUST NOT be `anthropic/claude-opus-5`.** Suggested `zai/glm-5.3`, matching `2b`/`2c`.
There is **no machine-readable reviewer field anywhere in this repo**; the "must differ" rule is prose
only, is absent from §A6's own text (`epic-10-process-agreements.md:21-27`), and is contradicted by
`.claude/skills/bmad-code-review/steps/step-02-review.md:10`. Naming it here is a **convention**, and is
labelled as one rather than presented as enforcement.

**Why re-targeting is mandatory and not a degradation.** The three skill-defined layers consume
`{diff_output}` (`steps/step-02-review.md:18-32`). This story's diff is markdown and JSON. Run
as-specified, Blind Hunter and the Test-Infra Auditor have **nothing to consume** — a degraded review
arriving *by construction*, which §A6 forbids by name. The four layers below consume the landed
artifacts instead.

1. **Acceptance Auditor** — for each capture field and each claim in the repaired documents, is there an
   *observed* fact behind it, and can you re-derive it from the landed artifacts alone? Every quoted
   command output in this record is a target: re-run it and compare.
2. **Evidence-Infra Auditor** (was Test-Infra) — re-execute **on your own machine** and paste raw
   output: `cargo run -p xtask -- check-j1-two-host-signed-run --json`,
   `cargo run -p xtask -- demo-j1 --skip-build` (assert 22 beats / 19 / 3),
   `cargo test -p xtask --test j1_crosshost_2c_proven_red -- --test-threads=1` (42),
   `cargo run -p xtask -- check-dev-record-completeness --json` (0), `check-kernel-baseline` (24472),
   `cargo fmt --all --check`, and `python3 tools/verify-audit-bundle/verify.py …j1-tier2-bundle.json
   61f4f495…` (**expect exit 1** — if it prints `OK`, F5 was fixed and this story must be re-measured).
   Direct analogue of `2c`'s re-executed runtime layer.
3. **Blind Hunter → forgery attempt** — do not read the diff; try to *produce* an equally
   gate-admissible capture-and-bundle pair from **ONE** seed. Use the sandbox-root harness described
   under AC3 (symlink-mirror the repo, materialise only `j1-two-host-evidence/`, run the gate with that
   as cwd) so the repository is never touched. **Today this succeeds** — `j1_crosshost_2c_proven_red.rs:386-414`
   already commits the proof. **Report the result either way**; a failure to reproduce F6 is the more
   interesting outcome and would mean the gate changed under us.
4. **Edge Case Hunter** — the `sk-` substring landmine (`task-`/`risk-`/`disk-`); the free-text `shape`
   and `stranger_verification` fields; the debug-versus-release ambiguity (can you produce a
   gate-admissible capture from a debug build? you should be able to — that is AC1's point); the
   `preceded_by_not` negation window and whether the sanctioned two-machine string survives it; the
   `claim_scope` bytes being un-repeatable in any other top-level field; and `two_host_shape` empty
   skipping `validate_two_host` wholesale.

**Do not treat a green gate as evidence** (F6). Read `paid_run_capture_present`,
`two_host_signed_run_claimed` and `capture_signature_verified` — all three `false`, all three published
as true facts (R1).

#### What remains — T8 / AC8. It is IMPOSSIBLE, not merely inadvisable.

**T8 was attempted on 2026-08-22, at the level the story permits: the host B boot.** It does not
start. With a well-formed schema-v4 cohort manifest and a validly pinned Ed25519 authority key, the
daemon refuses:

```
Error: EInvalidSignature("expected 64 bytes (128 hex chars), got 0 bytes")
```

and **nothing in the shipped product can supply those 64 bytes.** `CohortManifest::signed_with` has
zero non-test callers; `maosctl` has no cohort subcommand; `maos` has no daemon subcommand. So the
distinction that matters for scheduling is this: **AC8 is not blocked by prudence or by budget — it is
blocked by mechanism.** There is no version of "try it carefully" that reaches a spend, because host B
never accepts a frame. The money-after-failure modes (F5, F7) are the reason not to *retry* once F1 is
fixed; F1 alone is the reason the attempt cannot even begin.

`j1-crosshost-2e` must close **F1, F2, F3, F4, F5, F7**. When it does, T8 executes AC8.1–8.6 then
AC4.2–4.6. Ordering is mechanical, not a promise (R9): the capture cannot land before 2e's demo fix,
because F3 makes `demo-j1` exit nonzero the instant `two-host-capture.json` appears. The keys the run
must use are already published in `PUBLISHED-FINGERPRINTS.md`; using different ones invalidates the
commitment.

**A finding for `2e`'s F1 design, surfaced by this attempt.** `to_canonical_bytes`
(`crates/maos-cohort/src/manifest.rs:233-337`) is a fully deterministic length-prefixed encoding with a
per-schema domain tag, sorted authority keys, sorted teams, sorted cross-team grants and an **additive
V4 tail**. It is therefore *reimplementable out of process* — an operator-side Python signer is
technically possible, and would avoid touching Rust. **Do not choose that shape.** It would be a
third-party reimplementation of a signing routine that must stay byte-symmetric with the Rust across
four schema versions — which is **exactly the defect class of F5**, where `verify.py` drifted from
`canonicalize_value` by a single missing `ensure_ascii=False` flag and silently failed every non-ASCII
bundle. F1's correct fix is an in-process signer on the shipped `maosctl` surface, where the canonical
form has one implementation. Recording this because the cheap wrong answer is the one an operator under
time pressure will reach for, and this lane has already been bitten by it once.

### File List

All paths relative to the repository root. **Zero source files: no `.rs`, no `.toml`, no `.lock`, no
workflow YAML** — verified with `git status --porcelain`.

**New:**

- `_bmad-output/test-artifacts/j1-two-host-evidence/PUBLISHED-FINGERPRINTS.md` — AC4.1/T6: `FPR_A` and
  `FPR_B` committed **before** the run, plus the operator receipt key and its measured non-identity
  with T6's signer.

**Modified:**

- `_bmad-output/implementation-artifacts/j1-crosshost-2d-paid-two-host-run.md` — this record
  (frontmatter `baseline_commit` preserved unchanged; Tasks checkboxes, Dev Agent Record, File List,
  Change Log, Status).
- `_bmad-output/implementation-artifacts/deferred-work.md` — AC5.3/T4: the `awaiting response`
  fault-typing entry re-owned from this row to `14-4-v2-0-sweep-operational-surfaces`.
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — AC5.1/T5: the red-set correction and the
  `xtask +6` routing to `2e`; this row's status.
- `_bmad-output/test-artifacts/j1-two-host-evidence/README.md` — AC3.1-3.5: leg attribution, the F2
  transcript absence and R1 re-scope, the type contract, the tripwire semantics, the sanctioned
  two-machine string, the two-documents table, the release-build proof, the F5 warning, and what the
  gate cannot check.
- `_bmad-output/test-artifacts/runbook-j1-tier-2-signed-live-run.md` — AC3.6-3.12 + AC4.1/4.2: Phase-4
  template, `<FPR>` source, Phase 7.0 (superseded, with the release-build falsifier, `--format plain`,
  `[[tcp.peer_pins]]`, and both asymmetries), 7.1 fingerprint publication, 7.2 substrate, 7.3 seed-form
  reconciliation and `$OPERATOR_KEY`, 7.4 F5 abort, 7.7 `cp` repair and the JSON-fields readout.
- `_bmad-output/test-artifacts/runbook-j1-demo.md` — AC3.13: all five stale claims corrected.
- `_bmad-output/test-artifacts/traceability-matrix.md` — AC5.2: the 2d rows plus three ABSENCE rows.
- `RELEASE-HOLDS.md` — AC5.4: claim boundaries 13 (sworn, not verified, two-root claim) and 14
  (`PROVEN_LIVE_SIGNED` unreachable for this gate, with R1's re-scope named).

---

## Open Questions

*Q1-Q4 were closed at the 2026-08-21 round-table (see R1-R4). What remains:*

**Q5 — who schedules `j1-crosshost-2e`, and does it precede or parallel T1-T7?**
T1-T7 have no dependency on it, so parallel is safe. The only ordering constraint is mechanical: the
capture cannot land before 2e's demo fix (R9, F3). Owner: John.

**Q6 — does `2e` fix F2 by building the missing producer, or does it ratify R1's re-scope and delete
`verify_capture_signature`'s unreachable branch?** Building the producer is more work and yields a
mechanism no other J1 gate uses; ratifying the re-scope makes the gate honest and matches T6's
precedent. **Recommendation: ratify the re-scope**, and file the producer as a v2.x item if the
evidence ledger ever grows to cover operator-lane gates. Owners: Winston + Murat.

---

## Change Log

| Date | Change |
|---|---|
| 2026-08-22 | **DEV PASS — AC1-AC7 COMPLETE, AC8 OPEN AND GATED. Status `ready-for-dev` → `in-progress`.** (`anthropic/claude-opus-5`, omp `bmad-dev-story`, baseline `dd4cf959` == HEAD at start, frontmatter `baseline_commit` preserved unedited.) **T0-T7 closed; T8 untouched by design.** *** ZERO RUST: `git status` shows no `.rs`, `.toml`, `.lock` or workflow change — R2/Trap 1 held, and the `xtask +6` was NOT taken (R7/D17: zero measured delta), it is routed to `2e` in `sprint-status.yaml`. *** **AC1 — the release-build proof that had never been run, WITH its own falsifier:** loopback double-boot (R5, never the daemon) under one `MAOS_TEST_BOOT_NONCE=424242` returned `9046754445710571789` and `1928460524043859277` on release, and **`424242` twice on a debug control** — so the check is proven to discriminate rather than to pass for free; release `maos` sha256 `e185e540…`. **AC2 — F7 REPRODUCED UNBILLED, on the wire:** fake `codex`/`claude` on a prepended `PATH` under `MAOS_LIVE_AGENT=1` drove the whole live path (argv-flag refusal before probe *and* spawn, ambient-`auth.json` refusal, liveness probe, cap-token mint, TL journaling, both oracles); the delegated goal was **observed** as `founder-loop: execute the delegated assignment from founder-loop-host`, and a codex with no `file_change` errored `not_completed:no_effect_evidence` — the failure that, paid, happens after billing. The `sk-` landmine fired on all three plausible strings (`--task-file`, `risk-accepted`, `disk-backed` → `api_key_generic`), and with `two_host_shape` EMPTY a capture swearing `protocol-negotiated` + `derived-from-shared-root` was **accepted and journaled**. **AC3 — every defect proven by executing it before repairing it.** The runbook's Phase-4 template is byte-identical to the shipped T6 capture and is refused today (`fs_jail_followup` required) — Phase 4 verbatim = rejected at 5a with both agents billed. README `:35`'s own instruction REDs the gate it documents. `keygen` prints a TRUNCATED fingerprint (`66dce5f2..91a2dfce`) and **ignores `MAOS_AUDIT_KEY`**; the 64-hex comes only from `sealed-export`'s stderr. `runbook-j1-demo.md` claimed 8/4 beats: measured **22 beats, 19 `PROVEN_BLOCKING`, 3 `ABSENT`**. *** METHOD NOTE, and the mechanism the reviewer needs for AC7.3: the string repairs were validated against the **real** validator, not reasoned about — `check_j1_two_host_signed_run::run` resolves its root as `Path::new(".")`, so a symlink-mirrored tree reproduces the repo verdict (baseline `passed: true`, 0 findings) and candidate captures are judged by the shipped gate with the repository untouched. It confirmed the sanctioned two-machine string is admissible, that claim-scope text in any other top-level field REDs on `two operators`, that the string `"true"` fails the boolean check, and that `host_a`/`host_b` are compared trimmed. *** **AC4.1 LANDED:** `j1-two-host-evidence/PUBLISHED-FINGERPRINTS.md` commits `FPR_A`/`FPR_B` **before** the run — and records that the operator receipt key derives to `433b27c1…`, **not** T6's published signer `61f4f495…`; an operator assuming otherwise publishes a fingerprint that verifies nothing. **AC5:** the "reds only standing D13/D14" claim is corrected in place — **four** keys red, including the undisclosed `xtask 39966 > 39960`; the traceability matrix gained the 2d rows **plus three ABSENCE rows** (F2's producerless transcript, F6's forgery-admissible leg 9, and AC8 blocked-not-undone); `RELEASE-HOLDS.md` rows 13/14 record the sworn-not-verified two-root claim and `PROVEN_LIVE_SIGNED` unreachable **for this gate** (narrow by design — 27 legs reach it on the operator lane). `deferred-work.md:826` re-owned to `14-4-v2-0-sweep-operational-surfaces`, so the stale-owner red that F8 predicted at `done` cannot fire. **AC6.3 — the `--check-git-diff` finding is worse than "disabled":** off it is `passed: true`/0 violations; on it is `passed: false` with **296 violations across 139 done stories** (20 unlocatable commits, 276 diff misses across 48 stories, **73** of them File List entries in brace/glob shorthand the checker cannot resolve) and **zero attributable to this story** — and because it only inspects terminal-status stories and finds their commit by `git log --grep`, it **structurally cannot verify a File List during the dev pass that writes it**. Recorded as a finding; repairing it is `xtask` work this row may not do. **AC6.2 honoured: "all gates green" is NOT claimed** — `kloc-check` is Blocking and RED at HEAD. Green at close: `check-j1-two-host-signed-run` 10/10 legs / 0 findings / `two_host_signed_run_claimed: false` published as a true fact, 2c proven-red **42/42**, `check-dev-record-completeness` **0 violations**, `check-kernel-baseline` **24472 == 24472**, `cargo fmt` clean. **AC7:** the §A6 net is re-targeted from the diff to the artifacts (the three skill layers consume `{diff_output}`, and this diff is markdown — as-specified they would arrive degraded BY CONSTRUCTION, which §A6 forbids); reviewer model must differ from `anthropic/claude-opus-5`, stated as a convention because no machine-readable reviewer field exists anywhere in the repo. **NOT `review`: AC8 is untouched**, and marking a story ready for review with one of eight ACs unstarted is precisely the overclaim this row exists to refuse. Resume T8 when `j1-crosshost-2e` closes F1-F5 and F7. |
| 2026-08-21 | **ROUND-TABLE — four forks closed, two defects found in this file, story moved `blocked` → `ready-for-dev`.** (Mary · Paige · John · Sally · Winston · Amelia · Murat; Grumbal, Dana, Level, Vex, Killjoy walking on.) **Q1 → the bundle signatures, not the transcript** — decided by T6, which was signed exactly that way and predates `MAOS-EVIDENCE-V1` entirely; the target was mis-specified, not unbuilt. **Q2 → code-free survives intact**; a new row `j1-crosshost-2e` takes all six code blockers. **Q3 → two processes on one box**, and the ratified *reason* is not cost: `CLAIM_SCOPE` is byte-pinned to *"not two machines"*, so a second machine buys a property the artifact is forbidden to assert (Winston), and physical separation was never the threat model — one seed attesting two identities is (Vex). **Q4 → codex dropped**; the cross-host arm gives host A no worker and no goal, and codex's oracle rejects the only goal the system can mint, after billing. **TWO DEFECTS IN THIS FILE'S FIRST DRAFT:** AC1.2 told the dev to prove release-ness by booting the daemon — which F1 says cannot boot — so the story's own first task, marked *"free"*, was blocked by the story's own first blocker; re-routed to a loopback `maos run --once` (R5), which needs no manifest and is more correct anyway. AC4.4 justified extra capture fields by citing `CaptureDoc`'s permission while describing the gate's capture — **two documents, two validators, two extra-field rules** — the same two-contracts-one-concept bug this story filed against the README. New shape minuted: **a preflight inherits the defect it diagnosed.** **THE NAME RULING:** Grumbal moved to strip AC8 on the grounds that a row named `paid-two-host-run` which cannot run is a name making a claim it cannot support. **Withdrawn on measurement, not overruled** — the key is pinned at `demo_j1.rs:911`, in a Blocking gate leg (`check_j1_two_host_signed_run.rs:879-889`) and in an enrolled test (`demo_j1_tests.rs:55`), so renaming or re-homing the run costs the exact Rust this row forbids. The enforcement assigned the paid run to this key. First time in this lane the machine pointed at us rather than for us. **Routings:** `xtask +6` → `2e` (D17 bars a grant without a measured delta; this row's is zero); `deferred-work.md:826` → 14-4 (D7/D18 family); the capture gated on 2e **by mechanism, not by promise** — F3 fires the instant it lands. |
| 2026-08-19 | **Story authored by `bmad-create-story` from a seven-scout preflight at `dd4cf959`**, discharging the sprint-status row's authoring precondition. The preflight owed three things and delivered all three — the capture schema (already published by `2c`, so the row's "ABSENT" premise was stale), the release-build boot-nonce procedure (**disproved: impossible without a debugger**, F4), and proof that two hosts can hold independent audit roots (**they can, but the gate never checks it**, F6). It also found five defects nobody asked about: no tool signs a cohort manifest (F1), `verify.py` fails on any non-ASCII bundle and Phase 7.4 is a mandatory abort (F5, reproduced free on the real T6 artifact), the delegated goal cannot satisfy codex's oracle (F7), `demo-j1` breaks when this story's own capture lands (F3), and `deferred-work.md:826` assigns this row a Rust task that mechanically reds a Blocking gate at `done` (F8). Five claims in the ratified row corrected against measurement, including the stated money risk, which is backwards — a nonce mismatch is refused before the intake sink and bills nothing. |
