# Epic 19 — W4: Ship It (v1.0)

**Status:** `backlog` — created 2026-09-04 by `sprint-change-proposal-2026-09-04.md` (bmad-correct-course, Lunarpulse-ratified). One of six recovery epics (15–20); `sprint-status.yaml` is authoritative for status.

**Wave / duration:** 4–6 weeks · **Makes true:** v1.0 · team-ready, third-party Spirits ship

**Closes:** S4 (gates judge files), S5 (never shipped); FR1, FR3, FR33, FR35, FR36, FR37, FR59, FR60; NFR-Onb-1, NFR-Sec-5, NFR-Sec-7 (scheduling), NFR-Ops-9, NFR-Meta-3; register D1, D8, D9, D20

**Exit command (the epic is done when an operator runs this on a clean machine and sees what it says):**

```
brew install lunarpulse/maos/maos && maosctl install researcher@1.0 && maos shell
@researcher survey LLM-as-judge literature, 90 minutes
# v1.0.0 tag starts the LTS clock STABILITY.md has been waiting for
```

**Kernel-Δ:** **ZERO kernel-Δ @24472.**

**Dev-gate:** external holds (pen-test NFR-Sec-7 + export counsel NFR-Comp-1) = GA ledger only, non-gating for dev. **Model/review discipline:** frontier-class dev allowlist + §A6 full-layer review net is the binding control (E11 retro A1). **Story rules (CP-2):** AC1 of every story is a command plus what the operator observes; a story splits once; story files ≤3,000 words; governance work ≤20% of the epic.

---

## Stories

| Key | Story | Closes |
|---|---|---|
| `19-1-real-registry-publish-install` | Real registry publish and install | FR33/35/36/37/59 |
| `19-2-packaging-from-release-workflow` | Packaging from the release workflow | FR1; NFR-Ops-12 |
| `19-3-external-author-cohort-and-pen-test-scheduling` | External-author cohort and pen-test scheduling | NFR-Onb-1/Test-8 (honest N); Hold 1 |
| `19-4-gate-honesty-pass` | Gate honesty pass | S4; D1/D8/D9/D20; NFR-Sec-5, NFR-Ops-9, NFR-Meta-3 |
| `19-5-backends-and-multi-provider` | Backends and multi-provider | FR3/FR48; §A6 non-degradable (secret backends) |

Per-story ACs below are sketches; `bmad-create-story` finalizes them at preflight (≤6 ACs, AC1 unchanged in kind).

### 19-1-real-registry-publish-install — Real registry publish and install

*Closes:* FR33/35/36/37/59

- **AC1 (command).** `maos-spirit publish --tier=local` reaches a self-hosted MCP-Streamable-HTTP registry; on a clean host `maosctl install <name>` pulls it and verifies signature, ComplianceClaim and trust floor (the verification that lives under `import` today moves under `install`).
- **AC2.** A Python template (`templates/spirit-py`) generates a Spirit that passes `spirit-test` (FR33 third language).
- **AC3.** A vetter CLI (`maos-spirit vet issue|revoke`) wraps `issue_attestation`, which has zero non-test callers today (FR37 machinery end to end).
- **AC4.** Yank events apply the operator policy (warn / quarantine / auto-revoke) instead of journaling only (FR59).

### 19-2-packaging-from-release-workflow — Packaging from the release workflow

*Closes:* FR1; NFR-Ops-12

- **AC1 (command).** `brew install lunarpulse/maos/maos` on a clean host installs the signed release; deb and AUR artifacts are produced by `release.yml`; no `SCAFFOLD` / `PLACEHOLDER_SHA256` remains under `packaging/`.
- **AC2.** The air-gap build variant is a release artifact and the Docker image is built from it; `check-air-gap` is enrolled in a workflow.
- **AC3.** `maos uninstall` from 16-4 removes what the package installed.

### 19-3-external-author-cohort-and-pen-test-scheduling — External-author cohort and pen-test scheduling

*Closes:* NFR-Onb-1/Test-8 (honest N); Hold 1

- **AC1 (command).** At least three external authors run the 30-minute gate; participants, dates and outcomes are recorded in `docs/third-party-trial/results/trial-results.toml` (the real file the gate parses), with N<12 labelled honestly.
- **AC2.** The first non-Lunarpulse Spirit is installed from the registry of 19-1.
- **AC3.** The NFR-Sec-7 pen-test engagement has dates; `RELEASE-HOLDS.md` Hold 1 moves from "not scheduled" to "scheduled".

### 19-4-gate-honesty-pass — Gate honesty pass

*Closes:* S4; D1/D8/D9/D20; NFR-Sec-5, NFR-Ops-9, NFR-Meta-3

- **AC1 (command).** Every evidence-file gate reds on absent evidence (`EvidenceState` for all 12); `check-fuzz-floor` and `check-rto-gate` read real ledger branches after one 24-hour fuzz and one restore drill.
- **AC2.** `nfr-rel-1`/`nfr-rel-2` assert the numbers in their names (from 16-3); NFR-Test-4 is a registered gate with per-class numbers (from 17-1).
- **AC3.** `tests/coverage-matrix.yaml` is generated from `gate-registry.toml` plus `#[maos_covers(FR..)]` test attributes; the hand-maintained file is deleted.
- **AC4.** Gate count 75 → ≤40 without raising the `xtask` ceiling; every gate carries `BindingClass` (closes E12-B1 residual, D20).
- **AC5.** No signed erasure attestation asserts a completion it did not observe (14-e1); `spec-epic-5-review-finding-closure` verification list green or retired with a reason.

### 19-5-backends-and-multi-provider — Backends and multi-provider

*Closes:* FR3/FR48; §A6 non-degradable (secret backends)

- **AC1 (command).** `MAOS_DEFAULT_PROVIDER=gemini maos shell` gives a live reply; Gemini, Bedrock and Vertex drivers exist; `check-model-currency` governs their IDs.
- **AC2.** Vault and cloud-KMS backends behind the `maos-secrets` provider trait (composes 11.4c org-KMS).
- **AC3.** `provider_endpoint_pin` is enforced at inference time, not only fingerprinted into ComplianceClaims (FR3).

## Dependencies

Epic 18 (the J1 demo is what the cohort installs). 19-4 before Epic 20-2 (the nightly needs honest evidence states).

## Not in this epic

Anything whose only deliverable is a gate, ledger, registry or ceiling and is not named above. v2.5 parking rows (`v25-*`) stay parked.
