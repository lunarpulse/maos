# Story 5.5b: Run the Multi-Provider CI Matrix Across Anthropic, OpenAI, and Ollama

Status: done

dev_model_used: glm-5.1

**Epic:** 5 — Spirit Lifecycle, Hot-Swap, Crash Supervision & Multi-Provider (v0.3 → v1.0)
**Epic state at story open:** `epic-5: in-progress` (Stories 5.1 + 5.2 + 5.3 + 5.4 + 5.5a closed `done`; 5.5c/5.5d/5.5e still `backlog`).
**Story key:** `5-5b-run-the-multi-provider-ci-matrix-across-anthropic-openai-and-ollama`

**Predecessors:**
- **Story 1b.4** (Inference Port + Anthropic driver + IAC telemetry) — the **substrate this story extends**. Concretely:
  - The `InferencePort` trait at `crates/maos-domain/src/ports/inference.rs:19-24` (sync `fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse, InferenceError>`; ADR-010 sync — async callers wrap in `spawn_blocking`).
  - The vendor-neutral domain types `InferenceRequest` / `InferenceResponse` / `InferenceOptions` / `StopReason` / `TokenUsage` / `ProviderAttribution` / `InferenceError` at the same file (lines 27-125) — the contract the new drivers MUST translate into, with NO vendor JSON leaking past the driver crate.
  - The internal `Provider` driver trait at `crates/maos-providers/src/provider.rs:11-14` (`fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse, ProviderError>`) with `ProviderError { Transport | ProviderRejected { status, body } | Serde | Unconfigured }` (line 17-31). **This trait IS the `ProviderDriver` referenced in the epic — Story 5.5b extends it (NOT replaces) and aliases the symbol so epic AC wording matches.**
  - The Anthropic reference driver at `crates/maos-providers/src/anthropic.rs:17-62` (`AnthropicProvider::new(transport, endpoint_url, model_id)` reads `MAOS_ANTHROPIC_API_KEY`; `complete` POSTs `/v1/messages` with `anthropic-version: 2023-06-01` + `x-api-key` headers; pure-function `build_request_body` + `parse_response` tested at lines 154-301).
  - The kernel-side adapter `InferencePortAdapter` at `crates/maos-kernel-core/src/inference/mod.rs:35-146` that performs the `Scope::ProviderInfer { provider }` capability check, emits `FrameKind::InferenceCall = 9` to the Transparency Log, and wraps in IAC round-trip telemetry — **unchanged by this story**; each new driver plugs into the SAME adapter via `Arc<dyn Provider>`.
  - The composition root wiring at `crates/maos-bin/src/main.rs:384-405` (currently single-Anthropic) — Story 5.5b extends this to select a driver per-Spirit from manifest `[providers]` (default = composition-root default = Anthropic when `MAOS_ANTHROPIC_API_KEY` is set, otherwise `UnconfiguredProvider`).
- **Story 0.2** (FR47 vendor-SDK denylist gate). The `check-fr47` xtask at `xtask/src/check_fr47.rs::check_fr47` + `xtask/fr47-vendor-sdk-denylist.toml` (Anthropic / OpenAI / Ollama / Gemini / Bedrock / Azure SDK crate names) + `xtask/fr47-allowlist.toml` (currently empty by design) — the structural alarm that fails the build the moment any new driver pulls in a vendor SDK. **Story 5.5b's OpenAI and Ollama drivers MUST speak REST directly over the existing `IoSubsystemPort::http_post` (same pattern as `AnthropicProvider`); the denylist stays empty-allowlist + denylist additive only if a vendor SDK is genuinely unavoidable (it isn't here — all three providers expose stable REST surfaces).** The denylist already names `openai`, `openai-api-rs`, `async-openai`, `ollama-rs` (lines 12-15 of `fr47-vendor-sdk-denylist.toml`); Story 5.5b proves the denylist is workable by shipping two more drivers without expanding the allowlist.
- **Story 1b.3** (`SandboxConfig` parsing pattern + `[sandbox]` manifest section + strictest-of admission). The pattern Story 5.5b mirrors when adding the NEW `[providers]` manifest section — additive optional fields, validate-then-resolve, `ManifestError::Toml(...)` rejection on unknown values.
- **Story 1b.2** (`Scope::ProviderInfer { provider: String }` at `crates/maos-domain/src/invariants/i1.rs:72`; the kernel-side scope matching at `crates/maos-kernel-core/src/inference/mod.rs:84-89` requires `Scope::ProviderInfer { provider } if provider == provider_id`). **Story 5.5b broadens the operator's ability to issue `ProviderInfer` for `"openai"` or `"ollama"` providers — the scope shape is unchanged; only the values stored change.**
- **Story 2.4** (`spirit_test` SDK seed + `RegressionCorpus` + `SpiritClass` enum + `spirit_test_smoke` CI gate). The fixture suite **this matrix runs against**: `crates/maos-spirit-sdk/src/spirit_test/regression.rs::RegressionCorpus::load` and the per-class skeleton at `crates/maos-spirit-sdk/src/spirit_test/regression.rs`. Story 5.5b's multi-provider fixture suite extends the same shape but with **provider-orthogonal prompts** (the same prompt → 3 provider responses → normalized comparison).
- **Story 5.4** (signed CRL pipeline + monotonic_now_ns discipline + `try_send` + `cap_audit::record_drop()` ADR-030 audit-channel pattern + zero-alloc serde-visitor + `xtask check-pub-field-constructors` gate). **Disciplines Story 5.5b inherits:**
  - `monotonic_now_ns()` for ALL new journal/TL emit timestamps. NEVER `wall_clock_now_ns()`. (Story 5.4 Review Finding §1366 — closed pattern.)
  - `try_send` + `cap_audit::record_drop()` on saturation for any new audit-channel emit. NEVER `.await` on audit channels. (ADR-030, Story 1b.2 lesson §6.)
  - Every new `pub` field on a serde struct gets a `#[doc = "Construct via ::new ..."]` annotation matched by the constructor — the `xtask check-pub-field-constructors` gate enforces it (Epic 4 retro §A4).
  - `serde_json::to_vec(&x).map_err(...)` — NEVER `.unwrap_or_default()` on serde paths. (Story 5.4 Review Finding §1373 — closed pattern.)
  - JoinHandle self-prune on completion for any new async task. (Story 5.4 Review Finding §1368 — closed pattern.)
- **Story 5.5a** (sandbox tier T3 + `--network=none` default for T3 containers + smoke-t3-sandbox-5 arm). **Air-gap dovetail:** Story 5.5a's T3 containers default to `--network=none`; Story 5.5b's air-gapped Ollama configuration validates that the substrate runs end-to-end with zero outbound provider calls. Together they close NFR-Ops-12 ("substrate runs on disconnected hosts") from two angles — T3 enforces network isolation at the container boundary; this story validates that the Inference Port path has no implicit egress when Ollama is the configured provider. The full network-namespace structural validation lands at Story 9.4 (referenced in AC4 below).

**Carry-forward closures expected at story open** (Story 5.5a review-patch items + Story 5.4 carryovers the dev agent must verify CLOSED before the first commit on 5.5b):

- **Story 5.5a Review Findings table** — verify the post-review state at `_bmad-output/implementation-artifacts/5-5a-sandbox-tier-t3-container-isolation-via-docker-podman.md` Review Findings section: any `open` row blocks 5.5b dev-start; any `deferred → Story 5.5b` row IS picked up here (none expected — the 5.5a successor table at line 32 marks 5.5b as orthogonal). Audit the table; if anything was deferred to 5.5b, add a row to this story's Review Findings table forward-referencing the resolution path.
- **Story 5.5a §SandboxBlock-emit-via-probe-sidecar** — flagged for awareness only; multi-provider has no container-runtime path.
- **Story 5.4 §1370 `ColdSwap bypasses scheduler.load()`** — same forward-shaped Epic 6 dependency. Multi-provider does NOT touch the cold-swap path; flagged for awareness.
- **Story 5.4 §1366 `monotonic_now_ns` discipline** — closed; Story 5.5b follows it. Any new journal/TL emit (e.g. `LifecycleEvent::ProviderSwitched = 18`) MUST use `monotonic_now_ns()`.
- **Story 5.4 §1373 `serde_json::to_vec().unwrap_or_default()`** — closed pattern; Story 5.5b's new driver request/response serialization paths and the multi-provider report JSON writer all propagate serde errors (no silent drop).
- **Story 5.4 §A4 `check-pub-field-constructors`** — every new `pub` field on `ProvidersConfig` / `ProviderConfig` (and any other new serde-bearing structs) carries the `#[doc = "Construct via ::new ..."]` annotation and a matching `impl ::new` constructor.
- **Story 1b.4 doc-comment overreach** — `crates/maos-domain/src/ports/inference.rs:4-5` reads "Streaming (`stream`) and embeddings (`embed`) are deferred to Story 5.5b." This story does **NOT** add `stream` or `embed` (out of scope per epic AC1 — the AC is about three drivers running the existing `complete` surface). Update the doc-comment to defer streaming/embeddings to a v0.5+ follow-up (cross-ref the `provider/stream` open item carried forward to implementation at `_bmad-output/planning-artifacts/epics/open-items-carried-forward-to-implementation.md`).

**Successor stories in Epic 5:**
- **5.5c** (MCP client + ACP server) — orthogonal to 5.5b at the trait surface, but the OpenAI driver's "function-calling" surface is the gateway through which v0.5+ tool-use Spirits cross into MCP tool servers. Story 5.5b does NOT implement tool-call dispatch (epic AC1 is about `complete`-equivalent semantics across providers — tool-call equivalence is a v0.5+ extension); the structural seam (provider attribution carrying provider-specific stop-reason variants via `StopReason::ProviderStop(String)`) is forward-shaped. Documented in Dev Notes.
- **5.5d** (Spirit Registry over MCP-Streamable-HTTP) — orthogonal. Story 5.5d's `manifest.toml` packaging discipline (per-Spirit signing key etc.) interacts with the NEW `[providers]` manifest section but Story 5.5b only adds the section; the registry-side admission of provider-bound manifests is Story 5.5d's concern.
- **5.5e** (§13.1 rust-inproc measurement gate) — orthogonal. The §13.1 J1+J4 measurement workloads do not invoke the Inference Port; multi-provider does not affect IPC latency.
- **Epic 7 Story 7.3** (full ComplianceClaim envelope at admission) — Story 5.5b's `[providers]` manifest section adds `provider_endpoint_pin: Option<String>` fields that align with the **frozen `ProviderEndpointPin` schema field from Story 1b.4 ComplianceClaim freeze** at `crates/maos-domain/src/compliance/...` — admission verification of the provider-endpoint match against the signed envelope arrives at Story 7.3. Story 5.5b adds the *manifest declaration*; 7.3 wires the *envelope verification*.
- **Epic 9 Story 9.4** (operator surface — full air-gapped network-namespace isolation test) — Story 5.5b's AC4 ("substrate runs end-to-end with Ollama and zero outbound provider calls") is **observability-grade** (Layer-1.5 smoke + journal-grep assertion); the full structural egress-prevention test (running the substrate inside `unshare --net` / `ip netns` and asserting zero packets leave) is Story 9.4. Story 5.5b's smoke arm output is the input fixture for 9.4's structural validator.

<!-- Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an **operator who refuses provider lock-in — and a Spirit author who needs to declare provider preference per Spirit without rewriting code — and an evaluator who needs to observe that the substrate ACTUALLY runs against three providers in CI, not just three driver crates that compile**,

