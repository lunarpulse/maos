# Proof contract

Story `20-12-test-cost-budget-and-review-execution-contract` (2026-10-23). This page states the rule once and is the
tracked contract. The dev-story and code-review customizations (`_bmad/custom/bmad-dev-story.user.toml`,
`_bmad/custom/bmad-code-review.user.toml`) mirror it locally — `_bmad/` is gitignored, so they are per-checkout
copies, not the source of truth.

## The rule

**Proven-red on ONE image; green on the matrix once; a story owes one run on its final tree plus one planted-red run
on one image.**

- *Green on the matrix once.* `discipline.yml` runs every job (including the image matrices below) on the story's
  final tree. That single run is the story's green proof. A story does not re-run the matrix to re-prove a tree that
  has not changed.
- *Proven-red on one image.* A planted mutation proves its control can fail. It is run once, on one image
  (`ubuntu-24.04` unless the plant's subject is image-specific), and recorded once under
  `_bmad-output/implementation-artifacts/<story>-evidence/`.
- *Before* (17-8): three full runs × four images = **6.3 job-hours** for one story's exit-line proof
  (`17-8-evidence/ci/run-c1-attempt2.json`, `run-c2.json`, `run-c3.json`).

## Plants (AC7)

- In place in the real tree; restored with `git checkout -- <file>` / `git stash`; rebuilt incrementally. Never a
  workspace copy, never a git worktree built into the shared target dir, never a second `CARGO_TARGET_DIR`.
- At most **one plant per AC** by default; more only where no gate or test can otherwise falsify the AC, each named.
- The Dev Agent Record states the plant count and the per-plant wall-clock.
- `profile.dev` uses `debug = "line-tables-only"` and `split-debuginfo = "unpacked"`, so the relink a plant forces
  writes less.

## Review re-execution (AC5)

- The reviewer re-runs the **focused suites** of the crates the diff touches and the gates those crates enrol, and
  names them with wall-clock in the Review section. A focused suite is
  `cargo nextest run -p <crate> --locked`; when the diff touches `maos-kernel-core`, add
  `cargo test -p maos-kernel-core --lib` (nextest's `default-filter` excludes those unit tests — they share one
  process-global clock base and run under libtest).
- The **workspace suite runs in CI**, not at review.
- Mutation re-plants are recorded once by the dev pass; the review repeats one only for a disputed finding.
- **Closing a story re-runs the tree-bound tests that read its status.** Flipping a sprint key to `done` changes the
  verdict of tests that read `sprint-status.yaml` and the story records: before the closing commit, run
  `cargo nextest run -p xtask --locked --test d11_xtask_ceiling_ratchet --test recovery_lane_ceiling_rule`. (20-12
  closed with an xtask ceiling citing it and no exact `<old> → <new>` arrow grant on an `xtask` line of its record;
  `the_live_xtask_ceiling_cites_a_real_sprint_story` went red only after the flip, and four main runs carried it.)
- **A manifest or admission change widens the focused set to every reader of it.** When the diff changes a checked-in
  manifest (`spirits/*/manifest.toml`, `examples/*/manifest.toml`) or the manifest/capability parser
  (`crates/maos-manifest/src`, `capabilities_required_to_scopes`), the focused set adds every test file that names
  that manifest or parser (`grep -rlE '<manifest path>|from_toml_str|gate_manifest|capabilities_required_to_scopes'
  crates/*/tests spirits/*/tests`), `cargo test -p maos-kernel-core --test spirit_boundary_invariants` and
  `cargo run -p xtask -- check-corpus`. (17-11's review patches added `openai` to the Researcher manifest and refused
  an empty provider prefix; the focused set passed and the landing-tree workspace run found two reds.)
- The four read-only review layers (Blind Hunter, Edge Case Hunter, Acceptance Auditor, Test Infrastructure
  Auditor) are unchanged.
- **Reviewer model (operator ruling 2026-10-24).** Code-review layers run on `anthropic/claude-opus-5-5` or
  `gpt-6.1-sol`. `anthropic/claude-fable-5-1` is reserved for plan review (PRD, architecture, epic, story and
  decision documents) and for code review of a safety- or security-critical change — and every use of it, either
  kind, needs the operator's permission first. 17-9, 17-10 and 17-11 were reviewed on fable-5.1 before this ruling.

## Test cost (AC1/AC2)

- `cargo nextest run --workspace --locked --no-fail-fast --profile ci` is the CI workspace command
  (`workspace-test-suite`); its junit (`target/nextest/ci/junit.xml`) feeds
  `cargo run -p xtask -- check-test-budget --junit <path>`. The same job runs `cargo test --locked --workspace --doc`
  (nextest does not run doctests) and `cargo test -p maos-kernel-core --lib`; neither is in the junit budget.
- Every test binary's Σ per-test time stays under its budget in `xtask/test-budgets.toml` (default 30 s; an
  `[[exception]]` names `binary`, `seconds`, `reason`, `owner`). `check-test-budget` is blocking at v1.0 since
  `18-0-green-main-ci-baseline` (operator ruling R3 2026-10-24: calibrated on five CI junits, total 800 s).
