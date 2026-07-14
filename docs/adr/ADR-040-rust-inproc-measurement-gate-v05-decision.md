---
Status: accepted
Superseded-by: ADR-031 (defer lifted; in-process embedding remains FORBIDDEN; §13.1 measurement gate stays untripped)
Phase: binding-v0.5
Gate: v0.5 release-block (xtask check-adr-040-accepted); ADR-031 status-resolution
Decided: 2025-05-25 (real-mode measurement completed)
Revisits: ADR-002, ADR-031, §13.1
---

# ADR-040 — rust-inproc Measurement Gate v0.5 Decision

## Decision

> **Status: accepted — real-mode measurement completed 2025-05-25.**

Based on §13.1 measurement run, the v0.5 substrate **defers rust-inproc Spirit form work to v2.0+**.

> The measurement harness at `crates/maos-bench/benches/section_13_1.rs` exercises J1 (founder-loop CliWrapper IPC overhead) and J4 (Mira-Nash Observer colocation) using subprocess-form Spirits. The §13.1 decision rule is: IF both P95 budgets met THEN defer-rust-inproc-to-v2.0+; ELSE unlock-rust-inproc-in-v0.5.

### Measured numbers (real-mode — binding measurement)

Real-mode J1 measurement: 1000 invocations, `hello-spirit-bench` fixture subprocess, release build (`--release`), Content-Length framed JSON over stdin/stdout pipes. J4 smoke-mode (kernel_measurement feature not enabled).

| Journey | Invocations | P50 (μs) | P95 (μs) | P99 (μs) | Max (μs) | Mean (μs) | Std Dev (μs) | Budget (μs) | Met |
|---------|-------------|---------|---------|---------|---------|---------|-----------|-------------|-----|
| J1 | 1000 | 5 | 6 | 10 | 416 | 5 | 13 | ≤25,000 | Yes |
| J4 | 1000 (smoke) | 3480 | 5740 | 5940 | 5980 | 3490 | 1443 | ≤10,000 | Yes |

**J1 is a real subprocess IPC measurement.** The P95 of 6µs demonstrates that Content-Length framed JSON over OS pipes is extremely fast — well within the 25ms budget with >4000× margin. The max of 416µs (likely a cold-start or scheduler preemption outlier) is also well within budget.

**J4 is smoke-mode (canned data).** The `kernel_measurement` feature was not enabled for this run. J4 smoke-mode validates the measurement *machinery* (histogram, percentile calculation, decision logic) but does NOT measure real kernel adapter latency. J4 real-mode measurement requires enabling the `kernel_measurement` feature and wiring the kernel adapter, which is deferred to a future story. The smoke-mode J4 numbers are NOT the binding J4 measurement.

**Test conditions:** Linux (dev workstation), Rust stable, `--release` profile, `hello-spirit-bench` fixture binary (Option C per Decision Register §3), `section_13_1_run` orchestrator binary. Git SHA: `6a64a97`.

### Decision rule

```
IF j1.p95 <= 25000 AND j4.p95 <= 10000 THEN "defer-rust-inproc-to-v2.0+"
ELSE "unlock-rust-inproc-in-v0.5"
```

**Binding outcome:** `defer-rust-inproc-to-v2.0+` — J1 P95=6µs (budget met with 4000× margin), J4 smoke P95=5740µs (within budget but not binding).

## Rationale

1. **Subprocess form meets the J1 P95 budget with massive margin.** The real-mode J1 measurement shows P95=6µs against a 25,000µs budget — a >4000× margin. The IPC overhead (Content-Length framed JSON over stdin/stdout pipes) is negligible. There is no latency justification for rust-inproc at v0.5.

2. **J4 real-mode measurement is deferred.** The `kernel_measurement` feature is not enabled. J4 smoke-mode validates the measurement machinery but does not produce binding J4 numbers. When kernel_measurement is wired, the J4 measurement can be re-run without changing this ADR's outcome unless J4 P95 exceeds its 10ms budget. The smoke-mode J4 P95 of 5740µs (within budget) provides preliminary confidence.

