# 20-2 landing-1 acceptance evidence

**Historical pre-review record.** Source line references, counts and results
below describe the initial cbed4dad landing only. Current 88c7277c review
corrections, exact rebase, current size and witness lines are in
[review-corrections/record.md](review-corrections/record.md); no original result
has been overwritten or promoted to hosted proof.

Baseline `cbed4dad`; uncommitted implementation. Commands and exit codes are in
neighboring `.log` files. `runlog` removes CI/GITHUB_ACTIONS from local commands;
standard gates additionally set `CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask`.
The declared-CI prerequisite witness explicitly sets both variables to true.
`CARGO_TARGET_DIR` was absent in this agent environment; it was never set or
overridden. Cargo resolves the repository's existing target symlink; gate calls
use `/mnt/build/cargo-maos/debug/xtask`. No cargo clean was invoked.

## AC witness table

| AC | Final witness (`name:line`) | Before / plant evidence | Final observed result |
|---|---|---|---|
| AC1 | `packaging/test_release_20_2.py:45,48,59`; `packaging/deb/rules:18`; `xtask/tests/package_workflows_20_2.rs:15` | Creation W1 rc25/no changelog and W2 rc2/missing separator; cbed4dad source re-read. Hosted baseline and wrong-key deb build plant: `../ci/ci-20-2-verify.yml:65,77`. | Local real Ed25519/exact-name vectors pass; rendered make parse rc0. Real deb builds/install/removal on amd64/arm64 and signature plant **UNVERIFIED**. |
| AC2 | `xtask/tests/package_recipes_20_2.rs:6,16,38`; `packaging/test_release_20_2.py:64` | `ac2-before-recipes.log`: baseline check rc1 across four recipes, stale key/version/tokens printed. Actual formula SHA-token plant: `ac2-token-plant.log` rc1, 0.08s; `ac2-restore.log` copy/cmp/touch rc0. | Renderer vectors 3/3; real four-binary release manifest signed/rendered rc0; restored renderer rc0. Python duplicate-name vector passes. Native recipe install/audit/byte identity **UNVERIFIED**. |
| AC3 | `tests/air-gap-netns-corroborate.sh:20,25,48,51`; `packaging/test_release_20_2.py:69`; `crates/maos-bin/src/main.rs:1610` | `ac3-before-stub.log`: baseline run rc0 writes only `transparency.airgap-stub.log`; no SQLite file in baseline audit directory. Hosted baseline/outbound plant recipe at `../ci/ci-20-2-verify.yml:120,128`. | Real local init + replay + expected response + TL growth rc0 **without isolation**. Final compile-shape run rc2; local missing strace rc77 and declared-CI refusal rc1. Trace-oracle family vectors pass. Netns zero-outbound, canary and outbound plant **UNVERIFIED**. |
| AC4 | `xtask/src/check_ship_gate_completeness.rs:130`; `xtask/tests/ship_gate_binding_20_3a.rs:75`; `../ci/ci-20-2-verify.yml:218` | `ac4-before-enrolment.log`: baseline git grep has zero enrollment hits, rc1. `t0-airgap.log` measured red; full symbol list retained. Hosted default-subject plant authored with copy/cmp/touch restoration. | Conditional D-02 C applied. Final symbol advisory rc1 on TcpListener/getaddrinfo; dirty fixture rejected; same-binary mock scan rc0. 48-gate reconciliation rc0. Hosted subject plant **UNVERIFIED**. |
| AC5 | `xtask/tests/package_workflows_20_2.rs:51`; `packaging/test_release_20_2.py:54`; `Dockerfile:10` | Historical hosted run 36094027288 failed Docker Hub login before build (creation W3, not re-run here). Real image-byte plant authored at `../ci/ci-20-2-verify.yml:165`. | Credential-gating YAML vectors pass; changed-byte crypto rejection passes locally. Docker build/smoke/size, image plant, push/sign/verify **UNVERIFIED**. Operator publication job at `../ci/ci-20-2-verify.yml:265` requires explicit dispatch + existing signed tag + credentials. |
| AC6 | `crates/maos-bin/tests/maos_uninstall_16_4.rs:180,309`; `crates/maos-bin/src/purge.rs:15` | Actual old-hint plant: `ac6-ac7-old-hint-plant.log` rc101, missing installer-hint assertion, 45.96s. `ac6-restore.log` copy/cmp/touch rc0. | Restored full purge suite **23/23**, stdout and receipt both checked. Real package-owned four-binary removal is authored in `packaging/witness-deb.sh`, **UNVERIFIED**. |
| AC7 | `docs/maos.dev/install.md:64`; discipline `cargo-install`; verify recipe `cargo-and-symbols` | `ac7-before-route.log`: exact baseline Cargo git/tag route absent, rc1. Shares AC6's single old-hint plant (no duplicate mutation). | Correct four-package Cargo install/removal documentation present. Genuine hosted temp-root install/version/uninstall and wall clock **UNVERIFIED**. |
| AC8 | `tests/integration/onb_nfr2_timing.sh:115-181`; `size.json`; `standard-check-dev-record-completeness.log` | Historic size breach and compile debt retained with explicit CLOSED landing-1 provenance; real old ceiling not silently changed. No done-status plant/flip is authorized. | Original size leg rc0: **31,677,432 < 33,554,432 B**. Coverage/dev-record/epic coherence gates rc0. Compile and size rows CLOSED; hosted NFR-Ops-12 proof re-owned under rule10 to `ops-rc-signed-tag`. |

