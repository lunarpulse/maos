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

Story 15-6 implements the accepted selector. `MAOS_INFERENCE_MODE` is now a
user-facing entry in `crates/maos-bin/src/env_contract.rs`, and
`MAOS_REPLAY_CASSETTE` supplies the record/replay payload path. Mode selection
is resolved once in the `maos-bin` composition root, before any inference
consumer can execute.

`crates/maos-bin/src/worker_spawn.rs` still accepts `--live`.
`--replay-llm` is retired. Before retirement it was not a true no-op: its
parser arm assigned `live = false`, so its result was order-sensitive when
combined with `--live`. The spellings `--replay` and `--deterministic` have
never existed; they are not compatibility obligations and are not described
as retired.

Unset mode preserves the former Researcher selection: `--live` wins;
otherwise cassette presence selects replay; otherwise the deterministic
survey path is selected. A measured inventory found 35 `maos run … --once`
invocation sites across 18 files, including eight that load the Researcher;
none explicitly set `MAOS_REPLAY_CASSETTE`.

`MAOS_REPLAY_STRICT` is orthogonal. It changes replay hash-drift handling; it
does not select record, replay, or live. Three Epic-18 exit commands use it in
that role.

The router-level cassette provider seam is in the composition root before
`MultiProviderRouter::new`: replay and record replace or wrap every existing
provider value through `providers_map.values_mut()` while preserving the key
set and default identifier. The replay arm constructs
`cassette_replay::CassetteReplayProvider::from_file(cassette, *strict)` and
the record arm `cassette_replay::CassetteRecordProvider::new(…)`, and each
arm is anti-rot-anchored independently. The resulting router remains behind
`InferencePortAdapter`, so capability checks, attribution, IAC emission, and
Transparency-Log recording still mediate completions.

Provider availability is tracked separately from identifier presence. A
keyless boot can register the guessed-default Ollama endpoint without an
operator configuration, and an empty provider map still installs
`UnconfiguredProvider`; both fallbacks are explicitly flagged and do not
satisfy the live predicate. The `ReplayLiveConflict` refusal protects the
explicit replay-plus-`--live` contradiction.

Cassettes retain schema version `maos.journey.cassette/v1`. Runtime replay is
compatible with older v1 files that lack provenance and rejects a present
illegal value. The checked-in corpus separately requires
`provenance ∈ {seed, live-record}`, with a non-empty denominator enforced by
CI; record output uses `live-record`.

## Decision

`MAOS_INFERENCE_MODE={record,replay,live}` is the single normative mode
selector. `MAOS_REPLAY_CASSETTE=<path>` supplies the cassette for record and
replay. Story `15-6-inference-replay-seam` implements this contract and cites
this ADR rather than restating the precedence rules.

The compatibility lattice is deliberate:

1. When `MAOS_INFERENCE_MODE` is unset, behavior preserves the prior
   selection: `--live` wins; otherwise cassette presence selects replay;
   otherwise the deterministic path remains selected.
2. When `MAOS_INFERENCE_MODE` is set, it is authoritative.
3. `MAOS_INFERENCE_MODE=replay` together with `--live` is a typed configuration
   error with a non-zero exit, never a silent precedence choice.
4. `record` and `replay` require `MAOS_REPLAY_CASSETTE`; absence is a typed
   configuration error.
5. `live` requires a real configured provider. `UnconfiguredProvider` and the
   guessed-default Ollama endpoint remain available for unset compatibility,
   but are explicitly flagged and do not satisfy the live predicate.

The provider-value seam identified in Context is normative. Record and replay
replace or wrap router provider values before adapter construction; they do
not bypass `InferencePortAdapter`, change provider keys, or change the default
identifier.

The record wrapper's `credential_fingerprint()` hashes the cassette path by
ruling (15-6 §A6 review, decision 3): record-mode rate-limit identity and
`RateLimited` telemetry are intentionally keyed by cassette, not by
credential bytes.

Runtime replay accepts an absent provenance field for v1 compatibility and
rejects any present value outside `seed|live-record`. Every checked-in cassette
in the journey corpus must carry one of those two values, with the corpus
denominator checked before validation. The recorder writes `live-record`;
hand-authored fixtures use `seed`.

`MAOS_REPLAY_STRICT` remains a replay modifier and stays `HarnessOnly`.
`MAOS_INFERENCE_MODE` and `MAOS_REPLAY_CASSETTE` are `UserFacing`.

`--replay-llm` and `MAOS_JOURNEY_MODE` are retired by Story 15-6. No
`--replay` or `--deterministic` retirement is claimed because neither spelling
has existed.

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
