# 20-8 landing 1 — code-review triage (candidate 5610f9d6, parent a13140b8)

Method: `bmad-code-review` steps 01–04, non-interactive. Layers: Blind hunter (`reviewer`, `blind.md`), Edge-case hunter (`reviewer`,
`edge.md`), Acceptance auditor (`reviewer`, `acceptance.md`), Test-infrastructure audit (`light-reviewer`, `testinfra.md`). No security
layer (orchestrator runs it). No layer failed. Reviewer: orchestrated by `Review208L1`; the orchestrator re-read the code before rating
(admission.rs, adapter.rs, distillate.rs, spirit_session.rs, installed_spirits.rs, install.rs, main.rs, example manifest/index.ts,
`ts_component_17_3b.rs`, the recipe). Nothing was compiled or run (no cargo/xtask). `[INFERENCE]` marks unobserved claims.
Line numbers are at 5610f9d6 unless a worktree path is named.

Totals: **0 decision_needed formally raised in-line; 6 operator decisions listed at the end**, **24 patch**, **4 defer**, **7 dismissed**
(merged duplicates counted once). Severity: high = intolerable, medium = tolerable, low = cosmetic.

## Verdict on the focus list (what held up)

- Colon-hex encode/decode (`governance.rs` new `encode_declaration_bytes`/`decode_declaration_bytes`): round trip, 79-byte length,
  uppercase/odd/short/extra-group rejection all hold; colon grouping defeats the 64-hex TL redactor (run threshold 32, `redaction.rs`).
- Journal-before-admit ordering: `emit_manifest_declaration_event` (`admission.rs:1146`) sits before `load_obj` (`:1149`) and after every
  structural/provenance gate; serialisation failure is a typed pre-pid `AdmissionRefusal::Admission`; I2 fail-stop writer untouched.
- Schema 6→7: `fr8` gate on `class_section.manifest_schema_version >= FR8_DECLARATIONS_SINCE_SCHEMA`; sections ignored at ≤6, strict
  (`deny_unknown_fields`, 1..=32 entries, 1..=128 bytes, charset, no duplicates) at ≥7, typed `section_parse` before pid.
- Signed install path: `verify_installed` returns the key from the already-verified package; `bind_registry_publisher` now also sets
  `gated.publisher`, so every signed path (installed boot, `bind_registry_package_at` x3 in `main.rs`) carries it; local manifests stay
  `None`. The replacement of `bind_registry_package_at` in `installed_spirits.rs:50` loses no check (dismissed, see D-1).
- I13: registered at admission (`admission.rs:1264`), removed on door unload/upgrade/uninstall/supervisor unload/rollback/hook-panic; no
  set ⇒ `check_promotion` falls through (`continue`); routed hot path for non-TaskAssign / no-ref frames is two pattern tests (perf-counts
  not plausibly moved); `docs/invariants/I13.md` untouched (verified by `git show --stat`).
- WASM bridge: host binds pid via `Arc::new(move || Some(pid))`; no guest-named author/pid/evidence in the WIT; calls ride the
  `is_tagged` branch (`spirit_session.rs:468-476`) so they count against `MAX_RECALLS_PER_TURN`; in-export latch inherited from
  `HostState::exchange` (`host_state.rs:62`); refusal carries `SkillPortError::code()` static strings.
- No kernel file in the commit (kernel pinned figures reproduced by the Acceptance auditor); pins reproduced exactly
  (manifest 4710, bin 35861, wasm-host 2521, domain 10190, iac 8693, xtask 44715, aggregate 198074 / hardfail 198075; deltas sum +623;
  all within the rebased ceilings, domain inside the adjudicated 10,205).
- No `unwrap`/`expect` after serde in production hunks (only `unwrap_or` on the clock; `writeln!(..).unwrap()` is in a guest fixture).
- Ratifications `20-8-manifest-schema-7` and `20-8-wit-spirit-2.3.0`, `FR8_DECLARATIONS_SINCE_SCHEMA`, the four section types and the single
  writer exist: the 20-9 contract items 1,2,3,6 are delivered; item 5 is true only as narrowed in P19; item 4 is L2.

