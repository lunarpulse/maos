# Appendix E — v0.9+ Compliance Roadmap

This appendix describes the ComplianceClaim semantic evaluator that ships at v0.9 and the associated corpus methodology. The schema (defined in `maos-spirit-abi/src/compliance.rs`) is binding from v0.1; the **evaluator and corpus are non-binding until v0.9**, per ADR-005's general staging principle: stable schema first, semantic evaluation later.

## E.1 — CCAC staging table

Each phase has a falsifiable gate. The schema does not move; only the corpus and evaluator phase in.

| Version | Schema | Generation mechanism | Corpus N | Cross-Spirit agreement gate | Ship-blocking? |
|---|---|---|---|---|---|
| v0.1 | Frozen, validator implemented, emit pipeline live | N/A — mechanism specified, not exercised | 0 (smoke fixtures only, N≈10) | Schema validation 100%, emit-rate 100% | Yes — schema |
| v0.5 | Stable | **Mechanism A** (cross-Spirit independent re-decision on shared input set) | N=100, stratified across 4 decision classes | **Non-degeneracy criterion** (no fixed numeric floor — see E.1.1 below) | Yes — non-degeneracy gate |
| v0.7 | Stable | A + first calibration pass against v0.5 distribution | N≥150 (v0.5 N=100 + 50 fresh stratified additions), expanded to 6 decision classes | **First numeric agreement floor (deferred-numeric, formula-bound)** — see E.1.2 below for the formula | Yes — agreement floor at the formula-computed value |
| v0.9 | Stable | A + **Mechanism B** (planted-disagreement injection — validates the metric is not degenerate) | **N=600**, full stratification across 8 decision classes, balanced across Spirits | **±2% agreement** | Yes — full corpus delivered, full floor |
| v1.0 | Stable, deprecation policy for schema fields published | A + B + **Mechanism C** (drift detection: re-run v0.9 corpus quarterly, flag agreement regression >0.5%) | N=600 active + drift-corpus | ±2% on active, ≤0.5% drift quarter-over-quarter | Yes — GA gate |

**E.1.1 — v0.5 non-degeneracy criterion (replaces the placeholder ±5% floor).** The v0.5 acceptance criterion is *non-degeneracy*, not a fixed numeric agreement floor. The agreement metric must satisfy all three of:

1. **Computable** across at least 3 distinct ComplianceClaim instances on the v0.5 N=100 stratified corpus.
2. **Non-constant** — variance >0 across those instances. If the metric returns the same value on every input, it is not measuring agreement; it is measuring a constant.
3. **Directionally correlated** with independent reviewer judgment on a sample of **N ≥ 30 paired claims** (judge metric value, reviewer "agree / disagree / unclear" coded as +1 / −1 / 0). Spearman's **ρ ≥ 0.40** required (two-tailed p < 0.05; ρ_crit at N=30 is 0.364, so ρ ≥ 0.40 carries effect-size headroom above the significance threshold). Additionally: 95% bootstrap CI on ρ (10,000 resamples) MUST NOT cross zero. A passing gate establishes that the metric is not behaving as a random scorer; it does NOT establish metric fitness — see App-E.1.2 (planned, v0.7) for accuracy gates.

**Derivation of (N=30, ρ ≥0.40).** Critical values for Spearman's ρ at α=0.05 two-tailed: N=10 → ρ_crit=0.648; N=20 → 0.450; N=30 → 0.364; N=50 → 0.279. The earlier draft used N=10 with ρ ≥0.30 — at N=10, ρ ≥0.30 corresponds to p ≈ 0.4, which trips ~40% of the time on uncorrelated noise. (N=30, ρ ≥0.40) is the smallest pair that delivers a statistically discriminating gate while keeping the corpus burden modest. Bootstrap CI requirement closes the residual edge case where a single outlier inflates ρ.

If those three conditions hold, the metric is admissible to v0.7's tightening pass. v0.7 introduces the first numeric agreement floor (App-E v0.7 row, calibrated against the v0.5 distribution); v1.0 enforces ±2%.

**Why no v0.5 numeric agreement floor.** The original draft of this table proposed ±5% as the v0.5 floor. There was no derivation for ±5% — it was chosen because it felt looser than the v1.0 ±2% by enough margin to be obviously a calibration phase, but "felt looser" is not a floor. The v0.5 question is "is this metric meaningful at all?" not "does it hit 5%?" The non-degeneracy criterion answers the former honestly.

**Falsification rule.** At v0.5, if any of the three non-degeneracy conditions fail (N < 30, ρ < 0.40, or bootstrap CI crosses zero), ship is blocked or v0.5 is rebadged as v0.4-preview. No "we'll figure it out by v0.9."

**E.1.2 — v0.7 agreement floor (deferred-numeric, formula-bound).** At v0.7 release, the numeric agreement floor is computed as `floor_v0.7 = max(floor_v0.5, μ_v0.5 − k·σ_v0.5)` where `μ_v0.5` and `σ_v0.5` are the mean and standard deviation of the per-scenario Spearman ρ distribution measured across the v0.5 production window (minimum 30 scenarios, ≥ 14 days post-v0.5-GA), and `k = 1.0` (one standard deviation below v0.5 mean, clamped to never regress below the v0.5 floor). Until the v0.5 distribution is measured, v0.7 inherits the v0.5 non-degeneracy criterion verbatim. **The formula, the window, the minimum-N, and `k` are frozen at this revision; only the numeric output is deferred.** No numeric range, no "calibrated" judgment call at v0.7 release — just a deterministic computation against measured v0.5 telemetry.

## E.2 — Generation mechanisms

- **Mechanism A — Cross-Spirit independent re-decision.** Two reference Spirits independently emit ComplianceClaim verdicts on the same input set. Agreement metric: per-class verdict-equality rate. Any input where ≥3 Spirits disagree is flagged for human adjudication and excluded from the agreement floor.
- **Mechanism B — Planted-disagreement injection.** Synthetic inputs constructed to produce a known-correct verdict. If two Spirits disagree on a planted input where the verdict is unambiguous, the metric is degenerate (Spirit disagreement is noise, not signal). N=30 planted inputs per decision class.
- **Mechanism C — Drift detection.** Quarterly re-run of the v0.9 corpus against the current kernel + Spirit versions. Agreement regression >0.5% triggers a `ComplianceClaimDrift` audit ticket and a forced ADR review of any Spirit whose verdicts shifted.

## E.3 — Why the schema ships at v0.1 but the evaluator does not

If the schema were deferred to v0.9, every ComplianceClaim emitted between v0.1 and v0.9 would either be undefined (no schema) or would require a v0.9-breaking schema change at v0.9 (ABI break by construction). Shipping the schema at v0.1 means:

- Spirits emit well-formed ComplianceClaim objects from day one. The structural validator catches malformed emit; the semantic evaluator does not exist yet.
- Operators see the audit trail of emitted claims even before the evaluator scores them. The trail itself is useful — "the substrate has been emitting compliance claims since v0.1" is a stronger story than "compliance arrives at v0.9."
- The crate boundary makes the document boundary honest: the schema is in the wire-stable `maos-spirit-abi` crate; the evaluator is in `maos-compliance` (v0.9+, isolated, can break freely until v0.9 ships).

This is the pattern for any substrate-level commitment with a deferred validation surface: schema lands early, validation phases in. The same shape applies to halt-precision/recall (§6.3 — spec at v0.1, corpus by v0.5), Diego onboarding (§10.6 — staged), and replay determinism (§7.3 — v1.0 best-effort, v1.5 hard target).
