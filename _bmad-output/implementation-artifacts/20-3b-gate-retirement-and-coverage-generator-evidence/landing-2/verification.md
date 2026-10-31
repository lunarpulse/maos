# 20-3b landing-2 verifier/integrator — final evidence

**Current authoritative result:** see “Wasmtime 48.0.8 security patch verification” below. Earlier measurements, runs, the envelope STOP and the four-advisory deny failure are retained as history. The patched local build/tests/gates/prepush and all four cargo-deny checks pass; the recorded advisory, skipped scenarios and hosted-proof limitations remain explicit.

Tasks 1–7 completed locally, including the operator-authorized held-durable-value correction. Hosted execution remains deliberately unclaimed. Exact commands, working directories, exit codes and durations are in `commands.jsonl`; failed attempts are retained, not overwritten.

## Final formatted measurements and exact pins

| Surface | Measured | Pin | Authorized maximum |
|---|---:|---:|---:|
| Kernel physical | 26,548 | 26,548, `src_lines` line505 | 26,591 |
| Kernel tokei | 20,748 | 20,748 | 20,778 |
| xtask | 42,648 | 42,648 | 42,775 |
| maos-bin | 32,968 | 32,968 | 32,977 (`bin40`) |
| maos-domain | 9,977 | 9,977 | 9,977 |
| maos-audit | 7,660 | 7,660 | 7,660 |
| Aggregate | 187,905 | strict hardfail187,906 | exact measured+1 |

Kernel delta is +139 physical / +130 tokei from26,409/20,618, including formatting. Bin delta is +31 from32,937, inside operator ruling `bin40` (+40). Initial L2 bin32,946/aggregate187,883 were inside the first grant. The required integration correction initially breached +16, and work stopped without pinning past it. Main delivered the operator's explicit bin40 ruling on2026-10-30; only then was the correction compiled and exact pins raised. Both correction HISTORY segments cite `20-3b landing 2 held-durable-value correction, operator ruling bin40`; prior text remains under `| PRIOR:`. The xtask row's first segment retains the full open story key and42,405→42,648 arrow.

Production `check-kernel-baseline --emit-pin` supplied95 per-file hashes and set hash `e644d26eb74592103d606398df0653dce0828e375833b57c242b941e30d6d01c`. The production service snapshot regenerated the surface baseline:192 existing `use` leaf hashes and PrivateMemoryStore's struct hash changed, with no added/removed `(kind,path)` keys. Binding-method class metadata explicitly records the walker's top-level-item limitation.

## Task1 — build and witnesses

All cargo invocations used `env -u CI -u GITHUB_ACTIONS`, the inherited target directory, no clean, and no caller-supplied target-directory override. The official fixture producer internally uses its existing configured target paths; it was not modified.

- `cargo fmt --all`: exit0, initially3.20s; final correction formatting also0.
- `cargo build --locked --workspace --all-targets`: exit0, initially149.30s, and after final source changes. Final normal workspace artifact refresh exited0 in7.605s.
- Final serial command: `cargo nextest run --workspace --locked --color never --no-fail-fast --test-threads 1` — **exit0**,5,408/5,408 passed,137 skipped; runner609.468s, command612.25s. `workspace-serial-final.junit.xml` preserves exactly that run.
- Requested default-thread command, run once explicitly: `cargo nextest run --workspace --locked --color never --no-fail-fast` — **exit0**,5,408/5,408 passed,137 skipped (one slow); runner48.928s, command55.15s. Separate JUnit/transcript retained.
- `cargo test --locked -p maos-kernel-core --test principal_identity_20_3b` — exit0,6 passed.
- `cargo test --locked -p maos-audit --test governance_roundtrip_20_3b` — exit0,4 passed.
- `cargo test --locked -p maos-bin --test hook_pump_17_4 --test butler_scene_18_4` — exit0,26+10 passed.
- All five new xtask integration binaries, `d11_xtask_ceiling_ratchet`, kernel-pin and recovery-rule binaries — exit0 (`xtask-new-vectors.log`).
- `cargo test --locked -p maos-kernel-core --lib memory` — exit0,41 passed. Final prepush additionally ran the full kernel-core lib suite and workspace doctests, both exit0.
- Registered expensive-filter xtask subset — exit0,11/11 passed across ABI and diagnostics binaries, including all six ignored `diagnostics_20_3a` vectors (runner8.106s). Exact expanded filter is recorded.
- Authorized correction: `cargo test --locked -p maos-bin --test erasure_uninstall_13_5b --test maos_uninstall_16_4` — exit0,21+23 passed;9.371s. Full suite also exercised the related forget/purge coverage.

### Initial failures and actual fixes

