---
story_key: 20-12-test-cost-budget-and-review-execution-contract
status: ready-for-dev
updated: 2026-10-23
baseline_commit: f1db3356
depends_on: "`17-9-capability-issuance-quota-window` (`done` 2026-10-23 — its review ran the reduced contract this story makes durable). `20-3a-gate-honesty` (`done`) for the `EvidenceState`/`BindingClass` shape a new gate must use."
blocks: "`18-1-butler-inference-seam-and-conformant-mcp` by operator pull-forward ruling 2026-10-23 (review-cost-diagnosis Phase 2); `17-10`/`17-11` are reviewed under AC5's contract once it lands."
spec_alignment: "Epic §20-12 written 2026-10-23 from measurement (no prior epic text). Confidence rules: verb-first (AC1/AC2 are commands), size from measurement (every figure below is measured, not estimated), cite-or-cut (every AC cites file:line)."
kernel_grant: "NONE. kernel-Δ 0; `crates/maos-kernel-core/src` untouched; `check-kernel-baseline` must stay at 26361 / 95 files."
kloc_grant: "xtask +200–350 forecast (two gates + budget loader + fixture builder); xtask is at zero headroom → exact measured grant at the landing commit, the 15-1 R-11 formula (`measured + 35` is NOT used; strict measured+1 on the aggregate, exact on the crate). Test files under `tests/` are uncharged."
model_grant: "NONE."
review: "bmad-code-review under THIS story's AC5 contract (focused suites + enrolled gates; workspace once in CI). Dev model declared in the Dev Agent Record."
---

# 20-12 — Test-cost budget and review execution contract ◆

Status: ready-for-dev

> **The capability:** *The workspace suite proves the product in minutes, not hours; no test compiles the tree; every
> expensive test is named, budgeted and owned; a reviewer re-runs what the change touched and the matrix runs once in
> CI; a story's proof is one green run and one planted-red run.*

**Closes:** the lane's measured throughput bottleneck (`_bmad-output/planning-artifacts/review-cost-diagnosis-2026-10-22.md` §1–§3) — pulled forward before `18-1` by operator approval 2026-10-23.

**Does NOT close, and says so:**
- The dev host's conditions (swap 33 GB full, iowait 70–80 %, 531 zombies under a `sleep infinity` PID 1, two agent sessions on one `CARGO_TARGET_DIR`) — operator workstation actions, diagnosis §4.
- `maos audit query --spirit` FR4 classification on the 17-3d/17-9 test root → `20-3c` (`deferred-work.md:1192`).
- The watchdog refused-poll observer (kernel hook) → `epic-17-retrospective` (`deferred-work.md:1193`).
- Test COUNT reduction. This story makes tests cheap and named; deleting coverage is `20-3b`'s gate-retirement scope.

## Story

As **the operator who reviews every story and pays for every CI minute**,
I want **the test suite to cost what its assertions are worth — no test that builds the tree, every slow binary budgeted with an owner, a local run that skips what the host cannot provide, and a review that re-runs the touched crates instead of the world**,
so that **a story's review closes in one session and the lane's measured 2.36 days/story is an upper bound, not a floor.**

## Acceptance Criteria

*(Epic §20-12 verbatim; tightened only where measurement names a file.)*

