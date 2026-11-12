The Epic 20 retro is amended: a new "Amendment A" at the end of the file (A1–A6), plus small in-place edits. I touched only the retro and added `epic-20-retro-evidence/fr-live-table-2026-11-12.csv`.

I didn't run `xtask coverage-matrix`. The matrix is keyed by gate, corpus and phase and carries no per-FR live field, so the FR count is a documentary re-measure from witness notes and closure records. It does not meet the "hosted run id per FR on the final tree" bar.

**Surprise:** `git ls-remote` shows `origin/main` = `582f9509` at amendment time, so it was pushed after the retro was first written. The retro's earlier "unpushed" statements are marked superseded. Discipline run 43236109451 on it was queued (result unread). `582f9509` has no `ac4_v3b` fix, since no commit has touched that test file since `c8e5d035`. That second point is `[INFERENCE]`, from `git log` only.

**A1: FRs live from a product binary**
- **Count:** 43 of 66 FRs are LIVE by the strict definition. 53 of 66 count if a binary-driven FR with one named unproven clause is included. 13 are not live.
- **Prior figures:** 22 of 66 was measured on 2026-09-04. "≥50 of 66 by the end of Epic 19" is only a target in `sprint-change-proposal-2026-09-04.md:104`; no intermediate measured figure exists in the repo. The target is met only on the inclusive count.
- **Definition:** LIVE means a routing-audit LIVE-VERIFIED verdict, or a done story whose test drives the shipped `maos`/`maosctl`/door, and no closure or matrix note names an unproven clause. The CSV flags FR4, FR5, FR34, FR40, FR41 and FR63 as `[INFERENCE]`, because I took them from story-done status without re-reading the witness.
- **Boundary but counted inclusive (10):** FR4, 5, 6, 10, 17, 36, 47, 59, 64, 65.
- **Non-live (13):**
  - FR13, FR37, FR48, FR62, FR63 are asserted.
  - FR1, FR3, FR23b, FR28, FR29, FR33 are partial.
  - FR7 and FR54 are absent.
  - Only FR13, FR23b, FR28 and FR29 have an owner inside the GA set.

**A2: ACs that left Epic 20 unproven**
- The table has 31 rows (story, AC, clause, what was proven instead, where the rest went, closure-note line cites). It covers 20-7, 20-13, 20-9, 20-1b, 20-4, 20-5, 20-2, 20-8, 20-11, 20-14, 20-3b, 20-3c, 20-1a and 20-10.
- 20-3b (AC2 generator not built, spike NO-GO), 20-3c (hosted `kernel-l2`/`t3-legs` never pushed) and 20-1a/20-10 (main discipline not fully green) came from tracker lines, not closure notes.

**A3: Epic 19 reconciliation (all six are `open` today)**

| Item | Status | Reason |
|---|---|---|
| A1 | done | preflight ratified (`epics/index.md:165`) |
| A2 | in-progress | allowance included, but no per-envelope forecast-vs-landed table exists |
| A3 | in-progress | hosted plants ran for 20-13, 20-1b, 20-8 and 20-9, not for 20-3b or 20-3c, and no written rule exists |
| A4 | in-progress | `cargo-maos` target in use; prepush contention persists |
| A5 | in-progress | init half done, fuzz-ledger half and operator confirmation outstanding |
| A6 | open | `DW:1475` trigger fired and was never discharged |

- I corrected the earlier retro text, which called A2 and A3 complete.
- **A5 "0 zombies, init: true":** this is not recorded in any repo file. The grand plan lines 33 and 108 say the state is unknown. The compose file has `init: true` (commit `38e5b52a`, 2026-10-27). In my shell at amendment time PID 1 was `docker-init` and the zombie count was 0. The amendment is the first record of that point-in-time measurement.
- fuzz-ledger: the branch exists with 45 T2 bot appends and 0 T3; the T3 push is unexercised.

**A4: new D-rows**
- **D-E20-10 (GA vs perf):** `release.yml:165-171` (the `ga` case at `:171`) omits `--perf-absent-ok`, and `ops-stable-perf-runner` is dropped, so no `v1.0.0` tag can pass as written. The conflict is already filed at `deferred-work.md:1726`. Options: A amend `release.yml` so `ga` accepts ABSENT as `rc` does, B restore the runner, C accept a hosted-runner PASS as an exception. Recommendation A.
- **D-E20-11 (GA hidden prerequisite):** `sprint-status.yaml:419-420` put 21-7 and 21-8 "After 21-2", but 21-2 is not in the ga3 set. Options: A add 21-2 to the GA set, B drop the ordering and decide per story, C add only the Postgres slice. Recommendation B. Both rows reference `epic-21-preflight-2026-11-12.md`, which now exists untracked.
- **D-E20-3 and D-E20-4:** your decisions are recorded as resolved. D-E20-3: fix the `ac4_v3b` red before the candidate; reruns only for other known flakes, disclosed. D-E20-4: flip `epic-20` with the post-tag record commit.

**A5: readiness**
- Updated with the facts you listed: candidate not green, 20-5 and 20-9 recorded done over red runs, and the T3 chain never completed (one run ever, the plant 38926626626).
- §15, §16, E20-A3 and E20-A4 were edited to match.

**A6: proposed `sprint-status.yaml` edits** (nothing was edited)
1. **Epic 19 actions:** A1 → `done`; A2 → `in-progress`; A3 → `in-progress`; A4 → `in-progress`; A5 → `in-progress`; A6 stays `open`.
2. **Epic 20 actions:** append E20-A1 to A14 as in §13, with A3 and A4 as amended. Add three new rows:
   - E20-A15: resolve D-E20-10 before any GA tag.
   - E20-A16: resolve D-E20-11 in the Epic 21 preflight.
   - E20-A17: give FR63, FR62(b), the FR3 residual and FR37/FR48 either an owner or an explicit GA claim-boundary line.
3. **`epic-20-retrospective`:** `backlog` → `done`, with the comment text given in A6.
4. **`epic-20`:** stays `in-progress` until the post-tag record commit.