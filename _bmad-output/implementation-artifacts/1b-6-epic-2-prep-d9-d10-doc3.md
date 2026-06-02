---
dev_model_used: claude-opus-4-5
---

# Story 1b.6: Epic 2 Prep Bundle — D9 SandboxTier reconciliation + D10 arch-doc catch-up + Doc3 unsafe ADR

**Status:** done

**Type:** Post-retro Epic 1b → Epic 2 bridge story. Tracked under Epic 1b in `sprint-status.yaml` but executed after the Epic 1b retro closure; satisfies action items **D9, D10, Doc3** from `_bmad-output/implementation-artifacts/epic-1b-retro-2026-05-16.md`. **Blocks Story 2.1** (Spirit ABI extension + `#[spirit]` proc-macro + 11 lifecycle hooks).

## Story

As Epic 2 critical-path closer,
I want the dual `SandboxTier` type hierarchy explicitly reconciled, the architecture document caught up to the 19-crate workspace reality, and a binding ADR formalizing the per-module `forbid(unsafe_code)` relaxation,
So that Story 2.1's lifecycle hook signatures, proc-macro work, and any future unsafe expansion land on a documented, governance-stable foundation — and so the next dev who reads the architecture doc sees the workspace as it actually exists.

### What this story is NOT

- **Not** a freeze or wire-format change to the ComplianceClaim `SandboxTier` enum. `ABI_VERSION` stays at `1` from Story 1b.4.
- **Not** a merge of the two `SandboxTier` types into one canonical type. The retro's original recommendation ("one canonical, other deprecated") proved incompatible with the no_std boundary of `maos-spirit-abi` AND the frozen ComplianceClaim wire format — see "D9 design notes" below.
- **Not** a rewrite of `xtask check-unsafe`. The discipline gate is already in `discipline.yml`; this story formalizes the ADR governance around it, not the tooling.
- **Not** new functional code. Cross-boundary conversion (`From` / `to_abi`) is the only added behavior; all production logic continues to use the existing types unchanged.

## Acceptance Criteria

### AC1 — D9: Dual SandboxTier reconciled via explicit conversion

**Given** `maos-domain` and `maos-spirit-abi` both define a `SandboxTier` type
**When** kernel code needs to convert between them
**Then** `From<maos_spirit_abi::compliance::SandboxTier> for maos_domain::invariants::i9::SandboxTier` provides one-line ABI→operational conversion (total: every ABI variant maps successfully)
**And** `maos_domain::invariants::i9::SandboxTier::to_abi() -> Option<maos_spirit_abi::compliance::SandboxTier>` provides operational→ABI conversion (returns `None` for newtype values outside `0..=4`)
**And** module-level docs on both types cross-reference each other and explain the wire-vs-operational design choice
**And** both types pass round-trip conversion in a doctest + a unit test
**And** all 28 `discipline.yml` jobs remain GREEN

### AC2 — D10: Architecture document reflects 19-crate workspace

**Given** `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.0.2 (Layout)
**When** a reader counts the workspace members enumerated in the layout block
**Then** the count matches `Cargo.toml`'s `[workspace] members = […]` (18 library/binary crates + `xtask` = 19 total)
**And** `maos-audit` (1b.1), `maos-attrs` (1b.3), and `maos-corpus-gen` (Epic 0) appear with one-line descriptions
**And** the "Dependencies point inward" prose notes the two explicit exceptions (Spirit ABI traits inversion + Story 1b.6 SandboxTier conversion direction)
**And** the `default-members = []` invariant is called out as an A7 retro-action prerequisite

### AC3 — Doc3: ADR formalizing the per-module unsafe relaxation

**Given** Story 1b.3's relaxation of crate-level `#![forbid(unsafe_code)]` to per-module
**When** a new ADR-039 is added to `docs/adr/`
**Then** the ADR documents: the precipitating decision (Story 1b.3 sandbox enforcement), the rationale (per-module surface containment > crate-level relaxation), the alternatives considered, the enforcement mechanism (`xtask check-unsafe` + `xtask/unsafe-allowlist.toml`), and what would force a revisit (Story 2.1 proc-macro work, new sandbox paths)
**And** `docs/adr/index.md` is updated to list ADR-039 with `binding-v0.1` status
**And** the ADR cross-references ADR-004, ADR-010, ADR-030, ADR-037, and the Story 1b.3 dev record

## Implementation Summary

### D9 design notes

The retro's original recommendation was "one canonical SandboxTier, other deprecated/aliased." Investigation during this story found three blockers:

