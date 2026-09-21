---
baseline_commit: "**`bb9f0657`** (Story 16-1 landed). Authored 2026-09-14 from two scouts — a RUNTIME scout that drove `maos init` / `maos shell` / `maosctl` / `maos audit query` on freshly rebuilt debug binaries through a real PTY in scratch `HOME`/`MAOS_HOME`/`XDG_DATA_HOME`, and a STATIC scout that re-derived every citation in the epic's 16-2 section and exit-block provenance — plus author probes. **16-1 moved almost every file this section cites** (`main.rs`, `maos-shell/src/lib.rs`, `subcommands.rs`, `maos-audit/src/lib.rs`); 14 of the section's 24 line cites were stale and two describe code 16-1 deleted. Cites below are SYMBOL-first, line second. ⚠ **T0 re-measures. A grant is a global, not a reservation.**"
depends_on: "**`16-1-daemon-post-surface-and-verb-retarget` (`done`)** — the door, `control.json`, `OperatorDoor::resolve_halt_command`, the completion TL row this story re-kinds, and the shell root's bind (`is_door_root = shell_mode || …`). **`15-6`** — `MAOS_INFERENCE_MODE` + `MAOS_REPLAY_CASSETTE` (ADR-064). **`16-0`** — only as a tripwire: this story writes no `crates/maos-kernel-core/src` byte, and the content-hash pin must stay `changed == 0`."
blocks: "**`19-2`/`19-4`** (epic-19 AC5 resolves an Orchestrator halt 'against 16-2's daemon-held registry' — true already for `maos run` roots; this story makes the name→pid and `halt list` halt_id surfaces it relies on real), **`20-x`** (epic-20 exit line 9: 'the shell binds the door'), **`ops-live-key-j0-leg`** (its runbook transcribes lines 1–3, which are red at HEAD), **`16-5`** (its AC1 cites the `let _ = capability.record_invocation` swallows in `maos-shell`; this story deletes one of the two — cite amended in §12)."
spec_alignment: "⚠ **THE EPIC IS AMENDED IN THE SAME COMMIT (rule 9, Round 6 — 16-2 section, exit-block comments 2 and 4, provenance lines 2–4, the Kloc-asks 16-2 entry, the stories-table row, 16-4 AC2 and 16-5 AC1 cites).** Measurement disproved the premises behind **all three** of the exit block's J0 lines and three of the section's four ACs: `maos audit query --spirit hello-spirit` EXITS 1 (`Fr4SchemaViolation`) on any tokenless row and the resolution is not in the table it reads; `maosctl halt list` prints no `halt_id`, so line 3's `<halt_id>` cannot be substituted from it; hello-spirit has no `Spirit` impl, so it cannot be scheduler-loaded, so the door answers `spirit_not_loaded` 404; the PTY harness cannot type; the banner test passes when the shell fails to start; and the PRD's J0 has the evaluator resolve the halt **in the REPL, after which the Spirit proceeds** — a beat neither the epic nor the tree has. The rulings are the Decisions table; the applied epic edits are §12. ⚠ **ROUND-TABLE 2026-09-14 (§15, operator-ratified)** also wrote into the epic a NEW **16-3 AC4** (NFR-Rel-11's planned half: `maos run` roots unload their Spirits at shutdown) and a note in **16-4 AC1** (`maos purge` is the erasure route for pre-16-2 hello-spirit rows)."
split_from: "Not a split. Authored from `epics/epic-16-one-daemon-one-door-j0-w1.md` 16-2 section. **Sizing: WHOLE — RATIFIED at the round-table 2026-09-14 (§15 R1).** The obvious seam — AC5's audit-read repair as `16-2b` — is the one the exit block forbids: line 4 is red until AC5 lands, so a 16-2 without it closes with its own exit line red. Measured duration **5–8 d**."
kernel_grant: "**NONE. ZERO kernel-Δ @24474**, proven by `check-kernel-baseline` `changed == 0` over the 98-file set. Every kernel API driven here is already `pub`: `invoke_halt` (`crates/maos-kernel-core/src/halt/mod.rs:504`), `HaltRegistry::lookup_state` (`:411`), `SpiritSchedulerAdapter::{load,start,resolve_pid}` (`scheduler/scheduler_loop.rs:243,…,178`), `MemoryManagerPort::read`, `KernelHaltResolver` (via the 16-1 door). ⚠ Two kernel facts this story must route AROUND, never edit: the scheduler writes its `lifecycle.*` TL rows as `FrameKind::CapabilityInvocation` with no token and `FrameOrigin::SpiritAuto` (`scheduler_loop.rs:357-364`, `journal_lifecycle` `:615-622`), and `invoke_halt` writes its EpistemicHalt row with no token (`halt/mod.rs:528-535`). Both are why the FR4 view needs a classifier (D-16-2-G) rather than a kernel change."
kloc_grant: "**Measured raises only where a ceiling is actually crossed, taken AFTER the code exists and after `cargo fmt --all`, citing the OPEN key as the bare token `16-2`** (the full key does not tokenize, `xtask/tests/d11_xtask_ceiling_ratchet.rs`), under the RECOVERY-LANE CEILING RULE. Headroom measured by `kloc-check --json` at `bb9f0657`: `maos-shell` 320/500 **+180** (`xtask/kloc.toml:642`) · `maos-audit` 6916/6951 **+35** (`:386`, ⚠ BINDING — D-16-2-F's deletion of the 39-line wildcard branch funds part of D-16-2-G/H; the writer-shape table may still cross) · `maos-bin` 18832/20109 **+1277** (`:475`) · `maos-cli` 6042/6856 **+814** (`:459`) · `maos-journey-test` 508/1104 **+596** (`:632`) · `maos-spirit-hello` 365/1000 **+635** (`:396`) · `maos-kernel-core` 18935/18935 ZERO (untouched) · `xtask` ZERO (untouched). §10 forecasts `maos-shell` (×1.3 upper +221 vs +180) and `maos-audit` (+61 vs +35) as the crates that may cross. ⚠ Inline `#[cfg(test)]` IS charged (`maos-audit`, `maos-spirit-hello` carry inline test modules) — new tests go in `tests/`."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs`) is the machine-checked set; `equiv` is prose, NOT a match token. Keep the literal `allowlist {`."
review: "§A6 full-layer net BINDING (Blind + Edge Case + Acceptance + Test-Infra + non-author runtime). Not marked ◆ in the epic, but **this story edits the FR4 mediation projection** — a control whose failure mode is a silent false-green — so it is **NON-DEGRADABLE — RATIFIED at the round-table 2026-09-14 (§15 R2)**. E15-A6 binding by name — *does this test read the tree, or only what it set itself?* Obligations (a)–(m) under **Review obligations**."
---

# 16-2 — The shell's halt is real, the operator resolves it, and the Spirit proceeds

Status: done

> **The capability:** *An evaluator runs `maos init && maos shell`, types
> `@hello-spirit refactor src/main.rs to be more idiomatic`, and the kernel halts the running
> hello-spirit on `task.acceptance_criterion.ambiguous` — a halt with an id, a Transparency Log row
> and a pending registry entry. They answer it, either by typing a clarification into the REPL or with
> `maosctl halt resolve <id>` over the door, and **watch it resolve in the REPL, after which the Spirit
> proceeds with their context**. `maos audit query --spirit hello-spirit` shows the halt and the
> resolution.*

**Closes:** J0 (v0.1 evaluator — the halt → resolve → proceed scene, hermetic in CI) · FR15 for the
J0 Spirit (both resolution surfaces, one mechanism) · FR58's J0 halt beat · `deferred-work.md:926`
(`resolve_spirit_name`'s `hello-spirit → pid 0` wildcard — retired, both copies) · the J0 half of
FR4's operator view (a scheduler-loaded Spirit's FR4 view no longer dies on its own kernel rows) · NFR-Rel-11 for the `maos shell` root (the Spirit it loads is unloaded at exit, so every pending halt gets a receipt — §15 R6; the `maos run` roots' half is **16-3 AC4**, §15 Q-A).

**Does NOT close, and says so:**
- **The NFR-Onb-2 clean-VM 5-minute wall-clock** — operator lane, `ops-live-key-j0-leg` (the repo
  already demoted it to advisory, `tests/integration/onb_nfr2_timing.sh:11-30`). This story makes the
  *response* leg honest (AC7); it does not time a VM.
- **The live-key J0 transcript** — `ops-live-key-j0-leg`.
- **OS-keyring provider keys** — `16-4` AC2; AC8 documents today's env source and 16-4 amends the page.
- **Propagating `record_invocation`'s `Err`** from the surviving `shell.turn` call — `16-5` AC1 (cite
  amended, §12).
- **Kernel lifecycle rows mis-kinded as `capability.invocation`** — a kernel byte; routed AROUND by
  D-16-2-G's classifier and named in §11 row 1.

---

## What this story actually is

The epic describes wiring: pass three `Arc`s into `run_shell` and call `invoke_halt`. Measurement says
the wiring is the small half. At `bb9f0657`, in the order a dev will hit it:

1. **The Spirit that halts does not exist as a Spirit.** hello-spirit has no `impl Spirit`, is never
   `scheduler.load`ed in shell mode, and is admitted and token-issued at pid 0. The door's halt handler
   answers `spirit_not_loaded` 404 before it looks at the registry. (§1)
2. **None of lines 3–4 can pass even after the halt is real.** `halt list` prints no `halt_id`;
   `maos audit query --spirit` exits 1 on the first tokenless row — and the halt row, the scheduler's
   lifecycle rows and 16-1's completion row are all tokenless; the resolution is journaled to a table
   the query never reads. (§2, §3)
3. **Two private pid-0 wildcards, not one**, and retiring the audited one alone leaves line 4 on the
   other. (§4)
4. **The scene the PRD describes is not the scene the epic wrote.** J0: *"type a clarification, the
   Spirit proceeds."* No path in the tree renders a resolution or resumes a turn. (§5)
5. **The test that should hold it is a null control, and the harness cannot type.** (§6)

So this is **the story that makes the J0 halt real end to end** — Spirit, halt, both resolution
surfaces, the proceeding turn, and the audit reads that prove it.

---

## 1. 🔴 HELLO-SPIRIT IS NOT A SPIRIT THE SCHEDULER KNOWS

- **No `impl Spirit`.** `grep -rn "impl.*Spirit for" crates/maos-spirit-hello` = 0. `maos-spirit-hello`
  is two free functions (`dispatch_directive` `crates/maos-spirit-hello/src/lib.rs:176-188`, `say_hi`
  `:163-173`) and already depends on `maos-spirit-abi` — an impl can live there with no cycle.
- **Shell mode never loads it.** The shell block (`crates/maos-bin/src/main.rs` `if shell_mode` `:3617`)
  contains no `scheduler` reference; it calls `security.admit_spirit(0, "hello-spirit", …)` (`:3680`)
  and `maos_shell::run_shell` issues the token with pid literal `0` (`crates/maos-shell/src/lib.rs:268-277`).
- **The door therefore refuses the resolution** — runtime scout, shell alive:
  `maosctl halt resolve deadbeef --spirit hello-spirit --kind provided-context --text …` ⇒ exit 1
  `spirit_not_loaded … (HTTP 404)`, from `OperatorDoor::resolve_halt_command`'s first statement
  (`crates/maos-bin/src/operator_door.rs:875`). **And every refused attempt still writes a TL row** —
  16-1's completion row, `operator.halt-resolve.op-…`, at `spirit_pid` 0 (§3).
- **`maos run` shows the right order**: load FIRST, then admit at the REAL pid (`main.rs:5276-5296`,
  *"admitting a placeholder pid would make every live port fail closed"*), then `scheduler.start`.
  `allocate_pid` (`scheduler_loop.rs:31`) starts at **1 in every process**.
- **The manifest is read CWD-relative** (`main.rs:3630`, `spirits/hello-spirit/manifest.toml`). Runtime:
  from any directory but the repo root, `maos shell` exits 1 `shell: cannot read hello-spirit manifest`
  — an installed `maos` cannot run J0 at all (FR58 *"zero-config path from install"*) — and that `?`
  return skips the door teardown the block performs before `return shell_result`.
- **A second journal handle.** The shell block opens its own `JournalAdapter::open` (`main.rs:3677`)
  while the daemon's `shared_journal` (`:2886`) is open on the same path — 16-1 §11 row 1's cross-handle
  overwrite hazard, inside one process.

**Present and correct (no work):** the shell root already binds the door (`is_door_root = shell_mode || …`
`main.rs:3300`) with the **same** `halt_registry` `Arc` (`:2197`); `boot_nonce` (`:2077`),
`output_markers` (`:2198`) and `shared_journal` are in scope; the multi-thread runtime keeps serving
door handlers while `run_shell` blocks on stdin — measured, `halt resolve` answered in 0.07 s.

## 2. 🔴 `halt list` PRINTS NO `halt_id`

`halt_list_reader` (`crates/maos-cli/src/subcommands.rs:1916`) prints
`{"frame_id": <first 8 hex>, "timestamp_ns", "kind", "intent"}` (`:1945-1951`); `intent` is the tag.
The `halt_id` lives only in the row's payload (`EpistemicHaltPayload.halt_id`,
`crates/maos-domain/src/frame.rs:214`). `crates/maos-cli/src/cli.rs:714` documents the resolve argument
as *"HaltId returned by `maosctl halt list`"* — false. So exit line 3's *"the harness substitutes
`<halt_id>` from `halt list`"* is impossible as the tree stands.

Also: `halt list --spirit hello-spirit` resolves the name through `maos_audit::resolve_spirit_name`,
whose hello-spirit branch filters `spirit_pid = 0` — so once the Spirit has a real pid, the list would
filter to pid 0 and miss the real halt (§4). Load and wildcard retirement must ship together.

## 3. 🔴 EXIT LINE 4 EXITS 1, AND THE RESOLUTION IS IN ANOTHER TABLE

- **Runtime, HEAD, after one shell session:** `maos audit query --spirit hello-spirit` ⇒ exit 1
  `Fr4SchemaViolation { line: 1, missing_field: "capability_token" }`. `run_audit_query` sets
  `fr4_mode = spirit.is_some()` (`crates/maos-shell/src/lib.rs:184`); `project_to_fr4`
  (`crates/maos-audit/src/lib.rs:603-619`) errors on any row whose token is NULL. Scratch-copy probe: a
  DB holding ONLY the kind-3 row `invoke_halt` writes still exits 1.
- **Every row this story makes appear under hello-spirit's pid is tokenless by construction** and none
  is an external call: the EpistemicHalt row (`halt/mod.rs:528-535`, `None`), the scheduler's
  `lifecycle.admit`/`lifecycle.load` rows (`scheduler_loop.rs:330,357-364` — kind **7
  `capability.invocation`**, `None`, `FrameOrigin::SpiritAuto`), and 16-1's operator completion row
  (`OperatorDoor::run_command`, `operator_door.rs:1357-1370` — also kind 7, `None`, `FrameOrigin::Kernel`).
- **16-1 shipped that completion row off its own ratified decision.** D-16-1-E says
  `insert_frame_event(FrameKind::TelemetryEvent, …, "operator.command:<verb>:<spirit>:<operation_id>:<outcome>")`;
  the tree writes `FrameKind::CapabilityInvocation` with intent `operator.<verb>.<operation_id>` and no
  outcome. A tokenless row of kind `capability.invocation` is precisely what FR4's view exists to
  refuse — so after 16-1, **every** door command against a Spirit reds that Spirit's FR4 view.
- **The resolution is journaled to `approval_decision_log`, not `transparency_log`.** `HaltFlow::submit_resolution`
  → `HaltJournal::journal_halt_resolution` (`crates/maos-iac/src/adapter/transparency_log.rs:2105-2131`)
  → `insert_approval_decision` (`:1420`); `maos_audit::query` reads only `FROM transparency_log`. FR15
  (*"kernel journals the resolution with full reasoning chain"*) is satisfied there — but no audit read
  renders it, and line 4's *"its resolution row"* has no referent.
- **The plain renderers hide `intent`** (`to_plain` `:795-818`, `to_fr4_plain` `:825`): columns
  `call_id, boot_nonce, spirit_pid, call_type, timestamp_ns[, token]`. Seven `capability.invocation`
  rows are indistinguishable to an operator.
- **Butler has the same defect today**: its `lifecycle.*` rows sit at its real pid, tokenless, kind 7, so
  `maosctl audit query --spirit butler` dies the same way (static read; not run). **So do many more
  kernel rows** — every successful inference's kind-29 `cost:inference-attribution` row (validator probe:
  one-shot replay ⇒ `FR4 schema violation at line 4`), `schedule.fire:*` and `telemetry.self` (kind 7),
  hook-budget kinds 12/13, watchdog kinds 15/16. The fix is class-wide only because D-16-2-G classifies
  from an exhaustive writer inventory with a doorbell test, not from the rows J0 happens to write.
- **hello-spirit's own inference is journaled at pid 0 regardless of its token.** `maos_spirit_hello::run`
  builds `InferenceRequest::new(0, token, …)` (`crates/maos-spirit-hello/src/lib.rs:63-64`), and
  `InferencePortAdapter::complete` writes the `inference.call` and cost rows at `req.spirit_pid`
  (`crates/maos-kernel-core/src/inference/mod.rs:350-356, 422-429`) without checking the token's pid. A
  scheduler-loaded hello-spirit would still hide its mediated calls from `--spirit hello-spirit`.

## 4. 🔴 TWO pid-0 WILDCARDS, AND THE AUDITED ONE IS NOT LINE 4's

- `maos_audit::resolve_spirit_name` (`crates/maos-audit/src/lib.rs:1625`) opens with
  `if name == "hello-spirit"` (`:1634-1672`), answering `SELECT … WHERE spirit_pid = 0` — every pid-0
  kernel row of every boot. This is `deferred-work.md:926`, owner 16-2.
- **Line 4 does not use it.** `maos audit query` (`maos_shell::run_audit_query`) uses maos-shell's
  **private** `resolve_spirit_pid` (`crates/maos-shell/src/lib.rs:378-383`, `"hello-spirit" => Some(0)`,
  no boot filter). Once hello-spirit has pid 1, butler in a later `maos run` boot on the same TL is
  also pid 1 — a pid-only filter mixes them.
- **Consumers of the audited wildcard** (static scout): `maosctl` `halt list`, `audit query`,
  `sealed-export`, `scan-credentials`, `trajectory-export`; the uninstall cascade; tests
  `erasure_uninstall_13_5b.rs` (≈10, seed hello-spirit data at pid 0 with no identity row),
  `maos-cli/tests/audit_no_color_test.rs` (5, incl. the pinned diagnostic), and the CI scripts
  `tests/integration/v01_evaluator_path.sh` step 3 and `audit_query_fr4_smoke.sh`, both fed by the
  `MAOS_ONE_SHOT=hello-spirit` evaluator run at pid 0, which writes **no identity row** (it admits at pid
  0, `main.rs:7699-7715`, but nothing names hello-spirit in `transparency_log`).

## 5. 🔴 THE PRD's J0 RESOLVES IN THE REPL, AND THE SPIRIT PROCEEDS

- `_bmad-output/planning-artifacts/prd/user-journeys.md:293` (J0): *"The Spirit halts immediately on
  `task.acceptance_criterion.ambiguous` … They laugh, **type a clarification, the Spirit proceeds**."*
  The epic's own *Makes true*: *"can watch a halt resolve."*
- **No mechanism exists.** `run_shell` prints `[HALT …]` and moves on to the next stdin line; nothing
  observes the registry, renders a resolution, or resumes the directive.
- **The cassette is decorative for the halt beat.** `is_ambiguous` (`maos-spirit-hello/src/lib.rs:127-157`)
  is a substring match run BEFORE inference (`:181`); the j0 cassette's single entry is consumed only
  by `say hi`. Exit line 1's `MAOS_INFERENCE_MODE=replay` does nothing for J0 as the tree stands. A
  proceeding turn is exactly what would replay it.
- **The resolver already delivers the context to the Spirit.** `KernelHaltResolver::resolve` for
  `ProvidedContext` writes the text to the Spirit's private tier under `halt_context::<halt_id>`
  (`crates/maos-kernel-core/src/halt/resolver.rs:162-176`) — the Story 4.3 channel a Spirit reads to
  continue. Nothing reads it.

## 6. 🔴 THE J0 TEST IS A NULL CONTROL, AND THE HARNESS CANNOT TYPE

- `j0_shell_banner_via_pty` (`crates/maos-journey-test/tests/journey_j0.rs:55-77`) waits for ANY of
  `["maos shell", "hello-spirit"]`. `hello-spirit` also appears in the admission stderr line, the
  manifest-missing error and `--help`. **Proven:** running the compiled test with
  `CARGO_MANIFEST_DIR` pointed at a fake workspace with no `spirits/` ⇒ the child exits 1 with the
  manifest error and the test reports `ok`. (The epic's "no `shell` argument" reading was wrong: a bare
  `maos` IS the shell, `verbs::Outcome::NoArgs` `main.rs:1803-1808`.) It never runs `maos init`, so no
  door, and never sets `MAOS_INFERENCE_MODE`.
- **`Pty` has no input method** (`crates/maos-journey-test/src/lib.rs:433-595`: `spawn`, `screen`,
  `wait_for_screen`, `wait`, `wait_with_timeout`); the master is stored, never written.
- **CI cannot run line 3 from the J0 job.** `journey-hermetic-tier-1` (`.github/workflows/discipline.yml:2156`)
  builds only `maos` + `worker-cli-fixture` (`:2180`) — never `maosctl` — and sits in `aggregate.needs`
  (`:3830`) only. `journey_j0` also runs inside `workspace-test-suite`, where 16-1's decoy door holds the
  runner's real `control.json` endpoint — so every `maosctl` the harness runs must carry the world's
  isolated env.
- The no-wallclock guard (`guards_meta.rs:35`) covers `journey_j0.rs`: bounded waits live in the harness.

## 7. 🟠 SMALLER FALSE CLAIMS IN THE SAME SCENE

- **NFR-Onb-2's response leg accepts an error as a response.** `onb_nfr2_timing.sh` times
  `MAOS_ONE_SHOT=hello-spirit ./target/release/maos` (not the shell) and checks JSON keys only; runtime
  scout: the one-shot exits 0 with `"introduction": "… Inference transport error — the configured
  provider is unreachable."` and passes. The epic's AC1 says this budget *"stays blocking"* — it is
  blocking over a fallback string. Job `onb-nfr2-timing` (`discipline.yml:1093`) is in `aggregate.needs`
  (`:3782`).
- **hello-spirit's honest disclosure is not honest.** `transparency_log_default()` returns the literal
  `"xdg:maos/audit/transparency.sqlite"` (`maos-spirit-hello/src/lib.rs:194-196`); under `MAOS_HOME` the
  log is `$MAOS_HOME/audit/transparency.sqlite` (runtime: `maos init` prints the real path; `say hi`
  prints the literal).
- **AC4's doc is not the published page.** `/run-maos` builds from `docs-site/docs/run-maos.md` (with an
  `i18n/ko` counterpart counted by `docs-site/scripts/gate-ko-coverage.js`); `docs/maos.dev/run-maos.md`
  is an unpublished, diverged copy read by no gate.

