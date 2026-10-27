# Local implementation verification — 2026-10-27

This is local evidence, not the hosted full-bundle acceptance proof and not a story-done record.

## Formatted production delta

After `cargo fmt --all`, tokei measured `build_test_fixtures.rs` at 546 code lines versus baseline 508 (+38), and `check_test_cost.rs` at 573 versus 574 (-1): the then-current 20-15 xtask delta was **+37**, within the operator's +38 STOP. This predates the explicit mode update, which targets **+38** but has not been measured on its final formatted tree. This is not a ceiling re-pin. Measurement output: session artifact `4980`.

## Executed checks

- `cargo nextest run --locked -p xtask --test prepush_20_16 --test fixture_provenance_20_15 --test check_test_cost_proven_red --test ci_shape_18_0 --test epic17_exit_workflow_17_7 --test cgroup_ceiling_workflow_17_2`: **65 passed, 0 skipped**. This combined invocation also exercised 20-16's current deterministic vectors; it does not prove its aggregate command or its line grant. Session log `4983`.
- `cargo run -q --locked -p xtask -- check-test-cost`: **PASS**, 562 test files, 8 marked spawn sites (4 cargo-query), 7 expensive binaries. The corrected checker follows single-line static initializer references without exempting ordinary tests. Session log `4983`.
- Parsed `discipline.yml` with Bun's YAML parser; all **386** Bash/sh run scripts passed `bash -n` after replacing GitHub expressions with an inert token. There are 177 job keys. This is YAML/shell syntax evidence, **not actionlint or hosted runner validation**.
- Actual producer command, with `CI`/`GITHUB_ACTIONS` unset and `CARGO_TARGET_DIR=/mnt/build/cargo`: `cargo run -q --locked -p xtask -- build-test-fixtures --only maosctl --json`, exit **0**, 9.21 s wall. It reported `dirty checkout; local mtime entries only, no clean provenance`, built `/mnt/build/cargo/debug/maosctl`, and wrote `/mnt/build/cargo/maos-fixtures/manifest.json`. Session log `4986`. This proves the dirty local path, not clean CI provenance.

## Actual consumer harness smoke

A throwaway, std-only driver included the repository's real `tests/harness/fixtures.rs` and called `fixture("smoke")`. The isolated input was a tiny clean Git fixture (one `.rs` input plus `.gitignore`, **not a workspace copy**); the payload and manifest lived under its ignored `target/`. No Cargo build or second Cargo target was used.

- Control with matching tree/blob identities and `built_at: 1`: exit **0**, `FIXTURE-ACCEPTED`.
- Change only the manifest tree to forty zeroes: exit **101**. The actual harness reported `manifest tree 0000000000000000000000000000000000000000 differs from checkout tree 7e3f4cc53ae8c3c4be95e68f9db8130ec7a4cd0a` and named `cargo run -p xtask -- build-test-fixtures`.
- Restore the saved manifest bytes: exact byte equality checked; exit **0**, `FIXTURE-ACCEPTED`.

This exercises the real consumer's stale-tree branch but **does not replace the required in-place plant on the story tree**, which remains pending. The current story checkout is dirty, so a schema-3 entry there would correctly refuse cleanliness before reaching the tree comparison.

## Integration correction and focused runtime proof

`crates/maos-kernel-core/tests/intent_lineage_corpus_v0.rs` now writes its coverage run-record under `CARGO_TARGET_TMPDIR`, rather than appending to a tracked planning artifact while parallel tests verify clean fixture provenance. Kernel production source is untouched; corpus scenario assertions are unchanged.

Executed `cargo nextest run --locked -p maos-kernel-core --test intent_lineage_corpus_v0 --test cgroup_ceiling_smoke --no-capture`: **4 passed, 4 skipped**, 9.25 s command wall (session log `4997`). The four skipped cases are the existing ignored resource tests, not claimed substrate coverage. The corpus printed **120 / 120 scenarios passed**; `/mnt/build/cargo/tmp/intent-lineage-coverage-report.md` recorded 120 scenarios, 120 passed, 0 failures. The tracked report SHA-256 was identical before and after: `32c207588abddb398ec3f343047914aa9eab102599c0ddf953eafb89d476f5f9`.

