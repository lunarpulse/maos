## Deferred from: Story 10.5 R4-only re-review (2026-06-29) — non-blocking hardening follow-ups (GO logged these, did not block)

> Surfaced by the §A2/§A6 R4-only re-review (run `wcw24wqlm`, unanimous GO). All three are hardening items the synthesis lead explicitly **dismissed as 10.5 GO blockers** after independent re-confirmation — filed here so the gaps are owned, not assumed-closed. See `story-10-5-r4-rereview-2026-06-29.md`.

- **cargo-deny is effectively NON-BLOCKING.** `cargo deny check` runs as a step **inside** the `reproducible-build` job, which is `continue-on-error: true` (the aggregate's `contains(needs.*.result,'failure')` check is neutralized for a continue-on-error job — proven in the Epic-10 5th-push iteration). So the "NON-NEGOTIABLE" supply-chain dependency-closure gate (10.4b) is currently advisory. Pre-existing (continue-on-error dated 2026-06-12, FLAG-Winston); masks nothing **at HEAD** because `cargo deny check` genuinely passes (exit 0). **Action:** move `cargo deny check` into its **own** `continue-on-error: false` job so a future supply-chain failure cannot be silently swallowed. **Ownerless and open.** *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-owner sweep).* `Winston/John` was a review-panel attribution, not a pageable sprint-status key. Re-measured at HEAD: still true — `.github/workflows/discipline.yml:38` keeps `continue-on-error: true` on the job that runs `cargo deny check`.
- **windows-check sandbox step has no vacuous-green guard.** The step `cargo test -p maos-kernel-core --test sandbox_enforcement_windows ...` has no `>=1 test ran` assertion. On windows-latest `#![cfg(target_os="windows")]` is satisfied so all 6 tests run today; but a future cfg/target drift compiling the suite to **zero tests** would `exit 0` silently. **Action:** add a `test result: ok. [1-9]` grep guard, the idiom already used by `check-j4-latency`. **Ownerless and open.** *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-owner sweep).* `test-infra` names no story in sprint-status.
- **Epic 11 must carry REAL ja/zh-Hans + a language-identity gate.** AC5 i18n is honestly DESCOPED to v2.0/Epic 11 (Korean placeholders + DO-NOT-SHIP markers; coverage+glossary gates report-only because they tautologically pass on wrong-language content). Epic-11 Story 11.6 must add real translations **and** a language-identity gate that detects wrong-language content (Hangul-in-ja, etc.) — coverage/glossary-lock provably cannot. **Ownerless and open.** *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-owner sweep).* Epic 11 and its retrospective are both `done`; the language-identity gate was never claimed by a successor.

## Deferred from: Story 9.5b preflight party-mode Round 2 (2026-06-16) — OTel spans are a new telemetry data-sink the GDPR erasure scope must acknowledge

> Surfaced by Murat in the 9.5b OpenTelemetry-adapter preflight (Winston·Amelia·Murat·John, ratified Lunarpulse). Cross-story finding — out of 9.5b's own ACs by design, filed here so the gap is owned, not assumed-closed.

- **OTel span attributes are a 4th telemetry surface the forget cascade cannot reach.** The GDPR Art.17 forget cascade reaches exactly `REGISTERED_ERASURE_BACKENDS = ["private","principal_index","shared"]` (`crates/maos-kernel-core/src/memory/mod.rs:35`). Span attributes emitted by the new `maos-telemetry` adapter are a NEW data sink outside that set. Story 9.5b mitigates this **by construction** via `gate:otel-attr-contract` (AC-5): spans carry **zero principal/subject nexus** → erasure-exempt by construction, mirroring `governance.rs:122` ("zero principal nexus → stays OUT of the forget cascade"). So spans are NOT wired into the cascade, and the attr-contract gate prevents a future contributor from silently adding a PII key.
- **What's deferred / owned elsewhere:** the *existence* of this telemetry surface should be acknowledged in the **GDPR erasure scope documentation / data-sink inventory** documented by Stories 9.2 / 9.2b (forget cascade, both `done`) — **Ownerless and open** for the acknowledgement itself, *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-owner sweep).* and — not just constrained inside 9.5b's test gate. Action: when the 9.5b adapter lands, add "OTel span attributes (erasure-exempt by zero-principal-nexus construction; enforced by `gate:otel-attr-contract`)" to the erasure-scope/data-sink inventory so the exemption is a documented invariant, not an implicit one. Flag to the 9.2 erasure owner (John/Winston). E1b ComplianceClaim schema is unchanged (no new claim field) — this is scope-documentation, not a claim amendment.

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

> Two-round adversarial audit (party-mode: John / Winston / Murat / Amelia + Mary) of whether Epic 8 delivers the PRD user journeys. Surfaced **NEW security/invariant defects not previously logged**. Each was tracked to completion by the delivery stories 8.9–8.15 (all `done`), registered via `sprint-change-proposal-2026-06-06.md`. Listed here for canonical defect tracking.

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
  frame injection under load) was assigned to Story 6.3 (A2A bilateral mTLS, `done`) at v0.5+, and is **Ownerless and open** today. *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-owner sweep).*
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
> **GENERATION-ABORT HALF CLOSED 2026-06-26 (Epic 10 retro §A4) — VERIFIED.** Both
> `templates/spirit-{rust,ts}` now generate clean on cargo-generate **0.23.11**
> (`cargo generate --path templates/spirit-rust --name test-spirit --define class_name=MySpirit --silent`
> → Done; spirit-ts produces a valid `"name": "@local/test-spirit-ts"`). Fixes applied:
> dropped the reserved `crate_name` placeholder (both), removed the dead `[hooks]
> post-generate.rhai` reference (both), and — found by actually running it, beyond the
> notes below — fixed ts `package_name` (0.23 does NOT interpolate `default` values and
> regex-validates them as literals; dropped the placeholder, `package.json` now uses the
> built-in `{{project-name}}` directly). REMAINING (do NOT flip `smoke-spirit-author-7-1`
> to blocking until closed): the TS npm-publication blocker below. Once that lands, run the
> smoke green and graduate the gate.
The author-side scaffolding has bit-rotted since Story 7.1 and was never CI-validated
(main had no CI run from the 7.1.5 freeze through Epic 8). `smoke-spirit-author-7-1` is
now ADVISORY (continue-on-error in discipline.yml) until a dedicated story closes:
- ~~**cargo-generate 0.23 compat**: reserved `crate_name` placeholder aborts generation.~~ FIXED+VERIFIED 2026-06-26 (§A4).
- ~~**Missing hook file**: dangling `[hooks] post = ["post-generate.rhai"]`.~~ FIXED 2026-06-26 (§A4, `[hooks]` block removed; inline `[template.scripts]` retained).
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
- **`entry_path` provenance fidelity for CLI-discovered skills** — filesystem discovery (`discover_skills_detailed`) has no provenance signal, so 9.7 caches discovered skills as `package_shipped`. Faithful provenance (`AuthorSelf`/`RevisionProposal`) is an enqueue-time concept belonging to the Epic-10 F6b/R8 daemon-enqueue seam, and is **Ownerless and open**. *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-owner sweep).* **Constraint:** any future daemon admission-enforcement logic must key on the TL decided-set (approve/reject capability rows), NEVER on the cached `entry_path` label. Add a pinning test that fails when discovery gains a provenance signal or the daemon starts writing enqueue rows, forcing the follow-up. `crates/maos-cli/src/subcommands.rs:425-430`

