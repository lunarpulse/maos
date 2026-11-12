---
story_id: '20.7'
story_key: 20-7-stable-runner-perf-gates
created: 2026-11-01
context_commit: a4c00897
baseline_commit: a4c00897
security_sensitive: true
kernel_grant: "RULED 2026-11-01 (ask rows207; D-20-7-01 A + D-20-7-02 A; kernel row R = K1w + K2r): measured by reverted prototype at a4c00897: +40 physical / +31 tokei (26801 → 26841; 21011 → 21042). Ceilings (STOP above): kernel ≤ 26881 physical / 21073 tokei; pins re-fixed exactly at landing. ADR-069 (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`): no ENVELOPE ALARM row at this measurement. Kernel-Δ 0 is not an option: the kernel row is granted. Kernel interaction with 20-1b (R = KQ1 + P, +78 / +68): both grants were computed from HEAD 26801/21011; whichever lands second re-bases; cumulative measured +68 and +31 → 21110 tokei expected; cumulative tokei ceilings must not exceed 21209 (= 791 under the 22000 ADR-069 alarm); STOP otherwise."
kloc_grant: "RULED 2026-11-01 (ask rows207; D-20-7-08 A; D02 A as ruled 2026-10-30, ×2 measured / ×3 unmeasured). Ceilings (STOP above; pins re-fixed exactly at landing; aggregate hardfail = measured + 1): kernel ≤ 26881 physical / 21073 tokei (re-base rule in kernel_grant); xtask ≤ 45830; maos-bench ≤ 3051; maos-bin ≤ 33315; aggregate ≤ 192568. Every other crate 0."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:67-71`): a kernel FLAG-Winston grant (D-20-7-01/02, the T3 teardown kill path), a security-critical control (a self-hosted runner on a public repository: `check-perf-runner-policy` is the only thing between a fork PR and remote code execution on the perf host, ADR-068 §4) and concurrency (the T3 teardown/snapshot race, the supervisor barrier). Blind and Edge on opus-5.5, Acceptance on opus-5.5, a Security reading on AC2 and AC8, Test Infrastructure on glm-5.3 (standing policy `proof-contract.md:57-71`; any session substitution recorded at review). Creation anthropic/claude-opus-5-5."
---

# Story 20.7: Stable-runner performance gates

Status: done

<!-- Created 2026-11-01 at a4c00897 (main, one docs commit ahead of origin/main 77f7276a; `git log -1 -- crates xtask` = a4c00897) by bmad-create-story under the lighter measured creation contract. Operator rulings in force: D01 A, D02 A, D11 A (2026-10-30); D13 A (2026-11-01, rows203c). D-20-7-01…08 are PROPOSED, not ruled — the operator rules; a different ruling rewrites only the clauses marked with that row. No cargo ran: cites re-read at HEAD (`creation/cites-a4c00897.md`); kernel and kloc by the prebuilt gate binary on `git archive` copies (`creation/measurements.md`); the four flake families diagnosed from the hosted job logs fetched with the operator's gh token (`creation/flake-diagnosis.md`, `creation/ci-excerpts/`). Stale epic cites are corrected here and owed to E20 through `creation/records-owed.md` §A (shared file; the orchestrator applies). -->

**Status rationale — ready-for-dev, not blocked.** The epic sequences this story in two phases (E20:244; PF:138,142):
everything that needs no machine — the policy gate, the budget file, the verdict command against a fixture ledger, Tier C
counts, the benches, the J1 real-spawn bench's hosted smoke and every flake repair — can start now (landing L1). Only the
shadow and binding phases (AC1's stable-runner red proof, AC11, AC12's retirement) need `ops-stable-perf-runner` done,
plus ≥ 2 weeks and ≥ 20 runs on `main` (ADR-068 §7, `:143-150`), which no coding can compress (D11). At creation the runner
does not exist: `gh api repos/lunarpulse/maos/actions/runners` → 0 runners; no `perf-ledger` branch
(`git ls-remote`, 2026-11-01). Under D-20-7-05 A the key stays `in-progress` between L1 and L2; under D-20-7-05 C it
would be `blocked` today.

> **The capability:** *a performance number MAOS publishes is evidence — measured on one controlled machine under one protocol against one budget file, recorded per SHA, and able to fail — and the shared-runner tests that pretended to measure teardown and hot-path speed either measure what they claim or stop claiming it, with the real defects behind their flakes repaired.*

**Closes:**
- Epic-20 §20-7 (E20@HEAD:236-253) as amended 2026-10-30 (D01 A; PF §3.4): epic AC1–AC7, renumbered here AC1–AC5, AC11, AC12.
- Received from E19 (E20:242): the J1 real-spawn 50-Worker fan-out bench (D-19-3-11 C, `19-3-…md:108,132,293`; S:289,304) → AC6; the S3 SIGKILL/pidfd family → AC7; the root-shutdown ac4 family (incl. the ac3 door-unload route-budget row) → AC8; the supervisor kill-call barrier → AC9; `cap_token_verify_p99` → AC10.
- `deferred-work.md:1075` (NFR-Perf gates absent), `:1181-1191` (S3), `:1192-1198` (root-shutdown family + supervisor), `:1256-1259` (`ac4_v4`, 18-0 allowance), `:1297-1298` (cap-token P99), `:96` (ac3 route budget) → AC13.
- Routing audit `routing-audit-2026-10-22.md:48` (six PRD hot-path budgets, `success-criteria.md:40-46`; FR16 posture-shift corpus) → AC4.

**Does NOT close, and says so:**
- Provisioning, renting and upkeep of the machine (`ops-stable-perf-runner`, Lunarpulse; ADR-068:190-191) and every other operator prerequisite (`creation/records-owed.md` §O). This story writes the runbook the operator follows; it never fakes a host, a fingerprint or a ledger entry.
- The T3 24 h fuzz chain and its `fuzz-ledger` (20-5; PF:124) — not a 20-7 input; the branch already exists.
- The podman 4.x `start --attach` exit-code race itself ([INFERENCE] upstream, client exit 125); this story makes MAOS's records and cleanup true when it happens, not podman.
- NFR-Perf-6 (owned and gated by 18-3, `tests/coverage-matrix.yaml:1415-1426`); the 20-12 local dev-host cost levers.

## Story

As **Murat (gates) and Winston (runner policy, kernel)**, and the operator who reads a release's evidence,
I want **the NFR-Perf and §13.1 budgets measured on the stable runner with verdicts that cannot pass silently, the counts-only checks on every pull request, and the shared-runner flakes repaired at their causes**,
so that **"MAOS meets its performance budgets" is a ledger entry per SHA, and a red T3 teardown test means a real defect rather than a slow runner.**

**Contract:**
- Epic §20-7 (E20@HEAD:236-253) as amended 2026-10-30; ADR-068 (ACCEPTED 2026-09-25) is the design authority, not proof of a host (PF:236). Create-story tightened, did not add uncited ACs: AC6–AC10 are the received items E20:242 names; AC2.4 tightens AC2 to ADR-068 §4's `permissions: contents: read` (`ADR-068…md:96-98`).
- Tracker `sprint-status.yaml:304` (this key) and `:327` (`ops-stable-perf-runner`); E20 confidence rules 6, 9–11; proof contract `docs/testing/proof-contract.md:10-31` (one final-tree run + one plant per AC), `:33-71` (review, tiers).
- D13 A (RULED 2026-11-01, rows203c): repair real causes; tests stay fail-closed; no silent skip; any authorized full-run retry recorded by run id and test name. The E18 one-rerun allowance (`deferred-work.md:1257`, D-18-0-01 A) was scoped to 18-0 and is not inherited.

## Flake families at HEAD — measured, not assumed

Method (`creation/flake-diagnosis.md`): 14 failing jobs fetched (`gh api …/jobs/<id>/logs`, 2026-11-01), every cited
code path read at `a4c00897`; no test run. Verdict = whether the bound is a calibration of a correctly-measured
quantity (wider noise) or a real defect (discrete stall + wrong record), read from code and logs.

| Family | Hosted evidence | Pass vs fail (nextest wall) | Mechanism at HEAD | Verdict | AC |
|---|---|---|---|---|---|
| **S3** `s3_external_sigkill_requires_pidfd_fact` (`worker_terminal_facts_17_7.rs:426`) | 38807569588, 38987395150, 40200028265, 40671240744, 41243340929, 41281999213 — every one `10.03–10.10 s (bound 5s, podman 4.9.3)`, podman client **exit 125**, 4/6 `residue TerminationUnconfirmed; state retained`. **Not** 41233186774 (S3 passed 1.447 s there; brief corrected). | 1.2–3.8 s vs 11.5 s (= `SNAPSHOT_BUDGET`, `child.rs:26`) | snapshot stalls on `podman inspect` to its deadline (`child.rs:196-242`); `run_teardown` then returns `TerminationUnconfirmed` **before `rm`** although the pidfd confirmed the exit (`child.rs:418-422`, `:102-107`) → container retained, residue names the wrong fact. The bound times kill → root exit (`:268-286`), a teardown, not FR12's corpus floor (`functional-requirements.md:41`). | **Defect** (kernel teardown) + mis-aimed bound; a 12 s bound still reds at `:432-439` on these runs | AC7 (D-20-7-01) |
| **ac4** `ac4_v3b`, `ac4_v4` (`root_shutdown_unload_16_3.rs:348`), `cohort::ac4_v5`, + `ac3` door-unload (D:96) | 40980187631, 41008107054, 41281999213, 41289787129 (+ 40080259088, cohort 38974347270/38988216897); "**41275…**" (brief) is no run id | ac4_v4 1.3–2.4 s vs **16.77 s** ⇒ the root itself took ≈ 10 s to exit | payload still in `/proc` 5 s after the root exited. Teardown kills only through the runtime CLI (`child.rs:273-280,407-417`); the supervisor's pidfd reaches the podman client, never the payload (`supervision.rs:3201-3210`), though the verified payload pidfd is held (`spawn.rs:310-320`). Failure message carries no root stderr/`/proc` state → live vs zombie undetermined. | **Defect** if live (orphan on SIGTERM; NFR-Rel-11); oracle defect if zombie — decided by T0 diag | AC8 (D-20-7-02) |
| **supervisor** `binding_revokes_bearer_before_unconfirmed_stop_and_root_retains_lease` | 41013625894 @ `43ce3ea7` only | 5.03–5.05 s ×7 (= `KILL_BUDGET`) vs 20.59 s (hang guard) | barrier waits for the stub's `argv=<kill>` (`supervisor_scenes.rs:744-756`); no wait on the path at HEAD explains 20 s; calls log not printed. **Second, deterministic defect:** on the panic, `WorkerBinding::drop` (`supervision.rs:1453-1470`, "Never panic out of a drop", `:1459`) → `unload` → `block_on` (`:2503-2504`) panicked "Cannot start a runtime from within a runtime". | Cause **open**; drop-panic **defect** | AC9 (D-20-7-03) |
| **cap_token** `cap_token_verify_p99_under_5us_hot_path` (`cap_token_verify_assertion.rs:85`) | 40237233599 (5670 ns), 41243340929 (5480 ns) | — | 10k wall-clock samples, mock crypto, no warm-up, **debug** build, in a 5,000-test parallel run on a shared runner; ADR-068 names the runner for ADR-030's 5 µs (`ADR-068…md:6,161`). The "overall" test (`:92-111`) claims verify + audit-enqueue (`:3`) but times the same loop. | **Instrument** (calibration class); "overall" test dishonest | AC10 (D-20-7-04) |

## Current-tree corrections — read before implementing

Full OLD → NEW table: `creation/cites-a4c00897.md` (C1–C16, T1–T3, P1–P6, L1–L9).

| Premise (epic / preflight / tracker / brief) | Fact at `a4c00897` and consequence |
|---|---|
| Advisory jobs `discipline.yml:2940-3013` (E20:240), `:1693-1778` (E20:244,253) | `:2970-2983`, `:3022-3035`, `:3040-3055` (C1, C2). |
| J4 advisory `:4891-4931` | `check-j4-latency` `:4856-4896` (C3); registry already advisory (`gate-registry.toml:457-459`, C10) — "registered blocking" (E20:244) is 20-3a-repaired history. |
| Coverage `:1326-1374` / `:1364-1419` | NFR-Perf rows `tests/coverage-matrix.yaml:1380-1436` (C4, C5). |
| J1 budget `j1.rs:183-190` (E20:238) | `j1.rs:25,162-170` (C6); a fourth budget copy: J1 25 ms at `decision.rs:14` (C7). |
| "the existing `hot_swap_latency` bench" | it is `crates/maos-kernel-core/benches/hot_swap_latency.rs` (C9) — wiring it touches no kernel `src/` line. |
| Release precondition "refuses a tag without PASS" | reads only the check-runs `aggregate` (`check_release_precondition.rs:33-60`); the ledger read is new (C12). |
| kernel-Δ 0; xtask +250–450; bench +150–300 (E20:79,238) | flake repairs at cause touch the kernel T3 teardown (R +40/+31 measured); xtask +560…+990, bench `src/` +60…+200 forecast (C14, C15). |
| "wall-clock calibrations of T3 teardown claims" (D:1198) | bimodal one-budget stalls with degraded records (C16; table above). |
| S3 in 41233186774; ac4 in "41275…" (brief) | wrong / unresolvable (L7, L8). |
| Ledger "failure text not recorded" (D:1190,1191,1259,1298) | recorded now (L3–L6). |

## Kernel and kloc pricing — measured at creation

**Instrument:** prebuilt `/mnt/build/cargo-maos/debug/xtask` (`check-kernel-baseline --json`, `kloc-check --json`) on
`git archive a4c00897` copies; the untouched copy reproduces the pins exactly (26801 / 95 files / set hash
`c023d0d7…`; kernel 21011 tokei; aggregate 188816). Detail: `creation/measurements.md`,
`creation/kloc-kernel-a4c00897-and-prototypes.json`.

**Kernel (◆ FLAG-Winston, PROPOSED):**

| Shape | Kernel physical | Kernel tokei | Diff |
|---|---:|---:|---|
| K1w wait for an in-flight snapshot owner to publish, then remove (D-01 A) | +18 (26819) | +15 (21026) | `k1w-wait-snapshot-then-remove.diff` |
| K1 remove with the snapshot still in flight when termination is confirmed (D-01 B) | +4 (26805) | +1 (21012) | `k1-remove-confirmed-dead-payload.diff` |
| K2r pidfd SIGKILL of the verified payload before the runtime kill, `rustix` (D-02 A) | +22 (26823) | +16 (21027) | `k2r-pidfd-kill-rustix.diff` |
| K2 same via a new `unsafe` `libc::syscall` helper (D-02 B) | +44 (26845) | +33 (21044) | `k2-pidfd-kill-libc.diff` |
| **R = K1w + K2r (recommended)** | **+40 (26841)** | **+31 (21042)** | `k1wr-k1w-k2r-bundle.diff` |
| R′ = K1 + K2r | +26 (26827) | +17 (21028) | `k12r-k1-k2r-bundle.diff` |
| R″ = K1 + K2 | +48 (26849) | +34 (21045) | `k12-k1-k2-libc-bundle.diff` |

Both pins move with any shape. Not compiled ([INFERENCE]; measurement owed items 1–2).

**kloc forecast** (tokei; HEAD = every pin, zero headroom):

| Crate · component | Measured / forecast | Basis |
|---|---:|---|
| maos-kernel-core · R (AC7, AC8) | **+31** (physical +40) | prototype `k1wr` |
| xtask · budget loader, `check-perf-verdict`, `check-perf-runner-policy`, ledger append, release-precondition ledger read, Tier C compare (AC1–AC3, AC5, AC11) | +560…+990 [INFERENCE] | `check_rto_gate.rs` 292 + `check_rto.rs` 303 code lines (verdict/ledger analogue) |
| maos-bench `src/` · budget-file consumers, NFR-Perf-2/4/5 + Tier C harness helpers (AC3–AC5) | +60…+200 [INFERENCE] | `benches/` uncharged (`kloc_check.rs:305`) |
| maos-bin `src/` · drop never panics (AC9, D-03 A); real-spawn seam if needed (AC6) | 0…+40 [INFERENCE] | `supervision.rs:1453-1470,2503-2504` |
| tests, benches, workflows, docs | uncharged | — |
| **aggregate** | **+31 measured; +620…+1230 forecast** | sum |

**Envelope (D-20-7-08 A, PROPOSED; D02 A ×2 measured / ×3 unmeasured upper; ceilings, not reservations):**

| Crate | HEAD | Forecast | Ceiling with allowance | Inherited room | Operator ruling needed |
|---|---:|---:|---:|---:|---|
| maos-kernel-core (physical / tokei) | 26801 / 21011 | +40 / +31 | **26881 / 21073** (+80 / +62) | 0 | **yes — FLAG-Winston** |
| xtask | 42860 | +560…+990 | **45830** (+2970) | 0 | yes |
| maos-bench | 2451 | +60…+200 | **3051** (+600) | 0 | yes |
| maos-bin | 33195 | 0…+40 | **33315** (+120) | 0 | yes (if D-03 A / D-06 needs a seam) |
| aggregate | 188816 | +651…+1261 | **192568** (+62 + 3690); hardfail = measured + 1 | 0 | yes |

Under D-20-7-05 A, L1 lands every row except what L2 alone needs (the release-precondition ledger read may land in L1
reading an absent ledger as `ABSENT`); L2 asks only its own residue against L1's exact pins. Pins land exact after
`cargo fmt --all`; a lower measurement lowers the pin.

**Operator-ruling list (every draw above the inherited room; PROPOSED, not granted):** (1) FLAG-Winston kernel row R
(D-01 + D-02; or R′/R″, or one half, or none); (2) xtask envelope; (3) maos-bench envelope; (4) maos-bin envelope;
(5) aggregate hardfail; (6) D-20-7-01…08; (7) operator prerequisites §O of `creation/records-owed.md` (owners named
there) before L2. Each figure lands exact, with a HISTORY row citing this key.

## Decisions — RULED 2026-11-01 (rows207; table below = creation record)

Preflight rows that bear on this story stay as filed: D11 RULED A (2026-10-30: provisioning starts now; `PF:295`),
D13 RULED A (2026-11-01, rows203c; `PF:297`). Their 20-7 consequence is in AC7–AC10 and the status rationale.

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-20-7-01** | S3: what the 10 s stall is and what to repair | **A** K1w kernel repair (+18/+15): before removal, teardown waits — bounded by its own deadline — for an in-flight snapshot owner to publish (the owner's `inspect` is bounded by the same snapshot deadline and notifies on publish, `child.rs:236-241`) instead of returning `TerminationUnconfirmed` before `rm` (`:418-422`); snapshot-before-removal order kept. S3 asserts unconditionally what the pidfd and cleanup guarantee (`payload_exit` `signaled(sig=9)` from `pidfd`, `removed`, no residue, orphan cause `fault.signaled`/9, one crash receipt); the runtime's own facts (`runtime_exit_code` 137, client `exit_cause`) are asserted when the runtime answered and as the typed unavailable form (`DeadlineExpired`/`InspectTimedOut`, `child.rs:211-214,251-256`) when it stalled, the stall printed so its rate is visible; the per-sample bound becomes the documented teardown budget for both podman majors (`SNAPSHOT_BUDGET + 2 s`, as podman 5 since 2026-10-14); FR12's 2 s / 5 s floors are proven where FR12 states them — the ≥ 99/100 corpus (`worker_crash_corpus_16_3`) on T3 · **B** K1 (+4/+1): proceed to removal while the snapshot is in flight when `termination_confirmed()` holds; same test rework · **C** test-only: podman-4 bound = 12 s, residue and retained container accepted as "runtime-caused" (the fact assertions `:432-439` weakened) · **D** run T3 legs on podman 5 only (`ubuntu-26.04`) | **A.** Every failure is one `SNAPSHOT_BUDGET`; 4/6 carry a retained container and a residue the pidfd had already disproved, 2/6 do not — the waiter/owner race K1w removes. B is cheaper but its correctness hinges on a confirmation the no-pidfd fallback may not have. C hides a real retained container (D13 A). D drops the podman-4 evidence the epic-17 D6 rule keeps (`deferred-work.md:1196`) while production hosts ship podman 4 (`:1170`). Asserting runtime facts only when the runtime answered is the kernel's documented contract (typed unavailable evidence), not a weakening: the claim S3 names — the pidfd fact — stays unconditional; the single-sample root-exit bound is not FR12's form (`functional-requirements.md:41`). | PROPOSED |
| **D-20-7-02** | ac4: an orphan when the runtime CLI stalls | **A** K2r (+22/+16): teardown SIGKILLs the verified payload pidfd (container pid 1, so the kernel ends its pid namespace) before the bounded `podman kill`, via `rustix::process::pidfd_send_signal` (feature `"process"` already enabled workspace-wide by maos-bin, `Cargo.toml:123`); the 5 s oracle becomes zombie-aware and its failure prints root stderr, `/proc/<pid>/status` and the CID's `podman ps -a` · **B** K2 (+44/+33): same through a new `unsafe` `libc::syscall` helper in `sandbox/linux.rs` · **C** oracle and diagnostics only (no kernel): zombie-aware, context in the message; if the payload is live, the row stays open | **A**, confirmed or overturned by the T0 hosted diagnostic (measurement 4): a root exit of ≈ 10 s (16.77 s sample) is a budget stall, not reaping latency, and a live payload after root exit is NFR-Rel-11's orphan. B costs twice the lines and an `unsafe` block for the same syscall. C is right only if T0 shows a zombie. | PROPOSED |
| **D-20-7-03** | Supervisor barrier | **A** diagnose and repair what is deterministic: `WorkerBinding::drop` never panics in a runtime context (its own contract, `supervision.rs:1459`) — `unload` uses `block_in_place` on a multi-thread runtime worker and otherwise a scoped thread; the hang-guard message prints the stub calls log, binding phase and stop-task state; each stub invocation writes its record in one `printf`; ≥ 50 whole-package loaded runs recorded; if still unreproduced the row stays open with the improved diagnostics (rule 10 owner named at landing) · **B** test-only drop guard (`block_in_place` around every exit path) + diagnostics · **C** record-only | **A.** The drop panic is a production `Drop` violating its own documented invariant, observed in the log; B leaves it for production callers. The barrier cause is not reproduced; a guess-fix is not a repair. maos-bin +5…+15 [INFERENCE]. | PROPOSED |
| **D-20-7-04** | `cap_token_verify_p99` 5 µs | **A** relocate the claim to instruments that can carry it: ADR-030's 5 µs becomes a Tier C instruction-count metric for `CapTokensShardRing::verify` on every PR and a Tier W P99 metric (existing `crates/maos-kernel-core/benches/cap_token_verify_p99`) in `perf-budgets.toml`; the hosted test keeps NFR-Perf-3's 100 µs as a named catastrophe guard; the "overall" test times verify + audit-enqueue as its doc says · **B** keep a hosted 5 µs assertion in a serialized release-profile job with warm-up · **C** keep as is | **A.** A debug-profile wall-clock P99 inside a 5,000-test parallel run on a shared hypervisor cannot carry 5 µs (ADR-068 §1, Alternatives `:174-177`); B is still a shared hypervisor. Not silent (D13): the bound moves to the budget file with its source and disposition in the same commit. | PROPOSED |
| **D-20-7-05** | Landing shape / dev-start contract | **A** one key, two landings: L1 machine-free (AC1 fixture half, AC2–AC10, AC12 coverage half, AC13 partial) now; L2 after `ops-stable-perf-runner` done + ≥ 2 weeks/20 main runs (AC1 runner half, AC11, AC12, AC13 close); `done` after L2 · **B** split: this key = L1, a new key (e.g. `20-7b-perf-binding`) = L2 (epic + tracker rows via the orchestrator) · **C** `blocked` until the runner exists | **A.** PF:138,142 already sequence one key across waves 1 and 5; Tier C and the flake repairs are worth landing now (D11's two-week clock starts only once `perf.yml` runs). B is cleaner tracking at the cost of a new key; C idles work that needs no machine. | PROPOSED |
| **D-20-7-06** | J1 real-spawn fan-out bench (received, D-19-3-11 C) | **A** real T3 `worker-cli-fixture` Workers through the real `WorkerSupervisor` (the `worker_crash_corpus_16_3` harness shape, `:1-40,88-102`): 50 concurrent, 10 tasks/s, task-dispatch latency = `task.assign` journal → `cli_wrapper_loaded`, 0 dropped; weekly 1 h on Tier W (perf host needs rootless podman — operator prerequisite O4); a hosted smoke (≈ 5 Workers, 60 s) under `provision-t3-test-substrate` proves the spawns are real (distinct CIDs and child pids) · **B** T0 subprocess fixtures via `spawn_and_bridge` (19-3's rejected A: no production Worker spawns at T0) · **C** weekly 1 h T3 soak on a hosted `ubuntu-24.04` runner, verdict advisory | **A.** Measures the path production runs (every Worker is T3, `epic-19…md:186`); B measures a path nothing runs; C keeps the shared-runner noise ADR-068 exists to remove. NFR-Perf-8's in-process bench stays as the IAC dispatch metric, renamed honestly. | PROPOSED |
| **D-20-7-07** | Tier C instrument | **A** `valgrind --tool=callgrind` instruction counts (apt on hosted images) over a small maos-bench harness binary per hot path, minus an empty-harness baseline, plus a counting `#[global_allocator]` in that harness for allocation counts; committed baseline in `xtask/perf-budgets.toml` · **B** `iai-callgrind`/`gungraun` dev-dependency (new crate: deny/licence review, toolchain coupling) · **C** `perf stat -e instructions` (hardware counters unavailable on hosted runners) | **A.** No new dependency; deterministic; fork-safe on hosted runners (ADR-068 §1). Cost per PR is measurement 6. | PROPOSED |
| **D-20-7-08** | Per-crate envelope (D02 A applied) | **A** the envelope above (×3 unmeasured / ×2 measured) · **B** exact measured grants only, every review correction re-asks · **C** allowance ×1.3 (rule 6) | **A.** 20-3b and 20-3c landed at 2–3× creation forecasts (`20-3c-…md:342`); most of this story is unmeasured. | PROPOSED |
| **D-20-7-09 (R3)** | AC5 instruction baselines and allocation-neutral plant | A defer to L2; **B** obtain hosted callgrind measurements and commit binding baselines before L1 closes; C wait for stable host. | Hosted baselines and instruction-only red proof obtained; see evidence index. | **RULED B 2026-11-02 — done** |
| **D-20-7-10 (R11)** | AC4.1 NFR-Perf-5 production WASM recall instrument | **A** move recall leg to L2, blocked by missing guest harness; audit-query only in L1; B build in L1; C keep L1 open. | No substitution of audit-query for recall. | **RULED A 2026-11-02** |
| **D-20-7-11** | K2r after base witness contradiction | A retain as proven repair; B remove K2r; **C** retain as defense-in-depth, retract AC8.3 and historical-witness equivalence of AC8.5. | Base passed under shim; composite plant left a live orphan. Cause of historical flake unidentified; no further hosted cycle. | **RULED C 2026-11-02 (ask k2r)** |

