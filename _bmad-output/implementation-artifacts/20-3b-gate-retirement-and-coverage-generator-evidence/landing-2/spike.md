# 20-3b landing-2 coverage spike — NO-GO

Decision recorded before production generator code. Input is landing-1 tree 9d53764f, `tests/coverage-matrix.yaml` (82,158 bytes, 198 requirements, 111 requirement/gate edges, 65 distinct gate names). Original and reconstructed SHA-256: `209b3ffb9322b0b5e34c670fbca45b4e66305def99b0992d303c4ed182ed5119`.

## Oracle and measured design

A read-only Python decomposition extracted requirements with all non-gate YAML bytes intact, replacing each gate list with a layout-only `# gate-list: inline|4|6` marker. Gate membership and order were extracted into per-gate `covers` records (`requirement`, zero-based `position`). Reconstructing from those two inputs yielded byte-identical output. The initial replica exposed both block indentation styles and inline lists; preserving those three styles fixed the mismatch without retaining gate names in the requirements source.

Observed command (exit **0**, no output):

```text
cmp /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-3b-gate-retirement-and-coverage-generator-evidence/landing-2/spike-original.yaml /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-3b-gate-retirement-and-coverage-generator-evidence/landing-2/spike-reconstructed.yaml
```

No cargo, tests, gates, formatter or proc-macro was used. Two early helper attempts failed (missing optional Python yaml module; then root variable unset); the successful replica uses only stdlib and preserves raw YAML, not a YAML reserializer.

## S1–S8

1. **Requirements source design:** a requirements-keyed YAML source keeps phase, valid_until, notes, corpora and enforcement_cadence exactly (`coverage-matrix.yaml:1-16,234-339`). YAML rather than proposed TOML avoids opaque quoted YAML fragments. Layout comments contain no gate claim. Historical decomposed source is `spike-source.yaml`; it is evidence, not a production source.
2. **Per-gate covers design:** `[[coverage_gate]]` entries with registered `name` and `covers = [{ requirement = "…", position = N }, …]`. Position preserves non-alphabetical existing order. Historical extracted registry is `spike-covers.toml`.
3. **Oracle:** byte-identical reconstruction succeeded before any intentional changes. That proves representability, not that a strict v1.0 validator can accept today's honest claims.
4. **Invariant-lock:** on NO-GO its corpus-delta witness remains the hand-maintained `tests/coverage-matrix.yaml` (`invariant_lock.rs:156`). No generated/source pair is shipped.
5. **Enforcer:** `coverage_matrix.rs:93-120` remains unchanged, including phase validation and hard/warning parser.
6. **Blocking incompatibility / NO-GO:** inspection found **43** `phase: v1.0` rows with empty gates, all explicitly `enforcement: advisory-until-engagement`. The current completeness implementation exempts these (`check_coverage_matrix_completeness.rs:57-63`); AC2.3 instead requires refusing every empty v1.0 row. This is not one of the 33 unknown-reference/phase errors. NFR-Maint-6's LTS promise explicitly belongs to 20-5; other external/operator obligations likewise cannot acquire honest local control coverage in 20-3b. Keeping the exception violates AC2.3; inventing gate coverage or silently relabelling requirement delivery violates the story's honesty/scope contract. Therefore **NO-GO before production generator code**, despite the successful byte oracle. Completeness remains enrolled everywhere, including prepush; no generator or generated header lands. AC1.3 explicitly authorizes this branch.
7. **Hard scope:** `coverage_matrix.rs:145-153` enforces empty coverage only at/before delivered phase; names and phases are checked at **all** phases (`:125-144`). Matrix and `tests/phase-config.toml` remain synchronized at v0.1-alpha. Advancing delivered phase is a separate delivery claim, not implied by hard mode. NFR-Rel-1/2/11 retain honest empty coverage and v0.8 deferral.
8. **33 violations:** fixed at origin, never grandfathered. The companion `coverage-dispositions.md` records every occurrence and its concrete witness. Existing test/CI controls are registered; pseudo-corpora absent from the authoritative manifest are removed as corpus claims with their actual validation paths retained in notes. NFR-Rel-7/NFR-Scale-2 v2.2 and NFR-Test-8 v2.0 become v2.0+, preserving original PRD phases in notes (ruled row 03 B).

The 43 incompatible rows are: FR42, FR43, FR44, FR45, FR46, FR49, FR51, FR59, FR62, FR63, FR64, FR65, NFR-Aud-1, NFR-Aud-10, NFR-Aud-12, NFR-Aud-13, NFR-Aud-2, NFR-Aud-3, NFR-Aud-6, NFR-Comp-2, NFR-Comp-5, NFR-Cost-1, NFR-Doc-2, NFR-Doc-3, NFR-Doc-4, NFR-Doc-5, NFR-Doc-7, NFR-Maint-4, NFR-Maint-5, NFR-Maint-6, NFR-Maint-7, NFR-Maint-9, NFR-Meta-1, NFR-Obs-1, NFR-Ops-12, NFR-Ops-2, NFR-Ops-3, NFR-Rel-4, NFR-Rel-5, NFR-Rel-6, NFR-Sec-15, NFR-Sec-8, NFR-Test-14.

## Verification ownership

The designated verifier must compile/run the retained completeness vectors and `coverage-matrix --json`, check all 33 dispositions, and confirm hard-mode negative vectors still fail. No compiled generator or AC2 drift proof is claimed on the ruled NO-GO branch.