## Deferred from: code review of story-10.2 (2026-06-21) — RESOLVED pre-completion

Both items below were resolved during story 10.2 completion (per Lunarpulse).
Extracted to `xtask/src/gate_common.rs` and applied to all 4 gate modules:

- ✅ **Date validation** — now uses `chrono::NaiveDate::parse_from_str("%Y-%m-%d")` in `gate_common::validate_dates`; rejects impossible dates (`2026-99-99`), enforces `start <= end` ordering.
- ✅ **`--json` mode workflow commands** — `gate_common::emit_command` documents the stderr/stdout split; structured warning/error fields in the JSON payload (`advisory`, `failures`, `consistency_ok`) let programmatic consumers assert on JSON, not stderr.

## Deferred from: code review of 10-3-close-v1-0-compliance-gates-export-control-fuzz-hardening-korean-docs-cna-registration (2026-06-22)

- **Unmaintained `serde_cbor` 0.11 (RUSTSEC-flagged) re-used as a new fuzz-crate dependency.** Story 10.3 wire-protocol fuzz crate `crates/maos-domain/fuzz/Cargo.toml` depends on `serde_cbor = "0.11"` per ratified preflight N6 (fuzz harness MUST use the same CBOR crate as production code; `maos-compliance`/`canonical_cbor.rs` already depends on `serde_cbor` 0.11). The crate is deprecated and carries a known amplification DoS (mitigated operationally via `ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0` + `-rss_limit_mb=0`, documented in `docs/runbooks/fuzz-cadence.md`). Resolution = migrate `maos-compliance` canonical-CBOR off `serde_cbor` → `ciborium` (already used by `maos-kernel-core`) and update the fuzz harness to match — a supply-chain/modernization effort that spans production code, out of 10.3's docs/fuzz-i18n scope. **Ownerless and open.** *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-owner sweep).* “future hardening story” named nothing pageable; no successor has claimed the `serde_cbor` → `ciborium` migration.

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

## Deferred from: code review of 12-6-env-contract-registry-remediation (2026-07-13)

> Two FKCS-oracle findings surfaced by the 12.6 code review (Blind Hunter + Edge Case Hunter). Both are pre-existing to this story's diff — the 12.6 change touched only doc comments in those regions — so they are owned here, not treated as 12.6 blockers.

- **FKCS diff-oracle leg self-compares live surfaces** [xtask/src/check_fkcs.rs:140-148,334-364] — `capture_from_baselines` captures the LIVE ABI/host surfaces and `run_diff_oracle_derives_leg` derives its verdict from that snapshot compared to itself. The leg's purpose is to prove the oracle DERIVES `kernel_unchanged` and IGNORES a forged self-report (not to guard live ABI drift), and `validate_live_triple` only checks the ABI/host baseline files exist and are non-empty. An ABI/host regression can therefore still derive green on this leg; the real additive-only ABI guard lives outside FKCS (`cargo public-api --diff` / abi baseline). Pre-existing; untouched by 12.6.
- **FKCS frozen-tag leg uses CWD-relative kernel-baseline paths** [xtask/src/check_fkcs.rs:94-106] — `validate_live_triple` → `check_kernel_baseline::check()` opens `xtask/kernel-core-baseline.toml` and `crates/maos-kernel-core/src` relative to the CWD, so running the gate from a workspace subdirectory (e.g. `crates/maos-bin`) fails with `No such file or directory`. 12.6's new `workspace_root()` helper hardened only the `git()` invocation, not this path resolution. Not triggered in CI (jobs run from the workspace root). Pre-existing.
## Deferred from: code review of 13-5d-production-spirit-collective-route (2026-07-19)

- **Fallible `record_invocation` (audit-channel drop returns `Ok`)** [crates/maos-kernel-core/src/capability/mod.rs:332-351] — `CapabilityRegistryAdapter::record_invocation` treats a full-channel `try_send` failure as a dropped event and still returns `Ok(())`; a saturated audit channel silently loses the route-level `CapabilityInvocation` that AC5's correlation join reconciles against. The only real fix is a fallible invocation path, which is kernel-core work outside the L10 FLAG-Winston grant (fence item 2: do not spend the grant beyond the pid check). *Reason deferred:* requires a second FLAG-Winston escalation; Owner: `epic-14` — Epic-14 preflight, per the Epic-13 retrospective §4 disposition (2026-08-11). *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-owner sweep).* “the next kernel-touching story” has been overtaken FOUR times (13.5h, 13.5i, 13.5j, 13.6c all shipped kernel deltas without taking it), so the phrase now names nobody; the retro decides who carries kernel-core debt. Party-mode consensus 2026-07-19. The story's interim mitigations: audit-first ordering at the composition root + a drop-counter assertion on the `mediated-operation-correlation` gate leg + a may/may-not-table honesty line.
## Deferred from: code review of 13-3-cross-team-asymmetric-consent-multi-hop-distillation-provenance (2026-07-20)

- **Consent adapter ignores local-host membership** [crates/maos-bin/src/cross_team_consent.rs:23-38] — a fresh signed reissue can remove `state.local_host()` from `manifest.members` while retaining the teams and a matching `[[cross_team_consent]]` grant; `CrossTeamConsentAdapter::is_granted` checks only lease freshness + the grant and returns true, and the deliberately unguarded replication apply path then lands rows on an evicted host. *Reason deferred:* party-mode consensus D2 (2026-07-20) — the consent port is team-axis by ratified design (H2/AC2); host-membership is the host axis and belongs to the daemon that ADR-055 §5 assigns verbatim to Story 13.5c. CLOSED at the Epic-13 retrospective §4 (2026-08-11): the successor identity and endpoint work shipped in 13.6a/13.6b, and the 13.5c successor is `done`, so no live Epic-13 assignment survives. The recorded gap — "an evicted host holding a fresh lease still consents" — stays as an audit trail, not an open item. *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-ownership sweep).* Story 13.5c is `done` and closed WITHOUT a membership answer, so the reopen condition it named has fired.

## Deferred from: code review of 13-3b-provenance-crosses-the-wall (2026-07-21)

- **`deserialize_receipt` silently drops malformed nested refs** [crates/maos-iac/src/adapter/distillate.rs:163-170] — `source_log_ref` entries are parsed with `filter_map`, so an unparseable nested reference vanishes and a new digest can be written with an empty effective audit chain. *Reason deferred:* pre-existing — the `filter_map` predates this diff; 13.3b touched `flatten_source_log_ref` (diamond fix), not receipt deserialization.
- **Receipt `distillation_depth` truncates via `as u32`** [crates/maos-iac/src/adapter/distillate.rs:154-161] — a nested receipt carrying a depth > u32::MAX wraps instead of erroring; no `u32::try_from`/`checked_add`. *Reason deferred:* pre-existing cast; no 13.3b code path can produce such a receipt (depths originate from `effective_depth = max_seen + 1` over local traversal).

### Round 2 (post-rework review, 2026-07-21)

