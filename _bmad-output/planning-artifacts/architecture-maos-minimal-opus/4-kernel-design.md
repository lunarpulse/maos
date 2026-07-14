# 4. Kernel Design

## 4.0 Kernel Internal Architecture

Before the kernel decomposition individually, this section commits how the kernel is organized: the architectural style, the layout of code, how subsystems connect, and the principles that prevent kernel state-creep over time.

**Component classification (terminology lock).** The kernel comprises **one supervisor (Spirit Scheduler), four supervised services (Security Manager, Memory Manager, IAC Bus, Capability Registry), and two internal modules (I/O Subsystem, Telemetry Stream).** The supervisor / supervised-service / module taxonomy is the operational classification per §4.0.8's four-property test (P1–P4); a supervised service satisfies all four properties, a module fails at least one, the supervisor satisfies P1/P2/P4 and is exempt from P3 (boundary manifest) because its boundary *is* the union of its children's boundaries.

**v0.1 component-count rationale.** Two subsystems that earlier drafts modeled as separate services — I/O Subsystem (§4.4) and Telemetry Stream (§4.7) — collapse to **internal kernel modules** at v0.1 because their v0.1 surface (single Anthropic provider for I/O; pure-broadcast no-state for Telemetry) does not yet justify the separate-task-pool overhead a supervised service implies. **Security Manager remains a supervised service** even at v0.1 because its compilation boundary carries security invariants the type system enforces (capability-token signing-key isolation, audit log integrity, mTLS rotation injection point) — collapsing it would weaken those invariants to internal-API discipline. Extraction of I/O Subsystem and Telemetry Stream to full supervised services is a v0.5+ option, gated on real multi-provider and stream-processing demand.

References elsewhere in this document to "five services" should be read as "one supervisor + four supervised services" — the older shorthand predates the §4.0.8 formalization.

### 4.0.1 Architectural style — hexagonal for static structure, actor model on the hot path

**Static structure: hexagonal (ports-and-adapters).** The kernel is structured as a domain core (pure types, invariants, pure functions) surrounded by ports (trait definitions for kernel-external dependencies) implemented by an adapter ring (concrete adapters for HTTP, stdio, mTLS, MCP, ACP, providers, persistence, secrets). This gives the kernel multi-adapter-per-port flexibility (swap SQLite for Postgres without touching domain logic), testability (every port has a mock adapter), and keeps the domain core small.

**Runtime hot path: actor model.** Each Spirit is an actor — mailbox-addressable, behavior-encapsulated, no shared mutable state with peers. This pattern gives four properties for free: backpressure via bounded mailboxes, no locks on the hot path (each actor owns its state), failure isolation via Tokio task supervision, and natural hot-swap (replace `behavior` while preserving `state` and `open_tokens`). The seven kernel services are *not* themselves actors — they are shared services that actors call into, with their own task pools.

The two styles do not conflict. Hexagonal owns the static dependency graph; actor model owns the runtime topology. A Spirit's `IacBus::send` call routes through the IAC Bus service (a Tokio task pool), which writes to the Transparency Log (a Persistence adapter), which fsyncs to SQLite (a domain-side WAL append). Every layer has a clean contract; nothing leaks across.

### 4.0.2 Layout

```
maos/
├── crates/                             # 19 library + binary crates (+ xtask + examples/example-spirit = 21 workspace members)
│   ├── maos-domain/                    # v0.1 ✅  Pure types, invariants I1-I14, pure functions
│   │                                   #          std crate; depends on maos-spirit-abi for D9 SandboxTier conversion (1b.6)
│   ├── maos-spirit-abi/                # v0.1 ✅  Wire-stable types ONLY. #![no_std].
│   │   └── src/compliance.rs           #          ComplianceClaim schema (per §8.5, App-E).
│   │                                   #          Bumping = bumping ABI_VERSION (frozen at 1 since 1b.4).
│   ├── maos-kernel-core/               # v0.1 ✅  Five services + two internal modules.
│   │   ├── scheduler/                  #          Spirit Scheduler + journal + budget
│   │   ├── memory/                     #          Memory Manager + namespace enforcement
│   │   ├── security/                   #          Security Manager + sandbox + approval (compilation boundary)
│   │   ├── io/                         #          I/O module (HTTP, stdio, mTLS, ACP) — internal at v0.1
│   │   ├── iac/                        #          IAC Bus (mailbox, broadcast, retract) + Transparency Log (write side)
│   │   ├── capability/                 #          Capability Registry (decomposed per ADR-030):
│   │   │   ├── cap-tokens/             #            Hot path: token issue/verify, lock-free (I9 whitelist)
│   │   │   ├── cap-policy/             #            Consent rules + intent allowlist
│   │   │   ├── cap-audit/              #            Audit/lineage writer (slow path, bounded mpsc)
│   │   │   └── cap-quota/              #            Budget tracking + ContextPressure
│   │   ├── compliance/                 #          ComplianceClaim structural validator (~200 LOC, v0.1)
│   │   ├── pipeline/                   #          Emit pipeline (IACFrame + ComplianceClaim co-located)
│   │   ├── telemetry/                  #          Telemetry module + scalar.tap + IAC RT metrics (1b.4)
│   │   ├── inference/                  #          Inference Port adapter (1b.4)
│   │   ├── journal/                    #          Lifecycle Journal — raw NDJSON, fsync per entry (I9 whitelist, 1b.1)
│   │   └── hot_swap/                   #          Hot-Swap Coordinator
│   ├── maos-audit/                     # v0.1 ✅  Read-side SQLite query adapter (added 1b.1).
│   │                                   #          Holds the maosctl audit query path with READ_ONLY SQLite
│   │                                   #          connection; preserves the 1a.4 rule that maos-cli must NOT
│   │                                   #          depend on maos-kernel-core. Story 9.1 extends with
│   │                                   #          subject-access / posture-delta / sealed-export.
│   ├── maos-attrs/                     # v0.1 ✅  Proc-macro crate (added 1b.3, Story 1b.6 retro).
│   │                                   #          Hosts #[i9_exempt(reason = "...")] — must live outside
│   │                                   #          maos-kernel-core because Rust proc-macro crates cannot
│   │                                   #          live inside the crate they annotate. Future expansion:
│   │                                   #          additional kernel-discipline attributes.
│   ├── maos-spirit-derive/              # v0.1 ✅  Proc-macro crate (added 2.1). Hosts #[spirit] attribute.
│   │                                   #          Must live outside maos-spirit-sdk because Rust proc-macro
│   │                                   #          crates cannot live inside the crate they annotate
│   │                                   #          (serde/serde_derive precedent). Re-exported as
│   │                                   #          maos_spirit_sdk::spirit for Spirit-author ergonomics.
│   │                                   #          Dep direction: maos-spirit-sdk → maos-spirit-derive.
│   │                                   #          Exception to inward-flow rationale is explicit —
│   │                                   #          proc-macro constraint, parallel to maos-attrs.
│   ├── maos-corpus-gen/                # v0.1 ✅  Deterministic corpus generators (Epic 0 — secret-redaction
│   │                                   #          10⁴ + red-team ≥640). CorpusGenerator trait + SHA-pinned
│   │                                   #          seed format. Future: pub mod ccac; (Story 7.3, v1.0).
│   ├── maos-spirit-sdk/                # v0.1 ✅  Spirit-author helpers; #[spirit] proc-macro (Story 2.1)
│   ├── maos-spirit-hello/              # v0.1 ✅  Reference Spirit; validates SDK end-to-end
│   ├── maos-providers/                 # v0.1 ✅  Anthropic at v0.1; ≥3 providers in CI by v0.5
│   ├── maos-mcp/                       # v0.5    MCP client
│   ├── maos-acp/                       # v0.5    ACP server
│   ├── maos-a2a/                       # v0.9    Bilateral A2A peer module (loopback at v0.9, cross-Host at v1.0)
│   ├── maos-persistence/               # v0.1    SQLite at v0.1; Postgres+pgvector (Loom-lite) at v1.5
│   ├── maos-secrets/                   # v0.1    OS keyring adapter
│   ├── maos-compliance/                # v0.9 🔒  Semantic evaluator + N=600 corpus (App-E) — Story 7.3: placeholder → v0.9-binding evaluator (evaluate_envelope + RuntimeExecutionContext + canonical_cbor; admission consumes it; CCAC N=600 ship gate). Workspace count UNCHANGED at 29.
│   ├── maos-skill/                      # v0.5    Story 7.4 — kernel-mediated skill ecosystem: maos.skill.v1 schema (markdown + TOML frontmatter, deny_unknown_fields), filesystem discovery, FR39 operator-admission queue (3 entry paths, no auto-admit), FR57 revision proposals. Kernel validates schema + manages admission/audit ONLY; body is opaque (§4.0.7). Workspace count 29→30.
│   ├── maos-control/                   # v0.5    Control-plane HTTP API
│   ├── maos-cli/                       # v0.1    maosctl
│   ├── maos-director-surface/          # v0.3-β  Kernel-adjacent notification dispatcher (Story 3.1); terminal/ACP/mobile-push channels
│   └── maos-bin/                       # v0.1 ✅  Composition root
├── xtask/                              # v0.1 ✅  Workspace member — discipline gates
│                                       #          (check-empty-kernel, check-service-boundary, abi-diff,
│                                       #           invariant-lock, manifest-field-coverage, etc.)
├── spirits/                            # Reference Spirit crates (in-process)
├── templates/                          # Spirit-author scaffolding (Story 2.3 → 7.1)
│   ├── spirit-rust/                    # Thin cargo-generate template (Rust-only at v0.3;
│   │                                   # per-language TS/Python/Go at Story 7.1 v0.5+).
│   │                                   # Excluded from workspace via [workspace] exclude.
│   └── spirit-ts/                      # Story 7.1 v0.5 — TypeScript cargo-generate template.
│                                       # Parallel structure to spirit-rust. Excluded from workspace.
├── examples/                           # Workspace-member example Spirits (NOT kernel substrate)
│   ├── example-spirit/                 # Baked output of templates/spirit-rust (Story 2.3).
│   │                                   # Drift-detected via `xtask example-spirit-regen --check`.
│   └── example-spirit-ts/              # Story 7.1 v0.5 — baked TypeScript template output.
│                                       # Node project (npm ci && npm test), NOT a Cargo workspace member.
├── sdks/                               # Story 7.1 v0.5 — language-specific SDK packages
│   └── spirit-ts/                      # @maos/spirit-ts — TypeScript SDK shim (test harness only,
│                                       # not a kernel runtime per ADR-002). Built via tsc. NOT a Cargo member.
├── schemas/                            # JSON Schema + CBOR schemas
│   ├── trace-shape.schema.json
│   ├── halt-registry/<spirit-class>.toml
│   └── gateway-submodule.schema.json
├── docs/
└── fuzz/                               # Fuzz harnesses (manifest, wire, replay)
```

