# 19-5 proven-red plants

## Execution and scope — 2026-10-29

Executed sequentially in `/home/lunarpulse/wt-w3` (branch `w3-dev`, candidate `18082789` plus the existing round-4/5 working-tree fixes). This record implements the five controls required by AC1.5, AC2.4, AC3.5, AC4.1 and AC4.2; AC4.2 additionally has the two **masking-removed** supplements requested by Main. Seven diff files, eight red test invocations, six restored-green test invocations, and the final kernel-pin suite are recorded below.

**All plants are LOCAL.** The daemon witnesses run Butler in-process; the admitted-SCB witness uses a hermetic real admission in an archive-isolated libtest child; the continuity witness uses an in-process kernel harness. None requires T3, a container runtime, hosted provisioning, or a hosted workflow. No 19-5 entry is needed in `ci/ci-w3-verify.yml`. Nothing was skipped as substrate-dependent.

These diffs were cut from the actual in-place planted sources against pristine copies of the CURRENT working tree, not against HEAD (which would accidentally include existing fixes). Each source was restored immediately after its plant, including in a `finally` restoration path. The bundle omission was held in place for its two complementary one-test witnesses, then restored. The source tree is **not planted**.

No file under `crates/maos-kernel-core/src` needed modification: AC2 removes the shared door/hop ABI helper's operative kernel-helper invocation, disabling both callers at once. The kernel test source is not kernel hash-pinned, but it too was copied back byte-identically and touched. The final `kernel_pin_content_hash_16_0` suite passed, including `live_tree_matches_the_committed_pin`.

Wall seconds below use a monotonic clock around the full cargo subprocess, including incremental compilation and execution. Logs and pristine copies are retained under `/tmp/maos-w3-19-5-plants/`; the durable oracle excerpts, exact commands, hashes, return codes and restore proof are recorded here. No commit, push, formatter, broad workspace test or unrelated gate was run.

## Plant inventory

All paths in the site columns are relative to `/home/lunarpulse/wt-w3`. Panic line numbers are those observed in the **planted** file; source mutation can shift a test's own assertions.

