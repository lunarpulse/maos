# 20-11 landing1 review-correction report — Dev2011b, 2026-11-05

## Delivery boundary

Baseline HEAD: `93d39cc7ca185f513dee2d97bed2058f11554d9a` (amended20-2 handoff); original L1: `cbed4dad`. This is the kernel-free L1 correction: AC1/AC2/AC4/AC7 implementation and controls, bounded-work/door/receipt implementation for AC5, and the real live/T3 test path for AC6. **AC5/EC-13 remains OPEN**, as Main ruled: hosted stable-runner <25% wall and actual1M completion are still owed. Actual signed T3, delegated cgroups, rootless runtime and network-namespace proof are unverified locally. No AC3/KS/O4 or private/index cross-store lease repair is claimed.

No commit, push or tracker-status flip. Original T0 files, the unload witness `.rs.txt`, prior reds and historical landing reports remain intact. Older “final” reports are historical, not a replacement for this correction's observed results.

## Changes and why

- **Identity/inventory/cascade:** stable owner ids plus IAC boot/pid/admission-at-row qualification; ambiguous numeric owners fail closed. Durable subject scope survives a partial uninstall. Real late held/malformed observations drive terminal/package decisions; owned malformed bytes erase only under the ruled attribution, ambiguous/held bytes retain named gaps. Whole cascade/finalization executes inside the blocking boundary.
- **Linearization/bounded work:** finite entry high-water, actual4096-row pages, single-snapshot derivation closure, bounded late passes and final transactional TL marker. No pre-marker matching erasure-eligible body is claimed complete without that marker. Post-marker data and retained audit/numerical metadata are explicitly outside that claim. FairMutex handoff is non-poisoning; genuine unwind controls required RAII rollback and commit-ordered last-frame publication, not suppression of I2 panics.
- **Proof/verification:** exact16-category v2 DTO, coupled observations, exact typed weighted removal evidence and disjoint real frame ids, mandatory subject exclusions/count, signer-secret commitments, trusted-root inclusion, strict paired Rust/Python parsing/canonicalization and128MiB bound. Owner-only no-clobber durable publication retains truthful partial evidence. Smoke/collective callers are migrated; no compatibility shim.
- **Recovery:** complete streamed generation/index preimage survives deletion, lifecycle/I10 and final proof; explicit rollback errors reach callers. Package/proof failure does not reinstall torn bytes or discard the sole partial receipt.
- **Hosted A/B/C:** predicate-only kind/request indexes with new names, old full indexes dropped, readers migrated; retained stdin for the bare shell daemon; real100k/1M1KiB fixtures with bounded seed WAL, recursive footprint accounting, serial large-fixture group and prompt explicit normal-path cleanup. No perf baseline/tolerance, route/observation deadline, assertion or append durability is loosened.
- **Regression preservation:** stable-owner test binding and authentication of both provisional/terminal receipts unmasked genuine old-oracle failures; typed failure classes, independent held-metadata attribution, exact key/private reasons and regeneration-only Pending semantics preserve them. The full workspace then found an overbroad prior-source SQL OR: partial numerical redactions must remain citable. That generic IAC predicate is repaired, without changing the kernel oracle or kernel source.
- **Docs/CI:** hash-pinned Ed25519 backend provisioned in both jobs, PYTHONPATH retained through the cgroup filter with real child-import/missing-backend controls. ADR-044 and EN/KO operator docs explain marker/pending scope, receipt selection, numerical facts, kernel gaps and exact API-only late-request replay. There is no new retry API/policy/CLI or automatic redrive.

Principal changed areas are audit proof/Merkle/classifier/lib and tests; IAC adapter/sweep/scan/support/index tests and dependency edge; bin cascade/door/pump and inventory/live/scan/shell/13.5b/support tests; registry storage/recovery tests; collective verifier caller; Python verifier/requirements, provisioning action/workflow/cgroup filter/xtask runner tests; nextest grouping; story, deferred rows, coverage, ADR/operator docs and six KLOC rows. The story File List retains the detailed paths; the Korean mirror is `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md`.

## AC witness/control table

Coordinates are current test declarations, not historical reviewer offsets. Full before/control evidence: `before-state.md`, `final-control-results.md`, `current-cites.md` and adjacent exact-command sidecars.

