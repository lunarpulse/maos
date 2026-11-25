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

This ADR does **not** claim universal encryption-at-rest. Scope is opt-in adapter-store rows for loom-lite Collective writes plus out-of-kernel identity assertion writes. Generic kernel-authored Transparency Log payload encryption remains deferred because the live TL write path is `maos-kernel-core::TransparencyLogAdapter::insert_frame_event`; sealing it would require a kernel-core write-path seam and violate the zero-delta constraint. Kernel-core Private/Shared at-rest encryption and OS keyrings are deferred additive adapters behind the same port; a vendor KMS is reached through an operator-run sidecar (see the Story 23-8 amendment below).

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

## Amendment — Story 23-0 (online-adapters umbrella and outbound policy; 2026-11-24)

Decided by the operator on 2026-11-24 (D-23-6-02 A, D-23-6-01 A, D-23-0-02 A, D-23-1-04 A). Each point adds to the Decision; nothing above changes.

- **One umbrella.** `maos-bin` gains the feature `online-adapters = ["network"]`, and `default` becomes `["network", "online-adapters", "wasm-host"]`. `network` does not enable `online-adapters`, so `--no-default-features --features network,wasm-host` builds the profile without online adapters. CI builds and tests that profile (`check-composition-root-completeness`). Every online adapter of Epic 23 (MCP OAuth, OIDC discovery, OTLP, the TLS SIEM sink, the A2A HTTP listener) sits behind `online-adapters` from its first commit. The feature gates no code in Story 23-0.
- **One outbound policy.** `maos_domain::outbound` holds the shared rule. `IpClass` has `Public`, `Loopback`, `LinkLocal`, `Private` and `Reserved`. `OutboundPolicy` is `strict()` (public https only) with two knobs: `allow_private` (loopback and RFC 1918 / fc00::/7; never link-local, never reserved) and `allow_loopback_http` (loopback, and plain `http` to a literal loopback only). A host name is resolved through an injected `ResolveHost` and every answer must pass. `OutboundRefusal` carries one stable code per class (`outbound_http_refused`, `outbound_loopback_refused`, `outbound_link_local_refused`, `outbound_private_refused`, `outbound_reserved_refused`, `outbound_redirect_refused`, `outbound_unresolved`, `outbound_url_invalid`) and never any part of the input. `PolicedIo` wraps an `IoSubsystemPort`, vets every URL before it delegates, and refuses a 3xx answer; the IO port and the kernel adapter do not change.
- **Consumers.** 23-1 (MCP OAuth), 23-2 (OIDC discovery), 23-3 (OTLP), 23-4 (TLS SIEM sink, host class only through `vet_host_port`) and 23-8 (KMS endpoint, where ruled), plus Worker egress (ADR-061, D-23-0-03 A). Each already depends on `maos-domain`.
- **Dependency.** `url` 2.5 enters `maos-domain` so that the policy parses a URL with the parser `ureq` already uses in the kernel closure. It is not an OIDC, KMS or SIEM dependency, so the Consequence "OIDC/KMS/SIEM dependencies stay out of `maos-kernel-core` and `maos-domain`" holds.
- **Known boundary.** The rebinding window between the policy's lookup and the IO adapter's lookup is claim boundary 23-0.a in `RELEASE-HOLDS.md`.

## Amendment — Story 23-0b (outbound connection binding; 2026-11-25)

Decided by the operator on 2026-11-25 (D-23-0b-01 A, 02 a, 03 a, 04 a, 05 a, 06 a, 07 a). Each point adds to the Decision; nothing above changes. This amends lane rule 1 of Epic 23 and D-23-1-04 A's "the IO port stays as is; no kernel change" for this one seam.

