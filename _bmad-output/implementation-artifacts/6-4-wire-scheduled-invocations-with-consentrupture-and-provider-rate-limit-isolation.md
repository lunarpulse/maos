---
dev_model_used: claude-opus-4-7
---

# Story 6.4: Wire Scheduled Invocations with ConsentRupture and Provider Rate-Limit Isolation

**Status:** done

**Type:** Epic 6 application-layer wedge story — lands three INDEPENDENT but co-located surfaces against the substrate Stories 6.1 + 6.2 + 6.3 stood up: (1) **FR26 / ADR-025 scheduled invocations** — manifest `[[schedule]]` table firing `on_schedule(ctx, schedule_id, payload)` at declared cadence with rate-limit + ComplianceClaim-stamp + principal-revocability + side-effect allowlist; the hook signature exists at HEAD (Story 2.1) and `HookDispatcher::fire_on_schedule` is plumbed (Story 5.1) but NOTHING currently fires the hook on cadence — Story 6.4 lands the cadence loop; (2) **ADR-034 binding-v0.9 `ConsentRupture` partial-consent failure semantics** — when an IAC frame with multiple recipients lands at the receiver-side consent gate and some accept while others reject (intent allowlist mismatch, posture change during transmission, token revocation), the frame is split — accepted recipients receive their copy, rejected recipients are QUARANTINED (not silently dropped), and the SENDER receives a typed `ConsentRupture` IAC frame so the application can decide whether to proceed; (3) **NFR-Scale-4 provider rate-limit isolation** — per-`(provider_id, credential_fingerprint)` token bucket gating the inference router; on bucket exhaustion the kernel emits a typed `RateLimited` IAC frame to the invoking Spirit and returns an error (NOT a stalled call); Spirit A hitting Anthropic key K1's RPM limit MUST NOT throttle Spirit B on Anthropic key K2 (different credential, same provider) NOR Spirit C on OpenAI key K3 (different provider). All three surfaces converge on the existing `IacBusAdapter::deliver_typed` pipeline (Stories 3.1 + 4.5 + 6.1 + 6.2 substrate) and the existing `Mailbox::deliver` channel-class table (§7.1.1).

## Story

As **a Spirit author writing scheduled work AND a kernel operator demanding deterministic partial-failure semantics AND a multi-tenant operator running multiple Spirits against shared LLM providers**,
I want **(a) manifest `[[schedule]]` declarations firing `on_schedule(ctx, schedule_id, payload)` at the declared cadence with kernel-enforced per-schedule rate-limit + ComplianceClaim envelope reference + principal-revocability check (revoked principal = no fire) + side-effect capability allowlist (per-firing scope narrowing per ADR-025); (b) partial-consent `ConsentRupture` event semantics (ADR-034 binding-v0.9) — when a multi-recipient frame fails consent at SOME receivers, the frame is partitioned (accepted recipients receive, rejected recipients are quarantined to the Transparency Log, NOT silently dropped), the sender's Spirit receives a typed `ConsentRupture` IAC frame on its mailbox capturing accepted + rejected slices, and the operator surface logs the rupture for forensic review; (c) per-`(provider_id, credential_fingerprint)` token-bucket rate-limit isolation — exhaustion emits a typed `RateLimited` IAC frame to the invoking Spirit AND returns `InferenceError::RateLimited { retry_after_ms }` from the inference router (no stalled `complete()` call); the bucket parameters are declared in provider driver config (RPM / TPM); bucket refills per provider's published rate; cross-credential and cross-provider isolation is structurally enforced by the bucket key**,
So that **(i) Butler's anticipatory-loop / Researcher's daily-arXiv-watch / Mira's periodic-health-check patterns become substrate-ready without Spirits self-scheduling via internal timers (which would violate I1 capability mediation); (ii) the v0.9 binding gate "sender receives `ConsentRupture` IAC frame on receiver-side rejection" passes structurally and the I8 typed-intent-consent invariant gains its third failure-class (alongside ADR-022 crash detection + `task.orphaned`); (iii) the v0.5 NFR-Scale-4 gate "provider rate-limit isolation — per-(provider, credential) token bucket; typed `RateLimited` IAC frame" passes mechanically — one Spirit's quota exhaustion is structurally prevented from cascading to peers, the LLM-substrate analog of `EAGAIN` is now first-class typed feedback rather than a silent stall**.

## What this story is NOT

- **Not** the same-Host IAC bus retract / DRR / log-before-deliver substrate. That's Story 6.1 (DONE at HEAD). Story 6.4 inherits and uses the `Mailbox::deliver` pipeline + per-frame-kind channel class table (§7.1.1) — the new `ConsentRupture` (`mpsc` 1:1, capacity 32 — sender backpressure) + `RateLimited` (`mpsc` 1:1, capacity 32 — sender backpressure) variants are added to the `channel_class_for()` const-table.
- **Not** Orchestrator distillate dispatch / intent_lineage 100% gate / CliWrapperSpirit. That's Story 6.2 (DONE). Story 6.4 inherits AC4's 100% lineage coverage gate — every new frame Story 6.4 emits (`ConsentRupture`, `RateLimited`, on-schedule cap-token invocation rows) MUST carry unbroken `intent_lineage`. Story 6.4 ADDS 10× `lineage_via_consent_rupture` + 10× `lineage_via_rate_limited` + 10× `lineage_via_on_schedule` corpus scenarios into the Story 6.2 intent-lineage corpus.
- **Not** A2A loopback / cross-Host PKI / mTLS cert rotation chaos. That's Story 6.3 (DONE — review patches outstanding; AC1 below classifies whether they closed since Story 6.3 shipped). Story 6.4's frames flow through the same `Mailbox::deliver` Phase 1/2/3 partitioning, so a same-Host `ConsentRupture` returns to a same-Host sender; cross-Host `ConsentRupture` to a peer Spirit on another Host routes through the existing Phase-3 A2A path (no new A2A code in Story 6.4).
- **Not** the Gateway sub-modules (Telegram / Slack / Discord / Signal / email) AND **not** the Phase-1 `maos-iac` + `maos-manifest` extraction. Those are Story 6.5. Story 6.4 lives in EXISTING crates: `maos-kernel-core` (the schedule-watchdog + ConsentRupture orchestration), `maos-providers` (the rate-limit token-bucket substrate), `maos-domain` (new `FrameKind::ConsentRupture` + `FrameKind::RateLimited` + payload types), and `maos-spirit-abi` (the wire-stable `FrameKind` discriminator additions). NO new workspace crate is added.
- **Not** the Story 7.3 ComplianceClaim verify path. Story 7.3 is "verify ComplianceClaim envelopes at admission with the ccac-n=600 ship gate". Story 6.4 STAMPS the schedule firing with a ComplianceClaim envelope REFERENCE (the envelope hash) so the audit chain is complete; Story 7.3 lands the cryptographic verify against the registry. At v0.5 the stamp is a structural commitment, not yet the full crypto verify.
- **Not** the FR45 GDPR-Article-17 cascade (Story 9.2). Story 6.4 hooks `principal-revocability` into the schedule firing path (a revoked principal_id = no fire); the kernel-side cascade walker that propagates principal revocation across Spirits is Story 9.2 territory. At v0.5, Story 6.4 reads the revocation status from the existing `cap-tokens::revoke_*` surface (Story 1b.2); Story 9.2's cascade plumbing is independent.
- **Not** sub-second scheduling. Per ADR-025 "What would force a revisit: scheduled invocations require sub-second cadence (the kernel's tick is currently ≥1s)". Story 6.4 honors the ≥1s cadence floor; the `ScheduleWatchdog` poll interval is configurable (default 1000ms) but the minimum cadence is 1s. Sub-second is out of scope.
- **Not** a `ProviderRateLimiter` distributed across Hosts. The token bucket lives per-process — each Host's kernel owns its own per-`(provider_id, credential_fingerprint)` buckets. Cross-Host bucket coordination (e.g., a 30-host fleet sharing an Anthropic key) is a v2.0+ concern and would need a coordination protocol; Story 6.4 ships the single-Host bucket — which is the v0.5 binding.
- **Not** a kernel-side cron expression parser. Story 6.4 supports `cadence_secs: u32` (every-N-seconds firing) ONLY. Cron expressions (`@daily`, `0 9 * * *`) are deferred — the field shape is intentionally minimal at v0.5 per `[[feedback_lunarpulse_observability_preference]]` (observable cadence beats expressive scheduling-DSL).
- **Not** an ABI_VERSION bump. The `FrameKind` additions (`ConsentRupture = 22`, `RateLimited = 23`) are explicit-discriminant additive variants per the Story 6.2 precedent (`CliSubprocessOutput = 21`). `cargo-public-api --diff` reports `Added` on the new variants; `ABI_VERSION` remains `1`.
- **Not** any §A1 / §A2 / §A3 / §A5 / §A6 bridge work or 6.1 / 6.2 / 6.3 deferred-rows remediation. Those are **preconditions** mechanically classified in AC1 — Story 6.4 does NOT execute remediation, it verifies which closed since Story 6.3 shipped (especially the 22 review patches on Story 6.3 + the 3 decision-needed items).

## Bridge Preconditions (Story 6.1 + 6.2 + 6.3 deferrals + Epic 5 retro carry-forward)

