# Epic 16 — W1: One Daemon, One Door (J0 for real)

**Status:** `backlog` — created 2026-09-04 by `sprint-change-proposal-2026-09-04.md` (bmad-correct-course, Lunarpulse-ratified). One of six recovery epics (15–20); `sprint-status.yaml` is authoritative for status.

**Wave / duration:** 2–3 weeks · **Makes true:** v0.1 · J0 evaluator

**Closes:** S1 (control plane never reaches the daemon), S2 (sandbox/cgroups never applied), S5 (uninstall, secrets); FR2, FR4, FR5, FR6, FR9, FR12, FR13, FR15, FR16, FR20, FR24, FR47 (Worker), FR50, FR51, FR58

**Exit command (the epic is done when an operator runs this on a clean machine and sees what it says):**

```
cargo install maos && maos init && maos shell
@hello-spirit refactor src/main.rs to be more idiomatic      # halts on task.acceptance_criterion.ambiguous; resolve; frames logged
maosctl pause hello-spirit                                     # the RUNNING Spirit stops (daemon journal + scheduler state)
maos audit query --spirit hello-spirit && maos uninstall       # clean host afterwards
```

**Kernel-Δ:** **AUTHORIZED non-zero kernel-Δ** — the first since 13.5j. Baseline at open: kernel-Δ @24472. The grant is measured at landing per the T0 precedent (a grant lands WITH the lines it authorizes), FLAG-Winston on the pin and on `kloc.toml`. This epic holds the shared-write lock on `crates/maos-bin/src/main.rs` first (J1-lane T0 rule); parallel sessions coordinate through it.

**Dev-gate:** external holds (pen-test NFR-Sec-7 + export counsel NFR-Comp-1) = GA ledger only, non-gating for dev. **Model/review discipline:** frontier-class dev allowlist + §A6 full-layer review net is the binding control (E11 retro A1). **Story rules (CP-2):** AC1 of every story is a command plus what the operator observes; a story splits once; story files ≤3,000 words; governance work ≤20% of the epic.

---

## Stories

| Key | Story | Closes |
|---|---|---|
| `16-1-daemon-rpc-control-plane` | Daemon RPC control plane | S1; FR9/13/15/16/20/24/51 |
| `16-2-apply-sandbox-and-resource-caps` | Apply the sandbox and resource caps | S2; FR4/5/6/47 |
| `16-3-subprocess-crash-to-handle-crash` | Subprocess crash reaches the crash handler | FR12/FR50; NFR-Rel-1/2 |
| `16-4-kernel-uninstall-and-keyring-secrets` | Kernel uninstall and keyring secrets | FR2/FR65; NFR-Sec-16 |
| `16-5-j0-halt-on-ambiguity-scene` | J0 halt-on-ambiguity scene | J0; FR15/FR58; NFR-Onb-2 |
| `16-6-audit-drop-legal-hold-and-a2a-deny-vocabulary` | Debt slot: audit drop, legal hold, A2A deny vocabulary | D3, D5, D7, D18 — repaired out-of-kernel where 14-0 ruled; FLAG-Winston otherwise |

Per-story ACs below are sketches; `bmad-create-story` finalizes them at preflight (≤6 ACs, AC1 unchanged in kind).

### 16-1-daemon-rpc-control-plane — Daemon RPC control plane

*Closes:* S1; FR9/13/15/16/20/24/51

- **AC1 (command).** With `maos run spirits/hello-spirit/manifest.toml` running, `maosctl pause hello-spirit` stops that Spirit: the daemon journal records the pause and `maosctl spirit inspect` shows the scheduler state changed; `maosctl posture shift --spirit hello-spirit cautious` changes the next capability decision of the running Spirit.
- **AC2.** All 18 `MAOS_ONE_SHOT` verbs in `maos-cli/src/subcommands.rs` are re-targeted to an RPC over the authenticated operator socket the `maos-control` server already binds; one-shot survives only for `init`, `backup`, `import`.
- **AC3.** `maosctl revoke-token` mutates the daemon-held `CapTokensShardRing`; `maosctl revocations import` reaches running Spirits (`matched > 0`).
- **AC4.** `maosctl orchestrator queue` enqueues into the daemon's `OrchestratorBufferRegistry` and the instruction survives until a safe point (FR20).
- **AC5.** Proven-red: with the daemon stopped, every verb fails loud (typed error, non-zero exit) instead of journaling and exiting 0.

