---
story_id: '20.8'
story_key: 20-8-manifest-contract-completion
created: 2026-11-03
context_commit: 8c809f9c
baseline_commit: 8c809f9c
security_sensitive: true
kernel_grant: "RULED (2026-11-03, operator, ask d_batch4; ◆ FLAG-Winston D-20-8-07 A, KS; binding at L2 dev start after re-baselining): measured at 8c809f9c from 26841 physical / 21042 tokei, +38 / +33 (KB +10 / +8, KB4 +34 / +32, K0 0 priced). Ceilings (STOP above): kernel ≤ 26917 physical / 21108 tokei; L2 kernel last. The later kernel lander re-bases; cumulative STOP above 21209 tokei, ADR-069 alarm 22000; expected cumulative with 20-1b +68, 20-4 +9, 20-11 +6 and KS +33 is 21158 (21191 at KS's ×2 ceiling)."
kloc_grant: "RULED (2026-11-03, operator, ask d_batch4; D-20-8-12 A; D02 A ×2 measured / ×3 unmeasured; re-based for L1 on 2026-11-09 current pins). L1 ceilings (STOP above; exact pins at landing; aggregate hardfail = measured + 1): maos-manifest ≤ 4812; maos-bin ≤ 36160; maos-wasm-host ≤ 2874; maos-domain ≤ 10205 (orchestrator-adjudicated overdraft, disclosed to the operator); maos-iac ≤ 8865; maos-skill ≤ 1294; xtask ≤ 44749; aggregate ≤ 199078. Every other crate 0. AC8 kernel deferred to L2: see kernel_grant."
kloc_l1_overdraft: "2026-11-09 orchestrator-adjudicated overdraft, disclosed to the operator: domain ceiling 10174 -> 10205; formatted measure 10190 from pin 10144 (+46 versus creation estimate +11); enc208 decoder and typed declarations account for growth. Aggregate ceiling remains 199078. 20-9, 20-13 and 20-14 re-base on the eventual 20-8 landed domain pin."
dev_model_used: "openai-codex/gpt-6.1-terra"
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:57-71`): a kernel FLAG-Winston row (D-20-8-07), signatures and supply-chain identity in the admission record (AC1), a third-party-reachable capability bridge across the T2 WASM boundary (AC6), and an ABI/WIT and manifest-schema bump (AC2, AC6). Blind and Edge on opus-5.5, Acceptance on opus-5.5, a Security reading on AC1.3-1.5 (record integrity, signer), AC4 (I13 refusal) and AC6 (pid/class binding, no guest-named author, no escalation), Test Infrastructure on glm-5.3 (standing policy; any session substitution recorded at review). Creation anthropic/claude-opus-5-5."
---

# Story 20.8: Manifest contract completion and WASM/TS skills

**Shared kernel STOP supersession — 2026-11-04 (ask `halt_grant`).** Historical internal STOP
21209 / 21,209 and the 20-9-only 21230 / 21,230 are superseded by **21250 / 21,250 for the whole
cumulative measured kernel ledger at each landing**. D-20-14-12 C's joint 21250 now equals that shared STOP,
not a separate exception. With ruled 20-11 O4, cumulative measured tokei is **21203 + 12 = 21215**
(physical **27032 + 16 = 27048**). ADR-069 alarm **22000** and envelope **23000** are unchanged.
Older arithmetic/rulings below remain historical; later kernel landers must re-measure and re-base.


Status: in-progress

<!-- Created 2026-11-03 at 8c809f9c (main; `git log -1 -- crates xtask` = 8c809f9c) by bmad-create-story under the lighter measured creation contract. Operator rulings in force: D01 A, D02 A, D11 A (2026-10-30); D-18-7-11 A (2026-10-26, the received item). D-20-8-01…12 are PROPOSED, not ruled. The operator rules them; a different ruling rewrites only the clauses marked with that row. No cargo ran (parent rule). Cites were re-read in a `git archive 8c809f9c` copy (`creation/cites-8c809f9c.md`), because the working tree carries Dev204L1's uncommitted 20-4 L1 edits. Kernel and kloc were measured by the prebuilt gate binary on `git archive` copies (`creation/measurements.md`); prototypes are saved as diffs and were never applied. Stale E20 §20-8 cites were corrected in place (`creation/records-owed.md` §A). -->

**Status rationale — ready-for-dev, not blocked.** Every AC except AC7 is hermetic on the dev host. That includes the WASM legs
under `--features maos-bin/wasm-host` with the guest toolchain `build-test-fixtures` uses. AC7 is the hosted T2 leg on existing
jobs. No paid key, human or external author is needed. Start gate (D-20-8-02 A): `20-1a-registry-publish-install-exit-path` is
`done`. Its install and boot re-verification path is already implemented at HEAD (C3) and is reused, not re-implemented.

> **The capability:** *a third-party author publishes a signed Spirit that declares which skills it uses, what a digest must carry for it to consume it, what a successor must preserve and what its decisions must explain. The daemon verifies the signature, refuses an invalid or tampered declaration before any pid exists, and journals the pre-admission declaration and its signer. A declaration is evidence of what was declared, not proof of admission; later refusal is separately journaled. A WASM or TypeScript Spirit can then author and propose skills through the same governed port native Spirits use.*

**Closes:**
- Epic-20 §20-8 (E20@HEAD:256-262) as amended 2026-10-30 (D01 A; PF §3.5): epic AC1 → AC1 + AC2 + AC3 + AC4 + AC5; AC2 → AC6; AC3 → AC7, plus the creation-time measurement (done; `creation/measurements.md`).
- FR8 (`prd/functional-requirements.md:37`): the four missing declarations (`skills`, `explanation_shape`, `intent_promotion_set`, `swap_invariants`) and "declarations are signed and journaled" for the signed install path.
- The received item D-18-7-11 A (`deferred-work.md:1344-1350`): the WIT `skills` import, host bridge, TS SDK and ABI docs, and consumption of the v6 skill table with the manifest `skills` field.
- I13's runtime claim at the one digest-consumption path that exists (`docs/invariants/I13.md`; C11) under D-20-8-06 A.

**Does NOT close, and says so:**
- **Enforcement of `[explanation_shape]`** (D-20-8-05 A). No frame carries an explanation body (`frame.rs:208-217`, C12). The section is validated and journaled, and the label says it is not enforced. A deferred row is filed at creation.
- **Spawned-form lifecycle and state-preserving swap** (FR55/FR10, `20-9-spawned-form-lifecycle-hooks`). AC8's check is structural (the successor still *declares* each invariant), not proof that the state survived. 20-9 consumes this story's contract (§Contract for 20-9).
- **External-author cohort evidence** (E20:262, J6) is an operator lane. The mechanism is proven with internal keys.
- **Signature for local manifests** (D-20-8-09 A). `maos run <manifest>` / door load of a local file stays policy-gated (`allow_unsigned_local`), and the record says `publisher: null`.
- **A skill-content import** (`skills.load`, D-20-8-04 C). Declared skills are checked at admission, never delivered to the guest. Admission stays operator-only (`maosctl skills approve`).
- **20-1b's admission identity** (spirit_id + publisher fingerprint, its AC10) and yank policy. Serialized, not absorbed.

## Story

As **a third-party Spirit author shipping a WASM/TypeScript Spirit, and the operator who installs it**,
I want **the manifest to carry the FR8 declarations, the daemon to verify, refuse and journal them, and my component to author and propose skills through a WIT import**,
so that **a published Spirit's contract is complete, attributable to its signer, enforced where a consumer exists, and labelled where none does.**

**Contract:**
- Epic §20-8 (E20@HEAD:256-262) as amended 2026-10-30 (D01 A, PF §3.5, §2 row 20-8). Create-story may tighten but not add uncited ACs. AC6/AC7 are the received D:1344-1350 charter. AC4/AC8 are the consumers that make AC1's declarations true rather than parse-only (rule 11; the 20-4 `[providers]` precedent, its C14).
- Tracker `sprint-status.yaml:308`; rules 9–11 (E20:59-61); proof contract `docs/testing/proof-contract.md:10-31` (one final-tree run + one plant per AC), `:33-71` (review, tiers). A manifest change widens the focused set (`:48-53`).

## Inventory at HEAD — measured, not assumed

Method: every cite re-read at `8c809f9c` in a `git archive` copy (`creation/cites-8c809f9c.md` C1–C14, F1–F12); no cargo.

| # | Charter item | Disposition | Code at HEAD | Witness / measurement | AC |
|---|---|---|---|---|---|
| 1 | `skills`, `explanation_shape`, `intent_promotion_set`, `swap_invariants` | **absent** from every crate (`grep -w` 0 outside an HSIS fixture field) | sections end at `HaltProtocolCompatibilitySection` `manifest.rs:4087-4101`; `POST_V1_SCHEMA_SECTIONS` `:336-337` | prototype `man`: +103 | AC1, AC2 |
| 2 | Unknown sections at admission | **silently ignored** | `gate_manifest` reads only named sections (`crates/maos-bin/src/admission.rs:590-725`) | an invalid `[swap_invariants]` is admitted [INFERENCE, vector owed T1] | AC1 |
| 3 | Signed manifest | **install path only** | `verify_publisher_sig` (`maos-registry/src/admission.rs:546-562`); install `install.rs:61`; every boot `:116-123`; boot loop `installed_spirits.rs:16-124` | `installed_spirit_dispatch_20_1.rs:82`, `spirit_install_20_1.rs:18` | AC1 |
| 4 | Journaled declaration | **absent** | only `VetterKey` {id, version, tier, note} (`admission.rs:63-117`, grant `:940-947`) | — | AC1 |
| 5 | Schema | v6, window [1, 6] | `crates/maos-spirit-abi/src/lib.rs:122,143,171`; gate list `xtask/src/check_manifest_schema_version.rs:37-48` | — | AC2 |
| 6 | Skill search path | fixed roots, no manifest field | `maos-skill/src/discovery.rs:38-49` | — | AC3 |
| 7 | I13 consumer check | **no production caller** | `admit_for_consumer` `maos-iac/src/adapter/distillate.rs:430-465`; consumption path `TaskAssign.prior_distillate_ref` (`orchestrator_dispatch.rs:62-140`, `adapter.rs:406`) | tests only (C11) | AC4 |
| 8 | Explanation carrier | **none** | `DecisionDispatchPayload` `crates/maos-domain/src/frame.rs:208-217` | — | AC5 |
| 9 | WIT skills import | **absent** | `wit/spirit.wit:30,378-391` (recall, context) | — | AC6 |
| 10 | In-band channel + daemon service | exists for recall/context | `recall_wire.rs:69-131`; `host_state.rs:61-82,181-211`; `recall_mediation.rs:266-359`; `spirit_session.rs:356-369,461-480` | 17-3d/18-5 witnesses | AC6 |
| 11 | Native skill port | exists (18-7) | `skill_authoring.rs:27-294`; trait `spirits/researcher/src/skill_author.rs:156-169` | `skill_port_18_7`, `live_submission_18_7` | AC6 |
| 12 | Old-guest compatibility | semver minor imports resolve; 2.0/2.1 legacy bindings | `runner.rs:242-254,359-373`; `wit_guest.rs:8-20` | `crates/maos-wasm-host/tests/wit_corpus.rs` | AC6 |
| 13 | TS SDK | template + example pinned to 2.2.0 | `templates/spirit-ts/{wit/spirit.wit,src/index.ts}`, `examples/example-spirit-ts/…` (`index.ts:2` imports `recall@2.2.0`) | — | AC6, AC7 |
| 14 | Swap reads the declaration | **cannot**: successor parsed in-kernel, bundle has no field | `upgrade.rs:307-429`; `control_block.rs:195-238`; coordinator `:151-188` | prototype `ks` | AC8 ◆ |

## Current-tree corrections — read before implementing

Full OLD → NEW: `creation/cites-8c809f9c.md` (C1–C14).

| Premise (epic / preflight / audit / tracker) | Fact at `8c809f9c` and consequence |
|---|---|
| "fields missing at `manifest.rs:228-237,2423-2454`" (E20:258, PF:242) | Those ranges are `ClassSection`'s tail and `[providers]`. The fields are absent everywhere; new sections go after `:4101` (C1). |
| `deferred-work.md:1210-1216` (E20:261, PF:243) | `:1342-1350` (C2). |
| "After 20-1" (E20:258, RA:22, RI:593) | 20-1 is split; the reused path is 20-1a's, implemented at HEAD (C3; D-20-8-02). |
| "unsigned admission" (RA:22, RI:593, S:308) | Installed packages are verified at install and every boot; local manifests stay unsigned by policy. What is missing is the *record*: no digest, signer or declaration is journaled (C4). |
| "kernel-Δ measured" / "no numeric forecast" (E20:81, PF:82,126) | Priced (C7): KS +38/+33 recommended; K0 exists. |
| I13 "v0.5 runtime" (`i13.rs:9-12`) | No production caller (C11). AC4 makes it true at the one consumption path. |
| `[explanation_shape]` "for decision.* frames" (architecture `5-spirit-abi.md:44-46`) | No carrier exists (C12): D-20-8-05. |
| `[skills.search_path] paths` (architecture `:69-70`) | Roots are fixed in `maos-skill`. A third-party manifest steering daemon filesystem reads is rejected as a shape (D-20-8-04). |

## Kernel and kloc pricing — measured at creation

**Instrument:** prebuilt `/mnt/build/cargo-maos/debug/xtask` (`check-kernel-baseline --json`, `kloc-check --json`) run from a
`git archive 8c809f9c` copy root per prototype. The untouched copy reproduces the pins exactly (26841 / 95 files / set hash
`db39bc4a…`; kernel 21042 tokei; aggregate 192519). Detail and diffs: `creation/measurements.md`, `creation/prototypes/`,
`creation/kloc-kernel-8c809f9c-and-prototypes.json`.

**Kernel (◆ FLAG-Winston, RULED; binding at L2 dev start after re-baselining; `20-8-flag-winston-decision.md`):**

| Shape | Kernel physical | Kernel tokei | Other | Diff |
|---|---:|---:|---|---|
| **KS = KB + coordinator refusal (D-07 A, recommended)** | **+38 (26879)** | **+33 (21075)** | domain +2, bin +1 | `ks-kernel-only.diff` |
| KB `swap_invariants` on the SCB + kernel parser (D-07 B) | +10 (26851) | +8 (21050) | bin +1 | `kb-kernel-only.diff` |
| KB4 all four declarations on the SCB (D-07 D) | +34 (26875) | +32 (21074) | bin +4 | `kb4-kernel-only.diff` |
| K0 none (D-07 C) | 0 | 0 | 20-9 carries it | — |

**Cumulative (stated, not assumed):** 21042 + 68 (20-1b) + 9 (20-4) + 6 (20-11) = 21125 expected. With KS: **21158**. At KS's ×2
ceiling: **21191**. Both are ≤ the 21209 STOP (51 / 18 left) and 842 / 809 below the ADR-069 alarm. No split/exchange/bump row is needed.
Physical expected 26980. **Concurrent rows created the same day (now ruled as recommended by the operator, ask d_batch4) at `8c809f9c`:** 20-14 KT +25 tokei (`20-14-flag-winston-decision.md:26-33`)
and 20-13 KT3 −1 (`20-13-…-evidence/creation/cites-8c809f9c.md:25`). With all of them measured: 21125 + 33 + 25 − 1 = **21182** ≤ 21209 (27 left).
At every ×2 ceiling the sum exceeds the STOP. As ruled with 20-7/20-4, the measured cumulative governs, and the later lander returns to the operator
above 21209 (split / exchange / bump). Risk: 20-9 (forecast kernel) inherits ≤ 27–51 lines and will need its own bump row. KB (+8) instead of KS
(+33) would leave 25 more lines (D-20-8-07 B).

**kloc forecast** (tokei; HEAD = every pin, 1 line aggregate room; recommended options):

| Crate · component | Measured / forecast | Basis |
|---|---:|---|
| maos-manifest · four sections, validator, schema list, `FR8_DECLARATIONS_SINCE_SCHEMA` (AC1, AC2) | **+104** | `man` + `gate` |
| maos-domain · `ManifestDeclarationPayload` + kind (AC1) | **+9** | `gate` |
| maos-bin · gate fields, schema-7 read, record, signer carriage (AC1) | **+95** | `gate` |
| maos-bin · skills wire arms, session deps/scopes, binder at both sites (AC6) | **+60** | `wit` |
| maos-wasm-host · WIT 2.3 skills host, wire variants, linker, major check (AC6) | **+51** | `wit` |
| xtask · ratified post-v1 list (AC2) | **+4** | `man` |
| maos-kernel-core · KS (AC8) | **+33** (physical +38) | `ks` |
| maos-domain / maos-bin · `SwapInvariantDropped`, bundle carriage (AC8) | **+2 / +1** | `ks` |
| maos-bin · `[skills] uses` admission check, refusal codes, door mapping (AC3, AC8) | +40…+88 [INFERENCE] | measurements 3, 5 |
| maos-iac · I13 check at `TaskAssign` delivery + per-recipient sets (AC4) | +40…+80 [INFERENCE] | measurements 4 |
| maos-domain · `IacBusError::IntentPromotionDenied` (AC4) | +2…+4 [INFERENCE] | `iac_bus_types.rs:15` |
| maos-skill · admitted-and-capabilities lookup (AC3) | 0…+20 [INFERENCE] | measurements 3 |
| maos-wasm-host · skills guest probe + in-file wire vectors (AC6) | +60…+120 [INFERENCE] | guests are charged (F11) |
| xtask · fixture build for the new guest (AC6) | +5…+15 [INFERENCE] | measurements 7 |
| WIT, TS template/example, docs, tests, fixtures, `spirits/researcher` | uncharged | `kloc_check.rs:294-317` |
| **aggregate** | **+359 measured; +359…+686 total** | sum |

**Envelope (D-20-8-12 A, RULED; binding at dev start after re-baselining; ×2 measured / ×3 unmeasured upper; ceilings, not reservations):**

| Crate | HEAD | Forecast | Ceiling with allowance | L1 ceiling | Operator ruling |
|---|---:|---:|---:|---:|---|
| maos-kernel-core (physical / tokei) | 26841 / 21042 | +38 / +33 | **26917 / 21108** | — (L2) | **ruled — FLAG-Winston** |
| maos-manifest | 4604 | +104 | **4812** | 4812 | ruled |
| maos-bin | 34191 | +156 meas, +40…+88 | **34767** | 34765 | ruled |
| maos-wasm-host | 2412 | +51 meas, +60…+120 | **2874** | 2874 | ruled |
| maos-domain | 10099 | +11 meas, +2…+4 | **10133** | 10129 | ruled |
| maos-iac | 8270 | +40…+80 | **8510** | 8510 | ruled |
| maos-skill | 1234 | 0…+20 | **1294** | 1294 | ruled |
| xtask | 44406 | +4 meas, +5…+15 | **44459** | 44459 | ruled |
| aggregate | 192519 | +359…+686 | **194218** (+718 + 981); hardfail = measured + 1 | 194146 | ruled |

**Interaction with other open envelopes (honest cumulative; every lander re-bases on the exact landed pins):**

| Crate | Pending ceilings over each story's own base (unlanded) | With 20-8 |
|---|---|---|
| maos-bin (34191) | 20-4 L1 ≤ 34643 (+452; worktree measured 34363), 20-11 ≤ 34698 (+1200 over 33498), 20-2 ≤ 33534 (+36 over 33498), 20-1b ≤ 34410 (shared with landed 20-1a; unlanded part not separable) | +576 → up to ≈ +2264 plus 20-1b's remainder if every ceiling were drawn |
| maos-domain (10099) | 20-4 ≤ 10115 (+16; measured +8), 20-1b ≤ 10153 | +34 |
| maos-manifest (4604) | 20-4 ≤ 4610 (measured 0) | +208 |
| maos-iac (8270) | 20-11 ≤ 8630 (+360) | +240 → up to +600 |
| xtask (44406) | 20-7 ≤ 45830, 20-2 ≤ 44860, 20-4 ≤ 44466 | +53 |
| aggregate (192519) | 20-4 L1 ≤ 193526 (+1007), 20-11 ≤ 194058 (+2232 over 191826), 20-2 ≤ 192316 (+490 over 191826), 20-1b ≤ 193768 | +1699 → ≈ +5428 if all drawn |

**Operator-ruling list (RULED; operator, ask d_batch4; as recommended):** (1) FLAG-Winston kernel row (D-20-8-07; KS/KB/K0/KB4 priced); (2)–(9) the eight crate
ceilings above and the aggregate; (10) D-20-8-01…12. Each figure lands exact, with a HISTORY row citing this key with `->` (D11 A).

## Decisions — RULED (2026-11-03; operator, ask d_batch4; all as recommended)

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-20-8-01** | Landing shape | **A** one key, two landings: L1 kernel-free (AC1–AC7, AC9 partial), L2 kernel last (AC8, AC9 close); `done` after L2 · **B** one landing · **C** split keys `20-8a`/`20-8b` | **A.** The creation contract puts kernel last, and every L1 AC is independent of the SCB field. C adds keys, which trips `check-exit-commands`/`check-decision-register` on renames (20-1a incident). Interim state: between L1 and L2 a declared invariant is journaled but a swap does not check it (labelled in docs; no release tag between the landings). | RULED 2026-11-03: **A** (operator, ask d_batch4; as recommended) |
| **D-20-8-02** | Start gate | **A** after `20-1a-registry-publish-install-exit-path` is `done`; 20-1b is not a prerequisite; `admission.rs`/`installed_spirits.rs` edits serialized with 20-1b's AC10 by one integration owner · **B** after both 20-1a and 20-1b are `done` · **C** start now in parallel with 20-1a's hosted close | **A.** The reused verification (`verify_installed`, boot loop) is 20-1a's and implemented (C3). 20-1b changes the storage identity, not the signature path. B serializes a kernel-last story behind another. C risks a second integrator on `installed_spirits.rs` while 20-1a's hosted proof is open. | RULED 2026-11-03: **A** (operator, ask d_batch4; as recommended) |
| **D-20-8-03** | Schema for the four sections | **A** `MANIFEST_SCHEMA_VERSION` 6 → 7. The four names enter `POST_V1_SCHEMA_SECTIONS` and the gate's ratified list. Below 7 they degrade with the existing WARN (`warn_n_minus_1_degradations`) and are ignored. Ratification `20-8-manifest-schema-7`; `docs-site/docs/manifest/v7.md` en + ko; STABILITY.md regenerated (prototype `man`) · **B** inside v6, no bump · **C** bump, but a schema-6 manifest declaring them is refused, not degraded | **A.** It is the 18-7 precedent (`D-18-7-02 A`, v5 → v6): B breaks the degrade rule the schema gate encodes (`check_manifest_schema_version.rs:37-48`). C turns a supported N-1 manifest into a refusal, contrary to NFR-Maint-9's documented degradation. | RULED 2026-11-03: **A** (operator, ask d_batch4; as recommended) |
| **D-20-8-04** | `[skills]` shape and consumer | **A** `[skills] uses = ["<skill id>", …]`. At admission each id must be discovered on the conventional roots and `Admitted` (18-7's reconcile), and its `required_capabilities` ⊆ the Spirit's declared actions. Refusals typed `skill_unavailable` / `skill_escalation`; ids enter the record · **B** the architecture's `[skills.search_path] paths` (Spirit-chosen roots) · **C** A + a WIT `skills.load(id)` read import | **A.** B lets a third-party manifest steer the daemon's filesystem reads. The roots are fixed today (`discovery.rs:38-49`). A gives the declaration a consumer and closes the escalation path D-18-7-03 A names ("no escalation through a skill"). C is a new read capability with no charter row (D:1344-1350 lists author/propose only). | RULED 2026-11-03: **A** (operator, ask d_batch4; as recommended) |
| **D-20-8-05** | `[explanation_shape]` | **A** validated, journaled, **not enforced**. Labelled in v7 docs and the cookbook; deferred row filed (trigger-owned by the first story that gives a Spirit-emitted decision frame an explanation body) · **B** add an explanation body to `DecisionDispatchPayload` + WIT `frames` and enforce at IAC send (frame ABI change; every 2.x guest's `frames` record changes; fixtures re-issued) · **C** enforce against `TaskComplete` output JSON | **A.** No carrier exists (`frame.rs:208-217`). B is a breaking frame change for a section no Spirit emits against. C enforces a decision contract on a non-decision frame. Rule 11: declared-not-enforced is stated, never implied. | RULED 2026-11-03: **A** (operator, ask d_batch4; as recommended) |
| **D-20-8-06** | `[intent_promotion_set]` consumer | **A** enforced at IAC delivery. A `TaskAssign` carrying `prior_distillate_ref` to a recipient with a declared set is refused typed `IntentPromotionDenied` unless the referenced distillate's `intent_lineage` ⊆ the set, reusing `AllowedPromotionSet::allows`. Sets are registered per recipient at admission; no set → today's behaviour · **B** declaration and record only; I13 runtime stays false, labelled · **C** enforce in `MediatedLogRecall` fetch | **A.** It is the only digest-consumption path at HEAD (C11). The check exists and has zero production callers. C is inert: Distillate (kind 11) is not recallable. B leaves I13's "v0.5 runtime" sentence false. | RULED 2026-11-03: **A** (operator, ask d_batch4; as recommended) |
| **D-20-8-07** ◆ | Kernel seam for `[swap_invariants]` (FLAG-Winston) | **A** KS +38 / +33: the SCB carries the declaration, the kernel successor parser reads it, and the coordinator refuses a successor that drops a predecessor-declared invariant before `on_swap_out` (`HotSwapError::SwapInvariantDropped`) · **B** KB +10 / +8: carriage only; 20-9 adds the check · **C** K0: kernel-free; 20-9 prices and lands KB/KS · **D** KB4 +34 / +32: all four on the SCB | **A.** Only the kernel parses a swap successor (`upgrade.rs:307-429`). A gives the declaration its consumer and a failing-before witness in this story (the in-process swap exists today). B lands a kernel field no 20-8 code reads. C leaves FR8's `swap_invariants` parse-only until 20-9. D carries three sections whose consumers are outside the kernel. Cumulative 21158 ≤ 21209 (§Kernel). | RULED 2026-11-03: **A** (operator, ask d_batch4; as recommended) |
| **D-20-8-08** | "Journaled" record | **A** new `GovernanceEventKind::ManifestDeclaration {spirit_id, version, manifest_sha256, publisher, declared}`, intent `governance:manifest-declaration`, written after the granted admission and before start. A failed write rolls the load back (prototype `gate`) · **B** widen `VetterKeyPayload` · **C** a Lifecycle Journal row | **A.** `GovernanceEventKind` is `#[non_exhaustive]` with no kernel match (F4), so the change is kernel-free. B changes a payload `maos-audit` and existing rows already parse. C is not the Transparency Log operators query (`maosctl audit query --intent`). The digest binds the record to the exact admitted bytes. | RULED 2026-11-03: **A** (operator, ask d_batch4; as recommended) |
| **D-20-8-09** | Unsigned local manifests | **A** unchanged: local admission stays policy-gated; the record says `publisher: null`; "signed" is the install path (20-1a) · **B** refuse every `wasm-component` manifest without a signature, local included · **C** require a signature only when FR8 sections are present | **A.** B breaks `maos run spirits/<x>/manifest.toml` and every hermetic WASM test fixture, with no signing surface for local files. C makes declaring a section a reason to refuse. The record makes the difference auditable. | RULED 2026-11-03: **A** (operator, ask d_batch4; as recommended) |
| **D-20-8-10** | WIT version and world | **A** `maos:spirit@2.3.0` adds `interface skills` to `world spirit`. 2.0–2.2 guests run unchanged (semver import resolution, `runner.rs:359-373`). Ratification `20-8-wit-spirit-2.3.0`; equivalence fixtures re-issued; 20-9 builds on 2.3.0 · **B** a separate `world spirit-skills` the host selects · **C** defer to one combined 20-9 bump | **A.** An added import is additive. B multiplies host binding selection for one import. C makes AC6/AC7 unprovable in this story and leaves the received row open. | RULED 2026-11-03: **A** (operator, ask d_batch4; as recommended) |
| **D-20-8-11** | `skills` import surface | **A** `author(document)` and `propose(target-id, target-version, diff)` → `queued {entry-id, target}`; errors `capability-denied`, `refused(code)` (the port's stable receipt code), `unavailable`. Host binds pid, class and telemetry evidence (`skill_authoring.rs:139-168`). Callable only inside `handle-frame`; counts against `MAX_RECALLS_PER_TURN` · **B** A + `own-entries` listing · **C** A + `preflight` (telemetry summary to the guest) | **A.** It is the received charter (D:1345, "author/propose"). B and C widen a third-party-reachable surface with no consumer. A guest never names an author or supplies evidence. | RULED 2026-11-03: **A** (operator, ask d_batch4; as recommended) |
| **D-20-8-12** | Per-crate envelope (D02 A applied) | **A** the envelope above (×2 measured / ×3 unmeasured) · **B** exact measured grants only; every correction re-asks · **C** allowance ×1.3 (rule 6) | **A.** 20-3b/20-3c landed at 2–3× creation forecasts. 20-1a's review added +430 inside its ceiling. | RULED 2026-11-03: **A** (operator, ask d_batch4; as recommended) |

**D-20-8-13 — RULED A, operator ask `ac13i2` (2026-11-09):** retain I2's
fail-stop transparency-log writer; write the declaration before SCB creation
and pid issuance. Serialization/validation failures are typed pre-pid
refusals. No audit-writer or kernel modification is authorized by this row.
This supersedes D-20-8-08's write ordering and rollback sentence.
Orchestrator clarification of A: pid issuance implies a prior declaration
row, not conversely. A later load or security refusal may leave that row;
the declaration is not proof of admission and the refusal must be separately
journaled. This clarification must be disclosed to the operator at hand-off.

**D-20-8-14 — RULED A, operator ask `enc208` (2026-11-09):** the
declaration's digest and publisher use colon-delimited lowercase hexadecimal
groups, following the existing public fingerprint convention. Their exact
bytes are recoverable with the documented decoder; no redaction or audit-writer
change is authorized. The prior compact-hex requirement is superseded.

**L1 rebase at `a13140b8`:** D02 A allowances on the current pins give
manifest ≤ 4,812; bin ≤ 36,160; wasm-host ≤ 2,874; domain ≤ 10,205;
iac ≤ 8,865; skill ≤ 1,294; xtask ≤ 44,749; aggregate ≤ 199,078.
Registry is zero-growth (pin 6,151); kernel is unchanged. Other pending
stories also claim envelopes on shared crates. Exact formatted L1 and review
correction measurements are recorded in the Dev Agent Record below.
The domain ceiling was 10,174 before the orchestrator-adjudicated overdraft
(disclosed to the operator): formatted 10,190 from 10,144 (+46 versus +11
creation estimate); 15 lines slack. The approved `enc208` decoder and typed
declaration fields are real production code. 20-9, 20-13 and 20-14 must
re-base their domain draws on this landing's eventual exact pin.

## Contract for 20-9 (`20-9-spawned-form-lifecycle-hooks` consumes this; stated so it is not re-decided)

1. **WIT.** The base is `maos:spirit@2.3.0` = 2.2.0 + `interface skills`. 20-9 does not re-cut 2.3.0. Its lifecycle exports
   (snapshot, swap-in, idle, consolidate, telemetry, schedule) go in a new minor (2.4.0) in a form old guests need not implement:
   the host selects bindings by export set, as `wit_guest.rs:13-20` + `run_guest!` (`runner.rs:257-320`) do for 2.0/2.1. Imports
   `recall`, `context` and `skills` are callable only while `HostState.in_export`; 20-9 rules whether its new exports set it.
2. **Ratifications.** `20-8-manifest-schema-7` and `20-8-wit-spirit-2.3.0` (`xtask/abi-ratifications.toml`). Equivalence fixtures are re-issued at
   2.3.0 by this story. 20-9 adds its own entries.
3. **Manifest.** Schema 7 (`maos_manifest::FR8_DECLARATIONS_SINCE_SCHEMA`). Types `maos_manifest::{SkillsSection, ExplanationShapeSection,
   IntentPromotionSetSection, SwapInvariantsSection}`; `SwapInvariantsSection.preserve: Vec<String>` holds HSIS invariant names (C14).
   A 20-9 manifest section is schema 8, under its own D-row.
4. **SCB and swap (D-20-8-07 A/B).** `SpiritManifestBundle.swap_invariants: Option<SwapInvariantsSection>` is set by `GatedManifest::bundle()`
   at admission and by the kernel successor parser (`load_bundle_from_file`). Under A, the coordinator refuses
   `HotSwapError::SwapInvariantDropped { invariant }` before `on_swap_out`. 20-9's spawned swap/upgrade path goes through the same
   coordinator gate and adds the state-preservation proof (FR10, HSIS criterion 2): this story's check is structural only. Under C,
   20-9 owns KB + KS (priced here).
5. **Journal.** Each pid issued by `load_admit_start` has a prior `governance:manifest-declaration` row through `emit_manifest_declaration_event`; compiled-in first-party roots outside this path do not write one. A later
   refusal may leave a declaration row and must separately record its refusal. A declaration is not proof of admission. 20-9's successor
   admission on a swap or upgrade hop writes the successor's row through the same function; there is no second writer.
6. **Admission.** `GatedManifest.{skills, explanation_shape, intent_promotion_set, swap_invariants, publisher}`. The installed signed path sets
   `publisher`; local paths leave it `None`.
7. **Skills for spawned forms.** WASM guests reach 18-7's `LiveSkillAuthorPort` via `MediatedLogRecall.skill` (one port per session, pid
   fixed by the root). 20-9 decides whether lifecycle exports may author.

