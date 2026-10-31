# 20-3b landing-2 verifier/integrator — final evidence

**Current authoritative result:** see “Round-3 corrections — w10 adopted final verification” below. Operator ruling w10 authorized the exact measured pins. All 11 pin-dependent failures cleared: both serial workspace runs and the explicit default-thread run pass 5,446 tests, both serial timing judgments pass with zero findings, and prescribed prepush passes with one explicitly disclosed default-thread timing advisory. Earlier STOP/red runs remain historical.

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

## Round-2 corrections — initial measured STOP (historical)

Read both `R2Blind` and `R2Sec` reports in full. Implemented the six assigned corrections without changing any pin, ceiling, kernel surface/class metadata, or test expectation to hide the measured growth. All task command arrays, working directories, exit codes and durations are retained in `round-2/commands.jsonl`; complete stdout/stderr are in the adjacent labelled logs. These results supersede the earlier dependency-only green result for the changed source tree.

### Corrections and exercised regressions

| Fix | Implementation | Regression and observed result |
|---|---|---|
| 1 — shared index intent | `crates/maos-kernel-core/src/memory/principal.rs:83,119-150`: migrate a `written` protection flag; a second writer marks the row shared **before** its spill. Failed-inserter cleanup is `DELETE ... WHERE rowid=? AND written=0`; successful spills also set the flag. This protects both completed and in-flight second writers, while preserving cleanup of a genuinely new, unshared failed intent. Existing/migrated rows default protected. | `crates/maos-kernel-core/tests/principal_identity_20_3b.rs:675`, `failed_inserting_writer_preserves_a_second_index_instances_completed_spill`: two independent adapters/index instances share one DB; B completes while A is paused, A's temporary file is removed to fail its rename, then restart still exports B and discovers the stable owner. **PASS**. Existing failure-cleanup, committed-intent and legacy-migration tests also pass. |
| 2 — empty-key private count | `crates/maos-audit/src/lib.rs:2001-2004`: use recognized filename suffixes, not `Path::extension()`. Exactly zero production physical/tokei growth. | `crates/maos-audit/tests/private_tier_count_16_5.rs:26`, `empty_logical_keys_are_values_not_metadata`: `.md`, `.json`, `.bin`, `.txt` count as four values; sidecars/temp metadata do not. **PASS**. |
| 3 — atomic sidecars and incomplete recovery | `crates/maos-kernel-core/src/memory/private.rs:301-377`: exclusively create a temporary sidecar, write and fsync it, then atomically publish with `renameat_with(..., NOREPLACE)` and fsync the directory. A directory flock serializes recovery/publication across instances; a complete existing sidecar is only verified, never replaced. Legacy empty sidecars can be completed only on creation with no ambiguous data in the relevant identity/key address; populated unlabelled addresses refuse. Debug-only fault hooks are in `memory/spill_test_faults.rs:27-28`. | `principal_identity_20_3b.rs:732`, `interrupted_identity_publication_never_leaves_a_partial_final_sidecar`: both pre-sync/pre-publication failures across owner, namespace and key sidecars leave no partial final and restart succeeds (**six cases PASS**). At `:819`, `empty_legacy_sidecars_recover_only_without_ambiguous_values`: all three empty metadata cases recover; all three occupied cases refuse and preserve residue (**six cases PASS**). Existing nonempty identity-mismatch refusals pass. |
| 4 — held unreadability cannot cover other residue | `crates/maos-bin/src/main.rs:10473-10519,10564-10584`: add readable-held counts to physical files attributable specifically to unreadable **held** principal namespaces. Subtract only that total from remaining physical principal files; any positive unheld/unindexed remainder selects failure before the unreadable-held signed gap. | `crates/maos-bin/tests/erasure_uninstall_13_5b.rs:1309`, `unreadable_held_value_cannot_cover_an_unrelated_unindexed_private_value`: held-only unreadable data retains its signed gap; adding an unrelated unindexed `.md` value yields **failed/exit 5**, explicitly counts **1 other value**, preserves both files and verifies the signed proof. **PASS**. Existing mixed-erasure/unreadable-held and readable-held/orphan controls pass. |
| 5 — executing invariant evidence | `xtask/src/invariant_lock.rs:113-178,193-195`: corpus inclusion must initialize a named binding directly in top-level unconditional statement flow, and that binding must be consumed by a call/assertion. No recursive search through control-flow, closure or cfg-excluded bodies; unused/discarded/shadowed bindings and return-before-inclusion are refused. `continue-on-error` is accepted only when absent or the literal YAML boolean `false`, for both job and step. | `xtask/tests/invariant_claims_20_3b.rs:56,90-104`, `guaranteed_refuses_nonexecuting_or_unbound_witnesses`: boolean/string/expression nonblocking steps, expression nonblocking jobs, nested `if`, `match`, uncalled closure, cfg block/binding/use, discarded/unused inclusion, early return and shadowed bindings all refuse. **PASS**; the positive bound-and-used executing witness remains green. |
| 6 — cutover disclosure | `deferred-work.md:8`: the existing same-line rule-10 disposition now explicitly names R2Blind P1 #1/#2: raw-key principal spills **and** encoded owner/namespace components of 201..255 bytes become unreachable. Retains no installed base pre-v1.0, the operator-accepted 2026-10-30 clean cutover, and versioned layout + migration as the prerequisite if an installed base appears. No fallback aliases. | `check-dev-record-completeness --json`: **exit 0**; `check-epic-close-coherence --json`: **exit 0**. |

The first new uninstall regression incorrectly expected a `proof_path` field on the failed terminal. Its initial run exited 101 (24/25 passed). The existing failed-terminal contract carries the partial-proof path in its error; the corrected witness enumerates the single actual proof artifact and verifies its signature/category. It does not relax the failed outcome, exit 5, orphan count, retained-byte, or signed-gap assertions. The complete corrected bin targets pass.

### Formatted measured STOP — no re-pin

`cargo fmt --all` exited **0**. The actual formatted production measurements are:

| Surface | Measured | Unchanged pin | Delta / disposition |
|---|---:|---:|---|
| Kernel physical | **26,680** | 26,620 | **+60 — STOP** |
| Kernel tokei | **20,889** | 20,834 | **+55 — STOP** |
| maos-bin | **33,021** | 32,979 | **+42 — STOP** |
| maos-audit | **7,668** | 7,668 | **0 — zero-growth correction** |
| xtask | **42,780** | 42,757 | **+23; also 5 above the 42,775 cap — STOP** |
| maos-domain | **9,977** | 9,977 | 0 |
| Aggregate | **188,239** | 188,119 (hardfail 188,120) | **+120 — STOP** |

