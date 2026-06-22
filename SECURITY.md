# Security Policy

The MAOS substrate ships with a coordinated security disclosure pipeline
in place from day one (v0.1-α). This document is the single source of
truth for how to report, what to expect, which versions get patches,
and where advisories are published.

## Reporting a vulnerability

**Contact:** `security@maos.dev`
**GPG / report-encryption key:** the MAOS project publishes **no pre-shared
OpenPGP/GPG key**. Per ADR-047 the trust root is **operator-local and
air-gap-compatible**: the Transparency Log *signing* key is derived from the
operator's seed via HKDF-SHA256 (a symmetric key-derivation function used for
verification — **not** a report-encryption key). There is no project-level key
to "encrypt the report with"; do not attempt the older "encrypt with the
published GPG key" instruction.

**Encrypted submission:** the primary channel is **GitHub Security Advisories**
(private vulnerability reporting) on this repository — it carries encrypted
submission to the maintainers with no pre-published key. Use `security@maos.dev`
only for coordination that does not fit a GHSA draft.

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
- The security team triages, develops a fix, and publishes a CVE. **Once the
  MAOS CNA scope is assigned** (MITRE CNA registration application submitted
  2026-06-22; scope: `lunarpulse/maos`; pending assignment — see
  `docs/compliance/cna-registration.md`), CVEs are published via the MAOS CNA;
  **until then** they are requested through the MITRE general-form channel.
- Extensions beyond 90 days require mutual agreement, documented in
  the disclosure thread.

If the embargo lapses without acknowledgment from the security team
(receipt-of-report SLA is **5 business days**), the reporter is free to
disclose under their own timeline.

## Supported versions

Backports of security patches target the following versions:

| Version range | Status               | Backport window  |
|---------------|----------------------|------------------|
| `1.0.x`       | LTS (security-only)  | 1-year LTS window from the v1.0 tag (NFR-Maint-6) |
| `0.1.x`       | Superseded by 1.0.x  | Security-only backports on request during the v1.0 LTS window |
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

*This policy is binding at v1.0 (NFR-Ops-4 ship gate, FR61). MITRE CNA
registration application submitted 2026-06-22 — see
`docs/compliance/cna-registration.md`.*