Initial full run exited100:5,384 passed/22 failed/137 skipped. Nineteen failures named stale WASM/native fixture artifacts; the official producer refreshed14 artifacts. Three existing uninstall assertions exposed always-durable small principal values. No assertion was weakened.

1. **Production hold accounting (`crates/maos-bin/src/main.rs`)**: deduplicate held principals; count actual disk-backed recorded-owner exports rather than stale index rows; preserve held-only no-proof and mixed partial-proof terminals; represent retained held data as a legal-hold CoverageGap, never VerifiedEmpty; continue refusing unrelated orphan residue.
2. **Compile correction**: added the local `MemoryManagerPort as _` import after the first corrected build correctly refused the trait method (exit101). Subsequent correction build/tests passed.
3. **Affected fixture (`erasure_uninstall_13_5b.rs`)**: the no-residue unknown-name witness now starts with an empty TL, writes only an unrelated admission identity and asserts both memory tiers empty. Its original zero-proof assertion remains. Added a small durable unknown-name residue witness requiring failed partial proof, and held-plus-orphan witness preventing an overbroad held exemption. Held-only/mixed assertions remain intact.
4. **Advisory job labeling**: added the required in-job `ADVISORY — owner:20-3b; reason:...` annotation before continue-on-error. This is lane ownership, not ownership of lint repairs.
5. **Fixture/artifact freshness**: additional serial attempts exposed the journey guard's deliberate cross-check of BOTH binary source trees against BOTH executables. An explicit alternate-metadata relink was insufficient because the workspace build restored its older normal artifact. A one-sided source touch similarly made the other normal artifact appear stale. The successful repair touched both already-read entrypoints without changing any bytes, then rebuilt `--workspace --all-targets`, refreshed official fixtures, and passed all64 journey-package tests under workspace feature unification before the final green full runs. No guard was disabled and no binary timestamp was fabricated.
6. **New migration mirror prerequisite**: first prepush exited1 solely because the real migration corpus was absent. Production CI generates it before the gate. Ran its exact million-row deterministic generator at the gate's default local path; all pinned oracles matched and the gate exited0. Documented that prerequisite in `docs/testing/proof-contract.md`; no gate was removed or weakened.

The intermediate full-run failures (9 stale-cli,9 failed alternate-relink,24 cross-staleness) and their JUnits/transcripts remain in evidence. These are resolved build-prerequisite failures, not suppressed tests.

## Tasks2–3 — pins

Physical/tokei/content/set/surface/class pins and pin-test literals are updated. `src_lines` stays on505. Recovery-rule literal is20,748. The xtask ratchet uses the reviewed L1 starting ceiling42,405 while retaining historical retirement42,400; its real-tree authorization and negative vectors passed. All exact formatted pins and aggregate measured+1 pass kloc and kernel-baseline gates. No ceiling was reserved or silently raised.

## Task4 — proven-red evidence

`plants.md` contains exact real-checkout commands, durations, assertion text, copy-back/cmp/touch restoration commands and SHA-256 cut hashes. Every FR31 mutation exited101:

| Plant | Observed assertion | Seconds including compile |
|---|---|---:|
| Pid-keyed spill owner | Some(Markdown("A only")) versus None |9.141|
| Pid-keyed index/export | one index row versus two |5.490|
| Stop always-saving principal values | None versus Some(Text("small")) |5.623|
| Butler restore poisoning | seven attacker records versus zero |14.062|

Every restore copy/cmp/touch exited0. Restored identity and Butler witnesses returned green. Temporary backups were removed. `sha256sum --check plant-cuts.sha256` exited0. In-tree invariant-ledger and clippy-ratchet negative vectors passed; malformed/failed streams and either count's growth are refused. Real-tree AC3 if:false plant: clean0/planted1 naming disabled-workflow-job/restored0. AC2 follows the ruled NO-GO closure; no nonexistent generated-YAML drift proof is claimed.

The branch-only evidence workflow wires hosted FR31 green/mutation/restored-green vectors, cut checksums, hard-mode/NO-GO witnesses and real non-denying clippy stream. It was not pushed or run; no hosted run id or signed-ledger proof is claimed.

## Task5 — corrected-tree clippy

Final `cargo clippy --workspace --all-targets --locked --message-format=json` — exit0,6.785s. Real built `clippy-ratchet --input <absolute clippy-final.jsonl>` — exit0,0.204s: **1,409 warning occurrences /881 source-distinct**, below the unchanged **1,411/884** baseline. The first live stream independently had the same counts. Both streams and stderr are retained. No warning suppression or baseline increase was made.

## Task6 — final gates and prepush

Each requested standalone gate exited0: kloc-check; check-kernel-baseline; check-service-boundary; check-env-contract; check-serde-error-handling; check-composition-root-completeness; check-j1-loopback-delegation; check-dev-record-completeness; check-epic-close-coherence; check-review-findings-resolved; check-test-cost; coverage-matrix (hard,zero violations); check-coverage-matrix-completeness; invariant-lock.

