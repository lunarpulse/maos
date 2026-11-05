# Restored controls and final preservation — 2026-11-05

Baseline HEAD: `93d39cc7ca185f513dee2d97bed2058f11554d9a`. The first table records controls repeated at the formatted post-preservation checkpoint. The later one-line partial-cost predicate repair has its own exact final-source control below. Earlier raw results remain historical evidence; no failed or masked control is relabelled green.

Each command has its exact argv, environment, duration and exit in the adjacent `*.stdout.command.json`. A nextest exit100 here means the deliberately planted assertion failed, not a successful production run.

| AC/item | Current witness declaration (path relative to repository) | Deliberate regression and observed red | Raw result |
| --- | --- | --- | --- |
| AC1 | `crates/maos-bin/tests/erasure_inventory_20_11.rs:151` — `ac1_principal_free_distillate_gets_a_signed_proof` | Restore the principal-only empty guard. Real principal-free lineage is misreported NotFound/4 instead of erased/0; assertion at167. | `final-plants-1.stderr:747-792`, exit100 |
| AC2 | `crates/maos-bin/tests/erasure_inventory_20_11.rs:387` — `ac2_principal_and_principal_free_lineage_are_both_erased` | Replace the queried lineage status with constant VerifiedEmpty. Authenticated inventory has Null rather than Removed count1; assertion at393. | `final-ac2-plant.stderr:591-609`, exit100 |
| AC4 | `crates/maos-bin/tests/shell_host_16_2.rs:1760` — `ac4_door_snapshots_real_scb_schedules_before_unload` | Remove the real pre-unload SCB schedule snapshot. Observed [] rather than ["daily"]; assertion at1794. | `final-plants-1.stderr:812-830`, exit100 |
| AC5 bounded work, dense | `crates/maos-iac/src/adapter/erasure_scan_work.rs:23` — `ac5_every_scan_has_bounded_sqlite_work_at_100k` | Set the actual production row-window constant to literal `i64::MAX`, not merely a large finite approximation. One window consumes all scan work, ratio1, violating the unchanged quarter-work oracle at112. | `final-plants-1.stderr:850-867`, exit100 |
| AC5 bounded work, sparse | `crates/maos-iac/src/adapter/erasure_scan_work.rs:28` — `ac5_every_scan_has_bounded_sqlite_work_at_1m_rowid_with_100k_rows` | The same literal MAX plant consumes all work at one million rowid high-water with100k physical rows. This is a sparse VM vector, not a substitute for the real1M disk completion fixture. | `final-plants-1.stderr:885-902`, exit100 |
| AC7 | `crates/maos-bin/tests/erasure_inventory_20_11.rs:224` — `ac7_archive_and_authored_skill_are_named_and_counted` | Independently omit the real populated skill_queue observation. The signed inventory fails its count assertion at256. No temporary AC1 guard is active in this isolated run. | `final-ac7-plant.stderr:601-617`, exit100 |
| P5 recovery | `crates/maos-registry/tests/uninstall_review_20_11.rs:106` — `review_p5_index_recovery_error_is_returned_not_only_logged` | Swallow the real index recovery error. Caller gets () where the existing oracle requires Err; assertion at148. | `final-plants-1.stderr:711-727`, exit100 |

The combined batch is five genuine failures (`final-plants-1.stderr:921-927`); the isolated AC7 and AC2 commands each run one test and fail it. The exact diffs are `final-plants-1.patch`, `final-ac7-plant.patch`, and `final-ac2-plant.patch`.

All production files were restored with copy-back + `cmp` + `touch`, observed exit0, after each control. The saved final preimages were `/tmp/dev2011b-review.9OTbuj/final-plant-main.rs`, `final-plant-door.rs`, `final-plant-iac.rs`, and `final-plant-storage.rs`; no plant or profiling hook remains. The earlier contaminated combined AC7 failure remains explicitly excluded in `current-plant-results.md`; only isolated AC7 evidence counts.

## Positive source and scope of the claim

The post-preservation checkpoint passed333/333 selected cases,49 skipped, exit0. The subsequent final partial-cost repair passes346/346,49 skipped (`partial-redaction-repair-final.{stdout,stderr}`), including all16 inventory cases, all26 original13.5b cases, all114 IAC cases and all13 unchanged kernel cascade cases. The SCB snapshot case also passes the actual standalone5688-case run, whose two failures are separately recorded rather than hidden. The registry's two real recovery cases passed in the118-case recovery/IAC/Python-runner focus. These targeted positives do not make the standalone or the1M completion criterion green.

`before-state.md` retains genuine baseline/preservation reds, proof/verifier/identity/atomicity controls and excluded setup failures. Fourteen affected structural gates pass; kernel-baseline, exact KLOC and service-boundary are refreshed after the final predicate repair (`gate-post-partial-*.json`, exit0). The initial record-format gate failure and its actual repair remain recorded.

AC5/EC-13 remains OPEN: VM/constituent boundedness is demonstrated, but the unchanged <25% wall ratio and real1M in-flight completion require hosted stable-runner evidence. The real100k/1M1KiB disk/WAL vectors remain at `shell_host_16_2.rs:1503,1508`; the latter is not replaced by the sparse VM fixture.

AC6's actual signed T3 Worker leg (`erasure_live_20_11.rs:308`) is unverified locally. The native suite's named unsupported-runtime early return is not counted as hosted proof; the CI=true attempt without MAOS_T3_RUNTIME failed its real precondition, not a feature oracle. No fake Worker is substituted.

AC3/KS/O4 and the private/index hold lease are excluded final-kernel work.

## Final exact partial-cost control

The unchanged `crates/maos-kernel-core/tests/forget_cascade_18_6.rs:429` case failed at447 in the one required standalone workspace (`workspace-final-default.stderr:29810-29839`): an unrelated forget erased a neutral derivative of a citable partial cost redaction. The IAC prior-source SQL's broad kind28/29 OR was the cause. Removing that OR, not weakening the oracle, yields346/346 focused passes. No kernel source or test changed.

Reintroducing the exact OR independently fails that same assertion (`partial-redaction-plant.stderr:476-495`, exit100); the exact diff is `partial-redaction-plant.patch`. Restoration command, observed exit0:

```sh
cp /tmp/dev2011b-review.9OTbuj/partial-redaction-after.rs crates/maos-iac/src/adapter/transparency_log.rs &&
cmp /tmp/dev2011b-review.9OTbuj/partial-redaction-after.rs crates/maos-iac/src/adapter/transparency_log.rs &&
touch crates/maos-iac/src/adapter/transparency_log.rs
```

The complete standalone result remains5688 run/5686 pass/2 fail/138 skipped, exit100. Its other failure is the unchanged1M completion-observation deadline, retained under20-7's same-line timing/I/O disposition with AC5/EC-13 open under20-11. The100k vector passes ratio0.1164494388 and observes recursive allocated peak152236032 bytes plus explicit close/absence assertions. The1M vector observes seed peak1487568896 bytes; it does not reach the post-completion footprint/close assertions. No full1M cleanup/completion claim is inferred from its seed measurement.
