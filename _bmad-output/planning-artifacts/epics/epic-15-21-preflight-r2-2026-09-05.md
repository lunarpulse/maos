# Preflight R2 — Epics 15–21 (2026-09-05)

**Pinned at:** `9f920180` (commit "correct-course R2: Fork C, eight confidence rules, Epics 15-21"). **Method:** seven adversarial scouts, one per epic, dispatched sequentially; every claim in each epic file checked against the tree (citations, verbs, foundations, kernel-Δ, kloc, hermeticity, sizing, forks, dependencies, and whether Round 2 fixed what the first preflight found). Gates and tests were RUN, not inferred (`check-empty-kernel`, `check-service-boundary`, `kloc-check`, `check-env-contract`, `check-kernel-baseline`, `coverage-matrix`, `demo-j1 --skip-build`, `t_14_2a_post_grace_journal`, the `air-gap` feature build). Per-epic notes with the full evidence tables and OLD→NEW edit lists: `_bmad-output/planning-artifacts/review-2026-09-04/preflight-r2/epic-15.md` … `epic-21.md`.

**Question answered:** can we confidently and safely achieve what the seven epics promise, as written?

## 1. Answer

**Not yet as written. All seven epics are `needs-rework`; none is `blocked`.** Every seam the epics name is real (the kernel APIs, the halt registry, the replay port, the T3 argv builder, the wasmtime host, the registry protocol, the TOFU store, the provisioning action) — Round 2 removed the category errors of Round 1. What survived the rewrite is narrower and fixable: exit blocks that a machine cannot run yet, a handful of ACs standing on mechanisms with zero callers, inventories counted from memory, and budgets that add up only on paper.

| Epic | Verdict | Confidence (R2 target → measured) | Duration (stated → measured) | Claims audited (FALSE / PARTIAL / TRUE) | Note |
|---|---|---|---|---|---|
| 15 Foundations | needs-rework | 75 → **55** | 2–3 wk → 3–5 wk | 61 (6 / 14 / 41) | `epic-15.md` |
| 16 One Daemon, One Door | needs-rework | 70 → **48** | 4–5 wk → 3.5–5.5 wk | 58 (9 / 16 / 33) | `epic-16.md` |
| 17 Workers + third-party form | needs-rework | 60 → **38** | 4–6 wk → 5–8 wk | 61 (9 / 24 / 28) | `epic-17.md` |
| 18 Spirits That Think | needs-rework | 65 → **40** | 4–6 wk → 5–8.3 wk (4–6.5 on two tracks) | 52 (9 / 13 / 30) | `epic-18.md` |
| 19 Founder Loop | needs-rework | 60 → **40** | 4–6 wk → 4.5–6.5 wk | 44 (11 / 19 / 14) | `epic-19.md` |
| 20 Ship It | needs-rework | 55 → **42** | 5–7 wk → 5–8.5 wk | 49 (9 / 21 / 19) | `epic-20.md` |
| 21 Multi-host on live substrate | needs-rework | 55 → **42** | 5–7 wk → 7.8–12.6 wk serial (5.5–8.5 wk with 21-2 ∥ 21-3 ∥ 21-4; 21-5 adds 1.3–2) | 80 (11 / 25 / 44) | `epic-21.md` |

**Lane:** stated 28–40 weeks; measured **34–54 weeks** serial, ~30–48 weeks with the parallel tracks the notes identify (15-5 beside 15-1..4, 18-3 beside 18-1, the operator lane throughout).

**Trajectory.** First preflight (Epics 15–20 as first written): 8–35. Round 2 as written: 38–55. Projected after the §K edits below are applied and the five decisions in §4 are taken: 55–70. The gain from Round 1 to Round 2 is real and came from Fork C and cite-or-cut; the remaining gap is not design, it is execution discipline at planning time (§3).

## 2. What Round 2 fixed (confirmed true, do not re-check)

