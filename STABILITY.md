<!-- GENERATED FILE — do not edit by hand.
     Source of truth: workspace state (maos-spirit-abi constants + Cargo.toml).
     Regenerate: `cargo run -p xtask -- stability-matrix`
     CI rail:     `cargo run -p xtask -- stability-matrix --check` (Story 7.5a, NFR-Maint-4). -->

# MAOS ABI Stability Commitments

This document publishes the MAOS v1.0 **ABI Stability Triple** and the
compatibility, deprecation, and long-term-support guarantees the substrate
makes to Spirit authors and operators. It is GENERATED from the live workspace
(it is never hand-maintained); a third party can regenerate it byte-for-byte
with `cargo run -p xtask -- stability-matrix --check`.

## Compatibility Matrix

The **ABI Stability Triple** `(kernel_version, abi_version, manifest_schema_version)`
is the load-time compatibility contract. The running kernel REFUSES an
incompatible Spirit at admission with a typed error — this is enforced, not a
promise (see `SecurityManagerAdapter::admit_spirit`).

| Leg | Live value |
|---|---|
| `kernel_version` | `0.1.0-alpha` |
| `abi_version` | `1` |
| `manifest_schema_version` (current) | `3` |
| supported schema window | `1..=3` |
| workspace crates | `44` |

| Manifest schema | Kernel behavior |
|---|---|
| `manifest_schema_version = 3` (current) | ✅ strict load (`deny_unknown_fields`) |
| `manifest_schema_version = 2` (N-1) | ✅ supported (loads with WARN-level degradation notes) |
| `manifest_schema_version < 1` (N-2) | ⛔ hard refusal — typed `SecurityError::EAbiTooOld` at admit |
| `manifest_schema_version > 3` (future) | ⛔ hard refusal — typed `SecurityError::EAbiTooNew` (fail-closed; the operator is told a newer kernel is required) |
| `min_substrate_version` > running `kernel_version` | ⛔ hard refusal — typed `SecurityError::ESubstrateTooOld` (FR8) |

The version gate is **fail-closed in both directions**: an out-of-window
manifest is refused with an actionable typed error, never silently
warned-and-admitted (a manifest is a security artifact; a silent ignore would be
fail-open).

## Deprecations

Deprecated public surfaces follow the NFR-Maint-5 timeline: **2 minor releases of
warning, then 1 major release to remove.** Every `#[maos_attrs::deprecated_since(...)]`
surface MUST appear as a row below AND carry a dated entry in `BREAKING.md`; CI
(`stability-matrix --check`) enforces this cross-check.

| Surface | Deprecated since | Removal target | Migration |
|---|---|---|---|
| _(none at v1.0)_ | — | — | — |

## LTS Policy

MAOS v1.0 carries a **1-year LTS commitment** (NFR-Maint-6): the v1.0 line
receives **security-only patches for 1 year** from the LTS clock-start below. The
2-year LTS term is **deferred to v1.5** — v1.0 publishes the term "the v0.8 team
can cash," not an over-promised window.

<!-- lts-clock-start: filled by `stability-matrix` IFF a `1.0.0`/`v1.0.0` git tag exists (Epic 10 cuts the tag); placeholder until then — do NOT fabricate a SHA/tag. -->
- **LTS clock-start:** pending — the `1.0.0` tag is cut in Epic 10 (`epic-10-v10-ship-gate`); this fills automatically when the tag exists.

## Substrate-Self Compliance Scope

<!-- NFR-Comp-3 — full scope language (Story 9.5a). -->

The MAOS substrate draws a **kernel-as-service trust boundary**: the kernel
provides mechanisms (Transparency Log, capability mediation, sandbox tiers
T0–T3, ComplianceClaim envelopes, GDPR Art. 17 erasure cascade); it does
**not** assert compliance of any deployment, operator, or Spirit running on it.

**Compliance-framework scope is the OPERATOR's responsibility.**

| Framework | Substrate provides | Operator owns |
|---|---|---|
| **SOC 2** | Append-only audit trail (TL); capability-token TTL + PID binding; sandbox-tier enforcement; sealed-export for external audit | Control mapping; access reviews; monitoring; incident response |
| **ISO 27001** | Asset inventory via Spirit manifest + TL; cryptographic key derivation (HKDF-SHA256, operator-local seed); region-pinning (NFR-Comp-4) | ISMS scope; risk assessment; Statement of Applicability; corrective actions |
| **FedRAMP** | Pluggable crypto-provider seam (FR48) — FIPS-validated module is operator/distributor choice; boundary definition via sandbox tiers; continuous-monitoring data (TL + posture-delta) | System Security Plan (SSP); POA&M; 3PAO engagement; ATO package |

The trust root is **operator-local** and **air-gap compatible**: the
Transparency Log signing key is derived from the operator's seed via
HKDF-SHA256 with no online CA, OCSP, or key-server dependency
([ADR-047](docs/adr/ADR-047-trust-anchor-framing-carry-forward.md),
NFR-Ops-12). The substrate's competitive framing is
**substrate-as-substrate** — infrastructure in the Linux/Postgres/Kubernetes
reference class — not a certifying authority (ADR-047 §2, considered and
rejected).

## Export

<!-- full content: Story 10.3 (NFR-Comp-1) — this is the placeholder STUB. -->

Export-control classification (ECCN determination — e.g. EAR99 vs 5D002 for the
cryptographic surface) is pending the formal determination in Story 10.3. Do not
treat this section as legal export advice until that story lands.
