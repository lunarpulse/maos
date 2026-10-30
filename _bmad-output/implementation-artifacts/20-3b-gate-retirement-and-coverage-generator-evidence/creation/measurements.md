# 20-3b creation measurements (2026-10-30, HEAD `23f58da4`)

`23f58da4` = `3fd7f8b4` + the preflight report only (`git diff --stat 3fd7f8b4 HEAD` = one file,
`epic-20-preflight-2026-10-30.md`, +347); every code figure below therefore equals the preflight's `3fd7f8b4` figure
unless stated. No cargo ran (parent rule: this checkout inherits `CARGO_TARGET_DIR`; no build, no test). No crate
source in the tree was modified: every prototype was made on a `git archive 23f58da4` copy under
`/tmp/proto203b/<name>/`, `rustfmt --edition 2021` (1.9.0-stable) run on every changed file, then measured with the
prebuilt gate binary run **from that copy's root**:
`env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/tmp/proto203b/<name>/xtask /tmp/xtask-maos-w3 {check-kernel-baseline,kloc-check} --json`
(`check-kernel-baseline` resolves the root from cwd ancestors, `xtask/src/check_kernel_baseline.rs:462-466`;
`kloc-check` from its config path, `xtask/src/kloc_check.rs:269-273`). Every `.diff` here passes `git apply --check`
against the working tree. Nothing to revert; `intent-lineage-coverage-report.md` untouched by creation.

## Instrument replication