**Rulings 2026-11-01 (ask rows207, operator):** D-20-7-01 A, -02 A, -03 A, -04 A, -05 A (one key, two landings), -06 A, -07 A, -08 A; D11 and D13 already A. Kernel row R = K1w + K2r (+40 physical / +31 tokei); ceilings kernel ≤ 26881 / 21073. Other ceilings: xtask 45830, maos-bench 3051, maos-bin 33315, aggregate 192568. Kernel interaction: 20-1b holds the other kernel grant (R = KQ1 + P, +78 / +68 from HEAD 26801/21011; ceilings 26957 / 21147). Both grants are computed from HEAD 26801/21011; whichever lands second re-bases: cumulative measured +68 and +31 → 21110 tokei expected; cumulative tokei ceilings must not exceed 21209 (= 791 under the 22000 ADR-069 alarm); STOP otherwise.

## Acceptance Criteria

*(Epic §20-7 as amended 2026-10-30 and by D-20-7-10 A / D-20-7-11 C on 2026-11-02. AC8.3's failing-before claim is retracted; other named witnesses and plants retain their stated boundaries. **local** = hermetic dev host; **hosted** = GitHub-hosted branch run; **runner** = `maos-perf` host (L2 only).)*

### AC1 — `check-perf-verdict` prints one honest verdict per metric (epic AC1)

1. `cargo run -p xtask -- check-perf-verdict --sha <SHA> [--ledger <path>]` prints `PASS`, `FAIL`, `INDETERMINATE` or `ABSENT` per metric in `xtask/perf-budgets.toml`, read from the `perf-ledger` branch (ADR-068 §5–§6, `:105-139`): calibration CV > 2 % or median drift > 3 % ⇒ every verdict `INDETERMINATE`; five-run spread > tolerance ⇒ `INDETERMINATE`; regression `FAIL` only when median > baseline × (1 + tolerance) **and** > 3 × MAD; new fingerprint ⇒ regression checks `INDETERMINATE`; no record for the SHA ⇒ `ABSENT`; missing branch ⇒ loud error; newest entry > 48 h old ⇒ red (the scheduled staleness check).
2. **Failing-before (local):** at `a4c00897` the subcommand does not exist (`xtask/src/main.rs` has no `check-perf-verdict`). Fixture-ledger vectors in `xtask/tests/` cover each verdict and the two loud failures.
3. **Plant (local):** compute the verdict from the best of five runs instead of the median ⇒ the noisy-spread vector reads `PASS` and reds.
4. **Runner leg (L2):** on `maos-perf`, J4's `bench-fault-inject` 15 ms sleep (`crates/maos-bench/src/harness/j4.rs:196`) yields `FAIL`; a planted noisy calibration (CPU hog on a benchmark core during calibration) yields `INDETERMINATE`, never `PASS`. Run ids recorded. **MOVED to `ops-stable-perf-runner` under ruling r1 (operator, 2026-11-12; `_bmad-output/planning-artifacts/decisions-2026-11-12-acceleration.md` item 8) (not landed).** Claim boundary: "perf verdicts ABSENT; no stable-runner timing claim".

### AC2 — the runner only ever runs trusted code (epic AC2; ADR-068 §4)

1. `.github/workflows/perf.yml` is the only workflow with `runs-on` containing `maos-perf`; triggers `push` to `main`, `schedule`, `workflow_dispatch`, and `pull_request` with a job-level same-repository guard; never `pull_request_target`; `permissions: contents: read`; results leave as an artifact appended by a GitHub-hosted follow-up job (no write token on the host).
2. `check-perf-runner-policy` reds on each of: a second workflow targeting `maos-perf`; `perf.yml` without its fork guard; `pull_request_target` added; `permissions` wider than `contents: read` on a `maos-perf` job (AC2.4 tightening, `ADR-068…md:96-98`).
3. **Failing-before (local):** gate absent at HEAD. **Plants (local, four, each named — no other control falsifies these):** one per refusal above ⇒ red; restored ⇒ green. Enrolled in `discipline.yml` as blocking.
4. Security reading at review: a fork PR cannot reach the runner by any trigger in the tree.

### AC3 — one budget source (epic AC3)

1. `xtask/perf-budgets.toml` holds per metric: NFR / §13.1 / success-criteria id, quantile, budget, unit, noise tolerance, regression tolerance, per-phase disposition, tier (C/W) and instrument. Readers: the J1/J4 harnesses, `decision.rs`, the benches, the verdict command, Tier C.
2. The literals at `crates/maos-bench/src/harness/j1.rs:25,170`, `harness/j4.rs:36`, `src/decision.rs:14-15`, `benches/j4_latency_regression.rs:25`, `benches/iac_routing_budget.rs:37` and (D-04 A) `crates/maos-kernel-core/tests/cap_token_verify_assertion.rs:86` are replaced by reads of the file (the 50 ms J1 catastrophe ceiling becomes a named row).
3. **Failing-before (local):** an `xtask/tests` vector that finds budget literals outside the file reds at HEAD on the seven sites above.
4. **Plant (local):** re-add `const J4_P95_BUDGET_US: u64 = 10_000;` to `decision.rs` ⇒ the vector reds.

### AC4 — benches measure what the NFRs say, and fail on breach (epic AC4; routing-audit:48)

1. `iac_routing_budget` reports and checks NFR-Perf-1's P50 < 5 ms / P99 < 50 ms (`non-functional-requirements.md:9`), not a 1 ms P95 soft floor; `orchestrator_fanout_nfr_perf_8` keeps its in-process IAC dispatch measurement under an honest name and its 15 s/3600 s selector moves to `perf.yml` (weekly 1 h on Tier W); `crates/maos-kernel-core/benches/hot_swap_latency.rs` is wired for NFR-Perf-7; new benches for NFR-Perf-2 (sustained frames/s), NFR-Perf-4 (1000-shift posture corpus, P99 ≤ 2 s / P99.9 ≤ 5 s; FR16) and NFR-Perf-5 (30-day-sized audit fixture, P99 ≤ 2 s single-Spirit / ≤ 10 s global). **D-20-7-10 A:** the separate production WASM `recall` route measurement (R17-154/157) moves to L2; the missing guest benchmark harness blocks it. L1 audit-query coverage does not claim recall coverage.
2. The six PRD hot-path budgets (`success-criteria.md:40-46`: `iac/send` < 10 µs P99; issuance < 5 µs; dispatch < 5 µs; `memory/read` 50 µs / 5 ms; hot-swap 50 / 500 ms; telemetry < 1 µs) each have a metric row and an instrument (existing `crates/maos-kernel-core/benches/*` or new); no row without an instrument.
3. Each bench exits non-zero on a breach when its disposition for the current phase is binding; `-- --test` smoke jobs keep honest smoke names (ADR-068 §7.4).
4. **Failing-before (local):** an `xtask/tests` vector "every NFR-Perf-1…5,7,8 and every success-criteria hot-path id has a metric with an existing instrument" reds at HEAD (no file; `coverage-matrix.yaml:1389,1405,1410,1427,1432` `gates: []`). **Plant (local):** delete the NFR-Perf-4 bench target ⇒ the vector reds.

### AC5 — Tier C counts on every pull request (epic AC5; D-20-7-07 A)

1. A hosted, fork-safe job measures instruction and allocation counts for the CPU-bound hot paths (capability-token verify, IAC routing, frame lowering; ADR-068 §1, `:53-56`) and compares with the committed baseline in `perf-budgets.toml`; a count over tolerance blocks the PR; a deliberate change re-baselines in the same commit with the measured figure and reason (ADR-068 §6, `:133-135`).
2. **Failing-before:** absent at HEAD. **Plant (hosted, one, `ubuntu-24.04`):** an extra loop in the token-verify harness path ⇒ the Tier C job reds; restored ⇒ green. Run id and wall-clock recorded.

### AC6 — the J1 real-spawn fan-out bench (received from 19-3, D-19-3-11 C; D-20-7-06 A)

1. 50 concurrent real T3 `worker-cli-fixture` Workers through the real `WorkerSupervisor`, 10 tasks/s, dispatch latency `task.assign` → `cli_wrapper_loaded`, P99 ≤ 500 ms and 0 dropped over 1 h (NFR-Perf-8, `non-functional-requirements.md:16`; FR21 fan-out floor) — weekly on Tier W (L2 binding). **AC6.1 (the 1 h Tier W soak) MOVED to `ops-stable-perf-runner` under ruling r1 (operator, 2026-11-12; `_bmad-output/planning-artifacts/decisions-2026-11-12-acceleration.md` item 8) (not landed).** Claim boundary: "perf verdicts ABSENT; no stable-runner timing claim". AC6.2/6.3 (hosted smoke and plant) LANDED.
2. **Hosted smoke (L1):** ≈ 5 Workers, 60 s, under `provision-t3-test-substrate`; asserts the spawns are real — distinct CIDs and child pids equal to the Worker count, each with `cli_wrapper_loaded`/`cli_wrapper_exit` rows.
3. **Failing-before:** no such bench at HEAD (`orchestrator_fanout_nfr_perf_8.rs:22` spawns 50 in-process task handles). **Plant (hosted):** substitute in-process handles for the T3 spawn ⇒ the distinct-CID assertion reds.

### AC7 — S3: a dead payload's container is removed and its record is true (D-20-7-01 A) ◆ FLAG-Winston

1. Kernel: `k1w-wait-snapshot-then-remove.diff` shape — `run_teardown` (`child.rs:401-466`) waits, bounded by `overall`, for an in-flight snapshot owner to publish, then continues to `rm --force` and `container exists`; it returns `TerminationUnconfirmed` without removal only when `facts_by` gave nothing or the owner did not publish by `overall`.
2. **Failing-before witness (local, hermetic):** a stub-runtime vector (new stub mode: the first `inspect` hangs, later ones report `exited`; an observer thread owns the snapshot via `facts()`, then teardown runs), 20 instances run concurrently in one test (≈ one `SNAPSHOT_BUDGET` of wall clock) ⇒ at `a4c00897` at least one instance returns `TerminationUnconfirmed` with no `rm` in its stub calls log (the waiter/owner race seen 4/6 in hosted logs; the observed red count recorded at T0 — 20/20 green at HEAD is STOP + D-row, no un-witnessed kernel line); after K1w: 20/20 `rm` called, verdict `Ok`, state dir removed.
3. S3 (`worker_terminal_facts_17_7.rs:397-456`) asserts the pidfd and cleanup facts unconditionally and before any timing (`payload_exit` `signaled(sig=9)`, source `pidfd`, `removed`, no residue, orphan `fault.signaled`/9, one crash receipt); the runtime's own facts (`runtime_exit_code` 137, client `exit_cause`) when the runtime answered, the typed unavailable form when it stalled (printed); the failure message carries the client exit, the residue line and the elapsed time; the per-sample bound is the documented teardown budget `SNAPSHOT_BUDGET + 2 s` for both podman majors, with the source comment saying why; FR12's 2 s / 5 s floors stay asserted by `worker_crash_corpus_16_3` (≥ 99/100) on the T3 leg.
4. Measurement 3 recorded: on a stalled-inspect hosted sample the journaled exit cause is the pidfd's `signaled(9)`, not the client's 125 — if it is not, fix classification in maos-bin within this AC (envelope maos-bin).
5. **Hosted leg:** S3 green on `ubuntu-24.04` (podman 4.9.3) and `ubuntu-26.04` (podman 5.7) in the final discipline run; no retry, no ignore.
6. **Plant (local):** restore `if facts.is_none() || self.lock().snapshot_inflight { return Err(..) }` ⇒ AC7.2 red (≥ 1 of 20).

### AC8 — defense against runtime-cleanup unavailability (D-20-7-02 A, amended D-20-7-11 C) ◆ FLAG-Winston

1. The hosted family diagnostic prints root stderr, payload `/proc/<pid>/status` `State`/`PPid` and `podman ps -a --filter id=<CID>` on failure; ≥ 20 runs on `ubuntu-24.04`. Run 41423701978 was 20/20 green at HEAD, but ran post-repair with K2r present: it cannot decide K2r versus the oracle or identify the historical flake's cause.
2. Retain K2r defense-in-depth: `run_teardown` signals the verified payload pidfd before bounded runtime kill; `KillFailed` is reported only when both fail. This is not a proven repair of the historical ac4 flake.
3. **RETRACTED (D-20-7-11 C):** the claimed deterministic failing-before at `a4c00897` is contradicted: `ac4_v4` passed in 14.147 s under the stalled-kill shim in run 41423701978 / job 125205091430. No historical failing-before witness is claimed.
4. The oracle distinguishes a zombie awaiting reap (`State: Z`) from a live payload, and its message carries AC8.1's context permanently. `ac4_v3b`, `ac4_v4`, `ac4_v4b`, `cohort::ac4_v5`, `ac4_v5c` and `ac3_operator_door_unload_stops_a_live_worker_once` keep their 5 s bound (a live payload 5 s after root exit is a defect, never a calibration). **Amended 2026-11-12 under ruling r1 (operator, 2026-11-12; `_bmad-output/planning-artifacts/decisions-2026-11-12-acceleration.md` item 8):** for the rc the 5 s root-exit bound REMAINS the product bound and a live payload 5 s after root exit remains a defect in the product; what changes is only the disposition of timing-noise reds on hosted runners (ac4_v3b / v4 / v4b / v5 / v5c / door-unload reds are recorded in `deferred-work.md` as hosted-runner timing noise, owner `ops-stable-perf-runner`, with their same-line citations). This is a disclosed disposition, NOT a proof: the cause of the historical ac4 orphan remains UNIDENTIFIED (D-20-7-11 C), K2r is only defense-in-depth, and no bound was widened, no test ignored or retried to green.
5. **Injected composite failure defence (hosted):** with runtime kill stalled and removal of a running container refused by the shim, removing the pidfd signal at HEAD leaves a live payload (`State: S`, container Up); normal HEAD passes. This ablation is not equivalent to AC8.3's retracted historical failing-before witness and does not identify the historical flake's cause.

### AC9 — the supervisor barrier: what is deterministic is repaired, what is not is diagnosable (D-20-7-03 A)

1. `WorkerBinding::drop` never panics in a runtime context (`supervision.rs:1453-1470`, `:2503-2504`).
2. **Failing-before witness (local, deterministic):** a vector forces the scene's barrier assertion to fail in the re-exec child (`supervisor_scenes.rs:648-668` pattern) and asserts the child's stderr has exactly one panic and no "Cannot start a runtime from within a runtime" — red at `a4c00897` (observed in 41013625894, `creation/ci-excerpts/run-41013625894-job-123924858906.txt`).
3. The hang-guard message prints the stub calls log, the binding phase and whether the stop task finished; each stub invocation appends its record with one `printf` (`stub_scenes.rs:31-32`).
4. ≥ 50 loaded whole-package local runs recorded with outcome; unreproduced ⇒ D:1197's supervisor clause stays open with the new diagnostics and a named owner (rule 10), never closed on a guess.
5. **Plant (local):** restore the bare `block_on` in `unload` ⇒ AC9.2 red.

### AC10 — the 5 µs hot-path claim lives where it can be measured (D-20-7-04 A)

1. `perf-budgets.toml` rows: `cap_token_verify` Tier C count (blocking per PR) and Tier W P99 < 5 µs (ADR-030) from `crates/maos-kernel-core/benches/cap_token_verify_p99`; the hosted test asserts NFR-Perf-3's 100 µs P99 as a named catastrophe guard and prints the measured P99.
2. `cap_token_verify_p99_under_100us_overall` times verify **plus** the audit enqueue its doc names (`cap_token_verify_assertion.rs:1-4,92-111`), or is renamed to what it times.
3. **Failing-before:** n/a for a shared-runner P99 (no deterministic red exists; recorded hosted samples 5670/5480 ns are the evidence). **Plant:** AC5.2's verify-path loop is this AC's plant (one control, no second plant).

### AC11 — shadow, then bind (epic AC6; L2, runner)

> **MOVED to `ops-stable-perf-runner` under ruling r1 (operator, 2026-11-12; `_bmad-output/planning-artifacts/decisions-2026-11-12-acceleration.md` item 8).** AC11.1 (shadow data), AC11.2 (binding) and AC11.3 (`check-j4-latency` registry row binding at the Tier W metric) are NOT landed here. Claim boundary: "perf verdicts ABSENT; no stable-runner timing claim". Only the release-precondition perf-verdict leg of AC11.2 and the rc-only `--perf-absent-ok` waiver landed, under 20-5 L2 (`d00802aa`; `docs/release/decisions-2026-11-12.md`); a `GA` tag still refuses an `ABSENT` verdict.

1. ≥ 2 weeks and ≥ 20 `perf.yml` runs on `main` with verdicts recorded and no blocking (ADR-068 §7.2); tolerances replaced from the shadow data in one commit with the figures (rule 6).
2. Per-metric dispositions bind for the current phase; a `FAIL` on a trusted PR blocks its merge; `check_release_precondition.rs` refuses a tag SHA whose required verdicts are not all `PASS` (`ABSENT` included).
3. `check-j4-latency`'s registry row (`gate-registry.toml:457-459`) matches what its hosted job can do (sample count, integrity, mutation) and points its binding at the Tier W metric.
4. **Failing-before (local):** a release-precondition vector with a fixture ledger missing one verdict ⇒ passes at HEAD (no ledger read), reds after. **Runner leg:** AC1.4's two reds.

### AC12 — retirement, runbook and coverage (epic AC7)

1. (L2) `nfr-perf-1-iac-routing-budget`, `nfr-perf-j4-latency`, `nfr-perf-8-orchestrator-fanout` (`discipline.yml:2970-2983,3022-3035,3040-3055`) deleted with their `continue-on-error`; `-- --test` bench jobs keep honest smoke names. **AC12.1 MOVED to `ops-stable-perf-runner` under ruling r1 (operator, 2026-11-12; `_bmad-output/planning-artifacts/decisions-2026-11-12-acceleration.md` item 8) (not landed): retiring the three advisory jobs before binding would remove the only NFR-Perf-1/8 smoke.** Claim boundary: "perf verdicts ABSENT; no stable-runner timing claim".
2. (L1) `docs/runbooks/perf-runner.md`: host setup, boot-time tuning checks, fingerprint fields, calibration, maintenance window, re-baseline procedure, ledger staleness response; `docs/runbooks/provisioning-checklist.md:44` updated when the runner exists (operator).
3. (L1) `tests/coverage-matrix.yaml:1380-1436`: NFR-Perf-1 points at its own metric/gate (not `check-j4-latency`); -2/-4/-5/-7/-8 name their gates; -3 names AC10's metrics.
4. **Plant:** AC4.4's vector covers 12.3; 12.1 is a deletion (no plant).

### AC13 — the ledger stops naming this key the day it is true

1. At `done`: `check-dev-record-completeness` passes with no open `deferred-work.md` row owned by `20-7-…`: D:96, :1075, :1181-1191, :1192-1198, :1256-1259, :1297-1298 each `CLOSED <date> by 20-7 @<commit>` with its witness, or re-owned by rule 10 with a reason (AC9.4).
2. L1 closes nothing that needs the runner; L1 sub-bullets record each family's repair and witness.
3. **Closure under r1 (2026-11-12):** the six rows closable by landed work are `CLOSED 2026-11-12 by 20-7-stable-runner-perf-gates @<sha>` with witnesses; hosted-runner timing-noise rows carry a dated disposition quoting ruling r1, keep their same-line citations and name `ops-stable-perf-runner` as live owner; stable-runner-only rows are re-owned there with the claim boundary "perf verdicts ABSENT; no stable-runner timing claim"; rows with no honest owner are rule-10 dispositions with a trigger. Row-by-row table: `20-7-stable-runner-perf-gates-evidence/closure/closure-under-r1.md`.

## Tasks / Subtasks

- [x] **T0 — Preflight.** Operator rulings on D-20-7-01…08 recorded; tracker → `in-progress`. Run `creation/measurements.md` "Not measured here" items 1–6 (item 4 = AC8.1's `ci/20-7-diag` run; item 5 = AC9.4's loaded runs may continue through T9). Re-measure every cite this story touches (rule 9). A measured figure above the ruled envelope, or a T0 result contradicting a ruled option, is STOP + D-row. **LANDED under ruling r1 close:** rulings recorded 2026-11-01/02 (`e3231e88`, `a8af3215`); AC8.1 diag run 41423701978 and the 50 loaded runs (`9f64dd02`).
- [x] **T1 — Budget file (AC3).** `xtask/perf-budgets.toml`, loader, the seven literal sites, literal-scan vector + plant. **LANDED under ruling r1 close:** `1fd4de31`, `9f64dd02`.
- [x] **T2 — Runner policy (AC2).** `perf.yml` (shadow mode: no blocking), `check-perf-runner-policy`, four plants, `discipline.yml` enrolment, registry row. **LANDED under ruling r1 close:** `1fd4de31`, `9f64dd02`; hosted `check-perf-runner-policy` green in main run 43150757804.
- [x] **T3 — Verdict command (AC1.1–1.3).** Ledger reader/appender, fixture vectors, plant; staleness check scheduled. **LANDED under ruling r1 close:** `1fd4de31` (fixture vectors; `ledger-staleness` job in `perf.yml` gated on the runner variable).
- [x] **T4 — Benches (AC4).** NFR-Perf-1 quantiles, hot-swap wiring, NFR-Perf-2/4/5 (+ WASM recall leg), six hot-path rows; completeness vector + plant. **LANDED under ruling r1 close:** `1fd4de31`; production WASM `recall` leg carved out under D-20-7-10 A (guest harness missing; `deferred-work.md` rule-10 row).
- [x] **T5 — Tier C (AC5, AC10).** Harness, counting allocator, hosted job, committed baseline; hosted plant. **LANDED under ruling r1 close:** `1fd4de31`, `850e4c00` (binding baselines; hosted plant 41423701978).
- [x] **T6 — J1 real-spawn bench (AC6).** Tier W soak target + hosted smoke + hosted plant. **LANDED under ruling r1 close:** `1fd4de31`, `850e4c00`; hosted smoke + plant 41423701978 (the 1 h Tier W soak AC6.1 is moved, see below).
- [x] **T7 — S3 (AC7)** under the ruled kernel row: stub vector red-before, K1w, S3 test rework, measurement 3, local plant. **LANDED under ruling r1 close:** `1fd4de31`; hosted `s3-both` green on 24.04 and 26.04 in 41423701978.
- [x] **T8 — ac4 (AC8)** under the ruled row: diag result → K2r or oracle; podman-shim hosted witness and plant; oracle context. **LANDED under ruling r1 close:** `1fd4de31`; amended by D-20-7-11 C (`a8af3215`): K2r is defense-in-depth, AC8.3 retracted, cause still unidentified.
- [x] **T9 — Supervisor (AC9).** Drop repair, red-before vector, diagnostics, atomic stub records, loaded runs, plant. **LANDED under ruling r1 close:** `1fd4de31`, `9f64dd02`; AC9.4 stays open by rule 10 (48/50 green, original hang unreproduced).
- [x] **T10 — cap-token (AC10).** Guard + honest overall test; metric rows. **LANDED under ruling r1 close:** `1fd4de31`.
- [x] **T11 — L1 landing (D-20-7-05 A).** Exact pins inside the ruled envelope (kernel both pins + content hashes `t3/child.rs` (+ `Cargo.toml` feature)); FLAG-Winston decision record (precedent `20-3c-flag-winston-decision.md`); `check-kernel-baseline`, `kloc-check`, `check-service-boundary`, `check-dev-record-completeness`, `check-epic-close-coherence`, `prepush`; one hosted discipline run on the final tree + the `ci/20-7-verify` plant run; flakes that red it recorded by name/run id (D13 A). Revert `intent-lineage-coverage-report.md` if a run modifies it. No commit or push without the operator. **LANDED under ruling r1 close:** `1fd4de31`, `9f64dd02`, `850e4c00`, `ac1fc12a`, `5fe97eda`, `d00802aa`; main discipline run 43150757804 @ `d00802aa` green; kernel 26841/21042 exact; FLAG-Winston record `20-7-flag-winston-decision.md`.
- [ ] **T12 — L2 shadow (AC11.1)** — starts when `ops-stable-perf-runner` is `done` and records-owed §O items O1–O5 are satisfied: fingerprint + calibration report committed with the runbook; ≥ 2 weeks / ≥ 20 main runs. **MOVED to `ops-stable-perf-runner` under ruling r1 (operator, 2026-11-12; `_bmad-output/planning-artifacts/decisions-2026-11-12-acceleration.md` item 8); not ticked, not done.** Claim boundary: "perf verdicts ABSENT; no stable-runner timing claim".
- [ ] **T13 — L2 bind and retire (AC1.4, AC11.2–11.4, AC12.1, AC13).** Runner reds recorded; tolerances from data; release precondition; retirement; ledger closure; final landing. **MOVED to `ops-stable-perf-runner` under ruling r1 (operator, 2026-11-12; `_bmad-output/planning-artifacts/decisions-2026-11-12-acceleration.md` item 8); not ticked, not done.** Claim boundary: "perf verdicts ABSENT; no stable-runner timing claim". (The release-precondition perf-verdict leg of AC11.2 and the `--perf-absent-ok` rc waiver landed separately in `d00802aa` under 20-5 L2.)

## Dev Notes

### What changes, by file

| Area | Files | AC |
|---|---|---|
| Kernel ◆ | `crates/maos-kernel-core/src/security/sandbox/t3/child.rs:273-284,401-466`; `crates/maos-kernel-core/Cargo.toml:68` (`rustix` features, K2r); pins `xtask/kernel-core-baseline.toml:505` + EOF HISTORY, `xtask/kloc.toml:264` | AC7, AC8 |
| xtask | NEW `xtask/perf-budgets.toml`, `xtask/src/check_perf_verdict.rs`, `xtask/src/check_perf_runner_policy.rs` (+ ledger append, Tier C compare); `xtask/src/main.rs` (subcommands near `:645,791`); `xtask/src/check_release_precondition.rs:33-96`; `xtask/gate-registry.toml:457-459` + new rows; `xtask/kloc.toml:339` | AC1–AC5, AC11 |
| Bench | `crates/maos-bench/src/harness/{j1.rs:25,162-170,j4.rs:34-36}`, `src/decision.rs:14-15`; `benches/{iac_routing_budget.rs,orchestrator_fanout_nfr_perf_8.rs,j4_latency_regression.rs}` + new benches / Tier C harness; `crates/maos-kernel-core/benches/{hot_swap_latency,cap_token_verify_p99}.rs` wiring | AC3–AC5, AC10 |
| Bin | `crates/maos-bin/src/supervision.rs:1453-1470,2503-2504` (AC9); real-spawn harness (AC6) | AC6, AC9 |
| Tests (uncharged) | `crates/maos-bin/tests/{root_shutdown_unload_16_3.rs:342-391,worker_terminal_facts_17_7.rs:195-313,397-456}`, `…/worker_terminal_facts_17_7/{supervisor_scenes.rs:646-806,stub_scenes.rs:25-41}`; `crates/maos-kernel-core/tests/cap_token_verify_assertion.rs`; `xtask/tests/` | AC1–AC10 |
| Workflows | NEW `.github/workflows/perf.yml`; `discipline.yml` (policy + Tier C + real-spawn smoke jobs; L2 deletions `:2970-3055`); `ci/20-7-diag`, `ci/20-7-verify` branch-only workflows (never merged) | AC2, AC5, AC6, AC8, AC12 |
| Docs / records | NEW `docs/runbooks/perf-runner.md`; `tests/coverage-matrix.yaml:1380-1436`; ADR-068 `Gate:` line (`:3`) when the gates land; `deferred-work.md` rows (AC13) | AC12, AC13 |

### Overlap (serialize; re-measure)

- **`maos-kernel-core` pins** — 20-8, 20-9, 20-13, 20-14 forecast kernel touches; one integration owner per pin update (PF §6 "Pooled write surfaces"). K1w/K2r touch `t3/child.rs` only; 20-10 (purge of owned T3 residue) reads the same residue categories — coordinate the `TerminationUnconfirmed` meaning. → Coordinated at 20-10 creation: purge consumes `TerminationUnconfirmed` as "liveness unknown, CID/state retained" and never emits it (20-10 AC5.1).
- **`xtask` and `discipline.yml`** — 20-5 (release precondition: fuzz chain + perf verdict share `check_release_precondition.rs`), 20-16 (`prepush` set), 20-15 (build-once archive for the new jobs).
- **`maos-bin/src/supervision.rs`** — 20-9 (spawned lifecycle) edits supervision; AC9's change is local to the drop/unload pair.

### Current state that must be preserved

- FR12/NFR-Rel-1 floors as proven by `worker_crash_corpus_16_3` (≥ 99/100 within 2 s / 5 s) — AC7.3 leans on it; never weakened.
- NFR-Rel-11 planned half: every planned stop receipts (`root_shutdown_unload_16_3.rs` header, `:1-48`); the 5 s orphan bound stays.
- T3 teardown order kill → confirm → snapshot → `rm` → `exists` and its typed residues (`child.rs:321-324`); a final verdict is never retried; a non-terminal state is never cached as an exit (`:211-214`).
- The supervisor's pid-reuse safety: signals only through pidfds (`supervision.rs:3201-3210`); K2r keeps that (pidfd, never a bare pid).
- `check-j4-latency`'s blocking sample-count, integrity and mutation steps (`discipline.yml:4867-4896`).
- 20-3a's EvidenceState vocabulary (`PASS/FAIL/INDETERMINATE/ABSENT`); only `PASS` is green.

### Guardrails

- No kernel line without the ruled FLAG-Winston row; any `crates/maos-kernel-core/src` change outside AC7/AC8's shapes is STOP.
- Never raise a 5 s orphan bound, never `#[ignore]`/retry a family test to get green (D13 A). AC7.3's bound change is the documented budget made explicit, ruled under D-20-7-01, not a timeout edit to pass.
- The perf host is never given a write token, a repository secret, or a fork's code (ADR-068 §4); no dev step "simulates" a runner, a fingerprint or a ledger entry — `ABSENT` is the honest reading until L2.
- One cargo process per checkout; `env -u CI -u GITHUB_ACTIONS`; inherit the target directory, never override or clean it; plants byte-restored in the real tree (`proof-contract.md:24-31`).
- A shared target dir across worktrees evicts sibling binaries tests resolve by path (`deferred-work.md:994` lesson).

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red | Leg |
|---|---|---|---|
| Verdict (AC1) | best-of-five instead of median | noisy-spread vector | local |
| Runner policy (AC2) | ×4 (second workflow, no fork guard, `pull_request_target`, wide permissions) | gate | local |
| One budget file (AC3) | re-added J4 const | literal-scan vector | local |
| Bench completeness (AC4, AC12.3) | delete NFR-Perf-4 bench | completeness vector | local |
| Tier C (AC5, AC10) | extra loop in verify path | Tier C job | hosted |
| Real spawn (AC6) | in-process substitution | distinct-CID assertion | hosted |
| S3 removal (AC7) | restore early return | stub vector | local |
| ac4 pidfd kill (AC8) | remove pidfd signal (podman `kill` shim) | `ac4_v4` | hosted |
| Drop panic (AC9) | bare `block_on` | single-panic vector | local |
| Release precondition (AC11) | — (failing-before vector) | missing-verdict vector | local |
| Verdict on the runner (AC1.4) | J4 15 ms fault inject; noisy calibration | `FAIL`; `INDETERMINATE` | runner (L2) |

### Tests and commands

```bash
# local; one cargo process per checkout; inherit the target directory
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p xtask -p maos-bench -p maos-bin -p maos-kernel-core
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --lib
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test worker_terminal_facts_17_7 --test root_shutdown_unload_16_3   # T3-capable host only; local SKIP lines are not evidence
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-perf-runner-policy
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-perf-verdict --sha "$(git rev-parse HEAD)" --ledger xtask/tests/fixtures/perf-ledger/  # fixture path chosen at T3
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-kernel-baseline
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- kloc-check --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-dev-record-completeness
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-epic-close-coherence
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- prepush
```

### Review plan (`proof-contract.md:33-71`)

DIFFICULT: Blind + Edge (opus-5.5), Acceptance (opus-5.5), Security reading on AC2 (fork → runner) and AC8 (a new
kill path into a container pid namespace), Test Infrastructure (glm-5.3). Focused suites per touched crate +
`cargo test -p maos-kernel-core --lib`; the reviewer reads the hosted plant runs, never re-runs them.

### Previous-story intelligence (20-3c, 20-3b, 19-3, 18-0, 17-7, 16-3)

- 20-3c: known 20-7 flakes reddened two of its three landing runs (41281999213: `ac4_v4` + S3; 41289787129: `ac4_v4`) and were recorded by name and job id under D13 A — the pattern this story ends. Its two-landing shape (D-20-3c-10 A) is D-20-7-05 A's precedent; its review-correction growth is D-20-7-08's.
- 20-3b: three review-correction raises after creation (w8 → w10) — hence the ×2/×3 allowance.
- 19-3: D-19-3-11 C routed the real-spawn bench here because every Worker is T3 and a T0 bench measures a path nothing runs (`19-3-…md:108`).
- 18-0: the one-full-rerun allowance for these 5 s vectors (D-18-0-01 A) was 18-0-only and went unused (`deferred-work.md:1257`); the pidfd ESRCH race repair (`:1254-1255`) is the precedent for acknowledgement barriers over deadlines.
- 17-7: the 2026-10-14 ruling raised podman 5's S3 bound to `SNAPSHOT_BUDGET + 2 s` and kept 5 s on podman 4 on the claim "FR12's 5 s holds on podman 4" (`worker_terminal_facts_17_7.rs:407-409`) — the hosted runs above contradict that claim.
- 16-3: `ROOT_GRACE` 5 s is "the honest alternative to hanging" until 17-1's container teardown (`supervision.rs:2611-2619`); the 16-3 crash corpus (100 Workers, batches of 20) is AC6's harness shape.

### Git intelligence

`a4c00897` (20-3c done records; clippy baseline 1383/897), `77f7276a` (shared-support clippy fixes), `42489813` (20-3c
hosted clippy fixes), `1a1b11f3` (20-3c review patches), `a41a143f` (20-3c kernel L2). No change to `t3/child.rs`,
`supervision.rs`'s stop path or the flake tests since the families were first recorded.

### Library / framework

No new dependency under the recommendations: `rustix` (already a kernel-core dependency, `Cargo.toml:68`; `process`
feature already enabled by maos-bin `Cargo.toml:123`) for K2r; `serde_yaml`/`toml` (already xtask dependencies) for
the policy gate and budget file; `criterion` (already used by the benches). Tier C under D-07 A uses the
`valgrind` apt package on hosted images (measurement 6), no crate. Podman behaviour: exit 125 is podman's own error
(`https://docs.podman.io/en/v5.1.2/markdown/podman.1.html`); the `start --attach` exit-code race is [INFERENCE],
not reproduced here.

### References

E20@HEAD:79,236-253,381; preflight `epic-20-preflight-2026-10-30.md:50,81,125,138,142,223-236,295,297,306-308`;
tracker `sprint-status.yaml:289,304,326-327`; `deferred-work.md:96,1075,1181-1198,1256-1259,1297-1298`;
`routing-audit-2026-10-22.md:48`; PRD `functional-requirements.md:41`, `non-functional-requirements.md:9-16,20,30`,
`success-criteria.md:40-46`; ADR-068 (`docs/adr/ADR-068-stable-performance-runner.md:1-191`), ADR-030, ADR-040,
ADR-069 (`:35-46`); `docs/runbooks/provisioning-checklist.md:44`; 19-3 `…md:108,132,293`.

## Dev Agent Record

**Incident:** Hosted landing run `41372776259` on main `9f64dd02` had `clippy-advisory` red at 1385/899 (fixed at source; local 1339/875), and `check-exit-commands` and `check-decision-register` red from the stale `20-1` key after the split (fixed). The split-key rename broke two hosted doc gates that prepush does not run; when renaming story keys, run `check-exit-commands` and `check-decision-register`.

### Story context creation

Created 2026-11-01 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter measured contract) at `a4c00897`,
without cargo (parent rule).

