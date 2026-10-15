# 17-7 FLAG-Winston decision record

**Date:** 2026-10-13  
**Status:** `ratified` 2026-10-13 by the operator (Lunarpulse): **variant A**, the numerical rule as proposed, and FW-04, FW-07, FW-08 and FW-09. Story creation measured the E16-A7 prototype, filed epic §R12 (R17-117…R17-132) and passed a three-validator non-author check (story §Non-author validation) before the ruling.  
**Scope:** the original story-creation/measurement charter and the scoped Story 17-7 approval exception D-17-7-FW-10 below. This record changes no runtime, kernel, baseline, KLOC ledger, ratification file, invariant text or ADR. Figures move only in 17-7's landing commit.
**Model after:** `17-2-flag-winston-decision.md` and `17-1-flag-winston-decision.md`. A FLAG-Winston is an architectural *location charter*, not a numerical grant (`17-3c-early-start-flag-winston-decision.md`, D-17-3c-ES-04).

## Decision

| ID | Decision | Status |
|---|---|---|
| **D-17-7-FW-01** | **Evidence variant A and kernel locations.** Podman cannot tell `exit(137)` from an external SIGKILL (conmon `128+N`; no signal field in inspect), so 17-7 reads the payload's own wait status via a pidfd and `PIDFD_GET_INFO`/`PIDFD_INFO_EXIT` (Linux ≥ 6.15; runners 6.17 and 7.0) and delivers the epic's AC1 as written. Where that evidence is unavailable for a spawn (older kernel, `ESRCH`, unverifiable identity), the story's runtime-evidence fallback rows apply; PID 1 must be `/maos/spirit` (no image entrypoint, no `--init`).<br>Kernel locations: `security/sandbox/t3/argv.rs`, `t3/spawn.rs`, `t3/child.rs`, `t3/mod.rs` (docs and DR-5.5a-7), `lifecycle/cli_wrapper/runtime.rs`, `lifecycle/cli_wrapper/mod.rs` (re-exports) and `security/sandbox/linux.rs` — the existing unsafe zone (`security/sandbox/mod.rs:3-6`). `t3/image_lock.rs` is **not** in the charter: the prototype touched it only for probe scaffolding. `supervision/crash_detector.rs`, `scheduler/*` and `hook_dispatch.rs` are not touched. Any other kernel file returns as a charter request. | **Ratified 2026-10-13** |
| **D-17-7-FW-02** | **Kernel numerical ceiling = the measured variant-A prototype figure:** `src_lines` 25145 → **≤ 25574** (+429), kloc 19492 → **≤ 19946** (+454); the exact lower figure is re-pinned at landing (17-1 precedent). No speculative margin. The prototype includes 79 lines of probe scaffolding and omits pieces the story requires (CID guard before `start`, terminality gate, bounded helper, held-inode reader, typed residue, docs), so the complete implementation may land below or above the ceiling; any excess is a newly formatted, measured request before landing, never a re-pin. `RATIFIED_AT_EASING` moves with the kernel kloc pin. | **Ratified 2026-10-13** |
| **D-17-7-FW-03** | **Out-of-kernel ceilings: exact measured figures at landing**, each returned to the operator as a measured request **before** landing, because every affected row is an exact zero-headroom pin: `maos-domain` 9349 (prototype +47 tokei for the terminal-facts types and the observed PID), `maos-bin` 25396 (snapshot feed, terminality, bounded CID stop sequence, post-unload stop, lease state, lifecycle-journal threading, state dir, seams), `maos-audit` 7341 (the second exact `cli.subprocess.exit` shape), `maos-egress` 1769 (only if the *termination unconfirmed* lease needs a revoke-only seam in `proxy.rs`), `xtask` 44370 (workflow fan-in test), `maos-eval` 4831 (hold deletion lowers it), `_aggregate_hardfail` 173501 (measured+1). No envelope is pre-authorized: the ledger forbids speculative reservation (`xtask/kloc.toml:60-89`). | **Ratified 2026-10-13** |
| **D-17-7-FW-04** | **No ADR amendment.** ADR-061's transport, credential custody and `--network=none` are unchanged; ADR-067's purge stays report-only (`:87-90`) — the live owner removes its own CID, and no boot-time sweep is added (FW-07). DR-5.5a-7 ("PID identity = host-namespace PID", `t3/mod.rs:42`) is rewritten to CID identity in the landing commit. **New `unsafe`:** four SAFETY-commented sites inside the existing sandbox unsafe zone (`linux.rs`: `pidfd_open`, `OwnedFd::from_raw_fd`, `poll`, `ioctl(PIDFD_GET_INFO)`), no new crate or dependency; maos-bin stays `#![forbid(unsafe_code)]`. `check-unsafe` scans only its capability path (`xtask/src/main.rs:158-163`), so the §A6 review — not that gate — attests the wrappers. **I9:** the shared terminal handle and the guard field that holds it are new `Arc`/`OnceLock`-bearing kernel holders; each carries `#[i9_exempt]` and an entry in `docs/invariants/i9-exemptions.md` with ≥ 2 maintainer sign-offs (precedent `T3Launch`), and `check-empty-kernel` runs at landing. | **Ratified 2026-10-13** |
| **D-17-7-FW-05** | **Story status `ready-for-dev`**, set after FW-01…FW-04 and FW-07…FW-09 were ruled (17-1 FW-05 precedent; 17-2 RT-6 recorded the gap when that order was skipped). E16-A7 is satisfied by the measured prototype. | **Done 2026-10-13** |
| **D-17-7-FW-06** | **Kernel offset.** Deleting `inspect_container_host_pid` (`t3/spawn.rs:225-258`) and the name-based `Drop` steps is inside the measurement; the offset is never spent as a raise elsewhere. | **Taken 2026-10-13** |
| **D-17-7-FW-07** | **Stale-container policy.** `--restart=no` explicit; no MAOS path starts a CID it did not create in this process; the per-spawn bearer dies with the daemon's in-memory table and proxy; a retained container left by a crashed root is **reported** (`maos purge`'s existing scan) and never swept destructively at boot, because roots may share one data home (`egress.rs:189-191`) and a sweep could kill another live root's Worker. | **Ratified 2026-10-13** |
| **D-17-7-FW-08** | **CI-execution route.** `ci/17-7-<purpose>` throwaway branches are authorized under the 17-2 FW-08 conditions: a temporary branch-scoped push trigger that never lands on main; the two x64 T3 images (`ubuntu-24.04`, `ubuntu-26.04`), plus the four-image T2 jobs only if a T2 path changes; bounded runs of the story's eight variant-A proven-red controls; read-only workflow permissions and no repository secrets; each run's command, diff and run id recorded in the story; each branch deleted after its evidence is archived. This authorizes CI runs only — no merge, landing or ceiling authority. | **Ratified 2026-10-13** |
| **D-17-7-FW-09** | **Upstream advisory disposition.** CVE-2026-57231 (image `Env` key imports client env; fixed podman 5.8.4/6.0.0) and CVE-2026-55686 (WORKDIR traversal; fixed 5.7.1) affect both runner baselines (4.9.3, 5.7.0); 17-7 does not fix them. MAOS runs only operator-signed T3 images (`t3-image.lock` + trust anchor), and the scrubbed `create` client env limits what the first can import — accepted for CI, where the image is ours. The production host's runtime version is owned by the operator lane **`ops-live-worker-under-17-1-profile`** (`deferred-work.md` row, §17-7 creation). 17-7 logs the actual runtime versions in CI as evidence only. | **Ratified 2026-10-13** |
| **D-17-7-FW-10** | **Scoped founder approval-quorum exception.** Lunarpulse explicitly waives the second-maintainer signature for Story 17-7 while the founder is the sole maintainer. One actual founder approval satisfies this story's I9 approval gate at reviewed revision `f3fadea9c1ef0c283c836c7a988c5ff7b2a6dba5`; it is not two independent signatures. This supersedes only FW-04's signature cardinality for this story, not I9's substance, CI/review requirements, other exemptions, or the Windows release hold. | **Ratified by explicit founder instruction in the conversation** |

