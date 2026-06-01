# Breaking Changes

This file is the **CI-enforced** ledger of breaking changes to the MAOS ABI and
manifest surface (NFR-Maint-7). Every breaking change MUST land with a dated
`## YYYY-MM-DD` entry below, and every entry MUST carry a `**Migration:**` line
describing how a Spirit author / operator adapts. The `check-breaking-md`
discipline gate fails CI when an entry is missing its migration path.

Entry taxonomy (newest first):

- `## YYYY-MM-DD — <short title>` heading
- prose describing what changed and why
- `**Migration:**` — the concrete steps to adapt

The ABI Stability Triple and the compatibility window live in
[`STABILITY.md`](./STABILITY.md) (generated from workspace state). This file is
the human-authored change ledger that complements it.

## 2026-05-31 — v0.x → v1.0 ABI stability commitments activated (Story 7.5a)

Story 7.5a turns the v1.0 **ABI Stability Triple** `(kernel_version, abi_version,
manifest_schema_version)` from a published promise into mechanically-enforced
substrate. This entry documents a **policy activation**, not a source-level break
— all surface changes are strictly **additive**:

- The kernel now ENFORCES `min_substrate_version` at load: a Spirit whose
  declared minimum exceeds the running kernel is refused with a typed
  `SecurityError::ESubstrateTooOld` (FR8). Previously the field was parsed but
  never compared, so an incompatible Spirit admitted silently.
- The `manifest_schema_version` window is now fail-closed in BOTH directions at
  admission: below `MIN_SUPPORTED` → typed `SecurityError::EAbiTooOld` (N-2 hard
  refusal); above `MAX_SUPPORTED` → typed `SecurityError::EAbiTooNew` (a future
  Spirit is told a newer kernel is required — no silent warn-and-ignore window).
- `STABILITY.md` is published and generated from workspace state;
  `BREAKING.md` (this file) is published and CI-grep-enforced.

These are additive `SecurityError` variants on a kernel-internal enum; no
existing variant changed, `ABI_VERSION` stays `1`, `MANIFEST_SCHEMA_VERSION`
stays `2`, and `compliance.rs` is untouched. A manifest authored at
`manifest_schema_version = 1` (N-1) still loads — now with WARN-level
degradation notes for the newer sections it omits.

**Migration:** No action is required for any Spirit whose `[class]` section
declares a truthful `min_substrate_version` at or below the running kernel and a
`manifest_schema_version` within `MIN_SUPPORTED..=MAX_SUPPORTED`. A Spirit that
previously relied on a too-high `min_substrate_version` or an out-of-window
`manifest_schema_version` admitting silently must correct that field to match the
kernel it targets; the typed error names the exact mismatch (declared vs running
/ declared vs supported window).