| AC | Witness name:line | Before / deliberate plant | Final observation |
| --- | --- | --- | --- |
| AC1 | `erasure_inventory_20_11.rs:151` `ac1_principal_free_distillate_gets_a_signed_proof` | Principal-only guard plant returns NotFound/4 for real lineage; `final-plants-1.stderr:762-792`, exit100. | Current346-case focus passes; actual standalone/prepush inventory cases pass. Populated-log unknown-name Failed/5 contract also preserved. |
| AC2 | `erasure_inventory_20_11.rs:387` `ac2_principal_and_principal_free_lineage_are_both_erased`; ownership cases552/557/562 | Literal VerifiedEmpty plant yields Null rather than count1 (`final-ac2-plant.stderr:606-609`, exit100). Genuine cross-boot/same-boot/legacy-owner reds in `before-state.md`. | Current346 focus passes; both byte erasure and real evidence verified; held/ambiguous cases remain retained. |
| AC4 | `erasure_inventory_20_11.rs:205` installed package/schedule; `shell_host_16_2.rs:1760` real SCB snapshot | Omit pre-unload snapshot: [] rather than [daily] at1794 (`final-plants-1.stderr:827-830`, exit100). Real deletion/journal/index-recovery before witnesses retained. | Package/schedule/recovery focused cases pass; actual SCB case passes both required workspace legs. |
| AC5 bounded work | `erasure_scan_work.rs:23,28` dense100k / sparse1M-rowid100k-row VM vectors | Literal production `i64::MAX` window plant makes one page consume all work, ratio1 at112 (`final-plants-1.stderr:866-902`, exit100). | All114 IAC cases pass in final346 focus; unchanged quarter-work and independent absolute cap remain. This is not a1M-physical-row substitute. |
| AC5 wall/completion | `shell_host_16_2.rs:1503,1508` continuous writer / real1M in-flight completion+repeat | Genuine shared-host wall failures remain, including0.2501131127; no rerun-to-green or widened deadline. | Standalone100k passes0.1164494388, completion/repeat/bytes/cleanup. Real1M misses unchanged observation in standalone90.189s and once-only prepush42.312s. **OPEN under20-11;20-7 stable-runner half.** |
| AC6 | `erasure_live_20_11.rs:308` actual signed T3 Worker; native companion304 | Hosted41962667293 observed EOF/connection-reset exit69; daemon now retains piped stdin. CI=true/no-runtime local attempt fails its real prerequisite, not a feature oracle. | Native live-door companion passes. Actual T3 execution unverified locally; native unsupported-runtime return is not counted as hosted proof. |
| AC7 | `erasure_inventory_20_11.rs:224` `ac7_archive_and_authored_skill_are_named_and_counted` | Isolated omission of populated skill_queue reds count at256 (`final-ac7-plant.stderr:616-617`, exit100); masked earlier combined AC7 red excluded. | Current346 focus passes; real retained archive/skill observations are named/counted, not invented removals. |

Additional genuine final regression: unchanged `crates/maos-kernel-core/tests/forget_cascade_18_6.rs:429`, assertion447. Before: unrelated forget erased a neutral derivative of partial cost data (`workspace-final-default.stderr:29810-29839`). Repair removes the broad partial-redaction OR from IAC only. Reintroducing that exact OR reds the same assertion (`partial-redaction-plant.stderr:476-495`, exit100); copy-back + cmp + touch exits0. Current346 focus and post-restore13/13 cascade cases pass. The kernel test/source is unchanged. P9's four real root byte/proof cases at `tests/support/uninstall_review_20_11.rs:430,435,440,445` also pass.

## Exact measurement versus pins/envelopes

Measured after `cargo fmt --all` using exact gate-style tokei arguments. Authority: `exact-source-partial-redaction.json` and raw/command receipt. All six moving pin histories are repinned together; the xtask first HISTORY segment cites the live story with `->`.

| Scope | Handoff pin | Final measured/pin | Delta | Governing ceiling |
| --- | ---: | ---: | ---: | ---: |
| maos-bin |34823|34914|+91|35704|
| maos-iac |8551|8622|+71|8630|
| maos-audit |7785|8174|+389|8261|
| maos-registry |4322|4412|+90|exact re-base4412, authorized cap4430|
| xtask |44673|44673|0|44673|
| aggregate measured |194099|194740|+641|195404|
| aggregate hardfail |194100|194741|+641|measured+1|

Kernel remains26841 physical/21042 tokei; domain10107. Every other per-crate count delta is0. Source-only KLOC/kernel/service gates are refreshed after the final SQL repair, all exit0.

**Registry:** Main's expressly ruled **“review-correction re-base: real production growth (streaming-tar recovery + rollback through proof/I10)”**,4322->4412, former4368 exceeded44; no registry test relocation, no other ceiling increase.

**IAC accounting transfer, not a ceiling relief:** src/tests8551/2315 ->8622/3577. Only the original unit body remains external,506->504 from four expressly approved FairMutex caller adaptations; the168-line VM module returned verbatim to src. No-transfer counted src9126 (+575), aggregate195244; not every counted added line is runtime production code. Ceiling8630 unchanged. Original107 names identical around transfer,111 around approved lock adaptation, then three unwind tests make114; narrow return has identical114 names/count, all executed. See `accounting-transfer-final.json` and identity ledgers.

## Exercised verification (not inferred)

