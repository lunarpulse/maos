# 20-7 creation — records owed

Rulings: none yet for this story's rows. D01 A / D02 A / D11 A (operator, 2026-10-30) and D13 A (2026-11-01, rows203c)
govern; D-20-7-01…08 are PROPOSED. Line numbers are at `a4c00897` unless marked "post-edit". This creator edits only its
story file, its evidence directory and its own tracker line; everything in §A and §B is for the orchestrator to apply.

## Applied at creation (Create207 owns these files this session)

| # | File:line | Edit |
|---|---|---|
| X1 | `_bmad-output/implementation-artifacts/20-7-stable-runner-perf-gates.md` | new story, `ready-for-dev` |
| X2 | `_bmad-output/implementation-artifacts/20-7-stable-runner-perf-gates-evidence/creation/` | this directory |
| X3 | `_bmad-output/implementation-artifacts/sprint-status.yaml:304` | `20-7-stable-runner-perf-gates: backlog` → `ready-for-dev` with the creation note; the prior note kept after `WAS:` |

Not applied (shared line): `sprint-status.yaml:1` `last_updated: 2026-10-31` → `2026-11-01` (orchestrator; another creator edits the same file).

## §A — epic corrections owed to `epic-20-ship-it-w5.md` (orchestrator applies; OLD → NEW in `cites-a4c00897.md`)

| # | E20 line | Edit |
|---|---|---|
| A1 | `:79` (stories row) | "kernel-Δ 0 · xtask +250–450 (zero headroom, measured grant) · maos-bench +150–300" → append creation note: "created 2026-11-01 @a4c00897 — ready-for-dev; kernel FLAG-Winston R = K1w + K2r priced +40/+31 by reverted prototype (D-20-7-01/02 PROPOSED; 0 only if both rule a no-kernel option); xtask +560…+990, maos-bench src +60…+200, maos-bin 0…+40 forecast; L1 machine-free / L2 after `ops-stable-perf-runner` (D-20-7-05)" (C14, C15) |
| A2 | `:238` | `tests/coverage-matrix.yaml:1326-1374` → `:1380-1436`; `crates/maos-bench/src/harness/j1.rs:183-190` → `:25,162-170`; "kernel-Δ 0" → as A1 (C4, C6, C14) |
| A3 | `:240` | `discipline.yml:2940-3013` → `:2970-2983,3022-3035,3040-3055`; J4 `:4891-4931` → `check-j4-latency` `:4856-4896`; coverage `:1364-1419` → `:1380-1436` (C1, C3, C5) |
| A4 | `:242` | `deferred-work.md:1084-1094,1152-1154,1192` → `:96,1075,1181-1198,1256-1259,1297-1298`; add "measured at 20-7 creation: S3 and ac4 are budget-sized stalls with defective cleanup/records, not calibration (`20-7-…-evidence/creation/flake-diagnosis.md`); D13 RULED A 2026-11-01" (C13, C16, P6) |
| A5 | `:244` | `discipline.yml:1693-1778` → as A3; "`check-j4-latency` is registered blocking at the current phase while its test passes over budget (`…t_10_4c_j4_latency_gate.rs:76-102`)" → "was registered blocking (repaired by 20-3a AC7: `gate-registry.toml:457-459` advisory/advisory-substrate); its test prints over-budget as advisory (`:70-92`)" (C2, C10, C11) |
| A6 | `:248` (AC3) | add the fourth copy: J1 25 ms at `crates/maos-bench/src/decision.rs:14` (C7) |
| A7 | `:249` (AC4) | `iac_routing_budget.rs:37,153-172` → `:36-37,122,157-170`; "the existing `hot_swap_latency` bench" → "`crates/maos-kernel-core/benches/hot_swap_latency.rs`" (C8, C9) |
| A8 | `:252` (AC6) | add: "`check_release_precondition.rs` reads only the check-runs `aggregate` today (`:33-60`); the ledger read is new" (C12) |
| A9 | `:253` (AC7) | `discipline.yml:1693-1778` → as A3 (C2) |

## §B — ledger sub-bullets owed to `deferred-work.md` (orchestrator applies; no owner line changes, no closure)

| # | Row | Sub-bullet |
|---|---|---|
| B1 | `:1181` | cite `worker_terminal_facts_17_7.rs:419-434` → bound `:415-425`, assertion `:426-430` (L1) |
| B2 | `:1190` | failure text: `10.039739216s (bound 5s, runtime podman version 4.9.3)`, `residue TerminationUnconfirmed`, client `Exited { code: 125 }` (`20-7-…-evidence/creation/ci-excerpts/run-41243340929-job-124661589790.txt`) (L3) |
| B3 | `:1191` | failure text: `10.052044568s …`, `TerminationUnconfirmed`, client 125 (`ci-excerpts/run-41281999213-job-124775636112.txt`) (L4) |
| B4 | `:1198` | "Measured at 20-7 creation (2026-11-01): not calibrations — bimodal one-budget stalls; S3 teardown skips `rm` on a stalled snapshot (`child.rs:418-422`); ac4 root exit ≈ 10 s. `20-7-…-evidence/creation/flake-diagnosis.md`." (C16) |
| B5 | `:1256` | "already recorded at `:1158`" → `:1192` (L2) |
| B6 | `:1259` | failure text: "the standalone Worker (pid 32438 / 61396) is still in /proc after 5s"; nextest `FAIL [16.772s]` (L5) |
| B7 | `:1298` | failure text: `P99 = 5480ns, budget = 5000ns` (L6) |
| B8 | `:1197` | supervisor recurrence: also "`supervision.rs:2460:38` (@43ce3ea7) Cannot start a runtime from within a runtime — `WorkerBinding::drop` → `unload` → `block_on` during unwinding" (`ci-excerpts/run-41013625894-job-123924858906.txt`) |
| B9 | `:1193` | "job logs need a token this host does not have" → superseded 2026-11-01: the operator's gh token on the dev host reads job logs (L9) |

