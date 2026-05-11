# Epic 5: Spirit Lifecycle, Hot-Swap, Crash Supervision & Multi-Provider (v0.3 → v1.0)

**Goal:** Operator runs `maosctl spirit upgrade butler --to 0.3.2` with hot-swap policy; all in-flight tokens, working-memory state, and active halts preserved across the swap. Subprocess Spirit crashes within 2s; tasks NACKed within 5s. Three providers run in CI by v0.5.

**Owns:**
- Full Spirit Scheduler lifecycle: `load`, `start`, `pause`, `resume`, `unload` via authenticated control plane (CLI, ACP editor surface, operator HTTP API).
- Hot-swap state-transfer wire format (ADR-017, binding-v0.3): CBOR-encoded payloads conforming to per-Spirit-class schema declared in manifest `[hot_swap].state_schema_uri + state_schema_version`. Same-major+additive = forward-compat; same-major+breaking = forbidden; cross-major requires migrator. Saga-style compensation: on `on_swap_out` failure restore predecessor; on `on_swap_in` failure discard successor; auto-revert within 30s on post-swap invariant violation.
- Cross-major migration (ADR-020): `migrate(predecessor_state) -> Result<successor_state, Error>` declared via `migrates_from` manifest field; kernel refuses load with `EMigratorMissing` if predecessor archive exists and no migrator declared.
- Crash detection ≤2s for SIGKILL corpus (≥99/100 floor); `task.orphaned` IAC frame within 5s with exit-cause journaled.
- Hung-Spirit detection: no-progress IAC for >30s emits `task.stalled` event; ≥48/50 reclassified within 60s on hang corpus.
- Silent-failure detection (NFR-Rel-4): `silent_failure_suspect` event when no progress IAC for >30s despite healthy heartbeats; ≥45/50 on adversarial zombie-heartbeat corpus.
- Signed Revocation List artifact (CRL): registry-pushed (kernel polls every 5min) + offline-import path. Running Spirit instances receive `SpiritRevoked` event and execute declared revocation policy (terminate-immediately / drain-then-terminate / quarantine).
- Spirit upgrade with declared migration policy (FR49): hot-swap with state preservation (default) / cold-swap with re-init / migrator-mediated cross-major.
- Dead-Spirit task disposition (FR50): manifest `[on_crash].action` — NACK / reassign-to-replica / escalate-to-operator.
- Halt-continuity-across-hot-swap (I14, FR53) — **DEPENDENT on E4's halt schema; runtime path here**. Hot-Swap Coordinator rejects swaps that would orphan halts unless Spirit-author has declared `halt_protocol_compatibility = true` for predecessor's halt schema.
- Full lifecycle triggers: `on_load`, `on_start`, `on_frame`, `on_idle` (Butler v0.3 anchor), `on_telemetry_event`, `on_schedule`, `on_swap_in`, `on_pause`, `on_resume`, `on_unload`, `on_consolidate` — each carrying declared resource budgets per manifest.
- Cooperative + priority-weighted scheduling; OS-level CPU/memory budget enforcement via cgroups v2 / setrlimit / Job Objects.
- Sandbox tier T3 v0.5 (containers Docker/Podman).
- **Multi-Provider**: Inference Port full (Anthropic v0.1 → ≥3 providers v0.5 in CI: + OpenAI + local-LLM via Ollama; v1.5 = MAOS-mediated provider proxies; v2.0 = Bedrock/Vertex AI).
- MCP client all-three transports (stdio / SSE / Streamable HTTP) — Streamable HTTP default at v0.5.
- ACP server (NDJSON over stdio) for editor-hosted Spirits — Zed + VSCode tested at v1.0.
- Spirit registry over MCP-Streamable-HTTP (ADR-008, binding-v0.5): `registry.search` / `manifest` / `artifact` / `publish` / `deprecate` — three trust tiers (`local`, `org-internal`, `public-untrusted`).
- **§13.1 rust-inproc measurement story with go/no-go gate** before v0.5 ships: subprocess form measured against latency budgets (J1 <25ms P95 IPC; J4 <10ms P95). If subprocess meets budgets, rust-inproc form may be deferred. Otherwise, rust-inproc development unlocks.

