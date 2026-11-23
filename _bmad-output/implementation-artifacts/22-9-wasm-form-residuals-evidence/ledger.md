# 22-9 wasm-form-residuals: ledger (AC14)

Branch `e22-9-wasm-form-residuals-2026-11-23` (integration tree of main `e51b6877`, the 22-5 branch and the 22-6
branch). Decisions D-22-9-01…13 are PROPOSED and were taken at their recommended answers; the operator has not
confirmed them. One line per row: disposition, witness or kept boundary, evidence. "red" = the proven-red record under
`22-9-wasm-form-residuals-evidence/proven-red/`.

## The nine epic rows (21-9 AC10 / AC11)

| Row | Disposition | Witness / kept boundary | Evidence |
|---|---|---|---|
| F1 denied `author()` ends `OutOfFuel` | **KEPT, not reproduced** (D-22-9-05 A fallback). The denied path does not burn more than the accepted one; a turn needs 7-8 M of the 10 M fuel. Fuel not raised. | RELEASE-HOLDS 22-9.d; regression `ts_component_17_3b::typescript_component_declines_a_denied_author_without_running_out_of_fuel` (`--ignored`, passes at 10 M) | `proven-red/AC5.txt` (fuel sweep) |
| F2 pre-warm `console.log` dropped | **CLOSED** (D-22-9-06 A): stderr is journaled as `spirit.diagnostic` rows tagged `prewarm` at the Spirit's pid under the serving bound | `spawned_swap_20_9::{a_successors_swap_in_console_log_is_journaled_as_a_diagnostic, a_prewarm_guest_that_never_stops_writing_diagnostics_is_refused_at_its_cap}` | red `AC6.txt` |
| F3 `idle` fires once per quiescence | **KEPT** (D-22-9-07 B): documented, pinned by a test | RELEASE-HOLDS 22-9.c; `on_idle_substrate::idle_fires_once_per_quiescence_until_a_frame_arrives`; cookbook `lifecycle-hooks.md`, example README | red `AC7.txt` |
| F4 `lifecycle_session_ended` panic at shutdown | **CLOSED** (D-22-9-08 B): a trigger that never entered the guest is dropped (`session_ended_not_entered`); a session that dies inside an export keeps `Panicked` | `wasm_form_residuals_22_9::{a_trigger_after_the_session_ended_is_dropped_not_panicked, a_session_that_dies_inside_an_export_still_reports_the_panic}` (the stderr line `maos: lifecycle trigger dropped: '<id>' session ended` is not captured in-process; the witness is `HookOutcome::Fired` and no budget row) | red `AC8.txt` |
| F5 guest timers | **CLOSED** by A, with a deviation: a one-worker runtime owns the time driver (see deviations); the B fallback (`guest_timers_unsupported`) was NOT needed | `wasm_form_residuals_22_9::a_guest_timer_completes_under_t2_within_an_external_deadline` (real T2, external 30 s deadline, asserts exit 0 and stderr) | red `AC9.txt` |
| `DiscardSuccessor` journals `SwapInFired` | **CLOSED** (kernel `saga.rs`): the one exhaustive phase match serves both compensations | `hot_swap_cross_major_migration::{a_pid_unloaded_mid_swap_is_a_typed_not_loaded_refusal, a_failed_swap_in_journals_the_swap_in_phase}` | red `AC2.txt` |
| A spawned swap refuses a still-starting component | **CLOSED** (D-22-9-10 A): the probe wait draws on the 90 s attempt deadline | `spawned_swap_20_9::a_hot_swap_of_a_still_starting_component_waits_for_readiness` (6 s `hostile-slow-probe`); the retry loop in `support/installed_shell_20_9.rs` is removed; `a_successor_that_never_becomes_ready_is_refused_at_the_attempt_wall` green | red `AC10.txt` |
| `LifecycleCallers` has no unwind cleanup | **CLOSED**: `impl Drop` cancels the token; `ConsolidationWatchdog` cancels when dropped | `wasm_form_residuals_22_9::lifecycle_callers_dropped_on_a_panic_stop_their_watchdogs`; `yank_audit` AST guard green | red `AC11.txt` |
| Swap-invariant refusal has no component witness | **CLOSED** | `spawned_swap_20_9::a_spawned_successor_that_drops_a_swap_invariant_is_refused_typed_and_the_predecessor_keeps_serving` (declaration row followed by a `refused` outcome row) | red `AC12.txt` |

## The carried rows (RELEASE-HOLDS v1.0.0 rows 13-17)