| AC | Diff | Operative change | Test / assertion site | Observed red / wall seconds | Classification / restore |
|---|---|---|---|---|---|
| AC1.5 | `ac1-bare-load-plant.diff` | In `crates/maos-bin/src/operator_door.rs::admit_and_start`, replace `private_ops.load_spirit` (the admission triple) with `prepare_spirit` + bare `scheduler.load_obj`; retain construction, identity check, explicit start and unwind. No `load_admit_start` / vetter grant occurs. | `ac1_19_5_cold_swap_admits_the_successor_at_a_new_pid` → `crates/maos-bin/tests/one_daemon_one_door_16_1.rs:990:13` | `the successor must pass admit_spirit — no \`admission granted\` vetter-key row for 0.3.1 after its load`; exit **101**, **8.06 s** | LOCAL; copy-back + `cmp` + `touch`, all exit 0 |
| AC2.4 | `ac2-abi-precheck-removed-plant.diff` | Remove `security::check_abi_triple` and its error mapping from shared `operator_door::check_upgrade_abi`. Both the door precheck and `migration_plan::validate_upgrade_hop` call this helper, so neither masks the removal. Keep target parsing and the raw schema-window guard. | `ac2_19_5_an_abi_incompatible_target_is_refused_before_any_policy_runs` → `one_daemon_one_door_16_1.rs:1209:9` | `assertion \`left == right\` failed: [None] must be refused:`; **left 0 / right 1**; exit **101**, **13.43 s** | LOCAL; copy-back + `cmp` + `touch`, all exit 0 |
| AC3.5 | `ac3-fr11-gate-removed-plant.diff` | Remove only the `refuse_without_migrator` call/refusal block in `admission::load_admit_start`, before pid allocation. | `ac3_19_5_a_cross_major_successor_without_a_migrator_is_refused_at_admission` → `one_daemon_one_door_16_1.rs:1349:5` | `assertion \`left == right\` failed: the door must refuse the load:`; **left 0 / right 1**; exit **101**, **10.77 s** | LOCAL; copy-back + `cmp` + `touch`, all exit 0 |
| AC4.1 / admitted-SCB witness | `ac4-bundle-drops-hot-swap-plant.diff` | Delete `hot_swap: self.hot_swap.clone()` from `GatedManifest::bundle()`; `..Default::default()` supplies `None`. Parsing and all other carried sections remain unchanged. | `ac4_19_5_the_admitted_bundle_carries_the_swap_and_crash_sections` → outer `one_daemon_one_door_16_1.rs:1968:5`; real child assertion `:2004:5` | `admitted SCB assertions:` wrapping `assertion \`left == right\` failed`, **left None / right Some(HotSwapManifestSection { state_schema_uri: "maos://butler/state", state_schema_version: 65537 })**; exit **101**, **10.04 s** | LOCAL; restored after both witnesses, copy-back + `cmp` + `touch` all exit 0 |
| AC4.1 / daemon witness | same bundle diff | Same omission, no other change. | `ac4_19_5_the_hot_swap_reads_the_schema_admission_carried` → `one_daemon_one_door_16_1.rs:2194:5` | `a same-major hot swap must complete — EMigratorMissing here means the predecessor's [hot_swap] was dropped at admission`; stderr `hot-swap coordinator error: migrator missing: class=butler v0.3.0 → class=butler v0.3.1 (HTTP 500)`; **left 1 / right 0**; exit **101**, **0.86 s** | LOCAL; restored after both witnesses, copy-back + `cmp` + `touch` all exit 0 |
| AC4.2 / straight transition | `ac4-continuity-unload-load-plant.diff` | Replace the upgrade call/report assertion with real scheduler unload + load of `WITNESS_SUCCESSOR` + start at the new pid. Leave every subsequent original continuity assertion intact. | `hot_swap_keeps_tokens_and_working_memory_at_the_same_pid` → `crates/maos-kernel-core/tests/hot_swap_same_major_lifecycle.rs:272:5` (unplanted `:266`) | `assertion \`left == right\` failed: a hot swap keeps the pid`; **left Some(2) / right Some(1)**; exit **101**, **23.69 s** | LOCAL; copy-back + `cmp` + `touch`, all exit 0 |
| AC4.2 / **masking-removed, token** | `ac4-continuity-masking-removed-token-plant.diff` | Same unload/load; remove only the earlier pid-equality assertion, read the version from the actual successor SCB, and key the subsequent working-memory read by the actual successor pid. Preserve the token verification oracle. | same exact kernel witness → `hot_swap_same_major_lifecycle.rs:288:10` (unplanted token oracle `:287`) | `a token minted before the swap verifies after it: Revoked`; exit **101**, **3.53 s** | LOCAL; copy-back + `cmp` + `touch`, all exit 0 |
| AC4.2 / **masking-removed, memory** | `ac4-continuity-masking-removed-memory-plant.diff` | Same masking-removed transition; move the unchanged token-verification block after the working-memory assertion, so its already-proven `Revoked` result cannot mask the scalar oracle. | same exact kernel witness → `hot_swap_same_major_lifecycle.rs:290:5` (unplanted memory assertion `:293`) | `assertion \`left == right\` failed: a working-memory slot set before the swap reads back after it`; **left None / right Some(0.75)**; exit **101**, **4.16 s** | LOCAL; copy-back + `cmp` + `touch`, all exit 0 |

### Why these are operative-control reds