Every raw `*.command.json` records exact argv/env/status/duration. Common structural prefix: `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask $CARGO_TARGET_DIR/debug/xtask <gate> --json`. CARGO_TARGET_DIR is inherited, never reassigned.

- `cargo fmt --all`:0. Exact tokei measurement:0.
- Final focus: `env CARGO_BUILD_JOBS=8 cargo nextest run --locked -p maos-bin -p maos-iac -p maos-audit -p maos-kernel-core --lib --bin maos --test erasure_uninstall_13_5b --test erasure_inventory_20_11 --test erasure_review_20_11 --test erasure_smoke_test --test forget_cascade_18_6 -E 'package(maos-iac) | package(maos-audit) | binary(erasure_uninstall_13_5b) | binary(erasure_inventory_20_11) | test(uninstall_review_20_11) | binary(forget_cascade_18_6)' --retries 0 --no-fail-fast --success-output immediate --failure-output immediate`:0,346/346,49 skipped; test4.131s, command354.385s.
- Post-plant `env CARGO_BUILD_JOBS=8 cargo nextest run --locked -p maos-kernel-core --test forget_cascade_18_6 --retries 0 --no-fail-fast --success-output immediate --failure-output immediate`:0,13/13.
- Registered final `env CARGO_BUILD_JOBS=8 cargo run --locked -p xtask -- build-test-fixtures`:0,67.200s; then genuine `env CARGO_BUILD_JOBS=8 cargo build --locked -p maos-bin -p maos-cli`:0,0.499s. Previous genuine post-repair native build also0,26.143s.
- `env CARGO_BUILD_JOBS=8 cargo clippy --workspace --all-targets --locked --message-format json`:0,473.896s. `xtask clippy-ratchet --input .../clippy-partial-redaction-final.jsonl`:0;1329 occurrences/870 distinct versus1383/897, delta-54/-27. Baseline unchanged; warning debt remains, not warning-free.
- `cargo deny --locked --format json check`:0; once-only prepush cargo-deny also passes. Existing parking_lot0.12.5, no new crate version.
- Fourteen affected structural gates pass: kernel baseline, KLOC, service boundary, dev record, epic coherence, exit commands, decision register, serde errors, public field constructors, coverage completeness, composition root, env contract, test cost, breaking record. Kernel/KLOC/service and records are refreshed after relevant repairs. The initial dev-record formatting failure1 and repaired0 remain, not labelled flaky.
- Metadata producers:58 members/669 default targets (56lib/596test/2proc-macro/15bin). Two2900s build-only cells interrupted without test execution; explicit compiler jobs8 completes0 in762.297s. No target filter or test-thread override. Earlier exact-filter exit94/custom-benchmark all-target listing exit104 are documented setup failures, not passing oracles. See `metadata-producer-build-cells.md`.
- **One actual unfiltered standalone workspace:** fresh Cargo/binaries metadata reuse, no package filter, default runtime concurrency, retries0. Exit100,5688 run/5686 pass/2 fail/138 excluded, test152.350s/command201.394s. Genuine partial-cost regression repaired as above;1M timing/completion stays open. No second standalone run.

### Once-only prepush

Exact native invocation, via the evidence wrapper:

```sh
env -u CI -u GITHUB_ACTIONS \
  CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask \
  MAOS_17_2_CGROUP_MODE=undelegated NEXTEST_RETRIES=0 \
  PYTHONPATH=/tmp/dev2011b-review.9OTbuj/python/lib/python3.13/site-packages \
  CARGO_BUILD_JOBS=8 "$CARGO_TARGET_DIR/debug/xtask" prepush --json
```

**Exit1,1195.222s; no rerun.**84 pass records,2 red,1 timing-budget advisory,5 local marker-cardinality skips. Tracked records were frozen while it ran. Same-run JUnit copied before any subsequent tests:5688 executed,5687 pass,1 failure,0 JUnit-skipped. JUnit's skipped field is not a claim that unsupported hosted substrates executed.

- Workspace red is solely the unchanged1M observation deadline (42.312s, pending route10.000086555s). The repaired partial-cost kernel oracle passes0.372s; watched refused-successor cold-swap sibling passes1.029s.100k door case passes6.511s; its numerical ratio is not stored in the prepush JUnit, so no numeric ratio is invented.
- Other red: EN/KO glossary drift (`Spirit`7 versus4). The actual affected Korean sections are now translated, not keyword-padded. Focused glossary, bidi, anchors, docs build and routes each exit0 from docs-site. A first focused call from the wrong repository cwd failed MODULE_NOT_FOUND; it is an excluded setup error, not another feature witness. The original prepush red is retained, not renamed green.
- Five skips are dynamic CI marker cardinalities: SCOPED-VERDICT, GOVERNED-ATOMIC, HOSTILE-LIFTING, HOSTILE-LIFTING-CONTROL, RUNNER-SIGSYS. Hosted runtime guards remain authoritative.
- Kernel lib, workspace doctests, cargo-deny and docs build pass in that prepush. Generated lineage report restored from its pre-verification user preimage with copy/cmp/touch exit0.

