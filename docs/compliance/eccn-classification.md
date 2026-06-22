# Export-Control Classification — MAOS Substrate

| Field | Value |
|---|---|
| Artifact | ECCN classification (NFR-Comp-1) |
| Story | 10.3 (v1.0 ship gate) |
| Determination date | 2026-06-22 |
| Classification | **EAR99** |
| Gate | `cargo run -p xtask -- check-export-control` (disposition `v1_0 = "blocking"`) |

> **Engineering classification, not legal export advice.** This document is a
> best-effort engineering determination by the MAOS authors to support
> self-classification for distribution. It is binding only on the v1.0 ship
> gate's *completeness* (the artifact exists and enumerates the surface), not
> on legal correctness. Operators and distributors MUST confirm applicability
> with their own export-compliance counsel and the jurisdiction of distribution.
> A formal BIS advisory opinion (15 CFR §740.3) may be sought before enterprise
> distribution; this document is the substrate that request attaches to.

## 1. Determination: EAR99

The MAOS substrate is **AI-agent orchestration infrastructure** — its primary
function is lifecycle management, capability mediation, sandbox enforcement, and
audit/compliance *mechanisms* for Spirits (agents). Cryptography is **ancillary**
to that primary function, not the primary purpose of the software.

Under the U.S. Export Administration Regulations (EAR), cryptography software is
prima facie controlled under **ECCN 5D002** ("Information Security"). MAOS
qualifies for **EAR99** (items not specifically controlled) via two independent,
overlapping bases:

1. **Ancillary-cryptography classification — Note to ECCN 5D002.c.1.** Items
   whose cryptographic functionality is ancillary to the primary function (and
   not the principal purpose) fall OUTSIDE the scope of ECCN 5D002 ("Information
   Security") per the "ancillary cryptography" Note to 5D002.c.1, and are
   therefore classified EAR99 (not 5D002-controlled). MAOS's cryptographic
   operations (audit-log signing, sealed export, transport confidentiality) are
   subordinate to orchestration. MAOS does not present itself as, nor is it
   marketed as, an information-security product (ADR-047 §2: the substrate is
   *substrate-as-substrate*, in the Linux/Postgres/Kubernetes reference class —
   not a certifying authority).

2. **Mass-market / library reuse — 15 CFR §740.17(b) and §740.13(e)(3).** MAOS
   implements **no cryptographic primitive itself**. Every primitive is provided
   by already-classified, widely-available open-source libraries that carry their
   own EAR determinations:
   - `ring` (AEAD, HKDF) — EAR99-eligible mass-market.
   - `ed25519-dalek` (Ed25519) — EAR99-eligible mass-market.
   - `rustls` (TLS 1.3) — EAR99-eligible mass-market.

The combination — ancillary function + no self-implemented crypto + vetted
library reuse — places MAOS at **EAR99**. Re-classification is required if any
future change (a) makes cryptography the primary function, (b) ships a
self-implemented or non-public cryptographic primitive, or (c) adds a
non-mass-market cryptographic dependency.

## 2. Cryptographic Surface Enumeration (dual-use review)

Every cryptographic primitive reachable through the MAOS workspace surface is
enumerated below. The `check-export-control` gate asserts each primitive name
appears in this table.

| Primitive | Mechanism | Host crate(s) | EAR disposition |
|---|---|---|---|
| **HKDF-SHA256** | Key derivation — operator-seed → Transparency Log signing key (no online CA/OCSP) | `maos-iac` (via `ring`) | EAR99 (ancillary + mass-market lib) |
| **Ed25519** | Signing — capability tokens, signed revocations | `maos-kernel-core/capability` (via `ed25519-dalek`) | EAR99 (ancillary + mass-market lib) |
| **AEAD** | Sealed export — `CryptoProvider::seal_for_export` confidentiality for external audit bundles | `maos-kernel-core` (via `ring`) | EAR99 (ancillary + mass-market lib) |
| **TLS 1.3** | Cross-host A2A transport confidentiality + mTLS peer authentication | `maos-a2a-tcp` (via `rustls`) | EAR99 (ancillary + mass-market lib) |
| **SHA-256** | Content-addressing — Transparency Log frame digests, corpus hashes, manifest fingerprints | throughout (`sha2`) | EAR99 (hash functions: §740.13(b)/mass-market) |
| **CBOR** | Canonical deterministic encoding — ComplianceClaim fingerprint (RFC-8949 canonical) | `maos-compliance` (via `serde_cbor`) | EAR99 (encoding, not a cryptographic primitive; not 5D002) |

**No other cryptographic primitive is present in the workspace.** The gate
extends this table if a new primitive lands; an unenumerated crypto crate is a
ship-block.

## 3. BIS Advisory Citation

This self-classification follows the EAR's self-classification procedure (15 CFR
§740.3(a)). For enterprise/dual-use-sensitive distribution, the operator may
request a formal BIS advisory opinion citing:

> **Pending export-compliance counsel review.** This engineering
> self-classification is pending review by export-compliance counsel before
> v1.0 enterprise distribution; the citation correction above (re-citing the
> classification basis to the 5D002.c.1 ancillary Note) is a verifiable
> regulatory-text read, while "MAOS qualifies for EAR99" is a legal
> applicability opinion for counsel to confirm.

- ECCN **5D002.a.1** (encryption software) as the controlled alternative
  considered and rejected on the ancillary + mass-market bases above;
- the **"ancillary cryptography" Note to ECCN 5D002.c.1** as the classification
  basis (why the surface is EAR99, not 5D002); 15 CFR **§740.13(e)** (License
  Exception TSU — publicly-available/open-source encryption source code) and
  **§740.17(b)** (mass-market encryption) as defense-in-depth authorizations;
- ADR-047 (trust-anchor framing) for the operator-local, air-gap-compatible
  trust root (no key escrow, no online authority).

## 4. Trust Root (ADR-047)

The cryptographic trust root is **operator-local and air-gap compatible** (see
`STABILITY.md §Substrate-Self Compliance Scope` and ADR-047). The Transparency
Log signing key is derived from the operator's seed via HKDF-SHA256 with no
online CA, OCSP, or key-server dependency. This is consistent with EAR99: there
is no key-recovery/escrow facility (which would engage 5D002.b) and no real-time
confidentiality service offered to third parties.

## 5. Re-classification Triggers

This EAR99 determination is invalidated if ANY of the following lands without a
re-classification and a refreshed gate enumeration:

1. A self-implemented cryptographic primitive (not delegated to a vetted library).
2. A non-mass-market cryptographic dependency (e.g. a non-commercial license or
   key-length >256 symmetric / >512 ECC outside §740.17 scope).
3. Cryptography becoming the *primary* marketed function of the substrate.
4. Addition of a key-escrow, key-recovery, or real-time-confidentiality service.

The `check-export-control` gate's enumeration is the mechanical floor; the legal
determination above is the substantive basis. Both must stay in agreement.
