## Deferred from: code review of 5-5d-spirit-registry-over-mcp-streamable-http-with-three-trust-tiers (2026-05-24)

- Server tests are `#[ignore]` stubs with no replacement e2e test — deferred to Task 16 (unchecked). `server.rs` JSON-RPC dispatch path completely untested.
- `search()` holds index Mutex while repeatedly acquiring yanks Mutex — O(N×M) lock contention. Acceptable at v0.5-α scale (<10⁴ Spirits). Deferred to Story 7.2 for B-tree/inverted-index optimization.
- `monotonic_now_ns()` resets on process restart — `yanks_since` timestamps not comparable across restarts. Inherent to `Instant`-based approach. Deferred to Story 7.2 for persistent timestamp model.
- `bin/server.rs` is a stub exiting with code 1 — deferred to Task 13/16 decision on `[[bin]]` vs `MAOS_ONE_SHOT` mode.

## Deferred from: code review of 5-4-run-spirit-upgrades-and-propagate-signed-revocations-in-5s (2026-05-22)

- No upper bound on `entries.len()` in `SignedRevocationList::new()` — DoS hardening beyond v0.3-β scope (revocation.rs:107)
- No `issued_at_ns` sanity check — zero or future timestamps pass through silently (revocation.rs:96-97)
- `RevocationEntry::reason` has no validation — spec says "free-form", accepted by design (revocation.rs:144-147)
- Signature/pubkey JSON wire format (byte arrays) may need hex/base64 for future external CRL interop (Story 5.5d) — acceptable for v0.3-β internal format

## Deferred from: code review of 5-1-ship-full-lifecycle-verbs-and-11-triggers-with-priority-weighted-scheduling (2026-05-21)

- `smoke_epic_4.sh` validates presence but not magnitude of outcomes — meets spec floor, could be strengthened. Pre-existing weak test pattern.
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

## Deferred from: code review of 4-4-enforce-the-i11-audit-chain-on-distillates-with-log-recall-and-the-five-metric-gate (2026-05-19)

- `LogRecallAdapter` lacks optional `Arc<RedactionPolicy>` — Spec AC1 says "optionally"; not required at v0.3-β since redaction is handled at TL write time, not recall time. v0.5+ may add re-validation at fetch time.
- Dead fixture field `intent_lineage_expected` — Forward-shaped for v0.5+ live judge-LLM integration; not validated at v0.3-β because the harness is calibration-mode (corpus-author-annotated, not live-evaluated).

## Deferred from: Story 4.5 — author-the-cross-spirit-isolation-200-corpus-and-enforce-i14-halt-continuity-in-hot-swap

- **Sec-14b cross-Host adversarial runtime** — Story 4.5 ships Sec-14b structurally
  (kernel rejects cross-Host with `IacBusError::CrossHostUnsupported`). The transition
  to "kernel rejects forged peer attempt" (mTLS replay, certificate-pin attack, A2A
  frame injection under load) is owned by Story 6.3 (A2A bilateral mTLS) at v0.5+.
  The corpus is structurally ready — Story 6.3 wires the runtime check WITHOUT corpus
  regeneration.

- **Tier-T3 container-based sandbox-escape scenarios** — Story 4.5's corpus marks T3
  scenarios with `tier_target: "T3"`; v0.3-β runner skips them and counts the skipped
  as deferred-to-5-5a. Story 5.5a wires Tier-T3 container isolation via Docker/Podman
  and unlocks the T3 scenario execution path.

- **`HaltRegistry::drain_for_spirit` per-pid filtering** — already deferred from Story 4.1;
  restated here. The `validate_swap_halt_continuity` wrapper compensates structurally
  via snapshot-before-and-after-drain size diff. Story 5.3 refines.

- **`handauthored-v1` corpus tier** — v0.3-β ships `scripted-v0` per Epic 2 retro A2 closure.
  v1.0 expands to ≥10 hand-authored scenarios per category (≥80 hand-authored across the
  8 categories per split = ≥160 hand-authored across the full corpus). Story 10.2
  (third-party adversarial red-team gate) owns the expansion. The IAA gate also
  strengthens from solo-attestation to ≥2-attestor per category.

- **Shared `CorpusLoader<T>` refactor** — Story 4.1's deferred entry restated; the THIRD
  copy of the loader pattern lands in Story 4.5's `isolation_corpus.rs`. Refactor when
  bandwidth allows; not blocking.

