# Epic 18 — Spirits That Think (W3)

**Status:** `backlog` — Round 2 correct-course 2026-09-04 (`sprint-change-proposal-2026-09-04-round2.md`, Lunarpulse-ratified; Fork C). Replaces the morning's Epic 18 file. `sprint-status.yaml` is authoritative for status.

**Wave / duration:** 4–6 weeks (measured) · **Makes true:** v0.3 Butler and v0.5 Researcher: cognition through the Inference Port, measured against a model, replayable in CI

**Closes:** S3; FR17, FR30/32 measured, FR56; NFR-Test-4 per class, NFR-Aud-7 real, NFR-Perf-6; J-Butler, J-Researcher

**Hermetic exit command (CI, no secrets; the epic's first CI job, red until green):**

```
MAOS_INFERENCE_MODE=replay maos run spirits/butler/manifest.toml --once      # Butler proposes from the cassette; the halt fires on the conflict scenario
maos eval halt --class butler                                                   # recall ≥0.7 / precision ≥0.85 from recorded completions
maos run spirits/researcher/manifest.toml --replay tests/cassettes/j-researcher/survey-distill.json --once
```

**Operator-lane legs (never in the exit):**
- nightly re-record (`MAOS_INFERENCE_MODE=record`, provider key)
- Google Calendar MCP with OAuth (the CI server is a conformant fixture)

**Kernel-Δ:** ZERO kernel-Δ @24472 (clock injection at the adapter; if a kernel hook is needed, FLAG-Winston).

**Confidence rules (ratified 2026-09-04 R2, binding):** 1 verb-first (exit verbs exist at HEAD or are AC1 in this epic; `check-exit-commands`) · 2 hermetic exit, live optional (paid/human/clock legs are operator-lane rows) · 3 foundations first · 4 cite-or-cut (every AC names file:line + kernel-Δ) · 5 spike before build · 6 size from measurement (+30%) · 7 one decision per fork · 8 inventory from the tree.

**Dev-gate:** external holds (NFR-Sec-7, NFR-Comp-1) = GA ledger only. **Model/review:** frontier allowlist + §A6 net binding; non-degradable on stories marked ◆.

---

## Stories

| Key | Story | Closes · Δ |
|---|---|---|
| `18-1-butler-inference-seam-and-conformant-mcp` | Butler inference seam and a conformant MCP client | S3 · FR17/FR55 · kernel-Δ 0 |
| `18-2-eval-halt-verb-and-corpus-numbers` | `maos eval halt` and per-class numbers | NFR-Test-4 · kernel-Δ 0 |
| `18-3-researcher-replay-live-and-judged-five-metric` | Researcher replay/live flags and a judged five-metric gate | NFR-Aud-7 · NFR-Perf-6 · kernel-Δ 0 |
| `18-4-self-tuning-halt-with-injectable-clock` | Self-tuning halt with an injectable clock | J-Butler · kernel-Δ 0 |

AC1 is always the command and what the operator observes; every AC cites the code it changes; `bmad-create-story` may tighten but not add uncited ACs.

### 18-1-butler-inference-seam-and-conformant-mcp — Butler inference seam and a conformant MCP client

*Closes · Δ:* S3 · FR17/FR55 · kernel-Δ 0

- **AC1 (command).** Exit line 1: Butler's `on_idle` calls `InferencePort::complete` (deferred-port builder pattern, `main.rs:4599-4622`; Butler has no such field today, `spirits/butler/src/lib.rs:337-470`) and emits a proposal from the cassette; `=live` with a key produces a real completion (operator lane).
- **AC2.** `maos-mcp` streamable-HTTP client implements `initialize`, session id and bearer auth (today no `initialize`, no auth header; stdio spawns per call); proven in CI against a protocol-conformant fixture calendar server; Google OAuth documented as operator lane.
- **AC3.** FR17 morning digest produced on the live path (`morning_digest` gets a production caller; `butler/lib.rs:675-728`).
- **AC4.** The retired model pin is gone (depends on the hotfix lane).

### 18-2-eval-halt-verb-and-corpus-numbers — `maos eval halt` and per-class numbers

*Closes · Δ:* NFR-Test-4 · kernel-Δ 0

- **AC1 (command).** `maos eval halt --class butler` (new verb) replays the 30-row corpus (`spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl`) through the 18-1 seam from cassettes and prints recall/precision; ≥0.7/≥0.85 asserted; numbers written to registry metadata (NFR-Test-4, first real per-class number).
- **AC2.** The corpus gains model I/O fields (prompt, completion reference); provenance of each cassette recorded.
- **AC3.** The nightly re-record leg refreshes the cassettes (operator lane).

### 18-3-researcher-replay-live-and-judged-five-metric — Researcher replay/live flags and a judged five-metric gate

*Closes · Δ:* NFR-Aud-7 · NFR-Perf-6 · kernel-Δ 0

- **AC1 (command).** Exit line 3 runs from a cassette; `--deterministic` is explicit; live only when a provider is configured, otherwise a typed `Unconfigured` halt (the silent fallback at `lib.rs:925` removed).
- **AC2.** A judge harness in `maos-eval` scores real distillates against raw frames via recorded judge cassettes; `distillate_five_metrics_floor.rs:53-81` (a mean of fixture labels) is replaced by it and the corpus regenerated from real runs.
- **AC3.** `time_cap_seconds = 5400` and `BudgetWarning80` proven with an injected clock (`hook_dispatch.rs:357-366`).

### 18-4-self-tuning-halt-with-injectable-clock — Self-tuning halt with an injectable clock

*Closes · Δ:* J-Butler · kernel-Δ 0

- **AC1 (command).** `maos run spirits/butler/manifest.toml --replay … --clock compressed:21d --once` shows the acceptance ledger declining over three simulated weeks and the self-tuning halt firing; `jb3_self_tuning_halt` and `journey_butler.rs:285` un-ignored.
- **AC2.** The `notification_acceptance_log` exists and persists in the principal namespace (FR31); `Butler::with_clock` replaces `SystemTime::now()` at `lib.rs:833`.

## Dependencies

Epic 15 (15-6 seam) and Epic 16 (a pausable daemon). 18-1 before 18-2.