| Row | Disposition | Witness / kept boundary | Evidence |
|---|---|---|---|
| 13 21-7 F6-revert | **CLOSED** (D-22-9-03 A) for the restore; the coordinator still does not enforce keep-or-narrow for native/kernel swaps (RELEASE-HOLDS 22-9.a) | `coordinator_publication::{revert_restores_the_predecessors_consent_declaration, revert_leaves_a_consent_declaration_a_later_swap_installed}` (kernel `--lib`); `maos-iac` `consent_restore_22_9` | red `AC3.txt` |
| 14 21-7 F4 | **CLOSED** (D-22-9-04 A): `[consent]` needs schema 9; recipients must be known; schema 9 PROPOSED in `abi-ratifications.toml` as `22-9-manifest-schema-9`, not self-ratified (RELEASE-HOLDS 22-9.b) | `consent_schema_floor_22_9::{a_consent_section_below_schema_9_is_refused, a_consent_section_at_schema_9_parses}`; `consent_declaration_21_7::{a_consent_recipient_the_host_does_not_know_is_refused_typed, a_recipient_installed_after_its_sender_is_admitted}` | red `AC4a.txt`, `AC4b.txt` |
| 15 21-7 F7 | **KEPT word for word** (D-22-9-02 B): RELEASE-HOLDS 22-9.e | none (no WIT change) | - |
| 16 21-7 F3 residual | **KEPT word for word**, owner not ruled (D-22-9-12 B): RELEASE-HOLDS 22-9.g | none | - |
| 17 21-9 H1 | **NARROWED** (D-22-9-11 A): boot compares the package publisher with the durable pin; mismatch refused and journaled, absent pin boots and says so; a writer who rewrites both stays outside the guarantee (RELEASE-HOLDS 22-9.f) | `wasm_form_residuals_22_9::{an_installed_package_resigned_by_another_publisher_is_refused_at_boot, an_install_without_a_pin_still_boots_and_says_so}` | red `AC13.txt` |

## 21-9 M1-M5 (D-22-9-13 A)

Out of 22-9. Each is handed to the orchestrator as a `deferred-work.md` row with its "not validated" status and an owner the operator names: M1 rollback before a running owner
exists; M2 queued firings across runtime generations; M3 `ScopeInstall` cloneable undo authority (approved I9 exemption); M4 manifest whole-file read and a UTF-8 slice panic; M5 native T2
teardown hang with no WASM route. No claim boundary exists or is added for them.

## Deviations from the D-rows and the story text

1. **D-22-9-09 A (`enable_time()`)**: `new_current_thread().enable_time()` alone is not enough. Nothing turns the time driver while the runner thread is parked in `block_on`, so a guest timer never fires
   (the T2 witness hung until its external deadline: `proven-red/AC9.txt` (2)). The shipped runtime is `new_multi_thread().worker_threads(1).enable_time()` (one worker thread owns the time
   driver; no I/O driver, so no `epoll_create1`/`eventfd2`; thread creation was already inside the T2 allow-list). `maos-wasm-host`'s tokio gains the `rt-multi-thread` and `time` features. The fallback B was not needed.
2. **AC9 `ts_component_17_3b::typescript_set_timeout_resolves` was NOT written**: the shipped TypeScript example's exports are synchronous, so a `setTimeout` cannot be awaited inside one, and whether the
   StarlingMonkey event loop turns during a sync export was not measured. The witness for guest timers is the Rust probe guest (`timer MILLIS` script op, `std::thread::sleep` = `wasi:clocks` + `poll`) under T2.
3. **D-22-9-05 A**: F1 was not reproduced, so there is no fix and no plant (AC5's "red on the unfixed tree" has nothing to fail). `proven-red/AC5.txt` holds the measurement.
4. **AC10 test location**: the readiness witness lives in `spawned_swap_20_9.rs` (its `World` fixtures), not `spawned_door_20_9.rs` (the door's retry loop in `support/installed_shell_20_9.rs` is removed; `spawned_door_20_9` passes 6/6).
5. **AC4 roster at admission**: the bin tests exercise `check_consent_recipients`/`host_knows` directly; the call sites (`admit_inner`, `spawned_successor`) have no test of their own that fails if the call is removed.
6. **Kernel pin**: `scripts/repin.py --apply` REFUSED (tracker status of `22-9-wasm-form-residuals` is `backlog`, it must be live). Measured, not pinned: `src_lines` 28,324 -> 28,339 (+15), tokei 22,143 -> 22,154 (+11),
   files `hot_swap/coordinator.rs` and `hot_swap/saga.rs`. Inside the +10...+22 forecast and the 23,000 envelope (above the 22,000 alarm, as before).
7. `nextest.toml`: `maos-bin::wasm_form_residuals_22_9` joins the serial group (real runner children and wall-clock windows).