Per `_bmad-output/implementation-artifacts/6-3-build-the-a2a-peer-mesh-from-loopback-to-cross-host-with-mtls-rotation-chaos.md` §Review Findings (3 decision-needed + 22 patches + 2 deferrals dismissed) + `_bmad-output/implementation-artifacts/6-2-…orchestrator-distillates….md` §Review Findings + `_bmad-output/implementation-artifacts/6-1-…full-iac-bus….md` §Review Findings + `epic-5-retro-2026-05-24.md` §Action-Items, the following must be **mechanically classified** at Story 6.4 open (the AC1 gate distinguishes `closed_since_6_3` from `still_deferred` — Story 6.4 does NOT require closure of all rows; it requires honest classification, and rows marked `blocking_6_4` MUST close inline because they are blocking 6.4's surface):

| Row | Source | Closure required for 6.4? | Status check |
|---|---|---|---|
| **6.3-P1 / P2** — TOFU pin verify never invoked + consent envelope expiry stub | Story 6.3 §Review Findings Critical | **NO — verify-only** | Story 6.4 does not touch the A2A intake path; report current state. If 6.3-P1 closes, the same envelope-expiry check pattern informs Story 6.4's per-recipient consent gate (defense-in-depth note in AC3). |
| **6.3-P3** — Duplicate `A2ARouter` trait (local vs domain) | Story 6.3 §Review Findings High | **NO — verify-only** | Local-rename pattern (e.g., `A2APeerRouter`) is one option; Story 6.4 does not touch `maos-a2a`. |
| **6.3-P4** — CI `a2a-loopback-corpus-v0` referencing non-existent test targets | Story 6.3 §Review Findings High | **VERIFY — must PASS at HEAD** | If discipline.yml still references non-existent test targets, EVERY Story 6.4 PR will fail CI. AC1 confirms the job either has the missing tests OR has been corrected; if BROKEN, the dev STOPS and surfaces. |
| **6.3-P5** — `handle_intake` peer-lookup fallback (security bypass) | Story 6.3 §Review Findings High | **NO — verify-only** | Story 6.4 does not route through `handle_intake`; report state. |
| **6.3-P6** — Missing `boot_nonce` in JSON-RPC request | Story 6.3 §Review Findings High | **NO — verify-only** | Story 6.4 does not touch the JSON-RPC framing. |
| **6.3-P7..P22** — Remaining Medium / Low Story 6.3 patches | Story 6.3 §Review Findings | **NO — carry-forward** | AC1 reports count of `**open**` Critical/High patches; per `[[feedback_mechanical_gates_compound_promises_decay]]` `check-review-findings-resolved` gate catches if 6.3 ships Critical/High `**open**`. |
| **6.3-D1 / D2 / D3** — Decisions still pending (IntentClass→A2A intent-string contract; `A2AError`→`IacBusError::CrossHostRouteFailure(String)` type-loss; cross-host route failure ordering) | Story 6.3 §Review Findings Decision | **NO — carry-forward** | AC1 reports state; not blocking 6.4. |
| **6.3-D-Sub-arm** — `smoke-iac-bus-6` arm (Story 6.1 D5.1/5.2 carry-forward) | Story 6.1 D5.1/5.2 → 6.3 | **NO — carry-forward** | Story 6.4 emits a new smoke arm `smoke-schedule-6-4` (AC5); arm-chaining with `smoke-a2a-loopback-6-3` (shipped) and `smoke-iac-bus-6` (NOT shipped) is informational. |
| **6.1-D-3.\*** — DRR SCB integration + `[scheduler.weights]` config + spec-drift test | Story 6.1 Tasks 3.3-3.8 | **NO — carry-forward** | AC1 reports current state; Story 6.4's scheduled invocations DO NOT bypass DRR — they fire `on_schedule` through the existing hook dispatcher which runs under the existing scheduler quantum semantics. |
| **6.2-D-Bench-Note** — `cli_wrapper_subprocess_fan_out.rs` realistic-CLI bench | Story 6.2 AC6 §Bench-Note | **NO — verify-only** | Calibration-phase bench; not blocking 6.4. |
| **6.1-§A2 / 6.2 §A2 / 6.3 §A2** — Epic 5 §A2 backfill (5-1, 5-2, 5-5a, 5-5b formal review still placeholder) | Epic 5 retro §A2 | **NO — carry-forward** | AC1 reports current state. Per Story 6.3 AC1 evidence at completion: 4/5 placeholder remains. |
| **§A3** — `xtask check-serde-error-handling` gate | Epic 4 retro §A6 → Epic 5 §A3 | **VERIFY — gate must PASS at HEAD** | Gate SHIPPED + discipline.yml-wired (Story 6.3 confirmed). AC1 confirms PASS at HEAD; Story 6.4's `[[schedule]]` manifest parsing path (AC2) is a high-risk surface for `.unwrap_or_default()` regressions — the gate catches new violations. |
| **§A5** — `xtask check-review-findings-resolved` gate | Epic 5 retro §A5 | **VERIFY — gate must PASS at HEAD** | xtask binary SHIPPED at HEAD; discipline.yml wiring is Epic 5 retro carry-forward (still open per Story 6.3 evidence). AC1 confirms the xtask binary exists; the discipline.yml wiring gap is documented as inherited debt. |
| **§A6** — `xtask check-dev-record-completeness` gate | Epic 5 retro §A6 | **VERIFY — gate must PASS at HEAD** | xtask binary SHIPPED at HEAD; same discipline.yml wiring caveat as §A5. AC1 confirms the dev sets `dev_model_used` at story-start. |
| **6.3-AC7 smoke-arm shipped** — `smoke-a2a-loopback-6-3` arm + job | Story 6.3 AC7 | **VERIFY — shipped** | Confirmed shipped in `crates/maos-bin/src/main.rs` + `.github/workflows/discipline.yml:1023`. Story 6.4's `smoke-schedule-6-4` follows the same arm pattern. |
| **6.4-MAOS-PROVIDERS-BASELINE** | Story 6.4 substrate confirmation | **blocking_6_4** | Assert `crates/maos-providers/src/lib.rs` exists with `Provider` trait + Anthropic / OpenAI / Ollama drivers; assert NO existing `ProviderRateLimiter` type (Story 6.4 lands the canvas clean). If a partial implementation exists, the dev STOPS and surfaces. |
| **6.4-FRAMEKIND-BASELINE** | Story 6.4 substrate confirmation | **blocking_6_4** | Assert `FrameKind::ConsentRupture` and `FrameKind::RateLimited` do NOT yet exist; assert discriminants `22` and `23` are free. If a prior scaffold occupied them, the dev STOPS and surfaces (this preserves the explicit-discriminant additive contract). |
| **6.4-SCHEDULE-WATCHDOG-BASELINE** | Story 6.4 substrate confirmation | **blocking_6_4** | Assert `crates/maos-kernel-core/src/scheduler/schedule_watchdog.rs` does NOT yet exist; assert no `ScheduleWatchdog` type elsewhere. The substrate canvas is clean. |

AC1 classifies all 17 rows. Rows marked **VERIFY** are mechanically checked and the run output reported truthfully; **NO — carry-forward** rows are documented per Story 6.1 / 6.2 / 6.3 precedent; **blocking_6_4** rows are 3 substrate-canvas confirmations whose failure stops the dev at AC1. Per `[[feedback_mechanical_gates_compound_promises_decay]]` the AC1 gate that Story 6.1 introduced (`check-epic-6-bridge`) compounds in Story 6.4 — extended with the new 6.4-specific rows added to the gate's check list. The gate ships discipline-as-code rather than discipline-as-promise.

**Discipline floor:** Story 6.4 introduces ZERO new `unwrap_or_default()` on serde paths. The `[[schedule]]` manifest section parsing path (AC2) is the highest-risk surface for this anti-pattern — Story 5.5d shipped 8 such violations; Story 6.1 shipped 8 more; Story 6.4 ships ZERO new such patterns and the §A3 gate confirms. The `#[serde(deny_unknown_fields)]` posture applies to the new `RawScheduleEntry` struct per Story 5.5d post-hoc lesson.

## Acceptance Criteria

### AC1 — Bridge preconditions classified mechanically; 6.4-blocking rows confirmed before AC2 opens

**Given** the 17 bridge rows in the §Bridge-Preconditions table above
**When** the dev runs `cargo run -p xtask -- check-epic-6-bridge --story 6.4` at story start (the `--story 6.4` flag extends the umbrella gate with the new 6.4 row set — 6.4 EXTENDS, does not replace; per `[[feedback_mechanical_gates_compound_promises_decay]]` discipline-as-code stays compact)
**Then** each row is classified into one of `{closed_since_6_3, still_deferred, blocking_6_4, shipped_pass, shipped_fail}` and the command exits 0 only if every `blocking_6_4` row has cleared AND every `shipped_*` row reports its current state

**Specific mechanical checks (extending `xtask/src/check_epic_6_bridge.rs`):**

1. **§A3 / §A5 / §A6 xtask presence (shipped_pass expected):** Assert each xtask file exists (`xtask/src/check_serde_error_handling.rs`, `check_review_findings_resolved.rs`, `check_dev_record_completeness.rs`) AND run each gate sequentially. If `check-serde-error-handling` FAILS at HEAD because Story 6.4 introduced a NEW `.unwrap_or_default()` on a serde path, the dev STOPS and surfaces. Pre-existing Story 5.5d / 6.1 violations are inherited debt (Story 6.3 AC1 evidence documented at HEAD).
2. **6.3-AC7-smoke-arm verification (shipped):** Grep `crates/maos-bin/src/main.rs` for `"smoke-a2a-loopback-6-3"`. Assert present. The new Story 6.4 smoke arm at AC5 (`smoke-schedule-6-4`) chains on top.
3. **6.3-P4 CI test-target verification (must PASS at HEAD):** Parse `.github/workflows/discipline.yml`'s `a2a-loopback-corpus-v0` job; for each `cargo test -p maos-a2a --test <name>` invocation, assert the file `crates/maos-a2a/tests/<name>.rs` exists. If ANY referenced test file does NOT exist, the dev STOPS — every Story 6.4 PR would otherwise fail CI on this pre-existing breakage. If the dev finds 6.3-P4 already closed (test files added OR CI corrected), AC1 reports `closed_since_6_3`.
4. **6.3-P1/P2/P3/P5/P6/P7..P22/D1/D2/D3 row reporting (verify-only):** Parse `_bmad-output/implementation-artifacts/6-3-…mtls-rotation-chaos.md` `### Review Findings` table; count `**open**` Critical/High rows; report counts. Story 6.4 does NOT block on these.
5. **6.1-D-3.\* / 6.2-D-Bench-Note verification (carry-forward):** Report current state of DRR scheduler tasks (3.3-3.8) and `cli_wrapper_subprocess_fan_out.rs` bench. Story 6.4 does NOT depend on either.
6. **§A2 verification (carry-forward):** For each of `5-1-*.md`, `5-2-*.md`, `5-4-*.md`, `5-5a-*.md`, `5-5b-*.md`: check whether the `### Review Findings` block is still `_No review findings._` (placeholder) or populated. Report counts; do NOT block. Story 6.3 evidence: populated=1/5; placeholder=4/5.
7. **6.4-MAOS-PROVIDERS-BASELINE (blocking_6_4):** Assert `crates/maos-providers/Cargo.toml` exists; assert `crates/maos-providers/src/lib.rs` declares `Provider`, `ProviderError`; assert NO file named `crates/maos-providers/src/rate_limit.rs` exists at HEAD (Story 6.4 substrate canvas is clean).
8. **6.4-FRAMEKIND-BASELINE (blocking_6_4):** Parse `crates/maos-spirit-abi/src/identity.rs` — assert `FrameKind::ConsentRupture` and `FrameKind::RateLimited` do NOT yet exist; assert discriminants `22` and `23` are FREE (greppable `= 22,` / `= 23,` returns no matches in the enum body). If occupied, the dev SURFACES and either renumbers or escalates per the explicit-discriminant additive contract.
9. **6.4-SCHEDULE-WATCHDOG-BASELINE (blocking_6_4):** Assert `crates/maos-kernel-core/src/scheduler/schedule_watchdog.rs` does NOT yet exist; assert `ScheduleWatchdog` is not declared elsewhere in `crates/maos-kernel-core/src/`. The substrate canvas is clean.
10. **6.4-RP-Review-Findings status (verify-only):** Parse `_bmad-output/implementation-artifacts/6-3-…mtls-rotation-chaos.md` `### Review Findings` table; count `**open**` Critical/High rows. Report count for the dev record (informational — the §A5 gate would block at `done` if Story 6.4's OWN Review Findings table carries `**open**` Critical/High; Story 6.3's residual debt is its own, separately tracked).

**And** the AC1 run output is cited verbatim in the story's `### Completion Notes List` per Epic 1b retro §A8 + Story 6.1 / 6.2 / 6.3 AC1 precedent
**And** the dev MUST NOT begin AC2–AC5 implementation until AC1 exits 0 for every `blocking_6_4` row AND `6.3-P4` resolves (either shipped corrected OR documented as pre-fixed). If a `blocking_6_4` row regresses (substrate canvas dirty), the dev STOPS and surfaces to Lunarpulse
**And** the `check-epic-6-bridge` job already wired into `.github/workflows/discipline.yml` extends with the new `--story 6.4` matrix entry OR sibling job — Story 6.4 follows whichever pattern Story 6.3 chose for `--story 6.3` (consult `xtask/src/check_epic_6_bridge.rs` and `.github/workflows/discipline.yml:911` for the established matrix pattern)

### AC2 — `[[schedule]]` manifest table + `ScheduleWatchdog` firing `on_schedule(ctx, schedule_id, payload)` (FR26 / ADR-025)

**Given** the existing substrate at HEAD:
- `crates/maos-spirit-abi/src/lifecycle.rs:54-57` defines `SchedulePayload<'a> { schedule_data: &'a [u8], schedule_len: usize }` — the wire shape for the payload delivered to `on_schedule` (Story 2.1 substrate)
- `crates/maos-spirit-abi/src/lifecycle.rs:165` `Spirit::on_schedule(&self, ctx: &mut Ctx, payload: &SchedulePayload<'a>) {}` — the trait method with default no-op (Story 2.1 substrate)
- `crates/maos-spirit-abi/src/lifecycle.rs:289` `pub on_schedule: for<'a> fn(&T, &mut Ctx, &SchedulePayload<'a>)` — the vtable slot
- `crates/maos-kernel-core/src/scheduler/hook_dispatch.rs:266-271` `HookDispatcher::fire_on_schedule(scb, payload) -> HookOutcome` — the dispatcher entry point (Story 5.1 substrate). **Currently called only from local-runner tests and the `MAOS_ONE_SHOT` smoke arms (e.g., `crates/maos-bin/src/main.rs:1416`); NO production scheduler-loop fires it on cadence today.**
- `crates/maos-kernel-core/src/scheduler/idle_watchdog.rs` — the per-Spirit IdleWatchdog (Story 5.1) — the structural analog Story 6.4 mirrors for `ScheduleWatchdog`
- `crates/maos-kernel-core/src/security/manifest.rs:1052-1130` — `[scheduling]` section (`priority_weight` + `yield_every_polls` + `idle_window_ms`) exists; **NO `[[schedule]]` array section exists at HEAD** (Story 6.4 adds it)
- `crates/maos-kernel-core/src/security/manifest.rs:1136-1184` — `[lifecycle]` section with `VALID_HOOK_NAMES` array containing `"on_schedule"` (Story 5.1)
- `crates/maos-domain/src/invariants/i1.rs:60-103` `Scope` enum (`#[non_exhaustive]`) — the per-firing side_effect_allowlist binds a narrowed cap-token to a subset of `Scope` variants
- `crates/maos-capability/src/cap_tokens/mod.rs:272-294` `revoke()` / `revoke_all()` — the existing revocation surface; principal-revocability check at fire-time consults the existing surface (NEW per-principal index in this story is OUT OF SCOPE — Story 9.2 cascade walker; v0.5 uses the per-Spirit revocation as a proxy)
- ADR-025 verbatim: "Spirits may declare scheduled invocations via manifest `[schedule]` table; kernel fires `on_schedule(ctx, schedule_id, payload)` at declared cadence with rate-limit, ComplianceClaim-stamp, principal-revocability, and side-effect allowlist."
- FR26 verbatim: "Spirit can declare scheduled invocations via manifest `[schedule]` table; kernel fires `on_schedule(ctx, schedule_id, payload)` at declared cadence with rate-limit, ComplianceClaim-stamp, principal-revocability, and side-effect allowlist (per ADR-025)."
- Architecture §5.3 verbatim (table row): "`on_schedule` | A scheduled invocation fires | Run periodic task | Story 2.1 (signature), Story 5.1 (runtime)" — Story 5.1 shipped the runtime DISPATCHER; Story 6.4 ships the FIRING.

**When** Story 6.4 lands the schedule-firing surface

**Then** the manifest gains a new `[[schedule]]` array section parsed at `crates/maos-kernel-core/src/security/manifest.rs` (extending the `SpiritManifest` struct additively per Story 1b.4 ABI-additive contract):

```toml
# Example manifest fragment

# Existing sections — unchanged
[lifecycle]
enabled_hooks = ["on_load", "on_start", "on_schedule"]

[scheduling]
priority_weight = 100
idle_window_ms = 30000

# NEW — Story 6.4 / FR26 / ADR-025
[[schedule]]
id = "morning-digest"
cadence_secs = 3600  # every hour; minimum 1, maximum 604800 (1 week)
payload_b64 = ""     # optional opaque payload (base64-encoded bytes); omit for empty
# Per-schedule rate-limit — separate from provider rate-limit (AC4).
# Caps the firing rate even if the cadence would otherwise allow more.
rate_limit_per_hour = 60
# ComplianceClaim envelope reference (Story 7.3 envelope hash; v0.5 = optional structural stamp)
compliance_claim_ref_hex = "sha256:0000...."  # 64-hex; optional at v0.5
# Principal-revocability — when true, a revoked principal halts the firing.
principal_revocability = true
# Side-effect allowlist — Scope subset granted to the per-firing cap-token.
# Empty = no side-effects (memory-only).
side_effect_scopes = [
  { kind = "MemWrite", scope = "spirit:butler:digest" },
  { kind = "ProviderInfer", provider = "anthropic" },
]

[[schedule]]
id = "arxiv-watcher"
cadence_secs = 86400  # daily
payload_b64 = "eyJxdWVyeSI6ImNzLkFJIn0="
rate_limit_per_hour = 1
principal_revocability = true
side_effect_scopes = [
  { kind = "NetHttps", domain = "export.arxiv.org" },
  { kind = "MemWrite", scope = "spirit:researcher:papers" },
]
```

**And** the new Rust types land in `crates/maos-kernel-core/src/security/manifest.rs` (additive — NEW types, no existing types touched):

```rust
/// Story 6.4 / FR26 — `[[schedule]]` manifest entry.
///
/// Each entry declares one scheduled invocation that fires `on_schedule(ctx,
/// schedule_id, payload)` at the declared cadence. ADR-025 governs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleEntry {
    pub id: String,                              // unique within the manifest
    pub cadence_secs: u32,                       // [1, 604800]
    pub payload_bytes: Vec<u8>,                  // base64-decoded; empty if absent
    pub rate_limit_per_hour: u32,                // [1, 3600]; firing cap regardless of cadence
    pub compliance_claim_ref: Option<[u8; 32]>,  // optional envelope hash at v0.5
    pub principal_revocability: bool,            // default true
    pub side_effect_scopes: Vec<maos_domain::invariants::i1::Scope>,
}

/// The `[[schedule]]` section — Vec<ScheduleEntry> with cross-entry id uniqueness.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SchedulesSection {
    pub entries: Vec<ScheduleEntry>,
}

impl SchedulesSection {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawSchedulesSection =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSchedulesSection {
    #[serde(default, rename = "schedule")]
    entries: Vec<RawScheduleEntry>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScheduleEntry {
    id: String,
    cadence_secs: u32,
    #[serde(default)]
    payload_b64: String,
    #[serde(default = "default_rate_limit_per_hour")]
    rate_limit_per_hour: u32,
    #[serde(default)]
    compliance_claim_ref_hex: Option<String>,
    #[serde(default = "default_principal_revocability")]
    principal_revocability: bool,
    #[serde(default)]
    side_effect_scopes: Vec<RawSideEffectScope>,
}
```

**And** the `ScheduleWatchdog` lands at `crates/maos-kernel-core/src/scheduler/schedule_watchdog.rs` (NEW module — structural analog of `idle_watchdog.rs`):

```rust
//! Story 6.4 / FR26 / ADR-025 — per-Spirit + per-schedule_id ScheduleWatchdog.
//!
//! Polls the SCB map; for each Running Spirit with `[[schedule]]` entries
//! whose `on_schedule` hook is enabled (per `[lifecycle].enabled_hooks`), fires
//! `on_schedule(ctx, schedule_id, payload)` when `now_ns - last_fire_ns ≥ cadence_secs`.
//!
//! Per-firing gate (rejection ordered):
//!   1. `kernel_invocation_allowed("on_schedule")` per the manifest's lifecycle gate
//!   2. principal-revocability check (when entry.principal_revocability)
//!   3. rate-limit check against the per-schedule token bucket
//!   4. ComplianceClaim stamp recorded in the firing's TL row
//!   5. side_effect_allowlist narrows the cap-token issued for the firing
//!
//! Each firing is journaled to the Transparency Log (Story 1b.1) with
//! `FrameKind::CapabilityInvocation` carrying the schedule_id, the
//! ComplianceClaim envelope hash (when present), and the narrowed scope.
pub struct ScheduleWatchdog {
    scbs: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
    dispatcher: Arc<HookDispatcher>,
    capability: Arc<crate::capability::CapabilityRegistryAdapter>,
    iac: Arc<crate::iac::IacBusAdapter>,
    /// Per-schedule_id token bucket (keyed `(spirit_id, schedule_id)`).
    /// Refill rate = entry.rate_limit_per_hour / 3600 per second.
    rate_limits: Arc<dashmap::DashMap<(String, String), TokenBucket>>,
    /// `MAOS_SCHEDULE_FAST=1` collapses cadence by 100× (test convenience),
    /// matches the IdleWatchdog `MAOS_IDLE_FAST` convention.
    fast_mode: bool,
}

impl ScheduleWatchdog {
    pub fn new(...) -> Self { ... }
    pub fn spawn(self: Arc<Self>, cancel: CancellationToken) -> JoinHandle<()> { ... }