Dependencies point inward (adapter ring → kernel services → domain core), with **two explicit exceptions**: (1) kernel services calling into Spirit ABI traits — the inversion of control that makes Spirits hot-swappable; (2) `maos-domain` depends on `maos-spirit-abi` to host the D9-reconciled `From<ABI SandboxTier> for operational SandboxTier` impl + `to_abi()` method (the no_std boundary + frozen ABI_VERSION=1 wire format make a single canonical type infeasible — see Story 1b.6 dev record). The composition root in `maos-bin/main.rs` is the only place that knows about all crates.

**Workspace member count (post Story 1b.6):** 18 library/binary crates + xtask = **19 workspace members**. Added since the original §4.0.2 description: `maos-audit` (Story 1b.1 — read-side audit query adapter), `maos-attrs` (Story 1b.3 — `#[i9_exempt]` proc-macro), `maos-corpus-gen` (Epic 0 — deterministic corpus generators). `default-members = []` in the workspace root forces every cargo invocation to be `-p`-explicit (Story 1b.6 retro action A7).

**Workspace member count (post Story 7.1):** Story 7.1 adds `templates/spirit-ts/` (excluded from `[workspace] members` per Story 2.3 precedent), `examples/example-spirit-ts/` (Node project, NOT a Cargo workspace member), and `sdks/spirit-ts/` (Node package, NOT a Cargo workspace member). The Cargo workspace member count stays at **27** (post-Epic-6.5 baseline). Story 7.1 introduces non-Cargo workspace members built via `tsc`; the `check-workspace-count` gate stays at 27.

**Workspace member count (post Story 9.5b):** Story 9.5b adds ONE lib crate `crates/maos-telemetry/` (OpenTelemetry SLO-class adapter — TraceSink seam + bounded OTel exporter), moving the count from the post-Story-8.14a baseline 44 to **45 workspace members** (44 + 1 = 45). The `check-workspace-count` gate floor moves to 45. (History: 27 pre-7.1 → 28 with `examples/example-spirit` → 29 with `maos-spirit-cli` (Story 7.2) → 30 with `maos-skill` (Story 7.4) → 31 with `spirits/butler` (Story 8.1) → 32 with `spirits/researcher` (Story 8.2) → 33 with `spirits/observer` (Story 8.3) → 37 with `spirits/{orchestrator,worker,architect,reviewer}` (Story 8.4) → 39 with `spirits/{mira,nash}` (Story 8.5) → 41 with `crates/{maos-a2a-core,maos-a2a-tcp}` (Story 8.6) → 42 with `crates/maos-journey-test` (Story 8.11) → 43 with `crates/maos-notify-push` (Story 8.13) → 44 with `crates/maos-shell` (Story 8.14a) → 45 with `crates/maos-telemetry` (Story 9.5b).)

**Workspace member count (post Story 12.4b Task 1):**<!-- workspace-count-authoritative --> Story 12.4b adds ONE lib crate `spirits/digest/`, moving the reconciled Story 12.1 count from 54 to **55 workspace members**. The new reference Spirit remains outside the kernel/domain dependency closure. The `check-workspace-count` hard-equality gate floor moves to 55. History: 46 (Story 10.4a, maos-loom-lite) → 48 (Story 11.1a, maos-host + maos-wasm-host) → 49 (Story 11.4a, maos-pdp) → 50 (Story 11.4b, maos-escape-detector) → 51/52 (Story 11.4c, maos-siem + maos-sso) → 53 (Story 11.5, maos-fkcs) → 54 (Story 12.1, maos-cohort) → 55 (Story 12.4b, digest).

**A2A transport layering (post Story 8.6).** The A2A surface is now three crates with the dependency arrows drawn explicitly:

```text
  maos-a2a-tcp ──►  maos-a2a-core  ◄── maos-a2a
  (TcpA2ATransport:   (A2ATransport seam,    (LoopbackA2ARouter:
   live cross-Host     A2ARouterCore engine,  in-process A2AProfile::Loopback
   TCP/mTLS wire,      verify_pinned/TOFU,    over the same engine)
   FR23b v1.5)         JSON-RPC framing,
                       Lamport clock, consent,
                       config, identity, chaos)
```

- **`maos-a2a-core`** owns the transport-agnostic protocol substrate (the `A2ATransport` trait + the shared `A2ARouterCore` validation engine reused byte-for-byte by every transport; the TOFU pin store + `verify_pinned`; ADR-012 consent; the JSON-RPC framing + `try_from_bytes`; the `LamportClock`; the mTLS retry policy; the rotation/churn chaos harnesses; the operator config + peer identity + `A2AError`). It carries `rustls` only for verifier-driven config types and contains NO socket/codec/async-TLS wire mechanisms (epic AC-A2 grep-asserted).
- **`maos-a2a-tcp`** is the live cross-Host `A2ATransport` impl: a real TCP listener/dialer with operator-managed mTLS (the `TofuPinningVerifier` bridging WebPKI-then-pin into `verify_pinned`), length-delimited JSON-RPC framing (1 MiB cap), handshake retry, and bounded intake/partition timeouts. It depends ONLY on `maos-a2a-core` (NOT `maos-a2a`, NOT `maos-kernel-core`) — the kernel performs ZERO A2A auto-retry; the only retrier is `HandshakeRetryPolicy` on the transport (AC-T12).
- **`maos-a2a`** retains ONLY the in-process `LoopbackA2ARouter` (`A2AProfile::Loopback`) and `pub use`-re-exports the moved substrate so downstream import paths are unchanged. The extraction also resolved `maos-a2a`'s prior KLOC-ceiling overage (2550 → 202 by the tokei metric) with no ceiling bump.

**`spirit_test` feature on `maos-spirit-sdk` (post Story 2.4):** The crate gains an opt-in `spirit_test` cargo feature (depends on `local_runner` + `std` + `mock`) gating a new `crates/maos-spirit-sdk/src/spirit_test/` module that ships the SDK seed (assertion macros + IAC frame I/O capture + halt resolution simulator + manifest self-check + class-specific regression corpus skeleton + cross-Spirit isolation framework hooks). Workspace member count stays at **21** — the new module is feature-gated inside the existing crate, not a new workspace member.

### 4.0.3 Service dependency map

| Service | Depends on (kernel side) | Used by (kernel side) |
|---|---|---|
| **Spirit Scheduler** | Capability Registry (token revocation on unload), Memory Manager (archive on swap), Persistence (journal) | Control plane (load/swap/unload commands) |
| **Memory Manager** | Persistence, Capability Registry (scope validation) | Spirit Scheduler (archive), all Spirits (memory.read/write) |
| **Security Manager** | Sandbox backends, Secrets adapter, Approval rendering | Capability Registry (sandbox profile lookup) |
| **I/O Subsystem** | Concrete transport adapters (HTTP, stdio, mTLS) | All inbound clients; outbound calls from Capability Registry |
| **IAC Bus** | Telemetry Stream (logging), Persistence (Transparency Log), Spirit Scheduler (mailbox addresses) | All Spirits (iac.send), control plane (broadcasting) |
| **Capability Registry** | Security Manager, I/O Subsystem, Memory Manager, Telemetry Stream | Every Spirit interaction with the world |
| **Telemetry Stream** | nothing (pure broadcast) | Spirit Scheduler, IAC Bus, Capability Registry, all Spirits (subscriptions) |

