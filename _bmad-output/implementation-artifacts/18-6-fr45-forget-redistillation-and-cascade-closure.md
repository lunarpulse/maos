---
story_id: '18.6'
story_key: 18-6-fr45-forget-redistillation-and-cascade-closure
created: 2026-10-26
context_commit: 5495863e
baseline_commit: 5495863e
security_sensitive: true
kernel_grant: "◆ FLAG-Winston, **RATIFIED 2026-10-26 by the operator (Lunarpulse), ADR-069 envelope** (`18-6-flag-winston-decision.md` §Rulings): D-18-6-01 B + 02 A + 04 A = **+8 physical / +17 tokei**: `check-kernel-baseline` 26418 → **≤ 26426** (95 files, unchanged), `maos-kernel-core` 20643 → **≤ 20660**, exact measured figure at landing after `cargo fmt --all`. File: `crates/maos-kernel-core/src/memory/mod.rs` **only** (one any-writer, any-kind principal scan replaces the two index-keyed loops; derived-distillate closure; re-distillation requests formed before the scrub). Any other kernel file or a figure above the grant returns as a FLAG request. Kernel public surface unchanged (`check-service-boundary` PASSED, 0 violations, with the measured prototype)."
kloc_grant: "RULED 2026-10-26 (operator, Lunarpulse): D-18-6-08 A — **Epic 18's named maos-bin budget 28700 → 28770**; exact re-pin at landing; above 28770 → STOP and re-ask. Zero-headroom rows are measured requests at landing; STOP at dev above expected + 25 %: maos-iac 7979 + up to 150 expected → STOP above +188 (8167); maos-domain 9848 + up to 47 → STOP above +59 (9907); maos-audit 7473 + 54–105 → STOP above +131 (7604). maos-cli and maos-journey-test land exact inside their ceilings. `_aggregate_hardfail` = measured + 1."
review: "bmad-code-review under docs/testing/proof-contract.md. Difficult story (kernel FLAG-Winston + security-critical erasure ACs): creation anthropic/claude-opus-5-5; dev/finisher openai-codex/gpt-6.1-sol; Blind, Edge and Acceptance on opus-5.5, Test Infrastructure on glm-5.3, plus a security-focused reading of AC1–AC3 (erasure reach, over-erasure, pid fixing, purge of live working memory). FLAG-Winston signs off the kernel diff. fable-5.1 only with operator permission."
---

# Story 18.6: FR45 forget — re-distillation and cascade closure

Status: review

<!-- Created 2026-10-26 at 5495863e by bmad-create-story under the lighter creation contract (no party-mode preflight). 18-0/18-1/18-8/18-2/18-3 done; 18-4 and 18-5 implemented and pushed (landing CI running), their code treated as landed. Direct reads of every decision-bearing range; the host's only cargo process (CARGO_TARGET_DIR=/mnt/build/cargo, `env -u CI -u GITHUB_ACTIONS`) ran `check-kernel-baseline` (PASSED 26418 / 95) and `kloc-check --json` (passed, aggregate 185451) at HEAD, eight throwaway kernel prototypes over one shared out-of-kernel prototype (measured, then reverted), `cargo check --locked --workspace` for the ripple and `check-service-boundary` for the surface. FLAG record `18-6-flag-winston-decision.md`. RULED 2026-10-26 by the operator (Lunarpulse): D-18-6-01…10 all as recommended (01 B, 02–10 A); kernel grant +8 / +17 ratified; maos-bin named budget 28770. Rulings applied docs-only (no cargo). -->

> **The capability:** *when the operator forgets a principal, nothing anyone wrote about them stays findable, and what was summarised from them is rebuilt without them. Every Transparency-Log frame that names the principal — by any writer, indexed or not, including the distillates other Spirits made from it and the digests derived from those — is scrubbed and marked. The Private namespace and its index go as today. Each live Spirit drops the scrubbed frames from its own working memory, and the writer of every redacted distillate regenerates it from the sources that survived, through its own distillation logic; a writer that cannot is recorded, typed, as pending. A legal hold still refuses, typed.*

**Closes:**
- PRD FR45 (`prd/functional-requirements.md:102`): the cross-Spirit cascade and "distillates containing principal data are marked redacted with re-distillation triggered".
- NFR-Aud-10 (`prd/non-functional-requirements.md:70`): 50/50 clean removal, 50/50 redaction markers, 0 leakage in 100 subject-access queries, measured on the production forget path (D-18-6-07).
- ADR-044's unnamed "follow-up" (`docs/adr/ADR-044-gdpr-article-17-distillate-redaction.md:36-38,46`), superseded by an implementation note.
- Recovery Coverage Map `epics/requirements-inventory.md:563`; coverage-matrix rows FR45 / NFR-Aud-10 (`tests/coverage-matrix.yaml:692-700`, `:912-919`).
- **Inherited `deferred-work.md` rows owned by this story: none** (grep for `18-6` and `18-6-fr45`: zero owner rows). Adjacent rows recorded, not inherited: W1 cross-store non-atomicity `deferred-work.md:449` (no owner cue; D-18-6-10), crash-between-mutation-and-audit `:555` (`v25-erasure-crash-reconciliation`), pid-keyed Private memory `:1270` (`20-3b-gate-retirement-and-coverage-generator`), OTel span erasure-exemption acknowledgement `:19-20` (erasure-exempt by construction; scope documentation only).

**Does NOT close, and says so (rule 11: stated as limits):**
- **Content-blind detection.** The kernel finds the principal by its id (token-bounded, D-18-6-09), never by meaning (ADR-026 `:12`: the kernel does not interpret content). Data copied without the id escapes (D-18-6-01 C, value needles, was not ruled); transformed or JSON-escaped copies escape under every option.
- **Shared and Loom.** Under D-18-6-03 A they hold no principal-namespaced data by partition (13-5h). A non-principal Shared or Loom row that happens to name a principal is retained by design; no production writer exists at HEAD.
- **External re-ingestion.** A source outside MAOS (an MCP server) that still returns the principal after forget is a new collection, not resurrection; the witness's fixture serves the principal once.
- **Cross-boot re-distillation.** A redacted distillate whose writer is not live in this boot gets a typed pending row and is not resumed later (D-18-6-06 A); stable identity is 20-3b's.
- **NFR-Aud-12 / NFR-Aud-13.** The uninstall receipt inventory is `20-11-erasure-inventory-closure`'s; the SLA test (`erasure_sla_test.rs`) is unchanged — forget stays synchronous and re-distillation is regeneration, not erasure.
- **Other readable stores** not named in AC1.4 (the lifecycle journal, approval/retraction companion tables, the legal-hold table that must name the principal) are not scanned [INFERENCE: none carries principal content at HEAD; not measured].

## Story

As **a director whose data passed through several Spirits**,
I want **`maosctl forget --principal <id>` to remove every copy any Spirit wrote — raw frames, distillates and digests derived from them, and what live Spirits hold in memory — and to have each redacted summary rebuilt from what remains**,
so that **my right to be forgotten holds across Spirits, the system's knowledge stays useful, and nothing silently resurrects me on the next pass.**

**Contract:**
- Epic-18 §18-6 (`_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md:223-237`, corrected in place at creation and marked ruled; creation corrections `:227`); stories row `:115`; §R R18-19 `:88` (re-anchored), R18-20 `:89`; Kernel-Δ `:32` (18-6 grant `:40`); kloc ruling `:56` (maos-bin 28770, STOP lines); Dependencies `:268` (18-3 → 18-6), addendum `:274`; confidence rules 9–11 (`:58-60`).
- Epic-17 R17-224 (`epic-17-workers-and-third-party-form-w2.md:774`; D-17-9-PF-R5 A).
- PRD FR45 `functional-requirements.md:102`; NFR-Aud-10/12/13 `non-functional-requirements.md:70,72,73`.
- ADR-026 (`docs/adr/ADR-026-principal-memory-namespace.md:12`); ADR-044 (`:20-46`); ADR-069 (`:20-25,35-46`).
- 13-5h Decision D (`13-5h-shared-tier-principal-partition.md`); 18-4 principal ledger (`18-4-self-tuning-halt-with-injectable-clock.md` D-18-4-02 A, EC-1); 18-3 Researcher distillate (`18-3-researcher-replay-live-and-judged-five-metric.md` AC4.1).
- `routing-audit-2026-10-22.md:3,71,103`; `docs/testing/proof-contract.md` (binding).

