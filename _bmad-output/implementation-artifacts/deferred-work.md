## Deferred from: Story 9.5b preflight party-mode Round 2 (2026-06-16) — OTel spans are a new telemetry data-sink the GDPR erasure scope must acknowledge

> Surfaced by Murat in the 9.5b OpenTelemetry-adapter preflight (Winston·Amelia·Murat·John, ratified Lunarpulse). Cross-story finding — out of 9.5b's own ACs by design, filed here so the gap is owned, not assumed-closed.

- **OTel span attributes are a 4th telemetry surface the forget cascade cannot reach.** The GDPR Art.17 forget cascade reaches exactly `REGISTERED_ERASURE_BACKENDS = ["private","principal_index","shared"]` (`crates/maos-kernel-core/src/memory/mod.rs:35`). Span attributes emitted by the new `maos-telemetry` adapter are a NEW data sink outside that set. Story 9.5b mitigates this **by construction** via `gate:otel-attr-contract` (AC-5): spans carry **zero principal/subject nexus** → erasure-exempt by construction, mirroring `governance.rs:122` ("zero principal nexus → stays OUT of the forget cascade"). So spans are NOT wired into the cascade, and the attr-contract gate prevents a future contributor from silently adding a PII key.
- **What's deferred / owned elsewhere:** the *existence* of this telemetry surface should be acknowledged in the **GDPR erasure scope documentation / data-sink inventory** owned by Stories 9.2 / 9.2b (forget cascade) — not just constrained inside 9.5b's test gate. Action: when the 9.5b adapter lands, add "OTel span attributes (erasure-exempt by zero-principal-nexus construction; enforced by `gate:otel-attr-contract`)" to the erasure-scope/data-sink inventory so the exemption is a documented invariant, not an implicit one. Flag to the 9.2 erasure owner (John/Winston). E1b ComplianceClaim schema is unchanged (no new claim field) — this is scope-documentation, not a claim amendment.

## Epic-7 §A3 skill-queue closure — VERIFIED by Story 8.16 (2026-06-12, "verify don't assume"); 2 of 4 items still OPEN → Epic 9

Epic 7 retro §A3 required four skill-queue items closed "before Story 8.4 (founder-loop)". Story 8.16 AC6 verified them mechanically at HEAD (`crates/maos-skill/`, `crates/maos-cli/`):

- ✅ **SkillId charset enforced** — validated in `maos-skill/src/schema.rs:178` + `proposal.rs:52` (`a-z 0-9 - .`), typed errors `errors.rs:33,60`.
- ✅ **Duplicate-skill-ID enqueue rejected** — `ESkillQueue::DuplicateSkillId` typed error, returned in `admission.rs:106` (`enqueue_skill`) + `:144` (`enqueue_proposal`).
- ✅ **Skill-queue restart persistence — CLOSED by Story 9.7 (2026-06-17).** `SkillQueueStore` trait + `LocalFsSkillQueueStore` at `~/.local/share/maos/skills/queue.json` with own atomic-write helper (temp+rename+dir-fsync), `schema_version: "maos.skill-queue.v1"` hard-fail, `audit` = `#[serde(skip)]` (principal-free). Round-trip + fault-injection tests in `crates/maos-skill/tests/admission_store_test.rs`.
- ✅ **`maosctl skills approve/reject` functional — CLOSED by Story 9.7 (2026-06-17).** `dispatch_skills` Approve/Reject rewritten to load→journal-FIRST-to-TL→mutate→persist-atomically. Unknown/already-resolved no-op semantics preserved. `list` reads persisted+reconciled state.

**FILED to Epic 9** (explicit, NOT silently inherited): the durable skill-queue store + functional `approve/reject` (operator-admission exit that actually mutates state and survives restart). Natural home = Story 9.6 (scheduler/runtime) or a named skill-queue story at Epic-9 sprint planning. Tracked here so the gap is owned, not assumed-closed.

**RESOLVED 2026-06-17 → Story 9.7** (split from 9.6 AC-5 at party-mode preflight, ratified Lunarpulse). Both OPEN items are 9.7's AC-1 (durable store, own atomic write, `maos.skill-queue.v1` schema) + AC-2 (functional `approve/reject`). Slotted as the Epic-9 CLOSER with a hard gate: `epic-9-retrospective` cannot open until `9-7` is `done` — the compounding gate that ends the 3-epic decay.

## ~~Daemon-side skill-admission ENFORCEMENT — filed to Epic 10 (Story 9.7 preflight 2026-06-17, John's F6b finding)~~ **RESOLVED 2026-06-19**

**RESOLVED:** The `maos run` daemon now consults persisted admission state before spirit-load. Skills in `Rejected` state block daemon startup with a typed error; `Pending` skills emit a warning. The enforcement uses `maos_cli::subcommands::admission_view` (discovery + TL reconcile + LWW decided-set) at the composition root, before any spirit is loaded.

- ~~**Gap:** an operator who `maosctl skills reject <id>` expects that skill blocked from loading.~~ **FIXED:** Rejected skills now emit `FATAL` + block daemon startup. Pending skills emit `WARNING`.
- ~~**Follow-up story (Epic 10):** the `maos run` daemon honors persisted admitted/rejected skill state at spirit-load.~~ **IMPLEMENTED** in `maos-bin/src/main.rs` (composition root), reusing `admission_view` from `maos-cli`.
- **STILL OPEN (R8): single-writer TL via the daemon.** 9.7 ships the operator CLI writing the Transparency Log directly (`maos-cli → maos-iac`, WAL + `busy_timeout=5000` + bounded retry + journal-first commit ordering) — a documented multi-process-writer model whose residual after-timeout race is an accepted limitation. The true retirement of that race is routing the CLI's TL write THROUGH the daemon (one writer) — naturally bundled with the enforcement work above (the same daemon that learns to honor admission state should own the journal write). Latent watch-item (Murat): if reconcile-LWW ever resolves "latest" by `timestamp_ns`, an NTP step-backward could elect the wrong winner — the `, id ASC` tie-break added to `query_approvals` in 9.7 is the same fix; preserve it.

## Deferred from: code review of 8-12-live-cliwrapper-subprocess-bridge-founder-loop-over-real-clis (2026-06-08)

- No sandbox enforcement on hermetic path — Command spawn by design; live container = Tier-2. `runtime.rs:spawn_and_bridge`
- Cap-token mediation failure continues to spawn — Layered security design; host-grant already passed; cap-token = Epic 9 operator policy enrichment. `main.rs:371-388`
- on_pause/on_resume are documented v0.9 no-ops — Signals/NamedPipe emit advisory, not wired. `runtime.rs:692-700`
- on_unload always SIGKILLs — `#![forbid(unsafe_code)]` prevents `libc::kill(pid, SIGTERM)`. `shutdown_signal` is dead config. `runtime.rs:722`
- Recovery executor not wired in composition root — Tested in isolation; daemon-mode wiring is Epic 9. `runtime.rs:814-849`
- argv_prefix_hash check is tautological — Re-derives from in-memory spec (same data used to issue token). Catches misconfiguration, not disk TOCTOU. `runtime.rs:398-400`
- Backpressure::Block + slow journal stalls child — Design intent (backpressure), but slow journal write blocks child process. `runtime.rs:438-441`
- recv_line doesn't filter by stream — J1 bench's sh echo never writes stderr, so no hang today. `runtime.rs:647-653`
- No egress enforcement for live CLIs — `permitted_egress_destinations` declared but not kernel-enforced. `host_grant.rs`
- wait_and_finalize exit row failure + cap-token revoke gap — If exit journal write fails, revoke still fires. Audit trail gap. `runtime.rs:607-618`
- resolve_cli_binary no execute-permission check — Late failure at spawn time with cryptic OS error. `main.rs:246-268`
- StaticHostGrantAllowlist self-grant (v0.9 seam) — Allowlist is self-constructed from manifest values; operator-managed source = Epic 9. Annotation added per team consensus (Winston + John). `main.rs:334-336`

