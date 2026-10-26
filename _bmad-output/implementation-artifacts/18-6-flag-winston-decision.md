# 18-6 FLAG-Winston decision record

**Date:** 2026-10-26
**Status:** `ruled` — D-18-6-01…10 ruled by the operator (Lunarpulse) 2026-10-26, each as recommended (§Rulings). ◆ Kernel grant **+8 physical / +17 tokei** (26418 → ≤ 26426, 20643 → ≤ 20660, `memory/mod.rs` only) ratified, ADR-069 envelope. Story → `ready-for-dev`.
**Story:** `18-6-fr45-forget-redistillation-and-cascade-closure` (`implementation-artifacts/18-6-fr45-forget-redistillation-and-cascade-closure.md`), created at HEAD `5495863e` (18-0/18-1/18-8/18-2/18-3 done; 18-4 and 18-5 implemented and pushed, their code treated as landed).
**Authority:** E16-A7; ADR-069 (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:20-25` lists 18-6 among the unpriced ◆ stories, `:35-46` envelope ≤ 23,000 tokei, alarm 22,000, zero headroom, every line a measured grant); RECOVERY-LANE CEILING RULE (`xtask/kloc.toml:50-93`) for the rows outside the kernel; proof contract (`docs/testing/proof-contract.md:10-29,60-67`); D-17-9-PF-R5 A (the ruling that created this story, `epic-17-workers-and-third-party-form-w2.md:774`).
**Model:** `anthropic/claude-opus-5-5` (bmad-create-story, lighter creation contract: no party-mode preflight, no non-author validation pass).

## Decision

| ID | Decision | Status |
|---|---|---|
| **D-18-6-01** | **How far the cascade's Transparency-Log scrub reaches (AC1, AC2; security-critical; kernel).** Today the scan is keyed by the principal index: only Distillate (`memory/mod.rs:556-575`) and cost/governance frames (`:577-614`) written by pids that hold an index row are scanned, so a distillate or a raw source frame written by any other Spirit escapes. **A:** every writer's Distillate frames (drop the pid filter), id match; cost/governance as today. Measured **26417 (−1) / 20641 (−2)**. **B (recommended):** every frame of every kind and writer whose body names the principal, except the erasure-lifecycle rows that must keep naming it (`principal.forget`, `principal.forget.held`, `distillate.redacted`); Distillate → ADR-044 tombstone + marker; cost/governance (kinds 28/29) → SR-3 field scrub (figures kept); any other kind → whole-body tombstone + a non-personal `frame.redacted` marker appended by the maos-iac writer. One scan replaces the two pid-filtered loops, so the kernel shrinks: measured **26384 (−34) / 20623 (−20)**. **C:** B plus the principal's own stored values (≥ 16 bytes, read before deletion) as extra needles, so data copied without its id is found: **+16 / +14 over B** (measured over B+02 A+04 A). **D:** kernel 0: B's sweep runs in the root after `forget_with_reason` on each of the three call sites (door `operator_door.rs:1341`, one-shot `main.rs:6530`, uninstall `main.rs:9492`); not prototyped (by construction). | **ruled B** (Lunarpulse 2026-10-26) |
| **D-18-6-02** | **Derived distillates (AC2; kernel).** A distillate's `source_log_ref` is flattened to raw frames (`crates/maos-iac/src/adapter/distillate.rs:191-260`), so a digest written from a scrubbed source carries no trace of the principal in its body yet derives from the erased data. **A (recommended):** after the D-01 scrub, every Distillate whose effective sources cite a scrubbed frame is also tombstoned and marked (one pass suffices: refs are raw). **+17 / +14 over D-01 B**, measured. **B:** body match only; derived digests are a labelled limit. | **ruled A** (Lunarpulse 2026-10-26) |
| **D-18-6-03** | **The Shared-tier clause (AC2).** The epic premise "only Private writes record an index row; Shared at `:801`" is stale: since 13-5h (`dd4a908e`) a `Principal` namespace is refused outside the Private tier on write, read and scan (`memory/mod.rs:182-202`; calls `:784`, `:858`, `:912`), proven by `multi_backend_erasure_test.rs:280-326` and the `gdpr-backend-partition` leg (`xtask/src/check_reza_production_path.rs:530`). No Spirit-facing Shared surface exists (`Ctx`/WIT expose no memory tier; no production `MemoryTier::Shared` writer outside `memory/mod.rs`). **A (recommended): the partition stands**, kernel 0; AC2 is discharged by a door-level witness that no Shared row names the principal after forget, and non-principal Shared rows naming a principal are a labelled limit (13-5h / ADR-026 operational retention). **B:** forget deletes every Shared row whose value names the principal, any namespace: measured **+38 / +34** (`memory/shared.rs` + `mod.rs`); erases legitimate coordination data 13-5h ruled must survive. **C:** lift the partition for Shared: index principal-namespaced Shared writes and erase them in the cascade: measured **+40 / +36**, plus `export_redactable` (reads Private only, `:966-1004`) +8–12 [INFERENCE]; reverses 13-5h, its partition test and two Reza legs. | **ruled A** (Lunarpulse 2026-10-26) |
| **D-18-6-04** | **Where the re-distillation request is formed (AC3; kernel).** A scrubbed distillate loses its `source_log_ref` to the tombstone (`transparency_log.rs:1388-1415`), so the surviving sources must be read before the scrub, inside the cascade. **A (recommended):** the kernel captures each redacted distillate's sources, removes every frame scrubbed in this cascade, journals one non-personal `distillate.redistill.requested` row per redaction and returns `redistill: Vec<RedistillRequest { redacted_distillate_frame_id, writer_spirit_pid, writer_boot_nonce, surviving_source_log_ref }>` on `ForgetOutcome::Erased`. **+25 / +23 over D-01 B + 02 A**, measured. **B:** the kernel returns the pre-scrub sources only; the root filters, journals and dispatches: **+18 / +16**, measured; three root call sites must each do the filtering. **C:** kernel 0, only with D-01 D. | **ruled A** (Lunarpulse 2026-10-26) |
| **D-18-6-05** | **How the request reaches the writer, and how live working memory is purged (AC3; security-critical; kernel 0).** A live Researcher caches recalled frames and its last survey in process memory (`spirits/researcher/src/lib.rs:1054-1070` `mcp_recall.frames`, `:1076-1097` `seed_frames`, `last_output`), so its next pass would re-distil the erased data. **A (recommended):** on the daemon paths (door forget, door uninstall) the root sends every live compiled-in Spirit registered with the hook pump a typed, non-personal `ForgetNotice { scrubbed_frame_ids, redistill }` through the existing `HookDispatcher::fire_on_consolidate(target, payload)` (`crates/maos-kernel-core/src/scheduler/hook_dispatch.rs:336-343`), with the writer resolved by the root from the TL row's `(boot_nonce, spirit_pid)` against this boot's live SCB map. The Spirit purges cached frames by id and re-distils through its own DistillationPort; the root journals `distillate.redistilled` or a typed `distillate.redistill.pending`. Offline arms (one-shot forget, one-shot uninstall) journal pending. **B:** Spirit-pulled: Spirits recall `requested`/marker rows on their next pass; no purge until then. **C:** a new typed IAC frame kind (34) to mailboxes; native in-proc Spirits have no mailbox pump (18-5 D-18-5-04 finding), so B or A is still needed for them. | **ruled A** (Lunarpulse 2026-10-26) |
| **D-18-6-06** | **What "pending" means (AC3).** **A (recommended):** terminal and typed: `distillate.redistill.pending { redacted_distillate_frame_id, reason }` with `reason ∈ { writer-not-live, no-distillation-path, no-surviving-sources, form-unsupported, refused:<typed error> }`, counted in the forget terminal output; no automatic resume across boots (pids are boot-local; stable identity is `20-3b-gate-retirement-and-coverage-generator`'s, `deferred-work.md:1270`). **B:** the root re-dispatches pending requests when a Spirit of the writer's class is next admitted (class resolved from the lifecycle journal), maos-bin +30–50 [INFERENCE]. | **ruled A** (Lunarpulse 2026-10-26) |
| **D-18-6-07** | **Floor evidence (AC4).** `gdpr-cascade-v0` mixes 35 erased / 9 held / 5 not-found / 1 failed scenarios and replays at adapter level (`crates/maos-audit/tests/gdpr_cascade_corpus_test.rs`). **A (recommended):** new `gdpr-cascade-v1` (50 erased-only scenarios across strata: indexed Private, unindexed distillate, raw source frame, derived distillate, cross-boot pid reuse, multi-writer) + `gdpr-cascade-probe-v1` (100), seeded through the real adapters into a scratch `MAOS_HOME`, then forgotten through the daemon door (`maosctl forget`) and probed across every readable store; registered in `tests/corpora/MANIFEST`; v0 kept unchanged as the held/failed/not-found regression. **B:** regenerate v0 in place (erased-only), moving the other outcomes to a separate set; re-pins v0's sha and its Reza leg. **C:** keep adapter-level replay, erased-only subset of v0 (35, not 50) — does not meet "50 on the production path". | **ruled A** (Lunarpulse 2026-10-26) |
| **D-18-6-08** | **maos-bin kloc (budget).** maos-bin 28655 measured; Epic 18 named budget 28700 (18-5 D-09 addendum). Forecast +52–88 [INFERENCE]: door dispatch +12–20, `HookPump` notice delivery + target resolution +25–40, door handle to the pump +5–10, one-shot pending rows +4–8, uninstall arm +6–10. **A (recommended):** raise the named budget 28700 → **28770** (upper forecast × 1.3, rule 6); exact re-pin at landing; above → re-ask. **B:** hold 28700 (45 left); T0 prototypes the root wiring and returns a measured request before dev proceeds. Homes outside maos-bin are taken where correct: types in maos-domain, TL readers/writers in maos-iac, the handler in `spirits/researcher` (ungoverned); the dispatcher needs the scheduler and hook pump, which only the root holds. | **ruled A** (Lunarpulse 2026-10-26; maos-bin ≤ 28770) |
| **D-18-6-09** | **Principal-id matching (AC2; security-critical).** Today a raw substring (`memory/mod.rs:562-566`) scrubs `aa@b.org`'s distillate when `a@b.org` is forgotten (over-erasure of another principal's data). **A (recommended):** token-bounded match (`maos_domain::memory::principal_id_occurs`: the bytes either side are not `[A-Za-z0-9@._+-]`), SQL `instr` prefilter then Rust confirmation; JSON-escaped or transformed copies are a labelled limit. **B:** keep raw substring (no over-erasure fix). | **ruled A** (Lunarpulse 2026-10-26) |
| **D-18-6-10** | **The non-atomic cascade (W1, `deferred-work.md:449`, no owner cue).** 18-6 adds steps to the same unsynchronised sequence (TL scrub → Private delete → index delete → journal). **A (recommended):** keep every scrub before the deletions (as today) and, because the D-01 B scan no longer depends on index rows, witness that a second `forget` after an injected scrub failure completes and leaves nothing (no code beyond the test); at landing W1 is annotated "narrowed by 18-6 (re-runnable), true atomicity still open", owner `v25-erasure-crash-reconciliation`. **B:** route W1 unchanged to `v25-erasure-crash-reconciliation` at landing, no witness. | **ruled A** (Lunarpulse 2026-10-26) |

## Measured figures (E16-A7)

Measured in place at `5495863e` (clean tree) by throwaway prototypes in the workspace, after `cargo fmt --all`, with `cargo run -q --locked -p xtask -- check-kernel-baseline` (physical) and `-- kloc-check --json` (tokei 14.0.0). All runs used `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`, the only cargo process on the host. Each kernel prototype passed `cargo check --locked -p maos-kernel-core`; with the recommended stack applied, `cargo check --locked --workspace` failed only on the uninstall arm's exhaustive `ForgetOutcome::Erased` pattern (`crates/maos-bin/src/main.rs:9493-9497`, the third forget caller), and `check-service-boundary` PASSED with 0 violations (no kernel public-surface change). All prototypes were then **removed** with `git checkout -- crates`; `git diff --quiet HEAD -- crates xtask spirits Cargo.lock Cargo.toml` exited 0 and `check-kernel-baseline` PASSED 26418 / 95 files.

**Shared prototype content (out of kernel, the same for every kernel option):**
- maos-domain (+22 tokei): `RedistillRequest`; `ForgetOutcome::Erased.redistill` (`#[serde(default)]`); `principal_id_occurs` (token-bounded match).
- maos-iac (+122 tokei): `PrincipalBearingFrame`; `TransparencyLogAdapter::{principal_bearing_frames, distillates_citing, frames_where, scrub_frame_body, insert_redistill_request}`; `adapter::distillate::distillate_source_refs`. Apportioned: D-01 B core ≈ 83, D-02 A ≈ 15, D-04 A ≈ 14, source-ref parser ≈ 9 (shared by 02/04).
- Every kernel figure below includes one line (`redistill: Vec::new()` or `redistill,`) for the domain field.

| Instrument (HEAD 26418 / 20643) | Physical (`check-kernel-baseline`) | Tokei (`kloc-check`) | Files |
|---|---:|---:|---|
| D-01 A (any-writer distillates) | 26417 (−1) | 20641 (−2) | `memory/mod.rs` |
| **D-01 B** (all kinds, any writer; SR-3 folded in) | **26384 (−34)** | **20623 (−20)** | `memory/mod.rs` |
| **D-01 B + D-02 A** (lineage) | **26401 (−17)** | **20637 (−6)** | `memory/mod.rs` |
| **D-01 B + D-02 A + D-04 A** (recommended) | **26426 (+8)** | **20660 (+17)** | `memory/mod.rs` |
| D-01 B + D-02 A + D-04 B | 26419 (+1) | 20653 (+10) | `memory/mod.rs` |
| D-01 C increment (value needles), over the recommended stack | 26442 (+16) | 20674 (+14) | `memory/mod.rs` |
| D-03 B over HEAD (Shared content delete) | 26457 (+39; +38 own) | 20678 (+35; +34 own) | `memory/mod.rs`, `memory/shared.rs` |
| D-03 C over HEAD (lift the Shared partition) | 26459 (+41; +40 own) | 20680 (+37; +36 own) | `memory/mod.rs`, `memory/shared.rs` |
| D-01 D / D-03 A / D-05…D-10 | 0 | 0 | none (by construction) |

D-03 B and C touch a disjoint hunk and file and were measured over HEAD; added to the recommended stack they are additive (+38/+34 or +40/+36, and C re-adds the index lookup D-01 B removes, +1). maos-bin was 28655 in every prototype (no root wiring prototyped; D-08 is a forecast). Prototype aggregate with the recommended stack: 185612 (HEAD 185451).

**Envelope:** under the recommended rows, tokei 20643 → **20660**: **2,340** left to 23,000 and **1,340** to the 22,000 alarm. No `ENVELOPE ALARM`. D-01 C on top leaves 2,326; D-03 B on top 2,306; D-03 C on top 2,303.

Prototype diffs are not committed. sha256: shared `ca9f056e65602568b232aed0de91c513c861700464e3f003d80e810e2374dfae`; D-01 A `cf44fd67a61272f000d07d14cfa3eea470e9fcea607a11b5839ea38d325ed969`; D-01 B `298c07c7675a4fe6f97be200442609b502c5fc6a4dd3f751729c26a400cdbde8`; B+02 A `13ad86dd30f4ee1ebe5241df43f5e7d020ac545493ad1e67fff6de588a48775d`; recommended `e4c08dbf749b03e4e023b18154962fa311e348b02956e9ec19a39f2059f4e041`; + value needles `93023ae689da609c8aabd7897c0856dc9be3ca59ad3d1166114442eb62aaaf7e`; 04 B `99ba8f86691324fe394dd4b612254be2d4b4b1f0b1bfb5446eb5faf7ea5a982d`; 03 B `ec8bee9b7f5e02c4446093bf59544f8498d0765c2f7d035a5068b6c41329e7d7`; 03 C `f17845cd8d3c8644a081aed9816c996cd12c60a5a9c48512789385e49361dae9`.

**Not in the prototypes, forecast [INFERENCE]:**
- maos-iac: the `frame.redacted` marker inside `scrub_frame_body` (+6–10); outcome-row writers `distillate.redistilled` / `distillate.redistill.pending` (+12–18); SQL `instr` prefilter replacing the prototype's full-body load (±0).
- maos-domain: `ForgetNotice` payload and `RedistillOutcome` (+15–25).
- maos-audit: `WRITER_SHAPES` dispositions for 3–4 new writer sites (+9–15); a typed disposition for tombstoned bodies in `classify_fr4_row`'s payload matching (`crates/maos-audit/src/fr4_classifier.rs:556-581`; +5–10); `gen_gdpr_cascade` v1 corpus/probe modes (+40–80). Total +54–105 (zero headroom at 7473).
- maos-bin: D-08 (+52–88). maos-cli: forget terminal summary (+0–10; 6104 / 6856 fits).
- Tests are excluded from tokei (`xtask/src/kloc_check.rs:302-303`).

## Re-pin artifact set (E16-A7; recommended rows: ≤ +8 physical / ≤ +17 tokei, exact at landing)

1. `xtask/kernel-core-baseline.toml:505` `src_lines` 26418 → exact (≤ 26426) plus one `# HISTORY:` row after `:505` naming D-18-6-01/02/04, the driver and the figure (never above `:505`).
2. Digest `"memory/mod.rs"` (`:904`) and `set_hash` (`:862`) via `--emit-pin`; `"memory/shared.rs"` (`:909`) only under D-03 B/C. No kernel file added; the entry count stays 95.
3. `xtask/tests/kernel_pin_content_hash_16_0.rs` literals `:820`, `:838`, `:847`, `:858` (26418 → exact).
4. `xtask/kloc.toml:264` 20643 → exact (≤ 20660) with the driver note; `xtask/tests/recovery_lane_ceiling_rule.rs:279` `RATIFIED_AT_EASING` 20_643 → exact.
5. `docs/ci-baselines/kernel-surface-v0.1-beta.json` and `xtask/kernel-api-classes.toml`: **unchanged** — no public kernel item is added (the new items are maos-domain/maos-iac; `check-service-boundary` PASSED, 0 violations, with the recommended prototype). Re-derive only if dev adds a public kernel item.
6. Out-of-kernel rows at landing, exact measured: maos-iac `xtask/kloc.toml:406` (7979, zero headroom), maos-domain `:425` (9848), maos-audit `:409` (7473), maos-bin `:517` (≤ D-08's figure), `_aggregate_hardfail` `:718` (= measured + 1).
7. Gates at landing: `check-service-boundary`, `check-kernel-baseline`, `kloc-check`.

## Operator questions (creation)

1. D-18-6-01: scrub reach. A any-writer distillates (−1/−2) / **B every kind, any writer (−34/−20)** / C B + value needles (+16/+14 over B) / D kernel 0, root sweep?
2. D-18-6-02: derived distillates. **A lineage closure (+17/+14)** / B body match only?
3. D-18-6-03: Shared. **A partition stands (0)** / B content delete (+38/+34) / C lift partition (+40/+36 + export)?
4. D-18-6-04: request formed. **A kernel computes and journals (+25/+23)** / B kernel returns sources, root journals (+18/+16) / C kernel 0 (with D-01 D)?
5. D-18-6-05: vehicle. **A root `on_consolidate` `ForgetNotice` on daemon paths, pending offline** / B Spirit-pulled / C IAC kind 34?
6. D-18-6-06: pending. **A terminal and typed** / B re-dispatch on class re-admission?
7. D-18-6-07: floors. **A `gdpr-cascade-v1` + probe-v1 through the door** / B regenerate v0 / C adapter-level erased subset?
8. D-18-6-08: maos-bin. **A named budget 28700 → 28770** / B hold, T0 measured request?
9. D-18-6-09: matching. **A token-bounded** / B raw substring?
10. D-18-6-10: W1. **A scrub-first + re-run witness, narrowed row** / B route unchanged?
11. Kernel grant (D-01 B + D-02 A + D-04 A as recommended): ratify **26418 → ≤ 26426 physical / 20643 → ≤ 20660 tokei**, exact at landing, file `memory/mod.rs` only?

## Rulings

**Ruled 2026-10-26 by the operator (Lunarpulse), relayed by Main:** all ten as recommended — D-18-6-01 **B** · 02 **A** · 03 **A** · 04 **A** · 05 **A** · 06 **A** · 07 **A** · 08 **A** (Epic 18 named maos-bin budget 28700 → **28770**; exact re-pin at landing; above → STOP and re-ask) · 09 **A** · 10 **A**. Rationale per row as in §Decision. Recorded the same day by `anthropic/claude-opus-5-5` (docs only; no cargo).

**Kernel grant (E16-A7, ADR-069), ratified with these rulings:** D-01 B + D-02 A + D-04 A = **+8 physical / +17 tokei** — `src_lines` 26418 → **≤ 26426** (`xtask/kernel-core-baseline.toml:505`), `maos-kernel-core` 20643 → **≤ 20660** (`xtask/kloc.toml:264`; `RATIFIED_AT_EASING` `xtask/tests/recovery_lane_ceiling_rule.rs:279`), exact measured figure at landing after `cargo fmt --all`, file `crates/maos-kernel-core/src/memory/mod.rs` **only**; the re-pin set above applies. Any other kernel file or a figure above the grant returns as a FLAG request. Envelope after landing at the grant: 2,340 to 23,000, 1,340 to the alarm; no `ENVELOPE ALARM`.

**Out-of-kernel rows (RECOVERY-LANE rule) — measured requests at landing; STOP at dev above expected + 25 %** (expected = prototype measurement + creation forecast; above the line → stop and report the exact `kloc-check --json` figure before further production edits or re-pins): maos-bin ≤ **28770** (D-08 A); maos-iac 7979, expected up to +150 → **STOP above +188 (8167)**; maos-domain 9848, expected up to +47 → **STOP above +59 (9907)**; maos-audit 7473, expected +54–105 → **STOP above +131 (7604)**; maos-cli (6104 / 6856) and maos-journey-test (915 / 1104) land exact inside their ceilings; `_aggregate_hardfail` = measured + 1.

## Measured development addendum — operator ruling 2026-10-26

The developer stopped after formatting and measuring: maos-bin 28658 → 28916
(+258), above the original 28770 STOP; maos-audit 7473 → 7623 (+150), above 7604.
The operator selected **A**, relayed by Main: **maos-bin ≤ 28940** and
**maos-audit ≤ 7640**, exact measured re-pins at landing with no unused allowance.
Epic 18's named maos-bin budget is **28770 → exact landed figure**. The kernel
grant, IAC/domain STOP lines, and measured+1 aggregate rule remain unchanged.

Formatted maos-bin per-file attribution: `hook_pump.rs` 778 → 937 (+159),
`main.rs` 12552 → 12601 (+49), `operator_door.rs` 1653 → 1678 (+25),
`researcher_root.rs` 352 → 377 (+25), sum +258. These are root-owned dispatch,
writer identity verification, all-path binding/terminal accounting, and lazy
operation-scoped MCP/recall adapters that do not retain audit-channel owners.
No production edit or re-pin occurred between the STOP measurement and ruling.

## Exact development landing candidate

After `cargo fmt --all`: kernel **26418 → 26407 physical (-11)**,
**20643 → 20650 tokei (+7)**, 95 files, `memory/mod.rs` only. The full pin set
(baseline line505/HISTORY, emitted set_hash and memory digest, four reader
literals, kloc ceiling and `RATIFIED_AT_EASING`) moves together. Kernel public
surface/classification artifacts remain unchanged; `check-service-boundary`
passes with zero violations.

Exact governed figures: maos-bin **28658 → 28914 (+256)**, maos-audit
**7473 → 7625 (+152)**, maos-iac **7984 → 8080 (+96)**, maos-domain
**9848 → 9887 (+39)**, maos-journey-test **915 → 946 (+31)**, maos-cli **6104**
(unchanged, ceiling lowered to exact). Aggregate **185459 → 186040**,
strict hardfail **186041 = measured + 1**. The +2-line drop from the STOP root
candidate reuses the existing compact/colon frame-id decoder; audit's +2-line
finisher gives v1 JSON fields stable lexical order under serde feature unification.
All are inside the ratified grants; `kloc-check` passes with no over-budget rows.

Story and sprint row: `review`. Hosted proven-red execution belongs to the
integrator at landing; local checksum/apply/build/expected-red/restore/green
simulation passed, with no hosted-run claim here.
