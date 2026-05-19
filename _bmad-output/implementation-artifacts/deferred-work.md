# Deferred Work

## Deferred from: code review of 3-2-manage-director-posture-with-a-halt-policy-schema-and-bounded-shift-propagation (2026-05-17)

- `shift_posture` TOCTOU race — concurrent shifts on different spirits can lose updates via the read-clone-modify-store sequence on `ArcSwap<PolicyTableInner>`. Pre-existing CoW pattern limitation shared by all `PolicyTable` mutations including `manifest_scopes`. Would require CAS loop or mutex. Not caused by Story 3.2 specifically.
- Malformed fixtures cover only 1 failure mode each — `malformed-rejected/rules.toml` only tests out-of-range threshold, `malformed-rejected/default_action.toml` only tests unknown variant. The inline unit tests cover empty tag, whitespace tag, duplicate tag, and negative threshold. NFR-Test-13 walker only checks file existence.

## Deferred from: code review of 3-3-directors-halt-resolution-ux-decision-audit-i12 (2026-05-18)

- `MockHaltResolver` uses `.unwrap()` on `std::sync::Mutex::lock` at `resolver.rs` — can panic under concurrent use if a thread panics while holding the lock. Test-only struct, pre-existing pattern in the codebase (same unwrap-on-mutex pattern used elsewhere in test doubles). Low risk: test scenarios are single-threaded in practice.
- `HaltResolver` trait + `ResolveError` placed in `maos-domain::halt` instead of spec-required `maos-kernel-core::halt::resolver` to avoid circular dependency (kernel-core ↔ director-surface via NotificationDispatcher). Re-exported from kernel-core; public API surface preserved. Dev record documents rationale. Documented design decision, not a regression.
- Re-export set in `halt/mod.rs` differs from spec (`pub use resolver::{HaltResolver, MockHaltResolver, ResolveError}` vs split sources + extra `FailingHaltResolver`). Follows from trait relocation to maos-domain.
- `halt_ui.rs::tests` defines local `TestResolver`, `FailingResolver`, `CaptureChannel` instead of reusing canonical implementations from `approval_prompt_e2e.rs` — forced by circular dep (can't import `MockHaltResolver` from kernel-core into director-surface). Spec's "reuse, don't reinvent" principle violated by architectural constraint.
- Production binary wires `MockHaltResolver` at `main.rs` — spec-acknowledged v0.3-β bootstrap. Story 4.1 will swap for real `KernelHaltResolver`. No compile-time guard.
- Distinct-table assertion in `halt_resolution_journaled.rs` uses string search on `payload_redacted` bytes instead of SQL `SELECT COUNT(*) FROM transparency_log` per spec. Weaker verification but proves conceptual boundary.
- `EpistemicHaltPayload` pub fields allow bypassing NaN rejection via direct struct construction. Follows crate-wide public-field convention. Low risk: `new()` is the recommended constructor.

## Deferred from: code review of 3-4-buffer-orchestrator-instructions-and-honor-director-pause-resume-revoke-p99-2s (2026-05-18)

- u64 → i64 cast in SQL params for timestamps — pre-existing SQLite limitation. Practical timestamps won't exceed `i64::MAX` for centuries. Consistent with existing `AuditFilter` pattern.
- `NotificationEvent::AnomalyFlagged` public fields allow bypassing constructor validation (NaN/empty checks). Follows crate-wide pub-field convention. `anomaly_flagged()` constructor is the recommended path.
- `OrchestratorBuffer::with_capacity(0)` creates permanently-full buffer with no minimum guard. `new()` hardcodes 32. Not caused by this change; edge case.
- TransparencyLog entries always have `spirit_id: None` — pre-existing schema limitation. The log schema doesn't carry per-row spirit ownership.

## Deferred from: code review of 4-1-halt-protocol-mechanism-three-resolution-kinds-halt-receipt-99-9-single-halt-owner (2026-05-19)

- `drain_for_spirit` ignores `spirit_pid`, drains all halts globally — v0.3-β placeholder, Story 5.3 refines with per-Spirit filtering.
- `ProvidedContext` resolution arm is a no-op — intended placeholder, Story 4.3 wires the actual working-memory write.
  **Closed by Story 4.3 — `KernelHaltResolver::resolve::ProvidedContext` writes to private memory + publishes `halt.context_provided` marker scalar.**
- `simulate_predicate` handles only 2 of 4 universal-arithmetic predicates — `on_value_within` and `on_value_outside` fall through to silent no-op, remaining predicates land in Story 4.2.
  **Closed by Story 4.2 — `simulate_predicate` now dispatches to all four predicates (halt_recall_floor.rs).**
- `HaltCorpus` and `TerminationCorpus` loaders are structural copy-paste — refactor to shared `CorpusLoader<T>` when bandwidth allows.
- Termination corpus mechanically generated via `xtask/src/gen_termination_corpus.rs`, not hand-authored — deferred to Story 4.5 per spec contract (HSIS 100 scenarios).
- Test PID collision risk (`seed % 1000`) — harmless now since `drain_for_spirit` drains all, but will break silently when Story 5.3 adds per-Spirit filtering.
