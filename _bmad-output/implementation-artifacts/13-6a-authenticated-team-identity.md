---
baseline_commit: cb412348
depends_on: 13-5g-tl-stage2-datname-inversion-defense-in-depth
blocks: 13-6b-production-cross-team-crossing-initiators
kernel_grant: NONE — ZERO maos-kernel-core Δ expected, pin stays 23401. NON-kernel Δ: maos-cohort (COHORT_SCHEMA_V4), maos-a2a-core (seam semantics)
splits_from: 13-6b-production-cross-team-crossing-initiators (operator-ratified 2026-07-28; that story was named 13.6a at the time and was renamed in the same split — its stale pre-split file was deleted 2026-07-29)
---

# Story 13.6a — The cohort knows which host is which, but not which team it speaks for

Status: **done**

**Kernel-core Δ: ZERO expected — and that is not the same sentence as "zero delta."** `xtask/kernel-core-baseline.toml src_lines = 23401` unchanged; `xtask/fkcs-baseline.toml` byte-untouched at the frozen `23081`. Work lands in **`maos-cohort`** (`COHORT_SCHEMA_V4` + the per-member team binding) and **`maos-a2a-core`** (seam semantics). **No new gate** — legs onto `check-multi-tenant-loom` (13.5c **K5(b)**).

> **Read this first — this story exists so that the story after it cannot ship a bypassable wall.**
>
> Story 13.6 was split into a mechanism (13.6a) and a judge (13.6). The mechanism was then reviewed by party-mode, pressure-tested, and found to contain **two** separable mechanisms with a hard ordering between them. **Operator ratified 2026-07-28: identity first, crossing second.**
>
> **The crossing cannot be built safely before this story exists.** `build_replication_bundle_v2` (`bundle.rs:414`) derives its *signing* key via `derive_team_signing_seed(base_seed, region, team)` (`sealed_export.rs:72`), which works for **any** team. The crossing requires every emitter to hold `MAOS_CROSS_TEAM_BASE_SEED`. `apply_replication_bundle` then decides consent from `bundle.source_team` (`:875-879` → `:917`) — a **self-declared field whose signature the emitter can forge**. And nothing binds the authenticated host to a team: `handle_intake_verified` (`router.rs:1222-1232`) binds `host_id` only; `CohortMember { host_id, fingerprint, roles }` are hosts, `TeamEntry.members` are **Spirits**. There is no host→team edge in the signed manifest at all.
>
> Build the crossing first and there is a commit in which asymmetric cross-team consent — the property Story 13.3 exists to provide — is bypassable by impersonation. The review offered exactly that trade (*"accept single-operator trust, document it loudly"*) and **the room rejected it**: 13.6 is a judge, and a knowingly-bypassable mechanism can only be graded ABSENT. Rejecting it in the review and then re-creating it in the commit order would be the same mistake one layer down.
>
> **Everything here is provable today, against synthetic frames, with no crossing in existence.** The two dead-wire negatives stay green; this story inverts nothing.
>
> **What this story does NOT claim.** It does not deliver per-team key provisioning (`v25-signed-shard`, v2.5). A seed-holder can still forge **within its own team**, and an operator who controls a host controls that host's team. **Isolation is enforced against a peer, not against the operator** — which is the correct scope for a single-org product, and must be written down rather than implied.

---

## Story

**As** the operator of Reza's single-org Cortex,
**I want** the signed cohort manifest to declare which team each host speaks for, and both A2A seams to enforce it,
**so that** when the cross-team crossing is wired in the next story, "who is this row from?" is answered by an identity the shared signing seed cannot forge — instead of by a field the sender fills in itself.

---

## The gap, in code — measured at `cb412348`

### D-1 — one seed signs for every team

```rust
// crates/maos-loom-lite/src/replication/bundle.rs:414
let team_seed = derive_team_signing_seed(base_seed, source_region, source_team);
let signing_key = SigningKey::from_bytes(&team_seed);
let signature = signing_key.sign(&sign_payload);
```

`derive_team_signing_seed(base_seed, region, team)` (`crates/maos-audit/src/sealed_export.rs:72`) accepts **any** `(region, team)`. 13.6b requires every emitter to hold the seed. ⇒ **every emitter can sign under every team's key.**

The shipped 13.2 negative (`bundle.rs:1656-1676`, `:1822-1843`) forges by **relabel** — sign under team-b, swap the label to team-a — which fails because team-a's key was never used. That is a **seed-less** forger. A seed-holder derives the right key and signs correctly. **The existing control and this threat do not intersect at any point.**

### D-2 — consent is decided from the forgeable field, and nothing binds host to team

`apply_replication_bundle` sets `from_team = bundle.source_team.as_ref()` (`bundle.rs:875-879`) and calls `is_granted(from_team, context.to_team, context.intent)` (`:917`).

