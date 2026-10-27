# 16-1 daemon post surface and verb retarget — historical sections archive 2026-10-27

Verbatim relocation of historical sections of `16-1-daemon-post-surface-and-verb-retarget.md` (done story) on 2026-10-27, by operator ruling. Each section below holds the body that followed the same-named heading in the story, copied **verbatim**; the original line span at relocation time is recorded in a comment under each heading. The story keeps each heading with a one-line pointer here, and keeps all frontmatter, Acceptance Criteria, Tasks, Decisions, Dev Agent Record and Review Findings sections unchanged.

## 1. 🔴 `maos init` EXITS 1 ON EVERY CLEAN HOME, AND EVERY INIT TEST MASKS IT

<!-- original lines 75-99 -->

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

## 2. 🔴 THE EPIC'S AC1 SUBJECT CANNOT BE NAMED BY ANY VERB AT HEAD

<!-- original lines 105-126 -->

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

## 3. 🔴 THE DOOR IS BOUND TOO EARLY, BY THE WRONG ROOTS, ON A FALLBACK PORT

<!-- original lines 132-158 -->

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

## 4. 🔴 13 OF 16 "STATE-MUTATING" VERBS ARE FAKE OR HARMFUL ON A RUNNING DAEMON

<!-- original lines 164-186 -->

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

## 5. 🔴 "TL-ONLY READERS" ARE 2 OF 5

<!-- original lines 192-202 -->

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

## 6. 🔴 FOUR VERBS ARE CORRECT OFFLINE — "DAEMON DOWN ⇒ ERROR" WOULD BREAK THE ERASURE SPINE

<!-- original lines 208-214 -->

For **`forget`, `uninstall`, `legal-hold-release`, `governance-admit`**: their effect is durable and
correct when no daemon holds in-memory state; GDPR erasure (FR45/FR65) cannot require booting a root;
`crates/maos-bin/tests/erasure_uninstall_13_5b.rs` pins the one-shot contract `0/3/4/5` ↔
`erased/held/not_found/failed`, and `crates/maos-cli/tests/uninstall_exit_codes_13_5b.rs:17-29` pins
that maosctl forwards it. But offline is **wrong while a daemon runs** (false `Erased`, TL-lock
hazard), and a PID file or connect probe cannot settle "is a daemon running" without a race. → D-16-1-D:
an OS lock settles it.

## 7. 🔴 `control.json` HAS NO AGREED HOME, AND ADR-062 KEEPS ONE WRITER

<!-- original lines 220-233 -->

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

## 8. 🔴 THE SERVER CANNOT CARRY POST SAFELY, AND A GATE REDS ON THE FIRST `405`

<!-- original lines 239-261 -->

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

## 9. 🔴 TESTS: 2 OF THE "NINE FAKE-`maos`" FILES ARE FAKE; SEVEN RUN THE REAL BINARY

<!-- original lines 267-299 -->

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

## 15. Round-table 2026-09-13 — nine forks, operator-ratified (*"approved"*)

<!-- original lines 910-927 -->

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
