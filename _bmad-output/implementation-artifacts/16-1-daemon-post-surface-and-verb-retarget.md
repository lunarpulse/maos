---
baseline_commit: "**`f72b557b`** — RE-BASELINED 2026-09-13 before dev (authored at `a6ed282c`). Two commits landed after authoring: `07714750` (16-0 dev-record follow-up) and `f72b557b` (E15-A3 quick wins). `git diff --stat a6ed282c f72b557b`: `crates/maos-cli/src/subcommands.rs` +16 at `same_file` (`:278`), `xtask/kloc.toml` +11 at `:309`, `.github/workflows/discipline.yml` +21 at `:3501`/`:3511`, `deferred-work.md` line-neutral (`:916-918` only — `:550`/`:869` unchanged), `release_dry_run.rs`, a 16-0 story file, an abi doc. **`main.rs`, `maos-control`, `maos-shell`, `maos-domain`, `maos-audit` and `maos-kernel-core` are byte-identical to `a6ed282c`.** Every cite into a moved file (story + epic 16) re-derived and content-verified by `git show`; three epic cites already wrong at `a6ed282c` corrected. Gate sweep at `f72b557b` (binaries rebuilt): `check-kernel-baseline` **PASSED (24474 lines, 98 files)** · `kloc-check` **PASSED (aggregate 158315; maos-cli 5290, was 5279)** · `check-env-contract` PASS (89) · `check-workspace-count` PASS (55) · `check-service-boundary` PASS · `check-epic-close-coherence` PASS (21) · `check-exit-commands` PASS (6 blocks, 25 resolved, 5 owed) · `check-dev-record-completeness` **PASSED (154)** — the pre-existing red recorded at authoring (16-0 empty `Agent Model Used`, `deferred-work.md:916-918` ownerless) was closed by `07714750`. **Authoring note, still true:** five of the epic's 16-1 AC2 `main.rs` anchors were WRONG WHEN ROUND 4 WROTE THEM — `main.rs` is byte-identical at `843d5365` and `a6ed282c`, so an authoring error, not drift: bind `:2792`→**`:2827`** (+35), `scheduler` `:2644`→`:2681`, `crash_detector` `:2717`→`:2754`, `halt_registry` `:1985`→`:2022`, `orchestrator_registry` `:1994`→`:2031` (+37 each); AC6's doorbell cite (`cohort_daemon_smoke_13_5c.rs:863`, `17 ==`, `:937-941`) was never refreshed (actual **`:869`, 19, assert `:954-958`**). Original body: ≈236 citations re-measured by a fresh-context validator at `a6ed282c` (219 correct, 17 corrected). ⚠ **T0 re-measures. A grant is a global, not a reservation.**"
depends_on: "**`16-0-kernel-pin-content-hash` (`done`)** — only in that it opened the epic; 16-1 writes no kernel-core byte, so the file-set/content-hash pin must stay GREEN through this commit (a tripwire for this story, not an input). **Epic 15**: ADR-062 (15-5, the governing decision), `verbs.rs` + `MAOS_ONE_SHOT_MODES` + `check-exit-commands` (15-3), `MAOS_INFERENCE_MODE` (15-6, used by the AC1 root), the eased RECOVERY-LANE CEILING RULE."
blocks: "**`16-2-shell-halt-registry-and-j0-scene`** (its `maosctl halt resolve` travels over this door — ADR-062 Consumers), **`16-4-maos-uninstall-and-keyring`** (its AC3 `maosctl uninstall <spirit>` 'keeps its semantics over the 16-1 surface'), **`19-2-fr20-enqueue-door-and-safe-point`** (enqueue via this surface), **`16-3-subprocess-crash-to-handle-crash`** (ordered after 16-1 by the epic's Dependencies line)."
spec_alignment: "⚠ **THE EPIC IS AMENDED IN THE SAME COMMIT (rule 9, Round 5), AND ADR-062 IS AMENDED WITH THE CODE.** Measurement disproved premises inside 5 of the epic's 6 ACs; the evidence is §1–§9 of this file, the rulings are the Decisions table, the applied epic edits are §12. Headline: `maos init` exits 1 on every clean home; the AC1 subject cannot be named by any verb; the door is bound too early, by every one-shot child, onto a fallback port; 13 of 16 'mutating' verbs are fake or harmful on a running daemon; 3 of 5 'TL-only readers' read memory; 4 verbs are correct offline; four home resolvers; the server cannot carry POST and a gate reds on the first 405; 7 of 9 'fake-maos' test files run the real binary. ⚠ **ROUND-TABLE 2026-09-13 (§15, operator-ratified: *approved*)** closed all three §14 questions and six more forks the table found — incl. a MEASURED exploit path (bare Workers inherit the daemon env and run as the operator uid, so they can read `control.json`), a lock keyed to the wrong store, and an FR51 over-claim with no mechanism behind it."
split_from: "Not a split. Authored from `epics/epic-16-one-daemon-one-door-j0-w1.md` 16-1 section (HEAD `:118-127`; `:120-129` after this commit's 2-line Round-5 header). **SIZING RATIFIED WHOLE** at the round-table (§15 F-A); §13's seam is documented, not used. FR9's `load` half is a NEW sibling row, `16-6-maosctl-load-over-the-door` (§15 F-B). Measured duration **6–9 d**."
kernel_grant: "**NONE. ZERO kernel-Δ @24474, proven on every push by the 16-0 content-hash pin (`changed == 0`).** Every kernel API driven here is already `pub` (Dev notes → Reuse). ⚠ One kernel defect this story FOUND is routed OUT: the Lifecycle Journal cross-process overwrite (`crates/maos-kernel-core/src/journal/mod.rs:98-102`) → **16-5** (§11 row 1). ⚠ `crates/maos-kernel-core/src/orchestrator/mod.rs:17` has a doc comment naming `MAOS_ONE_SHOT=orchestrator-queue`, a mode this story deletes — it must **NOT** be edited here: a doc-comment byte reds the content-hash pin."
kloc_grant: "**Measured raises only where a ceiling is actually crossed, taken AFTER the code exists, citing the OPEN key as the bare token `16-1`** (the full key does not tokenize, `xtask/tests/d11_xtask_ceiling_ratchet.rs:164-179`). Baseline headroom: `maos-control` 803/1791 **+988** (`xtask/kloc.toml:454`) · `maos-bin` 17714/20109 **+2395** (`:475`) · `maos-cli` 5290/6856 **+1566** (`:459`; was 5279 at `a6ed282c` — `f72b557b` added the non-unix `same_file` arm) · `maos-shell` 307/500 **+193** (`:642`, ⚠ BINDING — 16-2 needs +30–60 after this) · `maos-domain` 8695/9192 **+497** (`:395`) · `maos-audit` 6845/6951 **+106** (`:386`) · `maos-capability` 1046/2000 **+954** (`:248`; D-16-1-V touches it — NOT the kernel pin, whose file set is `maos-kernel-core/src`) · `maos-kernel-core` ZERO (untouched) · `xtask` 44101/44101 ZERO (this story writes `xtask/tests/` only, kloc-free). §10 forecasts every crate inside headroom → **expected ceiling edits: zero**. ⚠ `maos-control`'s inline `#[cfg(test)]` bodies ARE charged (452 of 803 tokei lines) — new server tests go in `crates/maos-control/tests/`. ⚠ The aggregate ALARM (158608, a non-blocking notice) will fire; the hardfail (170884) will not."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs:45-48`) is the machine-checked set; `equiv` is prose, NOT a match token. Keep the literal `allowlist {` (`check_dev_model_used_populated.rs:302`)."
review: "◆ §A6 full-layer net, NON-DEGRADABLE (Blind + Edge Case + Acceptance + Test-Infra + non-author runtime); E15-A6 binding by name — *does this test read the tree, or only what it set itself?* Reviewer obligations (a)–(q) are in the body under **Review obligations**."
---

# 16-1 — The daemon gets a door, and the verbs behind it become real

Status: **done**

> **The capability:** *With a MAOS root running, `maosctl pause butler` pauses **that** Butler — the
> running one — and `maosctl spirit inspect butler` says `Paused`, read back from the process that
> owns it. Every state-changing operator verb either reaches the live daemon through one authenticated door, or
> runs offline only when the OS proves no daemon holds that home's stores (the FR58 evaluator workload
> `maosctl run` is not an operator verb, D-16-1-M).*

**Closes:** drift **S1** (*control plane never reaches the daemon*, full drift review 2026-09-04) ·
the door half of epic-14 action **A4** (`sprint-status.yaml`, `A4: Daemon RPC + applied sandbox`;
17-1/17-2 own the sandbox half, so the row stays `open`) · FR16 and FR51 **(b)**, plus the door reachability of FR51 (a)(d) (§15 F-E; (c) → 19-2 AC3, §17) ·
FR9 **start / pause / resume / unload** (`load` → `16-6`, §15 F-B) · FR13's operator import reaching running Spirits ·
`deferred-work.md:869` (operator HTTP serial accept thread — rule 10(b), this story rewrites that
loop) · the door half of `deferred-work.md:550` (one-shot `legal-hold-release` carried no operator
authorization boundary; over the door it carries the bearer).

**Does NOT close, and says so:**
- **FR9's `load` half.** No `maosctl load` verb exists anywhere in the tree; the only runtime load is
  `maos run <manifest>`. → **`16-6-maosctl-load-over-the-door`**, created by the round-table (§15 F-B),
  after 16-1, reusing `maos run`'s admission path behind this door.
- **FR51(a)'s bounded interrupt and (d)'s bounded fail-safe.** `SpiritSchedulerAdapter::pause`
  (`scheduler_loop.rs:401-425`) flips SCB state and fires `on_pause` (`hook_dispatch.rs:294-297`); an
  in-flight hook keeps running, and the watchdogs only skip the NEXT dispatch. No mechanism interrupts
  anything → **not claimed**; named in the epic's *Closes* line and raised to the **Epic-16
  retrospective** (§15 F-E).
- **FR51(c)'s recall of pending Orchestrator-buffered actions.** `OrchestratorBuffer::recall_all_pending`
  (`buffer.rs:72`) has no operator route; its only production caller re-enqueues at resume
  (`scheduler_loop.rs:452-460`). A recall means something only against a safe-point dequeue → **19-2 AC3** (§17).
- **Cold-swap upgrade over the door.** `UpgradePolicy::ColdSwap` (`crates/maos-kernel-core/src/lifecycle/upgrade.rs:162`)
  starts the successor under a new pid without `admit_spirit` (spike, §17) — a kernel byte; maos-bin's `pid_by_spirit_id` (`main.rs:2428`) is not refreshed either. The
  door refuses it, typed (D-16-1-W); raised to the **Epic-16 retrospective** row.
- **Worker reach to the door.** Until **17-1** lands, a Worker inherits the daemon's environment and runs
  as the operator uid, so it can read `control.json` → clause written into 17-1 AC1/AC3 (§15 F-D).
- **The Lifecycle Journal cross-process overwrite** → **16-5** (kernel-Δ owner; §11 row 1).
- **The J0 halt → resolve scene.** 16-1 makes `halt resolve` reach the daemon's `HaltRegistry` and
  deletes the fabricated halt; **16-2** makes the shell produce a real halt for it to resolve.
- **The home split-brain beyond `control.json`** → **16-4** (§11 row 2).

---

## What this story actually is

The epic describes a routing change: 18 spawn sites become HTTP calls. Measurement says routing is the
small half. What is true at `a6ed282c` (re-verified at `f72b557b`), in the order a dev will hit it:

1. **J0's first command fails on a clean machine.** `maos init` exits 1 with `NotFound`. (§1)
2. **The verbs are not mis-routed — most are fake.** A door in front of a fake handler is an
   authenticated fake: `revoke-token` has never succeeded, `spirit-upgrade` panics under test,
   `pause` never pauses, `halt-resolve` invents the halt it resolves. (§4)
3. **The door is bound before the objects it would serve exist, and by every one-shot child.** (§3)
4. **The server cannot carry POST safely**, and a gate reds on the first `405`. (§8)
5. **The three processes that must agree on `control.json` have no shared home rule.** (§7)

So this is **the story that makes the operator surface true** — door, discovery file, and the
handlers behind it — before 16-2 (halt), 16-4 (uninstall) and 19-2 (enqueue) build on it.

---

## 1. 🔴 `maos init` EXITS 1 ON EVERY CLEAN HOME, AND EVERY INIT TEST MASKS IT

`maos_shell::run_init` (`crates/maos-shell/src/lib.rs:46`) opens `home/config.toml` with
`create_new(true)` (`:53`) **before** `create_dir_all(&home)` (`:81`). On a home that does not exist,
`open` fails `ENOENT` and `:77` returns it. Measured against the built binary:

```
$ env -i PATH=$PATH HOME=$S/home XDG_DATA_HOME=$S/xdg ./target/debug/maos init      # ~/.maos absent
Error: Os { code: 2, kind: NotFound, message: "No such file or directory" }
EXIT=1
$ env -i PATH=$PATH HOME=$S/home MAOS_HOME=$S/newhome ./target/debug/maos init       # MAOS_HOME absent
Error: Os { code: 2, kind: NotFound, message: "No such file or directory" }
EXIT=1
```

All four tests that assert init **create the home first**:

| test | fixture |
|---|---|
| `crates/maos-journey-test/tests/journey_j0.rs:28` `j0_init_creates_config` | `MAOS_HOME = tempfile::TempDir` (exists) |
| `crates/maos-shell/tests/init_test.rs:46` `init_creates_config_and_dirs` | `isolated_home` `:31-43` → `create_dir_all` |
| `crates/maos-shell/tests/init_test.rs:82` `init_is_idempotent` | same fixture |
| `crates/maos-bin/tests/shell_8_14a.rs:28` `init_creates_dot_maos_and_is_idempotent` | `isolated_home` `:13-25` → `create_dir_all` |

The last test's **name** says it creates `.maos`; its fixture does. This is exit line 1 of the J0
block and inside the function AC2 edits → **fixed at origin** (rule 10(a)) with a test whose fixture
does **not** create the home, proven red first.

---

## 2. 🔴 THE EPIC'S AC1 SUBJECT CANNOT BE NAMED BY ANY VERB AT HEAD

AC1 is *"`maosctl pause butler` … makes `maosctl spirit inspect butler` report Paused"*. Against a
live `maos run spirits/butler/manifest.toml` root:

```
maosctl pause butler                        → exit 2  "unknown spirit 'butler' — only 'hello-spirit' is available at v0.1-β"
maosctl posture butler --shift cautious     → exit 2  (same)
maosctl halt list --spirit butler           → exit 2  (same)
maosctl orchestrator status --spirit butler → exit 2  (same)
maosctl spirit inspect butler               → exit 0  "requires --sandbox at v0.3-β"
```

Three layers refuse the name. The first two fire **inside maosctl, before any spawn**; the third
would refuse it again **inside the child**:

| layer | site | mechanism |
|---|---|---|
| maosctl preflight | `crates/maos-cli/src/subcommands.rs:2191` `resolve_spirit_pid` → `maos_audit::resolve_spirit_name` (`crates/maos-audit/src/lib.rs:1615`) | non-`hello-spirit` names need a **kind-19** `SpiritAdmitted` TL row (`:1685`); `maos run` writes **kind-7** `lifecycle.load` rows only (probe: `7\|1\|lifecycle.load`) |
| lifecycle verbs | `subcommands.rs:1472` `lifecycle_verb` | `if name != "hello-spirit"` |
| one-shot arms (in the child) | `crates/maos-bin/src/main.rs:5701-5709` (halt-list filter), `:5755`, `:5864`, `:5926`, `:5953` | `hello-spirit` only |

And the resolver's "latest boot" is **the largest random number**: `max(boot_nonce)`
(`maos-audit/src/lib.rs:1660,1720`) while nonces are random (`main.rs:1902-1916`). → D-16-1-K.

---

## 3. 🔴 THE DOOR IS BOUND TOO EARLY, BY THE WRONG ROOTS, ON A FALLBACK PORT

**Too early.** The production bind is `main.rs:2827-2846`. Objects the verbs need come **after** it:

| object | constructed | needed by |
|---|---|---|
| `hot_swap_coordinator` | `main.rs:2866` | hot-swap-precheck |
| `revocation_applier` | `main.rs:2880` | revocations-import / -list |
| `upgrade_orchestrator` | `main.rs:3087` | spirit-upgrade |
| monotonic clock base | `main.rs:3276` (shell), `:3496` (run) | every handler stamping a row — `monotonic_now_ns` has a `debug_assert!` (`crates/maos-capability/src/cap_tokens/mod.rs:68-71`): **panics in debug builds, silently stamps 0 in release** |

The bind moves **after `:3087`, before `if shell_mode` (`:3272`)**, with the clock base initialised
first, and stays **below** `Arc::get_mut(&mut scheduler)` (`:2766`) — 14-2a measured a retained `Arc`
there PANICKING every process that configured the surface (`kloc.toml` maos-bin row). The other objects
the handlers need are all in scope there: TL (`:2038`), memory (`:2316`), `shared_journal` (`:2743`);
the audit writer spawns at `:3109`.

**Wrong roots.** The bind precedes the `MAOS_ONE_SHOT` dispatch (`:5159`), and maosctl spawns do not
`env_clear`, so **every one-shot child boots the door** when the token is in its environment.
Measured: with `MAOS_OPERATOR_BEARER_TOKEN` exported beside a live daemon, a one-shot exits 1
`operator HTTP bind failed: Address already in use`. ADR-062 names `maos run` and `maos shell`
(`:48`); `crates/maos-bin/tests/cert_rotation_trigger_14_2a.rs:243-246` **requires** a third root
(`MAOS_ONE_SHOT=cohort-a2a-daemon`, scraping the listening marker `main.rs:2840` at `:172`). No other
test relies on the env override making a non-door root bind. → D-16-1-B.

**Fallback port.** AC2 says *"never a fallback port"*; HEAD has one: token set, bind unset ⇒
**`127.0.0.1:8787`** (`main.rs:2709-2713`), documented at `env_contract.rs:481`. The real gate is the
**token** (`:2705-2728`); the epic's old `:2687` cite is `Arc::clone(&telemetry)`. `8787` has no other
consumer in `crates/`, `tests/`, `xtask/src` or `docs/` — retired.

---

## 4. 🔴 13 OF 16 "STATE-MUTATING" VERBS ARE FAKE OR HARMFUL ON A RUNNING DAEMON

Per-arm measurement (scout B, probes against built binaries). **In-memory** = lost at process exit:
SCBs, `PolicyTable`, token shards (`maos-capability/src/cap_tokens/mod.rs:272`), CRL rules
(`kernel-core/src/revocation/rules.rs:19-27`), `OrchestratorBufferRegistry`, `HaltRegistry`,
private-tier inline memory (`kernel-core/src/memory/private.rs:60,670-676`).

| verb | arm (`main.rs`) | effect on a RUNNING daemon today |
|---|---|---|
| `pause` / `resume` | `:5948-6025` (one arm) | **nothing.** Journal + approval-log only; never calls `scheduler.pause`; `hello-spirit` only (`:5953`) |
| `start` / `stop` / `unload` | `:5345-5408` (shared with `uninstall`) | **nothing, and lies**: probe — `stop butler` exit 0 wrote `Halt butler` while butler kept running |
| `posture-shift` | `:5473-5682` | **nothing.** Re-admits pid **0** into a fresh `PolicyTable` from CWD-relative `spirits/{id}/manifest.toml` (`:5496`), shifts pid 0 (`:5626-5628`); the daemon's butler is pid **1** |
| `halt-resolve` | `:5744-5851` | **fabricates** a pending halt (`:5765-5784`: tag `"test"`, value `0.5`, `policy_id: "test-policy"`) and journals its resolution |
| `orchestrator-queue` / `-status` | `:5855-5945` | fresh registry; status **always `0/32`**; queue ids restart at 1 per process |
| `revoke-token` | `:6029-6075` | **can never succeed** — fresh ring; probe exit 1 "not found"; `revoke_token_test.rs` pins only failures. **And on a live ring a repeated revoke returns `Ok` again and writes another `cap.revoke` row** (`crates/maos-capability/src/cap_tokens/shard.rs:44-52`; spike §17) |
| `revocations-import` / `-list` | `:6804-6859` | fresh scheduler (0 SCBs) ⇒ `matched_count` always 0; rules lost at exit; list always empty; **exit 101 in debug builds** too (no `init_monotonic_base`; spike §17) |
| `hot-swap-precheck` | `:6442-6505` | **writes phantom rows**: loads a placeholder Spirit into a fresh scheduler, appending `lifecycle.admit`/`lifecycle.load` for pid 1 under a new boot into the SHARED TL (probe) |
| `spirit-upgrade` | `:6724-6802` | no `init_monotonic_base` in the arm ⇒ **exit 101 in debug builds** (probe; via `upgrade.rs:82`), zero timestamps in release — and `NotLoaded` regardless. **Inside a daemon (spike §17):** no butler successor loader (`main.rs:3046-3084`), butler's manifest lacks the `[scheduling]`/`[lifecycle]` a target needs (`upgrade.rs:393-400`), downgrades report `completed`, cold-swap starts an unadmitted successor under a new pid |
| `forget` / `uninstall` | `:5411-5431`, `:5359-5391` | durable sqlite erase is real, **but** misses the daemon's in-memory private values and live tokens ⇒ **false `Erased`** while a daemon runs |
| `legal-hold-release` / `governance-admit` | `:5178-5196`, `:5434-5472` | durable TL writes; **correct** |
| `halt-list` / `legal-hold-list` | `:5685-5741`, `:5167-5177` | durable TL reads; **correct** |
| `hello-spirit` (`maosctl run`) | `:8090-8286` | a **workload** — one evaluator turn, FR58 JSON, exit 0 |

**Consequence:** every door handler's AC names the **kernel transition** it performs, and every door
test asserts the transition happened, not the row alone.

---

## 5. 🔴 "TL-ONLY READERS" ARE 2 OF 5

| reader | reads | in maosctl it would report |
|---|---|---|
| `halt-list` | TL `query_frames` (`main.rs:5712`) — durable | **correct** |
| `legal-hold-list` | TL `list_legal_holds` (`:5168`) — durable | **correct** |
| `revocations-list` | `revocation_applier.list_applied()` — in-memory | **always empty** |
| `orchestrator-status` | `orchestrator_registry.get(..)` — in-memory | **always 0/32** |
| `hot-swap-precheck` | a scheduler + `hot_swap_coordinator` | **fake**, and maosctl cannot link kernel-core |

`maos-cli` already links `maos-audit` and `maos-iac`; the two durable readers use a **read-only**
connection, never the write-capable adapter (a write-capable open can hold the sqlite lock past the
daemon's `busy_timeout=5000` and trip the TL's panic-on-write-error, `crates/maos-iac` `transparency_log.rs:866`).

---

## 6. 🔴 FOUR VERBS ARE CORRECT OFFLINE — "DAEMON DOWN ⇒ ERROR" WOULD BREAK THE ERASURE SPINE

For **`forget`, `uninstall`, `legal-hold-release`, `governance-admit`**: their effect is durable and
correct when no daemon holds in-memory state; GDPR erasure (FR45/FR65) cannot require booting a root;
`crates/maos-bin/tests/erasure_uninstall_13_5b.rs` pins the one-shot contract `0/3/4/5` ↔
`erased/held/not_found/failed`, and `crates/maos-cli/tests/uninstall_exit_codes_13_5b.rs:17-29` pins
that maosctl forwards it. But offline is **wrong while a daemon runs** (false `Erased`, TL-lock
hazard), and a PID file or connect probe cannot settle "is a daemon running" without a race. → D-16-1-D:
an OS lock settles it.

---

## 7. 🔴 `control.json` HAS NO AGREED HOME, AND ADR-062 KEEPS ONE WRITER

Four resolvers disagree (scout B, running `maos run` under three environments):

| resolver | rule |
|---|---|
| `maos_shell::maos_home` (`crates/maos-shell/src/lib.rs:304`, private) — what `maos init` writes by | `MAOS_HOME`, else `$HOME/.maos` |
| `maos_audit` TL / journal (`crates/maos-audit/src/lib.rs:873-903`, `:1417-1443`) | `MAOS_HOME/…`, else `MAOS_AUDIT_DB`/`MAOS_JOURNAL_PATH`, else `$XDG_DATA_HOME/maos`, else `~/.local/share/maos` |
| memory root, spirit archives (`maos-audit/src/lib.rs:1456-1470`, `:1555-1575`) | **ignore `MAOS_HOME`** |
| CRL dir (`main.rs:2898`) | hardcoded `/tmp/maos/crl` |

maosctl has **no `control.json` resolver** (its TL path goes through `maos_audit`'s resolver,
`subcommands.rs:4023`, which honours `MAOS_HOME`). `maos-cli` cannot depend on `maos-shell` (the edge
runs `maos-shell` → `maos-cli`). ADR-062 names **one** writer of `control.json`, `maos init`
(`:52-55`, *"this preserves one trust path"*); the epic's *"the daemon records its `pid` and
`boot_nonce` in `control.json` at bind"* adds a second. → D-16-1-C.

---

## 8. 🔴 THE SERVER CANNOT CARRY POST SAFELY, AND A GATE REDS ON THE FIRST `405`

`crates/maos-control/src/lib.rs` (803 tokei: **351 production, 452 inline tests**):

| property | HEAD | why it matters for POST |
|---|---|---|
| concurrency | one `std::thread` (`:224`), non-blocking listener polled every 10 ms (`:220,237-238`), each connection served inline (`:228`) | one slow POST (`fire_on_pause` runs Spirit hook code, `scheduler_loop.rs:421`; an upgrade; a CRL admission guard) stalls every verb. `deferred-work.md:869` filed this against the operator-surface owner — this story |
| timeout | 2 s **per `read()`** (`:273`) | a byte-dripping client holds the only thread **before auth** |
| body | read to `\r\n\r\n` in 16 KiB (`:274-282`); no body parser, no `Content-Length`, no 413 | ADR-062 `:57-58` requires bounded body parsing |
| header scan | `lines` iterator (`:284-290`) does not stop at the blank line | a body line `Authorization: Bearer <token>` counts as the header |
| failure | non-`WouldBlock` accept error **breaks** the loop (`:240`); a handler panic kills the thread | process lives on with **no listener**, silently |
| status phrases | `respond` maps 200/401/404/503 only; else "Internal Server Error" (`:459-465`) | no 400/405/409/411/413/500 |
| spirit id | parsed from `/v1/spirits/` unvalidated (`:427-432`); only the client validates (`subcommands.rs:1956-1962`) | a new `GET /v1/spirits/{id}` would also match `x/sandbox` unless ordered |
| auth | `ring::constant_time::verify_slices_are_equal` (`:16,289`) before routing (`:291-298`) | correct — keep |

**The gate trap.** `xtask/tests/decision_adrs_and_provisioning.rs:310-339` (the `"062"` arm)
`require_contains` the four literals `"GET /v1/a2a/rotation-windows"`, `"GET /v1/cohort/peer-versions"`,
`"GET /v1/cohort/self-identity"`, `"GET /v1/spirits/"`, and `require_absent("Method Not Allowed")`
(`:330-338`). The first 405 phrase reds it; a refactor splitting method from path deletes the literals.
The test is kloc-free and is re-anchored in the same commit.

**The guard ambiguity.** ADR-062 `:66-69` keeps "two certificate-identity guards" and narrows "the
generic read-only assertion". The three HEAD tests are `POST → 404` at `lib.rs:607` (rotation windows),
`:786` (peer versions), `:927` (**self-identity**) — but self-identity *is* "this host's signed and
serving certificate identity" (`:143`), which ADR-062 `:63` preserves. → D-16-1-G.

---

## 9. 🔴 TESTS: 2 OF THE "NINE FAKE-`maos`" FILES ARE FAKE; SEVEN RUN THE REAL BINARY

`grep -l MAOS_BIN_PATH crates/maos-cli/tests/*.rs` = 9 files; only `legal_hold_cli_13_5b.rs:12-21` and
`uninstall_exit_codes_13_5b.rs:12-15` write a fake script. The other seven (`halt_resolve_test` 8 tests,
`orchestrator_queue_test` 7, `pause_resume_test` 6, `revocations_import_test` 5, `revoke_token_test` 4,
`spirit_upgrade_test` 4, `accessibility_test` 6) point `MAOS_BIN_PATH` at the **real** sibling binary and
pin **offline one-shot success** (e.g. `pause_resume_test.rs:71-81`, `halt_resolve_test.rs:125-216`;
`posture_shift_test.rs:49,69,86` spawn with `MAOS_ONE_SHOT=posture-shift`). Two are vacuous:
`spirit_upgrade_test.rs:101-110` passes on any stderr; `revocations_import_test.rs:141-167` kills the
child after 500 ms.

Other hits for modes this story deletes: `erasure_uninstall_13_5b.rs:767` (`legal-hold-list`),
`legal_hold_cli_13_5b.rs:17` (fake answers `legal-hold-list`), `epics/epic-19-founder-loop-w4.md:66`
(`MAOS_ONE_SHOT=orchestrator-queue` prose), and the kernel doc comment at
`crates/maos-kernel-core/src/orchestrator/mod.rs:17` (**do not edit** — content-hash pin). Nothing in
`.github/`, `tests/integration/` or `xtask/` names a deleted mode.

Hard constraints on the rewrite:
- **No `maos-control` dev-dependency in maos-cli**: `dep_kernel_core_free_test.rs:10-11` runs plain
  `cargo tree -p maos-cli` (dev edges included) and `maos-control` links kernel-core
  (`crates/maos-control/Cargo.toml:12`). The fixture door is hand-rolled.
- **Contract tests** (what maosctl sends, how it maps responses) live in maos-cli; **behaviour tests**
  (the verb really did the thing) live in maos-bin against real roots or the real port.
- **Shared-HOME roots.** `f4_pairing.rs:637-638` and `two_host_delegation_2b.rs:290-300` run two roots
  on the developer's real `HOME` by design (they isolate `MAOS_AUDIT_DB`, not `MAOS_HOME`, because
  `MAOS_HOME` redirects the TL), and seven more root-spawning tests set neither `HOME` nor `MAOS_HOME`
  (`maos_run_boot_loud_8_11`, `researcher_8_14c`, `inference_provider_polarity_8_11`, `smoke_run_8_11`,
  `smoke_mira_nash_tcp_8_13`, `jb3_self_tuning_halt`, `cohort_daemon_smoke_13_5c`). After a developer runs
  `maos init`, `$HOME/.maos/control.json` exists and these parallel roots would all bind one endpoint →
  `EndpointInUse` → **machine-dependent reds, green in CI**. → D-16-1-Q.

CI surface: all run by `workspace-test-suite` (`.github/workflows/discipline.yml:3534`), plus
`maosctl-smoke` (`:665`) and `v01-evaluator-path` (`:700`). `maosctl_smoke.sh` exercises
`run hello-spirit` (unchanged), `start`/`stop`/`unload hello-spirit` as journal-only offline verbs, and a
negative `maosctl start unknown-spirit` ⇒ exit **2**, journal unchanged (`:127-142`).

---

## 10. SIZING AND BUDGET (rule 6, ×1.3; interval arithmetic)

| crate | headroom | change | raw | ×1.3 upper |
|---|---|---|---|---|
| `maos-control` | +988 | POST + bounded body, header/body split, typed `HttpError` → 400/401/404/405/409/411/413/500/503, route table incl. `GET /v1/spirits/{id}` + `GET /v1/daemon`, `OperatorCommandPort` + typed enums, worker pool + read deadline + `catch_unwind`, accept-error continue, server-side id validation. **Tests → `crates/maos-control/tests/`.** | +250…+430 (+`operation_id` submission, 503 variants — §17) | **+559** |
| `maos-bin` | +2395 | NEW lib `src/operator_door.rs` (port impl, 18 handlers, `Handle` bridge) +520…+850; `BinPrivateOps` impl in `main.rs` +25…+45; bind relocation + root gating + store lock set +45…+80; SIGTERM fixes +10…+25; **minus** 12 deleted `if` blocks + lifecycle-arm reduction −560…−700 | +35…+685 (+per-Spirit serialization; §17: store lock set, completion TL row, shutdown order, butler successor helper, version/cold-swap refusal, anchor capture) | **+891** |
| `maos-cli` | +1566 | door client (discovery, mode check, GET/POST, typed errors, per-verb deadlines, exit-code table) +235…+360; durable readers +50…+90; offline-arm dispatch +35…+55; `stop` refusal; **minus** 13 spawn sites + preflights −220…−300 | +50…+335 (+exit 75, schema checks; §17: home-lock probe, `StoreInUse`, 503 mapping, CRL bytes, canonical upgrade paths) | **+436** |
| `maos-shell` | **+193 (binding)** | `run_init`: ENOENT fix, call the `maos-domain` minter, mint-when-absent | +25…+45 | **+59** — leaves ≥134 for 16-2 |
| `maos-domain` | +497 | NEW operator-door module: `maos_home()`, `ControlFile` + validation, atomic `0600` write, token mint (32 bytes via its existing `getrandom = "0.3"`, `crates/maos-domain/Cargo.toml:19`) | +60…+100 | **+130** |
| `maos-audit` | +106 | `resolve_spirit_name`: `lifecycle.load` rows + boot order by `timestamp_ns` | +10…+25 | **+33** |
| `maos-capability` | +954 | D-16-1-V: re-revoke ⇒ `Err(Revoked)` (`shard.rs` `swap`, `mod.rs` map); its test in `crates/maos-capability/tests/` | +3…+8 | **+11** |
| `maos-kernel-core` | ZERO | — | 0 | 0 |
| `xtask` | ZERO | `xtask/tests/decision_adrs_and_provisioning.rs` only (free) | 0 | 0 |

Spawn sites: 18 today (`subcommands.rs:582,1189,1486,1505,1513,1535,1561,1577,1617,1648,1666,1693,1711,1740,1766,1781,1818,1867`); **13 removed**; **5 survive**: `:582` (governance offline), `:1189` (`run hello-spirit`), `:1486` (`lifecycle_verb`, kept for the uninstall offline arm), `:1513` (legal-hold-release offline), `:1535` (forget offline).

**Every crate forecasts inside headroom → expected ceiling edits: zero.** If one IS crossed: code first,
`cargo fmt --all`, `kloc-check --json`, row comment carrying the bare token **`16-1`** followed by a
non-digit non-dash character, same commit.

Dependencies: **no new RNG dependency** (the minter lives in `maos-domain`, which has `getrandom`; the
lockfile carries three getrandom versions and `deny.toml:63` skips it). **`rustix` 1.1 `fs`** is added
to `maos-bin` and `maos-cli` under `[target.'cfg(unix)'.dependencies]`, matching
`crates/maos-kernel-core/Cargo.toml:65-67` (unix-gated there too). `maos-control` gains no dependency.

---

## Decisions ratified in this story (rule 7 — one decision per fork)

| # | Fork | Ruling | Rejected, and why |
|---|---|---|---|
| **D-16-1-A** | Which of the 21 maosctl one-shot modes go where? | **Four classes, by measured state (§4–§6):** **DOOR, no offline (14):** `pause, resume, start, unload, posture-shift, halt-resolve, orchestrator-queue, orchestrator-status, revoke-token, revocations-import, revocations-list, spirit-upgrade, hot-swap-precheck` + `stop` (refused client-side, D-16-1-I). **DOOR when live, OFFLINE under the exclusive lock (4):** `forget, uninstall, legal-hold-release, governance-admit`. **In-process durable readers in maosctl (2):** `halt-list, legal-hold-list`. **Not re-targeted (1):** `hello-spirit` (`maosctl run`, D-16-1-M). `MAOS_ONE_SHOT_MODES` loses the 14 door modes + 2 reader modes = **48 → 32**. | *The epic's two classes* — 3 of its 5 "TL readers" read memory; 4 of its 16 "mutating" verbs are correct offline. *Everything door-only* — GDPR erasure would require a running root and the 13.5b exit contract breaks. |
| **D-16-1-B** | Which roots bind the door, where, on what port? | **Exactly `{maos run, maos shell, MAOS_ONE_SHOT=cohort-a2a-daemon}`**, gated by root, never by env presence. Bind **moved after `upgrade_orchestrator` (`main.rs:3087`), before `if shell_mode` (`:3272`)**, clock base first, still below `Arc::get_mut(&mut scheduler)` (`:2766`). Config: env overrides (both-or-neither) else `control.json`; the token-only **`127.0.0.1:8787` fallback retired**. No `control.json` and no env ⇒ **no door**, announced once on stderr (`maos: operator door disabled — no <home>/control.json; run \`maos init\``). Second root on the same endpoint ⇒ `EndpointInUse`, fail-fast. ADR-062 amended to name the third root. | *Bind at `:2827`* — half the handlers' objects do not exist. *Env-gated* — every one-shot child binds (measured). *Refuse boot without `control.json`* — reds every root test that never ran init. *Keep 8787* — AC2 forbids a fallback; no consumer. |
| **D-16-1-C** | Where does `control.json` live, and who writes it? | **`MAOS_HOME`, else `$HOME/.maos`, single-sourced in a `maos-domain` module**; `maos_shell::maos_home` delegates. **One writer: `maos init`** (both arms, `main.rs:1385,1708`). Atomic (same-dir temp + `sync_all` + `rename`), mode **`0600`**; home directory created **`0700`**; maosctl and every root **refuse** a `control.json` with a wider mode or not owned by the current uid, naming the file and its mode. Minted when absent even on an initialised home (upgrade path; rotation = `rm control.json && maos init`). Endpoint probed at init from `127.0.0.1:0`. **Schema (§15 F-I):** `{"version": 1, "endpoint": "tcp://127.0.0.1:<port>", "token": "<64 hex>"}` — every reader refuses an unknown `version` or an endpoint scheme other than `tcp://`, so a future Unix-socket transport lands without a schema break. Daemon identity served by `GET /v1/daemon` → `{pid, boot_nonce}`; liveness by the locks (D-16-1-D). | *Daemon writes `pid`/`boot_nonce` into `control.json` (epic AC2)* — a second writer to a secret-bearing file. *XDG rule* — `maos init` already writes by the MAOS_HOME rule. *Resolver in `maos-audit`* — tighter headroom (+106); a home rule is not an audit concern. |
| **D-16-1-D** | Liveness and the offline arm (**re-ruled by validation round 2 and the consistency check, §17 V-2…V-5, V-16**) | **`flock` on a STORE LOCK SET — the store DIRECTORIES' own handles, no lock file is ever created** (`rustix::fs::flock`, unix-gated). The set: the home directory (`maos_home()`, when it exists) **and the directory of every store the four durable verbs write** — the Transparency Log (parent of `resolved_transparency_log_path()`, `crates/maos-bin/src/main.rs:115`), the memory db (parent of `maos_audit::default_transparency_log_path()`, `crates/maos-audit/src/lib.rs:872` — boot's `memory_db_path`, `main.rs:1793`), the private memory root (`maos_audit::default_memory_root()`, `maos-audit/src/lib.rs:1455`), the Lifecycle Journal (parent of `maos_audit::default_journal_path()`, `:1416`), the erasure proofs (`maos_audit::default_erasure_proofs_dir()`, `:1583`). **Resolved by those functions, never re-implemented**; a relative path is resolved against the cwd as the store itself would be. Each directory is **created if absent, mode `0700` for every component it creates** (`DirBuilder` + `mode(0o700)`, where boot uses `create_dir_all` at `main.rs:1795`, `:1844`, `:2733-2736` — the proofs dir, otherwise created lazily at `maos-audit/src/erasure/proof.rs:450`, is created here too); **a path that exists but is not a directory locks its parent instead** (`erasure_uninstall_13_5b.rs:627-637` plants one to force exit 5, which must survive). Then each is opened read-only with `std::fs::File::open` (which sets `O_CLOEXEC`, so a Worker spawned by the root never inherits a lock), **deduplicated by `(st_dev, st_ino)` of the open handle** (canonical paths miss bind-mount aliases, and a second `LOCK_EX` on the same inode from a second handle fails against the process's own first), sorted, and locked all-or-nothing; **after each lock, `fstat(handle)` must equal `stat(path)` by `(dev, ino)`, else release and retry** (a directory removed and re-created between open and lock — `maos purge` — must not be locked by a stale inode). A lock call that fails for any reason other than `EWOULDBLOCK` (e.g. `EBADF` for `LOCK_EX` on NFS, which requires a writable handle a directory cannot have; `EACCES`; a `mkdir` failure, including the `/var/lib/maos` last-resort fallback when `HOME` and XDG are unset) ⇒ typed **`LockUnavailable { path, errno }`** 78 — **network filesystems are unsupported for the offline arm**; roots refuse to boot with the same error rather than run unlocked. **Taken before any store is opened** — before `main.rs:1793` (feasible: `resolved_transparency_log_path()` reads env and stats ancestors via `validate_transparency_log_path` (`maos-audit/src/lib.rs:949-966`) but opens no store, and `MAOS_ONE_SHOT`, `run_args` and `shell_mode` are known by `:1770`); never at the bind or the one-shot dispatch (by `:5159` boot has opened the TL `:2038`, written the tenant binding `:2061`, opened the journal `:2743`). Door roots take **`LOCK_SH \| LOCK_NB`**; **the offline durable child (`maos`, not `maosctl`) takes `LOCK_EX \| LOCK_NB`**; **both retry for up to 500 ms** (absorbing maosctl's momentary probe) and then refuse — the root with 69 `OfflineOperationInProgress`, the child with typed **`StoreInUse { path }`** 69 and no write (a neutral name: `flock` cannot tell a door-less daemon from a second offline child). The child prints `maos: no daemon holds <paths>; running offline` **after** acquiring. **maosctl order for a durable verb:** no endpoint configured — neither the env pair (`MAOS_OPERATOR_HTTP_ENDPOINT` + token) nor `control.json` — ⇒ offline child (erasure never requires `maos init`); connect refused ⇒ offline child, whose lock decides; connected but no well-formed HTTP response ⇒ `DoorUnresponsive` 69, **never** offline, stderr naming the remedy (*if no MAOS daemon owns that endpoint, re-mint it: `rm <home>/control.json && maos init`*); a response (including 401 ⇒ 77) ⇒ that response, **never** offline. **maosctl never holds a lock**: for a DOOR verb whose connect is refused it probes the home directory with `LOCK_EX \| LOCK_NB` and releases at once — acquired ⇒ `DaemonNotRunning` 69, refused ⇒ `StoreInUse` 69. **Advisory**: coordinates MAOS processes, not an arbitrary `sqlite3` client. **Residual, stated:** another uid that can open a store directory — one that pre-dates this story with boot's default `0755`, or a shared parent such as `/tmp` named through `MAOS_AUDIT_DB` — can hold a lock and make **offline erasure refuse and roots refuse to boot** (69 after the retry: fail-safe, never a race, but a local denial of service). Such a uid can already read those stores; directories this story creates are `0700`, and hardening existing ones is not claimed. *(Round 2's `0600` lock files had been the guard against this; V-16 traded it for not touching store contents, and V-23 states the trade.)* **Non-unix**: no release target is non-unix (`.github/workflows/release.yml:43-51`); the offline arm refuses typed `OfflineUnsupported` (69). | *PID file / connect probe* — racy; PID reuse lies. *`std::fs::File::lock`* — stable in 1.89, MSRV is 1.88 (`Cargo.toml:68`). *A new lock crate* — `rustix` `fs` already in the workspace. *Exclusive lock for roots* — f4/2b run two roots on one `HOME` by design. *One lock in `MAOS_HOME` (pre-round-table)* — keyed to the address, not the store (§15 F-C). *Two named files, home + memory root (round-table)* — with `MAOS_HOME` unset the TL, memory db and proofs resolve through `MAOS_AUDIT_DB`/XDG, so a root and a child with different `HOME`s shared the log unlocked (§17 V-2/V-3). *A `.maos-daemon.lock` FILE in each store directory (validation round 2)* — reds 7 assertions in `erasure_uninstall_13_5b` that count files in the proofs dir and memory root, and a shared parent such as `/tmp` yields a foreign-owned lock file (§17 V-16). *A lock directory under `/tmp` or `$XDG_RUNTIME_DIR`* — squattable by another uid, or split by env. *`DaemonAliveWithoutDoor`* — a claim `flock` cannot justify (§17 V-4). |
| **D-16-1-E** | Server concurrency and robustness | **Bounded worker pool, cap 16**; each accepted connection gets a worker; over cap ⇒ immediate `503` fixed body. **Request-read deadline 5 s** covering headers + body (replaces the per-`read` timeout). Header parse stops at the first blank line; POST requires `Content-Length` (`411` if absent), ≤ 64 KiB (`413` without reading the body). Handler execution is **not** under the read deadline: each route has a budget (10 s default; 30 s for upgrade, uninstall, revocations import, hot-swap-precheck); the kernel future is **spawned on the runtime and never cancelled** — on budget expiry the worker answers `503 HandlerStillRunning` whose body carries an **`operation_id`**; the task runs to completion and writes its completion TL row **carrying that id** (§15 F-F), so the operator can find it with `maosctl audit query --intent-contains <operation_id>` instead of retrying blind. **Mechanics (§17 V-1):** `operation_id` = `op-<boot_nonce hex>-<seq>`, minted by maos-bin's port at `submit`; a MUTATING command that STARTS writes, on completion (success or failure), ONE row via `transparency_log.insert_frame_event(FrameKind::TelemetryEvent, <pid>, None, "operator.command:<verb>:<spirit>:<operation_id>:<outcome>", <json>, FrameOrigin::Kernel)` — the maos-bin pattern at `main.rs:2613-2621`, zero kernel-Δ (the kernel journal helpers hard-code `intent`, `orchestrator/mod.rs:62-78`, and stay untouched). `maosctl audit query --intent-contains` (`crates/maos-cli/src/cli.rs:430`) filters the TL `intent` column (`maos-audit/src/lib.rs:264-266`, `:444-445`); **never document `maos audit query`, whose parser silently ignores unknown flags and prints every row** (`main.rs:1714-1739`). Per-Spirit serialization (D-16-1-R) waits **inside** the route budget, arbitrated by **one atomic state shared by server and port** (`OperatorSubmission.state`: `Queued → Started | Withdrawn`, a std `AtomicU8`, no tokio in maos-control): the port CASes `Queued → Started` after acquiring the Spirit's lock and runs only if that succeeds; the server, at the deadline with no outcome, CASes `Queued → Withdrawn` — success ⇒ `503 SpiritBusy` (never ran; the minted id is discarded, no row), failure (already `Started`) ⇒ `503 HandlerStillRunning` with the id. **No timer race:** a starved runtime or an unpolled task can delay the port, never make the two sides disagree (§17 V-25). Pool full ⇒ `503 Busy`. Per-request `catch_unwind` ⇒ `500` fixed body; the server survives. Non-`WouldBlock` accept errors are logged and retried with backoff, never `break`. Closes `deferred-work.md:869`. The server stays `std::thread` + tokio-free. | *Serial* — one slow POST stalls every verb, before auth. *Cancel the future on timeout* — can drop mid-`fire_on_pause` with state already committed (`scheduler_loop.rs:403-422`). *hyper/tokio in maos-control* — a direct tokio dep and an HTTP stack in a 351-line surface. |
| **D-16-1-F** | Sync server ↔ async kernel | **`maos-control` defines a SYNC `OperatorCommandPort`** (`submit(OperatorCommand) -> OperatorSubmission { operation_id, state, completion }`, `state` the shared `Queued/Started/Withdrawn` atomic (D-16-1-E), `completion` a one-shot receiver of typed `OperatorOutcome` that the server worker waits on until the route deadline — so the server hands back an id it did not mint and withdraws a queued command by one CAS; the `SandboxReportSource`/`RotationWindowSource`/`CohortConvergenceSource` shape, `lib.rs:19,77,131`). **`maos-bin` implements it** (D-16-1-N) holding a `tokio::runtime::Handle` captured in async `main` (`multi_thread`, `main.rs:1663`), spawning each kernel future on it from the server worker thread. New constructor **`OperatorHttpServer::bind_with_commands(.., Option<Arc<dyn OperatorCommandPort>>)`**; `bind` delegates with `None`, so its existing call sites — **18 in tests** (14 in `lib.rs` at `:517,583,620,633,651,674,747,800,816,834,898,935,948,967`; `t_14_2b_cohort_convergence.rs:532`; `t_14_2c_local_leaf_declaration.rs:435,523,669`) plus production `main.rs:2829` — do not change. | *`bind` takes a `Handle`* — tokio becomes a direct maos-control dep and 19 sites break. *A field on `OperatorHttpConfig`* — every struct-literal site breaks. *Handlers in maos-control* — it cannot reach `run_uninstall_cascade`, `migration_plan` or `maos-director-surface`. |
| **D-16-1-G** | 405, the three POST tests, route namespaces | **Every non-GET on a read route ⇒ `401` if anonymous, else `405` with a fixed byte literal** (ADR-062 `:71-77`). **All three** HEAD `POST → 404` tests (`lib.rs:607,786,927`) become post-auth 405 **guards**; plus a negative that **no POST route exists under `/v1/cohort/` or `/v1/a2a/`**. `decision_adrs_and_provisioning.rs:330-338` flips to `require_contains` the 405 literal; the four GET literals stay. ADR-062 amended: three guards. | *"Narrow the generic one" (ADR-062 `:68-69`)* — `:927` guards self-identity, which `:63` preserves. *Leave the gate* — reds on the first 405. |
| **D-16-1-H** | Route shape | `GET /v1/daemon` · `GET /v1/spirits/{id}` → `{spirit_id, pid, boot_nonce, lifecycle_state, posture}` · `POST /v1/spirits/{id}/{start,pause,resume,unload,posture,upgrade,hot-swap-precheck,uninstall}` · `POST /v1/halts/{halt_id}/resolve` · `GET`/`POST /v1/orchestrator/{id}` · `POST /v1/tokens/{token_id}/revoke` · `GET`/`POST /v1/revocations` · `POST /v1/memory/forget` · `POST /v1/legal-holds/{principal}/release` · `POST /v1/governance/schemas`. JSON bodies; typed `outcome`, plus the one-shot's terminal code where it had one (`uninstall` 0/3/4/5, `forget` 3, precheck 0/2). Ids validated server-side (`[A-Za-z0-9._-]`, the client's existing rule). `/sandbox` matched before the bare id route. A dev may rename a path only in the same commit as the ADR-062 amendment. | *One `POST /v1/command` RPC* — un-auditable in a request log; makes 405 meaningless. |
| **D-16-1-I** | `stop` | **Refused client-side, exit 2, no journal row, no round trip, and BEFORE the `MAOS_ACCESSIBILITY_SMOKE` short-circuit:** *"no kernel transition is named stop — use `maosctl pause` (resumable) or `maosctl unload` (terminal)"*. `ScbLifecycleState` has no stopped state (`control_block.rs:31-36`); FR9 lists load/start/pause/resume/unload. | *stop → pause* — a "stopped" Spirit that resumes. *stop → unload* — silently destructive (`scheduler_loop.rs:505-516`). *stop → journal `Halt` (HEAD)* — measured lying. *Smoke short-circuit first* — keeps printing `stop smoke ok`. |
| **D-16-1-J** | `start` | **`scheduler.start(pid)`** (`scheduler_loop.rs:374`), `Loaded → Running` only; on a Running Spirit (every `maos run` root starts its Spirit, `main.rs:5006`) ⇒ `409 InvalidStateTransition`, no row. | *Idempotent success* — hides the state the operator asked about. |
| **D-16-1-K** | Name → pid | **Door verbs: daemon-side `resolve_pid`; unknown ⇒ `404 SpiritNotLoaded`**; maosctl's TL preflight deleted for door verbs. **Offline readers: `resolve_spirit_name` fixed at origin** — recognise `lifecycle.load` rows (`spirit_id` in payload), order boots by `timestamp_ns`. The `hello-spirit → pid 0` wildcard (`maos-audit/src/lib.rs:1624-1665`) stays → 16-2. | *Keep the preflight* — rejects the running butler. *Remove the wildcard now* — reds the v0.1 evaluator path. |
| **D-16-1-L** | Composed side effects | **Kernel transition FIRST; rows only on success.** `pause`/`resume`/`start`/`unload`: scheduler → Lifecycle Journal row via the daemon-held `shared_journal` (`main.rs:2743`) → `journal_director_lifecycle_action`. `posture`: `resolve_pid` → `policy.shift_posture(pid, ..)` → Journal `PostureShift` → `journal_posture_shift`. `uninstall`: `scheduler.unload(pid)` (revokes live tokens, drains halts, planned-unload receipts) → `run_uninstall_cascade`. | *Rows first (HEAD)* — the journal says paused when the scheduler refused. |
| **D-16-1-M** | `maosctl run hello-spirit` | **Not re-targeted** — the FR58 evaluator turn, no daemon counterpart; `maosctl-smoke`, `v01-evaluator-path`, `onb_nfr2_timing.sh`, `audit_query_fr4_smoke.sh`, `server_exit_drain.sh`, `one_shot_hello.rs` depend on it running without a daemon. | *Route it to the door (epic AC3 listed `run`)* — breaks the v0.1 evaluator path. |
| **D-16-1-N** | Where the handlers live | **NEW `crates/maos-bin/src/operator_door.rs` as a LIB module** (`pub mod operator_door;` in `lib.rs`, beside `cert_rotation`), holding the port impl over kernel objects. The four bin-private operations it needs — `run_uninstall_cascade` (`main.rs:8548`), `append_lifecycle_journal` (`:8513`), `enforce_vetted_upgrade_precondition` (`:133`), `migration_plan::upgrade_with_plan_guard` (`mod migration_plan`, `:37`) — enter through a small **`BinPrivateOps`** trait implemented in `main.rs`. Consequence: in-process proofs live in **`crates/maos-bin/tests/operator_door_16_1.rs` (kloc-free)**, not inline. `SCANNED_SOURCE_FILES` (`cohort_daemon_smoke_13_5c.rs:869`) **19 → 20** in the same commit. | *Bin module (`mod operator_door;` in `main.rs`)* — its proofs would have to be inline `#[cfg(test)]`, charged by kloc and unpriced. *Inline in the 14,347-line `main.rs`* — the 14-2b dodge. *Make the four fns `pub` in lib* — `migration_plan` and the cascade drag bin-only wiring into the library surface. |
| **D-16-1-O** | Sizing | **WHOLE**, flagged. Seam, if ordered: §13. | *Split by crate* — a door without real handlers is a claim standing in for a control. |
| **D-16-1-P** | Exit codes for door errors | **sysexits for transport/auth, preserved codes for terminal outcomes:** `0` success · `1` typed application error (404 not loaded, 409 invalid state, 500) and protocol errors maosctl never provokes (400, 405, 411, 413 — a client/daemon version skew; stderr names the status) · `2` usage (clap, local validation, `stop` refusal) · **`75` (EX_TEMPFAIL)** every retryable 503: `HandlerStillRunning` (stderr names the `operation_id` and `maosctl audit query --intent-contains <id>`, §15 F-F), `SpiritBusy` and `Busy` (nothing started, no id) · **`69` (EX_UNAVAILABLE)** `DaemonNotRunning`, `StoreInUse`, `DoorUnresponsive` (connected, no well-formed HTTP response), `OfflineOperationInProgress`, `OfflineUnsupported` · **`77` (EX_NOPERM)** 401 · **`78` (EX_CONFIG)** `NotInitialized`, `LockUnavailable` (a store directory that cannot be created or locked — NFS, permissions), `EndpointInUse`, bad `control.json` mode or owner, env override half-set · **preserved:** `uninstall` 3/4/5, `forget` 3, `hot-swap-precheck` 2 (unsafe) — whether answered by the door or the offline child — and `spirit inspect --sandbox`'s HEAD `4`/`5` (`subcommands.rs:1916-1927`; its GET `503 Unhealthy` keeps that mapping). | *401 ⇒ 4 everywhere* — collides with `uninstall`'s `not_found` = 4: an auth failure would read "nothing to erase". *Everything ⇒ 1* — scripts cannot tell "retry later" from "you are not allowed". |
| **D-16-1-Q** | Shared-HOME roots in the test suite | **Every test that spawns a door root sets `HOME` to a per-test tempdir, and sets `XDG_DATA_HOME` per test or removes it** (memory root, journal and proofs follow an exported `XDG_DATA_HOME`, §17 V-32) (no `MAOS_HOME` change, so f4/2b keep their `MAOS_AUDIT_DB` TL routing): `f4_pairing`, `two_host_delegation_2b`, `maos_run_boot_loud_8_11`, `researcher_8_14c`, `inference_provider_polarity_8_11`, `smoke_run_8_11`, `smoke_mira_nash_tcp_8_13`, `jb3_self_tuning_halt`, `cohort_daemon_smoke_13_5c` — each re-run green. **Validation round 2 found more** with neither `HOME` nor `MAOS_HOME`: `butler_8_14b` (`:41,82,253,348`), `smoke_cli_wrapper_8_12` (`:56,115,219`), `cross_team_crossing_13_6b` (`:1677` via `daemon_command` `:1599`), and a second `cohort_daemon_smoke_13_5c` site (`:732`); `cert_rotation_trigger_14_2a` (`:239`) sets both env overrides, never reads `control.json`, and is an allowlist entry, not an offender; a bare `maos` with no arguments is a shell root (`main.rs:1694-1699`). **T0 derives the list from the decoy run below, not from grep.** Plus a guard vector in `one_daemon_one_door_16_1`: two roots on one initialised home ⇒ the second exits **78** typed `EndpointInUse`, proving the fail-fast is intended. **And a control, not a promise (§15 F-H, re-shaped by §17 V-6 — a textual "same command" has no boundary: builders split across functions, `MAOS_HOME` added after a builder returns, journey spawns isolated by `crates/maos-journey-test/src/lib.rs:109-117`):** (1) **the binding control is a CI decoy** — in the `workspace-test-suite` job's test step (`discipline.yml:3534`), in the same shell before `cargo test`, build `maos`, run `maos init` against the runner's real `HOME` (needs T1's clean-home fix) and hold its `control.json` endpoint with a listener that accepts and closes; any root spawn that reads the real home, whatever builds its `Command`, then fails `EndpointInUse` deterministically, and any maosctl verb that reads it gets `DoorUnresponsive` 69 — never an offline write into the runner's real stores — proven red by removing `HOME` from one fixed test (estimate before the fix: ≈12 maos-bin/journey files red; maos-cli barely, since `accessibility_test` uses `env_clear`); (2) **the fast local floor** is a kloc-free `crates/maos-bin/tests/root_spawn_home_isolation_16_1.rs` at FILE granularity — every `crates/*/tests/**/*.rs` naming `CARGO_BIN_EXE_maos"` must contain a `"HOME"` or `"MAOS_HOME"` literal or sit on the lint's allowlist with a one-line reason (one-shot only; journey-test isolation; both env overrides set; `env_clear`) — proven red on the files that contain no isolation literal at all. The floor cannot see `butler_8_14b`, `smoke_cli_wrapper_8_12` or `cross_team_crossing_13_6b` (each already contains a `HOME`/`MAOS_HOME` literal elsewhere); the decoy does. | *No door unless a flag is set* — contradicts ADR-062 (discovery from `control.json` is the contract). *Leave it* — machine-dependent reds CI never sees. |
| **D-16-1-R** | Concurrent mutating commands on one Spirit (§15 F-G) | **Mutating commands are serialized per Spirit id** inside `operator_door.rs` (a map of per-id async mutexes, in maos-bin — the I9 `Mutex` denylist governs kernel-core, not this crate); reads are not serialized. The kernel's CAS already keeps SCB state sane; the lock is for the rows — two operators pausing and unloading the same Spirit must not interleave journal and approval-log rows. Proven by a concurrency vector in `operator_door_16_1.rs`: N concurrent pause/resume/unload on one id ⇒ every journal row pairs with exactly one approval-log row, in one consistent order. A command waits for its Spirit's lock **inside its own route budget**; at the deadline the server withdraws it if still `Queued` ⇒ `503 SpiritBusy` (D-16-1-E; §17 V-7, V-25). | *Rely on the kernel CAS* — state is safe, the audit trail is not. *A global command lock* — one slow upgrade would stall every Spirit's pause. |
| **D-16-1-S** | Who else can reach the door (§15 F-D) | **Measured:** `spawn_and_bridge` (`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:461-469`) has no `env_clear`, a green test forbids adding one (`crates/maos-cli/tests/credential_posture_2c.rs:299-310`), and Workers run as the operator uid — so a bare, prompt-injectable agent CLI inherits any exported `MAOS_OPERATOR_BEARER_TOKEN` and can read the `0600` `control.json`. **Not fixable in 16-1** (the strip is a kernel-core byte). **Ruled:** (1) the cure is 17-1's own AC3 `env_clear` + AC1/AC4 T3 isolation, and this commit **writes the operator bearer and `control.json` into 17-1's AC1/AC3 by name** with a probe assertion; (2) 16-1 documents the residual in `README.md` and the ADR-062 amendment: *until 17-1 lands, every Worker can reach the door; prefer `control.json` over the env override, which every child inherits*. | *Strip the env in `runtime.rs` now* — kernel Δ, reds the 16-0 pin and a green posture test. *Say nothing because 17-1 will fix it* — 17-1's ACs named vendor keys only; it would have closed green with the door token in the child env. |
| **D-16-1-T** | FR9 `load` (§15 F-B) | **NEW row `16-6-maosctl-load-over-the-door`** in Epic 16, after 16-1 and before 16-5, reusing `maos run`'s admission path behind this door via `BinPrivateOps`; epic *Closes* and Dependencies point at it; its threat model (AC3) states the F-D residual. | *Inside 16-1* — already 1.5–2× its sketch. *Narrow the epic's FR9 claim* — drift from the PRD the epic cites. *Park it in 19-x/20-1* — no charter match (rule 10). |
| **D-16-1-U** | FR51 claim (§15 F-E) | **Narrowed to FR51(b) + door reachability of (a)(d).** (c) — recall of pending Orchestrator-buffered actions — has no route either (`recall_all_pending` `buffer.rs:72`; its only production caller re-enqueues at resume, `scheduler_loop.rs:452-460`) → routed by charter to **19-2 AC3** (§17 V-12). Bounded interrupt of an in-flight action and bounded fail-safe of in-flight ops have no mechanism; named in the epic's *Closes* line and raised to the Epic-16 retrospective (rule 9's refinement vehicle). | *Claim it* — a claim standing in for a control. *Build hook cancellation here* — SCOPE: cancelling a running Spirit hook is a new kernel mechanism. |
| **D-16-1-V** | A repeated revoke (§17 spike Q1) | **`CapTokensShardRing::revoke` of an already-revoked token with `RevokeReason::Operator` ⇒ `Err(CapError::Revoked)` and no second audit event; with `RevokeReason::CliSubprocessExit` ⇒ `Ok(())` and its audit event as today** (exit provenance is never lost, §17 V-35) — `CapShard::set_revoked` (`crates/maos-capability/src/cap_tokens/shard.rs:44-52`) takes `revoked.swap(true)` and returns the prior flag as a value (`Option<bool>`: `None` absent — do not overload its `Err(())`); `revoke` (`cap_tokens/mod.rs:272-287`) maps an already-revoked token to the existing variant for the operator reason only (`crates/maos-domain/src/ports/capability.rs:125`). A few lines in `maos-capability`, outside the kernel-core pin. Production callers: `worker_spawn.rs:730` calls `revoke_cli_subprocess_exit`, which forwards to `tokens.revoke` and discards the result — so the reason-keyed rule keeps the Worker's `CliSubprocessExit` row even for a token already revoked by the operator, by unload or by a CRL (`revoke_for_spirit`) — without it that row would vanish for every Worker that exits after its Spirit is unloaded (§17 V-35); kernel `capability/mod.rs:329,374` forward it; CRL apply and unload use `revoke_for_spirit` (untouched). Tokens never leave the ring (`evict_expired` has no caller), so `UnknownToken` stays reserved for never-issued ids. | *Evict revoked tokens* — breaks `verify ⇒ Revoked` (`crates/maos-kernel-core/tests/revocation_verify_denial.rs:35-39`). *Keep `Ok` twice (HEAD)* — each repeat writes another `cap.revoke` row, and the door cannot show that a revoke changed anything. |
| **D-16-1-W** | Upgrade over the door (§17 spike Q3) | **Hot-swap only, forward only, faithful successor.** (1) `--policy cold-swap` ⇒ typed `409 ColdSwapUnsupported` — measured: cold-swap starts the successor under a new pid (pid 2 in the spike) without `admit_spirit` (the kernel `ColdSwap` arm, `crates/maos-kernel-core/src/lifecycle/upgrade.rs:162` — a kernel byte) ⇒ §11 row 11; maos-bin's `pid_by_spirit_id` (`main.rs:2428`) is also never refreshed (read from code, not instrumented by the spike) — that half is zero-Δ, but fixing it alone leaves an unadmitted Spirit running. (2) successor `[class] version` ≤ the loaded one ⇒ typed `409 VersionNotIncreasing` — measured: 0.3.0 → 0.2.0 reported `completed`. (3) A **butler arm in the successor factory** (`main.rs:3046-3084` has none — *"no successor loader registered for class 'butler'"*), built by ONE helper extracted from the `maos run` butler construction, with the policy re-parsed from the target manifest (`SpiritManifestBundle` carries no `[epistemic_policy]`; widening it is a kernel byte) — measured: a bare `Butler::new()` successor completes the swap and silently loses its halt. (4) `--to`, `--attestation`, `--keyring` are canonicalised client-side and sent absolute; `enforce_vetted_upgrade_precondition` (`main.rs:133`) takes them as parameters through `BinPrivateOps` instead of reading `MAOS_UPGRADE_TO_ATTESTATION`/`MAOS_VETTER_KEYRING` from the daemon's environment (`:178`, `:6737`). | *Cold-swap over the door* — kernel Δ. *Any version* — a downgrade that reports success. *Bare successor* — `completed` while the Spirit lost its behaviour. *Daemon-env attestation* — one daemon, many targets; relative paths resolve in the daemon's cwd. |
| **D-16-1-X** | CRL import over the door (§17 spike Q2) | **CRL bytes in the POST body (≤ 64 KiB); the trust anchor captured at boot** — clone `revocation_trust_anchor` before it moves into `LocalFileRegistryClient` (`main.rs:2890-2899`); `revocations list` renders `CrlId` as hex (serde yields a 32-integer array). Measured on a live butler root: `apply_crl` ⇒ `matched_count 1`, SCB gone in 8 ms, `spirit.revoked` + `lifecycle.unload` rows. | *Send a path* — resolves in the daemon's cwd. *Re-read `MAOS_CRL_TRUST_ANCHOR_PUB_HEX` per request (the one-shot's habit)* — the anchor could change under a running daemon. |

