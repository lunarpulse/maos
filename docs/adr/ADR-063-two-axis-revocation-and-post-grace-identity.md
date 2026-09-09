---
Status: ACCEPTED — ratified 2026-09-08 under Story 15-5, decisions F9 and F10
Gate: `cargo test -p xtask --test decision_adrs_and_provisioning`; implementation gate owned by Story 21-3
Decided: 2026-09-08
Accepted-in-PR: pending — Story 15-5
Revisits: RELEASE-HOLDS clause (c.6); architecture §7.2.1.a; Story 14-2a post-grace refusal
Supersedes: RELEASE-HOLDS clause (c.6)'s conclusion that no typed post-grace reason may be emitted; its non-distinguishability finding remains binding and defines the new reason's semantics
---

# ADR-063 — Two-axis revocation and post-grace identity

## Context

MAOS already ships two revocation mechanisms on different identity axes.

Spirit classes use the signed CRL model. `SignedRevocationList` and its entries
live in `crates/maos-domain/src/revocation.rs`; the kernel applies them through
the revocation applier and poller. The pinned behavior revokes capability
tokens, emits the Spirit revocation and halt evidence, and applies the
manifest's termination/drain/quarantine action.

Peer certificates use reissue-is-revocation. A signed cohort manifest reissue
opens a one-generation overlap window; closing it promotes the next
fingerprint, after which the old leaf fails the live handshake. There is no
X.509 CRL or OCSP consumer in the transport. These mechanisms are
complementary, not alternatives from which the project must choose one.

The current post-grace path is only partly typed. At
`crates/maos-a2a-tcp/src/transport.rs:1032-1052`, a TOFU mismatch produces a
`cert_post_grace_reject candidate` diagnostic and calls
`journal_peer_identity_refusal`. That journal seam accepts `detail: &str`, but
`crates/maos-a2a-core/src/router.rs:530-560` persists every such row as
`RuptureReason::PeerIdentityUnverified`; the diagnostic detail reaches tracing
only.

`RuptureReason` — `#[non_exhaustive]` at `crates/maos-domain/src/frame.rs:383`, the enum at `:384` — has six variants and no
`CertPostGraceReject`. The enum is `#[non_exhaustive]`, so adding one is an
additive domain change, but downstream exhaustive conversions must still be
updated.

RELEASE-HOLDS clause (c.6) correctly records that the leaf alone cannot prove
“retired generation after grace.” `TofuPin` at
`crates/maos-a2a-core/src/tofu.rs:18-31` contains no retired-generation
history, `close_rotation_window` returns a fingerprint no production caller consumes (production rotation calls `close_rotation_window_if`, `crates/maos-cohort/src/rotation.rs:1055`),
and the TLS failure surfaces no observed fingerprint at the classifier. An
unknown leaf and the retired leaf are therefore indistinguishable at this seam.
The available evidence is a join: a `cert_rotation_window_closed` row for a
peer followed by its refusal.

The listen-side verifier is also under-scoped. `build_server_config` at
`crates/maos-a2a-tcp/src/transport.rs:1322-1342` constructs the verifier with
`None, // listen side learns the peer from the cert`, so it accepts a
fingerprint found under any active pin. The dial side can pass an expected
`PeerId`; the listen side has no equivalent identity key before verification.

## Decision

Revocation remains a two-axis model:

- Spirit-class revocation uses the existing signed CRL pipeline.
- Peer-certificate revocation uses signed manifest reissue plus the bounded
  overlap window. No X.509 CRL or OCSP protocol is introduced.

Story `21-3-durable-tofu-and-self-leaf-rotation` adds
`RuptureReason::CertPostGraceReject`. The variant does not assert that the
presented leaf was cryptographically recognized as the retired generation. It
means the emitter observed a peer-scoped fingerprint mismatch in a
post-rotation context established by the durable join: the peer has a prior
`cert_rotation_window_closed` transition and this refusal follows that
transition. If the emitter cannot establish that join for the claimed peer, it
must emit the existing `PeerIdentityUnverified` reason.

The stable tracing token `cert_post_grace_reject` remains unchanged for log
continuity. The durable `ConsentRupture` row gains the typed variant. The
rotation close row plus rupture row are the authoritative evidence; neither row
alone claims leaf-generation identity.

This overturns the no-variant conclusion in RELEASE-HOLDS (c.6), not its
premise. The premise becomes the semantic boundary of the variant. No retired
fingerprint is added to `TofuPin`, and Story 21-3 must not implement the variant
by comparing against nonexistent retired-generation state.

### Listen-side peer scoping

The client certificate carries the operator-declared `PeerId` as a SAN identity
claim. The listener parses that claim before pin lookup and scopes verification
to the claimed peer's current fingerprint or open next generation. The
signature and fingerprint verification remain authoritative: a forged SAN or a
SAN paired with another peer's key is refused.

The SAN selects which pin record to verify; it does not replace TOFU, grant
membership, or authenticate itself. This follows the existing identity-weld
rule: derive the verification target from the claimed identity, then let
cryptography accept or reject it.

A post-handshake identity-binding frame is rejected. It would complete the TLS
handshake before resolving the trust decision, add a new frame and round trip,
and leave the existing flat verifier active at the critical boundary.

Story 21-3 therefore owns the SAN parser, server-verifier scoping, durable pin
and rotation-transition lookup, the new reason mapping, and the resulting
`maos-a2a-tcp` growth of approximately 30–80 lines already priced by Epic 21.
Kernel-core delta remains zero.

### Migration vehicle and budget

SAN-in-the-client-leaf is a fleet migration, not thirty lines: every peer's
leaf must be reissued carrying its declared `PeerId`. The reissue rides Story
21-3 AC2's own rotation path — `swap_serving_cert` gains its first production
caller, driven by the signed manifest reissue — so the story that needs the
migration builds its vehicle. The binding budget is
`G = cold_deployment_t_grace() + confirmation_interval() ≈ 65 s`, recorded as
**ratified but unbuilt** (`14-2d-self-identity-rotation.md:141-142`) and owned
by 21-3. Assigning the durable join and its typed reason to 21-3 also closes
14-2b's dangling post-grace ownership note
(`14-2a-production-mtls-rotation-trigger.md:873`).

## Consumers

- `21-3-durable-tofu-and-self-leaf-rotation` implements the typed reason,
  durable join, SAN-scoped listener, and its negative controls.
- `14-2a-production-mtls-rotation-trigger` remains the shipped source of the
  rotation-close and refusal behavior that Story 21-3 strengthens.

## Rationale

Calling Spirit CRLs and certificate reissue competing global models creates a
false choice. They revoke different objects and already coexist end to end.

A typed reason is useful only if its claim is honest. Defining it by observable
rotation history rather than by an unobserved retired leaf preserves the
(c.6) security finding while making the audit class machine-readable.

SAN scoping removes the listener's “any active pin” ambiguity at the handshake
boundary. Deferring identity to an application frame would authenticate too
late.

## Consequences

- Operators continue to use signed CRLs for Spirit classes and manifest reissue
  for peer certificates.
- `CertPostGraceReject` means post-rotation-context mismatch, not proof of the
  old leaf's bytes.
- Unjoined or unscoped certificate failures remain
  `PeerIdentityUnverified`.
- The durable audit preserves the existing log token and adds a typed reason.
- Client leaf issuance must include the declared `PeerId` SAN, and deployment
  validation must reject missing, ambiguous, or mismatched claims.
- Story 21-3 grows `maos-a2a-tcp` for listen-side SAN parsing and scoping rather
  than adding a post-handshake protocol.
