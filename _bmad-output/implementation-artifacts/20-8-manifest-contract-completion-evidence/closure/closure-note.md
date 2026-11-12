# 20-8 closure note (2026-11-12, record-only)

No code, cargo or git mutation in this slice. Hosted facts were read from `ci/run-43188003155.md` and the committed `landing-3/` evidence; commit shas were read with `git log`.

## AC table

| AC | Status | Evidence |
|---|---|---|
| AC1.1-1.4, 1.6, 1.7 sections, gate, journal-before-admit, publisher, local witnesses, plant | LANDED (5610f9d6, 37baeedf) | `fr8_declarations_20_8.rs`, `manifest_declarations_20_8.rs`, `wasm_skill_authoring_20_8/signed.rs`, `landing-1/preflight.md` |
| AC1.5 hosted command | LANDED (hosted, run 43188003155 `epic-20-exit (declared)` 131079942019) | `ci/run-43188003155.md` |
| AC2 schema 7 + degradation | LANDED (5610f9d6) | `manifest_n_minus_1_test.rs`, `docs-site/docs/manifest/v7.md` |
| AC3 skills `uses` admission | LANDED (5610f9d6, 37baeedf) | `skill_declarations_20_8.rs` |
| AC4 I13 at TaskAssign | LANDED (5610f9d6, 37baeedf); opt-in default is an accepted risk row | `intent_promotion_20_8.rs` |
| AC5 explanation_shape label | LANDED; enforcement not claimed (row stays open by design) | docs |
| AC6.1-6.4, 6.6, 6.7 | LANDED (5610f9d6; hosted: run 43188003155 componentize-js 14/14, wasm-host-tests 24.04, run 43104682092 four images) | |
| AC6.5 compatibility | LANDED for genuine 2.0 and frames-only 2.2/2.3 guests; **CLAIM-BOUNDARY**: no recall-calling genuine 2.1 guest was built | `recall_runner_17_3d` |
| AC7.1 hosted T2 legs | LANDED (hosted, run 43188003155) | job table in the story Closure |
| AC7.2 plant | LANDED (hosted, `epic-20-exit (removed-plant)` 131079942065; red = queue assertion fails) | `ci/run-43188003155/removed-plant/queue-assertion.log` |
| AC7.3 external author | **CLAIM-BOUNDARY**: not claimed; operator lane `ops-external-cohort-and-pen-test` | |
| AC8 kernel refusal, door 409, cold swap | LANDED (692b1e59, 1fd26dcc) | `swap_invariants_20_8.rs`, `landing-2/` |
| AC8 precheck | LANDED (44cf4f99, kernel-free) | `landing-3/precheck-door.md`, reviews in `landing-3/review/` |
| AC9 records | LANDED at this closure | coverage-matrix notes, deferred-work.md, story |

## Honest caveats
- Run 43188003155 is a branch-only `push` run on main 163ab9bd + the workflow file (fa8a7862); it did not test 44cf4f99. `workflow_dispatch` was never exercised. Its `wasm-host-tests` ran on ubuntu-24.04 only.
- The removed-plant job is green by design (its green means the assertion failed as required).
- Precheck door fix: 2 GPT P2 findings fixed in a correction round; round-2 re-review 0 findings; security review 0 findings with 3 residual observations (two filed as P3 rows; the installed-predecessor gap fixed by the live-SCB read). Security review of 1fd26dcc and the door ran on a GPT reviewer because GLM weekly quota was exhausted. Bin 38602 -> 38628 (+26, adjudicated).
- Tamper message is opaque (`failed to fill whole buffer`, `install.rs:220-231`); a same-length edit was not exercised. A runner panic (exit 101, cause unconfirmed) followed a denied `author()` in the plant leg. Timing margin is thin (plant ~303 s vs 300 s bound; final recipe 450 s). All filed as rule-10 rows.
- Stale text: tree is WIT 2.4.0; AC8 pins 27014/21181 are historical (authoritative kernel 26,062/20,429 after 21-5).
- Workspace-wide test runs had known unrelated failures during L1/L2 (recorded in the story Completion Notes); no final-tree full workspace pass is claimed.
