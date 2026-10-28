# 19-6 creation measurements (2026-10-28, HEAD `d9bd59c4`)

No cargo was run (the host's single cargo slot was held by another process). Every figure below was produced
WITHOUT touching a crate source file: the kloc instrument was replicated read-only on the tree, and every kernel
prototype was applied to a throwaway copy under `/tmp/proto19x6/<name>/` (git-tracked kernel `.rs` files copied,
the edit applied there, `rustfmt --edition 2021` (1.9.0) on the changed file, then counted). The tree was clean
before and after (`git status --short` = empty). Nothing to revert. `intent-lineage-coverage-report.md` untouched.

## Instrument replication at HEAD

| Figure | Method | Result | Pin |
|---|---|---|---|
| per-crate tokei code | tokei 14.0.0 with the argv of `xtask/src/kloc_check.rs:294-318`, grouped by `infer_crate_name` `:386-417` (`tokei-d9bd59c4.json`) | maos-kernel-core 20619 · maos-bin 30380 · maos-iac 8270 · maos-domain 9975 · maos-control 1442 · maos-cli 6317 · maos-shell 613 · maos-bench 2451 · xtask 47451 · maos-a2a-core 5037 · aggregate **189877** | `kloc.toml:264,517,406,425,…`; hardfail `:718` 189878 |
| kernel physical | `wc -l` over git-tracked `crates/maos-kernel-core/src/**/*.rs` (94 `.rs`; the gate reports 95 files) | **26378** | `kernel-core-baseline.toml:505` |
| kernel tokei on the `/tmp` base copy | same instrument, `crates/maos-kernel-core/src`, spill fault file excluded | 26378 / **20619** (instrument reproduces both pins) | — |

## Kernel-touching options, priced by throwaway prototype (in `/tmp`, never in the tree)

| Prototype | D-row | Diff | Kernel physical | Kernel tokei | Other crates |
|---|---|---|---:|---:|---|
| K1 — `CrashDetector::handle_crash` step 5 also delivers a kernel-sent `TaskComplete` to the originator via `self.iac.deliver_typed` | D-19-6-01 B | `k1-kernel-delivery-prototype.diff` | 26378 → **26405 (+27)** | 20619 → **20643 (+24)** | 0 |
| K2 — `enforce_disposition` names the resolved replica pid in the `task.reassigned` row (`_replica_pid` → `replica_pid`, `format!("pid:{replica_pid}")`) | D-19-6-05 B | `k2-replica-pid-prototype.diff` | **±0** (line-neutral) | **±0** | 0 — but `supervision/disposition.rs`'s content pin (`kernel-core-baseline.toml:972`) moves: a byte-break, FLAG-Winston re-pin |
| K3 — `ReplicaResolver::find_replica(&TaskAssignmentRecord) -> Option<ReplicaTarget>`; kernel arm uses `target.spirit_id` | D-19-6-05 C | `k3-trait-reshape-prototype.diff` | 26378 → **26377 (−1)** + content pin `:972` | 20619 → **20618 (−1)** | maos-domain 9975 → **9986 (+11)** without the in-src test edit (`supervision.rs:326-329` must change too, [INFERENCE] +3–6) — zero-headroom row `kloc.toml:425` |

Not compiled ([INFERENCE] that K1/K3 type-check: K1 uses only items in scope of `maos-kernel-core` — `maos-domain`, `maos-spirit-abi` deps `crates/maos-kernel-core/Cargo.toml:11,23`; `mint_frame_id` `transparency_log.rs:691`; `KERNEL_SENDER_SPIRIT_ID` via `crate::iac` = `pub use maos_iac::*` `iac.rs:13`).

## Measurements owed (the orchestrator runs these; one cargo process; prefix every command)

`P=env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`

1. `$P cargo run -q --locked -p xtask -- check-kernel-baseline` at `d9bd59c4` — expect `PASSED (maos-kernel-core/src = 26378 lines, 95 files, pinned 26378)`; confirms the replication above.
2. `$P cargo run -q --locked -p xtask -- kloc-check --json` — expect `passed: true`, aggregate 189877, per-crate as the table above; confirms `tokei-d9bd59c4.json`.
3. **K1 (D-19-6-01 B):** `git apply _bmad-output/implementation-artifacts/19-6-crash-originator-delivery-and-disposition-evidence/creation/k1-kernel-delivery-prototype.diff`; `$P cargo check --locked -p maos-kernel-core --lib` (expect exit 0); `check-kernel-baseline` (expect exit 1: 26405, changed `supervision/crash_detector.rs`); `kloc-check --json` (expect exit 1: kernel 20643, aggregate 189901); then `git checkout -- crates/maos-kernel-core/src/supervision/crash_detector.rs` and re-run 1–2 (expect PASS 26378 / aggregate 189877). Why: the only option that puts FR12 delivery in the kernel; its figure decides whether D-01 B is a FLAG-Winston grant of +27/+24.
4. **K2 (D-19-6-05 B):** apply `k2-replica-pid-prototype.diff`; `cargo check -p maos-kernel-core --lib`; `check-kernel-baseline` (expect exit 1 on the content hash of `supervision/disposition.rs` with the line count still 26378); `kloc-check --json` (expect unchanged 20619 / 189877); revert `disposition.rs`; re-run 1. Why: proves the line-neutral option is still a pinned-byte change.
5. **K3 (D-19-6-05 C):** apply `k3-trait-reshape-prototype.diff`, then change the in-src test `crates/maos-domain/src/supervision.rs:326-329` to the new signature; `cargo check --locked -p maos-domain -p maos-kernel-core`; `check-kernel-baseline` (expect 26377, `disposition.rs` hash); `kloc-check --json` (expect kernel 20618, maos-domain ≥ 9986 > 9975 ⇒ exit 1 on the zero-headroom row); revert both files; re-run 1–2.
6. Per-binary CI Σ of `maos-bin::epic_dispatch_19_1` from the `workspace-test-suite-junit` artifact of the latest hosted run on the 19-1 tree (`40671240805` or its successor; `xtask check-test-budget --junit <path>`), against the 30 s default (`xtask/test-budgets.toml:26`). Why: D-19-6-09 A adds three T3 vectors (~1.2 s each, 19-1 creation `ci-timings-40609213235.tsv`) to that binary.
7. Wall-clock of `nfr-rel-1-crash-detection-2s` on the latest main run (`gh run view <id> --job …`), against the job's 20-min ceiling (`discipline.yml:1590`; healthy 2m01s `:1573`). Why: D-19-6-08 A adds a mailbox drain to the same 100 kills, not new kills.

Expected figures for 3–5 are this file's `/tmp` measurements; a mismatch is a finding for the parent, not a reason to re-price here.