## Acceptance Criteria

*(Epic §20-8 as amended 2026-10-30. Clauses marked (D-…) carry the operator-ruled recommendation (2026-11-03, ask d_batch4).
Every AC names a failing-before witness and a plant. **local** = hermetic on the dev host; **hosted** = the discipline run.)*

### L1 — kernel-free

#### AC1 — a signed manifest's declarations are verified, refused when invalid or tampered, and journaled (epic AC1; D-20-8-08 A, D-20-8-09 A) — the command

1. `crates/maos-manifest/src/manifest.rs` gains `SkillsSection{uses}`, `ExplanationShapeSection{required_fields}`,
   `IntentPromotionSetSection{allowed}` (+ `to_allowed_set()`) and `SwapInvariantsSection{preserve}`. Each is `deny_unknown_fields`
   and passes the single bounded validator (1..=32 entries, 1..=128 bytes of `[a-z0-9_.:-]`, no duplicates; prototype `man`).
2. `gate_manifest` (`admission.rs:590-725`) reads them at schema ≥ `FR8_DECLARATIONS_SINCE_SCHEMA` into `GatedManifest`
   (prototype `gate`). An invalid section is a typed `section_parse` refusal naming the section, before any pid is allocated.
3. Before the SCB is created or a pid issued, `emit_manifest_declaration_event`
   journals `governance:manifest-declaration` with `{spirit_id, version,
   manifest_sha256 = colon_hex(sha256(exact manifest bytes)), publisher, declared}`.
   Serialization and validation failures are typed pre-pid refusals; a storage
   failure follows the existing I2 fail-stop. Every pid issued by `load_admit_start` has a prior row;
   a later refusal may retain a declaration row and must be separately
   journaled. A declaration is evidence of what was declared, not admission.