- **`check-service-boundary` baseline staleness** — The `kernel-surface-v0.1-beta.json`
  baseline predates many Stories 3.x–4.x public symbols, causing both "removed" and
  "new" NFR-Test-2 violations that are unrelated to Story 4.5. Baseline regeneration
  is a cross-cutting maintenance task, not Story 4.5 scope.

## Deferred from: code review of story 5-3 (2026-05-22)

- **Legacy halts silently orphaned** — Halts inserted via legacy `insert_pending` never match per-PID filter in `drain_for_spirit` and accumulate unboundedly. v0.3-β trusts lifecycle to drain but bypasses legacy entries.
- **Blocking `std::sync::RwLock` in async context** — Crash handler (`crash_detector.rs`) spawns async but uses synchronous locks throughout. Existing project-wide convention; same pattern in scheduler, halt, hot_swap modules. Could cause tokio worker starvation under contention.
- **`.expect()` on locks in crash recovery path** — Lock poison treated as unrecoverable in crash handler. If `spirits` RwLock is poisoned (e.g., a prior panic in scheduler), no crash can be handled again. Same `.expect()` pattern used throughout kernel.
- **`recover_in_flight_with_tasks` holds writer Mutex during full file parse** — No fsync occurs during cold-restart journal parsing. Acceptable for cold-restart-only path (infrequent), but could block background flush for seconds on large journal files.
- **`scb.transition()` result discarded in crash handler** — Transition to `Unloaded` uses CAS; race-lost is benign (mostly harmless). Follows existing pattern.
- **`terminate_spirit` drains halts during unload — conflicts with concurrent resolution** — Director's concurrent `resolve()` may receive `NotPending` if `drain_for_spirit` removes halt first. Follows established drain-then-resolve ordering.

## Deferred from: code review of 5-5b-run-the-multi-provider-ci-matrix-across-anthropic-openai-and-ollama (2026-05-23)

