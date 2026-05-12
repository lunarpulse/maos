# Deferred Work

## Deferred from: code review of 0-2-enforce-empty-kernel-invariants-via-structural-ci-lints (2026-05-11)

- **DF1 — DRY violation: walk_mod and walk_inline_mod_item.** The same 8 `match` arms for `pub fn`/`pub struct`/`pub enum`/`pub trait`/`pub type`/`pub const`/`pub static`/`pub use`/`pub mod` are copy-pasted between `walk_mod` and `walk_inline_mod_item` in `xtask/src/check_service_boundary.rs:200-260`. A bug fix in one won't automatically propagate to the other. Defer to cleanup story.
- **DF2 — Hardcoded baseline path in check-service-boundary CI job.** The CI gate in `.github/workflows/discipline.yml` always compares against the committed `kernel-surface-v0.1-alpha.json`, not a branch-specific baseline. Acceptable for v0.1-α stub; Story 2.2 may want a PR-target-branch diff mode.
- **DF3 — service_boundary_integration test builds baseline on-the-fly.** The violation test at `xtask/tests/service_boundary_integration.rs:22-40` snapshots the clean fixture to establish a baseline, then diffs against the violation fixture. Works correctly but is fragile — changes to the clean fixture silently change the test's expected behavior.

## Deferred from: code review of 0-3-content-addressed-corpora-infrastructure-coverage-matrix-ci-gate (2026-05-12)

- **W1 — `serde_yaml` 0.9.34 is explicitly deprecated.** Spec acknowledges this and defers migration. The crate works for the current contract; future concern.
- **W2 — `calibrate.rs` `successes = n` hardcoded placeholder.** At v0.1-alpha with no real corpora this is intentional scaffolding. Story 0.4 must replace with actual pass/fail from `expected_judgment` comparison.