4. `maos_registry::install::verify_installed` returns the verified publisher key with the text. `InstalledSpirits::boot` sets
   `gated.publisher`. Local paths leave `None`. No second signature implementation: verification stays `verify_publisher_sig`
   (`maos-registry/src/admission.rs:546-562`).
5. **The command (hosted AC7; unverified).** Publish the schema-7 TS example with `maos-spirit publish`, install and boot it, and query `governance:manifest-declaration`. Per operator O1 the shipped example and template declare three FR8 sections, not `[skills]`; the dedicated skill fixture covers that fourth section. The row must carry the installed manifest's sha256, verified publisher and those three names. Editing installed `[swap_invariants]` must make the next boot refuse with `installed_package_invalid` and write no declaration row.
6. **Local witnesses:** `crates/maos-bin/tests/manifest_declarations_20_8.rs` exercises invalid sections and local null publisher. `crates/maos-bin/tests/wasm_skill_authoring_20_8/signed.rs::installed_signed_manifest_journals_verified_publisher_and_rejects_tamper` publishes a real signed echo component, installs and boots it, verifies publisher and digest in its declaration, and rejects a post-install tamper without a new declaration. The publisher-binding removal plant was observed red and restored; hosted T2 remains separately unverified.
7. **Plant (local, one):** skip `emit_manifest_declaration_event` ⇒ the one-row assertion reds.

