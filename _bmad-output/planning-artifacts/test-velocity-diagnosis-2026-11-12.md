# Test velocity diagnosis (2026-11-12)
Persisted 2026-11-15 from an orchestrator session input of 2026-11-12
Status: input to the CI release-ready program (see ci-release-ready-program-2026-11-14.md); not a ruling

Operator-forwarded input from a separate read-only session (2026-11-12): measured causes of slow development from tests/CI. Nothing was changed. Input to the epic-20 retrospective, not an implementation order.

PRECONDITION: do nothing below until refs/tags/v1.0.0-rc.1 is pushed (ruling 13: main must not move before the tag). Do not clean the shared target dir while any cargo process runs.

Measured (gh API: 200 discipline runs, per-job data for 93 runs since 10-12; local nextest ci junit 2026-11-11; cargo metadata):
1. CI is mostly compile. Run 43175946640: 168/199 jobs "No cache found", 0 restored; 508 of 754 job-min are cargo compile (67%). Actions cache 10.1/10 GB, 9 entries: ~169 rust-cache steps with per-job keys evict each other. 126 `cargo run -p xtask` steps each compile ~400 crates (xtask depends on maos-bin et al.).
2. Queue: ~199 jobs/push; median pre-start wait 9-70 min; wall 55-207 min vs longest job 30-35 min. 129 of 187 job names never failed in 93 runs (406 job-min/push).
3. main red 397 of 663 h since 10-12 (56/71 pushes; one 236 h streak). workspace-test-suite red 46/83: 34 product integration tests (T3/daemon lifecycle on wall-clock budgets: worker_terminal_facts_17_7 x9, root_shutdown_unload_16_3 x6, t3_terminal_helpers_17_7 x3), 8 tree-bound xtask only (d11_xtask_ceiling_ratchet x6). check-test-budget red 20/72 (hosted timing); check-multi-tenant-loom 34/83 (INDETERMINATE leg); reproducible-build 12 reds were cargo-deny advisories.
4. Local: tests run 96 s wall (6,018 tests). Cost is build/link: 659 integration-test binaries; a maos-kernel-core edit relinks 520. target/ 173 GB; debug/deps 3.67M entries (3.62M stale .dwo); trivial rustc with that -L dir 1.7 s warm / 6.8 s cold vs 0.24 s. Prepush 908-2,943 s in story records; three 3,000 s timeouts.
5. 97 of 756 test files read workflow YAML / xtask registries / sprint-status / docs / source text; 12 --expected-count pins in discipline.yml.
6. Slow/flaky tests run real budgets with no seam: t3/child.rs:24-30, maos-control/src/lib.rs:249, maos-registry/src/yank.rs:252-262; paused time in only 6 files.

Asks:
A. After the tag, persist this as _bmad-output/planning-artifacts/test-velocity-diagnosis-2026-11-12.md.
B. In the epic-20 retrospective, raise D-rows for the standing rulings the plan touches: (1) 20-15 job names / aggregate.needs (gate-job consolidation); (2) 20-16 unconditional full-workspace prepush (change-scoped prepush); (3) R3 blocking per-binary budgets (per-binary advisory, total blocking); (4) two tiers: substrate/real-time tests leave the merge-blocking workspace suite for their substrate jobs. Acceleration ruling item 8 (no retry-to-green) stays.
C. Ruling-free stories first: build xtask once per run and ship it to gate jobs as an artifact; rust-cache shared-key per build family + save-if main; cargo-deny advisories off the push path (bans/licenses/sources stay blocking). Keep the `aggregate` check name (release precondition).
D. After rulings: one integration-test binary per crate (maos-bin, kernel-core first); one injectable Budgets/Clock seam replacing *_FAST env flags and budget constants; in-memory TL / batched inserts for logic tests; a shared test-harness dev crate; repo-state assertions moved out of cargo tests into gates tested on fixture trees.
E. Ops, no story: when cargo is idle, prune target/ (cargo clean or cargo sweep).
Record per landing: main green %, push->verdict p50, job-min/push, cache hit %, prepush p50, test-binary count.
