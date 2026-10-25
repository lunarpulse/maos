---
story_id: '18.2'
story_key: 18-2-eval-halt-verb-and-corpus-numbers
created: 2026-10-25
context_commit: 140b906b
baseline_commit: 140b906b
security_sensitive: true
kernel_grant: "none — kernel-Δ **0**, measured at creation (E16-A7) by two throwaway prototypes that compiled and ran against the public kernel API only: `maos-kernel-core` tokei 20608 → 20608 and physical pin 26377 / 95 files unchanged (`kloc-check --json`, `check-kernel-baseline` PASSED after the revert). No FLAG-Winston row. A third prototype (driver in `maos-eval` with a kernel dependency) was refused by cargo: `maos-kernel-core` → `maos-eval` is a production edge (`crates/maos-kernel-core/Cargo.toml:62`, used by `src/isolation/runner.rs:12`). If the kernel figure moves at landing, the story stops and returns a FLAG-Winston D-row."
kloc_grant: "RULED 2026-10-25 (operator, Lunarpulse): D-18-2-01 B — composition in maos-bench, thin verb in maos-bin, kernel-free scorer in maos-eval. **maos-bin pre-ratified ≤ 28140 for 18-2**, and Epic 18's named maos-bin budget raised +250 → +385 (27809 → 28194). D-18-2-07 is the standing measured-at-landing contract. Measured at creation, after `cargo fmt --all`, `kloc-check --json` (tokei 14.0.0) at 140b906b: maos-bin 28005 / 28005; maos-eval 4041 / 4041 at zero headroom (15-2 ledger 'Epic 18 halt-eval driver and judge harness +450–900', `xtask/kloc.toml:672-673`, so a measured request at landing); maos-bench 1512 / 1791; maos-registry 3392 / 4282; maos-cli 6057 / 6856; maos-kernel-core 20608; aggregate 182291 (`_aggregate_hardfail` 182292). Prototype B (ruled): maos-bin **+50**, maos-bench **+134**, maos-eval **+50**, measured. Forecast [INFERENCE] beyond the measured part: maos-bin record mode +40–80 and sidecar call (total ≤ 28140); maos-bench Researcher class +60–90 (fits); maos-eval overall recall + sidecar body + signing +30–90; maos-registry +40–70; maos-cli +40–80. Exceeding any ceiling still needs a measured request. Test files are uncharged. In-`src` `#[cfg(test)]` code is charged. `spirits/*` is ungoverned (`xtask/src/kloc_check.rs:310-311`)."
review: "bmad-code-review under `docs/testing/proof-contract.md` (focused suites + enrolled gates; workspace once in CI). Dev and reviewer model `anthropic/claude-opus-5-5` ONLY: operator ruling 2026-10-25, no fable. AC5 (the sidecar signature and its verification) is an integrity control; the review record names a Security layer for it, run on opus-5-5. No kernel diff, so no FLAG-Winston sign-off unless landing measures one."
---

# Story 18.2: `maos eval halt` and model-dependent per-class numbers

Status: done

<!-- Created 2026-10-25 at 140b906b by bmad-create-story under the lighter creation contract (no party-mode preflight); 18-1 and 18-8 were treated as landed. The host's only cargo process was used for measurement (CARGO_TARGET_DIR=/mnt/build/cargo, `env -u CI -u GITHUB_ACTIONS`). Three throwaway prototypes were built: two compiled and ran, one was refused by cargo. All were measured and reverted by copy-back, and the HEAD `maos` binary was rebuilt. RULED 2026-10-25 by the operator (Lunarpulse): D-18-2-01 B, 02 B, 03 A, 04 A, 05 A, 06 A, plus the maos-bin pre-ratification ≤ 28140 → `ready-for-dev`. -->

> **The capability:** *`maos eval halt --class butler` replays every row of the Butler halt corpus through Butler's inference seam and the real kernel halt path, using one recorded completion per row. It prints and publishes recall/precision as a signed sidecar, and exits non-zero when the floors are missed, when the run never consumed the model, or when a cassette drifts.*

**Closes:** NFR-Test-4 for the **Butler** class (model-driven) and the **Researcher** class (labelled not-model-driven) (`prd/non-functional-requirements.md:81`; coverage row `tests/coverage-matrix.yaml:1896-1898`, `gates: [] corpora: []` today). Exit line 2 (epic `:15`; red at HEAD, and `check-exit-commands` lists `eval` as owed to this key). FR15's per-class floors and resolution pathways for Butler and the Researcher (`prd/functional-requirements.md:49`). The routing-audit widening `routing-audit-2026-10-22.md:36` as ruled by D-18-2-05 A, with the Orchestrator received by `19-1` AC5. `deferred-work.md` has **no row owned by this key** (grep of `18-2`, `eval halt`, `maos eval`, `NFR-Test-4`, `halt_recall` at 140b906b).

**Does NOT close, and says so:**
- **Live model evidence in CI.** CI replays the committed cassette. While its provenance is `seed` (D-18-2-03), the published number is a seed's, and the sidecar says so. Live re-recording is the operator lane (`ops-nightly-cassette-rerecord`, `sprint-status.yaml:332`).
- **Recall over the calendar-conflict subset is not model-dependent** (D-18-2-02, measured). Butler's conflict halts come from the arithmetic `belief_variance` that 18-1 kept, and the story publishes this as a labelled fact rather than claiming otherwise.
- **Model-driven Researcher numbers, and the Orchestrator class.** The Researcher's halt scalars are claim-metadata arithmetic with no production writer (D-18-2-05 A), so its class number is published as `model_driven: false`. The Orchestrator class has no scalar producer at HEAD and is received by `19-1-orchestrator-dispatch-loop-and-epic-input` AC5. That AC does not depend on 18-2 (D-E).
- 18-1's capability authorization of provider dispatch (AC7 there), Mira/Nash classes (`21-1`, which also claims NFR-Test-4), the judge harness (18-3).

## Story

As **the director deciding whether to trust Butler's halts**,
I want **one command that replays Butler's halt corpus through the model path it really runs, scores it against the class floors, refuses a run that never consulted the model, and publishes the numbers where `maosctl` can read and verify them**,
so that **NFR-Test-4's per-class halt numbers are measured from completions, reproducible in CI without a key, and cannot be quietly replaced by the arithmetic number CI already holds.**

**Contract:** epic-18 §18-2 (`_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md:138-149`, corrected in place at creation, rulings `:144`), §R R18-11, R18-12, R18-22 (`:60-61`, `:71`), exit line 2 and its provenance (`:15`, `:23`), Kernel-Δ (`:32`), kloc asks (`:36`), confidence rules 9–11 (`:38-40`), Dependencies (`:224-226`); the received Orchestrator item `epic-19-founder-loop-w4.md:63` (§19-1 AC5); `docs/testing/proof-contract.md` (binding).

## Current-tree corrections — read before implementing

Measured at `140b906b` by direct reads of every cited range, a corpus profile (`jq` over the 30 rows), `check-exit-commands --json`, `kloc-check --json`, `check-kernel-baseline`, and the prototypes in §Kernel and kloc pricing.