- **Port change.** `IoSubsystemPort` (`maos-domain`) gains `bind(&VettedUrl) -> Result<Box<dyn IoSubsystemPort>, IoError>`, whose default fails closed with `outbound_unbound_refused` (`OutboundRefusal::Unbound`), and `never_follows_redirects() -> bool`, default `false`. `VettedUrl` gains the accessors `is_name`, `host_str`, `port` and `socket_addrs`.
- **Policy wrapper.** `PolicedIo::vet_url` resolves a host name once; a name goes through `inner.bind(&vetted)` and never down the raw-string path; a literal IP never calls `bind`. `PolicedIo::new` returns `Result` and refuses (`OutboundRefusal::Redirect`) an inner port that does not declare `never_follows_redirects`, and a bound port is held to the same declaration, because the 3xx check runs after the transport returned and a following transport would already have connected to an unvetted address.
- **Kernel change (`crates/maos-kernel-core/src/io/mod.rs`, +19 tokei).** One private agent builder is shared by `new()` and `bind()` (`.redirects(0)` in one place). `bind` returns an adapter on a fresh agent with `max_idle_connections(0)` (no pooled connection from another resolution) and a resolver that answers only the vetted `host:port` with the vetted addresses and refuses any other netloc. TLS keeps the URL host name for SNI and certificate verification.
- **Classifier.** `3fff::/20` (RFC 9637 documentation) is `Reserved`, so Worker egress refuses it too (more refused; `RELEASE-HOLDS.md` row 23-0.b).
- **Production resolver.** Not part of this story: the first consumer (23-2) builds `ResolveHost` at the composition root.
- **Boundary.** Claim boundary 23-0.a is closed for host names through `PolicedIo` over the kernel adapter only; row 23-0b.a states what remains open.

## Amendment — Story 23-8 (external KMS adapter; 2026-11-25)

Decided by the operator on 2026-11-24/25 (D-23-8-01…10 as recommended; D-23-8-11 proposed A, implemented as A). Each point adds to the Decision; nothing above changes. "Vault/cloud KMS … deferred" now reads "vendor KMS through a sidecar".