**FRs covered:** FR3 (full provider config), FR9 (full lifecycle pause/resume + load/start/unload), FR10, FR11, FR12, FR13, FR49, FR50, FR53, FR55, FR5 (T3 v0.5), FR48 (CryptoProvider runtime usage).

**Key NFRs:** NFR-Rel-1 (crash detection ≤2s, `task.orphaned` ≤5s), NFR-Rel-2 (hung-Spirit ≤60s), NFR-Rel-3 (HSIS ≥95% per Spirit class — 6 class-specific corpora × 50 = **300 scenarios**), NFR-Rel-4, NFR-Rel-5 (rollback ≤30s), NFR-Rel-9 (revocation propagation ≤5s p99 under 10⁴ concurrent validations), NFR-Rel-10 (cold-restart ≤30s graceful / ≤1 in-flight message loss on hard kill), NFR-Rel-11 (halt-receipt ≥99.9%), NFR-Perf-7 (hot-swap P99 <500ms), NFR-Test-7 partial (rust-inproc ↔ subprocess semantic equivalence ≥90% — full gate in E10).

**Corpora authored in E5:**
- HSIS additional 4 Spirit-class corpora × 50 = 200 scenarios (Butler + Orchestrator + Worker + CliWrapper; cumulative HSIS 300/300).
- Provider contract corpus ~40 cases (Pact-style; ≥3 providers × interface variations).
- spirit-test SDK integration tests ~25 cases.

**Acceptance demo:** Hot-swap from Butler v0.3.1 to v0.3.2 preserves in-flight `task.assign` frames and active halt context; SIGKILL of subprocess Spirit detected in 1.8s; `task.orphaned` emitted at 3.2s; signed CRL polled and applied; halt-receipt journaled on clean shutdown.

### Stories

## Story 5.1: Ship Full Lifecycle Verbs and 11 Triggers with Priority-Weighted Scheduling

As an operator,
I want full lifecycle verbs (`load` / `start` / `pause` / `resume` / `unload`) via authenticated control plane (CLI / ACP / operator HTTP API) AND all 11 lifecycle triggers firing at declared cadence with manifest-budgeted resources, AND cooperative + priority-weighted scheduling with OS-level CPU/memory caps,
So that I can drive Spirit lifecycle without trusting Tokio-cooperation alone — OS enforcement is the floor.

**Acceptance Criteria:**

**Given** an authenticated control plane (CLI, ACP, operator HTTP API)
**When** the operator invokes `maosctl <load|start|pause|resume|unload> <spirit>`
**Then** the Spirit Scheduler executes the verb (FR9 full)
**And** the state transition is journaled to the Lifecycle Journal
**And** the transition is auditable via E9 audit-query surface

**Given** a Spirit declares lifecycle hooks in its manifest
**When** the kernel reaches the corresponding lifecycle event
**Then** the kernel fires the declared hook (`on_load`, `on_start`, `on_frame`, `on_idle`, `on_telemetry_event`, `on_schedule`, `on_swap_in`, `on_pause`, `on_resume`, `on_unload`, `on_consolidate`)
**And** the hook executes within its manifest-declared resource budget
**And** budget overruns emit `BudgetWarning` IAC frame at 80% (NFR-Perf-6)

**Given** cooperative + priority-weighted scheduling
**When** multiple Spirits compete for runtime
**Then** the Spirit Scheduler dispatches per declared weights with operator-configurable defaults
**And** OS-level CPU/memory budgets are enforced via cgroups v2 (Linux) / setrlimit (macOS) / Job Objects (Windows) — not via Tokio cooperation

