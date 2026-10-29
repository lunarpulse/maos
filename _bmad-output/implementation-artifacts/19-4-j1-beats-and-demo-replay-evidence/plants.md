# 19-4 proven-red plants

## Basis and classification (2026-10-29)

Prepared and exercised in `/home/lunarpulse/wt-w3` against candidate `18082789` plus the existing uncommitted round-4/5 corrections. No command ran in `/home/lunarpulse/dev_ws` and no source changes remain from these plants. The concurrent kloc re-pin and story/epic kloc paragraphs are outside this record's edits.

Pattern: wave-2's `19-2-fr20-enqueue-door-and-safe-point-evidence/plants.md`, its plant diffs, and `ci/ci-w2-verify.yml`; proof contract `docs/testing/proof-contract.md:24-31`. Each diff disables the operative control against the current source, not a masked or dead branch. The AC3 and AC6 control-removal plants were applied **in place**, run individually, copied back from pristine `/tmp` copies, compared byte-for-byte, touched, and their exact witnesses re-run green. AC2 and AC5's real epic-scene plants are **HOSTED ONLY**: they were cut and hashed, but deliberately not applied/run locally. The local environment has `MAOS_T3_RUNTIME` and `MAOS_T3_IMAGE_LOCK_PATH` unset. A local T3 skip is not evidence of the real scene.

All local cargo invocations used this exact prefix, with cwd `/home/lunarpulse/wt-w3`:

```sh
P='env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo-maos-w3 nice -n 10 cargo'
```

`$P` below is notation for that argv prefix, not an extra environment setting. Wall seconds include cargo lock waits, incremental compilation and execution, measured with `time.monotonic()`. Commands ran sequentially within this assignment; cargo's lock serialized other agents. Full local command output and `commands.json` are under `/tmp/w3-plants194-pristine/`; the excerpts and exact commands below are the durable record.

## Plant inventory

| AC | Diff · SHA-256 | Operative plant and premise | Exact single-test/exit command | Expected red: site and oracle | Classification / observed result |
|---|---|---|---|---|---|
| AC2.6, AC1.3, AC4 | `ac2-clear-judgement-twin-plant.diff` · `5903fc4cfe7944d24dccdd1f8336c84b21c33ad49c6e588257bba968e139477b` | Change section 2's committed replay response from `dispatch_ambiguity:0.86` to `0.31` in `founder-loop.json:31`. Prompt hashes/order, strict replay and the kernel threshold remain intact. This is the clear twin's operative judgement, without changing unrelated cassette provenance. | `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_STRICT=1 MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j1/founder-loop.json $P run -p xtask -- demo-j1`; hosted journey: `cargo test --locked --color never -p maos-journey-test --test journey_j1 -- --exact j1_resume_continuity_ref_identity_oracle --test-threads=1` | Demo predicate `xtask/src/demo_j1.rs:963-965`, error return `:395`: `FAIL ambiguity-halt-within-2-frames`, `the kernel raised no halt`, final `executed beat(s) did not hold: …ambiguity-halt-within-2-frames`; decomposition must remain `ok` (three frames). Journey `crates/maos-journey-test/tests/journey_j1.rs:399`: `no halt reached the door (exited early: true)`. | **Hosted only, NOT RUN locally.** Local synthetic negative passed: one test, exit 0, **2.20 s**. It asserted the beat's red; this is not a claimed real-scene red. |
| AC3.3 | `ac3-extra-completion-reference-plant.diff` · `75e5ecbf58eff67990c24b21ae485ef7aba013596f6f6595964e15f0cc8c0add` | Remove `&& uncited.is_empty()` from `digest_cites_source_refs` at `xtask/src/demo_j1.rs:1066`. The existing synthetic vector adds one nonexistent completion ref to an otherwise valid digest with all three real completions. Count and single-digest checks still pass; this removes the actual citation gate, not its input-count checks. | `$P test --locked --color never -p xtask --bin xtask -- --exact demo_j1::tests::a_digest_citing_an_extra_completion_fails_the_digest_beat --test-threads=1` | `xtask/src/tests/demo_j1_tests.rs:629`: `1 morning_digest(s), 4 completion ref(s) (>= 3); refs that name no matching audit row of their kind among 4 row(s): ["0119000000000000…"]`. | **Local, observed RED**, exit **101**, **5.42 s**. `0 passed; 1 failed`. Restore verified; exact witness green after restoration, exit 0, **17.26 s**. |
| AC5.4 | `ac5-resolve-step-removed-plant.diff` · `0c2e7c714d64c11624635a06b5baf5a60a66354783453f1add0e7dbcbdfe09fa` | Remove the actual `dispatch_halted` → pending-id loop → `resolve_halt(bins, home, halt_id)` call in `xtask/src/demo_j1.rs:682-690`. Streaming/events, the actual kernel halt and the production **120-second** hold remain. No shorter bound, fake observation, resolver stub, or synthetic-only substitution is planted. The unused resolver helper may warn during the plant. | `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_STRICT=1 MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j1/founder-loop.json $P run -p xtask -- demo-j1` | Demo predicate `xtask/src/demo_j1.rs:1006` (unplanted numbering), error return `:395`: `FAIL halt-resume-referential-identity`, `resumed false`, `cited as resolved false`, `still listed open true`; halt beat remains `ok` with `frames before dispatch_halted Some(1)`, `dispatch_halted` appears and `dispatch_resumed` does not. Other three-section/count beats can fail too because only section 1 ran. | **Hosted only, NOT RUN locally.** Local recorded-stream negative: exit 0, **0.27 s**. Local production deadline/cancellation witness: exit 0, **2.29 s**. Neither is represented as the real mutated demo run. |
| AC6.3 | `ac6-outside-crate-unenrolled-plant.diff` · `470a16cab4fb3b21816630fc8c22c51761e360b41f114b04499881b02ba88385` | Restrict `derive_enrolled_targets` to `maos-bin`, immediately after the actual crate name is read (`xtask/src/check_j1_loopback_delegation.rs:867`). The existing in-tree vector creates un-enrolled `crates/maos-iac/tests/xyz_2b.rs` in its isolated fixture tree. All maos-bin baseline enrolments remain, so the gate becomes falsely green solely because it cannot see the outside-crate file. | `$P test --locked --color never -p xtask --test j1_crosshost_1b_proven_red -- --exact planting_an_unenrolled_2b_file_outside_maos_bin_must_red --test-threads=1` | `xtask/tests/j1_crosshost_1b_proven_red.rs:242`: `PROVEN-RED FAILURE — the gate stayed GREEN with ` + `a J1 test file outside crates/maos-bin/tests was added without enrolling it`. | **Local, observed RED**, exit **101**, **10.93 s**. `0 passed; 1 failed`. Restore verified; exact witness green after restoration, exit 0, **31.22 s**. |

