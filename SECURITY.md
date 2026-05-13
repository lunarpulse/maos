# Security Policy

The MAOS substrate ships with a coordinated security disclosure pipeline
in place from day one (v0.1-α). This document is the single source of
truth for how to report, what to expect, which versions get patches,
and where advisories are published.

## Reporting a vulnerability

**Contact:** `security@maos.dev`
**GPG public key fingerprint:** `<TO-BE-PUBLISHED>` (tracked at
issue [#TBD] — operator action item; the slot is a binding placeholder
at v0.1-α per Story 1a.4's `SECURITY.md` deliverable).

Encrypt the report with the published GPG key if it contains
exploit primitives, capability-token leak fragments, or other
material that should not transit cleartext mail.

Please include:
- A concise description of the vulnerability and its impact.
- Reproducer steps (corpus seed, manifest, capability scope, sandbox
  tier where applicable).
- Affected `maos` version (`maosctl --version`) and host OS.
- Suggested mitigation if known (optional).

## Coordinated-disclosure window

The MAOS substrate operates a **90-day coordinated-disclosure embargo**
by default (NFR-Ops-4 binding window). The clock starts when the report
is acknowledged by the security team. During the embargo:

- The reporter does not disclose publicly.
- The security team triages, develops a fix, and prepares a CVE
  request (CNA registration with MITRE lands at v0.5 per the
  NFR-Ops-4 phase-split; until then, CVEs are requested through the
  MITRE general-form channel).
- Extensions beyond 90 days require mutual agreement, documented in
  the disclosure thread.

If the embargo lapses without acknowledgment from the security team
(receipt-of-report SLA is **5 business days**), the reporter is free to
disclose under their own timeline.

## Supported versions

Backports of security patches target the following versions:

| Version range | Status               | Backport window  |
|---------------|----------------------|------------------|
| `0.1.x`       | Active development   | All security fixes during v0.1 phase |
| `< 0.1.0`     | Pre-release / unsupported | None       |

At v1.0+ the MAOS substrate will maintain a 2-year LTS branch policy
per NFR-Ops-2 (deferred to v1.5 maturation per the phased roadmap).

## Advisory channel

Published advisories appear in:
- GitHub Security Advisories on this repository
  (`https://github.com/lunarpulse/maos/security/advisories`).
- The MAOS substrate release notes for the fix-bearing version.

Advisories include: affected component, CVSS v3.1 severity, affected
versions, fixed version, mitigation guidance, and credit to the
reporter (with permission).

---

*This policy is binding at v0.1-α (NFR-Ops-4 ship gate, FR61). CNA
registration via MITRE lands at v0.5 per the NFR-Ops-4 phase-split.*
