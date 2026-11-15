---
story_id: '28.4'
story_key: 28-4-other-spirits-prompts-and-evals
created: 2026-11-15
context_commit: bf3cba25
baseline_commit: bf3cba25
security_sensitive: false
kernel_grant: "Not needed: kernel-Δ forecast 0 (method: read the Spirit sources listed below; the work lives under spirits/** and crates/maos-spirit-hello, none under crates/maos-kernel-core/src). Each per-Spirit landing measures with `check-kernel-baseline --json` / `kloc-check --json`; if the kernel moved, it re-pins with `scripts/repin.py` and writes the HISTORY row in the same commit."
kloc_grant: "Forecast 0 charged for the orchestrator (spirits/ excluded, kloc_check.rs:303-304; 28-1's tooling is reused unchanged). hello-spirit lives in crates/maos-spirit-hello (row 482, kloc.toml:434): a prompt file there is +5…15 [INFERENCE]; re-priced when this story leaves backlog."
review: "Re-created by bmad-create-story before it leaves backlog; this file is the inventory and order only. Creation anthropic/claude-opus-5-5."
---

# Story 28.4: The other Spirits — prompts and evaluations

Status: backlog

<!-- Created 2026-11-15 at bf3cba25 by bmad-create-story as a backlog inventory (operator scope: one story for observer, mira, nash, orchestrator, worker, architect, reviewer, hello-spirit; not GA; no further keys). Re-measured at bf3cba25. D-28-4-01…05 PROPOSED. Before it leaves backlog the story is re-created with ACs per Spirit under the ruled 28-1 protocol, and its exit lines are added to the Epic 28 block. -->

**Not a GA precondition** (ruling 2026-11-15: GA needs 28-1 and 28-2). Starts after 28-2 has proven the method on one class.

## Story

As **the operator who ships every reference Spirit**,
I want **each model-driven Spirit's prompt context-engineered and each Spirit's behaviour scored by the 28-1 method**,
so that **no Spirit ships a prompt or a floor that was never measured**.

## Inventory at `bf3cba25` (objective, tests, prompt)

| Spirit | Objective (source) | Model-driven? | Prompt today | Tests and corpora today | 28-1 method applies as |
|---|---|---|---|---|---|
| orchestrator 0.8.0 | judges whether a story section is too ambiguous to dispatch; halts above 0.5 (`spirits/orchestrator/manifest.toml:51-53`) | **yes** — `provider.complete` claude-haiku-5-5, gpt-4o-mini | `ambiguity_prompt` one `format!` string, JSON `{"dispatch_ambiguity": number}` (`spirits/orchestrator/src/lib.rs:185-195`), strict parse (`:203-228`) | `halt_corpus_19_1.rs`, `judgement.rs`, `epic_split.rs`, `followup.rs`, `distillate_dispatch.rs`, `orchestrator_buffer.rs`; corpus `halt-corpus-v0.1.jsonl` 30 rows, 15 expected halts (`tests/corpora/MANIFEST.toml:161`); `maos eval halt --class orchestrator` on a **seed** cassette `j1/orchestrator-halt-corpus-v0.1.json`; floors recall 0.70, precision 0.85 (`crates/maos-bench/src/halt_eval.rs:133-146`) | full: versioned prompt, train split, k live repeats, calibrated floors, promoted live cassette |
| hello-spirit 0.1.0 | introduces itself with its capability scope, halt tags and transparency-log endpoint (J0 evaluator path) | **yes** — five provider pins (`spirits/hello-spirit/manifest.toml`) | literal string (`crates/maos-spirit-hello/src/lib.rs:84-91`), free text, `max_tokens` 256, no schema | unit tests in `crates/maos-spirit-hello`; cassettes `j0/shell-intro.json`, `providers/{gemini,kimi}-shell-intro.json`, all with the wildcard hash | D-28-4-05 |
| worker 0.8 | CliWrapperSpirit: wraps an external CLI (`worker-cli-fixture`; `manifest-claude.toml:28` `claude`, `manifest-codex.toml`) | no Inference Port in the crate (0 `InferencePort` refs in `spirits/worker/src`) | none of its own; the task text goes to the wrapped CLI | `cli_wrapper_admission.rs`, `fixture_modes_16_3.rs`, `fixtures_pin.rs`; `canned-cli-output.json` (3 lines) | D-28-4-03 |
| observer 0.5.0 | read-only watchdog over `scalar.tap` writes; drift and structural anomalies (`spirits/observer/src/lib.rs:3-10`) | no (no `provider.complete`; 0 Inference Port refs) | none | `drift_watchdog.rs`, `structural_anomaly.rs`, `scalar_tap_subscribe.rs`, `activity_hooks_17_4.rs`; fixtures 4 drift + 5 structural scenarios | D-28-4-02 |
| mira 1.5.0 | prod-edge diagnostic, Host A of the bilateral pair (`spirits/mira/src/lib.rs:3-7`) | no | none | `a2a_pairing.rs`, `halt_bilateral.rs`; `diagnostic-scenarios.json` (2) | D-28-4-02 |
| nash 1.5.0 | dev-environment Senior Architect, Host B; consumes Mira's advisory (`spirits/nash/src/lib.rs:3-14`) | no | none | `fixtures_pin.rs`, `spirit_smoke.rs`; `architect-scenarios.json` (2) | D-28-4-02 |
| architect 0.8.0 | proposes a design from a task spec; deterministic, seeded, bit-identical; live generation is application-layer (`spirits/architect/src/lib.rs:1-7`) | no | none | `code_review_loop.rs`; `design-specs.json` (4 cases) | D-28-4-02 |
| reviewer 0.8.0 | critiques the Architect's proposal; deterministic, bit-identical (`spirits/reviewer/src/lib.rs:1-7`) | no | none | `review-cases.json` (3 cases) | D-28-4-02 |

