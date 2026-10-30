# 20-3b creation — cites re-measured at `23f58da4` (rule 9)

OLD = the epic's text at `git show 23f58da4:_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md` (E20@HEAD),
the tracker row `sprint-status.yaml:299` or the deferred-work row, as named. NEW = the anchor read at `23f58da4`
(code unchanged since `3fd7f8b4`). "same" = symbol and behaviour still at the cited line. Rows already applied by
the 2026-10-30 D01 A amendment (preflight §3.2) are marked **(preflight)**; rows the preflight did not carry are
**(new at creation)** and are owed to the epic owner as `records-owed.md` E-rows.

## Epic §20-3 pre-split AC4/AC5/AC7 (E20@HEAD:130-133) and split row (:64, :103)

| # | OLD | NEW @`23f58da4` | Source |
|---|---|---|---|
| C1 | `tests/coverage-matrix.yaml:15-33`, 1982 lines, 198 entries, `:3 mode: warning` | 2051 lines, 198 rows, `mode: warning` `:3`, `delivered_phase: v0.1-alpha` `:2`, `coverage:` `:15` | (preflight) + `delivered_phase` (new) |
| C2 | `tests/coverage-source.toml`; `covers = [...]` in `gate-registry.toml` "grep 0" | both still absent (`ls tests/coverage-source.toml` none; `grep -c covers xtask/gate-registry.toml` = 0) | (preflight) |
| C3 | `invariant_lock.rs:156`; `coverage_matrix.rs:93-120` "unchanged"; `mode: error` | `:156` same; `coverage_matrix.rs:93-120` same text, but the mode check is `:110-117` (`warning`/`hard` only) and enforcement `:65` (`mode == "hard"`) — "unchanged" and `error` are incompatible → **D08** | (preflight) |
| C4 | "absorbs 15-3's 33 coverage-matrix violations" | **33 reproduced by replica** (27 unknown-gate refs / 21 rows, 3 unknown corpora, 3 out-of-order phases); the gate itself not run (not permitted this run) | (new at creation) |
| C5 | `check_coverage_matrix_completeness.rs` 100 lines | 100 physical / 80 tokei; ALSO enrolled at `discipline.yml:4001-4014` (job), `:6254` (v1-0 needs), `:6516` (aggregate), `prepush.rs:25-28`, `proof-contract.md:156`, `xtask/tests/coverage_matrix_completeness_tests.rs` | (new at creation) |
| C6 | "SIX enrolment sites": `main.rs` mod `:96`, `#[command]` `:949`, dispatch `:1360`; `check_ship_gate_completeness.rs:20`; `tests/coverage-matrix.yaml`; `discipline.yml` `aggregate.needs` `:3510+` (123); `gate-registry.toml`; + `d19_story_file_governance.rs:355-362` | `main.rs` mods `:11-83`, commands `:421-941`, dispatch `:1353-1494`; `EXPECTED_GATES` `:17` (entry `:24`); YAML `:2004`; `aggregate:` `:6391`, needs `:6394-6535` (**133**); registry `gates` `:34,64,68,72` + `[[ship_gate]]` `:270` (46 rows); `d19_story_file_governance.rs:355-362` same (+ `:157-158,175-176,186,251,274,311-312`). **Additional sites (new at creation):** `prepush.rs:25-28,42`; job-level `needs` of `smoke-discipline-7-1-5` (`discipline.yml:3718`) and the maos-bin arm `crates/maos-bin/src/main.rs:9214-9216,13836-13887`; aggregate script outputs `discipline.yml:6582-6588,6689-6695,6769-6772`; v1-0/v1-5 ship-gate needs + summaries `:6254,6257,6313,6316,6359,6362,6830,6845`; **library dependents** `check_dev_model_tier.rs:19-21`, `check_dev_record_completeness.rs:476-478`; xtask tests `story_10_2_proven_red.rs:784-1188`, `coverage_matrix_completeness_tests.rs`; docs `proof-contract.md:156,159,167`, ADR-031 `:75`, ADR-040 `:5`, ADR-043 `:3,14-29`, ADR-045 `:60`, ADR-068 `:6` | (preflight: count of sites superseded) + rows (new) |
| C7 | (1) `check_epic_6_bridge.rs` **4592**; retired from discipline `discipline.yml:1370-1374`; `main.rs:73,:1251` | 4590 / 4023 tokei; retirement comment `discipline.yml:2890-2898`; `main.rs:73` (mod), `:580` (command), `:1364` (dispatch); also read by `check_j1_loopback_delegation.rs:103` (comment only) | (preflight) + comment cite (new) |
| C8 | (2) `check_cross_form_equiv.rs` **421**; "never scheduled" | 419 / 312; job **exists** `discipline.yml:4059-4077` but the artifact `docs/cross-form/results/cross-form-results.json` is absent (only the schema), so the job always takes its `::warning::` branch and the module never runs in CI; ship row `gate-registry.toml:270-272` advisory/advisory | (preflight) + artifact absence (new) |
| C9 | (3) `check_adr_040_accepted.rs` **217**, `grep -rl adr-040-accepted .github` empty | 217 / 174; `.github` grep for `adr040\|adr-040-accepted` empty (rc 1); registry `:34`; ADR-068 `:6` states ADR-040's `Status:` line "is read by `check-adr-040-accepted`" — that sentence is retired with the module | (new at creation) |
| C10 | (4) `check_literal_reappearance.rs` **158**, "in no workflow" | 158 / 125; same; registry `:64`; forbidden literals `smoke-founder-loop`, `smoke-mira-nash` (`:9`) occur in **no** production `src` (`git grep` hits only `crates/maos-bin/Cargo.toml:80,84` comments and a skipped test file) | (new at creation) |
| C11 | (5) `check_governance_categories.rs` **108**; round-trip "beside `kind_category_to_kinds`" (maos-audit +20) | 108 / 78; `kind_category_to_kinds` `crates/maos-audit/src/lib.rs:832`, inverse `:847`; ADR-045 `:60` names the module | (new at creation) |
| C12 | (7) `check_deprecations_declared.rs` **69**, "in no workflow" | 69 / 52; in no workflow, but **prepush** `:42` and `proof-contract.md:159,167` (named CR-2 exception) | (preflight) |
| C13 | (8) `check_bare_review_findings.rs` **164** → fold (+40) | 164 / 137 (non-test 72); job `discipline.yml:3371-3384`; `smoke-discipline-7-1-5` `:3718` | (new at creation) |
| C14 | (9) `check_dev_model_used_populated.rs` **463** → fold (+60) | 463 / 431 (non-test 335); job `:3386-3399`; four helpers imported by two live gates (C6) | (new at creation) |
| C15 | (10) `check_epic_close_green.rs` **114** → fold (+30) | 114 / 87; job `:3636-3649`; aggregate `:6497`; ADR-043 `:3` names it the removal verifier | (new at creation) |
| C16 | Arithmetic 6406 retired, net −4476…−5176 | **6402 physical / 5499 tokei**; measured gross with enrolment edits **−5409** xtask (GO) / −5316 (NO-GO), maos-bin −2; adds unmeasured (forecast +321…+914) | (preflight) + prototype (new) |
| C17 | "15-1's `pub use` group-hash defect" | `check_service_boundary.rs:393-396,480-483` same (`surface_item("use", &path, item)` hashes the whole `Item::Use`); filing `15-1-green-at-head.md:352-355` same | (preflight) |
| C18 | "`check-cross-form-equiv` never-scheduled advisory lift (D-G)" | premise false (C8) | (preflight) |