The local Python suite passes six real cryptographic/trace vectors. The system
Python lacks cryptography; the suite ran in a scratch venv with observed pip
installation (`python-verifier-dependency.log`). This does not claim the system
Python prerequisite is provisioned for unrelated tests.

## Measurements and local verification

- Baseline kernel gate rc0: 26,841 physical / 95 files, set hash
  `db39bc4a1ab6a6c22575990ae934e698a519d3efd46a8dbb38e62bc376697b59`.
- Final kernel rc0, no drift; kernel tokei 21,042 unchanged.
- Exact maos-bin **34,842 -> 34,823**; xtask **44,466 -> 44,673**;
  aggregate **193,911 -> 194,099**, hardfail **194,100**. D02 A re-base and
  per-component ceilings are in `preflight-decisions.md`. All other crates
  unchanged. No additional kernel/headroom grant was used.
- Release binary build rc0, genuine default debug maos/maosctl build rc0;
  stripped size and GLIBC_2.39 floor measured, not inferred from the alpha.
- `cargo fmt --all` rc0; shell syntax rc0. Affected all-target clippy rc0,
  664 warning events total, zero primary diagnostics intersecting changed
  production ranges (`clippy-delta.json`); no hosted compiler proof claimed.
- Focused results: renderer 3/3, workflow 3/3, release-workflow 2/2,
  purge 23/23, no-default verb table 7/7, tree-bound nextest 21/21,
  macOS guard 27/27, ship-gate binding 8/8, coverage 3/3 and internal
  48-gate assertion 1/1 pass.
- Standard gates rc0: kernel, kloc, ship-gate completeness, exit commands,
  decision register, dev record, epic coherence, coverage matrix, test cost.

## Workspace execution — not green

One explicit execution reused generated workspace cargo/binaries metadata:
run `24d43741-bd54-4575-943f-6c2fea571c5d`, rc100, **5646 run / 5640 pass /
6 fail / 138 skipped**, 138.49s command wall clock. Four landing-caused
cardinality/matrix/reference failures were corrected; focused correction suites
pass. The other failures were:

- `maos-bin::erasure_inventory_20_11::ac1_category_evidence_is_required_by_both_verifiers`:
  missing system-Python cryptography. Protected code not changed; reported to Main.
- `maos-bin::one_daemon_one_door_16_1::ac1_19_5_a_refused_cold_swap_successor_reverts_to_the_predecessor`:
  `cold_swap_predecessor_unknown` rather than `cold_swap_reverted`. Unclassified,
  not claimed a known flake, not retried to green; reported to Main.

The first metadata compile timed out at its 2900s cell deadline; the original
compile completed before metadata capture resumed (27.27s). A metadata run
invocation mistakenly combined `--workspace --locked` with reuse options and
exited 2 before any test; the corrected invocation is the single execution above.
The mandatory final prepush is a distinct prescribed gate that also contains its
own unconditional workspace leg; it is run once, not used as a retry-to-green.
Its exact command and actual disposition belong to `prepush.log`/`prepush.stdout`.
No tracked files are edited while it runs. Hosted discipline and every hosted
plant remain **UNVERIFIED** until the operator records Actions run ids.

## Prescribed final prepush and cleanup

Exactly one invocation was made:

```bash
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated /mnt/build/cargo-maos/debug/xtask prepush --json
```

The 2900-second execution deadline expired (2900.02s observed). The wrapper
terminated the entire prepush process group and recorded **124**; this is a
timeout, not a completed xtask verdict. The command buffers its final JSON, so
`prepush.stdout` is empty. No stage-level result or green prepush is inferred.
No tracked file was edited while it ran; it was **not retried**. The subsequent
/proc scan found no active cargo/rustc/nextest/clippy process. Main was notified
of this incomplete required proof and the two unrelated workspace failures.

The generated intent-lineage report was byte-compared with its original
snapshot: unchanged, so no restore was needed. Own Python import caches and
`/tmp/maos-20-2-dev202` were removed. After scanning /proc and open descriptors,
cleanup removed **515** stale incremental `s-*` snapshots and preserved **6465**,
including newest-per-crate-folder and every snapshot touched in the last two
hours. No deps/build/fingerprint tree was touched.

`cleanup.json` records **6,895,853,568 bytes** of removed allocated-file-block
accounting (hardlinked inodes counted only when every link was removed), of
which **350,384,128 bytes** belonged to own /tmp scratch. Filesystem available
space increased **5,301,002,240 bytes** during cleanup; reflinks/compression and
concurrent host activity mean file-block accounting is not a promise of the
same physical free-space delta.
