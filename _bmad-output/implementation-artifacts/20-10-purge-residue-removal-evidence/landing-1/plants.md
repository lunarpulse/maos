# 20-10 landing 1 — proven-red plants

Baseline: `d3e3f586cb325c16a691cd19d1fd62588b288a0d`. No worktree, workspace copy,
second target directory, commit, or push was used. The six local mutations ran
in the real tree, one per AC, and were restored by explicit **copy-back + cmp +
touch** commands. Exact argv, rc and elapsed seconds are in `commands.json`.
`plants.json` and the diffs preserve each control. Restore copies were removed
after use; every cmp exited 0. `focused-restored-green.log` is the combined
restored-tree proof: 142/142 PASS, rc 0 (real T3 scenes locally self-skip, not
hosted evidence).

| AC | Diff | SHA-256 | Red rc | Command wall seconds | Evidence and restore |
|---|---|---|---:|---:|---|
| AC2 | `ac2.diff` | `d93b428e19976873e6bae35515069c49870161139624e070ded7b271154fa86c` | 100 | 25.205 | `plant-ac2.log`; copy-back/cmp/touch each rc 0 |
| AC3 | `ac3.diff` | `e2fb43c2f984a33c48c36c0de9f53a81fe3e87dd700adc110e6f712c4eacc059` | 100 | 5.723 | `plant-ac3.log`; copy-back/cmp/touch each rc 0 |
| AC4 | `ac4.diff` | `f679a3225a974e205e35457cde429fbd5f1b73388dfcdbb367ae79882774b125` | 100 | 3.420 | `plant-ac4.log`; copy-back/cmp/touch each rc 0 |
| AC5 | `ac5.diff` | `65209845e9eb6b131504c30ee24e3c6c5f3c2c21ea61487241842ef5a04e31a4` | 100 | 8.488 | `plant-ac5.log`; copy-back/cmp/touch each rc 0 |
| AC6 | `ac6.diff` | `d6769c9236a4d042bb82e9605fbe16430bec94c7cff5ac50ab8fa1282a72fd13` | 100 | 6.575 | `plant-ac6.log`; copy-back/cmp/touch each rc 0 |
| AC7 | `ac7.diff` | `4f04adf35f47104b0b68c1fc854e18b57d493f4dfc9b0b572b403cc2c35ad7e0` | 100 | 5.823 | `plant-ac7.log`; copy-back/cmp/touch each rc 0 |
| AC1 | `../ci/ac1-removal-noop.diff` | `a93f7c5b725ad90be87c625198601194bafc6a3ea0cbee364538d733971ae80b` | not run | not run | Hosted-only cut in `../ci/ci-20-10-verify.yml`; no result claimed |

## Controls

- AC1: make container removal a no-op. The extended real root-crash scene must
  fail `owned CID ... survived purge`; the hosted workflow checks the diff
  digest, then copy-backs, cmps, touches and requires restored green.
- AC2: replace label/name/mount authority with name-prefix authority. The stub
  vector fails while all five CIDs, including the other home and impostor,
  are incorrectly removed (`plant-ac2.log`).
- AC3: call confirmed removal from the dry-run branch. The fixture's empty
  custody directory disappears and its existence assertion fails
  (`plant-ac3.log`); the dry-run vector also constrains runtime verbs.
- AC4: remove the creator-alive check. The local temp-cgroup live-creator twin
  fails `live creator's empty-leaf window must survive` (`plant-ac4.log`).
- AC5: skip the CID-exists check before custody deletion. The present-CID twin
  fails `live home's custody removed` (`plant-ac5.log`).
- AC6: bypass the bounded scrubbed runner with ambient `Command::output`.
  The stub observes the deliberately planted AWS credential and rejects `ps`,
  causing the shared-helper/receipt vector to fail (`plant-ac6.log`).
- AC7: remove the identity-label rule from ADR-067. Its record witness fails
  (`plant-ac7.log`), then the exact ADR bytes are restored.

## Failing-before witnesses and restored results

`red-before-head.log`: new behavioral/source-record witnesses ran against the
unchanged HEAD production tree, **6 run / 6 failed**, nextest rc 100.
Final definitions (line numbers after fmt):