The Capability Registry is the busiest service — every external call funnels through it. It is decomposed into four sub-services (cap-tokens / cap-policy / cap-audit / cap-quota) so the hot path (token issue/verify) does not serialize on the audit/lineage path. The Telemetry Stream is the simplest — pure broadcast, no state, no I/O. It is the kernel's lung.

### 4.0.4 Technology choices

| Concern | Choice | Why |
|---|---|---|
| Language | Rust + Tokio | Type-safe invariants, mature async runtime, zero-cost abstractions, no GC pauses on the hot path |
| In-process IPC | `tokio::sync::mpsc` + `tokio::sync::broadcast` | Bounded mailboxes, backpressure, codex precedent |
| Subprocess transport | LSP-style `Content-Length` framing over stdio with CBOR payloads | Boring, well-understood (LSP precedent), language-neutral, byte-stable across SDKs |
| Cross-Host transport | mTLS over TCP, JSON-RPC framing | Single transport for the bilateral case; well-understood TLS toolchain |
| Sandboxing | OS-native primitives (Landlock+seccomp on Linux, Seatbelt on macOS, restricted-token + Job Object on Windows) | Already production-grade; codex has shipped all three |
| Persistence | SQLite (per-Host Transparency Log + Approval Decision Log + Journal) + Postgres+pgvector (Loom-lite collective tier at v1.5) | SQLite for single-Host append-only audit; Postgres for the diagnostic-architect pair's shared pattern library |
| Secrets | OS keychain (Linux secret-service / macOS Keychain / Windows Credential Manager) | The kernel does not store secrets (I9) |
| Cryptography | Ed25519 for Spirit signing + signed export; mTLS via `rustls` | Boring, audited, FIPS-pluggable via provider trait |
| Hot-swap state-transfer | CBOR + per-Spirit-class versioned schema | Typed, compact, language-neutral, schema-evolved |

### 4.0.5 Spirit-form abstraction

Two Spirit forms ship; both speak the same Spirit ABI through different runtime substrates:

| Form | Phase | Languages | Latency | Trust |
|---|---|---|---|---|
| `rust-inproc` | v0.1+ | Rust only | Function-pointer dispatch, nanoseconds | Implicit (compiled into kernel binary) |
| `subprocess` | v1.0+ | Any language with a Spirit Wire Protocol implementation | tens of microseconds round-trip | Explicit (Ed25519-signed; trust-tier-enforced; sandboxed) |

A Spirit's manifest declares which forms it ships in. Capability scopes are not portable across forms: a Spirit calling `std::process::Command` builds under `subprocess` and `rust-inproc` but not in any environment that forbids exec. The Spirit registry refuses incompatible builds at publish time.

### 4.0.6 Why no kernel-resident memory store

The kernel itself stores no patterns, no Spirit memory beyond capability-token state, and no learned behaviors. Patterns, ADRs, fix templates, regression tests — the *Loom-curated collective knowledge* the diagnostic-architect bilateral pair maintains — live in user-space (Loom-lite, a Postgres+pgvector instance the operator deploys), not the kernel. The kernel only enables propagation. What gets propagated is the user's data, governed by the user's policy. This is Invariant I9 made concrete: the kernel is **mediator and supervisor**, not knowledge accumulator.

### 4.0.7 What the Kernel Does NOT Compute

The kernel's value comes from what it deliberately refuses to do as much as from what it provides:

- **The kernel does NOT interpret tag semantics.** Tagged scalars and tagged frames carry meaning the kernel transports without reading. Variance, entropy, expected free energy, KL divergence, ensemble disagreement, calibration, similarity, derivatives, statistical tests, contradiction detection — all Spirit-side computations. The kernel performs universal arithmetic comparison only via four predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`).
- **The kernel does NOT author cognitive content.** Distillation, summarization, planning, reasoning, dialectical update, hypothesis generation, posture inference — all Spirit-side. The kernel provides storage, lineage, namespacing, and the Inference Port; cognitive work belongs to actors.
- **The kernel does NOT embed an orchestration policy.** Multi-Spirit coordination patterns (supervisor, peer, market, pipeline) are user-space Spirit patterns, not kernel features. The kernel routes typed-intent IAC frames neutrally; Orchestrator-class Spirits do the directing.
- **The kernel does NOT write skills, rank skills, or curate skills.** Skills are Spirit-author craft; admission is operator-mediated; the kernel hosts the registry mechanism only.
- **The kernel does NOT host collective knowledge directly.** Loom-lite is a user-space service running under MCP-Streamable-HTTP. The kernel mediates access; the merge strategy and curation policy belong to the operator.
- **The kernel does NOT own application-layer concerns.** Messaging gateways, UI presentation, narrative digest content, training-data generation — all Spirit-side. The kernel offers extension contracts; applications fill them.

These refusals are what keep the kernel small and replaceable.

### 4.0.8 Service vs Internal Module — operational definition

The five-services-plus-two-internal-modules framing is not a stylistic distinction; it has testable consequences. A component is a **service** if and only if all four properties below hold; otherwise it is an **internal module** of a parent service.

| Property | Service | Internal module |
|---|---|---|
| **Crate boundary** | Separate Cargo crate under `crates/services/<name>/`; published `Cargo.toml` with own `[package]` section | Sub-module under a parent service crate (`crates/services/<parent>/src/<module>.rs` or `mod/`); shares parent's `Cargo.toml` |
| **Process boundary** | May run in its own OS process when the deployment topology requires (separate `tokio::main` or spawned binary; own `bin/` target). At v0.1 every service runs in the same kernel binary, but the service is *capable of* extraction without code change. | Always runs in the address space of its parent service; no independent binary |
| **IPC contract** | Inter-service calls go through the typed IAC bus (§7.1); contract defined in `crates/iac/proto/`; mockable for unit test | Intra-service calls are direct Rust function calls; no proto definition |
| **Failure domain** | Independently restartable by the supervisor (§4.1 Spirit Scheduler analog applies); a panic in this service does not take down peers | Crashes with parent service; supervisor restarts the parent, not the module |

**Reproduction test.** Given a candidate component X, an implementer answers the four yes/no questions against the codebase and gets a deterministic classification. Security Manager passes all four → service. The I/O Subsystem at v0.1 fails crate-boundary and process-boundary → internal module of `maos-kernel-core`. Telemetry Stream at v0.1 fails the same two → internal module.

**Boundary enforcement is mechanical, not type-system.** The four properties above are facts about the repository layout and Cargo manifests, not facts a Rust type can know — crate identity, bin-target presence, and supervisor restart policy are all external metadata that no `const` on a trait can encode. Enforcement lives in `xtask/src/check_service_boundary.rs`, run in CI as `cargo xtask check-service-boundary`.

The xtask asserts, for each entry in `SERVICES` (a const list in the xtask itself):

- **P1.** `crates/services/<name>/Cargo.toml` exists and declares `[lib]`.
- **P2.** `crates/services/<name>/src/bin/<name>.rs` exists OR Cargo.toml declares a `[[bin]]` target named `<name>`.
- **P3.** `crates/iac/proto/src/<name>.rs` exists and is `pub mod`-exported from `crates/iac/proto/src/lib.rs`.
- **P4.** `crates/services/<name>/src/main.rs` (or the bin target above) calls `std::process::exit` only via `iac_runtime::shutdown::exit_code(...)`, verified by `syn`-based AST scan rejecting bare `std::process::exit`.

Failure of any property fails CI. The xtask carries two const lists:

```rust
const SUPERVISED_SERVICES: &[&str] = &["security", "memory", "iac", "capability"];
const SUPERVISOR: &str = "spirit-scheduler";
```

Adding a fifth supervised service requires adding it to `SUPERVISED_SERVICES` AND ensuring P1–P4 are satisfied in the codebase. Service-Boundary Manifests (e.g., §4.3.5 for Security Manager) **declare** the canonical filesystem locations the xtask checks; the **xtask** verifies those locations exist and the **test suite** verifies the running system honors the boundary. Three layers, distinct: spec declares (this document, §4.3.5), xtask verifies anchors exist (`cargo xtask check-service-boundary`), tests verify behavior (release gate per §13).

**Supervisor exception.** Exactly one component in the system — the Spirit Scheduler — is the composition root: the binary whose `main` instantiates and supervises the four supervised services. The supervisor satisfies P1, P2, and P4 but is exempt from P3 (boundary manifest in the standard shape) because its boundary *is* the union of its children's boundaries. The xtask verifier checks the four supervised services against P1–P4 and verifies the supervisor against P1, P2, P4 only. Any future component must declare itself either a supervised service (full P1–P4) or a module (no boundary, contained within a service); a second supervisor is a structural change requiring this section to be revisited.

**Telemetry-label divergence (intentional).** The §4.7.1 telemetry contract (`iac_rt_*` metrics) labels `service ∈ {security, memory, iac, capability, spirit_scheduler}` — five entries, the supervised four plus the supervisor. This is intentional: the xtask classification answers "what does this run as" (architecture); the telemetry label answers "who originated this RT" (operations). Spirit Scheduler does originate IAC RTs (supervisor-initiated capability checks, lifecycle frames) and must be observable by the same labels as everyone else.

```rust
// xtask/src/check_service_boundary.rs (skeleton)
const SERVICES: &[&str] = &["security", "memory", "iac", "capability"];

