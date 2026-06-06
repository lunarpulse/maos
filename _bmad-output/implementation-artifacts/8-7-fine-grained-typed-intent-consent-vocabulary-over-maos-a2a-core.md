---
dev_model_used: claude-opus-4-8
---
# Story 8.7: Fine-Grained Typed-Intent Consent Vocabulary over `maos-a2a-core`

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

> **✅ PRECONDITION RESOLVED — Story 8.6 is DONE (2026-06-05).** This story lands its consent-enforcement
> change inside **`crates/maos-a2a-core`**, the protocol-seam crate **Story 8.6 created by extraction**
> (`sprint-status.yaml:89` `8-6-…: done`; workspace `39→41`; `maos-kernel-core` byte-identical at 15505).
> The enforcement helpers, consent types, and router substrate have **already moved** out of the
> over-budget `maos-a2a` into `maos-a2a-core` — every `file:line` in this story is grounded against the
> **post-8.6 tree** (verified 2026-06-05). The "build 8.6 first" sequencing fork from the original draft
> is therefore **closed**; placement is unambiguous. Two NEW post-extraction facts reshape the change and
> are baked into the ACs below: (1) the band-projection helper `frame_intent_str` is now **`pub`**
> (`maos-a2a-core/src/router.rs:212`) so it must **not** be renamed — add a helper instead; (2)
> `A2AConsentEnvelope` + its `From<ConsentEnvelope>` conversion are **dead on the enforcement path**
> (zero non-test callers — see Dev Notes).
>
> **✅ ALL DESIGN FORKS RESOLVED BY TEAM CONSENSUS (Winston + Murat + Security, 2026-06-05).** What were
> Open Questions Q2–Q5 in the draft are now **binding decisions** recorded in §"Team Consensus" at the end
> and woven into the ACs. Headlines: **Q2** — ship fine-grained-when-present NOW as the *transitional*
> mechanism, make `intent_class` population **hard-mandatory** for every reference cross-Host sender, and
> **commit fail-closed-for-cross-Host as a scheduled follow-up (Story 8.8)** gated on a new
> sender-completeness gate; **Q3** — additive canonical helper + `tracing::warn!` on unreachable entries
> (manifest-registry deferred per ADR-012's revisit trigger); **Q4** — register 8.7 formally via
> `bmad-correct-course` before `dev-story`; **Q5** — **DELETE** the dead `A2AConsentEnvelope` + `From`
> discard (it silently elevates a missing intent to the `"standard"` band — a latent fail-open), accepting
> a justified abi-diff **Removed** (the ONE ratified exception to AC8's Added-only).

## Story

As a **MAOS operator wiring cross-Host A2A consent (and the Spirit authors — Mira, Nash — who depend on it)**,
I want **the ADR-012 consent gate to enforce the actual fine-grained per-frame intent string an operator declares in a `ConsentAllowlists` (e.g. `diagnosis-handoff:read-only-evidence`, `rca-summary`), instead of silently collapsing every frame to one of the three coarse `IntentClass` bands `{highprivilege, standard, readonly}`**,
so that **ADR-012 is the "typed-*intent* consent" it was decided to be — the confused-deputy gap (Mira admissibly handing Nash `diagnosis-handoff:read-only-evidence` while `code-mutation-directive` is rejected) is closed for real, and an allowlist entry an operator writes can no longer fail-open by silently never matching.**

## Context & Problem Statement

**This story exists because of a gap Story 8.5 surfaced and Winston deferred** (epic-8 §AC-A6 "Noted gap", 2026-06-04, `epic-8-…miranash-v03-v15.md:274-275`; recorded again as the `8-7-…` comment in `sprint-status.yaml:90` and in `deferred-work.md:226-227`).

ADR-012 (`docs/adr/ADR-012-typed-intent-a2a-consent.md:10-18`) **decides**: "Cross-Host A2A consent is `(peer-identity, intent-class)`, not `(peer-identity)`… Mira's `diagnosis-handoff:read-only-evidence` is admissible at Nash; `code-mutation-directive` is rejected." The architecture restates it (`7-inter-agent-communication.md:52`): "The kernel rejects frames whose typed intent is not in the sender's send-allowlist or the receiver's accept-allowlist with `EIntentDenied`. This is what makes Mira's `diagnosis-handoff:read-only-evidence` admissible at Nash while `code-mutation-directive` is rejected." ADR-012 explicitly chooses an **open vocabulary** ("what would force a revisit: intent-class cardinality grows pathologically" — i.e. the open string space IS the design).

**But the implementation collapses every frame to 3 coarse bands.** Post-8.6 the enforcement path lives in `maos-a2a-core/src/router.rs`:

```rust
// crates/maos-a2a-core/src/router.rs:212-232  (moved here from maos-a2a/src/adapter.rs by Story 8.6)
pub fn frame_intent_str(frame: &IacFrame) -> String {
    frame.intent.a2a_consent_intent_str().to_string()   // ← THE BUG: 3-band projection only
}
fn send_admits(allow: &ConsentAllowlists, frame: &IacFrame) -> bool {
    let s = Self::frame_intent_str(frame);
    allow.send_allowlist.iter().any(|i| i.as_str().eq_ignore_ascii_case(&s))
}
fn accept_admits(allow: &ConsentAllowlists, frame: &IacFrame) -> bool {
    let s = Self::frame_intent_str(frame);
    allow.accept_allowlist.iter().any(|i| i.as_str().eq_ignore_ascii_case(&s))
}
```

`IntentClass::a2a_consent_intent_str()` (`crates/maos-domain/src/invariants/i1.rs:142-148`) projects the 3-variant enum to exactly `{"highprivilege","standard","readonly"}`. So:

- `ConsentAllowlists.send_allowlist` / `accept_allowlist` are `Vec<A2AIntent>` (free-form open-vocabulary `String` newtypes — `crates/maos-a2a-core/src/consent.rs:50-67`, `A2AIntent` at `crates/maos-domain/src/invariants/i8.rs:32-44`).
- An operator writes `A2AIntent::new("diagnosis-handoff:read-only-evidence")` into an allowlist (as `smoke-a2a-loopback-6-3` **aspirationally does** — `crates/maos-bin/src/main.rs:4826,4829,4839,4840` — and as `cross_host_consent_v1.rs:75` asserts).
- The frame projects to `"readonly"` / `"standard"`. The specific string **silently never matches** → the entry is dead. The gate is effectively "typed-*class* consent," NOT "typed-intent consent."

**The lever that makes the fix ABI-neutral:** the per-frame fine-grained intent **already rides the wire**. `IacFrame.consent_envelope` (`crates/maos-domain/src/frame.rs:36`) is `Option<ConsentEnvelope>`, and `ConsentEnvelope` (`frame.rs:408-421`) carries:

```rust
/// Story 6.3 / ADR-012 binding-v0.9 — typed-intent for cross-Host consent.
/// Filled by the sender's A2A outbound path; verified by the receiver's
/// A2A intake. Same-Host frames use `None`.
#[serde(default)]
pub intent_class: Option<crate::invariants::i8::A2AIntent>,   // ← the real per-frame fine-grained intent
```

The field exists, is serde-additive (`#[serde(default)]`), is documented as the intent the sender fills and the receiver verifies — and **is never consulted by the enforcement path, nor ever populated by any sender** (verified 2026-06-05: the only non-test write of `consent_envelope` anywhere is the `None` default at `frame.rs:586`; grep finds no production site that sets it to `Some(..)`).

**So this story is a refinement, not new substrate:** make enforcement read the per-frame `IacFrame.consent_envelope.intent_class` (the real `A2AIntent`) when present, and make senders populate it. No new `IacFrame` field, no `IntentClass` enum widening, no JSON-RPC field added (8.6 §AC-A6 — "consent rides the EXISTING JSON-RPC field" — is satisfied unchanged).

## Acceptance Criteria

> **AC numbering**: AC1–AC4 are the functional core (AC2 = mandatory sender population, AC2b = delete the dead fail-open type per Q5); AC5 vocabulary hygiene; AC6 reference-Spirit + smoke; AC7 backward-compat & no-regression; AC8 placement/ABI/discipline gates; AC9 commits the fail-closed end-state as scheduled Story 8.8 (Q2 consensus). Every AC is BDD-shaped and independently verifiable.

### AC1 — Enforcement matches the per-frame fine-grained intent, with documented band fallback

**Given** a cross-Host frame whose `consent_envelope.intent_class` is `Some(A2AIntent("diagnosis-handoff:read-only-evidence"))`
**When** the router runs send-allowlist (outbound, via `prepare_outbound`) and accept-allowlist (inbound, `handle_intake`) enforcement
**Then** the consent-match key is the frame's **fine-grained `A2AIntent`** (`consent_envelope.intent_class`), matched case-insensitively against `send_allowlist` / `accept_allowlist`; an allowlist containing `A2AIntent::new("diagnosis-handoff:read-only-evidence")` **admits** the frame, and one that does not (e.g. holds only `"rca-summary"`) **denies** it with `EIntentDenied` whose `intent` field carries the **literal fine-grained string** (`"diagnosis-handoff:read-only-evidence"`), NOT a band token
**And** when `consent_envelope` is `None` OR `consent_envelope.intent_class` is `None` (same-Host frames, or cross-Host frames that declare no fine-grained intent), enforcement **falls back** to the existing 3-band `IntentClass` projection (`frame_intent_str` → `frame.intent.a2a_consent_intent_str()`) — so today's `"readonly"`/`"standard"`/`"highprivilege"` allowlists keep working byte-for-byte (interlocks AC7)
**And** the single decision point is **one NEW private helper** (`consent_match_key(frame) -> String`) used by BOTH `send_admits` and `accept_admits` AND by the two `EIntentDenied.intent` construction sites (`router.rs:259`, `router.rs:427`), so send and accept can never diverge on which key they match or report
**And** `frame_intent_str` is **NOT renamed or removed** — it stays `pub` as the band-projection primitive that `consent_match_key` calls for its fallback (it became part of `maos-a2a-core`'s public surface during 8.6; renaming it is an abi-diff Removed → AC8 RED)
**And** the precedence rule (fine-grained-when-present, band-otherwise) is documented in a doc-comment on `consent_match_key`, citing ADR-012

### AC2 — Reference cross-Host senders populate the fine-grained intent end-to-end (HARD-MANDATORY — Q2 consensus)

**Given** the consensus that fine-grained-when-present is a *transitional* mechanism toward committed fail-closed (Story 8.8), so no reference cross-Host sender may ride the band-fallback path
**When** any reference cross-Host sender (Mira, Nash, and the `smoke-a2a-loopback-6-3` / `smoke-mira-nash-8-5` arms) constructs and routes an outbound frame via `prepare_outbound` / the loopback or TCP transport
**Then** that frame's `IacFrame.consent_envelope` carries `intent_class = Some(declared_intent)` end-to-end (sender → wire → receiver intake) with NO collapse to a band — **this is a hard requirement, not aspirational**: a reference cross-Host frame leaving with `intent_class == None` is a defect, asserted against in each smoke arm (Security/Murat round-2 condition: "no off-Host frame leaves with `intent_class == None`"); a sender-side helper is added if the Spirit/driver API needs one to attach a `ConsentEnvelope`
**And** a test asserts a round-tripped frame's `consent_envelope.intent_class` is **byte-equal** to what the sender set (no normalization beyond documented case-folding at match time), proving population is real
**And** the band-fallback in AC1 remains the mechanism for *legacy/unmigrated* band-only frames (so AC7 zero-regression holds for them), but it is explicitly the path Story 8.8's gate will prove empty for cross-Host senders before flipping to fail-closed

### AC2b — Delete the dead `A2AConsentEnvelope` fail-open footgun (Q5 consensus = DELETE)

**Given** `A2AConsentEnvelope` + its `From<ConsentEnvelope>` are **dead on the enforcement path** (the router reads `frame.consent_envelope` directly; zero non-test callers — verified 2026-06-05), and the `From` does `intent_class: env.intent_class.unwrap_or_else(|| A2AIntent::new("standard"))` (`consent.rs:37`) — a silent coercion of a *missing* intent to the **`"standard"` privilege band** (not `readonly`), i.e. a latent fail-open *privilege elevation* the instant anyone wires this type onto the path
**When** 8.7 lands
**Then** the unused `A2AConsentEnvelope` struct + its `From<ConsentEnvelope>` impl are **deleted** (along with the re-exports at `maos-a2a-core/src/lib.rs:48` and `maos-a2a/src/lib.rs:52`), removing the loaded gun entirely rather than doc-commenting it (the team rejected "leave + doc-comment" as a decaying-promise / loaded-gun pattern)
**And** evidence records the zero-non-test-caller grep justifying the change, and the resulting `abi-diff` **Removed** on `A2AConsentEnvelope` is the **one ratified exception** to AC8's Added-only discipline (flagged for Winston's sign-off, mirroring the 8.6 fail-closed flag — Winston, the freeze-owner, voted for this override)
**And** [fallback, only if a Removed is categorically refused at review] convert `A2AConsentEnvelope.intent_class` to `Option<A2AIntent>` (abi-diff Modified) so callers must handle `None` explicitly rather than silently elevating — but **never** leave the `"standard"` discard in place

### AC3 — Confused-deputy negative is closed at the fine-grained granularity (ADR-012's worked example)

**Given** Mira (Host A) and Nash (Host B) with Nash's `accept_allowlist = [A2AIntent("diagnosis-handoff:read-only-evidence")]` (and NOT `"code-mutation-directive"`)
**When** Mira emits a frame carrying `intent_class = Some("diagnosis-handoff:read-only-evidence")` — **admitted**; AND a second frame carrying `intent_class = Some("code-mutation-directive")` — **denied**
**Then** the admitted frame reaches Nash's intake and the denied frame is rejected with `EIntentDenied`/`CODE_INTENT_DENIED` (`-32001`, `maos-a2a-core/src/transport/json_rpc.rs:30`) whose payload names `"code-mutation-directive"`, observable in the Transparency Log
**And** this is the literal ADR-012 rationale ("Mira's `diagnosis-handoff:read-only-evidence` is admissible at Nash; `code-mutation-directive` is rejected") now passing as an executable test — NOT realized as the `readonly` band (which would admit BOTH, reopening the confused-deputy gap)

### AC4 — Defense-in-depth holds at the fine-grained granularity (send-side AND accept-side)

**Given** the same fine-grained intent
**When** a frame is denied
**Then** BOTH directions enforce independently on the fine-grained key: a send-side denial yields `A2AError::IntentDenied { direction: IntentDirection::Send, .. }` before the frame hits the wire (`prepare_outbound` step (1), `router.rs:253-263` — sender refuses to emit an intent its OWN `send_allowlist` forbids), and an accept-side denial yields the `CODE_INTENT_DENIED` NACK (`handle_intake`, `router.rs:420-431`) → `A2AError::IntentDeniedAtPeer` on the sender; the existing `EIntentDenied { peer, intent, direction }` + `AllowlistDirection::{Send,Accept}` + `A2AError::{IntentDenied,IntentDeniedAtPeer}` types are preserved, now carrying fine-grained `intent` strings

### AC5 — Vocabulary hygiene: silent-never-match becomes loud (the root complaint)

**Given** the original failure mode was "an operator writes an intent that *silently* never matches"
**When** the consent layer is exercised
**Then** an `A2AIntent` is given a **canonical-form check** — an additive `A2AIntent::is_canonical(&self) -> bool` (and/or `A2AIntent::parse(s) -> Result<Self,_>`) validating shape `^[a-z0-9]+(-[a-z0-9]+)*(:[a-z0-9]+(-[a-z0-9]+)*)?$` (lowercase, optional `namespace:verb`, bounded length; the de-facto shape already used: `diagnosis-handoff:read-only-evidence`, `rca-summary`, `code-mutation-directive`) — **existing free-form `A2AIntent::new` stays for back-compat**, the canonical helper is purely additive (abi-diff Added)
**And** a denial emits enough structured context (the fine-grained `intent` + `direction` + `peer`, already in `EIntentDenied`) that an operator can see *which* declared intent was rejected — denials are **fail-closed AND legible**, never silent
**And** [Q3 consensus = REQUIRED, not optional] an unreachable-entry diagnostic: a `tracing::warn!` (and/or debug-assertion) fires when an allowlist holds an intent that is neither a canonical fine-grained intent nor one of the 3 band tokens (catches the exact `smoke-a2a-loopback-6-3` typo class) — asserted via a `tracing-test` capture so the warning is regression-covered, not aspirational (Murat round-1 condition). A closed declared-intent **manifest registry** (Security's preferred config-load fail-closed) is **deferred** to a tracked follow-up: it is ADR-012's own documented "revisit when intent-class cardinality grows pathologically" trigger, which has not fired, so adopting it now would re-collapse the open vocabulary the ADR deliberately chose

### AC6 — Reference-Spirit wiring + runnable headline reflect fine-grained consent

**Given** the aspirational fine-grained allowlists in `smoke-a2a-loopback-6-3` and Mira/Nash
**When** the smoke / pairing is run
**Then** Mira's advisory carries `consent_envelope.intent_class = Some(A2AIntent("diagnosis-handoff:read-only-evidence"))` (replacing reliance on the `ADVISORY_CONSENT_INTENT = "readonly"` band constant at `spirits/mira/src/lib.rs:59`), Nash's `accept_allowlist` admits exactly that, and the deliberately-denied frame uses a fine-grained `"code-mutation-directive"` (NOT a band) that is rejected with `EIntentDenied` visible in the TL — **and per AC2 this population is mandatory: each smoke arm asserts no off-Host frame leaves with `intent_class == None`**
**And** a runnable headline (extend `smoke-a2a-loopback-6-3` at `crates/maos-bin/src/main.rs:4796-5019`, OR a new `smoke-a2a-consent-vocab-8-7` one-shot in `maos-bin` mirroring the 6.3/8.4/8.5/8.6 precedent — `maos-bin` smoke is NOT kernel KLOC) exits `0` and demonstrates: one fine-grained-admitted frame delivered + one fine-grained-denied frame logged — the observable end-to-end demo `[[feedback_lunarpulse_observability_preference]]`. Wire the arm into `.github/workflows/discipline.yml` alongside the existing `smoke-a2a-loopback-6-3` (lines 1387-1404) / `smoke-mira-nash-8-5` (lines 1406-1429) jobs (build with `--features fixture_replay`, `MAOS_ONE_SHOT=<arm>`)
**And** the `cross_host_consent_v1.rs` aspirational specific-intent assertion (`crates/maos-a2a/tests/cross_host_consent_v1.rs:75`) is now **truthful** — either updated in place or mirrored in a `cross_host_consent_v1_5.rs` (or `maos-a2a-core/tests/`, where enforcement now lives) with fine-grained scenarios (send-denied / both-admit / accept-mismatch at the fine-grained granularity)

### AC7 — Zero regression: all existing band-based behavior preserved

**Given** Stories 6.3 and 8.5 ship band-based consent (`"readonly"`/`"standard"`) and a full `cross_host_consent_v1.rs` suite, and 8.6 froze the protocol surface
**When** 8.7 lands
**Then** every existing A2A/consent test passes **unchanged** (the band fallback of AC1 guarantees frames without a fine-grained `intent_class` behave exactly as before); the `smoke-mira-nash-8-5`, `smoke-a2a-loopback-6-3`, and `smoke-a2a-tcp-8-6` arms still exit `0`; `IntentClass`, its `a2a_consent_intent_str()`, and the 3 bands are **not removed or renamed** (they remain the fallback + the manifest-declared coarse tier)
**And** the public signatures `frame_intent_str(&IacFrame) -> String`, `ConsentAllowlists::send_admits(&A2AIntent)` / `accept_admits(&A2AIntent)`, and the `EIntentDenied` / `AllowlistDirection` / `A2AError::{IntentDenied,IntentDeniedAtPeer}` types are **preserved** (the new fine-grained logic lives in the **private** `consent_match_key` helper + the private `send_admits`/`accept_admits` bodies + additive `A2AIntent` canonical helper + sender-side population) — **the SOLE preservation exception is the deliberate deletion of the dead `A2AConsentEnvelope` per AC2b** (ratified abi-diff Removed)
**And** the 8.6-frozen signatures (`verify_pinned`, `handle_intake`, `try_from_bytes`, `boot_nonce`/`logical_clock` placement) are **untouched** (8.6 §AC-A6) — a consent-fn *signature* change is the explicit RED flag 8.6 named; 8.7 changes only private *bodies* and adds additive helpers

### AC8 — Placement, ABI, kernel-KLOC, workspace, and discipline gates

**Given** the authoritative scope says "against `maos-a2a-core`, after 8.6" (epic-8 §AC-A6 Noted-gap; `sprint-status.yaml:90`) and 8.6 is now `done`
**When** 8.7 lands
**Then** the enforcement change lands in **`crates/maos-a2a-core`** (the crate 8.6 extracted that owns `router.rs` + `consent.rs` + `handle_intake`) — NOT in the over-budget `maos-a2a` (the `kloc-check` for `maos-a2a` (ceiling 1500, `xtask/kloc.toml`) and `maos-a2a-core` (ceiling 3000) both stay GREEN; record post-change line counts in evidence)
**And** `maos-kernel-core` is **byte-identical** to its pre-story state (zero-kernel-KLOC mandate; 8.4 proved 15505, 8.6 §AC-A7 held it); the kernel-KLOC sentinel is GREEN
**And** the workspace member count is **UNCHANGED at 41** (8.7 adds NO new crate — pure logic refinement); `check-workspace-count` stays at the `41` 8.6 set
**And** an `abi-diff` of `maos-a2a-core`'s public surface (use `--base` against `xtask/abi-baseline` — Story 8.3 lesson: no-base mode false-positives) is **Added-only EXCEPT the single ratified `A2AConsentEnvelope` Removed (AC2b)**: the new `A2AIntent` canonical helper is **Added**; `frame_intent_str`/`ConsentAllowlists`/`EIntentDenied`/`A2AError` public surfaces are **unchanged**; `A2AConsentEnvelope` is **Removed** (justified by the zero-non-test-caller grep + fail-open rationale, flagged for Winston); the private `consent_match_key` + private helper-body edits are invisible to abi-diff; the 8.6-frozen `verify_pinned`/`handle_intake`/`try_from_bytes` signatures are **untouched**
**And** all discipline gates are GREEN **at HEAD** (not flipped-while-red — the recurring AC4 trap `[[feedback_mechanical_gates_compound_promises_decay]]`); `dev_model_used` is recorded (§A2 discipline); `4-kernel-design.md` / `7-inter-agent-communication.md` are reconciled if any narrative says consent is 3-band-only (per the grounding check, neither currently over-claims — 7-iac.md:52 already describes per-frame typed-intent; verify and leave a note if accurate)

### AC9 — Commit the fail-closed-for-cross-Host end-state as a scheduled follow-up (Q2 consensus)

**Given** the Q2 consensus that fine-grained-when-present (AC1) is *transitional*, not the permanent posture, and that fail-closed-for-cross-Host (a cross-Host frame with absent/unrecognized `intent_class` is DENIED) is the committed long-term end-state
**When** 8.7 lands
**Then** a **new follow-up story (Story 8.8 — "fail-closed-for-cross-Host A2A consent")** is registered (alongside the Q4 `bmad-correct-course` adjustment) whose precondition is a **NEW sender-completeness discipline gate** asserting Security's non-negotiable invariant: *no cross-Host send path reaches the A2A router with an absent or unrecognized `intent_class`* — universal, well-typed population verified at the router-entry seam (NOT just the reference fleet)
**And** the gate is specified (per Murat round-2) to assert two things, GREEN-at-HEAD before any flip: (1) **sender-completeness** — a static/build-time scan + a runtime assertion in every smoke arm that no off-Host frame leaves with `intent_class == None`; (2) **fail-closed-readiness** — a test that, with the cross-Host router toggled to fail-closed mode, the FULL `a1_security_regression_guards` suite + all routed scenarios + both smoke arms still pass (i.e. the corpus is already B-clean), so the eventual flip is mechanical, never flipped-while-red
**And** Story 8.7 itself does **NOT** flip to fail-closed (no production sender populates `intent_class` today; flipping now would DENY live cross-Host traffic = flipped-while-red, the AC4 trap); 8.7 only ships the transitional mechanism (AC1), the mandatory reference-fleet population (AC2), and the registration of 8.8 + its gate spec

## Tasks / Subtasks

- [x] **Task 0 — Confirm placement (FAST — precondition already met)** (AC: #8)
  - [x] Confirm `maos-a2a-core` exists with the consent substrate present: `frame_intent_str`/`send_admits`/`accept_admits` (`router.rs:212-232`), `prepare_outbound` send-check (`router.rs:253-263`), `handle_intake` accept-check (`router.rs:420-431`), `ConsentAllowlists`/`A2AConsentEnvelope`/`EIntentDenied` (`consent.rs`), `CODE_INTENT_DENIED` (`transport/json_rpc.rs:30`). (Verified present 2026-06-05 — this is a confirm, not a blocker.)
- [x] **Task 1 — Add the fine-grained match key; do NOT rename `frame_intent_str`** (AC: #1, #4)
  - [x] Add private `consent_match_key(frame: &IacFrame) -> String`: `frame.consent_envelope.as_ref().and_then(|e| e.intent_class.as_ref()).map(|i| i.as_str().to_string()).unwrap_or_else(|| Self::frame_intent_str(frame))`.
  - [x] Point BOTH `send_admits` and `accept_admits` at `consent_match_key` (keep case-insensitive match). Point the two `EIntentDenied.intent` construction sites (`router.rs:259`, `router.rs:427`) at it too, so the reported `intent` matches the key actually tested.
  - [x] Keep `frame_intent_str` `pub` and unchanged in signature (it is the fallback primitive). Doc-comment `consent_match_key` with the precedence rule, citing ADR-012.
- [x] **Task 2 — Populate the per-frame intent (HARD for reference senders) + delete the dead footgun** (AC: #2, #2b)
  - [x] Ensure outbound frame construction sets `consent_envelope = Some(ConsentEnvelope { intent_class: Some(declared_intent), .. })` end-to-end for ALL reference cross-Host senders; add a small sender-side helper if the Spirit/driver API needs one. Confirm `prepare_outbound` does not strip it (it stamps `logical_clock` only — `router.rs:271`).
  - [x] Round-trip test: sent `consent_envelope.intent_class` == received, byte-equal. Smoke-arm assertion: no off-Host frame leaves with `intent_class == None`.
  - [x] **DELETE** the unused `A2AConsentEnvelope` struct + `From<ConsentEnvelope>` impl (`consent.rs:21-41`) + re-exports (`maos-a2a-core/src/lib.rs:48`, `maos-a2a/src/lib.rs:52`). Capture the zero-non-test-caller grep in evidence; record the abi-diff Removed + fail-open rationale; flag for Winston. (Fallback only if a Removed is refused: make `intent_class: Option<A2AIntent>` — never keep the `"standard"` discard.)
- [x] **Task 3 — Vocabulary hygiene** (AC: #5)
  - [x] Add additive `A2AIntent::is_canonical` (and/or `parse`) in `crates/maos-domain/src/invariants/i8.rs` (canonical-form regex; keep `A2AIntent::new` free-form). Pure addition.
  - [x] Add the unreachable-allowlist-entry `tracing::warn!` (+ optional debug-assert); cover it with a `tracing-test` capture so it is regression-pinned. (Manifest-registry deferred to a tracked follow-up per ADR-012 revisit trigger.)
- [x] **Task 4 — Reference Spirits + smoke + tests** (AC: #3, #6, #7)
  - [x] Mira emits fine-grained `"diagnosis-handoff:read-only-evidence"` via `consent_envelope.intent_class` (replace `ADVISORY_CONSENT_INTENT="readonly"` band reliance, `spirits/mira/src/lib.rs:59,335-358`); Nash `accept_allowlist` admits it (`spirits/mira/tests/a2a_pairing.rs:119-124`); add the `"code-mutation-directive"` confused-deputy negative.
  - [x] Update/extend `smoke-a2a-loopback-6-3` (`main.rs:4796-5019`, currently demos with `"standard"` at 4911-4912) or add `smoke-a2a-consent-vocab-8-7`: one fine-grained admit + one fine-grained deny, exits 0, denial in TL. Wire into `discipline.yml`.
  - [x] Make `cross_host_consent_v1.rs:75` aspirational specific-intent assertion truthful (update in place or add `cross_host_consent_v1_5.rs`, preferably under `maos-a2a-core/tests/`).
  - [x] Add positive/negative fine-grained tests; confirm band-fallback tests untouched.
- [x] **Task 5 — Gates & reconciliation** (AC: #7, #8)
  - [x] `kloc-check` GREEN for `maos-a2a` (1500) + `maos-a2a-core` (3000); record counts. `maos-kernel-core` byte-identical (15505). `check-workspace-count` = 41 (unchanged).
  - [x] `abi-diff` `maos-a2a-core` Added-only EXCEPT the ratified `A2AConsentEnvelope` Removed (use `--base`); `frame_intent_str`/`ConsentAllowlists`/`EIntentDenied`/`A2AError` unchanged; 8.6-frozen signatures untouched. Record the Removed justification.
  - [x] Run FULL existing A2A/consent/smoke suite — all GREEN at HEAD (incl. `smoke-a2a-tcp-8-6`). Record `dev_model_used`.
  - [x] Reconcile any `4-kernel-design.md` / `7-inter-agent-communication.md` narrative that asserts 3-band-only consent (grounding check found none over-claims; confirm and leave a note).
- [x] **Task 6 — Register downstream commitments (AC: #4, #9)**
  - [x] Run `bmad-correct-course` Direct Adjustment to register Story 8.7 formally in `epic-8-…md`, `epics/index.md`, `epics/dependency-dag.md` (mirror the 8.6 split), and register **Story 8.8 (fail-closed-for-cross-Host)** with its sender-completeness + fail-closed-readiness gate spec as the precondition. Add the 8.8 backlog row to `sprint-status.yaml`.

### Review Findings

- [x] [Review][Decision → Dismiss] Frame-level non-canonical intent warning — Team consensus (Winston, Murat, Amelia, Mary): unanimous NO. ADR-012 chose open vocabulary by design; canonical form is advisory hygiene for operator-controlled allowlists only. Frame-level warnings would create observability theater, compound the existing DoS vector, and potentially contradict Story 8.8's fail-closed enforcement. The spec's silence is intentional.

- [x] [Review][Patch] False-positive warning for case-differing/non-canonical entries [router.rs:268-281] — FIXED: warning message changed from "can never match" to "may not match canonical frame intents (matching is case-insensitive)" to accurately reflect `eq_ignore_ascii_case` semantics.
- [x] [Review][Patch] DoS amplification / log flooding on denials [router.rs:268-281] — FIXED: added `warned_entries: Arc<Mutex<HashSet<String>>>` to `A2ARouterCore`; each unique `(peer_id, entry, direction)` is warned once per router lifetime. Full allowlist scan still happens but log emission is deduplicated.
- [x] [Review][Patch] Security leak: raw intent strings logged at WARN on denials [router.rs:268-281] — FIXED: removed `intent = entry.as_str()` structured field from `tracing::warn!`; the raw string now appears only in the human-readable message, not in structured log output.
- [x] [Review][Patch] Zeroed consent_id collapses all fine-grained consents [frame.rs:433-444] — FIXED: `ConsentEnvelope::with_fine_grained_intent` now generates a unique monotonic `consent_id` from a static `AtomicU64` counter instead of hardcoding `[0u8; 16]`.
- [x] [Review][Patch] Test gap — case-insensitive matching untested for fine-grained intents [router.rs:930-931] — FIXED: added `case_insensitive_matching_for_fine_grained_intents` test (`cross_host_consent_v1_5.rs`) verifying `Diagnosis-Handoff:Read-Only-Evidence` allowlist admits `diagnosis-handoff:read-only-evidence` frame.
- [x] [Review][Patch] Misleading log target hardcoded to wrong module [router.rs:268-281] — FIXED: changed `target` from `"maos_a2a_core::consent"` to `"maos_a2a_core::router"`.
- [x] [Review][Patch] Brittle smoke test relies on substring matching of error messages [main.rs:5226-5231] — FIXED: changed from `message.contains(DENY_INTENT)` to prefix/suffix matching (`starts_with("intent {DENY_INTENT} ")` + `ends_with("for peer loopback")`) so formatting changes don't break the smoke.
- [x] [Review][Patch] Missing accept-side warning test [router.rs:938-950] — FIXED: added `warn_fires_on_accept_side_unreachable_entry` test (`cross_host_consent_v1_5.rs`) exercising the accept-side warning path via `handle_intake`.
- [x] [Review][Patch] Undocumented 128-byte canonical limit [i8.rs:38] — FIXED: added doc comment on `MAX_CANONICAL_INTENT_LEN` explaining the 128-byte bound and its purpose (preventing unbounded memory pressure from malicious intent strings).
- [x] [Review][Patch] Missing peer context in warning [router.rs:268-281] — FIXED: `warn_unreachable_entries` now takes `peer_id: &str` and includes `peer = peer_id` in the `tracing::warn!` structured fields.
- [x] [Review][Patch] Extremely long intent_class string — memory pressure/DoS [router.rs:250-257] — FIXED: `consent_match_key` now filters intent strings exceeding 1024 bytes (`filter(|i| i.as_str().len() <= 1024)`), falling back to the 3-band projection for oversized intents.

- [x] [Review][Defer] Consent envelope granter mismatch — replay attack [router.rs:250-257] — No validation that `consent_envelope.granter` matches `frame.from`. A stolen consent envelope could be replayed by a different sender. Pre-existing issue; not introduced by 8.7.
- [x] [Review][Defer] Expired consent masked by intent-denial error [router.rs:497-534] — The accept-side check runs `accept_admits` before `is_expired`. If both fail, the intent-denial error masks the expired-consent error. Pre-existing ordering issue in `handle_intake`; not introduced by 8.7.

- [x] [Review][Dismiss] Missing sender validation — "hard-mandatory" has no teeth — Intentional transitional design per Decision B/Q2 consensus: fine-grained-when-present, 3-band-fallback-otherwise. Story 8.8 will implement fail-closed-for-cross-Host.
- [x] [Review][Dismiss] Breaking public API deletion without deprecation — Ratified exception per Q5 consensus (2-1): delete the dead `A2AConsentEnvelope` fail-open. Zero non-test callers. Flagged for Winston's sign-off. Murat dissented on abi-discipline grounds.

## Dev Notes

### The exact code change (file:line — POST-8.6, grounded against `maos-a2a-core` 2026-06-05)

| Symbol | Current location (post-8.6) | Type / role | 8.7 change |
|---|---|---|---|
| `frame_intent_str` | `crates/maos-a2a-core/src/router.rs:212` | **`pub`** fn on the router (NEW: public since 8.6) | **KEEP** as-is — band fallback primitive; do NOT rename (abi-diff Removed if renamed) |
| `consent_match_key` | NEW, `router.rs` near 212 | **private** fn | add: prefer `consent_envelope.intent_class`, fall back to `frame_intent_str` |
| `send_admits` / `accept_admits` | `router.rs:217-232` | private fns | route through `consent_match_key` (no signature change) |
| `prepare_outbound` send-check | `router.rs:253-263` | step (1) ADR-012 send-allowlist | now denies on fine-grained key; `EIntentDenied.intent` (line 259) → `consent_match_key` |
| `handle_intake` accept-check | `router.rs:420-431` | ADR-012 accept-allowlist | now denies on fine-grained key; `EIntentDenied.intent` (line 427) → `consent_match_key` |
| `A2AConsentEnvelope` + `From` | `crates/maos-a2a-core/src/consent.rs:21-41` | `pub struct` + `From<ConsentEnvelope>` (discard at :37) | **DELETE** (Q5 consensus) — dead on hot path, zero non-test callers, `"standard"`-band discard is a latent fail-open elevation; accept abi-diff Removed (ratified exception) |
| `ConsentAllowlists{send,accept}_allowlist` | `consent.rs:50-67` | `Vec<A2AIntent>` + pub `send_admits`/`accept_admits(&A2AIntent)` | **unchanged** (already fine-grained-capable; the pub methods match by `==` and are not on the router hot path) |
| `IacFrame.consent_envelope` | `crates/maos-domain/src/frame.rs:36` | `Option<ConsentEnvelope>` | **unchanged** — the lever; consult + populate it |
| `ConsentEnvelope.intent_class` | `crates/maos-domain/src/frame.rs:408-421` | `Option<A2AIntent>`, `#[serde(default)]` | **unchanged** — read it in `consent_match_key`, set it on send |
| `A2AIntent` | `crates/maos-domain/src/invariants/i8.rs:32-44` | `String` newtype (no parse/is_canonical yet) | **add** `is_canonical`/`parse` (additive) |
| `IntentClass::a2a_consent_intent_str` | `crates/maos-domain/src/invariants/i1.rs:142-148` | 3-band projection | **unchanged** (now the fallback, via `frame_intent_str`) |
| `EIntentDenied{peer,intent,direction}` | `consent.rs:74-84` | rejection struct | **unchanged** shape; `intent` now fine-grained |
| `CODE_INTENT_DENIED` (-32001) | `crates/maos-a2a-core/src/transport/json_rpc.rs:30` | NACK code | **unchanged** |

**Critical reuse — do NOT reinvent:** the per-frame fine-grained intent field (`ConsentEnvelope.intent_class`), the allowlist types (`ConsentAllowlists`), the rejection type (`EIntentDenied`), the error variants (`A2AError::{IntentDenied, IntentDeniedAtPeer}`), the NACK code (`CODE_INTENT_DENIED`), and the defense-in-depth send+accept structure **all already exist and are correct**. This story is a ~30–60 line behavioral correction at the enforcement decision point + sender population + tests, NOT new substrate. The single most important line is `router.rs:213` (`frame.intent.a2a_consent_intent_str()` → consult `consent_envelope.intent_class` first via the new `consent_match_key`).

### Two post-8.6 facts that correct the original (pre-8.6) draft

1. **`frame_intent_str` is now `pub`** (`router.rs:212`) — it crossed into `maos-a2a-core`'s public surface during the 8.6 extraction and is used at 5 sites (`router.rs:212,219,227,259,427`). The original draft's "rename `frame_intent_str` → `consent_match_key`" would now register as an abi-diff **Removed** and fail AC8. Corrected: **add** a private `consent_match_key`, keep `frame_intent_str` as its fallback primitive.
2. **`A2AConsentEnvelope` + its `From<ConsentEnvelope>` are dead on the enforcement path** — grep (2026-06-05) finds the `From` impl has **zero non-test callers**, and `A2AConsentEnvelope` is only defined + re-exported (`lib.rs:48`, `maos-a2a/src/lib.rs:52`). The router enforces over `IacFrame.consent_envelope` directly, never via `A2AConsentEnvelope`. So the original AC2 premise ("the `From` discard makes enforcement misreport intent") is **false post-8.6**. The discard is cosmetically wrong but inert; "fixing" it by making the `pub` field `Option` is an abi-diff Modified. Downgraded to optional cleanup (Q5). The real AC2 work is **populating** `frame.consent_envelope.intent_class` (which no sender does today).

### Why ABI-neutral (and why 8.6 §AC-A6 is not violated)

8.6's §AC-A6 froze `verify_pinned` / `handle_intake` / `try_from_bytes`, mandated "consent rides the EXISTING JSON-RPC field, NOT a new TCP-specific field," and named **a consent-fn *signature* change a RED flag DURING 8.6**. 8.7 honors all of it: it changes the **private** enforcement *bodies* (not signatures), adds a **private** helper, consults an **existing** wire field (`consent_envelope.intent_class`, already serialized in the JSON-RPC frame, `#[serde(default)]`), and adds only an **additive** `A2AIntent` canonical helper. 8.7 is precisely the "future consent-vocabulary story… *after* 8.6 lands the seam, against `maos-a2a-core`" that the Noted-gap authorizes (epic-8:274-275) — the legitimate moment to reopen consent enforcement.

### Decisions (defaults chosen; forks flagged for the user at the end)

- **Decision A — Use the existing `ConsentEnvelope.intent_class` as the per-frame intent carrier.** (vs. widening `IntentClass` enum — ABI-breaking on a frozen 3-variant enum; vs. adding a new `IacFrame` field — redundant, the envelope field already exists and is documented for exactly this.) Confident default.
- **Decision B (RESOLVED — Q2 team consensus, 3-0 round 2) — Fine-grained-when-present NOW as a *transitional* mechanism; fail-closed-for-cross-Host committed as scheduled Story 8.8.** 8.7 ships fine-grained-when-present/band-fallback-otherwise (additive, zero-regression to unmigrated band-only frames) AND makes `intent_class` population hard-mandatory for every reference cross-Host sender (AC2), so (A) is never the permanent state. Fail-closed-for-cross-Host (a cross-Host frame with absent/unrecognized `intent_class` is DENIED) is the committed end-state, flipped in Story 8.8 once a new sender-completeness gate proves universal population at the router-entry seam (AC9). Rationale: the enforcement path IS the cross-Host router (same-Host never reaches it), so fail-closed needs no discriminator; but no production sender populates `intent_class` today, so flipping now would be flipped-while-red. Synthesis honors all three lenses: additive/authorized (Winston), no regression-net churn now + testable gate (Murat), fail-open neutralized by mandatory population + committed gated flip (Security).
- **Decision C (RESOLVED — Q3 team consensus, 3-0 on the floor) — Keep `A2AIntent` free-form; add additive canonical helper + REQUIRED unreachable-entry `tracing::warn!`.** Avoids an enum/registry that would re-collapse the open vocabulary ADR-012 deliberately chose. The `tracing::warn!` (regression-pinned via `tracing-test`) makes the silent-never-match failure mode loud. The closed declared-intent **manifest registry** (Security's config-load fail-closed preference) is deferred to a tracked follow-up — it is ADR-012's documented "revisit when cardinality grows pathologically" trigger, which has not fired.
- **Decision D — Placement is `maos-a2a-core` (RESOLVED, no longer a fork).** 8.6 is done; the seam exists; the original "8.6 first vs build-now-against-`maos-a2a`" sequencing fork is closed. All edits land in `maos-a2a-core` / `maos-domain` / `maos-bin` / `spirits`.
- **Decision E (RESOLVED — Q5 team consensus, 2-1) — DELETE the dead `A2AConsentEnvelope` + `From`.** Its `From` silently coerces a missing intent to the `"standard"` privilege band — a latent fail-open *elevation* the instant anyone wires it onto the path. Dead on the hot path, zero non-test callers. Winston (freeze-owner) + Security voted DELETE and both rejected "leave + doc-comment" as a decaying-promise / loaded-gun pattern; Murat dissented on abi-discipline grounds. Accept the abi-diff **Removed** with zero-caller grep + fail-open justification, flagged for Winston ratification (mirrors the 8.6 fail-closed flag). Fallback if a Removed is categorically refused: `Option`-ify the field (abi Modified) — never keep the `"standard"` discard.

### Source tree — what to touch

- `crates/maos-a2a-core/src/router.rs` — add `consent_match_key`; point `send_admits`/`accept_admits` + the two `EIntentDenied.intent` sites at it. **Primary edit site.**
- `crates/maos-domain/src/invariants/i8.rs` — additive `A2AIntent::is_canonical`/`parse`.
- `crates/maos-a2a-core/src/consent.rs` — **DELETE** `A2AConsentEnvelope` + `From<ConsentEnvelope>` (lines 21-41) + the re-exports in both crates' `lib.rs` (AC2b).
- `crates/maos-domain/src/frame.rs` — `ConsentEnvelope.intent_class` is the carrier; **no change** expected (read-only reference).
- `spirits/mira/src/lib.rs`, `spirits/nash/src/lib.rs` (+ `spirits/mira/tests/a2a_pairing.rs`) — emit/accept fine-grained intents via `consent_envelope.intent_class` (**mandatory population**, AC2); replace `ADVISORY_CONSENT_INTENT="readonly"` band reliance.
- `crates/maos-bin/src/main.rs` — `smoke-a2a-loopback-6-3` (lines 4796-5019) update or new `smoke-a2a-consent-vocab-8-7`; wire into `discipline.yml` like the 6-3/8-5/8-6 smokes. **Not kernel KLOC.**
- `crates/maos-a2a/tests/cross_host_consent_v1.rs` (line 75) and/or new `_v1_5.rs` (prefer `maos-a2a-core/tests/`) — make aspirational fine-grained assertions truthful.
- `.github/workflows/discipline.yml` — new smoke job near lines 1387-1429 if adding `smoke-a2a-consent-vocab-8-7`.
- `xtask/kloc.toml` — verify `maos-a2a` (1500) + `maos-a2a-core` (3000) ceilings stay GREEN; no bump.

### Testing standards

- Unit: positive (fine-grained admit), negative (fine-grained deny → `EIntentDenied` carrying the literal string), fallback (no `intent_class` → band gate unchanged), round-trip (sent==received `intent_class`), defense-in-depth (send-side AND accept-side deny). Mirror the structure of `cross_host_consent_v1.rs` scenarios.
- Integration/smoke: `maos-bin` one-shot exits 0, denial observable in Transparency Log (the `[[feedback_lunarpulse_observability_preference]]` runnable demo).
- Regression: FULL existing A2A + consent + `smoke-mira-nash-8-5` + `smoke-a2a-loopback-6-3` + `smoke-a2a-tcp-8-6` suite GREEN at HEAD.
- Determinism: no `SystemTime::now()` in consent decisions; no new flakes.

### Previous-story intelligence (8.1–8.6 patterns to reuse)

- **In-proc bridge pattern (8.1–8.5):** reference Spirits consume kernel/A2A substrate as **dev-deps**; Spirit-side code is a pure lib. Mira/Nash already do this — extend, don't restructure.
- **`register_spirit_typed` handle must be bound or the mailbox closes (`ChannelClosed`)** — Story 8.4 lesson `[[project_story_8_4_landed]]`. Applies if you add any new wiring in the smoke.
- **`abi-diff` needs the `--base` flag** — Story 8.3 lesson `[[project_story_8_3_landed]]`; no-base mode false-positives. Use `--base` (baseline lives under `xtask/abi-baseline`) when checking `maos-a2a-core`.
- **NEVER `cargo fmt -p crate` here** — Story 7.5a lesson; whole-crate collateral. Format only touched files.
- **`kloc-check` is pre-existing RED on some crates** — verify your change is *neutral* (8.3/8.4/8.5 precedent), don't try to fix unrelated reds.
- **Discipline gates flip GREEN at HEAD, never flipped-while-red** — recurring Epic-4/5/7 "AC4 trap" `[[feedback_mechanical_gates_compound_promises_decay]]`.
- **Loopback peer lookup keys `HostId == peer_id`** — Story 8.5 lesson `[[project_story_8_5_landed]]`; relevant if the smoke adds peers.
- **8.6 froze the protocol surface** `[[project_story_8_6_spec_landed]]` — touch private bodies only; a consent-fn *signature* change is the named RED flag.

### Project Structure Notes

- **Planning-artifact gap to flag:** Story 8.7 exists ONLY as (a) the epic-8 §AC-A6 Noted-gap paragraph (`epic-8-…:274-275`), (b) the `sprint-status.yaml:90` backlog comment, and (c) the `deferred-work.md:226-227` entry. There is **no formal `## Story 8.7` section** in `epic-8-…miranash-v03-v15.md`, `epics/index.md`, or `epics/dependency-dag.md`, and **no `sprint-change-proposal` registers it** (unlike 8.6, which got a Direct-Adjustment split on 2026-06-04). Recommend a `bmad-correct-course` Direct Adjustment to register 8.7 in those source artifacts (Open Question Q4). This story file is authored against the Noted-gap as the authoritative scope.
- **Dependency direction:** 8.7 → 8.6 (`maos-a2a-core`, now DONE) → {8.5 loopback pair, 6.3 A2A mesh}. 8.7 is a v1.5+ refinement, last in the Epic-8 chain.

### References

- [Source: epics/epic-8-…miranash-v03-v15.md:274-275] — §AC-A6 "Noted gap" (Winston, 2026-06-04): the authoritative scope; "silently never match" diagnosis; "after 8.6, against `maos-a2a-core`" mandate; open-vocabulary intent.
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml:89-91] — 8.6 `done`; 8.7 backlog comment restating the gap + DEPENDS-ON-8.6; 8.7 now `ready-for-dev`.
- [Source: _bmad-output/implementation-artifacts/deferred-work.md:226-227] — consent intent taxonomy gap entry.
- [Source: docs/adr/ADR-012-typed-intent-a2a-consent.md:10-18] — consent is `(peer-identity, intent-class)`; `diagnosis-handoff:read-only-evidence` admissible, `code-mutation-directive` rejected; open vocabulary by design; revisit trigger.
- [Source: architecture-maos-minimal-opus/7-inter-agent-communication.md:19,52] — per-frame typed-intent consent; `consent_envelope.intent_class` on the wire; `EIntentDenied`.
- [Source: crates/maos-a2a-core/src/router.rs:212-232,253-263,420-431] — the enforcement path post-8.6 (the bug + the two allowlist call sites + the two `EIntentDenied.intent` sites at 259/427).
- [Source: crates/maos-a2a-core/src/consent.rs:21-41,50-84] — `A2AConsentEnvelope` (`From` discard at :37, dead on hot path), `ConsentAllowlists` (+ pub `send_admits`/`accept_admits`), `EIntentDenied`, `AllowlistDirection`.
- [Source: crates/maos-a2a-core/src/transport/json_rpc.rs:30] — `CODE_INTENT_DENIED = -32001`.
- [Source: crates/maos-domain/src/frame.rs:36,408-421] — `IacFrame.consent_envelope: Option<ConsentEnvelope>` + `ConsentEnvelope.intent_class: Option<A2AIntent>` (the lever).
- [Source: crates/maos-domain/src/invariants/i1.rs:121-149] — `IntentClass` + `a2a_consent_intent_str()` (the 3-band fallback).
- [Source: crates/maos-domain/src/invariants/i8.rs:32-44] — `A2AIntent` newtype (no canonical helper yet).
- [Source: crates/maos-bin/src/main.rs:4796-5019,4826-4840,4911-4912] — `smoke-a2a-loopback-6-3` + its aspirational fine-grained allowlists (demo currently uses `"standard"`).
- [Source: crates/maos-a2a/tests/cross_host_consent_v1.rs:75] — aspirational specific-intent assertion to make truthful.
- [Source: spirits/mira/src/lib.rs:59,335-358; spirits/mira/tests/a2a_pairing.rs:39-51,119-124] — `ADVISORY_CONSENT_INTENT="readonly"`, advisory construction, Nash accept_allowlist wiring.
- [Source: .github/workflows/discipline.yml:1387-1404,1406-1429] — `smoke-a2a-loopback-6-3` / `smoke-mira-nash-8-5` job wiring (fixture_replay, `MAOS_ONE_SHOT`).
- [Source: xtask/kloc.toml] — `maos-a2a`=1500, `maos-a2a-core`=3000 ceilings.
- [Source: _bmad-output/implementation-artifacts/8-6-…maos-a2a-tcp-two-process.md §AC-A1/AC-A6/AC-A7] — `maos-a2a-core` extraction (moves `handle_intake`+consent substrate); consent rides the existing JSON-RPC field; consent-fn signature change is a RED flag; zero-kernel-KLOC (15505); workspace 39→41.

## Dev Agent Record

### Agent Model Used

**claude-opus-4-8** (Amelia / dev-story). Small-surface, high-precision security-semantics change where correctness of the enforcement decision-point (the confused-deputy gate) dominates — the strongest model was favored over throughput, consistent with the 8.1–8.6 Epic-8 recommendation.

### Debug Log References

- **Pre-existing maos-bin compile break (FIXED to unblock smoke builds):** `maos-bench::decide` gained a 3rd `j6: Option<&JourneyResult>` arg in Story 8.5 (J6 cold-start) but `maos-bin/src/main.rs:3034` still called it with 2 args (`decide(j1, j4)`). This break is present at HEAD (verified via `git stash`) and blocks `cargo build -p maos-bin --features fixture_replay` — i.e. all three A2A smoke CI jobs. Fixed minimally with `decide(j1, j4, None)` (this bench mode runs no J6). Same recurring break class the 7.3 / 8.6 stories fixed.
- **Two further pre-existing build reds confirmed UNRELATED + neutral** (both reproduce at HEAD with my work stashed, neither touched by 8.7): `maos-mcp` test `mcp_client_trait_test` needs `--features fixture_replay` (feature-gated `fixture_replay` module import); `maos-registry` test `registry_roundtrip_test` is missing `AdmissionConfig::{runtime_crypto_provider, runtime_provider_endpoint}` fields.
- **Loopback enforcement model (smoke wiring):** `route_outbound` checks the DESTINATION's `send_allowlist`; `handle_intake` checks the SOURCE's `accept_allowlist` (`tests/a2a_pairing.rs` doc). The `smoke-a2a-consent-vocab-8-7` deny arm therefore puts `code-mutation-directive` in host_b's `send_allowlist` but excludes it from host_a's `accept_allowlist` to land an accept-side `IntentDeniedAtPeer`.
- **abi-diff scope:** the mechanical `abi-diff` gate scans ONLY `maos-spirit-abi` (`xtask/src/abi_diff.rs:8`), which 8.7 does not touch — so the `A2AConsentEnvelope` deletion (a `maos-a2a-core` public-surface Removed) does NOT trip the gate; gate result `passed: true, removed: []`. The Removed is recorded here + flagged for Winston per AC2b/AC8.
- **`tracing-test` substitution:** AC5 names `tracing-test` for the warn capture; to avoid a brand-new external crate (not in `Cargo.lock`), the capture uses the already-vendored `tracing-subscriber` (`with_default` + custom `MakeWriter` into a shared buffer). Same regression guarantee (Murat's "not aspirational" condition met).

### Completion Notes List

- **AC1 — fine-grained match + band fallback (Task 1):** Added the private `A2ARouterCore::consent_match_key(frame) -> String` (`router.rs`): prefers `consent_envelope.intent_class`, falls back to `frame_intent_str` (the 3-band projection). Both `send_admits`/`accept_admits` AND both `EIntentDenied.intent` construction sites (prepare_outbound + handle_intake) route through it, so the key tested == the key reported. `frame_intent_str` kept `pub` and unchanged (no abi-diff Removed). Precedence documented on `consent_match_key`, citing ADR-012. Covered by `cross_host_consent_v1_5.rs` (fine-grained admit, fine-grained deny reports literal string not band).
- **AC2 — mandatory reference-fleet population (Task 2):** Added `ConsentEnvelope::with_fine_grained_intent(granter, intent)` sender-side helper (`maos-domain/src/frame.rs`, additive). Populated `intent_class = Some(..)` end-to-end for Mira/Nash (`a2a_pairing.rs`), `smoke-mira-nash-8-5`, `smoke-a2a-loopback-6-3`, and the new `smoke-a2a-consent-vocab-8-7`. Each smoke arm + the bilateral pairing test assert no off-Host frame leaves with `intent_class == None`; a wire round-trip test (`serde → try_from_bytes`) asserts byte-equality.
- **AC2b — DELETE the dead `A2AConsentEnvelope` (Task 2):** Verified zero non-test callers (grep 2026-06-05: only the definition + 2 re-exports). Deleted the struct + `From<ConsentEnvelope>` (the `None → "standard"`-band privilege-elevation footgun) + both `lib.rs` re-exports. **abi-diff Removed on `maos-a2a-core`'s public surface — the ONE ratified exception to AC8's Added-only, FLAGGED FOR WINSTON's sign-off** (Q5 consensus 2-1; Murat dissented on abi-discipline grounds — expect this in review). The mechanical abi-diff gate (maos-spirit-abi-only) stays GREEN.
- **AC3 — confused-deputy closed at fine granularity (Task 4):** `code-mutation-directive` and `diagnosis-handoff:read-only-evidence` both project to the `readonly` band; the fine-grained gate admits the latter and rejects the former with `-32001` naming the literal directive. Executable in `cross_host_consent_v1_5.rs::confused_deputy_closed_at_fine_granularity_on_accept`, `a2a_pairing.rs::confused_deputy_directive_denied_while_advisory_admitted`, and `smoke-a2a-consent-vocab-8-7`.
- **AC4 — defense-in-depth both directions (Task 1/4):** Send-side (`prepare_outbound`) and accept-side (`handle_intake`) enforce independently on the fine-grained key; `EIntentDenied`/`AllowlistDirection`/`A2AError::{IntentDenied,IntentDeniedAtPeer}` preserved, now carrying fine-grained strings. Covered by `defense_in_depth_independent_on_fine_grained_key`.
- **AC5 — vocabulary hygiene (Task 3):** Added additive `A2AIntent::is_canonical` + `A2AIntent::parse` + `NonCanonicalIntent` + `MAX_CANONICAL_INTENT_LEN` (`i8.rs`) for grammar `^[a-z0-9]+(-[a-z0-9]+)*(:[a-z0-9]+(-[a-z0-9]+)*)?$`; `A2AIntent::new` stays free-form. Added `warn_unreachable_entries` (`router.rs`) emitting `tracing::warn!` for non-canonical allowlist entries on a denial; regression-pinned via a `tracing-subscriber` capture test. Manifest-registry deferred (ADR-012 revisit trigger not fired).
- **AC6 — reference wiring + runnable headline (Task 4):** Mira declares `ADVISORY_FINE_GRAINED_INTENT = "diagnosis-handoff:read-only-evidence"` (band `ADVISORY_CONSENT_INTENT` retained for fallback/back-compat). New `smoke-a2a-consent-vocab-8-7` one-shot (one fine-grained admit + one fine-grained deny, exits 0) wired into `discipline.yml` + the gate-aggregation `needs` list + the `MAOS_ONE_SHOT` dispatch/known-modes. `cross_host_consent_v1.rs:75` made truthful via the mirror `maos-a2a-core/tests/cross_host_consent_v1_5.rs` (+ a clarifying note in v1 that it exercises the band-fallback path).
- **AC7 — zero regression (Task 5):** All existing A2A/consent tests pass unchanged (`cross_host_consent_v1` 8/8, `a1_security_regression_guards` 7/7, restart/chaos/churn suites, maos-a2a-tcp 8-6 suite). The three smokes + `smoke-a2a-tcp-8-6` exit 0. `frame_intent_str`/`ConsentAllowlists`/`EIntentDenied`/`A2AError` + 8.6-frozen signatures untouched; sole preservation exception = the ratified `A2AConsentEnvelope` deletion.
- **AC8 — placement/ABI/KLOC/workspace/discipline (Task 5):** Enforcement lands in `maos-a2a-core`. `kloc-check`: `maos-a2a` 201/1500 GREEN, `maos-a2a-core` 2644/3000 GREEN. `maos-kernel-core` BYTE-IDENTICAL (zero-kernel-KLOC). `check-workspace-count` = 41 (unchanged). `abi-diff passed: true`. `check-dev-model-used-populated` + `check-dev-record-completeness` GREEN (also backfilled Story 8.6's missing `dev_model_used: claude-opus-4-8` field — the §A2 reconciliation that flips both gates green at HEAD). Architecture narratives verified NOT over-claiming 3-band-only (7-iac.md:52 already documents per-frame typed-intent; 4-kernel-design.md has no band-only claim) — no reconciliation edit needed.
- **AC9 — fail-closed end-state committed (Task 6):** Story 8.8 (fail-closed-for-cross-Host) registered (epic-8 §Story 8.8 + index + dependency-dag + sprint-status backlog) with its sender-completeness + fail-closed-readiness gate spec as precondition. 8.7 ships only the transitional mechanism + mandatory reference-fleet population; does NOT flip to fail-closed (no production sender populates `intent_class` yet — flipping now = flipped-while-red).
- **Task 6 / Q4:** planning-artifact registration of 8.7 + 8.8 was already completed via `bmad-correct-course` Direct Adjustment (`sprint-change-proposal-2026-06-05.md`); verified coherent across epic-8 / index / dependency-dag / sprint-status.

### File List

**Production (enforcement + types):**
- `crates/maos-a2a-core/src/router.rs` — NEW private `consent_match_key` + `warn_unreachable_entries`; `send_admits`/`accept_admits` + 2 `EIntentDenied.intent` sites routed through it; `frame_intent_str` kept `pub`/unchanged; `A2AIntent` import added.
- `crates/maos-a2a-core/src/consent.rs` — DELETED `A2AConsentEnvelope` + `From<ConsentEnvelope>` (+ unused `FrameAddress` import); deletion rationale comment.
- `crates/maos-a2a-core/src/lib.rs` — removed `A2AConsentEnvelope` re-export.
- `crates/maos-a2a/src/lib.rs` — removed `A2AConsentEnvelope` re-export.
- `crates/maos-a2a-core/Cargo.toml` — `tracing` dep + `tracing-subscriber` dev-dep (AC5).
- `crates/maos-domain/src/invariants/i8.rs` — additive `A2AIntent::is_canonical`/`parse` + `NonCanonicalIntent` + `MAX_CANONICAL_INTENT_LEN` (+ unit tests).
- `crates/maos-domain/src/frame.rs` — additive `ConsentEnvelope::with_fine_grained_intent` sender helper.
- `spirits/mira/src/lib.rs` — new `ADVISORY_FINE_GRAINED_INTENT` const; `ADVISORY_CONSENT_INTENT` doc updated/retained.

**Reference wiring + smoke (maos-bin — NOT kernel KLOC):**
- `crates/maos-bin/src/main.rs` — NEW `smoke_a2a_consent_vocab_8_7` arm + dispatch + known-modes; `smoke-a2a-loopback-6-3` & `smoke-mira-nash-8-5` migrated to fine-grained + AC2 assertions; pre-existing `decide(j1, j4)` → `decide(j1, j4, None)` fix.
- `.github/workflows/discipline.yml` — new `smoke-a2a-consent-vocab-8-7` job + added to gate-aggregation `needs`.

**Tests:**
- `crates/maos-a2a-core/tests/cross_host_consent_v1_5.rs` — NEW: fine-grained admit/deny, confused-deputy, defense-in-depth, wire round-trip byte-equality, band fallback, warn capture.
- `spirits/mira/tests/a2a_pairing.rs` — `advisory_frame` gains `fine_intent: Option`; bilateral test migrated to fine-grained + intent_class assertion; NEW confused-deputy negative; band-fallback denial retained.
- `crates/maos-a2a/tests/cross_host_consent_v1.rs` — clarifying note on `scenario_3_1` (band-fallback path; fine-grained mirrored in v1_5).

**Planning artifacts:**
- `_bmad-output/implementation-artifacts/8-6-…maos-a2a-tcp-two-process.md` — backfilled `dev_model_used: claude-opus-4-8` frontmatter (§A2 reconciliation; clears pre-existing dev-record gate red).
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — 8-7 status transitions.

### Change Log

| Date | Change |
|---|---|
| 2026-06-05 | Implemented Story 8.7 (claude-opus-4-8 / dev-story): fine-grained typed-intent consent enforcement in `maos-a2a-core` (`consent_match_key`, fine-grained-when-present + band-fallback), mandatory reference-fleet `intent_class` population, DELETE dead `A2AConsentEnvelope` fail-open (ratified abi-diff Removed, flagged for Winston), additive `A2AIntent::is_canonical`/`parse` + unreachable-entry `tracing::warn!`, new `smoke-a2a-consent-vocab-8-7` + `cross_host_consent_v1_5.rs`. All ACs satisfied; kloc/abi/workspace/kernel/dev-record gates GREEN at HEAD. Fixed a pre-existing maos-bin `decide()` compile break + backfilled Story 8.6's `dev_model_used` (§A2). Status → review. |

---

## ✅ Team Consensus — design decisions resolved (Winston + Murat + Security, 2026-06-05)

All four open questions were taken to the team (the trio that resolved the 8.6 `ca_roots` fork) under the FIXED criterion: **most faithful to spec (ADR-012 + the 8.6 freeze) AND most correct long-term — explicitly NOT least-effort.** Q1 (the old sequencing blocker) was already moot once 8.6 landed. Results are binding and woven into the ACs above; recorded here with rationale and dissents for provenance.

**Q2 — Fallback semantics → SYNTHESIS (unanimous, round 2).** Round 1 split 2-1 (Winston+Murat for additive (A); Security for fail-closed (B)). A decisive structural fact surfaced mid-deliberation — *the entire `prepare_outbound`/`handle_intake` enforcement path IS the cross-Host A2A router; same-Host IAC never reaches it* — and the criterion explicitly deprioritizes the regression-churn cost that anchored the (A) votes. The reconciled, unanimous decision: **ship fine-grained-when-present NOW as a transitional mechanism (AC1) + make `intent_class` population hard-mandatory for every reference cross-Host sender (AC2) + commit fail-closed-for-cross-Host as scheduled Story 8.8 (AC9)**, gated on a sender-completeness gate. (A) is never permanent; (B) is the committed end-state with a concrete, gate-guarded trigger. This is more spec-faithful than pure (A) (which leaves a permanent fail-open) and more honest-at-HEAD than (B)-now (which would flip-while-red, since no sender populates `intent_class` yet).

**Q3 — Vocabulary strictness → (b) (unanimous floor).** Additive `A2AIntent::is_canonical`/`parse` + a REQUIRED `tracing::warn!` on unreachable allowlist entries (regression-pinned via `tracing-test`). The closed declared-intent **manifest registry** (Security's config-load fail-closed preference) is **deferred** to a tracked follow-up — it is ADR-012's own documented "revisit when intent-class cardinality grows pathologically" trigger, which has not fired; adopting it now would re-collapse the open vocabulary the ADR deliberately chose (Winston). (a) helper-only was rejected as leaving the root "silent never-match" complaint half-closed.

**Q4 — Planning-artifact registration → YES (unanimous).** Register 8.7 formally via `bmad-correct-course` Direct Adjustment (mirroring the 8.6 split) in `epic-8-…md` / `epics/index.md` / `epics/dependency-dag.md`, **before** `dev-story` — a `ready-for-dev` security-semantics story existing only as a Noted-gap + tracker comments is exactly the tracker-vs-source-of-truth drift the project's retros keep flagging `[[feedback_mechanical_gates_compound_promises_decay]]`. Register Story 8.8 in the same pass (AC9).

**Q5 — `A2AConsentEnvelope` cleanup → (c) DELETE (2-1).** The dead `From<ConsentEnvelope>` silently coerces a missing intent to the `"standard"` privilege band (`consent.rs:37`) — a latent fail-open *privilege elevation* if anyone ever wires the type onto the path. Winston (freeze-owner) + Security voted DELETE and both **rejected "leave + doc-comment"** as a decaying-promise / loaded-gun pattern; Murat dissented (prefers (a), keep AC8 strictly Added-only, do deletions in a dedicated abi-bump story). Resolution: **delete**, accept the abi-diff **Removed** with zero-non-test-caller grep + fail-open justification, flagged for Winston's ratification (the one sanctioned exception to AC8). Fallback if a Removed is categorically refused at review: `Option`-ify the field (abi Modified) — never keep the `"standard"` discard.

### Single remaining item for the user (not a design fork — an action to authorize)

Q4's consensus requires running `bmad-correct-course` to register Stories 8.7 + 8.8 in the planning artifacts. This edits the epic / index / dependency-dag / sprint-status. **Authorize this run** (recommended before `dev-story`), or proceed to `dev-story` and register afterward (carries the drift risk the team flagged).
