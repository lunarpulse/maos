# Superseded interim landing 1 handoff — Dev2011b

Base: `cb0abc06e6b085cb63db1be60d4addbeb2963811`, resumed after the priority park.
A harness wrap-up notice interrupted this step; Main clarified that it was not an
operator wrap-up request and directed continuation. This report preserves the
interim observations only; it is not the final landing disposition. Package
proof-failure retention and its live refused-boot recovery fixture were finished.
At this point AC5's wall result was red, AC6 required hosted evidence, and full
landing checks had not run.
No commit, push, tracker status change, kernel edit, domain edit, KS, AC3 or O4 implementation.

## AC witness table

Paths below are under `crates/maos-bin/tests/`.

| AC | Witness (`name:line`) | Genuine before / plant | Final observed |
|---|---|---|---|
| AC1 | `erasure_inventory_20_11.rs::ac1_principal_free_distillate_gets_a_signed_proof:151`; `ac1_category_evidence_is_required_by_both_verifiers:266` | Original-HEAD NotFound exit101 (`l1-before-inventory.log`, corrected-key log); combined old-NotFound plant red100 | Both pass. Valid Python proof accepted; freshly re-signed malformed proof rejected specifically for schema/category evidence mismatch by Rust and Python. Spirit marker and cost-attribution byte-preservation asserted. |
| AC2 | inventory `ac2_all_boots_and_pid_reuse_are_isolated:188`, `ac2_empty_query_is_verified_empty_when_only_package_remains:373`, `ac2_principal_and_principal_free_lineage_are_both_erased:386`, `ac2_held_derivation_and_installed_package_are_retained:401`; live `ac2_live_door_erases_principal_free_lineage:302` | Original-HEAD missing lineage / false empty reds101 (`l1-before-corrected-key.log`); VerifiedEmpty-lineage combined plant red100 | All pass. Real live door, real boot/pid residue, actual refused installed boot, live spirit_not_loaded, root exit and offline signed removal observed. |
| AC4 | inventory `ac4_installed_package_and_schedule_are_queried_without_principals:205`, `ac4_failed_proof_write_keeps_installed_package:449`; shell `ac4_door_snapshots_real_scb_schedules_before_unload:1634` | Original-HEAD package witness red101; empty door snapshot plant red100; genuine proof-write-failure retention witness red101 (`l1-before-failed-package-proof.log`) | All pass. Rollback-on-drop staged removal preserves installed generation/index when proof persistence fails. Watchdog cleanup explicitly remains a signed final-KS gap. |
| AC5 | scan `ac5_hold_between_forget_and_late_sweep_preserves_late_frame:7`; shell `ac5_door_hold_release_waits_for_inflight_forget:1421`, `ac5_door_bounded_scan_admits_continuous_writer:1503` | Original-HEAD late-hold/100k scan red101; original door-release red101. Initial i64MAX plant GREEN retained honestly; corrected constituent-query oracle then made the same plant red101 at100k/1M (`l1-ac5-constituent-plant.log`). | Hold/release and late-hold pass. Final performance FAILS:100k door ratio0.6513443437, constituent1.0503247877;1M HandlerStillRunning. No retry-to-green or threshold/deadline widening. AC5/EC-13 remains open. |
| AC6 | live `ac6_running_t3_worker_uninstall_proves_live_inventory:306` | Borrows AC4 snapshot plant per ruled contract; live lineage fallback is AC6.3 because schedule admission is already20-9's dependency | Explicit missing signed T3 substrate SKIP locally. Hosted owned-CID teardown/proof leg UNVERIFIED, not a pass. |
| AC7 | inventory `ac7_archive_and_authored_skill_are_named_and_counted:224` | Original-HEAD absent archive/skill categories red101; dropped typed archive entry combined plant red100 | Pass. Real archive and full-identity authored queue fixtures; Lifecycle Journal/workspaces named honestly. |

Controls: `l1-combined-plants.diff`, `l1-combined-plants.log`, `l1-ac5-constituent-plant.log`.
All planted production sources were explicitly copy-backed, cmp-checked and touched (exit0).
No plant remains. T0 files were not replaced/deleted, including `t0-unload-race-witness.rs.txt`.
The earlier populated-baseline key mistake and live-fixture mistakes are retained in their logs,
not counted as genuine failing-before acceptance evidence.

## Exact measurements after cargo fmt --all

| Surface | Resumed exact pin | Measured / final pin | Growth | Granted ceiling |
|---|---:|---:|---:|---:|
| maos-bin |34552|34845|293|35704|
| maos-iac |8270|8373|103|8630|
| maos-audit |7721|7775|54|8261|
| maos-registry |4248|4320|72|4368|
| xtask |44466|44466|0|44466|
| aggregate |193202|193724|522|195404|

Aggregate hardfail193725. Pins/HISTORY move together; xtask first HISTORY cites live20-11
with44466 ->44466 and retains the priority hotfix. Kernel26841 physical/21042 tokei,
95 files, identical hash db39bc4a1ab6a6c22575990ae934e698a519d3efd46a8dbb38e62bc376697b59;
domain10107 unchanged. `l1-current-step-check-kernel-baseline.log` and
`l1-current-step-kloc-check.log` exit0. Earlier pre-pin kloc exits1 are retained.