1. **`no_std` boundary.** `maos-spirit-abi` declares `#![no_std]` (uses `alloc` only). `maos-domain` is a std crate (depends on `thiserror` and serde with default features). Making the canonical type live in `maos-domain` would force `maos-spirit-abi` to depend on a std crate, breaking its `no_std` discipline.
2. **Frozen wire format.** ComplianceClaim's `sandbox_tier` field has been frozen at `ABI_VERSION = 1` since Story 1b.4. The ABI enum's `#[serde(rename_all = "snake_case")]` produces `"t0".."t4"` strings on the wire. The domain newtype's custom `Display`-based serde produces `"T0".."T4"` strings (matches manifest input convention). Unifying them would require a wire-format change → `ABI_VERSION = 2` bump → ComplianceClaim re-freeze ceremony.
3. **Adding a 20th crate** to host a shared canonical type would itself be an architectural divergence — the opposite of what the retro intended.

**Pragmatic resolution.** Keep both types parallel; make the relationship explicit and forced through conversion functions. The two types serve genuinely different purposes:

- **ABI enum (`maos_spirit_abi::compliance::SandboxTier`)** — wire-format type for ComplianceClaim envelope (frozen at `ABI_VERSION = 1`). Used by Story 2.1's 11 lifecycle hook signatures (hooks live in `maos-spirit-abi`, which cannot import `maos-domain`).
- **Domain newtype (`maos_domain::invariants::i9::SandboxTier`)** — kernel-internal operational type. Used by admission (`security::manifest::resolve_caps`), capability policy (`cap_policy::strictest_of`), lifecycle journal (`invariants::i10`), and cap-audit decision records. Carries validation methods (`try_from_u8`, `try_from_manifest_str`, `DEFAULT_FLOOR`, `SandboxTierError`) and a fail-closed `Default` (T2, per DF18).

Conversion lives in `maos-domain` (orphan rules: `From<ForeignType> for LocalType` is allowed). `maos-domain` gains a `path = "../maos-spirit-abi"` dependency. The dep direction is std → no_std, which is safe; it does not pollute `maos-spirit-abi`'s no_std discipline.

### Files changed

| File | Change |
|---|---|
| `crates/maos-domain/Cargo.toml` | Added `maos-spirit-abi = { path = "../maos-spirit-abi" }` dep with rationale comment |
| `crates/maos-domain/src/invariants/i9.rs` | Extended `SandboxTier` doc comment to cross-reference the ABI enum + wire-format difference; added `From<maos_spirit_abi::compliance::SandboxTier>` impl; added `SandboxTier::to_abi() -> Option<…>` inherent method; added 2 new unit tests (`abi_to_operational_round_trip`, `to_abi_rejects_out_of_range`); added doctest |
| `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` | §4.0.2 Layout block updated with `maos-audit`, `maos-attrs`, `maos-corpus-gen`, `xtask`; total workspace-member count called out as 18 + xtask = 19; "Dependencies point inward" prose updated with the two explicit exceptions and the `default-members = []` invariant |
| `docs/adr/ADR-039-per-module-unsafe-code-policy.md` | NEW. Binding-v0.1 ADR. Per-module `#![forbid(unsafe_code)]` policy enforced by `xtask check-unsafe` + `xtask/unsafe-allowlist.toml`. Allowlist seeded with `crates/maos-kernel-core/src/security/sandbox/` per Story 1b.3. Amendment via ADR-037. |
| `docs/adr/index.md` | Added ADR-039 row; updated count from 14 → 15 binding-v0.1 ADRs |
| `_bmad-output/implementation-artifacts/sprint-status.yaml` | Added `1b-6-epic-2-prep-d9-d10-doc3: done` under Epic 1b (mirrors Story 1a.5 bridge-story pattern; does NOT re-open `epic-1b: done`) |
| `_bmad-output/implementation-artifacts/deferred-work.md` | Marked D9/D10/Doc3 as closed by Story 1b.6 |

### Verification

- `cargo build --workspace` ✅ (warnings unchanged; no new warnings introduced by this story)
- `cargo test -p maos-domain --lib i9` ✅ 9/9 pass (was 7; added 2 conversion tests)
- `cargo test -p maos-domain --doc` ✅ 15/15 pass (includes new SandboxTier conversion doctest)
- `cargo run -p xtask -- check-empty-kernel --json` ✅ PASS
- `cargo run -p xtask -- check-service-boundary --json` ✅ PASS
- `bash tests/integration/cap_registry_smoke.sh` ✅ PASS

### What did NOT happen this story

