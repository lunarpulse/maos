# Landing-1 DIFFICULT-review corrections

Scope: fix-list items 1–9 against candidate `2e694cd6`; story remains in-progress for landing 2. No commit or push. Commands run from `/home/lunarpulse/dev_ws`, cargo always prefixed `env -u CI -u GITHUB_ACTIONS`, inherited target directory retained (no override, clean, or alternate worktree). Exact argv/environment/exit codes/wall time are in [commands.json](commands.json); per-command records retain output, except the clippy count form omits successful-artifact inventory and retains all warning/error diagnostics.

## Changes and decisive witnesses

| Item | Change | Witness / result |
|---|---|---|
| 1 | `xtask/tests/diagnostics_20_3a.rs:49` creates `.github/workflows/fixture.yml`; missing directory still fails closed in production. The nested witness cargo invocation no longer overrides target-dir. Searched expensive tests and isolated callers in xtask, crates, tests, spirits: diagnostics is the only coherence temp-tree fixture affected; d19 and owner-assertion fixtures invoke the other gates without a workflow dependency. | Requested ignored diagnostics: 6 passed. Entire `check-test-cost --expensive-filter` intersection with xtask: 11 passed (6 diagnostics + 5 ABI). |
| 2 | `xtask/src/check_review_findings_resolved.rs:103` scans placeholders through the next level-two heading independently of the existing table/checklist extent. | `d19_story_file_governance::a_nested_review_heading_does_not_hide_a_bare_placeholder` passed. Final real review gate: 196 stories, zero violations. |
| 3 | `xtask/tests/d11_xtask_ceiling_ratchet.rs:373` locates the historical `47682 -> 42400` arrow in any HISTORY segment and calls the file's `authorize_row` logic for the live row. The constant 42400 intentionally remains historical, not a frozen live ceiling. | Ratchet suite passed in the final workspace run; the current authorized `42400 -> 42405` correction itself exercises a later first HISTORY segment. Landing 2 can fund its own raise without editing this historical assertion. |
| 4 | `xtask/tests/d19_story_file_governance.rs:296` pairs an in-progress story with a concrete model (green) and without one (red), asserting `no concrete dev model recorded`. Done-story vectors remain. | Paired test passed. |
| 5 | `xtask/tests/diagnostics_20_3a.rs:344` exercises the CLI entry point on an isolated clean tree, plants `if: false`, requires exit 1 and `disabled-workflow-job`, removes it and requires green/empty violations. | Gate-entry-point test passed in the final workspace run; existing helper literal-form vectors remain. |
| 6 | Scratch `.github/workflows/zz-plant-203b.yml` planted in the real checkout, not a copy; removed afterward. | Clean 0 → planted 1 naming `disabled-workflow-job` → restored 0, each 0.019s. Empty scratch-path git status. All 12 existing workflows byte-identical; hashes and exact commands in [real-tree evidence](../plants/if-false-results.json). Old fixture record explicitly retained as [fixture-only evidence](../plants/if-false-fixture-results.json). |
| 7 | Actual coverage-completeness, coverage-matrix and clippy preflight measurements recorded. | Completeness rc0, 55 v1.0 entries, no empty gates. Coverage matrix rc0 **but passed=false**, warning mode, 33 violations / 198 rows; no replica claim. Clippy -D warnings rc101, then full count form rc0: **1411 warning occurrences / 884 source-distinct diagnostics**. STOP D-20-3b-11 A; no lint repairs/blocking-job addition. |
| 8 | `ci/ci-20-3b-verify.yml:5,24-26,35` builds release maos-bin + xtask for smoke and debug xtask for ship-completeness, matching the two production job profiles. Header and story explicitly disclaim ledger proof. | Static change only; no hosted run. Full discipline remains required to prove downloaded artifacts, audit key and `MAOS_LEDGER_ARTIFACTS` path. Direct ship-completeness rc0, expected 45, missing none, ledger not consumed. |
| 9 | Story Dev Agent Record documents AC3.1 ordering deviation. | A single candidate commit cannot independently prove bridge-first editing. No surviving bridge consumer exists in the retirement closure; retirement/enrolment removal is atomic, so no shipped intermediate dependency break. |

## Exact final pricing

New integration witnesses are uncharged; independent placeholder extent costs +5 tokei. HISTORY segments are prepended, all previous segments retained under `| PRIOR:`.

| Component | Before review | Final measured / pin | L1 envelope / invariant |
|---|---:|---:|---|
| xtask | 42400 | **42405 / 42405** | ≤43608 |
| maos-bin | 32937 | **32937 / 32937** | unchanged |
| kernel tokei | 20618 | **20618 / 20618** | unchanged |
| kernel physical | 26409 | **26409 / 26409** | 95 files, no changed/added/removed |
| aggregate | 187494 | **187499** | strict hardfail **187500** (= measured + 1) |