`handle_intake_verified` (`router.rs:1222-1232`) binds `request.params.from.host_id == verified_peer` — **host identity only, never team.** And the manifest has no host→team edge:

| | |
|---|---|
| `CohortMember { host_id, fingerprint, roles }` | `manifest.rs:120` — **hosts** |
| `TeamEntry { team_id, region, datname, members: Vec<SpiritId> }` | `manifest.rs:130` — members are **Spirits** |

The only host→team fact in the system is each host's own local `MAOS_LOOM_HOME_TEAM`: unsignable, uncheckable by a peer.

**Attack this closes.** team-a's host — a valid TLS-pinned member holding the seed — wants a row in team-c where `(team-a → team-c)` is **denied** but `(team-b → team-c)` is **allowed**. It stamps `source_team = team-b`, derives team-b's key, signs. At team-c: transport ✓, signature verifies against the **claimed** pair ✓, `is_granted(team-b, team-c)` ✓. Row lands.

### D-3 — carrying `SpiritId` in the leaf does **not** work; the mTLS identity does

Everything inside the bundle is forgeable, because one key signs all of it — a stable `SpiritId` in the leaf would be forged too. (The leaf carries `spirit_pid: i64`, runtime-local and meaningless on the destination host; `TenantMapPort::team_of` (`tenant_map.rs:115`) is local-only.)

**`CohortMember.fingerprint` is the exception**: operator-pinned, cert-bound, and **not derived from `base_seed`**. team-a's host cannot present team-b's certificate. That is the axis to bind on.

### D-4 — the canonical form prices it at `COHORT_SCHEMA_V4`

`to_canonical_bytes` (`manifest.rs:208`) writes per member `host_id` / `fingerprint` / `roles`, and the `TeamEntry` block sits inside the `V2 | V3` arm (`:254`). **Any new field changes the signed pre-image.** `COHORT_SCHEMA_V3` + `SIG_DOMAIN_V3` are **already spent** (13.3; `manifest.rs:53`, `SUPPORTED_COHORT_SCHEMAS` at `:54`). With `deny_unknown_fields` on and one shared pre-image, **there is no "just add an optional field" shortcut.**

⇒ **`COHORT_SCHEMA_V4` + `SIG_DOMAIN_V4`, v1/v2/v3 canonical bytes byte-identical by construction** — the 9.2b additive idiom, now over **three** frozen predecessors.

### D-5 — the eviction check the crossing will rely on is real, but conditional in four ways

*(Established by the 13.6b entailment pressure-test; recorded here because this story owns the conditions.)*

`handle_intake_verified` → `handle_intake` (`:840`) → cohort gate (`:1108`); `CohortConsentVerdict::NotCurrent` is documented (`cohort.rs:87-89`) as *"stale **or no longer contains the local member**"* → NACK `:1117`. The **emitter** side runs the same chokepoint (`:509`): `Send` seam `NotCurrent` → `A2AError::ConfigInvalid` (`:677-682`). `NotCurrent`'s condition (`state.rs:555-566`) is freshness **AND** `local_host ∈ members`.

Four conditions, all true at `cb412348`, **none asserted anywhere**:

1. `is_reserved_cohort_intent` (`router.rs:502`) short-circuits **both** seams (`:660` Send, `:1107` Accept) for exactly reissue + halt-receipt.
2. The gate is `Option<Arc<dyn CohortManifestGate>>`; without a real one, `LegacyCohortManifestGate` `Defer`s everything (`cohort.rs:137`).
3. Emitter-side self-check exists — verified.
4. ⚠ **`Defer` is evaluated BEFORE the self-membership check, and it is a pass:**

```rust
// state.rs:546-554
if !manifest.members.iter().any(|m| m.host_id == counterparty.as_str()) {
    return CohortConsentVerdict::Defer;      // ← returns BEFORE `current` is computed
}
// state.rs:555-566
if !current { return CohortConsentVerdict::NotCurrent; }
```

So **(a)** an unrostered counterparty means the local host's own eviction is never evaluated, and **(b)** an applier whose manifest has dropped the *sender* returns `Defer`, which `:1115` treats as a **pass** — **a de-rostered sender pushes a crossing into a healthy applier.** TOFU pin and peer allowlist do not help: those are *config-time* facts, not *manifest-time* facts. This is `Defer`'s documented intent (*"a mixed-deployment bilateral path, not a cohort denial"*) — deliberate generality that is **wrong for a crossing**.

*(Separately confirmed and NOT the hazard: `Defer` is also reachable for a **rostered** peer via `EConsentPeerNotMember` (`state.rs:592`) — roster membership and consent-table membership are two different notions — but that path sits **after** the `current` check and does not bypass self-eviction.)*

---

## Acceptance Criteria (5)

### AC1 — The signed manifest declares which team each host speaks for