---

## Acceptance Criteria (8)

**AC1 — (command) exit lines 5–6 against a real root, and the pause is REAL.**
`cargo test -p maos-bin --test one_daemon_one_door_16_1` (new, named in its own CI step). In an
isolated `HOME`/`MAOS_HOME`/`XDG_DATA_HOME` it runs `maos init`, starts
`MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json maos run spirits/butler/manifest.toml`,
and waits **bounded** until `GET /v1/spirits/butler` answers `lifecycle_state: Running` — **not** until
the endpoint merely accepts a connection (the bind precedes `scheduler.start` at `main.rs:5006`, so a
connect-ready door can still answer 404) — then asserts in order:
1. `maosctl pause butler` exits 0; `maosctl spirit inspect butler` (no `--sandbox`) exits 0 and reports
   `lifecycle_state: Paused`, read from `SpiritControlBlock::current_state` (`control_block.rs:395`)
   through `scbs()` (`scheduler_loop.rs:148`); the daemon's Lifecycle Journal carries a `Pause` row for
   `butler`, the TL a `lifecycle.pause` row for butler's **real pid** (not 0), and the approval log a
   director row.
2. **Falsifier:** with the handler's `scheduler.pause` call removed, step 1 reds on `lifecycle_state`
   (a journal-only handler cannot pass). Run, recorded in the Debug Log, reverted.