pub fn run(workspace_root: &Path) -> anyhow::Result<()> {
    for svc in SERVICES {
        check_p1_own_crate(workspace_root, svc)?;
        check_p2_own_bin(workspace_root, svc)?;
        check_p3_proto_module(workspace_root, svc)?;
        check_p4_supervised_exit(workspace_root, svc)?;
    }
    Ok(())
}
```

**v0.5+ extraction rule.** When extraction of an internal module to a service is proposed (e.g., I/O Subsystem becomes its own service when multi-provider rate-limiting demand justifies it), the change is mechanical: add the module's name to `SERVICES`, satisfy P1–P4 in the codebase, run `cargo xtask check-service-boundary`. The v0.5+ ADR for any extraction documents which properties flip and what the operational consequences are (independent restart policy, separate metric namespace, etc.).

The five services + two internal modules are detailed in §4.1–§4.7 below. **§4.1, §4.2, §4.3, §4.5, §4.6 describe services with their own task pools and explicit trust boundaries. §4.4 (I/O Subsystem) and §4.7 (Telemetry Stream) describe internal kernel modules** — they live inside `maos-kernel-core` rather than as separate services at v0.1. Read them in order — each builds on the previous.

**v0.1-β interpretation note (Story 2.2):** The §4.0.8 four-property test is mechanically enforced at v0.1-β against the current `crates/maos-kernel-core/src/{security,memory,iac,capability,scheduler,io,telemetry}/` module layout rather than the eventual `crates/services/<name>/` layout. P1 = supervision-tree AST scan of `crates/maos-bin/src/main.rs`'s adapter-constructor call sites; P2 = `maos_kernel_core::api::*` Adapter exports paired with `maos_domain::ports::*Port` re-exports (exemptions in `xtask/src/check_service_boundary.rs::ADAPTER_PORT_EXEMPTIONS`); P3 = cross-reference to `cargo xtask check-empty-kernel` (the I9 walker output is authoritative); P4 = AST scan against `xtask/p4-external-io-denylist.toml` with `xtask/p4-mediated-io-paths.toml` as the mediated-lane allowlist. Spirit-ABI type reflection lives alongside: vtable + trait + `HOOK_NAMES` + `count_hooks!()` consistency check via AST scan of `crates/maos-spirit-abi/src/lifecycle.rs` + `crates/maos-spirit-derive/src/lib.rs`. The v0.5+ `crates/services/<name>/` extraction remains the promotion path: add the module's name to `SERVICES`, satisfy P1–P4 in the new location, re-run the enforcer.

### 4.0.9 Crate dependency triangle rule (added Story 4.1 — A5 decision)

The substrate's three load-bearing crates form a triangle:

- `crates/maos-domain` — pure types, invariants, pure functions. No async runtime.
- `crates/maos-kernel-core` — kernel-side machinery (services, journals, IAC bus, capability registry, halt mechanism). Depends on `maos-domain` + `maos-director-surface` + `maos-spirit-abi` + `maos-providers`.
- `crates/maos-director-surface` — director-side UX flows (notification dispatcher, halt UI, posture shift CLI). Depends on `maos-domain` only.

The cycle that re-emerges in any kernel-machinery story (halt, lifecycle, IAC, capability):
`kernel-core → director-surface → <trait the kernel uses>` would close on itself if the trait lived in `kernel-core`.

**Rule:** trait definitions go to the lowest crate in the dependency graph that all consumers can reach.

- Halt trait `HaltResolver` → `maos-domain::halt` (consumers: `kernel-core::halt::KernelHaltResolver` and `director-surface::halt_ui::HaltFlow`).
- Halt journal trait `HaltJournal` → `maos-domain::halt` (consumers: `kernel-core::iac::transparency_log::TransparencyLogAdapter::impl` and `director-surface::halt_ui::HaltFlow`).
- (Future) Lifecycle trait `LifecycleResolver` (Story 5.1) → `maos-domain::lifecycle`.

**Test-double placement:** test doubles (`MockHaltResolver`, `FailingHaltResolver`) live in `kernel-core` because the kernel-side machinery (TL + Journal + Registry) is what their tests exercise. They are NOT under `#[cfg(test)]` because integration tests under `crates/*/tests/` consume them — but they MUST NOT appear in `target/release/maos` symbol table (Story 4.1 A2 ships `xtask check-mock-not-in-release` to enforce).

**Director-surface seam:** `director-surface` SHOULD NOT depend on `kernel-core` for test types. When test-only types would otherwise cycle, define a local test double inside `director-surface/tests/` (see `crates/maos-director-surface/src/halt_ui.rs::tests::TestResolver` for the established pattern, intentional per Story 3.3 review §What Was Challenging §5).

**Story 5.1 application:** the supervised lifecycle (Story 5.1) will introduce `LifecycleResolver` or equivalent — the spec author MUST place the trait at `maos-domain::lifecycle`, NOT at `kernel-core::lifecycle::resolver`. This addendum is the load-bearing reference.

## 4.1 Spirit Scheduler

**Responsibility:** Lifecycle management for all Spirits on this Host.

**State:**
- `Map<SpiritId, SpiritControlBlock>` — the OS-style PCB analog
- `Journal` — append-only on-disk log of all lifecycle transitions (for I10)
- `ResourceBudgets` — per-Spirit caps on tokens/min, $/hour, parallel tool calls

**Operations exposed to user-space (via control-plane API):**
- `load(manifest_path) → SpiritId`
- `start(SpiritId)`
- `pause(SpiritId)`
- `swap(SpiritId, new_manifest_path)` — hot-swap; preserves memory scope and in-flight Capability Tokens (I6)
- `migrate(SpiritId, target_host)` — bilateral A2A-mediated; serializes manifest + memory pages + token set; used by the diagnostic-architect pair handoff
- `snapshot(SpiritId) → SnapshotId` / `restore(SnapshotId)`
- `unload(SpiritId)` — graceful shutdown via lifecycle hooks (§5.3)

**Scheduling discipline (in-process):** Cooperative, priority-weighted, bounded by the Capability Registry's rate limits (so a runaway Spirit cannot starve peers via tool calls). LLM-bound Spirits yield naturally on streaming chunks; CPU-bound Spirits get a `tokio::task::yield_now` injection at sandbox boundaries. Cooperative-yield assumption holds inside a single Spirit's task pool only.

**OS-level CPU/memory budget enforcement:** subprocess-form Spirits run inside Linux cgroups v2 with declared `cpu.max` and `memory.max` ceilings — kernel sets these at spawn, enforced by the OS, not by Tokio cooperation. macOS uses POSIX `setrlimit(RLIMIT_CPU, RLIMIT_RSS)` per child; Windows uses Job Objects with `JOB_OBJECT_LIMIT_PROCESS_TIME` and `JOB_OBJECT_LIMIT_PROCESS_MEMORY`. Default ceilings declared in the `[resources]` table of the manifest; kernel applies the strictest-of (manifest, operator policy) at spawn. Across Spirit processes the OS, not the runtime, is the floor.

**Crash detection and recovery (I10).** The Scheduler supervises every subprocess Spirit. Crash detection ≤2s on SIGKILL; `task.orphaned` IAC frame emitted to in-flight task originators ≤5s with exit-cause journaled (signal, exit-code, stderr-tail). Hung-Spirit detection (alive but no progress IAC for >30s) emits `task.stalled` event within 60s. On crash mid-CBOR-snapshot-write, the supervisor's `JoinSet` returns `Err`; supervisor synchronously calls `cap_registry.revoke_all(spirit_id)`; journal records `HaltRecord{cause: Fault, in_flight_tokens: [...]}`; any half-written CBOR frame in the journal is marked `Torn` on replay and discarded. Replay rule: torn frame at tail = truncate; torn frame mid-log = fatal corruption requiring manual recovery.

**Trade-offs the Scheduler does NOT make:**
- It does not pick which Spirit handles a given user request. That is a user-space concern (the **Routing Spirit**, a default Butler-class instance, does it).
- It does not do auto-scaling, auto-replication, or HA. Those are deployment concerns, not kernel concerns.

## 4.2 Memory Manager

**Responsibility:** Provide three named memory tiers to every Spirit, enforce scope from the manifest, support hot-swap and migration.