---

## 10. SIZING AND BUDGET (rule 6, ×1.3; interval arithmetic)

| crate | headroom | change | raw | ×1.3 upper |
|---|---|---|---|---|
| `maos-shell` | **+180 (may cross)** | `ShellHost` port trait; REPL loop → stdin reader thread + `recv_timeout` tick; pending-halt state; clarification submit; resolution render; proceeding turn; delete private `resolve_spirit_pid`; `run_audit_query` → `resolve_spirit_name` + boot filter + trail view; drop `shell.halt` cap row | +110…+170 | **+221** |
| `maos-bin` | +1277 | NEW lib `src/shell_host.rs` (`ShellHost` impl: halt-id mint, `invoke_halt`, state, port submit, context read) +90…+160; shell block: load→admit→start, embedded manifest, shared journal, port built for every door root, error-path teardown +30…+70; one-shot identity row +8…+15; completion row kind + payload +3…+8 | +131…+253 | **+329** |
| `maos-audit` | **+35 (binding — may cross)** | **minus** the hello-spirit wildcard branch (−39); FR4 row classifier with the non-call kind set and the writer-shape table (one entry per tokenless kind-7 writer, T0) +40…+70; symmetric names for kinds 12/13/15/16 +8; plain `intent` column +4…+8; diagnostic wording | +13…+47 | **+61** |
| `maos-cli` | +814 | `halt list` `halt_id` + `record` +12…+28 | +12…+28 | **+37** |
| `maos-control` | +686 (1105/1791 after 16-1) | extract `submit_and_wait` + `SubmitOutcome`; server maps it; its test in `crates/maos-control/tests/` | +20…+50 | **+65** |
| `maos-spirit-hello` | +635 | `HelloSpirit` + `MANIFEST_TOML` + `proceed_with_context` + log-path parameter + request pid from the token | +22…+48 | **+63** |
| `maos-journey-test` | +596 | `JourneyWorldBuilder::env`; `Pty::send_line` + `send_eof`; `maosctl` resolver + world-env command helper; bounded row/door waits | +35…+80 | **+104** |
| `maos-kernel-core` | ZERO | — | 0 | 0 |
| `xtask` | ZERO | — | 0 | 0 |

`maos-shell` and `maos-audit` are the crates forecast to possibly cross. If it does: code first, `cargo fmt --all`,
`kloc-check --json`, the `kloc.toml` row carries the bare token **`16-2`** followed by a non-digit
non-dash character, the measured figure and the driver, same commit. **Do not compress the REPL to fit**
— the pending-halt state machine is the capability.

Dependencies: **THREE manifest edges as shipped (§15 R9 ratified ONE — the deviation is recorded in the Change Log), no new
crate.** (1) `ulid = "1.1"` in `crates/maos-bin/Cargo.toml` (kernel-core's spec; `Cargo.lock` resolves 1.2.1 through
`maos-kernel-core` and `maos-iac`) — mints the halt ids. (2) `parking_lot` in `crates/maos-shell/Cargo.toml`
**`[dev-dependencies]`** — backs the gated reader (`Condvar`/`Mutex`) in `shell_halt_repl_16_2.rs` that withholds EOF
until the test releases it. (3) `parking_lot` in `crates/maos-journey-test/Cargo.toml` as a **regular `[dependencies]`
edge** — it backs `Pty::writer` (`crates/maos-journey-test/src/lib.rs:458`), harness code that ships in the crate's
`src/`, so a dev-dependency would not compile. `parking_lot` was already locked at a single version, so no `deny.toml
[bans] multiple-versions` breach. The lockfile's `maos-bin` dependency list still changes — commit `Cargo.lock` with it,
or every `--locked` build (incl. `j0-scene`) fails. New file ⇒ `SCANNED_SOURCE_FILES`
(`crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:878`) **20 → 21**.

---

## Decisions ratified in this story (rule 7 — one decision per fork)

All rulings below are **operator-ratified** (round-table §15, 2026-09-14), as corrected by validation round 3 (Change
Log); **D-16-2-O** is the §A6 review's operator ruling of 2026-09-15.

