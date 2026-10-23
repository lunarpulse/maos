# Review Cost Diagnosis — 2026-10-22

**Complaint:** a story review now takes hours and saturates CPU/RAM; two months ago it did not.
**Sources:** story records Aug–Oct (`13-6`, `14-1`, `14-2a`, `15-6`, `16-1`, `16-6`, `17-1`, `17-2`, `17-3d`, `17-4`, `17-8`, `20-3a`), evidence logs (`17-8-evidence/t12-workspace-test.log.gz`, `17-8-evidence/ci/run-c*.json`, `17-2-evidence/t5-workspace-fixed-results.json`), `.github/workflows/discipline.yml`, the test tree, and live measurements on the dev host at 15:20 KST. The omp session archive on this host holds no maos sessions; the records above are the history.

## Findings, ranked by measured weight

### 1. One test is 67 % of the workspace suite — and it is a nested `cargo` run
`cargo test --workspace` (17-8 run, 618 binaries): **1,871 s of test time; 1,259 s (67 %) is `xtask/tests/kernel_pin_content_hash_16_0.rs`** (24 tests). Measured today in isolation: the other 23 tests finish in **1.14 s**; `a_composite_gates_json_stdout_is_machine_clean` (`:902-916`) shells out to `xtask check-escape-detector --json`, which runs **`cargo test -p maos-escape-detector` legs with different feature sets plus `cargo build --release`** (`xtask/src/check_escape_detector.rs:223-232,311-320,375,469-478`). Under today's host load it had not finished after 30 min. The test asserts that stdout parses as JSON; it does not need a real gate run.

21 of 618 binaries account for **94 %** of test time (1,765 s). Next: `worker_supervision_16_3` 275 s (`cargo build -p worker --bin worker-cli-fixture` inside the test, `:46-47`), `fr4_full_mediation_test` 76 s, `post_surface_16_1` 22 s.

### 2. Nested `cargo build`/`cargo test` inside tests — 33 files
Tests that compile the project from inside the suite: `spawned_scope_17_3c.rs` (three `cargo build --release` sites, `:52-101,1642`), `participant_recall_17_8.rs:71-74` and `recall_17_3d.rs:57-60` (release guest + runner), `equiv_harness.rs:666-670` (release native twin), `worker_supervision_16_3.rs:46`. Each feature/profile combination is a separate rebuild; the workspace run therefore compiles the tree several times in several profiles. Consequence on this host: `/mnt/build/cargo/debug` = **847 GB**, `deps/` holds **12,837 executables = 667 GB**; every workspace run relinks ~600 test binaries of ~100 MB.

### 3. The review contract moved from reading to re-execution (Sept 25 →)
The generic skill mandates three read-only layers (`.claude/skills/bmad-code-review/steps/step-02-review.md:11-37`); the user customization adds Test-Infra only for non-Claude/Codex authors (`_bmad/custom/bmad-code-review.user.toml:27`). Everything heavier is story/epic text:
- **Two- and four-image CI matrices mandatory** (ubuntu 24.04/26.04, ±arm) for sandbox/egress/WASM jobs since 2026-09-25 (`epic-17…md:64`; `discipline.yml:1722,1790,1826,3398,3455,3493`).
- **"Proven green / red / green in CI"** per exit line (17-8 D-17-8-05 A under FW-10): three full runs × four images. 17-8: `wasm-host-tests` 31–42 min per image, **3.0 + 1.5 + 1.8 = 6.3 job-hours** for one story's proof (`17-8-evidence/ci/run-c1-attempt2.json`, `run-c2.json`, `run-c3.json`); plus an attempt-1 flake rerun (`17-8…md:407`).
- **Mutation re-plant at review**: 17-1 re-planted 14 mutations, each diff → crate check → RED output → restore (`17-1…md:304`).
- **Workspace suite run locally during review with `CI=true`**, which fails closed on every missing host prerequisite: **61 failures per run** that are all environment (T3 substrate, cgroup2 delegation, `strace`, Python Ed25519, stale `maosctl`) — `17-9-evidence/dev/workspace-test-summary.txt:1-7`, `17-8…md:405`. Each run is ~25 min (`17-2-evidence/t5-workspace-fixed-results.json`: `workspace-test-suite` 24m50s in CI) and produces 61 red rows the reviewer must triage every time.
- August reviews already had 3–5 layers and 40–50 findings (`13-6`, `14-2a`); the layer count is not the delta. The delta is substrate-backed execution and CI proof rounds.

