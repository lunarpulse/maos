---
Status: binding-v2.0
Gate: Story 11.7 — `check-trial-attestation` producer gate plus `check-third-party-trial` consumer v2.0 provenance default-deny; producer `{ v1_0 = advisory, v1_5 = advisory, v2_0 = blocking }`; consumer `{ v1_0 = blocking-when-present, v1_5 = blocking-when-present, v2_0 = blocking }`
Decided: 2026-07-07
Accepted-in-PR: Story-11.7
Revisits: Story-10.2 §5/F4→A; ADR-052
Covers: NFR-Test-8
---

# ADR-053 — Third-party trial attestation

**Decision.** MAOS v2.0 asserts SBOM and signing-chain truth only when a CI-bot producer derives those facts from a clean re-load, an independently recomputed dependency closure, `maos_audit::release_verify`, and the existing `maos-eval` halt scorer. Hand-authored trial results remain valid for the v1.0/v1.5 human-path trial, but at v2.0 a participant record without the producer provenance stamp is default-denied by `check-third-party-trial`.

## Context

Story 10.2 made the N=12 third-party trial auditable but not machine-derived: `sbom_verified` and `signing_chain_verified` were recorded in `trial-results.toml`, yet the gate did not block on them. That was correct for v1.0/v1.5, where the trust boundary is PR review plus the raw participant conjunction. It is not sufficient for v2.0 infrastructure because a hand-authored perfect file can silently bypass the SBOM/signing claims.

ADR-052 split FKCS into v2.0 infrastructure and v2.5 genuine external population. ADR-053 applies the same discipline to NFR-Test-8: v2.0 delivers the attestation infrastructure and an in-house Chinese-wall proxy proof of mechanism; Epic 14/v2.5 owns the genuine external N=12 execution.

## Decision

### 1. Two trust boundaries keyed by release tier

| Release tier | Cohort | Trust control | SBOM/signing disposition |
|---|---|---|---|
| v1.0/v1.5 | Hand-authored N=12 trial record, if present | PR review, raw count/strata reconciliation, participant success conjunction | Advisory fields recorded; not blocking |
| v2.0 | CI bot plus in-house Chinese-wall proxy proof of mechanism | Machine derivation stamp from clean re-load, independent Cargo.lock-vs-`cargo tree --locked` closure, Ed25519 release verification, halt scoring | Derived and asserted by gate; non-provenanced records default-denied |
| v2.5 | Genuine external N=12 trial, Epic 14 | Same v2.0 machine derivation, populated by real external participants | Blocking external evidence |

### 2. Producer derives facts, never trusts self-reports

`check-trial-attestation` proves the producer machinery. Its legs independently falsify: binary load/frame derivation, hermetic environment, SBOM closure, signing chain, halt recall, forged self-report, blind harness, proxy cohort, release-graph absence, and kernel ABI baseline. Attempted-but-vacuous legs hard-fail.

The SBOM claim is v2.0's closure bill of materials: candidate-shipped `Cargo.lock` is reconciled against independently recomputed `cargo tree --locked`. CycloneDX emission is deliberately deferred to v2.5.

### 3. Consumer default-denies non-provenanced v2.0 records

`check-third-party-trial` keeps v1.0/v1.5 behavior: absent trial results are advisory; present hand-authored results block only on raw count/strata/participant conjunction. At `MAOS_SHIP_PHASE=v2_0`, each participant must carry `derivation_provenance = "maos-trial-attestation-v2"`, and SBOM/signing join the per-participant success conjunction. A perfect hand-authored TOML without that stamp fails.

## Consequences

- New opt-in `maos-eval/trial-attestation` feature for producer-only derivation code. Normal release builds that depend on `maos-eval` do not compile the 11.7 harness.
- New `xtask` producer gate: `check-trial-attestation`.
- `check-third-party-trial` gains v2.0 provenance enforcement without weakening v1.0/v1.5.
- NFR-Test-8 is now phase-split: v2.0 infrastructure delivered; genuine external N=12 moved to Epic 14/v2.5.
- Kernel baseline stays unchanged at `src_lines = 23081`.

## Alternatives considered and rejected

- **Keep SBOM/signing as hand-authored booleans at v2.0.** Rejected: this is exactly the canned-green failure mode; a planted lie would stay green.
- **Assert CycloneDX emission at v2.0.** Rejected: v2.0 needs closure truth, not a new artifact format. Emission is the v2.5 half.
- **Use proxy score as external evidence.** Rejected: the in-house Chinese-wall proxy is proof of mechanism only. It is advisory and cannot satisfy the genuine external N=12 floor.
- **Fold producer code into release binaries.** Rejected: trial derivation is tooling; it is opt-in and absent from the normal release graph.

## Gate

`check-trial-attestation` is advisory at v1.0/v1.5 and blocking at v2.0. `check-third-party-trial` is blocking-when-present at v1.0/v1.5 and blocking at v2.0 when records are present without derivation provenance. Together they implement the producer/consumer seam: a number is green only if a machine derived it and a planted lie would have turned it red.
