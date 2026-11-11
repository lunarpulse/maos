---
Status: accepted, binding-v2.2
Gate: Story 13.4 — `check-vetting-attestation`, blocking at v2.2
Decided: 2026-07-23
Accepted-in-PR: Story 13.4
Extends: §15.4 (FR37 vetting machinery)
Reuses: ADR-045 (operator audit root and governance-artifact discipline), ADR-036 planning seam
---

# ADR-056 — FR37 vetting machinery

## Context

FR37 requires an end-to-end internal-vetter flow for attestation issuance, verification, journaling, and revocation. `public-vetted` must not be a mutable registry flag: a flag loses the evidence of exactly what was vetted, by whom, for how long, and under which revocation semantics. The design must also preserve the pinned `maos-kernel-core` baseline: 23228 == pin.

Two similarly named trust-tier types have different authority and must not be conflated. Axis A is the out-of-kernel `maos_spirit_abi::compliance::TrustTier { Local, OrgInternal, PublicVetted, PublicUntrusted }`; its `PublicVetted` variant already exists and was deferred, not absent. Axis B is the kernel runtime sandbox-floor `maos-kernel-core::capability::cap_policy::decision::TrustTier { PublicUntrusted, Known, Verified, Internal }`; it deliberately has no public-vetted variant and hardcodes `Verified`. There is no bridge between the axes and `maos-kernel-core` has no `maos-registry` dependency.

## Decision

### 1. Public-vetted is an out-of-kernel signed artifact

`public-vetted` lives entirely on Axis A. Adding `PublicVetted` to the Axis-B kernel enum is forbidden. This preserves zero kernel-core delta and keeps registry policy out of the runtime sandbox floor.

The `maos-compliance::vetting` module defines `VettingAttestation`, an Ed25519-signed
envelope shaped like `ComplianceClaimEnvelope`: `signature[64]`, `vetter_pubkey[32]`,
canonical-CBOR `claim_bytes`, and `signing_alg`. Its inner `VettingClaim` binds
`sha256(manifest_toml)` over the exact bytes, `artifact_sha256`, the publisher public
key, Spirit name/version, the tier transition, vetter key id, issuance/expiry,
revocation semantics and optional successor policy. Verification receives these
package inputs independently through `VettingSubject`; it never derives the expected
artifact or publisher from the claim it is checking. Old manifest-only attestations
must be reissued; missing binding fields fail decoding rather than receiving defaults.

### 2. Admission promotion is attestation-conditional

The abstract ranking returned by `score()` and the existing `strictest_of` rule are unchanged. Promotion gates above `strictest_of`: `admit_spirit_with_attestation(pkg, op_cfg, attestation, keyring, now)` in `crates/maos-registry/src/admission.rs` wraps the byte-stable `admit_spirit`. A current valid attestation un-defers public-vetted at admission; an absent attestation keeps the existing `PublicVettedDeferred` outcome.

This is not a registry mutation. The attestation is the evidence that authorizes the Axis-A promotion, and ordinary tier and sandbox-floor evaluation continue unchanged below it.

### 3. Verification follows the operator-root chain

`verify_attestation` verifies the attestation signature; exact manifest and artifact
hashes; publisher key; promotion tiers and validity; target identity; operator-root
enrollment predating issuance; and revocation. An un-enrolled or revoked key refuses.

Vetter-key enrollment, rotation, and revocation are `VetterKeyEvent` artifacts with their own operator-root signatures under the §7.3 audit root. They are distinct from the unsigned `maos_domain::governance::VetterKeyPayload`; that name collision is not an authority shortcut. Issuance, verification, revocation, and these lifecycle events are journaled.

### 4. Lapse, revocation, and withdrawal remain distinguishable

The audit surface distinguishes `VettingTerminalCause` values `VettingRevocation`, `ExpiryLapse`, `RegistryYank`, and `OperatorLocal`; their declaration order is the deterministic precedence order when more than one condition applies. At v2.2, `RevocationSemantics::RefuseAtNextLoad` is the only disposition: no running Spirit is demoted or drained by this decision. `DrainAndRefuse` is reserved as the v2.5 slot.

When the compliance layer detects expiry or vetting revocation for a running Spirit, it must journal a `RunningSpiritObservation`. This observation exposes the lapsed running state instead of laundering it as continued vetted operation.

### 5. Upgrades require a current successor attestation

