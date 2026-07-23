---
baseline_commit: 247a1da9
---

Status: **ready-for-dev — ADVERSARIAL PREFLIGHT CLOSED 2026-07-23 (party-mode: Winston·Murat·Amelia·John·Mary·Grumbal, 3 code scouts).** Scope RATIFIED by PO (Lunarpulse) to **ONE whole story, 6 ACs** — no 13.4b split. Three scouts disproved the brief's headline premise and surfaced the real forks; all resolved below. **ZERO kernel-core-Δ @23228 holds — but not for the reason the epic sketch gave** (see Fork-1). `dev_model_used` is intentionally unrecorded: this story is not yet developed and the model is recorded at dev start (the `check-dev-record-completeness` gate only checks `done` stories). Depends on nothing — 13.4 is fully independent (11.4a/c not required; serves FR37, the only unserved PRD FR).

# Story 13.4 — FR37 vetting machinery (ADR-056)

## Story

**As** an operator running a single-org cross-team Cortex,
**I want** a signed vetting-attestation flow — issue → install → promote → revoke, end-to-end with internal vetter keys — that promotes a Spirit into the `public-vetted` trust tier as an **attestation artifact** rather than a mutable registry flag,
**so that** Diego's final promotion leg is runnable at v2.2, a Spirit's trust level is a cryptographically-verifiable claim chained to the operator root, and a revoked or expired attestation is refused at the next load with an auditable terminal cause — **without touching kernel-core**.

## PRD contract vs architecture scope (Scout C, item 5)

The **PRD FR37 contract is exactly four verbs**: *"attestation issuance / verification / journaling / revocation flow end-to-end with internal vetter keys"* (`prd/project-scoping-phased-development.md:265`; `prd-delta-full-spectrum-2026-07-06.md:48`). Accredited **external** vetters (NFR-Comp-2) are explicitly **v2.5** and out of scope.

Everything else in this story — the `public-vetted` 4th tier promotion, exact-hash binding + refuse-at-next-load, `successor_policy`, the **operator-root enrollment→issuance verify chain (AC5)**, the **four distinguishable terminal causes (AC4)**, and the `check-vetting-attestation` gate — is **architecture-derived (§15.4 / ADR-056, ADV-056-1/-2/-3)**, not PRD-mandated. It is legitimately in scope for the ADR-056 story, but the dev must know AC4 and AC5 are the design's embellishments on top of the bare contract **and** (per the scouts) the two heaviest net-new lifts. **Mary's floor (ratified):** AC5's chain is *not* splittable — verification without the enrollment-walk is verifying a signature from a key nobody vouched for, so it ships in the spine or "verification" is a lie.

---

## Resolved forks (preflight decision record — GOVERNS)

### Fork-1 — ZERO kernel-core-Δ HOLDS, but AC1's *reason* was a conflation (Scout A)

The epic sketch said *"kernel admission unchanged — the strictest-of floor already reads the tier."* **DISPROVEN as worded.** There are **two unrelated `TrustTier` enums on two axes**, with no `From`/`Into` bridge:

- **Axis A (compliance/registry tier) — has `PublicVetted`, out-of-kernel:** `crates/maos-spirit-abi/src/compliance.rs:194-203` — `{ Local=0, OrgInternal=1, PublicVetted=2, PublicUntrusted=3 }`, re-exported at `crates/maos-domain/src/ports/registry.rs:16`. **The 4th tier already EXISTS as a variant** — it is *deferred/hard-rejected*, not absent.
- **Axis B (kernel runtime sandbox tier) — NO `PublicVetted`:** `crates/maos-kernel-core/src/capability/cap_policy/decision.rs:77-87` — `{ PublicUntrusted, Known, Verified, Internal }`, hardcoded to `Verified` at every production construction site (`capability/mod.rs:415`, `mcp/mod.rs:162,320`, `inference/mod.rs:497,602`; defaulted `.unwrap_or(TrustTier::Verified)` at `security/mod.rs:317,351`). Never derived from the compliance tier.

`maos-kernel-core` does **not** depend on `maos-registry` (verified: Cargo.toml has no such dep); `admit_spirit` is called only from `maos-fkcs` + tests, never from kernel runtime. The one kernel-core file that references the compliance tier — `crates/maos-kernel-core/src/security/operator_config.rs:237` — **already** maps `"public_vetted" => TrustTier::PublicVetted` with a catch-all `_` arm, so even it needs **no edit**.

