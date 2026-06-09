# Story 8.13.1: Genuine Cross-Host Consent-Denial → ConsentRupture over Live TCP

Status: review

## Story

As a v1.5 operator running Mira (Host A) and Nash (Host B) over the live mTLS wire,
I want a **deliberate cross-host consent denial** to **produce a real `ConsentRupture` audit record through production code** — not a row the test hand-writes —
so that the J4 incident journey's consent-denial beat is genuinely observable in the transparency log when it crosses two real endpoints, and so we know whether the cross-host deny path journals ruptures at all.

## Acceptance Criteria

1. **AC1 — Design fork resolved + recorded.** The fork above is resolved by team consensus (party-mode; Winston owns the architecture call, Murat the test-fidelity bar, security view on the deny semantics). The ruling (A/B/C or a synthesis) and rationale are recorded in this story's Dev Agent Record and, if it changes behavior, flagged for John (PRD/AC wording) and Winston (architecture).

2. **AC2 — The faked rows are DELETED.** The hand-inserted `ConsentRupture` (and the `ConsentRequest` advisory stand-in, if it too is a fake) are removed from `smoke-mira-nash-tcp-8-13` ([main.rs:~6163-6179]). If the ruling is Option A/B, the corresponding `smoke-mira-nash-8-5` hand-insert ([main.rs:~5459]) is also removed and re-pointed at the real path (consistency: do not leave one smoke lying while the other tells the truth).

3. **AC3 — The rupture is EARNED over the live TCP wire (Murat's bar).** A real fine-grained typed-intent A2A frame is driven Mira→Nash over the live `TcpA2ATransport` whose intent **classifies** (passes the `-32009 CODE_CONSENT_UNCLASSIFIED` gate) and is then **policy-denied** (distinct from `-32007` peer-binding and from the `-32009` unclassified leg). The **consent DECISION may be fixtured** (a test-only consent policy / allowlist that denies the chosen band is acceptable — the consent algorithm is unit-covered elsewhere). The **`ConsentRupture` record must be produced by PRODUCTION code**, not the test. The smoke queries the transparency log (or observes the returned rupture frame) and asserts the rupture was written **by the real intake/deny path**, car…

4. **AC4 — No regression on the 8.13 guarantees.** The `-32007` confused-deputy negative stays clean (a matched peer is used for the deny path; identity-mismatch still rejects with `-32007` and never enters intake). The P4 receiver-side wire-content oracle, the real HTTP push assertions, the no-egress guard, and `XDG_DATA_HOME` isolation all remain green. `smoke-mira-nash-tcp-8-13` and `smoke-mira-nash-8-5` both exit 0.

5. **AC5 — Red-first (TEA discipline).** Author the assertion-on-the-real-row **before** wiring the production emission (Option A/B) — prove it FAILS against today's NACK-only path (the row is absent without the fix), then make it pass. If the ruling is Option C, the red phase is: assert the real `-32001`/audit signal, prove the old hand-insert assertion was vacuous, then delete it. Capture the red→green transition in the Debug Log.

6. **AC6 — Charter discipline (carry-forward from 8.13).** `maos-kernel-core/src` **byte-identical** (`git diff --stat` empty). Do **NOT** edit `maos-a2a` (over ceiling) or `maos-a2a-tcp` (over ceiling) — `maos-a2a-core` is the editable seam for Option A. Workspace stays **43**. `abi-diff --base abi-baseline/v1-pre-bump.txt` **Added-only**. `kloc-check` aggregate-RED is pre-existing/neutral; cross no new per-crate ceiling. `cargo fmt -p <crate>` banned (7.5a whole-crate collateral). If Option A's emission would push `maos-a2a-core` over a per-crate ceiling, that is a scope signal — surface it, do not bump silently.

## DESIGN FORK (already resolved — Option A, unanimous 4/4 party-mode)

- **Option A — Receiver-side TL journaling.** On classified-policy-deny, `handle_intake_verified` writes a `ConsentRupture` transparency-log row (and/or returns a typed `ConsentRupture` frame to the sender) before/with the NACK, mirroring the 6.4 `iac_bus` rupture-frame semantics.
- **Option B — Sender-side journaling.** The sender writes the `ConsentRupture` row on receiving `IntentDeniedAtPeer`.
- **Option C — Re-ground AC4.** A `-32001` NACK + existing A2A audit trail is sufficient; `ConsentRupture` is same-host-only.

**Ruling: Option A (receiver-side).** Key rationale: receiver's deny path IS reached by real inbound handling; sender-side would fire only from smoke/test code. Receiver's rupture carries policy reason (unforgeable-by-content).

## Dev Agent Record Summary

- **T1 — Design fork resolved:** Option A, unanimous 4/4.
- **T2 — Red-first assertion:** Authored `intake_policy_deny_emits_consent_rupture` + negative control; ran RED before wiring emission.
- **T3 — Implement Option A:** Additive `rupture_sink` + `install_rupture_sink` + `emit_consent_rupture` in `maos-a2a-core` `handle_intake` policy-deny branch. Both hand-inserts DELETED.
- **T4 — Real TCP end-to-end:** `smoke-mira-nash-tcp-8-13` drives denied frame over live `TcpA2ATransport`, observes production rupture.
- **T5 — Consistency + discipline:** `smoke-mira-nash-8-5` re-pointed. Kernel byte-identical, abi-diff Added-only, workspace 43. maos-a2a-core kloc 3041→3144 surfaced + documented.
