# Epic 26 — Worker Fleet (W11)

**Status:** `backlog` — created by correct-course 2026-11-13 (`sprint-change-proposal-2026-11-13.md` §4–§6, operator-approved 2026-11-13). Fifth and last epic of the Enterprise-Fit lane (Epics 22–26). The lane starts after `v1.0.0` (decision 1); one minor release per epic. `sprint-status.yaml` is authoritative for status.

**Wave:** W11; 4 stories, ≈ 4.75 median-story units `[INFERENCE]` · **Makes true:** Many CLI-wrapped Workers in parallel, metered, capped, crash-safe.

**Closes:** NFR-Perf-8 (orchestrator fan-out; owner `26-1`), FR73 (Worker metering). Re-owned rows: `RELEASE-HOLDS.md` row 10 (`:84`, delegation crash window between `journal(Written)` and the Worker spawn) → `26-2`; `deferred-work.md:1518` ("Worker CLI spend through the egress proxy does not produce cost frames") → `26-3`; `RELEASE-HOLDS.md` row 20 (`:94`, Worker workspaces are not size-capped) → `26-4`.

**Hermetic exit command (CI, no secrets; red until green):**

```
cargo test -p maos-journey-test --test journey_fleet   # 1 — fixture-CLI Workers run in parallel and zero tasks are dropped (26-1 AC1)
```

**Exit-block provenance (measured at `1a57035e`):**
- Line 1 — `journey_fleet` is created by 26-1 AC1 (applied clarification, 2026-11-13: the proposal names the story, not the AC). **HEAD: red** — no such test target (`ls crates/maos-journey-test/tests` has no `journey_fleet.rs`); `cargo test` exits non-zero. Green when the fan-out scene of 26-1 passes with the fixture CLI.
- Shape: one `cargo test` line with a trailing `#` comment; no secret, key, human, wall-clock or network (the Workers run the fixture CLI, `crates/maos-bin/src/worker_cli.rs:944`).

**Kernel-Δ:** forecast 0.

**Lane rules (proposal §4, binding for Epics 22–26).** (1) Adapters stay outside the kernel behind existing ports; `24-3` is the only kernel-touching story. (2) No new `xtask` gate; an exit is a `cargo test` target or a `maos` verb created by a named AC. (3) One hermetic exit per epic. (4) Each external standard is pinned by version in its story, which re-reads the specification. (5) A control is wired or it is labelled. (6) Each epic is preflighted at the previous retrospective.

**Confidence rules.** Rules 1–11 are those printed in `epic-21-multi-host-on-live-substrate-w6.md` under **Confidence rules** and **Confidence rule 11**; they bind here and are not repeated.

---

## Stories

| Key | Story | Closes · size |
|---|---|---|
| `26-1-concurrent-worker-fanout` | Concurrent Worker fan-out | NFR-Perf-8 · L |
| `26-2-crash-safe-delegation-reconciliation` | Crash-safe delegation reconciliation | RELEASE-HOLDS row 10 · M |
| `26-3-worker-egress-usage-metering` | Worker egress usage metering | `deferred-work.md:1518`; FR73 (metering) · M |
| `26-4-worker-workspace-size-cap` | Worker workspace size cap | RELEASE-HOLDS row 20 · S–M |

AC1 is the command and what the operator observes. `bmad-create-story` re-measures every cite and may tighten, not add uncited ACs.

### 26-1-concurrent-worker-fanout — Concurrent Worker fan-out

*Closes · Δ:* NFR-Perf-8 · size L · kernel-Δ forecast 0.

- *HEAD at `1a57035e`:* each Worker runs inside `block_in_place` (`crates/maos-bin/src/main.rs:5661`); the fixture CLI is `worker-cli-fixture` in `SUPPORTED_WORKER_CLIS` (`crates/maos-bin/src/worker_cli.rs:943-944`).
- **AC1 (command).** `maos run <topology> --epic <file> --parallel N` runs up to N declared-independent sections at once; N = 1 keeps today's order. This AC creates `cargo test -p maos-journey-test --test journey_fleet` (exit line 1) (applied clarification, 2026-11-13).
- **AC2.** With the fixture CLI (`crates/maos-bin/src/worker_cli.rs:944`) and N = 50, zero tasks are dropped; latency is recorded, not gated.
- **AC3.** Dependent sections still receive the prior distillate.

### 26-2-crash-safe-delegation-reconciliation — Crash-safe delegation reconciliation

*Closes · Δ:* `RELEASE-HOLDS.md` row 10 (`:84`) · size M · kernel-Δ forecast 0.

- *HEAD at `1a57035e`:* row 10 states that the delegation path's crash window between `journal(Written)` and the Worker spawn is bounded, not closed (`RELEASE-HOLDS.md:84`).
- **AC1 (command).** After a kill between `journal(Written)` and the spawn, the next boot re-dispatches the task or delivers `task.orphaned` to the originator.

### 26-3-worker-egress-usage-metering — Worker egress usage metering

*Closes · Δ:* `deferred-work.md:1518` ("Worker CLI spend through the egress proxy does not produce cost frames"); FR73 (metering) · size M · kernel-Δ forecast 0. Writes its part of the ADR-046 amendment.

- *HEAD at `1a57035e`:* the egress proxy is `crates/maos-egress/src/proxy.rs`; Worker CLI spend through it produces no cost frame (`deferred-work.md:1518`).
- **AC1 (command).** The egress proxy (`crates/maos-egress/src/proxy.rs`) emits a cost frame at the Worker's pid from the usage the provider returns; an unreadable usage is a row marked unknown.
- **D-row.** Whether the proxy sees response bodies `[INFERENCE]`.

### 26-4-worker-workspace-size-cap — Worker workspace size cap

*Closes · Δ:* `RELEASE-HOLDS.md` row 20 (`:94`) · size S–M · kernel-Δ forecast 0.

- *HEAD at `1a57035e`:* row 20 states that Worker workspaces are not size-capped (`RELEASE-HOLDS.md:94`).
- **AC1 (command).** As `RELEASE-HOLDS.md:94` specifies, with its proven-red vector.

**Left out:** vendor KMS backends; more Worker CLI adapters than `codex` and `claude`; an evaluation gate at admission; a graphical console; a non-loopback operator door; messaging gateways (FR54 stays v2.5).

## Dependencies

- Epic 25 is done and its retrospective preflights this epic (lane rule 6).
- `26-3` takes its spend figure from story S2 of the reasoning-model adapter, which lands before GA.
- ADR-046 amendment: `24-6`, `26-3`.