3. `maosctl posture butler --shift cautious` exits 0; `spirit inspect` reports `posture: Cautious`, read
   from the same `PolicyTable` `ArcSwap` (`cap_policy/mod.rs:86`) that `evaluate_with_posture` (`:278`)
   reads; the approval log carries the posture-shift row. `crates/maos-bin/tests/operator_door_16_1.rs`
   proves in-process that `evaluate_with_posture(pid, <routine class>)` changes across the port call —
   *the next capability decision* (FR16), not a label.
4. `maosctl resume butler` → `Running`; `maosctl start butler` ⇒ exit 1, typed `InvalidStateTransition`
   (D-16-1-J); `maosctl stop butler` ⇒ exit 2 with the D-16-1-I message and **no** journal row.
5. SIGTERM to the idle root exits 0 within **3 s** (HEAD: 10.0 s, three measurements — §11 row 3) with no
   `audit writer drain timed out` line, and every lock in the set is released (an immediate `LOCK_EX|LOCK_NB` on each succeeds). A
   command in flight at SIGTERM runs to completion and its row is in the TL (Trap 16).

`spirit inspect <id> --sandbox` keeps its HEAD report and exit codes (`subcommands.rs:1916-1927`).

**AC2 — `control.json`, discovery, door roots, the lock (D-16-1-B/C/D/Q).**
- `maos init` on a home that **does not exist** exits 0 and creates it — proven by a NEW test whose
  fixture does not create the directory, **red before the fix**; the four existing init tests stay green.