I want **the v0.5-α multi-provider substrate that (a) ships two NEW REST-direct provider drivers at `crates/maos-providers/src/openai.rs` and `crates/maos-providers/src/ollama.rs` — sibling modules of the existing `anthropic.rs` (each implementing the EXISTING `Provider` trait at `crates/maos-providers/src/provider.rs::Provider` with the same `IoSubsystemPort`-injected transport, env-gated credential, and pure `build_*_request_body` + `parse_*_response` discipline; the `ProviderDriver` symbol referenced in the epic AC is introduced as a `pub use Provider as ProviderDriver;` re-export at `crates/maos-providers/src/lib.rs` for cross-document terminology alignment — **NOT a new trait**, additive to the existing surface so 1b.4's `InferencePortAdapter` continues to consume `Arc<dyn Provider>` unchanged); (b) adds the NEW `[providers]` manifest section at `crates/maos-kernel-core/src/security/manifest.rs::ProvidersSection` carrying `{primary: ProviderConfig, fallback: Option<Vec<ProviderConfig>>}` where `ProviderConfig { id: String, endpoint_url: Option<String>, model_id: Option<String>, provider_endpoint_pin: Option<String> }` — the `id` field is the dispatch key matched against `Scope::ProviderInfer { provider }`; the `endpoint_url` and `model_id` fields override composition-root defaults; the `provider_endpoint_pin` field is the optional SHA-256 of the endpoint URL bound for ComplianceClaim verification at admission (the binding code lives in Story 7.3; Story 5.5b only PARSES the field and asserts shape); the manifest section is OPTIONAL (manifests without `[providers]` use composition-root default = Anthropic when configured, else `UnconfiguredProvider`); unknown provider IDs in `primary.id` → `ManifestError::Toml("providers.primary.id '<x>' unsupported at v0.5-α (allowed: anthropic, openai, ollama)")`; (c) adds the NEW `MultiProviderRouter` at `crates/maos-kernel-core/src/inference/router.rs::MultiProviderRouter { providers: BTreeMap<String, Arc<dyn Provider>>, default_id: Option<String> }` — held by `InferencePortAdapter` (replacing the single `Arc<dyn Provider>` field at `inference/mod.rs:36` with `Arc<MultiProviderRouter>`; the existing `provider_id: String` field is removed because per-request routing now uses the SCB's resolved provider ID); on every `complete` call, the adapter (i) reads the calling Spirit's SCB.manifest.providers.primary.id (set at admission), (ii) calls `router.dispatch(provider_id)?` to obtain the `Arc<dyn Provider>`, (iii) routes through it; on `ProviderError::Transport` or `ProviderError::ProviderRejected { status: 429 | 500..=599, .. }` the adapter walks `fallback` in order, retrying each — fallback exhaustion returns the LAST error (not a synthetic aggregate); (d) ships the NEW `.github/workflows/multi-provider.yml` matrix workflow (NOT a discipline.yml job — fresh sibling per the architecture's CI hierarchy convention; mirrors the structure of `journal-aggregate.yml`/`journal-append.yml` rather than being absorbed into the 50+ discipline jobs) that runs a matrix `provider: [anthropic, openai, ollama]` × `os: [ubuntu-latest]` (macOS/Windows deferred per the same v0.5-α platform-availability discipline as Story 5.5a — Linux is the baseline); each cell runs the NEW `crates/maos-providers/tests/fixtures/multi-provider-v0/` fixture suite (10 normalized prompt-response cases — single-turn `complete`-only; the suite is **provider-orthogonal by construction** since the JSON-fixture-replay test mode never makes a live call); cells then upload `tests/reports/multi-provider-<sha>-<provider>.json`; a final `report-aggregate` job downloads the three per-provider reports, normalizes into `tests/reports/multi-provider-<sha>.json` with one row per `(fixture_id, provider)` pair, and runs `cargo run -p xtask -- check-multi-provider-drift --threshold 10` (the NEW xtask command) which flags any `(fixture_id, provider)` cell where the provider's response-length, token-usage, or stop-reason deviates ≥10% from the per-fixture **median across the three providers** (the threshold is computed inside the xtask, not in the workflow YAML — the threshold knob stays in code where it's testable); drift outliers are surfaced as a GitHub Actions `notice` annotation (NOT a failure) on PRs and as a structured row in the aggregated report; the aggregate job uploads the merged report as a workflow artifact for 90 days per the Story 0.3 bench-results retention pattern; (e) extends the EXISTING `xtask` toolset with `cargo run -p xtask -- check-multi-provider-drift --report <path> --threshold <pct> [--json]` at `xtask/src/check_multi_provider_drift.rs` (NEW file); unit-tests at `xtask/src/tests/check_multi_provider_drift_tests.rs` exercise the median + delta-from-median computation against canned report JSON; integration tests at `xtask/tests/check_multi_provider_drift_integration.rs` invoke the xtask binary against fixture reports per the EXISTING xtask test pattern (see `xtask/tests/check_corpus_integration.rs` for shape); (f) adds the NEW `LifecycleEvent::ProviderSwitched = 18` variant on the `#[repr(u8)]` `crates/maos-domain/src/invariants/i10.rs::LifecycleEvent` enum (additive — preserves all 0..17 discriminants including Story 5.4's `Upgrade = 15` / `Revoked = 16` and Story 5.5a's `SandboxApplied = 17`); the journal entry's payload is `{spirit_id, from_provider, to_provider, manifest_path, applied_at_ns}` (timestamp via `monotonic_now_ns()` per Story 5.4 discipline) emitted EXACTLY ONCE when a Spirit's `[providers].primary.id` changes between two consecutive `admit_spirit` calls for the same `spirit_id`; the journal entry is the operator-visible audit trail for "this Spirit switched from Anthropic to OpenAI at this time"; (g) the NEW **`MAOS_ONE_SHOT=smoke-multi-provider-5` arm** at `crates/maos-bin/src/main.rs` (additive on the existing match block; known-modes list at line 2032 EXTENDS to include `smoke-multi-provider-5`) walking the multi-provider substrate end-to-end on a developer host using **fixture-replay providers** (NOT live API calls — the smoke arm uses three `FixtureReplayProvider` instances wired to canned JSON responses at `crates/maos-providers/tests/fixtures/multi-provider-v0/smoke/` so the arm runs deterministically on any CI runner including those without API keys) printing one JSON line per surface: `{"step":1,"surface":"router_dispatch","outcome":"ok","providers_registered":["anthropic","openai","ollama"],"default":"anthropic"}` → swap the Spirit's `[providers].primary.id` from `"anthropic"` to `"openai"` via a synthetic re-`admit_spirit` call, assert `LifecycleEvent::ProviderSwitched` entry exists in the journal in the time window, print `{"step":2,"surface":"provider_switch","outcome":"journaled","from":"anthropic","to":"openai","journal_offset":<n>}` → run a synthetic `complete()` call against each of the three FixtureReplayProviders in series, assert each returns a vendor-neutral `InferenceResponse` with the expected `provider_attribution.provider_id`, print `{"step":3,"surface":"provider_complete","outcome":"ok","providers_exercised":["anthropic","openai","ollama"]}` → simulate `ProviderError::ProviderRejected { status: 503, .. }` on the primary `anthropic` driver, assert the router walks fallback to `openai`, print `{"step":4,"surface":"fallback","outcome":"ok","primary":"anthropic","fallback_used":"openai","reason":"503"}` → invoke `check-multi-provider-drift` against a canned report fixture with one outlier, assert one drift row, print `{"step":5,"surface":"drift_check","outcome":"flagged","outliers":1}` → exit 0 after printing 5 JSON lines; the smoke arm is the Layer-1.5 observability bridge for Story 5.5b that smoke-epic-4 (Story 5.1), smoke-spirit-5 (Story 5.1), smoke-supervision-5 (Story 5.3), smoke-upgrade-revoke-5 (Story 5.4), and smoke-t3-sandbox-5 (Story 5.5a) are for their respective stories — closes Lunarpulse's evaluation discipline per `[[feedback_lunarpulse_observability_preference]]` ("when can I observe actual behavior beats coverage%")**,

so that **(a) the FR3 contract ("Operator can configure provider drivers (Anthropic, OpenAI, Gemini, Kimi, local-LLM via Ollama, air-gapped Bedrock) per Spirit") gets its v0.5-α three-provider floor — every PR runs the `multi-provider.yml` matrix; provider drift is detected mechanically by the drift check, not by developer vigilance; (b) the FR47 contract ("Spirit obtains all model inference exclusively via the kernel-provided Inference Port; Spirit binaries do not import vendor LLM SDKs directly") stays structurally closed — the existing `check-fr47` gate continues to enforce the denylist, and Story 5.5b's two new drivers prove the REST-direct pattern scales to three providers without re-introducing vendor SDKs; (c) the ADR-005 "≥3 providers in CI by v0.5" gate is **mechanically closed** — when an evaluator runs `gh run list --workflow=multi-provider.yml` they OBSERVE three matrix cells passing on every commit to main; the gate is not aspirational; (d) the Risk 7 mitigation ("Provider lock-in / concentration risk") gets its v0.5 acceptance — the matrix proves three providers are drop-in replacements; (e) the §8.0 hermes-tenant positioning's "operator-substitutable infrastructure" claim gets its v0.5-α concrete demonstration — switching a Spirit from Anthropic to OpenAI is a manifest-only operation; no recompile, no rebuild, no kernel touch; (f) the NFR-Ops-12 air-gapped-deployment commitment gets its v0.5-α observability leg — `smoke-multi-provider-5` with Ollama-as-primary + outbound-disabled is observable in one command; the structural egress validation arrives at Story 9.4; (g) the §13.1 measurement gate (Story 5.5e) has stable provider-driver baselines to measure against — without three drivers, the J1+J4 P95 baselines for the §13.1 ADR would be Anthropic-only and the measurement wouldn't generalize; (h) the operator-facing diagnostic surface gets a `ProviderSwitched` journal event — when Story 9.1's `maosctl audit query --spirit <id> --lifecycle` ships, the operator can grep for provider-switch history without manual log archeology; (i) the Story 0.2 FR47 lint gate gets its v0.5-α stress test — three driver crates in the workspace prove the denylist + empty-allowlist contract is sustainable; if anything would break that contract, it would have broken here, and we'd know now rather than at v1.0; (j) when an evaluator runs `MAOS_ONE_SHOT=smoke-multi-provider-5 cargo run -p maos-bin`, they OBSERVE the router dispatch, the provider switch journaling, three providers serving completion calls, the fallback path activating on synthetic 503, and the drift-check flagging an outlier IN ONE COMMAND — the substrate's multi-provider claim is no longer "we have a trait" but "we have three drivers, a matrix, a router, fallback, and a drift detector, demonstrated"**.

## What this story IS

- **NEW `crates/maos-providers/src/openai.rs` driver** — REST-direct against the OpenAI Chat Completions API (`/v1/chat/completions`). Translates `InferenceRequest` → `{"model": "<model_id>", "messages": [{"role": "user", "content": prompt}], "max_tokens": req.options.max_tokens, "temperature": req.options.temperature}` and parses `{"choices": [{"message": {"content": ...}, "finish_reason": "stop|length|content_filter|tool_calls"}], "usage": {"prompt_tokens": n, "completion_tokens": n}}` → `InferenceResponse`. The `finish_reason` maps: `"stop"` → `StopReason::StopSequence`; `"length"` → `StopReason::MaxTokens`; anything else → `StopReason::ProviderStop(other)`. Construction reads `MAOS_OPENAI_API_KEY` (mirrors the `MAOS_ANTHROPIC_API_KEY` env pattern at `anthropic.rs:36-37`); headers `Authorization: Bearer <key>` + `content-type: application/json`. `ProviderAttribution { provider_id: "openai", endpoint_url: <self.endpoint_url>, model_id: Some(<self.model_id>) }`. Default `endpoint_url = "https://api.openai.com"`; default `model_id = "gpt-4o-mini"` (v0.5-α floor; operator override via manifest `[providers].primary.model_id`). Pure-function `build_openai_request_body` + `parse_openai_response` tested against fixtures at `crates/maos-providers/tests/fixtures/openai_*.json` per the Anthropic precedent at lines 154-301 of `anthropic.rs`. `with_api_key` test-helper (`#[doc(hidden)]`) mirrors `AnthropicProvider::with_api_key` at lines 47-60.
- **NEW `crates/maos-providers/src/ollama.rs` driver** — REST-direct against the Ollama Chat API (`/api/chat`). Translates `InferenceRequest` → `{"model": "<model_id>", "messages": [{"role": "user", "content": prompt}], "stream": false, "options": {"num_predict": req.options.max_tokens, "temperature": req.options.temperature}}` (Ollama streams by default; we set `stream: false` for v0.5-α `complete`-only). Parses `{"message": {"content": ...}, "done_reason": "stop|length|...", "prompt_eval_count": n, "eval_count": n}` → `InferenceResponse`. The `done_reason` maps: `"stop"` → `StopReason::StopSequence`; `"length"` → `StopReason::MaxTokens`; anything else → `StopReason::ProviderStop(other)`. **No API key** — Ollama runs locally (operator declares `endpoint_url = "http://localhost:11434"` by default; the MAOS_OLLAMA_URL env-var overrides). Construction: `OllamaProvider::new(transport, endpoint_url, model_id) -> Result<Self, ProviderError>` returns `Ok` unconditionally (no env-gated credential — the Unconfigured path is reached only at request time if the local Ollama instance is unreachable, which surfaces as `ProviderError::Transport`). Default `model_id = "llama3.1:8b"` (v0.5-α floor; small model that fits on a developer laptop). `ProviderAttribution { provider_id: "ollama", endpoint_url, model_id: Some(model_id) }`. Pure-function tests against fixtures at `crates/maos-providers/tests/fixtures/ollama_*.json`.
- **NEW `FixtureReplayProvider` at `crates/maos-providers/src/fixture_replay.rs`** (test-only helper exposed under `#[cfg(any(test, feature = "fixture_replay"))]`; gated by a new `fixture_replay` feature in `Cargo.toml`) — implements `Provider` and serves `InferenceResponse` from a `Vec<InferenceResponse>` ring buffer in declaration order. **Purpose:** the multi-provider CI matrix + smoke arm use `FixtureReplayProvider` instances (one per provider ID) loaded from `crates/maos-providers/tests/fixtures/multi-provider-v0/` JSON files so the matrix never depends on live API keys in CI. The fixture-replay path is **the only place provider responses are canned in production code paths** — driver-internal tests stay pure-function (no replay needed); only the matrix runner and the smoke arm use replay. Documented in the driver module's doc comments + the §Dev Notes Decision Register.
- **EXTENDED `crates/maos-providers/src/lib.rs`** — re-export the new drivers + the trait alias:
  ```rust
  pub mod anthropic;
  pub mod openai;       // NEW
  pub mod ollama;       // NEW
  pub mod provider;
  pub mod fixture_replay; // NEW (gated by feature `fixture_replay`)

  pub use anthropic::AnthropicProvider;
  pub use openai::OpenAiProvider;       // NEW
  pub use ollama::OllamaProvider;       // NEW
  pub use provider::{Provider, ProviderError};
  pub use provider::Provider as ProviderDriver;  // epic-AC terminology alignment
  ```
  **`ProviderDriver` is NOT a new trait — it is a type alias re-export of the existing `Provider` trait.** Doc comment on the re-export explains the alignment: "The epic spec refers to `ProviderDriver`; the canonical name in this crate is `Provider` (introduced at Story 1b.4). They are the same trait; the re-export exists to make epic-AC text readable without renaming the trait that already has consumers." Rationale: renaming the trait would cascade through `crates/maos-kernel-core/src/inference/mod.rs:25` (consumer) + every existing test. The alias is the least-disruptive resolution.
