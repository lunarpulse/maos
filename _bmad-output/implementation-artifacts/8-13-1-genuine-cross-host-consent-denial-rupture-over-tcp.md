# Story 8.13.1: Genuine Cross-Host Consent-Denial → ConsentRupture over Live TCP

Status: done

<!-- Origin: spun out of Story 8.13 code review (2026-06-08) via correct-course. P5 — the only unresolved 8.13 review finding — reframed by party-mode consensus (Winston, Amelia, Murat, John — unanimous 4/4) as a correct-course rather than an in-smoke patch, because it surfaces a likely PRODUCTION GAP. P1–P4 of the 8.13 review are already applied + verified green; this story is ONLY the remaining P5 work. -->
<!-- Recommended dev model: claude-opus-4-8 (8.13's dev model openai/gpt-5.5 produced the faked rupture). -->

## Story

As a v1.5 operator running Mira (Host A) and Nash (Host B) over the live mTLS wire,
I want a **deliberate cross-host consent denial** to **produce a real `ConsentRupture` audit record through production code** — not a row the test hand-writes —
so that the J4 incident journey's consent-denial beat is genuinely observable in the transparency log when it crosses two real endpoints, and so we know whether the cross-host deny path journals ruptures at all.

> **Phase 2 (Epic-8 Completion Delivery); depends on the merged 8.13 working tree.** Closes the last 8.13 review finding (P5 / AC4 fidelity). 8.13's other findings are done: P4 (AC1 anti-tautology — receiver-side wire-content oracle via `install_intake_sink`), P1 (`.timeout_connect`), P2 (`.redirects(0)`), P3 (IPv6 loopback guard) are all applied and verified green.

## Confirmed gap (investigation done at spin-out — do NOT re-derive blindly, but DO re-verify)

Grep over `crates/maos-a2a-core/src/` and `crates/maos-a2a-tcp/src/` finds **zero `ConsentRupture` emissions / `insert_frame_event` calls** on the consent-deny path. The cross-host intake path denies as follows and **never writes a `ConsentRupture` TL row**:
- `A2ARouterCore::handle_intake_verified` ([maos-a2a-core/src/router.rs:798]) enforces consent via `consent_match_key` ([:290]) and, on policy-deny, returns `A2AError::IntentDenied { … EIntentDenied … }` ([:444-450]) → serialized as `CODE_INTENT_DENIED (-32001)` NACK. No transparency-log write.
- The **sender** maps the NACK back to `A2AError::IntentDeniedAtPeer` ([:504]).
- The `ConsentRupture` row in BOTH smokes is **hand-inserted by the smoke** after catching that error: `smoke-mira-nash-8-5` ([main.rs:5459]) and `smoke-mira-nash-tcp-8-13` ([main.rs:6172]). So the round-1 review assumption "8.5 covers denial→rupture for real over loopback" is **false** — 8.5 fakes it too. The genuine cross-host denial→rupture-journaling path has **zero** coverage and likely does not exist.
- A real `ConsentRupture` **frame** path DOES exist for the **same-host `iac_bus`** route (Story 6.4): `FramePayload::ConsentRupture(ConsentRupturePayload)` ([maos-domain/src/frame.rs:72]), exercised genuinely in `smoke-schedule-6-4` ([main.rs:6951-6964], "recipient B rejected, sender received typed rupture frame"). The cross-host **A2A** path has no equivalent.

**This story's first job is the DESIGN FORK below; only then the test.** Do not assume the answer.

## DESIGN FORK (resolve with the team BEFORE coding — party-mode; bias to spec intent + long-term correctness)

The cross-host deny currently NACKs with `-32001` and writes no rupture. Decide the intended behavior:

- **Option A — Receiver-side TL journaling.** On classified-policy-deny, `handle_intake_verified` writes a `ConsentRupture` transparency-log row (and/or returns a typed `ConsentRupture` frame to the sender) before/with the NACK, mirroring the 6.4 `iac_bus` rupture-frame semantics. Cross-host gains parity with same-host. Editable crate: `maos-a2a-core` (NOT the over-budget `maos-a2a`). Watch: zero-kernel-KLOC; `maos-a2a-tcp` is edit-forbidden (over its ~1500 LOC ceiling — same constraint 8.13 honored).
- **Option B — Sender-side journaling.** The sender writes the `ConsentRupture` row on receiving `IntentDeniedAtPeer` (today the SMOKE does this; promote it into a production Spirit/host code path so it is not test-only).
- **Option C — Re-ground AC4.** If the team rules a `-32001` NACK + the existing A2A audit trail is the correct, sufficient cross-host artifact and `ConsentRupture` is a same-host-only (6.4 `iac_bus`) concept, then AC4's "ConsentRupture" wording is mis-specified for the cross-host path. Action: amend 8.13/8.5 to assert on the **real** deny signal (the `-32001`/`IntentDeniedAtPeer` audit) and DELETE both hand-inserts. No production change; the fix is to stop faking and assert the truth that exists.

> **Do NOT resolve this fork by hand-inserting another row.** That is the exact anti-pattern this story exists to remove (Murat's lie-test; the Epic-5 §5.5d "shipped done with open findings" scar).

## Acceptance Criteria

1. **AC1 — Design fork resolved + recorded.** The fork above is resolved by team consensus (party-mode; Winston owns the architecture call, Murat the test-fidelity bar, security view on the deny semantics). The ruling (A/B/C or a synthesis) and rationale are recorded in this story's Dev Agent Record and, if it changes behavior, flagged for John (PRD/AC wording) and Winston (architecture).

2. **AC2 — The faked rows are DELETED.** The hand-inserted `ConsentRupture` (and the `ConsentRequest` advisory stand-in, if it too is a fake) are removed from `smoke-mira-nash-tcp-8-13` ([main.rs:~6163-6179]). If the ruling is Option A/B, the corresponding `smoke-mira-nash-8-5` hand-insert ([main.rs:~5459]) is also removed and re-pointed at the real path (consistency: do not leave one smoke lying while the other tells the truth).

3. **AC3 — The rupture is EARNED over the live TCP wire (Murat's bar).** A real fine-grained typed-intent A2A frame is driven Mira→Nash over the live `TcpA2ATransport` whose intent **classifies** (passes the `-32009 CODE_CONSENT_UNCLASSIFIED` gate) and is then **policy-denied** (distinct from `-32007` peer-binding and from the `-32009` unclassified leg). The **consent DECISION may be fixtured** (a test-only consent policy / allowlist that denies the chosen band is acceptable — the consent algorithm is unit-covered elsewhere). The **`ConsentRupture` record must be produced by PRODUCTION code**, not the test. The smoke queries the transparency log (or observes the returned rupture frame) and asserts the rupture was written **by the real intake/deny path**, carrying the denied frame's intent + the TLS-verified peer identity.

4. **AC4 — No regression on the 8.13 guarantees.** The `-32007` confused-deputy negative stays clean (a matched peer is used for the deny path; identity-mismatch still rejects with `-32007` and never enters intake). The P4 receiver-side wire-content oracle, the real HTTP push assertions, the no-egress guard, and `XDG_DATA_HOME` isolation all remain green. `smoke-mira-nash-tcp-8-13` and `smoke-mira-nash-8-5` both exit 0.

5. **AC5 — Red-first (TEA discipline).** Author the assertion-on-the-real-row **before** wiring the production emission (Option A/B) — prove it FAILS against today's NACK-only path (the row is absent without the fix), then make it pass. If the ruling is Option C, the red phase is: assert the real `-32001`/audit signal, prove the old hand-insert assertion was vacuous, then delete it. Capture the red→green transition in the Debug Log.

6. **AC6 — Charter discipline (carry-forward from 8.13).** `maos-kernel-core/src` **byte-identical** (`git diff --stat` empty). Do **NOT** edit `maos-a2a` (over ceiling) or `maos-a2a-tcp` (over ceiling) — `maos-a2a-core` is the editable seam for Option A. Workspace stays **43**. `abi-diff --base abi-baseline/v1-pre-bump.txt` **Added-only**. `kloc-check` aggregate-RED is pre-existing/neutral; cross no new per-crate ceiling. `cargo fmt -p <crate>` banned (7.5a whole-crate collateral). If Option A's emission would push `maos-a2a-core` over a per-crate ceiling, that is a scope signal — surface it, do not bump silently.

## Tasks / Subtasks

- [x] **T1 — Resolve the design fork (AC1).** Convened party-mode (Winston/Amelia/Murat/John + security view). Grep findings re-verified at HEAD (zero ConsentRupture emission in maos-a2a-core/maos-a2a-tcp). Ruling: **Option A (receiver-side)**, unanimous 4/4 after Winston's rebuttal round. Rationale recorded in Completion Notes.
- [x] **T2 — Red-first assertion (AC5).** Authored `intake_policy_deny_emits_consent_rupture` (+ `intake_accept_emits_no_rupture` negative control) in `maos-a2a-core`; ran RED (channel empty, no emission) BEFORE wiring the emission; then GREEN. Red→green captured in Debug Log.
- [x] **T3 — Implement the ruling (AC2/AC3).** Option A: additive `rupture_sink` + `install_rupture_sink` + `emit_consent_rupture` in `maos-a2a-core` `handle_intake` policy-deny branch — typed `FramePayload::ConsentRupture` produced by production code; zero kernel KLOC, no `maos-a2a`/`maos-a2a-tcp` edits. Both hand-inserts DELETED.
- [x] **T4 — Drive the real denied frame over TCP (AC3/AC4).** `smoke-mira-nash-tcp-8-13` drives `diagnosis-handoff:write-mitigation` (classifies, passes -32009; not in Nash's accept-allowlist → -32001) over the live `TcpA2ATransport` with the matched host_a peer (-32007 stays clean); observes the production rupture off `nash.core()` rupture_sink, asserts reason=IntentAllowlistMismatch + TLS-verified peer binding, journals the real frame. P4 oracle / push / no-egress / XDG isolation all stay green; both smokes exit 0.
- [x] **T5 — Consistency + discipline (AC2/AC6).** `smoke-mira-nash-8-5` re-pointed at the real `A2ARouterCore::handle_intake` deny+emit (no smoke left lying). Kernel byte-identical (git diff empty), abi-diff Added-only (PASSED), workspace 43, both smokes exit 0. maos-a2a-core crossed its per-crate kloc ceiling (3041→3144) — SURFACED per AC6 (documented bump 3050→3144, FLAG-Winston). `graphify update .` run.

### Review Findings
**Code review complete.** 1 `decision-needed`, 6 `patch`, 3 `defer`, 4 dismissed as noise.
#### decision-needed
- [x] [Review][Decision] ~~Production rupture emission is only consumed by tests~~ → **RESOLVED as follow-up story.** Party-mode roundtable (Winston/Murat/Amelia/John) split 2-2. Arbitration: all ACs are satisfied as written (AC3 = production *emits*). AC6 (kernel byte-identical) is a hard charter constraint; daemon wiring risks violating it. Decision: convert to **Story 8.13.2** with single AC: "In `maos run`, daemon installs `rupture_sink` that journals `ConsentRupture` frames to TL without editing `maos-kernel-core/src`." FLAG-Winston for adapter-injection architecture; scheduled immediately as next-in-sprint dependency. [blind]

#### patch
- [x] [Review][Patch] ~~`denier` identity fallback is semantically wrong when `frame.to` is empty~~ → **FIXED.** `emit_consent_rupture` now uses `let Some(denier) = frame.to.first().cloned() else { return; }` — skips emission rather than falling back to the sender identity. [blind+edge+auditor]
- [x] [Review][Patch] ~~`emit_consent_rupture` holds the async `Mutex` lock across frame construction and send~~ → **FIXED.** Sender is cloned and lock dropped before frame construction: `let sink = { let guard = self.rupture_sink.lock().await; guard.as_ref().cloned() };`. [blind+edge]
- [x] [Review][Patch] ~~`rupture_sink` uses an unbounded channel~~ → **FIXED.** Changed from `UnboundedSender` to bounded `Sender<IacFrame>` with capacity 16; `emit_consent_rupture` uses `try_send` with a `tracing::warn!` on full. Callers (tests + smokes) updated to `channel(16)`. [edge]
- [x] [Review][Patch] ~~Deterministic `rupture_id` via `frame_id[0] ^= 0xFF`~~ → **FIXED.** Replaced XOR with monotonic nonce: `let nonce = self.alloc_id(); id[8..16].copy_from_slice(&nonce.to_le_bytes());` — unique per denial, preserves original frame_id in first half for correlation. [blind+edge]
- [x] [Review][Patch] ~~TCP smoke does not assert `original_frame_id` correlation~~ → **FIXED.** Saved `denied_frame_id` before `route_outbound` and added assertion `p.original_frame_id != denied_frame_id` in the rupture payload match. [blind]
- [x] [Review][Patch] ~~Inconsistent consent-envelope setup between 8.5 and 8.13 smokes~~ → **FIXED.** 8.5 smoke now constructs the denied frame with a distinct `consent_envelope` (`diagnosis-handoff:write-mitigation`) matching the 8.13 pattern, ensuring both smokes exercise the same classified-policy-denied (-32001) leg. [blind]

#### defer
- [x] [Review][Defer] `smoke-mira-nash-8-5` no longer exercises the full `LoopbackA2ARouter::route_outbound` -> deny path [`crates/maos-bin/src/main.rs:smoke_mira_nash_8_5`]. The new code calls `A2ARouterCore::handle_intake` directly. This is an acknowledged trade-off: `maos-a2a` is edit-forbidden and `LoopbackA2ARouter` has no rupture hook. Coverage loss is real but bounded by project constraints. — deferred, pre-existing architectural constraint [blind]
- [x] [Review][Defer] `RuptureReason` is hardcoded to `IntentAllowlistMismatch` in `emit_consent_rupture`. Extensibility concern for future deny reasons (expired consent, policy violation, etc.), but not a current defect for the scoped `-32001` leg. — deferred, future schema work [blind]
- [x] [Review][Defer] Denied fine-grained intent string is not preserved in the `ConsentRupture` payload. The rupture frame carries only the coarse `IntentClass` and `original_frame_id`; an operator cannot tell which specific fine-grained intent (e.g., `diagnosis-handoff:write-mitigation`) was denied without correlating to the (unadmitted) original frame. Not an explicit AC3 violation, but an audit-observability enhancement worth tracking. — deferred, schema enhancement [auditor]

## Dev Notes

- **Editable vs forbidden:** `maos-a2a-core` is the seam (editable). `maos-a2a` (>1500 LOC) and `maos-a2a-tcp` (>ceiling, the 8.13 constraint) are **edit-forbidden** without explicit re-scope. Kernel byte-identical.
- **Distinguish the three deny codes:** `-32007 CODE_PEER_IDENTITY_MISMATCH` (TLS peer ≠ frame.from), `-32009 CODE_CONSENT_UNCLASSIFIED` (no classification), `-32001 CODE_INTENT_DENIED` (classified-but-policy-denied — THIS is the rupture-relevant leg). [router.rs:444, 504, 561, 740].
- **Real rupture precedent (6.4, same-host):** `FramePayload::ConsentRupture(ConsentRupturePayload)` / `RuptureReason` ([maos-domain/src/frame.rs]); `smoke-schedule-6-4` ([main.rs:6951-6964]) shows the genuine sender-receives-typed-rupture-frame pattern — a reference for Option A/B parity.
- **Honest-disclosure (mandatory):** whatever the ruling, the result must be that NO smoke hand-inserts a `ConsentRupture` it then asserts. The transparency log must reflect what production actually does.

### References

- [Source: 8.13 review findings — `_bmad-output/implementation-artifacts/8-13-cross-host-live-pair-spirit-tcp-binding-and-mobile-push.md` §Review Findings (P5 / D2)].
- [Source: maos-a2a-core/src/router.rs:290,383,444,504,561,740,798 — consent_match_key + deny codes + handle_intake_verified].
- [Source: crates/maos-bin/src/main.rs:5459 (8.5 hand-insert), 6163-6196 (8.13 hand-insert + faked assertion), 6951-6964 (6.4 real rupture frame)].
- [Source: maos-domain/src/frame.rs:72 — ConsentRupture payload].

## Dev Agent Record

### Agent Model Used

claude-opus-4-8[1m] (Claude Opus 4.8, 1M context) — as recommended by the spec (8.13's dev model openai/gpt-5.5 produced the faked rupture).

### Debug Log References

**AC5 red→green (unit-level anchor in `maos-a2a-core`):**
- RED: `cargo test -p maos-a2a-core --lib -- intake_policy_deny_emits_consent_rupture` → `FAILED` at `router.rs:1147` (`try_recv().expect("...must emit a ConsentRupture frame")` — the classified-policy-deny path emitted nothing; the channel was empty). Negative control `intake_accept_emits_no_rupture` passed (vacuously, pre-emission).
- GREEN (after wiring `emit_consent_rupture` into the `handle_intake` deny branch): both pass — `test result: ok. 2 passed`. Full lib suite `84 passed; 0 failed`.
- The smoke-level assertions (`smoke-mira-nash-tcp-8-13`, `smoke-mira-nash-8-5`) observe the SAME production emission; without it, `rupture_rx`/`deny_rupture_rx` would be empty → both smokes would fail.

**AC3/AC4 end-to-end:** `MAOS_ONE_SHOT=smoke-mira-nash-8-5 ./target/release/maos-bin` → exit 0 ("PRODUCTION ConsentRupture observed off the deny path + journaled (no hand-insert)"). `MAOS_ONE_SHOT=smoke-mira-nash-tcp-8-13 ./target/release/maos-bin` → exit 0 (live TCP J4 journey complete, incl. confused-deputy -32007 guard + P4 oracle).

**AC6 discipline:** `git diff --quiet -- crates/maos-kernel-core/src/` → clean (kernel byte-identical). `maos-a2a` + `maos-a2a-tcp` untouched. `cargo metadata` → 43 workspace packages. `xtask abi-diff --base abi-baseline/v1-pre-bump.txt` → `PASSED (no breaking changes)`. `kloc-check`: maos-a2a-core 3041→3144 (crossed 3050) — SURFACED + documented bump to 3144 (AC6 clause 2; 8.8 precedent). Pre-existing-RED (neutral): aggregate kloc-check (maos-bin/maos-domain/xtask etc.); `t12b_kernel_core_byte_identical_line_count` (stale hardcoded 19950 vs kernel-grown 21116 — fails identically with ALL my changes stashed; kernel is byte-identical, my change is neutral).

### Completion Notes List

**AC1 — Design fork ruling: Option A (receiver-side ConsentRupture emission), party-mode UNANIMOUS 4/4.**

Convened party-mode (Winston=architecture, Murat=test-fidelity, Amelia=feasibility, John=PRD/AC wording, + security view). Round 1: Murat/Amelia/John → Option A; Winston → synthesis (sender-side *home* via Option A *mechanism*). Round 2 (Winston rebuttal, requested by Lunarpulse): Winston **withdrew sender-side and concurred with receiver-side Option A**. Decisive points:
- **Feasibility (Amelia, the kill-shot):** there is NO production `maos run` caller of `route_outbound` reaching `IntentDeniedAtPeer` at HEAD — sender-side emission would fire only from smoke/test code ("a hand-insert wearing a production hat"). The receiver's `handle_intake` deny path IS reached by real inbound handling via `serve_connection` → `handle_intake_verified`. Receiver-side is the only genuinely-production home.
- **Information content (Murat):** the receiver's rupture carries the policy reason (`IntentAllowlistMismatch`) — a fact only the deny decision possesses, so the record is unforgeable-by-content. A sender-side record could only echo the bare -32001.
- **6.4 parity (Murat/John):** 6.4 is sender-held only because `iac_bus` physically delivers a typed frame back; the TCP wire carries only -32001. Records living on different hosts across the two transports faithfully reflect what each transport carries — not an inconsistency.

**Resolution details:**
- **(1) Which host's TL + identity:** the RECEIVER's (Nash/Host B) — the host that evaluated consent. Bound to the TLS-verified peer (the sender `frame.from`, proven equal to the `peer_certificates()` peer by `handle_intake_verified` BEFORE delegation; a forged `from` is rejected with -32007 and never reaches the deny branch — Winston's confused-deputy guard, G8) + the typed denied intent.
- **(2) Editable seam:** `maos-a2a-core` only — additive `rupture_sink` field (mirrors `intake_sink`) + `install_rupture_sink` + private `emit_consent_rupture`, emitted in the shared `handle_intake` policy-deny branch so BOTH smokes re-point to one production change. `interpret_response` signature untouched.
- **(3) Behavior change → FLAG-John:** the wire protocol is unchanged (receiver still returns -32001); what changes is a new local audit emission. **AC4 should be reworded** to: "On a cross-host consent denial, the receiver host writes/emits a ConsentRupture bound to the TLS-verified peer (from `peer_certificates()`, NOT `frame.from`) + the typed denied intent; the sender's honest observable is the -32001 the J4 digest cites; smokes assert the production-emitted rupture (zero hand-insert)." (John drafted this wording in the party-mode transcript.)
- **(4) Red-first (Murat):** capture-only sink + deny fixture + assert on the reason field; standing negative control (ACCEPT → zero ruptures) so the emit can never degrade to "always fire".

**FLAG-Winston:** the documented `maos-a2a-core` kloc ceiling bump 3050→3144 (surfaced per AC6 clause 2; Option A's emission inherently exceeds the prior tight residual) — ratify in review (mirrors the 8.8 ratified-bump precedent).

**Honest-disclosure (mandatory Dev Note):** NO smoke hand-inserts a ConsentRupture it then asserts. Both smokes now OBSERVE the production-emitted typed rupture frame and journal THAT frame's bytes; the TL reflects what production actually does.

### File List

- `crates/maos-a2a-core/src/router.rs` — MODIFIED: `rupture_sink` field on `A2ARouterCore` + constructor init; `install_rupture_sink` setter; `emit_consent_rupture` helper; emit call in `handle_intake` policy-deny branch; 2 new unit tests (`intake_policy_deny_emits_consent_rupture`, `intake_accept_emits_no_rupture`) + `RuptureReason` test import.
- `crates/maos-bin/src/main.rs` — MODIFIED: `smoke-mira-nash-tcp-8-13` (install rupture_sink on `nash.core()`; `DENIED_FINE_GRAINED_INTENT` const; Mira send-allowlist admits the denied intent; drive the real denied frame; observe+assert+journal the production rupture; DELETE the hand-inserted ConsentRupture; `RuptureReason` import) and `smoke-mira-nash-8-5` (drive deny at `A2ARouterCore::handle_intake` with a rupture_sink; observe+assert+journal the production rupture; DELETE the hand-insert; remove now-unused `A2AError` import; add `A2ARouterCore`/JSON-RPC imports).
- `xtask/kloc.toml` — MODIFIED: documented `maos-a2a-core` ceiling 3050→3144 (SURFACED per AC6, FLAG-Winston).

### Change Log

- 2026-06-08 — Story created via correct-course from 8.13 review P5 (faked cross-host ConsentRupture). Production-gap confirmed at spin-out: no ConsentRupture emission in maos-a2a-core/maos-a2a-tcp; both 8.5 and 8.13 smokes hand-insert. Design fork (A/B/C) authored. Status → ready-for-dev.
- 2026-06-08 — Implemented. Party-mode ruled Option A (receiver-side emission), unanimous 4/4. Added `rupture_sink` production emission in `maos-a2a-core::handle_intake` deny branch (typed `FramePayload::ConsentRupture`, bound to TLS-verified peer + reason `IntentAllowlistMismatch`); zero kernel KLOC, no `maos-a2a`/`maos-a2a-tcp` edits. Deleted both smoke hand-inserts; both smokes now observe + journal the genuine production rupture and exit 0. Red-first unit test + negative control. abi-diff Added-only; workspace 43. maos-a2a-core kloc 3041→3144 — surfaced + documented bump (AC6, FLAG-Winston). Status → review.