- `maos init` writes `<home>/control.json` = `{"version":1,"endpoint":"tcp://127.0.0.1:<port>","token":"<64 hex>"}`
  atomically, mode `0600`, home `0700`; re-running with `control.json` present changes **neither** byte;
  on an initialised home **without** it, mints it. Both init arms (`main.rs:1385`, `:1708`) reach the
  same writer.
- The home rule, schema, validation and minter live in **one** `maos-domain` module used by `maos-shell`,
  `maos-bin` and `maos-cli`.
- Door roots are exactly `maos run`, `maos shell` (including a bare `maos`, `main.rs:1694-1699`),
  `MAOS_ONE_SHOT=cohort-a2a-daemon`. **Proven:** with a live root on an env-override endpoint and **both**
  `MAOS_OPERATOR_HTTP_BIND=<that endpoint>` and `MAOS_OPERATOR_BEARER_TOKEN` exported (the root's both-or-neither
  pair, `main.rs:2705-2707`; maosctl's pair is `MAOS_OPERATOR_HTTP_ENDPOINT` + the token),
  `MAOS_ONE_SHOT=smoke-epic-4 maos` does **not** fail with `Address already in use`. `cert_rotation_trigger_14_2a.rs` stays green (marker `main.rs:2840`
  unchanged).
- Second root on the same endpoint ⇒ typed `EndpointInUse` naming `control.json`, exit 78; no fallback
  port; `8787` gone from `main.rs` and `env_contract.rs:481`.
- The bind happens after `upgrade_orchestrator` (`main.rs:3087`) with the clock base initialised; the lock set
  (D-16-1-D) is taken **before `main.rs:1793`**, not at the bind. Roots hold `LOCK_SH` on every lock in the set; a
  root started while `LOCK_EX` is held on any of them exits 69 `OfflineOperationInProgress` after its 500 ms retry.
  **Shared-log vector (§17 V-2):** a root and an offline `forget` with different `HOME`s, `MAOS_HOME` unset and one
  `MAOS_AUDIT_DB` ⇒ the child exits 69 `StoreInUse` naming the TL directory and writes nothing — red against the
  round-table's two-file lock.
- **CLOEXEC vector** (in-process, `operator_door_16_1.rs`): acquire the lock set, spawn `sleep 30`, drop the
  parent's handles ⇒ a fresh `LOCK_EX|LOCK_NB` on every directory succeeds (the child inherited none). **No-file
  vector:** acquiring the set creates no file in any store directory, and `erasure_uninstall_13_5b` stays green
  unchanged (its proofs-dir and memory-root file counts, `:458,480,503,607,623,846`, and its planted-file exit 5,
  `:627-637`).
- maosctl and every root refuse a `control.json` whose `version` is not `1` or whose endpoint scheme is
  not `tcp://` (78, naming the field).
- maosctl discovery: env overrides (both or neither; half-set ⇒ 78) → `control.json`; wider mode or
  foreign owner ⇒ 78 naming the mode; endpoint must be a loopback `SocketAddr` (the rule at
  `subcommands.rs:1970-1977`).
- `maos-cli` gains **no** `maos-control`/`maos-kernel-core` edge, dev edges included
  (`dep_kernel_core_free_test.rs` green); `check-workspace-count` stays 55.
- D-16-1-Q applied: every shared-HOME test T0 derives from the decoy run sets a per-test `HOME` and is green;
  the decoy step and the file-granular floor are both proven red first.

**AC3 — the POST surface is bounded, typed, concurrent, and its guards hold (D-16-1-E/F/G/H).**
In `crates/maos-control/tests/post_surface_16_1.rs` (kloc-free), each with a falsifier:
- anonymous request with any method to any path ⇒ `401` (auth before route and method);
- authenticated non-GET on each of the six GET routes (four HEAD + `/v1/spirits/{id}` + `/v1/daemon`)
  ⇒ `405`, body **byte-equal** to the fixed literal; the three HEAD `POST → 404` assertions
  (`lib.rs:607,786,927`) become 405 guards; no POST route matches under `/v1/cohort/` or `/v1/a2a/`;
- a body line `Authorization: Bearer <real-token>` with a wrong header token ⇒ `401`;
- POST without `Content-Length` ⇒ `411`; `> 64 KiB` ⇒ `413` **without reading the body**; malformed
  JSON ⇒ `400`; unknown spirit ⇒ `404` typed; invalid state ⇒ `409` typed;
- **8** idle connections held open do not prevent a 9th authenticated request from completing;
  **16** held ⇒ the 17th gets `503`;
- a command port that panics ⇒ `500` fixed body, and the next request succeeds;
- a command port that exceeds its route budget ⇒ `503 HandlerStillRunning` carrying the port's `operation_id`,
  the port call still completes (observed side effect); a fake port that leaves `state` at `Queued` ⇒ the server's
  CAS wins ⇒ `503 SpiritBusy`, no id (the mapping only — whether a withdrawn command truly never runs is proven
  against the REAL port in `operator_door_16_1.rs`: hold one Spirit busy past a second command's deadline ⇒
  `SpiritBusy`, no side effect, no row); 17 concurrent ⇒ `503 Busy`. In maos-bin
  (`operator_door_16_1.rs`) the completion is exactly ONE TL row whose `intent` contains the id, found by
  `maosctl audit query --intent-contains <id>`; maosctl exits 75 on all three 503s, naming the id and that query
  for `HandlerStillRunning`;
- a byte-dripping client is cut at the 5 s read deadline;
- ids outside `[A-Za-z0-9._-]` ⇒ `400`; `/v1/spirits/x/sandbox` still reaches the sandbox route.
`xtask/tests/decision_adrs_and_provisioning.rs` (`"062"` arm `:310-339`) re-anchored in the same commit
and green — **and extended to `require_contains` the ADR's `## Amendment — Story 16-1` heading, `control.json`
and `cohort-a2a-daemon`**, so an unamended ADR reds (§17 V-11: today the arm checks only the ADR's `## Context` section, `decision_adrs_and_provisioning.rs:187,240`, and the amendment heading lies outside it — the check reads the whole body). ADR-062 amended (§12 row 17). `deferred-work.md:869` closed with its citation.

**AC4 — the verbs that were fake become real (D-16-1-A/L/V/W/X).** Against live roots in
`one_daemon_one_door_16_1`, **one fresh root per state-changing concern, so no step contaminates another**, or
in-process in `operator_door_16_1.rs` against the real port and real kernel objects — never against a registry the
test filled and read back. Roots A–D's fixture recipes were run by the §17 spike at `f72b557b` (evidence
`16-1-evidence/spike/`); root E's `MAOS_IDLE_FAST=1` timing was NOT (the faithful successor's post-swap halt was
reported by the spike at default timing, 30.007 s, a figure not in the preserved logs) — T0 measures it, and if the fast-idle halt does not re-fire after a swap the
step uses default timing with a 45 s bound:
- **root A** (butler replay, non-destructive, in order): `maosctl orchestrator queue --spirit butler "<text>"` ⇒
  `orchestrator status` reports `1/32`, not `0/32`; `maosctl halt resolve <never-issued id> --spirit butler …` ⇒
  exit 1 typed `HaltNotPending`, **no** resolution row (never-issued, because butler's own `belief_variance` halt is
  pending from ≈30 s, `spirit_pid` 0 — Trap 19); `maosctl spirit hot-swap-precheck` prechecks the **loaded** SCB and
  writes **no** `lifecycle.load` row.
- **root B** (butler, fresh, `MAOS_CRL_TRUST_ANCHOR_PUB_HEX` on the **daemon**): `maosctl revocations import <file>`
  sends the CRL **bytes** (D-16-1-X) of a CRL the test signs — `RevocationEntry::new("butler", ">=0.3.0,<0.4.0", …)`,
  `ring` Ed25519 over `canonical_entries_bytes`, `CrlId::from_entries`, `SignedRevocationList::new` (no new
  dependency; matching on class name/version, `kernel-core/src/revocation/applier.rs:99-121`) ⇒ `matched_count 1`,
  `revoked_count 1`; `GET /v1/spirits/butler` ⇒ `404`; the TL gains `spirit.revoked`; `maosctl revocations list`
  lists `hex(CrlId)`. The test never writes `/tmp/maos/crl` (the daemon's poller reads it host-wide, §11 row 5).
- **root C** (butler, fresh): `maosctl uninstall butler` unloads the SCB first (`GET /v1/spirits/butler` ⇒ `404`)
  and preserves the `0/3/4/5` terminal contract.
- **root D** (researcher: `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j-researcher/survey-distill.json maos run spirits/researcher/manifest.toml`;
  butler mints no token in replay — Trap 17): bounded poll (≤ 5 s) of the TL for the researcher's `cap.issue` row;
  `maosctl revoke-token <id>` with that id in lowercase hex (Trap 18) ⇒ exit 0 and exactly one new `cap.revoke` row
  (its `spirit_pid` is 0, `RevokeReason::Operator`); a second revoke ⇒ exit 1 typed `Revoked`, **no** new row
  (D-16-1-V); a never-issued id ⇒ exit 1 typed `UnknownToken`.
- **root E** (butler replay, `MAOS_IDLE_FAST=1`): after the first `belief_variance` row, `maosctl spirit upgrade butler
  --to <0.3.1 fixture>` (hot-swap, the default) ⇒ report `successor_version "0.3.1"`, `outcome "completed"` —
  **not** exit 101; `GET /v1/spirits/butler` ⇒ `Running` at the **same pid**; a bounded wait ⇒ a NEW
  `belief_variance` row timestamped after the swap. **Falsifier:** a bare `Butler::new()` successor reds that step
  (run, recorded, reverted). `--to` the 0.3.0 manifest ⇒ typed `409 VersionNotIncreasing`, no swap; `--policy
  cold-swap` ⇒ typed `409 ColdSwapUnsupported` (D-16-1-W). The fixture is butler's manifest with `version = "0.3.1"`
  plus empty `[scheduling]` and `[lifecycle]`, generated at test time.
- in-process (D-16-1-R): N concurrent pause/resume/unload commands on one Spirit id ⇒ each journal row pairs with
  exactly one approval-log row, in one consistent order;
- in-process: a halt inserted by `invoke_halt` into the port's `HaltRegistry` resolves through the port
  and writes the resolution row. **`insert_pending_with_metadata` has zero production callers in
  `crates/maos-bin/src`** (the fabricated halt, `main.rs:5765-5784`, deleted, not moved). The end-to-end
  J0 halt is 16-2.
- `crates/maos-capability/tests/` (kloc-free): a second `revoke` of one id ⇒ `Err(CapError::Revoked)` with one audit
  event in total; `revocation_verify_denial.rs` stays green.

**AC5 — daemon down, stale home, wrong bearer: typed, never journal-and-succeed (D-16-1-P).**
For every DOOR-class verb, each proven red with its exit code:
- no endpoint configured (neither the env pair nor `control.json`) ⇒ `NotInitialized`, 78, naming `maos init`;
- `control.json` present, nothing listening, lock free ⇒ `DaemonNotRunning`, 69;
- `control.json` present, nothing listening, home lock held (a root on an env-override endpoint) ⇒
  `StoreInUse`, 69;
- wrong bearer ⇒ 401 ⇒ **77**;
and in every case **no Lifecycle Journal or TL row is written**.
For the four DURABLE verbs: live daemon ⇒ door; no endpoint configured ⇒ offline child directly (erasure never
requires `maos init`); no daemon and the child acquires the whole lock set ⇒ offline, with the CHILD's stderr line
`maos: no daemon holds <paths>; running offline` (printed after acquiring, never by maosctl) and the HEAD exit
contract; any lock held and connect refused ⇒ `StoreInUse`, 69, **never** offline; connected but no HTTP response ⇒
`DoorUnresponsive`, 69, **never** offline; wrong bearer on a live door ⇒ 77, **never** offline. **Split-home vector (§15 F-C):** a root with `MAOS_HOME=/a` and an offline `forget` with
`MAOS_HOME=/b` sharing one memory root ⇒ the offline child exits 69 `StoreInUse` and writes nothing; the
shared-log vector (AC2) likewise. **Held-exclusive vector:** the test itself holds `LOCK_EX` on one store directory for 2 s (beyond
the 500 ms retry) ⇒ an offline `forget` exits 69 `StoreInUse` (never a daemon claim) — deterministic, unlike two
racing children. **NFS / permission:** a store directory that cannot be locked exclusively ⇒ 78 `LockUnavailable`.
`erasure_uninstall_13_5b.rs`'s `uninstall` one-shots (`:69`) and
`legal-hold-release` (`:782`) stay green unchanged; its `legal-hold-list` one-shot (`:767`) names a
deleted mode and moves to a read-only TL query (AC7).

**AC6 — the one-shot surface shrinks, and the frozen table is re-measured.**
Deleted from `main.rs`: **12 `if` blocks** — `pause`/`resume` (one arm), `posture-shift`, `halt-list`,
`halt-resolve`, `orchestrator-queue`, `orchestrator-status`, `revoke-token`, `revocations-import`,
`revocations-list`, `hot-swap-precheck`, `spirit-upgrade`, `legal-hold-list` — and the shared lifecycle
arm (`:5345-5408`) reduced to `uninstall`. `verbs::MAOS_ONE_SHOT_MODES` (`verbs.rs:146`) **48 → 32**;
`one_shot_mode_table_is_complete_and_frozen` (`crates/maos-bin/tests/verb_table_15_3.rs:192-198`)
updated to 32, the count re-measured from the one-shot block. Surviving: `hello-spirit`, `acp-server`,
`registry-server`, `bench-section-13-1`, `collective-erase`, `cohort-a2a-daemon`, `forget`,
`governance-admit`, `legal-hold-release`, `uninstall`, and the 22 `smoke-*` modes. maosctl spawns
`MAOS_ONE_SHOT` only at the five surviving sites (§10). `SCANNED_SOURCE_FILES` 19 → 20.
`check-exit-commands` verdict unchanged.

**AC7 — the rewritten tests read the tree, not what they set (E15-A6).**
- `crates/maos-cli/tests/`: `pause_resume_test`, `halt_resolve_test`, `orchestrator_queue_test`,
  `revoke_token_test`, `revocations_import_test`, `spirit_upgrade_test`, `posture_shift_test` become
  **contract tests** against a hand-rolled fixture door: exact method, path, bearer and JSON body sent,
  **and** the exit code / stderr each typed response maps to (D-16-1-P). The two vacuous tests are
  replaced, not ported.
- `legal_hold_cli_13_5b.rs` and `uninstall_exit_codes_13_5b.rs` keep proving the offline arm with
  `HOME`/`MAOS_HOME` isolated; `legal_hold_cli_13_5b`'s `list` half is re-pointed at a seeded TL (the
  reader no longer spawns). `erasure_uninstall_13_5b.rs:767` re-pointed at a read-only TL query.
- `accessibility_test.rs`: the `start`/`unload`/`run` cascades keep passing; the `stop` cascade
  (`:178`) asserts **exit 2** and zero ANSI bytes on the refusal (D-16-1-I), and door **error** output is
  ANSI-free.
- `tests/integration/maosctl_smoke.sh` and `v01_evaluator_path.sh`: `run hello-spirit` unchanged; the
  `start`/`stop`/`unload hello-spirit` journal assertions and the `start unknown-spirit` ⇒ 2 negative
  (`maosctl_smoke.sh:127-142`) are replaced — `start`/`unload` with no daemon ⇒ 78/69 and journal byte
  count unchanged; `stop` ⇒ 2 — or moved to `one_daemon_one_door_16_1`, **decided at T0 by measuring
  each step's job budget, never by deleting an assertion to go green**. `halt_resolve_smoke.sh`,
  `orchestrator_queue_smoke.sh`, `maosctl_hot_swap_precheck.sh` (no CI job runs them) are deleted, the
  File List naming each replacement.
- `one_shot_hello.rs` and the `cohort-a2a-daemon` hits in `cohort_daemon_smoke_13_5c.rs` unchanged.
- D-16-1-Q's two controls: the CI decoy step RED with one test's `HOME` removed, GREEN restored; the file-granular
  `root_spawn_home_isolation_16_1.rs` RED on the files with no isolation literal, GREEN after (transcripts in the Debug
  Log).

**AC8 — the tracker, the spec and the README say what the tree says.**
Same commit: epic 16 Round-5 edits (applied at story creation; T11 re-verifies via `git diff`); ADR-062
amendment (§12 row 17); `env_contract.rs` operator rows re-worded (no 8787; `control.json` named);
`README.md:227-236` states the door, `control.json`, the overrides, the offline durable arm and the exit
codes; `deferred-work.md` `:869` closed and `:550`'s door half closed; §11 rows 1, 2, 7 filed with
owners; epic-19 `:66` and epic-21 `:70` cites re-pointed by symbol (T11). `README.md` and the ADR-062
amendment state D-16-1-S's residual. **Round-table spec edits (§12 rows 18–27) and validation-round-2, consistency-check and fourth-read edits (§12 rows 28–38) were applied before
dev** — 16-6 row + section, FR9/FR51 *Closes*, 17-1 AC1/AC3 clause, 16-5 AC5, 16-4 and 16-2 notes,
index count, dependency DAG, sprint rows — T11 re-verifies them by diff.

---

## Review obligations (§A6, non-degradable)

**(a)** run `maos init` with `HOME` pointing at a directory without `.maos` ⇒ exit 0; revert the fix ⇒
the NEW init test reds. **(b)** against a live `maos run butler`, `maosctl pause butler`, then read
`GET /v1/spirits/butler`, the Lifecycle Journal and the TL — all three moved; delete the handler's
`scheduler.pause` ⇒ the door test reds. **(c)** export `MAOS_OPERATOR_HTTP_BIND=<the live root's endpoint>` and
`MAOS_OPERATOR_BEARER_TOKEN`, run `MAOS_ONE_SHOT=smoke-epic-4 maos` beside that root ⇒ no `Address already in use`. **(d)** POST with a
body line `Authorization: Bearer <real-token>` and a wrong header token ⇒ 401. **(e)** hold 8 idle
connections ⇒ a 9th request completes; hold 16 ⇒ the 17th gets 503; a panicking handler ⇒ 500 and the
next request succeeds. **(f)** with a live root, `maosctl forget --principal x` goes through the DOOR
(daemon journal row); stop the root ⇒ the same command runs OFFLINE and says so; hold `flock -s <store dir> sleep 60`
on any directory in the set **as the same user** ⇒ the offline child refuses (69 `StoreInUse`) after its 500 ms retry
rather than races; confirm the lock set created no file in any store directory. **(g)** `MAOS_ONE_SHOT_MODES` shrank by exactly D-16-1-A's door+reader modes and the
frozen test was re-measured, not loosened. **(h)** no maos-cli `[dev-dependencies]` gained
`maos-control`. **(i)** `grep insert_pending_with_metadata crates/maos-bin/src` ⇒ zero. **(j)**
`check-kernel-baseline` PASSED with `changed == 0`; `crates/maos-kernel-core/src/orchestrator/mod.rs:17`
untouched. **(k)** with a real `~/.maos/control.json` present on the reviewer's machine,
`cargo test -p maos-bin --no-fail-fast` shows no `EndpointInUse` (D-16-1-Q). **(l)** run the split-home
vector by hand: root on `MAOS_HOME=/a`, `maosctl forget` on `MAOS_HOME=/b` with one memory root ⇒ refused
69; then delete that store's entry from the lock set and confirm the vector reds; repeat with the shared-log vector. **(m)** remove `HOME` from one
fixed root-spawning test and confirm the CI decoy step reds it with `EndpointInUse`; confirm the file-granular lint
reds a file with no isolation at all. **(n)** force a route budget expiry and confirm the 503 `operation_id` appears in exactly one TL row found by
`maosctl audit query --intent-contains <id>` and in maosctl's exit-75 message. **(o)** `kill -9` a root while a child
process it spawned is still alive ⇒ an immediate offline `forget` is not refused (no inherited lock). **(p)** revoke a
token twice over the door ⇒ the second is typed `Revoked` and adds no `cap.revoke` row. **(q)** upgrade butler to
the 0.3.1 fixture, then to 0.3.0 ⇒ refused typed; replace the faithful successor with a bare `Butler::new()` ⇒ the
post-swap halt vector reds.

---

## 11. Declared cut lines (rule 10 — each names an owner, never a bucket)

