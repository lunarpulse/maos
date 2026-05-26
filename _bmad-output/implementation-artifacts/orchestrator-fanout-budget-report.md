# NFR-Perf-8 — Orchestrator fan-out budget report

Story 6.2 AC3 — sustained 50 concurrent Worker Spirits / 10 tasks/sec / 1h /
P99 ≤500ms / 0 dropped tasks.

Bench: `crates/maos-bench/benches/orchestrator_fanout_nfr_perf_8.rs`.

## Methodology

- In-process fan-out at v0.5-α (subprocess CliWrapperSpirit fan-out lives in a
  separate bench per AC6 §Bench-Note).
- `tokio::time::interval(Duration::from_millis(100))` with
  `MissedTickBehavior::Skip` to enforce 10-tasks/sec sustained semantics
  (catch-up bursts violate the rate floor).
- `Arc<Semaphore::new(50)>` enforces the 50-concurrent floor as a hard limit.
- Per-dispatch latency = `IacBusAdapter::deliver_typed(task.assign)` start →
  Worker's `recv` return.

## Calibration phase (v0.5-α)

The bench currently runs in **soft-fail mode** (`continue-on-error: true` on
`nfr-perf-1-iac-routing-budget` + `nfr-perf-8-orchestrator-fanout`) per the
§13.1 runner-tier calibration window. The kernel_measurement feature surface
inherited from Story 5.5e drifted before this story — section_13_1 harness
restoration is tracked separately. Flip to hard-fail in a follow-up PR before
Epic 6 closes per `[[feedback_mechanical_gates_compound_promises_decay]]`.

## Run records

| run | git_sha | runner_tier | fan_out | P50_us | P95_us | P99_us | P99.9_us | dropped | breach |
|---|---|---|---|---|---|---|---|---|---|
| _initial_ | untracked | calibration | 50 | tbd | tbd | tbd | tbd | tbd | tbd |

_Records appended automatically by the bench at run time._