## PATCH

### P1 [high] Shipped example/template manifest declares an unadmitted skill; three hosted jobs that `maos run` it are now refused
- `examples/example-spirit-ts/manifest.toml:21-23` and `templates/spirit-ts/manifest.toml` (`[skills] uses = ["example-prerequisite"]`, schema 7)
  against `admission.rs:1116` `check_declared_skills(...)?` reached from the `maos run` arm.
- Hosted callers that were green at the parent: `.github/workflows/discipline.yml:2529` (epic-17-exit line 2),
  `:2980` (epic-20-exit AC4 `maos run "$manifest" --once` on the installed example), `scripts/prove_17_3c.py:214` (`check(returncode == 0)`).
  None seeds `~/.maos/skills/example-prerequisite.md` or runs `maosctl skills approve`; `skills/example-prerequisite.md` inside the example dir is not on
  `default_search_path()` (`~/.maos/skills/`, `_bmad/skills/`, `/usr/share/maos/skills/`).
- Fix (see operator decision O1): keep the four declarations the story requires (AC1.5/AC6.7) and make every launcher seed and approve the skill
  (copy to `$HOME/.maos/skills/`, `maosctl skills approve <entry>` before `run`; `prove_17_3c.py` writes the file and the approval before `run`); or drop `[skills]` from the shipped
  example/template and prove the declared-skill path with a dedicated hosted fixture.
- Failing-before witness: Node ≥ 22.20 + `--features wasm-host`, fresh `HOME`, `maos init`, `maos run examples/example-spirit-ts/manifest.toml --once` ⇒ exit non-zero,
  `skill_unavailable: example-prerequisite`; equivalently `python3 scripts/prove_17_3c.py` red at its `returncode == 0` check.

### P2 [high] `ts_component_17_3b` driver hard-codes `Recall` as the guest's first wire request
- `crates/maos-wasm-host/tests/ts_component_17_3b.rs:190-211` (asserts `RecallRequest::Recall{limit:16}` first). The example `src/index.ts:23-34` now sends
  `author(...)` first; the driver never answers `SkillAuthor`; the failing/unanswered call faults `handleFrame`. `discipline.yml:1419` runs it in
  `example-spirit-ts-tests`, a `needs` of `wasm-host-tests` (`:4554`). Local runs cannot see it (Node 20 < 22.20; component never built).
- Fix: expect `RecallRequest::SkillAuthor{..}` first and reply `RecallReply::Skill{entry_id,target}`, then keep the recall/fetch exchange; keep template and example byte-equal.
- Witness: build the example component, `MAOS_TS_COMPONENT=... cargo test --release -p maos-wasm-host --test ts_component_17_3b -- --ignored` ⇒ `assert_eq` left `SkillAuthor` right `Recall`.

### P3 [high for AC7] Hosted recipe tamper assertion can never match
- `ci/ci-20-8-verify.yml:152` `grep -F installed_package_invalid tamper.log` under `set -e`. That string is only the journal note of the vetter-key row
  (`installed_spirits.rs:41`); boot prints `refused at boot: admission_refused (...)` (`installed_spirits.rs:40`; existing test `installed_spirit_dispatch_20_1.rs:179`). The cell exits 1 and AC1.5 tamper is never proven.
- Fix: `grep -F 'refused at boot: admission_refused' tamper.log`; add `maosctl audit query --intent governance:vetter-key-rejection | grep -F installed_package_invalid`; keep `cmp declarations.ndjson after-tamper.ndjson`.
- Witness: run the `declared` cell once; locally edit an installed manifest and compare both strings in `maos shell` output / audit query.

### P4 [high] I13 delivery check scans the entire Transparency Log, with payloads, per guarded TaskAssign
- `crates/maos-iac/src/adapter/distillate.rs:145-170` `admit_from_log` uses `query_frames(FrameFilter::default())` (no `frame_id`, no `kind`, no `limit`; `FrameFilter.frame_id` exists, `transparency_log.rs:282`),
  called from `adapter.rs` `check_promotion` (~`:222-262`) inside async `deliver_typed` (`adapter.rs:473`), holding the TL mutex for the scan and a DashMap `Ref` across the scan and the denial journal write (stalls
  `register_promotion_set`/unload for other spirits in the shard).