1. **AC1 (command).** `cargo nextest run --workspace --locked` on CI finishes with **≤ 300 s** of test time (Σ per-binary `finished in`; today **1,871 s** over 618 binaries, `17-8-evidence/t12-workspace-test.log.gz`), every binary under its budget in `xtask/test-budgets.toml` (default 30 s; exceptions carry `reason` + `owner`), and `cargo run -p xtask -- check-test-budget --junit <path>` is green on the junit nextest produced. Proven red by planting a 31 s sleep in an unbudgeted binary. `xtask/tests/kernel_pin_content_hash_16_0.rs:902-916` `a_composite_gates_json_stdout_is_machine_clean` asserts the JSON writer over a stub `Report` after `xtask/src/check_escape_detector.rs` is split into `legs → Report` and `Report → stdout`; the file's 24 tests finish in ≤ 2 s (today 1,259 s).
2. **AC2.** `cargo run -p xtask -- check-test-cost` reds any `Command::new("cargo")` / `env!("CARGO")` under `crates/*/tests`, `xtask/tests`, `tests/` without an `// expensive: <reason>` marker on the preceding line; every marked test is `#[ignore = "expensive: …"]` and listed in `xtask/test-budgets.toml [expensive]`; one CI job `expensive-tests` runs exactly that list (`--run-ignored ignored-only`). The **34** files (inventory in §Dev Notes) are dispositioned one by one: fixture binaries are built once by `cargo run -p xtask -- build-test-fixtures` (a CI step before the suite; locally a missing fixture fails with that command in the message), tests resolve them via `MAOS_FIXTURE_DIR` / `CARGO_BIN_EXE_*`; gate integration tests call the gate's library function on a fixture tree. Proven red by planting an unmarked `Command::new("cargo")`.
3. **AC3.** Local profile: with `CI` and `GITHUB_ACTIONS` unset, every substrate-dependent test (`tests/harness/t3_substrate.rs:9-32` and the other `declared_ci()` users — 20 files — plus cgroup2 delegation, `strace`, Python Ed25519, prebuilt `maosctl`) prints one `SKIP: <prerequisite>` line and passes; with `CI=true` the fail-closed path is byte-identical (proven both ways). The dev container stops exporting `CI=true` into interactive shells (measured 2026-10-23: `env | grep ^CI` → `CI=true` in this container; the 61 environment reds of `17-9-evidence/dev/workspace-test-summary.txt:1-7` become 0).
4. **AC4.** CI contract in `.github/workflows/discipline.yml`: four-image matrices remain only where the image is the subject (`:1722,1790,1826` two-image; `:3398,3455,3493` four-image — each kept or narrowed with a one-line reason in the job); every other matrixed job runs one image; `workspace-test-suite` (`:4948-5071`) runs nextest with `--no-fail-fast`, uploads junit, and `check-test-budget` is in `aggregate.needs`. `docs/testing/proof-contract.md` states the rule once: **proven-red on ONE image; green on the matrix once; a story owes one run on its final tree plus one planted-red run on one image.** 17-8's three runs × four images (6.3 job-hours, `17-8-evidence/ci/run-c1-attempt2.json`, `run-c2.json`, `run-c3.json`) is the "before".
5. **AC5.** The review re-execution contract is config: `_bmad/custom/bmad-code-review.user.toml` `persistent_facts` gains — *focused suites for the touched crates and the gates those crates enrol; the workspace suite runs in CI, not at review; mutation re-plants are recorded once by the dev pass and repeated at review only for a disputed finding; the four read-only layers are unchanged.* `_bmad/custom/bmad-dev-story.user.toml` (new) states the dev pass records its proven-red once under `<story>-evidence/`. `xtask/src/check_dev_record_completeness.rs` accepts a review section that cites focused suites (verify by reading it: no workspace-run requirement exists today — if none, AC5 records that; no gate change).
6. **AC6.** `20-12-evidence/` records before/after: workspace test time (1,871 s → measured), this story's own CI job-hours under AC4, and the first measured review wall-clock (this story's review, start/end timestamps in the Dev Agent Record). Figures copied into `prd/project-scoping-phased-development.md` DELTA-2026-10-22's cost paragraph.

## Tasks / Subtasks