## Compiler-denied kernel archive consumer

Built the actual producer's kernel test-target family with nextest **0.9.149**:

```text
cargo nextest archive --locked -p maos-kernel-core --test cgroup_ceiling_smoke --test sandbox_enforcement_linux --test bridge_t2_route_17_6 --archive-file /tmp/maos-20-15-kernel-smoke.tar.zst
Archiving 4 binaries (including 1 non-test binary), 2 linked paths, and 1 standard library
Archived 42 files ... in 0.10s
```

Producer command exit 0, 1.74 s wall (session log `5006`). Created the consumer extraction directory first, then ran `cargo-nextest nextest run --archive-file ... --extract-to /tmp/maos-20-15-kernel-consumer --workspace-remap /home/lunarpulse/dev_ws --color never --no-capture -E 'binary_id(maos-kernel-core::cgroup_ceiling_smoke)'` with **RUSTC=/bin/false, CARGO_BUILD_RUSTC=/bin/false, CARGO=/bin/false**, CI/GITHUB_ACTIONS unset.

Observed `Extracted 42 files ... in 0.17s`; **3 tests run: 3 passed, 4 skipped**, test-phase 0.083 s, with no compile invocation in the consumer output. These are the three unignored fixture-validation cases; the four ignored resource cases were not executed. The initial extraction attempt to a nonexistent directory failed canonicalization (exit 96); creating the directory resolved that prerequisite.

This local extraction is deliberately outside the Cargo build cache and is not another Cargo build target. It proves archive execution with compiler-denial guards for these validators, **not** hosted same-absolute-path compatibility, delegated resource coverage, the missing-archive hosted plant, or the complete production bundle.

## Audited transport repairs and structural validation

