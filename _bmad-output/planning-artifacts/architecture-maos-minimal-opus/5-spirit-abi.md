# 5. Spirit ABI

The Spirit ABI is the contract between the kernel and a Spirit. Every Spirit conforms to it. The kernel does not negotiate; the Spirit either matches the ABI version or refuses to load.

**A Spirit's implementation is *behavior*, not *infrastructure*.** A Spirit's code contains lifecycle hook handlers, IAC frame handlers, telemetry handlers, decision logic, the system-prompt template, and (optionally) the output/explanation/epistemic predicate callbacks. **It does not contain HTTP libraries, LLM provider SDKs, MCP client implementations, socket code, or filesystem code.** All that work flows through Layer 1 capabilities which the kernel implements. The Spirit calls `capability/invoke(token, args)` and receives a stream of typed events; the kernel does the actual HTTP, the actual SDK calls, the actual sandboxed exec, the actual MCP wire protocol.

A Spirit binary therefore stays small (the Rust reference implementations target hundreds of KB to a few MB). Sharing a Spirit becomes cheap. Polyglot Spirit ecosystems become feasible — a TypeScript Spirit and a C# Spirit speak the same wire protocol because neither imports an HTTP library; both delegate to the kernel's adapters. And every provider call, every MCP invocation, every shell exec is uniformly audited via the Capability Registry — there is no Spirit shortcut path that bypasses the kernel by talking directly to an HTTP endpoint. **The Spirit author's job is to design behavior; the kernel's job is to be the substrate that behavior runs on.**

## 5.1 Spirit Manifest schema

The manifest is a TOML file declaring everything the kernel needs to load, sandbox, schedule, and audit a Spirit class.

```toml
[class]
name = "code-reviewer-pro"
version = "1.2.0"
abi = "1.0"
manifest_schema_version = 1
min_substrate_version = "1.0.0"     # kernel rejects load if its own version is below this
forms = ["subprocess"]               # which Spirit forms this class ships in: rust-inproc | subprocess
trust_tier = "public-untrusted"      # local | org-internal | public-untrusted
signing_key = "ed25519:xxx..."       # author's public key
description = "..."

[capabilities.required]
fs.read = ["**/*.rs"]
fs.write = ["**/*.rs"]                # gated by approval
provider.complete = ["anthropic.claude-3-5-sonnet"]
mcp.call = ["github.search"]
iac.send = ["broadcast", "spirit:peer:bilateral"]

[capabilities.parallelism]
max_concurrent_tool_calls = 4

[posture]
default = "assistive"                 # cautious | assistive | autonomous-with-halt | autonomous
allowed_max = "autonomous-with-halt"  # ceiling beyond which the Spirit cannot self-shift

[output_shape]
# Predicate over emitted frames. Kernel rejects emits failing this shape.
required_fields = ["severity", "file", "line", "suggestion"]
predicates = ["..."]

[explanation_shape]
# For decision.* frames: which "because" payload is mandatory
required_fields = ["evidence_refs", "alternatives_considered", "confidence"]

[epistemic_policy]
# Per-tag rules; kernel maps output frame tags to verbalize_only | flag | halt
[[epistemic_policy.rule]]
tag = "claim.security_vulnerability"
action = "halt"
on_confidence_below = 0.85
on_evidence_conflict = true

[[epistemic_policy.rule]]
tag = "claim.style_suggestion"
action = "verbalize_only"

default_action = "verbalize_only"

[budget]
context_window_size = 200000
context_pressure_threshold = 0.80     # emits ContextPressure
context_limit_threshold = 0.95        # emits ContextLimit
time_cap_seconds = 300                # soft warning at 80%; kernel emits BudgetWarning
cost_cap_usd_per_hour = 10.00

[skills.search_path]
paths = ["~/.maos/skills/", "_bmad/skills/", "/usr/share/maos/skills/"]

[hot_swap]
state_schema_uri = "https://schemas.maos.dev/spirit-state/code-reviewer-pro/v1.cbor"
state_schema_version = 1

[halt_protocol_compatibility]
version = 2                           # halts produced under this Spirit class can be migrated
                                       # to a successor declaring halt_protocol_compatibility >= 2

[intent_promotion_set]
# When THIS Spirit consumes a digest, which intent_lineage classes are admissible
allowed = ["consult", "review"]
# Digests with intent_lineage NOT in this set are rejected with EIntentPromotionDenied

[migrates_from]
versions = ["1.0", "1.1"]              # cross-major migration via the migrate() ABI entry point

[swap_invariants]
preserve = ["open_pr_state", "review_queue"]   # HSIS-tested invariants

[resources]
# Cgroups v2 / setrlimit / Job Object ceilings for subprocess-form Spirits
cpu_max_pct = 50
memory_max_mb = 512
fd_max = 64

[sandbox]
tier = "T2"                           # kernel applies strictest-of (manifest, trust-tier, operator-policy)

[forbidden_capabilities]
# Negative assertion; kernel enforces never holding tokens for these
deny = ["bash.exec", "git.commit"]

[lifecycle]
on_load    = ["spirit-code-reviewer-pro::hooks::on_load"]
on_idle    = ["spirit-code-reviewer-pro::hooks::on_idle"]
on_swap_in = ["spirit-code-reviewer-pro::hooks::on_swap_in"]

[author]
name = "Diego Hernandez"
contact = "diego@example.com"
homepage = "https://github.com/diego/code-reviewer-pro"
```