- **Successful cross-wall recalls journaled as plain local `log.recall`** [crates/maos-iac/src/adapter/log_recall.rs:281-288,370-377] — `recall_cross_wall` delegates to `recall` after consent, so the `CapabilityInvocation` audit row carries only limit/cursor metadata: a cross-wall disclosure is indistinguishable from a local recall and does not record which directional grant was exercised. *Reason deferred:* ADR-058 Decision 2 explicitly scopes journaling out ("Per-team TL isolation and refusal journaling remain outside this decision"); the dead-wire negative named 13.5e for refusal journaling. **CLOSED by Story 13.6d** — verified 2026-08-08: `recall_cross_wall` no longer delegates to `recall`; it journals `disclosing` before movement and `disclosed`/`failed` after, under the distinct intent `log.recall.cross-wall` (`crates/maos-iac/src/adapter/log_recall.rs:52`, `:403`, `:421`, `:424`, `:428`), and `cross_wall_recall_refusals_and_disclosures_are_journaled` (`crates/maos-bin/tests/cross_team_consent_13_3.rs:444`) pins the outcome pair. *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-owner sweep).*

## Deferred from: code review of 13-5b-collective-tier-erasure-legal-hold-cascade (2026-07-25)

> Six findings from the 13.5b three-layer review (Blind Hunter + Edge Case Hunter + Acceptance Auditor). All are pre-existing to this story's diff or explicitly ratified by its ACs/ADR, so they are owned here rather than treated as 13.5b blockers.

- **Legal-hold check-then-act race with deletion** [crates/maos-kernel-core/src/memory/mod.rs:470-498,571-573] — `forget_with_reason` consults `is_under_legal_hold` and releases the authority lock before any mutation; the private/index deletes happen ~100 lines later with no spanning transaction or recheck. A `place_legal_hold` that lands in that window is ignored and the principal erases despite an active hold. Each hold operation takes its own SQLite lock (`crates/maos-iac/src/adapter/transparency_log.rs:1155-1187`). *Reason deferred:* the check/mutate ordering predates this diff; 13.5b changed the orchestrator's handling of `Suspended`, not the kernel's hold-then-delete sequence. Closing it needs a hold-scoped transaction or an optimistic recheck in kernel-core — kernel-core lines, outside the ratified ZERO fence. Owner: `epic-14` — Epic-14 preflight, per the Epic-13 retrospective §4 disposition (2026-08-11). *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-owner sweep).* The `13-5h` candidacy lapsed: that story is `done` and did not take the hold-scoped transaction.
- **`revoke_all_for_pid(...).unwrap_or(0)` feeds a signed `CategoryStatus::Removed` count** [crates/maos-bin/src/main.rs:8168-8169,8217-8221] — a failed revocation is indistinguishable from "zero tokens existed", and the difference is then signed into the proof bundle as `capability_tokens: Removed { count }`. Exactly the D-1/D-4 genre this story exists to close, on a line the diff did not touch. *Reason deferred:* pre-existing; the honest fix is a `CoverageGap` on revocation failure, which reopens the AC1 vocabulary decision now pending on the region-pinned terminal.
- **`decommission_region_key` hardcodes `completed: true`** [crates/maos-audit/src/erasure/regional_teardown.rs:116-124] — it derives a public key from the still-supplied `base_seed` and asserts completion; no key material is revoked or destroyed. The signed regional receipt therefore attests a decommission that never happened. *Reason deferred:* pre-existing (Story 9.4b); currently unreachable in practice because phase (a) fails first on the `shared` coverage gap. Becomes live the moment the pending AC1 decision restores a `completed`-capable forget attestation — re-open then.
- **Crash between durable mutation and audit append leaves no reconciliation record** [crates/maos-kernel-core/src/memory/mod.rs:571-589; crates/maos-loom-lite/src/store.rs:1772-1823 → crates/maos-bin/src/main.rs:4862-4877] — both the forget cascade and the operator collective erase commit destructive state before their audit frame, and `insert_kernel_event_returning_id` panics on write failure by design (I2 binding). No startup repair or persisted reconciliation job exists; the one-sided-erase plant in `xtask/tests/story_10_4a_ac1_proven_red.rs:438-447` is a pure boolean helper that never observes production stores. *Reason deferred:* 13.5b took an explicit position (Trap 4 fail-fast, ADR-059:58-62) and registered mutation-to-audit crash atomicity as ownerless and open. Recorded here so it does not disappear.
- **Skipped `AdvisorySubstrate` legs still emit `passed: true`** [xtask/src/check_reza_production_path.rs:474-545] — when the two-datname Postgres environment is absent the live legs are not attempted, yet `passed` is `blockers.is_empty()` and the gate exits zero with a WOULD-HAVE-BLOCKED warning. The story's own testing standard says absent substrate must be UNMEASURED, never green. *Reason deferred:* this is the shared `gate_common` house pattern across every substrate-bound gate, not a 13.5b regression; CI provisions pgvector/pg16 for this job so the legs do run and do block there. Fixing it is a cross-gate change.
- **`MAOS_ONE_SHOT` legal-hold-release and collective-erase carry no operator authorization boundary** [crates/maos-bin/src/main.rs:4799-4828,4831-4877] — both accept a principal id / pid+namespace+key from environment variables and act, with no operator identity, capability, or approval check; `team_guard` is tenant placement, not authority. *Reason deferred:* AC4(a) explicitly mandated reusing the `MAOS_ONE_SHOT` dispatch idiom, and every other one-shot verb has the same shape — exec rights on the host binary already imply operator authority. Revisit if operator-axis work (NFR-Ops-11) ever lands.

### Round 2 (post-patch review, 2026-07-25) — raised BY the review's own proven-red work

- **Private-tier filesystem residue survives the forget cascade** [crates/maos-kernel-core/src/memory/private.rs:319-337,183-188,30-36] — `forget_principal` derives its removal set exclusively from the in-memory map, but the private tier deliberately does not cache `MemoryValue::Markdown` (it is filesystem-canonical so operator hand-edits stay visible) and always spills it to disk. The Markdown record is invisible to the removal set, `fs::remove_dir_all` never runs, the file survives — and the signed proof records `memory_namespace` as `Removed { count: 0 }` while `subject_access_query` reports the principal gone: an Article 15/17 asymmetry that hides its own residue. The same hole swallows any value above the 4 KiB spill threshold once the writing process exits, because `PrivateMemoryStore::new` never hydrates from `fs_root` — which describes every real operator uninstall. This is D-4's defect surviving D-4's fix: 13.5b corrected the *count*, but the *enumeration source* upstream of it was already wrong. *Reason deferred:* the correction means walking `fs_root` inside `forget_principal` — kernel-core lines, outside 13.5b's ratified ZERO-Δ fence. Trap 1(ii) says escalate rather than absorb. **FLAG-Winston, ownerless and open**; recorded as ADR-059 Decision 10 / Residual 8. Pinned by `private_tier_markdown_survives_the_forget_cascade` and bound as the Blocking Reza leg `gdpr-private-markdown-residue-pinned`, so a successor's fix goes RED and forces the proof category to be corrected with it. **CLOSED by Story 13.5i** — verified 2026-08-08: `forget_principal` now walks the private spill tree instead of the in-memory map (`crates/maos-kernel-core/src/memory/private.rs:811`, `forget_principal_unix` `:827`, `forget_principal_nonunix` `:926`), and the pinning leg `gdpr-private-markdown-residue-pinned` no longer exists in `xtask/src` because the residue it pinned is gone. *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-owner sweep).*