The table's local red/green times are the initial in-place edit/cut runs. Both plants were then executed again through the **literal saved `.diff` / `git apply` protocol** below: AC3 RED **31.35 s**, exit **101**, same `:629` oracle, restored GREEN **30.46 s**, exit **0**; AC6 RED **21.34 s**, exit **101**, same `:242` oracle, restored GREEN **14.64 s**, exit **0**. Both final red runs again selected exactly one test and reported `0 passed; 1 failed`; all copy-back/`cmp`/`touch` commands exited 0.

The hosted cargo commands do **not** unset `CI`/`GITHUB_ACTIONS`: the provisioned runner must fail closed instead of silently skipping. The demo exit is a command returning `Err`, not a libtest test; `--exact`, a libtest summary, and a panic site are inapplicable to that command. Its matcher therefore requires the named claim-table `FAIL` row, the final named executed-beat error and the control-specific oracle. The journey plant uses the full wave-2 libtest matcher, including the exact failure header and panic site.

## Local controls and existing negative vectors

Each row below ran exactly one test with `--exact`; all exited **0** and reported `test result: ok. 1 passed; 0 failed; 0 ignored;`. A green negative vector means its assertions observed the beat/gate go red; it is not a panic or an end-to-end hosted observation.

| Purpose | Exact command | Wall seconds | Assertions exercised |
|---|---|---:|---|
| AC2 clear-judgement synthetic negative | `$P test --locked --color never -p xtask --bin xtask -- --exact demo_j1::tests::a_clear_judgement_stream_fails_the_ambiguity_halt_beat --test-threads=1 --show-output` | 2.20 | `demo_j1_tests.rs:510-511`: beat failed; detail contains `the kernel raised no halt`. |
| AC3 extra-completion synthetic negative, before planting | `$P test --locked --color never -p xtask --bin xtask -- --exact demo_j1::tests::a_digest_citing_an_extra_completion_fails_the_digest_beat --test-threads=1 --show-output` | 0.30 | `:629-635`: beat failed; finding names the nonexistent ref and `name no matching audit row of their kind`. |
| AC5 recorded unresolved-hold synthetic negative | `$P test --locked --color never -p xtask --bin xtask -- --exact demo_j1::tests::the_resolve_step_removed_fails_the_resume_beat --test-threads=1 --show-output` | 0.27 | `:560-566`: halt beat holds, resume fails with `resumed false`, one frame is not the completed three-section epic. |
| Positive synthetic scene control | `$P test --locked --color never -p xtask --bin xtask -- --exact demo_j1::tests::the_full_epic_stream_proves_every_executed_beat --test-threads=1 --show-output` | 0.27 | `:437,460`: every executed scene beat and the digest citation beat hold against the unmodified stream. |
| AC6 positive fixture-tree control | `$P test --locked --color never -p xtask --test j1_crosshost_1b_proven_red -- --exact baseline_fixture_tree_is_green --test-threads=1 --show-output` | 0.29 | `j1_crosshost_1b_proven_red.rs:278`: unmodified fixture gate passes and exits 0, preventing a vacuous negative. |
| AC6 in-tree outside-crate mutation control, before planting | `$P test --locked --color never -p xtask --test j1_crosshost_1b_proven_red -- --exact planting_an_unenrolled_2b_file_outside_maos_bin_must_red --test-threads=1 --show-output` | 0.31 | `:711-720` / `assert_red :242-257`: gate red/nonzero, missing command is `cargo test -p maos-iac --test xyz_2b`, finding belongs to `completion-vectors-enrolled`. |
| AC5 production hold deadline/cancellation, no T3 required | `$P test --locked --color never -p maos-bin --test epic_halt_resume_19_4 -- --exact races::cancellation_at_entry_and_deadline_before_empty_pending_stop --test-threads=1 --show-output` | 2.29 | `epic_halt_resume_19_4.rs:1103,1110-1114`: real kernel/resolver/memory + production `hold_for_resolution`; cancellation stops at entry; after advancing past its one-second deadline, even a resolved/empty pending set cannot resume (`hold.await.is_none()`). |

