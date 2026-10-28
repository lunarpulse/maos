# 19-4 creation — cites re-measured at `61af7461` (2026-10-28)

Method: committed HEAD extracted read-only with `git archive HEAD | tar -x -C /tmp/h19x4`; every range below read there
with `read`/`grep -n`. The working tree carries uncommitted 19-3/19-6 code (`git status --short`: 40 modified,
4 untracked). Where a cited file is modified in the tree, the HEAD line is the cite and the tree line is given as
**WT** — re-measure both at T0 after 19-3/19-6 commit. No cargo was run (another build held the host).

Files cited by 19-4 that carry uncommitted wave-2 edits: `xtask/src/demo_j1.rs` (19-3 AC1.2: live take + module doc,
+2 lines above `:48`), `xtask/src/check_j1_loopback_delegation.rs` (19-3 `_19_3.rs`, +4/+6), `xtask/tests/j1_crosshost_1b_proven_red.rs`
(19-3 twin after `:670`), `xtask/src/main.rs` (19-3 doc strings, +2 after `:1101`), `crates/maos-bin/src/main.rs` (19-6 loop/crash
+ 19-3 literal, +85 net), `crates/maos-bin/src/orchestrator_root.rs` (19-6, +8 above `:41`, +127 after `:259`),
`crates/maos-bin/src/env_contract.rs` (+5 above `:332`), `spirits/orchestrator/src/lib.rs` (19-6 release helper after `:475`),
`.github/workflows/discipline.yml` (19-3 J1 step `:3427-3428`, +3). Unmodified: `spirits/butler/src/lib.rs`,
`crates/maos-cli/src/cli.rs`, `crates/maos-journey-test/tests/journey_j1.rs`, `crates/maos-corpus-gen/src/lcas.rs`,
`crates/maos-kernel-core/src/**` except 19-3's three K2 files, `spirits/orchestrator/manifest.toml`, `xtask/kloc.toml`,
`xtask/gate-registry.toml`, `xtask/src/check_j1_two_host_signed_run.rs`.

## Epic §19-4 / §R / exit-block cites (OLD → NEW)

