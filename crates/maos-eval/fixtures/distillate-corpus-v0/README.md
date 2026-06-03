# Distillate Corpus v0 — NFR-Aud-7 Five-Metric Gate

**Tier:** `synthetic-v0`  
**Count:** N=100 scenarios (per-commit CI slice, CI-width ≈0.124 at p=0.90)  
**Methodology:** Hand-authored synthetic digests against synthesized raw frames.  
**Threat model:** Covers recall degradation, faithfulness collapse, hedge erosion, traceability bypass, and secret leakage.  
**Derivation:** Appendix F.5 — floor values derived from judge-LLM noise floors and operational data.  

## Distribution

| Category | Count | Rationale |
|---|---|---|
| Typical (high quality) | 70 | Baseline scenarios with recall ≥0.92, faithfulness ≥0.99, hedge ≥0.96 |
| Hedge-preservation focus | 10 | Scenarios where hedge nuance must be preserved (hedge ≥0.95 gate) |
| Contradiction cases | 10 | Scenarios containing contradictions that should drop faithfulness |
| Planted-secret cases | 10 | Scenarios with literal secret tokens — redaction MUST fire |

## IAA

v0.3-β: solo project, single annotator, self-attested at κ=0.85 floor.
v1.0+: ≥2 annotators per Appendix F.5 — LANDED in Story 8.2 (`quarterly-audit-v0/`
has 2 annotators, κ=0.87).

## Corpus Growth

- v0.3-β: N=100 per-commit slice (this directory)
- v1.0 quarterly: N=500 quarterly audit slice (`quarterly-audit-v0/` — LANDED in
  Story 8.2; `test_distillate_corpus_quarterly_audit_shape` now enforced)
