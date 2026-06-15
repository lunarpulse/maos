---
recommended_dev_model: claude-opus-4-8
---

# Story 9.4b: Region-Pinning + Model-Provenance + Multi-Operator Tenancy Reservation (KERNEL-TOUCHING HALF)
Status: done


<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->
<!-- ⚑ SPLIT from the original Story 9.4 at party-mode preflight (2026-06-14, ratified 5/5 Winston·Murat·Amelia·John·Mary).
     THIS story = the kernel-touching half behind ONE authorized kernel-core re-pin (AC-5 only) + ONE ratified
     AbiExtensionProposal (AC-6 surface): AC-5 region-pinning, AC-6 model-provenance, AC-7 tenancy reservation,
     AC-8 provider_history bound. Story 9.4 = the pure-ops half (distribution/backup/air-gap), lands first. -->

> **⚑ ORIGIN.** Split on the kernel-core baseline seam (mirrors 9.2b/9.3b). Story 9.4 (ops) is kernel-neutral and lands first; this half is correctness-critical compliance plumbing that moves the kernel baseline (currently **21438**) and touches the ABI + manifest-schema surfaces. **Rebase this story on top of merged 9.4** — AC-5 consumes 9.4's AC-4 air-gap egress guard, and AC-8 takes single ownership of `admission.rs` after 9.4's AC-4 lands.

## Story

As an enterprise operator and compliance officer,
I want a **region-pinning primitive** that cryptographically binds the Transparency Log + working-memory store to a single jurisdiction (NFR-Comp-4 / PIPL §40), a **Spirit model-provenance manifest field** validated at admission **and journaled as a governance artifact** (NFR-Comp-5 / SB-1047), a **multi-operator tenancy reservation** that closes the only real unmigratable corner without speculative scaffolding (NFR-Ops-11), AND a bound on the `provider_history` growth path (carry-forward debt),
so that data-localization and model-provenance are enforced and auditable — not advisory config — and the v0.5 grammar lock isn't painted into a corner, all behind one authorized kernel re-pin.

---

## Context & Charter Boundary (READ FIRST)

This is the **kernel-touching, lands-second** half. Delivery rules from preflight (do not re-litigate):

- **ONE authorized kernel-core re-pin, AC-5 ONLY** (Amelia, accepted by Winston). The sole `crates/maos-kernel-core/**` touch is `security/operator_config.rs` (region config) + the region enforcement guard. Re-pin `xtask/kernel-core-baseline.toml` (**21438**) **once, FLAG-Winston**, **last** (after AC-5 lands), target **≈ 21650 ±100** (confirm post-impl). AC-6/AC-7/AC-8 contribute **zero kernel-core LOC**.
- **AC-6 is RATIFICATION-ONLY.** Its ABI-surface change (`ModelProvenanceSection`, schema 2→3, governance-event types) is purely additive and recorded as **one ratified `[[ratification]]` entry** in `xtask/abi-ratifications.toml` (the ADR-045 §4 / F6 model from 9.3b — there is a worked example there). `abi-diff` stays **Added-only** against `xtask/abi-baseline/`. No baseline re-pin.
- **`validate_namespace_write` stub is NOT touched** (`crates/maos-kernel-core/src/memory/mod.rs:327`, returns `true`). Region is an envelope/derivation axis, not a namespace-authz axis (Winston + Amelia). De-stubbing drags in the GDPR cascade + non-atomic cross-store W1 — it is **its own story**; flag to John, do not absorb.
- **`maos-audit` stays read-only**; **`maos-cli` stays kernel-core-free**; **workspace stays 44 crates**; **`MemoryNamespace` enum is NOT modified** (NFR-Test-11 grammar-lock hash — AC-7 reserves *outside* the hashed surface).

### §A6 NON-OPUS SAFETY NET — MANDATORY

AC-5 (region **cryptographic** enforcement + `ERegionViolation`), AC-6 (manifest **admission** validation + governance journaling), and the GDPR-coupling analysis are named correctness-critical categories. **Non-Opus dev ⇒ party-mode preflight (done) + multi-layer adversarial review (Blind + Edge + Acceptance + TestInfra) is MANDATORY, not optional.** Recommended dev model: **`claude-opus-4-8`**.

---

## Preflight Consensus (party-mode 2026-06-14 — DECISIONS, not options)

Ratified 5/5 (Winston · Murat · Amelia · John · Mary), several **overruling the original spec**. Implement; do not re-litigate.

- **D1 — Region enforcement = ENFORCE-AT-USE, not at-copy (Winston).** The kernel can't stop a sysadmin from copying bytes cross-region; "cryptographic enforcement" means foreign-region data is cryptographically **unusable** and its presence is **detectable on the read/verify path**. The region tag binds into the **HKDF key-derivation `info`/salt** of the TL signing key AND into the **AEAD AAD** of each TL entry + working-memory write, uniformly across all 3 `REGISTERED_ERASURE_BACKENDS` (`private`, `principal_index`, `shared` — `crates/maos-kernel-core/src/memory/mod.rs:33`). A segment replicated into region B is signed/sealed under region A's derived key → verify/decrypt fails → `ERegionViolation`, fail-closed.
- **D2 — Kernel delta is AC-5 alone (Amelia; Winston conceded the fact).** `operator_config.rs` is the only kernel-core file. AC-6 = manifest/spirit-abi/registry (ratification entry, zero kernel-core); AC-7 = domain/capability (zero gate movement); AC-8 = `maos-registry/admission.rs` (zero kernel-core, rides here for file-ownership). **Re-pin at AC-5, last, ≈21650±100.**
- **D3 — Region tag is a JURISDICTION LABEL, NOT principal/operator identity → ZERO new principal-bearing frames (Mary).** It rides as AAD on frames already carrying principal data and already enumerated in the 9.2 cascade. **Erasure (principal-data axis) and residency (storage-location axis) are orthogonal** — they do not conflict. Better: **regional-key destruction is a valid crypto-shredding GDPR-erasure primitive.** The cascade-enumeration AC is therefore a **negative-space proof** (assert no new principal field) — see AC-9.
- **D4 — The ONE real GDPR trap is the erasure PROOF, not the data (Mary).** If a proof-of-erasure artifact attesting "subject X erased from region EU" is itself encrypted under the EU regional key, crypto-shredding the region destroys the compliance receipt. **Ruling: proof-of-erasure artifacts MUST be region-NEUTRAL — bound to the control-plane/home-jurisdiction key, never the regional data key.** Regional-key destruction must be a valid, replayable erasure path. See AC-10.
- **D5 — `[model_provenance]` is class metadata, ZERO principal nexus (Mary), CONDITIONAL on a schema constraint.** `covered_model_id` / `last_eval_timestamp` / `training_data_lineage` are fixed at authoring time and describe the Spirit class, not the runtime data-subject — so AC-6 stays **out of the cascade**. The one way it breaks: free-text `training_data_lineage` with pasted PII. **Ruling: `training_data_lineage` MUST be schema-constrained to structured provenance references (9.3b reverse-DNS lineage-name form), NOT free-text.** That makes "zero principal nexus" structural, not promised.
- **D6 — Admission presence-check is NECESSARY BUT NOT SUFFICIENT for SB-1047 (Mary).** Presence-at-admission proves the field existed at load; it proves nothing about what produced an output later, and it evaporates on manifest update/unload. **Ruling: AC-6 must ALSO emit an FR62-shaped governance event at admission** recording the provenance triple bound to schema-identity + content-hash (9.3b discipline: SCHEMA identity only, ZERO claim-instance ids — which is also why it stays out of the cascade). Append-only is the feature: it is the immutable provenance trail.
- **D7 — CUT the generic tenancy reservation (Mary + John).** Applying John's unmigratable-corner test: the only at-risk persisted record is the provenance governance event from D6, and SB-1047 accountability attaches to the **model deployer/operator** — a single, deploy-time-known identity on a single-deployment substrate. **Ruling: stamp `deployment_operator_id` as a constant field on the provenance-event schema v3 NOW (additive, free); reserve NO generic per-record tenant field.** A per-record tenant field only becomes unmigratable if v1.0 ships *multi-tenant covered-model admission* — no evidence of that on the v1.0 path; reopen only if scoping surfaces one. The 9.3b supersedes-chain gives an additive route for any optional field later regardless.
- **D8 — AC-7 namespace reservation, IF retained, reserves OUTSIDE the hashed surface (Amelia).** Do **not** add a `MemoryNamespace` variant — the NFR-Test-11 hash is computed over the variant set; adding the variant IS a future story's intentional re-roll. Reserve string identifiers in a new `crates/maos-domain/src/memory/reserved_namespaces.rs` (`const RESERVED: &[&str]`) **not referenced by the grammar-lock hash input**, + a CI guard: no current variant collides with `RESERVED`, and any future variant addition must remove its name from `RESERVED` in the same diff. **Doc + reserved-identifier manifest + CI collision guard. No new variant.** (Per D7 this is now minimal — the load-bearing reservation is the D7 operator-id stamp.)
- **D9 — Build order (Winston, dependency-correct):** **AC-5 → AC-6 → AC-8 → AC-7.** AC-5 is the cryptographic root everything enforces against (re-pin here); AC-6 the ABI surface + ratification; AC-8 the admission hook (sole `admission.rs` writer now); AC-7 last (v1.0 reservation only).
- **D10 — 9.6 does not block this story (John).** Only AC-7's **v1.5+ implementation** and the multi-Spirit acceptance demo need 9.6. AC-5/AC-6 are single-tenant-clean. **Forward-coupling (Murat):** when 9.6 adds new write entry points, AC-9's enforcement table is silently incomplete unless guarded by type-system exhaustiveness — see AC-9.