## Deferred from: code review of 13-5i-private-tier-filesystem-residue (2026-07-27)

- **TOCTOU: the checked pid directory can be swapped for a symlink before `read_dir`** [crates/maos-kernel-core/src/memory/private.rs:374-382] — `forget_principal` validates `pid_entry.file_type()?.is_dir()` and then reopens the path *by name* at `:382`; no directory handle is retained. A process that replaces `<fs_root>/<pid>` with a symlink between those two operations makes the subsequent `read_dir` and `remove_dir_all` resolve outside `fs_root`. *Reason deferred:* requires a concurrent local writer inside the operator-owned memory root during an uninstall, and a real fix needs descriptor-anchored `openat(O_NOFOLLOW)` traversal — a new dependency and a materially larger kernel delta than any bounded repair 13.5i could carry.
- **A write concurrent with `forget_principal` can survive it** [crates/maos-kernel-core/src/memory/private.rs:326-355,368-410; :177-197] — there is no operation-wide exclusion between `write` and `forget_principal`. A target write landing after the `to_remove` snapshot stays in `in_mem`; a spill landing after `remove_dir_all` recreates the namespace directory. Either way the forget returns success and the store can still serve target data. *Reason deferred:* pre-existing — the lock discipline is unchanged in shape by 13.5i — and the production forget path is a one-shot CLI process with no Spirit running (story D-1).
- **Directory entries are unlinked while their parent's `ReadDir` is still being iterated** [crates/maos-kernel-core/src/memory/private.rs:399-421] — `fs::remove_dir_all(&ns_dir)` runs inside `for ns_entry in fs::read_dir(&pid_dir)`, and `fs::remove_dir(&pid_dir)` inside `for pid_entry in read_dir(&fs_root)`. POSIX leaves subsequent `readdir` results unspecified once the directory is modified after `opendir`, so on some backing stores a sibling namespace directory can be skipped — silent under-deletion under a success receipt, this story's own defect class. *Reason deferred:* [INFERENCE] not reproduced; glibc's `readdir` buffering makes it safe on ext4/tmpfs, which is what MAOS runs on today. Fix if revisited: collect entries into a `Vec` before deleting.

## Deferred from: code review of 13-5g-tl-stage2-datname-inversion-defense-in-depth (2026-07-27)

> One finding. Resolved by party-mode consensus (Code Review Crew, 5/5, criterion: per spec + long-term correctness). The room split the originally-filed TOCTOU in two and deferred only the adversary-grade half; the benign concurrent-boot half was escalated to a blocking patch inside the story.

- **TOCTOU: the Transparency Log artifact can be replaced between the Phase A verdict and the TL open** [crates/maos-bin/src/main.rs:2444,2649; crates/maos-audit/src/lib.rs:1056-1068] — nothing carries a file descriptor, inode identity, or SQLite snapshot from `phase_a_preflight` to `open_with_global_legal_holds` or to the subsequent `write_tenant_binding`. `read_tenant_artifact` itself does `symlink_metadata(path)` and then a *separate* `open_with_flags(path, ...)`. `SQLITE_OPEN_NOFOLLOW` rejects a symlink at each individual open but does not prevent a regular-file replacement between them, so an actor able to write the audit directory can have the process approve one artifact and open — or write a local binding into — another. *Reason deferred:* the exploit requires audit-directory write access plus arbitrary timing, which AC6's ratified honest limit already places outside the model ("detects misconfiguration, mis-restore and accidental substitution — not an adversary"). Closing it in-story would require carrying an fd/inode identity into the Transparency Log adapter, which lives in `maos-iac` — pinned at **zero delta** by this story's Budget table — and would stand up a second artifact-identity mechanism that residual #1 `v25-signed-shard` subsumes and deletes, since a signed genesis row travels inside the artifact and detects substitution regardless of timing. Recorded as **residual #6 in ADR-055 and explicitly assigned to `v25-signed-transparency-log-artifact-identity`** (the sprint-status key; `v25-signed-shard` was a nickname that resolved to nothing — *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-owner sweep).*), so it is owned rather than silently accepted. Structurally identical to the 13.5i private-tier TOCTOU deferred above, and deferred on the same reasoning. *Not deferred:* the benign sibling — two boots racing the `NeedsWrite` window, reachable through a config typo with no privilege — was split out and is a blocking patch on `write_tenant_binding` (`crates/maos-audit/src/lib.rs:1108-1121`).

## Deferred from: code review of 13-6d-cross-wall-recall-production-initiator (2026-07-30)

