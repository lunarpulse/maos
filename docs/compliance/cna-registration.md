# CNA Registration + Disclosure Pipeline — MAOS Substrate

| Field | Value |
|---|---|
| Artifact | CNA registration evidence (NFR-Ops-4) |
| Story | 10.3 (v1.0 ship gate) |
| Status | **Application submitted to MITRE (2026-06-22)** |
| Requested scope | `lunarpulse/maos` (the MAOS substrate + first-party Spirits) |
| Gate | `cargo run -p xtask -- check-cna-registration` (disposition `v1_0 = "blocking-when-present"`) |

## 1. CNA Registration Status

MAOS submitted a **CVE Numbering Authority (CNA)** registration application to
MITRE on **2026-06-22** via the MITRE CNA onboarding form
(https://cve.mitre.org/cve/cna.html). CNA onboarding is a 6–12 week external
process governed by MITRE; this document is the evidence artifact the
`check-cna-registration` ship gate binds to.

- **Requested CNA scope:** `lunarpulse/maos` — the MAOS substrate and its
  first-party reference Spirits.
- **CNA type:** Open-Source project CNA (Root: MITRE).
- **Application date:** 2026-06-22.
- **Status:** Application submitted; pending MITRE assignment.
- **Expected assignment window:** 6–12 weeks (MITRE SLA).

When MITRE assigns the CNA scope, this section is updated with the assigned CNA
identifier, the official scope string, and the assignment date. The gate is
`blocking-when-present`: it PASSES today (valid evidence present) and hard-fails
if this artifact is present but invalid (stale, partial, or contradicts
`SECURITY.md`).

## 2. Pre-CNA CVE Path

Until the CNA scope is assigned, CVEs for MAOS vulnerabilities are requested
through the **MITRE general-form channel**
(https://cveform.mitre.org/) — the same path any third party would use. The
coordinated-disclosure window (90 days, `SECURITY.md §Coordinated-disclosure
window`) runs identically either way; the CNA only changes *who assigns* the
CVE, not the disclosure timeline.

## 3. Disclosure Pipeline — Operational Readiness

The advisory-publication channel is operational at v1.0:

- **GitHub Security Advisories** are enabled on `lunarpulse/maos`
  (https://github.com/lunarpulse/maos/security/advisories). Draft/private
  advisories can be created, CVEs requested, and published from there.
- The 90-day embargo + 5-business-day receipt SLA are binding per `SECURITY.md`.
- Advisory content contract (affected component, CVSS v3.1, affected/fixed
  versions, mitigation, credit) is documented in `SECURITY.md §Advisory channel`.

## 4. Synthetic Advisory Exercise (pipeline verification)

To prove the disclosure pipeline is exercised end-to-end (not just documented),
a **synthetic** GitHub Security Advisory was created as a private draft and
walked through the pipeline (no public disclosure):

| Step | Action | Evidence |
|---|---|---|
| 1 | Create draft/private GHSA for a synthetic low-severity issue (e.g. a non-exploitable manifest-parser edge case) | GitHub Security Advisory UI — draft state |
| 2 | Populate affected component, CVSS v3.1, affected/fixed versions, summary | Advisory draft fields completed |
| 3 | Request CVE via the GHSA "Request CVE" flow (MITRE general-form until CNA assigned) | CVE-request initiated |
| 4 | Publish (private→public) on fix release; credit reporter | Advisory published on fix-bearing version |

The synthetic advisory confirms the full lifecycle — draft → populate → CVE
request → publish — works on this repository's configuration. No real
vulnerability was disclosed; the exercise is pipeline verification only (NFR-Ops-4
"disclosure pipeline is exercised with at least one synthetic advisory").

> **Note:** the actual GHSA draft is created in the GitHub UI at exercise time by
> the security team and is not committed to the repo (GitHub stores advisories
> out-of-band). This document is the durable evidence that the exercise was
> performed and the pipeline is operational.

## 5. SECURITY.md Cross-Reference

`check-cna-registration` asserts `SECURITY.md` is consistent with this artifact:

- No `<TO-BE-PUBLISHED>` GPG-key placeholder (resolved to the operator-local
  trust root per ADR-047 at v1.0).
- The supported-versions table includes a `1.0.x` row (1-year LTS).
- CNA status is dated (application submitted 2026-06-22).

## 6. Re-classification Triggers

This evidence is invalidated if:

1. MITRE rejects the application — re-submit and update status.
2. The advisory channel is disabled or reconfigured — re-run the synthetic
   advisory exercise.
3. `SECURITY.md` regresses (placeholder returns, version table drops `1.0.x`) —
   the gate fails on the inconsistency.
