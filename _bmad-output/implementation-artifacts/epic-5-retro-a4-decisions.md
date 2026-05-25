# Epic 5 Retro §A4 — Team Decisions on Three Inherited Discipline-Gate Failures

**Date:** 2026-05-24
**Forum:** Party-mode discussion (Amelia + Charlie + Alice + Dana + Lunarpulse)
**Directive:** "team choose per spec and long term correctness" (Lunarpulse, 2026-05-24)
**Status:** Decisions ratified; execution scheduled across §A1 remediation + Epic 6/7

---

## Debt 1 — `check-empty-kernel` 64 violations

### Evidence

Diagnostic run at HEAD (2026-05-24, commit `6a64a97`):

- **~14 "persistent struct not in I9 whitelist" violations** — all legitimate metadata routing structs:
  - `HotSwapCoordinator.pending_reverts` (Story 5.2 hot-swap)
  - `T3ImageLock.attestations` (Story 5.5a T3 sandbox)
  - Manifest section structs: `LifecycleSection.enabled_hooks`, `McpServerEntry.allowed_tools`, `McpCapabilityServerEntry.allowed_tools`, `RawMcpCapabilities.servers`, `RawMcpCapabilityServerEntry.allowed_tools`, `RawLifecycleSection.enabled_hooks`, `RawProvidersSection.fallback`, `RawMcpSection.servers`, `RawMcpServerEntry.allowed_tools`, `MigratesFromSection.versions`, `RawMigratesFromSection.versions`
  - `CaptureChannel.events` (approval notification capture)
- **~9 `#[i9_exempt]` documentation gaps**:
  - `supervision/silent_failure_detector.rs:23`, `progress_watchdog.rs:23`, `test_double.rs:22`, `crash_detector.rs:30`
  - `inference/router.rs:19`
  - `mcp/mod.rs:23`
  - `security/manifest.rs:352`, `:1450`, `:1473`, `:1574`
- **Zero actual I6 ("kernel learns no patterns") breaches** — no pattern-learning code, no statistical aggregators, no ML primitives in kernel paths.

### Decision

**MAINTAIN the I9 whitelist + DOCUMENT the `#[i9_exempt]` markers. No ADR amendment.**

### Rationale

- **Per spec (ADR-006 + I6 invariant):** The kernel-learns-no-patterns invariant is STRUCTURALLY upheld at HEAD. Every flagged struct is metadata routed by the kernel for admission/lifecycle/IAC decisions, not pattern-learning state. The gate is correctly catching unwhitelisted state but its whitelist has fallen behind organic additions across 5 epics of substrate work.
- **Per long-term correctness:** Loosening the gate (or amending ADR-006) would lose the ability to catch FUTURE real breaches. The discipline cost of maintaining the whitelist is small; the discipline value of catching a real I6 violation is large. Keep the gate strict.

### Execution

Three concrete actions (one CI gate update + two file maintenance tasks):

1. **Whitelist append**: Add the ~14 legitimate metadata structs to `xtask/i9-whitelist.toml` with one-line rationale per entry. The whitelist already has an established schema (verify at task open).
2. **Exemption documentation**: Create or update `docs/invariants/i9-exemptions.md` to document each of the ~9 `#[i9_exempt]` markers — what it exempts and why. The pattern exists per the gate's expectation; the documentation file just needs to be authored.
3. **Whitelist-vs-exemption discipline note**: Add a §3 to `docs/invariants/i9-exemptions.md` codifying when to use whitelist (legitimate routing metadata) vs `#[i9_exempt]` (legitimate kernel-side code that the structural gate cannot statically verify is safe). Reference Story 5.3's supervision structs as the canonical exempt-vs-whitelist boundary case.

**Where it lands:** Concurrent with §A1 5.5d remediation pass (one of the 5.5d cleanups touches `security/operator_config.rs` which also needs whitelist or refactor — see Debt 2 below).

---

## Debt 2 — `check-service-boundary` violations

### Evidence

Diagnostic run at HEAD:

**Sub-debt 2a — P3 violations**: Same root cause as Debt 1 (P3 cross-references the check-empty-kernel I9 walker output). Same set of metadata structs flagged.

**Sub-debt 2b — P4 violation (NEW from Story 5.5d)**: `maos_kernel_core::security::operator_config::RegistrySection::resolve_from_env_and_disk` calls `std::fs::read_to_string` outside the mediated I/O lane. The I/O hop bypasses `IoSubsystemPort` per ADR-010 hexagonal. This is a real per-spec violation introduced by 5.5d (the registry crate consumed the kernel-core operator_config module which directly read filesystem instead of routing through the IO port).

**Sub-debt 2c — spirit-ABI-drift**: Gate expects 11 hooks per FR55 literal; trait at HEAD has 14:
- 11 base hooks (Epic 2 spec)
- + `on_swap_out`, `snapshot`, `migrate` (Story 5.2 hot-swap per ADR-017/020)
- + `epistemic_resolve` (Story 4.1 halt-protocol resolution)
- Total: **15 planned hooks** per architecture §5-spirit-abi.md text — gate's "11" literal is the stale value, not 14 or 15.

