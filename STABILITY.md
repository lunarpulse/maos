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
| `manifest_schema_version` (current) | `2` |
| supported schema window | `1..=2` |
| workspace crates | `44` |

| Manifest schema | Kernel behavior |
|---|---|
| `manifest_schema_version = 2` (current) | ✅ strict load (`deny_unknown_fields`) |
| `manifest_schema_version = 1` (N-1) | ✅ supported (loads with WARN-level degradation notes) |
| `manifest_schema_version < 1` (N-2) | ⛔ hard refusal — typed `SecurityError::EAbiTooOld` at admit |
| `manifest_schema_version > 2` (future) | ⛔ hard refusal — typed `SecurityError::EAbiTooNew` (fail-closed; the operator is told a newer kernel is required) |
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

<!-- full content: Story 9.5 (NFR-Comp-3) — this is the structural-presence STUB. -->

The MAOS substrate itself is assessed against, and its boundary scoped relative
to, the following regimes: **SOC 2**, **ISO 27001**, **FedRAMP**, and the
**kernel-as-service trust boundary**. The substrate provides the mechanisms
(transparency log, capability mediation, sandbox tiers, ComplianceClaim
envelopes); **mapping a concrete deployment to any specific control framework is
the OPERATOR's responsibility.** Full scope language lands in Story 9.5.

## Export

<!-- full content: Story 10.3 (NFR-Comp-1) — this is the placeholder STUB. -->

Export-control classification (ECCN determination — e.g. EAR99 vs 5D002 for the
cryptographic surface) is pending the formal determination in Story 10.3. Do not
treat this section as legal export advice until that story lands.
