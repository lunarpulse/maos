# 20-11 L2 — witnesses, failing-before and plants (Dev2011k, 2026-11-11)

All plants applied in the real tree, run once, restored by copy-back + `cmp` rc 0 + `touch`. Raw logs in `logs/`; final-tree green logs in `final/`.
`file:line` are the final-tree lines of the witness.

| AC | Witness (name:line) | Failing-before (observed red) | Plant → red | Final green |
|---|---|---|---|---|
| O4 / AC3.1 refusal | `halt_unload_fence_20_11::o4_late_halt_during_unload_is_refused_typed_and_never_lost` (crates/maos-kernel-core/tests) | `logs/o4-red-head-plus-variant.log` (HEAD + domain variant only: invoke succeeds, `:249` red) | (a) no close in `terminate_spirit`: `logs/o4-plant-a-no-close-in-terminate.log` red | `final/w1-kernel-o4-ks.log` |
| O4 in-flight (S2) | `…::o4_inflight_halt_is_receipted_and_the_next_is_refused` | same log, red at the "unload drained while an invocation was still committing" assertion | (a) red; (b) release `closed` right after the check: `logs/o4-plant-b-release-before-tl-write.log` red (only this test) | green |
| O4 crash (S4) | `…::o4_crash_path_refuses_a_late_halt` | same log, red | (a) red. (c) "close in unload only": no realistic variant — it needs a `pub(crate)` close method (O1 shape) the ruled design deliberately omits, and plant (a) already reds S4 | green |
| KS kernel | `schedule_forget_spirit_20_11::forget_spirit_drops_only_that_spirits_buckets_and_stamps` | compile-red (method absent at HEAD) | keep buckets: `logs/ks-plant-1-keep-rate-buckets.log` red; keep stamps: `logs/ks-plant-2-keep-stamps.log` red | green |
| KS door / AC4.1 | `erasure_live_20_11::ac2_live_door_erases_principal_free_lineage` (waits for `schedule.fire:daily`, asserts `scheduled_invocations Removed{1}`) | `logs/ac4-live-red-head.log` (HEAD: constant `CoverageGap`) | drop the `forget_spirit` call in `BinPrivateOpsImpl`: `logs/plant-ks-door-drop-forget-spirit.log` red ("watchdog state retained for daily") | `final/w7-bin-live.log` |
| AC3.2 offline | `erasure_inventory_20_11::ac3_offline_orphaned_halt_gets_one_terminal_receipt_and_isolation_holds`, `ac3_spirit_with_only_an_orphaned_halt_is_not_not_found`, `ac3_no_halts_is_verified_empty_from_the_query` | `logs/ac3-offline-red-head.log` (HEAD: exit 4 `not_found` ×2, `CoverageGap` ×1) | AC3.5 count from a fresh registry (`orphans = Vec::new()`): `logs/plant-ac3-fresh-registry-count.log` red (2 of 3) | `final/w4-bin-erasure-inventory.log` |
| AC3.1 door | `shell_host_16_2::ac3_door_snapshots_pending_halts_around_the_unload`; `main.rs` composition `uninstall_review_20_11::{ac3_ac4_door_counts_unload_receipts_and_removed_watchdog_state, ac3_door_empty_registry_is_not_a_removed_claim_and_a_missing_receipt_is_a_gap, ac4_door_surviving_watchdog_stamp_is_named_not_removed}` | compile-red at HEAD (new `DoorUninstall` API); behavioural HEAD red is the live door row above (constant gap) | skip the receipt read-back: `logs/plant-ac3-door-skip-receipt-readback.log` red | `final/w5-…`, `final/w6-…` |
| AC1.3 / D-09 halt evidence, both verifiers | `erasure_review_20_11::p3_p4_f4_f5_f6_f11_f12_both_verifiers_enforce_the_same_signed_contract` (`halt_valid`, `halt_missing_receipt`, `halt_duplicate`, `halt_wrong_kind`) | compile/kind-red at HEAD (kind absent) | drop the `pending_halts` mapping in `verify.py`: `logs/plant-verifier-drop-halt-kind.log` red (`halt_valid`, Python) | `final/w3-audit-erasure-review.log` |

Notes: the `Worker` leg of `erasure_live_20_11` skips without the signed T3 substrate (hosted-only; unverified locally).
The red logs for the offline and live rows were produced by swapping the HEAD production files (`git show HEAD:path`) into the tree,
then restoring the landing files byte-for-byte (cmp rc 0).