## Received item §20-3b (E20@HEAD:278-280) — FR31

| # | OLD | NEW @`23f58da4` |
|---|---|---|
| F1 | `deferred-work.md:1252` | `deferred-work.md:1196` (confidentiality), `:1197` (restore-poisoning extension, 18-4 AA-7) |
| F2 | `crates/maos-kernel-core/src/memory/private.rs:264-279` | `open_namespace_dir` `:263-278`; the pid component `:273` |
| F3 | `scheduler/scheduler_loop.rs:29-33` | same (`NEXT_SPIRIT_PID` `:29`, `allocate_pid` `:31-33`) |
| F4 | `memory/mod.rs:835-860` ("read path checks region only") | read arm `:779-830` (private dispatch `:804`), scan arm `:832-860` (`:858`); principal-write enforcement checks the **pid set** only (`:715-726`, set `:99`, `authorize_principal_writes` `:401-407`) |
| F5 | `private.rs:618-626,644-675` | `should_spill_to_disk` `:618-626` same; write `:632-677` (spill decision `:644`, map insert `:671-675`) |
| F6 | inline threshold `crates/maos-bin/src/main.rs:2219-2222` (deferred row) | `main.rs:2154-2157` (`4 * 1024` at `:2156`) |
| F7 | `crates/maos-shell/src/lib.rs:198-200` (deferred row) | `:197-199` |
| F8 | "+1 physical / +1 tokei (durability), identity +20–50 [INFERENCE]" | **measured**: K1 +1/+1; K2a +41/+33; K2b +90/+79 (+ maos-domain +1); R = K1+K2a **+42/+34**; R′ = K1+K2b **+91/+80** |
| F9 | (not cited) principal index keyed by pid | `memory/principal.rs:16-26` (`PRIMARY KEY (principal_id, writer_spirit_pid, …)`); export reads by the recorded pid `memory/mod.rs:921`; forget walk skips non-numeric dirs `private.rs:849` |
| F10 | routing audit `:45,155` | same |

