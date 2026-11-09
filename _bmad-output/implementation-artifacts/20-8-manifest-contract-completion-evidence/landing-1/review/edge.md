# 20-8 landing-1 (5610f9d6) — edge-case hunter findings

Method: path enumeration over diff hunks plus the code they reach at 5610f9d6 (read-only; nothing was run).
Format: JSON array, four fields per item as requested. Severity guesses, failing-before witnesses and
[INFERENCE] marks are in the "Notes" section below the array, keyed by the item number (1-based).

```json
[
  {
    "location": "crates/maos-iac/src/adapter/distillate.rs:145-168 (admit_from_log); crates/maos-iac/src/adapter.rs:222-262 (check_promotion)",
    "trigger_condition": "Any routed TaskAssign with prior_distillate_ref to a recipient that has a promotion set",
    "guard_snippet": "let entry = log.query_frame_by_id(digest_frame_id).map_err(|e| DistillationError::Storage(e.to_string()))?; match entry { Some(e) if e.kind == FrameKind::Distillate => {...} _ => Err(SourceFrameNotFound{..}) }",
    "potential_consequence": "Full transparency-log scan under the TL mutex per delivery; stalls every I2 writer and the async bus."
  },
  {
    "location": "crates/maos-bin/src/spirit_session.rs:1036-1042 (authorize_and_deliver) with crates/maos-iac/src/adapter.rs:473 (check_promotion)",
    "trigger_condition": "Guest emits TaskAssign whose prior_distillate_ref is outside the recipient's set or unknown",
    "guard_snippet": "match deliver_bounded(..) { Err(SessionError::Mailbox(_)) if is_policy_denial(&e) => { report.denied_frames += 1; revoke(..)?; return Ok(false) } other => other.map(drop) }  // IntentPromotionDenied becomes a counted denial, not session death",
    "potential_consequence": "A normal policy refusal tears down the sender's whole session via fail(), same as a protocol fault."
  },
  {
    "location": "crates/maos-iac/src/adapter.rs:236-251 (check_promotion .is_err() branch)",
    "trigger_condition": "admit_from_log fails for Storage/serde reasons (TL busy, unparseable receipt) or digest id is absent",
    "guard_snippet": "match admit_from_log(..) { Ok(()) => {}, Err(DistillationError::IntentPromotionDenied{..}) => deny(), Err(DistillationError::SourceFrameNotFound{..}) => deny_with_reason(\"digest-not-found\"), Err(e) => return Err(IacBusError::SerializationFailed(e.to_string())) }  // and journal the reason, not just the recipient id",
    "potential_consequence": "Transient read failure is journaled and returned as an intent-lineage denial; the row carries no digest id or cause."
  },
  {
    "location": "crates/maos-bin/src/installed_spirits.rs:25,50 (bind_registry_publisher replaced bind_registry_package_at)",
    "trigger_condition": "Signed package whose signed metadata version differs from [class].version in its manifest",
    "guard_snippet": "let (text, publisher, pkg_version) = verify_installed(..)?; ... if gated.class_section.version != pkg_version { return Err(AdmissionRefusal::Admission{ detail: \"registry manifest snapshot differs from package\".into() }) }",
    "potential_consequence": "Boot no longer refuses the version mismatch that bind_registry_package_at (admission.rs:564-571) refused; the row records the unmatched version."
  },
  {
    "location": "crates/maos-bin/src/admission.rs:1046-1069 (check_declared_skills)",
    "trigger_condition": "Queue store load, TL approvals query, discovery merge or NoDataHome fails",
    "guard_snippet": "let store = LocalFsSkillQueueStore::new().map_err(|e| AdmissionRefusal::Admission{ detail: format!(\"skill store unavailable: {e}\") })?;  // likewise for load(), query_approvals(), skill_entries()",
    "potential_consequence": "Infrastructure failure is reported as skill_unavailable naming the first declared skill, misdirecting the operator."
  },
  {
    "location": "crates/maos-bin/src/admission.rs:1050-1088 (check_declared_skills loops)",
    "trigger_condition": "Declared id has a second discovered file (other root/version) that is Pending or requires more capability",
    "guard_snippet": "select the effective discovered item per (id) (highest version that is Admitted) before the Admitted and capability checks, or document that every discovered file with the id must pass; refuse with the offending path in the detail",
    "potential_consequence": "A stray pending or escalated duplicate file blocks a Spirit whose admitted skill is fine; or ambiguity admits the wrong one."
  },
  {
    "location": "crates/maos-bin/src/admission.rs:120-170 (emit_manifest_declaration_event: skill_ids, version) with crates/maos-iac/src/adapter/redaction.rs RULES + TOKEN_HEX_MIN_LEN",
    "trigger_condition": "Declared skill id starts sk-/ghp_/gho_/ya29. or contains 32+ consecutive [0-9a-f] chars",
    "guard_snippet": "in declared_names: reject names matching secret prefixes or hex runs >=32 (shared constant with the redactor), or encode skill_ids (e.g. colon-grouped) the way enc208 did for digest/publisher",
    "potential_consequence": "Validator accepts the id, journal stores <REDACTED:...> instead; record no longer equals the declaration."
  },
  {
    "location": "crates/maos-manifest/src/manifest.rs:4154-4175 (declared_names)",
    "trigger_condition": "Manifest entry is huge or holds control characters/newlines",
    "guard_snippet": "format!(\"invalid or duplicate entry {:?}\", name.chars().take(64).collect::<String>())  // and separate the 'invalid' and 'duplicate' messages",
    "potential_consequence": "Unbounded manifest-controlled text echoed into the refusal, logs and operator output; invalid and duplicate indistinguishable."
  },
  {
    "location": "crates/maos-bin/src/admission.rs:829-851 (fr8 gate) with crates/maos-manifest/src/manifest.rs:357-371 (warn_n_minus_1_degradations)",
    "trigger_condition": "Schema <=6 manifest literally contains [skills]/[intent_promotion_set]/[swap_invariants]/[explanation_shape]",
    "guard_snippet": "if !fr8 { for name in [\"skills\",\"explanation_shape\",\"intent_promotion_set\",\"swap_invariants\"] { if root.get(name).is_some() { tracing::warn!(section=name, schema=class_section.manifest_schema_version, \"declared section ignored below schema 7\"); } } }",
    "potential_consequence": "WARN says 'absent' though the section is present; a Spirit author relying on I13/skills gating is silently unenforced."
  },
  {
    "location": "crates/maos-bin/src/admission.rs:1146-1148 (declaration row) and :1153-1175 (AlreadyLoaded / load error)",
    "trigger_condition": "Two load attempts of one manifest: first admitted, second AlreadyLoaded (or any later refusal)",
    "guard_snippet": "add an attempt nonce (e.g. ulid) to ManifestDeclarationPayload and to the later vetter-rejection reason; or write the pid into a second correlation row once issued",
    "potential_consequence": "Duplicate-attempt rows are byte-identical to the admitted instance's row; no key joins a row to a pid or a refusal."
  },
  {
    "location": "examples/example-spirit-ts/src/index.ts:20-33 and templates/spirit-ts/src/index.ts (author call)",
    "trigger_condition": "Second run while the first authored entry is still Pending, or author_self denied, quota, foreign namespace",
    "guard_snippet": "try { author(doc); } catch (error) { /* ComponentError.payload.tag: duplicate refusals arrive as refused(code) */ if (!isSkillRefusal(error)) throw error; } authored = true;",
    "potential_consequence": "Uncaught WIT result error throws out of handleFrame; the shipped example faults on its first frame after a restart."
  },
  {
    "location": "crates/maos-manifest/src/manifest.rs:4121-4130,4190-4199 (IntentPromotionSetSection) and crates/maos-iac/src/adapter/distillate.rs:300-330 (compute_intent_lineage)",
    "trigger_condition": "Declared allowed entries are not TL intent-column values (routed frames journal high/standard/readonly)",
    "guard_snippet": "document in v7.md that allowed matches the TL intent column (band tokens for routed sources), and change example/template allowed = [\"consult\"] to a value that the distillate lineage can contain",
    "potential_consequence": "Shipped example set denies every distillate built from routed frames; fail-closed but unexplained."
  },
  {
    "location": "_bmad-output/implementation-artifacts/20-8-manifest-contract-completion.md:404,526 (Dev Agent Record file list)",
    "trigger_condition": "i13doc ruling keeps docs/invariants/I13.md untouched; the list names it as changed",
    "guard_snippet": "remove docs/invariants/I13.md from the changed-files list at lines 404 and 526 (git show --stat 5610f9d6 lists no docs/invariants path)",
    "potential_consequence": "Record contradicts the ruling and the commit; reviewers may think invariant-lock was triggered."
  }
]
```

