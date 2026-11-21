---
story_id: '21.7'
story_key: 21-7-production-consent-granter-and-gate
created: 2026-11-20
status: in-progress
context_commit: ea8c51b6
baseline_commit: ea8c51b6
security_sensitive: true
kernel_grant: "Forecast only; no prototype or grant requested. Kernel 0 net tokei under every option of D-21-7-01…05 (contingent 0…+5, method below). ADR-069 kfree applies inside the envelope; measure at landing, re-pin with scripts/repin.py and write the HISTORY row in the same commit if any kernel file changes. No alarm forecast."
kloc_grant: "Read-the-files forecast, not measured growth or permission: maos-bin +90…+260; maos-iac +30…+110; maos-manifest +40…+130 (D-21-7-01 A; 0 under B); maos-spirit-abi +1…+6; maos-domain 0…+12 and maos-wasm-host 0…+8 (D-21-7-02 A only); xtask 0…+30. Re-measure after the D-rows are ruled."
review: "DIFFICULT: security-critical cross-Spirit disclosure control. Follow docs/testing/proof-contract.md and D-E20-6; the trust-boundary classification is D-21-7-06. Story creation anthropic/claude-opus-5-5 (DraftProductionConsent). The read-only static reviewer self-reported GPT-family; the eligible non-GPT security review is still owed."
---

# Story 21.7: Production consent granter and gate

Status: in-progress

**Readiness:** operator rulings were recorded on 2026-11-21 UTC. Implementation is in progress on `ga-21-7-2026-11-21` with local evidence only; this update does not change the tracker. The ruled schema bump remains unallocated. The separate eligible D-E20-6 security review and 21-7's own executable proof remain required for landing.

**GA-required, size M.** Authority: `planning-artifacts/epics/epic-21-multi-host-on-live-substrate-w6.md:154-176`; SCP 2026-11-12 §5.B2/3.3 and §5.B2/3.5 (`planning-artifacts/sprint-change-proposal-2026-11-12.md:177-192`) with K5 `:488`; SCP 2026-11-13 §2 (`sprint-change-proposal-2026-11-13.md:51,67`); tracker `implementation-artifacts/sprint-status.yaml:419,508`; `RELEASE-HOLDS.md:114,133`. Paths under `planning-artifacts/` and `implementation-artifacts/` are relative to `_bmad-output/`.

## Story

As **an operator running the shipped MAOS daemon without Postgres**,
I want **each Spirit's declared recall restriction stamped on the frames it sends, a production consent gate on local delivery, retractions reaching every recipient who received the frame, and Spirit-origin bus sends never journaled as root**,
so that **FR29's "honors consent envelopes" holds in the product, not only on frames the test suites build. The release claims no Postgres consent survival and no legal meaning for "consent".**

**Exact scope:** AC1–AC7. FR29 production half only: what 17-8 labelled under rule 11 (`implementation-artifacts/17-8-participant-recall-and-consent-envelopes.md:114`, `:45`), and DW:1329/1330.

Out of scope:
- The Postgres leg: DW:1328, the J3/Reza-on-live-substrate assertion and the nightly Reza scene stay parked under `v25-reza-postgres-legs` (AC5).
- Cross-host retract.
- Guest-set recall policy: the WIT envelope stays elided (D-17-8-01 A).
- A2A intake changes.
- Expiring local grants.
- Any change to cross-wall emitter scope (ADR-058).

## Inventory at base — source measurements, not execution

Read at `ea8c51b6` (`/home/lunarpulse/dev_ws-ga-wasm` main base, `git show` only; the in-progress 21-9 working tree was not used as a source). That base is **not** an ancestor of `dev_ws-course28` HEAD `de16fa0a`: the two trees differ in `maos-bin/src/main.rs` (+73 lines at the base), so `main.rs` cites do not transfer to `de16fa0a`. Nothing was compiled, tested, gated, formatted, prototyped or reproduced.