## Current-tree corrections — read before implementing

Measured at `5495863e` by direct reads, `check-kernel-baseline`, `kloc-check --json`, reverted prototypes, a workspace `cargo check` and `check-service-boundary`.

| Premise (epic / §R) | Fact at `5495863e` and consequence |
|---|---|
| The forget path: `maosctl forget` (`subcommands.rs:1807`) → the door (`operator_door.rs:1334`) → `forget_with_reason` (`memory/mod.rs:439`, snapshot `:545`) | **Holds**, and it is one of **three** production callers: the door `operator_door.rs:1341`, the offline one-shot `crates/maos-bin/src/main.rs:6524-6544` (call `:6530`), and the uninstall cascade `main.rs:9492` (exhaustive `Erased` pattern `:9493-9497`), reached by `maosctl uninstall` over the door (`BinPrivateOps::run_uninstall_cascade`, `operator_door.rs:65-68`) or one-shot. Every request path must handle re-distillation (CR-3). |
| (1) "no re-distillation trigger exists; ADR-044 deferred it" | **Holds.** `grep -rniE 'redistil|re-distil' crates spirits xtask` = 0. ADR-044 Decision 3 `:36-38`, Consequences `:46`. |
| (2) "the cascade misses the Shared tier: only Private writes record a principal-index row (`:785-800`; Shared at `:801`)" | **Stale premise.** Since 13-5h (`dd4a908e`) a `Principal` namespace is refused outside Private on write, read and scan (`reject_principal_outside_private` `memory/mod.rs:182-202`; calls `:784`, `:858`, `:912`); the Private arm records the index row `:786-800`, Shared `:801` stores only non-principal namespaces. `REGISTERED_ERASURE_BACKENDS` = private, principal_index, shared, loom (`:37`), partitioned by `crates/maos-audit/tests/multi_backend_erasure_test.rs:280-326` and the `gdpr-backend-partition` leg (`xtask/src/check_reza_production_path.rs:530`). No Spirit-facing tier surface exists (`Ctx`/WIT have none) and no production `MemoryTier::Shared` writer exists outside `memory/mod.rs`. → D-18-6-03. |
| (3) "distillate scrubbing is limited to writers the index knows (`:545-560`)" | **Holds, wider than stated.** Snapshot `:545-547`; Distillate scan by indexed pids `:556-575` (`distillate_frames_for_pids` `crates/maos-iac/src/adapter/transparency_log.rs:1508-1557`); the SR-3 cost/governance scan has the same pid limit `:577-614`; frames of every other kind (MCP invocations, IAC frames, seed claims) are never scanned. Pids are boot-local, so the pid filter also matches other boots' Spirits. Match is a raw substring (`:562-566`). → D-18-6-01, D-18-6-09. |
| "the floors are asserted only over a fixture corpus whose 50 scenarios include held, failed and absent subjects" | **Holds.** `tests/corpora/gdpr-cascade-v0.jsonl`: 35 erased / 9 held / 5 not_found / 1 failed (measured); replayed against the adapters, not the door (`crates/maos-audit/tests/gdpr_cascade_corpus_test.rs`; Reza leg `gdpr-kernel-forget-outcome-corpus` `check_reza_production_path.rs:549-557`; CI `discipline.yml:4936`). → D-18-6-07. |
| "After 18-3, whose Researcher replay/live paths and distillate traceability are what a re-distillation re-runs" | **Half true.** The Researcher writes a distillate only when bound: `bind` adds the `DistillationPort` (`crates/maos-bin/src/researcher_root.rs:173-175`); deterministic mode has none and `persist` writes no distillate (`spirits/researcher/src/lib.rs:1103-1117`). Principal data reaches a Researcher distillate only through the `--live` MCP survey (`lib.rs:1040-1071`; `--live` fills `ResearcherWiring.mcp`, `main.rs:4027-4047`); the replay/daemon pass surveys fixed seed claims (`lib.rs:1075-1098`). Butler writes principal data (18-4 `PidBoundPrincipalPort`, `crates/maos-bin/src/butler_root.rs:52-120`, Private Markdown, indexed) and no distillate. So the production cross-Spirit case at HEAD is: an indexed Butler record + an unindexed Researcher `--live` distillate and its MCP-invocation source frames. |
| AC1 "working-memory references in other Spirits" | A live Researcher caches recalled frames and its last survey in process memory (`mcp_recall.frames` `lib.rs:1054-1070`, `seed_frames` `:1076-1097`, `last_output` `:608,613`); its next pass re-joins claims to those frames (`:1070`) and would re-distil the erased data. 18-4 already re-reads Butler's record per decision (EC-1). → D-18-6-05. |
| AC3 "the kernel triggers; the owning Spirit's distillation logic regenerates" | No Spirit has a re-distillation entry. `on_consolidate` is the one payload hook with a production caller (`crates/maos-bin/src/hook_pump.rs:669-766`, today an empty payload `:751`; dispatcher `crates/maos-kernel-core/src/scheduler/hook_dispatch.rs:336-343`, gated by `hook_allowed` `:174-184`). The Researcher implements none (only Observer, `spirits/observer/src/lib.rs:604`); its manifest enables all hooks (no `[lifecycle]`; empty = all, `crates/maos-spirit-abi/src/lifecycle.rs:514-519`). Compiled-in classes are registered with the pump on boot and door load (`crates/maos-bin/src/admission.rs:640-643,801`); the pump exists before the door (`main.rs:2997` vs `:3500`). → D-18-6-05. |
| (unstated) a redacted distillate still knows its sources | **False.** The ADR-044 scrub replaces the whole body (`transparency_log.rs:1388-1415`), `source_log_ref` included, and refs are flattened to raw frames (`crates/maos-iac/src/adapter/distillate.rs:191-260`). The surviving sources must be read before the scrub. → D-18-6-04, D-18-6-02. |
| AC2 "a legal hold still refuses, typed" | **Holds.** `ForgetOutcome::Suspended` (`memory/mod.rs:457-539`; durable hold `:471-478`); door terminal 3 (`operator_door.rs:1354-1362`); one-shot exit 3 (`main.rs:6537-6542`). |
| (unstated) readers parse frame bodies by kind | A tombstoned non-distillate body must not turn into a schema error: FR4's `classify_fr4_row` matches payload shapes (`crates/maos-audit/src/fr4_classifier.rs:556-581`). → CR-6. |
| Routing audit `:103` cites §18-6 at `epic-18:118-121` | **Stale.** §18-6 is `:223-237`. The audit file is outside this story's doc set and is not edited (recorded). |
| ADR-069 `:20-25`: 18-6 among the unpriced ◆ stories | Priced here and ratified 2026-10-26: +8 / +17. |
| Holds | FR45 `:102`; NFR-Aud-10/12/13 `:70,72,73`; ADR-026 `:12`; ADR-044 `:36-46`; inventory `:563`; tracker `sprint-status.yaml:282`; R17-224 `:774`; scout `review-2026-09-04/08-fr21-65-scout.md:3`; `subject_access_query` `crates/maos-audit/src/lib.rs:1868`. |

**Epic cites corrected in place (OLD → NEW @`5495863e`):**
- §18-6 "(2) the cascade misses the Shared tier: only Private writes record a principal-index row (`memory/mod.rs:785-800`; Shared at `:801`)" → the Shared tier refuses the `Principal` namespace since 13-5h (`:182-202`; `:784`, `:858`, `:912`); Private arm `:786-800`; the clause is D-18-6-03.
- §18-6 "(3) … (`:545-560`, `distillate_frames_for_pids`)" → `:545-575` (scan `:556-575`), plus the SR-3 cost/governance scan `:577-614` with the same index-pid limit; raw substring `:562-566`.
- §18-6 "`forget_with_reason` (`:439`, index snapshot `:545` @15b02d98)" **holds** at `5495863e`; the door is `operator_door.rs:1334` (call `:1341`); add the one-shot `main.rs:6524-6544` and uninstall `main.rs:9492` callers.
- §18-6 AC1 "A test named at creation" → `crates/maos-journey-test/tests/journey_forget_18_6.rs` (process leg) and `crates/maos-bin/tests/forget_floors_18_6.rs` (floors).
- §18-6 AC2 "Shared-tier principal writes are recorded where forget can find them (or forget walks the Shared tier itself — chosen and measured at creation)" → measured: partition stands 0 / content delete +38/+34 / lift partition +40/+36 (D-18-6-03, RULED A 2026-10-26).
- §18-6 "Why" (2)/(3) and AC1/AC2/AC5 carry dated in-place annotations (struck OLD, italic NEW); AC5 "or the Shared-tier cascade" → "or the any-writer scan (D-18-6-01)".
- R18-19 NEW gains the same re-anchors (forget block `:545-641` → `:545-662`). The stories row `:115` and the Kernel-Δ note `:40` carry the figure (ratified 2026-10-26).
- Routing audit `:103` (`epic-18:118-121`) → `:223-237`, not edited (outside the doc set).