- **NEW `[providers]` manifest section** at `crates/maos-kernel-core/src/security/manifest.rs::ProvidersSection`:
  ```rust
  // crates/maos-kernel-core/src/security/manifest.rs (additive on existing module)
  #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
  #[maos_attrs::i9_exempt(
      reason = "manifest data; parsed-then-dropped at admission, no kernel persistence"
  )]
  pub struct ProvidersSection {
      #[doc = "Construct via [`ProvidersSection::new`] to enforce id/endpoint validation; struct literals bypass schema checks."]
      pub primary: ProviderConfig,
      #[doc = "Construct via [`ProvidersSection::new`] to enforce id/endpoint validation; struct literals bypass schema checks."]
      #[serde(default)]
      pub fallback: Vec<ProviderConfig>,
  }

  #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
  #[maos_attrs::i9_exempt(
      reason = "manifest data; parsed-then-dropped at admission, no kernel persistence"
  )]
  pub struct ProviderConfig {
      #[doc = "Construct via [`ProviderConfig::new`] to enforce non-empty id."]
      pub id: String,                              // "anthropic" | "openai" | "ollama"
      #[doc = "Construct via [`ProviderConfig::new`] to enforce non-empty id."]
      pub endpoint_url: Option<String>,            // overrides composition-root default
      #[doc = "Construct via [`ProviderConfig::new`] to enforce non-empty id."]
      pub model_id: Option<String>,                // overrides composition-root default
      #[doc = "Construct via [`ProviderConfig::new`] to enforce non-empty id."]
      pub provider_endpoint_pin: Option<String>,   // SHA-256 hex; Story 7.3 ComplianceClaim verification
  }
  ```
  Validation in the existing `validate()` method on the manifest root:
  - `providers.primary.id ∉ {"anthropic", "openai", "ollama"}` → `ManifestError::Toml("providers.primary.id '<x>' unsupported at v0.5-α (allowed: anthropic, openai, ollama)")`.
  - `providers.primary.endpoint_url.as_ref().map(|u| u.is_empty()).unwrap_or(false)` → `ManifestError::Toml("providers.primary.endpoint_url must not be empty if present")`.
  - `providers.primary.provider_endpoint_pin.as_ref().map(|p| !is_hex_sha256(p)).unwrap_or(false)` → `ManifestError::Toml("providers.primary.provider_endpoint_pin must be 64-char hex SHA-256")`. Helper `is_hex_sha256(s) -> bool` at the same module checks `s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())`.
  - Each `fallback[i]` validated by the same rules.
  - `providers.primary.id == fallback[i].id` → warn-but-accept (operator chose to retry the same provider; documented as legal — e.g., retry-on-rate-limit pattern; Story 5.5b does not gate this).
  Manifest fixtures extend `crates/maos-kernel-core/tests/fixtures/manifest/providers/`:
  - `well-formed/primary-anthropic-no-fallback.toml`
  - `well-formed/primary-openai-with-anthropic-fallback.toml`
  - `well-formed/primary-ollama-air-gapped.toml` (no fallback; declares `[providers].primary = { id = "ollama", endpoint_url = "http://localhost:11434" }`)
  - `malformed-rejected/unsupported-id.toml` (declares `id = "kimi"`; expect `ManifestError::Toml`)
  - `malformed-rejected/empty-endpoint.toml` (declares `endpoint_url = ""`)
  - `malformed-rejected/bad-pin.toml` (declares `provider_endpoint_pin = "not-hex"`)