The historical retirement arrow `47682 -> 42400` remains recorded; current first HISTORY arrow `42400 -> 42405` cites `20-3b-gate-retirement-and-coverage-generator` and is also recorded in the story. No reservation/headroom was added.

## Final verification

- `cargo fmt --all`: rc0.
- `cargo build --locked --workspace --all-targets`: rc0, existing warnings retained.
- `cargo nextest run --workspace --locked --color never --no-fail-fast --test-threads 1`: rc0, **5376 passed / 137 skipped**. Serial run avoids the two existing per-binary timing budgets tripped by default parallel contention. The exact default-parallel requested command was also run and passed 5376/5376; its red timing report is preserved.
- Entire xtask expensive-filter set: **11 passed**, rc0. Existing official fixture manifest sufficed; no fixture-producer rerun needed.
- All nine requested standalone gates: rc0; `check-test-budget` final serial report has 567 binaries, sum **575.413s**, no findings. Extra `check-review-findings-resolved` and `check-ship-gate-completeness`: rc0.
- Prescribed prepush: rc0 (**61 pass, 0 red, 1 advisory, 8 skip**). Its own fresh default-parallel timing report is advisory-red on `maos-bin::one_daemon_one_door_16_1` and `maos-iac::participant_recall_17_8`; the standalone final serial budget is green. No exceptions or timing assertions were weakened. Skips: docs build/routes (no applicable changes), cargo-deny (no lock/deny changes), five dynamic CI-marker cardinalities. Runtime signed-T3, Python cryptography, and strace prerequisites were not provisioned; output diagnostics retained.
- Hosted workflow/full discipline not run; local green does not prove the ledger-consuming hosted path. No kernel or operator Dockerfile editing, no alternate worktree access, no commit/push.

## T0 clippy STOP and top lints grouped by crate

Deny-warnings form stopped with four denied lints in maos-spirit-abi: too_many_arguments ×2, type_complexity ×1, manual_contains ×1. Full JSON count form then completed rc0 without `-D warnings`: 1411 emitted warning diagnostics, comprising 1143 clippy-lint occurrences and 268 rustc-warning occurrences. Repeated library/lib-test-target emissions count separately; deduplication by package, lint/message and primary source ranges gives 884 source-distinct diagnostics. Complete grouped counts and diagnostic locations: [t0-clippy-summary.json](t0-clippy-summary.json). **Operator disposition is required before continuing the clippy lane; nothing was fixed or suppressed.**

