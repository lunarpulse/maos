---
Status: ACCEPTED — ratified 2026-09-08 under Story 15-5, decisions F11–F15
Gate: `cargo test -p xtask --test decision_adrs_and_provisioning`
Decided: 2026-09-08
Accepted-in-PR: pending — Story 15-5
Revisits: ADR-028 replay determinism; Story 15-6 inference replay seam; recovery-lane decision D-D
Supersedes: the uncoordinated `--live`, `--replay-llm`, `MAOS_JOURNEY_MODE`, and cassette-presence mode-selection conventions
---

# ADR-064 — One inference mode selector

## Context

MAOS currently has no `MAOS_INFERENCE_MODE` entry in
`crates/maos-bin/src/env_contract.rs`. Mode selection is distributed across a
CLI flag and environment-variable presence.

`crates/maos-bin/src/worker_spawn.rs:54-59` accepts `--live` and accepts
`--replay-llm` as a no-op that sets the same default selected by omitting
`--live`. The spellings `--replay` and `--deterministic` have never existed;
they are not compatibility obligations and are not described as retired.

The Researcher path in `crates/maos-bin/src/main.rs:4633-4680` has three
branches. `--live` wins first. `MAOS_JOURNEY_MODE=record` wraps the live port
and requires `MAOS_REPLAY_CASSETTE`. Otherwise, cassette presence selects
replay, and absence selects the deterministic survey path. Ten existing
`maos run … --once` callers rely on those unset semantics.

`MAOS_REPLAY_STRICT` is orthogonal. It changes replay hash-drift handling; it
does not select record, replay, or live. Three Epic-18 exit commands use it in
that role.

The shared adapter at `crates/maos-bin/src/main.rs:3170` has no cassette branch.
Consequently `MAOS_REPLAY_CASSETTE` is currently inert for `maos shell`, even
though downstream hermetic exit commands name a shell cassette. Supporting
that consumer is new wiring, not a rename.

Provider availability also needs an explicit predicate. When no real provider
is configured, `main.rs:3155-3159` installs `UnconfiguredProvider` under the
`anthropic` identifier, and the router wiring at `main.rs:3157` inserts a
matching default identifier, so identifier presence cannot prove that live
inference is usable.

## Decision

`MAOS_INFERENCE_MODE={record,replay,live}` is the single normative mode
selector. `MAOS_REPLAY_CASSETTE=<path>` supplies the cassette for record and
replay. Story `15-6-inference-replay-seam` implements this contract and cites
this ADR rather than restating the precedence rules.

The compatibility lattice is deliberate:

1. When `MAOS_INFERENCE_MODE` is unset, behavior remains byte-for-byte today's
   selection: `--live` wins; otherwise cassette presence selects replay;
   otherwise the deterministic path remains selected.
2. When `MAOS_INFERENCE_MODE` is set, it is authoritative.
3. `MAOS_INFERENCE_MODE=replay` together with `--live` is a typed configuration
   error with a non-zero exit, never a silent precedence choice.
4. `record` and `replay` require `MAOS_REPLAY_CASSETTE`; absence is a typed
   configuration error.
5. `live` requires a real configured provider. `UnconfiguredProvider` remains
   installed for the unset compatibility path but is explicitly flagged and
   does not satisfy the live predicate.

`MAOS_REPLAY_STRICT` remains a replay modifier and stays `HarnessOnly`.
`MAOS_INFERENCE_MODE` is `UserFacing`. `MAOS_REPLAY_CASSETTE` is promoted from
`HarnessOnly` to `UserFacing` with it because a user-facing mode cannot depend
on a hidden-only required argument.

`--replay-llm` and `MAOS_JOURNEY_MODE` are retired by Story 15-6. No
`--replay` or `--deterministic` retirement is claimed because neither spelling
has existed.

The shell/shared adapter gains cassette handling in Story 15-6. That is an
explicit implementation obligation; this ADR does not pretend the current
Researcher-only branch already covers every inference consumer.

## Consumers

- `15-6-inference-replay-seam` implements the selector, compatibility lattice,
  typed conflicts, stability classes, and shell cassette branch.
- `16-2-shell-halt-registry-and-j0-scene` consumes replay for the J0 shell.
- `18-1-butler-inference-seam-and-conformant-mcp` and the rest of Epic 18 use
  the selector for hermetic cognition paths.
- `19-1-orchestrator-dispatch-loop-and-epic-input` consumes it for the founder
  loop.
- `20-4-gemini-driver-and-endpoint-pin` and
  `21-1-j4-incident-fixture-and-live-mira-nash` consume the same contract.

## Rationale

D-D earns an ADR because the decision must survive the epic that first
implements it. It has five downstream epic consumers and defines a user-facing
environment contract. Recovery-lane decisions D-F, D-H, and D-I each have one
consumer and can remain scoped to their owning epic; this one cannot without
creating five drifting copies.

Preserving unset behavior avoids changing all existing unconfigured and
fixture runs in a documentation story. Making an explicitly set mode
authoritative gives new callers one testable contract. Rejecting contradictory
explicit inputs is safer than guessing which operator intent should win.

## Consequences

- `env_contract.rs` gains one user-facing variable and promotes the cassette
  variable when Story 15-6 builds the decision.
- The Researcher, shared shell, Butler, Orchestrator, Mira, and Nash paths use
  one mode vocabulary.
- Existing unset callers retain their current exit behavior.
- Live mode can no longer mistake `UnconfiguredProvider` for a real provider.
- `MAOS_REPLAY_STRICT` remains compatible with the three Epic-18 exit lines.
- ADR-028's trace-shape determinism remains unchanged; this ADR governs mode
  selection, not cassette schema or replay content.