- **`consent_grant` audit field records the compile-time intent constant, not an actual grant id/version/lease** [crates/maos-iac/src/adapter/log_recall.rs:107] — `CrossWallRecallConsentDecision::Granted` is a unit variant carrying no metadata (`crates/maos-domain/src/ports/cross_wall_recall_consent.rs:17`), so the cross-wall disclosure row cannot name the specific manifest grant that authorized it; only the intent string (`log:recall`). The grant is reconstructable from (home_team [the artifact's own binding], remote_team, intent). *Reason deferred:* enriching the consent decision requires widening the consent port + adapter across crate boundaries; out of 13.6d's local scope. ACCEPTED RISK — ratified at the Epic-13 retrospective §4 (2026-08-11); no successor assignment, by decision. The authorization is reconstructable from the artifact's home team, remote team, and intent; widening the consent decision crosses crate boundaries and is not required to validate the published minimum-disclosure journey. *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-ownership sweep).* The prior record was self-admitting: it cited 13.6a — already `done` — as the live assignee on the day it was written.

## Deferred from: code review of 13-6c-three-team-three-region-substrate (2026-08-03) — RESOLVED

All three items (TI-1 fsync-failure rollback, TI-2 io_lock serialization, TI-3 no-follow-open TOCTOU swap) were re-applied the same day via a kloc-excluded `cfg(test/debug_assertions)` fault module (`memory/spill_test_faults.rs`, excluded by `check_kloc.rs -e`). Ceiling 18248 unchanged. Kept here as an audit trail of the defer→resolve path.

## Story 13.6e preflight — `roundtrip-slo` floor breach: MEASURED, DIAGNOSED and RESOLVED 2026-08-04

> The tracking entry the RED-at-HEAD contingency (`13-6c…md:154`, from 10.4c D6 `10-4c…md:82`) required and that did not exist. **Operator chose option (B) — reduce the cost rather than re-base the floor. The floor was NOT touched and no control was removed.** Recorded with the full measurement chain, including a first hypothesis that measurement killed.

### Outcome

`check-multi-region-slo` is **`oracle_green: true`**, all five legs green, `roundtrip-slo` `passed: 2` (live + mutation). Clean p95 **17 802 µs / 17 866 µs** across two runs against the untouched **30 000 µs** floor. One test file changed; zero production lines.

### What was actually wrong: the probe counted a verification step that had MOVED

`verify_replication_bundle` is **7 137 µs** per call in this debug `cargo test` build. That single fact dominates everything — component attribution for one leg (p50 µs):

| component | µs |
|---|---|
| `write_with_source` | 564 |
| `read_all_rows_from` + `from_row` | 193 |
| `build_replication_bundle` (Merkle + Ed25519 **sign**) | 230 |
| **`verify_replication_bundle` (1 Ed25519 verify)** | **7 137** |
| `apply_replication_bundle` (internal verify + 1 write) | 7 956 |
| `apply` **minus** its internal verify | **819** |

Four verifies per round trip × 7 137 = 28 548 µs of a 31 596 µs modelled round trip — **~90%**; modelled 31 596 vs measured 31 059, so the model closes. The 7 ms is the signature verification itself, not key derivation: `derive_region_pubkey` measures **102 µs** (HKDF alone 19 µs).

**Root cause.** When this probe was written at 11.2b, `apply_replication_bundle` did **not** verify, so an explicit `verify_replication_bundle` before `apply` was genuinely part of the production path. **Story 13.2 moved verification INSIDE `apply`** (`bundle.rs:866` — before any store access, the Fork-4 payoff). The probe was never updated, so from 13.2 onward it performed **two verifies per leg where production performs one**. Production's sole caller of `apply_replication_bundle` — `crates/maos-bin/src/cross_team_crossing.rs:255` — calls it directly with no pre-verify.

So the apparent **1.94×** regression was **~85% probe infidelity**. With the redundant verify removed, HEAD measures **17 802 µs against 11.2b's 16 535 µs = 1.08×** — ~8% genuine machinery growth over five stories, which is a healthy tripwire margin, not a regression. **Coverage is unchanged**: a bundle that fails verification makes `apply` return `SignatureVerificationFailed` and the `.expect` panics.

### A hypothesis that measurement killed — recorded because the reasoning was persuasive and wrong

The 5 → 17 SQL-statement expansion on the write path (`store.rs:619-708`: `BEGIN` + `pg_advisory_xact_lock` + erasure-tombstone `SELECT` + 18-column upsert + `COMMIT`) was the leading explanation, with arithmetic that appeared to fit. A probe harness measured it per-write (p50 µs, N=200):

| variant | p50 | vs baseline |
|---|---|---|
| V0 autocommit, unprepared (11.2b shape) | 141 | 1.00× |
| V1 full path, **unprepared** (HEAD shape) | 380 | **2.70×** |
| V2 full path, **`prepare_cached`** | 178 | 1.26× |
| V3 no advisory lock | 143 | 1.01× |
| V4 no tombstone `SELECT` | 143 | 1.01× |

**The controls are nearly free (~35 µs each); statement PARSING was the entire write-path cost.** But 3 writes × 239 µs = 0.7 ms against a 15.3 ms round-trip delta — it explained **under 5%** of the regression, which is what redirected the investigation to the verify count. *The arithmetic that "fit" fit because two wrong numbers were multiplied.*

### Residuals — open, and none of them block

- ~~**`cross_region_roundtrip_mutation`'s second assert is still non-discriminating**~~ — **CLOSED by Story 13.6e (T5) and the remaining evidence-producer review, 2026-08-04.** The old `p95 >= 14_000` passed below the clean baseline. The final oracle alternates adjacent clean/injected samples, computes each pair's saturating delta, and requires the paired-delta median to carry at least 14 ms of the fixed 15 ms injection. `paired_delta_oracle_requires_the_injection_on_a_majority_of_pairs` is the direct proven-red: an absent injection stays below 14 ms while the injected vector crosses it. The `roundtrip-slo` trusted mapping also requires both signed records — 29 ordinary gate proofs plus this mutation falsifier make 30 guards total.
- **`prepare_cached` is an unclaimed, measured production win** — `deadpool_postgres::Client::prepare_cached` takes the write path from 2.70× to 1.26× with zero semantic change (`store.rs` currently passes `&str` to `query_one`/`query_opt`/`execute`, re-parsing every call). Worth ~200 µs/write. Not taken here: it is production code in `maos-loom-lite`, it is not needed to clear the floor, and it deserves its own story rather than a benchmark-driven drive-by. **Ownerless and open.** *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-owner sweep).* `maos-loom-lite performance maintainers` is not a role that exists in sprint-status, and an owner nobody can page converts an honest ownerless row into an unfalsifiable one. **Close when a dedicated performance story either ships cached statements with the roundtrip tripwire unchanged or records a measured reason to decline the optimization.**
- **This gate's absolute numbers are debug-build-dominated and always were** — ~7.1 ms per Ed25519 verify in `cargo test` debug versus roughly two orders of magnitude less in release. The floor is a *loopback regression tripwire, not a geo-SLO* (`ADR-049…md:125`), so a debug-build constant is legitimate as long as it stays a constant — but any future change to build mode invalidates the floor's provenance. `cross_region.rs:61-64` already demands rig + build mode be recorded with any change to the constant; **the constant did not change here.**

### Also landed (required by the governing rule regardless of disposition)

The diagnostic `eprintln!` in **both** `cross_region_roundtrip_live` and `..._mutation` moved **above** their asserts. Every assert panics, so emitting the distribution afterwards meant a RED run reported p95 and nothing else — the failure's shape was unmeasurable by construction, which is why "is this jitter?" stayed an opinion for two days. The distribution is what settled it: std_dev 499 µs and p95−p50 = 1 005 µs around a p50 that was *itself* over the floor — a uniform shift, not a tail. Both gate legs already pass `--nocapture`, so this surfaces in CI with no workflow change.

---

## Deferred from: Story 13.6e — evidence ledger, AC1's explicit non-goal (2026-08-04)

> Recorded rather than silently skipped, per the story's AC1. The ledger covers the FOUR journey-relevant gates derived from `check_loom_substrate_drift`'s `CONTRACTS`. It does **not** cover the other eight Family-B gates, and it must not be read as "every gate emits an evidence state".

- **Eight Family-B gates are outside the evidence ledger** [`xtask/src/check_scale_churn.rs`, `check_cohort_mesh.rs`, `check_enterprise_pdp.rs`, `check_enterprise_identity.rs`, `check_escape_detector.rs`, `check_trial_attestation.rs`, `check_vetting_attestation.rs`, `check_wasm_form_equiv.rs`] — each carries its own `{label, passed, failed, ran, [attempted,] green}` leg struct with no `binding`, no `substrate_present` and no evidence state, so none of them emits a `product_claim` and none of them publishes a ledger artifact. *Reason deferred:* 13.6e's ledger set is **derived** from the four `CONTRACTS` entries precisely so it cannot quietly grow; widening it is scope, not budget. **Owner: `epic-14` — Epic-14 preflight, per the Epic-13 retrospective §4 disposition (2026-08-11).** (alongside the `maos-a2a-core` third-consecutive-unratified grant and the `xtask` decomposition question) — the retro should decide whether the ledger is a four-gate journey instrument or a workspace-wide one before anyone extends it.
- **Two of those eight cannot express `ABSENT` at all** [`xtask/src/check_vetting_attestation.rs:31`, `xtask/src/check_wasm_form_equiv.rs:104`] — their `LegResult` has **no `attempted` field**, so "the leg never ran" and "the leg ran and produced nothing" are the same value. A projection cannot distinguish `ABSENT` from `INDETERMINATE` there without a struct change. Both gates are hermetic today, which is why the hole has not bitten; it becomes real the moment either grows a substrate-bound leg. **Owner: `epic-14` — Epic-14 preflight, per the Epic-13 retrospective §4 disposition (2026-08-11).**, same decision.
- **`check-kernel-baseline` prints its PASSED line to stdout** [`xtask/src/check_kernel_baseline.rs`] — every gate that reuses it as a leg (`check-cross-region-consensus`, `check-multi-region-slo`) therefore emits one non-JSON line before its `--json` payload. Pre-existing and harmless in CI (both jobs `tee` rather than parse), and 13.6e's ledger artifact is written to a file so it is unaffected. *Reason deferred:* fixing it changes a shared gate's output contract. **Ownerless and open.** *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-owner sweep).* `xtask gate-infrastructure maintainers` is not a role that exists in sprint-status. **Close when `check-kernel-baseline --json` is machine-clean and both composite callers consume a quiet structured result without transcript parsing.**

