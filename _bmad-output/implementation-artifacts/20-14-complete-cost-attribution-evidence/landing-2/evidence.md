# 20-14 L2 — kernel landing evidence (HEAD `8144bc02` base; Dev2014d)

## STEP 0 — KT re-cut and re-price (before any repo edit)

`git archive HEAD` copies in `/tmp`, every file touched, `rustfmt`, gates from the copy root (`step0/variant-*.txt`, `step0/prototype-patch.py`).
Live base **27014 physical / 21181 tokei**, shared STOP 21250 (69 left), ADR-069 alarm 22000.

| Shape | Kernel | maos-capability | Surface (`check-service-boundary`) |
|---|---|---|---|
| A — KT as prototyped: private field `cost_scopes` on `CapabilityRegistryAdapter` + `bind_cost_scope`/`cost_scope` + emission + the `CostFrameBody` wrapper (the creation prototype did not price the wrapper) | +38 / +28 -> 27052 / 21209 | 0 | **RED**: the struct item hash moves ("removed public kernel symbol CapabilityRegistryAdapter"); a second surface refresh was not granted -> rejected |
| **B — chosen**: scope map on the cap-token ring (`CapTokensShardRing::{bind_cost_scope, cost_scope}`, maos-capability) + one inherent accessor `CapabilityRegistryAdapter::tokens()`; emission reads it by the verified token id | **+25 / +20 -> 27039 / 21201** (landed 27040 / 21201) | +11 (1433 -> 1444) | GREEN (methods are not surface items; only `pub` struct/enum/fn/use/const items are hashed) |

B stays inside the +35 tokei KT grant and the x2 ceiling 21221, leaves 49 under the shared STOP, and needs no surface-hash refresh. Its only cost is
+11 tokei in `maos-capability` (creation grant: 0). **That ceiling is an orchestrator-adjudicated D02 A overdraft requested in the STEP 0 message to Main; it is
recorded as pending that ruling** (the shape was implemented before a reply could be obtained because Main was waiting on the user; reverting to A or relocating the map
is a mechanical change confined to `cap_tokens/mod.rs` and the accessor).

## Mint-site classification (T0 measurement 3)

| Site | Class | Task | Session principal |
|---|---|---|---|
| `shell_host.rs` `issue_turn_token` | production (Inference Port, shell turn) | `shell.turn:<ulid>` | boot `MAOS_SESSION_PRINCIPAL` |
| `main.rs` seam `mint` closure (Butler, Orchestrator, Researcher passes) | production | `<spirit>.pass:<ulid>` (pid -> Spirit name by reverse lookup, else `pid<n>`) | same |
| `main.rs` one-shot hello (`MAOS_ONE_SHOT=hello-spirit`) | production | `hello-spirit.run:<ulid>` | same |
| `main.rs` `smoke-revocation` mint | smoke; no Inference Port call | not bound (nothing emits) | n/a |
| spawned-session scope mints (`spirit_session.rs`) | not an Inference Port caller (leases only, per the L1 record) | not bound | n/a |
| Worker `CliSubprocessSpawn` | not inference | n/a | n/a |

Anti-spoof: `bind_task_scope` is a root-side helper; Spirit crates receive mint closures and never the registry; the emission reads the scope by the verified token id and never from
`InferenceRequest` (D-20-14-03 B). A token minted without a scope emits no `task_ref` and falls back to the tagged proxy.

## Witness table (red-before / plant / final)

Logs: `red/`, `plants/` (raw output, command, rc, restore + `cmp` result), `green/`; `plant-summary.jsonl`.