## ~~Deferred from: code review of 8-11-live-runtime-spine-daemon-composition-root-and-inference-port (2026-06-08)~~ **RESOLVED**

- ~~`JournalAdapter::open` fails on first corrupted line with no recovery~~ — **FIXED 2026-06-08**: corrupted lines are now skipped with a warning instead of failing the entire open. A counter tracks how many lines were skipped, and a summary warning is emitted after the scan. The daemon boots resiliently even if the journal has trailing corruption from a crash. `crates/maos-kernel-core/src/journal/mod.rs:113-138`
---
dev_model_used: claude-opus-4-7
---

## Deferred from: party-mode implementation audit of Epic 8 (2026-06-06)

> Two-round adversarial audit (party-mode: John / Winston / Murat / Amelia + Mary) of whether Epic 8 delivers the PRD user journeys. Surfaced **NEW security/invariant defects not previously logged**. Each is now owned by a completion-delivery story (8.9–8.15), registered via `sprint-change-proposal-2026-06-06.md`. Listed here for canonical defect tracking.

### Security — A2A cross-Host (closed by Story 8.9)
- **G8 — peer-identity bypass (CRITICAL).** The live mTLS verifier learns the true peer (`crates/maos-a2a-tcp/src/verifier.rs:177` — `find_active_pin_by_fingerprint → Some(_peer)`) then discards it; `serve_connection` hands `handle_intake` the *request*, which re-derives identity from attacker-supplied `frame.from.host_id` (fallback `"loopback"`) at `crates/maos-a2a-core/src/router.rs:438-441`, and the intake `verify_pinned` compares the config fingerprint against itself (`X==X`). A mesh peer with any one validly-pinned leaf can set `frame.from.host_id="nash"` and inherit Nash's accept-allowlist — the confused-deputy J4 exists to prevent. `fix: thread the verified PeerId into handle_intake; reject frame.from != tls_verified_peer`.
- **G1 — consent granter replay (HIGH).** No check that `consent_envelope.granter == frame.from` (`router.rs:531`); a leaked envelope can be replayed by a different sender (compounds with G8). `guard: if env.granter != frame.from { return Err(ConsentGranterMismatch) }`.
- **G10 — consent-expiry dead code (HIGH).** 8.6 fixed the consent clock to fail-closed (`wall_now_ns`), but `prepare_outbound` (`router.rs:339-377`) never populates `consent_envelope.valid_until_ns`, so the `if let Some(valid_until_ns)` expiry check (`router.rs:531-534`) is unreachable on every real cross-Host frame. Production has NO consent-expiry enforcement; all "expiry" tests pass against hand-built unit envelopes.
- **G2 — expired-consent masked by ordering (MEDIUM).** `accept_admits` runs before `is_expired` (`router.rs:517` before `:531`); an expired+denied frame reports intent-denial, masking expiry. Becomes a wrong-root-cause incident timeline once G10 is fixed.
- **G9 — intent length-bound contradiction (MEDIUM).** Router accepts `intent.len() <= 1024` (`router.rs:260`) but canonical caps at `MAX_CANONICAL_INTENT_LEN = 128` (`crates/maos-domain/src/invariants/i8.rs:44`); a 129–1024-byte intent is used as the match key yet can never be canonical → guaranteed non-match + warn-dedup amplification.
- **G4/G5/G6 (MEDIUM/LOW).** TOFU pin-store restart-TOCTOU + duplicate-peer silent overwrite (`router.rs:139-145,:474-514`); brittle webpki/io `Debug`-string error classification that can misclassify on dep wording drift (`verifier.rs:202`, `mtls.rs:63`); intake timeout covers only the socket read, not `handle_intake` + NACK write (`transport.rs:403,:433,:437`).

### Invariant enforcement holes (closed by Story 8.10)
- **I11 citer-authorization gap.** `write_distillate(spirit_pid, request)` (`crates/maos-iac/src/adapter/distillate.rs:289-369`) verifies cited frames EXIST but never that `spirit_pid` is authorized to cite them; Researcher forwards a caller-supplied pid (`spirits/researcher/src/lib.rs:528,533`). A Spirit can mint a digest whose lineage derives from another principal's raw frames. Compounded: `TransparencyLogAdapter::insert_frame_event` is `pub` and its doc admits direct `FrameKind::Distillate` inserts bypass the canonical producer (`transparency_log.rs:58`). `fix: gate insert_frame_event for Distillate to DistillateWriter; add principal-namespace citer check`.
- **I12 records nothing (silent fail-open).** `decision_logger.rs` v0.3-β `digest_provider` returns an empty refs set; `frame_carries_i12_refs` returns `true` unconditionally (doc: "always-true; production SHOULD NOT branch"). NFR-Aud-5's "100%" is satisfied by shape, not content. `fix: wire the Memory-Manager digest source-of-truth (Story 4.3's deferred seam)`.
- **Observer watchdog fails-open on NaN.** `spirits/observer/src/lib.rs:460,521` silently `return None` on NaN drift/structural magnitude → the safety watchdog goes quiet exactly when a Spirit emits garbage; `WatchThreshold::new` accepts NaN/Inf. `fix: reject-and-flag`.
- **pub-field constructor bypass class.** `anomaly_flagged` / `EpistemicHaltPayload` / `StructuralSignal` / `DistillationRequest` allow NaN/empty via struct-literal, skipping the validating `new()`.

### Story-correctness (closed by Story 8.10·AC1)
- **Butler AC2 — production halt never fires (regression of a `done` story).** `spirits/butler/src/lib.rs:250-270` production `on_idle` computes the assessment and drops it — no scalar write, no halt. The scalar→policy→halt path runs only in `tests/corpus_halt.rs`. The 8.1 review marked the `[Patch]` "Must fix — test-only proof insufficient for v0.3" as applied `[x]`; the hook code was never changed.

### Journey-presentability gaps (closed by Stories 8.11–8.14; asserted by harness 8.15)
- **No live LLM path** — zero reference Spirit calls the Inference Port; all cognition is deterministic (→ 8.11).
- **No runnable daemon** — `maos-bin` is 54 env-gated smoke arms, no serving loop (→ 8.11 + 8.14a).
- **CliWrapper subprocess bridge unbuilt** — `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs` is `argv_prefix_hash` under a doc comment; Worker spawns no real CLI (→ 8.12).
- **Live transport carries no Spirit traffic** — `smoke_a2a_tcp_8_6`'s `mira`/`nash` locals are router handles, not the cognitive structs; Mira/Nash cognition runs only on loopback (→ 8.13).
- **MCP is fixture-replay** — `spirits/butler/src/lib.rs:76-77`; no real Calendar/Slack/arXiv drivers (→ 8.14b/c).
- **`hello-spirit` is 0 LOC; no `maos init` / shell / `maos audit query`** (→ 8.14a).

## Deferred from: code review of 8-7-fine-grained-typed-intent-consent-vocabulary-over-maos-a2a-core (2026-06-06)

- Consent envelope granter mismatch — replay attack [router.rs:250-257] — No validation that `consent_envelope.granter` matches `frame.from`. A stolen consent envelope could be replayed by a different sender. Pre-existing issue; not introduced by 8.7. `guard_snippet: if let Some(ref env) = frame.consent_envelope { if env.granter != frame.from { return Err(A2AError::ConsentGranterMismatch); } }`
- Expired consent masked by intent-denial error [router.rs:497-534] — The accept-side check runs `accept_admits` before `is_expired`. If both fail, the intent-denial error masks the expired-consent error. Pre-existing ordering issue in `handle_intake`; not introduced by 8.7. `guard_snippet: // Move consent expiry check before accept_admits`

