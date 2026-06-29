# Wilson Score Interval — Advisory Rationale (Story 10.2)

The `check-third-party-trial` gate computes the **95% Wilson score confidence
interval** on the trial success rate and writes it to the GitHub step summary as
**advisory only** — it is logged, never asserted (F5→A-prime). This document is
the rationale for why a 12-person cohort's CI is directional rather than
load-bearing, and the conditions under which it could be promoted to blocking.

## 1. Formula

For `successes` out of `n`, with `p̂ = successes / n` and `z = 1.96` (the 95%
normal quantile), the Wilson score interval is:

```
z²     = z · z
denom  = 1 + z² / n
center = (p̂ + z² / (2·n)) / denom
margin = z · √( p̂·(1 − p̂)/n + z²/(4·n²) ) / denom
lower  = center − margin
upper  = center + margin
```

This is the exact computation performed by `wilson_ci(successes, n)` in
`xtask/src/check_third_party_trial.rs`, emitted to the step summary as
`successes={s} / n={n} → [{lower:.3}, {upper:.3}]`.

## 2. Worked example — N=12, successes=10

`p̂ = 10/12 = 0.833`. Plugging in:

- **CI = [0.552, 0.953]** (a ≈40-percentage-point band).

This is the value the gate logs for a 10/12 cohort (matching the `wilson_ci`
docstring). The point estimate (83%) looks strong, but the interval spans from
~55% to ~95% — wide enough that the *true* pass rate could plausibly be just
above half.

## 3. Why N=5 is meaningless (NFR-Test-8)

A 5-person cohort is below the floor this gate accepts, and not by accident: at
N=5 the Wilson band exceeds **60 percentage points** across the entire
decision-relevant success-rate range. For example, at `p̂=0.4` (2/5) the band is
≈65 points ([0.12, 0.77]); at `p̂=0.6` (3/5) it is ≈65 points ([0.23, 0.88]).
(`p̂=0.5` is not realizable at N=5; the continuous band peaks there.) An interval
that wide carries no usable information about the true rate — a "pass" could just
as easily be a coin flip. NFR-Test-8 therefore treats any N<12 result as
statistically void: it is not evidence for or against ship-readiness.

## 4. Why Wilson CI is advisory-only at N=12 (F5→A-prime)

At N=12 the band narrows to ≈40 points — better than N=5, but still a
**directional signal, not a precision instrument**. Gating on it would create two
problems:

1. **It cannot distinguish a real win from noise.** A 40-point band straddles
   the kind of threshold a gate would assert against, so a hard rule would flip
   on sampling luck rather than engineering quality.
2. **It would incentivize p-hacking.** If the CI were blocking, the cheapest way
   to "pass" would be to hunt for a success-rate/count combination whose band
   happens to clear the line — gaming the statistic instead of improving the
   product. The blocking decision is therefore carried by the **count floors**
   (≥ 10 of 12 succeed, §1 stratification) and the **per-participant success
   conjunction**, which are robust at N=12; the CI stays advisory as a
   directional health indicator (F5→A-prime).

## 5. Promotion threshold (future)

The CI may be promoted to a **blocking** assertion when **both** hold:

- **N ≥ 30** (the conventional small-sample floor), **and**
- the **realized CI band tightens below ≈20 percentage points.**

This is a conservative conjunction: at N=30 with a comparable success rate
(`p̂≈0.83`) the band is still ≈26 points, so in practice the 20-point bar is not
cleared until N approaches ~50. The criterion prevents promoting the CI until it
is genuinely precise enough to gate on, preserving the anti-p-hacking property.
