# Hosted follow-up42224167600 — implementation and verification record

Base HEAD: `fd4f60f15631ba09464b51024bce2c60236b8245`. No commit, push, tracker-status change or kernel modification. The original T0 and prior review-correction evidence remain intact. Current source is identified by `source-state.sha256`; actual command argv, exit status, duration and verifier environment are retained beside the raw logs.

## Four hosted corrections

1. **Native routing cost.** Compared the real compressed callgrind inputs from [42224167600](https://github.com/lunarpulse/maos/actions/runs/42224167600) and [41910498869](https://github.com/lunarpulse/maos/actions/runs/41910498869). After subtracting the36-instruction empty process, the reference/current summaries give997139.499/1038630.1585 instructions per iteration, a41490.6595 delta. SQLite prepare accounts for30924.7995 (74.53%) of that delta; FairMutex lock/drop is187 versus228, not the regression. Production now reuses the actual indexed INSERT program inside the unchanged RAII writer transaction and publication boundary. `review_cached_frame_insert_reuses_the_indexed_program` proves two real appends reuse that program; before/plant RUN0 becomes restored RUN2. No baseline/tolerance or fairness/durability change. The fixed budget baseline remains997449.662 and2%; local callgrind is unavailable, so the post-change instruction metric is **UNVERIFIED**, not inferred from elapsed time or allocations.
2. **Paired allocation guard.** The actual native harness caught rusqlite0.31's per-cache-hit Arc<str> key allocation:44483/2000=22.2415, above the unchanged21.6648 ceiling (baseline21.24×1.02). Only the private accepted-address buffer is now inline SmallVec capacity1; the already-locked smallvec moves from dev to normal dependency, with no new version. Fan-out spills and rupture wire payloads remain owned Vec. The temporary ids Vec remains unchanged. The Vec plant gives44478/2000=22.239 (oracle exit1); restored production gives42481/2000=21.2405 (oracle exit0), after an earlier positive42479/2000. This is a real paired allocation oracle, **not instruction-count proof**.
3. **Apple FIFO compile portability.** Replaced the Apple-excluded rustix mkfifoat call with safe POSIX `mkfifo -m 600`, preserving cfg(unix), forbid(unsafe_code), the real FIFO barrier and its failure-closed assertions. The native preservation test passes. The installed Apple target's required cargo check was actually attempted and exits101 in ring's C build: GNU cc rejects Apple flags and clang/zig are absent. Apple compile/runtime remains **UNVERIFIED locally**; no Linux pass is presented as Apple proof.
4. **Durable completed replay and aggregate budget.** Canonical decoded principal/optional reason identify the existing indexed completion snapshot. Lookup is inside the started handler, not submit; the10s route deadline remains unchanged. A new transport handle/audit completion links the original erasure id while the response preserves the original completed operation id, marker and proof. The snapshot is typed streamed byte receipt metadata, not application-body retention or JSON-per-byte Value construction. Actual marker validation, current holds and at most4096 tail rows gate replay; new eligible subject bodies, any new distillate or the cap force a fresh forget. Non-TL/private/index state is not re-inventoried by a historical replay; a distinct existing reason requests fresh inventory. A genuine composite DESC dependency bug was repaired while the exact old ascending projection oracle remains intact. The two physical100k/1M vectors move verbatim to a serial heavy binary; no size,1KiB payload,25% latency or route/observation deadline change. Main approved78s from hosted(5.646+56.675)×1.25=77.90125, while the other12 shell cases take3.919s. The existing Dev20242s exception is untouched.

See `control-results.md` for every observed before/plant/restored result, including the current-hold and tail-cap safety controls, and `current-cites.md` for current name:line citations. Plant A is2 IAC failures plus1 replay failure; plant B is3 safety failures; allocation plant is the real metric failure. All restored sources use observed exit0 copy-back+cmp+touch. No plant, weakened oracle or fake fallback remains.

## Exact final source envelope

| Account | Handoff pin | Final measured pin | Delta | Unchanged ceiling |
| --- | ---: | ---: | ---: | ---: |
| maos-bin |34914|35102|+188|35704|
| maos-iac |8622|8625|+3|8630|
| maos-audit |8174|8174|0|8261|
| maos-registry |4412|4412|0|4430 (prior ruled re-base)|
| xtask |44676|44676|0| existing pin/grant unchanged |
| aggregate |194743|194934|+191|195404|
| aggregate hardfail |194744|194935|+191| measured+1 |
| kernel tokei / physical |21042 /26841|21042 /26841|0| untouched |
| domain |10107|10107|0| untouched |

Actual final tokei command and stats are in `source-accounting-command.json`/`source-accounting-final.json`: bin src35102/tests64146 (37/115 files), IAC src8625/tests3640 (18/14 files). The original approved unchanged504-code IAC test transfer remains an **accounting transfer, not ceiling relief**: src8551→8625 (+74 measured), tests2315→3640 (+1325), no-transfer production9129 (+578), counterfactual aggregate195438 (34 above195404). Actual aggregate194934 remains within the unchanged ceiling; no further source-to-tests transfer or ceiling grant occurs. The new heavy-vector extraction is tests-to-tests only. The first measurement in before-state.md is explicitly preserved as historical, not the final pins.

## Completed focused/build checkpoints

- `cargo build -p maos-bench --bin tier_c_harness --features kernel_measurement --locked`: restored exit0; actual native `tier_c_harness iac_routing 2000`: exit0; real budget reader: exit0 with21.2405 allocations/iteration. The red oracle/plant outputs are retained separately.
- `cargo nextest run -p maos-iac --lib --locked --no-fail-fast`: final inline-buffer/restored source exit0,116/116, including actual VM-work, sparse-rowid, unwind/publication, held malformed and independent-writer marker tests. Same-run JUnit preserved.
- Restored bin checkpoint: exit0,7/7 (four replay branches, paired verifier, real FIFO and real SCB snapshot),27 unselected. This selected checkpoint is not substituted for workspace verification.
- The first integrated34-case checkpoint was29 pass/5 fail: two genuine latest-report ordering bugs repaired in production; missing declared cryptography backend repaired with the existing hash-locked provisioner and the runner's Python via PYTHONPATH; an unchanged REPL clarification timing occurrence; and the still-open1M first pending-completion observation. Its raw failures and JUnit are retained, not erased by later positives.
- `cargo run -p xtask --locked -- build-test-fixtures`: exit0, then genuine `cargo build -p maos-bin -p maos-cli -p xtask --locked`: exit0. No fixture-only build is misrepresented as the genuine affected binaries.
- `cargo clippy --workspace --all-targets --locked --message-format json`: exit0. Actual unchanged-baseline ratchet exits0 at1329 occurrences/870 distinct, versus hosted1383/897: delta−54/−27, unchanged from the preceding correction. No clippy baseline edit or new crate version.
- Initial full-workspace metadata producer hit the3000s cell during compilation, after clippy-ratchet0 and cargo-metadata0. No test case executed. The tool supplied no numeric child exit status; `workspace-metadata-timeout.json` records null rather than fabricating124. The post-timeout /proc scan found no cargo/rustc/clippy process. Compilation resumes in a distinct bounded producer cell, preserving the original partial stdout/stderr; this is not a test retry or configuration/deadline change.

## Evidence boundaries

AC5/EC-13 remains **OPEN under20-11** until the complete hosted1M vector demonstrates completion, durable repeat, actual post-operation footprint/close and the unchanged wall ratio. The local baseline82.923s and integrated57.922s failures occur before that repeat assertion; neither is called a reproduction of the exact hosted repeat failure or a demonstrated final1M cleanup. The observed1M seed allocation1487568896 bytes is a seed measurement, not the unobserved final peak. Hosted first completion15.54s/ratio0.0019 is distinct; its repeat was red. Timing/I/O occurrences have same-line rule-10 owner20-7 dispositions, with functional ownership20-11 retained.

Hosted native instruction counts, Apple compile/runtime and delegated-cgroup/rootless-runtime/network-namespace T3 legs remain locally unverified. AC3/KS/O4, watchdog cleanup and the private-tier/index hold lease remain final-kernel work. The durable transactional marker, not handler return, is the erasure boundary; post-marker data is not claimed erased. Same-UID independent writers without a barrier and API-only explicit replay of durable pending late-redistill requests remain the already-recorded named limitations, with no automatic redrive, cross-Spirit dispatch or new retry CLI/API. No tracker flip is warranted.

## Completed metadata preparation and test-name preservation

The distinct continuation producer (`cargo nextest list --workspace --locked --message-format json`) exits0 in1144.365s, after the original3000s compilation cell; no tests are retried in either producer. The actual reusable metadata contains5846 listed cases across670 binaries. `test-name-preservation.json` proves all15 names captured before extraction remain byte-identical, with no duplicates or removed names: shell12 plus heavy6 equals18, and the three newly added matching-state/tail/hold controls explain the increase. The captured15 already included the first new durable-replay control; all original14 names therefore remain. The physical100k/1M and first replay control intentionally move binary id, not function name or assertion behavior.

The initial run invocation exits2 before any test execution because this installed nextest correctly forbids Cargo build selectors (`--workspace`, `--locked`) together with `--binaries-metadata`. The raw usage failure is retained as `workspace-final.log`; no test failure is mislabelled or retried. The execution invocation removes only those incompatible build selectors and consumes the complete **locked workspace producer's** Cargo/binary metadata, with no package/test filter and no concurrency/retry override. This is full workspace reuse, not a narrowed suite or an unlocked rebuild.

Affected EN/KO documentation preflight: glossary, bidi, anchors, actual npm build and routes each exit0 (`docs-*-prepush` command records). The real build takes13.437s. No keyword padding, glossary waiver, generated route bypass or unrelated42s exception edit.

One further **pre-execution** reuse-schema error is retained, not hidden: the selector-free invocation exits96 because nextest requires the native `rust-binaries` schema, whereas the successful full test-name list has `rust-suites`. The producer is therefore invoked with its supported `--list-type binaries-only`; the original full list remains the authoritative name-parity input. Neither the exit2 argument rejection nor this exit96 deserialization rejection executes a test case. The actual workspace execution uses the native binaries-only schema, the same locked complete workspace build and default concurrency/filter, not fabricated/conversion metadata or a selective rerun.

## Standalone default-concurrency workspace — observed, not green

The actual native binaries-only producer exits0 in15.251s. The single standalone test execution consumes that whole locked workspace metadata with default concurrency and no package/test filter: exit100,5708 run,5702 pass,4 fail,2 time out,138 default-filter skips. Same-run JUnit time181.202s; captured command wall241.112s. No ad-hoc test retry follows; the separately prescribed once-only prepush and all its outcomes are recorded below, without retiring these failures.

| Failure / timeout | Observed result | Disposition |
| --- | --- | --- |
| cert_rotation_trigger_14_2a::t_14_2a_the_installed_production_closer_closes_the_window_and_journals_the_close |7.911s, operator read WouldBlock/EAGAIN |20-7 timing triage; source oracle unchanged, binary changed; no cause asserted. |
| hook_pump_17_4::one_budget_overrun_quarantines_the_subscriber |7.775s, delivered1/failed0 instead of delivered0/failed1 |20-7 existing timing family, no weakened quarantine oracle. |
| erasure_million_20_11::ac5_million_row_pending_forget_finishes_and_repeats_without_corruption |120.006s timeout, no seed/final-peak marker in captured output |20-7 timing/I/O half,20-11 functional AC5 remains open. |
| observer_activity_corpus_17_4::observer_activity_corpus_production_path |120.023s timeout |20-7 existing timing/I/O family, no timeout/budget change. |
| one_daemon_one_door_16_1::ac4_root_e_a_faithful_successor_survives_the_hot_swap |25.195s, infer-frame count stays1 |20-7 timing triage; not the watched refused-successor case, which passes0.881s. |
| forget_floors_18_6::fifty_door_cascades_and_one_hundred_all_store_probes |93.498s, floor048 gets typed route pending, never erased/complete |20-7 timing/I/O triage with20-11 functional ownership retained; unchanged10s route and old success assertion. |

`workspace-failure-briefs.json`, raw output and same-run JUnit preserve the exact diagnostics. Every occurrence has a same-line rule10 row in deferred-work.md. No unchanged-target claim, proven-cause claim, suppression, serial retry or timing/budget widening. A same-run-id /proc survivor scan finds no owned maos/heavy-test process left running.

**Real100k physical vector:** route10.000123606s returns pending, handler completion observed10.548058812s, max single insert2.538956458s over231 writes, wall ratio0.24070366910654267 (<0.25). Seed allocated peak151810048; actual recursive final allocated peak152236032 bytes. The pass includes completed identical replay, exact tombstone/unrelated bytes and explicit store-close/TempDir-absence assertions. These are raw uninstrumented production wall values. The1M timeout does not demonstrate any final footprint/cleanup or repeat pass; its earlier independent seed1487568896 remains a seed observation only. No ratio is invented for the timed-out1M case.

### Current AC witness table

| AC | Current witness name:line | Before / plant | Current observed result |
| --- | --- | --- | --- |
| AC1 | erasure_inventory_20_11::ac1_principal_free_distillate_gets_a_signed_proof:151; paired-category verifier:266 | Original real baseline and isolated restored plants retained under review-corrections/final-control-results.md. | Both pass this workspace; declared real Python backend, no evidence-count weakening. |
| AC2 | erasure_inventory_20_11::ac2_all_boots_and_pid_reuse_are_isolated:188; principal/free-lineage:387; erasure_live_20_11::ac2_live_door_erases_principal_free_lineage:304 | Original incarnation/lineage before+plant records preserved. | Current workspace passes; held derivation preservation:402 also passes. |
| AC4 | shell_host_16_2::ac4_door_snapshots_real_scb_schedules_before_unload:1180; inventory package:205/failure-closed proof-write:438 | Original omission/failure plants preserved, not inferred from package deletion. | Current workspace and restored selected SCB case pass; final-kernel watchdog gap remains named. |
| AC5 | erasure_million_20_11 physical vectors:14/19; durable replay:270; matching-state:352; finite-tail:397; current-hold:438 | New genuine before/plant table in control-results.md; original VM-window i64::MAX plants preserved. | Four small replay controls and100k full physical completion/repeat/cleanup pass;1M timeout, full AC5 OPEN. VM-work vectors pass. |
| AC6 | erasure_live_20_11::ac6_running_t3_worker_uninstall_proves_live_inventory:308 | Hosted substrate control remains hosted-only; no fabricated local plant/proof. | No local T3 exercise claimed; hosted delegated-cgroup/rootless/network-namespace proof remains unverified. |
| AC7 | erasure_inventory_20_11::ac7_archive_and_authored_skill_are_named_and_counted:224 | Original populated archive/skill before and isolated omission plant retained. | Current workspace passes, unchanged typed inventory/count oracle. |

AC3/KS/O4 are intentionally absent from this L1 table: they are final-kernel work, not an implied passing row.

## Final source/document gates and aborted fixture lifetime

The sequential20-command bundle exits0: fourteen source/record gates, `cargo fmt --all -- --check`, and the five affected EN/KO documentation checks (glossary, bidi, anchors, build, routes). Exact source gates use `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /mnt/build/cargo-maos/debug/xtask <gate> --json`: check-env-contract, check-test-cost, check-composition-root-completeness, check-breaking-md, check-serde-error-handling, check-pub-field-constructors, check-coverage-matrix-completeness, check-service-boundary, kloc-check, check-kernel-baseline, check-exit-commands, check-decision-register, check-dev-record-completeness, check-epic-close-coherence. All individual argv/status/duration records are preserved; no gate alias is invented.

The timed-out1M process cannot run its normal TempDir destructor. A read-only immutable ownership probe joins the exact deterministic frame id/kind1/pid7/boot2011/scan.filler/1024-byte seed signature with the actual120.006s case interval. Its checkpoint max_rowid937988 is below physical1M; **[INFERENCE]** termination is during seed preparation, not evidence that the repaired replay ran. There is no owned same-run surviving process. Removed only that positively identified aborted directory before prepush:1396526824 apparent bytes,1396523008 allocated-file bytes,1294843904 observed filesystem free-space gain, directory absent. Other candidates are untouched. `aborted-fixture-{ownership,cleanup}.json` preserve the conservative initial candidate probe and final actual-case-interval attribution. This resource cleanup is not a passing1M footprint or destructor assertion.

## Once-only prescribed prepush and explicit dependency check

Exact command: `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated /mnt/build/cargo-maos/debug/xtask prepush --json`, with the already-provisioned real verifier's PYTHONPATH. Exit1 in2943.623s, within the3000s cell. Tracked records were frozen throughout; no second prepush. Complete structured output, command metadata, output diagnostics and same-run JUnit are preserved.

- 84 pass,1 red,1 advisory,6 skips. Sole red: workspace-nextest; latest same-run JUnit5707/5708 passing, sole1M first-completion observation at50.162s. Route remains pending10.000723955s; seed peak1487572544 apparent/1487568896 allocated bytes. Final1M peak, repeat and close assertions remain unproven. AC5/EC-13 stays OPEN, not hidden behind5707 positives.
- Seven unchanged-budget findings: butler::hallucination30.973, erasure_uninstall37.547, hook_pump38.154, maos_uninstall35.868, one_daemon_one_door39.011, participant_recall41.008s each versus30s; suite1173.224 versus1100s. Exact human diagnostics are extracted because the embedded one-line JSON was truncated; their sums match this preserved prepush JUnit, not the standalone cohort. Same-line rule10 owner20-7 rows record every finding; no new exception or budget widening. Heavy binary61.897s stays within the already-approved78s.
- Six skips: five local dynamic CI-marker cardinalities plus cargo-deny, whose prepush change selector only sees Cargo.lock/deny.toml. The normal smallvec dependency promotion therefore gets an explicit `cargo deny check`: exit0 in1.351s. No new version or deny-policy edit.
- AC6's exact runtime marker says signed T3 substrate is not provisioned and requires scripts/provision_t3_tests.py plus its t3.env. This is a named local skip, not hosted proof or a borrowed ancestor result.
- Generated intent-lineage report restored from its original saved preimage with actual copy-back+cmp+touch exit0. Production/source pins are unchanged after the complete source verification.

The independently required prepush is not an ad-hoc retry of the standalone failures. Its later positives do not close or erase those earlier same-line observations. Hosted instruction counts, Apple compile/runtime, the completeAC5 vector and T3 substrate remain the missing external evidence.

## Final record closure and cleanup

After the final prepush/deferred observations, check-exit-commands, check-decision-register, check-dev-record-completeness and check-epic-close-coherence each exit0 under the prescribed xtask environment; individual closing command records are archived.

The final archive+guarded cleanup command exits0. Its /proc guard finds no cargo/rustc/clippy/rustdoc/nextest process. Own `/tmp/dev2011b-perf.242wuR` is removed;544 stale `debug/incremental/*/s-*` directories are pruned, retaining the newest per crate/cache unit and every session with a modification within2h. Actual cleanup data:8111838200 logical bytes unlinked,6504443904 last-link allocated-file bytes,86898 files and709 directories removed; observed Cargo-filesystem free-space gain5914886144 bytes. `cleanup-final.json` preserves paths, guard, retention lists and byte accounting; `cleanup.py.txt` is the executed script's evidence copy, not a live scaffold.

The earlier positively identified aborted fixture separately freed1396526824 logical/1396523008 allocated-file bytes (observed /tmp filesystem gain1294843904). Combined logical unlink is9508365024 bytes; physical free-space gains are reported separately, not summed across measurements/filesystems or claimed identical to st_blocks on reflink storage. No deps/build/fingerprint, `/home/lunarpulse/wt-w3`, .devcontainer, `/tmp/2011-wip` or unidentified temporary fixture is deleted. No cargo/check is run after final cleanup.

## Changed files / handoff

- Production: `crates/maos-iac/src/adapter/{transparency_log,mailbox}.rs`, `crates/maos-iac/Cargo.toml`, `crates/maos-bin/src/operator_door.rs`.
- Tests: IAC `tests/support/erasure_races_20_11.rs`; bin `tests/{erasure_inventory_20_11,shell_host_16_2,erasure_million_20_11}.rs` and `tests/support/shell_world_16_2.rs`.
- Configuration/records: `.config/nextest.toml`, `xtask/{test-budgets,kloc}.toml`, story/Dev Agent Record, deferred-work.md, coverage matrix, ADR-044, EN/KO operator documentation and this evidence directory. The generated lineage report is restored, not a delivered change.

Implementation of the four follow-up findings is complete. Acceptance is deliberately not claimed closed: Main must land the source and obtain current hosted native instructions, Apple compilation, full1M completion/replay/wall proof and T3 substrate evidence. AC3/KS/O4 and private-tier/index hold lease remain final-kernel work. No commit/push/tracker flip; no local failure is hidden or retried to green.