**Three tiers:**

| Tier | Scope | Backed by | Lifetime | Use case |
|---|---|---|---|---|
| `private` | This Spirit instance only | `Arc<RwLock<HashMap<...>>>` per-Spirit, plus `fs.write` to per-Spirit-namespaced filesystem area | Spirit lifetime + episodic persistence if declared | Working memory, scratchpad, session state |
| `shared` | All Spirits on this Host (subject to `[memory.shared]` access list) | SQLite-backed key-value with namespace prefix per writer Spirit | Host lifetime | Cross-Spirit coordination on this Host (Orchestrator-Worker handoff payloads, founder-loop state) |
| `collective` (Loom-lite) | Both Hosts in a bilateral pair | Postgres+pgvector exposed via MCP-Streamable-HTTP | Loom domain lifetime | ADR-pattern library, fix templates, regression-test references for the diagnostic-architect bilateral pair |

**Namespace enforcement (I5):** every read/write goes through a kernel-mediated path. `mem.write(scope, key, value)` validates that the calling Spirit's manifest declares write access to `scope`; `mem.read(scope, key)` validates declared read access. Cross-Spirit reads on `shared` are explicit allow-list; cross-Spirit reads on `private` are forbidden by construction (no surface to read another Spirit's private namespace from outside).

**Principal Memory Namespace.** A typed namespace within the private tier — `principal:<principal_id>:<spirit-author-defined-schema>`. Writes to this namespace are tagged as principal-related data and inherit three kernel-mediated operations: **subject-access query** (DPO requests "show everything about principal X"), **right-to-be-forgotten** (operator command removes all principal-namespaced entries for a given subject), **redaction-on-export** (sealed-export scrubs principal-namespace entries unless explicit `--include-principal` flag). Schema is Spirit-author-declared; the kernel only knows that data tagged `principal:<id>:*` is subject to the three operations above.

