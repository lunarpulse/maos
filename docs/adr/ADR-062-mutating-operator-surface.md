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

## Amendment — Story 16-1

Amended in the same commit as the code it governs: measurement against the
built binaries disproved premises inside the Context and Decision above. Each
point below is a decision with its measured reason; sections above are not
rewritten except where a point says it supersedes them.

- **Door roots.** The daemon binds the door in exactly three roots — `maos run`,
  `maos shell` (a bare `maos` included) and `MAOS_ONE_SHOT=cohort-a2a-daemon` —
  gated by root, never by env presence. Measured: the bind preceded the one-shot
  dispatch and `maosctl` spawns do not `env_clear`, so every one-shot child bound
  the door and exited 1 `Address already in use`. The root set is static and
  reviewable; env presence is not a gate, it is data.
- **The three read-route POST tests are all guards.** The Decision asked that
  "the generic read-only assertion" be narrowed; measurement says none of the
  three is generic. All three HEAD `POST → 404` tests become post-auth 405
  guards — the third guards `self-identity`, which *is* "this host's signed and
  serving certificate identity", preserved above as a read. Nothing is narrowed;
  all three keep guarding.
- **The 405 phrase now exists.** The Context statement that there is "no 405
  Method Not Allowed phrase today" (`:21-23`) is SUPERSEDED:
  `crates/maos-control/src/lib.rs` answers 405 with the fixed body
  `{"error":"method_not_allowed"}` and the wire phrase `405 Method Not Allowed`.
  The governance-gate clause that asserted `require_absent("Method Not
  Allowed")` is flipped to `require_contains` in the same commit, so the gate
  now REDS if the phrase disappears again.
- **`control.json` keeps one writer: `maos init`.** Liveness is NOT recorded in
  the file — a file-stamped heartbeat is stale-state by construction. Liveness
  is the store lock set: `flock` on the home directory's own handle and on every
  durable store's directory handle. No lock file is ever created — a `.lock`
  file inside a store directory would change the file counts
  `erasure_uninstall_13_5b` asserts. Roots hold the set shared.
- **The durable offline arm.** The four durable verbs (`forget`, `uninstall`,
  `legal-hold-release`, `governance-admit`) run offline only when the offline
  child acquires the whole set EXCLUSIVELY; any lock held ⇒ typed `StoreInUse`.
  Connected-but-unresponsive is `DoorUnresponsive` and NEVER falls back to
  offline: a live daemon that cannot answer may still hold unflushed state, and
  writing around it is exactly the split-brain the set exists to prevent.
- **The 8787 fallback is retired.** The token-only `127.0.0.1:8787` fallback
  port is gone: configuration is the env overrides (both-or-neither) else
  `control.json`, and a silent port guess has no third place in that order.
- **`control.json` schema.** `{"version":1,"endpoint":"tcp://127.0.0.1:<port>",
  "token":"<64 hex>"}`, file mode `0600`, home `0700`. Every reader refuses an
  unknown `version` or a non-`tcp://` scheme: the version field is what lets a
  future transport (a Unix socket dissolves half this threat model) change the
  shape without old readers guessing.
- **Residual threat, stated plainly.** Until Story 17-1 lands, a Worker
  inherits the daemon's environment and runs as the operator uid, so a bare,
  prompt-injectable agent CLI can read the `0600` `control.json` and use any
  exported `MAOS_OPERATOR_BEARER_TOKEN`. The cure is 17-1's own `env_clear`
  plus T3 isolation; naming it here is not a fix and must not read as one.
- **The `operation_id` contract.** A `503 handler_still_running` carries the
  id the PORT minted (the port, never the server — the server cannot know what
  it did not submit); the command runs to completion and writes exactly one
  Transparency-Log row whose `intent` contains that id, found by
  `maosctl audit query --intent-contains <id>`. A `503 spirit_busy` carries NO
  id because the command was withdrawn before it started — there is nothing to
  look up.
- **Upgrade over the door is hot-swap only, forward only, faithful
  successor.** `cold-swap` is refused typed: the kernel's cold-swap arm starts
  the successor under a new pid without `admit_spirit`, a door success there
  would report an unadmitted process. A non-increasing successor version is
  refused typed (measured: 0.3.0 → 0.2.0 reported `completed`). CRL import
  carries the CRL's own bytes, never a path — the daemon's working directory is
  not the operator's.
