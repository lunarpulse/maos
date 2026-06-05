# Story 8.7: Fine-Grained Typed-Intent Consent Vocabulary over `maos-a2a-core`

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

> **⛔ HARD PRECONDITION — BLOCKED ON STORY 8.6.** This story MUST land its consent-enforcement
> change inside **`maos-a2a-core`** (the protocol-seam crate that **Story 8.6 creates by extraction**).
> As of this story's creation (2026-06-04) **Story 8.6 is `backlog` (not built)** — `maos-a2a-core`
> does not yet exist; the enforcement code still lives in `crates/maos-a2a/src/adapter.rs:144-164` and
> the consent types in `crates/maos-a2a/src/consent.rs`. Per the DAG rule (forward deps resolved by
> *ordering* the dependency first, or by a *documented stub*), **the default sequencing is: build 8.6
> first, then 8.7 against the extracted seam.** A "land-now-against-`maos-a2a`" fallback exists and is
> documented under Decision D — but the authoritative plan (epic-8 §AC-A6 Noted-gap, 2026-06-04;
> `sprint-status.yaml` 8.7 comment) is **after 8.6, against `maos-a2a-core`.** Do not start
> implementation until this sequencing is confirmed — see the **Open Questions** at the end.

## Story

As a **MAOS operator wiring cross-Host A2A consent (and the Spirit authors — Mira, Nash — who depend on it)**,
I want **the ADR-012 consent gate to enforce the actual fine-grained per-frame intent string an operator declares in a `ConsentAllowlists` (e.g. `diagnosis-handoff:read-only-evidence`, `rca-summary`), instead of silently collapsing every frame to one of the three coarse `IntentClass` bands `{highprivilege, standard, readonly}`**,
so that **ADR-012 is the "typed-*intent* consent" it was decided to be — the confused-deputy gap (Mira admissibly handing Nash `diagnosis-handoff:read-only-evidence` while `code-mutation-directive` is rejected) is closed for real, and an allowlist entry an operator writes can no longer fail-open by silently never matching.**

## Context & Problem Statement

**This story exists because of a gap Story 8.5 surfaced and Winston deferred** (epic-8 §AC-A6 Noted-gap, 2026-06-04; recorded again as the `8-7-…` comment in `sprint-status.yaml:90`).

ADR-012 (`docs/adr/ADR-012-typed-intent-a2a-consent.md`) **decides**: "Cross-Host A2A consent is `(peer-identity, intent-class)`, not `(peer-identity)`… Mira's `diagnosis-handoff:read-only-evidence` is admissible at Nash; `code-mutation-directive` is rejected." The architecture restates it (`7-inter-agent-communication.md:52`): "The kernel rejects frames whose typed intent is not in the sender's send-allowlist or the receiver's accept-allowlist with `EIntentDenied`. This is what makes Mira's `diagnosis-handoff:read-only-evidence` admissible at Nash while `code-mutation-directive` is rejected."

**But the implementation collapses every frame to 3 coarse bands.** The enforcement path is:

```rust
// crates/maos-a2a/src/adapter.rs:141-164  (today's home; moves to maos-a2a-core after 8.6)
fn frame_intent_str(frame: &IacFrame) -> String {
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

`IntentClass::a2a_consent_intent_str()` (`crates/maos-domain/src/invariants/i1.rs:139-149`) projects the 3-variant enum to exactly `{"highprivilege","standard","readonly"}`. So:

- `ConsentAllowlists.send_allowlist` / `accept_allowlist` are `Vec<A2AIntent>` (free-form open-vocabulary `String` newtypes — `crates/maos-a2a/src/consent.rs:49-57`, `A2AIntent` at `crates/maos-domain/src/invariants/i8.rs:32-44`).
- An operator writes `A2AIntent::new("diagnosis-handoff:read-only-evidence")` into an allowlist (as `smoke-a2a-loopback-6-3` and `cross_host_consent_v1.rs` **aspirationally do** — `crates/maos-bin/src/main.rs:4617-4632`, `crates/maos-a2a/tests/cross_host_consent_v1.rs:75,100`).
- The frame projects to `"readonly"` / `"standard"`. The specific string **silently never matches** → the entry is dead. The gate is effectively "typed-*class* consent," NOT "typed-intent consent."

**The lever that makes the fix ABI-neutral:** the per-frame fine-grained intent **already rides the wire**. `ConsentEnvelope` (`crates/maos-domain/src/frame.rs:407-421`) carries:

```rust
/// Story 6.3 / ADR-012 binding-v0.9 — typed-intent for cross-Host consent.
/// Filled by the sender's A2A outbound path; verified by the receiver's A2A intake.
#[serde(default)]
pub intent_class: Option<crate::invariants::i8::A2AIntent>,   // ← the real per-frame fine-grained intent
```

The field exists, is serde-additive (`#[serde(default)]`), and is documented as the intent the sender fills and the receiver verifies. **It is simply never consulted by the enforcement path** — and never populated by senders (see below). The `A2AConsentEnvelope::From` conversion even reads it but **discards the signal** by defaulting to `"standard"` when `None` (`crates/maos-a2a/src/consent.rs:31-41`):

