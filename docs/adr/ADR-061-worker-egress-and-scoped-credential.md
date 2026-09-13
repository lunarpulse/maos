---
Status: ACCEPTED — ratified 2026-09-08 under Story 15-5, decisions F6 and F7
Gate: `cargo test -p xtask --test decision_adrs_and_provisioning`
Decided: 2026-09-08
Accepted-in-PR: pending — Story 15-5
Revisits: recovery-lane decision D-B; Story 17-1; the credential posture guard in `credential_posture_2c.rs`
Supersedes: ambient vendor-key inheritance by the production `cli_wrapper` Worker path once the proxy exists
---

# ADR-061 — Worker egress allowlist and scoped credential

## Context

The production Worker path is the `[cli_wrapper]` subprocess launched by
`spawn_and_bridge` at
`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:449-472`.
`Command::new` inherits the parent environment, then overlays `BridgeSpawnSpec`
environment entries. It does not call `env_clear`.

That behavior is intentional at HEAD. The adapter contract in
`crates/maos-bin/src/worker_cli.rs:121-148` says MAOS has no credential channel:
the operator exports a vendor key into the daemon environment and the child
inherits it. `credential_env_var` exists for scanning and redaction evidence,
not for injection.

The live regression guard is explicit. In
`crates/maos-cli/tests/credential_posture_2c.rs`, the test asserts
`!RUNTIME_SRC.contains("env_clear")` because clearing the environment before a
replacement credential path exists breaks the paid Worker.

T3 container isolation is a separate current path. Its argv builder at
`crates/maos-kernel-core/src/security/sandbox/t3/argv.rs:66-83` emits
`--network=none`, and its spawn path passes no `-e` environment flags. The only
production `spawn_t3` caller is currently a diagnostic busybox smoke, not the
production `cli_wrapper` Worker. T3 is the isolation model, not evidence that
the Worker is already protected.

The manifest has no egress contract. `SandboxConfig` contains only tier and
image pin. Epic 17's `[network]`/egress schema is priced against
`maos-manifest`, not `maos-domain`.

No daemon-side egress proxy exists. The wire-stable `CapabilityToken` at
`crates/maos-domain/src/invariants/i1.rs:175-185` carries token id, Spirit pid,
expiry, and signature. It has no audience, proxy scope, or secret material.
Scope is registry-side through `TokenIssuer::get_token_scope` at
`crates/maos-domain/src/ports/capability.rs:28-29`. “Scoped credential” is
therefore a design obligation, not a primitive already available.

The Transparency Log append entry point is
`TransparencyLogAdapter::insert_frame_event` at
`crates/maos-iac/src/adapter/transparency_log.rs:596`. Its log-before-deliver
contract is fail-closed and existing write failures panic; the proxy must not
weaken or silently bypass that behavior. It must also respect the enforced
`maos-a2a-tcp` to `maos-kernel-core` dependency barrier.

## Decision

Story `17-1-worker-egress-allowlist-and-scoped-credential` makes the
`cli_wrapper` Worker execute under the T3 network-isolation model: internet
egress remains denied, and the Worker's only reachable endpoint is a
daemon-side proxy on host loopback.

The proxy holds the raw vendor credential. It validates each destination
against a manifest allowlist, maps the request to a proxy-scoped per-spawn
bearer, forwards only admitted traffic, and journals every refusal before
returning it. The child never receives the raw vendor key.

The scoped credential is a dedicated proxy bearer with an audience bound to
the local proxy, a Worker/Spirit identity, a permitted destination set, and a
bounded lifetime. It is not the existing wire `CapabilityToken`, and no secret
material is added to that ABI type. The implementation may reuse registry
scope for authorization, but must define the bearer representation and verify
it at the proxy boundary.

Order is part of the decision:

1. Build and wire the proxy and scoped-bearer path.
2. Route the production `cli_wrapper` Worker through that path under the T3
   network model.
3. In the same Story 17-1 commit, add `env_clear`, repopulate only explicit
   non-secret variables plus the proxy endpoint/bearer, and invert the
   `credential_posture_2c.rs` guard.

`env_clear` must never land as standalone hardening before steps 1 and 2. That
would remove the only credential path and knowingly red a shipped control.

The proxy's crate home remains outside `maos-a2a-tcp` and cannot create a
`maos-a2a-tcp -> maos-kernel-core` edge. Story 17-1 owns the final placement
while preserving the architectural dependency barrier.

Story 17-1's projected `+65–130` kernel lines are authorized by the Epic-15
aggregate funding. The `maos-kernel-core` row has zero local headroom at
18935/18935, so its ceiling and baseline pin move together in that story; this
ADR grants no unmeasured local overage.

## Consumers

- `17-1-worker-egress-allowlist-and-scoped-credential` builds the manifest
  schema, proxy, bearer, Worker routing, audit path, and ordered environment
  cutover.
- `19-3-real-worker-default-with-effect-oracle` may run a live Worker only under
  the resulting egress profile.

## Rationale

The threat is ambient credential inheritance in the production
`cli_wrapper` path. Treating current T3 argv as if it already governed that
path would join two disjoint mechanisms and leave the real leak intact.

Proxy-first sequencing preserves working access while replacing raw key
inheritance with a narrow, observable channel. Defining the bearer separately
avoids pretending the existing capability ABI contains fields it does not.

Keeping denial and audit at one daemon-side boundary makes destination policy
independent of vendor CLI behavior and keeps refusal evidence out of the child.

## Consequences

- A Worker cannot reach the public internet directly.
- The raw vendor credential remains only in the daemon-side proxy.
- Child environment construction becomes allowlist-based only after a working
  replacement channel exists.
- Every denied destination produces durable Transparency Log evidence.
- Manifest schema/version and admission coverage change in Story 17-1.
- Existing T3 `--network=none` semantics remain the model and become real for
  the Worker rather than remaining confined to a diagnostic smoke.
