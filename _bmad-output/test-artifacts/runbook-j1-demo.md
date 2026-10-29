# Runbook — `xtask demo-j1` (the J1 founder-loop scene)

> **Purpose.** Watch J1 run on your own machine in one command, and read an honest
> claim table of what it does and does not prove. The free take costs nothing and
> touches no network; the signed take is the Tier-2 gate procedure automated.
>
> **Audience.** Anyone. The free take needs no keys, no Postgres, no Docker.

---

## The free take (default — no cost, no network, no keys)

```
MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j1/founder-loop.json cargo run -p xtask -- demo-j1
```

That is the whole thing (the Epic-19 exit line; CI's `epic-19-exit` job adds
`MAOS_REPLAY_STRICT=1`). It builds the workspace, provisions a throwaway state
home, `maos init`s it (the operator door), and runs the founder-loop EPIC once
(`spirits/topologies/j1-founder-loop-epic.toml --epic
tests/fixtures/j1/founder-loop-epic.md`, Story 19-4): the Orchestrator splits three
sections and judges each before its frame from the replayed cassette; the kernel
halts the ambiguous section 2; the demo answers the halt over the door
(`maosctl halt resolve … --kind provided-context`) while the child's stdout streams;
section 2 resumes with the answer; the run ends with an FR17 digest. Then it calls
the Blocking delegation gate as the judge and prints the claim table. The env pair
is the ONE inference selector (ADR-064, no flag): without it the Orchestrator judges
nothing and the halt and resume beats FAIL. A T3 substrate is required
(`scripts/provision_t3_tests.py`, then source its `t3.env`).

**Flags**

| Flag | Why |
|---|---|
| `--skip-build` | Reuse `target/debug` binaries (fast re-runs). |
| `--keep-home <DIR>` | Keep the state home so you can open the transparency log. |
| `--skip-gate` | Narrate the scene without running the delegation gate. |
| `--live-codex` | The paid Tier-2 take — see below. |
| `--codex-topology <PATH>` | The operator-authored codex topology for `--live-codex`. |

**Expected shape:** exit 0 with every EXECUTED beat `PROVEN_BLOCKING` and exactly one
`ABSENT` beat (`tier2-live-agent-signed`); `two-host-signed-run` renders
`INDETERMINATE` while a two-host capture is committed. The beat count changed with
Story 19-4's epic scene and has not been re-measured here (the last count, 22 beats
at `dd4cf959`, was the one-Worker scene). **Read the table below, not a remembered
count.**

---

## What the beats mean