```rust
intent_class: env.intent_class.unwrap_or_else(|| A2AIntent::new("standard")),
```

**So this story is a refinement, not new substrate:** make enforcement read the per-frame `ConsentEnvelope.intent_class` (the real `A2AIntent`) when present, and make senders populate it. No new `IacFrame` field, no `IntentClass` enum widening, no JSON-RPC field added (AC-A6 of 8.6 — "consent rides the EXISTING JSON-RPC field" — is satisfied unchanged).

## Acceptance Criteria

> **AC numbering**: AC1–AC4 are the functional core; AC5 vocabulary hygiene; AC6 reference-Spirit + smoke; AC7 backward-compat & no-regression; AC8 placement/ABI/discipline gates. Every AC is BDD-shaped and independently verifiable.

### AC1 — Enforcement matches the per-frame fine-grained intent, with documented band fallback

**Given** a cross-Host frame whose `consent_envelope.intent_class` is `Some(A2AIntent("diagnosis-handoff:read-only-evidence"))`
**When** the router runs send-allowlist (outbound, `route_outbound`) and accept-allowlist (inbound, `handle_intake`) enforcement
**Then** the consent-match key is the frame's **fine-grained `A2AIntent`** (`consent_envelope.intent_class`), matched case-insensitively against `send_allowlist` / `accept_allowlist`; an allowlist containing `A2AIntent::new("diagnosis-handoff:read-only-evidence")` **admits** the frame, and one that does not (e.g. holds only `"rca-summary"`) **denies** it with `EIntentDenied` whose `intent` field carries the **literal fine-grained string** (`"diagnosis-handoff:read-only-evidence"`), NOT a band token
**And** when `consent_envelope` is `None` OR `consent_envelope.intent_class` is `None` (same-Host frames, or cross-Host frames that declare no fine-grained intent), enforcement **falls back** to the existing 3-band `IntentClass` projection (`frame.intent.a2a_consent_intent_str()`) — so today's `"readonly"`/`"standard"`/`"highprivilege"` allowlists keep working byte-for-byte (interlocks AC7)
**And** the single decision point is one helper (rename/replace `frame_intent_str` → e.g. `consent_match_key(frame) -> String`) used by BOTH `send_admits` and `accept_admits`, so send and accept can never diverge on which key they match
**And** the precedence rule (fine-grained-when-present, band-otherwise) is documented in a doc-comment on that helper, citing ADR-012

### AC2 — Senders populate the fine-grained intent; the discard at the `From` conversion is removed

**Given** a Spirit (or the smoke driver) declares a fine-grained outbound intent
**When** the outbound frame is constructed / routed via `route_outbound`
**Then** `consent_envelope.intent_class` carries the declared `A2AIntent` end-to-end (sender → wire → receiver intake) with NO collapse to a band; the `A2AConsentEnvelope::From<ConsentEnvelope>` conversion (`consent.rs:31-41`) **no longer silently defaults a `None` intent to `"standard"`** — it preserves the distinction (`Option<A2AIntent>` faithfully, or the band fallback of AC1) so the projected envelope cannot misreport a frame's intent
**And** a test asserts a round-tripped frame's `consent_envelope.intent_class` is **byte-equal** to what the sender set (no normalization beyond documented case-folding), proving population is real, not aspirational

