# Epic 27 — Reasoning-Model Adapter

**Status:** `backlog` — filed 2026-11-14 for the two slices already implemented, so that they have live tracker keys. Design: `_bmad-output/planning-artifacts/research/reasoning-model-adapter-design-2026-11-13.md` (rev 7; slices S1–S9 in its §7; operator rulings Q1–Q7 in its §9, 2026-11-13). `sprint-status.yaml` is authoritative for status.

**Wave:** none yet; S1 and S2 are filed as stories `27-1` and `27-2`; S3a–S9 are **not filed** (operator: they become Epic 27 stories after `v1.0.0` GA; each is created later by `bmad-create-story` under the project's creation contract, and gets its key then). · **Makes true:** The model a manifest pins is the model the wire serves, and what a provider reports as spend is recorded as spend; later slices make model-specific reasoning behaviour data (family-first profile file) behind a pure resolver.

**Closes:** operator ruling Q6 (manifest pin authoritative: pin > provider default; design §3.3 gaps G9, G12) → `27-1`; NFR-Cost-1 (design §3.3 gaps G4, G5, §5.5) → `27-2`. `27-2` is a `v1.0.0` GA precondition (`sprint-change-proposal-2026-11-13.md` §4; the GA set of ten stories includes S2).

**Hermetic exit command (CI, no secrets; red until green):**

```
cargo test -p maos-bin --test manifest_model_pin_s1   # 1 — a manifest model pin reaches the wire and the transparency log on every admission and staging path (27-1)
cargo test -p maos-providers --test usage_s2          # 2 — Anthropic usage fails closed, Gemini counts thoughts, Ollama absent counters are 0 and present-invalid is refused (27-2)
```

**Exit-block provenance (measured at the worktree `b990bdc1` plus the uncommitted S1/S2 files):**
- Lines 1 and 2 — both test targets exist (`crates/maos-bin/tests/manifest_model_pin_s1.rs`, `crates/maos-providers/tests/usage_s2.rs`); no token is owed. Hosted proof of both is **pending**; the lines are the exit for the two filed stories only. Each later story (S3a–S9) adds its own line to this block when it is created.
- Shape: one `cargo test` line each with a trailing `#` comment; no secret, key, human, wall-clock or network (the tests use loopback or fake transports).

**Kernel-Δ:** `27-1` −1 line in `kernel-core` `inference/mod.rs` (kernel pin 26062 → 26061); `27-2` 0. Later slices forecast 0 (design §5.8; the `maos-providers` kloc raise needs a measured figure and an open story key).

**Confidence rules.** Rules 1–11 are those printed in `epic-21-multi-host-on-live-substrate-w6.md` under **Confidence rules** and **Confidence rule 11**; they bind here and are not repeated.

---

## Stories

| Key | Story | Closes · state |
|---|---|---|
| `27-1-manifest-model-pin-to-wire` | Manifest model pin reaches the wire on every admission and staging path (design S1) | Q6; G9, G12 · `review` (GPT review 2 rounds, ship; hosted proof pending) |
| `27-2-provider-usage-fixes` | Provider usage fixes (design S2) | NFR-Cost-1; G4, G5 · `review` (GPT review, ship; hosted proof pending); `v1.0.0` GA precondition |

Story files: `_bmad-output/implementation-artifacts/27-1-manifest-model-pin-to-wire.md` and `27-2-provider-usage-fixes.md` (retroactive records). Evidence (READMEs and GPT review rounds): local archive, not in git (ruling B7a) — `/home/lunarpulse/maos-evidence-archive/2026-11-14/s1-manifest-model-pin-evidence/` and `s2-usage-fixes-evidence/`.

### 27-1-manifest-model-pin-to-wire — Manifest model pin reaches the wire

*Closes · Δ:* operator ruling Q6 · design slice S1 · kernel-Δ −1 line.

- Carries the admitted `(provider, model)` from the manifest through admission and the binding into `options.model_id`; the manifest pin overrides the code defaults and the profile file; provider-only declarations stay provider-only; the judge keeps its separately admitted pin; the `InferenceCall` transparency-log intent records the served model.
- Implemented. Review: two GPT rounds, ship. Hosted proof: **pending**.

### 27-2-provider-usage-fixes — Provider usage fixes

*Closes · Δ:* NFR-Cost-1 · design slice S2 · kernel-Δ 0 · **`v1.0.0` GA precondition** (SCP 2026-11-13).

- Anthropic usage fails closed (no more 0-token record, no `as u32` truncation); Gemini `output_tokens` = `candidatesTokenCount` + `thoughtsTokenCount` with `checked_add`; Ollama absent `omitempty` counters are 0 and present-invalid values are refused; OpenAI unchanged.
- The `reasoning_tokens` breakdown is deferred to design slice S4, because `TokenUsage` is a public `maos-domain` type.
- Implemented. Review: GPT, ship. Hosted proof: **pending**.

## Unfiled scope (design §7; no keys until each is created after GA)

| Slice | Scope (design §7) |
|---|---|
| S3a | Family-first profile file; `default_model` moves from `main.rs` into `[providers.<p>]`; currency gate switched to `default_model` |
| S3b | Resolver for sampling and the Anthropic off switch (replaces `NO_SAMPLING`/`thinking_off`/`accepts_sampling`) |
| S3c | Fail closed for ids with no grammar match and no exact entry (Q1) |
| S4 | Intent/`Applied` contract, caller migration, empty-text `Incomplete` handling; the `reasoning_tokens` breakdown deferred from `27-2` |
| S5 | OpenAI Responses (`store:false`) and a Kimi profile |
| S6 | Cassette v2 (after S4) |
| S7 | Anthropic `2026-10-21` flip (Q2) |
| S8 | Continuations (first multi-turn/tool story) |
| S9 | Model-profile steward (outside the runtime; after S3a) |

**Left out:** runtime model discovery (design option C); a runtime `WireDialect` trait framework (option B′); streaming.

## Dependencies

- `27-1` and `27-2` have no dependency on Epics 22–26. S3a–S9 start after `v1.0.0` (SCP 2026-11-13 §4); S3b–S3c follow S3a, S6 follows S4, S9 follows S3a (design §7).
- `26-3` takes its spend figure from `27-2`, which lands before GA (`epic-26-worker-fleet-w11.md` Dependencies).
