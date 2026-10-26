# Story 18-6 plant evidence

All mutations run in place in the real tree, with `CARGO_TARGET_DIR=/mnt/build/cargo`, `CI` and `GITHUB_ACTIONS` unset, and one cargo process. Each modified production file is copied aside before mutation, copied back afterward, compared with `cmp`, and touched to force incremental rebuild. No matrix or hosted-run claim is made by local execution.

## Integrator-approved premise corrections

- **AC3.5 — superseded at review (BH-1 / EC-1 / AA-1):** the implementation introduced cached fetched claims, so the original “arguments only” premise was false. Review removes that obsolete cache, regenerates only from current TL `payload_redacted`, and clears last output on every non-empty notice. The revised MCP witness places the principal solely in response text, never invocation arguments; with argument-only survivors it returns typed `no-claims` pending without inference or a new distillate. The controls below supersede the old premise, rather than manufacturing a resurrection path.
- **AC1.2:** `DistillateWriter::write_distillate` (`adapter/distillate.rs:337-358`) rejects effective raw source writer pids different from the citer. Main approved a genuine same-pid digest-of-digest plus a historical cross-writer TL row inserted through the real TL adapter, explicitly labelled historical row, not new valid authoring. Citation authorization remains unchanged.
- **Historical-row injection detail:** direct Distillate insertion requires an unforgeable writer token. The historical fixture first creates its valid receipt through the real `DistillateWriter`/TL adapter, then explicitly adjusts persisted writer attribution with SQL; it does not claim that current cross-pid authoring is valid.
- **ADR-064 composition:** replay inference plus `--live` is refused by the existing selector. The daemon-door witness uses bound non-`--live` seed replay; the root-library MCP witness is now `mcp_response_only_principal_is_purged_without_cached_claim_resurrection`, combining real MCP + governed recall + replay inference + TL/DistillateWriter + forget completion.
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

## Review-patch controls (2026-10-26)

All commands retain the environment and one-cargo discipline above. `review-plant-results.json` is the machine-readable ledger; each row below has a same-named `.log` containing the complete cargo output, exact command wall, copy-back + `cmp` + `touch` exit codes, and full-cycle wall; `.diff` contains its mutation. Restored final proof is `review-package-final-green.log` (838/838), `review-scenario-final-green.log` (107/107), `review-cascade-restored-final.log` (13/13) and `review-kernel-lib.log` (258/258).

| Mutation | Accepted control red? | Full-cycle wall (s) | Falsifier |
|---|---|---:|---|
| `unicode-id` | yes | 7.315 | `principal_matching_respects_unicode_and_punctuation_bridges` |
| `punctuation-bridge` | yes | 3.608 | `principal_matching_respects_unicode_and_punctuation_bridges` |
| `direct-parent` | yes | 11.297 | `clean_raw_ancestry_does_not_hide` |
| `tombstoned-source` | NO — excluded | 3.691 | `clean_raw_ancestry_does_not_hide` |
| `held-coprincipal` | yes | 3.195 | `held_coprincipal` |
| `exact-tombstone-prior` | NO — excluded | 3.267 | `partial_redaction_is_not` |
| `uninstall-evidence` | yes | 3.393 | `partial_redaction_is_not` |
| `empty-principal` | yes | 3.395 | `partial_redaction_is_not` |
| `malformed-lineage` | yes | 3.216 | `malformed_distillates_are_scrubbed` |
| `tombstoned-raw-source` | yes | 3.216 | `an_erased_raw_source` |
| `exact-prior-repaired-witness` | yes | 3.332 | `an_unrelated_forget` |
| `unconditional-output-purge` | yes | 10.558 | `response_only_principal_is_not_resurrected` |
| `retry-cache-revalidation` | yes | 4.092 | `retry_notice_rechecks_tombstones` |
| `authoritative-tl-payload` | yes | 6.251 | `redistill_fetches_only_tl_survivors` |
| `observer-forget-is-not-curation` | yes | 13.837 | `forget_notices_do_not_close` |
| `no-raw-recall-fallback` | yes | 21.311 | `root_redistill_uses_governed` |
| `root-principal-rescan` | yes | 7.561 | `regenerated_bodies_are_rescanned` |
| `fresh-regeneration` | yes | 5.277 | `regenerated_bodies_are_rescanned` |
| `non-tombstone-result` | yes | 4.904 | `regenerated_bodies_are_rescanned` |
| `surviving-source-subset` | yes | 4.625 | `regenerated_bodies_are_rescanned` |
| `late-frame-sweep` | yes | 4.216 | `in_flight_pass_is_scrubbed` |
| `orphan-request-replay` | yes | 5.753 | `retry_after_distillate_scrub` |
| `quarantine-independent-purge` | yes | 7.239 | `privacy_notice_bypasses_quarantine` |
| `typed-undelivered-purge` | yes | 7.017 | `undelivered_purges_are_typed` |
| `post-erasure-pending` | yes | 5.421 | `undelivered_purges_are_typed` |
| `late-result-journal` | yes | 7.256 | `a_late_hook_result_appends` |
| `durable-request-metadata` | yes | 7.714 | `retry_after_distillate_scrub` |
| `late-cache-followup` | yes | 5.185 | `in_flight_pass_is_scrubbed` |
| `purge-quarantine-accounting` | yes | 7.187 | `a_late_hook_result_appends` |
| `uninstall-error-outcome` | NO — excluded | 5.613 | `redistillation_failure_after_erasure` |
| `uninstall-error-outcome-repaired` | yes | 4.087 | `redistillation_failure_after_erasure` |
| `transitive-parent-closure` | yes | 3.916 | `clean_raw_ancestry_does_not_hide` |

**29 accepted review-control mutations, three unsuccessful attempts excluded.** The initial tombstoned-distillate test was independently refused by receipt parsing, and the initial partial-redaction test never forgot an unrelated principal; those surviving plants led to new raw-tombstone and unrelated-forget tests, both green and then proven red. The first uninstall proof assertion used a wrong JSON path and failed unplanted; its initial mutation is not proof. The repaired signed-proof witness is green and its mutation is accepted.

**One regenerated shared AC1/AC4 mutation (30 distinct accepted mutations total):** `ci/plant.diff` now filters the shared IAC sweep back to index-associated pids, covering both initial and late sweeps. Checksum and `git apply --check` passed; applying it, rebuilding the daemon and executing the actual process control failed on `canary leakage` / `seed-rag-noisy`. The floor control failed on `CANARY-forget-18-6-001`. Copy-back + `cmp` + `touch` all returned0; rebuilt process control green. Complete simulation wall **98.980s**, commands and restoration in `review-ci-simulation.log`, with per-command outputs in `review-ci-*.log`. Hosted execution remains an integrator landing obligation, not a local claim.

Final new/expanded binary sums are in `review-binary-budgets.json`: HookPump26.895s, cascade5.588s, floors15.167s, MCP1.734s, uninstall8.922s, all≤30s. The unchanged existing Researcher-constructor exception remains47s (observed41.503s); no exception was added.
