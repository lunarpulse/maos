# 20-14 L1 correction round 3 — evidence

Scope: GPT `correction-review-r2.md` (R2-1 High, R2-2 Medium, inherited limits (a) escaped-value discovery and (b) exemption validity) and GLM `Sec2014R2`
(F-1..F-4) on commit `553336f6`. Base here: `29f9a3ea` (= 553336f6 + review notes). Rulings applied: R2-1/R2-2 preserve typed governance fields by envelope **and**
value form (no key-name exemption); inherited (a)/(b) closed; F-4 escape table; F-1/F-3 deferred as operator-visible privacy limits, F-2 accepted policy.

## Design changes (`maos-iac/src/adapter/transparency_log.rs`)

- `scrub` (shared by `subject_in_frame` = discovery and certification, and by `redacted_principals_body` = replacement) sets aside, then restores, the kind-28 typed fields
  `/event/governance_type` (valid `GovernanceEventKind` variant), `/event/status` (valid `RatificationStatus`, only under `AbiExtension`), `/event/manifest_sha256` and
  `/event/publisher` (canonical colon-hex per `decode_declaration_bytes`, only under `ManifestDeclaration`); `walk` does the value-only recursion. No key-name exemption for
  `status`/`governance_type`.
- In `walk` the exempt keys `sha256`, `supersedes`, `schema_content_hash` are exempt only when the value is a 64-hex digest; non-canonical values there are erased.
  The old `*hash` suffix heuristic is gone. `register_schema_lifecycle` (the single ingress of the operator door and `maos governance-admit`) rejects non-canonical digests.
- Discovery SQL: `(kind IN (28,29) OR instr(payload_redacted, ...) > 0)`, so escaped spellings reach decoded selection; other kinds keep the raw prefilter and the bounded window scan.
- Corrected statements: round 2's "keys, tags and hashes never change" and "discovery, replacement and certification cannot diverge" were false (see `correction-round2.md`).
- `maos-cli` `cost_terminal_field` escapes the code points of its `INVISIBLE` table (Cf and variation-selector entries incl. U+061C, U+180E, U+FEFF, tag characters, U+FE0F, U+E0100) besides controls, backslash and the round-2 ranges; it does **not** escape every Cf or Variation_Selector code point (U+110BD, U+110CD, U+13430–1343F, U+1BCA0–1BCA3, U+1D173–1D17A, U+180B–180D, U+180F; pre-existing, deferred).

## Patch / witness table

Red = observed on the unchanged `553336f6` source with only the round-3 witnesses added (`correction-round3/red/*.log`, raw output with command and rc). Plants = mutants of the final
code, observed red, restored by copy-back, `cmp` (identical) and `touch` (`correction-round3/plants/*.log`, raw output; `plant-summary.jsonl`).

| Patch | Failing-before witness (red log) | Plant (raw log) |
|---|---|---|
| R2-1 typed `governance_type` tag | `forget_governance_20_14::typed_governance_survives_a_forget_of_its_own_discriminators` (real `emit_vetter_key_event` / `emit_abi_extension_event` / `emit_manifest_declaration_event`, consumer `load_ratification_frames`; subjects `VetterKey`, `AbiExtension`, `ManifestDeclaration`, `ratified`; other Spirit + dependent distillate intact) (`red/bin-governance.log`) | R2-1a tag not preserved |
| R2-1 ABI `status` | same test (subject `ratified`) | R2-1b status not preserved |
| R2-2 `manifest_sha256`, `publisher` | `a_fingerprint_group_as_the_forget_subject_keeps_the_manifest_fingerprint` (four-hex group of the real fingerprint, and `9a9a` of the publisher key; `decode_declaration_bytes` recovers) (`red/bin-governance.log`) | R2-2a, R2-2b |
| R2-1/R2-2 guards (green before: they guard erasure and single-field change) | `personal_values_are_erased_while_the_typed_envelope_stays_decodable` (Alice summary and journal note erased, envelope and ratification still decode, Bob Spirit event and dependent distillate); `a_personal_declaration_field_changes_alone_beside_the_same_fingerprint` | plants R2-1a/R2-2a also fail these |
| Inherited (a): escaped spellings, kinds 28/29 | `forget_cascade_18_6::escaped_spellings_reach_decoded_selection_in_kinds_28_and_29` (`ali"ce`, `\u0061lice`, both kinds, dependent distillate, control); `erasure_races_20_11::review_round3_late_escaped_spellings_are_erased_before_certification` (one-shot late hook) (`red/kernel-forget_cascade.log`, `red/iac-late.log`) | R2-a1 raw prefilter applies to 28/29 |
| Inherited (b): digest form + ingress | `exempt_hash_fields_are_exempt_only_in_canonical_digest_form`, `schema_lifecycle_ingress_rejects_noncanonical_digests` (`red/kernel-forget_cascade.log`) | R2-b1 digest form unchecked; R2-b2 ingress accepts non-canonical |
| GLM F-4 escape table | `plain_escape_is_injective_for_backslashes_and_hides_bidi_overrides` (20 code points incl. U+00AD, U+061C, U+180E, U+FEFF, FE0F, E0100; readable text control) (`red/cli-escape.log`) | F-4 control-only; F-4 table reduced to round-2 ranges |
| Round-2 real-session legs (re-run after rebuilding `maosctl` so the receipt assertion fails, no stale-fixture failure) | `spawned_scope_17_3c::forget_erases_a_real_authorization_receipt_that_names_the_principal` | CR1a, CR1c, CR1d (each log fails at `spawned_scope_17_3c.rs:3263`, the post-forget `!after.contains("alice")`) |