> **⚠ HARD CONSTRAINT (write this into every task):** the ONE edit that converts 13.4 into a FLAG-Winston kernel re-pin is adding a `PublicVetted` arm to `maos-kernel-core/src/capability/cap_policy/decision.rs::TrustTier` to "teach the sandbox floor about vetted." **FORBIDDEN.** Public-vetted lives and dies on Axis A. Kernel-core is untouched; the kernel-baseline gate must read `23228 == pin` at the end.

**Accurate AC1 wording:** *"The public-vetted tier and the registry strictest-of floor that reads it live entirely out of kernel-core (`maos-spirit-abi` + `maos-registry`); kernel-core does not depend on `maos-registry`, and its runtime sandbox floor uses an unrelated `TrustTier` enum with no public-vetted concept, so kernel-core is untouched — ZERO-Δ @23228."*

### Fork-2 — The strictest-of lattice collision (Scout B) — promotion gates ABOVE the floor

Un-rejecting `PublicVetted` collides with a **contradictory lattice** in the **registry** floor (out-of-kernel, `crates/maos-registry/src/admission.rs`):

- enum `Ord` (declaration order): `Local < OrgInternal < PublicVetted < PublicUntrusted` (vetted is *more* trusted),
- `strictest_of::score()` (`admission.rs:399-406`): `PublicVetted = 3, "Most restricted"` (vetted is the *least* admissible).

So today a package resolving to `PublicVetted` is rejected **for being vetted** (`admission.rs:136-140` → `PublicVettedDeferred`; also `spirit-cli/src/publish.rs:186`, `manifest.rs:2262-2267`, `ports/registry.rs:416`, and the passing test `end_to_end_test.rs:167 e2e_public_vetted_always_rejected`).

**Winston's ruling (ratified):** promotion gates **ABOVE** `strictest_of`, never inside it. A valid **VettingAttestation's presence** flips the *manifest-declared* tier from public-untrusted → public-vetted **before** `strictest_of` runs; `score()` learns that `PublicVetted`-**with-a-valid-attestation-in-hand** is admissible, deferred **without** one. `score()` never ranks "vetted" as trusted *in the abstract* — it ranks "vetted-and-attested" as admissible. `PublicVettedDeferred` becomes **attestation-conditional**. This is four out-of-kernel files (`admission.rs`, `publish.rs`, `manifest.rs`, and the `score`/branch), ZERO kernel-Δ.

### Fork-3 — Scope: WHOLE, 6 ACs (PO-ratified 2026-07-23)

Not split into 13.4b. AC5's enrollment→operator-root chain is non-splittable (it *is* verification); AC4's four-cause taxonomy is the only seam and **13.6 reaches across it** (the Reza closer judges the four causes), so a split boundary runs through an enum 13.6 needs whole — buying a rebase, not isolation. Consistent with the fewer-larger-stories preference. One coherent end-to-end capability.

### Fork-4 — Reuse map (Amelia) — BUILD little, REUSE crypto

| Need | REUSE (do not reinvent) | Evidence |
|---|---|---|
| Signed-envelope shape for `VettingAttestation` | `ComplianceClaimEnvelope` `{ signature:[u8;64], attester_pubkey:[u8;32], claim_bytes (canonical CBOR), signing_alg }`, hand-rolled fixed-array serde, sig over `sha256(claim_bytes)` | `maos-spirit-abi/src/compliance.rs:58`; verified at `maos-compliance/evaluator.rs` |
| Ed25519 sign/verify | `CryptoProvider::verify_signature` / `sign_capability_token` (`ring`) **and** `ed25519-dalek` direct | `maos-domain/src/ports/crypto.rs:63,92`; `maos-audit/sealed_export.rs:6` |
| Operator §7.3 root (signs vetter-key lifecycle) | `audit_key.rs` — `load_audit_key_seed` (path→`MAOS_AUDIT_KEY`→default), `generate_audit_key`, `derive_pubkey_fingerprint`; consumed as the operator audit key at `main.rs:7744-7754` | `maos-domain/src/audit_key.rs:31,52` |
| Revocation transport | `SignedRevocationList` / `RevocationEntry` + registry `yank.rs` (`YankCache`, `YankPoller`, FR59 5-min propagation) + TL `FrameKind::SpiritRevoked=17` | `maos-domain/src/revocation.rs:19,147`; `maos-registry/src/yank.rs`; `maos-iac/.../transparency_log.rs:90` |
| Manifest exact-hash (attestation binds to this) | `sha256(manifest_toml)` **raw bytes** — NOT the canonical form | `maos-spirit-cli/src/compliance_claim.rs:52`; `maos-compliance/runtime_context.rs:88`; field `spirit-abi/compliance.rs:164` |
| Crate home | **EXTEND** `maos-compliance` (already exists, member `Cargo.toml:9`, already deps `ring`/`sha2`/`serde_cbor`/`hex`) with a new `vetting` module — do **not** create a crate | `crates/maos-compliance/` |

