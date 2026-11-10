# 20-14 L1 correction round 2 — evidence

Scope: independent reviews of commit `2ae11192`: OpenAI `correction-review.md` (CR1 High, CR2 High, CR3 Medium) and GLM `Sec2014Corr` (F-1 Medium, F-2 Low).
Orchestrator rulings applied: CR1 kind 28 is **not** exempt (ADR-045 has no blanket D5; the zero-principal-nexus note is `ModelProvenancePayload`-only,
`governance.rs:117-126`): value-only recursive scrub in discovery, replacement and the late-window certification predicate; CR2/F-1 kind-29 frames
selected by a whole-token value match in `principal_id`, `task_ref`, `session_principal`, `provider` or `model` also have provider/model values scrubbed;
CR3 backslash escaping; GLM F-2 bidirectional-format escaping (ruled yes by Main). Base: tree clean at `2ae11192`.

The round-1 claim "governance events carry no principal nexus (ADR-045 D5)" in the story and `corrections.md` was **false** and is withdrawn there.

## Design (one function)

`TransparencyLogAdapter::scrub(kind, value, principals) -> bool` (`maos-iac/src/adapter/transparency_log.rs`) replaces matching string values with `[REDACTED]`
and never touches object keys. Kind 29 traverses only the keys `principal principal_id task_ref session_principal provider model` (so `kind`/`Resolved` tags,
`dimensions`, `schema_version`, `attribution_source` are untouched). Kind 28 traverses every nested value except string values under fixed receipt fields
(`event kernel_verdict trusted_intent trusted_origin`), hashes (`*hash`, `sha256`, `supersedes`). `subject_in_frame` (scan filter and certification) is
`scrub` on a throwaway parse, so discovery, replacement and certification cannot diverge; unparseable kind-28/29 bodies keep the old whole-body test and the tombstone.
Non-28/29 kinds are not parsed.

Accepted trade-offs (documented in ADR-046 §8 and the three guides): a principal equal to a real provider/model name erases that value in every frame carrying it;
a principal equal to a kind-28 value that is not a fixed receipt field (for example a unit-variant tag inside quoted claims such as `TaskAssign`) over-erases that value.
The existing F2 fixture values changed from `provider:"model"`/`model:"provider"` to `anthropic`/`claude-x`: key-name values now legitimately match as values; the F2 assertions are unchanged.

## Patch / witness table

Red = observed on the unchanged `2ae11192` source with only the new witnesses added (`correction-round2/red/*.log`, raw output with command and rc).
Plants = mutant of the final code, observed red, restored by copy-back, `cmp` (identical) and `touch` (`correction-round2/plants/*.log`, raw output; `plant-summary.jsonl`).

| Patch | Failing-before witness (red log) | Plant (raw log) | Final |
|---|---|---|---|
| CR1 kinds 28 value scrub (`scrub`, `subject_in_frame`, `redacted_principals_body`, certification) | `forget_cascade_18_6::governance_receipts_are_value_scrubbed_with_dependent_closure`; `erasure_races_20_11::review_round2_late_governance_receipt_is_erased_before_certification`; real T2 session `spawned_scope_17_3c::forget_erases_a_real_authorization_receipt_that_names_the_principal` (`red/kernel-forget_cascade.log`, `red/iac-late.log`, `red/bin-real-session.log`) | CR1a kind-28 values not traversed; CR1c selection never matches; CR1d replacement skips scrub; CR1e certification ignores kinds 28/29 | green |
| CR1 fixed discriminators/hashes | guard witness `governance_fixed_discriminators_and_hashes_never_select_a_frame` (green before: it guards against over-erasure) | CR1b fixed fields and hashes traversed | green |
| CR2/F-1 kind-29 value labels | `cost_erasure_scrubs_subject_bearing_values_and_keeps_the_frame_parseable`, `a_principal_equal_to_a_provider_name_erases_that_value_in_every_frame_that_carries_it`, `review_round2_late_cost_label_value_is_erased_before_certification` | CR2a provider/model not scrubbed | green |
| F2 keep (schema keys/tags safe) | existing `cost_erasure_never_scrubs_schema_keys_or_unrelated_frames` (now 11 subjects) | F2x kind-29 traverses every value | green |
| CR3 backslash escape | `cost_report_review_20_14::plain_escape_is_injective_for_backslashes_and_hides_bidi_overrides` (`red/cli-escape.log`) | CR3a backslash not escaped | green |
| GLM F-2 bidi/format escape | same test | F2-GLM bidi ranges not escaped | green |
| CR3 key grammar pin | `names_and_fractions_...` pins `{"1:7":1.0,"2:7":1.0,"3:7":0.0}` (green before: contract pin) | CR3b key grammar changed | green |
| docs | `cost_docs_20_14` now requires `per_spirit_attributable_fraction`, `<boot_nonce>:<pid>`, `[REDACTED]` in all three guides | n/a | green |

