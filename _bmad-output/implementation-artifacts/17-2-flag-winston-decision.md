# 17-2 FLAG-Winston decision record

**Date:** 2026-10-11  
**Status:** `ratified` 2026-10-11 by the operator (Lunarpulse). This covers FW-01 (including `journal/mod.rs`), FW-02/FW-03 (*no raise; exact at landing*), FW-07 (new story `17-7`) and FW-08 (the `ci/17-2-*` CI-execution route, ruled after the readiness re-validation). The 17-2 preflight round-table proposed FW-01…07 the same day.  
**Scope:** story creation, the throwaway measurement prototype (2026-10-10) and the preflight round-table only. This record changes no runtime, kernel, baseline, KLOC ledger, ratification file or ADR. The kernel and KLOC figures move only in 17-2's landing commit.  
**Model after:** `17-1-flag-winston-decision.md`. A FLAG-Winston is an architectural *location charter*, not a numerical grant (`17-3c-early-start-flag-winston-decision.md`, D-17-3c-ES-04).

## Decision

| ID | Decision | Status |
|---|---|---|
| **D-17-2-FW-01** | The kernel **locations** for 17-2 are:<br>• `security/sandbox/linux.rs`: PID-controller discovery/enable, `pids.max`, CPU-or-PID refusal without delegation, and the parent-computed resource outcome.<br>• `security/sandbox/mod.rs`: a transient, Linux-only outcome on the native guard.<br>• `security/sandbox/t3/argv.rs`: CPU percentage → cores, `--pids-limit` from `pids_max`, and `--ulimit=nofile` from `fd_max`.<br>• `lifecycle/cli_wrapper/runtime.rs`: a one-shot outcome accessor.<br>• `scheduler/scheduler_loop.rs`: one `pids_max: None` literal (`:324-328`).<br>• **Deletion-only:** `scheduler/resource_ceiling.rs` (whole file) and its `scheduler/mod.rs` declaration.<br>• **Deletion-only:** `journal/mod.rs`, where `drop(file);` (`:188`) is deleted. The live index insert then happens under the writer guard; see the preflight finding below.<br>`security/mod.rs` gets **no** resolver API, because the existing `policy()` access is enough. Any other kernel location returns to the operator as a charter request. | **Ratified 2026-10-11.** A charter, not a number. |
| **D-17-2-FW-02** | **Kernel numerical ceiling: no raise.** `src_lines` ≤ **25267** (`kernel-core-baseline.toml:505`) and kloc ≤ **19553** (`kloc.toml`; `RATIFIED_AT_EASING`, `recovery_lane_ceiling_rule.rs:267-279`). The landing re-measures after `cargo fmt --all` and re-pins the **exact lower** figures. This tightens the ceiling, following 17-1's 19529 → 19513 precedent at `kloc.toml:255`. The retired file leaves the pinned file set. The mechanism prototype measured **−157 physical / −99 tokei** (`17-2-evidence/measurements.json`). A landing figure above the current pins is a new request, never a re-pin. | **Ratified 2026-10-11** |
| **D-17-2-FW-03** | **Out-of-kernel ceilings.**<br>• `maos-manifest`: no raise. Measured 4456 plus the prototype's +11 is 4467, within the 4474 ceiling.<br>• `maos-domain` (unit event 20 and typed payload), `maos-bin` (policy/journal plumbing), `xtask` and `_aggregate_hardfail`: exact measured figures **at landing**.<br>• A figure above a current ceiling (domain 9329, bin 25247, xtask 44370, aggregate 173383) returns to the operator as a measured request **before** landing.<br>No pre-authorized envelope is given, because the ledger forbids speculative reservation (`xtask/kloc.toml:60-69,85-89`).<br>**Expected, not hypothetical:** at creation `maos-domain` measured 9329 and `maos-bin` 25247, each **equal** to its ceiling (`17-2-evidence/measurements.json`; `kloc.toml:411-412`). Their growth therefore returns as measured requests. The 2026-10-08 S2 revision's domain bound of ≤ 9333 (`kloc.toml:411`; epic `:504`) covers at most +4. | **Ratified 2026-10-11** |
| **D-17-2-FW-04** | **No ADR amendment.**<br>• ADR-067 already makes cgroup residue report-only (`docs/adr/ADR-067-maos-data-root-layout-and-purge.md:87-90`), and the story keeps that.<br>• The prototype adds no `unsafe` (`17-2-evidence/prototype.diff`).<br>New `unsafe` or a contradicted ADR returns as a request. | **Ratified 2026-10-11** (with FW-01) |
| **D-17-2-FW-05** | **Story status: `ready-for-dev` retained.** The story was marked ready on 2026-10-10 before this record existed. The 17-1 FW-05 precedent (`17-1-flag-winston-decision.md:16`) requires the charter to be ruled first, and this ruling closes that gap. | **Done 2026-10-11** |
| **D-17-2-FW-06** | **Kernel offset.** Deleting `scheduler/resource_ceiling.rs` (170 lines: a root-layout API with no production caller that never places the process) and its export lowers the landing figure. The offset absorbs lines the prototype omitted and is **never** spent as a raise elsewhere. | **Taken 2026-10-11** |
| **D-17-2-FW-07** | **Handoffs** `deferred-work.md:1119` (T3 OOM/signal facts) and `:1120` (a short-lived T3 Worker is refused) go to the new story **`17-7-t3-worker-retained-lifecycle-and-terminal-facts`**:<br>• `backlog`, FLAG-Winston, ordered after 17-2.<br>• E16-A7: it cannot leave `backlog` without its own measured prototype.<br>• Its charter covers CID-owned cleanup.<br>17-2's scope is unchanged. | **Ratified 2026-10-11** |
| **D-17-2-FW-08** | **CI-execution route.** This is an execution prerequisite, not a charter or a number. It closes readiness BLOCKER V-1. The operator authorizes `ci/17-2-<purpose>` throwaway branches (the 17-1/17-6 precedent) under these conditions:<br>• A temporary **branch-scoped** push trigger, which never lands on main.<br>• Runs on the four T2 images (`ubuntu-24.04`, `ubuntu-26.04`, `ubuntu-24.04-arm`, `ubuntu-26.04-arm`) and the two x64 T3 images.<br>• Bounded proven-red mutation runs.<br>• Read-only workflow permissions and no repository secrets.<br>• Every run records its command, diff and run ID in the story.<br>• Each branch is deleted after its evidence is archived.<br>This authorizes CI runs only; it confers no merge, landing or ceiling authority. | **Ratified 2026-10-11** |