**Inputs:** E20 §20-7 at HEAD (amended 2026-10-30, PF §3.4) — stale cites corrected here and owed to E20
(`creation/records-owed.md` §A); ADR-068; preflight D11/D13 and §2/§6 rows; tracker `:304`, `:327`; every
`deferred-work.md` row naming 20-7; 19-3's D-19-3-11 C; 14 hosted job logs (`creation/ci-excerpts/`); direct reads of
every cited range (`creation/cites-a4c00897.md`); kloc and kernel by the prebuilt gate binary on `/tmp/proto207`
`git archive` copies (`creation/kloc-kernel-a4c00897-and-prototypes.json`); prototypes K1, K1w, K2, K2r, R, R′, R″ saved as
diffs, `rustfmt`'d, never applied to the tree.

**Measurements owed:** `creation/measurements.md` "Not measured here" 1–6.

**Records owed:** `creation/records-owed.md` (§A epic corrections for the orchestrator; §O operator prerequisites with owners; rulings; landing).

**Revert:** no crate source modified (prototypes only under `/tmp/proto207`, deleted); no cargo/test ran;
`intent-lineage-coverage-report.md` untouched.

**Gates after the record edits** (prebuilt `/mnt/build/cargo-maos/debug/xtask`, `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=…/xtask`, `--json`, in-tree; `creation/gates/*.json`): `check-dev-record-completeness` rc 0 (197 done stories, 150 deferred-owner assertions, 0 violations); `check-epic-close-coherence` rc 0 (21 epics, pin 26801, 0 violations); `kloc-check` rc 0 (aggregate 188816, `alarm=true`, `over_budget=[]`); `check-kernel-baseline` rc 0 (26801 / 95 files, set hash `c023d0d7…`, no drift).

