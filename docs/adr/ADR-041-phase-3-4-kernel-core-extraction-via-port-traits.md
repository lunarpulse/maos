---
Status: binding-v0.7
Gate: `xtask/kloc.toml` `[in_progress_decomposition]` Phase 3 + Phase 4 trackers; `xtask check-service-boundary` P1 (single-owner) compliance per extracted crate; `cargo-public-api` additive-only across the extraction window
Decided: 2026-05-28 (Epic 6 retrospective §A3 — closed in same session)
Accepted-in-PR: <PR_NUMBER>
Revisits: §4.0.4, §4.0.7, §4.0.8
Supersedes: none (extends ADR-038 with a phased-execution clause)
---

# ADR-041 — Phase 3/4 `maos-kernel-core` extraction via `maos-domain` port traits

**Decision.** The two remaining Epic-7 KLOC extractions land via `maos-domain` port-trait boundaries, not by moving implementations directly between crates:

1. **Phase 3 (Story 7.2 prep)** — `security/*` residual fold (~1,000 LOC). Re-homes manifest-validation tail into `maos-manifest`; re-homes T0/T1/T2/T3 admission glue into `maos-capability` if classifier matches, else `maos-manifest`. No new crate; no new port trait — the residual is type-only refactoring already covered by `maos_domain::ports::security::*`.

2. **Phase 4 (Story 7.4 prep — pre-Story-7.5a)** — `maos-scheduler` + `maos-memory` + `maos-hot-swap` + `maos-supervision` extracted as four new crates. The extraction is mediated by three port traits already living (or newly placed) in `crates/maos-domain/src/ports/`:

   - `SpiritSchedulerPort` — **already exists** at `crates/maos-domain/src/ports/scheduler.rs`; Story 7.4 fills in the full method surface (the v0.1-α stub declared only the lifecycle-verb surface; Story 5.1's 11 lifecycle verbs + Story 6.4's `ScheduleWatchdog` plug into this port).
   - `MemoryManagerPort` — **already exists** at `crates/maos-domain/src/ports/memory.rs`; Story 4.3 landed the three-tier mechanics (Private / Shared / Collective) + write/read/forget/export. Story 7.4 only needs the implementation hosted in `maos-memory` instead of `maos-kernel-core`.
   - `HotSwapPort` — **does NOT exist yet**; this ADR's same-PR addition stubs the trait in `crates/maos-domain/src/ports/hot_swap.rs`. Story 7.4 (pre-Story-7.5a) fills the method surface against the existing `crates/maos-kernel-core/src/hot_swap/{coordinator, saga, state_codec, precheck, post_swap_monitor, archive, migrator}.rs` machinery (~1,700 LOC) and moves the implementation into a new `maos-hot-swap` crate.

   `maos-supervision` is the smallest extraction (~569 LOC measured at Epic 5 retro) and does NOT require a new port trait — it consumes `SpiritSchedulerPort` + the journal port `crates/maos-domain/src/ports/iac_bus.rs`'s logging surface. The supervision implementation moves to a new `maos-supervision` crate behind those existing ports.

**Rationale.** Three branches were considered at the Epic 6 retrospective (§What-Was-Challenging §3):

- **(i) trait-boundary refactor [CHOSEN].** Highest one-time cost, cleanest steady-state surface. Story 7.5a's ABI Stability Triple `(kernel_version, abi_version, manifest_schema_version)` needs a settled `maos-kernel-core` ABI to publish in STABILITY.md. Trait-boundary refactor produces port-typed ABI for downstream crates: every cross-crate call in Phase 4 routes through `Arc<dyn TraitName>` rather than concrete types from `maos-kernel-core`. The trait surfaces stay in `maos-domain` (the most stable crate); concrete implementations may evolve under the port without breaking ABI.

- **(ii) accept partial extraction.** Lower upfront cost, accumulates debt. Epic 6's Phase 1 extraction (Story 6.5 `maos-iac`) demonstrated the cross-dependency blocker class — 3 of 13 IAC files retain cross-deps with `maos-kernel-core` internals. Choosing (ii) for Phase 4 would compound this debt at the highest cross-dep-density phase of the kernel (scheduler ↔ memory ↔ hot-swap ↔ iac ↔ capability is the kernel's hottest mesh). Story 7.5a's STABILITY.md cannot ship cleanly against a mid-decomposition `maos-kernel-core`.

- **(iii) amend ADR-038 ceiling.** Lowest effort, retroactively legalizes the current 2.5× overshoot. Inverts ADR-038's stated purpose ("the ceiling is the architectural discipline, not measurement of current state"). Rejected.

**(i) is the default recommendation Epic 6 retro §A3 carried into this session.**

**Two of the three port traits already exist.** The trait-boundary refactor's nontrivial work is the `HotSwapPort` authoring + the Phase 4 implementation moves (Story 7.4 prep window). The architectural decision is settled; the labor is scoped, sequenced, and budgeted under Story 7.4.

**Alternatives considered and rejected.**

- Phase 3 + Phase 4 in the same Story 7.2 PR. Rejected: scope explosion + Phase 4 has more cross-dep work than Phase 3.
- New crate `maos-cross-cutting` to hold scheduler+memory+hot-swap together. Rejected: violates §4.0.7 four-class taxonomy (would be a god-cluster).
- Extracting before Story 7.5a authors STABILITY.md. Rejected: STABILITY.md is downstream of the surface; the surface must settle first.

**What would force a revisit.**

- A cross-dep blocker in Phase 4 escalation surfaces a missing port surface (e.g. `MemoryManagerPort` proves insufficient for the Phase 4 `maos-memory` extraction). Add an additive port via ADR-037 amendment process.
- Story 7.5a measures the residual `maos-kernel-core` post-Phase-4 at > 6,000 LOC. ADR-038 amendment via ADR-037 process (raise the ceiling OR identify Phase 5 candidates).
- A Phase 4 implementation move triggers an ABI removal (not additive). Phase 4 is staged BEFORE the v1.0 cut on the Story 7.5a window; ABI breakage must be settled inside Phase 4, not after Story 7.5a publishes STABILITY.md.

**Phase 4 implementation choreography (Story 7.4 prep — sequenced):**

1. Author `HotSwapPort` trait in `crates/maos-domain/src/ports/hot_swap.rs` (this ADR's same-PR addition).
2. Extract `maos-scheduler`: move `crates/maos-kernel-core/src/scheduler/**` → new crate; `SpiritSchedulerPort` impl moves with it. Add `pub use` shim in `maos-kernel-core` to preserve the additive ABI surface.
3. Extract `maos-memory`: move `crates/maos-kernel-core/src/memory/**` → new crate; `MemoryManagerPort` impl moves with it. Same `pub use` shim pattern.
4. Extract `maos-hot-swap`: move `crates/maos-kernel-core/src/hot_swap/**` → new crate; new `HotSwapPort` impl lands here. Cross-dep on the journal-port + IAC-bus-port preserved.
5. Extract `maos-supervision`: move `crates/maos-kernel-core/src/supervision/**` → new crate; consumes `SpiritSchedulerPort` only.
6. Update `xtask/kloc.toml` per-crate ceilings for the four new crates; verify `_aggregate_alarm` and `_aggregate_hardfail` are not exceeded.
7. Update `xtask/check-workspace-count` baseline (27 → 31 crates).
8. Run `cargo-public-api --diff` to confirm additive-only at the workspace surface.

**Story 7.5a precondition gate.** STABILITY.md draft does NOT publish until Steps 1–8 close cleanly. The ABI surface that 7.5a freezes MUST be the post-Phase-4 surface, not a mid-decomposition surface.
