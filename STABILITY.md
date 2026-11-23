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
| `kernel_version` | `1.0.0-rc.2` |
| release channel | release candidate — pre-release; LTS clock not started |
| `abi_version` | `1` |
| `manifest_schema_version` (current) | `9` |
| supported schema window | `1..=9` |
| `[cli_wrapper]` `manifest_schema_version` (current; absent means 1) | `2` |
| supported `[cli_wrapper]` window | `1..=2` |
| workspace crates | `60` |

| Manifest schema | Kernel behavior |
|---|---|
| `manifest_schema_version = 9` (current) | ✅ strict load (`deny_unknown_fields`) |
| `manifest_schema_version = 8` (N-1) | ✅ within the supported version window (subject to declaration validation) |
| `manifest_schema_version < 1` | ⛔ hard refusal — typed `SecurityError::EAbiTooOld` at admit |
| `manifest_schema_version > 9` (future) | ⛔ hard refusal — typed `SecurityError::EAbiTooNew` (fail-closed; the operator is told a newer kernel is required) |
| `min_substrate_version` > running `kernel_version` | ⛔ hard refusal — typed `SecurityError::ESubstrateTooOld` (FR8) |
| `[cli_wrapper]` `manifest_schema_version` outside `1..=2` | ⛔ hard refusal before the strict parse — typed `CliWrapperAdmissionError::ECliWrapperSchemaUnsupported` |

The version gate is **fail-closed in both directions**: an out-of-window
manifest is refused with an actionable typed error, never silently
warned-and-admitted (a manifest is a security artifact; a silent ignore would be
fail-open).

Schema 8 was ratified for Story 21-9 as `21-9-manifest-schema-8`.
The `abi_version` and supported manifest floor remain `1`. Schemas 1–7 still
support non-listeners, but an explicit `on_telemetry_event` listing or a
`[lifecycle.telemetry]` table below schema 8 is refused, not degraded away.
Schema 8 requires both declarations together for a listener.
This change does not alter the guest wire contract.

## Deprecations

Deprecated public surfaces follow the NFR-Maint-5 timeline: **2 minor releases of
warning, then 1 major release to remove.** Every `#[maos_attrs::deprecated_since(...)]`
surface MUST appear as a row below AND carry a dated entry in `BREAKING.md`; CI
(`stability-matrix --check`) enforces this cross-check.

| Surface | Deprecated since | Removal target | Migration |
|---|---|---|---|
| _(none at v1.0)_ | — | — | — |

## LTS Policy

MAOS v1.0 carries a **1-year LTS commitment** (NFR-Maint-6) from the LTS
clock-start below; **security-only patches after year 1**. A 2-year commitment
is a v1.5 decision, taken once support load is known. A release candidate
(`-rc.N`) carries no LTS commitment and never starts the clock; `v1.0.0` does
([`docs/release/lts-policy.md`](docs/release/lts-policy.md)).

<!-- lts-clock-start: read from the committed `xtask/release-channel.toml`, which `ops-ga-tag-after-holds` writes when it cuts `v1.0.0`; placeholder until then — do NOT fabricate a SHA/tag. -->
- **LTS clock-start:** pending — `v1.0.0` is cut by `ops-ga-tag-after-holds` (RELEASE-HOLDS Holds 1–2 are explicit claim boundaries, not preconditions), which writes `xtask/release-channel.toml`; no clock runs before then.

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
No ECCN determination was made; the surface is not classified.

The sections below are the MAOS authors' unreviewed engineering notes, not a
classification: no export-compliance counsel reviewed them, and no ECCN or
EAR99 determination was made. The engineering analysis argues that
cryptography is **ancillary** to the primary AI-agent-orchestration function
(the "ancillary cryptography" Note to **5D002.c.1**) and that the
open-source-software aspect may be eligible under License Exception TSU,
15 CFR §740.13(e); neither argument is a determination. Every primitive is
provided by already-classified, mass-market open-source libraries (ring,
ed25519-dalek, rustls).

Enumerated cryptographic surface (Story 10.3 AC-1): **HKDF-SHA256** key
derivation, **Ed25519** signing, **AES-256-GCM** AEAD sealed-export (via
`ring`), **TLS 1.3** cross-host transport, **SHA-256** content-addressing, and
**SHA-256** digests canonically encoded in **CBOR (RFC 8949)** for
ComplianceClaim fingerprinting (CBOR is the deterministic encoding, not a
cryptographic primitive). The enumeration and engineering notes are on file at
`docs/compliance/eccn-classification.md` (NFR-Comp-1 is not claimed as met).

This is an engineering note, not legal export advice; **no export-compliance
counsel review was performed and no ECCN determination was made (operator
ruling 2026-11-12: claim boundary; NFR-Comp-1 is not claimed as met)**.
Operators and distributors must confirm applicability with their own counsel
and the jurisdiction of distribution.

The default `maos` build includes the Wasmtime engine (the `wasm-host` feature
is part of the default feature set; operator ruling 2026-11-12, `wasm` = on).
An explicit engine-free build remains supported:
`cargo build -p maos-bin --release --no-default-features --features network`
contains no `wasmtime` packages and reports `wasm_engine_off` for a
WASM-component manifest. Whether the engine, or only the user-supplied WASM
Spirit it runs, engages **5D002.c.1** is an unresolved question that was
never put to counsel; the shipped default artifact therefore carries no
classification, no counsel clearance and no permission to distribute in any
jurisdiction. The earlier counsel merge precondition was superseded by the
operator ruling and is now a release claim boundary
(`docs/compliance/export-counsel-precondition.md`; `RELEASE-HOLDS.md`
Boundary 2). The wasmtime 46.0.2 RustSec advisories addressed by Story 15-5
did not reach a published default binary at that time.
<!-- END PRESERVED:export -->
