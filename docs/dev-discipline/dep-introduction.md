---
title: Dependency-Introduction Discipline
status: active
since: 2026-05-13
origin: Epic 0 retrospective (Action Item A2)
applies_to: All MAOS workspace crates (`crates/**`, `xtask/`)
addresses: DF4 (Cargo.lock bloat from `tempfile` pulling ~25 WASI crates)
---

# Dependency-Introduction Discipline

## Why

In Story 0.4, adding `tempfile = "3"` as a single dev-dependency transitively pulled `getrandom 0.4.2` plus roughly 25 WASI / WebAssembly ecosystem crates (`wasip2`, `wasip3`, `wit-bindgen`, `wasm-metadata`, `wasmparser`, ...). The bloat was discovered post-merge during a `Cargo.lock` audit. The dev-review pass focused on logic; nobody looked at the dependency tree.

MAOS is a kernel-substrate project. Every transitive dep is audit surface, build-time, and supply-chain risk. We need a discipline that **surfaces the transitive blast radius at the moment of introduction**, when the cost of choosing differently is zero.

## What this is — and is not

This is a **written discipline**, not a CI gate. At v0.1-α we do not have automated dep policing beyond `cargo deny check` (license + advisory hits). The discipline is enforced by the PR author noting the blast radius in the dev record, and by the reviewer confirming it. It promotes to a CI gate when justified by a security or build-time incident — not preemptively.

## The rule

Every new entry in any `Cargo.toml`'s `[dependencies]`, `[dev-dependencies]`, or `[build-dependencies]` table — whether top-level or a transitive promotion — must carry:

1. **Justification** in the PR description (one sentence: what does this dep do that we cannot do ourselves at acceptable cost?).
2. **Blast-radius note** in the dev record (one or two lines, as below).
3. **License + audit confirmation** that `cargo deny check` continues to pass against the new dep tree.

## How to compute the blast radius

After adding the dep but BEFORE committing, run:

```bash
# 1. Run a clean lockfile resolution
cargo update --workspace --offline 2>/dev/null || cargo update --workspace

# 2. Count new entries in Cargo.lock vs the previous HEAD
git diff HEAD -- Cargo.lock | grep -c '^+name = '

# 3. List the names of new transitive deps
git diff HEAD -- Cargo.lock | grep '^+name = ' | sed 's/^+name = //'

# 4. For any dep you don't recognize, inspect its origin
cargo tree -p <crate> -i <new-dep>
```

Paste the count and the names into the dev record. Example:

```markdown
### Dependency-introduction note

Added: `tempfile = "3"` (dev-dependency for xtask integration test fixtures).

Blast radius: **27 new entries** in `Cargo.lock`.
Notable transitive deps: `getrandom 0.4.2`, `wasip2 1.x`, `wit-bindgen 0.x`, `wasm-metadata 0.x`.
Justification: tempfile is the idiomatic Rust testing primitive for fixture-tree
patterns; rolling our own would be 50+ LOC of platform-conditional cleanup logic
that already lives behind a well-maintained crate.
Trade-off: ~25-crate WASI ecosystem pulled in via `getrandom`'s WASI feature path.
Acceptable at v0.1-α; flag for review if WASI ecosystem deps prove problematic at v0.5+.
```

Numbers are non-negotiable in the note; "small" / "minimal" / "a few" are not blast-radius statements.

## Rejection criteria — when to choose differently

A PR author should consider alternative deps (or a hand-rolled implementation) when:

- **>50 new lockfile entries** for a single addition. Probably a wildcard-features case; pin specific features instead.
- **Any new entry under a `*-sys` crate** the project hasn't already vetted. `*-sys` crates link C code at build time; treat as supply-chain hot zone.
- **License downgrade** flagged by `cargo deny check`. Hard block; do not merge.
- **A dep already vetted for an overlapping purpose** exists in the workspace. Re-use, do not duplicate. Example: don't add `sha-1` if `sha2` is already a workspace dep.

If the dep clears these and the justification is solid, proceed. The point of the discipline is to make the choice **deliberate and visible**, not to forbid additions.

## What the reviewer checks

In code review:

- [ ] Dev record contains the dependency-introduction note with a concrete count.
- [ ] `cargo deny check` is documented as passing (or its output is attached).
- [ ] No alternative already in the workspace covers the same need.
- [ ] The justification reads as a deliberate choice, not a reflex.

If any of the above is missing, request the dev record be updated before approving. **Do not** treat this as a blocking review failure — it is a process-tightening pass, not a build break.

## When the discipline becomes a CI gate

Promote this to an automated gate when one of these holds:

- A security advisory hits a transitive dep that the introduction note would have flagged.
- Build time exceeds a defined budget (no budget set at v0.1-α; revisit at v0.5).
- A `cargo-machete`-style unused-dep detector finds enough drift to justify continuous enforcement.

Until then: the discipline is the dev record + the reviewer's eye. Both are real, neither is automated.

## References

- DF4 (Cargo.lock bloat from `tempfile`) — `_bmad-output/implementation-artifacts/deferred-work.md`
- Epic 0 retrospective Action Item A2 — `_bmad-output/implementation-artifacts/epic-0-retro-2026-05-13.md`
- `deny.toml` — repo-root supply-chain policy
- `cargo tree` documentation — https://doc.rust-lang.org/cargo/commands/cargo-tree.html