| # | Fork | Ruling | Rejected, and why |
|---|---|---|---|
| **D-16-2-A** | How hello-spirit becomes a Spirit in shell mode | **Scheduler-loaded, like every other Spirit.** A `pub struct HelloSpirit;` with `impl maos_spirit_abi::lifecycle::Spirit` (all hooks default) and `pub const MANIFEST_TOML: &str = include_str!("../../../spirits/hello-spirit/manifest.toml")` in **`maos-spirit-hello`**. The shell block does what `maos run` does (`main.rs:5276-5296`): `scheduler.load("hello-spirit", bundle, HelloSpirit, boot_nonce)` → `security.admit_spirit(pid, …)` at the **real pid** → `scheduler.start(pid)`; the token is issued for that pid. The manifest is parsed from `MANIFEST_TOML`, never the CWD. The admission journal is the daemon's `shared_journal`; the block's own `JournalAdapter::open` (`:3677`) is deleted. The block's body moves into one `async fn` returning `Result`, and the door teardown — in D-16-2-C's order (server shutdown → `drain_started_tasks` → unload → port + `ShellHost` drop → audit drain → locks) — runs in the CALLER on both arms — so no `?` can bypass it, by construction rather than by review. | *Keep pid 0, fake a `resolve_pid` answer in the door* — a door that lies about what is loaded. *Admit then load* — `maos run` measured that ordering failing live ports closed. *`impl Spirit` in `maos-bin`* — the Spirit's identity belongs with the Spirit; the `UpgradeSmokeSpirit` precedent is a test fixture. *Keep the CWD read* — J0 fails on every installed binary. |
| **D-16-2-B** | How the halt is raised | **Through `invoke_halt`, behind a `ShellHost` port, over an I/O seam.** `run_shell(host, input: impl BufRead + Send + 'static, output: impl Write, …) -> Result<(), _>` — `'static + Send` because D-16-2-D moves the reader onto its own thread (E0310 without `'static`), and production passes `std::io::BufReader::new(std::io::stdin())`, NOT `stdin().lock()` (a `MutexGuard`, not `Send` — E0277; both compiled by the fifth read); output stays on the REPL thread and production passes `std::io::stdout()`, not a held lock; tests pass `std::io::Cursor<Vec<u8>>` and a `Vec<u8>`. The `Result` keeps 15-6 D2's non-zero exit and P13's `shell_result.is_err()` flush check. **Every REPL line goes through `output`** — including the banner, `shell exiting`, refusals and resolution renders; today's `print_line` (`crates/maos-shell/src/lib.rs:400-405`) calls `println!` and would bypass the seam, so it takes the writer. Fake-host tests that need a live REPL with NO further stdin line (AC4's door-path and context-race vectors) pass a **gated reader** that withholds EOF until the test releases it (a `Cursor` hits EOF at once, and EOF unloads by design — never make EOF wait for a pending halt), and pass output as `&mut Vec<u8>` so the test can read it — today it reads real stdin and takes a concrete `Arc<CapabilityRegistryAdapter>` (`crates/maos-shell/src/lib.rs:204-209`), which no fake-host test can drive; token issuance moves behind `ShellHost` too. `maos-shell` defines a sync trait `ShellHost` (`spirit_pid`, `issue_turn_token() -> Result<CapabilityToken, _>`, `record_turn(token, payload) -> Result<(), CapError>` (passes `record_invocation`'s result through UNCHANGED — the one swallow stays `let _ = host.record_turn(..)` in `maos_shell::run_shell`, the site 16-5 AC1 cites), `raise_ambiguity_halt(tag, prompt) -> Result<String /*halt_id*/, _>`, `halt_state(id)`, `resolve_with_context(id, text) -> Result<(), ResolveRefusal>`, `take_context(id)`); `maos-bin` implements it in a NEW lib module **`crates/maos-bin/src/shell_host.rs`** over the daemon's `transparency_log`, `shared_journal`, `halt_registry`, `memory` and the door port. `raise_ambiguity_halt` mints a **ULID** (`ulid::Ulid::new().to_string()` — the kernel's own halt-id grammar, `crates/maos-kernel-core/src/capability/working_memory/policy_runtime.rs:127`, so one registry holds one id grammar; 26 Crockford-base32 chars, inside the door's `[A-Za-z0-9._-]{1,128}` rule, `crates/maos-cli/src/door_client.rs:532`; `ulid = "1.1"` is added to `crates/maos-bin/Cargo.toml` — already in `Cargo.lock` through kernel-core, so no new lockfile crate — §15 R9) and calls `invoke_halt(tl, &shared_journal, &halt_registry, EpistemicHaltPayload::new(id, "task.acceptance_criterion.ambiguous", 1.0, None, "hello-spirit.ambiguity", "shell.directive")?, pid, "hello-spirit", boot_nonce)`. The REPL prints `[HALT <tag>] <prompt>` **only after** `Ok`, followed by `halt <id> — type a clarification, or: maosctl halt resolve <id> --spirit hello-spirit --kind provided-context --text "…"`. The `shell.halt` `record_invocation` call (`maos-shell/src/lib.rs:317-321`) is **deleted**: no call was made, and a `capability.invocation` row for a halt claims one. | *Pass five `Arc`s into `run_shell` (epic AC2)* — `maos-shell` would need `KernelHaltResolver`'s seven collaborators too; a port keeps the REPL testable with a fake host. *`halt-<24 hex>` (the author's draft)* — a second id grammar in one registry (§15 R9). *Print before `invoke_halt`* — `halt list` could race an id the TL does not have yet. *Keep `shell.halt`* — a false FR4 row. |
| **D-16-2-C** | Two resolution surfaces | **One mechanism: the 16-1 `OperatorDoor` port.** `maosctl halt resolve` reaches it over HTTP (unchanged). A clarification typed in the REPL is `ShellHost::resolve_with_context`, which **submits `OperatorCommand::ResolveHalt { spirit_id: "hello-spirit", halt_id, resolution: "provided_context", rationale: Some(text) }` to the same `OperatorDoor` in-process** and waits on its completion — same validation, same `KernelHaltResolver`, same approval-log row, same completion TL row. To make that possible the port is **constructed for every door root** (`main.rs` `is_door_root`, `:3300`; every constructor argument is already in scope — `door_crl_trust_anchor` `:3025`, `successor_manifest_slot` `:3180`) whether or not a config exists; only the HTTP bind stays gated on the env pair / `control.json`. **ONE submit-and-withdraw implementation (§15 R4).** The HTTP server's private `fn submit` (`crates/maos-control/src/lib.rs:1308-1344` — `port.submit(command, command.route_budget())`, `recv_timeout(budget)`, and on `Timeout` the `COMMAND_QUEUED → COMMAND_WITHDRAWN` CAS that decides `SpiritBusy` vs `HandlerStillRunning{operation_id}`; `OperatorDoor::run_command` relies on that caller-side CAS, `operator_door.rs:1316-1332`) is extracted as **`pub fn submit_and_wait(port: &dyn OperatorCommandPort, command: OperatorCommand) -> SubmitOutcome`** with `SubmitOutcome::{Completed(OperatorOutcome), SpiritBusy, HandlerStillRunning { operation_id }, Internal}`; the server maps it to its existing responses byte-for-byte, and the shell host maps it to REPL lines. Neither re-implements the CAS; the REPL keeps the halt pending in its own state on `SpiritBusy`/`HandlerStillRunning` (the tick decides). **Shell teardown, ONE order shared with 16-3 AC4 (validation round 3):** `server.shutdown()` → `drain_started_tasks().await` (all three run-root teardowns do it, `main.rs:4498, 5461, 7880`; the shell's `:3749-3757` does not) → `scheduler.unload(pid).await` (R6 — AFTER the door is closed, so no `maosctl halt resolve` races the drain) → drop the port and the `ShellHost` → **drain the audit writer — specified as an INVARIANT with an ORACLE, not as a list to copy** (two successive reads found each copied list wrong: the butler `--once` block `:5466-5478` drops 7 owners and times out, probed; the SIGTERM block's first 19 names omit `iac`): *every owner of `audit_tx` is dropped before the writer is awaited, proven by the writer completing — no `drain timed out` on the screen.* The complete reference is the SIGTERM root's drop block `main.rs:7941-7970` (30 drops, incl. the two HIDDEN owners its comments name: `iac` → digest provider → `digest_memory` → `memory` → `capability` → `audit_tx`, and `delegation_leg` → `Mailbox` → `ScbTracker` → SCB map → Spirit → `capability`); the topology `--once` root's `:4501-4520` is the shorter proven one (`drain_once_audit_writer.rs:32`). In the shell arm, **omit `inference`** — it was moved into `inference_arc` (`:3721-3722`) and on into `run_shell`, so `drop(inference)` is E0382; drop the `ShellHost` and the port instead (shell-only owners of `capability`). The butler `--once` block is routed to **16-3 AC4** (§11 row 7). The shell never awaits `audit_writer` (spawned `:3280`), so without this `cap.issue`, `shell.turn` and unload's `cap.revoke` rows can be lost when `main` returns → release the store locks. Fixed at origin (16-1 Trap 16). The REPL offers `provided_context` only (the PRD's beat); `accepted_halt` / `authorized_override` stay `maosctl` verbs. | *Re-implement the server's CAS in the host (the author's draft)* — two copies of the rule 16-1's fourth read got right (§15 R4). *Call `HaltFlow` directly from the host* — a second resolution path that would drift from the door's spirit-mismatch check and completion row (16-1 review HIGH "verify a pending halt belongs to the requested Spirit"). *REPL resolution only when a door is bound* — J0 must work with no `maos init` (FR58 zero-config). *All three kinds in the REPL* — not in the PRD beat; a typed kind grammar in a chat REPL invites mistyped overrides. |
| **D-16-2-D** | "Watch it resolve" and "the Spirit proceeds" | **The REPL observes the registry, not a message.** `run_shell` reads stdin on a dedicated thread into a `std::sync::mpsc` channel and the loop `recv_timeout(100 ms)`; while a halt is pending, each tick calls `ShellHost::halt_state(id)`. When it leaves `PendingResolution` — by the REPL or by the door, indistinguishably — the REPL renders `halt <id> resolved: <kind>` and: **`Resumed` (provided_context)** ⇒ `take_context(id)` reads `halt_context::<id>` from the Spirit's private tier (the resolver's own delivery channel, `resolver.rs:162-176`) and runs `maos_spirit_hello::proceed_with_context(inference, token, directive, context)` — `run` with the context appended, **no ambiguity re-check** — rendering its response exactly as a normal turn (and its `shell.turn` row); **`Overridden`** ⇒ proceeds without context; **`Terminated`** ⇒ `turn abandoned (accepted_halt)`. While a halt is pending, a line starting with `@` is refused naming the pending id; any other non-empty line is the clarification. Two concurrent resolutions: the registry transition is atomic; the loser gets the door's typed `halt_already_resolved` 409, printed, and the winner is rendered on the next tick. **EOF unloads the Spirit (§15 R6, NFR-Rel-11 — *"every Spirit termination, planned or unplanned, produces a halt receipt before process exit"*, `prd/non-functional-requirements.md:30`):** after `run_shell` returns — on EITHER arm, `Ok` or `Err` — a `shell_host.rs` lib fn that `main` calls — it takes `shell_result` and returns the result `main` exits with, so `shell_host_16_2.rs` can prove the exit contract in-process — reads the Spirit's pending halts from the registry itself — in D-16-2-C's order, AFTER `drain_started_tasks` and immediately before `unload` — (`HaltRegistry::drain_for_spirit_dry_run(pid)`, `crates/maos-kernel-core/src/halt/mod.rs:363` — non-draining; pid from `scheduler.resolve_pid("hello-spirit")`), so no id has to survive a failed REPL, and then awaits `scheduler.unload(pid)`; that runs `terminate_spirit(…, TerminationKind::PlannedUnload)` (`crates/maos-kernel-core/src/halt/termination.rs:26`), draining every pending halt into a receipt row carrying its `halt_id` and `Terminated`. Only then, and only for each id from that dry run whose `HaltRegistry::lookup_state(id)` is now `None`, does the root print `halt <id> closed by planned unload (no resolution)` (`unload` returns `Result<(), LifecycleError>` (`scheduler_loop.rs:471`) and discards its receipts (`:502`), so the registry is the evidence; an `Err` is printed and the exit code is non-zero — never print before the act, D-16-2-B's own rule); the lib fn returns `Ok` (exit 0) on the `Ok` arm and `run_shell`'s `Err` on the `Err` arm (non-zero, 15-6 D2). A late clarification (a non-`@` line arriving when no halt is pending because the other surface resolved it) prints `no halt is pending — halt <id> was already resolved (<kind>); your text was not sent` (§15 R8). | *Leave the halt pending at EOF (the author's draft)* — violates NFR-Rel-11 and leaves a halt row no receipt ever closes (§15 R6). *Poll only when the next stdin line arrives* — the operator resolving from another terminal would see nothing until they typed; that is not "watch". *A notification channel from the door* — a new mechanism in maos-control; the registry is already the truth. *Re-run `dispatch_directive` with the text appended* — `is_ambiguous` still matches `more idiomatic`, so the clarification halts again forever. |
| **D-16-2-E** | `halt list` output | **Adds `halt_id` and `record`** (§15 R6), classified in THIS ORDER — the order is the correctness (validation round 3: a serialized `HaltReceipt` parses as `EpistemicHaltPayload`, whose only required field is `halt_id` and which denies no unknown fields, `crates/maos-domain/src/frame.rs:212-245` — probed): **(1)** empty payload ⇒ `record: "termination_marker"`, `halt_id: null` (the row `terminate_spirit` writes before each receipt, `halt/termination.rs:54-60,86-93`); **(2)** parses as `HaltReceipt` ⇒ `"termination_receipt"` with its `halt_id` — EXCEPT a receipt whose `halt_id` `starts_with("term-")` — the kernel's synthetic no-halt form `term-<spirit_id>-<pid>-<ns>` and its `term-fallback-<pid>`/`term-unknown` fallbacks (`termination.rs:44-52`, written when NOTHING was pending) ⇒ `"termination_no_pending"`, id shown (it was never raised; resolving it answers `halt_not_pending`; sound because every `invoke_halt` caller mints ULIDs, R9); **(3)** parses as `EpistemicHaltPayload` ⇒ `"raised"`; **(4)** anything else ⇒ `"unparseable"`, `halt_id: null`, counted on stderr. The other four keys unchanged; `cli.rs:714`'s doc becomes true. | *Filter receipts and markers out* — a reader that hides rows (§15). *`halt_id` only (the author's draft)* — every clean shell exit would add unexplained rows. *Raised-first classification (the round-table's wording)* — labels every receipt `raised` (validation round 3, probed). *Print the full frame id instead* — not what `resolve` takes. |
| **D-16-2-F** | The `hello-spirit → pid 0` wildcards | **Both retired.** (1) maos-shell's private `resolve_spirit_pid` is deleted; `run_audit_query` calls `maos_audit::resolve_spirit_name(db, name, false)` and filters on **both** `boot_nonce` and `spirit_pid`. (2) The `if name == "hello-spirit"` branch (`maos-audit/src/lib.rs:1634-1672`) is deleted; hello-spirit resolves through the identity rows like every Spirit. (3) The `MAOS_ONE_SHOT=hello-spirit` evaluator run, which really does admit hello-spirit at pid 0 in its boot, writes ONE identity row: `insert_frame_event(FrameKind::SpiritAdmitted, 0, None, "hello-spirit", <json {spirit_id, source:"one-shot"}>, FrameOrigin::Kernel)` right after its `admit_spirit` — the kind-19 branch matches `intent == name`. (4) Test fixtures (validator count at `bb9f0657`): `crates/maos-bin/tests/erasure_uninstall_13_5b.rs` — **9 tests red** plus `proof_write_failure_has_failed_terminal_code`, which passes only by accident, and **7 of the 9 are blocking legs of `check-reza-production-path`** (T8 runs that gate); `crates/maos-cli/tests/audit_no_color_test.rs` — **5 red**. Fixtures whose subject is erasing/reading hello-spirit data seed an identity row. **The 13.5b terminal contract is preserved, not re-derived:** `not_found_uninstall_has_distinct_terminal_code` expects 4, and without the wildcard a missing TL makes the resolver `Err`, which the cascade maps to 5 — so the cascade maps *no Transparency Log at all* to `not_found` 4 (provably nothing to erase), keeping 5 for write/proof failures; a TL that EXISTS but names no such Spirit keeps exactly the outcome the cascade gives any other unknown Spirit name today (T0 records it) — never a new `not_found`, which on a legacy TL holding unidentified hello-spirit rows would sign "nothing to erase" over data that exists (the 13.5b false-success shape). The test is NOT reseeded (that would change what it proves). maos-audit's `Err` string *"only 'hello-spirit' is available at v0.1-β"* is re-worded; maosctl's own diagnostic at `crates/maos-cli/src/subcommands.rs:2711` is pinned by `audit_no_color_test.rs:290` — leave it, or change both in one commit. Other consumers to re-verify: `audit record-capture` (`subcommands.rs:3390`), `log_composition::ranged_recall` (`:122`), and the prose in `crates/maos-audit/README.md:7,40,158`. **Residual, stated, with remedies (§15 R7):** a Transparency Log written before this story holds hello-spirit rows with no identity row — including `shell.turn` payloads that carry what an evaluator typed; resolving that name there now fails closed `unknown spirit` (erasure refuses rather than erasing every pid-0 row of every Spirit); those rows stay reachable by `maosctl audit query --boot <nonce>` and erasable by 16-4's `maos purge`. | *Identity-first with a pid-0 fallback* — keeps the defect `deferred-work.md:926` names for every DB with no shell boot. *Delete only the maos-audit copy* — line 4 stays on the other. *Keep pid 0 for hello-spirit* — the door needs a loaded pid. |
| **D-16-2-G** | FR4's view of rows that are not calls | **The FR4 projection classifies before it validates, against an exact inventory of the tree's tokenless writers.** In `maos-audit`, a row is a **non-call kernel event** iff **(a)** its kind is one that is never an external call by its `FrameKind` definition — `epistemic.halt` 3, `telemetry.event` 4, `BudgetWarning` 12, `BudgetExceeded` 13, `TaskStalled` 15, `SilentFailureSuspect` 16, `spirit.admitted` 19, `governance.event` 28, `cost.attribution` 29 (every successful inference writes a tokenless kind-29 `cost:inference-attribution` row, `crates/maos-kernel-core/src/inference/mod.rs:426` — measured red in FR4 at HEAD; the `inference.call` row it attributes carries the token) — which needs `kind_to_string`/`kind_from_string` (`maos-audit/src/lib.rs:666`) to name 12/13/15/16 symmetrically instead of `unknown`; **or (b)** its kind is `capability.invocation` 7 AND it matches, EXACTLY, a **`NonCall` entry** of the **writer-shape table** (a `Call` entry never exempts) — one entry per tokenless kind-7 `insert_frame_event` site in `crates/maos-kernel-core/src` and `crates/maos-bin/src` (26 `FrameKind::CapabilityInvocation` sites in those two at `bb9f0657`, token-bearing ones excluded; the Scope sentence below adds `crates/maos-iac/src`), each entry = an exact intent (or a fixed prefix ending in `:` for templated intents such as `schedule.fire:`) + the payload keys and types that writer sets. The eight lifecycle shapes, measured: `lifecycle.admit` ⇒ `{spirit_id: str}` only (`scheduler_loop.rs:326-334`); `lifecycle.load` / `lifecycle.journal` / `lifecycle.crash` ⇒ `lifecycle_event: str` + `spirit_id: str` (`:351-364`, `:611-622`, `supervision/crash_detector.rs:198-209`); `lifecycle.start` / `pause` / `resume` / `unload` ⇒ `lifecycle_event` equal to the capitalised verb AND `spirit_pid` equal to the row's `spirit_pid`, no `spirit_id` (`:381-384`, `:408-410`, `:435-437`, `:488-490`). T0 derives the rest of the table (`schedule.fire:*` `schedule_watchdog.rs:247`, `telemetry.self` `self_telemetry.rs:195`, …) from the grep, never from this list. **A kloc-free inventory test** re-derives the tokenless kind-7 writer sites from source and reds when one has no DISPOSITION — every entry is `NonCall { shape }` (exempt, matched by the classifier) or `Call` (deliberately NOT exempt: an FR4 finding if it ever lands at a Spirit's pid); `#[cfg(test)]` items are skipped (fourth read: `transparency_log.rs:2172,2236`) — **its unit is the CALL SITE, keyed (file, enclosing fn, ordinal within that fn), never the intent literal** — `SpiritSchedulerAdapter::load` holds two writers (`lifecycle.admit` and `lifecycle.load`), so file + fn alone merges them — and a writer whose intent is built by `format!` or passed in a variable still demands an entry naming the site (§15 R3). The scanner is a paren-balanced call parser (one-line calls exist, `revocation/applier.rs:166`), covers every writer method with its own token-argument position (`insert_frame_event` arg 2; `insert_frame_event_with_sender` arg 4 — tokenless kind-7 `cli.subprocess.exit` rows at `cli_wrapper/runtime.rs:664`, `main.rs:8939`; `_with_correlation`; `_with_id`), resolves kind paths through aliases (`TlFrameKind::` at `schedule_watchdog.rs:247`, `crate::iac::…`, `maos_kernel_core::iac::…`), and requires an explicit entry for any site whose KIND is a variable (`hook_dispatch.rs:269`). Scope: `crates/maos-kernel-core/src`, `crates/maos-bin/src` and `crates/maos-iac/src`; **`maos-iac`'s tokenless kind-7 `log.recall`/distillate rows (`crates/maos-iac/src/adapter/log_recall.rs:112,363,446`, `crates/maos-iac/src/adapter/distillate.rs:311`) and the variable-kind tokenless `insert_frame_event_with_id` (`crates/maos-iac/src/adapter.rs:575`) carry disposition `Call`** — not exempted. The fourth read measured that NO J0 shell or butler session writes them (they come only from the cross-wall traceback one-shot `main.rs:2555`, the J3 digest `:5392` and smoke arms `:5943,5957,8852`), so AC1 step 5 and AC5's butler vector are unaffected; if T0 finds otherwise, they are genuine FR4 findings, recorded with an owner under rule 10. Token arguments in scope are all literal `None`/`Some(…)`, so tokenlessness is decidable (validation round 3) — the doorbell that keeps "every kernel row is classified" true after this commit. A prefix match is refused (`lifecycle.bogus` is a call). **Every other row is a call**, and a call with no token is an `Fr4SchemaViolation` exactly as today (fail-closed: an unknown writer is a call until someone classifies it). `to_fr4_ndjson` emits **call rows only** — its six-key schema, which `audit_query_fr4_smoke.sh` and `v01_evaluator_path.sh` assert, is untouched — and prints `maos: FR4 feed omitted N non-call kernel row(s): <kind>×n, …` to **stderr**. `to_fr4_plain` renders call rows and non-call rows in one table: it has no token column, so a non-call row cannot be read as a mediated call. | *Skip tokenless rows* — hides an unmediated call. *Classify by `FrameOrigin`* — the scheduler writes lifecycle rows as `SpiritAuto`, measured. *Fix the kernel's kinds* — kernel-Δ, reds the 16-0 pin. *Emit non-call rows in NDJSON with `capability_token: null`* — breaks the per-line contract every NDJSON consumer relies on. |
| **D-16-2-H** | Legibility of the plain tables | **`to_plain` and `to_fr4_plain` gain a trailing `intent` column** (last, so every existing fixed-width column keeps its offset). NDJSON unchanged. | *Leave it* — line 4's "halt row and resolution row" are indistinguishable from any other `capability.invocation`/`telemetry.event` row. |
| **D-16-2-I** | 16-1's completion row | **Fixed at origin to D-16-1-E's ratified kind and outcome-bearing intent:** `OperatorDoor::run_command` writes `FrameKind::TelemetryEvent`; intent becomes **`operator.<verb>.<operation_id>:<outcome>`** (§15 R5 — the row must SAY whether it resolved; prefix-compatible with 16-1's lookups, `one_daemon_one_door_16_1.rs:374` `starts_with("operator.pause.")` and `:383`'s length check, and `--intent-contains <operation_id>`); payload gains `spirit_id` and `outcome`. **Outcome mapping, stated (validation round 3):** `OperatorOutcome::Completed` ⇒ the literal `completed` (it carries no code); every other variant ⇒ its typed `code` (44 codes at `bb9f0657`, all `[a-z_]`, none containing `:`). This is the Transparency Log's **resolution row** for a halt resolved through either surface; the reasoning stays in `approval_decision_log` (FR15). | *Leave `CapabilityInvocation`* — a tokenless call row at the Spirit's pid after every door command; reds FR4 for every Spirit. *A new `halt.resolved` row* — a second row for one event; the completion row already exists. *Keep the outcome out of the intent (the author's draft)* — line 4's reader cannot tell a resolution from a refusal (§15 R5). |
| **D-16-2-J** | What exit line 4 shows | **`maos audit query --spirit hello-spirit`** (plain FR4 table, D-16-2-G/H) lists hello-spirit's latest incarnation, including `epistemic.halt … task.acceptance_criterion.ambiguous` and `telemetry.event … operator.halt-resolve.op-<id>:completed`. `journey_j0` asserts the approval-log reasoning `halt=<id>: provided_context: <text>` directly (the line does not render that table). Epic line 4's comment amended. | *Add `approval_decision_log` rows to `maos audit query`* — a second table, a second schema, `maos-audit` at +35. *Replace line 4 with `maosctl halt list`* — drops the audit leg from J0. |
| **D-16-2-K** | The J0 harness | `Pty::spawn` splits its command on whitespace and execs it (`crates/maos-journey-test/src/lib.rs:454-459`), so env never rides the command: **`JourneyWorldBuilder::env(key, value)`** (a generic setter; J0 sets `MAOS_INFERENCE_MODE=replay`, which also overrides the nightly re-record job's job-wide `MAOS_INFERENCE_MODE=record`, `.github/workflows/journey-nightly.yml:87` — J0's seed cassette is not a re-record target, Trap 6). `Pty::send_line(&str)` (writer taken from the master once at spawn, `\r`-terminated) and **`Pty::send_eof()`** (writes `0x04`; `send_line` cannot end the REPL). A harness helper runs `target/debug/maosctl` with the world's env only (never the runner's `HOME`), bounded, returning exit code + stdout + stderr. `journey_j0` runs `maos init` in the world home, spawns `maos shell` under `MAOS_INFERENCE_MODE=replay`, and waits (bounded, harness-side) until `maosctl spirit inspect hello-spirit` reports `Running`. If the child fails `EndpointInUse` (78) — `maos init` probes a free port then releases it — the harness re-mints once (`rm control.json && maos init`) and records it; a second 78 is red. | *Env-override endpoint* — bypasses the discovery the scene demonstrates. *Retry forever* — hides a real collision. |
| **D-16-2-L** | CI | **A NEW job `j0-scene`** (16-1's `one-daemon-one-door` shape): builds `maos` and `maosctl`, runs `cargo test --locked -p maos-journey-test --test journey_j0`, enrolled at **exactly the three sites 16-1 used** (`git show bb9f0657 -- .github`): a `v1-0-ship-gate.needs` entry (`.github/workflows/discipline.yml` `one-daemon-one-door` at `:3646`), and the ship-gate summary's two echo lines (`:3688`, `:3725`). 16-1 did NOT add its job to `aggregate.needs`, `check_ship_gate_completeness.rs` `EXPECTED_GATES` or the gate registry, and neither does this story (J0's steps also run inside `journey-hermetic-tier-1`, already in `aggregate.needs` `:3830`). `journey-hermetic-tier-1`'s build step (`:2180`) also builds `maosctl`. | *Rely on the suite step* — a red J0 scene reads as one line in a suite; epic: "the epic's first CI job". |
| **D-16-2-M** | Honest J0 disclosure | (1) `onb_nfr2_timing.sh`'s response leg runs `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j0/shell-intro.json` and asserts `.introduction` **equals** the cassette's `entries[0].response.text` — a transport/unconfigured fallback string is red. (2) `HelloResponse.transparency_log` carries the path the process actually writes: `run`/`say_hi`/`proceed_with_context` take it as a parameter; the one-shot and the shell pass the resolved TL path. (3) **The inference request carries the token's pid**: `run` builds `InferenceRequest::new(token.spirit_pid, token, …)` instead of the literal `0` (`maos-spirit-hello/src/lib.rs:63-64`), so the loaded Spirit's `inference.call` and cost rows land at its pid (the one-shot's token is pid 0, so its rows do not move). Callers to update: `dispatch_directive`, `crates/maos-kernel-core/benches/hello_spirit_p95.rs:170` (built by `reproducible-build`, `--all-targets`; outside the 16-0 pin, which covers `src/` only), `crates/maos-shell/tests/shell_test.rs:55,78`. | *Keep key-presence checks* — the gate passes on "provider unreachable". *Keep the literal* — J0's honest-disclosure line names a file that does not exist. *Leave pid 0 in the request* — `--spirit hello-spirit` silently omits the Spirit's own mediated calls, the one thing FR4's view is for. |
| **D-16-2-N** | AC4 docs | **`docs-site/docs/run-maos.md`** (the published page) documents, inside its existing `## Start the daemon` section, `maos init` → `control.json`, `maos shell`, `MAOS_INFERENCE_MODE={record,replay,live}` + `MAOS_REPLAY_CASSETTE` (ADR-064), the J0 halt scene with both resolution surfaces, and where the provider key comes from today (`MAOS_ANTHROPIC_API_KEY` / `MAOS_OPENAI_API_KEY`; OS keyring by 16-4). The one locale counterpart, `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md`, is updated in the same commit so the Korean page does not document a scene that no longer exists (no `ja`/`zh-Hans` counterpart exists; `gate-ko-coverage.js` counts page existence per section and `run-maos.md` sits in the non-canonical root section, so no gate forces this — parity is the reason). `docs/maos.dev/run-maos.md` gets the same content. | *Edit only `docs/maos.dev`* — nobody reads it. *English only* — the ko page would contradict it. |
| **D-16-2-O** | How erasure's `not_found` terminal behaves once D-16-2-F retires the pid-0 wildcard | **Operator ruling, 2026-09-15 (§A6 review finding).** `run_uninstall_cascade_inner` derives `not_found` from `!audit_db_path.exists() \|\| (pre_frame_ids.is_empty() && shared_principal_rows == 0)`: an absent-or-empty TL with no shared residue ⇒ `NotFound`; an unknown name on a populated TL ⇒ the pre-existing failed cascade outcome, never a new `NotFound`; a `resolve_spirit_name` failure with shared residue maps to `Vec::new()` and still reaches its signed CoverageGap partial proof. **Residual recorded, NOT fixed by this ruling:** only `shared_tier_principal_row_count` is counted — no private-tier count participates — so a home whose `transparency.sqlite` is absent/empty while `memory.sqlite` holds the Spirit's private-tier principal rows signs "provably nothing to erase" over data that exists; filed to **`16-4-maos-uninstall-and-keyring`** (its `maos purge` owns the private-tier root). | *Mapping unknown-name to a new `NotFound`* — on a legacy TL holding unidentified hello-spirit rows it would sign "nothing to erase" over data that exists — the 13.5b false-success shape. |

---

## Acceptance Criteria (8)

**AC1 — (command) exit lines 1–4 against a real shell root, on a hosted runner.**
`cargo test -p maos-journey-test --test journey_j0` (named in its own CI job, D-16-2-L). In the
world's isolated `HOME`/`MAOS_HOME`/`XDG_DATA_HOME`, test `j0_halt_resolved_over_the_door`:
1. runs `maos init` (exit 0, `control.json` present), spawns `MAOS_INFERENCE_MODE=replay
   MAOS_REPLAY_CASSETTE=<j0 cassette copy> maos shell` in the PTY, waits bounded for `maosctl spirit
   inspect hello-spirit` ⇒ `lifecycle_state: Running` with a pid **≠ 0**;
2. `send_line("@hello-spirit refactor src/main.rs to be more idiomatic")` ⇒ the screen shows
   `[HALT task.acceptance_criterion.ambiguous]` and `halt <id>`;
3. `maosctl halt list --spirit hello-spirit` ⇒ exit 0, exactly one line, whose `halt_id` equals the
   screen's id; `maosctl halt resolve "<that id>" --spirit hello-spirit --kind provided-context --text
   "idiomatic = clippy-clean, no unwrap"` ⇒ exit 0;
4. the screen shows `halt <id> resolved: provided_context` and then the cassette's response text (the
   Spirit proceeded — the cassette entry is consumed by THIS turn);
5. `send_eof()` ⇒ shell exits 0, and the TL holds a `lifecycle.unload` row plus, for hello-spirit's pid, one
   `termination_marker` and one `termination_no_pending` row (nothing was pending — R6); the session's `shell.turn` and
   `cap.issue` rows are present AND the PTY screen (the shell's stdout and stderr share it) contains no `drain timed out` — the shell arm has NO drain-timeout message today (`main.rs:3617-3757`), so it ADDS one — `tokio::time::timeout(5 s)` over the writer, printing `maos shell: audit writer drain timed out after 5s` — matching the run roots' literal (`one_daemon_one_door_16_1.rs:481`); row presence alone passes a 5 s timed-out drain (fourth read); `maos audit query --spirit hello-spirit` ⇒ exit 0, stdout contains a row with
   `epistemic.halt` + `task.acceptance_criterion.ambiguous` and a row with `telemetry.event` +
   `operator.halt-resolve.` ending `:completed`.
Asserted against the TL/approval log directly (`rusqlite`, test-side — kloc-free): exactly one kind-3
row at the Spirit's pid whose payload `halt_id` is the id; exactly one `approval_decision_log` row with
`target = "hello-spirit"`, `capability = "halt.resolve"` and `reasoning = "halt=<id>: provided_context:
idiomatic = clippy-clean, no unwrap"`; exactly one kind-4 row with intent prefix
`operator.halt-resolve.` and suffix `:completed` whose payload `outcome` is `completed` and `spirit_id` is `hello-spirit`; the
Lifecycle Journal carries `Load` and `Halt` for `hello-spirit`.
**Falsifier (run, recorded in the Debug Log, reverted):** replace `invoke_halt` in `shell_host.rs` with a
direct `println!` of the halt line ⇒ step 3 reds on `halt list` (no row) and on `spirit_not_loaded`/`halt_not_pending`.
`tests/integration/onb_nfr2_timing.sh` stays blocking in its job (AC7 makes it honest); the clean-VM
5-minute leg is `ops-live-key-j0-leg`.

**AC2 — hello-spirit is a scheduler-loaded Spirit in shell mode (D-16-2-A).**
- `maos-spirit-hello` exports `HelloSpirit` (`impl Spirit`) and `MANIFEST_TOML`; its existing
  `test_manifest_validates` uses the constant.
- Shell mode: `scheduler.load` → `admit_spirit(pid)` → `scheduler.start(pid)` at start, `scheduler.unload(pid)` at exit
  (R6); the token for every
  hello-spirit turn is issued for that pid (TL `cap.issue`/`capability.invocation` rows at pid ≠ 0).
- **CWD vector:** `maos shell` started from a scratch directory with no `spirits/` reaches the banner and
  answers `@hello-spirit say hi` (red at HEAD: exit 1 `cannot read hello-spirit manifest`).
- **Teardown by construction (D-16-2-A):** the shell body is one `async fn … -> Result` whose caller runs
  the teardown on both arms; review obligation (n) checks that no `?`/`return` in `main` sits between the
  bind and that caller. (No runtime vector: once the manifest is embedded no reachable error remains that a
  hermetic test can trigger without a new env var or a host-global CRL — stated, not faked.)
- `grep -n "JournalAdapter::open" ` inside the shell block ⇒ 0; the admission uses `shared_journal`.
- Butler's `@butler pick` prototype arm is unchanged.

**AC3 — the halt is real (D-16-2-B, D-16-2-E).**
In-process in `crates/maos-bin/tests/shell_host_16_2.rs` (kloc-free) against real kernel objects —
never a registry the test filled and read back:
- `raise_ambiguity_halt` ⇒ `HaltRegistry::lookup_state(id) == Some(PendingResolution)`; one TL kind-3
  row at the Spirit's pid, intent = the tag, payload `halt_id` = id, `policy_id = "hello-spirit.ambiguity"`;
  one Lifecycle Journal `Halt` row for `hello-spirit`; ids from two calls differ and each parses as a ULID
  (`ulid::Ulid::from_string`).
- `run_shell` against a fake `ShellHost` (`crates/maos-shell/tests/`): the `[HALT …]` line is printed only
  after `raise_ambiguity_halt` returned `Ok`; a host that returns `Err` ⇒ `maos: error: …`, no `[HALT`
  line, the shell keeps serving.
- No `shell.halt` string remains in `crates/maos-shell/src`.
- `maosctl halt list` prints `halt_id` and `record`; rows WRITTEN BY THE REAL WRITERS — `invoke_halt`, and
  `terminate_spirit` both with a pending halt and with none — plus a garbage payload ⇒ `raised`+id,
  `termination_marker`+null, `termination_receipt`+id, `termination_no_pending`+`term-…` id, `unparseable`+null with
  the stderr count; **proven red:** swap classification steps (2) and (3) ⇒ the receipt row reads `raised`;
  `cli.rs:714` doc true.

**AC4 — two surfaces, one mechanism, and the Spirit proceeds (D-16-2-C, D-16-2-D).**
- `journey_j0::j0_halt_resolved_in_the_repl`: same set-up as AC1 steps 1–2, then
  `send_line("idiomatic = clippy-clean, no unwrap")` ⇒ `halt <id> resolved: provided_context`, then the
  cassette text; the approval-log row and the kind-4 `operator.halt-resolve.` completion row exist
  exactly as in AC1 (**same rows from the REPL as from the door** — the proof that there is one mechanism).
- In `crates/maos-shell/tests/` against a fake host: while pending, `@hello-spirit …` is refused naming the
  id and dispatches nothing; a host whose state flips to `Resumed` with NO stdin line ⇒ the resolution and
  the proceeding turn render within one tick (the door path, without HTTP); `Terminated` ⇒ `turn
  abandoned (accepted_halt)`, no inference call; `Overridden` ⇒ proceeds without context; a
  `resolve_with_context` refusal `halt_already_resolved` ⇒ printed, and the winner still renders; a late
  clarification after the other surface resolved ⇒ the `your text was not sent` line and no submission (R8);
  **context race (R8):** state `Resumed` with the context appearing two ticks later ⇒ proceeds with it; context
  never appearing within the bound ⇒ `context not delivered`, **no** inference call.
- Fake host, gated reader: every REPL line (banner, refusal, resolution, `shell exiting`) appears in the `&mut Vec<u8>` output — none on the process's stdout.
- **EOF with a halt pending (real kernel objects, `shell_host_16_2.rs`, through the lib fn `main` calls):** unload ⇒ exactly one
  `termination_receipt` row carrying that `halt_id` (after its `termination_marker`), the lib fn's `closed by planned unload`
  line printed only for ids the dry run listed and `lookup_state` no longer knows, `HaltRegistry::lookup_state(id)`
  no longer `PendingResolution`, exit 0 (red against the author's draft, which left it pending).
- **`Err` arm with the halt STILL pending:** pass `run_shell` a writer that fails on the write after the `[HALT` line (the
  only way to reach `Err` with a halt pending — any typed line would resolve it first, D-16-2-D) ⇒ `run_shell` returns
  `Err`, the lib fn still unloads, exactly one `termination_receipt` carries that id, the `closed by planned unload`
  line prints, and the root exits **non-zero** (15-6 D2 — a failed shell is never exit 0).
- **`Err` arm after the halt was RESOLVED** (clarification, then a failing inference turn): zero `closed by planned
  unload` lines and zero receipts for that id — `HaltRegistry::resolve` deletes the halt's metadata
  (`halt/mod.rs:251-259`), so the non-draining dry run (`:367-372`, keyed on metadata) no longer lists it; the only
  termination rows are the marker and the `term-…` `termination_no_pending` receipt. Tests that need `run_shell`'s
  result from a scoped thread check `.is_ok()`/`.is_err()` inside it (`Box<dyn Error>` is not `Send`), or let the fake
  host release the reader's gate so `run_shell` runs on the test thread.
- **Falsifier (run, recorded, reverted):** make the REPL render `resolved` on submit instead of on
  `halt_state` ⇒ the door-path shell test (no stdin line) reds.
- `proceed_with_context` passes the context into the inference request (the fake inference port records
  the prompt) and does not call `is_ambiguous`.
- The port is constructed with no `control.json` and no env pair: `maos shell` with an empty `HOME` ⇒
  `operator door disabled` on stderr, and the REPL clarification still resolves (`approval_decision_log`
  row present).

**AC5 — the audit reads are true (D-16-2-F, G, H, I, J).**
- **Wildcards:** `grep -n '"hello-spirit" =>' crates/maos-shell/src` ⇒ 0; `grep -n 'name == "hello-spirit"'
  crates/maos-audit/src` ⇒ 0. After a shell boot and a later `maos run spirits/butler/manifest.toml` boot
  on ONE TL (both pid 1), `maos audit query --spirit hello-spirit` lists no butler row and
  `--spirit butler` no hello-spirit row.
- **One-shot identity:** `MAOS_ONE_SHOT=hello-spirit maos` writes exactly one kind-19 `hello-spirit` row at
  pid 0; `tests/integration/audit_query_fr4_smoke.sh` and `v01_evaluator_path.sh` step 3 pass unchanged;
  the 13.5b erasure suite and `audit_no_color_test` pass with reseeded fixtures and nothing else loosened.
- **FR4 classifier** (`crates/maos-audit/tests/fr4_classifier_16_2.rs`, kloc-free), each proven:
  a tokenless `inference.call` ⇒ violation; a tokenless kind-7 `lifecycle.load` whose payload lacks a
  string `spirit_id` ⇒ violation; a tokenless `lifecycle.start` whose `spirit_pid` differs from the row's
  pid ⇒ violation; a tokenless kind-7 `lifecycle.bogus` ⇒ violation; a tokenless kind-7 `shell.turn` ⇒
  violation; a tokenless row of each kind in D-16-2-G (a) and one row per **`NonCall`** writer-shape entry,
  built from the measured payloads (incl. `lifecycle.admit`'s `{spirit_id}`-only and `lifecycle.start`'s
  `{lifecycle_event, spirit_pid}` shapes) ⇒ no violation; a tokenless row shaped like a **`Call`** entry (maos-iac `log.recall`) at a Spirit's pid ⇒ **violation**; the inventory test reds when a tokenless kind-7
  `insert_frame_event` site is added to a scratch copy of a scanned file with no table entry (proven red,
  recorded);
  NDJSON over a mixed set emits exactly the call rows, byte-equal to today's schema, and the stderr count
  names each omitted kind; plain renders every row with its intent.
- **Class-wide:** `maosctl audit query --spirit butler` against a live-then-stopped butler root ⇒ exit 0
  (red at HEAD on its `lifecycle.*` rows — T0 records the HEAD red first).
- **Completion row:** a door `pause` and a door `halt resolve` each write one kind-4 row, intent
  `operator.<verb>.op-…:<outcome>`, payload `{operation_id, verb, spirit_id, outcome}`; a refused door command's
  row ends with its refusal code (e.g. `:halt_not_pending`); `one_daemon_one_door_16_1`
  and `operator_door_16_1` stay green.

**AC6 — the harness can type, and the J0 test cannot pass on a dead shell (D-16-2-K, D-16-2-L).**
- `Pty::send_line` exists; the no-wallclock guard stays green over `journey_j0.rs`.
- `JourneyWorldBuilder::env` and `Pty::send_eof` exist.
- `j0_shell_banner_via_pty` requires the exact banner line `maos shell — type @hello-spirit <msg>  (Ctrl-D to exit)` **and**
  that the screen does NOT contain `cannot read` / `admission failed`. **Proven red** with the scout's
  recipe (compiled test, `CARGO_MANIFEST_DIR` → a workspace with no `spirits/`) BEFORE D-16-2-A lands;
  after it lands, the same recipe passes because the manifest is embedded — record both.
- Every `maosctl`/`maos` the harness runs gets the world env and no runner `HOME`: with 16-1's decoy
  `control.json` present in the runner's real home, `journey_j0` is green (run the decoy locally, as
  16-1 T0 did).
- `j0-scene` job exists with its `v1-0-ship-gate.needs` entry and both summary echo lines (D-16-2-L's three
  sites, nothing more); `journey-hermetic-tier-1` builds `maosctl`; the workflow parses
  (`python3 -c 'import yaml,sys; yaml.safe_load(open(".github/workflows/discipline.yml"))'`).

**AC7 — NFR-Onb-2's response leg and hello-spirit's disclosure are honest (D-16-2-M).**
- `onb_nfr2_timing.sh`'s response leg asserts `.introduction` equals the cassette text under replay.
  **Proven red on the ASSERTION, not on boot:** unset BOTH `MAOS_INFERENCE_MODE` and
  `MAOS_REPLAY_CASSETTE` for the leg — the unset mode boots, exits 0 and returns the fallback introduction
  (unsetting only the cassette makes the binary refuse at boot, `replay requires MAOS_REPLAY_CASSETTE`, and
  `set -e` would red the OLD script too — a null proof) — and the equality check reds; record the stderr
  line naming the mismatch. Budgets and the advisory cold-build leg unchanged.
- The hello-spirit `inference.call` row of a shell turn sits at the loaded pid (≠ 0); the one-shot's stays at 0.
- `@hello-spirit say hi` in a `MAOS_HOME` shell and `maosctl run hello-spirit` print a
  `transparency_log` that `test -f` finds.

**AC8 — the page, the tracker and the spec say what the tree says (D-16-2-N).**
- `docs-site/docs/run-maos.md` (+ every locale counterpart the coverage gate counts, T0) and
  `docs/maos.dev/run-maos.md` carry D-16-2-N's content; the docs-site gates that ran green at HEAD run
  green.
- `deferred-work.md:926` closed with its citation; §11 rows filed with owners.
- §12 epic edits (applied at story creation) re-verified by `git diff` at T11; `check-exit-commands` and
  `check-epic-close-coherence` PASS.
- `README.md`'s shell/audit lines (`README.md:224` area) match the behaviour (the REPL clarification;
  `maos audit query --spirit` trail).

---

## Review obligations (§A6)

**(a)** From a scratch directory, `maos init && maos shell`, type the directive ⇒ `[HALT …]` + `halt <id>`;
`maosctl spirit inspect hello-spirit` ⇒ `Running`, pid ≠ 0. **(b)** Resolve from a second terminal with
`maosctl halt resolve` WITHOUT touching the REPL ⇒ the REPL renders the resolution and the cassette turn
unprompted. **(c)** Repeat with a typed clarification ⇒ identical approval-log and completion rows.
**(d)** Resolve the same id from both surfaces at once ⇒ exactly one approval-log row, one typed 409.
**(e)** Replace `invoke_halt` with a `println!` ⇒ AC1 reds at `halt list`. **(f)** Make the REPL render on
submit ⇒ the door-path shell test reds. **(g)** Seed a tokenless `inference.call` at a Spirit's pid ⇒
`maosctl audit query --spirit <s>` exits non-zero; seed a `lifecycle.load` row whose payload has no `spirit_id`, and a
`lifecycle.bogus` row, ⇒ each non-zero. **(h)** `audit_query_fr4_smoke.sh` and `v01_evaluator_path.sh` green, FR4 NDJSON
byte-identical for a call-only DB. **(i)** One TL, shell boot then butler boot ⇒ `--spirit` never mixes
them. **(j)** `onb_nfr2_timing.sh` response leg reds on the fallback string. **(k)** `check-kernel-baseline`
`changed == 0`. **(l)** With a real `~/.maos/control.json` on the reviewer's machine, `cargo test -p
maos-journey-test` shows no `EndpointInUse`. **(m)** `grep -rn '"shell.halt"\|"hello-spirit" => Some(0)\|if name == "hello-spirit"' crates/*/src` ⇒ 0 (the
comment `class.name == "hello-spirit"` in `maos-spirit-hello/src/lib.rs:327` is not a match). **(n)** Read
`main.rs` from the bind to the shell caller: no `?`/`return` bypasses the teardown (D-16-2-A). **(o)** Remove
`kind 29` from the non-call set ⇒ AC1 step 5 reds; add an unlisted tokenless kind-7 writer ⇒ the inventory
test reds; flip a `Call` entry (maos-iac `log.recall`) to `NonCall` ⇒ the AC5 `Call`-shaped vector reds.

---

## 11. Declared cut lines (rule 10 — each names an owner, never a bucket)

| # | Item | Measured evidence | Destination |
|---|---|---|---|
| 1 | **Kernel lifecycle TL rows are kind 7 `capability.invocation`, tokenless, `SpiritAuto`** | `scheduler_loop.rs:357-364`, `journal_lifecycle` `:615-622` | a kernel byte ⇒ **`16-5`** (the epic's one FLAG-Winston re-pin, whose D3 charter is audit durability/truth): **DECIDED in 16-5's preflight with a recorded reason, never silently skipped** (§15 R3). The cost of declining is written into the note: every reader outside `maos-audit` — a SIEM, `verify.py`, a sealed-export consumer — sees a capability invocation without a token on every boot, and a false alarm trains people to ignore the real one. D-16-2-G's writer-shape table is permanent EITHER WAY: TLs written before a re-kind keep their kind-7 rows forever. |
| 2 | **`record_invocation` `Err` swallowed by the surviving `shell.turn` call** | `maos-shell/src/lib.rs:304-308` `let _ =` today; after this story `let _ = host.record_turn(..)` in `maos_shell::run_shell`, with `ShellHost::record_turn -> Result<(), CapError>` passing the error through (sixth read) | **`16-5` AC1** (already owns it; cite amended — one caller left after this story) |
| 3 | **Provider key from the OS keyring** | `anthropic.rs:37`, `openai.rs:7` | **`16-4` AC2** — AC8's page names today's env source; 16-4 amends the page (note written into 16-4 AC2, §12 row 8) |
| 4 | **Legacy TLs with hello-spirit rows and no identity row** — incl. `shell.turn` payloads carrying what an evaluator typed (Art.17 personal data) | D-16-2-F residual | **not cut — stated, with its remedies (§15 R7):** name resolution fails closed (erasing "every pid-0 row" is the defect being deleted); the rows remain reachable by `maosctl audit query --boot <nonce>` (`crates/maos-cli/src/cli.rs:436-438`) and erasable by **`16-4`'s `maos purge`** — note written into 16-4 AC1 (§12 row 15). |
| 5 | **Clean-VM NFR-Onb-2 5-minute leg; live-key transcript** | `onb_nfr2_timing.sh:11-30` | **`ops-live-key-j0-leg`** (operator lane; existing row) |
| 6 | **`maos run` roots never unload their Spirits at shutdown** (NFR-Rel-11's planned half for every root) | the only `.unload(` in `crates/maos-bin/src/main.rs` is a smoke arm (`:6119`); the SIGTERM teardown (`:7880`) and both `--once` returns (`:4498`, `:5461`) exit with SCBs loaded | **`16-3` AC4** — ratified at the round-table (§15 Q-A); 16-3's charter is termination reaching the kernel's receipt path (FR12/FR50 unplanned half; this is the planned half one level up). Written into its section (§12 row 14). |
| 7 | **The butler `maos run --once` audit drain times out** | `main.rs:5466-5478` drops 7 of the channel's owners; probed `audit writer drain timed out after 5s`, exit 0 (fourth read) | **`16-3` AC4** — rule 10(b): it edits that exact teardown to add the unload; written into its AC4 (§12 row 17) |

**Not cut (EFFORT, not SCOPE):** the `impl Spirit` and load order (§1), the CWD-independent manifest,
the single journal handle, `halt list`'s `halt_id` (§2), both wildcard retirements (§4), the FR4
classifier and the completion-row kind (§3 — J0's line 4 cannot pass without them), the proceeding turn
(§5 — the PRD beat), the harness input and the null-control repair (§6), NFR-Onb-2's response assertion
and the disclosure path (§7). Each is required for one of this story's own ACs to be true.

---

## Dev notes

**Reuse, do not reinvent.**
- **Kernel (all `pub`):** `invoke_halt` (`halt/mod.rs:504`) — the SINGLE owner of TL row + journal + registry
  insert; `HaltRegistry::lookup_state` (`:411`); `EpistemicHaltPayload::new` (`crates/maos-domain/src/frame.rs:250`,
  rejects empty id / NaN); `SpiritSchedulerAdapter::{load (:243, async), start, resolve_pid (:178)}`;
  `MemoryManagerPort::read` (`memory/mod.rs:786` impl) with `MemoryTier::Private`,
  `MemoryNamespace::Default`, key `halt_context::<id>` (exact key: `resolver.rs:162-176`).
- **Door:** `OperatorDoor` + `OperatorCommandPort::submit(OperatorCommand, deadline) -> OperatorSubmission`
  (`operator_door.rs` `submit` `:1381`). **Never call `submit` and block on `OperatorSubmission.completion` directly** — that re-implements the withdraw CAS §15 R4 ruled out; call `maos_control::submit_and_wait` (blocking the main thread is safe: `run_command` runs on runtime workers, measured). `OperatorCommand::ResolveHalt` (`crates/maos-control/src/lib.rs:309-314`).
- **Load recipe:** copy `maos run`'s rust-inproc sequence — `SpiritManifestBundle` built inline (`main.rs:4677`
  area), load, admit at the returned pid (`:5276-5296`), start. Do not write a second admission helper;
  16-6 will extract the shared one.
- **Id minting:** `ulid::Ulid::new()` — the kernel's halt-id grammar (§15 R9).
- **Resolver of names:** `maos_audit::resolve_spirit_name(db, name, all_boots)` → `Vec<(boot_nonce, pid)>`;
  `AuditFilter { boot_nonce, spirit_pid, .. }` (the `halt_list_reader` usage, `subcommands.rs:1916-1935`).
- **Identity row writer precedent:** kind 19 rows are matched by `intent == name` (`maos-audit/src/lib.rs:1716`).
- **Journey harness:** `JourneyWorld` env build (`crates/maos-journey-test/src/lib.rs:112-134`, incl. the
  16-1 doorless `HOME`); bounded polling belongs in the harness (`wait_for_screen` `:529`), never the test.

**Files being modified — current state, change, must-not-break:**

| file | today | this story | must not break |
|---|---|---|---|
| `crates/maos-spirit-hello/src/lib.rs` | 382 lines; `run`, `say_hi`, `dispatch_directive`, `is_ambiguous`; literal TL path `:194-196`; inline tests (charged) | `HelloSpirit`, `MANIFEST_TOML`, `proceed_with_context`, TL path parameter | the four FR58 JSON keys; `is_ambiguous`'s anti-over-fire dimension list |
| `crates/maos-kernel-core/benches/hello_spirit_p95.rs` | calls `maos_spirit_hello::run` (`:170`); built by `reproducible-build` (`--all-targets --workspace`) | follow `run`'s new parameters | NOT under the 16-0 pin (`src/` only) — still a kernel-core crate file: touch only the call |
| `crates/maos-shell/tests/shell_test.rs` | `dispatch_directive` callers `:55,78` | follow the new parameters | its existing assertions |
| `crates/maos-shell/src/lib.rs` | 419 lines / 320 tokei; `run_shell` `:204`; halt arm `:310-322`; private `resolve_spirit_pid` `:378`; `run_audit_query` `:162` | `ShellHost` trait; REPL loop (reader thread + tick); pending-halt state; delete `shell.halt` + private resolver | `run_init` (16-1: ENOENT fix, `control.json` mint); banner string; `@butler pick` arm; 15-6 D2 (a failed inference turn exits non-zero) |
| `crates/maos-bin/src/main.rs` | shell block `if shell_mode` `:3617-3757`; door port built inside `if let Some(config)`; one-shot hello admission `:7699-7715` | load→admit→start; embedded manifest; `shared_journal`; port for every door root; error-path teardown; one-shot kind-19 row | 16-1 ordering comments at `is_door_root` (clock base first; below `Arc::get_mut`; after `upgrade_orchestrator`); the `maos: operator HTTP listening on` stderr marker (scraped byte-for-byte by `cert_rotation_trigger_14_2a`); the record-mode flush-after-shell ordering (15-6 P13) |
| `crates/maos-bin/src/shell_host.rs` | — | NEW lib module (`pub mod shell_host;` in `lib.rs`) | — |
| `crates/maos-bin/Cargo.toml` + `Cargo.lock` | no `ulid` | `ulid = "1.1"` (R9; resolves the locked 1.2.1; the lock's `maos-bin` entry changes, same commit) | `deny.toml` `[bans] multiple-versions` — same version as kernel-core's |
| `crates/maos-control/src/lib.rs` | private `fn submit` `:1308-1344` | extracted `pub fn submit_and_wait` + `SubmitOutcome`; server maps it (R4) | every 16-1 response body and status byte-for-byte (`post_surface_16_1.rs`); maos-control stays tokio-free |
| `crates/maos-bin/src/operator_door.rs` | completion row `run_command` kind 7, payload `{operation_id, verb}` | kind 4, payload + `spirit_id`, `outcome` | intent format; "written BEFORE the outcome is delivered" ordering; never-cancelled tasks |
| `crates/maos-audit/src/lib.rs` | wildcard `:1634-1672`; `project_to_fr4` `:603`; `to_fr4_ndjson` `:628`; `to_plain` `:795`; `to_fr4_plain` `:825` | delete wildcard; classifier; NDJSON call-only + stderr count; plain `intent` column | FR4 NDJSON six keys and 64-char token; read-only opens; `resolve_spirit_name`'s 16-1 kind-7/19 identity semantics and timestamp ordering |
| `crates/maos-cli/src/subcommands.rs` | `halt_list_reader` `:1916` | `halt_id` | text anchors other gates count (see 16-1 files table: `resolve_region_home`, `derive_region_pubkey`, `fn dispatch_import`) |
| `crates/maos-journey-test/src/lib.rs` | `Pty` read-only | `send_line`; `maosctl` helper | `Drop` order (`pty_drop_order.rs`); `MAOS_INFERENCE_MODE` removal for cassette-free worlds |
| `crates/maos-journey-test/tests/journey_j0.rs` | 2 tests, banner ANY-of | AC1/AC4/AC6 tests | no-wallclock guard |
| `tests/integration/onb_nfr2_timing.sh` | key-presence response check | cassette-equality check | budgets; advisory cold-build leg |
| `.github/workflows/discipline.yml` | `journey-hermetic-tier-1` `:2156`, build `:2180` | NEW `j0-scene` job + enrolment; `maosctl` in tier-1 build | 16-1's decoy step and `one-daemon-one-door` job |

**Traps, in the order they will bite:**
1. **`load` is async and fires `on_load` synchronously; the shell block runs inside async `main`** — `.await`
   it there, before `run_shell` (which blocks the thread). Do not `block_on` inside the runtime.
2. **`Arc::get_mut(&mut scheduler)`** must still see strong count 1 — the shell block is after it; keep it so.
3. **Every process allocates pids from 1** — never assert a literal pid; assert `≠ 0` and equality with
   `resolve_pid`/`spirit inspect`.
4. **The door's completion row is written even for a refused command** — AC1 asserts `outcome == "completed"`,
   not "a row exists".
5. **`halt_state` flips to terminal BEFORE the resolver's memory write and marker publish** (`resolver.rs`:
   `registry.resolve` first) — `take_context` can race the write. Read the context after the door
   submission's completion returns (REPL path) or retry `take_context` for a bounded number of ticks (door
   path); a still-missing context after the bound renders `resolved: provided_context (context not delivered)`
   and does NOT proceed — never proceed with an empty context.
6. **Seed cassettes carry an all-zero `prompt_sha256`** (drift detection off, `deferred-work.md` 8-15 row) —
   the proceeding turn replays entry 0 regardless of prompt. Do not "fix" the cassette here; a recorded
   cassette would pin the proceed prompt (nightly re-record leg).
7. **`maos init` releases the probed port before the shell binds it** — the one re-mint in D-16-2-K.
8. **16-1's decoy door** holds the runner's real `control.json` endpoint in `workspace-test-suite` — every
   harness child gets the world env (Trap 8 of 16-1).
9. **`maos audit query` silently ignores unknown flags** (16-1 Trap 21) — AC5's selections are `--spirit` only.
10. **A new `src/*.rs` in maos-bin reds `cohort_daemon_smoke_13_5c.rs`** until `SCANNED_SOURCE_FILES` is 21.
11. **Inline `#[cfg(test)]` is charged** in `maos-audit` (+35) and `maos-spirit-hello` — tests go in `tests/`.
12. **A kernel doc-comment byte reds the 16-0 pin** — do not "tidy" `halt/mod.rs` or `resolver.rs`.
13. **The stdin reader thread outlives EOF handling only if you let it** — on EOF the channel disconnects;
    treat `RecvTimeoutError::Disconnected` as EOF, never spin.
14. **The PTY echoes input** (tty echo, not `run_shell`) — screen assertions must look for the response
    strings, not the typed line.
15. **Plain-table consumers.** `to_plain`/`to_fr4_plain` are called from `maos-audit`, `maos-cli`
    (`subcommands.rs`) and `maos-shell`; grep every test and script for the header or a positional `cut`/`awk`
    over those tables before adding the `intent` column (`query_schema_test.rs` pins the NDJSON keys, not the
    plain header — verified). Update any exact pin; never loosen one to `contains`.
16. **Unload at shell EOF (R6) is not free, and its place is fixed.** `unload` fires `on_unload` (a default no-op for
    `HelloSpirit`), calls `capability.revoke_all_for_pid` (`cap.revoke` rows, through the audit channel) and writes a
    `lifecycle.unload` row; it runs AFTER `server.shutdown()` + `drain_started_tasks()` and BEFORE the port drop and the
    audit-writer drain (D-16-2-C's order — the same as 16-3 AC4). Any shell test asserting an exact post-exit row set
    moves with it — update, never loosen.
17. **`terminate_spirit` writes TWO kind-3 rows per pending halt (empty-payload marker, then receipt), and when NOTHING is
    pending it still writes a marker plus a receipt with a synthetic `term-<spirit_id>-<pid>-<ns>` id** (`halt/termination.rs:41-93`)
    — every `halt list` assertion counts `record: "raised"` rows, never lines.

### Project Structure Notes

New files: `crates/maos-bin/src/shell_host.rs` (lib; doorbell 20 → 21), `crates/maos-bin/tests/shell_host_16_2.rs`,
`crates/maos-shell/tests/shell_halt_repl_16_2.rs`, `crates/maos-audit/tests/fr4_classifier_16_2.rs`.
No new crate (`check-workspace-count` 55), THREE new dependency EDGEs as shipped (the ratified single-edge freeze was
exceeded — the deviation is recorded in the Change Log): `ulid` in `maos-bin` (halt-id minting, already in `Cargo.lock`;
the lockfile's `maos-bin` entry changes and ships in the same commit), `parking_lot` in `maos-shell`
`[dev-dependencies]` (the gated reader in `shell_halt_repl_16_2.rs`), and `parking_lot` in `maos-journey-test` as a
REGULAR `[dependencies]` edge (backs `Pty::writer`, `crates/maos-journey-test/src/lib.rs:458`), no new env var
(`check-env-contract` unchanged), no `verbs.rs` change. Kernel-core `src` read, never written.

### References

- [Source: `_bmad-output/planning-artifacts/epics/epic-16-one-daemon-one-door-j0-w1.md` — 16-2 section, exit block lines 1–4, provenance lines 2–4, Kloc asks, Dependencies (Round 6 applied by this story, §12)]
- [Source: `_bmad-output/planning-artifacts/prd/user-journeys.md:293` — J0 "type a clarification, the Spirit proceeds"]
- [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md` — FR4 `:27`, FR15, FR58]
- [Source: `docs/adr/ADR-062-mutating-operator-surface.md` — Consumers (16-2 uses the door for halt operations); `docs/adr/ADR-064-*` `:108` (replay for the J0 shell)]
- [Source: `_bmad-output/implementation-artifacts/16-1-daemon-post-surface-and-verb-retarget.md` — D-16-1-E (completion row), D-16-1-K, §11 row 7, Traps 8/16/21/22]
- [Source: `_bmad-output/implementation-artifacts/deferred-work.md:926`]
- [Source: `xtask/kloc.toml` — `:386, :396, :459, :475, :632, :642`]

---

## Tasks / Subtasks

- [x] **T0 — re-measure; trust nothing in this file** (all ACs)
  - [x] `git status --short` clean; record HEAD; `kloc-check --json` (the six rows); `check-kernel-baseline`; `check-exit-commands`; `check-epic-close-coherence`.
  - [x] Re-run the runtime scout's probes on fresh binaries: HEAD `maos audit query --spirit hello-spirit` exit 1; `halt resolve` ⇒ `spirit_not_loaded`; non-repo-CWD `maos shell` exit 1; the banner null-control recipe ⇒ `ok`. Record each.
  - [x] Record the HEAD red of `maosctl audit query --spirit butler` after a stopped butler root (AC5 class-wide).
  - [x] Build D-16-2-G's writer-shape table from an exhaustive scan of `crates/maos-kernel-core/src`, `crates/maos-bin/src` and `crates/maos-iac/src` (26 `FrameKind::CapabilityInvocation` sites in the first two at `bb9f0657`; maos-iac adds `Call` sites): for each tokenless site record intent (literal or template), pid argument and the payload keys/types it builds. The eight lifecycle shapes are NOT uniform (start/pause/resume/unload carry `spirit_pid`, not `spirit_id`). Also measure whether any PRODUCTION path writes `verb_resolver.rs:128`'s tokenless kind-2 `decision.dispatch` `lifecycle.<Verb>` row (`HumanAuthored`); if one does, stop and re-rule D-16-2-G with its exact shape — never widen to a prefix.
  - [x] List the rows a J0 shell session and a butler `maos run` + door-verb session actually write (kind, intent, token present, payload keys); every tokenless one must be classified by D-16-2-G or be a real defect named in the Debug Log.
  - [x] Confirm `one_daemon_one_door_16_1` / `operator_door_16_1` never select completion rows by kind 7.
  - [x] List every enrolment site 16-1 touched for `one-daemon-one-door` (`git show bb9f0657 -- .github xtask`); D-16-2-L copies them.
  - [x] Measure what `gate-ko-coverage.js` (and ja/zh-Hans) count for `run-maos.md`; decide AC8's locale edits.
  - [x] Confirm the `MAOS_ONE_SHOT=hello-spirit` arm honours `MAOS_INFERENCE_MODE=replay` (15-6) for AC7.
- [x] **T1 — hello-spirit as a Spirit** (AC2, D-16-2-A)
  - [x] `HelloSpirit`, `MANIFEST_TOML`, `test_manifest_validates` on the constant.
  - [x] Shell block: embedded manifest; load → admit(pid) → start; `shared_journal`; error-path teardown.
  - [x] CWD vector; the caller-side teardown structure (no runtime vector — AC2).
- [x] **T2 — the halt** (AC3, D-16-2-B/E)
  - [x] `ShellHost` trait in `maos-shell`; `shell_host.rs` impl (`issue_turn_token`, `record_turn` → `Result`, ULID id mint, `invoke_halt`, `halt_state`, `take_context`); `run_shell` I/O seam (gated-reader tests; `print_line` takes the writer); `ulid` manifest edge; doorbell 20 → 21.
  - [x] Delete `shell.halt`; `[HALT]` + `halt <id>` after `Ok`.
  - [x] `halt list` `halt_id`; `cli.rs:714`.
  - [x] `shell_host_16_2.rs` + fake-host REPL tests.
- [x] **T3 — resolution + proceed** (AC4, D-16-2-C/D)
  - [x] Extract `maos_control::submit_and_wait` + `SubmitOutcome`; server re-mapped byte-for-byte (`post_surface_16_1.rs` green unchanged); a `crates/maos-control/tests/` vector per variant against a fake port.
  - [x] Port built for every door root; `resolve_with_context` via `submit_and_wait(ResolveHalt)`.
  - [x] REPL: reader thread, 100 ms tick, pending state, `@` refusal, render, `proceed_with_context`, `Terminated`/`Overridden`, late-clarification line, Trap 5 bounded context read + both race vectors.
  - [x] EOF: teardown in D-16-2-C's order — `server.shutdown` → `drain_started_tasks` → `scheduler.unload(pid).await` → port + `ShellHost` drop → drop EVERY `audit_tx` owner (reference `main.rs:7941-7970`, incl. `iac`; omit the moved `inference`) and await the writer → locks; the unload-and-print step is a `shell_host.rs` lib fn `main` calls; EOF-pending receipt vector; `halt list` `record` over all FIVE values; no `drain timed out` on the PTY screen (the shell's new 5 s drain-timeout message).
  - [x] Falsifier (render-on-submit) run, recorded, reverted.
- [x] **T4 — audit reads** (AC5, D-16-2-F/G/H/I/J)
  - [x] Delete both wildcards; `run_audit_query` via `resolve_spirit_name` + boot filter; one-shot kind-19 row; reseed fixtures; re-word the diagnostic.
  - [x] FR4 classifier; NDJSON call-only + stderr count; plain `intent` column; update pinned headers.
  - [x] Completion row kind 4, intent `:<outcome>` suffix, payload.
  - [x] `fr4_classifier_16_2.rs` vectors; shell-then-butler one-TL vector; butler class-wide vector.
- [x] **T5 — harness + J0 tests + CI** (AC1, AC4, AC6, D-16-2-K/L)
  - [x] `Pty::send_line`; `maosctl` helper; bounded door-ready wait; one re-mint on 78.
  - [x] `j0_halt_resolved_over_the_door`, `j0_halt_resolved_in_the_repl`; banner test repaired, proven red first by the recipe.
  - [x] AC1 falsifier (`println!` for `invoke_halt`) run, recorded, reverted.
  - [x] `j0-scene` job + enrolment; `maosctl` in tier-1 build; decoy run locally.
- [x] **T6 — honesty** (AC7, D-16-2-M)
  - [x] TL path parameter through `run`/`say_hi`/`proceed_with_context`; one-shot + shell pass it.
  - [x] `onb_nfr2_timing.sh` cassette equality; proven red on the fallback.
- [x] **T7 — docs + spec** (AC8, D-16-2-N)
  - [x] `docs-site/docs/run-maos.md` (+ locales per T0) and `docs/maos.dev/run-maos.md`; README lines.
  - [x] `deferred-work.md:926` closed; §11 rows filed.
- [x] **T8 — sweep**
  - [x] `git diff` the epic: §12 rows present, exit-block commands byte-unchanged (comments only).
  - [x] `cargo fmt --all -- --check`; `kloc-check` (raise only if crossed, `16-2` token); `check-kernel-baseline` `changed == 0`; `check-env-contract`; `check-workspace-count`; `check-exit-commands`; `check-epic-close-coherence`; `bash tests/integration/audit_query_fr4_smoke.sh`; `bash tests/integration/v01_evaluator_path.sh`; `cargo test --workspace --no-fail-fast`.
  - [x] Story row → **`review`**.

---

## 12. Epic OLD→NEW edits (rule 9 — APPLIED at story creation, 2026-09-14)

Every OLD string below was verified to exist exactly once in the file named before replacement;
no row replaces a pin value or a line number that is data.

| # | File / location | OLD (anchor) | NEW (summary; verbatim = the diff) |
|---|---|---|---|
| 1 | epic-16 header | after the Round-5 paragraph | NEW **Round 6** paragraph: 16-2 re-derived at `bb9f0657`, 14/24 cites stale, 3 exit-line premises disproved, pointer to this file |
| 1b | epic-16 exit-block intro | `` printed at `crates/maos-shell/src/lib.rs:272` `` | `` printed by `maos_shell::run_shell`'s `HelloError::Ambiguous` arm (`:311` at `bb9f0657`; re-shaped by 16-2) `` |
| 2 | epic-16 exit block line 2 comment | `then runs line 3` | `…then runs line 3, and waits for the REPL to render the resolution and the Spirit's proceeding turn (16-2 AC1)` — command unchanged |
| 3 | epic-16 exit block line 4 comment | `# 4 — the EpistemicHalt row and its resolution row` | `# 4 — the epistemic.halt row and the operator.halt-resolve completion row (telemetry.event); the reasoning is in approval_decision_log (16-2 D-16-2-J)` — command unchanged |
| 4 | epic-16 provenance Line 2 | `**green-but-vacuous at HEAD:** asserts banner strings only` | ⚠ R6: a NULL CONTROL — passes when the shell fails to start (ANY-of needles); the harness cannot type |
| 5 | epic-16 provenance Line 3 | `fixed by 16-1 AC3 + 16-2 AC2.` | ⚠ R6: 16-1 already moved resolve to the door; what stays red is `spirit_not_loaded` (hello-spirit is never scheduler-loaded) and `halt list` printing no `halt_id` — fixed by 16-2 AC2/AC3 |
| 6 | epic-16 provenance Line 4 | `**red:** no halt / resolution rows for hello-spirit at HEAD (16-2 AC1).` | ⚠ R6: exits 1 `Fr4SchemaViolation` on tokenless rows; resolution in `approval_decision_log`; private pid-0 table in maos-shell — fixed by 16-2 AC5 |
| 7 | epic-16 16-2 section | `*Closes · Δ:* J0 · FR15/FR58 · kernel-Δ 0 · maos-shell +30–60, maos-journey-test +50–120` | the Closes line is APPENDED (`⚠ R6: re-booked in the stories table`), followed by a ⚠ R6 SUPERSEDED paragraph summarising this story's 8 ACs; the four original ACs are KEPT VERBATIM below it (not struck through) so their cites stay auditable, and the paragraph says they are superseded (its wording: "kept verbatim below this note") |
| 8 | epic-16 16-4 AC2 | `` `FIXME(secrets)` at `main.rs:3116` retired. `` | appends: the key-source paragraph of `docs-site/docs/run-maos.md` (written by 16-2 AC8) is amended in the same commit |
| 9 | epic-16 16-5 AC1 | `` `crates/maos-shell/src/lib.rs:265,278`, `let _ =` — propagate it `` | inserts before `— propagate it`: ⚠ R6 — now the ONE `shell.turn` call in `maos_shell::run_shell`; 16-2 deletes the `shell.halt` call |
| 10 | epic-16 stories table 16-2 row | `J0 · FR15/FR58 · kernel-Δ 0 · maos-shell +30–60, maos-journey-test +50–120` | re-booked (⚠ MEASURED re-book, 2026-09-15, operator-ruled — re-book, not re-rule: maos-shell 320 → 573 = **+253 raw**, maos-audit 6916 → 7189 = **+273 raw**; the audit overrun's named driver is `crates/maos-audit/src/fr4_classifier.rs` at 368 lines, and the writer-shape table's size is set by the tree's tokenless-writer count, which the forecast had estimated from the rows J0 happens to write): maos-shell ≤ +253 (measured, ceiling 573), maos-bin ≤ +329, maos-audit ≤ +273 (measured, ceiling 7189), maos-cli ≤ +37, maos-control ≤ +65, maos-spirit-hello ≤ +63, maos-journey-test ≤ +104 |
| 11 | epic-16 Kloc asks | `16-2 wiring ~+20 · 16-5 call sites ~+20` | `16-2 wiring ~+20 (⚠ R6: ≤ +329, 16-2 story §10) · 16-5 call sites ~+20` — one pointer; the stories-table row (10) carries the full re-booking |
| 13 | epic-16 16-5 AC1 | `re-typing the verify signature is outside D3 and outside the grant.` | appends the ⚠ R6 kernel lifecycle re-kind note (§11 row 1) — reworded by the round-table from OPTIONAL to DECIDED-with-a-recorded-reason, decline cost spelled out (§15 R3) |
| 14 | epic-16 16-3 section | after `- **AC3.** Workers get an SCB at spawn …` | NEW **AC4** — NFR-Rel-11's planned half: every `maos run` shutdown path unloads its SCBs through `SpiritSchedulerAdapter::unload` (§15 Q-A); *Closes · Δ* gains NFR-Rel-11 (planned, root shutdown) and maos-bin +20…+40; stories-table 16-3 row re-booked |
| 15 | epic-16 16-4 AC1 | `` `maos init` afterwards is clean. `` (end of AC1) | appends: `maos purge` is the documented erasure route for pre-16-2 hello-spirit TL rows, which no longer resolve by name (§15 R7) |
| 16 | epic-16 exit block line 4 comment + 16-2 R6 paragraph | `operator.halt-resolve completion row` | outcome-bearing intent `…:completed`; the R6 paragraph gains R4/R6/R9 |
| 17 | epic-16 16-3 AC4 | `the pids are COLLECTED from` | inserts, BEFORE the anchor, the butler `--once` drain repair (§11 row 7) — plus the collected-pids/residual text from validation round 3 after it |
| 12 | `sprint-status.yaml` | `16-2-shell-halt-registry-and-j0-scene: backlog` | `ready-for-dev` + summary comment |

---

## 14. Open questions for the operator

**None open.** All four author questions and five round-table findings were closed at the 2026-09-14
round-table (§15), operator-ratified (*"approved, route it to 16-3 as a new AC"*).

---

## 15. Round-table 2026-09-14 — nine rulings, operator-ratified

Convened as a preflight on this file's §14 and the validator's residuals, with measurements taken before
the table sat (Amelia): `ulid` absent from `maos-bin` but present in kernel-core; the server's withdraw CAS
is a private `fn submit` (`crates/maos-control/src/lib.rs:1308-1344`); no `maos run` path calls
`scheduler.unload` at shutdown (only a smoke arm, `main.rs:6119`); `verb_resolver` is referenced only from
`scheduler/mod.rs` (no production kind-2 lifecycle row); `maosctl audit query --boot` exists.

| # | Finding (who) | Ruling | Applied at |
|---|---|---|---|
| **R1** | Sizing (John) | **WHOLE** — line 4 cannot pass without the audit half | frontmatter `split_from` |
| **R2** | Review net (Murat, Dana conceding) | **NON-DEGRADABLE** — a wrong classifier entry is a silent FR4 false-green | frontmatter `review` |
| **R3** | The writer-shape table is a shadow schema; the kernel's rows themselves claim a call to every external reader (Winston, Mary, Vex; Dana's cost objection recorded) | 16-5's re-kind note becomes **decided with a recorded reason**, the decline cost spelled out; the table stays permanently for legacy TLs; the doorbell's unit is the **call site**, not the intent literal (Boundary) | D-16-2-G; §11 row 1; epic 16-5 AC1 note |
| **R4** | The host re-implemented the server's `Queued → Withdrawn` CAS (Yui) | Extract **`maos_control::submit_and_wait` → `SubmitOutcome`**; server and shell host both call it | D-16-2-C; §10; T3 |
| **R5** | Line 4's "resolution row" does not say it resolved (Sally, John) | Completion intent **`operator.<verb>.<operation_id>:<outcome>`** — prefix-compatible with 16-1's lookups; closer to D-16-1-E as ratified (Splinter) | D-16-2-I/J; AC1; AC5; epic exit line 4 comment |
| **R6** | EOF "left pending" violates NFR-Rel-11 (Murat); `terminate_spirit` writes an empty-payload marker row per halt, and a marker plus a synthetic `term-…` receipt when none is pending, so every clean exit would add unexplained rows (Boundary, Grumbal counting) | **Shell EOF unloads hello-spirit** (after the door closes — validation round 3) ⇒ `PlannedUnload` receipts; `halt list` gains **`record: termination_marker \| termination_receipt \| termination_no_pending \| raised \| unparseable`**, classified in that order — nothing filtered | D-16-2-D/E; AC1 step 5; AC2; AC3; AC4 |
| **R7** | Legacy hello-spirit rows include what evaluators typed — Art.17 data unerasable by name (Mary) | Fail-closed stays (Vex); the residual **names its remedies** — `--boot` scoping and **16-4 `maos purge`**, noted in 16-4 AC1 (Paige) | D-16-2-F; §11 row 4; epic 16-4 AC1 note |
| **R8** | A late clarification reads like user error; the door-path context race had no vector (Sally, Boundary) | `your text was not sent` line; race vectors (late context proceeds; missing context never proceeds) | D-16-2-D; AC4 |
| **R9** | Two halt-id grammars in one registry (Amelia, Grumbal relieved) | **ULID**, the kernel's grammar; `ulid` manifest edge in `maos-bin`, no new lockfile crate | D-16-2-B; AC3; §10 |
| **Q-A** | `maos run` roots never unload at shutdown — NFR-Rel-11 for every root (Winston, Vex; Dana preferred a retro row) | **Routed to `16-3` as a NEW AC4** (operator) — 16-3's charter is termination reaching the kernel's receipt path | §11 row 6; epic 16-3 AC4 |

---

## Dev Agent Record

### Agent Model Used

glm-5.3 (zai/glm-5.3) — 2026-09-14, full T0–T8 pass.

### Debug Log References

**T0 (2026-09-14, HEAD `bb9f0657`, fresh `cargo build -p maos-bin -p maos-cli` debug binaries):**

- **Git state:** 4 dirty paths — exactly the story-creation artifacts (story file `A`, sprint-status `M`, epic-16 Round-6 `M`, party memlog `M`). No mid-flight corruption; nothing committed (commits are operator-request-only).
- **Gates:** `kloc-check --json` passed, none over — maos-shell 320, maos-audit 6916, maos-bin 18832, maos-cli 6042, maos-journey-test 508, maos-spirit-hello 365, maos-kernel-core 18935 (all match §Kloc grant). `check-kernel-baseline` PASSED (24474/98, pinned). `check-exit-commands` PASS. `check-epic-close-coherence` PASS.
- **Runtime probes re-run (scratch HOME/MAOS_HOME/XDG_DATA_HOME):** (P2) after one shell session `maos audit query --spirit hello-spirit` ⇒ exit 1 `Fr4SchemaViolation { line: 1, missing_field: "capability_token" }` (first tokenless row = kind-28 vetter admission). (P3) with a live shell, `maosctl halt resolve deadbeef --spirit hello-spirit …` ⇒ exit 1 `spirit_not_loaded … (HTTP 404)`. (P4) scratch-CWD `maos shell` ⇒ exit 1 `shell: cannot read hello-spirit manifest`. (P5) banner null control: compiled `journey_j0` + `CARGO_MANIFEST_DIR=/tmp/fakews16x2/crates/maos-journey-test` ⇒ `j0_shell_banner_via_pty ... ok` in 0.11 s while the child died on the manifest error — the ANY-of `hello-spirit` needle matched the error line; lever = RUNTIME `std::env::var("CARGO_MANIFEST_DIR")` in `Pty::spawn` (`crates/maos-journey-test/src/lib.rs:461`). (P6) butler live AND after SIGTERM: `maosctl audit query --spirit butler` ⇒ exit 2 `FR4 schema violation at line 1: missing field 'capability_token'` (scout said exit 1 for the `maos` form; the `maosctl` form maps to 2 — recorded). (AC7 evidence) say-hi introduction = `Inference transport error — the configured provider is unreachable.` and `Transparency Log: xdg:maos/audit/transparency.sqlite` literal.
- **J0 session TL rows (measured dump):** kind 28 `governance:vetter-key-admission` (pid 0, tokenless, origin Kernel) ×2; kind 7 `cap.issue.Discriminant(5)` (token); kind 7 `cap.verify.ok` (token); kind 9 `infer:ollama->unknown:default` (token); kind 7 `shell.turn` (token); kind 7 `operator.halt-resolve.op-<id>` (tokenless — 16-1's completion row from the refused resolve). Tokenless set ⇒ kind 28 ∈ D-16-2-G(a); the completion row ⇒ D-16-2-I re-kind.
- **Butler session TL rows (measured dump):** kind 7 `lifecycle.admit` `{spirit_id}` pid 1 SpiritAuto; kind 7 `lifecycle.load` `{lifecycle_event, spirit_id, spirit_pid}` pid 1; kind 28 vetter pid 0; kind 7 `lifecycle.start` `{lifecycle_event:"Start", spirit_pid}` pid 1.
- **Writer-shape table (measured, call-site keyed (file, fn, ordinal)):** kernel-core NonCall kind-7 tokenless sites — (1) `scheduler_loop.rs` load ord1 `lifecycle.admit` `{spirit_id:str}` (no-security-manager arm); (2) `scheduler_loop.rs` load ord2 `lifecycle.load` `{lifecycle_event:"Load", spirit_id:str, spirit_pid:num}` ⚠ **3 keys — story's 2-key shape corrected**; (3) `start` `lifecycle.start` `{lifecycle_event:"Start", spirit_pid==row}`; (4) `pause` `{lifecycle_event:"Pause", spirit_pid}`; (5) `resume` `{lifecycle_event:"Resume", spirit_pid}`; (6) `unload` `{lifecycle_event:"Unload", spirit_pid}`; (7) `journal_lifecycle` `lifecycle.journal` `{lifecycle_event:<Verb>, spirit_id}` pid 0; (8) `crash_detector.rs` `lifecycle.crash` `{lifecycle_event:"Crash", spirit_id, spirit_pid, cause}` ⚠ **4 keys — story's 2-key shape corrected**; (9) `self_telemetry.rs` `telemetry.self` payload is a NON-JSON string `self_telemetry: pid=… window=[…]`; (10) `schedule_watchdog.rs` `schedule.fire:` prefix, JSON `{spirit_id, schedule_id, fired_at_ns, compliance_claim_ref, side_effect_token_id, principal_revocability}` (`iac::payload::ScheduleFireRecord`); (11) `hook_dispatch.rs` `hook.budget.` prefix — **KIND VARIABLE** constrained to BudgetWarning 12 / BudgetExceeded 13 (both ∈ set (a)); payload `{spirit_pid, hook_name, wall_ns, cap_seconds, ratio_breached}`; (12) `applier.rs` `spirit.quarantine_requested` `{spirit_id, spirit_pid, quarantine_requested:true}`; (13) `upgrade.rs` `spirit.upgrade` `{spirit_id, predecessor_version, successor_version, policy, outcome, latency_ns, halt_receipts_produced}`; (14) `cli_wrapper/runtime.rs:664` `cli.subprocess.exit` `{event:"cli_subprocess_exit", cli_child_pid, exit_cause, is_crash}` (`_with_sender`, token `None`). maos-bin NonCall — (15) `main.rs:8939` smoke arm `cli.subprocess.exit` `{cli, exit_code, bytes, duration_ms}` (`_with_sender` token `None`; keys differ from (14) — both entries exist, site-keyed); (16) `operator_door.rs:1365` `operator.<verb>.<op-id>` `{operation_id, verb}` — kind-7 tokenless at HEAD, re-kinded to 4 by D-16-2-I (after which it is no longer a kind-7 writer; the doorbell re-derives). maos-iac **Call** — (17) `adapter.rs:574` `_with_id` variable `tl_kind`; (18–20) `log_recall.rs:112/:362/:445`; (21) `distillate.rs:310`.
- **verb_resolver.rs:124** (tokenless kind-2 `decision.dispatch` `lifecycle.<Verb>`): callers = module decl + `pub use` in `scheduler/mod.rs` only; `main.rs:6902`'s `fn resolve_verb` is an unrelated local trait impl. **No production kind-2 lifecycle row — D-16-2-G stands as ruled.**
- **Tokenless rows NOT classified by D-16-2-G (named, none reachable in J0/butler scenes — fail-closed by design):** `resolver.rs:222` kind-10 TaskComplete `orphaned: accepted_halt` (pid 0, only on an accepted_halt resolution); `hot_swap/coordinator.rs:563` + `saga.rs:152` HotSwapAborted (only on an aborted hot swap); `transparency_log.rs:2025` TaskAssign + `:961` TaskComplete (cross-wall ingest / forget cascade, pid 0). If any lands under a queried Spirit's pid it is a genuine FR4 finding.
- **16-1 completion-row selection:** `one_daemon_one_door_16_1.rs:374` `starts_with("operator.pause.")`, `:383` length check — intent-prefix only, never kind 7 ⇒ D-16-2-I's `:<outcome>` suffix is prefix-compatible. `operator_door_16_1.rs` selects no kind.
- **Enrolment sites (current lines):** job `discipline.yml:3581`, `v1-0-ship-gate.needs` `:3646`, summary echoes `:3688` + `:3725`; not in `aggregate.needs`/`EXPECTED_GATES`/gate registry. `journey-hermetic-tier-1` build step `:2180`.
- **ko gate:** report-only (`KO_COVERAGE_MIN` unset); `(root) 4/4 100%` — the `run-maos.md` ko counterpart EXISTS and is counted; `ls docs-site/i18n/` = `ko` only (no ja/zh-Hans) ⇒ AC8 edits the ko counterpart only, as D-16-2-N says.
- **One-shot honours replay:** `ResolvedInferenceMode::resolve` (`main.rs:3541`) replaces every provider in the router map (`:3569-3571`) BEFORE the `MAOS_ONE_SHOT` dispatch (`:5502+`); the hello one-shot composes through the shared `inference` ⇒ replay honoured (AC7 vector valid).

**T1–T8 (2026-09-14, same session):**

- **T1 (D-16-2-A):** `HelloSpirit` + `MANIFEST_TOML` in maos-spirit-hello (`include_str!` of `spirits/hello-spirit/manifest.toml`); `test_manifest_validates` reads the constant; new trait-bound test `hello_spirit_is_a_scheduler_loadable_spirit`. Shell block rewritten as ONE async block assigned to `shell_result` (teardown by construction — no `?` between the door bind and the caller): embedded manifest → `scheduler.load` → `admit_spirit(pid, shared_journal)` → `scheduler.start(pid)` → `authorize_principal_writes(pid)`; the block's own `JournalAdapter::open` deleted (the only remaining `JournalAdapter::open` text inside the block is the comment saying it is gone); tokens issued at the real pid. Live: shell from `/tmp` (no `spirits/`) prints the banner and answers `say hi` with the cassette text, all rows at pid 1, exit 0 — the T0 red inverted. Caller teardown gains `drain_started_tasks` (16-1's run-root pattern).
- **T2 (D-16-2-B/E):** `ShellHost` trait in maos-shell (`spirit_pid`, `issue_turn_token`, `record_turn -> Result<(), CapError>` pass-through, `raise_ambiguity_halt`, `halt_state`, `take_context`); production impl `crates/maos-bin/src/shell_host.rs` (doorbell 20 → 21 in `cohort_daemon_smoke_13_5c.rs`); `ulid = "1.1"` in maos-bin (lockfile entry ships); `run_shell` I/O seam (`input: impl BufRead + Send + 'static`, `output: impl Write`, production `BufReader::new(stdin())` + `stdout()`, tests `Cursor`/`&mut Vec<u8>`); `print_line` takes the writer and PROPAGATES write errors (no panic). `shell.halt` deleted. `halt list` prints `halt_id` + `record` (D-16-2-E order: marker → receipt → `term-` → raised → unparseable, with the stderr count); `cli.rs` resolve doc true. Live probe: `[HALT task.acceptance_criterion.ambiguous]` + ULID at pid 1; kind-3 row in the TL with the payload id.
- **T3 (§15 R4/C/D):** `maos_control::submit_and_wait` + `SubmitOutcome` extracted (the ONE withdraw-CAS copy; `submit_and_wait_with_deadline` seam for the ms-scale vectors); the server's route handler maps it byte-for-byte (`post_surface_16_1` 35/35 green unchanged); 4 variant vectors in `crates/maos-control/tests/submit_and_wait_16_2.rs`. The door PORT is constructed for EVERY door root (only the HTTP bind stays config-gated). `ShellHost::resolve_with_context` submits `ResolveHalt{provided_context}` through the same port. REPL: reader thread → `recv_timeout(100 ms)` tick; pending-halt state machine (`@` refusal naming the id, clarification submit with NO render, R8 late-clarification line, `halt <id> resolved: <kind>` renders from the REGISTRY, `Resumed` ⇒ bounded context wait (50 ticks) then `proceed_with_context` (context in the inference prompt, NO ambiguity re-check), `Terminated` ⇒ `turn abandoned (accepted_halt)`, `Overridden` ⇒ proceeds without context). EOF: `finish_shell_session` lib fn (dry-run pending ids → `scheduler.unload` → `halt <id> closed by planned unload` only for ids the registry no longer knows); caller teardown completed in D-16-2-C's order with the full SIGTERM drop list (minus the moved `inference`) + `tokio::time::timeout(5 s, audit_writer)` with the `maos shell: audit writer drain timed out after 5s` literal. **Falsifier run and reverted** (render-on-submit replacing the tick render ⇒ `door_path_resolution_renders_unprompted_within_ticks` RED — no stdin line, no resolution line). 13 fake-host vectors + 3 EOF/Err-arm kernel-object vectors + the no-`control.json` binary vector, all green. LIVE END-TO-END: `maosctl halt resolve <id> --kind provided-context --text …` over the door ⇒ exit 0; the REPL rendered `halt <id> resolved: provided_context` unprompted and the Spirit proceeded with the cassette text; approval row `halt=<id>: provided_context: idiomatic = clippy-clean, no unwrap`; EOF ⇒ `lifecycle.unload` + marker + `term-…` receipt, exit 0, no drain-timeout line.
- **T4 (D-16-2-F/G/H/I/J):** Both pid-0 wildcards deleted (maos-audit's `if name == "hello-spirit"` branch and maos-shell's private `resolve_spirit_pid`); `run_audit_query` resolves through `resolve_spirit_name` filtered on BOTH boot_nonce and pid; the one-shot evaluator run writes the kind-19 identity row (`{"spirit_id":"hello-spirit","source":"one-shot"}`); diagnostics re-worded (`unknown spirit '<name>' — no admission or load row names it in this Transparency Log`) with maosctl's pinned diagnostic changed in the same commit; 13.5b erasure fixtures reseeded (identity row in both seeders) with the `not_found`-4 contract preserved by mapping no-frames-and-no-shared-residue to `not_found` (a TL that names no such Spirit keeps today's failed outcome; shared-only pre-partition residue still reaches its failed partial proof — all 13 erasure tests green); `audit_no_color_test` reseeds + re-pins (6/6). FR4 classifier `crates/maos-audit/src/fr4_classifier.rs`: set (a) kinds + the writer-shape table (21 entries, call-site keyed, measured payloads) + exact payload matching (prefix refused, unknown keys refused, `spirit_pid`-equals-row enforced); `kind_to_string` gains symmetric 12/13/15/16; `to_fr4_ndjson` omits non-calls with the stderr count; `to_fr4_plain`/`to_plain` gain the trailing `intent` column (widened to 64 so `operator.halt-resolve.op-<24hex>:completed` renders whole). Completion row re-kinded to `telemetry.event` with `operator.<verb>.<op>:<outcome>` intent + `spirit_id`/`outcome` payload (live-verified kind 4 at pid 1). `crates/maos-audit/tests/fr4_classifier_16_2.rs`: 14 vectors incl. the inventory DOORBELL (paren-balanced scanner over kernel-core/bin/iac `src`, `#[cfg(test)]` skipped, alias kinds resolved, token-arg position per writer method, keyed (file, fn, ordinal)) — both directions: no unlisted site, no stale entry; the scratch-site proven-red vector. LIVE: `maos audit query --spirit hello-spirit` after a session ⇒ exit 0, the full trail with intents; butler live AND stopped ⇒ exit 0 (was exit 2 `Fr4SchemaViolation`); one-TL shell-then-butler ⇒ zero cross-rows (permanent vector `shell_boot_then_butler_boot_on_one_tl_never_mix`). Scripts: `audit_query_fr4_smoke.sh` PASS (omits `spirit.admitted×1` correctly); `v01_evaluator_path.sh` **was RED AT HEAD** (its `assert_refused_ansi_free` lost the exit code through a `set +e` command substitution — proven at `bb9f0657` in a clean worktree) — harness fixed, full script PASS.
- **T5 (D-16-2-K/L):** `JourneyWorldBuilder::env`, `Pty::send_line` (CR-terminated) + `Pty::send_eof` (0x04), `run_bounded`/`wait_until` harness helpers (bounded waits live in the harness per the no-wallclock guard). `journey_j0.rs` rewritten: `j0_halt_resolved_over_the_door` (AC1 steps 1–5 incl. the direct TL/approval-log/journal asserts and the no-drain-timeout oracle), `j0_halt_resolved_in_the_repl` (same rows — one mechanism), the banner test repaired to the EXACT line + no `cannot read`/`admission failed` (the fake-workspace recipe: RED-recorded at T0 against the old ANY-of test; PASSES now because the shell really boots from any CWD). **AC1 falsifier run and reverted**: `println!` in place of `invoke_halt` ⇒ the door test reds at `halt list` ("exactly one halt row; got: <empty>"). CI: NEW `j0-scene` job (one-daemon-one-door shape) enrolled at exactly 16-1's three sites (v1-0-ship-gate.needs + two summary echoes), `journey-hermetic-tier-1` builds `maosctl`, workflow YAML parses; decoy-door run: full `cargo test -p maos-journey-test` green with a listener holding an endpoint — 0 `EndpointInUse`.
- **T6 (D-16-2-M):** `transparency_log` parameter through `run`/`say_hi`/`proceed_with_context` (the literal `xdg:` default deleted); the one-shot and the shell pass the resolved TL path; the inference request carries `token.spirit_pid` (the loaded Spirit's `inference.call` + cost rows land at its pid — verified in the J0 TL); callers updated (main.rs, shell, shell_test.rs, `hello_spirit_p95` bench). `onb_nfr2_timing.sh`: response leg runs under `MAOS_INFERENCE_MODE=replay` + the J0 cassette and asserts `.introduction` EQUALS `entries[0].response.text` — **proven red on the ASSERTION first** (both env vars unset ⇒ boot exits 0 with the fallback introduction, equality reds: `actual: … Inference transport error — the configured provider is unreachable.`); full gate PASS (response 5 s blocking, cold build 57 s advisory).
- **T7 (D-16-2-N):** `docs-site/docs/run-maos.md` documents `maos init` → `control.json`, the evaluator shell, the J0 halt scene with BOTH surfaces, `MAOS_INFERENCE_MODE={record,replay,live}` + `MAOS_REPLAY_CASSETTE` (ADR-064), and today's env key source (OS keyring by 16-4); the ko counterpart rewritten in the same commit (parity — the coverage gate stays 42/48 report-only); `docs/maos.dev/run-maos.md` mirrored; README's shell/audit lines extended. `deferred-work.md:926` CLOSED with the full citation. Epic §12 edits verified present (staged, 19 insertions); `check-exit-commands` and `check-epic-close-coherence` PASS.
- **T8:** `cargo fmt --all` then `--check` clean. `kloc-check` crossed in the two forecast crates — measured recovery-lane raises citing the bare token `16-2`: `maos-shell` 500→573 (driver: the pending-halt state machine), `maos-audit` 6951→7189 (driver: the FR4 classifier + writer-shape table + intent column), ratchet test 7/7 green, `over_budget: []`. `check-kernel-baseline` PASSED (24474/98, `changed == 0` — ZERO kernel Δ). `check-env-contract` PASS (0 violations — no new env var). `check-workspace-count` 55/55. Both smoke scripts PASS. Full `cargo test --workspace --no-fail-fast --locked` — see Completion Notes.

### Completion Notes List

- **The J0 capability is real and hermetic.** `cargo test -p maos-journey-test --test journey_j0` drives the full scene twice (door + REPL) through a real PTY against the built binaries: init → shell → ambiguity halt with a ULID → resolution on either surface → the REPL renders it unprompted → the Spirit proceeds with the cassette text → EOF unloads with receipts and exit 0. The direct TL/approval-log/journal asserts pin: one kind-3 row carrying the id, the approval row with the exact reasoning, the kind-4 `operator.halt-resolve.op-<id>:completed` completion row with `outcome`/`spirit_id` payload, journal Load+Halt, `cap.issue`/`shell.turn` at the Spirit's pid, one `lifecycle.unload` + marker + `term-…` no-pending receipt, and NO `drain timed out` on the screen.
- **Both falsifiers run, recorded, reverted** (AC1: `println!` for `invoke_halt` reds `halt list`; AC4: submit-render replacing tick-render reds the door-path vector). The banner null-control recipe was proven red at T0 (old test) and passes now for the right reason (real boot from a fake workspace).
- **ZERO kernel-Δ** (`check-kernel-baseline` `changed == 0`, 24474/98); the one kernel-core file touched is the BENCH caller (`hello_spirit_p95.rs`, outside the `src/` pin, call-site-only).
- **The FR4 classifier is a control, not a cosmetic**: classify-before-validate against an exact writer inventory, fail-closed on everything else; the inventory doorbell re-derives the sites from source and reds both directions.
- **Class-wide repair measured**: butler's `--spirit` view was exit 2 at HEAD (tokenless `lifecycle.*` rows); now exit 0 live and stopped, and shell-then-butler on one TL never mixes rows.
- **Honest surfaces**: `halt list` names every row's `record` (nothing hidden); the completion row says whether it resolved; `onb_nfr2_timing.sh`'s response leg asserts cassette equality; hello-spirit's disclosure field names the file the process actually writes, and its mediated calls land at its pid.
- **Two measured ceiling raises** under the recovery-lane rule, bare token `16-2`, driver named, same commit; `v01_evaluator_path.sh`'s pre-existing exit-capture defect (red at HEAD in a clean worktree) fixed as part of the sweep.

### File List

**New (7)**
- `crates/maos-bin/src/shell_host.rs`
- `crates/maos-bin/tests/shell_host_16_2.rs`
- `crates/maos-shell/tests/shell_halt_repl_16_2.rs`
- `crates/maos-audit/src/fr4_classifier.rs`
- `crates/maos-audit/tests/fr4_classifier_16_2.rs`
- `crates/maos-control/tests/submit_and_wait_16_2.rs`
- `crates/maos-spirit-hello/tests/hello_spirit_16_2.rs` — added by the §A6 review, moving the inline trait-bound test out of the charged `#[cfg(test)]` module per Trap 11

**Modified — production (11)**
- `crates/maos-spirit-hello/src/lib.rs`
- `crates/maos-shell/src/lib.rs`
- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/src/lib.rs`
- `crates/maos-bin/src/operator_door.rs`
- `crates/maos-control/src/lib.rs`
- `crates/maos-audit/src/lib.rs`
- `crates/maos-cli/src/subcommands.rs`
- `crates/maos-cli/src/cli.rs`
- `crates/maos-journey-test/src/lib.rs`
- `crates/maos-kernel-core/benches/hello_spirit_p95.rs`

**Modified — build/deps (4)**
- `crates/maos-bin/Cargo.toml`
- `crates/maos-shell/Cargo.toml`
- `crates/maos-journey-test/Cargo.toml`
- `Cargo.lock`

**Modified — tests/gates (9)**
- `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs`
- `crates/maos-bin/tests/erasure_uninstall_13_5b.rs`
- `crates/maos-shell/tests/shell_test.rs`
- `crates/maos-cli/tests/audit_no_color_test.rs`
- `crates/maos-journey-test/tests/journey_j0.rs`
- `.github/workflows/discipline.yml`
- `tests/integration/onb_nfr2_timing.sh`
- `tests/integration/v01_evaluator_path.sh`
- `xtask/kloc.toml`

**Modified — docs/spec (5)**
- `docs-site/docs/run-maos.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md`
- `docs/maos.dev/run-maos.md`
- `README.md`
- `_bmad-output/implementation-artifacts/deferred-work.md`

> Reshaped 2026-09-20 (Epic-16 retrospective). Same 36 paths and the same Trap-11 note as the prose form this replaces; `check_dev_record_completeness.rs:432-448` only extracts bullet-marked paths, so the prose runs read as an empty File List.

### Change Log

| Date | Change |
|---|---|
| 2026-09-14 | Story created from the epic's 16-2 section at `bb9f0657`. Runtime scout (J0 scene on rebuilt binaries through a real PTY) + static scout (citation audit, design feasibility) + author probes. Premise-level disproofs §1–§7; 14 decisions; 8 ACs; 6 cut lines routed by charter; epic Round-6 edits applied (§12). Sizing WHOLE, flagged. |
| 2026-09-14 | **VALIDATION ROUND, same day (fresh-context validator with runtime probes): 4 must-fix, 8 should-fix, 3 nits — two must-fix were in the AUTHOR'S OWN RULINGS, and every one is applied.** (1) D-16-2-G's lifecycle signature was false for 4 of its 8 writers (`start/pause/resume/unload` carry `spirit_pid`, not `spirit_id` — measured `{"lifecycle_event":"Start","spirit_pid":1}`), which would have red AC1 step 5 on the shell's own `scheduler.start` → re-ruled as an exact writer-shape table over all tokenless kind-7 writers + an inventory doorbell test. (2) `run()` hard-codes `InferenceRequest::new(0, …)`, so the loaded Spirit's calls stayed at pid 0 → D-16-2-M(3). (3) every inference writes a tokenless kind-29 cost row (measured FR4 red) → kind 29 and the other never-a-call kinds (12/13/15/16) classified. (4) AC7's proven-red was a null proof (unsetting only the cassette refuses at boot) → unset both. Also: 16-1's CI enrolment is 3 sites, not `aggregate.needs`; in-process submit must perform the `Queued→Withdrawn` CAS and the shell teardown lacked `drain_started_tasks`; erasure red count 9 (+1 accidental, 7 blocking legs of `check-reza-production-path`) and the 13.5b `not_found` 4 preserved without signing false 'nothing to erase'; bench + `shell_test` callers; harness needs `JourneyWorldBuilder::env` + `send_eof`; the teardown vector replaced by construction; docs rationale corrected (only a `ko` counterpart); §12 row 7 now describes the applied edit; epic header/16-5/exit-intro cites fixed. Budget re-booked: `maos-audit` ≤ +61 may cross. Verified correct by the validator: load after `Arc::get_mut`, no pre-shell watchdog, port args in scope, context key/tier/namespace, no reader of completion rows by kind, one-shot honours replay, kloc headroom figures, doorbell 20. Gates: `check-exit-commands` PASS, `check-epic-close-coherence` PASS. |
| 2026-09-14 | **ROUND-TABLE (§15), operator-ratified (*"approved, route it to 16-3 as a new AC"*).** Nine rulings, three of them reversing the author's draft: shell EOF now UNLOADS the Spirit (NFR-Rel-11) with a `record` field in `halt list` so the kernel's termination marker rows are named, not hidden; one `submit_and_wait` instead of a re-implemented withdraw CAS; ULID halt ids. Also: completion intent carries `:<outcome>`; 16-5's lifecycle re-kind is decided with a recorded reason (external readers see tokenless calls); legacy residual names `--boot` and `maos purge`; late-clarification line and context-race vectors. Q-A: `maos run`'s missing shutdown unload routed to **16-3 AC4**. Epic edits §12 rows 13–16. |
| 2026-09-14 | **VALIDATION ROUND 3 (fresh reader on the round-table's own additions, with probes): 2 must-fix, 6 should-fix, 3 nits — all applied as corrections of ratified intent, no ruling reversed.** (1) R6's `record` classification read raised-first, but a serialized `HaltReceipt` PARSES as `EpistemicHaltPayload` (probed against real `maos-domain`) ⇒ every receipt would read `raised`; order now marker → receipt → raised → unparseable, with a swap-the-steps proven-red. (2) 16-2 unloaded BEFORE the door teardown while the new 16-3 AC4 unloads AFTER it ⇒ one order: shutdown → drain_started_tasks → unload → port → audit drain → locks. Also: the REPL cannot print 'closed by planned unload' before `main` awaits `unload` (registry is the evidence); the no-pending case writes a marker + a synthetic `term-…` receipt, not a marker pair (`termination_no_pending`); the doorbell keys (file, fn, ordinal), parses paren-balanced calls, covers `_with_sender` (token arg 4) and aliased kind paths, and does NOT exempt maos-iac's tokenless `log.recall`/distillate rows (calls); the shell root never drained its audit writer; `Completed` ⇒ `completed` stated; ulid lock entry ships; stale 'pending ratification', 'no new dependency', §12 row 10, epic `halt-<24 hex>`, §11 row order, a broken table row. 16-3 AC4 gains its measured zero-Δ residuals. |
| 2026-09-14 | **FOURTH READ (convergence check on round-3 corrections): 3 ship-blockers, 3 minor — all applied.** (1) the audit drain D-16-2-C told the dev to copy (butler `--once`, `:5466-5478`) itself TIMES OUT (probed, 5 s, exit 0) and AC1's row-presence check would pass it ⇒ copy the SIGTERM root's full drop list, assert no `drain timed out`; the broken butler `--once` drain routed to 16-3 AC4 (rule 10(b)). (2) the doorbell could not record a site as a call, yet maos-iac's `log.recall`/distillate/`_with_id` sites are ruled calls ⇒ every inventory entry carries `NonCall{shape}` or `Call`; `#[cfg(test)]` skipped; measured that no J0/butler session writes those rows. (3) T3 still said unload BEFORE teardown ⇒ fixed, five `record` values. Minor: the `{"error","halt_id"}` clause was false (would read `raised`; unreachable) — deleted, `starts_with("term-")`; `unload` returns `Result`; an I/O seam for `run_shell` and a lib fn for the unload step so the vectors are reachable; §15 R6 text and a dev note that invited a CAS copy. Verified sound: a raised payload can never parse as `HaltReceipt` (4 required fields, `halt.rs:260-273`). |
| 2026-09-14 | **FIFTH READ: 3 ship-blockers, 4 minor — all applied.** (1) the SIGTERM drop list was cited short (`:7938-7960`, 19 names) and omitted `iac` — the hidden owner via `digest_memory` → `memory` → `capability` — and `drop(inference)` is E0382 in the shell arm ⇒ the drain is now specified as an INVARIANT with an oracle (every `audit_tx` owner dropped, proven by no `drain timed out` on the PTY screen), citing the full block `:7941-7970` and naming both hidden owners; after two wrong lists, a list to copy is itself a premise. (2) `impl BufRead + Send` cannot move to the reader thread (E0310) and `stdin().lock()` is not `Send` (E0277) — compiled ⇒ `+ 'static`, `BufReader::new(stdin())`. (3) AC5 said every writer-shape entry ⇒ no violation, which would exempt `Call` sites ⇒ `NonCall` only, a `Call`-shaped violation vector, review (o) extended. Minor: T0 scans maos-iac; PTY screen not 'stderr'; D-16-2-A points at D-16-2-C's order; `ShellHost` gains token/turn methods; `run_shell -> Result<Option<String>, _>`; §12 rows 16/17 order and wording. |
| 2026-09-14 | **SIXTH READ: 2 ship-blockers, 6 minor — all applied; findings now at test-harness depth.** (1) `record_turn` had no return type, so the `record_invocation` swallow could silently move into `shell_host.rs` away from the site 16-5 AC1 cites ⇒ `-> Result<(), CapError>`, swallow stays in `run_shell`, 16-5 AC1 re-cited. (2) the named test input (`Cursor`) reaches EOF at once and EOF unloads by design, so AC4's no-stdin door-path and context-race vectors were unprovable ⇒ a gated reader that withholds EOF, output as `&mut Vec<u8>`. Simplified on the way: the unload step reads pending ids from the registry (`drain_for_spirit_dry_run`, non-draining) on BOTH arms, so `run_shell` returns `Result<(), _>` again. Minor: the shell arm gains a 5 s drain-timeout message with the literal; `print_line` goes through the writer; T1/T2/T3 aligned; maos-iac cites carry `adapter/`. Verified sound: all 28 names in `:7941-7970` in scope and unmoved at the shell arm except `inference`; the reader-thread model compiles and a blocked reader thread does not hang exit (probed); `log.recall` is distinguishable by intent alone. |
| 2026-09-14 | **SEVENTH READ: 1 ship-blocker, 3 minor — all applied.** AC4's `Err`-arm EOF vector (pending halt → failing inference → receipt, exit 0) was unbuildable under the story's own rules: any typed line resolves the pending halt first, `resolve` deletes its metadata, so the dry run skips it; and exit 0 contradicted 15-6 D2 ⇒ split into a failing-writer vector (halt still pending ⇒ receipt, line, NON-zero exit) and a resolved-then-failed vector (no line, no receipt). Minor: 16-5 AC1's grep sentence re-worded (the pass-through in `shell_host.rs` is a caller that does not swallow); scoped-thread `Send` note; dry run pinned after `drain_started_tasks`, before `unload`. Verified: the dry run lists only still-pending halts, and `lookup_state` guards the resolve window; `record_invocation -> Result<(), CapError>`, no new crate edge. |
| 2026-09-14 | **DEV COMPLETE (glm-5.3): T0–T8 executed in one session.** T0 re-measured everything (two writer shapes corrected vs the story's own list; all HEAD reds reproduced; the v01 script found RED AT HEAD — its exit-capture harness bug, fixed). T1–T6 implemented per the ratified decisions with zero kernel-Δ; T5 shipped the J0 journey (both surfaces) + the `j0-scene` CI job at 16-1's three enrolment sites; both falsifiers run+reverted; two measured kloc raises citing the bare `16-2` token; the full workspace suite and every named gate green (see Debug Log). Story → `review`. |
| 2026-09-14 | **EIGHTH READ: CONVERGED — 0 ship-blockers**; 3 wording minors applied (the unload lib fn takes `shell_result` and returns the exit result, so the `Err`-arm exit contract is provable in-process; `exit 0` scoped to the `Ok` arm; a duplicated order phrase removed; 16-5 AC1's grep sentence says the pass-through is the ONLY `record_invocation(` call left). Verified: a write error propagates as `Err` with the halt still pending; the resolved-then-failed vector holds. Ship-blocker curve across the fresh reads: 4 → 2 → 3 → 3 → 2 → 1 → 0. |
| 2026-09-15 | **§A6 CODE REVIEW (non-author, 5 layers: Blind + Edge Case + Acceptance + Test-Infra + runtime re-execution).** All 15 review obligations (a)–(o) EXECUTED LIVE: every one PASS, incl. both story falsifiers and all three obligation-(o) falsifiers red-as-claimed, obligation (d) measured (1 approval row, 2 kind-4 rows — one `:completed`, one `:halt_already_resolved`), obligation (h) NDJSON byte-identical to the `bb9f0657` binary, obligation (l) green with a synthesized decoy `control.json`. Every named gate PASS (`kernel-baseline` changed == 0 at 24474/98; `kloc-check` `over_budget: []`; `fmt --check` clean; env-contract/workspace-count/exit-commands/epic-close-coherence; YAML parses). **Net: 2 decision-needed, 16 patch, 9 defer, 14 dismissed — 0 findings that falsify the capability.** The two decision-needed items are the unratified `run_uninstall_cascade_inner` erasure-terminal rewrite and the §10 budget overshoot (maos-audit +273 measured vs ratified ×1.3 upper +61). Highest-consequence patch: `main.rs:3810`'s pre-existing record-mode-flush `return` now sits BETWEEN the shell body and the NEW teardown, so it bypasses the planned unload and the audit-writer drain — obligation (n) violated by a `return`, which the adjacent comment does not claim to cover. One layer finding REFUTED against the kernel (a "resolved halts get a closed-by line" claim: `resolve` removes metadata, `drain_for_spirit_dry_run` enumerates from metadata). |
| 2026-09-15 | **§A6 REVIEW PATCHES applied (operator-ratified; not a re-litigation). Dependency-edge deviation recorded:** THREE edges shipped against §15 R9's ratified single-edge freeze — `ulid` → `maos-bin` (the forecast edge; halt-id minting), `parking_lot` → `maos-shell` `[dev-dependencies]` (the gated reader in `shell_halt_repl_16_2.rs`), `parking_lot` → `maos-journey-test` as a REGULAR `[dependencies]` edge (backs `Pty::writer`, `crates/maos-journey-test/src/lib.rs:458`). `parking_lot` was already locked at a single version, so no `deny.toml [bans] multiple-versions` breach; no Cargo.toml is changed by this recording. §10 and Project Structure Notes corrected to state the three actual edges and why each is needed. |

### Review Findings

§A6 net, 2026-09-15. Non-author, 5 layers (Blind + Edge Case + Acceptance + Test-Infra + runtime re-execution of obligations (a)–(o)). Severity is consequence-for-the-operator, assigned at triage, not by the layer that raised it.

- [x] [Review][Patch] **RULED 2026-09-15 (operator): RATIFY AS-IS + add the missing tests.** `run_uninstall_cascade_inner`'s erasure `not_found` terminal rewritten on a signed-proof path [crates/maos-bin/src/main.rs:8405-8443, 8520-8524] — `not_found` is now derived from `!audit_db_path.exists() || (pre_frame_ids.is_empty() && shared_principal_rows == 0)`, and a `resolve_spirit_name` failure with shared residue maps to `Vec::new()` feeding a signed CoverageGap. Baseline: resolve error ⇒ hard fail, empty set ⇒ `NotFound`. No AC, decision or §11 cut line named the uninstall cascade; AC5 authorised only "reseeded fixtures and nothing else loosened". **Ruling rationale:** it is a forced consequence of ratified D-16-2-F — with the pid-0 wildcard gone, the old carry-over would either break the clean-home `not_found` contract or sign "nothing to erase" over legacy unidentified rows — and the fail-closed direction is preserved. **Work owed by this ruling:** (1) record the ruling as a new row in the story's Decisions table so the terminal is ratified rather than swept; (2) pin the three branches with tests in `crates/maos-bin/tests/erasure_uninstall_13_5b.rs` — unknown name + existing TL + shared residue > 0 ⇒ failed partial proof, unknown name + existing TL + no shared residue ⇒ cascade `Err` (was `NotFound`/exit 4), and absent-TL ⇒ `NotFound`; the suite currently pins only the old shapes (`not_found_uninstall_has_distinct_terminal_code` with an empty fixture, `shared_only_pre_partition_residue_is_reported` exercising the `!exists` arm). **Residual recorded, NOT fixed under this ruling:** the only residue counted is `shared_tier_principal_row_count(memory_db_path)` — no private-tier count participates, so a home whose `transparency.sqlite` was deleted/rotated/recreated-empty while `memory.sqlite` still holds the Spirit's private-tier principal rows takes the `!exists` arm and signs "provably nothing to erase" over data that exists. File that residual to **`16-4-maos-uninstall-and-keyring`**, whose `maos purge` owns the private-tier root.
- [x] [Review][Patch] **RULED 2026-09-15 (operator): RE-BOOK the epic row to the measured figures.** kloc raises exceed the story's own ratified §10 forecast [xtask/kloc.toml:386, 642; epic-16 §12 row 10 + stories-table 16-2 row] — measured (runtime `kloc-check --json`): `maos-audit` 7189/7189, `maos-shell` 573/573. Arithmetic against §10: `maos-audit` baseline 6916 ⇒ **+273 raw** vs ratified ×1.3 upper **+61** (≈4.5×; the named driver `src/fr4_classifier.rs` is **368 lines** against the +40…+70 the same §10 row forecast for classifier+table). `maos-shell` baseline 320 ⇒ **+253 raw** vs ratified upper **+221** (+32, 14.5% over). The raise *mechanics* are compliant and verified (bare token `16-2` followed by `"`, measured figure, named driver, same commit, `over_budget: []`), and ceilings are set exactly to measured so no headroom is granted. **Work owed by this ruling:** amend §12 row 10 and the epic-16 stories-table 16-2 row from "maos-audit ≤ +61 (may cross) · maos-shell ≤ +221" to the measured +273 / +253, with a one-line reason naming `fr4_classifier.rs` (the writer-shape table's size is set by the tree's writer count, which the forecast estimated from the rows J0 happens to write); then re-verify AC8's "§12 epic edits re-verified by `git diff`" against the amended rows. **Rejected:** compressing the classifier to fit — §10's own rule for the analogous case is "do not compress the REPL to fit", and shrinking an exhaustive inventory to hit a number is the wrong pressure on a non-degradable mediation control.
- [x] [Review][Patch] Record-mode flush `return` bypasses the entire new shell teardown — AC2 "teardown by construction" + obligation (n) violated [crates/maos-bin/src/main.rs:3810] — `return Err("maos: record-mode flush failed: …")` sits between the async shell body (`.await` at :3797) and the teardown (:3822+). Under `MAOS_INFERENCE_MODE=record` with a SUCCESSFUL session and a failing `recorder.flush()`, it skips `server.shutdown()`, `drain_started_tasks()`, `finish_shell_session` (⇒ no `PlannedUnload` receipt; NFR-Rel-11 breached for the shell root), the port drop, the 30-name `audit_tx` drop set and the 5 s writer drain ⇒ queued `shell.turn`/`cap.issue` rows lost. The line is pre-existing (`bb9f0657` main.rs:3741) but this story placed the new teardown BELOW it, so a pre-existing early return now bypasses a teardown that did not exist at baseline. AC2 names "`?`/**`return`**"; the adjacent comment is worded to cover only `?`. Found independently by the Acceptance and runtime layers. Fix: bind the flush error, run the teardown, then return it.
- [x] [Review][Patch] `finish_shell_session` replaces the causal shell error with the unload error, and on the admission-failure path the unload ALWAYS fails [crates/maos-bin/src/shell_host.rs:219-222] — returns `Err("planned unload failed: …")`, discarding `shell_result`, contradicting the invariant restated 20 lines earlier at `main.rs:3798-3800` ("the causal shell failure wins … never allowed to mask the real cause"). Sharpened: `is_transition_allowed` has **no `(Loaded, Unloaded)` arm** (`crates/maos-kernel-core/src/scheduler/control_block.rs:446-454`, explicit `loaded_to_unloaded_rejected` test), so when `scheduler.load` succeeded but `admit_spirit` failed the SCB is `Loaded` and `scheduler.unload` always errors — masking the real admission rejection with "invalid state transition" on exactly the path where the operator needs the cause. Fix: keep the `eprintln!`, return `shell_result`.
- [x] [Review][Patch] `.expect("write closed-by line")` panics on EPIPE and skips the audit drain [crates/maos-bin/src/shell_host.rs:230] — production passes `&mut std::io::stdout()`; Rust ignores SIGPIPE, so `maos shell | head` with a pending halt at EOF makes `writeln!` return EPIPE and the expect panic, unwinding out of the shell arm so the `audit_tx` drop set and the 5 s writer drain never run — the exact row loss D-16-2-C's invariant exists to prevent — and exiting 101 instead of the typed contract. `print_line`'s own doc 30 lines away: "A write error PROPAGATES (never a panic)". Uncovered by tests (`run_finish` always passes an infallible `&mut Vec::new()`). Fix: `let _ = writeln!(…)`.
- [x] [Review][Patch] REPL silently eats the operator's typed clarification when the door answers `SpiritBusy` [crates/maos-bin/src/shell_host.rs:175] — `SubmitOutcome::SpiritBusy` is documented "the budget expired and the withdraw CAS WON: the command never ran" (`crates/maos-control/src/lib.rs:1339-1340`), yet it folds to `Ok(())`; the REPL renders nothing on submit by design, and because the command never ran the registry never flips, so the tick never renders either. The text is consumed and lost with ZERO feedback, the halt stays pending forever, and every subsequent `@` is refused naming the id. Over HTTP the same outcome is an explicit 503. Contradicts §15 R8's own principle ("say what happened to the text instead of silently dropping it"), already implemented for `halt_already_resolved` at `crates/maos-shell/src/lib.rs:440-452`. Reachable whenever any mutating door command holds the per-Spirit lock past the budget. Also untested — no `RiggedDoor` vector reaches this fold. Fix: give `SpiritBusy` a refusal line; keep `HandlerStillRunning → Ok(())` (that one does render later).
- [x] [Review][Patch] `assert_j0_row_set`'s "exactly one" comments are not asserted [crates/maos-journey-test/tests/journey_j0.rs:124-161] — AC1 pins "exactly one `approval_decision_log` row" and "exactly one kind-4 row", but both use `query_row` (errors on zero, silently takes the first of many) with no `COUNT(*)` and no `ORDER BY`; only the kind-3 check uses `assert_eq!(COUNT, 1)`. A double-resolution regression — obligation (d)'s exact concern — passes green. Note the correct kind-4 assertion counts `:completed` rows, not all rows: runtime obligation (d) measured **2** kind-4 rows in the concurrent case (one `:completed`, one `:halt_already_resolved`), which is truthful per Trap 4.
- [x] [Review][Patch] Review obligation (d) has no automated test [crates/maos-bin/tests/shell_host_16_2.rs:857; crates/maos-shell/tests/shell_halt_repl_16_2.rs] — "concurrent resolution from both surfaces ⇒ exactly one `approval_decision_log` row" was proven only by hand (runtime obligation (d) PASS). Existing coverage is the loser's UX against a scripted fake (`halt_already_resolved`) and a single-surface `COUNT(*) == 1`; no vector starts a door resolve and a typed clarification concurrently. A regression that journals the approval row before winning the registry CAS produces two rows and nothing reds.
- [x] [Review][Patch] Doorbell keys writer sites by file BASENAME — duplicate basenames collapse distinct sites [crates/maos-audit/tests/fr4_classifier_16_2.rs:596; key def crates/maos-audit/src/fr4_classifier.rs:106-109] — `scan_tree` reduces each path via `path.file_name()`, so `…/a/mod.rs` and `…/b/mod.rs` are indistinguishable to the `(file, fn, ordinal)` key. Measured: only `lib.rs` and `mod.rs` duplicate across the three scanned roots, and today's `mod.rs` writers use literal non-7 kinds, so no collision is live — but a future tokenless kind-7 site whose `(basename, fn, ordinal)` triple coincides with an existing entry matches it and the doorbell stays GREEN. This is the non-degradable control's own doorbell (§15 R2). The mediation view still fails closed. Fix: key on the workspace-relative path.
- [x] [Review][Patch] Two new compiler warnings in added code [crates/maos-bin/src/main.rs:8438; crates/maos-shell/src/lib.rs:416] — runtime layer diffed warning locations against an isolated `bb9f0657` worktree build: (1) `unused variable: error` on the ADDED line `Err(error) if shared_principal_rows > 0 => Vec::new(),` — the resolve failure's diagnostic is also discarded, so the signed CoverageGap cannot say why per-incarnation enumeration did not run; (2) `unused import: maos_domain::invariants::i1::IntentClass` — the import was ADDED while its only use (`IntentClass::Standard`) was REMOVED to `shell_host.rs`. Fix: `Err(_)` (or fold the detail into the CoverageGap) and delete the import.
- [x] [Review][Patch] `to_fr4_ndjson` leaks one `String` allocation per omitted row via `String::leak()` [crates/maos-audit/src/lib.rs:642-643] — `*omitted.entry(kind.leak()).or_insert(0) += 1` leaks the per-row `String` that `kind_to_string` produced, so N omitted rows leak N allocations (not one per distinct kind). Every caller today is a one-shot process, but `maos-audit` is linked into the long-lived shell/daemon via `maos_shell::run_audit_query`. Fix: `BTreeMap<String, usize>`, or make `kind_to_string` return `&'static str`.
- [x] [Review][Patch] Same violation, two different reported line numbers across the two FR4 renderers [crates/maos-audit/src/lib.rs:639-650 vs :884-903] — `to_fr4_ndjson` increments `line_no` only for emitted CALL rows; `to_fr4_plain` increments for EVERY row and does not count its own header line. Both surface through the same `AuditError::Fr4SchemaViolation { line }` and the same `FR4 schema violation at line {line}` message, so an operator cannot locate the row. At `bb9f0657` both used `idx + 1` over all rows and agreed. Fix: one counting rule (restore "rows fed to the projection" in both).
- [x] [Review][Patch] Committed doorbell red-control is a tautology [crates/maos-audit/tests/fr4_classifier_16_2.rs:664-667] — `assert!(!listed)` matches a fabricated filename `"scratch.rs"` that no `WRITER_SHAPES` entry can ever name, so that half can never fail; only `scan_source`'s paren-balanced detection is actually proven. AC5 specifies "a scratch copy of **a scanned file**", and the epic's binding null-control question is "does this test read the tree, or only what it set itself?". **Mitigated by runtime evidence:** obligation-(o) falsifier 2 appended an unlisted tokenless kind-7 site to the REAL `crates/maos-bin/src/shell_host.rs` and `every_tokenless_kind7_writer_site_has_a_disposition` DID red — the tree-level doorbell genuinely works; only the committed control is weak. Fix: point the red control at a real scanned file's basename.
- [x] [Review][Patch] Three dependency edges shipped against a ratified "one manifest edge" [crates/maos-journey-test/Cargo.toml; crates/maos-shell/Cargo.toml; Cargo.lock] — §10 says "one manifest edge, no new crate" and "`maos-shell` gains no dependency"; Project Structure Notes say "one new dependency EDGE (`ulid` in `maos-bin`)". Actual: `ulid` → `maos-bin`, `parking_lot` → `maos-shell` `[dev-dependencies]`, and **`parking_lot` → `maos-journey-test` as a regular `[dependencies]` edge** (`[dependencies.parking_lot]`, confirmed in the `Cargo.lock` dependency list). `parking_lot` is already locked at one version, so no `deny.toml [bans] multiple-versions` breach. Fix: move the journey-test edge to `[dev-dependencies]` (it backs test-harness code only) and record the deviation in the Change Log.
- [x] [Review][Patch] `init_world`'s documented port re-mint does not exist [crates/maos-journey-test/tests/journey_j0.rs:63-78] — the doc comment promises "on the (rare) case where the probed port is taken between init and the shell bind, the world re-mints ONCE (`rm control.json && maos init`) and records it"; the body runs `maos init` once and asserts. No re-mint logic anywhere in the test or the harness. T5's task list claims "one re-mint on 78" done, and D-16-2-K/Trap 7 name it as the mitigation. On a collision the shell fails to bind and the journey reds at the 15 s banner wait instead of retrying. Fix: implement it or delete the claim.
- [x] [Review][Patch] Committed gate-run debris, unlisted in the File List [_bmad-output/implementation-artifacts/intent-lineage-coverage-report.md:906-940] — five appended `## run @untracked` / `corpus: intent-lineage-corpus-v0` / `120 passed` blocks, boilerplate emitted by an intent-lineage gate run during dev. In no AC, and the file is absent from the story's File List. Fix: revert the hunk.
- [x] [Review][Patch] Trap 11 violated: new test added inline in `maos-spirit-hello` [crates/maos-spirit-hello/src/lib.rs:453] — `hello_spirit_is_a_scheduler_loadable_spirit` was added inside the charged `#[cfg(test)] mod tests` (line 267). Trap 11: "Inline `#[cfg(test)]` is charged in `maos-audit` (+35) and `maos-spirit-hello` — tests go in `tests/`." No budget breach (the crate took no raise, measured 404/1000), and the sibling `test_manifest_validates` edit is a legitimate change to a pre-existing inline test. Fix: move the NEW test to `crates/maos-spirit-hello/tests/`.
- [x] [Review][Patch] Story File List said "New (5)" and listed SIX paths [_bmad-output/implementation-artifacts/16-2-shell-halt-registry-and-j0-scene.md:732] — APPLIED: now `**New (7):**` listing all seven (the seventh is `crates/maos-spirit-hello/tests/hello_spirit_16_2.rs`, created by the Trap 11 fix). `intent-lineage-coverage-report.md` is correctly absent — the debris fix reverted it to its `bb9f0657` bytes.
- [x] [Review][Patch] **Found while applying the patches, not by any layer:** all six new Rust sources were committed mode **100755**, and `crates/maos-journey-test/tests/journey_j0.rs` carried a gratuitous `mode change 100644 => 100755` [git `diff --summary`] — APPLIED: all normalized to 100644, the repo norm (14 of 16 files in `crates/maos-audit/tests/` are 644; the two 755s are pre-existing legacy). Rust sources are not executables, and a recorded mode flip on a tracked file the story only EDITS is outside its scope (AGENTS.md "minimal diff"). `git diff --summary` no longer reports any mode change.
- [x] [Review][Defer] Registry flips to `Resumed` BEFORE the resolver's context write; a failed `memory.write` strands the halt terminal-Resumed forever [crates/maos-kernel-core/src/halt/resolver.rs:143-175; crates/maos-shell/src/lib.rs:603-613] — deferred, pre-existing kernel ordering (Trap 12 forbids touching it). The REPL's bounded 5 s wait converts a kernel write failure into a quiet dead end: the directive is abandoned with only `(context not delivered)` and no abandonment notice, while a retry of `maosctl halt resolve` gets `halt_already_resolved` and the clarification text is unrecoverable. Route: **16-5** (D3 audit durability) or the epic-16 retro, which already owns the sibling NFR-Rel-11 gaps.
- [x] [Review][Defer] `terminate_spirit` receipts skipped for a Spirit loaded-but-never-started and on `on_unload` hook failure [crates/maos-kernel-core/src/scheduler/control_block.rs:446-454; scheduler_loop.rs:497-512] — deferred, pre-existing; **already filed by this story itself** in the `epic-16-retrospective` sprint row (kernel bytes, no story charter). Recorded here because the new `finish_shell_session` is the first caller to hit it on a reachable shell path.
- [x] [Review][Defer] `classify_halt_record` labels the kernel's serde-failure receipt payload `raised` [crates/maos-cli/src/subcommands.rs:2003-2004] — deferred, pre-existing kernel writer. When `serde_json::to_vec(&receipt)` fails, `terminate_spirit` writes `{"error":…,"halt_id":"<ULID>"}` (`crates/maos-kernel-core/src/halt/termination.rs:105-110`, unchanged), which fails `HaltReceipt` but parses as `EpistemicHaltPayload` ⇒ a termination row renders as a raised halt. Unreachable today (all receipt fields serialize). Route: **16-5**.
- [x] [Review][Defer] Journal-write failure after resolver commit yields a resolution with no `approval_decision_log` row [crates/maos-bin/src/operator_door.rs:954-981 over crates/maos-director-surface/src/halt_ui.rs:80-84] — deferred, pre-existing ordering (`submit_resolution` resolves then journals). The door returns `halt_resolution_failed`, the REPL prints the refusal, but the tick then sees `Resumed`, `take_context` succeeds and the Spirit PROCEEDS with zero approval rows. Fail-closed in the intended direction; the reverse side is unhandled. Route: **16-5**.
- [x] [Review][Defer] `run_bounded` can deadlock a chatty child on full pipes [crates/maos-journey-test/src/lib.rs:742-761] — deferred, latent. The bounded loop is `try_wait()` + 50 ms sleep while `stdout`/`stderr` stay `piped()` and are read only after exit; a child writing past the ~64 KiB pipe buffer blocks forever ⇒ deadline ⇒ kill ⇒ `panic!("did not exit within Ns")`, a hang masquerading as a timeout. Today's children are small-output.
- [x] [Review][Defer] REPL clarification has no size cap while the HTTP surface caps at 64 KiB [crates/maos-shell/src/lib.rs:431 vs crates/maos-control/src/lib.rs:215] — deferred. One mechanism, two surfaces, one bound: an HTTP `halt resolve` over `MAX_BODY_BYTES` is refused unread, while the same resolution from the REPL carries an arbitrarily long stdin line verbatim into `Resolution::ProvidedContext`, the private-tier write and the journal `reasoning` string.
- [x] [Review][Defer] `take_context` reads but never removes `halt_context::<id>` [crates/maos-bin/src/shell_host.rs:144-154] — deferred, harmless. The trait name promises consumption; the impl is a pure `MemoryManagerPort::read` and no delete exists on the shell or resolver path, so the key persists in the Private tier (and on disk under `persist` retention) one row per halt. Safe under ULID ids; owed either a post-proceed delete or a rename.
- [x] [Review][Defer] `v01_evaluator_path.sh` exit-capture repair landed in the sweep rather than being routed [tests/integration/v01_evaluator_path.sh:132-165] — deferred as an accepted in-sweep repair. It fixes a genuine pre-existing defect (the helper "failed even against a correct binary — measured red at `bb9f0657` in a clean worktree"), and AC5 authorised only "step 3 passes unchanged". Repo rule 10 wants such items routed with an owner; reverting a real fix is worse, so this records the rule-10 waiver instead.
- [x] [Review][Defer] `cargo test -p maos-journey-test` can silently test a STALE `target/debug/maos` [crates/maos-journey-test/tests/journey_j0.rs:23-41] — deferred, pre-existing harness behaviour; it bit the review's own runtime layer (obligation-(e) falsifier initially PASSED against a binary whose mtime predated the source edit; the red appeared only after an explicit `cargo build -p maos-bin`). CI is safe — `journey-hermetic-tier-1` and `j0-scene` both build the binaries first (`discipline.yml:2180`, `:3610`). Owed: a build-freshness assertion or a `required-features`/build-script guard in the harness.

**Dismissed (14, not persisted as action items).** One layer finding was REFUTED against the kernel: "`finish_shell_session` prints `closed by planned unload` for halts that WERE resolved, and `terminate_spirit` writes a contradictory receipt" — `HaltRegistry::resolve` removes metadata (`halt/mod.rs:259`) and BOTH `drain_for_spirit_dry_run` (`:363-372`) and `drain_for_spirit` (`:326-330`) enumerate FROM metadata, so a resolved halt cannot appear in the snapshot or be drained; `drain_started_tasks().await` also completes before the snapshot, and a typed clarification blocks in `submit_and_wait` until completion, so neither claimed window exists. The story's AC4 premise is correct. The other 13 were noise or covered elsewhere: the fatal-path goodbye `print_line?` masking on EPIPE (exit stays non-zero; same EPIPE killed the diagnostic's destination); the admission-failure path writing lifecycle rows (subsumed — on that path the unload now fails, so the receipts are not written); four test-hygiene nits that are fail-closed or already covered by runtime evidence (`NON_CALL_KINDS` self-iteration, the unasserted NDJSON omitted-kind stderr line which runtime obligation (h) OBSERVED live, the `contains("ambiguous") || contains("lifecycle.")` disjunct, the self-seeded `take_context` unit vector); the scanner's four miss-vectors (variable-bound `None`, `cfg(test)`-on-use over-skip, three-crate scope as ratified by D-16-2-G, comment-occurrence ordinal drift — all fail-closed, none live); `cassette_path()` reading the runner's `MAOS_INFERENCE_MODE` (the child runs replay because j0's `extra_env` overrides the mode, so the seed is never rewritten today); two AC clauses called untested whose behaviour runtime observed live; fixed 1.5 s sleeps in `shell_host_16_2.rs` (stdin pipe preserves order, exit is bounded, and the no-wallclock guard deliberately does not scan maos-bin tests); orphaned daemon roots after the test battery; and `onb_nfr2_timing.sh`'s pre-existing `cargo clean`.

**Not findings — verified strengths.** `journey_j0` reads the tree, not itself: the assertions query the real `transparency_log`/`approval_decision_log`/Lifecycle Journal that `invoke_halt`, `terminate_spirit`, `OperatorDoor::run_command` and the scheduler wrote, driving the real binaries through a real PTY. The FR4 classifier resisted every probed false-green: exact key-set equality (`map.len() != shape.len()`), per-key typing, `SpiritPidEqualsRow` equality with the row's own pid, `Prefix` requiring a strictly longer intent, `Call` entries never exempting, and unknown writers classifying as calls — runtime obligation (g) confirmed all three seeded vectors exit non-zero. `classify_halt_record`'s receipt-before-raised order is sound in BOTH directions (`HaltReceipt` requires `timestamp_ns`/`spirit_pid`/`boot_nonce`/`frame_id`, which a raised payload lacks). The `submit_and_wait` CAS extraction is the single copy, with per-variant vectors, and the server's responses stayed byte-identical (`post_surface_16_1` 35/35). CI enrolment is exactly D-16-2-L's three sites, and the ship gate's `contains(needs.*.result, …)` step covers `j0-scene` automatically.
