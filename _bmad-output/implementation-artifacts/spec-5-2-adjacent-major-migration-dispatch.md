---
title: 'Fix adjacent-major hot-swap migration dispatch'
type: 'bugfix'
created: '2026-08-12'
status: 'done'
review_loop_iteration: 0
baseline_commit: 'acf0e48a155b0ce4ee0ca6d6a70a86f4e4c00ebe'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/5-2-implement-hot-swap-state-transfer-and-cross-major-migration-against-hsis-95.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `state_codec::decode` rejects every adjacent upward cross-major swap before migration because `pred_major.wrapping_sub(succ_major)` underflows. `HotSwapCoordinator::initiate_swap` consequently returns `SchemaIncompatible` instead of invoking the successor's declared migrator.

**Approach:** Compute major-version distance symmetrically, preserving the one-major migration window and the existing typed rejection for larger gaps. Add a coordinator-level regression that proves an adjacent upward swap reaches `migrate` and delivers migrated bytes to `on_swap_in`.

## Boundaries & Constraints

**Always:** Treat the high 16 bits of `state_schema_version` as the major component; permit a cross-major distance of exactly one in either direction; reject distance two or greater as `StateCodecError::SchemaVersionMismatch`; preserve same-major behavior, wire shape, public errors, and coordinator error mapping. Use overflow-safe integer arithmetic without allocation.

**Ask First:** Any change to the one-major migration policy, version encoding, CBOR envelope, public error variants, or production files outside `state_codec.rs`.

**Never:** Broaden this fix into Story 5.2 findings 3–15; change migrator declaration matching; change precheck semantics; reinterpret schema version zero; weaken non-adjacent rejection; replace the coordinator regression with a direct helper-only test.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|---------------|----------------------------|----------------|
| Adjacent upward | predecessor `0x0001_0001`, successor `0x0002_0001` | Decode succeeds as cross-major input; coordinator invokes the successor migrator | No error |
| Adjacent downward | predecessor `0x0002_0001`, successor `0x0001_0001` | Decode remains allowed, preserving current behavior | No error |
| Wider upward gap | predecessor major 1, successor major 3 | Decode rejects before migration | Existing `SchemaVersionMismatch`, mapped to `SchemaIncompatible` by coordinator |
| Wider downward gap | predecessor major 3, successor major 1 | Decode rejects before migration | Existing `SchemaVersionMismatch`, mapped to `SchemaIncompatible` by coordinator |
| Same major | equal high 16 bits | Existing same-major ordering checks are unchanged | Existing behavior |

</frozen-after-approval>

## Code Map

- `crates/maos-kernel-core/src/hot_swap/state_codec.rs` -- faulty major-distance guard and focused codec tests.
- `crates/maos-kernel-core/src/hot_swap/coordinator.rs` -- unchanged production caller that must reach `detect_compat` and `run_migrator` after decode.
- `crates/maos-kernel-core/src/hot_swap/migrator.rs` -- unchanged migration declaration and hook-dispatch semantics.
- `crates/maos-kernel-core/tests/hot_swap_cross_major_migration.rs` -- existing AC2 integration target; currently tests helpers but not coordinator dispatch.
- `crates/maos-kernel-core/benches/hot_swap_latency.rs` -- reference for constructing a fully wired local `TestKernel`; not an edit target.
- `crates/maos-kernel-core/src/lifecycle/upgrade.rs` -- unchanged indirect caller for `HotSwap` and `Migrator` policies; both inherit the corrected coordinator behavior.

## Tasks & Acceptance

**Execution:**
- [x] `crates/maos-kernel-core/src/hot_swap/state_codec.rs` -- replaced directional wrapping subtraction with symmetric major distance; added focused adjacent-upward, adjacent-downward, and two-or-more-major boundary coverage.
- [x] `crates/maos-kernel-core/tests/hot_swap_cross_major_migration.rs` -- added a fully wired coordinator regression using the established hot-swap harness pattern: running predecessor snapshot `b"hello"`, adjacent-major successor with matching `migrates_from`, transforming migrator, and `on_swap_in` capture.

**Acceptance Criteria:**
- Given a running major-1 predecessor and a same-class major-2 successor declaring the predecessor version, when `HotSwapCoordinator::initiate_swap` runs, then it returns `HotSwapResult::Completed` with `SchemaCompat::CrossMajor`.
- Given the adjacent-major coordinator fixture, when the swap executes, then the successor's `migrate` hook runs exactly once with `b"hello"` and `on_swap_in` receives transformed successor-state bytes rather than predecessor bytes.
- Given the regression test, when it runs against the pre-fix wrapping-subtraction implementation, then it fails before migration; when it runs against the symmetric-distance implementation, then it passes.
- Given the focused and existing suites, when verification runs, then same-major, missing-migrator, pattern-mismatch, malformed-envelope, and non-adjacent mismatch behavior remains green.

## Spec Change Log

- 2026-08-12: Replaced the directional wrapping subtraction with `u32::abs_diff`, added bidirectional codec boundaries, and added a coordinator-level adjacent-upward migration regression.
- 2026-08-12: Edge Case Hunter found `schema_version = 0` became adjacent to major 1 under symmetric distance; decode now rejects zero on either side and a malformed-envelope regression preserves the nonzero invariant.

## Verification

**Commands:**
- `cargo test -p maos-kernel-core --lib hot_swap::state_codec::tests` -- expected: all codec boundary tests pass.
- `cargo test -p maos-kernel-core --test hot_swap_cross_major_migration` -- expected: adjacent upward swap completes through the real coordinator/migrator path and all existing AC2 tests pass.
- `cargo fmt --all --check` -- expected: no formatting diff.

**Observed 2026-08-12:**
- `cargo test -p maos-kernel-core --lib hot_swap::state_codec::tests` -- PASS, 12 tests.
- `cargo test -p maos-kernel-core --test hot_swap_cross_major_migration` -- PASS, 7 tests.
- `cargo fmt --all --check` -- PASS.
- LSP diagnostics for both changed Rust files -- clean.

## Suggested Review Order

**Compatibility policy**

- Symmetric major distance removes directional underflow while retaining the one-major window.
  [`state_codec.rs:141`](../../crates/maos-kernel-core/src/hot_swap/state_codec.rs#L141)

- Explicit zero rejection preserves the codec's existing nonzero schema invariant.
  [`state_codec.rs:131`](../../crates/maos-kernel-core/src/hot_swap/state_codec.rs#L131)

**Behavioral proof**

- A fully wired coordinator fixture exercises the real major-1 to major-2 path.
  [`hot_swap_cross_major_migration.rs:75`](../../crates/maos-kernel-core/tests/hot_swap_cross_major_migration.rs#L75)

- Assertions prove one migration and delivery of transformed successor state.
  [`hot_swap_cross_major_migration.rs:180`](../../crates/maos-kernel-core/tests/hot_swap_cross_major_migration.rs#L180)

**Boundary proof**

- Codec tests cover zero, both adjacent directions, and both wider-gap directions.
  [`state_codec.rs:239`](../../crates/maos-kernel-core/src/hot_swap/state_codec.rs#L239)
