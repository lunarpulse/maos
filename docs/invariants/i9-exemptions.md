---
id: I9-exemptions
title: I9 Exemption Register
---

# I9 Exemption Register

This file enumerates every `#[i9_exempt(reason = "...")]` site in the kernel-core tree.
Each entry must include a one-paragraph rationale signed by ≥2 maintainers.

## Entries

### `PolicyTable` — `crates/maos-kernel-core/src/capability/cap_policy/mod.rs`

**Reason:** operator policy table; structural-state caching per I9 — bounded TTL,
key=spirit_id, no parameter drift. The `Arc<ArcSwap<PolicyTableInner>>` pattern
enables read-mostly copy-on-write updates without blocking hot-path readers.

### `CapQuotaTracker` — `crates/maos-kernel-core/src/capability/cap_quota/mod.rs`

**Reason:** per-Spirit budget counter; structural-state caching per I9 — bounded
by Spirit lifetime, key=spirit_id, no parameter drift. The `DashMap<SpiritId, AtomicU64>`
shards concurrent access with negligible contention at v0.1-β's small Spirit counts.

### `CapTokensShardRing` — `crates/maos-kernel-core/src/capability/cap_tokens/mod.rs`

**Reason:** capability token shard ring lives inside the I9-whitelisted directory
`capability/cap_tokens/` per `xtask/i9-whitelist.toml`. Persistent state (the
64-shard `Arc<[CapShard; 64]>`) is structurally cached per I9 — bounded by token
TTL (≤60s high-privilege per ADR-023), keyed by token_id, no cross-key aggregation.

### `PolicyTableInner` — `crates/maos-kernel-core/src/capability/cap_policy/mod.rs`

**Reason:** inner policy data behind `ArcSwap<PolicyTableInner>`. Holds manifest scopes,
trust tier floors, and operator config. Updated atomically via CoW swap; bounded by
operator policy refresh cadence.

### `OperatorPolicyConfig` — `crates/maos-kernel-core/src/capability/cap_policy/mod.rs`

**Reason:** operator policy config embedded in PolicyTableInner. Per-capability approval
overrides and per-Spirit tier floors. Updated atomically via CoW swap.

### `ManifestCapabilityScope` — `crates/maos-kernel-core/src/capability/cap_policy/mod.rs`

**Reason:** per-Spirit manifest scope declaration embedded in PolicyTableInner. Updated
atomically via CoW swap.

### `CapabilityRegistryAdapter` — `crates/maos-kernel-core/src/capability/mod.rs`

**Reason:** composite adapter holding `Arc` references to the four ADR-030 sub-modules
(tokens, policy, quota) and the audit channel sender. Each sub-module is independently
exempted; the composite holds only shared references, no additional persistent state.

### `ClassSection` / `RawClassSection` — `crates/maos-kernel-core/src/security/manifest.rs`

**Reason:** manifest data structs (Story 1b.5c) — parsed once from a TOML file at
Spirit admission and dropped immediately after `admit_spirit` consumes the validated
shape. The `forms: Vec<String>` field triggers the I9 walker's non-primitive-Vec
heuristic, but no instance survives past the admission stack frame. Coverage gated
by NFR-Test-13's `manifest_field_coverage` walker.

### `ProviderCapabilities` / `RawProviderCapabilities` — `crates/maos-kernel-core/src/security/manifest.rs`

**Reason:** manifest data structs (Story 1b.5c) — declared `provider.complete` capability
list parsed from the `[capabilities.required]` manifest section. Same parsed-then-dropped
lifecycle as `ClassSection`. The `complete: Vec<String>` field is the AC3 NFR-Test-13
gated enumeration of allowed Inference Port providers; no kernel persistence.

### `OutputShape` / `RawOutputShape` — `crates/maos-kernel-core/src/security/manifest.rs`

**Reason:** manifest data struct (Story 1b.5c) — declared FR58 `required_fields` list
parsed from the `[output_shape]` manifest section. Parsed-then-dropped at admission;
the orchestrator verifies the Spirit's response shape against this list and discards
the struct after validation. No kernel persistence.