    /// One pass — called from the spawn loop at `poll_interval_ms` (default 1000ms;
    /// `MAOS_SCHEDULE_FAST=1` → 40ms).
    async fn check_and_fire(&self) {
        let candidates = self.collect_candidates_outside_lock();
        for (scb, entry) in candidates {
            if !self.kernel_invocation_allowed_for_on_schedule(&scb) { continue; }
            if entry.principal_revocability && self.is_principal_revoked(&scb) { continue; }
            if !self.bucket_consume(&scb.spirit_id, &entry.id) { continue; }
            self.fire_one(&scb, &entry).await;
        }
    }
}
```

**And** the firing path emits a typed Transparency Log row at firing time (`FrameKind::CapabilityInvocation` — the existing variant carrying a `schedule_fire` payload):

```rust
// crates/maos-kernel-core/src/iac/payload.rs — additive payload variant
pub struct ScheduleFireRecord {
    pub spirit_id: String,
    pub schedule_id: String,
    pub fired_at_ns: u64,
    pub compliance_claim_ref: Option<[u8; 32]>,
    pub side_effect_token_id: maos_domain::invariants::i1::TokenId,
    pub principal_revocability: bool,
}
```

**And** the per-firing cap-token is issued with the narrowed `side_effect_scopes` slice — the Spirit's `on_schedule` handler holds a token strictly less privileged than its admission token (the narrowing is enforced at issue time via the existing `cap_tokens::issue` surface; the token TTL is `2 × cadence_secs` capped at `300s` per ADR-023 Standard intent_class)
**And** the principal-revocability check at v0.5 is the per-Spirit `cap_tokens::is_revoked(spirit_pid)` proxy (full per-principal cascade is Story 9.2); when the Spirit's tokens are wholesale revoked, all of its schedules are halted; when only some tokens revoked, the schedule still fires but the side_effect_allowlist intersects the surviving capabilities
**And** the ComplianceClaim stamp at v0.5 is a STRUCTURAL pass-through — the entry's `compliance_claim_ref_hex` (if present) is written verbatim into the TL row. Story 7.3 lands the cryptographic verify (which fingerprint matches which registry envelope); at v0.5 the stamp's truth is the operator's manifest claim
**And** the `[[schedule]]` section parsing path uses `#[serde(deny_unknown_fields)]` per Story 5.5d post-hoc lesson; cross-entry `id` uniqueness is validated AT PARSE TIME (`ManifestError::DuplicateScheduleId { id }`)
**And** validation enforces:
  - `1 ≤ cadence_secs ≤ 604_800` (1 week max)
  - `1 ≤ rate_limit_per_hour ≤ 3600` (per ADR-025 ≥1s cadence floor; can't fire more than once per second)
  - `id` is non-empty, `[a-zA-Z0-9_-]{1,64}`, unique
  - `compliance_claim_ref_hex` if present must parse to 32 bytes
  - `side_effect_scopes` entries must round-trip through `Scope` deserialization (uses the existing Scope shape; NO new Scope variant added)
**And** the `[lifecycle].enabled_hooks` array MUST include `"on_schedule"` for ANY `[[schedule]]` entry to fire — if a manifest has `[[schedule]]` entries but `[lifecycle].enabled_hooks` excludes `on_schedule`, parsing succeeds (additive) but the watchdog skips firing (the lifecycle gate wins). Document this in the field-comment per ADR-025.
**And** integration tests at `crates/maos-kernel-core/tests/schedule_watchdog_fr26.rs` (8 scenarios):
  - **2.1**: Single `[[schedule]]` entry with `cadence_secs = 1` + `MAOS_SCHEDULE_FAST=1`; assert hook fires within 100ms; payload_bytes round-trip correct
  - **2.2**: Two entries with different cadences; assert each fires independently at its own cadence
  - **2.3**: `rate_limit_per_hour = 1` with `cadence_secs = 1` + fast mode; assert only ONE fire happens despite cadence ticking
  - **2.4**: `principal_revocability = true` + spirit's tokens revoked via `cap_tokens::revoke_all(pid)`; assert NO fire
  - **2.5**: `[lifecycle].enabled_hooks` excludes `on_schedule`; assert NO fire even with `[[schedule]]` entry present
  - **2.6**: `side_effect_scopes = [MemWrite { scope: "x" }]`; assert per-firing cap-token issued with EXACTLY that scope (verify via `cap_tokens::verify(token_id) -> CapabilityTokenRecord.scope`)
  - **2.7**: Spirit transitions Running → Paused; assert in-flight `on_schedule` completes but no new fires until Resumed (mirrors IdleWatchdog semantics)
  - **2.8**: `compliance_claim_ref_hex` present; assert the TL row carries the verbatim 32-byte hash
**And** the `SpiritManifestBundle` at `crates/maos-kernel-core/src/scheduler/control_block.rs` extends additively with `pub schedules: SchedulesSection` (`#[serde(default)]` round-trip; `Default::default()` is the empty section)
**And** the composition root at `crates/maos-bin/src/main.rs` constructs and spawns the ScheduleWatchdog alongside the IdleWatchdog — the kernel daemon body wires the cancellation token to the same `tokio_util::sync::CancellationToken` used for graceful shutdown
**And** `cargo-public-api --diff` reports: `Added` count > 0 (`ScheduleEntry`, `SchedulesSection`, `ScheduleFireRecord`, `ScheduleWatchdog`, `TokenBucket` placeholder for rate-limit, the new `SpiritManifestBundle::schedules` field); `Removed` = 0; `Changed` = 1 (the additive field on `SpiritManifestBundle` — additive-friendly because the field is `#[serde(default)]`)

### AC3 — `ConsentRupture` partial-consent failure event (ADR-034 binding-v0.9)

**Given** the existing substrate at HEAD:
- `crates/maos-spirit-abi/src/identity.rs:16-35` `FrameKind` enum with explicit discriminants 0..=9 + 21 (`CliSubprocessOutput`). Story 6.4 ADDS discriminant `22` for `ConsentRupture` (continuing the explicit-discriminant additive contract Story 6.2 established)
- `crates/maos-domain/src/frame.rs:25-50` `IacFrame` struct with `to: Vec<FrameAddress>` (multi-recipient is the existing shape)
- `crates/maos-domain/src/frame.rs:341-..` `ConsentEnvelope` extended at Story 6.3 with `intent_class: Option<A2AIntent>` + `valid_until_ns: Option<u64>` (both `#[serde(default)]`) — the v0.9 binding consent envelope shape
- `crates/maos-kernel-core/src/iac/mod.rs:280-..` `IacBusAdapter::deliver_typed` — the I2 log-before-deliver enforcement pipeline + the I13 lineage check + Story 6.2's `check_orchestrator_distillate_required` gate (line 301-370 substrate; Story 4.5 + 6.2 evolved)
- `crates/maos-kernel-core/src/iac/mailbox.rs:133-255` `Mailbox::deliver` — Phase 1 (partition recipients), Phase 2 (same-host delivery), Phase 3 (cross-host A2A route per Story 6.3). Story 6.4 INSERTS a per-recipient consent check between Phase 1 and Phase 2 (the consent-rupture detection point)
- `crates/maos-kernel-core/src/iac/channels.rs:..` `channel_class_for(kind) -> Option<(ChannelClass, capacity)>` const-table. Story 6.4 ADDS the new `(ConsentRupture, Mpsc, 32)` row to align with §7.1.1 `consent.request` cardinality (1:1 sender ← bus)
- ADR-034 verbatim: "Sender-approved / receiver-rejected mid-frame becomes a `ConsentRupture` event; frame is quarantined, not delivered, not silently dropped. Sender receives `ConsentRupture` IAC frame; operator surface logs the rupture for forensic review."
- Architecture §4.5 verbatim: "Partial-consent failure semantics. A frame whose sender approved but whose receiver rejected mid-frame (intent allowlist mismatch, posture change during transmission, token revocation) becomes a typed `ConsentRupture` event; the frame is quarantined, not delivered, not silently dropped. The sender's Spirit receives a `ConsentRupture` IAC frame; the operator surface logs the rupture for forensic review."

**When** Story 6.4 lands the ConsentRupture surface

**Then** the `FrameKind` enum at `crates/maos-spirit-abi/src/identity.rs` gains the additive variant:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum FrameKind {
    TaskAssign = 0,
    TaskComplete = 1,
    DecisionDispatch = 2,
    EpistemicHalt = 3,
    TelemetryEvent = 4,
    ConsentRequest = 5,
    Retract = 6,
    CapabilityInvocation = 7,
    SandboxBlock = 8,
    InferenceCall = 9,
    CliSubprocessOutput = 21,
    /// Story 6.4 — ADR-034 binding-v0.9. Sender-approved / receiver-rejected
    /// mid-frame becomes a `ConsentRupture` event; the original frame is
    /// quarantined for the rejected slice and DELIVERED to the accepted slice;
    /// the sender's mailbox receives a typed ConsentRupture frame so the
    /// application can decide retry/escalate/halt.
    ConsentRupture = 22,
    /// Story 6.4 — NFR-Scale-4. Per-(provider, credential) token bucket
    /// exhaustion emits this typed frame to the invoking Spirit. The frame
    /// is NOT a stalled call; the inference router returns
    /// `InferenceError::RateLimited { retry_after_ms }` simultaneously.
    RateLimited = 23,
}
```

**And** `FrameKind::from_u8` is extended with the new arms (22 / 23)
**And** the `ChannelClass` const-table at `crates/maos-kernel-core/src/iac/channels.rs` gains the new rows aligned to §7.1.1 cardinality:

| `kind` | Channel class | Capacity floor | Drop policy |
|---|---|---|---|
| `ConsentRupture` | `Mpsc` | 32 | Backpressure (await capacity); no drop |
| `RateLimited` | `Mpsc` | 32 | Backpressure (await capacity); no drop |

**And** the new payload types land at `crates/maos-domain/src/frame.rs` (additive — NEW structs):

```rust
/// Story 6.4 / ADR-034 — ConsentRupture payload.
///
/// Emitted to the SENDER on receiver-side partial consent rejection.
/// `accepted` recipients received the original frame; `rejected` recipients
/// did NOT (their slice is quarantined in the Transparency Log, not delivered).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConsentRupturePayload {
    pub rupture_id: [u8; 16],          // ULID for correlation
    pub original_frame_id: [u8; 16],   // the frame whose consent fractured
    pub original_kind: maos_spirit_abi::identity::FrameKind,
    pub accepted: Vec<FrameAddress>,
    pub rejected: Vec<RuptureRejection>,
    pub ruptured_at_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RuptureRejection {
    pub address: FrameAddress,
    pub reason: RuptureReason,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RuptureReason {
    /// Receiver's intent allowlist excludes the frame's `intent`.
    IntentAllowlistMismatch,
    /// Receiver's posture shifted between sender's send-decision and
    /// receiver's accept-evaluation (e.g., autonomous → cautious mid-frame).
    PostureShiftedDuringTransmission,
    /// Receiver's capability token was revoked between send and accept.
    TokenRevoked,
    /// Receiver's principal_id has an active revocation.
    PrincipalRevoked,
    /// Receiver's mailbox channel is closed (Spirit unloaded mid-frame).
    RecipientUnloaded,
}

/// Story 6.4 / NFR-Scale-4 — RateLimited payload.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RateLimitedPayload {
    pub provider_id: String,
    /// SHA-256 hex (8-byte prefix) of the credential bytes for cross-credential
    /// disambiguation in logs; full credential never logged.
    pub credential_fingerprint_prefix_hex: String,
    pub retry_after_ms: u64,
    /// Bucket state at exhaustion time (for client-side back-off planning).
    pub bucket_remaining: u32,
    pub bucket_capacity: u32,
    pub refill_per_sec: u32,
    /// Optional schedule_id when the rate-limit fires under a scheduled
    /// invocation context (cross-link to AC2).
    pub schedule_id: Option<String>,
}
```

**And** `Mailbox::deliver` at `crates/maos-kernel-core/src/iac/mailbox.rs` gets a NEW pre-Phase-2 step: **Phase 1.5 — per-recipient consent gate** that runs the receiver-side consent check per recipient. The check consults:
1. The receiver's `[a2a]` (or future `[consent]`) intent_allowlist when the frame carries a non-None `ConsentEnvelope.intent_class` (cross-host; Story 6.3 substrate)
2. The receiver's posture at consent-evaluation time vs the sender's send-decision posture (carried on the frame's metadata at send time)
3. The receiver's `cap_tokens::is_revoked(receiver_pid)` status
4. The receiver's principal_id revocation status (proxy via per-Spirit revocation at v0.5)
5. The receiver's mailbox-channel-closed status (RecipientUnloaded — happens when a Spirit unloads mid-frame)

For each recipient that REJECTS, the address moves to a `rejected: Vec<RuptureRejection>` collection; for each that ACCEPTS, the address remains in the delivery slice. After per-recipient evaluation:
- If `accepted.is_empty() && !rejected.is_empty()` → ENTIRE frame is quarantined; sender receives ConsentRupture; original frame is NOT delivered
- If `!accepted.is_empty() && !rejected.is_empty()` → frame is DELIVERED to `accepted` (existing Phase 2 / Phase 3 pipeline); sender receives ConsentRupture capturing both slices
- If `!accepted.is_empty() && rejected.is_empty()` → normal delivery (no rupture; existing path unchanged)
- If `accepted.is_empty() && rejected.is_empty()` → caller error (empty `to` is rejected by `IacBusAdapter::deliver_typed` per Story 1b.1 substrate)

**And** the quarantine record is a typed Transparency Log row written BEFORE the ConsentRupture frame is dispatched to the sender (per I2 log-before-deliver). The TL row carries: `original_frame_id` + `quarantined_recipients: Vec<FrameAddress>` + `reasons: Vec<RuptureReason>` + `ruptured_at_ns`. This is NOT a silent drop — the operator surface SHOWS the rupture row.

**And** the ConsentRupture frame routes from the `IacBusAdapter::deliver_typed` to the SENDER's mailbox via the same `Mailbox::deliver` pipeline (recursive — the new frame goes through the SAME PARTIAL-CONSENT CHECK; the sender ACCEPTS its own ConsentRupture by default per the per-Spirit kernel-internal trust posture). The recursion is bounded: a ConsentRupture frame whose own delivery ruptures is logged as a `Critical` telemetry event and the cycle is broken at the second-level (NOT looping infinitely).
**And** intent_lineage on the new frame: the ConsentRupture carries the SAME `intent_lineage` chain as the original frame. The frame is not a new originating intent; it is a derived emission per Story 4.5's I13 substrate. The Story 6.2 100% lineage gate at `IacBusAdapter::deliver_typed:301-370` catches any rupture frame missing lineage.

**And** integration tests at `crates/maos-kernel-core/tests/consent_rupture_adr_034.rs` (10 scenarios):
- **3.1**: Single-recipient frame, recipient accepts → no rupture; existing path unchanged
- **3.2**: Two-recipient frame, both accept → no rupture; both receive the frame
- **3.3**: Two-recipient frame, recipient A accepts + B rejects (intent_allowlist_mismatch) → A receives; B does NOT; sender receives ConsentRupture with accepted=[A], rejected=[(B, IntentAllowlistMismatch)]; quarantine TL row written
- **3.4**: Two-recipient frame, both reject → entire frame quarantined; sender receives ConsentRupture with accepted=[], rejected=[(A, …), (B, …)]
- **3.5**: Recipient's posture shifts from `autonomous` (when sender decided) to `cautious` (when receiver evaluated) mid-frame → RuptureReason::PostureShiftedDuringTransmission
- **3.6**: Recipient's cap-token revoked between send and accept (call `cap_tokens::revoke_all(recipient_pid)` between phases) → RuptureReason::TokenRevoked
- **3.7**: Recipient unloads mid-frame (channel closed) → RuptureReason::RecipientUnloaded
- **3.8**: ConsentRupture frame's own delivery to sender ruptures (e.g., sender unloads between original frame send and rupture emission) → a `Critical` telemetry event logged; the cycle breaks at depth-2 (NO infinite recursion)
- **3.9**: intent_lineage preserved on the ConsentRupture frame — assert the rupture frame's `intent_lineage` matches the original frame's
- **3.10**: Multi-host frame (2 same-host + 1 cross-host recipient via Story 6.3 A2A) where the cross-host recipient rejects per ADR-012 EIntentDenied → ConsentRupture emitted with the cross-host recipient in `rejected`; same-host recipients receive normally; the A2A NACK is the per-frame-consent signal feeding the rupture detection

### AC4 — Per-(provider, credential) token-bucket rate-limit isolation (NFR-Scale-4)

**Given** the existing substrate at HEAD:
- `crates/maos-providers/src/lib.rs:14-23` exports `AnthropicProvider`, `OpenAiProvider`, `OllamaProvider`, `Provider` trait, `ProviderError` enum
- `crates/maos-providers/src/provider.rs:10-31` `Provider::complete(req: &InferenceRequest) -> Result<InferenceResponse, ProviderError>` with `ProviderError::ProviderRejected { status, body }` (used today for the existing 429 fixture case)
- `crates/maos-providers/src/anthropic.rs:18-25` `AnthropicProvider { api_key: String, base_url: String, model: String }` — credential carrier
- `crates/maos-bin/src/main.rs:477-507` composition root — providers wired into a `BTreeMap<String, Arc<dyn Provider>>` keyed by provider name (`"anthropic"`, `"openai"`, `"ollama"`)
- `crates/maos-kernel-core/src/inference/router.rs:14` `use maos_providers::{Provider, ProviderError};` — the kernel-side router that routes Spirit inference calls to providers
- `crates/maos-providers/tests/fixtures/multi-provider-v0/cases/case_provider_429_rate_limit.json` — existing rate-limit fixture (Story 5.5b)
- NFR-Scale-4 verbatim: "Provider rate-limit isolation — per-(provider, credential) token bucket; typed `RateLimited` IAC frame. v0.5."
- Architecture §4.4 verbatim: "Provider rate-limit isolation. Per-(provider, credential) token bucket with kernel-mediated backpressure surfaced as a typed `RateLimited` IAC frame, not a stalled call. One Spirit hitting Anthropic's RPM limit must not block another Spirit on a different provider, or even the same provider with a different key. Bucket parameters declared in provider driver config."

**When** Story 6.4 lands the rate-limit isolation surface

**Then** a new module at `crates/maos-providers/src/rate_limit.rs` ships the bucket substrate (LIVES IN `maos-providers` — this keeps the new code OUT of `maos-kernel-core` per the KLOC posture; the router CONSULTS the bucket via a trait):

```rust
//! Story 6.4 / NFR-Scale-4 — per-(provider, credential) token bucket.
//!
//! Each bucket is keyed by `(provider_id, credential_fingerprint)` where
//! `credential_fingerprint = first-8-bytes-of-SHA256(api_key_bytes)`. The
//! 8-byte prefix is sufficient for cross-credential disambiguation within
//! a single Host's bucket map (2^64 keyspace); the full credential is
//! NEVER stored beyond the existing provider driver's in-memory api_key field.
//!
//! Refill: continuous (per-second) at `refill_per_sec = capacity / refill_window_secs`.
//! At v0.5 the refill_window_secs is 60 (RPM semantics); TPM (token-per-minute)
//! is a future second bucket — out of scope per Story 6.4 (RPM-only at v0.5).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BucketKey {
    /// Stable provider identifier — "anthropic" | "openai" | "ollama" | ...
    pub provider_id: &'static str,
    /// First 8 bytes of SHA-256(api_key) as little-endian u64 — opaque key.
    pub credential_fingerprint: u64,
}

#[derive(Debug)]
pub struct TokenBucket {
    capacity: u32,
    refill_per_sec: f32,
    /// Lock-free `(remaining_x_1000, last_refill_ns)` packed into AtomicU64;
    /// upper 32 bits = remaining * 1000 (millitokens), lower 32 bits = (last_refill_ns >> 16).
    /// CAS loop on take/refill; lock-free; suitable for hot-path inference router gating.
    state: AtomicU64,
}

impl TokenBucket {
    pub fn new(capacity: u32, refill_per_sec: f32) -> Self { ... }
    /// Try to consume one token. Returns `Ok(())` on success, `Err(RetryAfter)` on empty.
    pub fn try_consume(&self) -> Result<(), RetryAfter> { ... }
    /// Read-only state for telemetry / `RateLimited` payload population.
    pub fn snapshot(&self) -> BucketSnapshot { ... }
}

pub struct ProviderRateLimiter {
    buckets: dashmap::DashMap<BucketKey, Arc<TokenBucket>>,
    config: ProviderRateLimitConfig,
}

#[derive(Debug, Clone)]
pub struct ProviderRateLimitConfig {
    /// Per-provider defaults; operator can override via env vars.
    pub per_provider: std::collections::HashMap<&'static str, ProviderQuota>,
}

#[derive(Debug, Clone, Copy)]
pub struct ProviderQuota {
    pub rpm: u32,  // requests per minute (e.g., Anthropic free tier ~50; tier-1 ~1000)
}

impl ProviderRateLimitConfig {
    /// Default configuration sourced from env vars:
    ///   - `MAOS_ANTHROPIC_RPM` (default 1000)
    ///   - `MAOS_OPENAI_RPM` (default 3500)
    ///   - `MAOS_OLLAMA_RPM` (default 999_999_999)  // local; effectively unbounded
    pub fn from_env() -> Self { ... }
}
```

**And** the kernel-side inference router consults the rate-limiter BEFORE forwarding to `Provider::complete`. The router lives at `crates/maos-kernel-core/src/inference/router.rs`; Story 6.4 EXTENDS it (the new code is in `maos-providers`; the router imports + consults — keeps the kernel-core LOC delta minimal per `xtask/kloc.toml`):

```rust
// Sketch — extends crates/maos-kernel-core/src/inference/router.rs
pub struct InferenceRouter {
    providers: BTreeMap<String, Arc<dyn Provider>>,
    rate_limiter: Arc<maos_providers::rate_limit::ProviderRateLimiter>,
    iac: Arc<crate::iac::IacBusAdapter>,
}

impl InferenceRouter {
    pub async fn dispatch(
        &self,
        invoking_spirit_id: &str,
        provider_id: &str,
        credential_fingerprint: u64,
        req: InferenceRequest,
    ) -> Result<InferenceResponse, InferenceError> {
        let key = BucketKey { provider_id: intern(provider_id), credential_fingerprint };
        match self.rate_limiter.try_consume(key) {
            Ok(()) => {
                // proceed to Provider::complete
            }
            Err(RetryAfter { retry_after_ms, snapshot }) => {
                // Emit RateLimited IAC frame to the invoking Spirit
                let payload = RateLimitedPayload { ... };
                let frame = IacFrame {
                    kind: FrameKind::RateLimited,
                    from: kernel_address(),
                    to: vec![FrameAddress::for_spirit(invoking_spirit_id)],
                    payload: ..., // serialized payload
                    intent: IntentClass::Standard,
                    intent_lineage: ..., // inherits from the invocation's intent_lineage
                    ...
                };
                self.iac.deliver_typed(frame).await?;
                return Err(InferenceError::RateLimited { retry_after_ms });
            }
        }
        ...
    }
}
```

**And** the inference router does NOT stall on rate-limit — the `RateLimited` frame emission is fire-and-forget (best-effort on a separate task) and the `dispatch` returns `Err(InferenceError::RateLimited { retry_after_ms })` SYNCHRONOUSLY. The Spirit author observes the rate-limit by EITHER catching the error from the kernel-side `infer.complete()` SDK call OR by handling the asynchronous `RateLimited` IAC frame in `on_frame` — both paths are guaranteed to fire (the error is synchronous; the frame is asynchronous but eventually-delivered)
**And** the bucket key keeps cross-credential isolation structural — `Spirit A` invoking with credential K1 hits its own bucket; `Spirit B` invoking with credential K2 (different `credential_fingerprint`) hits a DIFFERENT bucket. The bucket is INDEPENDENT of the invoking Spirit's identity; isolation is per-credential
**And** when the `Provider::complete` itself returns `ProviderError::ProviderRejected { status: 429, body }` (the provider-side rate-limit response, e.g., Anthropic's HTTP 429), the router DOES NOT decrement the bucket twice — it treats the provider's 429 as an additional signal AND emits a `RateLimited` frame with `retry_after_ms` parsed from the `retry-after` header (when present) OR a default `5000ms` fallback
**And** the composition root at `crates/maos-bin/src/main.rs` constructs the `ProviderRateLimiter` from `ProviderRateLimitConfig::from_env()` and threads it into the `InferenceRouter` (NEW field on the router; the existing constructor signature extends additively with a `with_rate_limiter` builder method per the existing Story 6.3 `Mailbox::with_a2a_router` precedent)
**And** the credential_fingerprint is computed AT PROVIDER-CONSTRUCTION time (when `with_api_key` is called) and stored on the provider struct — the router asks the provider for its `credential_fingerprint() -> u64` (NEW trait method on `Provider` with a default-impl returning `0` for backward compat; each concrete provider OVERRIDES with `sha256(api_key) -> u64 prefix`). Note: this is a backward-compatible `Provider` trait extension via DEFAULT method (NOT a breaking change; `cargo-public-api --diff` reports as `Changed` not `Removed`)
**And** integration tests at `crates/maos-providers/tests/rate_limit_isolation_nfr_scale_4.rs` (8 scenarios):
- **4.1**: Two spirits using SAME provider + SAME credential; spirit A exhausts bucket (10 calls / capacity 10); assert spirit B's 11th call across the same bucket key returns `Err(InferenceError::RateLimited)`
- **4.2**: Two spirits using SAME provider + DIFFERENT credentials (K1, K2); spirit A exhausts K1's bucket; assert spirit B's call on K2 SUCCEEDS (independent buckets)
- **4.3**: Two spirits using DIFFERENT providers (Anthropic, OpenAI); spirit A exhausts Anthropic; assert spirit B's call to OpenAI SUCCEEDS (independent buckets)
- **4.4**: Bucket refill: capacity 10 + refill_per_sec=1.0; exhaust bucket; wait 5s (via `tokio::time::pause` + `advance(5s)`); assert 5 tokens regenerated (5 calls succeed; 6th returns RateLimited)
- **4.5**: Concurrent contention: 100 tasks racing on a single bucket with capacity 50; assert exactly 50 succeed AND 50 receive RateLimited frames (lock-free CAS correctness)
- **4.6**: Provider returns HTTP 429 with `retry-after: 30` header; assert the emitted `RateLimitedPayload.retry_after_ms = 30_000`
- **4.7**: Provider returns HTTP 429 without retry-after header; assert default 5000ms retry_after
- **4.8**: `RateLimited` IAC frame round-trip: emit frame; subscribe a Spirit; assert the frame is delivered to the invoking Spirit with intent_lineage preserved and the FrameKind discriminant = 23

### AC5 — Smoke arm + discipline sweep + dev-record discipline + Review Findings populated

**Given** Story 6.4 adds CI jobs `nfr-scale-4-rate-limit-isolation`, `fr26-schedule-firing-corpus`, `adr-034-consent-rupture-corpus`, plus the new `smoke-schedule-6-4` smoke arm. Net new CI jobs: 4
**And** the smoke-arm proliferation pattern from `[[project_epic_5_retro_outcomes]]` + Story 6.1 / 6.2 / 6.3 carry-forward continues per `[[feedback_lunarpulse_observability_preference]]`

**When** the dev completes AC1–AC4 and runs the full discipline sweep

**Then** all discipline.yml jobs (current+4 from Story 6.4) are GREEN at HEAD — explicit `gh run watch` conclusion cited verbatim in the dev record per Epic 1b retro §A8 + Story 6.1 / 6.2 / 6.3 AC7 precedent
**And** `cargo-public-api --diff` reports: `Added` (`FrameKind::ConsentRupture` + `FrameKind::RateLimited`; `ScheduleEntry` + `SchedulesSection` + `ScheduleFireRecord` + `ScheduleWatchdog`; `ConsentRupturePayload` + `RuptureRejection` + `RuptureReason`; `RateLimitedPayload`; `ProviderRateLimiter` + `TokenBucket` + `BucketKey` + `ProviderRateLimitConfig` + `ProviderQuota`; `Provider::credential_fingerprint` (default-impl method)); `Removed` = 0; `Changed` = 2 (`SpiritManifestBundle` adds `schedules` field with `#[serde(default)]`; `Provider` trait adds default-method `credential_fingerprint`)
**And** `cargo run -p xtask -- check-empty-kernel` PASSES — Story 6.4 introduces NO new persistent kernel state outside I9-sanctioned locations. The rate-limit bucket map lives in-memory in `maos-providers` (transient per-process state — same I9 posture as TOFU pin store in Story 6.3). The ScheduleWatchdog's `last_fire_ns` per-`(spirit_pid, schedule_id)` lives on the SCB (existing I9-exempt surface)
**And** `cargo run -p xtask -- check-service-boundary` PASSES — Story 6.4 does NOT change any service boundary; `ProviderRateLimiter` lives in `maos-providers` (existing crate); `ScheduleWatchdog` lives in `maos-kernel-core::scheduler` (existing module). No new P1/P2/P3/P4 violations
**And** `cargo run -p xtask -- check-fr47` PASSES — Story 6.4 introduces NO new FR47-denied dependencies (`cargo tree -p maos-providers` shows no new `mcp` / `jsonrpc` / `hyper` / `axum` / `tonic` deps; the rate-limit substrate is hand-rolled atomics + dashmap (existing dep))
**And** `cargo run -p xtask -- check-unsafe` PASSES — every new file declares `#![forbid(unsafe_code)]` at the top
**And** `cargo run -p xtask -- check-workspace-count` PASSES — Story 6.4 does NOT add a new workspace crate
**And** `cargo run -p xtask -- kloc-check` reports — `maos-providers` ceiling is 2000 LOC. Story 6.4's rate-limit additions should land WELL under (target ≤300 LOC for `rate_limit.rs`). `maos-kernel-core` is already over-ceiling (~21K vs 6K per `xtask/kloc.toml`); Story 6.4's additions (the ScheduleWatchdog + Mailbox::deliver Phase 1.5 extension) MUST be minimal. Per `xtask/kloc.toml` interim-posture: "Stories that ADD to `maos-kernel-core` MUST first extract a candidate module netting to ≤0 added LOC." Story 6.4 LANDS the new code without extraction (the discipline-gate continues to FAIL on `maos-kernel-core` — same posture as Stories 6.1 / 6.2 / 6.3); surface to Lunarpulse and document in dev record
**And** `cargo run -p xtask -- check-serde-error-handling` PASSES — ZERO new `.unwrap_or_default()` on serde paths. The `[[schedule]]` manifest parsing path is the highest-risk surface; the gate confirms zero regressions. Existing pre-existing-debt counts are unchanged
**And** `cargo run -p xtask -- check-review-findings-resolved` PASSES — Story 6.4's Review Findings table has zero `**open**` Critical/High rows at sprint-status `done` transition
**And** `cargo run -p xtask -- check-dev-record-completeness` PASSES — the `dev_model_used:` frontmatter, `### Agent Model Used`, `### Completion Notes List`, `### File List` are populated per the §A6 contract
**And** a new `MAOS_ONE_SHOT=smoke-schedule-6-4` arm lands in `crates/maos-bin/src/main.rs` (extending the known-modes table around the existing `smoke-a2a-loopback-6-3` arm):
  - Constructs a fake Spirit with manifest `[[schedule]]` entries: `morning-digest` (cadence_secs=1, fast-mode) + `arxiv-watcher` (cadence_secs=2, fast-mode)
  - Spawns the `ScheduleWatchdog` with `MAOS_SCHEDULE_FAST=1`
  - Demonstrates **schedule firing**: waits ~150ms; asserts `morning-digest` fired AT LEAST ONCE; asserts the per-firing cap-token was issued with the manifest's `side_effect_scopes`; asserts a TL row `FrameKind::CapabilityInvocation` carries the `schedule_fire` record
  - Demonstrates **rate-limit cap**: `morning-digest` with `rate_limit_per_hour = 1`; in fast-mode this means at most 1 fire per ~36s; asserts only 1 fire happens in the smoke window
  - Demonstrates **ConsentRupture**: constructs a 2-recipient frame where recipient B has `cap_tokens::revoke_all(B.pid)` called between send and accept; asserts:
    - The frame's accepted-slice (recipient A) receives the frame
    - The sender receives a `FrameKind::ConsentRupture` frame on its mailbox
    - The TL has a quarantine row for recipient B
    - `RuptureReason::TokenRevoked` populates the payload
  - Demonstrates **RateLimited**: construct a `ProviderRateLimiter` with capacity=2 + refill=0; issue 3 consume calls; assert 2 succeed; assert 3rd emits a `FrameKind::RateLimited` frame to the invoking spirit AND returns `InferenceError::RateLimited`
  - Logs one line per surface confirming behavior; exits 0 on healthy substrate; exit code reported in the dev record
**And** a corresponding `smoke-schedule-6-4` discipline.yml job wires the smoke arm into CI with `timeout-minutes: 5`
**And** Task 6.5 lineage corpus extension: 10 NEW scenarios at `crates/maos-eval/fixtures/intent-lineage-corpus-v0/scenario-071..080.json` (10× `lineage_via_consent_rupture`) + 10 NEW scenarios at `scenario-081..090.json` (10× `lineage_via_rate_limited`) + 10 NEW scenarios at `scenario-091..100.json` (10× `lineage_via_on_schedule`). Reuse existing `LineageChainUninterrupted` class (additive, no enum variants needed). Discipline.yml gets NEW `intent-lineage-6-4-extension` job that runs `crates/maos-eval/tests/intent_lineage_corpus_load.rs`. Total `intent-lineage-corpus-v0` size grows from 70 to 100 scenarios (Story 6.3 left 70; Story 6.4 extends to 100)
**And** the story's `### Review Findings` table is populated via `bmad-code-review` skill execution — NOT left as `_No review findings._`. The §A5 gate (verified in AC1) blocks `done` while any `**open**` Critical/High row remains. Per `[[project_epic_5_retro_outcomes]]` + `[[feedback_mechanical_gates_compound_promises_decay]]` Story 6.4 MUST receive formal review — the surface (3 interacting features touching the IAC bus + manifest parsing + capability tokens + inference router) is integration-dense enough that the §A2 carry-forward debt cannot extend further
**And** the `dev_model_used:` frontmatter field is set to the ACTUAL model used at story-start (NOT left as `TBD*`); per `[[feedback_deepseek_v4_pro_patterns]]` AND Story 6.4's classification as a **3-feature integration story** (ScheduleWatchdog + ConsentRupture + RateLimitedFrame), **strong recommendation: `claude-opus-4-7`** (or current Claude Opus 4.x). The story is structurally less dense than 6.3 (3 surfaces vs 6.3's 6 surfaces) BUT touches more pre-existing code paths (Mailbox::deliver + IacBusAdapter::deliver_typed + inference router + manifest parser + cap_tokens). If the dev substitutes another model, the substitution decision logs into the dev record per Epic 4 retro §A3 / Story 6.1 / 6.2 / 6.3 precedent AND the `Test Infrastructure Auditor` review axis fires automatically per `bmad-code-review.user.toml` (Story 2.5 AC5) on non-Claude / non-Codex models
**And** `### File List` enumerates every file touched; `xtask check-dev-record-completeness` PASSES on the file list at sprint-status `done`

## Tasks / Subtasks

- [x] **Task 0** — Bridge precondition gate verification (AC1)
  - [x] 0.1 Extend `xtask/src/check_epic_6_bridge.rs` with the new `--story 6.4` flag; implement the 10 row classifications per AC1 (specific mechanical checks 1–10)
  - [x] 0.2 Update `.github/workflows/discipline.yml`'s `check-epic-6-bridge` job to invoke `--story 6.4` (matrix entry OR sibling job per the Story 6.3 pattern at line 911)
  - [x] 0.3 Run the AC1 gate at HEAD; cite the run output verbatim in dev record's Completion Notes List
  - [x] 0.4 Confirm §A3 gate PASSES at HEAD; if FAILS, STOP and surface — `[[schedule]]` parsing is a high-risk serde surface
  - [x] 0.5 Confirm 6.3-P4 (CI test-target verification) PASSES at HEAD; if FAILS, STOP and surface — every Story 6.4 PR would otherwise fail CI on this pre-existing breakage
  - [x] 0.6 Verify the three `blocking_6_4` substrate-canvas confirmations (PROVIDERS-BASELINE, FRAMEKIND-BASELINE, SCHEDULE-WATCHDOG-BASELINE)

- [x] **Task 1** — `[[schedule]]` manifest section + parsing (AC2 substrate)
  - [x] 1.1 Add `SchedulesSection` + `ScheduleEntry` + `RawSchedulesSection` + `RawScheduleEntry` types at `crates/maos-kernel-core/src/security/manifest.rs`
  - [x] 1.2 `RawScheduleEntry` uses `#[serde(deny_unknown_fields)]`; validation enforces `cadence_secs ∈ [1, 604_800]`, `rate_limit_per_hour ∈ [1, 3600]`, `id` regex `[a-zA-Z0-9_-]{1,64}`, cross-entry id uniqueness
  - [x] 1.3 `SpiritManifestBundle::schedules: SchedulesSection` additive field with `#[serde(default)]`
  - [x] 1.4 Round-trip unit tests in `crates/maos-kernel-core/src/security/manifest.rs::tests` (12 scenarios — extends AC2.1-2.8 with payload_b64 round-trip + sha256 prefix + id regex + two-entries)
  - [x] 1.5 `[lifecycle].enabled_hooks` MUST allow `"on_schedule"` for any `[[schedule]]` entry to fire — semantics documented; watchdog enforces (not parsing)

- [x] **Task 2** — `ScheduleWatchdog` runtime (AC2 firing)
  - [x] 2.1 NEW `crates/maos-kernel-core/src/scheduler/schedule_watchdog.rs` module — `ScheduleWatchdog` struct mirroring `IdleWatchdog`'s shape
  - [x] 2.2 `ScheduleWatchdog::new` + `::spawn` + `::check_and_fire`; default poll interval 1000ms; `MAOS_SCHEDULE_FAST=1` → 40ms (test-only)
  - [x] 2.3 Per-firing gate ordering: lifecycle gate → principal-revocability → rate-limit bucket → ComplianceClaim stamp → narrowed cap-token
  - [x] 2.4 `ScheduleFireRecord` payload variant at `crates/maos-kernel-core/src/iac/payload.rs` (additive)
  - [x] 2.5 TL row write on every fire (`FrameKind::CapabilityInvocation` reused; the row's payload carries `ScheduleFireRecord`)
  - [x] 2.6 Per-`(spirit_id, schedule_id)` token bucket for `rate_limit_per_hour` enforcement (lives on the ScheduleWatchdog — separate from AC4's provider buckets)
  - [x] 2.7 Composition root at `crates/maos-bin/src/main.rs` constructs + spawns `ScheduleWatchdog`; wired to the same `CancellationToken` as `IdleWatchdog`
  - [x] 2.8 8-scenario test at `crates/maos-kernel-core/tests/schedule_watchdog_fr26.rs` per AC2.1-2.8 (all 8 passing)

- [x] **Task 3** — `FrameKind::ConsentRupture` + `FrameKind::RateLimited` + payload types (AC3 + AC4 substrate)
  - [x] 3.1 Add `ConsentRupture = 22` and `RateLimited = 23` variants to `FrameKind` at `crates/maos-spirit-abi/src/identity.rs`; extend `from_u8` arms
  - [x] 3.2 Add channel-class rows for both variants at `crates/maos-kernel-core/src/iac/channels.rs` (both `Mpsc`, capacity 32)
  - [x] 3.3 Add `ConsentRupturePayload` + `RuptureRejection` + `RuptureReason` at `crates/maos-domain/src/frame.rs`
  - [x] 3.4 Add `RateLimitedPayload` at `crates/maos-domain/src/frame.rs`
  - [x] 3.5 Update `Mailbox::register_spirit` to allocate mpsc channels for the two new kinds (extending the `kinds: &[FrameKind]` slice at `mailbox.rs:106`)
  - [x] 3.6 Round-trip serde tests for both new frames

- [x] **Task 4** — ConsentRupture detection + emission in `Mailbox::deliver` (AC3)
  - [x] 4.1 Insert Phase 1.5 per-recipient consent gate in `Mailbox::deliver` (between Phase 1 partition and Phase 2 same-host delivery)
  - [x] 4.2 Per-recipient evaluation: pluggable `ConsentGate` trait that operators inject; default = accept-all (the 5 RuptureReason variants are typed for the operator to surface)
  - [x] 4.3 Partition recipients into `accepted` + `rejected: Vec<RuptureRejection>`
  - [x] 4.4 Quarantine TL row written BEFORE the ConsentRupture frame is emitted (I2 log-before-deliver)
  - [x] 4.5 Emit `ConsentRupture` frame back to sender (via the same `Mailbox::deliver`; recursion bounded at depth-2)
  - [x] 4.6 intent_lineage preserved on the rupture frame (copy from original)
  - [x] 4.7 11-scenario test at `crates/maos-kernel-core/tests/consent_rupture_adr_034.rs` per AC3.1-3.10 + serde round-trip (all 11 passing)

- [x] **Task 5** — `ProviderRateLimiter` substrate (AC4)
  - [x] 5.1 NEW `crates/maos-providers/src/rate_limit.rs` — `TokenBucket` (lock-free CAS on packed AtomicU64), `BucketKey`, `ProviderRateLimiter`, `ProviderRateLimitConfig`, `ProviderQuota`
  - [x] 5.2 `ProviderRateLimitConfig::from_env()` reads `MAOS_ANTHROPIC_RPM` / `MAOS_OPENAI_RPM` / `MAOS_OLLAMA_RPM` with documented defaults
  - [x] 5.3 Add `credential_fingerprint(&self) -> u64` default-method (returning 0) to the `Provider` trait at `crates/maos-providers/src/provider.rs`; override in each concrete provider (Anthropic / OpenAI / Ollama) to return `sha256(api_key)` first-8-bytes-as-u64-LE
  - [x] 5.4 Extend `crates/maos-kernel-core/src/inference/mod.rs::InferencePortAdapter` to consult `ProviderRateLimiter::try_consume(key)` BEFORE forwarding to `Provider::complete` (router itself stays bucket-agnostic per ADR-010)
  - [x] 5.5 On `Err(RetryAfter)` — emit `RateLimited` IAC frame to invoking Spirit (fire-and-forget) AND return `InferenceError::RateLimited { retry_after_ms }` SYNCHRONOUSLY
  - [ ] 5.6 On `ProviderError::ProviderRejected { status: 429, body }` from the provider — emit `RateLimited` frame with `retry_after_ms` parsed from header (default 5000ms); do NOT double-decrement the bucket (DEFERRED — bucket-side gate already handles in-process exhaustion; provider-side 429 handling logged as carry-forward per the AC4.6/4.7 partial coverage)
  - [x] 5.7 Composition root wires `ProviderRateLimiter` into `InferencePortAdapter` via `with_rate_limiter` + `with_iac` builders
  - [x] 5.8 8-scenario test at `crates/maos-providers/tests/rate_limit_isolation_nfr_scale_4.rs` per AC4.1-4.8 (all 8 passing) + 6 unit tests in rate_limit.rs

- [x] **Task 6** — Cross-cutting: lineage corpus extension (AC5)
  - [x] 6.1 10 NEW scenarios at `intent-lineage-corpus-v0/scenario-071..080.json` (10× `lineage_via_consent_rupture`)
  - [x] 6.2 10 NEW scenarios at `scenario-081..090.json` (10× `lineage_via_rate_limited`)
  - [x] 6.3 10 NEW scenarios at `scenario-091..100.json` (10× `lineage_via_on_schedule`)
  - [x] 6.4 Discipline.yml `intent-lineage-6-4-extension` job runs `intent_lineage_corpus_load.rs` and asserts the corpus loads to ≥100 scenarios with all three new classes present (matching count = 10 each) — added 2 new test scenarios to existing `intent_lineage_corpus_load.rs`; CI job wired in Task 8

- [x] **Task 7** — Smoke arm + dev-record discipline (AC5)
  - [x] 7.1 `MAOS_ONE_SHOT=smoke-schedule-6-4` arm at `crates/maos-bin/src/main.rs` — extends known-modes table; demonstrates: schedule fire + rate-limit cap + ConsentRupture + RateLimited per the AC5 enumeration
  - [x] 7.2 `smoke-schedule-6-4` job in `.github/workflows/discipline.yml` with `timeout-minutes: 5`; `aggregate.needs:` extended
  - [ ] 7.3 `bmad-code-review` skill execution (deferred to post-dev review step per workflow Step 10)
  - [ ] 7.4 Resolve every Critical/High Review Finding inline (post-review)
  - [x] 7.5 `dev_model_used: claude-opus-4-7` in frontmatter (set at story-start)
  - [x] 7.6 `### Agent Model Used`, `### Completion Notes List`, `### File List` populated per §A6 contract

- [x] **Task 8** — Discipline sweep + sprint-status update (AC5 close)
  - [x] 8.1 `cargo build` succeeds; 38 new Story 6.4 tests (8 watchdog + 11 rupture + 8 rate-limit + 6 rate-limit-unit + 12 manifest + 2 ScheduleFireRecord + 2 lineage corpus + smoke) all PASS
  - [x] 8.2 `check-epic-6-bridge --story 6.4` PASSES (cited verbatim in Completion Notes); pre-existing Epic 5 / 6 carry-forward debt remains documented
  - [ ] 8.3 `gh run watch` — full discipline.yml sweep deferred to PR (post-review); local sweep results documented in Completion Notes
  - [x] 8.4 sprint-status `6-4-…` → `review`
  - [x] 8.5 epic-6 status remains `in-progress` (Story 6.5 + retro still pending)

## Dev Notes

### Model Recommendation

**Recommendation: `claude-opus-4-7` (or current Claude Opus 4.x)**

**Why:** Story 6.4 is structurally less dense than 6.3 (3 surfaces vs 6.3's 6) BUT integrates with MORE pre-existing code paths: `Mailbox::deliver` (touched again — its second multi-recipient phase since Story 6.3's cross-host partition); `IacBusAdapter::deliver_typed` (Story 4.5 / 6.2 substrate); manifest parser (`[[schedule]]` added alongside the 12 existing sections); `cap_tokens` issue/revoke (per-firing narrowing); `InferenceRouter` (gating point); `Provider` trait (default-method extension). Per `[[feedback_deepseek_v4_pro_patterns]]`, deepseek-v4-pro's weakness profile intersects ALL THREE risk surfaces:
- (a) **Async invariants** — ScheduleWatchdog's poll loop + `tokio::time::pause()` test correctness + the lock-free `TokenBucket`'s CAS loop (must terminate without starvation; `compare_exchange` ordering matters)
- (b) **Integration plumbing** — Mailbox Phase 1.5 insertion ordering (consent check BEFORE same-host delivery is the I2 invariant; getting the ordering wrong silently breaks log-before-deliver); ConsentRupture recursion bounding (the rupture-on-rupture cycle must break at depth-2)
- (c) **Env-var threading** — `MAOS_SCHEDULE_FAST=1` parity with `MAOS_IDLE_FAST=1` + `MAOS_ANTHROPIC_RPM` / `MAOS_OPENAI_RPM` / `MAOS_OLLAMA_RPM` defaults + the `ProviderRateLimitConfig::from_env()` shape

Per Story 6.1 + 6.2 + 6.3 precedent, all three completed cleanly on claude-opus-4-7. The pattern is now strong enough to be predictive: dense Epic 6 integration → Claude Opus 4.x. **Story 6.4 is the fourth dense integration in Epic 6; do not substitute** unless the substitute can clear the same TaskInfra/Auditor bar Story 5.5d's deepseek substitution failed.

**If the dev substitutes:** Log the substitution decision in the dev record per Epic 4 retro §A3 pattern + Story 6.1 / 6.2 / 6.3 precedent. The `Test Infrastructure Auditor` review axis fires automatically per `bmad-code-review.user.toml` (Story 2.5 AC5) on any non-Claude / non-Codex model. Recommend running A4 parallel-review-agents (Blind Hunter + Edge Case Hunter + Acceptance Auditor + Test Infrastructure Auditor) regardless of dev model.

### Architecture Compliance

**Relevant architecture sections (verbatim references):**

- `architecture-maos-minimal-opus/4-kernel-design.md` §4.4 — Provider rate-limit isolation paragraph (the verbatim source for AC4)
- `architecture-maos-minimal-opus/4-kernel-design.md` §4.5 — IAC Bus partial-consent failure semantics paragraph (the verbatim source for AC3)
- `architecture-maos-minimal-opus/5-spirit-abi.md` §5.3 table — `on_schedule | A scheduled invocation fires | Run periodic task | Story 2.1 (signature), Story 5.1 (runtime)` (the FR26 ABI substrate Story 6.4 brings to firing)
- `architecture-maos-minimal-opus/7-inter-agent-communication.md` §7.1.1 — Per-frame-kind channel class (Story 6.4 adds `ConsentRupture` + `RateLimited` rows aligned to `consent.request` cardinality — Mpsc 1:1 capacity 32)
- `architecture-maos-minimal-opus/12-architecture-decision-records.md` ADR-025 (binding-v0.3 · Gate: Butler on_idle Sandra-scene replay; on_schedule fires at declared cadence) — verbatim source for AC2
- `architecture-maos-minimal-opus/12-architecture-decision-records.md` ADR-034 (binding-v0.9 · Gate: sender receives `ConsentRupture` IAC frame on receiver-side rejection) — verbatim source for AC3
- `_bmad-output/planning-artifacts/prd/functional-requirements.md` FR26 — verbatim binding for AC2
- `_bmad-output/planning-artifacts/prd/non-functional-requirements.md` NFR-Scale-4 — verbatim binding for AC4
- `_bmad-output/planning-artifacts/prd/domain-specific-requirements.md` "Provider rate-limit isolation (Winston's addition)" — AC4 rationale

**Invariants Story 6.4 must preserve:**

- **I1 — Every capability invocation through the registry:** The per-firing narrowed cap-token for `on_schedule` (AC2) IS issued via `cap_tokens::issue` like every other capability — Story 6.4 does NOT bypass I1. The rate-limit gate (AC4) lives ABOVE the I1 surface (the bucket consume happens BEFORE the `Provider::complete` call which itself is gated by I1)
- **I2 — Log-before-deliver:** Every quarantine row + every `ConsentRupture` frame + every `RateLimited` frame is written to the TL BEFORE delivered to the sender's mailbox. The Phase 1.5 insertion in `Mailbox::deliver` does NOT bypass I2 — the quarantine TL row is the I2 evidence for the rejected slice
- **I8 — Cross-Host A2A interactions require explicit consent at both ends:** Story 6.4's ConsentRupture is the THIRD failure-class (alongside ADR-022 crash detection + `task.orphaned`) of I8; same-Host frames inherit I8 via the existing kernel-internal trust posture (NO consent envelope on same-Host frames at v0.5; the consent gate evaluates posture / token-revocation / principal-revocation regardless)
- **I9 — Empty kernel:** All Story 6.4 state is transient per-process (ScheduleWatchdog's last_fire_ns on SCB; rate-limit buckets in dashmap); NO new persistent kernel state outside I9-sanctioned locations
- **I13 — Intent provenance:** Every Story 6.4 frame carries unbroken `intent_lineage`. ConsentRupture inherits from the original frame; RateLimited inherits from the invocation's `intent_lineage`; on-schedule fires originate a NEW lineage chain anchored to the schedule_id (the originating intent IS the schedule itself, per ADR-025's "kernel-mediated scheduling lets the operator surface scheduled work in audit")
- **I14 — Halt continuity:** Story 6.4 does NOT touch the EpistemicHalt channel; ConsentRupture / RateLimited are independent of halt continuity

**ADRs governing Story 6.4:**

- **ADR-025** — Proactive scheduling binding-v0.3 → AC2 lands the firing path
- **ADR-034** — Partial-consent failure semantics binding-v0.9 → AC3 implements
- **ADR-022** — Failure semantics floor → ConsentRupture is the third failure-class alongside crash detection / `task.orphaned`
- **ADR-023** — Capability-token TTL bind-to-PID → AC2 per-firing cap-token TTL = `2 × cadence_secs` capped at 300s (Standard intent_class)
- **ADR-010** — Hexagonal architecture → `ProviderRateLimiter` substrate lives in `maos-providers` (adapter side); the rate-limit interface to the inference router is via direct construction (NOT a port trait — same-process synchronous; no need for the hexagonal split for an in-crate substrate). Per Story 6.3 review-pattern, if the dev judges a port-trait split warranted, the trait lives in `maos-domain::ports::inference_rate_limit` (NEW port at v0.5)
- **ADR-016** — Token-budget accounting → AC4 is the LLM-substrate analog; the bucket is a request-counter not a token-counter at v0.5 (TPM is future)
- **ADR-038** — Per-service KLOC ceiling → see `xtask/kloc.toml` interim posture in AC5

### Library / Framework Requirements

| Surface | Crate | Version | Notes |
|---|---|---|---|
| Runtime | `tokio` | workspace pin | reuse existing; `tokio::time::interval` + `tokio::time::pause()` for tests |
| Cancellation | `tokio-util` | workspace pin | reuse existing for the watchdog spawn-cancel pattern (IdleWatchdog precedent) |
| Map | `dashmap` | workspace pin | reuse existing for `ProviderRateLimiter::buckets` + ScheduleWatchdog rate-limit map |
| Hash | `sha2` | workspace pin | reuse existing for `credential_fingerprint` computation; first-8-bytes-as-u64-LE projection |
| Atomics | `std::sync::atomic::AtomicU64` | std | TokenBucket's packed (millitokens, last_refill_ns_shifted) CAS state |
| Errors | `thiserror` | workspace pin | reuse existing for `InferenceError`, `ManifestError` |
| Async traits | `async-trait` | workspace pin | reuse existing |
| Serde | `serde` + `serde_json` + `toml` | workspace pin | reuse existing for `[[schedule]]` parsing |
| Base64 | `base64` | workspace pin if present; else NONE needed | only used for `payload_b64` field; if no workspace dep, hand-roll a tiny decoder OR omit the b64 wrap and accept `payload` as a JSON-string at v0.5 (FOLLOW EXISTING WORKSPACE CONVENTION — verify before adding) |

**NEW dependencies:** ZERO new workspace crates. ZERO new `[dependencies]` entries — Story 6.4 reuses dashmap / sha2 / tokio / serde / thiserror that already exist. If `base64` is not in the workspace, the dev DECIDES: hand-roll a 30-line decoder OR change `payload_b64` to `payload_hex` (which can use the existing hex crate). Document the decision per Epic 4 retro §A3 pattern.

**FR47 verification:** `cargo tree -p maos-providers` MUST report no new `mcp` / `jsonrpc` / `reqwest` / `hyper` / `axum` / `warp` / `tonic` deps. Verify via AC5's `check-fr47` gate.

### File Structure Requirements

| Path | New / Update | AC |
|---|---|---|
| `crates/maos-spirit-abi/src/identity.rs` | UPDATE | AC3 (FrameKind::ConsentRupture=22, RateLimited=23) |
| `crates/maos-domain/src/frame.rs` | UPDATE | AC3 (ConsentRupturePayload, RuptureRejection, RuptureReason, RateLimitedPayload) |
| `crates/maos-kernel-core/src/iac/channels.rs` | UPDATE | AC3 (channel_class_for: new rows for kinds 22, 23) |
| `crates/maos-kernel-core/src/iac/mailbox.rs` | UPDATE | AC3 (Mailbox::register_spirit kinds slice + Phase 1.5 per-recipient consent gate + ConsentRupture emission) |
| `crates/maos-kernel-core/src/iac/payload.rs` | UPDATE | AC2 (ScheduleFireRecord) |
| `crates/maos-kernel-core/src/iac/mod.rs` | UPDATE | AC3 (deliver_typed integration with Phase 1.5 detection) |
| `crates/maos-kernel-core/src/security/manifest.rs` | UPDATE | AC2 (SchedulesSection + ScheduleEntry + RawSchedulesSection + RawScheduleEntry; SpiritManifest::schedules field) |
| `crates/maos-kernel-core/src/scheduler/control_block.rs` | UPDATE | AC2 (SpiritManifestBundle::schedules field) |
| `crates/maos-kernel-core/src/scheduler/schedule_watchdog.rs` | **NEW** | AC2 |
| `crates/maos-kernel-core/src/scheduler/mod.rs` | UPDATE | AC2 (pub mod schedule_watchdog) |
| `crates/maos-kernel-core/src/inference/router.rs` | UPDATE | AC4 (rate_limiter field + dispatch consults bucket before Provider::complete) |
| `crates/maos-kernel-core/tests/schedule_watchdog_fr26.rs` | **NEW** | AC2 (8 scenarios) |
| `crates/maos-kernel-core/tests/consent_rupture_adr_034.rs` | **NEW** | AC3 (10 scenarios) |
| `crates/maos-providers/src/lib.rs` | UPDATE | AC4 (pub mod rate_limit; re-export) |
| `crates/maos-providers/src/rate_limit.rs` | **NEW** | AC4 |
| `crates/maos-providers/src/provider.rs` | UPDATE | AC4 (Provider::credential_fingerprint default-method) |
| `crates/maos-providers/src/anthropic.rs` | UPDATE | AC4 (override credential_fingerprint) |
| `crates/maos-providers/src/openai.rs` | UPDATE | AC4 (override credential_fingerprint) |
| `crates/maos-providers/src/ollama.rs` | UPDATE | AC4 (override credential_fingerprint — uses base_url hash since Ollama has no api_key) |
| `crates/maos-providers/tests/rate_limit_isolation_nfr_scale_4.rs` | **NEW** | AC4 (8 scenarios) |
| `crates/maos-eval/fixtures/intent-lineage-corpus-v0/scenario-071..080.json` | **NEW** (10 files) | AC5 / Task 6 |
| `crates/maos-eval/fixtures/intent-lineage-corpus-v0/scenario-081..090.json` | **NEW** (10 files) | AC5 / Task 6 |
| `crates/maos-eval/fixtures/intent-lineage-corpus-v0/scenario-091..100.json` | **NEW** (10 files) | AC5 / Task 6 |
| `crates/maos-bin/src/main.rs` | UPDATE | AC2 (composition root: ScheduleWatchdog spawn) + AC4 (composition root: ProviderRateLimiter wired to InferenceRouter) + AC5 (smoke-schedule-6-4 arm) |
| `xtask/src/check_epic_6_bridge.rs` | UPDATE | AC1 (--story 6.4 flag + 10 new row classifications) |
| `.github/workflows/discipline.yml` | UPDATE | AC1/AC5 (4 new jobs: smoke-schedule-6-4, fr26-schedule-firing-corpus, adr-034-consent-rupture-corpus, nfr-scale-4-rate-limit-isolation, intent-lineage-6-4-extension; `aggregate.needs:` extended; `check-epic-6-bridge --story 6.4` invocation added) |
| `_bmad-output/implementation-artifacts/sprint-status.yaml` | UPDATE | AC5 (6-4 status transitions) |

### Testing Requirements

- **`[[schedule]]` manifest parsing (AC2 Task 1):** 8 unit tests in `manifest.rs::tests` covering well-formed; deny-unknown-fields; cadence-out-of-range (0, 604801); rate-limit-out-of-range (0, 3601); duplicate id; empty section parses to default; `compliance_claim_ref_hex` malformed length; `side_effect_scopes` Scope round-trip
- **ScheduleWatchdog firing (AC2 Task 2):** 8 integration tests at `crates/maos-kernel-core/tests/schedule_watchdog_fr26.rs`. Use `tokio::test(start_paused=true)` + `tokio::time::advance()` for deterministic cadence reproduction. `MAOS_SCHEDULE_FAST=1` parity with `MAOS_IDLE_FAST=1` (Story 5.1 precedent) — collapses cadence by 100×; document in dev notes
- **ConsentRupture detection (AC3 Task 4):** 10 integration tests at `crates/maos-kernel-core/tests/consent_rupture_adr_034.rs`. Use deterministic 2-recipient frames + manually-revoked cap-tokens between phases. The recursion-bound test (3.8) requires synchronous-style mocking of "sender unloaded between original and rupture"
- **ProviderRateLimiter (AC4 Task 5):** 8 integration tests at `crates/maos-providers/tests/rate_limit_isolation_nfr_scale_4.rs`. Use `tokio::time::pause()` + `tokio::time::advance(5s)` for bucket refill determinism (rule: do NOT use wall-clock in tests per Story 6.3 NFR-Sec-13 calibration-mode pattern). Cross-credential isolation tests (4.2) require constructing two distinct `AnthropicProvider` instances with distinct keys
- **Smoke arm (AC5):** End-to-end demonstration — schedule fire + rate-limit cap + ConsentRupture + RateLimited; the smoke arm IS the observable wedge per `[[feedback_lunarpulse_observability_preference]]`. Each surface gets one demonstrative call; the discipline-job `smoke-schedule-6-4` runs the arm with `timeout-minutes: 5`
- **Lineage corpus (Task 6):** 30 NEW scenarios (10 each for the three new lineage classes). `intent_lineage_corpus_load.rs` loader extends to scan the full 100-scenario corpus and asserts class counts

### Previous-Story Intelligence

From **Story 6.3** (`6-3-build-the-a2a-peer-mesh-from-loopback-to-cross-host-with-mtls-rotation-chaos.md`):
- `Mailbox::deliver` Phase 1/2/3 partition pattern (lines 133-255) is the STRUCTURAL precedent — Story 6.4's Phase 1.5 per-recipient consent gate inserts BETWEEN Phase 1 and Phase 2
- `ConsentEnvelope` was extended additively with `intent_class` + `valid_until_ns` (`#[serde(default)]`) — Story 6.4 READS the envelope's `intent_class` in the per-recipient consent check (when present)
- Story 6.3 left 3 decision-needed items + 22 patches OPEN in Review Findings — Story 6.4 AC1 reports state; per Story 6.3 spec line 642 the §A5 gate blocks Story 6.3 from `done` while Critical/High remain open. Story 6.4 does NOT block on Story 6.3's remediation; it carries its own
- `boot_nonce` JSON-RPC header path at v0.5 (Story 6.3 D-decision) — orthogonal to Story 6.4
- Story 6.3 introduced 4 new discipline.yml jobs + 1 smoke arm; Story 6.4 adds 4 new jobs + 1 smoke arm (parallel cadence)

From **Story 6.2** (`6-2-…orchestrator-distillates….md`):
- `IacBusAdapter::deliver_typed`'s `check_orchestrator_distillate_required` gate (line 301-370) is the PRECEDENT for inserting a kernel-side gate BEFORE the I13 lineage check — Story 6.4's per-recipient consent gate follows the same structural pattern (mailbox-side, not deliver_typed-side; the consent gate is per-recipient not per-frame, so the natural home is `Mailbox::deliver`)
- 50+ scenario `intent-lineage-corpus-v0/` substrate (Story 6.3 EXTENDED to 70; Story 6.4 EXTENDS to 100)
- `FrameKind::CliSubprocessOutput = 21` discriminant addition is the additive-variant precedent — Story 6.4 ADDS 22 + 23

From **Story 6.1** (`6-1-…full-iac-bus….md`):
- `channel_class_for(kind) -> Option<(ChannelClass, capacity)>` const-table at `crates/maos-kernel-core/src/iac/channels.rs` — Story 6.4 ADDS rows for kinds 22 + 23
- `Mailbox::register_spirit` allocates mpsc channels per kind from a slice (mailbox.rs:106) — Story 6.4 EXTENDS the slice
- Story 6.1's 9-bridge AC1 gate compounds in 6.4 with the `--story 6.4` flag; the gate's check function gains a new `match` arm per story number

From **Story 5.1** (`5-1-…lifecycle-verbs-and-11-triggers….md`):
- `IdleWatchdog` at `crates/maos-kernel-core/src/scheduler/idle_watchdog.rs` — the STRUCTURAL twin of Story 6.4's `ScheduleWatchdog`. Story 6.4 mirrors:
  - `MAOS_IDLE_FAST=1` → `MAOS_SCHEDULE_FAST=1` (same 100× cadence collapse)
  - `Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>` scbs handle
  - `Arc<HookDispatcher>` dispatcher
  - `tokio::spawn` with `CancellationToken` and `tokio::time::interval` ticker
  - SCB candidates collected outside the lock (avoid holding RwLock across await per Story 5.1's review-backfill lesson)
- `HookDispatcher::fire_on_schedule(scb, payload)` exists and is exercised in `MAOS_ONE_SHOT` smoke arms; Story 6.4 hooks it into the cadence loop

From **Story 4.5** (`4-5-…cross-spirit-isolation-200-corpus….md`):
- intent_lineage I13 enforcement at `IacBusAdapter::deliver_typed:301-370` — Story 6.4's new frames (`ConsentRupture`, `RateLimited`, on-schedule cap-invocations) MUST carry unbroken lineage; the 100% gate from Story 6.2 catches violations

From **Story 5.5b** (`5-5b-…multi-provider-ci-matrix….md`):
- `case_provider_429_rate_limit.json` fixture exists at `crates/maos-providers/tests/fixtures/multi-provider-v0/cases/` — Story 6.4's AC4 test 4.6 leverages the existing fixture pattern for the HTTP-429 → `RateLimited` frame path

From **Epic 5 retro** (`epic-5-retro-2026-05-24.md`):
- 6 of 9 stories shipped without formal review — Epic 6 MUST NOT repeat. Story 6.4 AC5 EXPLICITLY requires `bmad-code-review` skill execution
- Mechanical gates compound; promises decay per `[[feedback_mechanical_gates_compound_promises_decay]]` — Story 6.4 ships 4 new discipline gates inline (NOT promising future shipping)

From **Epic 6 preparation** (`[[project_epic_6_preparation]]`):
- §A1/§A2 bridge work + §A3/§A5/§A6 gates land before Story 6.1; verified at HEAD per Story 6.3 AC1 evidence
- §A4 Phase-2 maos-capability extraction precondition Story 6.1; SHIPPED (per `xtask/kloc.toml` `phase_2 = { ..., status = "done" }`)
- Phase 1 (`maos-iac` + `maos-manifest` extraction) is Story 6.5 territory; Story 6.4 does NOT touch the kernel-core extraction boundary — additions land in EXISTING `maos-kernel-core` (KLOC-debt inherited; gate continues to fail per `xtask/kloc.toml` interim posture)

### Git Intelligence

Recent commit log (HEAD-25 walk):

```
79fc591 6-3-build-the-a2a-peer-mesh-from-loopback-to-cross-host-with-mtls-rotation-chaos   ← Story 6.3 ships; substrate Story 6.4 builds on
d3c77c1 6-2-dispatch-orchestrator-distillates-with-intent-lineage-and-cliwrapperspirit-worker-pattern   ← Story 6.2 ships; FrameKind=21 precedent
5c4f348 6-1-ship-the-full-iac-bus-with-retract-primitive-and-drr-fairness-scheduler   ← Story 6.1 ships; Mailbox Phase 1/2 substrate
da3574d epic-5-retrospective                                                            ← §A1–§A8 actions; closed via Stories 6.1/6.2/6.3
23e5b7a feat: add smoke benchmark mode and reporting for measurement gate              ← Story 5.5e bench infrastructure
6a64a97 5-5d-spirit-registry-over-mcp-streamable-http-with-three-trust-tiers          ← 27 OPEN findings reference; Story 6.4 inherits the §A1 §A2 carry-forward debt classification at AC1
3d751b4 5-4-run-spirit-upgrades-and-propagate-signed-revocations-in-5s
6f76660 5-3-detect-spirit-crashes-hangs-and-silent-failures-with-halt-receipt-99-9     ← `cap_tokens::revoke_all(pid)` substrate (Story 6.4 AC3 RuptureReason::TokenRevoked)
5f34833 5-1-ship-full-lifecycle-verbs-and-11-triggers-with-priority-weighted-scheduling ← IdleWatchdog substrate (Story 6.4's ScheduleWatchdog twin) + HookDispatcher::fire_on_schedule
e14910d 4-5-author-the-cross-spirit-isolation-200-corpus-and-enforce-i14-halt-continuity-in-hot-swap   ← I13 lineage runtime substrate
f4d87f9 3-1-route-task-assign-frames-over-the-iac-bus-with-notification-surface-dispatch   ← IacFrame + Mailbox + deliver_typed substrate
```

**Substrate fingerprint at story open** (post Story 6.3):
- 24 workspace crates per `ls crates/` (Story 6.4 adds zero new crates)
- ~70+ discipline.yml jobs (Story 6.4 adds 4: `smoke-schedule-6-4`, `fr26-schedule-firing-corpus`, `adr-034-consent-rupture-corpus`, `nfr-scale-4-rate-limit-isolation`, `intent-lineage-6-4-extension` — 5 new total)
- `ABI_VERSION = 1` (frozen since Story 1b.4; Story 6.4 PRESERVES — explicit-discriminant additive variants only)
- `cargo-public-api` baseline additive-only across Epic 5 + Stories 6.1/6.2/6.3; Story 6.4 has 0 documented `Removed` and 2 `Changed` (`SpiritManifestBundle::schedules` field + `Provider::credential_fingerprint` default-method)
- §A3/§A5/§A6 xtask binaries SHIPPED at HEAD (`xtask/src/check_serde_error_handling.rs`, `check_review_findings_resolved.rs`, `check_dev_record_completeness.rs`)
- §A5/§A6 discipline.yml wiring gap (Epic 5 retro carry-forward; Story 6.3 documented as "gate-exists semantic honored" — Story 6.4 inherits the posture)
- 4/5 Epic 5 §A2 backfill placeholder (5-1, 5-2, 5-5a, 5-5b) carries forward
- Story 6.3 Review Findings: 22 patches + 3 decision-needed at v0.5 — Story 6.4 AC1 reports state; not blocking
- Workspace KLOC ceiling for `maos-kernel-core` continues to fail at HEAD per `xtask/kloc.toml` interim posture (`Story 6.4 ADDs ~600-1000 LOC to maos-kernel-core via ScheduleWatchdog + Phase 1.5 consent gate; over-ceiling carries forward`)

**Story 6.3 ships:**
- `crates/maos-a2a` filled in (placeholder → 2286+ LOC full crate)
- `A2ARouter` port trait in `maos-domain::ports::a2a`
- `ConsentEnvelope` additive extension (`intent_class`, `valid_until_ns`)
- `IacBusError::CrossHostNotConfigured` + `CrossHostRouteFailure(String)` replacing `CrossHostUnsupported`
- `Mailbox::with_a2a_router` builder + Phase-3 cross-host route
- Smoke arm `smoke-a2a-loopback-6-3` + 4 discipline jobs
- 22 review patches + 3 decisions OPEN at v0.5 (Story 6.3 review findings)

### Latest Technical Information

**Token bucket implementations**: At v0.5 the bucket is hand-rolled atomics — no `governor` crate, no `tower-rate-limit`, no `tokio-rate-limit` dep introduced (zero new workspace deps per FR47 posture). The lock-free design uses a packed `AtomicU64` carrying `(millitokens_remaining: u32, last_refill_ns_scaled: u32)` and a CAS loop on take/refill — pattern used in production-grade rate limiters (e.g., the standard "leaky bucket on AtomicU64" idiom). The 1000× scaling for milli-tokens supports fractional refill rates without floating-point in the CAS state. Test correctness with `tokio::time::pause` + `tokio::time::advance` per the Story 6.3 NFR-Sec-13 calibration-mode pattern — no wall-clock dependence.

**`tokio::time::interval` cadence drift**: The `interval` ticker can drift if a tick handler blocks. Story 6.4's ScheduleWatchdog uses `MissedTickBehavior::Skip` (the default; tick-coalescing) — per Tokio docs, this means "if multiple ticks elapse during a single iteration, only one tick fires when the iteration returns". For the 1s cadence + ≤100ms tick-handler budget, this is acceptable. If a handler exceeds 1s sustained, the watchdog falls behind silently — Story 6.4 documents this in dev notes; the operator surface can detect via the `iac_pending_frames_total{kind="capability.invocation"}` gauge (Story 1b.4 telemetry).

**SHA-256 fingerprinting for credentials**: `sha2::Sha256` is already in the workspace (Story 6.3 uses it for cert fingerprints). The 8-byte prefix gives 2^64 unique credential keys per Host — sufficient for the v0.5 operator-scale workload (≤ 100 distinct credentials per Host; collision probability < 10^-19 per birthday-paradox math). The FULL SHA-256 is computed and the first-8-bytes are extracted via `let hash = Sha256::digest(api_key.as_bytes()); let fp = u64::from_le_bytes(hash[..8].try_into().unwrap());`.

**Manifest section ordering**: The new `[[schedule]]` section MUST be parsed after `[lifecycle]` so the watchdog can consult `enabled_hooks` before firing. The existing parser at `manifest.rs` is single-pass over the TOML tree — the order is fixed by the section definitions, not by the TOML file ordering. Document the cross-section dependency in `[[schedule]]` field comments.

**ConsentRupture as a derived emission**: The rupture frame is NOT a NEW originating intent; it is a derived emission from the original frame's consent failure. Per architecture §7.3.2 the intent_lineage chain accumulates derived emissions. Story 6.4 PRESERVES the existing chain rather than starting a new one. This matches Story 4.5's halt-receipt pattern (the halt-receipt frame inherits lineage from the halted frame).

**Recursion bound for ConsentRupture on ConsentRupture**: The recursion is bounded structurally — the kernel address (Story 6.4 implementation detail: a reserved `SpiritId("kernel")` constant) is exempt from the per-recipient consent check. If a sender unloads between original-frame send and rupture-emission, the rupture's delivery to the sender ruptures with `RuptureReason::RecipientUnloaded` and the SECOND rupture is logged as a Critical telemetry event with `recursion_depth=2` AND the cycle breaks (no third-level rupture is emitted). Document the bound + test 3.8 verifies.

### Project Structure Notes

- ZERO new workspace crates. Story 6.4's additions live in EXISTING crates (`maos-kernel-core`, `maos-providers`, `maos-domain`, `maos-spirit-abi`, `maos-bin`, `maos-eval`, `xtask`)
- The `ScheduleWatchdog` lives at `crates/maos-kernel-core/src/scheduler/schedule_watchdog.rs` — under the `scheduler` module (the natural home alongside `IdleWatchdog`). This adds to `maos-kernel-core` LOC; per `xtask/kloc.toml` interim posture this carries the existing over-ceiling debt forward
- The `ProviderRateLimiter` lives at `crates/maos-providers/src/rate_limit.rs` — keeps the new code OUT of `maos-kernel-core` per the KLOC posture. The router consults the limiter via direct import (no port trait — same-process synchronous)
- The `[[schedule]]` manifest section schema lives in `crates/maos-kernel-core/src/security/manifest.rs`; the parsing path MUST use `#[serde(deny_unknown_fields)]` per Story 5.5d post-hoc lesson (the §A3 gate catches `.unwrap_or_default()` regressions but `deny_unknown_fields` prevents silent acceptance of typos in operator config)
- Per `xtask/kloc.toml` `[in_progress_decomposition]` Phase 1 (`maos-iac` + `maos-manifest` extraction) is Story 6.5 territory; Phase 4 (`maos-scheduler` extraction) is Story 7.x territory. Story 6.4 does NOT touch the extraction boundary — `ScheduleWatchdog` lives in `maos-kernel-core::scheduler` (existing module) and will move to `maos-scheduler` at Phase 4 with no design changes (the file structure is forward-compatible)
- The `FrameKind::ConsentRupture` and `FrameKind::RateLimited` additions are explicit-discriminant additive (22 and 23). The Story 6.2 precedent (`CliSubprocessOutput = 21`) confirms the project's interpretation that ADDING new explicit discriminants is wire-additive. The dev SHOULD consider adding `#[non_exhaustive]` to `FrameKind` in a follow-up Story (Story 6.5 or Epic 7) — Story 6.4 does NOT modify the existing enum attribute (out of scope; would be a broader ABI cleanup)

## References

- `_bmad-output/planning-artifacts/epics/epic-6-multi-spirit-coordination-full-iac-bus-a2a-peer-mesh-worker-patterns-v05-v15.md` — Epic 6 spec; Story 6.4 statement (lines 137-163)
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.4 (Provider rate-limit isolation) + §4.5 (Partial-consent failure semantics)
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md` §5.3 (on_schedule hook firing semantics)
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/7-inter-agent-communication.md` §7.1.1 (per-frame-kind channel class)
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md` ADR-022 (failure semantics floor), ADR-023 (cap-token TTL), ADR-025 (proactive scheduling), ADR-034 (partial-consent semantics)
- `_bmad-output/planning-artifacts/prd/functional-requirements.md` FR26
- `_bmad-output/planning-artifacts/prd/non-functional-requirements.md` NFR-Scale-4
- `_bmad-output/planning-artifacts/prd/domain-specific-requirements.md` (Provider rate-limit isolation paragraph)
- `_bmad-output/implementation-artifacts/6-3-build-the-a2a-peer-mesh-from-loopback-to-cross-host-with-mtls-rotation-chaos.md` — Story 6.3 substrate + Review Findings classified in AC1
- `_bmad-output/implementation-artifacts/6-2-dispatch-orchestrator-distillates-with-intent-lineage-and-cliwrapperspirit-worker-pattern.md` — Story 6.2 substrate (FrameKind = 21 precedent; intent-lineage corpus to extend)
- `_bmad-output/implementation-artifacts/6-1-ship-the-full-iac-bus-with-retract-primitive-and-drr-fairness-scheduler.md` — Story 6.1 substrate (channel_class table; Mailbox::deliver Phase 1/2)
- `_bmad-output/implementation-artifacts/5-1-ship-full-lifecycle-verbs-and-11-triggers-with-priority-weighted-scheduling.md` — Story 5.1 substrate (IdleWatchdog + HookDispatcher::fire_on_schedule)
- `_bmad-output/implementation-artifacts/epic-5-retro-2026-05-24.md` — §A1–§A8 + §A4 actions
- `crates/maos-spirit-abi/src/identity.rs:16-35` — `FrameKind` enum (Story 6.4 ADDS variants 22, 23)
- `crates/maos-spirit-abi/src/lifecycle.rs:52-57` — `SchedulePayload` wire shape (Story 2.1 substrate)
- `crates/maos-spirit-abi/src/lifecycle.rs:165` — `Spirit::on_schedule` trait method (Story 2.1 substrate)
- `crates/maos-domain/src/frame.rs` — `IacFrame` + `ConsentEnvelope` (Story 6.3 extended)
- `crates/maos-domain/src/invariants/i1.rs:60-103` — `Scope` enum (`#[non_exhaustive]`)
- `crates/maos-kernel-core/src/iac/mailbox.rs:133-255` — Phase 1/2/3 partition (Story 6.4 ADDS Phase 1.5 per-recipient consent gate)
- `crates/maos-kernel-core/src/iac/channels.rs` — `channel_class_for(kind)` const-table (Story 6.4 ADDS rows for 22, 23)
- `crates/maos-kernel-core/src/iac/mod.rs:280-..` — `IacBusAdapter::deliver_typed` (Story 6.4 frames flow through)
- `crates/maos-kernel-core/src/scheduler/idle_watchdog.rs` — IdleWatchdog (STRUCTURAL TWIN of Story 6.4's ScheduleWatchdog)
- `crates/maos-kernel-core/src/scheduler/hook_dispatch.rs:266-271` — `HookDispatcher::fire_on_schedule` (Story 5.1 dispatcher; Story 6.4 wires the cadence loop)
- `crates/maos-kernel-core/src/security/manifest.rs:1052-1184` — `[scheduling]` + `[lifecycle]` sections (Story 6.4 ADDS `[[schedule]]` array alongside)
- `crates/maos-kernel-core/src/inference/router.rs:14` — `InferenceRouter` (Story 6.4 ADDS rate-limit gate)
- `crates/maos-providers/src/lib.rs`, `provider.rs`, `anthropic.rs`, `openai.rs`, `ollama.rs` — providers crate (Story 6.4 ADDS `rate_limit.rs`)
- `crates/maos-capability/src/cap_tokens/mod.rs:272-294` — `revoke()` / `revoke_all()` (Story 6.4 AC3 RuptureReason::TokenRevoked + AC2 principal-revocability proxy)
- `xtask/src/check_epic_6_bridge.rs` — bridge gate (Story 6.4 EXTENDS with `--story 6.4`)
- `xtask/src/check_serde_error_handling.rs` + `check_review_findings_resolved.rs` + `check_dev_record_completeness.rs` — §A3 / §A5 / §A6 gates (MUST-PASS at HEAD)
- `xtask/kloc.toml` — `maos-kernel-core` ceiling = 6000 (currently over at ~21K; Story 6.4 carries forward the over-ceiling posture per `[in_progress_decomposition]` interim phase); `maos-providers` ceiling = 2000 (Story 6.4 lands ≤300 LOC of new rate-limit code)
- `.github/workflows/discipline.yml` — Story 6.4 ADDS 4 new jobs (smoke + 3 corpus) + 1 lineage-extension job; `aggregate.needs:` extended

## Completion Status

- [x] Story foundation extracted from epic-6 spec
- [x] Acceptance criteria authored with Given/When/Then per AC
- [x] Bridge preconditions explicitly enumerated (AC1)
- [x] FR26 / ADR-025 schedule manifest + watchdog scoped (AC2)
- [x] ADR-034 binding-v0.9 ConsentRupture scoped (AC3)
- [x] NFR-Scale-4 provider rate-limit isolation scoped (AC4)
- [x] Smoke arm + dev-record discipline per Story 6.1 / 6.2 / 6.3 carry-forward (AC5)
- [x] Source-file references cited at line precision
- [x] "What this story is NOT" boundary documented
- [x] File-change inventory enumerated per AC
- [x] Model recommendation documented (`claude-opus-4-7`) with substitution path
- [x] Architecture / ADR / Invariant compliance cross-referenced
- [x] Dev pass — AC1 through AC5
- [ ] Code review via `bmad-code-review` (3-agent parallel review; +Test Infrastructure Auditor if non-Claude/non-Codex)
- [ ] Discipline sweep — Story 6.4 jobs PASS at HEAD; pre-existing Epic 5/6 carry-forward debt documented in Completion Notes
- [ ] sprint-status `6-4-…` → `done` (currently `review`; user transitions post-review)

## Dev Agent Record

### Agent Model Used

claude-opus-4-7

### Debug Log References

* Local smoke arm dry-run: `MAOS_ONE_SHOT=smoke-schedule-6-4 MAOS_SCHEDULE_FAST=1 cargo run -p maos-bin --features fixture_replay` — output:
  ```
  smoke-schedule-6-4: starting wedge demo
  smoke-schedule-6-4: ✅ ConsentRupture surface — recipient B rejected, sender received typed rupture frame
  smoke-schedule-6-4: ✅ RateLimited surface — bucket exhausted; retry_after_ms=29999
  smoke-schedule-6-4: ✅ ScheduleWatchdog firing — `morning-digest` fired 1 time(s) under rate_limit_per_hour=1 cap
  smoke-schedule-6-4: TL state — 1 schedule.fire row(s), 1 ConsentRupture row(s)
  smoke-schedule-6-4: ✅ wedge demo complete; all four surfaces verified
  ```
* `gh run watch` link — pending PR submission post-review.

### Completion Notes List

**AC1 — Bridge precondition gate `check-epic-6-bridge --story 6.4` verbatim output (post-implementation):**

```
[PASS] A1 — Story 5.5d: 0 open Critical/High findings
[FAIL] A2 — Review Findings debt: 5-1: contains '_No review findings._' placeholder; 5-2: contains '_No review findings._' placeholder; 5-5a: contains '_No review findings._' placeholder; 5-5b: contains '_No review findings._' placeholder
[PASS] A3 — check-serde-error-handling.rs exists and wired in discipline.yml
[FAIL] A5 — discipline.yml missing check-review-findings-resolved job
[FAIL] A6 — discipline.yml missing check-dev-record-completeness job
[FAIL] A4-Debt-1 — i9-whitelist.toml (0 entries) + i9-exemptions.md present
[PASS] A4-Debt-2b — P4 mediated-io exemptions file exists (debt 2b closed via exemption)
[FAIL] A4-Debt-2c — spirit-abi-hook-count.toml exists but count != 15
[PASS] Umbrella — discipline.yml has check-epic-6-bridge job
[PASS] 6.4-A3-A5-A6 — verify: §A3 xtask=true job=true §A5 xtask=true job=false(carry-forward) §A6 xtask=true job=false(carry-forward)
[PASS] 6.4-AC7-SMOKE-ARM — verify: smoke-a2a-loopback-6-3 arm in main.rs present=true (does NOT block 6.4)
[PASS] 6.4-P4 — blocking_6_4: 6.3-P4 — every a2a-loopback-corpus-v0 test target resolves
[PASS] 6.4-6.3-RF — verify-only: Story 6.3 has 5 open Critical/High findings (does NOT block 6.4)
[PASS] 6.4-6.1-D-3 — carry-forward: DRR test_present=false job_present=false (does NOT block 6.4)
[PASS] 6.4-6.2-BENCH-NOTE — carry-forward: cli_wrapper_subprocess_fan_out.rs bench_present=false (does NOT block 6.4)
[PASS] 6.4-A2-BACKFILL — carry-forward: §A2 backfill — populated=1/5 placeholder=4/5 (does NOT block 6.4)
[PASS] 6.4-MAOS-PROVIDERS-BASELINE — blocking_6_4: maos-providers Provider/ProviderError exported=true rate_limit.rs=true module_declared=true → consistent=true
[PASS] 6.4-FRAMEKIND-BASELINE — blocking_6_4: FrameKind::ConsentRupture=22 present=true FrameKind::RateLimited=23 present=true → consistent=true
[PASS] 6.4-SCHEDULE-WATCHDOG-BASELINE — blocking_6_4: schedule_watchdog.rs present=true mod declared=true → consistent=true
check-epic-6-bridge[6.4]: PASS
```

Story 6.4 scope (`--story 6.4`) gate exits PASS — every `blocking_6_4` row (P4, PROVIDERS-BASELINE, FRAMEKIND-BASELINE, SCHEDULE-WATCHDOG-BASELINE) confirms substrate; verify-only rows report inherited debt from Epic 5/6 carry-forward; legacy 6.1 `[FAIL]` rows (A2/A5/A6/A4-Debt-*) are out of scope at `--story 6.4` scope (the §A1/§A2/§A5/§A6 carry-forward is Epic 5 retro debt, not blocking 6.4).

**AC2 — `[[schedule]]` manifest + ScheduleWatchdog:**
- 12 manifest section unit tests at `crates/maos-kernel-core/src/security/manifest.rs::tests` (well-formed, deny-unknown-field, cadence/rate-limit out-of-range, duplicate-id, empty-section default, compliance_claim_ref_hex length + sha256: prefix, side_effect_scopes Scope round-trip, payload_b64 round-trip, id regex, two-entries) — all 12 PASS.
- 8 ScheduleWatchdog integration tests at `crates/maos-kernel-core/tests/schedule_watchdog_fr26.rs` covering AC2.1 (single-entry firing), AC2.2 (independent cadence), AC2.3 (rate-limit cap), AC2.5 (lifecycle gate skip), AC2.7 (Paused state), AC2.8 (ComplianceClaim stamp in TL row), empty-section, cadence respected — all 8 PASS.
- ScheduleFireRecord payload type at `crates/maos-kernel-core/src/iac/payload.rs` (NEW module) — 2 round-trip unit tests PASS.
- Composition root spawns `ScheduleWatchdog` alongside `IdleWatchdog` with shared `CancellationToken`.

**AC3 — ConsentRupture (ADR-034 binding-v0.9):**
- `FrameKind::ConsentRupture = 22` + `FrameKind::RateLimited = 23` added to `crates/maos-spirit-abi/src/identity.rs` (additive variants; `from_u8` extended).
- Channel-class rows added at `crates/maos-kernel-core/src/iac/channels.rs` (both `Mpsc` cap 32) + 3 existing tests updated.
- `ConsentRupturePayload` + `RuptureRejection` + `RuptureReason` + `RateLimitedPayload` at `crates/maos-domain/src/frame.rs` (additive, `#[non_exhaustive]` on the reason enum).
- `Mailbox::register_spirit` extended to allocate per-kind mpsc channels for the two new kinds (8 receivers per Spirit, was 6).
- Phase 1.5 per-recipient consent gate inserted into `Mailbox::deliver` via pluggable `ConsentGate` trait; default = accept-all (existing flows unchanged). 11 integration tests at `crates/maos-kernel-core/tests/consent_rupture_adr_034.rs` covering AC3.1–3.10 + serde round-trip — all 11 PASS.
- Quarantine TL row written via `FrameKind::ConsentRupture` BEFORE rupture frame is emitted to sender (I2 log-before-deliver).
- Rupture frames inherit `intent_lineage` from the original frame (derived emission per architecture §7.3.2).
- Recursion bounded at depth-2 via the `KERNEL_SENDER_SPIRIT_ID = "__kernel"` reserved identity + `rupture_depth` parameter on `deliver_inner`.

**AC4 — Provider rate-limit isolation (NFR-Scale-4):**
- NEW `crates/maos-providers/src/rate_limit.rs` ships `TokenBucket` (lock-free CAS on packed AtomicU64 `(milli-tokens × 32 bits, last_refill_ns_scaled × 32 bits)`) + `BucketKey { provider_id, credential_fingerprint }` + `ProviderRateLimiter` + `ProviderRateLimitConfig::from_env()` reading `MAOS_ANTHROPIC_RPM`/`MAOS_OPENAI_RPM`/`MAOS_OLLAMA_RPM`.
- `Provider::credential_fingerprint(&self) -> u64` default-method (returns 0) + overrides on Anthropic/OpenAI/Ollama returning `first-8-bytes-of-SHA256(api_key)` (Ollama uses `endpoint_url`).
- `InferencePortAdapter` extended with `.with_rate_limiter(...)` + `.with_iac(...)` builders. On `Err(RetryAfter)` from bucket consume: returns `InferenceError::RateLimited { retry_after_ms }` SYNCHRONOUSLY AND emits typed `RateLimited` IAC frame fire-and-forget to the invoking Spirit (`spirit:<pid>`).
- 8 integration tests at `crates/maos-providers/tests/rate_limit_isolation_nfr_scale_4.rs` covering AC4.1–4.8 (same provider + credential shared; different credentials isolated; different providers isolated; refill window; concurrent CAS correctness 100-task race; retry-after computation; zero-refill marker; credential fingerprint stability) — all 8 PASS.
- 6 unit tests in `rate_limit.rs::tests` PASS.
- AC4.6 provider-side HTTP 429 handling DEFERRED to follow-up: the bucket-side gate already returns `InferenceError::RateLimited`; mapping `ProviderError::ProviderRejected { status: 429 }` into a duplicate `RateLimited` frame with `retry-after` header parsing remains an open enhancement (documented as Review-Findings carry-forward).

**AC5 — Smoke arm + discipline gates:**
- `MAOS_ONE_SHOT=smoke-schedule-6-4` arm in `crates/maos-bin/src/main.rs` exercises all four surfaces (schedule firing + per-schedule rate-limit cap + ConsentRupture + per-provider bucket exhaustion); end-to-end log captured under Debug Log References.
- 5 new discipline.yml jobs wired into `aggregate.needs`: `smoke-schedule-6-4`, `fr26-schedule-firing-corpus`, `adr-034-consent-rupture-corpus`, `nfr-scale-4-rate-limit-isolation`, `intent-lineage-6-4-extension`.
- `check-epic-6-bridge` job extended with `cargo run -p xtask -- check-epic-6-bridge --story 6.4 --json`.
- Lineage corpus extended from 70 to 100 scenarios (10× `lineage_via_consent_rupture` + 10× `lineage_via_rate_limited` + 10× `lineage_via_on_schedule`). 2 new tests at `crates/maos-eval/tests/intent_lineage_corpus_load.rs` confirm corpus loads + class counts.

**Discipline gate deltas (local sweep):**
- `cargo run -p xtask -- check-unsafe` — PASS (0 violations; every new file declares `#![forbid(unsafe_code)]`).
- `cargo run -p xtask -- check-serde-error-handling` — baseline 283 → after 282 violations across 71 files. **Story 6.4 introduces ZERO new `unwrap_or_default()` on serde paths** (the production `serde_json::to_vec(&record)` in `ScheduleWatchdog::check_and_fire` returns the error via `eprintln + continue` rather than `unwrap_or_default`); production-path delta = 0 violations. Test-mod test helpers in `payload.rs` add 2 `.expect()` violations that are universal noise (other test files at HEAD have similar patterns). Per spec posture, §A3 gate continues to FAIL on inherited 282 violations.
- `cargo run -p xtask -- check-empty-kernel` — baseline 76 → after 78 violations (delta +2; both are test-only structs `CountingSpirit` and `RejectingGate` declared by the new test fixtures, consistent with the `HookCounter`/`SnapshotSpirit`/`TestSpirit` test-pattern already in the baseline). Production `ScheduleBucket` is annotated `#[maos_attrs::i9_exempt(...)]` AND documented in `docs/invariants/i9-exemptions.md`. Production `ScheduleWatchdog` annotated + documented.
- `cargo run -p xtask -- check-service-boundary` — unchanged from baseline (Story 6.4 does NOT touch service boundaries; `ScheduleWatchdog` lives in existing `maos-kernel-core::scheduler` module; `ProviderRateLimiter` lives in existing `maos-providers`).
- `cargo run -p xtask -- check-dev-record-completeness` — baseline 41 → after 41 (this story's `dev_model_used: claude-opus-4-7` is set; Status = `in-progress` so it doesn't count yet; will be re-verified at `done` transition).
- `cargo run -p xtask -- check-fr47` — Story 6.4 introduces NO new FR47-denied dependencies. New `[dependencies]` in `maos-providers/Cargo.toml`: `dashmap = "6.1"`, `sha2 = "0.10"` (both already present in `maos-kernel-core` workspace pin; not protocol-layer deps).
- `cargo run -p xtask -- check-workspace-count` — unchanged (Story 6.4 adds zero new workspace crates).

**Test totals:** 38 new tests for Story 6.4 directly — 12 manifest unit + 8 ScheduleWatchdog integration + 11 ConsentRupture integration + 8 rate-limit integration + 6 rate-limit unit + 2 ScheduleFireRecord payload unit + 2 lineage-corpus integration tests + 30 corpus scenarios. All pass; the smoke arm passes end-to-end at `MAOS_SCHEDULE_FAST=1`.

**Carry-forward debt (per Story 6.1 / 6.2 / 6.3 precedent):**
- Pre-existing kernel-core lib-test failures (6 at HEAD): `i12_10_decision_frames_100_percent_carry_refs`, `approval_log_is_distinct_table`, `mcp::tests::*` — all panic on `monotonic_now_ns() called before init_monotonic_base()` (Story 5/6 carry-forward; orthogonal to Story 6.4).
- Pre-existing parallel-test env-var race (`security::operator_config::tests::env_allows_negating_bools` flakes when run alongside other tests under `cargo test --lib`; passes under `--test-threads=1` and in isolation). Not introduced by Story 6.4.
- Pre-existing `maos-bin` build under no-features fails on `maos_providers::fixture_replay::FixtureReplayProvider` and 4 similar imports — these compile under `--features fixture_replay` (the documented build configuration).
- §A5 / §A6 discipline.yml wiring gap (Epic 5 retro carry-forward) — Story 6.3 documented; Story 6.4 inherits the posture.
- KLOC `maos-kernel-core` ceiling continues to fail at HEAD per `xtask/kloc.toml` interim posture; Story 6.4 added ~450 LOC (ScheduleWatchdog + Phase 1.5 consent gate + payload module). Phase-4 extraction is Story 7.x territory.

### File List

#### AC1 — Bridge precondition gate (xtask)
- UPDATE `xtask/src/check_epic_6_bridge.rs` — added `--story 6.4` flag + 10 row classifications (`check_6_4_*` functions)
- UPDATE `xtask/src/main.rs` — extended doc comment for `--story 6.4` scope
- UPDATE `.github/workflows/discipline.yml` — added `Run check-epic-6-bridge --story 6.4` step

#### AC2 — `[[schedule]]` manifest section + ScheduleWatchdog
- UPDATE `crates/maos-kernel-core/src/security/manifest.rs` — added `ScheduleEntry`/`SchedulesSection`/`RawSchedulesSection`/`RawScheduleEntry` types; `decode_b64_strict` hand-rolled decoder (no new workspace dep); 12 unit tests
- UPDATE `crates/maos-kernel-core/src/scheduler/control_block.rs` — added `schedules: SchedulesSection` additive field on `SpiritManifestBundle`
- UPDATE `crates/maos-kernel-core/src/lifecycle/upgrade.rs` — extended `load_bundle_from_file` to parse `[[schedule]]` array-of-tables
- NEW `crates/maos-kernel-core/src/scheduler/schedule_watchdog.rs` — `ScheduleWatchdog` runtime mirroring `IdleWatchdog` (per-firing gate ordering: lifecycle → principal-revocability → rate-limit → ComplianceClaim → narrowed cap-token); `ScheduleBucket` per-schedule token bucket; `MAOS_SCHEDULE_FAST` parity with `MAOS_IDLE_FAST`
- NEW `crates/maos-kernel-core/src/iac/payload.rs` — `ScheduleFireRecord` typed payload for TL rows
- UPDATE `crates/maos-kernel-core/src/iac/mod.rs` — declared `payload` module
- UPDATE `crates/maos-kernel-core/src/scheduler/mod.rs` — exported `ScheduleWatchdog`
- NEW `crates/maos-kernel-core/tests/schedule_watchdog_fr26.rs` — 8 integration tests
- UPDATE `crates/maos-bin/src/main.rs` — composition root spawns `ScheduleWatchdog` with shared `CancellationToken`

#### AC3 — `FrameKind::ConsentRupture` + `Mailbox::deliver` Phase 1.5
- UPDATE `crates/maos-spirit-abi/src/identity.rs` — added `FrameKind::ConsentRupture = 22` + `FrameKind::RateLimited = 23` variants; extended `from_u8`
- UPDATE `crates/maos-domain/src/frame.rs` — added `ConsentRupturePayload` + `RuptureRejection` + `RuptureReason` (non-exhaustive) + `RateLimitedPayload`; extended `FramePayload` enum with the two new variants
- UPDATE `crates/maos-domain/src/log_recall.rs` — added `FrameKindLabel::ConsentRupture` + `RateLimited` variants
- UPDATE `crates/maos-kernel-core/src/iac/transparency_log.rs` — added `FrameKind::ConsentRupture = 22` + `RateLimited = 23`; extended `from_i64`
- UPDATE `crates/maos-kernel-core/src/iac/channels.rs` — added 2 new channel-class rows (Mpsc, cap 32); updated `channel_classes_match_addendum` + `all_iac_frame_kinds_are_routable` tests
- UPDATE `crates/maos-kernel-core/src/iac/log_recall.rs` — extended `to_domain_kind` match
- UPDATE `crates/maos-kernel-core/src/iac/mod.rs` — extended `deliver_typed`'s FrameKind→tl_kind mapping
- UPDATE `crates/maos-kernel-core/src/iac/mailbox.rs` — `ConsentGate` trait + `consent_gate: OnceLock<Arc<dyn ConsentGate>>` + `transparency_log: OnceLock<...>` fields; `with_consent_gate` + `with_transparency_log` builders; `install_consent_gate`/`install_transparency_log` post-construction installers; Phase 1.5 per-recipient consent gate in `deliver_inner` with recursion-depth tracking; `KERNEL_SENDER_SPIRIT_ID` constant; `random_16_bytes` helper for rupture/frame_id; refactored `scbs` mutex usage to lexical scopes so the boxed `deliver_inner` future is `Send`-safe
- NEW `crates/maos-kernel-core/tests/consent_rupture_adr_034.rs` — 11 integration tests
- UPDATE `crates/maos-bin/src/main.rs` — `mailbox.install_transparency_log(...)` wiring

#### AC4 — `ProviderRateLimiter` substrate
- NEW `crates/maos-providers/src/rate_limit.rs` — `TokenBucket` (lock-free CAS on packed AtomicU64) + `BucketKey` + `ProviderRateLimiter` + `ProviderRateLimitConfig::from_env` reading `MAOS_*_RPM`; `fingerprint_credential` helper using sha2::Sha256
- UPDATE `crates/maos-providers/Cargo.toml` — added `dashmap = "6.1"` + `sha2 = "0.10"` dependencies; `[dev-dependencies] tokio = ...`
- UPDATE `crates/maos-providers/src/lib.rs` — declared `pub mod rate_limit`; re-exported the new types
- UPDATE `crates/maos-providers/src/provider.rs` — added `Provider::credential_fingerprint(&self) -> u64` default-method
- UPDATE `crates/maos-providers/src/anthropic.rs` — override `credential_fingerprint` returning `fingerprint_credential(&self.api_key)`
- UPDATE `crates/maos-providers/src/openai.rs` — same override pattern
- UPDATE `crates/maos-providers/src/ollama.rs` — override using `endpoint_url` (Ollama has no api_key)
- UPDATE `crates/maos-kernel-core/src/inference/mod.rs` — `InferencePortAdapter` extended with `rate_limiter: Option<Arc<ProviderRateLimiter>>` + `iac: Option<Arc<IacBusAdapter>>` fields; `.with_rate_limiter(...)` + `.with_iac(...)` builders; `bucket_key` + `emit_rate_limited_frame` helpers; `complete()` consults the rate-limiter BEFORE dispatching to the provider
- UPDATE `crates/maos-domain/src/ports/inference.rs` — added `InferenceError::RateLimited { retry_after_ms }` variant
- NEW `crates/maos-providers/tests/rate_limit_isolation_nfr_scale_4.rs` — 8 integration tests
- UPDATE `crates/maos-bin/src/main.rs` — composition root wires `ProviderRateLimiter::from_env()` into `InferencePortAdapter` via the new builders

#### AC5 — Smoke arm + corpus + dev-record discipline
- UPDATE `crates/maos-bin/src/main.rs` — added `smoke-schedule-6-4` arm + body + known-modes table entry
- UPDATE `.github/workflows/discipline.yml` — 5 new jobs (`smoke-schedule-6-4`, `fr26-schedule-firing-corpus`, `adr-034-consent-rupture-corpus`, `nfr-scale-4-rate-limit-isolation`, `intent-lineage-6-4-extension`); `aggregate.needs` extended
- NEW `crates/maos-eval/fixtures/intent-lineage-corpus-v0/scenario-071..080.json` — 10× `lineage_via_consent_rupture`
- NEW `crates/maos-eval/fixtures/intent-lineage-corpus-v0/scenario-081..090.json` — 10× `lineage_via_rate_limited`
- NEW `crates/maos-eval/fixtures/intent-lineage-corpus-v0/scenario-091..100.json` — 10× `lineage_via_on_schedule`
- UPDATE `crates/maos-eval/tests/intent_lineage_corpus_load.rs` — added 2 new tests asserting the corpus extends to 100 scenarios across 3 new classes
- UPDATE `docs/invariants/i9-exemptions.md` — documented `ScheduleWatchdog` and `ScheduleBucket` exemptions
- UPDATE `_bmad-output/implementation-artifacts/sprint-status.yaml` — `6-4-…` transitioned `ready-for-dev` → `in-progress` (will → `review` at Step 9)
- UPDATE `_bmad-output/implementation-artifacts/6-4-wire-scheduled-invocations-with-consentrupture-and-provider-rate-limit-isolation.md` — Status: `ready-for-dev` → `in-progress` → `review`; `dev_model_used: claude-opus-4-7`; Dev Agent Record populated

### Review Findings

**Code review executed:** 2026-05-27
**Review patches applied:** 2026-05-27 — all 35 patch findings fixed. via `bmad-code-review` skill (4 parallel layers: Blind Hunter + Edge Case Hunter + Acceptance Auditor + Test Infrastructure Auditor; dev_model_used = `claude-opus-4-7`).

**Triage summary:** 0 `decision-needed`, 35 `patch` (ALL APPLIED), 0 `defer`, 1 dismissed.

---

#### Critical / High

- [x] [Review][Patch] **Quarantine TL row introduces new `.unwrap_or_default()` on serde path** — violates Story 6.4 discipline floor ("ZERO new unwrap_or_default on serde paths"). `mailbox.rs:2567`: `serde_json::to_value(&r.reason).unwrap_or_default()` silently drops `RuptureReason` on serialization failure, producing `null` in the transparency log. Also breaks §A3 gate. **Source:** blind+edge+auditor.

- [x] [Review][Patch] **`RateLimited` frame discards invocation `intent_lineage` and uses zeroed metadata** — `inference/mod.rs:2941-2965`: `frame_id: [0u8; 16]`, `timestamp_ns: 0`, `intent_lineage: IntentLineage::default()`. Breaks I13 unbroken-chain invariant and makes TL correlation impossible. Per spec, frame must inherit the invocation's lineage. **Source:** blind+edge+auditor.

- [x] [Review][Patch] **`is_principal_revoked()` is a hardcoded `false` stub** — `schedule_watchdog.rs:288-305`: unconditionally returns `false` even when capability registry is present. AC2.4 test (revoked principal halts firing) is non-functional. The per-firing gate 2 is dead code. **Source:** blind+edge+auditor.

- [x] [Review][Patch] **TokenBucket u32 overflow corrupts large-capacity buckets** — `rate_limit.rs`: Ollama default RPM `999_999_999` gives `cap_milli = 999_999_999_000 > u32::MAX`. The cast to `u32` silently truncates to ~2.3M, corrupting bucket state. **Source:** edge.

- [x] [Review][Patch] **ScheduleWatchdog consumes rate-limit token then skips firing on `serde_json` error** — `schedule_watchdog.rs:213,231-241`: `bucket.try_consume()` succeeds, then `serde_json::to_vec(&record)` fails → `continue` without updating `last_fire_ns`. Schedule stuck: token depleted, hook never fired, next poll retries with empty bucket. **Source:** edge.

- [x] [Review][Patch] **Unknown provider IDs silently bypass rate-limiting** — `inference/mod.rs:2905-2909`: `bucket_key()` hardcodes `"anthropic" | "openai" | "ollama"` and returns `None` for all others. When `None`, the router skips rate-limiting entirely and forwards to `Provider::complete` ungated. **Source:** blind+edge+auditor.

- [x] [Review][Patch] **Default `Provider::credential_fingerprint` returns `0`** — `provider.rs`: default trait impl returns `0`. Any new provider driver forgetting to override shares fingerprint `0` with every other such driver, violating NFR-Scale-4 cross-credential isolation. **Source:** edge.

- [x] [Review][Patch] **TokenBucket `now_scaled` wraps every ~3.2 days, granting free refill** — `rate_limit.rs:4880-4884`: 32-bit counter wraps every `2^48` ns. On first consume after wrap, `wrapping_sub(last_scaled)` computes huge elapsed time, refilling bucket to full capacity regardless of actual elapsed time. **Source:** edge.

- [x] [Review][Patch] **Token issuance failure proceeds with `TokenId::ZERO` but hook still fires** — `schedule_watchdog.rs:219,335-340`: `issue_narrowed_token` fails → falls back to `TokenId::ZERO`, then `fire_one` executes anyway. Creates unmediated capability invocation path. **Source:** edge.

- [x] [Review][Patch] **AC1 xtask gates not executed, only file-existence checked** — `check_epic_6_bridge.rs:5553`: `check_6_4_a3_a5_a6_shipped()` checks paths but never invokes the gates. A regression in `check-serde-error-handling` would go undetected at AC1. Spec requires: "Assert each xtask file exists AND run each gate sequentially." **Source:** auditor.

- [x] [Review][Patch] **AC2.4 test missing: principal-revocability never functionally verified** — `schedule_watchdog_fr26.rs`: Test 2.4 (revoked principal halts firing) is absent from the 8-scenario file. The spec mandates it; the stub `is_principal_revoked()` makes it impossible to pass. **Source:** auditor+test.

- [x] [Review][Patch] **AC2.6 test missing: narrowed cap-token scope assertion absent** — `schedule_watchdog_fr26.rs`: Test 2.6 (per-firing cap-token issued with exactly the manifest's `side_effect_scopes`) is absent. **Source:** auditor+test.

- [x] [Review][Patch] **AC4 provider-side HTTP 429 handling completely missing** — `inference/mod.rs`: Spec requires mapping `ProviderError::ProviderRejected { status: 429 }` into `RateLimited` frame with `retry-after` header parsing. Dev documented as deferred (Task 5.6 [ ]), but AC4.6/4.7/4.8 still mandate it. **Source:** auditor.

- [x] [Review][Patch] **AC4 integration tests 4.6–4.8 do not match spec scenarios** — `rate_limit_isolation_nfr_scale_4.rs`: Tests substitute bucket-side "retry-after computation", "zero-refill marker", and "credential fingerprint stability" — none exercise the provider-side 429→frame path or the IAC round-trip required by AC4.6–4.8. **Source:** auditor+test.

- [x] [Review][Patch] **ConsentRupture recursion bound is off-by-one from spec** — `mailbox.rs:2536,2612`: Code disables consent gate at `rupture_depth >= 1`, so a `ConsentRupture` frame can never itself rupture. Spec says cycle breaks at depth-2. Test 3.8 is unreachable. **Source:** edge+test.

- [x] [Review][Patch] **`emit_rate_limited_frame` spawns fire-and-forget task without await/error handling** — `inference/mod.rs:2967-2970`: `tokio::spawn(async move { let _ = iac.deliver_typed(frame).await; })`. If IAC bus is backpressured or runtime shutting down, frame is silently dropped. Spec says both error and frame are "guaranteed to fire". **Source:** blind.

---

#### Medium / Low

- [x] [Review][Patch] **`last_fire_count()` returns timestamp, not a count** — `schedule_watchdog.rs:127-134`: Method name implies count but returns `last_fire_ns` nanosecond timestamp (~1.7e18 for fired-once). Misleading API. **Source:** blind+edge.

- [x] [Review][Patch] **`ProviderRateLimitConfig::from_env` accepts `rpm=0` without validation** — `rate_limit.rs:5247-5251`: Setting `MAOS_ANTHROPIC_RPM=0` creates capacity-0 bucket with refill rate 0.0, permanently bricking the provider. `retry_after_ms = u64::MAX`. **Source:** edge.

- [x] [Review][Patch] **`SpiritManifestBundle::schedules` lacks required `#[serde(default)]`** — `control_block.rs:206`: Spec requires `#[serde(default)]` for backward-compatible deserialization when `schedules` key is absent. Breaking change for manifests without `[[schedule]]`. **Source:** auditor.

- [x] [Review][Patch] **`rupture_id` is not a ULID as specified** — `mailbox.rs:2393-2400`: Spec defines `rupture_id: [u8; 16]` with comment "ULID for correlation". Implementation uses `monotonic_now_ns() + PID`, which is not a ULID and lacks lexicographic-sortable/timestamp properties. **Source:** auditor.

- [x] [Review][Patch] **Process-global env var mutation in tests causes race conditions** — `schedule_watchdog_fr26.rs:4408`, `rate_limit.rs:5410`, `main.rs:1506`: `std::env::set_var` is not thread-safe. Parallel test execution races on globals, causing flaky results. **Source:** blind+edge+test.

- [x] [Review][Patch] **Smoke arm uses wall-clock `sleep` instead of paused time** — `main.rs:1510`: `tokio::time::sleep(Duration::from_millis(300))` without `tokio::time::pause()`. On loaded CI runners, 300ms may be insufficient, causing intermittent smoke arm failures. **Source:** edge+test.

- [x] [Review][Patch] **Loose `retry_after_ms` assertion masks computation regressions** — `rate_limit_isolation_nfr_scale_4.rs:5320-5324`: Asserts `<= 2000ms` for expected ~1000ms. 2x tolerance band could hide significant refill-rate computation errors. **Source:** test.

- [x] [Review][Patch] **No quarantine Transparency Log verification in tests** — `consent_rupture_adr_034.rs`: None of the 11 tests verify the I2 log-before-deliver invariant that quarantined frames are written to the TL BEFORE the `ConsentRupture` frame is dispatched. `tl` adapter constructed but never asserted. **Source:** test.

- [x] [Review][Patch] **`CountingSpirit` cannot verify per-schedule behavior** — `schedule_watchdog_fr26.rs:4342-4353`: `on_schedule` ignores both `schedule_id` and `payload`. Cannot verify AC2.1 payload round-trip or AC2.2 independent cadence firing. **Source:** test.

- [x] [Review][Patch] **`run_watchdog_for` silently ignores shutdown timeout** — `schedule_watchdog_fr26.rs:4414`: `let _ = tokio::time::timeout(..., handle).await;` drops timeout result. Hanging watchdog masked. **Source:** test.

- [x] [Review][Patch] **Cross-host test lacks A2A short-circuit verification** — `consent_rupture_adr_034.rs:4250-4282`: AC3.10 claims to prove cross-host rejection short-circuits Phase 3, but no spy/mock on A2A router. Cannot detect if mailbox incorrectly attempted Phase 3 routing anyway. **Source:** test.

- [x] [Review][Patch] **Relaxed ordering used in exact-count concurrent assertion** — `rate_limit_isolation_nfr_scale_4.rs:5288-5309`: `AtomicUsize` with `Ordering::Relaxed` for success/fail counters, then asserts exact `50/50` split. Undermines visibility guarantee the test claims to verify. **Source:** test.

- [x] [Review][Patch] **TokenBucket tested in isolation; limiter integration uncovered** — `rate_limit_isolation_nfr_scale_4.rs:5261-5273`: AC4.4 calls `bucket.force_refill_for_test` directly on `TokenBucket`. `ProviderRateLimiter` wiring (DashMap lookup, lazy bucket creation, per-key refill scheduling) is never tested for refill behavior. **Source:** test.

- [x] [Review][Patch] **Fixture bloat without structural variation** — `scenario-071..100.json`: All 30 new fixtures are structurally identical (`hop_count: 1`, `accepted: true`, single intent, no negative cases). Add volume but exercise no edge cases (missing lineage, multi-hop, corrupted chain, rejected frames). **Source:** test.

- [x] [Review][Patch] **Hand-rolled base64 decoder accepts malformed padding** — `manifest.rs:3680-3688`: Input like `"A=BC"` (malformed mid-string padding) is accepted. Middle `=` treated as value `0`, but `pad_count` is `0` because only trailing `=` are counted. Produces incorrect bytes instead of rejecting. **Source:** edge.

- [x] [Review][Patch] **Predictable `random_16_bytes()` collision risk** — `mailbox.rs:2393-2400`: Uses `monotonic_now_ns() + PID`. Same PID + rapid consecutive calls = collisions. Functions named `random_*` should actually be random, or documented why predictability is acceptable. **Source:** blind.

- [x] [Review][Patch] **`ProviderRateLimitConfig` side effects in `Default` impl** — `rate_limit.rs:5041-5045`: `Default::default()` reads environment variables. Surprising and non-deterministic in tests. Also `read_rpm` silently ignores parse errors (`s.parse::<u32>().ok()`), masking misconfiguration. **Source:** blind.

- [x] [Review][Patch] **Sprint status YAML has invalid timestamp format** — `sprint-status.yaml:1`: `last_updated: '2026-05-27T00:00:00Z+story-6-4-review'` is invalid ISO 8601. Any parser expecting a real datetime will fail. **Source:** blind.

- [x] [Review][Dismiss] **AC5 Review Findings placeholder** — The placeholder `_No review findings._` at line 1104 is expected at pre-review stage. This review populates the section. Not a defect. **Dismissed.** **Source:** auditor.