**Dev-head correction, AC3.5 (integrator-approved 2026-10-26):** the live MCP cache contains invocation argument frames, not the old fetched claims. `input_frames` fetches the current claims and joins them to cached source keys on each pass. With the specified serve-once MCP fixture removing the canary, merely skipping cache purge cannot make the next live pass's body contain that old canary. The honest purge falsifier therefore asserts that scrubbed frame ids/source references no longer remain in the Researcher's cache; a unit-seeded pending cached claim also witnesses next-pass non-resurrection where reachable. Production semantics are not changed to manufacture a resurrection path. The live serve-once process leg remains required.

**Dev-head correction, AC1.2 (integrator-approved 2026-10-26):** `DistillateWriter::write_distillate` rejects effective raw sources whose writer pid differs from the citer (`adapter/distillate.rs:337-358`), even when the citer is a recipient. New cross-pid digest-of-digest authoring is therefore not a valid fixture. The lineage witness uses (1) a genuine same-pid digest-of-digest through `DistillateWriter`, and (2) a persisted historical cross-writer TL row inserted through the real TL adapter, explicitly labelled “historical row, not new valid authoring”, to prove the any-writer sweep reaches it. The citation authorization rule is unchanged.

**Dev-head correction, AC1 process composition (integrator-approved 2026-10-26):** ADR-064's existing selector refuses `MAOS_INFERENCE_MODE=replay` together with `--live` (`inference_mode.rs`, `ReplayLiveConflict`). The mandated combined command is therefore unreachable. ADR-064 remains unchanged: the real daemon-door journey uses the non-`--live`, bound seed-claim replay path (D-18-3-01); a separate root-library witness composes real mock MCP transport, replay inference, real TL/`DistillateWriter` and the door command adapter in-process for the MCP-sourced live re-distillation leg. No flag or production selector exception is introduced.

**Dev-head correction, next-pass proof (integrator-approved 2026-10-26):** no scheduled-repeat process claim — the idle watchdog fires once per quiet period, and a door reload reseeds the fixed seed claims. The daemon witness ends after real door forget/re-distillation and typed pending/hold checks; the root-library MCP witness and Researcher purge unit test prove the same instance's subsequent pass is clean. The process principal deliberately collides with built-in seed vocabulary (`noisy corpus.`; canary `seed-rag-noisy`); reloading re-journals compiled constant text, which forget cannot and need not erase. Main assessed this as a fixture artifact, not recreation of principal-derived data.

## Kernel and kloc pricing (E16-A7) — measured at creation

**Prototypes (throwaway, at `5495863e`, `cargo fmt --all`, reverted):** one shared out-of-kernel prototype (maos-domain `RedistillRequest`, the `ForgetOutcome::Erased.redistill` field, `principal_id_occurs`; maos-iac `principal_bearing_frames`, `distillates_citing`, `scrub_frame_body`, `insert_redistill_request`, `distillate_source_refs`) under eight kernel variants. Full figures and diff hashes: `18-6-flag-winston-decision.md` §Measured figures.

| Option | Kernel physical | Kernel tokei | Files |
|---|---:|---:|---|
| D-01 A (any-writer distillates) | 26418 → 26417 (−1) | 20643 → 20641 (−2) | `memory/mod.rs` |
| **D-01 B** (any kind, any writer) | **26384 (−34)** | **20623 (−20)** | `memory/mod.rs` |
| **+ D-02 A** (derived distillates) | **26401 (−17)** | **20637 (−6)** | `memory/mod.rs` |
| **+ D-04 A** (requests formed and journaled in the kernel) — **ruled stack (ratified grant)** | **26426 (+8)** | **20660 (+17)** | `memory/mod.rs` |
| D-04 B instead (pre-scrub sources only) | 26419 (+1) | 20653 (+10) | `memory/mod.rs` |
| D-01 C increment (value needles) over the stack | +16 | +14 | `memory/mod.rs` |
| D-03 B (Shared content delete), own | +38 | +34 | `memory/mod.rs`, `memory/shared.rs` |
| D-03 C (lift the Shared partition), own | +40 (+ export path +8–12 [INFERENCE]) | +36 | `memory/mod.rs`, `memory/shared.rs` |
| D-01 D / D-03 A / D-05…10 | 0 | 0 | none (by construction) |

**kloc at `5495863e`** (`kloc-check --json`, `passed: true`, `over_budget: []`, aggregate 185451; `alarm: true` is the pre-existing aggregate alarm):

| Crate | Measured / ceiling | 18-6 (measured by prototype / [INFERENCE]) | Consequence |
|---|---|---|---|
| maos-kernel-core | 20643 / 20643 (`xtask/kloc.toml:264`); pin 26418 (`kernel-core-baseline.toml:505`) | +17 tokei / +8 physical (recommended) | FLAG grant (D-01/02/04) |
| maos-bin | 28655 / 28655 (`:517`); Epic 18 named budget 28700 | +52–88 [INFERENCE] | **D-18-6-08** |
| maos-iac | 7979 / 7979 (`:406`) | +122 measured + 18–28 | measured request |
| maos-domain | 9848 / 9848 (`:425`) | +22 measured + 15–25 | measured request |
| maos-audit | 7473 / 7473 (`:409`) | +54–105 [INFERENCE] (FR4 shapes and tombstone disposition, v1 generator) | measured request |
| maos-cli | 6104 / 6856 (`:489`) | +0–10 | fits |
| maos-journey-test | 915 / 1104 (`:681`) | +0–15 (MockMcp serve-once, if needed in src) | fits |
| maos-spirit-abi / sdk | 1042 / 3000; 1085 / 3000 | 0 | — |
| aggregate | 185451; `_aggregate_hardfail` 185452 (`:718`) | prototype 185612 | re-pin = measured + 1 |

`spirits/*` is ungoverned (`xtask/src/kloc_check.rs:310-311`), so the Researcher's `on_consolidate` handler, cache purge and re-distillation go there. The maos-bin remainder is root composition: resolving the writer from `(boot_nonce, spirit_pid)` against this boot's SCB map, delivering the notice through the hook pump, journaling the outcome, and handling `redistill` on all three forget call sites. Binary size: release + strip 29,864,312 B at 18-5 (3,690,120 B under 32 MiB); re-measure at landing.

## Decisions — RULED 2026-10-26 by the operator (Lunarpulse)

**Ruling record.** D-18-6-01 **B**, 02 **A**, 03 **A**, 04 **A**, 05 **A**, 06 **A**, 07 **A**, 08 **A** (Epic 18 named maos-bin budget 28700 → **28770**), 09 **A**, 10 **A** — every row as recommended, for the rationale in its row. ◆ **FLAG-Winston kernel grant ratified (ADR-069 envelope):** D-01 B + 02 A + 04 A = **+8 physical / +17 tokei** — `check-kernel-baseline` 26418 → **≤ 26426** (95 files), `maos-kernel-core` 20643 → **≤ 20660**, exact measured figure at landing after `cargo fmt --all`, file `crates/maos-kernel-core/src/memory/mod.rs` **only**; any other kernel file or a figure above the grant returns as a FLAG request. Evidence and measured figures: `18-6-flag-winston-decision.md`. CR-n are creation defaults carried into the rulings.