### The RED that closing `check_fkcs.rs:325` uncovered — HELD ADVISORY, not fixed, not re-pinned

- **`check-fkcs`'s `admission-path-unmodified` leg has been RED since Story 13.4** [`xtask/fkcs-baseline.toml:22`, `crates/maos-registry/src/admission.rs`, `crates/maos-skill/src/admission.rs`] — 13.4 (FR37 vetting machinery, `148a33ee`) changed both admission sources and never re-pinned `admission_baseline.sha256`; the last re-pin was `5767cf0d` (Story 12.6). The gate's exit tested `blocking_now` while its JSON `passed` field tested `dev_blocks`, so it printed `"passed": false` and exited **0** — the one-identifier defect Story 13.6e's AC2 named. **Measured both ways at 13.6e:** with the original identifier the gate exits 0 over the same RED leg; with the fix it exits 1.
  - **The harness is correct.** The admission path genuinely is not unmodified. Re-pinning here would re-can a fixture over another story's unreviewed admission-path change, which the governing rule (`13-6c…md:154`) forbids: *"Never re-can a fixture, never silently relax a floor."*
  - **Disposition:** held advisory with a loud banner, a named owner and this entry. The hold is bound to the exact recorded baseline SHA-256 and current Story-13.4 worktree SHA-256; malformed baseline data, unreadable sources, or any later admission drift blocks instead of inheriting the hold. `check-fkcs` emits a `WOULD HAVE BLOCKED` banner and reports `held_advisory_legs` in JSON. The gate-level `advisory` field now matches its actual verdict.
  - **Owner: `14-3-ecosystem-readiness-verification-v2-5-graduation-ledger`** — handed by the Epic-13 retrospective §4 (2026-08-11): 14.3 owns FKCS infrastructure readiness and must decide the conformant re-pin or retain the exact bounded hold. *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-owner sweep).* The original record read “with Story 11.5 (FKCS infrastructure)”; Story 11.5 is `done`, so this residual was STALE AT BIRTH — authored inside the story that built the judge, naming a completed owner on the day it was written. The FKCS infrastructure it depends on is 11.5's shipped output, not a live owner. The decision is whether 13.4's admission-path change is frozen-kernel-conformant; if it is, re-pin `admission_baseline.sha256` and delete the hold entry — the gate then blocks on the leg again with no code change.

## Fixed, not deferred: `check-service-boundary` counted cfg(test) modules as kernel ABI (2026-08-07)

> Recorded because the RED was live on `main`'s last CI run and was filed nowhere, and because the fix corrected a mis-captured ABI baseline.

- **The gate's two walks disagreed about the same module.** `walk_p4_mod` / `walk_p4_inline_item` have always skipped a `mod` whose `#[cfg(...)]` predicate mentions `test`; the SURFACE walk (`walk_mod`, `walk_inline_mod_item`) never received that rule. It went unnoticed until Story 13.6c added `maos_kernel_core::memory::spill_test_faults` — the first `pub mod` under `#[cfg(any(test, debug_assertions))]` to reach the surface walk — whose five functions were then reported as unclassified public kernel API. `check-service-boundary` is `BindingClass::Blocking` and sits in `aggregate`'s needs, so CI run `30881082656` at `b568a052` failed on it (154 jobs, one real failure). Story 13.6c's own change log claims *"All gates green (baseline/kloc/drift/ship-gate/service-boundary/fmt)"* at that same commit — a claim standing in for a control.
- **Why the rejected fix was rejected.** Adding the five symbols to `xtask/kernel-api-classes.toml` would have blessed test fault-injection as permanent public kernel API under a real class (`universal-arithmetic` / `data-movement` / `supervision`) via invariant-lock review, asserting something false: the symbols do not exist in a release build. `kloc_check` already excludes the same module for exactly that reason. **Fixed** by porting the existing 8-line predicate into a shared `is_test_cfg_mod` used by both walks.
- **The baseline had one mis-captured entry.** With the walks agreed, `maos_kernel_core::security::crypto::tests::MockCryptoProvider` (`crypto.rs:100-101`, `#[cfg(test)] pub mod tests`) became a *removed* symbol. It was captured by the buggy walk and was never real ABI — a release build has never exported it — so it was deleted from `docs/ci-baselines/kernel-surface-v0.1-beta.json` (371 → 370 items). It was the **only** `::tests::` entry in the baseline. This is a correction of a mis-capture, not an ABI break, and the file is not invariant-lock guarded.
- **Proven a real control, not a null one.** A planted non-cfg-gated `pub fn planted_unclassified_probe` in `security/crypto.rs` still reds the gate with the ordinary unclassified-symbol message; restored byte-identically (SHA-256 verified). Only cfg(test)-gated modules are now skipped.
- **Related, still open:** the older *"`check-service-boundary` baseline staleness"* entry above (Stories 3.x–4.x drift) is untouched by this fix and remains a cross-cutting maintenance task.

## Story 13.6 — findings the journey closer RULED but did not build (2026-08-08)

Story 13.6 judges; it never invents a missing mechanism (`epic-13:57`). Everything
below was MEASURED against the working tree during the composed three-team journey
run and is filed with a live owner rather than patched here.

### The kernel collapses EIGHT collective causes into ONE — the ruling