## Evidence and boundary

Three read-only scouts re-read the story's cites at `b19556ab`. Their findings are recorded in the story's §Preflight round-table (RT-1…RT-10) and in the epic's §R11 (R17-109…R17-116). Four facts decided this record:

- **The journal live index is not linearized with the file.** `append_transition` writes under the writer mutex, releases it at `journal/mod.rs:188`, and only then inserts into `most_recent` (`:189-195`). 17-2 introduces the first concurrent same-Spirit producer pair: the session thread appends the downgrade while the pidfd observer appends `Crash` (`supervision.rs:2487-2535`). Two interleavings can therefore persist A then B while the live index ends at A. Holding the guard through the insert fixes this at origin. The lock order is writer → index, and readers take only the index, so there is no inversion.
- **No boot-time state replay exists.** The daemon builds an empty SCB map (`scheduler_loop.rs:132-167`; `main.rs:2725-2778`). `recover_in_flight` returns every indexed identity, including crashed ones (`journal/mod.rs:206-212`). The only production recovery call is the smoke arm's task-row count (`main.rs:6678-6702`). An informational row such as `ProviderSwitched = 18` already overwrites the raw index without a state effect.
- **The two handoffs are a different size class.** Removing `--rm` alone fixes neither:
  - Conmon maps signal death to 128+signal (`containers/conmon` `src/ctr_exit.c:146-153`).
  - Its OOM marker fires on `oom` **or** `oom_kill` (`src/cgroup.c:287-355`).
  - An exited container reports `State.Pid = 0`.
  - A correct cutover needs an owned CID, a runtime-written `--pidfile`, a terminal snapshot shared with both classifiers before `task.orphaned`, and explicit removal. A scout estimates it at **+120…+360 kernel physical lines** `[INFERENCE, unmeasured]`, against 17-2's measured −157.
