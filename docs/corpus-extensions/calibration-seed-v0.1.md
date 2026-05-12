# Calibration Seed Corpus v0.1

**Corpus ID:** `calibration-seed-v0.1`
**Authored in:** Story 0.4
**Status:** v0.1-alpha scaffolding — synthetic clearly-decidable items; no live judge-LLM calls.

## Purpose

This is the first non-empty corpus committed to `tests/corpora/`. It enables the `xtask calibrate` gate (NFR-Aud-8) to run against real data instead of the hardcoded `successes = n` placeholder deferred from Story 0.3.

At v0.1-α, every item's `expected_judgment` equals `baseline_response` by construction, so the offline-mode judge (comparing `item["expected_judgment"] == expected`) trivially passes all 100 items. The Wilson-CI for n=100/p=0.95 with pass_rate=1.0 yields ci_width ≈ 0.037, well within the 0.20 threshold → **PASSED**.

When Story 1b.4 lands the Inference Port and the first real `JudgeRunner`, the same corpus structure plugs into a real judgment loop without restructuring the gate.

## Corpus Schema (v0.1-α)

Each JSONL item:

```json
{
  "id": "calib-v0.1-NNN",
  "category": "<one of 5 categories below>",
  "bucket": "clearly-decidable",
  "prompt": "<short clearly-decidable prompt prose>",
  "baseline_response": "<the unambiguous correct answer>",
  "expected_judgment": "<same as baseline_response by construction>",
  "rationale": "<one-sentence why this is clearly-decidable>"
}
```

## Category Distribution

Per NFR-Aud-7 five-metric distillation gate and NFR-Aud-8 two-tier corpus contract.

| Category | N | IDs | Description |
|---|---|---|---|
| `digest_recall` | 20 | 001–020 | Binary recall questions about digest content |
| `digest_faithfulness` | 20 | 021–040 | Source-digest contradiction detection |
| `digest_hedge_preservation` | 20 | 041–060 | Hedge word preservation in digests |
| `digest_traceability` | 20 | 061–080 | Source-ref traceability in digests |
| `digest_secret_leakage` | 20 | 081–100 | Secret detection in digest content (no actual secrets) |

**Total:** 100 items, clearly-decidable bucket only. The LCAS (NFR-Test-6) genuinely-ambiguous and adversarially-misleading buckets are Story 8.2/8.3 territory — not in this corpus.

## Judge Mode at v0.1-α

Offline mode (`OfflineMode` in `rebaseline_check.rs`): compares `item["expected_judgment"] == expected`. Trivially passes for all items where `expected_judgment == baseline_response` (which is every item in this corpus).

## Relationship to Downstream Stories

- **Story 1b.4:** Adds `judge_id = "anthropic-claude-sonnet-4-6-T0-seed42"` to the manifest row when the real Inference Port lands.
- **Story 4.4 (v0.5+):** Grows the corpus by adding richer items per category (real-distillate evaluation) without re-categorizing — the five categories match NFR-Aud-7 metric names.
- **Story 7.3 (v1.0):** The CCAC N=600 gate uses a different corpus entirely; this calibration seed corpus remains the per-commit calibration anchor.

## Schema Metadata (for prompt_version_hash)

```json
{
  "schema_version": 1,
  "categories": [
    "digest_recall",
    "digest_faithfulness",
    "digest_hedge_preservation",
    "digest_traceability",
    "digest_secret_leakage"
  ],
  "bucket": "clearly-decidable",
  "n_per_category": 20,
  "total_n": 100,
  "authored_in_story": "0.4"
}
```