The schema is versioned (`manifest_schema_version`) independently of the kernel and ABI; the kernel ships a compatibility matrix in `STABILITY.md`.

## 5.2 Spirit Wire Protocol (subprocess form)

Subprocess Spirits speak a JSON-RPC-shaped protocol over stdio with CBOR payloads. Wire-level details:

**Framing.** LSP-style: `Content-Length: <decimal>\r\n\r\n` followed by exactly N bytes of CBOR-encoded payload. Header is ASCII, case-insensitive name, max header block 4 KiB.

**Backpressure.** `BufReader` cap = 1 MiB; frames exceeding cap = `WireError::Oversize`, halt the Spirit. Writer uses `tokio::io::AsyncWriteExt::write_all` over a bounded `mpsc<Frame>(64)` — channel full = backpressure to caller, never drop.

**Stderr separation.** A separate `tokio::process::ChildStderr` is piped to `tracing` at `WARN` level with the `spirit_id` span. Never multiplexed onto stdout; out-of-band Spirit logs go through stderr only.

**EOF semantics.** Clean EOF after the last full frame = `Halt::Voluntary`. EOF mid-frame = `Halt::Fault(Truncated)`.

**Signal handling.** SIGTERM → 5-second grace period → SIGKILL. The supervisor records the halt cause.

**Method set (kernel-to-Spirit, lifecycle):**
- `lifecycle/load(manifest)` → `loaded`
- `lifecycle/start()` → `started`
- `lifecycle/swap_in(predecessor_state)` → `running` — hot-swap; you inherit this state
- `lifecycle/snapshot()` → `<state CBOR>` — produce hot-swap snapshot
- `lifecycle/pause()` → `paused`
- `lifecycle/resume()` → `running`
- `lifecycle/unload()` → `unloaded`
- `lifecycle/migrate(predecessor_state)` → `successor_state` — cross-major migration entry point
- `event/inbound(frame)` → `()` — IAC frame delivery; kernel writes a shadow-recall record before invocation (per I12)
- `event/telemetry(event)` → `()` — telemetry tick delivery
- `event/idle()` → `()` — `on_idle` lifecycle hook fire
- `epistemic/resolve(halt_id, resolution)` → `resumed | unloaded | halted`

**Method set (Spirit-to-kernel, capability invocation):**
- `capability/invoke(token, args)` → stream of typed events
- `iac/send(frame)` → `frame_id`
- `iac/recall(filter, limit, cursor)` → `[frame_ids]` — `log.recall` per the audit-chain primitive
- `iac/fetch(frame_id)` → `frame_payload`
- `iac/broadcast(topic, frame)` → `()`
- `mem/read(scope, key)` → `value`
- `mem/write(scope, key, value)` → `()`
- `working_memory/set_scalar(tag, value, derived_from)` → `()`
- `epistemic/halt(payload)` → `halt_id`
- `approval/request(intent, target, capability)` → `decision`