#### AC2 — schema 7 with documented degradation (D-20-8-03 A)

1. `crates/maos-spirit-abi/src/lib.rs:122` → 7 (doc table row). Four names enter `POST_V1_SCHEMA_SECTIONS` (`manifest.rs:337`) and
   `RATIFIED_POST_V1_SCHEMA_SECTIONS` (`check_manifest_schema_version.rs:37-48`). Add `[[ratification]] 20-8-manifest-schema-7`. Add
   `docs-site/docs/manifest/v7.md` en + ko (the v6 page pattern). Regenerate `STABILITY.md` (`check-stability-matrix`) and `gen-abi-docs`.
2. A schema-6 manifest declaring the sections is admitted. `warn_n_minus_1_degradations(6)` names the four, and the record says
   `declared: []`. Shipped manifests at schema ≤ 6 are unchanged.
3. **Failing-before (local):** a schema-7 manifest is refused `EAbiTooNew` at HEAD (`check_abi_triple`, window max 6).
   Witness: `crates/maos-spirit-abi/tests/manifest_n_minus_1_test.rs` + AC1's degrade vector.
4. **Plant (local):** drop `swap_invariants` from `POST_V1_SCHEMA_SECTIONS` ⇒ the degrade-names vector reds.

#### AC3 — declared skills exist, are admitted, and do not escalate (D-20-8-04 A)

1. At admission each `[skills].uses` id must be discovered on `maos_skill::default_search_path()` and `Admitted` after 18-7's
   reconcile (`maos_cli::subcommands::reconcile_entries` + `decided_set`, as `skill_authoring.rs:150-157`). Its
   `required_capabilities` must be ⊆ the Spirit's declared actions (`scope_action_key`). Otherwise the typed refusal is
   `skill_unavailable` / `skill_escalation`, before a pid exists. With no skill store (`NoDataHome`, D-18-7-14 A), a declared `uses` refuses typed.
2. **Failing-before (local):** at HEAD `[skills] uses = ["absent-skill"]` is admitted. Vectors in `manifest_declarations_20_8.rs`:
   absent, pending (not admitted), admitted but requiring `mcp.call` undeclared ⇒ each refused; admitted and covered ⇒ admitted, id in the record.
3. **Plant (local):** skip the capability-subset check ⇒ the escalation vector admits ⇒ red.

#### AC4 — a consumer refuses a digest outside its promotion set (D-20-8-06 A) ◆ control (rule 11)

