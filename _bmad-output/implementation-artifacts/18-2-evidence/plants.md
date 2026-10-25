# 18-2 proven-red plants

Dev pass 2026-10-25, `anthropic/claude-opus-5-5`. One cargo process, `CARGO_TARGET_DIR=/mnt/build/cargo`, local runs under
`env -u CI -u GITHUB_ACTIONS`, after `cargo fmt --all`. A scripted runner made each plant in place: copy the file aside,
plant, focused `cargo nextest run --offline --no-fail-fast`, copy back, `cmp` (`CMP_OK` every time), `touch`, then a green
re-run. The `touch` is needed because the copy-back restores the old mtime, and cargo would otherwise keep the planted
artifact. Diffs: `<row>-plant.diff` in this directory. Wall-clock is plant → red observed (including the rebuild), then
restore → green observed.

Six plants: two for AC1 (named in AC1.6 under `proof-contract.md:26`'s exception for two independent controls), one each
for AC2, AC4 and AC5, and one local plant for AC3's hermetic record witness. AC3.4 rules out a CI plant for AC3, and this
local plant doesn't change that.

| AC | File · change | Red test → assertion | Restore | Wall-clock |
|---|---|---|---|---|
| AC1 (a) model consumption | `crates/maos-bench/src/halt_eval.rs` — `.with_deferred_inference_port(port, …)` removed from the Butler chain (`let _ = (port, binding);`), so the pass takes the arithmetic path | `eval_halt_18_2`: `exit_line_2_replays_the_seed_cassette_and_holds_every_floor` FAIL at `:116` (`maos eval: 0 of 30 completions consumed — not a model-dependent number`, exit 1). Drift-blind, precision-planted, 29-entry, drifted-hash and in-process record cases also red, each on the same refusal. Result: 9 tests run: 2 passed, 7 failed | copy-back + `cmp` + `touch`; 9/9 green | red 16.6 s, green 5.0 s |
| AC1 (b) overall-recall floor (= `ci/plant.diff`) | `crates/maos-eval/src/halt_replay.rs` `missed_floors` — the `halt_recall_overall` entry deleted | `the_drift_blind_negative_control_misses_overall_recall` FAIL at `eval_halt_18_2.rs:144` (`left: Some(0)` / `right: Some(1)`): the drift-blind cassette exits 0. Result: 9 tests run: 8 passed, 1 failed | copy-back + `cmp` + `touch`; 9/9 green | red 7.1 s, green 7.2 s |
| AC2 prompt pinning | `crates/maos-journey-test/cassettes/j-butler/halt-corpus-v0.3.json` — `entries[0].prompt_sha256` set to the all-zero wildcard | `halt_eval_corpus_18_2`: `no_entry_carries_the_wildcard_hash` FAIL at `:111` ("seq 0: the wildcard skips the strict drift check"). `every_entry_hash_is_the_rows_proposal_prompt` (`:102`) and `the_negative_controls_differ_from_the_seed_only_in_their_planted_rows` (`:144`) also red. Result: 6 tests run: 3 passed, 3 failed, 1 skipped | copy-back + `cmp` + `touch`; 6/6 green | red 5.2 s, green 0.8 s |
| AC3 record flush (local only) | `crates/maos-bench/src/halt_eval.rs` — `flush()?;` → `let _ = flush;` (the 15-6 recorder never writes) | `record_through_the_router_writes_a_cassette_that_replays_green` FAIL at `eval_halt_18_2.rs:349` (`cassette "/tmp/…/recorded.json": No such file or directory`). Result: 9 tests run: 8 passed, 1 failed | copy-back + `cmp` + `touch`; 9/9 green | red 4.1 s, green 4.3 s |
| AC4 FR15 journaling (departure 4: director-surface, not the kernel) | `crates/maos-director-surface/src/halt_ui.rs` `HaltFlow::submit_resolution` — the `self.journal.journal_halt_resolution(…)?` write removed | `fr15_a_butler_halt_resolves_once_per_kind_journaled` and `fr15_a_researcher_halt_resolves_once_per_kind_journaled` both FAIL at `tests/harness/fr15_resolution.rs:138` ("provided_context: one journaled resolution", `left: 0` / `right: 1`). Results: butler 6 tests run: 5 passed, 1 failed, 1 skipped; researcher 4 tests run: 3 passed, 1 failed | copy-back + `cmp` + `touch`; 6/6 and 4/4 green | red 7.5 s (2.3 + 5.3), green 5.3 s |
| AC5 sidecar signature | `crates/maos-cli/src/subcommands.rs` `spirit_benchmark` — `sidecar::verify(&bytes, &pubkey)` replaced by an unverified parse of the body | `benchmark_sidecar_18_2`: `a_tampered_body_exits_one_and_prints_nothing` FAIL at `:105` (exit 0, stderr "signature verified"). `a_producer_signature_verifies_only_against_the_producer_pubkey` also red (`:128`). Result: 6 tests run: 4 passed, 2 failed | copy-back + `cmp` + `touch`; 6/6 green | red 3.7 s, green 1.8 s |

Kernel: no plant touched `crates/maos-kernel-core`. AC4.6 named a temporary kernel edit in `HaltRegistry::resolve`, but
that function writes no journal. The write is `HaltFlow::submit_resolution` (`crates/maos-director-surface/src/halt_ui.rs:91-92`),
which is what the FR15 witnesses drive (story departure 4).

## CI plant

`epic-18-exit`'s exit-line-2 twin is the standing red for exit line 2 on every run. It runs
`maos eval halt --class butler --cassette spirits/butler/tests/fixtures/halt-corpus-v0.3.drift-blind.json` and fails the
step when that run exits 0 or doesn't name `halt_recall_overall 0.800 below floor 0.90`. `ci/plant.diff` is the AC1 (b)
plant, sha256 `f3255f459efd043860f825546056f0ea374941d7852afa89d3dd80c541deedb2` (`ci/plant.diff.sha256`).

Simulated locally from the workflow text: each step's `run:` block was extracted from `discipline.yml` and run under `bash`
from the workspace root, with `PATH` pointing at `/mnt/build/cargo/debug`.
- **Planted** (`maos` rebuilt, 19.1 s): the twin step exited 1 with
  `::error title=Epic-18 exit line 2 twin::the drift-blind cassette exited 0 — the overall-recall floor is not a control`.
- **Restored** (copy-back + `cmp` + `touch`, rebuilt 8.1 s): exit line 2 exited 0 with
  `::notice title=Epic-18 exit line 2::rc=0`, and the sidecar was copied. The twin step exited 0 with
  `::notice title=Epic-18 exit line 2 twin::rc=1, halt_recall_overall 0.800 below floor 0.90`.

The integrator owns the remote CI proven-red run on a branch with `ci/plant.diff` applied.

## Review plants (2026-10-25, review patch pass)

Same discipline: in place, copy-back + `cmp` (`CMP_OK` every time) + `touch`, then a green re-run; one cargo process,
`CARGO_TARGET_DIR=/mnt/build/cargo`, `env -u CI -u GITHUB_ACTIONS`, after `cargo fmt --all`. maosctl plants ran under
`cargo test --offline -p maos-cli --test benchmark_sidecar_18_2` ALONE (maos-cli's graph has no serde_json
`float_roundtrip`, which is what makes BH-1 observable); maos-bin plants under
`cargo nextest run --offline --no-fail-fast -p maos-bin --test eval_halt_18_2`. Diffs: `review-<id>-plant.diff`.

| Finding | File · change | Red test → result | Restore | Wall-clock |
|---|---|---|---|---|
| BH-1/EC-1 exact-bytes signature | `crates/maos-eval/src/halt_replay.rs` `sidecar::verify` — verify over `serde_json::to_vec(parse(body))` (the pre-review re-encoding) instead of the stored bytes | `a_float_that_does_not_round_trip_verifies_byte_exact` FAIL (exit 1: 14/15 = `0.9333333333333333` re-encodes 1 ulp low, signature fails). 10 run: 9 passed, 1 failed | copy-back + `cmp` + `touch`; 10/10 | red 2.1 s, green 1.6 s |
| BH-2/EC-2/AA-2 signer bound to key | `halt_replay.rs` `verify` — the `("signer", …, signer_for(pubkey))` claim removed | `a_dev_seed_signature_claiming_the_producer_exits_one` FAIL (exit 0). 9 passed, 1 failed | copy-back + `cmp` + `touch`; 10/10 | red 2.3 s, green 2.5 s |
| BH-3/EC-3/AA-3 class / version / schema | `halt_replay.rs` `verify` — the `schema`, `class`, `spirit_version` claims removed | `a_sidecar_of_another_class_version_or_schema_exits_one` FAIL (the researcher-class body exits 0). 9 passed, 1 failed | copy-back + `cmp` + `touch`; 10/10 | red 1.8 s, green 1.8 s |
| BH-4 non-UTF-8 key | `halt_replay.rs` `key_from_env` — any `VarError` → `Ok(None)` (the pre-review fallback) | `a_non_utf8_pubkey_refuses_and_never_falls_back` FAIL (falls back to the dev pubkey, exit 0). 9 passed, 1 failed | copy-back + `cmp` + `touch`; 10/10 | red 1.8 s, green 1.8 s |
| R2 (D-18-2-R2) override publishes no sidecar | `crates/maos-bench/src/halt_eval.rs` `EvalRequest::publishes_sidecar` → always `true` | `an_env_cassette_is_replayed_and_publishes_no_sidecar` and `the_drift_blind_negative_control_misses_overall_recall` FAIL (no "override cassette" stderr; sidecar written). 12 run: 10 passed, 2 failed | copy-back + `cmp` + `touch`; 12/12 | red 6.1 s, green 4.1 s |
| BH-5 replay honours `MAOS_REPLAY_CASSETTE` | `crates/maos-bin/src/main.rs` `Eval` arm — `env_cassette` = `None` | `an_env_cassette_is_replayed_and_publishes_no_sidecar` FAIL (scores the committed seed, exit 0). 11 passed, 1 failed | copy-back + `cmp` + `touch`; 12/12 | red 8.1 s, green 8.3 s |
| EC-4 duplicate `scenario_id` | `halt_replay.rs` `score_class` — `if false && rows != distinct` | `a_duplicate_scenario_id_is_refused` FAIL (scores, no `Err`). 11 passed, 1 failed | copy-back + `cmp` + `touch`; 12/12 | red 5.4 s, green 5.5 s |
| AC5 (dev row above, **regenerated**) | `crates/maos-cli/src/subcommands.rs` `spirit_benchmark` — `sidecar::verify(…)` replaced by an unverified `SignedBenchmark` parse. The dev `ac5-plant.diff` no longer applied after the review changed `verify`'s signature; `ac5-plant.diff` is overwritten with this one | `a_tampered_body_exits_one_and_prints_nothing`, `a_producer_signature_verifies_only_against_the_producer_pubkey`, `a_dev_seed_signature_claiming_the_producer_exits_one`, `a_sidecar_of_another_class_version_or_schema_exits_one` FAIL. 10 run: 6 passed, 4 failed | copy-back + `cmp` + `touch`; 10/10 | red 1.8 s, green 1.4 s |

AA-1 (basis label) is a published field, asserted in the exit-line-2 witness; TIA-3/TIA-4 are isolation fixes. Neither is
a control, so neither has a plant. After the review patches every diff in this directory applies (`git apply --check`,
14/14), and `ci/plant.diff` still applies unchanged: sha256 `f3255f459efd043860f825546056f0ea374941d7852afa89d3dd80c541deedb2`
(re-hashed after the patches). The remote CI proven-red run stays the integrator's (TIA-5).