### AC3 — Confused-deputy negative is closed at the fine-grained granularity (ADR-012's worked example)

**Given** Mira (Host A) and Nash (Host B) with Nash's `accept_allowlist = [A2AIntent("diagnosis-handoff:read-only-evidence")]` (and NOT `"code-mutation-directive"`)
**When** Mira emits a frame carrying `intent_class = Some("diagnosis-handoff:read-only-evidence")` — **admitted**; AND a second frame carrying `intent_class = Some("code-mutation-directive")` — **denied**
**Then** the admitted frame reaches Nash's intake and the denied frame is rejected with `EIntentDenied`/`CODE_INTENT_DENIED` (`-32001`) whose payload names `"code-mutation-directive"`, observable in the Transparency Log
**And** this is the literal ADR-012 rationale ("Mira's `diagnosis-handoff:read-only-evidence` is admissible at Nash; `code-mutation-directive` is rejected") now passing as an executable test — NOT realized as the `readonly` band (which would admit BOTH, reopening the confused-deputy gap)

### AC4 — Defense-in-depth holds at the fine-grained granularity (send-side AND accept-side)

**Given** the same fine-grained intent
**When** a frame is denied
**Then** BOTH directions enforce independently on the fine-grained key: a send-side denial yields `A2AError::IntentDenied { direction: Send, .. }` before the frame hits the wire (sender refuses to emit an intent its OWN `send_allowlist` forbids), and an accept-side denial yields the `CODE_INTENT_DENIED` NACK → `A2AError::IntentDeniedAtPeer` on the sender (`adapter.rs` `route_outbound` step 1 + `handle_intake` step "ADR-012 accept-allowlist check"); the existing `EIntentDenied { peer, intent, direction: Send|Accept }` and `AllowlistDirection` discrimination is preserved, now carrying fine-grained `intent` strings

### AC5 — Vocabulary hygiene: silent-never-match becomes loud (the root complaint)

**Given** the original failure mode was "an operator writes an intent that *silently* never matches"
**When** the consent layer is exercised
**Then** an `A2AIntent` is given a **canonical-form check** (a non-breaking `A2AIntent::parse`/`is_canonical` helper validating shape `^[a-z0-9]+(-[a-z0-9]+)*(:[a-z0-9]+(-[a-z0-9]+)*)?$` — lowercase, `namespace:verb` optional, bounded length; this is the de-facto shape already used: `diagnosis-handoff:read-only-evidence`, `rca-summary`, `code-mutation-directive`) — **existing free-form `A2AIntent::new` stays for back-compat**, the canonical helper is additive
**And** a denial emits enough structured context (the fine-grained `intent` + `direction` + `peer`, already in `EIntentDenied`) that an operator can see *which* declared intent was rejected — denials are **fail-closed AND legible**, never silent
**And** [FORK — see Open Questions] an OPTIONAL unreachable-entry diagnostic: a debug-assertion or `tracing::warn!` when an allowlist holds an intent that is neither a canonical fine-grained intent nor one of the 3 band tokens (catches the exact `smoke-a2a-loopback-6-3` typo class at author time) — scope this only if it stays inside the LOC/no-new-public-API budget

### AC6 — Reference-Spirit wiring + runnable headline reflect fine-grained consent

**Given** the aspirational fine-grained allowlists in `smoke-a2a-loopback-6-3` and Mira/Nash
**When** the smoke / pairing is run
**Then** Mira's advisory carries `consent_envelope.intent_class = Some(A2AIntent("diagnosis-handoff:read-only-evidence"))` (replacing the `ADVISORY_CONSENT_INTENT = "readonly"` band reliance — `spirits/mira/src/lib.rs`), Nash's `accept_allowlist` admits exactly that, and the smoke's deliberate denied frame uses a fine-grained `"code-mutation-directive"` (NOT a band) that is rejected with `EIntentDenied` visible in the TL
**And** a runnable headline (extend `smoke-a2a-loopback-6-3`, OR a new `smoke-a2a-consent-vocab-8-7` one-shot in `maos-bin` mirroring the 6.3/8.4/8.5 precedent — `maos-bin` smoke is NOT kernel KLOC) exits `0` and demonstrates: one fine-grained-admitted frame delivered + one fine-grained-denied frame logged — the observable end-to-end demo `[[feedback_lunarpulse_observability_preference]]`
**And** the `cross_host_consent_v1.rs` aspirational specific-intent assertions (`adapter.rs` matching) are now **truthful** — either updated in place or mirrored in a `cross_host_consent_v1_5.rs` with fine-grained scenarios (send-denied / both-admit / accept-mismatch at the fine-grained granularity)