| # | Item | Measured evidence | Destination |
|---|---|---|---|
| 1 | **Lifecycle Journal cross-process overwrite** | `JournalAdapter::open` `.write(true)`, no `.append(true)` (`crates/maos-kernel-core/src/journal/mod.rs:98-102`); `append_transition` writes at the handle's cursor (`:178-181`); replayed: a one-shot's line overwritten, torn fragment left; the comment at `main.rs:2863-2864` assumes atomic append | **`16-5`** — kernel-core byte ⇒ content-hash re-pin ⇒ FLAG-Winston; 16-5 owns the epic's only re-pin and D3's audit-durability charter. **Written into epic-16 16-5 AC5 at the round-table.** 16-1 removes the cross-process writers for every verb it re-targets. |
| 2 | **Home split-brain beyond `control.json`** | memory root and archives ignore `MAOS_HOME` (`maos-audit/src/lib.rs:1456-1470,1555-1575`) | **`16-4-maos-uninstall-and-keyring`** — its AC1 already enumerates `MAOS_HOME` and the XDG data root; **note written into its AC2 at the round-table**. 16-1's store lock set (D-16-1-D) holds the race closed in the meantime; **16-4's `maos purge` deletes those directories and must take the same set exclusively** (note written into its AC2, §17 V-21). |
| 3 | ~~SIGTERM drain takes 10 s~~ | `audit_tx` clones held by scheduler/hot-swap/revocation/upgrade never dropped on the SIGTERM path; writer times out at 10 s (`main.rs:8352-8390`) | **ADOPTED, rule 10(a)** — AC1 step 5 |
| 4 | ~~`cohort-a2a-daemon` ignores SIGTERM~~ | serves until `ctrl_c` only (`main.rs:10113`) | **ADOPTED, rule 10(a)** — a door root must release its lock and listener on SIGTERM |
| 5 | **`/tmp/maos/crl` host-global CRL dir** | `main.rs:2898` | already filed by 15-3 to **`20-3a-gate-honesty`** — cited, not re-filed |
| 6 | **FR9 `load` via the control plane** | no verb in `crates/maos-cli/src/cli.rs`; only `maos run` loads | **`16-6-maosctl-load-over-the-door`** — created by the round-table (§15 F-B) |
| 7 | **`resolve_spirit_name`'s `hello-spirit → pid 0` wildcard** matches every pid-0 kernel row | `maos-audit/src/lib.rs:1624-1665` | **`16-2`** — it loads hello-spirit through the scheduler with a real pid; **note written into its AC2 at the round-table** |
| 8 | **Workers can reach the door** (env inherited, same uid reads `control.json`) | `runtime.rs:461-469` no `env_clear`; `credential_posture_2c.rs:299-310` forbids one | **`17-1`** — its own AC3 `env_clear` + AC1/AC4 T3 isolation; **the operator bearer and `control.json` written into 17-1 AC1/AC3 by name at the round-table** (D-16-1-S) |
| 9 | **FR51(a)(d) bounded guarantees** | `pause` flips state; in-flight hooks keep running | **Epic-16 retrospective** — rule 9's refinement vehicle; named in the epic *Closes* line (D-16-1-U) |
| 10 | **FR51(c) recall of pending Orchestrator-buffered actions** | `recall_all_pending` (`buffer.rs:72`) has no operator route; resume re-enqueues (`scheduler_loop.rs:452-460`) | **`19-2` AC3** — the FR20 owner; written into epic 19 (§17 V-12) |
| 11 | **Cold-swap upgrade starts an unadmitted successor under a new pid** | `UpgradePolicy::ColdSwap` (`upgrade.rs:162`): new pid, no `admit_spirit` (spike); maos-bin `pid_by_spirit_id` (`main.rs:2428`) not refreshed (code) | the admission half is a kernel byte ⇒ **Epic-16 retrospective** row (no story's charter matches); the door refuses cold-swap typed (D-16-1-W) |

**Not cut (EFFORT, not SCOPE):** the init fix (§1), the resolver fix (§2), bind relocation and root
gating (§3), the real handlers (§4), the server hardening (§8), the two SIGTERM fixes, the shared-HOME
test isolation (D-16-1-Q). Each is required for one of this story's own ACs to be true.

---

## Dev notes

**Reuse, do not reinvent.**
- **Kernel APIs (all `pub` at HEAD):** `SpiritSchedulerAdapter::{start,pause,resume,unload,resolve_pid,scbs}`
  (`crates/maos-kernel-core/src/scheduler/scheduler_loop.rs:374,401,428,472,178,148`);
  `SpiritControlBlock::current_state` (`control_block.rs:395`), transitions `is_transition_allowed`
  (`:444-455`); `PolicyTable::{inner,shift_posture,evaluate_with_posture}` (`capability/cap_policy/mod.rs:86,233,278`);
  `CapabilityRegistryAdapter::revoke`; `OrchestratorBufferRegistry::{get,get_or_create}`;
  `RevocationApplier::{apply_crl,list_applied}`; `HaltRegistry` + `KernelHaltResolver`.
- **Port shape** — `SandboxReportSource` (`crates/maos-control/src/lib.rs:19`), `RotationWindowSource`
  (`:77`), `CohortConvergenceSource` (`:131`), `CohortSelfIdentitySource` (`:163`); maos-bin impls
  `crates/maos-bin/src/cert_rotation.rs:74,120,160`; `None` ⇒ 404, `Unhealthy` ⇒ 503.
- **Constant-time bearer** — keep `ring::constant_time::verify_slices_are_equal` (`lib.rs:16,289`).
- **Sync→async with panic capture** — `crates/maos-loom-lite/src/adapter.rs:60-78`.
- **Client** — generalise `fetch_live_sandbox_report` (`crates/maos-cli/src/subcommands.rs:1955-2005`);
  do not add an HTTP crate. Client read deadline = route budget + 5 s.
- **Handler bodies** — the deleted arms are the reference for the journal/approval-log calls
  (`journal_director_lifecycle_action`, `journal_posture_shift`, `journal_orchestrator_queue`,
  `journal_token_revocation`, `KernelHaltResolver` + `HaltFlow::submit_resolution` `main.rs:5811-5829`,
  `migration_plan::upgrade_with_plan_guard`, `enforce_vetted_upgrade_precondition`,
  `run_uninstall_cascade`). Move the calls; drop pid 0 / `hello-spirit` / CWD manifests and the per-arm
  `drop(..)` + drain choreography (the daemon owns drain at shutdown).
- **Lock** — `rustix::fs::flock(fd, FlockOperation::{NonBlockingLockShared, NonBlockingLockExclusive})`.
- **Atomic `0600` write** — `OpenOptions` + `std::os::unix::fs::OpenOptionsExt::mode(0o600)`, `sync_all`,
  `rename` within the same directory.

**Files being modified — current state, change, must-not-break:**

| file | today | this story | must not break |
|---|---|---|---|
| `crates/maos-control/src/lib.rs` | 986 lines / 803 tokei (351 prod); serial accept `:224-243`; `handle_connection` `:265`; GET routes `:299,347,392,428`; READ-ONLY rustdoc `:66-77` **and** `:121-124`; 11 inline tests | POST, body, 405, pool, deadline, `catch_unwind`, `OperatorCommandPort`, `bind_with_commands`, two GET routes; rustdoc notes superseded (do not carry the dead `main.rs:9844-9853` pointer, ADR-062 `:79-80`) | the four GET routes' payloads and request-line literals (ADR gate); `bind`'s signature; 401-before-route |
| `crates/maos-bin/src/main.rs` | 14,347 lines; env config `:2705-2728`; bind `:2827-2846`; one-shot block `:5159-8286` | bind moved + root-gated + `control.json` config; store lock set before `:1793`; `BinPrivateOps` impl; 12 blocks deleted + lifecycle arm reduced; SIGTERM fixes | marker `:2840`; `Arc::get_mut` ordering `:2766`; surviving arms; `verbs::resolve_one_shot_mode` fail-closed dispatch |
| `crates/maos-bin/src/lib.rs` | `pub mod` list | `pub mod operator_door;` | — |
| `crates/maos-bin/src/verbs.rs` | `MAOS_ONE_SHOT_MODES` 48 at `:146` | 32 | sorted, unique, rendered by `print_help` |
| `crates/maos-bin/src/env_contract.rs` | operator rows `:474-484` | re-worded | the names (15-1 AC5) |
| `crates/maos-cli/src/subcommands.rs` | 18 spawn sites (§10); `resolve_spirit_pid` `:2191`; `fetch_live_sandbox_report` `:1955` | door client, readers, offline dispatch, `stop` refusal | ⚠ text-anchor gates: `xtask/src/check_j1_two_host_signed_run.rs:215-229` and `xtask/tests/j1_crosshost_2c_proven_red.rs:505-506` count `let region_home = match resolve_region_home()` / `derive_region_pubkey(&seed, r)` (sealed-export); `xtask/src/check_epic_6_bridge.rs:3132` looks for `fn dispatch_import`. Do not move or rename them. |
| `crates/maos-shell/src/lib.rs` | `run_init` `:46`; `maos_home` `:304` | ENOENT fix; mint via `maos-domain`; delegate home | idempotent `already initialized` output |
| `crates/maos-audit/src/lib.rs` | `resolve_spirit_name` `:1615` | `lifecycle.load` rows; boot order by timestamp | the `hello-spirit` wildcard; read-only open flags; the store resolvers (`:872,1416,1455,1583`) are CALLED by the lock set, not changed |
| `crates/maos-capability/src/cap_tokens/{shard,mod}.rs` | `set_revoked` `shard.rs:44-52` stores `true` and returns `Ok(true)` for any present id | `swap` + map to `CapError::Revoked` (D-16-1-V) | `revoke_for_spirit`; `verify ⇒ Revoked`; `worker_spawn.rs:730` discards the result |
| `.github/workflows/discipline.yml` | `workspace-test-suite` test step `:3534` | the D-16-1-Q decoy in that step; the `one_daemon_one_door_16_1` step (T8) | `fetch-depth: 0` and the pinned tokei added by `f72b557b` |
| `docs/adr/ADR-062-mutating-operator-surface.md` | ACCEPTED 2026-09-08 | append `## Amendment — Story 16-1` | `Gate:` header; the four GET literals |
| `xtask/tests/decision_adrs_and_provisioning.rs` | `"062"` arm `:310-339` | re-anchor 405 | other ADRs' arms |

**Traps, in the order they will bite:**
1. **Every `maos` child binds the door today.** Fix the root gate (D-16-1-B) first, or token-exporting
   test environments fail mysteriously.
2. `monotonic_now_ns()` `debug_assert!`s before `init_monotonic_base()` — tests run debug; init before the bind.
3. `Arc::get_mut(&mut scheduler)` (`main.rs:2766`) needs strong count 1 — take the port's scheduler
   `Arc` after it.
4. `pause` on a Paused SCB is `InvalidStateTransition` (`control_block.rs:444-455`), and the state is
   committed before a failing `on_pause` hook returns (`scheduler_loop.rs:403-422`) — map honestly
   (409 / 500), never retry.
5. All four watchdogs skip non-Running SCBs (`idle_watchdog.rs:73`, `schedule_watchdog.rs:166`,
   `progress_watchdog.rs:67`, `silent_failure_detector.rs:67`) — that is what makes pause observable.
6. The door is connect-ready before `scheduler.start` (`main.rs:5006`) — readiness is `Running`, not accept.
7. `cargo tree -p maos-cli` includes dev edges — no `maos-control` in its `[dev-dependencies]`.
8. Offline durable tests and every root-spawning test must isolate `HOME` and `XDG_DATA_HOME` (D-16-1-Q).
9. The TL adapter panics on any non-duplicate write error (`maos-iac` `transparency_log.rs:866`) —
   maosctl readers open **read-only**.
10. A new `src/*.rs` in maos-bin reds `cohort_daemon_smoke_13_5c.rs:954-958` until `SCANNED_SOURCE_FILES` is 20.
11. `kloc-check` charges inline `#[cfg(test)]` — new tests go in `tests/`.
12. `rustix` is unix-gated in `maos-kernel-core/Cargo.toml:65`; gate it the same way.
13. `crates/maos-kernel-core/src/orchestrator/mod.rs:17` names a deleted mode in a doc comment — **leave
    it**; a kernel byte reds the content-hash pin.
14. CRL import revokes butler's tokens and emits halt receipts — never run it on the root later AC4 steps use.
15. The epic's own `main.rs` anchors were wrong at authoring (see `baseline_commit`). Cite symbols.
16. **The door keeps the audit channel open, so SIGTERM still waits 10 s unless the server goes first.** Measured at
    `f72b557b`: `_operator_http_server` (`main.rs:2827`) lives to the end of `main` and already holds
    `Arc::clone(&scheduler)` (`:2831`); the scheduler holds `Arc::clone(&capability)` (`:2683`), and the capability
    adapter holds an `audit_tx` clone (`:1917-1922`) — so the drain's `drop(capability)` (`:8378`) closes nothing and the writer
    times out (`:8385`). This story's port adds more holders (scheduler, `PolicyTable`, revocation applier, hot-swap,
    upgrade orchestrator, `HaltRegistry`), and every spawned handler task (D-16-1-E, never cancelled) holds a port
    clone. **Shutdown order for the `maos run` root:** stop flag → join the accept thread and every pool worker (drop
    the server) → await in-flight handler tasks, each bounded by its remaining route budget (not cancelled) → drop
    the port → release the lock set → the existing `drop(audit_tx)…` sequence. AC1 step 5's 3 s is measured with no
    command in flight. The `maos shell` root returns at `:3401` (`return shell_result`) without that drain: apply the
    same order before the return — join the server, await in-flight handler tasks within their budgets — so no
    `operation_id` already handed out lacks its completion row; never `unwrap` a `JoinError`.
17. **Butler mints no capability token in replay** (spike: 12/42/50 s runs, `capability_token` NULL in every row). In
    `maos run`, tokens are minted at researcher admission with an explicit inference mode (`main.rs:4979-4995`) and by
    the `--live` MCP ports. Token vectors use the researcher root (AC4 root D).
18. **SQLite `hex()` is uppercase; `parse_token_id_hex` (`main.rs:8917`) and maosctl accept lowercase only** — select
    `lower(hex(substr(capability_token,1,16)))`, or the first 32 chars from `maosctl audit query --format json`.
19. **Butler's `belief_variance` halt is pending from ≈30 s** (default `idle_window_ms` 30000; `MAOS_IDLE_FAST=1` ≈300 ms)
    and its TL row has `spirit_pid` 0 (`BUTLER_SPIRIT_PID`, `spirits/butler/src/lib.rs:122`) — a "no pending halt" step
    uses a never-issued id, and no "real pid" assertion may target that row.
20. **The lock set is taken before `main.rs:1793`** — not at the bind, not at the one-shot dispatch (D-16-1-D).
21. **`maos audit query` silently ignores unknown flags** (`main.rs:1714-1739`) — the `operation_id` lookup is
    `maosctl audit query --intent-contains`.
22. **A bare `maos` is the shell root** (`main.rs:1694-1699`) — root gating and the CI decoy both treat it as one.
23. **Do not port the one-shot's per-call env reads** — the trust anchor (`MAOS_CRL_TRUST_ANCHOR_PUB_HEX`) and the
    upgrade attestation/keyring (`MAOS_UPGRADE_TO_ATTESTATION`, `MAOS_VETTER_KEYRING`) — into the daemon (D-16-1-W/X).
24. **The proofs dir is created lazily today** (`maos-audit/src/erasure/proof.rs:450`); the lock set creates it early,
    and a store path that exists as a non-directory locks its parent (`erasure_uninstall_13_5b.rs:627-637` plants one).

### Project Structure Notes

New files: `crates/maos-bin/src/operator_door.rs` (lib; doorbell 19 → 20),
`crates/maos-bin/tests/one_daemon_one_door_16_1.rs`, `crates/maos-bin/tests/operator_door_16_1.rs`,
`crates/maos-control/tests/post_surface_16_1.rs`, `crates/maos-bin/tests/root_spawn_home_isolation_16_1.rs`, a revoke test
under `crates/maos-capability/tests/`, one new module file under `crates/maos-domain/src/`.
No new crate (`check-workspace-count` 55). Dependencies: `rustix` (unix-gated) to `maos-bin` and
`maos-cli` only. Kernel-core `src` read, never written.

### References