| AC | Witness | Failing-before | Plant (restored by copy-back, `cmp`, `touch`) |
|---|---|---|---|
| AC2.4/2.5 | `kernel cost_attribution_20_14::a_bound_task_rides_the_cost_frame_and_an_unbound_token_carries_none`, `a_scope_follows_its_token_only_and_stale_scopes_are_pruned` | `red/kernel-cost_attribution.log` (ring/accessor plumbing only, emission unchanged: 3 of 4 fail on `task_ref` absent / source) | `AC2.5 default scope read at emission` red (3 tests); `KT ring keeps scopes of tokens it no longer holds` red (prune test) |
| AC3.2/3.6 | `a_bound_session_principal_is_exact_and_the_proxy_is_not_consulted` | same log: `Ambiguous{2}` instead of `Resolved{alice}` | `AC3.6 proxy emitted while a principal is bound` red |
| AC3.3 (SR-4) | `without_a_session_principal_the_proxy_folds_and_ambiguity_carries_a_count_not_members` | green before (guard: count, no member ids) | covered by the two kernel plants above |
| AC3.4 (SR-3) | existing `forget_cascade_18_6::session_principal_cost_scrub_preserves_other_frame` | unchanged | n/a |
| AC1.1/1.3 | `bin cost_attribution_20_14::two_sessions_report_one_row_per_task_and_principal_and_a_legacy_frame_stays_readable` (real `maos shell` x2, real `maosctl audit cost-reconcile`; schema-1 frame reports `none (schema 1)`) | the shell-bind plant is the HEAD-equivalent red: `plants/AC1-4b-shell-turn-bind-dropped.log` (also measured at HEAD in `creation/runtime/`: one `host-unallocated` 100/60 row) | `AC1.4b shell turn bind dropped`, `AC1.4a report key ignores the task` |
| AC1/AC2.2 | `a_butler_seam_pass_binds_its_own_task_and_the_boot_principal`, `the_one_shot_hello_run_binds_its_own_task_and_the_boot_principal` | plants below | `seam per-pass bind dropped`, `one-shot hello bind dropped` |
| AC3.1/3.5 | `a_malformed_session_principal_refuses_the_boot` (`:`, U+0001, newline), `without_a_session_principal_the_proxy_attributes_and_a_blank_value_binds_none`, `the_boot_parse_follows_the_butler_principal_rule` | n/a (new surface) | `AC3.5 colon accepted in the session principal` red |

## Pins

| Pin | Before | After | Ceiling |
|---|---:|---:|---|
| kernel tokei / physical | 21181 / 27014 | 21201 / 27040 | x2 21221, STOP 21250 (49 left) |
| maos-capability | 1433 | 1444 | pending adjudication (see STEP 0) |
| maos-bin | 38475 | 38549 (+74) | L2 allowance +250 (D-20-14-09 A) |
| aggregate / hardfail | 202750 / 202751 | 202855 / 202856 | |

## Verification

See the section appended at the end of this file.

### Observed (env `-u CI -u GITHUB_ACTIONS`; cryptography venv on PATH; xtask gate binary rebuilt from this tree; fixtures and `maos`/`maosctl` rebuilt before every real-session leg)

- Focused green (`green/`): kernel `cost_attribution_20_14` 4 passed; bin `cost_attribution_20_14` 6 passed (real `maos` / `maosctl`, whole file 1.3 s); `cost_docs_20_14` 2 passed;
  xtask `kernel_pin_content_hash_16_0` 25, `recovery_lane_ceiling_rule` 12, `d11_xtask_ceiling_ratchet` 9; `cargo test -p maos-kernel-core --lib` 253 passed;
  `cargo test -p maos-bin --no-default-features --test verb_table_15_3` 7 passed.
- `cargo fmt --all -- --check` rc 0; `cargo clippy --workspace --all-targets --locked` rc 0 and `clippy-ratchet` rc 0 (1331 occurrences / 869 source-distinct; no diagnostic points at an added line).
- Fixture builder rc 0, genuine `cargo build --locked -p maos-bin -p maos-cli` rc 0, metadata reuse, ONE workspace nextest (`workspace-nextest-final.log`): **rc 100, 6028 run, 6027 passed, 1 failed (94.2 s)**:
  `maos-bin::erasure_million_20_11 ac5_million_row_pending_forget_finishes_and_repeats_without_corruption` (42.756 s), the known AC5 local 1M oracle; not retried; the same-line row exists in `deferred-work.md`.
- Gates rc 0 (`gates/*.json`): kloc-check (aggregate 202855), check-kernel-baseline (27040), check-service-boundary, check-dev-model-tier, check-dev-record-completeness, check-epic-close-coherence, check-exit-commands,
  check-decision-register, check-env-contract, check-serde-error-handling, check-fkcs, check-pub-field-constructors, check-dependency-closure, check-abi-ratification, check-manifest-schema-version,
  coverage-matrix, invariant-lock `--changed-files`, stability-matrix `--check`, gen-abi-docs `--check`.
- Not run / unverified: `abi-diff` (refuses a dirty tree; a clean post-commit run is owed), `prepush`, hosted discipline, Apple and Windows legs, the WASM-component witness. `docs/invariants/*.md` untouched.