1. Admission registers the recipient's `AllowedPromotionSet` (`to_allowed_set()`) with the IAC bus. Unload removes it. At delivery, a
   `TaskAssign` with `prior_distillate_ref` to a recipient with a set resolves the distillate receipt (`DistillateWriter`'s
   deserialization, `distillate.rs:430-465`). It is refused `IacBusError::IntentPromotionDenied` (maos-domain `iac_bus_types.rs:15`)
   unless `allows(&lineage)`. The refusal is journaled with a typed intent. Recipients without a set are unchanged.
2. **REWORDED 2026-11-09 (operator ask `i13doc` = A):** `docs/invariants/I13.md` stays UNTOUCHED, because ADR-037 `invariant-lock` treats any edit of an invariant doc as an invariant change requiring a coverage-matrix delta AND a forward-only `enforcement_cadence` phase commitment (the creation pricing missed this; hosted `invariant-lock` would go red). The witness `crates/maos-iac/tests/intent_promotion_20_8.rs::recipient_promotion_set_checks_stored_lineage_before_enqueue` is named in this story, `tests/coverage-matrix.yaml` and the dev record. The claim level is unchanged (`asserted`); no phase-commitment change is made.
3. **Failing-before (local):** `crates/maos-iac/tests/intent_promotion_20_8.rs` (new): a distillate with lineage `[consult]` referenced
   by a `TaskAssign` to a recipient declaring `allowed = ["review"]` is **delivered** at HEAD. After: refused, nothing enqueued, one
   refusal row. `[consult]` is delivered. No-set is delivered.
4. **Plant (local):** skip the check ⇒ the refusal vector delivers ⇒ red.

#### AC5 — `[explanation_shape]` is declared, never claimed enforced (D-20-8-05 A)

1. Validated (AC1.1) and journaled (AC1.3). `docs-site/docs/manifest/v7.md` (en + ko) and the cookbook state: "declared and journaled;
   not enforced — no frame carries an explanation body (`frame.rs:208-217`)". The deferred row filed at creation stays open
   (`deferred-work.md` § "Deferred from: 20-8 creation").
2. Witness: AC1's valid/invalid vectors. This is not a control, so there is no plant (rule 11: a label). A different ruling (B/C) rewrites
   this AC into a control with its own plant.

#### AC6 — WASM/TS Spirits author and propose skills through `maos:spirit@2.3.0` (received D-18-7-11 A; D-20-8-10 A, D-20-8-11 A)

1. `wit/spirit.wit`: package `2.3.0`, `interface skills { record queued; variant skill-error { capability-denied, refused(string),
   unavailable }; author; propose }`, `import skills;` in `world spirit` (prototype `wit`). Add `[[ratification]] 20-8-wit-spirit-2.3.0`.
   Re-issue the four provenance-pinned components (`tests/fixtures/wasm/equiv-fixtures.provenance.toml`; `wit/spirit.wit:21-23`)
   using the manual reproduction recipe in the provenance header. `check-equiv-fixture-provenance` green.
2. `maos-wasm-host`: `RecallRequest::{SkillAuthor, SkillPropose}`, `RecallReply::Skill`, `WireError::Refused` with code
   `refused`; `skills::Host` on `HostState` over the shared `exchange`; linker registration; `skills@` in the major check;
   instantiate error text names 2.3.0; in-file wire round-trip vectors.
3. `maos-bin`: `MediatedLogRecall.skill` + two `serve_wire` arms → 18-7's `SkillAuthorPort`. Skill calls count against
   `MAX_RECALLS_PER_TURN` (`spirit_session.rs:46,464`). `refused(code)` carries the port's stable receipt code (a
   `SkillPortError::code()` in `spirits/researcher`, reused by `skill_authoring.rs:202-214`), never `Display` text.
   `SessionDeps.skill` is passed at both construction sites (`main.rs` installed boot and `maos run`). `SessionLaunch.scopes` =
   `capabilities_required_to_scopes(&gated.caps_required)`.
4. Preserved: the host binds pid and class; the evidence is root-attached (`skill_authoring.rs:139-168`); own-namespace and
   target rules (D-18-7-03 A) via `prepare_author`/`prepare_proposal`; admission operator-only; `TaskAssign` delegation of
   `SkillAuthorSelf` still refused (`spirit_session.rs:906-910`). A guest-supplied author, pid or evidence field does not exist in the WIT.
5. **Failing-before (local, `wasm-host` feature):** `crates/maos-bin/tests/wasm_skill_authoring_20_8.rs` (new) with a Rust guest
   (extend `guests/recall-probe` or new `guests/skills-probe`). At HEAD the 2.3 guest fails to instantiate (unsatisfied
   `maos:spirit/skills@2.3.0`). After: (a) declaring `author_self = true` ⇒ one Pending entry authored by the Spirit's class
   (`maosctl skills show <entry_id>`); (b) undeclared ⇒ `capability-denied` + `skill.author.refused:capability-denied`; (c) propose
   on a skill the Spirit does not own ⇒ `refused(...)`; (d) a call outside `handle-frame` ⇒ `unavailable`; (e) the 2.0 legacy guest runs. Genuine 2.1 and 2.2 guests are not locally witnessed; compatibility remains unverified.
6. **Plant (local, one):** the daemon arm answers `Unavailable` unconditionally ⇒ (a) reds.
7. TS SDK and docs: `templates/spirit-ts/` and `examples/example-spirit-ts/` `wit/spirit.wit` copies equal the root. `src/index.ts` in the example
   imports `maos:spirit/skills@2.3.0` and authors one skill on its first turn under `[capabilities.required.skill] author_self = true`.
   The manifests move to schema 7 and declare three sections; the dedicated skill witness covers `[skills]` without making the shipped example depend on a pre-approved skill. README + `docs-site/docs/cookbook/wasm-component-spirit.md` en + ko
   cover the import, its errors, the in-export rule and the limits (no content import, operator-only admission).
   `gen-abi-docs --check` green. The example manifest change widens the focused set (AC9 commands).

#### AC7 — hosted T2 proof of host/guest admission and author/propose (epic AC3)

1. The branch-only evidence recipe builds, publishes, installs and boots the example, checks its declaration and authoring, and runs the wasm-host and equivalence legs. This hosted run remains unverified and is not installed on main.
2. **Plant (hosted, one image):** the example manifest drops `author_self` ⇒ the exit job's queue assertion fails (`proof-contract.md:14-17`).
3. External-author cohort evidence is not claimed (operator lane).

### L2 — kernel last

#### AC8 — a successor that drops a declared invariant is refused (D-20-8-07 A) ◆ FLAG-Winston

1. Apply `ks-kernel-only.diff` (re-cut, never blindly): `SpiritManifestBundle.swap_invariants` + `Default`; `load_bundle_from_file` parses
   `[swap_invariants]` under the same schema-7 rule as the gate; the coordinator refusal after halt continuity and before `on_swap_out`, with saga
   `RestorePredecessor`; `HotSwapError::SwapInvariantDropped { invariant }` (maos-domain). `GatedManifest::bundle()` carries the section. The door maps the error to typed
   `409 swap_invariant_dropped`.
2. **Failing-before (local):** `crates/maos-kernel-core/tests/swap_invariants_20_8.rs` (new; in-process hot swap, harness of the existing coordinator tests). A predecessor
   declares `preserve = ["open_pr_state", "review_queue"]` and the successor `["open_pr_state"]`; at HEAD the swap completes. After: `Err(SwapInvariantDropped { invariant: "review_queue" })`,
   predecessor restored and Running, `on_swap_out` not fired. A superset successor swaps, and the SCB carries the successor's declaration.