| Beat | Reads | Meaning |
|---|---|---|
| `topology-spirits-loaded` | PROVEN_BLOCKING | orchestrator + architect + reviewer came up from the topology. |
| `delegation-frame-crosses-loopback` | PROVEN_BLOCKING | every section's `task.assign` (three) carried an ADR-012 consent envelope to `developer-remote`. **v0.8 rung — loopback rehearsal.** |
| `worker-admitted-under-host-grant` | PROVEN_BLOCKING | three host-managed grants, T3, three distinct real `child_pid`s. Admission by grant, not by trust. |
| `worker-completed-by-adapter-oracle` | PROVEN_BLOCKING | each section's completion came from the adapter's own structured output — never from an exit code. |
| `delegation-closed-at-safe-point` | PROVEN_BLOCKING | `TaskComplete` journaled for each section, no frame left in flight. |
| `worker-exited-and-loop-went-idle` | PROVEN_BLOCKING | every child reaped without a crash and every class Spirit fired `on_idle`. |
| `state-home-clean` | PROVEN_BLOCKING | zero `journal:` warnings in a fresh home. |
| `lifecycle-stages-in-order` | PROVEN_BLOCKING | per section: routed → loaded → completion → closed, each once, sections in order, then the drain once. |
| `audit-drain-clean` | PROVEN_BLOCKING | every queued audit row reached SQLite before exit. |
| `sealed-export-covers-the-run` | PROVEN_BLOCKING | what an independent reader sees in the window is what the signer sealed (rows queried ≤ rows covered). |
| `frame-borne-route-intact` | PROVEN_BLOCKING | `check-j1-loopback-delegation` (hermetic, Blocking) agrees. |
| `loopback-from-host-unverified` | PROVEN_BLOCKING | the wire-identity boundary is exactly where rung 1 says it is. |
| `completion-oracle-per-adapter` | PROVEN_BLOCKING | each adapter reads its OWN structured output; codex and claude are not interchangeable. |
| `worker-cli-under-library` | PROVEN_BLOCKING | the adapter seam stays nameable by its vectors. |
| `completion-vectors-enrolled` | PROVEN_BLOCKING | every J1 test target is actually invoked by CI. |
| `consent-refusal-proofs` | PROVEN_BLOCKING | `-32001` / `-32009` / `-32003` stay distinct and asserted. |
| `cross-host-identity-proof` | PROVEN_BLOCKING | the crossing is proven in two logs under a verified wire identity. |
| `disallowed-intent-refused-blocking` | PROVEN_BLOCKING | a disallowed intent is REFUSED (`-32001`, distinct from `-32009`). **Landed by `j1-crosshost-1b`; this row read `ABSENT` until 2026-08-22.** |
| `two-host-delegation` | PROVEN_BLOCKING | two real hosts over mTLS/TOFU, a frame crossed, a worker ran on the far side, both logs carry the same sixteen bytes. |
| `tier2-live-agent-signed` | **ABSENT** | earned only by `--live-codex` (operator-local, never CI). |
| `two-host-signed-run` | **INDETERMINATE** (`--`) / **ABSENT** without a capture | owned by **`j1-crosshost-2d-paid-two-host-run`**. A committed capture renders `INDETERMINATE` — validated, never claimed; with no capture it is `ABSENT`. |
| `epic-decomposed-n-frames` | PROVEN_BLOCKING | the epic split into three frames, one per `### Story` section, dispatched and closed in section order (Story 19-4). |
| `ambiguity-halt-within-2-frames` | PROVEN_BLOCKING | the Orchestrator judged section 2 from the replayed cassette (`dispatch_ambiguity` > 0.5) and the KERNEL halted it before its frame. |
| `digest-cites-source-refs` | PROVEN_BLOCKING | the FR17 digest at drain cites ≥ 3 completions and the halt, each ref resolving to a row of `maosctl audit query`. |
| `halt-resume-referential-identity` | PROVEN_BLOCKING | the halt was answered over the door, section 2 resumed with the answer, and the post-resume digest cites the exact pre-halt frame under `resolved_halts`. **Landed by Story 19-4; this row read `ABSENT` (`FOLLOWUP-J1-RESUME-SEAM`) until then.** |

`ABSENT` never becomes green and never fails the run — it is a visible placeholder,
not a silent skip. **Read the table, never the exit code:** the scene exits 0 with
ABSENT beats outstanding, because that is J1's honest state today.

---

## What this scene does NOT prove

- **Not cross-host.** `developer-remote` is a peer id on *this* host. No packet
  leaves the machine. Two real hosts over mTLS/TOFU is `j1-crosshost-2`.
- **Not peer-authenticated.** On loopback, `frame.from.host_id` is self-asserted —
  the frame effectively picks which allowlist judges it. Rung 2 binds it to a
  TLS-verified identity; 1a records this as a boundary leg so the flip shows up in
  a CI diff.
- **Not capability-mediated.** The `cli_wrapper` token path proceeds under
  host-grant authority. Kernel `proc.exec` mediation is an Epic-9 operator-policy
  surface, and a Cedar permit alone cannot green it (the wrapper path never
  registers the manifest with `SecurityManager`). The `CapabilityInvocation` exit
  row IS journaled either way.
