---
story_key: 20-6-macos-compile-guard
status: done
updated: 2026-10-07
dev_model_used: anthropic/claude-opus-5-5
initial_draft_model: "anthropic/claude-sonnet-5-5 — the first dev pass and its review (2026-10-06). Operator ruling 2026-10-06: dev re-run under a frontier model; this record's dev of record is `anthropic/claude-opus-5-5` (see §Dev Agent Record)."
baseline_commit: 0d73cac32bb6155e46bdc4374f77ff2672f3f060
depends_on: "`ops-darwin-rehearsal` (`done`) — landed `0c661995`, which fixed the three breaks this story guards against. D-15-4-F (`15-4-release-repair-and-first-signed-tag.md` AC2): macOS stays out of every matrix and off ordinary pull requests; the operator ruled THIS job as the single exception (2026-09-25)."
blocks: "none. `20-5` (rc tag) and every later release leg benefit from it, none wait for it."
spec_alignment: "Rule 9 applied at creation (2026-10-06 @`0d73cac3`): the three named breaks are ALREADY FIXED (`0c661995`), so AC2's falsifier is a re-plant; the AC1 command, runner rationale and filter were re-derived from measurement (§Preflight, OLD→NEW applied to the epic file in the same change); a FOURTH macOS break was found outside the release graph (`maos-exec-deps`). Frontier re-run (2026-10-06 @`32420928`, wasmtime 48): operator ruling Q1 applied — `maos-exec-deps` is gated in source, the guard checks the whole workspace (epic OLD→NEW updated)."
kernel_grant: "NONE. kernel-Δ 0; `xtask/kernel-core-baseline.toml` untouched; `check-kernel-baseline` and `check-service-boundary` green."
kloc_grant: "◆ OPERATOR GRANT 2026-10-06 (ruling Q1), applied by the frontier re-run: `xtask/kloc.toml` `maos-exec-deps` 357 → 358 (+1, the exact `kloc-check` measurement after `cargo fmt --all`: the crate-level `#![cfg(target_os = \"linux\")]`). ZERO headroom kept. Every other row untouched; aggregate 170434 (hardfail 170884). (The draft's figure was kloc 0 with `--exclude maos-exec-deps`.)"
model_grant: "NONE NEEDED. The draft requested `sonnet-5-5` in `FRONTIER_FAMILIES`; the operator DECLINED it (2026-10-06) and ruled a frontier re-run instead. Dev of record `anthropic/claude-opus-5-5` matches the existing `opus-5` token; `FRONTIER_FAMILIES` untouched; `check-dev-model-tier` green."
review: "bmad-code-review §A6, FRESH frontier pass 2026-10-06 (Blind Hunter + Edge Case Hunter + Acceptance Auditor as separate subagents at the session model's capability, anthropic/claude-opus-5-5); the Test-Infrastructure axis of `_bmad/custom/bmad-code-review.user.toml` is a no-op for an `anthropic.*` dev model. Findings and dispositions in §Review Findings; the draft's own review is kept below it as history."
---

# 20-6 — A path-filtered macOS compile guard

Status: done

> **The capability:** *A change that can break `aarch64-apple-darwin` — a `cfg`, an OS-typed API call, a dependency,
> a build script — compiles for macOS before it merges, and a change that cannot does not pay for a macOS runner.
> When the guard is skipped, the pipeline says "not applicable", never "passed".*

**Closes:** the epic-20 §20-6 charter (operator ruling 2026-09-25, `v0.1.0-alpha.1` rehearsal): release-build honesty
for `aarch64-apple-darwin` — the three breaks the first macOS build found had each cost a rehearsal round at release
time and had been dormant since 2026-05-14.

**Does NOT close, and says so:**
- The pre-tag rehearsal (`docs/release/tag-procedure.md` step 6) stays mandatory regardless — AC3.
- macOS **test** execution (this is `cargo check`; no test runs on macOS), `clippy`, `--all-features` (not resolvable
  offline here) and a `release-dry-run` darwin leg (D-15-4-F stands).
- `maos-exec-deps` on macOS is compiled EMPTY (crate-level `#![cfg(target_os = "linux")]`, operator ruling Q1
  2026-10-06, §D-20-6-F): there is no macOS port of an ELF/procfs resolver, and none is attempted.
- A portable edit that breaks UNCHANGED macOS-only code in another file (a rename, a changed signature) is not seen by
  a per-file filter — stated in the job and in tag-procedure step 6; the rehearsal is what covers it.
- macOS **runtime** behaviour of T2 (`security/sandbox/macos.rs` is a Seatbelt wrapper, never exercised by a test
  here): this story proves the code *compiles*.

## Story

As **the operator who ships `aarch64-apple-darwin` through `release.yml` and the Homebrew formula**,
I want **every change that can break the macOS build to compile for macOS in the pull request that makes it, and the
pipeline to say plainly when it did not look**,
so that **a macOS break is found on the PR that caused it, not three rehearsal rounds into a release.**

## Acceptance Criteria

*(Epic §20-6, tightened against measurement; no AC added. Each cites what it changes.)*

1. **AC1 (command).** `.github/workflows/discipline.yml` gains `macos-scope` (ubuntu; reads the diff; outputs
   `applicable` + `reason`) and `macos-check`, which runs
   `cargo check --locked --workspace --all-targets --target aarch64-apple-darwin` on
   `macos-latest` — the runner label of `release.yml:50`'s darwin leg, pinned equal by test — **only** when
   `needs.macos-scope.outputs.applicable == 'true'`. The filter is applicable on: `**/macos.rs`,
   `crates/maos-kernel-core/src/security/sandbox/**`, `crates/maos-bin/src/{purge,operator_door}.rs`, `Cargo.toml`,
   `Cargo.lock`, `build.rs`, `.cargo/`, `rust-toolchain.toml`, `.github/workflows/release.yml`, any edit inside the
   `macos-scope`/`macos-check` job blocks or any other `discipline.yml` line mentioning macos/darwin, any changed `.rs`
   file containing a platform `cfg` (`unix`/`windows`/`target_{os,family,vendor,env,arch,…}`, wrapped or not) or an
   OS-typed API (`libc::`, `rustix::`, `nix::`, `os::{unix,linux,fd,macos,darwin,windows}` anywhere in a path,
   `MetadataExt`, `/proc/`, and every crate a manifest takes only under `cfg(target_os = "linux")`), or a diff that
   adds/removes such a line; paths inside trees that are not workspace members (`spikes/`, `templates/`, fixture,
   guest and fuzz workspaces) never start it; the diff is the whole pushed range, or the PR merge commit against its
   first parent; an unknown diff fails open. The rule and its cost rationale (D-15-4-F stands otherwise) are stated in
   the job. *(Frontier re-run: `--exclude maos-exec-deps` dropped by operator ruling Q1; the API/outside-tree clauses
   are review patches F3/F4/F9.)*
