# Story 18-6 plant evidence

All mutations run in place in the real tree, with `CARGO_TARGET_DIR=/mnt/build/cargo`, `CI` and `GITHUB_ACTIONS` unset, and one cargo process. Each modified production file is copied aside before mutation, copied back afterward, compared with `cmp`, and touched to force incremental rebuild. No matrix or hosted-run claim is made by local execution.

## Integrator-approved premise corrections

- **AC3.5:** the live MCP cache originally held invocation argument frames, not old fetched claims. `input_frames` re-fetches current claims and joins by source key on every pass. Serve-once MCP removes the canary, so skipping purge alone cannot produce the prescribed live next-pass canary resurrection. Main approved an honest cache scrubbed-frame-id/source-reference assertion, with a unit-seeded pending cached claim next-pass body check where reachable. Production is not changed to manufacture a resurrection.
- **AC1.2:** `DistillateWriter::write_distillate` (`adapter/distillate.rs:337-358`) rejects effective raw source writer pids different from the citer. Main approved a genuine same-pid digest-of-digest plus a historical cross-writer TL row inserted through the real TL adapter, explicitly labelled historical row, not new valid authoring. Citation authorization remains unchanged.
- **Historical-row injection detail:** direct Distillate insertion requires an unforgeable writer token. The historical fixture first creates its valid receipt through the real `DistillateWriter`/TL adapter, then explicitly adjusts persisted writer attribution with SQL; it does not claim that current cross-pid authoring is valid.
- **ADR-064 composition:** replay inference plus `--live` is refused by the existing selector. Main approved a real daemon-door witness using bound non-`--live` seed replay, plus `mcp_sourced_forget_scrubs_unindexed_writer_and_redistills_live` in the root-library binary for real MCP adapter + replay inference + TL/DistillateWriter + the door's forget-completion route.
- **No scheduled-repeat process claim:** the idle watchdog fires once per quiet period. A door reload creates a fresh Researcher and re-journals its constant seed claims. Main assessed this as a fixture artifact: the process-witness principal `noisy corpus.` deliberately collides with built-in seed vocabulary, which forget cannot and need not erase from the Spirit's compiled constants. The same-instance next-pass-clean proof is in the root-library witness and Researcher cache-purge unit test, not a claimed repeated daemon idle.

## Setup failure, not control evidence

The initial AC2 closure mutation reached a fixture authorization failure (`CiterUnauthorized { citer_pid: 7, source_pid: 8 }`) before forget ran. It is not counted as proven-red. The production file was restored byte-identically (`cmp` exit0) and touched. Command/output/restoration are in `ac2-fixture-failure.log`. The same closure mutation is used after correcting the fixture; no additional control mutation is introduced.

## AC1 / AC4 — shared index-filter mutation; AC5 local CI simulation

One identical mutation proves both scan-reach controls: after discovering the
full closure, retain only frames whose writer pid occurs in the principal index.
The checked-in diff is `ci/plant.diff`; `sha256sum -c ci/plant.diff.sha256` passed,
then `git apply --check` and `git apply` succeeded in the real tree.

Commands (all with `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`):

- `cargo build --locked -p maos-bin --bin maos`
- `cargo nextest run --locked -p maos-journey-test --test journey_forget_18_6 -E 'test(=live_forget_redistills_and_never_resurrects_then_pending_is_typed)'`
- `cargo nextest run --locked -p maos-bin --test forget_floors_18_6`

AC1: exit 100, `canary leakage in researcher:seed_claim`, with `seed-rag-noisy`
still present; 1 failed / 1 skipped, 2.849 s nextest runtime. The exact hosted
template red-oracle predicates (nonzero exit, `canary leakage`, `seed-rag-noisy`)
all passed locally. Full output: `ci/local-planted.log`.

AC4: exit 100, `canary leakage in mcp:forget-witness`, principal
`floor-001@example.org`, canary `CANARY-forget-18-6-001`; 3.967 s nextest runtime.
The production floor deliberately fails fast at the first unindexed leak;
this is not presented as an observed 49/50 aggregate. Output: `ac4-red.log`.
Unplanted, the binary completes all 50 erasures, all 50 marker checks and all
100 store probes.

Restore:
`cp /tmp/18-6-ac14-memory.rs crates/maos-kernel-core/src/memory/mod.rs && cmp /tmp/18-6-ac14-memory.rs crates/maos-kernel-core/src/memory/mod.rs && touch crates/maos-kernel-core/src/memory/mod.rs`.
Exit 0, byte-identical. Rebuilt the daemon, then the journey + floor binaries
passed 3/3 in 11.547 s (`ac14-restored-green.log`). Full wall-clock from backup
through both red controls, restore, rebuild and green: **171.16 s**.

**AC5:** `ci/ci-18-6-verify.yml`, diff and checksum are ready. The local
checksum/apply/build/expected-red/restore/green simulation passed once. Hosted
`ci/18-6-verify` run: **integrator, at landing; no hosted-run claim at review**.

## AC2 — derived closure

Mutation: replace the closure's `frames.push(frame)` with a comment, leaving the
principal-body scan intact. The genuine same-pid digest-of-digest control failed:
expected 2 redacted distillates, observed 1 (exit 100; red command 2.67 s wall).
Command and assertion: `ac2-red.log`; the earlier fixture setup failure is
separately labelled above. Copy-back + `cmp` + `touch` returned 0.
The restored kernel suite passed 7/7; together with the floor binary, 8/8 passed
in 10.522 s (`focused-restored-summary.log`).

The exact start-to-green wall-clock was not instrumented for AC2. A conservative
observed upper bound from backup mtime 1792992677 through the later evidence
capture at 1792996660 is **3983 s**, including intervening work; it is not
misrepresented as command runtime.

## AC3 — live cache purge

Mutation: make the single `scrubbed` predicate return false, retaining scrubbed
frames in every shared cache and preserving cached last output. Run:
`cargo nextest run --locked -p researcher -E 'test(forget_purges_all_shared_caches_before_the_next_pass)'`.
It failed on `researcher.last_output().is_none()` (exit 100; 1 failed, 59 skipped,
0.017 s nextest runtime). A preceding wrong exact module selector ran zero tests
(exit 4) and is not accepted as control evidence.

Restore:
`cp /tmp/18-6-ac3-researcher.rs spirits/researcher/src/lib.rs && cmp /tmp/18-6-ac3-researcher.rs spirits/researcher/src/lib.rs && touch spirits/researcher/src/lib.rs`.
Exit 0, byte-identical. The same control then passed 1/1 (59 skipped), 4.04 s
command wall; all 60 Researcher tests also passed in the final broad run.
Exact full-cycle timing was not instrumented; the conservative backup-mtime
1792994711 → evidence-capture 1792996660 upper bound is **1949 s**, including
intervening work, not test runtime.

## Count and final proof

Four AC control proofs, **three distinct mutations**: AC1/AC4 share the identical
index-filter mutation, AC2 drops closure, AC3 disables purge. AC5 reuses the
single AC1 expected-red execution for the local CI-template simulation; no
cosmetic re-plants were run. Final broad focused run: **2077 passed / 23 skipped**;
remaining reader set: **94 passed / 38 skipped**; kernel libtest: **258 passed**.
All affected new/expanded binary runtime sums are below 30 s
(`new-binary-budgets.json`).
