# Spirit Registry Security Posture

## Deployment boundary

The server remains a **loopback development listener over plaintext HTTP**.
It rejects non-loopback listeners and enforces Host/Origin/content-type/path guards.
Those guards are not authentication and do not establish a first-contact trust root.
Do not expose this listener publicly or treat a reverse proxy as an implemented
end-to-end authenticated installation path.

Story `20-1b-vetting-yank-policy-and-quarantine` AC15 (operator `f1b`, 2026-11-06)
requires authenticated first contact and TLS. Its D-20-1b-01/02/03 choices are
**PROPOSED**, not silently implemented. Hostile-registry and interception refusal
proof remains owed after that ruling. No TLS security or public deployment claim
is made by the kernel-free landing.

## Authenticated mutations

- `registry.publish` requires a valid Ed25519 publisher signature over the exact
  manifest and artifact. The signed manifest identity must match the requested
  name/version. The first publisher key owns the name across all versions;
  another key is refused, including after a server restart.
- `registry.deprecate` requires a signed, domain-separated yank authorization
  binding name, version, reason, origin, signing key, vetter id and issuance time.
  Publisher yanks verify against the stored package owner. Vetter yanks require
  operator-root-signed enrollment predating issuance and no effective revocation.
  Configure both `--vetter-keyring <CBOR-path>` and `--operator-root <hex-key>`;
  neither a response-provided key nor loopback membership authorizes a vetter.
  The server loads that keyring at startup: restart it after enrollment or key
  revocation updates. This is not a hot-reloaded server authorization store.
- Unsigned requests and authenticated requests from the wrong owner or an
  unenrolled vetter refuse without changing package/yank storage.

## Installation and revocation

Installation verifies publisher signatures, semantic ComplianceClaims, admission
policy and any promotion attestation before activation. An installed name/version
cannot change package bytes, publisher, origin or promotion metadata; a different
registry cannot overwrite it. A yanked version refuses before local persistence.
Publisher pins are local integrity/provenance constraints, not authenticated first
contact; AC15 remains required.

Vetting claims bind exact manifest bytes, artifact SHA-256 and publisher key.
`maos-spirit vet issue` signs a package claim; `vet revoke` updates the supplied
keyring with an operator-root-signed attestation revocation list. The supplied live
keyring path is retained with the installation so running-promotion re-evaluation
can observe later expiry and revocation. Internal-key machinery does not establish
an independently accredited external-vetter ecosystem.

Yanks retain signed publisher/vetter origins and a durable server-assigned sequence.
The old `since_ns` wire field now means sequence; display wall time never drives the
cursor. Cursor files are atomically persisted, without monotonic-clock remapping.
Historical unsigned yank rows remain refusal evidence for installation but cannot
authorize daemon effects. Old manifest-only attestations require reissuance.

`MAOS_YANK_POLICY=warn` (default) records the event. `quarantine` records a typed
`quarantine-requested` outcome; **L1 does not claim its isolation effect**. The final
KQ1 kernel landing supplies the sticky fence. `auto-revoke` signs an exact-version
local CRL with origin `RegistryYank` and invokes the existing applier, preserving
the Spirit's declared revocation action rather than implementing a second applier.