`check-test-budget --junit <absolute workspace-serial-final.junit.xml> --json` — **exit0**,574 binaries,Σ609.160s,span609.329s,no findings (0.202s gate). This saved serial report was not overwritten by later default/prepush runs.

Final prescribed command:

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated /home/lunarpulse/dev_ws/target/debug/xtask prepush --json
```

**Exit0**,132.50s (report132.472s): **83 pass,0 red,1 advisory,8 skip**. Full kernel lib tests, workspace doctests, workspace nextest, release/size and tracked-tree checks passed. Its fresh default-thread timing judgment remained advisory with five timing findings; the independently required final serial budget above is green, and no timing budget was weakened.

Compared with the prior L1 record,22 newly mirrored standalone legs consumed **21.247s** in the final run, including **17.266s** for the real migration Merkle check. This is the measured added-leg sum, not an invalid subtraction of noisy whole-run times (L1 total171.242s versus finalL2 total132.472s). Per-leg times are in `prepush-runtime.json`. The one-time corpus prerequisite took30.345s including compilation; gate verification took16.969s separately.

The migration corpus has1,000,000 rows; SHA-256 `a23b8a90235bb23279d4a4ecef5e5d09c28a3d3bc81002fe3aa4e0882f58a246`, Merkle `dd42bbd1fdc9826a0377a8e80aa1834b78c474d56fe958e9d6744ff4802e7d69`, payload oracle `6783fcaead6ff0f1d327571dd9090167d69ef661adadf2b3eb9438e4a65f0988`. Local gate honestly reports partial:true/live_postgres_cross_check:false; no live PostgreSQL proof is claimed.

## Task7 — dispositions

New deferred-work section records the FrameKind30/31 non-enum rows and32/33 unclassified domain (the retired global-contiguity gate could not have passed); unowned clippy debt with unchanged ratchet; same-name squatting owned20-1; explicit ownerless legacy-spill restart cutover; and all43 advisory empty v1.0 rows, individually routed by open charter or explicitly dispositioned ownerless. Initial uncited ownerless lines correctly redded dev-record-completeness; adding same-line rule10 disposition citations fixed the documentation, not the gate.

## Unverified and intentionally unchanged

Hosted workflow/full discipline and signed-ledger artifact/key paths were not run. The137 nextest skips and81 named prepush runtime diagnostics remain visible: signed rootless T3 substrate, Python Ed25519 backend and strace were not provisioned locally. Dynamic CI marker cardinalities remain CI-only. Docs build/routes and cargo-deny were conditionally skipped because their trigger paths did not change; no claim of a current advisory-DB deny recheck is made. Non-Unix compilation and live PostgreSQL cross-check were not exercised. These limitations do not replace the green local commands.

**AC6 cargo-deny recheck remains required:** the verifier must run `cargo deny check`
against its then-current advisory database and record the command, exit status
and result. The editing/review-fix pass did not run it and claims no deny result;
the prior conditional prepush skip does not satisfy this landing recheck.

No commit,push,cargo clean,alternate-worktree access or Dockerfile edit occurred. All mutation scaffolds were restored/removed. The real generated migration corpus remains as the documented local prerequisite. The operator's uncommitted Dockerfile edit was left alone. Prepush's tracked-tree check passed.

## DIFFICULT kernel/bin repairs — editing evidence, not a verification run

The post-review editing pass adds bounded owner/namespace components and lowercase
hex principal-key filenames. Components exceeding 200 bytes use SHA-256 names,
with full-identity sidecars checked before reads/writes; mismatches refuse rather
than sharing a digest address. The index retains full stable owner addresses.
This is a clean filename-format cutover: pre-landing raw-key principal spill files
are unreadable; there is no installed base pre-v1.0. A versioned layout and
migration is a prerequisite if an installed base appears. No ambiguous raw-key
fallback was added. The same-line rule-10 disposition is in `deferred-work.md`.

Principal writes commit an index intent before spilling. A spill failure removes
only a newly inserted intent, preserving pre-existing discoverability; cleanup
errors are surfaced. A crash after the intent but before the value may leave an
index row without a value, which export represents as redacted-missing. Uninstall
unions recorded stable-owner principals with current incarnation audit principals,
counts values before destructive operations, and signs a retained-under-hold/
unreadable coverage gap when a held value cannot be counted. The legacy schema
check, owner-column ALTER and primary-key migration share one immediate transaction.
Private-tier audit counting recognizes verified hashed namespaces and ignores
identity/key sidecars.

Budget offset is explicit: four original `principal.rs` unit tests were relocated
without weakening their assertions into `principal_identity_20_3b.rs`:
`write_and_lookup`, `lookup_nonexistent_returns_empty`,
`forget_then_lookup_empty`, and `second_forget_returns_zero`. The old unit module
occupied 71 physical / 59 Rust-code lines; kernel library test count decreases by
four and the integration target gains those four tests. New deterministic witnesses
cover long identity/namespace/key round trips, case-fold-distinct key filenames,
identity mismatches, intent-before-spill ordering, injected spill and index
failures, missing-value export, stable-owner discovery/forget, concurrent legacy
opens, unreadable held-value signed proofs, and hashed-namespace audit counting.

No cargo command, test, gate or formatter was run by this editing pass. Earlier
green results above precede these repairs and do not verify them. The orchestrator
must compile and run the changed kernel/bin integration targets and kernel memory
library tests, then refresh the granted pins/surfaces and run the required gates.

Observed source-only measurements (`tokei --files` and `wc -l` on the five
changed production files, each exit 0; no build/test/gate):

| Source | Physical before → after | Rust code before → after |
|---|---:|---:|
| kernel `memory/private.rs` | 1,037 → 1,122 | 910 → 1,000 |
| kernel `memory/principal.rs` | 272 → 201 | 223 → 161 |
| kernel `memory/mod.rs` | 1,170 → 1,173 | 924 → 925 |
| bin `main.rs` | 16,506 → 16,493 | 13,308 → 13,306 |
| audit `lib.rs` | 3,476 → 3,476 | 2,614 → 2,614 |

Net kernel delta is **+17 physical / +29 Rust code** including the disclosed
71/59 test relocation; without that offset production changes are +88/+88.
Applied arithmetically to the supplied pins, kernel becomes **26,565 / 20,777**
(ceilings 26,591 / 20,778), bin becomes **32,966** (ceiling 32,977), and audit
remains **7,660** (ceiling 7,660). These are source-count deltas, not a claim that
the verifier has re-pinned or run kloc/surface checks. `git show HEAD:crates/maos-kernel-core/src/memory/principal.rs`
also exited 0 for the original 71-line unit-module measurement.

## Review-correction verifier STOP — formatted envelope breach

This result supersedes the source-only forecast above. Earlier green verification
does **not** verify the current review-correction tree.

- `env -u CI -u GITHUB_ACTIONS cargo fmt --all` — **exit 0**, 2.76 seconds.
- `env -u CI -u GITHUB_ACTIONS /home/lunarpulse/dev_ws/target/debug/xtask kloc-check --json`
  — **exit 1**, 0.055 seconds; authoritative formatted counts below.
- Physical kernel lines were counted over every `crates/maos-kernel-core/src/**/*.rs`
  file, matching the existing kernel-baseline counting scope.

| Surface | Formatted measured | Current authorized maximum | Disposition |
|---|---:|---:|---|
| Kernel physical | **26,620** | **26,591** | **STOP: +29 above grant** |
| Kernel tokei | **20,834** | **20,778** | **STOP: +56 above grant** |
| maos-bin | **32,979** | **32,977** | **STOP: +2 above grant** |
| maos-audit | **7,668** | **7,660** | **STOP: +8 above grant** |
| xtask | 42,757 | 42,775 | Inside grant |
| maos-domain | 9,977 | 9,977 | Unchanged |
| Aggregate | **188,119** | Exact measured + 1 required | Would require hardfail **188,120** |

No baseline or ceiling was re-pinned past its grant. No build, test, plant,
`cargo deny`, clippy or prepush command was run for this review-correction tree.
Main received the exact figures and the request for revised operator authority
or edit-only corrections inside the existing grant. Raw gate output and the
command ledger are under `landing-2/review-corrections/`.

## Review-correction final verification — operator ruling w8

The operator subsequently authorized **“A: Raise to measured exactly”** on
2026-10-31. This resumes and supersedes the historical envelope STOP above;
no pin was raised before that authority. HISTORY uses
`20-3b landing 2 review corrections, operator ruling w8`, prepending the old
text under `| PRIOR:`. The following results verify the adopted
`L2FixKernel`/`L2FixXtask` corrections, not their earlier source-only forecasts.

### Final formatted measurements, pins and metadata

| Surface | Measured | Exact enforced pin |
|---|---:|---:|
| Kernel physical | **26,620** | **26,620**, `src_lines` remains line505 |
| Kernel tokei | **20,834** | **20,834** |
| xtask | **42,757** | **42,757**; inside the42,775 provisional grant |
| maos-bin | **32,979** | **32,979** |
| maos-domain | **9,977** | **9,977**, unchanged |
| maos-audit | **7,668** | **7,668** |
| Aggregate | **188,119** | strict hardfail **188,120** |

Kernel/bin/audit are exactly at their w8 grants, with no reserved headroom.
The production `check-kernel-baseline --emit-pin` returned95 file hashes and
set hash `ca8eb931958207cbe5bd76b599764aeeb75b9142374e10ff4cbff157598616d2`.
The95-file set is unchanged; the three memory implementation hashes changed.
The production surface writer regenerated a byte-identical current snapshot;
the earlier one-time per-path `pub use` leaf-hash migration remains intact,
not reclassified as removals. Added the data-movement classification for
`MemoryManagerAdapter::principal_ids_for_spirit`.

Updated `kernel-core-baseline.toml`, `kloc.toml`, kernel pin-content literals,
`RATIFIED_AT_EASING = 20_834`, API-class metadata, and both story/epic grant
tables. The xtask HISTORY first segment retains the full
`20-3b-gate-retirement-and-coverage-generator` key and42,648→42,757 arrow.
Kernel-pin/recovery/d11 regression binaries and the real-tree gates passed.
The final post-test-fix kloc check also exited0; test lint fixes did not change
any production measurement.

### Commands, regressions and all integration fixes

Exact argument vectors, absolute working directories, exit codes and elapsed
seconds are in [`review-corrections/commands.jsonl`](review-corrections/commands.jsonl).
Corresponding stdout/stderr logs, JUnits, clippy streams and both prepush
reports are retained in that directory. Every cargo command used
`env -u CI -u GITHUB_ACTIONS`; no target-directory override or clean was used.

| Command or requested regression set | Final exit | Observed result |
|---|---:|---|
| `cargo fmt --all` |0| Final test-diagnostic formatting2.451s |
| `cargo build --locked --workspace --all-targets` |0| Initial134.36s; refreshed final build90.299s |
| `cargo run -p xtask -- build-test-fixtures` |0| Official refresh14.769s |
| Kernel `--lib memory` |0|7.087s |
| Kernel principal identity / forget-restart / spill-supersession / routed halt binaries |0|19+10+18 supported tests pass; routed ignored red explicitly exercised below |
| Bin erasure/uninstall and uninstall/purge binaries |0|24+23 tests pass,7.253s |
| Audit private-tier-count and governance-roundtrip binaries |0|3.776s |
| New xtask coverage/invariant/clippy/surface-root/prepush/routed-catalog binaries |0| Final direct command0.830s; negative vectors included |
| SDK routed binary with `--features spirit_test` |0| Final import-only regression0.646s; ignored red explicitly exercised below |
| Bin hook-pump and Butler binaries |0| Final direct command21.248s |
| Registered expensive-filter xtask set, ignored-only, serial |0|11/11 pass across ABI/diagnostics, including all six `diagnostics_20_3a` vectors;31.647s command/5.602s runner |
| Full workspace serial nextest |0| **5,433 passed /139 skipped**,588.73s command/585.624s runner |
| Full workspace default-thread nextest |0| **5,433 passed /139 skipped**,41.96s command/40.665s runner |

Initial targeted kernel/bin commands exited101 on three obsolete fixture
assumptions. The complete verifier integration fixes were:

1. `private_spill_supersession_13_5j.rs`: plant the principal sibling as
   `6b.bin`, alongside `6b.json`, matching the adopted lowercase-hex key
   encoding; the logical forgotten count remains exactly one.
2. `erasure_uninstall_13_5b.rs`: assert the new typed held-count message
   `Some(1) under legal hold`. The stable-owner uninstall witness now checks
   the actual terminal's `erased_principals == 1` and the signed proof's
   `memory_namespace = Removed { count: 1 }`, instead of a nonexistent
   `deleted_entries` terminal field. Owner-directory/index removal, pid2 and
   proof-signature assertions remain.
3. `routed_sdk_20_3b.rs`: remove an unused `Spirit` import.
4. `principal_identity_20_3b.rs`: remove two redundant borrows of freshly
   repeated schema strings; `erasure_uninstall_13_5b.rs`: remove the analogous
   redundant borrow in its long-schema witness.
5. `prepush_blocking_20_3b.rs`: compile the direct-xtask regex once outside the
   workflow loops, rather than on every visited command.

No production logic was changed by this review-correction integration pass.
The adopted production changes are the edit-only units' changes. No assertion,
gate or warning policy was weakened. Byte-neutral touches of both already-read
bin/cli entrypoints, followed by the all-target build and official fixture
producer, refreshed normal workspace-feature artifacts; source bytes were
asserted unchanged. Earlier full serial/default runs were also green before
the final lint-only test edits; the final runs above cover those edits.

### Explicit labelled reds and recut FR31 mutation evidence

Both ignored routed witnesses were run with `--ignored --exact --nocapture`,
not counted as default-suite coverage:

- FR34 SDK capture command — **exit101**,0.119s, at the intended assertion:
  **“FR34 SDK frame I/O must be observable in the harness report; empty capture
  is not coverage”**; observed `[]` versus the bytes of `iac-frame`.
- FR53 hot-swap command — **exit101**,0.172s, at the intended assertion:
  **“FR53 preserves the ORIGINAL active halt id, source-frame/policy replay
  payload, owner and fire time; re-firing is not continuity”**; observed
  `([], [])` versus the original halt id and full metadata. This remains
  explicitly owned20-3c, not presented as passing continuity.

[`plants.md`](plants.md) now begins with the recut current-tree FR31 proof.
All four mutations reached their intended assertions and exited101:
pid spill disclosure2.149s, pid index/export2.101s, lost small-value durability
1.833s, Butler restore poisoning6.042s. Every copy-back/cmp/touch command
exited0 and restored exact bytes. Restored principal tests passed19/19 and
the Butler witness passed. Cut SHA-256 verification exited0.
Backups remain at `/tmp/20-3b-review-plants-yweae8gs`; no cleanup was run.
Current root `*-plant.diff` files and `plant-cuts.sha256` replace the earlier
cuts and remain wired into the unpushed hosted evidence workflow.

AC2 remains the ruled NO-GO closure, AC3's prior real-tree if:false proof is
retained, and the in-tree invariant-ledger/clippy negative vectors pass.
No extra hosted-only cut or nonexistent generated-YAML drift proof was added.

### Actual cargo-deny and live clippy verdicts

`env -u CI -u GITHUB_ACTIONS cargo deny check` — **exit1**,1.145s.
The actual current database reports **four vulnerabilities**:
**RUSTSEC-2026-0420**, **RUSTSEC-2026-0422**, **RUSTSEC-2026-0423** and
**RUSTSEC-2026-0424**, affecting wasmtime/wasmtime-wasi48.0.7 and recommending
48.0.8 or newer. Summary: **advisories FAILED; bans/licenses/sources ok**.
`deny.toml`, dependencies and lockfile were not changed. This is an unresolved
ship-gate result, not replaced by prepush's conditional cargo-deny skip.

The first actual non-denying clippy JSON stream exited0 but the unchanged
ratchet correctly exited1 at **1,415 occurrences /886 source-distinct**.
Only the avoidable new test diagnostics listed above were corrected.
Final `cargo clippy --workspace --all-targets --locked --message-format=json`
exited0 in1.757s; the built `clippy-ratchet --input <absolute
review-corrections/clippy-fixed.jsonl>` exited0 in0.165s at
**1,410 occurrences /881 source-distinct**. Baseline remains **1,411/884**;
no suppression, baseline raise or unrelated production refactor was made.

### Standalone gates, saved serial budget and final prepush

Every requested standalone gate exited0: kloc-check; check-kernel-baseline;
check-service-boundary; check-env-contract; check-serde-error-handling;
check-composition-root-completeness; check-j1-loopback-delegation;
check-dev-record-completeness; check-epic-close-coherence;
check-review-findings-resolved; check-test-cost; coverage-matrix
(hard mode,zero violations); check-coverage-matrix-completeness; invariant-lock.

`check-test-budget --junit <absolute
review-corrections/workspace-serial-final.junit.xml> --json` — **exit0**,
0.017s:576 binaries, **Σ585.489s**,span585.603s,zero findings.
That exact serial report is saved independently of later default/prepush runs.

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated /home/lunarpulse/dev_ws/target/debug/xtask prepush --json
```

