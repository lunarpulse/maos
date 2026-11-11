Independent adversarial correction review of fc7373f0 (OpenAI gpt reviewer; persisted by the orchestrator because the reviewer was read-only)

**Counts: 0 new actionable code findings. R2-1/R2-2 close; F4’s universal escape-coverage claim does not.**

**Persistence blocker:** the developer’s explicit read-only/no-file-edit restriction prevented writing `correction-review-r3.md`. Report content follows for the main agent to persist.

# Independent adversarial correction review — fc7373f0

## Verdict and execution boundary

**Overall correctness: correct. Confidence: 0.94.** No provable, unintended, patch-introduced code defect was established. R2-1/R2-2 and the escaped-discovery correction are substantive; complete Unicode-format coverage and universal exemption authenticity must not be inferred.

Reviewed committed sources using `git show`, not the concurrently changing working tree. No builds, tests, plants, formatters, measurements or git mutations executed. Outcomes below are observed in retained author logs, not independently reproduced.

## Typed governance and consuming decoder

`governance.rs` contains **two enums** relevant here: internally tagged `GovernanceEventKind` and snake-case `RatificationStatus`. Its payload structs contain no additional nested enums; `Option<String>`, vectors and scalar fields introduce no discriminator.

| GovernanceEventKind variant | Discriminators and treatment |
|---|---|
| `AbiExtension` | `/event/governance_type` preserved. Nested `RatificationStatus` values `proposed`, `ratified`, `rejected` preserved only at `/event/status` under this subtype. Other strings remain erasable. |
| `VetterKey` | Tag preserved. `effective_tier` is a String, not an enum. Boolean `admitted` is unaffected. |
| `SchemaLifecycle` | Tag preserved. No nested enum. Canonical 64-hex `schema_content_hash`/`supersedes` survive; noncanonical strings remain erasable. |
| `ModelProvenance` | Tag preserved. No nested enum. Canonical content hash survives; other string fields are not discriminator exemptions. |
| `TelemetrySubscription` | Tag preserved. No nested enum. `class`, `scope`, `reason` are Strings; numeric/boolean fields are unaffected. |
| `ManifestDeclaration` | Tag preserved. Canonical colon-hex fingerprint and publisher preserved. Nullable publisher needs no string exemption. `outcome` is a String, not an enum. |

The six tag names exactly match the current serialized enum. `scrub` at `transparency_log.rs:2278–2301` temporarily removes approved values, traverses the remainder, then restores them. Recursion calls `walk`, not `scrub`, so nested lookalike objects do not acquire these root-envelope exemptions.

The actual receiving dispatch, `maos-audit/src/lib.rs:557–610`, skips only recognized receipt intents, deserializes every remaining kind-28 row as `GovernanceEventPayload`, and returns only ratified ABI proposals. Successfully decoded other variants are intentionally ignored, not accidentally dropped.

**R2-1 closed:** recognized tags and all three ABI statuses survive; ordinary personal strings remain erasable. The retained tests exercise actual emitters and `load_ratification_frames`, with byte-identity controls.

**R2-2 closed:** production `manifest_sha256` and publisher encodings survive, including a forgotten principal equal to a fingerprint group. The personal declaration-field test requires only `spirit_id` plus redaction metadata to change.

**Coverage qualification:** executable discriminator-collision tests cover three variants and `ratified`, not all six variants and all three statuses. The remaining cases are established by source inspection, not equivalent retained execution.

## Exemption binding and hostile values

The new preservation is location-, subtype- and value-bound, rather than a recursive exemption for any field named `status`, `publisher` or `governance_type`.

- Arbitrary `alice` at `/event/publisher` fails `decode_declaration_bytes` and remains erasable.
- A principal named `ratified` or `VetterKey` can equal a legitimate structural value. Preserving that structural occurrence while erasing occurrences in free text is the intended distinction.
- A canonical colon-hex string can contain a principal such as `face` as one group. **Encoding validation alone does not authenticate its meaning.** The helper checks neither the full typed envelope nor row origin/intent.
- On the traced production declaration path, however, the fingerprint is computed from manifest bytes and publisher bytes originate from package verification. `GatedManifest::bind_registry_package_at` checks package/snapshot agreement; registry verification validates the publisher signature and deployed bytes. Guest authorization claims are nested beneath a fixed receipt envelope and cannot populate root `/event/publisher`.

Thus a fabricated raw adapter row can demonstrate that the helper is shape-based, but no current Spirit-controlled production route to an arbitrary preserved root publisher string was established. Reporting that as a reachable new exploit would require an unstated assumption.

The recursive fixed-receipt/digest-key exemptions are still not general provenance validation. Do not advertise them as such.

## Discovery, scan cost and bounded certification

The SQL change at `transparency_log.rs:2001` lets **every kind-28/29 row in the finite row window** reach decoded selection. Other kinds retain the raw `instr` prefilter.

Discovery, replacement and tail certification route through the same changed predicate. `write_erasure_body` explicitly handles kinds 28/29; no dispatch omission was found.

Boundaries remain:

- `scan_rowid_range` processes at most **4,096 actual rows per mutex window** and releases the mutex between windows.
- Each sweep snapshots a finite high-water mark.
- Final certification uses an immediate SQLite writer transaction and `LIMIT 4097`; more than 4,096 tail rows returns pending.
- Matching-tail retries remain capped; exhausting the cap does not publish completion.

**Cost qualification:** all eligible kind-28/29 bodies are now materialized and JSON-decoded even without literal subject bytes. Work increases with their total payload size, repeated per subject. Row boundedness is not a byte or wall-time bound. The retained million-row fixture seeds kind-1 filler, so it does not establish dense governance/cost performance.

