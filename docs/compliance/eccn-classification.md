# Export-Control Classification — MAOS Substrate

| Field | Value |
|---|---|
| Artifact | ECCN classification (NFR-Comp-1) |
| Story | 10.3 (v1.0 ship gate) |
| Engineering notes dated | 2026-06-22 (no determination; see below) |
| Classification | **None — no ECCN determination was made; the surface is not classified.** |
| Gate | `cargo run -p xtask -- check-export-control` (disposition `v1_0 = "blocking"`) |

> **Engineering notes, not a classification and not legal export advice.** No
> export-compliance counsel reviewed this document and no ECCN or EAR99
> determination was made (operator ruling 2026-11-12: NFR-Comp-1 is a claim
> boundary and is not claimed as met; see `RELEASE-HOLDS.md` Boundary 2). The
> `check-export-control` gate asserts only *completeness* (the artifact exists
> and enumerates the surface), not legal correctness. Operators and
> distributors MUST obtain their own export-compliance counsel advice for the
> jurisdiction of distribution. A formal BIS advisory opinion (15 CFR §740.3)
> was not sought.

## 1. Status: no determination (engineering analysis retained)

The MAOS substrate is **AI-agent orchestration infrastructure** — its primary
function is lifecycle management, capability mediation, sandbox enforcement, and
audit/compliance *mechanisms* for Spirits (agents). Cryptography is **ancillary**
to that primary function, not the primary purpose of the software.

Under the U.S. Export Administration Regulations (EAR), cryptography software is
prima facie controlled under **ECCN 5D002** ("Information Security"). The
authors' unreviewed engineering analysis argues the surface may fall outside
5D002 (and so be EAR99) on two overlapping bases. **This is a candidate
argument for counsel to evaluate, not a determination:**

1. **Ancillary-cryptography — Note to ECCN 5D002.c.1.** Items
   whose cryptographic functionality is ancillary to the primary function (and
   not the principal purpose) fall outside the scope of ECCN 5D002 per the
   "ancillary cryptography" Note to 5D002.c.1. MAOS's cryptographic
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

The WASM hosting surface (the Wasmtime engine, `maos-wasm-host`,
`maos-wasm-runner`) is a separate open question: whether 5D002.c.1 is engaged
by the engine itself or only by the user-supplied WASM Spirit is undecided (see
`export-counsel-precondition.md`). The default `maos` build includes that
engine; an explicit engine-free build remains supported.

Re-analysis would be required if any future change (a) makes cryptography the
primary function, (b) ships a self-implemented or non-public cryptographic
primitive, or (c) adds a non-mass-market cryptographic dependency.

## 2. Cryptographic Surface Enumeration (dual-use review)

Every cryptographic primitive reachable through the MAOS workspace surface is
enumerated below. The `check-export-control` gate asserts each primitive name
appears in this table. The last column is the authors' unreviewed engineering
note; it is not a determination.

| Primitive | Mechanism | Host crate(s) | Engineering note (not a classification) |
|---|---|---|---|
| **HKDF-SHA256** | Key derivation — operator-seed → Transparency Log signing key (no online CA/OCSP) | `maos-iac` (via `ring`) | Not classified; candidate argument: ancillary + mass-market lib |
| **Ed25519** | Signing — capability tokens, signed revocations | `maos-kernel-core/capability` (via `ed25519-dalek`) | Not classified; candidate argument: ancillary + mass-market lib |
| **AEAD** | Sealed export — `CryptoProvider::seal_for_export` confidentiality for external audit bundles | `maos-kernel-core` (via `ring`) | Not classified; candidate argument: ancillary + mass-market lib |
| **TLS 1.3** | Cross-host A2A transport confidentiality + mTLS peer authentication | `maos-a2a-tcp` (via `rustls`) | Not classified; candidate argument: ancillary + mass-market lib |
| **SHA-256** | Content-addressing — Transparency Log frame digests, corpus hashes, manifest fingerprints | throughout (`sha2`) | Not classified; candidate argument: hash function (§740.13(b)) / mass-market |
| **CBOR** | Canonical deterministic encoding — ComplianceClaim fingerprint (RFC-8949 canonical) | `maos-compliance` (via `serde_cbor`) | Not classified; encoding, not a cryptographic primitive |

**No other cryptographic primitive is present in the workspace.** The gate
extends this table if a new primitive lands; an unenumerated crypto crate is a
ship-block.

## 3. BIS Advisory Citation

No BIS advisory opinion was requested and no self-classification under the
EAR's procedure (15 CFR §740.3(a)) was completed. An operator or distributor
who wants one may cite:

> **Not reviewed by export-compliance counsel.** These notes are pending any
> counsel review; the citation of the 5D002.c.1 ancillary Note is a read of
> regulatory text, while whether MAOS qualifies for EAR99 is a legal
> applicability opinion that nobody has given.

- ECCN **5D002.a.1** (encryption software) as the controlled alternative the
  engineering notes would need to rule out on the ancillary + mass-market bases;
- the **"ancillary cryptography" Note to ECCN 5D002.c.1** as the candidate
  basis for a non-5D002 outcome; 15 CFR **§740.13(e)** (License Exception TSU —
  publicly-available/open-source encryption source code) and **§740.17(b)**
  (mass-market encryption) as candidate authorizations;
- ADR-047 (trust-anchor framing) for the operator-local, air-gap-compatible
  trust root (no key escrow, no online authority).

## 4. Trust Root (ADR-047)

The cryptographic trust root is **operator-local and air-gap compatible** (see
`STABILITY.md §Substrate-Self Compliance Scope` and ADR-047). The Transparency
Log signing key is derived from the operator's seed via HKDF-SHA256 with no
online CA, OCSP, or key-server dependency. There is no key-recovery/escrow
facility (which would engage 5D002.b) and no real-time confidentiality service
offered to third parties; that is a factual description of the design, not a
classification.

## 5. Re-analysis Triggers

Any future determination would be invalidated, and the engineering notes above
must be refreshed, if ANY of the following lands:

1. A self-implemented cryptographic primitive (not delegated to a vetted library).
2. A non-mass-market cryptographic dependency (e.g. a non-commercial license or
   key-length >256 symmetric / >512 ECC outside §740.17 scope).
3. Cryptography becoming the *primary* marketed function of the substrate.
4. Addition of a key-escrow, key-recovery, or real-time-confidentiality service.

The `check-export-control` gate's enumeration is the mechanical floor only; it
does not stand in for a legal determination, which has not been made.
