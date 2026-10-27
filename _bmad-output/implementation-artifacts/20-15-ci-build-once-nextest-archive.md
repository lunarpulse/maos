---
story_id: '20.15'
story_key: 20-15-ci-build-once-nextest-archive
created: 2026-10-27
context_commit: 84e0e659
baseline_commit: 84e0e659
security_sensitive: false
kernel_grant: "none — kernel-Δ 0 by construction (no file under `crates/maos-kernel-core/src` is touched; `check-kernel-baseline` PASSED 26367 / 95 at `84e0e659`, unchanged after the creation spikes). No prototype needed."
kloc_grant: "RULED 2026-10-27 (D-20-15-08 A): xtask 46364 (exact, zero headroom, `xtask/kloc.toml:339`) — forecast +15–30 [INFERENCE] (D-03 A, ruled 2026-10-27; the 0 forecast applied only to the not-taken D-03 B/C) → own Epic-20 measured grant at landing, STOP above +38, NOT the D19-1 Epic-19 envelope (46364 → 46566 covers 19-1…19-7 forecasts only, `epics/epic-19-founder-loop-w4.md:41,129,135`). `.github/workflows/*.yml`, `.config/nextest.toml`, `scripts/*.py`, `tests/harness/*` and `xtask/tests/*` are not kloc-charged (`xtask/src/kloc_check.rs:299-317`)."
review: "bmad-code-review under docs/testing/proof-contract.md. Easy-tier story (no kernel grant, no security-critical AC; CI infrastructure): creation anthropic/claude-opus-5-5; dev/finisher openai-codex/gpt-6.1-sol; Blind+Edge merged on opus-5.5, Acceptance and Test Infrastructure on glm-5.3 — EXCEPT that the Test-Infrastructure layer re-reads every consumer job against the 'no silent fallback' guardrail on opus-5.5 (a silent in-job compile is this story's one failure mode that a green run hides)."
---

# Story 20.15: Build-once CI with `cargo nextest archive`

Status: in-progress

<!-- Created 2026-10-27 at 84e0e659 by bmad-create-story under the lighter creation contract (no party-mode preflight). Prep story before 19-1 (E18 retro R2, operator ruling 2026-10-27; vehicle of E17-A2/E18-A2). D-20-15-01…08 RULED 2026-10-27 — D01 C, D02 B, D03 A, D04 A, D05 A, D06 A, D07 A, D08 A (+38 xtask STOP); every recommendation accepted; dev may start. Measured, not read: the per-job compile classification of the whole landing run 40396208547 (189 jobs, run log zip + jobs API, 18-0 AC10 method extended to every job — `20-15-evidence/creation/`), two local `cargo nextest archive` spikes (0.9.149, the CI pin), `cargo tree -e features` workspace-vs-package resolution, the repo's Actions cache inventory. One cargo process; CARGO_TARGET_DIR=/mnt/build/cargo; `env -u CI -u GITHUB_ACTIONS`. No crate source changed; `intent-lineage-coverage-report.md` reverted after a workspace nextest run touched it. -->

> **The capability:** *a discipline run compiles the code under test once per architecture, not once per job. A producer job builds the test binaries, the fixtures and the binaries the bucket jobs execute, packs them, and every consuming job runs exactly those artefacts — with the compiler made unavailable, so a missing archive is a red job, never a silent 10-minute rebuild. The gate set and every pinned count stay what they are; the run gets cheaper.*

**Closes:**
- E18 retro **R2** / action **E18-A2** (`epic-18-retro-2026-10-27.md:78,88`; `sprint-status.yaml:540-543`) and the E17-A2 measurement loop (18-0 AC10, `18-0-evidence/ci/compile-vs-test-40080259088.json`).
- Epic-20 §20-15 (`epics/epic-20-ship-it-w5.md:242-252`; stories row `:73`). Prep before 19-1 (`epics/epic-19-founder-loop-w4.md:156,209`).

**Does NOT close, and says so:**
- The ~75 single-image `ubuntu-latest` jobs that compile a few test binaries each: measured run-wide test-profile compile outside the buckets is ≈ 1.56 job-h over 75 jobs, and a per-job archive download eats it (net ≈ −0.04 h at 60 s per consumer, `20-15-evidence/creation/job-pricing-40396208547.json` `runwide_test_only_ubuntu_latest`). Out of scope by measurement.
- The run-wide `xtask` recompile (≈ 1.99 job-h over 72 jobs, ≈ 100 s each; a prebuilt `xtask` artifact nets ≈ +1.65 h) and the smoke jobs' release `maos` builds (2.53 job-h over 32 jobs, six feature sets) — listed under D-20-15-07 as priced levers; only taken if the operator widens scope.
- Jobs that must keep compiling (§Dev Notes "Must keep compiling"); the stable performance runner (20-7 / ADR-068); self-hosted runners; local dev-host cost (20-12).

## Story

As **the operator paying for every landing run**,
I want **each discipline run to compile the tree once per architecture and hand the built artefacts to the jobs that test them**,
so that **a landing run costs fewer job-hours and finishes sooner, with exactly the same gates, counts and proofs.**

**Contract:**
- Epic-20 §20-15 (`_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:242-252` after the in-place corrections below; stories row `:73`; `:76` create-story may tighten, not add uncited ACs — three ACs kept).
- Retro `epic-18-retro-2026-10-27.md:27,78,88,112`; tracker `sprint-status.yaml:312` (row) and `:540-543` (E18-A2).
- `docs/testing/proof-contract.md:8-20` (one landing run + one proven-red run on one image), `:70-91` (nextest is the workspace runner; fixtures built once by `build-test-fixtures`; staleness rule `:88-91`), `:100-114` (image matrices kept because the **runtime** is the subject).
- 18-0 AC10 method (`18-0-evidence/ci/compile-vs-test.method.py`), 20-12 (nextest + fixture manifest + `check-test-budget` junit).

## Current-tree corrections — read before implementing

Measured at `84e0e659` (= `271e2a33` for every file this story reads; the landing run 40396208547 ran that tree).

