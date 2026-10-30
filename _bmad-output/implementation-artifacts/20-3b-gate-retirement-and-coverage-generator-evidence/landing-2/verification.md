# 20-3b landing-2 verifier/integrator — final evidence

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

No commit,push,cargo clean,alternate-worktree access or Dockerfile edit occurred. All mutation scaffolds were restored/removed. The real generated migration corpus remains as the documented local prerequisite. The operator's uncommitted Dockerfile edit was left alone. Prepush's tracked-tree check passed.