**Given** the Butler `on_idle` substrate (v0.3 anchor)
**When** the Spirit Scheduler detects an idle window
**Then** the kernel fires `on_idle(ctx)` on Butler
**And** Butler can perform anticipatory reasoning within its budget (E8 Story 8.1 implements the Butler behavior)

## Story 5.2: Implement Hot-Swap State Transfer and Cross-Major Migration Against HSIS ≥95%

As an operator upgrading a running Spirit,
I want the Hot-Swap Coordinator to preserve in-flight tokens + working memory + active halts across the swap with CBOR state-transfer per ADR-017, AND `migrate(predecessor_state)` cross-major migration per ADR-020, AND HSIS ≥95% pass per Spirit class measured against a 6×50=300 scenario corpus,
So that Spirit upgrades are routine operations — not high-risk redeploys.

**Acceptance Criteria:**

**Given** a Spirit with manifest `[hot_swap].state_schema_uri` and `state_schema_version`
**When** `crates/maos-lifecycle/src/hot_swap/coordinator.rs::initiate_swap` runs against a same-major successor with additive schema changes
**Then** predecessor state is CBOR-encoded per `crates/maos-lifecycle/src/hot_swap/state_codec.rs` (ADR-017)
**And** the successor's `on_swap_in(state)` ABI hook receives the decoded state
**And** swap completes with P99 <500ms measured by `crates/maos-bench/benches/hot_swap_latency.rs` (NFR-Perf-7)

**Given** a cross-major upgrade
**When** the manifest declares `migrates_from = "0.x"` and the predecessor archive exists at `~/.local/share/maos/spirit-archives/<spirit_id>/<predecessor_version>/`
**Then** `crates/maos-lifecycle/src/migration/migrator.rs::run_migrator(predecessor_state)` invokes the Spirit's declared `migrate(predecessor_state) -> Result<successor_state, Error>` entry point (ADR-020)
**And** the kernel refuses to load with `EMigratorMissing` if predecessor archive exists and no migrator declared

**Given** the saga-style compensating transactions in `crates/maos-lifecycle/src/hot_swap/saga.rs`
**When** `on_swap_out` fails
**Then** the kernel restores the predecessor (no successor activation)
**And** when `on_swap_in` fails, the kernel discards the successor and restores predecessor

**Given** a post-swap invariant violation (state-shape, halt continuity, capability mismatch)
**When** the violation is detected within 30s of swap completion by `crates/maos-lifecycle/src/hot_swap/post_swap_monitor.rs`
**Then** the kernel auto-reverts to the predecessor (NFR-Rel-5)
**And** the kernel emits `HotSwapAborted` IAC frame to interested parties via `crates/maos-kernel-core/src/iac/`

**Given** the I14 halt-continuity enforcement (round-3 Amelia + Murat: explicit integration partner of Story 4.1)
**When** the Hot-Swap Coordinator calls Story 4.1's `crates/maos-kernel-core/src/halt/mod.rs::validate_halt_set(spirit_manifest)` before swap
**Then** the swap is rejected with `EHaltContinuityViolation` if Spirit hasn't declared `halt_protocol_compatibility = N` matching the predecessor's halt schema version
**And** active halts retain identity, replay context, and resumption guarantees across the swap (FR53)
**And** the end-to-end integration test lives in `crates/maos-lifecycle/tests/hot_swap_halt_continuity_test.rs` (owned here, not in Story 4.1)
**And** the test exercises ≥10 scenarios from `crates/maos-eval/fixtures/halt-continuity-corpus-v0/` (committed alongside this story)

**Given** the HSIS corpus (NFR-Rel-3)
**When** 6 Spirit-class corpora × 50 scenarios = 300 scenarios run through `crates/maos-eval/tests/hsis_runner.rs`
**Then** ≥95% pass per Spirit class
**And** zero invariant violations CVSS-7 class are tolerated
**And** **corpus authoring schedule (round-3 split):** 100 scenarios authored in Story 4.5 (Researcher + Observer Spirit classes) and committed at `crates/maos-eval/fixtures/hsis-researcher-observer-v0/`; **the remaining 200 scenarios are authored HERE in Story 5.2** for Butler / Orchestrator / Worker / CliWrapper classes and committed to `crates/maos-eval/fixtures/hsis-butler-orchestrator-worker-cliwrapper-v0/` — total 300
**And** **intra-E5 ordering:** Story 5.2 authoring MUST close before Story 4.1's halt-receipt gate AC closes at v1.0 (so the production-grade HSIS corpus replaces Story 4.1's `synthetic-v0` corpus)