### Decisions

**Sub-debt 2a:** ROUTE through Debt 1 fix. Same whitelist maintenance.

**Sub-debt 2b:** FIX in §A1 5.5d remediation pass. Route `RegistrySection::resolve_from_env_and_disk` through `IoSubsystemPort::read_file` (or equivalent surface from the mediated I/O lane — verify HEAD-current ADR-010 surface at remediation time). NO ADR amendment.

**Sub-debt 2c:** REFACTOR the gate to read expected hook count from `xtask/spirit-abi-hook-count.toml` (NEW config file). Initial content: `count = 15` with line-by-line documentation of which Story/ADR added each hook. NO ADR amendment to FR55 — the architecture text already documents the planned growth to 15; the gate's stale literal is the bug.

### Rationale

- **Per spec:** ADR-010 hexagonal mandates ALL I/O through `IoSubsystemPort`. Sub-debt 2b is a clean per-spec violation. FR55 architecture text explicitly enumerates 11 + 3 + 1 = 15 hooks. The gate should reflect the spec, not predate it.
- **Per long-term correctness:** Toml-driven hook-count keeps the gate in sync with future hook additions without code changes — same pattern as the existing `xtask/kloc.toml`, `xtask/i9-whitelist.toml`, `xtask/fr47-vendor-sdk-denylist.toml`, etc.

### Execution

1. **Sub-debt 2b**: Patch in §A1 — `crates/maos-kernel-core/src/security/operator_config.rs::resolve_from_env_and_disk` migrates from `std::fs::read_to_string` to `IoSubsystemPort::read_file_string` (or equivalent — verify HEAD-current). If the port's surface needs a new method, add it as an additive enum variant on `IoOperation` (which is `#[non_exhaustive]`).
2. **Sub-debt 2c**: NEW `xtask/spirit-abi-hook-count.toml` with `count = 15` + per-hook documentation. Update `check-service-boundary`'s spirit-ABI-drift check to read this config. Half-day xtask work; can land in the same PR as the three new gates (§A3/§A5/§A6).

**Where it lands:**
- Sub-debt 2b: §A1 remediation pass (already in scope).
- Sub-debt 2c: alongside §A3/§A5/§A6 gates (this session).

---

## Debt 3 — `kloc-check` / `maos-kernel-core` overshoot

### Evidence

- **`xtask/kloc.toml`** declares `maos-kernel-core = 6000` LOC ceiling.
- **HEAD measurement** (`tokei crates/maos-kernel-core/src`): **21,370 Rust LOC across 102 files**.
- **Overshoot ratio: 3.56x** — almost 4x over budget.

**Module breakdown:**

| Module | LOC | Decomposition candidate |
|---|---|---|
| `security` | 5,256 | YES — extract `maos-manifest` + leave admission in kernel |
| `iac` | 3,350 | YES — extract `maos-iac` (TL + log_recall + bus routing) |
| `capability` | 2,400 | YES — extract `maos-capability` (Story 1b.2 pattern formalized) |
| `scheduler` | 1,961 | YES — extract `maos-scheduler` |
| `memory` | 1,659 | YES — extract `maos-memory` |
| `hot_swap` | 1,317 | YES — extract `maos-hot-swap` |
| `halt` | 971 | NO — stays in kernel (single-halt-owner per Epic 4) |
| `inference` | 682 | Possibly fold into `maos-providers` |
| `telemetry` | 589 | Possibly fold into `maos-iac` |
| `supervision` | 569 | YES — extract `maos-supervision` |
| `revocation` | 548 | Possibly fold into `maos-capability` (token revocation) or new `maos-revocation` |
| Smaller modules | <400 each | Likely stay in kernel as glue |

Extracting the top 4 (`security` + `iac` + `capability` + `scheduler` = 12,967 LOC) brings kernel-core to ~8,400. Further extracting `memory` + `hot_swap` brings it to ~5,400 — finally under the 6k ceiling.

### Decision

**DECOMPOSE `maos-kernel-core` across Epic 6/7 stories. NO ADR-038 amendment.**

### Rationale

- **Per spec (ADR-038):** The per-service KLOC ceiling exists explicitly to PREVENT this kind of accretion. Amending the ceiling to fit the current size (~22k) would invert ADR-038's purpose — the ADR is a structural discipline, not a measurement-of-current-state. ADR-038's value comes from its strictness; if every overshoot resulted in a ceiling raise, the gate is a vanity metric.
- **Per long-term correctness:** Story 1b.2's capability-registry decomposition (cap_tokens / cap_policy / cap_audit / cap_quota sub-modules) established the pattern for in-kernel decomposition. Crate-level extraction is the next phase. Each extracted crate becomes a focused, independently-testable, independently-classifiable surface that the §4.0.7 four-class taxonomy can reason about more cleanly.
- **Architecturally cleaner:** Each extracted crate has a clear class:
  - `maos-manifest` = data-movement (parses TOML, produces typed values)
  - `maos-iac` = data-movement (routes frames, no semantic interpretation)
  - `maos-capability` = universal-arithmetic (the cap-token verify hot path is the only place ADR-030 truly fires)
  - `maos-scheduler` = supervision
  - `maos-memory` = data-movement (per ADR-026 principal namespace)
  - `maos-hot-swap` = supervision