| Premise (epic / §R) | Fact at `140b906b` and consequence |
|---|---|
| Typed verb table `crates/maos-bin/src/verbs.rs:59-138`; dispatcher `main.rs:1659`; unknown arm `:1670-1674` (AC1, R18-11, exit provenance `:23`) | `enum VerbName` is `verbs.rs:59-68` and `VERBS` is `:88-137`. There are **two** dispatchers: air-gap `main()` `verbs::dispatch` at `main.rs:1447` (unknown arm `:1460-1465`) and network `main()` at `:1790` (unknown arm `:1801-1806`). Observed at HEAD: `maos eval halt --class butler` → `maos: unknown command 'eval' in network mode — see 'maos --help'`, exit 1. `check-exit-commands --json`: `passed: true`, `owed: [{token: "eval", owner_story: "18-2-eval-halt-verb-and-corpus-numbers"}]`. Corrected in place. |
| `ButlerOrchestratorAdapter` `main.rs:556`, constructed `:701` (AC1, R18-11) | Struct `main.rs:557` (`#[cfg(feature = "network")]` `:556`), `EpistemicScalarPort` impl `:579-643`, built in `construct_butler_core` `:666-740` at `:724`, and also at `:4841` and `:5807`. It is `main.rs`-private and network-only, so a lib or another crate cannot reuse it. Corrected in place. |
| `corpus_halt.rs:73-139` builds the world; floors `:296-305`; corpus lacks model-I/O fields `:129-151` | World `make_world` `spirits/butler/tests/corpus_halt.rs:73-102`; `CorpusRow` `:129-139`; `load_rows` `:141-152`; size lock `:160`; floor asserts `:297-306` (`>= 0.90` `:298`, `>= 0.85` `:303`). Corrected in place (R18-12 too). |
| **AC1/exit line 2: "the number CI already asserts … becomes model-dependent"** | **Only precision is model-dependent.** `Assessment::primary_scalar` (`spirits/butler/src/lib.rs:488-514`) writes `belief_variance` whenever a conflict exists, and that value stays arithmetic (`assess` `:1111-1154`; 18-1 AC1.1). The model's `user_preference_drift` (`:852`) decides the halt only on rows with no unresolved conflict. Corpus profile: 12 `cc*` rows have real conflicts, which makes the recall denominator (`calendar_conflict && expected_halt`, `crates/maos-eval/src/onboarding_gate_corpus.rs:476-500`) all arithmetic. 6 `pb*` rows (tentative/cancelled) and 9 `tn*` rows take the drift scalar (false positives possible), and 3 `df*` rows take drift (true positives). **Measured on the prototype:** planting drift 0.1 on all 12 `cc*` completions leaves recall 1.0. A drift-blind cassette (`df*` drift 0.99) passes **both** floors (recall 1.0, precision 1.0). → D-18-2-02. |
| AC1: "a run in which no completion was consumed … exits non-zero" | Achievable outside the kernel: a counting `InferencePort` wrapper around the replay port, with refusal when consumed ≠ rows. The prototype plant that unbinds the port prints `0 of 30 completions consumed — not a model-dependent number`, exit 1, 272 ms. A refused pass (`Butler::take_pass_error`, `lib.rs:982`) is a hard error, never "no halt". |
| AC1: "hosted in `maos-bin` … the 18-2 driver in `maos-eval`" (kloc asks `:36`) | **`maos-eval` cannot depend on `maos-kernel-core`**: the kernel depends on `maos-eval` in production (`crates/maos-kernel-core/Cargo.toml:62`, `src/isolation/runner.rs:12`). The prototype got `error: cyclic package dependency`, and `maos-eval/src/lib.rs:11-15` states the rule. Only a kernel-free scorer can live in `maos-eval`. The Butler + kernel composition must sit above the kernel → D-18-2-01. |
| AC1: sidecar "in `maos-registry` storage (`crates/maos-registry/src/storage.rs`) … readable by `maosctl` through a read verb named in the story"; exit comment "the sidecar the registry serves" | Storage is per-version dirs under `~/.local/share/maos/registry/spirits/<id>/<ver>/` (`storage.rs:7-20`, `:88-105`); the `RegistryStorage` trait (`:34-79`) has no sidecar method; `grep -rn halt_recall crates/maos-registry/src` = 0. No persistent host signing key exists in `maos-bin` (the daemon draws a fresh seed per boot, `main.rs:2076-2083`, `FIXME(1b.3)`). The existing producer-key precedent is Story 11.7's split: an env seed/pubkey pair with a documented dev seed (`crates/maos-eval/src/trial_attestation.rs:33-87`) over `maos_audit::release_verify::{sign_sha256sums, verify_release_signature}` (`crates/maos-audit/src/release_verify.rs:154,174`). `maos-cli` already depends on `maos-registry` and `maos-audit` (`crates/maos-cli/Cargo.toml:21,23`). → D-18-2-04. |
| AC2: corpus pin `discipline.yml:949-952`; "re-pinned in the same commit" | The pin is `spirits/butler/tests/corpus_pin.rs:21-31` (`PINNED` `:23` = `89c07335…f3ad`), run by job `butler-tests` (`.github/workflows/discipline.yml:974-991`, `cargo test -p butler --locked` `:991`). The same SHA is in `tests/coverage-matrix.yaml:1218` (NFR-Onb-1 notes). Row deserializers tolerate new fields (`OnbScenario` `onboarding_gate_corpus.rs:329-340`, `CorpusRow` `corpus_halt.rs:129-139`, neither `deny_unknown_fields`). Corrected in place. |
| AC2: "30 rows need 30 cassette entries (the port is sequence-keyed, cursor `cassette_replay.rs:133-144`)" | Holds (`complete_ref` `crates/maos-bin/src/cassette_replay.rs:133-167`). **However,** the all-zero `prompt_sha256` wildcard skips the drift check **even when strict** (`:151-152`), so a wildcard entry would silently accept a changed prompt. The eval cassette must carry real per-row hashes (CR-3). The prompt is deterministic (`proposal_prompt` `spirits/butler/src/lib.rs:170-193`), and NFR-Test-1 asks for the prompt-version hash committed alongside (`prd/non-functional-requirements.md:78`). |
| AC3: "the nightly re-record leg refreshes the cassettes" | `tier-2-rerecord` (`.github/workflows/journey-nightly.yml:67-107`, record step `:89-100`) runs only `cargo nextest -p maos-journey-test` under `MAOS_INFERENCE_MODE=record` and uploads `crates/maos-journey-test/cassettes/` (`:102-107`). Nothing records the eval cassette today. → D-18-2-06. The epic's `journey-nightly.yml:65-99` and `xtask/src/main.rs:663-671` (`:29`) are stale: the age gate is `CassetteAgeGate` `xtask/src/main.rs:736`, dispatched at `:1417-1421`. Corrected in place. |
| AC4: "Butler, Researcher and Orchestrator have completion-consuming class corpora" | **Researcher:** the halt scalars `methodology_conflict`/`load_bearing_confidence` are claim-metadata arithmetic (`spirits/researcher/src/lib.rs:835-886`). The model only summarizes (`synthesize` `:914-958`). `primary_scalar` (`:265`) has no production caller, and no `maos-bin` path writes a Researcher scalar (`grep methodology_conflict crates/maos-bin/src` = 0). **Orchestrator:** no scalar producer exists. `dispatch_ambiguity` appears only in `maos-bin` test modules (`main.rs:13981,14274-14294`), and the manifest omits `provider.complete` by Decision E (`spirits/orchestrator/manifest.toml:33-48`). `19-1-orchestrator-dispatch-loop-and-epic-input` (backlog, `sprint-status.yaml:287`) adds the Orchestrator inference port. → D-18-2-05. FR15's three pathways exist generically (`maosctl halt resolve --kind provided-context\|accepted-halt\|authorized-override`, `crates/maos-cli/src/cli.rs:729-762`; kernel `HaltRegistry::resolve` `crates/maos-kernel-core/src/halt/mod.rs:243`, `journal_halt_resolution` `:51`). They are proven in kernel and `maosctl` tests (`crates/maos-kernel-core/tests/halt_resolution_journaled.rs`, `crates/maos-cli/tests/halt_resolve_test.rs`) but never per class. |
| `tests/verb_table_15_3.rs` (not cited by the epic) | **Regression hazard:** `unknown_verb_exits_non_zero_with_the_table` uses `maos eval` as *the* unknown verb (`crates/maos-bin/tests/verb_table_15_3.rs:387-404`). It must switch to an unknown token, and the expected surfaces (`:45-55`, `:68`, `:163`, `:168`) gain `eval`. |
| Stories table `:91`: maos-bin +80–200, maos-eval +150–300 | Measured by prototype (§Kernel and kloc pricing); corrected in place to the measured figures, pending D-18-2-01. |
| Holds | `grep -n '"eval"' crates/maos-bin/src/main.rs` = 0; `grep -n Eval crates/maos-cli/src/cli.rs` = 0; `spirits/butler/Cargo.toml:35` (kernel dev-dep); corpus `wc -l` = 30; `BUTLER_CORPUS_REL` `onboarding_gate_corpus.rs:79`; floors `:60,:62`; PRD 0.7 `non-functional-requirements.md:81`; FR15 `functional-requirements.md:49`; `xtask/gate-registry.toml:57`; routing audit `:36`; tracker `sprint-status.yaml:278` (backlog); `epic-18-exit` job `discipline.yml:2233-2313`, in `aggregate.needs` `:5683`; kernel-Δ 0 (measured). |

## Kernel and kloc pricing (E16-A7) — measured at creation