- **AC1:** The successor really loads and starts at pid 2. The failure's ordered trail is `["lifecycle.unload @1", "planned_unload @1", "planned_unload @1", "cap.revoke @1", "lifecycle.admit @2", "lifecycle.load @2", "lifecycle.start @2", "spirit.upgrade @1", "operator.upgrade.op-…:completed @1"]`. A scheduler `lifecycle.admit` label is not the security manager's vetter-key grant: the latter is absent, and that exact ordered-row oracle goes red. This plant still constructs the real class object, so a placeholder/constructor failure does not mask the admission defect.
- **AC2:** The kernel is compatible with the predecessor; only the successor manifest requires `min_substrate_version = "99.0.0"`. Removing just the initial door call would leave the per-hop check operative. Removing the kernel-helper invocation in their **shared** helper disables both without editing kernel source or bypassing parsing/identity/vetting. The full named test begins with default hot-swap (`policy == None`); it now exits 0, and its expected-refusal assertion fails before the later migrator/cold lanes. Those later lanes were not reached in this red run. No claim is made that this diff disables the distinct raw manifest-schema-window guard: this plant's premise is the incompatible **substrate** target.
- **AC3:** The real daemon first writes the predecessor archive, then unloads it. Removing the gate admits the undeclared cross-major successor; the refusal-exit assertion fails immediately. The later no-`lifecycle.load`, absence, and standalone-`maos run` assertions are intentionally unchanged but are **not reached** after that first failure. The restored full named test passes all those assertions.
- **AC4.1:** The first witness reads the admitted SCB, not merely a parsed proposal: its child observes `None` rather than the declared schema. The daemon witness independently observes the resulting cross-major classification and migrator-missing failure. The wrapper's outer failure header and panic are recorded in addition to the real child's assertion; this is not an ignored child silently counted as a pass.
- **AC4.2:** The straight plant proves the earliest continuity failure without weakening assertions. The token and working-memory assertions would otherwise be masked by that failure and the old-pid SCB lookup. The supplements remove the pid-equality mask and inspect the actual successor for the unchanged version assertion; they do not synthesize a successful token or scalar value. Token-first observes real revocation; memory-first observes a missing slot **at the successor pid**. Memory-first reorders, rather than deletes, the unchanged token oracle. All supplements are explicit diffs against the same pristine current source, not stacked patches.

## Exact one-test commands

Every command below was run from `/home/lunarpulse/wt-w3`. The identical command was also run on restored sources for each distinct witness (green results below). AC4.2's three diffs all use the same full exact test; none uses a substring filter or edits down the test to a partial body.

### AC1.5

```sh
env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo-maos-w3 nice -n 10 cargo test --locked --color never -p maos-bin --test one_daemon_one_door_16_1 -- --exact ac1_19_5_cold_swap_admits_the_successor_at_a_new_pid --test-threads=1
```

### AC2.4

```sh
env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo-maos-w3 nice -n 10 cargo test --locked --color never -p maos-bin --test one_daemon_one_door_16_1 -- --exact ac2_19_5_an_abi_incompatible_target_is_refused_before_any_policy_runs --test-threads=1
```

### AC3.5

```sh
env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo-maos-w3 nice -n 10 cargo test --locked --color never -p maos-bin --test one_daemon_one_door_16_1 -- --exact ac3_19_5_a_cross_major_successor_without_a_migrator_is_refused_at_admission --test-threads=1
```

### AC4.1 — admitted bundle and same-major daemon swap

```sh
env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo-maos-w3 nice -n 10 cargo test --locked --color never -p maos-bin --test one_daemon_one_door_16_1 -- --exact ac4_19_5_the_admitted_bundle_carries_the_swap_and_crash_sections --test-threads=1
env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo-maos-w3 nice -n 10 cargo test --locked --color never -p maos-bin --test one_daemon_one_door_16_1 -- --exact ac4_19_5_the_hot_swap_reads_the_schema_admission_carried --test-threads=1
```

### AC4.2 — straight, masking-removed token, masking-removed memory

Run once for each separately applied diff:

```sh
env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo-maos-w3 nice -n 10 cargo test --locked --color never -p maos-kernel-core --test hot_swap_same_major_lifecycle -- --exact hot_swap_keeps_tokens_and_working_memory_at_the_same_pid --test-threads=1
```

## Red verdicts and local logs

