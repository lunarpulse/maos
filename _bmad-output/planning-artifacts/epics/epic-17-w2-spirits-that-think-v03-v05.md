# Epic 17 — W2: Spirits That Think (v0.3 / v0.5 for real)

**Status:** `backlog` — created 2026-09-04 by `sprint-change-proposal-2026-09-04.md` (bmad-correct-course, Lunarpulse-ratified). One of six recovery epics (15–20); `sprint-status.yaml` is authoritative for status.

**Wave / duration:** 3–4 weeks · **Makes true:** v0.3 Butler · v0.5 Researcher + Observer

**Closes:** S3 (one Spirit thinks); FR17, FR26, FR29 (wire), FR30/32 measured, FR55 (hooks), FR56/57; NFR-Test-4, NFR-Aud-7, NFR-Obs-3

**Exit command (the epic is done when an operator runs this on a clean machine and sees what it says):**

```
MAOS_ANTHROPIC_API_KEY=… maos run spirits/butler/manifest.toml --live
# 19:00 fixture calendar → Butler proposes; three weeks of declining acceptance → self-tuning halt
maos eval halt --class butler                                   # recall ≥0.7 / precision ≥0.85 from REAL completions
```

**Kernel-Δ:** **ZERO kernel-Δ @24472** unless 17-4's wire recall op needs a kernel line (FLAG-Winston, measured at landing).

**Dev-gate:** external holds (pen-test NFR-Sec-7 + export counsel NFR-Comp-1) = GA ledger only, non-gating for dev. **Model/review discipline:** frontier-class dev allowlist + §A6 full-layer review net is the binding control (E11 retro A1). **Story rules (CP-2):** AC1 of every story is a command plus what the operator observes; a story splits once; story files ≤3,000 words; governance work ≤20% of the epic.

---

## Stories

| Key | Story | Closes |
|---|---|---|
| `17-1-butler-through-inference-port` | Butler reasons through the Inference Port | S3; FR17/FR55 on_idle; NFR-Test-4 |
| `17-2-researcher-live-by-default-real-five-metric` | Researcher live by default, real five-metric gate | S3/S4; NFR-Aud-7; J-Researcher |
| `17-3-observer-launchable-live-telemetry` | Observer launchable on the live Telemetry Stream | S3; NFR-Obs-3; FR55 |
| `17-4-three-more-hooks-and-wire-log-recall` | Three more hooks and log.recall over the wire | FR29/FR55; NFR-Test-14 |

Per-story ACs below are sketches; `bmad-create-story` finalizes them at preflight (≤6 ACs, AC1 unchanged in kind).

### 17-1-butler-through-inference-port — Butler reasons through the Inference Port

*Closes:* S3; FR17/FR55 on_idle; NFR-Test-4

- **AC1 (command).** The exit command runs Sandra's 7 PM scene with a real model: Butler's `on_idle` calls `InferencePort::complete` with the anticipatory prompt over one LIVE MCP driver (Google Calendar read), and the operator sees the proposal and the later self-tuning halt.
- **AC2.** FR17 morning digest generated on the live path within 30 s of first session, citing source log refs for every claimed completion.
- **AC3.** Halt-recall ≥0.7 and precision ≥0.85 measured on the 30-scenario Butler corpus with real completions; numbers published in the registry metadata (NFR-Test-4 per class, for the first time).
- **AC4.** The retired `claude-3-haiku-20240307` pin is replaced and `check-model-currency` is green.

### 17-2-researcher-live-by-default-real-five-metric — Researcher live by default, real five-metric gate

*Closes:* S3/S4; NFR-Aud-7; J-Researcher

- **AC1 (command).** `maos run spirits/researcher/manifest.toml --once` performs a live survey; `--deterministic` is the explicit opt-out and `--replay <cassette>` the hermetic mode; a provider error is a typed halt with non-zero exit, never a silent fallback (`lib.rs:925` removed).
- **AC2.** `nfr-aud-7-distillate-five-metrics-floor` computes recall, faithfulness, hedge-preservation, traceability and secret-leakage from real distillates against raw frames — the fixture-label mean is deleted.
- **AC3.** The 90-minute budget is enforced through `[budget].time_cap` with a `BudgetWarning` frame at 80% (NFR-Perf-6).

### 17-3-observer-launchable-live-telemetry — Observer launchable on the live Telemetry Stream

*Closes:* S3; NFR-Obs-3; FR55

- **AC1 (command).** `maos run spirits/observer/manifest.toml` alongside Butler shows live activity from the Telemetry Stream (observer added to `LoadedSpiritKind`, `main.rs:447-454`).
- **AC2.** Missed-event rate ≤2% and causal-ordering ≥99% on the 20-scenario activity-stream corpus, measured.
- **AC3.** `fire_on_telemetry_event` gets its first kernel caller.

### 17-4-three-more-hooks-and-wire-log-recall — Three more hooks and log.recall over the wire

*Closes:* FR29/FR55; NFR-Test-14

- **AC1 (command).** A TypeScript Spirit built from `sdks/spirit-ts` calls `log.recall` over the subprocess NDJSON wire and receives participant-scoped frames (FR29 for a non-Rust form, for the first time).
- **AC2.** `on_schedule` and `on_consolidate` are implemented in at least one reference Spirit each and their kernel firers have callers.
- **AC3.** Every reference manifest declares `forms=["subprocess"]`; `rust-inproc` is retired from the tree as the roadmap disposition already states.
- **AC4.** The cross-language golden corpus (NFR-Test-14) covers the new wire ops.

## Dependencies

Epic 16 (a Spirit the operator can pause while it reasons). 17-4 before Epic 20-2 (J3 needs the wire recall).

## Not in this epic

Anything whose only deliverable is a gate, ledger, registry or ceiling and is not named above. v2.5 parking rows (`v25-*`) stay parked.