`kloc-check --json` exited **1** and `check-kernel-baseline --json` exited **1**. The kernel still has 95 files, no added/removed files; actual set hash is `f18147a4742e99aec0f7fd38a8205d440613c836cf9740438a5e147dec5118cf`, versus unchanged pin `ca8eb931958207cbe5bd76b599764aeeb75b9142374e10ff4cbff157598616d2`. Changed files are the two memory implementations and their existing debug fault-hook file. No content/set/surface/class or line-count pin was adopted without operator authority.

Per-fix attribution, measured against the assigned HEAD using `git show HEAD:<path>` and `tokei --files --output json` (each exit 0; raw figures in `round-2/per-file-measurements.json`):

| Fix / source | Physical before → after | Rust code before → after |
|---|---:|---:|
| 1 — `memory/principal.rs` | 211 → 225 (**+14**) | 173 → 187 (**+14**) |
| 3 — `memory/private.rs` | 1,166 → 1,210 (**+44**) | 1,044 → 1,085 (**+41**) |
| 3 — debug fault hooks | 107 → 109 (**+2**) | 72 → 74 (**+2**, excluded from production KLOC) |
| 4 — bin `main.rs` | 16,506 → 16,550 (**+44**) | 13,319 → 13,361 (**+42**) |
| 2 — audit `lib.rs` | 3,484 → 3,484 (**0**) | 2,622 → 2,622 (**0**) |
| 5 — `invariant_lock.rs` | 654 → 677 (**+23**) | 563 → 586 (**+23**) |

### Verification actually run

All Cargo commands removed `CI` and `GITHUB_ACTIONS`, did not supply a target override and did not clean. The subprocess environment had no `CARGO_TARGET_DIR`; Cargo's existing `/home/lunarpulse/dev_ws/target` symlink resolves to **`/mnt/build/cargo-maos`**, the same target exposed by the inherited shell variable. Both discovery commands (`printenv CARGO_TARGET_DIR`, `readlink -f /home/lunarpulse/dev_ws/target`) exited 0. No alternate target/worktree was used.

The following commands ran from `/home/lunarpulse/dev_ws`, with `env -u CI -u GITHUB_ACTIONS` prepended. `XTASK` below is the absolute `/home/lunarpulse/dev_ws/target/debug/xtask`; the complete unabridged arrays and exit codes are in `round-2/commands.jsonl`.

| Command | Exit / observed result |
|---|---|
| `cargo fmt --all` | **0**, both invocations |
| `cargo build --locked --workspace --all-targets` | **0**, initial and final restored-source build |
| `cargo test --locked -p maos-kernel-core --lib memory` | **0**, 37 passed |
| `cargo test --locked -p maos-kernel-core --test principal_identity_20_3b --test private_forget_restart_13_5i --test private_spill_supersession_13_5j` | **0**, 22 + 10 + 18 passed |
| `cargo test --locked -p maos-bin --test erasure_uninstall_13_5b --test maos_uninstall_16_4` | Initial **101** for the new proof-path fixture error described above; corrected run **0**, 25 + 23 passed |
| `cargo test --locked -p maos-audit --test private_tier_count_16_5 --test governance_roundtrip_20_3b` | **0**, 2 + 4 passed |
| `cargo test --locked -p xtask --test invariant_claims_20_3b` | **0**, 6 passed, including all new adversarial vectors |
| `cargo nextest run --workspace --locked --color never --no-fail-fast --test-threads 1` | **100**, 5,427 passed / 11 failed / 139 skipped; runner 607.962s, command 775.005s |
| `cargo nextest run --workspace --locked --color never --no-fail-fast` | **100**, once explicitly; 5,427 passed / 11 failed / 139 skipped; runner 42.761s, command 44.658s |
| `XTASK kloc-check --json` | **1**, exact STOP figures above |
| `XTASK check-kernel-baseline --json` | **1**, exact STOP figures/hash above |
| `XTASK check-service-boundary --json` | **0** |
| `XTASK check-dev-record-completeness --json` | **0** |
| `XTASK check-epic-close-coherence --json` | **0** |
| `XTASK invariant-lock --json` | **0** |
| `XTASK coverage-matrix --json` | **0** |
| `cargo clippy --workspace --all-targets --locked --message-format=json` | **0**; pure real JSON stdout saved as `round-2/clippy-live.jsonl`, stderr separately |
| `XTASK clippy-ratchet --input <absolute round-2/clippy-live.jsonl>` | **0**: **1,410 occurrences / 881 source-distinct**, below unchanged **1,411 / 884** |
| `cargo deny check` | **0**: advisories, bans, licenses and sources all **ok** |
| `CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated XTASK prepush --json` | **1**, 520.878s: **80 pass / 4 red / 1 advisory / 7 skip** |
| `XTASK check-trial-attestation --json` (diagnosing prepush red) | **1**; the sole failed leg is `kernel-abi-diff`, not the attestation machinery |

Both full runs have the same 11 baseline-dependent failures: the a2a kernel-line-count assertion; FKCS's current-kernel baseline assertion; six kernel-pin-content-hash tests; the escape-detector evidence test; the trial-attestation evidence test; and the workspace KLOC assertion. The escape and trial reports identify their `kernel-abi-diff` leg as red. Their underlying kernel pin was deliberately **not** changed at the measured STOP. Prepush's four red checks are `check-kernel-baseline`, `kloc-check`, `check-trial-attestation`, and `workspace-nextest`. This is not a green workspace/prepush claim.

One command-shape error is retained: the first `clippy-ratchet` invocation included unsupported `--json` and exited **2**; the supported command above consumed the same real stream and passed. Likewise the first checksum command ran from the repository root and exited **1** because checksum entries are relative to `landing-2`; running the identical checksum file from `landing-2` exited **0**. Neither was accepted as product verification.

### Four final-code FR31 plants

All four mutations compiled and failed at the intended behavioural assertion (**exit 101**, never a setup/build failure):

