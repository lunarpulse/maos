# 20-3c landing 2 — local plants (AC2, AC3) and failing-before run

Every plant is applied in place to the landing-2 tree, the named test command is run, and the file is restored by copy-back from a byte copy taken before the plant, then `cmp` (identical) and `touch` (so cargo rebuilds the restored bytes). Commands run from `/home/lunarpulse/dev_ws` as `env -u CI -u GITHUB_ACTIONS <command>` with the inherited `CARGO_TARGET_DIR=/mnt/build/cargo-maos`. Cut diffs: `plant-cuts/*.diff`, sha256 in `plant-cuts.sha256`; full logs: `plant-logs/*.log`. Seconds are the wall time of the whole cargo invocation (incremental rebuild included).

| Plant | AC | File | Exit | Seconds | Restored byte-identical |
|---|---|---|---:|---:|---|
| `P1-ac2-restore-drain` | AC2.5 | `crates/maos-kernel-core/src/halt/mod.rs` | 101 | 7.5 | yes (cmp + sha256 equal, touched) |
| `P2-ac2-precheck-global-set` | AC2.1 (precheck/swap same set) | `crates/maos-kernel-core/src/hot_swap/precheck.rs` | 101 | 8.0 | yes (cmp + sha256 equal, touched) |
| `P3-ac2-monitor-pending-set` | AC2 (post-swap monitor, resolved != lost) | `crates/maos-kernel-core/src/hot_swap/post_swap_monitor.rs` | 101 | 7.2 | yes (cmp + sha256 equal, touched) |
| `P4-ac3-no-runtime-noop` | AC3.4 | `crates/maos-kernel-core/src/inference/mod.rs` | 101 | 7.6 | yes (cmp + sha256 equal, touched) |
| `P5-ac3-discard-delivery-error` | AC3.2 second vector | `crates/maos-kernel-core/src/inference/mod.rs` | 101 | 11.8 | yes (cmp + sha256 equal, touched) |
| `P0-t0-pre-change-kernel-tree` | failing-before (AC2, AC3) | five kernel files reset to HEAD `3ea4bbb9` bytes | 101 | 12.4 | yes (cmp + sha256 equal, touched) |

## T0 — failing-before: the new vectors against the pre-change kernel

`python3` driver reset `halt/mod.rs`, `hot_swap/{precheck,coordinator,post_swap_monitor}.rs` and `inference/mod.rs` to `git show HEAD:<path>` (the `maos-capability` `RateLimited` site stays, the tests need the variant), ran the command below, then restored the working copies byte-identically (copy-back, sha256 equal, touch).

```
env -u CI -u GITHUB_ACTIONS cargo test --locked --no-fail-fast -p maos-kernel-core --test routed_halt_swap_20_3b --test halt_carry_20_3c --test hot_swap_halt_continuity_corpus_integration --test context_budget_gate_18_5 --test audit_sink_truth_16_5   # exit 101, 12.4 s
```

RED (12): `a_429_with_no_runtime_journals_the_rate_limited_frame_synchronously`, `a_429_whose_spawned_delivery_fails_is_counted_as_a_rate_limited_drop`, `another_spirits_pending_halt_changes_neither_verdict_nor_precheck`, `precheck_verdict_equals_swap_verdict_for_zero_one_and_two_carried_halts`, `a_halt_resolved_after_the_swap_writes_exactly_one_approval_row`, `a_swap_compensated_after_the_i14_gate_leaves_the_halt_pending_with_its_metadata`, `resolving_the_carried_halt_inside_the_monitor_window_is_not_a_loss`, `an_incompatible_successor_refuses_the_swap_with_the_halt_intact`, `a_carried_halt_that_leaves_the_registry_inside_the_window_is_still_a_loss`, `chain_walk_carries_every_real_pending_halt_at_every_hop_with_zero_dropped`, `swap_continuity_with_seeded_halts_carries_them_instead_of_draining`, `active_halt_keeps_identity_and_replay_context_across_hot_swap`