**Out-of-kernel rows — measured requests at landing, STOP at dev above expected + 25 %** (expected = prototype measurement + creation forecast; a dev figure above the STOP line → stop and report the exact `kloc-check --json` figure to the operator before any further production edit or re-pin):
- **maos-bin:** ≤ **28770** (D-08 A, named budget); exact re-pin at landing; above 28770 → STOP, re-ask.
- **maos-iac** (7979, zero headroom): expected +122 measured + 18–28 [INFERENCE] = up to +150 → **STOP above +188 (8167)**.
- **maos-domain** (9848, zero headroom): expected +22 measured + 15–25 = up to +47 → **STOP above +59 (9907)**.
- **maos-audit** (7473, zero headroom): expected +54–105 [INFERENCE] → **STOP above +131 (7604)**.
- **maos-cli** (6104 / 6856) and **maos-journey-test** (915 / 1104): land exact inside their ceilings; reaching a ceiling → STOP.
- `_aggregate_hardfail` = final measured aggregate + 1; no speculative headroom.

**D-18-6-08 measured addendum, operator ruling 2026-10-26 (A, relayed by Main):** the formatted STOP was observed and reported before further production edits or re-pins. Exact figures are ratified with finishing allowance: **maos-bin ≤ 28940**, **maos-audit ≤ 7640**; Epic 18's named maos-bin budget **28770 → exact landed figure** (28916 measured at this ruling). Other STOP lines and the kernel grant are unchanged. Final pins remain exact, without unused allowance.