### Agent Model Used

anthropic/claude-opus-5-5 (creation); landing 1 by the Dev unit (`Dev207L1`, `anthropic/claude-sonnet-5-5`), 2026-11-01/02. No status flipped; no commit; no push. Closure record under r1 (story/ledger/tracker text only, no code) by `Close207Docs` (`anthropic/claude-sonnet-5-5`), 2026-11-12.

### Debug Log References

Plants and hosted result index: `20-7-stable-runner-perf-gates-evidence/landing-1/plants.md`. Branch-only workflow: `…-evidence/ci/ci-20-7-verify.yml`. FLAG-Winston record: `20-7-flag-winston-decision.md`.

### Completion Notes List

**Landing 1 (D-20-7-05 A) — machine-free, as ruled in `rows207`.**

*Per-AC witness (local, observed):*

- **AC1 (fixture half)** — `check-perf-verdict` (`xtask/src/check_perf_verdict.rs`), fixture ledger `xtask/tests/fixtures/perf-ledger/`, 24 vectors in `xtask/tests/perf_gates_20_7.rs`: PASS / FAIL / INDETERMINATE (noisy spread, noisy or absent calibration, new fingerprint until a `rebaseline` record) / ABSENT, loud missing-ledger and malformed-line errors, staleness. Plant (best-of-five) reds `the_median_decides_a_budget_breach_not_the_best_of_five`. Runner leg (AC1.4) is L2.
- **AC2** — `check-perf-runner-policy` + `.github/workflows/perf.yml` (`MAOS_PERF_RUNNER_ENABLED` variable gate so no job queues for a runner that does not exist), enrolled blocking in `discipline.yml`, `gate-registry.toml`, `EXPECTED_GATES` (47). Four plants on the real `perf.yml`/`container.yml` each red `xtask check-perf-runner-policy` (rc 1), clean tree rc 0.
- **AC3** — `xtask/perf-budgets.toml`; `maos-bench/src/budgets.rs`; the seven literal sites + cap-token test read it; `budget_literals` vector reds on all eight sites at `a4c00897` (`MAOS_20_7_TREE`) and on the J4 re-add plant.
- **AC4** — `iac_routing_budget` P50/P99 (+ `iac/send`), new `iac_throughput_nfr_perf_2`, `posture_shift_nfr_perf_4`, `j1_bridge_latency`, `kernel-core/benches/hot_path_budgets.rs`, `hot_swap_latency` and `cap_token_verify_p99` wired to rows; `orchestrator_fanout_nfr_perf_8` renamed `orchestrator_inproc_dispatch` (not the NFR-Perf-8 claim); completeness vector (reds at `a4c00897`, on the NFR-Perf-4 bench deletion). Benches print `MAOS-PERF` lines and exit non-zero only on a `blocking` row (none at L1: all Tier W rows advisory). The benches were compiled (`--all-targets`) but NOT RUN locally. The NFR-Perf-5 WASM `recall` leg is OWED to landing 2 (blocker: no WASM guest harness; deferred row filed; coverage row says so).
  **Ruling:** D-20-7-10 A amended AC4.1: WASM recall remains unclaimed and blocked on the guest harness in L2.