The first reviewed-tree prepush exited1 in421.50s:82 pass,one red,one advisory,
eight skips. Its sole red was `journey_forget_18_6` Butler startup:
`EndpointInUse` at127.0.0.1:46645, child exit78. The complete failure/JUnit is
retained. Both standalone full suites had passed that test. The exact same
prepush command was rerun **without a source or configuration change**:
**exit0**,118.50s (report118.490s), **83 pass,zero red,one advisory,eight skips**.
Workspace nextest, full kernel lib tests, doctests, release/size and
tracked-tree checks passed. The22 added standalone mirror legs consumed
**19.350s** in that run; per-leg records are in
`review-corrections/prepush-runtime.json`.

The prepush timing judgment remains advisory with three findings; it does not
supersede the requested saved serial budget's green verdict. No timing ceiling
was raised. Its eight explicit skips are docs build/routes, unchanged
lockfile/deny-trigger cargo-deny, and five dynamic CI marker cardinalities.
Runtime substrate diagnostics remain visible in the JSON: signed T3 substrate,
Python Ed25519 backend and strace are unavailable locally.

### Dispositions and remaining limits

The task7 deferred-work section and43 individually dispositioned advisory
coverage rows remain recorded; the hard-mode and disposition gates pass.
Same-name squatting remains owned20-1. The review units additionally recorded
their precise remaining ownerless legacy-spill restart scope and the labelled
FR34/FR53 boundaries rather than claiming those are solved.