**BUILD only:** the `VettingAttestation` type, the promotion flow (attestation-conditional un-defer), the vetter-key lifecycle + verify-chain, and the four-cause terminal-disposition enum.

### Fork-5 — Three named traps (Scouts B/C)

1. **`governance.rs` `VetterKeyPayload` is a NAME-COLLISION — do NOT reuse it.** `crates/maos-domain/src/governance.rs:53-65` `VetterKeyPayload` / `GovernanceEventKind::VetterKey` (ADR-045) is **per-spirit admission-decision telemetry** ("Emitted at `admit_spirit()` decision points", fields `spirit_id/version/admitted/effective_tier/journal_note`), and is **UNSIGNED** (TL-Merkle integrity only, zero `sign`/`Ed25519` in the file). It is NOT the vetter-**key** enrollment/rotation/revocation lifecycle. Author a distinct type; do not `impl` the lifecycle against this and ship an unsigned chain.
2. **ADR-036 is planning-only — hook the existing seam, don't cite a missing doc.** ADR-036 has no `docs/adr/` file (it's at `planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md:496`, listed in `docs/adr/index.md:50` as "tracked in the planning doc"). There is no literal `maosctl swap --plan` command — the real seam is `maosctl spirit upgrade --plan` (`maos-cli/src/cli.rs:683-686`; `subcommands.rs:1469-1509`, resolves/validates/hashes/persists a migration plan) + `HotSwapPrecheck` (`cli.rs:657-659`, reporting-only, ADR-036-tagged). Hook the attestation precondition there.
3. **ADR-056 must be AUTHORED; fix the stale index.** `docs/adr/` jumps **055 → 058** (056/057 slots empty; 13.3b authored 058 and ported 013/018). FR37's ADR is authoritatively **056** (`architecture-maos-minimal-opus/15-full-spectrum-v2-2.md:5,72,138`). Stale pre-renumber references map FR37→ADR-054 (e.g. `docs/adr/index.md:144`, `review-rubric-reconcile.md:59`) — fix `index.md:144` and cite **056**.

---

## Acceptance Criteria (6)

### AC1 — `VettingAttestation` type + attestation-conditional `public-vetted` promotion (ZERO kernel-Δ, Axis A only)
`VettingAttestation` = an Ed25519-signed envelope (`ComplianceClaimEnvelope` shape) binding: **manifest exact-hash** (`sha256(manifest_toml)`), **from-tier**, **to-tier** (`public-vetted`), **vetter-key-id**, **expiry**, **`revocation_semantics`**, optional **`successor_policy`**. Built in a new `vetting` module of `maos-compliance`. Promotion is the **attestation artifact**, never a registry flag. `public-vetted` is un-deferred **conditionally**: a valid attestation flips the manifest-declared tier *above* `strictest_of` (Fork-2); `PublicVettedDeferred` stays for attestation-absent packages. Touches only Axis-A / out-of-kernel files (`maos-compliance` new module, `maos-registry/admission.rs`, `maos-spirit-cli/publish.rs`, `maos-manifest/manifest.rs`). **kernel-core untouched; kernel-baseline `23228 == pin`.** No `PublicVetted` arm added to `cap_policy::decision::TrustTier`.