- **`CollectivePortError::Transport(_) => CollectiveErrorKind::Transport`**
  [`crates/maos-kernel-core/src/memory/mod.rs:206`] maps **all eight**
  `TransportCause` variants
  (`crates/maos-domain/src/ports/collective_memory.rs:24-58`: `Other`,
  `PartitionRefused`, `ErasureTombstoneDominates`, `ConsentDenied`, `MapStale`,
  `AttestationInvalid`, `UnmappedSpirit`, `ConnectionMismatch`) onto **one**
  kind. The erasure is **8 → 1**, not 8 → 5.
  **THE RULING (Story 13.6, the named owner of the inherited question):** the
  claim *"the operator can see why the wall refused"* is **ALLOWED on the
  host-initiated crossing path** — `emit_cross_team_share` journals a typed
  `status` per outcome (`crates/maos-bin/src/main.rs:9876-9893`, labels at
  `:9910-9922`), and Story 13.6 proved the label survives into the tenant
  Transparency Log for a REFUSED crossing
  (`refused_crossing_is_operator_visible_and_retry_needs_a_consent_repair`).
  It is **NOT ALLOWED on the Spirit path**: of the eight variants, `ConsentDenied`
  has no production constructor (documented in-code at
  `crates/maos-loom-lite/src/adapter.rs:196-200`) and `Other` is a free-text
  fallback, so the **six** causes that path can actually produce — `MapStale`,
  `ConnectionMismatch`, `UnmappedSpirit`, `AttestationInvalid`,
  `PartitionRefused`, `ErasureTombstoneDominates` — all reach the caller as the
  single word `Transport`.
  Successor: the machine-readable leg `kernel-collective-cause-distinguishable`
  (`xtask/src/check_multi_tenant_loom.rs:114`), whose owner string the Epic-13
  closer re-assigned away from itself when it ruled the collapse.
  **ACCEPTED RISK with an explicit claim boundary — ratified at the Epic-13
  retrospective §4 (2026-08-11); no successor assignment, by decision.** The
  host-initiated crossing publishes a typed status, so that path may claim
  explainability. The Spirit path may not, and no artifact may imply otherwise.
  Widening the kernel remains a future FLAG-Winston decision, not hidden
  Epic-13 debt: it is a kernel-core edit outside the closer's ZERO-Δ fence.

### `CrossWallRecallRefusal` collapses SIX variants into the token `refused`

- **A second, independent cause collapse on a SHIPPED operator surface.**
  `CrossWallRecallRefusal` (`crates/maos-domain/src/log_recall.rs:290-304`) has
  **six** distinguishable variants — `NoConsentProvider`, `NoGrant`,
  **`WrongDirection`**, `ConsentStateStale`, `ConsentStateUnavailable`,
  `ReadPortUnavailable` — and the production `cross_wall_traceback` one-shot
  collapses all six into the single token `"refused"`
  (`crates/maos-bin/src/main.rs:3075-3080`), preserving the cause only inside a
  free-text `"error"` string. `WrongDirection` in particular tells an operator
  something completely different from `NoGrant`, and the machine-readable
  outcome cannot tell them apart. Distinct from the kernel collapse above: this
  one is in the CLI, not the kernel. **Owner: `epic-14` — Epic-14 preflight, per the Epic-13 retrospective §4 disposition (2026-08-11).**

### Two operator one-shots are UNREACHABLE on a tenant host

- **`MAOS_ONE_SHOT=collective-erase` has no reachable configuration in tenant
  mode.** `MAOS_LOOM_POSTGRES` makes `MAOS_LOOM_HOME_TEAM` mandatory
  (`crates/maos-bin/src/main.rs:2784`), which makes a tenant-map source
  mandatory (`crates/maos-bin/src/tenant_map.rs::tenant_map_for_store` →
  `SourceUnavailable`); the only source is the cohort bootstrap, and
  `TenantMapAdapter::new` refuses every mode that is neither
  `cohort-a2a-daemon` nor `run --once`
  (`crates/maos-bin/src/main.rs:2804-2807`,
  `crates/maos-bin/src/tenant_map.rs:65-67` → `SourceUnrefreshable`). Both arms
  are now PINNED by the composed journey leg, so a repair reds the leg instead
  of passing silently. The GDPR operator erase control 13.5b shipped therefore
  cannot be executed on the Reza substrate at all.
  **CLOSED at the Epic-13 retrospective §4 (2026-08-11).** The final operator
  evidence records this one-shot reaching its production dispatch after the
  tenant-mode reachability repairs — bounded-refreshable classification for the
  cohort-backed `collective-erase` arm, measured live in
  `reza_three_team_three_region_production_journey` (PROCESS 5).
- **`maos traceback --team <T>` is unreachable for the same reason, and the
  consequence is sharper:** a CONSENTED, SERVED cross-wall read is impossible in
  production. The consent provider is attached only when a cohort bootstrap
  exists (`crates/maos-bin/src/main.rs:3033-3044`) and the read port only when
  `MAOS_LOOM_POSTGRES` is set (`:3050-3054`) — but that same combination builds
  the tenant map and dies at `SourceUnrefreshable` before the dispatch at
  `:3057`. Every reachable configuration ends in a refusal. Story 13.6 exercised
  the SHIPPED `CrossWallLogReadAdapter` in-process against a tenant
  Transparency Log a real daemon wrote (the first producer/consumer meeting on
  that surface) and pinned the CLI's measured refusal beside it.
  **CLOSED at the Epic-13 retrospective §4 (2026-08-11).** The final operator
  evidence records this one-shot reaching its production dispatch after the
  tenant-mode reachability repairs — measured live in
  `reza_three_team_three_region_production_journey` (PROCESS 6), which asserts
  `outcome: ok` and the exact six-field minimum-disclosure DTO.

### AC4 (c) — a vetting lapse cannot refuse a crossing: NOT BUILDABLE

- **There is no code path from a vetting lapse to a refused crossing.**
  Re-confirmed 3/3 and stronger: `grep -rn "TrustTier|trust_tier|Vetted"`
  returns **zero** in `crates/maos-loom-lite/src`, `crates/maos-a2a-core/src`,
  `crates/maos-cohort/src`, and `crates/maos-bin/src/main.rs:9300-10000`. The
  13.4 vetting machinery is orthogonal to the crossing: it gates the *upgrade*
  surface (`spirit-upgrade` / `hot-swap-precheck` →
  `enforce_vetted_upgrade_precondition`), and that is the ONLY surface on which
  a vetting lapse is demonstrable. Building the crossing-side refusal would be
  inventing a mechanism, which this story is forbidden to do.
  **Owner: `14-3-ecosystem-readiness-verification-v2-5-graduation-ledger`** —
  the v2.2 story that already owns FKCS/trial vetting infrastructure.

### AC4 (d) — an unauthorized legal-hold bypass is NOT CONSTRUCTIBLE

- **There is no join key, so the control cannot be built without breaking
  Decision D.** `grep -rni "legal.hold" crates/maos-loom-lite/` → **zero,
  including tests**. This is deeper than "no code": a hold is keyed by
  `principal_id`, and the collective tier is principal-namespace-free BY
  CONSTRUCTION (Decision D,
  `crates/maos-kernel-core/src/memory/mod.rs:180-193`). `CollectiveEraseReceipt`
  (`crates/maos-domain/src/ports/collective_memory.rs:86-89`) is
  `{deleted_rows, tombstone_recorded}` — **`held` is not representable**. So
  "erased vs failed" IS judgeable (Story 13.6 exercised exactly that
  reconciliation live: team B's row erased with a tombstone while team A's
  origin row survives — a one-sided erase) and **`held` is not**. "Unauthorized
  hold bypass is RED" would first require giving collective rows a principal
  nexus that Decision D forbids.
  **ACCEPTED RISK — Decision D preserved, ratified at the Epic-13 retrospective
  §4 (2026-08-11); no successor assignment, by decision.** Collective rows are
  principal-namespace-free and `CollectiveEraseReceipt` cannot represent
  `held`. Introducing a principal nexus merely to make the negative
  constructible would violate the governing partition-by-construction decision,
  so the control stays unbuildable and the boundary stays stated.

### `MAOS_REGION_HOME` is never reconciled against the signed `TeamEntry.region`

