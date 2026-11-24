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
  private (RFC 1918, fc00::/7) addresses only with `MAOS_KMS_ALLOW_PRIVATE=1`;
  link-local, reserved and unspecified addresses never; a name that resolves to
  loopback never. **For a host name the policy does not bind the connection to
  the addresses it vetted** (RELEASE-HOLDS 23-0.a, DNS rebinding): MAOS makes no
  SSRF claim for names. Use a literal address, or accept the residual.
- Timeouts: connect 2 s, read 2 s, write 2 s, overall 5 s. Responses are capped
  at 64 KiB. Bodies are `application/json`.
- The data key crosses the wire in base64, not encrypted: the endpoint must be
  loopback or TLS.
- Optional bearer: `Authorization: Bearer <MAOS_KMS_BEARER>` on `POST /wrap`
  and `POST /unwrap`, to the configured endpoint only. `GET /health` carries no
  credential. The bearer is in no error, log line, journal row or `Debug` output.

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
clock, and no availability failure (table above) happened since. `maos` probes
once at boot and then every 10 s, on a blocking thread, until shutdown. An
endpoint that is down at boot is not a boot refusal: sealed writes and reads of
sealed values are refused until a probe succeeds. A stopped prober therefore
means an unhealthy KMS and refused writes, never plaintext.

## What this does not do

- It does not rotate or re-wrap existing rows.
- It does not make Collective rows readable: sealed Collective rows are
  write-only until a reader exists (RELEASE-HOLDS, Epic 23, row 23-8.a). The one
  production reader of a sealed value is the encrypted-file secret store.
- The air-gap build (no `online-adapters`) refuses `MAOS_KMS_ENDPOINT`
  (`kms_endpoint_unsupported_build`) and keeps the local master key only.