| Crate | Warning occurrences | Top three lint codes |
|---|---:|---|
| maos-kernel-core | 282 | unused_imports ×53, dead_code ×42, clippy::too_many_arguments ×22 |
| xtask | 207 | clippy::doc_lazy_continuation ×29, dead_code ×25, clippy::unnecessary_map_or ×15 |
| maos-bin | 149 | clippy::too_many_arguments ×23, clippy::unusual_byte_groupings ×19, clippy::await_holding_lock ×13 |
| maos-a2a-tcp | 86 | clippy::too_many_arguments ×24, clippy::assertions_on_constants ×22, clippy::cloned_ref_to_slice_refs ×22 |
| maos-eval | 83 | clippy::unnecessary_map_or ×29, clippy::io_other_error ×14, clippy::needless_borrow ×7 |
| maos-iac | 66 | clippy::result_large_err ×18, clippy::result_unit_err ×8, clippy::redundant_closure ×6 |
| maos-registry | 63 | clippy::needless_borrows_for_generic_args ×24, unused_imports ×11, unused_variables ×10 |
| maos-loom-lite | 52 | clippy::await_holding_lock ×42, clippy::redundant_field_names ×4, clippy::too_many_arguments ×2 |
| maos-audit | 44 | clippy::field_reassign_with_default ×7, clippy::explicit_counter_loop ×4, clippy::redundant_closure ×4 |
| butler | 43 | clippy::result_large_err ×27, clippy::needless_borrow ×7, clippy::type_complexity ×6 |
| maos-domain | 38 | clippy::derivable_impls ×6, clippy::bool_assert_comparison ×5, async_fn_in_trait ×4 |
| maos-corpus-gen | 34 | clippy::needless_borrows_for_generic_args ×14, clippy::should_implement_trait ×4, clippy::type_complexity ×4 |
| maos-cli | 33 | clippy::suspicious_to_owned ×13, clippy::manual_strip ×4, clippy::type_complexity ×3 |
| maos-bench | 29 | clippy::unnecessary_map_or ×4, clippy::unusual_byte_groupings ×4, clippy::manual_is_multiple_of ×2 |
| maos-manifest | 16 | clippy::derivable_impls ×6, clippy::chunks_exact_to_as_chunks ×2, clippy::manual_is_multiple_of ×2 |
| maos-cohort | 14 | clippy::duplicated_attributes ×6, clippy::type_complexity ×3, clippy::manual_div_ceil ×2 |
| maos-spirit-sdk | 13 | unused_mut ×10, clippy::doc_lazy_continuation ×2, dead_code ×1 |
| researcher | 12 | clippy::needless_borrow ×5, clippy::unusual_byte_groupings ×3, clippy::chunks_exact_to_as_chunks ×2 |
| maos-a2a-core | 11 | clippy::doc_lazy_continuation ×6, clippy::result_large_err ×2, clippy::useless_format ×2 |
| maos-wasm-host | 11 | clippy::needless_borrows_for_generic_args ×7, clippy::cloned_ref_to_slice_refs ×2, clippy::new_without_default ×2 |
| maos-spirit-abi | 10 | clippy::too_many_arguments ×4, clippy::assertions_on_constants ×2, clippy::manual_contains ×2 |
| maos-a2a | 9 | clippy::doc_lazy_continuation ×6, clippy::doc_overindented_list_items ×2, clippy::useless_format ×1 |
| maos-skill | 8 | clippy::doc_lazy_continuation ×6, clippy::incompatible_msrv ×2 |
| nash | 8 | clippy::doc_lazy_continuation ×6, clippy::needless_borrow ×2 |
| maos-pdp | 7 | clippy::cloned_ref_to_slice_refs ×5, clippy::useless_conversion ×2 |
| maos-capability | 6 | clippy::clone_on_copy ×2, clippy::new_without_default ×2, unused_imports ×2 |
| maos-control | 6 | deprecated ×4, clippy::needless_lifetimes ×2 |
| maos-journey-test | 6 | clippy::derivable_impls ×2, clippy::question_mark ×2, clippy::while_let_loop ×2 |
| maos-spirit-cli | 6 | clippy::needless_borrows_for_generic_args ×4, dead_code ×2 |
| maos-providers | 5 | unused_imports ×5 |
| maos-acp | 4 | unused_imports ×2, unused_variables ×2 |
| maos-director-surface | 4 | clippy::redundant_closure ×2, unused_variables ×2 |
| maos-shell | 4 | dead_code ×3, unused_imports ×1 |
| architect | 4 | clippy::doc_lazy_continuation ×2, clippy::needless_borrow ×2 |
| orchestrator | 4 | clippy::too_many_arguments ×2, clippy::needless_borrow ×1, clippy::unusual_byte_groupings ×1 |
| maos-compliance | 3 | clippy::too_many_arguments ×2, clippy::doc_lazy_continuation ×1 |
| maos-egress | 3 | clippy::collapsible_if ×2, clippy::type_complexity ×1 |
| example-spirit | 3 | clippy::needless_return ×2, clippy::needless_borrow ×1 |
| mira | 3 | clippy::needless_borrow ×2, clippy::too_many_arguments ×1 |
| observer | 3 | clippy::needless_borrow ×2, clippy::needless_borrows_for_generic_args ×1 |
| worker | 3 | clippy::unusual_byte_groupings ×3 |
| maos-fkcs | 2 | clippy::derivable_impls ×2 |
| maos-mcp | 2 | unused_imports ×2 |
| maos-secrets | 2 | unused_imports ×2 |
| maos-siem | 2 | clippy::field_reassign_with_default ×2 |
| maos-sso | 2 | unused_imports ×2 |
| digest | 2 | clippy::chunks_exact_to_as_chunks ×1, unused_mut ×1 |
| reviewer | 2 | clippy::needless_borrow ×2 |
| maos-frame-codec | 1 | clippy::manual_repeat_n ×1 |
| maos-spirit-hello | 1 | clippy::single_component_path_imports ×1 |

## Real-tree restoration digest

Restored workflow-set digest: `39a6d498c9d5434d847c94853b4cd5c05acee546f623a56ac379152ad923382b`. Algorithm: SHA-256 over UTF-8 `json.dumps(sha256_by_workflow, sort_keys=True)`, where each file digest hashes the exact workflow bytes. Per-file before/restored digests and scratch removal status are recorded in `../plants/if-false-results.json`.

## Complete command-result ledger

All exact command strings are in [commands.json](commands.json). Earlier failed/intermediate attempts are retained, not overwritten: the initial charged unit witness was moved to uncharged CLI integration coverage; widening table parsing exposed unrelated sibling-table findings, so placeholder scope was isolated; timing contention was resolved for the standalone budget via a full serial workspace run.

