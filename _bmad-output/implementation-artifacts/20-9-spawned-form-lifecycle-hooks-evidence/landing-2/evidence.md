# 20-9 landing 2 (L2, kernel) — evidence

Dev `Dev209K` (anthropic/claude-sonnet-5-5), 2026-11-12, live base `e1a44cd5` (kernel 27,044 physical / 21,202 tokei), uncommitted.
Scope (operator rulings 2026-11-12, relayed by Main): row R as priced (AC7 `poll_once`, AC8 D07 A); options (a) typed `NotLoaded`
refusal, (b) atomic archive record, (c) idle-watchdog parallel fire (`JoinSet`); the kernel-free bin halves of (a) (the marker that
stops the phantom `Completed`) and (b) (provisional record before start). AC7.2 "no sleep" = a bounded `yield_now` retry loop asserting
only on the outcome sequence. AC9.1 (`docs/invariants/I14.md`) skipped by ruling; AC6 stays hosted. Pricing and plan: `pricing-plan.md`,
`protos/*.diff` (measured in `git archive` copies before the cargo handoff).

## Witness table

Every "red before" ran against the HEAD source of the touched file(s) (kernel: the unchanged `schedule_watchdog.rs` / `coordinator.rs` /
`idle_watchdog.rs` / `archive.rs`; bin: HEAD `spirit_session.rs`, `spawned_lifecycle.rs`, `admission.rs`), with raw logs. Every plant was
applied by `/tmp` script, the test run, then copy-back + `cmp` + `touch` (the log ends `restored: cmp ok, touched`).

