---
story_key: 27-2-provider-usage-fixes
status: review
updated: 2026-11-14
dev_model_used: anthropic/claude-sonnet-5-5
baseline_commit: b990bdc1
kernel_grant: "NONE. The change is outside `maos-kernel-core`; no re-pin applies."
review: "Non-author cross-family review (§A6 sense: author and reviewer are different model families): openai-codex/gpt-6.1-sol, finding round plus closure round (one P2, closed, verdict ship). The three-layer Blind Hunter / Edge Case Hunter / Acceptance Auditor net was NOT run."
---

# 27-2 — Provider usage fixes (NFR-Cost-1)

Status: review

> Retroactive record. The work was implemented and reviewed before this file existed; the key was filed afterwards (2026-11-14). Facts are copied from the evidence, which lives in a **local archive, not in git** (ruling B7a): `/home/lunarpulse/maos-evidence-archive/2026-11-14/s2-usage-fixes-evidence/` (`README.md`, `gpt-review.md`, `gpt-review-2.md`).

**`v1.0.0` GA precondition** (`sprint-change-proposal-2026-11-13.md`: the GA set includes design slice S2). **Design source:** `_bmad-output/planning-artifacts/research/reasoning-model-adapter-design-2026-11-13.md` §3.3 gaps G4/G5, §5.5, §7 slice S2. Epic: `_bmad-output/planning-artifacts/epics/epic-27-reasoning-model-adapter.md`.

## Story

As **an operator who is billed by token**,
I want **a provider reply whose usage cannot be accounted to be refused, and Gemini thinking tokens to be counted as billed output**,
so that **spend is never recorded as 0 tokens and never under-reported (NFR-Cost-1).**

## Acceptance Criteria

1. Anthropic usage fails closed: a missing usage object or field, a non-integer value, or a value above `u32::MAX` is `ProviderError::Serde`, never `Ok` with 0 and never truncated (`usage_count`, `crates/maos-providers/src/provider.rs`). `output_tokens` stays the inclusive billed total.
2. Gemini `output_tokens` = `candidatesTokenCount` + `thoughtsTokenCount`, added with `checked_add`; `thoughtsTokenCount` is 0 only when the field is absent; a present invalid value, a missing `candidatesTokenCount`, or an overflow is refused.
3. Ollama: an absent counter is 0 (vendor `omitempty`; ollama issue #3427) via `ollama_count`; a present invalid counter is refused.
4. OpenAI is unchanged (already fail-closed; `completion_tokens` is inclusive of reasoning tokens).
5. The `reasoning_tokens` breakdown is **deferred to design slice S4**, because `TokenUsage` is a public `maos-domain` type.

## Tasks / Subtasks

- [x] Anthropic fail-closed usage (`crates/maos-providers/src/anthropic.rs`).
- [x] Gemini `billed_output_tokens` (`crates/maos-providers/src/gemini.rs`).
- [x] Ollama `ollama_count` and its unit test (`crates/maos-providers/src/ollama.rs`); review round 1 P2 fixed.
- [x] `crates/maos-providers/tests/usage_s2.rs` (fake-transport tests, no socket).
- [x] Orchestrator verification: `maos-providers` 114/114, consumer crates green, `cargo fmt` clean.
- [ ] Hosted proof (discipline run on the landing PR).

## Review Findings

Reviewer: `openai-codex/gpt-6.1-sol` (read-only; no checks rerun). Implementer `anthropic/claude-sonnet-5-5` — a different model family.

| # | ID | Finding | Status | Resolution |
|---|---|---|---|---|
| 1 | R1-P2 | Round 1, P2 (confidence 0.95): requiring both Ollama counters rejects otherwise successful non-streaming `/api/chat` replies when Ollama omits a zero counter (`omitempty`; issue #3427), turning them into terminal `MalformedResponse` errors. | closed | Fixed in `crates/maos-providers/src/ollama.rs` (`ollama_count`: absent counter → 0; present value → `usage_count`, so null, string, float, negative and `> u32::MAX` are still refused) and `crates/maos-providers/tests/usage_s2.rs` (`ollama_absent_counters_are_zero_omitempty`, `ollama_present_invalid_counts_are_a_typed_error_never_zero`). Closure round: P2 closed, no new findings. |

Counts after closure: P0 0 · P1 0 · P2 0 · P3 0. Round 1 verdict: no-ship as submitted. **Closure verdict: ship S2.** Reviewer conclusions recorded: Anthropic and Gemini changes correct; `ProviderError::Serde` terminal routing (no fallback) is intended, but a billed malformed-usage call emits no token cost frame, so this does not establish complete billing reconciliation; cassette replay (`cassette_replay.rs:87–95`) keeps a pre-existing silent-zero path outside S2; deferral of the reasoning breakdown to S4 is appropriate.

## Dev Agent Record

### Agent Model Used

Model: anthropic/claude-sonnet-5-5 (agent `coder`, effort high). Reviewer: openai-codex/gpt-6.1-sol (different family).

### Completion Notes List

- Anthropic, Gemini and Ollama usage handling changed as in the acceptance criteria; OpenAI covered by a regression test only.
- `usage_s2` tests: Anthropic (normal, inclusive total next to the thinking breakdown, missing, non-integer, above `u32`), Gemini (thoughts counted, no thoughts field, zero thoughts, missing candidates, present-invalid thoughts, overflow refused and `u32::MAX - 1 + 1` accepted), OpenAI (inclusive total, missing or invalid), Ollama (normal, absent counters, present-invalid).
- Verified by the orchestrator: `maos-providers` 114/114, consumer crates green, fmt clean (as reported in the closure review and the handoff). No `xtask/kloc.toml` or `xtask/kernel-core-baseline.toml` change for this story.
- Hosted proof pending.

### File List

- crates/maos-providers/src/anthropic.rs (modified)
- crates/maos-providers/src/gemini.rs (modified)
- crates/maos-providers/src/ollama.rs (modified)
- crates/maos-providers/tests/usage_s2.rs (new)
- _bmad-output/implementation-artifacts/27-2-provider-usage-fixes.md (new; this file)

The change landed in the same working tree as other slices (commit `b990bdc1` is a combined batch; `provider.rs` `usage_count` was already present at HEAD for OpenAI and Gemini); the list is the S2-attributable files per the evidence README.

### Change Log

- 2026-11-13 — implemented and reviewed (GPT round 1: one P2 on Ollama; closure: P2 closed, ship).
- 2026-11-14 — story key filed in Epic 27; status `review`; GA precondition; hosted proof pending.
