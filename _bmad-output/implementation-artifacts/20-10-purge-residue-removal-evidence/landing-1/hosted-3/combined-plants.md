# Final combined follow-up: custody, vanished candidates and S5 oracle

Baseline: `73e7afbcae81f4fb4858e9d52ded3d592bed636d`. The earlier `plants.md`, `implementation.diff`, `verify-cut.diff`, and88-test measurements preserve the initial custody-only phase. This record and `combined-*` artifacts cover the final tree after Main added two more diagnosed items.

## Scope and observations

1. **Production AC5 custody:** hosted verification41803510235 removes the owned CIDs but leaves spawn custody. Podman's cidfile removal invalidated the old late read. Capture a valid CID with held no-follow private/euid-owned directory before removal; require a fresh bounded absence proof and ownership/mode/device/inode identity before clearing and rmdir. Missing/invalid-at-inventory remains conservative. See `hosted-diagnosis.md` and `custody-scan.md` (custody-only source-line snapshot).
2. **Production enumeration:** Main reports workspace job126484784599 in main run41803503598 failing root-crash preflight when a shared-runtime candidate disappears before inspect. Nonzero inspect now probes bounded `container exists`: only exit1 skips silently; exit0,125, timeout or any ambiguity remains fatal. Inspect exit status/stderr are included in failures. Success with incomplete output and inspect timeout remain fatal, not silently skipped. The conditional exists probe is read-only and applies during dry inventory; Main explicitly adjudicated that clarification, reflected in AC3/ADR/operator English/Korean wording. No retry or mutation is added.
3. **Test-only S5:** loom job126484785052 in the same run counted2 budget rows. Retain frame kind and count exactly one `BudgetExceeded`, never infer kind from ratio. The original injected behavior/payload checks remain. Both in-process World constructors now use one shared terminal-binary `Once` clock initializer. Source grep finds exactly that callback and two callers. No production clock, budget, timeout or assertion-count relaxation. Historical hosted frame kinds/reset trace were absent; warning+exceeded and clock-offset attribution are source-supported inference, not a captured clock trace. See `s5-hosted-diagnosis.md`.

## Failing-before / mutations / restoration

| Final witness | Before | Mutation and observed red | Final |
| --- | --- | --- | --- |
| `purge_residue_20_10.rs:278` `ac5_only_absent_cid_and_empty_state_are_reclaimed` | Faithful `rm` unlinks the corresponding cidfile; original production exits100 on removed-container custody leaking | Discard captured proof:100 on `absent CID custody leaked` | PASS |
| `:341` missing-at-inventory CID | Negative boundary | PID/custody retained and receipted; no deletion authority from missing file | PASS |
| `:365` path replacement during removal | Negative identity boundary | Original held and replacement directory contents both retained | PASS |
| `:164` `ac1_listed_candidate_vanishing_before_inspect_is_skipped` | Current pre-race-fix code exits100 on `vanished listed candidate must not abort read-only purge` | Delete absent-candidate skip:100 on the same intended assertion, with captured inspect status/stderr | PASS |
| `:200` failed-inspect negative | Present/error/deadline boundary | exists==0,125 and a10s stub bounded at5s all retain configuration refusal78, with inspect evidence and no mutation/receipt | PASS |
| `unload_scenes.rs:21` S5 typed oracle | Old ratio predicate sees warning2.422539097 + exceeded2.622919372 and counts2;100, expected1 | Restore old ratio filter:100, left2/right1 | PASS |

All three mutations were saved and individually copy-backed, compared byte-for-byte, touched, then restore intermediaries removed (every restore command exit0). Final `cargo fmt --all` exit0. No active plant. Exact commands/exits and expected nonzero classifications are in `commands.md` / `commands.json`. `diff -u` exit1 means differences found, not test failure. No failed hosted timing test was retried locally to call it green; the S5 old predicate is exposed by a deterministic pure-row vector.

### Mutation SHA-256

- Capture discard: `ec5b121aff6690f803271519b7809f8b1bbfb128c17c8f99ee2a274558aebc31`
- Vanished-skip deletion: `8328de1a2a868ba1735075fccc244d695cbf6c85f1eae701e1c2b6610c6ce612`
- Old S5 ratio filter: `9dbbe447b0f2ecd88aaa4971a5b10222482a22545db59c186af02c1adc4a1cfb`

## Final serial and lint matrix