**maos-bin growth attribution, HEAD → formatted candidate (+258):** `hook_pump.rs` 778 → 937 (**+159**: `redistill_sink`, `finish_forget`, typed result storage, boot/pid selection, broadcast hook delivery, writer verification and outcome journaling); `main.rs` 12552 → 12601 (**+49**: bindings on boot/load/successor, offline forget/uninstall, daemon uninstall dispatch, terminal accounting); `operator_door.rs` 1653 → 1678 (**+25**: pump binding and daemon forget terminal dispatch/counts); `researcher_root.rs` 352 → 377 (**+25**: lazy `DeferredMcpPorts` lifetime wrapper, dropping operation-scoped adapters so audit drain is not pinned). Measured with the gate's per-file `tokei --output json --types Rust` `reports[].stats.code`; sum 159 + 49 + 25 + 25 = 258 (28658 → 28916).

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-18-6-01** | How far the TL scrub reaches (security-critical; kernel) | A — every writer's Distillates, id match (−1/−2) · **B — every frame of every kind and writer naming the principal, except `principal.forget`, `principal.forget.held`, `distillate.redacted`; Distillate → ADR-044 tombstone + marker; kinds 28/29 → SR-3 field scrub; other kinds → whole-body tombstone + non-personal `frame.redacted` marker (−34/−20)** · C — B + the principal's stored values (≥ 16 B) as needles (+16/+14) · D — kernel 0, the same sweep in the root on all three call sites | **B.** The index-keyed scan misses every unindexed writer and every non-distillate frame; the MCP-invocation frames that carry the principal to the Researcher are exactly such frames. One any-writer scan replaces two pid-keyed loops (the kernel shrinks). C finds id-less copies at the cost of a read of each forgotten value; D splits one cascade across three root call sites and three journal rows. | **RULED 2026-10-26 — B** |
| **D-18-6-02** | Derived distillates (kernel) | **A — after the scrub, every Distillate whose effective sources cite a scrubbed frame is tombstoned and marked (+17/+14)** · B — body match only (labelled) | **A.** Refs are flattened to raw frames (`distillate.rs:191-260`), so one pass finds every digest derived from a scrubbed source even when its body never names the principal. | **RULED 2026-10-26 — A** |
| **D-18-6-03** | The Shared clause | **A — the 13-5h partition stands (kernel 0); a door-level witness that no Shared row names the principal; non-principal rows naming one are a labelled limit** · B — delete every Shared row naming the principal (+38/+34) · C — lift the partition for Shared and erase indexed Shared rows (+40/+36, export +8–12) | **A.** The epic premise is stale: Shared already refuses principal namespaces, and 13-5h ruled that legitimate cross-Spirit coordination data must survive a forget. B erases that data on a substring; C reverses 13-5h, its partition test and two Reza legs. No production Shared writer exists. | **RULED 2026-10-26 — A** |
| **D-18-6-04** | Where the re-distillation request is formed (kernel) | **A — the kernel reads each redacted distillate's sources before the scrub, removes everything scrubbed in this cascade, journals a non-personal `distillate.redistill.requested` row and returns `redistill` on `ForgetOutcome::Erased` (+25/+23)** · B — the kernel returns pre-scrub sources; the root filters and journals (+18/+16) · C — kernel 0 (only with D-01 D) | **A.** The tombstone destroys the sources, so they can only be captured inside the cascade; journaling there gives one trigger for all three callers ("the kernel triggers", epic AC3). B moves the filtering to three root sites for 7 lines. | **RULED 2026-10-26 — A** |
| **D-18-6-05** | How the writer learns, and how live working memory is purged (security-critical; kernel 0) | **A — on daemon paths (door forget, door uninstall) the root sends every live compiled-in Spirit registered with the hook pump a typed non-personal `ForgetNotice { scrubbed_frame_ids, redistill }` via `fire_on_consolidate`; the writer is resolved by the root from `(boot_nonce, spirit_pid)`; outcome rows `distillate.redistilled` / `distillate.redistill.pending`; offline arms journal pending** · B — Spirit-pulled on the next pass · C — new IAC frame kind 34 | **A.** `on_consolidate` is the existing memory-curation hook with a production caller and a payload; native Spirits have no mailbox pump (18-5), so C still needs A or B; B leaves the cache live until the next pass, which may re-distil the erased data first. | **RULED 2026-10-26 — A** |
| **D-18-6-06** | What "pending" means | **A — terminal and typed: `reason ∈ {writer-not-live, no-distillation-path, no-surviving-sources, form-unsupported, refused:<typed>}`, counted in the forget terminal; no cross-boot resume** · B — re-dispatch when the writer's class is next admitted (maos-bin +30–50) | **A.** Pids are boot-local and stable identity is 20-3b's (`deferred-work.md:1270`); the epic asks for a typed pending outcome, never a silent skip. | **RULED 2026-10-26 — A** |
| **D-18-6-07** | Floor evidence | **A — new `gdpr-cascade-v1` (50 erased-only, cross-Spirit strata) + `gdpr-cascade-probe-v1` (100), seeded through the real adapters into a scratch `MAOS_HOME`, forgotten over the daemon door, probed across every readable store; v0 unchanged as the held/failed/not-found regression** · B — regenerate v0 in place · C — adapter-level erased subset (35) | **A.** "Production path" means the door; v0 and its Reza leg keep covering the other three outcomes; C cannot reach 50. | **RULED 2026-10-26 — A** |
| **D-18-6-08** | maos-bin budget | **A — Epic 18 named budget 28700 → 28770; exact re-pin; above → re-ask** · B — hold 28700; T0 returns a measured request | **A.** Forecast +52–88 [INFERENCE] × 1.3. The dispatch needs the scheduler and hook pump, which only the root holds; types, TL helpers and the handler live outside maos-bin. | **RULED 2026-10-26 — A** (maos-bin ≤ 28770) |
| **D-18-6-09** | Principal-id matching (security-critical) | **A — token-bounded (`principal_id_occurs`; neighbours not `[A-Za-z0-9@._+-]`), SQL `instr` prefilter + Rust confirmation** · B — raw substring (today) | **A.** The raw substring erases `aa@b.org`'s data when `a@b.org` is forgotten. Escaped/transformed copies are a labelled limit. | **RULED 2026-10-26 — A** |
| **D-18-6-10** | W1 non-atomicity (`deferred-work.md:449`) | **A — scrubs stay before deletions; witness that a re-run after an injected scrub failure completes; W1 annotated "narrowed, re-runnable" at landing, owner `v25-erasure-crash-reconciliation`** · B — route unchanged | **A.** Under D-01 B the scan does not depend on index rows, so a re-run is complete; no new code. | **RULED 2026-10-26 — A** |
| CR-1 | Scan | One SQL pass `instr(payload_redacted, ?)` over `transparency_log`, streamed, excluding the three lifecycle intents; Rust confirms with `principal_id_occurs`. Never load every body into memory (the prototype did; not acceptable). Ordered by `timestamp_ns`. | — |
| CR-2 | Request | `RedistillRequest { redacted_distillate_frame_id, writer_spirit_pid, writer_boot_nonce, surviving_source_log_ref }`: no principal id, no body bytes. `surviving_source_log_ref` excludes every frame scrubbed or derived-redacted in this cascade; empty → pending `no-surviving-sources` (I11 forbids an empty source set). | — |
| CR-3 | Every call site (18-1/18-3 lesson) | Door forget (`operator_door.rs:1341`): dispatch. Door uninstall (`run_uninstall_cascade`, daemon impl): dispatch. One-shot forget (`main.rs:6530`) and one-shot uninstall (`main.rs:6483`, cascade `:9492`): pending `writer-not-live` for each request, journaled before exit. The uninstall pattern `:9493-9497` binds `redistill`, never `..`. | — |
| CR-4 | Pids (root fixes pids) | The root resolves the target from the TL row's `(boot_nonce, spirit_pid)`: same boot nonce and a live SCB registered with the hook pump, else pending `writer-not-live`. The Spirit never receives a `Fn(pid)` or a pid to write as; it re-distils at the pid its binding minted, and the root checks the new Distillate row's `spirit_pid` equals the target before journaling `redistilled` (else `refused:writer-mismatch`). | — |
| CR-5 | Typed outcomes | The Researcher returns `Result<RedistillOutcome, ResearcherError>` per request; a refused pass, an unbound Researcher (`no-distillation-path`), a parse error and a hook `SkippedManifest`/quarantine/timeout map to a typed pending reason. Serde results in `src` are never `.unwrap/.expect/.unwrap_or*` (the prototype's `serde_json::to_vec(..).unwrap_or_default()` in `insert_redistill_request` is exactly what is forbidden). | — |
| CR-6 | Readers of tombstoned rows | `classify_fr4_row` (`fr4_classifier.rs:556-581`), `query_with_redaction`, sealed and trajectory export treat `{"redacted":true,…}` as a typed redacted row, never a schema error or panic. | — |
| CR-7 | Purge | The Researcher's `on_consolidate` with a non-empty payload decodes `ForgetNotice`, drops every cached frame whose id is in `scrubbed_frame_ids` (`mcp_recall.frames`, `seed_frames`), clears `last_output` if it cites one, then re-distils each request addressed to it. An empty payload (the `ConsolidationWatchdog`, `hook_pump.rs:751`) keeps today's behaviour. Re-distillation reads surviving sources by id through its `LogRecallPort`, re-runs `survey_with` through its bound path (bound: inference; ADR-016 exits keep `ProviderInfer` and `MemWrite` open under context exhaustion) and writes through its `DistillationPort`. | — |

## Acceptance Criteria

*(Epic §18-6 as corrected at creation and ruled 2026-10-26 (D-18-6-01 B, 02–10 A). One plant per control AC, `docs/testing/proof-contract.md:26`. Security-critical: AC1, AC2, AC3.)*

**Inherited deferred rows owned by this story:** none (`deferred-work.md` grep: zero owner rows). Adjacent rows are listed under *Closes*; D-18-6-10 decides W1's annotation at landing.

### AC1 — command: forget leaves nothing to find and regenerates what it redacted (D-01, 02, 04, 05, 09; security-critical)

1. Process leg (daemon-level), `crates/maos-journey-test/tests/journey_forget_18_6.rs`, keyless runner, scratch `HOME` with `maos init` (control.json → door):
   - **Indexed writer:** the 18-4 scene `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_STRICT=1 … maos run spirits/butler/manifest.toml --clock compressed:4d --acceptance tests/fixtures/j-butler/acceptance-21d-steady.jsonl` writes `principal:director-ada:butler.acceptance/v1` (Private, Markdown, indexed).
   - **Unindexed writer, live:** `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_STRICT=1 MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j-researcher/forget-redistill-18-6.json maos run spirits/researcher/manifest.toml --live` (no `--once`; the root hosts the door) with the `MockMcp` servers (`crates/maos-journey-test/src/lib.rs:278-376`, struct `:282`, was cited `:202-355`; used as in `journey_researcher.rs:50-106`) answering the survey once with a claim that names `director-ada` and a canary, then without it. One pass writes a Distillate embedding both; its McpInvocation source frame carries both.
   - `maosctl forget --principal director-ada` exits 0; its JSON lists the scrubbed frames, the redacted distillates, one `redistill` request per redacted distillate with its surviving sources, and the outcome of each.
   - The Researcher pid (same boot, live) receives the `ForgetNotice`; the TL then holds one `distillate.redistilled` row per request naming a NEW Distillate frame at the Researcher's pid whose body contains neither `director-ada` nor the canary and whose `source_log_ref` ⊆ the surviving set. The next scheduled pass writes no Distillate containing either.
   - 100 follow-up subject-access probes (`gdpr-cascade-probe-v1` subset for `director-ada` + controls) over every readable store — `maos_audit::subject_access_query` (`crates/maos-audit/src/lib.rs:1868`), a raw byte scan of `transparency_log.payload_redacted`, `shared_memory.value`, the Private spill tree under `MAOS_HOME` and the principal index — find the canary 0 times and `director-ada` only in `principal.forget`, `principal.forget.held` (none here) and `distillate.redacted` rows. A control principal's frames and distillates are untouched.
2. Library legs (`crates/maos-kernel-core/tests/forget_cascade_18_6.rs`, real adapters): a non-distillate frame of each of three kinds by an unindexed pid is tombstoned with a `frame.redacted` marker; a cost frame keeps its figures (SR-3); a digest-of-digest written by a second unindexed pid from a scrubbed source is redacted and gets its own request (D-02); `aa@b.org`'s frames survive forgetting `a@b.org` (D-09); a re-run after an injected scrub failure completes (D-10).
3. **Proven red (1 plant):** restore the index-pid filter on the scan → the process leg finds the canary in the Researcher's Distillate → red.

### AC2 — the cascade reaches every store; a legal hold still refuses, typed (D-01, 02, 03, 09; security-critical)

1. `forget_with_reason` scans per CR-1, scrubs per D-01 B and closes derived distillates per D-02 A; the Private delete, index delete, `principal.forget` journal, schema stamp and receipt (`memory/mod.rs:616-662`) are unchanged in order and content, the payload gaining nothing personal.
2. Shared (D-03 A): the AC1 leg asserts zero `shared_memory` rows naming the principal; `multi_backend_erasure_test` and the `gdpr-backend-partition`, `gdpr-regional-shared-verified-empty` legs stay green unchanged.
3. Legal hold: `maosctl forget --principal <p> --reason legal-hold` over the door → terminal 3, `principal.forget.held`, **no** scrub, no `redistill` request, no notice (witnessed in the AC1 binary with a second principal).
4. **Proven red (1 plant):** drop the D-02 closure → the derived Distillate keeps a scrubbed source and survives unmarked → red.

### AC3 — re-distillation is Spirit-authored, root-dispatched, pid-fixed; pending is typed (D-04, 05, 06, CR-2…CR-5, CR-7; security-critical)

1. `RedistillRequest` and `ForgetNotice` (maos-domain); `ForgetOutcome::Erased.redistill` (`#[serde(default)]`, old JSON still parses). Requests are journaled by the kernel cascade before dispatch.
2. The root (CR-3, CR-4) delivers the notice to every live compiled-in Spirit registered with the hook pump (`crates/maos-bin/src/hook_pump.rs`; no new maos-bin src file unless listed in `SCANNED_SOURCE_FILES`, `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:927`) and journals one outcome row per request.
3. The Researcher implements `on_consolidate` per CR-7 and is bound on every construction path (`researcher_root::construct` `:96-141`: boot arm, door `load`, hot-swap successor); its distillation port is the root's `DistillateWriter` (`bind` `:173-175`).
4. Witnesses: the AC1 process leg (live dispatch); a door-level leg in the same binary in which the writer was unloaded before forget → `pending: writer-not-live`; the one-shot arm (`MAOS_ONE_SHOT=forget`) → pending for every request; library: an unbound (deterministic) Researcher → `no-distillation-path`; a request whose sources were all scrubbed → `no-surviving-sources`; a malformed payload → typed refusal, no panic.
5. **Proven red (1 plant):** skip the purge of cached frames (CR-7) → the Researcher's next pass writes a Distillate containing the canary → red.

### AC4 — the floors are production evidence (D-07)

1. `gdpr-cascade-v1.jsonl` (50 erased-only scenarios: indexed Private; unindexed Distillate; raw MCP-invocation frame; derived Distillate; cross-boot pid reuse; multi-writer) and `gdpr-cascade-probe-v1.jsonl` (100), generated deterministically by `gen_gdpr_cascade` (`crates/maos-audit/src/bin/gen_gdpr_cascade.rs`), registered in `tests/corpora/MANIFEST` (sha256, item counts) and the coverage matrix (FR45, NFR-Aud-10).
2. `crates/maos-bin/tests/forget_floors_18_6.rs` seeds the scenarios into one scratch `MAOS_HOME` through the real adapters, boots one daemon, forgets each principal with `maosctl forget` over the door, and asserts 50/50 clean removal at the queryable surface, 50/50 redaction markers (ADR-044 marker for distillates, `frame.redacted` for other kinds), and 0/100 leakage over the AC1.1 store set. Writers are not live, so each request ends `pending: writer-not-live` (typed, counted).
3. v0 and its Reza leg (`gdpr-kernel-forget-outcome-corpus`) stay as the held/failed/not-found regression; any v0 assertion changed by D-01 B is updated with its reason recorded.
4. Docs: ADR-044 gains an 18-6 implementation note superseding Decision 3; `requirements-inventory.md:563` and the coverage-matrix notes say "delivered by 18-6" at landing.
5. **Proven red (1 plant):** generate a v1 scenario whose principal appears only in an unindexed writer's frame with the D-01 scan reverted → clean removal 49/50 → red.

### AC5 — proven red in CI (rule 11)

1. One hosted run on `ubuntu-24.04` (`ci/18-6-verify`, plant diff + sha256 in `18-6-evidence/ci/`) applies AC1.3's plant and reds the AC1 process leg; the run id is recorded. The other plants are local, with commands and outputs in `18-6-evidence/plants.md`.
2. The new suites run in CI: the journey binary in `journey-hermetic-tier-1` (`discipline.yml:3327`), `forget_floors_18_6` and `forget_cascade_18_6` beside the GDPR legs (`discipline.yml:4925-4940`) or as Reza legs with `--exact` anti-vacuity; any CI test-count pin over an enrolled binary is updated with the new count.

## Tasks / Subtasks

- [x] **T0 — Preflight at the dev HEAD.** D-18-6-01…10 RULED 2026-10-26 (all as recommended); tracker `ready-for-dev` → `in-progress`. Re-anchor: `git diff --stat 5495863e -- crates/maos-kernel-core/src/memory crates/maos-iac/src/adapter/{transparency_log.rs,distillate.rs} crates/maos-domain/src/memory.rs crates/maos-bin/src/{operator_door.rs,hook_pump.rs,researcher_root.rs,main.rs} spirits/researcher/src crates/maos-audit/src`. Re-measure `check-kernel-baseline` / `kloc-check --json` (expect 26418 / 20643 unless something landed; if the pin moved, the +8 / +17 grant applies over the landed values and is re-stated, never widened). Confirm the Researcher pid is registered with the hook pump on the boot arm, door `load` and successor. Watch the STOP lines in §Decisions throughout dev.
- [x] **T1 — Domain** (AC3.1): `RedistillRequest`, `ForgetNotice`, `RedistillOutcome`, `principal_id_occurs`, the `redistill` field.
- [x] **T2 — Bus** (AC1, AC2): maos-iac streamed `principal_bearing_frames` (CR-1), `distillates_citing`, `scrub_frame_body` (+ `frame.redacted` marker), `distillate_source_refs`, request and outcome writers; FR4 `WRITER_SHAPES` for every new `insert_frame_event` site (`crates/maos-audit/tests/fr4_classifier_16_2.rs` doorbell) and the tombstone disposition (CR-6).
- [x] **T3 — Kernel** (AC2.1, AC3.1): `forget_with_reason` per the ruled D-01/02/04 rows; exactly the ruled file and figure.
- [x] **T4 — Root** (AC3.2, CR-3, CR-4): door forget and door uninstall dispatch through the hook pump; one-shot forget/uninstall pending; uninstall pattern binds `redistill`; terminal JSON counts.
- [x] **T5 — Researcher** (AC3.3, CR-5, CR-7): `on_consolidate` handler, purge, re-distillation, typed outcomes.
- [x] **T6 — Corpus** (AC4.1): `gen_gdpr_cascade` v1 modes; files; MANIFEST; coverage matrix.
- [x] **T7 — Witnesses** (AC1–AC4): journey process leg + cassette (`provenance: seed`, real hashes, as 18-4 CR-3) + MockMcp serve-once; door/one-shot pending legs; kernel library legs; floors binary. Each binary ≤ 30 s Σ (`xtask/test-budgets.toml` default; no exception requested). Approved split process/root-library proof and historical-row correction recorded above.
- [x] **T8 — Plants and CI local deliverables** (AC1.3, AC2.4, AC3.5, AC4.5, AC5): every control proven red and byte-restored; checked-in hosted diff + checksum + workflow; local expected-red simulation passed. Integrator-approved handoff: hosted `ci/18-6-verify` run is the integrator's landing obligation, no hosted-run claim at review (checkbox in Dev Agent Record).
- [x] **T9 — Docs:** ADR-044 implementation note; inventory `:563`; coverage-matrix notes; `deferred-work.md:449` annotation per D-18-6-10; docs-site en + ko for `maosctl forget` output and re-distillation.
- [x] **T10 — Governance:** kernel re-pin set (FLAG record §Re-pin), crate ceilings exact, `_aggregate_hardfail` = measured + 1; `check-service-boundary`, `check-kernel-baseline`, `kloc-check`, `check-serde-error-handling`, `check-test-cost`, `check-corpus`, `check-reza-production-path`; release binary < 32 MiB.
- [x] **T11 — Regression and record:** focused suites (below), `cargo test -p maos-kernel-core --lib`; Dev Agent Record, File List, Change Log; tracker. Do not edit `intent-lineage-coverage-report.md` (revert if a run modifies it). No commit or push.

## Dev Notes

### What changes, by file

| Area | Files | Today → after |
|---|---|---|
| Domain | `crates/maos-domain/src/memory.rs:419-432` | no request type → `RedistillRequest`, `ForgetNotice`, `RedistillOutcome`, matcher |
| Bus | `crates/maos-iac/src/adapter/transparency_log.rs:1385-1674`; `adapter/distillate.rs:142-260` | index-pid queries → any-writer scan, generic scrub + marker, request/outcome writers |
| Kernel | `crates/maos-kernel-core/src/memory/mod.rs:545-662` | two index-pid loops → one scan + closure + requests |
| Root | `crates/maos-bin/src/operator_door.rs:1334-1365`; `hook_pump.rs:449-766`; `main.rs:2997,3500,6524-6544,9479-9534` | no dispatch → notice + outcome rows on every call site |
| Researcher | `spirits/researcher/src/lib.rs:545-720,1038-1118,1304-1345` | no `on_consolidate` → purge + re-distil |
| Audit | `crates/maos-audit/src/fr4_classifier.rs:523,556-581`; `src/bin/gen_gdpr_cascade.rs` | — → shapes, tombstone disposition, v1 corpus |
| Data | `tests/corpora/gdpr-cascade-v1.jsonl`, `gdpr-cascade-probe-v1.jsonl`, `tests/corpora/MANIFEST`, `tests/coverage-matrix.yaml:692-700,912-919`; `crates/maos-journey-test/cassettes/j-researcher/forget-redistill-18-6.json` | new |

### Current state that must be preserved

- **Legal hold** semantics and terminals (P29, `memory/mod.rs:457-539`; door terminal 3; one-shot exit 3); the per-principal forget serializer (`:444-447`, 16-5).
- **ADR-044** tombstone shape and append-only marker; SR-3 field scrub of cost/governance frames (cost figures kept, NFR-Cost-1).
- **13-5h** partition and its tests/legs (under D-03 A); `REGISTERED_ERASURE_BACKENDS` unchanged.
- **18-4**: Butler re-reads its principal record per decision (EC-1); the forget/restart witnesses in `butler_scene_18_4.rs` stay green.
- **18-3**: the Researcher's one construction path, per-pass mint, typed pass refusal, five-metric gate.
- **18-5**: context exits — re-distillation runs on `ProviderInfer` + `MemWrite`, which an exhausted Spirit keeps.
- **`ConsolidationWatchdog`** periodic `on_consolidate` with an empty payload.

### Guardrails

- The kernel never interprets content beyond the token-bounded id.
- No principal id or body bytes in requests, notices, markers or outcome rows.
- The root fixes pids; the Spirit never receives a `Fn(pid)`.
- No silent loss: every request ends `redistilled` or a typed pending row; no proceed-on-refusal.
- Kernel diff exactly the ruled file and figure; anything else returns as a FLAG request.
- One target dir; `env -u CI -u GITHUB_ACTIONS`; no cargo inside a `#[test]`; plants restored from byte-identical backups.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red |
|---|---|---|
| Any-writer reach (AC1) | restore the index-pid filter | canary in the Researcher Distillate |
| Derived closure (AC2) | drop D-02 | derived Distillate survives unmarked |
| Live purge (AC3) | skip CR-7 purge | next pass re-distils the canary |
| Floors (AC4) | revert the D-01 scan, unindexed scenario | 49/50 |

### Tests and commands

```bash
# local; one cargo process; CARGO_TARGET_DIR=/mnt/build/cargo
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-domain -p maos-iac -p maos-audit -p researcher
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core --test forget_cascade_18_6
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --lib
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test forget_floors_18_6 --test erasure_uninstall_13_5b --test butler_scene_18_4
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-journey-test --test journey_forget_18_6 --test journey_researcher
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-audit --test gdpr_cascade_corpus_test --test gdpr_cascade_test --test multi_backend_erasure_test --test erasure_smoke_test --test erasure_sla_test --test fr4_classifier_16_2
cargo run -q --locked -p xtask -- check-kernel-baseline && cargo run -q --locked -p xtask -- check-service-boundary && cargo run -q --locked -p xtask -- kloc-check --json
cargo run -q --locked -p xtask -- check-reza-production-path && cargo run -q --locked -p xtask -- check-corpus && cargo run -q --locked -p xtask -- check-test-cost && cargo run -q --locked -p xtask -- check-serde-error-handling
cargo fmt --all -- --check
```

### Review plan (proof contract `:31-67`)

- **Models/tiering:** difficult story — Blind, Edge, Acceptance on opus-5.5; Test Infrastructure on glm-5.3; dev/finisher gpt-6.1-sol never reviews its own code.
- **Security layer (AC1–AC3):** scan reach and the lifecycle-intent exclusion; over-erasure (D-09); derived closure; nothing personal in requests/notices/rows; pid fixing and the writer-mismatch check; purge before re-distillation; typed pending on every path; legal hold untouched.
- **FLAG-Winston:** kernel diff = `memory/mod.rs`, figure exact.
- **Daemon-level witnesses** for the door, uninstall and one-shot paths.

### Previous-story intelligence

- **18-1 / 18-3:** a seam is bound on every construction path (D-18-1-R1, D-18-3-R1/R2); refused passes are typed; `researcher_root::{construct, bind}`; the Researcher distils only when bound.
- **18-4:** the principal port, Markdown durability, the forget-between-decisions witness (EC-1), TL attestations carry nothing personal (D-18-4-R1); pid reuse is 20-3b's.
- **18-5:** native Spirits have no mailbox pump; sync-port emission must not drop futures; FR4 doorbell for new writers; new maos-bin src files enter the cohort roster.
- **18-8:** `MockMcp` and the fixture conventions.
- **13-5h / 13-5i / 16-5:** Shared partition; Private spill-tree walk; legal-hold serializer.

### Git intelligence

| Commit | Content |
|---|---|
| `5495863e` | 18-5 review patches; kernel 26418 / 20643 |
| `be12ca0b` | 18-5 dev pass |
| `79f49709` | 18-4 review patches (principal record re-read per decision) |
| `b50ea844` | 18-4 dev pass (principal ledger) |
| `b96d123a` | 18-3 review (one Researcher construction path) |

None touched `forget_with_reason`, `distillate_frames_for_pids` or the scrub primitives since 16-5.

### References

- Epic `epic-18-spirits-that-think-w3.md`: §18-6 `:223-237` (creation corrections `:227`), stories row `:115`, R18-19 `:88`, R18-20 `:89`, Kernel-Δ `:32` (18-6 grant `:40`), kloc ruling `:56`, rules `:58-60`, Dependencies `:268,274`.
- `prd/functional-requirements.md:102`; `prd/non-functional-requirements.md:70,72,73`; `requirements-inventory.md:563`; `epic-17…w2.md:774`.
- `docs/adr/ADR-026-principal-memory-namespace.md:12`; `ADR-044-…md:20-46`; `ADR-069-…md:20-25,35-46`; `docs/testing/proof-contract.md:10-29,31-67`.
- `routing-audit-2026-10-22.md:3,71,103`; `13-5h-shared-tier-principal-partition.md`; `18-6-flag-winston-decision.md`.

## Dev Agent Record

### Story context creation

Created 2026-10-26 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter contract) at `5495863e`, clean tree (an untracked `18-5-evidence/ci/run-40267145141-excerpt.log` from a concurrent 18-5 record pass was left untouched).