- **AC5 / AC10** — `tier_c_harness`, `check-perf-counts` and hosted callgrind: D-20-7-09 B is done. Binding instruction baselines are 12944.0156, 997449.662 and 188885.9382; allocation-neutral plant yields instruction FAIL and allocation PASS in run 41423701978. Measurement provenance and allocation baselines are in the evidence index. AC10 retains the NFR-Perf-3 100 µs catastrophe guard and verify + audit-enqueue overall test.
- **AC6** — `crates/maos-bin/tests/nfr_perf_8_real_spawn_fanout_20_7.rs` (real T3 Workers through `WorkerSupervisor` via `run_cli_wrapper_manifest`, re-exec child, `cli_wrapper_loaded`/`cli_wrapper_exit` events). Compiles; SKIPs without `MAOS_20_7_FANOUT`; NOT RUN (no T3 substrate here). Deviation accepted by the orchestrator: the dispatch stamp is the harness's, immediately before the supervised run, not a `task.assign` journal row.
  **R10 smoke correction (operator-authorized through Main, 2026-11-01):** the hosted smoke now caps at five Workers, schedules one task/s for 60 s and requires at least two concurrently loaded Workers. The previous five-Worker/2-second fixture cannot sustain ten arrivals/s (theoretical capacity at most 2.5/s before launch overhead). Smoke proves real spawn/lifecycle pairing and lossless scheduled/admitted/completed accounting only. The unchanged runner-only soak retains 50 concurrent, ten tasks/s, 3600 s and zero drops; only that run can prove NFR-Perf-8. Neither run is claimed executed locally.