- **kernel-core post-decomposition** becomes the supervisor coordinator + glue (halt + smaller modules + composition root forwarders). Naturally under 6k.

### Execution

**Phased decomposition across Epic 6/7:**

| Phase | Story | Extract | Rationale |
|---|---|---|---|
| **Phase 1** | Story 6.5 (gateway sub-modules) | `maos-iac` (3,350 LOC) | Story 6.5 already decomposes gateway sub-modules; IAC extraction is the natural companion (gateways route IAC frames). |
| **Phase 2** | Story 6.1 (full IAC bus + DRR + retract) prep | `maos-capability` (2,400 LOC) | Story 6.1 extends the capability surface for the retract primitive; clean extraction point before the new code lands. |
| **Phase 3** | Story 7.2 (full registry publish/install/yank) | `maos-manifest` (subset of security/, ~2,500 LOC) | 7.2 extends manifest parsing for full registry surface; clean extraction point. |
| **Phase 4** | Story 7.x (TBD) | `maos-scheduler` + `maos-memory` + `maos-hot-swap` | Brings kernel-core under 6k ceiling. Sequencing depends on Story 7.x scope (PR-by-PR decomposition is safer than big-bang). |

**Phase 1 + 2 alone bring kernel-core to ~15,600 LOC** — still over but ratio drops to 2.6x. Phase 3 brings to ~13,100 (2.2x). Phase 4 brings to ~6,000 (1.0x — at ceiling).

**Interim posture (Epic 6 opens before Phase 4 completes):**

The kloc.toml ceiling at HEAD is breached but is the OUTCOME of legitimate substrate growth, not undisciplined accretion. **Document the decomposition plan in `xtask/kloc.toml` as a TODO header comment** so future story openings see the in-progress migration. The gate stays strict (does NOT raise the ceiling); stories that ADD to kernel-core MUST first extract a candidate module that NETS to ≤0 added LOC.

**§A4 immediate output:** add `[in_progress_decomposition]` block to `xtask/kloc.toml` documenting Phase 1-4 schedule + per-phase target crate + target completion epic. The kloc gate continues to fail until Phase 4 — making the breach VISIBLE on every CI run so it's not forgotten.

---

## Summary of §A4 Decisions

| Debt | Decision | Per Spec? | Per Long-Term Correctness? | Where it lands |
|---|---|---|---|---|
| **Debt 1** | Maintain I9 whitelist + document exemptions | ✅ I6/ADR-006 holds; gate is correct | ✅ Strict gate catches future real breaches | Concurrent with §A1 |
| **Debt 2a** | Same as Debt 1 (whitelist) | ✅ | ✅ | Concurrent with §A1 |
| **Debt 2b** | Route operator_config through IoSubsystemPort | ✅ ADR-010 hexagonal | ✅ Closes accidental I/O leak | §A1 remediation pass |
| **Debt 2c** | Move spirit-ABI hook count to xtask/spirit-abi-hook-count.toml | ✅ FR55 architecture text says 15 hooks | ✅ Toml-driven keeps gate in sync with future additions | This session (alongside §A3/§A5/§A6) |
| **Debt 3** | Decompose maos-kernel-core across Epic 6/7 (NO ADR-038 amendment) | ✅ ADR-038 ceiling is the spec | ✅ Story 1b.2 pattern formalized; per-crate class classifier cleaner | Phased: 6.5 / 6.1-prep / 7.2 / 7.x |

**No ADR amendments required across all three debts.** Every decision honors the existing spec; every decision pays long-term-correctness compound by keeping discipline gates strict.

**Forward-shapes for Epic 6 opening:**

1. Story 6.1 spec must include precondition: "Phase 2 `maos-capability` extraction completes before 6.1 retract primitive code lands."
2. Story 6.5 spec must include: "Phase 1 `maos-iac` extraction happens as Task 0 before gateway sub-module work."
3. `xtask/kloc.toml` gets `[in_progress_decomposition]` block this session as Action A4-immediate.

---

**Discussion close:**

**Lunarpulse (Project Lead):** [satisfied with decisions per spec + correctness criteria]

**Charlie (Architect):** This honors ADR-006, ADR-010, ADR-038, and FR55 simultaneously. No spec drift, no ADR amendment debt accruing.

**Amelia (Developer):** Three structured execution paths. Two of them (Debt 1 + 2b) close inside §A1. One (Debt 2c) closes alongside §A3/§A5/§A6 this session. One (Debt 3) is phased across Epic 6/7 with a kloc.toml decomposition block landing this session as the discipline anchor.

**Dana (Test Architect):** And the gates stay strict throughout — no "inherited not regression" disclaimers in future story Completion Notes. The CI failure surface is honest.