Every red run exited **101**, selected `running 1 test`, carried the exact named `---- <test> stdout ----` failure header, and ended with `test result: FAILED. 0 passed; 1 failed;`. The bundle wrapper additionally prints its child run's one-failure summary and panic; the outer named header and `:1968:5` panic are present. These are assertion/oracle failures after successful compilation, not compilation failures, zero-selection successes, skips, or unrelated-test failures.

| Invocation | Local log | Exit | Wall seconds |
|---|---|---:|---:|
| AC1 | `/tmp/maos-w3-19-5-plants/ac1-red.log` | 101 | 8.06 |
| AC2 | `/tmp/maos-w3-19-5-plants/ac2-red.log` | 101 | 13.43 |
| AC3 | `/tmp/maos-w3-19-5-plants/ac3-red.log` | 101 | 10.77 |
| AC4 bundle | `/tmp/maos-w3-19-5-plants/ac4-bundle-red.log` | 101 | 10.04 |
| AC4 daemon | `/tmp/maos-w3-19-5-plants/ac4-daemon-red.log` | 101 | 0.86 |
| AC4 straight continuity | `/tmp/maos-w3-19-5-plants/ac4-continuity-red.log` | 101 | 23.69 |
| AC4 masking-removed token | `/tmp/maos-w3-19-5-plants/ac4-token-red.log` | 101 | 3.53 |
| AC4 masking-removed memory | `/tmp/maos-w3-19-5-plants/ac4-memory-red.log` | 101 | 4.16 |

## Restore proof

Before any mutation, the following pristine copies were made (the `mkdir -p` and all three `cp` commands exited 0):

```sh
mkdir -p /tmp/maos-w3-19-5-plants
cp /home/lunarpulse/wt-w3/crates/maos-bin/src/operator_door.rs /tmp/maos-w3-19-5-plants/operator_door.rs
cp /home/lunarpulse/wt-w3/crates/maos-bin/src/admission.rs /tmp/maos-w3-19-5-plants/admission.rs
cp /home/lunarpulse/wt-w3/crates/maos-kernel-core/tests/hot_swap_same_major_lifecycle.rs /tmp/maos-w3-19-5-plants/hot_swap_same_major_lifecycle.rs
```

After each of AC1 and AC2, these exact commands ran, each exiting **0**:

```sh
cp /tmp/maos-w3-19-5-plants/operator_door.rs /home/lunarpulse/wt-w3/crates/maos-bin/src/operator_door.rs
cmp /tmp/maos-w3-19-5-plants/operator_door.rs /home/lunarpulse/wt-w3/crates/maos-bin/src/operator_door.rs
touch /home/lunarpulse/wt-w3/crates/maos-bin/src/operator_door.rs
```

After AC3 and after the two AC4.1 runs, these exact commands ran, each exiting **0**:

```sh
cp /tmp/maos-w3-19-5-plants/admission.rs /home/lunarpulse/wt-w3/crates/maos-bin/src/admission.rs
cmp /tmp/maos-w3-19-5-plants/admission.rs /home/lunarpulse/wt-w3/crates/maos-bin/src/admission.rs
touch /home/lunarpulse/wt-w3/crates/maos-bin/src/admission.rs
```

After each of the three AC4.2 runs, these exact commands ran, each exiting **0**:

```sh
cp /tmp/maos-w3-19-5-plants/hot_swap_same_major_lifecycle.rs /home/lunarpulse/wt-w3/crates/maos-kernel-core/tests/hot_swap_same_major_lifecycle.rs
cmp /tmp/maos-w3-19-5-plants/hot_swap_same_major_lifecycle.rs /home/lunarpulse/wt-w3/crates/maos-kernel-core/tests/hot_swap_same_major_lifecycle.rs
touch /home/lunarpulse/wt-w3/crates/maos-kernel-core/tests/hot_swap_same_major_lifecycle.rs
```

The pristine sources include pre-existing working-tree corrections; no `git checkout`/HEAD restore was used. Every restored source was touched to force cargo to rebuild rather than reuse the planted executable. All six distinct exact witnesses were then run on the restored tree:

