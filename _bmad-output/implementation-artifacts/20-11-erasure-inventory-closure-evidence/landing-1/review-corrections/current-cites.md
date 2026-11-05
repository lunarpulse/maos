# Re-measured correction citations — 2026-11-05

The immutable baseline HEAD is `93d39cc7ca185f513dee2d97bed2058f11554d9a`; original L1 is `cbed4dad`. Coordinates below are the final formatted **working-tree source**, re-measured after the preservation repairs and restored controls, not falsely labelled as committed HEAD coordinates. Earlier T0/source measurements remain historical evidence.

| Contract / implementation | Current source |
| --- | --- |
| Whole-cascade blocking boundary and durable terminal/report | `crates/maos-bin/src/main.rs:10595`; inner cascade `:10769`; real receipt reports assembled `:10630-10635` |
| Real per-principal pump completion and typed evidence | `crates/maos-bin/src/main.rs:10935-10958` |
| Aggregate stable-subject/admission-qualified final scope; bounded eight-pass tail; real Pending/Refused(late-frame) requests | `crates/maos-bin/src/main.rs:10970-10989` |
| Real removal evidence, marker-qualified clearing of earlier malformed observations | `crates/maos-bin/src/main.rs:10991-11019` |
| Held namespace metadata attribution independent of unreadable payloads | `crates/maos-audit/src/lib.rs:2092`; cascade `crates/maos-bin/src/main.rs:11022-11039` |
| Named frozen-kernel/private/key/frame coverage and actual frame ids | `crates/maos-bin/src/main.rs:11046-11061` |
| Mandatory16 categories and exact typed removal/batch evidence | `crates/maos-audit/src/erasure/proof.rs:21-90`; proof DTO `:154-173` |
| Coupled proof builder / bundle verifier / against-log verifier / durable no-replace publication | `crates/maos-audit/src/erasure/proof.rs:356,674,714,778` |
| Trusted-root inclusion-only Merkle helper and migrated collective caller | `crates/maos-audit/src/erasure/merkle.rs:233`; `crates/maos-loom-lite/src/store.rs:1713` |
| FR4 projection classifier; AST cfg(test) boundaries and braceless production-writer witness | `crates/maos-audit/src/fr4_classifier.rs:662`; `crates/maos-audit/tests/fr4_classifier_16_2.rs:714,1103` |
| IAC FairMutex (non-poisoning), queued-writer handoff | `crates/maos-iac/src/adapter/transparency_log.rs:672,1941` |
| Predicate-only indexes and removal of old full indexes; outcome index retained | `crates/maos-iac/src/adapter/transparency_log.rs:557-570`; actual pinned readers `:1490,1988,2092` |
| Finite entry high-water and4096 actual-row windows, including sparse histories | `crates/maos-iac/src/adapter/transparency_log.rs:1863,1895-1942`; literal production cap `:1902` |
| Constant-work populated-log presence check preserving D-16-2-F | `crates/maos-iac/src/adapter/transparency_log.rs:1873` |
| Recorded Pending is NOT ordinary outstanding-request recovery | `crates/maos-iac/src/adapter/transparency_log.rs:2088-2123` |
| Prior sources are full-body tombstones, not citable partial cost/governance redactions | `crates/maos-iac/src/adapter/transparency_log.rs:1957-1975`; unchanged kernel oracle `crates/maos-kernel-core/tests/forget_cascade_18_6.rs:429` |
| Admission identity at frame row; current-pid identity; real sweep; durable certification | `crates/maos-iac/src/adapter/transparency_log/erasure_sweep.rs:106,116,123,252` |
| Complete streamed generation preimage held through finalization; explicit recovery | `crates/maos-registry/src/storage.rs:110,130,150` |
| Retained stdin for bare live shell root | `crates/maos-bin/tests/erasure_live_20_11.rs:251` |
| Real100k /1M disk door vectors | `crates/maos-bin/tests/shell_host_16_2.rs:1503,1508` |
| Recursive whole-fixture apparent/allocated footprint, not SQLite-only accounting | `crates/maos-bin/tests/shell_host_16_2.rs:1512-1534`; seed sampling `:1588`; post sampling `:1711`; explicit directory close/absence `:1747-1751` |
| Unchanged wall-ratio oracle after cleanup; measurement captured before repeat | `crates/maos-bin/tests/shell_host_16_2.rs:1754` |
| Large disk vectors share serial group, without changing default suite concurrency | `.config/nextest.toml:27-30` |
| Hash-pinned backend provisioning in both hosted jobs | `.github/actions/provision-erasure-verifier/action.yml:14`; `.github/workflows/discipline.yml:5496,6306`; `tools/verify-erasure/requirements.txt` |
| Declared PYTHONPATH crosses cgroup environment filter; real child-import witness | `scripts/run_cgroup_ceiling_17_2.py:356`; `xtask/tests/erasure_verifier_runner_20_11.rs:87-92` |

## Acceptance/control declarations

- AC1 `ac1_principal_free_distillate_gets_a_signed_proof`: `crates/maos-bin/tests/erasure_inventory_20_11.rs:151`.
- AC2 mixed principal/free lineage: same file `:387`; cross-boot, same-boot-after-exit, ambiguous numeric-owner cases `:552,557,562`; exact two-verifier evidence `:266`.
- AC4 installed schedule/package without principals: same file `:205`; failed proof retains generation `:438`; actual SCB pre-unload snapshot: `crates/maos-bin/tests/shell_host_16_2.rs:1760`.
- AC5 VM dense/sparse: `crates/maos-iac/src/adapter/erasure_scan_work.rs:23,28`; assertion `:112`; ordering `:126`; malformed recovery `:162`. Real wall/completion declarations are the shell vectors above. **AC5/EC-13 remains OPEN.**
- AC6 actual signed T3 Worker: `crates/maos-bin/tests/erasure_live_20_11.rs:308`; native live-door companion `:304`. The native unsupported-runtime return is not hosted proof.
- AC7 archive/authored skill: `crates/maos-bin/tests/erasure_inventory_20_11.rs:224`.
- P5 complete preimage / explicit index-recovery error: `crates/maos-registry/tests/uninstall_review_20_11.rs:45,106`.
- P9 held / owned malformed / own hold / mixed ambiguous malformed: `crates/maos-bin/tests/support/uninstall_review_20_11.rs:430,435,440,445`.
- Non-poisoning real-trigger schema/ordinary/kernel-append unwind witnesses: `crates/maos-iac/tests/support/erasure_races_20_11.rs:459,540,601`.

## Frozen scope and measurement

Kernel remains26841 physical/21042 tokei; domain10107. The kernel hold/private/index interval (`crates/maos-kernel-core/src/memory/mod.rs`, lines521–570) is inspected only, not repaired. AC3/KS/O4 and the separately measured/granted cross-store hold lease belong to the final kernel landing.

After `cargo fmt --all`, the exact gate-style measurement is bin34914, IAC8622, audit8174, registry4412, xtask44673, aggregate194740/hardfail194741 (`exact-source-partial-redaction.json`). The only change from the preserved preservation-checkpoint measurement is IAC-1; all other per-crate deltas are0. This does not mean every other file is byte-identical (the collective verifier caller changed without net code growth). The IAC source/test accounting transfer and unchanged test names are disclosed in `accounting-transfer-final.json`; it is not ceiling relief.
