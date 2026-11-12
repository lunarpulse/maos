# 21-5 creation — records owed

At creation (2026-11-12, `0baa76fb`) a dev agent (Dev2014d) was editing the working tree and the tracker, so this
creation wrote **only new files**: the story, `21-5-flag-winston-decision.md` and this directory. Every edit below to an
existing file is owed, to be applied by Main (or the operator's delegate) once the tracker is free. Line numbers are at
`0baa76fb`; re-read before applying. No implementation closure is claimed.

## Applied at creation (new files only)

| # | File | Content |
|---|---|---|
| X1 | `_bmad-output/implementation-artifacts/21-5-orchestrator-port-and-move.md` | new story, `ready-for-dev` with every D-row `PROPOSED` |
| X2 | `_bmad-output/implementation-artifacts/21-5-orchestrator-port-and-move-evidence/creation/` | this directory |
| X3 | `_bmad-output/implementation-artifacts/21-5-flag-winston-decision.md` | new, `PROPOSED` (nothing granted) |

## §A — epic corrections owed to `epic-21-multi-host-on-live-substrate-w6.md` (OLD → NEW in `cites-0baa76fb.md`)

| # | Line | Edit |
|---|---|---|
| A1 | `:26` | after "−817 tokei code / −1028 physical": append "(re-measured 2026-11-12 at `0baa76fb`: module unchanged; prototype KP −1041 physical / −827 tokei from 27040 / 21201, `21-5-…-evidence/creation/measurements.md`)"; "re-pinned in tokei units because 21-4 lands first" → "re-pinned in both current units (physical baseline, tokei kloc) — ruling `s21` puts 21-5 after 20-9, before 21-4 (D-21-5-04)" |
| A2 | `:46` | "(negative; after 21-4)" → "(negative; pulled forward by ruling `s21`, 2026-11-12: after 20-9)"; "maos-domain +30–60" → "maos-domain +3 (KP)" |
| A3 | `:94` | `scheduler_loop.rs:89,111` → `:89,113` (+ replay `:490-505`); `main.rs:1995` → `:2355-2356`; `:5696`/`:5788`/`:5859` → `operator_door.rs:2028` / `:1031,1142` / `:2178`; `nfr_perf_4_pause_resume_latency.rs` → `lifecycle_journal_write_latency.rs`; `check_cohort_mesh.rs:674` → `:703`; `kloc.toml:195` (18933) → `:264` (21201); add the maos-bin test-support importers (C7) |
| A4 | `:92` | `4-kernel-design.md:185` → `:186`; add FR20 `prd/functional-requirements.md:54` ("NOT kernel-buffered") to *Closes* |
| A5 | `:119` | "→ 21-5 after 21-4 (unit of the re-pin)" → "→ 21-5 after 20-9 (ruling `s21`; both units re-pinned, D-21-5-04)" |
| A6 | `:5` | D-J's "ordered after 21-4": add "(superseded for ordering by ruling `s21`, 2026-11-12; the leaf-crate destination stands)" |

## §B — tracker owed (`sprint-status.yaml`)

| # | Line | Edit |
|---|---|---|
| B1 | `:415` | `21-5-orchestrator-port-and-move: backlog` → `ready-for-dev  # 📝 CREATED 2026-11-12 @0baa76fb (bmad-create-story, anthropic/claude-opus-5-5; lighter measured contract; Create215): story 21-5-orchestrator-port-and-move.md; evidence …-evidence/creation/; FLAG-Winston record 21-5-flag-winston-decision.md (PROPOSED). Pulled forward by ruling s21 (after 20-9). KP −1041 physical / −827 tokei; 22 surface triples; D-21-5-01…14 PROPOSED. \| WAS: backlog  # <prior note>` |
| B2 | `:410` | `epic-21: backlog` → `in-progress` only if D-21-5-13 A is ruled (first story created in the epic); otherwise unchanged |
| B3 | `:409` | if D-21-5-13 A: note on `epic-20-retrospective` that ruling `s21` waives its "before epic 21's first story is created" precondition for 21-5 only (the retro still re-preflights the rest of Epic 21) |

## §C — planning records owed

| # | File:line | Edit |
|---|---|---|
| C1 | `planning-artifacts/decisions-2026-11-12-acceleration.md:34,65` | "Status: story-creation task, NOT yet done" → "story created 2026-11-12 (`21-5-orchestrator-port-and-move.md`, D-rows PROPOSED)" |
| C2 | `implementation-artifacts/deferred-work.md:1463` | per D-21-5-14: A keeps the owner and adds "accepted as AC6 (L1)"; B re-points the owner to the key the operator names; the row's stale `xtask/kloc.toml:406` "exact 8270" → "8770" either way |

## §D — owed at the landings (inside the story's tasks; listed so nothing is missed)

| # | File | Landing | Edit |
|---|---|---|---|
| D1 | `xtask/kernel-core-baseline.toml:505,1070,1120-1124` + content hashes of `lib.rs`, `scheduler/scheduler_loop.rs`, `isolation/mod.rs` + EOF HISTORY | L2 | `--emit-pin`; HISTORY first segment cites this key with `->` |
| D2 | `xtask/kloc.toml:264,684,425,339,718` | L1 (domain, aggregate) / L2 (kernel, leaf, xtask, aggregate) | exact; kernel row HISTORY carries `ADR-069 RELIEF VALVE (21-5, ruling s21)` |
| D3 | `xtask/tests/kernel_pin_content_hash_16_0.rs:34,820,838,847,858`; `xtask/tests/recovery_lane_ceiling_rule.rs:279-283` | L2 | constants + comment; or the ruling-4 `repin` script if it has landed |
| D4 | `docs/ci-baselines/kernel-surface-v0.1-beta.json` | L2 | surgical: delete 21 triples, refresh one hash (D-21-5-08) |
| D5 | `xtask/kernel-api-classes.toml:367-375` | L2 | delete nine rows |
| D6 | `xtask/serde-error-allowlist.toml:733,736` | L2 | delete (stale at HEAD; the file moves) |
| D7 | `Cargo.toml:3-62`, `Cargo.lock`, `crates/maos-orchestrator-buffer/Cargo.toml` (new), `crates/maos-bin/Cargo.toml:55`, `spirits/orchestrator/Cargo.toml:21` | L2 | member + dependencies |
| D8 | `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md:117` | L2 | new `<!-- workspace-count-authoritative -->` sentence (58 → 59); the 17-1 sentence keeps its text without the sentinel |
| D9 | `STABILITY.md:30` | L2 | regenerate (`stability-matrix`) |
| D10 | `docs/adr/ADR-070-…md` (new) + `docs/adr/index.md` | L2 | if D-21-5-08 A |
| D11 | `prd/functional-requirements.md:54` (FR20), `:68` (FR54); `4-kernel-design.md:186` | L1 | dated notes per D-21-5-01 (no FR text deleted) |
| D12 | `xtask/src/check_dependency_closure.rs:42-64` | L2 | `maos-orchestrator-buffer` in `FORBIDDEN_CRATES` (D-21-5-09 A) |
| D13 | `spirits/orchestrator/src/lib.rs:40-47` | L2 | doc: "the real `OrchestratorBuffer` (crate `maos-orchestrator-buffer`)" |

## §E — routed observations (rule 10)

| # | Observation | Destination |
|---|---|---|
| E1 | The resume replay drains and re-enqueues under two lock scopes — a window where a concurrent enqueue lands ahead of older items (`scheduler_loop.rs:494-503`, `buffer.rs:46-80`) | (a) fix at origin: this story, D-21-5-07 |
| E2 | `check-cohort-mesh` shells out to `cargo build` (`check_cohort_mesh.rs:26,48,157`) — not runnable on a measurement copy without a build | observation only; the gate is correct for CI |