- [Source: `_bmad-output/planning-artifacts/epics/epic-16-one-daemon-one-door-j0-w1.md` — 16-1 section (HEAD `:118-127`), exit block `:17-25`, provenance `:28-34`, Kernel-Δ `:41`, Kloc asks `:43`, Dependencies `:167`; add 2 to every line after this commit's Round-5 header]
- [Source: `docs/adr/ADR-062-mutating-operator-surface.md` — Decision `:46-80`, Consumers `:82-89`, Consequences `:105-113`]
- [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md` — FR9 `:37`, FR13 `:41`, FR16 `:49`, FR20 `:53`, FR51 `:54`, FR65 `:107`]
- [Source: `_bmad-output/implementation-artifacts/deferred-work.md` — `:550`, `:869`]
- [Source: `_bmad-output/implementation-artifacts/16-0-kernel-pin-content-hash.md` — the pin this story keeps green; the rule-9 verification lesson]
- [Source: `_bmad-output/implementation-artifacts/15-3-single-phase-source-and-exit-command-check.md` — `verbs.rs`, `MAOS_ONE_SHOT_MODES`, F13 doorbell]
- [Source: `xtask/kloc.toml` — `:386,:395,:454,:459,:475,:642,:655-656` (at `f72b557b`)]

---

## Tasks / Subtasks

- [x] **T0 — re-measure; trust nothing in this file** (all ACs)
  - [x] `git status --short` clean; record HEAD; re-run the baseline gate sweep; re-read the six `kloc.toml` rows.
  - [x] Re-run §1's init probe and §2's butler probe against fresh binaries; record output.
  - [x] Re-measure the 48 in `one_shot_mode_table_is_complete_and_frozen` against `main.rs`'s one-shot block.
  - [x] Derive D-16-1-Q's shared-HOME offenders from the decoy run (below), not from grep; expect more than nine.
  - [x] Time each step of `maosctl_smoke.sh` / `v01_evaluator_path.sh`; decide AC7's disposition; record it.
  - [x] Run the CI decoy locally (scratch `HOME`, `maos init`, a listener on its endpoint, `cargo test -p maos-bin -p maos-cli --no-fail-fast`); record the offender list for D-16-1-Q.
  - [x] Confirm the lock set creates no file: grep test helpers and `read_dir`/`remove_dir_all` over the store directories (incl. `erasure_uninstall_13_5b.rs` counts); grep for any `!proof_dir.exists()`-style assertion the early proofs-dir creation would break.
  - [x] Re-run §17's spike probes: researcher replay writes `cap.issue` within 5 s; butler's halt timing; the double-revoke `Ok`; the missing butler successor loader.
- [x] **T1 — `maos init` ENOENT, red first** (AC2)
  - [x] NEW test, home does NOT exist ⇒ exit 0. RED at HEAD → fix `run_init` ordering → GREEN; four existing init tests green.
- [x] **T2 — home rule, `control.json`** (AC2, D-16-1-C)
  - [x] `maos-domain` module: `maos_home()`, `ControlFile{version, endpoint(tcp://), token}`, version/scheme/mode/owner validation, atomic `0600` writer, minter (existing `getrandom`), no lock-file name (the set locks directory handles).
  - [x] `run_init` mints (incl. mint-when-absent); byte-stable on re-run; `maos_shell::maos_home` delegates.
- [x] **T3 — root gating, bind relocation, lock** (AC2, D-16-1-B/D)
  - [x] Config from env (both-or-neither) else `control.json`; retire 8787; no config ⇒ stderr notice.
  - [x] Bind only in the three roots (bare `maos` included), after `:3087`, clock base first.
  - [x] Store lock set (D-16-1-D): resolved by the existing resolvers, directories created (non-directory ⇒ parent), canonicalised, deduplicated, sorted, opened read-only (`O_CLOEXEC`), all-or-nothing; taken before `main.rs:1793`; dirs created `0700`; dedup by `(dev, ino)`; `fstat`=`stat` re-check after locking; non-`EWOULDBLOCK` ⇒ `LockUnavailable` 78; roots `LOCK_SH|LOCK_NB` and the child `LOCK_EX|LOCK_NB`, both with 500 ms retry.
  - [x] Proofs: BIND+token exported + live root + `MAOS_ONE_SHOT=smoke-epic-4` ⇒ no `Address already in use`; `EndpointInUse` ⇒ 78; `cert_rotation_trigger_14_2a` green; shared-log and CLOEXEC vectors.
  - [x] D-16-1-Q: add the CI decoy to the `workspace-test-suite` test step and write the file-granular `root_spawn_home_isolation_16_1.rs`; capture BOTH red first; then per-test `HOME` in every offender; both GREEN.
- [x] **T4 — the server** (AC3, D-16-1-E/F/G/H)
  - [x] Header/body split; `Content-Length`; 64 KiB; 400/405/409/411/413/500 phrases; fixed literals.
  - [x] Pool cap 16 + `503 Busy`; 5 s read deadline; route budgets with spawned, uncancelled futures; `submit` ⇒ `OperatorSubmission{operation_id, state, completion}` with the `Queued/Started/Withdrawn` CAS; `HandlerStillRunning{operation_id}`; `SpiritBusy`; `catch_unwind`; accept-error continue.
  - [x] `OperatorCommandPort` + typed enums; `bind_with_commands`; `bind` delegates with `None`.
  - [x] `GET /v1/spirits/{id}` (after `/sandbox`), `GET /v1/daemon`; id validation.
  - [x] Convert `lib.rs:607,786,927` to 405 guards; `/v1/cohort/`+`/v1/a2a/` no-POST negative.
  - [x] `crates/maos-control/tests/post_surface_16_1.rs` — every AC3 bullet with its falsifier.
  - [x] Re-anchor `decision_adrs_and_provisioning.rs:330-338` and add the ADR-amendment `require_contains`; run green.
- [x] **T5 — the handlers** (AC1, AC4, D-16-1-F/J/K/L/N)
  - [x] `crates/maos-bin/src/operator_door.rs` (lib) + `BinPrivateOps` impl in `main.rs`; kernel transition first, rows on success; typed outcomes with preserved terminal codes.
  - [x] Doorbell 19 → 20.
  - [x] Delete the 12 blocks + reduce the lifecycle arm; zero production `insert_pending_with_metadata`; modes 48 → 32; update `verb_table_15_3.rs`.
  - [x] Per-Spirit serialization of mutating commands, waiting inside the route budget ⇒ `SpiritBusy` (D-16-1-R).
  - [x] Completion TL row per mutating command (`insert_frame_event`, `intent` carries the `operation_id`).
  - [x] D-16-1-V in `maos-capability` + its test in `crates/maos-capability/tests/`.
  - [x] D-16-1-W: extract the butler construction helper; butler successor arm; `VersionNotIncreasing`; `ColdSwapUnsupported`; attestation/keyring as parameters.
  - [x] D-16-1-X: capture the trust anchor at boot; CRL bytes in the body; hex `CrlId` in `list`.
  - [x] `crates/maos-bin/tests/operator_door_16_1.rs`: posture ⇒ `evaluate_with_posture` changes; `invoke_halt` halt resolves; concurrency vector.
- [x] **T6 — the client** (AC2, AC5, D-16-1-A/D/I/P)
  - [x] Discovery + mode/owner/loopback checks; typed errors; the D-16-1-P exit table.
  - [x] Delete door-verb TL preflights; `stop` refusal before the smoke short-circuit.
  - [x] Durable verbs: no endpoint configured (env pair or `control.json`) ⇒ offline child; door → else offline child (child takes `LOCK_EX|LOCK_NB` on the whole set with 500 ms retry, prints the running-offline line after acquiring, refuses 69 `StoreInUse`); connected-without-response ⇒ `DoorUnresponsive` 69; `OfflineUnsupported` on non-unix.
  - [x] Door verbs on connect-refused: momentary home-lock probe ⇒ `DaemonNotRunning` / `StoreInUse`.
  - [x] Exit 75 for every retryable 503; `HandlerStillRunning` names the id and `maosctl audit query --intent-contains <id>`; 400/405/411/413 ⇒ 1.
  - [x] CRL bytes and canonical upgrade paths sent by the client (D-16-1-W/X).
  - [x] Read-only in-process `halt list` / `legal-hold list`; fix `resolve_spirit_name` (D-16-1-K).
- [x] **T7 — SIGTERM** (AC1 step 5, §11 rows 3–4)
  - [x] Drop the held `audit_tx` clones on the SIGTERM path (exit ≤ 3 s, no drain-timeout line); SIGTERM in the `cohort-a2a-daemon` select.
  - [x] Trap 16 order: server (accept + pool joined) → in-flight handler tasks awaited within their budgets → port → locks → audit drain. `maos shell` applies the same order before `return shell_result`.
  - [x] Vector: a command in flight at SIGTERM completes and its row reaches the TL (no drain-timeout line); idle SIGTERM ≤ 3 s.
- [x] **T8 — `one_daemon_one_door_16_1`** (AC1, AC4, AC5)
  - [x] AC1 steps 1–5 + the `scheduler.pause`-removed falsifier (run, recorded, reverted); readiness = `Running`.
  - [x] AC4 roots A–E; AC5 negatives with exit codes, asserting no row each time; two-roots-one-home guard; split-home, shared-log and two-offline-children lock vectors.
  - [x] Trap 16 vector: a command in flight at SIGTERM completes and its completion row is in the TL.
  - [x] Named in its own `discipline.yml` step.
- [x] **T9 — the test rewrite** (AC7)
  - [x] Seven maos-cli contract tests against a hand-rolled fixture door; replace the two vacuous ones.
  - [x] Isolate `HOME`/`MAOS_HOME` in the two 13-5b maos-cli tests; re-point `legal_hold_cli_13_5b` list half and `erasure_uninstall_13_5b.rs:767`.
  - [x] `accessibility_test.rs` `stop` cascade ⇒ exit 2, zero ANSI.
  - [x] `maosctl_smoke.sh` / `v01_evaluator_path.sh` per T0; delete the three non-CI scripts naming each replacement.
- [x] **T10 — ADR, README, env contract, deferred-work** (AC8)
  - [x] ADR-062 amendment (§12 row 17, incl. (h) the D-16-1-S residual, (i) the `control.json` version/scheme, (j) the `operation_id` lookup, (k) hot-swap-only/forward-only upgrade); `env_contract.rs`; `README.md:227-236` incl. the residual sentence.
  - [x] `deferred-work.md`: close `:869` and `:550` (door half); file §11 rows 1, 2, 7.
- [x] **T11 — sweep**
  - [x] `git diff` the epic: every §12 OLD gone, every R5 note present, exit block byte-unchanged; epic-17 17-1 AC1/AC3 clause, index `7 stories`, DAG `16-1 → 16-2, 16-6, 19-2`, sprint row `16-6` present.
  - [x] Re-cite by symbol the cites this commit invalidates in active epics: `epic-19…md:66` (`MAOS_ONE_SHOT=orchestrator-queue`, `main.rs:5667-5672`), `epic-21…md:70` (`maos-control/src/lib.rs:285-297`, `:299`); grep epics 17–21 for any further `maos-control/src/lib.rs:` or one-shot `main.rs:` cite.
  - [x] `cargo fmt --all -- --check`; `kloc-check`; `check-kernel-baseline` (`changed == 0`); `check-env-contract`; `check-workspace-count`; `check-epic-close-coherence`; `check-exit-commands` (unchanged); `cargo test -p xtask --test decision_adrs_and_provisioning`; `cargo test --workspace --no-fail-fast`.
  - [x] Story row → **`review`**.

### Review Findings

- [x] [Review][Patch] HIGH: Retain store locks through store-writer shutdown and amend T7 ordering [crates/maos-bin/src/main.rs:7855]
- [x] [Review][Patch] HIGH: Preserve `forget --reason` across the live door [crates/maos-cli/src/subcommands.rs:1772]
- [x] [Review][Patch] HIGH: Keep `spirit upgrade --plan` plan-only [crates/maos-bin/src/operator_door.rs:663]
- [x] [Review][Patch] HIGH: Keep the completion channel alive for the `SpiritBusy` decision [crates/maos-bin/src/operator_door.rs:1224]
- [x] [Review][Patch] HIGH: Reject malformed or truncated successful HTTP responses [crates/maos-cli/src/door_client.rs:76]
- [x] [Review][Patch] HIGH: Enforce one absolute bounded client response deadline and size cap [crates/maos-cli/src/door_client.rs:193]
- [x] [Review][Patch] HIGH: Isolate successor-manifest staging across concurrent upgrades [crates/maos-bin/src/operator_door.rs:689]
- [x] [Review][Patch] HIGH: Track and await started command tasks during daemon shutdown [crates/maos-bin/src/operator_door.rs:1280]
- [x] [Review][Patch] HIGH: Verify a pending halt belongs to the requested Spirit [crates/maos-bin/src/operator_door.rs:801]
- [x] [Review][Patch] HIGH: Retain and wire a daemon-lifetime output-marker registry [crates/maos-bin/src/operator_door.rs:863]
- [x] [Review][Patch] HIGH: Preserve halt resolver error variants at the door [crates/maos-bin/src/operator_door.rs:877]
- [x] [Review][Patch] HIGH: Tear down the door before both `maos run --once` returns [crates/maos-bin/src/main.rs:4514]
- [x] [Review][Patch] HIGH: Canonicalize symlinked store targets before deriving lock directories [crates/maos-bin/src/operator_door.rs:1567]
- [x] [Review][Patch] HIGH: Publish `control.json` without replacing a concurrent initializer [crates/maos-domain/src/operator_door.rs:382]
- [x] [Review][Patch] HIGH: Replace the predictable fail-open effective-UID probe [crates/maos-domain/src/operator_door.rs:409]
- [x] [Review][Patch] HIGH: Compare upgrade versions with valid prerelease semantics [crates/maos-bin/src/operator_door.rs:1452]
- [x] [Review][Patch] HIGH: Reject wrong-typed upgrade options instead of changing semantics [crates/maos-control/src/lib.rs:1168]
- [x] [Review][Patch] HIGH: Reject governance versions outside the `u32` range [crates/maos-bin/src/operator_door.rs:1127]
- [x] [Review][Patch] HIGH: Require absolute MAOS home paths across processes [crates/maos-domain/src/operator_door.rs:54]
- [x] [Review][Patch] MEDIUM: Authenticate requests before exposing overload as `503` [crates/maos-control/src/lib.rs:565]
- [x] [Review][Patch] MEDIUM: Canonicalize upgrade attestation and keyring paths client-side [crates/maos-cli/src/subcommands.rs:2215]
- [x] [Review][Patch] MEDIUM: Deduplicate resolved Spirit incarnations after dropping timestamps [crates/maos-audit/src/lib.rs:1731]
- [x] [Review][Patch] MEDIUM: Let a lifecycle payload identity override legacy intent [crates/maos-audit/src/lib.rs:1715]
- [x] [Review][Patch] MEDIUM: Filter halt-list rows by resolved boot nonce and PID [crates/maos-cli/src/subcommands.rs:1911]
- [x] [Review][Patch] MEDIUM: Validate the halt body’s Spirit ID server-side [crates/maos-control/src/lib.rs:1001]
- [x] [Review][Patch] MEDIUM: Strictly validate HTTP request line, header cap, and body framing [crates/maos-control/src/lib.rs:724]
- [x] [Review][Patch] MEDIUM: Bound CRL reads before allocating the request body [crates/maos-cli/src/subcommands.rs:2083]
- [x] [Review][Patch] MEDIUM: Reject present non-UTF-8 operator override variables [crates/maos-cli/src/door_client.rs:119]
- [x] [Review][Patch] MEDIUM: Reject port zero in persisted door endpoints [crates/maos-domain/src/operator_door.rs:192]
- [x] [Review][Patch] MEDIUM: Map invalid daemon bind configuration to exit 78 [crates/maos-bin/src/main.rs:3412]
- [x] [Review][Patch] MEDIUM: Add the promised real-port serialization test that consumes `spirit_lock` [crates/maos-bin/src/operator_door.rs:249]

---

## 12. Epic and ADR OLD→NEW edits (rule 9)

⚠ **Rows 1–16 were APPLIED to `epics/epic-16-one-daemon-one-door-j0-w1.md` at story creation; rows 18–27 at the round-table; rows 28–36 by validation round 2, row 37 by the consistency check and row 38 by the fourth read (§17) — all re-verified: `check-exit-commands` PASS, `check-epic-close-coherence` PASS, YAML parses.** Each OLD
fragment was asserted to occur the stated number of times before replacement (rows 1–2 share one
fragment present twice; every other row exactly once), and each replaced number was checked to be a line
cite, not data. **The table is a history:** each row records OLD→NEW as applied at that time, and a later row supersedes an earlier one (e.g. rows 6/14's `daemon.lock` / `DaemonAliveWithoutDoor` are superseded by rows 30/32). Line numbers are **HEAD** numbering; the Round-5 header adds 2 lines above them. The NEW
column is a **summary** — the verbatim applied text is this commit's `git diff` of the epic, and every
applied NEW that changes a premise carries the marker `⚠ R5` (T11 verifies by diff, not by
string-matching this table). Row 17 (ADR-062) lands **with the code**, because
`decision_adrs_and_provisioning.rs` anchors ADR and source together.

| # | HEAD line | OLD (verbatim fragment) | NEW (summary) |
|---|---|---|---|
| 1 | `:56` Stories table | `maos-bin +300–500, maos-control +400–600, maos-cli +400–700, maos-shell ~+30` | re-booked: `maos-control ≤ +533, maos-bin ≤ +670 (net), maos-cli ≤ +377 (net), maos-shell +33…+59, maos-domain +78…+130, maos-audit +13…+33` (post-round-table figures) |
| 2 | `:120` 16-1 *Closes · Δ* | same fragment | same |
| 3 | `:122` AC1 | `The CI job for exit lines 5–6, \`cargo test -p maos-bin --test one_daemon_one_door_16_1\`, starts the root` | R5 prefix: the four verbs exit 2 inside maosctl before any spawn (TL preflight / `lifecycle_verb`), arms would refuse again in the child, `pause` never calls the scheduler ⇒ ACs assert the kernel transition; `stop` refused; `start` on Running ⇒ 409 |
| 4 | `:123` AC2 | `required iff \`MAOS_OPERATOR_HTTP_BIND\` \`:2687\`, bind executed` | enabled by the TOKEN (`:2705-2728`; `:2687` is `Arc::clone(&telemetry)`) |
| 5 | `:123` AC2 | `bind executed at \`:2792\`)` | `:2827`; R5: 8787 retired; bind moves after `:3087`; gated to the three door roots |
| 6 | `:123` AC2 | `the daemon records its \`pid\` and \`boot_nonce\` in \`control.json\` at bind so \`maosctl\` can tell "no daemon" from "stale file"` | R5 REPLACED: one writer (`maos init`); `daemon.lock` shared flock + `GET /v1/daemon` |
| 7 | `:123` AC2 | `(\`policy\` \`main.rs:1899\`, \`halt_registry\` \`:1985\`, \`orchestrator_registry\` \`:1994\`, \`scheduler\` \`:2644\`, \`crash_detector\` \`:2717\`` | `:2022`, `:2031`, `:2681`, `:2754` |
| 8 | `:123` AC2 | `so \`OperatorHttpServer::bind\` takes a \`tokio::runtime::Handle\` from the root and POST handlers \`block_on\` the scheduler calls;` | R5: SYNC port implemented in maos-bin holding the `Handle`; `bind` does NOT take one |
| 9 | `:123` AC2 | `are all in scope at the bind site \`:2792-2798\`, which already receives \`Arc::clone(&scheduler)\`.` | `:2827-2846`; R5: `operator_door.rs` + `bind_with_commands`; `bind`'s 18 test call sites + production `main.rs:2829` untouched |
| 10 | `:123` AC2 | `the two certificate-identity guards remain guards and the generic read-only assertion is narrowed, none inverted into acceptance)` | R5: all THREE become 405 guards (`:927` = serving-leaf identity) |
| 11 | `:124` AC3 | `state-mutating verbs {pause, resume, posture-shift, halt-resolve, orchestrator-queue, revoke-token, revocations-import, spirit-upgrade, forget, legal-hold-release, governance-admit, start, stop, unload, uninstall, run} go to the surface;` | R5: 13 live-state verbs to the surface; `stop` refused; `run` not re-targeted |
| 12 | `:124` AC3 | `TL-only readers {halt-list, revocations-list, orchestrator-status, legal-hold-list, hot-swap-precheck} run in-process in \`maosctl\` with no child spawn.` | R5: only {halt-list, legal-hold-list} in-process; three live-state readers to the door; four durable verbs door-when-live else offline under lock; `run` not re-targeted |
| 13 | `:124` AC3 | `\`MAOS_ONE_SHOT\` survives only for \`smoke-*\`, \`acp-server\`, \`registry-server\`, \`collective-erase\`, \`cohort-a2a-daemon\`` | adds `bench-section-13-1`, `hello-spirit`, four durable modes; 48 → 32 |
| 14 | `:126` AC5 | `Proven-red: with the daemon down (no listener at the \`control.json\` endpoint, or no \`control.json\`) every re-targeted verb exits non-zero …` (full sentence) | live-state verbs typed (`NotInitialized`/`DaemonNotRunning`/`DaemonAliveWithoutDoor`); R5: durable verbs offline under the exclusive lock |
| 15 | `:127` AC6 | `the nine fake-\`maos\` contract test files` ; `\`SCANNED_SOURCE_FILES\` (\`…:863\`, 17 == …, equality assert \`:937-941\`)` | R5: only TWO write a fake; `:869`, 19, `:954-958`, 20 after `operator_door.rs` |
| 16 | `:32` exit-block provenance, Line 5 | `the door is env-gated (\`MAOS_OPERATOR_HTTP_BIND\`, \`main.rs:2706-2724\`, bind at \`:2792\`) and GET-only;` | enabled by the token (`:2705-2728`, 8787 fallback), bound at `:2827` before half the verbs' objects exist, GET-only (R5) |
| — | after `:7` (new paragraph) | — | **Round 5** header: 16-1 section only; five AC2 anchors wrong at authoring (`main.rs` byte-identical `843d5365`↔`a6ed282c`); `maos init` clean-home failure |
| 17 | `docs/adr/ADR-062-mutating-operator-surface.md` (append `## Amendment — Story 16-1`; do NOT rewrite Decision) | — | (a) door roots: `maos run`, `maos shell`, `MAOS_ONE_SHOT=cohort-a2a-daemon`; (b) three read-route POST tests are guards; (c) the 405 phrase now exists (Context `:21-23` superseded); (d) `control.json` single writer stays `maos init`; liveness is the store lock set (`flock` on the home directory and on every durable store's directory; no lock file) held shared by roots; (e) the four durable verbs run offline only when the offline child acquires the whole set exclusively; (f) the token-only 8787 fallback is retired; (h) the Worker-reach residual (D-16-1-S): until 17-1 lands every Worker can read `control.json` and inherits an exported bearer; (i) `control.json` carries `version` and a scheme-qualified endpoint (D-16-1-C); (j) every mutating command gets an `operation_id` and one completion TL row, found with `maosctl audit query --intent-contains` (D-16-1-E); (k) upgrade over the door is hot-swap-only and forward-only, CRL import carries bytes (D-16-1-W/X); (g) stale anchors corrected: init arms `main.rs:1352`/`:1699` → `:1385`/`:1708` (`:42-43`), client `subcommands.rs:1930-1975` → `fetch_live_sandbox_report` (`:1955-2005` at `f72b557b`) (`:32`), "three existing read routes" → four (`:66`) |
| 18 | epic-16 *Closes* (HEAD `:11`) | `FR2, FR9, FR12, … FR50, FR51, FR58;` | FR9 split (16-1 / 16-6); FR51 narrowed to (b)(c) + door reachability of (a)(d); R5 note that (a)(d) bounded guarantees have no mechanism → Epic-16 retrospective (round-table F-B, F-E) |
| 19 | epic-16 Stories table | — (insert before the 16-5 row) | NEW `16-6-maosctl-load-over-the-door` row (F-B) |
| 20 | epic-16 before `## Dependencies` | — | NEW `### 16-6-maosctl-load-over-the-door` section, AC1–AC3 (F-B, F-D threat model) |
| 21 | epic-16 Dependencies | `16-1 before 16-2 and 16-4; 16-3 after 16-1; 16-5 last.` | adds `16-6 after 16-1` |
| 22 | epic-16 16-5 section | after `16-5 lands last in the epic.` | NEW **AC5** — Lifecycle Journal `O_APPEND` (§11 row 1) riding 16-5's re-pin |
| 23 | epic-16 16-2 AC2 / 16-4 AC2 | before `is turned by the shell into \`invoke_halt` / after `\`FIXME(secrets)\` at \`main.rs:3116\` retired.` | owner notes for §11 rows 7 and 2 |
| 24 | epic-16 exit-block narrative (HEAD `:15`) | `and runs line 3's \`maosctl\` verbs from the harness against the shell process's door` | `halt resolve` over the door, `halt list` as an in-process TL read (D-16-1-A) |
| 25 | `epic-17…md` 17-1 AC1 and AC3 | `the fixture's \`open("/host-home/.ssh")\` fails with \`ENOENT\`/\`EACCES\` per the bind-mount set` ; `the paid path now works through the proxy).` | `<MAOS_HOME>/control.json` unreadable from the Worker; `env_clear` removes `MAOS_OPERATOR_*`; probe asserts both (F-D) |
| 26 | `epics/index.md` Epic 16 row; `dependency-dag.md` | `— 6 stories,` ; `16-1 → 16-2, 19-2 \|` | `— 7 stories,` (check-epic-close-coherence compares it to sprint-status) ; `16-1 → 16-2, 16-6, 19-2` |
| 27 | `sprint-status.yaml` | — | NEW row `16-6-maosctl-load-over-the-door: backlog`; `epic-16-retrospective` comment OWES the FR51(a)(d) re-preflight (a queryable home, not prose) |
| 28 | epic-16 *Closes* | `FR51 (b)(c), and the door reachability of FR51 (a)(d)` | FR51 (b) + door reachability of (a)(d); (c) routed to 19-2 with evidence (§17 V-12) |
| 29 | epic-16 16-1 AC1 | `waits (bounded) for the endpoint in \`control.json\` to accept a connection` | readiness = `GET /v1/spirits/butler` ⇒ `Running` (§17 V-9) |
| 30 | epic-16 16-1 AC2 | `\`{endpoint, token}\``; `holds a shared \`flock\` on \`MAOS_HOME/daemon.lock\``; `\`block_on\`s the kernel calls` | versioned `tcp://` schema; store lock set taken before any store opens; spawned uncancelled futures + `operation_id`; + per-Spirit serialization and CI-decoy sentence (§17 V-9) |
| 31 | epic-16 16-1 AC3 | `run offline under an exclusive \`daemon.lock\``; `:1470`, `:1173` | offline `maos` child takes the store lock set exclusively; `:1486`, `:1189` (§17 A1) |
| 32 | epic-16 16-1 AC5 | `\`DaemonAliveWithoutDoor\``; `when the exclusive \`daemon.lock\` is acquired` | `StoreInUse` 69 with exit codes 78/69/77; whole-set acquisition (§17 V-4) |
| 33 | epic-16 16-5 AC5 | `The open gains \`O_APPEND\` semantics;` | `O_APPEND` **and** one `write_all` per line (§17 V-13) |
| 34 | epic-16 16-6 | `maos-bin +80–160`; `(ABI Stability Triple, model provenance, vetting precondition)`; `ships only after 17-1's AC1/AC3 clause` | maos-bin +250–450; admission extraction from `main.rs:3490-5146` and the corrected gate list; vacuous ship gate replaced (§17 V-14) |
| 35 | `epic-17…md` 17-1 AC1 / AC3 | `Both become kind-8`; `\`MAOS_OPERATOR_BEARER_TOKEN\` / \`MAOS_OPERATOR_HTTP_ENDPOINT\``, `asserts neither variable` | three probes; the daemon's `…_HTTP_BIND`; `MAOS_OPERATOR_` prefix assertion (§17 V-15) |
| 36 | `epic-19…md` 19-2; epic-16 budget figures; `sprint-status.yaml`; epic-16 `subcommands.rs` cites | 19-2 `*Closes · Δ:* FR20` (×2) and epic `**Closes:** FR20,`; `maos-control ≤ +533, maos-bin ≤ +670 …, maos-cli ≤ +377` (×2); rows `16-1`, `19-2`, `epic-16-retrospective`; ten cites at `a6ed282c` | NEW 19-2 AC3 + FR51(c) in three Closes; `≤ +559 / +891 / +436`, `maos-capability ≤ +10`; row notes + retro OWES cold-swap; cites re-derived at `f72b557b` (baseline) |
| 37 | epic-16 16-6 Stories row; 16-3 AC2; 16-1 AC2; 16-4 AC2; figures; `sprint-status.yaml` | `maos-bin +80–160`; `discipline.yml:3708-3709`; `on \`.maos-daemon.lock\` in its home and in the directory of every store`; —; `maos-capability ≤ +10`; rows `16-1`, `epic-16-retrospective` | +250–450; `:3727-3728`; directory handles, no lock file; `maos purge` takes the set exclusively; ≤ +11; row notes, cold-swap wording (§17 V-16…V-22) |
| 38 | epic-16 16-1 AC2 / AC5; 16-4 AC2 purge note | `mutating commands are serialized per Spirit id;`; `a wrong bearer (401 ⇒ 77) is a separate negative.`; `run offline when their \`maos\` child acquires the whole store lock set exclusively`; `and refuses 69 \`StoreInUse\` while` | `SpiritBusy`/`HandlerStillRunning` by atomic CAS; `DoorUnresponsive` 69, `LockUnavailable` 78; durable order by configured endpoint, never bypassing a connecting door; purge re-checks inode and holds the set (§17 V-23…V-36) |

---

## 13. The split seam, if the operator orders one (D-16-1-O)

**16-1 (still ◆):** §1 init fix; `control.json` + home rule; store lock set **shared hold by roots**;
root gating and bind relocation; the whole server (AC3); handlers + client for `pause, resume, start,
unload, posture-shift`, the `stop` refusal, `GET /v1/spirits/{id}`, `GET /v1/daemon`; AC1; AC5 for those
verbs; SIGTERM fixes; D-16-1-Q; ADR amendment (a)(b)(c)(d)(f)(g); the pause/posture test rewrites.
**16-1b:** `halt-resolve, orchestrator-*, revoke-token, revocations-*, spirit-upgrade, hot-swap-precheck`,
`forget/uninstall/legal-hold-release/governance-admit` + the **exclusive** offline-child lock, durable
readers, AC4, AC6's table shrink, remaining test rewrites, ADR amendment (e).
**Cost of splitting:** 16-2 depends on halt-resolve and would wait on 16-1b; for a wave, a real door would
serve real lifecycle verbs while `maosctl halt resolve` still resolved a fabricated halt beside it.

---

## 15. Round-table 2026-09-13 — nine forks, operator-ratified (*"approved"*)

Convened by the operator (*"preflight check for the story, resolve the issues raised in the story creation for
the long term correctness"*). Winston, Murat, Amelia, Mary, Paige, John, Sally; Vex and Grumbal sat in. One
measurement was taken for the table before it opened: Worker spawn env inheritance (F-D).

| Fork | Ruling | What changed in this file |
|---|---|---|
| **F-A** sizing | **WHOLE** (Amelia: the handlers are the deleted arms, moved — splitting leaves two dispatch worlds for a wave; John: accepted on condition nothing else sneaks in) | `split_from`; D-16-1-O stands; §13 kept as record |
| **F-B** FR9 `load` | **NEW row `16-6-maosctl-load-over-the-door`** after 16-1, reusing `maos run`'s admission path (Amelia); ships with its threat model stated (Vex); not inside 16-1 (John) | D-16-1-T; Closes; §11 row 6; §12 rows 18–21, 26–27 |
| **F-C** offline durable arm | **Confirmed — but the lock was keyed to the wrong thing.** Sally asked why a connect probe isn't enough; Murat answered (three causes of connect-refused, two alive); Sally then found the memory root ignores `MAOS_HOME`, so a home-only lock let two homes race one store. → two locks, home + memory root, canonical order (Winston); split-home vector (Murat). **Superseded by §17 V-2/V-3:** the same mistake one level down — a store lock set | D-16-1-D; AC2; AC5; review (l); T3, T6, T8 |
| **F-D** who else reaches the door | **Measured exploit path (Vex):** Workers inherit the daemon env (`runtime.rs:461-469`, `env_clear` forbidden by a green test) and run as the operator uid, so a prompt-injected agent CLI can read `control.json` and call the door. Not fixable in 16-1 (kernel byte). → the cure is 17-1's own AC3/AC1, **now naming the operator bearer and `control.json`** with a probe assertion; 16-1 documents the residual (Winston, Murat, Vex) | NEW D-16-1-S; §11 row 8; ADR amendment (h); §12 row 25 |
| **F-E** FR51 over-claim | **Narrowed** (John, Mary): pause flips state; nothing interrupts an in-flight hook; AC4 proves the ring changed, not that in-flight ops failed safe. Named in the epic *Closes* and raised to the Epic-16 retrospective, whose sprint row now OWES it (Paige: in the file, not in memory). **§17 V-12:** (c) was over-claimed too → 19-2 AC3 | NEW D-16-1-U; Closes; §11 row 9; §12 rows 18, 27 |
| **F-F** `HandlerStillRunning` | Grumbal: *"the operator runs it again"*; Sally: *"'it might have happened' is the worst message there is"* → 503 carries an `operation_id`, completion journaled with it, maosctl exits **75** naming the audit query (Winston). **§17 V-1:** that query is `maosctl audit query`, and the id's row is a maos-bin TL row | D-16-1-E, D-16-1-P; AC3; review (n) |
| **F-G** concurrent mutations | **Per-Spirit serialization** of mutating commands; the kernel CAS keeps state sane but not the audit rows (Murat) | NEW D-16-1-R; AC4; T5 |
| **F-H** shared-HOME tests | Grumbal: *"nine edits is a receipt — where's the lint?"* → kloc-free `root_spawn_home_isolation_16_1.rs`, **proven red on the nine first** (Murat). **§17 V-6:** re-shaped — CI decoy binding, file-granular floor; the list is longer than nine | D-16-1-Q; AC7; review (m); T3 |
| **F-I** future transport | TCP stays (ADR-062, three 14-2 tests, 21-3's `/v1/a2a/pins`); a Unix socket would dissolve half the evening's hazards, so the schema must not forbid it: `control.json` gets `version` and a `tcp://` scheme, unknown values refused (Winston, Amelia) | D-16-1-C; AC2; ADR amendment (i); T2 |

**Ratified in the same approval:** `maosctl stop` becomes a refusal (exit 2) naming `pause`/`unload` (Sally's
question to the operator).

---

## 16. Goal and drift check (operator-requested, 2026-09-13)

**The goal this story serves.** Epic 16 *makes true*: *"the operator controls a running Spirit and can watch a
halt resolve"*; the recovery lane's standing test (full drift review 2026-09-04) is that a story advances a wave's
demo command, not a gate or a ledger.

**Does 16-1 reach it? — traced line by line against the J0 exit block, not asserted:**

| exit line | needs | after 16-1 |
|---|---|---|
| 1 `maos init && … maos shell` | init works on a clean home; shell binds the door | **16-1 delivers** (§1 fix; door roots) |
| 2–3 `journey_j0` + `halt list` / `halt resolve` | a real halt in the shell's registry, resolvable over the door | **16-1 delivers the door and the real resolver; 16-2 delivers the halt.** `halt list` reads the shell's TL in-process (epic narrative corrected, §12 row 24) |
| 4 `maos audit query` | halt + resolution rows | 16-2 |
| 5–6 `maos run butler` · `pause` · `inspect` · `kill` | a real pause, read back from the live SCB, a clean SIGTERM | **16-1 delivers exactly this** (AC1 steps 1–5) |
| 7 `maos purge --keep-log` | product removal | 16-4 (unaffected; its AC3 `uninstall` rides this door) |

**Verdict: on goal.** Every line of the exit block has a named story, and 16-1's own lines are asserted by
behaviour (the scheduler state, not a row). The deliverable is operator-observable — the lane's test — and its
governance footprint is small and subordinate: one kloc-free lint test, one ADR amendment, one gate re-anchor.

**Where it could still drift, stated so the retro can check each one:**
1. **Duration.** 16-1 at 6–9 d plus the new 16-6 pushes Epic 16 toward the top of its 3.5–5.5 wk band. Not a spec
   drift — a schedule risk. The operator's fewer-larger-stories preference was weighed against FR9 fidelity; F-B
   chose fidelity.
2. **FR51(a)(d) and cold-swap.** The narrowed claim is honest today but becomes a *dead* claim if the retro skips it.
   Its only guard is the `epic-16-retrospective` row comment — a queryable place, still a promise until a vehicle is
   named. The same row now OWES the cold-swap stale-pid defect (§11 row 11).
3. **The Worker-reach window (F-D).** Between 16-1 and 17-1 the door is reachable by any bare Worker. Hermetic CI
   uses fixture Workers; live agents run on the operator lane. If 17-1 slips, the residual lives longer — it is
   now 17-1's AC text, so it cannot close green around it.
4. **ADR-062's amendment lands at dev, not now.** If T10 is skipped, `decision_adrs_and_provisioning` reds — but
   only because §17 V-11 extends its `"062"` arm to require the amendment text; as first written this line was
   false (the arm checks only the ADR's `## Context`, where the amendment does not live).
5. **Re-measurement.** Every line citation here is a snapshot at `f72b557b` (re-baselined from `a6ed282c`); T0 re-measures, and the dev cites
   symbols. Two waves of this project died on inherited citations.

**Spec coherence.** As first written after the round-table this paragraph said story and specs *agree*; validation
round 2 (§17 V-9) found the epic's 16-1 ACs still carrying pre-round-table text (one `daemon.lock`, `{endpoint,
token}`, `block_on`, accept-readiness). Replaced then. At `f72b557b`, story, epic 16 (Closes, Stories, 16-1 ACs,
16-2/16-4/16-5/16-6 sections, Dependencies, exit narrative), epic 17 (17-1 AC1/AC3), epic 19 (19-2 AC3),
`index.md`, `dependency-dag.md` and `sprint-status.yaml` agree — re-verified by diff, with `check-epic-close-coherence`
and `check-exit-commands` PASS. The only owed spec edit is ADR-062's amendment, bound to the code commit and now
to its gate.

---

## 17. Validation round 2 and the AC4 spike (2026-09-13, pre-dev, HEAD `f72b557b`)

The readiness check the operator asked for (*"is the story truly ready to dev?"*) named four gaps: the round-table's
additions had never been read by anyone but their authors, AC4's fixtures were unmeasured, the SIGTERM drop order
was unwritten, and the baseline had moved. All four were worked the same day: a fresh-context validator (read-only,
≈45 items confirmed) and a spike engineer (isolated worktree, throwaway hook calling the daemon's own objects,
nothing merged). **The round-table's additions carried the same density of false premises as the first draft** —
this section is the record.

**Validator findings → rulings** (ship-blocker/must-fix unless marked):

| # | Finding (evidence) | Ruling | Where |
|---|---|---|---|
| V-1 | The `operation_id` lookup named `maos audit query --intent-contains`; that parser drops unknown flags and printed every row, exit 0 (`main.rs:1714-1739`). The flag is `maosctl`'s (`cli.rs:430`) and filters the TL `intent` column only; the kernel journal helpers hard-code `intent` (`orchestrator/mod.rs:62-78`); the port signature had nowhere to carry an id | port `submit` returns `{operation_id, completion}`; maos-bin writes one completion TL row per mutating command via `insert_frame_event` (`main.rs:2613-2621` pattern), zero kernel-Δ; lookup = `maosctl audit query` | D-16-1-E/F/P; AC3; review (n); Trap 21 |
| V-2 | Two locks (home + memory root) still missed the stores: with `MAOS_HOME` unset the TL, memory db (shared tier, principal index, holds — `main.rs:1793,1855-1862,2039`) and proofs dir (`:8834`) resolve through `MAOS_AUDIT_DB`/XDG — a root and a child with different `HOME`s shared the log with no common lock | **store lock set** over the home and every durable store's directory, resolved by the existing resolvers (its lock-FILE form was superseded by V-16) | D-16-1-D; AC2 shared-log vector; review (l) |
| V-3 | Two different file names + "take once when same directory" ⇒ a root locking one name and a child locking the other both succeed | dedup by canonical directory (moot after V-16: no names at all) | D-16-1-D |
| V-4 | "Before touching any store" was impossible at the one-shot dispatch (`:5159`): boot has already opened the TL (`:2038`), written the tenant binding (`:2061`), opened the journal (`:2743`); and `flock` cannot tell a door-less daemon from a second offline child, so `DaemonAliveWithoutDoor` was a claim the mechanism cannot make | lock set taken before `main.rs:1793`; neutral typed `StoreInUse`; maosctl's momentary home-lock probe + roots' 500 ms retry (the round-table's "maosctl never takes the lock" was unimplementable for AC5) | D-16-1-D/P; AC2; AC5; Trap 20 |
| V-5 | Lock fd inheritance: a Worker could outlive a crashed daemon holding its lock; "running offline" printed by maosctl before the child's lock decided; non-unix door-only contradicted §6 now that maos-cli builds on Windows | `std::fs::File::open` (`O_CLOEXEC`) + CLOEXEC vector; the child prints after acquiring; no release target is non-unix (`release.yml:43-51`) ⇒ typed `OfflineUnsupported` | D-16-1-D; AC2; AC5; review (o) |
| V-6 | D-16-1-Q's nine files were incomplete (`butler_8_14b`, `smoke_cli_wrapper_8_12`, `cross_team_crossing_13_6b`, a second `13_5c` site, `cert_rotation_trigger_14_2a`) and the lint "same command" has no textual boundary (split builders, late `MAOS_HOME`, journey isolation in `maos-journey-test/src/lib.rs:109-117`, bare `maos` = shell root) | CI decoy `control.json` is the binding control; file-granular lint with allowlist is the local floor; T0 derives the list from the decoy run | D-16-1-Q; AC7; review (m); T0/T3 |
| V-7 | Per-Spirit queueing consumed worker slots and could return `HandlerStillRunning` + id for a command that never started; 503 meant three things with one exit mapping; 405/411/413 unmapped | wait inside the budget ⇒ `SpiritBusy` (unstarted, no id); `Busy`; all retryable 503 ⇒ 75; protocol errors ⇒ 1 | D-16-1-E/P/R; AC3 |
| V-8 | AC2's proof exported the token alone, which the both-or-neither rule makes a 78 for a root | export the root's pair (`MAOS_OPERATOR_HTTP_BIND` + token) | AC2 |
| V-9 | **Rule-9 drift:** the epic's 16-1 ACs carried none of the round-table rulings (one `daemon.lock`, `{endpoint, token}`, `block_on`, accept-readiness), and §16 said "agree" | epic 16-1 AC1/AC2/AC3/AC5 replaced; §16 corrected | §12 rows 29–32 |
| V-10 | Durable verbs with no `control.json`: unspecified | offline child directly — erasure never requires `maos init` | D-16-1-D; AC5 |
| V-11 | §16 item 4 claimed the ADR gate would catch a skipped amendment; the `"062"` arm checks `lib.rs`, the dep test and only the ADR's `## Context` (`decision_adrs_and_provisioning.rs:187,240,310-339`) | extend the arm to `require_contains` the amendment heading and two literals | AC3; T4; §16 |
| V-12 | *(minor)* FR51(c) — *recall pending Orchestrator-buffered actions per FR20* — claimed with no AC; resume's `recall_all_pending` re-enqueues (`scheduler_loop.rs:452-460`) | narrowed to (b); (c) → 19-2 AC3 by charter (FR20 owner; recall needs its safe-point dequeue) | Closes; D-16-1-U; §11 row 10; §12 rows 28, 36 |
| V-13 | 16-5 AC5: `O_APPEND` alone still interleaves — `write!(file, "{line}\n")` on a raw `File` is ≥ 2 `write(2)` calls (`journal/mod.rs:180`) | one `write_all` per line | §12 row 33 |
| V-14 | 16-6: admission is inline in `if let Some(run)` (`main.rs:3490-5146`), per class via `classify_spirit` (`:453`); the vetting precondition is an upgrade gate only (`:6452,:6734`); its AC3 ship gate was already satisfied when written | extraction in scope, budget +250–450; gate list corrected; AC3 replaced | §12 row 34 |
| V-15 | *(minor)* 17-1: a Worker inherits the daemon's `MAOS_OPERATOR_HTTP_BIND`, not maosctl's `…_ENDPOINT`; AC1 said "Both" of three probes | prefix assertion; three probes | §12 row 35 |
| A1–A3 | *(minor)* epic 16-1 AC3 `:1470`/`:1173` un-shifted; AC8 "rows 18–26"; §12 header duplicated sentence | corrected | epic; AC8; §12 |

**Consistency check (third fresh reader, same day, on the round-2 text only; ≈75 new cites confirmed):**

| # | Finding (evidence) | Ruling | Where |
|---|---|---|---|
| V-16 | **HIGH.** A `.maos-daemon.lock` file in each store directory reds `erasure_uninstall_13_5b` — file counts in the proofs dir and memory root (`:458,480,503,607,623,846`) and the planted-file exit 5 (`:627-637`) — contradicting AC5's "stays green unchanged"; a shared parent like `/tmp` yields a foreign-owned lock file | **flock the store directories' own handles — no file created**; create absent dirs (the proofs dir early), lock the parent of a non-directory path; residual (another uid that can open a store dir can make offline erasure refuse) stated | D-16-1-C/D/P; AC2 no-file vector; Trap 24; T0/T3 |
| V-17 | `SpiritBusy` was inexpressible: an id minted for every command, no started/withdraw signal in `{operation_id, completion}` | `submit(cmd, deadline)`; the port answers `SpiritBusy` itself before `deadline − 250 ms`, so a server timeout means started | D-16-1-E/F |
| V-18 | Offline children took `LOCK_EX` with no retry, so a concurrent maosctl probe could make erasure refuse; the durable order's "else offline" included a reachable door answering 401; the decoy's accept-and-close listener had no exit mapping | both roots and children retry 500 ms; order split by connect-refused / no-response / response; typed `DoorUnresponsive` 69 | D-16-1-D/P; AC5 |
| V-19 | D-16-1-Q: the file-granular floor cannot red on files that already hold a `HOME` literal; `cert_rotation_trigger_14_2a` is an allowlist case, not an offender; `accessibility_test` and six maos-cli files use `env_clear`; the decoy needs `maos` built and T1 in its step | floor scoped to files with no literal; allowlist reasons incl. `env_clear`; decoy step builds `maos`; ≈12 files estimated red | D-16-1-Q; AC7 |
| V-20 | Cold-swap: `pid_by_spirit_id` is a maos-bin map (`main.rs:2428`), not kernel; only the missing `admit_spirit` is the kernel half; the map's staleness was read from code, not instrumented | wording corrected; the door still refuses (fixing only the map leaves an unadmitted Spirit) | D-16-1-W; §11 row 11; retro row |
| V-21 | 16-4's `maos purge` deletes the XDG `memory/`, `erasure-proofs/`, `audit/` directories and nothing told it to take the lock set | note written into epic 16-4 AC2 | §11 row 2; epic |
| V-22 | *(minor)* D-16-1-E's intent-filter cite (`:184-196` is the SELECT; filter `:264-266`, `:444-445`); `release.yml:43-51`; epic 16-3 `discipline.yml:3727-3728` (already wrong at `a6ed282c`); the journal dir is created at `main.rs:2733-2736`, the proofs dir lazily; D-16-1-V's `worker_spawn.rs:730` caller is `revoke_cli_subprocess_exit` (and loses its second audit row for an already-revoked token); `set_revoked`'s `Err(())` must not be overloaded; epic 16-6 table row still +80–160; leftover "two-lock"/`daemon.lock`/"nine" wording; T7 vs Trap 16; review (c) token-only; frontmatter (a)–(n); `maos-capability` ×1.3 rounding; spike evidence lived only in a scratchpad; root E's fast-idle recipe was never run | all corrected; evidence preserved in `16-1-evidence/spike/` with its gap stated | throughout |

**Fourth read (on the V-16…V-22 text only; no ship-blocker; flock-on-directory probed on Linux with `rustix` 1.1.4 — `File::open(dir)` works, `LOCK_SH` blocks another process's `LOCK_EX|NB`, released on drop even with a live child, symlinks lock the same inode, SQLite transactions inside do not conflict; macOS unprobed; no MAOS code takes flock/fcntl locks today):**

| # | Finding (evidence) | Ruling | Where |
|---|---|---|---|
| V-23 | Another uid that can open a store dir (boot's default `0755`, or `/tmp` via `MAOS_AUDIT_DB`) can make **roots refuse to boot**, not only offline erasure refuse — the `0600` lock files of round 2 had guarded this, dropped without a note | created dirs `0700`; boot-DoS residual stated with the trade | D-16-1-D |
| V-24 | `LOCK_EX` on a directory fails `EBADF` on NFS (needs a writable handle); `EACCES`, `mkdir` failures unmapped | typed `LockUnavailable{path, errno}` 78; network filesystems unsupported for the offline arm; roots refuse rather than run unlocked | D-16-1-D/P; AC5 |
| V-25 | **"No outcome at deadline ⇒ started" was not guaranteed** — a starved runtime or unpolled task delivers the port's `SpiritBusy` after the server gives up, yielding an id with no row | shared `Queued/Started/Withdrawn` atomic; port CASes to `Started`, server CASes to `Withdrawn` | D-16-1-E/F/R; AC3; T4 |
| V-26 | Durable order branched on `control.json`, ignoring AC2's env-pair discovery — `forget` with the pair exported against a live door exited 69 | "no endpoint configured (env pair nor `control.json`)" | D-16-1-D; AC5; T6 |
| V-27 | "Two offline children" vector timing-dependent after the 500 ms retry | the test holds `LOCK_EX` itself for 2 s | AC5 |
| V-28 | AC3's `SpiritBusy` "never runs" was asserted against a fake port while the waiting lives in maos-bin (E15-A6) | maos-control proves the mapping; the real port proves never-runs | AC3 |
| V-29 | Rule 9: epic 16-1 AC5 lacked `DoorUnresponsive` / never-offline-when-connected; `SpiritBusy`, the retry absent from the epic | epic AC2/AC5 amended | §12 row 38 |
| V-30 | `maos purge` locks inodes it then deletes; a root arriving after the unlink locks a new dir without conflict | `fstat(handle)` = `stat(path)` re-check after every lock; purge holds the set to exit | D-16-1-D; epic 16-4 |
| V-31 | *(minor)* dedup by canonical path misses bind-mount aliases ⇒ a child's second `LOCK_EX` fails against its own handle | dedup by `(dev, ino)` | D-16-1-D |
| V-32 | *(minor)* `HOME` alone does not isolate stores when `XDG_DATA_HOME` is exported (no conflicting test pair at HEAD) | isolate `XDG_DATA_HOME` too | D-16-1-Q; Trap 8 |
| V-33 | *(minor)* early proofs-dir creation with `HOME`/XDG unset hits `/var/lib/maos` ⇒ `EACCES` at boot, where only erasure touched it before | `LockUnavailable` 78 naming the path (`MAOS_ERASURE_PROOFS_DIR` is the remedy) — intended: a daemon that cannot write erasure proofs is misconfigured | D-16-1-D |
| V-34 | *(minor)* "a command that STARTS writes one row" lost *mutating* (GET polling would write rows); `DoorUnresponsive` named no remedy | restored; remedy in stderr | D-16-1-D/E |
| V-35 | *(minor, but an audit regression)* D-16-1-V would drop the Worker's `CliSubprocessExit` row for EVERY token already revoked — by unload and CRL too, not only by the operator | reason-keyed: only `Operator` re-revoke is typed and silent; `CliSubprocessExit` stays `Ok` with its row | D-16-1-V |
| V-36 | *(minor)* "30.007 s" not in preserved logs; `resolved_transparency_log_path()` also stats ancestors (`maos-audit/src/lib.rs:949-966`); `cap_tokens/mod.rs:272-287`; leftover lock-file constants (§10, T2), old `SpiritBusy` model (D-16-1-R, AC3), `submit` without state (T4), cold-swap "stale pid map" wording (§4, §11, Q3), §12 row-history ambiguity, D-16-1-W "(2)", headline vs `maosctl run` | all corrected; §12 marked as history | throughout |

**Spike (AC4 fixtures), measured at `f72b557b`:**

| Q | Verdict | Measured | Became |
|---|---|---|---|
| Q1 tokens | **not feasible as written** (evidence: `16-1-evidence/spike/logs/spikeD*`, `spikeD4.out`) | butler replay mints no token (12/42/50 s, `capability_token` NULL); the researcher replay writes `cap.issue` ≈4 ms after boot; a second revoke of a live token returns `Ok` and writes a second `cap.revoke` (`shard.rs:44-52`); a 3-line `maos-capability` change makes it `Err(Revoked)` with 28/28 capability tests and the kernel revoke tests green | AC4 root D; D-16-1-V; Traps 17–18 |
| Q2 CRL | feasible with change | `apply_crl` on a live butler root ⇒ `matched_count 1`, SCB gone in 8 ms; ≈45-line fixture with no new dependency; anchor moves into `LocalFileRegistryClient` at boot; poller reads `/tmp/maos/crl` ≈1/s; `CrlId` serialises as an int array; the one-shot also exits 101 | AC4 root B; D-16-1-X |
| Q3 upgrade | **not feasible as written** | canonical butler manifest lacks `[scheduling]`/`[lifecycle]`; no butler successor loader; a bare successor "completes" and loses its halt; a faithful successor keeps it (halt 30.007 s after swap — reported by the spike, not in the preserved logs); downgrades accepted; cold-swap starts an unadmitted successor under a new pid; attestation/keyring read from daemon env | AC4 root E; D-16-1-W; §11 row 11 |
| Q4 root lifetime | feasible | butler stays up and Running until SIGTERM; `on_idle` fires once at ≈30 s ⇒ pending `belief_variance` halt with `spirit_pid` 0; SIGTERM still 10 s | AC4 root A wording; Trap 19 |

**Author rulings — RATIFIED by the operator 2026-09-13 (*"approved"*).** V-2/V-3/V-4/V-16 (store lock set on directory handles, `StoreInUse`),
V-6/V-19 (CI decoy), V-7/V-17/V-25 (`SpiritBusy` by an atomic CAS), V-18 (`DoorUnresponsive`), V-23/V-24/V-33 (`0700`
dirs, `LockUnavailable`, the boot-DoS residual), V-35 (reason-keyed re-revoke), V-12 (FR51(c) → 19-2) and
D-16-1-V/W/X were ruled by the author under the round-table's direction,
not at a table. None adds scope beyond the epic's 16-1 section, none touches the kernel, and every crate stays inside
headroom (§10). They were marked here for the operator, who ratified all of them with the commit instruction.

**What this round shows.** The round-table fixed the first draft's premises and introduced its own at the same rate
(6 must-fix of ≈60 items), two of the three unmeasured fixtures were impossible as written — and the corrections
written in response carried one HIGH of their own (V-16), found only by a third reader; the fourth read found no
ship-blocker but eight must-fix refinements (V-23…V-30). The curve — 17 → 6 must-fix → 1 HIGH → 0 ship-blockers —
is the evidence that the design has converged. Every round was cheaper than a dev discovering it. *A ruling is a
premise too: a table that decides without measuring hands the dev a new unmeasured claim.*

---

## Dev Agent Record

### Agent Model Used

`opus-5` (frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1,
glm-5.2, glm-5.3, opus-5, equiv}).

### Debug Log References

**T0 — the story's premises re-measured at `f72b557b`, before any edit.**
- §1 init probe, both vectors, against the built binary: `maos init` with `HOME` pointing at a
  directory without `.maos` ⇒ `Error: Os { code: 2, kind: NotFound }`, `EXIT=1`; same with
  `MAOS_HOME` absent. Premise CONFIRMED.
- `verbs::MAOS_ONE_SHOT_MODES.len()` = **48**, and `one_shot_mode_table_is_complete_and_frozen`
  pinned 48. Premise CONFIRMED.
- D-16-1-Q offender derivation, measured from the tree rather than from the story's list:
  **43** test files spawn a `maos` root; **20** of them spawn a DOOR root with no `HOME`
  isolation; **16** carried no isolation literal at all. The story said nine and validation
  round 2 estimated ≈12 — **both were low**.

**AC1 step 2 — the pause falsifier (run, recorded, reverted).** With
`"pause" => self.scheduler.pause(pid).await` replaced by `Ok(())` in
`operator_door::lifecycle_command`:
```
test ac1_pause_is_real_and_read_back_from_the_owning_process ... FAILED
assertion `left == right` failed: the SCB the daemon owns must actually be Paused, not merely journaled
  left: Some("Running")   right: Some("Paused")
```
The Lifecycle Journal `Pause` row still landed — which is precisely the HEAD behaviour the story
measured, and precisely what a journal-only assertion would have accepted. Reverted; 14/14 green.

**AC4 root E — the faithful-successor falsifier (run, recorded, reverted).** With the successor
factory's butler arm replaced by a bare `butler::Butler::default()`:
```
test ac4_root_e_a_faithful_successor_survives_the_hot_swap ... FAILED
the swapped-in successor must re-fire butler's belief_variance halt (0 -> 0)
```
Every earlier assertion in that test still passed — the swap reported `outcome: completed`, exit 0,
same pid, `Running`. Only the halt was silently lost. Reverted.

**D-16-1-Q floor — proven red, then tightened after it was measured LYING.** First run listed 16
offender files. After fixing them the floor went green — but `f4_pairing.rs` and
`verb_table_15_3.rs` were clearing it on a COMMENT that merely mentioned `MAOS_HOME` while setting
nothing. The predicate now requires an assignment (`.env`/`insert`/`set_var`/`env_clear`) and
follows one level of `#[path]` include; `f4_pairing.rs` was then fixed for real.

**T7 — the 10 s SIGTERM drain, measured to its cause.** Three runs at HEAD-behaviour: `exit_ms=10021`,
`10020`, `10021`, each with `maos: audit writer drain timed out after 10s`. Dropping the eleven
"obvious" `audit_tx` owners left `Arc::strong_count(&capability) == 1`. `Arc::downgrade` probes at the
drain point named the chain:
`delegation_leg → Arc<Mailbox> → ScbTracker → the SCB map → butler's SCB → the butler object →
ButlerOrchestratorAdapter → WorkingMemoryOrchestrator → Arc<CapabilityRegistryAdapter> → audit_tx`,
plus `revocation_poller → RevocationApplier → Arc<scheduler>`. Neither is suggested by any single
line. Also decisive: the watchdog JoinHandles were awaited AFTER the audit writer, so four live tasks
held the channel open while the process waited for it to close. After the fix, three runs with the
door live and butler `Running`: `exit_ms=12 / 11 / 19`, `drain_timeouts=0`, `drained 3 cap-audit row(s)`.

**Live verb sweep against a real butler root** (`maos run spirits/butler/manifest.toml`, replay):
`start` on Running ⇒ 1 `invalid_state_transition`; `pause` ⇒ 0 then `lifecycle_state: Paused`;
`start` on Paused ⇒ 1, naming `maosctl resume`; `posture --shift cautious` ⇒ 0 then
`posture: Cautious`; `resume` ⇒ 0 then `Running`; `stop` ⇒ 2; `orchestrator queue` then `status` ⇒
`1/32`; `halt resolve <never-issued>` ⇒ 1 `halt_not_pending`; unknown spirit ⇒ 1 (404); wrong bearer
⇒ 77. Journal after the sweep: exactly `Load`, `Pause`, `PostureShift`, `Resume` — **zero `Start`
rows**, because both refused starts wrote nothing. Transparency Log: `lifecycle.pause` at butler's
REAL pid 1, plus exactly one `operator.pause.op-<id>` completion row.

### Completion Notes List

**Five premises this story asserted were measured WRONG during dev and re-ruled.**
1. **D-16-1-D's `maosctl` probe was unimplementable as written.** A momentary `LOCK_SH` probe can
   never detect a live daemon, because roots hold `LOCK_SH` and shared locks are mutually
   compatible — AC5's "home lock held ⇒ `StoreInUse`" vector is decidable only by an EXCLUSIVE
   attempt. `maosctl` now takes `LOCK_EX|LOCK_NB` momentarily: `EWOULDBLOCK` ⇒ `StoreInUse`,
   success ⇒ `DaemonNotRunning`, other errno ⇒ `LockUnavailable` 78.
2. **D-16-1-H's `POST /v1/legal-holds/{principal}/release` could not carry a real principal.**
   Every legal-hold principal in this tree is email-shaped (`held-uninstall@example.org`,
   `held@example.org`) and `@` is outside the `[A-Za-z0-9._-]` class every path segment is
   validated against. The route is now `POST /v1/legal-holds/release` with the principal in the
   BODY, symmetric with `/v1/memory/forget`. Widening the charset for one route, or
   percent-decoding inside the door, were both worse. Regression-guarded.
3. **D-16-1-J is not what the kernel's transition table gives.** `is_transition_allowed` keys on
   the TARGET state, so it permits `Paused → Running`; measured, `maosctl start` on a PAUSED butler
   returned 200 `Running` and journaled a `Start` row. The door now enforces `Loaded → Running`
   only and the refusal names `maosctl resume`.
4. **The terminal codes cannot ride the HTTP status.** `OperatorOutcome::Completed` is always 200,
   so `uninstall` 0/3/4/5, `forget` 0/3 and precheck 0/2 travel in a `terminal_code` field of the
   200 body.
5. **D-16-1-W(3)'s butler arm compiled as an UNREACHABLE pattern** when first placed after the
   factory's catch-all, so every butler upgrade answered "no successor loader registered for class
   'butler'". Caught by a `warning: unreachable pattern`, not by a test — the AC4 root E vector was
   written afterwards and is what now holds it.

**Two scope calls, both toward keeping a shipped capability rather than dropping it.**
- `maosctl spirit upgrade --plan/--from/--candidates` was initially refused client-side as "not on
  the door surface". That would have left `migration_plan::upgrade_with_plan_guard` — which still
  runs daemon-side on every upgrade — reading a plan nothing could ever write, and a shipped
  operator flag with no implementation. Plan creation now travels over the door
  (`create_plan`/`from_version`/`candidates` in the upgrade body, canonicalised paths) through a
  new `BinPrivateOps::create_migration_plan`. `--policy migrator` stays refused (exit 2): it names
  the kernel's multi-hop executor, which D-16-1-W does not expose.
- The daemon refuses a CRL import typed (`crl_trust_anchor_unconfigured`) when it booted without
  `MAOS_CRL_TRUST_ANCHOR_PUB_HEX`, rather than verifying against an empty anchor. An unverified CRL
  import would let one POST unload every Spirit on the host.

**Two gates re-measured, each with its reason recorded in the gate itself.**
- `manifest_field_coverage::production_capability_parsers_are_all_schema_degraded`: 5 → 4 parsers
  and 6 → 5 degradations. The parser that went away is the `posture-shift` one-shot arm's, which
  re-admitted pid 0 into a fresh `PolicyTable` from a CWD-relative manifest and then shifted pid 0
  — it degraded a schema it threw away. (`maos-kernel-core/tests/`, not `src/`: the content-hash
  pin covers `src` only and stayed GREEN at `changed == 0`.)
- `decision_adrs_and_provisioning` `"062"`: `require_absent("Method Not Allowed")` flipped to
  `require_contains`, plus three new ADR-body anchors. Proven red by removing the amendment
  heading, then restored.

**`kloc-check` was failing before any code changed**, on the story's own preserved spike evidence:
`16-1-evidence/spike/spike_crl_fixture_16_1.rs` is a `.rs` file outside every crate, so the budget
walker attributed it to `(unknown:_bmad-output)`. Renamed to `.rs.txt` — it is a preserved
transcript, not compiled source.

**Measured budgets, `cargo fmt --all` first, all inside headroom — ZERO ceiling edits, as §10
forecast.** `maos-control` 1105/1791 · `maos-bin` 18703/20109 · `maos-cli` 5919/6856 ·
`maos-shell` 320/500 (leaves 180 for 16-2) · `maos-domain` 9027/9192 · `maos-audit` 6915/6951 ·
`maos-capability` 1043/2000 (−3) · `maos-kernel-core` **0 Δ**, `check-kernel-baseline` PASSED with
24474 == 24474 over 98 files.

**Not done, and saying so:** AC4 root B (the signed-CRL import vector against a live root) and
root D (the researcher token double-revoke vector) are NOT covered by an automated test. The CRL
import path, the trust-anchor-at-boot capture and the reason-keyed re-revoke are all implemented
and unit-covered (`crates/maos-capability/tests/operator_re_revoke_16_1.rs`, 3 tests, including
the `CliSubprocessExit` falsifier), and `revocations import`/`revoke-token` are covered as contract
tests in `maos-cli`; what is missing is the end-to-end vector that signs a CRL and applies it to a
live butler, and the researcher-root token vector. Both need fixture work the story sized into
root B/root D and neither is exercised by `one_daemon_one_door_16_1`. **This is the story's one
incomplete AC4 item** and belongs in the §A6 review net's runtime layer.

### File List

**New**
- `crates/maos-domain/src/operator_door.rs` — the ONE home rule, `ControlFile`, custody validation, atomic `0600` writer, token minter
- `crates/maos-control/tests/post_surface_16_1.rs` — 30 tests
- `crates/maos-bin/src/operator_door.rs` — `OperatorDoor` (the `OperatorCommandPort` impl), `BinPrivateOps`, `SuccessorManifestSlot`, per-Spirit serialization
- `crates/maos-bin/tests/one_daemon_one_door_16_1.rs` — 14 tests (AC1/AC2/AC4/AC5)
- `crates/maos-bin/tests/root_spawn_home_isolation_16_1.rs` — the D-16-1-Q file-granular floor
- `crates/maos-cli/src/door_client.rs` — discovery, exchange, typed errors, the D-16-1-P exit table
- `crates/maos-cli/tests/support/fixture_door.rs` — the hand-rolled contract-test door
- `crates/maos-capability/tests/operator_re_revoke_16_1.rs` — D-16-1-V
- `tests/harness/doorless_home.rs` — the shared empty-scratch-`HOME` harness

**Modified — production**
- `crates/maos-control/src/lib.rs` · `crates/maos-bin/src/main.rs` · `crates/maos-bin/src/lib.rs` ·
  `crates/maos-bin/src/verbs.rs` (48 → 32) · `crates/maos-bin/src/env_contract.rs` ·
  `crates/maos-bin/Cargo.toml` · `crates/maos-cli/src/subcommands.rs` · `crates/maos-cli/src/cli.rs` ·
  `crates/maos-cli/src/lib.rs` · `crates/maos-cli/Cargo.toml` · `crates/maos-audit/src/lib.rs`
  (`resolve_spirit_name`, `list_legal_holds_readonly`) · `crates/maos-capability/src/cap_tokens/{mod,shard}.rs` ·
  `crates/maos-domain/src/lib.rs` · `crates/maos-shell/src/lib.rs` · `crates/maos-journey-test/src/lib.rs` · `Cargo.lock`

**Modified — tests**
- `crates/maos-shell/tests/init_test.rs` (+4, red first) · `crates/maos-bin/tests/verb_table_15_3.rs` (32) ·
  `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs` (`SCANNED_SOURCE_FILES` 19 → 20) ·
  `crates/maos-bin/tests/erasure_uninstall_13_5b.rs` (list half → read-only reader) ·
  `crates/maos-kernel-core/tests/manifest_field_coverage.rs` (re-measured) ·
  seven `crates/maos-cli/tests/*` contract rewrites + `accessibility_test.rs` +
  `legal_hold_cli_13_5b.rs` + `uninstall_exit_codes_13_5b.rs` ·
  `xtask/tests/decision_adrs_and_provisioning.rs` (405 anchor + ADR-body anchors) ·
  `HOME` isolation in `f4_pairing`, `two_host_delegation_2b`, `maos_run_boot_loud_8_11`,
  `researcher_8_14c`, `inference_provider_polarity_8_11`, `smoke_run_8_11`,
  `smoke_mira_nash_tcp_8_13`, `smoke_bench_5e_test`, `smoke_multi_provider_test`,
  `smoke_registry_5d_test`, `one_shot_hello`, `jb3_self_tuning_halt`, `journey_butler`, `journey_j3`

**Modified — spec, docs, CI**
- `docs/adr/ADR-062-mutating-operator-surface.md` (`## Amendment — Story 16-1`) · `README.md` ·
  `.github/workflows/discipline.yml` (D-16-1-Q decoy step + the `one-daemon-one-door` job, enrolled in
  `v1-0-ship-gate`) · `tests/integration/maosctl_smoke.sh` · `tests/integration/v01_evaluator_path.sh` ·
  `_bmad-output/implementation-artifacts/deferred-work.md` · `_bmad-output/implementation-artifacts/sprint-status.yaml` ·
  `_bmad-output/planning-artifacts/epics/{epic-19,epic-20,epic-21}` (invalidated cites re-pointed by symbol)

**Deleted**
- `tests/integration/halt_resolve_smoke.sh` → replaced by `one_daemon_one_door_16_1::ac4_root_a_orchestrator_halt_and_precheck_are_real` (typed `halt_not_pending`, no resolution row) and the `maos-cli` `halt_resolve_test` contract suite
- `tests/integration/orchestrator_queue_smoke.sh` → replaced by the same root-A test (`queue` then `status` ⇒ `1/32`, the live buffer) and `orchestrator_queue_test`
- `tests/integration/maosctl_hot_swap_precheck.sh` → replaced by root A's "precheck writes no phantom admit/load row" assertion and `spirit_upgrade_test`'s clap/exit-code cases

**Renamed**
- `16-1-evidence/spike/spike_crl_fixture_16_1.rs` → `.rs.txt` (a `.rs` file outside every crate broke `kloc-check`)

### Change Log

| Date | Change |
|---|---|
| 2026-09-13 | Story created from the epic's 16-1 section at `a6ed282c`. Two adversarial scouts (server side; daemon runtime per verb) plus author probes against built binaries. Premise-level disproofs §1–§9. 17 decisions, 8 ACs, 7 cut lines routed by charter; epic Round-5 edits applied; ADR-062 amendment owed with the code. Sizing WHOLE, seam §13. |
| 2026-09-13 | **VALIDATION ROUND, same day.** A fresh-context validator re-measured ≈236 citations (219 correct, 12 off-by-N, 5 wrong) and every number. Applied: the baseline's "+37 past `:2700`" claim was false (five AC2 anchors wrong at authoring; `main.rs` unchanged since R4); `monotonic_now_ns` is a `debug_assert!` (spirit-upgrade exit 101 is debug-only); bind call sites are 18 test + 1 production, not 15; maosctl does have a TL resolver; §10 re-done with interval arithmetic and 13 (not 16) removed spawn sites; no new RNG dependency (maos-domain has getrandom; `deny.toml:63` skips it); `rustix` unix-gated. **Design gaps closed as decisions:** D-16-1-D lock semantics (the offline CHILD takes `LOCK_EX`; roots `LOCK_SH|NB` fail fast; `0600` against a local DoS), D-16-1-E worker cap vs idle connections (cap 16; read deadline ≠ handler budget; futures never cancelled), D-16-1-N lib module + `BinPrivateOps` (in-process proofs kloc-free), **NEW D-16-1-P** exit-code table (401 ⇒ 77, not 4, which collides with `uninstall` `not_found`), **NEW D-16-1-Q** nine shared-HOME root tests (machine-dependent `EndpointInUse` reds). ACs: AC1 readiness = `Running` not accept; AC4 destructive steps on separate roots (CRL import revokes butler); AC7 `stop` accessibility cascade and the `maosctl_smoke.sh` negative leg. Epic: contradicted clauses REPLACED rather than annotated (AC2 `Handle`, two-guards, `:2687`; AC3 `stop`/`run`; provenance Line 5), false row-3 wording corrected, kloc figures re-booked, the applied `:953-958` and "15 call sites" corrected. |
| 2026-09-13 | **ROUND-TABLE (§15), operator-ratified (*"approved"*).** Nine forks closed; three premises changed: the offline lock was keyed to the wrong store (Sally), the door is reachable by every bare Worker via env inheritance + same uid (Vex, measured), and FR51(a)(d) had no mechanism behind the claim (Mary, John). NEW D-16-1-R/S/T/U; D-16-1-C/D/E/P/Q amended; exit code 75; `control.json` versioned + scheme-qualified; per-Spirit serialization; HOME-isolation lint. Spec edits applied across epic 16 (Closes, NEW `16-6` row + section, Dependencies, 16-2/16-4/16-5 notes, exit narrative), epic 17 (17-1 AC1/AC3), index (7 stories), DAG, sprint-status (NEW `16-6` row; retro row OWES FR51(a)(d)). §16 goal/drift check added at the operator's request. Gates: `check-exit-commands` PASS, `check-epic-close-coherence` PASS. Status stays `ready-for-dev`. |
| 2026-09-13 | **RE-BASELINE + VALIDATION ROUND 2 + AC4 SPIKE (§17), operator-requested readiness pass.** Baseline `a6ed282c` → `f72b557b` (`subcommands.rs` +16, `kloc.toml` +11, `discipline.yml` +21; every moved cite content-verified; gate sweep green incl. `check-dev-record-completeness` now PASS). Trap 16 (the door holds the audit channel open through `Arc<scheduler>` → `capability` → `audit_tx`; shutdown order). Validator: 6 must-fix + contradictions in the round-table additions → store lock set, `StoreInUse`, `operation_id` mechanics via a maos-bin TL row and `maosctl audit query`, CI decoy for HOME isolation, `SpiritBusy`, 503/exit mapping, ADR gate extension, FR51(c) → 19-2, epic 16-1 ACs brought up to the round-table (rule-9 drift), 16-5/16-6/17-1 fixes. Spike: AC4 rewritten to five roots; NEW D-16-1-V (re-revoke typed, `maos-capability`), D-16-1-W (hot-swap-only, forward-only, faithful butler successor), D-16-1-X (CRL bytes, anchor at boot); §11 rows 10–11; Traps 17–23; budget re-booked (≤ +559 / +891 / +436 / +10). Status stays `ready-for-dev`; §17 lists the author rulings (ratified: *"approved"*). |
| 2026-09-13 | **CONSISTENCY CHECK (§17 V-16…V-22), third fresh reader on the round-2 text.** HIGH: lock files inside store directories would red `erasure_uninstall_13_5b` → the lock set now flocks the directories' own handles (no file created, proofs dir created early, parent of a non-directory). `SpiritBusy` moved into the port (`submit(cmd, deadline)`); 500 ms retry for children too; `DoorUnresponsive` 69; decoy/floor scoping; cold-swap wording (maos-bin map vs kernel admission); 16-4 purge note; 8 cites and 12 contradictions fixed; `maos-capability` ≤ +11; spike evidence preserved in `16-1-evidence/spike/` with root E's unmeasured fast-idle timing stated. |
| 2026-09-13 | **FOURTH READ (§17 V-23…V-36), operator: *proceed*.** No ship-blocker; directory flock probed on Linux. Fixed: boot-time local DoS stated + `0700` dirs; `LockUnavailable` 78 (NFS/permissions/`/var/lib`); `SpiritBusy`/`HandlerStillRunning` decided by a shared atomic CAS, not a timer; durable order keyed to the configured endpoint (env pair or `control.json`); deterministic held-exclusive vector; `SpiritBusy` never-runs proven on the real port; epic 16-1 AC2/AC5 and 16-4 purge note (inode re-check) amended; dedup by `(dev, ino)`; `XDG_DATA_HOME` isolation; reason-keyed re-revoke keeps `CliSubprocessExit` rows; cites and wording. Author rulings still await operator ratification (§17). |
| 2026-09-14 | **OPERATOR RATIFICATION (*"approved"*).** All §17 author rulings ratified as presented: store lock set on directory handles with `StoreInUse`/`LockUnavailable`; CI decoy; `SpiritBusy` by atomic CAS, `DoorUnresponsive`, never bypassing a connecting door; FR51(c) → 19-2 AC3; reason-keyed re-revoke (D-16-1-V); hot-swap-only/forward-only/faithful successor with cold-swap → retro (D-16-1-W); CRL bytes + anchor at boot (D-16-1-X); the boot-DoS residual on pre-existing `0755` directories. No open question; nothing owed before T0. |
| 2026-09-14 | **DEV COMPLETE (opus-5). 68/68 tasks, 8 ACs.** The operator surface is TRUE: against a live `maos run spirits/butler/manifest.toml`, `maosctl pause butler` exits 0 and `maosctl spirit inspect butler` reports `Paused` read from `SpiritControlBlock::current_state` through the door, with `posture: Cautious` read from the same `PolicyTable` snapshot `evaluate_with_posture` reads. Both falsifiers were RUN: removing `scheduler.pause` reds AC1 on `lifecycle_state` while the journal row still lands; a bare `Butler::default()` successor completes the hot swap and silently loses the halt. 12 fake one-shot arms deleted and the lifecycle arm reduced to `uninstall`; `MAOS_ONE_SHOT_MODES` 48 → 32 (10 + 22 `smoke-*`); zero production `insert_pending_with_metadata`; the hardcoded fallback port retired. **Five story premises measured WRONG in dev and re-ruled** (the `LOCK_SH` probe cannot see a live daemon; `{principal}` as a path segment cannot carry an email-shaped principal; the kernel's transition table permits `Paused → Running` under `start`; terminal codes cannot ride the HTTP status; the butler successor arm compiled unreachable). **T7's 10 s drain traced to its cause** — `delegation_leg → Mailbox → ScbTracker → SCB map → butler → orchestrator → capability → audit_tx`, plus watchdogs awaited after the writer — now 11–19 ms with zero drain timeouts over three runs. D-16-1-Q re-measured from the tree: 20 door-root spawners unisolated, not nine; the floor itself was caught clearing on a COMMENT and tightened to require an assignment. `--plan` kept over the door rather than dropped (the guard would have had nothing to read). **Gates:** `check-kernel-baseline` PASSED `changed == 0` (24474 == 24474, 98 files) · `kloc-check` PASSED, every crate inside headroom, **zero ceiling edits** · `check-env-contract`, `check-workspace-count` (55), `check-epic-close-coherence`, `check-exit-commands` (PASS, 26 resolved / 4 owed) · `decision_adrs_and_provisioning` 3/3 with the 405 anchor flipped and proven red · `cargo fmt --all --check` clean · **`cargo test --workspace --no-fail-fast`: 4325 passed, 0 failed.** ⚠ AC4 roots B and D (signed-CRL-over-the-door and the researcher token double-revoke) have implementation and unit/contract coverage but NO end-to-end vector — stated in the Completion Notes for the review net's runtime layer. |

---

## 14. Open questions for the operator

**None open.** All three were closed at the 2026-09-13 round-table (§15): sizing WHOLE (F-A), FR9 `load` →
new row `16-6` (F-B), offline durable arm confirmed with the lock corrected (F-C). The `stop` refusal was
ratified in the same approval.
