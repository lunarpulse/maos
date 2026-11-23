# Third-Party Trial — N=12 Ship-Gate Protocol (Story 10.2)

**Release criterion:** v1.0 / v1.5. This document is the **reproducible
protocol** for the N=12 stratified third-party trial that gates the MAOS ship
decision. The trial itself is an **out-of-band human-research activity** — this
repo ships the *gate-execution infrastructure* (Story 10.2): this protocol, the
results schema, the Wilson-CI rationale, the SBOM/signing runbook, and the
`check-third-party-trial` CI gate. **No part of Story 10.2 recruits participants
or runs the 14-day trial.**

The gate reports missing results as **ABSENT**, never a pass, and an empty cohort
(N = 0) as **INDETERMINATE**, also not a pass; both exit 0 with a warning at v1.0/v1.5
and exit 1 at v2.0. Integrity failures are fatal at every N.

> **Story 22-7.** The gate no longer has a human-author denominator: no N = 12, no ≥ 10
> successes, no strata, no Wilson CI (§7). The Spirit-authoring evidence is now agent-run
> records (`docs/agent-runs/22-7/`), labelled agent-authored. No human N = 12 cohort has been run,
> and a green `check-third-party-trial` does not claim one.

## 1. Cohort target & stratification floor

**Floor policy date: 2026-10-15 (superseded by Story 22-7: there is no sampling floor).**

A recruited cohort should still be varied; the strata below are **recruiting guidance only**. Since Story 22-7
`check-third-party-trial` does not read them, count them or enforce a floor, and a results file needs no
`stratum` field.

| # | Stratum | Guidance | Screener question |
|---|---|---|---|
| 1 | No prior MAOS contribution | ≥ 4 of 12 | Have you ever contributed to MAOS (code, docs, issues, review)? |
| 2 | Never wrote a Rust Spirit | ≥ 3 of 12 | Have you ever written a MAOS Spirit in Rust before? |
| 3 | Never wrote Rust at all | ≥ 2 of 12 | Have you ever written any Rust before (Spirit or not)? |
| 4 | Non-English-native | ≥ 2 of 12 | Is English your native language? |
| 5 | Working offline-only | ≥ 1 of 12 | Will you complete the trial working offline-only (no network during the task)? |

The strata are **not** mutually exclusive — one participant may satisfy several.
Guidance of this kind keeps a cohort from being silently skewed toward MAOS insiders or
expert Rustaceans. Participant identifiers
in any committed artifact are opaque (`P001`…`P012`) — never real names.

## 2. The 14-day zero-DM-support window

Each participant has **14 days** from kit delivery to produce a passing Spirit.
During the window:

- **Zero direct-message support.** Maintainers MUST NOT answer participant
  questions over DM, email, private chat, or call. **A DM-support breach
  invalidates that participant's run** (the run is dropped from the cohort, not
  scored as a failure — a maintainer error must not be laundered into a data
  point).
- **All support is routed through the public issue tracker.** A participant who
  is stuck opens a public issue; the answer (and the friction that prompted it)
  becomes part of the durable record. *If it isn't in the public tracker, it
  didn't happen.*

The window models the real onboarding experience: a stranger with the public
docs and nothing else.

## 3. Trial environment setup

Every participant starts from a **clean Host VM** so the result reflects only
the public docs and toolchain, not a maintainer-tuned machine.

1. **Provision a fresh Host VM** (e.g. a clean Ubuntu install). No MAOS
   artifacts, credentials, or caches are pre-loaded.
2. **Install the substrate** from the public release channel:

   ```sh
   maosctl install
   ```

   `maosctl install` pulls the kernel substrate and CLI; verify with
   `maosctl --version`.
3. **Scaffold from the reference Spirit template** — the same template the
   `write-a-spirit` door publishes:

   ```sh
   cargo generate --git https://github.com/lunarpulse/maos templates/spirit-rust \
     --name trial-spirit
   ```

   The in-repo `spirits/hello-spirit` and `spirits/worker` manifests serve as
   known-good reference shapes for manifest fields, capabilities, and posture.
4. **Build, load, and run** the Spirit locally against the kernel; record frames
   run and halt-recall for the results file.

Participants who declared `offline-only` (stratum 5) repeat the whole sequence
with the network disabled after the initial `maosctl install` / template fetch.

