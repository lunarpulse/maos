# 12. Architecture Decision Records

Thirty-nine contested decisions, each with rationale, alternatives considered, and what would force a revisit. ADRs are the substrate's evolution mechanism; amendments touching invariants I1–I14 require the `invariant-lock` CI gate to pass.

Every ADR carries a `Status:` and `Gate:` frontmatter line. **Status** is one of:
- `binding-v0.1` — the kernel must implement this from v0.1 onwards. Most-load-bearing tier; changes require a major-version bump and the `invariant-lock` CI gate.
- `binding-v0.3` / `binding-v0.5` / `binding-v0.9` / `binding-v1.0` / `binding-v1.5` — the kernel must implement this by the named phase. Changes before that phase are free; after, they require the `invariant-lock` gate.
- `speculative-vNext` — the decision is on the table for a future phase but the substrate has not yet committed. May be cut, may be promoted to `binding-*`. Speculative ADRs additionally carry a `Resolves-by:` field naming the gate that promotes them.

**Gate** names the falsifiable acceptance test that proves the ADR is implemented at its `Status` phase — a corpus, an invariant CI check, a measurement threshold, or a milestone artifact. `design-only` means no runtime gate yet; promotion to a measurable gate is required before the next phase boundary.

## 12.0 ADR Index Table

Sorted by Status (binding-v0.1 first), then by ADR number. Reviewers triaging "what must I read?" can stop after the binding-v0.1 cluster.

| ADR | Title | Status | Gate |
|---|---|---|---|
| 001 | Kernel language is Rust + Tokio | `binding-v0.1` | v0.1 ships in Rust + Tokio; alternative-language proposals require ADR + benchmark |
| 002 | Spirit form at v0.1 — subprocess only, inproc gated on measurement | `binding-v0.1` | §13 measurement gate (`benches/iac_roundtrip.rs`); promotion to inproc requires three-condition check + superseding ADR |
| 003 | IAC topology — mailbox-on-Host + bilateral A2A cross-Host | `binding-v0.1` | mailbox at v0.1; A2A loopback at v0.9; cross-Host A2A at v1.0 |
| 004 | Sandbox tiers T0/T1/T2/T3 with strictest-of-floor | `binding-v0.1` | T0/T1 at v0.1; T2 at v0.3; T3 at v0.5; trust-tier floor enforced by Capability Registry |
| 005 | Pluggable provider drivers | `binding-v0.1` | Anthropic driver at v0.1; ≥3 providers in CI by v0.5 |
| 006 | The kernel learns no patterns (I9) | `binding-v0.1` | structural-state lint blocks new persistent fields outside `{Journal, TransparencyLog, CapabilityRegistry::tokens}` |
| 007 | Spirit-form phasing | `retired` | Subsumed by ADR-002 (see §12 ADR-007 stub) |
| 010 | Hexagonal architecture for static structure | `binding-v0.1` | crate boundary lint enforces port/adapter ring |
| 011 | Actor model on the runtime hot path | `binding-v0.1` | per-Spirit Tokio task supervision + bounded mailbox |
| 030 | Capability Registry decomposition (cap-tokens / cap-policy / cap-audit / cap-quota) | `binding-v0.1` | hot-path token verify <5µs P99 benchmark |
| 032 | Spirit Wire Protocol bytes-on-wire (LSP framing + CBOR) | `binding-v0.1` | byte-equal golden corpus per frame variant per SDK; 168h cumulative pre-GA fuzz floor |
| 037 | Constitutional amendment process | `binding-v0.1` | `invariant-lock` CI gate runs on every PR touching I1–I14 |
| 038 | Per-service KLOC ceiling | `binding-v0.1` | `xtask/kloc.toml` enforced by `tokei` in CI; aggregate ≤20 KLOC, alarm at 16 |
| 017 | Hot-swap state-transfer wire format (CBOR + per-class schema + saga rollback) | `binding-v0.3` | swap conformance corpus passes; auto-revert ≤30s on post-swap invariant violation |
| 022 | Tagged-scalar working-memory slot with epistemic-policy binding | `binding-v0.3` | `[epistemic_policy]` rules trigger halts via four universal-arithmetic predicates |
| 025 | Proactive scheduling (`on_schedule` lifecycle hook) | `binding-v0.3` | Butler `on_idle` Sandra-scene replay |
| 033 | Subprocess supervision and halt-crash intersection (crash matrix) | `binding-v0.3` | crash-recovery corpus: torn-frame-at-tail truncate, mid-log fatal |
| 008 | Spirit registry as MCP-Streamable-HTTP server | `binding-v0.5` | `registry.search`/`manifest`/`artifact` operational; MCP-Streamable-HTTP transport |
| 009 | Three trust tiers with strictest-of-floor enforcement | `binding-v0.5` | strictest-of-(manifest, trust-tier, operator-policy) floor in registry admission tests |
| 014 | Distillation audit-chain (I11 — `source_log_ref`, `distillation_depth`) | `binding-v0.5` | Capability Registry rejects digest writes with `EDigestAuditChainMissing`; segment-level by default |
| 015 | Decision-context recording (I12 — `working_memory_digest_refs`) | `binding-v0.5` | shadow-recall record on `event/inbound`; refs attached on `decision.*` emit |
| 018 | Intent provenance preservation across distillation (I13 — `intent_lineage`) | `binding-v0.5` | kernel-computed `intent_lineage`; consumer admission rejects with `EIntentPromotionDenied` |
| 020 | Hot-swap migration policy (`migrate(predecessor_state)`) | `binding-v0.5` | kernel refuses load with `EMigratorMissing` if predecessor archive exists and migrator absent |
| 026 | Principal Memory Namespace with redaction-aware operations | `binding-v0.5` | subject-access query / right-to-be-forgotten / redaction-on-export operate on `principal:*` namespace |
| 027 | Skill-package external-standard interop (`maos.skill.v1`) | `binding-v0.5` | Spirit-form adapter loads ≥1 third-party skill format |
| 035 | Observer scalar trajectory channel (`scalar.tap`) | `binding-v0.5` | Observer subscribers see pre-halt scalar drift in real time |
| 040 | Threat-model split — Sec-14a (same-Host) + Sec-14b (cross-Host) | `binding-v0.5` | 200-scenario isolation corpus passes (Sec-14a at v0.9, Sec-14b at v1.0) |
| 012 | Typed-intent A2A consent (closes confused-deputy gap) | `binding-v0.9` | A2A Gateway rejects frames with intent not in send-allowlist or accept-allowlist |
| 013 | Two-level `task.assign` typed-intent IAC primitive | `binding-v0.9` | founder-loop epic-7 reproducible end-to-end |
| 016 | Token-budget accounting (`ContextPressure`/`ContextLimit`/`EContextExhausted`) | `binding-v0.9` | typed frames emit at 80%/95%; new tool calls fail above 100% |
| 019 | Halt continuity across hot-swap (I14) | `binding-v0.9` | kernel refuses swap with `EHaltContinuityViolation` if drain or schema-compatible migration absent |
| 021 | CliWrapperSpirit output-shape adapter contract (fail-loud) | `binding-v0.9` | startup `EOutputShapeAdapterMismatch` if observed shape ≠ declared version |
| 024 | Spirit-authored skills (admission queue + audit) | `binding-v0.9` | `skill.author.self` capability + operator-admission queue operational |
| 034 | Partial-consent failure semantics (`ConsentRupture`) | `binding-v0.9` | sender receives `ConsentRupture` IAC frame on receiver-side rejection |
| 036 | Hot-swap × halt continuity precondition check | `binding-v0.9` | `maosctl swap` surfaces precondition status before initiating |
| 028 | Replay determinism primitive (trace-shape, not payload) | `binding-v1.0` | trace-shape contract validated in CI per `schemas/trace-shape.schema.json`; v1.0 best-effort, v1.5 hard target |
| 029 | Provider/CLI Gateway sub-module contract (`GatewaySubmodule` trait) | `binding-v1.0` | gateway sub-modules registered via `gateway.toml`; per-FR54 conformance |
| 031 | Cross-Form Spirit Equivalence (`spirit-conformance`) | `speculative-vNext` | Resolves-by: ADR-002 measurement gate triggering inproc unlock; conformance suite ≥90% on 200-scenario class corpus |
| 023 | Capability-token TTL + bind-to-PID | `binding-v0.1` | TTL ≤60s for high-privilege; tokens bound to (Spirit-PID + boot-nonce + expiry); TOCTOU re-validation at use |

