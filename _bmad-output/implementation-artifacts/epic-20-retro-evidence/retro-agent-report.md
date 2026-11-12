I wrote `_bmad-output/implementation-artifacts/epic-20-retro-2026-11-12.md` (about 49 KB, 18 sections). It follows the Epic 19 retro structure and covers every item in the BMad retrospective content list. Every claim is cited or labelled `[INFERENCE]`. No gates were run beyond the read-only gate calls. I did not edit any other file, and nothing was run through cargo or `git`.

## Findings that need your attention
- **The rc candidate is red on hosted CI.**
  - `23b2200b` (discipline run 43215484029) failed `check-fmt` and also `nfr-rel-11-halt-receipt-999pct` (job 131173533987).
  - `582f9509` fixes only `check-fmt`.
  - I did not read the `nfr-rel-11` failure cause.
  - Pre-tag condition (b) in ruling 13 needs a green `aggregate` at the pushed SHA, so this blocks the tag.
- **Minimization correction 2 is not done.** The `fuzz-prerelease.yml` change and `scripts/minimize-fuzz-corpus.sh` are still uncommitted. The re-review found 2 new P2s, and `PT/correction-2/` has no README or review yet.
- **A trigger fired and was never discharged.** 20-9 L2 touched `hot_swap/archive.rs`, but the stale doc at `archive.rs:3-7` is unchanged. It is the Epic 19 trigger-owned row `DW:1475`.
- **`ops-stable-perf-runner` is a dropped key that still owns about 95 open `deferred-work.md` lines.** The count is a heuristic regex, labelled as such in the file. The two recurring flakes sit under it.
- **Incident evidence differs by incident.**
  - Repo-cited: the 59b51a5e unrequested push and the 20-5 file loss (grand plan lines 265-266; 20-5 story line 586).
  - Commit-cited: 76ae19c9, 582f9509 and the 20-4 done-flip revert 345ae30f.
  - Orchestrator account only, so labelled: the red-gates commit chain, the 429, the queue saturation.
- **GLM-to-GPT substitution** is repo-cited (C5, C9, C14 closure notes).
- **Success-criteria re-measure.** The 1.14:1 definition from 2026-09-04 is not recorded anywhere I could find. I took a first point measurement with `tokei` at HEAD:

  | Definition | Ratio |
  |---|---|
  | Narrow (`xtask/src` over `crates/*/src`) | 0.31 : 1 |
  | Broader (adds `xtask/tests`) | 0.48 : 1 |

  Both are indicative only. §12 gives the full method for the FRs-live and ratio re-measure.
- **Kernel ledger.** Epic 20 opened at 19,529 tokei / 25,293 physical (`86d5f8f0`). It peaked at 21,256 / 27,103 (20-9 L2), and ended at 20,429 / 26,062 after 21-5. That is +900 tokei net from the opening, but Epics 17–19 landed in the same window, so it is not Epic-20-only growth. The shared STOP went 21,250 → 22,000 → withdrawn by `kfree`. No Epic 21 forecast crosses 22,000 or 23,000, so no kernel D-row is raised.