| AC | Witness (`name:line`) | HEAD red-before | Restored green-after |
|---|---|---|---|
| AC1 | `purge_residue_20_10.rs:127` `ac1_owned_cid_removed_before_workspaces_and_receipted` | missing forced CID removal / container receipt | PASS |
| AC2 | `purge_residue_20_10.rs:153` `ac2_other_home_missing_mount_mismatched_name_and_impostor_survive` | no unattributable-CID report | PASS |
| AC3 | `purge_residue_20_10.rs:169` `ac3_dry_run_uses_only_readonly_verbs_and_preserves_custody` | no `would remove container` line | PASS |
| AC4 | `purge_runtime.rs:535` `ac4_dead_empty_live_creator_populated_and_symlink_twins`; hosted `root_crash.rs:262` | hosted baseline scene cut, **not executed locally**; no fake temp-tree HEAD claim | local PASS; real delegated leg unverified |
| AC5 | `purge_residue_20_10.rs:192` `ac5_only_absent_cid_and_empty_state_are_reclaimed` | empty process root leaked | PASS |
| AC6 | `purge_residue_20_10.rs:215` `ac6_runtime_helper_is_shared_and_receipt_convention_is_preserved` | duplicate runtime wait-loop implementation | PASS |
| AC7 | `purge_residue_20_10.rs:236` `ac7_adr_documents_home_scoped_runtime_removal` | obsolete ADR authority/receipt contract | PASS |

AC2's live-root/runtime-log twin and AC3's storage guard also pass. The full
final serial and default suites each ran 5,562 tests, all passing, with 138
configured skips. Their local T3 self-skips are not real-runtime evidence.

## Producer diagnosis (AC5)

Three production `researcher --once` runs left zero process roots. The isolated
HEAD test `worker_supervision_16_3.rs:1212`
`a_stop_during_the_mint_never_spawns_the_child` itself passed but changed the
private root count **1 -> 2**: the root-count postcondition was red
(`t0-mint-stop-root-count.json`). Its strong `after_mint` latch formed
`supervisor -> seam -> latch -> supervisor`, preventing `Drop` and therefore
`cleanup_state_root`. The fix is test-only: a Weak latch plus a release
assertion. Three fixed runs left the private count **2 -> 2**
(`producer-fixed-counts.json`). No production supervisor or kernel line was
changed. This attributes the measured leaking fixture, not every historical
empty directory on the host.

## Review corrections (additive, after `ac1fc12a`)

The preceding tables preserve the original landing snapshot and line numbers.
Current witnesses, five baseline-red vectors, nine assertion-red mutations,
SHA-256 digests and byte-restoration commands are recorded in
[`review-1/plants.md`](review-1/plants.md). Each mutation was copy-backed,
compared byte-for-byte and touched before the final 147-test focused green
run. Full serial and default suites each passed 5,571 tests, with 138
configured skips; substrate-gated local returns are not hosted proof.
This correction retains live/anonymous process roots and shared parents,
requires safe absolute HOME/legacy ancestors, and reports residue explicitly
with a nonzero exit without changing receipt schema/status vocabulary.

## Hosted incidents (additive, baseline `16c35ef0`)

[`hosted-1/plants.md`](hosted-1/plants.md) records the branch-only harness-unit
regression and the main-run AF_UNIX path-length / fixture HOME regressions.
All three new plants fail on their intended assertions; source/fixture/cut
restorations use copy-back + `cmp` + `touch`. Hosted branch baseline/plant
failures in run 41584515474 are unrelated harness setup failures, **not**
intended removal/dead-creator proven reds. Both T0 jobs passed without
contradicting a ruled D-row. The terminal scene count remains 34.

## Hosted shared-ancestry follow-up (baseline `2752e750`)

[`hosted-2/plants.md`](hosted-2/plants.md) records the deterministic shared-lock
ancestor defect in runs41610868202 /41610893716, the independent foreign
TempDir fix, the original-layout red and the nested-layout mutation. Both reds
hit the pre-T3 structural invariant; copy-back/cmp/touch restore precedes the
81/81 focused serial green. The suite remains34 terminal tests. Production
locking and the hosted cut's intended removal/dead-leaf diagnostics are
unchanged; corrected real hosted baseline/head/plant proof remains unverified.
