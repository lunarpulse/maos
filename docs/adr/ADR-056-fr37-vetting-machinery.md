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

The new `maos-compliance::vetting` module defines `VettingAttestation`, an Ed25519-signed envelope shaped like `ComplianceClaimEnvelope`: `signature[64]`, `vetter_pubkey[32]`, canonical-CBOR `claim_bytes`, and `signing_alg`. Its inner `VettingClaim` binds `sha256(manifest_toml)` over the exact manifest bytes, not a canonicalized manifest form; `from_tier`; `to_tier = public-vetted`; `vetter_key_id`; `issued_at`; `expires_at`; `RevocationSemantics`; and optional `SuccessorPolicy` (`ExactOnly` or `ReissueRequiredWithExpeditedReview`).

### 2. Admission promotion is attestation-conditional

The abstract ranking returned by `score()` and the existing `strictest_of` rule are unchanged. Promotion gates above `strictest_of`: `admit_spirit_with_attestation(pkg, op_cfg, attestation, keyring, now)` in `crates/maos-registry/src/admission.rs` wraps the byte-stable `admit_spirit`. A current valid attestation un-defers public-vetted at admission; an absent attestation keeps the existing `PublicVettedDeferred` outcome.

This is not a registry mutation. The attestation is the evidence that authorizes the Axis-A promotion, and ordinary tier and sandbox-floor evaluation continue unchanged below it.

### 3. Verification follows the operator-root chain

`verify_attestation` verifies, in order: the attestation signature; the exact manifest hash; the public-vetted target tier; expiry; vetter-key enrollment predating issuance; and revocation. A structurally valid attestation signed by an un-enrolled key is refused.

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
