# Architecture Decision Records

Committed ADRs for the MAOS project.

| ADR | Title | Status | Gate |
|-----|-------|--------|------|
| [ADR-001](ADR-001-kernel-language-is-rust-tokio.md) | Kernel language is Rust + Tokio | binding-v0.1 | v0.1 ships in Rust + Tokio |
| [ADR-002](ADR-002-spirit-form-at-v01-subprocess-only-inproc-gated-on-measurement.md) | Spirit form at v0.1 — subprocess only, inproc gated on measurement | binding-v0.1; single-form clause superseded by ADR-031 | §13 measurement gate |
| [ADR-004](ADR-004-hexagonal-sandboxing-with-os-native-primitives.md) | Hexagonal sandboxing with OS-native primitives | binding-v0.1 | T0/T1 at v0.1; T2 at v0.3; T3 at v0.5 |
| [ADR-006](ADR-006-kernel-learns-no-patterns.md) | The kernel learns no patterns | binding-v0.1 | structural-state lint |
| [ADR-010](ADR-010-hexagonal-architecture-for-static-structure.md) | Hexagonal architecture for static structure | binding-v0.1 | crate boundary lint |
| [ADR-011](ADR-011-actor-model-on-the-runtime-hot-path.md) | Actor model on the runtime hot path | binding-v0.1 | per-Spirit Tokio task supervision |
| [ADR-012](ADR-012-typed-intent-a2a-consent.md) | Typed-intent A2A consent | binding-v0.1 | A2A Gateway rejects frames with intent not in allowlist (at v0.9) |
| [ADR-014](ADR-014-distillation-audit-chain.md) | Distillation audit-chain (introduces I11) | binding-v0.1 | Capability Registry rejects digest writes with EDigestAuditChainMissing (at v0.5) |
| [ADR-022](ADR-022-tagged-scalar-working-memory-slot.md) | Tagged-scalar working-memory slot | binding-v0.1 | [epistemic_policy] rules trigger halts via four universal-arithmetic predicates (at v0.3) |
| [ADR-023](ADR-023-capability-token-ttl-bind-to-pid.md) | Capability-token TTL + bind-to-PID | binding-v0.1 | TTL ≤60s; tokens bound to (Spirit-PID + boot-nonce + expiry) |
| [ADR-024](ADR-024-out-of-kernel-sandbox-escape-structural-detector.md) | Out-of-kernel sandbox-escape structural detector (NFR-Sec-3) | Accepted (arch 2026-06-29); binding-v2.0 at Story 11.4b | Story 11.4b — out-of-kernel detector (no `maos-kernel-core` dep); TP-floor/FP-ceiling on a live-syscall proven-red; no-verdict (structural-not-semantic) test; producer-wired proven-red |
| [ADR-026](ADR-026-principal-memory-namespace.md) | Principal Memory Namespace | binding-v0.1 | subject-access query / right-to-be-forgotten (at v0.5) |
| [ADR-028](ADR-028-replay-determinism-trace-shape.md) | Replay determinism over trace-shape | binding-v1.0 | `replay_byte_identical_two_process` + `redaction_k_anonymity` CI gate |
| [ADR-030](ADR-030-capability-registry-decomposition.md) | Capability Registry decomposition | binding-v0.1 | hot-path token verify <5µs P99 benchmark |
| [ADR-031](ADR-031-wasm-component-model-spirit-form.md) | WASM Component-Model Spirit form (host-as-adapter; resolves Cross-Form Equivalence) | binding-v2.0 | Story 11.1a host+WIT (kernel-core HARD 0 via check-kernel-baseline; NEW maos-host public-API baseline via check-host-surface; WIT byte-equal corpus vs ADR-032 frame set); Story 11.1b `check-wasm-form-equiv` tiered cross-form equivalence GREEN (blocking at v2.0); `check-cross-form-equiv` relabeled CLI-wrapper distributional advisory |
| [ADR-032](ADR-032-spirit-wire-protocol-bytes-on-wire.md) | Spirit Wire Protocol bytes-on-wire | binding-v0.1 | byte-equal golden corpus per frame variant per SDK |
| [ADR-037](ADR-037-constitutional-amendment-process.md) | Constitutional amendment process | binding-v0.1 | invariant-lock CI gate |
| [ADR-038](ADR-038-per-service-kloc-ceiling.md) | Per-service KLOC ceiling | binding-v0.1 | xtask/kloc.toml enforced by tokei |
| [ADR-039](ADR-039-per-module-unsafe-code-policy.md) | Per-module `#![forbid(unsafe_code)]` policy | binding-v0.1 | `xtask check-unsafe` + `xtask/unsafe-allowlist.toml` |
| [ADR-040](ADR-040-rust-inproc-measurement-gate-v05-decision.md) | §13.1 rust-inproc measurement gate — v0.5 decision | binding-v0.5; superseded by ADR-031 | `xtask check-adr-040-accepted` + `crates/maos-bench/` |
| [ADR-041](ADR-041-phase-3-4-kernel-core-extraction-via-port-traits.md) | Phase 3/4 `maos-kernel-core` extraction via port traits | binding-v0.7 | `xtask/kloc.toml [in_progress_decomposition]` + `xtask check-service-boundary` P1 per extracted crate |
| [ADR-045](ADR-045-governance-audit-artifacts.md) | Governance audit artifacts (FR62) | binding-v0.5 (Story 9.3b Task 0) | abi-diff⊆ratified one-directional reconciliation (3-test) + `--kind governance` completeness round-trip; frozen-`Claim` regression |
| [ADR-046](ADR-046-cost-attribution-and-reconciliation.md) | Cost attribution + reconciliation (FR64) | binding-v0.5 (Story 9.3b Task 0) | observability-not-invoice posture; CI golden-vector oracle (no `f64`/no pricing-fn import, sum-then-round); SR-3 forget-cascade coverage; kernel re-pin ~21400–21440 |
| [ADR-047](ADR-047-trust-anchor-framing-carry-forward.md) | Trust-anchor framing carry-forward (NFR-Ops-8) | binding-v0.3 (Story 9.5a) | STABILITY.md NFR-Comp-3 scope references this ADR; no runtime gate (framing decision) |
| [ADR-048](ADR-048-doc-site-toolchain-docusaurus.md) | Doc-site toolchain — Docusaurus (NFR-Doc-7) | binding-v1.0 (Story 9.5) | `docs-site` builds for all locales; isolation contract in `xtask/kloc.toml`; frozen URL contract via redirects plugin |
| [ADR-049](ADR-049-cross-region-collective-memory-consensus.md) | Cross-region convergent replication with enforced re-attestation — TL-anchored CRDT (formerly "consensus") | binding-v2.0 (Story 11.2a; **operational envelope extended 11.2b**) | Story 11.2a — mediated re-attestation of KV rows via `canonical_kv_leaf` + region-signed `CrossRegionReplicationBundle` (no transparent replication, NOT TL-frame-crossing; "TL-anchored" = the 9.4b-derived signing key); independent per-region convergence oracle over collective_memory rows; region-identity reflex; dropped-audit-orphan surfaced; AP-local-degrade proven-red; kernel-core = one bounded `WriteEntryPoint::CrossRegionReadmit` re-pin 22964→~22980 (`maos-loom-lite` + `maos-audit`). **Story 11.2b** — operational-envelope extension: the in-tree CI-bound portion (3-region ≥10-agent convergence + single-clock A→B→A round-trip SLO machinery + loopback floor + **fail-closed region-identity on the LIVE read path** via `LoomLiteStore::region_guard`, ZERO kernel-Δ) binds via the NEW `check-multi-region-slo` gate (per-leg independence); the geo-operational portion (absolute geo-SLO + sustained live operation + 30-day soak) stays DEFERRED to the release-gate pilot artifact. |

> 16 `binding-v0.1` ADRs as of Story 1a.1; **15 as of Story 1b.6** (ADR-039 — per-module unsafe policy, accepts the 1b.3 relaxation).
> **17 as of Story 5.5e** (ADR-040 — §13.1 rust-inproc measurement gate, binding-v0.5).
> ADR-024 (out-of-kernel escape detector) and ADR-031 (WASM component-model form)
> were resolved from `speculative-vNext` in the Epic 11 v1.5 hold-window (2026-06-29):
> ADR-024 architecture-accepted (binding-v2.0 at Story 11.4b), ADR-031 binding-v2.0
> after Story 11.1b `check-wasm-form-equiv` GREEN.
> The remaining `speculative-vNext` and post-v0.1 ADRs (ADR-008, 009, 014 [runtime],
> 015, 016–021, 025, 027–029, 033–036) are tracked in
> `architecture-maos-minimal-opus/12-architecture-decision-records.md` and land
> at their respective phase epics.