**Given** no host→team edge exists in the cohort manifest (D-2),
**When** an operator issues a manifest,
**Then** `CohortMember` carries an operator-signed team declaration, landing as **`COHORT_SCHEMA_V4` + `SIG_DOMAIN_V4`** with the field written into the canonical pre-image **only** in the V4 arm,
**And** **v1, v2 and v3 canonical bytes are byte-identical by construction**, proven by golden-byte tests for **all three** frozen predecessors (D-4; the 9.2b idiom — 13.3b ran it over two, this is three),
**And** `SUPPORTED_COHORT_SCHEMAS` accepts V4 and a V4→V3 downgrade is rejected and audited on the shipped `SchemaDowngrade` path,
**And** the binding is **fail-closed**: a member with no declared team **cannot originate a crossing at all** — absence refuses, never permits.

### AC2 — Both seams refuse a peer that does not speak for the team it claims

**Given** `handle_intake_verified` binds host identity only,
**When** a frame claims a source team,
**Then** the seam refuses unless the **TLS-verified peer** speaks for that team under the V4 manifest — the fingerprint is operator-pinned and **not seed-derived**, so a seed-holder cannot forge it,
**And** the **impersonation negative** is executed and proven-red **against synthetic frames** (no crossing need exist): a peer bound to team-a, presenting a correctly-signed bundle claiming `source_team = team-b`, is refused **for identity mismatch** — not incidentally, and not for a signature reason,
**And** ⚠ the refusal is distinguishable from the seam's existing `host_id`-mismatch NACK; two different failures must not collapse into one code.

### AC3 — Eviction is enforced on **both** endpoints, and its four conditions are pinned

**Given** the crossing's eviction safety rests on four unasserted facts (D-5),
**When** this story lands,
**Then** `Blocking` legs pin all four: **(i)** the crossing intent is **not** in `is_reserved_cohort_intent`, so the gate is consulted on **both** seams; **(ii)** both endpoints wire a **real** `CohortManifestGate`, never `Legacy`/`None`; **(iii)** live evicted-endpoint legs on **both sides** — an evicted applier NACKs `NotCurrent`, an evicted emitter fails `ConfigInvalid` — each **proven-red by restoring membership**; **(iv)** AC4 below,
**And** ⚠ **do not attempt to route this through `team_guard`**: it guards exactly `write:497` / `erase:718` / `read:834` / `scan:956` and refuses foreign-team rows by design, so it would refuse every legitimate crossing. The eviction check inside `TenantMapAdapter::current_manifest()` is reachable only via `team_of` (from `team_guard`) or `datname_for` (from `connection_assignment_guard`, **boot-time only**) — **it is a side effect of the tenant-map lookup, not a control in its own right.**

### AC4 — On the crossing intent, `Defer` is a refusal — and general A2A keeps its bilateral fallback

**Given** `Defer` returns **before** the self-membership check and is treated as a pass (D-5(4)),
**When** a de-rostered sender presents a crossing to a healthy applier,
**Then** it is **refused**, on **both** seams, for the crossing intent specifically,
**And** the mixed-deployment bilateral fallback is **preserved unchanged for every other intent** — proven by a regression control that a non-crossing frame from an unrostered peer still `Defer`s and is admitted,
**And** the negative plants a sender removed from the roster by a **fresh, valid signed reissue** (not a stale or forged manifest) and requires refusal — the point is that a *legitimately revoked* member is stopped,
**And** ⚠ this is **additive to a new intent**, not a rollback of Story 12.1: `collective:share` postdates 12.1, so no shipped behavior changes. Say that in the ADR.

### AC5 — Threat model corrected, ADR amended, budget held

