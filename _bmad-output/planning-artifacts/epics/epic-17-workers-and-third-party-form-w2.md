# Epic 17 — Workers and the Third-Party Form (W2, Fork C)

**Status:** `backlog` — Round 2 correct-course 2026-09-04 (`sprint-change-proposal-2026-09-04-round2.md`, Lunarpulse-ratified; Fork C). Replaces the morning's Epic 17 file. `sprint-status.yaml` is authoritative for status.

**Wave / duration:** 4–6 weeks (measured) · **Makes true:** FR5 true by form: Workers under a real profile, third-party Spirits isolated by construction in WASM; FR29 over the WASM wire; FR33 via a TS→WASM toolchain

**Closes:** S2 (sandbox never applied) for every non-first-party form; FR4/FR47 for Workers; FR29 (non-Rust); FR33 (TS); FR55 hooks; NFR-Obs-3

**Hermetic exit command (CI, no secrets; the epic's first CI job, red until green):**

```
maos run spirits/topologies/worker-sandbox.toml --once           # fixture Worker probes ~/.ssh and dials 127.0.0.1:9 → EACCES + refused rows in `maos audit query`
MAOS_FEATURES=wasm-host maos run examples/ts-spirit/spirit.wasm --once   # the TS Spirit recalls its own frames via log.recall; another Spirit's frames are refused
```

**Operator-lane legs (never in the exit):**
- live `claude`/`codex` Worker under the 17-1 profile (needs a vendor key)

**Kernel-Δ:** **17-1 and 17-2 are FLAG-Winston** (`t3/argv.rs`, `cli_wrapper/runtime.rs`, `resource_ceiling.rs`: +40–90 lines, re-pin in their commits). 17-3b is an ABI extension (`maos:spirit@1.1`, additive; `abi-ratifications.toml`), zero kernel-core lines. 17-4 zero.

**Confidence rules (ratified 2026-09-04 R2, binding):** 1 verb-first (exit verbs exist at HEAD or are AC1 in this epic; `check-exit-commands`) · 2 hermetic exit, live optional (paid/human/clock legs are operator-lane rows) · 3 foundations first · 4 cite-or-cut (every AC names file:line + kernel-Δ) · 5 spike before build · 6 size from measurement (+30%) · 7 one decision per fork · 8 inventory from the tree.

**Dev-gate:** external holds (NFR-Sec-7, NFR-Comp-1) = GA ledger only. **Model/review:** frontier allowlist + §A6 net binding; non-degradable on stories marked ◆.

---

## Stories

| Key | Story | Closes · Δ |
|---|---|---|
| `17-1-worker-egress-allowlist-and-scoped-credential` | Worker egress allowlist and scoped credential ◆ FLAG-Winston | S2 (Workers) · FR4/FR5/FR47 · kernel-Δ +30–60 |
| `17-2-worker-cgroups-applied` | Worker cgroups applied ◆ FLAG-Winston | FR6 · kernel-Δ +10–30 |
| `17-3a-wasm-recall-and-componentize-spike` | Spike: WASM recall ops and componentize-js (≤3 days) | rule 5 · kernel-Δ 0 |
| `17-3b-wasm-third-party-form-with-log-recall` | WASM as the third-party form, with log.recall over the wire | FR29/FR33/FR5-by-form · kernel-Δ 0 · maos-wasm-host +200–400 |
| `17-4-hooks-observer-and-activity-corpus` | Hooks, Observer, and the activity corpus | FR55 · NFR-Obs-3 · kernel-Δ 0 |

AC1 is always the command and what the operator observes; every AC cites the code it changes; `bmad-create-story` may tighten but not add uncited ACs.

### 17-1-worker-egress-allowlist-and-scoped-credential — Worker egress allowlist and scoped credential ◆ FLAG-Winston

*Closes · Δ:* S2 (Workers) · FR4/FR5/FR47 · kernel-Δ +30–60

- **AC1 (command).** Exit line 1: the probing fixture Worker gets `EACCES` on `~/.ssh` and a refused connect; a second fixture that dials an allowlisted host succeeds; both outcomes are rows in `maos audit query`.
- **AC2.** T3 argv gains an egress allowlist derived from the manifest `[network].egress` (today `--network=none`, `t3/argv.rs:68-73`); T2 unchanged; admission refuses undeclared egress.
- **AC3.** The CliWrapper spawn `env_clear()`s and injects `nonsecret_env()` plus a kernel-minted scoped credential (ADR-061); a test reads `/proc/<pid>/environ` and asserts no raw vendor key (`credential_env_var()` gets its first consumer; note at `worker_spawn.rs:545` retired).
- **AC4.** `spawn_sandboxed` (`sandbox/mod.rs:140-149` returns `SandboxUnavailable` for T3 today) is either extended to T3 or `spawn_t3` is the single Worker path — one path, stated.

### 17-2-worker-cgroups-applied — Worker cgroups applied ◆ FLAG-Winston

*Closes · Δ:* FR6 · kernel-Δ +10–30

- **AC1 (command).** With cgroup delegation available, `cat /sys/fs/cgroup/maos/spirit-<pid>/{cpu.max,memory.max,pids.max}` shows the manifest caps for a running Worker; without delegation the daemon journals a typed downgrade.
- **AC2.** One layout (today two: `resource_ceiling.rs:45-66` and `linux.rs:372-394`); `pids.max` added; `apply_resource_ceiling` called from the Worker spawn path (zero callers today).
- **AC3.** `cgroup_ceiling_smoke` updated to the single layout.

### 17-3a-wasm-recall-and-componentize-spike — Spike: WASM recall ops and componentize-js (≤3 days)

*Closes · Δ:* rule 5 · kernel-Δ 0

- **AC1 (command).** A written go/no-go in the story file answers three questions: componentize-js produces a `maos:spirit@1.0` component from a TS Spirit; a `log.recall` request op can be added to the WIT additively (`check-wasm-form-equiv` and `abi-diff` stay green on the ratification path); `frame_bridge` can carry intent/consent/lineage/scope.
- **AC2.** Spike code lives under `spikes/`, never merged to `crates/` (11-0 precedent).

### 17-3b-wasm-third-party-form-with-log-recall — WASM as the third-party form, with log.recall over the wire

*Closes · Δ:* FR29/FR33/FR5-by-form · kernel-Δ 0 · maos-wasm-host +200–400

- **AC1 (command).** Exit line 2 runs: the TS Spirit compiled with componentize-js calls `log.recall` and prints its own participant frames; a request for another Spirit's frames is refused by token scope.
- **AC2.** WIT `maos:spirit@1.1` adds `log.recall`/`log.fetch` (additive; entry in `xtask/abi-ratifications.toml`; `abi-diff` additive-only).
- **AC3.** `frame_bridge` carries intent, consent, lineage and scope (dropped today).
- **AC4.** Registry admission accepts WASM artifacts at trust tier ≥ `public-untrusted` and refuses `rust-inproc` from any registry (ADR-060).
- **AC5.** `check-wasm-form-equiv` stays green; `sdks/spirit-ts` README no longer says "not a kernel runtime" for the WASM path.

### 17-4-hooks-observer-and-activity-corpus — Hooks, Observer, and the activity corpus

*Closes · Δ:* FR55 · NFR-Obs-3 · kernel-Δ 0

- **AC1 (command).** `maos run spirits/observer/manifest.toml` alongside Butler prints live scalar/telemetry events (Observer added to `LoadedSpiritKind`, `main.rs:433-442`).
- **AC2.** A telemetry pump in `maos-bin` bridges `TelemetryStreamAdapter` to `fire_on_telemetry_event` (first caller; `hook_dispatch.rs:314`).
- **AC3.** `fire_on_consolidate` gets an idle-cadence caller and Observer implements `on_consolidate`; `fire_on_schedule` already has one (`schedule_watchdog.rs:259`).
- **AC4.** A 20-scenario activity-stream corpus is authored with missed-event ≤2% and causal-ordering ≥99% oracles over the TL (none exists today).

## Dependencies

Epic 15 (15-2, ADR-060/061). 17-3a before 17-3b. 17-1 before Epic 19-3.
