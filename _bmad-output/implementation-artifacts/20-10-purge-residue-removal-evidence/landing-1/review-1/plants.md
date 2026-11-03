# 20-10 landing 1 — review correction plants

Review baseline: `ac1fc12a7f95c4ab003f35d5ca479c181c0f22d2`. Initial landing evidence remains a historical snapshot in `../plants.md`; these additive vectors do not rewrite its red-before results. All commands unset `CI` and `GITHUB_ACTIONS`, use the inherited cargo target, and run from `/home/lunarpulse/dev_ws`. Exact argv, environment overrides, exits and timings are in `commands.json` / `commands.md`.

## Before production corrections

`review-red-before.log`: five new vectors ran against the review baseline and all five failed (nextest exit 100): P1 actual live second-home supervisor cached root was removed; P2 shared `maos.slice` parent was eligible; F1 surviving custody returned exit zero; F2 unset HOME was accepted; F3 a HOME-equal purge root was accepted. P3/P4/P5 and the AC6 scan are witness hardening, not claims that those previously correct production behaviors failed at baseline.

## One mutation per finding

Each diff was captured before its single test run. Every mutation produced a test assertion failure (exit 100), not a compile failure. Immediately afterward the source was copied back from its byte-for-byte backup, `cmp` exited 0, and `touch` exited 0 to invalidate the built artifact. `restore-*.log`, `cmp-*.log`, `touch-*.log` and the command ledger record each restoration. No mutation remains in the source tree.

| Finding | Witness (`name:line`) | Mutation | Red exit | Restored green |
|---|---|---|---|---|
| P1 | `worker_supervision_16_3.rs:2857` `purge_review_p1_preserves_another_live_supervisors_cached_root` | Reclaim any empty process root without dead-PID proof | 100 | PASS |
| P2 | `purge_runtime.rs:633` `review_p2_shared_slice_parent_is_never_reclaimed` | Inventory/reclaim the shared slice parent | 100 | PASS |
| P3 | `purge_runtime.rs:547` `ac4_dead_empty_live_creator_populated_and_symlink_twins` | Remove `NOFOLLOW`; valid-name symlink leaf must fail independently of its name, with linked-ancestor twin also asserted | 100 | PASS |
| P4 | `purge_residue_20_10.rs:137` `ac1_owned_cid_removed_before_workspaces_and_receipted` | Move runtime removal after workspace removal; runtime stub requires sentinel still exists at `rm` | 100 | PASS |
| P5 | `purge_residue_20_10.rs:202` `ac5_only_absent_cid_and_empty_state_are_reclaimed` | Suppress retained custody report and receipt update | 100 | PASS |
| F1 | `purge_residue_20_10.rs:382` `review_f1_surviving_residue_has_an_explicit_summary_and_nonzero_exit` | Return exit zero despite residue | 100 | PASS |
| F2 | `purge_residue_20_10.rs:435` `review_f2_unset_relative_home_and_legacy_ancestor_links_are_refused` | Omit legacy ancestor validation | 100 | PASS |
| F3 | `purge_residue_20_10.rs:474` `review_f3_home_ancestor_and_parent_components_are_refused` | Omit HOME/ancestor root guard | 100 | PASS |
| AC6 | `purge_residue_20_10.rs:247` `ac6_runtime_helper_is_shared_and_receipt_convention_is_preserved` | Put ambient `std::process::Command::new(...).output()` runner in the extracted purge runtime module | 100 | PASS |

`focused-final.log`: 147/147 passed, no configured skips, exit 0. Local substrate-gated scenes may return early; this is not delegated-runtime proof. Full serial/default suites each ran 5,571 tests, all passing, with 138 configured skips. The explicitly prescribed prepush also exited 0, with two budget advisories recorded rather than retried.

## Diff SHA-256

| Diff | SHA-256 |
|---|---|
| `P1-live-cached-root.diff` | `ad8e171611898c079bf0cec867175942fc0d9506415ccbea6e45a9f998f21f67` |
| `P2-shared-slice.diff` | `dff2f76756730b7ebfb31c5474a5d4fcb56f21af8e8d6324f03cf40f4f5246a8` |
| `P3-linked-leaf-and-ancestor.diff` | `28ba7dd71dee28afc38c71a309886c4b5a1100fa1ae77c0062e619bc0dcd0a66` |
| `P4-container-before-workspace.diff` | `9987a24107adb2c97496a7135a5291d69c2f5e3b7fa8e398a05be4fa36185c0c` |
| `P5-retained-report-receipt.diff` | `75ade5783f5e684ddd1d270719237e659e562486b87fa6b2502345ad3374c5e8` |
| `F1-residue-exit.diff` | `e5d5fcca0ea1a9b8b5780d1d70c10ba5845b82b2ab647903b30c8d550edde2ba` |
| `F2-legacy-ancestor.diff` | `22f5bdaaf4b6ce4d4a3feb8a561cc2192b960b8fd1637ce232a2c778d22dc32f` |
| `F3-home-boundary.diff` | `b3353cb8facd9552fda5a8e90e1b6c6b10bec17dd2e759bf06ae870ccdd1c1f5` |
| `AC6-ambient-runner-in-module.diff` | `516887d6fd2c932edd5996a2b3de6b57997e8e9ff44dd69ad55618e0a3f26645` |

## Hosted-only boundary

`../../ci/ci-20-10-verify.yml` remains an unpushed branch-only cut, including baseline/head/plant real Podman and delegated epic-17-exit scenes, byte-restored AC1 mutation, Docker T0 decision probes and a same-hosted-toolchain warning comparison. Its removal mutation is unchanged by these corrections (SHA-256 `a93f7c5b725ad90be87c625198601194bafc6a3ea0cbee364538d733971ae80b`). No hosted execution or unforgeable same-UID provenance is claimed. Labels/name/mount are attribution within the documented rootless-user/PATH trust boundary.