## Tracker row `sprint-status.yaml:299` and deferred-work rows owned by this key

| # | OLD | NEW @`23f58da4` |
|---|---|---|
| T1 | "TEN gate retirements across SIX enrolment sites, xtask net −4.5k…−5.2k — incl. `check_epic_6_bridge.rs` 4592" | C6, C7, C16 |
| T2 | "`check-cross-form-equiv` never-scheduled advisory lift" | C8 |
| D1 | `deferred-work.md:902` — `cargo deny` advisory (`discipline.yml:38,44,121`); clippy not executed | **deny half CLOSED by 18-0** (D-18-0-04): own blocking `cargo-deny` job `discipline.yml:157-189`, no `continue-on-error`, in `aggregate.needs` `:6397`; clippy half OPEN: `components: rustfmt, clippy` `:81`, no `cargo clippy` invocation in `.github/workflows/*.yml` |
| D2 | `deferred-work.md:909` — rustls 0.23.40 RUSTSEC-2026-0285, chacha20 0.10.0 yanked; deny advisory | **CLOSED**: `Cargo.lock:4541-4542` rustls **0.23.46**, `:4574-4575` rustls-webpki **0.103.16**, `:664-665` chacha20 **0.10.2**; deny blocking (D1). Advisory DB not queried (no cargo-deny run here) |
| D3 | `deferred-work.md:1220` — `prepush.rs:14-46` lacks `check-epic-close-coherence`; `check_epic_close_coherence.rs:183-191` | `prepush.rs:14-45` (`GATES`), still absent; `error-catalog-check` reads `CARGO_MANIFEST_DIR` at runtime `check_error_catalog.rs:644-657` |
| D4 | `deferred-work.md:1196-1197` | F1–F9 |
| D5 | 16-3 §11 row 15: NFR-Rel-1/2/11 rows name job ids absent from the registry (`tests/coverage-matrix.yaml:1376,1386,1391`) | rows now `:1421` (NFR-Rel-1), `:1431` (NFR-Rel-11), `:1436` (NFR-Rel-2), each `gates: []`, phase `v0.8` (deferred under `delivered_phase: v0.1-alpha`) — uncovered, not mis-named |