| Cite (epic line) | OLD | NEW @61af7461 (WT) | How measured |
|---|---|---|---|
| exit L1 `MAOS_INFERENCE_MODE` registry (`:25`, §R R19-1 `:68`) | `env_contract.rs:317` | `:332` (WT `:337`) | grep `name: "MAOS_INFERENCE_MODE"` |
| exit L1 resolver (`:25`, R19-1) | `main.rs:3793-3814`, `resolve` `:3807` | `main.rs:3833-3840` (`resolve(` `:3835`; WT `:3838`) | grep `ResolvedInferenceMode::resolve(` |
| exit L1 `MAOS_REPLAY_CASSETTE` (`:26`, R19-2 `:69`) | `env_contract.rs:322`, read `main.rs:3805` | `:337` (WT `:342`), read `main.rs:3833` (WT `:3836`) | grep |
| AC4 strict (`:202`, R19-2/R19-22) | `env_contract.rs:327`, read `main.rs:3806` | `:342` (WT `:347`), read `main.rs:3834` (WT `:3837`) | grep `let replay_strict` |
| exit L1 `cassettes/` = `j-butler j-researcher j0` (`:26`, R19-2) | 3 dirs | `j-butler j-researcher j0 j1` — `j1/orchestrator-halt-corpus-v0.1.json` (19-1 AC5); `j1/founder-loop.json` still absent | `ls` |
| AC1 spawn (`:199`, R19-19) | `demo_j1.rs:544-552` | holds (WT `:543-551`; `env_remove` `:550`, WT `:549`) | read |
| AC1 `TOPOLOGY` | `:48` | holds (WT `:50`) | grep |
| AC1 `apply_capture_judgement` | `:958-984` | holds (WT `:957-983`) | grep |
| AC1 `SIGNABLE_WORKER_CLIS` | `:384` | holds (WT `:386`) | grep |
| AC1 tier-2 owner / resume beat / `unlanded_beats` | `:892-896` / `:902-906` / `:890-908` | hold (WT −1) | read |
| AC1 two-host README | `j1-two-host-evidence/README.md:11-18` | holds | read |
| AC1 beats P1…P6b | `demo_j1.rs:578-589` (P1) | P1 holds; all beats `evaluate_beats` `:572-802` (WT `:571-801`); `ORDERED_STAGES` (each stage exactly once) `:729-767`; `EXPECTED_IDLE_FIRES` `:372` | read |
| AC1 demo dispatch | `xtask/src/main.rs:1094-1111`, `:1510` | `:1100-1117` (variant `:1101`), dispatch `:1517` (WT `:1519`) — as 19-3 creation recorded | grep |
| AC2 Orchestrator "loaded bare" | `main.rs:4997-5001` | **false** — 19-1 constructs it through `orchestrator_root::construct_at_admitted_pid` (`orchestrator_root.rs:65-83`, WT `:73-91`): topology arm `main.rs:5165-5184` (WT `:5250-5269`), standalone `:6010-6023` (WT `:6095-6108`), successor factory `construct` `:997-1015` | read |
| AC2 Mira arm | `main.rs:5012-5058` | topology `:5195-5241` (WT `:5280-5326`); standalone `:6036-6072` | grep |
| AC2 adapter / impl | `:562` / `:585` | hold (`write_scalar` `:586-647`) | read |
| AC2 allowlist / doc | `:513-532` / `:506-511` | hold | read |
| AC2 tests asserting no port | `:14111`, `:14404-14424` | `story_9_6_port_requirement_uses_halt_transport_not_posture` `:14322-14356`, `story_9_6_halt_port_non_allowlisted_tag_returns_false` `:14616-14633`, `story_9_6_halt_port_orchestrator_deterministic_no_port` `:14635-14650` (WT +86); plus integration `crates/maos-bin/tests/maos_run_boot_loud_8_11.rs:86-120` (`boot_loud_port == false` for the Orchestrator, `:115-119`) | grep |
| AC2 manifest rule | `spirits/orchestrator/manifest.toml:40-43` | `dispatch_ambiguity` `:50-53`; `delegation_confidence` `:55-58` (no writer anywhere: `git grep delegation_confidence -- crates spirits xtask` = the manifest + one test doc comment) | read |
| AC2 `with_scalar_port` grep | 0 in `spirits/orchestrator/src/lib.rs` | holds (0) | grep |
| AC2 `judge_ambiguity` | (19-1 AC3) | `spirits/orchestrator/src/lib.rs:396-445`; production callers 0 (`git grep 'judge_ambiguity(' -- crates` = 0) | grep |
| AC2 emitter / const | `maos-spirit-hello/src/lib.rs:190` / `shell_host.rs:35` | hold | read |
| AC2 seeds | `lcas.rs:84,124,164` | hold; ALSO 24 occurrences in the committed corpus `tests/corpora/lcas-v0.3.jsonl` (sha-pinned `tests/corpora/MANIFEST.toml:45-46`; `RULE_VERSION` "bump on any seed change" `lcas.rs:25-26`) — outside the epic's `crates spirits xtask` grep | `git grep -c` |
| AC2 halt tag actually journaled | "`task.acceptance_criterion.ambiguous`" | the scalar path journals `payload.tag` = the scalar tag (`policy_runtime.rs:134-141`; TL intent `halt/mod.rs:554-561`) — i.e. `dispatch_ambiguity` | read |
| R19-P9 `HaltPending` | `hook_dispatch.rs:43-44`, fired `:281-285` | `:43-44`, fired `:283-285` | read |
| R19-P9 "the loop must consult the pending halt" | open | **done by 19-1** (D-19-1-09 A): `dispatch_stop` `orchestrator_root.rs:201-225` called `main.rs:4868-4878` (WT `:4887-4897`), `epic_halted` `:4876` | read |
| AC3 renderer | `butler/src/lib.rs:405-475`; `render_at` `:462-474`; `render_scene_day` `:438-444` | `:405-491`; `render_at` `:462-490`; `render_scene_day` `:438-444` holds | read |
| AC3 root renderer | `main.rs:3987` | `main.rs:4015-4032` (WT `:4018`) | grep |
| AC3 digest field | `completions[].source_log_ref` | the field is `completed[]` (`MorningDigest` `butler/src/lib.rs:706-715`; `CompletedTask.source_log_ref` `:681`; `OpenHalt` `:688`) | read |
| AC3 other | `:1677`, `j0.rs:83`, `journey_butler.rs:145`, `Cargo.toml:86`, `manifest.toml:21`, `main.rs:500`, FR17 `functional-requirements.md:51` | all hold | read |
| AC4 tests / guard | `journey_j1.rs:78,233,276`; `guards_meta.rs:24-25` | hold (guard fn `:23-26`) | read |
| AC4 job | `discipline.yml:3329-3368`, T3 `:3344`, pre-build `:3358` | `journey-hermetic-tier-1` `:3568-3602` (T3 `:3583`, pre-build `:3596-3597`, `cargo test -p maos-journey-test` `:3601`; WT +3) | grep |
| AC4 "the CI job sets `MAOS_REPLAY_STRICT=1`" | job-level env | ineffective for `journey_j1`: its child is `env_clear`ed (`production_command` `journey_j1.rs:28-42`); STRICT must be set on the child | read |
| AC5 `halt resolve` | `cli.rs:732-753`, kinds `:756-764` | `Resolve` `:780-802`, `ResolutionKindChoice` `:804-812` | read |
| AC5 oracle comment | `journey_j1.rs:363-368` | holds; `TrajectoryRef` / `cited_refs` exist nowhere (`git grep` = the comment only) | grep |
| AC5 provided context | (unstated) | written to the Spirit's private tier `halt_context::<halt_id>` by `KernelHaltResolver` (`halt/resolver.rs:165-181`); resolution journaled as an approval decision `capability = "halt.resolve"` (`halt/mod.rs:51-80`), no frame | read |
| AC5 registry | "daemon-held" | in-process only (`HaltRegistry` `halt/mod.rs:100-112`; no rehydration API) | read |
| AC5(b) `J1_TEST_SUFFIXES` | `check_j1_loopback_delegation.rs:121` | `:129` (WT `:133`); dir `MAOS_BIN_TESTS_DIR` `:113`; D-2 rule in `derive_enrolled_targets` `:815-860` (rule `:839-855`); WT +6 | grep |
| AC5(b) planted vector | `j1_crosshost_1b_proven_red.rs:620-633` | `planting_an_unenrolled_j1_test_file_must_red` `:632-649` | grep |
| AC5(b) blind spot | `_2c.rs` lane "neither enrolled nor named" | **already enrolment-gated** by `check-j1-two-host-signed-run` leg 10 `two-host-vectors-enrolled` (`check_j1_two_host_signed_run.rs:1044-1156`, derives `crates/*/tests/*_2c.rs` + `t_2c_*.rs` `:83-89`; Blocking `gate-registry.toml:481-483`; proven red `an_unenrolled_story_test_target_reds` `xtask/tests/j1_crosshost_2c_proven_red.rs:1015`). Residual: the loopback derivation walks only `crates/maos-bin/tests` (`:113`), so `crates/maos-iac/tests/transparency_duplicate_discrimination_2b.rs` (named by no J1 job) and `crates/maos-cli/tests/cohort_sign_2e.rs` (no gate) are underived | grep over `.github/workflows/*.yml` |
| Tracker precondition (`:197`, R19-24) | `deferred-work.md:1010` / `:931` open | `:931` **closed by 19-7** — f4_pairing V4 `restart_refusal_is_repinned_by_operator_consent_then_delivered` `crates/maos-bin/tests/f4_pairing.rs:1174`, proven red 40671241869, landing 40687349154 @ `309e277a` | read |
| E19 exit CI job | "the epic's first CI job" | **none exists**: no workflow runs `demo-j1` (`grep -n demo-j1 .github/workflows/*.yml` = 0; only `cargo test -p xtask demo_j1` `:3373`) | grep |
| kloc pins | `kloc.toml:517,339,718` | hold: maos-bin 30380, xtask 47451, `_aggregate_hardfail` 189878 | read |
| kernel pin | `kernel-core-baseline.toml:505` | `src_lines = 26378` (`:505`) | read |