2. **AC2 (falsifier).** Re-introducing any one of the three measured breaks reds the darwin `cargo check`; a change
   touching none of the filtered paths does not start a macOS runner.
3. **AC3.** A skipped guard is reported by `aggregate` as **NOT APPLICABLE — not a pass** (log, step summary, PR
   comment), and `aggregate` FAILS when the guard was judged applicable yet did not run; both new jobs are in
   `aggregate.needs`. The pre-tag rehearsal (tag-procedure step 6) remains mandatory, and says so.

## Tasks / Subtasks

- [x] **T1 — Rule-9 preflight** (AC1, AC2). Re-measure the epic's premises at `0d73cac3`.
  - [x] The three breaks: fixed at `0c661995` (not present). Locations at HEAD recorded in §Preflight.
  - [x] 17-3b/17-3c Linux-only symbols: none reach the darwin graph.
  - [x] What CI runs for macOS today: `release.yml` darwin leg only (label-gated PR / push tag / dispatch).
  - [x] OLD→NEW edits applied to `epic-20-ship-it-w5.md` §20-6, status header, table row.
- [x] **T2 — The jobs** (AC1, AC3). `macos-scope`, `macos-check`, `aggregate` wiring + disposition step + PR-comment row.
- [x] **T3 — The contract test** (AC1, AC2, AC3). `xtask/tests/macos_compile_guard_20_6.rs`: structure pins and the
  filter + disposition scripts EXECUTED against planted diffs. Draft: 22 tests, 17 mutations each red. Frontier re-run:
  **27 tests**; 12 more mutations (M18–M29) each red (§Evidence).
- [x] **T4 — Local darwin proof** (AC2). `cargo check --target aarch64-apple-darwin`: green at HEAD; each of the three
  breaks re-planted → the original errors; restored → green. Re-run on `32420928` (wasmtime 48, rustc 1.99.0) with the
  whole workspace, plus the 4th (O_PATH) re-plant. Evidence in `20-6-evidence/`.
- [x] **T5 — Docs** (AC3). `docs/release/tag-procedure.md` step 6 states the guard does not replace the rehearsal and
  names its blind spots; `discipline.yml`'s commented darwin matrix line cross-references the exception.
- [x] **T6 — Gates, fmt, xtask suite** (§Completion Notes).
- [x] **T7 — `bmad-code-review`** (§Review Findings). Draft pass: 14 raw → 10 distinct, 10 patched. **Frontier pass
  (fresh):** 13 raw → 10 distinct → 9 patched, 1 dismissed, 0 deferred, 0 decisions.
- [x] **T9 — Operator rulings 2026-10-06** (frontier re-run). Q1: crate-level `#![cfg(target_os = "linux")]` on
  `maos-exec-deps` (lib + its integration test), `--exclude` dropped, kloc 357 → 358. Q2: declined (no `sonnet-5-5`
  token; frontier re-run instead). Q3: keep `macos-latest`.
- [x] **T8 — CI proof (parent's push).** First real `macos-check` run on a macOS runner: green on this story's PR
  (it changes `discipline.yml` macOS lines, so the filter starts it), and RED on a pushed re-plant of one break
  (AC1's runtime half, AC2's pushed half). **Both halves observed in CI 2026-10-07.**
  - **Green half:** push of `86d5f8f0` to `main`, discipline run `37532509983`: `macos-scope` success (job
    `112505181482`) started the guard; `macos-check` success on `macos-latest` (job `112516208814`), its
    `cargo check (aarch64-apple-darwin)` step ran 188 s from a cold cache (2 s restore).
  - **Red half:** PR #8 from throwaway branch `ci/20-6-t8-replant` (`57748f8f` = `main` minus the crate-level
    `#![cfg(target_os = "linux")]` in `crates/maos-exec-deps/src/lib.rs`, operator-approved, never merged), discipline
    run `37654943930` (`pull_request`): `macos-scope` success (job `112907591624`) judged it applicable; `macos-check`
    FAILED on `macos-latest` (job `112913629046`) at `cargo check (aarch64-apple-darwin)` after 173 s, exit 101 — the
    same command that exits 101 locally with E0425 `libc::O_PATH` on this re-plant (the error text is in the job log,
    which needs a token; the annotation carries the exit code). The branch was deleted afterwards.

## Dev Notes

### Preflight (rule 9, measured 2026-10-06 @`0d73cac3`)

