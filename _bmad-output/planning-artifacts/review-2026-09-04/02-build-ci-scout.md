# Build/CI scout (HEAD 4657cace)
- cargo check --workspace --all-targets: 0 errors, 302 warnings (71 unused imports, 27 unused vars, ~40 dead code).
- Tests: 4-crate subset 1035 pass / 0 fail / 1 ignored. Workspace 4365 #[test] (2356 src / 2114 tests/). 136 #[ignore] (107 postgres, 60 live, 30 gate, 7 substrate; 13 bare).
- GATE RUNS AT HEAD: PASS kernel-baseline(24472), workspace-count 55, check-unsafe 0, epic-close-coherence, rotation-real-timing (p99 306ms), stability-matrix, abi-diff (vs CI base).
  FAIL (rc1, all in aggregate needs => aggregate RED at HEAD):
   * check-empty-kernel: I9 violation ScbRuntimeSnapshot control_block.rs:244; SecurityManagerAdapter security/mod.rs:130; VerifiedImageLock t3/image_lock.rs:27; 2 undocumented #[i9_exempt]
   * check-service-boundary: 9x NFR-Test-2 'other' class symbols (hot_swap::state_codec, t3::image_lock, spawn_t3, HotSwapCoordinator, ScbRuntimeSnapshot) + 4 P3
   * kloc-check: maos-domain 8695 > 8644 OVER; NFR-Maint-1 alarm current=155093 (16 KLOC threshold)
   * check-env-contract: main.rs:2669 MAOS_OPERATOR_BEARER_TOKEN, :2670 MAOS_OPERATOR_HTTP_BIND unregistered
  NOT-FOUND: check-model-currency
- 14-2b story ledger discloses only kloc-check + check-multi-tenant-loom red; 3 others undisclosed.
- gate-registry: 75 flat gates, 38 ship_gate rows; 12 flat gates have no discipline job; 5 in NO workflow (check-adr040-accepted, check-equiv-fixture-provenance, check-literal-reappearance, example-spirit-regen, rebaseline-check).
- CURRENT_PHASE="v1_5" gate_common.rs:166 vs tests/phase-config.toml:5 "v0.1-alpha" vs check_escape_detector.rs:62 private copy. 24 blocking / 14 advisory; 2 purely phase-keyed (cross-form-equiv, escape-detector).
- discipline.yml 160 jobs; aggregate needs 127; zero if:false; continue-on-error: reproducible-build, nfr-perf-1, nfr-perf-j4, nfr-perf-8; step-level skip: determinism-tests, t3-escape-corpus, t3-smoke-busybox. aggregate tolerates 'skipped' (:3844). Orphans: bench-audit-query-latency, nfr-perf-j4-latency.
- xtask src 51,675 LOC (tokei 41,925 vs own ceiling 41,932 = 7 lines headroom); 72 check_* modules; CI YAML 5,049.
- sprint-status.yaml:300 epic-14 cites pin @23141 (stale vs 24472).
- tests/ dirs (2114 tests) escape kloc ceilings; src #[cfg(test)] charged.

# Full workspace test run (my own, 2026-09-04 05:08 UTC)
- cargo test --workspace --no-fail-fast: 495 binaries, 4119 passed, 1 FAILED, 118 ignored.
- Deterministic RED (reproduced twice): crates/maos-a2a-tcp/tests/t_14_2a_post_grace_journal.rs:362 `t_14_2a_a_promoted_generation_makes_the_retired_leaf_a_queryable_refusal` — asserts tracing line contains `cert_post_grace_reject`; captured tracing: [] (empty). Test authored in ee3422d0 (14-2a, 2026-08-31). So HEAD carries a red test from a story marked done.
- CLASSIFIED: the red passes alone and with --test-threads=1 => test-isolation race (global tracing subscriber shared across tests in one binary), same class as register D16. check-cert-rotation-trigger PASSES (runs each enrolled test by exact name in isolation) while `cargo test --workspace` reds => "gate green, suite red".