- **One generic adapter, no vendor API in the binary.** `maos_secrets::http_kms::HttpKms` implements `KeyManagementPort` over `maos.kms.v1`, a MAOS-defined JSON wire (`docs/reference/kms-wire-v1.md`): `POST /wrap`, `POST /unwrap`, `GET /health`, base64 fields, `"version":"maos.kms.v1"` on every message. The endpoint is a sidecar the operator runs in front of a KMS or HSM. `examples/kms-sidecar-openbao/` is a reference sidecar for OpenBao Transit (v2.7.3, MPL-2.0); it is not a release artifact, is not in the workspace and has not been security-reviewed. The envelope (`MAOSKMS1`, AADs) is unchanged: only who wraps the data key changes.
- **Selection and refusals.** `MAOS_KMS_ENDPOINT` selects the adapter. Both `MAOS_KMS_ENDPOINT` and `MAOS_KMS_MASTER_KEY` is a boot refusal (`kms_config_conflict`, exit 78). The local master key stays for dev/CI. A `MAOS_KMS_*` variable with neither source, an unusable master key (not 64 hex characters, or all zero) and an endpoint that fails the policy are boot refusals too (`kms_config_incomplete`, `kms_master_key_invalid`, `kms_endpoint_refused`), and a build without `online-adapters` refuses the endpoint (`kms_endpoint_unsupported_build`). Neither variable set: byte-identical Option-A plaintext as before.
- **Fix of the serve-path plaintext fail-open (D-23-8-07 A).** Before this amendment an unusable local master key with the Collective tier on latched the runtime `ConfiguredDown`, `at_rest_seal_hook()` returned `None`, and the serve `main()` kept the Option-A plaintext default: the Host booted printing `kms=true` and wrote plaintext `val_data`, against "KMS down → sealed write refused" above. Now a configured KMS always yields a hook: it seals and fails closed per write (`seal_at_rest` checks `is_healthy`), or, for a runtime with no usable adapter, always refuses. An invalid configuration refuses the boot; an endpoint that is merely unreachable does not, because its health recovers by probe. The cohort daemon's refusal to start with an unbuilt KMS stays.
- **Health (D-23-8-03 A, D-23-8-10 A).** `is_healthy()` does no I/O: it is true only if the last `GET /health` probe succeeded within three probe intervals (default 10 s) on an injected clock and no availability failure happened since; a probe that started before a failing call cannot clear that failure. A transport error, timeout, 5xx, any other non-answer, and a 2xx with a malformed or wrong-version body flip it down at once; a typed `422` (`wrap_failed`, `unwrap_failed`) is a refusal of one payload, returns `KmsError::Crypto` and leaves it alone, so one planted foreign wrapped key cannot stop sealed writes. `maos` probes once at boot and every 10 s on a blocking task stopped by the shutdown token; a running probe cannot be cancelled and has its own 2 s limit.
- **Transport (D-23-8-01 A, D-23-8-06 A, D-23-8-11 A).** `https`, or `http` only to a literal loopback address; `maos_domain::outbound` vets the operator-set endpoint once at boot (link-local and reserved never; private **and loopback** addresses only with the explicit `MAOS_KMS_ALLOW_PRIVATE`, which admits them by any route, including a name that resolves to them); no redirects; for a literal-IP endpoint connect, read and write 2 s, overall 5 s for wrap and unwrap and 2 s for the health probe; responses capped at 64 KiB. The timed transport (`TimedHttp`) lives in `maos-secrets` over its own `ureq` agent, so the kernel IO adapter is untouched. **No host-name SSRF claim:** for a host name the policy does not bind the connection to the vetted addresses (RELEASE-HOLDS 23-0.a), the endpoint is operator configuration and never read from a peer, and `https` plus a literal address is the recommended deployment. **Three limits of this transport:** loopback `http` does not authenticate the listener (a local user can claim the port while the sidecar is down; deploy on an unclaimable port, under the same OS user with the port bound first, or behind TLS; D-23-8-01 B, a Unix-domain-socket owner check, was not chosen); `https` trusts the bundled webpki roots only, so a private-CA endpoint is not supported (a CA-bundle setting is a follow-up decision); and ureq applies none of the timeouts to the DNS lookup, so they hold for a literal IP only (fail-closed still holds: health goes stale after 30 s).
- **Bearer and secret store (D-23-8-08 A).** An optional bearer travels as `Authorization: Bearer` on wrap and unwrap only, to the configured endpoint, never in an error, log line, journal row or `Debug` output. It is the new closed `SecretKey::KmsBearer` (`MAOS_KMS_BEARER`; in `SecretKey::ALL` and in `maos purge`). The KMS adapter is built once, before the secret store and the enterprise runtime, and the same instance goes to both; `MAOS_SECRETS_BACKEND=encrypted-file` works over whichever KMS is configured, which gives the endpoint a production unwrap reader. The bearer is read through the configured store, except under `encrypted-file` (the encrypted store cannot unlock itself), where it is read through the environment. Until the bearer is settled the adapter refuses every call; a bearer that cannot be read leaves it refusing for the whole process, and the boot and the probe lines say so instead of claiming that writes resume. A failed wrap or unwrap is logged once per change of status class (401, 403, 5xx, transport), never with a value.
- **Placement (D-23-8-09 A).** The adapter is behind `maos-secrets` feature `http-kms`, which only `maos-bin`'s `online-adapters` enables: the sixth adapter that dials out. The air-gap profile (no `online-adapters`) has the local key only.
- **Limits.** Sealed Collective rows are write-only until a reader exists (`seal.rs:118`, `store.rs:1683-1684`); the Transparency Log is not sealed, and **identity assertions in it are not sealed by this story** (`append_identity_asserted` writes plain JSON; `EnterpriseRuntime::seal_row_at_rest` has no production caller; the scope sentence above that mentions identity assertion writes is design intent, not shipped behaviour); no key rotation or re-wrap; the example sidecar is unreviewed. See `RELEASE-HOLDS.md`, Epic 23 rows 23-8.a…g.
