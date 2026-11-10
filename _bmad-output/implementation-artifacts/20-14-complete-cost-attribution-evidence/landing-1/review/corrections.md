# 20-14 L1 review corrections — evidence

Scope: `review/triage.md` P1–P4 and `review/security.md` F1–F8 (reviewers: gpt/glm families). Base `a44c76d9` (HEAD, pushed). The
orchestrator rulings applied: F1 identity rows only (`kind==7 AND intent=='lifecycle.load' AND capability_token IS NULL`, or kind 19, at/before the cost
frame's timestamp); F2 scrub by path; F3 escape + cap; F4 origin=Kernel, row pid == payload pid, distinct labels, task_ref cap; F5 `(boot_nonce,pid)`
fractions; F6 unpriced warning; F7 checked/saturating arithmetic; F8 wording. The D02 A CLI overdraft is an
**orchestrator-adjudicated overdraft, disclosed to the operator**: CLI ceiling 6545 -> 6610.

## Patch / witness table

Every plant was applied to the file shown, run, observed red, then restored by `cp` of the pre-plant copy, `cmp` against it (identical) and `touch`.
Harness: one exact-once replacement per plant, backup under `/tmp/maos2014d-bak`, command in the third column. Cargo was the only process.

| Patch (file: symbol) | Plant (before / mutant) | Failing witness (observed red, exit 101) | Final |
|---|---|---|---|
| P1/F1 `maos-audit/src/lib.rs`: `spirit_names` | F1a: old `WHERE kind IN (7,19) AND intent != 'lifecycle.journal'` | `spirit_resolution::cost_spirit_names_distinguish_boots_and_skip_echoes` | green |
| P1/F1 same | F1b: drop `capability_token IS NULL` | same test (token-bearing fake load `spoof`) | green |
| F4 `maos-audit/src/lib.rs`: `AuditFilter.origin` | `origin != ?` | `spirit_resolution::cost_query_origin_filter_keeps_only_kernel_frames` | green |
| P2/F1 `maos-cli/src/subcommands.rs`: `build_cost_report` name lookup | P2a: unnamed fallback `"unknown"` | `cost_report_review_20_14::names_and_fractions_are_boot_local_and_unnamed_falls_back` | green |
| P2 same | P2b: lookup ignores `boot_nonce` | same test and `tests::cost_report_separates_task_source_and_boot_names` | green |
| F1 same | P2c: name not bounded by frame timestamp | `names_and_fractions_...` (a later `future` identity wins) | green |
| F3 `cost_terminal_field` | F3a: control characters not escaped | `cost_report_review_20_14::plain_report_escapes_control_characters_and_caps_fields` | green |
| F3 same | F3b: length cap removed | same test | green |
| F4 `build_cost_report` | F4a: pid mismatch accepted | `report_rejects_pid_mismatch_and_distinguishes_absent_task` | green |
| F4 same | F4b: schema-1 absent task labelled like schema 2 | same test + `tests::cost_report_reads_payload_not_intent` | green |
| F4 same | F4d: task_ref cap 256 -> 256000 (re-observed against the final non-vacuous assertion) | same test | green |
| F5 `per_spirit_attributable_fraction` | keyed by pid only | `names_and_fractions_...` | green |
| F6 unpriced warning | warning removed | `cost_report_review_20_14::unpriced_rows_warn` (two rows, one deduplicated warning) | green |
| F7 totals | F7a: plain `+=` in the total accumulation | `extreme_costs_do_not_silently_wrap` (debug overflow panic) | green |
| F7 allocation | F7b: plain `*` in the last-row split | same test | green |
| F7 narrowing | F7c: `as u64` instead of saturating `try_from`; first run stayed green, the test was strengthened (`total_cost_micro == u64::MAX`) and re-observed red | same test | green |
| F2 `maos-iac/.../transparency_log.rs`: `subject_in_frame` (selection) | F2a: cost frames matched by raw `principal_id_occurs` | `forget_cascade_18_6::cost_erasure_never_scrubs_schema_keys_or_unrelated_frames` and `erasure_races_20_11::review_cost_schema_token_does_not_block_late_certification` | green |
| F2 `redacted_principals_body` (scrub) | F2b: original recursive key-deleting / value-replacing scrub | `cost_erasure_never_scrubs_schema_keys_or_unrelated_frames` | green |
| F2 `erasure_sweep.rs`: certification | F2c: certification uses raw occurrence | `erasure_races_20_11::review_cost_schema_token_does_not_block_late_certification` (a late cost frame holding the key `provider` keeps certification pending to the 8-pass cap) | green |
| P3 ADR-046 §10 | SR-1 sentence deleted | `cost_docs_20_14::adr_retains_write_path_authorization_debt` | green |
| P4 records | no runnable witness: record comparison (decision row carried an unsupported calendar date; story and tracker said ready-for-dev) | n/a | date removed, `in-progress` set, T0/T4/L2 left open |

Two earlier witness drafts were wrong and were replaced before the table rows above were taken: the first certification test never ran its hook
(captured `bool` copied into the closure; replaced by an `AtomicBool` and an explicit `assert!(hook ran)`), then used a subject (`tokens`) that
`principal_id_occurs` does not match inside `tokens_in`; the final subject is the exact schema key `provider`. The long-task assertion was vacuous
(`all` over an empty row list) and now asserts the frame is skipped with one warning.

## Reader contracts changed (disclosed)

- `maos_audit::spirit_names` returns `BTreeMap<(boot,pid), Vec<(timestamp_ns, name)>>` (type alias `SpiritNames`); the report takes the latest identity
  at or before the frame's timestamp, else `pid:<n>`. Only `cli` consumes it.
- `per_spirit_attributable_fraction` is now keyed `"<boot_nonce>:<pid>"` (was `<pid>`). This closes review defer D1 (recycled pids pooled); NDJSON
  consumers of that field must read the new key.
- Absent task renders `none (schema 1)` / `none (schema 2)`; a literal task that equals one of those (or starts `literal:`) renders `literal:<task>`.
- Frames with `task_ref` over 256 bytes or a row pid different from the payload pid are skipped with a warning; the query keeps only `origin=Kernel`.
- **[WITHDRAWN in round 2: see `correction-round2.md`]** Round 1 narrowed the forget cascade for kinds 28 and 29 to `/principal/principal_id`, `/task_ref` and
  `/session_principal` and claimed governance events carry no principal nexus (ADR-045 D5). That claim was false (D5's note is `ModelProvenancePayload`-only,
  `governance.rs:117-126`; spawned-frame authorization receipts quote guest frames) and the path-only scrub under-erased kind 28 and cost labels.

## Pins versus ceilings (exact, `cargo fmt --all` then `kloc-check --json`)

| Crate | Before | After (pin) | Ceiling | Note |
|---|---:|---:|---:|---|
| maos-cli | 6529 | 6599 (+70) | 6610 (was 6545) | orchestrator-adjudicated overdraft, disclosed to the operator (D02 A): 6545 -> 6610; +11 under the new ceiling |
| maos-audit | 8209 | 8227 (+18) | 8237 | inside the rebased ceiling |
| maos-iac | 8728 | 8728 | 8728 | unchanged (test-support and `#[cfg(test)]` files are not src) |
| maos-domain | 10203 | 10203 | 10215 (standing overdraft) | unchanged |
| maos-bin | 36234 | 36234 | 36475 | unchanged |
| kernel | 26841 / 21042 | 26841 / 21042 | untouched | `check-kernel-baseline` 26841 = pin, set-hash equal |
| aggregate | 199058 | 199146 (hardfail 199147) | 199378 | `kloc-check --json` rc 0 |

## Verification observed (commands exact; env `-u CI -u GITHUB_ACTIONS`)

- `cargo test -p maos-audit --test spirit_resolution` 4 passed; `cargo test -p maos-cli --lib cost_report` 7 passed; `cargo test -p maos-cli --test cost_docs_20_14`
  2 passed; `cargo test -p maos-iac --lib erasure_races_20_11` 14 passed; `cargo test -p maos-kernel-core --test forget_cascade_18_6` 15 passed;
  `cargo test -p maos-bin --no-default-features --test verb_table_15_3` 7 passed (all rc 0).
- Fixture builder (`xtask build-test-fixtures`) rc 0; `cargo build --locked -p maos-bin -p maos-cli` rc 0; `cargo metadata` and
  `cargo nextest list --list-type binaries-only` rc 0; one workspace `cargo nextest run --no-fail-fast --cargo-metadata … --binaries-metadata …`
  (default profile, python `cryptography` 50.0.2 in a scratch venv on PATH): **rc 100, 5890 run, 5889 passed, 1 failed** (93.2 s):
  `maos-bin::erasure_million_20_11 ac5_million_row_pending_forget_finishes_and_repeats_without_corruption` (pending 10.00012028 s, case 42.142 s), the
  existing 20-11/20-7 timing occurrence; not retried, recorded under owner `20-7` with same-line evidence in `deferred-work.md` row 58 and
  `corrections-workspace-nextest.log:5815-5818,5897-5920`.
- `cargo clippy --workspace --all-targets --locked --message-format=json` rc 0, then `xtask clippy-ratchet` rc 0 (1325 occurrences, 866 source-distinct);
  no diagnostic points at the new or changed lines (the listed warnings in the touched files predate this change).
- Gates (prebuilt `/mnt/build/cargo-maos/debug/xtask <gate> --json`): `kloc-check` 0, `check-kernel-baseline` 0, `check-dev-record-completeness` 0,
  `check-epic-close-coherence` 0, `check-exit-commands` 0, `check-decision-register` 0, `check-env-contract` 0, `invariant-lock --changed-files` 0, `check-fkcs` 0
  (own cell). The dev-record and epic-close gates were re-run after the final record edits.
- Not run: `prepush` (it has timed out at 3000 s in every recent attempt; the workspace nextest leg and the gates above cover its legs and it is not retried);
  hosted discipline, Apple and Windows legs; the T0 items of the story (wrapper kernel delta, mint inventory, seam identity, AC1 timing) remain unmeasured.

## Claims boundary

L1 only: the reader, the report, the identity query, the forget scrub and the documentation. AC1–AC3 and AC6 closure belongs to the kernel landing
(root writer, binding, command); current writers still emit schema 1 with the proxy source. Status `in-progress`; no task ticked.

## Cleanup

Removed own scratch (`/tmp/maos2014d-*` venv, plant backups, logs, clippy stream, and `/tmp/maos2014-tl.rs`): 40,247,296 allocated bytes. Incremental pruning
(sessions that are not the newest per crate directory, untouched for more than 3 h, with non-blocking `flock`/`lockf` probes): 0 sessions eligible (46
sessions younger than 3 h, 0 locked), 0 bytes. `/proc/1/fd` and `/proc/7/fd` were not readable; only the lock probes and mtime were relied on. Nothing under
deps, build or fingerprint was touched.