### AC7 — Zero regression: all existing band-based behavior preserved

**Given** Stories 6.3 and 8.5 ship band-based consent (`"readonly"`/`"standard"`) and a full `cross_host_consent_v1.rs` suite
**When** 8.7 lands
**Then** every existing A2A/consent test passes **unchanged** (the band fallback of AC1 guarantees frames without a fine-grained `intent_class` behave exactly as before); the `smoke-mira-nash-8-5` and `smoke-a2a-loopback-6-3` arms still exit `0`; `IntentClass`, its `a2a_consent_intent_str()`, and the 3 bands are **not removed or renamed** (they remain the fallback + the manifest-declared coarse tier)
**And** the public signatures `ConsentAllowlists::send_admits(&A2AIntent)` / `accept_admits(&A2AIntent)` and the `EIntentDenied` / `AllowlistDirection` / `A2AError::{IntentDenied,IntentDeniedAtPeer}` types are preserved (changes are confined to the **private** `LoopbackA2ARouter` enforcement helpers + the `From` conversion + additive `A2AIntent` canonical helper)

### AC8 — Placement, ABI, kernel-KLOC, workspace, and discipline gates

**Given** the authoritative scope says "against `maos-a2a-core`, after 8.6" (epic-8 §AC-A6 Noted-gap; `sprint-status.yaml:90`)
**When** 8.7 lands
**Then** the enforcement change lands in **`maos-a2a-core`** (the crate 8.6 extracts that owns `handle_intake` + its consent types + the router substrate per 8.6 §AC-A1) — NOT in the over-budget `maos-a2a` (the `kloc-check` for `maos-a2a` and `maos-a2a-core` both stay GREEN; record post-change line counts in evidence)
**And** `maos-kernel-core` is **byte-identical** to its pre-story state (zero-kernel-KLOC mandate, as 8.4 proved with 15505 / 8.5 / 8.6 §AC-A7); the kernel-KLOC sentinel is GREEN
**And** the workspace member count is **UNCHANGED** (8.7 adds NO new crate — it is a pure logic refinement); `check-workspace-count` stays at whatever 8.6 set
**And** an `abi-diff` of `maos-a2a-core`'s public surface is **Added-only or unchanged** (no Removed/Modified on the public consent surface — the new `A2AIntent` canonical helper is Added; private helper changes are invisible to abi-diff); the 8.6-frozen `verify_pinned` / `handle_intake` / `try_from_bytes` signatures are **untouched** (interlocks 8.6 §AC-A6)
**And** all discipline gates are GREEN **at HEAD** (not flipped-while-red); `dev_model_used` is recorded (§A2 discipline); `4-kernel-design.md` / `7-inter-agent-communication.md` are reconciled if any narrative says consent is 3-band-only

## Tasks / Subtasks

