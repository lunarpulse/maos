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
| workspace crates | `55` |

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

MAOS v1.5 carries a **2-year LTS commitment** (NFR-Maint-6): the v1.0 line
receives **security-only patches for 2 years** from the LTS clock-start below.
The 2-year term takes effect at v1.5, extending the original v1.0 1-year window.

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

<!-- PRESERVED:export -->
The MAOS substrate's cryptographic surface is classified **EAR99** under the
U.S. Export Administration Regulations (EAR). Cryptography is **ancillary** to
the primary AI-agent-orchestration function, so the surface falls **outside**
ECCN 5D002 ("Information Security") per the "ancillary cryptography" Note to
**5D002.c.1** — items whose cryptographic functionality is ancillary are not
5D002-controlled (and thus EAR99). The open-source-software aspect is
separately eligible under License Exception TSU, 15 CFR §740.13(e). Every
primitive is provided by already-classified, mass-market open-source libraries
(ring, ed25519-dalek, rustls).

Enumerated cryptographic surface (Story 10.3 AC-1): **HKDF-SHA256** key
derivation, **Ed25519** signing, **AES-256-GCM** AEAD sealed-export (via
`ring`), **TLS 1.3** cross-host transport, **SHA-256** content-addressing, and
**SHA-256** digests canonically encoded in **CBOR (RFC 8949)** for
ComplianceClaim fingerprinting (CBOR is the deterministic encoding, not a
cryptographic primitive). The full determination, dual-use review, and BIS
advisory citation are on file at `docs/compliance/eccn-classification.md`
(NFR-Comp-1).

This is an engineering classification, not legal export advice; it is **pending
export-compliance counsel review before v1.0 enterprise distribution**.
Operators and distributors must confirm applicability with their own counsel
and the jurisdiction of distribution.
<!-- END PRESERVED:export -->