**Reserved:** ADR-039 — number reserved; not in scope for the substrate. The four universal-arithmetic predicates from ADR-022 cover the journeys this architecture ships. Future predicate-vocabulary extensions, if justified by a Spirit class, would land in a user-space stdlib without altering the kernel surface (see App-D.3).

## ADR-001 — Kernel language is Rust + Tokio

`Status: binding-v0.1` · `Gate: v0.1 ships in Rust + Tokio; alternative-language proposals require ADR + benchmark` · `Decided: 2026-04-15` · `Revisits: §13 v0.1 row`

**Decision.** The kernel is implemented in Rust on the Tokio async runtime. Spirit-side runtimes inherit this for the in-process form; subprocess and cross-form Spirits use language-neutral wire protocols.

**Rationale.** Type-safe invariants (the 14 invariants are easier to enforce structurally in Rust than in Go or TypeScript). Mature async runtime with work-stealing scheduler. Zero-cost abstractions for the hot path (token verify under 5µs P99). No GC pauses. The cohort survey confirmed the choice: codex, ironclaw, rustain are all Rust+Tokio.

**Alternatives considered.** Go (rejected: lack of trait-based zero-cost abstractions; GC pauses unacceptable on capability-token verify). TypeScript with Deno (rejected: no path to FIPS-validated crypto provider; runtime overhead). C++ (rejected: memory safety burden too high for a substrate kernel).

**What would force a revisit.** Rust's async story regresses materially relative to alternatives (unlikely). Tokio bifurcates and a fork becomes the standard (low probability).

## ADR-002 — Spirit form at v0.1 — subprocess only, inproc gated on measurement

`Status: binding-v0.1` · `Gate: §13 measurement gate (benches/iac_roundtrip.rs); promotion to inproc requires three-condition check + superseding ADR` · `Decided: 2026-04-15` · `Revisits: §13 measurement gate; ADR-031` · `Subsumes: ADR-007`

**Decision.** v0.1 ships **subprocess form only**. Spirits run as subprocess binaries speaking the Spirit Wire Protocol (LSP-style `Content-Length` framing + CBOR payloads, ADR-032) over stdio. In-process Rust Spirits (`rust-inproc`) are **not** an alternative on the table at v0.1; they are a future option gated by §13's measurement harness.

**Rationale.** Subprocess gives polyglot reach and process isolation; it is the form Diego's `code-reviewer-pro` ships in, the form the Orchestrator/Worker/Reviewer skill-package overlays use, and the form that makes third-party Spirit publication safe. Adding a second form at v0.1 would double the invariant-enforcement surface (two crash recovery semantics, two memory models, two hot-paths) for a latency win no in-scope journey has been measured to require.

**Alternatives considered.** Two forms at v0.1 (`rust-inproc` + `subprocess`) — rejected: doubles ABI surface during the foundational phase; the operational complexity is not journey-justified. rust-inproc only — rejected: forces every Spirit author into Rust; kills polyglot ambition. Three forms (+ WASM-component) — rejected: third tier adds substantial toolchain complexity without journey-driving demand at this scope.

**Status reconciliation with §13 (Measurement Gate).** This ADR commits to subprocess-only IAC at v0.1. In-process transport is **not** an alternative on the table at v0.1; it is a future option gated by §13's harness (`benches/iac_roundtrip.rs`, journeys J1/J-Butler/J-Researcher). Promotion to inproc requires (a) sustained 24h breach of one threshold in §13's table, (b) confirmation that J-Butler p95 is not >4× J1 p95 (rules out fixable code overhead), and (c) a follow-up ADR superseding this one. Until those three conditions land in writing, "subprocess-only" is the architecture, not a default.

**What would force a revisit.** §13's measurement gate trips for a journey-required Spirit class, with the three-condition check satisfied. A capability-isolation requirement emerges that subprocess's process boundary cannot meet (in which case WASM-component, not rust-inproc, is the candidate). ADR-031 (Cross-Form Spirit Equivalence) is `speculative-vNext` and resolves only when this revisit fires.

## ADR-003 — IAC topology is mailbox-on-Host + bilateral A2A cross-Host

`Status: binding-v0.1` · `Gate: mailbox at v0.1; A2A loopback at v0.9; cross-Host A2A at v1.0` · `Decided: 2026-04-15` · `Revisits: §7.2`

**Decision.** Same-Host IAC uses the kernel-internal mailbox (mpsc + broadcast). Cross-Host IAC uses bilateral A2A — exactly two pre-paired Hosts, mTLS+TOFU, per-frame typed-intent consent.

**Rationale.** Same-Host mailbox is the codex-precedent pattern: low latency, kernel-internal, easy to log-before-deliver. Bilateral A2A is the topology the Diagnostic Engineer + Senior Architect pair operates on; the operator names the two Hosts in deployment configuration, and there is no discovery to hide. The bilateral case is a strict subset of the general A2A protocol — same wire format, same consent envelope, same logical clock — restricted to two endpoints.

**Alternatives considered.** Single-Host only (rejected: J4 Mira-Nash requires production-edge / dev-environment separation). Gateway-based cross-Host (rejected: introduces a single point of failure and makes the gateway a privileged kernel-external component).

**What would force a revisit.** A use case emerges that requires three or more Hosts coordinating in real-time. (At that point this is a different architecture, not an extension.)

