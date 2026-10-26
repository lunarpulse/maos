# 18-7 FLAG-Winston decision record

**Date:** 2026-10-26
**Status:** `ruled`. The operator (Lunarpulse) ruled D-18-7-01…16 on 2026-10-26, each as recommended (§Rulings). ◆ Kernel grant **+5 physical / +4 tokei** (26407 → ≤ 26412, 20650 → ≤ 20654) ratified under the ADR-069 envelope. Files: `capability/cap_policy/decision.rs`, `capability/cap_policy/mod.rs`, `capability/mod.rs`, `scheduler/scheduler_loop.rs`, plus the `Intent` re-pin in `docs/ci-baselines/kernel-surface-v0.1-beta.json`. Story → `ready-for-dev`.
**Story:** `18-7-live-skill-authoring-and-revisions` (`implementation-artifacts/18-7-live-skill-authoring-and-revisions.md`), created at HEAD `8fdf497f`. At that HEAD 18-0…18-4 and 18-8 were done, 18-5 was done pending one CI marker pin, and 18-6 was implemented and under review; its code was treated as landing.
**Authority:** E16-A7; ADR-069 (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`: envelope ≤ 23,000 tokei, alarm 22,000, zero headroom, every line a measured grant); RECOVERY-LANE CEILING RULE (`xtask/kloc.toml:50-93`) for the rows outside the kernel; proof contract (`docs/testing/proof-contract.md:10-29,60-67`); epic-18 §18-7 (created at the Epic-17 retrospective from `routing-audit-2026-10-22.md:19`).
**Model:** `anthropic/claude-opus-5-5` (bmad-create-story, lighter creation contract: no party-mode preflight, no non-author validation pass).

## Decision

| ID | Decision | Status |
|---|---|---|
| **D-18-7-01** ◆ | **How authoring/proposing is capability-mediated (AC2; security-critical; kernel).** `Scope::SkillAuthorSelf` (`crates/maos-domain/src/invariants/i1.rs:112-119`) has a policy arm (`capability/cap_policy/mod.rs:189-201`) but no `scope_to_intent` arm (`capability/mod.rs:58-111`, fall-through `PolicyDenied` `:106-109`). As a result `issue_with_mediation` refuses it for every Spirit and `is_mediated_scope` calls it unmediated (`crates/maos-bin/src/spirit_session.rs:899-928`; the 17-3c exemplar `crates/maos-bin/tests/spawned_scope_17_3c.rs:2021,2185`). **A (recommended):** add `Intent::SkillAuthorSelf` plus a `scope_to_intent` arm and an `intent_action_key` arm. The root port then mints one governed token per submission (`issue_enterprise_governed_capability`, `worker_spawn.rs:386-462`) at the pid it resolves itself, verifies it before the queue write, and refuses `TaskAssign` delegation of the scope. Measured **+4 / +3**. **B:** kernel 0. The root checks declared scopes and the deny map itself; no token, no quota/context/tier checks. **C:** keep the demo path. | **ruled A** (Lunarpulse 2026-10-26) |
| **D-18-7-02** ◆ | **How a Spirit declares the scope (AC2; kernel ripple).** `[capabilities.required]` has no skill table (`crates/maos-manifest/src/manifest.rs:603-609,869-917`). Capability tables are schema-gated (`:674-683`). Adding a field to `CapabilitiesRequired` breaks the kernel harness literal (`scheduler/scheduler_loop.rs:326-332`) and three maos-bin literals. **A (recommended):** schema v6 `[capabilities.required.skill] author_self`. Measured **+1 / +1** in the kernel; maos-manifest +22, maos-bin +3. **B:** the same table in v5 without a bump; the kernel line is the same. **C:** kernel 0; an out-of-band root grant after admission. | **ruled A** (Lunarpulse 2026-10-26) |
| D-18-7-03…16 | Kernel 0 by construction (root, maos-skill, maos-manifest, maos-cli/control, spirits). Rows, options and rationale: story §Decisions. | **ruled as recommended**: 03 A, 04 A, 05 A, 06 B, 07 A, 08 A, 09 A, 10 A, 11 A, 12 A, 13 B, 14 A, 15 A, 16 B |

## Measured figures (E16-A7)

Measured in place at `8fdf497f` (clean tree) by throwaway prototypes in the workspace, after `cargo fmt --all`. Physical lines come from `cargo run -q --locked -p xtask -- check-kernel-baseline`; tokei comes from `-- kloc-check --json` (tokei 14.0.0). Every run used `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`, the only cargo process on the host.

| Instrument (HEAD 26407 / 20650) | Physical (`check-kernel-baseline`) | Tokei (`kloc-check`) | Files | Other |
|---|---:|---:|---|---|
| D-01 A alone | 26411 (+4) | 20653 (+3) | `capability/cap_policy/decision.rs`, `capability/cap_policy/mod.rs`, `capability/mod.rs` | `cargo check --locked -p maos-kernel-core` clean; `check-service-boundary` FAILED: `removed public kernel symbol 'maos_kernel_core::capability::cap_policy::decision::Intent'` (the gate hashes whole enums, so a new variant reads as removal) |
| **D-01 A + D-02 A** (recommended, ratified) | **26412 (+5)** | **20654 (+4)** | + `scheduler/scheduler_loop.rs` | `cargo check --locked --workspace` clean after the three maos-bin literal ripples (`crates/maos-bin/src/admission.rs:615-625`, `main.rs:7334-7344`, `:13724-13730`); maos-manifest 4583 → 4605 (+22); maos-bin 28914 → 28917 (+3); aggregate 186040 → 186069; `check-service-boundary` FAILED on `Intent` only |
| D-02 A or B alone | +1 | +1 | `scheduler/scheduler_loop.rs` | — |
| D-01 B / D-02 C / D-03…16 | 0 | 0 | none (by construction) | — |

Revert: `git checkout -- crates`; `git diff --quiet HEAD -- crates xtask spirits Cargo.lock Cargo.toml` exited 0; `check-kernel-baseline` PASSED 26407 / 95 afterwards. Prototype diffs were not kept (no sha recorded).

Test-only ripple (not counted by the kernel baseline or tokei, `xtask/src/kloc_check.rs:302-303`):
- Seven test files build `CapabilitiesRequired` literally.
- The v6 bump moves the N-1 window tests (`crates/maos-manifest/tests/manifest_n_minus_1_compat.rs`, `crates/maos-spirit-abi/tests/manifest_n_minus_1_test.rs`).
- 17-3c's "unmediated" case moves its exemplar to `GatewaySend`. Its CI pins stay at 23 (`discipline.yml:3890-3891,4032-4036`).

**Envelope:** under the ratified rows tokei goes 20650 → **20654**, leaving **2,346** to 23,000 and **1,346** to the 22,000 alarm. No `ENVELOPE ALARM`.

**Not in the prototypes, forecast [INFERENCE] (story §Kernel and kloc pricing):**

| Crate | Forecast |
|---|---|
| maos-bin | +125–235 (port, wiring, door handler, boot fix; demo cut −30–35) |
| maos-skill | +165–270 |
| maos-cli | +40–75 |
| maos-control | +25–45 |
| maos-iac | 0–20 |
| maos-audit | +6–18 |
| maos-journey-test | 0–25 |
| spirits/researcher | ungoverned |

## Re-pin artifact set (E16-A7; ratified: ≤ +5 physical / ≤ +4 tokei, exact at landing)

Line numbers are at `8fdf497f`. 18-6's review patches may move the pin and these lines first: T0 re-measures from the landed values and applies +5 / +4 over them, never widened.

1. `xtask/kernel-core-baseline.toml:505` `src_lines` 26407 → exact (≤ 26412), plus one `# HISTORY:` row after `:505` naming D-18-7-01/02, the driver and the figure (never above `:505`).
2. Digests `"capability/cap_policy/decision.rs"` (`:874`), `"capability/cap_policy/mod.rs"` (`:875`), `"capability/mod.rs"` (`:876`), `"scheduler/scheduler_loop.rs"` (`:934`) and `set_hash` (`:867`), all via `--emit-pin`. No kernel file is added; the entry count stays 95.
3. `xtask/tests/kernel_pin_content_hash_16_0.rs` literals `:820`, `:838`, `:847`, `:858` (26407 → exact).
4. `xtask/kloc.toml:264` 20650 → exact (≤ 20654) with the driver note; `RATIFIED_AT_EASING` in `xtask/tests/recovery_lane_ceiling_rule.rs:279` 20_650 → exact.
5. `docs/ci-baselines/kernel-surface-v0.1-beta.json`: the `maos_kernel_core::capability::cap_policy::decision::Intent` item (`:62`) re-hashed. **Only that item.** `xtask/kernel-api-classes.toml:128` (`universal-arithmetic`) is unchanged. Any other surface delta returns as a FLAG request.
6. `xtask/fkcs-baseline.toml:41` (`admission_baseline.sha256`, D-18-7-10 A) is re-pinned in the same commit after review. It is not a kernel file but is part of this story's admission-path re-pin.
7. Out-of-kernel rows at landing, exact measured, inside the STOP lines of §Rulings: maos-bin `xtask/kloc.toml:517` (≤ 29150), maos-skill `:693`, maos-manifest `:415`, maos-cli `:489`, maos-iac `:406`, maos-audit `:409`, maos-control `:484`, maos-journey-test `:681`, maos-fkcs `:677`, and `_aggregate_hardfail` `:718` (= measured + 1).
8. Gates at landing: `check-service-boundary`, `check-kernel-baseline`, `kloc-check`, `check-fkcs`, `check-manifest-schema-version`.

## Operator questions (creation)

1. D-18-7-01, mediation: **A mint (+4/+3)** / B root-side check, no token (0) / C demo path?
2. D-18-7-02, declaration: **A schema v6 table (+1/+1)** / B v5 no bump (+1/+1) / C out-of-band root grant (0)?
3. D-18-7-03…16: story §Decisions (kernel 0).
4. Kernel grant (D-01 A + D-02 A as recommended): ratify **26407 → ≤ 26412 physical / 20650 → ≤ 20654 tokei**, exact at landing, limited to the four files plus the `Intent` surface re-pin?

## Rulings

**Ruled 2026-10-26 by the operator (Lunarpulse), relayed by Main:** all sixteen as recommended:

| Rows | Ruling |
|---|---|
| 01, 02, 03, 04, 05 | **A** |
| 06 | **B** |
| 07, 08, 09, 10 | **A** |
| 11 | **A** (WASM/TS routed to `20-8-manifest-contract-completion`; received row filed in `deferred-work.md`) |
| 12 | **A** |
| 13 | **B** |
| 14 | **A** |
| 15 | **A** (Epic 18 named maos-bin budget 28914 → **29150**; exact re-pin; above → STOP) |
| 16 | **B** |

Rationale per row is in the story's §Decisions. Recorded the same day by `anthropic/claude-opus-5-5` (docs only; no cargo).

**Kernel grant (E16-A7, ADR-069), ratified with these rulings:** D-01 A + D-02 A = **+5 physical / +4 tokei**.
- `src_lines` 26407 → **≤ 26412** (`xtask/kernel-core-baseline.toml:505`).
- `maos-kernel-core` 20650 → **≤ 20654** (`xtask/kloc.toml:264`; `RATIFIED_AT_EASING` `xtask/tests/recovery_lane_ceiling_rule.rs:279`).
- The exact measured figure is taken at landing after `cargo fmt --all`.
- Files `capability/cap_policy/decision.rs`, `capability/cap_policy/mod.rs`, `capability/mod.rs`, `scheduler/scheduler_loop.rs` **only**, plus the `Intent` surface re-pin. The re-pin set above applies.
- Any other kernel file, surface item, or a figure above the grant returns as a FLAG request.
- If 18-6's review patches land first and move the pin, +5 / +4 applies over the landed values.
- Envelope after landing at the grant: 2,346 to 23,000, 1,346 to the alarm; no `ENVELOPE ALARM`.

**Out-of-kernel rows (RECOVERY-LANE rule): measured requests at landing.** At dev, STOP above expected + 25 %, where expected = prototype measurement + upper creation forecast. Above the line → stop and report the exact `kloc-check --json` figure before further production edits or re-pins.

| Row | Now | Expected | STOP line |
|---|---|---|---|
| maos-bin | 28914 | — | **≤ 29150** (D-15 A, named budget) |
| maos-skill | 800 / 900 | up to +270 | **above +338 (1138)** |
| maos-manifest | 4583 | +22 measured + 15 | **above +46 (4629)** |
| maos-cli | 6104 | up to +75 | **above +94 (6198)** |
| maos-iac | 8080 | up to +20 | **above +25 (8105)** |
| maos-audit | 7625 | up to +18 | **above +23 (7648)** |
| maos-domain, maos-capability, maos-wasm-host, maos-spirit-sdk | — | 0 | **any growth** |
| maos-control (1286 / 1791), maos-journey-test (946 / 1104), maos-fkcs (491 / 591), maos-spirit-abi (1042 / 3000) | — | — | land exact inside ceiling; reaching a ceiling → STOP |

`_aggregate_hardfail` = measured + 1. All figures are re-stated over the landed values at T0.
