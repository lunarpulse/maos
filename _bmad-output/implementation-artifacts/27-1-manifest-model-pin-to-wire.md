---
story_key: 27-1-manifest-model-pin-to-wire
status: review
updated: 2026-11-14
dev_model_used: anthropic/claude-sonnet-5-5
baseline_commit: b990bdc1
kernel_grant: "kernel-core `inference/mod.rs` -1 line; kernel pin 26062 -> 26061, `set_hash` 97af61f5e61077fc1be810f9bb0786115dbccb29cb4e571e3ecc6e4abc847a74; `scripts/repin.py --apply` run against this key (27-1)."
review: "Non-author cross-family review (§A6 sense: author and reviewer are different model families): openai-codex/gpt-6.1-sol, two rounds (round 1 one P2, round 2 closed, verdict ship). The three-layer Blind Hunter / Edge Case Hunter / Acceptance Auditor net was NOT run."
---

# 27-1 — Manifest model pin reaches the wire

Status: review

> Retroactive record. The work was implemented and reviewed before this file existed; the story key was filed afterwards (2026-11-14) so the change has a tracker key. Facts below are copied from the implementer's and reviewer's evidence, which lives in a **local archive, not in git** (ruling B7a): `/home/lunarpulse/maos-evidence-archive/2026-11-14/s1-manifest-model-pin-evidence/` (`README.md`, `gpt-review.md`, `gpt-review-2.md`).

**Design source:** `_bmad-output/planning-artifacts/research/reasoning-model-adapter-design-2026-11-13.md` §7 slice S1, operator ruling Q6 (§9). Epic: `_bmad-output/planning-artifacts/epics/epic-27-reasoning-model-adapter.md`.

> Operator, verbatim (2026-11-13): "q6 manifest pin overrides the code base or profile file."

## Story

As **the operator who pins a model in a Spirit manifest**,
I want **the admitted `(provider, model)` pin to be the model the wire serves, on every path that admits or stages a manifest**,
so that **the model I declared is the model that is called and billed, and a retired pin is refused before it can fail every live call.**

## Acceptance Criteria

1. The manifest pin is carried through admission and the binding into `options.model_id`, with precedence **pin > provider default**; a provider-only declaration stays provider-only (the driver default serves).
2. Several admitted models per provider are served (the Haiku default and the separately admitted Sonnet judge pin coexist); an unadmitted model gets no transport call.
3. The `InferenceCall` transparency-log intent records the served model, not the literal `default`.
4. A manifest pin that is retired or not in the currency allowlist is refused `model_not_admitted` on **every** admission and staging path (round-1 review finding P2-1), not only `load_admit_start`.
5. Gemini keeps its configured-model equality restriction.
6. Kernel-core changes only the served-model attribution (−1 line), with the authorized re-pin.

## Result (facts from the evidence)