Same-run unchanged30s aggregate advisories, explicitly recorded under20-7 with rule-10 same-line evidence (no new budget/exception/retry):

| Binary | Cases | Sum seconds |
| --- | ---: | ---: |
| erasure_uninstall_13_5b |26|31.188|
| hook_pump_17_4 |26|32.233|
| one_daemon_one_door_16_1 |42|35.469|
| shell_host_16_2 |14|58.484|
| participant_recall_17_8 |29|42.060|
| xtask::bin/xtask |463|30.597|

## Fixture/performance observations

Recursive accounting includes the entire fixture root, not only SQLite.4096-row seed commits capture DB+WAL high-water before truncation; large vectors cannot overlap each other. The1M physical vector/payload is unchanged.

| Vector | Old doubled allocated peak | Bounded seed allocated peak | Post-operation observation |
| --- | ---: | ---: | --- |
|100k1KiB|297504768|151810048|standalone152236032 allocated/152235512 apparent; explicit close/absence assertions pass|
|1M1KiB|2976133120|1487568896|seed apparent1487572544; standalone/prepush completion deadline prevents post-peak/close assertions|

No full1M completion/cleanup assertion is claimed from a seed measurement. The smaller VM-only fixture is100k physical rows with a sparse1M rowid companion; it does not replace the real1M door vector.

Hosted A's original callgrind1049938.8 versus997449.662 (+5.3%,2% tolerance) is not locally rerun. Valgrind unavailable (command lookup exit1). Cheap20k ordinary-append proxy: full indexes860 pages/3522560 bytes versus partial742/3039232; VM steps both1640000. This is page/work evidence, not an instruction-count or <2% verdict. Baseline/tolerance untouched; hosted callgrind unverified.

## Remaining boundaries

- **AC5/EC-13 OPEN:** stable-runner wall plus real1M completion;20-11 owns acceptance,20-7 timing/I/O half. No retry-to-green or widened budget/deadline.
- **Hosted-only unverified:** actual signed T3, delegated cgroups/rootless runtime/netns, native callgrind and dynamic marker-cardinality proofs.
- **Final kernel:** AC3/KS/O4 and the private/index hold lease. IAC transaction ordering is fixed; frozen kernel deletion interval still lacks that lease. Forecasts in before-state are explicitly unmeasured/ungranted; do not borrow KS/O4 allowance.
- **Named accepted limitations:** independent same-UID post-marker writers; retained audit/lineage/numerical metadata; final-tail Pending/Refused(late-frame) with API-only explicit replay outside uninstall, no operator/automatic redrive; purge-proof retention policy. Ownerless rows have recorded dispositions/triggers, not phantom owners.

The durable TL marker is the completion boundary, not handler return. Signed body-removal evidence never means completed regeneration or erasure of every retained substrate. All controls restore exact source; no plant/profiling hook remains.

## Final records and cleanup

Immediately before cleanup, final dev-record completeness, epic-close coherence and exit-command gates each exit0 (`gate-handoff-*.json`). No build, test, formatter or gate runs after cleanup.

Cleanup command `python3 /tmp/dev2011b-review.9OTbuj/cleanup-final.py`, via the evidence wrapper, exits0 in18.967s. Its `/proc` guard observes no cargo/rustc/cargo-nextest. It deletes the owned scratch/venv/wrappers/preimages and486 stale incremental `s-*` sessions, retaining7519 newest-per-crate/config sessions and1335 additionally recent sessions (any mtime/ctime in the last2h). No deps/build/fingerprint, other worktree or `/tmp/2011-wip` is touched. Scratch absence is recorded.

| Accounting | Bytes |
| --- | ---: |
| Logical unlinked total |6064487193|
| Incremental logical unlinked |6020330686|
| Owned scratch logical unlinked |44156507|
| Unique last-link allocated total |5121658880|
| Incremental last-link allocation |5074714624|
| Scratch last-link allocation |46944256|
| Observed Cargo-volume available-space gain |3897225216|
| Observed /tmp-volume available-space gain |0|

The allocation counter deduplicates hardlinks and excludes inodes with surviving links; it is not an assertion of physical reflink sharing or attribution of concurrent filesystem changes. The observed free-space gain is reported separately rather than falsely equating st_blocks with physical space freed. Full selected paths, guards and before/after statvfs data are in `cleanup-final.json`.

The temporary Python environment is intentionally gone after verification. To reproduce the standalone Python verifier, provision the declared hash-pinned requirements as documented; do not depend on the deleted scratch path from the historical command receipts.