## 4. Success criteria

A participant **succeeds** when all four hold (the conjunction the gate asserts
per participant) — i.e. *a Spirit binary that loads + ≥1000 frames run +
halt-recall ≥ 0.85*:

| Criterion | Field | Threshold |
|---|---|---|
| Produced a Spirit binary | `produced_binary` | `true` |
| The kernel loads it | `binary_loads` | `true` |
| Frames run | `frames_run` | ≥ 1000 |
| Halt-recall | `halt_recall` | ≥ 0.85 |

The gate reports every cohort's per-participant outcome; it does not require 10 of 12 (Story 22-7). SBOM and
signing-chain verification (`sbom-signing-verification.md`) are recorded per participant but are
**operational, not gate logic** at v1.0/v1.5 — see §5 and F6→C.

## 5. Trust model (F4→A)

**Pull-request review is the trust boundary at N=12.** There is **no GPG /
release-key signing requirement** on participant-produced Spirits at this cohort
size — the cost and key-distribution friction would distort a 12-person trial,
and a reviewed PR is the load-bearing control instead. Specifically (F4→A):

- A participant's Spirit enters the trial corpus only via a reviewed PR; the
  review is the integrity boundary, not a cryptographic signature.
- SBOM completeness and signing-chain verification (F6→C) are **operational
  checks** the CI bot records (`sbom_verified`, `signing_chain_verified` in the
  results schema) but **does not assert** as blocking. They exist to surface
  supply-chain health, not to gate the ship decision at N=12.
- Cryptographic signing of participant binaries is a v-next concern.

## 6. Participant consent / NDA (PLACEHOLDER STUB)

> **⚠️ Placeholder — not a legal document.** The block below is a structural
> stub for the consent/NDA text the trial coordinator will finalize with counsel
> before recruitment. It MUST NOT be used as-is. The final instrument must cover
> informed consent, data-handling and retention, the public-tracker disclosure
> rule (§2), and any confidentiality terms, and must be reviewed by qualified
> counsel before any participant is enrolled.

*[Consent and NDA text to be drafted by the engagement coordinator and legal
counsel prior to recruitment. Until finalized and reviewed, this section is a
non-binding placeholder and no participant may be enrolled against it.]*

## 7. Gate behavior

Story 22-7 (D-22-7-10 A) removed the human-author denominator from the gate: there is **no sampling floor**
(no N = 12, no ≥ 10 successes, no strata) and **no Wilson CI**. The gate judges the cohort it is given.

- **Results absent** → `passed:false`, `evidence_state:ABSENT`; warning and exit
  0 at v1.0/v1.5, exit 1 at v2.0.
- **Results present** → validate dates, nonnegative counts bounded by N, complete
  enumeration, unique IDs and declared/derived success-count equality. Invalid
  evidence always fails. A `trial-results.toml` needs no strata fields and no
  per-participant `stratum`; stale ones are ignored.
- **N = 0** → `trial_status:empty`, `passed:false`, `evidence_state:INDETERMINATE`
  (advisory at v1.0/v1.5, exit 1 at v2.0).
- **N ≥ 1 and every per-participant check passes** → green, and the reason says
  `N=<n>; no sampling floor (human-author denominator removed, 22-7)`.
- **Any per-participant finding** (a producer under 1000 frames or under 0.85
  halt-recall, a v2.0 attestation shortfall) → red, at any N. The v2.0
  producer-signed attestation leg and the production-key requirement are unchanged.

**A green gate does not claim the N = 12 human cohort.** NFR-Test-8 and NFR-Onb-1 are not measured for human
authors (`RELEASE-HOLDS.md`, "Claim boundaries — Spirit authoring kit", K2). The authoring kit's own evidence
is the agent-run records in `docs/agent-runs/22-7/`.

## 8. Companion artifacts

| Artifact | Path |
|---|---|
| Results schema (this trial) | `docs/third-party-trial/results/trial-results-schema.toml` |
| Actual results (committed post-trial) | `docs/third-party-trial/results/trial-results.toml` |
| SBOM + signing-chain runbook | `docs/third-party-trial/sbom-signing-verification.md` |
| CI gate implementation | `xtask/src/check_third_party_trial.rs` |
