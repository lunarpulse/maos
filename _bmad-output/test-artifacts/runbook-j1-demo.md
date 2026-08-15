# Runbook — `xtask demo-j1` (the J1 founder-loop scene)

> **Purpose.** Watch J1 run on your own machine in one command, and read an honest
> claim table of what it does and does not prove. The free take costs nothing and
> touches no network; the signed take is the Tier-2 gate procedure automated.
>
> **Audience.** Anyone. The free take needs no keys, no Postgres, no Docker.

---

## The free take (default — no cost, no network, no keys)

```
cargo run -p xtask -- demo-j1
```

That is the whole thing. It builds the workspace, provisions a throwaway state
home, runs the founder loop once, calls the Blocking delegation gate as the judge,
and prints the claim table.

**Flags**

| Flag | Why |
|---|---|
| `--skip-build` | Reuse `target/debug` binaries (fast re-runs). |
| `--keep-home <DIR>` | Keep the state home so you can open the transparency log. |
| `--skip-gate` | Narrate the scene without running the delegation gate. |
| `--live-codex` | The paid Tier-2 take — see below. |
| `--codex-topology <PATH>` | The operator-authored codex topology for `--live-codex`. |

**Expected shape** (2026-08-14, post-`j1-crosshost-1a`): 14 events, exit 0, founder
loop ≈**0.19s**, eight executed beats `PROVEN_BLOCKING`, four `ABSENT`.

---

## What the beats mean

| Beat | Reads | Meaning |
|---|---|---|
| `topology-spirits-loaded` | PROVEN_BLOCKING | orchestrator + architect + reviewer came up from the topology. |
| `delegation-frame-crosses-loopback` | PROVEN_BLOCKING | a real `task.assign` carried an ADR-012 consent envelope to `developer-remote`. **v0.8 rung — loopback rehearsal.** |
| `worker-admitted-under-host-grant` | PROVEN_BLOCKING | host-managed grant, T3, real `child_pid`. Admission by grant, not by trust. |
| `worker-completed-by-adapter-oracle` | PROVEN_BLOCKING | completion came from `parse_completion` + a worker TL ref — never from an exit code. |
| `delegation-closed-at-safe-point` | PROVEN_BLOCKING | `TaskComplete` journaled, no frame left in flight. |
| `state-home-clean` | PROVEN_BLOCKING | zero `journal:` warnings in a fresh home. |
| `audit-drain-clean` | PROVEN_BLOCKING | every queued audit row reached SQLite before exit. |
| `frame-borne-route-intact` | PROVEN_BLOCKING | `check-j1-loopback-delegation` (hermetic, Blocking) agrees. |
| `disallowed-intent-refused-blocking` | **ABSENT** | owned by `j1-crosshost-1b`. |
| `tier2-live-agent-signed` | **ABSENT** | earned only by `--live-codex`. |
| `two-host-signed-run` | **ABSENT** | owned by `j1-crosshost-2` (v1.0). |
| `halt-resume-referential-identity` | **ABSENT** | owned by `FOLLOWUP-J1-RESUME-SEAM`. |

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
- **Halt/resume**: safe shutdown with no in-flight delegation is proven; the
  post-resume digest citing the exact pre-halt ref is not.

---

## The signed take (`--live-codex`) — real money, real signature

This automates Phases 3–5 of
[`runbook-j1-tier-2-signed-live-run.md`](runbook-j1-tier-2-signed-live-run.md).
Read that first for the abort conditions; they still bind.

**You must supply the codex profile.** The repo ships only the fixture worker
manifest, so author the codex topology + manifest per that runbook's Phase 1.5
(pin the exact argv first), then pass it:

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
| `spirits/topologies/j1-founder-loop.toml not found` | Run from the repository root. |
| `audit-drain-clean` reads FAIL | A drain regression: queued rows may be lost, so a sealed export over that window could sign an incomplete bundle. Do not sign; see the story's AC4 and `crates/maos-bin/tests/drain_once_audit_writer.rs`. |
| `state-home-clean` reads FAIL | The run itself produced a journal warning — a real finding, not ambient noise. |
| Worker hangs at "Reading additional input from stdin…" | Stale `maos` binary predating the stdin-EOF fix. Rebuild. |