| Plant / exact test selector | Observed assertion | Command seconds |
|---|---|---:|
| owner → pid / `recycled_pid_cannot_disclose_another_spirits_principal_spill` | `Some(Markdown("A only"))` versus `None`, test line 47 | 10.225 |
| owner → pid / `recycled_pid_cannot_poison_butlers_acceptance_restore` | seven attacker records versus zero, test line 1009 | 17.516 |
| index/export → pid / `subject_access_export_preserves_both_owners_when_pid_and_key_are_reused` | one index row versus two, test line 111 | 2.774 |
| remove always-durable principal spill / `small_non_markdown_principal_values_survive_restart_and_pid_change` | `None` versus `Some(Text("small"))`, test line 76 | 2.473 |

Exact commands and assertions are in `round-2/plant-*.log` and `commands.jsonl`. Original bytes were held in memory, surgical edits applied/reversed using the editor, and all three restored source files compared byte-identical with their pre-plant bytes. No backup script/file remains. Restored `principal_identity_20_3b` passed all 22 tests (**exit 0**); restored Butler poisoning witness passed (**exit 0**); final normal workspace all-targets build passed (**exit 0**).

Recut `principal-index-export-pid-plant.diff` to mutate the new conflict target as well as the primary key/export path; otherwise that plant would fail at SQL setup instead of the intended assertion. Recut `small-principal-durability-plant.diff` to the final line location. Owner/poisoning cuts are unchanged. Updated `plant-cuts.sha256`; final `sha256sum --check <absolute plant-cuts.sha256>` from `landing-2` exited **0**:

```text
6bdf6cf69fdc3de36322b36ac4460d1fd02e5eabff73ccdc1cbecc5289b0976d  disclosure-owner-pid-plant.diff
a5ec173fea6834747c47b888690ff91fbed7270ba06c6e16e2390e3d6e233b23  principal-index-export-pid-plant.diff
904f25a4acc06c60261e43ccd7bff789264882a3c44bd239da51f1f85b7a8b51  small-principal-durability-plant.diff
6bdf6cf69fdc3de36322b36ac4460d1fd02e5eabff73ccdc1cbecc5289b0976d  butler-restore-poisoning-plant.diff
```

### Open authority and unverified surfaces

At this historical pre-w9 STOP, operator authority for the measured pin/cap changes was still required. No pins had been raised, no tests/gates weakened or filtered to manufacture a green result, and no unrelated code removed to offset the growth. Main received the exact STOP figures. The subsequent w9 ruling and completed re-verification below supersede this authority blocker.

Hosted workflow execution/signed-ledger artifacts, non-Unix compilation, live PostgreSQL, signed T3 runtime scenarios, Python Ed25519 and strace scenarios remain unverified/skipped as disclosed by the actual runs. FR34/FR53's deliberately labelled ignored reds remain unchanged. No commit, push, `cargo clean`, alternate-worktree access, Dockerfile edit, or tracking of `tests/corpora/migration-corpus-1e6.sqlite` occurred. Final Markdown-only evidence edits follow the recorded commands; no post-evidence prepush run is claimed.

## Round-2 corrections — w9 adopted final verification

**Operator authority, 2026-10-31, ask w9: “A: Raise to measured exactly.”** FLAG-Winston extends w8 by **+60 physical / +55 tokei** to **26,680 / 20,889**; bin becomes **33,021**, extending the inherited epic envelope **33,010 → 33,021**; xtask cap **42,775 → 42,780**; audit stays **7,668**; aggregate is **measured + 1**. This authority was received before re-pinning. No implementation change, budget exception, warning suppression or weakened test was used to resolve the initial STOP.

All current commands, absolute paths, exit codes and durations are in **`round-2/w9/commands.jsonl`**. Full logs, separate serial/default/prepush JUnits, original-source restoration digests, emitted pin block, unchanged surface snapshot and machine-readable `summary.json` are alongside it. Earlier `round-2/` logs remain the historical unadopted-pin runs.

### Exact final formatted pins

`cargo fmt --all` exited **0** before measurement/adoption and again after pin-test edits. Final production measurements after restoring all four plants are identical to the granted figures:

| Surface | Formatted measured | Exact final enforced pin / ceiling |
|---|---:|---:|
| Kernel physical | **26,680** | **26,680**, `src_lines` still at line **505** |
| Kernel tokei | **20,889** | **20,889** |
| maos-bin | **33,021** | **33,021**, epic envelope now **33,021** |
| xtask | **42,780** | **42,780**, authorized cap now **42,780** |
| maos-audit | **7,668** | **7,668**, unchanged |
| maos-domain | **9,977** | **9,977**, unchanged |
| Aggregate | **188,239** | strict hardfail **188,240**, measured + 1 |

**Zero headroom.** Both final `kloc-check --json` and `check-kernel-baseline --json` exit **0**. Production `check-kernel-baseline --emit-pin` exits **0**; its three changed digests and set hash were adopted exactly. Final kernel report has **95 files**, empty `changed`/`added`/`removed`, and matching actual/pinned set hash:

```text
f18147a4742e99aec0f7fd38a8205d440613c836cf9740438a5e147dec5118cf
```

Updated `kernel-core-baseline.toml` HISTORY, the four relevant KLOC rows plus aggregate, `kernel_pin_content_hash_16_0.rs`'s four reader literals, and `recovery_lane_ceiling_rule.rs`'s exact 20,889 literal. HISTORY cites **`20-3b landing 2 round-2 corrections, operator ruling w9`** and retains prior grants under `PRIOR`. The xtask row's **first segment** includes the full open story key and **42757 -> 42780** arrow; the D11 real-tree authorization/negative tests pass. Story frontmatter/table and both affected epic grant records carry the w9 ceilings and bin-envelope extension.

The production service-boundary report's **401-item** `current_surface` equals the committed surface baseline exactly; `round-2/w9/kernel-surface-current.json` is byte-identical to it. No public API moved, so the surface JSON and API-class metadata are intentionally unchanged. The service-boundary gate passes; there was no unnecessary baseline/class rewrite.

### Final build, focused and full test results

Every Cargo invocation used `env -u CI -u GITHUB_ACTIONS` and the inherited shell target **`/mnt/build/cargo-maos`**, carried unchanged into subprocesses; no target override, alternate target/worktree, or clean was used.

