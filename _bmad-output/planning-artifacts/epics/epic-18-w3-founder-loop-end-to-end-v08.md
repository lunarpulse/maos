# Epic 18 — W3: The Founder Loop, End to End (v0.8)

**Status:** `backlog` — created 2026-09-04 by `sprint-change-proposal-2026-09-04.md` (bmad-correct-course, Lunarpulse-ratified). One of six recovery epics (15–20); `sprint-status.yaml` is authoritative for status.

**Wave / duration:** 3–4 weeks · **Makes true:** v0.8 · J1 founder-loop wedge

**Closes:** J1 ABSENT beats; FR20, FR21, FR25, FR52; NFR-Perf-8; RELEASE-HOLDS row 16 (claude verdict is not an effect oracle)

**Exit command (the epic is done when an operator runs this on a clean machine and sees what it says):**

```
cargo run -p xtask -- demo-j1 --live     # one command, zero ABSENT beats, digest at the end cites a source ref per completion
```

**Kernel-Δ:** **ZERO kernel-Δ @24472.**

**Dev-gate:** external holds (pen-test NFR-Sec-7 + export counsel NFR-Comp-1) = GA ledger only, non-gating for dev. **Model/review discipline:** frontier-class dev allowlist + §A6 full-layer review net is the binding control (E11 retro A1). **Story rules (CP-2):** AC1 of every story is a command plus what the operator observes; a story splits once; story files ≤3,000 words; governance work ≤20% of the epic.

---

## Stories

| Key | Story | Closes |
|---|---|---|
| `18-1-orchestrator-decomposes-via-inference-port` | Orchestrator decomposes through the Inference Port | FR20/FR21 |
| `18-2-real-worker-default-with-effect-oracle` | Real Worker by default, with an effect oracle | FR25/FR52; NFR-Perf-8 |
| `18-3-j1-halt-and-overnight-digest-beats` | J1 halt and overnight-digest beats | J1; FR17 |

Per-story ACs below are sketches; `bmad-create-story` finalizes them at preflight (≤6 ACs, AC1 unchanged in kind).

### 18-1-orchestrator-decomposes-via-inference-port — Orchestrator decomposes through the Inference Port

*Closes:* FR20/FR21

- **AC1 (command).** `maos run spirits/topologies/j1-founder-loop.toml --epic <file>` makes the Orchestrator emit N `task.assign` frames derived by the model from the epic; `MAOS_WORKER_TASK` is removed.
- **AC2.** FR21 distillate-required dispatch holds; raw output recallable via `log.recall`.
- **AC3.** FR20: an instruction queued while a delegation is in flight is applied at the next safe point of the running daemon.

### 18-2-real-worker-default-with-effect-oracle — Real Worker by default, with an effect oracle

*Closes:* FR25/FR52; NFR-Perf-8

- **AC1 (command).** The default Worker manifest wraps a real agent CLI (claude or codex) under the sandboxed CliWrapper from 16-2; the fixture Worker is test-only.
- **AC2.** The claude adapter gets an effect oracle: a kernel-side worktree diff at the spawn seam, so "I have written the file" over an untouched tree scores `completed=false`, exit 1 (closes RELEASE-HOLDS row 16).
- **AC3.** Three concurrent Workers on one epic without context bleed (≤1/20 leak on the 20-story corpus); `nfr-perf-8-orchestrator-fanout` promoted to blocking at the measured floor.

### 18-3-j1-halt-and-overnight-digest-beats — J1 halt and overnight-digest beats

*Closes:* J1; FR17

- **AC1 (command).** The exit command runs unattended to completion; `demo_j1.rs` renders no beat as ABSENT.
- **AC2.** A planted ambiguous story makes the Orchestrator halt within 2 frames on `task.acceptance_criterion.ambiguous`.
- **AC3.** The overnight digest cites `source_log_ref` for every claimed completion; 0 hallucinated tasks on the run (FR17 floor, measured on the run itself).
- **AC4.** `journey_j1` asserts the halt and digest beats.

## Dependencies

Epic 16 (16-2 sandbox for real Workers) and Epic 17 (Inference-Port patterns).

## Not in this epic

Anything whose only deliverable is a gate, ledger, registry or ceiling and is not named above. v2.5 parking rows (`v25-*`) stay parked.
