# NFR-Onb-1 — 30-Minute First Spirit Validation Gate: Recruitment & Execution Protocol

**Release criterion:** v0.3. This document is the **reproducible protocol** for
the N=12 stratified human trial that evaluates NFR-Onb-1. The trial itself is an
**out-of-band human-research activity** — this repo ships the *gate-execution
infrastructure* (Story 7.5b): the protocol, screener, schemas, scoring harness,
cohort gate evaluator, and the NFR-Onb-4 cadence machinery. **No part of Story
7.5b recruits participants or runs the 14-day trial.**

> **Provisional-until-Butler note (LOCKED Decision 1).** Until Story 8.1 ships the
> canonical Butler corpus at `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl`,
> the scoring harness scores against the SHA-pinned 7.5b *fixture* stand-in and
> every verdict is stamped `provisional: true`. A provisional PASS is **not** the
> live v0.3 gate.

## 1. Cohort target & stratification floor

The cohort is **N = 12** participants. The cohort is PASS-eligible only if it
meets every stratum floor below (enforced mechanically by
`maos_eval::onboarding_gate_corpus::validate_stratification`; a deficient cohort
FAILs with the deficient stratum named):

| # | Stratum | Floor | Screener question |
|---|---|---|---|
| 1 | No prior MAOS contribution | ≥ 4 | Q1 |
| 2 | Never written a Rust Spirit | ≥ 3 | Q2 |
| 3 | Never written Rust at all | ≥ 2 | Q3 |
| 4 | Non-English-native | ≥ 2 | Q4 |
| 5 | Working offline-only | ≥ 1 | Q5 |

The strata are **not** mutually exclusive — one participant may satisfy several.
The floors guarantee the cohort is not silently skewed toward MAOS insiders or
expert Rustaceans, which would inflate the success rate.

## 2. The 14-day zero-DM-support window

Each participant has **14 days** from kit delivery to produce a passing first
Spirit. During the window:

- **Zero direct-message support.** Maintainers MUST NOT answer participant
  questions over DM, email, private chat, or call. **A DM-support breach
  invalidates that participant's run** (the run is dropped from the cohort, not
  scored as a failure — a maintainer error must not be laundered into a data
  point).
- **All support is routed through the public issue tracker.** A participant who
  is stuck opens a public issue; the answer (and the friction that prompted it)
  becomes part of the durable record. This is the support-routing rule: *if it
  isn't in the public tracker, it didn't happen.*

The window models the real onboarding experience: a stranger with the public
docs and nothing else.

## 3. Outcome tracking

For each participant the harness records one `outcomes.jsonl` row (schema:
`nfr-onb-1-outcomes.schema.json`) capturing, per the NFR-Onb-1 floor:

- `compiles_against_abi` — did their Spirit build against the published ABI?
- `corpus_pass` — did it produce a decision for all 30 Butler-class scenarios?
- `halt_recall_calendar_conflict` — recall on the calendar-conflict subset (floor ≥ 0.90)
- `halt_precision_overall` — precision overall (floor ≥ 0.85)
- `time_to_success_min` — wall-clock minutes to first passing Spirit
- `succeed` — the NFR-Onb-1 conjunction: `compiles_against_abi ∧ corpus_pass ∧
  recall ≥ 0.90 ∧ precision ≥ 0.85`, completed within the window.

The **cohort gate** (`evaluate_cohort`) PASSes only when **≥ 10 of 12 succeed,
median `time_to_success_min` ≤ 45, and p95 ≤ 90**, naming every failing
sub-criterion.

## 4. NFR-Onb-4 — iteration cadence (this is an operational commitment, not a one-shot)

NFR-Onb-1 is not passed once and forgotten. The NFR-Onb-4 cadence
(`CadenceMachine`) ledgers every gate run:

- On a **miss**, the directive **"run a fresh 6-author cohort within 2 weeks"**
  is surfaced and recorded in the private run-ledger
  (`_research/nfr-onb-1/v0.3/run-ledger.jsonl`; schema:
  `nfr-onb-1-run-ledger.schema.json`).
- **3 consecutive misses** raise an `EscalateReleaseReview` signal to the
  PRD-author + architecture lead + research lead.
- A **PASS resets** the consecutive-miss counter.

## 5. Private vs committed artifacts

Participant data is private. The repo commits **only schemas + redacted
examples**; the live data lives under `_research/` which is **gitignored**:

| Artifact | Location | Committed? |
|---|---|---|
| Cohort schema | `docs/research/nfr-onb-1-cohort.schema.json` | ✅ |
| Outcomes schema | `docs/research/nfr-onb-1-outcomes.schema.json` | ✅ |
| Run-ledger schema | `docs/research/nfr-onb-1-run-ledger.schema.json` | ✅ |
| Redacted example cohort | `docs/research/examples/cohort.example.json` | ✅ |
| Redacted example outcomes | `docs/research/examples/outcomes.example.jsonl` | ✅ |
| Recruitment log | `_research/nfr-onb-1/v0.3/recruitment-log.jsonl` | ❌ (private) |
| Live outcomes | `_research/nfr-onb-1/v0.3/outcomes.jsonl` | ❌ (private) |
| Run-ledger | `_research/nfr-onb-1/v0.3/run-ledger.jsonl` | ❌ (private) |

Participant ids in committed examples are opaque (`P00`…`P11`) — never real names.