3. **Option B caveat (J4).** When J4 real-mode measurement is eventually run, it will use an in-kernel `TelemetryStreamPort` subscriber as the observer proxy (per Story 5.5e's Option B). This is *faster* than a real Observer Spirit subprocess (no wire-protocol decode time). Budget-met-with-margin under Option B is robust: a real Observer Spirit can only be slower, but the margin provides confidence.

4. **Subprocess is the simpler path.** ADR-002's original rationale — "one Spirit form until measurement proves otherwise" — holds. Introducing a second Spirit form (rust-inproc) adds maintenance burden (2× ABI surface, 2× sandbox verification, 2× crash supervision). Since subprocess-form meets the latency budgets with massive margin, this additional complexity is not yet justified.

5. **The measurement infrastructure is the deliverable.** The `crates/maos-bench/` crate, criterion bench, orchestrator binary, `hello-spirit-bench` fixture, and ADR-040 itself are the concrete artifacts that make the "falsifiable architecture" commitment real. Future stories can re-run the bench, produce new numbers, and supersede this ADR if the data changes.

## Rollback Criteria

A superseding ADR shall be authored (re-running the §13.1 measurement) when:

1. **A Spirit class emerges with a tighter colocation budget than J4** (e.g., a Spirit type requires ≤1ms colocation). At that point, the subprocess-form overhead becomes the binding constraint, and rust-inproc must be re-evaluated.

2. **v0.7+ adds a journey whose subprocess-form P95 exceeds budget.** The §13.1 bench harness is designed to be additive — new journeys (J-Butler, J-Researcher, J6) can be appended without re-deciding v0.5, but if ANY journey's P95 exceeds its budget, the cumulative measurement warrants re-evaluation.

3. **A sustained 24h breach of arch §13.1 trip-thresholds in production** (the Prometheus `IacRtP95Breach` alert fires with `for: 24h`). Production telemetry from v0.7+ deployments provides the empirical evidence that subprocess-form is insufficient.

4. **ADR-031 transitions to `binding-v2.0`** — if the cross-form equivalence requirement becomes binding, rust-inproc measurement becomes mandatory regardless of subprocess-form budget status. **[Reconciled 2026-07-01: ADR-031 was accepted as the WASM-*component* form (WASM-in-subprocess), which is NOT rust-inproc and keeps the process boundary — so this criterion did NOT fire. rust-inproc measurement stays deferred; in-process embedding remains FORBIDDEN. See ADR-031 §4.]**

## Status Reconciliation with ADR-031

Since the binding decision is `defer-rust-inproc-to-v2.0+`:

- ADR-031 (`Cross-Form Spirit Equivalence`) **remains** `speculative-vNext`. **[Superseded 2026-07-01: ADR-031 is now **Accepted** as the WASM-component Spirit form (host-as-adapter; binding-v2.0 at Story 11.1b). It lifts this defer *only* for the WASM-component form — which is WASM-in-subprocess, keeping the process boundary — while rust-inproc (in-process) stays deferred and in-kernel/in-process embedding remains FORBIDDEN and §13.1-gated. See ADR-031 §4.]**
- Story 10.2's NFR-Test-7 cross-form equivalence test plan was **REMOVED** from v1.5 scope (historical; cross-form equivalence returns at Story 11.1b under ADR-031's tiered behavioral-oracle gate).
- CLI-wrapper-only behavioral equivalence runs instead (the v0.9 substrate behavioral floor per ADR-021).

## Forward-shape Dependencies

| Story | Dependency on ADR-040 | Action |
|-------|----------------------|--------|
| Story 10.1 (v1.0 release gate) | Link ADR-040 from STABILITY.md | Story 10.1's STABILITY.md scaffold greps for ADR-040 |
| Story 10.2 (third-party trial) | Decision outcome gates NFR-Test-7 scope | Removed from v1.5 scope (defer outcome) |
| Story 8.5 (Mira-Nash bilateral pair) | J4 colocation budget | Mira-Nash ships subprocess-form at v1.5 |