- No test spawns `cargo` (`Command::new("cargo")`, `env!("CARGO")`, `var("CARGO")`/`var_os("CARGO")`),
  `rustc` or `trybuild` unless the line before the call says `// expensive: <reason>`, the test is
  `#[ignore = "expensive: <reason>"]`, and its binary is listed in `[[expensive]]`
  (`cargo run -p xtask -- check-test-cost`). The `[[expensive]]` rows with `job = "expensive-tests"` run in that job,
  exactly that list, with `--run-ignored ignored-only`; every other row names the dedicated job that provisions its
  substrate and runs its ignored tests there (`--ignored`/`--include-ignored`, directly or through that job's gate).
- A cargo call that compiles nothing — `cargo tree`, `cargo metadata`, `cargo locate-project`, `cargo pkgid` — says
  `// cargo-query: <reason>` on the line before instead, and stays in the default run (no `#[ignore]`).
  `cargo public-api` builds rustdoc JSON and stays expensive.
- Fixture binaries are built once by `cargo run -p xtask -- build-test-fixtures` (manifest at
  `$MAOS_FIXTURE_DIR/manifest.json` — an absolute path — default `<target>/maos-fixtures/`); a test that needs one
  and finds it missing, or finds any of its recorded `.rs`/`.toml`/`.wit` inputs newer than the build, fails with that
  command in its message.

## Local profile (AC3)

With `CI` and `GITHUB_ACTIONS` unset, a substrate-dependent test prints one `SKIP: <prerequisite>` line and passes;
with either set, the same test fails closed exactly as before (`tests/harness/ci.rs`). The dev container does not
export `CI`; the agent harness's tool shell does, so agents run local suites as
`env -u CI -u GITHUB_ACTIONS cargo …`.

## Image matrices (AC4)

A matrix stays only where the image is the test's subject (kernel, seccomp, cgroup v2, OCI runtime, WASM runtime of
that image); every other job runs one image. Each kept matrix carries its one-line reason in `discipline.yml`:

| Job | Images | Kept because |
|---|---|---|
| `t3-escape-corpus` | 2 × x64 | T3 containment is the image's kernel + OCI runtime (user namespaces, seccomp, netns). |
| `t3-smoke-busybox` | 2 × x64 | the live T3 boundary (uid/gid, netns) is the image's runtime. |
| `epic-17-exit` | 2 × x64 | the Epic-17 exit lines are product claims per shipped T3 image. |
| `cgroup-ceiling-delegated` | 4 | cgroup v2 delegation is the image's systemd + kernel. |
| `cgroup-ceiling-undelegated` | 4 | the Delegate=no fallback is enforced by the image's systemd; the negative control is per image. |
| `wasm-host-tests` | 4 | T2 seccomp and the WASM runtime are per kernel/arch (R17-50, exit line 3). |

`release-dry-run` is a *target* matrix (the release artifact per target triple), not an image matrix.
