# Safety-Critical Spirit Corpus — Methodology (Story 8.5, Decision F)

This document defines the **2-annotator human annotation protocol** for the
safety-critical Spirit corpus that the Mira + Nash bilateral pair (Story 8.5) is
measured against, and records why CI replays the annotation labels deterministically
(the **stand-in seam**).

## 1. Scope and floors

| Property | Value | Source |
|---|---|---|
| Scenarios per Spirit (Mira, Nash) | **N ≥ 150** | AC5 / `MIN_SCENARIOS_PER_SPIRIT` |
| Annotators | **≥ 2** | AC5 / `IaaAttestation.annotator_count` |
| Inter-annotator agreement floor | **Cohen's κ ≥ 0.7** | AC5 / `SAFETY_CRITICAL_KAPPA_FLOOR` |
| Computation | `maos_eval::cohen_kappa` | `crates/maos-eval/src/safety_critical_corpus.rs` |
| SHA-256 pin (Story 0.3) | `safety_critical_corpus::CORPUS_SHA256_PIN` | `tests/corpora/MANIFEST.toml` |

The corpus computes a [Cohen's κ](https://en.wikipedia.org/wiki/Cohen%27s_kappa)
over two annotators' categorical safety labels and produces an `IaaAttestation`
(the same shape Story 4.4's distillate corpus uses), with κ verified at or above
**0.7** for each Spirit and for the corpus as a whole.

### Why κ ≥ 0.7 here, not 0.85 (distillate)

Story 4.4's distillate corpus enforces **κ ≥ 0.85** because it measures
**hedge-preservation** — a fine-grained judgment about whether a distillate kept
the epistemic hedges of its source, where annotators are comparing near-identical
text and small disagreements are meaningful signal.

The safety-critical corpus measures a **coarser categorical judgment**: each
scenario is labelled `benign` / `caution` / `critical` — "what safety action does
this prod-edge diagnosis (Mira) or proposed fix (Nash) warrant?". Categorical
safety labelling is inherently noisier than hedge-comparison (annotators draw the
`caution`/`critical` boundary slightly differently), so the epic sets the floor at
**0.7** — "substantial agreement" on the Landis–Koch scale — which is the
appropriate, defensible bar for this kind of label. The two floors are intentionally
distinct; this difference is the documented rationale Decision F flags for Winston,
ratified in **[ADR-042](adr/ADR-042-safety-critical-kappa-floor-distinct-from-distillate.md)**
(a third κ floor appearing is the trigger to generalize ADR-042 into a per-corpus-class table).

## 2. The 2-annotator human protocol (production)

In production the labels are produced by **two independent human annotators**:

1. **Corpus assembly.** A corpus author collects ≥ 150 representative scenarios per
   Spirit — for Mira, prod-edge diagnostic situations (anomalies on real or
   synthetic service shards); for Nash, dev-environment architecture situations
   (proposed fixes touching real or synthetic subsystems). Each scenario is a
   self-contained prompt with no label attached.
2. **Independent labelling.** Two annotators (a domain SRE for Mira, a senior
   architect for Nash, drawn from a pool of ≥ 2) **independently** assign one of
   `{benign, caution, critical}` to each scenario, blind to each other's labels.
3. **Agreement computation.** Cohen's κ is computed over the two label vectors per
   Spirit via `maos_eval::cohen_kappa`. The corpus **ships only if κ ≥ 0.7** for
   each Spirit; otherwise the label guide is refined and re-annotated (a corpus
   below floor is a methodology failure, not a Spirit failure).
4. **Adjudication (optional).** Disagreements may be adjudicated by a third
   annotator to produce a gold label, but the **κ floor is computed on the raw
   pre-adjudication labels** — adjudication does not inflate the agreement metric.
5. **Attestation.** The author records `IaaAttestation { corpus_version,
   annotator_count, hedge_cohen_kappa, computed_at }` and the corpus SHA-256, and
   registers the corpus in `tests/corpora/MANIFEST.toml` + `tests/coverage-matrix.yaml`.

## 3. The stand-in seam (CI / v1.5)

The real human annotation above is a **documented process**, not a CI dependency —
CI cannot block on human annotators, and the κ metric must be **bit-stable**. So,
exactly as Story 7.5b's stand-in corpus and Story 4.4's `iaa-attestation.json`
pattern, **CI fixture-replays the two annotators' labels**:

- `SafetyCriticalCorpus::generate()` deterministically emits N = 150 scenarios per
  Spirit, each carrying both annotators' replayed labels. Annotator B mirrors
  annotator A except on a fixed ~1-in-9 cadence — a stand-in for genuine
  inter-annotator disagreement that yields **κ = 0.83** (≥ the 0.7 floor, with
  margin) deterministically every run.
- The corpus is **SHA-256-pinned** (`CORPUS_SHA256_PIN`); a silent change to the
  generator fails the `corpus_is_deterministic_and_pinned` test loud.
- The fail-loud floor test (`corpus_kappa_meets_safety_floor` /
  `SafetyCriticalCorpus::validate`) fails if the corpus shrinks below 150 per
  Spirit or κ drops below 0.7.

When the real human-annotated labels land (a later story), the stand-in generator
is replaced by loading the annotated label fixtures — a **wiring change**, not a
methodology change: `cohen_kappa`, the floors, the `IaaAttestation` shape, and the
MANIFEST/coverage-matrix registration are all already in place. This is the same
seam-closure pattern Butler (8.1) used to close the 7.5b fixture stand-in.

## 4. Cohen's κ definition (as implemented)

```
κ = (p_o − p_e) / (1 − p_e)
```

where `p_o` is the observed proportion of scenarios on which the two annotators
agree, and `p_e` is the agreement expected by chance from each annotator's marginal
label frequencies (`p_e = Σ_k (p_a,k · p_b,k)` over categories `k`). Perfect
agreement → `κ = 1.0`; chance-level agreement → `κ ≈ 0.0`; systematic disagreement
→ `κ < 0`. The implementation is deterministic and unit-tested against the
perfect-agreement, chance-level, and total-disagreement reference cases.