- After the initial Opus transport audit, the hidden rootless canary gained explicit archive mode, per-process libtest summaries stopped being counted alongside nextest aggregates, and consumer logs/externally directed scene outputs moved under `RUNNER_TEMP`. Existing ignored Cargo scratch is staged there for mixed uploads rather than weakened through ignore rules.
- The first repaired snapshot passed **67** focused Rust tests and **29** Python helper tests. Later mandatory-provenance and fixed-case-name repairs require their own subsequent verification; these counts are not a final-tree approval.
- Re-ran the exact previously failing no-capture parser smoke: three per-test libtest result lines plus the nextest aggregate now validate as **one** suite with **3** passes.
- Exercised the real canary selector with all compiler guards `/bin/false`: an absent archive override refused with `compiler-denied rootless canary requires MAOS_CGROUP_CANARY_ARCHIVE`. A real `cargo-nextest` invocation selected through that helper against a corrupt archive returned **96**, `Unknown frame descriptor`, naming the archive; no `Compiling` output or fallback. The temporary corrupt archive was removed. This is refusal-path proof, **not** positive rootless/T3 execution.
- Independent Blind/Edge review caught `runner.temp` incorrectly placed in epic-17-exit's job env. The variable now belongs to each rootless-canary step env, where the runner context is legal.
- Ran **actionlint 1.7.12** structural/expression validation (shellcheck/pyflakes disabled). The unconfigured run reported only its outdated catalog's unknown `ubuntu-26.04` / `ubuntu-26.04-arm` labels. A temporary validator-only configuration added those two labels, already exercised by the hosted T1 spike; the structural check then exited **0** with no other diagnostics. No actionlint configuration was added to the repository, no workflow check was suppressed, and the labels remain hosted—not self-hosted—runners.
- Validator binary came from the [upstream v1.7.12 release](https://github.com/rhysd/actionlint/releases/tag/v1.7.12); the downloaded Linux amd64 archive matched the release API's SHA-256 `8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8`.

## Subsequent review repairs and isolation

- Latest focused result before the explicit transport-mode update: **74 Rust tests and 30 Python helper tests passed**. These are not final-snapshot results; the new mode bindings and subsequent changes need retest and independent review.
- The producer and five selected consumer families explicitly opt in with `MAOS_FIXTURE_TRANSPORT=1`; systemd receives it. In this mode a dirty source cannot be sealed, and consumer provenance is mandatory, including missing, corrupt or unsealed manifest entries for profile-owned `maosctl` and `worker-cli-fixture`. Unregistered product binaries retain their presence-only allowance. Own-build jobs and local runs, including ordinary CI, retain mtime checks without a blanket `CI`/`GITHUB_ACTIONS` requirement. The explicit-mode edits have not yet received final verification.
- The no-capture parser counts nextest aggregate summaries rather than also counting per-test libtest summaries. Required case names now bind the 23-case session gate in both delegated and undelegated jobs, the two-case Worker gate and the four-case cgroup smoke gate; counts and existing markers are not repinned.
- `xtask/tests/example_spirit_regen_integration.rs` no longer mutates the tracked example: its drift case uses a temporary fixture and the canonical `templates-regen` command. Parent's actual CLI smoke observed stale check exit **1**, regeneration exit **0** producing `pub struct ExampleSpirit;\n`, then check exit **0**; tracked example bytes were unchanged. The lineage corpus separately writes its report to Cargo scratch as recorded above.
- Two invalid tests, the manual-overwrite echo and README-wording checks, were deleted rather than repinned. This is not a change to the required workflow counts or marker pins.
- `BlindEdge2015` identified job-level `runner.temp` and the tracked-example race; the step-only environment correction has the actionlint validation described above. `AcceptInfra2015` additionally identified 23-case substitution and absent profile-entry downgrade; repairs need final independent re-review. No final approval or hosted full-bundle result is claimed.

## Explicit-mode runtime smoke

After the explicit-mode code handoff, the parent compiled a std-only driver including the real harness, without Cargo, against an isolated tiny Git fixture (not a workspace copy). With `CI=true`, `GITHUB_ACTIONS=true` and `MAOS_FIXTURE_TRANSPORT=1`, the sealed fixture, manifest-owned profile `maosctl` and intentionally unrecorded product `maos` all exited **0**. Removing both identities made explicit transport exit **101**, `fixture transport requires clean tree and artifact provenance`; with CI still true but only the transport flag removed, that same valid unsealed mtime fixture exited **0**. Deleting the owned `maosctl` entry in transport mode exited **101**; restoring the sealed manifest exited **0**. The tiny fixture and driver were removed.

This actual mode smoke supersedes any earlier blanket-CI-policy smoke; it is not the formal in-place story-tree plant, a hosted bundle or a positive rootless canary run. Focused Rust/Python runs after the explicit-mode change remain pending.

## Still required

Final formatted grants/pins; required in-place provenance and no-compile shape plants; independent reviews; the hosted missing-archive proven-red run; two landing-shaped runs with per-class compile/test and critical-path measurements; final story/tracker records. The earlier T1 spike proves only its selected archive family.

## Review-correction status (2026-10-27)

- Model roles for this phase are a SESSION-ONLY substitution (operator latest: Opus 5.5 coding/high-risk reviews, Sonnet 5.5 light reviews/docs; an earlier temporary Terra substitution was superseded); the standing tracked policy is unchanged.
- Archive-family assertions and fixture/runtime regressions are under correction by separate units and are **not yet verified**. The +37 figure above is the pre-mode measurement; a post-edit formatted remeasurement/re-pin and independent review remain outstanding (+38 STOP unchanged).
- Shared 20-16 observation: a full actual prepush run (bg_19, 20-16 evidence) passed with no red records before these patches. It is not a final-tree, warm, plant or hosted result for this story.