Green on the old tree (controls that the red ones are the L2 behaviour, not the harness): `audit_sink_truth_lifecycle_healthy_then_closed`, `every_audit_drop_site_counts_into_its_own_counter`, `revocation_preserves_the_exhausted_slot_until_teardown`, `an_exhausted_pid_mints_only_the_exits_and_refusals_are_never_charged`, `an_mcp_token_minted_before_the_crossing_is_refused_at_use`, `a_provider_429_journals_and_delivers_one_rate_limited_frame`, `halt_signal_observation_corpus_exercises_swap_continuity_wrapper`

Key red assertions at the old tree:

- `active_halt_keeps_identity_and_replay_context_across_hot_swap`: assertion `left == right` failed: FR53 preserves the ORIGINAL active halt id, source-frame/policy replay payload, owner and fire time; re-firing is not continuity
- `a_429_with_no_runtime_journals_the_rate_limited_frame_synchronously`: assertion `left == right` failed: exactly one kind-23 row, journaled in-line
- `a_429_whose_spawned_delivery_fails_is_counted_as_a_rate_limited_drop`: the failed delivery was never counted: the frame was lost silently
- `an_incompatible_successor_refuses_the_swap_with_the_halt_intact`: a successor that cannot carry the halt must refuse the swap: HotSwapResult { spirit_pid: 7, predecessor_version: "unknown", successor_version: "unknown", drained_halts: 0, migrated_halts: 0, latency_ns: 149280, schema_compat: SameMajor }

## Named plants

### P1-ac2-restore-drain

- Plant: AC2.5 — `registry.drain_for_spirit(predecessor_spirit_pid)` restored as the first statement of `validate_swap_halt_continuity`.
- Command: `env -u CI -u GITHUB_ACTIONS cargo test --locked --no-fail-fast -p maos-kernel-core --test routed_halt_swap_20_3b --test halt_carry_20_3c --test hot_swap_halt_continuity_corpus_integration` — exit 101, 7.5 s.
- Expected red: `routed_halt_swap_20_3b::active_halt_keeps_identity_and_replay_context_across_hot_swap` and the carry vectors (b)/(c)/(d)/(e) red; in `hot_swap_halt_continuity_corpus_integration` the red vectors are `swap_continuity_with_seeded_halts_carries_them_instead_of_draining` and `chain_walk_carries_every_real_pending_halt_at_every_hop_with_zero_dropped` (the wrapper `halt_signal_observation_corpus_exercises_swap_continuity_wrapper` runs on an empty registry and stays green).
- Failed tests: `precheck_verdict_equals_swap_verdict_for_zero_one_and_two_carried_halts`, `an_incompatible_successor_refuses_the_swap_with_the_halt_intact`, `a_swap_compensated_after_the_i14_gate_leaves_the_halt_pending_with_its_metadata`, `resolving_the_carried_halt_inside_the_monitor_window_is_not_a_loss`, `a_carried_halt_that_leaves_the_registry_inside_the_window_is_still_a_loss`, `a_halt_resolved_after_the_swap_writes_exactly_one_approval_row`, `swap_continuity_with_seeded_halts_carries_them_instead_of_draining`, `chain_walk_carries_every_real_pending_halt_at_every_hop_with_zero_dropped`, `active_halt_keeps_identity_and_replay_context_across_hot_swap`
- Red assertions (first lines): `1 carried, successor [1]: Ok(SafeDrained { drained_count: 0 })` | `a successor that cannot carry the halt must refuse the swap: HotSwapResult { spirit_pid: 7, predecessor_version: "unknown", successor_version: "unknown", drained_halts: 0, migrated_halts: 0, latency_n` | `swap-out refused after the I14 gate` | `assertion 'left == right' failed: a compensated swap restores the predecessor WITH its halt`
- Restored: byte-identical (cmp equal, sha256 equal, touched).

### P2-ac2-precheck-global-set