Method: each variant was written in place at `140b906b`, built (`cargo build -p maos-bin --bin maos`), run from the workspace root, `cargo fmt --all`'d, and measured with `cargo run -p xtask -- kloc-check --json` (tokei 14.0.0) and `check-kernel-baseline`. Each was reverted by copy-back, followed by `git status` (only a pre-existing untracked `18-8-evidence/ci/run-40191749007-excerpt.log`, not this story's), `check-kernel-baseline` PASSED 26377 / 95, and a rebuild of the HEAD `maos` binary. The cassette for the runs was a temporary 30-entry seed (wildcard hashes, drift = the row's `preference_alignment` or 0.95), deleted after.

| Variant | maos-bin (28005) | maos-eval (4041) | maos-bench (1512 / 1791) | Kernel tokei / physical | Run (seed cassette) |
|---|---|---|---|---|---|
| P0 — Butler + kernel composition in `maos-eval` behind a feature | — | — | — | — | **refused by cargo**: `cyclic package dependency … maos-kernel-core → maos-eval → maos-kernel-core` |
| **P-A** — `crates/maos-bin/src/eval_halt.rs` (lib, network) composes per row: fresh kernel world (orchestrator, in-memory TL, temp journal), `Butler::with_scenario(row).with_scalar_port(..).with_deferred_inference_port(counting replay port, binding)`, `on_idle`, `last_halt_receipt()`; kernel-free `maos_eval::halt_replay::score_class` (counts, floors, sha); VERBS row + both dispatch arms + `run_eval` | **28182 (+177)**: `eval_halt.rs` 138, verb/arms/`run_eval` 39 | **4091 (+50)** | — | **20608 / 26377 (0)** | recall 1.0, precision 1.0, 30/30 consumed, exit 0 |
| **P-B** — the same composition in `crates/maos-bench/src/halt_eval.rs` (kernel, `butler`, `researcher` are already prod deps, `crates/maos-bench/Cargo.toml:17-19`; `toml` added), port injected; `maos-bin` builds the strict replay port, calls it, scores via `maos-eval` | **28055 (+50)** | **4091 (+50)** | **1646 (+134)**, no raise | **20608 / 26377 (0)** | identical numbers |

Plants measured on P-A (each one run ≈ 0.3 s):

| Plant | Output | Exit |
|---|---|---|
| `tn01,tn03,tn05` drift 0.3 | precision 0.833 | 1 |
| `pb01–06` drift 0.3 | precision 0.714 | 1 |
| `cc01–12` drift 0.1 | recall 1.0 (unchanged) | 0 |
| `df01–03` drift 0.99 (drift-blind) | recall 1.0, precision 1.0; overall recall over all 15 expected halts would be 12/15 = 0.80 (computed from the run's verdicts) | 0 |
| cassette cut to 29 entries | `tn09: pass refused: … cassette replay: exhausted` | 1 |
| port unbound (code plant) | `0 of 30 completions consumed` | 1 (272 ms) |

## Decisions — RULED 2026-10-25 by the operator (Lunarpulse)

**Ruling record.** D-18-2-01 **B**, 02 **B**, 03 **A**, 04 **A**, 05 **A** (the operator deviated from the creation recommendation B), 06 **A**. kloc: maos-bin pre-ratified **≤ 28140** for 18-2, and Epic 18's named maos-bin budget raised **+250 → +385** (27809 → 28194). D-18-2-07 is the standing measured-at-landing contract. Review on `anthropic/claude-opus-5-5`, no fable. CR-n are creation defaults carried into the rulings.

| ID | Question | Options | Ruling and rationale | Status |
|---|---|---|---|---|
| **D-18-2-01** | Where does the Butler + kernel replay composition live? `maos-eval` is impossible (P0). | A — maos-bin lib (measured maos-bin +177) · **B — composition in `maos-bench`, thin `eval` verb in maos-bin, kernel-free scorer in `maos-eval`** (measured maos-bin +50, maos-bench +134 of 279 headroom, maos-eval +50) · C — a new crate | **B.** Smallest maos-bin growth. `maos-bench` already holds the kernel, `butler` and `researcher` deps (`crates/maos-bench/Cargo.toml:17-19`), and maos-bin already depends on it (`crates/maos-bin/Cargo.toml:66`). The crate description (`:8`) widens to measurement harnesses. | RULED 2026-10-25 — B |
| **D-18-2-02** | Which recall is NFR-Test-4's class recall? | A — as filed, labelled arithmetic · **B — publish the filed calendar-conflict recall AND `halt_recall_overall` over all 15 expected-halt rows, floor 0.90** · C — model-derived `belief_variance` | **B.** It is the only option that catches a model ignoring the comms signal. The drift-blind cassette fails at 12/15 = 0.80 and is the committed negative control and plant (AC1). | RULED 2026-10-25 — B |
| **D-18-2-03** | Provenance of the 30-entry cassette | **A — hand-authored seed**, real per-row prompt hashes · B — one operator recording · C — A, then B before `done` | **A.** Hermetic exit (rule 2). The sidecar and stdout state `provenance: seed` (rule 11). AC3's record mode replaces it with `live-record`. | RULED 2026-10-25 — A |
| **D-18-2-04** | Sidecar signing key and read surface | **A — `benchmark.signed.json` in `LocalFsRegistryStorage` via new `RegistryStorage` methods, Ed25519 by a producer seed/pubkey pair mirroring Story 11.7 (`crates/maos-eval/src/trial_attestation.rs:33-87`; `MAOS_EVAL_PRODUCER_SEED`/`_PUBKEY`, else the documented dev seed, labelled `signer: dev-seed`), read and verified by `maosctl spirit benchmark`** · B — A plus a registry-server method · C — unsigned | **A.** Reuses `maos_audit::release_verify` and the existing producer-key pattern. No server method; the exit-line comment reads "registry storage holds, `maosctl` verifies". AC5 gets a Security review layer on opus-5-5. | RULED 2026-10-25 — A |
| **D-18-2-05** | AC4: Researcher and Orchestrator classes, FR15 per class | **A — Researcher class corpus here, over `Researcher::survey` → `primary_scalar` through the kernel halt path, labelled not-model-driven; Orchestrator → `19-1` as a received item; FR15 pathways per shipped class (Butler, Researcher)** · B — Butler only; Researcher → 18-3, Orchestrator → 19-1 · C — all three now | **A (operator; deviates from the creation recommendation B).** The PRD's per-class floor is met where a producer exists, and labelled honestly. The Orchestrator item is filed as `epic-19-founder-loop-w4.md` §19-1 AC5 (received). **D-E tension, recorded:** 19-1 AC5 must not require 18-2 to have landed. It reuses the `maos-bench`/`maos-eval` scorer if present at its dev HEAD, else ships its own harness with the same floors, and 18-2 later adopts that corpus. No 19-1 → 18-2 edge is added. | RULED 2026-10-25 — A |
| **D-18-2-06** | AC3: re-recording the eval cassette | **A — `maos eval halt` honours `MAOS_INFERENCE_MODE=record` through the shared router's live driver wrapped by `CassetteRecordPort` (`crates/maos-bin/src/cassette_replay.rs:326-360`); the nightly `tier-2-rerecord` runs it** · B — operator procedure only · C — through a journey test | **A.** It is the only option under which AC3 is true, with one record path (15-6's wrapper). Its maos-bin lines count against the ≤ 28140 pre-ratification. | RULED 2026-10-25 — A |
| **D-18-2-07** | kloc / kernel governance | Exact measured requests at landing (after `cargo fmt --all`, `kloc-check --json`), pinned with dated notes citing this key. `_aggregate_hardfail` = measured + 1. Kernel 0; any kernel line found at landing stops the story for a FLAG-Winston D-row. **Operator pre-ratification 2026-10-25: maos-bin ≤ 28140 for 18-2** (+135 over 28005: the verb +50 measured, record mode +40–80 [INFERENCE], sidecar call). **Epic 18's named maos-bin budget is +385** (was +250; 27809 → 28194). Other crates per forecast (maos-eval needs a measured request: zero headroom, ledger `xtask/kloc.toml:672-673`; maos-bench, maos-registry, maos-cli fit their ceilings). Exceeding any ceiling still needs a measured request. | Standing contract 2026-10-23 (`18-1` D-18-1-13, `18-8` D-18-8-02) + the 2026-10-25 pre-ratification. | STANDING + RULED 2026-10-25 |
| CR-1 | Verb shape | `eval` is a `VERBS` row for **network only** (`builds: &[Build::Network]`; the air-gap table rejects it as unknown, like `shell`/`traceback`), because `cassette_replay` (`crates/maos-bin/src/lib.rs:15-16`) and the Butler arm are network-gated. Syntax: `maos eval halt --class <class> [--cassette <path>]`. `--cassette` overrides the rows' completion reference (for the twin and tests only). Exit 0 when every floor holds. Exit 1 on a missed floor, a refused pass, consumed ≠ rows, drift/exhaustion, or a sidecar write failure. Exit 2 on usage. stdout: one JSON line `{"event":"eval_halt", …numbers, provenance, signer}`. | — |
| CR-2 | Run discipline | The replay port is always strict (`CassetteReplayPort::from_file(path, true)`). Each row gets a fresh kernel world (no cross-row state, as `corpus_halt.rs:73-102`). The Butler binding names Butler's first declared provider (`spirits/butler/manifest.toml:21`). The replay port does not check the token (18-1 AC7 owns that control). A pass that sets `take_pass_error` aborts the run. | — |
| CR-3 | Corpus and cassette fields (AC2) | Each row gains `"completion": {"cassette": "crates/maos-journey-test/cassettes/j-butler/halt-corpus-v0.3.json", "sequence": N}` and `"prompt_sha256"`. The cassette's `entries[N]` carries the real `prompt_sha256`/`prompt_len` of `proposal_prompt(row.input, assess(row.input))`. **No wildcard hash** in this cassette: a test asserts it. A generator `#[ignore]` test writes hashes and seed completions (precedent: the digest corpus generator, `spirits/butler/tests/corpus_pin.rs:33-34`). Re-pin `corpus_pin.rs:23` and `tests/coverage-matrix.yaml:1218` in the same commit. The cassette lives under the journey cassette tree so `cassette-age-gate` and the nightly upload (`journey-nightly.yml:102-107`) see it. | — |
| CR-4 | CI | `epic-18-exit` (`discipline.yml:2233-2313`) gains exit line 2 verbatim (scratch `HOME`, keyless) and a twin running the committed **drift-blind** cassette via `--cassette`. The twin must exit 1 with `halt_recall_overall` 0.80 (D-18-2-02 B). It also uploads the sidecar. Precedent D-18-1-11 A. NFR-Test-4's coverage row gets `gates: [eval-halt]` and `corpora: [spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl, spirits/researcher/tests/fixtures/halt-corpus-v0.5.jsonl]`, and `xtask/gate-registry.toml` gains `"eval-halt"` (the `coverage-matrix` gate refuses unknown names, `xtask/src/coverage_matrix.rs:142`). | — |
| CR-5 | Class table (D-18-2-05 A) | One table in `maos-bench` maps class → corpus, composition, floors and `model_driven`. **butler**: the Butler corpus, recorded completions, floors calendar-conflict recall ≥ 0.90, overall recall ≥ 0.90, precision ≥ 0.85, `model_driven: true`; the consumed-completion refusal applies. **researcher**: NEW `spirits/researcher/tests/fixtures/halt-corpus-v0.5.jsonl` (30 rows of `ClaimPayload` sets (`spirits/researcher/src/lib.rs:190-204`) wrapped as the `RecalledFrame`s `survey` reads (`:755`), each with `expected_halt` from the manifest rules `methodology_conflict` > 0.7 / `load_bearing_confidence` < 0.7, `spirits/researcher/manifest.toml:115-131`; a mix of conflict, low-confidence load-bearing, and negative rows). Each row is scored through `Researcher::survey` → `SurveyOutput::primary_scalar` (`:265`) → kernel `process_scalar_write` with the Researcher policy. Floors NFR-Test-4 overall recall ≥ 0.70, precision ≥ 0.85. `model_driven: false`, printed and stored in the sidecar with the reason (claim-metadata arithmetic, `:835-886`; no production scalar writer). No cassette and no consumed-completion check. **orchestrator**: not a row; received by `19-1` AC5. An unknown class exits 2. | — |

## Acceptance Criteria

*(Epic §18-2 as corrected and ruled 2026-10-25. AC5 is split out of epic AC1's sidecar clause; it is a tightening, not new scope. One plant per control AC, `docs/testing/proof-contract.md:26`, except where named.)*

### AC1 — command: exit line 2 replays the Butler corpus through the seam (D-18-2-01 B, 02 B, 03 A; CR-1, CR-2, CR-4)

1. `maos eval halt --class butler` (run from the workspace root, keyless) is a network-only `VERBS` row (`crates/maos-bin/src/verbs.rs:59-68,88-137`) dispatched by the network `main()` (`main.rs:1790`) to a thin `run_eval`. The replay composition lives in `maos-bench` (D-18-2-01 B). For each of the 30 rows (`spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl`) it runs one bound Butler pass on a fresh kernel world: `Butler::with_scenario` + `with_scalar_port` (kernel `WorkingMemoryOrchestrator::process_scalar_write`) + `with_deferred_inference_port` (strict `CassetteReplayPort`), then `on_idle` (`spirits/butler/src/lib.rs:767-775`). `user_preference_drift` comes from the row's recorded completion (`:852`). The verdict is `last_halt_receipt().is_some()`.
2. The kernel-free scorer (`crates/maos-eval/src/halt_replay.rs`) prints:
   - `halt_recall_calendar_conflict` (the filed NFR-Onb-1 recall, ≥ 0.90, labelled arithmetic under the 18-1 contract);
   - `halt_recall_overall` over all 15 expected-halt rows (NFR-Test-4 class recall, ≥ 0.90, D-18-2-02 B);
   - `halt_precision_overall` (≥ 0.85);
   - the counts, the corpus and cassette SHA-256, `provenance` (`seed`, D-18-2-03 A) and `model_driven: true`.

   It exits per CR-1. The floors come from one source, `HALT_RECALL_FLOOR`/`HALT_PRECISION_FLOOR` (`crates/maos-eval/src/onboarding_gate_corpus.rs:60,62`) plus the class table (CR-5), and are never re-typed.
3. Reds:
   - the committed **drift-blind** cassette (`df01–03` drift 0.99): overall recall 12/15 = 0.80, exit 1. This is the negative control and the CI twin.
   - a precision-planted cassette (`tn01,tn03,tn05` drift 0.3): precision 0.833, exit 1 (measured at creation).
   - consumed completions ≠ rows (the arithmetic path).
   - a refused pass, exhaustion, or strict drift.
4. `crates/maos-bin/tests/verb_table_15_3.rs` gains `eval` in the expected network surface (`:45-55`, `:68`, `:168`). Its unknown-verb probe (`:387-404`) switches from `eval` to an unknown token. `check-exit-commands` resolves exit line 2 (no `owed` row for `eval`).
5. Process-level witness `crates/maos-bin/tests/eval_halt_18_2.rs` (runs `env!("CARGO_BIN_EXE_maos")`, fresh `HOME`, no key, no nested cargo):
   - the seed cassette exits 0 with the numbers;
   - the drift-blind and precision-planted cassettes exit 1 and name the missed floor;
   - a 29-entry cassette exits 1 naming the row;
   - a drifted prompt hash exits 1;
   - `--class nosuch` exits 2.
6. **Proven red (2 plants, named under `proof-contract.md:26`'s exception: two independent controls):**
   - (a) Unbind the inference port in the composition → `0 of 30 completions consumed`, exit 1 (measured at creation, 272 ms).
   - (b) Drop the `halt_recall_overall` floor → the drift-blind case exits 0 → the witness reds.

   In CI the drift-blind twin is the standing red.

### AC2 — the corpus carries its model I/O (CR-3; D-18-2-03 A)

1. Every Butler row carries `completion {cassette, sequence}` and `prompt_sha256`. The hand-authored 30-entry seed cassette carries real prompt hashes, `provenance: seed`, `model_id: hand-authored-seed` and `recorded_at`. `corpus_pin.rs:23` and `coverage-matrix.yaml:1218` are re-pinned in the same commit. `corpus_halt.rs` stays green (it ignores the new fields).
2. Tests (`spirits/butler/tests/`):
   - every row's `sequence` is unique and equals its index;
   - every entry's hash equals `sha256(proposal_prompt(row))`;
   - no entry carries the wildcard hash;
   - the drift-blind and precision-planted fixtures differ from the seed only in the planted rows.
3. **Proven red (1 plant):** set one entry's hash to the wildcard → the no-wildcard test reds.

### AC3 — the eval cassette is re-recordable, and the nightly leg records it (D-18-2-06 A)

1. `MAOS_INFERENCE_MODE=record MAOS_REPLAY_CASSETTE=<out> maos eval halt --class butler` uses the shared router's live driver, through the kernel `InferencePortAdapter` with a token minted for Butler's declared provider (`spirits/butler/manifest.toml:21`), wrapped by `CassetteRecordPort` (`crates/maos-bin/src/cassette_replay.rs:326-360`). One record path, never a second copy. It writes a 30-entry cassette with real hashes and `provenance: live-record`. Unset mode or `replay` replays as AC1.
2. `.github/workflows/journey-nightly.yml` `tier-2-rerecord` (`:67-107`) runs it after the journey step (record step `:89-100`). The upload (`:102-107`) already covers the cassette directory. *Review ruling D-18-2-R3 (2026-10-25): the step records into the sibling `halt-corpus-v0.3.live.json`, never the committed seed; promotion is the operator procedure in §Review.*
3. Hermetic witness in `eval_halt_18_2.rs`: `record` against a local stub provider writes 30 entries with real hashes and `provenance: live-record`, then replays them green.
4. The live leg is operator lane (`ops-nightly-cassette-rerecord`), never gating. No CI plant (rule 2); the hermetic witness is the proof.

### AC4 — per-class numbers and FR15 pathways (routing audit `:36`; D-18-2-05 A; CR-5)

1. `--class` resolves corpus, composition, floors and `model_driven` from one class table in the `maos-bench` composition (CR-5). An unknown class exits 2.
2. **Butler**: AC1's floors.
3. **Researcher** (`maos eval halt --class researcher`): the NEW 30-row corpus `spirits/researcher/tests/fixtures/halt-corpus-v0.5.jsonl` is scored through `Researcher::survey` → `primary_scalar` → the kernel halt path with the Researcher `[epistemic_policy]`.
   - It meets the NFR-Test-4 floors: overall recall ≥ 0.70, precision ≥ 0.85. 0.70 is one new constant beside the floors at `onboarding_gate_corpus.rs:60,62`.
   - It is published as `model_driven: false`, with the reason (claim-metadata arithmetic, `spirits/researcher/src/lib.rs:835-886`; no production scalar writer, rule 11(a)), in stdout and the sidecar.
   - A pin test fixes the corpus SHA-256.
4. **Orchestrator**: not a row here. It is received by `19-1-orchestrator-dispatch-loop-and-epic-input` AC5 (`epics/epic-19-founder-loop-w4.md` §19-1). That AC does not depend on 18-2 landing (D-E).
5. **FR15 per shipped class** (`prd/functional-requirements.md:49`): for Butler and for the Researcher, a witness halts one row through the kernel path. It resolves the halt once per kind (`provided-context`, `accepted-halt`, `authorized-override`) via `HaltRegistry::resolve` (`crates/maos-kernel-core/src/halt/mod.rs:243`). Each resolution is journaled with its reasoning chain (`journal_halt_resolution` `:51`).
6. **Proven red (1 plant):** skip the journal write in `HaltRegistry::resolve` (temporary kernel edit, restored; kernel-Δ stays 0) → the FR15 witnesses red.

### AC5 — the numbers are published as a verifiable signed sidecar (D-18-2-04 A; split from epic AC1)

1. Every run that produced numbers (exit 0 or 1) writes `spirits/<class>/<manifest version>/benchmark.signed.json` in `LocalFsRegistryStorage` (`crates/maos-registry/src/storage.rs:82-117`) through new `RegistryStorage` methods (`:34-79`).
   - The body holds the numbers, floors, pass/fail, corpus/cassette SHA-256, `provenance`, `model_driven`, the `maos` version and `signer`.
   - It is signed Ed25519 via `maos_audit::release_verify::sign_sha256sums` (`crates/maos-audit/src/release_verify.rs:174`). The key is `MAOS_EVAL_PRODUCER_SEED`, else the documented dev seed labelled `signer: dev-seed`, mirroring `crates/maos-eval/src/trial_attestation.rs:33-87`.
   - The seed never appears in a log or error.
2. `maosctl spirit benchmark <class> [--version <v>]` (a new `SpiritOp`, `crates/maos-cli/src/cli.rs:823`) prints the body and verifies the signature (`verify_release_signature`, `release_verify.rs:154`) against `MAOS_EVAL_PRODUCER_PUBKEY` or the dev pubkey. A missing sidecar or a bad signature exits non-zero, typed.
3. **Proven red (1 plant):** skip the signature check in the read verb → `crates/maos-cli/tests/benchmark_sidecar_18_2.rs`'s tampered-body case exits 0 → red.

## Tasks / Subtasks

- [x] **T0 — Preflight at the dev HEAD.**
  - Every D-18-2 row RULED (2026-10-25); story and tracker `ready-for-dev` → `in-progress`.
  - Run `git diff --stat 140b906b -- crates/maos-bin/src/{main.rs,verbs.rs,lib.rs,cassette_replay.rs} crates/maos-eval crates/maos-bench crates/maos-registry/src crates/maos-cli/src spirits/butler spirits/researcher .github/workflows/{discipline,journey-nightly}.yml tests/coverage-matrix.yaml xtask/gate-registry.toml`; re-anchor any drift in the Debug Log.
  - Re-measure `kloc-check --json` and `check-kernel-baseline` (expect kernel 20608 / 26377 unless another story moved it).
  - Check whether 19-1 has landed an Orchestrator corpus (D-18-2-05); if so, adopt it as a class row.
- [x] **T1 — Scorer** (AC1.2, AC4; D-18-2-02 B) — `crates/maos-eval/src/halt_replay.rs` (kernel-free): class rows, `score_candidate` reuse, `halt_recall_overall`, the NFR-Test-4 0.70 constant, consumed-count refusal, sidecar body type.
- [x] **T2 — Composition** (AC1.1, AC4; CR-2, CR-5; D-18-2-01 B) — `crates/maos-bench/src/halt_eval.rs`: class table, per-row kernel world, bound Butler + counting port, Researcher `survey` path, verdicts. Widen the crate description (`crates/maos-bench/Cargo.toml:8`).
- [x] **T3 — Verb** (AC1.1–1.4, CR-1) — `verbs.rs` row + `VerbName::Eval`; network arm + air-gap `unreachable!`; thin `run_eval` (`--class`, `--cassette`); `verb_table_15_3.rs` updates.
- [x] **T4 — Corpora + cassettes** (AC2, AC4.3; CR-3, CR-5) — generator test; 30-entry hashed seed cassette; drift-blind and precision-planted fixtures; Butler row fields; re-pins; no-wildcard test; the Researcher corpus and its pin.
- [x] **T5 — Record mode** (AC3; D-18-2-06 A) — `record` path through `CassetteRecordPort`; hermetic stub witness; nightly step.
- [x] **T6 — FR15** (AC4.5) — Butler and Researcher resolution witnesses.
- [x] **T7 — Sidecar** (AC5; D-18-2-04 A) — storage methods, signing, `maosctl spirit benchmark`, `benchmark_sidecar_18_2.rs`; env rows `MAOS_EVAL_PRODUCER_SEED`/`_PUBKEY` in `crates/maos-bin/src/env_contract.rs` (`check-env-contract`).
- [x] **T8 — CI + coverage** (CR-4) — `epic-18-exit` line 2 + drift-blind twin + sidecar artifact; NFR-Test-4 row (gate + both corpora); `gate-registry.toml` `eval-halt`.
- [x] **T9 — Plants and records** — AC1 (a)+(b), AC2, AC4, AC5, recorded once in `18-2-evidence/plants.md` (command, output, wall-clock); epic §18-2 if anything moved at dev HEAD.
- [x] **T10 — Governance** (D-18-2-07) — `cargo fmt --all`; `kloc-check --json`. maos-bin must stay ≤ 28140 (pre-ratified); maos-eval and any crate over its ceiling get a measured request returned **before** landing. Also run:
  - `check-kernel-baseline` (expect unchanged), `check-exit-commands`, `coverage-matrix`, `check-env-contract`, `check-test-cost`, `check-dev-record-completeness`;
  - `cargo nextest run -p xtask --locked --test d11_xtask_ceiling_ratchet --test recovery_lane_ceiling_rule` before the closing commit.

## Dev Notes

### What changes, by file

| Area | Files | Today → after |
|---|---|---|
| Verb | `crates/maos-bin/src/verbs.rs:59-68,88-137`; `main.rs:1466-1476` (air-gap arms), `:1814-1876` (network arms), new `run_eval`; `crates/maos-bin/Cargo.toml` (`maos-eval` dep) | unknown verb exit 1 → `eval` row |
| Composition | `crates/maos-bench/src/halt_eval.rs` (D-18-2-01 B; class table CR-5), `crates/maos-bench/Cargo.toml:8,10-20` | `corpus_halt.rs` test-only world → production composition (Butler + Researcher) |
| Researcher class | new `spirits/researcher/tests/fixtures/halt-corpus-v0.5.jsonl` + pin test; FR15 witness in `spirits/researcher/tests/` | no class corpus → labelled not-model-driven class |
| Scorer | new `crates/maos-eval/src/halt_replay.rs`, `crates/maos-eval/src/lib.rs:17-31` | — |
| Corpus | `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl`, `spirits/butler/tests/corpus_pin.rs:23`, new cassette `crates/maos-journey-test/cassettes/j-butler/halt-corpus-v0.3.json`, `tests/coverage-matrix.yaml:1218` | no model I/O → completion refs + hashes |
| Sidecar | `crates/maos-registry/src/storage.rs:34-117`; `crates/maos-cli/src/cli.rs:823` (`SpiritOp`) + handler; `crates/maos-eval` signing | none → signed, verified |
| CI | `.github/workflows/discipline.yml:2233-2313` (`epic-18-exit`), `.github/workflows/journey-nightly.yml:89-100`, `tests/coverage-matrix.yaml:1896-1898`, `xtask/gate-registry.toml` | — |
| Tests | new `crates/maos-bin/tests/eval_halt_18_2.rs`, `crates/maos-cli/tests/benchmark_sidecar_18_2.rs`, `spirits/butler/tests/` (corpus/cassette integrity, FR15 witness), `crates/maos-bin/tests/verb_table_15_3.rs` | — |

### Current state that must be preserved

- **NFR-Onb-1** (`nfr-onb-1-gate`, `resolve_corpus` `onboarding_gate_corpus.rs:280-304`, `score_candidate` `:446-522`): semantics unchanged. The resolver still prefers the Butler corpus. Only its SHA moves (re-pinned).
- **`corpus_halt.rs`** (`cargo test -p butler`, `discipline.yml:991`) keeps its self-validation and floor asserts (`:297-306`).
- **Unset mode = deterministic** (ADR-064). The verb is replay-only (plus `record` under D-18-2-06 A). `maos run` arms are untouched.
- **Verb table contract** (`verbs.rs:1-24`): the `maos --help` shape is frozen; one new row, no other change.
- **The kernel**: no file under `crates/maos-kernel-core/src` changes (AC4.3's plant is temporary).

### Guardrails

- Never re-type the floors; never score a run that consumed fewer completions than rows; never treat a refused pass as "no halt".
- One record/replay path: `CassetteReplayPort`/`CassetteRecordPort` (`cassette_replay.rs:17-175,326-360`), never a second cassette reader.
- No wildcard prompt hash in the eval cassette (strict mode does not catch it, `cassette_replay.rs:151-152`).
- The sidecar never claims more than the cassette recorded (`provenance`, `signer`). A dev-seed signature is labelled as one.
- One plant per control AC (AC1 two, named), in place, restored by `git checkout -- <file>`. One target dir. Local runs `env -u CI -u GITHUB_ACTIONS`. No `cargo` inside a `#[test]` (`proof-contract.md:22-27,68-76`).
- If `intent-lineage-coverage-report.md` is modified by a kernel run, revert it.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red |
|---|---|---|
| Model-consumption refusal (AC1 a) | unbind the inference port | `0 of 30 completions consumed`, exit 1 (creation: 272 ms) |
| Floors (AC1 b) | drop the `halt_recall_overall` floor | `eval_halt_18_2` drift-blind case exits 0 (must exit 1 at 12/15 = 0.80) |
| Prompt pinning (AC2) | wildcard one entry | no-wildcard test |
| FR15 journaling (AC4) | skip the resolution journal write | FR15 Butler and Researcher witnesses |
| Sidecar signature (AC5) | skip verification | tampered-body case exits 0 |

In CI the `epic-18-exit` twin (drift-blind cassette → exit 1 at `halt_recall_overall` 0.80; CR-4, AC1.3, D-18-2-02 B) is the standing red for exit line 2. The landing run of the whole matrix is the green proof. *(Corrected at review 2026-10-25, AA-7: this line said "precision-planted"; the ruling, the workflow and `ci_shape_18_0` all use the drift-blind cassette.)*

### Tests and commands

```bash
# local; one cargo process; CARGO_TARGET_DIR=/mnt/build/cargo
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p butler -p researcher -p maos-eval -p maos-bench -p maos-registry
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test eval_halt_18_2 --test verb_table_15_3
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-cli --test benchmark_sidecar_18_2 --test halt_resolve_test
cargo build --locked -p maos-bin --bin maos && target/debug/maos eval halt --class butler   # from the workspace root
cargo run -p xtask -- check-exit-commands && cargo run -p xtask -- coverage-matrix && cargo run -p xtask -- check-kernel-baseline
cargo run -p xtask -- kloc-check --json && cargo run -p xtask -- check-test-cost
cargo fmt --all -- --check
```

### Review plan (proof contract `:31-57`)

- **Reviewer model:** `anthropic/claude-opus-5-5` only (operator ruling 2026-10-25; no fable). The Security layer for AC5 runs on opus-5-5.
- **Focused suites:** `-p butler`, `-p researcher`, `-p maos-eval`, `-p maos-bench`, `-p maos-bin` (`eval_halt_18_2`, `verb_table_15_3`), `-p maos-registry`, `-p maos-cli`. No kernel source change, so no `cargo test -p maos-kernel-core --lib` unless landing measures a kernel line.
- **Gates:** `check-exit-commands` (exit line 2 resolves, `eval` no longer owed), `coverage-matrix` (`eval-halt` registered), `kloc-check`, `check-kernel-baseline` (unchanged), `check-test-cost`, `cassette-age-gate`.
- **Security layer (AC5):** signing key handling (no seed in logs, dev-seed labelling, CI secret), verification on read, canonical body.

### Previous-story intelligence

- **18-1 (review, landing):** the seam this story drives (`with_deferred_inference_port` `spirits/butler/src/lib.rs:966-973`; `parse_proposal` `:133-165`; `proposal_prompt` `:170-193`). D-18-1-03 A completion contract; D-18-1-04 A refused passes; JB-5 pattern for exit-code witnesses; `epic-18-exit` created there (D-18-1-11 A). The per-pass mint (D-18-1-05 A) is not needed by a replay-only port (CR-2).
- **18-8 (review, landing):** D-18-8-R5 put maos-bin at ≤ 28050 pre-ratified with Epic 18's named budget effectively exhausted. That is why D-18-2-01 is measured.
- **8.1 / 7.5b:** the corpus, its self-validation and the NFR-Onb-1 scorer whose seam (`Some(observations)`) this story feeds.
- **15-3:** the verb table and its test (`verb_table_15_3.rs`), whose unknown-verb probe is `eval`.
- **15-6:** one selector (ADR-064), the cassette format, `CassetteRecordPort`.
- **11.7:** the producer seed/pubkey pattern D-18-2-04 A mirrors.

### Git intelligence

`140b906b` 18-8 dev + review (`maos-bin` 28005, kernel 20608 / 26377); `08ae68ad` 18-1 CI proven-red evidence; `a85673f0` 18-1 dev + review (Butler seam, `epic-18-exit`); `7b0b59e1` 18-1 created; `c816c544` 18-0 done. None touched the corpus, `maos-eval`, `maos-bench`, `maos-registry` or `verbs.rs`.

### References

- Epic `epic-18-spirits-that-think-w3.md`: §18-2 `:138-149` (creation corrections `:142`, rulings `:144`), exit block `:11-26` (`:15`, `:23`, `:29`), Kernel-Δ `:32`, kloc asks `:36`, rules 9–11 `:38-40`, §R `:44-81` (R18-11 `:60`, R18-12 `:61`, R18-22 `:71`), stories table `:91`, Dependencies `:224-226`. Epic 19 §19-1 AC5 (received item, `epic-19-founder-loop-w4.md:63`).
- `routing-audit-2026-10-22.md:36,108`; `prd/non-functional-requirements.md:78,81`; `prd/functional-requirements.md:49`; `review-2026-09-04/preflight-r2/epic-18.md:85,187-188,230,234` (§H-2/§H-3 origin); `15-2-kloc-ceiling-rebase.md:389`; `docs/testing/proof-contract.md:22-27,31-57,68-76`.

## Dev Agent Record

### Story context creation

Created 2026-10-25 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter creation contract) at `140b906b`. Inputs: direct reads of every cited range; a `jq` profile of the 30 corpus rows; `check-exit-commands --json` (`eval` owed to this key); `kloc-check --json` and `check-kernel-baseline` at HEAD; the HEAD binary's red (`maos: unknown command 'eval' in network mode`, exit 1). Three throwaway prototypes, measured and reverted by copy-back (§Kernel and kloc pricing): P0 refused by cargo; P-A and P-B built and ran against a temporary seed cassette, with five data plants and one code plant. After the revert: `git status` showed no crate or workflow change, and `check-kernel-baseline` PASSED 26377 / 95. The HEAD `maos` binary was rebuilt so the shared target dir matches HEAD. `intent-lineage-coverage-report.md` was not modified.

### Agent Model Used

anthropic/claude-opus-5-5 (creation, 2026-10-25); anthropic/claude-opus-5-5 (dev, 2026-10-25: two passes, Dev182 then Dev182b, on the uncommitted tree above `140b906b`)

### Debug Log References

- One cargo process, `CARGO_TARGET_DIR=/mnt/build/cargo`, local runs under `env -u CI -u GITHUB_ACTIONS`.
- T0 (first pass, at the dev HEAD `140b906b`): the `git diff --stat 140b906b` preflight was run, and no anchor drift was recorded. 19-1 is `backlog` with no Orchestrator corpus, so the class table has two rows. Kernel 20608 / 26377, re-measured at landing in the second pass.
- First pass: a `maos` build failed once in the maos-cli lib while the Cargo.toml edits were in flight. Every later build of `maos` and `maosctl` was clean, and the error did not recur.
- Story-text inconsistency: §Proven-red controls (last paragraph) names the precision-planted cassette as the CI twin, but CR-4, AC1.3 and D-18-2-02 B name the drift-blind one. The drift-blind cassette was implemented as the twin, following the ruling.

### Completion Notes List

**Per-AC tests**

- **AC1:** `crates/maos-bin/tests/eval_halt_18_2.rs` passes 9/9.
  - Seed cassette: exit 0. Calendar-conflict recall 1.0, overall recall 1.0, precision 1.0; 30/30 completions consumed; provenance `seed`; signer `dev-seed`. The sidecar verifies.
  - Drift-blind cassette: exit 1, `halt_recall_overall 0.800 below floor 0.90`.
  - Precision-planted cassette: exit 1, precision 0.833.
  - 29-entry cassette: exit 1, naming the row. Drifted prompt hash: exit 1. Usage errors: exit 2.
  - `verb_table_15_3` passes 7/7: `eval` is in the network surface, and the unknown-verb probe now uses an unknown token. `check-exit-commands` PASS, with `eval` no longer owed.
- **AC2:** `spirits/butler/tests/halt_eval_corpus_18_2.rs` checks:
  - every row names its own sequence;
  - every entry hash is `sha256(proposal_prompt(row))`;
  - no entry carries the wildcard hash;
  - the seed states its provenance;
  - each control differs from the seed only in its planted rows.

  `corpus_pin` is re-pinned to `60ed4f93…` (also at `coverage-matrix.yaml:1218`). `corpus_halt` stays green.
- **AC3:** record mode with no provider refuses and writes no cassette. Recording through a router whose `anthropic` driver is an in-process stub writes 30 real-hash `live-record` entries, which then replay green. `journey-nightly.yml` `tier-2-rerecord` gains the record step; the upload now runs `!cancelled()` so a live run that misses a floor still uploads.
- **AC4:**
  - `--class researcher`: exit 0, overall recall 14/15 = 0.933, precision 1.0, `model_driven: false` with the reason.
  - `spirits/researcher/tests/halt_corpus_18_2.rs` passes 4/4: SHA pin `b4b3e33e…`, 15 expected halts from the manifest rules, kernel self-validation, and FR15.
  - Butler FR15 witness `fr15_a_butler_halt_resolves_once_per_kind_journaled`: `df01`'s recorded drift 0.4 halts on `user_preference_drift`.
  - Both FR15 witnesses share `tests/harness/fr15_resolution.rs`. Each resolves the halt once per kind through `HaltFlow` over `KernelHaltResolver`, and each resolution is journaled with its reasoning chain.
  - Unknown class: exit 2.
- **AC5:** `crates/maos-cli/tests/benchmark_sidecar_18_2.rs` passes 6/6 against the real `maosctl` under a scratch `HOME`:
  - a dev-seed sidecar verifies and prints;
  - a tampered body exits 1 and prints nothing;
  - a producer signature verifies only against the producer pubkey;
  - a malformed pubkey refuses without echoing the value;
  - a missing sidecar exits 5, typed;
  - with no `--version`, the highest benchmarked version is read.
- **Focused suites:** `-p butler -p researcher -p maos-eval -p maos-bench -p maos-registry -p maos-cli` gave 477 passed, 6 skipped. xtask `ci_shape_18_0`, `ship_gate_binding_20_3a`, `gate_honesty_20_3a`, `d11_xtask_ceiling_ratchet` and `recovery_lane_ceiling_rule` all pass (`--locked`).

**Plants:** six, all proven red, then restored with copy-back, `cmp` and `touch`. Details are in `18-2-evidence/plants.md`:
- AC1 (a) and (b);
- AC2;
- AC3, a local plant (no CI plant, per AC3.4);
- AC4, which reds both FR15 witnesses;
- AC5.

The CI plant is AC1 (b), `ci/plant.diff`, sha256 `f3255f459efd043860f825546056f0ea374941d7852afa89d3dd80c541deedb2`. Simulated locally from the workflow text: planted, the twin step exits 1 with "the drift-blind cassette exited 0". Restored, line 2 gives rc=0 and the twin gives rc=1 naming the floor. The integrator owns the remote CI proven-red run.

**Dev deviations (each is a departure from the story text, recorded for review):**

1. **Replay and record share one path.** Both go through the kernel `InferencePortAdapter`, with a real token minted for Butler's declared provider. CR-2 and AC1.1 had replay hand Butler the strict `CassetteReplayPort` directly. *Reason:* with one path, record mode (AC3.1 requires the adapter and the token) and replay exercise the same composition. The strict replay driver still sits under the router; the token check still belongs to 18-1 AC7.
2. **Record mode flushes the existing recorder.** It flushes the 15-6 router recorder that `MAOS_INFERENCE_MODE=record` already wraps around every driver, instead of adding a second `CassetteRecordPort` wrapper (D-18-2-06 A, AC3.1). *Reason:* a second wrapper would record every completion twice. D-18-2-06 A's "one record path, never a second copy" is honoured.
3. **The AC3 witness records in-process.** It records through an in-process router whose `anthropic` driver is a stub, then replays the result with the `maos` binary. *Reason:* the Anthropic endpoint can't be pointed at a local stub. At process level, record mode with no provider configured refuses and writes no cassette, and the witness asserts that too.
4. **The AC4 plant targets director-surface, not the kernel.** It targets `HaltFlow::submit_resolution` (`crates/maos-director-surface/src/halt_ui.rs:91-92`), not `HaltRegistry::resolve`. *Reason:* `HaltRegistry::resolve` writes no journal. The resolution journal write is in `submit_resolution`, which the FR15 witnesses drive. The kernel is untouched (kernel-Δ 0).
5. **The NFR-Test-4 coverage row names no corpora.** The row is `gates: [eval-halt]`, `corpora: []`, with the two class corpora named in its `notes`. CR-4 had listed them under `corpora:`. *Reason:* `coverage-matrix` accepts in `corpora:` only names registered in `tests/corpora/MANIFEST.toml` (`xtask/src/coverage_matrix.rs:135-138`), and neither class corpus is registered there.

**kloc (D-18-2-07; after `cargo fmt --all`, `kloc-check --json`, tokei 14.0.0):**
- maos-bin 28005 → **28087** (+82: `main.rs` +65, `verbs.rs` +7, `env_contract.rs` +10). This is within the pre-ratified ≤ 28140. Epic 18's named budget is now at +278 of +385.
- maos-eval 4041 → **4242** (+201). This is a measured request against the 15-2 ledger line.
- maos-bench ceiling 1791 → **2018** (measured 1512 → 2018, +506). The creation forecast was +134. **This is a measured request over the ceiling, returned for operator ratification at review.**
- maos-registry 3452 / 4282 and maos-cli 6097 / 6856: no raise needed.
- Aggregate 183180; `_aggregate_hardfail` 182292 → **183181**.
- Kernel 20608 / 26377, unchanged.

**Gates:**
- `kloc-check` PASSED (aggregate=183180).
- `check-kernel-baseline` PASSED (26377 lines, 95 files).
- `check-exit-commands` PASS (4 epics, 12 tokens resolved, 3 owed, all owed by 20-1).
- `coverage-matrix` exit 0: NFR-Test-4 is deferred at v0.5, and the row raises no violation.
- `check-env-contract` PASS (108 vars).
- `check-corpus` PASSED (10 entries).
- `check-test-cost` PASS.
- `cassette-age-gate` exit 0: a WARN for the three pre-existing June seeds; the new eval cassette is dated 2026-10-25.
- `check-service-boundary` PASSED (0 violations).

`intent-lineage-coverage-report.md` was not modified.

**CI (2026-10-25, integrator; TIA-5):** proven-red run `40200067350` (`ci/18-2-verify` @ `68aae558`, job `121189173425`; recorded in `18-2-evidence/plants.md` §CI plant). Landing `40200028265` @ `ee395748`: `epic-18-exit` (exit lines 1, 2 and both twins) green; reds only the 20-7-owned S3 podman-4 bound + fan-in gates. No deferred row is owned by this key.

### Review (2026-10-25, anthropic/claude-opus-5-5, 4 layers)

Layers: Blind Hunter (BH), Edge Case Hunter (EC), Acceptance Auditor (AA), Test-Infra Auditor (TIA), all `anthropic/claude-opus-5-5`. Patches applied by `anthropic/claude-opus-5-5` the same day on the uncommitted tree above `140b906b`; story stays `review`.

**Review rulings D-18-2-R1…R5 (operator Lunarpulse, 2026-10-25):**
- **D-18-2-R1 = K1 A** (BH-7, AA-4): maos-bench 2018 and maos-eval 4242 ratified. The 15-2 ledger reservation "19-3 real-spawn bench +80–200" is consumed; 19-3 makes its own measured maos-bench request at creation (`xtask/kloc.toml` ledger line + HISTORY; `epic-19-founder-loop-w4.md` kloc row and the §19-3 stories-table row). Epic-18 stories table `:91` row updated (landed figures, `review`).
- **D-18-2-R2 = K2 A** (EC-5): the class sidecar is published ONLY when the replay cassette is the class's committed cassette (path-canonical equality), or in record mode. An override-cassette run (`--cassette` or `MAOS_REPLAY_CASSETTE`) prints its numbers, writes no sidecar, and says so on stderr: `maos eval: override cassette — numbers only, no sidecar written`.
- **D-18-2-R3 = K3 A** (EC-6, TIA-1): the nightly re-record writes the sibling `crates/maos-journey-test/cassettes/j-butler/halt-corpus-v0.3.live.json`, never the committed seed. **Promotion procedure (operator):** download the `refreshed-cassettes` artifact of a green `tier-2-rerecord` run; check the live cassette (30 entries, real prompt hashes, `provenance: live-record`, a real `model_id`); replay it with `maos eval halt --class butler --cassette <live>` (numbers only, per R2); then, in a PR, copy it over `halt-corpus-v0.3.json` and re-pin the seed-pinned tests (deferred row, owner `ops-nightly-cassette-rerecord`). The same text is the nightly step's comment. Nothing in the tests or `generate()` writes a `.live.json`, so the seed-pinned tests stay valid.
- **D-18-2-R4 = K4 A** (AA-5, TIA-2): accepted; the nightly record step is the recorded proof of the binary's record-flush success path. Deferred row, owner `ops-nightly-cassette-rerecord`.
- **D-18-2-R5 = K5 B** (AA-6, dev deviation 5 withdrawn): both class corpora registered in `tests/corpora/MANIFEST.toml` with sha256 — `butler-calendar-comms-v0.3` (`60ed4f93…b9da`, 30 items, the v0.3 corpus with the 18-2 halt-eval fields) and `researcher-halt-corpus-v0.5` (`b4b3e33e…7604`, 30 items). `check-corpus` reads `tests/corpora/<name>.jsonl`, so each entry is a committed symlink to the Spirit fixture (one copy of the bytes; the Spirit-side pin tests stay). The NFR-Test-4 row lists both under `corpora:`. `check-corpus` PASSED (12 entries); `coverage-matrix` exit 0, no violation on the row.

**Dispositions:**
- **BH-1 / EC-1 (HIGH) — patched.** `SignedBenchmark.body` is a `Box<serde_json::value::RawValue>` (`raw_value` feature on maos-eval's serde_json). `sign` signs the body's compact JSON and stores those exact bytes; `verify` checks the signature over the stored bytes, then parses — no re-serialization. `maosctl spirit benchmark` prints the exact signed bytes. Test `a_float_that_does_not_round_trip_verifies_byte_exact` (14/15 = `0.9333333333333333`), run with `cargo test -p maos-cli --test benchmark_sidecar_18_2` alone (no `float_roundtrip` in that graph) — the plant proves it red there.
- **BH-2 / EC-2 / AA-2 — patched.** `sidecar::signer_for(pubkey)`: the dev pubkey vouches only for `dev-seed`, any other key only for `producer`; `verify` refuses any other `signer`. `producer_seed` labels by the same rule. maosctl stderr names the trust basis (`signature verified against the development seed's key — integrity, not provenance` / `against MAOS_EVAL_PRODUCER_PUBKEY — the producer`). Test: a dev-seed-signed body claiming `producer` exits 1.
- **BH-3 / EC-3 / AA-3 — patched.** `verify(bytes, pubkey, class, version)` refuses, typed (`sidecar <field> is '<got>', expected '<want>'`, exit 1), a `schema` ≠ `BENCHMARK_SCHEMA`, a `class` ≠ the requested class or a `spirit_version` ≠ the resolved version. Test covers all three.
- **BH-4 — patched.** `key_from_env`: only `VarError::NotPresent` is "unset"; a non-UTF-8 value refuses as malformed. Test: a non-UTF-8 `MAOS_EVAL_PRODUCER_PUBKEY` exits 1.
- **BH-5 — patched.** A replay without `--cassette` reads `MAOS_REPLAY_CASSETTE` (ADR-064; `parse_args(argv, mode, env_cassette)`); under R2 such a run is an override unless it names the committed path. Tests: the env drift-blind cassette is scored (0.80, exit 1) and writes no sidecar; `./<committed path>` via the env publishes.
- **BH-6 — deferred** → `19-1-orchestrator-dispatch-loop-and-epic-input` (cwd-relative corpus/cassette paths; 19-1 AC5 edits the same class table). Not ruled; recorded as a decision for the operator.
- **BH-7 / AA-4 — ruled R1.**
- **EC-4 — patched.** `score_class` refuses `rows ≠ distinct verdicts` (`3 rows, 2 verdicts — a duplicate scenario_id`). Test in `eval_halt_18_2`.
- **EC-5 — patched per R2.** **EC-6 — ruled R3**, promotion re-pin deferred → `ops-nightly-cassette-rerecord`.
- **AA-1 — patched.** `HaltClass.calendar_conflict_basis` / `BenchmarkBody.halt_recall_calendar_conflict_basis`: Butler `"arithmetic: belief_variance (18-1 AC1.1)"`, Researcher `null`; asserted in the exit-line-2 witness (and so in the sidecar body, which the witness compares to the line).
- **AA-5 — ruled R4**, deferred → `ops-nightly-cassette-rerecord`. **AA-6 — ruled R5**, patched. **AA-7 — patched** (§Proven-red controls, last paragraph).
- **TIA-1 — ruled R3** (nightly path; `ci_shape_18_0` asserts the `.live.json` target), deferred row shared with EC-6. **TIA-2 — ruled R4** (row shared with AA-5).
- **TIA-3 — patched.** The exit-line-2 witness verifies with `derive_pubkey(&DEV_SEED)`, not the ambient env; the in-process record test removes `MAOS_EVAL_PRODUCER_SEED`/`_PUBKEY` before `evaluate`. **TIA-4 — patched.** `eval()` sets `MAOS_SECRETS_BACKEND=env`.
- **TIA-5 — open, the integrator's** (remote CI proven-red run of `ci/plant.diff`, which still applies unchanged, sha256 `f3255f45…edb2`).
- **TIA-6 — deferred** → `18-3-researcher-replay-live-and-judged-five-metric` (a `maosctl spirit benchmark` step in `epic-18-exit`; 18-3 AC1 adds exit line 3 to that job).

**Plants (review pass):** 7 new (BH-1, BH-2, BH-3, BH-4, R2, BH-5, EC-4), all red then restored (copy-back + `cmp` + `touch`) and green; the dev AC5 plant was regenerated (the old diff no longer applied once `verify` changed) and re-proven red (4 tests). Table and wall-clock: `18-2-evidence/plants.md` §Review plants; diffs `18-2-evidence/review-*-plant.diff`, `ac5-plant.diff`. All 14 diffs `git apply --check` clean.

**Focused suites (local, `env -u CI -u GITHUB_ACTIONS`, one cargo process, `CARGO_TARGET_DIR=/mnt/build/cargo`, final tree):** `-p maos-eval -p maos-bench -p maos-registry` 202/202, 4 skipped (6.3 s); `-p maos-cli --test benchmark_sidecar_18_2` 10/10 (nextest 0.8 s; `cargo test -p maos-cli` alone 10/10); `-p maos-bin --test eval_halt_18_2 --test verb_table_15_3 --test cohort_daemon_smoke_13_5c` 28/28, 1 skipped (3.4 s; eval_halt 12/12); `-p butler --test halt_eval_corpus_18_2 --test corpus_pin --test corpus_halt` 13/13, 1 skipped (9.5 s); `-p researcher --test halt_corpus_18_2` 4/4 (9.1 s); `-p maos-journey-test --test mcp_conformance_18_8` 15/15 (7.2 s); xtask `d11_xtask_ceiling_ratchet`, `recovery_lane_ceiling_rule`, `retro_action_items`, `ci_shape_18_0` (journey-nightly reader) + readers of the changed MANIFEST / coverage matrix / epics / deferred-work (`corpus_staleness_integration`, `coverage_matrix_integration`, `coverage_matrix_completeness_tests`, `d19_story_file_governance`, `owner_assertion_epic_key_rejected`) 65/65 (0.9 s).

**Gates (prebuilt `xtask`, `maos`, `maosctl`):** `cargo fmt --all -- --check` clean; `kloc-check` PASSED (aggregate 183247); `check-kernel-baseline` PASSED (26377 lines, 95 files); `check-env-contract` PASS (108 vars, 0 violations; no new env var); `check-exit-commands` PASS (4 epics, 12 tokens resolved, 3 owed — all 20-1); `check-service-boundary` PASSED (0 violations); `check-test-cost` PASS (534 test files); `check-corpus` PASSED (12 entries); `coverage-matrix` exit 0 (NFR-Test-4 deferred at v0.5, no violation); `check-serde-error-handling` PASSED (0 violations); `check-dev-record-completeness` PASSED (178 done stories); gates wall-clock 2.4 s total (each ≤ 1.2 s). No new maos-bin src file (`SCANNED_SOURCE_FILES` unchanged). `intent-lineage-coverage-report.md` not modified.

**Figures (kloc-check --json, tokei 14.0.0, after `cargo fmt --all`; HISTORY notes cite `18-2-eval-halt-verb-and-corpus-numbers`):** maos-bin 28087 → **28094** (+7, ≤ 28140 pre-ratified); maos-bench 2018 → **2032** (+14 over the R1-ratified 2018, within +30); maos-eval 4242 → **4271** (+29 over the R1-ratified 4242, within +30); maos-cli 6104 / 6856 (+7, no raise); maos-registry 3452 / 4282 (no change); maos-journey-test 915 (+10, the integrator's 18-8 fix); kernel 20608 / 26377 unchanged; `_aggregate_hardfail` 183181 → **183248** (measured 183247 + 1).

**Separate (not an 18-2 finding):** the integrator's 18-8 landing fix is in the tree — `crates/maos-journey-test/src/lib.rs` `McpRequestCapture::json` match and `sse_frame` returning `Result` with a 500 fallback, for `check-serde-error-handling` (which 18-8 redded in CI); its +10 lines are inside the measured aggregate above. `18-8-evidence/ci/run-40191749007-excerpt.log` left untouched.

### File List

- Modified: `.github/workflows/discipline.yml` (`epic-18-exit`: exit line 2 plus the drift-blind twin, sidecar artifact)
- Modified: `.github/workflows/journey-nightly.yml` (`tier-2-rerecord` eval record step; upload `!cancelled()`)
- Modified: `Cargo.lock` (new dev/prod edges only)
- Modified: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Modified: `_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md`, `_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md` (creation corrections and the received 19-1 AC5)
- Modified: `crates/maos-bench/Cargo.toml`, `crates/maos-bench/src/lib.rs`
- New: `crates/maos-bench/src/halt_eval.rs`
- Modified: `crates/maos-bin/Cargo.toml`, `crates/maos-bin/src/env_contract.rs`, `crates/maos-bin/src/main.rs`, `crates/maos-bin/src/verbs.rs`, `crates/maos-bin/tests/verb_table_15_3.rs`
- New: `crates/maos-bin/tests/eval_halt_18_2.rs`
- Modified: `crates/maos-cli/Cargo.toml`, `crates/maos-cli/src/cli.rs`, `crates/maos-cli/src/subcommands.rs`
- New: `crates/maos-cli/tests/benchmark_sidecar_18_2.rs`
- Modified: `crates/maos-eval/Cargo.toml`, `crates/maos-eval/src/lib.rs`, `crates/maos-eval/src/onboarding_gate_corpus.rs`
- New: `crates/maos-eval/src/halt_replay.rs`
- New: `crates/maos-journey-test/cassettes/j-butler/halt-corpus-v0.3.json`
- Modified: `crates/maos-registry/src/storage.rs`
- Modified: `spirits/butler/Cargo.toml`, `spirits/butler/tests/corpus_pin.rs`, `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl`
- New: `spirits/butler/tests/halt_eval_corpus_18_2.rs`, `spirits/butler/tests/fixtures/halt-corpus-v0.3.drift-blind.json`, `spirits/butler/tests/fixtures/halt-corpus-v0.3.precision-planted.json`
- Modified: `spirits/researcher/Cargo.toml`
- New: `spirits/researcher/tests/halt_corpus_18_2.rs`, `spirits/researcher/tests/fixtures/halt-corpus-v0.5.jsonl`
- New: `tests/harness/fr15_resolution.rs`
- Modified: `tests/coverage-matrix.yaml` (NFR-Onb-1 SHA re-pin; NFR-Test-4 row)
- Modified: `xtask/gate-registry.toml` (`eval-halt`), `xtask/kloc.toml` (re-pins), `xtask/tests/ci_shape_18_0.rs` (exit line 2 shape test)
- New: `_bmad-output/implementation-artifacts/18-2-evidence/{plants.md,ac1a-plant.diff,ac1b-plant.diff,ac2-plant.diff,ac3-plant.diff,ac4-plant.diff,ac5-plant.diff,ci/plant.diff,ci/plant.diff.sha256}`
- Review (2026-10-25): modified `crates/maos-eval/Cargo.toml` (serde_json `raw_value`), `crates/maos-eval/src/halt_replay.rs`, `crates/maos-bench/src/halt_eval.rs`, `crates/maos-bin/src/main.rs`, `crates/maos-cli/src/subcommands.rs`, `crates/maos-bin/tests/eval_halt_18_2.rs`, `crates/maos-cli/tests/benchmark_sidecar_18_2.rs`, `.github/workflows/journey-nightly.yml` (`.live.json` target, promotion comment), `xtask/tests/ci_shape_18_0.rs`, `xtask/kloc.toml`, `tests/corpora/MANIFEST.toml`, `tests/coverage-matrix.yaml` (NFR-Test-4 `corpora:`), `_bmad-output/implementation-artifacts/deferred-work.md`, both epics; new symlinks `tests/corpora/butler-calendar-comms-v0.3.jsonl`, `tests/corpora/researcher-halt-corpus-v0.5.jsonl`; new `18-2-evidence/review-{bh1,bh2,bh3,bh4,bh5,ec4,r2}-plant.diff`, regenerated `18-2-evidence/ac5-plant.diff`, `plants.md` §Review plants.

### Change Log

- 2026-10-25 — Story created (`backlog`): §Current-tree corrections applied in place in epic §18-2, §R R18-11/R18-12, the exit-block provenance (`:23`, `:29`), the stories table (`:91`), kloc asks (`:36`) and Kernel-Δ (`:32`). D-18-2-01…06 PROPOSED; D-18-2-07 standing. Kernel-Δ 0 measured by prototype; maos-bin +50 (P-B) / +177 (P-A) measured.
- 2026-10-25 — RULED (operator, Lunarpulse): D-18-2-01 **B**, 02 **B** (drift-blind cassette = negative control and plant, 12/15 = 0.80), 03 **A**, 04 **A**, 05 **A** (deviates from the recommendation B: Researcher class here, labelled not-model-driven; Orchestrator received by `epic-19-founder-loop-w4.md` §19-1 AC5 with the D-E tension recorded — no dependency on 18-2), 06 **A**. maos-bin pre-ratified ≤ 28140; Epic 18's named maos-bin budget +250 → +385. Review opus-5-5 only. ACs, CR-4/CR-5, tasks and the epic updated. Status `ready-for-dev`.
- 2026-10-25 — Dev (anthropic/claude-opus-5-5, passes Dev182 + Dev182b). T0–T10 done; AC1–AC5 green. Six plants proven red; the CI plant is AC1 (b). Five dev deviations are recorded in the Completion Notes. kloc:
  - maos-bin 28087 (≤ 28140);
  - maos-eval 4242 (measured request);
  - maos-bench 2018 (measured request over the forecast);
  - `_aggregate_hardfail` 183181;
  - kernel 0.

  Status `review`.
- 2026-10-25 — Code review (anthropic/claude-opus-5-5, 4 layers: BH, EC, AA, TIA) and review patch pass. Operator rulings D-18-2-R1…R5 recorded (§Review). Patched BH-1/EC-1 (HIGH), BH-2/EC-2/AA-2, BH-3/EC-3/AA-3, BH-4, BH-5, EC-4, EC-5 (R2), AA-1, AA-6 (R5), AA-7, TIA-3, TIA-4; R3 nightly `.live.json`; deferred BH-6 → 19-1, AA-5/TIA-2 and EC-6/TIA-1 promotion → `ops-nightly-cassette-rerecord`, TIA-6 → 18-3; TIA-5 stays the integrator's. 7 review plants + the regenerated AC5 plant proven red. kloc: maos-bin 28094, maos-bench 2032, maos-eval 4271, `_aggregate_hardfail` 183248, kernel 0. Status stays `review`.
- 2026-10-25 — CI: proven-red `40200067350` (`68aae558`); landing `40200028265` @ `ee395748`, `epic-18-exit` green. Status → `done`.