- Researcher is now served `claude-sonnet-5-5` as pinned (before: the Anthropic driver default, Haiku); Butler and Orchestrator are served `claude-haiku-5-5`; a provider-only Orchestrator is served the driver default.
- One validation (`admission::check_pins` = `check_provider_pins` + `check_declared_pins`) runs before admission or successor commit on every path: `load_admit_start` (standalone, topology, door load, installed Spirits, cold-swap admission and restoration), shell Hello and one-shot Hello, control-Spirit admission, and the hot-swap successor factory on the door-staged snapshot.
- Pins are provider-scoped, not per-Spirit model isolation (reviewer conclusion; candidate D-row for design slice S4).
- Hosted proof of the change: **PENDING** (the landing PR's discipline run).

## Tasks / Subtasks

- [x] Carry the pin from manifest to `options.model_id` (`maos-manifest`, `admission`, `researcher_root`, `orchestrator_root`, `shell_host` `BoundInference`, Butler seam).
- [x] Provider drivers serve any admitted model of their provider; `AdmittedModels` gate.
- [x] Real-binary and loopback tests (below).
- [x] Review round 1 P2 fixed (shared pin check on every path).
- [x] Kernel baseline re-pin against this key.
- [ ] Hosted proof (discipline run on the landing PR).

## Review Findings

Reviewer: `openai-codex/gpt-6.1-sol` (read-only; no builds or paid calls). Implementer `anthropic/claude-sonnet-5-5` (agent `coder`, effort high) — a different model family.

| # | ID | Finding | Status | Resolution |
|---|---|---|---|---|
| 1 | R1-P2 | Round 1, P2 (confidence 0.97): the new pin check ran only in `load_admit_start`; Hello's shell/one-shot paths call `SecurityManagerAdapter::admit_spirit` directly and hot-swap successors pass `gate_staged` and the successor factory without it, so a successor pinning retired `claude-3-haiku-20240307` could replace a working Researcher and then fail every live call; explicit replay accepted the same pin. | closed | Fixed by one shared check on every path: `crates/maos-bin/src/admission.rs` (`check_pins`, `check_gated_pins`), `crates/maos-bin/src/main.rs` (shell/one-shot Hello, control Spirit, staged-successor factory), `crates/maos-bin/src/shell_host.rs`. Real-binary tests in `crates/maos-bin/tests/one_daemon_one_door_16_1.rs` (hot swap refused with the predecessor still serving; cold swap reverted; replay run and load) and `crates/maos-bin/tests/one_shot_hello.rs` (one-shot Hello). Round 2 verdict: P2 closed, no new findings. |

Counts after round 2: P0 0 · P1 0 · P2 0 · P3 0. Round 1 verdict: no-ship pending the admission-path fix. **Round 2 verdict: ship.** Round 2 noted two acknowledged, non-blocking items: the shell-Hello call has function-level but no real-binary regression test (the shell admits the compile-time embedded manifest), and cold swap refuses after the predecessor unload (documented design). The kernel re-pin and ceiling authorizations remain separate landing prerequisites.

## Dev Agent Record

### Agent Model Used

Model: anthropic/claude-sonnet-5-5 (agent `coder`, effort high). Reviewer: openai-codex/gpt-6.1-sol (different family).

### Completion Notes List

- Pin precedence, multi-pin admission and served-model attribution implemented as in the design's S1 slice; the judge keeps its separately admitted Sonnet pin.
- New tests: `crates/maos-bin/tests/manifest_model_pin_s1.rs` (6; real bindings, kernel dispatch, real Anthropic driver over the real IO adapter against a loopback server that records request bodies), `crates/maos-providers/tests/admitted_models_s1.rs` (6), a `maos-manifest` unit test for the provider-pin split, and the round-1 real-binary tests named above. Proven red: replacing `.with_model(model)` in `researcher_root::bind` with `.with_model(None…)` failed the Researcher test; restored.
- Orchestrator verification: see the evidence archive README §4–§5 (gates, kloc deltas). `xtask check-kernel-baseline` failed as expected until the −1 re-pin (26062 → 26061), applied against this key.
- Open risks recorded in the archive README §6: Researcher cost rises (Haiku 5.5 → Sonnet 5.5, design §8); allowlisted is not wire-compatible (design slice S3 owns profiles); third-party manifests with a retired pin are now refused; several pins for one provider: first declared wins.
- Hosted proof pending.

### File List

- crates/maos-bin/src/admission.rs (modified)
- crates/maos-bin/src/main.rs (modified)
- crates/maos-bin/src/orchestrator_root.rs (modified)
- crates/maos-bin/src/researcher_root.rs (modified)
- crates/maos-bin/src/shell_host.rs (modified)
- crates/maos-manifest/src/manifest.rs (modified)
- crates/maos-manifest/src/lib.rs (modified)
- crates/maos-providers/src/admitted.rs (modified)
- crates/maos-kernel-core/src/inference/mod.rs (modified; −1 line)
- xtask/kernel-core-baseline.toml (modified; re-pin)
- crates/maos-bin/tests/manifest_model_pin_s1.rs (new)
- crates/maos-providers/tests/admitted_models_s1.rs (new)
- crates/maos-bin/tests/one_daemon_one_door_16_1.rs (modified)
- crates/maos-bin/tests/one_shot_hello.rs (modified)
- _bmad-output/implementation-artifacts/27-1-manifest-model-pin-to-wire.md (new; this file)

The change landed in the same working tree as other slices (commit `b990bdc1` is a combined batch); the list is the S1-attributable files per the evidence README.

### Change Log

- 2026-11-13 — implemented and reviewed (GPT round 1: one P2; round 2: closed, ship).
- 2026-11-14 — story key filed in Epic 27; status `review`; hosted proof pending.