**Inputs:** direct re-reads of every decision-bearing range; `check-kernel-baseline` (PASSED 26418 / 95) and `kloc-check --json` (`passed: true`, aggregate 185451) at HEAD.

**Prototypes** (one cargo process; fmt-clean; kernel `cargo check --locked` clean):
- D-01 A −1/−2; D-01 B −34/−20; + D-02 A −17/−6 net; + D-04 A +8/+17 net (recommended); D-04 B +1/+10 net; D-01 C +16/+14 increment; D-03 B +39/+35 (own +38/+34); D-03 C +41/+37 (own +40/+36).
- Shared out-of-kernel prototype: maos-iac +122, maos-domain +22.
- `cargo check --locked --workspace` with the recommended stack: one ripple, the uninstall arm's exhaustive pattern (`main.rs:9493-9497`). `check-service-boundary` PASSED, 0 violations.

**Revert:** `git checkout -- crates`; `git diff --quiet HEAD -- crates xtask spirits Cargo.lock Cargo.toml` exit 0; `check-kernel-baseline` PASSED 26418 / 95. `intent-lineage-coverage-report.md` not modified.

### Agent Model Used

openai-codex/gpt-6.1-sol (development); anthropic/claude-opus-5-5 (creation).

### Debug Log References

- Preflight at dev HEAD: `check-kernel-baseline` PASSED, 26418 physical / 95 files; `kloc-check --json` passed, aggregate 185459, kernel 20643, maos-bin 28658, maos-iac 7984, maos-domain 9848, maos-audit 7473. Landed 18-5 differences preserve the ratified kernel grant.
- **Historical STOP, lifted before continuing:** formatted candidate maos-bin 28916 > original28770 and audit7623 > original7604; exact evidence `18-6-evidence/kloc-stop.log`. Operator A ruling2026-10-26 ratified bin≤28940/audit≤7640. No production edit or re-pin occurred between STOP and ruling.
- Initial fixture/compile corrections are recorded, not misrepresented as control reds: cross-pid citation was unauthorized; a test writer missed `admit_for_consumer`; SQLite JSON must cast payload BLOBs to TEXT; compact/colon frame ids must compare through the existing decoder; v1 JSON key order must remain stable under serde feature unification. All corrected; v0 corpus unchanged. Final proof: `broad-final-green.log`, `extra-reader-regressions.log`, `kernel-lib.log`, `xtask-ratchets.log`.
- STOP lifted by the operator's 2026-10-26 A ruling relayed by Main: maos-bin ≤28940, audit ≤7640; exact re-pins at landing. Formatted maos-bin +258 attribution: hook_pump +159, main +49, operator_door +25, researcher_root +25 (functions and old/new figures in D-18-6-08 measured addendum). Test fixture API corrections applied after this ruling, using a genuine DistillateWriter receipt for the wrong-writer witness.