Also present and not in the operator's list: `spirits/digest` (read-only J3 team Digest Spirit; no `provider.complete`; `five_metric_digest_18_3.rs`, `hallucination_18_1.rs`) → D-28-4-04.

## Kernel and kloc pricing — forecast at creation

**Kernel:** forecast **0** (method: read the sources above). **kloc:** see `kloc_grant`; re-priced at re-creation.

## Decisions — ACCEPTED (operator: proceed with recommendation, 2026-11-15)

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-28-4-01** | Order | **A** orchestrator → hello-spirit → the non-model Spirits as ruled in D-28-4-02 · **B** hello-spirit first (J0 evaluator path) · **C** all in one landing | **A.** The orchestrator is the only other model-driven halt class and already has a 30-row corpus, a class floor and `maos eval halt` (`halt_eval.rs:133-146`); its cassette is still a seed. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-4-02** | Spirits with no Inference Port (observer, mira, nash, architect, reviewer) | **A** evaluation only: name families in their existing fixtures and keep the deterministic tests; no prompt, no live runs · **B** out of scope until a Spirit gains a model call (record it here and stop) · **C** apply the full 28-1 method | **B.** Their cognition is deterministic and bit-identical (architect/reviewer docs; no `provider.complete`); k repeats and intervals measure nothing there. A is cheap if the operator wants family labels for later. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-4-03** | Worker | **A** not this method: the wrapped CLI's quality is measured by the Worker-fleet journeys (Epic 26) · **B** version the task-brief template the Orchestrator sends and evaluate it with 28-1 | **A.** The Worker has no prompt of its own; the brief is the Orchestrator's dispatch text. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-4-04** | `digest` (not listed by the operator) | **A** inventory only, same treatment as D-28-4-02 · **B** omit | **A.** Completeness of the inventory; no key is added. | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |
| **D-28-4-05** | hello-spirit | **A** a versioned prompt file and a small golden corpus (one row per pinned provider) asserting the intro names scope, halt tags and the TL endpoint; replay on hash-bound (non-wildcard) cassettes · **B** leave as a smoke test | **A.** It is the first model output an evaluator sees (J0), and its three cassettes use the wildcard hash, which strict replay does not check (`crates/maos-bin/src/cassette_replay.rs:151`). | ACCEPTED (operator: proceed with recommendation, 2026-11-15) |

## Acceptance Criteria

Written when the story is re-created to leave `backlog`: one AC group per Spirit in the ruled order, each with its exit line (a `cargo test` baseline target and, for a model-driven class, `MAOS_INFERENCE_MODE=replay maos eval …`).

## Tasks / Subtasks

- [ ] Rulings D-28-4-01…05 recorded; 28-2 done.
- [ ] Re-create this story with per-Spirit ACs under the ruled 28-1 protocol; add its exit lines to the Epic 28 block.

## Dev Agent Record

### Agent Model Used

<set by dev at story start>

### Completion Notes List

### File List

## Change Log

- 2026-11-15: created as a backlog inventory (bmad-create-story, anthropic/claude-opus-5-5) at `bf3cba25`; D-28-4-01…05 PROPOSED.
- 2026-11-15: D-28-4-01…05 ACCEPTED (operator: proceed with recommendation). Statuses in the Decisions table above.