The subprocess production vector `an_unresolved_hold_expires_into_the_19_1_stop` requires a real T3 Worker before the section-2 hold. It was deliberately **not invoked locally** to manufacture a green skip; it is mandatory in the hosted unplanted target control. The local production witness above exercises the real hold seam without subprocess networking/Workers and does not claim to replace that vector.

## Observed local red output

### AC3 — citation check removed

```text
running 1 test
test demo_j1::tests::a_digest_citing_an_extra_completion_fails_the_digest_beat ... FAILED
---- demo_j1::tests::a_digest_citing_an_extra_completion_fails_the_digest_beat stdout ----
panicked at xtask/src/tests/demo_j1_tests.rs:629:5:
1 morning_digest(s), 4 completion ref(s) (>= 3); refs that name no matching audit row of their kind among 4 row(s): ["0119000000000000…"]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 535 filtered out; finished in 0.00s
```

### AC6 — outside-crate traversal disabled

```text
running 1 test
test planting_an_unenrolled_2b_file_outside_maos_bin_must_red ... FAILED
---- planting_an_unenrolled_2b_file_outside_maos_bin_must_red stdout ----
panicked at xtask/tests/j1_crosshost_1b_proven_red.rs:242:5:
PROVEN-RED FAILURE — the gate stayed GREEN with `a J1 test file outside crates/maos-bin/tests was added without enrolling it` planted. A gate that cannot see this regression is an empty box.
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 41 filtered out; finished in 0.01s
```

The AC6 red also printed the real gate JSON with `"passed":true,"oracle_green":true,"binding":"Blocking"` and `completion-vectors-enrolled` in the leg audit: narrowing the walk actually hid the planted outside-crate file, rather than causing an unrelated read failure.

## Byte-identical restoration proof

The initial in-place source edits were cut into the recorded diffs and restored. Then both saved diffs were literally applied with `git apply`, in place, one at a time, and each exact red witness was run again before restoration and a green re-run. Before each application the source was copied with `cp --preserve=mode`. The following exact copy/apply/restore commands all exited **0**, in this order (the intervening test commands and their final timings are recorded above):

```sh
cp --preserve=mode /home/lunarpulse/wt-w3/xtask/src/demo_j1.rs /tmp/w3-plants194-pristine/demo_j1.rs
git apply /home/lunarpulse/wt-w3/_bmad-output/implementation-artifacts/19-4-j1-beats-and-demo-replay-evidence/ac3-extra-completion-reference-plant.diff
# Exact AC3 single-test run: exit 101, 31.35 s.
cp /tmp/w3-plants194-pristine/demo_j1.rs /home/lunarpulse/wt-w3/xtask/src/demo_j1.rs
cmp /tmp/w3-plants194-pristine/demo_j1.rs /home/lunarpulse/wt-w3/xtask/src/demo_j1.rs
touch /home/lunarpulse/wt-w3/xtask/src/demo_j1.rs
# Same exact AC3 test after restore: exit 0, 30.46 s.

cp --preserve=mode /home/lunarpulse/wt-w3/xtask/src/check_j1_loopback_delegation.rs /tmp/w3-plants194-pristine/check_j1_loopback_delegation.rs
git apply /home/lunarpulse/wt-w3/_bmad-output/implementation-artifacts/19-4-j1-beats-and-demo-replay-evidence/ac6-outside-crate-unenrolled-plant.diff
# Exact AC6 single-test run: exit 101, 21.34 s.
cp /tmp/w3-plants194-pristine/check_j1_loopback_delegation.rs /home/lunarpulse/wt-w3/xtask/src/check_j1_loopback_delegation.rs
cmp /tmp/w3-plants194-pristine/check_j1_loopback_delegation.rs /home/lunarpulse/wt-w3/xtask/src/check_j1_loopback_delegation.rs
touch /home/lunarpulse/wt-w3/xtask/src/check_j1_loopback_delegation.rs
# Same exact AC6 test after restore: exit 0, 14.64 s.
```

