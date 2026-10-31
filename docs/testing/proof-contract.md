# Proof contract

Story `20-12-test-cost-budget-and-review-execution-contract` (2026-10-23), with the operator rulings for stories
`20-15-ci-build-once-nextest-archive` and `20-16-prepush-drift-gate` (2026-10-27). This page is the tracked
contract; the dev-story and code-review customizations (`_bmad/custom/bmad-dev-story.user.toml`,
`_bmad/custom/bmad-code-review.user.toml`) mirror it locally. `_bmad/` is gitignored, so those are per-checkout
copies, not the source of truth. The 20-15 and 20-16 sections state the ruled requirements, not evidence of landing
or passing runs.

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
- The **workspace suite runs in CI**, not as a separate review re-execution. The required pre-push command below is
  an exception: it includes a local workspace suite before a story-tree push and does not replace CI proof.
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
  Layer assignment is superseded by the 2026-10-26 ruling below: gpt-6.1-sol is now the dev model and never reviews
  code it wrote.
- **Agent models by role (operator ruling 2026-10-26, token cost).** Story creation runs on
  `anthropic/claude-opus-5-5` (`.omp/agents/story-creator.md`); dev, finisher and review-patch passes run on
  `openai-codex/gpt-6.1-sol` (the `task` agent; soft request budget 400); docs-only record work (ruling application,
  done flips, deferred-work closures, tracker, CI excerpts) runs on `zai/glm-5.3` (`.omp/agents/scribe.md`).
  Review is tiered by risk. A story is **difficult** when it carries a kernel FLAG-Winston grant, a security-critical
  AC (auth, secrets, signatures, cross-Spirit isolation), concurrency or a new protocol. Blind and Edge always run on
  opus-5.5; Acceptance runs on opus-5.5 for difficult stories and on `zai/glm-5.3` (`.omp/agents/review-light.md`)
  otherwise; Test Infrastructure runs on glm-5.3 on every story. An easy story may merge Blind and Edge into one opus
  layer. Every story keeps at least one opus layer; glm-5.3 is never a story's only reviewer.
  *Session-only substitution (operator, latest 2026-10-27):* for the 20-15/20-16 review-correction phase coding
  and high-risk reviews run on `anthropic/claude-opus-5-5` and light reviews/docs on Sonnet 5.5, superseding an
  earlier temporary Terra substitution. This does not rewrite the standing policy above.

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
  `$MAOS_FIXTURE_DIR/manifest.json` — an absolute path — default `<target>/maos-fixtures/`). A missing or stale
  fixture fails with that command in its message. For entries without a source tree ID, any recorded
  `.rs`/`.toml`/`.wit` input newer than the build makes the fixture stale; tree-ID provenance is specified below.

## Build once (20-15)

The ruled contract for the bucket consumers is one producer per architecture per discipline run: build on
`ubuntu-24.04` and `ubuntu-24.04-arm`, then consume the matching architecture's artifacts on both images of that
architecture. The same source tree, package set, features and profile as the original invocation are required;
workspace feature unification means a workspace archive must not replace a package-specific archive.

- Keep the original invocation families separate: `maos-bin` with `network,wasm-host`; `maos-bin` with `network`;
  `maos-kernel-core`; the `maos-wasm-host` T2 suite; `maos-escape-detector`; the combined `maos-host` /
  `maos-wasm-host` suites; and the standalone native-twin package. `workspace-test-suite` remains its own
  workspace build, not a package-specific consumer.
- Also produce, per architecture, a file artifact containing the fixture manifest and every listed artifact,
  including standalone probes outside `target/`, plus the release `maos` (wasm-host) and `maos-wasm-runner`
  binaries and debug `maos` and `maosctl`. Restore these at their identical absolute paths. The producer and the
  five selected consumer job families explicitly set `MAOS_FIXTURE_TRANSPORT=1`; this mode is also passed into
  systemd canary units. The producer requires a clean source checkout and seals each manifest entry with the source
  tree ID (`git rev-parse HEAD^{tree}`) and artifact identity. Transport consumers require and verify that provenance,
  including for manifest-owned profile fixtures `maosctl` and `worker-cli-fixture`: absent, corrupt or unsealed
  entries cannot downgrade to presence-only checks. Separately built, unregistered product binaries may still use
  the existing presence-only profile lookup.
  Ordinary own-build jobs and local callers, including CI jobs outside the selected consumers, retain the mtime
  staleness rule; `CI` or `GITHUB_ACTIONS` alone does not enable transport mode.
