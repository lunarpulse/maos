# Epic 18 landing reds → the prepush leg that would have caught them

Measured 2026-10-27 with `gh api repos/lunarpulse/maos/actions/runs/<id>/jobs` (failed jobs and failed steps) for every run the retro cites (`epic-18-retro-2026-10-27.md:58`). Fan-in gates (`aggregate`, `v1-0-ship-gate`, `v1-5-ship-gate`) omitted.

| Run @ SHA (story) | Red job :: step | Class | prepush leg (20-16, recommended rows) |
|---|---|---|---|
| 40183021781 @ 08ae68ad (18-1) | workspace-test-suite :: nextest | unlisted `butler_root.rs` in `SCANNED_SOURCE_FILES` (`cohort_daemon_smoke_13_5c.rs:927`) | workspace nextest (D-20-16-01 C); maos-bin package under A |
| | check-j1-loopback-delegation; check-reza-production-path | tests outside the focused set (maos-bin suites under the gates) | workspace nextest covers the suites; substrate legs SKIP locally |
| | check-multi-tenant-loom | 20-7-owned S3 podman-4 5 s bound (sprint row 18-1) — not drift | none (CI only) |
| 40191703238 @ 140b906b (18-8) | check-serde-error-handling | serde handling | `check-serde-error-handling` |
| 40237233599 @ b96d123a (18-3) | workspace-test-suite; check-test-budget | FR4 scanner (`fr4_classifier_16_2.rs:859-861` scans kernel-core/bin/iac src) + 37.3 s > 30 s budget | workspace nextest (cross-crate scanner); budget: local junit is advisory only (D-20-16-03) |
| 40251998973 @ 79f49709 (18-4) | onb-nfr2-timing :: script | binary size 33,666,176 > 33,554,432 B (`tests/integration/onb_nfr2_timing.sh:115-118`) | conditional release+strip leg (D-20-16-03 A) |
| | check-multi-tenant-loom | S3 bound (as above) | none |
| 40267102242 @ 5495863e (18-5) | gen-abi-docs | WIT/ABI parity | `gen-abi-docs --check` |
| | check-ko-coverage :: glossary-lock | KO parity | docs glossary-lock + ko-coverage (MIN=100) |
| | epic-17-exit, nfr-rel-11, journey-hermetic-tier-1, one-daemon-one-door, j0-scene, workspace-test-suite, check-test-budget, example-spirit-ts-tests, check-multi-tenant-loom | runtime defect (strong IAC factory → drain stalls, fixed by a weak factory, S5) — not drift | workspace nextest catches the in-suite part; daemon/substrate legs are CI |
| 40273714381 @ 3aad2f4c (18-5) | wasm-host-tests ×4 :: verdict/hostile gates | marker pin GOVERNED-ATOMIC 8 vs 6 (`discipline.yml:4048-4071`) | **Named SKIP under operator C** (D-20-16-02 addendum): the marker is dynamically emitted, so prepush reports an indeterminate named SKIP and does **not** catch the 8-vs-6 drift by design; the actual count stays CI-only. The count pin on the same step is still checked. |
| 40319842655 @ f8f8abc2 (18-7, docs-site) | docs :: gate:static | `FAIL: route-manifest error_codes != error-catalog keys` (`docs-site/scripts/gate-troubleshoot-bidi.js:76-94`) | docs troubleshoot-bidi (no build) — AC1's proven-red |
| 40336401565 @ a2d69a29 (18-7) | cargo-deny | zstd advisories (RUSTSEC-2026-0391/0398/0403) | conditional `cargo deny check` (D-20-16-03 A; tool absent locally today) |
| | check-stability-matrix; smoke-abi-7-5a | STABILITY.md drift ×2 | `stability-matrix --check` |
| | journey-hermetic-tier-1 :: skill port | ANSI in a count pin (`discipline.yml:3367-3368`, `CARGO_TERM_COLOR: always` `:28`) | pin reader ANSI lint (D-20-16-02) |
| | workspace-test-suite | `error_catalog_20_3a` 49 → 67 (xtask test over another crate's codes) + `context_refusal_18_5` | `error-catalog-check` + workspace nextest |
| | check-multi-tenant-loom | loom package leg (maos-bin whole package) | workspace nextest (maos-bin package) |

Coverage: every drift-class row (all rows except the S3 podman bound and the 18-5 runtime defect) has a prepush leg, with one qualification: the GOVERNED-ATOMIC marker row is a named SKIP under operator C (uncaught by design; its count pin is still checked, the actual marker value is CI-only). The rows that need a substrate (marker-pin values, loom/podman legs) are caught only statically or by their in-suite part.