**Hot-swap and migration:** the Memory Manager swaps memory scope along with Spirit class (I6). For `swap()`, private memory is preserved (the swapping-in Spirit inherits it via `on_swap_in`'s `predecessor_state` argument). For `migrate()`, private memory is serialized into the migration payload along with the manifest and the open token set; the receiving Host's Memory Manager rehydrates on `on_swap_in`. Shared memory is left in place (it belongs to the Host, not the Spirit). Collective memory (Loom-lite) is reachable from either Host in the bilateral pair without migration.

**The kernel does not interpret memory contents.** Schema is entirely Spirit-author-declared. The kernel only knows what scope a write targets, what `kind` tag (`raw`, `digest`, `principal:*`, ...) the payload carries, and — for digest-tagged writes — what `source_log_ref` and `distillation_depth` claim per I11.

## 4.3 Security Manager

**Responsibility:** Sandboxing, secret materialization, approval mediation, posture enforcement.

### 4.3.1 Sandbox tiers

Four tiers, declared per Spirit in the manifest, enforced at process spawn:

| Tier | Profile | Use case |
|---|---|---|
| **T0** | No sandbox; full host privileges | Trusted local-tier Spirits only (operator-authored, `local` trust tier) |
| **T1** | Process isolation; UID separation; no special syscall filtering | Default for `org-internal` trust tier |
| **T2** | Linux: Landlock + seccomp-bpf with allow-listed syscalls + filesystem subtree restriction. macOS: Seatbelt with `.sbpl` profile. Windows: restricted token. Default for `public-untrusted` trust tier. | Default for third-party Spirits at `public-untrusted` |
| **T3** | T2 + container (Docker/Podman) | Spirits with broad capability surfaces (Researcher with web/arXiv/GitHub/citation-graph; Diagnostic Engineer with cross-environment telemetry queries) |

**Strictest-of-(manifest, trust-tier, operator-policy) floor.** The kernel applies the strictest sandbox tier from any of: the Spirit's manifest declaration, its trust tier, the operator's deployment policy. A `public-untrusted` Spirit declaring T0 in its manifest is forced to T2 by the trust-tier floor.

**Per-Spirit resource isolation.** Each subprocess-form Spirit runs under a resource cgroup (Linux cgroups v2; equivalent on macOS/Windows) with kernel-enforced caps on CPU, memory, file descriptors, and process count. Sandbox tiers cover the *security* boundary; resource cgroups cover the *resource* boundary. A runaway Spirit gets throttled, not the host.

### 4.3.2 Secret materialization

The kernel itself stores nothing (I9). Secrets are materialized just-in-time from:
- OS keyring (Linux secret-service / macOS Keychain / Windows Credential Manager) — default
- Encrypted-file vault (`maos-secrets` with `encrypted-file` feature) — for headless operator deployments

Secrets pass through the Capability Registry to the calling adapter (e.g., `provider.complete` materializes the Anthropic API key just before the HTTPS request, redacts it from any log) and are never journaled in cleartext. **Pre-write secret-redaction filter at the Transparency Log boundary.** Frames passing through the IAC Bus are scanned for known secret patterns (API keys, capability tokens, mTLS private-key bytes) before being written to the log; any match is redacted with a typed marker (`<REDACTED:type=api_key,len=…,hash=…>`). Floor: 0 secrets in any logged frame across the bounded test populations (10⁴-case corpus per-commit, 10⁵-case quarterly audit, 1000-canary-secrets-per-month production canary system). Production canary leak detection halts the distillation pipeline until root-caused; discovery latency ≤24h p95.

### 4.3.3 Approval class taxonomy

Six classes, with default policies the operator may override per Spirit:

| Class | Examples | Default policy |
|---|---|---|
| `readonly_scoped` | `fs.read` within manifest-declared subtree, `mem.read` private | Silent allow |
| `readonly_search` | `web.search`, `arxiv.search`, `mcp.tool` reads | Silent allow with rate limit |
| `mutating` | `fs.write` private, `mem.write` private | Silent allow within scope |
| `exec_capable` | `bash.exec`, `git.commit`, `provider.complete` (cost) | `prompt_with_diff` — show what will change before approving |
| `control_plane` | Spirit lifecycle (load/swap/unload), capability scope expansion, posture change | `prompt` — explicit approval, no remember-this-decision |
| `interactive` | Tool calls that emit audio/visual or external messages | `prompt` |

The Approval Manager's UX surface is owned by the IAC Bus — prompts can render in the local TUI, in the editor (via ACP), or as a mobile push notification.

### 4.3.4 Token Lifecycle Manager

Capability tokens are short-lived (TTL ≤60s for high-privilege operations), bound to (Spirit-PID + boot-nonce + expiry), audit-logged at every use with origin-Spirit-ID. Tokens are non-transferable — they bind to the Spirit that requested them. **Hot-swap (I6)** preserves the token but rebinds the actor: when Spirit A is swapped to Spirit B, B inherits the in-flight tokens but its first action against any of them triggers a `posture_change` audit event.

The Token Lifecycle Manager handles re-validation at use against current state (TOCTOU correctness): every capability invocation re-reads the current posture, the current sandbox tier, the current consent envelope, and rejects if any have changed since issuance. There is no caching past state-change boundaries.

### 4.3.5 Service-Boundary Manifest (P1–P4 per §4.0.8)

Security Manager is one of the kernel's services (per the §4.0.8 four-property test). The four boundary properties are recorded here as filesystem-and-Cargo-manifest facts; the §4.0.8 xtask reads these locations and fails CI on drift between manifest and filesystem.

| Property | Location at v0.1 |
|---|---|
| **P1: own crate** | `crates/services/security/Cargo.toml` (declares `[lib] name = "security"`); compiled as a separate Cargo crate within the `maos-kernel-core` workspace |
| **P2: own bin target** | `crates/services/security/src/bin/security.rs` (also declared as a `[[bin]]` target in the crate's `Cargo.toml`); v0.1 ships this in the same kernel binary via composition root, but the bin target exists for future extraction without code change |
| **P3: IPC contract crate** | `crates/iac/proto/src/security.rs` (re-exported as `iac_proto::security`); inter-service calls into Security Manager go through the typed IAC bus (§7.1) |
| **P4: independently restartable** | Supervised by `iac_runtime::supervisor::ServiceHandle`; restart-on-exit policy `RestartPolicy::Always { backoff_ms: 500..=30_000 }`; a panic in Security Manager does not take down peer services |

Analogous Service-Boundary Manifests for Memory Manager (§4.2), IAC Bus (§4.5), and Capability Registry (§4.6) — same four-property shape as this §4.3.5 — are recommended for a future revision; only Security Manager carries one in v1.0. Spirit Scheduler (§4.1) is the **supervisor** as defined in §4.0.8 and is exempt from P3 per that section's supervisor exception. It satisfies P1 (own crate), P2 (own bin target — `crates/services/spirit-scheduler/src/bin/spirit-scheduler.rs`, the kernel binary's `main`), and P4 (independently restartable — though as the supervisor it is the *target* of restart, not the *initiator*).

The two internal modules — I/O Subsystem (§4.4) and Telemetry Stream (§4.7) — fail at least one of P1–P4 at v0.1 (no separate crate, no bin target). They are eligible for extraction to services at v0.5+ when the four-property test can be satisfied.

## 4.4 I/O Subsystem (internal kernel module at v0.1; service-extraction at v0.5+)

**Status at v0.1:** Internal module within `maos-kernel-core`, not a separate service. Lives in `maos-kernel-core::io`. Service extraction (with its own task pool, retry budget, and circuit-breaker policy) is a v0.5+ option gated on real multi-provider deployment and observed contention.

**Responsibility:** Concrete transport adapters for everything that crosses the Host boundary.

**Adapters:**
- HTTP/HTTPS client (provider drivers, MCP-Streamable-HTTP servers, Spirit registry)
- HTTP/HTTPS server (control plane, ACP server, registry-side endpoints)
- Stdio transport (subprocess Spirit Wire Protocol; ACP server fallback)
- mTLS server + client (bilateral A2A peer)
- WebSocket (optional, for real-time editor integrations)

**Provider rate-limit isolation.** Per-(provider, credential) token bucket with kernel-mediated backpressure surfaced as a typed `RateLimited` IAC frame, not a stalled call. One Spirit hitting Anthropic's RPM limit must not block another Spirit on a different provider, or even the same provider with a different key. Bucket parameters declared in provider driver config.

**Network partition behavior in cross-host A2A.** A2A in-flight frames during partition are NACKed after a configurable timeout (default 30s); the kernel does NOT auto-retry. The application layer (the Orchestrator or peer Spirit) decides retry/escalate/halt.

## 4.5 IAC Bus (Inter-Agent Communication)

**Responsibility:** Same-Host frame routing, cross-Host bilateral A2A, the `retract` primitive, the notification surface dispatch.

**Same-Host (mailbox):** mpsc + broadcast, addressable by `SpiritId`. Bounded queues; backpressure via the Spirit Scheduler. Modeled on codex's `Mailbox`. Every frame is logged before delivery (I2).

**Cross-Host (bilateral A2A):** mTLS over TCP between two pre-paired Hosts. Each Host has the other's mTLS certificate fingerprint configured at deployment time (no discovery; the operator names the two endpoints). Per-frame ADR-012 typed-intent consent at both ends — sender's manifest declares which intent classes it will send under to which peer; receiver's manifest declares which intent classes it accepts from which peer. The kernel rejects frames whose typed intent is not in the sender's send-allowlist or the receiver's accept-allowlist with `EIntentDenied`. Logical clocks (Lamport or hybrid logical clock) are used for cross-Host frame ordering; wall-clock is metadata only.

**Logical-clock frame ordering.** Cross-Host frame ordering is consistent under clock skew. Certificate validity windows remain wall-clock (X.509 conventions; the kernel does not reinvent).

**The IAC Bus also owns the `retract` primitive:** a Spirit can issue `retract(message_id, reason)`; the kernel marks the original log entry as retracted, sends a structured `retract` frame to the peer, and the peer's IAC Bus surfaces it to its human. **Retract is not delete** — the Transparency Log is append-only.

**Partial-consent failure semantics.** A frame whose sender approved but whose receiver rejected mid-frame (intent allowlist mismatch, posture change during transmission, token revocation) becomes a typed `ConsentRupture` event; the frame is quarantined, not delivered, not silently dropped. The sender's Spirit receives a `ConsentRupture` IAC frame; the operator surface logs the rupture for forensic review.

**Orchestrator distillate dispatch (Story 6.2 / FR21).** Orchestrator dispatch follow-up to prior Worker completion within `ORCHESTRATOR_DISPATCH_WINDOW` (default 60s; operator-configurable) MUST reference the `DistillationReceipt::digest_frame_id`, not raw frame ids — FR21 closes the raw-output context-overflow loophole. The kernel-side gate `check_orchestrator_distillate_required` fires from `IacBusAdapter::deliver_typed` BEFORE the I13 lineage check and rejects offending frames with `EOrchestratorDispatchRawOutput`. The bus REJECTS the frame; the Transparency Log row is NOT written.

## 4.6 Capability Registry

**Responsibility:** Mediate every external call. Issue, verify, and revoke capability tokens. Enforce manifest-declared capability surfaces. Validate I11/I12/I13 audit-chain fields on digest writes. Track per-Spirit budget (ADR-016 token-budget accounting).

**Decomposition (round-2 ADR-030).** The Capability Registry is internally split into four sub-services so the hot path does not serialize on the slow path:

| Sub-service | Responsibility | Lock model |
|---|---|---|
| `cap-tokens` | Token issue, verify, revoke | Sharded `Arc<[CapShard; 64]>` where each shard is `RwLock<HashMap<TokenId, AtomicCounters>>`. Verify path takes read-lock on one shard (hash(token) % 64), CAS on `AtomicU64` quota counters. No global lock. |
| `cap-policy` | Consent rules, intent allowlists, posture-bound capability surfaces | Read-mostly; copy-on-write for policy updates |
| `cap-audit` | Transparency Log writer, lineage validation (I11/I12/I13) | bounded MPSC `tokio::sync::mpsc::channel(8192)` to a single `audit_writer` task that batches into the journal |
| `cap-quota` | Per-Spirit budget tracking, `ContextPressure`/`ContextLimit`/`EContextExhausted` typed frames | Per-Spirit atomic counters; soft threshold (80%) emits `ContextPressure`, hard (95%) emits `ContextLimit`, above 100% returns `EContextExhausted` on new tool calls |

The hot path (token verify on every IAC frame and every tool call) goes through `cap-tokens` only, with sharded atomic operations. The audit/lineage path is async (mpsc channel) so a slow Transparency Log write cannot block frame delivery.

### 4.6.1 Epistemic Halt mechanism

When a Spirit's evidence is insufficient or contradictory, the Spirit invokes `epistemic.halt(payload)`. The kernel takes four actions atomically:

1. **Logs** the halt to the Transparency Log as a typed `epistemic_halt` entry, with the structured payload, the tasks/frames in flight, and the Spirit's confidence at halt time.
2. **Transitions** the Spirit to the `EpistemicHalt` lifecycle sub-state — distinct from `AwaitingApproval` (which gates Capability Tokens) and `Suspended` (user-initiated pause). All in-flight Capability Tokens are *frozen*, not released — if the user provides resolution and the Spirit resumes, the tokens come back live (subject to expiry).
3. **Surfaces** the halt to the user via the kernel-rendered notification surface as a structured "I cannot answer this confidently" outcome.
4. **Returns** a `halt_id` to the Spirit, which the Spirit retains for correlation when resolution arrives.

**Halt payload schema** (the kernel-known shape; v1.0 closed):

```jsonc
{
  "gap_kind": "evidence_conflict | evidence_insufficient | source_unreliable | beyond_capability | resource_unavailable",
  "summary": "human-readable, 1–2 sentences",
  "evidence_so_far": "optional reference: memory key, transcript range, or list of MCP-resource URIs",
  "query_strategies": ["optional", "list", "of", "concrete next steps the Spirit would take if given resolution"],
  "confidence_at_halt": 0.42  // optional float [0,1]
}
```

**Resolution.** The user (or another Spirit acting through the control plane) responds via `epistemic/resolve(halt_id, resolution)`. Three resolution kinds:

- `provided_context` — additional evidence, sources, or instructions are attached. The Spirit's `epistemic/resolve` handler decides whether the new context closes the gap. If yes, the Spirit transitions back to `Running` with frozen tokens reactivated. If no, the Spirit may halt again (with a refined payload) or accept the halt.
- `accepted_halt` — the user agrees the Spirit cannot proceed. The Spirit transitions to `Unloaded` (or to a clean checkpoint, depending on its `on_unload` hook). The original task is marked `abandoned` in the Transparency Log; downstream consumers can route the work elsewhere.
- `authorized_override` — the user explicitly accepts the risk and tells the Spirit to proceed despite the gap. The Transparency Log records the override with the user's stated reason. The Spirit's subsequent output carries an `override_marker` (mandatory for `output_shape` predicates), so downstream consumers can see the output proceeded past an acknowledged epistemic gap.

**Halt detection strategy — three-layer composition.** The kernel does NOT introspect Spirit-internal "uncertainty" — it cannot. There is no kernel-side LLM-state inspection, no Future-state probing, no statistical drift detector. Halt detection is composed:

1. **Spirit-self-invocation (primary).** The Spirit calls `epistemic.halt(payload)` from its `[epistemic_policy]` manifest rules (per-tag thresholds + four universal-arithmetic predicates from ADR-022). Trust model: the Spirit is the authority on whether its own evidence is sufficient; the kernel only enforces the declared policy on emit.
2. **Budget-based stall detection (secondary, kernel-side).** When a Spirit holds a `task.assign` and emits no progress IAC frame for >`timeout_no_progress` seconds (default 30s, configurable per manifest), the kernel emits a typed `task.stalled` event to the operator surface. This is NOT a halt — it is an external-detected stall — but it is the kernel's only mechanism for catching Spirits that are silently looping or wedged. Resolution is operator-mediated.
3. **Scalar trajectory tap (tertiary, instrumentation).** Observer Spirits subscribe to a `scalar.tap` stream that emits every Spirit's `working_memory.set_scalar` write. This lets diagnostic Spirits *observe* pre-halt scalar drift, but the *halt decision* still belongs to the Spirit being observed — Observer cannot force a halt on a peer.

**Manifest policy (`[epistemic_policy]`).** Spirits declare per-tag rules that map output frame tags (e.g., `claim.load_bearing`, `claim.exploratory`, `speculation`, `conversational`, `diagnosis.root_cause`) to one of three actions: `verbalize_only`, `flag`, or `halt`. Each rule may also specify `on_confidence_below` (numeric threshold) and `on_evidence_conflict` (boolean). Frames not matching any rule fall through to `default_action`, which itself defaults to `verbalize_only` — the kernel fails *open*, never closed. The Capability Registry intercepts emits on the path to the IAC Bus and enforces the rule for the frame's tag; Spirits cannot opt out of their own declared policy mid-task.

**A Spirit configured well halts rarely.** A Researcher tagged correctly might emit fifty `verbalize_only` frames (conversational, observational, exploratory), a few `flag` frames (claims with non-trivial uncertainty), and one `halt` per session at most — when a load-bearing conclusion sits on contradictory or insufficient evidence. The halt is the alarm bell, not the doorbell.

## 4.7 Telemetry Stream (internal kernel module at v0.1; service-extraction at v0.5+)

**Status at v0.1:** Internal module within `maos-kernel-core`, not a separate service. Lives in `maos-kernel-core::telemetry`. The module exposes a sink interface (so v0.5+ stream-processor implementations swap without API churn). Broadcast subscriptions are spawned onto the **shared kernel `tokio::runtime::Handle`** — no dedicated `LocalSet` or runtime instance, no separate worker-thread pool — which keeps fanout latency tight at v0.1's small subscriber counts. Tokio is **cooperatively scheduled at `.await` points** (no preemption, no time-slice quanta in the OS-scheduler sense); the per-task `coop` budget (Tokio's automatic yield after ~128 poll operations) protects only against accidental tight async loops, not against synchronous blocking. Synchronous blocking calls inside subscriber callbacks (file I/O, `std::sync::Mutex` held across await, CPU loops > 100 µs) MUST be offloaded via `tokio::task::spawn_blocking`. Service extraction is a v0.5+ option gated on real stream-processing demand (e.g., Observer fanout exceeding shared-runtime capacity, or stream-processor implementations that need a dedicated runtime). At v0.5+ extraction, the module would gain its own runtime and become a service per §4.0.8's four-property test.