## ADR-004 — Hexagonal sandboxing with OS-native primitives

`Status: binding-v0.1` · `Gate: T0/T1 at v0.1; T2 at v0.3; T3 at v0.5; trust-tier floor enforced by Capability Registry` · `Decided: 2026-04-15` · `Revisits: §4.3.1, §8.2`

**Decision.** Sandbox tiers T0 (trusted), T1 (UID separation), T2 (Landlock+seccomp narrow / Seatbelt / Windows restricted-token), T3 (T2 + container) form the security boundary. The strictest-of-(manifest, trust-tier, operator-policy) floor applies.

**Rationale.** OS-native primitives are production-grade (Landlock+seccomp on Linux 5.13+; Seatbelt's `.sbpl` profiles on macOS; restricted-token + Job Object on Windows). Codex has shipped all three in production. Adding a process-level container at T3 layers defense-in-depth without inventing new sandbox primitives.

**Alternatives considered.** WASM-component sandbox for Spirits (considered: capability-isolation by construction; rejected for this scope because subprocess + Ed25519 signing + T2 is sufficient for Diego's third-party publishing). Pure container-based isolation (rejected: containers do not give per-syscall granularity).

**What would force a revisit.** The OS sandbox primitives diverge sufficiently that maintaining all three becomes impractical.

## ADR-005 — Pluggable provider drivers

`Status: binding-v0.1` · `Gate: Anthropic driver at v0.1; ≥3 providers in CI by v0.5` · `Decided: 2026-04-15` · `Revisits: §4.4`

**Decision.** LLM provider access is mediated by `maos-providers`, a feature-gated crate exposing a uniform `provider/complete`, `provider/stream`, `provider/embed` capability surface. Spirit manifests declare which providers they can use; the kernel materializes provider credentials at the capability boundary.

**Rationale.** Provider lock-in is a substrate-level risk. Having pluggable drivers means a Spirit author writes against the kernel's provider API once and runs against any driver. Providers are independent crates; new drivers ship without kernel changes.

**Alternatives considered.** Bundle one provider (Anthropic) and require Spirit authors to call HTTP directly for others (rejected: violates I1 capability mediation). Use a single SDK like LiteLLM (rejected: introduces a third-party dependency on the substrate's hot path).

**What would force a revisit.** A provider semantic emerges that the uniform API cannot represent.

## ADR-006 — The kernel learns no patterns

`Status: binding-v0.1` · `Gate: structural-state lint blocks new persistent fields outside {Journal, TransparencyLog, CapabilityRegistry::tokens}` · `Decided: 2026-04-15` · `Revisits: §3.2 invariant I9; §9.3`

**Decision.** Patterns, ADRs, fix templates, regression tests — the curated collective knowledge — live in user-space (Loom-lite), not the kernel. The kernel mediates access and audits the access; the kernel does not store, index, or learn from the contents.

**Rationale.** Auditability. The kernel is replaceable; the user's data is not. If patterns lived in the kernel, every kernel upgrade would risk corrupting accumulated knowledge, every audit would have to inspect kernel internals, and the substrate's "boring substrate" claim would erode.

**Alternatives considered.** Build a kernel-resident pattern store (rejected: violates I9; turns the kernel into a state machine that depends on accumulated history).

**What would force a revisit.** A use case emerges where Loom-lite's MCP-Streamable-HTTP latency is unacceptable for a hot-path operation. (Threshold: p99 > 200ms on diagnostic-architect bilateral pair operations.)

## ADR-007 — Spirit-form phasing (retired — subsumed by ADR-002)

`Status: retired` · `Subsumed-by: ADR-002`

**Decision retired.** The phasing content of this ADR has been merged into ADR-002 ("Spirit form at v0.1 — subprocess only, inproc gated on measurement"), which is the single source of truth for the Spirit-form question (deployment, isolation, phasing, WASM scope). ADR-007's number is preserved to keep the ADR sequence stable; the number is not reused.

**Why retired.** ADR-002 and ADR-007 had drifted into near-duplicate commitments after the editing pass that converged both on "subprocess at v0.1, rust-inproc gated on §13 measurement, no WASM in scope." Two ADRs saying the same thing means future amendments would be made in one place and not the other; the number-preserving merge eliminates the drift surface.

**Where to look instead.** All Spirit-form decisions: ADR-002. Cross-form equivalence (when/if rust-inproc unlocks): ADR-031 (`speculative-vNext`). Wire protocol: ADR-032. Crash matrix: ADR-033.

## ADR-008 — Spirit registry as MCP-Streamable-HTTP server

`Status: binding-v0.5` · `Gate: registry.search/manifest/artifact operational; MCP-Streamable-HTTP transport` · `Decided: 2026-04-15` · `Revisits: §13 v0.5 row`

**Decision.** The Spirit registry is itself an MCP-Streamable-HTTP server. `maosctl install` calls `registry.search` / `registry.manifest` / `registry.artifact`; `maos-spirit publish` calls `registry.publish`; `registry.deprecate` for yanks.

**Rationale.** The kernel already speaks MCP for tools and Loom-lite. Reusing MCP for the Spirit registry means zero new transport code, and operators can self-host a registry on any MCP-compatible server.

**Alternatives considered.** Custom protocol (rejected: invents a fifth wire protocol). OCI registry (considered: well-understood; rejected because Spirit packages are not OCI-shaped — they include a manifest, a binary, and signing metadata in a structure OCI does not natively support).

**What would force a revisit.** MCP's evolution diverges in a way that makes registry-over-MCP brittle.

## ADR-009 — Three trust tiers with strictest-of-floor enforcement

`Status: binding-v0.5` · `Gate: strictest-of-(manifest, trust-tier, operator-policy) floor in registry admission tests` · `Decided: 2026-04-15` · `Revisits: §13 v0.5 row`

**Decision.** Three trust tiers exist: `local` (operator-authored Spirits or Spirits the operator has personally vetted), `org-internal` (Spirits authored within the organization, vouched for by the operator's signing key), `public-untrusted` (Spirits authored by anyone, signed with the author's Ed25519 key, no organizational vouch). Strictest-of-(manifest, trust tier, operator-policy) floor: a Spirit at `public-untrusted` is forced to T2 sandbox + cautious posture regardless of what its manifest claims.

**Rationale.** Trust tiers enable a public Spirit registry to be safe by default. A Spirit at `public-untrusted` runs under T2 sandbox + cautious posture; the operator can promote individual installations to `org-internal` based on local trust evaluation.

**Alternatives considered.** No trust tiers (rejected: a public registry would be a supply-chain-attack surface). Centralized vetting (rejected: gatekeeping fails the substrate-not-product framing; promotion is operator-local).

**What would force a revisit.** A trust tier is needed between `org-internal` and `public-untrusted` for federations of cooperating organizations.

## ADR-010 — Hexagonal architecture for static structure

`Status: binding-v0.1` · `Gate: crate boundary lint enforces port/adapter ring; domain core compiles without async runtime` · `Decided: 2026-04-15` · `Revisits: §4.0.1`

**Decision.** The kernel is structured hexagonally: a domain core (pure types, invariants, pure functions) surrounded by ports (trait definitions for kernel-external dependencies) implemented by an adapter ring (concrete adapters for HTTP, stdio, mTLS, MCP, ACP, providers, persistence, secrets).

**Rationale.** Hexagonal gives multi-adapter-per-port flexibility (swap SQLite for Postgres without touching domain logic), testability (every port has a mock adapter), and keeps the domain core small. Clean Architecture's call-direction discipline does not fit a runtime kernel where the kernel calls into Spirit ABI traits as part of its control flow.

**Alternatives considered.** Clean Architecture (rejected: call-direction discipline contradicts the kernel-calls-into-Spirit-ABI inversion of control). Layered (rejected: less flexible for adapter-per-port).

**What would force a revisit.** A subsystem emerges where hexagonal's port abstraction is more friction than value.

## ADR-011 — Actor model on the runtime hot path

`Status: binding-v0.1` · `Gate: per-Spirit Tokio task supervision + bounded mailbox` · `Decided: 2026-04-15` · `Revisits: §4.0.1`

**Decision.** Each Spirit is a Tokio-supervised actor with a bounded mailbox; no shared mutable state between Spirit actors. The seven kernel services are not actors — they are shared services with their own task pools.

**Rationale.** Four properties for free: backpressure via bounded mailboxes, no locks on the Spirit-to-Spirit hot path, failure isolation via Tokio task supervision, and natural hot-swap (replace `behavior` while preserving `state` and `open_tokens`). Codex's `AgentRegistry` + `Mailbox` is the precedent.

**Alternatives considered.** Shared-memory state (rejected: violates I5 and complicates hot-swap). Channel-only architecture without supervisors (rejected: failure handling becomes ad-hoc).

**What would force a revisit.** Tokio's supervisor primitives change materially.

## ADR-012 — Typed-intent A2A consent

`Status: binding-v0.9` · `Gate: A2A Gateway rejects frames with intent not in send-allowlist or accept-allowlist` · `Decided: 2026-04-15` · `Revisits: §7.2, §13 v0.9 row`

**Decision.** Cross-Host A2A consent is `(peer-identity, intent-class)`, not `(peer-identity)`. A read-only Spirit cannot pass a payload to a writeable Spirit that, when interpreted, causes a write the read-only Spirit was forbidden from.

**Rationale.** Channel-consent does not imply transaction-consent. The confused-deputy class of attacks at the inter-Spirit boundary requires intent-class scoping. Mira's `diagnosis-handoff:read-only-evidence` is admissible at Nash; `code-mutation-directive` is rejected. Without ADR-012, Mira could trigger a Nash-side action she cannot trigger directly.

**Alternatives considered.** Channel-consent only (rejected: leaves the confused-deputy gap open). Typed-intent at the IAC bus layer for ALL frames (considered: more uniform; rejected because cross-Host frames are where the trust boundary actually is, and same-Host IAC frames already inherit the kernel's process-internal trust).

**What would force a revisit.** A workload pattern emerges where intent-class cardinality grows pathologically.

## ADR-013 — Two-level `task.assign` typed-intent IAC primitive

`Status: binding-v0.9` · `Gate: founder-loop epic-7 reproducible end-to-end` · `Decided: 2026-04-15` · `Revisits: §10.4, §13 v0.9 row`

**Decision.** `task.assign` is a typed IAC frame with two levels of granularity: human → Orchestrator at epic granularity (the founder loop entry point); Orchestrator → Worker at story granularity (the Orchestrator decomposes the epic into stories and dispatches to Workers). Same primitive, different topology.

**Rationale.** The founder loop wedge demo requires both levels — the founder dispatches an epic; the Orchestrator dispatches stories within. A single primitive at both levels means the kernel mediates uniformly and the auditor walks one frame type.

**Alternatives considered.** Separate primitives per level (rejected: kernel surface inflation without gain).

**What would force a revisit.** A use case emerges that needs a third level.

## ADR-014 — Distillation audit-chain (introduces I11)

`Status: binding-v0.5` · `Gate: Capability Registry rejects digest writes with EDigestAuditChainMissing; segment-level granularity by default` · `Decided: 2026-04-15` · `Revisits: §3.2 invariant I11; §9.5`

**Decision.** Add invariant I11. Every payload tagged `kind: digest` written to private/shared/collective memory carries non-empty `source_log_ref` (transitively flattened to original raw frames) and `distillation_depth`. Kernel rejects malformed writes with `EDigestAuditChainMissing`. Segment-level granularity is the default contractual unit; write-level audit is opt-in for forensic Spirits via manifest declaration.

**Rationale.** Distillation is a substrate-level pattern. Without an audit chain back to raw, the Transparency Log becomes ceremonial. Segment granularity keeps the audit path through 10K-writes/sec workloads without saturating fsync cadence.

**Alternatives considered.** Per-write audit by default (rejected: 10K writes/sec workloads stall on CAS contention). No audit chain (rejected: defeats the point of the Transparency Log).

**What would force a revisit.** A Spirit class needs forensic granularity by default and the segment-level option becomes too coarse.

## ADR-015 — Decision-context recording (introduces I12)

`Status: binding-v0.5` · `Gate: shadow-recall record on event/inbound; refs attached on decision.* emit` · `Decided: 2026-04-15` · `Revisits: §3.2 invariant I12`

**Decision.** Add invariant I12. When a Spirit emits a `decision.*` frame, the kernel attaches `working_memory_digest_refs` populated from the Spirit's declared in-context digests AND from any raw frames delivered via inbound events (kernel writes a shadow-recall record before invoking the Spirit's `event/inbound` handler).

**Rationale.** Closes the gap where the digest hid the critical finding → the agent never recalled raw → audit shows raw existed but the agent never reasoned over it. Without I12, audit can prove what raw + what digest, but not what the agent actually saw at decision time.

**Alternatives considered.** Track in-context digest set without inbound-event shadow-recall (rejected: leaves the audit gap open for raw frames delivered via push, not pull). Track every byte of the LLM context window (rejected: requires kernel introspection of LLM-internal state).

**What would force a revisit.** A Spirit class operates on raw frames in working memory without ever calling `log.recall` and without inbound delivery (e.g., reading from a private memory cache populated outside the audit chain).

## ADR-016 — Token-budget accounting

`Status: binding-v0.9` · `Gate: typed frames emit at 80%/95%; new tool calls fail above 100%` · `Decided: 2026-04-15` · `Revisits: §4.6`

**Decision.** The kernel's Capability Registry tracks per-Spirit `context_window_size`, `context_used`, `context_pressure_threshold`. Soft threshold (default 80%) emits typed `ContextPressure` IAC frame; hard threshold (default 95%) emits `ContextLimit`; above 100% the kernel returns `EContextExhausted` on new tool calls.

**Rationale.** Context tokens are agent-infrastructure's analog of provider-billed resources. Spirits need to know they are approaching limits before they hit them; the kernel surfaces the signal so the Spirit's persona logic decides whether to distill, hand off, or halt.

**Alternatives considered.** Provider-side rate limiting only (rejected: providers do not surface per-Spirit context state). Token counting in the kernel (rejected: model-specific, requires kernel to interpret provider configurations).

**What would force a revisit.** Token-counting becomes provider-uniform and the kernel can take it on without violating I9.

## ADR-017 — Hot-swap state-transfer wire format

`Status: binding-v0.3` · `Gate: swap conformance corpus passes; auto-revert ≤30s on post-swap invariant violation` · `Decided: 2026-04-15` · `Revisits: §3.2 invariant I6; §4.1`

**Decision.** Hot-swap state transfer uses CBOR-encoded payloads conforming to a per-Spirit-class schema declared in the manifest (`[hot_swap].state_schema_uri` + `state_schema_version`). The kernel rejects swap operations where the predecessor's schema version is not declared compatible by the successor's manifest. Compatibility rules: same major + additive forward = forward-compat; same major + breaking = forbidden (use major bump); cross-major requires explicit migrator. The Hot-Swap Coordinator implements saga-style compensating transactions: on `on_swap_out` failure, the kernel restores the predecessor; on `on_swap_in` failure, it discards the successor and restores the predecessor with original tokens; on post-swap invariant violation, it auto-reverts within 30s.

**Rationale.** Hot-swap correctness depends on predecessor and successor agreeing on state-blob meaning. CBOR + per-class schema gives typed encoding, compactness, language-neutrality, and a kernel-mediated compatibility check. Saga rollback closes the "what if the swap itself fails" gap.

**Alternatives considered.** Untyped opaque blob (rejected: makes hot-swap a demo trick). serde-json without schema (rejected: textual JSON fails forward-compat silently). Single transaction without rollback (rejected: leaves operators with broken state on swap failure).

**What would force a revisit.** A Spirit-class evolution pattern emerges that CBOR + schema does not cover (e.g., embedded streaming-state with seek points).

## ADR-018 — Intent provenance preservation across distillation (introduces I13)

`Status: binding-v0.5` · `Gate: kernel-computed intent_lineage; consumer admission rejects with EIntentPromotionDenied` · `Decided: 2026-04-15` · `Revisits: §3.2 invariant I13; §9.5`

**Decision.** Add invariant I13. The kernel computes `intent_lineage` from input frame_ids on every digest write — the union of intent classes of all input frames the digest was distilled from. A consumer that operates under intent `Y` rejects digests whose `intent_lineage` is not contained in `allowed-promotion-set(Y)` declared in the consuming Spirit's manifest. Producer-side enforcement is kernel-computed (not Spirit-self-reported).

**Rationale.** Closes consent-laundering through distillation: data received under `consult` cannot be silently re-purposed under `delegate` via a digest hop. Kernel-computed (not Spirit-self-reported) closes the asymmetric-enforcement gap.

**Alternatives considered.** Make intent_lineage advisory (rejected: makes I13 advisory; consent laundering becomes silent the moment one Spirit forgets to propagate). Track intent_lineage at the IAC bus layer for ALL frames, not just digests (considered: more uniform, but explodes header overhead for frames that never cross consent boundaries).

**What would force a revisit.** A workload pattern emerges where intent_lineage cardinality grows pathologically.

## ADR-019 — Halt continuity across hot-swap (introduces I14)

`Status: binding-v0.9` · `Gate: kernel refuses swap with EHaltContinuityViolation if drain or schema-compatible migration absent` · `Decided: 2026-04-15` · `Revisits: §3.2 invariant I14`

**Decision.** Add invariant I14. When a Spirit with non-empty `halt_set` is hot-swapped, either every halt is drained (resolved before swap) OR every halt is migrated to the successor with full resolution-path state, AND the successor's manifest declares `halt_protocol_compatibility = N` (matching the predecessor's halt-protocol version registered in `halt-registry/<spirit-class>.toml`). Kernel refuses the swap with `EHaltContinuityViolation` otherwise.

**Rationale.** An in-flight halt represents a user's open question; if hot-swap silently drops it, the substrate's halt-resolution-path-completeness claim collapses.

**Alternatives considered.** Always drain before swap (rejected: forces operators to wait on user resolution before urgent kernel updates). Drop halts on swap (rejected: breaks the user trust contract).

**What would force a revisit.** A Spirit class adopts a halt protocol that does not version cleanly.

## ADR-020 — Hot-swap migration policy

`Status: binding-v0.5` · `Gate: kernel refuses load with EMigratorMissing if predecessor archive exists and migrator absent` · `Decided: 2026-04-15` · `Revisits: §4.1`

**Decision.** Cross-major hot-swap with persistent state requires a `migrate(predecessor_state) -> Result<successor_state, Error>` entry point declared in the successor's manifest's `migrates_from` field. Kernel refuses load with `EMigratorMissing` if predecessor archive exists and no migrator is declared. Predecessor's historical journal stays in cold storage, addressed by `(class, version, instance_id)`.

**Rationale.** Cross-major migration with persistent state is a known graveyard pattern. Forcing Spirit authors to declare migration intent and provide the entry point makes the kernel's contribution structural (allow-list check, migrator presence check); migration logic itself is Spirit-author concern.

**Alternatives considered.** Implicit migration (rejected: silent state corruption). No cross-major hot-swap (rejected: forces full restarts on every breaking change).

**What would force a revisit.** Migration-authoring proves operationally infeasible for typical Spirit classes.

## ADR-021 — CliWrapperSpirit output-shape adapter contract

`Status: binding-v0.9` · `Gate: startup EOutputShapeAdapterMismatch if observed shape ≠ declared version` · `Decided: 2026-04-15` · `Revisits: §6.6, §13 v0.9 row`

**Decision.** CLI-wrapper Spirits use the kernel-builtin `CliWrapperSpirit` class with declared `output_shape_version`. The kernel asserts on startup; refuses to start with typed `EOutputShapeAdapterMismatch` if observed shape does not match declared version. Wrappers cannot fall back to "best-effort parsing" on shape mismatch — fail-loud.

**Rationale.** Audit drift is the failure mode the substrate cannot tolerate. The founder loop's CLI-wrapper Spirits (Orchestrator + Workers) speak the wrapped CLI's output format; if the CLI's output format drifts (CLI upgrade), the kernel must catch it at startup, not after a corrupted IAC frame lands in the Transparency Log.

**Alternatives considered.** Best-effort parsing with logged warnings (rejected: corrupts audit trail). Per-CLI native Rust wrapper crate (rejected: forces forking the wrapped CLI's release cycle into the MAOS team).

**What would force a revisit.** A wrapped CLI's output format becomes versionless (i.e., changes within minor releases without an explicit version field).

## ADR-022 — Tagged-scalar working-memory slot with epistemic-policy binding

`Status: binding-v0.3` · `Gate: [epistemic_policy] rules trigger halts via four universal-arithmetic predicates` · `Decided: 2026-04-15` · `Revisits: §4.0.7, §4.6.1, §6.1, §6.2, §6.3`

**Decision.** Spirits write working-memory tagged scalars via `working_memory.set_scalar(tag, value, derived_from)`. The kernel persists and routes tagged scalars by tag identity without interpreting tag-specific semantics. Kernel performs only universal-arithmetic comparison via four predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`). Spirit's `[epistemic_policy]` rules reference tagged scalars via these predicates; kernel triggers halts when predicates fire and journals halt reason with structured payload (tag, value, threshold, policy_id, derived_from).

**Rationale.** The tagged-scalar slot is the smallest theater-side primitive that lets the actor's epistemic state become legible to the kernel's halt mechanism without the kernel knowing what the actor is reasoning about. Theater-side primitive: minimal — one typed slot, two APIs, four universal-arithmetic predicate forms. Actor-side responsibility: total — Spirit decides what to track (variance, entropy, ensemble disagreement, KL, EFE, custom proxy), how to compute it, when to update it.

**Alternatives considered.** Kernel computes Spirit-specific scalars (rejected: violates §4.0.7 — kernel does no Spirit-specific cognitive computation). Spirit-author declares custom predicate functions (rejected: opens the kernel surface to arbitrary code execution).

**What would force a revisit.** A Spirit class needs to compare two scalars to each other rather than a scalar to a constant. (At that point: extend the predicate vocabulary additively, not redesign.)

## ADR-023 — Capability-token TTL + bind-to-PID

`Status: binding-v0.1` · `Gate: TTL ≤60s for high-privilege; tokens bound to (Spirit-PID + boot-nonce + expiry); TOCTOU re-validation at use` · `Decided: 2026-04-15` · `Revisits: §4.3.4, §0.6 commitment 6`

**Status correction.** ADR-023 was previously tagged `binding-v1.5`. The token-binding mechanism (PID + boot-nonce + expiry, ed25519-signed) is required from v0.1 onward — without it, the Capability Token surface (ADR-030) has a replay vulnerability across Spirit restarts, which v0.1's Capability Registry mediation invariant (I1) cannot tolerate. The mechanism is implementation detail of v0.1's foundational commitment 6 (§0.6).

**Decision.** Capability-token TTL ≤60s for high-privilege operations. Tokens bound to (Spirit-PID + boot-nonce + expiry); audit-logged at every use with origin-Spirit-ID. Re-validation at use against current state, not cached state (TOCTOU correctness).

**Rationale.** Long-lived tokens are a replay-attack surface. Short TTL + PID binding makes token theft useless across process boundaries. Re-validation at use ensures posture changes during the token's lifetime are honored.

**Alternatives considered.** Long-lived tokens with revocation lists (rejected: revocation propagation latency too high). No expiry (rejected: replay-attack surface).

**What would force a revisit.** A workload pattern emerges where 60s TTL is too short for the task's natural duration.

## ADR-024 — Spirit-authored skills

`Status: binding-v0.9` · `Gate: skill.author.self capability + operator-admission queue operational` · `Decided: 2026-04-15` · `Revisits: §13 v0.9 row`

**Decision.** Spirits may author skills (markdown with TOML frontmatter conforming to `maos.skill.v1`) and either ship them in the Spirit's package or write them dynamically at runtime via the `skill.author.self` capability scope. New skills land in pending state pending operator admission. Operator-admission queue handles the pending state.

**Rationale.** A Spirit author may want to ship a Spirit-authored-skills-loop (the Spirit improves its own skill library based on user feedback). The kernel's contribution: a registry mechanism for skills + a pending-admission queue + audit on every skill admission.

**Alternatives considered.** No Spirit-authored skills (rejected: forecloses self-improving-loop patterns). Auto-admission of Spirit-authored skills (rejected: cargo-culting risk; LLM-generated skills entering the library without operator review is a known failure mode).

**What would force a revisit.** Spirit-authored skills accumulate at scale and operators want kernel-mediated bulk admission.

## ADR-025 — Proactive scheduling

`Status: binding-v0.3` · `Gate: Butler on_idle Sandra-scene replay; on_schedule fires at declared cadence` · `Decided: 2026-04-15` · `Revisits: §5.3, §6.1`

**Decision.** Spirits may declare scheduled invocations via manifest `[schedule]` table; kernel fires `on_schedule(ctx, schedule_id, payload)` at declared cadence with rate-limit, ComplianceClaim-stamp, principal-revocability, and side-effect allowlist.

**Rationale.** Butler's anticipatory loop, Mira's periodic-health-check pattern, Researcher's daily-arXiv-watch — all need scheduled invocations beyond the user's explicit request. The kernel provides the scheduling primitive; Spirits decide the semantic.

**Alternatives considered.** Spirits self-schedule via internal timers (rejected: kernel-mediated scheduling lets the operator surface scheduled work in audit and lets the kernel rate-limit). Cron-style external scheduling (rejected: violates I1 capability mediation).

**What would force a revisit.** Scheduled invocations require sub-second cadence (the kernel's tick is currently ≥1s).

## ADR-026 — Principal Memory Namespace with redaction-aware operations

`Status: binding-v0.5` · `Gate: subject-access query / right-to-be-forgotten / redaction-on-export operate on principal:* namespace` · `Decided: 2026-04-15` · `Revisits: §4.2`

**Decision.** The kernel adds a typed namespace within the existing private-tier memory: `principal:<principal_id>:<spirit-author-defined-schema>`. Writes to this namespace are tagged as principal-related data and inherit three kernel-mediated operations: subject-access query, right-to-be-forgotten, redaction-on-export. The kernel does NOT interpret principal-namespace content; schema is entirely Spirit-author-declared.

**Rationale.** Privacy-aware Spirits (Butler watching the user's calendar; Researcher accumulating per-author bibliographies) need a namespace where principal data inherits the three operations. Without this primitive, every Spirit author would re-invent principal-aware curation.

**Prior art.** The principal-scoped memory model is informed by hermes-agent's principal-namespaced memory pattern lifted into a kernel-allocated contract. Hermes-as-application demonstrated the operational shape; MAOS lifts it into a kernel primitive so the substrate can offer the contract uniformly to any Spirit-author.

**Alternatives considered.** Spirit-author-handled principal scope (rejected: every Spirit re-invents the wheel). Dedicated principal-store as a new memory tier (rejected: tier inflation; the existing private tier suffices with the namespace tag).

**What would force a revisit.** A workload pattern emerges where the three operations are insufficient and a fourth is needed.

## ADR-027 — Skill-package external-standard interop

`Status: binding-v0.5` · `Gate: Spirit-form adapter loads ≥1 third-party skill format` · `Decided: 2026-04-15` · `Revisits: §13 v0.5 row`

**Decision.** Skills are markdown with TOML frontmatter conforming to `maos.skill.v1`. The format is intentionally close to (but distinct from) the Anthropic Skills format and similar emerging conventions. A Spirit-form adapter can load at least one third-party skill format without kernel modification.

**Rationale.** Skill ecosystems are converging across vendors. The substrate supports the convergence by making `maos.skill.v1` close to the dominant external standards while retaining the kernel-mediated admission flow.

**Alternatives considered.** Adopt a third-party skill format wholesale (rejected: gives up control over admission semantics). Define a wholly novel format (rejected: forces every author to re-learn).

**What would force a revisit.** A dominant skill format emerges that `maos.skill.v1` cannot interop with cleanly.

## ADR-028 — Replay determinism primitive

`Status: binding-v1.0` · `Gate: trace-shape contract validated in CI per schemas/trace-shape.schema.json; v1.0 best-effort, v1.5 hard target` · `Decided: 2026-04-15` · `Revisits: §7.3`

**Decision.** Replay determinism is over the **shape of the trace** (IAC frame ordering, capability-token issuances, halt events, decision-frame emission), NOT over redacted payload content. Redacted slots replay as `<REDACTED:type=<class>, len=<bytes>, hash=<sha256-prefix>>` placeholders carrying the same structural shape. The trace-shape contract is specified in `schemas/trace-shape.schema.json` (JSON Schema draft-2020-12); the schema is validated in CI on every kernel commit.

**Rationale.** Replay-vs-redaction is an architectural tension: bit-exact replay makes the Transparency Log a forensic record; full redaction makes the log privacy-respecting. The shape-of-trace compromise satisfies both — the shape replays bit-exact (auditor can verify ordering, frame types, halts), the content redacts cleanly. The WAL+MVCC pattern from databases applied to the agent-runtime layer.

**Alternatives considered.** Bit-exact replay including payload (rejected: inconsistent with right-to-be-forgotten). No replay (rejected: defeats the audit story).

**What would force a revisit.** A regulatory regime emerges requiring bit-exact payload replay regardless of redaction.

## ADR-029 — Provider/CLI Gateway sub-module contract

`Status: binding-v1.0` · `Gate: gateway sub-modules registered via gateway.toml; per-FR54 conformance` · `Decided: 2026-04-15` · `Revisits: §13 v1.0 row`

**Decision.** Provider and CLI gateway sub-modules (FR54) are first-class crates implementing the `GatewaySubmodule` trait — `auth`, `model_capabilities`, `stream_translate`, `halt_lift`. No direct kernel coupling; gateway sub-modules registered via `gateway.toml`. Schema specified in `schemas/gateway-submodule.schema.json`.

**Rationale.** A hermes-class tenant needs a gateway abstraction that mediates between Spirits and external-CLI subprocesses with revocation, rate-limit, secret-redaction, and per-subprocess capability scoping. ADR-021 covers fail-loud parsing; ADR-029 covers the gateway-side contract.

**Alternatives considered.** Direct kernel-side gateway (rejected: violates kernel-stays-small). Spirit-side gateway (rejected: leaves the secret-redaction boundary inside Spirit code).

**What would force a revisit.** A gateway-able external surface emerges that the trait's four method shapes cannot represent.

## ADR-030 — Capability Registry decomposition

`Status: binding-v0.1` · `Gate: hot-path token verify <5µs P99 benchmark` · `Decided: 2026-04-15` · `Revisits: §4.6`

**Decision.** The Capability Registry is internally split into four sub-services: `cap-tokens` (hot path, lock-free token issue/verify), `cap-policy` (consent rules + intent allowlist), `cap-audit` (transparency log writer, slow path), `cap-quota` (per-Spirit budget tracking). IAC traverses only `cap-tokens` on the hot path; the audit/lineage path is async via bounded MPSC.

**Rationale.** A monolithic Capability Registry is a god-service. Decomposing it preserves the Capability Registry as a single mediation surface from the Spirit-API perspective while internally separating the hot path from the slow path so audit writes do not block frame delivery.

**Alternatives considered.** Monolithic Capability Registry (rejected: serializes IAC hot path). Per-Spirit Capability Registry instances (rejected: cross-Spirit mediation becomes ad-hoc).

**What would force a revisit.** A new capability surface emerges that does not fit into the four sub-service shapes.

## ADR-031 — Cross-Form Spirit Equivalence

`Status: speculative-vNext` · `Gate: ≥90% on 200-scenario class corpus when both forms exist` · `Resolves-by: ADR-002 measurement gate triggering rust-inproc unlock + superseding ADR` · `Decided: 2026-04-15` · `Revisits: ADR-002, §13 measurement gate`

**Decision.** rust-inproc and subprocess Spirits MUST pass an identical conformance suite (`spirit-conformance` crate). Form is a deployment knob, not a semantic one. Cross-form Semantic equivalence floor: ≥90% on a 200-scenario class corpus. CLI-wrapper requires distributional behavioral equivalence (Mann-Whitney U-test p > 0.05 over 30 runs per scenario).

**Rationale.** Spirit authors must be able to develop in rust-inproc and ship as subprocess (or vice versa) without behavior drift. The conformance suite enforces this empirically.

**Alternatives considered.** Allow per-form behavior divergence (rejected: defeats the form-portability claim).

**What would force a revisit.** A Spirit class emerges where the two forms cannot match (e.g., rust-inproc-only filesystem semantics).

## ADR-032 — Spirit Wire Protocol bytes-on-wire

`Status: binding-v0.1` · `Gate: byte-equal golden corpus per frame variant per SDK; 168h cumulative pre-GA fuzz floor` · `Decided: 2026-04-15` · `Revisits: §5.2`

**Decision.** LSP-style `Content-Length` framing over stdout: `Content-Length: <decimal>\r\n\r\n` followed by exactly N bytes of CBOR-encoded payload. Header is ASCII, case-insensitive name, max header block 4 KiB. Stderr reserved for diagnostics; never multiplexed onto stdout. EOF after a clean frame = `Halt::Voluntary`; mid-frame EOF = `Halt::Fault(Truncated)`. Backpressure via credit-based windowing on bounded `mpsc<Frame>(64)`.

**Rationale.** LSP framing is well-understood and implementations are abundant. CBOR is compact, language-neutral, schema-evolved. The framing details are spelled out so subprocess implementations across languages produce byte-equal output.

**Alternatives considered.** Newline-delimited JSON (rejected: large payloads break easily on partial newline encoding). Raw JSON-RPC without length prefix (rejected: parser ambiguity on partial frames).

**What would force a revisit.** A use case emerges where Content-Length framing cannot represent the message structure cleanly.

## ADR-033 — Subprocess supervision and halt-crash intersection

`Status: binding-v0.3` · `Gate: crash-recovery corpus: torn-frame-at-tail truncate, mid-log fatal` · `Decided: 2026-04-15` · `Revisits: §4.1`

**Decision.** Defines the (open-halt × in-flight CBOR × SIGKILL) crash matrix. Supervisor reissues halt to successor only if CBOR snapshot is `committed`; otherwise halt is poisoned and surfaced to operator. Per-Spirit `Arc<RwLock<TokenLedger>>` lives kernel-side, not Spirit-side. On crash mid-CBOR-write: supervisor's `JoinSet` returns `Err`, supervisor calls `cap_registry.revoke_all(spirit_id)` synchronously, journal records `HaltRecord{cause: Fault, in_flight_tokens: [...]}`. Replay rule: torn frame at tail = truncate; torn frame mid-log = fatal corruption requiring manual recovery.

**Rationale.** The intersection of three independent mechanisms (subprocess form + hot-swap state-transfer + halt continuity) at the moment a subprocess Spirit dies is the most subtle correctness boundary in the architecture. Specifying it explicitly closes the "what happens when..." gap.

**Alternatives considered.** Treat crash + open halt as fatal (rejected: too restrictive). Allow successor to inherit crashed predecessor's halts unconditionally (rejected: silent successor-confusion events are exactly the failure mode I14 exists to prevent).

**What would force a revisit.** A Spirit class needs different crash semantics than the matrix supports.

## ADR-034 — Partial-consent failure semantics

`Status: binding-v0.9` · `Gate: sender receives ConsentRupture IAC frame on receiver-side rejection` · `Decided: 2026-04-15` · `Revisits: §4.5`

**Decision.** Sender-approved / receiver-rejected mid-frame becomes a `ConsentRupture` event; frame is quarantined, not delivered, not silently dropped. Sender receives `ConsentRupture` IAC frame; operator surface logs the rupture for forensic review.

**Rationale.** ADR-022's failure-semantics floor covers crash detection and `task.orphaned`; partial-consent failure is a third class of failure that needed explicit semantics.

**Alternatives considered.** Best-effort delivery with logged warning (rejected: violates I8). Reject the entire conversation on first rupture (rejected: too restrictive).

**What would force a revisit.** Partial-consent ruptures become common enough that the operator surface needs aggregation.

## ADR-035 — Observer scalar trajectory channel

`Status: binding-v0.5` · `Gate: Observer subscribers see pre-halt scalar drift in real time` · `Decided: 2026-04-15` · `Revisits: §4.7, §6.5`

**Decision.** A read-only `scalar.tap` stream emits every Spirit's `working_memory.set_scalar` write so Observer Spirits see pre-halt scalar trajectory, not just halt events.

**Rationale.** Mira-class diagnostic Spirits use pre-halt scalar drift as a diagnostic signal. Without scalar.tap visibility, Observer can describe halts but not the runup.

**Alternatives considered.** Telemetry stream emits scalar writes (considered: similar; rejected because telemetry stream already carries non-scalar events and conflating the two raises subscriber complexity).

**What would force a revisit.** Scalar writes become high-frequency enough that the dedicated channel saturates.

## ADR-036 — Hot-swap × halt continuity precondition check

`Status: binding-v0.9` · `Gate: maosctl swap surfaces precondition status before initiating` · `Decided: 2026-04-15` · `Revisits: ADR-019, §4.1`

**Decision.** `maosctl swap` precondition check: predecessor open-halts ⊆ successor accepted protocol versions per `halt-registry/<spirit-class>.toml`. Operator UX surfaces "predecessor has 3 open halts at protocol v2; successor accepts v2; safe" before initiating the swap.

**Rationale.** I14 enforcement at the kernel boundary prevents `EHaltContinuityViolation` at swap-time; ADR-036 surfaces the same check at the operator UX so operators see the safety status before triggering the swap.

**Alternatives considered.** Kernel-only enforcement (rejected: leaves operator without pre-flight visibility).

**What would force a revisit.** Halt-protocol versioning becomes finer-grained than registry-table allows.

## ADR-037 — Constitutional amendment process

`Status: binding-v0.1` · `Gate: invariant-lock CI gate runs on every PR touching I1–I14` · `Decided: 2026-04-15` · `Revisits: §3.2, §8.7`

**Decision.** ADRs touching invariants I1–I14 require two-reviewer + invariant-test diff; CI gate `invariant-lock` enforces. ADR amendments require: (a) machine-checkable diff against the invariant set, (b) a corpus delta showing the test surface that exercises the change, (c) a phase-commitment update.

**Rationale.** The constitutional commitment (Innovation #7 in PRD Step 6) requires architectural enforcement, not founder discipline. Without ADR-037, ADRs are markdown that one human can rewrite.

**Alternatives considered.** Process-only amendment (rejected: relies on founder discipline). External governance board (rejected: scope inflation).

**What would force a revisit.** The reviewer pool becomes too small for the two-reviewer requirement.

## ADR-038 — Per-service KLOC ceiling

`Status: binding-v0.1` · `Gate: xtask/kloc.toml enforced by tokei in CI; aggregate ≤20 KLOC, alarm at 16` · `Decided: 2026-04-15` · `Revisits: §4.0.4`

**Decision.** Kernel ≤20 KLOC trusted core enforced as the sum of per-crate ceilings. Per-crate budgets in `xtask/kloc.toml`: `maos-kernel-core ≤6 KLOC`, `maos-cap-registry ≤3 KLOC`, `maos-wire ≤2 KLOC`, `maos-journal ≤2 KLOC`, etc. Aggregate ≤20 KLOC, alarm at 16. CI gate via `tokei`.

**Rationale.** "Kernel stays small" needs structural enforcement, not memo discipline. Per-crate ceilings make the KLOC budget legible and machine-checked.

**Alternatives considered.** Aggregate-only ceiling (rejected: no early warning when one crate consumes the budget). No ceiling (rejected: erodes silently).

**What would force a revisit.** A new kernel surface justifies a ceiling extension; amendment via ADR-037 process.

## ADR-040 — Threat-model split: same-Host vs A2A

`Status: binding-v0.5` · `Gate: 200-scenario isolation corpus passes (Sec-14a at v0.9, Sec-14b at v1.0)` · `Decided: 2026-04-15` · `Revisits: §8.1`

**Decision.** NFR-Sec-14 (cross-Spirit memory isolation) splits into Sec-14a (same-Host: namespace, seccomp, capability tokens) and Sec-14b (A2A: mTLS, signed frames, replay window). Each has its own 200-scenario adversarial corpus.

**Rationale.** Same-Host attack vectors (one Spirit subvert another via shared filesystem, broadcast topic, or capability-token forgery) and cross-Host attack vectors (peer Host injecting false frames, certificate-pin attack, replay) are sufficiently different that separate corpora are needed.

**Alternatives considered.** Combined corpus (rejected: dilutes coverage of either attack class).

**What would force a revisit.** A third attack class emerges that does not fit either category.