- **AC7 (K1w)** — `child.rs` `snapshot_settled`; stub vector `teardown_waits_for_a_stalled_snapshot_owner_then_removes_the_container_20_7` (20 instances): green after K1w (9/9 reruns incl. 8 under full-core load); K1w plant reds 3/20, 3/20, 2/20 instances. S3 test reworked (facts first; runtime facts only when podman answered; bound `SNAPSHOT_BUDGET + 2 s` with its reason; failure message carries elapsed, client exit line, residue line, stderr). One unexplained red of that vector (10.055 s) in the first run right after a large build; 14 later runs green, message lost. AC7.4 conditional: not triggered locally — a stalled-runtime sample classifies `fault.truncated` by the 17-7 rule (`unavailable state is always truncated`), `maos-domain` untouched; measurement 3 is hosted-only; deferred row filed (rule 10(b)). AC7.5 hosted S3 on 24.04/26.04 unrun.
- **AC8 (K2r)** — pidfd SIGKILL, zombie-aware oracle and diagnostic context retained as defense-in-depth. Hosted diagnostic at post-repair HEAD was 20/20 green, not a K2r-vs-oracle decision. Base passed under the shim, contradicting AC8.3; HEAD passed and HEAD-minus-K2r's composite plant left a live payload. D-20-7-11 C retracts historical-witness equivalence and does not attribute the historical flake to a proven mechanism.
- **AC9** — `block_on_anywhere` (`block_in_place` on a multi-thread runtime thread, scoped thread otherwise) in `unload`, `disposition`, `join_crash_handler`; forced barrier failure vector `barrier_failure_unwinds_with_exactly_one_panic_20_7` (re-exec child; green; bare-`block_on` plant reds); hang guard prints calls log, binding phase, stop-task state; one `printf` per stub record. **55 loaded runs** of the supervisor vector (32 busy-loops) passed 55/55; the original 20 s no-kill-record hang was not reproduced — D-row stays open with the new diagnostics (rule 10: owner this story, landing 2).
- **AC12 (L1)** — `docs/runbooks/perf-runner.md`, coverage rows NFR-Perf-1…5/7/8 name their own gates (`gate-registry.toml` metric-family names), ADR-068 `Gate:` line.

