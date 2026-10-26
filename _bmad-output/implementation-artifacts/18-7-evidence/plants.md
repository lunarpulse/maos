# Story 18-7 proven-red controls

Four mutations, one per AC. Each was applied in place in the real tree, then restored from a byte-identical copy with `cp`, verified with `cmp` (exit0), and `touch`ed before the incremental rebuild. No workspace copy/worktree/second target directory. No hosted run claimed.

All cargo commands used `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`. Real process binaries were rebuilt with `cargo build --locked -p maos-bin --bin maos -p maos-cli --bin maosctl`. For root-only source changes the unchanged CLI lib was touched too, so the journey harness's cross-package source-freshness guard required a real CLI rebuild rather than an artificially fresh binary timestamp. Process tests used `TMPDIR=/dev/shm` after measuring the host disk's `wait_for_commit` stall; CI retains its normal scratch filesystem. No test timeout or budget was widened.

The focused command is `cargo nextest run --locked -p maos-journey-test --test journey_skill_authoring_18_7 -E 'test(=<TEST>)'`.

| AC | Plant and checked-in diff | TEST | Observed red (exit100) | Restored green | Wall-clock touch/red/restore/green |
|---|---|---|---|---|---:|
| AC1 | Drop persisted revision payload; `ci/plant.diff` | `live_author_revision_restart_and_rejection_preserve_the_payload` | durable proposal payload: diff missing; payload:null; `ac1-red.log` | one test passed; `ac1-restored-green.log` | 91.328 s |
| AC2 | Bypass root issuance and verification; `ac2-mint.diff` | `undeclared_live_researcher_is_refused_without_queue_mutation` | undeclared Researcher cannot enqueue; `ac2-red.log` | one test passed; `ac2-restored-green.log` | 94.204 s |
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
