# Third-Party Trial — N=12 Ship-Gate Protocol (Story 10.2)

**Release criterion:** v1.0 / v1.5. This document is the **reproducible
protocol** for the N=12 stratified third-party trial that gates the MAOS ship
decision. The trial itself is an **out-of-band human-research activity** — this
repo ships the *gate-execution infrastructure* (Story 10.2): this protocol, the
results schema, the Wilson-CI rationale, the SBOM/signing runbook, and the
`check-third-party-trial` CI gate. **No part of Story 10.2 recruits participants
or runs the 14-day trial.**

The gate is **advisory-if-absent**: until
`docs/third-party-trial/results/trial-results.toml` is committed it PASSes with a
warning annotation and a step-summary notice, exactly like
`check-pentest-gate`. Once the results file lands, the gate activates and
asserts the blocking floors below.

## 1. Cohort target & stratification floor

The cohort is **N = 12** participants minimum. The cohort is PASS-eligible only
if it meets every stratum floor below (enforced by `check-third-party-trial`,
which FAILs with the deficient stratum named):

| # | Stratum | Floor | Screener question |
|---|---|---|---|
| 1 | No prior MAOS contribution | ≥ 4 | Have you ever contributed to MAOS (code, docs, issues, review)? |
| 2 | Never wrote a Rust Spirit | ≥ 3 | Have you ever written a MAOS Spirit in Rust before? |
| 3 | Never wrote Rust at all | ≥ 2 | Have you ever written any Rust before (Spirit or not)? |
| 4 | Non-English-native | ≥ 2 | Is English your native language? |
| 5 | Working offline-only | ≥ 1 | Will you complete the trial working offline-only (no network during the task)? |

The strata are **not** mutually exclusive — one participant may satisfy several.
The floors guarantee the cohort is not silently skewed toward MAOS insiders or
expert Rustaceans, which would inflate the success rate. Participant identifiers
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

The cohort **passes** when **≥ 10 of 12 participants succeed** *and* every
stratification floor in §1 is met. SBOM and signing-chain verification
(`sbom-signing-verification.md`) are recorded per participant but are
**operational, not gate logic** — see §5 and F6→C.

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
- Cryptographic signing of participant binaries is a v-next concern, revisited
  when the cohort scales (see the promotion threshold in `wilson-ci.md`).

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

- **Results absent** → `check-third-party-trial` PASSes with an advisory
  `warning` annotation and a step-summary notice. The gate is structural
  infrastructure only.
- **Results present** → the gate parses `trial-results.toml` against the typed
  schema, rejects negative counts and malformed ISO-8601 dates, then asserts the
  §1 stratification floors, `successes ≥ 10`, and the per-participant success
  conjunction from §4. Any deficiency FAILs the gate with the failing field
  named.
- **Wilson 95% CI** on the success rate is computed and written to the
  step-summary as **advisory only** (F5→A-prime) — it is logged, never asserted.
  See `wilson-ci.md`.

## 8. Companion artifacts

| Artifact | Path |
|---|---|
| Results schema (this trial) | `docs/third-party-trial/results/trial-results-schema.toml` |
| Actual results (committed post-trial) | `docs/third-party-trial/results/trial-results.toml` |
| Wilson CI rationale | `docs/third-party-trial/wilson-ci.md` |
| SBOM + signing-chain runbook | `docs/third-party-trial/sbom-signing-verification.md` |
| CI gate implementation | `xtask/src/check_third_party_trial.rs` |