### Completion Notes List

- **AC1:** process `live_forget_redistills_and_never_resurrects_then_pending_is_typed`, root-library `mcp_sourced_forget_scrubs_unindexed_writer_and_redistills_live`, and 7 kernel cascade controls green. Any-writer index-filter plant red on real seed canary; byte-restored green. Approved process/root-library/seed-vocabulary proof split is explicit above, not a claim of repeated daemon idle.
- **AC2:** any-kind/unindexed, same-pid flattened digest chain, labelled historical cross-writer attribution, SR-3 figures, token-boundary over-erasure, injected scrub-failure retry, Shared partition, and legal-hold controls green. Derived-closure plant red (1 vs2 redactions), restored green.
- **AC3:** all four caller routes, boot/load/successor bindings, boot/pid fixing and wrong-writer refusal, weak result sink/audit drain, no-distillation-path/no-survivors/malformed notice outcomes, live purge and same-instance next pass green. Cache-purge plant red, restored green; actual kernel compact ids covered.
- **AC4:** `fifty_door_cascades_and_one_hundred_all_store_probes` green:50/50 removal,50/50 markers,0/100 leakage; SHA-pinned v1 generator reproduces frozen bytes, v0 held/failed/not-found regression green. Shared index-filter plant fails fast on first unindexed leak (not a fabricated49/50 aggregate).
- **AC5 local:** checksum/apply/build/expected-red/restore/green simulation PASS; workflow/diff/sha checked in. Four AC control proofs, three distinct mutations (AC1/AC4 share identical scan filter). Shared scan/CI full wall171.16s; AC2/AC3 command runtimes and conservative full-cycle timing bounds, including setup-selector failures, are honestly recorded in `18-6-evidence/plants.md`.
- **Final focused regressions:**2077 passed/23 skipped (9 focused crates incl.kernel;76.489s nextest,135.34s command wall); remaining keyword-reader set94 passed/38 skipped (32.88s wall); kernel lib258 passed (4.99s wall); xtask ratchets65 passed. New/expanded binary Σtimes: hook_pump20.083s, root-library1.785s, floors15.239s, journey5.342s, kernel cascade3.178s, domain0.006s, all≤30s.
- **Exact governance:** kernel26418 →26407 physical (-11),20643 →20650 tokei (+7),95files, memory/mod.rs only; full baseline/HISTORY/set_hash/digest/literals/ceiling/RATIFIED set moved together. Bin28658 →28914 (+256; hook_pump+157/main+49/operator_door+25/researcher_root+25), audit7473 →7625, IAC7984 →8080, domain9848 →9887, journey915 →946, CLI6104 unchanged; aggregate185459 →186040, hardfail186041. Operator allowance is not unused headroom. Kernel surface/class table unchanged.
- **Gates:** kloc-check, check-kernel-baseline, check-env-contract, check-exit-commands, check-service-boundary, check-test-cost, check-corpus, coverage-matrix, check-serde-error-handling, check-dev-record-completeness, check-reza-production-path, gen-abi-docs --check and fmt check PASS; Korean canonical coverage100% and glossary PASS through their exact node scripts (npm executable absent, no packages installed). Release+strip29,968,248B<32MiB (3,586,184B headroom).
- **Advisory limits of local gates:** coverage-matrix exits0 while reporting existing NFR-Meta-3 registry/phase warnings. Reza reports PASSED in evidence-advisory mode, but explicitly NOT_PROVEN for four absent live-Postgres successors (persisted datname/live database, collective partition, collective erase, Spirit route/tenant audit stage2). No local live-Postgres product proof is claimed; hosted substrate CI owns those existing legs.
- **Budget observation, integrator-dispositioned:** optional `check-test-budget` over the partial focused junit reported existing `maos-iac::participant_recall_17_8` Σ34.951s plus absent-binary exception rows. No touched recall SQL and no exception added. Main ruled CI's workspace junit authoritative (baseline landing40273714381 passed); partial-junit absent rows are not findings. See gate log, do not claim this optional local invocation passed.
- Protected `intent-lineage-coverage-report.md` is byte-identical to its pre-run backup. No commit or push. W1 remains narrowed/re-runnable, true cross-store atomicity owned by `v25-erasure-crash-reconciliation`; other explicit scope limits remain.
- [ ] **AC5 hosted run:** integrator, at landing — record hosted `ci/18-6-verify` run id at the done flip; no hosted-run claim at review (Main-approved disposition, same as18-1…18-5).