## §O — operator prerequisites (never faked; the story records `ABSENT` until each exists)

| # | Prerequisite | Needed by | Owner | State at creation (2026-11-01) |
|---|---|---|---|---|
| O1 | Rented single-tenant bare-metal Tier W host per ADR-068 §2–§4 (≥ 16 physical cores, NVMe, ≥ 64 GB, Ubuntu 26.04 LTS kernel held; boot-time tuning; unprivileged runner, one job at a time, label `maos-perf`, job hooks wiping workspace/tmp, outbound-only firewall, key-only SSH) + fingerprint and calibration report | L2 (AC1.4, AC6.1 soak, AC11, AC12.1) | Lunarpulse (`ops-stable-perf-runner`, `sprint-status.yaml:327`; rental, money, network) | absent: `gh api repos/lunarpulse/maos/actions/runners` → 0; `provisioning-checklist.md:44` `absent` |
| O2 | `perf-ledger` branch created and writable by the GitHub-hosted append job only (precedent `rto-ledger`, `.github/workflows/rpo-rto-cadence.yml:17-30`) | AC1 (live), AC11 | Lunarpulse (repository admin) | absent (`git ls-remote origin` lists `fuzz-ledger`, `rto-ledger` only) |
| O3 | **OPERATOR setting, L1 security correction:** `perf-host` environment restricted to protected main with required reviewers; runner group restricted to this repository's `.github/workflows/perf.yml@refs/heads/main`. Verify environment deployment-branch/reviewer settings and runner-group selected-workflow/ref settings via UI/API export; a non-main dispatch must be denied. A PR-editable fork guard is not admission. | before enabling any real perf runner | Lunarpulse (repository/organization admin) | unverified: no settings export or non-main denial witness supplied |
| O4 | Rootless podman + the signed T3 image lock/trust anchor on the perf host, for AC6's Tier W soak (D-20-7-06 A only) | L2 AC6.1 | Lunarpulse (host); image trust anchor with `ops-live-worker-under-17-1-profile` | absent (host absent) |
| O5 | Runner registration (token, label) and the maintenance-window calendar the runbook names | L2 | Lunarpulse | absent |
| O6 | Hosting-provider account and billing for the rental | O1 | Lunarpulse | not inventoried (PF:137 "current secret/host availability unmeasured") |
| O7 | **OPERATOR setting:** require approval for fork pull-request workflows from all outside collaborators. Verify Settings → Actions → General (or repository Actions permissions API export), plus a fork PR awaiting approval. Additional defense, not a substitute for O3. | before enabling any real perf runner | Lunarpulse (repository admin) | unverified: settings not queried in this correction assignment |
| — | Fuzz-ledger access | **not a 20-7 input** (20-5's T3 24 h chain, PF:124) | — | `fuzz-ledger` branch exists |
| — | CI-log read token | diagnosis only | — | available on the dev host (all 14 logs fetched); `provisioning-checklist.md:45` row is the orchestrator's to update |

Tracker owed (orchestrator): `sprint-status.yaml:327` (`ops-stable-perf-runner`) append "20-7 created 2026-11-01: L1 needs nothing from this row; L2 needs O1–O5 (`20-7-…-evidence/creation/records-owed.md` §O)".
Review correction: L2 prerequisites now include **O3 and O7** as documented in `docs/runbooks/perf-runner.md`; repository code does not configure or prove these operator settings.

## Records owed after the rulings (not now)

- Story: replace "PROPOSED" with the ruled option per row; rewrite only the clauses marked with that row; change log; frontmatter `kernel_grant`/`kloc_grant` to the granted ceilings.
- FLAG-Winston decision record for the ruled kernel bundle (precedent `20-3c-flag-winston-decision.md`), stating ADR-069's remaining room (R reaches 21028, under 22000).
- Under D-20-7-05 B: a new tracker key and E20 section for L2 (orchestrator); this story's AC1.4/AC11/AC12.1/AC13 close move there.
- Under D-20-7-01 C: an epic-17 D6 rule amendment (podman-4 evidence retired) — operator.
- Tracker `:304`: append the ruling ask id.

## Records owed at landing (not now)

- `xtask/kloc.toml:264` (kernel), `:339` (xtask), `:520` (maos-bench), `:517` (maos-bin), `:718` (aggregate, measured + 1) — exact after `cargo fmt --all` / `kloc-check --json`, HISTORY rows citing this key.
- `xtask/kernel-core-baseline.toml:505` (+ EOF HISTORY, content hashes `security/sandbox/t3/child.rs`).
- `deferred-work.md:96,1075,1181-1191,1192-1198,1256-1259,1297-1298` → `CLOSED <date> by 20-7 @<commit>` with the witness, or rule-10 re-owned (AC9.4); `routing-audit-2026-10-22.md:48` → owned/witnessed by 20-7 @<commit>.
- `tests/coverage-matrix.yaml:1380-1436`; `xtask/gate-registry.toml:457-459` + new gate rows; ADR-068 `Gate:` (`:3`) and `Accepted-in-PR:` (`:5`) lines; `docs/runbooks/perf-runner.md` (new); `docs/runbooks/provisioning-checklist.md:44` (operator, when O1 exists).
- E20 `:79` stories row → in-progress/done notes.
