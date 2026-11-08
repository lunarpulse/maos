# Spirit Registry Security Posture

## Deployment boundary

Production uses **rustls TLS plus a separately configured Ed25519 registry root**.
The leaf uses bundled WebPKI roots, optionally extended with an operator CA, and
verifies the certificate hostname and validity. Neither a response-supplied key
nor a valid certificate by itself authorizes registry metadata.

Provision the registry signing key out of band. The server reads its 32-byte hex
seed from `--signing-key`; supply the certificate chain with `--tls-cert`, its
private key with `--tls-key`, and the canonical external HTTPS `/mcp` URI with
`--uri`. Keep key files operator-owned, mode0600, outside packages and the registry
store. Certificate/key and vetter authorization changes require a server restart.
TLS permits a non-loopback bind; it does not introduce anonymous mutation access.

Installers, producers and daemon pollers share `$HOME/.config/maos/operator.toml`:
`[registry].uri` is the exact canonical registry URI, `signing_pubkey` is the
out-of-band 64-hex-character public key, and optional `ca_certificate` is a PEM CA
file path. The root key must be bound to that URI, including when supplied for
development HTTP. URI aliases, credentials, query and fragment are refused.
HTTPS without a configured root refuses before contact; there is no TOFU,
response-key bootstrap, redirect following or HTTPS-to-HTTP fallback.
Use absolute operator-controlled CA paths for consistent CLI/daemon working
directories; a relative CA path is opened relative to the invoking process.
This is a forward installation guarantee, not retroactive authentication of
earlier unsigned HTTP installs. Noncanonical legacy origins are not silently
rewritten: use explicit operator uninstall/reinstall when migrating them, with
the durable publisher/version pins and local pin-release rules below still active.

Plain HTTP is an explicit **development exception**: server `--dev-http` and
installer/producer `[registry].development_http = true` are both required.
Only literal loopback IPs are permitted (not `localhost`/DNS aliases); the client
sends an explicit development handshake header. Server Host/Origin/path/type
guards remain enforced. Without a configured signing root, this mode provides
**neither peer authentication nor confidentiality nor yank omission/freshness
protection** against a hostile local peer/interceptor. Mutation authorization,
present-yank signatures, publisher/version/origin pins and admission still apply.
Providing a URI-bound root also authenticates development responses/heads, but
never turns plaintext HTTP into confidential transport.

AC15 local witnesses use freshly rcgen-generated test CA/leaf keys, real TLS,
wrong-root/URI refusal, wrong-host/expired/untrusted certificates, plaintext traps,
and real CLI persistence oracles. The branch-only hosted recipe is authored but
not executed; hosted interception and macOS compilation remain unverified.

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
or an enrolled vetter before refusing it, before persistence. HTTPS additionally
requires an authenticated response/list head before persistence: a stripped entry,
missing envelope/head, nonce/URI mismatch or sequence gap refuses. Local publisher
pins alone are not authenticated first contact; unsigned development remains the
explicit exception above.

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
Unknown uninstalled publishers never acknowledge a cursor. Signed response envelopes
bind a fresh256-bit request nonce, exact request/response SHA-256, canonical URI and
complete durable yank head from one snapshot. Incremental lists must reach that
head contiguously from the requested cursor; no unsigned head advances trust.
This prevents network replay/omission, not a configured signing authority lying,
its signing-key compromise, or a fresh client's detection of an authority-signed
rollback. Neither an unset HOME nor an empty explicit cursor path selects `/tmp`.

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
