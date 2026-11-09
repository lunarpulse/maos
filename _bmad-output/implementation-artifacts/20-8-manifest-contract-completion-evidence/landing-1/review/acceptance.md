# 20-8 landing 1 — Acceptance audit (commit 5610f9d6, parent a13140b8)

Auditor scope: story ACs 1-7 and 9 (L1), rulings `enc208` / `ac13i2` / `i13doc`, Contract for 20-9 items 1-7, kloc envelope and pins, hosted
recipe `ci/ci-20-8-verify.yml`, STABILITY / docs-site / generated docs, File List / Dev Record truthfulness, coverage-matrix,
deferred-work, preflight + witness-matrix evidence. Read-only: no cargo / nextest / clippy / fmt / xtask run, no git mutation.
Independent measurement used `git archive` of both commits into `/tmp/aud/{old,new}` plus `tokei 14.0.0` with the exact `kloc_check.rs` arguments.
Claims I did not verify are marked [INFERENCE].

## 1. Per-AC verdict table

| AC | Verdict | What is verified | What is missing / wrong (finding #) |
|---|---|---|---|
| AC1 declarations verified, refused, journaled | **PARTIAL — not met as written** | Sections + bounded validator (`manifest.rs` new block), gate reads at schema ≥ 7, typed `manifest_section_invalid`, `emit_manifest_declaration_event` before `load_obj` (`admission.rs:1146`), I2 fail-stop witness `declaration_storage_failure_stops_before_pid_issuance`, one-row witness in `hook_pump_17_4`, plant on the writer. | AC1.4/1.5/1.6 signed installed path has **no local witness and no plant** (`gated.publisher` from `verify_installed`, `publisher: null` for local, tamper ⇒ no row). The required `manifest_declarations_20_8.rs` has no signed-package / valid-admitted / tamper vector (#4). Hosted recipe has a wrong assertion (#3). |
| AC2 schema 7 + degradation | **MET locally** | `MANIFEST_SCHEMA_VERSION=7`, `POST_V1_SCHEMA_SECTIONS` + `RATIFIED_POST_V1_SCHEMA_SECTIONS` (+4), ratification `20-8-manifest-schema-7`, v7 en+ko, STABILITY.md 1..=7, plant (`swap_invariants` dropped ⇒ red) observed. | Failing-before witness is a tautological const compare (`MAX_SUPPORTED >= 7`); schema-6 `declared: []` is not asserted (#10). `docs-site/docs/manifest/index.md` (+ko) still says v6 Current / `1..=6`; `v5.md` says current is v6 (#6). |
| AC3 skills exist/admitted/no escalation | **MET locally** | `check_declared_skills` (before pid), vectors absent / pending / escalation / admitted+id-in-record, plant (subset check skipped ⇒ red). | Failing-before at HEAD not observed (witness-matrix says so honestly) (#10). Multi-version semantics unspecified (#13). |
| AC4 I13 at TaskAssign delivery | **MET locally, one weak oracle** | `check_promotion` in `deliver_typed` before enqueue, stored-lineage resolution, `IntentPromotionDenied`, journaled `iac:intent-promotion-denied`, register at admission, unregister at door unload/upgrade/uninstall/supervisor unload; plant (check removed ⇒ red). `docs/invariants/I13.md` untouched (verified, ruling `i13doc` honoured in code). | "nothing enqueued" is not asserted against the recipient mailbox (#5). File List claims I13.md changed (#7). |
| AC5 explanation_shape declared, not enforced | **MET** | v7 en+ko and cookbook en+ko say "validated and journaled but not enforced — no decision frame carries an explanation body"; deferred row (`deferred-work.md:1483-1486`) stays open; no control claimed. | — |
| AC6 WIT 2.3.0 author/propose | **MET locally except 6.5(e); TS half unverified and breaking** | WIT 2.3.0 + ratification, wire variants, `HostState::skill` over shared `exchange` (in-export rule inherited), linker + `skills@` major check, `SkillPortError::code()`, `MediatedLogRecall.skill`, both `SessionDeps` sites, `SessionLaunch.scopes`, real-runner witness (a)(b)(c)(d), plant, fixtures re-issued and byte-identical to `build-test-fixtures` output (sha256 compared), template/example WIT == root (cmp). | 6.5(e) no 2.1/2.2 guest fixture exists (#9). Example `index.ts` now breaks hosted `ts_component_17_3b` (#2) and `maos run` of the example in three hosted jobs (#1). TS vitest/componentize never run locally (Node 20 < 22.20). |
| AC7 hosted T2 proof | **NOT MET (open, honestly labelled)** | Recipe authored, branch-only, loopback dev-http opt-in correct, not installed on main. | `discipline.yml` / `epic-20-exit` / `wasm-host-tests` not edited; recipe never run; tamper assertion cannot match (#3); plant cell unobserved. |
| AC9 records (L1) | **PARTIAL** | coverage-matrix FR8/FR39/FR57 entries, STABILITY, abi pages, deferred rows annotated, ratifications present, kloc rows exact. | Manifest index stale (#6); File List false + unlisted 20-11 payload + Tasks unchecked (#7); T7 gates (`prepush`, `abi-diff`, hosted run) not satisfied (#8). |
| AC8 | out of L1 | no kernel line changed (kernel-core 26841/21042 reproduced unchanged). | — |

### Rulings
| Ruling | Verdict | Evidence |
|---|---|---|
| `enc208` colon-hex + documented decoder | **Honoured** | `encode_declaration_bytes` → 16×4-hex groups joined by `:` = 79 chars; `decode_declaration_bytes` checks length 79, 4 lowercase-hex digits per group, no trailing group; round-trip asserted from the stored TL row (`manifest_declarations_20_8.rs`). Decoder is named in v7.md en+ko. |
| `ac13i2` row before SCB/pid, I2 fail-stop retained, one-directional | **Honoured in code; thinly witnessed** | `emit_manifest_declaration_event` precedes `load_obj`; serialization error ⇒ `AdmissionRefusal::Admission`; later refusals journal via `emit_vetter_key_event` for AlreadyLoaded / load error / storage-identity / admission / start (diff adds the first three plus start). Only AlreadyLoaded has an assertion (#10). A duplicate load writes a second declaration row (test codifies it) (#12). |
| `i13doc` I13.md untouched | **Code honoured; record false** | `git show --stat` has no `docs/invariants/I13.md`; the story File List line still lists it (#7). Witness function name is in AC4.2 only, not in the Dev Record; coverage-matrix names the file `intent_promotion_20_8` but not the function (#7). |

### Contract for 20-9
| # | Verdict |
|---|---|
| 1 WIT base 2.3.0; imports gated by `in_export` | Holds: `HostState::exchange` checks `in_export` for recall/context/skills alike; `on_start` guest asserts `Unavailable`. 2.4.0 lifecycle exports not touched. |
| 2 Ratifications `20-8-manifest-schema-7`, `20-8-wit-spirit-2.3.0` + fixtures | Present (`xtask/abi-ratifications.toml`); four fixtures re-issued, provenance toml matches committed blobs. `check-equiv-fixture-provenance` pass is the dev's report (preflight), not re-run by me. |
| 3 Manifest types / `FR8_DECLARATIONS_SINCE_SCHEMA` | Present and re-exported from `maos_manifest`. |
| 4 SCB/swap | L2; not in L1 (correctly absent). |
| 5 One writer, "each issued pid has a prior row" | True for `load_admit_start`, the only `load_obj` caller in `maos-bin/src` (grep). [INFERENCE] the kernel in-process hot-swap successor path is outside this claim until L2/20-9. |
| 6 `GatedManifest` fields | Present; `publisher` set by `bind_registry_publisher` (installed boot and `bind_registry_package_at` callers). |
| 7 WASM guests reach `LiveSkillAuthorPort` via `MediatedLogRecall.skill`, pid fixed by the root | Holds: one port per session, `Arc::new(move || Some(pid))`, no guest-named pid/author/evidence in the WIT. |

### kloc envelope and pins (independent reproduction, tokei 14.0.0, exact `kloc_check.rs` args on `git archive`)
| Crate | a13140b8 | 5610f9d6 | Δ | Pin in `kloc.toml` | Ceiling | OK |
|---|---|---|---|---|---|---|
| maos-manifest | 4604 | 4710 | +106 | 4710 | 4812 | yes |
| maos-bin | 35586 | 35861 | +275 | 35861 | 36160 | yes |
| maos-wasm-host | 2412 | 2521 | +109 | 2521 | 2874 | yes |
| maos-domain | 10144 | 10190 | +46 | 10190 | 10205 (adjudicated overdraft; 15 slack) | yes |
| maos-iac | 8625 | 8693 | +68 | 8693 | 8865 | yes |
| xtask | 44696 | 44715 | +19 | 44715 | 44749 | yes |
| aggregate | 197451 | 198074 | +623 | hardfail 198075 (= measured+1) | 199078 | yes |
All other crates (kernel-core 21042 tokei, registry 6151, skill 1234, cli 6429, spirit-abi) unchanged. 106+275+109+46+68+19 = 623 ✓. Every changed row's first HISTORY segment cites `20-8-manifest-contract-completion` with `old -> new` (D11 A); the cited key is `ready-for-dev` (not `done`) so `d11_xtask_ceiling_ratchet::the_live_xtask_ceiling_cites_a_real_sprint_story` holds today. [INFERENCE] when the key flips to `done`, that test requires a line in the story containing `xtask` and the arrow `44696 -> 44715`; none exists yet (the story only has `xtask ≤ 44749`).

## 2. Findings

### 1. [P1] Shipped example manifest declares an unadmitted skill; three hosted jobs that `maos run` it will be refused `skill_unavailable`
- **Where:** `examples/example-spirit-ts/manifest.toml:21-23` (`[skills] uses = ["example-prerequisite"]`, schema 7); `crates/maos-bin/src/admission.rs:1116` (`check_declared_skills(gated, deps.transparency_log)?` in `load_admit_start`, reached from the `maos run` arm `main.rs:6710`).
- **Evidence:** `.github/workflows/discipline.yml:2529` (epic-17-exit line 2: `target/release/maos run examples/example-spirit-ts/manifest.toml --once`), `:2980` (epic-20-exit AC4 step: `maos run "$manifest" --once` on the installed example), `:4887` → `scripts/prove_17_3c.py:214` (`check(process.returncode == 0, stderr)` after `maos run <example manifest> --once` in a fresh HOME). None copies `skills/example-prerequisite.md` to `$HOME/.maos/skills/` or runs `maosctl skills approve`; `discipline.yml` is not in the commit. `check_declared_skills` returns `SkillUnavailable{skill:"example-prerequisite"}` whenever the id is not discovered on `default_search_path()` (`~/.maos/skills/`, `_bmad/skills/`, `/usr/share/maos/skills/`) and `Admitted`. The example's own README (line ~128) says the operator must copy+approve first, which only the *new* branch-only recipe does. The hosted run `42533166219` at the parent shows these jobs green today (20-11 evidence), so this is a regression introduced here.
- **Fix:** either (a) add the copy+approve step to every hosted job/script that launches the example (`discipline.yml` epic-17-exit line 2 and epic-20-exit AC4; `prove_17_3c.py` must seed `home/.maos/skills/example-prerequisite.md` and write the approval before `run`), or (b) drop `[skills]` from the shipped example/template manifests and prove AC3's declared-skill path with a dedicated hosted fixture. Update `scripts/epic_17_exit_line2b.py` expectations if the first turn now also emits skill rows (it only forbids recall/channel rows, so likely unaffected [INFERENCE]).
- **Failing-before witness:** build the example with Node ≥ 22.20, build `maos` with `--features wasm-host`, fresh `HOME`, `maos init`, then `maos run examples/example-spirit-ts/manifest.toml --once` ⇒ non-zero exit, stderr `skill_unavailable: example-prerequisite`. Same in `python3 scripts/prove_17_3c.py ...` (`check(process.returncode == 0)` red).

### 2. [P1] `ts_component_17_3b` (hosted `example-spirit-ts-tests`, `--ignored`) hard-codes the guest's first wire request as `Recall`; the example now sends `SkillAuthor` first
- **Where:** `examples/example-spirit-ts/src/index.ts:23-34` (`author(...)` inside `if (!authored)` *before* `recall({ limit: 16 })`); unchanged `crates/maos-wasm-host/tests/ts_component_17_3b.rs:190-211` (`assert_eq!(decode_request(&request), RecallRequest::Recall{filter: limit 16 ...})`, then `Fetch`, then expects exactly one emitted frame).
- **Evidence:** the test file is not in the commit; the driver never answers a `SkillAuthor` request, and a failing `author()` (error reply) throws in componentize-js, so `handleFrame` traps before emitting. `discipline.yml:1419` runs the driver in `example-spirit-ts-tests`, which `wasm-host-tests` (`:4554`) `needs`. Local runs could not catch it (Node 20.19 < required 22.20, component never built).
- **Fix:** update the driver to expect `RecallRequest::SkillAuthor{..}` first and reply `RecallReply::Skill{entry_id,target}`, then the existing recall/fetch exchange (or move the `author` call after `fetch` and add the third exchange). Keep `template`/`example` byte-equal (`templates-regen --check`).
- **Failing-before witness:** `npm ci && npm run build && npm run componentize` in the example, then `MAOS_TS_COMPONENT=$PWD/examples/example-spirit-ts/dist/spirit.wasm cargo test --release -p maos-wasm-host --test ts_component_17_3b -- --ignored` ⇒ `assert_eq` left `SkillAuthor{..}` right `Recall{..}`.

### 3. [P1 for AC7] Recipe tamper assertion can never match
- **Where:** `ci/ci-20-8-verify.yml:152` `grep -F installed_package_invalid tamper.log` (under `set -e`).
- **Evidence:** `installed_package_invalid:` is only the journal note of the `governance:vetter-key-rejection` TL row (`installed_spirits.rs:34`). What `maos shell` prints (stderr merged into `tamper.log`) is `maos: installed spirit '<n>' refused at boot: admission_refused (admission rejected: <detail>)` (`installed_spirits.rs:40`, `AdmissionRefusal::code()` = `admission_refused`, `Display` = `admission rejected: {detail}` at `admission.rs:447`); the existing 20-1 test asserts `refused at boot: admission_refused` (`installed_spirit_dispatch_20_1.rs:179`). So the `declared` cell exits 1 at the tamper step and the AC1.5 tamper claim is never proven.
- **Fix:** replace with `grep -F 'refused at boot: admission_refused' tamper.log` and add `maosctl audit query --intent governance:vetter-key-rejection | grep -F installed_package_invalid` for the journal; keep the `cmp declarations.ndjson after-tamper.ndjson` check.
- **Failing-before witness:** run the step once (hosted `declared` cell); or locally `printf 'x' | maos shell` after editing an installed manifest and grep both strings.
- Other recipe checks that hold: trigger is `push` on `ci/20-8-verify` only, file lives under the evidence dir (not `.github/workflows`), `permissions: contents: read`, loopback opt-in is the same `[registry] development_http = true` + `--dev-http` pair `discipline.yml:2928/2935` already uses, `scripts/run_17_3c_delegated.sh`, `./.github/actions/provision-t3-test-substrate`, `shell-intro.json`, `maos-registry-server`, `maos-spirit`, `check-wasm-form-equiv` all exist; NDJSON `payload` / `governance_type`-tagged event / hex `publisher_pubkey` parsing in the Python block matches `maos-audit` + `SignedPackage` serialization.

### 4. [P2] AC1.4–1.6 signed-install path has no local witness; required `manifest_declarations_20_8.rs` does not contain the specified vectors
- **Where:** `crates/maos-bin/tests/manifest_declarations_20_8.rs` (2 tests: gate refusals + direct writer with a hand-set `gated.publisher = Some([0x9a;32])`); `installed_spirits.rs:25,50`; `admission.rs:550`.
- **Evidence:** AC1.6 requires "door/run admission of temp manifests + a signed installed package via `tests/harness/registry_20_1.rs`", "the valid manifest is admitted with one row", and a tamper vector. None exists. In non-wasm builds `InstalledSpirits::boot` returns `WasmEngineOff` before `load_admit_start`, so the existing 20-1 tests never reach the writer. Nothing asserts `publisher: null` for a local manifest. The preflight/witness-matrix admit "No claim of an observed original-HEAD signed-path red".
- **Fix:** add a `wasm-host`-gated local test (the dev ran wasm-host tests locally) that installs through `registry_20_1.rs`, boots `InstalledSpirits` with a prebuilt component fixture (`echo_spirit_component.wasm`) and asserts one `governance:manifest-declaration` row whose decoded `publisher` equals the signed key and `manifest_sha256` equals sha256 of the installed manifest; a local `load_admit_start` vector asserting `publisher == null`; and the post-install `[swap_invariants]` edit ⇒ no new row.
- **Failing-before / plant:** change `installed_spirits.rs:50` to `let _ = &publisher;` (publisher never bound) ⇒ today every local test stays green; the new test must red.

### 5. [P2] I13 witness does not prove "nothing enqueued"
- **Where:** `crates/maos-iac/tests/intent_promotion_20_8.rs:62` (`_handle` is created and never read; refusal asserts at `:84-97`); AC4.3 requires "refused, nothing enqueued, one refusal row".
- **Evidence:** the only post-refusal assertions are TL rows (`!rows.iter().any(|row| row.frame_id == [1;16])`), but `deliver_typed` journals accepted frames after the mailbox push in other paths; a mutation that enqueues the frame and then returns `Err(IntentPromotionDenied)` (or moves `check_promotion` after the push) passes this test.
- **Fix:** keep `let mut handle = bus.register_spirit(..)`; after the denied delivery `assert!(matches!(handle.try_recv(), Ok(None)))`; after the `[consult]` delivery assert the frame is received. `Handle::try_recv` exists (`mailbox.rs:751`).
- **Failing-before witness:** plant `check_promotion` call moved to after the enqueue at `adapter.rs` ~line 473 ⇒ current test green, strengthened test red.

### 6. [P2] Docs-site and doc comments not current (AC2.1 / AC9.1 "docs current")
- `docs-site/docs/manifest/index.md:14,26` and ko `index.md:16,27`: v6 is still the "**Current**" row, range "currently `1..=6`", "Latest … v5"; v7 is not linked. `docs-site/docs/manifest/v5.md:9` (and ko) says "The current schema is [v6]". v6.md was changed to say v7 is current, so the site now contradicts itself.
- `docs-site/docs/cookbook/wasm-component-spirit.md:88,130` (+ko `:87,127`) still say `maos:spirit@2.2.0` / `recall@2.2.0` / `context@2.2.0` in the same page that now documents 2.3.0.
- Stale `maos:spirit@2.2.0` comments: `crates/maos-wasm-host/src/lib.rs:9`, `frame_bridge.rs:1`, guests `echo-spirit/src/lib.rs:1`, `recall-probe/src/lib.rs:1`, `equiv-fixture/{logic,wasm-guest}/src/lib.rs`.
- **Fix:** update the index tables/range, v5 pointer, cookbook version strings and comments (equiv-guest comment edits move `source_sha256`, so re-issue provenance if those files are pinned).

### 7. [P2] Story File List / Dev Record not truthful or not complete
- `20-8-manifest-contract-completion.md:526` File List claims `docs/invariants/I13.md` was changed. It was not (commit stat; ruling `i13doc` forbids it). Also stale: Dev Notes table `:404` and T4 `:382` ("I13 doc").
- The commit also contains 20-11 records (`20-11-erasure-inventory-closure.md` +16/−…, `20-11-…-evidence/landing-1/hosted-evidence.md`, 20-11/20-1b annotations in `deferred-work.md`, the 20-11 line in `sprint-status.yaml`) that no 20-8 File List line owns.
- Tasks T0–T7 (`:378-385`) are all `- [ ]` although the Dev Record says they were implemented; T0 "tracker → `in-progress`" never happened (`sprint-status.yaml` 20-8 stays `ready-for-dev`).
- D-20-8-08 row text (`:~178`) still says "written after the granted admission and before start. A failed write rolls the load back"; only the later D-20-8-13 paragraph supersedes it.
- `i13doc` requires the witness to be named in "this story, `tests/coverage-matrix.yaml` and the dev record": the Dev Record names no I13 witness; coverage-matrix FR8 names the file `intent_promotion_20_8`, not `recipient_promotion_set_checks_stored_lineage_before_enqueue`.
- **Fix:** remove I13.md from the File List and T4/Dev Notes; list or split the 20-11 payload; tick tasks or state which are open; name the witness function in the Dev Record and coverage note; mark D-20-8-08's ordering/rollback sentence "superseded by D-20-8-13" in place.

### 8. [P2] T7 / proof-contract landing gates not met (candidate only)
- `prepush.md`: the single `prepush` hit the 3000 s deadline (wall 3000.32 s) with no JSON; `docs/testing/proof-contract.md` "Pre-push (20-16)" requires exit 0 before every push of a story tree, recorded in the Dev Record. The post-correction JUnit shows 5849 tests, 1 failure (20-7 1M oracle, timing) but the rest of the aggregate (fmt check, `check-serde-error-handling`, `check-workspace-count`, `check-pub-field-constructors`, `check-composition-root-completeness`, `check-unsafe`, `check-dependency-closure`, `check-test-budget`, doc tests, `cargo test -p maos-kernel-core --lib`) has no recorded outcome.
- `abi-diff` not run (dirty-tree refusal); `check-test-budget` exit 1 (`workspace-budget.txt`, `prepush-budget.txt`); `cargo check --workspace --all-targets --target aarch64-apple-darwin` exit 101 (local C toolchain) — portable-compile claim for the new maos-bin / maos-iac / maos-wasm-host code on `aarch64-apple-darwin` is unverified; `clippy-advisory`/`clippy-ratchet` never run for this diff (hosted ratchet against `xtask/clippy-baseline.toml`); no hosted discipline run.
- The "final tree" workspace run predates the two deterministic corrections (the dev record says so). The commit message calls it a candidate; nothing may close T7 or flip status on this evidence.
- **Fix:** run the required once-only gates in cells that fit (split `prepush` legs or run the aggregate gates individually and record each exit) and a hosted discipline run; record them in the Dev Record before T7 is ticked.

### 9. [P2] AC6.5(e) not witnessed: no genuine 2.1/2.2 guest runs on the 2.3 host
- **Where:** `crates/maos-wasm-host/guests/` has only `compat-echo-2-0` (own 2.0.0 WIT); every other fixture is rebuilt from the root 2.3.0 WIT (`echo-spirit`, `recall-probe`, `routed-relay`, `equiv-fixture/*`). `wit_corpus.rs` changes only add the `queued`/`skill-error` vocabulary checks; `witness-matrix.md` row 6 is silent on (e).
- **Impact:** the claim "2.0–2.2 guests run unchanged (semver import resolution)" is exercised only for 2.0; the 2.1/2.2 `context@2.2.0` + `recall@2.2.0` imports against the new host linker (with `skills` added) are untested.
- **Fix:** add a compat guest built against a vendored 2.2.0 `spirit.wit` (same pattern as `compat-echo-2-0/wit`), build it in `build-test-fixtures`, and run it in `recall_runner_17_3d` beside the 2.0 case (also rename the stale `compat-2-0-on-2-2` label). Or record the gap as an explicit limitation.
- **Failing-before witness:** none today; e.g. renaming the host's registered `recall@` interface version in `runner.rs` so only 2.2-era imports fail to resolve would not red any test.

### 10. [P3] Evidence is honest but thinner than the story's "failing-before + plant per AC"
- Observed HEAD red exists only for AC2 (`schema_seven_is_supported_for_fr8_declarations`, a constant compare — it does not exercise `check_abi_triple`) and the two fixture-bug reds; AC1/AC3/AC4/AC6 failing-before at HEAD are explicitly "not observed" (`witness-matrix.md`).
- AC1 plant removes the TL insert *inside* the emitter; the ordering claim (row before `load_obj`) is witnessed by the I2 drop-guard test but was never planted (e.g. moving `emit_manifest_declaration_event` below `load_obj` was not tried).
- New refusal journaling for "load rejected", "storage identity rejected", "start rejected" (`admission.rs` at `:1173`, `:1197`, `:1292`) has no assertion and no plant; only AlreadyLoaded is asserted (vetter-rejection count 1).
- AC2.2 "record says `declared: []`" at schema 6 is not asserted (the test only checks `gate_manifest(...).is_ok()`).
- Plants were restored from `/tmp` copies (`cp` + `cmp` + `touch`) whereas `proof-contract.md` "Plants" says restore with `git checkout -- <file>` / `git stash` and "never a workspace copy". Byte restoration was verified, so this is procedural.
- Preflight table inconsistency: the T0 "L1 ceiling" table at the end of `preflight.md` (bin 36,163 / domain 10,167 / aggregate 198,591) contradicts the rebased ceilings at the top (36,160 / 10,205 / 199,078) and the story.
- **Fix:** add plants for the ordering and the three new refusal rows; assert `declared == []` for schema 6 at the writer level; fix the stale table.

### 11. [P3] `build_test_fixtures.rs` adds three equiv-guest builds that no test or gate consumes
- `xtask/src/build_test_fixtures.rs:237-255` registers `equiv_{identity,divergent,cosmetic}_spirit.wasm`; the only consumers of the equiv blobs read the committed `tests/fixtures/wasm/*_component.wasm` (`equiv_harness.rs:655-657`). I verified the builder output is byte-identical to the committed blobs (sha256 compare), so this is the reproduction recipe, but the CI producer will now build, seal and transport three extra artifacts every run for no consumer. Informational; if kept, document it in the provenance toml header (the added comment line only says "Re-issued … by 20-8").

### 12. [P3] A refused duplicate load writes another declaration row
- `admission.rs:1146` runs before `load_obj` returns `AlreadyLoaded`; `hook_pump_17_4::manifest_declaration_precedes_pid_and_refusal_is_distinct` codifies two rows for one issued pid. Consistent with ruling `ac13i2` (pid ⇒ row, not conversely) but every repeated `maosctl load` of a loaded Spirit appends a kind-28 row. Optional fix: refuse `AlreadyLoaded` (`deps.scheduler.resolve_pid(&spirit_id).is_some()`) before the writer, keeping the separate refusal row.

### 13. [P3] [INFERENCE] `check_declared_skills` requires every discovered copy of an id to be Admitted and capability-covered
- `admission.rs:1061-1085`: the `for item in discovered.filter(id)` loop returns `Err` for the first non-admitted/escalating copy, so a pending newer version on another root blocks a Spirit whose declared id has an admitted older copy. The story says only "discovered … and Admitted". Decide and document ("some admitted copy" vs "all copies"); add a vector.

### 14. [P3] [INFERENCE] I13 check re-reads the whole log per delivery
- `DistillateWriter::admit_from_log` (`distillate.rs:145-170`) calls `query_frames(FrameFilter::default())` and scans linearly on every TaskAssign with a `prior_distillate_ref` to a recipient that has a set (the pre-existing `DistillationPort` path did the same, but it was off the delivery path). `intent_promotion_20_8` measures nothing; the preflight's "no measurement establishes a new instruction baseline" is the only perf statement. If this path is hot, use the id-keyed metadata query (`query_frame_metadata_by_id` pattern from 17-3d).

## 3. Items checked and found clean
- `docs/invariants/I13.md` unchanged; `i13.rs` doc comment still true.
- `enc208` encoder/decoder round trip and docs; `GovernanceEventKind` is `#[serde(tag = "governance_type")]` so the new row decodes as `GovernanceEventPayload` and does not break `load_ratification_frames` (`maos-audit/src/lib.rs:536-600`).
- `bind_registry_package_at` → `bind_registry_publisher(&publisher)` swap in `InstalledSpirits::boot` loses no check: `verify_installed` → `signed_package_at` → `verify_deployed_package` already verifies signature, manifest/artifact bytes and `check_identity` (name+version) (`install.rs:184-244`).
- All four production `AdmissionDeps {` sites and every `SessionDeps {` site carry the new fields; `AdmissionRefusal` new variants fall into the door's catch-all `Invalid` (`operator_door.rs:1031-1036`).
- `promotion_sets` unregistered after successful unload (door verb, upgrade, uninstall, supervisor worker unload) and after rollback / hook-panic.
- Kernel-core, registry, skill, CLI tokei counts unchanged; no kernel file in the commit.
- Example/template manifests differ only in `name` / skill id placeholders; WIT copies byte-equal to `wit/spirit.wit`.