## Evidence and boundary

Five read-only scouts re-measured every epic cite at `37c34be4` (kernel T3, maos-bin supervision, tests/CI/docs, upstream podman/conmon/crun, governance); one agent built the prototype in an isolated copy; three non-author validators checked the draft. The facts that shaped this record:

- **Podman evidence loses the signal.** conmon writes `128+WTERMSIG` (`ctr_exit.c` v2.1.10/v2.1.13 `:100-107`); `InspectContainerState` has no signal field (podman v4.9.3 `:208-232`, v5.7.0 `:298-322`); `StoppedByUser` is set by any successful podman kill or stop and is not death provenance. An external SIGKILL and `exit(137)` therefore produce identical runtime facts.
- **The kernel keeps the signal for any pidfd holder.** Since Linux 6.15, `PIDFD_GET_INFO` with `PIDFD_INFO_EXIT` returns the raw wait status once the task is reaped, to any holder of a pidfd opened earlier ([vfs-6.15 pidfs](https://lkml.iu.edu/hypermail/linux/kernel/2503.2/09488.html)). The prototype read `Exited(137)` and `Signaled(9)` from non-child processes reaped by another parent on kernel 7.2.9 (`17-7-evidence/smoke.log`). Opening the pidfd right after `podman init`, while the payload is gated in the runtime's exec FIFO, removes the open-after-exit race.
- **Startup identity is runtime-written.** `--pidfile` is a `create` option, written during OCI create (`podman init`) before the workload runs, and is not removed at exit; `podman create` prints the CID before start. Runtimes truncate or rename over existing files, so freshness needs a private per-spawn directory.
- **OOMKilled is bounded evidence.** conmon sets it on `memory.events` `oom` or `oom_kill` (shipped `cgroup.c` 2.1.10 `:268-314`); the story's OOM authority contract labels a death `fault.oom_killed` only when the marker coincides with a SIGKILL, or with runtime code 137 and an unknown payload status.
- **Measured:** variant B +291 physical / +326 tokei; variant A +429 / +454; `maos-domain` +47 tokei; 0 / 4 new `unsafe` sites (`17-7-evidence/measurements.json`).

**Evidence correction (2026-10-13).** The first archived `prototype-variant-b.diff` had been generated from the variant-A tree. The non-author validation caught it; the three diffs were regenerated from the recorded commits (base `20530588`, B `cab9a7e8`), all three revisions were re-measured from separate worktrees, and the figures were identical. Hashes are in `measurements.json`.
## Preflight clarifications — 2026-10-13

At the operator's request for long-term correctness, party-mode preflight filed story **PF-01…PF-08** and epic **R17-133…R17-140**. These resolve implementation-contract gaps within the existing charter; they are not a new operator ratification and do not change FW-01…FW-09, the measured prototype, the four-site unsafe allowance, ceilings or CI permissions.

- **Creation custody:** add `--cidfile` in the same fresh private state directory as the sole cleanup authority; require receipt/stdout agreement for normal startup and preserve receipt authority across create-client failure. Missing/invalid receipt, even with a valid-looking stdout CID, is explicit creation-ownership-unknown residue: no destructive recovery by name or stdout. This supersedes only R17-119's stdout-only recovery detail.
- **Evidence versus intent:** intentional stops latch `PlannedStop`; mechanical cleanup after unexpected client failure does not. One immutable snapshot serves both classifiers; later termination confirmation can release the lease but never rewrite the fault/completion.
- **Wire and decoding:** the story now declares all six nested exit-row keys, failure nullability and removal invariants. Invalid runtime codes without valid pidfd evidence are `Truncated`, not process exit codes. Native/historical four-key rows are unchanged; valid pidfd status remains authoritative.
- **Budget and execution:** one concurrent root teardown deadline covers all CIDs, including signal-listener forced exit and finalizer/Drop races. Invocation deadline expiry does not synchronously wait on blocked drains/reaping. PID bounds, returned EXIT-mask checks, complete State-field types and non-duplicated ENTRYPOINT argv are explicit.

Development still owes the real runtime/CI oracles in story §Preflight and the full §A6 net. Every new line is measured inside FW-02/FW-03; no extra allowance is implied. Existing unrelated exit-command/model-provenance gate debt is documented in the story and is not waived or repaired by changing author attribution.


## Grant protocol (landing order)

The landing commit re-pins everything in one change, in this order (E16-A1, `kernel-core-baseline.toml:849-857`):

1. Class rows in `xtask/kernel-api-classes.toml` for every new or changed public seam.
2. Regenerate `docs/ci-baselines/kernel-surface-v0.1-beta.json`, enumerating every removal and addition.
3. `src_lines` and HISTORY in `xtask/kernel-core-baseline.toml` (line 505 stays line 505).
4. File digests and `set_hash` in the same file.
5. The literals and, if the file set changes, `PINNED_ENTRIES` in `xtask/tests/kernel_pin_content_hash_16_0.rs`.
6. The `xtask/kloc.toml` rows (kernel, domain, bin, audit, eval, egress if touched, xtask, `_aggregate_hardfail`).
7. `RATIFIED_AT_EASING` in `xtask/tests/recovery_lane_ceiling_rule.rs`.
8. Additive entries in `xtask/abi-ratifications.toml` (the sandbox-report wire change and the terminal-facts types).
9. Record the I9 exemptions (`docs/invariants/i9-exemptions.md`), then run `check-service-boundary`, `check-kernel-baseline` and `check-empty-kernel` at HEAD, then `kloc-check --json`.

Every figure must be within FW-02/FW-03, or it returns as a request first.

## Disposition

- **Ruling (2026-10-13, Lunarpulse):**
  - FW-01: **variant A** (payload pidfd + `PIDFD_INFO_EXIT`), with the seven kernel locations above.
  - FW-02/FW-03: ratified as proposed — kernel ≤ 25574 physical / ≤ 19946 tokei, exact lower figure at landing; out-of-kernel exact at landing as measured requests.
  - FW-04, FW-07, FW-08, FW-09: ratified as proposed.
  - FW-05/FW-06 follow from these rulings.
- **Recorded changes:**
  - The story is `ready-for-dev`; its AC1 rows are the variant-A rows, with the runtime-evidence rows kept as the per-spawn fallback.
  - `sprint-status.yaml` moves the 17-7 row to `ready-for-dev`.
  - Epic §R12 rows R17-117, R17-125, R17-129, R17-130 and R17-132 record the rulings; the header and table row say *ratified*.
  - `deferred-work.md`'s advisory row names its owner by ruling.

## Founder quorum exception — Story 17-7 closure

Lunarpulse's explicit instruction: “override that two signature rule for now.
only one maintainer here. me founder. I approve both slots. lets move on.”

D-17-7-FW-10 records **one genuine founder approval plus a scoped waiver of
the second signature**, never a fabricated second reviewer. The exception is
limited to this story's reviewed implementation and its eight I9 exemption
rationales. It is not a standing waiver for future stories or other entries.
The default two-maintainer rule remains intact outside this exception.

No runtime, invariant text, whitelist, invariant-lock gate or general governance
document changes under this decision. The substantive I9 invariant is unchanged.
The non-author review, exact measured authorization, dual-image runtime
acceptance and eight proven-red controls remain required and are satisfied by
the evidence in `17-7-evidence/review-ci.json`. Windows release remains deferred
until an explicit operator command; closing Story 17-7 does not authorize a
commit, merge or release.

