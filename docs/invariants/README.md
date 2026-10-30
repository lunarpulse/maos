# Invariant Register

This directory contains the canonical machine-readable register of MAOS invariants I1–I14.

## Files

- `I1.md` through `I14.md` — one file per invariant, with frontmatter enforcing the phase-cadence matrix from Architecture §3.2.1.
- `journal.jsonl` — append-only journal of every invariant-lock merge event.

## Claim ledger (20-3b)

Every register frontmatter declares `claim: guaranteed | asserted`. The existing
`invariant-lock` reads **all fourteen** entries on every invocation and includes
them in its JSON report, even when no invariant was changed.

- `guaranteed` requires `property_test: relative/path.rs#test_name`,
  `adversarial_corpus: relative/path`, and `ci_job`. The gate refuses missing
  files, missing test functions or a missing discipline job.
- `asserted` requires a non-empty `owner`. It is not a test failure or an
  assertion that implementation is absent: narrower unit/runtime anchors do
  not earn the full invariant-wide guarantee. Null evidence fields explicitly
  disclose that no complete property/adversarial pair has been established.
- The ledger owner owns the claim classification; it does not silently create
  a new implementation story. All broad invariants currently remain asserted.
  I5 separately records **FR13 asserted, owner 20-1** and **FR31 proven by
  20-3b** (cross-boot adversarial vectors in `principal_identity_20_3b.rs` and
  `butler_scene_18_4.rs`). Principal isolation does not prove quarantine isolation.
  The landing verifier must exercise these new vectors; metadata is not execution evidence.

The coverage spike selected NO-GO (43 explicitly advisory v1.0 empty gate
lists cannot satisfy strict generator completeness without false claims).
The matrix remains hand-maintained, in hard mode; its corpus-delta witness
therefore remains `tests/coverage-matrix.yaml`.


## Lock Protocol (ADR-037)

Any PR touching an invariant file (`I*.md`), the `lock.toml` mapping, or the `maos-domain` invariant types must pass the `invariant-lock` CI gate. The gate requires:

1. **Machine-checkable diff** against the invariant set (present by construction).
2. **Corpus delta** — touch to `tests/coverage-matrix.yaml` (file-touched check at v0.1-α; row-count comparison at v0.3+).
3. **Phase-commitment update** — at least one touched `I*.md` has its enforcement-cadence table modified.
4. **≥2 maintainer sign-offs** verified via platform API.
5. **Forward-only progression** — no demotion of enforcement cells (`runtime → CI` is forbidden).

On merge success, the gating CI job appends to `journal.jsonl`.