## Story 5.3: Detect Spirit Crashes, Hangs, and Silent Failures with Halt-Receipt 99.9%

As a Spirit Scheduler operator,
I want the kernel to detect Spirit-process crashes within 2s and emit `task.orphaned` within 5s (FR12), detect hung Spirits via no-progress IAC >30s, detect silent failures despite healthy heartbeats, produce halt-receipts at 99.9% on every termination, AND support kernel cold-restart in ≤30s on graceful shutdown / ≤1 in-flight message loss on hard kill,
So that the substrate's reliability floor is mechanical not aspirational.

**Acceptance Criteria:**

**Given** a Spirit subprocess receives SIGKILL
**When** the kernel observes the process exit
**Then** crash detection completes within 2s (≥99/100 on the SIGKILL crash corpus, NFR-Rel-1)
**And** `task.orphaned` IAC frames are emitted to in-flight task originators within 5s
**And** exit cause is journaled

**Given** a Spirit alive but emitting no progress IAC for >30s
**When** the kernel's hung-Spirit detection runs
**Then** `task.stalled` event is emitted within 60s
**And** ≥48/50 reclassified within 60s on the hang corpus (NFR-Rel-2)

**Given** a Spirit emits healthy heartbeats but no progress IAC for >30s
**When** the silent-failure detector runs (NFR-Rel-4)
**Then** `silent_failure_suspect` event is emitted
**And** ≥45/50 detected on the adversarial zombie-heartbeat corpus

**Given** a Spirit terminates (planned `unload` or unplanned crash/halt)
**When** the kernel processes the termination
**Then** a halt-receipt is produced before process exit (cross-ref E4 Story 4.1)
**And** halt-receipt production rate is ≥99.9% across termination corpora (NFR-Rel-11)

**Given** the dead-Spirit task disposition (FR50)
**When** a Spirit dies with in-flight tasks
**Then** the kernel applies the manifest-declared `[on_crash].action`: `NACK` / `reassign-to-replica` / `escalate-to-operator`
**And** the disposition is journaled

**Given** kernel cold-restart
**When** `maosctl restart` is invoked gracefully
**Then** restart completes ≤30s with no data loss (NFR-Rel-10)
**And** on hard kill, ≤1 in-flight message is lost

## Story 5.4: Run Spirit Upgrades and Propagate Signed Revocations in ≤5s

As an operator,
I want `maosctl spirit upgrade <spirit> --to <version> --policy <hot-swap|cold-swap|migrator>` (FR49) AND a signed Revocation List (CRL) artifact polled every 5min + offline-import path (FR13), AND revocation propagation ≤5s p99 under 10⁴ concurrent capability-token validations,
So that I can roll forward and roll back Spirits safely and revoke any compromised Spirit across the whole substrate within seconds.

**Acceptance Criteria:**

**Given** a Spirit version upgrade with `--policy hot-swap` (default)
**When** `maosctl spirit upgrade butler --to 0.3.2 --policy hot-swap` runs
**Then** the upgrade uses Story 5.2's hot-swap path with state preservation
**And** the upgrade journals the version transition

**Given** `--policy cold-swap`
**When** the upgrade runs
**Then** the kernel performs an `unload` + `load` cycle with re-init (no state preservation)
**And** in-flight tasks are NACKed per Story 5.3's disposition

**Given** `--policy migrator`
**When** the upgrade is across a major version
**Then** the kernel invokes the manifest-declared `migrate(predecessor_state)` entry point per Story 5.2
**And** failure paths trigger saga-style compensation

