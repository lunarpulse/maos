---
Status: ACCEPTED — ratified 2026-09-08 under Story 15-5, decisions F3–F5; §D-C transport AMENDED 2026-09-26 (operator-ratified — see Amendment)
Gate: `cargo test -p xtask --test decision_adrs_and_provisioning`; implementation gates owned by Story 17-3b
Decided: 2026-09-08
Accepted-in-PR: pending — Story 15-5
Revisits: ADR-040; ADR-002; architecture §13.1 in `13-phased-roadmap.md`; FR5 and FR33
Supersedes: ADR-031 rust-inproc deferral clauses in §§4, Alternatives, and Consequences (source lines 54, 63, and 71); ADR-031's WASM subprocess and equivalence decisions remain binding
---

# ADR-060 — Spirit forms by trust tier

## Context

MAOS has two distinct form taxonomies today.

The manifest taxonomy is the string list `ClassSection.forms` in
`crates/maos-manifest/src/manifest.rs`. Since Story 17-3b its validator accepts
`rust-inproc`, `subprocess` and `wasm-component`; `wasm-component` must be the
only form and requires `[class].artifact` and manifest schema 5. The shipped
negative test still uses `wasm` as the unknown value.

The host-port taxonomy is `maos_host::SpiritForm` at
`crates/maos-host/src/lib.rs:48`. It contains `NativeSubprocess` and
`WasmComponent`. It describes the executable host adapter, not manifest trust
admission. There is no host enum value corresponding to `rust-inproc`.

`[cli_wrapper]` is a third manifest shape, not a form string. It is mutually
exclusive with `[class]`; combining the two is
`EManifestSchemaConflict`. It therefore cannot carry `class.forms` and must not
be listed as one of its values. Its own admission control requires T3 through
`ECliWrapperRequiresT3`.

Form-based admission exists since Story 17-3b.
`maos_registry::admit_spirit` and `admit_spirit_with_attestation` in
`crates/maos-registry/src/admission.rs` parse the manifest once as a
`ManifestDocument`; its `class_section` method reads `class.forms` after the
same frozen-surface gate both call first: a package declaring `rust-inproc` is
refused (`FirstPartyFormFromRegistry`) at every trust tier, attested or not. The daemon's
own gate (`maos_bin::admission::gate_manifest`) refuses a `wasm-component`
manifest by name until the engine and launch ship. `SecurityPolicy::admit` at
`crates/maos-kernel-core/src/security/mod.rs` does not read `class.forms`.

First-party in-process loading does exist by construction. The scheduler's
`load<T: Spirit>` at `scheduler_loop.rs:243` receives a compiled-in Rust type.
No child process exists to sandbox, and `spawn_sandboxed` has no `maos-bin`
callers for that path.

The corpus carries conflicting rust-inproc dispositions. ADR-002 selected one
subprocess form at v0.1. ADR-040 deferred rust-inproc. ADR-031 added the WASM
component subprocess and states that rust-inproc remains deferred. The phased
roadmap later calls rust-inproc retired, while the product-scoping delta calls
it the first-party form. This ADR resolves that chain explicitly.

“No subprocess Spirit host was built” refers to the native Spirit Wire
Protocol host planned at v0.1. It does not deny ADR-031's wasmtime runner
process. The runner at `crates/maos-wasm-host/src/runner.rs:1-5` is
`BridgeSpawnSpec.program` and speaks the unchanged ADR-032 protocol over stdio.

The WIT world at `wit/spirit.wit` (`maos:spirit@2.0.0` since Story 17-3b) has
one `use`, three exports, and no imports. There is no Unix listener, stream, or
datagram implementation in the workspace, and no WASM recall path exists yet.

`Butler::morning_digest` exists at `spirits/butler/src/lib.rs:675` and Butler is
already linked by `maos-bin`. It has four test/benchmark callers and zero
production callers.

## Decision

The manifest `class.forms` trust policy is:

1. `rust-inproc` is the first-party form. It is kernel-distribution code,
   compiled into the daemon, and in-process by construction. Registry
   admission must refuse any package declaring it.
2. `wasm-component` is the third-party and polyglot form token. Story 17-3b
   adds it to manifest validation and admits it at the appropriate registry
   trust tiers. The shorter token `wasm` remains invalid, preserving the
   existing negative vector.
