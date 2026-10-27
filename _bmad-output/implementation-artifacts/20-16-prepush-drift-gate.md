---
story_id: '20.16'
story_key: 20-16-prepush-drift-gate
created: 2026-10-27
context_commit: 84e0e659
baseline_commit: 84e0e659
security_sensitive: false
kernel_grant: "none — kernel-Δ 0 by construction (xtask, docs and process only; `check-kernel-baseline` PASSED 26367 / 95 at `84e0e659`). No prototype needed."
kloc_grant: "RULED A (D-20-16-07, operator 2026-10-27): xtask 46364 (exact, zero headroom, `xtask/kloc.toml:339`) — forecast +405–645 tokei, ×1.3 ≈ +525–840 [INFERENCE, component table below] → own Epic-20 measured grant at landing: expected ≤ +815, STOP above +1020 (+25 %), outside D19-1; the Epic-19 xtask envelope rebases to (landed xtask figure) + 202 after 20-15/20-16 (was 46364 → 46566 = Σ 19-x forecasts × 1.3, `epics/epic-19-founder-loop-w4.md:41,129,135`)."
review: "bmad-code-review under docs/testing/proof-contract.md. Easy tier (no kernel grant, no security-critical AC): creation anthropic/claude-opus-5-5; dev/finisher openai-codex/gpt-6.1-sol; Blind+Edge merged on opus-5.5; Acceptance and Test Infrastructure on glm-5.3. The Edge layer replays the Epic-18 landing-red map (`20-16-evidence/creation/epic18-landing-reds.md`) against the implemented legs."
---

# Story 20.16: `xtask prepush` aggregate drift gate

Status: in-progress

<!-- Created 2026-10-27 at 84e0e659 by bmad-create-story under the lighter creation contract (no party-mode preflight). Prep story before 19-1 (E18 retro R3, operator ruling 2026-10-27; action E18-A3). D-20-16-01…08 RULED 2026-10-27 (01 C; 02–08 A — all recommendations accepted); dev may start. Measured, not read: every Epic-18 landing red the retro cites, mapped job-by-job from the jobs API (`20-16-evidence/creation/epic18-landing-reds.md`); every candidate check timed locally at HEAD (`prepush-timings.tsv`); `cargo nextest run` workspace vs per-package from one warm target dir. One cargo process; CARGO_TARGET_DIR=/mnt/build/cargo; `env -u CI -u GITHUB_ACTIONS`. No crate source changed; `intent-lineage-coverage-report.md` reverted after the workspace run touched it. -->

> **The capability:** *one local command, before every push, runs the cheap mirrors of the checks that turned Epic-18 landing runs red — generated-artifact drift, pin drift, enrolment lists, cross-crate scanners — plus the test suites the diff can break, and either exits 0 or prints one line per red check naming the artifact and its pin. The CI landing run stays the authority; it just stops being where drift is discovered.*

**Closes:**
- E18 retro **R3** / action **E18-A3** (`epic-18-retro-2026-10-27.md:58,79,89`; `sprint-status.yaml:544-547`); the E17-A6 class "manifest/admission change widens the focused set" automated (`:31`; `docs/testing/proof-contract.md:45-51`).
- Epic-20 §20-16 (`epics/epic-20-ship-it-w5.md:254-264`; stories row `:74`). Prep before 19-1 (`epics/epic-19-founder-loop-w4.md:156,209`).

**Does NOT close, and says so:**
- The CI landing run (stays authoritative); substrate-bound legs (T2/T3/cgroup/podman suites print `SKIP:` and pass locally, `proof-contract.md:93-98`) — their marker **values** stay CI-only; runtime defects (the 18-5 strong-factory drain class) beyond what the suites catch; wall-clock perf gates (20-7); the build-once archive (20-15 — `prepush` may consume it when present, not required).

## Story

As **a dev agent about to push a story tree**,
I want **one command that tells me, in about a minute, whether my diff drifted any generated artifact, pin or enrolment list, or broke a suite outside my focused set**,
so that **the landing run proves the tree instead of discovering drift — and Epic 19 does not pay ≈ 11 fix runs again.**

**Contract:**
- Epic-20 §20-16 (`_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:254-264` after the in-place corrections below; stories row `:74`; `:76` create-story may tighten, not add uncited ACs — three ACs kept, proven-red split out as AC4).
- Retro `:58,60,79,89,112`; tracker `sprint-status.yaml:313` (row) and `:544-547` (E18-A3).
- `docs/testing/proof-contract.md:31-51` (focused suites; closing re-runs; manifest change widens the set), `:70-91` (nextest, kernel-core lib under libtest, doctests), `:93-98` (local profile).

## Current-tree corrections — read before implementing

Measured at `84e0e659`.