**Given** the signed Revocation List artifact
**When** a Spirit is revoked (publisher or operator origin)
**Then** the CRL is registry-pushed and kernel polls every 5min
**And** offline-import via `maosctl revocations import <signed-crl>` is supported (FR60 path)
**And** running Spirit instances receive `SpiritRevoked` event and execute declared revocation policy (`terminate-immediately` / `drain-then-terminate` / `quarantine`)

**Given** revocation propagation under load
**When** 10⁴ concurrent capability-token validations are in flight and a revocation arrives
**Then** propagation latency is ≤5s p99 (NFR-Rel-9)
**And** subsequent token uses against the revoked Spirit fail with `ECapabilityRevoked`

## Story 5.5a: Sandbox Tier T3 — Container Isolation via Docker / Podman

As an operator hosting `public-untrusted` Spirits at v0.5,
I want sandbox tier T3 (containers wrapping T2 protections, via Docker or Podman) implemented in `crates/maos-sandbox/src/t3/`,
So that public-untrusted Spirits run under defense-in-depth (container boundary + Landlock/Seatbelt/restricted-token inside) without operator-managed container orchestration glue.

**Acceptance Criteria:**

**Acceptance Criteria:**

**Given** a Spirit manifest declaring `sandbox_tier = "T3"` (or forced to T3 by trust-tier floor in Story 1b.3)
**When** the Spirit Scheduler spawns the Spirit via `crates/maos-sandbox/src/t3/spawn.rs::spawn_t3`
**Then** the Spirit runs inside a Docker or Podman container chosen by `crates/maos-sandbox/src/runtime_detect.rs`
**And** T2 protections (Landlock+seccomp on Linux inside the container) are also applied
**And** the strictest-of-(manifest, trust-tier, operator-policy) floor from Story 1b.3 is maintained

**Given** the container image used for T3
**When** the kernel builds or pulls it
**Then** the image SHA is pinned in `crates/maos-sandbox/t3-image.lock`
**And** Ed25519 signature verification runs against the image before spawn
**And** image-mismatch fails with `ESandboxImageMismatch`

**Given** a T3-spawned Spirit
**When** the operator runs `maosctl spirit inspect <id> --sandbox`
**Then** the report shows the active container runtime, image SHA, applied T2 protections, and the strictest-tier reasoning chain
**And** the report is journaled to the Lifecycle Journal at spawn time

**Given** the v0.5 ship gate
**When** the T3-escape corpus runs (a Spirit attempting forbidden file/network/exec ops from inside the container)
**Then** the corpus lives in `crates/maos-sandbox/tests/fixtures/t3-escape-attempts/`
**And** 100% of escape attempts are blocked
**And** every block is recorded via `cap-audit` (Story 1b.2) to the Transparency Log

## Story 5.5b: Run the Multi-Provider CI Matrix Across Anthropic, OpenAI, and Ollama

As an operator who refuses provider lock-in,
I want `crates/maos-providers/` hosting driver implementations for Anthropic + OpenAI + Ollama (local-LLM) with a CI matrix that runs the same Spirit-test fixture suite across all three providers and emits a behavioral-comparison report,
So that v0.5 ecosystem expansion is real, provider behavior drift is detectable, and Spirit authors can choose providers per-Spirit without rewriting their code (FR3).

**Acceptance Criteria:**

