---
Status: ACCEPTED — ratified 2026-09-08 under Story 15-5, decisions F9 and F10; AMENDED 2026-11-25 under Story 23-7 (see Amendment)
Gate: `cargo test -p xtask --test decision_adrs_and_provisioning`; implementation gate owned by Story 23-7 (moved from Story 21-3)
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
`G = cold_deployment_t_grace() + 2 × confirmation_interval() ≈ 125 s` (amended by Story 23-7), recorded as
**ratified but unbuilt** (`14-2d-self-identity-rotation.md:141-142`) and owned
by 21-3. Assigning the durable join and its typed reason to 21-3 also closes
14-2b's dangling post-grace ownership note
(`14-2a-production-mtls-rotation-trigger.md:873`).

## Consumers

- `23-7-peer-identity-rotation` implements the typed reason, the durable join, the
  SAN-scoped listener, the own-leaf trigger and their negative controls (the
  implementation owner moved from `21-3-durable-tofu-and-self-leaf-rotation`, which landed the
  durable pin file this story's schema 2 extends).
- `14-2a-production-mtls-rotation-trigger` remains the shipped source of the
  rotation-close and refusal behavior that Story 23-7 strengthens.

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

## Amendment — Story 23-7

Ruled 2026-11-24/25 (D-23-7-01 A, D-23-7-02 C, D-23-7-03…11 A). This amendment records what
Story 23-7 built; the decision above is unchanged.

- **PKI profile.** Peer certificates come from the enterprise's own PKI; MAOS does not issue
  them. A leaf is both served and presented, so it needs both `serverAuth` and `clientAuth`
  where an extended key usage is present (the verifier asks webpki for `server_auth` on the
  dial side and `client_auth` on the listen side). Under `peer_san = "required"` it carries
  exactly one URI SAN, a SPIFFE X509-SVID `spiffe://<peer_trust_domain>/maos/peer/<peer_id>`
  (`peer_id` in `[a-zA-Z0-9._-]`); a second URI SAN, a foreign trust domain or a missing SAN
  is refused (X509-SVID §2: validators MUST reject more than one URI SAN). The SAN is read
  with `x509-cert`.
- **`peer_san` modes.** `[tcp] peer_san = "off"` (default) keeps the flat listen side: any
  active pin owns a client leaf. `"required"` scopes the listen-side check to the SAN-named
  peer's current fingerprint or open next generation; the SAN selects the record and the
  fingerprint decides. Boot, and every own-leaf swap, refuse an own leaf without its SAN under
  `required`. Until a deployment reissues its leaves with the SAN and sets `required`, the
  listen side stays unscoped and row 12 clause (iii) stays open for it.
- **`G` is built.** A window opened by a reissue closes after
  `G = cold_deployment_t_grace() + 2 × confirmation_interval()`, the interval being the larger
  of the manifest being applied and the one in force (125 s at the default `t_stale_secs =
  120`): members apply one reissue up to one pull interval apart, and the own-leaf swap trails
  the last peer's apply by up to one more. It is derived, never an operator knob.
- **The join source.** The durable join is a `promoted` marker on the peer's pin record (pin
  file schema 2): true once a rotation promotion closed on the current fingerprint, false after
  a re-pair or re-seed. `CertPostGraceReject` is a *candidate* signal, not proof: the verifier
  types a refusal with it only when, in one read of the pin state, the leaf chained to the
  configured `ca_roots`, failed a plain fingerprint comparison against the subject
  (`EPinMismatch::Mismatch`; the dial target, or the SAN-named listen peer) and the subject has
  `promoted = true`. A leaf that chains to the CA, names the subject and is not its pin reads the
  same whether it is the retired key or another leaf issued for that SAN; no retired fingerprint
  is stored. Everything else stays `PeerIdentityUnverified`: an invalidated pin (a promoted peer
  that restarted and presents its current leaf keeps `promoted` until re-pin approval), a
  never-promoted peer, an unknown or non-chaining leaf, every `off`-mode listen refusal, and
  every handshake with `ca_roots` unset (the leaf is its own anchor, so anyone can present one
  whose SAN names a promoted peer; there the retired-key signal is not made at all). The row is
  attributed to the transport event (`a2a:peer-identity-refusal`, no fabricated consent
  envelope) and its `to_spirit_id` is `<direction>@<peer>`, where `<peer>` is the dial target,
  the SAN-named listen peer when the verifier had one, else the socket address. Before this
  amendment's review fixes the listen-side row held only the socket address, so the rotation-close
  ⋈ refusal join could not be made from durable rows for a SAN-less or SAN-unparsed refusal; it
  can for a refusal whose subject the verifier knew. The tracing token `cert_post_grace_reject`
  is unchanged. A WASM guest sees `PeerIdentityUnverified` (the WIT enum has no post-grace
  variant).
- **Own-leaf trigger.** The signed manifest stays the only authority for replacing the serving
  leaf (ADR-062 is not amended): when a reissue moves this host's declaration off the served
  leaf, on every confirmation tick and whenever a peer's `Pull` arrives while they differ, the
  daemon re-reads the configured PEM paths, validates the identity it would commit (the exact
  host SAN under `required`, the chain and both TLS usages), and swaps only if the leaf there
  equals the declaration. The declaration check, the peers' confirmation and the swap are one
  step under the manifest lock, so a newer reissue cannot slip between them. The swap waits until
  every peer this host has an edge with has declared the manifest version in force (14-2b's
  convergence table): a peer that has not applied the reissue pins only the retiring leaf. A
  silent peer cannot hold it past one `t_stale_secs` lease since this host applied the reissue,
  after which the swap runs and a `self_leaf_swapped_unconfirmed` row names the unconfirmed
  peers.
- **No session resumption.** The listener keeps no TLS session cache or tickets: every inbound
  connection runs the verifier, so a resumed session cannot bind a connection to a fingerprint
  since assigned to another peer.
- **Owner move.** The typed reason, the durable join and the SAN-scoped listener move from Story
  21-3 to Story 23-7.

Not decided here: an immediate hard cut for a compromised key (D-23-7-10 A: the exposure bound
is the pull lag per hop plus `G`, and holds only while every peer has an honest pull path to a
member holding the new version; see `RELEASE-HOLDS.md`).