**Not verified:** hosted evidence/discipline execution and signed-ledger
artifacts/keys, dynamic hosted marker counts, non-Unix compilation, signed T3
runtime scenarios, Python Ed25519/strace scenarios and a live PostgreSQL
cross-check. No hosted run id is claimed.
**Known reds:** actual cargo-deny's four advisories and the deliberately
labelled ignored FR34/FR53 witnesses. The initial prepush endpoint collision is
retained as a failed observation, despite the unchanged rerun being green.
No commit/push, cargo clean, alternate-worktree access or Dockerfile edit was
performed. The ignored real migration corpus remains on disk, untracked.

## Wasmtime 48.0.8 security patch verification

This section supersedes only the historical cargo-deny vulnerability verdict
and refreshes the requested local verification after the lockfile patch.
The [crates.io release listing](https://crates.io/api/v1/crates/wasmtime)
identified **48.0.8** as the latest published 48.0.x patch (published
2026-10-29). No major/minor upgrade was taken.

### Smallest dependency change

Only the root `Cargo.lock` changed dependency resolution: **32 coupled packages**,
with no package additions/removals or unrelated upgrades in Cargo's update report.
The existing `wasmtime = "48.0.7"` and `wasmtime-wasi = "48.0.7"`
requirements permit this patch and remain unchanged. No `Cargo.toml`,
`deny.toml`, advisory ignore, feature setting, source file or ceiling was changed.

| Locked family | Before | After | Packages |
|---|---|---|---:|
| Wasmtime/WASI/internal crates |48.0.7|48.0.8|14|
| Pulley interpreter/macros |48.0.7|48.0.8|2|
| Wiggle/runtime/generator/macros |48.0.7|48.0.8|3|
| Cranelift assembler/codegen/support crates |0.135.7|0.135.8|13|

The Wasmtime rows comprise `wasmtime`, `wasmtime-environ`,
`wasmtime-internal-{component-macro,component-util,core,cranelift,fiber,jit-debug,jit-icache-coherence,unwinder,versioned-export-macros,wit-bindgen}`,
`wasmtime-wasi` and `wasmtime-wasi-io`. Pulley is
`pulley-{interpreter,macros}`; Wiggle is `wiggle`, `wiggle-generate`,
`wiggle-macro`; Cranelift is
`cranelift-{assembler-x64,assembler-x64-meta,bforest,bitset,codegen,codegen-meta,codegen-shared,control,entity,frontend,isle,native,srcgen}`.

The first Wasmtime-only dry run correctly refused the locked WASI/Wiggle
`=48.0.7` coupling. Dry-running the exact WASI companion update resolved the
entire family without widening the upgrade. The real WASI update was then
followed by the explicitly requested precise Wasmtime update (already satisfied).
**RUSTSEC-2026-0420, RUSTSEC-2026-0422, RUSTSEC-2026-0423 and
RUSTSEC-2026-0424 are cleared:** actual `cargo deny check` reports
**advisories ok, bans ok, licenses ok, sources ok**, exit 0.
Its existing unmatched-license/unnecessary-skip warnings remain visible;
no policy was weakened to silence them.

### Exact command and exit-code ledger

Working directory for every command below: `/home/lunarpulse/dev_ws`.
Every Cargo row below was executed with the exact prefix
`env -u CI -u GITHUB_ACTIONS`; the prefix is omitted from table cells solely
for readability. All builds used the inherited `CARGO_TARGET_DIR`; no override
or `cargo clean` was issued. The official fixture producer retains its existing
internal target-directory arguments, unchanged.

| Command (in execution order) | Exit | Observed result |
|---|---:|---|
| `cargo update --dry-run -p wasmtime --precise 48.0.8` |101| Refused locked Wiggle/WASI `wasmtime-environ =48.0.7`; no lock write |
| `cargo update --dry-run -p wasmtime-wasi --precise 48.0.8` |0| Previewed exactly the 32 coupled patch updates above; no lock write |
| `cargo update -p wasmtime-wasi --precise 48.0.8` |0| Applied those 32 updates |
| `cargo update -p wasmtime --precise 48.0.8` |0| Exact patch already satisfied; no further updates |
| `cargo tree --locked -i wasmtime` |101| Root is virtual with empty default-members; requires explicit workspace scope |
| `cargo tree --locked --workspace -i wasmtime` |0| Default workspace consumer: `maos-wasm-host` |
| `cargo tree --locked --workspace --all-features -i wasmtime` |0| Also identified `maos-bin`, `maos-journey-test` and `xtask` through optional/dev edges |
| `cargo deny check` |0| All four checks pass; all four reported advisories cleared |
| `cargo build --locked --workspace --all-targets` |0| All targets built, 185.41 s |
| `cargo run -p xtask -- build-test-fixtures` |0| Official WASM/T2 refresh: 14 artifacts, 125.49 s |
| `cargo nextest run --locked --color never --no-fail-fast -p maos-wasm-host -p maos-bin -p maos-eval -p maos-journey-test -p xtask` |0| **2,222 passed, 31 skipped**; 27.527 s runner /121.55 s command |
| `cargo nextest run --workspace --locked --color never --no-fail-fast` |0| **5,433 passed, 139 skipped**; 42.160 s runner /181.30 s command; one standalone full-workspace run |
| `cargo run -q -p xtask -- check-dev-record-completeness` |0| 195 done-status stories checked, 0.362 s |
| `cargo run -q -p xtask -- check-env-contract` |0| Passed, 0.597 s |
| `cargo run -q -p xtask -- kloc-check` |0| Aggregate 188,119 LOC, 0.220 s |
| `cargo run -q -p xtask -- check-kernel-baseline` |0| 26,620 physical lines /95 files, 0.195 s |
| `cargo run -q -p xtask -- check-service-boundary` |0| Zero violations, 1.284 s |
| `cargo fmt --all -- --check` |0| No formatting changes required, 2.501 s |
| Prescribed prepush command below |0| **84 pass, zero red, one advisory, seven skips**; 412.50 s command /412.477 s report |
| `cargo run -q -p xtask -- kloc-check --json` |0| Recorded all per-crate figures unchanged, 14.500 s |
| `cargo run -q -p xtask -- check-kernel-baseline --json` |0| Exact source/file-set/hash match, 0.205 s |

The two update commands, and separately the two workspace-scoped tree commands,
were each issued as a left-to-right `&&` pair; both components exited 0.
Workflow setup also ran
`python3 /home/lunarpulse/dev_ws/_bmad/scripts/resolve_customization.py --skill /home/lunarpulse/dev_ws/.claude/skills/bmad-quick-dev --key workflow`,
exit 0. Read-only evidence parsing initially used the nonexistent `checks`
JSON key and raised `KeyError`; correcting the lookup to the actual `records`
key produced the counts below without rerunning prepush. This was not a
failed Cargo command or gate.

### Prescribed prepush: deny actually ran

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated $CARGO_TARGET_DIR/debug/xtask prepush --json
```

**Exit 0.** The report includes `Cargo.lock` in `changed_paths`; its
`cargo-deny` record is **pass, detail `exit 0`, 683 ms**, not a conditional
skip. Workspace nextest, full kernel-core library tests, workspace doctests,
fixture refresh, release/size and tracked-tree gates also passed.

The one advisory remains `test-budget`: its inner check exited 1 with three
timing findings on the default-thread JUnit. This does not turn the prepush
exit into a failure, and is not claimed as a green timing-budget proof.
No budget/exception/pin was changed. The seven explicit skips are docs build,
docs routes and five dynamic CI-marker cardinality checks. The report retains
81 runtime diagnostics, including missing signed T3 substrate, Python
Ed25519 backend and strace scenarios; these are not claimed as exercised.

### No KLOC or kernel movement

Compared the JSON `per_crate` map and aggregate with
`review-corrections/gate-kloc-check.log`: **every per-crate figure and the
aggregate are identical**, not merely below their ceilings.

| Surface | Before | After |
|---|---:|---:|
| Kernel physical |26,620|26,620|
| Kernel tokei |20,834|20,834|
| xtask |42,757|42,757|
| maos-bin |32,979|32,979|
| maos-domain |9,977|9,977|
| maos-audit |7,668|7,668|
| Aggregate |188,119|188,119|

Kernel baseline still has 95 files and set hash
`ca8eb931958207cbe5bd76b599764aeeb75b9142374e10ff4cbff157598616d2`;
its `changed`, `added` and `removed` arrays are empty. No source figure or
baseline moved.

### Remaining limits

The standalone default suites preserve their 31 targeted /139 workspace
skips. Hosted discipline/evidence execution, signed-ledger artifacts/keys,
dynamic hosted marker cardinalities, non-Unix compilation, signed T3 runtime
scenarios, Python Ed25519/strace scenarios and live PostgreSQL cross-check
remain unverified. The deliberately ignored FR34/FR53 reds documented above
were not rerun or fixed by this dependency-only change. No extra all-features
test run is claimed: `--all-features` was used only to discover reverse
dependency consumers. Final Markdown evidence/Debug Log edits were made
after verification and were not followed by another prepush run.

Changed deliverables are exactly the root `Cargo.lock`, this verification
record and the story's one-line Debug Log note. No commit/push,
alternate-worktree access or Dockerfile edit occurred.