- **No assumed Spirit host.** No story assumes the subprocess Spirit form; rust-inproc first-party / WASM third-party / cli_wrapper agents is consistently applied. (E17 scout: Fork C consistency holds; the honest caveat that first-party rust-inproc Spirits run unsandboxed in the daemon is stated.)
- **Every named seam exists** — 41/61 (E15), 33/58 (E16), 28/61 (E17), 30/52 (E18), 14/44 (E19), 19/49 (E20) claims confirmed with file:line; the four red gates + 1 test red reproduce exactly; the eight `CURRENT_PHASE` constants are where the epic says; ADR-060..063 numbers are free; `release.yml` defects are real; all 18 crates in the 15-2 table exist; the E13 "~20 crates escape kloc" flag is resolved at HEAD (only `spirits/*` and `examples/*` are excluded, by design).
- **Operator rows exist** (`ops-provisioning-secrets-and-accounts`, `ops-external-cohort-and-pen-test`, `ops-brew-tap-and-aur-publication`, `ops-ga-tag-after-holds`) and the epics route paid/human legs to them in most places.
- **Sizing is within ~30 % of measurement** for six of seven epics (Round 1 was off by 2×).
- **A spike precedes the WASM build** (17-3a) and the registry/vetting/air-gap machinery credited to Epic 20 is real (the `air-gap` feature compiles at HEAD; vetting crypto incl. revoke exists).

## 3. What survived the rewrite (cross-cutting, with the fix)

