# 20-10 AC5: retain CID authority across runtime removal

Baseline: `73e7afbcae81f4fb4858e9d52ded3d592bed636d`. Incident: hosted verification run41803510235. See `hosted-diagnosis.md` for exact head/base/plant job and raw-log citations.

This preserves the custody-only phase. Main subsequently added enumeration and
S5 repairs; final counts, adjudicated caps, combined digests and proof are in
[`combined-plants.md`](combined-plants.md). Earlier measurements/digests below
are historical snapshots, not the final combined tree.

## Classification

The independent-home fixture fix works: both real heads remove both owned CIDs and preserve the live foreign CID and label-less impostor, then fail on spawn custody. Both real cgroup/native-binding scenes pass. The base hits both required intended reds; the no-op removal plant hits its intended red, but the restored source fails on the same production custody defect. These are not StoreInUse or timing-flake failures.

The old implementation first removed containers and only afterward read `<spawn>/cid`. Podman's documented removal deletes that file. The historical hosted logs did not print its contents/listing; that link is source-supported inference for those old logs. The strengthened hermetic runtime now actually unlinks matching cidfiles during `rm`, reproducing this production ordering defect locally.

## Failing before / green after

| Witness | Observed before / mutation | Final observed result |
| --- | --- | --- |
| `purge_residue_20_10.rs:202` `ac5_only_absent_cid_and_empty_state_are_reclaimed` | Original production + faithful cidfile-deleting stub: exit100, `removed container's custody leaked after runtime deleted its cidfile` (`cidfile-red-before.log`) | PASS in final14-test purge suite; all former absent/present/unknown/live/legacy/receipt assertions preserved |
| `:265` `ac5_missing_inventory_cid_is_retained_after_container_removal` | Negative authority boundary: missing CID at inventory never grants deletion | PASS; pid/custody retained and receipted after actual modeled container removal |
| `:289` `ac5_inventory_directory_replacement_retains_both_inodes` | Runtime removal swaps the captured path for a new owned0700 directory | PASS; replacement data and original held-inode data both survive |
| `xtask/tests/epic17_exit_workflow_17_7.rs:471` restored checker | Execute the actual cut's checker on log fixtures | PASS for selected required scene +33 filter exclusions; reject explicit SKIP, wrong required name and zero run |

Inventory captures only a validated CID plus a held private/euid-owned no-follow directory. Reclamation uses the captured CID with a new bounded `container exists` exit1 proof and rechecks ownership/mode and held/current device+inode. Final removal is bound to that held directory. Missing/invalid-at-inventory remains conservative; a missing file alone is never deletion authority. See `custody-scan.md` for adjacent-consumer audit.

## Capture-step proven-red mutation

Mutation: replace the stored spawn `proof` with `proof: None`, discarding the inventory capture. The strong AC5 witness exits **100** on `absent CID custody leaked`, before its later removed-container assertion. This is the intended capture-loss assertion, not a compile/setup failure. The mutation is `capture-discard.diff`, SHA-256:

`ec5b121aff6690f803271519b7809f8b1bbfb128c17c8f99ee2a274558aebc31`

Source was saved, then restored by copy-back + `cmp` + `touch`, each exit0. Restore intermediary removal and final formatting exited0. No mutation or restore scratch remains. Exact absolute-path commands and exits are in `commands.json` / `commands.md`; `diff -u` exit1 is the normal differences-found result, not a test failure.

## Final local matrix

- Registered scene siblings provisioned: exit0,5 artifacts.
- Serial focused nextest: **88/88**, exit0,97.929 s test time. Binaries: egress5, uninstall23, purge14, shutdown12, terminal34. There were25 filter-excluded bin-unit tests, not configured skips in these five selected suites. Saved junit is adjacent.
- Library runtime safety twins: **3/3**, exit0;21 other library tests filter-excluded. This separate run corrects the initial selection of `--bin maos`, which does not contain library-side purge tests.
- Executed workflow contracts: **5/5**, exit0; includes real checker behavior and both delegated-unit validations.
- Final fmt and KLOC: exit0. maos-bin **34537** (+33; under story ceiling34548), aggregate **193173** (+33 over inherited193140; under inherited post-20-4 ceiling193526), exact hardfail **193174**. xtask44452 unchanged. No kernel source change; kernel baseline gate is in the final ledger.
- Complete non-denying workspace/all-target clippy: exit0, Rust1.98.1. Ratchet exit0: **1331 occurrences /871 source-distinct**, unchanged baseline1383/897. Comparison against the complete20-4 corrected local stream finds **no new lint/file/message multiplicities**. Three touched maos-bin Rust files have no warnings; the xtask test retains its pre-existing line15 `needless_borrows_for_generic_args` unchanged. No baseline edit.
- Structural/record gate commands and final outcomes are in the command ledger. No new full workspace/default/prepush or docs-site build was requested/run for this scoped follow-up.

The first KLOC attempt was blocked by a peer's `.rs` evidence witness entering the production scan. Its owner renamed it to a non-source suffix; this agent did not modify that work. The next measurement correctly failed the old exact pins with +33; after measured re-pin, the gate passes. No scan exclusion or budget workaround was introduced.

## Hosted cut and digests

Hermetic count19 now requires both new negative vectors; terminal count remains34. Restored green selects exactly1 required scene through the shared `check-log`, permitting33 filter exclusions while refusing runtime SKIP. Baseline/plant intended removal/dead-leaf diagnostics and copy/cmp/touch remain unchanged. Existing removal-noop patch still applies (`git apply --check`, exit0). No workflow was installed under `.github` or pushed.

- `implementation.diff`: `0e6e845215187b976747f2fd4e31c2e41bea3fd0bb77935faf26d2c94add0e42`
- `verify-cut.diff`: `46bd5c420ac88065fe66d8388f5947ba230fe2cdf574ad1fe2e5c211eeb2a227`
- Full `ci-20-10-verify.yml`: `4f5a8ee0339bd2c270a8eb21298aef9c083480c93880a391a69212f99d90df60`

Digests are also recorded in `diff-sha256.json`. New real hosted delegated/Podman head and restored-plant green, expanded hosted failure diagnostics, and hosted-toolchain lint remain **unverified**, pending operator rerun. Local substrate-gated scene returns are not that proof.

## Cleanup

After all cargo jobs completed, a `/proc` scan examined29 PIDs: no active cargo/rustc/clippy/nextest/rustfmt, no candidate fd/maps/cwd/exe reference, no inaccessible same-UID entry (four inaccessible other-UID entries recorded). Nonblocking flock/POSIX locks guarded removal of **19 individual stale sessions** belonging only to the three affected test crates, older than one hour before task start. `du` allocated total: **234217472 bytes**. Observed filesystem free-space increase after flush: **167460864 bytes**; this global delta is not an exclusive-reclamation guarantee. Current sessions, crate roots, lockfiles, other crates, deps and target roots remain. No cargo clean/broad deletion. Named workflow scratch and source restore intermediary are absent. Full scan/removal evidence is adjacent.