- Transport is run-scoped `actions/upload-artifact@v4` / `download-artifact@v4`, named with bundle, architecture
  and `github.sha`, with one-day retention and `if-no-files-found: error`; never use `actions/cache` for bundles.
  Consumers download their own architecture's artifacts from the same SHA. Extract nextest archives at the
  producer's `$GITHUB_WORKSPACE` path using `--extract-to` and `--workspace-remap`; hosted evidence must establish
  the path and 24.04-built binary compatibility on 26.04 for both architectures.
- Consumers `wasm-host-tests`, `cgroup-ceiling-delegated`, `cgroup-ceiling-undelegated`, and `epic-17-exit` run
  their archives with `cargo-nextest nextest run --archive-file`; `epic-18-exit` consumes debug binaries only.
  Pin one nextest version, `0.9.149`, for producers and consumers. Preserve test counts and marker specs, using
  `--color never` for parsed output and `--test-threads 1` / `--no-capture` where the original suite requires
  serial execution / captured markers.
- Consumers install no Rust toolchain and use no rust-cache; set `RUSTC`, `CARGO_BUILD_RUSTC` and `CARGO` to
  `/bin/false` at job level (all three denied). No `cargo build`, `cargo test`, or `cargo run` is permitted in a consumer: a missing
  artifact or attempted compile must fail, never silently rebuild. A failed producer skips its consumers and the
  fan-in fails closed. Keep consumer job names, image matrices, step names, count and marker pins,
  `aggregate.needs`, and the workspace junit feeding `check-test-budget`.
- Rootless canaries inside compiler-denied consumers use the same-run `network,wasm-host` archive through
  `MAOS_CGROUP_CANARY_ARCHIVE`; a missing or invalid override is red, never permission to compile. The workspace
  job and local callers without compiler denial retain their separate compiling canary mode.
- Consumer logs and externally directed scene outputs live under `RUNNER_TEMP`, outside the checkout. Existing
  test-owned Cargo scratch outputs may remain under ignored `target/tmp`; stage them under `RUNNER_TEMP` when
  uploading alongside those logs so artifact layout stays coherent. Do not weaken tree-ID provenance or ignore
  arbitrary untracked files to accommodate a consumer's own outputs.

## Pre-push (20-16)

Before **every push of a story tree**, require `cargo run -p xtask -- prepush` to exit 0. Record its exit and
wall-clock in the story's Dev Agent Record before the landing push. It mirrors CI's drift classes; the landing CI
run remains authoritative, and a local pass does not prove substrate-dependent marker values.

- The test leg unsets `CI` and `GITHUB_ACTIONS` and runs `cargo nextest run --workspace --locked --no-fail-fast`,
  `cargo test -p maos-kernel-core --lib` **unconditionally**, and `cargo test --workspace --doc`. The workspace
  resolution covers cross-crate scanners; nextest excludes kernel-core lib tests and does not run doctests.
  Substrate-dependent local skips are counted and printed, not treated as CI coverage.