- Registered fixture provisioning0:5 artifacts, required scene siblings current.
- Focused serial nextest0: **91/91**,103.118s,0 nextest skips: egress5, uninstall23, purge16, shutdown12, terminal35. Saved `combined-focused-junit.xml`.
- Library runtime safety0: **3/3** (21 unrelated library tests filter-excluded).
- Executed workflow contracts0: **5/5**. Real checker accepts exactly1 required scene with34 excluded tests; rejects explicit SKIP, wrong required name and zero run.
- Non-denying workspace/all-target clippy0, Rust1.98.1; complete-stream ratchet0 at **1331 occurrences /871 source-distinct**, baseline1383/897 unchanged. Complete20-4 corrected local baseline comparison: **no new lint/file/message multiplicities**. Existing xtask warning1 and supervisor warnings2 are unchanged; other touched Rust files have none. `combined-clippy-delta.json` preserves sites.
- All structural/record gate commands and observed outcomes are in the final ledger; no claimed hosted toolchain lint or runtime result.

Local real-runtime scene returns remain substrate-gated; neither the91-pass aggregate nor the pure S5 vector certifies corrected real Podman/cgroup/S5 hosted execution. Parent requested focused/serial, formatting, lint delta and structural/record gates, not a new full workspace/default/prepush/docs-site matrix.

## Orchestrator-adjudicated budget re-base — 2026-11-03, D02 A

Main explicitly adjudicated the combined required fixes under existing inherited ceilings, not a widening: **maos-bin<=34643**, **aggregate<=193526**. Original20-10 bin cap34548 was based on pre-20-4 bin34191; inherited current bin34504 changes the absolute-base context. Stop above the adjudicated caps.

| Measure | Inherited | Final | Delta | Cap / exact pin |
| --- | ---: | ---: | ---: | --- |
| maos-bin |34504|**34551**|+47 (+33 custody,+14 enumeration)|<=34643; exact34551 |
| Aggregate |193140|**193187**|+47|<=193526; hardfail**193188** = measured+1 |
| xtask |44452|**44452**|0|unchanged |
| Kernel physical/code |26841/21042|unchanged source|0|baseline gate in final ledger |

Measurements follow final formatting. S5/test-file changes are uncharged by the existing gate; only the production delta moves counts. KLOC row first HISTORY segments cite the live20-10 story key and `->`; prior histories are preserved. No other grant/baseline is changed.

## Workflow cut and source SHA-256

- `combined-implementation.diff`: `93ce55e728b7c6d3905155d495e2a8bce486ad1b417bbc75f17d2ab4192045e8`
- `combined-verify-cut.diff`: `d018ec708e111b9d90f65101f08e51ff1693706d8b669efd641d54ff85794496`
- Final full `ci-20-10-verify.yml`: `b5394fe3cff775e8e242e8c3c79c292f6a8fa5fa5603a0763c96b7b09c5590c4`

`diff-sha256.json` retains both phase snapshots and all mutations. Main discipline and its reader now pin **35** terminal tests and require the new S5 oracle. Branch-only cut pins **21** hermetic and35 terminal, while restored green still requires exactly1 named PASS. Baseline/plant intended removal/dead-leaf diagnostics and copy/cmp/touch stay strict. Existing removal no-op patch still passes `git apply --check` (0). No cut installed under `.github`, no CI-branch edit or push; only the existing main discipline test-count/required-name pin changed.

## Lesson and cleanup

S5's same-line rule-10 note is appended to the existing20-7-owned `deferred-work.md:4`: samples2.422539097s/2.622919372s, gap200.380275ms, and the18-0 clock-reinit precedent (`hook_pump_17_4.rs:60-65`). This does not close unrelated timing families or claim a traced clock jump. Production custody/race causes have separate owned20-10 disposition rows.

Granular cleanup removed19 old sessions only after a29-PID `/proc` scan, no candidate references/active Rust tooling/same-UID denial, and nonblocking flock+POSIX locks. Allocated bytes unlinked:234217472; observed free-space increase after flush:167460864 bytes (filesystem-wide, not exclusive physical reclamation). Current sessions, crate roots, locks, other crates, deps and target roots retained; no cargo clean/broad deletion. After the final combined jobs, a new27-PID scan finds no Rust tooling; all three restore intermediaries and named workflow scratch are absent (`cleanup-final.json`).

Unverified external legs: corrected real hosted head/restored-plant and shared-runtime race behavior, real S5/libtest clock isolation, and hosted-toolchain lint. Existing valid base/plant reds and cgroup passes from41803510235 remain honestly recorded, not relabeled as corrected hosted green.
