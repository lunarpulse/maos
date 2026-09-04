# Epic 19 — The Founder Loop (W4)

**Status:** `backlog` — Round 2 correct-course 2026-09-04 (`sprint-change-proposal-2026-09-04-round2.md`, Lunarpulse-ratified; Fork C). Replaces the morning's Epic 19 file. `sprint-status.yaml` is authoritative for status.

**Wave / duration:** 4–6 weeks (measured) · **Makes true:** v0.8 · J1 wedge: Orchestrator decomposes, real Workers execute under a profile, halt on ambiguity, digest at the end

**Closes:** FR20, FR21, FR25, FR52; RELEASE-HOLDS row 16; J1 ABSENT beats except the two-host signed run

**Hermetic exit command (CI, no secrets; the epic's first CI job, red until green):**

```
MAOS_INFERENCE_MODE=replay cargo run -p xtask -- demo-j1 --replay     # unattended; ABSENT = {two-host-signed-run} only; digest cites a source ref per completion
```

**Operator-lane legs (never in the exit):**
- `demo-j1 --live-codex` (paid)
- two-host signed run (owned by the closed `j1-crosshost-2d` lane; permanent operator artifact)

**Kernel-Δ:** ZERO kernel-Δ @24472 (19-2 may need ≤10 lines for a safe-point hook; FLAG-Winston if so).

**Confidence rules (ratified 2026-09-04 R2, binding):** 1 verb-first (exit verbs exist at HEAD or are AC1 in this epic; `check-exit-commands`) · 2 hermetic exit, live optional (paid/human/clock legs are operator-lane rows) · 3 foundations first · 4 cite-or-cut (every AC names file:line + kernel-Δ) · 5 spike before build · 6 size from measurement (+30%) · 7 one decision per fork · 8 inventory from the tree.

**Dev-gate:** external holds (NFR-Sec-7, NFR-Comp-1) = GA ledger only. **Model/review:** frontier allowlist + §A6 net binding; non-degradable on stories marked ◆.

---

## Stories

| Key | Story | Closes · Δ |
|---|---|---|
| `19-1-orchestrator-dispatch-loop-and-epic-input` | Orchestrator dispatch loop and `--epic` input | FR21 · kernel-Δ 0 |
| `19-2-fr20-enqueue-door-and-safe-point` | FR20 enqueue door and safe-point dequeue | FR20 · kernel-Δ 0–10 |
| `19-3-real-worker-default-with-effect-oracle` | Real Worker by default, with an effect oracle | FR25/FR52 · kernel-Δ 0 |
| `19-4-j1-beats-and-demo-replay` | J1 beats and `demo-j1 --replay` | J1 · FR17 · kernel-Δ 0 |

AC1 is always the command and what the operator observes; every AC cites the code it changes; `bmad-create-story` may tighten but not add uncited ACs.

### 19-1-orchestrator-dispatch-loop-and-epic-input — Orchestrator dispatch loop and `--epic` input

*Closes · Δ:* FR21 · kernel-Δ 0

- **AC1 (command).** `MAOS_INFERENCE_MODE=replay maos run spirits/topologies/j1-founder-loop.toml --epic tests/fixtures/j1/epic.md --once` makes the Orchestrator emit N `task.assign` frames decomposed from the epic (the composition root currently forbids a loop, `main.rs:3434-3440`, and calls `assign_frame_remote` once per host).
- **AC2.** `MAOS_DELEGATED_GOAL` (`main.rs:3495`) is removed on the loopback arm and `env_contract` updated; `check_j1_loopback_delegation.rs` stays green.
- **AC3.** Orchestrator gains `with_inference_port` and a capability scope (`spirits/orchestrator/manifest.toml:36` today "provider.complete omitted"); it stays `rust-inproc` (ADR-060).
- **AC4.** FR21 distillate rows are written for Worker output in production; `followup_dispatch` (`lib.rs:389-401`) consumes them.

### 19-2-fr20-enqueue-door-and-safe-point — FR20 enqueue door and safe-point dequeue

*Closes · Δ:* FR20 · kernel-Δ 0–10

- **AC1 (command).** `maosctl orchestrator queue "…"` during an in-flight delegation is applied at the next safe point of the running daemon (`dequeue_at_safe_point` gets its first caller; enqueue arrives over the 16-1 surface).
- **AC2.** The `Ctx` ABI is unchanged (`lib.rs:143-147`); the loop is daemon-side.

### 19-3-real-worker-default-with-effect-oracle — Real Worker by default, with an effect oracle

*Closes · Δ:* FR25/FR52 · kernel-Δ 0

- **AC1 (command).** Hermetic: `demo-j1 --replay` runs the fixture Worker. Live (operator lane): the default worker manifest is a real agent CLI under the 17-1 profile.
- **AC2.** A worktree-diff effect oracle in `worker_spawn.rs` with an explicit child cwd replaces the claude adapter's effect-blind verdict (`worker_cli.rs:492-495`); RELEASE-HOLDS row 16 closed.
- **AC3.** `ci_local_split_refuses_a_granted_real_agent_without_the_live_flag` is rewritten for the new default; three concurrent fixture Workers run hermetically; `nfr-perf-8` measures the real spawn path (advisory until wall-clock stable).

### 19-4-j1-beats-and-demo-replay — J1 beats and `demo-j1 --replay`

*Closes · Δ:* J1 · FR17 · kernel-Δ 0

- **AC1 (command).** The exit command runs unattended; `demo_j1.rs` renders only `two-host-signed-run` as ABSENT (today three: `:888-905`).
- **AC2.** A planted ambiguous story makes the Orchestrator halt within 2 frames on `story.acceptance_criterion.ambiguous` (the tag exists only in `lcas.rs:84` today; no emitter).
- **AC3.** Butler joins the J1 topology (or the daemon renders) so the digest cites `source_log_ref` for every completion; 0 hallucinated on the run.
- **AC4.** `journey_j1` asserts the halt and digest beats.

## Dependencies

Epic 17 (17-1 profile) and Epic 18 (inference seams). 19-1 before 19-4.