| Premise (epic / retro) | Fact at `84e0e659` and consequence |
|---|---|
| Why: the landing-red runs and classes (`:258`) | **Holds, mapped job by job** (`20-16-evidence/creation/epic18-landing-reds.md`): 40183021781 → workspace-test-suite (unlisted file) + check-j1-loopback-delegation + check-reza-production-path + check-multi-tenant-loom; 40191703238 → check-serde-error-handling; 40237233599 → workspace-test-suite + check-test-budget; 40251998973 → onb-nfr2-timing (binary size) + check-multi-tenant-loom; 40267102242 → gen-abi-docs, check-ko-coverage (glossary) **and nine runtime reds** (the 18-5 strong-IAC-factory defect — not drift); 40273714381 → wasm-host-tests ×4 (GOVERNED-ATOMIC marker); 40319842655 → docs `gate:static`; 40336401565 → cargo-deny, check-stability-matrix, smoke-abi-7-5a, journey-hermetic-tier-1, workspace-test-suite, check-multi-tenant-loom. **Refinement:** check-multi-tenant-loom's reds in 40183021781/40251998973 are the 20-7-owned S3 podman-4 5 s bound (sprint row 18-1, `sprint-status.yaml:277`) — a flake, not drift; prepush does not claim them. |
| AC1: "the docs-site static gates incl. route-manifest parity" | The 18-7 red was `FAIL: route-manifest error_codes != error-catalog keys`, printed by **`docs-site/scripts/gate-troubleshoot-bidi.js:76-94`** (reads `docs/errors/error-catalog.json` + `docs-site/route-manifest.json`; no build), not by `gate-routes.js`. `gate:static` = routes + troubleshoot + glossary-lock + anchor-ids (`docs-site/package.json` scripts); `gate-routes.js` needs a docusaurus `build/` (`:14,57`) and passes against a **stale** local `build/` — only honest after a fresh build (measured 7 s with a warm webpack cache). Every static script needs Node builtins only (`fs`, `path`, `child_process`) — no `npm ci`. → D-20-16-03. |
| AC1: "the ko coverage + glossary gates" | CI runs `KO_COVERAGE_MIN=100 npm run gate:ko-coverage` + `gate:glossary-lock` (`discipline.yml:4298-4319`); without the env var the script is report-only. Measured 61 ms at 100 %. |
| AC1: "`error-catalog-check`" | **Not a CI job** (no workflow invokes it; grep). The CI authority for the class is `xtask/tests/error_catalog_20_3a.rs` inside `workspace-test-suite` (the 18-7 49 → 67 red). Prepush runs the gate (788 ms) as the cheap mirror; the test stays the authority. |
| AC1: "`SCANNED_SOURCE_FILES`/`WRITER_SHAPES` enrolment" | Both are **tests, not gates**: `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:927` (31 files); `WRITER_SHAPES` `crates/maos-audit/src/fr4_classifier.rs:195`, enforced by `crates/maos-audit/tests/fr4_classifier_16_2.rs`, which scans **other crates'** sources (`:859-861`: kernel-core, maos-bin, maos-iac). 16 test files read another crate's `src` by literal path (12 in `xtask/tests`, plus `fr4_classifier_16_2`, `cross_team_crossing_13_6b`, `journey_j0`, `support/binaries.rs`). A changed-crates-only leg misses exactly these (the 18-3 FR4 red: maos-bin changed, maos-audit's test red). → D-20-16-01. |
| AC1: "the CI count/marker pin readers" | No reader exists. `discipline.yml` holds **20** count pins (`test result: ok. N passed`, `N tests run: N passed`, `--expected-count N`) and marker specs (e.g. `:4048-4071` `'GOVERNED-ATOMIC :10'` — the 18-5 8-vs-6 red). Under `CARGO_TERM_COLOR: always` (`:28`) a pin over nextest output needs `--color never` (the 18-7 journey red, `:3367-3368`). `check_test_cost.rs:518` (`job_block`) already slices job blocks; `serde_yaml` is an xtask dependency (`xtask/Cargo.toml:42`). → D-20-16-02. |
| AC3: "the nextest leg is limited to changed crates" | **Measured more expensive, and blind to cross-crate scanners.** From one warm target dir: `cargo nextest run --workspace` **45 s** wall (5072 passed, 135 skipped; test phase 40.4 s) vs `-p maos-bin` **92 s** (602 passed; ≈ 70 s rebuild — per-package feature resolution differs from the workspace's: tokio `test-util`/`io-std`), `-p xtask` 49 s, `-p maos-skill` 4 s. CI also needs `cargo test -p maos-kernel-core --lib` (default-filter, `.config/nextest.toml`) and `--doc` (6 s on CI, 40396208547). → D-20-16-01. |
| AC3: "total wall-clock measured at creation" | **Measured** (`20-16-evidence/creation/prepush-timings.tsv`): 21 xtask gates + `cargo fmt --check` = **15.7 s** (largest `gen-abi-docs --check` 3.5 s, fmt 2.6 s, `check-service-boundary` 1.5 s; the rest 0.26–0.79 s); 4 docs static scripts **0.33 s**; docs build + routes 7.1 s (warm); workspace nextest 45 s. **≈ 61 s warm** without the conditional legs, + incremental build after an edit (diff-dependent, unmeasured). |
| (unstated) binary size and cargo-deny run locally | Size: `tests/integration/onb_nfr2_timing.sh:115-118` (release build + strip, ≤ 33,554,432 B) — a release maos-bin build (`codegen-units = 1`, `Cargo.toml:110-111`). `cargo-deny` is **not installed** on the dev host (`which cargo-deny` → none; CI pins 0.19.4, `discipline.yml:30`) and needs the advisory DB (network). → D-20-16-03. |
| Holds | `stability-matrix --check` (`xtask/src/main.rs:668`), `gen-abi-docs --check` (`:879`, nightly-2026-05-01 installed), `check-serde-error-handling`, all PASS at HEAD; 68 xtask subcommands (`main.rs` `#[command(name = …)]`). |

**Epic cites corrected in place (OLD → NEW @`84e0e659`; `epics/epic-20-ship-it-w5.md`):**
- §20-16 Why `:258`: runs and fixes hold; appended "check-multi-tenant-loom reds in 40183021781/40251998973 = the 20-7-owned S3 bound (not drift); 40267102242's other nine reds = the 18-5 runtime defect".
- §20-16 AC1 `:260`: "docs-site static gates incl. route-manifest parity" → "the build-free docs scripts (`gate-troubleshoot-bidi.js` — its `:76-94` is the route-manifest `error_codes` parity that reds on the 18-7 class —, glossary-lock, anchor-ids, ko-coverage at `KO_COVERAGE_MIN=100`) and, per D-20-16-03, build + `gate-routes.js`"; "`error-catalog-check`" annotated (no CI job; CI authority `error_catalog_20_3a`); "`SCANNED_SOURCE_FILES`/`WRITER_SHAPES` enrolment" → "the tests that enforce them (`cohort_daemon_smoke_13_5c.rs:927`; `fr4_classifier_16_2.rs:859-861`), via the nextest leg (D-20-16-01)"; "full-package `cargo nextest` of every crate the pushed diff touches" → per D-20-16-01.
- §20-16 AC3 `:262`: "the nextest leg is limited to changed crates" → "the nextest leg per D-20-16-01 (measured: workspace 45 s < `-p maos-bin` 92 s from a warm dir)"; wall-clock measured (≈ 61 s warm).
- `epics/epic-19-founder-loop-w4.md:209`: tracker `sprint-status.yaml:312` → `:313` (this story; with 20-15's `:311` → `:312`).

## Kernel and kloc pricing — measured at creation

**Kernel:** Δ 0 by construction; `check-kernel-baseline` PASSED 26367 / 95 at `84e0e659`.

**kloc at `84e0e659`** (`kloc-check --json` passed, aggregate 187437; xtask 46364 exact, `xtask/kloc.toml:339`). xtask forecast by component (tokei code; analogues measured with `tokei`: `check_serde_error_handling.rs` 300, `check_workspace_count.rs` 265, `check_test_cost.rs` 574):

| Component (recommended rows) | Forecast [INFERENCE] |
|---|---:|
| `main.rs` subcommand + dispatch (`Prepush { json, base, all, skip_nextest }`) | 10–15 |
| check table + self-exec runner (D-04 A: `current_exe()` with the CI argv, capture, exit, one red line each) | 60–90 |
| docs legs (node builtins scripts; `KO_COVERAGE_MIN=100`; conditional build + routes) | 25–40 |
| changed-path set (`git merge-base`, diff, untracked) → packages (`cargo metadata --no-deps`) and triggers | 50–80 |
| nextest leg (D-01) + kernel-core `--lib` + `--doc` | 25–40 |
| CI pin reader (D-02 A: count pins vs `cargo nextest list`, marker-site count, ANSI lint) | 140–220 |
| conditional legs (D-03 A: release size + strip, `cargo deny` when installed, local `check-test-budget` advisory) | 50–80 |
| report: one line per red naming artifact + pin; `--json` | 40–60 |
| **Σ** | **+400–625 → ×1.3 ≈ +520–815** |

D-02 B instead of A: −60–90. D-01 A instead of C: +20–40 (per-crate selection + scanner set).

## Decisions — RULED (operator 2026-10-27: 01 C, 02–08 A)

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-20-16-01** | The test leg | A — changed crates only (`-p` each touched package; epic AC3 as written) · B — changed crates + every package whose tests name a changed crate's `src` path (grep-derived scanner set, 16 files today) · **C — `cargo nextest run --workspace --locked --no-fail-fast` + `cargo test -p maos-kernel-core --lib` + `cargo test --workspace --doc`, the CI command (`discipline.yml:5596,5603,5611`) minus the substrate — as ruled, the kernel-core `--lib` run is unconditional (creation prose had "when kernel-core changed")** | **C.** Measured from one warm dir: workspace 45 s < `-p maos-bin` 92 s (per-package feature resolution rebuilds ≈ 70 s); C is CI's resolution, catches every cross-crate scanner (18-3 FR4, 18-7 `error_catalog_20_3a`, 18-1 `SCANNED_SOURCE_FILES`) and the 17-11/E17-A6 class by construction. A misses three of the Epic-18 drift rows; B is A plus a grep heuristic that literal-path scans only. Cost is the incremental build after an edit, which A also pays. | **RULED C** (operator 2026-10-27) |
| **D-20-16-02** | CI count/marker pin readers | **A — static reader over `discipline.yml`: each pin bound to a `--test <bin>` (20 count pins) is compared with `cargo nextest list` for that binary and filter; each marker spec (`'NAME :N'`) with the number of emission sites of that marker in the named test source; every step that greps cargo/nextest output carries `--color never` or `CARGO_TERM_COLOR=never` (`:28` is `always`)** · B — count pins + ANSI lint only (marker specs stay CI-only) · C — no pin reader; CI stays the only reader | **A.** 18-5's 8-vs-6 marker and 18-7's ANSI pin were both pin drift a static reader sees; counts come from `nextest list` (no substrate needed). The marker-site count is a heuristic (a site in a loop counts once) — the reader reports a mismatch as red with both numbers, and B is the fallback if review finds the heuristic noisy. | **RULED A** (operator 2026-10-27) |
| **D-20-16-02 addendum** | Dynamically emitted markers (NEW operator addendum 2026-10-27) | **C — dynamic-only markers get a named indeterminate SKIP; statically determinate markers are compared exactly; a missing emission site is RED. Dynamic actual counts remain CI-only.** | The original **RULED A** row above stands as the historical ruling; it is **not** read as having already permitted C. The addendum is explicit and later, and does not weaken any CI runtime marker guard. | **ADDENDUM RULED C** (operator 2026-10-27) |
| **D-20-16-03** | Expensive / tool-dependent legs | **A — conditional on the change set, each skip printed as one named line: release build + strip + 33,554,432 B (`onb_nfr2_timing.sh:115-118`) when `Cargo.toml`/`Cargo.lock`/`crates/maos-bin/**`/a maos-bin dependency changed; docusaurus build + `gate-routes.js` when `docs-site/**`, `docs/errors/**` or `xtask/error-catalog.toml` changed (the docs-site workflow's own path filter, `.github/workflows/docs-site.yml:7-29`); `cargo deny check` when `Cargo.lock`/`deny.toml` changed and `cargo-deny` is installed — absent tool on a lockfile change = red naming the tool; local `check-test-budget` on the nextest junit as advisory (local times ≠ CI)** · B — run all of them every time · C — leave them to CI | **A.** Each class reded a landing run (18-4 size, 18-7 docs, 18-7 deny, 18-3 budget); unconditional B adds a release build (minutes) to every push; C reopens the four classes. | **RULED A** (operator 2026-10-27) |
| **D-20-16-04** | How the gates run | **A — self-exec: `std::env::current_exe()` with exactly the CI argv (e.g. `stability-matrix --check --json`), output captured, one red line per failure** · B — in-process calls to each gate's `run` | **A.** Byte-for-byte the CI command, no rebuild (measured 0.26–3.5 s each through `cargo run`, ≈ 0.25 s of it cargo's own check), and a gate that exits or prints cannot corrupt the aggregate report. B couples prepush to 20+ module signatures. | **RULED A** (operator 2026-10-27) |
| **D-20-16-CR2** | `check-deprecations-declared` (NAMED CR-2 exception, operator ruling 2026-10-27) | Kept as an explicit exception to "prepush mirrors a CI step/test": it has no CI step and no test authority, yet AC1 lists it | Prepush runs it as an extra local drift check; it is not a CI mirror and the landing run does not authorise it. Recorded here and in `docs/testing/proof-contract.md` (Pre-push). | **RULED** (operator 2026-10-27) |
| **D-20-16-05** | "Changed" base | **A — `git merge-base origin/main HEAD` diff ∪ working tree ∪ untracked; prepush never fetches and prints the base SHA and its age** · B — `@{upstream}` · C — `HEAD~1` | **A.** The landing target is `main`; stories span several commits; untracked new files are exactly the 18-1 class. | **RULED A** (operator 2026-10-27) |
| **D-20-16-06** | "Required before push" — enforcement | **A — documented: `proof-contract.md` gains the row (AC2), the local dev-story/code-review customizations mirror it, and the Dev Agent Record of every story names the prepush run (exit 0, wall) before its landing push** · B — A + `xtask prepush --install-hook` writing `.git/hooks/pre-push` · C — CI verifies a prepush receipt | **A.** AC2: CI stays the authority, prepush is not a second gate set — C would make it one. B is a cheap opt-in add-on if the operator wants a hard local stop (+15–25 lines). | **RULED A** (operator 2026-10-27) |
| **D-20-16-07** | xtask kloc | **A — own Epic-20 measured grant at landing: expected ≤ +815 (upper ×1.3), STOP above +1020 (+25 %), outside D19-1** · B — book against D19-1 (46364 → 46566: +202, which this story alone exceeds) · C — home the aggregator in `scripts/prepush.py` (ungoverned; a second convention beside xtask, the gate home) | **A.** D19-1 is Σ(19-x forecasts) × 1.3 with no prep-story slack. 20-15 and 20-16 both land before 19-1, so the operator also states the E19 envelope base after them: (landed xtask figure) + 202. | **RULED A** (operator 2026-10-27) |
| **D-20-16-08** | How AC1's proven red is kept | **A — local plant only (AC1 as written): drop one code from `docs-site/route-manifest.json` `error_codes`, run prepush → red naming `route-manifest.json` and the code; restored from a byte-identical backup; `20-16-evidence/plants.md`** · B — A + an `xtask/tests/prepush_20_16.rs` vector over a temp copy (needs a `--root` on prepush and on the docs leg; +15–25 lines) | **A.** The CI authority for the class stays the docs-site job; prepush's own report/aggregation logic gets in-src unit tests (a red leg → non-zero exit + its line). | **RULED A** (operator 2026-10-27) |
| CR-1 | Output contract | Exit 0 with one summary line (checks run, skipped-with-reason, wall); or exit 1 with one line per red check: `RED <check> — <artifact> — <pin or expected/actual>`; `--json` emits the same records. Never a pass on a check that could not run: a missing tool on a triggered leg is red. | — |
| CR-2 | Mirror, not a second gate set | Every check prepush runs exists in CI or is the cheap mirror of a CI test (`error-catalog-check` ↔ `error_catalog_20_3a`); prepush adds no rule CI lacks. `CI`/`GITHUB_ACTIONS` are unset for the nextest leg (local profile, `proof-contract.md:93-98`); substrate SKIPs are counted and printed. | — |

## Acceptance Criteria

*(Epic §20-16 as corrected at creation; ACs 1–3 kept (epic `:76`), the proven red stated as AC4. D-rows RULED 2026-10-27 (01 C; 02–08 A, all recommendations accepted) — wording matches the rulings.)*

### AC1 — command

1. `cargo run -p xtask -- prepush` runs, in one aggregate exit (CR-1): `stability-matrix --check`, `error-catalog-check`, `gen-abi-docs --check`, `check-serde-error-handling`, `check-workspace-count`, `check-manifest-schema-version`, `check-skill-schema`, `check-coverage-matrix-completeness`, `check-test-cost`, `check-env-contract`, `check-unsafe`, `check-service-boundary`, `check-kernel-baseline`, `kloc-check`, `check-breaking-md`, `check-composition-root-completeness`, `check-pub-field-constructors`, `check-dependency-closure`, `check-deprecations-declared`, `coverage-matrix`, `check-corpus`, `cargo fmt --all -- --check` (the measured 22, each with its CI argv, D-04); the build-free docs scripts incl. route-manifest parity and ko at 100 %; the pin reader (D-02); the conditional legs (D-03); the test leg (D-01).
2. Observable: exit 0 with the summary, or one `RED` line per failing check naming the drifted artifact and its pin.

### AC2 — the contract names it

`docs/testing/proof-contract.md` gains a "Pre-push (20-16)" row/section: `cargo run -p xtask -- prepush` exit 0 is required before every push of a story tree; the landing run remains the authority; prepush mirrors its drift classes (CR-2). The local `_bmad/custom/bmad-dev-story.user.toml` / `bmad-code-review.user.toml` mirror it (gitignored, per `proof-contract.md:3-6`).

### AC3 — cost-bounded

Total wall-clock recorded in `20-16-evidence/` on the landing tree: warm no-change run, and one run after a one-line edit in `crates/maos-bin/src` (incremental build included). Budget at creation: ≈ 61 s warm (`prepush-timings.tsv`); a regression above 2× that on the warm run is reported in the Dev Agent Record.

### AC4 — proven red (rule 11)

Planting the 18-7-class drift (one code removed from `docs-site/route-manifest.json` `error_codes`) reds `prepush` locally with a line naming `route-manifest.json` and the code — the class that escaped to run 40319842655; restored byte-identical (D-08). One more local plant per leg family is NOT required (proof contract `:26`); the Edge review replays `epic18-landing-reds.md`.

## Tasks / Subtasks

- [ ] **T0 — Preflight.** D-20-16-01…08 RULED; tracker → `in-progress`. Re-measure `prepush-timings.tsv` rows that moved; re-check that 20-15 has not renamed any job a pin reader reads.
- [ ] **T1 — Skeleton** (AC1, CR-1, D-04, D-05): subcommand, self-exec runner, change set, report/JSON; in-src tests for aggregation and exit.
- [ ] **T2 — Legs** (AC1): the 22 gates; docs scripts; D-03 conditional legs; D-01 test leg.
- [ ] **T3 — Pin reader** (D-02): parse `discipline.yml` (`serde_yaml`; reuse `check_test_cost::job_block`, `:518`, made `pub(crate)`), counts vs `nextest list`, marker sites, ANSI lint; it must be green on HEAD (fix or exempt-with-reason any real drift it finds, each named).
- [ ] **T4 — Proven red** (AC4) and **cost** (AC3): plant, restore, timings.
- [ ] **T5 — Contract** (AC2): `proof-contract.md`; local customizations; kloc exact re-pin per D-07 (`d11_xtask_ceiling_ratchet` cites this story's full key); Dev Agent Record, File List, Change Log; tracker. Do not edit `intent-lineage-coverage-report.md` (revert if a run modifies it). No commit or push without the operator.

## Dev Notes

### What changes, by file

| Area | Files | Today → after |
|---|---|---|
| xtask | `xtask/src/main.rs` (subcommand + dispatch), new `xtask/src/prepush.rs`; `xtask/src/check_test_cost.rs:518` (`job_block` visibility) | — → aggregate command |
| Contract | `docs/testing/proof-contract.md` (new section after `:51` or `:91`) | — → required step |
| Kloc | `xtask/kloc.toml:339`, `:718` | exact re-pin (D-07) |
| Read only | `.github/workflows/discipline.yml`, `docs-site/scripts/*.js`, `docs-site/route-manifest.json`, `docs/errors/error-catalog.json`, `tests/integration/onb_nfr2_timing.sh:115-118` | — |

### Current state that must be preserved

- Every gate's own CLI and CI step (prepush only invokes them); docs scripts unchanged; CI unchanged (CR-2).
- The local profile: substrate tests SKIP and pass with `CI` unset — prepush never sets `CI`.

### Guardrails

- No pass on a check that did not run (CR-1); skips are printed with the trigger that was absent.
- Never fetch, never write tracked files; a leg that writes (e.g. `intent-lineage-coverage-report.md` from the workspace suite — measured at creation) is restored or reported.
- `--color never` on every cargo/nextest invocation prepush parses.
- One target dir; `env -u CI -u GITHUB_ACTIONS`; no second `CARGO_TARGET_DIR`; plants byte-restored.
- D-02 C addendum (operator 2026-10-27): the pin reader never weakens CI's runtime marker pins; a dynamically emitted marker is reported as a named indeterminate SKIP (its actual count stays CI-only), a statically determinate marker is compared exactly, and a marker with no emission site is RED.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red |
|---|---|---|
| Docs parity (AC4) | remove one code from `route-manifest.json` `error_codes` | `RED docs:troubleshoot-bidi — docs-site/route-manifest.json — in catalog not manifest: <code>` |
| Aggregation (in-src test) | one stub leg red | non-zero exit, exactly one `RED` line |

### Tests and commands

```bash
# local; one cargo process; CARGO_TARGET_DIR=/mnt/build/cargo
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- prepush
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- prepush --json
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p xtask --lib prepush
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p xtask --test d11_xtask_ceiling_ratchet --test recovery_lane_ceiling_rule
cargo run -q --locked -p xtask -- kloc-check --json && cargo run -q --locked -p xtask -- check-test-cost && cargo fmt --all -- --check
```

### Review plan (proof contract `:31-68`)

- Easy tier: Blind+Edge on opus-5.5 (Edge replays `epic18-landing-reds.md` row by row against the legs); Acceptance and Test Infrastructure on glm-5.3.
- Reviewer re-runs `prepush` once on the review tree and records its wall.

### Previous-story intelligence

- **20-12:** the focused-suite contract and the local profile; nextest default-filter leaves kernel-core lib to libtest; closing re-runs `d11_xtask_ceiling_ratchet` (its xtask row must cite the full story key — the 20-12 HIGH).
- **18-7 landing (`271e2a33`):** six reds in one run, all drift/pin/outside-focused-set — the shape this command mirrors; `--color never` for nextest count pins.
- **18-5 landing:** marker 8 vs 6 — markers are pins too.
- **18-3 / 18-1:** cross-crate scanners and enrolment lists live in other packages' tests.

### Git intelligence

| Commit | Content |
|---|---|
| `84e0e659` | Epic 18 closed + retro (R3 → this story) |
| `f889b3e4` | 18-5/6/7 done; landing green 40396208547 |
| `271e2a33` | 18-7 landing fixes (six reds) |
| `a2d69a29` | 18-7 review patches incl. docs-site `error_codes` fix (red 40319842655) |
| `e1d09f0b` | 20-12: nextest, proof contract, `check-test-cost` |

### References

- Epic `epic-20-ship-it-w5.md:74,76,254-264`; epic-19 `:41,129,135,156,209`; retro `:31,58,60,79,89,112`; tracker `:277,313,544-547`.
- `discipline.yml:28,30,3367-3368,4048-4071,4298-4319,5596,5603,5611`; `.github/workflows/docs-site.yml:7-29`; `docs-site/package.json` scripts; `docs-site/scripts/gate-troubleshoot-bidi.js:17,76-94`, `gate-routes.js:14,57`; `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:927`; `crates/maos-audit/src/fr4_classifier.rs:195`; `crates/maos-audit/tests/fr4_classifier_16_2.rs:859-861`; `xtask/tests/error_catalog_20_3a.rs`; `tests/integration/onb_nfr2_timing.sh:115-118`; `xtask/src/check_test_cost.rs:518`; `xtask/Cargo.toml:42`; `xtask/kloc.toml:339,718`; `docs/testing/proof-contract.md:3-6,26,31-51,70-98`.

## Dev Agent Record

### Story context creation

Created 2026-10-27 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter contract) at `84e0e659`, clean tree.

**Inputs:** direct reads of every cited range; `check-kernel-baseline` PASSED 26367 / 95; `kloc-check --json` passed (aggregate 187437); jobs API for the nine runs of retro `:58` (two retried after an HTTP/2 stream error); every candidate check timed (`prepush-timings.tsv`); `cargo nextest run` workspace / maos-bin / xtask / maos-skill from one warm dir.

**Revert:** the workspace nextest run modified `intent-lineage-coverage-report.md`; restored with `git checkout --`; `git status --short` empty; no crate source changed.

### Agent Model Used

anthropic/claude-opus-5-5 (creation); openai-codex/gpt-6.1-sol (implementation).

**Model roles are a SESSION-ONLY substitution (operator, latest 2026-10-27):** coding tasks and high-risk reviews run on `anthropic/claude-opus-5-5`; light reviews and docs on Sonnet 5.5. An earlier temporary Terra substitution was superseded. This does not rewrite the standing tracked role policy in `docs/testing/proof-contract.md`, and it is not a claim that the current parent conversation switched away from Sol.

### Debug Log References

- `20-16-evidence/local-integration-smoke-2026-10-27.md`: first real JSON aggregate, exit 1, 756.686 s internal wall; 5108/5109 workspace tests passed with only the un-repinned KLOC test failing. Kernel lib, doctests, lineage restoration and tracked-tree preservation passed.
- *Historical (superseded):* operator reaffirmed **keep +1020 and redesign** after the +1700 formatted draft; later states measured +1200 and then 1172 + 7 = +1179, still over STOP. **Current exact candidate grant:** 20-16 **+1019** = `prepush.rs` 1013 + `main.rs` +6, within STOP1020; with 20-15 +38 (46364 -> 46402) the xtask candidate pin is **47421**, aggregate measured **188494**, hardfail **188495** (`xtask/kloc.toml` HISTORY corrected). Candidate only; not a landed figure.
- The integration smoke exposed 16 ANSI guard records and a guard-only regex-selection false positive; both were corrected (see triage). Human-mode smoke, in-place route-manifest plant, final warm/incremental timings and hosted proof remain pending.
- **Observed full actual post-fix prepush (bg_19), JSON mode** — raw `artifact://5134`, parsed session artifact `local://prepush-postfix-5134.json` (not a portable repository link). **No red records: 64 pass, 64 skip, 1 advisory**; internal wall **671.768 s**, shell wall **672.71 s**; base `271e2a33768a8f2b75c6cd41bd6f0c82d0526f39`; **68 changed paths**. This predates the current review-correction patches and the D-02 C addendum implementation; it is **not** the warm/no-change AC3 run, **not** final-tree proof, and not a plant, human-mode, hosted or landing result. The 64 skips include arbitrary captured negative-control diagnostics; they are **not** 64 independently absent substrates.
- **Human-mode prepush bg_21** (before the latest corrections) exited **1** with **ONE** red, `tracked-tree`: the orchestrator was concurrently editing `.devcontainer/docker-compose.yml` and the tracker during the run, so the guard correctly detected a modified tracked file (not a product defect). Otherwise 63 checks ran, 8 skipped-with-reason, 56 captured SKIP diagnostics, internal wall 475.018 s. It predates the final corrections and is not final-tree, AC3, plant or hosted evidence. A final post-correction full prepush is **not yet recorded**.
- **Parent verification (2026-10-27):** focused xtask suites 142 passed / 6 skipped; dev-record lib 22 passed; Python 30 OK; `check-test-cost` PASS; `kloc-check` passed with no over-budget crate; kernel baseline 26367 / 95 unchanged (tokei 20610); record gates semantically unchanged after archival (100 owner assertions, 22 dispositioned, 186 done stories); `example-spirit-regen` alias smoke WARN, `passed:true`.

### Completion Notes List

The first JSON smoke was red and included compilation; it is not the AC3 warm benchmark. Mini-Git/local-driver checks are not the formal in-place story-tree plant. Neither the earlier narrow 20-15 T1 spike nor this aggregate smoke is a full hosted-bundle or landing proof.

**Review-correction triage (2026-10-27).** Reviewers: Opus 5.5 for coding/high-risk, Sonnet 5.5 for light audits (SESSION-ONLY substitution). No final approval is claimed; the story stays in-progress.
- **A1 resolved by the operator C addendum** (D-20-16-02): dynamic-only markers are a named indeterminate SKIP, determinate markers compare exactly, a missing emission site is RED. The archive-family and fixture regressions were fixed under 20-15.
- *Opus 20-16 prepush review:* P2 stale budget-exception advisory — **fixed**; P3 closure markers — **fixed**; P3 unbounded RED detail — **fixed**; P3 unrecorded `CARGO_TERM_COLOR` global `never` — recorded here as a **named drift fix** from the ANSI-lint correction (the workflow-level colour default is now `never`; explicit nextest `--color never` and command-scoped overrides are still detected; no blanket reader exemption).
- *Sonnet audit of 20-16:* no unresolved finding recorded here.
- *Also accepted and fixed:* Git environment scrub in provenance tests; stale ship-gate summary wording; count-drift RED regression via child process + stub cargo; static marker pass arm; removed the std `Command::get_args` assertion; change-set union (base->HEAD ∪ HEAD->working ∪ untracked, rename endpoints); captured SKIP lines treated as diagnostics, not substrate skips; obsolete lineage restore removed; add-only tests removed.
- *Dismissed or kept, with reasons:* pin-reader `nextest list` cost recorded honestly at AC3; pin reader covers recognised count/marker shapes only (contract wording); TI-6 real-workflow reader test not added (the prepush runtime exercises it); `check-deprecations-declared` kept as the named CR-2 exception (D-20-16-CR2).
- Producer-parse P3 (unknown-field erasure) was fixed on the 20-15 side with `deny_unknown_fields` (+1 producer line, inside STOP38).
- Not done: final post-correction full prepush, in-place plants, warm/incremental timing, hosted proof, final approval, savings or landed-E19 base.

### File List

- `_bmad-output/implementation-artifacts/20-16-prepush-drift-gate.md`
- `_bmad-output/implementation-artifacts/20-16-evidence/creation/prepush-timings.tsv`
- `_bmad-output/implementation-artifacts/20-16-evidence/creation/epic18-landing-reds.md`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md` (§20-16 corrections)
- `_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md` (`:209` tracker cite)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (20-16 row; E18-A3 row comment)
- `_bmad-output/implementation-artifacts/20-16-evidence/creation/epic18-landing-reds.md` (GOVERNED-ATOMIC row and Coverage sentence under operator C)

**Development changes in the current session** (shared files appear in both story records where applicable; no pin or ceiling change is claimed):

- `xtask/src/prepush.rs`, `xtask/src/main.rs` (command wiring)
- `xtask/src/check_test_cost.rs` (`job_block` visibility and shared 20-15 checker changes)
- `xtask/tests/prepush_20_16.rs`, plus shared 20-15 workflow/fixture tests: `xtask/tests/ci_shape_18_0.rs`, `xtask/tests/epic17_exit_workflow_17_7.rs`, `xtask/tests/cgroup_ceiling_workflow_17_2.rs`, `xtask/tests/check_test_cost_proven_red.rs`, `xtask/tests/example_spirit_regen_integration.rs`, `xtask/tests/fixture_provenance_20_15.rs`
- `.github/workflows/journey-nightly.yml` (nextest pin `cargo-nextest@0.9.149`; shared with 20-15)
- `docs/testing/proof-contract.md`
- `_bmad/custom/bmad-dev-story.user.toml`, `_bmad/custom/bmad-code-review.user.toml` — gitignored per-checkout mirrors only, not tracked source of truth.
- `_bmad-output/implementation-artifacts/20-16-evidence/local-integration-smoke-2026-10-27.md`
- `xtask/kloc.toml` — shared exact candidate pin (HISTORY corrected): 20-16 +1019, 20-15 +38, pin 47421, aggregate measured 188494.

### Change Log

- 2026-10-27: Story created (ready-for-dev pending D-20-16-01…08) at `84e0e659`. Epic-18 landing reds mapped to legs; candidate checks timed; epic §20-16 corrected in place; tracker row refreshed.
- 2026-10-27: Operator ruled D-20-16-01…08 (01 C; 02–08 A — all recommendations accepted); PROPOSED → RULED, ratification 2026-10-27. D-07 as ruled: own Epic-20 xtask grant, STOP above +1020, and the E19 xtask envelope rebased to (landed xtask) + 202. D-01 as ruled: workspace nextest plus `cargo test -p maos-kernel-core --lib` and `--doc`, the kernel-core `--lib` run unconditional (the operator's selected option carries no condition). Story ready-for-dev.
- 2026-10-27: Development started; Status synchronized with the in-progress tracker. Recorded the first actual aggregate RED and operator's unchanged +1020 redesign ruling. No final cost claim, ceiling re-pin, completion flip, commit or push.
- 2026-10-27: Recorded the subsequent ANSI and guard-reader corrections, the later +1179 state and outstanding final rerun, plants, timings and review. The operator's +1020 STOP remains fixed.
- 2026-10-27: Exact formatted candidate grant allocation for `20-16-prepush-drift-gate`: xtask 46401 -> 47417 (+1016 = `prepush.rs` 1010, `main.rs` 1230 -> 1236), within the unchanged +1020 STOP. The prior 20-15 allocation is 46364 -> 46401 (+37); aggregate measured188490, strict hardfail188491. Candidate pin only: independent final review, actual prepush rerun, plants, warm/incremental timing and hosted proof remain pending; E19 rebases to the eventual landed xtask +202, not an invented landed value.
- 2026-10-27: Recorded the operator's **NEW addendum** to D-20-16-02 (option C for dynamically emitted markers; see the addendum row under the decision table). The original ruling A is retained as historical and is not reinterpreted as already permitting C. Recorded the SESSION-ONLY model-role substitution, the observed pre-patch full actual prepush result (bg_19) and the review-correction triage. Story remains in-progress: no final review, plant, warm/incremental timing, hosted, landed-base or cost-savings claim.
- 2026-10-27: Post-correction record update. Entries above carrying +1016/47417/188490 or +1179 are **historical**. Current candidate grant: 20-16 **+1019** (`prepush.rs` 1013, `main.rs` +6; STOP1020), 20-15 +38; xtask 47421, aggregate measured 188494, hardfail 188495. Recorded review triage (fixed/dismissed with reasons), the `CARGO_TERM_COLOR` global `never` named drift fix, the `check-deprecations-declared` named CR-2 exception, the human-mode bg_21 single `tracked-tree` red (concurrent orchestrator edits) and the parent verification figures. Story remains in-progress; no final prepush, plant, timing, hosted, approval, savings or landed-E19 claim.
- 2026-10-27: Final post-correction `prepush --json` with exclusive cargo: **exit 0, 232 s, 62 pass / 8 skip / 1 advisory / 0 red** (`20-16-evidence/local-integration-smoke-2026-10-27.md` § Final post-correction JSON prepush). The earlier 3600 s kill was external (shared target wiped mid-run plus a concurrent outside prepush), not a prepush defect. Remaining: in-place AC4 plant, AC3 warm/incremental pair on the committed tree, hosted proof and landing.