- Nothing in production compares the environment's `MAOS_REGION_HOME` with the
  region the SIGNED manifest carries for that host's team
  (`TeamEntry.region`). A misconfigured host stamps rows with a region its own
  cohort manifest contradicts. Story 13.6's composed journey now DERIVES each
  daemon's region from the signed entry
  (`crates/maos-bin/tests/cross_team_crossing_13_6b.rs::journey_region`), so the
  scene is honest — but the harness deriving it is not production enforcing it.
  **Owner: `epic-14` — Epic-14 preflight, per the Epic-13 retrospective §4 disposition (2026-08-11).**

### AC5's ownerless register, re-measured 2026-08-08 and dispositioned

- **No gate reconciles the kernel pin with its own HISTORY.** `grep -rn "HISTORY" xtask/src/` → **0**. `xtask/kernel-core-baseline.toml:438` reads *"23596 → 23517"* while its own prose two lines down says *"+116 physical lines from the 23401 pin"* (23401 + 116 = 23517). The `from` value could be arbitrary and nothing would red. Fixing it needs a NEW gate, which Story 13.6 is forbidden to add, so the gap is recorded rather than closed.
  **Owner: `14-6-v2-0-sweep-constitutional-ceiling-formal-methods-disposition`** — handed by the Epic-13 retrospective §4 (2026-08-11): 14.6 owns the ceiling instrument and retro-residual discipline.
- **In-`src` kernel test modules are budget-charged but CI-unexecuted.** Re-measured: **41** files under `crates/maos-kernel-core/src` declare a `mod tests`, every one of them counted by `kloc-check` (only `spill_test_faults` is excluded, `xtask/src/kloc_check.rs:189`), and all 58 `--test` invocations in `.github/workflows/discipline.yml` name an integration target, so **zero** CI invocations run the kernel lib target. The counting rule matters: "files declaring `mod tests`" = 41, which is why the earlier "42 in 42" and "44 in 43" numbers disagreed — they counted different things.
  **Owner: `14-6-v2-0-sweep-constitutional-ceiling-formal-methods-disposition`** — handed by the Epic-13 retrospective §4 (2026-08-11): 14.6 owns the ceiling instrument and retro-residual discipline.
- **`maos-bench --bench audit_query_latency` has been broken since 9.1.** `crates/maos-bench/benches/audit_query_latency.rs:24` and `:235` use the kind `"capability.invoke"`; the accepted spelling is `"capability.invocation"` (`crates/maos-audit/src/lib.rs:675`). The bench has run **zero** times.
  **Owner: `14-4-v2-0-sweep-operational-surfaces`.**
- **`EXPECTED_GATES` is hand-maintained and nothing derives it from the workflow.** Re-framed rather than re-filed: 13.6e added a genuine reverse check (`ledger_ship_badge_problems`, `xtask/src/check_ship_gate_completeness.rs:137-166`), so *"never validates anything CI produced"* is now FALSE. The surviving defect is the forward direction — 36 `EXPECTED_GATES` entries against the workflow's `check-*` jobs, with no derivation between them.
  **Owner: `14-6-v2-0-sweep-constitutional-ceiling-formal-methods-disposition`** — handed by the Epic-13 retrospective §4 (2026-08-11): 14.6 owns the ceiling instrument and retro-residual discipline.

- source_spec: none
  summary: Replace simulated post-revocation capability denial with an issued-token to CRL-apply to verification production path.
  evidence: Split from the critical remediation tranche because Story 5.4 finding 01 is independently shippable; Lunarpulse selected Story 5.2 finding 01 first on 2026-08-12.
- source_spec: none
  summary: Define successor Spirit instantiation so upgrades never reuse the predecessor runtime object as the successor.
  evidence: Split from the critical remediation tranche because Story 5.4 finding 10 requires its own architecture decision and implementation; Lunarpulse selected Story 5.2 finding 01 first on 2026-08-12.
- source_spec: none
  summary: Replace the revocation applier pipeline stub with observable end-to-end propagation coverage.
  evidence: Split from the critical remediation tranche because Story 5.4 finding 11 is independently shippable; Lunarpulse selected Story 5.2 finding 01 first on 2026-08-12.

## Deferred from: code review of j1-crosshost-2a-signable-heterogeneous-worker (2026-08-16)

- Clean-home TOCTOU window between `refuse_ambient_auth` and the spawn (10s liveness probe in between) — a credential file appearing after the check is available to the child while the signed run asserts a clean home. Real closure is spawn-time enforcement on the kernel lane (F22 `env_clear` adjacency, FLAG-Winston). Evidence: crates/maos-bin/src/main.rs:1084→1221.
- The sealed capture's completion claim cites `last_stdout_tl_ref` (documented in-code as NOT a completion witness) and the oracle verdict itself is println-only, never journaled — owner j1-crosshost-2b (typed task-outcome vocabulary, Trap 15). Evidence: crates/maos-bin/src/main.rs:1324-1337; xtask/src/demo_j1.rs:1241.
- `record-capture` accepts caller-asserted control strings (`fs_jail`, `redaction_result`, free-form `audit_refs`) with no run evidence — pre-existing `egress`-precedent shape inherited by the new AC4.2 fields. Evidence: crates/maos-cli/src/subcommands.rs `CaptureDoc::validate`.

## Deferred from: code review of j1-crosshost-2b-cross-host-delegation-mechanism (2026-08-17)

- Ctrl-C/graceful-shutdown can wait forever on a never-exiting remote worker: `spawn_blocking` enters `run_cli_wrapper_manifest` and the shutdown await has no deadline or abort path — fault-injection semantics owned by j1-crosshost-2c. Evidence: crates/maos-bin/src/delegation.rs:601-633; crates/maos-bin/src/main.rs:9561-9571.
- Crash window between journal(`Written`) and worker spawn makes a delegated task durably look processed (every replay returns `Duplicate`, no execution record) — mechanism fix (reconciliation/recovery) is 2c's; should be recorded as a RELEASE-HOLDS claim boundary. Evidence: crates/maos-bin/src/delegation.rs:442-450.
- Digest-reply path ACKs a retry as `Duplicate` after a dropped-receiver NACK (`observe_reply` precedes `push_to_intake_sink`, so the retry short-circuits before the consumer) — 12.4a seam, owner j1-crosshost-2c. Evidence: crates/maos-a2a-core/src/router.rs:1353-1379.
- `parse_host_grants_toml` defaults an omitted `permitted_tier` to T3 (highest), contradicting its own "missing required field is an error" doc — pre-existing moved code (worker_spawn.rs relocation); owner: worker-grant hardening lane. Evidence: crates/maos-bin/src/worker_spawn.rs:170-202.
- Host grant is keyed to manifest `command`/`author.name` claims, not an attested executable (no digest, signature, or resolved-path binding); a same-named file beside the daemon or on inherited $PATH satisfies it — pre-existing 2a design. Evidence: crates/maos-bin/src/worker_spawn.rs:384-427.
- `revoke_cli_subprocess_exit` errors are discarded; a failed revocation leaves the minted `Scope::CliSubprocessSpawn` token valid until its 300s TTL with no surfaced failure or compensating action — pre-existing moved code. Evidence: crates/maos-bin/src/worker_spawn.rs:662-667.