## Notes (severity guess; verification status; failing-before witness)

1. **P2.** Verified: `admit_from_log` calls `log.query_frames(FrameFilter::default())` (no `frame_id`, no `limit`; `frame_query` only adds LIMIT when `filter.limit` is set, transparency_log.rs:2846) and `query_frames` holds `self.inner.lock()` for the whole row build (transparency_log.rs:2588). `query_frame_by_id` already exists and is used by `compute_intent_lineage`. `check_promotion` runs inside the async `deliver_typed` before the I2 write, and also holds a DashMap `Ref` across the call. Witness: seed N=50k TL rows, set a promotion set, time `deliver_typed` of a ref-carrying TaskAssign; compare to `query_frame_by_id`; a concurrent `insert_frame_event` blocks for the scan duration.
2. **P2.** Verified: WIT `task-assign` carries `prior-distillate-ref` (wit/spirit.wit:146, frame_bridge.rs:249), so a guest can send it. `deliver_bounded` maps any bus error to `SessionError::Mailbox`; `authorize_and_deliver` returns `Err(fail(..))`, which ends the loop. Pre-existing errors also did this, but this patch adds a routine policy refusal on the same path (an arbitrary 16-byte digest id suffices). Witness: wasm probe script `send TaskAssign ref=<unknown>` to a set-holding peer; sender session ends with `SessionError::Mailbox("intent promotion denied...")` and `denied_frames` stays 0.
3. **P3.** Verified: `.is_err()` discards the kind; row payload is only the recipient id. Witness: corrupt the Distillate row payload (invalid JSON) and observe `iac:intent-promotion-denied` instead of a storage error.
4. **P3.** Verified: old `bind_registry_package_at` compared `pkg.version != class_section.version` (admission.rs:564-571); installed boot now uses `verify_installed` (returns text + publisher only) and `bind_registry_publisher`. `SignedPackage.version` is a separate field (used by `path_segment(&pkg.version)` in verify_deployed_package). Witness: signed package with metadata version 1.0.0 and manifest `[class] version = "9.9.9"` boots at HEAD, refused at a13140b8.
5. **P3.** Verified from code: four `map_err(|_| unavailable(first))` sites.
6. **P3 / [INFERENCE] on intent.** Verified: `discovered.iter().filter(id)` checks every match; discovery returns files from all three roots with no precedence. Whether "all must pass" is intended is not stated in AC3.
7. **P3.** Verified in redaction.rs: `sk-`, `ghp_`, `gho_`, `ya29.` prefixes (preceded by a non-[A-Za-z0-9_-] byte, e.g. the JSON quote) and `TOKEN_HEX_MIN_LEN = 32`; the marker stays inside the JSON string, so `load_ratification_frames` still parses the row (no poison). `declared_names` charset allows `-`, `_`, `.` and digits, so `sk-review` and a 32-hex id pass. Witness: `uses = ["sk-review"]` with a pre-approved skill; decoded `skill_ids[0]` is `<REDACTED:type=api_key_generic,...>`.
8. **P3.** Verified: format string at manifest.rs:4170; refusal Display flows to operator output.
9. **P3.** Verified: `warn_n_minus_1_degradations` text says "absent ... defaulted"; gate sets all four to `None` when `!fr8` with no per-section notice. Docs (v7.md) do say "ignored with a degradation warning", but the warning does not distinguish present-and-ignored.
10. **P3.** Verified: payload has `spirit_id, version, manifest_sha256, publisher, declared, skill_ids` only; hook_pump test asserts counts (2 declaration rows, 1 rejection), not correlation. Witness: load twice, decode the two rows, they differ only by `recorded_at_ns`.
11. **P2.** Verified: `author(...)` is unwrapped in both example and template; WIT result errors are thrown by componentize-js (the file's own `fetch` handler shows the `ComponentError.payload` shape). `check_insert` (maos-skill/src/admission.rs:289-299) returns `DuplicateSkillId` while a Pending entry with the same id exists. Witness: run the example twice against one data home without approving; second first-frame faults. AC7's hosted run boots once, so it cannot see this.
12. **P3 / [INFERENCE] on intent.** Verified: lineage = TL `intent` column of source frames; `deliver_typed` journals `intent_str` as high/standard/readonly. Example manifest declares `allowed = ["consult"]`. Not a defect in enforcement; it is an unstated vocabulary.
13. **P3.** Verified: story line 526 lists `docs/invariants/I13.md`; `git show --stat 5610f9d6` has no `docs/invariants` path; AC4.2 (line 301) says untouched.

### Boundaries walked and found handled (no finding)
- colon-hex decode: len != 79, group len != 4, uppercase, non-ASCII, extra/missing group all return None; encode/decode round-trips.
- validator: 0 and 33 entries, 0 and 129 bytes, uppercase, duplicate, unknown key, missing key, non-table section value all refuse with `section_parse` before any pid; schema 6 ignores them (witnessed by manifest_declarations_20_8).
- publisher None/Some: every signed path (`bind_registry_package_at` x3 in main.rs, installed boot) reaches `bind_registry_publisher`; local unsigned stays None.
- declared-empty: `uses = []` refused by the validator; `Some(empty)` built by hand refuses `skill_unavailable("")`.
- empty lineage: `write_distillate` refuses empty lineage, so `allows` cannot vacuously pass a stored distillate.
- wasm: in_export gate returns Unavailable in on_start/on_shutdown without touching the wire; skill calls share `turn.recalls` (MAX_RECALLS_PER_TURN=64) through the `is_tagged` branch; no guest field names author/pid; `Refused` carries the static `code()`.
- unload: door lifecycle commands hold a per-spirit lock, forget runs after a successful unload only; load-failure paths unregister; HookPanicked path unregisters.
- cross-boundary: `IacBusError` has no exhaustive consumer (the new variant falls into generic `Mailbox` mapping, see item 2); `GovernanceEventKind::ManifestDeclaration` parses in `load_ratification_frames`; `AdmissionRefusal` new codes have no exhaustive match outside admission.rs.