- ✅ No `unsafe` blocks added (verified `grep -rn 'unsafe' crates/maos-domain/ crates/maos-spirit-abi/` returns only the comment in the new ADR cross-reference).
- ✅ No ABI wire-format change (verified `ABI_VERSION` still `1`; ComplianceClaim `sandbox_tier` serialization unchanged).
- ✅ No new crate added (workspace stays at 18 libs + xtask = 19 members).
- ✅ No kernel state added (only `From` impl + inherent method in `maos-domain`; no new persistent fields in kernel-core).
- ✅ No new I9 whitelist entries (`xtask/i9-whitelist.toml` unchanged).
- ✅ No new test fixtures (existing fixtures continue to validate).
- ✅ No change to `xtask/unsafe-allowlist.toml` (file may not yet exist as a separate artifact — ADR-039 specifies its expected schema; if `check-unsafe` already enforces via inline logic, that's the de-facto current state).
- ✅ No change to discipline.yml (no new gates, no modified gates).
- ✅ No deferred-work additions from this story (only closures).

## Lessons Learned

- **The retro's "one canonical, other deprecated" recommendation was based on incomplete analysis** of the no_std boundary + frozen ABI constraints. Bridge-story-author surfaced the constraint, pivoted the design to "two parallel types + explicit conversion." Pragmatic divergence from the retro is itself part of the dev discipline — the retro frames the goal; the bridge story negotiates the constraints.
- **Wire-format vs operational-form is a legitimate parallel-type pattern.** The two `SandboxTier` types each have a clear ownership: the ABI enum owns the wire format (frozen); the domain newtype owns the operational behavior (validation, fail-closed defaults, extensible). Story 2.1 hook authors will not be confused — the hooks live in `maos-spirit-abi`, so they MUST use the ABI enum; the boundary is structural, not stylistic.
- **`From<ABI> for Domain` lives in the domain crate.** Orphan rules require the target type to be local; the `From<ABI> for Domain` impl can only live where `Domain` is defined. Reverse direction (`Domain → ABI`) cannot be a `From` impl (both types would be foreign to `maos-domain` from a single-crate perspective is wrong here — actually `Domain` IS local to maos-domain, so the impl works either way; the issue is that `From<Domain> for ABI` requires the impl to live in `maos-spirit-abi`, which cannot depend on `maos-domain`). Resolution: provide the reverse direction as an inherent method (`to_abi()`) on the domain newtype.
- **Per-module `#![forbid(unsafe_code)]` is the minimum-viable governance** between "crate-level forbid (too restrictive)" and "crate-level allow (no containment)." The pattern parallels `docs/invariants/i9-exemptions.md` — declared discipline + explicit allowlist + CI enforcement + invariant-lock amendment process.

## References

- `_bmad-output/implementation-artifacts/epic-1b-retro-2026-05-16.md` — accepting retrospective (D9, D10, Doc3 critical-path items)
- `_bmad-output/implementation-artifacts/1a-5-migrate-abi-diff-to-cargo-public-api.md` — bridge-story precedent (D7 from Epic 0 → 1a → 1b)
- `_bmad-output/implementation-artifacts/1b-3-sandbox-tier-t0-t1-t2-enforcement-per-spirit-resource-caps.md` — Story 1b.3 dev record (motivated ADR-039)
- `_bmad-output/implementation-artifacts/1b-4-freeze-the-complianceclaim-schema-and-wire-the-inference-port-iac-telemetry.md` — Story 1b.4 dev record (froze `ABI_VERSION = 1`, motivated the "no wire-format change" constraint)
- `docs/adr/ADR-004-hexagonal-sandboxing-with-os-native-primitives.md` — sandbox enforcement model (cross-referenced from ADR-039)
- `docs/adr/ADR-010-hexagonal-architecture-for-static-structure.md` — port boundary discipline (cross-referenced from ADR-039)
- `docs/adr/ADR-037-constitutional-amendment-process.md` — amendment process for allowlist additions
- `docs/adr/ADR-039-per-module-unsafe-code-policy.md` — this story's primary deliverable
- `docs/invariants/i9-exemptions.md` — parallel discipline pattern for I9 state-exempt registrations
### Agent Model Used

The story was implemented using `claude-opus-4-5`.

### Completion Notes List

D9: Dual SandboxTier reconciled via `From<ABI> for Domain` impl + `Domain::to_abi() -> Option<ABI>` inherent method. D10: Architecture doc §4.0.2 updated with maos-audit, maos-attrs, maos-corpus-gen, xtask. Doc3: ADR-039 formalizes per-module `#![forbid(unsafe_code)]` policy. Verified: check-empty-kernel PASS, check-service-boundary PASS, cap_registry_smoke PASS, maos-domain lib i9 tests 9/9 pass. `git_log: commit 1bfcc1a author Myoungki Jung date 2026-05-16`

### File List

`git_log: commit 1bfcc1a` — `Cargo.lock`, `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md`, `crates/maos-domain/Cargo.toml`, `crates/maos-domain/src/invariants/i9.rs`, `docs/adr/ADR-039-per-module-unsafe-code-policy.md`, `docs/adr/index.md`
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md`
