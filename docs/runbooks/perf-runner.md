# Runbook — the stable performance runner (`maos-perf`)

Design authority: `docs/adr/ADR-068-stable-performance-runner.md` (ACCEPTED 2026-09-25). Code:
Story `20-7-stable-runner-perf-gates`. Provisioning owner: `ops-stable-perf-runner` (operator).
Until the host exists every Tier W verdict reads `ABSENT`, never `PASS`
(`docs/runbooks/provisioning-checklist.md`, row *stable performance runner*: `absent`).

This runbook is what the operator follows. Nothing here is simulated by the repository: no
fingerprint, calibration or ledger entry exists until the host produces one.

## 1. What runs where

| Tier | Where | Trigger | Gate |
|---|---|---|---|
| C — counts | GitHub-hosted `ubuntu-24.04`, fork-safe | every pull request | `check-perf-counts` (`discipline.yml`) |
| W — wall clock | the `maos-perf` host only | `push` to `main`, nightly `schedule`, weekly soak, `workflow_dispatch`, same-repository `pull_request` | `check-perf-verdict` on the ledger |

`.github/workflows/perf.yml` is the only workflow that may name `maos-perf`; `check-perf-runner-policy`
(blocking) refuses a second workflow, a missing fork guard, `pull_request_target` (or any trigger
outside `push`-to-`main` / `schedule` / `workflow_dispatch` / `pull_request`) and permissions wider
than `contents: read` on a host job.

## 2. Host setup (ADR-068 §2–§4)

1. Rent single-tenant bare metal: x86_64, ≥ 16 physical cores, NVMe, ≥ 64 GB RAM. Ubuntu 26.04 LTS,
   kernel package held (`apt-mark hold linux-image-*`).
2. Boot-time tuning, applied by root-owned units, never by a job: turbo/boost off, `performance`
   governor, deep C-states limited, SMT off; `isolcpus`, `nohz_full`, `rcu_nocbs` on the benchmark set
   (for example 12 of 16 cores); IRQs and the runner agent on the housekeeping cores
   (`irqbalance` off); swap off; transparent huge pages `madvise`; benchmark filesystem `noatime`;
   no unattended upgrades; the clock slews, never steps.
3. Unprivileged runner account without sudo, one job at a time, label `maos-perf`; job hooks
   `ACTIONS_RUNNER_HOOK_JOB_STARTED` / `ACTIONS_RUNNER_HOOK_JOB_COMPLETED` wipe the workspace and
   temporary directories; outbound-only firewall (GitHub and the package registries); key-only SSH.
4. Rootless podman and the signed T3 image lock on the host, for the weekly NFR-Perf-8 soak
   (operator prerequisite O4).
5. Repository settings: require approval for fork pull-request workflows (defence in depth; the fork
   guard is the control); create the `perf-ledger` branch (empty `ledger.jsonl`), writable by the
   GitHub-hosted `append` job only.
6. Register the runner with label `maos-perf`, then set the repository variable
   `MAOS_PERF_RUNNER_ENABLED=true`. Until it is set no host job queues. Update
   `provisioning-checklist.md` (row *stable performance runner*) in the same change.

The host holds no write token and no repository secret. Results leave as an artifact; the hosted
`append` job appends them to `perf-ledger`.

## 3. Boot-time tuning checks (each run, before calibration)

`perf.yml` records, into the fingerprint, the state each setting actually reached; compare against
the baseline fingerprint before trusting a run: CPU governor, `no_turbo`/`boost`, SMT control,
transparent-hugepage mode, the `isolcpus`/`nohz_full`/`rcu_nocbs` command-line words, thermal-throttle
counters read before and after the run (a throttled run is not evidence — investigate before
re-running).

## 4. Fingerprint fields (ADR-068 §5)

CPU model and microcode, governor/boost/SMT/THP state, the isolated set, kernel (`uname -r`),
`rustc -V`, RAM total, NVMe model and firmware; the step hashes them (`sha256`, first 16 hex) into
every ledger record. A provider hardware or firmware change is a new fingerprint.

## 5. Calibration (ADR-068 §5)

`cargo run --release -p xtask -- perf-calibrate --sha <SHA> --fingerprint <hash> --reference perf-ledger/ledger.jsonl`
times a fixed CPU workload and a fixed 4 KiB-write-plus-fsync workload 30 times each. A coefficient
of variation over 2 % (`calibration_cv_max_pct`), or a median more than 3 % from the machine's
recorded calibration (`calibration_drift_max_pct`; the reference is the fingerprint's earliest
calibration record), makes every verdict of that run `INDETERMINATE`. Each benchmark then runs five
times as separate processes; a metric is the median of the five per-run quantiles, and a spread over
its `noise_tolerance` is `INDETERMINATE`.

## 6. Verdicts and the ledger

`cargo run -p xtask -- check-perf-verdict --sha <SHA> [--ledger <file|dir>]` prints `PASS`, `FAIL`,
`INDETERMINATE` or `ABSENT` per Tier W metric of `xtask/perf-budgets.toml`; only `PASS` is green.
`FAIL` needs the median over budget, or over the baseline by more than the metric's
`regression_tolerance` **and** more than three times the baseline's MAD. The baseline is the median of
the last ten `PASS` runs on `main` for the fingerprint. A missing ledger or branch is a loud error.

Every row is `advisory` (shadow) until step 7; a non-`PASS` advisory row prints
`WOULD BLOCK once binding` and does not fail the run.

## 7. Shadow, then bind (ADR-068 §7)

Shadow ≥ 2 weeks and ≥ 20 `perf.yml` runs on `main`, then one commit replaces the opening
tolerances with measured ones (confidence rule 6) and flips the dispositions to `blocking` for the
current phase. That is Story 20-7 landing 2; it needs this host.

## 8. Maintenance window and re-baseline

OS, kernel, firmware and hardware changes happen in a maintenance window, followed by a calibration
run. A deliberate change of speed, or a new fingerprint, re-baselines through a pull request that
states the measured figure and the reason in the same commit: a `rebaseline` ledger record
(`{"kind":"rebaseline","fingerprint","metric","median","mad","reason"}`) for Tier W, a changed
`baseline` row in `xtask/perf-budgets.toml` for Tier C.

## 9. Ledger staleness

`perf.yml`'s `ledger-staleness` job reds when the newest ledger entry is older than 48 hours
(`check-perf-verdict --stale-check`). Response: check the runner service and its last job; check that
`MAOS_PERF_RUNNER_ENABLED` is still `true`; check for a stuck `concurrency: perf-runner` run; if the
host is down, the ledger stays red — never wave it green; restore the host and run
`workflow_dispatch`.