**Responsibility:** Per-Spirit perceptual surface. Topic-based broadcast + filtered subscription.

The Telemetry Stream is the only kernel service Spirits consume passively. Spirits emit `telemetry.event` frames; the Stream broadcasts to all subscribers matching the topic. Observer-class Spirits subscribe broadly; other Spirits subscribe narrowly (Butler subscribes to Calendar/Slack/Figma topics; Mira subscribes to production-service-metrics topics).

**`scalar.tap` channel.** A dedicated read-only stream from the Capability Registry's tagged-scalar slot. Every `working_memory.set_scalar(tag, value, derived_from)` write emits a `scalar.tap` event with `(spirit_id, tag, value, timestamp)`. Observer Spirits subscribe to see pre-halt scalar drift in real time; this is the diagnostic signal Mira-class Spirits use to characterize an incident's runup.

**OpenTelemetry export adapter.** v0.5 ships basic OpenTelemetry export (every IAC frame, every capability invocation, every halt event); v1.0 adds SLO-class export with structured trace IDs and span linkage.

**Author-observability contract.** A Spirit author can read the same diagnostic surface the operator sees for their own Spirit, redacted of cross-Spirit data. This makes the Spirit author's debugging loop tight without exposing peer-Spirit state.

The Telemetry Stream owns no state. It is the kernel's lung — pure broadcast, no buffering beyond per-subscriber bounded queues.

### 4.7.1 Telemetry Contract — IAC Round-Trip

The Telemetry Stream module is the producer for the IAC round-trip metrics §13.1 alert rules consume. The metric names, types, label sets, and histogram bucket boundaries are normative — implementers cannot wire the §13.1 PromQL without these.

Exposed by the kernel on `/metrics` (Prometheus text format, scrape interval 15s):

| Metric | Type | Unit | Labels |
|---|---|---|---|
| `iac_rt_duration_us` | histogram | microseconds | `service` ∈ {security, memory, iac, capability, spirit_scheduler}, `outcome` ∈ {ok, err, timeout} |
| `iac_rt_inflight` | gauge | requests | `service` |
| `iac_rt_errors_total` | counter | errors | `service`, `kind` ∈ {transport, decode, timeout, app} |

**Note on the `service` label set.** The label set includes `spirit_scheduler` (the supervisor per §4.0.8) in addition to the four supervised services. SS appears as `service` when it originates an IAC RT (supervisor-initiated capability check, lifecycle frame); appears as `peer_id` when it dispatches on behalf of another service. The label set therefore has five entries while the xtask `SUPERVISED_SERVICES` list has four; the divergence is intentional and explained in §4.0.8.

**Metric pair semantics.** `iac_rt_inflight` (gauge, count of in-flight requests) and `iac_rt_duration_us` (histogram, microseconds of round-trip duration) are linked by Little's Law in steady state: `E[inflight] ≈ arrival_rate × E[duration]`. Implementers MUST NOT multiply gauge × histogram-quantile to estimate traffic — use the histogram's `_count` series (Prometheus auto-derived) for arrival rate. The pair is exposed jointly because saturation diagnosis requires both load (inflight) and latency (duration) to discriminate "slow per request" from "more requests than headroom."

**Histogram buckets for `iac_rt_duration_us`** (exponential, base √2, anchored on the 1500µs SLO from §13.1):

```
le = [50, 75, 100, 150, 200, 300, 450, 700, 1000, 1500, 2200, 3300, 5000, 7500, 11000, 16000, 25000, +Inf]
```

