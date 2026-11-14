# Reasoning-model adapter — investigation and design (2026-11-13, rev. 7)

**Scope.** Investigation and design only.

**Sources.**
- Code: the worktree `/home/lunarpulse/dev_ws-posttag` (branch `posttag-2026-11-12`, base `1a57035e` plus the uncommitted stop-gap). Every `file:line` cite is relative to that root.
- Vendor pages: all accessed **2026-11-13**. Anything not read from a primary page is tagged **[INFERENCE]**.
- No paid API calls and no builds were made.

**Rev. 2** applies the cross-family review (`…-evidence/gpt-review.md`); **rev. 3** applies the closure review (`…-evidence/gpt-review-2.md`); **rev. 4** applies `…-evidence/gpt-review-3.md` (caller handling). **Rev. 5** applies the operator rulings of 2026-11-13: family-first profiles, AI-agent stewardship, Q1–Q7 ruled. **Rev. 6** applies `…-evidence/gpt-review-5.md` (§5.4/§5.11 mechanics). The disposition is in §10.

## 1. Executive summary

**Vendor behaviour.** Current Claude 5.x, OpenAI GPT-5.6/6.x and Kimi K3.1 Pro models, and most current Gemini models, think by default (`gemini-2.5-flash-lite` is an exception: off by default). Thinking is billed as output and counts against the output cap. Reasoning state differs in kind:
- Claude, OpenAI Responses and Gemini return opaque signed or encrypted artefacts.
- Kimi needs the full assistant message back, with readable `reasoning_content`.
- Ollama returns readable thinking and documents no continuation requirement.

Sampling is rejected per model today, and on every model under Anthropic `2026-10-21`.

**MAOS today.** Model rules are code branches (the Anthropic stop-gap). Neither the request nor the response can express reasoning intent or what was applied. Gemini under-counts billed output; Anthropic reads missing usage as zero; cassettes record no request options. The OpenAI driver's reasoning-model incompatibility is a **latent** selection-path defect: today it serves only `gpt-4o-mini` and sends `temperature` only when one is given.

**Most urgent finding (correctness, pre-existing on `main`).** A manifest's model pin never reaches the wire. The Researcher declares `claude-sonnet-5-5` but is served by the driver default, `claude-haiku-5-5` in this worktree and `claude-haiku-4-5-20251001` at `1a57035e`.

**Recommendation (operator-ruled 2026-11-13).** Model *capability facts* live in a reviewed data file, `xtask/model-profiles.toml`; *protocol implementation* stays in code.
1. Profiles are **family-first**: vendor defaults → model family → version entries only where facts change → optional per-model detail. Each family declares an id grammar, so a new id inherits from the nearest declared version at or below it. `claude-opus-6-0` needs **zero** profile edits unless a breaking change is found (§5.4).
2. One pure resolver turns provider-neutral intent plus the resolved profile into a wire plan and an `Applied` record. The record includes `profile_source` (`declared:` or `inherited:`).
3. The **existing** per-vendor drivers build and parse from that plan. There is no second dialect framework.
4. The **manifest pin is authoritative** for the served model; the provider default model moves out of code into the profile file (Q6).
5. An AI-agent **model-profile steward** keeps the file current outside the runtime and escalates to the operator only on user-visible behaviour or cost changes (§5.11).

An id is refused before any network call when it matches **no family grammar and no exact `[[models]]` entry**, or when it parses under a family below its `baseline_version`. **No exception exists:** an operator who needs an older id lowers that family's baseline, with complete facts. The migration fixes model selection first.

## 2. Vendor facts

### 2.1 Anthropic (Claude 5.x)

| Topic | Documented behaviour | Source |
|---|---|---|
| Thinking default | Messages API: "On by default". On Claude 5+ and Mythos Preview "thinking is already on". `display` defaults to `"omitted"`: empty `thinking` text, signature still present | https://platform.claude.com/docs/en/build-with-claude/thinking |
| Per-model `thinking` | **Opus 5.5, Fable 5.x and Mythos 5.x** accept only adaptive (no field, or `adaptive`); `enabled`, `between_tools` and `disabled` return 400. **Sonnet 5.5:** `between_tools` = "Up-front thinking off at `high` effort or below"; `disabled` returns 400. **Haiku 5.5:** `disabled` works at `high` effort or below; `between_tools` returns 400. **Opus 5:** `disabled` = "Thinking off at `high` effort or below" (400 at `xhigh`/`max`). **Sonnet 5:** `disabled` = "Thinking off" (no effort qualifier). **Mythos Preview:** adaptive by default, *and* `enabled`+`budget_tokens` = "Extended thinking". **Opus 4.6 / Sonnet 4.6:** off until `adaptive`; `enabled` = "Extended thinking (deprecated)". **Opus 4.7/4.8:** off by default; `enabled` returns 400. **Haiku 4.5:** manual `enabled` only | same page, "Configuring thinking" table |
| `between_tools` | Takes no other field (`display`, `budget_tokens`, `block_binding` each return 400). Returns 400 at `xhigh`/`max` | same page, "Progress updates" |
| Effort | `output_config.effort` ∈ `low`/`medium`/`high`/`xhigh`/`max`, with per-model subsets. Default `high`, except `medium` on Opus 5.5 and Haiku 5.5. "Effort is a behavioral signal, not a strict token budget" | https://platform.claude.com/docs/en/build-with-claude/effort |
| Sampling (`2023-06-01`) | On Fable 5.2/5.1/5, Mythos 5.2/5.1/5/Preview, Opus 5.5/5/4.8/4.7, Sonnet 5.5/5 and Haiku 5.5, "non-default `temperature`, `top_p`, or `top_k` values return a 400". `top_p` defaults to `0.99` there | thinking page, "Sampling parameters" |
| `max_tokens` | Thinking counts toward it, and it "is enforced as a strict limit". 5.x ceiling: 128K. The 21,333-token streaming requirement is an **SDK client-side guard**, not an API limit | thinking page, "Thinking and the context window", "Output limits", "Long requests" |
| Signatures | `signature` is "opaque: don't interpret or parse it". `redacted_thinking` carries encrypted `data` | thinking page, "Thinking encryption" |
| Multi-turn | Within a tool-use turn, passing thinking blocks back is required. Across turns it is recommended. Modified blocks return 400. Filtering on `type == "thinking"` alone "silently drops `redacted_thinking`" | thinking page, "Preserving thinking blocks" |
| Preserved-thinking constraints | Fable/Mythos 5.1–5.2, Opus 5.5, Sonnet 5.5 and Haiku 5.5 get a **prefix check**: changing `system`, `tools` or an earlier message makes later blocks fail ("can get a 400"; "bound to a different conversation"). Blocks from Sonnet 5.5, Haiku 5.5, Fable 5.2 and Mythos 5.2 "work only in the account that produced them or in an account linked to it" | thinking page, "Preserved thinking", "Thinking encryption" |
| Streaming | `thinking_delta`s, then one `signature_delta` before `content_block_stop` | thinking page, "Streaming thinking" |
| Usage | `output_tokens` is "the inclusive, authoritative total used for billing". `output_tokens_details.thinking_tokens` is "Always ≤ `output_tokens`". Stop reasons include `refusal`, `pause_turn` and `model_context_window_exceeded` | https://platform.claude.com/docs/en/api/messages/create |
| Errors | `400 invalid_request_error` is generic: "an issue with the format or content of your request" | https://platform.claude.com/docs/en/api/errors |
| `2026-10-21` | "`temperature`, `top_p`, and `top_k` … removed. A request that sends any of them returns a 400 error on every model". A `document` block placed directly in an `assistant` message returns 400. A `text` block with a **non-empty `citations` array in a `user` message** returns 400. Without sampling parameters "a request samples with their defaults, including a `temperature` of 1.0" | https://platform.claude.com/docs/en/api/versioning ; https://platform.claude.com/docs/en/api/migration-2026-10-21 |
| Ids | "Every Claude model ID is a pinned snapshot, including the dateless IDs". The Models API reports `max_tokens` and `capabilities.thinking.types.{disabled,between_tools}.supported` | https://platform.claude.com/docs/en/about-claude/models/overview |

### 2.2 OpenAI and Kimi

| Topic | Documented behaviour | Source |
|---|---|---|
| API choice | "Reasoning models work better with the Responses API"; Chat Completions "is still supported". "Starting with GPT-5.4, Chat Completions does not support tool calling with `reasoning_effort` values other than `none`" | https://developers.openai.com/api/docs/guides/reasoning ; https://developers.openai.com/api/docs/guides/migrate-to-responses |
| Effort | `reasoning.effort` (Responses) or `reasoning_effort` (Chat). Values are model-dependent: `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max`. GPT-6 Astra rejects `none`. GPT-6.1 Sol rejects `none` **and** `minimal`. GPT-5.6, 6.1 Sol, 6 Sol and 6 Luna default to `medium` | reasoning guide; https://developers.openai.com/api/docs/guides/latest-model |
| Sampling | "When reasoning effort is not `none`, remove `temperature`, `top_p`, and `top_logprobs`" (plus `logprobs` on Chat) | latest-model guide |
| Output cap | Responses: `max_output_tokens`. Chat: `max_completion_tokens`, "including visible output tokens and reasoning tokens". `max_tokens` is "deprecated … and is not compatible with **o-series models**" (scoped to o-series; nothing is claimed for other models) | https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create |
| Incomplete | `status: "incomplete"` with `incomplete_details.reason` ∈ `max_output_tokens` / `max_messages` / `content_filter` / `steered`. A cap hit may land "before any visible output tokens are produced" | https://developers.openai.com/api/reference/resources/responses/methods/create ; reasoning guide |
| Roles | "With o1 models and newer, `developer` messages replace the previous `system` messages". Responses has `instructions` | Chat reference |
| State | Stateless (`store: false` or ZDR) reasoning items include `encrypted_content` **by default**. The legacy `include` value is accepted but not required. With function calls, pass back the reasoning items. Responses "are stored by default" | reasoning guide; migrate guide §4 |
| Usage | Reasoning tokens "are billed as output tokens". Responses `output_tokens_details.reasoning_tokens`; Chat `completion_tokens_details.reasoning_tokens` | reasoning guide; references |
| Kimi K3.1 Pro | "Always has thinking mode enabled". `reasoning_effort` ∈ `low`/`high`/`max`, default `max`. `max_completion_tokens` "defaults to 131072 and can be set up to 1048576". "`temperature=1.0`, `top_p=0.95`, `n=1`, `presence_penalty=0`, and `frequency_penalty=0` are fixed; omit them" (the page does not say what a supplied value does). Return the complete assistant message unchanged. Readable `reasoning_content` is separate from `content` | https://platform.kimi.ai/docs/guide/kimi-k3-1-pro-quickstart.md |

