# Rebased HEAD and final working-tree cites

Immutable baseline: `cb0abc06e6b085cb63db1be60d4addbeb2963811`. Original failing-before controls used `76efc19d3ffd36a1b897724704d76f94fba3d9eb`. Both are distinguished from the uncommitted L1 implementation below.

`l1-rebased-cite-extractions.json` records the exact seven `git show <base>:<path>` commands, each exit0 and SHA256, used to re-read the rebased sources. `l1-baseline-rebase-source.diff` is the exact restricted-path diff between those baselines: only `main.rs` changes, adding `cfg(feature="network")` to `configured_provider_catalog` and tightening its review module to `cfg(all(test,feature="network"))`, around baseline lines16799/16822. The erasure implementation and the other six files are unchanged between the original before-witness base and the resumed HEAD. The priority hotfix is preserved.

## Entry-point map

All paths are relative to the repository. Baseline columns refer to immutable `cb0abc06`; final columns refer to the formatted working tree, not a claimed commit.

| Path / surface | Rebased baseline name:line | Final name:line |
|---|---|---|
| `crates/maos-bin/src/main.rs`, door private operation | `run_uninstall_cascade:10134` | `run_uninstall_cascade:10137` |
| same, cascade wrapper | `run_uninstall_cascade:10495` | `run_uninstall_cascade:10629` |
| same, cascade implementation | `run_uninstall_cascade_inner:10557` | `run_uninstall_cascade_inner:10691` |
| same, retained-store inventory | not present | `RetainedUninstallStore:10503`; `UninstallContext:10623` |
| same, intent lineage status | constant `VerifiedEmpty:10887-10888` | query-derived category:11075 |
| same, pending halts | generic gap:10891 | explicit final-KS/O4 gap:11085 |
| same, schedules | generic gap:10897 | observed ids/retained watchdog gap:11091 |
| `crates/maos-bin/src/operator_door.rs`, private trait | `run_uninstall_cascade:68` | schedule-aware `run_uninstall_cascade:68` |
| same, uninstall / snapshot boundary | `uninstall_command:1609` | `uninstall_command:1613` |
| same, forget | `forget_command:2039` | `forget_command:2062` |
| same, release | `release_legal_hold_command:2083` | serialized release:2106 |
| `crates/maos-bin/src/hook_pump.rs` | `complete_forget:642` | `complete_forget:644`; off-worker recovery call:660 |
| `crates/maos-iac/src/adapter/transparency_log.rs`, original scans | `forget_frames:1555`; `scrub_forget_frames:1636` | shared pager:1663; `scrub_forget_frames:1764` |
| same, recovery | unbounded `outstanding_redistill_requests:1722` | paged/indexed `outstanding_redistill_requests:1863` |
| same, all-incarnation lineage | not present | `distillates_written_by:1470` |
| `crates/maos-audit/src/erasure/proof.rs` | `build_erasure_proof:167`; `verify_erasure_proof:284`; against-log verifier:405 | `CategoryEvidence:57`; builder:179; verifier:320; against-log verifier:457 |
| `crates/maos-registry/src/storage.rs` | `default_installed_manifests:136` | boot query:186; `PendingPackageUninstall:110`; `uninstall_package:219` |

## Final acceptance witness locations

Under `crates/maos-bin/tests/`:

- `erasure_inventory_20_11.rs`: principal-free signed proof151; boot/pid isolation188; installed package/schedules205; archive/authored inventory224; both verifiers266; actual empty query373; principal plus principal-free386; held derivation/package401; failed-proof package retention449.
- `erasure_live_20_11.rs`: local live door/refused-boot recovery302; hosted T3 Worker306 (explicit local missing-substrate skip, not hosted proof).
- `erasure_scan_20_11.rs`: late hold survives7.
- `shell_host_16_2.rs`: serialized release1421;100k continuous writer1503;1M completion/repeat1508; real pre-unload SCB schedules1659.

Under `crates/maos-iac/src/adapter/erasure_scan_work.rs`:100k VM work21;1M VM work26; timestamp/frame-id ordering96; opaque outcome storage / honest recovery error132. Its leading `#![cfg(test)]` explicitly marks the whole external module test-only.

FR4 closure: `crates/maos-audit/src/fr4_classifier.rs:565-575` records the measured kind1/token-absent `scrub_uninstalled_distillate` shape. `crates/maos-audit/tests/fr4_classifier_16_2.rs::every_writer_site_demanding_a_disposition_has_one:922` and `cfg_test_items_are_skipped:1020` cover production inventory and the actual file-level compilation boundary, including the production-only negative control.

Hosted enrollment is inherited, not fabricated: `.github/workflows/discipline.yml:6099-6100` pins the full workspace job to `ubuntu-24.04`; `:6142` provisions the signed T3 substrate. The new integration target is included by that existing whole-workspace run. No hosted run id is claimed for this uncommitted working tree.

Kernel source and its baseline/surface files are untouched. The final kernel baseline gate reports26841 physical lines,95 files and identical set/content hash `db39bc4a1ab6a6c22575990ae934e698a519d3efd46a8dbb38e62bc376697b59`; tokei remains21042. AC3, KS and O4 are not implemented by L1.