**Given** `crates/maos-providers/` with three driver crates (`maos-providers/anthropic/`, `maos-providers/openai/`, `maos-providers/ollama/`)
**When** each driver implements the `ProviderDriver` trait declared in `crates/maos-providers/src/lib.rs`
**Then** every driver routes through the kernel-side `Inference Port` from Story 1b.4
**And** Spirit binaries do not import vendor SDKs directly (FR47 enforcement from Story 0.2's kernel-API surface lint)

**Given** the CI matrix configured in `.github/workflows/multi-provider.yml`
**When** the matrix runs the spirit-test fixture suite from Story 2.4 against each provider
**Then** all three providers execute the same fixture inputs
**And** results are normalized into `tests/reports/multi-provider-<sha>.json` with one row per (fixture, provider) pair
**And** behavioral-difference outliers (any provider deviating ≥10% from the median on any fixture) are flagged in the CI report

**Given** the operator-configurable per-Spirit provider declaration in manifest `[providers]`
**When** an operator switches a Spirit from Anthropic → OpenAI mid-deployment
**Then** the switch requires only a manifest change (no Spirit rebuild)
**And** the switch is journaled to the Lifecycle Journal with both provider identities

**Given** air-gapped deployments
**When** the operator configures Ollama as the only provider and disables outbound network
**Then** the substrate runs end-to-end with zero outbound provider calls
**And** this is structurally validated in CI via the air-gapped network-namespace isolation test (later wired in Story 9.4)

## Story 5.5c: MCP Client + ACP Server — Tool Servers and Editor Hosts

As a Spirit author at v0.5 wanting to consume MCP tool servers from inside my Spirit AND host my Spirit inside an IDE (Zed, VSCode),
I want a fully-featured MCP client in `crates/maos-mcp/` supporting all three transports (stdio / SSE / Streamable HTTP) AND an ACP server in `crates/maos-acp/` exposing the Spirit via NDJSON over stdio,
So that Spirits can call external tools AND be edited/debugged in their authors' IDEs without ad-hoc adapters.

**Acceptance Criteria:**

**Given** `crates/maos-mcp/src/client.rs` implementing the MCP client
**When** a Spirit calls `kernel.mcp.call(server_uri, tool, args)`
**Then** the client supports all three MCP transports (stdio / SSE / Streamable HTTP)
**And** Streamable HTTP is the default for Loom-lite, Spirit registry, and production tool servers
**And** transport selection is operator-configurable per server URI

**Given** the ACP server in `crates/maos-acp/src/server.rs`
**When** an editor connects via NDJSON over stdio
**Then** the editor hosts the Spirit's UX surface (task.assign in, halt-resolution out)
**And** ACP-hosted Spirits emit `task.assign` IAC frames equivalently to terminal-hosted Spirits
**And** halt notifications reach the editor via the notification surface from Story 3.1

**Given** Zed and VSCode integrations at v1.0
**When** the integration test runs against each editor's plugin in `tests/integration/acp-editors/`
**Then** both editors successfully host a Spirit through full lifecycle (load → task.assign → halt → resolve → unload)
**And** the editor displays the Spirit's structured introduction in a native UI surface

**Given** the kernel-API surface invariant (Story 0.2)
**When** MCP and ACP code is committed
**Then** neither crate introduces kernel-API surface functions classified `other`
**And** the boundary remains adapter-only — `maos-domain` does not import MCP or ACP types

## Story 5.5d: Spirit Registry over MCP-Streamable-HTTP with Three Trust Tiers

As an operator at v0.5 installing third-party Spirits,
I want the Spirit registry implemented as an MCP-Streamable-HTTP server in `crates/maos-registry/` supporting all five operations (`registry.search` / `registry.manifest` / `registry.artifact` / `registry.publish` / `registry.deprecate`) with three trust tiers enforced at admission (`local`, `org-internal`, `public-untrusted`; `public-vetted` deferred to v2.5 via FR37),
So that the v0.5 ecosystem expansion has a working publish → discover → install → deprecate loop without operator-managed registry glue.

**Acceptance Criteria:**

**Given** `crates/maos-registry/src/server.rs` exposing the registry as an MCP-Streamable-HTTP server (ADR-008)
**When** the operator points `maosctl registry use <uri>` at the registry endpoint
**Then** all five operations succeed against the configured endpoint:
  - `registry.search(query)` returns matching Spirits
  - `registry.manifest(spirit_id, version)` returns the signed manifest
  - `registry.artifact(spirit_id, version)` returns the signed binary
  - `registry.publish(signed_package)` uploads with Ed25519 verification
  - `registry.deprecate(spirit_id, version)` marks the version yanked

**Given** the three trust tiers at v0.5
**When** an operator attempts to install a Spirit
**Then** admission enforces the strictest-of-(manifest declared tier, registry tier, operator policy) floor
**And** `local` Spirits bypass the registry entirely
**And** `org-internal` Spirits require the registry's org-key signature
**And** `public-untrusted` Spirits require both publisher signature AND admission-time ComplianceClaim verification (full envelope from Story 7.3 lands at v1.0; v0.5 uses the frozen schema from Story 1b.4)
**And** FR37 `public-vetted` tier is explicitly excluded from v1.0 scope per round-2 decision

**Given** the registry yank surface (FR59 full at Story 7.2; v0.5 baseline here)
**When** a yank event arrives via `registry.deprecate`
**Then** the kernel polls every 5min and propagates the yank to running Spirit instances
**And** the yank is distinguishable from operator-local revocation (FR13 via signed CRL from Story 5.4)

**Given** the v0.5 ship gate
**When** the registry roundtrip corpus runs (`crates/maos-registry/tests/fixtures/registry-roundtrip-v05/`)
**Then** 100% of well-formed publish→search→manifest→artifact flows succeed
**And** 100% of malformed flows are rejected with typed errors from the FR63 catalog

## Story 5.5e: §13.1 rust-inproc Measurement Gate — Subprocess vs In-Process Latency Decision

As the architecture lead deciding whether to invest in a second Spirit form,
I want the §13.1 measurement story executed as `crates/maos-bench/benches/section_13_1.rs` measuring J1 (founder-loop CliWrapper IPC) and J4 (Mira-Nash colocation) latency on subprocess form AND a published ADR recording the go/no-go decision before v0.5 ships,
So that the rust-inproc Spirit form is a DATA-DRIVEN unlock, not an architectural aspiration — and downstream stories (NFR-Test-7 cross-form equivalence in Story 10.2) only ship if the measurement requires them.

**Acceptance Criteria:**

**Given** `crates/maos-bench/benches/section_13_1.rs` instrumenting subprocess-form Spirits
**When** the bench runs the J1 workload (founder-loop CliWrapper invocation chain) and J4 workload (two Spirits emitting via the IAC bus at colocation latency)
**Then** P95 IPC latency for J1 and P95 colocation latency for J4 are measured with statistical significance over ≥1000 invocations
**And** the bench report is committed to `tests/reports/section-13-1-<sha>.json`

**Given** the measurement outcome
**When** the architecture lead reviews the J1 + J4 P95 numbers
**Then** **IF** J1 P95 ≤25ms AND J4 P95 ≤10ms (both budgets met by subprocess form): the decision is `defer-rust-inproc-to-v2.0+`, the rust-inproc crate is NOT created in v1.5, NFR-Test-7 cross-form equivalence is REMOVED from v1.5 scope, and CLI-wrapper-only behavioral equivalence runs in Story 10.2
**And** **OTHERWISE**: the decision is `unlock-rust-inproc-in-v0.5`, the `maos-spirit-rust-inproc` crate is scaffolded in v0.5 with NFR-Test-7 cross-form equivalence (rust-inproc ↔ subprocess ≥90%) gating v1.5 in Story 10.2

**Given** the go/no-go decision
**When** the v0.5 release process runs
**Then** a new ADR (numbered after ADR-037) is committed to `docs/adr/` with status `accepted` recording: the measurement methodology, the J1+J4 numbers, the decision, and the rollback criteria
**And** the ADR is linked from STABILITY.md (E10 Story 10.1)
**And** v0.5 release is BLOCKED until the ADR exists with status `accepted`

**Given** the measurement bench
**When** any subsequent v0.x release adds Spirit-class-specific workloads
**Then** the bench can be extended without re-deciding the v0.5 outcome
**And** the decision history is preserved in the ADR ledger

---
