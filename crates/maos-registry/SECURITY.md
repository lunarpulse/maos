# Spirit Registry Security Posture

## Deployment boundary

The server remains a **loopback development listener over plaintext HTTP**.
It rejects non-loopback listeners and enforces Host/Origin/content-type/path guards.
Those guards are not authentication and do not establish a first-contact trust root.
Do not expose this listener publicly or treat a reverse proxy as an implemented
end-to-end authenticated installation path.

Story `20-1b-vetting-yank-policy-and-quarantine` AC15 (operator `f1b`, 2026-11-06)
requires authenticated first contact and TLS. D-20-1b-01/02/03 were **RULED A/A/A**
on 2026-11-07: a URI-bound configured signing key, rustls with operator-managed
certificate/key paths, and explicit both-end development opt-in for literal
loopback HTTP. Execution is held for the separate AC15 handoff. Hostile-registry
and interception refusal proof remains owed. No TLS security or public deployment
claim is made by this kernel-free correction landing.

## Authenticated mutations

- `registry.publish` requires a valid Ed25519 publisher signature over the exact
  manifest and artifact. The signed manifest identity must match the requested
  name/version. The first publisher key owns the name across all versions;
  another key is refused, including after a server restart.
  Published name/version bytes are immutable, including for the same key;
  byte-identical republishing is idempotent. Initial package files become visible
  together by an atomic staged-directory rename.
- `registry.deprecate` requires signature-preimage/authorization **v2**. Its CBOR
  tuple binds the publisher/vetter role, format version, exact registry URI,
  128-bit random nonce, maximum age, name, version, reason, origin, signing key,
  vetter id and issuance time. Nominal lifetime is at most five minutes, with a
  named five-minute clock-skew allowance; nonce reuse for a different withdrawal
  refuses. V1 credentials are not accepted.
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
cannot change package bytes, publisher or origin; a different registry cannot
overwrite it, even with the same key and byte-identical package. The durable
`installed-pins.json` retains publisher and version digests across uninstall;
legacy signed generations are imported before mutation/removal. Only the local
operator's `maos-registry-server [--root DIR] release-pin NAME` explicitly releases
that ledger entry. Retained installed generations remain immutable. A missing
`HOME` requires an explicit root for this verb.

Newly verified promotion evidence may atomically replace `vetting.json` only for
an otherwise identical package and origin; renewal does not require erasing data.
Installation verifies each present matching yank against the package publisher
or an enrolled vetter before refusing it, before persistence. **An omitted yank
is not detectable here:** authenticated first contact/transport and head integrity
remain AC15 dependencies. Local publisher pins are integrity/provenance constraints,
not authenticated first contact.

Vetting claims bind exact manifest bytes, artifact SHA-256 and publisher key.
`maos-spirit vet issue` signs a package claim; `vet revoke` updates the supplied
keyring with an operator-root-signed attestation revocation list. The supplied live
keyring path is retained with the installation so running-promotion re-evaluation
can observe later expiry and revocation. Internal-key machinery does not establish
an independently accredited external-vetter ecosystem.
There is no shipped vetter-key enrollment/revoke-key operator verb: the lifecycle
tests use explicitly fixture-built keyrings. That operator lifecycle remains open
under 20-1b, not an accredited-vetter claim. Live-keyring rollback protection and
CLI keyring mutation locking are also not supplied here.

Attestation and operator key-event signatures now use distinct v2 role domains:
SHA-256(domain || NUL || little-endian u64 wire-length || wire-claim), then Ed25519.
Canonical claim CBOR is unchanged; old unscoped signatures must be reissued.
The CRL/attestation-revocation preimages still share the existing kernel contract;
changing their role format is deferred to the final kernel landing.

Yanks retain signed publisher/vetter origins and a durable server-assigned sequence.
The old `since_ns` wire field now means sequence; display wall time never drives the
cursor. Schema-2 cursor files bind the exact registry URI, persist each verified
row atomically with fsync, and reset on obsolete/corrupt/foreign-registry state.
Gaps/overshoots trigger a journaled history resync, never trusted advancement.
Permanent rejected rows are journaled; only a verified row advances the frontier.
Unknown uninstalled publishers never acknowledge a cursor. Without a signed head,
withholding/rollback of registry history remains an AC15 dependency. Neither an
unset HOME nor an empty explicit cursor path selects a shared `/tmp` fallback.

Running-promotion reevaluation has an independent local timer even without a
registry source, including installs appearing after boot. The remote poller is
not started for a null source; cancellation does not await synchronous network I/O.
The cadence defaults to 300 seconds and clamps to 30..300; zero is refused rather
than silently disabling expiry. Shell/worker/oneshot drains join timer/poller
owners before the audit writer, so no poller retains the audit channel at drain.

`MAOS_YANK_POLICY=warn` (default) records the event. `quarantine` records a typed
`quarantine-requested` outcome; **L1 does not claim its isolation effect**. The final
KQ1 kernel landing supplies the sticky fence. `auto-revoke` signs an exact-version
local CRL with origin `RegistryYank` and invokes the existing applier, preserving
the Spirit's declared revocation action rather than implementing a second applier.
