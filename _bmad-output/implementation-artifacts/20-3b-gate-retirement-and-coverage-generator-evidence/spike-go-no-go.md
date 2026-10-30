# 20-3b spike decision: NO-GO

The complete S1–S8 answers, measured byte oracle, blocker and verification handoff are in [landing-2/spike.md](landing-2/spike.md); the 33 per-occurrence corrections are in [landing-2/coverage-dispositions.md](landing-2/coverage-dispositions.md).

The requirements-source plus per-gate covers replica **did** regenerate the historical matrix byte-identically (`cmp` exit 0; 82,158 bytes; SHA-256 `209b3ffb9322b0b5e34c670fbca45b4e66305def99b0992d303c4ed182ed5119`). However, 43 v1.0 requirements have explicitly advisory empty gate lists which the current completeness control permits. AC2.3 requires the generator to reject every such list, including obligations outside this story (for example NFR-Maint-6 LTS, owned by 20-5). No production generator was written: adding false coverage, weakening AC2.3, or silently moving delivery phases is not authorized.

Under AC1.3, the hand-maintained matrix, completeness command and all its enrollments remain. Hard mode and all 33 origin corrections land independently. Invariant-lock keeps reading the hand-maintained matrix as its corpus-delta witness. No compiled/gate/test run is claimed by this edit-only implementation unit.