| Premise (epic / retro) | Fact at `84e0e659` and consequence |
|---|---|
| Why: "74 % of CI wall is compile — 13167 s of 17737 s across the five largest jobs" (run 40080259088) | **Holds** (`compile-vs-test-40080259088.json` total 13167/17737 = 0.742; `build-test-fixtures` the largest compile step in 4 of 5 buckets — epic-17-exit's is exit line 2). Re-measured on the landing run 40396208547 with the same method: buckets 13137 s compile of 18133 s wall (72 %). **But only 3051 s of the buckets' 13137 s is test binaries** (`test` profile); fixtures 6490 s (dev 2558 + release 3932), release CLI/runner 3076 s, debug `maos`/`maosctl` 520 s. A test-binaries-only archive (the epic as written) removes 0.85 job-h and nets **+0.05…+0.46 job-h** per run → D-20-15-01. |
| Why: "a landing run ≈ 13.7 job-h" | **Holds**: 189 jobs, 13.71 job-h, 73.9 min wall (20:30:12Z → 21:44:07Z). Run-wide compile **9.30 job-h (67.8 %)**: release-other 2.53, xtask-dev 1.99, fixtures 1.96, test 1.70, dev-other 0.79, bench 0.32 (`20-15-evidence/creation/job-pricing-40396208547.json`). |
| (unstated) jobs reuse a warm cache | **Every compile is cold.** All 180 jobs that use `Swatinem/rust-cache@v2` (168 uses in `discipline.yml`) logged "No cache found." in 40396208547; the repo's Actions cache holds 8.63 GB in 5 entries against the 10 GB limit (`gh api …/actions/caches`, 2026-10-27). Post-job cache saves cost 0.69 job-h per run for nothing. Consequence: per-job compile = full dependency graph; transport must not be `actions/cache` (D-20-15-06); rust-cache lever D-20-15-07. |
| AC1: "`epic-17-exit`/`epic-18-exit`" consume a test archive | `epic-18-exit` (`discipline.yml:2284-2548`) runs **no** `cargo test`: it builds `maos` and `maosctl` debug (`Build the root` / `Build maosctl`, 133 s) and runs scenes. It is a consumer of debug binaries only (D-01 C). `epic-17-exit` (`:1985-2283`, 2 × x64) runs 3 `cargo test` suites (`--features network`) plus a release `maos`+`maos-wasm-runner` build for exit line 2 (`--features maos-bin/wasm-host`, 1114 s release across both images). |
| AC1: consumers run `cargo nextest run --archive-file` | The bucket jobs run **21 `cargo test` invocations with 21 libtest-format pins** (`test result: ok. N passed;`, `--expected-count N`): wasm-host-tests 13 (`:3991-4198`, e.g. `:4047` 23 passed, `:4087` 2, `:4128` 18, `:4177` 8), cgroup-ceiling-delegated 3 (`:3895,3902,3913`), -undelegated 1 (`:3953`), epic-17-exit 4 (`:2070,2102,2149,2195`); the check-log parser is libtest-only (`scripts/run_cgroup_ceiling_17_2.py:17-18`). Nextest prints `N tests run: N passed`, coloured under `CARGO_TERM_COLOR: always` (`discipline.yml:28`; the 18-7 journey ANSI red, fixed with `--color never` at `:3367-3368`). → D-20-15-05. |
| (unstated) a workspace archive serves `cargo test -p maos-bin …` jobs | **Different binaries.** Workspace feature unification adds tokio `test-util` + `io-std` to maos-bin's graph (`cargo tree -e features -i tokio`, `--workspace` vs `-p maos-bin`); maos-bin's `wasm-host` feature is not unified (`crates/maos-bin/Cargo.toml` `[features]`; `default = ["network"]`). Measured cost of the difference: `cargo nextest run -p maos-bin` after a workspace build rebuilt for ≈ 70 s (92 s wall vs 45 s for the whole `--workspace` run). Consequence: one archive per invocation family, matching today's flags (CR-1). |
| (unstated) archived binaries are relocatable | **107 test files** use compile-time `env!("CARGO_BIN_EXE_*")` (99 × `maos`, 48 × `xtask`, 15 × `maosctl`, …); the absolute build path is embedded (spike: `/mnt/build/cargo/debug/maos-corpus-gen` found in `red_team_integration-*`). 0 files read it at runtime. Nextest's runtime `CARGO_BIN_EXE_*` (≥ 0.9.130; CI pins 0.9.149 at `discipline.yml:640,5492`) does not reach `env!`. → D-20-15-04. |
| (unstated) the fixture manifest travels | **The staleness rule refuses it.** `tests/harness/fixtures.rs:118` (`newer_input`), `:171-172` (entry vs `built_at`), `:210` (artefact vs mtime): any `.rs`/`.toml`/`.wit` input newer than the build refuses the fixture (`xtask/src/build_test_fixtures.rs:18-24`; `proof-contract.md:88-91`). A consumer's fresh checkout makes every input newer than the producer's `built_at`. Standalone probes are built **outside** the target dir (`build_test_fixtures.rs:97-115`, `<probe dir>/target/release/…`), so nextest `archive.include` (target-relative) cannot carry them. → D-20-15-03, CR-2. |
| AC1: "gate set and job count unchanged" | Gate set can stay unchanged; **job count cannot**: one producer per arch (+2) or per image (+4) (D-02). `aggregate.needs` (134 entries, `:5864-6295`) can stay unchanged — 42 jobs (incl. `wasm-host-tests`) already reach it only through ship gates; a red producer skips its consumers and the fan-in gates fail on skipped. |
| (unstated) workflow shape is free to change | 27 test files read `discipline.yml` (grep). Bucket-coupled: `xtask/tests/ci_shape_18_0.rs` (25 bucket/fixture/pin references), `epic17_exit_workflow_17_7.rs` (5), `cgroup_ceiling_workflow_17_2.rs` (1), `crates/maos-kernel-core/tests/cgroup_ceiling_smoke.rs` (8). They change with the jobs, in the same commit. |
| (unstated) nextest pin hygiene | `journey-hermetic-tier-1` installs **unpinned** `cargo-nextest` (`discipline.yml:3339`); the other two installs pin `0.9.149` (`:640,5492`). Producers and consumers pin one version (archive format). |
| Holds | Image matrices and reasons (`discipline.yml:3858-3860,3918-3920,3959-3961`; `proof-contract.md:105-112`); workspace job command (`:5596`, inside `run_cgroup_ceiling_17_2.py run --mode delegated`); junit upload `:5631` → `check-test-budget` `:592-626` (`needs: [workspace-test-suite]`); `.config/nextest.toml` default-filter (kernel-core lib under libtest) and `serial` test group. |

**Epic cites corrected in place (OLD → NEW @`84e0e659`; `epics/epic-20-ship-it-w5.md`):**
- §20-15 Closes · Δ `:244`: "xtask + `.github/workflows/discipline.yml` only" → "`.github/workflows/discipline.yml`, `.config/nextest.toml`, `scripts/run_cgroup_ceiling_17_2.py`, `tests/harness/fixtures.rs`, the workflow-shape tests, `docs/testing/proof-contract.md`; xtask only under D-20-15-03 A (`build_test_fixtures.rs`)".
- §20-15 Why `:246`: holds; appended the landing-run classification (9.30 of 13.71 job-h compile; buckets' test binaries 3051 s of 13137 s; 180/180 rust-cache misses).
- §20-15 AC1 `:248`: "`epic-17-exit`/`epic-18-exit`" annotated (epic-18-exit has no test binaries — debug-binary consumer); "gate set and job count unchanged" → "gate set unchanged; job count + the producer jobs (D-20-15-02)"; consumer runner per D-20-15-05.
- §20-15 AC2 `:249`: "once per image" → "once per image or per arch (D-20-15-02)"; artifact transport per D-20-15-06.
- `epics/epic-19-founder-loop-w4.md:209`: tracker `sprint-status.yaml:311` → `:312` (this story; 20-16 `:312` → `:313`).

## Kernel and kloc pricing — measured at creation

**Kernel:** Δ 0 by construction; `check-kernel-baseline` PASSED 26367 / 95 files at `84e0e659` (before and after the spikes). No kernel prototype (contract: kernel-Δ 0 expected).

**Job-hour pricing** (run 40396208547, every job; producer cost = per arch/image the max (lo) or Σ over job families of the max (hi) of what the consumers stop compiling; consumer overhead 45 s each [INFERENCE until T1]; `20-15-evidence/creation/job-pricing-40396208547.json`, method `job-pricing.method.py`):

| Bundle (D-01) | Removed from 16 consumers | Producers, per image (4) | Net / run, per image | Producers, per arch (2) | **Net / run, per arch** |
|---|---:|---:|---:|---:|---:|
| A — test binaries only (epic as written) | 0.85 h | 0.35–0.59 h | +0.05…+0.30 h | 0.19–0.36 h | +0.29…+0.46 h |
| B — A + the jobs' `build-test-fixtures` artefacts | 2.65 h | 0.91–1.35 h | +1.10…+1.54 h | 0.47–0.81 h | +1.64…+1.98 h |
| C — B + release CLI/runner (`:4205`, epic-17-exit line 2) + debug `maos`/`maosctl` | 3.65 h | 1.46–2.35 h | +1.10…+1.99 h | 0.74–1.34 h | **+2.11…+2.71 h** |

Against 13.71 job-h, D-01 C per arch ≈ −15…−20 % job-h. Wall-clock: 13.71 job-h ran in 73.9 min, i.e. ≈ 11 jobs concurrently on average — wall follows job-hours, ≈ −11…−15 min per run [INFERENCE]. The producer is on the critical path (≈ the wasm-host-tests compile, 20–25 min on 4-vCPU runners [INFERENCE]) and replaces the 23–34-min wasm-host-tests chain; T1 measures it.

**Local archive spikes** (cargo-nextest 0.9.149 = the CI pin):
- `cargo nextest archive --workspace --locked`: 623 binaries (15 non-test), compile 5 m 24 s on 32 cores warm-ish, archive **3,303,584,617 B** (zstd), 12 s to pack.
- `cargo nextest archive -p maos-bin --features network,wasm-host -E 'binary_id(…)'` (the five bucket suites): **136,954,146 B**, 6 binaries (1 non-test, `maos`); it still compiles all 85 maos-bin test targets (filters only shape the archive).
- Archive layout `target/…` relative + `target/nextest/{binaries-metadata,cargo-metadata}.json` + `libdirs`; run from the archive with `RUSTC=/bin/false CARGO=/bin/false cargo-nextest nextest run --archive-file … --workspace-remap . --extract-to <dir>` → 19/19 passed (maos-corpus-gen): **a consumer can make compilation structurally impossible** (CR-3).

**kloc at `84e0e659`** (`kloc-check --json` passed, `over_budget: []`, aggregate 187437; hardfail 187438 `xtask/kloc.toml:718`):

| Row | Measured / ceiling | 20-15 forecast | Consequence |
|---|---|---|---|
| xtask | 46364 / 46364 (`xtask/kloc.toml:339`, zero headroom) | D-03 A: manifest `tree` field + `git rev-parse HEAD^{tree}` + dirty refusal + schema 2→3 in `build_test_fixtures.rs` ≈ **+15–30 [INFERENCE]** (analogue: `unix_now` + `built_at` plumbing, `:448-455`); D-03 B/C: 0 | **D-20-15-08** — own Epic-20 grant |
| everything else touched | workflow YAML, nextest config, Python, test harness, `xtask/tests` | not charged | — |
| aggregate | 187437 | +0…+30 | `_aggregate_hardfail` = measured + 1 at landing |

## Decisions — RULED 2026-10-27 (operator selections: D01 C, D02 B, D03 A, D04 A, D05 A, D06 A, D07 A, D08 A)

| ID | Question | Options | Recommendation and evidence | Ruling |
|---|---|---|---|---|
| **D-20-15-01** | What the producer builds (the bundle) | **A** — test binaries only (`cargo nextest archive`), as the epic is written · **B** — A + the `build-test-fixtures` artefacts the bucket jobs build today (`discipline.yml:3886,3905,3944,3974,2184,5538`) · **C** — B + the release `maos`(wasm-host)/`maos-wasm-runner` builds (`:4205`; epic-17-exit line 2) and epic-18-exit's debug `maos`/`maosctl` | **C.** Measured: test binaries are 23 % of the buckets' compile (3051 of 13137 s); A nets +0.29…+0.46 h/run per arch — below the noise of one re-run. C nets +2.11…+2.71 h; every added artefact is the same SHA, same flags, same profile the job builds today. | **RULED 2026-10-27: C** |
| **D-20-15-02** | Producer granularity | A — per image (4 producers; AC2 as written) · **B — per arch: built on `ubuntu-24.04` and `ubuntu-24.04-arm`, consumed by both images of that arch** | **B.** The matrices are kept because the runtime is the subject, not the compiler (`proof-contract.md:100-112`; `discipline.yml:3858,3918,3959`). Measured: release `maos` links only `libc`/`libm`/`libgcc_s` (`ldd`), so a 24.04 (older glibc) build runs on 26.04 [INFERENCE until T1 runs it on 26.04 both arches]. B halves producer cost (+0.5 h/run vs A under D-01 C). | **RULED 2026-10-27: B** |
| **D-20-15-03** | Fixture staleness in a consumer (fresh checkout ⇒ every input newer than `built_at`, `tests/harness/fixtures.rs:118,171-172,210`) | **A — the manifest records the source tree id (`git rev-parse HEAD^{tree}`; a dirty tree refuses to record) and the harness accepts an entry whose tree equals the checkout's tree; the mtime rule stays for entries without a tree (local); schema 2 → 3; xtask +15–30** · B — the consumer back-dates checkout mtimes before running (`git ls-files -z \| xargs -0 touch -d @1`), workflow-only · C — consumers keep rebuilding fixtures (D-01 A) | **A.** B blinds the control in CI (every input "older" by fiat, whatever was built); A replaces "newer by time" with "built from this tree", which is what the rule means, and keeps the local plant protection (`build_test_fixtures.rs:18-24`). Tree id, not commit: a re-run of the same tree on another commit stays valid. | **RULED 2026-10-27: A** |
| **D-20-15-04** | Compile-time `env!("CARGO_BIN_EXE_*")` in 107 test files (absolute build path embedded — spike) | **A — extract at the producer's absolute path: producer builds with the default target dir under `$GITHUB_WORKSPACE`; consumers `--extract-to "$GITHUB_WORKSPACE" --workspace-remap "$GITHUB_WORKSPACE"`; T1 proves the path is identical on x64 and arm runners** · B — migrate the 107 files to runtime `std::env::var("CARGO_BIN_EXE_…")` (nextest sets it, ≥ 0.9.130) — test-only churn, uncharged | **A.** Zero test edits; the runner workspace path is stable per repo [INFERENCE until T1]. B is the portable form but touches 107 files for no behaviour; route it to a later story only if A fails on a runner. | **RULED 2026-10-27: A** |
| **D-20-15-05** | Consumer runner and the 21 libtest pins | **A — `cargo-nextest nextest run --archive-file … --color never` (AC1 wording; `--test-threads 1`, `--no-capture` for today's `--test-threads=1 --nocapture`); every pin rewritten to the nextest summary (`N tests run: N passed`) with the same N; `run_cgroup_ceiling_17_2.py` `_RESULT` (`:17-18`) accepts the nextest form; marker greps unchanged (`--no-capture` passes stdout through)** · B — execute the archived binaries directly under libtest (path from `target/nextest/binaries-metadata.json`, cwd = package dir — spike: `red_team_integration-* --test-threads=1` → `test result: ok. 4 passed`); pins untouched, nextest only packs | **A.** Nextest is the project runner since 20-12; the bucket suites' process-global state is fixture lookup only (`static FIXTURES: LazyLock` at `spawned_scope_17_3c.rs:45`, `recall_17_3d.rs:49`, `participant_recall_17_8.rs:75`; `static AVAILABLE: LazyLock<bool>` at `recall_17_3d.rs:566`, `live_escape_anomaly_17_10.rs:248` — measured, no `Mutex` serialisation), so process-per-test does not change what they prove. B keeps a second runner shape and hand-rolled env. | **RULED 2026-10-27: A** |
| **D-20-15-06** | Transport | **A — run-scoped `actions/upload-artifact@v4` / `download-artifact@v4`, name `<bundle>-<arch>-${{ github.sha }}`, `retention-days: 1`, `if-no-files-found: error`** · B — `actions/cache` keyed `<arch>-${{ github.sha }}` | **A.** Measured: the repo cache is at 8.63 GB of 10 GB in 5 entries and 180/180 rust-cache restores missed — a 0.14–3.3 GB key per arch per run would evict the rest and could restore across runs. Public repo: artifact storage is not billed (`gh api repos/lunarpulse/maos` → `private: false`). | **RULED 2026-10-27: A** |
| **D-20-15-07** | Scope of the other priced levers | **A — consumers drop `Swatinem/rust-cache` (they compile nothing); nothing else** · B — A + `save-if: false` on rust-cache workflow-wide except producers (−0.69 job-h/run of saves that 180/180 never restored) · C — B + a prebuilt `xtask` artifact for the 72 xtask-gate jobs (net ≈ +1.65 h/run; 111 MB debug binary) | **A** within the epic's scope; B and C are measured, separable and safe, but they widen the story beyond "test jobs" — rule them in only by an explicit scope widening (each would get its own AC and plant). | **RULED 2026-10-27: A** |
| **D-20-15-08** | xtask kloc | **A — own Epic-20 measured grant at landing (expected ≤ +30 under D-03 A; STOP above +38), outside the D19-1 envelope** · B — book it against D19-1 (46364 → 46566) | **A.** D19-1 is Σ(19-x forecasts) × 1.3 (`epic-19…:129,135`) and has no slack for prep stories; 20-16 needs the same answer (D-20-16-07). If both land first, the E19 envelope's base moves from 46364 to the landed figure (same +202) — the operator states which. | **RULED 2026-10-27: A** — own Epic-20 grant, STOP above +38 xtask kloc |
| CR-1 | Archive families = today's invocations | One `cargo nextest archive` per (package set, features, profile) family exactly as the consumer invokes it today: (i) `-p maos-bin --features network,wasm-host` + `-p maos-kernel-core` + `-p maos-wasm-host` for the wasm-host/cgroup buckets; (ii) `-p maos-bin --features network` for epic-17-exit's three suites; (iii) `--workspace` for `workspace-test-suite`, which stays its own producer (it compiles exactly that already; no consumer gains). Never a workspace archive for a `-p` job (feature unification — measured). | — |
| CR-2 | Fixtures and binaries travel as files | A second artifact per arch: the fixture manifest + every artefact path it lists (incl. the standalone probes outside `target/`) and the release/debug binaries of D-01 C, restored at identical absolute paths (`tar -C /`, manifest paths absolute). | — |
| CR-3 | No silent fallback | Consumer jobs carry no `dtolnay/rust-toolchain`, no `rust-cache`, and set `RUSTC: /bin/false` + `CARGO_BUILD_RUSTC: /bin/false` at job level; they invoke `cargo-nextest nextest run` directly (spike: runs with `CARGO=/bin/false`). Any compile attempt is a hard red with the compiler path in the log. A workflow-shape test asserts it (no `cargo build\|test\|run` in a consumer job). | — |
| CR-4 | Unchanged surfaces | Same job names (consumers), same image matrices, same steps' names and pinned counts, same `aggregate.needs`; `check-test-budget` still reads `workspace-test-suite`'s junit. | — |

## Acceptance Criteria

*(Epic §20-15 as corrected at creation; three ACs kept (epic `:76`); D-rows RULED 2026-10-27 — D01 C, D02 B, D03 A, D04 A, D05 A, D06 A, D07 A, D08 A: every recommendation accepted; wording below matches the ruling.)*

### AC1 — command: a landing-shaped run with compile only in producers

1. `discipline.yml` gains one producer per arch (D-02 B) that builds the D-01 C bundle per CR-1/CR-2 and uploads it per D-06 A; the consumers — `wasm-host-tests` (4), `cgroup-ceiling-delegated` (4), `cgroup-ceiling-undelegated` (4), `epic-17-exit` (2), `epic-18-exit` (1, binaries only) — `needs:` their producer, download, and run the archive per D-04 A/D-05 A. `workspace-test-suite` keeps its own build (CR-1 (iii)).
2. Observable: a push run on the story tree where every consumer job's log has **zero** `Compiling`/`Finished … profile` lines and every pin (`test result`/`tests run`, `--expected-count`, marker specs) holds with today's numbers; gate set unchanged; job count = today's 176 keys + producers.
3. Per-job compile-vs-test wall before/after in `20-15-evidence/` by the 18-0 AC10 method (`job-pricing.method.py` over the run).

### AC2 — once per arch per run, nothing else moves (D-02 B)

1. One producer per arch (D-02 B) per run; artifact names carry arch and `github.sha`; consumers download only their arch's bundle.
2. `check-test-budget` (`:592-626`) and `aggregate.needs` (`:5864-6295`) unchanged; the four workflow-shape tests (`ci_shape_18_0`, `epic17_exit_workflow_17_7`, `cgroup_ceiling_workflow_17_2`, `cgroup_ceiling_smoke`) updated in the same commit, plus one new assertion: consumer jobs contain no compiling cargo command and set `RUSTC: /bin/false` (CR-3).
3. One cargo-nextest version everywhere (pin `0.9.149`, incl. `:3339`).

### AC3 — numbers

Total job-hours and wall on ≥ 2 landing-shaped runs, stated against 13.71 job-h / 73.9 min (40396208547), with the per-class breakdown (`job-pricing.method.py`) and the producer's critical-path wall; `20-15-evidence/ci/`.

### AC4 — proven red in CI (rule 11; the epic's AC1 proven-red, kept as its own AC)

One hosted run on `ubuntu-24.04` only (`ci/20-15-verify`, plant diff + sha256 in `20-15-evidence/ci/`): the plant deletes the downloaded archive in `wasm-host-tests (ubuntu-24.04)` before its first run step. Expected red: that step fails naming the missing archive file, and the job log contains no `Compiling` line (CR-3 makes a fallback impossible, the plant proves it). Run id + excerpt recorded.

## Tasks / Subtasks

- [ ] **T0 — Preflight at the dev HEAD.** D-20-15-01…08 RULED; tracker → `in-progress`. Re-anchor: `git diff --stat 84e0e659 -- .github/workflows/discipline.yml .config/nextest.toml scripts/run_cgroup_ceiling_17_2.py tests/harness/fixtures.rs xtask/src/build_test_fixtures.rs`; re-run `job-pricing.method.py` on the latest green landing run if the bucket jobs moved.
- [x] **T1 — Hosted spike** (`ci/20-15-spike`, not merged): one producer per arch + one consumer per image of `wasm-host-tests`; measure artifact up/download + extract wall (replaces the 45 s [INFERENCE]), prove `$GITHUB_WORKSPACE` identical on x64/arm (D-04 A), 24.04-built binaries on 26.04 (D-02 B), `RUSTC=/bin/false` consumer green. Recorded in `20-15-evidence/t1-spike.md`; run 40489534163 proves its selected roundtrip family only, not the full bundle.
- [ ] **T2 — Fixture provenance** (D-03): per ruling; harness + manifest schema; `build_test_fixtures` unit tests; plant: a stale tree id refuses (local, `plants.md`).
- [ ] **T3 — Producers** (D-01, D-02, CR-1, CR-2, D-06): jobs, archive commands per family, fixtures/binaries tarball, uploads.
- [ ] **T4 — Consumers** (D-04, D-05, CR-3, CR-4): needs/download/extract, runner switch, 21 pins rewritten 1:1 with the same N, `run_cgroup_ceiling_17_2.py` regex + `scripts.tests.test_cgroup_ceiling_17_2`, rust-cache removed, `RUSTC: /bin/false`.
- [ ] **T5 — Shape tests** (AC2.2): update the four readers; new consumer-no-compile assertion; nextest pin unified.
- [ ] **T6 — Proven red** (AC4) and **numbers** (AC1.3, AC3): two landing-shaped runs + the plant run; evidence files.
- [ ] **T7 — Docs and record:** `proof-contract.md` gains "Build once (20-15)" (producer, families, no-fallback rule); kloc re-pin exact if xtask moved (D-08); Dev Agent Record, File List, Change Log; tracker. Do not edit `intent-lineage-coverage-report.md` (revert if a run modifies it). No commit or push without the operator.

## Dev Notes

### What changes, by file

| Area | Files | Today → after |
|---|---|---|
| Workflow | `.github/workflows/discipline.yml` `:1985-2283` (epic-17-exit), `:2284-2548` (epic-18-exit), `:3855-3954` (cgroup ×2), `:3955-4258` (wasm-host-tests), `:3339` (nextest pin) | per-job cold compile → producer + consumers |
| Nextest | `.config/nextest.toml` | + archive profile only if `archive.include` is used for target-relative fixtures (CR-2 may make it unnecessary) |
| Fixtures | `tests/harness/fixtures.rs`; `xtask/src/build_test_fixtures.rs` (D-03 A) | explicit `MAOS_FIXTURE_TRANSPORT=1` seals and requires tree/artifact provenance; ordinary own-build CI and local runs retain mtime checks |
| Helper | `scripts/run_cgroup_ceiling_17_2.py:17-44`, `scripts/tests/test_cgroup_ceiling_17_2.py` | libtest summary → + nextest summary |
| Shape tests | `xtask/tests/{ci_shape_18_0,epic17_exit_workflow_17_7,cgroup_ceiling_workflow_17_2}.rs`, `crates/maos-kernel-core/tests/cgroup_ceiling_smoke.rs` | follow the jobs |
| Docs | `docs/testing/proof-contract.md` (new section after `:91`) | — |

### Must keep compiling (classification; never a consumer)

`reproducible-build` (byte identity of two builds, `:60-163`), `release-dry-run` ×2 (target matrix, `:5176-5257`), `check-mock-not-in-release` (release graph, `:372-388`), `windows-check`, `macos-check`, `fuzz-build` (nightly), `abi-diff`/`gen-abi-docs` (nightly rustdoc JSON), every `--no-default-features`/air-gap leg (`check-exit-commands` `:508-540` builds both), bench jobs (bench profile), `expensive-tests` (spawns cargo by definition, `:627-664`), `example-spirit-ts-tests` (TS build, `:1241-1359`), `check-multi-tenant-loom` (whole-package maos-bin under a systemd unit), the 75 single-image test jobs (D-07 / Does NOT close).

### Current state that must be preserved

- Every pinned count and marker spec, by number (21 in the buckets); `--no-capture` so marker lines still reach the log.
- `run_cgroup_ceiling_17_2.py run --mode …` units around each suite (cgroup delegation is the subject); the rootless canary steps (`:3903-3913`, x64 only).
- The four-image matrices and their one-line reasons; `fail-fast: false`.
- Local behaviour of the fixture harness (mtime rule without a tree id).

### Guardrails

- No silent fallback (CR-3); a red producer skips consumers, and skipped is red at the fan-in.
- One artefact family per invocation (CR-1); never substitute a workspace archive for a `-p` job.
- Same SHA only: artifact names carry `github.sha`; never `actions/cache` for bundles.
- `--color never` on every nextest command whose output a pin reads (`CARGO_TERM_COLOR: always`, `:28`).
- One target dir locally; `env -u CI -u GITHUB_ACTIONS`; no second `CARGO_TARGET_DIR`; plants restored byte-identical.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red |
|---|---|---|
| No fallback (AC4, hosted) | delete the archive before the run step in `wasm-host-tests (ubuntu-24.04)` | step red naming the archive; no `Compiling` line |
| Provenance (D-03 A, local) | manifest tree id ≠ checkout tree | harness refuses the fixture, naming the command |
| Shape (AC2.2, local) | add `cargo build` to a consumer job | the new shape assertion reds |

### Tests and commands

```bash
# local; one cargo process; CARGO_TARGET_DIR=/mnt/build/cargo
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p xtask --test ci_shape_18_0 --test epic17_exit_workflow_17_7 --test cgroup_ceiling_workflow_17_2
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core --test cgroup_ceiling_smoke
python3 -m unittest scripts.tests.test_cgroup_ceiling_17_2
MAOS_FIXTURE_TRANSPORT=1 cargo run -q --locked -p xtask -- build-test-fixtures --only maosctl   # requires a clean checkout; manifest carries provenance
env -u CI -u GITHUB_ACTIONS cargo nextest archive --locked -p maos-bin --features network,wasm-host --archive-file /tmp/wh.tar.zst
RUSTC=/bin/false CARGO=/bin/false cargo-nextest nextest run --archive-file /tmp/wh.tar.zst --workspace-remap . --extract-to "$PWD" --color never
cargo run -q --locked -p xtask -- kloc-check --json && cargo run -q --locked -p xtask -- check-test-cost
python3 _bmad-output/implementation-artifacts/20-15-evidence/creation/job-pricing.method.py jobs.jsonl logs discipline.json
```

### Review plan (proof contract `:31-68`)

- Easy tier (no kernel, no security AC): Blind+Edge on opus-5.5; Acceptance and Test Infrastructure on glm-5.3; the no-fallback reading of every consumer job (CR-3) on opus-5.5.
- Reviewer re-runs the shape tests and the helper unit tests; the hosted runs are the dev pass's evidence (not re-run at review).

### Previous-story intelligence

- **20-12:** nextest process-per-test voided 14 in-process serialisations (`serial` group, `.config/nextest.toml`); `build-test-fixtures` + manifest + staleness rule; "CI is compile-dominated: the local test-phase win is ≤ 3 min of a ~13 h run" (13.4 → 12.7–13.0 job-h) — this story is the compile half.
- **18-0 AC10:** the compile-vs-test method and the 74 % figure; reuse the method, extend it to every job.
- **18-7 landing:** `--color never` for a nextest count pin under `CARGO_TERM_COLOR: always` (`:3367-3368`).
- **18-5 landing:** GOVERNED-ATOMIC marker 8 vs 6 in `wasm-host-tests` — marker specs are product pins; keep them byte-for-byte.

### Git intelligence

| Commit | Content |
|---|---|
| `84e0e659` | Epic 18 closed + retro (R2 → this story) |
| `f889b3e4` | 18-5/6/7 done; landing green 40396208547 @ `271e2a33` |
| `271e2a33` | 18-7 landing fixes incl. nextest `--color never` count pin |
| `e1d09f0b` | 20-12: nextest, `build-test-fixtures`, staleness rule, `check-test-budget` |

`.config/nextest.toml`, `tests/harness/fixtures.rs` and `build_test_fixtures.rs` last changed at 20-12 (`e1d09f0b`).

### References

- Epic `epic-20-ship-it-w5.md:73,76,242-252`; epic-19 `:41,129,135,156,209`; retro `:27,58,78,88,112`; tracker `:312,540-543`.
- `discipline.yml:28,60-163,372-388,508-540,592-626,627-664,640,1241-1359,1985-2283,2284-2548,3329-3369,3339,3855-3954,3955-4258,4205,5176-5257,5450-5641,5492,5596,5631,5864-6295`.
- `tests/harness/fixtures.rs:118,171-172,210`; `xtask/src/build_test_fixtures.rs:18-24,97-115,366-455`; `scripts/run_cgroup_ceiling_17_2.py:17-44`; `.config/nextest.toml`; `docs/testing/proof-contract.md:8-20,70-91,100-114`; `xtask/kloc.toml:339,718`; `xtask/src/kloc_check.rs:299-317`.
- nextest archiving (https://nexte.st/docs/ci-features/archiving/), env vars incl. runtime `CARGO_BIN_EXE_*` ≥ 0.9.130 (https://nexte.st/docs/configuration/env-vars/).

## Dev Agent Record

### Story context creation

Created 2026-10-27 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter contract) at `84e0e659`, clean tree.

**Inputs:** direct reads of every cited range; `check-kernel-baseline` PASSED 26367 / 95; `kloc-check --json` passed (aggregate 187437, xtask 46364); jobs API + run log zip of 40396208547 (189 jobs, classified by `job-pricing.method.py`); `gh api …/actions/caches` (8.63 GB, 5 entries); `cargo tree -e features` (workspace vs `-p maos-bin`).

**Spikes** (one cargo process; no source change): `cargo nextest archive -p maos-corpus-gen` (layout; compile-time path embedded; run with `RUSTC`/`CARGO` = `/bin/false` → 19/19); `--workspace` (3.30 GB, 623 binaries, 324 s); `-p maos-bin --features network,wasm-host -E …` (137 MB). `cargo nextest run -p maos-bin` 92 s vs `--workspace` 45 s from the same warm dir (feature-resolution rebuild). Archives under `/tmp` only.

**Revert:** `git status --short` empty after the spikes except `intent-lineage-coverage-report.md`, modified by the workspace nextest run and reverted with `git checkout --`; `check-kernel-baseline` PASSED 26367.

### Agent Model Used

anthropic/claude-opus-5-5 (creation); openai-codex/gpt-6.1-sol (implementation); anthropic/claude-opus-5-5 (independent no-fallback transport audit, incomplete whole-story review).

Model roles for the current review-correction phase are a **SESSION-ONLY substitution** (operator, latest 2026-10-27): coding and high-risk reviews on `anthropic/claude-opus-5-5`, light reviews and docs on Sonnet 5.5; an earlier temporary Terra substitution was superseded. The standing tracked role policy (`docs/testing/proof-contract.md`) is not rewritten, and no claim is made that the parent conversation switched away from Sol.

### Debug Log References

- `20-15-evidence/t1-spike.md`: hosted four-image narrow archive spike, run 40489534163.
- `20-15-evidence/local-smoke-2026-10-27.md`: historical +37 xtask delta (measured pre-mode, before the explicit transport mode; the 46364 -> 46401 arrow in the 2026-10-27 candidate Change Log entry is that historical +37, **superseded** by the corrected +38 split below), focused tests, cost-gate pass, real producer/consumer smokes, review repairs and isolation evidence.
- **Current exact candidate grant (post-correction, 2026-10-27, parent-measured):** 20-15 **+38** = `build_test_fixtures.rs` 547 (producer +39, including `deny_unknown_fields` on the producer manifest) and `check_test_cost.rs` -1, exactly the unchanged **STOP38**; xtask 46364 -> 46402. With the separately allocated 20-16 **+1019** (see its record, within STOP1020) the xtask candidate pin is **47421**; aggregate measured **188494**, hardfail **188495** (totals unchanged by the correction; `xtask/kloc.toml` HISTORY corrected). Candidate only: not a landed figure, E19 rebase, final approval or hosted success.
- Parent verification (2026-10-27): focused xtask suites **142 passed / 6 skipped**, dev-record lib **22 passed**, Python **30 OK**; `check-test-cost` PASS; `kloc-check` passed with no over-budget crate; kernel baseline **26367 / 95** unchanged (tokei kernel 20610); record gates semantically unchanged after the deferred-work archival (100 owner assertions, 22 dispositioned, 186 done stories); `example-spirit-regen` alias smoke WARN with `passed:true`.
- Independent `TransportReview2015`, `BlindEdge2015` and `AcceptInfra2015` identified the rootless canary compile path, mixed summary parsing, checkout-local output, invalid job-level `runner.temp`, tracked-example mutation, missing required-case bindings and provenance downgrade paths. Repairs are recorded in the local smoke; final independent re-review and final-snapshot tests remain pending.
- Hosted plant run 40536260336 (`ci/20-15-verify` @ `f10f0381`): `20-15-evidence/ci/run-40536260336/` (`run.json`, `jobs.jsonl`, `per-job-compile.json`, `totals.txt`, `ac1.2.txt`, `consumer-logs.list`, `plant-job-steps.txt`, `plant-job-excerpt.log`, `epic17-ubuntu-24.04-dirty-excerpt.log`); plant diff and sha256 in `20-15-evidence/ci/plant.diff` / `plant.diff.sha256`. The 191 full job logs are not copied.

### Completion Notes List

The explicit transport mode is bound only to the producer and five selected consumer families and passed into systemd. Producer cleanliness and consumer provenance are mandatory in that mode; ordinary own-build CI/local jobs retain mtime checks. Manifest-owned `maosctl` and `worker-cli-fixture` cannot downgrade on absent or corrupt entries, while unregistered product binaries retain their existing allowance. No arbitrary ignored-file exception was added; ignored `target/tmp` observer scratch is staged under `RUNNER_TEMP` for upload.

Latest focused result before the explicit mode update was **74 Rust and 30 Python passed**; it is not a final-snapshot approval. A corrupt rootless archive really returned 96 with no compiling fallback, but positive rootless execution remains hosted evidence. The mini-Git local smoke is not the required in-place story-tree provenance plant. Full review/retest, in-place plants, hosted missing-archive plant, full-bundle runs and timings remain pending; T1 proves only its selected family.

After the mode handoff, the parent ran the real harness via a std-only driver in a tiny Git fixture: sealed transport and unregistered product controls passed, missing identities and absent owned `maosctl` failed, and CI without the explicit flag accepted a valid unsealed mtime fixture. Details and exit codes are in the local smoke; this is neither the formal in-place plant nor focused Rust/Python or hosted proof.

**Review-correction triage (2026-10-27; reviewers: Opus 5.5 for coding/high-risk, Sonnet 5.5 for light audits, a SESSION-ONLY substitution).** No approval is claimed; the story stays in-progress.
- *Opus Blind+Edge, transport:* one P3 (dead zero-test guard) — **fixed**.
- *Opus, fixtures:* P2 `profile_binary` end-to-end gap — **fixed**; P3 producer unknown-field erasure — **fixed** (`deny_unknown_fields`, the +1 in the producer line count).
- *Sonnet audit of 20-15:* no unresolved finding recorded here.
- *Also accepted and fixed:* Git environment scrub in the provenance tests; consumer archive-family binding shape test with a swap negative; upload `overwrite: true` for same-run reruns; `journey-nightly` nextest pinned to `cargo-nextest@0.9.149`; count-drift RED regression through a child process with a stub cargo; static marker pass arm; removed the std `Command::get_args` assertion; earlier: change-set union (base->HEAD ∪ HEAD->working ∪ untracked, with rename endpoints), captured SKIP lines as diagnostics not substrate skips, obsolete lineage restore removed, add-only tests removed, fixture-runtime child regression, README byte preservation, exact archive-family producer validator.
- *Dismissed or kept, with reasons:* nextest's default 120 s per-test kill is kept for consumers (20-12 test-cost contract; a kill is a fail-closed red); the Python manifest heredoc round-trip is left to the hosted landing run (fail-closed); `tar -C /` artifact extraction shares the job's own trust boundary (PR code), no change; the pin reader's `nextest list` cost is recorded honestly at 20-16 AC3; the pin reader covers recognised count/marker shapes only (contract wording); a TI-6 real-workflow reader test was not added (the prepush runtime exercises it).
- Still not done: in-place plants, the hosted missing-archive plant, two landing-shaped runs, warm/incremental timing, final independent approval, savings figures, and any landed E19 base.

The D-20-15-08 table says the operator states the Epic-19 base, while the D-20-16-07 metadata and change log describe the formula `(landed xtask figure) + 202` as ruled. No landed figure or final rebase is evidenced here; preserve both historical statements rather than asserting a completed rebase.

### Hosted plant run 40536260336 (recorded 2026-10-27; evidence in `20-15-evidence/ci/run-40536260336/`)

Branch `ci/20-15-verify` @ `f10f0381`; plant diff `20-15-evidence/ci/plant.diff`, sha256 `f390dab88fe9864c34f84918027595405789f9cb4e49477b9ec40469c56fa01f` (`plant.diff.sha256`). The run is **red by design**: 191 jobs, 184 success, 7 failure. Not a landing-shaped green run, so it does not satisfy AC3.

- **AC4 — DONE (hosted plant).** `wasm-host-tests (ubuntu-24.04)`: step 7 "AC4 plant - delete the downloaded wasm-host-t2 archive" succeeded; step 8 "Epic-17 exit line 3 — T2 kill and narrow-prctl proof" failed with `error extracting archive .../ci-test-archives/wasm-host-t2.tar.zst` … `No such file or directory (os error 2)`, `rc=96`; steps 9-21 skipped; no `Compiling`/`Finished` line in the job log. Step 22 (`upload-artifact`) also failed because its log files were never written (consequence of the plant, not a separate defect). The other three `wasm-host-tests` legs were green. Excerpt: `plant-job-excerpt.log`; step table: `plant-job-steps.txt`.
- **AC1.2 — zero-compile holds over 15 consumer logs** (`ac1.2.txt`, `consumer-logs.list`): no `Compiling` and no `Finished … profile` line in any consumer log. Producers (`ci-test-producer` x64 and arm64) both success; `cgroup-ceiling-delegated`/`-undelegated` 4/4 each green; `epic-18-exit` green; `epic-17-exit (ubuntu-26.04)` green; `wasm-host-tests` 3 non-plant legs green.
- **First cost sample (AC3 input, NOT the AC3 pair; red run, slightly understated):** 10.79 job-h vs 13.71 baseline (40396208547); wall 43.7 min vs 73.8. Family seconds, run vs baseline: wasm-host-tests 1146 vs 7205; cgroup-delegated 517 vs 3423; cgroup-undelegated 350 vs 2854; epic-17-exit 586 vs 3151; epic-18-exit 20 vs 170; ci-test-producer 3394 (x64 1934, arm64 1460). Critical path: last producer done at 36.6 min; last wasm leg +414 s after it. Compile hours by class (baseline -> this run): release-other 2.53->2.19, xtask-dev 1.99->2.07, test 1.70->1.28, fixtures(release) 1.15->0.27, fixtures(dev) 0.81->0.28, dev-other 0.79->0.71, bench 0.32->0.32; total 9.3 -> ~7.12 (`totals.txt`, `per-job-compile.json`, `jobs.jsonl`, `run.json`).
- **OPEN 20-15 DEFECT (non-plant), under diagnosis:** `epic-17-exit (ubuntu-24.04)` failed at step 20, line 3, after steps 12-18 passed: `test fixture stale — forbidden-syscall-probe: checkout is dirty; clean provenance cannot authorize changed files` (panic at `tests/harness/fixtures.rs:61:5`, test `benign_rust_probe_survives_t2_under_same_spec`, `rc=100`). The `ubuntu-26.04` leg was green. **Root cause unproven** — which path was dirtied, and by which earlier step, is unknown. The harness (`tests/harness/fixtures.rs`, uncommitted at record time) now names the dirty paths in that diagnostic; the next landing-shaped run is expected to identify the writer. Until fixed and shown green on a hosted run, AC1 is not met. Excerpt: `epic17-ubuntu-24.04-dirty-excerpt.log`.
- **Unrelated reds, classified pre-existing (not 20-15 regressions):** `workspace-test-suite` `root_shutdown_unload_16_3 ac4_v3b` (deferred-work 20-7, 5 s bound) and `cap_token_verify` p99 5270 ns (deferred-work 20-7); `check-multi-tenant-loom` `researcher_8_14c` race — first observed here; the test over-asserts against the ruled fail-fast contract; the fix is test-side at its origin, not in 20-15.
- **Still pending:** fix and hosted proof of the dirty-checkout defect; two landing-shaped runs for AC3 with per-class breakdown against 13.71 job-h / 73.9 min; final independent approval; landed E19 base. No completion claim.

### File List

- `_bmad-output/implementation-artifacts/20-15-ci-build-once-nextest-archive.md`
- `_bmad-output/implementation-artifacts/20-15-evidence/creation/job-pricing-40396208547.json`
- `_bmad-output/implementation-artifacts/20-15-evidence/creation/job-pricing.method.py`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md` (§20-15 corrections)
- `_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md` (`:209` tracker cite)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (20-15 row; E18-A2 row comment)

**Development changes in the current session** (shared files appear in both story records where applicable; this is not a claim that pins or ceilings changed):

- `.github/workflows/discipline.yml`
- `scripts/run_cgroup_ceiling_17_2.py`, `scripts/tests/test_cgroup_ceiling_17_2.py`
- `tests/harness/fixtures.rs`
- `xtask/src/build_test_fixtures.rs`, `xtask/src/check_test_cost.rs`
- `xtask/tests/fixture_provenance_20_15.rs`, `xtask/tests/check_test_cost_proven_red.rs`
- `xtask/tests/ci_shape_18_0.rs`, `xtask/tests/epic17_exit_workflow_17_7.rs`, `xtask/tests/cgroup_ceiling_workflow_17_2.rs`
- `crates/maos-kernel-core/tests/cgroup_ceiling_smoke.rs`, `crates/maos-kernel-core/tests/intent_lineage_corpus_v0.rs`
- `xtask/tests/example_spirit_regen_integration.rs`
- `docs/testing/proof-contract.md`
- `_bmad-output/implementation-artifacts/20-15-evidence/t1-spike.md`, `_bmad-output/implementation-artifacts/20-15-evidence/local-smoke-2026-10-27.md`
- `xtask/kloc.toml` — shared exact candidate pin (HISTORY corrected); separate 20-15 allocation **+38** (see Debug Log); no kernel or reserved-ceiling change.
- `.github/workflows/journey-nightly.yml` — nextest pinned to `cargo-nextest@0.9.149`.
- `xtask/src/main.rs` is shared with 20-16 (wiring), not a 20-15 allocation.

### Change Log

- 2026-10-27: Story created (ready-for-dev pending D-20-15-01…08) at `84e0e659`. Landing run 40396208547 priced per job and per bundle option; epic §20-15 corrected in place; tracker row refreshed.
- 2026-10-27: Operator ruled D-20-15-01…08 — D01 C, D02 B, D03 A, D04 A, D05 A, D06 A, D07 A, D08 A (D-08 A: own Epic-20 xtask grant, STOP above +38). Decision table and metadata PROPOSED → RULED 2026-10-27; AC1.1/AC2 wording aligned with the selections. Story remains ready-for-dev.
- 2026-10-27: Development started; Status synchronized with the in-progress tracker. T1 evidence recorded. The initial +53 draft exceeded the unchanged +38 STOP; redesign measured +37 before the current transport repairs. Local smoke evidence recorded; three independent transport findings remain under correction. No ceiling re-pin, completion flip, commit or push.
- 2026-10-27: Recorded the subsequent review repairs and explicit transport mode, the pre-mode 74 Rust / 30 Python focused results, and remaining review/plant/hosted obligations. The +38 STOP is unchanged; final measurement and approval remain pending.
- 2026-10-27: Exact formatted candidate grant allocation for `20-15-ci-build-once-nextest-archive`: xtask 46364 -> 46401 (+37 = `build_test_fixtures.rs` 508 -> 546, `check_test_cost.rs` 574 -> 573), within the unchanged +38 STOP. Combined candidate pin after the separately allocated 20-16 +1016 is 47417; aggregate measured188490, strict hardfail188491. `kloc-check --json` measured the candidate before re-pin; this is not a landed figure, completed E19 rebase, final review or hosted success.
- 2026-10-27: Corrected the debug-log reference: the historical **+37** measurement predates the explicit transport mode; it is not the final measurement of the current tree. Recorded the SESSION-ONLY model-role substitution and the review-correction triage (pending archive-family/runtime regressions under correction, not yet verified). The +38 STOP is unchanged; candidate grant arrows above remain historical, and a post-edit remeasurement/re-pin plus independent review are outstanding. Story remains in-progress.
- 2026-10-27: Post-correction record update. The 2026-10-27 candidate entry above (46364 -> 46401, +37; pin 47417, aggregate 188490/188491) is **historical**; the current candidate is **+38** (`build_test_fixtures.rs` 547, `check_test_cost.rs` -1; 46364 -> 46402), exactly STOP38, with the 20-16 +1019 giving pin 47421, aggregate measured 188494, hardfail 188495. Review triage (fixed/dismissed with reasons), `journey-nightly.yml` nextest pin and the parent verification figures are recorded in the Dev Agent Record. Story remains in-progress; no plant, hosted, timing, approval, savings or landed-E19 claim.
- 2026-10-27: Hosted evidence recorded for run 40536260336 (see Dev Agent Record, "Hosted plant run 40536260336"): AC4 plant proven red (supersedes the "no plant" clause above); AC1.2 zero-compile over 15 consumers; first cost sample; one open non-plant defect (epic-17-exit ubuntu-24.04 dirty checkout) under diagnosis. AC3 still needs two landing-shaped runs; no completion, approval or savings claim; story remains in-progress.
