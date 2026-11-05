# Current-tree negative controls — observed, not proposed

All plants below were temporary production-source mutations. The raw command sidecars record exact commands and exits. None changes an oracle, route deadline, observation deadline, retry count, durability setting, perf baseline or tolerance.

| Contract | Real test / assertion | Observed plant result |
| --- | --- | --- |
| AC1 | `erasure_inventory_20_11::ac1_principal_free_distillate_gets_a_signed_proof:167` | Restored subject-only NotFound guard: exit 100; actual CLI exit 4 / `not_found`, expected 0. `current-plants-1.stderr:795-825`. |
| AC2 | `erasure_inventory_20_11::ac2_principal_and_principal_free_lineage_are_both_erased:393` | Restored `intent_lineage: VerifiedEmpty` constant: exit 100; populated lineage count became Null rather than 1. `current-ac2-max-plant.stderr:650-653`. |
| AC4 | `shell_host_16_2::ac4_door_snapshots_real_scb_schedules_before_unload:1794` | Dropped the actual pre-unload SCB schedule snapshot: exit 100; captured ids `[]`, expected `["daily"]`. `current-plants-1.stderr:757-760`. |
| AC5 bounded work | `erasure_scan_work::ac5_every_scan_has_bounded_sqlite_work_at_100k` and `...at_1m_rowid_with_100k_rows`, shared assertion `:112` | Literal `i64::MAX` window: both fail, exit 100; each actual first predicate consumes one window / 2,000,058 VM steps / ratio 1. `current-ac2-max-plant.stderr:687-690,722-725`. These are work controls, **not** a wall-ratio acceptance pass. |
| AC7 | `erasure_inventory_20_11::ac7_archive_and_authored_skill_are_named_and_counted:256` | Isolated omission of the populated `skill_queue` observation: exit 100 at its real counted-reason assertion. `current-ac7-plant.stderr:616-617`. |
| P5 error surface | `uninstall_review_20_11::review_p5_index_recovery_error_is_returned_not_only_logged:148` | Swallowing the actual durable-index recovery error returned `()` instead of Err: exit 100. `current-plants-1.stderr:721-722`. |

**Non-witness explicitly excluded.** The first batch's AC7 failure (`current-plants-1.stderr:860-861`) was masked by AC1's temporary subject-only guard: its legacy owner did not populate the stable subject set. It is NOT counted for AC7. The standalone AC7 rerun is a different isolated negative control, not retry-to-green. The batch's `1_000_000` page control also failed both VM cases (`:923-926,958-961`), but the later literal `i64::MAX` control above is the one matching the story's named plant.

**Source and restoration.** Exact diffs are `current-plants-1.patch`, `current-ac7-plant.patch`, and `current-ac2-max-plant.patch`. Current preimages were saved under `/tmp/dev2011b-review.9OTbuj/plant-current-{main,door,iac,storage}.rs`. The expanded restoration commands used `cp BEFORE SOURCE && cmp BEFORE SOURCE && touch SOURCE` for every affected file:

1. First batch: main, operator door, IAC, registry — one dependent command chain, exit 0.
2. Isolated AC7: main — exit 0.
3. Literal AC2/MAX: main and IAC — one dependent command chain, exit 0.

All restoration commands finished before the next compile. `cargo fmt --all` after all restorations exited 0 (`fmt-review-final.stdout.command.json`). No plant remains in production source.

**AC6 limitation.** The real local SCB/door boundary shares AC4's snapshot control. This is not a running spawned-T3 proof or plant. Container/cgroup/network-namespace capability is unavailable locally; hosted AC6 remains unverified and is not marked closed.

**Independent wall criterion.** The real door's <25% assertion remains unchanged and is now evaluated after its completion/repeat/footprint/cleanup assertions, using the original pre-repeat elapsed/max measurements. AC5/EC-13 stays open under Main's disposition until hosted proof shows both that wall bound and real 1M completion; work-control reds do not close it.