| Restored witness | Exit | Observed summary | Wall seconds |
|---|---:|---|---:|
| AC1 cold successor | 0 | `1 passed; 0 failed` | 11.34 |
| AC2 incompatible target | 0 | `1 passed; 0 failed` | 1.58 |
| AC3 cross-major refusal | 0 | `1 passed; 0 failed` | 1.49 |
| AC4 admitted bundle wrapper | 0 | `1 passed; 0 failed` | 1.14 |
| AC4 same-major daemon swap | 0 | `1 passed; 0 failed` | 1.27 |
| AC4 kernel continuity | 0 | `1 passed; 0 failed` | 3.08 |

Final required kernel-pin command:

```sh
env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo-maos-w3 nice -n 10 cargo nextest run -p xtask -E 'binary(kernel_pin_content_hash_16_0)'
```

Observed **exit 0**, **48.28 s** wall: `25 tests run: 25 passed, 0 skipped`. `live_tree_matches_the_committed_pin` passed. Full local log: `/tmp/maos-w3-19-5-plants/restored-kernel-pin.log`.

## Diff SHA-256

| Diff | SHA-256 |
|---|---|
| `ac1-bare-load-plant.diff` | `8f5a9820d19aaf7cbcf50699ab04663cabc24801c57279c9873320edd85fb0b6` |
| `ac2-abi-precheck-removed-plant.diff` | `39f699208690d961af224458f2194bd2ec784a1d7593153a7758bf2ccec527e4` |
| `ac3-fr11-gate-removed-plant.diff` | `263dadf573252bf4a1af7b551b2a5c7f2cfb3ae59075a717485cfcd61b415cb3` |
| `ac4-bundle-drops-hot-swap-plant.diff` | `a33ea033cd686b180ff3d73e2d85796e0033898044b7be426c13c13163a4fe62` |
| `ac4-continuity-unload-load-plant.diff` | `aefc0fedbc12eaf68a9b4b53ffc63799a794908a47f1721d8ba1e4400051286d` |
| `ac4-continuity-masking-removed-token-plant.diff` | `8f52dd8267745f1bce215a149f98fd3ab5b719c22831a2fa55f95b9cd4d98e2d` |
| `ac4-continuity-masking-removed-memory-plant.diff` | `a543c17e991b805ae94ec7cf1705f8f38c319a1c293a7fd764fb86384f9f5f47` |

## Hosted evidence — none required (plants are LOCAL-ONLY)

Re-affirmed 2026-10-30 by the records pass: every 19-5 plant is **local-only** — the daemon witnesses run Butler in-process, the admitted-SCB witness uses a hermetic real admission in an archive-isolated libtest child, and the continuity witness an in-process kernel harness; none needs T3 or a hosted workflow, so `ci/ci-w3-verify.yml` carries no 19-5 planted step. The **six restored witnesses** (above) re-ran green on the unplanted tree — `ac1` cold successor 11.34 s, `ac2` incompatible target 1.58 s, `ac3` cross-major refusal 1.49 s, `ac4` admitted-bundle wrapper 1.14 s, `ac4` same-major daemon swap 1.27 s, `ac4` kernel continuity 3.08 s — plus the `kernel_pin_content_hash_16_0` suite 25/25 (48.28 s). Hosted coverage for 19-5's vectors rides the unplanted controls of 19-4's branch runs: `ci/w3-verify` **40980181567** (first) and **40986864882** @ `5026ceea` (all green, `one_daemon_one_door_16_1` under the provisioned substrate), with the landing-shape validation **40980187631** (6 failures: 19-4's clear-twin journey assertion + one `root_shutdown_unload_16_3` flake not reproduced) → **40986869634** @ `59ee0412` (**192/192**).

## Open items

None for this plant assignment. All required controls were runnable locally and went red at the recorded oracles; all planted sources were restored byte-identically; all distinct restored witnesses and the required final kernel-pin suite passed. The story/epic kloc paragraphs and `xtask/kloc.toml` are owned by the concurrent re-pin assignment and were not edited here.
