# ADR-047: Trust-Anchor Framing Carry-Forward (NFR-Ops-8)

## Status

**Accepted** — ratified at Story 9.5a (2026-06-15). binding-v0.3 (v0.3 release-block; absence blocks the tag). Consistent with [ADR-005](ADR-005-pluggable-provider-drivers.md) (pluggable providers, substrate-not-product), [ADR-009](ADR-009-three-trust-tiers-with-strictest-of-floor-enforcement.md) (trust tiers, operator-local promotion), and the 9.4b re-ratification (TL-anchored trust root, Option A). References the NFR-Ops-12 air-gap deployment constraint and the NFR-Comp-3 compliance-scope declaration in `STABILITY.md`.

## Context

NFR-Ops-8 requires a published ADR by v0.3 declaring which competitive framing is committed — **substrate-as-substrate** or **substrate-as-trust-anchor** — with absence constituting a release-block. The decision was deferred from the innovation/scoping phase (Step 8 carry-forward) because both framings were architecturally consistent through v0.3; the lock is a positioning decision that determines which innovation is the spear tip.

Two reference classes are in play:

1. **OSS infrastructure substrate** (Linux / Postgres / Kubernetes / Apache HTTPD): the kernel is a trusted runtime under which third-party autonomous agents execute. The operator owns the deployment, the trust boundary, and the compliance mapping. The substrate provides mechanisms; it makes no claims about the agents running on it.

2. **Agent-ecosystem trust anchor** (Mozilla-CA / IETF / W3C): the substrate acts as a certifying authority for the agent ecosystem — issuing trust assertions, maintaining a curated registry, and vouching for agent behavior. The moat is institutional authority, not infrastructure quality.

Three prior architectural decisions already lean heavily toward framing (1):

- **ADR-005** (pluggable provider drivers): "providers/drivers are independent crates; new drivers ship without kernel changes" — the substrate mediates, it does not preference.
- **ADR-009** (three trust tiers): "centralized vetting fails the substrate-not-product framing; promotion is operator-local" — trust decisions are delegated to operators, not centralized in the substrate.
- **ADR-006** (the kernel learns no patterns): patterns live in user-space; the kernel mediates and audits but does not store or interpret content — the substrate is deliberately ignorant of what runs on it.

The 9.4b re-ratification (2026-06-15) established the **TL-anchored trust root** as the committed cryptographic model: region is bound into the HKDF key-derivation `info` of the Transparency Log signing key and the AEAD AAD of TL entries. Memory rows are plaintext-at-rest, region-bound by audit governance (not per-row sealed). The `hkdf` crate (RustCrypto) was landed. D1 (plaintext stores) was the ratified waiver. This model is **operator-local by construction**: the signing seed is the operator's secret, derived locally via HKDF with no network dependency.

## Decision

### 1. Committed framing: substrate-as-substrate

MAOS is an **open-source infrastructure substrate** in the Linux/Postgres/Kubernetes reference class. The competitive identity is:

- **Mechanisms, not assertions.** The substrate provides the Transparency Log, capability mediation, sandbox tiers (T0–T3), ComplianceClaim envelopes, and GDPR erasure cascade. It does not issue trust assertions about Spirits, operators, or deployments.
- **Operator-local trust decisions.** Trust-tier promotion (`local` / `org-internal` / `public-untrusted`) is operator-local (ADR-009). The substrate enforces the operator's declared policy; it does not maintain a curated registry of approved agents.
- **Audit trail, not audit opinion.** The Transparency Log is append-only, tamper-evident, and operator-owned. It records what happened; it does not assert what should have happened. ComplianceClaim envelopes carry the operator's (or a third-party assessor's) claims — the substrate is the filing cabinet, not the auditor.
- **Replaceable kernel.** The kernel is deliberately small (ADR-038, ≤20 KLOC ceiling) and the user's data is not co-located with kernel state (ADR-006). An operator can replace the kernel without losing accumulated knowledge — the substrate does not create lock-in through accumulated institutional authority.

### 2. Considered and rejected: substrate-as-trust-anchor

The trust-anchor framing (Mozilla-CA / IETF / W3C reference class) was evaluated as an alternative competitive identity. Under this framing, MAOS would act as a certifying authority: maintaining a curated Spirit registry, issuing behavioral trust assertions, and building institutional authority as the moat.

**Rejected because:**

- **Conflicts with ADR-005/ADR-009.** Pluggable providers and operator-local trust promotion are structurally incompatible with a centralized trust authority. The architecture would need to be unwound.
- **Creates a governance bottleneck.** A certifying-authority role requires a governance structure, liability framework, and dispute-resolution process that do not exist and are not on the v1.0 roadmap. The overhead would consume the solo-founder phase.
- **Misaligns the moat.** The substrate's moat is spec + trademark + ComplianceClaim (per the scoping-phase analysis), not institutional authority. An infrastructure substrate's adoption compounds with ecosystem size; a trust anchor's authority concentrates with governance quality. The former scales with the founder's code; the latter scales with an organization MAOS does not yet have.
- **The trust-anchor signal is routed, not lost.** The first-auditor-references-a-TL-frame validation signal (the strongest early indicator under trust-anchor framing) is fully available under substrate-as-substrate: auditors reference TL frames because the audit trail is tamper-evident, not because MAOS vouched for the agent. ComplianceClaim envelopes carry this signal without the substrate taking an opinion.