### 2.3 Gemini and Ollama (generalisation check)

| Topic | Documented behaviour | Source |
|---|---|---|
| Gemini control | `generationConfig.thinkingConfig.{thinkingLevel, thinkingBudget, includeThoughts}`; setting it on a non-thinking model returns an error. `gemini-3.5-flash-lite` is "On (minimal)" with levels minimal/low/medium/high. `minimal` = "**Near-off** thinking". `gemini-2.5-flash-lite` defaults to **Off** | https://ai.google.dev/api/generate-content ; https://ai.google.dev/gemini-api/docs/thinking |
| Gemini state (generateContent, MAOS's API) | Signatures live on `Part.thoughtSignature` ("An opaque signature for the thought so it can be reused in subsequent requests"). "In the `generateContent` API, there are no dedicated thought blocks … signatures are metadata that can be attached to any part, such as living inside `functionCall` parts". The "MUST always resend all `thought` blocks" rule belongs to the **Interactions** API. That Gemini 3 function calling makes pass-back mandatory is **[INFERENCE]**, per the reviewer's link: the generate-content function-calling page rendered as the site home page on 2026-11-13 | thinking page callout; generate-content reference |
| Gemini usage | `thoughtsTokenCount` is separate from `candidatesTokenCount`; pricing "is the sum of output tokens and thinking tokens" | same pages |
| Ollama | `think`: `true`/`false`/`null`/level string. A model that cannot disable thinking falls back to its "lowest supported level". `/api/show` reports `thinking.values`/`default`. `message.thinking` is readable. No reasoning-token count or continuation requirement is documented | https://docs.ollama.com/capabilities/thinking |

**Generalisation.** The common core is small: an effort-like knob and an output cap that includes thinking. Everything else is an **optional, semantically distinct capability** for each profile:
- a *guaranteed* off switch versus *near-off*;
- opaque versus readable continuation;
- a reasoning-token count;
- an enforceable reasoning budget.

## 3. Current architecture

### 3.1 Request path

1. **Callers** build `InferenceRequest{prompt, options{max_tokens, temperature, model_id}, …}` (`crates/maos-domain/src/ports/inference.rs:30-45,69-76`):
   - butler: 512 tokens, temperature 0.0 (`spirits/butler/src/lib.rs:1180-1183`);
   - orchestrator: 64 tokens, 0.0 (`spirits/orchestrator/src/lib.rs:78,453-456`);
   - researcher: 0.0, `model_id: None` (`spirits/researcher/src/lib.rs:1308-1311`);
   - hello: 256 tokens, no temperature (`crates/maos-spirit-hello/src/lib.rs:112-115`);
   - judge: **8** tokens, 0.0, `model_id = Sonnet` (`crates/maos-eval/src/distill_replay.rs:57-68`). Record mode refuses a reply attributed to another model (`:288`).
2. **Model pin path (G9).**
   - The manifest pin `anthropic.claude-sonnet-5-5` (`spirits/researcher/manifest.toml:49`) becomes `Scope::ProviderInfer{provider}`, and the model is dropped (`crates/maos-manifest/src/manifest.rs:895-903`).
   - The Researcher binding carries the provider only (`crates/maos-bin/src/researcher_root.rs:190-207`).
   - The kernel dispatches by provider.
   - `anthropic.rs:75` substitutes the configured model, built from `ANTHROPIC_DEFAULTS` (`crates/maos-bin/src/main.rs:3884-3887,17733`).
   - At `1a57035e` that default was `claude-haiku-4-5-20251001`, so the defect predates the Haiku 5.5 sweep. The sweep changed the unintended served model.
   - The `[providers].model_id` check (`crates/maos-bin/src/admission.rs:982-1002`) is a different surface. Its catalog holds **one** model per provider (`admission.rs:974-978`).
3. **Kernel adapter** (`crates/maos-kernel-core/src/inference/mod.rs`):
   - capability check, refusal of cross-provider fallback, rate bucket, dispatch (`:285-336`);
   - 429 mapped to `RateLimited` (`:340-358`);
   - error mapping (`:359-367`);
   - `InferenceCall` TL record with model `options.model_id` or `"default"` (`:378`). Cost frames use response attribution (`:462-465`).
   - Router retries only on transport errors and 429/5xx (`crates/maos-kernel-core/src/inference/router.rs:74-82`).
4. **Drivers** (`crates/maos-providers/src/`):
   - Anthropic: override honoured `anthropic.rs:75`; sampling list `:99-122`; off switch `:129-135`; body `:138-153`; `anthropic-version: 2023-06-01` `:87`.
   - OpenAI/Kimi: Chat Completions, `max_tokens`, `temperature` only if `Some`; override **ignored** (`openai.rs:101-103,126-138`).
   - Ollama: override ignored, `num_predict` (`ollama.rs:51,68-83`).
   - Gemini: override must equal the configured model (`gemini.rs:69-75`).
   - Errors go through `post_json` and the sanitized excerpt (`provider.rs:108-125,153-202`). There is no streaming (`inference.rs:4-6`).
5. **Parse and usage:**
   - Anthropic concatenates the `text` blocks; usage `unwrap_or(0) as u32` (`anthropic.rs:161-218`), not the fail-closed `usage_count` (`provider.rs:349-358`).
   - Gemini: `candidatesTokenCount` only (`gemini.rs:149-150`).
   - Cost dimensions are `TokensIn`/`TokensOut` (`crates/maos-domain/src/cost.rs:27-31`), emitted at `inference/mod.rs:446-453` and priced by cost-reconcile from `xtask/provider-pricing.toml` (`crates/maos-cli/src/cli.rs:761`; `subcommands.rs:4855`).
6. **Cassette** `maos.journey.cassette/v1` (13 committed files under `crates/maos-journey-test/cassettes/`):
   - Each entry holds the prompt hash and length plus the response (`crates/maos-bin/src/cassette_replay.rs:244-272`).
   - Replay matches the hash and length, with an all-zero wildcard (`:151-152`).
   - Replay **replaces** the live driver: `CassetteReplayProvider` is a router `Provider` (`:178-210`).
7. **Judge provenance.** `judge_temperature_applied` comes from `anthropic::accepts_sampling` (`crates/maos-bench/src/distill_eval.rs:147-148`). `tests/judge-config.toml` pins `temperature = 0.0` "exactly".

### 3.2 Where model-specific behaviour lives

| Concern | Where |
|---|---|
| Sampling / thinking-off rules | `anthropic.rs:99-135`, `distill_eval.rs:147-148` |
| API version header | `anthropic.rs:87` |
| Output-cap spelling | `anthropic.rs:141`, `openai.rs:129`, `ollama.rs:76`, `gemini.rs:97` |
| Override policy (three rules) | `anthropic.rs:75`, `openai.rs:101`, `ollama.rs:51`, `gemini.rs:69-75` |
| Stop and usage mapping | `anthropic.rs:185-203`, `openai.rs:155-170`, `gemini.rs:124-150`, `ollama.rs:96-110` |
| Defaults, pins, policy | `main.rs:17733-17740`; spirit manifests; `xtask/model-currency.toml`; `xtask/provider-pricing.toml`; `tests/judge-config.toml` |

### 3.3 Gaps

| # | Gap | Effect | Evidence |
|---|---|---|---|
| G1 | Model rules are code; an undocumented id keeps old behaviour | New ids risk 400s. The currency gate prompts a fix but does not enforce one | `anthropic.rs:103-104`; `…/live-leg-fixes-evidence/README.md:169-172` |
| G2 | No reasoning intent and no applied record | The 8-token judge depends on Sonnet 5.5's `between_tools` switch. On a model without an off switch (Opus 5.5, Fable, Mythos), adaptive thinking *can* spend the cap before any text, because docs say thinking counts toward `max_tokens`. It can also skip thinking on simple requests (effort page). If no text block arrives, MAOS's parser raises the local error "no text content block" | `distill_replay.rs:65`; `anthropic.rs:178-182` |
| G3 | The OpenAI driver is Chat Completions with `max_tokens`; `temperature` is sent when supplied | **Latent.** Today it serves `gpt-4o-mini` only (`main.rs:17734`; override ignored). Selecting an allowed reasoning model (e.g. `gpt-6-astra`) would need `max_completion_tokens` (or Responses), sampling removal when effort ≠ `none`, and Kimi's fixed parameters | `openai.rs:101-136` |
| G4 | Gemini output omits thoughts; Anthropic usage falls back to 0 | Gemini cost is under-reported; there can be silent zero frames. Anthropic and OpenAI totals already include reasoning, so only the *breakdown* is lost | `gemini.rs:150`; `anthropic.rs:196-203` |
| G5 | Cassettes lack request intent and applied parameters | Replay cannot detect a changed effort or model | `cassette_replay.rs:244-272` |
| G6 | Single-turn only; nowhere to carry a continuation | The first multi-turn or tool story would drop signatures | `inference.rs:36` |
| G7 | Rejections are typed only by status | Profile drift is invisible | `provider.rs:40-41` |
| **G9** | **Manifest model pin is not the wire model** (pre-existing) | Researcher declares Sonnet but is served by the default (Haiku) when Anthropic is selected | §3.1 item 2 |
| G10 | The judge contract says `temperature = 0.0` "exactly" | That value cannot be applied on 5.x, or on any model under `2026-10-21` | `tests/judge-config.toml` |
| G11 | No streaming | Long high-effort calls risk HTTP timeouts | `inference.rs:4-6` |
| G12 | `InferenceCall` TL logs `"default"` | Audit cannot see the served model (cost frames can) | `inference/mod.rs:378` |

## 4. Options

| | A. Per-driver code rules (status quo) | **B. Profiles (data) + pure resolver + existing drivers consume plan** | B′. B plus a separate runtime `WireDialect` trait framework | C. Runtime discovery (Models API, `/api/show`) |
|---|---|---|---|---|
| Add a model | Rust edit | In-family: an `allowed` line only (profile inherited); fact change: one small version entry; plus regenerated goldens | Same as B | Automatic and unreviewed |
| Moving parts | Scattered | Data + one function; drivers stay the Strategy boundary | A second polymorphic layer duplicating the drivers | Network client and cache |
| Fail closed | No | Yes | Yes | Partial |
| Gates and replay | Weak | Hermetic | Hermetic | Breaks the offline gate (`xtask/src/check_model_currency.rs:11-16`) |

Vendor SDKs are excluded by FR47 (`xtask/fr47-vendor-sdk-denylist.toml`).

**Choose B.** B′ was rev. 1's proposal and is withdrawn: the drivers already are the per-vendor Strategy, so a second framework adds code without adding capability. C survives only as **inputs to the offline model-profile steward** (§5.11), never as runtime discovery.

## 5. Recommended design

### 5.1 Principle

- **Model capability facts are data:** off-switch kind, effort levels and default, sampling rule, output ceiling, which usage key carries reasoning, continuation kind, doc URLs and `verified` date.
- **Protocol implementation is code:** wire field names, parsing and error interpretation, inside each existing driver.
- **Shared helpers are added only where semantics genuinely match:** `post_json`, `usage_count`.

### 5.2 Domain contract (`maos-domain`)

```rust
pub struct InferenceOptions {
    pub max_output_tokens: u32,          // ENFORCEABLE total ceiling (thinking + answer) — today's max_tokens
    pub reasoning: ReasoningIntent,
    pub determinism: Determinism,
    pub model_id: Option<String>,        // set from the admitted pin (S1), not only by the judge
}
pub struct ReasoningIntent {
    pub effort: Option<Effort>,                 // None = model default
    pub allocation_hint: Option<u32>,           // reasoning share TARGET; applied where the profile has a
                                                // budget knob, never enforced (`allocation_enforced` = false)
    pub hard_reasoning_cap: bool,               // true ⇒ refused on every profile: no vendor documents a strict reasoning cap
    pub if_unsatisfiable: Unsatisfiable,        // Refuse | DegradeAndRecord
}
#[non_exhaustive] pub enum Effort { Off, Minimal, Low, Medium, High, XHigh, Max }
pub enum Determinism { ProviderDefault, PreferGreedy }      // never described as guaranteed determinism
pub struct TokenUsage { pub input_tokens: u32, pub output_tokens: u32 /* inclusive, billed */,
                        pub reasoning_tokens: Option<u32> /* breakdown only, ⊆ output */ }
pub enum Applied { Recorded(AppliedParams), Unrecorded { why: UnrecordedWhy } }   // explicit provenance state
pub enum UnrecordedWhy { PreS6Replay, MigratedV1Cassette, SeedExpectation }
pub struct AppliedParams { pub model_id: String, pub effort: Option<Effort>, pub thinking: ThinkingApplied,
                     pub greedy_applied: bool, pub wire_output_cap: u32, pub allocation_applied: Option<u32>,
                     pub allocation_enforced: bool /* always false today */, pub degraded: Vec<Degradation>,
                     pub profile_source: ProfileSource, pub profile_verified: &'static str }
pub enum ProfileSource { Declared { entry: String }, Inherited { from: String } }  // rev. 5; Inherited ⇒ unverified for this id
pub enum ThinkingApplied { ModelDefault, Off, UpFrontOff, NearOff, Level(Effort), BudgetTarget(u32) }
#[non_exhaustive] pub enum StopReason { MaxTokens, StopSequence, Refusal, ContextWindowExceeded,
    Incomplete { reason: String, has_text: bool }, ProviderStop(String) }
```

`InferenceResponse` gains `applied: Applied`. `InferenceError` gains `UnknownModel`, `IntentUnsatisfiable` and `InvalidRequest{code, message}`, all non-retryable (`router.rs:74-82` already treats non-429/5xx as terminal).

**`Applied::Unrecorded` (rev. 3).** Replay builds an `InferenceResponse` for every cassette entry (`crates/maos-bin/src/cassette_replay.rs:87-123`). Until S6, no cassette records the applied parameters, so S4 makes replay return `Applied::Unrecorded{PreS6Replay}`. That lets S4 land on its own. After S6, migrated v1 entries decode as `Unrecorded{MigratedV1Cassette}`, and hand-authored seeds as `Unrecorded{SeedExpectation}`. The state passes unchanged into TL and telemetry (`applied: "unrecorded:<why>"`) and into judge provenance, where the sidecar writes `greedy_applied: null` with the reason and never `false`/`true` invented after the fact. Nothing fabricates historical settings.

`max_output_tokens` is the **only** enforced limit. No vendor documents a strict reasoning-only cap:
- adaptive Claude has none;
- OpenAI effort is a level, not a cap;
- Claude manual `budget_tokens` "is a target rather than a strict cap … `max_tokens` remains the hard ceiling on total output" (https://platform.claude.com/docs/en/build-with-claude/extended-thinking#budget-rules-and-tuning);
- Gemini `thinkingBudget` is "the number of thoughts tokens that the model should generate" (https://ai.google.dev/api/generate-content).

So `hard_reasoning_cap` is refused on every profile, and a profile may set `budget_strict = true` only by citing a vendor page that documents a strict cap.

### 5.3 Profiles, resolver and drivers (`maos-providers`)

```rust
pub struct ModelProfile { pub provider: &'static str, pub id: &'static str, pub snapshot_suffix: bool,
    pub max_output_tokens: u32, pub sampling: SamplingRule, pub thinking: ThinkingRule,
    pub effort: Option<EffortRule>, pub reasoning_budget: Option<BudgetRule>,  // budget KNOB (a target), if any
    pub usage_reasoning_key: Option<&'static str>, pub continuation: ContinuationKind,
    pub docs: &'static [&'static str], pub verified: &'static str }
pub enum SamplingRule { Accepted, AcceptedUnlessThinking, NonDefaultRejected, RejectedUnlessEffortNone, Fixed }
pub struct ThinkingRule { pub default_on: bool, pub off: OffKind, pub off_max_effort: Option<Effort> }
pub enum OffKind { None, Guaranteed(&'static str) /* "disabled", "effort:none" */,
                   UpFrontOnly(&'static str) /* "between_tools" */, NearOff(&'static str) /* gemini minimal, ollama think:false fallback */ }
pub enum ContinuationKind { None, OpaqueBlocks, OpaqueItems, PartSignatures, FullAssistantMessage }

pub fn profile(provider: &str, model_id: &str) -> Result<ResolvedProfile, ProviderError>;
//   ResolvedProfile = merged ModelProfile + ProfileSource + verified; Err(UnknownModel) iff no family grammar AND no exact entry matches, or the id parses below its family baseline (no exception) (§5.4)
pub fn resolve(p: &ResolvedProfile, o: &InferenceOptions) -> Result<(WirePlan, Applied), ProviderError>;
pub fn served_model(provider: &str, pin: Option<&str>) -> Result<String, ProviderError>; // pin > [providers.p].default_model > Err
// Each existing driver: fn build(plan: &WirePlan, prompt: &str) -> Value; fn parse(json, plan) -> Result<…>
```

**Resolution rules** (pure, table-tested):
- **Effort.** Use the nearest supported level, ties going up; otherwise apply the `Unsatisfiable` policy.
- **`Off`.**
  - `Guaranteed` satisfies it.
  - `UpFrontOnly` satisfies it for tool-free requests (MAOS has no tools today). Its record reads `UpFrontOff`.
  - `NearOff` is **never** a success: under `Refuse` the call is refused; under `DegradeAndRecord` it records `NearOff`.
  - With no off switch, the profile's lowest level is used and recorded as a degradation (or the call is refused).
- **Cap.** The wire cap is `min(max_output_tokens, profile.max_output_tokens)`. A clamp is recorded.
- **Allocation hint.** It is *applied*, never *enforced*, through a budget knob: Gemini `thinkingBudget`, or manual `enabled`+`budget_tokens` on the profiles that accept it (Mythos Preview, Haiku 4.5, Opus/Sonnet 4.5, deprecated on 4.6). The record reads `allocation_applied = Some(n)`, `allocation_enforced = false`. Profiles without a knob ignore the hint (`allocation_applied = None`). `hard_reasoning_cap = true` is refused before the network on every profile (§5.2).
- **Sampling depends on the resolved thinking mode, not only on the static category** (rev. 3). Under `2023-06-01`, on Claude models outside the 5.x "non-default rejected" list, "the restriction applies only while thinking is on: `temperature` and `top_k` are incompatible with thinking, and `top_p` is allowed at values between 0.95 and 1" (https://platform.claude.com/docs/en/build-with-claude/thinking, "Sampling parameters"). Those profiles are therefore `AcceptedUnlessThinking`. The resolver settles thinking first, then sampling:
  - `temperature: 0` is sent for `PreferGreedy` only if the profile is `Accepted`, or `AcceptedUnlessThinking` with resolved thinking `Off`/`ModelDefault`-off, or `RejectedUnlessEffortNone` with effort `none`.
  - When thinking is on (e.g. a budget target on Haiku 4.5 or adaptive on Sonnet 4.6), `greedy_applied = false` is recorded with the degradation "sampling dropped: thinking on". Under `if_unsatisfiable = Refuse`, the call is refused instead.
  - `top_p`/`top_k` are never sent. An Anthropic `2026-10-21` row makes every Anthropic profile send none.
- **Served model (Q6, ruled).** The served model is the **manifest pin**. Failing that, it is `[providers.<p>].default_model` from the profile file. Failing both, the call fails with `Unconfigured`. No code-base default exists after S3a. A driver serves any **admitted** model of its provider (the set of pins admitted for the calling Spirit, S1), so the judge's separately admitted Sonnet pin (`tests/judge-config.toml`) coexists with Haiku workflow pins.

**Response rules:**
- Anthropic: `text` blocks only, plus `output_tokens_details.thinking_tokens`.
- **Cap exhaustion without text keeps its usage** (rev. 3). Responses `incomplete` with `reason = max_output_tokens` and no `output_text`, or Anthropic `stop_reason: max_tokens` after thinking only, returns **`Ok`** with empty `text`, `StopReason::Incomplete{reason, has_text: false}` and the vendor's usage. It is no longer an error; today Anthropic raises a `Serde` "no text content block" error at `anthropic.rs:178-182`. Reasons for choosing this over carrying usage on the error path:
  - the kernel emits cost frames only on `Ok` (`crates/maos-kernel-core/src/inference/mod.rs:392-483`), and `InferenceError` carries no usage;
  - OpenAI warns this case can "incur costs for input and reasoning tokens without receiving a visible response" (https://developers.openai.com/api/docs/guides/reasoning#allocating-space-for-reasoning);
  - an error-path variant would need a second, parallel accounting branch in kernel-core.

  Callers decide usefulness *after* accounting. **Rev. 4:** the rev. 3 [INFERENCE] that every caller already rejects empty text was **false**. I audited every production consumer of `InferenceResponse.text` (all non-test `.complete(` sites under `crates/` and `spirits/`) on 2026-11-13. Partial text keeps `has_text: true`.

  | Caller | Where `text` is consumed | Empty text today | Required S4 behaviour | Caller regression test (S4) |
  |---|---|---|---|---|
  | Researcher survey | `spirits/researcher/src/lib.rs:1316-1330` `complete_bound` → `Ok(response.text)`; `survey_bound` `:1286-1291` → `summarize`; `survey_with` puts it in `Finding.statement` `:1516` | **Accepted.** An empty finding statement enters the digest | `complete_bound` returns `ResearcherError::Inference(Incomplete)` when `stop_reason = Incomplete{has_text:false}` or `text.trim().is_empty()`. It reports context usage **before** the check (`:1320-1329`). The pass fails; no empty finding is produced | `researcher_survey_rejects_empty_incomplete_after_usage` |
  | Hello | `crates/maos-spirit-hello/src/lib.rs:121-128` → `HelloResponse.introduction = resp.text`; consumers: the shell `crates/maos-shell/src/lib.rs:565-568`, `:717-723`, `:740-758`, and the `MAOS_ONE_SHOT` path `crates/maos-bin/src/main.rs:9788` (`:4587` only passes `HelloSpirit` into scheduler loading; corrected after `gpt-review-4.md` P3) | **Accepted.** Success with an empty introduction | Return `HelloError` (new `IncompleteCompletion` variant) instead of an empty introduction. The `Unconfigured`/`ProviderTransport` fallbacks (`:129-157`) are unchanged | `hello_empty_incomplete_is_an_error_not_an_empty_introduction` |
  | Butler proposal | `spirits/butler/src/lib.rs:1188-1198` → `parse_proposal(&response.text)` (`:159-161`) | **Rejected only incidentally:** serde fails on `""` with "completion is not a Butler proposal: EOF…" | Explicit check after the context report (`:1191`): `ButlerError::Inference(Incomplete)`, typed and distinct from a malformed proposal | `butler_empty_incomplete_is_typed_not_a_parse_error` |
  | Orchestrator ambiguity | `spirits/orchestrator/src/lib.rs:461-472` → `parse_ambiguity(&response.text)` (`:194-196`) | **Rejected only incidentally:** serde error → `OrchestratorInferenceError::Completion` | Explicit check after the usage report (`:465-466`) → a typed incomplete variant; the dispatch-ambiguity scalar is never defaulted | `orchestrator_empty_incomplete_is_typed_not_a_parse_error` |
  | Distill judge (eval) | `crates/maos-bench/src/distill_eval.rs:311-314` collects replies; `crates/maos-eval/src/distill_replay.rs:294` → `judge_verdict` (`:83-91`) | **Rejected:** `""` matches neither word → `Err("judge answered ``, …")` | Explicit check first, giving the message "judge incomplete: no text (reason …)"; the recorded reply keeps its usage in the cassette | `judge_empty_incomplete_is_refused_with_reason` |
  | Halt eval wrapper | `crates/maos-bench/src/halt_eval.rs:709-712` passes the response through | n/a (wrapped Butler/Orchestrator decide) | None; covered by the two rows above | covered by Butler/Orchestrator tests |
  | Cassette record/replay | `crates/maos-bin/src/cassette_replay.rs:244-272` records; `:87-123` replays | Recorded/replayed verbatim | Record `stop_reason` incomplete + `has_text` (S6 schema); replay reproduces the empty `Incomplete` so the caller checks above fire under replay too | `replay_reproduces_empty_incomplete` (S6) |

  Kernel, router and provider tests that read `.text` are not production consumers.
- Gemini: output = `candidates + thoughts`.
- Kimi: `content` only; `reasoning_content` is never shown as the answer.

**Errors.** All 400s map to `InvalidRequest`. `UnsupportedParameter` is produced only when a vendor supplies a discriminating field. OpenAI's `error.param`/`error.code` is **[INFERENCE]**, not re-read. Anthropic's generic `invalid_request_error` is never message-parsed. The drift signal is a per-profile *count* of `InvalidRequest` rejections, not a classification. Excerpts still pass only through `rejection_excerpt` (`provider.rs:153-202`).

### 5.4 Family-first profiles (operator ruling 2026-11-13)

Operator, verbatim: *"can we do model family approach inheritance then detailed model specification of each model? for example claude opus 6.0 may have the claude opus 5.5's profile inherited, unless breaking change occur somewhere before 6.0 versioning. profile file is better than hardcoding in the code base."*

**Layers**, each carrying only facts the layer above does not already give:
1. **Vendor/API defaults** `[providers.<p>]`: API version, `default_model`, usage keys, continuation kind.
2. **Family** `[families.<f>]`: provider, **id grammar**, `baseline_version`, and **complete** baseline facts for that version, with `docs`, `verified` and `declares`.
3. **Version entries** `[[families.<f>.versions]]`: present **only where facts change** — a *breaking boundary*, strictly above the baseline. Each carries only the changed facts, plus `docs`, `verified` and `declares`.
4. **Exact-id entries** `[[models]]`: either an **overlay** on an id that also parses under its family (a snapshot or model-specific exception), or a **standalone** model that no grammar covers (`claude-mythos-preview`, `gpt-4o-mini`, Ollama tags).

**Id grammar** is data and is parsed structurally, never by substring prefix. A grammar is literal segments plus typed placeholders:
- `{major}` and `{minor}`: 1–2 digits;
- `{date8}`: exactly 8 digits;
- `{tier}`: from a declared enum.

Optional groups are written `[...]`. The loader compiles each grammar into a matcher. It rejects at load time any pair of **family grammars** that could both match one id, and the gate checks this too. An exact-id entry is not a grammar, so it never counts as a second match.

**What an entry declares (rev. 6).** A baseline or version entry declares **exactly** the full id strings in its `declares` list, and nothing else. Each listed id must parse under the family to a version in that entry's range: at or above the entry, and below the next entry. Listing an id records that its facts were verified against that entry. An `[[models]]` entry declares its own `id`.

**Resolution of `(provider, id)`.** There are two paths:

- **(a) Family path.** Exactly one family grammar matches.
  1. Extract `(major, minor)` (minor defaults to 0) and an optional `date8`.
  2. If the version is below `baseline_version`, refuse with `UnknownModel` (never backwards). There is **no exception**, including for an exact entry. An exact entry whose id parses below its family baseline fails the gate. To support an older id, lower the family's `baseline_version` and supply complete facts and evidence for that version.
  3. Select the greatest entry at or below the version. The baseline counts as an entry.
  4. Merge provider defaults ← baseline ← every version entry in ascending order up to the selected one ← an exact `[[models]]` **overlay** for this full id, if present.
- **(b) Exact-only path.** No grammar matches, but a `[[models]]` entry has this exact id.
  - The facts are provider defaults ← the merged facts of its `inherit` target (`<family>@<version>`, resolved by path (a) without an id) if one is given ← the entry's own fields.
  - There is no version arithmetic. The merged result must be complete; the gate checks this.
- **Refused** (`UnknownModel`, before any transport call): the id matches **no family grammar and no exact entry**, or it parses under a family below that family's `baseline_version`.

**Provenance (precedence in this order):**
1. An exact `[[models]]` entry for the full id gives `Declared{entry: "models:<id>"}`.
2. Otherwise, if the selected entry's `declares` contains the full id, `Declared{entry: "<family>@<version>"}`.
3. Otherwise `Inherited{from: "<family>@<version>"}`. This includes an **undeclared snapshot of a declared version** (e.g. `claude-opus-5-5-20261101` when only `claude-opus-5-5` is listed) and every forward id (e.g. `claude-opus-6-0`).

`verified` is the date of the newest contributing entry.

**Merge semantics (rev. 6):**
- **Records** (TOML tables without a `kind` field, e.g. `effort = { levels, default }`) merge **field-wise**: a later layer sets only the fields it names. So `effort = { default = "medium" }` at 5.5 keeps the levels from 4.7.
- **Arrays** (`effort.levels`) **replace** whole.
- **Scalars** replace.
- **Tagged capability variants** are tables with a `kind` field: `thinking.off`, `reasoning_budget`. They replace **atomically**. When `off` changes from `{kind="guaranteed", value="disabled", max_effort="high"}` to `{kind="none"}`, the old `max_effort` disappears.
- **Clearing.** `unset = ["reasoning_budget"]` removes a field inherited from earlier layers. Removing a required field fails the gate.
- **Entry metadata** never merges: `at`, `declares`, `docs`, `verified`.

Prices are **never** inherited. They are policy in `provider-pricing.toml`, and a missing row leaves the call unpriced by cost-reconcile (a warning, no inherited price).

```toml
schema = "maos.model-profiles/v2"

[providers.anthropic]
api_version = "2023-06-01"                 # flipped only by its own slice (S7)
default_model = "claude-haiku-5-5"         # replaces ANTHROPIC_DEFAULTS.1 in main.rs (Q6)
usage_reasoning_key = "output_tokens_details.thinking_tokens"
continuation = "opaque-blocks"

[families.claude-opus]
provider = "anthropic"
grammar = "claude-opus-{major}[-{minor}][-{date8}]"
baseline_version = "4.5"                   # complete facts for Opus 4.5
declares = ["claude-opus-4-5-20251101"]
thinking = { default_on = false, off = { kind = "guaranteed", value = "disabled" } }
sampling = "accepted-unless-thinking"
max_output_tokens = 64000
reasoning_budget = { kind = "manual", knob = "enabled.budget_tokens", strict = false }
effort = { levels = ["low","medium","high"], default = "high" }
docs = ["https://platform.claude.com/docs/en/build-with-claude/thinking",
        "https://platform.claude.com/docs/en/build-with-claude/effort"]
verified = "2026-11-13"

[[families.claude-opus.versions]]
at = "4.6"                                 # 128K; `max` effort; manual thinking deprecated but accepted
declares = ["claude-opus-4-6"]
max_output_tokens = 128000
effort = { levels = ["low","medium","high","max"] }
docs = ["https://platform.claude.com/docs/en/build-with-claude/thinking"]
verified = "2026-11-13"

[[families.claude-opus.versions]]
at = "4.7"                                 # non-default sampling → 400; manual `enabled` → 400; `xhigh`
declares = ["claude-opus-4-7", "claude-opus-4-8"]
sampling = "non-default-rejected"
unset = ["reasoning_budget"]
effort = { levels = ["low","medium","high","xhigh","max"] }
docs = ["https://platform.claude.com/docs/en/build-with-claude/thinking"]
verified = "2026-11-13"

[[families.claude-opus.versions]]
at = "5.0"                                 # thinking on by default; `disabled` only at ≤ high
declares = ["claude-opus-5"]
thinking = { default_on = true, off = { kind = "guaranteed", value = "disabled", max_effort = "high" } }
docs = ["https://platform.claude.com/docs/en/build-with-claude/thinking"]
verified = "2026-11-13"

[[families.claude-opus.versions]]
at = "5.5"                                 # no off switch (atomic replace); effort default medium (levels kept)
declares = ["claude-opus-5-5"]
thinking = { off = { kind = "none" } }
effort = { default = "medium" }
docs = ["https://platform.claude.com/docs/en/build-with-claude/thinking",
        "https://platform.claude.com/docs/en/build-with-claude/effort"]
verified = "2026-11-13"

[[models]]                                 # standalone: no grammar covers it
provider = "anthropic"
id = "claude-mythos-preview"
inherit = "claude-mythos@5.0"
reasoning_budget = { kind = "manual", knob = "enabled.budget_tokens", strict = false }
docs = ["https://platform.claude.com/docs/en/build-with-claude/thinking"]
verified = "2026-11-13"

[[models]]                                 # standalone OpenAI non-reasoning model (Chat)
provider = "openai"
id = "gpt-4o-mini"
sampling = "accepted"
thinking = { default_on = false, off = { kind = "none" } }

[providers.ollama]
default_model = "llama3.1:8b"              # Ollama: standalone [[models]] entries only (local tags are arbitrary)
```

There is no reserve table: total ceilings stay at the callers' current values (Q5).

**Every currently allowed id resolves** (`xtask/model-currency.toml` `allowed` lists; families sketched as above):

| Allowed id | Path | Selected entry | Provenance |
|---|---|---|---|
| `claude-opus-4-5-20251101` | family | `claude-opus@4.5` (baseline) | Declared |
| `claude-opus-4-6` | family | `claude-opus@4.6` | Declared |
| `claude-opus-4-7`, `claude-opus-4-8` | family | `claude-opus@4.7` | Declared (both listed) |
| `claude-opus-5` | family | `claude-opus@5.0` | Declared |
| `claude-opus-5-5` | family | `claude-opus@5.5` | Declared |
| `claude-sonnet-4-6` | family | `claude-sonnet@4.6` (baseline) | Declared |
| `claude-sonnet-5` | family | `claude-sonnet@5.0` (thinking on, `disabled` without effort limit, sampling rejected) | Declared |
| `claude-sonnet-5-5` | family | `claude-sonnet@5.5` (`off = up_front_only between_tools ≤ high`) | Declared |
| `claude-haiku-4-5`, `claude-haiku-4-5-20251001` | family | `claude-haiku@4.5` (baseline: manual thinking, no effort) | Declared (both listed; the dateless id is the vendor alias for the dated snapshot) |
| `claude-haiku-5-5` | family | `claude-haiku@5.5` (`off = guaranteed disabled ≤ high`, effort default medium, `unset reasoning_budget`) | Declared |
| `claude-fable-5`, `claude-fable-5-1` | family | `claude-fable@5.0` (baseline; adaptive only) | Declared. 5.1 is listed on the baseline because it is identical **only in the modelled single-turn capability fields** (thinking modes, sampling, effort, ceiling). Their multi-turn behaviour differs: Fable 5.1 is a "model with preserved thinking" whose failed prefix check gives a "Possible 400 error", while Fable 5 is not on that list (https://platform.claude.com/docs/en/build-with-claude/preserved-thinking/prefix-checks, "What each model does when the prefix check fails"). Those continuation facts are not modelled until S8, which must add them per id, and that may split 5.1 into its own entry. |
| `gpt-5.6-sol`, `gpt-6-sol`, `gpt-6.1-sol` | family `gpt-{major}[.{minor}]-sol` | `@5.6` baseline, `@6.0`, `@6.1` (rejects `none`/`minimal`) | Declared |
| `gpt-5.6-terra` | family `…-terra` | `@5.6` baseline | Declared |
| `gpt-5.6-luna`, `gpt-6-luna` | family `…-luna` | `@5.6` baseline, `@6.0` | Declared |
| `gpt-6-astra` | family `…-astra` | `@6.0` baseline (rejects `none`) | Declared |
| `gpt-4o-mini` | exact-only | `models:gpt-4o-mini` | Declared |
| `gemini-3.5-flash-lite` | family `gemini-{major}[.{minor}]-flash-lite` | `@3.5` baseline (`off = near_off minimal`) | Declared |
| `kimi-k3.1-pro` | family `kimi-k{major}[.{minor}]-pro` | `@3.1` baseline (`sampling = fixed`) | Declared |
| `llama3.1:8b` | exact-only | `models:llama3.1:8b` | Declared |

The OpenAI per-version facts follow §2.2. Their exact levels and defaults per tier are filled from each model page in S3a **[INFERENCE: the tier split is the design; the per-tier values still need reading from each model page]**.

**Three files, one join key `(provider, model_id)`, no duplicated facts:**

| File | Owns |
|---|---|
| `xtask/model-currency.toml` | Pin policy: what a manifest **may** pin |
| `xtask/model-profiles.toml` | Capability facts and the provider default model |
| `xtask/provider-pricing.toml` | Prices |

The profile file is embedded into `maos-providers` with `include_str!` (precedent: `distill_eval.rs:131`) and parsed once with typed errors.

**Gate cross-checks** (extending `check-model-currency`, which today scans manifests, pricing and `main.rs` constructor literals, `xtask/src/check_model_currency.rs:18-29`):
1. Every `allowed` id **resolves** by either path.
2. Every `default_model`, manifest pin and judge-config model is `allowed` and resolves. This replaces the `main.rs` literal scan once S3a removes the code defaults.
3. Integrity:
   - family grammars are pairwise unambiguous;
   - version entries are strictly ascending and above `baseline_version`; no exact entry parses below its family baseline;
   - every `declares` id parses into its entry's range and is listed once;
   - `inherit` targets exist and are acyclic.
4. Every baseline is complete, and so is every merged result: no required field is unset.
5. Every entry has `docs` and `verified`.
6. Retired ids need nothing.

**Q1 reconciled (fail closed, ruled).**
- An id that parses under a family grammar at or above its baseline resolves. It is recorded `Declared` if listed, otherwise `Inherited` (unverified for that id).
- An exact-only id resolves through its `[[models]]` entry.
- An id with **no grammar match and no exact entry**, or parsing below a family baseline (no exception), is refused before any network call.
- **Policy stays with the currency gate:** a manifest may pin only `allowed` ids.
- So the **per-new-model cost** is one `allowed` line, plus a price row if cost-reconcile should price it. A profile edit is needed only when facts change, or when the steward verifies the id and adds it to a `declares` list.

**Accepted risk.** Forward inheritance replaces the Correction 2 exact-id rule (`…/live-leg-fixes-evidence/README.md:169`). It is an operator-accepted risk (2026-11-13): an inherited id may differ from its predecessor in an unknown way. Mitigations:
- the vendor's 400 surfaces as a typed `InvalidRequest` carrying the sanitized vendor message (`provider.rs:153-202`);
- per-profile rejection counts, keyed by `profile_source`, so an inherited id's 400s point at the entry it inherited from;
- `Applied`/TL/eval provenance show `Inherited` + `verified`, so no judge result silently claims a verified profile;
- the steward (§5.11) checks every `allowed` inherited id against the vendor's documented capabilities and pages.

### 5.5 Usage and cost

- `output_tokens` stays the inclusive billed total.
- Gemini becomes `candidates + thoughts`.
- Anthropic uses the fail-closed `usage_count`.
- `reasoning_tokens` is an informational breakdown. If it is surfaced as a `CostDimension`, cost-reconcile keeps pricing only `TokensIn`/`TokensOut` (`subcommands.rs:4855`), so nothing is double-billed.

### 5.6 Continuations (last slice)

```rust
pub struct ReasoningContinuation { kind: ContinuationKind, provider: &'static str, model_id: String,
    conversation_prefix_digest: [u8; 32], account_scope: AccountScope, digest: [u8; 32], bytes: Box<[u8]> }
// Debug shows kind/model/len/digest prefix only; no Serialize/Display.
```

- Bytes are the vendor artefact, verbatim:
  - Anthropic: every `thinking`/`redacted_thinking` block, in order.
  - OpenAI: every output item.
  - Gemini generateContent: the model `Content` with its `thoughtSignature` parts in original order, not Interactions `thought` steps.
  - Kimi: the full assistant message, whose readable `reasoning_content` is treated as sensitive.
- `conversation_prefix_digest` exists because a changed system prompt, tools or history invalidates later blocks.
- `account_scope` (the credential fingerprint, `provider.rs:28`) exists because some blocks are account-bound. A mismatch drops the artefact and records it.
- Continuations are memory-only and are never logged or committed.

### 5.7 Cassette v2

**Recorded per entry:**
- `plan_digest`: canonical `WirePlan`, minus credentials, with each continuation represented only by its digest;
- `applied`;
- `usage.reasoning_tokens`;
- `continuation: {kind, digest, len}`, never bytes.

**Two decisions:**
1. **Where replay gets the plan.** Replay replaces the driver, so `CassetteReplayProvider` calls the same pure `profile()` + `resolve()` on the incoming request (the model comes from the request after S1), computes the current plan digest, and compares it with the recorded one. A mismatch is strict-mode drift. The replay provider needs only the embedded registry, no driver.
2. **Synthetic continuations.** In replay, a continuation is a handle carrying the recorded `digest` and `kind` with empty bytes. Plan digests use the digest, so live and replay digests agree. A driver refuses to send a synthetic handle to a vendor.

**Labelling v1 history.** Cassettes are rewritten once to v2 (clean cutover; the v1 reader is deleted). Migrated entries get `plan: {"status": "unrecorded-v1"}` and decode to `Applied::Unrecorded{MigratedV1Cassette}`, or `SeedExpectation` when `provenance = "seed"`. Strict replay skips the plan comparison for exactly those entries, and the report lists them. Between S4 and S6, the v1 reader returns `Unrecorded{PreS6Replay}` for every entry (§5.2). The existing `provenance` stays separate: `seed` cassettes are hand-authored expectations, not observations.

### 5.8 Placement

- **`maos-domain`:** contract types.
- **`maos-providers`:** profiles, resolver, driver changes. This crate is already over its ceiling, 1867 vs 1679 (`…/live-leg-fixes-evidence/README.md:445`); raises need a measured figure and an open story key (`xtask/kloc.toml:61-66`).
- **Kernel-core:** narrow edits only, under the ADR-069 envelope (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:36-46`) with pin and history updates. They are new error arms (`inference/mod.rs:359-367`) and the TL served model (`:378`). "About +10 lines" is an estimate, not a budget.
- **S1 also touches `maos-bin` admission and bindings** (`admission.rs:974-1002`, `researcher_root.rs:190-207`) and possibly the scope or binding types. Whether the model joins `Scope::ProviderInfer` (a kernel capability change) or travels in the binding is S1's design decision. The binding route keeps the kernel capability model unchanged.
- **No new service.** That `check-service-boundary` is unaffected is **[INFERENCE]**; it depends on the final signatures.

### 5.9 Testing

1. **Resolver tables:** every profile × effort × determinism × cap boundary (exact cap, clamp above the profile ceiling, overflow) × `Refuse`/`DegradeAndRecord`, giving an expected `(WirePlan, Applied)` or a typed refusal. Also:
   - `hard_reasoning_cap = true` is refused on **every** profile, including the manual-budget and Gemini profiles;
   - an allocation hint on a manual-budget profile yields `allocation_applied = Some`, `allocation_enforced = false`.
2. **Golden request bodies** per profile and case, byte-exact; regeneration is a reviewed diff. Today's judge body (`anthropic_5_5_live_leg_correction.rs`) stays identical through the behaviour-preserving slices.
3. **Doc-derived negatives:**
   - Sonnet 5.5 never gets `disabled`;
   - Opus 5.5 never gets an off switch;
   - `between_tools` never gets sibling fields;
   - Opus 5 `disabled` never goes with `xhigh`/`max`;
   - no sampling on `NonDefaultRejected`.
4. **End-to-end:**
   - manifest → binding → wire model, with Researcher's Sonnet pin on the wire;
   - workflow Haiku plus judge Sonnet on the same configured provider;
   - an unknown model is refused with **zero transport calls**.
5. **Response fixtures:**
   - thinking-first and `redacted_thinking`;
   - Responses `incomplete` *with* partial text vs *without* (and `content_filter`);
   - Gemini thoughts usage;
   - Gemini `minimal` reported as `NearOff`, never `Off`;
   - generateContent part signatures vs Interactions steps kept separate;
   - Kimi `reasoning_content` never becomes `text`.
6. **Errors:** a generic Anthropic `invalid_request_error` maps to `InvalidRequest`, never to `UnsupportedParameter`. A positively identified vendor code (where documented) maps to `UnsupportedParameter`.
7. **Cassette v2:** round trip; plan drift; `unrecorded-v1` labelling; synthetic continuation is never sent. Between S4 and S6, every replayed response carries `Applied::Unrecorded{PreS6Replay}`, and the judge sidecar writes `greedy_applied: null` + reason.
8. **Sampling vs resolved thinking (negative, rev. 3):** `PreferGreedy` + budget target on Haiku 4.5, and `PreferGreedy` + adaptive on Sonnet 4.6, never emit `temperature`/`top_k` and record "sampling dropped: thinking on". The same request with thinking off does emit `temperature: 0`.
9. **Usage on empty incomplete (rev. 3):** a Responses `incomplete`/`max_output_tokens` body with only a reasoning item, and an Anthropic thinking-only `max_tokens` body, each return `Ok` with empty text, `has_text: false` and the vendor's usage. A kernel test asserts that a `CostAttribution` frame is emitted with those token counts.
10. **Caller-level failure after accounting (rev. 4):** the five named per-caller tests in the §5.3 table. Each feeds an `Ok` response with empty text, `Incomplete{has_text:false}` and non-zero usage, then asserts (a) a typed caller error, (b) usage/context was reported first where the caller reports it, and (c) no empty artefact (finding, introduction, proposal, scalar, verdict) is produced.
11. **Family resolution (rev. 5/6):**
    - **Two paths:**
      - `claude-opus-6-0` → family path, `Inherited{from: claude-opus@5.5}`;
      - `gpt-4o-mini`, `llama3.1:8b` and `claude-mythos-preview` → exact-only path (the last follows `inherit`), `Declared{models:…}`, with no version arithmetic;
      - an exact overlay `claude-opus-5-5-20261101` added as `[[models]]` → family path plus overlay, `Declared{models:…}`, and **not** an ambiguity error.
    - **Baseline:** every id in the §5.4 allowed-id table resolves with the listed provenance (`claude-opus-4-5-20251101` → `claude-opus@4.5` baseline, Declared). `claude-opus-4-1` (below baseline 4.5) → `UnknownModel`. Adding an exact `[[models]] id = "claude-opus-4-1"` **fails the gate**, rather than resolving it.
    - **Example B boundary (§6):** with the 6.2 entry added, `claude-opus-6-0` and `-6-1` resolve to `off = {kind: "none"}` (inherited from 5.5). `claude-opus-6-2` and `-7-0` resolve to `off = {kind: "up_front_only", value: "between_tools", max_effort: "high"}`, the whole variant replaced. `thinking.default_on` stays `true` (field-wise merge), and `effort` is unchanged from 5.5 (levels `[low, medium, high, xhigh, max]`, default `medium`).
    - **Refusal:** `claude-opusx-5-5` and `my-claude-opus-5-5` (no grammar, no exact entry; proves no substring matching) → `UnknownModel` with zero transport calls.
    - **Provenance precedence:** `claude-opus-5-5-20261101` without an overlay is `Inherited{from: claude-opus@5.5}`, **not** Declared, even though `(5,5)` has an entry. `claude-opus-4-8` is Declared via the 4.7 entry's `declares`. A `declares` id outside its entry's range fails the gate.
    - **Merge boundaries:**
      - at 5.5, `effort.default = medium` with `effort.levels` still `[low, medium, high, xhigh, max]` (field-wise record merge);
      - at 5.0, `off = {guaranteed, disabled, max_effort: high}`; at 5.5, `off = {kind: none}` with **no** leftover `max_effort` (atomic variant replace);
      - `reasoning_budget` present at 4.5/4.6 and absent from 4.7 onwards (`unset`);
      - at 4.6, `levels` replaced whole (adds `max`, no `xhigh`); at 4.7, `xhigh` is added;
      - `unset` of a required field fails the gate.
    - **Forward change:** an added 6.2 entry changes 6.2/7.0 and leaves 6.0/6.1 unchanged.
    - **Integrity:** two overlapping family grammars fail the load and the gate.
    - **Served model:** a pin beats `default_model`; no pin and no default gives `Unconfigured`.
12. **Resolved-facts diff (steward PRs, rev. 6):** a deterministic dump of resolved facts and provenance for every `allowed` id, before and after the patch. The gate fails if any id outside the PR's declared `expected_changes` list differs.

### 5.10 Observability

- **Recorded** (TL and telemetry): served model, effort, `ThinkingApplied`, wire cap, `allocation_applied`/`allocation_enforced`, reasoning tokens, degradations, `verified`, or `unrecorded:<why>`.
- **Never recorded:** thinking text, summaries, `reasoning_content`, signatures or encrypted content.
- **Not requested:** Claude `display` stays at its omitted default; OpenAI `reasoning.summary` is not requested.
- **Unchanged:** credential handling stays with header confinement and `rejection_excerpt`.

### 5.11 Model-profile steward (AI agent, outside the runtime; replaces the rev. 4 "drift report")

This addresses the operator's maintenance concern: *"I concern that will needs much maintenance unless an AI agent can manage the models and review them."* It is a procedure an AI agent runs on demand, or on a schedule (e.g. a weekly CI job), **never inside the runtime**. There is no runtime network discovery.

**Inputs.**
- The three data files.
- The doc URLs cited in `docs` (re-read and quoted).
- Vendor list/capability endpoints where documented:
  - Anthropic Models API: `max_input_tokens`, `max_tokens`, `capabilities.thinking.types.{disabled,between_tools}.supported` (https://platform.claude.com/docs/en/about-claude/models/overview);
  - OpenAI `GET /v1/models`: ids, `created` and `owned_by` only, no capabilities, so facts come from the model pages (https://developers.openai.com/api/reference/resources/models/methods/list);
  - Gemini `models` resource: `inputTokenLimit`, `outputTokenLimit`, `thinking`, `maxTemperature` (https://ai.google.dev/api/models);
  - Ollama `/api/show`: `thinking.values`/`default` (https://docs.ollama.com/capabilities/thinking);
  - Kimi: doc pages only (no capability endpoint verified).
- List calls only; **no paid completion calls** (that list endpoints are unbilled is **[INFERENCE]**).

**Procedure.**
1. **Fetch** endpoints and pages, keeping URLs, access date and quotes. Credentials never appear in outputs.
2. **Resolve** every `allowed` id and every newly listed vendor id through §5.4.
3. **Diff** resolved facts against endpoint and doc facts.
4. **Classify** each difference:
   - (a) A new id parses and no fact differs → propose an `allowed` line + price-row draft. If the docs verify it, add it to the containing entry's `declares`, which flips it from Inherited to Declared without changing any facts.
   - (b) **Family-version boundary:** the vendor documents that a fact changed *from a version onwards* (it holds for that model version and its successors) → the **minimal** version entry (changed facts only, `unset` where a capability is removed) at the first affected version, with doc quotes.
   - (b′) **Model- or snapshot-specific exception:** the fact holds for one id or snapshot only, or the docs do not say it carries forward → an exact-id `[[models]]` **overlay** for that id. It never goes on a version entry, so siblings and successors are not affected. When the evidence does not establish monotonicity, the steward chooses the overlay (narrower blast radius) and says so in the evidence note.
   - (c) An id that matches no grammar → a new family, or an exact-only `[[models]]` entry.
   - (d) Vendor deprecation/retirement → move to `retired` with a replacement.

   Every steward PR carries an `expected_changes` id list. The resolved-facts diff (§5.9(12)) must show changes for exactly those ids; any other allowed id whose resolved facts or provenance changed fails the gate.
5. **Regenerate** golden request bodies and resolver tables. Golden diffs are part of the patch and are reviewed, never auto-blessed.
6. **Run the gates** (below).
7. **Cross-family agent review** of the patch and evidence (a reviewer from a different model family, as with the `gpt-review*.md` rounds).
8. **Escalate** to the operator only when user-visible behaviour or cost changes (ruling B7d):
   - a price row changes;
   - a `default_model` changes;
   - a pinned or default model's resolved behaviour changes (sampling dropped, off switch lost, ceiling lowered, effort default changed);
   - a pinned model is deprecated or retired.

   Everything else lands after agent review.

**Outputs.**
- A patch touching only the three data files and generated goldens; code changes are out of scope, and a new `OffKind` becomes an ordinary story.
- An evidence note: per change, quote + URL + access date + endpoint excerpt.
- The golden diff and gate log.
- The escalation list, if any.

**Gates the patch must pass.**
- Extended `check-model-currency` (§5.4 cross-checks);
- the profile-parse and grammar-unambiguity crate test;
- resolver tables, goldens and doc-derived negatives (§5.9 1–3, 8);
- `check-judge-config`;
- `check-serde-error-handling`.

**How proposals are tested.** Every new version entry must add its doc-derived negative rows (e.g. "6.0 never sends `disabled`"). A changed golden must show the exact wire delta in review. An `Inherited` id's first live use is a normal call whose `InvalidRequest` count is watched (§5.4 mitigations).

## 6. Worked examples

**A — `claude-opus-6-0`, zero profile change.**
1. Vendor releases Claude Opus 6.0; the steward fetches it and finds no differing fact.
2. `claude-opus-6-0` parses under `claude-opus-{major}[-{minor}][-{date8}]` as `(6, 0)`.
3. The greatest entry ≤ 6.0 is `5.5`.
4. Merging baseline + 4.6 + 4.7 + 5.0 + 5.5 gives: thinking default-on with no off switch, non-default sampling rejected, no manual budget, 128K, effort default `medium`.
5. `Applied.profile_source = Inherited{from: "claude-opus@5.5"}`, `verified = 2026-11-13`.
6. To pin it, add one `allowed` line (and a price row) and get operator review if it becomes a default or changes cost.

**B — a breaking change found later.** The steward reads that from Opus 6.2, `between_tools` is accepted up to `high`. It adds one entry:

```toml
[[families.claude-opus.versions]]
at = "6.2"
declares = ["claude-opus-6-2"]
thinking = { off = { kind = "up_front_only", value = "between_tools", max_effort = "high" } }   # variant replaced atomically; default_on kept (field-wise)
docs = ["https://platform.claude.com/docs/en/build-with-claude/thinking"]
verified = "<date read>"
```

- 6.0 and 6.1 keep inheriting 5.5.
- 6.2, 6.3 and 7.0 inherit the switch with no further edits.
- Goldens for affected ids change (e.g. the judge-shaped call on `claude-opus-6-2` gains `thinking: between_tools`), and the doc-derived negative "6.0/6.1 never send `between_tools`" is added.
- This changes resolved behaviour for any *pinned* 6.2+ model, so it is escalated to the operator.

## 7. Migration slices (ordered)

Sizes: S < 200, M 200–500, L > 500 production lines **[INFERENCE: estimates]**.

| # | Slice | Size | Notes |
|---|---|---|---|
| **S1** | **Manifest pin → wire model, plus served-model attribution.** Carry the admitted `(provider, model)` from the manifest through admission and the binding into `options.model_id`, with **precedence pin > provider default** (Q6, ruled). Drivers serve any admitted model of their provider (several per provider), so the judge keeps its separately admitted Sonnet pin. The `InferenceCall` TL records the served model. Provider-only declarations stay provider-only. Researcher is then served `claude-sonnet-5-5` as pinned (cost rises, §8) | M | Fixes G9/G12. Touches `maos-bin` admission and bindings. In S1 the provider default is still `ANTHROPIC_DEFAULTS` (S3a moves it) |
| S2 | Usage fixes: fail-closed Anthropic usage; Gemini `candidates + thoughts`; optional `reasoning_tokens` breakdown | S | Independent |
| S3a | Family-first profile file (§5.4): layering, id grammars with load-time unambiguity, cumulative version merge, `ProfileSource`; gate cross-checks; **`default_model` moved from `main.rs` `*_DEFAULTS` into `[providers.<p>]`**, with the currency gate switched from `main.rs` literals to `default_model` (`check_model_currency.rs:24-29`). Request bodies unchanged | M (was S) | Defaults move here rather than in S1: S1 stays a pure selection/correctness fix landable without the file; moving defaults needs the file and the gate change together |
| S3b | Resolver for sampling and the Anthropic off switch, replacing `NO_SAMPLING`/`thinking_off`/`accepts_sampling`. Same total caps, byte-identical goldens; `distill_eval` provenance from the resolver | S | Removes the stop-gap |
| S3c | Fail closed for ids with **no grammar match and no exact entry**, or parsing below a family baseline (no exception; gate rejects below-baseline exact entries); zero-transport test; family-resolution and merge tests §5.9(11); resolved-facts diff tool §5.9(12) | S | Q1 |
| S4 | Intent/`Applied` contract (§5.2) including **`Applied::Unrecorded`** (replay returns `PreS6Replay`, so S4 lands without S6); caller migration with current ceilings; thinking-aware sampling rule; empty-text `Incomplete` with usage for Anthropic thinking-only `max_tokens`. **This slice must also make every caller in the §5.3 table treat `Incomplete{has_text:false}` and empty text as a caller-level failure after accounting:** Researcher and Hello change behaviour (they accept empty text today); Butler, Orchestrator and the judge get explicit typed checks. Each caller's named regression test is in §5.9(10). Also the judge-config schema and `greedy_applied: null` provenance (Q3) | M | Q3–Q5 |
| S5 | OpenAI Responses (`store:false`, incomplete handling incl. empty-text `Ok` with usage, generic `InvalidRequest`) and a Kimi profile on Chat (`max_completion_tokens`, `reasoning_effort`, fixed parameters omitted) | M | |
| S6 | Cassette v2 (§5.7): records `applied`; one-time rewrite with `unrecorded-v1` / seed labels | M | After S4 |
| S7 | Anthropic `2026-10-21` flip as its own reviewed slice, with goldens | S | Q2 |
| S8 | Continuations (§5.6) | L | First multi-turn/tool story |
| S9 | **Model-profile steward** (§5.11): procedure, evidence-note template, golden regeneration, escalation rules; runs outside the runtime | S | Can start any time after S3a; scheduled job optional |

## 8. Risks

- **Inherited profiles are unverified** (operator-accepted risk, 2026-11-13). An inherited id may break on first use. Mitigations: typed `InvalidRequest` with the vendor message, per-`profile_source` rejection counts, `Inherited` provenance in `Applied`/eval, and the steward (§5.11).
- **Grammar mistakes.** An over-broad grammar would capture an unrelated id. Mitigations: load-time and gate pairwise-unambiguity checks, typed placeholders, no substring matching, and steward review of every new family.
- **Stale profiles.** Mitigated by `verified` dates, the steward, and `InvalidRequest` counts per profile.
- **Q6 raises Researcher cost.** Serving the pinned `claude-sonnet-5-5` instead of the default Haiku 5.5 is 20× per token at the ≤100K-prompt tier and 4× above it (`xtask/provider-pricing.toml`). Ceilings are unchanged; spend per call rises. This is consistent with the 2026-11-12 sweep, which moved butler, hello, orchestrator and the default to Haiku but not the Researcher (`xtask/model-currency.toml:42-46`).
- **Time-outs.** Without streaming, high effort risks HTTP timeouts. Keep the current ceilings until streaming exists.
- **Continuation constraints.** Prefix and account binding mean multi-turn state can be silently dropped by the vendor. `Applied` must say so.
- **Version flip.** The `2026-10-21` flip removes temperature lowering on allowed 4.x pins.
- **Review discipline.** Goldens must be reviewed, not re-blessed. The `maos-providers` kloc raise must be measured.

## 9. Operator rulings (2026-11-13) — no open questions remain

Operator, verbatim: *"profile file is better than hardcoding in the code base. other suggestions of q1-q7 are accepted. q6 manifest pin overrides the code base or profile file."*

- **Q1 — RULED: fail closed.** Ids matching no declared family grammar **and** no exact `[[models]]` entry, or parsing below a family baseline (no exception), are refused before any network call. Local Ollama models are exact entries. Rev. 5 refinement: grammar-parsed ids inherit (§5.4) as an accepted risk.
- **Q2 — RULED:** the Anthropic `2026-10-21` migration is a separate slice (S7), after request cleanup and goldens.
- **Q3 — RULED:** the judge contract becomes `determinism = "prefer-greedy"` plus the recorded `Applied`, with `check-judge-config` updated. No determinism guarantee is claimed.
- **Q4 — RULED:** refuse when "off" cannot be honoured for judges and short classifiers; elsewhere any degradation is explicit and recorded. Gemini `minimal` and Ollama's lowest-level fallback count as near-off.
- **Q5 — RULED:** existing total ceilings are kept; workload-specific reasoning allocations come later, from recorded evidence.
- **Q6 — RULED: the manifest pin is authoritative.** It overrides code-base defaults and the profile file. Served-model precedence: **manifest pin > `[providers.<p>].default_model` > `Unconfigured`**. Provider default model ids move from `main.rs` into the profile file (S3a). The judge's separately admitted pin stays. Researcher is served `claude-sonnet-5-5` as pinned (cost note in §8).
- **Q7 — RULED:** `store: false` on Responses calls. This is not a claim that every form of vendor retention is disabled.

## 10. Review and ruling disposition

| Round | Disposition | Where |
|---|---|---|
| Rev. 2 (`gpt-review.md`: 3 P1, 8 P2, plus §2–§5 notes) | All fixed. Pin propagation is S1; multiple admitted pins per provider; enforceable total ceiling; Gemini generateContent vs Interactions; near-off ≠ off; incomplete handling; generic `InvalidRequest`; optional capabilities; Opus 5 effort limit; conditional 8-token risk; latent OpenAI defect; vendor qualifications; simplification and re-ordering | §2, §3, §5.2–§5.9, §7 |
| Rev. 3 (`gpt-review-2.md`: 1 P1, 3 P2) | All fixed. Budgets are applied, never enforced; sampling depends on resolved thinking; `Applied::Unrecorded`; empty `Incomplete` keeps usage | §5.2, §5.3, §5.7, §5.9(1, 7–9), §7 S4 |
| Rev. 4 (`gpt-review-3.md`: 1 P2; `gpt-review-4.md`: 1 P3) | All fixed. The audited per-caller table replaces the false inference; S4 requires typed caller failure with five named tests; Hello consumer cites corrected by the orchestrator; verdict story-ready | §5.3 table, §5.9(10), §7 S4 |
| **rev 5:** operator ruling — family-first profiles | Applied: four layers, data id grammars parsed structurally, cumulative nearest-≤ version merge, `ProfileSource` declared/inherited; worked examples for Opus 6.0 with zero change and a later breaking entry | §1, §5.2, §5.3, §5.4, §6 |
| **rev 5:** Q1 reconciliation | Applied: grammar-parsed ids inherit, ungrammatical ids are refused; the currency allowlist stays the pin policy; per-model cost is one `allowed` line; forward inheritance recorded as an accepted risk with mitigations | §5.4, §8, §9 |
| **rev 5:** AI-agent stewardship | Applied: S9 drift report replaced by the steward procedure (inputs, classification, outputs, gates, cross-family review, B7d escalation) | §5.11, §7 S9 |
| **rev 5:** Q6 ruled | Applied: pin > profile-file `default_model` > `Unconfigured`; defaults leave `main.rs` in S3a; the judge pin is kept; Researcher served Sonnet with cost note | §5.3, §7 S1/S3a, §8, §9 |
| **rev 5:** Q1–Q7 ruled | §9 rewritten as rulings with verbatim words | §9 |
| **rev 6:** 1 P2 two resolution paths | Fixed: a family path (one grammar match + optional exact overlay, never an ambiguity) and an exact-only path (standalone or `inherit`, no version arithmetic); refusal = no grammar **and** no exact entry, or below baseline; reconciled in §1, §5.3, §5.4, §9 and S3c | §1, §5.3, §5.4, §7 S3c, §9 |
| **rev 6:** 2 P2 baseline minimum | Fixed: `baseline_version` with complete facts and evidence; below-baseline ids are refused; allowed-id resolution table (all 25 ids resolve) | §5.4 table, §5.9(11) |
| **rev 6:** 3 P2 merge semantics | Fixed: field-wise record merge, arrays replace, `kind`-tagged variants replace atomically, `unset` clearing, metadata never merges; boundary tests | §5.4, §5.9(11) |
| **rev 6:** 4 P2 provenance precedence | Fixed: exact entry → Declared; else the selected entry's `declares` list → Declared; else Inherited (including undeclared snapshots of declared versions); entries declare exactly their `declares` ids | §5.4, §5.9(11) |
| **rev 6:** 5 P2 steward classification | Fixed: version boundary vs model/snapshot overlay (overlay when monotonicity is unproven); `expected_changes` plus a resolved-facts diff over all allowed ids | §5.11, §5.9(12) |
| **rev 7:** P2 below-baseline exception (`gpt-review-6.md`) | Fixed: the exception is removed everywhere; below-baseline ids are always refused, and a below-baseline exact entry fails the gate; the remedy is lowering the baseline with full facts | §1, §5.3, §5.4, §5.9(11), §7 S3c, §9 |
| **rev 7:** P2 example B schema | Fixed: the 6.2 entry uses the tagged `off = {kind, value, max_effort}` variant with atomic replacement; the resolved 6.0/6.1 vs 6.2+ results are added to the §5.9(11) boundary tests | §6, §5.9(11) |
| **rev 7:** Fable 5.1 qualification | Applied: identical only in the modelled single-turn fields; prefix-check and continuation behaviour differ (prefix-checks page) and belong to S8 | §5.4 table |

Review findings were never rebutted. Rev. 5 applies operator rulings, not review findings.