- [ ] **Task 0 — Confirm sequencing & placement (BLOCKING — do FIRST)** (AC: #8)
  - [ ] Verify `maos-a2a-core` exists with the consent substrate moved (`handle_intake`, `ConsentAllowlists`, `frame_intent_str`/`send_admits`/`accept_admits`, `EIntentDenied`). **If 8.6 is not yet landed, STOP** and resolve the Open-Question fork (build 8.6 first — default — or take the documented `maos-a2a` fallback of Decision D).
  - [ ] Locate the moved enforcement helpers in `maos-a2a-core` (post-8.6 file:line) and the `A2AConsentEnvelope::From` conversion.
- [ ] **Task 1 — Replace the enforcement key** (AC: #1, #4)
  - [ ] Rename/replace `frame_intent_str(frame)` → `consent_match_key(frame) -> String`: prefer `frame.consent_envelope.as_ref().and_then(|e| e.intent_class.as_ref()).map(|i| i.as_str().to_string())`; fall back to `frame.intent.a2a_consent_intent_str().to_string()` when `None`.
  - [ ] Point BOTH `send_admits` and `accept_admits` at the single helper; keep case-insensitive match. Doc-comment the precedence rule citing ADR-012.
  - [ ] Verify `route_outbound` (send-side, step 1) and `handle_intake` (accept-side check) both flow through it; `EIntentDenied.intent` now carries the fine-grained string.
- [ ] **Task 2 — Populate the per-frame intent; fix the `From` discard** (AC: #2)
  - [ ] Ensure outbound frame construction carries `consent_envelope.intent_class = Some(declared_intent)` end-to-end; add a sender-side helper if the Spirit API needs one.
  - [ ] Remove the `unwrap_or_else(|| A2AIntent::new("standard"))` collapse in `A2AConsentEnvelope::From` (`consent.rs:37`); preserve the `Option`/band-fallback distinction instead of inventing `"standard"`.
  - [ ] Round-trip test: sent `intent_class` == received `intent_class`, byte-equal.
- [ ] **Task 3 — Vocabulary hygiene** (AC: #5)
  - [ ] Add additive `A2AIntent::parse`/`is_canonical` (canonical-form regex; keep `A2AIntent::new` free-form for back-compat) in `maos-domain/src/invariants/i8.rs`.
  - [ ] (Fork) Optional unreachable-allowlist-entry `tracing::warn!`/debug-assert — only if within budget; otherwise defer and note.
- [ ] **Task 4 — Reference Spirits + smoke + tests** (AC: #3, #6, #7)
  - [ ] Mira emits fine-grained `"diagnosis-handoff:read-only-evidence"`; Nash `accept_allowlist` admits it; add the `"code-mutation-directive"` confused-deputy negative.
  - [ ] Update/extend `smoke-a2a-loopback-6-3` (or new `smoke-a2a-consent-vocab-8-7`) in `maos-bin`: one fine-grained admit + one fine-grained deny, exits 0, denial in TL.
  - [ ] Make `cross_host_consent_v1.rs` aspirational specific-intent assertions truthful (update in place or add `cross_host_consent_v1_5.rs`).
  - [ ] Add positive/negative fine-grained tests; confirm band-fallback tests untouched.
- [ ] **Task 5 — Gates & reconciliation** (AC: #7, #8)
  - [ ] `kloc-check` GREEN for `maos-a2a` + `maos-a2a-core`; record counts. `maos-kernel-core` byte-identical. `check-workspace-count` unchanged.
  - [ ] `abi-diff` `maos-a2a-core` Added-only/unchanged; 8.6-frozen signatures untouched.
  - [ ] Run FULL existing A2A/consent/smoke suite — all GREEN at HEAD. Record `dev_model_used`.
  - [ ] Reconcile any `4-kernel-design.md` / `7-inter-agent-communication.md` narrative that asserts 3-band-only consent.

## Dev Notes

### The exact code change (file:line — pre-8.6 home; rebase onto `maos-a2a-core` after extraction)

| Symbol | Today (pre-8.6) | Type / role | 8.7 change |
|---|---|---|---|
| `frame_intent_str` | `crates/maos-a2a/src/adapter.rs:144-146` | **private** fn on `LoopbackA2ARouter` | replace body → prefer `consent_envelope.intent_class`, fall back to band |
| `send_admits` / `accept_admits` | `adapter.rs:149-164` | private fns | route through new helper (no signature change) |
| `route_outbound` send-check | `adapter.rs:255-272` | step (1) ADR-012 send-allowlist | now denies on fine-grained key |
| `handle_intake` accept-check | `adapter.rs:425-436` | ADR-012 accept-allowlist | now denies on fine-grained key |
| `A2AConsentEnvelope::From` | `crates/maos-a2a/src/consent.rs:31-41` | `From<ConsentEnvelope>` | remove `"standard"` discard at line 37 |
| `ConsentAllowlists{send,accept}_allowlist` | `consent.rs:49-57` | `Vec<A2AIntent>` | **unchanged** (already fine-grained-capable) |
| `ConsentEnvelope.intent_class` | `crates/maos-domain/src/frame.rs:412` | `Option<A2AIntent>`, `#[serde(default)]` | **unchanged** (the lever — just consult + populate it) |
| `A2AIntent` | `crates/maos-domain/src/invariants/i8.rs:32-44` | `String` newtype | **add** `parse`/`is_canonical` (additive) |
| `IntentClass::a2a_consent_intent_str` | `crates/maos-domain/src/invariants/i1.rs:139-149` | 3-band projection | **unchanged** (now the fallback) |
| `EIntentDenied{peer,intent,direction}` | `consent.rs:69-84` | rejection struct | **unchanged** shape; `intent` now fine-grained |

**Critical reuse — do NOT reinvent:** the per-frame fine-grained intent field (`ConsentEnvelope.intent_class`), the allowlist types (`ConsentAllowlists`), the rejection type (`EIntentDenied`), the error variants (`A2AError::{IntentDenied, IntentDeniedAtPeer}`), the NACK code (`CODE_INTENT_DENIED = -32001`), and the defense-in-depth send+accept structure **all already exist and are correct**. This story is a ~30–60 line behavioral correction at the enforcement decision point + sender population + tests, NOT new substrate. The single most important line is `adapter.rs:145` (`frame.intent.a2a_consent_intent_str()` → consult `consent_envelope.intent_class` first).

### Why ABI-neutral (and why AC-A6 of 8.6 is not violated)

8.6's §AC-A6 froze `verify_pinned` / `handle_intake` / `try_from_bytes` and said "consent rides the EXISTING JSON-RPC field, NOT a new TCP-specific field" — and that **a consent-fn *signature* change is a RED flag DURING 8.6**. 8.7 honors all of that: it changes the **private** enforcement *body* (not signatures), consults an **existing** wire field (`consent_envelope.intent_class`, already serialized in the JSON-RPC frame), and adds only an **additive** `A2AIntent` canonical helper. 8.7 is precisely the "future consent-vocabulary story… *after* 8.6 lands the seam, against `maos-a2a-core`" that the Noted-gap authorizes — the one moment it is legitimate to reopen consent enforcement.

### Decisions (defaults chosen; forks flagged for the user at the end)

- **Decision A — Use the existing `ConsentEnvelope.intent_class` as the per-frame intent carrier.** (vs. widening `IntentClass` enum — ABI-breaking on a frozen 3-variant enum; vs. adding a new `IacFrame` field — redundant, the envelope field already exists and is documented for exactly this.) Confident default.
- **Decision B — Fine-grained-when-present, 3-band-fallback-otherwise.** A frame that declares no fine-grained intent keeps the coarse band gate (backward-compat for all 6.3/8.5 tests). A frame that declares one is gated on it (strictly tightens). This is additive and non-breaking. **FORK:** the stricter alternative is **fail-closed-for-cross-Host** (a `CrossHost` frame with no `intent_class` is DENIED) — maximal confused-deputy closure but breaks existing band-only cross-Host tests unless migrated. See Open Questions Q2.
- **Decision C — Keep `A2AIntent` free-form; add an *additive* canonical-form helper.** Avoids an enum/registry that would re-collapse the open vocabulary ADR-012 deliberately chose ("what would force a revisit: intent-class cardinality grows pathologically" — i.e. open vocabulary IS the design). The canonical helper makes typos catchable without removing flexibility. **FORK:** whether to also add the unreachable-entry warning (AC5 last bullet) — Open Questions Q3.
- **Decision D — Placement is `maos-a2a-core`, gated on 8.6 (default).** Fallback if the team chooses to build 8.7 *before* 8.6: land the identical change in today's `crates/maos-a2a/src/adapter.rs` + `consent.rs`; 8.6's later extraction carries it into `maos-a2a-core` unchanged (it's the same code moving crates). This trades the authoritative "after 8.6" sequencing for earlier delivery and adds a few lines to the already-over-budget `maos-a2a` until 8.6 relieves it. **The default remains: 8.6 first.** Open Questions Q1.

### Source tree — what to touch

- `crates/maos-a2a-core/src/…` (post-8.6) — the enforcement helpers + `From` conversion + `ConsentAllowlists` (moved from `maos-a2a` by 8.6 §AC-A1). **Primary edit site.**
- `crates/maos-domain/src/invariants/i8.rs` — additive `A2AIntent::parse`/`is_canonical`.
- `crates/maos-domain/src/frame.rs` — `ConsentEnvelope.intent_class` is the carrier; **no change** expected (read-only reference).
- `spirits/mira/src/lib.rs`, `spirits/nash/src/lib.rs` — emit/accept fine-grained intents; replace `ADVISORY_CONSENT_INTENT = "readonly"` band reliance.
- `crates/maos-bin/src/main.rs` — `smoke-a2a-loopback-6-3` (~lines 4576-4810) update or new `smoke-a2a-consent-vocab-8-7` arm; wire into `discipline.yml` like 8.4/8.5 smokes. **Not kernel KLOC.**
- `crates/maos-a2a*/tests/cross_host_consent_v1.rs` (and/or new `_v1_5.rs`) — make aspirational fine-grained assertions truthful.
- `xtask/kloc.toml` — verify `maos-a2a` (1500) + new `maos-a2a-core` ceilings stay GREEN; no bump.

### Testing standards

- Unit: positive (fine-grained admit), negative (fine-grained deny → `EIntentDenied` carrying the literal string), fallback (no `intent_class` → band gate unchanged), round-trip (sent==received `intent_class`), defense-in-depth (send-side AND accept-side deny). Mirror the structure of `cross_host_consent_v1.rs` scenarios 3.1/3.2/3.3.
- Integration/smoke: `maos-bin` one-shot exits 0, denial observable in Transparency Log (the §`feedback_lunarpulse_observability_preference` runnable demo).
- Regression: FULL existing A2A + consent + `smoke-mira-nash-8-5` + `smoke-a2a-loopback-6-3` suite GREEN at HEAD.
- Determinism: no `SystemTime::now()` in consent decisions; no new flakes.

### Previous-story intelligence (8.1–8.6 patterns to reuse)

- **In-proc bridge pattern (8.1–8.5):** reference Spirits consume kernel/A2A substrate as **dev-deps**; Spirit-side code is a pure lib. Mira/Nash already do this — extend, don't restructure.
- **`register_spirit_typed` handle must be bound or the mailbox closes (`ChannelClosed`)** — Story 8.4 lesson `[[project_story_8_4_landed]]`. Applies if you add any new wiring in the smoke.
- **`abi-diff` needs the `--base` flag** — Story 8.3 lesson `[[project_story_8_3_landed]]`; no-base mode false-positives. Use `--base` when checking `maos-a2a-core`.
- **NEVER `cargo fmt -p crate` here** — Story 7.5a lesson; whole-crate collateral. Format only touched files.
- **`kloc-check` is pre-existing RED on some crates** — verify your change is *neutral* (8.3/8.4/8.5 precedent), don't try to fix unrelated reds.
- **Discipline gates flip GREEN at HEAD, never flipped-while-red** — recurring Epic-4/5/7 "AC4 trap" `[[feedback_mechanical_gates_compound_promises_decay]]`.
- **Loopback peer lookup keys `HostId == peer_id`** — Story 8.5 lesson `[[project_story_8_5_landed]]`; relevant if the smoke adds peers.

### Project Structure Notes

- **Planning-artifact gap to flag:** Story 8.7 currently exists ONLY as (a) the epic-8 §AC-A6 Noted-gap paragraph and (b) the `sprint-status.yaml:90` backlog comment. There is **no formal `## Story 8.7` section in `epic-8-…miranash-v03-v15.md`, `epics/index.md`, or `epics/dependency-dag.md`** (unlike 8.6, which got a full `sprint-change-proposal-2026-06-04.md` Direct-Adjustment). Recommend a `bmad-correct-course` Direct Adjustment to register 8.7 in the three source artifacts (mirroring the 8.6 proposal) so tracker and source-of-truth agree. This story file is authored against the Noted-gap as the authoritative scope.
- **Dependency direction:** 8.7 → 8.6 (needs `maos-a2a-core`) → {8.5 loopback pair, 6.3 A2A mesh}. 8.7 is a v1.5+ refinement, last in the Epic-8 chain.

### References

- [Source: epics/epic-8-…miranash-v03-v15.md#AC-A6 Noted-gap (Winston, 2026-06-04)] — the authoritative scope for this story; the "silently never match" diagnosis and the "after 8.6, against maos-a2a-core" mandate.
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml:90] — 8.7 backlog comment restating the gap + DEPENDS-ON-8.6.
- [Source: docs/adr/ADR-012-typed-intent-a2a-consent.md] — the decision: consent is `(peer-identity, intent-class)`; `diagnosis-handoff:read-only-evidence` admissible, `code-mutation-directive` rejected; open vocabulary by design.
- [Source: architecture-maos-minimal-opus/7-inter-agent-communication.md:52,19] — per-frame typed-intent consent; `consent_envelope.intent_class` on the wire.
- [Source: crates/maos-a2a/src/adapter.rs:141-164,255-272,425-436] — the enforcement path (the bug + the two call sites).
- [Source: crates/maos-a2a/src/consent.rs:13-84] — `A2AConsentEnvelope` (`From` discard at :37), `ConsentAllowlists`, `EIntentDenied`, `AllowlistDirection`.
- [Source: crates/maos-domain/src/frame.rs:407-421] — `ConsentEnvelope.intent_class: Option<A2AIntent>` (the lever).
- [Source: crates/maos-domain/src/invariants/i1.rs:120-149] — `IntentClass` + `a2a_consent_intent_str()` (the 3-band fallback).
- [Source: crates/maos-domain/src/invariants/i8.rs:32-44] — `A2AIntent` newtype.
- [Source: crates/maos-bin/src/main.rs:4576-4810,4617-4632] — `smoke-a2a-loopback-6-3` + its aspirational fine-grained allowlists.
- [Source: crates/maos-a2a/tests/cross_host_consent_v1.rs:75,100] — aspirational specific-intent assertions to make truthful.
- [Source: _bmad-output/implementation-artifacts/8-5-…miranash….md:95,348,456] — Story 8.5 Dev-Agent-Record documentation of this gap + the deferred review item.
- [Source: epics/epic-8-…#AC-A1/AC-A7 (Story 8.6)] — `maos-a2a-core` extraction (moves `handle_intake`+consent substrate); zero-kernel-KLOC; abi-diff Added-only discipline.
- [Source: epics/dependency-dag.md:53-56] — Epic-8 / 8.6 dependency arrows.

## Dev Agent Record

### Agent Model Used

_Recommended: `claude-opus-4-8` (consistent with 8.1–8.5; this is a small-surface, high-precision security-semantics change where correctness of the enforcement decision-point dominates — favor the strongest model over throughput; the deepseek async/integration weaknesses `[[feedback_deepseek_v4_pro_patterns]]` are not the risk here, but the precision bar is)._

### Debug Log References

### Completion Notes List

### File List

---

## ⚠️ Open Questions for the User (resolve before `dev-story`)

These are genuine planning/design decisions the spec encoded with a default; confirm or override.

**Q1 — Sequencing (the blocker).** Default = **build Story 8.6 first** (it creates `maos-a2a-core`; 8.7 lands there). Alternatives: (a) confirm 8.6 first (default); (b) build 8.7 NOW against today's `maos-a2a` (Decision D fallback — earlier delivery, a few lines added to the over-budget crate until 8.6 relieves it); (c) build 8.6 and 8.7 together. This changes where every file goes.

**Q2 — Fallback semantics (Decision B).** Default = **fine-grained-when-present, 3-band-fallback-otherwise** (additive, zero-regression). Stricter alternative = **fail-closed-for-cross-Host** (a `CrossHost` frame with no fine-grained `intent_class` is DENIED — maximal security, but requires migrating existing band-only cross-Host tests). Which?

**Q3 — Vocabulary strictness (Decision C / AC5).** Default = **additive canonical helper + keep free-form `A2AIntent::new`**, plus an OPTIONAL `tracing::warn!` on unreachable allowlist entries. Confirm: (a) helper-only; (b) helper + unreachable-entry warning; (c) go further (a declared-intent manifest registry that rejects unknown intents at admission — larger scope, possible follow-up story).

**Q4 — Planning-artifact registration.** Story 8.7 is not yet a formal section in the epic-8 markdown / `index.md` / `dependency-dag.md` (only the Noted-gap + sprint-status comment). Want a `bmad-correct-course` Direct Adjustment to register it (mirroring the 8.6 `sprint-change-proposal`) so source-of-truth and tracker agree?