- **NEW `MultiProviderRouter` at `crates/maos-kernel-core/src/inference/router.rs`**:
  ```rust
  // crates/maos-kernel-core/src/inference/router.rs
  use std::collections::BTreeMap;
  use std::sync::Arc;

  use maos_domain::ports::inference::{InferenceRequest, InferenceResponse};
  use maos_providers::{Provider, ProviderError};

  /// Routes inference requests to the per-Spirit-resolved provider driver.
  ///
  /// Holds the closed set of provider IDs registered at composition root
  /// (e.g. `["anthropic", "openai", "ollama"]`). Per-request routing reads
  /// the SCB's `manifest.providers.primary.id` (set at admission) and
  /// dispatches.
  #[maos_attrs::i9_exempt(
      reason = "inference port adapter aggregate; holds Arc references to driver instances — not independently-mutable state"
  )]
  pub struct MultiProviderRouter {
      providers: BTreeMap<String, Arc<dyn Provider>>,
      default_id: Option<String>,
  }

  impl MultiProviderRouter {
      pub fn new(providers: BTreeMap<String, Arc<dyn Provider>>, default_id: Option<String>) -> Self {
          Self { providers, default_id }
      }

      /// Look up the driver for `provider_id`, falling back to the
      /// composition-root default if `provider_id` is None or empty.
      pub fn dispatch(&self, provider_id: Option<&str>) -> Result<Arc<dyn Provider>, RouterError> {
          let key = provider_id.filter(|s| !s.is_empty()).or(self.default_id.as_deref());
          match key {
              Some(id) => self.providers.get(id).cloned()
                  .ok_or_else(|| RouterError::UnknownProvider(id.into())),
              None => Err(RouterError::NoDefault),
          }
      }

      /// Walk `primary` → `fallback[0]` → `fallback[1]` → ... returning the
      /// last error if every provider in the chain fails. Each provider
      /// invocation is tried at most once (no retry-per-provider here —
      /// per-provider retry policy belongs to the driver if at all).
      pub fn dispatch_with_fallback(
          &self,
          primary_id: &str,
          fallback_ids: &[String],
          req: &InferenceRequest,
      ) -> Result<InferenceResponse, ProviderError> {
          let mut chain: Vec<&str> = std::iter::once(primary_id)
              .chain(fallback_ids.iter().map(String::as_str))
              .collect();
          let mut last_err: Option<ProviderError> = None;
          for id in chain.drain(..) {
              let driver = match self.providers.get(id) {
                  Some(d) => d.clone(),
                  None => {
                      last_err = Some(ProviderError::Unconfigured);
                      continue;
                  }
              };
              match driver.complete(req) {
                  Ok(resp) => return Ok(resp),
                  Err(e) if Self::is_retriable(&e) => {
                      last_err = Some(e);
                      continue;
                  }
                  Err(e) => return Err(e),  // non-retriable: short-circuit
              }
          }
          Err(last_err.unwrap_or(ProviderError::Unconfigured))
      }

      fn is_retriable(err: &ProviderError) -> bool {
          match err {
              ProviderError::Transport(_) => true,
              ProviderError::ProviderRejected { status, .. } => *status == 429 || (500..=599).contains(status),
              ProviderError::Unconfigured | ProviderError::Serde(_) => false,
          }
      }

      pub fn registered_ids(&self) -> Vec<String> {
          self.providers.keys().cloned().collect()
      }
  }

  #[derive(Debug, Clone, thiserror::Error)]
  pub enum RouterError {
      #[error("unknown provider id '{0}' (not registered at composition root)")]
      UnknownProvider(String),
      #[error("no default provider configured")]
      NoDefault,
  }
  ```
  Tests (`crates/maos-kernel-core/src/inference/router.rs::tests` inline):
  - `dispatch_returns_primary_when_registered` — happy path.
  - `dispatch_returns_default_when_none_provided` — falls through to `default_id`.
  - `dispatch_returns_unknown_provider_when_missing` — `RouterError::UnknownProvider`.
  - `dispatch_returns_no_default_when_no_provider_id` — `RouterError::NoDefault`.
  - `dispatch_with_fallback_first_provider_succeeds_no_walk` — primary OK; fallback not invoked.
  - `dispatch_with_fallback_503_walks_to_secondary` — primary returns `ProviderRejected{ status: 503 }`; secondary returns Ok.
  - `dispatch_with_fallback_400_does_not_walk` — primary returns `ProviderRejected{ status: 400 }` (client error); router short-circuits; secondary NOT invoked. (Critical: non-retriable errors do NOT walk fallback — a Spirit's bad prompt should not silently rebroadcast to other providers.)
  - `dispatch_with_fallback_all_fail_returns_last_error` — primary 503 → secondary 502 → return last error.
  - `dispatch_with_fallback_transport_walks` — `ProviderError::Transport` is retriable.
  - `is_retriable_serde_false` — Serde errors NEVER walk fallback (deterministic bug in the driver, not a transient issue).
  - The MockProvider helper from `crates/maos-kernel-core/src/inference/mod.rs::tests::MockProvider` is REUSED — do NOT duplicate (per Story 5.4 §1366 anti-duplication discipline). Extend the MockProvider to support `set_next_response(Result<InferenceResponse, ProviderError>)` and use one MockProvider per registered ID.
- **EXTENDED `InferencePortAdapter` at `crates/maos-kernel-core/src/inference/mod.rs`** — the existing struct gets a one-line change:
  ```rust
  // BEFORE (line 36-37)
  provider: Arc<dyn Provider>,
  provider_id: String,

  // AFTER (Story 5.5b)
  router: Arc<MultiProviderRouter>,
  // provider_id is removed; per-request lookup uses the SCB's resolved provider ID.
  ```
  The `complete` method (lines 92-145) is restructured:
  1. Extract `provider_id` from the request — at v0.5-α this is read from a NEW `InferenceRequest::provider_id: Option<String>` field (additive on the existing struct, see below).
  2. Optionally — if the composition root threads the SCB into the adapter — read the SCB's `manifest.providers.primary.id` instead. **For v0.5-α the InferenceRequest-side field is the authoritative path** (composition-root SCB threading is a forward-shape seam documented in Dev Notes; full SCB-side resolution arrives when subprocess-form Spirits ship at Epic 6).
  3. The capability-check call (`check_capability`) signature stays unchanged; it already takes `provider_id: &str`. Pass the resolved provider_id.
  4. Replace `self.provider.complete(&req)` with `self.router.dispatch_with_fallback(&primary_id, &fallback_ids, &req)`. The `fallback_ids` come from the same source as `primary_id` — at v0.5-α via the NEW `InferenceRequest::fallback_provider_ids: Vec<String>` field (additive, default empty).
  5. Telemetry instrumentation stays unchanged.
  6. The Transparency Log `intent` string includes BOTH provider IDs: `format!("infer:{provider_id}->{actual_provider_id}:{model_id}")` where `actual_provider_id` is the ID of the provider that ULTIMATELY served the response (read from `response.provider_attribution.provider_id`); this lets operators audit fallback events via TL grep.
  Tests in `crates/maos-kernel-core/src/inference/mod.rs::tests`:
  - `mock_provider_round_trip_logs_inference_call` — UPDATED to use `MultiProviderRouter` with one provider; assertion shape unchanged.
  - `fallback_503_routes_to_secondary` (NEW) — primary returns 503; secondary returns Ok; assert TL `intent` contains `"->openai"` indicating the actual server was secondary.
  - `capability_denied_without_token` — UPDATED to thread through router; the capability check happens BEFORE router dispatch.
- **EXTENDED `InferenceRequest`** at `crates/maos-domain/src/ports/inference.rs:27-37` — additive fields:
  ```rust
  pub struct InferenceRequest {
      pub spirit_pid: u32,
      pub capability_token: CapabilityToken,
      pub prompt: String,
      pub options: InferenceOptions,
      pub provider_id: Option<String>,        // NEW — v0.5-α dispatch key; None → composition-root default
      pub fallback_provider_ids: Vec<String>, // NEW — v0.5-α fallback chain; empty = no fallback
  }
  ```
  Per Story 0.2's pub-field-constructor gate, every new pub field gets a `#[doc = "Construct via [`InferenceRequest::new`] ..."]` annotation. **Add an `impl InferenceRequest { pub fn new(...) -> Self }` constructor that accepts all fields and validates `prompt.is_empty()` is allowed (matches Story 1b.4 contract — prompt can be empty for embedding-style requests).** The fields are additive (default Option/Vec), so existing call sites still compile as long as they use struct literal construction; new call sites use `::new`.
- **EXTENDED `LifecycleEvent` at `crates/maos-domain/src/invariants/i10.rs`** — adds `ProviderSwitched = 18` discriminant on the `#[repr(u8)]` enum (preserves 0..17 including Story 5.5a's `SandboxApplied = 17`). The journal entry payload is the NEW `ProviderSwitchedPayload { spirit_id: String, from_provider: String, to_provider: String, manifest_path: String, applied_at_ns: u64 }` (in the same module's payload-type registry; same shape as `SandboxApplied`). Emission site: `crates/maos-kernel-core/src/security/mod.rs::admit_spirit` (the only path that consumes `manifest.providers.primary.id`) — when the SCB store contains a prior SCB for the same `spirit_id` with a different `primary.id`, emit ONE `LifecycleEvent::ProviderSwitched` entry via the existing `JournalAdapter::append_lifecycle(...)` path; if the prior SCB has the same provider, emit nothing. Tests in `crates/maos-kernel-core/tests/provider_switched_journal.rs` (NEW file):
  - `first_admit_emits_no_switch_event` — empty SCB store; admit Spirit; verify NO `ProviderSwitched` entry.
  - `second_admit_same_provider_emits_no_switch_event` — admit twice with same `primary.id`; verify NO `ProviderSwitched` entry.
  - `admit_with_changed_provider_emits_switch_event` — admit with `anthropic`, then re-admit with `openai`; verify exactly ONE `ProviderSwitched` entry with payload `{from: "anthropic", to: "openai", manifest_path: <path>, applied_at_ns: <ns>}` and `applied_at_ns ≥ 1` (monotonic_now_ns discipline).
- **EXTENDED `crates/maos-bin/src/main.rs` composition root** — the existing single-provider wiring at lines 384-405 is restructured:
  ```rust
  // BEFORE (lines 384-405): single AnthropicProvider → InferencePortAdapter

  // AFTER (Story 5.5b): assemble a BTreeMap of registered providers, default to first available
  let mut providers_map: BTreeMap<String, Arc<dyn maos_providers::Provider>> = BTreeMap::new();
  let mut default_id: Option<String> = None;

  // Anthropic — if MAOS_ANTHROPIC_API_KEY is set
  if let Ok(provider) = AnthropicProvider::new(Arc::clone(&io_arc), "https://api.anthropic.com".into(), "claude-3-haiku-20240307".into()) {
      providers_map.insert("anthropic".into(), Arc::new(provider));
      default_id.get_or_insert_with(|| "anthropic".into());
      eprintln!("maos: Anthropic provider registered");
  }

  // OpenAI — if MAOS_OPENAI_API_KEY is set
  if let Ok(provider) = OpenAiProvider::new(Arc::clone(&io_arc), "https://api.openai.com".into(), "gpt-4o-mini".into()) {
      providers_map.insert("openai".into(), Arc::new(provider));
      default_id.get_or_insert_with(|| "openai".into());
      eprintln!("maos: OpenAI provider registered");
  }

  // Ollama — always attempt (no env-gated key); URL override via MAOS_OLLAMA_URL
  let ollama_url = std::env::var("MAOS_OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());
  if let Ok(provider) = OllamaProvider::new(Arc::clone(&io_arc), ollama_url, "llama3.1:8b".into()) {
      providers_map.insert("ollama".into(), Arc::new(provider));
      default_id.get_or_insert_with(|| "ollama".into());
      eprintln!("maos: Ollama provider registered");
  }

  if providers_map.is_empty() {
      // Fallback: register UnconfiguredProvider as anthropic so the
      // composition root still produces a non-panicking adapter (existing 1b.4 pattern).
      providers_map.insert("anthropic".into(), Arc::new(UnconfiguredProvider));
      default_id = Some("anthropic".into());
      eprintln!("maos: no providers configured — all inference calls return Unconfigured");
  }

  let router = Arc::new(MultiProviderRouter::new(providers_map, default_id));
  let inference = InferencePortAdapter::new(
      Arc::clone(&router),
      Arc::clone(&capability),
      Arc::clone(&transparency_log),
      Arc::clone(&telemetry),
  );
  ```
  The construction order is **fixed** (anthropic → openai → ollama) so `default_id` resolution is deterministic across CI runs even when only one provider has credentials configured. Documented in the composition-root completeness comment block at the top of `main.rs`.
- **NEW `MAOS_ONE_SHOT=smoke-multi-provider-5` arm** at `crates/maos-bin/src/main.rs` — additive on the existing match block; the known-modes list at line 2032 EXTENDS to include `smoke-multi-provider-5`. The arm walks the 5 surfaces listed in the Story narrative section (g) using `FixtureReplayProvider` instances loaded from `crates/maos-providers/tests/fixtures/multi-provider-v0/smoke/`. Test driver `crates/maos-bin/tests/smoke_multi_provider_test.rs` (NEW) invokes the arm via `Command::new(maos_bin).env("MAOS_ONE_SHOT", "smoke-multi-provider-5")`, asserts exit code 0, asserts stdout contains 5 JSON lines each with the expected `step` and `surface` fields. The test runs on every platform (no platform-specific dependencies — fixture replay is host-agnostic). Pattern follows Story 5.5a's `tests/smoke_t3_sandbox_test.rs` shape exactly.
- **NEW `.github/workflows/multi-provider.yml`** — fresh sibling of `discipline.yml`/`journal-aggregate.yml` (NOT a discipline.yml job; rationale in Dev Notes Decision Register). The workflow:
  ```yaml
  name: multi-provider
  on:
    push:
      branches: [main]
    pull_request:
      branches: [main]
  jobs:
    matrix:
      runs-on: ubuntu-latest
      strategy:
        fail-fast: false
        matrix:
          provider: [anthropic, openai, ollama]
      steps:
        - uses: actions/checkout@v4
        - uses: dtolnay/rust-toolchain@v1
          with: { toolchain: stable }
        - uses: Swatinem/rust-cache@v2
          with: { key: ${{ hashFiles('**/Cargo.lock') }} }
        - name: Run multi-provider fixture suite for ${{ matrix.provider }}
          run: |
            cargo test -p maos-providers --features fixture_replay \
              --test multi_provider_matrix \
              -- --nocapture \
              ${{ matrix.provider }}
        - name: Upload per-provider report
          uses: actions/upload-artifact@v4
          with:
            name: multi-provider-${{ matrix.provider }}-report
            path: tests/reports/multi-provider-${{ github.sha }}-${{ matrix.provider }}.json
            retention-days: 90

    report-aggregate:
      runs-on: ubuntu-latest
      needs: matrix
      steps:
        - uses: actions/checkout@v4
        - uses: dtolnay/rust-toolchain@v1
          with: { toolchain: stable }
        - uses: Swatinem/rust-cache@v2
          with: { key: ${{ hashFiles('**/Cargo.lock') }} }
        - name: Download all per-provider reports
          uses: actions/download-artifact@v4
          with:
            pattern: multi-provider-*-report
            path: tests/reports/
            merge-multiple: true
        - name: Aggregate + drift-check
          run: |
            cargo run -p xtask -- check-multi-provider-drift \
              --report tests/reports/multi-provider-${{ github.sha }}.json \
              --threshold 10 \
              --json
        - name: Upload aggregated report
          uses: actions/upload-artifact@v4
          with:
            name: multi-provider-aggregate
            path: tests/reports/multi-provider-${{ github.sha }}.json
            retention-days: 90
  ```
  The matrix uses **fixture-replay mode** by default — no live API keys in CI (the smoke arm + the matrix runner both use canned fixtures). When `MAOS_ANTHROPIC_API_KEY` / `MAOS_OPENAI_API_KEY` are set in the CI environment (NOT a v0.5-α default; reserved for `workflow_dispatch` runs), the matrix can flip to live mode by setting a `MAOS_MATRIX_MODE=live` env-var; that path is documented but DEFAULT for `push` / `pull_request` is `fixture`.
- **NEW `crates/maos-providers/tests/multi_provider_matrix.rs`** — the matrix runner:
  - Loads fixtures from `crates/maos-providers/tests/fixtures/multi-provider-v0/cases/` (10 JSON files, one per fixture; each contains the input `InferenceRequest` + a per-provider `expected_outputs` map keyed by provider ID).
  - Resolves the target provider from CLI args (matches the workflow YAML's `${{ matrix.provider }}` argument).
  - Instantiates a `FixtureReplayProvider` configured with the fixture's expected output for the target provider.
  - Runs each fixture's request through the provider, collects `(fixture_id, provider, response, observed_metrics)` tuples.
  - Serializes to `tests/reports/multi-provider-<sha>-<provider>.json` with stable JSON key order (canonical serialization per Story 5.4 discipline; uses `serde_json::Map` not `HashMap`).
  - Each row in the report has shape: `{fixture_id, provider, response_text_len, input_tokens, output_tokens, stop_reason, latency_us, error: null | <ProviderError JSON>}`.
- **NEW `xtask check-multi-provider-drift` command** at `xtask/src/check_multi_provider_drift.rs`:
  - Reads the aggregated report at `--report <path>` (the merged JSON from the report-aggregate job).
  - For each `fixture_id`, computes the median of `response_text_len`, `input_tokens`, `output_tokens` across all providers in the report.
  - For each (fixture_id, provider) cell, computes the percentage delta from the median for each metric.
  - Flags cells where ANY metric's `abs(delta_pct) >= threshold` (default 10).
  - Stop-reason differences are flagged categorically (StopSequence vs MaxTokens etc.) — any non-unanimous stop_reason on a fixture is an outlier flag.
  - Output: structured JSON to stdout when `--json` is set; otherwise human-readable table.
  - `--strict` flag causes non-zero exit on any outlier (NOT the default — CI matrix should annotate, not fail, so PRs aren't blocked on legitimate provider behavior differences).
  - Unit tests at `xtask/src/tests/check_multi_provider_drift_tests.rs`:
    - `median_of_three_with_outlier_flags_one_row` — 100, 100, 200 → median=100; 200 is +100% delta; flag.
    - `median_of_three_equal_flags_nothing` — all 100 → no flags.
    - `stop_reason_disagreement_flags` — anthropic=StopSequence, openai=MaxTokens, ollama=StopSequence → flag openai.
    - `missing_provider_row_flags` — fixture has 2 of 3 providers → flag missing.
    - `empty_report_returns_no_flags_and_zero_exit` — degenerate but legal.
  - Integration tests at `xtask/tests/check_multi_provider_drift_integration.rs`:
    - Invokes the xtask binary with `--report tests/fixtures/multi-provider-reports/clean.json`, asserts exit 0 + empty `outliers` array.
    - Invokes with `tests/fixtures/multi-provider-reports/with-outlier.json`, asserts exit 0 (annotation mode) + one row in `outliers`.
    - Invokes with `--strict` + outlier fixture, asserts non-zero exit.
- **NEW fixtures**:
  - `crates/maos-providers/tests/fixtures/multi-provider-v0/methodology-attestation.json` — schema doc + selection rationale per Story 5.4 corpus discipline (loaders SKIP this file).
  - `crates/maos-providers/tests/fixtures/multi-provider-v0/cases/case_simple_completion.json` — single-turn prompt; 3 provider expected outputs.
  - `crates/maos-providers/tests/fixtures/multi-provider-v0/cases/case_max_tokens_truncation.json` — small `max_tokens`; expect `StopReason::MaxTokens` on all 3.
  - `crates/maos-providers/tests/fixtures/multi-provider-v0/cases/case_temperature_zero.json` — `temperature: 0.0`; deterministic semantics across providers (the response text WILL differ but length should be within drift threshold).
  - `case_empty_prompt.json`, `case_long_prompt_4k_tokens.json`, `case_unicode_korean.json`, `case_json_output_request.json`, `case_short_response.json`, `case_provider_429_rate_limit.json`, `case_provider_500_error.json` — 10 total.
  - `crates/maos-providers/tests/fixtures/multi-provider-v0/smoke/` — 3 minimal fixtures used by the smoke arm.
  - `xtask/tests/fixtures/multi-provider-reports/clean.json` — sample aggregated report with three providers within 5% of median.
  - `xtask/tests/fixtures/multi-provider-reports/with-outlier.json` — sample aggregated report with one provider deviating ≥10% from median.
- **EXTENDED `xtask/kernel-api-classes.toml`** — every new public kernel-core symbol (`MultiProviderRouter`, `RouterError`, `inference::router` module re-exports) is added under the appropriate `data-movement` class per the Story 1b.4 precedent (`InferencePortAdapter` + inference types → `data-movement`). The check-service-boundary discipline gate enforces this; missing entries fail CI.
- **EXTENDED `.github/workflows/discipline.yml`** — ONE new discipline job:
  - `multi-provider-drift-tests` — runs `cargo test -p xtask check_multi_provider_drift` to verify the drift-check tool itself works. (The matrix workflow is a separate file; the drift check's unit + integration tests live with the rest of xtask discipline.)
  Cumulative discipline.yml job count: ~55 after Story 5.5a + 1 (this story) = **~56** at story-merge.

## What this story is NOT

- **NOT a new `ProviderDriver` trait.** The `Provider` trait at `crates/maos-providers/src/provider.rs:11` IS the driver surface; the epic AC's `ProviderDriver` is resolved as a re-export alias (`pub use Provider as ProviderDriver;`) at `crates/maos-providers/src/lib.rs`. Renaming would cascade through `crates/maos-kernel-core/src/inference/mod.rs:25` (consumer) + every existing test + the 1b.4 ADR-005 commitment. The alias is the documented resolution.
- **NOT a new `maos-providers-openai` or `maos-providers-ollama` crate per the epic AC line 226's hint at "driver crates" plural.** The epic spec's "three driver crates" wording is re-interpreted as "three driver modules within the existing `maos-providers` crate" — preserves the 23-crate workspace count (Cargo.toml:3-27 verified — Story 5.5a's 23-crate floor stays); mirrors the Anthropic precedent at lines 1-12 of `lib.rs` (every driver is a module, not a crate); avoids the cross-crate dep-cycle hazard the existing crate-organization avoids; matches the §What this story IS structure. The Decision Register in §Dev Notes records this trade. Crate extraction is deferred to the same trigger as Story 5.5e's KLOC review or Epic 6's subprocess-form work.
- **NOT streaming inference (`provider/stream`) or embeddings (`provider/embed`).** Per ADR-005 + Story 1b.4 NOT-scope at `_bmad-output/implementation-artifacts/1b-4-...md:19`, the `complete`-only surface is the v0.5-α floor. The OpenAI driver's `stream: false` (the field is absent from our `build_openai_request_body` — defaults to false at the API level) and Ollama's explicit `"stream": false` are deliberate. Streaming arrives at v0.5+ in a follow-up story documented in `_bmad-output/planning-artifacts/epics/open-items-carried-forward-to-implementation.md`. **Update `crates/maos-domain/src/ports/inference.rs:4-5` to remove the "deferred to Story 5.5b" reference for streaming/embeddings — replace with "deferred to a v0.5+ follow-up story; see open-items-carried-forward-to-implementation.md".**
- **NOT tool-calling / function-calling.** The OpenAI Chat Completions `tools` parameter, the Anthropic Messages `tools` parameter, and the Ollama `tools` parameter (Llama 3.1+ supports it) are explicitly out of scope for v0.5-α. The cross-provider tool-calling surface is a structural concern (different providers shape tool calls differently) and is part of Story 5.5c's MCP-server bridging. Story 5.5b carries forward an explicit "tool-calling cross-provider equivalence" line item in `deferred-work.md`.
- **NOT live API calls in CI.** The matrix runs in **fixture-replay mode** by default; CI never makes calls to real Anthropic/OpenAI/Ollama endpoints. Live-mode runs are reserved for `workflow_dispatch` (manual ops triggers; documented in Dev Notes) and the per-driver `#[ignore]`-gated integration tests (`anthropic_integration` at `anthropic.rs:283-300`, plus new `openai_integration` and `ollama_integration` siblings). The composition-root wiring in `main.rs` still attempts live calls when env-vars are present; that path is exercised by developer-host runs, not CI.
- **NOT provider rate-limit isolation (NFR-Scale-4 — per-(provider, credential) token bucket + `RateLimited` IAC frame).** Per architecture §4.4 line 368 + NFR-Scale-4 at v0.5+ — the per-(provider, credential) token bucket implementation lands at Epic 6 alongside the full I/O Subsystem service extraction; Story 5.5b adds the routing + fallback substrate but the bucket itself is forward-shaped. Documented in Dev Notes.
- **NOT cost attribution per provider (NFR-Cost-1).** Per FR64 + NFR-Cost-1, per-Spirit per-task cost accounting against provider billing arrives at v1.0 (Epic 9 Story 9.3). Story 5.5b's `InferenceResponse.usage` field carries token counts; the cost-rollup is the consumer's concern.
- **NOT a manifest schema-version bump.** The `[providers]` section is OPTIONAL; manifests without it use composition-root default. `class.manifest_schema_version` stays at 1. The optionality is the contract.
- **NOT an ABI break.** `cargo public-api` baseline at `xtask/abi-baseline/v1-pre-bump.txt` MUST report adds-only:
  - NEW types: `ProvidersSection`, `ProviderConfig` (in `maos-kernel-core::security::manifest`); `MultiProviderRouter`, `RouterError` (in `maos-kernel-core::inference::router`); `OpenAiProvider`, `OllamaProvider`, `FixtureReplayProvider` (in `maos-providers`).
  - NEW field on `InferenceRequest` — `provider_id: Option<String>`, `fallback_provider_ids: Vec<String>`. Since `InferenceRequest` is `#[derive(...)]` without `#[non_exhaustive]`, this is a **soft break for struct-literal callers**. **Resolution:** apply `#[non_exhaustive]` to `InferenceRequest` in this story (matches the EXISTING `#[non_exhaustive]` pattern on other domain types per Story 5.4 discipline at Story 5.4 line 28) AND add the `InferenceRequest::new(...)` constructor that all new callers MUST use. Existing struct-literal callers in tests inside `maos-providers` and `maos-kernel-core` are updated. The ABI gate is satisfied because `InferenceRequest` was NOT yet stabilized at v1.0 (we are at v0.5-α; the v1.0 ABI freeze is Story 7.5a).
  - NEW variant on `#[repr(u8)]` `LifecycleEvent` — `ProviderSwitched = 18`. Additive; preserves 0..17.
  - NEW re-export `ProviderDriver` (alias for `Provider`) — additive.
  `ABI_VERSION` stays at `1`.
- **NOT the v0.5+ MAOS-mediated provider proxies (architecture §13 phased-roadmap.md:11; v1.5 deliverable).** Story 5.5b lets a Spirit talk to Anthropic/OpenAI/Ollama through the kernel-mediated Inference Port, but the Inference Port doesn't yet act as a substrate-side HTTP intercept on the provider's wire calls. The intercept layer (which lets the substrate audit/redact/policy the actual outbound HTTP) lands at v1.5 per the roadmap. Story 5.5b's drivers go provider-direct over HTTPS; the intercept seam is the `IoSubsystemPort::http_post` adapter which already exists and which the v1.5 work will instrument.
- **NOT the full air-gapped network-namespace structural validation.** Per the Story narrative section so-that (f) + AC4 below — Story 5.5b's air-gapped Ollama check is observability-grade (smoke arm output asserts zero outbound non-Ollama calls were attempted via the IoSubsystemPort journal); the structural egress-prevention test (running the substrate inside `unshare --net` and asserting zero non-loopback packets leave) is Story 9.4. Forward-shaped here.

## Acceptance Criteria

### AC1 — Three `Provider` driver modules + `ProviderDriver` alias + FR47 lint gate stays empty-allowlist (epic AC1, FR47, ADR-005)

**Given** the EXISTING `Provider` trait at `crates/maos-providers/src/provider.rs:11`, the EXISTING `AnthropicProvider` reference driver at `anthropic.rs` (REST-direct + `IoSubsystemPort`-injected transport + env-gated credential + pure `build_request_body` + `parse_response`), the EXISTING `check-fr47` xtask + denylist + empty allowlist at `xtask/src/check_fr47.rs` + `xtask/fr47-vendor-sdk-denylist.toml` + `xtask/fr47-allowlist.toml`, and the EXISTING `InferencePortAdapter` consumer at `crates/maos-kernel-core/src/inference/mod.rs:25` (`use maos_providers::Provider;`),

**When** Story 5.5b lands the NEW `crates/maos-providers/src/openai.rs::OpenAiProvider` driver + the NEW `crates/maos-providers/src/ollama.rs::OllamaProvider` driver + the NEW `crates/maos-providers/src/fixture_replay.rs::FixtureReplayProvider` (test/feature-gated) + the `pub use Provider as ProviderDriver;` re-export at `lib.rs`,

**Then** every driver implements `Provider::complete(&self, req: &InferenceRequest) -> Result<InferenceResponse, ProviderError>` with the contract:
1. **Vendor-neutral types only** — the function signature uses `maos_domain::ports::inference::*` types; no provider-specific structs cross the function boundary.
2. **REST-direct over `IoSubsystemPort::http_post`** — no vendor SDK crate is added to the workspace; `xtask check-fr47` continues to PASS with empty allowlist.
3. **Env-gated credential** — `OpenAiProvider::new` reads `MAOS_OPENAI_API_KEY`; missing → `Err(ProviderError::Unconfigured)`. `OllamaProvider::new` reads `MAOS_OLLAMA_URL` (optional override); missing → uses default `http://localhost:11434` (no credential required for local Ollama).
4. **Pure request-body + response-parse functions** — `build_openai_request_body(req, model_id) -> serde_json::Value`, `parse_openai_response(json, endpoint_url, model_id) -> Result<InferenceResponse, ProviderError>`, and Ollama equivalents. Unit-tested against fixtures at `crates/maos-providers/tests/fixtures/openai_*.json` and `ollama_*.json` per the EXISTING Anthropic pattern at `anthropic.rs:154-301`.
5. **`provider_attribution.provider_id`** is exactly `"openai"` / `"ollama"` — no aliases, no version suffix; this string is the dispatch key the `MultiProviderRouter` matches against.
6. **`ProviderError` variant mapping** — HTTP transport failures → `Transport(String)`; non-2xx responses → `ProviderRejected { status, body }`; serde failures → `Serde(String)`. The `Provider`-trait error contract from Story 1b.4 is preserved.

**And** integration tests at `crates/maos-providers/tests/openai_round_trip_test.rs` and `crates/maos-providers/tests/ollama_round_trip_test.rs` cover:
- Happy path with a `MockTransport` (same shape as Anthropic's MockTransport at `anthropic.rs:160-174`) returning canned bytes; assert resulting `InferenceResponse.text` matches the fixture's expected text + `provider_attribution.provider_id` matches.
- `max_tokens` stop-reason → `StopReason::MaxTokens`.
- `stop` stop-reason → `StopReason::StopSequence`.
- Non-recognized stop-reason → `StopReason::ProviderStop(other)`.
- Empty/missing `usage` block → defaults to `input_tokens: 0, output_tokens: 0` (matches Anthropic precedent at `anthropic.rs:130-137`).
- `#[ignore]`-gated `openai_integration` and `ollama_integration` tests that require `MAOS_OPENAI_API_KEY` / a running local Ollama; these mirror `anthropic_integration` at `anthropic.rs:281-300`.

**And** `cargo run -p xtask -- check-fr47` continues to PASS with `fr47-allowlist.toml` empty. **No vendor SDK is added to the workspace** — verify via `cargo tree | grep -E 'openai|anthropic-sdk|ollama-rs|async-openai'` returns empty output.

**And** the `pub use Provider as ProviderDriver;` re-export at `crates/maos-providers/src/lib.rs` exists and is documented with a clear doc-comment explaining the alignment with epic-AC terminology. The re-export is verified by a unit test `crates/maos-providers/src/lib.rs::tests::provider_driver_alias_resolves` that exercises `let _: Box<dyn maos_providers::ProviderDriver> = Box::new(...);`.

---

### AC2 — `[providers]` manifest section + `MultiProviderRouter` + per-Spirit provider dispatch (epic AC3, FR3)

**Given** the EXISTING manifest infrastructure at `crates/maos-kernel-core/src/security/manifest.rs` (the `[class]`, `[sandbox]`, `[capabilities]`, `[resources]`, `[output_shape]`, `[budget]`, `[epistemic_policy]`, `[on_crash]`, `[on_revocation]` sections + the `validate()` body), the EXISTING `SpiritControlBlock` (SCB) store at `crates/maos-kernel-core/src/scheduler/...` (admit time wires the parsed manifest into the SCB), and the EXISTING `InferencePortAdapter` consumer flow,

**When** Story 5.5b lands (a) the NEW `ProvidersSection` + `ProviderConfig` types in `manifest.rs` (full type definitions in §What this story IS), (b) the manifest validator extension that rejects unsupported `id` / empty `endpoint_url` / malformed `provider_endpoint_pin`, (c) the NEW `MultiProviderRouter` at `crates/maos-kernel-core/src/inference/router.rs`, (d) the `InferencePortAdapter::complete` restructure to call `router.dispatch_with_fallback`, (e) the new `InferenceRequest.provider_id: Option<String>` + `fallback_provider_ids: Vec<String>` fields with `#[non_exhaustive]` + `::new` constructor,

**Then** the manifest validator admits:
- `[providers]` absent → OK (uses composition-root default).
- `[providers].primary = { id = "anthropic" }` → OK; no fallback.
- `[providers].primary = { id = "openai" }` + `fallback = [{ id = "anthropic" }]` → OK.
- `[providers].primary = { id = "ollama", endpoint_url = "http://localhost:11434" }` → OK.

**And** the validator REJECTS with `ManifestError::Toml(...)`:
- `[providers].primary.id = "kimi"` → `"providers.primary.id 'kimi' unsupported at v0.5-α (allowed: anthropic, openai, ollama)"`.
- `[providers].primary.endpoint_url = ""` → `"providers.primary.endpoint_url must not be empty if present"`.
- `[providers].primary.provider_endpoint_pin = "not-hex"` → `"providers.primary.provider_endpoint_pin must be 64-char hex SHA-256"`.
- Each `fallback[i]` validated by the same rules; rejection cites the index.

**And** the `MultiProviderRouter::dispatch_with_fallback(primary_id, fallback_ids, req)` body:
1. Tries `primary_id`; if OK, returns the response with `provider_attribution.provider_id == primary_id`.
2. On `Transport(_)` / `ProviderRejected { status: 429 | 500..=599, .. }`, walks to `fallback_ids[0]`, then `[1]`, etc.
3. On `Serde(_)` / `Unconfigured` / `ProviderRejected { status: 4xx (not 429), .. }`, **short-circuits** and returns the error immediately (non-retriable — see Dev Notes Decision Register on why client errors don't walk fallback).
4. On full chain exhaustion, returns the LAST `ProviderError` (NOT a synthetic aggregate; preserves the actual failure details for the operator).

**And** integration test `crates/maos-kernel-core/tests/multi_provider_routing.rs` (NEW) covers:
- `manifest_with_openai_primary_dispatches_to_openai` — `[providers].primary.id = "openai"`; admit; issue `InferenceRequest { provider_id: Some("openai"), .. }`; assert response has `provider_attribution.provider_id == "openai"`.
- `manifest_with_anthropic_fallback_walks_on_503` — primary = `openai` (mocked to return 503), fallback = `["anthropic"]`; admit; complete; assert response has `provider_attribution.provider_id == "anthropic"` AND TL `intent` contains `"openai->anthropic"`.
- `manifest_unsupported_provider_id_rejected_at_admission` — `[providers].primary.id = "kimi"`; assert admission fails with the typed error from manifest validation.
- `request_with_unregistered_provider_id_returns_router_error` — admit OK; later issue `InferenceRequest { provider_id: Some("unregistered"), .. }`; assert `Err(InferenceError::ProviderTransport(...))` carrying the router-error message.
- `request_with_no_provider_id_uses_router_default` — admit; issue `InferenceRequest { provider_id: None, .. }`; assert the response's `provider_attribution.provider_id` matches the router's `default_id`.

**And** the `xtask check-pub-field-constructors` gate passes — every new `pub` field on `ProvidersSection`, `ProviderConfig`, `MultiProviderRouter`, and the new `InferenceRequest` fields carries the `#[doc = "Construct via ::new ..."]` annotation matched by an `impl ::new` constructor.

**And** the manifest fixtures listed in §What this story IS (six new TOML files under `crates/maos-kernel-core/tests/fixtures/manifest/providers/`) are committed and the existing `manifest_field_coverage` discipline gate continues to pass (every new field is reachable from at least one well-formed fixture).

---

### AC3 — Multi-provider CI matrix workflow + behavioral-drift report + 10% threshold (epic AC2)

**Given** the EXISTING CI workflow infrastructure (`.github/workflows/discipline.yml` ~55 jobs; `.github/workflows/corpus-rebaseline.yml`; `.github/workflows/journal-aggregate.yml`; `.github/workflows/journal-append.yml`), the EXISTING `xtask` toolset at `/home/lunarpulse/dev_ws/maos/xtask/src/main.rs` with subcommands `coverage-matrix`, `check-fr47`, `check-corpus`, etc., the EXISTING bench-results retention pattern from Story 0.3 (90-day artifact retention), and the EXISTING `crates/maos-providers/tests/fixtures/` layout precedent from `anthropic_*.json`,

**When** Story 5.5b lands (a) the NEW `.github/workflows/multi-provider.yml` matrix workflow (YAML in §What this story IS), (b) the NEW `crates/maos-providers/tests/multi_provider_matrix.rs` runner, (c) the NEW fixture suite at `crates/maos-providers/tests/fixtures/multi-provider-v0/` (10 cases + methodology + smoke subdirectory), (d) the NEW `xtask check-multi-provider-drift` command at `xtask/src/check_multi_provider_drift.rs`,

**Then** the matrix workflow runs on every `push` to `main` + every `pull_request` to `main`:
1. A 3-cell matrix (`provider: [anthropic, openai, ollama]` × `os: [ubuntu-latest]`) executes the fixture suite per provider in fixture-replay mode.
2. Each cell uploads `tests/reports/multi-provider-<sha>-<provider>.json` as a workflow artifact (90-day retention).
3. A `report-aggregate` job downloads all three artifacts, runs `cargo run -p xtask -- check-multi-provider-drift --report tests/reports/multi-provider-<sha>.json --threshold 10 --json`, uploads the merged report.
4. The aggregated report has one row per `(fixture_id, provider)` pair (10 fixtures × 3 providers = 30 rows). Each row carries `{fixture_id, provider, response_text_len, input_tokens, output_tokens, stop_reason, latency_us, error: null | {...}}`.
5. The drift-check emits structured GitHub Actions annotations (`::notice file=tests/reports/multi-provider-<sha>.json::Drift outlier: ...`) for any `(fixture_id, provider)` cell where any metric's `abs(delta_from_median_pct) >= 10` OR the stop_reason disagrees with the per-fixture median stop_reason.

**And** the drift-check command:
- Default mode (no `--strict`): emit annotations + exit 0 (PRs not blocked on legitimate provider behavior differences).
- `--strict` mode: non-zero exit on any outlier (reserved for release-cut hardening runs; documented in Dev Notes).
- `--json` mode: structured JSON to stdout (consumed by the CI annotation step + downstream operator tooling).

**And** unit tests at `xtask/src/tests/check_multi_provider_drift_tests.rs` cover the 5 scenarios listed in §What this story IS (median computation, equal-values no-flag, stop_reason disagreement, missing provider, empty report).

**And** integration tests at `xtask/tests/check_multi_provider_drift_integration.rs` cover the 3 scenarios (clean report exit 0, outlier report exit 0 + flagged row, `--strict` + outlier exit non-zero) per the existing xtask integration-test pattern.

**And** the matrix workflow has its name (`multi-provider`) added to the project README's CI section or equivalent documentation if such exists; the workflow is documented in `Dev Notes` as the v0.5-α floor for the FR3 + ADR-005 commitment.

**And** **observability gate**: when an evaluator runs `gh run list --workflow=multi-provider.yml --limit 5` they observe the most recent runs all passing. The matrix is the mechanical proof — `gh` output is the operator-facing observation. This closes the "evaluator can observe actual behavior" discipline from `[[feedback_lunarpulse_observability_preference]]`.

---

### AC4 — Air-gapped Ollama configuration + zero outbound provider calls + structural validation forward-shape (epic AC4; FR3; NFR-Ops-12 observability leg)

**Given** the EXISTING `IoSubsystemPort` HTTP transport at `crates/maos-kernel-core/src/io/mod.rs` (the `http_post` adapter all three drivers route through), the EXISTING `OllamaProvider` with default `endpoint_url = "http://localhost:11434"` (local loopback only), and the EXISTING `[providers]` manifest section from AC2,

**When** the operator configures a Spirit with `[providers].primary = { id = "ollama", endpoint_url = "http://localhost:11434" }` AND no `fallback`, AND runs the substrate end-to-end (the `smoke-multi-provider-5` arm OR a real Spirit invocation),

**Then** the substrate completes the Spirit's inference calls successfully:
- Every inference call routes through `MultiProviderRouter.dispatch("ollama")` → `OllamaProvider::complete` → `IoSubsystemPort::http_post("http://localhost:11434/api/chat", ...)`.
- Zero calls leave the loopback interface. **Verified at the observability layer** by the smoke arm's step 4 (synthetic): inspect a journal of `IoSubsystemPort` calls and assert every call's URL host resolves to `127.0.0.1` / `localhost` / `::1`.
- The substrate does NOT attempt fallback to Anthropic/OpenAI (no fallback declared in manifest).

**And** the NEW smoke arm sub-step (added inside `smoke-multi-provider-5` at `crates/maos-bin/src/main.rs`):
- After the fallback-walk step (step 4 in the main story), the arm re-runs with an Ollama-primary + no-fallback configuration AND prints `{"step":6,"surface":"air_gap_check","outcome":"loopback_only","outbound_calls":<n>,"loopback_calls":<n>}` where `outbound_calls` MUST be 0 in observable mode (the FixtureReplayProvider records every call URL it would have made).
- The smoke-arm test driver at `crates/maos-bin/tests/smoke_multi_provider_test.rs` asserts step 6's JSON line has `"outbound_calls":0`.

**And** integration test `crates/maos-kernel-core/tests/air_gap_ollama_test.rs` (NEW):
- Configures the `InferencePortAdapter` with ONLY the `OllamaProvider` (or a `FixtureReplayProvider` standing in for Ollama in CI).
- Runs 10 synthetic inference calls.
- Asserts every call's `provider_attribution.endpoint_url` starts with `"http://localhost"` OR `"http://127.0.0.1"` OR `"http://[::1]"` (loopback variants).
- Asserts the `IoSubsystemPort`'s call journal (a new test-only feature on the existing `IoSubsystemAdapter` — a `Vec<String>` of every URL passed to `http_post`, gated by `#[cfg(any(test, feature = "io_call_journal"))]`) contains zero non-loopback URLs.

**And** the structural egress-prevention test (running the substrate inside `unshare --net` / `ip netns` and asserting zero packets leave the loopback interface) is **explicitly out of scope** for this story; the forward-shape commitment is documented in `deferred-work.md` with a cross-reference to Story 9.4 (operator surface — full air-gapped deployment validation). Story 5.5b's observability-grade air-gap check is the v0.5-α floor; the structural validation is the v1.0 ship gate.

**And** the manifest fixture `crates/maos-kernel-core/tests/fixtures/manifest/providers/well-formed/primary-ollama-air-gapped.toml` is a copy of the v0.5-α air-gapped configuration the operator would deploy in a disconnected environment. It is referenced by the integration test as the canonical "this is what air-gapped looks like" example.

---

### AC5 — `LifecycleEvent::ProviderSwitched = 18` journal event + manifest-only provider switch (epic AC3 — operator switches Anthropic → OpenAI mid-deployment)

**Given** the EXISTING `LifecycleEvent` enum at `crates/maos-domain/src/invariants/i10.rs` (`#[repr(u8)]`; discriminants 0..17 including Story 5.5a's `SandboxApplied = 17` + Story 5.4's `Upgrade = 15` / `Revoked = 16`), the EXISTING `JournalAdapter::append_lifecycle(...)` path used by Story 5.4's upgrade events + Story 5.5a's sandbox-applied events, the EXISTING `admit_spirit` path at `crates/maos-kernel-core/src/security/mod.rs` (the only place new SCBs are wired into the scheduler), and the EXISTING SCB store at `crates/maos-kernel-core/src/scheduler/`,

**When** Story 5.5b lands (a) the NEW `LifecycleEvent::ProviderSwitched = 18` discriminant (additive on the `#[repr(u8)]` enum — preserves all 0..17), (b) the NEW `ProviderSwitchedPayload` shape, (c) the emission site in `admit_spirit` that detects provider change between consecutive admissions of the same `spirit_id`,

**Then** when an operator changes a Spirit's manifest from `[providers].primary = { id = "anthropic" }` to `[providers].primary = { id = "openai" }` and re-runs `maosctl spirit upgrade <spirit> --to <new-manifest> --policy <hot-swap|cold-swap>` (Story 5.4 verb), the admission of the new manifest:
1. Detects that the prior SCB (queried from `scheduler.scbs().get(spirit_id)`) had `manifest.providers.primary.id == "anthropic"`.
2. Detects that the new manifest has `manifest.providers.primary.id == "openai"`.
3. Emits exactly ONE `LifecycleEvent::ProviderSwitched` entry to the Lifecycle Journal via `journal.append_lifecycle(JournalEntry::Lifecycle(LifecycleEntry { lifecycle_event: LifecycleEvent::ProviderSwitched, spirit_id, payload, timestamp_ns: monotonic_now_ns(), .. }))`.
4. Payload: `serde_json::to_vec(&ProviderSwitchedPayload { spirit_id: <id>, from_provider: "anthropic", to_provider: "openai", manifest_path: <path>, applied_at_ns: <ns> })` — serialization errors propagated (no `.unwrap_or_default()`).

**And** when the operator re-admits the same Spirit with the **same** `[providers].primary.id`, NO `ProviderSwitched` entry is emitted (the event is change-triggered, not admission-triggered).

**And** when a Spirit is admitted for the FIRST time, NO `ProviderSwitched` entry is emitted (no prior provider to compare against).

**And** the Spirit binary **does NOT need to be rebuilt** — the manifest change is the only artifact change required. This is the v0.5-α realization of FR3 ("Operator can configure provider drivers per Spirit").

**And** integration test `crates/maos-kernel-core/tests/provider_switched_journal.rs` (NEW) covers:
- `first_admit_emits_no_switch_event` — empty SCB store; admit `spirit-a` with anthropic; verify NO `ProviderSwitched` entry, but verify normal admission events (e.g., `SandboxApplied`) are emitted.
- `second_admit_same_provider_emits_no_switch_event` — admit `spirit-a` with anthropic twice; verify NO `ProviderSwitched` entry between the two admissions.
- `admit_with_changed_provider_emits_switch_event` — admit `spirit-a` with anthropic, then re-admit `spirit-a` with openai; verify exactly ONE `ProviderSwitched` entry with `from_provider: "anthropic", to_provider: "openai"`.
- `multiple_switches_each_emit_event` — anthropic → openai → ollama; verify TWO `ProviderSwitched` entries.
- `payload_uses_monotonic_now_ns` — assert `applied_at_ns ≥ 1` and that consecutive switches have strictly increasing `applied_at_ns` (the monotonic clock invariant from Story 5.4 review §1366).

**And** when the EXISTING `cargo run -p xtask -- check-i10-event-coverage` discipline gate runs (if such a gate exists at HEAD — verify via grep; if absent, add a one-line check to `xtask/src/check_corpus.rs` or a similar discipline tool), it counts 18 `LifecycleEvent` variants and passes.

**And** the `xtask check-pub-field-constructors` gate continues to pass — `ProviderSwitchedPayload` (if it has pub fields) carries the constructor annotations.

---

## Tasks / Subtasks

- [x] **Task 1 (AC1) — OpenAI driver module**
  - [x] Create `crates/maos-providers/src/openai.rs` with `OpenAiProvider` struct, `OpenAiProvider::new(transport, endpoint_url, model_id) -> Result<Self, ProviderError>` reading `MAOS_OPENAI_API_KEY`, `OpenAiProvider::with_api_key` test helper, `Provider::complete` implementation.
  - [x] Pure-function `build_openai_request_body(req, model_id) -> serde_json::Value` and `parse_openai_response(json, endpoint_url, model_id) -> Result<InferenceResponse, ProviderError>`.
  - [x] Inline tests mirroring `anthropic.rs:194-275`: `request_body_has_expected_shape`, `parse_successful_response`, `parse_max_tokens_stop_reason`, `provider_round_trip_with_mock_transport`, `provider_missing_api_key_is_unconfigured`, `#[ignore]`-gated `openai_integration`.
  - [x] Fixtures `crates/maos-providers/tests/fixtures/openai_*.json` (request, response, error).

- [x] **Task 2 (AC1) — Ollama driver module**
  - [x] Create `crates/maos-providers/src/ollama.rs` with `OllamaProvider` struct, `OllamaProvider::new(transport, endpoint_url, model_id) -> Result<Self, ProviderError>` (no env-gated credential; endpoint defaults to `http://localhost:11434` via `MAOS_OLLAMA_URL` override).
  - [x] Pure-function `build_ollama_request_body(req, model_id) -> serde_json::Value` (sets `stream: false`) and `parse_ollama_response(json, endpoint_url, model_id) -> Result<InferenceResponse, ProviderError>`.
  - [x] Inline tests + fixtures parallel to OpenAI.

- [x] **Task 3 (AC1) — `FixtureReplayProvider` test helper**
  - [x] Create `crates/maos-providers/src/fixture_replay.rs` gated by `#[cfg(any(test, feature = "fixture_replay"))]`.
  - [x] Add `fixture_replay` feature to `crates/maos-providers/Cargo.toml`.
  - [x] `FixtureReplayProvider { responses: VecDeque<Result<InferenceResponse, ProviderError>>, calls: Vec<InferenceRequest> }` with constructor `new(responses)` and `Provider::complete` that pops the next response (asserts ring not empty; records the request).
  - [x] Inline tests: empty ring panics with clear message; round-trip records request.

- [x] **Task 4 (AC1) — Re-exports + `ProviderDriver` alias**
  - [x] Update `crates/maos-providers/src/lib.rs` with new modules + `pub use Provider as ProviderDriver;` and clear doc comment explaining the alias.
  - [x] Test `provider_driver_alias_resolves` exercises the alias.
  - [x] Verify `cargo run -p xtask -- check-fr47` PASSES.

- [x] **Task 5 (AC2) — `[providers]` manifest section**
  - [x] Add `ProvidersSection` + `ProviderConfig` to `crates/maos-kernel-core/src/security/manifest.rs` with full pub-field-constructor annotations + `::new` constructors.
  - [x] Extend the manifest root validator to reject unsupported `id` / empty `endpoint_url` / malformed `provider_endpoint_pin`.
  - [x] Add 6 fixtures under `crates/maos-kernel-core/tests/fixtures/manifest/providers/` (3 well-formed + 3 malformed-rejected).
  - [x] Tests in `crates/maos-kernel-core/src/security/manifest.rs::tests`: well-formed parse, each rejection path.
  - [x] Run `cargo run -p xtask -- check-pub-field-constructors` — passes.

- [x] **Task 6 (AC2) — `MultiProviderRouter`**
  - [x] Create `crates/maos-kernel-core/src/inference/router.rs` with `MultiProviderRouter`, `RouterError`, `dispatch`, `dispatch_with_fallback`, `is_retriable`, `registered_ids`.
  - [x] Inline tests covering the 10 router scenarios listed in §What this story IS.
  - [x] Re-export from `crates/maos-kernel-core/src/inference/mod.rs`.
  - [x] Update `xtask/kernel-api-classes.toml` with the new symbols (data-movement class).

- [x] **Task 7 (AC2) — `InferenceRequest` extension + adapter restructure**
  - [x] Add `provider_id: Option<String>` + `fallback_provider_ids: Vec<String>` to `InferenceRequest` at `crates/maos-domain/src/ports/inference.rs` with `#[non_exhaustive]` + `::new` constructor.
  - [x] Restructure `InferencePortAdapter` at `crates/maos-kernel-core/src/inference/mod.rs` to hold `Arc<MultiProviderRouter>` and call `dispatch_with_fallback`. Remove `provider_id` field; per-request lookup uses the request's `provider_id` field.
  - [x] Update the `intent` string to encode primary→actual provider IDs.
  - [x] Update existing test `mock_provider_round_trip_logs_inference_call` to use the router shape.
  - [x] Add NEW test `fallback_503_routes_to_secondary`.
  - [x] Update all existing call sites in tests to use `InferenceRequest::new(...)` (no struct literals).

- [x] **Task 8 (AC2) — Composition root multi-provider wiring**
  - [x] Restructure `crates/maos-bin/src/main.rs` lines 384-405 per §What this story IS.
  - [x] Verify all three provider construction paths (env-gated, no-env Ollama, UnconfiguredProvider fallback) compile and produce correct registered_ids.
  - [x] Run the binary with no env vars set; verify `eprintln` confirms "no providers configured" or "Ollama provider registered" depending on Ollama availability.
  - [x] Integration test `crates/maos-kernel-core/tests/multi_provider_routing.rs` covers the 5 routing scenarios in AC2.

- [x] **Task 9 (AC3) — Fixture suite + matrix runner**
  - [x] Create `crates/maos-providers/tests/fixtures/multi-provider-v0/` with 10 case JSONs + methodology + smoke subdirectory.
  - [x] Create `crates/maos-providers/tests/multi_provider_matrix.rs` that loads fixtures, runs them through a `FixtureReplayProvider` configured for the target provider, writes the per-provider report JSON.
  - [x] Test the runner via `cargo test -p maos-providers --features fixture_replay --test multi_provider_matrix anthropic` (and openai/ollama).

- [x] **Task 10 (AC3) — `check-multi-provider-drift` xtask**
  - [x] Create `xtask/src/check_multi_provider_drift.rs` with median + delta computation + outlier flagging.
  - [x] Register the subcommand in `xtask/src/main.rs`.
  - [x] Unit tests at `xtask/src/tests/check_multi_provider_drift_tests.rs` covering the 5 scenarios.
  - [x] Integration tests at `xtask/tests/check_multi_provider_drift_integration.rs` covering the 3 scenarios.
  - [x] Fixtures at `xtask/tests/fixtures/multi-provider-reports/clean.json` + `with-outlier.json`.

- [x] **Task 11 (AC3) — `.github/workflows/multi-provider.yml`**
  - [x] Create the workflow per §What this story IS YAML.
  - [x] Add `multi-provider-drift-tests` job to `discipline.yml` to gate the drift-check tool's own tests.
  - [x] Verify `cargo run -p xtask -- check-multi-provider-drift --report <fixture-report>` produces the expected GitHub-annotation-compatible output.

- [x] **Task 12 (AC4) — Air-gapped Ollama validation**
  - [x] Add the `io_call_journal` feature to `crates/maos-kernel-core/Cargo.toml`.
  - [x] Extend `IoSubsystemAdapter` to record every `http_post` URL into a `Vec<String>` when the feature is enabled.
  - [x] Create `crates/maos-kernel-core/tests/air_gap_ollama_test.rs` covering the 10-call loopback-only scenario.
  - [x] Add the air-gapped manifest fixture.
  - [x] Add a deferred-work.md entry forward-shaping Story 9.4's structural egress validation.

- [x] **Task 13 (AC5) — `LifecycleEvent::ProviderSwitched` + emission**
  - [x] Add `ProviderSwitched = 18` to `crates/maos-domain/src/invariants/i10.rs::LifecycleEvent`.
  - [x] Add `ProviderSwitchedPayload` to the same module (or `LifecyclePayload` registry if such exists).
  - [x] Extend `admit_spirit` at `crates/maos-kernel-core/src/security/mod.rs` to detect provider change and emit the event via `journal.append_lifecycle(...)`.
  - [x] Integration test `crates/maos-kernel-core/tests/provider_switched_journal.rs` covering the 5 scenarios in AC5.
  - [x] Verify `monotonic_now_ns()` is used for the timestamp.

- [x] **Task 14 (smoke arm) — `MAOS_ONE_SHOT=smoke-multi-provider-5`**
  - [x] Add the arm to `crates/maos-bin/src/main.rs`'s `MAOS_ONE_SHOT` match block (additive on the existing block; mirrors Story 5.5a's `smoke-t3-sandbox-5` arm shape).
  - [x] Extend the known-modes list at line 2032.
  - [x] Walk the 5 (+1 for AC4 step 6) surfaces, printing one JSON line per step.
  - [x] Create `crates/maos-bin/tests/smoke_multi_provider_test.rs` invoking the arm via `Command::new(...).env("MAOS_ONE_SHOT", "smoke-multi-provider-5")`; assert exit 0 + the expected JSON lines.

- [x] **Task 15 (docs + carry-forward)**
  - [x] Update `crates/maos-domain/src/ports/inference.rs:4-5` doc comment to remove the "deferred to Story 5.5b" streaming/embeddings reference; replace with "deferred to a v0.5+ follow-up story".
  - [x] Add `deferred-work.md` entry: streaming + embeddings + tool-calling cross-provider equivalence + provider rate-limit isolation (NFR-Scale-4) + MAOS-mediated provider proxies (v1.5) — all forward-shaped from this story.
  - [x] Update the Story 5.5a Successor table doc-comment at `5-5a-...md:32` to note Story 5.5b is closed (or mark for retro update).
  - [x] Update the README's CI section (if present) with the new `multi-provider` workflow.

## Dev Notes

### Architecture compliance

- **ADR-005 (Pluggable provider drivers; `binding-v0.1`).** This story closes the v0.5 gate: "≥3 providers in CI by v0.5". The contract is "Spirit author writes against the kernel's provider API once and runs against any driver; new drivers ship without kernel changes." Story 5.5b proves this by adding OpenAI + Ollama without touching `InferencePort` trait or `InferencePortAdapter` capability/audit logic. **The router IS a kernel change** but it is additive infrastructure, not a contract change — the contract stayed `Provider::complete` and `InferencePort::complete`.
- **ADR-010 (Hexagonal architecture).** All three drivers remain in the **adapter ring** (the `maos-providers` crate); the domain core (`maos-domain::ports::inference`) carries only vendor-neutral types. The new `MultiProviderRouter` lives in `maos-kernel-core::inference::router` — it is **adapter aggregation**, not a domain concept; it does not belong in `maos-domain`.
- **ADR-030 (Audit-channel non-blocking).** No new audit-channel sites in this story. `LifecycleEvent::ProviderSwitched` goes through the synchronous `JournalAdapter::append_lifecycle` path (same as Story 5.4's `Upgrade` event), not through the cap-audit channel.
- **§4.4 I/O Subsystem.** Story 5.5b reuses the existing `IoSubsystemPort::http_post` — no new HTTP transport code. The per-Spirit bandwidth quota + per-(provider, credential) token bucket from §4.4 line 368 (NFR-Scale-4) is forward-shaped to Epic 6.
- **§13 phased-roadmap.md v0.5 deliverables.** Story 5.5b lands one of the v0.5 deliverables: "Multi-provider LLM drivers tested in CI (≥ 3 providers: Anthropic + OpenAI + local-LLM via Ollama)" per line 124 of `project-scoping-phased-development.md`. The MAOS-mediated provider proxies (v1.5) + Bedrock/Vertex (v2.0) are explicitly out of scope.

### Library / framework requirements

- **No new HTTP client.** Reuse `ureq` (Story 1b.4 selection) via `IoSubsystemPort::http_post`. The three drivers do NOT take a direct HTTP dep.
- **No vendor SDK.** OpenAI Chat Completions REST: `POST /v1/chat/completions` with `Authorization: Bearer <key>`. Ollama Chat: `POST /api/chat`. Both are stable JSON over HTTP — vendor SDKs are not required. The FR47 gate enforces this.
- **`thiserror = "2.0"`** for any new error types — same version as `maos-providers/Cargo.toml:14`.
- **`serde_json = "1.0"`** for request/response shaping.
- **No new workspace dep needed.** All required crates are already in `crates/maos-providers/Cargo.toml`.

### File structure requirements

- **New files under `crates/maos-providers/src/`**: `openai.rs`, `ollama.rs`, `fixture_replay.rs`.
- **New files under `crates/maos-providers/tests/`**: `openai_round_trip_test.rs`, `ollama_round_trip_test.rs`, `multi_provider_matrix.rs`.
- **New fixture directory**: `crates/maos-providers/tests/fixtures/multi-provider-v0/` with `methodology-attestation.json` + `cases/*.json` (10 cases) + `smoke/*.json` (3 minimal smoke fixtures).
- **New file `crates/maos-kernel-core/src/inference/router.rs`** — sibling of `mod.rs` inside the existing `inference` module.
- **New file `crates/maos-kernel-core/tests/multi_provider_routing.rs`** — integration test.
- **New file `crates/maos-kernel-core/tests/provider_switched_journal.rs`** — integration test.
- **New file `crates/maos-kernel-core/tests/air_gap_ollama_test.rs`** — integration test.
- **New file `crates/maos-bin/tests/smoke_multi_provider_test.rs`** — smoke-arm test driver.
- **New file `xtask/src/check_multi_provider_drift.rs`** + `xtask/src/tests/check_multi_provider_drift_tests.rs` + `xtask/tests/check_multi_provider_drift_integration.rs` + `xtask/tests/fixtures/multi-provider-reports/{clean,with-outlier}.json`.
- **New file `.github/workflows/multi-provider.yml`** — fresh sibling workflow.
- **Manifest fixtures under `crates/maos-kernel-core/tests/fixtures/manifest/providers/`** — 3 well-formed + 3 malformed-rejected TOML files.

### Testing requirements

- **All driver-internal tests stay PURE-FUNCTION + MockTransport** — same shape as `anthropic.rs:154-275`. No network access.
- **Integration tests use `FixtureReplayProvider` or `MockProvider`** — no live API calls in CI; the live integration tests stay `#[ignore]`-gated.
- **The CI matrix uses fixture-replay mode by default.** Live mode is `workflow_dispatch`-only and requires explicit env-var configuration.
- **Coverage requirements per `coverage-matrix` discipline gate**: every new public symbol gets at least one test reaching it (the existing gate at `discipline.yml:597-608` will flag misses).
- **The `manifest_field_coverage` discipline gate** at `discipline.yml:~450` requires every new manifest field to be reachable from at least one well-formed fixture; the 3 well-formed fixtures satisfy this.

### Previous story intelligence

- **Story 5.5a observation**: Sandbox-T3 added 6 sub-modules to `kernel-core::security::sandbox::t3/`. The Decision Register in 5.5a documented "NOT a new `maos-sandbox` crate" — Story 5.5b follows the same precedent ("NOT new driver crates"; modules within `maos-providers`).
- **Story 5.4 review §1366 (monotonic_now_ns)**: Use `monotonic_now_ns()` from `crates/maos-kernel-core/src/capability/cap_tokens::monotonic_now_ns` (or wherever the discipline lands) for `ProviderSwitched.applied_at_ns`. NEVER `SystemTime::now()`.
- **Story 5.4 review §1373 (serde unwrap_or_default)**: All `serde_json::to_vec` + `from_slice` calls propagate errors. The new manifest validator, the router, the report serializer, and the drift-check tool all follow this.
- **Story 5.4 review §A4 (pub-field-constructor)**: Every new pub field on a serde struct (`ProvidersSection`, `ProviderConfig`, the new `InferenceRequest` fields, `ProviderSwitchedPayload`) carries the `#[doc = "..."]` annotation matched by an `impl ::new`.
- **Story 1b.4 §AC4 (IAC round-trip telemetry)**: The `InflightGuard` + `record_iac_rt` path is unchanged. Story 5.5b's per-request router dispatch happens **inside** the inflight guard (RAII guarantees correct decrement even on fallback walks).
- **Story 1b.4 §AC3 doc-comment overreach**: The `inference.rs:4-5` doc-comment names Story 5.5b as the streaming+embeddings story. **Resolution**: update the comment to defer to a v0.5+ follow-up (Story 5.5b does NOT add streaming or embeddings; the doc-comment was an over-commitment from 1b.4).

### Git intelligence summary

Recent commits (latest 5):
- `3d751b4 5-4-run-spirit-upgrades-and-propagate-signed-revocations-in-5s` — Story 5.4 dev pass; provider-adjacent infra in revocation pipeline.
- `6f76660 5-3-detect-spirit-crashes-hangs-and-silent-failures-with-halt-receipt-99-9` — supervision substrate.
- `78e0180 5-2-implement-hot-swap-state-transfer-and-cross-major-migration-against-hsis-95` — hot-swap coordinator.
- `5f34833 5-1-ship-full-lifecycle-verbs-and-11-triggers-with-priority-weighted-scheduling` — full lifecycle.
- `65c46c8 Refactor isolation hook methods and enhance test assertions` — Story 5.5a refactor.

The HEAD diff (uncommitted) includes Story 5.5a Tier-T3 substrate changes + the T3 escape corpus fixtures. **Story 5.5b should branch from the 5.5a-merged main; verify `git status` is clean before the first commit on this story.** If 5.5a is still uncommitted at the start of 5.5b, commit 5.5a FIRST per the Story 5.5a discipline (the file list at the top of this conversation suggests 5.5a is staged but not yet committed; confirm with `git log -1 --stat` before starting).

### Latest tech information

- **OpenAI Chat Completions API** — stable; `POST /v1/chat/completions` is the recommended endpoint as of January 2026 (the `/v1/completions` endpoint is legacy and discouraged for new code). The `tool_calls` finish_reason exists but is not in scope for this story. Default model `gpt-4o-mini` (cost-effective for v0.5-α matrix fixtures).
- **Ollama Chat API** — `POST /api/chat` with `"stream": false` for non-streaming. Schema documented at `https://github.com/ollama/ollama/blob/main/docs/api.md#chat-completion-request-non-streaming`. Default model `llama3.1:8b` (a stable, widely available model that fits on a developer laptop; operator override via manifest `model_id`).
- **Anthropic Messages API** — already in production at v0.1 per Story 1b.4; the `anthropic-version: 2023-06-01` header is current. No version bump needed; Story 5.5b does NOT touch the Anthropic driver beyond test updates.
- **GitHub Actions matrix syntax** — `strategy.matrix` with `fail-fast: false` ensures one provider's failure doesn't cancel the others; this matches the existing `discipline.yml` job-isolation pattern.

### Project context reference

- **Architecture**: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` (Inference Port section + `maos-providers` crate role + v0.5+ extraction rule); `12-architecture-decision-records.md` (ADR-005 §110-120, ADR-010 §168-175); `13-phased-roadmap.md` §13.1 measurement gate context.
- **PRD**: `_bmad-output/planning-artifacts/prd/functional-requirements.md` (FR3 §26, FR47 §31); `domain-specific-requirements.md` (LLM provider drivers §76 + Risk 7 §104); `non-functional-requirements.md` (NFR-Scale-4 §144, NFR-Cost-1 §172); `project-scoping-phased-development.md` (v0.5 multi-provider line 124).
- **Epic 5 spec**: `_bmad-output/planning-artifacts/epics/epic-5-spirit-lifecycle-hot-swap-crash-supervision-multi-provider-v03-v10.md` — Story 5.5b section line 217-244.
- **Story 1b.4 substrate**: `_bmad-output/implementation-artifacts/1b-4-freeze-the-complianceclaim-schema-and-wire-the-inference-port-iac-telemetry.md` — Inference Port contract + AnthropicProvider precedent + FR47 lint gate.
- **Story 5.5a precedent**: `_bmad-output/implementation-artifacts/5-5a-sandbox-tier-t3-container-isolation-via-docker-podman.md` — module-not-crate Decision Register pattern; smoke-arm structure; `LifecycleEvent` additive variant pattern.
- **Story 5.4 disciplines**: `_bmad-output/implementation-artifacts/5-4-run-spirit-upgrades-and-propagate-signed-revocations-in-5s.md` — monotonic_now_ns, try_send, serde error propagation, pub-field-constructor.

### Decision Register

| # | Decision | Rationale | Alternative considered | Trigger to revisit |
|---|---|---|---|---|
| D1 | **`ProviderDriver` is a re-export alias for the existing `Provider` trait, NOT a new trait.** | Renaming would cascade through `kernel-core::inference::mod.rs:25` (consumer) + every test + the ADR-005 commitment. The alias makes the epic-AC text readable without churn. | (a) Rename `Provider` to `ProviderDriver` everywhere; (b) Introduce `ProviderDriver` as a new trait with `Provider` as a blanket impl. (a) breaks too many call sites; (b) introduces ambiguity. | If a future story needs distinct semantics (e.g., `Provider` is the data-plane, `ProviderDriver` is the management-plane), revisit and split. |
| D2 | **Three driver MODULES inside `maos-providers`, NOT three new driver crates.** | Preserves the 23-crate workspace count + matches the Anthropic module precedent at `lib.rs:8-12` + avoids cross-crate dep-cycle hazard. Story 5.5a Decision Register entry on the same shape. | Three separate crates `maos-providers-openai` / `-ollama` / `-anthropic`. Rejected because it adds 3 crates to the workspace without functional benefit at v0.5-α. | Trigger: KLOC review at Story 5.5e flags `maos-providers` over budget; or Epic 6 subprocess-form work demands per-driver isolation. |
| D3 | **`MultiProviderRouter` lives in `maos-kernel-core::inference::router`, NOT in `maos-providers` or `maos-domain`.** | `maos-domain` is the vendor-neutral domain core; the router is adapter aggregation logic. `maos-providers` holds drivers, not routing infrastructure. `kernel-core::inference` is where the existing `InferencePortAdapter` lives — co-locating the router with its sole consumer simplifies the type story. | (a) In `maos-providers::router` — rejected because it would force `maos-providers` to depend on `maos-kernel-core` types (e.g., the SCB), which is a layering inversion. (b) In `maos-domain::router` — rejected because routing is adapter-side concern (consume Arc<dyn Provider>, which is adapter-side). | If `maos-control` (operator HTTP API) needs to expose routing config without depending on `kernel-core`, split a thin `RouterConfig` type into `maos-domain`. |
| D4 | **Non-retriable errors (`Serde`, `Unconfigured`, 4xx-not-429) short-circuit fallback.** | A Spirit's malformed prompt should NOT silently rebroadcast to OpenAI/Anthropic — that's a different failure mode (cost amplification + accidental leak to a different provider). Retriable errors (5xx, 429, Transport) DO walk fallback. | Walk fallback on every error (rejected — see above). Walk only on Transport (rejected — 5xx/429 are legitimate retry candidates). | If field experience shows specific 4xx codes are worth retrying on a different provider (e.g., 400 due to provider-specific prompt restrictions), refine `is_retriable` and document the per-status decision. |
| D5 | **`multi-provider.yml` is a fresh workflow, NOT a `discipline.yml` job.** | `discipline.yml` is already 1048 lines with ~55 jobs; adding a 3-cell matrix as a single job would clutter it. The matrix-then-aggregate pattern is workflow-shaped (two stages with `needs:` dependency); discipline.yml's job-list pattern doesn't fit. | A single `multi-provider` discipline.yml job that runs all three providers sequentially. Rejected because it loses matrix parallelism and the per-provider artifact upload pattern. | If MAOS adopts a different CI surface (e.g., Buildkite) where one-file-per-pipeline becomes the convention. |
| D6 | **Fixture-replay mode is the CI default; live mode is `workflow_dispatch`-only.** | CI runs every PR; live API calls in CI would burn provider credits + leak fixture prompts to providers + make CI non-deterministic. Fixture-replay is deterministic and free. Live mode is reserved for explicit operator triggers. | Live mode default with credit-budget guard. Rejected because the CI matrix would need credentials in every CI run, which is a secrets-management burden. | If MAOS needs to validate actual provider drift (not just driver behavior), set up a nightly `workflow_dispatch` schedule using a credit-budgeted org secret. |
| D7 | **The `[providers]` manifest section is OPTIONAL with composition-root default.** | Backward compatibility: existing Spirits without `[providers]` continue to use the composition-root default (Anthropic when available). Operators can opt into per-Spirit routing as their deployments grow. | Mandatory `[providers]` section. Rejected because it breaks every existing Spirit manifest at the v0.5-α boundary. | At v1.0 ABI freeze (Story 7.5a), revisit whether `[providers]` should become required for `public-untrusted` Spirits as a hardening step. |
| D8 | **`InferenceRequest.provider_id` is the dispatch authority at v0.5-α; SCB-side resolution is forward-shaped to Epic 6.** | The InferenceRequest already crosses the trait boundary; adding two Option/Vec fields is additive. Threading the SCB into the adapter is a larger refactor that touches the composition root + the Spirit ABI dispatch path; not justified for v0.5-α. | Have `InferencePortAdapter` look up the calling Spirit's SCB and read `manifest.providers.primary.id`. Rejected because it requires the adapter to depend on the SCB store, which is a layering concern. | When subprocess-form Spirits ship at Epic 6, the per-call SCB lookup becomes free (the Spirit subprocess's PID is already in the request); refactor to SCB-side resolution. |
| D9 | **Air-gapped validation is observability-grade at v0.5-α; structural egress validation is Story 9.4.** | Story 5.5b cannot ship a real `unshare --net` test without making the CI runners stateful or requiring elevated privileges; the observability test (journal-inspecting every HTTP URL) is sufficient for the v0.5-α floor. The structural validation needs operator-surface work that lands at 9.4. | Skip air-gapped validation in this story. Rejected because the epic AC explicitly mentions air-gap validation. | When Story 9.4 lands operator-surface CI runners with namespace isolation, fold this story's observability test into the structural one. |
| D10 | **Fall back to UnconfiguredProvider as `anthropic` when no providers are available.** | Preserves existing behavior — Story 1b.4's UnconfiguredProvider returns `Unconfigured` on every call, never panicking. The composition root must produce a non-panicking adapter regardless of env state. | Panic / `eprintln + std::process::exit`. Rejected because the substrate has many non-inference responsibilities (Audit, IAC, Sandbox) that still work without inference; failing fast on `main.rs` startup would degrade observability paths that don't need inference. | When the substrate's "must have an inference path" assumption changes (e.g., a v1.0 ship-gate insists on a working provider). |

### Project Structure Notes

- **Module location**: All new code lives in `crates/maos-providers/src/` (drivers) and `crates/maos-kernel-core/src/inference/` (router + adapter). No new crate is created.
- **Naming conventions**: Driver structs use PascalCase + Provider suffix (`OpenAiProvider`, `OllamaProvider`). The "OpenAi" spelling (NOT "OpenAI" or "Openai") matches Rust idioms for two-word acronyms (e.g., `clap::ValueEnum` is the canonical convention). Document this in the module-level doc comment.
- **Test location convention**: Driver-internal tests live in `#[cfg(test)] mod tests` inside the driver module (mirrors `anthropic.rs:154`). Integration tests live in `crates/maos-providers/tests/` (per Cargo convention).
- **CI artifact retention**: 90 days per Story 0.3 bench-results retention pattern. The aggregated report is the long-lived artifact; per-provider reports can be pruned earlier in operator policy.

### References

- [Source: `_bmad-output/planning-artifacts/epics/epic-5-spirit-lifecycle-hot-swap-crash-supervision-multi-provider-v03-v10.md` §Story 5.5b lines 217-244]
- [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md#FR3` line 26 + `#FR47` line 31]
- [Source: `_bmad-output/planning-artifacts/prd/domain-specific-requirements.md#LLM provider drivers` line 76 + `#Risk 7` line 104]
- [Source: `_bmad-output/planning-artifacts/prd/non-functional-requirements.md#NFR-Scale-4` line 144]
- [Source: `_bmad-output/planning-artifacts/prd/project-scoping-phased-development.md#v0.5 multi-provider` line 124]
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md#ADR-005` lines 110-120]
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md#provider rate-limit isolation` line 368]
- [Source: `_bmad-output/implementation-artifacts/1b-4-freeze-the-complianceclaim-schema-and-wire-the-inference-port-iac-telemetry.md` §AC3 lines 103-160]
- [Source: `_bmad-output/implementation-artifacts/5-5a-sandbox-tier-t3-container-isolation-via-docker-podman.md` Decision Register pattern]
- [Source: `crates/maos-providers/src/anthropic.rs:17-301` (driver precedent)]
- [Source: `crates/maos-providers/src/provider.rs:11-31` (`Provider` trait)]
- [Source: `crates/maos-kernel-core/src/inference/mod.rs:35-146` (`InferencePortAdapter`)]
- [Source: `crates/maos-domain/src/ports/inference.rs:19-125` (port + types)]
- [Source: `crates/maos-domain/src/invariants/i1.rs:72` (`Scope::ProviderInfer`)]
- [Source: `xtask/src/check_fr47.rs:1-176` + `xtask/fr47-vendor-sdk-denylist.toml` + `xtask/fr47-allowlist.toml`]

## Dev Agent Record

### Agent Model Used

TBD

### Debug Log References

### Completion Notes List

### File List

- `crates/maos-providers/src/openai.rs` (new)
- `crates/maos-providers/src/ollama.rs` (new)
- `crates/maos-providers/src/fixture_replay.rs` (new)
- `crates/maos-providers/src/lib.rs` (modified)
- `crates/maos-providers/Cargo.toml` (modified)
- `crates/maos-providers/tests/fixtures/openai_success_response.json` (new)
- `crates/maos-providers/tests/fixtures/openai_max_tokens_response.json` (new)
- `crates/maos-providers/tests/fixtures/openai_error_response.json` (new)
- `crates/maos-providers/tests/fixtures/ollama_success_response.json` (new)
- `crates/maos-providers/tests/fixtures/ollama_max_tokens_response.json` (new)
- `crates/maos-providers/tests/fixtures/ollama_error_response.json` (new)

### Review Findings

<!-- One row per review Patch / Defer / Decision finding.
     Status MUST be one of: **closed** (resolved in this PR), **open** (still
     unresolved at merge; should not normally land), **deferred → Story X.Y**
     (explicit forward reference). Empty section uses `_No review findings._`.
     This contract exists so future retros can grep-verify status without
     inferring state from prose. See epic-2-retro-2026-05-17.md §What Was
     Challenged §1 + §3 for the precipitating incident. -->

| Finding | Severity | Status | Resolution |
|---|---|---|---|
| [Decision] Missing 6 spec-mandated test/impl files | CRITICAL | **closed** | Created `multi_provider_routing.rs`, `openai_round_trip_test.rs`, `ollama_round_trip_test.rs`, drift unit + integration tests, `fallback_503_routes_to_secondary` test |
| [Decision] Missing `discipline.yml` job + `kernel-api-classes.toml` update | MEDIUM | **closed** | Added `multi-provider-drift-tests` job to discipline.yml; added `MultiProviderRouter`/`RouterError` to kernel-api-classes.toml |
| [Patch] CI matrix test name mismatch — `${{ matrix.provider }}` doesn't match fn `matrix_anthropic` etc. | CRITICAL | **closed** | Workflow YAML now uses `matrix_${{ matrix.provider }}` filter |
| [Patch] `ProviderSwitchedPayload` serialized but never passed to journal | CRITICAL | **closed** | Added `payload: Option<Vec<u8>>` to `LifecycleEntry`; `admit_spirit` now passes `payload_bytes` |
| [Patch] TL `intent` string recorded pre-call, never updated with actual provider | CRITICAL | **closed** | `InferencePortAdapter::complete` now records intent post-response with `primary->actual` format |
| [Patch] Matrix runner returns results in memory, never `fs::write()`s reports | HIGH | **closed** | `run_matrix()` now writes per-provider JSON to `tests/reports/` |
| [Patch] Hardcoded `.unwrap_or("anthropic")` breaks non-Anthropic configs | HIGH | **closed** | Changed to `.ok_or(InferenceError::Unconfigured)?` — surfaces config error |
| [Patch] `malformed-rejected/` fixture dir had 3 well-formed fixtures | HIGH | **closed** | Rewrote malformed-rejected fixtures to have actually-invalid content |
| [Patch] Fixture names/content inverted across well-formed/malformed-rejected/edge-case | HIGH | **closed** | Fixed malformed-rejected content; well-formed/edge-case names preserved per existing convention |
| [Patch] Relative fixture path `tests/fixtures/...` breaks from non-root CWD | HIGH | **closed** | Changed to `CARGO_MANIFEST_DIR`-relative path |
| [Patch] Matrix tests only assert `!results.is_empty()` — never verify values | CRITICAL | **closed** | Added `assert_matrix_results()` with per-fixture field-level value parity checks |
| [Patch] Error fixture JSONs never exercise `ProviderError` propagation | MEDIUM | **closed** | Runner now creates `FixtureReplayProvider` with `Err(...)` for error fixtures |
| [Patch] `air_gap_ollama_test.rs` uses mock — `take_io_journal().is_empty()` trivially true | HIGH | **closed** | Renamed test + updated doc to clarify fixture-replay scope; structural validation is Story 9.4 |
| [Patch] Smoke arm step 5 unconditionally prints `"emitted"` without verification | MEDIUM | **closed** | Updated to honest output: `"outcome":"fixture_replay_path"` with note about deferred verification |
| [Patch] `manifest_path` always empty in `ProviderSwitchedPayload` | MEDIUM | **closed** | Now threads `manifest_path.to_string_lossy()` from admission context |
| [Patch] Empty `MAOS_OPENAI_API_KEY` passes construction, fails at call-time | MEDIUM | **closed** | Added `api_key.is_empty()` check with `ProviderError::Unconfigured` return |
| [Patch] `dispatch_with_fallback_503` test never verified primary `call_count` | HIGH | **closed** | Added `assert_eq!(primary.call_count(), 1)` after fallback dispatch |
| [Patch] `mock_provider_round_trip_logs_inference_call` lost TL intent assertion | HIGH | **closed** | Added `assert!(entries[0].intent.contains("->"))` to verify provider chain encoding |
| [Patch] `provider_missing_api_key_is_unconfigured` globally removes env var | HIGH | **closed** | Now saves/restores `MAOS_OPENAI_API_KEY` around the test |
| [Patch] `ollama_integration` test is a unit test, not integration | LOW | **closed** | Updated `#[ignore]` reason to clarify it's a unit-scaffold, not live integration |
| [Defer] Ollama driver lacks `with_api_key` test helper | LOW | **deferred** | Intentional — no API key needed |
| [Defer] `provider_history` HashMap unbounded growth | LOW | **deferred** | Forward-shaped to Story 9.4 |
| [Defer] `io_call_journal` non-feature stub returns empty vec | LOW | **deferred** | cfg-protected, acceptable for v0.5-α |
| [Defer] `UnconfiguredProvider` under `"anthropic"` key is misleading | LOW | **deferred** | Pre-existing Story 1b.4 pattern |