Rationale: 18 buckets (under Prometheus's soft 20-bucket guidance per histogram); the SLO threshold (1500µs) is itself a bucket boundary, so `histogram_quantile(0.95, ...)` interpolates within a bucket whose boundaries are explicit, not implementation-dependent. Buckets below 50µs are omitted — IAC round-trip never goes sub-50µs in practice (network syscalls dominate).

The `_bucket`, `_count`, `_sum` suffixes referenced in §13.1 PromQL are the standard Prometheus histogram-derived series; no separate definition is needed beyond standard Prometheus client-library behavior.

Reference Rust constant for the kernel's metric emitter:

```rust
pub const IAC_RT_BUCKETS_US: &[f64] = &[
    50.0, 75.0, 100.0, 150.0, 200.0, 300.0, 450.0, 700.0,
    1000.0, 1500.0, 2200.0, 3300.0, 5000.0, 7500.0, 11000.0,
    16000.0, 25000.0,
];
```

## §4.1.2 Hot-Swap Coordinator — supervisor body (Story 5.2)

The Hot-Swap Coordinator at `crates/maos-kernel-core/src/hot_swap/` implements
ADR-017 (state-transfer wire format, binding-v0.3). It is part of the Spirit
Scheduler supervisor per §4.0.2 line 47.

### Coordinator shape

`HotSwapCoordinator` holds Arc handles to all shared adapters (spirits map,
HaltRegistry, CapabilityRegistryAdapter, IacBusAdapter, JournalAdapter,
TransparencyLogAdapter, HookDispatcher, IacRtMetrics). Constructed exactly once at
the composition root per the §A5 gate (no duplicate adapter instances).

The 12-step `initiate_swap` protocol:
1. Resolve spirit_id → predecessor_pid
2. Snapshot predecessor SCB for saga rollback
3. I14 gate — `validate_swap_halt_continuity` (Story 4.5 wrapper)
4. Fire `on_swap_out` hook
5. Call `snapshot()` → CBOR state blob
6. Decode + validate envelope (same-major vs cross-major)
7. Cross-major → `run_migrator`
8. Atomic SCB swap under write-lock
9. Fire `on_swap_in(payload)` hook
10. Journal `LifecycleEvent::Swap`
11. Spawn `PostSwapMonitor` (30s window)
12. Return `HotSwapResult::Completed`

### Saga compensating transactions (ADR-017)

Three failure boundaries with three compensating arms:
- **on_swap_out failure** → predecessor remains running (no compensation needed)
- **on_swap_in failure** → restore `pre_swap_snapshot` to spirits map
- **Post-swap invariant violation** → auto-revert within 30s

### Post-swap monitor (NFR-Rel-5)

Spawned at swap commit. Polls at 1s cadence for 30s. Checks:
1. Halt-set delta — pending halts before swap ⊆ pending halts after
2. Boot-nonce stability (defensive)
3. Output-shape regression (sample 5 most-recent frames)

`MAOS_AUTO_REVERT_FAST=1` collapses the window to 300ms for tests.

### Cross-major migration (ADR-020)

When schema_version major differs, the coordinator calls `run_migrator`:
1. Verifies `[migrates_from].versions` contains predecessor_version
2. Fires `migrate(predecessor_state)` hook on successor
3. Returns `EMigratorMissing` if no migrator declared

`matches_version_pattern` supports `"0.3.x"`-style wildcards.

### ADR-036 precheck UX

`HotSwapPrecheck::check` is a pure function (no kernel-state mutation).
Wired through `maosctl spirit hot-swap-precheck <spirit> --from <ver> --to <manifest>`.
Reporting-only at v0.3-β; Story 5.4's `maosctl spirit upgrade` calls this internally.

### I14 halt-continuity enforcement

`HotSwapCoordinator::initiate_swap` step 3 calls Story 4.5's
`validate_swap_halt_continuity`. SafeDrained/SafeMigrated permits the swap;
HaltContinuityError maps to `HotSwapError::HaltContinuityViolation`.

### HSIS 300-corpus

`crates/maos-eval/fixtures/hsis-corpus-v0/` hosts 6 classes × 50 = 300
scenarios. Per-class pass threshold ≥95% (NFR-Rel-3), zero CVSS-7 violations.
Measured by `crates/maos-eval/tests/hsis_runner.rs`.

## §4.1.3 Spirit Scheduler — supervision body (Story 5.3)

The supervision module at `crates/maos-kernel-core/src/supervision/` implements crash detection, hung-Spirit detection, silent-failure detection, cold-restart recovery, and FR50 dead-Spirit task disposition. It is part of the Spirit Scheduler supervisor per §4.0.2 line 47 and follows the same sub-module precedent as `hot_swap/` (Story 5.2).

### CrashDetector (NFR-Rel-1, FR12)

`CrashDetector::handle_crash` executes a 7-step protocol:
1. Acquire SCB from the shared `spirits` map
2. Mark SCB state atomically to `Unloaded`
3. Revoke all capability tokens for the PID
4. Produce halt-receipts via `terminate_spirit` (per-PID `drain_for_spirit`)
5. Emit `task.orphaned` IAC frames for each in-flight task
6. Apply FR50 disposition (`Nack | ReassignToReplica | EscalateToOperator`)
7. Remove SCB and journal `LifecycleEvent::Crash`

The rust-inproc panic seam wires through `HookOutcome::Panicked` → `tokio::spawn(handle_crash)` fire-and-forget before the lifecycle verb returns its error. The subprocess form (forward-shaped) wires through the `SubprocessSupervisor` trait; `OsProcessChildSupervisor` lands at Story 5.5x.

### ProgressWatchdog (NFR-Rel-2)

Polls every 1s (100ms under `MAOS_SUPERVISION_FAST=1`). For each `Running` Spirit with in-flight tasks, compares `last_progress_iac_ns` against `progress_threshold_ms` (default 30s). On stall, emits `FrameKind::TaskStalled` with 60s multi-fire suppression. `last_progress_iac_ns` is updated by the kernel-side mailbox on every spirit-origin IAC frame.

### SilentFailureDetector (NFR-Rel-4)

Polls on the same cadence. Detects Spirits where `last_heartbeat_ns > last_progress_iac_ns + silent_failure_threshold_ms` while holding in-flight tasks. Emits `FrameKind::SilentFailureSuspect`. `KernelCtx::heartbeat()` updates `last_heartbeat_ns`; SDK ergonomics land at Story 7.x.

### Cold-restart (NFR-Rel-10)

`graceful_drain` iterates `scheduler.unload` per Spirit with a deadline. `hard_kill_drain` fsyncs the journal. `JournalEntry::InFlight` persists task assignments for post-restart recovery. `recover_in_flight_with_tasks` re-scans the journal and returns both lifecycle events and in-flight records.

### FR50 disposition

`OnCrashAction` is parsed from the `[on_crash]` manifest section (`nack | reassign-to-replica | escalate-to-operator`). `NullReplicaResolver` is the v0.3-β default (always `None`); multi-instance hosting lands at Story 6.1 + 8.4.

### Halt-receipt unification (NFR-Rel-11)

Planned termination (`scheduler.unload`) and unplanned termination (`CrashDetector::handle_crash`) both route through `terminate_spirit`, producing `FrameKind::EpistemicHalt` rows. The unified pipeline is measured by `halt_receipt_production_rate.rs` (1100 scenarios: 1000 planned + 100 crash).

### smoke-supervision-5 observability arm

`MAOS_ONE_SHOT=smoke-supervision-5` walks all four supervision surfaces end-to-end in one command, printing JSON lines per step with magnitude assertions (≥1 halt receipt, ≥1 task stalled, ≥1 silent failure suspect, ≥1 in-flight recovery).

## §4.1.4 Spirit Lifecycle — upgrade body + revocation pipeline (Story 5.4)

### UpgradeOrchestrator (FR49)

`UpgradeOrchestrator` at `maos-kernel-core::lifecycle::upgrade` dispatches three upgrade policies:

1. **`hot-swap`** (default) — delegates to `HotSwapCoordinator::initiate_swap` (Story 5.2's 12-step protocol with saga compensation, I14 halt-continuity gate, PostSwapMonitor 30s window).
2. **`cold-swap`** — sequenced `scheduler.unload(predecessor_pid)` then manual SCB insertion with new PID. In-flight tasks are lost; halt-receipts produced during unload are captured.
3. **`migrator`** — upfront `[migrates_from]` declaration check (`UpgradeError::MigratorNotDeclared` if absent), then delegates to `HotSwapCoordinator` which routes `SchemaCompat::CrossMajor` through `run_migrator` automatically.

All successful upgrades journal `LifecycleEvent::Upgrade = 15` and emit one `FrameKind::CapabilityInvocation` row with `cap_used=spirit.upgrade`.

### RevocationApplier + RevocationPoller (FR13, FR60)

`RevocationApplier` at `maos-kernel-core::revocation::applier` implements the CRL propagation pipeline:

1. **Idempotency** — `applied_crls: BTreeSet<CrlId>` rejects re-imports with `RevocationError::AlreadyApplied`.
2. **Match** — iterates `spirits.read()` snapshot, matches each SCB against CRL entries by `(spirit_class, semver_range_contains(version, entry.version_range))`.
3. **Revoke** — `capability.revoke_all_for_pid(pid)` sets per-token `AtomicBool` on all 64 shards.
4. **Emit** — `FrameKind::SpiritRevoked = 17` IAC frame with full payload (spirit_id, pid, class, version, origin, reason, action).
5. **Apply action** — `TerminateImmediately` (default): `terminate_spirit(RevocationTerminated)` + fire-and-forget unload. `DrainThenTerminate`: spawn deadline task (`progress_threshold_ms * 2`), let in-flight tasks complete, then terminate + unload. `Quarantine`: v0.3-β downgrades to `DrainThenTerminate + quarantine_requested` audit marker; real T3 container isolation lands at Story 5.5a.
6. **Journal** — `LifecycleEvent::Revoked = 16` per revoked Spirit.

`RevocationPoller` spawns a periodic fetch loop (300s default, 100ms under `MAOS_REVOCATION_FAST=1`) via `RegistryClient` trait. v0.3-β default `LocalFileRegistryClient` reads `~/.local/share/maos/crl/latest.signed.json`. Production `McpRegistryClient` lands at Story 5.5d.

### RegistryClient trait seam

`RegistryClient` lives in `maos-domain::revocation` per architecture §4.0.9 dependency-triangle rule. The kernel-side `RevocationPoller` holds `Arc<dyn RegistryClient>`; the v0.3-β default `LocalFileRegistryClient` lives in `maos-domain::revocation` (zero kernel-core deps); the production `McpRegistryClient` lands at Story 5.5d in `crates/maos-registry/` which depends on `maos-mcp` (Story 5.5c) but NEVER on `maos-kernel-core`.

### NFR-Rel-9 bench

`cargo bench -p maos-kernel-core --bench revocation_propagation_p99` measures `revoke_all_for_pid` latency under 100 issued tokens. The bench is a v0.3-β structural placeholder; the full 10⁴-concurrent-verify storm with first-revoked observation lands at v0.5 measurement gate (§13.1).

### smoke-upgrade-revoke-5 observability arm

`MAOS_ONE_SHOT=smoke-upgrade-revoke-5` walks hot-swap upgrade, cold-swap upgrade, CRL apply, and capability denial in one command, printing 4 JSON lines.

### CI gates

- `nfr-rel-9-revocation-5s-p99` — runs the Criterion bench
- `upgrade-policy-corpus` — runs `upgrade_orchestrator_three_policies` integration test
- `revocation-corpus` — runs `revocation_applier_pipeline` integration test