## Deferred from: code review of 7-1-6-section-a2-full-flip (2026-06-02)

- `cargo public-api --diff` skipped instead of run (AC6) — spec requires running the command but story is discipline-substrate only (.md + discipline.yml); zero ABI impact expected. Spec deviation acknowledged.

## Deferred from: code review of 7-5b-execute-nfr-onb-1-30-minute-first-spirit-validation-gate-at-v0-3 (2026-06-01)

- Fragile `LocalRunner` string-contains heuristic in `classify_prerequisites` — greps source for `"impl LocalRunner"` (matches comments/doc comments). Test-only prereq check; works for current codebase. `crates/maos-eval/src/onboarding_gate_corpus.rs:1211-1214`
- `participant_id` schema pattern `^P[0-9]{2,}$` not enforced by Rust code — `ParticipantRecord` accepts any `String`. Schema is the validation boundary. `crates/maos-eval/src/onboarding_gate_corpus.rs:611`
- ~~`CorpusLine` uses `#[serde(untagged)]` producing poor error messages on malformed fixture lines.~~ — FIXED: replaced untagged with manual dispatch on `stand_in_for` key. `crates/maos-eval/src/onboarding_gate_corpus.rs`
- `workspace_root()` in test helpers relies on CWD being crate directory — pre-existing project convention. `crates/maos-eval/src/onboarding_gate_corpus.rs`

## Deferred from: code review of 7-5a-publish-and-enforce-v1-0-abi-stability-commitments (2026-05-31)

- ColdSwap/HotSwap upgrade paths bypass `admit_spirit` entirely — directly insert successor SCB without ABI version checks. Pre-existing gap (Story 5.x). Story 5.5x tracks the fix. `crates/maos-kernel-core/src/lifecycle/upgrade.rs:131-171`
- ~~`POST_V1_SCHEMA_SECTIONS` must be manually maintained on future schema bumps~~ — FIXED: `check-manifest-schema-version` Step 5 now gates the constant; future bumps that forget it fail CI.

## Deferred from: code review of 7-4-author-skills-and-propose-revisions-with-output-shape-fail-loud (2026-05-31)

- `maosctl skills approve/reject` are acknowledgement-only stubs (no real queue interaction) — acknowledged v0.5 limitation; queue logic IS tested in-unit. Persistent queue store is future work.
- `parse_skill` unknown-field classification depends on serde error message string (`"unknown field"`) — bounded by `check-skill-schema` xtask gate but fragile coupling to serde internals.
- Queue is in-process only (`Vec<PendingEntry>`) — no cross-invocation persistence; audit trail lives only as long as the process. Acknowledged v0.5 gap per dev record.
- ~~Discovery scans only top-level files (`read_dir`, flat, non-recursive)~~ — **RESOLVED by Story 10.5 (2026-06-25)**: `crates/maos-skill/src/discovery.rs` now supports directory-aware `dir/SKILL.md` bundles for Anthropic Skills conformance while keeping top-level file discovery intact.

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
- ~~`provider_history` HashMap in `admit_spirit` grows unbounded under high spirit churn — no cleanup path for terminated spirits. Forward-shaped to Story 9.4.~~ **CLOSED (Story 9.4b AC-8, 2026-06-15):** replaced the unbounded `HashMap` with the bounded `ProviderHistory` (`crates/maos-kernel-core/src/security/mod.rs`) — cap 4096, **overflow policy = evict-oldest-by-first-insertion** (never reject-new, so the latest provider is always tracked). Eviction of a stale Spirit only makes a later re-admission first-seen (no false ProviderSwitched); state is ephemeral and never serialized into replayed artifacts. Tests: `provider_history_is_bounded_under_churn`, `provider_history_tracks_switch_and_keeps_newest`.
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

## Deferred from: code review of 7-1-7-baseline-reset-service-boundary-101-stale-plus-72-boundary-and-serde-300-green-at-head (2026-06-01)

- `McpClientAdapter` exemption documents admitted architectural gap — exemption itself says "deferred tidy-up" with no automated escalation. Pre-existing belt-and-suspenders pattern; the exemption is honestly documented and tracked. `xtask/src/check_service_boundary.rs:78-81`
- `// p1-allow:` magic-string comment convention — no compiler enforcement; a typo silently disables the exemption. Spec-chosen mechanism for P1 false-positive resolution; the gate-correctness fix was agreed design. `xtask/src/check_service_boundary.rs:594,599`
- `ADAPTER_PORT_EXEMPTIONS` third field is free-form text — mixes `"N/A"` sentinels with narratives in a `(&str, &str, &str)` tuple with no enum. Pre-existing pattern established in prior stories. `xtask/src/check_service_boundary.rs:40-83`
- `serde-error-allowlist.toml` line-number entries have zero staleness detection — any line insertion above a frozen site shifts its number, silently dropping the allowlist entry and causing CI hard-fail. Known FREEZE posture tradeoff; follow-up Story 8.x full remediation will empty the allowlist. `xtask/serde-error-allowlist.toml`
- Frozen allowlist ratchet can only grow — the gate hard-fails on NEW violations but has no content-digest for existing entries; stale entries produce false-positive CI failures. Story 8.x serde-remediation follow-up tracked in allowlist header. `xtask/serde-error-allowlist.toml`
- `// p1-allow:` marker context-free — bare substring match on constructor line or line above can match unrelated `// p1-allow:` comment, silently exempting a different construction. Accepted risk in gating-correctness decision. `xtask/src/check_service_boundary.rs:593-600`
- `infer_module_path` hardcodes crate name `"maos_kernel_core"` — if check is re-used for another kernel crate, exemption documentation cross-check would fail. Pre-existing; single-kernel-crate workspace layout makes this benign. `xtask/src/check_empty_kernel.rs:222`
- `MockLifecycleResolver` is pub, not `#[cfg(test)]`-gated — `pub mod test_double` compiles into production binaries; `#[i9_exempt]` masks it from I9. Protected by separate `check-mock-not-in-release` gate. `crates/maos-kernel-core/src/scheduler/verb_resolver.rs:131-142`
- Serde hard-fail flip unconditional on line-number-based allowlist — removing `continue-on-error` makes all serde violations blocking; false positives from line drift can block CI. Accepted FREEZE posture tradeoff; mitigation = Story 8.x follow-up. `.github/workflows/discipline.yml:1008`

## Deferred from: code review of 8-3-observer-v0-5-telemetry-stream-subscriber-pre-halt-scalar-drift-watchdog (2026-06-03)

