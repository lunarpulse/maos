# 20-11 creation — records owed

Rulings in force: D01 A / D02 A / D11 A (2026-10-30); D-20-11-01…10 RULED exactly as recommended by Lunarpulse
on 2026-11-02 (ask `s2011`; -02 B, -03 B, -04 A). Original line numbers below are at `a8af3215` unless marked
"post-edit"; §A–§C records are applied/dispositioned as accounted below. No implementation closure is claimed.

## Applied at creation (Create2011 owns these files this session)

| # | File:line | Edit |
|---|---|---|
| X1 | `_bmad-output/implementation-artifacts/20-11-erasure-inventory-closure.md` | new story, `ready-for-dev` |
| X2 | `_bmad-output/implementation-artifacts/20-11-erasure-inventory-closure-evidence/creation/` | this directory |
| X3 | `_bmad-output/implementation-artifacts/sprint-status.yaml:309` | `20-11-erasure-inventory-closure: backlog` → `ready-for-dev` with the creation note; the prior note kept after `WAS:` |

Applied 2026-11-02: `sprint-status.yaml` `last_updated` is `2026-11-02`. Existing EC-13, FR65 and NFR-Aud-12
owners are unchanged; EC-13 was D:1331 at creation. No owner row is closed by this recording pass; those
obligations remain with this story at implementation landing.

## §A — epic corrections applied to `epic-20-ship-it-w5.md` (2026-11-02, ask s2011)

Line numbers are at `a8af3215`. The working tree carries a sibling's uncommitted E20 edit at `:276` (+2 lines), so
post-edit the 20-11 section is `:281-286` (A2 → `:285`, A3 → `:286`); `:84` is unchanged.

| # | E20 line | Edit |
|---|---|---|
| A1 | `:84` (stories row) | "FR65 · kernel-Δ unresolved pending prototype" → append "· created 2026-11-02 @a8af3215 — ready-for-dev; every category has a kernel-free shape; optional kernel rows priced by reverted prototype KH +10/+6, KS +10/+6, KF +37/+33 (recommended KS only, D-20-11-04 A); maos-iac +90 measured, maos-bin/audit/registry forecast (D-20-11-01…10 PROPOSED)" (C6) |
| A2 | `:283` (AC1) | `crates/maos-bin/src/main.rs:10570-10584` → `:10845-10860` (category vector `:10812-10861`) (C1) |
| A3 | `:284` (AC2) | `deferred-work.md:1206` → `:1331` (C3) |

## §B — other planning records (applied)

| # | File:line | Edit |
|---|---|---|
| B1 | `planning-artifacts/epic-20-preflight-2026-10-30.md:248` | `main.rs:10570-10584` → `:10845-10860` (C1) |
| B2 | `…preflight…:129`, `:249` | `D:1206` → `D:1331` (C3) |
| B3 | `planning-artifacts/routing-audit-2026-10-22.md:25`, `:185` | `main.rs:9290-9305` → `main.rs:10845-10860` (C2) |
| B4 | `planning-artifacts/epics/requirements-inventory.md:571`, `:596` | owner "16-4" → "`20-11-erasure-inventory-closure`"; `main.rs:9290-9305` → `:10845-10860` (C2, C5) |

## §C — routed observations (all recorded; unresolved behavior remains honestly open)

| # | Observation | Proposed destination | Why not here |
|---|---|---|---|
| R1 | `maos purge` deletes the erasure-proofs directory with the product data (`crates/maos-bin/src/purge.rs:56,106`); NFR-Aud-12 asks for a receipt "retained independent of the substrate" (`prd/non-functional-requirements.md:72`) | `20-10-purge-residue-removal` (rule 10(b): it already edits `purge.rs`; preflight D09 governs the purge contract) — an operator decision whether purge keeps, exports or deletes proofs | per-Spirit uninstall never deletes proofs; the product purge is 16-4/20-10's surface |
| R2 | Python verifier P18 ignores `redacted_principal_frame_proofs` (`tools/verify-erasure/verify.py:304-312` vs `crates/maos-audit/src/erasure/proof.rs:384-396`) | this story under D-20-11-09 A (it edits both verifiers); otherwise a deferred row on `20-11` | — |
| R3 | The second, unreceipted `drain_for_spirit` in `unload` (`scheduler_loop.rs:574`) may discard a halt invoked after `terminate_spirit` | T0 measurement 2 decides; if real, STOP + D-row here (rule 10(a): the uninstall halt path is this story's blast radius) | unmeasured |
| R4 | Sibling story `20-10-purge-residue-removal.md:37` cites the `uninstall` one-shot at `main.rs:7173-7211`; at `a8af3215` that range is the `MAOS_ONE_SHOT` dispatch preamble and collective erase — the uninstall arm is `main.rs:7360-7399` | 20-10's record (orchestrator) | not this story's file |

## Ruling-record application — 2026-11-02

| Items | Resolution |
|---|---|
| A1–A3 | Epic creation/pricing row records ruled KS +10/+6 and kernel-free alternatives; category-vector/current inventory and EC-13 anchors corrected. |
| B1–B4 | Preflight inventory/EC-13, both routing-audit inventory cites, and requirements-inventory ownership/cite corrections applied. Preflight pricing now distinguishes granted KS from rejected optional rows. |
| R1 | Open `deferred-work.md` row routes the purge-proof retention question to 20-10 under rule 10(b); no keep/export/delete behavior was invented or ruled by this recording. |
| R2 | D-20-11-09 A includes both Rust/Python verifier fixes in this story; row now RULED A, AC1/proof contract retained. No duplicate deferral needed. |
| R3 | Open `deferred-work.md` row names this story and T0 measurement 2; if real, STOP + D-row. Measurement not faked and no kernel grant expanded. |
| R4 | 20-10's per-Spirit uninstall reference corrected to `main.rs:7360-7399`. |
| Story / tracker / grand plan | All 10 decision rows RULED with option/date/ask; frontmatter KS/envelope and changelog retained. Status remains ready-for-dev. Later kernel landing re-bases; expected 21125, cumulative STOP 21209. |

**Accounting:** all 11 §A/§B/§C items applied or explicitly routed under the ruled recommendation; 0 outstanding current recording items. The real R1 decision, R3 measurement and implementation/landing obligations remain open with their named owners, not silently treated as resolved behavior.