The escaped initial-snapshot and one-shot late-hook witnesses substantively cover the corrected omission. The late hook asserts it executed.

## Schema-lifecycle ingress

`register_schema_lifecycle` at `transparency_log.rs:3122–3131` now rejects content hashes and supplied predecessor hashes unless they are exactly 64 ASCII hex digits. Uppercase hex is accepted. Rejection happens before registry/frame writes.

The error is **`AuditError::Serialization("schema digests must be 64 hex digits")`**, not a panic or a dedicated validation variant.

Caller tracing:

- Operator `AdmitGovernanceSchema` reaches the adapter and maps rejection to `OperatorOutcome::Failed`, code `governance_admit_failed`.
- CLI `GovernanceOp::Admit` forwards the supplied strings unchanged through the door or offline environment channel.
- Offline `governance-admit` propagates the adapter error.
- The ABI-extension branch returns before schema registration and is unaffected.
- CLI help already specifies SHA-256 hex; the stricter validation does not reject that documented format.
- The unwind fixture was changed to a canonical digest so it still reaches the intended injected SQLite failure.

Previously accepted abbreviated/arbitrary hashes now fail deliberately. Canonical Admit requests retain their flow. The added ingress witness proves adapter behavior, **not an executed CLI/door admission round trip**; the green verb-table suite proves dispatch-table integrity, not that stronger behavior.

## Escape-table completeness: F4 remains partial

`maos-cli/src/subcommands.rs:5051–5069` preserves backslash injectivity, controls, previous bidi ranges, and adds the specifically tested format/tag/selector characters.

However, **“escapes the Cf class … and variation selectors” is overbroad**. The new table omits:

- Cf: U+110BD, U+110CD;
- Cf: U+13430–1343F;
- Cf: U+1BCA0–1BCA3;
- Cf: U+1D173–1D17A;
- Variation selectors: U+180B–180D and U+180F.

These classifications are explicit in Unicode’s [UnicodeData](https://www.unicode.org/Public/17.0.0/ucd/UnicodeData.txt) and [PropList](https://www.unicode.org/Public/17.0.0/ucd/PropList.txt).

These characters also passed through the parent implementation, so this is **not counted as a newly introduced code regression**. It prevents declaring universal F4 closure.

**Exact completion:** include those ranges, or narrow the closure claims to the actual table. **Witness needed for universal closure:** assert escaping for every Cf and Variation_Selector code point in a pinned Unicode dataset, retaining readable-text and literal-backslash controls. The current 20 examples and two mutants cannot establish table completeness.

## Retained logs and twelve plants

The twelve summary records have corresponding raw failure logs:

| Plants | Substantive failure |
|---|---|
| R2-1a / R2-1b | Byte-identity assertion fails for `VetterKey` / `ratified`. |
| R2-2a / R2-2b | Fingerprint/publisher collision assertion fails for the actual fingerprint group / `9a9a`. |
| R2-a1 | Initial escaped-body assertion and late exact-frame-id assertion fail. |
| R2-b1 | Noncanonical hash remains; ingress assertion also fails because this mutant affects the shared predicate. |
| R2-b2 | Noncanonical ingress acceptance assertion fails independently. |
| CR1a / CR1c / CR1d | Rebuilt real-session legs reach `spawned_scope_17_3c.rs:3263`, showing retained Alice in the actual authorization receipt. |
| F4 control-only / reduced table | Escape assertion fails on U+00AD. |

The three formerly stale-fixture session legs now reach the intended erasure assertion. This closes that specific evidence objection.

All plant headers report rc 101 and restored/cmp identity; summary durations are present. **Exact mutant diffs, copy-back/cmp execution and touch execution are not independently demonstrated by those headers.** Semantic failure output is inspectable; restoration/provenance remains author-attested. No inspected evidence establishes fabrication.

Focused green logs record: kernel 22, iac 17, CLI report 8, CLI docs 2, typed governance 4, real session 1, verb table 7, audit Spirit resolution 4 passing tests.

## Claims honesty, ceilings and limits

- Round-2 false tag/hash and cannot-diverge statements are explicitly corrected.
- Case/normalization sensitivity, Spirit-name-column retention and whole-value label policy are disclosed rather than represented as solved.
- The universal Cf/variation-selector statement still needs narrowing or completion.
- Workspace evidence honestly remains **non-green**: **5,905 run, 5,904 passed, one failed, 137 skipped; 94.801 s; rc 100**. The million-row case lasted 44.081 s and failed waiting for pending completion. No retry was performed here; its owner attribution was not independently re-proved.
- Workspace log headers record fmt/clippy/ratchet statuses and counts. The additional listed gate successes and exact size measurements lack dedicated raw gate output in this correction directory; treat them as author-reported.
- Prepush, hosted, Apple/Windows and T0 remain unproved.

Committed arithmetic is coherent:

- **iac 8,770 ≤ adjudicated 8,770**, zero headroom;
- **CLI 6,609 ≤ 6,610**, one line remaining;
- aggregate **199,148 + 44 + 6 = 199,198**;
- hardfail **199,199**, below recorded ceiling **199,378**;
- audit/domain/bin/kernel pins unchanged.

These are inspected ledger values, not fresh reviewer measurements.

**Final headlines: R2-1/R2-2 closed; escaped discovery corrected with finite certification retained; no demonstrated Admit regression; twelve substantive mutant failures retained; F4 universal coverage still unproved.**