| Vector | Red before (HEAD) | Green after | Plant (red) |
|---|---|---|---|
| AC7 `schedule_17_9_refused_fire_mints_nothing` (`[Fired]`, `[RateRefused]`), `schedule_poll_once_reports_a_revoked_principal` (`[PrincipalRevoked]`, 0 fires), `schedule_poll_once_reports_a_suppressed_fire` (`[Suppressed]` then `[RateRefused]`) | `failing-before-poll-once-compile.log` (E0432/E0599: no `poll_once`, no `PollOutcome`) | `green-kernel-focused.log` (47 passed), `green-kernel-fr26-fixture_replay.log` (17 passed, the hosted job's shape) | `plants/plant-ac7-rate-refused.log` (rate-refused arm pushes no outcome: 2 reds) |
| AC8 `schedule_17_9_exhausted_quota_suppresses_the_fire` (inverted FW-10 vector), `schedule_d07a_partial_mint_is_revoked_and_the_fire_suppressed` (new: 2 scopes, the second refused by policy: one `Revoke` per minted token, no live token, no fire row), `schedule_d07a_declared_scopes_without_a_registry_suppress_the_fire` (new) | `failing-before-fr26-vectors.log`: 3 red with real assertions (`left: 1 / 7, right: 0`); the scope-free control `schedule_d07a_scope_free_entries_still_fire_with_the_zero_token` green at HEAD and after | same | `plants/plant-ac8-fall-through.log` (suppression skipped: 3 reds), `plants/plant-ac8-no-revoke.log` (partial mint not revoked: 1 red) |
| FR4 doorbell for `schedule.suppressed:<id>` (`every_writer_site_demanding_a_disposition_has_one`, plus the new `schedule suppressed` classifier vector) | by construction of the doorbell: the new tokenless kind-7 writer has no entry until the `WriterShapeEntry` lands | `green-iac-audit.log`, `green-audit-erasure-venv.log` | `plants/plant-fr4-doorbell.log` (entry's site renamed: red) |
| (a1) `hot_swap_cross_major_migration::a_pid_unloaded_mid_swap_is_a_typed_not_loaded_refusal` | `failing-before-a-c.log`: panic `coordinator.rs:359:22 successor SCB just inserted` | green | `plants/plant-f2-coordinator.log` (typed refusal replaced by `Internal`: red) |
| (c) `on_idle_substrate::idle_watchdog_fires_due_spirits_side_by_side` | `failing-before-a-c.log`: FAIL after 2.04 s (sequential: the waiter times out before the peer fires) | green | `plants/plant-idle-sequential.log` |
| (b) `spirit_archive_atomic_20_9::{archive_overwrite_never_mutates_the_previous_inode, archive_write_survives_a_stale_temp_file_and_leaves_none}` | `failing-before-b-archive.log` (both red) | green | `plants/plant-f3-archive.log` (write in place: both red) |
| (a, bin) `spawned_swap_20_9::a_predecessor_that_exits_after_its_snapshot_still_completes_its_swap` (twin mode `snapshot-then-exit`: answers `snapshot` with counter 7 and exits 0; the swap must complete at the same pid and the successor continue from 7) | `failing-before-bin-a-b.log`: HEAD journals `spirit.upgrade ... outcome: completed` then `lifecycle.unload` (the phantom `Completed`; the pump logs `lifecycle_session_ended`), no report | `green-bin-spawned.log` (44 passed), `green-bin-focused-final.log` (80 passed) | `plants/plant-f2-bin-marker.log` (`swap_in_flight` always false: red) |
| (b, bin) `spawned_swap_20_9::a_cold_swap_successors_fr11_record_precedes_its_start_and_a_failed_start_withdraws_it` (slow-ready component, 3 s budget: the record exists inside the start window, and is gone after the refusal) | `failing-before-bin-a-b.log`: red at `the record was written before the component answered` | green | `plants/plant-f3-bin-no-provisional.log`, `plants/plant-f3-bin-no-withdraw.log` |

Also green (no change in behavior, run as the focused set): `schedule_forget_spirit_20_11`, `halt_carry_20_3c`, `lifecycle_all_hooks_hot_swap_17_4`,
`routed_halt_swap_20_3b`, `cargo test -p maos-kernel-core --lib` (253 passed, `green-kernel-lib.log`), `-p maos-iac` and `-p maos-audit`
(`green-iac-audit.log`: 477 passed, 1 environment failure, see below), the bin `wasm_form_admission_17_3b` and `hook_pump_17_4`, and the xtask pin tests
`kernel_pin_content_hash_16_0`, `recovery_lane_ceiling_rule`, `d11_xtask_ceiling_ratchet`, `kernel_pin_ci_wiring_20_3a` (`green-xtask-pin-tests.log`, 49 passed).

**Environment failure, not this change:** `maos-audit::erasure_review_20_11::p3_p4_f4_f5_f6_f11_f12_both_verifiers_enforce_the_same_signed_contract`
needs the Python `cryptography` package (`ModuleNotFoundError`); with a scratch venv from `tools/verify-erasure/requirements.txt` on PATH the whole
`erasure_review_20_11` binary passes (11 of 11, `green-audit-erasure-venv.log`), the same recipe 20-13 L2 recorded.

## Measurements (exact, after `cargo fmt --all`)

| Crate | Before | After | Delta | Note |
|---|---:|---:|---:|---|
| maos-kernel-core tokei | 21,202 | **21,256** | +54 | priced +47; +7 = revoke-failure diagnostic |
| maos-kernel-core physical | 27,044 | **27,103** | +59 | priced +51 |
| maos-iac | 8,831 | 8,875 | +44 | priced +39; +5 = row-serialize diagnostic |
| maos-audit | 8,308 | 8,320 | +12 | FR4 `WriterShapeEntry` (prototype gap) |
| maos-bin | 38,564 | 38,602 | +38 | bin halves of a and b |
| maos-wasm-host | 3,141 | 3,149 | +8 | test twin mode |
| xtask / domain / capability / cli | 45,355 / 10,274 / 1,454 / 6,612 | same | 0 | |
| aggregate | 202,978 | 203,134 | +156 | `_aggregate_hardfail` 203,135 |

Surface (`check-service-boundary`): passed, 0 violations; the 302 item signature hashes equal the base's (verified on the pre-handoff copy for the
same kernel diff, and the gate passes against the unchanged pin on the live tree). No hash refresh, no removal.

## Verification commands (all `env -u CI -u GITHUB_ACTIONS`, `target` symlink, one cargo at a time)

- `cargo run -q --locked -p xtask -- build-test-fixtures` (the twin changed): `build-test-fixtures.log`, rc 0 (a first run failed on a type error in the new twin mode, fixed, rerun).
- Kernel: `cargo nextest run --locked -p maos-kernel-core --test schedule_watchdog_fr26 --test schedule_forget_spirit_20_11 --test on_idle_substrate --test halt_carry_20_3c --test hot_swap_cross_major_migration --test lifecycle_all_hooks_hot_swap_17_4 --test routed_halt_swap_20_3b --test spirit_archive_atomic_20_9` rc 0; `cargo test --locked -p maos-kernel-core --lib` rc 0.
- Bin: `cargo nextest run --locked -p maos-bin --features maos-bin/wasm-host --test spawned_swap_20_9 --test spawned_lifecycle_20_9 --test spawned_door_20_9 --test wasm_form_admission_17_3b --test hook_pump_17_4` rc 0 (80 passed, 3 slow vectors).
- Clippy: `cargo clippy --workspace --all-targets --locked --message-format=json` rc 0, `xtask clippy-ratchet --input` rc 0 (`gates/clippy-ratchet.json`: 1332 occurrences / 870 distinct source-distinct); `cargo clippy -p maos-bin --features maos-bin/wasm-host --all-targets` rc 0 with no new lint in the touched files (a `report` unused variable in `spawned_lifecycle_20_9.rs:57` and `maos-registry` lints predate this landing). The hosted lints (`result_large_err`, `collapsible_if`, `too_many_arguments`, `unused_mut`, `trim_split_whitespace`) did not fire on the changed code.
- Gates: `gates/summary.txt` and the per-gate `.json` / `.err`.

## Not done / unverified

- The fsync and rename order of `write_atomic` is not observed (no `strace` on this host); the hard-link vector proves replace-not-rewrite only.
- The full workspace suite is not run (accepted process); hosted CI is authoritative. AC6 and every hosted-only leg stay hosted.
- The `snapshot-then-exit` twin witness proves the exit-after-snapshot path through the real door and session; a real component that traps mid-swap is not separately witnessed.
- Deferred-work closings carry no commit sha (Main stamps it).
