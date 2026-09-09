---
Status: ACCEPTED — ratified 2026-09-08 under Story 15-5, decision F8
Gate: `cargo test -p xtask --test decision_adrs_and_provisioning`
Decided: 2026-09-08
Accepted-in-PR: pending — Story 15-5
Revisits: `crates/maos-control/src/lib.rs:66-77`; read-only route tests at lines 607-611, 786-790, and 927-931; Story 16-1
Supersedes: nothing — this supersedes a rustdoc decision, not an ADR
---

# ADR-062 — One mutating operator surface

## Context

`maos-control` currently exposes a read-only loopback HTTP surface. Its
`handle_connection` authenticates the bearer before dispatch at
`crates/maos-control/src/lib.rs:286-297`, then serves four GET routes:
rotation windows, peer manifest versions, self identity, and per-Spirit
sandbox reports. The final route is parsed from the `/v1/spirits/` prefix.

The server reads only through the end of the HTTP headers
(`lib.rs:273-283`). It has no request-body parser. Its catch-all responds 404,
and `respond` maps 200, 401, 404, 503, and a catch-all 500; there is no 405 Method Not
Allowed phrase today.

The rustdoc decision at `lib.rs:66-77` describes the surface as READ-ONLY and
warns against a second trust path. Three tests preserve that posture. Two are
certificate-identity guards, not generic objections to operator mutation:
cohort manifest and rotation state may change only through signed manifest
reissue. Story 21-3 still depends on those guards.

`maos-cli` already has a loopback HTTP client at
`crates/maos-cli/src/subcommands.rs:1930-1975`. It performs authenticated GET
using `MAOS_OPERATOR_HTTP_ENDPOINT` and `MAOS_OPERATOR_BEARER_TOKEN`. The
required client delta is POST, request bodies, and typed errors—not acquiring
an HTTP client.

The existing `crates/maos-cli/tests/dep_kernel_core_free_test.rs` enforces that
`maos-cli MUST NOT depend on maos-kernel-core`. Linking `maos-control` would
cross that boundary because `maos-control` depends on kernel-core. Discovery
therefore remains a data contract rather than an in-process library call.

`maos init` has two execution arms (`crates/maos-bin/src/main.rs:1352` and
`:1699`). Both currently delegate to `maos_shell::run_init`, which writes
`MAOS_HOME/config.toml`. `control.json` does not exist in production source.

## Decision

The `maos run` and `maos shell` daemon process owns one loopback HTTP control
door. The door requires the existing bearer and accepts route-scoped POST
requests for state-changing operator verbs.

`maos init` mints the endpoint and bearer and writes both to
`MAOS_HOME/control.json`. Both init arms use the same writer. `maosctl`
discovers the endpoint and bearer from that file; the environment variables
remain explicit overrides. This preserves one trust path.

Story `16-1-daemon-post-surface-and-verb-retarget` implements POST parsing,
bounded request-body parsing, typed transport/application errors, and command
routing. `maos-cli` must not link `maos-control` or `maos-kernel-core`.

The general read-only posture is superseded. The certificate-identity
write-path exclusion is not. Cohort membership, peer identity, certificate
rotation, and serving-leaf replacement remain driven by signed manifest
reissue and never gain an operator POST mutation route.

The three existing read routes answer 405 to an authenticated unsupported
method. The two certificate-identity tests remain guards against mutation;
they are not inverted into acceptance tests. The generic read-only assertion
is narrowed to the preserved routes.

Authentication continues before route or method dispatch. An anonymous caller
therefore receives 401 without learning whether a path supports POST.

Every 405 body is a fixed byte literal. It must not echo or interpolate the
request line, path, headers, or body. `respond` accepts `&[u8]` literals today;
that property remains the response boundary so an operator log cannot become
an injection surface.

The stale `main.rs:9844-9853` pointer in the superseded rustdoc comment is not
carried forward. It no longer identifies the claimed warning.

## Consumers

- `16-1-daemon-post-surface-and-verb-retarget` builds the POST door,
  `control.json`, discovery, 405 handling, and verb retargeting.
- `16-2-shell-halt-registry-and-j0-scene` uses the door for halt operations.
- `19-2-fr20-enqueue-door-and-safe-point` uses it for queue mutation.
- `21-3-durable-tofu-and-self-leaf-rotation` relies on the preserved signed
  manifest identity path rather than an operator mutation route.

## Rationale

A running daemon must own runtime state, so mutating one-shot CLI processes
cannot be authoritative. A single authenticated loopback door makes that
ownership observable without creating another in-process dependency edge.

Preserving the signed identity path separates operator control from peer trust.
The decision changes the general surface posture without deleting the exact
guards the certificate-rotation work still needs.

405 after authentication is precise for an unsupported method and does not
expose route existence to anonymous scanners. A static response body keeps the
new precision from introducing request reflection.

## Consequences

- `MAOS_HOME/control.json` becomes the discovery contract for local operator
  clients and must be written atomically with restrictive permissions.
- Existing GET reads remain authenticated and keep their current payloads.
- Runtime mutations reach daemon-held state rather than fresh-process state.
- Certificate and cohort identity state remain writable only by signed reissue.
- `MAOS_OPERATOR_HTTP_ENDPOINT` remains an override and must be registered when
  Story 21-4 widens environment-contract scanning to every crate.