| Figure | Method | In-tree `/home/lunarpulse/dev_ws` | `/tmp` copy `base` | Pin |
|---|---|---|---|---|
| Kernel physical | `check-kernel-baseline --json` | **26409**, 95 files, set hash `66dc2e24…`, changed/added/removed `[]`, rc 0 | 26409, identical | `xtask/kernel-core-baseline.toml:505` |
| Kernel tokei | `kloc-check --json` | **20618** | 20618 | `xtask/kloc.toml:264` |
| Per-crate tokei | `kloc-check --json` (`passed=true`, `alarm=true`, `over_budget=[]`, rc 0) | xtask **47682** · maos-bin **32939** · maos-cli **6371** · maos-audit **7660** · maos-domain **9975** · maos-control 1496 · aggregate **192778** | identical | `:339`, `:517`, `:489` (6856 ceiling), `:409`, `:425`, `:484`; `_aggregate_hardfail` `:718` = 192779 |
| Retirement denominator | `wc -l` + `tokei --types Rust --output json` (14.0.0) on the ten files | 6402 physical / **5499** tokei (per file in the story) | — | = preflight §1.4 |
| Coverage matrix | `wc -l`; row regex `^  [^ #][^:]*:$` | **2051** lines, **198** rows, `mode: warning`, `delivered_phase: v0.1-alpha` (`tests/coverage-matrix.yaml:1-3`) | — | — |
| Coverage violations | **replica, not the gate** (`coverage-matrix` is not in this run's permitted gate list): `coverage-matrix-replica.py` re-implements `xtask/src/coverage_matrix.rs:93-160` (unknown gate, unknown corpus, phase outside `phase_order`, delivered row with zero coverage) | **33** = 27 unknown-gate refs in 21 rows + 3 unknown corpora + 3 out-of-order phases (`NFR-Rel-7`/`NFR-Scale-2` `v2.2`, `NFR-Test-8` `v2.0`) + 0 delivered-zero | — | 15-3's "33" (epic AC5) reproduced by replica |
| Ship-gate rows | `grep -c '^\[\[ship_gate\]\]' xtask/gate-registry.toml` | **46** | — | — |
| `aggregate.needs` | entries `^      - ` in `discipline.yml:6394-6535` (`aggregate:` `:6391`, steps `:6536`) | **133** (epic: 123 at `:3510+`) | — | — |

## Kernel-touching options, priced by throwaway prototype (D03 / D-20-3b-05)

| # | Prototype | Diff | Kernel physical | Kernel tokei | Also moves |
|---|---|---|---:|---:|---|
| K1 | Principal values always durable: `needs_spill = matches!(namespace, Principal{..}) \|\| should_spill_to_disk(..)` (`memory/private.rs:644`) | `k1-principal-durability.diff` | 26409 → **26410 (+1)** | 20618 → **20619 (+1)** | content pin `memory/private.rs` |
| K2a | Identity-keyed spill tree: `PrivateMemoryStore.owners: RwLock<HashMap<u32,String>>`, `bind_storage_identity(pid, identity)` (sanitised), `owner_dirname` (`id-<hex(identity)>`, unbound pid → legacy `<pid>`), `open_namespace_dir` uses it (`:273`), the forget walk accepts `id-*` dirs (`:849`), in-memory erase label; `MemoryManagerAdapter::bind_storage_identity` delegate (`memory/mod.rs` beside `authorize_principal_writes` `:401`) | `k2a-identity-keyed-spill-dirs.diff` | **26450 (+41)** | **20651 (+33)** | pins `memory/private.rs`, `memory/mod.rs`; two new pub fns → `kernel-api-classes.toml` (`PrivateMemoryStore` is `data-movement`, `:440,456,481`) + `docs/ci-baselines/kernel-surface-v0.1-beta.json` (`:797,1137`) |
| K2b | K2a + principal index owner column: `writer_owner TEXT NOT NULL DEFAULT ''` in `CREATE TABLE` + guarded `ALTER TABLE … ADD COLUMN` for existing DBs (`memory/principal.rs:16-26,41-49`), `record_write`/`lookup` carry it (`:65-140`; inline tests `:179-225` updated), write records `owner_dirname(pid)` (`mod.rs:741`), export reads by recorded owner (`mod.rs:921`) through a new disk-only `PrivateMemoryStore::read_owner` | `k2b-identity-index-owner-column.diff` | **26499 (+90)** | **20697 (+79)** | + pin `memory/principal.rs`; **maos-domain +1** (9975 → 9976): `PrincipalIndexRow.writer_owner` (`crates/maos-domain/src/memory.rs:319-325`) |
| **R** | **K1 + K2a** (D-20-3b-05 A) | `krec-k1-k2a-bundle.diff` | **26451 (+42)** | **20652 (+34)** | as K2a |
| **R′** | **K1 + K2b** (D-20-3b-05 B, recommended) | `kfull-k1-k2b-bundle.diff` | **26500 (+91)** | **20698 (+80)** | as K2b; maos-domain +1 |

ADR-069 (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`): 20,698 is 1,302 under the 22,000 alarm and
2,302 under the 23,000 envelope; the envelope is not headroom — R/R′ each need a FLAG-Winston grant row.
Not compiled ([INFERENCE]): `hex` is already a kernel dependency (`crates/maos-kernel-core/Cargo.toml:28`);
`read_owner` is `#[cfg(unix)]` while the `export_redactable` caller is not — the dev pass adds the non-Unix twin
(the store already refuses non-Unix spills, `private.rs:658-664`; ≈ +5 [INFERENCE]).

## maos-bin, priced by prototype

| Prototype | D-rows | Diff | maos-bin |
|---|---|---|---:|
| Bind `spirit_id` as the storage identity after `on_loaded(pid)` and before admission/start (rollback + `AdmissionRefusal::Load` on refusal), `crates/maos-bin/src/admission.rs:859`; shell hello-spirit path before `start`, `main.rs:4363-4368` | D03 A, D-20-3b-06 A | `b1-maos-bin-identity-binding.diff` (on R) | 32939 → **32948 (+9)** |
| Retire the two gate names from the `smoke-discipline-7-1-5` arm list + its printed JSON (`main.rs:13843-13848,13884`) | D-20-3b-10 A | part of `x1-retirement-enrolment-edits.diff` | **−2** |

## xtask retirement, priced by prototype (gross deletion only)

`x1-retirement-prototype.py` (run in a copy root) deletes the ten modules, their `mod` lines (`xtask/src/main.rs:11-83`),
`#[command]` variants (`:421-941`) and dispatch arms (`:1353-1494`), the `EXPECTED_GATES` entry
(`check_ship_gate_completeness.rs:24`), the two `prepush` legs (`prepush.rs:25-28,42`) and the maos-bin smoke names;
it **moves** (does not delete) the four helpers `check_dev_model_tier.rs:19-21` and
`check_dev_record_completeness.rs:476-478` import from `check_dev_model_used_populated.rs` (`:17-108`, `:301-367`)
into `check_dev_record_completeness.rs`. Folds, generator and other adds are **not** prototyped.

| Variant | Diff / script | xtask | maos-bin | aggregate |
|---|---|---:|---:|---:|
| GO (all ten) | `x1-retirement-enrolment-edits.diff` + module deletion | 47682 → **42273 (−5409)** | −2 | 192778 → **187367 (−5411)** |
| NO-GO (keep `check_coverage_matrix_completeness.rs` + its prepush leg) | same script, `nogo` | **42366 (−5316)** | −2 | **187460 (−5318)** |

−5,409 vs the 5,499 module tokei: the +90 difference is the relocated helpers net of the removed `main.rs`/`prepush`/
`EXPECTED_GATES` enrolment lines. Workflow/registry/YAML/test/doc edits are kloc-free
(`xtask/src/kloc_check.rs:303,311`; `crates/*/tests`, `xtask/tests` uncharged) and are not in these figures.

## Add-side forecast (not prototyped — [INFERENCE], measured analogues)

Non-test tokei = file head before the first `#[cfg(test)]`, tokei 14.0.0.

| Add | Forecast | Analogue (measured) |
|---|---:|---|
| Fold `check-bare-review-findings` → `check-review-findings-resolved` | +25…+72 | retired logic 72 non-test; target 226 non-test |
| Fold `check-dev-model-used-populated` → `check-dev-record-completeness` | +60…+185 | retired 335 non-test, of which ≈ 150 helpers already relocated in X1 |
| Fold `check-epic-close-green` → `check-epic-close-coherence` | +30…+87 | retired 87; target 298 non-test |
| Generator (source + emitter + byte-identical oracle; subsumes completeness) | +200…+500 | `coverage_matrix.rs` 174 non-test + completeness 80 |
| `pub use` group-hash fix (per-leaf hash) | +5…+25 | `check_service_boundary.rs:393-396,480-483` |
| prepush Blocking-set mirror (D-20-3b-12 B literal + pin test in `xtask/tests`) | +1…+5 | `prepush.rs:14-45` |
| Invariant ledger (D-20-3b-14 A: frontmatter + existing `invariant-lock`) | +0…+40 | `invariant_lock.rs` reads `docs/invariants/I*.md` |
| **Σ adds** | **+321…+914** | — |

Net xtask forecast (GO): −5,409 + 321…914 = **−5,088…−4,495**. With the D02 A review-correction allowance applied
to the adds only (×3 upper = E19's observed 2–3×): −5,409 + 2,742 = **−2,667** worst case — net-negative in every case
provided the deletion lands in the same landing as the adds.

## Measurements owed (the dev pass runs these; one cargo process; prefix every command)

`P="env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo"`; gates as `$P cargo run -q --locked -p xtask -- <gate>`.
D = `_bmad-output/implementation-artifacts/20-3b-gate-retirement-and-coverage-generator-evidence/creation`.

1. `check-kernel-baseline --json` and `kloc-check --json` at T0 — expect 26409 / 20618, aggregate 192778 (or the
   then-HEAD figure; a mismatch re-prices, it is not absorbed).
2. `coverage-matrix --json` — expect 33 violations (the replica's set); a different count is a finding to record
   before the spike (AC1).
3. `git apply $D/kfull-k1-k2b-bundle.diff` (or `krec-…` per the ruling) → `$P cargo check --locked -p maos-kernel-core --lib -p maos-domain`
   (expect rc 0 on Linux) → both gates (expect 26500/20698, domain 9976) → `check-service-boundary` (expect the two
   new pub fns unclassified) → `git checkout --` the touched files → gates re-PASS.
4. `cargo clippy --workspace --all-targets --locked -- -D warnings` warning count (D-20-3b-11 A/B fork).
5. `git apply $D/x1-retirement-enrolment-edits.diff`, delete the ten modules → `$P cargo check --locked -p xtask -p maos-bin --features network`
   (the moved helpers' imports are [INFERENCE]) → `kloc-check --json` (expect xtask 42273, aggregate 187367 before
   adds) → restore.
6. Hosted wall-clock of `smoke-discipline-7-1-5` and the jobs retired here (`discipline.yml:3371-3399`, `:3636-3649`,
   `:4001-4014`, `:4059-4077`) on the latest green main run — the CI time the retirement saves.

Expected figures are this file's `/tmp` measurements; a mismatch is a finding for the parent, not a reason to
re-price inside the dev pass.