| Label | Exit code | Wall seconds | Output record |
|---|---:|---:|---|
| `fmt` | 0 | 2.441 | `fmt.json` |
| `build` | 0 | 57.224 | `build.json` |
| `t0-coverage-completeness` | 0 | 0.015 | `t0-coverage-completeness.json` |
| `t0-coverage-matrix` | 0 | 0.012 | `t0-coverage-matrix.json` |
| `t0-clippy` | 101 | 15.395 | `t0-clippy.json` |
| `t0-clippy-count` | 0 | 61.807 | `t0-clippy-count.json` |
| `ignored-diagnostics` | 0 | 13.965 | `ignored-diagnostics.json` |
| `kloc-measurement` | 1 | 0.034 | `kloc-measurement.json` |
| `expensive-filter` | 0 | 0.242 | `expensive-filter.json` |
| `plant-real-tree-clean` | 0 | 0.019 | `plant-real-tree-clean.json` |
| `plant-real-tree-disabled` | 1 | 0.019 | `plant-real-tree-disabled.json` |
| `plant-real-tree-restored` | 0 | 0.019 | `plant-real-tree-restored.json` |
| `plant-restoration-status` | 0 | 0.006 | `plant-restoration-status.json` |
| `fmt-final` | 0 | 2.480 | `fmt-final.json` |
| `build-final` | 0 | 3.145 | `build-final.json` |
| `workspace-nextest` | 0 | 46.912 | `workspace-nextest.json` |
| `expensive-xtask-gates` | 0 | 4.081 | `expensive-xtask-gates.json` |
| `gate-kloc-check` | 0 | 0.030 | `gate-kloc-check.json` |
| `gate-check-kernel-baseline` | 0 | 0.010 | `gate-check-kernel-baseline.json` |
| `gate-check-dev-record-completeness` | 0 | 0.082 | `gate-check-dev-record-completeness.json` |
| `gate-check-epic-close-coherence` | 0 | 0.018 | `gate-check-epic-close-coherence.json` |
| `gate-check-env-contract` | 0 | 0.406 | `gate-check-env-contract.json` |
| `gate-check-service-boundary` | 0 | 1.075 | `gate-check-service-boundary.json` |
| `gate-check-j1-loopback-delegation` | 0 | 0.037 | `gate-check-j1-loopback-delegation.json` |
| `gate-check-test-cost` | 0 | 0.237 | `gate-check-test-cost.json` |
| `gate-check-test-budget` | 1 | 0.017 | `gate-check-test-budget.json` |
| `workspace-nextest-serial` | 0 | 579.522 | `workspace-nextest-serial.json` |
| `gate-check-test-budget-serial` | 0 | 0.017 | `gate-check-test-budget-serial.json` |
| `gate-check-review-findings-resolved` | 1 | 0.085 | `gate-check-review-findings-resolved.json` |
| `prepush` | 0 | 168.678 | `prepush.json` |
| `fmt-placeholder-scope` | 0 | 2.465 | `fmt-placeholder-scope.json` |
| `kloc-placeholder-scope` | 1 | 0.031 | `kloc-placeholder-scope.json` |
| `final-fmt` | 0 | 2.453 | `final-fmt.json` |
| `final-build` | 0 | 71.833 | `final-build.json` |
| `final-workspace-nextest` | 0 | 642.905 | `final-workspace-nextest.json` |
| `final-gate-check-test-budget` | 0 | 0.017 | `final-gate-check-test-budget.json` |
| `final-expensive-xtask-gates` | 0 | 4.141 | `final-expensive-xtask-gates.json` |
| `final-gate-kloc-check` | 0 | 0.032 | `final-gate-kloc-check.json` |
| `final-gate-check-kernel-baseline` | 0 | 0.010 | `final-gate-check-kernel-baseline.json` |
| `final-gate-check-dev-record-completeness` | 0 | 0.093 | `final-gate-check-dev-record-completeness.json` |
| `final-gate-check-epic-close-coherence` | 0 | 0.018 | `final-gate-check-epic-close-coherence.json` |
| `final-gate-check-env-contract` | 0 | 0.413 | `final-gate-check-env-contract.json` |
| `final-gate-check-service-boundary` | 0 | 1.075 | `final-gate-check-service-boundary.json` |
| `final-gate-check-j1-loopback-delegation` | 0 | 0.038 | `final-gate-check-j1-loopback-delegation.json` |
| `final-gate-check-test-cost` | 0 | 0.243 | `final-gate-check-test-cost.json` |
| `final-gate-check-review-findings-resolved` | 0 | 0.061 | `final-gate-check-review-findings-resolved.json` |
| `final-gate-check-ship-gate-completeness` | 0 | 0.030 | `final-gate-check-ship-gate-completeness.json` |
| `final-prepush` | 0 | 171.242 | `final-prepush.json` |