1. **Rule 1 cannot be mechanised as designed.** `maos` (crates/maos-bin) has no clap: argv is hand-parsed (`main.rs:1653`), there is no `--help`, and `maos --help` boots the daemon until SIGTERM (observed). `check-exit-commands` as specified in 15-3 AC3 would hang or vacuously pass on every `maos` token; `maos eval`, `maos uninstall`, `maos run --epic/--replay/--clock/--incidents` are all unresolvable. **Fix:** 15-3 first gives `maos` a real `--help` from the hand parser's verb table (or a machine-readable `maos --verbs`), then the gate resolves against it. This is the single highest-leverage edit in the lane.
2. **Exit blocks were written, not run.** E16: `maosctl halt resolve … --provided-context` does not parse (real shape `--spirit <s> --kind provided-context --text`), `maosctl posture shift` is `posture <s> --shift`, line 2 is a REPL turn; E17: four non-existent tokens (`worker-sandbox.toml`, `MAOS_FEATURES`, `examples/ts-spirit`, `maos run <x>.wasm`); E18: wrong cassette path, three undeclared flags; E19: the target state ("ABSENT = {two-host-signed-run} only") cannot occur — that beat is INDETERMINATE forever and `demo-j1` is red at HEAD; E20: `maos-registry-server` is a 7-line stub that exits 1, line 2 is a bash syntax error, `researcher@1.0` does not exist and is rust-inproc (which 17-3b AC4 refuses). **Fix:** rule 1 gains a planning-time leg — an exit block is saved only after it was executed or dry-parsed at HEAD, with the red recorded; `check-exit-commands` runs on epic files, not only on CI.
3. **Assumed foundations, second generation.** 16-3 (`handle_crash` returns `NotLoaded` for a pid without an SCB; Workers have no SCB and no `task_assignments_in_flight` record), 17-1 (the production Worker spawn is bare `Command::new`; T3 is asserted at admission, never applied; an egress allowlist is not expressible with podman argv → host-side proxy), 17-3b (the WIT world imports nothing; no production path launches a WASM Spirit; `forms` rejects everything but `rust-inproc|subprocess`), 18-4 (`notification_acceptance_log`, `Butler::with_clock`: zero occurrences; the cited `SystemTime::now()` is a display stub; `jb3_self_tuning_halt` already runs green), 19-4 (Orchestrator has no scalar port; `Butler::morning_digest` has zero production callers; the ambiguity tag has two spellings), 20-4 (`MAOS_DEFAULT_PROVIDER` and `xtask/model-currency.toml` do not exist — both are hotfix-lane rows outside every epic chain). **Fix:** each §K rewrite names the missing mechanism as its own AC with the crate and line count; 19-3 and 17-1 move to FLAG-Winston with figures.
4. **Inventory by recollection persists.** "19 one-shot verbs" is 18 sites/21 values; "13 test sites, hello-spirit 7" is wrong (the 7 are `cohort-a2a-daemon`); "23 absent-pass gates" is 19 (+2 borderline); "4 red gates" is **5** — `check-j1-loopback-delegation` is red and Blocking at HEAD (suffix allowlist `_2a.rs` catches `cert_rotation_trigger_14_2a.rs`) and no story owns it; "16 `other` symbols" is 17 rows/16 unique (fine); "19 signature-changed rows" are ~9 signature changes + ~8 genuinely removed symbols, and regenerating the baseline for the latter contradicts the gate's own additive rule. **Fix:** every count in an epic file carries the grep that produced it.
5. **The budget does not add up.** maos-bin's 15-2 allowance (+1200) is claimed by 15-6 (+150–300), Epic 16 (+480–820), Epic 18 (+300–680), Epic 19 (+500–1000), Epic 20 (+100–200) → over-subscribed roughly 2×; xtask gets "net ≤0" while 15-3, 19 and 20-3 need lines and it sits at 41953/41953; maos-domain +300 is claimed by 16-4/16-5 and 21-4 and the crate is already OVER; kernel-core +150 reverses the E13 retro's explicit refusal and omits 15-1's own +2 and 19-3's +3–5; `maos-manifest`, `maos-mcp`, `maos-journey-test`, `maos-bench` have no rows at all. **Fix:** 15-2's table is rebuilt per epic from the notes' §E tables (consolidated in §5 below), and the E13 refusal is either overruled in writing or the FLAG-Winston stories are re-scoped.
6. **Rule 2 still leaks.** `git tag && git push --tags` inside the E15 and E20 exit blocks (network + `RELEASE_SIGNING_KEY`); `curl -L <release>` in E20; cross-workflow `needs:` in 15-4 AC3 does not exist in GitHub Actions. Seven operator legs have no tracker row: live-key J0 (E16), live Worker under the 17-1 profile (E17), nightly cassette re-record and Google Calendar OAuth (E18), `demo-j1 --live-codex` signed take (E19), first signed tag (E15/E20), J4 live incident recording (E21 — Mira/Nash do no inference today, so "recorded turns" must first be paid for). **Fix:** add the seven `ops-*` rows; replace tag/push with a secret-free `release-dry-run` job.
7. **Forks still live inside ACs (rule 7).** 15-1 attribute-vs-whitelist (resolve: attribute — the whitelist is path-scoped and semantically wrong); 15-6 unset-mode default; 16-1 which process binds the door and how `maosctl` discovers it (`maos-cli` may not link `maos-control`, which depends on kernel-core); 17-1 proxy design; 17-3b recall transport (maos-bin side-channel vs kernel bridge); the replay selector (four spellings: `--replay-llm`, `MAOS_REPLAY_CASSETTE`, `MAOS_INFERENCE_MODE`, `--replay`); 18-4 tuning rule and ledger home; 19-4 Butler-in-topology vs daemon render; 19's seven; 20-1 yank-policy home (a knob in kernel `RegistrySection` breaks kernel-Δ 0); 20-3 which ten gates retire. These are the decisions in §4.
8. **Two premises inside the foundation story 15-2 are wrong.** The formula is at `kloc.toml:58-59` (not :82) and the rule at `:61-65` already permits authorized per-crate grants; the "+1200/+700" allowances are a ceiling-rule amendment, not a re-base; four rows already have the headroom asked. 15-2 remains the right first story, but its AC2/AC3 text must change before `bmad-create-story`.

9. **ACs that promise what is already true (a residual stale at birth).** 21-2 AC1/AC3 (the ignored journey tests already run per-commit on a provisioned substrate; `tl-phase-b` is already binding), 18-2 AC1 (the per-class number is already asserted at a stricter floor), 18-4 AC1 (`jb3_self_tuning_halt` is not ignored and runs green), 15-4 AC5 (`Cargo.lock` is already tracked). Each would have been marked `done` without changing behaviour — the same "claim standing in for a control" shape the Epic 12 retro named, now inverted. **Fix:** an AC that names a state must cite the command that shows the state is *false* at HEAD (proven-red) before it may be written.