- Fix: `log.query_frame_by_id(digest_frame_id)` (or `FrameFilter{frame_id: Some(id), kind: Some(Distillate), limit: Some(1), ..}`); in `check_promotion` copy the verdict then `drop(allowed)` before the denial `insert_frame_event`.
- Witness: file-backed TL with ~100k filler rows (as `erasure_million_20_11`), register a set, time `deliver_typed` of a guarded TaskAssign: O(N) today, O(1) after; and a concurrent `insert_frame_event`/`register_promotion_set` that blocks for the scan duration today.

### P5 [medium] Skill-gate and bus-unavailable refusals leave no audit row on the door/run load paths
- `admission.rs:1116-1121` bare `?` returns; the migrator refusal at `:1128-1131` calls `emit_structural_rejection`; `BinPrivateOpsImpl::load_spirit` (`main.rs` ~10206-10240) journals only `prepare_spirit` refusals, so `maosctl load` of a manifest with an unavailable/unadmitted/escalating `[skills]` leaves zero TL rows (no declaration, no rejection).
- Fix: wrap the three early gates: on Err `emit_structural_rejection(deps.transparency_log, &gated.manifest_toml, &refusal)` then return, same shape as the migrator branch.
- Witness: `load_admit_start` test with schema-7 `[skills] uses=["missing"]`; assert a `governance:vetter-key-rejection` row containing `skill_unavailable` (today: none).

### P6 [medium] TS template/example crashes on its second session (unguarded `author()`)
- `templates/spirit-ts/src/index.ts:23-34` and `examples/example-spirit-ts/src/index.ts:23-34`: a repeat of the fixed id@version is refused by the port (`prepare_author` `StaleTargetVersion`, `check_insert` `DuplicateSkillId` while Pending) or `capability-denied`/`quota`; the WIT result error is thrown out of `handleFrame`, faulting the guest. Vitest mock returns `undefined`, so it cannot see it; hosted AC7 boots once.
- Fix: wrap in `try/catch` tolerating `refused`/`unavailable`/`capability-denied` (`error.payload.tag`), log the code via `console.log`, set `authored = true`; extend the vitest mock to throw `{payload:{tag:"refused",val:"duplicate-skill-id"}}` and assert the echo still happens.
- Witness: run the component twice against one data home without approving; second session faults. Vitest: mock throws ⇒ `handleFrame` throws today.

### P7 [medium] Documented/templated `allowed = ["consult"]` cannot match production distillate lineage
- `templates/spirit-ts/manifest.toml:29`, `examples/example-spirit-ts/manifest.toml:29`, `docs-site/docs/manifest/v7.md:19` (+ko): lineage is the TL `intent` column of the source frames (`distillate.rs:300-330`); routed frames are journaled `high`/`standard`/`readonly` by `deliver_typed`. The only witness (`intent_promotion_20_8.rs:62-100`) fabricates a `consult` row.
- Fix: document the real vocabulary in v7.md (+ko); change example/template to a value that can occur (e.g. `["standard"]`); add a witness whose source frame is created through `deliver_typed`.
- Witness: distil a `deliver_typed` Standard frame; set `{consult}` ⇒ denied; `{standard}` ⇒ allowed.

### P8 [medium] Promotion denial: causes conflated, row unattributable, and a WASM sender's whole session is killed
- `adapter.rs` `check_promotion` (`.is_err()` collapses `SourceFrameNotFound`, `Storage` (incl. scrubbed/tombstoned body) and a real lineage denial into `IntentPromotionDenied`); the journal row `iac:intent-promotion-denied` (kind TelemetryEvent) carries only the recipient id, no sender, frame id, digest id or cause; `spirit_session.rs:1036-1042` turns any `deliver_typed` error into `SessionError::Mailbox`, ending the sender's session after an `allowed:true` authorization row.
- Fix: distinct errors (denial vs storage vs not-found); journal `{recipient, sender, frame_id, digest_frame_id, cause}`; in `authorize_and_deliver` treat `IntentPromotionDenied` as a denied frame (`revoke`, `Ok(false)`, count in `denied_frames`) (see O3).
- Witness: scrub/corrupt the distillate body, deliver ⇒ today `IntentPromotionDenied`, expect a storage error; WASM sender TaskAssign with disallowed lineage ⇒ today session ends, expect `denied_frames == 1` and the session continues.