The real-session witness runs a T2 `hostile-twin` provenance session whose operator seed carries the goal `review alice profile` and
`MemRead{principal:alice:profile}`; the real `spirit.frame.authorization` receipt (kind 28, pid of the admitted Spirit) contains both strings before `forget(alice)`
and neither after, a distillate citing it is redacted, and a Bob copy of the receipt (alice replaced by bob) is byte-identical.
(Other kind-28 frames do not exist at that pid, hence the Bob copy rather than a second real run.)

## Pins versus ceilings

| Crate | Before | After | Ceiling |
|---|---:|---:|---:|
| maos-cli | 6599 | 6603 (+4) | 6610 (orchestrator-adjudicated overdraft, disclosed to the operator) |
| maos-iac | 8728 | 8726 (-2) | 8728 |
| maos-audit | 8227 | 8227 | 8237 |
| maos-domain / bin | 10203 / 36234 | unchanged | 10215 / 36475 |
| kernel | 26841 / 21042 | unchanged | untouched |
| aggregate | 199146 | 199148, hardfail 199149 | 199378 |

## Verification

See the final section appended below.

### Observed (env `-u CI -u GITHUB_ACTIONS`; xtask rebuilt from this tree first: the shared gate binary had been rebuilt from another tree at 23:16 and its fixture builder failed on a stray empty `guests/lifecycle-probe/`)

- Focused green (`correction-round2/green/*.log`): `forget_cascade_18_6` 19 passed; `maos-iac --lib erasure_races_20_11` 16 passed; `maos-cli --lib cost_report` 8 passed;
  `maos-audit --test spirit_resolution` 4 passed; `maos-cli --test cost_docs_20_14` 2 passed; `maos-bin --features network,wasm-host --test spawned_scope_17_3c forget_erases` 1 passed
  (real T2 session); `maos-bin --no-default-features --test verb_table_15_3` 7 passed.
- `cargo clippy --workspace --all-targets --locked` rc 0, `clippy-ratchet` rc 0 (1325 occurrences, 866 source-distinct, equal to the previous baseline; one new
  `needless_character_iteration` in the new test was fixed first).
- Fixture builder rc 0, `cargo build --locked -p maos-bin -p maos-cli` rc 0, metadata reuse, one workspace nextest (`workspace-nextest-run1.log`): **rc 100, 5897 run, 5894 passed,
  3 failed (94.2 s)**: the known 20-11 1M timing case (not retried; owner `20-7`, `deferred-work.md` row 59) and two verifier tests that failed only because the scratch python venv
  with `cryptography` was not on PATH for that run; both passed on a targeted nextest re-run with the venv (2 passed).
- Gates rc 0: `kloc-check` (aggregate 199148), `check-kernel-baseline`, `check-dev-record-completeness`, `check-epic-close-coherence`, `check-exit-commands`, `check-decision-register`,
  `check-env-contract`, `invariant-lock --changed-files`, `check-fkcs`; `cargo fmt --all -- --check` rc 0.
- Not run: `prepush` (3000 s timeouts in every recent attempt; nextest leg + gates cover its legs; no time to spare); hosted, Apple, Windows legs; T0 items. Round-1 plants (`corrections.md`)
  were not re-run with retained logs; this round's plants all have raw logs.