3. **Plant (local):** drop the check ⇒ the drop vector swaps ⇒ red.
4. Pins: `xtask/kernel-core-baseline.toml` (src_lines, content hashes of `control_block.rs`, `upgrade.rs`, `coordinator.rs`, set hash) and `xtask/kloc.toml:264`
   exact, HISTORY rows citing this key with `->`. Re-base on whichever of 20-1b/20-4/20-11 landed first. Update `20-8-flag-winston-decision.md` with the landed figure.

   **LANDED (L2, 2026-11-11, Dev208K, base `608735d2`; uncommitted, awaiting review).** The creation diff no longer applied; it was re-cut by hand at the live
   kernel 26967 / 21141 (T0 on a `git archive HEAD` copy): the shape is `ks-kernel-only.diff`'s plus the AC8.1 schema-7 read rule. Measured **+42 physical /
   +37 tokei** (kernel **27009 / 21178**, 95 files) against the priced +38 / +33: +4 / +4 over because the creation prototype parsed `[swap_invariants]` at any
   schema (a schema-6 successor's declaration would have been honoured at swap while ignored at admission); inside the ×2 ceiling re-based onto the live base
   (27043 / 21207) and the shared STOP 21250 (72 left). Domain 10217 (+2), bin 38415 (+23: `bundle()` +1, door mapping +22), aggregate hardfail 202677.
   One surface-hash refresh, the symbol the grant names: `SpiritManifestBundle` `e295e7ff…` → `ca1ac1ec…` (`check-service-boundary` flagged only it).
   Door mapping: `UpgradeFailure.conflict` + `migration_plan::upgrade_hop` ⇒ `409 swap_invariant_dropped` (the kernel error's text names the invariant).
   Witnesses: `crates/maos-kernel-core/tests/swap_invariants_20_8.rs` (3 vectors) and `crates/maos-bin/tests/one_daemon_one_door_16_1.rs::ac8_20_8_a_dropped_swap_invariant_is_a_typed_409_and_the_predecessor_keeps_serving`;
   failing-before, plants and exact figures in `20-8-manifest-contract-completion-evidence/landing-2/` and `20-8-flag-winston-decision.md`. The refusal is
   structural only; precheck does not report it and no WASM-component witness exists (two deferred rows, `deferred-work.md` § "Deferred from: Story 20-8 landing 2").

   **REVIEW CORRECTIONS (2026-11-11, Dev208K; GPT P2 ×2, GLM F-1; Main-approved).** (1) The successor parser propagates a serialization failure of `[swap_invariants]` as `UpgradeError::ManifestParse` (a scalar `swap_invariants = false` is no longer "no declaration"). (2) The aborted-swap receipt carries the dropped name as labelled, reversible colon-separated hex of its UTF-8 bytes, because the transparency log's credential redactor erases secret-shaped names (32+ hex characters, `sk-…`); `HotSwapError` keeps the raw name. Kernel **27014 / 21181** (+5 / +3, set hash `c6136060…`, no new surface hash). (3) `maosctl upgrade --policy cold-swap` refuses `409 swap_invariant_dropped` before the unload (`admission::dropped_swap_invariant`, door): bin 38475 (+29; cumulative 20-8 draw +52 on the live pins). Witnesses: `swap_invariants_20_8.rs` now 5 vectors (`a_scalar_swap_invariants_declaration_is_a_parse_refusal_not_absence`, `a_redactor_shaped_dropped_name_is_recoverable_from_the_receipt`) and `one_daemon_one_door_16_1::ac8_20_8_a_cold_swap_that_drops_a_swap_invariant_is_a_typed_409_before_unload`; red-before at HEAD and four plants in `…-evidence/landing-2/review-corrections/`. F-2 (precheck), F-3 and N-1 are deferred rows with dispositions; the reviews are persisted verbatim in `landing-2/review/`.

#### AC9 — records

1. `tests/coverage-matrix.yaml:922-928` FR8 names AC1/AC2/AC3/AC4/AC8 witnesses. FR39 (`:656-666`) and FR57 (`:834`) note the WASM reach (AC6).
   `docs-site/docs/manifest/v7.md`, the cookbook and STABILITY.md are current.
2. At `done`: `deferred-work.md:1344-1350` → `CLOSED <date> by 20-8 @<commit>` with witness. The explanation_shape row stays open (AC5).
   `check-dev-record-completeness` passes. Epic, tracker and inventory records are updated per `creation/records-owed.md`.

## Tasks / Subtasks

- [ ] **T0 — Preflight.** Operator rulings on D-20-8-01…12 recorded; tracker → `in-progress`. Confirm `20-1a` is `done` (D-02). Run `creation/measurements.md` "Not measured here" 1–2. Re-measure every cite this story touches (rule 9; `admission.rs`/`main.rs` will have moved with 20-4 L1). A measured figure above the ruled envelope, or a T0 result contradicting a ruled option, is STOP + D-row.
- [x] **T1 — Sections + schema (AC1.1, AC2).** `man` shape; v7 docs; ratification; STABILITY.md; degrade vector; plant.
- [x] **T2 — Gate + record + signer (AC1.2–1.7).** `gate` shape; `manifest_declarations_20_8.rs`; local signed install witness; plant. The TS hosted command remains AC7.
- [x] **T3 — Skills `uses` (AC3).** Admission check, refusal codes, vectors, plant.
- [x] **T4 — I13 consumer (AC4).** Bus registration, delivery check, `IntentPromotionDenied`, `intent_promotion_20_8.rs`, plant. Invariant registry documentation remains unchanged.
- [x] **T5 — Explanation label (AC5).** Docs text; deferred row stays open.
- [ ] **T6 — WIT 2.3 + bridge + TS (AC6).** Implementation and local skill witness exist; genuine 2.1/2.2 guest execution and hosted TS proof remain unverified.
- [ ] **T7 — L1 landing (D-20-8-01 A).** Exact pins inside the ruled envelope. Gates: `check-manifest-schema-version`, `check-stability-matrix`, `gen-abi-docs --check`, `check-equiv-fixture-provenance`, `abi-diff`, `kloc-check`, `check-kernel-baseline` (unchanged), `check-service-boundary`, `check-corpus`, `check-env-contract` (no new variable expected), `check-dev-record-completeness`, `check-epic-close-coherence`, `check-exit-commands`, `check-decision-register`, `prepush`. One hosted discipline run on the final tree + AC7 plant. Revert `intent-lineage-coverage-report.md` if a run modifies it. No commit or push without the operator.
- [x] **T8 — Kernel (AC8)** under the ruled row: red-before test, the shape, door mapping, plant. Done 2026-11-11 (landing 2).
- [ ] **T9 — L2 landing + records (AC9).** Kernel pins + hashes, FLAG-Winston record, `cargo test -p maos-kernel-core --lib`, coverage and docs are done (landing 2). Open: the hosted run (AC7 legs + plant), review, the ledger closure `CLOSED <date> by 20-8 @<commit>` (Main-owned), and the 20-8-owned deferred rows (D-18-7-11 charter, `[explanation_shape]`, I13 opt-in default, `precheck` verdict).

## Dev Notes

### What changes, by file

| Area | Files | AC |
|---|---|---|
| Manifest | `crates/maos-manifest/src/manifest.rs:320-337` (const, `POST_V1_SCHEMA_SECTIONS`), new block after `:4101` | AC1, AC2 |
| ABI / schema | `crates/maos-spirit-abi/src/lib.rs:33,118-122`; `xtask/src/check_manifest_schema_version.rs:37-48`; `xtask/abi-ratifications.toml` (two entries); `STABILITY.md` | AC2, AC6 |
| Domain | `crates/maos-domain/src/governance.rs:146-189` (payload + kind); `iac_bus_types.rs:15` (AC4); `hot_swap.rs:90-133` (AC8) | AC1, AC4, AC8 |
| Registry | `crates/maos-registry/src/install.rs:109-150` (return the publisher) | AC1 |
| Bin | `crates/maos-bin/src/admission.rs:63-117` (new emitter beside), `:438-497` (`GatedManifest`, `bundle()`), `:590-745` (gate, `fr8_section`), `:838-1014` (record after grant; skills check; promotion-set registration); `installed_spirits.rs:16-124`; `recall_mediation.rs:20-66,266-359`; `spirit_session.rs:60-152,356-369`; `main.rs` the two `SessionDeps` sites and the 18-7 binder; door upgrade error mapping (AC8) | AC1, AC3, AC4, AC6, AC8 |
| IAC | `crates/maos-iac/src/adapter.rs:350-420` (delivery path), `adapter/distillate.rs:430-465` (reuse); NEW `tests/intent_promotion_20_8.rs` | AC4 |
| WASM host | `wit/spirit.wit`; `crates/maos-wasm-host/src/{recall_wire.rs:69-131, host_state.rs:12-226, runner.rs:242-373, wit_guest.rs:1-11}`; guest `guests/recall-probe` or NEW `guests/skills-probe`; `tests/fixtures/wasm/*` | AC6 |
| Researcher (uncharged) | `spirits/researcher/src/skill_author.rs:137-154` (`code()`) | AC6 |
| Kernel ◆ (L2) | `crates/maos-kernel-core/src/scheduler/control_block.rs:195-238`; `lifecycle/upgrade.rs:307-429`; `hot_swap/coordinator.rs:158-188`; NEW `tests/swap_invariants_20_8.rs`; pins `xtask/kernel-core-baseline.toml`, `xtask/kloc.toml:264` | AC8 |
| TS / docs | `templates/spirit-ts/`, `examples/example-spirit-ts/` (`wit/spirit.wit`, `src/index.ts`, `manifest.toml`, README); `docs-site/docs/manifest/v7.md` + ko; `docs-site/docs/cookbook/wasm-component-spirit.md` + ko | AC2, AC5, AC6 |
| CI / records | `.github/workflows/discipline.yml:2898` (`epic-20-exit` steps); `tests/coverage-matrix.yaml:656,834,922`; `deferred-work.md:1344-1350`; `xtask/kloc.toml` rows | AC7, AC9 |

New maos-bin `src` files (none planned) must join `SCANNED_SOURCE_FILES` (`crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs`). A new
env variable (none planned) must be registered in `crates/maos-bin/src/env_contract.rs`.

### Overlap (serialize; re-measure)

- **`admission.rs` / `main.rs`** — 20-4 L1 (in the working tree now: `[providers]` gate, endpoint pins, `AdmissionDeps`), 20-1b AC10 (storage identity), 20-13 (crypto at the composition root). One integration owner per file per landing; the `gate`/`wit` prototypes are re-cut after 20-4 L1 lands.
- **`installed_spirits.rs` / `install.rs`** — 20-1a (hosted close), 20-1b (identity, yank quarantine).
- **maos-iac pin** — 20-11 (rowid windows, distillate scrub, `maos-iac ≤ 8630`). The later lander re-bases.
- **Kernel pins** — 20-1b, 20-4 L2, 20-11 L2, 20-9, 20-13, 20-14; cumulative STOP 21209 (§Kernel).
- **WIT / fixtures** — 20-9 (lifecycle exports on 2.3.0; §Contract for 20-9).
- **`manifest.rs`** — 20-4 (`ALLOWED_PROVIDER_IDS` `:2413`, tests `:4325-4399`): distinct hunks.

### Current state that must be preserved

- 20-1a: install and boot re-verification refuse any manifest/artifact/component byte change (`install.rs:116-123`). The tamper witnesses stay green.
- 18-7: native authoring/revision semantics, v2 queue, operator-only admission, refusal receipts, `TaskAssign` refusal of `SkillAuthorSelf` delegation, `NoDataHome` (D-18-7-14 A).
- 17-3d/18-5: recall/context in-band channel, one stdio buffer pair (`host_state.rs:24-27`), per-turn budget, protocol-violation closure journaling, 2.0/2.1 legacy bindings.
- 19-5: FR11 archive at every granted admission (`admission.rs:950`), migrator gate before pid (`:849-855`), halt-continuity gate first in the swap.
- 7.5a: N-1 degradation WARN; `EAbiTooOld`/`EAbiTooNew` fail-closed window.
- FR21 orchestrator distillate gate (`orchestrator_dispatch.rs:62-140`) unchanged; AC4 adds a recipient-side check, not a second sender-side one.

### Guardrails

- No kernel line without the ruled FLAG-Winston row. Any `crates/maos-kernel-core/src` change outside AC8's shape is STOP.
- No second signature implementation, no second declaration writer, no second skill port: reuse `verify_publisher_sig`, `emit_manifest_declaration_event`, `LiveSkillAuthorPort`.
- A wire request never supplies a pid, class, author or evidence. A refusal carries a stable code, never free text.
- No `unwrap`/`expect` after serde in production paths. Journal-write failures are typed refusals with rollback.
- No test spawns cargo without the expensive marker. Guest fixtures come from `build-test-fixtures`, never built inside a test.
- One cargo process per checkout; `env -u CI -u GITHUB_ACTIONS`; inherit the target directory, never override or clean it. Plants are byte-restored (`proof-contract.md:24-31`). Renaming or adding story keys is not planned. If done, run `check-exit-commands` and `check-decision-register` (20-1a incident).

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red | Leg |
|---|---|---|---|
| Declaration record (AC1) | emitter skipped | one-row assertion | local |
| Schema degrade (AC2) | name dropped from `POST_V1` | degrade-names vector | local |
| Skill escalation (AC3) | subset check skipped | escalation vector admits | local |
| I13 refusal (AC4) | check skipped | refusal vector delivers | local |
| Skills import (AC6) | daemon arm → `Unavailable` | authoring vector | local |
| Hosted T2 (AC7) | `author_self` dropped | exit job queue assertion | hosted |
| Swap invariant (AC8) | check dropped | drop vector swaps | local |

### Tests and commands

```bash
# local; one cargo process per checkout; inherit the target directory
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-manifest -p maos-spirit-abi -p maos-domain -p maos-registry -p maos-iac
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test manifest_declarations_20_8 --test installed_spirit_dispatch_20_1
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-iac --test intent_promotion_20_8
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- build-test-fixtures
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-wasm-host -p maos-bin --features maos-bin/wasm-host --test wasm_skill_authoring_20_8 --test wit_corpus
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core --test swap_invariants_20_8   # L2
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --lib                               # L2
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --test spirit_boundary_invariants   # manifest change
for g in check-manifest-schema-version check-stability-matrix check-equiv-fixture-provenance abi-diff check-corpus check-env-contract check-service-boundary check-kernel-baseline check-dev-record-completeness check-epic-close-coherence check-exit-commands check-decision-register; do env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- $g; done
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- gen-abi-docs --check
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- kloc-check --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- prepush
```

### Review plan (`proof-contract.md:33-71`)

DIFFICULT: Blind + Edge (opus-5.5), Acceptance (opus-5.5), a Security reading on AC1.3–1.5, AC4 and AC6, and Test Infrastructure (glm-5.3).
The focused suites are each touched crate plus the manifest-reader widening (every reader of the example/template manifests,
`gate_manifest`, `from_toml_str`, `spirit_boundary_invariants`, `check-corpus`), and `cargo test -p maos-kernel-core --lib` at L2.
Reviewers read the plant record and never re-run plants except for a disputed finding.

Operator review rule 2026-11-09: the implementer `gpt-6.1-terra` is never
reviewed by terra; choose reviewers from sonnet/glm/haiku/sol families.

### Previous-story intelligence (18-7, 18-5, 17-3d, 19-5, 20-1a, 20-4)

- 18-7: the native port, schema v6 bump, docs `/manifest/v6`, STABILITY.md regeneration and a measured STOP on maos-bin (D-18-7-15). Its review found pre-journal materialization and resubmission-identity escalations. AC6 reuses the port unchanged.
- 18-5: the closest WIT-minor precedent. It added `interface context` with additive import, ratification entry, version-selected legacy export bindings, and typed refusal of nested calls on legacy guests. maos-wasm-host landed +157 for a smaller surface.
- 17-3d: the in-band recall channel. "The frame pump and nested host imports MUST share this one buffer pair" (`host_state.rs:24-27`).
- 19-5: kernel cold-swap arm removed; the door gates targets; the FR11 archive is written at every granted admission. AC8's refusal sits beside the halt-continuity gate the 19-5 hot path keeps.
- 20-1a: hosted runs found faults local runs missed (header size at real component size, debug runner startup budget). Run AC7 on release builds. Renaming keys broke two doc gates.
- 20-4: `[providers]` was parse-only and never read, which counted as false (its C14). This story gives every declaration a consumer or a label.

### Git intelligence

`8c809f9c`/`16c35ef0` (20-10 hosted corrections; bin 34191, aggregate 192519), `ac1fc12a` (20-1a/20-7 cold-swap oracle), `75538bf7`,
`d3e3f586` (20-1a exit leg release build; 20-2/4/10/11 created and ruled). `manifest.rs`, `wit/`, `maos-wasm-host` and `skill_authoring.rs`
last changed at 18-7 (`271e2a33`, `a2d69a29`, `3d31b8c1`) and 19-x (`18082789`). `admission.rs` last changed at 20-3c
(`ffd19698`) and 20-1a (`850e4c00`…). Uncommitted 20-4 L1 edits are in the working tree (Dev204L1).

### Library / framework (read 2026-11-03)

No new crate. `wasmtime` 48.0.8 (locked): `bindgen!` generates `add_to_linker` per imported interface; semver-compatible minor imports resolve against the host's newer definitions (`runner.rs:359-373` relies on it).
`ciborium` 0.2.2 (wire tags `RECALL_REQUEST_TAG`/`REPLY_TAG`). TS: `@bytecodealliance/jco` 1.35.0 (`guest-types --strict`),
`componentize-js` 0.23.0 (`examples/example-spirit-ts/package.json`). `sha2`/`hex` are already maos-bin dependencies (`crates/maos-bin/Cargo.toml:100-101`).

### References

E20@HEAD:7,55-61,81,256-262; PF `epic-20-preflight-2026-10-30.md:82,126,141,242-243`; tracker `sprint-status.yaml:296-297,308-309`;
`deferred-work.md:1342-1350`; `routing-audit-2026-10-22.md:22,153`; `requirements-inventory.md:21,593`; PRD `functional-requirements.md:37`,
`developer-tool-specific-requirements.md:89,165`; architecture `5-spirit-abi.md:9-117`; `18-7-live-skill-authoring-and-revisions.md:140`;
`docs/invariants/I13.md`; ADR-027, ADR-031, ADR-045, ADR-060, ADR-069.

## Dev Agent Record

### Agent Model Used

`openai-codex/gpt-6.1-terra` (L1 implementation). The independent review must use a different model family under the 2026-11-09 rule.

`anthropic/claude-sonnet-5-5` (L2 final kernel landing, AC8, Dev208K). Reviewers are OpenAI/GLM family.

### Completion Notes List

- Implemented the L1 schema-7 declarations and degradation, pre-pid single declaration writer, verified publisher carriage, skill admission checks, I13 recipient promotion enforcement and WIT 2.3 skill bridge. The declaration row means only “declared”; I2 failure remains fail-stop and a later refusal has a separate record. This one-directional clarification of ruling A is disclosed to the operator.
- The domain ceiling is the orchestrator-adjudicated overdraft 10,174 → 10,205, disclosed to the operator; L1 measured 10,190 from 10,144 and review corrections measured 10,198. Exact re-pins are in `xtask/kloc.toml`; review aggregate 198,235 / 199,078, hardfail 198,236; kernel unchanged. xtask 44696 -> 44715 at L1 and 44715 -> 44700 after removal of unused builder rows. 20-9/20-13/20-14 must re-base on the landed domain pin.
- Local witnesses, plants, commands and limitations are in `20-8-manifest-contract-completion-evidence/landing-1/preflight.md`. The CI recipe in the evidence `ci/` directory is branch-only and unverified. Bare `abi-diff --json` is deferred to the orchestrator's clean post-commit run; local Apple SDK/cross C compiler and Node ≥22.20 are unavailable.
- The original L1 standalone workspace run exited 100 (5,841/5,847) and its prepush timed out at 3,000 s; see L1 evidence. The separately prescribed review-correction workspace run exited 100 (5,835/5,854; 19 failures), and its single frozen-tree prepush also timed out at 3,000 s without JSON. The workspace failures include verifier dependency and stale fixture preparation omissions, catalog/FKCS drift corrected afterward, and the recorded 20-7 1M timing observation. No final-tree workspace pass or completed prepush is claimed; `review/workspace-observations.md` and the same-run JUnit retain the results.
- AC8 and kernel work remain L2; explanation-shape enforcement and external-author J6 are not claimed. No status flip is made by this L1.
- Independent-review corrections and observed witnesses are recorded in `20-8-manifest-contract-completion-evidence/landing-1/review/correction-evidence.md`; genuine 2.1 and 2.2 guests, hosted T2 and clean-tree ABI diff remain unverified. I13 no-set behavior remains opt-in, accepted as a risk until a future default-deny policy story. `recipient_promotion_set_checks_stored_lineage_before_enqueue` is the L1 I13 witness; no invariant documentation was changed.
- **L2 (2026-11-11, Dev208K, base `608735d2`).** AC8 landed under FLAG-Winston KS: T0 re-cut +42/+37 versus the priced +38/+33 (the AC8.1 schema-7 read rule the creation prototype lacked), inside the ×2 ceiling and the shared STOP; kernel 27009 / 21178, set hash `aca977c8…`, one `SpiritManifestBundle` surface hash refresh. The coordinator refuses after the halt-continuity gate and before `on_swap_out` (saga `RestorePredecessor`, typed `HotSwapError::SwapInvariantDropped`, journaled `HotSwapAborted` + `hot_swap.aborted`), the successor parser reads the section at schema ≥ 7, the SCB carries the declaration, and the door maps the error to `409 swap_invariant_dropped`. Red-before: compile-red at HEAD, then the behavioural red (field + parser, no check ⇒ the drop vector swaps, exit 101). Four plants (kernel check, schema rule, door mapping, kernel check over the door) red and restored by copy-back + `cmp` + `touch`. Evidence, commands and exits: `20-8-manifest-contract-completion-evidence/landing-2/`.
- Unverified at this landing: AC7 hosted T2 legs and plant, genuine 2.1/2.2 guests, the hosted discipline run, the spawned (WASM) hop's refusal end to end (deferred to 20-9), and the dry-run `precheck` verdict (deferred row owned by this story). The status stays `in-progress`.
- **L2 review corrections (2026-11-11, Dev208K, base `a2bbd687`).** See the AC8 "REVIEW CORRECTIONS" paragraph: kernel 27014 / 21181, door cold-swap refusal, bin 38475, aggregate hardfail 202751; failing-before at HEAD (kernel: scalar vector, label/receipt vectors red; door: cold swap of the dropping successor exits 0) and plants (scalar error swallowed, raw name in the receipt, raw name under the hex label, cold-swap check dropped) red and restored by copy-back + `cmp` + `touch`. Unverified: the spawned (WASM) hop and `precheck` as before, hosted legs, `abi-diff` on a dirty tree.
- L2 verification (observed, `…-evidence/landing-2/commands.md`): kernel `--lib` 253/253; the new kernel vectors 3/3 and the door vector 1/1; workspace clippy + ratchet clean; no-default verb table 7/7; every named gate passes except `check-serde-error-handling` (the pre-existing 20-11 violation at `main.rs:11091`, owned by 20-11) and `abi-diff` (refuses a dirty tree; clean post-commit run owed); the ONE final-tree workspace nextest ran 6011 tests, 6010 passed, 1 failed (`erasure_million_20_11` AC5 1M oracle, the documented 20-7-owned timing/I/O row, not retried). `prepush` and every hosted leg were not run.
- L2 review-corrections verification (observed, `…-evidence/landing-2/review-corrections/commands.md`): kernel `--lib` 253/253; kernel vectors 5/5, door vectors 2/2; workspace clippy + ratchet clean (no diagnostic on an added line); no-default verb table 7/7; every named gate passes (`check-serde-error-handling` is green at this base) except `abi-diff` (dirty-tree refusal, clean post-commit run owed); the ONE final-tree workspace nextest ran 6018 tests, 6017 passed, 1 failed (`erasure_million_20_11` AC5 1M oracle, the documented 20-7-owned row, not retried). `prepush` and every hosted leg were not run.

### File List

- `crates/maos-manifest/src/{lib.rs,manifest.rs}`; `crates/maos-manifest/tests/{fr8_declarations_20_8.rs,manifest_n_minus_1_compat.rs}`; `crates/maos-spirit-abi/src/lib.rs`; `crates/maos-spirit-abi/tests/manifest_n_minus_1_test.rs`.
- `crates/maos-bin/src/{admission.rs,installed_spirits.rs,main.rs,operator_door.rs,recall_mediation.rs,skill_authoring.rs,spirit_session.rs,supervision.rs}`; `crates/maos-bin/tests/{hook_pump_17_4.rs,j_researcher_survey_18_3.rs,live_escape_anomaly_17_10.rs,manifest_declarations_20_8.rs,observer_activity_corpus_17_4.rs,one_daemon_one_door_16_1.rs,participant_recall_17_8.rs,recall_17_3d.rs,skill_declarations_20_8.rs,spawned_scope_17_3c.rs,wasm_skill_authoring_20_8.rs,yank_policy_20_1b.rs}`.
- `crates/maos-registry/src/install.rs` (only `verify_installed` returns the already-verified publisher); `crates/maos-cli/src/cli.rs`; `crates/maos-cli/tests/manifest_intent_20_8.rs`; `crates/maos-domain/src/{governance.rs,iac_bus_types.rs}`; `crates/maos-iac/src/{adapter.rs,adapter/distillate.rs}`; `crates/maos-iac/tests/intent_promotion_20_8.rs`; `spirits/researcher/src/skill_author.rs`.
- Review correction: `crates/maos-skill/src/{admission.rs,errors.rs}` and `tests/live_submission_18_7.rs`; `crates/maos-bin/tests/wasm_skill_authoring_20_8/signed.rs`; `.github/workflows/discipline.yml`.
- `wit/spirit.wit`; `crates/maos-wasm-host/src/{host_state.rs,recall_wire.rs,runner.rs,wit_guest.rs}`; `crates/maos-wasm-host/tests/wit_corpus.rs`; `crates/maos-wasm-host/guests/recall-probe/src/lib.rs`; `tests/fixtures/wasm/{echo_spirit_component.wasm,equiv_cosmetic_spirit_component.wasm,equiv_divergent_spirit_component.wasm,equiv_identity_spirit_component.wasm,equiv-fixtures.provenance.toml}`.
- `templates/spirit-ts/` and `examples/example-spirit-ts/` manifest, WIT, source, tests and README; the obsolete prerequisite fixtures were removed under O1. `docs-site/docs/{manifest/v6.md,manifest/v7.md,cookbook/wasm-component-spirit.md}` and their Korean counterparts; `docs-site/abi/v1/*.md`; `STABILITY.md`. `docs/invariants/I13.md` was not changed.
- Error catalog metadata and generated artifact: `xtask/error-catalog.toml`, `docs/errors/error-catalog.json`, `docs-site/route-manifest.json`, and `docs-site/docs/errors/ESkillQueue-{PayloadTooLarge,PendingLimit}.md`. The independently reviewed skill-admission limits re-pin `xtask/fkcs-baseline.toml`; any later limit change needs review.
- `tests/coverage-matrix.yaml` — FR8, FR39 and FR57 L1 witness and deferred-proof notes.
- `_bmad-output/planning-artifacts/epics/{epic-20-ship-it-w5.md,requirements-inventory.md}` — L1 status and remaining-proof clarifications.
- `.config/nextest.toml`; `xtask/{abi-ratifications.toml,kloc.toml,test-budgets.toml,src/build_test_fixtures.rs,src/check_manifest_schema_version.rs}`; `_bmad-output/implementation-artifacts/{20-8-manifest-contract-completion.md,20-8-flag-winston-decision.md,deferred-work.md,sprint-status.yaml,20-8-manifest-contract-completion-evidence/landing-1/,20-8-manifest-contract-completion-evidence/ci/}`.
- L2: `crates/maos-kernel-core/src/{hot_swap/coordinator.rs,lifecycle/upgrade.rs,scheduler/control_block.rs}`; `crates/maos-kernel-core/tests/swap_invariants_20_8.rs`; `crates/maos-domain/src/hot_swap.rs`; `crates/maos-bin/src/{admission.rs,main.rs,migration_plan.rs,operator_door.rs}`; `crates/maos-bin/tests/one_daemon_one_door_16_1.rs`.
- L2 review corrections: `crates/maos-kernel-core/src/{hot_swap/coordinator.rs,lifecycle/upgrade.rs}`; `crates/maos-kernel-core/tests/swap_invariants_20_8.rs`; `crates/maos-bin/src/{admission.rs,operator_door.rs}`; `crates/maos-bin/tests/one_daemon_one_door_16_1.rs`; `xtask/{kernel-core-baseline.toml,kloc.toml}`; `xtask/tests/{kernel_pin_content_hash_16_0.rs,recovery_lane_ceiling_rule.rs}`; docs v7 and cookbook (EN/KO); `tests/coverage-matrix.yaml`; this story, the FLAG-Winston record, `deferred-work.md`, `20-8-manifest-contract-completion-evidence/landing-2/{review,review-corrections}/`.
- L2 pins and records: `xtask/{kernel-core-baseline.toml,kloc.toml}`; `xtask/tests/{kernel_pin_content_hash_16_0.rs,recovery_lane_ceiling_rule.rs}`; `docs/ci-baselines/kernel-surface-v0.1-beta.json`; `docs-site/docs/{manifest/v7.md,cookbook/wasm-component-spirit.md}` and the Korean counterparts; `tests/coverage-matrix.yaml`; `_bmad-output/implementation-artifacts/{20-8-manifest-contract-completion.md,20-8-flag-winston-decision.md,deferred-work.md,sprint-status.yaml,20-8-manifest-contract-completion-evidence/landing-2/}`; `_bmad-output/planning-artifacts/{epics/epic-20-ship-it-w5.md,epic-20-preflight-2026-10-30.md}`.

### Change Log

- 2026-11-03: Story created at `8c809f9c` (ready-for-dev; D-20-8-01…12 PROPOSED). Inventory measured: the four FR8 sections absent; invalid sections silently unread; signature verified only on the install path; no declaration journaled; I13 check without production caller; no explanation carrier; WIT 2.2 without skills. Kernel priced by reverted prototypes (KS +38 / +33 recommended; KB +10 / +8, KB4 +34 / +32, K0). Out-of-kernel measured: manifest +104, bin +156, wasm-host +51, domain +11, xtask +4. E20 §20-8 cites corrected in place.
- 2026-11-03: Operator ruled D-20-8-01…12 all as recommended (ask `d_batch4`): each row selects A. Kernel KS and the per-crate envelope are ruled; ceilings bind at the applicable dev start after re-baselining, with L2 kernel last. Creation-time history above is retained.
- 2026-11-09: L1 kernel-free implementation, `ac13i2` and `enc208` rulings, one-directional admission clarification, exact pins and disclosed orchestrator-adjudicated domain overdraft. Hosted proof is authored, not run; L2 remains open.
- 2026-11-09: Review corrections, independently reviewed FKCS skill-limit re-pin, explicit workspace and prepush limitations, and catalog documentation.
- 2026-11-11: L2 final kernel landing (AC8): KS re-cut and landed at +42/+37, door `409 swap_invariant_dropped`, exact kernel pins and one surface-hash refresh, failing-before and plants, records. Status `ready-for-dev` → `in-progress`; hosted AC7, review and the ledger closure remain.
- 2026-11-11: L2 review corrections: ManifestParse on a malformed schema-7 `[swap_invariants]`, reversible-hex receipt reason, cold-swap door refusal (`409 swap_invariant_dropped`), kernel 27014 / 21181, bin 38475, three deferred rows (F-2 detail, F-3, N-1).
