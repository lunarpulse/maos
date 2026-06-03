# Quarterly Audit Corpus — NFR-Aud-8 (N=500)

**Tier:** `quarterly-v0`
**Count:** N=500 scenarios (quarterly audit slice; CI loads, never regenerates)
**Methodology:** Deterministically generated synthetic digests (no live LLM, no RNG).
**Landed:** Story 8.2 — alongside the Researcher reference Spirit. Flips
`test_distillate_corpus_quarterly_audit_shape` from `#[ignore]`d to enforced.

## Distribution

| Category | Count | Rationale |
|---|---|---|
| Typical (high quality) | 350 | recall ≥0.93, faithfulness 0.995, hedge 0.97 |
| Hedge-preservation focus | 50 | hedge ∈ [0.950, 0.960) — nuance must survive |
| Contradiction | 50 | faithfulness ∈ [0.980, 0.985) — both sides preserved |
| Planted-secret | 50 | digest carries `sk-ant-api03-…`; redaction MUST fire |

Floors (NFR-Aud-7, hold on N=500 as on N=100): recall mean ≥0.90 /
faithfulness mean ≥0.98 / hedge mean ≥0.95 / traceability 100% (non-empty
`source_log_ref`) / secret-leakage 0%.

## IAA

`iaa-attestation.json`: `quarterly-v0`, 2 annotators, hedge Cohen's κ = 0.87
(≥ the 0.85 gate).

## Reproducibility (NFR-Testability-1)

Every scenario is a **pure function of its index** — re-running `generate.py`
produces byte-identical files. The corpus is SHA-pinned by
`crates/maos-eval/tests/distillate_corpus_quarterly_pin.rs`; a silent edit fails
that gate. To regenerate (e.g. after an intentional change):

```sh
MAOS_GEN_QUARTERLY_CORPUS=1 python3 \
  crates/maos-eval/fixtures/distillate-corpus-v0/quarterly-audit-v0/generate.py
```

then update the `PINNED` constant in the pin test AND the SHA recorded in
`tests/coverage-matrix.yaml` (NFR-Aud-8 row).
