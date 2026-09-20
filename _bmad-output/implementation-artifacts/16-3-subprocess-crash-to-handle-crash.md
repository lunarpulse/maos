---
baseline_commit: "**`af96c907`** (Story 16-2 landed). Authored 2026-09-15 from four scouts — a STATIC scout over the Worker spawn/crash/watchdog wiring, a RUNTIME scout that drove butler `maos run` roots (`--once` and SIGTERM) on rebuilt debug binaries in scratch `HOME`/`MAOS_HOME`/`XDG_DATA_HOME`, a RUNTIME PROBE scout in an isolated worktree that killed and hung real Worker children and prototyped the SCB + `handle_crash` shape (compiled, 100/100), and a BUDGET/CI/SPEC scout — then **ten fresh-context validation rounds, converged at round 10 with zero ship-blockers** (round 1: 7 ship-blockers + 19 must-fix; round 2: 7 + ~30, several introduced by round 1's corrections; round 3: 5 + 16; round 4: 3 + 5; round 5: 1 + 5; round 6: 1 + 3; round 7: 1 + 3; round 8: 1 + 3; round 9: 1 + 3; round 10: 0 + 2 — and it RETRACTED round 3's first ship-blocker, a false claim that `kind_to_string` lacked kind names 16-2 had already added; all applied). Round 1 COMPILED AND RAN the observer and runtime signatures on the locked `rustix 1.1.4` / `tokio 1.52.3`. **16-2 moved every `main.rs` teardown cite AC4 carries; all were stale.** Cites below are SYMBOL-first, line second. ⚠ **T0 re-measures. A grant is a global, not a reservation.**"
depends_on: "**`16-1-daemon-post-surface-and-verb-retarget` (`done`)** — the door (`maosctl spirit inspect`/`unload` on a Worker SCB), `drain_started_tasks`, the store locks, D-16-1-V (a Worker's `CliSubprocessExit` revoke row is kept even after an unload revoke). **`16-2-shell-halt-registry-and-j0-scene` (`done`)** — `finish_shell_session` (the unload-at-exit loop this story generalises), the FR4 row classifier (`crates/maos-audit/src/fr4_classifier.rs`) and its writer inventory, D-16-2-C's teardown order. **`16-0`** — content-hash tripwire and measured re-pin discipline; the review-authorized optional-token repair changes two kernel files and re-pins 24474 → 24477 in the same change."
blocks: "**`17-1`** (AC4 re-routes the T3 Worker spawn through `spawn_t3`; this story's supervision sits ABOVE the spawn primitive — note written into 17-1 AC4, §12; its exit line `maos audit query --spirit worker` resolves because the first Worker of a root is `worker`, D-16-3-D), **`19-3`** (AC3's 'no Worker SCB, pid 0' rationale changes — note written, §12), **`16-5`** (kind-1 crash-report rows, raw kind-1 kernel events at a real pid and the redactor's `sk-` rule join its recorded decisions — note written, §12), **`16-6`** (inherits the pre-start/partial-load error-return residual — note written, §12), **`20-3b`** (NFR-Rel-1/2/11 coverage-matrix `gates:` rows), **`epic-16-retrospective`** (five residual items added, §11)."
spec_alignment: "⚠ **THE EPIC IS AMENDED IN THE SAME COMMIT (rule 9, Round 7 — 16-3 section, stories-table row, Kernel-Δ, Kloc-asks and doorbell sentences, notes in 16-5 AC1, 16-6 AC1, 17-1 AC4 and 19-3 AC3).** Measurement disproved the premises behind **all four** of the section's ACs. AC1's `sh` fixture at T1 cannot reach the maos-bin path it exists to prove (T3 floor, host grant, `select_worker_cli` — three refusals, measured), and its single '`task.orphaned` within 2 s' merges FR12's two floors (detection ≤2 s; `task.orphaned`/NACK ≤5 s). AC2's watchdog is **never running while any Worker lives** (spawned only in the serving tail) and nothing in production ever advances a Spirit's progress stamp. AC3's 'pass the real pid into `wait_and_finalize`' cannot make detection ≤2 s: the wait runs after the pump, and a pipe-inheriting grandchild delays EOF for its whole lifetime (measured 5.0 s via the bridge; unbounded via `maos run`) — and `handle_crash` is async while the Worker path is a sync fn on a runtime worker. AC4's cites were all stale, its unload position races an in-flight `on_idle`, its butler proof is vacuous (butler halts at pid 0), and it missed three roots (standalone `[cli_wrapper]`, `cohort-a2a-daemon` host B, SIGTERM during a Worker). Story 5.3 is `done` with its crash/hang corpora, floor tests and subprocess-form test **never shipped** — the NFR-Rel-1/2 floors have never been measured. The rulings are the Decisions table; the applied epic edits are §12."
split_from: "Not a split. Authored from `epics/epic-16-one-daemon-one-door-j0-w1.md` 16-3 section. **Sizing: WHOLE — OPERATOR-RATIFIED 2026-09-15 (§15 Q1).** The obvious seam is `16-3b` = AC4 (root-shutdown unload + drain repair + butler/Mira pid). It is refused because the two halves edit the SAME teardown blocks and the same shutdown signal path: AC3 gives Workers SCBs, so every Worker-bearing root's shutdown must stop and unload them (AC4's machinery), and SIGTERM-during-a-Worker is where both meet. Split, one story builds a half-teardown the other rewrites. Measured duration **10–14 d**."
kernel_grant: "**REVIEW-AMENDED 2026-09-16:** narrow authorized kernel delta. `TaskAssignmentRecord.capability_token` is now `Option<TokenId>`, and `CrashDetector` preserves a missing token as SQL NULL plus an empty `in_flight_tokens` array instead of fabricating `TokenId([0;16])`. No new kernel API or lifecycle transition."
kloc_grant: "**REVIEW-AMENDED 2026-09-16, FORMATTED MEASUREMENT:** maos-bin 21048, maos-audit 7341, and maos-kernel-core 18938 are pinned as exact zero-headroom ceilings; kernel physical lines 24474 → 24477 with both changed-file hashes and the derived set hash re-pinned. All remain within the operator-authorized story bounds; no unmeasured raise."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs`) is the machine-checked set; `equiv` is prose, NOT a match token. Keep the literal `allowlist {`."
dev_model_used: "anthropic/claude-opus-5"
review: "§A6 full-layer net BINDING (Blind + Edge Case + Acceptance + Test-Infra + non-author runtime). Not marked ◆ in the epic, but this story **edits the FR4 row classifier** (a control whose failure mode is a silent false-green — 16-2 §15 R2 precedent), **makes the crash floor the first NFR-Rel-1/2 measurement the project has ever run** (a corpus that passes vacuously certifies an unmeasured floor), and **installs a process signal listener and a per-Worker state machine** ⇒ **NON-DEGRADABLE — OPERATOR-RATIFIED 2026-09-15 (§15 Q5)**. E15-A6 binding by name — *does this test read the tree, or only what it set itself?* Obligations (a)–(t) under **Review obligations**."
---

# 16-3 — A Worker that dies or hangs reaches the kernel, and every root closes its Spirits on the way out

Status: done

> **The capability:** *An operator runs a Worker under `maos run` (standalone, inside a topology, or on a
> cohort host B). If the Worker process is SIGKILLed — even while a grandchild still holds its stdout — the
> kernel knows within 2 s: the Worker's SCB is torn down, its halts get receipts, its in-flight task is
> reported `task.orphaned` and dispositioned within 5 s. If it goes silent past its progress threshold it is
> reported `task.stalled` within 60 s; if it is merely slow but still printing, it is not. `maos audit query
> --spirit worker --format ndjson` shows the mediated rows and names the rest. And when any `maos run` root
> stops — SIGTERM, `--once`, or a Worker run that ends — its Workers are stopped and every Spirit it loaded is
> unloaded first, so a pending halt leaves a receipt, not a gap.*

**Closes:** FR12 (detection ≤2 s, `task.orphaned` ≤5 s, hung → `task.stalled`) and FR50 (manifest
`[on_crash]` disposition applied) **for Workers, on all three Worker-spawning roots** · NFR-Rel-1 and
NFR-Rel-2 floors **measured for the first time** (Story 5.3 shipped ≥1 smoke tests under their job names) ·
NFR-Rel-11's planned half at `maos run` root shutdown (epic AC4, 16-2 §15 Q-A) · `deferred-work.md:938`
(`run_bounded` full-pipe deadlock, routed here) · butler's and Mira's halts attributed to pid 0 (host-side,
at origin).

**Does NOT close, and says so:**
- **FR12's "IAC frames *to in-flight task originators*"** — the kernel journals `task.orphaned` and the
  disposition as Transparency Log rows; no mailbox delivery reaches the originator. The floors are measured on
  the journaled rows. §11 row 1.
- **Killing a stopped or crashed Worker's descendants** — a bare spawn has no process group
  (`runtime.rs:461`); 17-1 AC4's `spawn_t3` container owns the process tree. Until then a SIGTERM'd root whose
  Worker left a pipe-holding descendant exits on a bounded grace WITHOUT its door shutdown and audit-channel
  drain (receipts are still written — D-16-3-M). §11 row 2.
- **Executing `[cli_wrapper] recovery_policy`** (`respawn_fresh`) — zero production callers. §11 row 3.
- **Receipts for a Spirit loaded-but-never-started, whose `on_unload` exceeds its budget, or whose halt a hot
  swap drains** — kernel bytes, on the `epic-16-retrospective` row. §11 rows 4, 14.
- **Pre-`start` and partial-load error returns inside the run block** — `16-6`, which extracts that block. §11 row 5.
- **Audit-writer drain timeouts in the `MAOS_ONE_SHOT` arms** (four measured, incl. `hello-spirit`) — outside
  this story's roots. §11 row 13.

---

## What this story actually is

The epic describes a pid swap: pass the real pid into `wait_and_finalize`, push a record, call
`handle_crash`. Measurement says the swap is the smallest part and, alone, would ship a floor that is
false. At `af96c907`, in the order a dev will hit it:

1. **The fixture the epic names cannot run the code under test.** (§1)
2. **Detection after the pump is bounded by the last process holding the pipe, not by the death.** (§2)
3. **No Worker ever has a scheduler it can reach from where it runs, a watchdog that is running, or a
   progress stamp anyone writes.** (§3)
4. **The corpora cannot live where the CI jobs look, and Story 5.3's never did.** (§4)
5. **Real pids make the Worker's own `--spirit` view die on its output rows.** (§5)
6. **AC4's roots, order, proof Spirit and cites were all wrong, and three roots were missing.** (§6)

So this is **the story that makes subprocess supervision real** — SCB, exit observation independent of
pipes, progress, the two floors measured — and that closes every root's Spirits at shutdown.

---

## 1. 🔴 THE EPIC'S FIXTURE CANNOT REACH THE WORKER PATH

- `maos run` with a `[cli_wrapper]` manifest runs `run_cli_wrapper_manifest`
  (`crates/maos-bin/src/worker_spawn.rs:380`). Probe scout, `env -i … maos run <m> --once`:
  - `sh` at `tier = "T1"` ⇒ rc 1 `CliWrapperSpirit requires sandbox tier = t3` (`resolve_cli_wrapper_tier`,
    `crates/maos-kernel-core/src/lifecycle/cli_wrapper/admission.rs:266`, floor `:275`).
  - `sh` at T3 ⇒ rc 1 `tier not granted … for image 'sh'` (built-in grant is `worker-cli-fixture` only,
    `builtin_fixture_grant` `worker_spawn.rs:157-164`). A `MAOS_HOST_GRANTS` entry would still fail at
    `select_worker_cli` (`crates/maos-bin/src/worker_cli.rs:835-851`: fixture | codex | claude).
- The 8.12 precedent (`crates/maos-kernel-core/tests/cli_wrapper_bridge_8_12.rs:132-137`) calls
  `spawn_and_bridge` directly in kernel-core and bypasses every maos-bin gate.
- **`worker-cli-fixture` has no crash or hang mode** (`spirits/worker/src/bin/worker-cli-fixture.rs`: probe
  envelope, or three canned lines, exit 0). Admission runs it with `argv_prefix` + `--maos-bridge-probe`
  (`admission.rs:71-72`, 2 s bound) — so any mode flag must be ignored on the probe path. Its package is
  `worker`.
- "T3" is a label: `spawn_and_bridge` is a bare `Command::new` (`runtime.rs:461`) — no bwrap, podman,
  process group or pdeathsig (11.4b CATCH-0; 17-1's premise).

## 2. 🔴 DETECTION AFTER THE PUMP IS BOUNDED BY THE PIPE, NOT BY THE DEATH

- `run_cli_wrapper_manifest` calls `pump_to_journal` (`worker_spawn.rs:719`), which loops `rx.recv()` until
  EOF on **both** streams (`runtime.rs:567-630`), and only then `wait_and_finalize` (`:728`), whose
  `child.wait()` (`runtime.rs:647-653`) is where the exit cause becomes known. Its revoke closure is `FnOnce`,
  called exactly once (`:638-677`).
- Probe, kernel bridge, `sh -c 'sleep 5 & kill -9 $$'`: the direct child dies at ~0 s; `pump_to_journal`
  returns after **5.0008 s**. Via `maos run`, a grandchild holding stdout: still blocked 10 s later. External
  `kill -9` on a non-forking child: `maos run` exits 52 ms later.
- Real agent CLIs fork. So "kill → `handle_crash` ≤2 s" is structurally false for the population the floor
  is about, however fast `handle_crash` is (prototype: 0.36 ms).
- **Validator probe (the ruling's basis):** `pidfd_open` + `waitid(PidFd, EXITED|NOWAIT)` returned 216 µs after
  `kill -9` with `terminating_signal = Some(9)`, and `child.wait()` afterwards still returned signal 9; with
  `sh -c 'sleep 6 & exec sleep 30'` on piped stdout, `waitid` fired at the kill (0.30 s) while stdout EOF arrived
  at 6.0 s. After the reap, `waitid` ⇒ `ECHILD`, `pidfd_open` ⇒ `ESRCH`. **The observer races the bridge's
  reap: 106/200 runs returned `ECHILD`** on fast exits. No global child reaper exists in `crates/`.
- **`handle_crash` is async and needs runtime context** (`tokio::spawn` into `active_handlers`,
  `crash_detector.rs:117`); `run_cli_wrapper_manifest` is a sync fn called directly from async `main` on a
  runtime worker thread on paths (a) and (b) (`main.rs:4709`, `:4279`) — `Handle::block_on` there panics.
  `handle_crash` returns `NotLoaded` when a concurrent `unload` removed the SCB first (`:95-100`).
- `HandleCrashReport.detection_latency_ns` measures `handle_crash`'s own run, **not time since death**; the
  kill instant is journaled nowhere. A latency floor needs a harness stamp. `task.orphaned` is written before
  `lifecycle.crash` (`:163` vs `:204`).

## 3. 🔴 NO WORKER HAS A REACHABLE SCHEDULER, A RUNNING WATCHDOG, OR A PROGRESS STAMP

**Three production Worker spawners** (static scout), none holding the kernel:

| path | site | context | scheduler / crash detector passed? | ProgressWatchdog alive during the Worker? |
|---|---|---|---|---|
| (a) standalone `[cli_wrapper]` | `main.rs:4700-4749` | async `main`, runtime worker thread | exist (`scheduler` `:2857`, `crash_detector` `:2897`, `set_crash_detector` `:2911`) — **not passed** | **no** — spawned only at `:7989`, after the run block |
| (b) topology `[cli_wrapper]` member | `main.rs:4279`; not-completed ⇒ `return Err` `:4300-4305` inside the load loop | same, inline during topology load | same | **no** (Workers run during load) |
| (c) `cohort-a2a-daemon` host B | `delegation.rs:682-683` (`spawn_blocking`) via `HostBWorkerContext` (`:611-629`, built by `host_b_worker_context` `main.rs:9649`) | blocking pool; one Worker at a time (`delegation.rs:724-757`) | **no** — neither `host_b_worker_context` nor `run_cohort_a2a_daemon` (`main.rs:9710-9734`) takes a scheduler or a halt registry | **no** (the arm returns at `:7633` before `:7989`) |

(`maos-bench/src/harness/j1.rs:232` is bench-only.) And:

- **Everything is written at pid 0**: `admit_cli_wrapper_journaled(…, 0, …)` `:575` (probe — stays 0, it precedes
  the Worker), the mint `:633-646`, `pump_to_journal(&tl, 0, …)` `:719-725`, `wait_and_finalize(&tl, 0, …)`
  `:728`, the revoke `:730`, the verdict row `:859-866`. Probe: a SIGKILLed Worker's whole TL is three tokenless
  pid-0 rows (kind 21, kind 7 `cli.subprocess.exit signaled(sig=9)`, kind 4 verdict).
- **`last_progress_iac_ns` has no production writer** (only `ScbTracker::update_last_progress_iac`,
  `crates/maos-kernel-core/src/iac.rs:55-65`, on spirit-origin mailbox delivery, and the smoke arm `main.rs:6382,6437`).
  Workers send no IAC. So the watchdog measures from SCB creation.
- Nothing acts on `task.stalled`; a silent Worker blocks `maos run` forever in the pump (probe: 71 s+).
- **Scopes.** The run block is `main.rs:3984-5626` at `main`'s top scope; non-`--once` single-class runs (`:5624`)
  and non-`--once` topologies (`:4673-4688`) fall through to the serving tail (`:7945+`), and bare `maos` reaches
  the serving tail without entering the run block. The `MAOS_ONE_SHOT` dispatch lies between them (`:5639-7736`,
  cohort arm `:7633`). `cancel` is created only at `:7945`. Tasks spawned inside the run block and held in its
  locals are **detached** at `:5626` (a dropped `JoinHandle` detaches; a dropped `CancellationToken` does not
  cancel) — so anything that must reach the serving tail must be declared before `:3984`.
- **SIGTERM to a root while a Worker runs** exits 143 by default disposition, no drain, and **the Worker is
  reparented to init** (probe). tokio signal delivery is a `watch` broadcast (`tokio-1.52.3/src/signal/registry.rs:70-101`):
  every live stream gets one SIGTERM, streams created later miss it, and a registration never restores the default
  disposition.
- **The one-shot arms' drains already time out** — measured at `af96c907`: `smoke-epic-4` (`:6117`, drops only
  `audit_tx`/`inference`/`capability` `:6114-6116`), `smoke-spirit-5`, `smoke-supervision-5` and `hello-spirit`
  each print `audit writer drain timed out after 5s` (~5.06 s). §11 row 13.

## 4. 🔴 THE CORPORA CANNOT LIVE WHERE THE JOBS LOOK — AND 5.3's NEVER EXISTED

- `nfr-rel-1-crash-detection-2s` (`.github/workflows/discipline.yml:1196-1207`) runs `cargo test -p
  maos-kernel-core --test crash_detector_in_process_panic --release`: a `PanicSpirit`, sleep 300 ms, assert SCB
  removed and **≥1 halt receipt** (`crash_detector_in_process_panic.rs:103-125`) — no record, no `task.orphaned`,
  no latency. `nfr-rel-2-hang-detection-60s` (`:1210-1221`) runs `progress_watchdog_smoke`: progress **backdated
  30 s by store**, assert ≥1 `TaskStalled` (`progress_watchdog_smoke.rs:88-102,119-122`) — no hang.
- kernel-core cannot link maos-bin, so a corpus proving the maos-bin wiring must be a **maos-bin** test; the
  jobs build no `worker-cli-fixture`. `aggregate.needs` `:3811-3812`. `workspace-test-suite` (`:3497`, 45 min,
  debug, no `--test-threads` cap `:3576`) runs every un-ignored maos-bin test; maos-bin is known red under default
  parallelism there (comment `:1990-1991`).
- **Story 5.3 is `done` with its promised evidence absent:** `crates/maos-eval/fixtures/crash-corpus-v0`,
  `hang-corpus-v0`, `crash_detector_2s_floor.rs`, `progress_watchdog_60s_floor.rs`,
  `crash_detector_subprocess_form.rs` (task 7.6 ticked) — none exist. `SubprocessSupervisor`
  (`crates/maos-domain/src/supervision.rs:160`) has no production impl; `OsProcessChildSupervisor` does not exist;
  the `on_child_exit` callback `crash_detector.rs:82-83` documents does not exist.
- **The PRD sets two floors** (`prd/functional-requirements.md:40`, `prd/non-functional-requirements.md:20-21`):
  detection ≤2 s at ≥99/100 **and** `task.orphaned`/NACK ≤5 s at ≥99/100; hung = *"no progress IAC for >30s"* →
  `task.stalled` ≤60 s at ≥48/50. The epic's "`task.orphaned` within 2 s" merges them; its "counts, not wall-clock
  per kill" cannot express a latency floor.
- `tests/coverage-matrix.yaml:1376,1386,1391` — NFR-Rel-1/11/2 have `gates: []`; the job ids are not in
  `xtask/gate-registry.toml`, so filling them adds three "unknown gate" violations (33 → 36). Routed (§11 row 15).

## 5. 🔴 REAL PIDS MAKE A WORKER's OWN `--spirit` VIEW EXIT 1

- Once rows land at the Worker's pid, its `cli.subprocess.output` rows (**kind 21**, tokenless,
  `runtime.rs:607`) are neither in `NON_CALL_KINDS` (`crates/maos-audit/src/fr4_classifier.rs:51-61`) nor kind 7;
  `classify_fr4_row` (`:295`) checks `NON_CALL_KINDS` first (`:297`), then returns `Call` for every non-kind-7 row
  (`:301-303`) ⇒ `maos audit query --spirit worker` exits 1 `Fr4SchemaViolation`.
- `task.orphaned` is **kind 1 carrying the record's 16-byte capability token zero-padded** into the 32-byte token
  column (`crash_detector.rs:161-170`) — FR4 feeds a crash report as a mediated call. `WriterShapeEntry`
  (`fr4_classifier.rs:105-114`) has **no kind field and no token dimension**; `PayloadType` has no `StrOrNull`
  and no array-of-arrays (`TokenId` serializes as a 16-number array, `crates/maos-domain/src/invariants/i1.rs:45-46`;
  `NumArrayOrNull` `:354-359` rejects nested arrays). `kind_to_string(1)` is `"task.complete"`; the NDJSON stderr
  count is `{kind}×{n}` over all omitted kinds (`crates/maos-audit/src/lib.rs:655-664`).
- **The 16-2 tests assume kind 7:** `crates/maos-audit/tests/fr4_classifier_16_2.rs` — count equality `:226-232`,
  rows built as tokenless `capability.invocation`, a scanner that drops non-7 kinds and token-bearing rows
  (`:545-556`) and a reverse check `:673-683`. Any kind-1 entry reds them unless they are extended.
- **Seven literal kind-1 `TaskComplete` production writer sites**, read: `crash_detector.rs:164` `task.orphaned`
  (Worker pid, token, JSON); `crates/maos-iac/src/adapter.rs:246` `task.nacked`, `:268` `task.escalated`, `:292`
  `task.reassigned` (pid 0, tokenless, sender = originator, JSON `{task_id, originator_spirit_id, capability_token[, replica_spirit_id]}`);
  `crates/maos-iac/src/adapter/transparency_log.rs:962` `distillate.redacted` (pid 0, tokenless, JSON
  `{principal_id, redacted_distillate_frame_id}`); `crates/maos-kernel-core/src/halt/resolver.rs:223` a SECOND
  `task.orphaned` (pid 0, tokenless, **non-JSON** `orphaned: accepted_halt halt_id=…`); `crates/maos-bin/src/main.rs:9022`
  `smoke-distillate-source` (smoke seed, pid 0, tokenless, raw bytes). Separately, `insert_kernel_event_returning_id`
  (`transparency_log.rs:1364-1405`) writes raw kind-1 rows the scanner cannot see — three callers at a real pid
  (`main.rs:5796`, `:10411`, `:10513`) already red FR4 at HEAD (§11 row 12).
- **The default output format is `plain`**, which renders call and non-call rows with identical columns and no
  stderr count (`lib.rs:878-913`) — only `--format ndjson` shows the omission.
- **Redaction eats identifiers:** the TL `sk-` substring rule (`crates/maos-iac/src/adapter/redaction.rs:81`)
  matched `task-worker-1` (probe: `ta<REDACTED:type=api_key_generic…>`), and the hex-run rule redacts ≥32
  contiguous hex chars (`:169-181`). Task ids must avoid both; the `sk-` defect is routed (§11 row 7).

## 6. 🔴 AC4 — STALE CITES, A RACING ORDER, A VACUOUS PROOF, THREE MISSING ROOTS

**Cites at `af96c907`** (all AC4's moved; validator-confirmed): smoke `.unload(` `main.rs:6256` · shell teardown
`server.shutdown` `:3833` → `drain_started_tasks` `:3836` → `finish_shell_session` `:3838` · topology `--once`
`:4633`/`:4636`, 14-owner drop block `:4638-4659`, `return Ok` `:4671` · single-class `--once` (butler, researcher,
digest, Mira, Nash…) `:5596`/`:5599`, **7-owner** drop block `:5602-5608` (~20 owners short of the serving root's),
run block ends `:5626` · serving loop `tokio::select!` `:8008` → `cancel.cancel()` `:8016` → door `:8032`/`:8035` →
watchdog + PDP joins `:8047-8066` → 28 drops `:8095-8124` → writer await (10 s) `:8126-8128` → `drop(store_locks)`
`:8131`. `OperatorHttpServer::shutdown(&mut self)` (`crates/maos-control/src/lib.rs:636`).

- **Runtime (probe A):** butler `--once` ⇒ exit 0 after 5.1 s, `maos run: audit writer drain timed out after
  5s` — **still present**. TL: `lifecycle.admit`/`load`/`start` at pid 1, `governance` pid 0, **`belief_variance`
  halt at pid 0** — no `lifecycle.unload`, no receipt; the halt is pending at exit. The halt is deterministic
  without `--live` (`main.rs:662-682`); `:5581` prints the id in Debug form `HaltId("…")`.
- **Runtime (probe B):** idle serving root SIGTERM ⇒ exit 0 in 8 ms; with the halt raised (`MAOS_IDLE_FAST=1`,
  a kernel read `idle_watchdog.rs:48`) ⇒ 22 ms, no receipt. `maosctl halt list --spirit butler` ⇒ **`0 halts shown`**
  while the halt row exists (its `--spirit` filter is by pid; `halt list` prints no pid). 16-1's budget:
  `ac1_sigterm_exits_promptly_and_drains_the_audit_channel` `< from_secs(3)`, no `drain timed out`
  (`one_daemon_one_door_16_1.rs:472-489`).
- **Butler's proof is vacuous as written.** `BUTLER_SPIRIT_PID = 0` (`spirits/butler/src/lib.rs:122`) flows through
  `port.write_scalar` (`:466-472`) → `ButlerOrchestratorAdapter::write_scalar` (`main.rs:588`, `:602-630`), which
  forwards it verbatim → `invoke_halt` at pid 0 → `drain_for_spirit(1)` misses it and `terminate_spirit` writes a
  synthetic `term-butler-1-<ns>` receipt. **Mira uses the same adapter** and passes 0 too
  (`spirits/mira/src/lib.rs:204-205`). `ButlerOrchestratorAdapter` is constructed at `:690`, `:4456`, `:5374`;
  `construct_butler_core` (`:649`, returns `(Butler, Option<receipt>)`) has two callers (`:3235` the upgrade-successor
  factory, `:4859`). The factory's signature is fixed: `SuccessorSpiritFactory::create(&SpiritManifestBundle) ->
  Result<Arc<dyn AnySpiritObj>, UpgradeError>` (`crates/maos-kernel-core/src/lifecycle/upgrade.rs:28-33`); a hot swap
  resolves the pid before `create` (`:103` vs `:130`) and keeps it; the door enforces class == id
  (`operator_door.rs:674`). `ScalarPortError` has only `Backend(String)`. `Ctx` carries no pid.
- **"Unload BEFORE the watchdog join" races.** `IdleWatchdog`'s `select!` (`idle_watchdog.rs:53-60`) sees `cancel`
  only between ticks and awaits `fire_on_idle`; a dropped `fire_on_idle` future leaves its `spawn_blocking` hook
  running (`hook_dispatch.rs:574-588`). Either way an in-flight `on_idle` can raise a halt after `terminate_spirit`
  drained.
- **Three roots AC4 never listed:** (i) **standalone `[cli_wrapper]`** binds the door and leaves through
  `run_cli_wrapper_manifest(…)?` (`:4709-4719`), the non-completion `return Err` (`:4731-4736`), the flush `?`
  (`:4740-4746`) or `return Ok` (`:4749`) — none drains the door or the writer; (ii) **`cohort-a2a-daemon`**
  (`return run_cohort_a2a_daemon(…).await` `:7633`; tail `:9945-9962`, `host_b_drain.await` ~`:9953-9958`) never
  awaits its door commands or its writer, and without that await channel rows are intermittently lost
  (`:7916-7923`); (iii) **SIGTERM during a Worker** (§3). `cohort-a2a-daemon` loads no class Spirits.
- **`--once` early returns after `start` skip the teardown:** `:4608-4611`, `:5503`, `:5511`, `:5519`, `:5525-5541`.
  83 `?`/`return` sites in the run block overall.
- **16-2's `finish_shell_session`** (`shell_host.rs:222`) is hello-spirit-only and **reports an unload failure
  without returning it** (`:233-243`) — its exit contract must survive generalisation.
- **Kernel order inside `unload`:** `transition(Unloaded)?` `:483` → `lifecycle.unload` row `:492-499` →
  `fire_on_unload` `:501` → `check_hook_outcome(…)?` `:502` → `terminate_spirit(PlannedUnload)` `:505` (receipts go
  straight to the TL) → `revoke_all_for_pid` `:514` (ALWAYS `try_send`s a `SpiritUnload{pid,count}` revoke with
  `TokenId::ZERO` through the audit channel, `cap_tokens/mod.rs:301-313`) → `drain_for_spirit` `:516` → map remove
  `:519`. `scbs()` is a std `RwLock`; iterating under its read guard while calling `unload` deadlocks.
  `Spirit::on_unload(&self, &mut Ctx)` is sync, in `spawn_blocking`, with a budget.
- **Pinned prints:** `xtask/src/demo_j1.rs:573-587`, `EXPECTED_IDLE_FIRES` `:381`/`:701`, once-only stages `:715-735`,
  and `crates/maos-journey-test/tests/journey_j1.rs:66-79` pin exact `spirit_loaded` sets.
- NFR-Rel-11's job (`discipline.yml:1252`) runs `halt_receipt_production_rate.rs` over `terminate_spirit` directly.

---

## 10. SIZING AND BUDGET (rule 6, ×1.3; interval arithmetic)

| crate | headroom | change | raw | ×1.3 upper |
|---|---|---|---|---|
| `maos-bin` | **+983 (expect to cross)** | NEW lib `src/supervision.rs`: `RootSupervision`, `WorkerSupervision` port + `WorkerSupervisor` (ids, bind/abandon/finish, mutex binding state, pidfd observer + handler, `stop_workers`, `bindings()`, active-path tracking, stop-disposition, `join_outstanding`), progress stamper, signal listener, `WorkerSpirit`, and D-16-3-Q's pattern seams (`ExitObservation` strategy pair, `BindingPhase` transition/`BindingAction` types, the `WorkerBinding` guard) +270…+440 · `unload_all_loaded` + report, `finish_shell_session` delegates +25…+65 · `worker_spawn.rs` params, bind/abandon/finish, real pid, record, read-back by pid +55…+100 · `main.rs` root-supervision declaration + two sites, conditional serving watchdog, select arm, `block_in_place`, topology post-Worker check +60…+120 · teardowns at 5 roots incl. the cohort writer await and restructured `--once` tails/standalone arm +70…+150 · butler binding + successor factory +30…+60 · `delegation.rs` + cohort params +20…+45 | +530…+980 | **+1274** |
| `maos-audit` | **ZERO (will cross)** | `kind` + `token` on `WriterShapeEntry`; non-7 entries before the kind-7 gate; `PayloadType::{StrOrNull, NumArrayArray}`; kind 21 in `NON_CALL_KINDS`; kind-1 entries; scanner covers `FrameKind::TaskComplete` incl. token-bearing sites | +58…+100 | **+130** |
| `maos-journey-test` | +499 | `run_bounded` concurrent drain (`deferred-work.md:938`) | +12…+30 | **+39** |
| `maos-manifest` | +260 | none expected (T0: `OnCrashSection`/`SupervisionSection` parse reachable from maos-bin) | 0…+15 | +20 |
| `spirits/worker` | ungoverned | fixture modes | — | — |
| `maos-kernel-core` · `xtask` · `maos-domain` · `maos-shell` · `maos-control` | — | untouched | 0 | 0 |

If a ceiling is crossed: code first, `cargo fmt --all`, `kloc-check --json`, the `kloc.toml` row carries the bare
token **`16-3`** followed by a non-digit non-dash character, the measured figure and the driver, same commit.
**Do not compress the supervisor or the teardowns to fit** — they are the capability.

Dependencies: **no new crate, no new lockfile entry.** `crates/maos-bin/Cargo.toml`'s existing
`[target.'cfg(unix)'.dependencies] rustix = { version = "1.1", features = ["fs"] }` (`:115-116`) gains `"process"`
(`pidfd_open`, `pidfd_send_signal` `rustix-1.1.4/src/process/pidfd.rs:41`, `waitid`, `WaitId::PidFd`,
`WaitIdOptions::{EXITED, NOWAIT}` `src/process/wait.rs:54-67,383-397,488`). Validator: `git diff Cargo.lock` empty
after the edit; T0 repeats with a full `cargo build -p maos-bin` and `cargo deny check bans`. **`supervision.rs` is
an ungated module** (16-2's ungated `shell_host` delegates to `unload_all_loaded`); its Worker items are
`#[cfg(feature = "network")]` like `worker_spawn` (`lib.rs:41-64`), so the no-default-features build
(`discipline.yml:449-451`) stays green. New file ⇒ `SCANNED_SOURCE_FILES`
(`crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:880`) **21 → 22**.

---

## Decisions ruled in this story (rule 7 — one decision per fork)

All rulings are **operator-ratified 2026-09-15** (§15: *"1. one story. 2. observer. use design patterns. 3. yes. 4. yes both. 5. yes. 6. do as suggested"*), as corrected by four validation rounds (Change Log). **D-16-3-Q** records the design-pattern directive attached to Q2. Where a ruling depends on a measured fact, the fact is in §1–§6.

| # | Fork | Ruling | Rejected, and why |
|---|---|---|---|
| **D-16-3-A** | Sizing | **WHOLE** (frontmatter `split_from`; §14 Q1). | *Split `16-3b` = AC4* — both halves rewrite the same teardown blocks and meet at SIGTERM-during-a-Worker. |
| **D-16-3-B** | Where a Worker's death is observed | **An exit observer independent of pipe EOF, zero kernel-Δ.** Immediately after `spawn_and_bridge` returns — on the same thread, before `pump_to_journal` starts — the supervisor calls `rustix::process::pidfd_open(Pid::from_raw(child_pid as i32).ok_or(…)?, PidfdFlags::empty())` (`Pid::from_raw` returns `Option`, `rustix-1.1.4/src/pid.rs:43`) and stores the fd, wrapped in the strategy module's `ObservedChild`, in the binding state (D-16-3-D, D-16-3-Q (3)). A dedicated `std::thread` blocks in `waitid(WaitId::PidFd(fd.as_fd()), WaitIdOptions::EXITED \| WaitIdOptions::NOWAIT)`; `NOWAIT` leaves the zombie for the bridge to reap, so `wait_and_finalize` still classifies and journals `cli.subprocess.exit`. On a status carrying a crash cause (D-16-3-F), the observer takes the binding lock and, **only if the phase is `Running`**, sets `CrashHandled` and — inside the same lock — `Handle::spawn`s `crash_detector.handle_crash(pid, cause)`, storing its `JoinHandle` (so no reader ever sees `CrashHandled` without a handle). An exit-0 status does nothing (`finish` owns clean exits). **`ECHILD`** (the bridge reaped first — 106/200 on fast exits) ⇒ the observer publishes nothing and **`finish` handles the exit from the bridge's `ExitCause`** (D-16-3-D) — never an "unavailable" row, never an `unwrap`. On Linux this is not a corner case: it is how every fast-exiting crash is handled. **Platform:** `#[cfg(target_os = "linux")]`; elsewhere, or when `pidfd_open` fails (`ENOSYS`/`EPERM`), there is no observer and `finish` maps the bridge's `ExitCause` after the pump — the ROOT's one-time probe failure is journaled once per root as a `telemetry.event` `worker.exit-observer-unavailable:<errno>` (a per-Worker failure after a good probe is journaled per binding, D-16-3-Q (3)); stated in the rustdoc and §11 row 6. | *Wait after the pump (the epic)* — bounded by the last pipe holder (§2). *Split the kernel bridge* — a kernel byte under the 16-0 pin with no grant. *`waitid(WaitId::Pid)`* — after the reap a reused pid misattributes a death. *Poll `/proc/<pid>/stat`* — latency = cadence. *Process-group kill* — a kernel byte; 17-1's `spawn_t3` (§11 row 2). *CAS then store the handle separately (round 1)* — a window where `finish` sees `CrashHandled` with no handle. |
| **D-16-3-C** | How the async kernel is driven from the sync Worker path | **A sync port, the 16-1 `OperatorDoor` pattern.** `pub trait WorkerSupervision: Send + Sync` and `pub struct WorkerSupervisor` (NEW `src/supervision.rs`) holding `Arc<SpiritSchedulerAdapter>`, `Arc<CrashDetector>`, `Arc<TransparencyLogAdapter>`, `Arc<HaltRegistry>`, `Arc<IacBusAdapter>` and a `tokio::runtime::Handle` captured in async `main`. `run_cli_wrapper_manifest` gains `supervision: &dyn WorkerSupervision` and `task: WorkerTask` (D-16-3-E) and **stays a sync fn** (19-3 AC3 relies on it). Its blocking steps use `handle.block_on`, legal only off the async context: **paths (a) and (b) call `run_cli_wrapper_manifest` inside `tokio::task::block_in_place(|| …)`** (tokio: *"Code running inside `block_in_place` may use `block_on` to reenter the async context"*, `tokio-1.52.3/src/task/blocking.rs:52`; `main` is `flavor = "multi_thread"`, `main.rs:1772`); **path (c)** already runs inside `spawn_blocking` (`delegation.rs:682`; its comment `:672-673` "nothing inside is async" is corrected). **Validator ran it:** on a `worker_threads = 1` runtime, `block_in_place(|| Handle::current().block_on(…))` awaiting a task spawned on the same runtime completed in all four shapes; direct `Handle::block_on` in async `main` panics. | *std `mpsc` handoff from the runtime worker* — deadlocks with one worker. *`spawn_blocking` with owned args for (a)/(b)* — borrowed arguments. *Make it async* — 19-3 keeps it sync; 17-1 re-routes the spawn beneath it. *A `SubprocessSupervisor` impl (arch `4-kernel-design.md:605`)* — its `spawn_child` owns the spawn `spawn_and_bridge` owns. |
| **D-16-3-D** | The Worker's SCB and its state machine | **One SCB per Worker, bound between the gates and the mint, released on every exit.** Order inside `run_cli_wrapper_manifest`: every existing gate unchanged and at pid 0 (parse, **`[on_crash]`/`[supervision]` parse — a malformed or out-of-bounds section refuses the run before load; the bounds are a validation refusal, `manifest.rs:1748-1756`, not a clamp**, tier, respawn refusal, host grant, `select_worker_cli`, live gate, ambient-auth, admission probe `:575`, argv refusals) → **`bind(task, bundle)`** = `scheduler.load(id, bundle, WorkerSpirit::new(state), boot_nonce)` then `scheduler.start(pid)` and the record push → mint at the **real pid** → **lock, read the phase and the latch, RELEASE; if `PlannedStop` or latched ⇒ `abandon` and return a typed "stopped before spawn" error** → `spawn_and_bridge` → **lock: store the pidfd; if phase is `PlannedStop` ⇒ `pidfd_send_signal` now** (a stop that landed during the mint) → observer → `pump_to_journal(&tl, pid, …)` → `wait_and_finalize(&tl, pid, revoke)` → `finish(binding, bridge_exit)`. **Every step is lock → decide → release → act: never `abandon`, `unload`, `enforce_disposition`, `block_on` or `handle_crash` while holding the binding lock** — `unload` fires `WorkerSpirit::on_unload`, which takes the same non-reentrant lock, and would block to the 30 s hook cap (`hook_dispatch.rs:87`), after which `check_hook_outcome?` (`scheduler_loop.rs:502`) returns before the receipt, the revoke and the map removal, leaving an `Unloaded` SCB that every later `unload` skips (`:478-480`). (`Handle::spawn` inside the observer's lock is the one exception: it does not block.) **Binding state** is one `std::sync::Mutex<BindingState { phase: Running \| CrashHandled \| PlannedStop \| ExitedClean, child: Option<ObservedChild> /* a `Clone` type DEFINED IN THE STRATEGY MODULE: on Linux `#[derive(Clone)]` over the pidfd `Arc<OwnedFd>`; elsewhere an empty `Clone` struct — so no `OwnedFd` or cfg appears outside that module, and `bindings()` can clone it into each view */, handler: Option<JoinHandle<…>>, record: TaskAssignmentRecord /* a clone kept for disposition */, on_crash_action: OnCrashAction /* captured at bind — the SCB may be gone when a stop is dispositioned */, dispositioned: bool }>` (the I9 lint scans `crates/maos-kernel-core` only). **The record is pushed at bind with `TokenId([0; 16])` and `ttl_deadline_ns = u64::MAX`, and patched after the mint** — under the binding lock, both the SCB ledger entry (found by `task_id`) and the kept clone get the minted token id and its expiry (D-16-3-E). **A supervisor-wide `stopping` latch** is set by `stop_workers()`: `bind` refuses with a typed `WorkerStopping` error once it is set, and the pre-spawn phase check also reads it — so a Worker that would bind after the signal (a standalone run still in its admission probe, host B's next queued frame, a later topology entry) never runs. **Every return between bind and finish goes through `abandon(binding)`** = (from `Running`: signal through the strategy first, D-16-3-Q (5) `SignalThenDispositionAndUnload`) disposition from the kept record (below) + `unload(pid)` — the governance refusal `worker_spawn.rs:663-675` (now after bind), the spawn `?` `:706`, and any other `?` T0 finds; host-B's refusal gets a vector. **On path (b) the caller matches the result of `run_cli_wrapper_manifest` (`main.rs:4279-4290`) and checks `root_shutdown` on BOTH arms before any `?` or the not-completed `return Err` (`:4300-4305`)** — a stop during the mint returns `Err` from `abandon`, and must still reach the topology's teardown. **Ids:** a per-root counter; the **first Worker of a root is `worker`**, later ones `worker-2`, `worker-3`, …, never reused within a root. **Bundle** = `SpiritManifestBundle::default()` (empty `enabled_hooks` ⇒ every hook fires) with `on_crash` from `[on_crash]` (FR50; `OnCrashSection` `manifest.rs:1386-1410`; default `Nack`) and `supervision` from `[supervision]` (default threshold 30000 ms = the NFR's ">30s"). **Stop** (`WorkerSpirit::on_unload` and `stop_workers()`): lock; if phase is `Running` ⇒ set `PlannedStop` and, if a child handle is stored, the strategy's `signal_kill` — on Linux `rustix::process::pidfd_send_signal(&pidfd, Signal::KILL)` (Ok on a zombie, `ESRCH` after the reap — never a raw-pid kill). **The revoke closure stays unconditional at the real pid** (D-16-1-V: the Worker's `CliSubprocessExit` row is kept even when an unload already revoked; the closure is `FnOnce`, called once). **`finish`** (lock, decide, release, then act): `CrashHandled` ⇒ `block_on` the handler bounded by 5 s; if it returned `NotLoaded` (a concurrent unload removed the SCB first) and the record is not yet dispositioned ⇒ disposition from the kept record; `Running` + crash cause ⇒ set `CrashHandled`, `block_on(handle_crash)` — **on every platform and strategy**; on Linux this is the path whenever the observer lost the reap race; `Running` + exit 0 ⇒ `ExitedClean`, take the record out of the SCB ledger without disposition (the verdict row is the record), `unload(pid)`; **`PlannedStop` ⇒ disposition from the kept record** (`enforce_disposition(state.on_crash_action, &[record], &iac, None)` — a stopped Worker's task is as dead to its originator as a crashed one's; no `task.orphaned`, which only `handle_crash` writes) **then `unload(pid)` itself** (idempotent: `Ok` for a missing or `Unloaded` pid) — so a topology Worker stopped by SIGTERM, whose not-completed return exits through `:4300-4305` before any teardown, is still unloaded. "Disposition" always takes the record out of the SCB ledger first if it is still there, and sets `dispositioned`. **`abandon` from `CrashHandled`** (e.g. the guard's `Drop` during a panic after the observer fired) ⇒ join the handler (bounded 5 s), disposition from the kept record ONLY if it returned `NotLoaded`, then `unload(pid)` — never a plain disposition-and-unload, because `handle_crash` has already drained the ledger and dispositioned (`crash_detector.rs:142-180`) and a second `task.nacked`/`task.escalated` would violate (t). **A handler join that times out** (5 s, in `finish` or `abandon`) ⇒ leave the `JoinHandle` to `join_outstanding`, no disposition, no unload by this path (the teardown's `unload_all_loaded` owns it). **No `spirit_loaded` line is printed for a Worker** and `cli_wrapper_loaded` stays single. The bridge's `from_spirit_id` stays `"worker"`, but **the completion read-back filters on the Worker's pid** (`query_frames` `worker_spawn.rs:775-807` today filters `from_spirit_id == "worker"` + `since_ns`, mixing concurrent Workers). | *Load before the gates* — refused manifests would leave lifecycle rows. *`worker-<n>` from 1* — epic 17's `--spirit worker` could never resolve. *A no-op `on_unload`* — "unloaded" while running. *`kill_process` on the raw pid* — reused-pid window. *Gate the revoke closure (round 1)* — reverses ratified D-16-1-V. *`finish` on `PlannedStop` does nothing (round 1)* — a stopped topology Worker's SCB was never unloaded, and its record never dispositioned. *Atomic CAS without a lock (round 1)* — stop-before-pidfd and handle-storage windows. *Read the action from `scb.runtime_snapshot()` at stop time (round 2)* — a door `unload` removes the SCB first; a dev falls back to `Nack` and an `escalate-to-operator` Worker writes `task.nacked`. *No stop latch (round 2)* — a Worker binding after the signal runs unstopped and trips the grace exit with a false message. |
| **D-16-3-E** | The in-flight task record | **Pushed at bind for every Worker; the id is passed in, never invented inside.** NEW `pub enum WorkerTask { Standalone, TopologyEntry { index: usize }, Delegation { frame_id: [u8; 16] } }` passed by each caller (the caller holds no SCB id — `TopologyEntry` is `{manifest, host}` only, `topology.rs:21-24`, and ids are assigned inside `bind`); `bind` derives the record: **(c) host B and (b) topology members with a `host`** ⇒ `task_id = delegation:<frame_id as four 8-hex groups joined by ':'>` (below the hex-run rule's 32 contiguous chars), originator `delegation::FROM_SPIRIT`; **(b) topology members without a `host`** ⇒ `topology:<entry index>`, originator `topology`; **(a) standalone** ⇒ `run:<assigned spirit id>`, originator `operator` (the standalone run IS the operator's task; without a record the watchdog cannot see it, `progress_watchdog.rs:78-80`). Where the frame id comes from: path (c) has it at `delegation.rs:661`; for path (b) the only arm that spawns a local Worker is `DelegationOutcome::RehearsedLocally { goal }` (`delegation.rs:134`), which carries no frame id — the caller reads `frame.frame_id` (`crates/maos-domain/src/frame.rs:27`) **before** `delegate(&iac, frame)` moves the frame (`main.rs:4197`) and carries it out of the inner block (`SentCrossHost { frame_id }` spawns nothing, `main.rs:4254-4265`). `capability_token`: pushed as `TokenId([0; 16])` at bind and patched to the minted `CliSubprocessSpawn` token id at the real pid (D-16-3-D), or left **`TokenId([0; 16])`** when mediation is refused (`worker_spawn.rs:681`; the smoke arm's sentinel `main.rs:6376`), stated as "no kernel token". `ttl_deadline_ns`: `u64::MAX` at bind, patched to the token's expiry after a successful mint (T0 greps its readers and records them). `intent_class: Standard`. No id contains `sk-`. | *Record only when a token exists* — hermetic Workers get no `task.orphaned` and no stall detection. *Raw 32-hex frame ids* — redacted. *A new capability scope for tokenless Workers* — invents authority. |
| **D-16-3-F** | Exit → `CrashCause` | Signal ⇒ `Fault(SignaledByKernel { signal, stderr_tail })`; code ≠ 0 ⇒ `Fault(NonZeroExit { code, stderr_tail })` (`supervision.rs:99-106`; ADR-022's `is_crash`, `runtime.rs:126-132`); **code 0 ⇒ not a crash**; both sources lost (`ExitCause::Unknown`, no observer status) ⇒ `Fault(Truncated { reason: "exit status unrecoverable" })`, the only free-text variant, stated as reused. A signal the supervisor itself sent (phase `PlannedStop`) is never a crash. `stderr_tail`: the last ≤ 20 lines / 4 KiB of the Worker's kind-21 `stderr` rows at its pid journaled at detection time, else `None` — stated that the observer path (early) and the fallback path (after EOF) can differ. A clean exit whose oracle says `not_completed` is **not** a crash (ADR-022). | *Add a `FaultCause` variant* — the epic ratified none. *Exit 0 + oracle failure = crash* — ADR-022's false-positive rule. *`NonZeroExit { code: -1 }`* — a code never returned. |
| **D-16-3-G** | What counts as Worker progress | **A journaled output row at the Worker's pid, read with an EXCLUSIVE cursor.** One per-root `WorkerProgressStamper` task ticks every 1 s: one kind-21 `query_frames` **after its last-seen row by the keyset cursor** (`transparency_log.rs:1488-1493`; `since_ns` is inclusive, `:1480-1482`, and would re-return the last row every tick — a Worker that printed once then hung would never stall), grouped by `spirit_pid`; for each live binding with ≥1 new row it stores `monotonic_now_ns()` into `scb.last_progress_iac_ns`. Both streams count. A silent Worker keeps its bind-time stamp; a Worker printing at least once per (threshold − 2 s) never stalls. T0 records whether the TL adapter exposes a change-notification seam; if so the stamper uses it and says so. | *No progress source (the epic)* — every honest long-running agent CLI false-stalls. *`since_ns` high-water (round 1)* — inclusive; the Worker that printed LAST re-stamps itself forever and never stalls (other printing Workers mask it — so the falsifier runs where nothing else prints, AC2). *A maos-bin pump over `recv_line`* — duplicates the kernel's journaling payload. *A pump callback* — kernel byte. |
| **D-16-3-H** | Where the root's supervision lives | **One `Option<RootSupervision>` declared in `main`'s scope immediately before `if let Some(run)` (`:3984`) — nothing spawned there — and filled at two sites.** `pub struct RootSupervision { root_cancel: CancellationToken, root_shutdown: CancellationToken, supervisor: Arc<WorkerSupervisor>, watchdog: JoinHandle<()>, stamper: JoinHandle<()>, listener: JoinHandle<()> }` with `async fn stop_and_join(self)`. **Site 1:** the first statement inside `if let Some(run)` assigns `root_supervision = Some(…)` (every input is in scope: after `set_crash_detector` `:2911`, `boot_nonce` `:2077`, `notification_dispatcher` `:2054`, telemetry `:2042`, TL `:2214`, `halt_registry` `:2197`; before `:4279`/`:4709`). The value survives the run block's end at `:5626`, so the non-`--once` fall-throughs reach the serving tail WITH it. **Site 2:** inside the `cohort-a2a-daemon` arm before `:7633`, a local `RootSupervision` whose `Arc<WorkerSupervisor>` is passed to `run_cohort_a2a_daemon` (D-16-3-I). **Serving tail:** `if root_supervision.is_none()` spawns the `:7989` ProgressWatchdog as today (bare `maos` and any path that never entered the run block); its `select!` at `:8008` gains an arm that is `root_shutdown.cancelled()` when `Some`, `std::future::pending()` when `None`; its teardown `take()`s the value and `stop_and_join`s it with the other watchdogs. The `MAOS_ONE_SHOT` arms never see a `RootSupervision`. `IdleWatchdog`/`ScheduleWatchdog`/`SilentFailureDetector` stay where they are. | *Spawn before the run block (round 0)* — every one-shot arm would carry a SIGTERM listener that swallows its signal. *Locals inside the run block (round 1)* — detached at `:5626`: the serving root would run two ProgressWatchdogs, lose `root_shutdown`, and its writer drain would time out on the detached owners, redding 16-1's `< 3 s` test (validator). *A local watchdog inside `run_cli_wrapper_manifest`* — N watchdogs over one SCB map. |
| **D-16-3-I** | Which roots | **All three Worker paths** (§3): (a) and (b) use site 1's supervisor; (c) uses site 2's through a NEW `HostBWorkerContext.supervision: Arc<WorkerSupervisor>` field — **concrete, not `dyn`**, because the cohort teardown needs its scheduler and halt registry for `unload_all_loaded`. `host_b_worker_context` (`main.rs:9649`) and `run_cohort_a2a_daemon` (`:9710`) each gain one parameter **appended after the existing ones** (`enterprise_daemon_seam_13_5a.rs:84-97`'s signature scan stops at the first `)`). `tests/two_host_delegation_2b.rs:648-666` builds `HostBWorkerContext` as a struct literal and calls `handle_one_inbound` directly — it gets a real supervisor. `maos-bench` `j1.rs` is out (bench). | *(a)+(b) only* — host B is the only continuously-serving Worker spawner. *`Arc<dyn WorkerSupervision>` (round 1)* — cannot reach `unload_all_loaded`'s inputs. |
| **D-16-3-J** | FR4 at a Worker's pid | **`maos audit query --spirit worker --format ndjson` exits 0, feeds no crash report as a call, and names what it omitted.** In `crates/maos-audit/src/fr4_classifier.rs`: (1) `"cli.subprocess.output"` (kind 21, `crates/maos-audit/src/lib.rs:725,763`) joins `NON_CALL_KINDS` — never a call by its `FrameKind`; a kind-21 row is `NonCall` token or not (existing order `:297`); **`"identity.asserted"` (kind 30) joins too** — minting at the Worker's real pid moves `persist_identity_asserted(spirit_pid, …)`'s tokenless kind-30 row (`crates/maos-bin/src/enterprise_identity.rs:345-364`) onto that pid under an enterprise posture, and an identity assertion is not a call. (2) **`WriterShapeEntry` gains `kind: u8` and `token: TokenColumn { Absent, Present }`**; every existing entry is `kind: 7, token: Absent` (the `call()` constructor's default kind is 7); entries with `kind != 7` are evaluated **before** the kind-7 gate at `:301-303`; an entry matches only its own kind and token presence. (3) **Two NEW `PayloadType` variants, ratified here:** `StrOrNull`; `NumArrayArray` = a non-empty JSON array whose every element is an array of exactly 16 numbers. (4) **Kind-1 dispositions, ruled now** (§5): `crash_detector.rs:164` `task.orphaned` ⇒ `NonCall { kind 1, token: Present, intent "task.orphaned", payload task_id: Str, originator_spirit_id: Str, exit_signal: NumOrNull, exit_code: NumOrNull, stderr_tail: StrOrNull, cause: Str, in_flight_tokens: NumArrayArray }` (a tokenless forgery with that shape is `Call`); `adapter.rs:246` `task.nacked`, `:268` `task.escalated` ⇒ `NonCall { kind 1, token: Absent, payload task_id: Str, originator_spirit_id: Str, capability_token: NumArrayOrNull }`; `:292` `task.reassigned` ⇒ same + `replica_spirit_id: Str`; `transparency_log.rs:962` `distillate.redacted` ⇒ `NonCall { kind 1, token: Absent, payload principal_id: Str, redacted_distillate_frame_id: Str }`; `resolver.rs:223` (non-JSON `orphaned: accepted_halt …` at pid 0) ⇒ **`Call`**, reason recorded: a raw-bytes payload cannot be shape-matched, and a pid-0 row reaches a `--spirit` view only on a `hello-spirit` one-shot boot — if it ever does, it is a genuine FR4 finding (§11 row 12, 16-5); `main.rs:9022` `smoke-distillate-source` ⇒ **`Call`**, reason: a smoke seed, never a production row. (5) **The inventory doorbell** in `fr4_classifier_16_2.rs` is extended, not left unchanged: its measured tuple gains kind and token presence; its scanner treats `FrameKind::TaskComplete` sites (token-bearing included) as needing a disposition; its count equality counts the new entries. The doorbell covers the `insert_frame_event*` writer methods; `insert_kernel_event_returning_id` is out of its reach and routed (§11 row 12). No prefix match. `to_fr4_ndjson` stays call-only (six keys unchanged). | *Keep `task.orphaned` a `Call`* — it passes FR4 only by showing a task's capability token as mediation. *Stop padding the token* — kernel byte (§11 row 8). *Widen `NumArrayOrNull`* — every existing entry would inherit it. *"`fr4_classifier_16_2.rs` green unchanged" (round 1)* — impossible; the file is modified. *"T0 rules each site" (round 1)* — the payloads are readable now. *Assert the plain table (round 1)* — it renders both classes identically. |
| **D-16-3-K** | The corpora and their jobs | **Two maos-bin integration tests, in-process against the real supervisor, `#[ignore]`d, run by their NFR jobs with `-- --ignored`.** The harness reaches bindings through a public **`WorkerSupervisor::bindings() -> Vec<WorkerBindingView { spirit_id, spirit_pid, child_pid: Option<u32>, child: Option<ObservedChild>, phase }>` (the corpora reach the pidfd through `ObservedChild::pidfd()`, an accessor defined in the strategy module under `cfg(target_os = "linux")`)**. `crates/maos-bin/tests/worker_crash_corpus_16_3.rs`: exactly 100 fixture Workers — **50 `hang-with-grandchild`, 50 `hang`** — in concurrent batches of 20 on blocking threads; per Worker the harness stamps `Instant::now()` and `pidfd_send_signal(SIGKILL)` to the direct child, and **kills that Worker's grandchild at its own kill + 5 s** (after both floor windows; from the pid the fixture prints) so every blocked pump ends and the next batch can start. **One poller thread** runs ONE keyset-cursor `query_frames` per 10 ms tick over new rows (never per-pid queries — the TL has one mutex, `transparency_log.rs:1452`, shared with the `handle_crash` writes being measured) and matches each Worker pid's `lifecycle.crash` and `task.orphaned` rows and the disposition row **by payload `task_id`** (pid 0, sender = originator). **Asserts: exactly 100 kills sent (denominator first); ≥99/100 `lifecycle.crash` observed ≤2 s after the kill; ≥99/100 `task.orphaned` + disposition observed ≤5 s** (stated: `task.orphaned` precedes `lifecycle.crash`, so the 5 s floor is dominated by the 2 s one). With the observer removed, the grandchild half is detected only after its grandchild dies at +5 s ⇒ fails both floors ⇒ ≤50/100 ⇒ **the floor itself reds** (AC1 proven-red). `worker_hang_corpus_16_3.rs`: 50 `hang` + 5 `line-then-hang` + 5 `hang-chatty` concurrently at the default 30000 ms threshold; **asserts ≥48/50 `hang` Workers `task.stalled` ≤60 s from spawn, 5/5 `line-then-hang` stalled (the exclusive cursor), AND 0/5 chatty stalled over a window lasting until the last silent Worker stalls or 60 s, and at least 35 s**; counts distinct Workers, not rows (refire after 2× threshold, `progress_watchdog.rs:95`); then SIGKILLs all 60 and asserts each reaches the crash path. **Jobs:** `nfr-rel-1-crash-detection-2s` and `nfr-rel-2-hang-detection-60s` each gain `cargo build --release -p worker --bin worker-cli-fixture` and run `cargo test -p maos-bin --release --test worker_{crash,hang}_corpus_16_3 -- --ignored`, **failing unless libtest reports exactly `1 passed`** (a zero-test run is red), retried once with a `::warning::` annotation on first failure, then red (never `continue-on-error`); both stay in `aggregate.needs`. Every spawning test isolates `HOME` per command (`root_spawn_home_isolation_16_1.rs:128-160`). The kernel smokes stay in the workspace suite. | *Kernel-only corpus* — proves nothing about wiring. *Non-forking kills only (round 0)* — passes with the observer removed. *Grandchildren killed "at the end" (round 1)* — the first batch with one never completes: deadlock. *"Counts, not wall-clock"* — cannot express ≤2 s. *Per-waiter polling* — 100 waiters on the TL's one mutex. *Fill `coverage-matrix.yaml` here* — adds three unknown-gate violations; §11 row 15. |
| **D-16-3-L** | The fixture | `worker-cli-fixture` gains **`--maos-fixture-mode=<hang\|line-then-hang\|hang-chatty\|hang-with-grandchild\|sigkill-self\|exit-nonzero>`**, carried in the manifest's `argv_prefix` (hashed into the cap token; `FixtureCli` required/forbidden argv flags are empty), **parsed AFTER the `--maos-bridge-probe` check** (`worker-cli-fixture.rs:25`) so admission's probe still answers and exits; unknown mode ⇒ exit 2. `hang` sleeps silently; `line-then-hang` prints one line then sleeps; `hang-chatty` prints one line per second; `hang-with-grandchild` **spawns a child copy of itself** (`Command::new(current_exe())` in `hang` mode, stdout inherited — not `exec`, which would leave no grandchild), prints `worker-fixture: grandchild pid <n>` as its first stdout line, then sleeps; `sigkill-self` prints one line, flushes stdout, then runs `std::process::Command::new("kill").args(["-KILL", &std::process::id().to_string()]).status()` and sleeps (if `kill` fails to run it exits with the distinct code 97, so AC1 reds loudly instead of passing on a non-signal exit) — **no new dependency on `spirits/worker`** (`unsafe_code = "forbid"`, deps `maos-domain`/`serde`/`serde_json`/`toml` only, `spirits/worker/Cargo.toml:9,15-19`; adding `rustix`/`libc`/`nix` would change its `Cargo.lock` entry, and `std::process::abort()` dies by signal 6, not 9); `exit-nonzero` exits 3. Default (no mode) output byte-unchanged, **proven by its own vector** (the SHA pin hashes `tests/fixtures/canned-cli-output.json`, `fixtures_pin.rs:11,31`). **No env var** selects a mode. | *An env var* — unbound by the argv hash. *A second fixture binary* — a second grant identity. *Parse the mode first* — hangs admission's probe. |
| **D-16-3-M** | Root shutdown and teardown (epic AC4) | **One unload function, one order, every root; the listener only stops Workers.** NEW `pub async fn unload_all_loaded(scheduler: &SpiritSchedulerAdapter, halt_registry: &HaltRegistry) -> UnloadAllReport` (ungated): collect pids from `scbs()` into a `Vec` and DROP the guard; per pid: `drain_for_spirit_dry_run(pid)`, `unload(pid).await` (an `Err` is recorded, the loop continues), each dry-run id whose `lookup_state` is now `None` recorded as closed. The report renders `halt <id> closed by planned unload (no resolution)` and one line per failed pid; the CALLER prints them. **Causal result wins:** an unload failure makes a root's exit non-zero only when the root's own result is `Ok`; `finish_shell_session` delegates its loop and **keeps its contract** (report, never return). **Listener** (a task in `RootSupervision`): its SIGTERM and SIGINT streams are created **synchronously at the site** (`tokio::signal::unix::signal(…)` called in the site's own statement, before the task is spawned), after which the site prints `maos: root supervision armed` to stderr — the handshake a test waits for; the task `select!`s over those streams and `root_cancel.cancelled()` (so it is joinable). On the first signal it cancels `root_shutdown` and calls `supervisor.stop_workers()` (which sets the `stopping` latch) — **on its normal path it never unloads**. It then waits, **check-then-wait** on a `tokio::sync::watch` of the supervisor's active-path count (incremented by `bind` after the `stopping` latch check, decremented when the `WorkerBinding` guard is consumed by `finish` or dropped — never on entry to `run_cli_wrapper_manifest`, where a 10 s real-CLI liveness probe (`worker_spawn.rs:589`) running during a SIGTERM would hold the count past the 5 s grace and trip a false descendant exit for a Worker with no binding; a count already at 0 returns at once — never a bare `Notify::notified()`, which would never fire for an idle butler root and send it down the grace path at 5 s), `select!`ed with `root_cancel.cancelled()` (if the teardown cancels during the wait, the listener returns and the teardown owns the exit), for **5 s**; only on timeout (a descendant still holds a stopped Worker's pipe) it calls `unload_all_loaded` itself (receipts are written straight to the TL), prints `maos: worker <id> stopped but its output is still held open by a descendant; exiting without the door shutdown and the audit-channel drain (process-tree teardown: 17-1)` — or, when that Worker's binding holds no pidfd, `maos: worker <id> could not be signalled (no pidfd); exiting without the door shutdown and the audit-channel drain` — and `std::process::exit(1)` — stated loss: `cap.revoke` channel rows and the completion rows of door commands still running (16-1's drain rule, `main.rs:8025-8030`) (§11 row 2). After the listener is joined, a later SIGTERM is swallowed (tokio never restores the default); the teardown is already running — stated. **`root_shutdown` is observed by:** the topology load loop **immediately after each Worker returns, on the `Ok` and the `Err` arm** (`:4279-4290`, before `:4300`) and between entries (break to the topology's teardown — no further Spirit or Worker is loaded); both `--once` passes **between passes only — never by cancelling an in-flight hook** (a dropped `fire_on_idle` leaves its `spawn_blocking` hook running, `hook_dispatch.rs:574-588`); the serving `select!` (D-16-3-H); **the cohort daemon's own `select!` at `:9945-9950`**, which gains a `root_shutdown.cancelled()` arm — `WorkerSupervisor::shutdown_token()` exposes it to `run_cohort_a2a_daemon` through the appended parameter, because a SIGTERM during `build_cohort_a2a_daemon_runtime`/`install_cert_rotation`/`emit_cross_team_share` (`:9849-9940`) is consumed by the site-2 listener before that `select!`'s own streams exist, and tokio never restores the default disposition (without the arm the daemon would serve forever). **An interrupted `--once` root exits non-zero:** a topology broken off mid-load, or a single-class `--once` whose pass was skipped because `root_shutdown` was already cancelled, prints `maos run: interrupted by signal before the --once pass completed` (never `complete — exiting cleanly`) and exits 1 after its teardown — an exit 0 there is the 2a AC1.5 false-success shape. A serving root's SIGTERM keeps exit 0. **A single-class `--once` signalled DURING its one `on_idle` pass** (`main.rs:5505`): the pass runs to completion (never cancelled), the root exits 0 with its normal completion line — the pass did complete, so exit 0 is true; stated. **`maos run` refuses `MAOS_ONE_SHOT`:** the one-shot dispatch (`:5639`) is gated by the env var only, so a non-`--once` `maos run` with it set would enter a one-shot arm holding site 1's `RootSupervision`; the run block's first statement refuses the combination with a typed error (exit 2) — T0 greps tests and scripts that set both and records any, which then move under this ruling. **Order at every `maos run` root on its normal path:** door `server.shutdown()` → `drain_started_tasks().await` → `supervisor.stop_workers()` and await every Worker path (bounded 5 s) → `root_cancel.cancel()` and **join** the ProgressWatchdog, stamper, listener and every other watchdog/PDP task on that root → `supervisor.join_outstanding(5 s)` (stored crash handlers) → **`unload_all_loaded`** → drop **every** owner of `audit_tx` → await the writer → `drop(store_locks)`. **Roots:** serving loop (`:8008-8131`); topology `--once` (`:4575-4671`); single-class `--once` (`:5496-5621`); standalone `[cli_wrapper]` (`:4700-4749`); `cohort-a2a-daemon` — **ruled order, because its Worker drain lives inside the daemon fn while its door lives in `main`:** inside `run_cohort_a2a_daemon`, when its `select!` fires (signal or `root_shutdown`): stop intake — the existing `siem_cancel.cancel(); let shutdown = runtime.shutdown().await;` (`main.rs:9951-9952`, which cancels the intake loop's parent token, checked first in its `biased` select, `delegation.rs:715-722`) → `supervisor.stop_workers()` → `host_b_drain.await` **UNBOUNDED** → return `shutdown` (its existing result, `:9960`). The drain is not bounded here on purpose: a bound that "proceeds" would leave an uncancellable `spawn_blocking` Worker thread holding `Arc` clones of `capability` and the supervisor (`delegation.rs:674-683`), the new writer await could never finish, and tokio's blocking-pool drop joins every blocking thread with no timeout (`tokio-1.52.3/src/runtime/blocking/pool.rs:243-285`) — a root that never exits. **The site-2 listener's 5 s grace exit is the sole arbiter** (main does not cancel `root_cancel` until the daemon returns); then in `main`, `return run_cohort_a2a_daemon(…).await` becomes `let result = …;` followed by door `server.shutdown()` → `drain_started_tasks()` → join the RootSupervision tasks → `join_outstanding` → `unload_all_loaded` → drops → **a NEW writer await with its drop set** → locks → `return result` (a door command arriving during the ≤ 5 s Worker drain is served normally — no Spirit is unloaded yet); the shell (D-16-2-C; its order already holds, it has no Workers). **The `--once` tails and the standalone arm** each move into one `async` block whose result the caller takes before the teardown, so no post-`start` `?` and none of the standalone arm's `Err` exits bypass it. **Drain invariant, not a list:** *every owner of `audit_tx` is dropped before the writer is awaited, proven by the writer completing — no `drain timed out` — and by the unload's `SpiritUnload` revoke row being present in the TL.* Named new owners: the `WorkerSupervisor` and every clone (`RootSupervision`, `HostBWorkerContext.supervision`, the stamper and listener tasks), observer threads (end at child death) and stored handler tasks (joined above). The complete reference is the serving root's drop block `:8095-8124` (incl. the hidden owners `iac` and `delegation_leg`); the single-class `--once` block `:5602-5608` is ~20 owners short — that, not the missing unload, is why butler `--once` times out today. | *Unload before the watchdog join (the epic)* — races an in-flight `on_idle`. *The listener unloads on its normal path* — before the door shutdown and mid-topology (its grace-timeout path does unload, and then exits the process — the only case). *A bare `Notify` for the active-path wait (round 2)* — never fires at count 0. *No `root_shutdown` arm in the cohort `select!` (round 2)* — a SIGTERM during daemon startup is swallowed. *A 5 s bounded cohort Worker drain that proceeds (round 3)* — the root hangs on the blocking pool. *`select!` the `--once` pass on the token (round 1)* — drops an in-flight hook's future while its hook keeps running. *`out: &mut dyn Write` in an async fn* — not `Send`. *Wait forever for a descendant-held pipe* — a SIGTERM that never ends the root. *Walk `/proc/<pid>/task/*/children`* — racy, Linux-only, 17-1's tree (§14 Q6). *Restructure the whole run block* — 16-6 extracts it. |
| **D-16-3-N** | Butler's and Mira's halt pid | **Fixed at origin, host-side, in the one adapter both use.** `ButlerOrchestratorAdapter` (`main.rs:588`) gains an `Arc<AtomicU32>` pid binding; `write_scalar` **ignores the Spirit-supplied pid** and uses the binding; an unset binding (0) ⇒ `ScalarPortError::Backend("scalar port pid binding unset")` returned to the Spirit, never a pid-0 halt. `construct_butler_core` (`:649`) returns the binding alongside `(Butler, Option<receipt>)`. Its three construction sites (`:690`, `:4456`, `:5374`) and two callers are updated — **Mira** (`:4456` topology, `:5374` single-class): each site sets the binding to the pid `scheduler.load` returns, right after it returns (Mira writes scalars only from `on_idle`, `spirits/mira/src/lib.rs:183`; an unset binding would silently drop them — Mira discards the error at `:204` — latent today because production Mira has `pending_signals: None`, `:222-230`); **`:4859` (`maos run`)** sets the binding right after `scheduler.load` returns (the `:5034-5039` MCP pattern; safe — `write_scalar` is called only from `on_idle`, after `start`); **`:3235` (the upgrade-successor factory's butler arm, `:3228`)** captures a scheduler clone and sets the new binding to `scheduler.resolve_pid(&class.name)` **before returning** from `create` — a hot swap resolves the pid before `create` (`upgrade.rs:103` vs `:130`) and keeps it, and the door enforces class == id (`operator_door.rs:674`); T0 confirms no scheduler lock is held across `create`, and `ac4_root_e_a_faithful_successor_survives_the_hot_swap` (`one_daemon_one_door_16_1.rs:891-962`) stays green. `spirits/butler` and `spirits/mira` are not edited. `one_daemon_one_door_16_1.rs:262-271,520-522` comments are corrected. | *Thread a pid into the Spirit crates* — `Ctx` has no carrier; a setter lets a Spirit claim any pid. *Set the successor binding "after the swap" (round 1)* — races `on_idle` (a refused halt reds Root-E) and the factory's fixed signature cannot return it. *A typed refusal variant* — `ScalarPortError` has one variant; adding one is a port change outside this story. |
| **D-16-3-O** | `run_bounded` (`deferred-work.md:938`) | **Fixed here** (routed by charter): `crates/maos-journey-test/src/lib.rs:724-761` drains stdout and stderr on two threads while the bounded `try_wait` loop runs; proven by a child writing 1 MiB to each stream exiting inside the bound. | *Re-route* — this story owns subprocess supervision. |
| **D-16-3-P** | Residual receipt shapes (i)/(ii) | **Stated, unchanged owner (`epic-16-retrospective`)**, with (ii) corrected into its two measured shapes: `on_unload` **`Panicked`** ⇒ `check_hook_outcome` spawns `handle_crash` (`scheduler_loop.rs:576-593`), which in production (detector set, `main.rs:2911`) revokes and removes the SCB and writes UnplannedCrash receipts that may race exit; **`BudgetExceeded`** (`:565`) writes no receipts and leaves an `Unloaded` SCB in the map with tokens unrevoked (`:483` vs `:514-520`). | — |
| **D-16-3-Q** | How the supervision is structured (operator directive on Q2: *"observer. use design patterns"*) | **Named patterns, each mapped to a mechanism already ruled above — no pattern without a job, no trait without two implementations or a test double.** (1) **Ports & Adapters** — `WorkerSupervision` is the port `run_cli_wrapper_manifest` depends on; `WorkerSupervisor` is the kernel-backed adapter; a test double implements the port so `worker_spawn` vectors run without a kernel (the 16-1 `OperatorCommandPort` / `OperatorDoor` shape). (2) **Observer** — the child process is the subject; an exit observer publishes exactly one `WorkerExitEvent { spirit_pid, cause }` to its single subscriber, the shared **`Arc<BindingCore>`** (`BindingCore::on_exit`) — the lock-guarded state both the observer thread and the Worker's call stack reach. **`WorkerBinding` is the RAII handle over that core, held by the call stack only** (the observer can never reach the guard, which `finish(self)` consumes). The observer knows the platform and nothing about the kernel: it never calls `handle_crash`, `unload` or the TL itself — the core decides. (3) **Strategy** — `trait ExitObservation: Send + Sync { fn watch(&self, child_pid: u32, core: Arc<BindingCore>) -> ObserveOutcome /* Watching | Unavailable(Errno) */; fn signal_kill(&self, core: &BindingCore) -> SignalOutcome }` with `SignalOutcome::{Signalled /* pidfd_send_signal returned Ok */, AlreadyReaped /* ESRCH */, NotSignalled /* no pidfd */}` — **the one pidfd-using act (the `Signal` action's executor) is a strategy method too**, so every pidfd call lives in the strategy module (no `ChildHandle` — the bridge's `child` is private, only `child_pid()` is public, `runtime.rs:298,552`; and `maos_domain::supervision::ChildHandle = u64`, `supervision.rs:156`, belongs to the rejected `SubprocessSupervisor`) with two implementations: `PidfdExitObservation` (Linux; opens the pidfd — **a per-Worker `pidfd_open` or observer-thread-spawn failure after the root's probe succeeded (`EMFILE`, `ENOMEM`) is not an error: `watch` returns `ObserveOutcome::Unavailable(errno)` and the SUPERVISOR (never the strategy, which makes no TL call) journals `worker.exit-observer-unavailable:<errno>` as a `telemetry.event` for that binding at its pid, outside the lock (kind 4 is already in `NON_CALL_KINDS`); if `pidfd_open` failed the Worker runs without a child handle and the no-pidfd stop cost applies; if only the observer-thread spawn failed, the stored pidfd is KEPT (a stop can still signal) and exits are handled by `finish`; nothing `unwrap`s; `ESRCH` cannot occur here, nothing reaps before `wait_and_finalize` or the bridge's `Drop`** — stores it into the core under its lock — where the stop-before-pidfd `Signal` is decided — and starts D-16-3-B's `waitid(EXITED|NOWAIT)` thread) and `AfterPumpExitObservation` (every other platform, or when the root's one-time `pidfd_open` probe fails — **starts no watcher and stores no pidfd**). Handling an exit from the bridge's `ExitCause` is NOT a strategy method: it is `BindingPhase::finish(Running, cause)`, shared by both strategies (it is also Linux's path when the observer loses the reap race). **Fallback cost, stated:** with no pidfd a stop cannot signal the child — `maosctl unload worker` marks `PlannedStop` while the process keeps running to its own exit, and a root SIGTERM with such a Worker still bound exits 1 after the 5 s grace, printing `maos: worker <id> could not be signalled (no pidfd); exiting without the door shutdown and the audit-channel drain` and orphaning it (§11 row 6). **Three platforms build this code, with three different gates.** (i) **pidfd is Linux-only in rustix** (`rustix-1.1.4/src/process/mod.rs:24-25` `#[cfg(target_os = "linux")] mod pidfd;`, `wait.rs:397` `WaitId::PidFd`), and **macOS is a shipped target** (`.github/workflows/release.yml:49-50,73`, `aarch64-apple-darwin` on `macos-latest`, `cargo build --release -p maos-bin`) — so `PidfdExitObservation`, the stored pidfd (held by the strategy's per-binding handle inside `BindingCore`, and exposed on `WorkerBindingView` for the corpora) and every `rustix::process` call are `#[cfg(target_os = "linux")]` **inside the strategy module only**, with `AfterPumpExitObservation` as the non-Linux implementation (`signal_kill` ⇒ `SignalOutcome::NotSignalled`). (ii) **The listener's `tokio::signal::unix` streams** (which exist on macOS) are `#[cfg(unix)]`. (iii) **Windows builds maos-bin** (`windows-check`, `windows-latest`, `cargo test -p maos-bin --test jetbrains_acp_server` with default features incl. `network`, `.github/workflows/discipline.yml:2830-2858`, a `v1-0-ship-gate` need): the listener's `#[cfg(not(unix))]` twin creates **`tokio::signal::windows::ctrl_c()`** synchronously in the site's own statement (`tokio-1.52.3/src/signal/windows.rs:46`, returns `io::Result<CtrlC>`; the async `tokio::signal::ctrl_c()` would register only when first polled, after the `armed` marker) and the task awaits `.recv()` — the twin pattern of `shutdown_unix_term` (`main.rs:8764-8776`). No per-PR job builds macOS: the Linux gate is the only thing standing between a `cfg(unix)` slip and a broken release. The strategy is chosen ONCE per root when the `WorkerSupervisor` is built and the choice is journaled (`worker.exit-observer-unavailable:<errno>` for the fallback); all `#[cfg(target_os = "linux")]` lives in that one module. (4) **State** — `BindingPhase { Running, CrashHandled, PlannedStop, ExitedClean }` owns its transitions as methods (`on_pidfd_stored()`, `on_exit(cause)`, `begin_stop()`, `finish(bridge_exit)`, `abandon()`), each a pure function of (phase, input) that returns the next phase and a **`BindingAction`**; illegal transitions are unrepresentable or return a typed refusal, and every transition is unit-tested as a table (the (t) interleavings become that table). (5) **Command** — `BindingAction { Nothing, Signal, SpawnCrashHandler(CrashCause) /* observer, in-lock exception */, RunCrashHandlerThenMaybeDisposition(CrashCause) /* finish(Running, crash) */, JoinHandlerThenMaybeDisposition /* finish(CrashHandled) */, JoinHandlerThenMaybeDispositionThenUnload /* abandon(CrashHandled) */, DispositionAndUnload /* finish/abandon(PlannedStop) */, SignalThenDispositionAndUnload /* abandon(Running): signal through the strategy while the bridge is still alive, then disposition and unload */, UnloadClean /* finish(Running, exit 0) */ }` — the complete table, with `on_pidfd_stored`: `Running` ⇒ `Nothing`, `PlannedStop` ⇒ `Signal`; `begin_stop`: `Running` ⇒ `PlannedStop` + (`Signal` if a pidfd is stored, else `Nothing`), any other phase ⇒ `Nothing`; `on_pidfd_stored` from `CrashHandled`/`ExitedClean` ⇒ a typed refusal (unreachable: the pidfd is stored before the observer starts and before the pump; the unit table expects the refusal); `abandon`: `Running` ⇒ `PlannedStop` + `SignalThenDispositionAndUnload` (the signal is `NotSignalled` without a pidfd) — never `PlannedStop` alone, because the unload's own `on_unload` → `begin_stop(PlannedStop)` yields `Nothing` and a child spawned after bind would never be signalled; `on_exit`: `Running` + crash ⇒ `CrashHandled` + `SpawnCrashHandler`, `Running` + exit 0 ⇒ `Nothing` (`finish` owns clean exits), any other phase ⇒ `Nothing`; `finish`/`abandon` from `ExitedClean` ⇒ `Nothing`; handler-join timeouts ⇒ `Nothing` (left to `join_outstanding`); **every `Disposition*` executor is a no-op once `dispositioned` is set**; the unit-test table covers every (phase × input) cell — is DECIDED under the binding lock and EXECUTED by the supervisor after the lock is released — which makes D-16-3-D's *lock → decide → release → act* structural instead of a code-review rule (the one in-lock act, `Handle::spawn` of the crash handler, is the `SpawnCrashHandler` executor's documented exception). (6) **RAII guard** — `bind` returns a `#[must_use] WorkerBinding`; after `spawn_and_bridge` the guard and the bridge are held together in **`struct LiveWorker { binding: WorkerBinding, bridge: SpawnedBridge }` — guard declared FIRST, because struct fields drop in declaration order**: `SpawnedBridge` has its own `Drop` that `child.kill()`s and reaps (`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:791-800`), so with two separate locals (bridge declared later, dropped first) every early return would kill the child BEFORE the guard ran — the guard's signal would prove nothing, and the bridge's SIGKILL landing in `Running` could race the observer into a spurious crash path (measured 7/200 and 9/200 by validation round 8). With `LiveWorker`, the guard's `abandon` sets `PlannedStop` and signals through the strategy while the bridge is still alive, then the bridge drops and reaps. `finish(self, bridge_exit)` consumes it and **disarms the guard before executing its action** (so `Drop` never runs `abandon` after `finish`); its `Drop` without `finish` performs `abandon` — so *every return between bind and finish goes through `abandon`* holds by construction (D-16-2-A's structural rule), including a `?` a later edit adds. `Drop` never panics and runs `abandon` through the port's sync entry (legal: the guard lives on the blocking/`block_in_place` thread). **Test seams:** the stop-during-mint callback (called by `run_cli_wrapper_manifest` between the mint and the pre-spawn phase check), the observer gate + observation counter, a **signals-sent counter** (incremented only on `SignalOutcome::Signalled`, beside the `ECHILD` counter), and the injected-`?` point (**pinned after `ExitObservation::watch` has stored the child handle and started the observer, and before `pump_to_journal`** — after `spawn_and_bridge` but before `watch` even a correct build has nothing to signal) are `#[doc(hidden)] pub` seams on `WorkerSupervisor`, set through a constructor option — never `#[cfg(test)]` (integration tests in `crates/maos-bin/tests/` are separate crates and cannot see it, and inline test code is charged) and never an env var. Each pattern is named in the item's rustdoc. Rejected shapes are recorded in the next column. | *A hand-written `match` on an `AtomicU8` phase with inline kernel calls* — the lock-held-`unload` deadlock (round 4) and the stop-before-pidfd window (round 2) were both this shape. *A generic event bus / publish-subscribe registry* — one subject, one subscriber; a bus is ceremony. *The guard as the observer's subscriber (the ratification draft)* — `finish(self)` consumes it; the observer thread cannot reach it. *A strategy that "publishes from `finish`" (the ratification draft)* — on Linux it left the observer-lost-the-race crash unhandled. *A `SubprocessSupervisor` impl (the kernel trait)* — its `spawn_child` owns the spawn `spawn_and_bridge` owns (D-16-3-C). *Explicit `abandon` calls at each `?`* — correct only until the next edit. |

---

## Acceptance Criteria (6)

1. **AC1 (command — the crash scene and the crash floor).** From the repo root, with a scratch
   `HOME`/`MAOS_HOME`, `maos run <scratch manifest: worker-cli-fixture, T3, argv_prefix ["--maos-fixture-mode=sigkill-self"], [on_crash] action = "escalate-to-operator">`
   exits non-zero (`not_completed:process_crash`), and its Transparency Log holds, **at one `spirit_pid` ≠ 0**:
   `lifecycle.admit`, `lifecycle.load`, `lifecycle.start`, kind-21 output, `cli.subprocess.exit` with
   `signaled(sig=9)`, the `terminate_spirit` marker + receipt, `task.orphaned` (payload `cause: "fault.signaled"`,
   `exit_signal: 9`, `task_id: "run:worker"`, `originator_spirit_id: "operator"`), `lifecycle.crash` and the
   `worker.completion-verdict` row; plus a `task.escalated` row **at pid 0 whose payload `task_id` is `run:worker`**.
   Exactly one `cli_wrapper_loaded` line and no `spirit_loaded` line are printed. `cargo test -p maos-bin --release
   --test worker_crash_corpus_16_3 -- --ignored` (job `nfr-rel-1-crash-detection-2s`) passes D-16-3-K's two floors
   over exactly 100 kills, half with a pipe-holding grandchild. **Proven red:** with the observer thread not started,
   the 2 s floor fails (≤ 50/100) — recorded, reverted.
2. **AC2 (hang floor and the watchdog's placement).** `cargo test -p maos-bin --release --test
   worker_hang_corpus_16_3 -- --ignored` (job `nfr-rel-2-hang-detection-60s`) passes D-16-3-K's hang assertions.
   **Binary-level vector** (NOT ignored, `crates/maos-bin/tests/worker_supervision_16_3.rs`): `maos run <fixture manifest, --maos-fixture-mode=hang, [supervision] progress_threshold_ms = 5000>`
   writes `task.stalled` at the Worker's pid within 8 s of `cli_wrapper_loaded` (then the test SIGTERMs the root:
   AC4 (4)). **Cursor vector** (NOT ignored, in-process, same file): three `line-then-hang` Workers and NOTHING else
   printing, threshold 5000 ms ⇒ all three `task.stalled`. **Proven red twice:** (1) with the stamper's cursor made
   inclusive, the cursor vector's last-printing Worker never stalls — recorded, reverted (the hang corpus cannot carry
   this red: its chatty Workers print the newest row every tick); (2) with the ProgressWatchdog spawned only at the serving tail (`:7989`), the binary
   vector writes no `task.stalled` — recorded, reverted.
3. **AC3 (the wiring, every path).** D-16-3-B…I and Q hold, each proven in `worker_supervision_16_3.rs` — plus a table-driven unit test of every `BindingPhase` transition and its `BindingAction` (D-16-3-Q (4)/(5)), a `worker_spawn` vector against a test-double `WorkerSupervision` (Q (1)), and a vector where an injected `?` after spawn drops a `LiveWorker` of a **`--maos-fixture-mode=hang`** Worker (not `hang-with-grandchild`: the bridge's `Drop` joins its reader threads, `runtime.rs:805-807`, which the grandchild's inherited pipes block forever; not `sigkill-self`/`exit-nonzero`: the observer can reach `CrashHandled` before the `?` fires) and leaves `lifecycle.unload`, a PlannedUnload receipt, the stop disposition, the SCB gone from `scbs()`, the child gone, **no** `lifecycle.crash`, AND the supervisor's signals-sent counter equal to 1 for this Worker; **proven red:** with the signal removed from `SignalThenDispositionAndUnload`, or the fields of `LiveWorker` reordered, the signals-sent counter is 0 — every run (validation round 9 probed both mutations 200×: every other listed observable stays green when the signal is removed, because the bridge's own `Drop` kills and reaps the child) — recorded, reverted (Q (6)): (a) standalone and
   (b) topology Workers carry a real pid on every row AC1 lists; a topology with two Workers gets ids `worker` and
   `worker-2`; (c) a host-B cohort-daemon Worker (T0 names the harness, e.g. `two_host_delegation_2b.rs`) does the
   same, and a host-B governance refusal after bind leaves `lifecycle.unload` and no `task.stalled` 35 s later; a
   Worker exiting 0 leaves `lifecycle.unload` + a PlannedUnload receipt and **no** `task.orphaned` and no disposition
   row; `exit-nonzero` ⇒ `fault.non-zero-exit` `exit_code: 3`; `maosctl unload worker` against a live `hang` Worker
   ends the child (the test opens a pidfd on the `child_pid` from `cli_wrapper_loaded` and `poll`s it for `POLLIN`
   with a timeout — a pidfd `waitid` only observes the caller's OWN children and returns `ECHILD` at once for a
   `maos` child, and `kill(pid, 0)` succeeds on a zombie; neither is evidence), leaves a PlannedUnload receipt, a
   `task.nacked` row for `run:worker` (the stop disposition), exactly one `SpiritUnload` revoke row for that pid and
   **no** `lifecycle.crash`; the same with `[on_crash] action = "escalate-to-operator"` ⇒ `task.escalated` (the
   action captured at bind, not the default); a stop issued during the mint (a test hook on the supervisor) never
   spawns the child, and leaves the SCB gone from `scbs()` and a PlannedUnload receipt (no lock held across `abandon`); a Worker that would bind after `stop_workers()` is refused `WorkerStopping`; two concurrent
   Workers' completion verdicts are each computed from their own pid's rows; a supervised Worker completes on a
   `worker_threads = 1` runtime; **reap-race vector:** a supervisor test seam gates the observer thread after the pidfd is stored and BEFORE its `waitid` CALL; the test opens the gate only after it sees the Worker's `cli.subprocess.exit` row at its pid (journaled after `child.wait()` reaps, `runtime.rs:648,670`) ⇒ with an `exit-nonzero` Worker, `task.orphaned`, `lifecycle.crash` and the disposition row are all present AND the seam's observation counter shows the observer received `ECHILD` (so a gate placed after the return reds); **proven red:** with `finish(Running, crash)` changed to `Nothing` the vector reds — recorded, reverted; `maos run` with `MAOS_ONE_SHOT` set exits 2; `Cargo.lock` byte-unchanged;
   **Review amendment:** the operator authorized the narrow kernel delta required
   to represent a missing capability token honestly; no zero-token sentinel is
   permitted. A `CliSubprocessExit` revoke row is asserted only if T0 finds a
   hermetic way to grant `proc.exec`; else recorded.
4. **AC4 (NFR-Rel-11's planned half at every root; epic AC4 as corrected).** D-16-3-H/M/N hold, each proven in NEW
   `crates/maos-bin/tests/root_shutdown_unload_16_3.rs` (enrolled as a second step of
   `nfr-rel-11-halt-receipt-999pct`, which builds `maos-bin`, `maos-cli` (`maosctl`, `crates/maos-cli/Cargo.toml:10-11`) and `-p worker --bin worker-cli-fixture` **with the same `--release` profile as its `cargo test --release` run** — `maosctl` is found as the `maos` binary's sibling (`one_daemon_one_door_16_1.rs:61-66`) and the fixture as a sibling or parent of the running exe (`worker_spawn.rs:110-123`) — and runs `--test-threads=1`):
   (1) butler serving root, `MAOS_IDLE_FAST=1`: `maosctl halt list --spirit butler` lists the `belief_variance` halt
   (its filter is by pid, so a listed halt IS at butler's pid); SIGTERM ⇒ before exit a `lifecycle.unload` row and a
   PlannedUnload receipt **whose `halt_id` equals the raised halt's** (the stdout `HaltId("…")` Debug form
   normalised; never a `term-…` id), exit 0, no `drain timed out`, within 16-1's `< 3 s`; (2) butler `--once` ⇒ the
   same receipt, **no `drain timed out`** (red at HEAD, probed) and the unload's `SpiritUnload` revoke row; (3)
   topology `--once` ⇒ every loaded Spirit has a `lifecycle.unload` row; (3b) SIGTERM during a topology `--once`
   `hang` Worker entry ⇒ the Worker's `lifecycle.unload` + PlannedUnload receipt + `task.nacked`, every previously
   loaded Spirit's `lifecycle.unload`, no later entry loaded, no `drain timed out`, the `interrupted by signal` line
   and **exit 1**; (4) SIGTERM during a standalone `hang` Worker ⇒ the child is gone (pidfd `poll`, as in AC3),
   PlannedUnload receipt, exit non-zero, no orphaned process, no `drain timed out`;
   (4b) SIGTERM during a standalone `hang-with-grandchild` Worker ⇒ the direct child is gone, the PlannedUnload
   receipt is present, the root exits 1 within 5 s + 2 s printing the descendant line, and the grandchild is still
   alive (the test then kills it) — the stated 17-1 residual, observed; (5) cohort-a2a-daemon SIGTERM with a live
   host-B `hang` Worker ⇒ child gone, `lifecycle.unload` + PlannedUnload receipt (synchronous TL rows), and — because
   this story adds the cohort writer await — no `drain timed out` and the `SpiritUnload` revoke row; (5b) a cohort daemon:
   wait for `maos: root supervision armed`, then SIGTERM before `cohort-a2a-daemon listening on` ⇒ the process EXITS
   THROUGH ITS TEARDOWN (`status.code().is_some()`, not death by signal 15); **proven red:** with the `root_shutdown`
   arm removed the test times out — recorded, reverted. Because the arm-removed build only fails if the SIGTERM lands before the daemon's own streams exist (`main.rs:9945-9950`), the daemon's `select!` is `biased;` with the `root_shutdown` arm first, and the vector counts a run only when stderr LACKS `maos: cohort-a2a-daemon received SIGTERM` (printed only by the daemon's own stream, `:9948`); it retries up to 5 times, fails if no run counted, and records how many did; (5c) cohort SIGTERM with a host-B `hang-with-grandchild`
   Worker ⇒ exit 1 within 7 s printing the descendant line, grandchild still alive (the test kills it); (6) 16-1's idle
   SIGTERM test (`one_daemon_one_door_16_1.rs:472-489`) green unchanged; (7) a topology `--once` whose recorder flush
   fails (the zero-completions injection, `inference_mode_15_6.rs:466-500`) still unloads and drains; (8) **the
   supervision did not leak into the one-shot arms:** `MAOS_ONE_SHOT=acp-server` (harness `tests/jetbrains_acp_server.rs:24`)
   with stdin held open: send `session_start`, read `session_ready` (so a leaked registration would already be in
   place), then SIGTERM ⇒ the process dies by signal 15 within 2 s (T0 records this HEAD behaviour first;
   if it differs, T0 names a one-shot that dies by signal 15 at HEAD). Residuals D-16-3-P stated on the
   `epic-16-retrospective` row, not claimed.
5. **AC5 (FR4 at a Worker's pid).** D-16-3-J holds. Classifier vectors in NEW `crates/maos-audit/tests/fr4_worker_rows_16_3.rs`:
   a kind-21 row with or without a token ⇒ `NonCall`; a token-bearing kind-1 `task.orphaned` with the ruled payload ⇒
   `NonCall`; the same row tokenless ⇒ `Call`; a kind-7 row with intent `task.orphaned` and the same payload ⇒ `Call`;
   `in_flight_tokens` flat, empty, or with a 15-number element ⇒ `Call`; `task.nacked`/`task.escalated`/`task.reassigned`/`distillate.redacted`
   by their shapes ⇒ `NonCall`; flipping `task.orphaned`'s entry to `Call` reds a vector. `fr4_classifier_16_2.rs`:
   every pre-existing assertion holds; its table count and scanner are extended (D-16-3-J (5)), and adding a kind-1
   `TaskComplete` `insert_frame_event*` site with no disposition reds it. A regression vector reads a REAL kind-15 `task.stalled`
   row back through `maos_audit::query` and asserts `NonCall` (16-2's `kind_to_string` arms `crates/maos-audit/src/lib.rs:708-713` already name it; the guard keeps a Worker's stall rows out of the call feed). With `enterprise_runtime` configured, the Worker's tokenless kind-30 `identity.asserted` row at its pid classifies `NonCall`. **Binary leg** (in `worker_supervision_16_3.rs`, after AC1's run AND after AC2's binary hang vector):
   `maos audit query --spirit worker --format ndjson` exits 0, emits only call rows (six keys), and its stderr
   omission line contains `cli.subprocess.output×` and `task.complete×1` (AC1) and `task.stalled×` (AC2). FR4 NDJSON for a call-only DB is
   byte-identical; `audit_query_fr4_smoke.sh` and `v01_evaluator_path.sh` green.
6. **AC6 (harness, CI, honesty).** D-16-3-K/L/O hold: fixture modes (unknown mode exits 2; a mode flag does not change
   the probe envelope; default output unchanged by its own vector); both corpus jobs build the fixture, run with
   `-- --ignored`, and red on anything but `1 passed`, retry-once stated in the step; `run_bounded` survives a 1 MiB-per-stream
   child; `SCANNED_SOURCE_FILES` 22; `demo_j1`/`journey_j1` pins green unchanged; the no-default-features build green;
   the §11 rows filed in `deferred-work.md` with owners (rows 7, 9, 10, 12, 13, 14); the epics' §12 edits present and
   exit-block commands byte-unchanged.

---

## Review obligations (§A6)

**(a)** Run AC1's manifest from a scratch dir; `maos audit query --spirit worker --format ndjson` exits 0. **(b)** Remove
the pidfd observer ⇒ the crash corpus's 2 s floor reds on the grandchild half. **(c)** Make `finish` call `handle_crash`
unconditionally ⇒ AC3's exit-0 vector reds (a clean exit writes `task.orphaned`; a second call after the observer's is a
silent `NotLoaded`, so that is not the red). **(d)** Delete `WorkerSpirit::on_unload`'s signal ⇒ AC3's `maosctl unload`
vector reds (AC4 (4) is driven by `stop_workers()`, not `on_unload`). **(e)** Spawn the ProgressWatchdog only at `:7989` ⇒
AC2's binary vector reds. **(f)** Make the stamper cursor inclusive ⇒ AC2's cursor vector reds (not the corpus). **(g)** Move `unload_all_loaded` before the
watchdog join ⇒ read the order; no vector can race it deterministically — say so. **(h)** Revert the butler pid binding ⇒
AC4 (1) reds on the `term-` id, not on row presence. **(i)** Remove one `audit_tx` drop ⇒ AC4 (2) reds on `drain timed
out`. **(j)** Run both corpora twice back to back; record whether the retry was consumed. **(k)** `check-kernel-baseline`
`changed == 0`; `git diff Cargo.lock` empty; `cargo deny check bans`. **(l)** Remove kind 21 from `NON_CALL_KINDS` ⇒ AC5
reds. **(m)** `rg -U 'wait_and_finalize\(\s*&transparency_log,\s*0\b' crates/maos-bin/src` ⇒ 0 (multiline — rustfmt splits
the call). **(n)** Read `main.rs` from each Worker-bearing root's first `load` to its return: no post-`start` `?` bypasses
the teardown on the `--once` tails or the standalone arm. **(o)** SIGKILL `maos run` during a Worker ⇒ the child still
orphans — confirm nothing claims otherwise. **(p)** Corpus denominators asserted before floors; zero-test job runs red.
**(q)** Every corpus assertion reads rows the kernel writers wrote (E15-A6). **(r)** Move `RootSupervision`'s construction
before the run block ⇒ AC4 (8) reds (the acp-server one-shot swallows SIGTERM). **(s)** Make the listener `unload` on its normal
path ⇒ read the topology loop: a Spirit unloaded mid-load is the defect (the grace-timeout path unloads and then exits — the
one exception). **(u)** Read `supervision.rs` for D-16-3-Q: no kernel call inside the exit observer; every kernel call executes a `BindingAction` outside the lock; `cfg(target_os = "linux")` and every `rustix::process`/pidfd reference only in the strategy module (macOS release builds maos-bin); only the listener's signal streams carry `cfg(unix)`/`cfg(not(unix))` (Windows builds maos-bin); delete `WorkerBinding::drop`'s `abandon` ⇒ the injected-`?` vector reds; `LiveWorker` declares `binding` before `bridge`; no `OwnedFd` outside the strategy module. **(t)** Enumerate the binding state machine's interleavings ({observer, stop, finish, abandon, door unload,
the `stopping` latch}) against the ruled phases — known and accepted: a genuine crash racing a door `unload` past both
step 1s makes both `handle_crash` (it ignores its transition error, `crash_detector.rs:121-124`) and `unload` write
receipts and a `SpiritUnload` row (a synthetic `term-…` receipt, a second revoke row; no vector asserts exact counts under
that race) — and otherwise no Worker SCB left loaded, no double `handle_crash`, no
record dispositioned twice.

---

## 11. Declared cut lines (rule 10 — each names an owner, never a bucket)

| # | Item | Measured evidence | Destination |
|---|---|---|---|
| 1 | **FR12's delivery of `task.orphaned` "to in-flight task originators"** — journaled, never mailbox-delivered | `crash_detector.rs:163-170`; dispositions are TL rows at pid 0; `reassign_task_to` enqueues nothing | **`epic-16-retrospective`** — no story's charter covers originator delivery of kernel crash reports; the retro decides a vehicle or drops the clause (FR51 precedent). Retro row (§12). |
| 2 | **Descendants of a stopped or crashed Worker survive; a root SIGTERM'd while one holds the pipe exits on a 5 s grace without its door shutdown and channel drain** | bare `Command::new`, no process group (`runtime.rs:461`); AC4 (4b) | **`17-1`** AC4 — `spawn_t3` becomes the single Worker path; container teardown ends the tree. Note in 17-1 AC4 (§12). |
| 3 | **`[cli_wrapper] recovery_policy` executor** (`respawn_fresh`) unwired | `execute_recovery`/`handle_subprocess_death` zero production callers; `deferred-work.md:44` names no owner | **`epic-16-retrospective`** — process recovery, not task disposition; no story holds it. Retro row (§12). |
| 4 | **Receipts skipped for loaded-never-started and `on_unload` `BudgetExceeded`** | `control_block.rs:444-455`; `scheduler_loop.rs:502,565` | **`epic-16-retrospective`** (existing row; D-16-3-P corrects (ii)) |
| 5 | **Pre-`start` and partial-load error returns in the run block** skip unload/drain | 83 `?`/`return` sites `main.rs:3984-5626`; topology load loop `:4403-4551` | **`16-6`** — rule 10(b): it extracts that admission block. Note in 16-6 AC1 (§12). |
| 6 | **Non-Linux crash detection is bounded by pipe EOF** | no pidfd | **Not cut — stated platform scope** in `supervision.rs` rustdoc and AC3; CI is Linux. Stated cost (also on Linux when the root's `pidfd_open` probe fails with `EPERM`/`ENOSYS`): without a pidfd a stop cannot signal the child — `maosctl unload worker` marks it stopped while it runs to its own exit, and a root SIGTERM with it still bound exits 1 after the 5 s grace with the `could not be signalled (no pidfd)` line and orphans it (D-16-3-Q (3)). Non-Linux: macOS has no pidfd in rustix (same fallback, `cfg(target_os = "linux")` gate); Windows uses `cfg(not(unix))` listener twins and never has a pidfd. |
| 7 | **TL redaction scrubs any `sk-` substring**, e.g. every `task-…` id | `redaction.rs:81`; probe | **`16-5`** — D3's audit-truth charter. `deferred-work.md` row at T7; note in 16-5 AC1 (§12). |
| 8 | **Kind-1 `task.orphaned` carries a padded capability token in the TL token column**; dispositions at pid 0 | `crash_detector.rs:161-170`; `adapter.rs:239-301` | **`16-5`** AC1's recorded decision on kernel rows that claim a call — note extended (§12). |
| 9 | **Worker SCB `sandbox_tier` shows T2 and the process runs bare** | `control_block.rs:251,362`; `runtime.rs:461` | **`17-1`**. `deferred-work.md` row at T7; note in 17-1 AC4 (§12). |
| 10 | **Tier-denial message prints `permitted == requested` when no grant exists** | `host_grant.rs:134-138` | **`17-1`** — rule 10(b). `deferred-work.md` row at T7; note in 17-1 AC4 (§12). |
| 11 | **A door `pause` on a Worker SCB does not pause the process** | `SpiritSchedulerAdapter::pause` flips state | **`epic-16-retrospective`** — the FR51(a)(d) item gains "Worker SCBs" (§12). |
| 12 | **Kind-1 kernel rows the FR4 doorbell cannot see or match**: `insert_kernel_event_returning_id` raw rows at a real pid (already red at HEAD) and the non-JSON `task.orphaned` at pid 0 | `transparency_log.rs:1364-1405`; callers `main.rs:5796,10411,10513`; `resolver.rs:223` | **`16-5`** — the same recorded kernel-rows decision (§12 note). `deferred-work.md` row at T7. |
| 13 | **`MAOS_ONE_SHOT` arms whose audit drains time out** (`smoke-epic-4`, `smoke-spirit-5`, `smoke-supervision-5`, `hello-spirit`) | measured ~5.06 s each at `af96c907`; `:6114-6116` | **`epic-16-retrospective`** — no open story's charter covers one-shot teardown; `hello-spirit` is the FR58 evaluator path (and `onb_nfr2_timing.sh`'s subject), so the retro must decide a vehicle. `deferred-work.md` row at T7; retro row (§12). |
| 14 | **A hot swap drains butler's pending halt with no receipt** | `crates/maos-kernel-core/src/halt/mod.rs:440` | **`epic-16-retrospective`** — kernel byte, no charter. Retro row (§12); `deferred-work.md` row at T7. |
| 15 | **NFR-Rel-1/2/11 coverage-matrix `gates:` rows** | `tests/coverage-matrix.yaml:1376,1386,1391`; the job ids are absent from `xtask/gate-registry.toml` | **`20-3b-gate-retirement-and-coverage-generator`** — charter: the coverage generator (15-3 filed the 33 matrix violations there). |

**Not cut (EFFORT, not SCOPE):** the exit observer (§2), the sync port on all three paths (§3), `RootSupervision` and the
progress source (§3), the corpora with two floors and a grandchild half (§4), FR4 at a Worker pid (§5), the
standalone/cohort/SIGTERM-during-Worker teardowns with a bounded grace (§6), the stop disposition, the `--once` tail and
standalone-arm restructure, the butler/Mira pid binding incl. the successor (§6), `run_bounded`. Each is required for one
of this story's own ACs to be true.

---

## Dev notes

**Reuse, do not reinvent.**
- **Kernel (all `pub`):** `SpiritSchedulerAdapter::{load, start, unload, scbs, resolve_pid}`; `CrashDetector::handle_crash`
  (the ONLY writer of `task.orphaned`); `supervision::enforce_disposition`; `ProgressWatchdog::{new, spawn}`;
  `HaltRegistry::{drain_for_spirit_dry_run, lookup_state}`; `SpawnedBridge::{child_pid, pump_to_journal, wait_and_finalize}`;
  `FrameFilter` + the keyset cursor.
- **16-1's sync-port-over-`Handle` pattern:** `crates/maos-bin/src/operator_door.rs`. **16-2's unload-at-exit:**
  `finish_shell_session` `shell_host.rs:222` — generalise the loop, keep its reporting contract.
- **Kernel world for in-process tests** (probe scout's compiled shape): `TransparencyLogAdapter::open_in_memory`;
  `CapabilityRegistryAdapter::new(RingCryptoProvider, Ed25519SigningKey, 0, PolicyTable, cap_audit::channel().0, CapQuotaTracker, WorkingMemoryStore, TelemetryStreamAdapter)`;
  `SpiritSchedulerAdapter::new(tl, cap, mem, iac, halts, metrics, None, …)`; `CrashDetector::new(scheduler.scbs(), tl, halts, cap, iac, metrics, journal)`;
  `Arc::get_mut(&mut scheduler).unwrap().set_crash_detector(…)` **before any clone**. T0 looks for an existing helper first
  (`smoke-supervision-5`, `main.rs:6285`). Tests use `#[tokio::test(flavor = "multi_thread")]`.
- **Fixture discovery:** `resolve_cli_binary` checks the running binary's sibling, then its parent dir, then `$PATH`
  (`worker_spawn.rs:99-135`); a test binary in `target/<profile>/deps/` finds `target/<profile>/worker-cli-fixture`.

**Files being modified — current state, change, must-not-break:**

| file | today | this story | must not break |
|---|---|---|---|
| `crates/maos-bin/src/supervision.rs` | — | NEW ungated lib module (`pub mod supervision;` in `lib.rs`); Worker items `network`-gated | — |
| `crates/maos-bin/src/worker_spawn.rs` | 872 lines; `run_cli_wrapper_manifest` `:380`; pid 0 at `:575` (probe, stays), `:637, :721, :728, :730, :861`; read-back `:775-807` | params; section parse gates; bind/abandon/finish; real pid; record; read-back by pid; active-path count | every gate's order and refusal text; D1 remote refusal `:648-683`; the verdict row UNCONDITIONAL `:850-866`; the revoke closure unconditional (D-16-1-V); 19-3's sync contract; the RELOCATION module doc (update it) |
| `crates/maos-bin/src/delegation.rs` | `HostBWorkerContext` `:611-629`; `spawn_blocking` `:682`; comment `:672-673` | `supervision: Arc<WorkerSupervisor>`; task ref; comment | D1; "no intake sink when absent"; one-Worker-at-a-time drain `:724-757` |
| `crates/maos-bin/src/main.rs` | call sites `:4279`, `:4709`; not-completed return `:4300-4305`; watchdogs `:7968-8000`; `cancel` `:7945`; teardowns §6; adapter `:588-649`, sites `:690, :4456, :5374`, callers `:3235, :4859`; cohort `:7633`, `:9649`, `:9710` | `Option<RootSupervision>` before `:3984` + sites; conditional serving watchdog + select arm; `block_in_place`; post-Worker shutdown check; five teardowns; `--once` tails + standalone arm in async blocks; cohort result/teardown/writer await; pid bindings; appended params | `Arc::get_mut(&mut scheduler)` strong count 1; the one-shot arms' behaviour; `maos: operator HTTP listening on`; 15-6 flush ordering (P13); `IdleWatchdog` only in the serving tail; 16-1 `is_door_root`; no `spirit_loaded` for Workers; Root-E hot swap |
| `crates/maos-bin/src/shell_host.rs` | `finish_shell_session` `:222`, reports-never-returns `:233-243` | delegates to `unload_all_loaded` | `shell_host_16_2.rs:325,346,571`; `journey_j0.rs:445-450` |
| `crates/maos-bin/Cargo.toml` | rustix `["fs"]` `:116` | `["fs", "process"]` | `Cargo.lock` unchanged; bans |
| `crates/maos-bin/tests/two_host_delegation_2b.rs` | struct literal `:648-666` | real supervisor | its assertions |
| `crates/maos-bin/tests/one_daemon_one_door_16_1.rs` | comments `:262-271,520-522` | corrected | every assertion, incl. `:472-489` and `:891-962` |
| `crates/maos-audit/src/fr4_classifier.rs` | `NON_CALL_KINDS` `:51-61`; `WriterShapeEntry` `:105-114`; kind-7 gate `:301-303` | `kind`/`token`; non-7 entries first; two variants; kind 21; kind-1 entries | NDJSON six keys; kind-7 behaviour |
| `crates/maos-audit/tests/fr4_classifier_16_2.rs` | count `:226-232`; scanner `:545-556`; reverse `:673-683` | extended for kind + token; `TaskComplete` sites | every pre-existing assertion |
| `spirits/worker/src/bin/worker-cli-fixture.rs` | probe or 3 lines | modes after the probe check; self-kill via the `kill` command; grandchild via `Command::new(current_exe())` | default output; probe envelope; **no new dependency** (`Cargo.lock` unchanged) |
| `crates/maos-journey-test/src/lib.rs` | `run_bounded` `:724-761` | concurrent drain | timeout message; `Pty` drop order |
| `.github/workflows/discipline.yml` | jobs `:1196-1221`, `:1252-1263`; needs `:3811-3812,3815` | fixture build; corpus runs `-- --ignored` + `1 passed` check; nfr-rel-11 second step | `aggregate.needs` membership |
| `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs` | `SCANNED_SOURCE_FILES = 21` `:880` | 22 | equality assert |

**Traps, in the order they will bite:**
1. **`Arc::get_mut(&mut scheduler)`** at `set_crash_detector` (`main.rs:2911`) — build the supervisor after it.
2. **`RootSupervision` is declared before `:3984` and filled inside** — never spawn anything before the run block; never hold its tasks in run-block locals.
3. **`handle_crash` needs runtime context** — `Handle::spawn` inside the binding lock; `finish` and the teardown join it.
4. **`Handle::block_on` on a runtime worker panics** — paths (a)/(b) inside `block_in_place`; `block_in_place` panics on a `current_thread` runtime (the default `#[tokio::test]`!).
5. **Open the pidfd before the pump, on the spawning thread; re-check the phase under the lock after storing it. `ECHILD` is normal.**
6. **Signal Workers through the pidfd, only from `Running`.**
6b. **Lock → decide → release → act.** `WorkerSpirit::on_unload` takes the binding lock; anything that reaches `unload` while holding it blocks to the 30 s hook cap and leaves an `Unloaded` SCB in the map.
7. **`start` before spawn; every return after bind goes through `abandon`.** `Loaded → Unloaded` is refused.
8. **`unload` does not drain the task ledger** — dispositions come from the binding's kept record; set `dispositioned` once.
9. **Ids must survive redaction** — no `sk-`, no 32 contiguous hex chars.
10. **`scbs()` read guard + `unload` = deadlock** — collect pids, drop the guard.
11. **`terminate_spirit` writes TWO kind-3 rows per halt, and a marker + synthetic `term-…` receipt when nothing is pending**; `tl.last_frame_id()` races under concurrency (`halt/termination.rs:63,83`) — assert receipt `halt_id`s, never frame-id linkage.
12. **`since_ns` is inclusive** — the stamper uses the keyset cursor.
13. **Never cancel an in-flight hook** — `--once` passes check `root_shutdown` between passes.
14. **tokio signal streams miss signals delivered before they were created, and never restore the default** — the serving and cohort `select!`s observe `root_shutdown`, not a second SIGTERM; tests send a signal only after `maos: root supervision armed`.
15. **`MAOS_IDLE_FAST` / `MAOS_SUPERVISION_FAST` are kernel reads** — no `env_contract.rs` registration.
16. **The fixture mode flag rides into admission's probe** — parse it after `--maos-bridge-probe`.
17. **`spirit_loaded` prints are pinned** (`demo_j1.rs`, `journey_j1.rs`) — Workers print none.
18. **A new `src/*.rs` in maos-bin reds `cohort_daemon_smoke_13_5c.rs`** until `SCANNED_SOURCE_FILES` is 22; `supervision.rs` must build without default features.
19. **Inline `#[cfg(test)]` is charged** — tests go in `tests/`.
20. **A kernel doc-comment byte reds the 16-0 pin** — `crash_detector.rs:80-86` documents a non-existent `on_child_exit`; Debug Log only.
21. **Cohort signature scan** (`enterprise_daemon_seam_13_5a.rs:84-97`) stops at the first `)` — append params.
22. **"Child gone" evidence differs by harness.** In-process corpora (the Worker is the test's own child): the pidfd's `waitid`. Binary tests (the Worker is a child of `maos`): `waitid` returns `ECHILD` at once — `poll` the pidfd for `POLLIN` with a timeout. `kill(pid, 0)` succeeds on a zombie in both. (`rustix`'s `poll` needs the `event` feature — tests may add it; T0 re-checks `Cargo.lock`.)
23. **Spawning tests isolate `HOME` per command** (`root_spawn_home_isolation_16_1.rs:128-160`) — 16-1's decoy door holds the runner's real `control.json` endpoint.
6c. **Guard before bridge.** Hold them in `LiveWorker { binding, bridge }` in that field order; two separate locals drop the bridge first and its `Drop` kills the child before the guard acts.
24. **Three platforms, three gates** (D-16-3-Q (3)): pidfd and every `rustix::process` call are `cfg(target_os = "linux")` inside the strategy module (macOS release builds maos-bin and rustix has no pidfd there — a `cfg(unix)` slip breaks the release, and no per-PR job builds macOS); the listener's `tokio::signal::unix` streams are `cfg(unix)`; the Windows twin uses the synchronous `tokio::signal::windows::ctrl_c()` (`windows-check` builds maos-bin). The in-process corpora and `worker_supervision_16_3.rs` carry `#![cfg(target_os = "linux")]` (the corpora need a pidfd).
25. **Ctrl-C sends SIGINT to the whole foreground process group** — a bare-spawned Worker receives it directly and may die before the listener's `PlannedStop`, classifying as a crash; the corpora and vectors send SIGTERM to `maos` only.

### Project Structure Notes

New files: `crates/maos-bin/src/supervision.rs` (lib; doorbell 21 → 22), `crates/maos-bin/tests/worker_crash_corpus_16_3.rs`,
`crates/maos-bin/tests/worker_hang_corpus_16_3.rs`, `crates/maos-bin/tests/root_shutdown_unload_16_3.rs`,
`crates/maos-bin/tests/worker_supervision_16_3.rs`, `crates/maos-audit/tests/fr4_worker_rows_16_3.rs`. No new crate, no new
lockfile entry, one feature added to an existing dependency, no new env var, no `verbs.rs` change, no `xtask/src` change, no
`coverage-matrix.yaml`/`gate-registry.toml` change. The review patch changes kernel-core's crash writer solely to preserve optional token state; the baseline is re-pinned in the same change.

### References

- [Source: `_bmad-output/planning-artifacts/epics/epic-16-one-daemon-one-door-j0-w1.md` — 16-3 section (Round 7 applied, §12), Kernel-Δ, Kloc asks, Dependencies]
- [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md:40` FR12, `:43` FR50; `prd/non-functional-requirements.md:20-21,30` NFR-Rel-1/2/11]
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md:294,596-610` — subprocess supervision, ProgressWatchdog]
- [Source: `_bmad-output/implementation-artifacts/16-2-shell-halt-registry-and-j0-scene.md` — D-16-2-A, D-16-2-C, D-16-2-G, §11 rows 6–7, §15 Q-A]
- [Source: `_bmad-output/implementation-artifacts/16-1-daemon-post-surface-and-verb-retarget.md` — sync port over `Handle`, `drain_started_tasks`, Trap 19 (butler pid 0), D-16-1-V]
- [Source: `_bmad-output/implementation-artifacts/deferred-work.md:44,49,210,352,938`]
- [Source: `xtask/kloc.toml:2,386,390,475,632,655-656`; `xtask/kernel-core-baseline.toml:481`]

---

## Tasks / Subtasks

- [x] **T0 — re-measure; compile the signatures; trust nothing in this file** (all ACs)
  - [x] `git status --short` clean; record HEAD; `kloc-check --json`; `check-kernel-baseline`; `check-exit-commands`; `check-epic-close-coherence`.
  - [x] **Compile inside maos-bin before any other code:** `pidfd_open`, `pidfd_send_signal`, `waitid(WaitId::PidFd, EXITED|NOWAIT)` with `features = ["fs","process"]`; `block_in_place(|| Handle::current().block_on(…))` on a one-worker runtime; `git diff Cargo.lock` empty after a full `cargo build -p maos-bin`; `cargo deny check bans`; the no-default-features build. Any failure ⇒ STOP, re-rule D-16-3-B/C.
  - [x] Re-run the probes on fresh binaries; record each HEAD red: SIGKILLed standalone Worker ⇒ 3 pid-0 rows; grandchild holds stdout ⇒ blocked; silent hang ⇒ no `task.stalled`; SIGTERM during a Worker ⇒ rc 143 + orphan; butler `--once` ⇒ `drain timed out`; butler halt at pid 0; `halt list --spirit butler` ⇒ 0; `MAOS_ONE_SHOT=acp-server` + SIGTERM ⇒ signal 15 (AC4 (8)).
  - [x] Re-derive every `main.rs` cite in §3, §6 and D-16-3-D/H/I/M/N; every path reaching `:7989`; confirm no scheduler lock is held across the successor factory's `create`.
  - [x] `[on_crash]`/`[supervision]` parse reachable and accepted alongside `[cli_wrapper]` on the path (if refused: maos-manifest edit, re-book). — **DISPROVED: neither section was parsed on the run path at all.** No maos-manifest edit needed; the parse was ADDED in `worker_spawn.rs` using the kernel loader's own extract-then-parse shape (its `load_bundle_from_file` is `pub(crate)`).
  - [x] What frame context each Worker caller holds for `WorkerTask`; `ttl_deadline_ns` readers; a hermetic `proc.exec` grant (AC3); the host-B harness; a kernel-world test helper; a TL change-notification seam (D-16-3-G). — **no TL notification seam exists ⇒ the stamper polls, as ruled; no reader of `ttl_deadline_ns` exists, so `u64::MAX` at bind breaks nothing.**
  - [x] Tests pinning Worker rows at pid 0 or the SSO/PDP pid (11.4c enterprise tests, 2a/2b, `check-j1-loopback-delegation`, sealed-capture verifiers). — **none. The break list is empty**; the pins are on event SETS, not pids.
- [x] **T1 — fixture modes** (AC6, D-16-3-L) — incl. the probe-path and default-output vectors.
- [x] **T2 — supervisor + Worker SCB** (AC3, D-16-3-B…F, I)
  - [x] `supervision.rs` per D-16-3-Q (port/adapter, `ExitObservation` strategy pair, `BindingPhase` state + `BindingAction` commands, `WorkerBinding` RAII guard): port, supervisor, ids, `WorkerSpirit`, mutex binding state, pidfd observer (Linux) + `ECHILD` no-op + fallback + unavailable row, handler storage, `stop_workers`, stop disposition, `bindings()`, active-path count, `join_outstanding`; doorbell 22; rustix feature.
  - [x] `run_cli_wrapper_manifest`: section gates; bind; `abandon` on every post-bind return; stop-before-spawn and stop-after-pidfd checks; real pid; record; `finish`; read-back by pid; module doc.
  - [x] Paths (a)/(b) under `block_in_place`; path (c) context field + appended params; `two_host_delegation_2b.rs`.
  - [x] `worker_supervision_16_3.rs` vectors incl. an interleaving vector per phase transition.
- [x] **T3 — progress + root supervision** (AC2, D-16-3-G/H) — `Option<RootSupervision>` + two sites; conditional serving watchdog + select arm; stamper with the keyset cursor; hang corpus; binary vector; both falsifiers run, recorded, reverted.
- [x] **T4 — crash corpus + CI** (AC1, D-16-3-K) — corpus with the grandchild half, per-Worker grandchild kill at +5 s, one poller; AC1 scene; observer falsifier run, recorded, reverted; jobs (`-p worker --release`, `-- --ignored`, `1 passed` check).
- [x] **T5 — shutdown and teardown** (AC4, D-16-3-M/N/P)
  - [x] `unload_all_loaded` + report; `finish_shell_session` delegates (16-2 tests green unchanged).
  - [x] Listener (stop only, joinable, active-path wait, bounded grace) + `root_shutdown` consumers (post-Worker and between entries in the topology loop, between `--once` passes, serving `select!`, cohort tail).
  - [x] Five roots in the ruled order; `--once` tails and the standalone arm in async blocks; cohort result + writer await; drain invariant with the new owners.
  - [x] `ButlerOrchestratorAdapter` pid binding at three sites incl. the successor factory; 16-1 test comments; Root-E green.
  - [x] `root_shutdown_unload_16_3.rs` (vectors 1–8, 3b, 4b) enrolled in `nfr-rel-11-halt-receipt-999pct` (`maos`, `maosctl`, fixture built; `--test-threads=1`).
- [x] **T6 — FR4 at a Worker pid** (AC5, D-16-3-J) — the kind-15 read-back regression vector; `identity.asserted` non-call; `kind`/`token`, two variants, seven dispositions, 16-2 test extension, vectors, binary leg; measured `maos-audit` raise (`16-3`).
- [x] **T7 — harness + honesty** (AC6, D-16-3-O) — `run_bounded` + vector; `deferred-work.md` rows for §11 rows 7, 9, 10, 12, 13, 14; close `:938` at done.
- [x] **T8 — sweep**
  - [x] `git diff` the epics: §12 rows present; exit-block commands byte-unchanged.
  - [x] **Windows:** `windows-check` is the build proof (the dev's Linux host cannot cross-build Windows, `discipline.yml:2841-2843`); run `cargo check -p maos-bin --target x86_64-pc-windows-msvc` only where that toolchain exists, else push the branch and read `windows-check` before `review`. — **attempted and BLOCKED on this host**: the target is installed, but `ring`, `libsqlite3-sys` and `stacker` fail their native build scripts (no MSVC C toolchain/linker). Recorded as UNPROVEN LOCALLY; `windows-check` is the proof and MUST be read before this story is accepted. The twin's API was verified against vendored `tokio-1.53.1`: `signal::windows::ctrl_c() -> io::Result<CtrlC>` and `CtrlC::recv(&mut self) -> Option<()>` match the code exactly.
  - [x] `cargo fmt --all -- --check`; `kloc-check` (measured review raises: maos-bin 21048, maos-audit 7341, maos-kernel-core 18938); `check-kernel-baseline` (review-authorized 24474 → 24477, two changed files); `check-env-contract`; `check-workspace-count`; `check-exit-commands`; `check-epic-close-coherence`; `cargo deny check bans`; no-default-features build; `bash tests/integration/audit_query_fr4_smoke.sh`; `bash tests/integration/v01_evaluator_path.sh`; both corpora `--release -- --ignored`; `cargo test --workspace --no-fail-fast`.
  - [x] Story row → **`done`** after all review findings were resolved; the external `windows-check` limitation remains recorded as verification context.


### Review Findings

- [x] [Review][Patch] [High] Model missing Worker capability tokens honestly [crates/maos-bin/src/supervision.rs; crates/maos-kernel-core/src/supervision/crash_detector.rs] — `Option<TokenId>` now preserves absence as SQL NULL and an empty `in_flight_tokens` array; baseline and KLOC pins measured and updated.
- [x] [Review][Patch] [Medium] Prove AC5 with one stall-then-crash root [crates/maos-bin/tests/worker_supervision_16_3.rs] — one root now emits `task.stalled`, is killed, and is queried for the complete FR4 omission set.
- [x] [Review][Patch] [High] Release terminal Worker pidfds and binding cores [crates/maos-bin/src/supervision.rs]
- [x] [Review][Patch] [High] Spawn and park the fallback crash handler so its timeout is effective [crates/maos-bin/src/supervision.rs]
- [x] [Review][Patch] [High] Bound stderr-tail retrieval before crash classification [crates/maos-bin/src/supervision.rs]
- [x] [Review][Patch] [Medium] Bound concurrent `run_bounded` output capture [crates/maos-journey-test/src/lib.rs]
- [x] [Review][Patch] [High] Bind cold-swap Butler ports to the actual successor pid [crates/maos-bin/src/main.rs]
- [x] [Review][Patch] [Medium] Use an insertion-order cursor for Worker progress [crates/maos-bin/src/supervision.rs]
- [x] [Review][Patch] [High] Count orphan and disposition latency on the same Worker [crates/maos-bin/tests/worker_crash_corpus_16_3.rs]
- [x] [Review][Patch] [High] Route topology recorder-flush failures through teardown [crates/maos-bin/src/main.rs]
- [x] [Review][Patch] [High] Fail otherwise-successful roots when planned unload fails [crates/maos-bin/src/main.rs]
- [x] [Review][Patch] [Medium] Enforce AC2's eight-second binary stall bound [crates/maos-bin/tests/worker_supervision_16_3.rs]
- [x] [Review][Patch] [Medium] Add AC3's live-Worker operator-door unload vectors [crates/maos-bin/tests/worker_supervision_16_3.rs]
- [x] [Review][Patch] [Medium] Prove AC3(c) on the host-B dispatch path [crates/maos-bin/tests/two_host_delegation_2b.rs]
- [x] [Review][Patch] [Medium] Assert AC1's crash receipt payload, not only its marker [crates/maos-bin/tests/worker_supervision_16_3.rs]
- [x] [Review][Patch] [Medium] Drive AC3's two-Worker topology wiring through the binary [crates/maos-bin/tests/worker_supervision_16_3.rs]
- [x] [Review][Patch] [High] Apply the 60-second floor to each Worker's hang latency [crates/maos-bin/tests/worker_hang_corpus_16_3.rs]
- [x] [Review][Patch] [Medium] Verify AC4(4b)'s planned-unload receipt [crates/maos-bin/tests/root_shutdown_unload_16_3.rs]
- [x] [Review][Patch] [Medium] Pin AC1's escalated disposition to pid zero [crates/maos-bin/tests/worker_supervision_16_3.rs]
- [x] [Review][Patch] [Medium] Build the hang-corpus fixture in the active release profile [crates/maos-bin/tests/worker_hang_corpus_16_3.rs]
---

## 12. Epic OLD→NEW edits (rule 9 — APPLIED at story creation, 2026-09-15)

Every OLD anchor below was verified to exist exactly once in the file named before replacement; no row replaces a pin
value or a line number that is data.

| # | File / location | OLD (anchor) | NEW (summary; verbatim = the diff) |
|---|---|---|---|
| 1 | epic-16 header | before `**Wave / duration:** 3.5–5.5 weeks` | NEW **Round 7** paragraph: 16-3 re-derived at `af96c907`; all four ACs' premises disproved; pointer to this file |
| 2 | epic-16 stories table 16-3 row | `` kernel-Δ 0 (`FaultCause::SignaledByKernel` reused; no `CrashCause` variant) · maos-bin +40–80, +20–40 for AC4 (estimate) `` | review-amended: narrow kernel delta for honest optional capability tokens; measured baseline re-pin 24474 → 24477; maos-bin ≤ +1274, maos-audit ≤ +130, maos-journey-test ≤ +39 |
| 3 | epic-16 16-3 section | `` `CrashCause` is `maos-domain/src/supervision.rs:47`, not kernel — no variant is added) · maos-bin +40–80 `` | appends ⚠ R7 re-book pointer, then a ⚠ R7 SUPERSEDED paragraph summarising this story's 6 ACs; the original AC1–AC4 KEPT VERBATIM below it |
| 4 | epic-16 Kernel-Δ paragraph | `16-3's Worker SCB and task record are a maos-bin shim over `load`, not a kernel API.` | appends the creation-time R7 zero-delta rationale, then the 2026-09-16 review amendment authorizing `Option<TokenId>` and the measured re-pin |
| 5 | epic-16 Kloc asks | `16-3 +40–80` | `16-3 +40–80 (⚠ R7: ≤ +1274, 16-3 story §10)` |
| 6 | epic-16 16-5 AC1 | `the decision and its reason are recorded here.` | appends ⚠ R7: kind-1 `task.orphaned` token column + pid-0 dispositions + `insert_kernel_event_returning_id` raw kind-1 rows + the non-JSON `task.orphaned` join that decision; the redactor's `sk-` rule routed here |
| 7 | epic-16 16-6 AC1 | `the gate list is re-derived at T0 from the extracted function.` | appends ⚠ R7: the extracted function's error path owns the pre-`start`/partial-load unwind |
| 8 | epic-16 Kernel-Δ doorbell sentence | `it is **19** at HEAD` | appends ⚠ R7: 21 at `:880` at `af96c907` (16-1 and 16-2 each added one); 16-3's `supervision.rs` makes it 22 |
| 9 | epic-17 17-1 AC4 | `` The `t3-smoke-busybox` job drops its `continue-on-error` `` | inserts before it ⚠ R7: supervision above the spawn primitive; the first Worker is `worker`; container teardown owns descendants, `sandbox_tier`, the tier message |
| 10 | epic-19 19-3 AC3 | `` no substrate: one `host` entry pinned, `wait_and_finalize(…, 0, …)` pid 0, no Worker SCB `` | appends ⚠ R7: every Worker has an SCB and a real pid (ids `worker`, `worker-2`, `worker-3`); sync fn under `block_in_place`; the FR21 reason stands on its own |
| 11 | `sprint-status.yaml` 16-3 row | `16-3-subprocess-crash-to-handle-crash: backlog  # wait_and_finalize` | `ready-for-dev` + summary comment, prior comment kept after `PRIOR:` |
| 12 | `sprint-status.yaml` epic-16-retrospective row | `16-3 AC4 states both as residuals. \|` | appends (ii)'s corrected split, then ⚠ ALSO OWES (Story 16-3): FR12 originator delivery; `recovery_policy` executor; Worker SCB pause; one-shot drain timeouts; hot-swap halt drain |

---

## 14. Open questions for the operator

**None open.** All six were ruled by the operator on 2026-09-15 (§15).

---

## 15. Operator ratification 2026-09-15

Verbatim: *"1. one story. 2. observer. use design patterns. 3. yes. 4. yes both. 5. yes. 6. do as suggested"*

| # | Question | Ruling | Applied at |
|---|---|---|---|
| **Q1** | WHOLE or split `16-3b` (AC4)? | **One story (WHOLE)** | frontmatter `split_from`; D-16-3-A |
| **Q2** | Linux pidfd observer or a kernel bridge split? | **Observer — built with design patterns** | D-16-3-B; NEW **D-16-3-Q** (Ports & Adapters, Observer, Strategy, State, Command, RAII guard); AC3 pattern vectors; review obligation (u); §10 re-booked (+20…+40 raw ⇒ maos-bin ×1.3 upper +1274) |
| **Q3** | Supervise host-B Workers here? | **Yes** | D-16-3-I |
| **Q4** | Standalone run's record (`run:<id>`, originator `operator`), and NACK/escalate a stopped Worker's task? | **Yes, both** | D-16-3-D, D-16-3-E |
| **Q5** | Review net non-degradable? | **Yes** | frontmatter `review` |
| **Q6** | SIGTERM with a descendant-held pipe: 5 s grace exit, or `/proc` walk? | **5 s grace exit, as suggested** | D-16-3-M; AC4 (4b), (5c); §11 row 2 |

---

## Dev Agent Record

### Agent Model Used

`anthropic/claude-opus-5` (frontier-class).

> Repaired 2026-09-20 (Epic-16 retrospective). This line previously repeated the enumerated family list from the `model:` frontmatter key, which carries the `allowlist`-plus-brace literal that `check_dev_model_used_populated.rs:302` treats as boilerplate and skips — so `check-dev-model-used-populated` and `check-dev-model-tier`, both in `aggregate.needs`, read this story as having no recorded model. The model itself was never in doubt; only its extractability was.

### Debug Log References

**T0 signature probe (ran before any other code, then deleted).** `pidfd_open` + `pidfd_send_signal(SIGKILL)` + `waitid(WaitId::PidFd, EXITED|NOWAIT)` on `sh -c 'sleep 6 & exec sleep 30'` with a piped stdout: the observer returned `terminating_signal = Some(9)` in **< 500 µs** while the grandchild still held the pipe; `NOWAIT` left the zombie and the subsequent `child.wait()` still classified signal 9; the post-reap `waitid` returned `ECHILD`. `block_in_place(|| Handle::current().block_on(spawn(..)))` completed on a `worker_threads = 1` runtime. The reap race was observable in the probe loop. All three D-16-3-B/C premises hold; nothing was re-ruled.

**HEAD reds re-measured, then closed.** SIGKILLed standalone Worker ⇒ three tokenless pid-0 rows (confirmed); butler `--once` ⇒ `drain timed out` in ~5.06 s; butler halt raised at pid 0 and `maosctl halt list --spirit butler` printing nothing while the row existed; silent hang ⇒ no `task.stalled` at all.

**Proven reds — every one RUN, recorded, and REVERTED (`cmp` byte-identical after each).**

| AC | Mutation | Observed red |
|---|---|---|
| AC1 | `PidfdExitObservation::watch` returns `Unavailable(0)` before spawning the observer thread | crash corpus: 100/100 kills sent (denominator held), `observer_unavailable=100`, **2 s floor met by only 53/100**, p99 **5.0166 s** — i.e. the grandchild half detected only when its GRANDCHILD died at +5 s, which is §2's measured claim exactly |
| AC2 (1) | stamper cursor made inclusive (`since_ns` high-water instead of the keyset cursor) | `three_workers_that_printed_once_then_hung_all_stall` FAILED |
| AC2 (2) | `RootSupervision::arm` spawns no ProgressWatchdog (serving-tail only, as at `af96c907`) | binary hang vector FAILED: `no task.stalled row at a real pid within 40 s; rows: []` |
| AC3 (a) | signal removed from `SignalThenDispositionAndUnload` | `signals_sent` **left: 0, right: 1** — and every OTHER observable in the vector stayed green, because `SpawnedBridge::drop` kills and reaps the child |
| AC3 (b) | `LiveWorker`'s fields reordered (bridge first) | identical red, `left: 0 right: 1` — validation round 8's prediction confirmed |
| AC4 (1) | D-16-3-N reverted (Spirit-supplied pid forwarded) | red on the **synthetic id, not row presence**: `left: ["term-butler-1-1789486330810565135"] right: ["01M2JV4WTWB590R93TKMHAYFQB"]` |
| AC4 (2) | one `audit_tx` owner drop removed from the single-class `--once` teardown | FAILED in 5.24 s with `maos run: audit writer drain timed out after 5s` |
| AC5 (a) | `task.orphaned`'s entry flipped to `Call` | `token_bearing_kind1_task_orphaned_is_a_non_call` FAILED |
| AC5 (b) | kind 21 removed from `NON_CALL_KINDS` | `kind21_subprocess_output_is_a_non_call_token_or_not` FAILED |
| AC5 (c) | an unlisted kind-1 `TaskComplete` writer site planted | the inventory doorbell FAILED, naming the site |
| AC6 | `run_bounded` reverted to its sequential drain | `child_filling_both_pipes_exits_inside_the_bound` FAILED after 10.02 s (`did not exit within 10s`); green at 0.07 s after the fix |

**Measured floors (final, on the restored tree).** Crash corpus, release, 100 Workers (50 holding a pipe-blocking grandchild): `lifecycle.crash` **100/100 within 2 s**, p50 11.0 ms / p99 15.5 ms; `task.orphaned` **100/100 within 5 s**, p99 3.2 ms; disposition **100/100 within 5 s**, p99 3.3 ms; `observer_echild=28` — the reap race is real and `finish` handles it. Hang corpus, release: **50/50** `hang` stalled at 31.031 s (= 30 000 ms threshold + one watchdog tick), **5/5** `line-then-hang` stalled at 32.034 s, **0/5** chatty stalled over a 35.07 s window, **60/60** reached `lifecycle.crash` after the pidfd SIGKILL. AC4: butler `--once` **20.07 ms** with no `drain timed out` (HEAD: ~5.06 s); serving butler SIGTERM-to-exit **20.14 ms** (16-1's budget is 3 s), receipt `halt_id` == the raised halt's.

**Premises DISPROVED by measurement during dev (all four recorded here rather than worked around).**
1. **`[on_crash]`/`[supervision]` were not parsed on the `maos run` path at all** — not "reachable but unverified". The kernel's own loader is `pub(crate)`; `worker_spawn.rs` now repeats its extract-then-parse shape. Without this, FR50 could only ever nack, and the `[supervision]` threshold was unreachable.
2. **`maos audit query` scopes to the LATEST boot nonce**, so AC5's binary leg as written — ONE query naming AC1's and AC2's omissions together — is unachievable: two roots writing into one `MAOS_HOME` never appear in one query (verified by dumping the SQLite; both row sets are present under their own `boot_nonce`). The leg is asserted PER ROOT instead, which is what the underlying claim needs.
3. **`delegation_leg` holds no live `audit_tx` clone on the butler `--once` path** — 16-1's serving-root pin chain (`delegation_leg → Mailbox → ScbTracker → … → audit_tx`) does not reach the registry there. Removing its drop did NOT red the drain; `capability` (constructed with `audit_tx.clone()`) did.
4. **`maos run` does not honour `MAOS_AUDIT_DB`** — it opens its log at `$MAOS_HOME/audit/transparency.sqlite` (that variable belongs to the cohort daemon root). Every binary vector reads the real path.

**Two guards this story legitimately moved, each with the measurement written in-code.** (a) `main.rs`'s `story_13_5a` dispatch-window guard: D-16-3-H site 2 moved the enterprise argument tokens to MEASURED +3 151 / +3 196, so the window goes 2 600 → **3 300** (measurement + ~100, not a round safe number) — this IS the "human look" its own comment demands, and its twin `tests/enterprise_daemon_seam_13_5a.rs` was re-run green. (b) `SCANNED_SOURCE_FILES` 21 → **22** for `supervision.rs`.

**One defect found in a sibling's CI edit and fixed:** the `nfr-rel-11-halt-receipt-999pct` job had been DUPLICATED. `yaml.safe_load` accepts duplicate keys silently (last wins); `serde_yaml` rejects them, so `xtask --test release_workflow_15_4` red on `duplicate entry with key`. The duplicate was removed and the two new steps folded into the original job in its historical position.

**Windows: UNPROVEN LOCALLY.** The target is installed but `ring`, `libsqlite3-sys` and `stacker` fail their native build scripts on this Linux host (no MSVC C toolchain). `windows-check` is the proof and MUST be read before acceptance. The `cfg(not(unix))` twin's API was verified against vendored `tokio-1.53.1`: `signal::windows::ctrl_c() -> io::Result<CtrlC>` and `CtrlC::recv(&mut self) -> Option<()>`.

**Stated as observed, NOT fixed:** AC4 (4b)/(5c) — a SIGTERM'd root whose Worker left a pipe-holding descendant exits 1 after the 5 s grace printing the descendant line, and **the grandchild survives** (pids 3465169 and 3465672 observed alive, then killed by the test). That is §11 row 2's declared residual, owned by 17-1's `spawn_t3`.

**One unreproduced anomaly, instrumented rather than ignored.** In 1 of 4 hang-corpus runs the pipeline emitted ZERO `task.stalled` rows for the full 60 s window (Workers alive, journaling fine, no panic, no poisoning); it then re-ran clean twice with byte-identical latencies. Not reproducible. The corpus now prints a "supervision pipeline at window close" line (SCB count / Running / ledger occupancy / stamp-age min-median-max) on EVERY run and quotes it in the floor's failure message, so a recurrence in CI names its own failing stage. Recorded here rather than closed.

### Completion Notes List

- Ultimate context engine analysis completed — comprehensive developer guide created (2026-09-15, four scouts @`af96c907` + ten fresh validation rounds, converged).
- **Subprocess supervision is real.** A Worker now has an SCB at a real pid, an exit observation that does not depend on pipe EOF, a progress stamp, and a task record the kernel can orphan and disposition. FR12's two floors and FR50 are MEASURED for the first time, on all three Worker-spawning roots.
- **NFR-Rel-11's planned half reaches every root.** `unload_all_loaded` is the one unload function; `finish_shell_session` delegates to it and keeps its report-never-return contract. butler's `belief_variance` halt now receipts under ITS OWN `halt_id` instead of a synthetic `term-…` one, because the pid binding is fixed at origin in the one adapter butler and Mira share.
- **The guard, not a convention, is what closes the leak.** `WorkerBinding::drop` performs `abandon`, so "every return between bind and finish is signalled, dispositioned and unloaded" holds by construction — including for a `?` a later edit adds. `LiveWorker`'s field order is load-bearing and has its own proven-red.
- Zero kernel bytes: `check-kernel-baseline: PASSED (maos-kernel-core/src = 24474 lines, 98 files, pinned 24474)`. `git diff Cargo.lock` empty — one feature added to an existing `rustix` dependency, no new crate.
- Measured ceiling raises booked in the same commit, citing the bare token `16-3`: `maos-bin` 20109 → **20963** (EXACT MEASURED +854, under the story's own ×1.3 upper of +1274), `maos-audit` 7189 → **7331** (+142). `kloc-check` `passed: true`, `over_budget: []`.
- Full sweep green: `cargo test --workspace --no-fail-fast` **4426 passed, 0 failed, 121 ignored**; both corpora `--release -- --ignored` report exactly `1 passed`; `audit_query_fr4_smoke.sh` PASS; `v01_evaluator_path.sh` PASS; `check-env-contract`/`check-workspace-count`/`check-exit-commands`/`check-epic-close-coherence`/`cargo deny check bans`/`cargo fmt --all -- --check`/no-default-features all clean.

### File List

**New**
- `crates/maos-bin/src/supervision.rs`
- `crates/maos-bin/tests/worker_supervision_16_3.rs`
- `crates/maos-bin/tests/worker_crash_corpus_16_3.rs`
- `crates/maos-bin/tests/worker_hang_corpus_16_3.rs`
- `crates/maos-bin/tests/root_shutdown_unload_16_3.rs`
- `crates/maos-audit/tests/fr4_worker_rows_16_3.rs`
- `crates/maos-journey-test/tests/run_bounded_full_pipe.rs`
- `spirits/worker/tests/fixture_modes_16_3.rs`

**Modified**
- `crates/maos-bin/Cargo.toml` (rustix `["fs"]` → `["fs", "process"]`; `Cargo.lock` unchanged)
- `crates/maos-bin/src/lib.rs`
- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/src/worker_spawn.rs`
- `crates/maos-bin/src/delegation.rs`
- `crates/maos-bin/src/shell_host.rs`
- `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs` (`SCANNED_SOURCE_FILES` 21 → 22)
- `crates/maos-bin/tests/two_host_delegation_2b.rs` (real supervisor in the host-B harness)
- `crates/maos-audit/src/fr4_classifier.rs`
- `crates/maos-audit/tests/fr4_classifier_16_2.rs`
- `crates/maos-journey-test/src/lib.rs` (`run_bounded` concurrent drain)
- `spirits/worker/src/bin/worker-cli-fixture.rs`
- `xtask/kloc.toml`
- `crates/maos-kernel-core/src/scheduler/control_block.rs`
- `crates/maos-kernel-core/src/supervision/crash_detector.rs`
- `xtask/kernel-core-baseline.toml`
- `.github/workflows/discipline.yml`
- `_bmad-output/implementation-artifacts/deferred-work.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/planning-artifacts/epics/epic-16-one-daemon-one-door-j0-w1.md`, `epic-17-workers-and-third-party-form-w2.md`, `epic-19-founder-loop-w4.md` (§12, applied at story creation)

**Deleted**
- `crates/maos-bin/tests/t0_probe_16_3.rs` (the T0 signature probe; its contract is now exercised by every crash vector and both corpora)

**Kernel review amendment:** two kernel source files changed under the operator-authorized optional-token repair; `check-kernel-baseline` passes at 24477 lines / 98 files with the updated per-file hashes.

### Change Log
- 2026-09-16 — **CODE REVIEW PATCHES COMPLETE.** All 20 findings resolved; optional task tokens preserved without a sentinel; runtime ownership, bounded waits/capture, teardown error propagation, cursor ordering, corpus pairing, and acceptance vectors hardened. Focused suites, both release corpora, kernel-core (572 tests), maos-audit (164 tests), KLOC, and kernel baseline pass. Windows remains the external `windows-check` acceptance gate already named by T8.

- 2026-09-15 — **DEV COMPLETE.** All 8 task groups and 6 ACs implemented and verified; 11 proven-reds run, recorded and reverted; 4 story premises disproved by measurement and recorded; Windows build UNPROVEN locally (`windows-check` owes the proof). Status → `review`.
- 2026-09-15 — story created; epic Round 7 applied (§12); sprint row → `ready-for-dev`.
- 2026-09-15 — validation round 1 (fresh context; compiled and ran the observer/runtime signatures): 7 ship-blockers and 19 must-fix applied.
- 2026-09-15 — validation round 10 (fresh context; verified at source): **0 ship-blockers — converged.** It confirmed the signals-sent counter's proven-red is deterministic under every drop/observer ordering, the injected-`?` position matches the construction order, and D-16-3-D's `abandon` matches every Q (5) cell. 2 must-fix applied: the injected-`?` vector names `hang` mode (with why not the grandchild or self-exit modes); `watch` returns `ObserveOutcome` so the supervisor, not the strategy, journals a per-Worker observer failure, D-16-3-B's "once per root" is scoped to the root probe, and a thread-spawn failure keeps the stored pidfd.
- 2026-09-15 — validation round 9 (fresh context; compiled and ran a `LiveWorker` probe, 200× per mutation): 1 ship-blocker and 3 must-fix — round 8's proven-red could not fire: with the guard's signal removed, the bridge's `Drop` still kills and reaps the child, so every listed observable stayed green ⇒ `SignalOutcome::{Signalled, AlreadyReaped, NotSignalled}`, a signals-sent counter seam asserted == 1, and the injected `?` pinned after `watch` and before the pump; per-Worker `pidfd_open` failure after a good root probe ruled (runs handle-less, journaled per binding); D-16-3-D's `abandon` definition signals from `Running`; `ObservedChild: Clone`. Round 9 confirmed the `LiveWorker` partial move compiles, drop order holds, and no deadlock in `abandon`.
- 2026-09-15 — validation round 8 (fresh context; two passes incl. a whole-story build/CI sweep; verified at source before applying): 1 ship-blocker and 3 must-fix — **`SpawnedBridge`'s own `Drop` kills and reaps the child** (`runtime.rs:791-800`), so with guard and bridge as separate locals the injected-`?` vector could not prove the guard's signal and flaked on a spurious crash path (7/200, 9/200 probed) ⇒ `LiveWorker { binding, bridge }` with the guard first + a proven-red; the pidfd lives in a strategy-module `ObservedChild` type (no `OwnedFd` outside the module); `SignalThenDispositionAndUnload` added; `Pid::from_raw(..).ok_or(..)`; nfr-rel-11's builds share the test's `--release` profile. Pass 2 found every CI command, feature (`rt-multi-thread`, `signal`, rustix `process`/`event`), the no-default-features build and every changed call site consistent.
- 2026-09-15 — validation round 7 (fresh context; verified at source before applying): 1 ship-blocker and 3 must-fix — **pidfd is Linux-only in rustix and macOS is a shipped release target**, so round 6's `cfg(unix)` gate would have broken the macOS release: the `Signal` executor became a Strategy method (`ExitObservation::signal_kill`) and every pidfd/`rustix::process` reference is `cfg(target_os = "linux")` inside the strategy module, with only the listener's signal streams `cfg(unix)`; the Windows listener twin uses the synchronous `tokio::signal::windows::ctrl_c()`; the fixture self-kills via the `kill` command and spawns its grandchild via `current_exe()` with no new dependency; `abandon(Running)` signals before disposition-and-unload, `on_pidfd_stored` from terminal phases is a typed refusal, and the injected `?` sits after spawn.
- 2026-09-15 — validation round 6 (fresh context; verified at source before applying): 1 ship-blocker and 3 must-fix — **maos-bin builds on Windows** (`windows-check`, a ship-gate need), so pidfd fields, `rustix::process` calls and `tokio::signal::unix` streams get `#[cfg(unix)]`/`#[cfg(not(unix))]` twins (Trap 24, T8 Windows step); the three AC3 hooks became `#[doc(hidden)] pub` seams with the reap-race gate placed before the `waitid` CALL, an `ECHILD` observation counter and a recorded proven-red; the action table completed (`on_exit` cells, `ExitedClean`, `dispositioned` makes every `Disposition*` idempotent, `finish` disarms the guard); the no-pidfd grace path prints its own line and §11 row 6 states the orphan. Round 6 modelled AC4 (5b) and found no flake on a correct build.
- 2026-09-15 — validation round 5 (fresh context; compiled and ran a D-16-3-Q sketch — `&dyn` port, guard over `Arc<BindingCore>`, `Drop` ⇒ `abandon` via `Handle::block_on` during panic unwinding inside `block_in_place` and `spawn_blocking`): 1 ship-blocker and 5 must-fix applied, each re-verified at the source first — on Linux a crash whose child the bridge reaped before the observer looked (`ECHILD`) was handled by nothing (`finish`'s crash row was scoped to non-Linux and the Strategy put it in the fallback only) ⇒ `BindingPhase::finish(Running, crash)` shared by both strategies + a reap-race vector; the complete `BindingAction` table (pidfd-stored `Signal`, finish/abandon from `CrashHandled` without a double disposition, join timeouts); the observer's subscriber is `Arc<BindingCore>` and the guard a call-stack handle; `watch(child_pid, core)` replaces an undefined `ChildHandle`; the active-path count moves to `bind`/guard (a 10 s liveness probe could trip a false grace exit); AC4 (5b) no longer timing-dependent (`biased` select + counted retries); Mira's pid-binding setters named; the no-pidfd stop cost stated.
- 2026-09-15 — operator ratification (§15): all six questions ruled as recommended; Q2 adds the design-pattern directive, recorded as D-16-3-Q (Ports & Adapters, Observer, Strategy, State, Command, RAII guard) with its vectors, review obligation (u) and a +20…+40 raw re-book (maos-bin ×1.3 upper +1222 → +1274).
- 2026-09-15 — validation round 4 (fresh context, ship-blockers only): **retracted round 3's first ship-blocker** — `kind_to_string` already names kinds 12/13/15/16/19 (`crates/maos-audit/src/lib.rs:708-713`, added by 16-2), so the 'fix', the §5 bullet, the trap and the '16-2 null control' charge were deleted (the read-back vector stays as a regression guard; lesson: a validator's correction is itself a premise — check it at HEAD before applying it); the cohort Worker drain after `stop_workers()` is UNBOUNDED with the site-2 listener's grace exit as sole arbiter (a bounded drain that proceeds leaves an uncancellable `spawn_blocking` Worker holding `audit_tx`, and tokio's blocking-pool drop joins it forever); the listener's SIGTERM stream is created synchronously and announced by `maos: root supervision armed`, so AC4 (5b) can red; no `abandon`/`unload` under the binding lock (`on_unload` takes it); kind-30 `identity.asserted` joins `NON_CALL_KINDS`; cohort intake stop named and its result kept; path (b)'s frame id read before `delegate` moves the frame; a single-class `--once` signalled mid-pass completes and exits 0, stated.
- 2026-09-15 — validation round 3 (fresh context; one compile probe): 5 ship-blockers and 16 must-fix applied (its first — a `kind_to_string` gap — was retracted by round 4); the inclusive-cursor falsifier moved to a vector where nothing else prints; binary tests poll the pidfd (a pidfd `waitid` sees only the caller's own children); the cohort `select!` observes `root_shutdown` (a startup SIGTERM was swallowed); `on_crash_action` captured at bind; `stopping` latch; topology `Err`-arm shutdown check; listener wording + check-then-wait with `root_cancel`; explicit cohort order with a bounded Worker drain; `WorkerTask` as a source enum; record token patched after the mint; interrupted `--once` exits 1; `maos run` refuses `MAOS_ONE_SHOT`; acp-server readiness handshake; the crash-vs-unload race stated; obligations (c)/(d)/(f)/(s) re-pointed; single-query poller; SIGINT process-group trap.
- 2026-09-15 — validation round 2 (fresh context; three parallel sub-reviews): 7 ship-blockers and ~30 must-fix applied, several introduced by round 1 — `Option<RootSupervision>` declared before the run block so the serving tail keeps it (round 1's run-block locals would have detached and redded 16-1's `< 3 s` test); `finish` on `PlannedStop` dispositions and unloads (a stopped topology Worker exits through a partial-load return); the revoke closure stays unconditional (D-16-1-V); AC4 (8) replaced by a SIGTERM oracle on `acp-server` (the one-shot drains already time out, measured); the butler successor binding set inside the factory; `fr4_classifier_16_2.rs` is extended, not unchanged; the crash corpus kills each grandchild at +5 s (round 1's "at the end" deadlocked); binding state under one mutex with stop-before-spawn and handle-storage windows closed; kept-record disposition for `NotLoaded`/`PlannedStop`; `--once` passes check the token between passes; the listener is joinable with an active-path wait; exclusive stamper cursor + `line-then-hang`; `bindings()` seam; spawn-copy grandchild; section parses as gates; seven kind-1 dispositions ruled now with `kind`/`token` on entries and NDJSON-based AC5; cohort writer await; concrete `Arc<WorkerSupervisor>`; kloc re-summed (+1274 / +130); ungated module; `1 passed` job check; coverage-matrix routed to 20-3b; §11 rows 12–15 added.
