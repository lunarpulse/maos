---
Status: binding-v2.0 (architecture + mechanism ratified 2026-07-06 party-mode; binding at Story 11.4c gate green)
Gate: Story 11.4c — `check-enterprise-identity` (7 per-leg-independent legs: oidc-verify, principal-provenance, at-rest-seal, siem-redaction-export, additive-and-failclosed, release-graph-absence, kernel-abi-diff); `{ v1_0 = advisory, v1_5 = advisory, v2_0 = blocking }`; absent/unmeasured → BLOCK@v2.0
Decided: 2026-07-06 (party-mode preflight)
Accepted-in-PR: Story-11.4c
Supersedes: none (extends ADR-050 enterprise PDP and ADR-024 sandbox/TL telemetry)
Revisits: ADR-006 (kernel learns nothing); ADR-010 (hexagonal port/adapter ring); ADR-030 (capability hot-path budget); ADR-050 (enterprise PDP principal attributes)
---

# ADR-051 — Enterprise identity, opt-in at-rest AEAD, and SIEM projection as out-of-kernel leaves

**Decision.** Enterprise identity assertion, adapter-store at-rest encryption, and SIEM export are additive out-of-kernel leaves behind three domain ports: `IdentityAssertionPort`, `KeyManagementPort`, and `SiemProjectionPort`. The reference leaves are `maos-sso` (OIDC static-JWKS verifier), `maos-secrets` (dev/CI local-master-key KMS + envelope seal helpers), and `maos-siem` (read-only TL projection through `query_with_redaction`). Kernel-core remains unchanged at the Story 11.4b baseline (23081); no CapabilityToken/issuance-record identity field is added.

## Context

Story 11.4c closes the enterprise-deployment slice left intentionally shaped by ADR-050: the PDP request already has `principal_attributes`, but identity was not yet asserted; adapter-store at-rest encryption was not yet opt-in; and NFR-Aud-11 still needed a SIEM projection path. The slice is security-adjacent and has three canned-green traps: fake JWT acceptance, key-ignoring encryption, and redaction-bypass SIEM forwarding.

## Decision

### 1. Identity assertion port and `maos-sso`

`IdentityAssertionPort` lives in `maos-domain` and returns an `AuthenticatedPrincipal` only after the assertion passes signature, issuer, audience, `exp`/`nbf`, and algorithm-allowlist checks. The reference `maos-sso` crate verifies static JWKS material offline for CI and rejects `alg:none` plus HS256-with-RS256-public-key confusion. It exposes `govern_authorization` to project verified principals into PDP `principal_attributes` and to emit an out-of-kernel `identity.asserted` provenance record.

The provenance is authorization-layer evidence (`subject`, `issuer`, `spirit_pid`, decision context), not a kernel token field. Synthetic records are not attested and do not increment `reconcile_provenance`, preventing blind-source canned greens.

### 2. At-rest encryption port and `maos-secrets`

`KeyManagementPort` wraps/unwraps envelope data keys outside the kernel. The Story 11.4c reference KMS is a local 32-byte master-key adapter intended only for dev/CI tripwires. It proves the envelope contract: ciphertext differs from plaintext, the right key opens, the wrong key fails, and the unconfigured default remains byte-identical Option-A plaintext.

This ADR does **not** claim universal encryption-at-rest. Scope is opt-in adapter-store rows for loom-lite Collective writes plus out-of-kernel identity assertion writes. Generic kernel-authored Transparency Log payload encryption remains deferred because the live TL write path is `maos-kernel-core::TransparencyLogAdapter::insert_frame_event`; sealing it would require a kernel-core write-path seam and violate the zero-delta constraint. Kernel-core Private/Shared at-rest encryption, Vault/cloud KMS, and OS keyrings are deferred additive adapters behind the same port.

### 3. SIEM projection port and `maos-siem`

`SiemProjectionPort` is the domain seam. `maos-siem` reads the Transparency Log read-only and applies `query_with_redaction` before projection. It produces NDJSON and CEF content framed for RFC5424 syslog transport. Count reporting distinguishes measured non-empty exports (`Some(n)`) from empty TL N/A (`None`) so a silent no-op cannot green as `Some(0)`.

The reference exporter is local/in-process. HTTPS/network sinks must be TLS-only; plaintext TCP/file sinks are localhost-only. Production sinks are additive adapters.

### 4. Composition root fail-closed semantics

`maos-bin::enterprise_identity` holds the three ports as `Option<Arc<dyn Port>>` slots and is environment gated by `MAOS_SSO_*`, `MAOS_KMS_*`, and `MAOS_SIEM_*`. Zero config is byte-identical to v1.5 defaults: no SSO gate, plaintext Option-A storage, no SIEM forwarding. Configured-but-down subsystems fail closed independently:

- SSO down → capability issuance denied; never falls open to `spirit_pid`.
- KMS down → sealed write refused; never silently writes plaintext under encryption posture.
- SIEM sink down → records buffered and an operator-visible error surfaces; never silently drops.

### 5. Release fault-inject guards

`sso-fault-inject`, `kms-fault-inject`, and `siem-fault-inject` are dev/CI-only falsifiers. A release build with any one enabled must fail with `compile_error!`. The gate treats the compile-error as the positive ship-blocker signal.

## Alternatives considered and rejected

- **Identity inside `CapabilityToken` / kernel issuance records.** Rejected: assertion time precedes token issuance; adding token fields mutates the hot path and kernel ABI. Provenance belongs out-of-kernel at the authorization layer.
- **Accept-all SSO stub as reference.** Rejected: it greens without signature/claim verification. The reference must verify real JWT signatures and include algorithm-confusion negatives.
- **Key-ignoring or plaintext-fallback KMS.** Rejected: wrong-key success is the primary at-rest defeat. Configured KMS down must refuse writes.
- **Direct TL `query()` SIEM export.** Rejected: it can preserve scrubbed payloads while dropping `redaction` provenance. Export must call `query_with_redaction`.
- **Production Vault/cloud-KMS/SAML in the same story.** Rejected as over-scope. The ports are additive; one real OIDC verifier and one dev/CI KMS prove the contract.

## Consequences

- New domain ports: `identity_assertion`, `key_management`, `siem_projection`.
- New leaf crates: `maos-sso`, `maos-siem`; existing `maos-secrets` placeholder becomes the reference KMS/envelope helper.
- New out-of-kernel audit kind: `identity.asserted` (discriminator 30).
- New `maos-bin` library surface for enterprise identity composition-root tests.
- New gate: `check-enterprise-identity`, advisory at v1.0/v1.5, blocking at v2.0.
- Dependency closure remains enforced: OIDC/KMS/SIEM dependencies stay out of `maos-kernel-core` and `maos-domain`.

## Gate

The `check-enterprise-identity` gate has seven independently falsifiable legs: `oidc-verify`, `principal-provenance`, `at-rest-seal`, `siem-redaction-export`, `additive-and-failclosed`, `release-graph-absence`, and `kernel-abi-diff`. The gate reads `xtask/gate-registry.toml` and requires `{ v1_0 = advisory, v1_5 = advisory, v2_0 = blocking }`. A vacuous attempted leg is a hard failure at every phase.

## Ratification

Ratified by the Story 11.4c party-mode preflight (Winston · Murat · John · Amelia · Mary + Vex/security & Grumbal/adversary + Paige, 2026-07-06). The key decisions were: AC split into OIDC verification plus principal-governed authorization; provenance is `identity.asserted` out-of-kernel; at-rest AEAD is opt-in and scoped; SIEM is NDJSON+CEF with RFC5424 transport; fail-closed is per subsystem; fault features are release-blocked.