*Measured vs ceilings (tokei, `kloc-check --json` after `cargo fmt --all`):* kernel 21042 / physical 26841 (ceiling 21073 / 26881); xtask 43910 (45830); maos-bench 2737 (3051); maos-bin 33215 (33315); aggregate 190203, hardfail pinned 190204 (192568). Every other crate unchanged (`kloc-check` `over_budget` lists only the four).

*Verification (all `env -u CI -u GITHUB_ACTIONS`, inherited target dir):* `cargo fmt --all` rc 0; `cargo build --locked --all-targets` xtask, maos-bench (`kernel_measurement`), maos-kernel-core, maos-bin: no errors; `cargo nextest run --workspace --locked --no-fail-fast --profile ci --test-threads 1`: 5503/5504 passed (613 s); default profile: 5503/5504. The single failure in both is `xtask::invariant_claims_20_3b committed_ledger_covers_all_fourteen_and_keeps_requirement_ownership` (expects owner `20-1`, the working tree now says `20-1b-vetting-yank-policy-and-quarantine` in `docs/invariants/I5.md`, edited by the 20-1 lane — not a 20-7 change). Gates rc 0: `kloc-check`, `check-kernel-baseline` (26841 / 95 files), `check-service-boundary`, `check-env-contract`, `check-serde-error-handling` (allowlist row re-pointed `orchestrator_fanout_nfr_perf_8.rs:245` → `orchestrator_inproc_dispatch.rs:236`, row for the deleted `iac_routing_budget.rs:167` removed; nothing weakened), `check-composition-root-completeness`, `check-j1-loopback-delegation`, `check-dev-record-completeness`, `check-epic-close-coherence` (pin 26841), `check-review-findings-resolved`, `check-test-cost`, `check-test-budget` on the serial junit (613 s of 1100 s), `invariant-lock`, `coverage-matrix`, `check-coverage-matrix-completeness`, `check-ship-gate-completeness` (47), `check-perf-runner-policy`, `check-perf-verdict` on the fixture ledger. `clippy-ratchet` against a real non-denying stream: 1338 occurrences / 875 source-distinct (hosted baseline 1383 / 897, unchanged; no growth). `cargo deny check`: advisories, bans, licenses, sources ok. Prescribed prepush: see Debug Log below.