- [ ] **T0 — Baseline** (AC1, AC6): install `cargo-nextest` (pin the version in `rust-toolchain.toml`'s neighbour `xtask/tool-pins.toml` if one exists, else the CI `cargo install --locked cargo-nextest@<ver>` step); run `cargo nextest run --workspace --locked --no-fail-fast` locally with `CI` unset; save per-binary timings → `20-12-evidence/t0-baseline.json`; confirm the 1,259 s / 275 s / 76 s / 22 s binaries from the diagnosis.
- [ ] **T1 — Kill the 67 %** (AC1): split `check_escape_detector.rs` so `run()` = `collect_legs() → Report` and `emit(report, json: bool)`; `a_composite_gates_json_stdout_is_machine_clean` builds a stub `Report` (one green leg, one red leg) and asserts `emit(.., true)` parses as JSON and `emit(.., false)` does not; delete the subprocess call. Re-measure the file (≤ 2 s).
- [ ] **T2 — Inventory and disposition the 34 `cargo`-in-test files** (AC2): table in §Dev Notes → one of {fixture-prebuilt, library-call, expensive-marked}. Build `cargo run -p xtask -- build-test-fixtures` (`worker-cli-fixture`, `maosctl`/`maos` dev binaries, WASM guests `guests/recall-probe` + `guests/compat-echo-2-0`, `equiv-native-twin`), writing `MAOS_FIXTURE_DIR` manifest; tests read it. Mark the genuinely expensive ones (`worker_crash_corpus_16_3` 100 Workers, `worker_hang_corpus_16_3` 60, `t_11_3_scale_churn`, `cross_region_live`) `#[ignore = "expensive: …"]` and list them.
- [ ] **T3 — `check-test-cost` gate** (AC2): xtask gate + `gate-registry.toml` row (`BindingClass` blocking at the current phase; `EvidenceState` per 20-3a); proven red by a planted unmarked `Command::new("cargo")` in a throwaway test file (`xtask/tests/check_test_cost_proven_red.rs`, the 15-1 `proven_red.rs` shape).
- [ ] **T4 — Budget file + `check-test-budget`** (AC1): `xtask/test-budgets.toml` (`default_seconds = 30`, `[[exception]] binary/seconds/reason/owner`, `[expensive]` list); `.config/nextest.toml` (`slow-timeout = { period = "30s", terminate-after = 4 }`, junit on); gate reads junit, reds over-budget without exception; proven red by a planted 31 s sleep.
- [ ] **T5 — Local SKIP profile** (AC3): `tests/harness/ci.rs` gains `fn skip_unless(prereq: &str, present: bool) -> bool` used by every `declared_ci()` site; `.devcontainer/` stops exporting `CI=true` to interactive shells (find the source: `docker-compose.yml`/`Dockerfile`/`container-entrypoint.sh` — `grep -n '^CI' .devcontainer/*` was empty on 2026-10-23, so it is set by the harness or the compose `environment:`); prove both ways.
- [ ] **T6 — CI contract** (AC4): review the six matrixed jobs; narrow with reasons; `workspace-test-suite` → nextest + junit upload; enrol `check-test-budget`; write `docs/testing/proof-contract.md`; add the `expensive-tests` job.
- [ ] **T7 — Review contract in config** (AC5): the two `_bmad/custom/*.user.toml` facts; read `check_dev_record_completeness.rs` for any workspace-run requirement (record the finding).
- [ ] **T8 — Evidence + DELTA** (AC6): before/after table; copy into the PRD DELTA cost paragraph; `kloc-check` exact grant for xtask at the landing commit; `check-dev-record-completeness`, `check-epic-close-coherence`, `check-exit-commands` green.

### Review Findings

<!-- Filled by bmad-code-review under AC5's contract. -->

## Dev Notes

### Measured facts this story stands on (2026-10-22/23)

| Fact | Value | Source |
|---|---|---|
| Workspace test time | 1,871 s over 618 binaries | `17-8-evidence/t12-workspace-test.log.gz` (`finished in` sum) |
| Single worst binary | `kernel_pin_content_hash_16_0` 1,259 s (67 %); other 23 tests 1.14 s | measured in isolation 2026-10-23 |
| Share of top 21 binaries | 1,765 s (94 %) | same log |
| Files with `Command::new("cargo")` under tests | 34 | `grep -rlE 'Command::new\("cargo"\)' crates/*/tests xtask/tests tests` |
| `declared_ci()` / `require_t3` users | 20 files | grep 2026-10-23 |
| Local workspace reds with `CI=true` | 61, all environmental | `17-9-evidence/dev/workspace-test-summary.txt:1-7` |
| CI job-hours for one story's exit-line proof | 6.3 (3 runs × 4 images) | `17-8-evidence/ci/run-c{1,2,3}.json` |
| CI `workspace-test-suite` wall | 24m50s | `17-2-evidence/t5-workspace-fixed-results.json` |
| Target dir | 847 GB debug; 12,837 executables = 667 GB | `du`, `ls deps` 2026-10-23 |
| `cargo nextest` | not installed | `cargo nextest --version` → "no such command" |

### The 34 files (disposition is T2's deliverable; starting classification)

`crates/maos-audit/tests/fr4_full_mediation_test.rs` · `crates/maos-bin/tests/{f4_pairing, participant_recall_17_8, recall_17_3d, spawned_scope_17_3c, two_host_delegation_2b, worker_crash_corpus_16_3, worker_hang_corpus_16_3, worker_resource_limits_17_2, worker_supervision_16_3}.rs` · `crates/maos-cli/tests/dep_kernel_core_free_test.rs` · `crates/maos-corpus-gen/tests/{red_team_integration, secret_redaction_integration}.rs` · `crates/maos-escape-detector/tests/common/mod.rs` · `crates/maos-exec-deps/tests/resolver_17_6.rs` · `crates/maos-kernel-core/tests/bridge_t2_route_17_6.rs` · `crates/maos-telemetry/tests/otel_gates.rs` · `crates/maos-wasm-host/tests/{t2_sandbox_kill, recall_runner_17_3d, equiv_harness}.rs` · `xtask/tests/{abi_diff_integration, calibrate_integration, check_corpus_integration, check_judge_config_integration, check_multi_provider_drift_integration, check_unsafe_integration, corpus_staleness_integration, coverage_matrix_integration, example_spirit_regen_integration, loom_integration, trial_attestation_proven_red, empty_kernel_integration, j1_crosshost_1b_proven_red, service_boundary_*, kernel_pin_content_hash_16_0}.rs` (re-derive the exact list at T2; the grep is the source).

Classification rule: **fixture-prebuilt** when the test needs a binary (worker fixture, maosctl, WASM guest/runner, native twin); **library-call** when the test invokes a gate that is a library function in xtask (`check_*::check()` on a temp tree — the 16-0 file already does this for 23 of 24 tests); **expensive-marked** only for real workloads (100-Worker crash corpus, 60-Worker hang corpus, 30/100-host meshes, live Postgres).

### Design decisions (open for the dev; defaults given)

- **D-20-12-A nextest vs libtest.** Default nextest: per-test process isolation, `slow-timeout`, junit. libtest alternative needs a custom timing harness. Pin the version in CI.
- **D-20-12-B fixture builder shape.** Default `xtask build-test-fixtures` writing `target/maos-fixtures/manifest.json`; tests read `MAOS_FIXTURE_DIR` (default that path). Cargo artifact-dependencies are nightly-only — not used.
- **D-20-12-C budget default 30 s.** Measured: 597 of 618 binaries already finish under 5 s. 30 s leaves room for real daemon boots.
- **D-20-12-D `CI=true` source.** Not in `.devcontainer/*` by grep; find where the interactive shell inherits it and remove only that path. The harness's `declared_ci()` semantics are unchanged.
- **D-20-12-E matrix narrowing.** Keep four images where the test exercises the kernel/seccomp/cgroup/WASM runtime of that image; one image otherwise. Each narrowed job carries a comment naming the subject.

### Governance

- xtask zero headroom (`xtask/kloc.toml`) → exact measured grant at the landing commit; `_aggregate_hardfail` strict measured+1.
- `gate-registry.toml` rows for `check-test-cost` and `check-test-budget`; `check-ship-gate-completeness` count 44 → 46 (update its expectation in the same commit, the 20-3a way).
- `check-exit-commands` reads epic AC1 text: AC1's command tokens (`cargo nextest run`, `check-test-budget`) must resolve or be declared owed by this story — run the gate at T8.
- Rule 11: the review section of every later story says "focused suites" and names them; the workspace run is CI's.

### References

- `_bmad-output/planning-artifacts/review-cost-diagnosis-2026-10-22.md` (why; §1–§5)
- `_bmad-output/implementation-artifacts/17-9-capability-issuance-quota-window.md` § Review Findings (the first reduced-contract review, 2026-10-23)
- `xtask/src/check_escape_detector.rs:216-232,303-320,360-394,465-500` (the nested legs)
- `xtask/tests/kernel_pin_content_hash_16_0.rs:902-916` (the 67 % test)
- `tests/harness/ci.rs`, `tests/harness/t3_substrate.rs:9-32`
- `.github/workflows/discipline.yml:1722,1790,1826,3398,3455,3493,4948-5071`
- `_bmad/custom/bmad-code-review.user.toml:21-28`

## Evidence (`_bmad-output/implementation-artifacts/20-12-evidence/`)

- `t0-baseline.json` — per-binary nextest timings before.
- `t8-after.json` — per-binary timings after; CI job-hours; review wall-clock.

## Dev Agent Record

### Agent Model Used

_(filled by dev-story)_

### Debug Log References

### Completion Notes List

### File List

### Change Log

- 2026-10-23 — Story created from measurement (routing audit + review-cost diagnosis), epic §20-12 written the same day; operator approval 2026-10-23 to pull forward before `18-1`. Status → ready-for-dev.