3. `subprocess` remains a manifest token only where existing compatibility
   requires it; it is not the first-party distribution mechanism.
4. `[cli_wrapper]` remains an alternative manifest shape, mutually exclusive
   with `[class]`, and is governed by its T3 admission control rather than by
   `class.forms`.

This ADR governs the manifest string taxonomy. The `maos-host::SpiritForm` enum
continues to select native-subprocess versus WASM-component host adapters. The
two representations may be mapped by the composition root, but they are not
silently treated as the same schema.

Story `17-3b-wasm-form-admission-and-contract` owns the validator and
registry changes. `RawClassSection` is `deny_unknown_fields`; adding the token
has schema-version and field-coverage consequences that land with that story.

ADR-031's WASM-component runner, T2 process boundary, unchanged ADR-032 wire,
and cross-form equivalence remain binding. Only its statements that
rust-inproc remains deferred are superseded. ADR-002's process-isolation
rationale remains applicable to third-party code; its single-form conclusion
does not govern trusted compiled-in first-party classes. ADR-040's measurement
history remains evidence, not the current disposition.

### D-C — WASM recall transport

WASM recall is built as a `maos-bin` side-channel service over a Unix socket.
The daemon chooses and passes the socket path to the runner. The runner's stdio
continues to be the ADR-032 kernel bridge and remains byte-identical. Recall
does not transit a new kernel protocol, so kernel-core delta is zero.

This is a to-build decision. Unix socket primitives and a WASM recall path are
absent today. Story 17-3a prices and validates the design before Story 17-3d
builds it (epic-17 R17-51 split the old 17-3b). The WIT world gains only the versioned capability surface selected
by that spike; this ADR does not claim an existing import.

### D-E — digest rendering

The daemon renders FR17 digests by calling `Butler::morning_digest` directly
over the run's audit window. Butler need not join the active topology merely to
format the digest. The call becomes a production call in the owning founder-
loop story; current test and benchmark callers are not represented as live
wiring.

FR5's operator-configured strictest-of-three mechanism is amended to be
form-aware by this policy. FR33's per-language templates, including the Go
v1.5+ commitment, are amended rather than silently dropped by the WASM
component toolchain.

## Consumers

- `17-3b-wasm-form-admission-and-contract` implements `wasm-component`
  validation, registry admission by form/trust tier, and refusal of registry
  `rust-inproc` packages.
- `17-3c-wasm-spirit-on-the-bus-under-t2` runs a `wasm-component` Spirit on the
  IAC bus under ADR-031's T2 process boundary.
- `17-3d-wasm-log-recall` builds the D-C recall side channel.
- `17-3a-wasm-recall-and-componentize-spike` validates the D-C side channel.
- `19-4-j1-beats-and-demo-replay` supplies the D-E production digest caller.
- `20-1-registry-client-install-verb-vetter-and-yank` relies on the registry
  form policy for install admission.

## Rationale

Trust is the distinction the earlier form decisions lacked. First-party code
can be compiled into the daemon without creating a public package admission
path. Third-party/polyglot code needs a constrained component host and explicit
registry policy. Agent CLIs remain a separate shape because their command and
credential contract is not a Spirit form declaration.

Naming `wasm-component` aligns manifest intent with the host enum while keeping
`wasm` available as a regression vector. Explicitly separating present facts
from Story 17 work prevents this ADR from claiming controls that do not exist.

## Consequences

- The architecture has three hosting shapes but only `[class].forms` is governed
  by the form-to-trust mapping in this ADR.
- Registry admission gains a new typed refusal for `rust-inproc` packages in
  Story 17-3b.
- WASM recall uses an out-of-kernel side channel; ADR-032 stdio is unchanged.
- The daemon owns digest rendering without adding Butler to every topology.
- The phased roadmap and product-scoping text are reconciled with the explicit
  ADR-002/031/040 supersession chain.
- Kernel-core source remains unchanged by this decision story.

## Implementation note — Story 17-3b (2026-09-29)

