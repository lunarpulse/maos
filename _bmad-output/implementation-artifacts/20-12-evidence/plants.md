# 20-12 proven-red plants (AC7 protocol: in place, one per AC, recorded once)

Recorded by the integrator from the dev pass's Debug Log (dev of record `anthropic/claude-opus-5-5`, 2026-10-23); the dev pass itself did not write this file — review finding (LOW, record) closed here.

| AC | Plant | Where | Red observed | Restore | Wall-clock |
|---|---|---|---|---|---|
| AC1 (`check-test-budget`) | a `#[test]` with `std::thread::sleep(31 s)` appended to an unbudgeted binary | `crates/maos-cli/tests/dep_kernel_core_free_test.rs` (end of file) | gate: `… took 31.0 s … budget 30 s` → exit 1 | `git checkout -- crates/maos-cli/tests/dep_kernel_core_free_test.rs` | 33.3 s round trip (31 s of it the sleep) |
| AC2 (`check-test-cost`) | an unmarked `Command::new("cargo")` in a non-ignored test | same file, `:35` | gate: unmarked cargo call at `:35` → exit 1 | same `git checkout --` | 0.32 s |

Caveat recorded by the dev pass: the `git checkout --` restore also discarded a sub-agent's uncommitted edit to that file; it was re-applied (dev asserted identical content). The dev-story protocol fact "plant into a file with no uncommitted edits" was added because of this.

Permanent vectors (not plants): `xtask/tests/check_test_cost_proven_red.rs`, `xtask/tests/check_test_budget_proven_red.rs` — tempdir fixture trees, < 1 s each; review 2026-10-23 added the missing branches (see § Review Findings).
