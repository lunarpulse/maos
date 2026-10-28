# 19-5 creation measurements (2026-10-28, HEAD `61af7461`)

No cargo was run (the parent's build holds the host). Every figure below was produced WITHOUT touching a crate
source file: the kloc instrument was replicated read-only, and every prototype was applied to a throwaway copy
under `/tmp/proto19x5/<name>/` built from `git archive 61af7461 crates/maos-kernel-core/src` (kernel) or
`git archive 61af7461 crates/maos-bin/src` (maos-bin), the edit applied there, `rustfmt --edition 2021` (1.9.0; the
repo has no `rustfmt.toml`; verified a no-op on an unchanged `upgrade.rs`) run on the changed files, then counted.
Every `.diff` in this directory passes `git apply --check` against the working tree (none of the files it touches
carries an uncommitted 19-3/19-6 edit). Nothing was applied to the tree; nothing to revert;
`intent-lineage-coverage-report.md` untouched.

## Instrument replication

| Figure | Method | HEAD `61af7461` | Working tree (19-3/19-6 uncommitted) | Pin |
|---|---|---|---|---|
| kernel physical | `wc -l` over git-tracked `crates/maos-kernel-core/src/**/*.rs` (94 files; the gate reports 95) | **26378** | **26389** (+11 = 19-3 K2: `runtime.rs` +3, `t3/argv.rs` +5, `t3/spawn.rs` +3) | `xtask/kernel-core-baseline.toml:505` = 26378 |
| kernel tokei | tokei 14.0.0, `crates/maos-kernel-core/src`, `spill_test_faults.rs` excluded | **20619** | **20628** (+9) | `xtask/kloc.toml:264` = 20619 |
| per-crate tokei | argv `xtask/src/kloc_check.rs:294-318`, grouping `infer_crate_name` `:386-417` (`tokei-61af7461-and-tree.json`) | maos-bin 30380 · xtask 47451 · maos-bench 2451 · maos-control 1442 · maos-cli 6317 · maos-shell 613 · maos-domain 9975 · maos-wasm-host 2412 · aggregate **189877** | maos-bin **30847** (+467) · xtask 47454 (+3) · kernel 20628 · others = HEAD · aggregate **190356** | `kloc.toml:517` 30380, `:718` hardfail 189878 |

## Kernel-touching options, priced by throwaway prototype (on the HEAD copy; identical Δ on the tree copy)

| # | Prototype | D-row | Diff | Kernel physical | Kernel tokei | Also moves |
|---|---|---|---|---:|---:|---|
| K1 | delete the kernel `UpgradePolicy::ColdSwap` arm (`upgrade.rs:162-210`, its raw spirits-map writes), the `ColdSwap` variant, its `as_str`/`FromStr` arms and `UpgradeError::Lifecycle`; drop the unused `FrameFilter` import and the `(predecessor_scb, predecessor_state)` tuple legs | D-19-5-01 A | `k1-delete-kernel-coldswap.diff` | 26378 → **26321 (−57)** | 20619 → **20566 (−53)** | content pin `lifecycle/upgrade.rs` (`kernel-core-baseline.toml:920`); surface hashes `UpgradePolicy`, `UpgradeError` (`kernel-surface-v0.1-beta.json:160-173`) |
| K2 | keep the arm, but load the successor through `scheduler.load_obj` + an injected `SuccessorAdmitter` (new pub trait, `with_admitter`, `ColdSwapWithoutAdmitter`), rollback by `unload` + `load_obj` of the predecessor's object | D-19-5-01 C | `k2-kernel-coldswap-admitter.diff` | **+21** (26399) | **+19** (20638) | pins `:919`, `:920`; surface + a new class row (`kernel-api-classes.toml:583-587` family) |
| K3 | FR11 gate in the kernel: `SpiritArchive::refuse_without_migrator(&self, spirit_id, &SpiritManifestBundle) -> Result<(), HotSwapError>` — latest archived record ≠ successor version, `detect_compat` on `[hot_swap].state_schema_version`, `[migrates_from]` via `matches_version_pattern` | D-19-5-05 B | `k3-kernel-archive-gate.diff` | **+53** (26431) | **+48** (20667) | pin `hot_swap/archive.rs` `:899`; surface (new pub fn on `SpiritArchive`) |
| K4 | K3 + `SpiritSchedulerAdapter` holds the archive (`set_predecessor_archive`) and `load_obj` refuses | D-19-5-05 C | `k4-scheduler-load-gate.diff` | **+73** (26451) | **+66** (20685) | pins `:899`, `:948` (`scheduler_loop.rs` — also 19-2's FR51 prototype file) |
| K5 | coordinator writes the predecessor archive (identity record + the real state blob) at hot-swap commit, after step 10 | D-19-5-03 B | `k5-coordinator-archive-writer.diff` | **+16** (26394) | **+13** (20632) | pin `hot_swap/coordinator.rs` `:900` |
| K6 | extract ABI-triple legs 1–2 of `admit_spirit` (`security/mod.rs:283-332`) into `pub fn check_abi_triple(spirit_id, &ClassSection)`; `admit_spirit` calls it | D-19-5-09 A | `k6-abi-triple-extract.diff` | **+8** (26386) | **+4** (20623) | pin `security/mod.rs` `:954`; surface + class row |
| K7 | `pub fn load_bundle_from_str` split out of `load_bundle_from_file` (one parser for upgrade and admission) | D-19-5-06 C | `k7-shared-bundle-parser.diff` | **+10** (26388) | **+6** (20625) | pins `:919`, `:920`; still REQUIRES `[scheduling]`/`[lifecycle]` (needs K8 to serve admission) |
| K8 | `load_bundle_from_file` defaults a missing `[scheduling]`/`[lifecycle]` (as admission does, `maos-bin/src/admission.rs:565-574`) | D-19-5-07 B | `k8-upgrade-target-section-defaults.diff` | **+8** (26386) | **+6** (20625) | pin `:920` |
| **R** | **recommended rows: K1 + K6 + K8** | D-01 A, D-07 B, D-09 A | `krec-recommended-bundle-k1-k6-k8.diff` | **26337 (−41)**; tree 26389 → **26348** | **20576 (−43)**; tree 20628 → **20585** | pins `:920`, `:954`; surface `UpgradePolicy`, `UpgradeError`, + `check_abi_triple` |
| R′ | K1 + K3 + K6 + K8 (D-05 B instead of A) | D-05 B | `kalt` (not kept; = krec + k3) | 26390 (+12); tree 26401 | 20624 (+5); tree 20633 | as R + `:899` |

E19 kernel endpoint (`epics/epic-19-founder-loop-w4.md:50` (a) + D-19-2-20 B): ≤ 26483 physical / ≤ 20686 tokei = tree
26389 / 20628 (19-3 applied) + 19-2's capped ≤ +93 / +58 ⇒ **19-5 has 0 lines of kernel room**. Every positive option
(K2–K8 alone) needs its own FLAG-Winston grant; R is net negative (frees 43 tokei of ADR-069 room,
`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`: 20585 is 1,415 under the 22,000 alarm). Not compiled
([INFERENCE] K1/K6/K8 type-check: K1 deletes the only users of the removed items inside the kernel lib — kernel
tests and the maos-bin smoke arm are cut over by the story; K6/K8 use only items already in scope).

## maos-bin components, priced by prototype (tokei code per file, HEAD copy)

| Prototype | D-rows | Diff | File Δ |
|---|---|---|---:|
| `GatedManifest` parses `[hot_swap]`, `[migrates_from]`, `[halt_protocol_compatibility]`; `bundle()` carries them | D-06 A | `mb-admission-d03a-d04a-d05a-d06a-prototype.diff` (part) | +27 |
| FR11 gate `refuse_without_migrator` before Gate 8 + `AdmissionRefusal::MigratorMissing` (`migrator_missing`, Display names `EMigratorMissing`) + write-if-absent admission record `archive_admitted` | D-03 A, D-04 A, D-05 A | same diff (rest) | +76 |
| — same with the gate in the kernel (K3), maos-bin only maps the error | D-05 B | `mb-admission-d05b-variant-prototype.diff` | +45 (admission.rs total +72) |
| door cold swap: refusal deleted, `cold_swap` (unload → `load_spirit` + `start`, restore on refusal, `spirit.upgrade` TL row, journal `Upgrade`), `admit_and_start`, `BinPrivateOps::admitted_manifest` | D-01 A, D-02 A | `mb-door-cold-swap-d01a-d02a-prototype.diff` | +75 |

Not prototyped ([INFERENCE], analogues in the story's kloc table): `admitted_manifest` impl + the path map filled at
the three admission sites in `main.rs` (+10–18); the smoke arm's cold-swap step `main.rs:7546-7562` HEAD deleted
(**−16 counted**); door migrator mapping (+3); ABI pre-gate call (+5–8); typed `[cli_wrapper]` upgrade refusal (+6–10).

## Measurements owed (the parent runs these; one cargo process; prefix every command)

`P="env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo"`; gates as `$P cargo run -q --locked -p xtask -- <gate>`.
D = `_bmad-output/implementation-artifacts/19-5-cross-form-upgrade-and-cold-swap-evidence/creation`.

1. `check-kernel-baseline` — at a clean `61af7461` expect `PASSED (26378 lines, 95 files)`; on the current tree expect a FAIL at 26389 (19-3's uncommitted K2) until 19-3 re-pins. Confirms the replication.
2. `kloc-check --json` — at `61af7461` expect aggregate 189877, maos-bin 30380, kernel 20619; on the tree expect maos-bin 30847, kernel 20628, aggregate 190356 (exit 1 while 19-3/19-6 are un-pinned).
3. **R (recommended):** `git apply $D/krec-recommended-bundle-k1-k6-k8.diff`; `$P cargo check --locked -p maos-kernel-core --lib` (expect rc 0); `check-kernel-baseline` (expect 26348 on the tree / 26337 at HEAD, changed `lifecycle/upgrade.rs`, `security/mod.rs`); `kloc-check --json` (kernel 20585 tree / 20576 HEAD); `check-service-boundary` (expect it to name `UpgradePolicy`, `UpgradeError` hash changes and the unclassified `check_abi_triple`); then `git checkout -- crates/maos-kernel-core/src/lifecycle/upgrade.rs crates/maos-kernel-core/src/security/mod.rs` and re-run 1–2. `cargo check -p maos-bin` is EXPECTED to fail (smoke arm `main.rs:7551` names `ColdSwap`) — that is the cutover the story does, not a pricing error.
4. **K1, K2, K3, K4, K5, K6, K7, K8** one at a time, same procedure (`git apply $D/<k>.diff` → `cargo check -p maos-kernel-core --lib` → both gates → `git checkout --` the touched kernel files → gates re-PASS). Expected figures: the table above (HEAD; add +11/+9 on the tree).
5. Hosted wall-clock of job `one-daemon-one-door` (`discipline.yml:6063` HEAD; 30-min ceiling) on the latest green main run — D-19-5-14 adds 4–6 daemon roots to its `--test-threads=1` binary.
6. Hosted wall-clock of job `upgrade-policy-corpus` (`:1807`) — K1 cuts `upgrade_orchestrator_three_policies.rs` to two policies; expect it to shrink.

Expected figures are this file's `/tmp` measurements; a mismatch is a finding for the parent, not a reason to re-price here.