## 4. Decisions that gate confidence (one decision per fork; recommended resolution from the notes)

| # | Fork | Epic/AC | Recommended resolution (scout) | Why it matters |
|---|---|---|---|---|
| D-A | Who binds the door | 16-1 AC2 | `maos run`/`maos shell` daemon process binds loopback POST; `maosctl` discovers via `MAOS_HOME/control.json` (endpoint + bearer minted by `maos init`); `maos-cli` gets an HTTP client without linking `maos-control` | 16-1/16-2/19-2 and the J0 scene all depend on it |
| D-B | Egress allowlist + credential | 17-1 AC2/AC3 | Host-side proxy (kernel-minted scoped token → vendor key at the proxy), `--network=none` stays; `env_clear` test inverted | Only mechanism that keeps the raw key out of `/proc/<pid>/environ`; ADR-061 content |
| D-C | WASM recall transport | 17-3b AC1/AC2 | maos-bin side-channel service over a unix socket the runner connects to (the runner's stdio IS the kernel bridge and stays unchanged; kernel-Δ 0), priced by 17-3a | The WIT has no imports; a kernel-bridge relay breaks kernel-Δ 0 |
| D-D | One replay selector | 15-6 / 18 / 19 | `MAOS_INFERENCE_MODE={record,replay,live}` + `MAOS_REPLAY_CASSETTE=<path>`; retire `--replay-llm`; no `--replay` flag | Four spellings today; every Epic 18/19 exit line uses a different one |
| D-E | Digest render path | 19-4 AC3 | Daemon renders via `Butler::morning_digest` (zero callers today) without Butler joining the topology | Removes the Epic 18 dependency from the founder loop |
| D-F | Kernel headroom | 15-2 AC1 | Overrule the E13 retro refusal in writing with per-story figures (15-1 +2, 16-5 +5–25, 17-1 +65–130, 17-2 net ≤+10, 19-3 +3–5, 21-5 negative) | Three FLAG-Winston stories cannot land under 18933/18933 |
| D-G | Absent-pass gate list | 20-3 AC1/AC6 | The 19 named gates; ten retirees named with arithmetic; `BindingClass` scoped to the 40 `[[ship_gate]]` rows | Unbounded story otherwise |
| D-H | I9 exemption path | 15-1 AC2 | Attribute (single-line form, +2 kernel lines, re-pin 24474) | Whitelist is path-scoped; semantically wrong |
| D-I | Env registry home | 21-4 AC2 | Keep the registry as a kloc-free data file read by `check_env_contract`; do not relocate 479 lines into maos-domain | +750 does not exist; 15-2's +300 is spent |
| D-J | Orchestrator move home | 21-5 AC1 | A named leaf crate for the 817 lines, ordered after 21-4; still optional | A negative re-pin needs a destination |

## 5. Consolidated kloc asks (from the notes' §E tables, tokei code at HEAD)

| Crate | HEAD (lines/ceiling/headroom) | 15-2 grants | Asks by epic | Verdict |
|---|---|---|---|---|
| maos-bin | 17109 / 17109 / 0 | +1200 | 15-6 +150–300 · 16 +480–820 · 18 +300–680 · 19 +500–1000 · 20 +100–200 | **over-subscribed ~2×** → +2500–3000 or re-scope |
| xtask | 41953 / 41953 / 0 | net ≤0 | 15-3 gate +150–250 · 19 +150 · 20-3 matrix generator + BindingClass | needs a named row; retirements must be arithmetic |
| maos-domain | 8695 / 8644 / −51 | +300 | 15-1 (+51 red) · 16-4/16-5 +40–90 · 18 +50–100 · 21-4 env registry ≈ +750 | over-subscribed unless D-I (data file) is taken |
| maos-kernel-core | 18933 / 18933 / 0 | +150 | 15-1 +2 · 16-5 +5–25 · 17-1 +65–130 · 17-2 ≤+10 · 19-3 +3–5 · 21-5 ≈ −1000 | short unless 21-5 lands early or the grant grows |
| maos-cli | 5270 / 5270 / 0 | +400 | 16-1 HTTP client for 21 verbs +700 · 20-1 `spirit install` | +700 (E16 note) |
| maos-manifest | ~100 headroom | none | 17: `[network]`, `pids_max`, `wasm-component` (schema v5 + N-1 compat) | add +150 |
| maos-mcp | 999 / 1100 / 101 | none | 18-1 initialize/session/bearer +80–150 | add +300 |
| maos-journey-test | 493 / 593 / 100 | none | 16-2 +50–120 · 18-1 fixture MCP server +150–300 | add +400 |
| maos-wasm-host | 1057 / 1129 / 72 | +400 | 17-3b +200–400 | fits |
| maos-eval, maos-control, maos-secrets, maos-providers, maos-a2a-core, maos-iac, maos-audit | see notes | as tabled | fits after 15-2 | — |

## 6. Per-epic findings

### Epic 15 — Foundations (55/100, needs-rework, 3–5 wk)
Top: (1) 15-3 AC3 `maos` has no `--help` (rule 1 lever); 15-3 AC2 conflates the ship-ladder phase (`gate_common.rs:163-166`, `v1_5`) with the delivered phase (`tests/phase-config.toml`, `v0.1-alpha`) — measured 145 coverage-matrix violations if unified; (2) 15-4 AC3 cross-workflow `needs:` impossible; exit line 3 non-hermetic; (3) 15-1 AC2 fork, AC6 cause contradicted by measurement (passes alone and at 2 threads), 15-6 "every Spirit" vacuous (two inference consumers) and its new default reds 10 existing sites. 18 edits in §K.

### Epic 16 — One Daemon, One Door (48/100, needs-rework, 3.5–5.5 wk)
Top: (1) 16-3 rests on an SCB Workers do not have; `FaultCause::SignaledByKernel` already exists and `CrashCause` is domain, not kernel; (2) exit block flag shapes wrong, `spirit inspect` has no lifecycle-state route; (3) "one daemon" topology undecided (three composition roots; `maos-cli` cannot link `maos-control`). Inventories wrong (verbs, test sites, routes). No `ops-live-key-j0-leg` row; 16-1 lacks ◆. 18 edits.

### Epic 17 — Workers and the third-party form (38/100, needs-rework, 5–8 wk)
Top: (1) Worker spawns bare; AC1's observables come from two different sandboxes and none reaches the TL as a kind-8 row; allowlist needs a host proxy; AC3 reds a pinned CI test; (2) `log.recall` has no wire; no production WASM launch path; four non-existent exit tokens; (3) 17-2's path is the root layout (three mechanisms, not two); `cgroup_ceiling_smoke` in no CI job. Three hidden manifest schema bumps (`deny_unknown_fields`, schema v4, N-1 compat). 20 edits.

### Epic 18 — Spirits That Think (40/100, needs-rework, 5–8.3 wk)
Top: (1) 18-4 built on nothing (ledger, `with_clock`, cited line, cited test all wrong; cadence is kernel `idle_watchdog.rs`); (2) 18-2's "first real number" false — `corpus_halt.rs:296-305` already asserts ≥0.90/≥0.85 in CI through the real orchestrator, and the metric is halt arithmetic, not a model; (3) exit block path/flags/four-way replay fork; line 3 cannot fail (fallback at `researcher/src/lib.rs:930`). `5400` has no source; no conformant MCP fixture exists. 22 edits.

### Epic 19 — Founder Loop (40/100, needs-rework, 4.5–6.5 wk)
Top: (1) exit state unreachable; `demo-j1` red at HEAD via `check-j1-loopback-delegation` (fifth red Blocking gate, unowned); (2) 19-3 needs +3–5 kernel lines (no cwd on `BridgeSpawnSpec`) and "real Worker by default" rewrites a fail-closed control (`live_agent_gate`, fixture-only grant, the very test it proposes to rewrite); no substrate for concurrent Workers; (3) 19-4's halt and digest stand on zero-caller mechanisms. Dependency line over-tight by two waves (hermetic path needs 15-2 + 15-6 + 16-1 only). 21 edits.

### Epic 20 — Ship It (42/100, needs-rework, 5–8.5 wk)
Top: (1) exit block not runnable (stub server, syntax error, wrong flags, wrong artifact shape, non-existent `researcher@1.0`); (2) 20-4 stands on two hotfix-lane rows outside every epic chain; (3) 20-3 unbounded (19 not 23 gates; 59 gates lack `BindingClass`; ten unnamed retirees against zero xtask headroom; generated matrix cannot house 198 hand-authored entries). `TlYankObserver` does more than journal; a yank knob in kernel `RegistrySection` breaks kernel-Δ 0. 18 edits.

### Epic 21 — Multi-host on live substrate (42/100, needs-rework, 7.8–12.6 wk serial (5.5–8.5 wk with 21-2 ∥ 21-3 ∥ 21-4; 21-5 adds 1.3–2))
Top: (1) 21-3 AC1 observes a route that does not exist — `GET /v1/a2a/peer-versions` is not served; `GET /v1/cohort/peer-versions` (`maos-control/src/lib.rs:347`) returns manifest-version rows with no fingerprint and no route reads the pin store; "typed to the trait" needs a `maos-a2a-core` change (zero headroom); `maos-persistence` is an 8-line placeholder; (2) 21-2's premise is inverted — the four ignored `cross_team_crossing_13_6b` tests and the five `t_12_4a` mechanism tests already run per-commit as legs of `check-multi-tenant-loom` / `check-loom-substrate-drift` / `check-cohort-mesh` on a provisioned 3-DB substrate (`discipline.yml:3054,3077`), and the `tl-phase-b` leg is already binding (`AdvisorySubstrate` hard-fails when the substrate is present, `gate_common.rs:216-233`); (3) 21-4 AC2 is arithmetically impossible — `env_contract.rs` is 479 tokei lines, maos-domain is −51 with the +300 already drawn, relocation plus ~55 rows needs ≈ +750. 21-1 stands on four undeclared mechanisms (`--incidents`, the fixture, a clock from 18-4, an A2A router in the `maos run` topology path — Mira and Nash load into one scheduler with no router, `main.rs:3797-3847`) and Mira/Nash do no inference, so "recorded turns" require a paid recording with no `ops-*` row. Dependencies: `17-3b → 21-2` is a vestigial WASM edge; 20-2 has no consuming AC; 15-2/15-3/15-5 (ADR-063 does not exist yet)/15-6/18-4 missing; "Closes D1" contradicts the proposal (D1 → 20-3). Wrong-seam citations: `router.rs:502-507` already has a `detail` param, `transport.rs:1341` is `:1333`, `check_cohort_mesh.rs:604` is `:674`, the "≤25K kernel-crate-set / ADR-057" citation traces to ADR-040's J1 latency budget in microseconds. Fifteen forks enumerated in §H. 22 edits.

## 7. Recommended next step

**Round 3 is mechanical, not conceptual.** The seven §K lists hold 139 OLD→NEW edits with the AC id and the evidence; the eight decisions in §4 each have a recommended resolution. Apply in this order:

1. **Decide §4 D-A..D-H** (one message from the operator suffices; defaults = the recommended column).
2. **Apply the §K edits to the seven epic files** via `bmad-correct-course` (Batch), add the six `ops-*` rows and the 15-2 budget table from §5, add `check-j1-loopback-delegation` to 15-1's red list, and dry-parse every exit block against HEAD before saving (record the reds inline).
3. **Land 15-2 (the budget) first, then 15-3's real `maos --help`, then 15-1** — maos-bin has zero headroom, so the verb table cannot land before the grant. Until `maos` can be asked what verbs it has, rule 1 is a promise, not a control.
4. **Re-scout only the changed sections** (one scout, all seven files, citations only) — expected confidence after the pass: 15·70 / 16·60–65 / 17·50–55 / 18·55 / 19·55–60 / 20·55 / 21·55.

The lesson of this round, in one line: **a rule that says "run the command" is only a control if the command was run while the file was being written.** Round 2 ratified the rule and then wrote seven exit blocks from memory; the rewrite must carry its own execution trace.