- Ollama driver lacks `with_api_key` test helper — intentional, no API key needed. `OllamaProvider::new` returns `Ok` unconditionally.
- `provider_history` HashMap in `admit_spirit` grows unbounded under high spirit churn — no cleanup path for terminated spirits. Forward-shaped to Story 9.4.
- `io_call_journal` non-feature stub returns empty `Vec` — currently `#[cfg]`-protected on tests. Acceptable for v0.5-α; structural improvement at v0.5+.
- `UnconfiguredProvider` inserted under `"anthropic"` key in composition root — semantically misleading (enumerating `registered_ids()` shows Anthropic when it's not configured). Pre-existing Story 1b.4 pattern.

## Deferred from: code review of 6-1-ship-the-full-iac-bus-with-retract-primitive-and-drr-fairness-scheduler (2026-05-26)

- AC4 IAC routing budget benchmark entirely deferred to Story 6.2 — no bench file, latency measurement, throughput sweep, budget report, or nfr-perf-1 CI job shipped.
- 4 of 5 promised CI jobs not yet wired — retract-corpus-tests, nfr-scale-3-drr-fairness, nfr-perf-1-iac-routing-budget, smoke-iac-bus-6 all marked as acknowledged deferrals in Task checklist.
- No smoke-iac-bus-6 arm in crates/maos-bin/src/main.rs — Task 5.1 deferred.
- DRR SpiritControlBlock weight integration + [scheduler.weights] config parsing deferred — Tasks 3.3/3.4. All spirits get uniform quantum, hardcoded weight=1.
- NFR-Scale-3 5-spirit + 60s sustained fairness gate test not shipped — Tasks 3.7/3.8 deferred. Only 3 basic integration tests exist.
- iac_log_writer_quantum_consumed_total metric deferred — Task 3.5.
- Spec-drift test log_writer_drr_matches_scheduler.rs deferred — Task 3.7.
- Bridge precondition failures (A2/A3/A5/A6/A4-Debt-1/A4-Debt-2c) accepted as documented debt per Option D team consensus. Missing CI wiring for A3/A5/A6 gates.

## Deferred from: code review of 6-2-dispatch-orchestrator-distillates-with-intent-lineage-and-cliwrapperspirit-worker-pattern (2026-05-26)

- CliWrapper runtime scaffold — actual subprocess stdio bridge NOT implemented; v0.5-α scaffolding by design; full bridge lands in Story 6.5 / Epic 8 Worker pattern [lifecycle/cli_wrapper/runtime.rs:26-30]
- log_recall.rs maps CliSubprocessOutput -> CapabilityInvocation because domain label enum lacks variant; Spirit queries for CapabilityInvocation will leak CLI output rows [iac/log_recall.rs:127]
- FramePayload enum missing CliSubprocessOutput variant — semantic gap between kind and payload type; kernel-internal audit rows don't need domain payload by design [domain/src/frame.rs:62-71]
- from_i64 silently defaults unknown TL discriminants to TaskAssign — pre-existing; cross-version log inspection would misclassify rows [iac/transparency_log.rs:549-553]
- retract_frame_id placeholder [0u8;16] before actual TL write — race window in concurrent retract; pre-existing from Story 6.1 [iac/mod.rs:646-706]
- i9-exemptions.md scope creep — post-hoc exemptions for ProvidersSection, ProviderConfig, McpSection, etc. documented in Story 6.2 sweep [docs/invariants/i9-exemptions.md]
- DashMap race can modestly exceed MAX_LINEAGE_CACHE_ENTRIES (4096) under concurrent delivery; soft cap design [iac/mod.rs:453-456]
- resolve_command TOCTOU between exists() check and spawn — unlikely in practice [lifecycle/cli_wrapper/admission.rs:134-149]
- monotonic_now_ns() returns 0 before init_monotonic_base() — integration test concern [capability/cap_tokens/mod.rs:63-70]
- handle_subprocess_death signature promises Result but always returns Ok [lifecycle/cli_wrapper/lifecycle.rs:30-44]
- Smoke test distillate_id could be [0u8;16] if TL insert silently fails [maos-bin/src/main.rs:3283]

## Deferred from: code review of 7-1-5-section-a2-step-3-closure-17-review-findings-25-dev-model-backfills-hard-fail-flip (2026-05-29)

- Custom YAML frontmatter parsing via `extract_frontmatter()` is fragile — UTF-8 BOM, multiple `### Review Findings` sections, placeholder in code blocks cause false positives/negatives. Not caused by this change; applies to all frontmatter-based gates.
- Smoke arm has no per-gate timeout; a hanging gate blocks indefinitely. CI default timeout applies; code-level timeout is a future hardening item.
- `cargo public-api --diff` verification not cited in Completion Notes despite being an AC5 requirement. Likely done but not recorded.

## Deferred from: code review of 7-2-ship-end-to-end-registry-publish-install-yank-and-air-gapped-import (2026-05-29)

- W1: Test temp files never cleaned up in 4 test files under `crates/maos-spirit-cli/tests/` — test-only, no production impact, follows existing codebase pattern
- W2: `extract_toml_kv` prefix match latent fragility in `signing.rs` — currently safe due to `=` check, would only matter if manifest keys share prefixes
- W3: `epoch_to_components` month overflow with extreme timestamps in `yank.rs` — theoretical, can't happen with valid SystemTime values
- W4: `yank_cursor_persistence_test.rs` as separate file vs inline — behavioral coverage exists inline in `yank.rs::cursor_tests`, file placement is cosmetic
- W5: `cargo public-api --diff` not run — verification step, not a code issue (already captured as dev RF-9)
- W6: Unrecognized tar entries silently discarded without warning in `import.rs` — minor, matches common tar tooling behavior

## Deferred from: code review of 7-3-verify-complianceclaim-envelopes-at-admission-with-the-ccac-n-600-ship-gate (2026-05-31)

- D1: Legacy shim `verify_envelope_structural` passes `now=0`, disabling expiry — pre-existing design for backward compat, v0.5-α path intentionally weaker (`compliance_verify.rs:655`)
- D2: `run_coverage_with_fixture` skips total/drift-count validation — **closed**: added drift count to fixture coverage path
- D3: `build_malformed`/`mutate_field` panic on unknown seed ops — **closed**: refactored to return `Result`/`Option`
- D4: `extract_manifest_fingerprint_fields` still silently defaults unknown enums — pre-existing from v0.5-α lift; `parse_claim_strict` (v1.0 path) correctly rejects
- D5: Empty string `CryptoProviderId`/`ProviderEndpointPin` bypass drift — frozen ABI types; empty strings are default for manifests without these fields
- D6: `deny_unknown_fields` may not be enforced by `serde_cbor` 0.11 — `serde_cbor` 0.11 does support the attribute; corpus malformed seeds would catch any failure
- D7: `serde_cbor` determinism not pinned to exact patch version — Cargo.lock pins exact version; dev record documents trade-off
- D8: No test for simultaneous multi-field drift — **closed**: added `multi_field_drift_names_first_divergent_field` test
- D9: `drift_count()` builds full corpus just to count drift items — **closed**: arithmetic count from seeds (`seeds.filter × VARIATIONS_PER_SEED`)
- D10: `reference_context` hardcoded values must match manifest strings — shared builder produces both from same parameters; ship gate validates round-trip