Fixture changes: kernel hash fixtures `deadbeef`/`cafe` are now 64-hex digests (a non-canonical value is no longer exempt); the kind-28 ABI fixture is the real serialized
`GovernanceEventPayload` (internally tagged), not the externally tagged lookalike; the `erasure_races_20_11` unwind fixture uses a canonical digest because ingress now validates.

## Pins versus ceilings

| Crate | Before | After | Ceiling |
|---|---:|---:|---:|
| maos-iac | 8726 | 8770 (+44) | 8770 (orchestrator-adjudicated D02 A overdraft, disclosed to the operator; was 8728) |
| maos-cli | 6603 | 6609 (+6) | 6610 |
| maos-audit | 8227 | 8227 | 8237 |
| maos-domain / bin | 10203 / 36234 | unchanged | 10215 / 36475 |
| kernel | 26841 / 21042 | unchanged | untouched |
| aggregate | 199148 | 199198, hardfail 199199 | 199378 |

## Operator-visible privacy limits (deferred, `deferred-work.md` rows under "correction round 3")

Matcher is case/zero-width/normalization-sensitive for all frame kinds (GLM F-1); Spirit-name columns are outside erasure (GLM F-3); whole-value label replacement is accepted policy (GLM F-2).
Documented in ADR-046 §8 and the three guides.

## Verification observed (env `-u CI -u GITHUB_ACTIONS`, python `cryptography` venv on PATH; `xtask` rebuilt from this tree, fixtures rebuilt, `maosctl`/`maos` rebuilt before every real-session leg)

- Focused green (`correction-round3/green/*.log`): `forget_cascade_18_6` 22 passed; `maos-iac --lib erasure_races_20_11` 17 passed; `maos-cli --lib cost_report` 8 passed; `maos-cli --test cost_docs_20_14` 2 passed;
  `maos-bin --features network --test forget_governance_20_14` 4 passed; `maos-bin --features network,wasm-host --test spawned_scope_17_3c forget_erases` 1 passed (real T2 session);
  `maos-bin --no-default-features --test verb_table_15_3` 7 passed.
- `cargo clippy --workspace --all-targets --locked` rc 0 and `clippy-ratchet` rc 0 at 1325 occurrences / 866 source-distinct (equal to baseline; two new lints, `dropping_copy_types` in the iac restore and an unused
  import in the new bin test, were fixed after the first run showed 1328).
- One workspace nextest on the final tree (`workspace-nextest-final.log`, metadata reuse): **rc 100, 5905 run, 5904 passed, 1 failed (94.8 s)**: the known 20-11 1M timing case
  (`erasure_million_20_11`, 44.081 s, not retried; owner `20-7`, `deferred-work.md` row 60). Nothing else failed; the cryptography venv was on PATH.
- Gates rc 0: `kloc-check` (iac 8770, cli 6609, aggregate 199198), `check-kernel-baseline`, `check-dev-record-completeness`, `check-epic-close-coherence`, `check-exit-commands`, `check-decision-register`,
  `check-env-contract`, `check-fkcs` (own cell), `invariant-lock --changed-files`; `cargo fmt --all -- --check` rc 0.
- Not run: `prepush` (3000 s timeouts in every recent attempt; nextest leg + gates cover its legs), hosted/Apple/Windows legs, T0 items.