### 16-2-apply-sandbox-and-resource-caps — Apply the sandbox and resource caps

*Closes:* S2; FR4/5/6/47

- **AC1 (command).** A T2 Spirit that opens `~/.ssh/id_ed25519` gets `EACCES`, and `maos audit query` shows the denial as a capability event.
- **AC2.** `spawn_sandboxed` is the only spawn path for CliWrapper and subprocess Spirits; the `SandboxSpec` built at admission is the spec the process runs under (no second computation).
- **AC3.** `apply_resource_ceiling` writes `cpu.max`, `memory.max`, `pids.max` for the Spirit's cgroup; the operator can read them under `/sys/fs/cgroup/maos/spirit-<pid>/` (FR6).
- **AC4.** A Worker child's environment carries no raw provider key: either a kernel-minted scoped credential, or the vendor call is an explicit out-of-port record in the Transparency Log (FR4/FR47 honesty).
- **AC5.** T3 (podman/docker) is reachable from a manifest, not only under `smoke-t3-sandbox-5`.

### 16-3-subprocess-crash-to-handle-crash — Subprocess crash reaches the crash handler

*Closes:* FR12/FR50; NFR-Rel-1/2

- **AC1 (command).** `kill -9` on a running Worker produces `task.orphaned` for its in-flight task within 2 s; on a 100-SIGKILL corpus ≥99/100, and the `nfr-rel-1-crash-detection-2s` job asserts that number (today it asserts ≥1 frame).
- **AC2.** A 50-hang corpus yields `task.stalled` within 60 s for ≥48/50, asserted by `nfr-rel-2-hang-detection-60s`.
- **AC3.** `on_crash.action` policies apply to the dead Spirit's tasks; `ReassignToReplica` degrades to escalate *visibly* (journaled reason), never silently.

### 16-4-kernel-uninstall-and-keyring-secrets — Kernel uninstall and keyring secrets

*Closes:* FR2/FR65; NFR-Sec-16

- **AC1 (command).** `maos uninstall` removes the binary, `MAOS_HOME`, sockets and caches; `--keep-log` retains the Transparency Log; a re-install on the same host is clean (FR2).
- **AC2.** `maos-secrets` gains the OS-keyring backend its `Cargo.toml` already promises; provider keys are read from it by default, with the env path explicit and logged as a downgrade; `FIXME(secrets)` at `main.rs:3079` retired.
- **AC3.** `maosctl uninstall <spirit>` still emits the FR65 proof; the kernel uninstall emits an aggregate receipt.

### 16-5-j0-halt-on-ambiguity-scene — J0 halt-on-ambiguity scene

*Closes:* J0; FR15/FR58; NFR-Onb-2

- **AC1 (command).** On a clean VM, the exit command above runs end to end inside 5 minutes (NFR-Onb-2), and the operator sees the halt, the resolution, the audit rows and the clean uninstall.
- **AC2.** hello-spirit halts on an under-specified request with tag `task.acceptance_criterion.ambiguous`; the three resolution pathways are journaled (FR15).
- **AC3.** `docs/maos.dev` run guide documents the API key and the scene; `crates/maos-journey-test` `journey_j0` asserts the halt and the audit row, not the banner.

### 16-6-audit-drop-legal-hold-and-a2a-deny-vocabulary — Debt slot: audit drop, legal hold, A2A deny vocabulary

*Closes:* D3, D5, D7, D18 — repaired out-of-kernel where 14-0 ruled; FLAG-Winston otherwise

- **AC1 (command).** Fault-injecting the audit sink makes `record_invocation` return `Err` and the invocation is refused (I2) — a test proves the drop can no longer return `Ok`.
- **AC2.** The legal-hold check-then-act race is closed by a serialized path; `forget_with_reason` under hold is refused with a typed reason.
- **AC3.** `map_a2a_error_to_iac_bus` preserves the deny vocabulary; `CrossWallRecallRefusal` variants surface as typed causes on the operator path (register D7, D18).

## Dependencies

Epic 15 (green at HEAD). 16-1 before 16-5 (the scene needs a verb that reaches the daemon). 16-2 before Epic 18-2 (real Workers must be sandboxed).

## Not in this epic

Anything whose only deliverable is a gate, ledger, registry or ceiling and is not named above. v2.5 parking rows (`v25-*`) stay parked.