**Cross-language byte-equal golden corpus.** Every frame variant ships with a `golden/<frame_name>.json` (authoritative reference shape) and `golden/<frame_name>.cbor` (canonical encoding); every language SDK must serialize a constructed frame to byte-equal CBOR and deserialize golden to a structurally-equal frame. Canonical encoding: sorted keys, no whitespace, UTF-8 NFC. Floor: 100% per frame variant per SDK at v1.0 ship gate.

**Wire-protocol fuzz commitment — tiered cadence ladder.** Three tiers, all mandatory; cumulative floor non-negotiable.

| Tier | Cadence | Time budget | Corpus seed | Failure gate |
|---|---|---|---|---|
| **T1 — Per-commit** | Every PR, blocking | **10 min wall-clock** on N=4 parallel workers (≈40 CPU-min) | Last-known-bad regressions + 500 mutated frames from coverage-guided pool | Any new crash, any stalled handshake >30s, any auth bypass |
| **T2 — Nightly** | 1×/day on main | **4 hours wall-clock** on N=8 workers (≈32 CPU-hours) | Full grammar fuzz + 5k mutated frames + dictionary-guided | Any crash, any state-machine deviation, p99 frame-parse latency regression >20% |
| **T3 — Pre-release** | Every release candidate, blocking | **24 hours wall-clock** on N=8 workers (≈192 CPU-hours) | T2 corpus + adversarial-Spirit transcripts + replay corpus | Zero crashes, zero auth bypasses, zero TLS downgrade paths |