| Command, with `env -u CI -u GITHUB_ACTIONS` prefix | Exit / observed result |
|---|---|
| `cargo fmt --all` | **0**, before pinning and final pin-test formatting |
| `cargo build --locked --workspace --all-targets` | **0**, adopted-pin build and final restored-source build |
| `cargo test --locked -p xtask --test d11_xtask_ceiling_ratchet --test kernel_pin_content_hash_16_0 --test recovery_lane_ceiling_rule --test invariant_claims_20_3b` | **0**: D11 **9**, kernel-pin **25**, recovery **12**, invariant claims **6** passed |
| `cargo test --locked -p maos-kernel-core --lib memory` | **0**, **37 passed** |
| `cargo test --locked -p maos-kernel-core --test principal_identity_20_3b --test private_forget_restart_13_5i --test private_spill_supersession_13_5j` | **0**, **22 + 10 + 18 passed**, after plant restoration |
| `cargo test --locked -p maos-bin --test erasure_uninstall_13_5b --test maos_uninstall_16_4` | **0**, **25 + 23 passed**, after plant restoration |
| `cargo test --locked -p maos-audit --test private_tier_count_16_5 --test governance_roundtrip_20_3b` | **0**, **2 + 4 passed** |
| `cargo nextest run --workspace --locked --color never --no-fail-fast --test-threads 1` | **0**, **5,438/5,438 passed, 139 skipped**; runner **679.997s**, command **882.212s** |
| `cargo nextest run --workspace --locked --color never --no-fail-fast` | **0**, once explicitly; **5,438/5,438 passed, 139 skipped**, three slow; runner **74.206s**, command **86.689s** |

**All 11 initial baseline-dependent failures cleared**, including the six kernel-pin-content tests, a2a line count, FKCS baseline, escape/trial evidence legs and KLOC workspace assertion. The six correction witnesses named in the historical implementation table above are unchanged and green in the current focused/full runs.

### Serial timing finding — explicit advisory, no exception

Ran the actual required command:

```sh
env -u CI -u GITHUB_ACTIONS /mnt/build/cargo-maos/debug/xtask check-test-budget --junit /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-3b-gate-retirement-and-coverage-generator-evidence/landing-2/round-2/w9/workspace-serial.junit.xml --json
```

**Exit 1**, 576 binaries; **Σ678.870s**, span **679.850s**. The sole finding is untouched `maos-iac::participant_recall_17_8`: **Σ37.355s over 29 tests**, versus **30.000s**. The saved serial JUnit was copied before the default/prepush runs and is not overwritten.

**Main/operator disposition received during this run:** this is a timing-only advisory on this host under load, **not a 20-3b regression**; prior local observations were **43–68s**, while the E19 landing hosted budget gates were green. Those earlier observations are supplied disposition context, not fresh measurements claimed by this agent. **Add no exception now; the hosted JUnit decides at landing.** `xtask/test-budgets.toml` is unchanged. The finding is retained, not rerun away or relabelled as an exit-0 budget check.

### Gates, real clippy and dependency policy

Each of the following standalone commands with `--json` exited **0** using the absolute built `/mnt/build/cargo-maos/debug/xtask`: `kloc-check`; `check-kernel-baseline`; `check-service-boundary`; `check-dev-record-completeness`; `check-epic-close-coherence`; `invariant-lock`; `coverage-matrix`; `check-coverage-matrix-completeness`; `check-env-contract`; `check-serde-error-handling`; `check-composition-root-completeness`; `check-j1-loopback-delegation`; `check-review-findings-resolved`; `check-test-cost`; `check-trial-attestation`. Final restored KLOC/kernel reports also exit **0** at the same exact figures. No advisory output was accepted in place of these blocking checks.

`env -u CI -u GITHUB_ACTIONS cargo clippy --workspace --all-targets --locked --message-format=json` exited **0**, **233.557s**. Its real stdout is `round-2/w9/clippy-live.jsonl`, with stderr separate. The built `clippy-ratchet --input <absolute round-2/w9/clippy-live.jsonl>` exited **0**, **0.209s**, counting **1,410 occurrences / 881 source-distinct**, within unchanged **1,411 / 884**. `cargo deny check` exited **0**, **10.284s**: advisories, bans, licenses and sources all **ok**. This standalone deny run is current; prepush's conditional deny skip below does not replace it.

### Four FR31 plants re-exercised under w9

All four existing root cuts still match the adopted final production source exactly; no further recut is needed. The two owner cuts are identical; index cut includes its necessary conflict-target mutation. Each command compiled and reached the intended behavioural assertion, **exit 101**:

| Plant / exact test selector | Intended assertion reached | Command seconds |
|---|---|---:|
| `recycled_pid_cannot_disclose_another_spirits_principal_spill` | `Some(Markdown("A only"))` versus `None`, test line 47 | **108.031** |
| `recycled_pid_cannot_poison_butlers_acceptance_restore` | seven attacker records versus zero, test line 1009 | **174.361** |
| `subject_access_export_preserves_both_owners_when_pid_and_key_are_reused` | one index row versus two, test line 111 | **90.506** |
| `small_non_markdown_principal_values_survive_restart_and_pid_change` | `None` versus `Some(Text("small"))`, test line 76 | **50.921** |

The complete commands and red assertions are `round-2/w9/plant-*.log` and its command ledger. Source originals were retained in memory; reverse surgical edits restored all three files exactly. `sha256sum --check <absolute round-2/w9/plant-source-originals.sha256>` from the repository root exited **0** for all three; `sha256sum --check <absolute landing-2/plant-cuts.sha256>` from `landing-2` exited **0** for all four unchanged cuts. No temporary script or backup file remains. Restored kernel focused targets (**22 + 10 + 18**), bin uninstall targets (**25 + 23**), exact Butler poisoning witness (**1**) and final normal workspace build each exited **0**.