Exact-hash binding means an upgraded manifest is a different artifact. A new version without its own current attestation is refused at the admission floor; the upgrade flap is intentional. `SuccessorPolicy` makes the required re-vetting cadence explicit.

The check attaches to the existing `maosctl spirit upgrade --plan` and `HotSwapPrecheck` precondition seam before the chain starts. It does not create a `maosctl swap` command, and ADR-036 remains planning-only rather than a new documentation surface.

### 6. The gate proves independent failure modes

`check-vetting-attestation` is the binding v2.2 discipline gate with seven hermetic blocking legs:

1. issue → install → promote → revoke round trip, with a verifier independently derived from the issue codec;
2. forged-attestation-signature negative;
3. expired-attestation negative;
4. forged-vetter-key negative for a valid signature from an un-enrolled key;
5. upgrade-flap positive and negative cases;
6. inversion of `e2e_public_vetted_always_rejected`; and
7. four-cause audit distinguishability.

Each negative control must red on its own defect; the gate cannot accept a shared or merely structural substitute.

## Consequences and limits

- FR37's four-verb contract is issuance, verification, journaling, and revocation with internal vetter keys. The richer AC4 and AC5 controls are architecture-derived commitments from §15.4 and this ADR.
- The kernel baseline remains 23228 == pin. `maos-kernel-core` receives neither a registry dependency nor an Axis-B `PublicVetted` arm.
- Attestation absence, expiry, unenrolled issuer, manifest drift, and revocation fail closed before public-vetted admission; the unchanged `strictest_of` calculation remains the sandbox-floor rule.
- The v2.2 running-state outcome is journal-and-refuse-at-next-load, not drain. v2.5 may add `DrainAndRefuse` using existing drain machinery.
- Accredited external vetters under NFR-Comp-2 are v2.5 and out of scope. The deferred partner-organization federation tier is a separate future concern.

## Rejected alternatives

- **Mutable `public-vetted` registry flag:** rejected; it cannot bind the particular manifest, issuer, lifecycle, and expiry evidence that promotion requires.
- **Add `PublicVetted` to the kernel trust enum:** rejected; it conflates the independent trust axes, breaches the kernel boundary, and violates the 23228 pin.
- **Put attestation logic inside `strictest_of`:** rejected; it changes abstract tier ranking instead of making promotion conditional on evidence.
- **Trust any correctly signed vetter attestation:** rejected; signature validity without an operator-root enrollment predating issuance is not authorization.
- **Family or version-range manifest binding:** rejected; it would permit unvetted successor code. Exact-hash binding and the upgrade flap are required by ADV-056-1.
- **Immediate running-Spirit drain at v2.2:** rejected; no such zero-kernel-delta enforcement exists. The explicit v2.2 posture is `RefuseAtNextLoad` plus observation journaling, with the v2.5 drain slot reserved.
- **External accredited vetters now:** rejected; NFR-Comp-2 belongs to v2.5.

ADV-056-1 governs exact-hash upgrade refusal, ADV-056-2 governs lapse/revocation disposition and four-cause audit visibility, and ADV-056-3 governs the signed operator-root vetter-key lifecycle.

## Amendment — facts recorded by the D8 review (2026-09-29)

The FKCS conformance review that discharged decision D8 (`epic-14-preflight-decisions.md`, §D8 ruling) read Story
13.4's change to `crates/maos-registry/src/admission.rs` in full and records two facts this ADR states otherwise.

- **`admit_spirit` was not byte-stable.** §2 says the attested entry point "wraps the byte-stable `admit_spirit`".
  13.4's review remediation replaced its lenient line-based tier read with the structural, fail-closed
  `maos_manifest::parse_manifest_trust_tier` (hyphenated tiers now parse; unknown, non-string and invalid TOML
  refuse), extracted `verify_public_untrusted_baseline`, and inlined equivalent branches. None of it can escalate a
  package — the manifest tier is author-controlled and origin/floor are unchanged — and every path still refuses
  `[fkcs]` off-surface declarations first.