## Action-item table (`epic: 20`, status open)
| # | Action | Owner |
|---|---|---|
| E20-A1 | Add `cargo fmt --all -- --check` to the orchestrator commit routine and local pre-push step | orchestrator |
| E20-A2 | Hosted-proof-before-done: a story with a hosted AC needs a named hosted run id on the final tree; the done-flip re-runs the d11 ratchet and `recovery_lane_ceiling_rule` | orchestrator |
| E20-A3 | Clear the rc.1 pre-tag checklist a–e: finish correction 2 and its review, push, get a green `aggregate`, dispatch the T3 chain, run the candidate-SHA rehearsal, tag, post-tag record commit | orchestrator |
| E20-A4 | Fix or re-own the two recurring flakes (`s3_external_sigkill_requires_pidfd_fact`, `ac4_v3b` / `nfr-rel-11` family) | orchestrator |
| E20-A5 | Name an owner for the kernel T3 admission arm's production caller, or label the FR48 / NFR-Sec-15 claim bin-side-only | orchestrator |
| E20-A6 | 21-9: F1–F5, fold the swap leg into main `epic-20-exit`, macOS/Windows `wasm-host` leg, size bound with the engine in, `archive.rs:3-7` doc | `21-9-wasm-host-default-on` |
| E20-A7 | 21-10: I14 row and FR10 witnesses; state per invariant what the product binary proves | `21-10-invariant-ledger-proofs` |
| E20-A8 | Run the success-criteria re-measure using the §12 method (FRs live from a binary, governance:product ratio) | orchestrator |
| E20-A9 | Quarantine-closure gaps, including CRL reaching a Worker via a class-bearing SCB; kernel delta as a forecast | `later-registry-quarantine-closure` |
| E20-A10 | Cap concurrent agents below the 429 threshold; write the review-model fallback rule for when GLM is exhausted | orchestrator |
| E20-A11 | Commit evidence directories promptly; re-check `git status` before any cleanup | orchestrator |
| E20-A12 | Record which `prepaid` condition authorized each push | orchestrator |
| E20-A13 | Refresh the stale fuzz lockfiles for `maos-manifest` and `maos-domain`, or record why they float | orchestrator |
| E20-A14 | Carry forward E19-A4, E19-A5, E19-A6 | orchestrator |

## D-rows (decisions left to you)
- **D-E20-1:** Who owns a production caller for the kernel T3 admission arm? A: `21-10`; B: `21-9`; C: leave it a claim boundary. Recommend **A**.
- **D-E20-2:** What happens to the ~95 rows owned by `ops-stable-perf-runner`? A: keep; B: re-own only the two recurring flakes; C: close as won't-fix. Recommend **B**.
- **D-E20-3:** May a known-flake rerun satisfy ruling 13(b)? A: rerun and disclose; B: accept a flake-only red; C: hold the tag. Recommend **A** for the rc, **C** for GA.
- **D-E20-4:** When does `epic-20` flip to done? A: at this retro; B: with the post-tag record commit; C: at GA. Recommend **B**.
- **D-E20-5:** When is the ABI row `20-5-cli-wrapper-schema-version` ratified? A: before the rc; B: before `v1.0.0`; C: leave it `proposed`. Recommend **B**.
- **D-E20-6:** What is the review-model rule while GLM is out? A: GPT standing and disclosed; B: hold sensitive landings; C: add a third family. Recommend **A**, with B for new trust boundaries.
- **D-E20-7:** Which paths count as governance, and does the "50 of 66 FRs LIVE by Epic 19" threshold still bind? A: keep both and report; B: restate the FR threshold for GA after the first measurement; C: drop the ratio criterion. Recommend **B**.
- **D-E20-8:** Does an FR proven only in-process count as LIVE? A: built-binary tests only; B: in-process counts if a hosted run exists; C: label separately. Recommend **A**.
- **D-E20-9:** What is the Epic 21 re-preflight scope? A: all ten remaining stories; B: GA set first; C: only the three sections owed. Recommend **B**. The roster showed `Preflight21` running; I don't know its scope, so there may be overlap.

## Proposed `sprint-status.yaml` edits (not applied)
1. **Append** 14 entries under `action_items:` after the Epic 19 block (after the "A6: Disposition…" entry), one per table row above. Format: `- epic: 20 / action: "E20-An: <text>" / owner: "<owner>" / status: open`.
2. **Epic 19 actions.** Update status only if you accept my findings:
   - A1 → `done` (preflight ratified).
   - A2 → `done` with the overdraft note.
   - A3 → `done` (practiced, not codified).
   - A4 → stay `open`.
   - A5 → stay `open`.
   - A6 → stay `open`.
3. **`epic-20-retrospective`:** `backlog` → `done`, comment: `Record written 2026-11-12: epic-20-retro-2026-11-12.md (19/19 + 21-5; claim-boundary tally; hosted-found defects; T3 chain audit; kernel ledger 19,529→21,256 peak→20,429; E20-A1…A14; D-E20-1…9 open). Epic 21 re-preflight owed. rc.1 not tagged.`
4. **`epic-20`:** leave `in-progress` pending D-E20-4. If you choose B, flip with the post-tag record commit.