**Cumulative pre-GA floor — per-target, not aggregate.** For each fuzz target T in `crates/iac/fuzz/`, the sum of `libfuzzer.exec_time_seconds` across all T1+T2+T3 runs in the 90 days preceding the GA tag MUST be ≥ **72 CPU-hours per target** (measured by libFuzzer's own runtime counter, summed across parallel workers; not wall-clock). Aggregate floor across all targets MUST be ≥ **1,000 CPU-hours**. CI publishes `fuzz_cpu_hours_per_target_90d` to Prometheus; release gate fails if any target < 72 or aggregate < 1,000.

**Catch-up rule (T2-elastic, T3-fixed).** If at the release-candidate cut, `fuzz_cpu_hours_per_target_90d` < 72 for any target, T2 nightly is extended on the deficient targets only — 4-hour nightly runs replaced with 12-hour nightly runs on those targets until each clears 72 CPU-hours or the GA date is reached. T3 budget remains fixed at 24h; T3 does not absorb T2 deficit. If GA arrives with any target still under floor, release is blocked.

*The tiered cadence is the execution model; the per-target floor is the gate.* Reducing T1 per-commit budget is a developer-experience optimization. Reducing the per-target or aggregate floor is a major-version conversation (ADR-037 `invariant-lock` gate fires). The unit is **CPU-hours by libFuzzer counter**, not wall-clock — wall-clock is gameable by adding workers; libFuzzer's own counter is not.

## 5.3 Lifecycle hooks

The hooks below are the part that makes hot-swap possible. The Spirit may handle any subset; unhandled hooks are no-ops.

The FR55 contract commits to 11 hooks at Epic 2. The remaining 3 (`on_swap_out`, `snapshot`, `migrate`) ship in Story 5.2 (hot-swap state transfer), and `epistemic_resolve` ships in Story 4.1 (halt-protocol resolution).

| Hook | Fires when | What the Spirit does | Implemented at |
|---|---|---|---|
| `on_load` | Manifest read, capability tokens issued, Spirit loaded into memory | Initialize state, open persistent connections, load skills | Story 2.1 (signature), Story 5.1 (runtime) |
| `on_start` | First IAC frame routed to this Spirit | Begin operating | Story 2.1 (signature), Story 5.1 (runtime) |
| `on_frame(frame)` | An IAC frame addressed to this Spirit lands | Decide and emit response frame(s) | Story 2.1 (signature), Story 5.1 (runtime) |
| `on_telemetry_event(event)` | A subscribed telemetry topic emits | Update working state, possibly emit derived frames | Story 2.1 (signature), Story 5.1 (runtime) |
| `on_schedule` | A scheduled invocation fires | Run periodic task | Story 2.1 (signature), Story 5.1 (runtime) |
| `on_idle` | No work for ≥30s (configurable) | Proactive opportunity (Butler!); else no-op | Story 2.1 (signature), Story 5.1 (runtime) |
| `on_swap_out` | Kernel about to swap this Spirit out | Final state blob; in-flight tokens enumerated | Story 5.2 |
| `on_swap_in(predecessor_state)` | This Spirit is the successor in a hot-swap | Inherit state; rebind in-flight tokens | Story 2.1 (signature), Story 5.2 (state transfer) |
| `snapshot()` → state | Kernel requests a hot-swap snapshot | Produce CBOR-encoded state per `[hot_swap].state_schema_version` | Story 5.2 |
| `migrate(predecessor_state)` → successor_state | Cross-major migration; predecessor's class is in this class's `migrates_from` list | Translate predecessor's schema to this class's schema | Story 5.2 |
| `epistemic_resolve(halt_id, resolution)` | User responded to a halt | Process resolution; transition back to `Running` or accept halt | Story 4.1 |
| `on_pause` | Operator paused this Spirit | Drop in-flight non-critical work; preserve halt state | Story 2.1 (signature), Story 5.1 (runtime) |
| `on_resume` | Operator resumed | Resume | Story 2.1 (signature), Story 5.1 (runtime) |
| `on_unload` | Graceful shutdown | Persist final state; close connections | Story 2.1 (signature), Story 5.1 (runtime) |
| `on_consolidate` | Spirit-author-defined cadence for memory-curation passes | Compact private memory; produce digests | Story 2.1 (signature), Story 5.1 (runtime) |

## 5.4 Posture

Posture is the Spirit's autonomy stance. Posture is mutable; class is not. The user can shift posture at runtime (`Butler, be more cautious for the next hour`); the kernel logs the shift and applies it to subsequent capability-scope decisions. Posture-shift propagation: P99 ≤2s, P99.9 ≤5s.

| Posture | Behavior |
|---|---|
| `cautious` | Every capability invocation prompts |
| `assistive` | Reads silent allow; writes prompt; default for most Spirit classes |
| `autonomous-with-halt` | Proceed unless `[epistemic_policy]` triggers a halt; user resolves via halt mechanism |
| `autonomous` | Proceed without prompts; rare; explicit user grant; halt mechanism still active |

The manifest's `[posture].allowed_max` sets a ceiling beyond which the Spirit cannot self-shift. The operator may override the ceiling per deployment.

> **v0.3 prerequisite — Spirit-author scaffolding (Story 2.3):** Spirit authors at v0.3 prerequisite scaffold a new Rust Spirit via `cargo generate --git https://github.com/lunarpulse/maos templates/spirit-rust --name my-spirit`. The generated crate uses the `#[spirit]` proc-macro from Story 2.1, declares a TOML manifest mirroring the hello-spirit shape, and ships a test driven by `maos_spirit_sdk::local_runner::LocalRunner` (no kernel instance required). The baked output is committed at `examples/example-spirit/` and CI-enforced via `example-spirit-tests` + `example-spirit-drift`. Per-language templates (TS / Python / Go) land in Story 7.1; the NFR-Onb-1 30-Min First Spirit Validation Gate executes at Story 7.5b against Butler from Story 8.1.
>
> **v0.3 prerequisite — spirit-test SDK seed (Story 2.4):** Spirit authors at v0.3 prerequisite gain the `spirit_test` cargo feature on `maos-spirit-sdk` exposing `SpiritTest<S>` (wraps `LocalRunner` with halt resolution + manifest self-check + frame capture), 5 assertion macros (`assert_emits_frame!`, `assert_halts_with!`, `assert_hook_fired!`, `assert_no_capability_invocation!`, `assert_manifest_well_formed!`), the 3-kind halt resolution simulator (forward-anchor for Story 4.1 — `ProvidedContext`, `AcceptedHalt`, `AuthorizedOverride`), and the cross-Spirit memory isolation framework hooks (`IsolationHookPoint` 4-point trait + `CrossSpiritIsolationFixture` 2-Spirit harness + 8-category `IsolationAttackCategory` enum per §8.1). The LCAS clearly-decidable 70-item bucket ships at `tests/corpora/lcas-v0.3.jsonl`. Full per-language SDK with judge-LLM agreement layer + registry publish path lands at Story 7.1 at v0.5+; the NFR-Sec-14 200-scenario adversarial corpus (Sec-14a + Sec-14b) lands at Story 4.5 at v0.8.
>
> **v0.5 binding — Full spirit-test SDK + per-language scaffolding (Story 7.1):** Story 7.1 lands the v0.5 Spirit-author ecosystem milestone. (1) Three new assertion macros — `spirit_test::assert!(condition, "diagnostic")`, `spirit_test::expect_frame!(report, kind = ..., bytes_matches = ..., bytes_exact = ..., from_spirit = ...)`, `spirit_test::expect_halt!(report, halt_id = ..., kind_matches = ...)` — provide structured, file+line+condition diagnostics on failure. The v0.3 macros are preserved for backward compat. (2) `Ctx::deprecation_warnings()` channel + `DeprecationWarning` type — the channel is empty-present at v0.5 (zero deprecations); Story 7.5a's ABI compatibility matrix gate consumes it at v1.0. (3) TypeScript SDK seed at `sdks/spirit-ts/` — a test harness (not a kernel runtime per ADR-002); TypeScript Spirits in production use the subprocess form via Story 6.2 CliWrapperSpirit. (4) NFR-Test-3 structural surface — `tests/coverage-matrix.yaml` gains the 5-Spirit `reference_spirits` slot table with `xtask coverage-matrix --measure-nfr-test-3` walker; floor is soft at v0.5, hard at v1.0 per Story 7.5a. Cross-references: Story 7.5a (ABI compatibility matrix), Story 7.5b (30-Min Gate), Story 10.2 (N=12 third-party trial).
>
> **v0.5 binding — Skill ecosystem (Story 7.4):** Story 7.4 stands up the real skill ecosystem in the NEW `maos-skill` crate (workspace 29→30), replacing the `skill_bundle: Vec<String>` persona-reference placeholder. (1) `maos.skill.v1` schema (ADR-027) — markdown body + TOML frontmatter (`id`, `version` (semver), `name`, `description`, optional `required_capabilities`, `min_substrate_version`), all `#[serde(deny_unknown_fields)]`; `parse_skill` rejects an unknown field as `ESkillSchema::UnknownField` (never a silent default). (2) Three operator-admission queue entry paths (FR39) — **package-shipped**, **`skill.author.self`** (the new `Scope::SkillAuthorSelf` capability authorizes the WRITE-to-queue ONLY, never activation), and **FR57 revision proposal** (built from the Spirit's own Story 4.3 `SelfTelemetryReport`); EVERY path lands `Pending` and requires an explicit operator `approve` — no skill is auto-admitted. (3) §4.0.7 boundary — the kernel validates the SCHEMA, discovers, and manages admission/audit; the markdown `body` and the FR57 `proposed_diff` are OPAQUE (the kernel does not write/rank/curate/interpret skill content). Skills are filesystem-discovered at v0.5 (the three `[skills.search_path]` roots); `min_substrate_version` is parsed but its kernel-load enforcement is Story 7.5a. Operator surface: `maosctl skills <list|approve|reject>`. ABI: `Scope::SkillAuthorSelf` is additive on the `#[non_exhaustive]` enum; `ABI_VERSION` stays 1.
