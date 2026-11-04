## Verdict

**KNOWN harness/timing family, newly observed in S5; not evidence of a 20-4/20-10 production regression.** The two rows are best explained by **one BudgetWarning plus one BudgetExceeded**, both misclassified by S5’s `ratio_breached >= 1` filter. There is also a concrete, previously diagnosed **process-global clock reinitialization hazard** in this test binary.

**Confidence:** high on the faulty oracle and absence of a changed production budget path; moderate-high that clock reinitialization explains the inflated timestamps. The log omits frame kinds and clock-reset instrumentation, so that last attribution remains **[INFERENCE]**, not a captured runtime trace.

### 1. What produces the two rows

- S5 injects a **3 s blocking sleep with a 1 s cap** (`crates/maos-bin/tests/worker_terminal_facts_17_7/unload_scenes.rs:510-518`). `WorkerSpirit::on_unload` sleeps, then stops the session; it does not recurse into unload (`crates/maos-bin/src/supervision.rs:2622-2632`).
- The dispatcher deliberately emits **BudgetWarning at the 80% timer**, followed by **BudgetExceeded on timeout** (`crates/maos-kernel-core/src/scheduler/hook_dispatch.rs:587-611,661-679`). Both fallback rows use intent `hook.budget.on_unload`, with their actual sampled elapsed clock value divided by the cap (`:247-267`).
- S5 drops `row.kind` in `World::rows()` (`unload_scenes.rs:121-134`) and distinguishes warning/exceeded solely by ratio (`:409-423`). **A warning is not guaranteed to have ratio < 1.** Delayed scheduling or a clock jump can invalidate that assumption.
- Failed-job log **L4760-4763** contains:
  - `wall_ns=2422539097`, ratio `2.422539097`
  - `wall_ns=2622919372`, ratio `2.622919372`
  
  Their separation is **200.380275 ms**, closely matching the 800 ms warning → 1 s timeout interval. This supports the warning/exceeded pair, not a retry.
- Unload snapshots each PID once; scheduler unload removes the SCB before firing the hook (`supervision.rs:148-162`; `scheduler_loop.rs:531-580`). No retry/second invocation is needed to explain this failure. The hook pump only dispatches telemetry/consolidation, not unload (`hook_pump.rs:189-201,435-446`).

**Important clock evidence:** both `unload_scenes.rs:27` and `supervisor_scenes.rs:52` call `init_monotonic_base()` per World. That function overwrites `MONOTONIC_BASE` while preserving the first `BOOT_INSTANT` (`crates/maos-capability/src/cap_tokens/mod.rs:45-73`). Concurrent test setup can therefore jump reported elapsed time forward. An approximately **1.622 s offset** converts ordinary warning/timeout samples near 0.8/1.0 s into this observed pair. Consequently, **2.42 s is not proof of 2.42 s actual timer latency**.

### 2. Candidate-commit attribution

`git diff 8c809f9c ef134444 --stat`: **285 files, +110293/-343**, predominantly evidence.

Targeted diffs show **no changes** to kernel-core, capability clocks, IAC, `supervision.rs`, `worker_spawn.rs`, `hook_pump.rs`, or `unload_scenes.rs`.

- **2752e750:** provider/admission/root wiring; hook-pump and observer tests only gain `providers: None`. S5 constructs its own World and directly calls `run_cli_wrapper_manifest`; it does not exercise the new provider admission path.
- **ef134444:** root-crash fixture gets independent target/foreign temporary HOME ancestry. It can perturb scheduling, but does not change S5’s budget semantics.

**No plausible direct causal regression found; incidental scheduling changes remain possible.**

### 3. Job load and other failures

Failed job `126484785052` log, retained as `artifact://13284`:

- **L5598-5601:** runtime **7m45.638s**, CPU **15m48.296s**, memory peak **7.7G**, swap **0B**. Substantial resource use, but not proof of memory pressure or OOM.
- **L4630-4638:** `worker_freeze_19_2`’s six tests passed in **52.97 s**.
- **L4770:** terminal suite finished **32 passed / 2 failed in 37.13 s**.
- The other failure is **root-crash custody retention**, at `root_crash.rs:280`, log **L4746-4750**—separate from S5, not another budget flake.
- `one_budget_overrun_quarantines_the_subscriber` passed; hook-pump suite **26/26, 7.79 s** (**L3926-3929**).
- Observer activity passed; suite **16/16, 1.24 s** (**L4108-4110**).
- No “running over 60 seconds” notices appeared. This log does **not** show widespread timing failures.

### 4. Classification and recommended disposition

The strongest precedent is **`deferred-work.md:1262-1263`**: the same loom/libtest **World clock-reinitialization defect**, repaired in hook-pump by 18-0. Its existing pattern is explicit at `crates/maos-bin/tests/hook_pump_17_4.rs:60-65`.

Other related but **not identical** open families are `deferred-work.md:4,27` (hook overrun/observer timing). Row **126** is a different operator-door unload route-budget failure, identified in `18-3-evidence/plants.md:28-31`; do not claim it already lists S5.

**Prefer a narrow test repair over record-only “heavy-load flake” disposition:**

1. Share **one clock-init `Once` across the terminal test binary**, replacing both per-World calls at `unload_scenes.rs:27` and `supervisor_scenes.rs:52`.
2. At `unload_scenes.rs:411-423`, count **`FrameKind::BudgetExceeded`**, preserving the exact-one assertion. Retain/query frame kinds rather than guessing from ratios; precedent: `crates/maos-kernel-core/tests/budget_seam_8_11.rs:154-173`.
3. Record this occurrence and diagnosis under **20-7**, with same-line rule-10 evidence following `deferred-work.md:4` and citing the clock-family precedent `:1263`. Do not widen budgets, serialize the isolation gate, retry-to-green, or label the overall job green—the independent custody failure remains.

Read-only investigation; no edits, cargo execution, or verification reruns.