- Plant: AC2.1 — `HotSwapPrecheck::check` reads `pending_halt_ids()` (global) instead of `carried_for_spirit(pid)`.
- Command: `env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --test halt_carry_20_3c` — exit 101, 8.0 s.
- Expected red: vector (a) and (b) red: precheck and swap disagree.
- Failed tests: `another_spirits_pending_halt_changes_neither_verdict_nor_precheck`, `precheck_verdict_equals_swap_verdict_for_zero_one_and_two_carried_halts`
- Red assertions (first lines): `assertion 'left == right' failed` | `assertion 'left == right' failed: 0 carried, successor [1]`
- Restored: byte-identical (cmp equal, sha256 equal, touched).

### P3-ac2-monitor-pending-set

- Plant: AC2 (the swap carries the halt, so the monitor must not call a resolved one lost) — the post-swap monitor compares against the live pending set again.
- Command: `env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --test halt_carry_20_3c` — exit 101, 7.2 s.
- Expected red: `resolving_the_carried_halt_inside_the_monitor_window_is_not_a_loss` red: the valid swap is auto-reverted when the operator resolves the carried halt.
- Failed tests: `resolving_the_carried_halt_inside_the_monitor_window_is_not_a_loss`
- Red assertions (first lines): `assertion 'left == right' failed: a resolved halt is terminal, not lost: the valid swap must not be auto-reverted`
- Restored: byte-identical (cmp equal, sha256 equal, touched).

### P4-ac3-no-runtime-noop

- Plant: AC3.4 — the no-runtime arm of `emit_rate_limited_frame` made a no-op.
- Command: `env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --test context_budget_gate_18_5` — exit 101, 7.6 s.
- Expected red: `a_429_with_no_runtime_journals_the_rate_limited_frame_synchronously` red: zero kind-23 rows.
- Failed tests: `a_429_with_no_runtime_journals_the_rate_limited_frame_synchronously`
- Red assertions (first lines): `assertion 'left == right' failed: exactly one kind-23 row, journaled in-line`
- Restored: byte-identical (cmp equal, sha256 equal, touched).

### P5-ac3-discard-delivery-error

- Plant: AC3.2 second vector — the spawned delivery error discarded again (`let _ =`).
- Command: `env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --test context_budget_gate_18_5` — exit 101, 11.8 s.
- Expected red: `a_429_whose_spawned_delivery_fails_is_counted_as_a_rate_limited_drop` red: counter never moves.
- Failed tests: `a_429_whose_spawned_delivery_fails_is_counted_as_a_rate_limited_drop`
- Red assertions (first lines): `the failed delivery was never counted: the frame was lost silently`
- Restored: byte-identical (cmp equal, sha256 equal, touched).

## Negative control (AC1.4)

`ac1-4-negative-control-before-edit.log`: with `20-3c-…` flipped to `done` in a scratch copy of `_bmad-output/implementation-artifacts/` and the landing-1 deferred register, `check-dev-record-completeness` reds with 21 `STALE owner` violations. `ac1-1-scratch-done-after-edit.log`: after the AC1 edits, the same flip passes (rc 0). `ac1-4-negative-control-one-live-row.log`: one closed row re-opened with `Owner: \`20-3c-…\`` reds with exactly that row (rc 1).

## Notes

- `P1` was run twice: the first run (without `--no-fail-fast`) stopped after the first failing test binary and never reached `routed_halt_swap_20_3b`; the recorded row and `plant-logs/P1-ac2-restore-drain.log` are the rerun with `--no-fail-fast`, which shows the FR53 witness red. The first run's other four plants are as recorded.
- The drivers (`plant-drivers/run.py`, `t0.py`) apply the cut in place, run the command, restore from a pre-plant byte copy, and `cmp` + sha256 the result; `git apply --check` of every cut against the final tree passes, and `sha256sum -c plant-cuts.sha256` is OK.
- The AC2 plant named in the story (restore `drain_for_spirit` in `validate_swap_halt_continuity`) is `P1`; the AC3 plant (no-op the no-runtime arm) is `P4`. `P2`, `P3`, `P5` are extra controls for contracts the story's AC text implies (precheck and swap judge one set; carrying must not make an operator resolution a "loss"; a failed delivery is counted).
