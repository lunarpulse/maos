# RC.1 tag and release record (2026-11-14)

Post-tag record for `v1.0.0-rc.1`. Times are UTC. Facts come from run logs and orchestrator checks. Anything not observed is marked [INFERENCE] or listed under section 8.

## 1. State at record time

- main: `40f7b3d8` (docs PR #13 merged).
- Tag `v1.0.0-rc.1`: lightweight tag on `1a57035e61741c2d7a49521a1466771b89659a2a`, pushed 2026-11-14T02:07:07Z under operator ruling 13. Only that ref was pushed. The tag has not been moved or deleted.
- Release: NOT published. release.yml run 43544486838 failed at step 17. No GitHub Release exists for rc.1 (the only release is `v0.1.0-alpha.1`). No `.deb`, `.rpm` or `SHA256SUMS-packages` were attached.
- `ops-rc-signed-tag`: stays `backlog` (not done).
- Next step: operator ruling of 2026-11-14, path A (section 6).

## 2. Timeline

| When (UTC) | Event |
|---|---|
| 2026-09-25 | `v0.1.0-alpha.1` tagged at `0c661995` (main tip); release.yml run 36094027381 green (`ops-first-signed-tag` done). |
| 2026-11-12 15:03 | Candidate `1acac36e` pushed. Discipline run 43243673212 failed only on check-ff-j6 (plus the fan-in ship gates and aggregate). The provisioning-checklist note named job id `j6-real-measurement`, which the whole-file scan read as a J6 latency binding. Release rehearsal 43243725733 at `1acac36e`: success (sign-and-publish skipped). T3 chain 43243720169 cancelled. |
| 2026-11-12 ~16:10 | Candidate `80b5b931` pushed. Discipline run 43255653207, attempt 1, failed on check-multi-tenant-loom (`one_daemon_one_door_16_1::cold_swap_refuses_mira_before_unload` got `cold_swap_predecessor_unknown`; pre-existing race, first hosted occurrence in ~15 runs) and check-test-budget (`one_daemon_one_door_16_1` 30.17 s vs 30 s; `erasure_million_20_11` 85.75 s vs 78 s). Diagnosis: `_bmad-output/implementation-artifacts/cold-swap-admit-race-evidence/diagnosis.md`. |
| (after attempt 1) | Rerun of workspace-test-suite job 131313055404 failed because of toolchain drift, not a single-job rerun: rustc 1.99.1 changed to 1.100.0 (hosted stable moved 2026-11-09). Cargo 1.100 places test executables under `target/<profile>/build/<pkg>/<hash>/out/`, breaking `tests/harness/fixtures.rs` and ~60 `current_exe()` tests. 116 test failures. |
| (after attempt 2) | ~20 jobs red. |
| (date not in source notes) | Operator validation run 43239794859 (582f9509): tier-2-rerecord failed (journey_j0 harness bug; judge HTTP 400 with body discarded at io/mod.rs:107-111). Diagnosis: `_bmad-output/implementation-artifacts/live-rerecord-400-diagnosis/README.md` (posttag worktree). |
| (date not in source notes) | Early rehearsal 43227542864 at `23b2200b`: success. |
| 2026-11-12 ~22:15 | Cleanup (report `/home/lunarpulse/cleanup-report-2026-11-12.md`): `cargo clean` of `/mnt/build/cargo-maos` (~216 GiB to 3.2 GB); sccache 13 GB to 247 MB; /tmp 7.1 GB to 36 MB. |
| (date not in source notes) | Toolchain fix: `rust-toolchain.toml` pinned to 1.99.1 with 4 targets, commit `1a57035e` (PinToolchain). |
| (date not in source notes) | Release rehearsal 43326196621 at `1a57035e`: green (workflow_dispatch; sign-and-publish skipped). |
| 2026-11-13 00:09:31 | Discipline aggregate green at `1a57035e`: 207 success, 5 skipped, 0 failure. |
| 2026-11-13 00:10:55 | T3 chain dispatched: fuzz-prerelease run 43334961225 at `1a57035e`. |
| 2026-11-13 ~03:00 | Dry run at `1a57035e`: check-release-precondition refused only the T3 fuzz leg (expected); stability-matrix PASS; check-rc-readiness PASS. |
| 2026-11-14 ~02:00 | T3 chain complete: run 43334961225 success (~26 h). |
| 2026-11-14 ~02:05 | Pre-tag checks at `1a57035e` (section 3): all PASS. |
| 2026-11-14 02:07:07 | Tag `v1.0.0-rc.1` pushed. |
| 2026-11-14 | release.yml run 43544486838 (push of the tag): 3 builds success; sign-and-publish FAILED at step 17 (section 4). |
| 2026-11-14 02:31-02:33 | Attempt 2 (re-run of the failed job) failed identically. |
| 2026-11-14 04:44-05:05 | relfix dry-run: release.yml run 43558157808 (workflow_dispatch, dry_run=true) on `relfix-2026-11-14` at `566c6f72`. Failure by design (main ancestry and discipline aggregate guards, non-main commit). Sign-and-publish steps 1-20 success, including step 17 for both arches; step 21 Create GitHub Release skipped (section 5). |
| 2026-11-14 | Operator ruling, path A (section 6). |

## 3. Pre-tag facts at `1a57035e` (2026-11-14 ~02:05Z)

- check-release-precondition: PASS (discipline aggregate plus complete T3 chain for all 3 targets).
- `cargo metadata --locked --offline`: rc=0.
- `stability-matrix --check`: rc=0.
- `check-rc-readiness --tag v1.0.0-rc.1`: rc=0.
- origin/main == `1a57035e`; no remote tag existed before the push.
- Discipline aggregate at `1a57035e` also includes air-gap-netns-witness green (per the orchestrator's check).

## 4. T3 chain (fuzz-prerelease run 43334961225)

- Dispatched at `1a57035e` 2026-11-13T00:10:55Z; completed success 2026-11-14 ~02:00Z (~26 h).
- 5 links x 3 targets: all success. t3-ledger-collect: success.
- Each link is ~5 h 15 min. The 72004 figure is ONE link (4 workers x 18000 s), not a cumulative total.
- Corpus minimization between links worked: manifest_parser 50753 to 3084 files; fuzz_exec_deps_parse_elf 6353 to 218; frame_deser 450480 to 33254.
- fuzz-ledger: 5 T3 records at `1a57035e`. libFuzzer cpu_seconds per target: manifest_parser 360020, fuzz_exec_deps_parse_elf 360020, frame_deser 360022 (floor 86400).

## 5. Release failure (run 43544486838)

- Steps 1-16 green: main ancestry, tag classification, rc readiness, aggregate, perf label, signing, dev-key refusal, self-test verify, recipes render.
- Step 17 "Build deb and RPM for both Linux architectures without rewriting binaries" failed: `docker: cannot overwrite digest sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852` (exit 125) on the arm64 iteration. The amd64 iteration had already pulled the same multi-arch index digest. Docker 28.0.4, overlay2.
- Attempt 2 failed identically, so the failure is deterministic (observed twice).
- At the rc.1 run, steps after 17 had never run hosted. The workflow_dispatch rehearsal skipped the whole sign-and-publish job; commit `ebf2ab03` recorded "hosted legs unverified".
- Nothing was published.
- Cause: [INFERENCE] the two iterations share one multi-arch index digest and the second docker operation collides (docker 28.0.4, overlay2, per the rc.1 log). Not reproduced with the pre-fix workflow. Fix on branch `relfix-2026-11-14`, commit `566c6f72`: see the dry-run below.
- relfix dry-run: release.yml workflow_dispatch run 43558157808 on `relfix-2026-11-14` at `566c6f72`, dry_run=true, 2026-11-14 04:44Z to 05:05Z. Overall conclusion failure BY DESIGN: the rehearsal verdict step is red only because two guards fail on an unmerged branch commit (`main ancestry`, `discipline aggregate and ledgers`), both expected for a non-main commit.
- Hosted proof from the dry-run: 3 builds success; refuse-dispatch-publish skipped; sign-and-publish steps 1-20 success, including step 17 (deb and rpm built for BOTH amd64 and arm64 with per-platform child digests; no "cannot overwrite digest"; dpkg-deb built `maos_1.0.0~rc.1-1_amd64.deb` and `_arm64.deb`; rpm x86_64 and aarch64 written), step 18 (package metadata signed and self-verified), step 19 (rc notes file present), step 20 (every publish file pattern resolves). Step 21 `Create GitHub Release` SKIPPED: nothing published, no release created. rc tree readiness, tag classification, rc notes perf label and rc notes file guards: success.

## 6. Operator ruling 2026-11-14 (path A)

1. Leave `v1.0.0-rc.1` as an unpublished tag. It is never moved or deleted.
2. Fix release.yml (branch `relfix-2026-11-14`, commit `566c6f72`); hosted non-publishing dry-run run 43558157808 completed 2026-11-14 05:05Z; result in section 5.
3. Cut `v1.0.0-rc.2` at a new candidate after a new T3 chain.
4. Order: land everything first, then ONE chain: docs PR #13 (merged, main `40f7b3d8`); this record commit; release.yml fix plus dry-run; B7c-A; the prepared batch (Haiku 5.5, live-leg fixes, layout-agnostic harness and toolchain unpin, B1 GA perf ABSENT, S1, S2); then dispatch the T3 chain at the final tip; then version bump, `docs/release/v1.0.0-rc.2.md`, and tag rc.2.
5. This record does not claim any item in step 4 has landed beyond what section 8 states.

## 7. Corrections to earlier statements

- Attempt 2 of run 43255653207 failed because of the toolchain layout change (1.99.1 to 1.100.0), not because of a single-job rerun.
- 72004 is per link, not cumulative (section 4).
- Local gates missed check-fmt and check-ff-j6 at candidates `23b2200b` and `1acac36e`. check-fmt reached hosted on `23b2200b`; check-ff-j6 reached hosted on `1acac36e`.

## 8. Unverified or not done

- release.yml step 21 `Create GitHub Release`: never run hosted (not for rc.1, not in any rehearsal). Steps 18-20 (package signing and self-verify, rc notes file, publish file pattern resolution) ran hosted in the relfix dry-run only (run 43558157808, section 5).
- The relfix dry-run (run 43558157808) completed 2026-11-14 05:05Z; result in section 5. Still unverified: the real `Create GitHub Release` step, and a fully green rehearsal at the rc.2 candidate (candidate must be the origin/main tip with a T3 chain at that SHA, so the main ancestry and aggregate/ledger guards pass).
- The docker digest collision fix (`566c6f72`): the hosted dry-run shows the failure absent on the same runner image (docker 28.0.4, overlay2). The root-cause mechanism is still an inference, not separately reproduced with the pre-fix workflow.
- rc.1 artifacts: no `.deb`, `.rpm`, `SHA256SUMS-packages` or signatures were produced for publication; no GitHub Release exists.
- The cold-swap admit race (check-multi-tenant-loom): fix not landed; the proper fix (hold the per-Spirit door command lock through boot admission and rollback) is filed as a Low deferred row under an Epic 21 key.
- Test daemon orphan (section 9): cause not reproduced under nextest.
- The prepared batch in section 6 step 4: ordered, not claimed landed.
- `ops-rc-signed-tag`: not done.

## 9. Process incidents (condensed; run ids and SHAs kept)

- Candidate `1acac36e` failure: check-ff-j6 scan read a job id in a provisioning note as a binding (section 2).
- Candidate `80b5b931` failures: the cold-swap race and test-budget overruns (section 2). A fix attempt was reverted after two GPT P1 findings.
- Shared target dir: building the posttag worktree against `CARGO_TARGET_DIR=/mnt/build/cargo-maos` reused unit hashes, so main's binary linked worktree crates (haiku55=0 after the fix). Rule: no cargo in a second worktree against the shared target dir.
- Four agents edited the frozen main checkout through relative paths. All edits reverted, and main was verified clean each time: HaikuSweep (edits moved to posttag via git apply/-R), FixLiveLeg (6 files restored, 3 new files removed), S1ManifestPin (2 files reverted), B7cScriptsDocs (docs/testing/proof-contract.md reversed with git apply -R). Rule: every worktree task uses absolute worktree paths.
- Local file `local://operator-rulings-third-set-2026-11-12.md` was accidentally emptied and restored within the same turn.
- Test hygiene: an orphaned `maos run /tmp/.tmpUzUq8W/package/v1.toml` (pid 839642, parent 1) and its `maos-wasm-runner --component .../lifecycle_probe.wasm` child (pid 839671) ran ~11 h from an earlier local test run (20-9 spawned-lifecycle fixture). The orchestrator stopped both with TERM and removed the scratch directory.
- Local gates do not include the doc-scanning gates that reach hosted. check-air-gap must be excluded locally (it terminated the shell session).

## 10. Model and process disclosures

- GLM quota exhausted (until 2026-11-15 per sprint-status row 373). The GPT reviewer performed the security reviews instead.
- The prepared batch includes a Haiku 5.5 sweep (`claude-haiku-5-5`, released 2026-10-07 per source notes).
