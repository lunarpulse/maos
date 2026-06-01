# NFR-Onb-1 — Participant Screener

This screener selects the N=12 cohort for the 30-Minute First Spirit Validation
Gate. Each question maps **1:1** to a stratification stratum in
`nfr-onb-1-protocol.md` §1 and to a boolean flag in the cohort manifest
(`nfr-onb-1-cohort.schema.json`). Answers are recorded as the flags so the cohort
can be validated mechanically by
`maos_eval::onboarding_gate_corpus::validate_stratification`.

> **Privacy:** record only the opaque participant id (`P00`…`P11`) and the five
> boolean answers. Do **not** store names, emails, or free text in the committed
> cohort manifest — recruitment contact details live only in the private
> `_research/nfr-onb-1/v0.3/recruitment-log.jsonl`.

## Questions

| # | Question | Stratum (floor) | Cohort flag | "Yes" sets flag to |
|---|---|---|---|---|
| **Q1** | Have you ever contributed to MAOS (code, docs, issues, review)? | No prior MAOS contribution (≥4) | `no_prior_maos_contribution` | **No** → `true` |
| **Q2** | Have you ever written a MAOS Spirit in Rust before? | Never wrote a Rust Spirit (≥3) | `never_wrote_rust_spirit` | **No** → `true` |
| **Q3** | Have you ever written *any* Rust before (Spirit or not)? | Never wrote Rust at all (≥2) | `never_wrote_rust` | **No** → `true` |
| **Q4** | Is English your native language? | Non-English-native (≥2) | `non_english_native` | **No** → `true` |
| **Q5** | Will you complete the trial working **offline-only** (no network access to docs/registry during the task)? | Offline-only (≥1) | `offline_only` | **Yes** → `true` |

Note the polarity: Q1–Q4 set their flag to `true` on a **"No"** answer (the flag
names the *deficit* being sampled for); Q5 sets `offline_only = true` on a
**"Yes"**.

## Recording

Each screened participant becomes one record in the cohort manifest:

```json
{
  "participant_id": "P00",
  "no_prior_maos_contribution": true,
  "never_wrote_rust_spirit": true,
  "never_wrote_rust": false,
  "non_english_native": false,
  "offline_only": false
}
```

A participant may satisfy several strata at once (the floors are non-exclusive).
See `docs/research/examples/cohort.example.json` for a complete, redacted N=12
cohort that PASSes the stratification floor.