### P9 [medium] AC1.4–1.6 signed-install path has no local witness or plant
- `crates/maos-bin/tests/manifest_declarations_20_8.rs` has two tests (gate refusals; the writer with a hand-set `publisher`); no signed package via `tests/harness/registry_20_1.rs`, no valid-admitted-with-one-row, no `publisher: null` for a local manifest, no post-install `[swap_invariants]` tamper ⇒ no new row (the story's AC1.6/AC1.5). Non-wasm builds return `WasmEngineOff` before `load_admit_start`, so the 20-1 tests never reach the writer.
- Fix: `wasm-host`-gated test installing through `registry_20_1.rs`, booting `InstalledSpirits` with the `echo_spirit_component.wasm` fixture, asserting one row whose decoded `publisher` equals the signed key and `manifest_sha256` equals sha256 of the installed manifest; a local `load_admit_start` vector with `publisher == null`; tamper ⇒ no additional row.
- Plant/witness: change `installed_spirits.rs:50` to `let _ = &publisher;` ⇒ all current local tests stay green; the new test must go red.

### P10 [medium] I13 witness does not prove "nothing enqueued"
- `crates/maos-iac/tests/intent_promotion_20_8.rs:62` creates `_handle` and never reads it; refusal asserts at `:84-97` are TL rows only. A mutant that enqueues then returns `Err`, or moves `check_promotion` after the push, passes.
- Fix: keep `handle`; after the denied send `assert!(matches!(handle.try_recv(), Ok(None)))`; after the `[consult]`/no-set sends assert the frame is received.
- Witness: move `check_promotion` after the enqueue (`adapter.rs` ~473) ⇒ current test green, strengthened test red.

### P11 [medium] AC6.5(e): no genuine 2.1/2.2 guest runs on the 2.3 host
- Only `guests/compat-echo-2-0` is a legacy guest; every other fixture is built from the root 2.3.0 WIT. `wit_corpus.rs` adds vocabulary checks only. "2.0–2.2 guests run unchanged" is exercised only for 2.0.
- Fix: vendor a 2.2.0 `spirit.wit` compat guest (as `compat-echo-2-0/wit`), build it in `build-test-fixtures`, run it in `recall_runner_17_3d` beside the 2.0 case; or record 6.5(e) as an explicit limitation in the story and witness matrix.
- Witness: none exists today (renaming the host's registered `recall@` version would not red any test).

### P12 [medium] Docs-site and doc comments not current (AC2.1/AC9.1)
- `docs-site/docs/manifest/index.md:14,26` (+ko `:16,27`): v6 still "Current", range `1..=6`; `v5.md:9` (+ko) says current is v6; `docs-site/docs/migrate/index.md:15,17,19,35` (+ko `:20,36`); ko abi `constants.md:69,90`; `docs-site/docs/cookbook/wasm-component-spirit.md:88,130` (+ko `:87,127`) still say `maos:spirit@2.2.0`/`recall@2.2.0`/`context@2.2.0`; doc comments `maos-wasm-host/src/lib.rs:9`, `frame_bridge.rs:1`, guests (`equiv-fixture` comment edits move `source_sha256`: re-mint provenance if those files are pinned).
- Fix: update tables/ranges/versions (+ko); touch pinned guest sources only with a provenance re-mint.
- Witness: `git grep -nE '1\.\.=6|schema is \[v6\]' docs-site` must be empty after.

### P13 [medium] Story records contradict the commit
- File List (`20-8-manifest-contract-completion.md:526`), T4 (`:382`) and Dev Notes (`:404`) list `docs/invariants/I13.md` as changed (ruling `i13doc`; `git show --stat 5610f9d6` has no such path).
- The commit carries 20-11 record edits (`20-11-erasure-inventory-closure.md`, `20-11-…-evidence/landing-1/hosted-evidence.md`, 20-11/20-1b rows in `deferred-work.md`, a 20-11 line in `sprint-status.yaml`) that no 20-8 File List line owns.
- T0–T7 (`:378-385`) are unchecked although implemented; `i13doc` requires the witness named in "this story, coverage-matrix and the dev record": the Dev Record and the FR8 coverage note name the file, not `recipient_promotion_set_checks_stored_lineage_before_enqueue`; D-20-8-08's ordering/rollback sentence (`:~180`) is superseded only by a later paragraph; the `preflight.md` tail "L1 ceiling" table (bin 36,163 / domain 10,167 / aggregate 198,591) contradicts the rebased ceilings; `d11_xtask_ceiling_ratchet` will need a story line containing `xtask` and `44696 -> 44715` when the key goes `done` [INFERENCE].
- Fix: correct each; add the `xtask … 44696 -> 44715` line to the Dev Record.
- Witness: `git show --stat 5610f9d6 | grep -c invariants/I13` = 0 versus the File List mention.

### P14 [medium] T7 / proof-contract landing gates not met (candidate only)
- `prepush.md`: single prepush hit 3000 s with no JSON; `abi-diff` unrun; `check-test-budget` exit 1; `cargo check --workspace --all-targets --target aarch64-apple-darwin` exit 101 (no local C toolchain), so portable-call claims are unverified; `clippy-advisory`/`clippy-ratchet` not run on this diff (hosted lints: result_large_err, collapsible_if, too_many_arguments, unused_mut, trim_split_whitespace — none visible by inspection in the hunks, but unproven); no hosted discipline run; the final-tree workspace run predates two deterministic corrections.
- Fix: record each aggregate gate's exit individually (or split prepush cells), run `abi-diff` after the clean commit, run the hosted branch recipe, before T7 or any status flip. Not a code patch.

### P15 [medium] `build-test-fixtures` gained three rows that nothing reads
- `xtask/src/build_test_fixtures.rs:248-264` builds `equiv_{identity,divergent,cosmetic}_spirit.wasm`; tests read the committed `tests/fixtures/wasm/equiv_*_spirit_component.wasm` (`equiv_harness.rs:655-657`); no repo path resolves the new names. Every CI fixture step builds three extra wasm32-wasip2 guests, kloc +19, and AC6.1's "re-issued by `build-test-fixtures`" is false (the provenance gate documents manual re-mint). The committed provenance hashes themselves are correct (testinfra recomputed all 8).
- Fix: delete the three rows and re-pin xtask (re-measure), reword AC6.1 to the manual recipe; or make `equiv_harness` resolve the builder names and stage `_component.wasm`.
- Witness: mutate the equiv guest source, run `build-test-fixtures`, run `equiv_harness` ⇒ still green on the stale blob.

### P16 [medium] The live skill-authoring witness runs in no mainline CI job
- `wasm_skill_authoring_20_8.rs:45,49` is `wasm-host`-gated; `discipline.yml:6368` workspace run uses default features (only the 0.068 s AST test runs), the archive job `:2123-2126` omits it, the existing wasm-host job (`:1455`) runs only `wasm_form_admission_17_3b`. Only the unverified branch-only recipe executes it.
- Fix: add `--test wasm_skill_authoring_20_8` to the `:1455` job (or the archive list); keep the four-image matrix as amplifier.
- Witness: `grep -rn wasm_skill_authoring .github/workflows` = 0 hits; the "daemon arm answers `Unavailable`" plant reds no scheduled workflow.

### P17 [low] Declared skill ids/names can be mangled by the TL redactor in the declaration row
- `manifest.rs` `declared_names` accepts `sk-…`, and ≥32 consecutive hex characters; `redaction.rs` rewrites `sk-`/`ghp_`/`gho_`/`ya29.` prefixes and 32+-hex runs ⇒ `skill_ids[0]` is `<REDACTED…>` (row still parses). Record no longer equals the declaration.
- Fix: reject such names in `declared_names` with a shared constant, or encode `skill_ids` colon-grouped like enc208. Witness: `uses = ["sk-review"]` with an approved skill, decode `skill_ids[0]`.

### P18 [low] `check_declared_skills` mislabels infrastructure failures and has undefined multi-copy semantics
- `admission.rs:1046-1069`: `LocalFsSkillQueueStore::new`/`load`/`query_approvals`/`skill_entries` errors all become `skill_unavailable` naming the first declared id; `:1050-1088` requires EVERY discovered copy of an id (any root, any version, incl. CWD-relative `_bmad/skills/`) to be Admitted and capability-covered.
- Fix: map infrastructure errors to `AdmissionRefusal::Admission{detail:"skill store unavailable: …"}`; define and document one effective-copy rule (see O2) and add a vector.
- Witness: `X@1.0.0` admitted in `~/.maos/skills`, `X@0.9.0` unadmitted in `./_bmad/skills` ⇒ refused today.

### P19 [low] "Every issued pid has a prior declaration row" is overstated
- Story AC1.3, Contract-for-20-9 item 5, v7 docs. `scheduler.load` is also called outside `load_admit_start` for compiled-in/test roots (`main.rs` ~4411 `maos shell` hello-spirit from the compile-time manifest; smoke paths ~7782-8113) with no declaration row and no skill/promotion registration; the kernel in-process hot swap (L2/20-9) also issues successors outside the gate.
- Fix: restate the contract as "every pid issued through `load_admit_start` (every manifest-admitted Spirit); compiled-in first-party roots declare no FR8 sections and write no row"; same in v7.md (+ko) and the 20-9 contract text. Witness: `maos shell` run ⇒ pid issued, zero `governance:manifest-declaration` rows.

### P20 [low] `declared_names` echoes unbounded manifest text into the refusal
- `manifest.rs` ~4170: `format!("invalid or duplicate entry '{name}'")` with unbounded/control-character input, and invalid vs duplicate indistinguishable. Fix: `{:?}` of the first 64 chars; separate messages. Witness: 10 kB entry ⇒ refusal detail ~10 kB.

### P21 [low] Schema ≤6 manifests that literally contain the four sections get an "absent" WARN
- `admission.rs:829-851` sets the four to `None`; `warn_n_minus_1_degradations` says "absent … defaulted". Fix: when `!fr8` and `root.get(name).is_some()`, warn "declared section ignored below schema 7". Witness: schema-6 manifest with `[swap_invariants]` ⇒ WARN text.

### P22 [low] AC2/AC3/ordering witnesses are thinner than claimed
- `manifest_n_minus_1_test.rs:31` `MAX_SUPPORTED >= 7` is a constant compare, not `check_abi_triple`; schema-6 `declared: []` is not asserted; ordering (row before `load_obj`) was never planted (move the emitter below `load_obj`); the new refusal rows for load/storage-identity/start (`admission.rs:1167,1191,1286`) have no assertion (only AlreadyLoaded does); plants restored from `/tmp` copies rather than `git checkout`/`stash` (proof-contract "Plants", bytes verified, procedural).
- Fix: add those assertions and plants. Witness: each mutant above currently leaves the suite green.

### P23 [low] Evidence/records wording (testinfra F3, F4, F5)
- `deferred-work.md:1559` omits that `hook_pump_17_4` grew 26→28 tests (+1.360 s) in this commit; `.config/nextest.toml:54-55` "no per-binary exception widening" now precedes two 31 s exception binaries; "skill calls count against MAX_RECALLS_PER_TURN" is only structurally true (add a budget vector: 64 recalls then one `skill-author` ⇒ `"recall budget exceeded"`).

### P24 [low] Fixture-builder/duplicate-load wording
- Document in `equiv-fixtures.provenance.toml` header whether builder output is a reproduction recipe (if P15 keeps the rows). Merge with P15 if the rows are deleted.

## DEFER

- **W1 [medium → L2/20-9]** A kernel hot-swap successor never re-registers or withdraws the I13 promotion set (set registered only in `load_admit_start`; kernel swap path bypasses it), and a crashed/cold-exited recipient keeps a stale set until a reload overwrites (fail-closed, so harmless in the interim). Owner: L2 / `20-9-spawned-form-lifecycle-hooks` (swap/upgrade admission). `admission.rs:1264`, `supervision.rs:2523`.
- **W2 [low]** Every refused duplicate load (`AlreadyLoaded`) appends another byte-identical `governance:manifest-declaration` row (`admission.rs:1146` before `:1155`; codified by `hook_pump_17_4`), indistinguishable from the admitted instance's row. Consistent with `ac13i2`; hardening (nonce, or `resolve_pid` check before the writer) is optional. Owner: 20-9 successor-admission design (see O5).
- **W3 [low]** AC7 hosted T2 proof is authored but unrun (`discipline.yml`/`epic-20-exit`/`wasm-host-tests` untouched; recipe branch-only); external-author J6 is an operator lane. Stays open by design of D-20-8-01 A; owner: the hosted run before `done`.
- **W4 [low]** TS/vitest/componentize never run locally (Node 20 < 22.20); the TS half of AC6.7 has no local proof. Owner: hosted run.

## DISMISSED (7)

- D-1 "Installed boot lost the version check" (Edge #4): `verify_installed` → `signed_package_at` → `verify_deployed_package` calls `check_identity(document, pkg.spirit_id, pkg.version)` (`install.rs:227`) on the signed manifest bytes, and `gated` is built from those same bytes, so `bind_registry_package_at`'s old `pkg.version != class_section.version` check is subsumed.
- D-2 `ratified_at = "2026-11-03"` on the new ratification rows: matches the operator ruling date of D-20-8-03/10; not a defect.
- D-3 `emit_manifest_declaration_event` `unwrap_or(Duration::ZERO)` on the clock: pre-existing pattern in the sibling emitters; not serde.
- D-4 `register_promotion_set(.., None)` removing an entry: intended (a successor without a set clears the predecessor's).
- D-5 `SessionDeps.skill` not cfg-gated: `SkillBuilder` is an ungated type alias and both construction sites are already inside `wasm-host` blocks.
- D-6 Hook for `#[non_exhaustive]` `IacBusError::IntentPromotionDenied` placement: no exhaustive consumer outside the crate (falls into generic `Mailbox`).
- D-7 `maos-cli/src/cli.rs` one-line doc edit and the `.wasm` re-issue: mechanical; provenance hashes verified by the test-infra layer.

## Operator decisions needed

- **O1 (P1)** Example/template manifests: keep `[skills] uses` and teach every launcher (discipline.yml epic-17-exit line 2, epic-20-exit AC4, `prove_17_3c.py`) to seed+approve `example-prerequisite`, or drop `[skills]` from the shipped example/template (AC1.5/AC6.7 then say three sections, with the skills vector proven by a dedicated fixture).
- **O2 (P18)** `[skills] uses` semantics when an id is discovered in several roots/versions: all copies must pass (today, CWD-dependent) versus "first root wins / highest admitted version".
- **O3 (P8)** I13 denial of a WASM Spirit's own TaskAssign: terminate the sender's session (today, like any bus error) or treat as a denied frame and continue (recommended).
- **O4 (P14)** Whether the L1 candidate may land on the current evidence (prepush timed out; `abi-diff`, clippy ratchet, aarch64 compile, hosted run absent) or must wait for those; this review recommends no status flip.
- **O5 (W2)** Accept repeated declaration rows for refused duplicate loads as the ac13i2 one-directional contract, or require pre-writer `AlreadyLoaded` refusal.
- **O6 (P7)** The intent vocabulary for `[intent_promotion_set].allowed`: document the TL `intent` column tokens (`high`/`standard`/`readonly` for routed frames) as the contract, or introduce a lineage vocabulary mapping in a later story.