- **Egress is `declared-not-enforced`** (`FOLLOWUP-EPIC14-V2.0-PACKET-EGRESS-ENFORCEMENT`).
- **Halt/resume is in-boot only** (Story 19-4 D-19-4-05 A): the halt is answered
  over the door while the same process holds the section. A halt still pending
  when the process exits is not resumed by a later boot.

---

## The signed take (`--live-codex`) — real money, real signature

This automates Phases 3–5 of
[`runbook-j1-tier-2-signed-live-run.md`](runbook-j1-tier-2-signed-live-run.md).
Read that first for the abort conditions; they still bind.

**The codex profile now SHIPS — author nothing.** All three files exist at HEAD and
are the ones the flag expects (verified 2026-08-22): the topology
`spirits/topologies/j1-founder-loop-codex.toml`, the worker manifest
`spirits/worker/manifest-codex.toml`, and its claude sibling
`spirits/worker/manifest-claude.toml`. This paragraph previously told the operator to
author all three per Phase 1.5 — work that `j1-crosshost-2a` had already landed.
You still **must** pin the exact argv standalone before a signed run, because
`argv_prefix` is TOCTOU-hashed into the cap-token; that part of the instruction stands.
Pass the shipped topology:

```
export CODEX_API_KEY="$OPENAI_API_KEY"      # codex IGNORES OPENAI_API_KEY for auth
export MAOS_HOST_GRANTS=~/.maos/host-grants.toml
export MAOS_DEMO_J1_SIGNER="Your Name"
export MAOS_DEMO_J1_SIGNER_KEY=~/.maos/keys/j1-tier2-signer.key
cargo run -p xtask -- demo-j1 --live-codex --codex-topology <path>
```

It **refuses to start** — rather than downgrading to a fixture take — when:
`~/.codex/auth.json` exists (an unattestable subscription token must never enter a
signed run), `CODEX_API_KEY` is unset, the grants file is missing, the signer or
audit key is unset, or the codex topology does not exist.

On success it writes the capture (non-secret fields only), journals it with
`maosctl audit record-capture`, seals the window with `sealed-export --range`,
verifies with `verify-bundle`, and flips `tier2-live-agent-signed` to
`PROVEN_LIVE_SIGNED`. A fixture run can never claim that beat.

---

## Where things land

- **State home**: ephemeral temp dir, removed on exit. Pass `--keep-home <DIR>` to
  keep the transparency log, journal, and (on a signed take) the capture and
  sealed bundle.
- Both `MAOS_HOME` and `XDG_DATA_HOME` point at that home, so the daemon, the
  journal, the transparency log, and `maosctl` all resolve to the same fresh tree.
  Your real `~/.local/share/maos` is never touched or read.

## Troubleshooting

| Symptom | Cause |
|---|---|
| `maos not found at target/debug/maos` | You passed `--skip-build` without building. Drop it. |
| `spirits/topologies/j1-founder-loop-epic.toml not found` (or `tests/fixtures/j1/founder-loop-epic.md`) | Run from the repository root. |
| `ambiguity-halt-within-2-frames` reads FAIL with "no dispatch_judged" (and the resume beat with it) | `MAOS_INFERENCE_MODE` / `MAOS_REPLAY_CASSETTE` were not set: the Orchestrator judges nothing without them (ADR-064). Use the command at the top. |
| `halt-resume-referential-identity` reads FAIL with "resumed false" | The door answer did not land (look for `resolve FAILED` in the streamed output); the hold expired into the 19-1 stop. |
| `audit-drain-clean` reads FAIL | A drain regression: queued rows may be lost, so a sealed export over that window could sign an incomplete bundle. Do not sign; see the story's AC4 and `crates/maos-bin/tests/drain_once_audit_writer.rs`. |
| `state-home-clean` reads FAIL | The run itself produced a journal warning — a real finding, not ambient noise. |
| Worker hangs at "Reading additional input from stdin…" | Stale `maos` binary predating the stdin-EOF fix. Rebuild. |