- The aggregate self-executes the gates with their CI arguments: `stability-matrix --check`, `error-catalog-check`,
  `gen-abi-docs --check`, `check-serde-error-handling`, `check-workspace-count`, `check-manifest-schema-version`,
  `check-skill-schema`, `check-epic-close-coherence`, `check-coverage-matrix-completeness`, `check-test-cost`, `check-env-contract`,
  `check-unsafe`, `check-service-boundary`, `check-kernel-baseline`, `kloc-check`, `check-breaking-md`,
  `check-composition-root-completeness`, `check-pub-field-constructors`, `check-dependency-closure`,
  `coverage-matrix`, and `check-corpus`; it also runs `cargo fmt --all -- --check`.
  Story 20-3b also pins standalone Blocking aggregate invocations: `abi-diff --base abi-baseline/v1-pre-bump.txt`,
  `check-empty-kernel`, `check-loom`, `check-fr47`, `check-a2a-sender-completeness`, `check-judge-config`,
  `check-security-md`, `templates-regen --check`, `corpus-staleness`,
  `calibrate --corpus calibration-seed-v0.1 --n 100 --p 0.95`, `check-review-findings-resolved`,
  `check-dev-record-completeness`, `check-dev-model-tier`, `check-ship-gate-completeness`,
  `check-pentest-gate`, `check-third-party-trial`, `check-trial-attestation`, `check-red-team-gate`,
  `check-migration-merkle`, `nfr-onb-1-gate --check`, and `cassette-age-gate`.
  The migration leg needs the same deterministic million-row corpus that CI generates before invoking it.
  On a fresh local checkout, provision it once with
  `env -u CI -u GITHUB_ACTIONS cargo run --locked --release -p maos-loom-lite --example generate_migration_corpus -- --out "$PWD/tests/corpora/migration-corpus-1e6.sqlite"`
  from the workspace root, or generate it elsewhere and set `MAOS_MIGRATION_CORPUS` to that absolute path.
  This is the real pinned corpus, not a reduced substitute; absence or oracle drift remains RED.
  The pin reads aggregate dependencies and executable xtask invocations, not summary-table echo strings.
  Assignment-prefixed and `env` commands and backslash continuations are parsed;
  other executable xtask shell shapes fail closed. Gate arguments are compared
  after whitespace/`--json` normalization: `--check` and baseline/calibration
  values remain binding. Explicit auxiliary exceptions are the NFR-Test-3
  `--dry-run` soft-floor measurement and expensive-suite selector discovery;
  fixture producer `--only`/`--release` setup is covered by the full local build,
  not treated as an alternative invocation of a Blocking gate.
  Explicit no-standalone-form exclusions: invariant-lock's PR/reviewer/journal context, the CI release-binary
  mock scan, the Epic-19 scene/twin reconciliation, target-matrix/signing release dry-runs, and the junit
  budget gate (already conditionally mirrored over fresh local junit). None is downgraded to advisory in CI.
  Build-free docs checks include route-manifest parity, glossary, anchors and ko coverage
  (`KO_COVERAGE_MIN=100`). The CI pin reader compares test-bound count pins against `cargo nextest list`, marker
  specs against emission-site counts, and checks color settings on parsed cargo/nextest output; it covers recognised
  count/marker shapes only, so a pin in an unrecognised shape is not checked locally. Marker handling
  (20-16 D-02 operator addendum 2026-10-27, C): a statically determinate marker is compared exactly; a marker
  emitted only dynamically is a named indeterminate SKIP; a marker with no emission site is RED. Actual marker
  values, including dynamic counts, remain CI-only, and CI's runtime marker pins are not weakened.
  Deprecation policy is enforced by the existing `stability-matrix --check` leg; the obsolete empty-channel
  check and its CR-2 exception retired in Story 20-3b.
  The 20-3b coverage spike selected NO-GO, so the hand-kept matrix and its completeness leg remain live.
  The error-catalog copied-binary fallback selects the nearest cwd ancestor
  containing a workspace manifest and `xtask/`. This is a **cwd trust
  assumption**, not authentication of the repository root: invoke it inside
  the intended trusted checkout. A hostile cwd tree can supply those markers
  and select a counterfeit workspace; the fallback does not authenticate a
  checkout, publisher or ownership boundary.
  `clippy-advisory` is deliberately outside this Blocking mirror and `aggregate.needs`: its non-denying
  `cargo clippy --workspace --all-targets --locked --message-format=json` output is checked by
  `clippy-ratchet --input <jsonl>` against `xtask/clippy-baseline.toml` (T0: 1411 warning occurrences,
  884 source-distinct). Both count ceilings ratchet; repeated library/lib-test emissions count separately.
  Source-distinct keys are package, level, code, message and primary source start/end ranges; target and
  secondary spans are excluded. Lower the pins when debt is intentionally reduced; do not increase them
  to hide newly introduced warnings. A failed/malformed/truncated clippy run cannot count as zero debt.
- Determine changes from `git merge-base origin/main HEAD` through HEAD, plus the working tree and untracked files;
  never fetch, and print the base SHA and its age. Run the release build, strip and 33,554,432-byte size check when
  `Cargo.toml`, `Cargo.lock`, `crates/maos-bin/**`, or a maos-bin dependency changes; run a fresh Docusaurus build
  and `gate-routes.js` when `docs-site/**`, `docs/errors/**`, or `xtask/error-catalog.toml` changes; run
  `cargo deny check` when `Cargo.lock` or `deny.toml` changes. Each untriggered conditional leg prints a named skip
  and reason. Required tools missing on a triggered leg are red, including missing `cargo-deny` for a lockfile
  change; a check that cannot run is never reported as passing. Local nextest-junit `check-test-budget` is advisory,
  because local timings are not CI timings.
- Exit 0 reports checks run, skips with reasons and wall-clock; exit 1 reports each failing check as
  `RED <check> — <artifact> — <pin or expected/actual>`. `--json` emits the same records. Do not silently leave
  tracked files modified by a leg: restore or report them.

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
