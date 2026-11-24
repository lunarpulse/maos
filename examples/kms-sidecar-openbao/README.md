# kms-sidecar-openbao

A reference sidecar that lets MAOS wrap and unwrap its at-rest data keys through
[OpenBao](https://openbao.org) Transit. It speaks `maos.kms.v1`
([`docs/reference/kms-wire-v1.md`](../../docs/reference/kms-wire-v1.md)) to MAOS and
OpenBao's Transit `encrypt` and `decrypt` API to OpenBao.

**This is an example. It is not a release artifact, it is not in the MAOS binary,
and it has not been security-reviewed.** Read it before you trust it with a key.
It is Python 3, standard library only, and no part of the Cargo workspace.

| | |
|---|---|
| OpenBao | `v2.7.3` (2026-11-19), Transit API docs 2.7.x: <https://openbao.org/docs/api/secret/transit/> |
| OpenBao licence | MPL-2.0 (the sidecar does not link or copy OpenBao code; it calls its HTTP API) |
| Transit key type | `aes256-gcm96` (the Transit default) |

## Run it

```sh
# OpenBao side, once: enable Transit and create the key MAOS will use.
bao secrets enable transit
bao write -f transit/keys/maos type=aes256-gcm96

# A token that may only encrypt and decrypt with that one key, and read it:
bao policy write maos-kms - <<'EOF'
path "transit/encrypt/maos" { capabilities = ["update"] }
path "transit/decrypt/maos" { capabilities = ["update"] }
path "transit/keys/maos"    { capabilities = ["read"] }
EOF
export BAO_TOKEN="$(bao token create -policy=maos-kms -field=token)"

# Sidecar, beside MAOS on the same host:
export BAO_ADDR=http://127.0.0.1:8200
export SIDECAR_BEARER="$(head -c32 /dev/urandom | od -An -tx1 | tr -d ' \n')"   # optional
python3 kms_sidecar.py            # serves 127.0.0.1:8300

# MAOS side:
export MAOS_KMS_ENDPOINT=http://127.0.0.1:8300
export MAOS_KMS_BEARER="$SIDECAR_BEARER"                                          # optional
maos
```

Leave `MAOS_KMS_MASTER_KEY` unset: setting it together with `MAOS_KMS_ENDPOINT`
refuses the boot. Environment of the sidecar: `BAO_TOKEN` (required),
`BAO_ADDR`, `BAO_TRANSIT_KEY` (default `maos`), `BAO_TRANSIT_MOUNT` (default
`transit`), `SIDECAR_BIND` (default `127.0.0.1:8300`), `SIDECAR_BEARER`.

Self-test (a stub Transit on loopback; no OpenBao needed): `python3 selftest.py`.
The MAOS test suite runs it
(`crates/maos-bin/tests/external_kms_wiring_23_8.rs`).

## How the wire maps to Transit

| `maos.kms.v1` | OpenBao |
|---|---|
| `POST /wrap` `{"version","plaintext":<b64 32 bytes>}` | `POST /v1/transit/encrypt/<key>` with `plaintext` (base64); the answer's `ciphertext` (`vault:v1:...`) is returned as `wrapped`, base64 of its bytes |
| `POST /unwrap` `{"version","wrapped":<b64>}` | `POST /v1/transit/decrypt/<key>` with `ciphertext`; the answer's `plaintext` is returned |
| `GET /health` | `GET /v1/transit/keys/<key>` with the token; `200` is `{"status":"ok"}`, anything else `503` |
| OpenBao `400` (invalid ciphertext) | `422 {"error":"unwrap_failed"}` or `{"error":"wrap_failed"}`: a refusal of one payload, which MAOS does not count as an outage |
| OpenBao unreachable, `403`, `5xx` | `502`: MAOS marks the KMS unavailable and refuses sealed writes until a probe succeeds |

## Threat notes

- **The OpenBao token lives in the sidecar's environment.** MAOS never sees it.
  It is sent only as `X-Vault-Token` to `BAO_ADDR`, and it is never logged. A
  local user who can read the sidecar's environment (`/proc/<pid>/environ`) can
  use it: run the sidecar under its own account with a token limited to the
  policy above.
- **The data key crosses this hop in the clear** (base64, not encrypted), as it
  must for any envelope scheme. The sidecar therefore binds loopback only and
  refuses a non-loopback `SIDECAR_BIND` unless `SIDECAR_ALLOW_NON_LOOPBACK=1`;
  if you set that, put TLS in front and point `MAOS_KMS_ENDPOINT` at the `https`
  URL. MAOS accepts plain `http` only to a literal loopback address.
- **`SIDECAR_BEARER`** keeps another local process from asking the sidecar to
  decrypt. Without it any local process that can reach the port can unwrap.
- **Nothing is logged but method and path.** No body, header or key reaches the
  log.
- **No rate limit, no audit trail of its own.** OpenBao's audit device is the
  record of who decrypted what.
- **A host name for `BAO_ADDR` is resolved by the operating system on every
  call**; use a literal address or a name you control.
- **The call to OpenBao never follows a redirect and ignores proxy variables.**
  `urllib`'s default opener follows redirects (the token would travel to the
  redirect target) and honours `HTTP_PROXY` / `HTTPS_PROXY` / `ALL_PROXY` (the
  token and the data key would pass the proxy). The sidecar builds an opener
  with neither; a 3xx from OpenBao is a `502`. Each client connection has a
  10 s socket timeout, so a client that stalls cannot hold a handler thread.
  `selftest.py` asserts all three.
- Rotating the Transit key (`bao write -f transit/keys/maos/rotate`) keeps old
  wrapped keys readable; re-wrapping existing rows is not done by MAOS (story
  23-8 does not rotate).

MAOS side of the same limits (details: `docs/reference/kms-wire-v1.md`,
RELEASE-HOLDS rows 23-8.a to 23-8.g):

- `MAOS_KMS_ENDPOINT` is operator configuration and is never read from a peer;
  for a host **name** MAOS's outbound policy does not bind the connection to the
  addresses it vetted (RELEASE-HOLDS 23-0.a), so a name is not SSRF-protected:
  prefer a literal address. `MAOS_KMS_ALLOW_PRIVATE=1` admits private **and
  loopback** addresses, including a name that resolves to them (the connection
  resolves the name again, so it can move between them).
- **Loopback `http` does not authenticate the sidecar.** MAOS sends the data key
  and the bearer to whatever process listens on the port. If the sidecar is
  down, another local user can bind the port, answer `/health`, collect data
  keys and the bearer and replay the bearer to the real sidecar. Run the sidecar
  on a privileged or otherwise unclaimable port, or under the same OS user with
  the port bound before MAOS starts, or put TLS in front and use `https`. A
  Unix-domain-socket owner check is not implemented.
- **No private-CA `https`:** MAOS trusts the bundled webpki roots only, so an
  `https` endpoint behind an internal PKI fails the handshake and writes are
  refused. Use a loopback sidecar or a publicly-trusted certificate; a
  CA-bundle setting is a follow-up decision.
- The 5 s call timeout and the 2 s probe timeout hold for a literal-IP endpoint;
  for a host name the DNS lookup is not bounded by them (writes are still
  refused once health goes stale, 30 s).
- Sealed Collective rows are write-only until a reader exists (RELEASE-HOLDS,
  Epic 23, row 23-8.a). Identity assertions in the Transparency Log are not
  sealed by this story.