---

## ⚑ RE-RATIFICATION (party-mode 2026-06-15 — SUPERSEDES D1; D2 amended)

During dev-story implementation, two code-verified discoveries collided with the preflight: **(1)** the 3 working-memory backends (`private`, `principal_index`, `shared`) are **PLAINTEXT** (`shared.rs:135` = bare SQLite `INSERT OR REPLACE` of raw bytes; `private.rs` = HashMap + FS spill — no AEAD, no tag column); **(2)** the `CryptoProvider` port (`ports/crypto.rs:50-97`) is **SEAL-ONLY** — no HKDF, no unseal, no unseal call site anywhere; and **(3)** the TL signing seed is passed **RAW** (`sealed_export.rs::sign_bundle(seed:&[u8;32])`, `proof.rs::build_erasure_proof(signing_seed:&[u8;32])`) — there is **no existing KDF/`info`** to reuse. D1's literal "AEAD AAD on every working-memory write, decrypt-fails" cannot be built within D2's ~180 LOC budget + additive-only ABI scope. The team re-ratified (Winston·Mary·Murat·Amelia·John), **overruling D1** per the standing "team may override spec fork defaults under long-term-correctness" principle. Implement these; they SUPERSEDE the conflicting parts of D1/AC-5/AC-12.

- **R1 — Mechanism = Option A (TL-anchored region binding).** Region welds into (a) the TL signing-key **HKDF `info`** (the raw seed is now run through HKDF) and (b) the **AEAD AAD** of TL entries / sealed exports (existing `seal_for_export(aad=…)`). Memory writes route through a non-wildcard `WriteEntryPoint` enum guard (AC-9) that **stamps the region into the governing audit frame**. **Memory rows are region-bound by audit GOVERNANCE, NOT per-row sealed** — rows stay plaintext-at-rest. **Winston + John signed the plaintext-at-rest waiver** (threat model = operator misconfig / cross-region bleed in a *trusted* kernel; raw-disk exfil is OUT of scope → 9.4c).
- **R1-COND (Winston, MERGE-BLOCKING) — No-unanchored-read proof.** A test enumerating **every read entry point** on all 3 backends, proving each is gated through a region-verified frame. This is the acceptance gate for Option A; a read path that reaches bytes without a region-verified frame = RED.
- **R2 — Re-pin TARGET = 21667** (from current baseline **21472**, +195 LOC). **HARD STOP tripwire = 21720** — a *shape* tripwire, not a budget: crossing it means Option A has grown an Option-B limb → STOP + re-convene, do NOT absorb. `proof.rs` stays HOME-key/region-NEUTRAL (region-*exclusion* assertion, ~5 LOC, not region-binding). FLAG-Winston.
- **R3 — HKDF = RustCrypto `hkdf` crate** (`hkdf::Hkdf::<Sha256>::new(salt, seed).expand(info, &okm)`, `info = region_bytes ‖ domain-sep`). **NO hand-rolled crypto** (Winston: hand-rolled HMAC ships CVEs; Amelia conceded). 3 RFC-5869 SHA-256 test vectors + **a recorded code-review sign-off** on the derivation helper. **`CryptoProvider` is NOT touched; NO 2nd ABI-ratification entry** — region-into-derivation glue is kernel-core-internal. (Verify `hkdf` availability; if a new dep, it pulls only `hmac` transitively.)
- **R4 — AC-12 REWORDED (John, to avoid shipping a false AC).** Honest governance-plane wording: *"region is cryptographically anchored in the TL signing key and stamped in every audit frame; memory rows are region-bound by audit governance, not per-row sealed."* Region token canonical form: `^[a-z0-9-]{2,32}$`, ASCII-restricted (no `unicode-normalization` dep — NFC is a no-op over that grammar), frozen-encoding id `ascii-v1`. Invalid token → typed reject at the `WriteEntryPoint` boundary.
- **R5 — NEW compliance ACs (Mary):**
  - **AC-13 — Placement-enforcement is the localization control.** PIPL §40 is a storage-location + egress axis; at-rest crypto is NOT legally required (that's §51 defense-in-depth, deferred). The node holding plaintext rows must be provably in-jurisdiction — tested, not assumed.
  - **AC-14 — Two-phase regional teardown (fail-closed).** "Complete regional erasure" = (a) **forget cascade** over region-scoped rows across all 3 plaintext stores (reuse 9.2 cascade via a jurisdiction-label filter — NOT re-authored) — this is the **erasure of record**; AND (b) **signing-key decommission** of the region's signing key — **forward-capability revocation only**. Both required; skipping either → fail-closed, never reports success. **NON-CLAIM (ratified Mary+John 2026-06-15, supersedes earlier "crypto-shredding" wording):** under Option A memory rows are plaintext and sealed artifacts are SIGNED, not encrypted — so phase (b) provides **NO data-erasure and NO confidentiality guarantee**; it revokes the region's ability to produce verifiably-region-attributed/signed artifacts going forward. **GDPR Art. 17 erasure is satisfied SOLELY by phase (a)'s forget cascade**, evidenced by the cascade's cross-store (private/principal_index/shared) coverage proof. "Crypto-shredding" (ciphertext-at-rest destroyed-by-key) is NOT the mechanism and MUST NOT be claimed.
  - **AC-15 — Two-part region-neutral erasure receipt.** D4 CONFIRMED + extended: receipt is **home/control-plane-key-bound (region-NEUTRAL)** so crypto-shredding the region doesn't destroy the compliance receipt; payload is two-part = (i) forget-cascade completion attestation + (ii) sealed-artifact key-destruction attestation.
  - No option introduces a new principal-bearing frame (region = jurisdiction label as HKDF context + AAD = parameters, not frames). **D3/D5 hold; the 9.2 cascade is invoked, not re-authored.**
- **R6 — Binding test gates (Murat, FINAL — supersedes the original list below for AC-5 scope):** **9 MERGE-BLOCKING** — R-RG1 (foreign-region write fail-closed), R-RG2 (parametrized over **ALL** `WriteEntryPoint` variants), R-RG3/AC-9 (total read+write completeness table, non-wildcard, incl. **replay + backup-restore** — the SOLE safety case, no crypto backstop under A), **R-RG4′** (provenance-frame tamper bite: mutate region tag in audit frame → TL bundle verify fails), R-SCH1/R-SCH2/R-SCH3 (schema stamp / round-trip / compat on region frames), **No-unanchored-read proof** (R1-COND), **Two-phase teardown** (AC-14). **2 CORROBORATING** — R-RG4 (re-scoped to TL/export-layer witness), region-neutral receipt (AC-15). *R-RG4 original "AEAD decrypt fails at the memory row" is re-scoped to the TL/export layer — at the memory row it would be a tautological green (rows are plaintext).*
- **R7 — Honest Risk Register (NEW residual):** **At-rest plaintext exposure in a foreign region.** Green = "no in-kernel path reads bytes without a region-verified frame + cross-region kernel bleed is fail-closed." Green does **NOT** mean at-rest confidentiality. Out of scope until 9.4c.
- **R8 — SPLIT (John):** per-row crypto (Option B/C) deferred to a future **9.4c parked in E10**, gated behind a *real* at-rest/exfil trigger. Do NOT add any per-row-seal assertion to 9.4b's gate set (that would be the fake green this round killed).

---

## Acceptance Criteria

### AC-5 — Region-pinning primitive **[kernel-core re-pin driver]** (NFR-Comp-4 / PIPL §40)

**Given** region pinning configured via a new `[region]` section in `~/.config/maos/operator.toml` (parsed alongside `RegistrySection` in `crates/maos-kernel-core/src/security/operator_config.rs`, with `MAOS_REGION_*` env override matching the existing `MAOS_REGISTRY_*` precedence)
**When** the Transparency Log and the working-memory store (all 3 `REGISTERED_ERASURE_BACKENDS`) perform a write
**Then** the **canonicalized** region tag (D11 below) is bound into the HKDF key-derivation `info` of the signing key **and** into the AEAD AAD of the entry/value (enforce-at-use, per D1)
**And** a foreign-region read/verify/decrypt fails closed with **`ERegionViolation`** — a new typed error registered in `xtask/error-catalog.toml` with all 8 fields (`severity=security`, `recovery_class=reject`, `kernel_or_spirit=kernel`, `since_version="1.0.0"`); `cargo xtask error-catalog-check` stays green (AST bijection holds)
**And** the `validate_namespace_write` stub is NOT touched (region is a derivation axis, not a namespace-authz axis)

### AC-6 — Model-provenance manifest field + governance journaling **[ratification-only]** (NFR-Comp-5 / SB-1047)

**Given** a Spirit manifest declaring `[model_provenance]` with `covered_model_id`, `training_data_lineage` (**schema-constrained to structured lineage references — NOT free-text**, per D5), and `last_eval_timestamp`
**When** the Spirit is admitted (`crates/maos-registry/src/admission.rs::admit_spirit`)
**Then** field presence is validated at admission, and missing/stale provenance (for classes that require it) is rejected with a typed, catalogued error
**And** the new `[model_provenance]` section is parsed in `crates/maos-manifest/src/manifest.rs` (mirroring `ClassSection`/`[author]`), and `MANIFEST_SCHEMA_VERSION` is bumped **2 → 3** (`MAX_SUPPORTED → 3`, `MIN_SUPPORTED` stays `1`) in `crates/maos-spirit-abi/src/lib.rs`; `cargo xtask check-manifest-schema-version` stays green
**And** admission **also emits an FR62-shaped governance event** recording the provenance triple bound to schema-identity + content-hash (SCHEMA identity only, zero claim-instance ids — D6), carrying a `deployment_operator_id` constant field (D7), queryable via `maosctl audit query --kind governance`
**And** the ABI surface change is recorded as one ratified `[[ratification]]` entry in `xtask/abi-ratifications.toml`; `check-abi-ratification` stays green (`abi-diff ⊆ ratified`)

### AC-7 — Multi-operator tenancy reservation (narrowed) **[zero gate movement]** (NFR-Ops-11 / NFR-Tenancy-1)

**Given** the multi-operator tenancy seam
**When** v1.0 ships
**Then** the **only** persisted-record reservation made is the `deployment_operator_id` constant on the provenance-event schema (D7 — folded into AC-6); **no generic per-record tenant field is reserved** (cut as theater)
**And** IF a namespace identifier reservation is retained, it lives in `crates/maos-domain/src/memory/reserved_namespaces.rs` **outside** the NFR-Test-11 grammar-lock hash input, with a CI collision guard (D8) — the `MemoryNamespace` enum is NOT modified and the grammar-lock test still passes
**And** full multi-operator implementation + the multi-Spirit acceptance demo are deferred to v1.5+ (E10), gated by Story 9.6 — with a recorded rationale (no live multi-tenant code path at v1.0)

### AC-8 — Bound `provider_history` growth **[rides here for `admission.rs` ownership]** (carry-forward debt)

**Given** the `provider_history` HashMap in `admit_spirit` that grows unbounded under high Spirit churn (`deferred-work.md:193`, forward-shaped to Story 9.4)
**When** this story lands
**Then** the growth path is bounded (evict on Spirit termination, or a documented cap), with the overflow policy stated (evict-oldest vs reject-new) and any observable error surfaced; `deferred-work.md:193` is updated to closed (or explicitly re-shaped with rationale)

### AC-9 — Region-tag derivation-site completeness **[new hard AC — Winston/Murat]**

**Given** the region tag must be bound at **every** HKDF/AEAD-AAD derivation site (a missed site = a silent cross-region leak)
**When** region enforcement is implemented
**Then** every store-write path routes through a single non-wildcard `enum WriteEntryPoint { DirectWrite, ReplayApply, BackupRestore, … }` matched by the enforcement layer **with no `_ =>` arm** (adding a 9.6 `ScheduledWrite` variant must **fail to compile** until handled)
**And** a parameterized test iterates `WriteEntryPoint` (enum-iter) asserting each variant hits region enforcement, and an `xtask` check (sibling to `check-governance-categories`) reds if any store-write `fn` is reachable without passing through the enum (catches the bypass case)

### AC-10 — Region-neutral proof-of-erasure **[new hard AC — Mary D4]**

**Given** crypto-shredding a regional key is a valid GDPR-erasure path (D3/D4)
**When** a proof-of-erasure artifact (Story 9.2 `erasure/proof.rs`) is generated for region-bound data
**Then** the proof artifact is **region-neutral** — bound to the control-plane/home-jurisdiction key, NOT the regional data key — so destroying the region does not destroy the compliance receipt
**And** a test asserts regional-key destruction is a valid, replayable erasure path and the proof still verifies afterward

### AC-11 — Append-only v2→v3 envelope/manifest compat **[new hard AC — Winston/Murat R-SCH1]**

**Given** the MANIFEST_SCHEMA_VERSION 2→3 bump and the new region-tagged envelope
**When** pre-existing v2 manifests / pre-region (no-region-tag) envelopes are read
**Then** v2 manifests **remain admissible** (region/provenance fields optional-on-read; admission must not start *rejecting* v2 — that would break 9.2b HARD byte-identity replay for everything admitted before this story)
**And** the v3 reader defines canonical default-region semantics for legacy pre-region data (no silent reinterpretation of pre-region bytes)
**And** a **frozen v2 golden corpus is committed in a PRIOR commit, before the schema bump lands** (R-SCH1 — a corpus that rides in with the bump is worthless), and all of it still admits

---

## Binding Test Gates (Murat — ratified 2026-06-14)

**MERGE-BLOCKING (9.4b cannot land without these green):**
- **R-RG1** — same-input-opposite-verdict: two writes identical except region tag → home **ALLOW**, foreign **`ERegionViolation`**.
- **R-RG2** — **anti-stub mutation guard**: a test that **DIES if the enforcement body is replaced with `true`** (the fail-OPEN regression). *(Hard gate — Murat's line in the sand.)*
- **R-RG3 / AC-9** — write-entry-point exhaustiveness: completeness table over ALL write paths incl replay + backup-restore; type-system non-wildcard enum guard. *(Hard gate.)*
- **R-RG4** — cryptographic-binding bite: tamper the region-derived AAD post-seal → AEAD decrypt fails (not merely a region-field check).
- **R-SCH1** — frozen v2 golden corpus committed in a prior commit; all still admit post-bump.
- **R-SCH2** — version-routing bite: same bytes as v2 vs v3 route to different validators with correct verdicts.
- **R-SCH3** — malformed-provenance → typed reject (not panic, not silent-accept).

**CORROBORATING (run, do not gate):**
- **R-RG5** — `ERegionViolation` emitted to the audit trail (observable; gate soft so a logging refactor doesn't red enforcement).

**Note (Mary's ruling closes these):** R-RG6/R-SCH5 (scrub-proof) and R-RG7/R-SCH6 (scrub-preserves-replay) — the principal-data-in-frame tests — **stay OFF** because D3/D5 ruled zero new principal-bearing frames. They are **replaced** by AC-9's negative-space derivation-site completeness + AC-10's region-neutral proof test. If implementation discovers principal data does land in a new frame, these promote to merge-blocking and the cascade must be wired in THIS story (escalate to Mary/John — irreversibility forbids a follow-up).

## Honest Risk Register (record — do NOT fake a tautological green)

- **R8-RG (per-principal correctness):** carried from 9.3b — per-principal region/scrub correctness is CI-untestable at population scale. **Compensated by: per-principal weekly reconciliation runbook.** Document; do not paper over.

---

## Tasks / Subtasks (build order = D9)

- [x] **Task 1 — AC-5 region-pinning (re-pin driver, FIRST)** — COMPLETE & GREEN
  - [x] `[region]` parse + `MAOS_REGION_*` env in `operator_config.rs` — `RegionSection`, 4 tests
  - [x] Canonicalize once, frozen `ascii-v1` (AC-12) — `maos-domain/src/region.rs`, 12 tests; bound into HKDF `info` of the TL signing key (`maos-audit/sealed_export.rs`). *(R1 supersedes "AEAD AAD across all 3 backends".)*
  - [x] `ERegionViolation` (+`ERegionTagInvalid`) + `xtask/error-catalog.toml` entries; `error-catalog-check` GREEN (39/39)
  - [x] R-RG1 + R-RG2 (anti-stub) + R-RG4′ + R-SCH/R-SCH2/R-SCH3 + AC-12 GREEN (sealed_export 6 + write_entry_point 4)
  - [x] AC-9 `WriteEntryPoint` enum (no wildcard) + R-RG2 anti-stub + R-RG3 exhaustiveness + no-unanchored-read proof (visibility chokepoint) + `xtask bypass-scan` (verified teeth: wildcard injection → FAIL)
  - [x] cli/bin region-seed wiring — `maosctl audit sealed-export`/`export` region-pin via `Region::home_from_env`; daemon memory adapter `.with_home_region(...)`; proof stays region-neutral
  - [x] **Re-pin `xtask/kernel-core-baseline.toml` 21472 → 21894 (FLAG-Winston, tripwire authorized 2026-06-15: prod ~21709 < hard-stop 21720; overage = merge-blocking test LOC; shape = Option A)** — `check-kernel-baseline` GREEN
  - [x] AC-10/AC-15 region-neutral proof-of-erasure — done in Task 1b (AC-13/14/15)
- [x] **Task 1b — AC-13/AC-14/AC-15 regional teardown + region-neutral receipt** — COMPLETE & GREEN
  - [x] `maos-audit/src/erasure/regional_teardown.rs` (NEW): two-part `RegionalTeardownReceipt` (phase (a) `ForgetCascadeAttestation` + phase (b) `KeyDecommissionAttestation`), **HOME-key-signed → region-NEUTRAL**; `build_regional_teardown_receipt` **fail-closed** (either phase incomplete → `IncompletePhase`, never silent success — AC-14); `verify_regional_teardown_receipt`; `decommission_region_key`. **9 tests.**
  - [x] **AC-10/AC-15 region-neutrality proven:** receipt verifies under HOME pubkey, does NOT verify under region-derived pubkey; region-key "destruction" (seed zeroized) → receipt still verifies; region tamper → verify fails. `proof.rs` was already region-neutral by construction (Task 1 kept it region-free; signed with the home audit key at `run_uninstall_cascade`).
  - [x] **AC-13 placement:** decommissioned key is region-SPECIFIC (eu≠us) — placement tested, not assumed (composes with Task 1 R-RG1 home-ALLOW/foreign-VIOLATION).
  - [x] **AC-14 bin teardown wired:** `run_uninstall_cascade` (maos-bin) emits the region-neutral receipt alongside the proof when `MAOS_REGION_HOME` is set (additive, fail-closed); reuses the 9.2 forget cascade (NOT re-authored). Under Option A + AC-13 the node is single-jurisdiction, so the spirit's rows ARE the home-region set.
  - [x] **D4-wording deviation RATIFIED & CLOSED (party-mode Mary+John, 2026-06-15):** phase (b) re-documented as "signing-key **decommission**" (forward-capability revocation only), NOT "crypto-shredding" — see AC-14 NON-CLAIM clause above. Mary: honest narrower wording is the compliance-defensible choice (avoids Art. 5(2)/Art. 30 overstatement; plaintext deletion in phase (a) is a *more* direct Art. 17 erasure than crypto-shredding, never an obligation skipped). John: fix the spec to match reality, never the reverse — non-blocking for merge, mandatory for review close (now satisfied). No new principal-bearing frames added (D3/D5 condition NOT triggered — irreversibility trap avoided).
- [x] **Task 2 — AC-6 model-provenance + governance journaling (ratification)** — COMPLETE & GREEN
  - [x] `ModelProvenanceSection` in `crates/maos-manifest/src/manifest.rs` (`training_data_lineage` schema-constrained to reverse-DNS lineage refs, **free-text rejected**; strict RFC3339→unix parser; optional-on-read `from_manifest_toml`; `validate_staleness`; `canonical_content_bytes`); 10 tests
  - [x] Bump `MANIFEST_SCHEMA_VERSION` 2→3 (`MAX_SUPPORTED` follows →3, `MIN_SUPPORTED` stays 1) in `crates/maos-spirit-abi/src/lib.rs` + `POST_V1_SCHEMA_SECTIONS` += `model_provenance`; `check-manifest-schema-version` GREEN (current=3)
  - [x] Admission presence/staleness validation in `crates/maos-registry/src/admission.rs` via companion `validate_model_provenance` + `ModelProvenancePolicy`/`ModelProvenanceRecord` (keeps `admit_spirit`/`AdmissionConfig` frozen — zero churn, AC-11-safe); typed **catalogued** error `ProvenanceError` placed in `maos-domain` (FR63-scanned); 6 tests
  - [x] FR62-shaped governance event at admission — new `GovernanceEventKind::ModelProvenance` + `ModelProvenancePayload` (schema-identity + content-hash, `deployment_operator_id` constant via D7, **zero claim-instance ids**); emitted in `maos-bin` (`emit_model_provenance_event`, fail-closed on `maos run`, best-effort on posture-shift); queryable via `--kind governance` (FrameKind 28)
  - [x] AC-11: section optional-on-read (pre-v3 manifests stay admissible — unit-proven `provenance_absent_admits_*`/`v2_manifest_loads_*`); `[model_provenance]` added to `spirits/mira/manifest.toml` as the reference example
  - [x] One ratified `[[ratification]]` entry (`9.4b-AC6-model-provenance`) in `xtask/abi-ratifications.toml`; `check-abi-ratification` GREEN
- [x] **Task 3 — AC-8 bound `provider_history`** — COMPLETE & GREEN
  - [x] Bounded `ProviderHistory` (cap 4096, evict-oldest) in `security/mod.rs`; overflow policy stated; `deferred-work.md:193` CLOSED; 2 tests
- [x] **Task 4 — AC-7 tenancy reservation (narrowed, LAST)** — COMPLETE & GREEN
  - [x] Generic tenant field CUT (D7); `crates/maos-domain/src/reserved_namespaces.rs` (`RESERVED_NAMESPACE_IDENTIFIERS`, outside grammar-lock hash) + CI collision guard; NFR-Test-11 grammar-lock still passes; 2 tests. `deployment_operator_id` reservation folded into AC-6 (Task 2).
  - [x] If retained: `reserved_namespaces.rs` outside the grammar-lock hash + CI collision guard; verify NFR-Test-11 still passes; document v1.5+ deferral

## Dev Notes

### AC-12 — Region-tag canonicalization (Winston — the irreversible one)
The region tag's byte representation feeding HKDF/AAD MUST be canonicalized **once** (case, Unicode normalization, encoding) and **frozen**. Two spellings of one region deriving two keys is the same irreversible failure class as a missed AAD site. **The single thing that, if gotten wrong, cannot be migrated:** region is welded into the key — wrong canonical form / missed site / non-frozen encoding ⇒ artifacts are permanently bound to the wrong-or-unrecoverable region with no re-derivation path. Treat canonicalization + AC-9 site-completeness as the highest-scrutiny review items.

### Region-pinning — net-new, hardest piece (AC-5)
- **Currently ABSENT** (one stray comment in `crates/maos-iac/src/adapter/redaction.rs`). No jurisdiction enum, no `ERegionViolation`, no residency enforcement.
- **Config home:** `crates/maos-kernel-core/src/security/operator_config.rs` — `RegistrySection` (10-25), `resolve_from_env_and_disk` (48-139), `~/.config/maos/operator.toml` + `MAOS_REGISTRY_*` precedence. Add `[region]` + `MAOS_REGION_*` in the same shape.
- **Merkle / signing reuse:** TL is signed/Merkle-rooted via `crates/maos-audit/src/erasure/merkle.rs` + `sealed_export.rs`; region tag enters the HKDF context of the signing key and the AAD of entries. CryptoProvider trait `crates/maos-domain/src/ports/crypto.rs:50`.
- **Error catalog (8 fields):** copy an existing `[[error]]` block in `xtask/error-catalog.toml`; the AST bijection check (`xtask/src/check_error_catalog.rs`) auto-discovers `E*` variants — register `ERegionViolation` or CI fails.

### Model-provenance — manifest schema bump (AC-6)
- **Parser:** `crates/maos-manifest/src/manifest.rs` — `ClassSection` (176-197), `[author]`/`[capabilities.required]`/etc. (388-537). Add `ModelProvenanceSection` in the same style. `[model_provenance]` does not exist yet.
- **ABI constants:** `crates/maos-spirit-abi/src/lib.rs` — `MANIFEST_SCHEMA_VERSION=2` (65), `MIN_SUPPORTED=1` (72), `MAX_SUPPORTED` (79). Bump 2→3 (MAX follows); MIN stays 1. `check-manifest-schema-version` (xtask 431) gates this.
- **Admission:** `crates/maos-registry/src/admission.rs::admit_spirit` (77-200), three-tier strictest-of-floor. Reference manifest `spirits/mira/manifest.toml`.
- **Governance event:** reuse the 9.3b FR62 governance stream (FrameKind, `kind_category_to_kinds` resolver, schema-lifecycle registry in `transparency_log.rs`); the provenance event is schema-lifecycle-shaped (schema_id reverse-DNS, content-hash, supersedes).

### Tenancy — touch the LOCKED grammar carefully (AC-7, narrowed)
- **`MemoryNamespace`** (closed enum, NFR-Test-11 hash): `crates/maos-domain/src/memory.rs` (36-108). Do NOT add a variant. Reserve identifiers in a separate non-hashed manifest (D8).
- **Cap-token signing key:** `crates/maos-capability/src/cap_tokens/key.rs` `Ed25519SigningKey([u8;32])` (11-25); `CapTokensShardRing` one `signing_key` (`cap_tokens/mod.rs` 111-145). Per-operator key is the v1.5+ slot (reserved, not built).

### Cross-cutting hazards (do not re-introduce)
- **GDPR cascade is frame-kind-specific** (9.3b sec-redteam CRITICAL): Mary ruled AC-5/AC-6 add ZERO principal-bearing frames (D3/D5), **conditional** on (a) `training_data_lineage` non-free-text and (b) region-neutral erasure proof. If implementation violates either, a new principal-bearing frame appears → it MUST be enumerated in the 9.2 cascade in THIS story (append-only + HARD byte-identity replay = permanent leak otherwise). Escalate to Mary/John before merge.
- **Determinism:** new serialized payloads use `BTreeMap` (never `HashMap`) + integer units — 9.2b/9.3b byte-identity replay breaks on nondeterministic ordering.
- **Cross-store atomicity (W1, pre-existing HIGH):** `forget_with_reason` non-atomic across stores (`memory/mod.rs:155-205`); region enforcement on the memory store must not assume a transaction that isn't there.

### Project Structure Notes
- Modified (kernel-core, re-pin): `crates/maos-kernel-core/src/security/operator_config.rs` + region enforcement guard.
- Modified (ABI/manifest, ratification): `crates/maos-manifest/src/manifest.rs`, `crates/maos-spirit-abi/src/lib.rs` (2→3), `crates/maos-registry/src/admission.rs` (AC-6 + AC-8).
- Modified (domain/capability, zero gate): `crates/maos-domain/src/memory.rs` + new `reserved_namespaces.rs`, `crates/maos-capability/src/cap_tokens/key.rs`.
- Gates: `xtask/error-catalog.toml` (`ERegionViolation`), `xtask/abi-ratifications.toml` (entry), `xtask/kernel-core-baseline.toml` (re-pin), new `xtask` write-path-bypass check.
- Reference example: `spirits/mira/manifest.toml` (`[model_provenance]`).

### References
- [Source: 9-4-...md] the ops half (lands first; rebase on it)
- [Source: epic-9-...-v05-v10.md#Story 9.4] AC source (199-214); requirements-inventory.md NFR-Comp-4 (247), NFR-Comp-5 (248), NFR-Ops-11 (239), NFR-Test-11 (171), NFR-Tenancy-1 (253)
- [Source: 9-3b-...md] kernel re-pin + ABI ratification model (ADR-045 §4/F6), FR62 governance stream, schema-identity discipline, determinism rules, §A6 net
- [Source: xtask/abi-ratifications.toml] the 9.3b worked `[[ratification]]` example
- [Source: deferred-work.md:193] `provider_history` unbounded growth (AC-8)

## Dev Agent Record

### Agent Model Used

claude-opus-4-8 (1M context) — Opus (§A6 net N/A). Party-mode preflight DONE (2026-06-14, 5/5) + re-ratification (2026-06-15, 5/5).

<!--
§A6 NON-OPUS SAFETY NET — MANDATORY here. AC-5 (region crypto enforcement / ERegionViolation / HKDF+AEAD binding),
AC-6 (manifest admission + governance journaling), AC-9 (derivation-site completeness), AC-10 (region-neutral
erasure proof) are correctness-critical. Non-Opus dev ⇒ record "non-Opus → preflight + multi-layer review attached"
with links, or "Opus (net N/A)". Party-mode preflight is DONE (2026-06-14, 5/5).
-->
Opus (net N/A). Correctness-critical AC-5/AC-12 cryptographic root implemented test-first; merge-blocking gates R-RG1/R-RG4′/R-SCH/AC-12 green at the crypto layer.

### Debug Log References

**Implementation Plan (grounded in code, 2026-06-15, post-re-ratification — Option A):**

Dep-graph facts forcing the file layout: `maos-cli` → {maos-audit, maos-domain} (kernel-free ✓); `maos-bin` → {maos-audit, maos-domain, maos-kernel-core}; `maos-kernel-core` does NOT depend on maos-audit. Only common crate = `maos-domain`. TL signing (`sign_bundle`, `build_erasure_proof`) lives in `maos-audit`, called from cli (sealed-export) + bin (proof), both passing a RAW seed. Memory writes do NOT emit a per-write TL frame today (only `forget_with_reason` journals). `ring 0.17.14` already in workspace lock; `maos-domain` has `ed25519-dalek`+`getrandom` but no `ring`/`hkdf`.

- **`maos-domain/src/region.rs`** (NEW, pure): `Region` newtype + AC-12 canonicalize (trim→ascii-lowercase→validate `^[a-z0-9-]{2,32}$`, frozen `ascii-v1`); `RegionError::{ERegionViolation, ERegionTagInvalid}` (thiserror; auto-catalogued — maos-domain in scan_dirs). No new dep.
- **`maos-audit`** (crypto root, R-RG4′): add `ring` dep (no lock growth); `derive_region_signing_seed(base,&region)` via `ring::hkdf` (info = domain-sep ‖ canonical region bytes); add optional `region` to the sealed bundle, bind into signing+AAD; verify re-derives. v2 bundles (no region) verify under non-derived key (AC-11/R-SCH compat). `proof.rs` stays region-NEUTRAL (D4/AC-15).
- **`maos-kernel-core/src/security/operator_config.rs`**: `RegionSection` + `MAOS_REGION_*` (mirrors RegistrySection). **`maos-kernel-core/src/memory/write_entry_point.rs`** (NEW): non-wildcard `WriteEntryPoint` enum (current DirectWrite variants + reserved ReplayApply/BackupRestore/ScheduledWrite for AC-9 compile-time guard); raw store writers → `pub(in crate::memory)`. No store schema change.
- **`maos-cli`/`maos-bin`**: derive region seed before `sign_bundle`; bin teardown (AC-14) + region-neutral receipt (AC-15).
- **xtask**: `error-catalog.toml` entries; `bypass_scan` (AC-9); `kernel-core-baseline.toml` re-pin 21472→~21667 (HARD STOP 21720) LAST.
- **DEVIATIONS to flag:** (1) `ring::hkdf` instead of ratified RustCrypto `hkdf` crate — same intent (audited crate, no hand-roll), strictly fewer deps (ring already vendored). (2) `maos-audit` gains region-derived signing + a bundle field — additive, does NOT make audit a store-writer (read-only invariant preserved); flag in review.

### Completion Notes List

**In progress (2026-06-15) — AC-5 cryptographic root + config landed & green:**
- ✅ AC-12 `Region` canonicalizer (`maos-domain/src/region.rs`): frozen `ascii-v1` (`^[a-z0-9-]{2,32}$`, trim+lowercase), `RegionError::{ERegionViolation, ERegionTagInvalid}`. **11 tests** incl. two-spellings-key-identically, homoglyph/non-ASCII rejection, frozen-encoding tripwire.
- ✅ AC-5 crypto root (`maos-audit/src/sealed_export.rs` + `Cargo.toml` `hkdf`): region-derived Ed25519 signing seed via HKDF-SHA256 (`derive_region_signing_seed`/`derive_region_pubkey`), additive `region: Option<String>` on bundle (skip-if-None ⇒ byte-identity preserved), region covered by signed digest. **6 tests**: **R-RG1** (home ALLOW/foreign VIOLATION), **R-RG4′** (region tamper breaks verification), **R-SCH** byte-identity, **R-SCH2** round-trip, **R-SCH3** v2 compat, **AC-12** two-spellings identical key.
- ✅ `RegionSection` config (`maos-kernel-core/src/security/operator_config.rs`): `[region].home_region` + `MAOS_REGION_HOME` env (mirrors RegistrySection precedence), invalid-tag fail-safe (DISABLE not mis-bind). **4 tests**.
- Dep deviation RESOLVED: used ratified RustCrypto `hkdf` crate (NOT ring) — maos-audit Cargo.toml declares "Decision B: NOT ring", so `hkdf`-over-`sha2` is the correct, consistent choice (the earlier ring idea is withdrawn).

**Task 1 (AC-5) — COMPLETE & GREEN (2026-06-15):** AC-9 `WriteEntryPoint` non-wildcard enum + `enforce_region` guard (`memory/write_entry_point.rs`, 4 tests: R-RG1/R-RG2-anti-stub/R-RG3-exhaustive); raw store read+write methods tightened to `pub(in crate::memory)` (no-unanchored-read structural chokepoint); `xtask bypass-scan` gate registered + GREEN + **teeth-verified** (injected `_ => None` arm → FAIL, restored → PASS); daemon memory adapter `.with_home_region(...)` wired (main.rs:1543); `maosctl audit sealed-export`/`export` region-pin via `Region::home_from_env`; kernel re-pin 21472→**21894** (tripwire authorized — see baseline HISTORY), `check-kernel-baseline` GREEN.

**Task 3 (AC-8) — COMPLETE & GREEN:** bounded `ProviderHistory` (cap 4096, evict-oldest, never reject-new); `deferred-work.md:193` CLOSED; 2 tests.

**Task 4 (AC-7) — COMPLETE & GREEN:** generic tenant field CUT (D7); `reserved_namespaces.rs` + CI collision guard; grammar-lock intact; 2 tests.

**Task 2 (AC-6 model-provenance) — COMPLETE & GREEN (2026-06-15):**
- `ModelProvenanceSection` (`maos-manifest/src/manifest.rs`): mirrors `ClassSection`; `training_data_lineage` schema-constrained to **reverse-DNS lineage refs** (`is_reverse_dns_lineage`) so **free-text/PII is structurally rejected** (D5 → "zero principal nexus" structural, not promised); strict UTC RFC3339→unix parser (`parse_rfc3339_utc_secs`, leap-year-tested); `from_manifest_toml` (OPTIONAL-on-read → AC-11); `validate_staleness`; `canonical_content_bytes` (field-ordered, length-prefixed → reproducible content-hash). **10 tests.**
- Schema bump (`maos-spirit-abi/src/lib.rs`): `MANIFEST_SCHEMA_VERSION` 2→3 (MAX follows; **MIN stays 1** per re-ratification — N-2 floor-lift deferred to 7.5a); `POST_V1_SCHEMA_SECTIONS` += `model_provenance`. Two deliberate schema tripwire tests updated consciously (`manifest_schema_version_pinned_*` → ledger entry; `n_minus_2_hard_refusal_posture` → documents the AC-6 deferral).
- Typed **catalogued** error (`maos-domain/src/provenance.rs`): `ProvenanceError::{EModelProvenanceMissing, EModelProvenanceStale}` — placed in `maos-domain` because the FR63 catalog scans it (NOT `maos-registry`); both registered in `error-catalog.toml` (**41/41**).
- Admission gate (`maos-registry/src/admission.rs`): companion `validate_model_provenance` + `ModelProvenancePolicy`/`ModelProvenanceRecord` + `AdmissionError::ModelProvenance(#[from])` / `ModelProvenanceMalformed`. Kept `admit_spirit`/`AdmissionConfig` **frozen** (avoided churning 27 construction sites; AC-11 byte-stable). New dep edge `maos-registry → maos-manifest` (kernel-free, no cycle). **6 tests.**
- FR62 governance event: `GovernanceEventKind::ModelProvenance` + `ModelProvenancePayload` (`maos-domain/src/governance.rs`) — schema-identity (`MODEL_PROVENANCE_SCHEMA_ID`) + content-hash + `deployment_operator_id` (D7 constant), **zero claim-instance ids** (structural test). Emitted in `maos-bin` (`emit_model_provenance_event`; **fail-closed** on `maos run`, best-effort re-emit on posture-shift); queryable via `--kind governance` (FrameKind 28, additive sub-type — `check-governance-categories` PASS). **2 domain tests.**
- Ratification: `[[ratification]] 9.4b-AC6-model-provenance` in `abi-ratifications.toml` (`check-abi-ratification` PASS). `[model_provenance]` reference example added to `spirits/mira/manifest.toml`.

**Drive-by fix (pre-existing HEAD red, caught by AC-6 regression):** `maos-iac` `approval_log_is_distinct_table` asserted exactly 4 SQLite tables but 9.3b (commit 3041fec) added `schema_lifecycle_registry` at open and left the assertion stale → corrected to 5 (table legitimately exists).

**Regression (2026-06-15, post-AC-6):** **workspace lib+bins 1622 passed / 0 failed**; touched-crate integration (registry/manifest/spirit-abi/domain) **522 / 0**. Gates GREEN: `error-catalog-check` (41/41), `check-manifest-schema-version` (current=3), `check-abi-ratification`, `check-kernel-baseline` (**21894 UNCHANGED** — confirms D2: AC-6 = zero kernel-core LOC), `check-governance-categories`, `check-corpus`, `bypass-scan`. (One pre-existing parallel-timing flake `cassette_replay::replay_serves_sequenced_entries` — passes in isolation, unrelated to AC-6.)

**Task 1b (AC-13/AC-14/AC-15 regional teardown + region-neutral receipt) — COMPLETE & GREEN (2026-06-15):**
- `maos-audit/src/erasure/regional_teardown.rs` (NEW): two-part `RegionalTeardownReceipt` (phase (a) forget-cascade attestation + phase (b) key-decommission attestation), **HOME-key-signed → region-NEUTRAL**; `build_regional_teardown_receipt` **fail-closed** on either incomplete phase (AC-14); `verify_regional_teardown_receipt`; `decommission_region_key`; `ForgetCascadeAttestation::from_outcome` computes `completed` structurally (all 3 required stores). **9 tests** (AC-14 fail-closed ×2, AC-15 two-part/home-key/region-neutral, AC-10 region-key-destruction-then-still-verifies, AC-13 region-specific placement, region-tamper-breaks-verify, JSON round-trip).
- AC-14 bin surface: `run_uninstall_cascade` (maos-bin) emits the region-neutral receipt next to the erasure proof when `MAOS_REGION_HOME` is set (additive, fail-closed); reuses the 9.2 forget cascade.
- **D4-wording deviation RATIFIED & CLOSED (party-mode Mary+John, 2026-06-15):** Option A artifacts are signed not encrypted → phase (b) is "signing-key decommission" (forward-capability revocation only), not "crypto-shredding". AC-14 text updated with the mandated NON-CLAIM clause (Art. 17 erasure = phase (a) cascade alone). Behaviour unambiguous; wording only. **No new principal-bearing frames** (D3/D5 irreversibility trap NOT triggered).

**ALL ACs IMPLEMENTED. No open story-level items** — the single D4-wording ratification was closed by party-mode (Mary+John, 2026-06-15); AC-14 text now carries the ratified "signing-key decommission" + NON-CLAIM wording.

### File List

- `crates/maos-domain/src/region.rs` (NEW) — AC-12 Region newtype + canonicalization + RegionError
- `crates/maos-domain/src/lib.rs` (MOD) — `pub mod region;`
- `crates/maos-audit/Cargo.toml` (MOD) — add `hkdf = "0.12"`
- `crates/maos-audit/src/sealed_export.rs` (MOD) — region-derived signing + `region` bundle field + 6 tests
- `crates/maos-kernel-core/src/security/operator_config.rs` (MOD) — `RegionSection` + tests
- `crates/maos-domain/src/region.rs` (NEW) — Region + canonicalization + RegionError + `home_from_env` (12 tests)
- `crates/maos-domain/src/reserved_namespaces.rs` (NEW) — AC-7 reserved identifiers + CI collision guard (2 tests)
- `crates/maos-domain/src/lib.rs` (MOD) — `pub mod region; pub mod reserved_namespaces;`
- `crates/maos-kernel-core/src/memory/write_entry_point.rs` (NEW) — AC-9 `WriteEntryPoint` enum + `enforce_region` (4 tests)
- `crates/maos-kernel-core/src/memory/mod.rs` (MOD) — `home_region` field + `with_home_region` + write-path `enforce_region` wiring + submodule decl
- `crates/maos-kernel-core/src/memory/{private,shared,principal}.rs` (MOD) — raw read/write methods → `pub(in crate::memory)` (chokepoint)
- `crates/maos-kernel-core/src/security/operator_config.rs` (MOD) — `RegionSection` + tests
- `crates/maos-kernel-core/src/security/mod.rs` (MOD) — bounded `ProviderHistory` (AC-8) + 2 tests
- `crates/maos-bin/src/main.rs` (MOD) — daemon memory adapter `.with_home_region(...)`
- `crates/maos-cli/src/subcommands.rs` (MOD) — sealed-export/export region-pin via `Region::home_from_env`
- `xtask/src/bypass_scan.rs` (NEW) + `xtask/src/main.rs` (MOD) — AC-9 `bypass-scan` gate
- `xtask/error-catalog.toml` (MOD) — `ERegionViolation` + `ERegionTagInvalid` (AC-5) + `EModelProvenanceMissing` + `EModelProvenanceStale` (AC-6); bijection **41/41** PASS
- `xtask/kernel-core-baseline.toml` (MOD) — re-pin 21472 → 21894 (FLAG-Winston, tripwire authorized); unchanged by AC-6 (zero kernel-core LOC)
- **AC-6 (Task 2) files:**
- `crates/maos-domain/src/governance.rs` (MOD) — `GovernanceEventKind::ModelProvenance` + `ModelProvenancePayload` + `MODEL_PROVENANCE_SCHEMA_ID` (+2 tests)
- `crates/maos-domain/src/provenance.rs` (NEW) — catalogued `ProvenanceError` (admission missing/stale)
- `crates/maos-domain/src/lib.rs` (MOD) — `pub mod provenance;`
- `crates/maos-manifest/src/manifest.rs` (MOD) — `ModelProvenanceSection` + reverse-DNS/RFC3339 validators + `POST_V1_SCHEMA_SECTIONS` += `model_provenance` (+10 tests)
- `crates/maos-manifest/src/lib.rs` (MOD) — re-export `ModelProvenanceSection`
- `crates/maos-spirit-abi/src/lib.rs` (MOD) — `MANIFEST_SCHEMA_VERSION` 2→3 (MAX follows, MIN=1)
- `crates/maos-spirit-abi/tests/manifest_n_minus_1_test.rs` (MOD) — schema tripwire guards updated (bump ledger + N-2 deferral)
- `crates/maos-registry/Cargo.toml` (MOD) — add `maos-manifest` dep (AC-6 admission validation)
- `crates/maos-registry/src/admission.rs` (MOD) — `validate_model_provenance` + `ModelProvenancePolicy`/`ModelProvenanceRecord` + `AdmissionError` variants (+6 tests)
- `crates/maos-bin/src/main.rs` (MOD) — `emit_model_provenance_event` + policy/operator-id resolvers; wired into `maos run` (fail-closed) + posture-shift admission paths
- `xtask/abi-ratifications.toml` (MOD) — `[[ratification]] 9.4b-AC6-model-provenance`
- `spirits/mira/manifest.toml` (MOD) — `[model_provenance]` reference example
- `crates/maos-iac/src/adapter/transparency_log.rs` (MOD) — drive-by: stale 4-table assertion → 5 (9.3b `schema_lifecycle_registry`)
- **AC-13/14/15 (Task 1b) files:**
- `crates/maos-audit/src/erasure/regional_teardown.rs` (NEW) — two-phase fail-closed teardown + region-neutral home-key receipt (9 tests)
- `crates/maos-audit/src/erasure/mod.rs` (MOD) — `pub mod regional_teardown;`
- `crates/maos-bin/src/main.rs` (MOD) — `run_uninstall_cascade` emits region-neutral teardown receipt when `MAOS_REGION_HOME` set
- `_bmad-output/implementation-artifacts/deferred-work.md` (MOD) — AC-8 line CLOSED
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (MOD) — 9.4b → in-progress
- `_bmad-output/implementation-artifacts/9-4b-...md` (MOD) — re-ratification section + Dev Agent Record

### Review Findings

Adversarial review completed 2026-06-15 (Blind Hunter + Edge Case Hunter + Acceptance Auditor, Opus 4.8).

#### Team decisions (resolved by consensus — per spec and long-term correctness)

- [x] [Review][Decision] R-SCH1 frozen v2 golden corpus absent — **DECISION: Waive strict 'prior commit' ordering and commit 9.4b with `MANIFEST_SCHEMA_VERSION 3` now; v2 corpus will be added retroactively in a follow-up commit.** Risk accepted: without a pre-existing frozen corpus, a future regression in v2 admission cannot be bisected to a pre-bump baseline. Mitigation: synthetic v2 compatibility tests (`provenance_absent_admits_*`, `v2_manifest_loads_*`) already green; retroactive corpus must be added before next manifest-schema change.

#### Patch

- [x] [Review][Patch] Implement R4 typed reject for invalid region tags — Propagate `RegionError::ERegionTagInvalid` from config resolution through the CLI/bin paths and ensure the `WriteEntryPoint` boundary rejects invalid tags fail-closed. [source: Team decision on decision-needed #2]
- [x] [Review][Patch] R1-COND / AC-9 no-unanchored-read proof gap — The implementation relies on `pub(in crate::memory)` visibility restrictions for raw store read methods, but R1-COND requires an explicit test enumerating every read entry point on all 3 backends proving each is gated through a region-verified frame. Add a read-side enumeration test/enum or equivalent explicit proof. [source: Acceptance Auditor; crates/maos-kernel-core/src/memory/]
- [x] [Review][Patch] `emit_model_provenance_event` panics on serialization failure — `serde_json::to_vec(&gov_payload).unwrap()` at `crates/maos-bin/src/main.rs:813` can panic the host process. Change to `map_err` and propagate as an admission error (fail-closed). [source: Blind Hunter + Edge Case Hunter]
- [x] [Review][Patch] `ProviderHistory` order/map desynchronization under churn — `insert` only pushes to `order` on first insertion; re-inserting a spirit after eviction leaves a ghost entry in `order`, so the deque can grow unboundedly and future evictions can remove a live re-inserted entry. Keep `order` in sync with `map` on insert/remove. [source: Blind Hunter + Edge Case Hunter; crates/maos-kernel-core/src/security/mod.rs:158-172]
- [x] [Review][Patch] `validate_staleness` u64→i64 cast wraps — `if age > max_age_secs as i64` in `crates/maos-manifest/src/manifest.rs:1777` silently wraps large `max_age_secs` values to negative `i64`, causing incorrect staleness verdicts. Make the comparison type-safe (e.g., compare as `u64` after clamping `age` to non-negative). [source: Blind Hunter + Edge Case Hunter]
- [x] [Review][Patch] `Region::home_from_env()` vs `RegionSection::resolve_from_env_and_disk()` split-brain — `maos-cli` uses env-only `Region::home_from_env()` for sealed exports, while `maos-bin` uses toml+env `RegionSection` for memory pinning. An operator setting `home_region` only in `operator.toml` gets memory pinned to that region but exports signed with the raw (non-region-derived) seed. Unify the resolution path. [source: Edge Case Hunter]
- [x] [Review][Patch] `ForgetCascadeAttestation::from_outcome` accepts unknown store names — `stores_covered` is an arbitrary `Vec<String>`; a caller can include fake store names and still get `completed: true`. Validate that every entry is a known store name (or at least that only known stores are reported). [source: Edge Case Hunter; crates/maos-audit/src/erasure/regional_teardown.rs:150-159]
- [x] [Review][Patch] `xtask bypass-scan` uses brittle text matching — Visibility and wildcard checks are string-based and can be defeated by formatting changes or false-positive comment matches. Harden with AST-aware checks or tighter regex anchored on token boundaries. [source: Blind Hunter; xtask/src/bypass_scan.rs:84-89]
- [x] [Review][Patch] `region.rs` tests use `env::set_var/remove_var` without `ENV_LOCK` — `std::env::set_var` is unsound under parallel test execution. The `operator_config.rs` tests already use `ENV_LOCK`; the `region.rs` tests need the same serialization or `serial_test`. [source: Blind Hunter; crates/maos-domain/src/region.rs tests]

#### Defer

- [x] [Review][Defer] Ed25519 double-hash composition — `regional_teardown.rs` and `sealed_export.rs` sign a SHA-256 digest with Ed25519 (which itself hashes with SHA-512), creating a non-standard `Ed25519(SHA-256(msg))` composition. Internally consistent (verify also pre-hashes), but differs from pure Ed25519 and forgoes its collision-resistance proof. Deferred: consider signing canonical bytes directly in a future hardening pass. [source: Blind Hunter]
- [x] [Review][Defer] Home signing seed reused as region-key derivation base — `run_uninstall_cascade` uses the same `signing_seed` as both the HKDF base for region keys and the raw home signing key. HKDF differentiates the derived keys, but ideal key separation would use distinct seeds for each role. Requires broader design change; not a correctness bug under current Option A model. [source: Blind Hunter + Edge Case Hunter]

#### Dismissed (6)

- Posture-shift re-admission swallows model-provenance errors — spec explicitly says "best-effort on posture-shift" (Task 2 dev notes); not a deviation.
- Region-key "decommission" is purely attestation — matches AC-14 NON-CLAIM clause (Option A artifacts are signed not encrypted; phase (b) is forward-capability revocation only).
- `deferred-work.md:193` closure not visible in diff — the file was modified (git status shows it staged); the review diff excluded `_bmad-output/implementation-artifacts/*` files.
- Region grammar allows leading/trailing hyphens — matches the ratified grammar `^[a-z0-9-]{2,32}$`.
- R-RG2 anti-stub test does not cover `DirectWrite` — `DirectWrite` is home-by-construction; the anti-stub property is enforced via foreign/untagged variants.
- `_token` discarded from `insert_frame_event` — `insert_frame_event` returns `LogBeforeDeliver<()>` (a typestate receipt); dropping it is the intended usage.