Pristine and restored SHA-256 values matched:

- `demo_j1.rs`: `9f03ba8e21bb7ee7eb0080b999c65e2a2eec5c4a71673eb4dd6147de6fc2ee3a`.
- `check_j1_loopback_delegation.rs`: `abd6099bbf8c0b47a40e3d12e35f5a15a64b702754d68443d6aaa9be01ee7f4b`.

After restoring both sources, `git apply --check /home/lunarpulse/wt-w3/_bmad-output/implementation-artifacts/19-4-j1-beats-and-demo-replay-evidence/<diff>` exited **0** separately for **all four** inventory diffs. AC2/AC5 were applicability-checked only, never applied or executed locally. No unrelated working-tree changes were reset.

## Hosted T3 run — prepared, not yet executed

Workflow: `ci/ci-w3-verify.yml`. Copy only that file into `.github/workflows/` on branch **`ci/w3-verify`**; its only trigger is push to that branch. No workflow is installed in the main workflow directory by this evidence change. Provisioning is the identical `./.github/actions/provision-t3-test-substrate` action used by wave 2 on `ubuntu-24.04`; fixtures are built through `build-test-fixtures --only worker-cli-fixture --only maosctl`.

The hosted order is:

1. Provenance and diff SHA-256 recording.
2. Whole unplanted `epic_halt_resume_19_4` control, all **17 named vectors**, serial with `--show-output`, exit 0, 0 failed, **0 ignored**, no `SKIP:`. This includes the real unresolved-hold-expiry subprocess vector; every T3 vector must run.
3. Unplanted verbatim `demo-j1` exit command with strict replay; named epic/decomposition/ambiguity/resume/digest beats all `ok PROVEN_BLOCKING`, no executed FAIL or SKIP. Honest nonclaims remain visible.
4. Whole unplanted `journey_j1` control, all four named vectors including the resume oracle and the clear twin; 0 failed/ignored and no SKIP. The clear twin proves three completed frames, no halt or resume, and no halt digest citations.
5. AC2 cassette plant, SHA check → pristine copy under `/tmp` → apply → actual demo must fail the named ambiguity beat while decomposition stays green → exact positive journey oracle must panic at `journey_j1.rs:399` with the no-halt/early-exit oracle → copy back → `cmp` → `touch` → clean diff. The libtest matcher requires `running 1 test`, `test result: FAILED. 0 passed; 1 failed;`, `---- <name> stdout ----`, the panic site, and oracle text; it never relies on the potentially split `test … FAILED` progress line.
6. The exit-job twin command with **`founder-loop.clear-twin.json` itself**, nonzero with named ambiguity FAIL and three-frame decomposition green. This is separate from the operative cassette diff so the literal exit-job twin is exercised too.
7. AC5 actual resolve-call removal, SHA check → pristine copy under `/tmp` → apply → actual demo hold expires with halt `ok`, resume FAIL and its open-halt/no-resume oracles → byte restore and clean diff. The original 120-second production bound is retained.
8. Upload logs, per-command exit/wall-ms timing sidecars, diffs, this record and workflow as artifact **`w3-proven-red`**, even on failure.

Hosted plants run only after the unplanted controls pass. An EXIT trap restores from the pristine `/tmp` copy even if an oracle or command fails; every changed file is copied back, `cmp`-checked and touched. Restored/pristine SHA values and clean `git diff --exit-code` are logged. The workflow does not replace the user's uncommitted fixes with a checkout.

**Hosted run id: NOT RUN / pending operator branch push.** Hosted wall times, real AC2 demo/journey reds, actual AC5 mutated-demo red, and hosted restore proofs are intentionally unclaimed until that run. They are not runnable locally because the signed T3 substrate is absent. No commit, push, hosted dispatch, broad test suite, formatter or kloc gate was run by this assignment.
