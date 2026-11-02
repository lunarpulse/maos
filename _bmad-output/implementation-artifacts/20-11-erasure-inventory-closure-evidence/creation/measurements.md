# 20-11 creation — measurements (2026-11-02, HEAD `a8af3215`)

## Instrument

No cargo ran (parent rule). Prebuilt gate binary `/mnt/build/cargo-maos/debug/xtask` (inherited
`CARGO_TARGET_DIR=/mnt/build/cargo-maos`), invoked from each copy's root as
`env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/tmp/proto2011/<name>/xtask /mnt/build/cargo-maos/debug/xtask {check-kernel-baseline,kloc-check} --json`
on `git archive a8af3215` copies under `/tmp/proto2011/` (deleted after measurement). Each changed file was formatted
with `rustfmt 1.9.0 --edition 2021` (the repo has no `rustfmt.toml`). Format control: every touched file is
`rustfmt --check` clean at HEAD, and no prototype diff carries unrelated reformatting (contrast 20-1's `fmt0`).
Prototypes were never applied to the tree and never compiled ([INFERENCE] that they build; T0 item 1). Every `.diff`
here passes `git apply --check` against the working tree. Raw figures: `kloc-kernel-a8af3215-and-prototypes.json`.

## Baseline (untouched copy) — reproduces the pins

`check-kernel-baseline` rc 0: **26841** physical, 95 files, set hash `db39bc4a…`, changed/added/removed `[]`.
`kloc-check` rc 0: kernel **21042** tokei, aggregate **191826** (hardfail 191827), `alarm=true`, `over_budget=[]`.
Per-crate tokei touched by this story: maos-bin 33498 · maos-iac 8270 · maos-audit 7721 · maos-registry 4248 ·
maos-domain 10099 · maos-cli 6429 · xtask 44406. Every pin has zero headroom; under D02 A every added line needs a
grant.

## Kernel-touching options (all optional — every category has a kernel-free shape)

| # | Prototype (D-row) | Diff | Kernel physical | Kernel tokei | Changed kernel file |
|---|---|---|---:|---:|---|
| KH | `SpiritSchedulerAdapter::unload_receipted` returns the halt receipts `terminate_spirit` wrote; `unload` delegates (D-20-11-03 A) | `kh-unload-returns-halt-receipts.diff` | +10 (26851) | +6 (21048) | `scheduler/scheduler_loop.rs` |
| KS | `ScheduleWatchdog::forget_spirit(spirit_id, pid)` drops the Spirit's rate-limit buckets and last-fire stamps (D-20-11-04 A) | `ks-schedule-watchdog-forget-spirit.diff` | +10 (26851) | +6 (21048) | `scheduler/schedule_watchdog.rs` |
| KF | `MemoryManagerAdapter::late_forget_sweep` — the root's late sweep under `forget_serialization` with a hold re-check (D-20-11-06 B) | `kf-late-sweep-under-forget-serializer.diff` | +37 (26878) | +33 (21075) | `memory/mod.rs` |
| KH+KS | | `khks-kh-ks-bundle.diff` | +20 (26861) | +12 (21054) | both scheduler files |
| KH+KS+KF | | `kall-kh-ks-kf-bundle.diff` | +57 (26898) | +45 (21087) | three |

Every shape moves both kernel pins and the touched files' content hashes; none adds or removes a kernel file.
Recommended set (D-20-11-03 B, -04 A, -06 A) = **KS only: +10 physical / +6 tokei** (26851 / 21048).

ADR-069 (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-48`): from 21042, KS reaches 21048; with 20-1b's
pending +68 (re-based onto 20-7's landed pin, 21110) the cumulative is 21116 (recommended) … 21155 (all three) —
845…884 under the 22,000 alarm; no ENVELOPE ALARM row.

## Non-kernel prototypes (maos-iac, measured)

| # | Prototype | Diff | maos-iac tokei |
|---|---|---|---:|
| P1 | EC-13: `scan_rowid_windows` (stable-rowid windows of 4096 rows, TL lock per window); `forget_frames` tombstone + principal/kind-11 scans and `scrub_forget_frames`' kind-11 malformed scan through it; result re-sorted to the pre-EC-13 (timestamp, frame id) order (D-20-11-06 A, paging half) | `p1-iac-rowid-window-forget-scans.diff` | +30 (8300) |
| P2 | `distillates_written_by(&[(boot_nonce, pid)])` (tombstones excluded) + `scrub_uninstalled_distillate` (ADR-044 tombstone, reason `spirit-uninstall`, `distillate.redacted` marker naming the Spirit) (D-20-11-01 A) | `p2-iac-distillates-written-by-and-scrub.diff` | +60 (8330) |
| P1+P2 | | `nk-p1-p2-iac-bundle.diff` | +90 (8360) |

## Non-kernel forecasts (not prototyped — [INFERENCE] from analogues)

| Crate · component | Forecast (tokei) | Analogue |
|---|---:|---|
| maos-iac · halt-receipt read-back query for the door count (D-03 B) + per-window hold re-check in sweeps (D-06 A) | +20…+60 | P1 window helper; `is_under_legal_hold` `transparency_log.rs:2026` |
| maos-audit · offline orphan-halt enumeration over `(boot_nonce, pid)` (read-only SQL) | +40…+80 | `resolve_spirit_name` `lib.rs:1679-1767` |
| maos-audit · D-09 A per-category evidence (post-inclusion proofs of evidence frames, schema bump) in `erasure/proof.rs` | +40…+100 | `redacted_principal_frame_proofs` handling `proof.rs:298,354-367` |
| maos-bin · cascade: lineage scrub loop, orphan-halt close through `terminate_spirit` on a transient registry, schedule enumeration, installed-package step, named gap categories (D-10 A), NotFound rule; door pre/post enumeration + watchdog handle; off-worker forget (`block_in_place`/`spawn_blocking`); door release through the kernel serializer | +200…+400 | shared/private status block `main.rs:10765-10810`; door uninstall `operator_door.rs:1609-1623` |
| maos-registry · `uninstall_package` (atomic index rewrite, generation dirs removed) | +20…+40 | `install_package` `storage.rs:179-220` |
| maos-cli · doc string `cli.rs:63-66` | 0 (comment) | — |
| `tools/verify-erasure/verify.py` (D-09 A), tests, workflows, docs | uncharged | — |

## Not measured here (owed at T0; a figure above the ruled envelope, or a result contradicting a ruled option, is STOP + D-row)

1. Compilation and clippy of the ruled prototypes (`cargo check -p maos-kernel-core -p maos-iac`).
2. Whether a halt can be invoked for a pid after its SCB is `Unloaded` but before the second, unreceipted
   `drain_for_spirit` (`scheduler_loop.rs:562-574`) — if yes, that drain discards a pending halt without a receipt
   (FR53/NFR-Rel-11 class); STOP + D-row (kernel).
3. Forget-scan wall time and maximum concurrent-writer stall at HEAD on a 100k- and a 1M-frame TL (calibrates
   AC6.2's bound and the window size).
4. How an operator uninstalls an installed Spirit that the serving root refused at boot (`WasmEngineOff` in
   published builds): the door answers `spirit_not_loaded` (`operator_door.rs:1610-1611`) and the offline child needs
   the root stopped (LOCK_EX). Record the observed path; it decides AC4.4's operator instruction.
5. Whether any existing test depends on forget-receipt id order beyond (timestamp, frame id) (P1 re-sorts to it).
6. Python vs Rust verifier parity on `redacted_principal_frame_proofs` (F13) — record; D-09 A fixes both verifiers.
7. Whether a T3 Worker manifest is admitted with `[[schedule]]` and `on_schedule` in `enabled_hooks` (decides
   AC6's category; `schedule_watchdog.rs:166-176`).