### 4. The dev host is swapping and has no init
Measured 15:21: 32 cores, 62 GB RAM, **swap 33 GB with 150 MB free**, `vmstat` iowait **70–80 %**, load **50–68**, swap-in/out 14–20 MB/s. Processes inside this container use 0.7 GB RSS — the pressure is other containers/sessions on the same host. PID 1 is `sleep infinity`: **531 zombies** (440 `git`, 31 `sccache`, 12 `rustc`) are never reaped. Two agent sessions (this one and the 17-9 dev session) share one `CARGO_TARGET_DIR`, serialising on cargo's lock and invalidating each other's artifacts. These facts explain "hours, CPU, RAM" more than any single test does, and they are not in any story.

### 5. Suite growth is real but secondary
Passed tests 4,119 (Sep 4) → 4,964 (Oct 22), +20 %; binaries 495 → 618; CI 160 → 172 jobs, `aggregate.needs` 130; 449 Rust integration files, 118 `#[ignore]`. Growth multiplies §1–§3; it would not by itself turn minutes into hours.

## Fix — one story, pulled forward, plus three operator actions

**Story `20-12-test-cost-budget-and-review-execution-contract`** (tracker row added `backlog`; recommended pull-forward before `18-1`, the 20-3a precedent). Scope, all measurable:
1. **No `cargo` inside `#[test]` in the workspace suite.** Tests that need a fixture binary use `CARGO_BIN_EXE_*` / `[[bin]]` build-deps or a prebuilt artifact; gate tests call the gate's library function on a fixture tree. `a_composite_gates_json_stdout_is_machine_clean` asserts on the JSON writer with a stub report. A `check-test-cost` xtask gate reds any `Command::new("cargo")` under `tests/` without an `// expensive: <reason>` marker, and every marked test is `#[ignore = "expensive: …"]` and listed in ONE CI job.
2. **Per-binary time budget.** Workspace suite runs under `cargo nextest` with `slow-timeout` and a budget file (`xtask/test-budgets.toml`): default 30 s per binary, named exceptions with a reason and an owner; the gate reds a new exception. Target: workspace test time ≤ 300 s on CI (today 1,871 s).
3. **Local profile.** `CI` unset locally ⇒ substrate-dependent tests print one `SKIP: <prereq>` line and pass; the fail-closed behaviour stays under `CI=true`. The review runs `cargo nextest -p <touched crates>` + the touched gates; the workspace run happens once, in CI, on the final tree.
4. **CI proof contract.** "Proven red" = one image; "green" = the full matrix once. Four-image matrices only on jobs whose subject is the image (sandbox/cgroup/WASM runtime); everything else one image. Review-time mutation re-plants are recorded by the dev pass (the story's own proven-red step) and not repeated at review unless a finding disputes them.
5. **Evidence.** Before/after: workspace test time, CI job-hours per story, local review wall-clock; recorded in the story and carried into the DELTA.

Kernel-Δ 0. xtask delta measured at creation. Owner: Murat (suite + gate) + Winston (CI contract).

**Operator actions (workstation, not a story):**
- Give the devcontainer an init (`"init": true` / tini) so zombies are reaped; stop the host swapping (33 GB swap full) — fewer concurrent sessions or more RAM; **one cargo session per `CARGO_TARGET_DIR`** (give each agent session its own target dir or run them in sequence).
- Reclaim `/mnt/build/cargo/debug` (847 GB; `cargo sweep --time 7` or purge `deps/` executables older than the current pin) and keep sccache.
- Until 20-12 lands: run `cargo test --workspace` locally **without `CI=true`** and with `--skip a_composite_gates_json_stdout_is_machine_clean`; that alone removes 61 environmental reds and 21 of 31 minutes.