| # | Epic premise | Measured | Disposition |
|---|---|---|---|
| P1 | `macos.rs:41` E0425 (`cgroup_path`) | **Fixed** — `0c661995` deleted the line; `macos.rs` (112 lines) no longer names `cgroup_path`. Re-plant reproduces `E0425: cannot find value cgroup_path`. | AC2 = re-plant |
| P2 | `st_dev` hard-coded `u64` in `purge.rs`, `operator_door.rs` (7 × E0308/E0277) | **Fixed** — `purge.rs:1400`, `operator_door.rs:1980-1981` use `rustix::fs::Dev`. Re-plant: purge.rs 3 errors, operator_door.rs 4 errors = **7**, E0308 + E0277 `can't compare i32 with u64`. | AC2 = re-plant |
| P3 | AC1 command `cargo check --locked -p maos-bin -p maos-cli` | Green at HEAD. Narrower than "every crate/source that compiles on macOS". `--workspace --all-targets` is green **except** `maos-exec-deps` (P4). | OLD→NEW (epic) |
| P4 | *(not in the epic)* | **A fourth break.** `crates/maos-exec-deps/src/lib.rs:218` `libc::O_PATH` — E0425 on darwin. 17-6's crate (2026-09-28, after the rehearsal): ELF parser + procfs reopen, Linux-only by construction; `maos-kernel-core` takes it only under `[target.'cfg(target_os = "linux")'.dependencies]` (`Cargo.toml:70-75`). `cargo tree --target aarch64-apple-darwin -p maos-bin -i maos-exec-deps` prints nothing. | §D-20-6-F (draft: `--exclude`; operator ruling Q1: crate-level cfg) |
| P5 | 17-3b/17-3c Linux-only symbols (memfd, cgroup, `pre_exec`, `prctl`, seccomp)? | None reach darwin: `libc::SYS_*` (incl. `SYS_memfd_create`, `linux.rs:328`), `seccompiler`, `landlock`, the Landlock/seccomp `pre_exec` closure all live in `security/sandbox/linux.rs` behind `#[cfg(target_os = "linux")]` (`sandbox/mod.rs:11,99-180`). `prctl` appears only in `maos-wasm-host/test-fixtures/forbidden-syscall-probe` (a separate manifest, not a workspace member). `macos.rs` is a Seatbelt wrapper with its own `pre_exec` (setrlimit only). | none |
| P6 | What CI runs for macOS today | `release.yml` `build` job: matrix leg `aarch64-apple-darwin` on `macos-latest` (`:49-51`), `cargo build --release -p maos-bin -p maos-cli --locked`; runs on tag push, `workflow_dispatch`, and PRs **only with the `release-rehearsal` label** (`:26-37`). `discipline.yml` has no macOS job: `release-dry-run`'s darwin entry is commented out (D-15-4-F, `:4043`). | this story |
| P7 | Runner pinning ("pin explicitly; not `*-latest` if a house rule forbids it") | **No such rule.** 180 of 184 `runs-on` lines under `.github/workflows/` are `ubuntu-latest`; the others are `windows-latest`, `ubuntu-24.04`, and two matrix-driven (`release.yml` `matrix.os`, 17-6's `matrix.image`). | §D-20-6-B |
| P8 | `aggregate` semantics for a skipped job | `aggregate` is `if: always()` and fails only on `failure`/`cancelled` in `needs.*.result` (`discipline.yml` "Check for failures" / "Fail if any gate failed"): **a skipped job is invisible** — exactly AC3's "silent pass". | §D-20-6-E |
| P9 | Diff convention for path-driven jobs | `invariant-lock` uses `git diff --name-only HEAD~1` (`:524`) — the LAST COMMIT, wrong for a multi-commit push. | §D-20-6-D |

### Design decisions

**D-20-6-A — two jobs, not a `paths:` filter.** One workflow serves ~165 jobs, so a workflow-level `on.*.paths` would
gate all of them. A job-level `if` cannot read the diff, so a cheap Linux `macos-scope` job computes it first.
`macos-scope` is also the only way to give `aggregate` a *reason* to print (AC3).

**D-20-6-B — `macos-latest`, equal to `release.yml`'s darwin leg.** The guard exists to compile what the release
compiles; two copies of a label drift. `xtask/tests/macos_compile_guard_20_6.rs` derives the expected label from
`release.yml`'s matrix, so changing one without the other reds the suite. An image flip (`macos-latest` moving) then hits
the guard and the release together, which is the point. Operator question Q3 asked whether to pin both to an explicit
image; **ruled 2026-10-06: keep `macos-latest`**, matching `release.yml`.

**D-20-6-C — `--workspace --all-targets`, no exclusion.** A superset of the release's `-p maos-bin -p maos-cli`:
test and bench code and crates the release does not ship also compile for macOS (a macOS workstation runs
`cargo test`). `--all-targets` is `check`, not `build`: nothing links, so it needs no macOS SDK beyond what cargo
needs. No `-D warnings` (the tree carries warnings today; this is a compile guard, not a lint gate).

**D-20-6-D — the filter reads what the three real breaks looked like.** Rule (1) is the epic's named paths plus the
inputs that change *what* compiles (`build.rs`, `.cargo/`, toolchain), matched on a NUL-separated path list so a
non-ASCII path is never C-quoted past it. Rule (2) lets this job's own definition start itself — any edit inside the
`macos-scope`/`macos-check` blocks (compared base vs head, so a change on a line that never says "macos" still counts),
or any other `discipline.yml` line mentioning macos/darwin — so the story's own PR exercises the guard without every
`discipline.yml` edit paying for a macOS runner. Rule (3): `purge.rs` already had `cfg(unix)` but its failing `st_dev`
sites had none, so an OS-typed API (`libc::`, `rustix::`, `std::os::unix`, `MetadataExt`, `/proc/`) counts like a `cfg`;
the cfg regex is line-oriented and also matches the bare predicate lines rustfmt leaves when it wraps a `cfg(any(…))`,
and `target_env`/`target_arch`. It is judged on the file at HEAD **and** on the removed/added lines, because deleting
the last `cfg` from a file leaves a file with none. The base is the **first parent of the PR merge commit**
(`pull_request.base.sha` is not refreshed when `main` moves — review finding R10) or `github.event.before` (the **whole
pushed range**, not `HEAD~1`). **An unknown diff fails open** (no/zero/unreachable base ⇒ the guard runs): a filter that
fails closed turns a force-push into a silent skip. Outputs are sanitised: a path may carry a newline.
*Frontier review patches:* the removed/added-line pass cuts the `+`/`-` prefix before matching, so the anchored
bare-predicate alternative works there too (F2); OS-typed modules are matched as `os::{unix,linux,fd,macos,darwin,windows}`
anywhere in a path and as `std::os::{` (F4); every crate a manifest takes only under `cfg(target_os = "linux")`
(`landlock`, `seccompiler`, `maos_exec_deps`) counts as an OS-typed API, the list pinned against the manifests by test
(F3); paths in trees that are not workspace members (`spikes/`, `templates/`, root and xtask `tests/fixtures/`, and any
`fuzz/`, `guests/`, `test-fixtures/` workspace) are dropped before rules (1) and (3), with a test that no workspace
member matches (F9). A bare `unix`/`windows` line that is an identifier, not a predicate, still counts (F6, dismissed:
it fails safe — one more runner, never a missed break; one file in the tree, `maos-bin/src/cert_rotation.rs`).

**D-20-6-E — NOT APPLICABLE is said, and an applicable guard that did not run is red.** `aggregate` gets a
"macOS guard disposition" step that prints (log + `$GITHUB_STEP_SUMMARY`) one of: RAN and PASSED; **NOT APPLICABLE
(not a pass)** + the pre-tag-rehearsal reminder; NOT RUN because `macos-scope` failed (already red); or SKIPPED
although `macos-scope` said applicable — which sets `unran=true`, failed by the **last** step of the job (after the PR
comment, so the comment is refreshed instead of left stale by an early `exit 1` — review finding R1). The PR
comment row renders "NOT APPLICABLE (not a pass)" only behind `macos-scope == success && applicable == false`;
any other skip renders "NOT RUN" (R2). Both jobs are in `aggregate.needs`, so a red guard reds the aggregate.

**D-20-6-F — `maos-exec-deps` compiles EMPTY off Linux (operator ruling Q1, 2026-10-06).** It parses ELF and re-opens
through `/proc/self/fd`; there is no macOS meaning to port, and a `O_PATH = 0` stub would compile and silently open the
wrong thing (typed refusals stay typed — nothing is stubbed). The draft excluded it as a root
(`--exclude maos-exec-deps`) and filed the alternative as Q1; the operator ruled the alternative: a crate-level
`#![cfg(target_os = "linux")]` in `src/lib.rs` (plus the same line on its integration test `tests/resolver_17_6.rs`,
which otherwise fails to resolve the crate under `--all-targets`), the guard checks the WHOLE workspace, and
`xtask/kloc.toml` `maos-exec-deps` 357 → 358 (exact measured, ZERO headroom). Dependents: only `maos-kernel-core`, under
`cfg(target_os = "linux")`, with its two use sites in `security/sandbox/linux.rs`; the fuzz crate is a separate
workspace. `cargo check --workspace` therefore works on a developer's Mac, and deleting the gate is a darwin red
(E0425 `O_PATH`, re-planted on `32420928`). The test pins the gate in both files' crate header and pins that the guard
carries no `--exclude`/`-p` narrowing.

**D-20-6-G — the test EXECUTES the scripts.** `macos_compile_guard_20_6.rs` extracts the `macos-scope` and aggregate
disposition `run:` bodies from the real YAML and runs them under `bash` in throwaway git repositories with planted
diffs. A Rust reimplementation of the regex would prove the Rust, not the job (the 17-6 rule RT-10: a planted fault is
proven only with its diff).

### Governance

- **Kernel:** none. **KLOC:** `maos-exec-deps` 357 → 358, operator grant 2026-10-06 (Q1), applied by the frontier re-run at
  the exact measured figure; no other row. **Ratification / surface:** none.
- **Model allowlist:** the draft requested `"sonnet-5-5"` in `FRONTIER_FAMILIES`; the operator **declined** (2026-10-06)
  and ruled a frontier re-run. Dev of record `anthropic/claude-opus-5-5` matches the existing `opus-5` token;
  `FRONTIER_FAMILIES` is untouched and `check-dev-model-tier` is green.

### Operator questions (for the end; none blocked the story)

- **Q1 — RULED 2026-10-06:** cfg the crate, drop the `--exclude`, kloc 357 → 358. Applied (§D-20-6-F).
- **Q2 — RULED 2026-10-06:** declined; frontier re-run instead. Applied (this record).
- **Q3 — RULED 2026-10-06:** keep `macos-latest`, matching `release.yml`. Applied (no change).
- **Q4.** AC1's command moved from `-p maos-bin -p maos-cli` to `--workspace --all-targets` (OLD→NEW in the epic): accept
  the extra cold-cache runner time for the wider net? (Not measured on a runner — the ceiling is 45 min, mirroring
  `release.yml`; tighten to 3× the first measured run per E16-A2.)
- **Q5.** Filter breadth: `libc::`/`std::os::unix`/`/proc/` in a changed `.rs` file starts the guard (the `purge.rs`
  class). Broad on purpose; say if it starts macOS runners too often once there is data.

### What this story could NOT prove locally (named, for the parent's CI)

1. **The first real macOS-runner execution** of `macos-check` (AC1 green; AC2's pushed re-plant red) and the GitHub
   expression plumbing (`github.event.before`, `github.sha` as the PR merge commit and its `^1`, `needs.*.outputs.*`,
   the `if`, the PR-comment github-script — executed locally under node with stubs, §Evidence).
2. **The C halves** of `ring` and `libsqlite3-sys` for darwin: this host has no macOS SDK, so a throwaway shim
   (`20-6-evidence/darwin-check-cc-shim.sh`) stood in as the C compiler — every *Rust* type-check is real, the C
   compilation of those two `-sys` crates is not exercised. `release.yml`'s darwin leg (`done`, run 36089542264) is the
   standing evidence that they compile on a runner.
3. **`--all-features`**: not resolvable offline here (`axum` is not in the offline cache). The job does not use it.
4. Cold-cache macOS wall time (budget decision Q4).

### References

- Epic: `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md` §20-6 (OLD→NEW applied), table row `:68`.
- `0c661995` (the three fixes; FLAG-Winston kernel re-pin 24922 → 24920); sprint rows `ops-darwin-rehearsal`, `20-6-macos-compile-guard`.
- D-15-4-F: `15-4-release-repair-and-first-signed-tag.md` AC2; `docs/release/tag-procedure.md` step 6; `release.yml:1-50`.
- `check_release_precondition.rs:7,53` (the release looks for `aggregate` `completed/success` on the tagged SHA).
- Precedents: `release_workflow_15_4.rs` (workflow-as-config test), 17-6 RT-10 (show the planted diff).

### Project Structure Notes

CI plus one docs paragraph, one integration test beside its precedent (`xtask/tests/release_workflow_15_4.rs`), and
the operator-ruled crate gate (`crates/maos-exec-deps/src/lib.rs`, its `tests/resolver_17_6.rs`, `xtask/kloc.toml`).
No new crate, no new dependency (`serde_yaml`, `tempfile`, `toml`, `regex` already in `xtask`), no `main.rs` line.

## Evidence (`_bmad-output/implementation-artifacts/20-6-evidence/`)

- *(draft, @`0d73cac3`, rustc 1.98.1)* `aarch64-apple-darwin-check.log` — the green runs at HEAD (`-p maos-bin -p maos-cli`;
  `--workspace --exclude maos-exec-deps` with and without `--all-targets`), the red `--workspace` (P4: `E0425 O_PATH`),
  the empty `cargo tree`, the unresolvable `--all-features`.
- `aarch64-apple-darwin-falsifier.log` — the three re-plants with their diffs and the errors they produce, then the restored tree green.
- `scope-filter-mutation-proofs.log` — seventeen mutations of the workflow (one per filter rule, the PR base selection, the
  NUL separation, the fail-open branch, the runner label, `aggregate` needs/steps/comment wiring), each with its diff and
  the contract test that goes red; the unplanted suite green.
- `darwin-check-cc-shim.sh` — the local stand-in C compiler (no macOS SDK here). Evidence tooling only; not used by CI.
- **Frontier re-run (2026-10-06 @`32420928`):** `aarch64-apple-darwin-frontier-rerun.log` — the guard's exact command
  (whole workspace, no exclusion) green from a cold darwin target dir on rustc 1.99.0 / wasmtime 48; the O_PATH
  re-plant (crate cfg deleted) red with E0425; the three historical re-plants red again (E0425; 3; 4 errors); restored
  green. `frontier-review-mutation-proofs.log` — mutations M18–M29 (operator-ruling pins and the review patches), each
  with its diff and the test that goes red; the 27-test suite green unplanted; the PR-comment github-script executed
  under node for six result combinations (and the draft's order throwing `ReferenceError` — F1).

## Review Findings

### Frontier §A6 pass (2026-10-06, anthropic/claude-opus-5-5) — FRESH; the draft's conclusions were not reused

*bmad-code-review over the frontier re-run's diff (`git diff HEAD` of `.github`, `crates`, `docs`, `xtask` plus the new
test file, 1134 lines): Blind Hunter (diff only, `bmad-review-adversarial-general`), Edge Case Hunter (diff + project
read access, `bmad-review-edge-case-hunter`, experiments with the extracted scripts in scratch repos), Acceptance
Auditor (diff vs the ACs with the 2026-10-06 rulings + epic §20-6), as three separate read-only subagents at the session
model's capability. Dev model `anthropic.*` ⇒ the Test-Infrastructure axis is a no-op. No layer failed or returned
empty.*

**13 raw findings (Blind 7, Edge 3, Auditor 3) → 10 distinct (3 merged duplicates) → 9 patched · 1 dismissed ·
0 deferred · 0 decisions needed.** Every patch is proven by a planted mutation that reds the 27-test contract
(`frontier-review-mutation-proofs.log` M23–M29) or, for the docs/comments, by the text itself.

- [x] [Review][Patch] **F1 (Edge+Auditor, HIGH) — the PR-comment script read `const icon` before its declaration.**
  `mcsCell` (draft) called `icon(mcs)` above `const icon = …`: a temporal-dead-zone `ReferenceError` on every pull
  request where `macos-check` RAN (success, failure, cancelled) — the github-script step fails, the comment is never
  refreshed, and `aggregate` (the branch-protection check) reds, including on this story's own PR. Reproduced under
  node (draft order: `THREW ReferenceError`). The draft's substring test could not see it. **Fixed:** `mcsCell` moves
  below `icon`/`advisoryIcon` with a comment saying why; new test `the_pr_comment_script_reads_no_const_before_its_declaration`
  scans every `const` (incl. destructuring) for an earlier identifier-bounded read; M23 reds it; the fixed script
  executed under node for six result combinations renders the right row each time [.github/workflows/discipline.yml:4917]
- [x] [Review][Patch] **F2 (Edge+Blind, med) — the removed/added-line pass never cut the `+`/`-` prefix**, so the anchored
  bare-predicate alternative could not match there: removing a rustfmt-wrapped `#[cfg(all(⏎ unix, …))]` judged NOT
  APPLICABLE (reproduced). **Fixed:** `| cut -c2-` before the final grep; test `removing_a_wrapped_bare_unix_cfg_starts_the_guard`;
  M24 [.github/workflows/discipline.yml:4235]
- [x] [Review][Patch] **F3 (Edge+Blind, med) — Linux-only crates were not OS-typed APIs.** An un-gated `landlock::`,
  `seccompiler::` or `maos_exec_deps::` (absent/empty on darwin — the latter by this story's own gate) in a file with no
  other marker judged NOT APPLICABLE (reproduced). **Fixed:** added to `API_RE`; test
  `every_linux_only_dependency_is_an_os_typed_api_for_the_filter` DERIVES the list from every member manifest's
  `cfg(target_os = "linux")` table minus what macOS also gets, so a new Linux-only dependency reds until the filter
  names it; M25 [.github/workflows/discipline.yml:4223]
- [x] [Review][Patch] **F4 (Blind, med) — grouped `std` imports escaped `std::os::(unix|linux|fd)`**
  (`use std::{io, os::linux::net::SocketAddrExt};`, `use std::os::{fd::…, linux::…}`), and `std::os::{macos,darwin,windows}`
  were not listed (reproduced). **Fixed:** `(^|[^A-Za-z0-9_])os::(unix|linux|fd|macos|darwin|windows)\b|std::os::\{`;
  three new positive cases and one negative (`std::os::raw`, portable); M26 [.github/workflows/discipline.yml:4223]
- [x] [Review][Patch] **F5 (Blind, low) — the cross-file blind spot was not stated:** a portable rename that breaks
  UNCHANGED macOS-only code elsewhere is judged NOT APPLICABLE (inherent to a per-file filter, which the operator ruled).
  **Fixed (docs):** stated in the job's rule-(3) comment and in tag-procedure step 6's list of what the filter cannot
  see; the rehearsal stays the net [docs/release/tag-procedure.md:81-83]
- [x] [Review][Patch] **F7 (Blind, low) — the runner comment claimed "every job uses a `*-latest` label"** (false:
  `discipline.yml:3022` matrix of pinned images, `rpo-rto-cadence.yml:95`). **Fixed:** replaced by the operator ruling
  (keep `macos-latest`, equal to `release.yml`) [.github/workflows/discipline.yml:4247]
- [x] [Review][Patch] **F8 (Blind, low) — the "only macOS job" pin ignored matrix-driven `runs-on`.** Adding
  `macos-latest` to a `${{ matrix.image }}` job would put a second billed macOS runner on every PR with the test green.
  **Fixed:** the test also scans the job's `strategy.matrix` when `runs-on` is an expression; M27
  [xtask/tests/macos_compile_guard_20_6.rs:214-244]
- [x] [Review][Patch] **F9 (Auditor, low) — `Cargo.toml`/`Cargo.lock` matched at any depth**, so a spike, fuzz, guest or
  fixture workspace's lockfile started a billed macOS runner (the Auditor's history replay: `5fa8843b`, applicable only
  through a spike lockfile). **Fixed:**
  `OUTSIDE_RE` drops non-member trees before rules (1) and (3); test `no_workspace_member_is_treated_as_outside_the_workspace`
  reads `workspace.members` from the root manifest and proves none matches, and executes a member-manifest change;
  six negative cases (spike lock, fuzz manifest, guest source, fixture `macos.rs`, template manifest, xtask fixture
  source); M28/M29 [.github/workflows/discipline.yml:4170]
- [x] [Review][Patch] **F10 (Auditor, low) — docs drift after the ruling:** the epic and this story still said
  `--exclude maos-exec-deps`, `kloc 0` and `22 tests`. **Fixed:** epic §20-6 (Δ line, table row, OLD→NEW, premise,
  AC1) and this story (frontmatter, AC1, tasks, §D-20-6-C/F, governance, evidence, completion notes)
  [_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:160-166]
- **Dismissed (1): F6 (Blind) — a line holding only the identifier `windows`/`unix` counts as a bare cfg predicate**
  (`maos-bin/src/cert_rotation.rs:89`, the one such file in the tree). The error is fail-SAFE (an extra macOS runner on
  edits to one file, never a missed break); narrowing the anchored alternative to attribute context would need
  multi-line state the `-U0` pass does not have, trading a measured over-trigger for an under-trigger. Recorded in
  §D-20-6-D; operator question Q5 (filter breadth) already covers tuning once there is runner data.
- [x] **S2 (self-found by the frontier dev, outside the 14-gate list) — the draft redded `check-epic-close-coherence`**, a
  gate in `aggregate.needs`: it flipped `epic-20` to `in-progress` in sprint-status and the epic header but not in the
  planning index (`epics/index.md:162` still said `backlog` → `prose-status-mismatch`). **Fixed:** index row →
  `in-progress` (and 20.6 added to its stories line); gate `PASSED (21 epics re-derived against pin 25293)`.

### Draft review (anthropic/claude-sonnet-5-5, 2026-10-06) — history, kept verbatim

*bmad-code-review, 2026-10-06 — Blind Hunter (diff only), Edge Case Hunter (diff + experiments in throwaway repos),
Acceptance Auditor (diff vs this story + epic §20-6), run as separate read-only subagents at the session model's
capability. The dev model is `anthropic.*`, so the Test-Infrastructure axis of `_bmad/custom/bmad-code-review.user.toml`
is a no-op; the test file was reviewed under the same three layers. No layer failed or returned empty.*

**14 raw findings → 10 distinct (4 merged duplicates) → 10 patched · 0 dismissed · 0 deferred · 0 decisions needed**,
plus 1 self-found hardening (S1). Every patch below is re-proven by the 22-test contract and the 17-mutation log.

| ID | Src | Sev | Finding | Disposition |
|---|---|---|---|---|
| R1 | Auditor, Edge | med | The disposition step's `exit 1` (applicable but skipped) sat BEFORE the PR-comment step, so the comment was skipped and an earlier all-green table stayed. | **Patched.** The step records `unran=true`; a new LAST step "Fail if the macOS guard was applicable but did not run" fails the job after the comment. Test pins step order, `id`, and the `if`. |
| R2 | Auditor, Edge, Blind | med | The PR-comment row printed "not applicable — no platform-sensitive path" for ANY skipped `macos-check`, including one skipped because `macos-scope` failed; and never said "not a pass". | **Patched.** `mcsCell`: "NOT APPLICABLE (not a pass)" only behind `scope == success && applicable == false`; every other skip renders "❌ NOT RUN — macos-scope …". Test + mutation M15. |
| R3 | Auditor | low | Epic AC2 / sprint row claimed a mutation per filter branch; the log had none for rule 2 or the removed-line scan. | **Patched.** Log now has 17 mutations incl. rule 2a, 2b, 3b individually (M7/M8/M9); wording corrected. |
| R4 | Auditor | low | Epic said the first CI run exercises `--all-features`; the job never passes it. | **Patched.** Epic + story say it stays unproven and out of scope. |
| R5 | Auditor | med | T6/T7 ticked while Completion Notes / Review Findings were placeholders. | **Patched.** This section and §Completion Notes. |
| R6 | Auditor | low | `release.yml` label-gate cite `:28-36` off by two lines. | **Patched** to `:26-37`. |
| R7 | Edge | med | `git diff --name-only` C-quotes non-ASCII paths, so `crates/é/macos.rs` / `crates/ü/Cargo.toml` judged NOT APPLICABLE (reproduced). | **Patched.** `git diff -z` into a temp file, `mapfile -d ''`, bash `=~` per path. Test + M11. |
| R8 | Edge, Blind | med | The cfg regex was line-oriented with `cfg(` and the key on one line: a rustfmt-wrapped `#[cfg(any(⏎ target_os = …))]` was missed (new file, body edit, and un-gating — all reproduced); `target_env`/`target_arch` were not predicates at all. | **Patched.** Bare `target_{os,family,vendor,env,arch,pointer_width,endian} =` and bare `unix`/`windows` predicate lines match. Test (5 shapes) + M12. |
| R9 | Edge | med | An edit inside `macos-check`'s own steps (`toolchain:`, `timeout-minutes:`, `uses:`) never said macos/darwin, so a broken guard merged unexercised (reproduced). | **Patched.** Rule 2a compares the `macos-scope`/`macos-check` job blocks base vs head. Test + M7. |
| R10 | Blind | med | `pull_request.base.sha` is not refreshed when `main` moves; diffing it against the merge commit charges the PR for other people's commits and starts macOS runners it did not earn. | **Patched.** For PRs the base is `git rev-parse HEAD^1` (first parent of the merge commit). Test builds the stale-base scenario (control: the stale range DOES trip) + M10. |
| S1 | self | low | A path containing a newline could append a forged `applicable=false` line to `$GITHUB_OUTPUT` via the `reason`. | **Patched.** `emit` strips CR/LF. Test (exactly one verdict line) + M13. |

**Review-layer coverage honestly bounded:** none of the three layers had a macOS runner either; R-level claims about
GitHub-side behaviour (`github.sha` is the merge commit for `pull_request`; `needs.*.outputs` of a skipped dependent;
`github.event.before` on the first push of a branch) rest on GitHub's documented semantics and the fail-open branch,
and are T8's to observe.

## Dev Agent Record

### Agent Model Used

anthropic/claude-opus-5-5 — dev of record (frontier re-run, operator ruling 2026-10-06), in the detached worktree
`/tmp/maos-20-6-wt` @`32420928`; its §A6 review layers ran as separate subagents at the same capability.

Initial draft (history, kept): anthropic/claude-sonnet-5-5 (effort high) — story creation, dev and review orchestration
in one session, in the same worktree @`0d73cac3`; its review layers ran as separate subagents at that model's capability.

Frontier re-run (operator ruling 2026-10-06): the initial draft and its review were produced by
anthropic/claude-sonnet-5-5; this pass re-verified every AC, applied the operator rulings Q1–Q3 (crate-level
`#![cfg(target_os = "linux")]` on `maos-exec-deps` and its integration test, `--exclude` dropped, kloc 357 → 358,
`macos-latest` kept, no `sonnet-5-5` token) and the nine frontier-review patches F1–F5, F7–F10 (PR-comment TDZ
`ReferenceError`, `+`/`-` prefix cut, Linux-only crates and grouped `os::` imports as OS-typed APIs, non-member trees
skipped, matrix-aware only-macOS pin, blind-spot docs, runner comment, docs drift), and re-ran §A6 review.

### Debug Log References

- `20-6-evidence/aarch64-apple-darwin-check.log`, `aarch64-apple-darwin-falsifier.log`, `scope-filter-mutation-proofs.log` (draft).
- `20-6-evidence/aarch64-apple-darwin-frontier-rerun.log`, `frontier-review-mutation-proofs.log` (frontier re-run).
- Frontier method: worktree's own `target` symlink (`unset CARGO_TARGET_DIR`), `RUSTUP_TOOLCHAIN=1.99.0` (the
  `aarch64-apple-darwin` std was added to that toolchain for the check), `RUSTC_WRAPPER=`, the draft's CC shim. The very
  first warm darwin run of the session exited 101 with its error text cut by my own `| head`; five later runs (one cold,
  one on 1.98.1 then 1.99.0) all exit 0 on the identical tree — recorded, not reproduced.
- Draft method note: every darwin `cargo check` used `CARGO_TARGET_DIR=/mnt/build/maos-20-6-tgt`, `RUSTC_WRAPPER=` (sccache
  breaks cc-rs compiler detection) and `CC_aarch64_apple_darwin=<shim>`.
- An early edit-tool call resolved a relative path against the main checkout and wrote 162 lines into
  `/home/lunarpulse/dev_ws/.github/workflows/discipline.yml`; the diff was saved, reverse-applied there (main is clean:
  `git status --porcelain` empty) and applied byte-identically here.

### Completion Notes List

- **Result (frontier re-run, 2026-10-06):** story `review`, not `done`, pending T8 — the first real `macos-check` run on
  a GitHub macOS runner (AC1's runtime half and AC2's pushed half), including the C halves of `ring`/`libsqlite3-sys`.
  Everything else was green locally (below). **Superseded 2026-10-07: T8 observed in CI, both halves (§Tasks T8);
  story `done`.**
- **AC1 (frontier re-verified):** the job runs exactly `cargo check --locked --workspace --all-targets --target
  aarch64-apple-darwin` on `macos-latest` behind `needs.macos-scope.outputs.applicable == 'true'` (pinned by
  `macos_check_runs_on_release_yamls_darwin_runner_and_checks_the_whole_workspace`, no `--exclude`/`-p`). Local darwin
  proof on `32420928` (host linux x86_64, rustc 1.99.0, wasmtime 48, shim C compiler): that command (`--offline` in
  place of the network) **exit 0 from a cold `target/aarch64-apple-darwin`** (25.7 s), and
  exit 0 again on the final tree. `cargo tree --workspace --target aarch64-apple-darwin -i maos-exec-deps` = the crate
  alone (a root, no dependent). The filter script is EXECUTED by 16 of the 27 contract tests, the disposition step by 3.
- **AC2 (frontier re-verified):** the three re-plants on this base red the guard's command with the original errors —
  `macos.rs` E0425 `cgroup_path`; `purge.rs` E0308 ×2 + E0277 (3); `operator_door.rs` E0308 ×3 + E0277 (4) — and the
  4th, deleting the `maos-exec-deps` crate gate, reds with E0425 `O_PATH` (lib and lib test); each restore is green and
  `git status -- <file>` is empty. "No macOS runner for an unrelated change": `changes_touching_none_of_the_filtered_paths_start_no_macos_runner`
  (13 planted diffs incl. the non-member trees) asserts `applicable=false` with "NOT a pass".
- **AC3 (frontier re-verified):** disposition step executed by 3 tests over every result combination; the PR-comment
  script executed under node for six combinations (rows: `✅ success`, `❌ failure`, `❌ cancelled`, `➖ NOT APPLICABLE (not a
  pass)…`, `❌ NOT RUN — macos-scope failure, applicable=unset`, `❌ NOT RUN — macos-scope success, applicable=true`), after
  F1 — the draft's order threw `ReferenceError` whenever the guard ran. Both jobs in `aggregate.needs`; the final step
  fails on `unran`. tag-procedure step 6 says the rehearsal stays mandatory and lists the filter's blind spots.
- **`cargo test --offline --locked -p xtask`: 968 passed, 0 failed** (base 941 + the 27 contract tests);
  `kloc_check::tests::kloc_check_runs_on_workspace` ok. `cargo fmt --all --check`: exit 0. Toolchain rustc 1.99.0.
- **Gates (`cargo run -q --offline --locked -p xtask -- <gate>`, rustc 1.99.0, after `cargo build -p maos-bin -p maos-cli`
  and `-p maos-cli --bin maosctl`), 14 of 14 GREEN:** `check-kernel-baseline` (25293 lines, 98 files, pinned 25293),
  `kloc-check --json` (`passed: true`, `over_budget: []`, maos-exec-deps 358, xtask 44040, aggregate 170434),
  `check-dev-record-completeness` (164 done stories), `check-exit-commands` (5 epics, 15 tokens), **`check-dev-model-tier`
  (PASS — 59 frontier-era stories, all allowlisted with a §A6 artifact)**, `check-dev-model-used-populated` (PASS with
  43 advisory warnings, one of them this story's `unknown model: anthropic/claude-opus-5-5` — the same advisory every
  `opus-4-8`/`opus-4-6` story carries; the blocking tier gate accepts it), `check-fkcs --json`, `templates-regen --check`, `gen-abi-docs --check`,
  `check-manifest-schema-version` (5/1/5), `stability-matrix --check`, `check-equiv-fixture-provenance` (4 fixtures),
  `check-wasm-form-equiv --json`, `check-service-boundary` (0 violations). Also `check-serde-error-handling --json`
  (passed), `check-ship-gate-completeness` (41 gates) and `check-epic-close-coherence` (PASSED, 21 epics — after S2).
- **Governance:** only `xtask/kloc.toml` `maos-exec-deps` 357 → 358 (operator grant, exact measured). Unchanged:
  `xtask/kernel-core-baseline.toml`, `abi-ratifications.toml`, `FRONTIER_FAMILIES`/model allowlists. Kernel-Δ 0.
- **Draft notes (history, @`0d73cac3`, rustc 1.98.1):** draft proved `--workspace --exclude maos-exec-deps --all-targets`
  exit 0 and `--workspace` alone exit 101 `E0425 O_PATH`; 963 xtask tests (941 + 22); 13 of 14 gates green with
  `check-dev-model-tier` red only on `anthropic/claude-sonnet-5-5` not being allowlisted (grant requested, then declined).

### File List

- **Modified:** `.github/workflows/discipline.yml` (jobs `macos-scope`, `macos-check`; `aggregate` needs + disposition step + PR-comment row + final unran step; one cross-reference comment)
- **Modified:** `crates/maos-exec-deps/src/lib.rs` (crate-level `#![cfg(target_os = "linux")]` + its doc lines — operator ruling Q1)
- **Modified:** `crates/maos-exec-deps/tests/resolver_17_6.rs` (the same gate on its integration test)
- **Modified:** `xtask/kloc.toml` (`maos-exec-deps` 357 → 358, operator grant)
- **Modified:** `docs/release/tag-procedure.md` (step 6: the guard does not replace the rehearsal; its blind spots)
- **Modified:** `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md` (status header, table row `:68`, §20-6 Δ line, preflight block + AC1–AC3 OLD→NEW)
- **Modified:** `_bmad-output/planning-artifacts/epics/index.md` (Epic 20 row `backlog` → `in-progress`; 20.6 listed — S2)
- **Modified:** `_bmad-output/implementation-artifacts/sprint-status.yaml` (`epic-20` → `in-progress`; `20-6-macos-compile-guard` → `review`)
- **New:** `xtask/tests/macos_compile_guard_20_6.rs` (27 tests)
- **New:** `_bmad-output/implementation-artifacts/20-6-macos-compile-guard.md` (this file)
- **New:** `_bmad-output/implementation-artifacts/20-6-evidence/{aarch64-apple-darwin-check.log,aarch64-apple-darwin-falsifier.log,scope-filter-mutation-proofs.log,darwin-check-cc-shim.sh,aarch64-apple-darwin-frontier-rerun.log,frontier-review-mutation-proofs.log}`

### Change Log

- 2026-10-06 — story created (`ready-for-dev`), developed and reviewed in one automated pass by anthropic/claude-sonnet-5-5; status `review` pending the CI proof T8 and the operator questions.
- 2026-10-06 — operator rulings: Q1 cfg the crate (kloc +1), Q2 declined (frontier re-run), Q3 keep `macos-latest`.
- 2026-10-06 — frontier re-run by anthropic/claude-opus-5-5 on `32420928`: every AC re-verified, rulings applied, fresh §A6
  review (9 patched, 1 dismissed), 14/14 gates green, 968 xtask tests; status stays `review` — T8 (CI) is the only open item.
- 2026-10-07 — T8 green half observed in CI (run `37532509983`, `macos-check` 188 s on `macos-latest`); red half (PR
  with a re-plant) still open; status stays `review`.
- 2026-10-07 — T8 red half observed: PR #8 (`ci/20-6-t8-replant`, the `maos-exec-deps` cfg re-plant), run
  `37654943930`, `macos-scope` applicable, `macos-check` failed at the darwin `cargo check` (exit 101, 173 s). All ACs
  proven; `review` → `done`. Throwaway branch deleted.