### AC2 — Full flow with INTERNAL vetter keys: issue → install → promote → revoke
End-to-end round-trip on a clean host with internal vetter keys, verifier **independently derived** from the issue codec (not the same function). Diego's final promotion leg is runnable. Accredited external vetters (NFR-Comp-2) are **v2.5** and explicitly out of scope — say so.

### AC3 — Upgrade semantics (ADV-056-1): exact-hash flap + `successor_policy`
Exact-hash binding ⇒ an upgrade to a new manifest version **without its own current attestation** is an **admission refusal at the floor** (the flap is the feature). `successor_policy ∈ { exact-only | re-issue-required-with-expedited-review }`. The target version's attestation is evaluated **before the migration chain starts** — folded into the existing `spirit upgrade --plan` / `HotSwapPrecheck` precondition seam (Trap-2), **not** a new `maosctl swap` command and **not** a new ADR-036 doc.

### AC4 — Expiry/revocation vs running Spirits (ADV-056-2): refuse-at-next-load + four terminal causes
v2.2 ships **`refuse-at-next-load` only** (drain-and-refuse is the named **v2.5** slot — honest zero-kernel-Δ) **plus** a **mandatory journaled observation event** when the compliance layer detects expiry/revocation while an affected Spirit is running. Audit distinguishes **four terminal causes**: `vetting-revocation` / `expiry-lapse` / `registry-yank` / `operator-local`. **Scout C reality:** none of these four is a unified audit concept today — `registry-yank` has a reusable path (`yank.rs` + `FrameKind::SpiritRevoked=17`), `expiry-lapse` has an adjacent-but-different mechanism (ComplianceClaim/cap-token expiry, keyed to a different object — do NOT conflate), and `vetting-revocation` + `operator-local` are net-new; the **four-way distinguishability rendered in the audit layer is 100% new**. Author the terminal-cause enum; each cause is independently observable and labeled.

