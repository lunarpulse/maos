---
Status: binding-v1.5
Gate: maos-eval safety-critical-corpus-8-5 job (κ ≥ 0.7, N ≥ 150/Spirit, SHA-pin); distillate floor κ ≥ 0.85 enforced by nfr-aud-7-distillate-five-metrics-floor
Decided: 2026-06-04
Accepted-in-PR: <PR_NUMBER>
Revisits: Story 8.5 Decision F; docs/safety-critical-corpus-methodology.md
---

# ADR-042 — Safety-critical κ floor (0.7) is distinct from the distillate κ floor (0.85)

**Decision.** Inter-annotator agreement (Cohen's κ) carries **two different floors** in MAOS, because it measures two different things:

- **Distillate corpus: κ ≥ 0.85** (Story 4.4, NFR-Aud-7) — annotators judge **hedge-preservation** over near-identical text.
- **Safety-critical Spirit corpus: κ ≥ 0.7** (Story 8.5, Decision F) — annotators assign a **3-way categorical scenario label** (`benign` / `caution` / `critical`).

The two floors are intentionally NOT harmonized. Each is the appropriate "substantial agreement" bar for its measurement's inherent noise.

**Rationale.** Hedge-preservation compares two close renderings of the same content; a small disagreement is genuine signal, so a high (0.85) floor is warranted. Safety-critical scenario labelling is a coarser categorical judgment where reasonable annotators draw the `caution`/`critical` boundary differently; **κ = 0.7** is "substantial agreement" on the Landis–Koch scale and is the defensible bar for that label class. Holding the categorical corpus to 0.85 would reject sound corpora for noise that is intrinsic to the task, not a quality defect. The single κ *computation* (`maos_eval::cohen_kappa`) is shared; only the floor differs by corpus.

**Alternatives considered.**
- *One unified κ floor (0.85 or 0.7) for all corpora* — rejected: a single number either over-rejects categorical corpora (at 0.85) or under-protects hedge corpora (at 0.7). The floor must track the measurement.
- *No floor for the safety-critical corpus, report-only* — rejected: the corpus gates a safety-critical reference deployment; a hard, fail-loud floor is required (the `safety-critical-corpus-8-5` job fails if κ < 0.7 or N shrinks below 150/Spirit).

**What would force a revisit.** A **third** κ floor appearing (a new corpus class) is the trigger to generalize this ADR into a per-corpus-class κ-floor table, so the rationale stays legible and no future maintainer "tidies" the floors into one value. Amendment via the ADR-037 process. The κ values + the stand-in 2-annotator protocol are documented in `docs/safety-critical-corpus-methodology.md`; the floors live as named constants (`maos_eval::safety_critical_corpus::SAFETY_CRITICAL_KAPPA_FLOOR = 0.7`; distillate floor in `distillate_corpus`).