## Commands and observed verification

All cargo commands inherited CARGO_TARGET_DIR; none set/override it. Single cargo slot,
except an accidentally queued control immediately cancelled before verification when the
prior helper backgrounded (no test outcome; ledger preserves that interruption).

- `env -u CI -u GITHUB_ACTIONS cargo fmt --all`: exit0, final current-step run.
- `env -u CI -u GITHUB_ACTIONS PATH=/tmp/dev2011b-8n8sp4f9/python/bin:$PATH cargo test --locked -p maos-bin --test erasure_inventory_20_11 --test erasure_live_20_11 --test erasure_scan_20_11 --test shell_host_16_2 -- --nocapture`: **exit101** (`l1-current-step-focused.log`):inventory9/9;live local1/1 plus hosted skip;late-hold1/1;shell11/13. Shell failures: existing REPL clarification timing family and new AC5 performance. Same-line rule-10 owner20-7 records in deferred-work; no exempted acceptance claim.
- `env -u CI -u GITHUB_ACTIONS cargo build --locked -p maos-bin -p maos-cli`: exit0 (`l1-affected-binaries.log`), **before** resumed package guard; not final certification.
- `env -u CI -u GITHUB_ACTIONS PATH=/tmp/dev2011b-8n8sp4f9/python/bin:$PATH cargo clippy --workspace --all-targets --locked --message-format=json`: exit0, raw `l1-clippy.jsonl` + stream log, **before** resumed package guard. New diagnostics found then were addressed, but final formal delta was not run.
- For each gate, exact form `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask <gate> --json`: **exit0** for check-kernel-baseline,kloc-check,check-service-boundary,check-dev-record-completeness,check-epic-close-coherence,check-decision-register,check-exit-commands. Logs `l1-current-step-<gate>.log`.

Full command/interruption ledger: `l1-command-ledger.json`; original before/plant logs and
all compilation/harness failures are retained. Priority park custody manifests/checksums copied
into this evidence directory before scratch cleanup.

## Changed files and reasons

- Audit proof/smoke tests and Python verifier: clean v2 cutover and signed per-category post inclusion.
- IAC transparency log: all-incarnation lineage queries, bounded stable windows, hold-safe mutation.
- Bin main/hook pump/operator door: honest inventory/category statuses, pre-unload schedules,
  off-worker blocking scans, serialized release and staged package commit ordering.
- Registry storage: validated serialized uninstall with same-filesystem staging/rollback guard.
- CLI doc: actual uninstall/package behavior; new inventory/live/scan tests and shell regression oracles;
  existing worker helper forwards real hosted substrate, does not spawn cargo.
- Story, deferred-work, ADR-044, coverage matrix, kloc pins and L1 evidence: exact measurements,
  ownership, witness and limitation records. No new maos-bin src/env var.

## Still open at operator-requested wrap-up

AC5 performance acceptance is **not met** on the final run; attribution belongs with20-7's
stable-runner lane but20-11 cannot close EC-13 on that basis. The existing REPL failure is
recorded, not rerun. Production schedule admission defaults the SCB schedule section in both
admission paths; already owned by20-9 AC3 / D-20-9-08 A, so AC6.3 uses live lineage fallback.

Final fixture builder (`cargo run -p xtask -- build-test-fixtures`), genuine post-builder
bin/CLI build, workspace nextest at default concurrency using both metadata reuse inputs,
final formal clippy delta, preservation suites13.5b/18-6, once-only prescribed prepush,
and hosted discipline/T3 legs have **not run** in this continuation's final tree.
No workspace nextest or prepush was attempted, so there is no invented pass or retry.
Hosted delegated cgroup/rootless runtime/network namespaces remain unverified locally.
AC3 halt counts/receipts, O4 and KS remain final kernel landing, untouched here.
Cross-store crash atomicity is not claimed by the in-process rollback guard.

## Requested final feature control and cleanup

`env -u CI -u GITHUB_ACTIONS PATH=/tmp/dev2011b-8n8sp4f9/python/bin:$PATH cargo test -p maos-bin --no-default-features --test verb_table_15_3`:
**exit0,7/7 passed** (`l1-final-no-default-control.log:651-663`). This was the last cargo run.
The earlier accidentally queued invocation was cancelled without a verification result;
it was not a failed test retried to green. The final control leaves a no-default-features
binary in the shared target: perform the prescribed fixture-builder followed by genuine
bin/CLI build before any integration suite or metadata-reuse workspace run.

Cleanup (`l1-cleanup.json`): `/proc` scan found no cargo/rustc. Deleted this agent's
`/tmp/dev2011b-8n8sp4f9` (including temporary Python cryptography venv) and `/tmp/2011-wip`
after preserving park custody records in evidence. Removed622 stale incremental sessions;
kept newest5929 and19 additional sessions touched within2h. No deps/build/fingerprint deletion.
**3,223,033,786 logical bytes removed**; observed filesystem free-space increase
**2,865,713,152 bytes**. Exclusive-inode allocated-byte accounting is separately recorded.
Temporary Python backend must be provisioned again for future standalone verifier tests;
no permanent dependency/host workaround was added.