### `OutputShapePredicate` — `crates/maos-kernel-core/src/security/manifest.rs`

**Reason:** manifest-derived predicate (Story 2.1) — constructed from `OutputShape` at
admission and held in `SandboxSpec`. Dropped after spawn together with the spec;
the `fields: Vec<String>` is a structural copy of the manifest's `required_fields`.
No kernel persistence beyond the admission stack frame.

### `InferencePortAdapter` — `crates/maos-kernel-core/src/inference/mod.rs`

**Reason:** Inference Port runtime adapter (Story 1b.4) — composite holding `Arc<dyn Provider>`,
`Arc<CapabilityRegistryAdapter>`, `Arc<TransparencyLogAdapter>`, and `Arc<IacRtMetrics>`.
All four are shared references to independently-exempt sub-services; the adapter holds
no additional persistent state. Sanctioned per Epic 1b Owns ("Inference Port implementation
(Anthropic provider at v0.1; ADR-005)"). Wired at composition root in `maos-bin/src/main.rs`.

### `SecurityManagerAdapter` — `crates/maos-kernel-core/src/security/mod.rs`

**Reason:** Security Manager runtime adapter (Story 1b.3) — promoted from ZST in v0.1-α
to hold `Arc<PolicyTable>`. The `PolicyTable` is independently exempted (operator policy
table per I9 structural-state caching). The adapter is a thin trait-implementation
wrapper over the policy table; no additional persistent state. Composed at the
composition root with the same `Arc<PolicyTable>` shared by `CapabilityRegistryAdapter`.

### `SandboxSpec` — `crates/maos-kernel-core/src/security/sandbox/mod.rs`

**Reason:** manifest-derived spawn parameter (Story 1b.3) — fully-resolved sandbox
specification produced at admission and consumed by `spawn_sandboxed`. The `Vec<Scope>`
field carries the declared capability scopes for the spawn call. Parsed-then-dropped:
constructed per-Spirit at admission, consumed during the spawn syscall, dropped immediately
after the child process is launched. No kernel persistence.

### `HistogramSeries` / `CounterSeries` / `IacRtMetrics` — `crates/maos-kernel-core/src/telemetry/iac_rt.rs`

**Reason:** IAC round-trip telemetry registry (Story 1b.4) — sanctioned per Epic 1b Owns
("Telemetry IAC round-trip metrics (binding from v0.1)"). The `AtomicU64` buckets/counters
and `Vec<(Service, Outcome, ...)>` fan-out tables are the metric accumulator state required
by the Prometheus-compatible `iac_rt_duration_us` histogram, `iac_rt_inflight` gauge, and
`iac_rt_errors_total` counter (NFR-Obs-4 / NFR-Perf-3 baselines). Bounded by the fixed
service × outcome cardinality declared in `IAC_RT_BUCKETS_US`; no per-Spirit growth.

### `Mailbox` — `crates/maos-kernel-core/src/iac/mailbox.rs`

**Reason:** per-Spirit mailbox router (Story 3.1). The `DashMap<(String, FrameKind), mpsc::Sender<IacFrame>>`
holds transient per-process channel senders keyed by Spirit identity. No persistence across
restarts; channels live as long as the Mailbox. The `broadcast_sender` for telemetry events
is similarly transient. Bounded by active Spirit count × 6 frame kinds.

### `SpiritMailboxHandle` — `crates/maos-kernel-core/src/iac/mailbox.rs`

**Reason:** per-Spirit receiver handle (Story 3.1). The `Vec<(FrameKind, mpsc::Receiver<IacFrame>)>`
holds the six per-kind MPSC receivers a Spirit's drain loop polls. Transient per-Spirit state;
dropped with the Spirit. The `metrics: Arc<IacRtMetrics>` is a shared reference to the
already-exempt metrics registry.

### `IacBusAdapter` — `crates/maos-kernel-core/src/iac/mod.rs`

**Reason:** IAC Bus port adapter (Story 3.1). Holds `Arc<Mailbox>` and `Arc<TransparencyLogAdapter>`
— both are I9-sanctioned locations. The adapter is the trait-implementation bridge between
the domain port and the kernel runtime; no additional persistent state.

### `ApprovalManager` — `crates/maos-kernel-core/src/security/approval.rs`

**Reason:** Approval Manager adapter (Story 3.1). Holds `Arc<TransparencyLogAdapter>` (already I9-exempt)
and an `AtomicU64` decision counter for auto-incrementing decision IDs. The counter is transient
per-process state reset on restart; no persistence needed beyond the Approval Decision Log rows.

### `PostureState` — `crates/maos-kernel-core/src/security/posture.rs`

**Reason:** Per-Spirit runtime posture + halt-policy state held inside
PolicyTableInner. Updated atomically via CoW swap (same shape as
manifest_scopes). Bounded by Spirit lifetime, keyed by spirit_pid, no
parameter drift — structural caching per I9.

### `EpistemicPolicySection` — `crates/maos-kernel-core/src/security/manifest.rs`

**Reason:** manifest data struct (Story 3.2) — parsed once from a TOML file at
Spirit admission and stored inside `PostureState` (already I9-exempt). When
used standalone, parsed-then-dropped at admission (mirrors `OutputShape`).
Coverage gated by NFR-Test-13's `manifest_field_coverage` walker.

### `RawEpistemicPolicySection` — `crates/maos-kernel-core/src/security/manifest.rs`

**Reason:** manifest data struct (Story 3.2) — raw deserialization target for
`EpistemicPolicySection`. Parsed-then-consumed by `validate()`; the validated
form is stored inside `PostureState`. No kernel persistence of the raw form.

### `MockHaltResolver` — `crates/maos-kernel-core/src/halt/resolver.rs`

**Reason:** test double (Story 3.3) — captures `resolve()` calls in a
`Mutex<Vec<(HaltId, Resolution)>>` for unit-test assertion. Production
code never constructs this struct; at v0.3-β the composition root wires
it as bootstrap scaffolding, but Story 4.1 will swap it for the production
`KernelHaltResolver` that holds halt-state in an already-I9-exempt location.
Parallel to the existing `CaptureChannel` exemption at
`crates/maos-kernel-core/tests/approval_prompt_e2e.rs`.

### `OrchestratorBuffer` — `crates/maos-kernel-core/src/orchestrator/buffer.rs`

**Reason:** orchestrator instruction buffer (Story 3.4) — transient per-process
`Mutex<VecDeque<OrchestratorInstruction>>` for the FR20 checkpoint/resume primitive.
Bounded by a fixed capacity floor of 32 instructions per Spirit. Transient per-process
state dropped on restart; parallel to the Mailbox's routing state
(`DashMap<(String, FrameKind), mpsc::Sender<IacFrame>>`). No cross-Host replication,
no structural inference — raw FIFO forwarding.

### `OrchestratorBufferRegistry` — `crates/maos-kernel-core/src/orchestrator/registry.rs`

**Reason:** orchestrator per-Spirit registry (Story 3.4) — `DashMap<String, Arc<OrchestratorBuffer>>`
mapping Spirit names to their bounded instruction buffers. Transient per-process state;
parallel to `Mailbox::mpsc_senders` registration. Bounded by active Orchestrator-class
Spirit count; no persistence across restarts.

### `HaltRegistry` — `crates/maos-kernel-core/src/halt/mod.rs`

**Reason:** halt mechanism (Story 4.1) — per-process pending-resolution state for
SINGLE-HALT-OWNER protocol. `RwLock<HashMap<HaltId, HaltState>>` stores transient
halt lifecycle entries; bounded by in-flight Spirit count × per-Spirit halt-set size.
Parallel to capability-token ledger; no persistence or pattern-learning. Drained on
Spirit termination.

### `OutputMarkerRegistry` — `crates/maos-kernel-core/src/halt/output_markers.rs`

**Reason:** halt mechanism (Story 4.1) — per-process override markers awaiting
`output_shape` consumption (Story 4.2). `DashMap<HaltId, Mutex<VecDeque<OutputMarker>>>`
stores transient kernel-side markers; parallel to OrchestratorBuffer. Drained on
resolution; no persistence across restarts.

### `WorkingMemoryStore` — `crates/maos-kernel-core/src/capability/working_memory/store.rs`

**Reason:** capability registry tagged-scalar slot (Story 4.2) — per-Spirit
working memory state for ADR-022 universal-arithmetic predicate evaluation.
`RwLock<HashMap<(u32, String), WorkingMemorySlot>>` is scoped per-Spirit-per-tag;
bounded by active Spirit count × declared tags count. Parallel to
capability-token ledger; no pattern-learning, no persistence across restarts.

### `TelemetryStreamAdapter` — `crates/maos-kernel-core/src/telemetry/mod.rs`

**Reason:** telemetry stream adapter (Story 4.2) — per-process broadcast channel
state for ADR-035 `scalar.tap` telemetry. `Arc<DashMap<TelemetryTopic, broadcast::Sender<ScalarTapEvent>>>`
holds transient per-process channel registration state; bounded by declared
scalar tag count (O(dozens)). No persistence across restarts; parallel to
`IacRtMetrics` exemption (Epic 1b).

### `PrivateMemoryStore` — `crates/maos-kernel-core/src/memory/private.rs`

**Reason:** memory manager private tier (Story 4.3) — per-Spirit-keyed in-memory
map (`RwLock<HashMap<(u32, MemoryNamespace, String), MemoryValue>>`) + per-Spirit-namespaced
filesystem area for ADR-026 + I5 isolation. Bounded by principal forget-cascade
and per-Spirit memory budget; no cross-Spirit aggregation, no pattern-learning.

### `SharedMemoryStore` — `crates/maos-kernel-core/src/memory/shared.rs`

**Reason:** memory manager shared tier (Story 4.3) — Host-wide SQLite-backed
key-value store with namespace prefix per writer for cross-Spirit coordination.
`writer_spirit_pid` is kernel-set, not Spirit-supplied. Bounded by Spirit lifetime
+ namespace ownership; SQLite table is keyed by `(writer_spirit_pid, namespace, key)`.
No content interpretation per §4.0.7.

### `PrincipalNamespaceIndex` — `crates/maos-kernel-core/src/memory/principal.rs`

**Reason:** principal namespace index (Story 4.3) — kernel-side address-only index
of `principal:<principal_id>:<schema>` writes for ADR-026 subject-access query +
GDPR Art. 17 forget cascade. Bounded by principal forget cascade; SQLite table
carries NO content (only addressing tuples per §4.0.7). No content interpretation.

### `SelfTelemetryAggregator` — `crates/maos-kernel-core/src/memory/self_telemetry.rs`

**Reason:** self-telemetry aggregator (Story 4.3) — read-only composer over existing
kernel state (IacRtMetrics, HaltRegistry, TransparencyLogAdapter). Does NOT retain
its own state across calls; the `Arc` fields are references to existing kernel-level
state. FR56 surface for Spirit-side calibration without per-read operator admission.

### `MemoryManagerAdapter` — `crates/maos-kernel-core/src/memory/mod.rs`

**Reason:** memory manager adapter (Story 4.3) — composite dispatcher holding `Arc`
references to the three tier stores (PrivateMemoryStore, SharedMemoryStore,
PrincipalNamespaceIndex) and TransparencyLogAdapter. Does NOT retain mutable state
across calls; delegates to the already-exempt sub-modules. The `next_frame_counter`
is a monotonic ULID counter for audit-frame IDs, not learned state. Bounded by
per-Spirit budget + principal forget-cascade per §4.0.7.

### `IsolationCorpusReport` — `crates/maos-kernel-core/src/isolation/runner.rs`

**Reason:** isolation corpus runner report (Story 4.5) — transient value type produced
by the test-only `IsolationCorpusRunner::run_all`. The `BTreeMap<String, usize>`
fields (`per_category`, `per_split`) hold per-run scenario counters, not persistent
kernel state. The runner itself is a stateless composer over `Arc` references to
existing exempt holders (TransparencyLogAdapter, MemoryManagerAdapter,
LogRecallAdapter, HaltRegistry). Same exemption shape as `SelfTelemetryAggregator`
(Story 4.3) and `LogRecallAdapter` (Story 4.4). No parameter drift; no learned
state; counters are discarded after each `run_all` call.

### `UpgradeOrchestrator` — `crates/maos-kernel-core/src/lifecycle/upgrade.rs`

**Reason:** upgrade orchestrator composite (Story 5.4) — holds `Arc` references to
existing exempt kernel adapters (SpiritSchedulerAdapter, HotSwapCoordinator,
TransparencyLogAdapter, JournalAdapter, IacRtMetrics). Does NOT retain mutable
state across calls; delegates to already-exempt sub-modules. The `UpgradeReport`
and `UpgradeError` types are transient value types. No parameter drift; no learned
state.

### `RevocationApplier` — `crates/maos-kernel-core/src/revocation/applier.rs`

**Reason:** revocation applier composite (Story 5.4) — holds `Arc` references to
existing exempt kernel adapters (CapabilityRegistryAdapter, IacBusAdapter,
HaltRegistry, TransparencyLogAdapter, JournalAdapter, IacRtMetrics). The
`applied_crls: BTreeSet<CrlId>` is an idempotency cache for already-processed
CRLs; it is bounded by operator-supplied input (not learned state). The
`active_drains: BTreeMap<u32, JoinHandle<()>>` tracks in-flight deadline tasks
for `DrainThenTerminate` policy; bounded by the number of loaded Spirits. No
parameter drift.

### `RevocationPoller` — `crates/maos-kernel-core/src/revocation/poller.rs`

**Reason:** revocation poller composite (Story 5.4) — holds `Arc` references to
RevocationApplier, RegistryClient, CryptoProvider, and IacRtMetrics. The poller
is a stateless periodic fetch loop; all mutable state lives in the already-exempt
RevocationApplier. No parameter drift; no learned state.

### `ProvidersSection` — `crates/maos-kernel-core/src/security/manifest.rs`

**Reason:** manifest data (Story 5.5b multi-provider); parsed-then-dropped at
admission, no kernel persistence. Holds the primary + fallback `ProviderConfig`
vector declared in the Spirit manifest's `[providers]` section. Documented post-hoc
as a Story 6.2 sweep — entry pre-dates this story.

### `ProviderConfig` — `crates/maos-kernel-core/src/security/manifest.rs`

**Reason:** manifest data (Story 5.5b multi-provider); parsed-then-dropped at
admission, no kernel persistence. Per-provider id + endpoint_url + model_id +
provider_endpoint_pin. Documented post-hoc as a Story 6.2 sweep.

### `McpSection` — `crates/maos-kernel-core/src/security/manifest.rs`

**Reason:** manifest data (Story 5.5c MCP tool-server declarations); parsed-then-
dropped at admission, no kernel persistence. Holds `Vec<McpServerEntry>`
declared in the Spirit manifest's `[mcp]` section. Documented post-hoc as a
Story 6.2 sweep.

### `CliWrapperConfig` — `crates/maos-kernel-core/src/security/manifest.rs`

**Reason:** Story 6.2 AC5 manifest data per ADR-021 + architecture §6.7;
parsed-then-dropped at admission, no kernel persistence. Holds the `[cli_wrapper]`
section: `command`, `argv_prefix`, `output_shape_version`, `skill_bundle`,
`recovery_policy`, `posture` for the CliWrapperSpirit class. PRESENT means the
Spirit is a CliWrapperSpirit; ABSENT means native Rust Spirit. The two modes
are mutually exclusive at admission (`EManifestSchemaConflict`).

### `SilentFailureDetector` — `crates/maos-kernel-core/src/supervision/silent_failure_detector.rs`

**Reason:** supervision surface (Story 5.3) — holds only Arc references to
existing kernel state (SCB map, TransparencyLog, telemetry); no independently-
mutable persistent state. Documented post-hoc as a Story 6.2 sweep.

### `ProgressWatchdog` — `crates/maos-kernel-core/src/supervision/progress_watchdog.rs`

**Reason:** supervision surface (Story 5.3) — holds only Arc references to
existing kernel state (SCB map, TransparencyLog, telemetry); no independently-
mutable persistent state. Documented post-hoc as a Story 6.2 sweep.

### `SimulatedChildSupervisor` — `crates/maos-kernel-core/src/supervision/test_double.rs`

**Reason:** test double (Story 5.3) — transient per-test state; production
wiring uses the real SubprocessSupervisor impl. Documented post-hoc as a
Story 6.2 sweep.

### `CrashDetector` — `crates/maos-kernel-core/src/supervision/crash_detector.rs`

**Reason:** supervision surface (Story 5.3) — holds only Arc references to
existing kernel state (SCB map, TransparencyLog, HaltRegistry, CapabilityRegistry,
IAC Bus, telemetry); no independently-mutable persistent state. Documented
post-hoc as a Story 6.2 sweep.

### `MultiProviderRouter` — `crates/maos-kernel-core/src/inference/router.rs`

**Reason:** inference port adapter aggregate (Story 5.5b multi-provider); holds
`BTreeMap<String, Arc<dyn Provider>>` of driver instances — not independently-
mutable state. Updated at composition-root time only. Documented post-hoc as a
Story 6.2 sweep.

### `McpClientAdapter` — `crates/maos-kernel-core/src/mcp/mod.rs`

**Reason:** MCP-client adapter aggregate (Story 5.5c); holds Arc references to
wire-level client + audit infrastructure. No independently-mutable state.
Documented post-hoc as a Story 6.2 sweep.

### `McpCapabilities` — `crates/maos-kernel-core/src/security/manifest.rs`

**Reason:** manifest data (Story 5.5c); parsed-then-dropped at admission, no
kernel persistence. Holds `Vec<McpCapabilityServerEntry>` declared in the
Spirit manifest's `[capabilities.required.mcp]` section. Documented post-hoc
as a Story 6.2 sweep.

### `IacBusAdapter` (extended) — `crates/maos-kernel-core/src/iac/mod.rs`

**Reason:** Story 6.2 AC4 extends the existing exemption to also cover the
`frame_lineage_cache: Arc<DashMap<[u8;16], IntentLineage>>` populated at
deliver_typed time and consumed by `retract()` for lineage continuity. The
cache is bounded by `MAX_LINEAGE_CACHE_ENTRIES = 4096` (sized for ~5min of
10-tasks/sec sustained throughput); entries are never explicitly removed but
new inserts skip the cache once the cap is reached (long-tail eviction is
observable in retract continuity). NFR-Aud-14 corpus PASSES on the cache window.

### `ScheduleWatchdog` — `crates/maos-kernel-core/src/scheduler/schedule_watchdog.rs`

**Reason:** per-Spirit schedule firing watchdog (Story 6.4 / FR26 / ADR-025).
Holds two `DashMap` entries:

* `rate_limits: DashMap<(String, String), Arc<ScheduleBucket>>` — per-schedule
  token buckets for `rate_limit_per_hour` enforcement. Transient per-process;
  bucket lifetime matches the Spirit's manifest declaration. No persistence.
* `last_fire_ns: DashMap<(u32, String), u64>` — last-fire timestamp per
  `(spirit_pid, schedule_id)`. Used to honor cadence intervals between fires.
  Transient per-process; entries are scoped to active Spirit PIDs.

Both maps are bounded by `active_spirit_count × declared_schedule_count` (≤ a
few hundred entries in practice). The watchdog itself is dropped on graceful
shutdown via the same `CancellationToken` pattern as `IdleWatchdog`.

### `ScheduleBucket` — `crates/maos-kernel-core/src/scheduler/schedule_watchdog.rs`

**Reason:** per-schedule rate-limit token bucket (Story 6.4 / FR26 / ADR-025).
The `AtomicU64 last_refill_ns` carries the monotonic timestamp of the last
refill; the `parking_lot::Mutex<f64> tokens` carries the current token balance.
Both fields are transient per-process state OWNED by the
`ScheduleWatchdog::rate_limits` DashMap (already I9-exempt). Bucket lifetime
matches its parent watchdog; entries drop on Spirit unload. No persistence.

<!-- ============================================================= -->
<!-- Story 7.1.7 baseline-reset — I9 exemptions for supervised      -->
<!-- kernel state-holders (the 13 production P3 structs) plus the   -->
<!-- previously-undocumented GatewayDispatcher. These are genuine   -->
<!-- supervision-tree state owners (the scheduler is the supervisor -->
<!-- per §4.0.8); their persistent-typed fields are structural per  -->
<!-- I9 (bounded, keyed, no parameter drift, recreated on restart). -->
<!-- The 19 former test-double "violations" (mocks in #[cfg(test)]  -->
<!-- mods + tests/ + benches/) are NOT exempted here — they were    -->
<!-- gate over-reach and are fixed at root by scoping the I9 walker  -->
<!-- to src/ and skipping #[cfg(test)] modules.                     -->
<!-- ============================================================= -->

### `SpiritControlBlock` — `crates/maos-kernel-core/src/scheduler/control_block.rs`

**Reason:** per-Spirit kernel-side control block (the PCB analog). Supervised
single-owner runtime state held under the Spirit Scheduler supervisor per
§4.0.8. The atomics (`state`, `deficit_counter`, the `last_*_ns` timestamps),
the `Arc<dyn AnySpiritObj>` spirit handle, and the `Mutex<Vec<TaskAssignmentRecord>>`
in-flight set are structural runtime state per I9 — bounded by Spirit lifetime,
keyed by `spirit_id`, no parameter drift, recreated on kernel restart.

### `SpiritSchedulerAdapter` — `crates/maos-kernel-core/src/scheduler/scheduler_loop.rs`

**Reason:** the kernel-side Spirit Scheduler — the supervision-tree supervisor
itself (P3 supervisor-exception per §4.0.8). Owns the supervised SCB index
(`Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>`) plus `Arc` handles to the
already-exempt transparency-log adapter, hook dispatcher, capability/memory/iac
adapters and halt registry. Structural runtime state per I9, keyed by `spirit_pid`.

### `HotSwapCoordinator` — `crates/maos-kernel-core/src/hot_swap/coordinator.rs`

**Reason:** composition-root singleton coordinating hot-swap. Holds `Arc` handles
to the supervised SCB set and to already-exempt kernel adapters (journal,
transparency log, halt registry, capability, iac, hook dispatcher, iac-rt
metrics) plus the `active_monitors` / `pending_reverts` maps. Supervised
transient state per I9, recreated on kernel restart, no cross-key aggregation.

### `PostSwapMonitor` — `crates/maos-kernel-core/src/hot_swap/post_swap_monitor.rs`

**Reason:** post-swap invariant monitor task. Holds `Arc` to the already-exempt
`HotSwapCoordinator` and to the `PostSwapInvariantSnapshot`. Supervised transient
task state per I9; the monitor task is dropped on graceful shutdown.

### `PostSwapInvariantSnapshot` — `crates/maos-kernel-core/src/hot_swap/post_swap_monitor.rs`

**Reason:** immutable invariant snapshot captured at swap commit. The
`Vec<String>` of pre-swap halt-ids and frame-shapes is bounded structural state
per I9 (sized by the live Spirit's halt/frame set), no parameter drift.

### `KernelLifecycleResolver` — `crates/maos-kernel-core/src/scheduler/verb_resolver.rs`

**Reason:** kernel lifecycle verb resolver routing operator verbs through the
Spirit Scheduler and journaling FR42 director-action rows. Holds `Arc` handles to
the already-exempt scheduler and transparency-log adapters. Supervised composite
per I9.

### `IdleWatchdog` — `crates/maos-kernel-core/src/scheduler/idle_watchdog.rs`

**Reason:** per-Spirit idle watchdog firing `on_idle`. Holds `Arc` handles to the
supervised SCB set and the already-exempt hook dispatcher. Supervised transient
state per I9, keyed by `spirit_pid`; dropped on graceful shutdown via the
`CancellationToken` pattern shared with `ScheduleWatchdog`.

### `HookDispatcher` — `crates/maos-kernel-core/src/scheduler/hook_dispatch.rs`

**Reason:** lifecycle-hook dispatcher enforcing manifest gates and budget
envelopes. Holds `Arc` handles to the already-exempt transparency-log adapter and
iac-rt metrics. Supervised composite per I9.

### `WorkingMemoryOrchestrator` — `crates/maos-kernel-core/src/capability/working_memory/orchestrator.rs`

**Reason:** working-memory orchestrator holding `Arc` handles to the already-exempt
capability registry and halt registry. Supervised composite per I9, no parameter
drift.

### `GatewayDispatcher` — `crates/maos-kernel-core/src/orchestrator/gateway_dispatcher.rs`

**Reason:** dispatcher managing gateway submodules for all Spirits. Holds per-process
transient state (`Arc<DashMap<(u32, String), GatewayInstance>>` of running gateway
tasks plus the factory registry); recreated on kernel restart — structural-state
caching per I9. (Carried an `#[i9_exempt]` since Story 6.5 but was undocumented
here; documented at Story 7.1.7 baseline-reset.)

### `GatewayInstance` — `crates/maos-kernel-core/src/orchestrator/gateway_dispatcher.rs`

**Reason:** per-gateway running state owned by `GatewayDispatcher.gateways`. The
`Arc<dyn GatewaySubmodule>` submodule handle and `Arc<AtomicBool>` cancel flag are
per-process transient task state per I9, recreated on kernel restart.

### `GatewayCancelHandle` — `crates/maos-kernel-core/src/orchestrator/gateway_dispatcher.rs`

**Reason:** gateway cancel handle. The `Arc<AtomicBool>` cancel flag is per-process
transient cooperative-cancellation state per I9, recreated on kernel restart.

### `T3ImageLock` — `crates/maos-kernel-core/src/security/sandbox/t3/image_lock.rs`

**Reason:** loaded T3 image lock config. The `Vec<T3ImageAttestation>` attestation
list is bounded structural config state per I9, keyed by image digest, no
parameter drift.

### `ScbTracker` — `crates/maos-kernel-core/src/iac.rs`

**Reason:** std-wrapper exposing the iac Spirit-control-block index to the
maos-iac trait. The `Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>` is the
supervisor-owned SCB map per I9, keyed by `spirit_pid` — the same supervised state
owned by the scheduler, surfaced here only because neither std's `RwLock<BTreeMap>`
nor the maos-iac trait is local to maos-kernel-core without a wrapper struct.

### `MockLifecycleResolver` — `crates/maos-kernel-core/src/scheduler/verb_resolver.rs`

**Reason:** public test double in `mod test_double` (intentionally NOT
`#[cfg(test)]`-gated so director-surface tests in other crates can consume it).
The `Mutex<Vec<(String, LifecycleVerb)>>` captures every `resolve_verb` call for
test assertions — test-only capture state, never production runtime state, per I9.

### `BridgeSpawnSpec` — `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs`

**Reason:** Story 8.12 live CLI subprocess bridge. A transient value object
describing one subprocess spawn: program path, argv prefix + hash, host-injected
process environment (`Vec<(String, String)>` credentials, never journaled), stdio
shape, and control-channel config. Consumed once at spawn and dropped — it caches
no cross-invocation kernel domain state per I9.

### `SpawnedBridge` — `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs`

**Reason:** Story 8.12 live CLI subprocess bridge RAII handle. Owns the OS
resources for one in-flight CLI invocation: the `Child` process, its stdin
control channel, the reader-thread `Vec<JoinHandle<()>>`, the bounded `Receiver`,
and an `Arc<AtomicU64>` drop-counter. `Drop` kills+reaps the child and joins the
readers so no orphaned process or thread survives. This is process-bound resource
ownership scoped to a single invocation, not cached kernel domain state per I9.