**Given** `docs/loom-threat-model.md` **T1** currently states *"same-region cross-team forgery is cryptographically infeasible"* and **CLOSED on the team axis at ENTRY**, with Vex's blast-radius note scoping the exposure to an attacker who **"recovers"** the seed,
**When** this story lands,
**Then** **T1 is corrected in the same commit**: its close is re-scoped to a **seed-less** forger; the blast-radius note is extended from *a breach* to *"every emitter holds the seed by design as of 13.6b"*; and the surviving limit is stated plainly — **a seed-holder can still forge within its own team, and an operator controlling a host controls that team; isolation is enforced against a peer, not against the operator.** True per-team key provisioning stays in the `v25-signed-shard` family (ADR-055 residual #6),
**And** **no artifact may say "Fork-4 proven"** of the crossing path,
**And** legs go on **`check-multi-tenant-loom`** — no new gate — with **one `#[test]` per `--exact` leg** (the gate's only anti-vacuity oracle is `"running 1 test"` + `"1 passed"`),
**And** `check-kernel-baseline` proves **23401 unchanged**; if it disagrees, **stop for FLAG-Winston**. State both sentences: kernel-core ZERO, non-kernel Δ in `maos-cohort` + `maos-a2a-core`.

---

## Traps

1. **No optional-field shortcut.** One shared pre-image + `deny_unknown_fields`. It is V4 or it is a broken signature.
2. **Three golden-byte predecessors**, not two. v1, v2 **and** v3.
3. **`Defer`-as-refusal is scoped to the crossing intent.** Widening it to all intents rolls back 12.1's mixed-deployment path — the regression control in AC4 exists to catch exactly that.
4. **Do not route eviction through `team_guard`** (AC3) — it would refuse every legitimate crossing.
5. **This story inverts nothing.** Both dead-wire negatives (`replication-crossing-has-no-production-initiator`, `cross-wall-recall-no-production-caller`) stay **green**; 13.6b inverts them. A leg that reds here means the crossing leaked in early.
6. **`.expect()`, never skip**, on live legs; skipped ≠ passed (13.5g).
7. **Proven-red per limb**, serialized, byte-identical restore (`diff -q`).
8. **Measure the baseline in an isolated `git worktree`** (the 13.5i error, fixed at 13.5j). Code first, then the pin.
9. **`cargo run -q -p xtask -- <cmd>`.** No `cargo xtask` alias.

---

## Tasks

- [x] **T1 (AC1)** — `CohortMember` team field; `COHORT_SCHEMA_V4` + `SIG_DOMAIN_V4`; canonical-form V4 arm; golden bytes for v1/v2/v3; `SUPPORTED_COHORT_SCHEMAS`; downgrade rejection.
- [x] **T2 (AC2)** — Seam check: TLS-verified peer must speak for the claimed team. Distinct refusal code from the `host_id` mismatch.
- [x] **T3 (AC2)** — Impersonation negative against synthetic frames, proven-red.
- [x] **T4 (AC3)** — Four eviction pins, including live evicted-emitter **and** evicted-applier legs, each proven-red by restoring membership.
- [x] **T5 (AC4)** — `Defer`-as-refusal on the crossing intent, both seams + the bilateral-preserved regression control + the de-rostered-sender negative.
- [x] **T6 (AC5)** — Correct `docs/loom-threat-model.md` T1; amend ADR-055; register legs on `check-multi-tenant-loom`.
- [x] **T7 (AC5)** — Gates: `check-kernel-baseline` (**23401**), `kloc-check`, `check-multi-tenant-loom`, `check-reza-production-path`, `cargo fmt --all -- --check`, `cargo test --workspace`. Check `maos-cohort` / `maos-a2a-core` kloc ceilings **before** writing, not at "done" (breached four consecutive stories).
- [x] **T8** — Record the dev model (`check-dev-model-tier` is live; frontier-class, §A6 net **non-degradable** for this boundary per `epic-13:13`).

### Review Findings

- [x] [Review][Patch] Bind team declarations to the manifest cert fingerprint — fixed by `reconcile_transport_identity_with_manifest` at daemon bootstrap across `tcp.peer_pins`, `file.peers`, and the daemon's own leaf, plus per-frame equality between the negotiated leaf and signed `CohortMember.fingerprint` on both seams. Two new cert-axis negatives prove stale/unnamed leaves cannot speak for a team. (blind+edge+auditor, high)
- [x] [Review][Patch] Read consent and team identity from one manifest snapshot — fixed by `CohortManifestGate::consent_and_team`; the verified Accept verdict is threaded into intake, and the Send verdict plus stamp come from the same lock. (blind+edge+auditor, medium)
- [x] [Review][Patch] Decode `CODE_TEAM_IDENTITY_MISMATCH` (-32010) at the sender — fixed in `interpret_response`; `claimed_team`/`declared_team` now reconstruct `CohortTeamIdentityRefused` instead of falling through to `TransportFailed`. (blind+edge+auditor, medium)
- [x] [Review][Patch] Attribute Send-seam declaration failures to the local host — fixed by reading the outbound frame's `from.host_id`, not the destination peer. (blind+edge, medium)
- [x] [Review][Patch] Make the AC3(i) Accept-seam leg non-vacuous — fixed by retaining `RecordingGate` and asserting `accept_calls == 1`. (edge+auditor, medium)
- [x] [Review][Patch] Scope the AC3(ii) composition-root assertion to the production bind — fixed with the full `gate + observer + Arc<digest_port> + rupture_sink` sequence, which the in-file fixture cannot satisfy. (edge+auditor, medium)

## Dev Notes

| Need | Use | Path |
|---|---|---|
| Canonical signed form | `to_canonical_bytes` | `crates/maos-cohort/src/manifest.rs:208`; member loop `:235-243`; V2\|V3 arm `:254` |
| Schema constants | `COHORT_SCHEMA_V1/V2/V3`, `SUPPORTED_COHORT_SCHEMAS` | `manifest.rs:51-55` |
| Gate contract | `CohortManifestGate::consent_decision` | `crates/maos-a2a-core/src/cohort.rs:107` |
| Real gate impl | `impl CohortManifestGate for CohortManifestState` | `crates/maos-cohort/src/state.rs:531` |
| Verdicts | `CohortConsentVerdict::{Defer, Admit, AdmitOutbound, NotCurrent, Deny}` | `cohort.rs:79-91` |
| Accept seam | `handle_intake` cohort block | `crates/maos-a2a-core/src/router.rs:1107-1130` |
| Send seam | outbound cohort block | `router.rs:660-690` |
| Sole chokepoint | `cohort_consent_decision` | `router.rs:509` |
| Peer identity bind | `handle_intake_verified` | `router.rs:1222-1232` |

**Budget.** kernel-core ZERO @ **23401**; fkcs frozen `23081` byte-untouched; non-kernel Δ in `maos-cohort` + `maos-a2a-core`.

**Previous-story intelligence.** 13.5g's review found *"legs green while connecting to nothing"* and that **no leg exercised the composition root** — deleting the whole Phase A block left 15 Blocking legs green. **Apply the same test here:** after T2–T5, delete the seam check and confirm the new legs red. If they stay green they are testing a struct, not a seam.

### References

- [Source: `epics/epic-13-reza-cortex-v2-2.md#232`] — checklist item 5 (split rather than hide).
- [Source: `docs/loom-threat-model.md#T1`] — the claim this story corrects.
- [Source: `docs/adr/ADR-055-multi-tenant-loom.md`] — tenant wall, Fork-4 HKDF weld.
- [Source: `_bmad-output/implementation-artifacts/13-6a-review-party-mode-2026-07-28.md`] — B1/B3 review package.

---

## Residuals

1. **Operator-trust limit (OPEN, documented, correctly out of scope).** Isolation is enforced **against a peer, not against the operator**. Reza is one operator, so no Reza claim is made truer by closing it. Per-team key provisioning = `v25-signed-shard`, v2.5.
2. **`v25-signed-shard`** (ADR-055 residual #6) — unchanged.
3. **Fallible `record_invocation`** (13.5d, kernel-core, needs a second FLAG-Winston) — unchanged.

---

## Dev Agent Record

### Agent Model Used

Model: anthropic/claude-opus-5 (`opus-5`, frontier-class allowlist per E11 retro §A1 / E12-B3) for the dev pass. The non-degradable §A6 full-layer review net ran 2026-07-29 (Blind Hunter + Edge Case Hunter + Acceptance Auditor): 6 unique findings, all patched and verified; 0 deferred, 0 dismissed.

### Debug Log References

**Baseline, measured in-tree at `cb412348` before any code was written** (Trap 8):

| Budget | Before | After | Note |
|---|---|---|---|
| `maos-kernel-core/src` | 23401 (pin) | **23401** | `check-kernel-baseline: PASSED (actual = pinned)`. ZERO kernel-core Δ. |
| `xtask/fkcs-baseline.toml` | frozen 23081 | frozen 23081 | `git diff --stat` empty — byte-untouched. |
| `maos-a2a-core` kloc | 4009 / 4109 | **4187 / 4271** | §A6 review correctness/security repair. Operator-ratified named grant 4109→4271 (measured 4187 + 84 reserve by the 2% formula); FLAG-Winston for Epic-13 retro. |
| `maos-cohort` kloc | 4452 | **4800** | No per-crate ceiling exists for this crate (the documented unlisted-crate governance gap, retro-scoped). |
| `maos-bin` kloc | 14231 / 14433 | **14298 / 14433** | Bootstrap identity reconciliation in the production daemon; still within the existing ceiling. |
| `xtask` kloc | 31378 / 31530 | **31587 / 31690** | Re-based per the documented process with a named driver + tight residual (reserve 103). Pure gate-registry data. |

**Proven-red per limb, serialized, byte-identical restore (`diff -q`) — dev-pass mutations plus two review mutations:**

| Deleted / mutated | Leg that reds | Result |
|---|---|---|
| accept-side team-identity block (`handle_intake_verified`) | `impersonation_is_refused_at_the_accept_seam` | RED |
| accept-side team-identity block | `crossing_without_a_verified_team_claim_is_refused` | RED |
| emitter-side `source_team_stamp` | `emitter_refuses_a_crossing_it_cannot_speak_for` | RED |
| Send-seam `Defer if crossing` arm | `derostered_crossing_is_refused_on_both_seams` | RED |
| Accept-seam `Defer`→`Deny` rebind | `derostered_crossing_is_refused_on_both_seams` | RED |
| **widen `Defer`-as-refusal to every intent** (Trap 3) | `bilateral_fallback_survives_for_every_non_crossing_intent` | RED |
| manifest fingerprint equality | `a_certificate_the_manifest_does_not_name_speaks_for_no_team` + `emitter_with_a_stale_local_leaf_cannot_originate_a_crossing` | RED (9 siblings green) |
| rewritten accept-side team-identity block | five team-axis legs, including impersonation and absence | RED (6 siblings green) |

The last row is the 12.1-rollback trap firing as designed: the regression control catches the widening, not just the absence.

**13.5g "legs green while connecting to nothing" check applied.** Every one of the five limbs above was deleted and the legs red — they are wired to the seams, not to a struct. `crossing_intent_is_not_reserved_so_both_seams_consult_the_gate` additionally counts gate invocations through a recording decorator with a reserved-intent negative control, so it proves the gate is *consulted* rather than that a predicate reads a certain way.

**Golden bytes (three frozen predecessors, all unchanged):**

| Schema | Length | SHA-256 |
|---|---|---|
| v1 | 640 | `835573a8…3712a` (pre-existing, unchanged) |
| v2 | 762 | `7bd7b8b0…5d0d` (pre-existing, unchanged) |
| v3 | **806** | `d2bde459…0d20` (newly pinned — v3 had no golden before this story) |
| v4 | 863 | `a8b4c40d…db1b` |

### Completion Notes List

**T1 — `COHORT_SCHEMA_V4` + `SIG_DOMAIN_V4` + the host→team edge.** `CohortMember.team: Option<TeamId>` is written into the canonical pre-image in a **V4-only tail appended after every predecessor section** — the 9.2b additive idiom over three frozen predecessors, so v1/v2/v3 bytes are byte-identical *by construction*, not by luck. `v4_is_additive_over_all_three_frozen_predecessors` asserts the structural claim directly: past the domain separator and the 8-byte `schema_version` scalar, the V4 pre-image repeats the V3 pre-image byte-for-byte and only then appends. The tail re-writes `host_id` beside each declaration so the edge binds by name, not by positional index. Referential integrity: a member may only declare a team the same signed body declares (`ECohortMemberTeamUnknown`); a pre-V4 manifest carrying a member `team` is **refused** (`ECohortSchemaMemberTeamMismatch`), because an ignored declaration is an *unsigned* declaration on the shared pre-image. The V4→V3 downgrade needed **no new branch** — `schema_is_downgrade` was already ordinal; the floor is now asserted at 4→3/4→1 and behaviourally on the shipped `SchemaDowngrade` audit path, with a same-schema-higher-revision positive control so the refusal is about the schema and not the revision.

**T2 — the seams, review-hardened.** `A2AJsonRpcRequest.cohort_source_team` follows the 12.2 `cohort_acting_role` carrier idiom (no `maos-domain` touch, no `FrameKind` variant). `CohortManifestGate::consent_and_team` returns the consent verdict and team declaration from one locked manifest snapshot and returns a declaration only when the relevant endpoint's TLS leaf equals the signed `CohortMember.fingerprint`. Send stamps from the local host's signed edge and refuses an undeclared/stale local cert; verified Accept checks the negotiated peer leaf and reuses the same-snapshot verdict in intake. Bootstrap independently reconciles `tcp.peer_pins`, `file.peers`, and the daemon's own certificate against the signed manifest before binding. `CODE_TEAM_IDENTITY_MISMATCH = -32010` remains distinct from the host-axis -32007 and now round-trips through `interpret_response` as `CohortTeamIdentityRefused`.

**T3 — the impersonation negative, on synthetic frames.** `host-a` is a valid TLS-pinned member bound to `team-a`; it presents a crossing claiming `source_team = team-b` over its own validly-pinned connection and is refused **for identity**, with the refusal naming what the signed manifest declares. The AC2 ⚠ is discharged in the same test: a forged `from.host_id` over the same connection still returns `CODE_PEER_IDENTITY_MISMATCH (-32007)`, and the two codes are asserted distinct. A positive control (honest claim → ACK) runs **first**, so a refuse-everything seam cannot satisfy the leg.

**T4 — the four eviction conditions, pinned.** (i) The crossing intent is not reserved — asserted **behaviourally** by counting gate invocations on both seams, with a reserved-intent negative control that must stay at zero. (ii) A source-level leg pins the composition root to a real `CohortManifestState`-derived gate that is actually *passed* to the bind, and that `LegacyCohortManifestGate` is never named in production sources; both endpoints run the same daemon, so one root covers both. (iii) Live evicted-endpoint legs on both sides — evicted applier NACKs on the `NotCurrent` path, evicted emitter fails `ConfigInvalid` — each **proven-red by restoring membership** through a fresh valid signed reissue, plus a staleness leg for `NotCurrent`'s other limb. (iv) See T5. Eviction is **not** routed through `team_guard` (Trap 4): that guard refuses foreign-team rows by design and would refuse every legitimate crossing.

**T5 — `Defer` is a refusal on the crossing intent only.** Both seams route it through the shipped `Deny` path under a new attributable cause `CohortConsentDenial::CrossingDeferRefused` (`reason = "crossing_defer_refused"`), so the rupture emission and typed NACK data are the ones already in production. The bilateral regression control proves an unrostered peer's non-crossing frame still defers **and is admitted**, and the discriminating pair (same peer, crossing → refused) sits beside it. A rostered peer that merely lost its accept entitlement is refused as `no_grant` — a *different* cause — so the Defer rule is not swallowing every accept denial.

**⚠ Honest finding, recorded rather than smoothed over: on the ACCEPT seam the AC4 rule is defense-in-depth behind AC2, and the test says so.** At accept, `Defer` ⟺ the counterparty is outside `manifest.members`; a dropped member also loses its signed team edge, so on the verified path the AC2 team binding fires **first** and a de-rostered sender is refused under `CODE_TEAM_IDENTITY_MISMATCH`, not under the Defer rule. The Defer rule is therefore exercised where it lives — the shared `handle_intake` body, which is also the direct production entry the loopback router uses — and the layering is asserted explicitly in `derostered_crossing_is_refused_on_both_seams` (four ordered clauses) rather than left implied. The rule is kept because it is the backstop if a future change ever lets a team edge survive de-rostering. On the **Send** seam the rule is independently reachable and is proven there directly (local host keeps its edge; the counterparty is revoked).

**T6 — threat model corrected in the same commit.** `docs/loom-threat-model.md` T1's claim *"same-region cross-team forgery is cryptographically infeasible"* is **withdrawn** and re-scoped to a **seed-less** forger, with the reason stated: `derive_team_signing_seed` accepts any `(region, team)`, so the shipped 13.2 negative forges by *relabel* and never intersects the seed-holder threat. Vex's blast-radius note is extended from *a breach* to *"every emitter holds the seed by design as of 13.6b"*. The surviving limit is written plainly — **isolation is enforced against a peer, not against the operator** — and per-team key *provisioning* stays `v25-signed-shard` (v2.5). ADR-055 gains §4b, four rejected alternatives, and a re-scoped §4 consequence; the status/gate/date front matter and `tests/coverage-matrix.yaml` ADR-055 notes are updated. **No artifact says "Fork-4 proven"** of the crossing path.

**T7 — final verification after §A6 remediation.** `cargo fmt --all -- --check` clean; `check-kernel-baseline` PASSED at **23401** (actual = pinned, ZERO kernel-core Δ); `kloc-check` PASSED (aggregate **135955**); `check-multi-tenant-loom` PASSED (live-substrate advisory, 0 absent successors); `check-reza-production-path` PASSED (live-substrate advisory, 2 absent successors); `cargo test --workspace` PASSED (exit 0). Focused behavioral evidence: `team_identity_13_6a` **11/11** green; `cohort_daemon_smoke_13_5c` **9 passed / 1 advisory ignored**.

**Two gates are RED at HEAD and were RED at the `cb412348` baseline, byte-identically — pre-existing, not introduced here** (verified by re-running both against a stashed tree): `check-service-boundary` (3 violations: removed `maos_kernel_core::memory::REGISTERED_ERASURE_BACKENDS`, plus `SecurityManagerAdapter`/`CapabilityRegistryAdapter` constructed N=2 in `main.rs`) and `check-dev-model-tier` (10 violations across 25 story files, including the sibling 13.6/13.6b spec artifacts and 13.5g/13.5i/13.5j records). This story neither adds nor removes any of them.

**Budget sentences, both stated as required.** *Kernel-core is ZERO — and that is not the same sentence as "zero delta."* The non-kernel delta is real: `maos-cohort` (schema V4 + the per-member binding), `maos-a2a-core` (seam semantics), `maos-bin`/`xtask` (tests + gate legs).

**Inverts nothing (Trap 5).** `replication-crossing-has-no-production-initiator` and `cross-wall-recall-no-production-caller` are both still green. 13.6b inverts them.

### File List

**New**

- `crates/maos-bin/tests/team_identity_13_6a.rs`

**Modified — production**

- `crates/maos-cohort/src/manifest.rs` — `COHORT_SCHEMA_V4`/`SIG_DOMAIN_V4`, `CohortMember.team`, V4 canonical tail, widened V2|V3 arms, member-team referential integrity, `team_of_host`, `cross_team_admits` at V4, V4 goldens/negatives.
- `crates/maos-cohort/src/state.rs` — locked-snapshot `consent_and_team`, signed-fingerprint equality, local-team helper, ordinal schema-floor + V4→V3 downgrade legs.
- `crates/maos-cohort/src/error.rs` — `ECohortSchemaMemberTeamMismatch`, `ECohortMemberTeamUnknown`.
- `crates/maos-cohort/src/lib.rs` — re-export `COHORT_SCHEMA_V4` / `SIG_DOMAIN_V4`.
- `crates/maos-cohort/src/consent.rs` — fixture field only.
- `crates/maos-a2a-core/src/cohort.rs` — `COHORT_INTENT_COLLECTIVE_SHARE`, `CohortConsentDenial::CrossingDeferRefused`, combined `consent_and_team` port, fail-closed legacy impl.
- `crates/maos-a2a-core/src/error.rs` — `A2AError::CohortTeamIdentityRefused`.
- `crates/maos-a2a-core/src/transport/json_rpc.rs` — `CODE_TEAM_IDENTITY_MISMATCH = -32010`, `cohort_source_team` + builder.
- `crates/maos-a2a-core/src/router.rs` — one-snapshot consent+team evaluation on both seams, local/peer leaf binding, source-team stamp, scoped `Defer` refusal, -32010 response dispatch, correct Send attribution.
- `crates/maos-a2a-tcp/src/transport.rs` — forwards the negotiated peer fingerprint per frame and wires the loaded local leaf into the router core.
- `crates/maos-bin/src/main.rs` — fail-closed bootstrap reconciliation across signed manifest, handshake pins, frame-level peer configs, and the daemon's own certificate.

**Modified — gate / CI / docs**

- `xtask/src/check_multi_tenant_loom.rs` — 14 new Blocking legs; header updated.
- `xtask/kloc.toml` — `xtask` 31530→31690 dev-pass grant; operator-ratified §A6 review grant for `maos-a2a-core` 4109→4271 (measured 4187 + 84 reserve).
- `.github/workflows/discipline.yml` — tenant-gate job comment.
- `tests/coverage-matrix.yaml` — ADR-055 notes + `valid_until`.
- `docs/adr/ADR-055-multi-tenant-loom.md` — front matter, §1 four schemas, NEW §4b, consequences, rejected alternatives.
- `docs/loom-threat-model.md` — T1 scope correction, blast-radius extension, status line, carried debt.

**Modified — test fixtures (compiler-forced `team: None`)**

- `crates/maos-a2a-core/tests/cohort_consent_router_12_2.rs` (also implements the new port method, fail-closed)
- `crates/maos-a2a-core/tests/cohort_consent_chokepoint_12_2.rs` — pins one verdict-only fallback plus exactly two combined seam calls.
- `crates/maos-a2a-core/tests/cohort_manifest_chokepoint_12_1.rs` — pins atomic currentness/team reads on both verified seams.
- `crates/maos-a2a-tcp/tests/t_12_1_cohort_mesh.rs`
- `crates/maos-a2a-tcp/tests/t_12_2_cohort_consent.rs`
- `crates/maos-a2a-tcp/tests/t_12_3_cohort_halt_receipt.rs`
- `crates/maos-a2a-tcp/tests/t_12_4a_digest_read.rs`
- `crates/maos-a2a-tcp/tests/t_12_5_cohort_hot_swap.rs`
- `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs`
- `crates/maos-bin/tests/cohort_halt_receipt_12_3.rs`
- `crates/maos-bin/tests/cross_team_consent_13_3.rs`
- `crates/maos-bin/tests/tenant_map_13_1.rs`

---

## Change Log

| Date | Change |
|---|---|
| 2026-07-28 | Created by operator-ratified split of the former 13.6a. Identity ships **before** the crossing so no commit ever contains a bypassable cross-team wall — the same reasoning the room used to reject the review's P2, applied to commit order. 5 ACs; ZERO kernel-core @23401; inverts no dead-wire leg; everything provable against synthetic frames. |
| 2026-07-28 | **Implemented (dev pass, `opus-5`).** `COHORT_SCHEMA_V4` + `SIG_DOMAIN_V4` carry an operator-signed `CohortMember.team` in a V4-only canonical tail; v1/v2/v3 goldens unchanged and v3's golden newly pinned (806 / `d2bde459…`). Both A2A seams enforce the edge — send stamps from the local host's own declaration, accept refuses a claim the TLS-verified peer does not speak for under `CODE_TEAM_IDENTITY_MISMATCH (-32010)`, distinct from `-32007`. `Defer` becomes a refusal on `collective:share` only, with the 12.1 bilateral fallback proven preserved. 14 Blocking legs added to `check-multi-tenant-loom` (44/44 green); six mutation limbs proven-red with byte-identical restore. `loom-threat-model.md` T1 re-scoped to a seed-less forger in the same commit; ADR-055 §4b added. ZERO kernel-core Δ @23401; `maos-a2a-core` trimmed to fit its existing ceiling; `xtask` ceiling re-based with a named driver. §A6 review net NOT yet run. |
| 2026-07-29 | **§A6 code review complete.** Three adversarial layers found 6 unique issues; all 6 patched, 0 deferred, 0 dismissed. Core repair: team identity now binds to the signed cert fingerprint, not only the TLS-derived host label; bootstrap reconciles both pin surfaces plus the local cert; both seams read consent+team from one snapshot; -32010 decodes at the sender; audit attribution and two vacuous Blocking legs fixed. Cert-axis mutations RED, 11 focused legs green, daemon smoke fixture corrected to declare its real cert. Named maos-a2a-core grant 4109→4271 (4187 measured + 84 reserve), operator-ratified; kernel remains 23401. Final fmt/kernel/kloc/loom/Reza/workspace gates all pass. Story closed `done`. |