| Surface | State at base | Preserve / required change |
|---|---|---|
| Gate seam | `ConsentGate` trait `crates/maos-iac/src/adapter/mailbox.rs:54-56`; set-once `OnceLock` `:155`; `with_consent_gate :225`, `install_consent_gate :242`. `prepare_at :332-391`: the gate runs once per address before the journal write; telemetry, `KERNEL_SENDER_SPIRIT_ID` (`"__kernel"`, `:61`) and depth ≥2 bypass it (`:344-346`). With no gate, every address is accepted (`:360`). | Install through `install_consent_gate` (set-once). Verdicts stay computed once in `prepare` and consumed by `deliver_prepared` (17-8 AC2.4). Do not evaluate the gate a second time. Keep the quarantine/rupture path and the depth guard. |
| Production composition | `crates/maos-bin/src/main.rs:2758-2763`: the comment says "operator-supplied gates can be installed"; only `install_transparency_log` is called. The recipient resolver is installed once at `:2730-2732` over `pid_by_spirit_id` (`:2722-2723`). The only `with_consent_gate` call is the `smoke_schedule_6_4` one-shot arm (`:14733`, private mailbox, `AlwaysRejectB` `:14750-14763`). | Install production granter and gate here, beside the TL installer. A failed install is a boot error, like the resolver's. One policy source; no second pid map. |
| No production granter | Guest sends: `spirit_session.rs:1298` sets `frame.consent_envelope = None` before `deliver_typed(frame, pid, send_token)` `:1305`. WIT lift always yields `recall: Full` (`crates/maos-wasm-host/src/frame_bridge.rs:574-586`). `ConsentRecall` is daemon-side (`crates/maos-domain/src/frame.rs:458-500`; ADR-012 implementation note `docs/adr/ADR-012-typed-intent-a2a-consent.md:22-24`). The manifest has no consent declaration (`crates/maos-manifest/src/manifest.rs`; only `log.recall` `:625`). | The granter must be daemon-side and bound to the authenticated sender (granter = sender, 17-8 CR-2). Guest-supplied envelopes stay discarded. The policy source is D-21-7-01. |
| Persistence | Side tables `transparency_log_recipients` / `_consent` / `_consent_ruptures` live in the SQLite Transparency Log (`crates/maos-iac/src/adapter/transparency_log.rs:592-617`). They are written with the routed row (`SideRows::Routed` `:410-450`, `insert_routed_frame :1293-1325`). Recall predicates are at `:372-387`. | Unchanged. No new table is needed under the recommended options. These tables are the subject of the parked PG migration (DW:1328). |
| Postgres-free path | `MAOS_LOOM_POSTGRES` / `MAOS_LOOM_HOME_TEAM` are the only Postgres env-contract names (`crates/maos-bin/src/env_contract.rs:477,482`). When unset, boot prints "collective tier (Loom-lite) not configured" (`main.rs:2370-2383,2553-2557`). `MAOS_CROSS_TEAM_SHARE_PEER` requires Postgres (`:13132`). `maos-iac` has no Postgres code. | AC1 proves this arm was actually taken, not assumed. The epic's conditional AC5 D-row ("no Postgres-free proof possible") is **not triggered** on source evidence: a Postgres-free path exists. This stays `[INFERENCE]` until the AC1 witness runs. |
| Retract routing | `retract()` `crates/maos-iac/src/adapter.rs:750`. `to` is built from `original_entry.to_spirit_id` (`:855-879`); the base row stores only the first addressee (`transparency_log.rs:1291,1306`). Legacy empty → route to the sender. The retract goes through `prepare` → **the gate** (`adapter.rs:905-907`) and `deliver_with_overtake` (`:947`; `mailbox.rs:679-687`). It is journaled at the original sender's pid (`adapter.rs:917,929`). No source reader returns the recorded recipients for one frame (only the recall predicate `:373`). | Policy is D-21-7-03. Keep sender-only authority (`:784-791`), idempotent `Already`, one id per operation (17-8 CR-4), the halt on a kernel-minted duplicate, the legacy sender fallback and lineage continuity. |
| Native send attribution | `impl IacBusPort for IacBusAdapter` `mailbox.rs:767`: `deliver` `:782-803` calls `deliver_typed(frame, 0, None)` (`:798`). The trait is frozen (`crates/maos-domain/src/ports/iac_bus.rs:22-64`; ADR-010 comment `mailbox.rs:783-785`). **Re-measured premise:** `IacBusPort::deliver` has **no non-test caller** in `crates/` or `spirits/`. The compiled-in `Ctx` has no send primitive (`crates/maos-spirit-abi/src/ctx.rs:58-133`). Today pid 0 is reachable only through the port. | Policy is D-21-7-04. Do not change the trait signature. Leave kernel/root emitters at pid 0: `inference/mod.rs:204` (`__kernel`), `scheduler/hook_dispatch.rs:242` (`"kernel"` `:211`), session seed `spirit_session.rs:559`, orphan `supervision.rs:2771`, host-B intake/outcome `delegation.rs:719,788`. DW:1220 is a separate row owned by 20-3c. |
| Non-Spirit senders under a gate | Senders other than `__kernel` that a gate would see: `"operator"` (`installed_spirits.rs:158`), `"kernel"` budget frames (`hook_dispatch.rs:211`) and peer-intake frames. | With the recommended rule (accept-all for a sender that has no consent rows) these keep today's behaviour. Admission name reservation is not widened here. |
| Admission lifecycle | `pid_by_spirit_id` insert `crates/maos-bin/src/admission.rs:1531`; removal `:1580,1631,1648` via `remove_pid_by_spirit_id :1804`. | Consent policy registration follows the same lifetime: registered on admit, removed at every removal site, and replaced by the successor's own declaration on a swap. No stale grant may outlive its incarnation. |
| Feature shape | `spirit_session` / `spawned_lifecycle` are `cfg(all(network, wasm-host))` (`crates/maos-bin/src/lib.rs:90-93`); `wasm-host` is off by default at base (`crates/maos-bin/Cargo.toml:15,34`). | The shipped default artifact can send Spirit-to-Spirit frames only after 21-9 AC1 lands. See Dependencies. |

