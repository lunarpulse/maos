### P2 — Bound the artifact-name sort
**scripts/minimize-fuzz-corpus.sh:202–204** · Confidence: 0.94

The artifact scan’s timeout covers `find`, but the subsequent `sort` runs outside `bounded`. If sorting stalls, the classifier blocks waiting on its process substitution, bypassing the advertised 1659-second bound. The workflow’s 30-minute timeout then fails the minimization step instead of preserving the corpus with an INCONCLUSIVE warning and exit 0. Remove the unnecessary sorting or bound it and handle failure explicitly.

**Verdict: no-ship — incorrect.** The RSS inheritance, exact prefix classification, evidence ordering, exit dispatch, marker upload path, and validation-before-swap otherwise match Correction 4.

Counts: P0: 0 · P1: 0 · P2: 1 · P3: 0. Static review only; no checks executed.