### File List

- `.github/workflows/discipline.yml`
- `_bmad-output/implementation-artifacts/18-6-fr45-forget-redistillation-and-cascade-closure.md`
- `_bmad-output/implementation-artifacts/18-6-evidence/`
- `_bmad-output/implementation-artifacts/deferred-work.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/planning-artifacts/epics/requirements-inventory.md`
- `_bmad-output/implementation-artifacts/18-6-flag-winston-decision.md`
- `_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md`
- `crates/maos-domain/src/memory.rs`
- `crates/maos-domain/tests/forget_cascade_18_6.rs`
- `crates/maos-iac/src/adapter/transparency_log.rs`
- `crates/maos-kernel-core/src/memory/mod.rs`
- `crates/maos-kernel-core/tests/forget_cascade_18_6.rs`
- `crates/maos-audit/src/bin/gen_gdpr_cascade.rs`
- `crates/maos-audit/src/fr4_classifier.rs`
- `crates/maos-audit/src/lib.rs`
- `crates/maos-audit/tests/fr4_classifier_16_2.rs`
- `crates/maos-bin/src/hook_pump.rs`
- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/src/operator_door.rs`
- `crates/maos-bin/src/researcher_root.rs`
- `crates/maos-bin/tests/hook_pump_17_4.rs`
- `crates/maos-bin/tests/forget_floors_18_6.rs`
- `crates/maos-bin/tests/j_researcher_survey_18_3.rs`
- `crates/maos-journey-test/src/lib.rs`
- `crates/maos-journey-test/tests/journey_forget_18_6.rs`
- `crates/maos-journey-test/cassettes/j-researcher/forget-redistill-18-6.json`
- `spirits/researcher/src/lib.rs`
- `tests/harness/forget_18_6.rs`
- `tests/corpora/gdpr-cascade-v1.jsonl`
- `tests/corpora/gdpr-cascade-probe-v1.jsonl`
- `tests/corpora/MANIFEST.toml`
- `tests/coverage-matrix.yaml`
- `docs/adr/ADR-044-gdpr-article-17-distillate-redaction.md`
- `docs-site/docs/run-maos.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md`
- `xtask/kernel-core-baseline.toml`
- `xtask/kloc.toml`
- `xtask/tests/kernel_pin_content_hash_16_0.rs`
- `xtask/tests/recovery_lane_ceiling_rule.rs`

### Change Log

- 2026-10-26: Story created (backlog; D-18-6-01…10 PROPOSED) at `5495863e`. Kernel priced by reverted prototypes (recommended +8 physical / +17 tokei); epic §18-6, R18-19 and the stories row corrected in place; FLAG record `18-6-flag-winston-decision.md`.
- 2026-10-26: RULED by the operator (Lunarpulse), relayed by Main: D-18-6-01 B, 02–10 A (all as recommended); kernel grant +8 physical / +17 tokei ratified (`memory/mod.rs` only); maos-bin named budget 28700 → 28770; out-of-kernel STOP lines written into §Decisions. Status → `ready-for-dev`. Docs only, no cargo.
- 2026-10-26: Development by openai-codex/gpt-6.1-sol; implementation and witnesses authored, domain 2/2 intermediate green, AC2 closure control proven red and byte-restored. Mandatory formatted kloc STOP: maos-bin 28916 > 28770 and maos-audit 7623 > 7604. No re-pin or completion claim; status remains `in-progress` pending operator ruling.
- 2026-10-26: Operator A ruling lifts the measured STOP: maos-bin ≤28940 / maos-audit ≤7640, exact landing pins, per-file +258 root attribution recorded; implementation verification resumes.
- 2026-10-26: Completed local implementation/proof and exact governance pins after the ratified STOP addendum. Focused2077+94green, kernel lib258, ratchets65; all named gates PASS, AC1–AC4red/restored and AC5local simulationPASS. Approved premise corrections and hosted landing handoff recorded. Story/tracker → `review`; no commit/push.