## Decisions — prior authority and recorded operator rulings

### Already ruled; do not ask again

| Decision | Binding disposition |
|---|---|
| `ga3` (`decisions-2026-11-12-dedrift.md:28`), B3 (`SCP 2026-11-12:33`) | 21-7 is a `v1.0.0` GA precondition and is not postponed. |
| B2 / D-21-PF-02 C (`SCP 2026-11-12:171-174`; `epic-21-preflight-2026-11-12.md:261`) | No 21-2 edge. 21-2 is not GA-transitive. |
| K5 + SCP 2026-11-13 §2 (`SCP 2026-11-12:488`; `SCP 2026-11-13:51,67`) | GA clause = Postgres-free binary proof. DW:1328 and the Reza-on-Postgres leg are parked under `v25-reza-postgres-legs` (not 21-2). DW:1329/1330 stay here. |
| R4 (Epic-17 retro, 2026-10-24) | Received ACs: all-addressee retract (DW:1329) and native pid attribution (DW:1330). |
| D-17-8-01 A / -02 / -03 A, CR-2 | Recall policy is daemon-side and absent from WIT. `Full` is the default. Grant-scoped rupture rows. Consent binds recipients only, never the sender. |
| `kfree` / ADR-069 amendment (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:74-90`) | In-envelope kernel growth is self-service. No FLAG-Winston ask; the old "◆ measured grant clause" has no ask semantics. |
| Ordering (`SCP 2026-11-12:443`, §10 item 7) | 21-9 goes first; the other GA stories follow in parallel. The epic's "Dependencies: none" is corrected below. |

### D-rows — approved by the operator on 2026-11-21 UTC

| ID | Choice and bounded options | Original recommendation and evidence | Operator ruling |
|---|---|---|---|
| **D-21-7-01** | **Policy source for granter and gate.** **A** A version-governed manifest `[consent]` declaration on the sending Spirit: per local recipient spirit id, a `recall` of `full` / `header_only` / `withheld`. Admission validates it (unknown id/policy refused typed). Schema bump under existing discipline. **B** An operator-owned host config file keyed (sender, recipient) → recall, loaded at boot and validated against admitted ids. **C** A as the grant, plus B as an operator overlay that can only narrow (Full → HeaderOnly → Withheld), never widen. | **A.** The granter is the sender (CR-2; `ConsentEnvelope::granted_by`, `frame.rs:525-527`). The epic's AC1 has "one [Spirit] grants". The manifest is the admitted, signed declaration of a Spirit's authority. B makes the operator the granter, which contradicts granter = sender. C adds a second source and a merge rule. Cost of A: schema bump in `maos-manifest` / `maos-spirit-abi` (`MANIFEST_SCHEMA_VERSION = 7`, `crates/maos-spirit-abi/src/lib.rs:126`, at the draft baseline). 21-9's D-21-9-02 A also adds a version-governed manifest declaration and touches `manifest.rs`. | **A — APPROVED.** Schema 8 remains 21-9's. No version is allocated to 21-7 here; its next manifest version requires a fresh post-21-9 baseline and ratification. |
| **D-21-7-02** | **Production gate rule and rupture reason.** A and B keep accept-all for a sender with no consent rows and refuse an uncovered addressee once rows exist. **A** Rejection carries a new typed `RuptureReason::ConsentNotGranted`: domain enum, WIT variant, minor WIT bump with an older-guest mapping in `frame_bridge.rs:208-232`. **B** Reuse `IntentAllowlistMismatch`; no ABI change; document the label. **C** No new consent-based delivery refusal: the gate only refuses unloaded recipients (`RecipientUnloaded`) and the policy acts at recall alone. | **A.** `frame.rs:409-416` records the rule this codebase applied when it added `PeerIdentityUnverified`: do not file a failure under an adjacent reason, because "a claim standing in for a control" misleads audits. `IntentAllowlistMismatch` means the **receiver's** intent allowlist (`:398-399`), and local sends are all forced to `Standard` (`spirit_session.rs:1295`). B is cheapest but mislabels. C leaves the gate unable to refuse delivery, which is weaker than the epic's AC2. | **A — APPROVED.** |
| **D-21-7-03** | **Retract audience (DW:1329) and gate interaction.** **A** Route to the original frame's **recorded recipients** (`transparency_log_recipients`, gate-accepted local incarnations), delivered only to a recorded pid that is still the admitted incarnation. Retract frames bypass the consent gate, as telemetry does. Remote and gate-rejected addressees never received the payload, and the TL records that. **B** Persist every original addressee (a new side table or column) and route to all of them. **C** A's audience, but retracts pass through the gate like any frame. | **A.** The side table already holds exactly who received the frame (17-8 CR-1, `transparency_log.rs:320-326,410-428`). A needs one new read by frame id and no schema change. B adds a TL schema change that the parked PG migration (DW:1328) would also have to carry, and it notifies parties who never received the payload. C lets a policy change between send and retract silence a retraction, which leaves disclosure in place. A reads "every original addressee" as every addressee that received the frame: a narrowing of the received AC's literal wording, so the operator must rule it. | **A — APPROVED.** |
| **D-21-7-04** | **Native bus-send attribution (DW:1330).** **A** The unbound adapter's `IacBusPort::deliver` refuses any non-kernel sender with a typed `IacBusError`. A per-admitted-Spirit **bound** port (pid and spirit id fixed at admission, `from` overwritten) becomes the only Spirit-attributed native path. **B** Resolve `frame.from.spirit_id` through the installed admitted-incarnation resolver, journal that pid, and refuse an unresolved sender. **C** Refuse only: non-kernel senders are refused typed, and the record states that compiled-in Spirits have no bus-send primitive at GA. No bound type until a native send API exists. | **C.** Measured: `IacBusPort::deliver` has no production caller and `Ctx` has no send method, so A adds a type with no production caller. B derives a pid from a frame claim, which 17-8's identity rule forbids ("never a pid from the frame"). C closes the only path that could journal a Spirit's frame at pid 0, at the lowest cost. C reinterprets "attributed to the admitted Spirit's pid" as "never unattributed", so the operator must rule. Choose A if a native send API is planned before GA. | **C — APPROVED.** |
| **D-21-7-05** | **AC1 witness Spirits and job.** **A** Two installed signed `wasm-component` Spirits built from the existing `recall-probe` fixture (sends `spirit:peer`, recall/fetch; `crates/maos-bin/tests/participant_recall_17_8.rs:42,323-355`), run by the built `maos` binary. Enrolled in an existing T2 cgroup-lane job (`wasm-host-tests`); no new job. **B** One native Researcher recipient plus one WASM sender (both forms, as in the 17-8 vectors). **C** A new dedicated fixture guest pair. | **A.** It reuses the probe, which already sends, recalls and fetches, and the binary-harness pattern (`installed_spirit_dispatch_20_1.rs:7-62`, `CARGO_BIN_EXE_maos`). B: a native recipient's recall works, but native Spirits cannot **send** (D-21-7-04), so the second grant direction would need a WASM sender anyway. C is new fixture surface for no added coverage. | **A — APPROVED.** |
| **D-21-7-06** | **Trust-boundary classification and reviewer eligibility (D-E20-6).** **A** New trust boundary: the landing waits for a non-GPT eligible security review (GLM-family when available, or `anthropic/claude-fable-5-1` with the operator's prior permission per `proof-contract.md:58-61`). **B** Tightening of the existing IAC boundary: the standing GPT-family security review, disclosed on every record. | **A.** D-E20-6 names "21-7's consent gate" (`SCP 2026-11-12:82`). This story adds a new admission-declared policy input (D-21-7-01 A) and the first production delivery-refusal control. D-E20-6's quota premise was dated "until 2026-11-15" (`epic-20-retro-2026-11-12.md:264`). Verify current GLM availability at implementation; do not infer it here. The difficult tier applies either way (Blind/Edge/Acceptance on opus-5.5, Test Infrastructure on glm-5.3; `proof-contract.md:68-73`). | **A — APPROVED.** The eligible security review is separate from the difficult-tier review and 21-7's own executable proof. No reviewer selected or review performed by this record. |

No kernel permission, surface-hash permission or STOP exception is requested. No forecast crosses 22,000, so no ENVELOPE ALARM row is owed.

## Acceptance Criteria

These are defensive controls and observable product behaviour. This story authorizes no exploit construction or attack replication.

1. **AC1 — binary-level, no Postgres (hosted-proof AC).** A test built against the shipped default `maos`:
   - Starts it with `MAOS_LOOM_POSTGRES`, `MAOS_LOOM_HOME_TEAM` and `MAOS_CROSS_TEAM_SHARE_PEER` unset and no database service. It asserts the boot line "collective tier (Loom-lite) not configured" (`main.rs:2553-2557`), so the Postgres-free arm is proven, not assumed.
   - Admits two Spirits (D-21-7-05). Spirit A's declared consent grants B `header_only`; B's grants A `withheld`.
   - Each sends one frame to the other. The recipient's `log.recall` / `log.fetch` returns header-only (`payload-available = false`, fetch `consent-withheld`) and `consent-withheld`, matching the suite vector `header_only_including_oversized_and_withheld_envelopes_both_forms` (`participant_recall_17_8.rs:1098`).
   - A third, uncovered addressee is refused by the gate: it is never a recorded recipient, the sender receives the typed rupture, and the third Spirit's fetch reports `scope-violation`, as in V14 (`crates/maos-iac/tests/participant_recall_17_8.rs:533`).
   - No withheld payload byte appears in any recipient-visible output or recipient-pid row.
   - Bind the evidence to binary SHA, tree, env and job.
   - **Proven red:** removing the daemon's gate install reds the uncovered-addressee assertion (the epic's plant). Removing granter stamping reds the header-only/withheld assertions. That second plant is named because gate removal cannot falsify the granter (`proof-contract.md:28`).
2. **AC2 — production granter and gate installed.**
   - The composition root (`main.rs:2758-2763`) installs exactly once one production granter and one `ConsentGate` over one shared policy source (D-21-7-01). A failed install is a boot error.
   - The granter stamps a daemon-minted envelope on an admitted Spirit's outbound frame, in place of `spirit_session.rs:1298`'s `None`, before `deliver_typed`, so the envelope commits in the routed transaction. The envelope has: granter = the authenticated sender; `recall` from the declaration for that addressee; a restart-unique `consent_id` (not `with_fine_grained_intent`'s process counter, `frame.rs:540-560`); `intent_class: None` (same host); `valid_until_ns: None`; and timestamp from `consent_wall_now_ns`.
   - Guest-supplied envelopes stay discarded.
   - The gate follows the ruled D-21-7-02 rule. Accept-all applies only to a sender with no consent rows.
   - Policy registration follows the `pid_by_spirit_id` lifetime (`admission.rs:1531,1580,1631,1648,1804`), including swap successors and unload.
3. **AC3 — retract reaches its audience (DW:1329)** as ruled in D-21-7-03, from a TL read of the original frame's recorded recipients. Preserve sender-only authority, `Already` idempotency, one id per operation, lineage continuity, the legacy fallback, and the journaling pid. A multi-recipient vector proves every recorded recipient, not only the first, receives the Retract. Proven red: restore first-recipient routing.
4. **AC4 — no Spirit send journals at pid 0 (DW:1330)** as ruled in D-21-7-04. The `IacBusPort` signature is unchanged. Kernel/root emitters keep pid 0. Migrate test callers that pass non-kernel senders through the port. Proven red: restore the unconditional `deliver_typed(frame, 0, None)`.
5. **AC5 — Postgres leg parked, named residual.** This story does not migrate `transparency_log_recipients`, `transparency_log_consent` or `transparency_log_consent_ruptures`, and it asserts nothing about J3/Reza consent on the live substrate. The residual record names DW:1328, `v25-reza-postgres-legs` and `RELEASE-HOLDS.md:114` (boundary 3, unchanged). The conditional D-row of epic AC5 is not raised (Inventory: Postgres-free path). If the AC1 witness cannot run Postgres-free, stop and open it with options (A) narrow / (B) bring `v25-reza-postgres-legs` into GA. Never restore a 21-2 edge.
6. **AC6 — honest labels.**
   - Replace the rule-11 production-limit text with the shipped state, and nothing more, in: `docs/adr/ADR-012-typed-intent-a2a-consent.md:24`, `ADR-013-…:65`, `docs-site/docs/cookbook/wasm-component-spirit.md:106`, `docs-site/docs/manifest/v5.md:119` and both KO mirrors (`docs-site/i18n/ko/…/wasm-component-spirit.md:106`, `…/manifest/v5.md:119`). EN/KO TOML blocks stay byte-identical.
   - Document the D-21-7-01 declaration in the manifest reference.
   - State that "consent" here is a technical recall/delivery restriction. It is not legal consent, a GDPR lawful basis, or counsel-reviewed.
   - Keep: peer-identity ruptures carry no envelope; cross-wall recall stays emitter-scoped (ADR-058 D2).
7. **AC7 — preservation.** Unchanged:
   - log-before-deliver (I2);
   - gate-once verdicts;
   - quarantine row before the rupture frame;
   - rupture depth guard;
   - telemetry and `__kernel` bypass;
   - A2A intake checks (granter = sender, expiry; `consent_refusal_1b.rs` vectors);
   - sender-owned frames ignoring received consent (`participant_recall_17_8.rs:871`);
   - the earlier-boot out-of-scope rule;
   - independent TL readers over side tables (`tl_readers_side_tables_17_8.rs:82`);
   - the `smoke_schedule_6_4` arm.

## Tasks / Subtasks

- [ ] **T0 — apply rulings and refresh the baseline.** Apply the approved D-21-7-01…06 rulings. Rebase on landed 21-9 (default-on, schema 8) and re-measure every cite. Obtain fresh ratification of 21-7's manifest version on that baseline before allocating it. Apply D-E20-6 per D-21-7-06 before implementation review.
- [ ] **T1 — declaration (AC2).** Manifest consent section, validation and schema discipline (D-21-7-01). Admission registers and removes the policy at every `pid_by_spirit_id` site; successors re-derive it from their own manifest.
- [ ] **T2 — granter and gate (AC2).** One policy source. Stamping at the session send seam. A `ConsentGate` implementation in `maos-bin`, installed once at the composition root. The approved D-21-7-02 reason and its WIT/bridge mapping.
- [ ] **T3 — retract (AC3).** TL read of recorded recipients by frame id; audience and gate treatment per D-21-7-03.
- [ ] **T4 — attribution (AC4).** `mailbox.rs:782-803` per D-21-7-04; migrate port test callers.
- [ ] **T5 — witness (AC1).** Binary-level test per D-21-7-05 in an existing T2-lane job. The named plants run once on one image, with bundles under `implementation-artifacts/21-7-…-evidence/`.
- [ ] **T6 — docs and residual (AC5/6).** Labels, manifest docs, release-notes handoff text, and the parked-residual record. Do not edit `docs/release/`; that is the notes owner's job.
- [ ] **T7 — landing, orchestrator-owned.**
  - Run focused suites (`maos-iac`, `maos-bin`, `maos-manifest`, plus `maos-wasm-host` / `maos-domain` if touched). Widen to every manifest reader, `spirit_boundary_invariants` and `check-corpus` (`proof-contract.md:49-55`).
  - Run one final-tree matrix run.
  - Run the difficult-tier review and a separate D-E20-6 security review by a reviewer eligible under D-21-7-06; report the actual models. Neither review replaces 21-7's own executable proof.
  - Measure `check-kernel-baseline --json` / `kloc-check --json`. If a kernel file changed, re-pin with `scripts/repin.py` and write HISTORY in the same commit.
  - Only the orchestrator edits tracker, epic and DW status.

## Proof anchors — existing, no execution claimed

These anchors exist at the base; that does not mean they cover the new controls.

| Boundary | Anchor |
|---|---|
| Header-only / withheld outcome | `crates/maos-bin/tests/participant_recall_17_8.rs`: `header_only_including_oversized_and_withheld_envelopes_both_forms` `:1098`, `valid_consent_then_real_gate_rupture_withholds_same_frame_both_forms` `:1091`, `direct_recipient_ids_roles_dedup_multiaddress_and_gate_rejection_both_forms` `:1025`, `sender_owned_frames_ignore_received_consent_in_native_and_wasm` `:871`. |
| Gate rejection / recipients / retract id | `crates/maos-iac/tests/participant_recall_17_8.rs`: `v14_gate_rejected_addressee_is_never_recorded` `:533`, `v2_v3_v4_every_addressed_incarnation_is_a_recipient` `:298`, `retract_mints_one_id_per_operation` `:573`, `only_local_addresses_become_recipients` `:776`. |
| Rupture semantics | `crates/maos-kernel-core/tests/consent_rupture_adr_034.rs`; `crates/maos-cohort/tests/rupture_restart_18_0.rs`. |
| Retract lineage | `crates/maos-kernel-core/tests/retract_corpus_v0.rs:110`; corpus `crates/maos-eval/fixtures/intent-lineage-corpus-v0/scenario-041…050.json`. |
| A2A consent intake | `crates/maos-bin/tests/consent_refusal_1b.rs:216-461`. |
| Binary harness pattern | `crates/maos-bin/tests/installed_spirit_dispatch_20_1.rs:7-62`. |
| Readers | `crates/maos-bin/tests/tl_readers_side_tables_17_8.rs:82`. |

## Kernel and kloc forecast — method, not a prototype

**Method:** read the touched routines and the kernel-core send sites: `crates/maos-kernel-core/src/iac.rs:13` (a `pub use maos_iac::*` shim only), `scheduler/hook_dispatch.rs:186-245`, `inference/mod.rs:131-215`. Mailbox, adapter and TL code is in `maos-iac`, **outside** the ADR-069 kernel instrument. Under every option the kernel senders (`__kernel`, `"kernel"`) have no consent rows, so they stay accepted; no kernel edit is required. Forecasts are `[INFERENCE]`.

| Area | Forecast | Basis |
|---|---|---|
| `maos-kernel-core` | **0** (contingent 0…+5) | Only if the implementation normalizes `hook_dispatch.rs:211`'s `"kernel"` sender onto `KERNEL_SENDER_SPIRIT_ID`. Not required. |
| `maos-bin` | +90…+260 | Granter, gate implementation, policy registry and admission lifetime, composition install, session stamping. |
| `maos-iac` | +30…+110 | Recipients-by-frame read +15…+40; retract audience/bypass +10…+30; port refusal (C) +5…+15, or bound port (A) +30…+60. |
| `maos-manifest` | +40…+130 (A); 0 (B) | Consent section, validation, schema window. |
| `maos-spirit-abi` | +1…+6 | Schema constant/doc row (D-01 A). |
| `maos-domain` / `maos-wasm-host` | 0…+12 / 0…+8 | Only under D-02 A (variant and bridge mapping). |
| `xtask` | 0…+30 | Schema-version and corpus fixture migration, if a schema bump is ruled. |

Recorded basis: **20,428 tokei / 26,061 physical / 90 files** (`xtask/kernel-core-baseline.toml:505-508` HISTORY). Ceiling **20,429** (`xtask/kloc.toml:272`). Projection 20,428…20,433. Even added to 21-9's concurrent +87…+250, the total stays below 22,000. Non-kernel ceilings at base: maos-bin 39,719 (`kloc.toml:529`), maos-iac 9,053 (`:417`), maos-manifest 4,889 (`:426`), maos-domain 10,483 (`:436`), maos-spirit-abi 1,145 (`:278`), maos-wasm-host 3,278 (`:716`), xtask 46,754 (`:350`). These are retrospective ceilings, not per-landing grants. At landing, measure with `check-kernel-baseline` / `kloc-check --json`. If a kernel file changed, re-pin with `scripts/repin.py` and write the HISTORY row in the same commit.

**Local measurement (2026-11-21):** The orchestrator adjudicated raising the ceilings to the measured counts: `maos-bin` 39,909 → 39,962 and `maos-iac` 9,053 → 9,073. The kernel was unchanged at 20,428 tokei.

## Dependencies and claim limits

- **After 21-9 AC1.** At base the guest session that sends frames is compiled only with `wasm-host`, which is off by default (`lib.rs:90-93`, `Cargo.toml:15,34`). 21-9 also edits `admission.rs`, `manifest.rs` and `hook_dispatch.rs`. Rebase on its landing. Schema 8 belongs to 21-9; 21-7's version remains unallocated pending a fresh baseline and ratification.
- **Parallel GA:** 21-10's I8 proves existing cross-host typed-intent consent; that is not this story's local recipient/recall policy. No serial 21-7 → 21-10 dependency is added. The shared prerequisite is 21-9's default artifact. 21-7 still requires its own Postgres-free binary witness (AC1) and the separate eligible D-E20-6 security review; 21-10's I8 proof substitutes for neither.
- **Release-notes handoff:** shipped local consent restriction; Postgres boundary 3 unchanged; consent is technical, not legal. NFR-Comp-1 / NFR-Sec-7 not claimed. Cross-host retract is unsupported. Compiled-in Spirits have no bus-send primitive (D-21-7-04 C).

## Re-measured cite corrections — parent-owned

Only this file is written. These are a handoff to the owners, not edits.

| Source premise | OLD → NEW at `ea8c51b6` |
|---|---|
| Epic §21-7 feasibility (`epic-21…w6.md:167`) and SCP 11-12 `:178`: non-test mention | `main.rs:2798-2799` (SCP `:2799`) → comment `:2758-2762`, installer `:2763`. |
| Same: "sole `with_consent_gate` call is inside a test (`main.rs:14711`)" | → `:14763`, inside the compiled-in **`smoke_schedule_6_4` one-shot arm** (`:14733`), not `cfg(test)` (test module starts `:16058`). |
| Epic §21-7 "Dependencies: none" (`:166`) | → after 21-9 AC1 (default-on; `lib.rs:90-93`; SCP 11-12 `:443`). |
| Epic AC4 / received AC "native compiled-in sends … reaches the kernel send path" (`:163,173`) | `IacBusPort::deliver` has **no production caller**; `Ctx` has no send (`ctx.rs:58-133`); the fix is `maos-iac`, not kernel-core (`kernel-core/src/iac.rs:13`). Kernel forecast 0…+20 → 0 (contingent ≤+5). |
| `mailbox.rs:54,225,242,798`; `adapter.rs:750`; `iac_bus.rs:22`; DW `:1328-1330`; tracker `:419,508` | Hold at `ea8c51b6`. DW rows also hold in the `dev_ws-course28` working tree. |
| 17-8 `:45` `mailbox.rs:672-691`; CR-5 `adapter.rs:743-764` | → `mailbox.rs:767-803`; `adapter.rs:750-950` (historical record; for information). |
| `routing-audit-2026-10-22.md:26` names a 21-2 edge | Superseded by B2; `requirements-inventory.md:605` already carries the drop. Historical audit, no edit needed. |

Stable pointers: ADR-012 `:22-24`; ADR-013 `:65`; ADR-058 `:62`; ADR-069 `:35-46,74-90`; `docs/testing/proof-contract.md:12-20,28,49-73`; `RELEASE-HOLDS.md:114,133`.

## Dev Agent Record

### Agent Model Used

Creation: **anthropic/claude-opus-5-5** (`DraftProductionConsent`), story draft only.

Decision recording: **openai-codex/gpt-6.2-sol** (`ConsentPolicyRecord`), record only.

### Completion Notes List

- Read `bmad-create-story` and used 21-9 as the exemplar. Read `proof-contract.md`, epic §21-7, SCP 11-12/11-13, the 17-8 story, ADR-012/013/069, and all source anchors, using read-only `git show` at `ea8c51b6`.
- Wrote only this file. No tracker, epic, DW or code edits. No staging, commit or push.
- No Cargo, build, tests, gates, formatters, prototypes or exploit reproduction. Measurement methods were text reads and the checked-in HISTORY only.
- At decision recording, the operator approved D-21-7-01…06 (A/A/A/C/A/A) on 2026-11-21 UTC. The story and tracker were then `backlog` with all tasks open; that recording allocated no manifest version and performed no implementation, review or proof run.

### File List

- `_bmad-output/implementation-artifacts/21-7-production-consent-granter-and-gate.md` — measured story record and operator rulings.

## §A6 independent review artifact — 2026-11-21 (static)

The independent reviewer was a read-only static agent that self-reported GPT-family. This review does **not** satisfy D-21-7-06: the eligible non-GPT security review is **STILL OWED**. Implementation is on `ga-21-7-2026-11-21`; the evidence below is local only, with no hosted receipts.

**Implementation.** The `[consent.recall]` manifest declaration validates ID syntax and caps the map at 32 entries. It feeds `ConsentPolicy` in `maos-iac`, which acts as both granter and `ConsentGate`: it discards any guest-supplied envelope and stamps the sender's frame with the policy-derived envelope. The gate uses `RuptureReason::ConsentNotGranted`; guests see `IntentAllowlistMismatch` because there is no WIT variant for the new reason. Unbound `IacBusPort::deliver` refuses non-kernel senders with `IacBusError::UnattributedSender`. Retract routes to recorded recipients only if their recorded incarnation is still admitted.

**Review fixes and regression coverage.** The findings below were fixed; dedicated test gaps are listed separately.

- **H1:** Broadcast telemetry creates no directed participant records.
- **H2:** Consent declarations are bound to the admitted PID. Stamping and gating use one pinned policy snapshot per frame; unregister only removes the declaration for a matching PID.
- **H3:** The retract audience is same-boot only, and recipient incarnation is rechecked at enqueue.
- **M1:** Swaps may keep or narrow consent, not widen it. Hot swap refuses with `swap_consent_widened` before the coordinator; cold swap refuses before unload. Committed handover installs the successor's declaration.
- **M2:** Guest retract requires `spirit:peer` scope.
- **M3:** An unloaded addressee no longer suppresses the rupture, and a send for which all addressees are refused is not counted as delivered.

**Deferred / open.**

- The manifest schema floor and roster validation of IDs are open. The ruled schema bump is not allocated; `[consent.recall]` is currently additive at every schema.
- The WIT variant for `ConsentNotGranted` requires builds and re-issued provenance-pinned fixtures.
- A hot swap auto-reverted after commit does not restore the predecessor's consent declaration. The native/kernel hot-swap coordinator does not enforce keep-or-narrow; native Spirits cannot send today.
- Retract still has a window between the resolver read and prepare; there is no per-incarnation epoch beyond PID.
- There is no dedicated test for the M2 scope-token requirement or the M3 all-refused delivered count.
- The witness proves recall withholding only: a delivered frame is not hidden from the recipient session. The guarantee is recall-only, as documented in ADR-012 and the cookbook. Neither named plant (remove gate installation; remove stamping) nor the AC1 proven-red was run.
- The Postgres leg was not migrated. It remains parked under DW:1328 / `v25-reza-postgres-legs` / `RELEASE-HOLDS.md:114`.

**Orchestrator-run local results (no hosted receipts).** `maos-iac`, `maos-manifest`, and `maos-wasm-host` were green. In `maos-bin`, `consent_declaration_21_7`, `consent_gate_21_7`, `spawned_installed_20_9`, `spawned_door_20_9`, `spawned_swap_20_9`, and `--lib` were green. The kernel was unchanged at 20,428 tokei; the orchestrator's ceiling adjudication is recorded in the forecast above.

## Change Log

- 2026-11-20: Created at `ea8c51b6`. Recorded the Postgres-free scope, parked Postgres/Reza residual, six PROPOSED D-rows, conditional forecasts and parent-owned cite corrections. Status backlog pending operator rulings. Tracker untouched (`sprint-status.yaml:419` still `backlog`).
- 2026-11-21 UTC: Recorded operator approval of D-21-7-01…06 (A/A/A/C/A/A), record only. Retained schema 8 for 21-9 and left 21-7's version to fresh baseline and ratification. No serial 21-7 → 21-10 edge; 21-7 retains its separate eligible D-E20-6 review and own executable proof. Story and tracker remain `backlog`; tasks remain open.
- 2026-11-21: Recorded implementation on `ga-21-7-2026-11-21` and the independent static review; local evidence only. Moved the story to `in-progress`; the eligible non-GPT security review and AC1 proven-red remain open.