- **Historical manifest-only gap, superseded by Story 20-1b L1 (2026-11-06).**
  The original claim bound neither artifact bytes nor publisher key. The new
  required bindings and golden CBOR pin close that shape before the author-facing
  issuer is enabled. Independent artifact and publisher swap controls live in
  `crates/maos-registry/tests/vetting_attestation_gate.rs`
  (`artifact_swap_is_refused_ac6`, `publisher_swap_is_refused_ac6`).
  The issuer and operator-root-signed attestation-revocation-list writer live in
  `maos-spirit-cli`; `vet issue`/`vet revoke` do not enroll a vetter implicitly.
  Running expiry/revocation uses the yank cadence and preserves cause labels.
  Quarantine requests in L1 had no effect; 20-1b landing 2 (final kernel landing, KQ1) adds a
  capability-mediated fence: the `quarantine` policy fences each signed-yank-matched running
  Spirit (issue, verify and recheck: MCP, inference, memory-by-token, IAC-by-token) and journals one
  `spirit.quarantined` receipt. Private and shared memory writes, scalar mutation, DRR/native IAC and
  IO, hooks and telemetry are not fenced; the fence is per-pid and in memory and is shed by restart,
  unload, swap and reload; a consumed yank row is never re-applied (the CRL path is durable). It is
  not isolation (operator ruling `q`, 2026-11-12).

## Amendment — 20-1b L1 independent-review corrections (2026-11-07)

The operator door resolves the named target's installed `SignedPackage` before
creating temporary manifest snapshots. It carries that explicit identity through
both upgrade guards; neither `/tmp` neighbors nor a genuine sidecar hiding altered
deployed bytes supplies identity. One verifier checks the actual manifest, artifact
and declared component against that package.

Attestation and operator key-event signature preimages are now v2 role-separated:
`SHA-256(domain || NUL || wire_length_u64_le || wire_claim)`, signed with Ed25519.
The domains are `maos.vetting.attestation.v2` and `maos.vetting.key-event.v2`.
Canonical claim CBOR stays byte-pinned; old unscoped signatures require reissuance.
Role/framing vectors and raw-digest refusal controls live in
`maos-registry/tests/yank_auth_20_1b.rs`. CRL/attestation-revocation role migration
still depends on the existing kernel signature contract and is not changed here.
The shipped `vet issue`/`vet revoke` verbs operate on attestations, not vetter-key
enrollment or key revocation: those lifecycle witnesses use fixture-built keyrings,
and the missing operator verbs remain an explicit 20-1b deferred obligation.

Signed yank mutation credentials v2 additionally bind registry audience, random
nonce and bounded lifetime. Accepted historical withdrawals do not expire merely
because their mutation credential has aged. Schema-2 daemon cursors bind the URI,
persist each verified frontier, reject unsigned advancement and resync on sequence
anomalies. These controls do not detect an omitted/rolled-back registry head:
authenticated first contact/TLS and head integrity remain AC15 work.

Local running-promotion reevaluation remains active without a registry source
and discovers installs appearing after boot; zero cadence is refused, maximum
cadence is 300 seconds. Network work cannot hold boot/shutdown or the audit drain
open. One bad Spirit or rejected withdrawal does not starve another observation,
and retrying an effect does not repeat its terminal observation.
The correction landing has zero kernel delta (26,841 physical / 21,042 tokei);
quarantine was still a typed request only at that landing; its capability-mediated fence (not isolation) arrived at 20-1b landing 2 (kernel 26,967 physical / 21,141 tokei).

## Amendment — AC15 authenticated first contact and yank heads

Under D-20-1b-01/02/03 A/A/A, the shared registry leaf requires WebPKI TLS and
an operator-configured Ed25519 signing key bound to the canonical `/mcp` URI.
The server's operator-provided certificate/key and signing seed are separate
credentials. A certificate or response-provided public key cannot bootstrap
registry authority. Explicit literal-loopback development is the only HTTP mode;
without a configured root its confidentiality/peer/omission guarantees stay open.

The v1 response envelope signs
`SHA-256("maos.registry.response.v1" || NUL || claim_CBOR)` with Ed25519.
The fixed claim field order is version, URI,256-bit nonce, exact request SHA-256,
exact response SHA-256 and optional yank-head sequence. Root verification is
outside the kernel. The full yank head and filtered entries share a durable
snapshot; a client requires all sequences through the authenticated head.
Nonce/request binding refuses replay; body binding refuses stripping or rewriting.
This is not protection against a trusted signing authority lying or signing a
rollback to a fresh client. No KQ1 enforcement or kernel signature-format change
is introduced. Hosted/macOS recipe legs remain unverified until execution.