### 3. TL-anchored trust root — reconciliation with 9.4b

The 9.4b re-ratification (2026-06-15) established the cryptographic trust model that the substrate-as-substrate framing rests on:

- **Operator-local signing seed.** The Transparency Log signing key is derived from the operator's seed via HKDF-SHA256 (`hkdf` crate, RustCrypto). The seed is the operator's secret, stored locally (`~/.config/maos/operator.toml` or `MAOS_REGION_HOME` env). No network dependency.
- **Region-pinned derivation.** The canonical region tag (frozen `ascii-v1`, `^[a-z0-9-]{2,32}$`) is bound into the HKDF `info` parameter, producing a per-region signing key. Foreign-region data is cryptographically unusable on the TL verify path (`ERegionViolation`, fail-closed).
- **Plaintext-at-rest waiver (D1).** Memory rows are not per-row sealed. Region enforcement is by audit governance (every write routes through a `WriteEntryPoint` enum guard with no wildcard arm), not by per-row encryption. The threat model scopes to operator misconfig / cross-region bleed in a trusted kernel; raw-disk exfiltration is out of scope (deferred to 9.4c).
- **No online CA / OCSP dependency.** The anchoring and rotation mechanism is HKDF + local seed. NFR-Ops-12 (air-gapped deployment) forbids the network path a naive PKI assumes. Rotation is seed rotation with re-derivation, not certificate renewal.

This trust model is **substrate-as-substrate by construction**: the operator owns the seed, the operator owns the trust boundary, and the substrate provides the mechanism (HKDF derivation + TL signing + fail-closed enforcement) without making claims about what the operator does with it.

### 4. Air-gap-compatible anchoring and rotation

The anchoring mechanism satisfies NFR-Ops-12 (zero outbound network calls):

- **Key derivation:** HKDF-SHA256 with operator-local seed + region tag. No key server, no CA, no OCSP responder.
- **Rotation:** Operator rotates the seed locally; re-derivation produces new per-region signing keys. In-flight TL entries signed under the old key remain verifiable (the old seed is retained for verification, not signing). No certificate-revocation-list fetch.
- **Boot:** The substrate boots and produces TL entries with the operator's local seed. No network handshake, no license check, no external authority contact.

### 5. Explicit v1.0 scope

**Committed at v1.0:**

- Substrate-as-substrate competitive framing (this ADR).
- Operator-local TL-anchored trust root via HKDF-SHA256 (9.4b).
- Region-pinning with air-gap-compatible anchoring/rotation (9.4b, NFR-Ops-12).
- Transparency Log, capability mediation, sandbox tiers T0–T3, ComplianceClaim envelopes.
- GDPR Art. 17 erasure cascade with externally-verifiable Merkle proof (9.2).
- Model-provenance manifest field with governance journaling (9.4b AC-6).
- SOC 2 / ISO 27001 / FedRAMP compliance-scope declaration — operator's responsibility (NFR-Comp-3, `STABILITY.md`).
- 1-year LTS commitment (NFR-Maint-6).

**Deferred (explicitly not committed at v1.0):**

- Per-row at-rest encryption for memory stores (9.4c, parked in E10 — gated on a real at-rest/exfiltration trigger, not speculative).
- Multi-operator tenancy beyond the `deployment_operator_id` reservation (9.6 / E10 v1.5+).
- Online CA / PKI integration (not on roadmap — air-gap-first is the architectural commitment).
- Trust-anchor-as-certifying-authority capabilities (rejected framing — not deferred, architecturally excluded).
- 2-year LTS (deferred to v1.5).
- SIEM export (v2.0; OTel adapter at v1.0 SLO-class per 9.5b).

## Consequences

- **NFR-Ops-8 release-block is closed.** The v0.3 tag is unblocked on the framing axis.
- **Story 9.5 (doc site) references this ADR** as the authoritative framing decision. The information architecture depends on 9.5a merging first.
- **`STABILITY.md` NFR-Comp-3 scope language** references this ADR for the trust-boundary definition (updated by the `xtask stability-matrix` generator in the same commit).
- **No architectural change.** This ADR commits the direction the architecture already leans (ADR-005, ADR-006, ADR-009). It does not introduce new code, new abstractions, or new runtime behavior.
- **The trust-anchor competitive signal remains available** through ComplianceClaim envelopes and TL tamper-evidence — it is routed through the substrate-as-substrate framing, not abandoned.

## Gate

- This ADR is registered in `docs/adr/index.md` with status `binding-v0.3`.
- `STABILITY.md` Substrate-Self Compliance Scope references this ADR (generated by `xtask stability-matrix`, verified by `--check`).
- No runtime gate (this is a framing/positioning decision, not a code change).

## Ratification

Ratified by the 9.4b consensus authority (the same authority that ratified the TL-anchored trust root, region-pin Option A, and the D1 plaintext-stores waiver). Consistency verified: this ADR's §3 names the TL-anchored trust root as the committed model, does not contradict D1 (plaintext-at-rest), and states the air-gap-compatible mechanism per NFR-Ops-12. Recorded at Story 9.5a (2026-06-15).