- `WatchThreshold::new` accepts NaN/Inf threshold — silently disables drift detection. Code-constructed config; NaN is a programming error, not a runtime condition. `spirits/observer/src/lib.rs:742-758`
- `CapturingChannel` test doubles use `.lock().unwrap()` — inconsistent with production poison-safe pattern. Test-only code; pre-existing pattern across Spirit test doubles. `spirits/observer/tests/drift_watchdog.rs:1386`
- `StructuralSignal` has no constructor validation — magnitude outside `[0.0, 1.0]` silently clamped. Fixture-controlled at v0.5; the clamp is defensive. `spirits/observer/src/lib.rs:249-263`
- Multiple watches for same tag — only first `find()` match used. Observer always constructed with unique tags; configuration correctness issue. `spirits/observer/src/lib.rs:458`
- Empty `PrincipalScope` silently drops all events. Observer always constructed with at least one pattern. `spirits/observer/src/lib.rs:89-125`
- Empty `observer`/`subject` strings not validated by `anomaly_flagged` constructor — pre-existing gap in `maos-domain`. `crates/maos-domain/src/notification.rs:81-103`
- `NotificationEvent` `#[non_exhaustive]` wildcard branch in `TerminalChannel` is dead code — pre-existing, not Observer-introduced. `crates/maos-director-surface/src/notification.rs:226-228`

## Deferred from: code review of 8-2-ship-the-researcher-reference-spirit-with-distillation-pattern-and-log-recall-walker (2026-06-03)

- No runtime enforcement of manifest budgets — `time_cap_seconds` and `memory_max_mb` are declarative only with no timer, frame-count limit, or allocation tracker in `survey`. Pre-existing pattern shared with Butler and other Spirits.
- `on_idle` cannot consume `pending` frames / architecturally disconnected from `walk` — `pending` is a bare `Option<Vec<RecalledFrame>>` and `on_idle` takes `&self`; production path unclear. Design choice for test harness pattern; full lifecycle integration expected at v0.5+.
- `incorporate_scalar` truncates precision via `event.value as f32` then stores back as `f64` — precision loss is acceptable for epistemic-policy scalar thresholds which operate at 0.1 granularity.
- `bibliography` may contain duplicate entries — no deduplication on push; minor issue, same pattern as other Spirit outputs.
- Aggregate `needs` array continues unmaintainable growth — appending `researcher-tests` to 80+ item single-line `needs` array. Pre-existing CI pattern across all workflow jobs.

## Deferred from: code review of 8-5-ship-the-mira-nash-diagnostic-architect-bilateral-pair-with-safety-critical-corpus (2026-06-04)

- HaltFlow::submit_resolution partial failure (resolved-but-unjournaled) — If `resolver.resolve()` succeeds but `journal.journal_halt_resolution()` fails, halt is resolved in registry but has no audit journal row. `crates/maos-director-surface/src/halt_ui.rs:71-80` — pre-existing, not introduced by this change.
- NotificationDispatcher swallows all channel errors — `dispatch()` counts `Err(_)` in `report.errors` but does not propagate, log, or identify which channel failed. `crates/maos-director-surface/src/notification.rs:62-81` — pre-existing, not introduced by this change.
- A2A un-pinned peer path untested — `verify_pinned` returns `EPinMismatch::NotPinned` if peer never pinned. All new tests call `pin_first_contact` before routing. `crates/maos-a2a/src/tofu.rs:196-200` — pre-existing, not introduced by this change.
- A2A timeout leaves handle_intake future dangling — `tokio::time::timeout(timeout, intake_fut).await` returns `PartitionTimeout` on expiry, but `handle_intake` may still be executing. `crates/maos-a2a/src/adapter.rs:289-298` — pre-existing, not introduced by this change.
- install_intake_sink is racy with in-flight frames — Sink replaced under `tokio::sync::Mutex`, but frames already accepted by `handle_intake` and awaiting sink access could be dropped. `crates/maos-a2a/src/adapter.rs:115-121` — pre-existing, not introduced by this change.
- LoopbackA2ARouter duplicate peer_id silently overwrites — `LoopbackA2ARouter::new` logs warning and overwrites on duplicate `peer_id`. `crates/maos-a2a/src/adapter.rs:97-102` — pre-existing, not introduced by this change.
- A2A handle_intake boot_nonce restart detection races on invalidation — `invalidate_for_restart` called; if NACK lost, peer could retry with old boot_nonce. `crates/maos-a2a/src/adapter.rs:383-423` — pre-existing, not introduced by this change.
- Consent intent taxonomy gap — `ConsentAllowlists` accepts free-form `A2AIntent` strings, but `frame_intent_str()` only projects to `"highprivilege"` / `"standard"` / `"readonly"`. Specific intent like `"diagnostic.advisory"` would silently never match. Acknowledged substrate gap in story doc Ruling 1.

## Deferred from: code review of 8-6-ship-the-live-cross-host-a2a-tcp-mtls-transport (2026-06-05)

> Security-critical-first PARTIAL review pass (src only; tests/CI/composition-root deferred to follow-up review runs).

- ~~**Consent-expiry fails open on the live wire** [crates/maos-a2a-core/src/router.rs]~~ **RESOLVED 2026-06-05** (user-directed, fixed in 8.6): replaced the `monotonic_now_ns` per-call counter with `wall_now_ns()` (fails closed) + an additive pinnable `consent_now_ns` clock on `A2ARouterCore` (`new()` signature unchanged); +2 regression tests. Flagged for Winston as additive AC-A6 churn (no protocol-surface change). No longer deferred.
- **End-to-end peer↔cert binding gap** [crates/maos-a2a-core/src/router.rs:312,330] — receiver `handle_intake` identifies the peer from attacker-supplied `frame.from.host_id` (fallback `"loopback"`) and its "TOFU verify" compares config-against-config, never the wire cert. Real binding is the TLS-layer verifier; this is the receiver half of the dial-side peer-scoping decision item. Pre-existing pin-store races (boot_nonce restart TOCTOU, duplicate-peer overwrite) remain logged from 8.5.
- **Brittle upstream-string error classification** [crates/maos-a2a-tcp/src/verifier.rs:179; transport.rs is_frame_too_large] — webpki `Debug` and io::Error `Display` substring matches can silently regress AC-T5/AC-T8 on dep wording drift. Deps version-pinned; recommend typed-error matching in a hardening follow-up.
- **Intake timeout covers only the READ, not handle_intake / the timeout-NACK write** [crates/maos-a2a-tcp/src/transport.rs serve_connection] — a slow validation step or a best-effort NACK write to a non-reading slow-loris peer is unbounded by the intake timeout. Narrow today (handle_intake is in-memory; H6 drop-guard still aborts on teardown). Wrap handle_intake + NACK write in the same timeout budget if intake ever gains I/O.

## Deferred from: code review of 8-13-cross-host-live-pair-spirit-tcp-binding-and-mobile-push (2026-06-08)

- **Mock push server single-shot / thread never joined / panic surfaces only as recv_timeout disconnect** [crates/maos-bin/src/main.rs:83 `spawn_push_server`; crates/maos-notify-push/src/lib.rs:134 `spawn_one_shot_server`] — test robustness only. The server accepts exactly one connection (matches the single expected POST) and the spawned thread is never joined; a server-side read error/panic propagates as a generic `recv_timeout` disconnect rather than the underlying cause, making CI flakes harder to diagnose. Not a production defect.
- **Subprocess smoke has no outer timeout — a hang blocks rather than fails** [crates/maos-bin/tests/smoke_mira_nash_tcp_8_13.rs:15] — `Command::output()` has no wall-clock kill. Internal ops are individually bounded (push 2s, recv_timeout 2s, TLS handshake 30s), so a true infinite hang is unlikely today, but any future unbounded await would block CI indefinitely instead of failing. Consider spawn + kill-on-timeout.

## Deferred from: code review of 8-13-1-genuine-cross-host-consent-denial-rupture-over-tcp (2026-06-09)

