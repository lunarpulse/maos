---
Status: ACCEPTED — ratified 2026-09-25 by the operator under Story 17-3a follow-up decision D7 (approved in principle, then ratified as designed; sourcing: a rented dedicated server, §2)
Gate: none yet — `20-7-stable-runner-perf-gates` lands `check-perf-verdict` and `check-perf-runner-policy`; this file's index row is held by `cargo test -p xtask --test decision_adrs_and_provisioning`
Decided: 2026-09-25
Accepted-in-PR: pending — operator ratification 2026-09-25 (Story 17-3a follow-up D7)
Revisits: ADR-040 only for where its §13.1 budgets are enforced — its `Status:` line is read by `check-adr-040-accepted` and does not change; ADR-030's 5 µs hot-path figure gains a machine it can be measured on
Supersedes: no ADR — replaces the per-job "ADVISORY … wall-clock-sensitive on shared GH runners" posture at `.github/workflows/discipline.yml:1696,1748,1766`
---

# ADR-068 — A stable performance runner, and performance verdicts that cannot pass silently

## Context

MAOS makes eight numeric performance promises (NFR-Perf-1…8,
`_bmad-output/planning-artifacts/prd/non-functional-requirements.md:9-16`) and three §13.1
journey budgets (ADR-040). Measured at `92911f59`, none of them has an instrument that can fail
for the right reason:

- Every performance job runs on `ubuntu-latest`; the repository has no self-hosted runner
  (`docs/runbooks/provisioning-checklist.md:47-49`).
- The three wall-clock jobs meant to gate — `nfr-perf-1-iac-routing-budget`,
  `nfr-perf-j4-latency` and `nfr-perf-8-orchestrator-fanout` (`discipline.yml:1693-1778`) — are
  `continue-on-error`, and their code never fails on a breach
  (`crates/maos-bench/benches/iac_routing_budget.rs:153-172`, `j4_latency_regression.rs:36-78`).
  NFR-Perf-8 asks for one hour; its bench runs a 15-second window
  (`orchestrator_fanout_nfr_perf_8.rs:62-68`). NFR-Perf-1 asks for P50 < 5 ms and P99 < 50 ms;
  its bench checks a 1 ms P95 it calls a "soft-fail calibration floor" (`iac_routing_budget.rs:37`).
- `check-j4-latency` is registered `v1_5 = "blocking"` (`xtask/gate-registry.toml:336-337`) at
  `CURRENT_PHASE = "v1_5"` (`xtask/src/gate_common.rs:166`), yet over budget its test prints
  *WOULD BLOCK at v1.5* and passes; its own comment says no phase-aware enforcer exists
  (`crates/maos-bench/tests/t_10_4c_j4_latency_gate.rs:76-102`).
- Two bench jobs say they fail "on a regression vs. the baseline" (`discipline.yml:1024,1045`).
  They run each bench once (`-- --test`), and no performance baseline exists anywhere in the tree.
- Budgets are constants copied by hand: J4's 10 ms lives in
  `crates/maos-bench/src/harness/j4.rs:36`, `crates/maos-bench/src/decision.rs:15` and
  `crates/maos-bench/benches/j4_latency_regression.rs:25`.
- NFR-Perf-2, -4, -5, -7 and -8 map to no gate (`tests/coverage-matrix.yaml:1335-1374`); NFR-Perf-1
  maps to `check-j4-latency`, which measures telemetry delivery, not IAC routing (`:1326-1334`).
- The hardware is already named: NFR-Perf-1's "NVMe + 16-core tier", and J1's strict 25 ms P95,
  "hard-gated on a pinned bench runner" (`crates/maos-bench/src/harness/j1.rs:183-190`). That
  runner does not exist, so CI uses a 50 ms ceiling instead (`:191`).
- Shared-runner noise is measured, not assumed: one commit's cold build took 267 s and 316 s on
  consecutive runs (`tests/integration/onb_nfr2_timing.sh:11-35`), and `check-multi-region-slo`
  went red one run in seven from tail stalls
  (`_bmad-output/implementation-artifacts/deferred-work.md:1005`).
- The repository is public. A self-hosted runner that executes pull-request code from a fork is
  remote code execution on that machine.

## Decision

### 1. Two tiers, split by what noise can move

- **Tier C — counts, every pull request, GitHub-hosted.** Deterministic cost metrics for the
  CPU-bound hot paths (capability-token verify, IAC routing, frame lowering): instruction counts
  under Callgrind and heap-allocation counts, compared with a committed baseline. Noise cannot move
  them, so they may block any pull request, including one from a fork.
- **Tier W — wall clock, trusted code only, the stable runner.** Latency percentiles and throughput
  against the NFR-Perf and §13.1 budgets. This is the only place a latency number is believed.