### Prescribed final prepush

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated /mnt/build/cargo-maos/debug/xtask prepush --json
```

**Exit 0**, command **1,298.180s**: **83 pass, 0 red, 1 advisory, 8 skip**. Kernel lib tests, workspace doctests, workspace nextest, release/size, tracked-tree and all blocking legs pass. Exact per-leg results are preserved in `round-2/w9/prepush.log`; `prepush.junit.xml` is separate from serial/default JUnits.

The one prepush advisory is its independent default-thread test-budget judgment (**exit 1**): `maos-bin::one_daemon_one_door_16_1` **Σ35.437s/38 tests** and `maos-iac::participant_recall_17_8` **Σ40.094s/29 tests**, each versus 30s. This is reported, not presented as a green budget leg; no budget/exception was changed. The eight skips are docs build/routes, conditional cargo-deny, and five hosted-marker checks. The independent requested deny check above passed. Prepush retains **81 named runtime diagnostics** for unavailable substrates; no skipped scenario is claimed exercised.

### Remaining limits and changed records

Hosted discipline/evidence execution, signed-ledger artifacts/keys, hosted timing judgment/marker cardinalities, non-Unix compilation, live PostgreSQL and the disclosed unavailable signed-T3/Python-Ed25519/strace scenarios remain unverified. FR34/FR53's labelled ignored reds are unchanged. No extra all-features test run is claimed.

This w9 continuation changes only exact pin/HISTORY/test-literal metadata, the story and two affected epic grant records, and final evidence records. The prior six implementation corrections are unchanged. Surface/class files and test budgets are intentionally unchanged. No commit, push, `.devcontainer/*` edit, alternate-worktree access, target override, `cargo clean`, or tracking of `tests/corpora/migration-corpus-1e6.sqlite` occurred. Final Markdown/summary evidence updates follow verification; no post-evidence prepush run is claimed.

## Round-3 corrections — initial measured STOP (historical)

The four R3Blind/R3Sec corrections are implemented. Exact verification commands,
absolute working directories, exit codes and durations are in
[`round-3/commands.jsonl`](round-3/commands.jsonl); logs, separate JUnits,
restoration checksums, real clippy streams and
[`round-3/summary.json`](round-3/summary.json) are alongside it.
No pin, ceiling, test budget, warning baseline or assertion was weakened.

### Fixes, locations and passing witnesses

1. **Interrupted hashed-directory creation is removable debris only without
   value files.** `crates/maos-kernel-core/src/memory/private.rs:537`
   (`identity_debris`), `:546` (recursive value-free check), and `:1147`
   (forget) remove unlabelled owner/namespace debris during unrelated forget;
   unlabelled values fail closed with the affected guarded path.
   `crates/maos-audit/src/lib.rs:1956` supplies the shared guarded-directory
   resolver; audit `:2001` counts debris as zero and refuses unlabelled values.
   Uninstall attribution uses the same resolver at
   `crates/maos-bin/src/main.rs:10496`, including guarded owners.
   This is the smaller reader-side recovery design rather than staging and
   renaming the entire directory hierarchy; same-identity creation still heals
   an interrupted empty directory.
   Passing tests:
   - `principal_identity_20_3b.rs:880`:
     `missing_identity_debris_does_not_block_unrelated_forget_but_values_do`
     exercises both owner and namespace debris and path-bearing refusal.
   - `private_tier_count_16_5.rs:52`:
     `missing_identity_counts_debris_as_zero_and_refuses_unattributed_values`.
   - `erasure_uninstall_13_5b.rs:1374`:
     `interrupted_identity_debris_does_not_block_uninstall_or_held_attribution`
     covers ordinary erasure and the unreadable-held attribution scan.
   All three tests passed in their focused commands and workspace runs.

2. **Portable atomic sidecar publication.** `private.rs:301`
   (`verify_identity`) now uses POSIX `linkat` atomic no-replace, removes the
   temp before reopening the sidecar (the normal no-hardlinks check remains),
   and on `EEXIST` accepts only a reread matching identity. Obsolete
   `renameat_with`/`RenameFlags` imports were removed. Reads no longer acquire
   an exclusive directory lock. `private.rs:513` treats exclusive-lock `EBADF`
   as unavailable, allowing fresh `O_EXCL`/`linkat` publication.
   Passing tests:
   - `principal_identity_20_3b.rs:946`:
     `identity_publication_accepts_matching_eexist_and_refuses_mismatch`.
   - `principal_identity_20_3b.rs:991`:
     `identity_creation_and_reads_work_when_nfs_exclusive_directory_lock_returns_ebadf`.
   - `principal_identity_20_3b.rs:1017`:
     `legacy_empty_identity_on_nfs_refuses_without_lock_and_reports_path`.
   These are deterministic publication/lock fault witnesses, not a claim of
   execution on a live NFS mount.

   **Accepted filesystem limitation:** Main explicitly accepted safe refusal
   for the combination of a legacy **empty** final sidecar and an unavailable
   real exclusive lock. Its typed `MemoryError::Storage` includes the guarded
   directory/sidecar path. Fresh creation and reads still work on the EBADF
   fallback. Unlinking an empty final without a real lock could delete a
   concurrent publisher's final metadata. The creation lock also protects
   stale-temp cleanup from deleting another creator's live temp; it is not
   needed for no-replace publication or reads. No alternative lock protocol
   was invented. This limitation is documented in
   `docs/testing/proof-contract.md:199`.

3. **Identity temp recovery.** `private.rs:301` retries the next nonce on
   temp `EEXIST`; `:503` and `:526` remove stale identity temps under the
   directory lock during creation and forget. Forget cleans owner and hashed
   namespace temps before reading their sidecars, including an interrupted
   link publication's second hardlink. On EBADF, unsafe unlocked temp cleanup
   is skipped. `memory/spill_test_faults.rs` adds only deterministic
   publication/EBADF test hooks.
   `principal_identity_20_3b.rs:915`:
   `stale_identity_temps_are_cleaned_on_creation_and_forget` passed. The test
   first plants a stale PID/nonce temp, then plants a hardlink of the completed
   owner sidecar to model a crash between link and temp unlink; forget removes
   the owner completely. Existing interrupted-publication, legacy-empty
   recovery and long-identity tests also pass.

4. **Nonvacuous invariant witnesses and constrained CI execution.**
   `xtask/src/invariant_lock.rs:104` supports `Try`, `Reference` and `Paren`,
   permits preceding `?` statements, and evaluates an initializer's dataflow
   before removing its shadowed binding. Corpus-derived bindings must reach
   an assertion, an asserted call result, or a checked `?` in a Result-returning
   test. Top-level `exit` calls and shadowing assertion macros are refused.
   Runner/wrapper environment overrides and nondefault shell/working-directory
   settings are refused. `xtask/tests/invariant_claims_20_3b.rs:56` adds
   adversarial vectors for discarded results, both exit forms, local/file
   assertion-macro shadowing, target/build runners, rustc wrappers, custom
   shell/directory and job shell defaults. `:126`,
   `guaranteed_accepts_asserted_dataflow_and_question_mark_tests`, covers
   references/parentheses, parsed-result assertions, same-name trimming and
   preceding/consuming `?` statements. The integration binary passes **7/7**.

### Exact formatted measurements versus unchanged pins

Measurements are from the final restored, formatted production tree, including
the path-bearing NFS refusal and all test hooks:

| Surface | Final measured | Unchanged pin | Delta |
|---|---:|---:|---:|
| Kernel physical | **26,793** | **26,680** | **+113** |
| Kernel tokei | **20,992** | **20,889** | **+103** |
| maos-bin | **33,030** | **33,021** | **+9** |
| maos-audit | **7,707** | **7,668** | **+39** |
| xtask | **42,841** | **42,780** | **+61** |
| maos-domain | **9,977** | **9,977** | **0** |
| Aggregate | **188,451** | prior measured **188,239**; hardfail **188,240** | **+212** |

The exact measured-plus-one hardfail would be **188,452**; it was **not**
adopted. Kernel content drift is confined to `memory/private.rs` and
`memory/spill_test_faults.rs`; the 95-file set is unchanged. Final actual set
hash is `a7719696a749180a3fa6b7b8d62de4f25e75ea2c14497297e7b6ada17a0e2cc2`,
versus unchanged pinned
`f18147a4742e99aec0f7fd38a8205d440613c836cf9740438a5e147dec5118cf`.

Per-fix budget attribution: fix 1 accounts for the audit **+39** and bin **+9**
surfaces; fix 4 accounts for xtask **+61**. Fixes 1–3 share the single kernel
implementation and its cleanup/publication helpers, measured together at
**+113 physical / +103 tokei**; these are shared totals, not three independent
additive measurements. No charged in-source test was moved to integration
tests and no existing test was removed to offset growth. Operator authority
for the exact figures remains outstanding; no repinning was performed.

### Verification commands and observed exit codes

Every Cargo build/test/clippy/deny invocation used `env -u CI -u GITHUB_ACTIONS`
and the Bash-inherited target `/mnt/build/cargo-maos`, unchanged in subprocesses;
no alternate target or clean was used. The initial formatter does not build
artifacts; subsequent invocations carried the inherited target explicitly
through the command runner's environment, not a caller-selected target override.

| Command / target, with the environment prefix above | Exit | Observed result |
|---|---:|---|
| `cargo fmt --all` | **0** | Final formatting before plants; all formatter attempts exited 0 |
| `cargo build --locked --workspace --all-targets` | **0** | Final restored build; earlier initial test-only compile failure retained |
| `cargo test --locked -p maos-kernel-core --lib memory` | **0** | **37 passed** |
| Kernel `principal_identity_20_3b`, `private_forget_restart_13_5i`, `private_spill_supersession_13_5j` | **0** | Final restored **27 + 10 + 18 passed** |
| Bin `erasure_uninstall_13_5b`, `maos_uninstall_16_4` | **0** | **26 + 23 passed** |
| Audit `private_tier_count_16_5`, `governance_roundtrip_20_3b` | **0** | **3 + 4 passed** |
| xtask `invariant_claims_20_3b` | **0** | **7 passed** |
| `cargo nextest run --workspace --locked --color never --no-fail-fast --test-threads 1` | **100** | Final source: **5,435 passed, 11 failed, 139 skipped**; runner **1,190.406s**, command **2,052.750s** |
| `cargo nextest run --workspace --locked --color never --no-fail-fast` | **100** | One explicitly requested default-thread run: **5,434 passed, 11 failed, 139 skipped**, runner **42.583s**, command **44.008s** |
| `cargo clippy --workspace --all-targets --locked --message-format=json` | **0** | Real final stream, **191.354s** |
| `clippy-ratchet --input <absolute round-3/clippy-final.jsonl>` | **0** | **1,410 occurrences / 881 source-distinct**, within **1,411 / 884** |
| `cargo deny check` | **0** | Advisories, bans, licenses and sources pass |

The explicit default-thread run preceded only the final path-specific
legacy-empty/NFS diagnostic, its additional passing test, and removal of
obsolete rename imports. The prescribed prepush also ran default-thread
nextest on the final source, including that 27th identity test. The final
serial command above ran after those changes and after all plant restores.
Both serial attempts and the explicit default run remain in the ledger.

The **11 failures are baseline-dependent**, not suppressed or excluded:
`t12b_kernel_core_byte_identical_line_count`; FKCS's independent current-kernel
baseline assertion; `escape_substrate_refusal_never_passes`; six
`kernel_pin_content_hash_16_0` assertions; the trial-attestation evidence
assertion; and `kloc_check_runs_on_workspace`. Escape/trial evidence is red
because its kernel-ABI leg sees the deliberately unadopted kernel pin.

Standalone gate exits, each with `--json`:
- **1:** `kloc-check`, `check-kernel-baseline`, including final restored measurements.
- **0:** `check-service-boundary`, `check-dev-record-completeness`,
  `check-epic-close-coherence`, `check-env-contract`, `invariant-lock`,
  `coverage-matrix`, `check-coverage-matrix-completeness`,
  `check-review-findings-resolved`, `check-test-cost`.

Initial diagnostics were fixed, not hidden: the first workspace build exited
**101** because the new integration test attempted the private store's private
`write` method; it was corrected to use the public memory-manager port. The
first real clippy ratchet exited **1**, **1,412 / 882**, because the replaced
publication left obsolete rename imports. Removing those imports restored
the unchanged **1,410 / 881** final counts. All failed attempts and their
exact commands remain in `round-3/commands.jsonl`.

### Four FR31 plants and restoration proof

All four final-code mutations compiled and reached their intended behavioral
assertions, not setup failures:

| Exact test selector | Exit | Intended assertion | Command seconds |
|---|---:|---|---:|
| `recycled_pid_cannot_disclose_another_spirits_principal_spill` | **101** | `Some(Markdown("A only"))` versus `None`, test line **47** | **3.445** |
| `recycled_pid_cannot_poison_butlers_acceptance_restore` | **101** | seven attacker records versus zero, test line **1009** | **8.554** |
| `subject_access_export_preserves_both_owners_when_pid_and_key_are_reused` | **101** | one row versus two, test line **111** | **3.093** |
| `small_non_markdown_principal_values_survive_restart_and_pid_change` | **101** | `None` versus `Some(Text("small"))`, test line **76** | **2.753** |

The source originals stayed in memory; reverse surgical edits restored all
three files byte-identically. `sha256sum --check <absolute
round-3/plant-source-originals.sha256>` exited **0**; all four unchanged
root-cut checksums also exited **0** from `landing-2`. Restored kernel focused
targets, the exact Butler poisoning witness, and the normal workspace
all-targets build each exited **0**. No mutation script or backup remains.

### Prescribed prepush and timing results

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated /mnt/build/cargo-maos/debug/xtask prepush --json
```

**Exit 1**, **578.148s**: **79 pass, 4 red, 1 advisory, 8 skip**. The four
red records are `check-kernel-baseline`, `kloc-check`,
`check-trial-attestation` and `workspace-nextest`, consistent with the
unchanged-pin results above. Kernel lib tests, workspace doctests, release/size,
tracked-tree and the other blocking legs passed. Its timing advisory has
two findings; its 81 named runtime diagnostics remain visible. No skipped
runtime scenario is claimed exercised. Conditional deny was skipped there;
the independently required standalone `cargo deny check` passed.

The final saved serial JUnit's separate `check-test-budget --junit <absolute
round-3/workspace-serial-final.junit.xml> --json` exited **1**:
576 binaries, sum **1,188.152s**, span **1,190.203s**, **8 findings**.
They are Butler **37.826s**, forget floors **40.980s**, hook pump **30.258s**,
observer activity corpus **113.868s**, one-daemon/one-door **84.290s**,
participant recall **38.646s**, xtask binary **35.949s**, and suite total
**1,188.152s > 1,100s**. These timing failures are reported as observed,
not rerun away; no timing exception or budget was changed. The prepush
default-thread timing judgment is separately advisory under the existing
contract. Separate serial/default/prepush JUnits are preserved.

### Remaining authority and unverified items

The implementation and requested verification commands are complete, but this
is **not** a green full-workspace/prepush claim. Raising the exact measured
pins remains an operator decision; no authority was assumed. Final evidence
edits follow verification; no post-evidence prepush run is claimed.

Live NFS, non-Unix compilation, hosted discipline/evidence execution,
signed-ledger artifacts/keys, hosted timing/marker judgments, live PostgreSQL,
signed-T3 runtime substrates, Python Ed25519 and strace scenarios remain
unverified/skipped. The accepted legacy-empty/NFS lock limitation above is
intentional and has a passing refusal witness. FR34/FR53's labelled ignored
reds remain unchanged. No commit, push, `.devcontainer/*` edit/stage,
alternate-worktree access, target override, `cargo clean`, or tracking of
`tests/corpora/migration-corpus-1e6.sqlite` occurred.

## Round-3 corrections — w10 adopted final verification

**Operator ruling w10, 2026-10-31: “A: Raise to measured exactly.”**
This authorizes the previously measured **+113 physical / +103 tokei** kernel
extension over w9, bin **+9**, audit STOP **7,668 → 7,707**, xtask **+61**
and aggregate measured-plus-one. Authority was received before any pin moved.
The four implementation fixes documented above are unchanged; this continuation
adopts their exact pins and reruns the requested verification.

Exact absolute commands, exit codes and durations are in
[`round-3/w10/commands.jsonl`](round-3/w10/commands.jsonl). Full logs, distinct
serial/repeat/default/prepush JUnits, the production-emitted pin block,
restoration checksums, real clippy stream, unchanged API snapshot and
[`round-3/w10/summary.json`](round-3/w10/summary.json) are alongside it.
The earlier `round-3/` root records remain the historical unadopted-pin runs.

### Exact pins and grant records

| Surface | Final formatted measured | Adopted exact pin / ceiling |
|---|---:|---:|
| Kernel physical | **26,793** | **26,793**, `src_lines` still line **505** |
| Kernel tokei | **20,992** | **20,992** |
| maos-bin | **33,030** | **33,030**, epic envelope **33,021 → 33,030** |
| maos-audit | **7,707** | **7,707**, STOP **7,668 → 7,707** |
| xtask | **42,841** | **42,841** |
| maos-domain | **9,977** | **9,977**, unchanged |
| Aggregate | **188,451** | strict hardfail **188,452** |

`cargo fmt --all` and final restored `kloc-check --json` /
`check-kernel-baseline --json` each exited **0**. No measurement exceeds w10.
Production `check-kernel-baseline --emit-pin` exited **0**; its two changed
file digests and set hash were adopted exactly. The 95-file set is unchanged,
and the final report has empty `changed`/`added`/`removed` arrays, with equal
actual/pinned set hash:

```text
a7719696a749180a3fa6b7b8d62de4f25e75ea2c14497297e7b6ada17a0e2cc2
```

Updated kernel-baseline HISTORY and KLOC kernel/bin/audit/xtask/aggregate rows
cite **`20-3b landing 2 round-3 corrections, operator ruling w10`**, retaining
prior grants under `PRIOR`. The xtask row's first grant segment retains the
complete open story key and **42780 -> 42841** arrow; D11's real-tree and
negative vectors pass. Updated all four kernel-pin reader literals,
`RATIFIED_AT_EASING = 20_992`, story frontmatter/grant table, Epic-20 grant
rows and Epic-19's bin-envelope extension. The current public kernel surface
has **401 items**, exactly equal to the existing baseline; the surface/class
metadata files are intentionally unchanged. No tests were relocated for offsets.

### Build, focused tests, both full suites and gates

All Cargo commands used `env -u CI -u GITHUB_ACTIONS`, with the unchanged
Bash-inherited **`CARGO_TARGET_DIR=/mnt/build/cargo-maos`** carried into
subprocesses. No target override or clean was used.

| Command / requested target set | Exit | Observed result |
|---|---:|---|
| `cargo fmt --all` | **0** | Before adoption and final restored measurement |
| `cargo build --locked --workspace --all-targets` | **0** | Initial adopted-pin and final restored builds |
| xtask D11 / kernel-pin / recovery-rule / invariant-claims integration targets | **0** | **9 + 25 + 12 + 7 passed** |
| Kernel `--lib memory` | **0** | **37 passed** |
| Kernel identity / forget-restart / spill-supersession targets | **0** | **27 + 10 + 18 passed**, also after plant restores |
| Bin erasure/uninstall and uninstall/purge targets | **0** | **26 + 23 passed**, also after plant restores |
| Audit private-tier-count / governance-roundtrip targets | **0** | **3 + 4 passed** |
| `cargo nextest run --workspace --locked --color never --no-fail-fast --test-threads 1` | **0** | **5,446 passed / 139 skipped**, runner **597.432s**, command **601.188s** |
| Same full serial command, one requested timing repeat | **0** | **5,446 passed / 139 skipped**, runner **597.320s**, command **599.083s** |
| `cargo nextest run --workspace --locked --color never --no-fail-fast` | **0** | **5,446 passed / 139 skipped**, runner **42.711s**, command **45.004s** |

**All 11 pin-dependent failures cleared.** No failing test was filtered out
or weakened. Both requested modes use the existing workspace filter and retain
the 139 named skips; kernel lib tests are also exercised by prepush.

Each standalone gate below ran with `--json` and exited **0**:
`kloc-check`, `check-kernel-baseline`, `check-service-boundary`,
`check-dev-record-completeness`, `check-epic-close-coherence`,
`check-env-contract`, `invariant-lock`, `coverage-matrix`,
`check-coverage-matrix-completeness`, `check-review-findings-resolved`,
`check-test-cost`, `check-serde-error-handling`,
`check-composition-root-completeness`, `check-j1-loopback-delegation`
and `check-trial-attestation`.

### All eight prior serial timing findings, individually reassessed

The requested fresh serial timing judgment and one additional fresh
serial-suite/timing repeat both exited **0**, **zero findings**. The tests,
production implementation and budgets did not change between these two runs.
Both saved JUnits contain 576 binaries; first sum/span **597.216s/597.406s**,
repeat **597.146s/597.304s**. The previous eight findings are retained below
rather than erased from history:

| Prior finding | Prior seconds | w10 serial | Fresh serial repeat | Limit | Repeated? |
|---|---:|---:|---:|---:|---|
| `maos-bin::butler_scene_18_4` | 37.826 | **8.652** | **8.840** | 30 | No |
| `maos-bin::forget_floors_18_6` | 40.980 | **12.495** | **12.483** | 30 | No |
| `maos-bin::hook_pump_17_4` | 30.258 | **21.004** | **21.602** | 30 | No |
| `maos-bin::observer_activity_corpus_17_4` | 113.868 | **6.445** | **6.026** | 30 | No |
| `maos-bin::one_daemon_one_door_16_1` | 84.290 | **25.694** | **25.699** | 30 | No |
| `maos-iac::participant_recall_17_8` | 38.646 | **25.182** | **25.931** | 30 | No |
| `xtask::bin/xtask` | 35.949 | **15.980** | **16.184** | 30 | No |
| Suite sum | 1,188.152 | **597.216** | **597.146** | 1,100 | No |

**Attribution for each row:** [INFERENCE] host/runtime timing variance, not a
new production-code regression. The Round-3 production implementation is
unchanged since the historical high-time run; only authorized pin/HISTORY/
test-literal and grant records changed. In particular none of these seven
binaries received a code or budget repair to clear its timing. Two independent
fresh serial runs are now below every limit. No host-load trace was collected,
so a specific CPU/I/O scheduler cause is not claimed. **No exception rows or
budget changes were added.**

### Real clippy and dependency policy

`cargo clippy --workspace --all-targets --locked --message-format=json`
exited **0**, **239.125s**. The real built `clippy-ratchet --input <absolute
round-3/w10/clippy-live.jsonl>` exited **0**, **0.197s**, counting
**1,410 occurrences / 881 source-distinct** within unchanged **1,411 / 884**.
No warning suppression or baseline increase was made.

`cargo deny check` exited **0**, **0.940s**: advisories, bans, licenses and
sources all **ok**. Existing unnecessary-skip configuration warnings remain
visible; no deny configuration was changed. The standalone recheck, not
prepush's conditional deny skip, supplies the required dependency-policy proof.

### Four w10 FR31 plants and restored-green proof

Each mutation compiled in the real checkout and failed at the intended
behavioral assertion, **exit 101**:

| Test selector | Assertion | Command seconds |
|---|---|---:|
| `recycled_pid_cannot_disclose_another_spirits_principal_spill` | `Some(Markdown("A only"))` versus `None`, line **47** | **3.910** |
| `recycled_pid_cannot_poison_butlers_acceptance_restore` | seven attacker records versus zero, line **1009** | **9.569** |
| `subject_access_export_preserves_both_owners_when_pid_and_key_are_reused` | one row versus two, line **111** | **3.225** |
| `small_non_markdown_principal_values_survive_restart_and_pid_change` | `None` versus `Some(Text("small"))`, line **76** | **3.187** |

Original bytes stayed in memory; reverse surgical edits restored all three
source files. `sha256sum --check <absolute
round-3/w10/plant-source-originals.sha256>` exited **0**, as did all four
unchanged root-cut checksums from `landing-2`. Restored kernel focused targets,
both bin uninstall targets, the exact Butler poisoning witness and the final
normal workspace all-targets build each exited **0**. The final formatter and
KLOC/kernel measurements also exited **0** at the exact w10 figures. No
temporary mutation script or backup remains.

### Prescribed final prepush

```sh
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated /mnt/build/cargo-maos/debug/xtask prepush --json
```

**Exit 0**, **765.908s**: **83 pass, 0 red, 1 advisory, 8 skip**.
Kernel lib tests, workspace doctests, workspace nextest, release/size,
tracked-tree and all blocking legs pass. Its JUnit is saved separately from
both serial and the explicit default run.

The one advisory is prepush's independent default-thread timing judgment
(**exit 1**). A separate read of its saved JUnit with `check-test-budget`
preserves all eight findings: forget floors **35.997s**, hook pump **32.021s**,
one-daemon/one-door **37.595s**, participant recall **52.329s**, xtask binary
**56.918s**, ledger collectors **44.291s**, macOS compile guard **52.934s**
(each versus 30s), plus sum **1,286.601s > 1,100s**. This remains advisory
under the existing local prepush contract; it does not replace the two
independently green serial budget checks. No exception or timing budget changed.

Eight skips remain: docs build/routes, conditional deny and five hosted-marker
cardinality checks. Prepush's **81 named runtime diagnostics** remain disclosed.
No unavailable signed-T3, Python-Ed25519 or strace scenario is claimed run.

### Completion and limits

The w10 pin authority is fully adopted and all requested blocking verification
is green. The four implementation fixes and their file/test anchors remain
documented in the historical Round-3 implementation section above. Accepted
legacy-empty/NFS `EBADF` refusal remains intentional, path-bearing and covered
by its passing test; fresh creation and reads pass simulated EBADF tests.

Live NFS, non-Unix compilation, hosted discipline/evidence and signed-ledger
artifacts, hosted timing/marker judgments, live PostgreSQL and the unavailable
runtime substrates remain unverified. FR34/FR53's labelled ignored reds are
unchanged. No commit, push, `.devcontainer/*` edit/stage, alternate-worktree,
target override, clean or migration-corpus tracking occurred. Final evidence
edits follow verification; no post-evidence prepush run is claimed.
