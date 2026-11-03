# Hosted follow-up: independent lock ancestry

Baseline: `2752e750c8cc7cfcb6fed2ccabbd4e4e0ed4f040`. This is a fixture correction, not a production lock-policy change.

## Hosted diagnosis

Main discipline run **41610868202** and verification run **41610893716** submitted `8c809f9c`. Their remaining failure was `root_crash_retains_stopped_container_and_purge_reports_it`: target `<scratch>/home` and live foreign `<scratch>/foreign-home` shared the user-HOME/locked ancestor `<scratch>`. The live foreign process correctly prevented exclusive purge. Distinct MAOS_HOME leaf names did not provide independent lock ancestry. See `branch-run-diagnosis.md` and `main-run-classification.md` for exact job/log citations.

The head-image real cgroup/native-binding scenes passed in both images; those checks are not implicated. The baseline and plant root-crash failures were StoreInUse, **not** the intended owned-CID-survival red, and must not be counted as accepted reds. No timing flake explains this defect.

## Local failing-before witness

Witness: `crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs:157`, canonical shared-ancestor assertion at lines174-176. The invariant is deliberately before the optional T3 scene gate. It tests that no locked ancestor below canonical OS temp root is shared by target and foreign roots.

With the new assertion applied to the original HEAD fixture layout, the focused nextest command exited **100** with `Some("/tmp/.tmpo6iLjA")` and the intended `share locked ancestor below OS temp root` diagnostic. See `layout-red-before.log`. This is an observed red on the original layout, not a claim that unmodified HEAD runs a new assertion.

Fix: allocate the foreign MAOS_HOME under an independent `tempfile::tempdir()`, declared before the foreign LiveRoot so the process is dropped before its own temporary ancestry. Keep first/second target-root sequencing, CID removal, foreign CID survival and label-less impostor survival assertions intact.

## Proven-red mutation and restoration

| Mutation | Intended failure | Observed exit | Diff SHA-256 |
| --- | --- | --- | --- |
| Replace independent `tempdir()` with `tempdir_in(dir.path())`, restoring a shared locked ancestor through a nested directory | Same pre-gate canonical shared-ancestor assertion (`Some("/tmp/.tmp1S0d2w")`) | 100 | `616870463ce323148a3ba1af2739ca6d0a977c6a5795faf0fd9b752d002d60bf` |

`nested-foreign-plant.diff` and `nested-layout-plant.log` preserve the mutation and failure. Original-source copy, copy-back, `cmp`, and `touch` each exited **0**; exact absolute paths and commands are in `commands.json` / `commands.md`. The restore intermediary was removed (exit0), and final `cargo fmt --all` exited0. No active mutation remains. Both named witness TempDirs are absent after RAII cleanup.

The reviewable fixture-only fix is `layout-fix.diff`, SHA-256 `c16a69428338bb36eb9971b54ef81d33001a8aec93a868b3242d850e310d5b57`. `diff-sha256.json` records both digests.

## Green and proof limits

Focused serial nextest ran all **81/81** tests across `worker_terminal_facts_17_7` (34), `purge_residue_20_10` (12), `maos_uninstall_16_4` (23), and `root_shutdown_unload_16_3` (12), exit **0**, 96.520 s test time. The structural invariant executes even without the optional T3 substrate. Logs and fresh junit are adjacent. The terminal suite count remains **34**, not35.

The scoped fixture scan found no other success-expected live-foreign purge twin to relocate. The same-store live-root refusal witness is intentional and unchanged. Production ancestor locking, StoreInUse refusal, signed-substrate gates, intended-red diagnostics and the workflow's copy/cmp/touch/restored-green checks are unchanged; see `fixture-scan.md` and `workflow-guard-check.log`.

This follow-up has **not** obtained a new hosted real-podman/delegated baseline/head/plant result. The existing verification cut must still prove owned-CID survival for the intended base/plant red, not StoreInUse; `dead-creator empty leaf survived` remains required for the baseline cgroup leg. Local pass/self-return is not that proof. Parent scoped this follow-up to focused serial suites, formatting, structural/record gates and clippy delta; no new full workspace/default/prepush matrix was requested or run.

## Counts, lint and cleanup

`kloc-check` passed with no over-budget rows: maos-bin34363, aggregate192896, xtask44420; inherited aggregate hardfail192897 is unchanged after the separate 20-4 landing. Kernel remains26841 physical /21042 code lines, with no changed/added/removed kernel files. This fixture-only follow-up adds no counted production lines or pin changes.

Local Rust1.98.1 non-denying workspace/all-target clippy exited0. Complete-stream ratchet exited0 at **1331 occurrences /871 source-distinct** against unchanged1383/897 baseline. The sole changed Rust file has **zero primary warnings**, hence no new lint/file warnings in this local stream. Hosted-toolchain lint after this fix remains unverified.

Cleanup scanned27 `/proc` PIDs: no cargo/rustc/nextest/clippy/rustfmt process, no candidate fd/maps/cwd/exe reference, and no unreadable same-UID process. Four inaccessible entries belonged to other UIDs and are recorded. Nonblocking flock and POSIX locks were acquired before removing **14 individual stale terminal-suite incremental sessions**, all older than one hour before task start. Removed sessions accounted for **653934592 allocated bytes** by `du`; after Btrfs flush the observed filesystem free-space increase was **12709888 bytes**. This is not a promise of exclusive physical reclamation: surviving hardlinks/reflinks and unrelated host activity can affect the filesystem-wide delta. All crate roots, session lockfiles, other crates, deps, target roots and current-task sessions were retained. See `cleanup-proc-scan.json`, `cleanup-result.json` and exact command ledger. No cargo clean or broad target deletion.
