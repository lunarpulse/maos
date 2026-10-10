---
Status: ACCEPTED — ratified 2026-09-08 under Story 15-5, decisions F6 and F7; transport AMENDED 2026-10-06 (operator-ratified under Story 17-1 — see Amendment)
Gate: `cargo test -p xtask --test decision_adrs_and_provisioning`
Decided: 2026-09-08
Accepted-in-PR: pending — Story 15-5
Revisits: recovery-lane decision D-B; Story 17-1; the credential posture guard in `credential_posture_2c.rs`
Supersedes: ambient vendor-key inheritance by the production `cli_wrapper` Worker path once the proxy exists
---

# ADR-061 — Worker egress allowlist and scoped credential

## Context

The production `[cli_wrapper]` Worker enters
`crates/maos-bin/src/worker_spawn.rs` and reaches `spawn_and_bridge` in
`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs`. Story 17-1
replaces the former ambient vendor-key inheritance at this boundary. Plain
command and admission-probe children use `env_clear`; only explicitly selected
non-secret environment entries are restored.

The T3 arm requires a verified `T3Launch`, never a bare host process. Its image
lock is signature-verified against the configured trust anchor and its runtime
argv keeps `--network=none`. The Worker's lease supplies a per-spawn Unix socket,
mounted at `/proxy/egress.sock`; the std-only forwarder exposes the vendor-compatible
loopback endpoint inside that network namespace. Only a daemon-minted scoped
bearer reaches the child, through the adapter's explicit environment. A Codex
base-URL CLI-config override is non-secret and follows the hash-bound prefix,
before task arguments; Claude uses its supported URL environment variable.

The daemon owns the raw vendor credential and the hash-only bearer table.
Authorization is the exact intersection of the typed manifest egress list and
the host-managed grant, bound to Worker identity, destination and expiry.
Neither the endpoint adapter nor the child receives the raw vendor key.

T3 currently admits only Raw/Signals and NdjsonOverStdio/Signals. All seven
stdin-keeping combinations refuse at load, before probing or Worker binding.
The first production stdin writer owns interactive container support; 17-1
does not silently approximate it.

The obsolete inheritance/source-wording guard in
`crates/maos-cli/tests/credential_posture_2c.rs` is retired rather than re-pinned.
Executable child-environment and probe regressions cover custody. This ADR's
anti-rot gate now parses the actual plain-command `env_clear` control, so prose,
comments and an unrelated test assertion cannot satisfy the architecture anchor.
The stored-credential escape/non-disclosure tests remain.

The wire-stable `CapabilityToken` remains unchanged: its authorization scope is
registry-side, and no proxy audience or secret is added to that ABI. The dedicated
proxy bearer provides the separate narrowed credential channel.

The Transparency Log append entry point is
`TransparencyLogAdapter::insert_frame_event` at
`crates/maos-iac/src/adapter/transparency_log.rs:612`. Its log-before-deliver
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

## Amendment — Story 17-1 (transport; ratified 2026-10-06)

Ratified by the operator (Lunarpulse) on 2026-10-06, on Story 17-1's creation
record (`_bmad-output/implementation-artifacts/17-1-flag-winston-decision.md`,
D-17-1-FW-04) and Story 17-3a's measured Q6 verdict. Each point below
supersedes the sentence it names; everything else in the Decision stands,
including the order (proxy and bearer, then the T3 route, then `env_clear` and
the guard inversion in the same commit).

- **The transport is a daemon-owned Unix socket, not host loopback.**
  SUPERSEDES *"the Worker's only reachable endpoint is a daemon-side proxy on
  host loopback."* A `--network=none` container cannot reach host loopback by
  construction, and the two mechanisms that can — `slirp4netns`
  `allow_host_loopback=true` (M1) and `pasta` (M2) — were measured to reach the
  internet on ubuntu-24.04 (podman 4.9.3) and ubuntu-26.04 (podman 5.7.0)
  (17-3a Q6, CI runs `36207588609`, `36206431636`). The Worker's only
  reachable endpoint is a daemon-owned Unix socket bind-mounted into the
  container at `/proxy/egress.sock`, served inside the container by a
  forwarder on `127.0.0.1:18080`; `--network=none` is retained (M3).
- **Socket custody.** The socket is mode 0666 inside a daemon-owned 0700
  directory: a 0600 socket is refused to the remapped container uid under the
  rootless user namespace (measured, 17-3a Q6). The directory keeps other uids
  out; it is not a boundary against a same-uid process, so the bearer is the
  authentication, and Story 17-1 proves that a same-uid process without the
  bearer is refused by the proxy.
- **The bearer is daemon-minted.** The Decision never required a kernel mint:
  the per-spawn proxy bearer is minted by the daemon's composition root,
  stored by the proxy only as a hash, and verified at the proxy boundary.
- **Refusals are journaled by a confirmed write.** The proxy journals every
  refusal through `TransparencyLogAdapter::insert_frame_event`
  (`crates/maos-iac/src/adapter/transparency_log.rs:612`) before it returns the
  refusal.
- **The kernel figure is measured, not projected.** SUPERSEDES the Decision's
  *"projected `+65–130` kernel lines … zero local headroom at 18935/18935"*
  paragraph. The ratified ceiling is variant B of the 17-1 measurement (the
  dead `ci_default_guard` is deleted): `maos-kernel-core/src` 25293 → at most
  25353 lines, kloc 19529 → at most 19564, `maos-domain` 9270 → 9274. The
  landing commit re-measures and re-pins the exact figure, which may not
  exceed these. `_aggregate_hardfail` and the `maos-bin`, `maos-egress` and
  `xtask` rows are raised to their exact measured figures in the landing
  commit; the `_aggregate_hardfail` raise is capped at +1182 lines, the top of
  17-1's measured envelope.
- **Landed figures (operator rulings 2026-10-07 to 2026-10-09).** SUPERSEDES
  the previous bullet's `maos-domain` 9274 and its +1182 aggregate cap. The
  operator raised the `maos-domain` ceiling to 9333 (S2 2026-10-07, revised
  2026-10-08) and ratified the per-crate rows and `_aggregate_hardfail` at their
  exact measured landing values (2026-10-09). The kernel ceiling did not move.
  The landing commit pins `maos-kernel-core/src` 25212 lines in 96 files (the
  two dead T3 emitters deleted, D-16) and kloc 19513. It also pins `maos-domain`
  9329, `maos-bin` 25247, `maos-egress` 1716, `xtask` 44370, and
  `_aggregate_hardfail` 173290 against a measured aggregate of 173289.
