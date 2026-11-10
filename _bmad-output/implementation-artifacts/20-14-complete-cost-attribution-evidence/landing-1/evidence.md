# 20-14 L1 implementation evidence

## Rulings and measurements

D-20-14-13 A (operator `d1408`) retains the kernel's exhaustive payload literal and adds a flattened frame wrapper. D02 A exact starting pins: domain 10198, CLI 6429, audit 8179, bin 36225, aggregate 198914, kernel 26841 physical/21042 tokei. Original rebased ceilings domain 10200, CLI 6524, audit 8237, bin 36475, aggregate 199369. Orchestrator-adjudicated overdrafts, disclosed to the operator: domain 10200 -> 10215 (wrapper: initial 10207 + 8), CLI 6524 -> 6545 (formatted reader and new in-source witness: 6528 + 17). Other ceilings unchanged. New serde-only vectors moved to tests/ without moving prior tests; after the workspace run a one-line schema-1 task assertion brought CLI to 6529 and aggregate to 199049 (hardfail 199050); domain 10203, audit 8209, bin 36225, kernel 26841/21042. That last assertion was not run. `kloc-check --json` exited 1 before these exact pins were updated; the earlier 199048 gate and kernel baseline exited 0. 20-9's domain forecast is 0..6 and 20-5's +8: both re-base; if those forecasts materialize the cumulative pin is 10211..10217 [INFERENCE], not a reservation.

The new report vector exercises private `build_cost_report` and private row fields; moving it to an integration test would require exposing production internals or a second reporting API. Existing counted tests were not moved.

After the schema-1 assertion and exact re-pin, `kloc-check --json` exited 0
at aggregate 199049; that test assertion itself remains unrun.

## Witnesses and plants

- AC4: creation/runtime/reconcile-head.ndjson observed the old one-row 100/60 result. `cost_frame_task_and_legacy_serde` (maos-domain/src/cost.rs) covers legacy and set task, u64 max and unknown top-level fields; `flattened_frame_rejects_unknown_dimensions_and_duplicate_keys` (maos-domain/tests/cost_frame_20_14.rs) covers duplicate fields and unknown dimensions. `cost_report_separates_task_source_and_boot_names` (maos-cli/src/subcommands.rs) separates task/source/boot-name and checks both output notes; `cost_spirit_names_distinguish_boots_and_skip_echoes` (maos-audit/tests/spirit_resolution.rs) exercises real SQLite identity rows. Restoring the old key by fixing the new key's added axes caused the report vector to fail with 1 row instead of 4 (exit 101, 51.74 s); source restored by cp, cmp, touch. Focused restored report, domain wrapper and audit identity tests exited 0 before the vector split.
- AC5: the creation inventory records the missing operator page. `operator_cost_guides_disclose_limits` (maos-cli/tests/cost_docs_20_14.rs) checks all three guides and ADR. Plant replacing the source guide heading exited 101 (28.26 s); cp, cmp, touch restoration; restored test exited 0 (2.71 s).
- AC6: coverage matrix names the actual L1 witnesses without claiming a gate; FR64 and NFR-Cost-1 closure remain L2. Worker CLI spend remains ownerless/open under the existing D-20-14-06 row.

## Verification limitations

The focused three-crate nextest invocation exited 100 after 204/796 passed: `erasure_review_20_11::p3_p4_f4_f5_f6_f11_f12_both_verifiers_enforce_the_same_signed_contract` reported missing Python `cryptography`. It was not retried to green. Independent focused tests above passed. `check-exit-commands` and `check-decision-register` exited 0. Hosted branch recipe `ci/ci-20-14-verify.yml` is unverified and is not installed on main.

T0 ring inspection: `CapTokensShardRing::get_scope` is a lookup without expiry/revocation validation; expiration is removed by periodic shard eviction. The original prototype's pruning must account for TTL at L2 as creation measurement 2 anticipated. Researcher mints once per survey and reuses the token for completions; Butler mints once per pass. Other T0 measurements and final-tree verification are not asserted by this record. AC1, AC2, AC3 and AC6 full closure remain open until L2; L1 writer does not emit schema 2 or bind principals.

Preflight source classification: `shell_host.rs:114` and `main.rs:4161` are
the production shell and shared seam ProviderInfer mints; `main.rs:9578` is
the one-shot hello mint; `main.rs:8281` is a revocation smoke mint. The
generic spawned-frame `spirit_session.rs:914-940` can mint mediated task
scopes, including ProviderInfer, but retains issued token IDs as session
leases (`:1053-1056`); it does not hand a capability token to an Inference
Port caller. The Worker mint at `worker_spawn.rs:966` is
`CliSubprocessSpawn`, not inference. The D-01 binding remains L2.

The fixture builder and the genuine `cargo build --locked -p maos-bin -p
maos-cli` exited 0. The prepush job was launched but a runtime interruption
lost its handle before a result was delivered; no timeout or pass is claimed.

The separately restarted frozen-tree prepush timed out at 3000.05 s without
buffered JSON. No tracked files were edited during it. After fixture and
binary builds, cargo metadata and nextest binary metadata both exited 0.
One workspace run with those metadata files exited 100: 5879 passed, two
failed, 137 skipped (`workspace-nextest.log:5957-5960`). The failures were
`no_data_home_does_not_abort_the_daemon_or_offline_children` (exit 78,
`:2801-2817`, reported to Main) and the existing 20-11 1M pending observation
(`:5843-5862`, routed to 20-7 in deferred-work). No failure was retried.
`check-fkcs --json`, `coverage-matrix --json`, `check-test-cost --json`,
`check-env-contract --json`, `check-dev-record-completeness --json`,
`check-epic-close-coherence --json`, and changed-files `invariant-lock --json`
each exited 0. The no-default-feature verb test and Apple/Windows checks were
not run locally; hosted execution is unverified.

After separating the new serde-only vectors, `cargo test --locked -p
maos-domain --test cost_frame_20_14` exited 0 (one test, 188.52 s).
The prior bare-payload test was replaced in place, not moved. The post-format
pre-pin kloc gate exited 0 at aggregate 199048; exact downward pins are 10203
for domain and 199049 aggregate hardfail.

T0 SR-3 witness `session_principal_cost_scrub_preserves_other_frame`
(`crates/maos-kernel-core/tests/forget_cascade_18_6.rs:195`) exited 0:
the schema-2 session-principal frame is redacted with dimensions retained and
the other principal's frame bytes unchanged (one test, 271.44 s including build).

Cleanup: removed own scratch (33,837,056 allocated bytes) and 22 stale
incremental session directories (approximately 596,881,408 allocated bytes),
excluding each crate's newest session, requiring age over three hours and
non-blocking flock and record-lock probes. `/proc/1/fd` and `/proc/7/fd`
were unreadable (permission denied); no deps, build or fingerprint trees
were pruned.
