# HSIS Corpus v0 — Hot-Swap Invariant Strength

6 classes × 50 scenarios = 300 total (Story 5.2).

## Methodology

- **Tier:** `scripted-v0` (synthetic kernel-side fixtures, deterministic seed)
- **Regeneration seed:** `0xHSIS_CORPUS_V0_0001` (recorded in `methodology-attestation.json`)
- **Pass threshold:** ≥95% per class (NFR-Rel-3), zero CVSS-7 violations

## Spirit classes

| Class | Scenarios | Invariant focus |
|---|---|---|
| Butler | 50 | on_idle continuity, calendar state, principal namespace |
| Researcher | 50 | Citation graph, distillation lineage (I11), tool-call queue |
| Observer | 50 | Subscription continuity, scalar.tap stream, broadcast window |
| Orchestrator | 50 | Task-assign queue, Worker handoff, founder-loop checkpoint |
| Worker | 50 | CLI handle preservation, output-shape adapter version pinning |
| CliWrapper | 50 | Output-shape adapter mismatch (ADR-021), stdin/stdout buffer |

## Scenario JSON schema

Each scenario conforms to `HsisScenario` in `crates/maos-eval/src/hsis_corpus.rs`.

## References
- ADR-017 (state-transfer wire format)
- ADR-019 (I14 halt continuity)
- ADR-020 (cross-major migration)
- NFR-Rel-3 (HSIS ≥95% per class)

## Carry-forward from Story 4.5

Story 4.5's spec promised a 100-scenario `hsis-researcher-observer-v0/` corpus;
verification at HEAD shows it was never authored. Story 5.2 absorbs the full
300 scenarios into `hsis-corpus-v0/` under one consolidated tree.
