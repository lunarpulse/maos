# Story 18-7 proven-red controls

The original four AC mutations remain recorded below; review refreshed AC2 against the reviewed source and added six named mutation bundles because the review assignment requires a plant for every new control. Every bundle was applied in place, then restored from a byte-identical `cp` backup, verified with `cmp` (exit0), and `touch`ed before rebuilding. No workspace copy/worktree/second target directory. No hosted run claimed.

All cargo commands used `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`. Real process binaries were rebuilt with `cargo build --locked -p maos-bin --bin maos -p maos-cli --bin maosctl`. For root-only source changes the unchanged CLI lib was touched too, so the journey harness's cross-package source-freshness guard required a real CLI rebuild rather than an artificially fresh binary timestamp. Process tests used `TMPDIR=/dev/shm` after measuring the host disk's `wait_for_commit` stall; CI retains its normal scratch filesystem. No test timeout or budget was widened.

The focused command is `cargo nextest run --locked -p maos-journey-test --test journey_skill_authoring_18_7 -E 'test(=<TEST>)'`.

| AC | Plant and checked-in diff | TEST | Observed red (exit100) | Restored green | Wall-clock touch/red/restore/green |
|---|---|---|---|---|---:|
| AC1 | Drop persisted revision payload; `ci/plant.diff` | `live_author_revision_restart_and_rejection_preserve_the_payload` | daemon 1's first persisted proposal read at test `:207` fails `durable proposal payload: diff missing`; payload:null, before restart; `ac1-red.log` | one test passed; `ac1-restored-green.log` | 91.328 s |
| AC2 | Bypass root issuance/verification and the new pre-model declaration gate; refreshed `ac2-mint.diff` | `undeclared_live_researcher_is_refused_without_queue_mutation` | undeclared Researcher cannot enqueue; refreshed `ac2-red.log` (exit100) | one test passed; refreshed `ac2-restored-green.log` | 23.387 s (Σ recorded build/test commands) |
| AC3 | Remove revision #entry_id target suffix; `ac3-identity.diff` | `live_author_revision_restart_and_rejection_preserve_the_payload` | revision rejection cannot flip the base; `ac3-red.log` | one test passed; `ac3-restored-green.log` | 92.735 s |
| AC4 | Remove discovered/on-path rejection filter; `ac4-boot.diff` | `live_author_revision_restart_and_rejection_preserve_the_payload` | daemon3 exits: rejected skill(s) block daemon startup; `ac4-red.log` | one test passed; `ac4-restored-green.log` | 116.851 s |

## Exact restoration commands

```sh
cp /tmp/18-7-ac1.yBZMxm crates/maos-skill/src/store.rs && cmp /tmp/18-7-ac1.yBZMxm crates/maos-skill/src/store.rs && touch crates/maos-skill/src/store.rs
cp /tmp/18-7-ac2.fuXEzy crates/maos-bin/src/skill_authoring.rs && cmp /tmp/18-7-ac2.fuXEzy crates/maos-bin/src/skill_authoring.rs && touch crates/maos-bin/src/skill_authoring.rs crates/maos-cli/src/lib.rs
cp /tmp/18-7-ac3.onoW82 crates/maos-skill/src/store.rs && cmp /tmp/18-7-ac3.onoW82 crates/maos-skill/src/store.rs && touch crates/maos-skill/src/store.rs
cp /tmp/18-7-ac4.Re1jtT crates/maos-bin/src/main.rs && cmp /tmp/18-7-ac4.Re1jtT crates/maos-bin/src/main.rs && touch crates/maos-bin/src/main.rs crates/maos-cli/src/lib.rs
```

AC4's hosted-verification payload control reuses the single AC1 plant, not an additional mutation. `ci/ci-18-7-verify.yml` follows18-6's branch-only model. The local simulation exercised its exact durable-payload oracle, SHA256 verification and patch applicability; see `ci/local-simulation.log`. Hosted run on ubuntu-24.04 is the integrator's assignment.

## Review control bundles — 2026-10-26

All commands, elapsed times and exit statuses are in the named logs. Red runs use `--no-fail-fast` where controls are bundled: each listed control has its own failing assertion, rather than inferring coverage from an earlier failure. The additional plants were explicitly required by the review assignment, superseding the default one-per-AC cap for these new controls.

| Bundle / checked-in diff | Independently falsified controls | Red result | Copy-back result | Σ recorded cargo wall-clock |
|---|---|---|---|---:|
| `review-library-controls.diff` | all-state and discovered version ceilings; rejected-base isolation; 128-byte ids; finite evidence; pre-journal destination/ownership refusal; actual FKCS receipt; declaration-before-completion; nonfatal authoring; telemetry-informed revision prompt | 44 tests, 18 failed; `review-library-controls-red.log`, exit100 | 44 passed; `review-library-controls-restored-green.log` | 15.679 s |
| `review-home-route-binder.diff` | canonical home queue path; `GET /v1/skills/decide` remains a show; binder uses a fresh telemetry window | 3 tests, 3 failed; `review-home-route-binder-red.log`, exit100 | 3 passed; `review-home-route-binder-restored-green.log` | 13.340 s |
| `review-migration-journal.diff` | failed migration journal cannot consume or write the queue | 1 failed; `review-migration-journal-red.log`, exit100 | 1 passed; `review-migration-journal-restored-green.log` | 7.283 s |
| `review-migration-once.diff` | successful migration consumes the shared legacy file exactly once | 1 failed; `review-migration-once-red.log`, exit100 | 1 passed; `review-migration-once-restored-green.log` | 5.317 s |
| `review-boot-controls.diff` | no-home and corrupt-queue boot availability; non-`run` enforcement; a rejected authored draft is boot-inert | 4 tests, 4 failed; `review-boot-controls-red.log`, exit100 | 4 passed; `review-boot-controls-restored-green.log` | 33.369 s |
| `review-airgap.diff` | ungated door code cannot depend on the network-gated authoring module | compile red (`cannot find skill_authoring in crate`); `review-airgap-red.log`, exit101 | supported air-gap build compiles; 7 passed; `review-airgap-restored-green.log` | 6.244 s |

Exact `cp`, `cmp`, and `touch` commands and exit0 results are in `ac2-mint-restore.log` and each `review-*-restore.log`; throwaway backups were removed after comparison. AC2's planted/restored process builds are `ac2-review-{planted,restored}-build.log`; boot bundle builds are `review-boot-controls-{planted,restored}-build.log`. Every root-only process rebuild also touches `crates/maos-cli/src/lib.rs`, preserving the source-freshness guard rather than touching an executable.

The final applicability check includes every original and review diff and the CI payload plant; all `git apply --check` calls exit0 (`review-plant-applicability.log`). The CI plant remains byte-identical because it still applies cleanly; its hosted ubuntu-24.04 execution/run id remains integrator-owned. R5 changes only sequence7 of `skill-authoring-18-7.json` (real revision prompt SHA256 `201199bdc738ae2a2e73245cfc6d0f9420bdcdd5bdde4d80bf62423a0b2e185e`, length915); author-only fixtures and per-entry pins remain unchanged.