*Owed (superseded 2026-11-12 by ruling r1: AC1.4, AC6.1, AC11, AC12.1, T12, T13 moved to `ops-stable-perf-runner`; AC13 closed in the closure note):* stable-runner AC1.4, AC11, AC12.1, AC13 and release-precondition ledger read are L2; WASM recall is blocked on the guest harness. Hosted branch results and their limitations are in the evidence index. `perf.yml` and the discipline `check-perf-counts` job were not part of that branch proof.

### Landing 1 review corrections — observed verification (2026-11-02)

Per-finding changes, vector names/lines, commands/exit codes and remaining boundaries:
[review correction report](20-7-stable-runner-perf-gates-evidence/landing-1/review-corrections/report.md).
At review time D-20-7-09 and D-20-7-10 were proposals; the 2026-11-02 rulings above supersede that state. No status flip is claimed.

- Final formatted measurements: kernel **26841 physical / 21042 tokei**, unchanged
  (95 files, unchanged hashes); xtask **44297**, maos-bench **2737**, maos-bin
  **33215**, aggregate **190590**. Exact D11 pins: xtask 44297 and aggregate
  hardfail 190591; HISTORY names this review correction and full sprint key.
  Every ruled ceiling is respected; no kernel growth.
- All-target builds (affected packages with kernel measurement, and the full
  workspace) exit 0; focused gate vectors **35/35**, fan-out binary **3/3**
  (real T3 path SKIP). Full workspace serial **5517/5517**, 613.984s; default
  **5517/5517**, 41.165s; 138 skipped in each. Initial D11 citation failures
  were corrected before the complete green serial/default rerun.
- Every requested structural/performance gate exits 0 with its stated evidence
  boundary. Serial JUnit test-budget: **613.790 / 1100s**, 586 binaries.
  Fixture verdict rows for real metrics remain **ABSENT/advisory**; count-gate
  instruction inputs are labelled synthetic fixtures, not hosted measurements.
- Real successful clippy stream: **1339 occurrences / 876 source-distinct**,
  below unchanged hosted **1383 / 897**; ratchet exit 0. `cargo deny check`
  exit 0. Prescribed prepush exit 0: **84 checks, 8 skipped-with-reason,
  0 red**, 82 captured SKIP diagnostics; hosted CI remains authoritative.
- Production-signing optimized bench measured token issuance **P99 39.514 us**
  locally, **above the unchanged advisory 5 us budget**. This is one local
  process, not stable-host/five-process evidence; no budget was relaxed.
- AC9.4 now has **50 whole-package maos-bin loaded runs** (765 tests / 98
  binaries per run, 32 CPU-load processes): **48 green, 2 red**. Run22:
  `purge_refuses_live_store_without_changing_filesystem_identity`; run50:
  `ac1_19_5_cold_swap_admits_the_successor_at_a_new_pid`. Both named reds remain
  open outside these correction files; no retry-to-green. Original supervisor
  hang not reproduced. Logs and load-process cleanup receipt are retained.
- Five local regression plants each red, then byte-identical restores proved.
  The new instruction-only CPU plant keeps **10000 allocations / 10000
  iterations** before and under mutation; instruction-red remains hosted-only.
  O3/O7 are explicit **OPERATOR prerequisites**, not settings claimed verified.

### File List

- .github/workflows/discipline.yml
- .github/workflows/perf.yml
- _bmad-output/implementation-artifacts/20-7-flag-winston-decision.md
- _bmad-output/implementation-artifacts/20-7-stable-runner-perf-gates-evidence/ci/ci-20-7-verify.yml
- _bmad-output/implementation-artifacts/20-7-stable-runner-perf-gates-evidence/ci/tier-c-plant.diff
- _bmad-output/implementation-artifacts/20-7-stable-runner-perf-gates-evidence/closure/closure-under-r1.md
- _bmad-output/implementation-artifacts/20-7-stable-runner-perf-gates-evidence/creation/records-owed.md
- _bmad-output/implementation-artifacts/20-7-stable-runner-perf-gates-evidence/landing-1/plants.md
- _bmad-output/implementation-artifacts/20-7-stable-runner-perf-gates-evidence/landing-1/review-corrections/report.md
- _bmad-output/implementation-artifacts/20-7-stable-runner-perf-gates.md
- _bmad-output/implementation-artifacts/deferred-work.md
- crates/maos-bench/Cargo.toml
- crates/maos-bench/benches/audit_query_latency.rs
- crates/maos-bench/benches/iac_routing_budget.rs
- crates/maos-bench/benches/iac_throughput_nfr_perf_2.rs
- crates/maos-bench/benches/j1_bridge_latency.rs
- crates/maos-bench/benches/j4_latency_regression.rs
- crates/maos-bench/benches/orchestrator_inproc_dispatch.rs
- crates/maos-bench/benches/posture_shift_nfr_perf_4.rs
- crates/maos-bench/src/bin/tier_c_harness.rs
- crates/maos-bench/src/budgets.rs
- crates/maos-bench/src/decision.rs
- crates/maos-bench/src/harness/j1.rs
- crates/maos-bench/src/harness/j4.rs
- crates/maos-bench/src/lib.rs
- crates/maos-bin/src/supervision.rs
- crates/maos-bin/tests/erasure_million_20_11.rs
- crates/maos-bin/tests/nfr_perf_8_real_spawn_fanout_20_7.rs
- crates/maos-bin/tests/one_daemon_one_door_16_1.rs
- crates/maos-bin/tests/root_shutdown_unload_16_3.rs
- crates/maos-bin/tests/worker_terminal_facts_17_7.rs
- crates/maos-bin/tests/worker_terminal_facts_17_7/stub_scenes.rs
- crates/maos-bin/tests/worker_terminal_facts_17_7/supervisor_scenes.rs
- crates/maos-kernel-core/Cargo.toml
- crates/maos-kernel-core/benches/cap_token_verify_p99.rs
- crates/maos-kernel-core/benches/hot_path_budgets.rs
- crates/maos-kernel-core/benches/hot_swap_latency.rs
- crates/maos-kernel-core/src/security/sandbox/t3/child.rs
- crates/maos-kernel-core/tests/cap_token_verify_assertion.rs
- crates/maos-kernel-core/tests/cgroup_ceiling_smoke.rs
- crates/maos-kernel-core/tests/t3_terminal_helpers_17_7.rs
- docs/adr/ADR-068-stable-performance-runner.md
- docs/runbooks/perf-runner.md
- docs/runbooks/provisioning-checklist.md
- tests/coverage-matrix.yaml
- xtask/gate-registry.toml
- xtask/kernel-core-baseline.toml
- xtask/kloc.toml
- xtask/perf-budgets.toml
- xtask/src/check_perf_counts.rs
- xtask/src/check_perf_runner_policy.rs
- xtask/src/check_perf_verdict.rs
- xtask/src/check_release_precondition.rs
- xtask/src/lib.rs
- xtask/src/main.rs
- xtask/src/perf_budgets.rs
- xtask/src/perf_ledger.rs
- xtask/tests/fixtures/perf-ledger/budgets.toml
- xtask/tests/fixtures/perf-ledger/ledger.jsonl
- xtask/tests/kernel_pin_content_hash_16_0.rs
- xtask/tests/perf_gates_20_7.rs
- xtask/tests/release_precondition_perf_20_5.rs

### Closure under ruling r1 (2026-11-12)

Status flipped to `done` by Main under ruling r1 (operator, 2026-11-12; `_bmad-output/planning-artifacts/decisions-2026-11-12-acceleration.md` item 8): the story closes at L1 plus the rc waiver. L2 (T12, T13) and the runner-only ACs (AC1.4, AC6.1, AC11.1-11.3, AC12.1) live on the UNCLAIMED row `ops-stable-perf-runner`. **Claim boundary: "perf verdicts ABSENT; no stable-runner timing claim".** This is not a widening of any time budget and not retry-to-green. Per-AC and per-task table, row-by-row ledger dispositions and counts: `20-7-stable-runner-perf-gates-evidence/closure/closure-under-r1.md`.

## Change Log

- 2026-11-01: Story created at `a4c00897` (ready-for-dev; D-20-7-01…08 PROPOSED). Four flake families diagnosed from hosted logs: S3 and ac4 are budget-sized stalls with defective cleanup/records (kernel R = K1w + K2r priced +40/+31 by reverted prototype), the supervisor barrier's cause is open with a deterministic drop-panic defect, `cap_token_verify_p99` is an instrument problem. Brief corrections: S3 did not fail in 41233186774; "41275…" is no run id.
- 2026-11-01: Operator rulings recorded (ask rows207): D-20-7-01…08 A, D11 and D13 A (already); kernel row R = K1w + K2r (+40 / +31) and ceilings per frontmatter `kernel_grant`/`kloc_grant`; both kernel grants re-base rule (20-1b holds the other). Status line and tracker unchanged by this entry: the tracker row `20-7` is `in-progress` (sprint-status DEV START note).
- 2026-11-02: Landing 1 (machine-free) implemented by the Dev unit: K1w + K2r kernel row landed at exactly the prototype's 26841 / 21042 (ceiling 26881 / 21073), budget file + `check-perf-verdict` / `check-perf-runner-policy` / `check-perf-counts`, benches, Tier C harness, flake repairs, `perf.yml`, runbook, FLAG-Winston record. Hosted-only legs (`ci-20-7-verify.yml`) and the L2 legs unrun. No status flip.
- 2026-11-01: L1 review corrections harden runner admission policy and trusted append workflow, validate count/budget/ledger inputs, bind process and calibration identity, preserve temporal verdicts, use production signing and strengthen fan-out/diagnostic accounting. D-20-7-09/10 are PROPOSED scope amendments only; no story/status flip.
- 2026-11-02: D-20-7-09 B done with hosted binding baselines and allocation-neutral plant; D-20-7-10 A moves WASM recall to L2; D-20-7-11 C retains K2r as defense-in-depth, retracts AC8.3 and AC8.5 historical-witness equivalence after run 41423701978 contradicted the base-red claim. Historical orphan cause remains open; no further hosted cycle.
- 2026-11-12: Closure under ruling r1 (operator, 2026-11-12; `_bmad-output/planning-artifacts/decisions-2026-11-12-acceleration.md` item 8): T0-T11 ticked with landed commits (`1fd4de31`, `9f64dd02`, `850e4c00`, `a8af3215`, `ac1fc12a`, `5fe97eda`, `d00802aa`); T12/T13 and AC1.4/AC6.1/AC11/AC12.1 moved to `ops-stable-perf-runner` (claim boundary "perf verdicts ABSENT; no stable-runner timing claim"); AC8.4 amended for the rc (5 s product bound unchanged; hosted timing-noise reds dispositioned, cause unidentified per D-20-7-11 C); File List rewritten as path bullets; `deferred-work.md` rows closed / dispositioned / re-owned in one pass (see closure note); status `done` at the tracker.
