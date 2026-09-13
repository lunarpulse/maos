---
Status: ACCEPTED — ratified 2026-09-08 under Story 15-1 (`15-1-green-at-head`), AC3 / D-4 / D-5. Recovery-lane Epic 15 (Foundations, W0).
Gate: `xtask check-service-boundary` PASSED (0 NFR-Test-2 rows) against the surgically refreshed `docs/ci-baselines/kernel-surface-v0.1-beta.json` (370 → 369 items); symmetric diff attached to the landing commit in the `1dda03eb` form: paths added 0 · paths removed 1 · hashes changed 18
Decided: 2026-09-08
Accepted-in-PR: <PR_NUMBER>
Revisits: the `af788c3e` surface movement (Epic 5 review-finding closure, 2026-08-13) which changed the kernel public surface without touching either paired instrument; `kloc.toml:199`'s prior adjudication of the same commit
Supersedes: nothing — records and sanctions a drift that already happened
---

# ADR-066 — Sanctioned removal of `get_default_image`, the verified-type signature changes, and where the note lives

**Context.** `check-service-boundary` was RED at `2fd492c0` with 19 `removed` rows and 17
`other`-class rows. Measured by driving the gate's own `--json` emission against the shipped
baseline: **18 of the 19 removed triples are signature-hash refreshes of symbols that still
exist; exactly ONE is a genuine removal** — `security::sandbox::t3::image_lock::get_default_image`,
deleted by `af788c3e` (*"fix(epic-5): close review findings and harden release gates"*), the
same commit `kloc.toml:199` already indicts for taking its PIN delta while leaving its paired
CEILING behind. `af788c3e` touched 30 kernel-core files and updated neither
`kernel-surface-v0.1-beta.json` nor `kernel-api-classes.toml` — **same commit, second
instrument, same shape of drift.** The baseline was last blessed at `accf763c` (2026-08-07).

## Decision (a) — the removal is sanctioned, and its rationale is security

`get_default_image` returned an **unverified fallback image URI**. Its deletion is not an ABI
loss but a security repair: T3 spawning now admits images only through
`VerifiedImageLock::resolve_pin` — a type that *cannot be constructed from deserialized lock
material* — so the unverified-default path that `get_default_image` enabled is closed at the
type level. Removing it from the baseline is the monotonicity ledger catching up with a
repair that should have been booked when it landed.

## Decision (b) — the three breaking signature changes, which the epic omitted

Three refreshed symbols changed **breaking-ly** (not cosmetic drift); all three now return
or consume verified types:

| Symbol | Change |
|---|---|
| `security::sandbox::t3::spawn::spawn_t3` | consumes `VerifiedImageAttestation` instead of a raw image URI |
| `security::sandbox::t3::argv::build_runtime_argv` | consumes the verified attestation's entry |
| `security::sandbox::t3::image_lock::load_and_verify_lock` | returns `VerifiedImageLock`, failing closed |

The remaining 15 refreshes are additive or internal (new fields, doc moves, the `pub use`
group-hash shift on the five `lifecycle::upgrade` re-exports).

## Decision (c) — the drift's cause is recorded, not just its repair

`af788c3e` moved the surface without its paired instrument. The 2026-08-13 baseline blessing
cadence had no gate tying a kernel-touching commit to a re-bless, which is the same absent
instrument `kloc.toml:199` names for the ceiling. This ADR is the second entry in that
pattern's ledger; the systemic control (a done-story's commit as the measurement point for
its own rows) is filed to Story 20-3, not built here.

## Decision (d) — why this note is an ADR and not a key in the baseline JSON

`invariant-lock` maps `I1..I14 → docs/invariants/I*.md` only and never fires on
`kernel-api-classes.toml` or the baseline JSON: the "invariant-lock review" line in both is a
**prose convention**, exactly as ADR-065 §87-92 adjudicates for `gate-registry.toml`.
Mechanically, a key inside the JSON cannot survive: `KernelSurface` derives plain
`Serialize`/`Deserialize` (no `deny_unknown_fields`), so an added `"_invariant_lock_review"`
key would *parse* — and then be **silently destroyed by the next re-emit**, which writes only
the three declared fields. ADR-043 → ADR-065 is the discharging chain this closes.

## Provenance of the repair (Story 15-1 AC3 / D-4)

The baseline was **hand-edited surgically** — 18 in-place `signature_hash` refreshes plus the
single-item deletion, 370 → 369 — not re-emitted: a full re-emit is a proven one-button green
(0 rows with `kernel-api-classes.toml` untouched) that would have made the 8
genuinely-new-path classifications unreachable by construction. `--write-baseline` was
rejected (+15 charged xtask lines onto an over-ceiling crate; funding source did not exist).
The 8 genuinely-new paths are classified in `kernel-api-classes.toml` under the
`# Story 15-1 AC3` banner, one justification line each.
