# `maos.kms.v1` — the external KMS wire

Story 23-8. ADR-051 amendment "Story 23-8". MAOS wraps and unwraps the 32-byte
data key of every sealed row through an HTTP endpoint that the operator runs.
The endpoint is a small sidecar in front of the organisation's KMS or HSM; the
`maos` binary carries no vendor API. `examples/kms-sidecar-openbao/` is a
reference sidecar for OpenBao Transit (an example, not a release artifact, not
security-reviewed). This document is the contract: the MAOS adapter
(`maos_secrets::http_kms::HttpKms`), the test fixture and the sidecar all follow
it.

## Transport

- `MAOS_KMS_ENDPOINT` is a **base URL**: `https://host[:port][/prefix]`, or
  `http://` to a **literal loopback address** only (`127.0.0.1`, `[::1]`). No
  userinfo, query or fragment. The three paths below are appended to it.
- The URL is set by the operator. MAOS never reads an endpoint from a peer or
  from a response, and never follows a redirect: a 3xx is a refusal.
- The shared outbound policy (`maos_domain::outbound`) vets it once at boot:
  link-local, reserved and unspecified addresses never; private (RFC 1918,
  fc00::/7) **and loopback** addresses only with `MAOS_KMS_ALLOW_PRIVATE=1`.
  With that opt-in both are admitted by any route, including a name that
  resolves to them (`https://localhost:8443` boots), and because the connection
  resolves the name again it can move between them. Without it a name that
  resolves to loopback is refused. Plain `http` stays literal loopback only,
  opt-in or not. **For a host name the policy does not bind the connection to
  the addresses it vetted** (RELEASE-HOLDS 23-0.a, DNS rebinding): MAOS makes no
  SSRF claim for names. Use a literal address, or accept the residual.
- Timeouts, **for a literal-IP endpoint**: connect 2 s, read 2 s, write 2 s,
  overall 5 s for `POST /wrap` and `POST /unwrap`, and 2 s overall for
  `GET /health`. **For a host name the DNS lookup is not bounded by any of
  these** (ureq 2.12 applies no deadline to it), so a call or a probe can block
  longer; fail-closed still holds, because health goes stale after 30 s and
  writes are refused. Responses are capped at 64 KiB. Bodies are
  `application/json`.
- The data key crosses the wire in base64, not encrypted: the endpoint must be
  loopback or TLS. **Loopback `http` does not authenticate the listener**: MAOS
  sends the data key and the bearer to whatever process listens on that port. If
  the sidecar is down, another local user can bind the port, answer `/health`
  ok, collect data keys and the bearer, and later replay the bearer to the real
  sidecar. Required deployment condition: run the sidecar on a privileged or
  otherwise unclaimable port, or under the same OS user with the port bound
  before MAOS starts, or put TLS in front and use `https`. A Unix-domain-socket
  owner check is not implemented (D-23-8-01 B was not chosen).
- `https` trusts the bundled webpki root set only; there is no CA-bundle
  setting. **Private-CA `https` endpoints are not supported in this story**: the
  handshake fails and reads as unavailable (writes refused). Use a loopback
  sidecar or a publicly-trusted certificate; a CA-bundle setting is a follow-up
  decision (RELEASE-HOLDS 23-8.g).
- Optional bearer: `Authorization: Bearer <MAOS_KMS_BEARER>` on `POST /wrap`
  and `POST /unwrap`, to the configured endpoint only. `GET /health` carries no
  credential. The bearer is in no error, log line, journal row or `Debug`
  output. A bearer that is still being read, or could not be read, makes every
  call refuse without sending anything; a rejected bearer (`401`/`403`) marks the
  endpoint down at once. MAOS logs one line per change of failure class (status
  class or transport), never a value.

## Messages

Every request and response carries `"version": "maos.kms.v1"`. A different
version, a missing field, a field of the wrong type, bad base64, an unwrap
result that is not 32 bytes, a wrapped key that is empty or longer than 4,096
bytes, or a body that is not JSON, is a malformed message.

| Request | Success |
|---|---|
| `POST /wrap` `{"version","plaintext":<base64 of 32 bytes>}` | `200 {"version","wrapped":<base64, 1..4096 bytes>}` |
| `POST /unwrap` `{"version","wrapped":<base64, 1..4096 bytes>}` | `200 {"version","plaintext":<base64 of 32 bytes>}` |
| `GET /health` | `200 {"version","status":"ok"}` |

`wrapped` is opaque to MAOS. It is stored in the sealed envelope
(`MAOSKMS1 | u16 wrapped_len | wrapped | nonce | ciphertext`) next to the row.

## Errors

| Answer | MAOS result | Health |
|---|---|---|
| `422 {"error":"wrap_failed"}` or `{"error":"unwrap_failed"}` | `KmsError::Crypto` | **unchanged** (a refusal of one payload, not of the endpoint: D-23-8-10 A) |
| any 2xx or a 422 that is a malformed message | `KmsError::MalformedPayload` | down at once |
| 3xx, 4xx other than the typed 422, 5xx | `KmsError::Unavailable` | down at once |
| connection refused, timeout, TLS failure, body over the cap | `KmsError::Unavailable` | down at once |

A sidecar answers `502` or `503` when its KMS does not answer, and `422` only
when its KMS refuses this one payload (for example a wrapped key it did not
produce).

## Health

`HttpKms::is_healthy()` does no I/O. It is true only when the last `GET /health`
probe succeeded within three probe intervals (default 10 s: 30 s) on a monotonic
clock, and no availability failure (table above) happened since: a probe that
started before a failing call cannot clear that failure. `maos` probes
once at boot and then every 10 s, on a blocking thread, until shutdown; a probe
that is running cannot be cancelled, so a shutdown can wait for it (2 s for a
literal-IP endpoint). An
endpoint that is down at boot is not a boot refusal: sealed writes and reads of
sealed values are refused until a probe succeeds. A stopped prober therefore
means an unhealthy KMS and refused writes, never plaintext.

## What this does not do

- It does not rotate or re-wrap existing rows.
- It does not make Collective rows readable: sealed Collective rows are
  write-only until a reader exists (RELEASE-HOLDS, Epic 23, row 23-8.a). The one
  production reader of a sealed value is the encrypted-file secret store.
- It does not seal identity assertions: `identity.asserted` rows in the
  Transparency Log are plain JSON and are not sealed by this story.
- The air-gap build (no `online-adapters`) refuses `MAOS_KMS_ENDPOINT`
  (`kms_endpoint_unsupported_build`) and keeps the local master key only.