Tier C catches most regressions before merge; Tier W decides whether MAOS keeps its promises.

### 2. The machine

- Dedicated bare-metal x86_64 — not a VM and not a VPS: a shared hypervisor is the noise this ADR
  removes.
- At least 16 physical cores and NVMe storage (NFR-Perf-1's reference tier) and at least 64 GB of
  RAM; a second NVMe for benchmark I/O when affordable.
- Ubuntu 26.04 LTS server with the kernel package held. It is the release `ubuntu-latest` moves to
  in November 2026 (actions/runner-images#14748), so CI and the performance host converge.
  Upgrading it is a re-baseline event (§6).
- Sourcing: a **rented dedicated server** — single-tenant bare metal from a hosting provider, never
  a VPS or a cloud instance (operator decision 2026-09-25; provisioned by `ops-stable-perf-runner`).
  Its firmware settings are usually out of reach, so §3's tuning is applied through the kernel
  command line and sysfs, and the fingerprint (§5) records the state each setting actually reached.
  A provider hardware or firmware change is a new fingerprint, and therefore a re-baseline event (§6).

### 3. Host tuning — applied at boot by root-owned units, never by a job

- Fixed frequency: turbo/boost off, `performance` governor, deep C-states limited, SMT off.
- Core isolation: `isolcpus`, `nohz_full` and `rcu_nocbs` on the benchmark set (for example 12 of
  16 cores). The runner agent, the OS and every IRQ stay on the housekeeping cores (`irqbalance`
  off, IRQ affinity pinned); benchmark processes run pinned to the benchmark set.
- Memory and storage: swap off; transparent huge pages set explicitly (`madvise`); the benchmark
  filesystem mounted `noatime`; `fstrim` only in the maintenance window.
- A quiet host: no unattended upgrades and no other services; the clock slews, never steps, during
  a run; thermal-throttle counters are read before and after each run.

### 4. Security — the runner only ever runs trusted code

- `.github/workflows/perf.yml` is the only workflow that may target the runner label `maos-perf`.
  It triggers on `push` to `main`, `schedule`, `workflow_dispatch`, and `pull_request` **only** when
  the head repository is this repository — never a fork (a job-level `if:`) — and never on
  `pull_request_target`.
- `check-perf-runner-policy` (xtask) enforces that, proven red by planting each violation: a second
  workflow that uses `maos-perf`, a removed fork guard, an added `pull_request_target`.
- The runner account is unprivileged, with no sudo. `perf.yml` holds `permissions: contents: read`;
  results leave the host as a workflow artifact, and a GitHub-hosted follow-up job appends them to
  the ledger, so the host never holds a write token or a repository secret.
- Runner job hooks (`ACTIONS_RUNNER_HOOK_JOB_STARTED`, `ACTIONS_RUNNER_HOOK_JOB_COMPLETED`) wipe the
  workspace and temporary directories before and after every job.
- Network: outbound only to GitHub and the package registries the build needs; inbound only the
  operator's key-only SSH or a management VPN. The repository requires approval for fork
  pull-request workflows as defence in depth; the fork guard is the control.

### 5. Measurement protocol — no silent pass

- One job at a time on the machine.
- **Calibration first.** A fixed CPU workload and a fixed fsync workload, timed 30 times each. If
  either coefficient of variation exceeds 2 %, or either median drifts more than 3 % from the
  machine's recorded calibration, every verdict in that run is `INDETERMINATE`. These are opening
  values; the shadow phase (§7) measures and replaces them (confidence rule 6).
- **Repetition.** Each benchmark runs five times as separate processes after warm-up. A metric's
  value is the median of the five per-run quantiles; when the spread (max − min) / median exceeds
  the metric's noise tolerance, the metric is `INDETERMINATE`.
- **Verdicts** follow `20-3a-gate-honesty`'s EvidenceState: `PASS`, `FAIL`, `INDETERMINATE`, and
  `ABSENT` (no run for this SHA). Only `PASS` is green.
- **Budget check** against one file, `xtask/perf-budgets.toml` — per metric: NFR or §13.1 id,
  quantile, budget, unit, noise tolerance, regression tolerance and per-phase disposition. No budget
  constant lives anywhere else.
- **Regression check** against the ledger baseline for the same machine fingerprint: `FAIL` only
  when the median exceeds the baseline by more than the metric's tolerance **and** the change
  exceeds three times the baseline's median absolute deviation, so noise alone cannot fail a run.
- **Fingerprint:** CPU model and microcode, core/SMT/turbo/governor state, the isolated set, kernel,
  `rustc`, RAM, NVMe model and firmware, hashed into every record. A new fingerprint makes the
  regression checks `INDETERMINATE` until a re-baseline entry exists.

### 6. Ledger and baselines

- Results append to a `perf-ledger` branch as JSON lines (SHA, fingerprint, metric, the five
  per-run values, verdict) — the pattern `rto-ledger` already runs
  (`docs/runbooks/provisioning-checklist.md:50-52`). The collector fails loudly when the branch is
  missing.
- A metric's baseline is the median of its last ten `PASS` runs on `main` for that fingerprint. A
  deliberate change of speed re-baselines through a pull request that states the measured figure
  and the reason in the same commit, the discipline kloc grants already follow.
- OS, kernel and hardware changes happen in a maintenance window, followed by a calibration run and
  a re-baseline entry.
- A scheduled GitHub-hosted job reds when the newest ledger entry is older than 48 hours, so a dead
  machine is loud, never green.

### 7. Migration

1. **Provision** (`ops-stable-perf-runner`, operator lane): the machine per §2–§4, registered with
   the label `maos-perf`; its fingerprint and calibration report are committed with the runbook.
2. **Shadow** for at least two weeks and 20 runs on `main`: everything in the table runs and
   records verdicts without blocking, to measure the noise floor and set tolerances from data.
3. **Bind**: per metric, the disposition in `xtask/perf-budgets.toml` for the current phase
   decides. A `FAIL` on a trusted pull request blocks its merge, and a tag is cut only from a SHA
   whose verdicts are `PASS` (`xtask/src/check_release_precondition.rs` reads the ledger), so a
   regression that reached `main` still blocks the release.
4. **Retire**: the three advisory wall-clock jobs (`discipline.yml:1693-1778`) are deleted with their
   `continue-on-error`; the single-iteration `-- --test` bench jobs stay as smoke under honest
   names, with no regression claim.

| Metric | Budget (source) | Today | On the stable runner |
|---|---|---|---|
| IAC routing | P50 < 5 ms, P99 < 50 ms (NFR-Perf-1) | advisory; 1 ms P95 soft floor | every push to `main`, at the NFR's own quantiles |
| J4 `scalar.tap` | P95 ≤ 10 ms (ADR-040) | registered blocking, never blocks | binding; the hosted job keeps only its sample-count and mutation checks |
| J1 bridge | P95 ≤ 25 ms (`j1.rs:183-190`) | 50 ms ceiling on shared runners | the strict budget; the 50 ms ceiling stays as a catastrophe guard |
| J0 in-process | P95 ≤ 60 ms | asserted on shared runners | also here, for the trend |
| Capability-token verify | P99 < 100 µs (NFR-Perf-3); < 5 µs hot path (ADR-030) | asserted in `workspace-test-suite` | here, with the 5 µs figure |
| Hot swap | P99 < 500 ms (NFR-Perf-7) | the bench exists; no job runs it | the existing bench, wired |
| Fan-out | 50 Workers, 10 tasks/s, 1 h, P99 ≤ 500 ms, 0 dropped (NFR-Perf-8) | 15 s window, never fails | a weekly one-hour soak |
| Throughput | 5,000–10,000 frames/s (NFR-Perf-2) | no bench | a new bench |
| Posture shift | P99 ≤ 2 s, P99.9 ≤ 5 s over 1000 shifts (NFR-Perf-4) | no bench | a new bench |
| Audit query | P99 ≤ 2 s single-Spirit, ≤ 10 s global (NFR-Perf-5) | correctness-only bench | a sized fixture and a budget |

Until step 3, `20-3a-gate-honesty` (pulled forward) records what exists honestly: `check-j4-latency`
cannot be `blocking` while its job cannot fail, and the "regression vs. the baseline" wording comes
off jobs that have no baseline.

## Alternatives rejected

- **Shared runners with wider tolerances.** J1's CI ceiling is already twice its budget; a
  tolerance wide enough not to flake is wide enough to miss a real regression.
- **GitHub larger runners for Tier W.** More cores, the same shared hypervisor and no frequency
  control: lower noise, not controlled noise. Acceptable later for Tier C capacity.
- **Counts only.** Deterministic, but blind to I/O, scheduling, locks and tail latency — what the
  NFRs actually promise.
- **Fork pull requests on the runner inside a VM or container.** It adds the virtualisation noise
  this ADR removes and bets the host on an escape-free sandbox; forks get Tier C instead.

## Consequences

- A performance number becomes evidence: one machine, one protocol, one budget file, a history.
- A regression that a trusted pull request did not measure is caught after merge; the release
  precondition is the backstop.
- One machine is a single point of failure for Tier W evidence; that failure reads `ABSENT` or
  stale, never green.
- The operator lane carries the rental, the host's upkeep and its maintenance windows;
  `20-7-stable-runner-perf-gates` carries the code.