- **Governance gap.** The 17-1 precedent keeps a story in `backlog` until its charter is ruled. No 17-2 record existed at creation, and this record supplies it.

## Grant protocol (landing order)

The landing commit re-pins everything in one change, in this order:

1. Class rows in `xtask/kernel-api-classes.toml`.
2. Regenerate the surface pin `docs/ci-baselines/kernel-surface-v0.1-beta.json`, enumerating every removal and addition.
3. `src_lines` and HISTORY in `kernel-core-baseline.toml` (line 505 stays line 505).
4. File digests and `set_hash`, with the retired file removed.
5. The literals in `kernel_pin_content_hash_16_0.rs`.
6. The `kloc.toml` rows (kernel, domain, bin, xtask, `_aggregate_hardfail`).
7. `RATIFIED_AT_EASING`.
8. The additive ratification entry in `abi-ratifications.toml`.
9. Run `check-service-boundary` **and** `check-kernel-baseline` at HEAD.

Every figure must be within FW-02/FW-03, or it returns as a request first.

## Disposition

- **Ruling (2026-10-11, Lunarpulse):**
  - FW-01 ratified, **including** `journal/mod.rs`.
  - FW-02/FW-03: *no raise; exact at landing*.
  - FW-07: new story 17-7.
  - FW-08: `ci/17-2-<purpose>` throwaway-branch CI runs, which closes readiness BLOCKER V-1.
  - FW-04/05/06 follow from these rulings.
- **Recorded changes:**
  - The story keeps `ready-for-dev`; its §Preflight round-table holds the rulings.
  - `deferred-work.md:1119-1120` now name 17-7.
  - `sprint-status.yaml` gains the 17-7 `backlog` row.
  - The epic gains §17-7 and §R11, and `epics/index.md` counts 10 stories. `check-epic-close-coherence` passed after these edits.

## Measured FW-03 grant — 2026-10-12

The operator selected **Authorize exact measured increases** after `cargo fmt --all -- --check` passed and the actual formatted `cargo run --locked -p xtask -- kloc-check --json` output was presented (`17-2-evidence/t5-validation.json`; `artifact://353`).

| Ledger row | Previous ceiling | Authorized exact ceiling |
|---|---:|---:|
| `maos-domain` | 9329 | 9349 |
| `maos-bin` | 25247 | 25397 |
| `_aggregate_hardfail` | 173383 | 173505 |

Drivers: the typed lifecycle downgrade payload/event in domain; live resource-policy, shared journal and native outcome ownership wiring in bin. Aggregate measures173504; the established strict `aggregate >= hardfail` rule requires measured+1,173505. No speculative headroom is granted. Any later larger measurement requires another measured request.

FW-02 remains unchanged: the kernel receives no raise and must re-pin its exact lower physical/tokei figures. The current formatted kernel tokei measurement is19495; manifest4466 remains within4474; xtask44370 remains within its current ceiling. No unused kernel allowance is transferred. The existing Grant-protocol ordering, seven-artifact pin coordination and final acceptance/review gates remain mandatory. This decision authorizes the exact numerical grant, not a merge or landing.

## Exact lower post-A6 measurements — 2026-10-12

The evidence-backed containment correction retains best-effort owned native
placement independently of configured cgroup ceilings. It stays within the
existing eight source locations and introduces no new unsafe code, public type,
dependency, or ADR contradiction.

The actual formatted checks now measure kernel **25146 physical / 19492 tokei**
across **95 files**, manifest4466, domain9349, bin25397, xtask44370, and
aggregate173501. The strict aggregate threshold is re-pinned to **173502**
(measured+1), below the previously authorized173505. Kernel physical/content and
independent KLOC/RATIFIED literals are re-pinned to their exact lower figures;
the source-line assignment remains on line505.

These are lower exact measurements, not an additional grant or speculative
headroom. Original authorization and final runtime/review requirements remain
unchanged. Actual baseline/service/KLOC/unsafe and corrected contract/quality
outputs are archived in `17-2-evidence/t6-final-quality-gates.log` and
`17-2-evidence/t6-corrected-contract-gates.log`.