### AC5 — Vetter-key lifecycle (ADV-056-3): operator-root-signed enrollment/rotation/revocation + verify-chain
Vetter-key enrollment/rotation/revocation are **Ed25519-signed events signed by the operator audit key (§7.3 root, `audit_key.rs`)**, journaled. `verify` walks **attestation → vetter-key enrollment → operator root**, refusing an attestation whose vetter key lacks a **journaled enrollment predating issuance**. This is the non-splittable core of "verification" (Mary's floor). Author a distinct signed lifecycle type — **NOT** the unsigned `governance.rs::VetterKeyPayload` (Trap-1).

### AC6 — `check-vetting-attestation` gate (anti-null, Murat's floor)
A new discipline gate, evidence-state-honest per the E11/§A7 pattern. Blocking hermetic legs:
1. **Round-trip** issue→install→promote→revoke, verifier independently derived from the issue codec.
2. **forged-signature** negative — reds.
3. **expired-attestation** negative — reds.
4. **forged-vetter-key** negative — an **unenrolled key with a structurally valid signature** is refused (the enrollment-predating-issuance walk fires). This is the 3am case; it MUST red on the defect.
5. **upgrade-flap control** — a new manifest version without its own attestation is refused at the floor; **and the positive** — the same version *with* a valid attestation is admitted (so the leg isn't reject-everything).
6. **`e2e_public_vetted_always_rejected` INVERTS** — the existing passing negative becomes "rejected **without** a valid attestation, admitted **with** one," so the pre-existing test is repurposed as the anti-null control for the un-defer, not deleted.
7. **four-cause distinguishability** — a planted mislabel (e.g. a revocation logged as expiry, or a yank logged as operator-local) reds the leg; the four terminal causes are provably distinct in the audit surface.
ZERO kernel-Δ; ADR-056 authored as an AC deliverable; `index.md:144` corrected 054→056.

---

## Tasks / Subtasks

- [ ] **Task 0 — Guardrails first.** Pin the kernel constraint in the dev record: NO edit to `maos-kernel-core/src/capability/cap_policy/decision.rs`; the end-state kernel-baseline is `23228 == pin`. Confirm the two-axis model in code before writing anything (`compliance.rs:194` vs `decision.rs:77`).
- [ ] **Task 1 — `VettingAttestation` type** in a new `maos-compliance::vetting` module (AC1). Model on `ComplianceClaimEnvelope`; bind `sha256(manifest_toml)`; canonical-CBOR claim bytes; `CryptoProvider::verify_signature`. Golden byte-pin for the canonical claim encoding.
- [ ] **Task 2 — Attestation-conditional promotion** (AC1/Fork-2). Un-defer `PublicVetted` in `admission.rs`/`publish.rs`/`manifest.rs`, gated on a verified attestation *above* `strictest_of`; keep `PublicVettedDeferred` for attestation-absent. Unit-pin: same package rejects without, admits with.
- [ ] **Task 3 — Vetter-key lifecycle + verify-chain** (AC5). Distinct signed type (NOT `VetterKeyPayload`); operator-root-signed enrollment/rotation/revocation; verify walks attestation→enrollment→root; refuse enrollment-not-predating-issuance.
- [ ] **Task 4 — Upgrade flap + `successor_policy`** (AC3). Hook `spirit upgrade --plan` / `HotSwapPrecheck`; evaluate target-version attestation before the chain.
- [ ] **Task 5 — Revocation + four terminal causes** (AC4). Reuse `SignedRevocationList` + yank poller for `registry-yank`; author the four-cause terminal enum; journaled running-Spirit observation event; refuse-at-next-load.
- [ ] **Task 6 — `check-vetting-attestation` gate** (AC6). All 7 legs; invert `e2e_public_vetted_always_rejected`; prove each negative reds on its own defect (planted-lie discipline).
- [ ] **Task 7 — ADR-056 + docs** (AC6). Author `docs/adr/ADR-056-fr37-vetting-machinery.md`; add to `index.md`; fix stale `index.md:144` (054→056).
- [ ] **Task 8 — Baseline + gates green.** `check-kernel-baseline` 23228==pin; `cargo fmt`; `kloc-check`; register `check-vetting-attestation` in `gate-registry.toml` + `discipline.yml` (and confirm `check-ship-gate-completeness` sees it — the D30 CI→registry meta-gap).

## Dev notes

- **The whole story is Axis-A + out-of-kernel.** Every file you touch is in `maos-compliance`, `maos-registry`, `maos-spirit-cli`, `maos-manifest`, `maos-domain` (types), `maos-cli` (the `--plan` hook), `xtask` (the gate), `docs/adr`. If you find yourself opening a file under `crates/maos-kernel-core/src/`, stop — you've taken a wrong turn (only `operator_config.rs:237` is relevant and it already works).
- **Don't bind the canonical manifest hash.** `sha256(manifest_toml)` raw is the exact-hash the design wants; `Manifest::canonical_content_bytes()` / `canonical_hash()` are for FR62 governance and cohort identity — wrong target here.
- **Reuse the envelope serde exactly.** `ComplianceClaimEnvelope` hand-rolls fixed-array serde deliberately (`[u8;64]`/`[u8;32]`); mirror that so the attestation round-trips byte-stably.
- **The gate is the story.** Prose ACs don't ship correctness — the 7 anti-null legs do. Every negative must red on its own defect; a `check-vetting-attestation` that verifies a signature it also computed is 13.3b's null control with a compliance badge (that story just shipped a leg that couldn't fail; do not repeat it).

## Gate discipline (§A7 reflex)

`check-vetting-attestation` — Blocking at v2.2 on the hermetic legs (issue-codec-independent round-trip, four adversarial negatives, upgrade-flap positive+negative, four-cause distinguishability, inverted e2e). No live-substrate dependency — the whole flow is in-process crypto + the registry admission path. Evidence state honest: absent legs emit ABSENT, never disappear.

## Dev Agent Record

### Agent Model Used
_(record the exact frontier-class model at dev start — do not fill until developed)_

### Completion Notes List

### File List

## Change Log

- 2026-07-23: Story 13.4 created + adversarial preflight closed (party-mode Winston·Murat·Amelia·John·Mary·Grumbal + 3 scouts). Scope PO-ratified WHOLE (6 ACs). Five forks resolved: ZERO kernel-Δ holds via the two-axis truth (AC1 reason corrected from the sketch's conflation); lattice collision resolved by gating promotion above `strictest_of`; reuse map (ComplianceClaimEnvelope / audit_key / SignedRevocationList / sha256(manifest_toml)); three named traps (VetterKeyPayload name-collision, ADR-036 planning-only, ADR-056 must-author). Baseline pinned 23228. → ready-for-dev.