Recorded, not a change to clauses 1–4. Story
`17-3b-wasm-form-admission-and-contract` implemented clauses 1 and 2 as
follows. "The appropriate registry trust tiers" in clause 2 is **every** tier:
`local`, `org-internal`, `public-untrusted` and an attested `public-vetted`
each admit a `wasm-component` package under that tier's existing signature,
org-key and envelope obligations (epic-17 R17-58). FR5's strictest-of-three
floor is form-aware: a `wasm-component` package's returned sandbox floor is T2
at every tier, because its only boundary is ADR-031's T2 runner process plus
the WIT surface, and `t3_for_public_untrusted` does not apply to it (T3 has no
WASM path; T4 is the reserved WASM tool sandbox, refused at admission). A
package without a `[class]` table keeps its legacy admission unchanged. The
daemon refuses a `wasm-component` manifest as `wasm_engine_off` in every
default build and as `wasm_launch_not_built` in a `wasm-host` build until
`17-3c-wasm-spirit-on-the-bus-under-t2` ships the launch.

> **Amendment (2026-10-06, 17-3c):** `wasm_launch_not_built` no longer exists.
> The default/published build still refuses `wasm_engine_off`; a `wasm-host`
> build launches the component in the contained T2 runner via `maos run`, and
> refuses unsupported in-process surfaces (operator door, topology, hot-swap
> successor) as `spawned_surface_unsupported`.

## Amendment — §D-C transport (ratified 2026-09-26)

Ratified by the operator (Lunarpulse) on 2026-09-26, on Story 17-3a's measured
proposal (Go/No-Go Q4 row and ratification checklist) as re-shaped by Story
17-6's preflight round-table (epic-17 §R6, R17-46). Measurement disproved two
statements in §D-C above; each point below supersedes the sentence it names.
Everything else in §D-C stands: recall is a `maos-bin` side channel, the
runner's stdio stays the byte-identical ADR-032 kernel bridge, and recall does
not transit a new kernel protocol.

- **The transport is an inherited descriptor, not a socket path.** SUPERSEDES
  *"The daemon chooses and passes the socket path to the runner."* The daemon
  creates a connected `socketpair`, keeps one end, and hands the other to the
  runner at spawn as an inherited descriptor. Measured (17-3a Q4b, CI run
  `36210072648`, both x64 images): under the T2 allow-list — which carries no
  `socket` and no `connect`, and must not, because a sandbox with them reached
  unrelated AF_UNIX and loopback TCP listeners — a path-based recall is denied
  while five inherited-descriptor variants complete. The runner needs only
  `sendto` and `recvfrom` on the descriptor it was given.
- **Condition (i) — same-uid safety rests on host ptrace protection.** A process
  at the operator's uid can take the inherited descriptor through
  `pidfd_getfd` or `/proc/<pid>/fd` wherever Yama `ptrace_scope` is `0`; with
  `ptrace_scope=1` both are refused (17-3a `q4-fd-vectors.log`). The recall
  service binds the caller's pid per spawn (epic-17 R17-23) rather than treating
  possession of the descriptor as identity, and the Yama dependency is a stated
  deployment condition, not a guarantee this design provides.
- **Condition (ii) — availability must not rest on a one-shot path accept.** A
  path socket accepted once and unlinked is denial-of-service-able by the first
  same-uid connector, because the unlink precedes the `SO_PEERCRED` check (17-3a
  `recall_service.rs.txt:93-98`). Any path-socket fallback keeps `SO_PEERCRED`,
  a 0600 socket in a daemon-owned 0700 directory, and a retry path; the
  descriptor transport above is the design.
- **Kernel-core is not unchanged.** SUPERSEDES *"so kernel-core delta is zero"*
  and the Outcomes bullet *"Kernel-core source remains unchanged by this
  decision story"* as it applies to building D-C. The inherited descriptor's
  `FD_CLOEXEC` must be cleared inside the child before `execve`, which only a
  `pre_exec` hook can do; that is `unsafe` and belongs to the sandbox zone
  (`security/sandbox/`), and `lifecycle/cli_wrapper/runtime.rs` forbids
  `unsafe`. The runner's own allow-list entries (`memfd_create`, `sendto`,
  `recvfrom`) also land there. Stories `17-3c-wasm-spirit-on-the-bus-under-t2`
  (`memfd_create`) and `17-3d-wasm-log-recall` (the descriptor hand-off,
  `sendto`, `recvfrom`) are therefore ◆ FLAG-Winston for exactly those lines,
  each with its kernel figure measured by a prototype at its creation (E16-A7;
  epic-17 R17-46, routed by R17-51 when the old 17-3b was split).