- `smoke-mira-nash-8-5` no longer exercises the full `LoopbackA2ARouter::route_outbound` -> deny path [`crates/maos-bin/src/main.rs:smoke_mira_nash_8_5`] — acknowledged trade-off because `maos-a2a` is edit-forbidden and `LoopbackA2ARouter` lacks a rupture hook. Coverage loss is real but bounded by project constraints.
- `RuptureReason` is hardcoded to `IntentAllowlistMismatch` in `emit_consent_rupture` — extensibility concern for future deny reasons (expired consent, policy violation, etc.), not a current defect for the scoped `-32001` leg.
- Denied fine-grained intent string is not preserved in the `ConsentRupture` payload — the rupture frame carries only coarse `IntentClass` + `original_frame_id`; audit observability of which specific intent was denied requires correlating to the unadmitted original frame. Schema enhancement, not an explicit AC3 violation.


## Deferred from: code review of 8-14a-j0-evaluator-surface-and-runtime-cli (2026-06-09)

- Posture hash all-zeros placeholder — v0.1 scope, no derivation spec; correct for initial release [crates/maos-shell/src/lib.rs:170] — pre-existing design decision
- `MAOS_REPO_ROOT` env var trusted without validation — dev-only env var for skill staging, not user-facing [crates/maos-shell/src/lib.rs:49] — internal tooling
- `copy_dir_all` has no symlink or depth guards — dev-only skill staging utility, not user-facing [crates/maos-shell/src/lib.rs:271-283] — internal tooling

## Deferred from: code review of 8-14c-researcher-mcp-driver-set-web-arxiv-github-citation (2026-06-10)

- Missing `researcher_8_14c.rs` subprocess test (AC3 §7) — requires mock MCP server scaffolding in Story 8.15 test harness [crates/maos-bin/tests/]
- Missing `journey_researcher.rs` journey test (AC3 §8) — depends on Story 8.15 PTY harness [crates/maos-journey-test/tests/]
- Missing two-sided barrier-gated parallelism test (AC3 §4) — requires `LiveResearcherMcpPort` + async runtime instrumentation [crates/maos-bin/tests/]
- Missing BudgetWarning@80% observability test (AC3) — kernel mechanism exists; assertion requires subprocess harness
- ~~Missing citation replay negative falsifiability test (AC3 §6b)~~ — **CLOSED 2026-06-10**: added `fabricated_cite_replays_empty` unit test in `spirits/researcher/src/lib.rs`
- Missing golden-snapshot determinism floor test (AC3 §3) — **PARTIALLY CLOSED 2026-06-10**: added `survey_over_fixed_frames_is_deterministic` unit test (byte-identical serialization guard); zero-side-effect assertion (zero McpInvocation frames) still needs TL access → deferred to 8.15
- `spirit_pid = 0` hardcoded in --live arm — pre-existing pattern from 8.14b Butler; all daemon smoke arms use pid 0 [crates/maos-bin/src/main.rs:2001]

## Deferred from: code review of 8-15-journey-acceptance-test-harness-and-red-phase-suites (2026-06-11)

