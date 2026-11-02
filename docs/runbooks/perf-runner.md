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
| W — wall clock | the `maos-perf` host only | `push` to `main`, nightly `schedule`, weekly soak, main-only `workflow_dispatch` | `check-perf-verdict` on the ledger |

`.github/workflows/perf.yml` is the only allowed host workflow. The structural
policy refuses PR/comment triggers, ambiguous selectors, aliases, non-exact
main-ref guards, host secrets and permissions wider than `contents: read`.
This gate is defense in depth, NOT the admission boundary: a PR can edit code.

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
5. **O3 — OPERATOR prerequisite, before enabling the host:** configure the
   `perf-host` environment with deployment branches restricted to protected
   `main`, required reviewers, and prevent self-review/bypass where available.
   Configure the runner group to allow only this repository's
   `.github/workflows/perf.yml@refs/heads/main`; no general repository-wide host
   access. Verify in Settings → Environments → perf-host and organization
   Settings → Actions → Runner groups → selected workflows. Save screenshots
   or API exports of both restrictions; a non-main dispatch must be denied.
   If the account plan cannot enforce workflow/ref scoping, do not enable it.
   Create `perf-ledger` with force pushes/deletions prohibited and writes limited
   to the trusted hosted append job.
   **O7 — OPERATOR prerequisite:** Settings → Actions → General → Fork pull
   request workflows: require approval for **all outside collaborators**.
   Verify with the repository Actions permissions API/UI export and a fork PR
   awaiting approval. This is additional defense, not a substitute for O3.
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

`GITHUB_RUN_ID` and `GITHUB_RUN_ATTEMPT` identify one run/attempt; calibration
and metric records must match both that identity and the fingerprint. Logs
must be named `run-1-*.log` through `run-5-*.log`; each registered bench metric
must appear exactly once in each process run. Malformed, duplicate or missing
observations refuse the entire append. Re-appending an observed SHA is refused,
so a rerun cannot replace a FAIL with PASS.

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

OS, kernel, firmware and hardware changes happen in a maintenance window.
After review, use the trusted main build of `perf-ledger-append --rebaseline
request.json` alongside `--ledger`, `--sha`, `--branch main` and `--fingerprint`.
The request has exactly `metric`, `median`, `mad`,
`reason`, `review` (a GitHub pull-request URL). The append tool alone generates
`writer`, `actor`, `sha`, `run_id` and timestamp provenance from the trusted
main run. Ledger-branch protection is required: provenance is validated, not a
cryptographic defense against an operator who can rewrite that branch.
Median/MAD must be finite and nonnegative. A rebaseline only applies to
measurements at or after its instant; later maintenance never rewrites an old
verdict. Tier C changes its reviewed `baseline` in `xtask/perf-budgets.toml`.

## 9. Ledger staleness

`perf.yml`'s `ledger-staleness` job reds when the newest ledger entry is older than 48 hours
(`check-perf-verdict --stale-check`). Response: check the runner service and its last job; check that
`MAOS_PERF_RUNNER_ENABLED` is still `true`; check for a stuck `concurrency: perf-runner` run; if the
host is down, the ledger stays red — never wave it green; restore the host and run
`workflow_dispatch`.