- Seed cassettes all-zero `prompt_sha256` — intentional for hand-authored seeds; drift detection activates for Tier-2 recorded cassettes. `crates/maos-journey-test/cassettes/*/`
- `extract_recorded_at` line-by-line scan fails on minified JSON — cassettes always written with `to_string_pretty`; not a practical concern. `xtask/src/cassette_age_gate.rs:75-84`
- `CassetteRecordPort` non-atomic write on drop — low-severity, process-kill edge case. `crates/maos-bin/src/cassette_replay.rs:195`
- `check_env_contract` text-only matching catches common patterns — not a guarantee; `std::env::var` / macro usage evaded. `xtask/src/check_env_contract.rs`
- `Pty::screen` re-parses entire VT100 buffer per call — O(n) per assertion; optimization for later. Current tests produce bounded output. `crates/maos-journey-test/src/lib.rs:444-449`
- BudgetWarning@80% render not asserted — acknowledged deferral; requires real wall-clock time (`time_cap_seconds`), incompatible with <2s target. `crates/maos-journey-test/tests/journey_researcher.rs`
- Barrier-gated parallelism test absent — requires test-only parallelism seam in `LiveResearcherMcpPort`; would require kernel-core or Spirit edits, violating zero-kernel-KLOC constraint. `crates/maos-journey-test/tests/journey_researcher.rs`
- Seal infrastructure absent (cfg/feature-flag severable seams) — Task 7 BLOCKED for non-author reviewer; infrastructure not needed until seals are executed. Spec: "Mechanize where expressible."
- J1 resume-continuity (John's THREE MISSING BEATS) — Grade B smoke arm (`MAOS_ONE_SHOT=smoke-founder-loop-8-4`) has no halt/resume cycle. Mandatory bridge item that auto-activates when FounderLoopClass gap closes and J1 upgrades to Grade A. Consensus: Winston/Amelia defer (mechanism absent), Murat/John note obligation is real. `crates/maos-journey-test/tests/journey_j1.rs`
## Deferred from: CI remediation 2026-06-11 (first Epic-8 CI validation, round 3)

### NEW STORY NEEDED — spirit-authoring template suite repair (templates/spirit-{rust,ts})
The author-side scaffolding has bit-rotted since Story 7.1 and was never CI-validated
(main had no CI run from the 7.1.5 freeze through Epic 8). `smoke-spirit-author-7-1` is
now ADVISORY (continue-on-error in discipline.yml) until a dedicated story closes:
- **cargo-generate 0.23 compat**: both `templates/spirit-{rust,ts}/cargo-generate.toml`
  declare a `[placeholders] crate_name` that newer cargo-generate RESERVES → generation
  aborts ("you can't override `project-name`/`crate_name`/..."). Drop the placeholder;
  use the built-in `crate_name`/`project-name` (fed by `--name`).
- **Missing hook file**: both tomls reference `[hooks] post = ["post-generate.rhai"]` but
  no `.rhai` exists. Either add the hook files or remove the `[hooks]` block (the inline
  `[template.scripts] post-generate` already prints the next-step guidance).
- **TS template npm path blocked on SDK publication**: `templates/spirit-ts/package.json`
  declares `@maos/spirit-ts: "^0.5.0"` (unpublished) and the scaffolded output runs
  `npm ci` (needs a lockfile). A scaffold-local `file:` path + `npm install` would fix the
  smoke but break real authors scaffolding outside the repo. The honest fix is to PUBLISH
  `@maos/spirit-ts` to npm (or ship a vendored tarball) before this gate can block.
- Rust template git-deps (`maos-spirit-{sdk,abi}` @ github main) DO resolve in CI; not a blocker.

### CLOSED in this round (example-spirit-ts-tests, now GREEN)
- Generated + committed `package-lock.json` for `sdks/spirit-ts` and `examples/example-spirit-ts`.
- Fixed `sdks/spirit-ts` compile errors: `ctx.ts` import `../spirit.js`→`./spirit.js`; type-only
  re-exports → `export type` (index.ts, spirit_test/types.ts); re-export `MockCtx` from spirit_test.
- `examples/example-spirit-ts` dep `@maos/spirit-ts: "^0.5.0"` → `file:../../sdks/spirit-ts`.

## Deferred from: CI remediation 2026-06-12 (round 5 — nfr-perf compile regression)

### ~~NEW STORY NEEDED — rebuild J4/J6 real `kernel_measurement` harnesses (maos-bench)~~ CLOSED

> **CLOSED by Story 10.4c** (2026-06-24):
> - **J4 (`harness/j4.rs::run_j4_kernel`) → DONE.** Real in-kernel `scalar.tap` measurement rebuilt from the verified template (`scalar_tap_subscriber.rs:24-77`). Gate renamed `check-j4-placeholder-red` → `check-j4-latency`. P95=1µs at HEAD (well within 10ms budget). Falsifiability proven: `bench-fault-inject` feature injects ≥15ms delay → P95 crosses 10000µs → RED.
> - **J6 (`harness/j6.rs::run_j6_kernel`) → CUT, FF-J6-guarded.** J6 cold-start latency harness is CUT from 10.4c — §13.1 declares it non-binding ("correctness gate dominates") and it is out of v1.5 scope; it is revived only when a J6 latency assertion or user-facing J6 latency claim is introduced, which CI guard FF-J6 (`xtask check-ff-j6`) blocks until the harness is rebuilt. `run_j6_kernel` now returns `JourneyResult::not_measured("J6")` instead of a plausible fake number. `mira`/`nash` dev-deps and the `FrameOrigin` path fix were NOT pulled in.

### FIXED in this round (nfr-perf gates now build + RUN)
- `orchestrator_fanout_nfr_perf_8.rs`: (a) `handle.recv()` now yields
  `(FrameKind, IacFrame)` — destructure `(_kind, frame)`; (b) removed the
  criterion `bench_function` wrapper (it sampled a 15s sustained-load op → 1
  sample → criterion `slice.len() > 1` panic) in favor of a plain `main`
  (`harness = false` already set) that runs the fan-out once and emits the
  JourneyResult report. nfr-perf-8 → exit 0, p99≈147µs ≪ 500ms, 0 dropped.
- nfr-perf-1 (iac_routing_budget) unblocked by the lib compiling; runs in
  `--quick` (no-panic-on-breach), exit 0. NOTE: it measures P95≈1.65ms vs the
  1ms v0.5-α soft floor — a real over-budget observation, surfaced not masked
  (the gate stays soft-fail/advisory until the §13.1 calibration window closes).

## ~~Deferred from: CI remediation 2026-06-12 (round 6 — disable two broken advisory gates)~~ — CLOSED by Story 8.16 (2026-06-12)

### ✅ RESOLVED — both `if: false` gates RETIRED per ADR-043 (no longer parked)
Story 8.16 (Epic 8→9 readiness bridge, retro §A1) closed this entry. Per the
retro ruling, `if: false` is not an acceptable terminal state — each gate was
RETIRED with a ratified ADR (ADR-043), not re-parked. The new `check-epic-close-green`
gate now hard-fails on ANY `if: false` workflow job, so this failure mode cannot recur.

1. **`smoke-spirit-author-7-1`** — **RETIRED** (ADR-043 Decision 2). Advisory
   spirit-authoring smoke broken by Epic-7 template bit-rot. Spirit-authoring stays
   covered by `example-spirit-tests` / `example-spirit-drift` / `example-spirit-ts-tests`
   / `spirit-test-tests` (all live). A future template-repair story may re-introduce
   a fixed authoring smoke. Job removed from discipline.yml; `if: false` deleted.

2. **`check-epic-6-bridge`** — **RETIRED** (ADR-043 Decision 1). Legacy Epic-6
   debt beacon whose §A2/§A3/§A5/§A6 enforcement migrated to live hard-fail gates
   in Story 7.1.5; its last red (`A4-Debt-1`) was a stale entry-counting predicate
   (i9-whitelist.toml + i9-exemptions.md both exist; I9 enforced live by
   check-empty-kernel + check-service-boundary). Job removed from discipline.yml;
   `if: false` deleted; xtask module kept in tree as archived history.

Aggregate `needs:`/`report-aggregate` updated to drop both retired jobs (no dangling
`needs.<job>.result`); the `²` DISABLED footnote removed. `check-kernel-baseline` (§A4)
and `check-epic-close-green` (§A5) added. YAML re-validated.

## Deferred from: code review of story-9-2-execute-gdpr-article-17-cascade-with-deterministic-replay-and-proof-of-erasure (2026-06-13)

- **No cross-store transaction in the forget cascade (W1, HIGH).** private-delete + index-delete + distillate-scrub + journal run sequentially in `forget_with_reason` with no transaction/compensation; a failure after the private delete leaves dangling index rows and no rollback. Largely pre-existing (the original `MemoryManagerPort::forget` has the same non-atomic pattern); this change widens the non-atomic window by adding the distillate scrub step. ADR-044 covers only the distillate mark+scrub atomicity, not cross-store atomicity. True cross-store transactionality (private FS + sqlite index + TL sqlite) is an architectural lift. `crates/maos-kernel-core/src/memory/mod.rs:155-205`

## ~~Deferred from: code review of 9-2b-journal-export-trajectory-and-deterministic-trace-shape-replay (2026-06-13)~~ **RESOLVED BEFORE COMPLETION**

- ~~**resolve_spirit_pid may return multiple matches; export silently uses the first** [crates/maos-cli/src/subcommands.rs:1625-1630]~~ — **FIXED 2026-06-13**: `audit_trajectory_export` and `audit_sealed_export` now fail with exit code 2 and a clear disambiguation message when `resolve_spirit_pid` returns more than one `(boot_nonce, spirit_pid)` pair.
- ~~**Unmapped kind string in filter.kind is silently dropped** [crates/maos-audit/src/lib.rs:302-307]~~ — **FIXED 2026-06-13**: both `query()` and `query_with_redaction()` now return `AuditError::UnknownKind` for unmapped `filter.kind` strings instead of silently omitting the filter.
- ~~**SQLite numeric casts can silently wrap/truncate** [crates/maos-audit/src/lib.rs:349-352]~~ — **PARTIALLY FIXED 2026-06-13**: binding casts for `spirit_pid` and `limit` now use `i64::try_from` with `AuditError::ValueOverflow`. Row-extraction `as` casts left unchanged because the kernel stores u64 values (including values > `i64::MAX`) via bit-cast in SQLite's signed INTEGER column; changing them to `try_from` would break round-trip of legitimate TL rows.

## Deferred from: code review of 9-4b-region-pinning-model-provenance-and-tenancy-reservation (2026-06-15)

- **Ed25519 double-hash composition** — `regional_teardown.rs` and `sealed_export.rs` sign a SHA-256 digest with Ed25519 (which itself hashes with SHA-512), creating a non-standard `Ed25519(SHA-256(msg))` composition. Internally consistent but differs from pure Ed25519. Deferred: consider signing canonical bytes directly in a future hardening pass.
- **Home signing seed reused as region-key derivation base** — `run_uninstall_cascade` uses the same `signing_seed` as both the HKDF base for region keys and the raw home signing key. HKDF differentiates outputs, but ideal key separation would use distinct seeds. Deferred to a future crypto-hardening story.

## Deferred from: code review of 9-5a-trust-anchor-framing-adr-and-stability-compliance-scope (2026-06-15)

- STABILITY.md relative ADR link will need doc-site rewriting — `[ADR-047](docs/adr/ADR-047-trust-anchor-framing-carry-forward.md)` is correct for GitHub root rendering today, but Docusaurus (Story 9.5) serves STABILITY.md at a different path so the relative link will break. Story 9.5 owns the frozen-URL-contract link handling (AC-1 spine + manifest-seeded link check); re-evaluate the link format when 9.5 wires the doc site. Emitted by the generator at `xtask/src/stability_matrix.rs:218`; rendered at `STABILITY.md:57`.

## Deferred from: code review of 9-5-publish-five-canonical-docs-with-wcag-aa-korean-i18n-and-onboarding-artifacts (2026-06-15)

- **error-catalog `cause`/`remediation` not schema-validated by the Rust xtask gate** — the new structured fields added by 9.5 are enforced by the Node-side `gate:troubleshoot-bidi.js` for docs purposes, but the Rust `xtask error-catalog-check` was not updated to require them. The xtask catalog contract is pre-existing/kernel-adjacent; enforcing cause/remediation there would close the D4 "burden lands on error-definers" contract on the Rust side. Deferred: add field requirements to `xtask/src/check_error_catalog.rs` / `xtask/error-catalog.toml` in a follow-up.
- **BREAKING.md not auto-verified vs STABILITY.md deprecation table** — AC-5 requires "verify BREAKING.md is current vs STABILITY.md"; satisfied manually per the dev record, but no automated gate enforces consistency. Deferred: add a cross-doc drift check as a follow-up.
- **D1 — real axe-core WCAG AA scan (AC-2)** — `@axe-core/cli` + `serve` are installed devDeps but `gate-a11y.js` never invokes them; it runs 3 regex landmark checks on `build/index.html` only, and its `scanned == expected` coverage assertion is WARN-only (D7 not enforced). P-claim (applied in 9.5) makes the current gate honest in the meantime. Deferred to a follow-up: wire axe-core over the served build × manifest × {en,ko}, make coverage a hard gate, resolve AC-2 (hard-gate) vs AC-4 (fallback-is-OK) semantics.
- **D2 — 5 Binding Test Gates + Playwright capability (AC-1 links / AC-3 deep-link / AC-4 fallback+switcher+version-dropdown)** — `gate:links` (orphan-detection seeded from manifest, not crawl-from-root), `gate:fallback`, `gate:switcher`, `gate:version-dropdown`, `gate:deep-link-preserve` are entirely unimplemented; no Playwright dependency exists; `gate:all` omits them. Deferred to a tracked follow-up story: add `@playwright/test`, a served-build CI step, and all 5 behavioral gates (D7: config-presence ≠ behavior).
- **D3 — rustdoc-JSON→MDX /abi/ generation + ≥2-version archive (AC-1/D1)** — `/abi/<version>/` is hand-written prose today (no rustdoc→MDX pipeline exists anywhere in `docs-site/`); only a single `current` version exists (no archive). Direct violation of the ratified D1 ruling ("MUST be generated from rustdoc JSON → MDX, never hand-written"). Deferred to a tracked follow-up story: build the generation pipeline from `maos-spirit-abi` rustdoc JSON + the archive strategy.
  - **RESOLVED into stories at 9.5c preflight (2026-06-16, party-mode Winston·Paige·Murat·Amelia, Lunarpulse approved).** Split on coupling + gate-lifecycle: **9.5c** = CORE generation pipeline (xtask `gen-abi-docs`, hand-rolled serde + `format_version` assert, `.md` not `.mdx`, scoped nightly) + CONTENT richness-preservation (port curated prose into `maos-spirit-abi` doc-comments so generation doesn't regress docs — the original "zero crate delta" claim was FALSE; it's additive doc-comments + doctests) + anti-rot/value-provenance/cross-gate-contract gates + parity-gated atomic cutover; writes to the flat `/abi/` path. **9.5d** (expanded) = versioned `/abi/v1/` URL-space + 301 redirects + version dropdown + ADR-048 D6 deep-link contract, **unified with** its Playwright behavioral proof (config + gate ship together — no orphaned gate). **9.5c BLOCKS 9.5d.** "≥2 archives" stays honestly unmet pre-1.0 (only v1); the freeze mechanism lands in 9.5d, the count fills as `ABI_VERSION` bumps accumulate.
- **D5 — proven-red tempdir-isolation refactor (D8)** — proven-red mutates production files in-place because the gates read fixed paths and accept no override; `runExpectFail`/`getOverrideEnv` are dead code. P-safety (applied in 9.5) adds try/finally + fail-if-zero-tests + removes the dead helpers. Deferred: refactor gates to accept a path/env override so proven-red runs in real tempdirs (D8-faithful).

## Deferred from: code review of story-9.7 (2026-06-17)

- **`/tmp` fallback for `queue.json` when `HOME` is unset** — `dirs_fallback()` writes under `/tmp/.local/share/maos/skills/`, a world-writable dir; operator admission state becomes world-readable/mutable. Mirrors the existing `maos-registry` `dirs_fallback` convention; systemic XDG/`MAOS_DATA_DIR` resolution aligned with `default_transparency_log_path()` is a separate effort. `crates/maos-skill/src/store.rs:240-244`
- **`default_transparency_log_path()` hard-exits the CLI (`std::process::exit(2)`) on empty `MAOS_AUDIT_DB`** — pre-existing in `maos-audit`, newly reachable from `skills list/approve/reject` (via reconcile + decide). A single misconfigured env var terminates the CLI from inside a path-resolution helper, bypassing the typed `ESkillStore→ExitCode` flow and surfacing no `SkillsArgs`-level error. `crates/maos-audit/src/lib.rs:853-858`

## Deferred from: code review of story-9.7 re-review (2026-06-17)

- **Audit `actor` is unauthenticated env data** — `$USER` and `--actor` are forgeable with no OS-identity binding (e.g., `USER=ceo maosctl skills approve ...`). Systemic CLI audit-identity design, not introduced by 9.7; a real identity binding belongs to a broader authn/authz story. `crates/maos-cli/src/subcommands.rs:194-201`
- **`from_stored` rewrites unknown `entry_path` labels to `PackageShipped`** — pre-existing enum limitation; `RevisionProposal` cannot be reconstructed from its string label, so the round-trip is lossy. `crates/maos-skill/src/admission.rs:286-287`
- **`query_approvals` ordering semantic change** — changed from `timestamp_ns ASC` to `decision_id ASC`; intentional per Review #5/R8 to eliminate non-monotonic-clock LWW hazard, but it is a behavior change to a public read API. `crates/maos-iac/src/adapter/transparency_log.rs:1305-1307`
- **Reconcile keys on raw target string, `parse_approval_target` removed** — intentional per Review #13; safe today because `SkillId` charset excludes `@`, but it abandons the bidirectional "cannot drift" guarantee R6 originally sought. `crates/maos-cli/src/subcommands.rs:334`
- **`entry_path` provenance fidelity for CLI-discovered skills** — filesystem discovery (`discover_skills_detailed`) has no provenance signal, so 9.7 caches discovered skills as `package_shipped`. Faithful provenance (`AuthorSelf`/`RevisionProposal`) is an enqueue-time concept owned by the Epic-10 F6b/R8 daemon-enqueue seam. **Constraint:** any future daemon admission-enforcement logic must key on the TL decided-set (approve/reject capability rows), NEVER on the cached `entry_path` label. Add a pinning test that fails when discovery gains a provenance signal or the daemon starts writing enqueue rows, forcing the follow-up. `crates/maos-cli/src/subcommands.rs:425-430`

## Deferred from: code review of story-10.2 (2026-06-21) — RESOLVED pre-completion

Both items below were resolved during story 10.2 completion (per Lunarpulse).
Extracted to `xtask/src/gate_common.rs` and applied to all 4 gate modules:

- ✅ **Date validation** — now uses `chrono::NaiveDate::parse_from_str("%Y-%m-%d")` in `gate_common::validate_dates`; rejects impossible dates (`2026-99-99`), enforces `start <= end` ordering.
- ✅ **`--json` mode workflow commands** — `gate_common::emit_command` documents the stderr/stdout split; structured warning/error fields in the JSON payload (`advisory`, `failures`, `consistency_ok`) let programmatic consumers assert on JSON, not stderr.

## Deferred from: code review of 10-3-close-v1-0-compliance-gates-export-control-fuzz-hardening-korean-docs-cna-registration (2026-06-22)

- **Unmaintained `serde_cbor` 0.11 (RUSTSEC-flagged) re-used as a new fuzz-crate dependency.** Story 10.3 wire-protocol fuzz crate `crates/maos-domain/fuzz/Cargo.toml` depends on `serde_cbor = "0.11"` per ratified preflight N6 (fuzz harness MUST use the same CBOR crate as production code; `maos-compliance`/`canonical_cbor.rs` already depends on `serde_cbor` 0.11). The crate is deprecated and carries a known amplification DoS (mitigated operationally via `ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0` + `-rss_limit_mb=0`, documented in `docs/runbooks/fuzz-cadence.md`). Resolution = migrate `maos-compliance` canonical-CBOR off `serde_cbor` → `ciborium` (already used by `maos-kernel-core`) and update the fuzz harness to match — a supply-chain/modernization effort that spans production code, out of 10.3's docs/fuzz-i18n scope. Owner: future hardening story; flag to Winston/security.

## Deferred from: Story 10.3 code review — export classification counsel confirmation (2026-06-22)

- **Export-compliance counsel confirmation before v1.0 enterprise distribution.** Story 10.3 corrected the engineering self-classification citation in `STABILITY.md` §Export and `docs/compliance/eccn-classification.md`: the classification basis is now the "ancillary cryptography" Note to ECCN 5D002.c.1, while 15 CFR §740.13(e) is scoped to the open-source-software / License Exception TSU aspect. The team consensus (Winston·Murat·John·Mary, 2026-06-22) was **A-now + C-distribution-gate**: the citation correction is a verifiable regulatory-text read, but "MAOS qualifies for EAR99" remains a legal applicability opinion. **Gate:** export-compliance counsel must confirm (or amend) the EAR99/5D002 determination before v1.0 enterprise distribution. This does not block code completion, but it blocks enterprise distribution materials and final legal sign-off.
## Deferred from: code review of 10-4a-postgres-pgvector-loom-lite-collective-tier-and-sqlite-postgres-migration (2026-06-22, Chunk A)

- **`LoomLiteAdapter::handle.block_on` latent deadlock under tokio blocking-pool saturation** [crates/maos-loom-lite/src/adapter.rs write/read/scan impls]. Each sync port method parks a `spawn_blocking` worker via `Handle::block_on` for the whole async op; the blocking pool is bounded (default 512). A burst of collective-tier calls that saturates the pool, where parked futures themselves need blocking threads, can deadlock. This is the pre-existing topology risk acknowledged in ratified preflight §3 (the async boundary lives in `maos-loom-lite`, never kernel-core); no AC covers pool-saturation deadlock at v1.5. Monitor at scale; consider a bounded semaphore / timeout budget around collective-tier calls if concurrency rises. (The related panic-on-runtime-shutdown gap is filed as a Chunk A patch, not deferred.)
- **J — Kernel-port-sig Provenance threading** (`crates/maos-domain/src/ports/collective_memory.rs` `write` sig). I11 is enforced NOW via a Postgres store-layer CHECK constraint (`kind='pattern' => source_log_ref<>'' AND distillation_depth>0`). The kernel-sig threading (adding a `Provenance { kind, source_log_ref, distillation_depth }` param) is DEFERRED to the pattern-distillation story — threading `distillation_depth` through the kernel would blur ADR-006 ("kernel learns nothing"; distillation depth is a Loom concept) and force a speculative FLAG-Winston re-pin (YAGNI). Party-mode consensus 3/4 (Winston/Murat/John vs Amelia); the CHECK constraint makes a future violation un-mergable.
- **R — pgvector/HNSW similarity-search + embedding population** (`crates/maos-loom-lite/src/store.rs`, `schema.rs`). AC1's operational clauses are KV mediation + I9 + transport — no similarity-search requirement; populating embeddings needs an embedding-provider (excluded from the kernel per ADR-006) with no v1.5 consumer. DEFERRED to a named pattern-retrieval/distillation story; v1.5 ships KV-only. Document staging in the story + ADR; no gate/proven-red may claim similarity-search works at v1.5. Party-mode consensus 4/4.
- **HNSW session `SET`s apply to ONE pooled connection, not the whole pool** (`crates/maos-loom-lite/src/store.rs` `init_schema`). `init_schema` runs `SET hnsw.iterative_scan='relaxed_order'` on one checked-out connection; lazily-created pool connections run with the default (off). No per-connection init/recycle hook is wired. DEFERRED WITH R as latent-until-embeddings (no v1.5 op issues a vector query); wire a deadpool `Manager`/`recycle` per-connection init when embeddings land.
- **AF — At-scale 4h-breaching RTO-timing falsifiability** (`xtask/src/check_rto.rs` drill). At v1.5 scale (10⁶ rows) no restore of either SQLite or Postgres approaches the 4h SLA — the 4h target is a v2.0-capacity-envelope NFR (CUT to v2.0 in the 10.4 preflight). 10.4a lands the Postgres collective-tier drill (representativeness) + the §A1 timing-branch gate-mechanics proven-red (injected delay / threshold=0 → RED, falsifiable in principle) + the surrounding RTO patches. True 4h-breaching falsifiability DEFERRED to v2.0; v1.5 RTO timing documented as nominal. Party-mode consensus 3/4 (Winston/Murat/John vs Amelia).

## Deferred from: code review of 10-4a re-review (2026-06-23)

- **W1 — B18 per-row INSERT performance (~300s for 10⁶ rows)** [crates/maos-loom-lite/src/migration.rs]. Acknowledged correctness-OK. COPY batch optimization tracked as future performance improvement. Pre-existing design choice; ~300s functional at engagement scale.
- **W2 — Manifest corpus pins derived from same generator — not independently anchored** [tests/corpora/MANIFEST.toml]. No production TL exists yet at v1.5. Generator is deterministic; pins are internally consistent. Document limitation for future production-sample anchoring when real TL data exists.
- **W3 — AC2 live cross-backend tests are #[ignore]-only — no CI Postgres service** [crates/maos-loom-lite/tests/migration_live.rs]. Skipped-not-silent-PASS semantics are honest. Missing piece is a scheduled live Postgres environment in CI. Pre-existing infrastructure gap; not a code defect.
- **W4 — frames_25k theatrical for in-memory proven-red vectors** [xtask/tests/story_10_4a_proven_red.rs:37-43]. Batch-boundary coverage genuinely met by migration_live.rs (#[ignore]). In-memory vectors don't exercise batching despite header claim. Minor documentation inaccuracy.
- **W5 — RPO≤1h not independently gate-enforced on weekly cadence** [xtask/src/check_rto_gate.rs]. Drill folds `rpo_ok` into `passed` (immediate fix applied), but the weekly gate only checks drill_success + rto_seconds. RPO enforcement is drill-scoped, not gate-scoped. Minor gap.

## Deferred from: code review of 10-4c-j4-j6-real-kernel-measurement-harness-rebuild (2026-06-24)

- **2-of-3 reproduce-to-block de-flaking control (AC3/D2)** [crates/maos-bench/src/harness/j4.rs; t_10_4c_j4_latency_gate.rs]. The spec-mandated "a RED must reproduce 2-of-3 in-process passes before the gate fails" retry loop is unimplemented — the J4 measurement runs once per test. *Reason deferred:* at HEAD the measured P95 is ~1µs vs the 10ms budget (~10000× headroom), so a flake-induced false-RED is implausible near-term; revisit when J4 latency approaches the budget. The absolute-budget gate (AC3) and the Gate-1 mutation falsifier (AC2) remain load-bearing every PR.