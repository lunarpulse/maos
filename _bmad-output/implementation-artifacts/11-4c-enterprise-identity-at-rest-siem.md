---
baseline_commit: bc0616bce97bdadd926a21664fe05561a05c1b34
story_key: 11-4c-enterprise-identity-at-rest-siem
---

# Story 11.4c: Enterprise identity + at-rest + SIEM

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story.
     PREFLIGHT RESOLVED 2026-07-06 (party-mode — Winston · Murat · John · Amelia · Mary + Vex/security & Grumbal/adversary + Paige walk-ons). All 8 forks ratified; two improved on their leans (F1→6 ACs, F8→new identity.asserted audit kind). See the ✅ PREFLIGHT RESOLVED block below the Story section. -->

**Model (§A5 — RESOLVED, F1): opus-4-8 MANDATORY (whole story) + full §A6** (Blind · Edge · Acceptance · **Test-Infra** + a **runtime-execution check**). Escalated from the epic's `opus-4-6 (token-issuance slice opus-4-8)` (`epic-11…md:48`; `sprint-status.yaml:190`) **on evidence** — this is a **security-adjacent** story (an SSO assertion that governs which principal is authorized for a capability grant IS the authorization path, §4.6) **and** it carries **three** independent canned-green traps ("identity is REALLY verified, not a canned principal"; "rows are REALLY sealed, not passthrough"; "SIEM records are REALLY redacted, not leaked"), each the exact vacuous-green class opus-4-6 shipped as a P1 on 11.2a. Same basis as 11.4a F1 / 11.2b F1. **The split-tier boundary was rejected as too loosely drawable at review — whole-story opus-4-8.** **§A6 non-negotiable:** the review MUST run the identity-verify **live over a real JWKS/OIDC signature check and watch the `sso-fault-inject` falsifier accept a forged token → red the leg** (plus the `alg:none`/alg-confusion negatives — F4), and watch `kms-fault-inject` (passthrough) + `siem-fault-inject` (redaction-blind) red their legs — a degraded/rate-limited review is not a review and hard-blocks completion.

**Kernel-Δ (RESOLVED, F8): ZERO — held, not hoped (`check-kernel-baseline` @ 23081 is the proof, not a promise — Grumbal).** Baseline `src_lines = 23081` (`xtask/kernel-core-baseline.toml:205`; pinned by 11.4a/held zero-delta by 11.4b). All three capabilities land as **out-of-kernel `maos-domain` ports + leaf adapter crates + `maos-bin` composition-root wiring** (the DONE 11.4a/11.4b pattern). **The kernel-delta landmine (F8 — RESOLVED, materially refined):** "SSO assertions flow through to capability-token issuance" (`domain-specific-requirements.md:82`) must NOT be read as "add a `principal` field to `CapabilityToken`" — that struct is `#[non_exhaustive]`, kernel-core, ABI-frozen, `spirit_pid`-bound (`i1.rs:175`); a principal field is a kernel-core delta AND an ABI break — **FORBIDDEN**. The flow-through is realized **off-hot-path in two out-of-kernel legs**: (a) the authenticated principal populates the already-shipped, additive `PolicyDecisionRequest.principal_attributes` (`policy_decision.rs:112`, `None` in 11.4a) that **governs the PDP authorization** of issuance; and (b) provenance is recorded by a **NEW out-of-kernel `identity.asserted` audit kind** emitted by the `maos-sso` adapter (the 11.4b `kind=8` pattern) — **NOT** by mutating the kernel's issuance audit record (which would be a kernel edit — Grumbal's catch). **Correlation is at the AUTHORIZATION layer, not the token:** at assertion time no `token_id` exists yet (the token is minted downstream), so the honest binding is "principal X was asserted and governed the authorization of `spirit_pid` P's grants at decision-time T" — never "principal X owns token Y." ADR-051 states this scope in ink (Vex/John: a reviewer WILL read the requirement literally and expect a token field). Any churn **outside** the named adapter crates — even a `cargo fmt` reflow of kernel-core (10.5 R3 / §A7.4) — is a finding: revert or disclose. `maos-kernel-core` / `maos-domain` dependency closure stays free of the OIDC / KMS-SDK / SIEM-format client crates (`check-dependency-closure`). **If** dev discovers a genuine kernel seam unavoidable (it should not be — F8 is ratified zero), that is a **bounded FLAG-Winston re-pin** from 23081, HISTORY-disclosed and ADR-051-documented — never a silent edit (Task 0 contingency only).

**Depends:** **11.4a (DONE — `epic-11…md:48` `Depends: 11.4a`).** 11.4c is a Wave-2 leaf that **layers a real authenticated principal** onto the port 11.4a shipped: the `principal_attributes` hook was shaped by 11.4a F7 **specifically for this story** (`policy_decision.rs:95-112`: "populated by the SSO→capability-token issuance slice; kept additive so 11.4c is non-breaking"). It reuses the DONE port/adapter/composition-root pattern (ADR-041/ADR-050 — `PolicyDecisionPort`/`maos-pdp`, `CryptoProvider`/`RingCryptoProvider`, the `escape_detector_consumer.rs` read-only-TL-consumer shape) and the DONE gate infra (`check_enterprise_pdp.rs`/`check_escape_detector.rs` shape, `gate-registry.toml`, `check-dependency-closure`, `check-ship-gate-completeness`). It is the **last enterprise-bundle story** — nothing downstream depends on it inside Epic 11.

**Dev-gate:** Epic-11 dev/**merge** is GATED on the 2 external v1.5 holds (real external pen-test zero-P0/P1 + export-compliance counsel 5D002.c.1; `epic-11…md:5`). Story-level build/preflight is **NOT** gated — 11.4c may be built and proven green on branch `epic-11` during the hold; it MUST NOT merge to the shippable line before those holds clear. **Export note (5D002.c.1):** an OIDC adapter (TLS to an IdP), a KMS adapter (envelope encryption / AEAD-at-rest), and a SIEM sink (TLS to a collector) each add a **cryptographic surface** — confirm at review that they introduce **no new kernel crypto** (all AEAD routes through the existing `CryptoProvider` trait, `crypto.rs:78`; all TLS is in the leaf adapters) and flag the at-rest-encryption + IdP-TLS surfaces to export counsel, since a new encryption capability can change the 5D002.c.1 classification.

---

## Story

**As** the enterprise MAOS operator standing up a compliance-bound, SSO-governed deployment (Journey: Reza Cortex / cross-team policy; PRD §Reza; enabler-prereqs §10.7.2(a)-(c) per `full-prd-gap-map-and-planning-plan-2026-07-06.md:27`),
**I want** three enterprise integrations — **(1)** human/service SSO via **OIDC** verified against my org IdP so an **authenticated principal** governs and is recorded on capability-token issuance (`domain-specific-requirements.md:82`: "SSO assertions flow through to capability-token issuance"); **(2)** **encryption-at-rest** of Collective/audit memory sealed with an **org-KMS**-wrapped data key (`product-scope.md:70`: "encrypted-at-rest memory with org KMS"); and **(3)** **SIEM export** of the Transparency Log to my security stack with the redaction policy applied (**NFR-Aud-11** "SIEM export at v2.0"; NFR-Obs-4) — each behind an **out-of-kernel port**, proven to be **real** (real IdP signature verification, real AEAD ciphertext-at-rest, real redacted forwarding — not canned/passthrough/leaking), **fail-closed** on integration unavailability, and **byte-identical to v1.5 when unconfigured**,
**So that** MAOS satisfies the v2.0 "Enterprise identity + at-rest + SIEM" commitment (`product-scope.md:70`; `project-scoping-phased-development.md:244`; NFR-Aud-11) **without breaking the v1.0-frozen ABI or drawing the kernel-core budget** — the kernel stays the small, dumb mediator (ADR-006 "the kernel learns nothing"; the org's identity, keys, and SIEM policy are the operator's data), the authenticated principal never welds into the frozen `spirit_pid`-bound `CapabilityToken` (F8), and Option-A plaintext rows remain the **default** posture (at-rest AEAD is an additive enterprise opt-in — F2).

### Explicitly NOT in 11.4c (defend at review)
- **A production Vault / multi-cloud-KMS backend matrix.** The gap-map lists "**Vault/cloud-KMS secret backends**" as **NOT absorbed by Epic 11** (`full-prd-gap-map…:34`). 11.4c ships the **`KeyManagementPort` + envelope/at-rest AEAD mechanism + ONE reference adapter** (F3 lean: a local/env-backed KMS that is CI-runnable, mirroring 11.4a's "the PDP port, not the Spirit / Cedar reference only"). AWS-KMS / Vault / GCP-KMS adapters are **additive-per-port** (ADR-010) — named in ADR-051, not built here.
- **An Enterprise reference Spirit.** The PRD Spirit-count progression's 11th Spirit is out of scope — "**11.4a shipped the PDP port, not the Spirit**" (`full-prd-gap-map…:34`); 11.4c likewise ships enterprise *infrastructure*, not a Spirit.
- **Adding a `principal` / identity field to `CapabilityToken` or ANY kernel-core edit.** ZERO-Δ (F8). The frozen token stays `spirit_pid`-bound; the principal lives in `principal_attributes` (PDP request) + the audit provenance. A kernel seam is a **preflight FLAG-Winston escalation**, never a silent edit.
- **Encrypting-at-rest by DEFAULT / reversing the ratified Option-A plaintext decision.** 9.4b/ADR-049 ratified plaintext rows, region-bound by governance (sign-only; `write_entry_point.rs:4-9`). 11.4c's at-rest AEAD is an **opt-in layer enabled only when org-KMS is configured** — the default (no `MAOS_KMS_*` env) stays byte-identical Option-A (F2, AC4). Do **not** silently flip the default.
- **SSO for the operator-surface UI / interactive login UX / SAML.** 11.4c verifies **OIDC assertions** at the capability/issuance boundary (machine-to-machine + assertion verification). SAML and interactive human web-login UX are named in the prose (`domain-specific-requirements.md:82` "OIDC / SAML") but **SAML is additive-per-port, deferred** — one reference identity protocol (OIDC) proves the flow-through, mirroring the 11.4a one-reference-adapter discipline.
- **Real-time streaming analytics / a SIEM query-back channel.** 11.4c **forwards** TL entries out (read-only tail → CEF/syslog/NDJSON → configured sink) with redaction; it does not pull data back from the SIEM or add a query API (that is `maosctl audit`, Story 9.1, already DONE).
- **The OpenTelemetry adapter (NFR-Aud-11 *first* phase).** Already DONE (Story 9.5b, `maos-telemetry` / `TraceSink`). 11.4c delivers the **second phase** (SIEM export) only; it does NOT re-implement OTel.

---

## ✅ PREFLIGHT RESOLVED — UNANIMOUS (party-mode 2026-07-06 — Winston · Murat · John · Amelia · Mary + Vex/security & Grumbal/adversary + Paige walk-ons)

All 8 forks + Grumbal's fail-closed catch ratified on verified code evidence. The rulings below **supersede** the leans in the ⚠️ FORKS block (retained as the code record). Through-line: 11.4c is **three strangers wearing the same "enterprise" name tag** (SSO, KMS-at-rest, SIEM share no code/port/harness) — so every ruling makes each subsystem's thesis *independently falsifiable* and keeps the two seductive free-words ("provenance", "encrypted") honest. **Two rulings improved on the story's leans:** F1 added a 6th AC to give the security-adjacent SSO flow-through its own falsifiable line; F8 replaced "record the principal in the issuance audit" (a hidden kernel edit — Grumbal) with a NEW out-of-kernel `identity.asserted` audit kind correlated at the authorization layer.

- **F1 — RESOLVED: opus-4-8 whole-story + full §A6, AND AC count 5→6.** The split-tier boundary is too loosely drawable at review → whole-story opus-4-8 (security-adjacent + 3 canned-green traps). Murat's structural catch: 5 ACs across 3 security-critical subsystems = ~1.6 AC each, and the *most* dangerous slice (SSO→authorization flow-through) was riding shotgun as an `And` clause of AC1. **The bundle stays whole (epic ratified "full PRD bundle"), but the §A5 ceiling is 6, not 5** — AC1 splits into AC1 (OIDC-verify fail-closed) + AC2 (principal governs authorization + provenance, the opus-4-8 slice). Verify and flow-through fail in different ways; each needs its own proven-red (Vex). John: "the epic said 5 as an estimate, not scripture; falsifiability per failure mode is the rule."
- **F2 — RESOLVED: opt-in AEAD, scoped to provable out-of-kernel persisted rows ONLY; kernel-core Private/Shared and generic kernel-authored TL at-rest DEFERRED.** The story's "opt-in, Option-A default" lean was correct but incomplete — it never said *which* memory. `write_entry_point.rs` (Private/Shared) is **kernel-core**; the live Transparency Log write path is also **kernel-core** (`TransparencyLogAdapter::insert_frame_event`); a seal there kills zero-delta. **Scope the at-rest AEAD to loom-lite Collective rows plus out-of-kernel `identity.asserted` writes**. Kernel-core Private/Shared and generic TL payload at-rest encryption are kernel write-path seams → **deferred, out of scope** (Winston). Option-A plaintext stays byte-identical default. **Honesty clause names the scoped rows + the deferral** (Grumbal: "'encrypted-at-rest memory' without a boundary is a lawsuit with YAML").
- **F3 — RESOLVED: `KeyManagementPort` + envelope + ONE reference local-master-key KMS, marked dev/CI-ONLY.** Vex: an env-var master key leaks via `/proc/PID/environ`, core dumps, crash reporters — if the reference adapter becomes the *only* thing in the box, it ships key-disclosure-by-default. **The reference adapter is explicitly reference/dev/CI-only, NOT production-recommended** (ADR-051 + NFR-Sec-19); production = OS keyring (maos-secrets/NFR-Sec-16) or a real cloud-KMS backend, additive-per-port, deferred. Env master key is acceptable for the CI *tripwire* (test key) only. Mirrors 11.4a's Cedar-in-process-is-the-reference discipline.
- **F4 — RESOLVED: real JWKS verify + explicit algorithm allowlist + `alg:none`/alg-confusion rejection as GATE NEGATIVES.** Vex's threat model is binding: `jsonwebtoken` `Validation` MUST pin an explicit algorithm allowlist (reject `alg:none`; reject HS256-signed-with-the-RS256-public-key — the alg-confusion CVE class); fail-closed on signature, `exp`/`nbf`, `aud` (= this deployment, else cross-RP replay), `iss` (allowlist), and JWKS-fetch-failure. The gate feeds an `alg:none` token + a confusion token and watches them **rejected** — a real per-commit leg, not advisory. Offline static-JWKS right-vs-wrong-key vector = the tripwire (Murat: in-process, never network-dependent — L5).
- **F5 — RESOLVED: redaction-before-forward ✓ + format set tightened.** NDJSON (already exists, `to_ndjson`) + **CEF** (the projection-port proof) as the two content formats; **RFC5424 syslog is transport framing, not a third serializer** (Murat/John — 3 formats was reference scope-creep). The HTTPS network sink is **TLS-only** (Vex: don't forward the forensic trail in cleartext); plaintext-TCP/file sink is localhost only. `query_with_redaction` applies BEFORE projection; derive-and-reconcile on redacted-field count; `siem-fault-inject` bypass → leak reds; empty TL → N/A not vacuous-pass.
- **F6 — RESOLVED: three leaf crates (`maos-sso`, `maos-secrets`, `maos-siem`) + 3 `maos-domain` ports.** Closures diverge (OIDC libs ≠ crypto ≠ format libs); `check-dependency-closure` keeps kernel-core/domain clean only if separated (Amelia). KMS lives in the `maos-secrets` placeholder. Ratified as-is.
- **F7 — RESOLVED: ADR-051 (lead-with-it) + NFR-Sec-18 (SSO/OIDC identity) + NFR-Sec-19 (opt-in at-rest AEAD) + NFR-Aud-11 SIEM-half annotation.** Mary: SSO-identity + at-rest have no upstream FR/NFR ID (prose-only) → each new NFR MUST cite ADR-051 **and** its scope-bullet source "or it traces to nothing." **Honesty clauses live IN the requirement text**, not a footnote: NFR-Sec-18 "one reference OIDC protocol; SAML additive-per-port, deferred"; NFR-Sec-19 "opt-in; loom-lite Collective rows plus out-of-kernel identity assertion writes; generic kernel-authored TL and kernel Private/Shared deferred; reference KMS is dev/CI-only"; NFR-Aud-11 "SIEM second-half delivered; OTel first-half = 9.5b."
- **F8 — RESOLVED (materially refined): principal → `principal_attributes` (authorization) + NEW out-of-kernel `identity.asserted` audit kind (provenance); correlation at the authorization layer; ZERO kernel-core delta.** Grumbal's catch killed the lean's hidden kernel edit: "record the principal in the issuance audit provenance" would mutate the kernel's `cap_tokens::issue` audit record. **Instead the `maos-sso` adapter emits its OWN `identity.asserted` entry** (new `kind`, out-of-kernel — the 11.4b pattern). At assertion time no `token_id` exists (minted downstream) → the honest binding is authorization-layer ("principal governed `spirit_pid` P's grants at T"), not token-ownership. ADR-051 states the scope explicitly (a reviewer will read the requirement literally). Zero kernel-core delta survives.
- **CATCH (Grumbal — fail-closed PER subsystem, folded as a CONSTRAINT on AC5, NOT a 7th AC):** the "no config → byte-identical" proof is the easy half; the 3am pager is "config present, integration DOWN." Each of the three must fail **closed and loud**, proven distinctly: **SSO IdP unreachable → deny issuance** (never fall open to `spirit_pid`); **KMS unreachable → refuse the sealed write** (never silently write plaintext under an encryption posture — Vex: the #1 real-world at-rest defeat); **SIEM sink down → buffer + operator-visible error** (never silent-drop the audit trail). Three distinct proven-reds inside AC5.
- **CARRIED (Murat/Vex non-negotiables):** every count needs a per-item blind that reds it (blind the verifier → forged-token reds; passthrough the seal → ciphertext≠plaintext reds; blind redaction → leak reds; blind one principal source → `identity-source-blind` provenance leg reds); the at-rest seal test MUST prove **wrong-key-fails** (a passthrough that ignores the key passes a naive ciphertext≠plaintext-only check — hunt the exact fake); tripwire legs stay in-process/offline (any live IdP/cloud-KMS/SIEM-collector leg is ADDITIONAL and advisory — L5).

---

## ⚠️ PREFLIGHT FORKS — AS RAISED (superseded by ✅ PREFLIGHT RESOLVED above; retained as the code-evidence record)

These were the load-bearing questions carried into the party-mode. Each lean is evidence-grounded against the live tree; read the ✅ RESOLVED block for the ratified ruling (which **improved on** F1 → 6 ACs and F8 → new audit kind, and **scoped** F2/F3/F5).

### F1 — Model tier: opus-4-6 (+ token-issuance slice opus-4-8) vs whole-story opus-4-8. **[LEAN: escalate the whole story to opus-4-8 + full §A6]**
Epic + sprint-status = `opus-4-6 (token-issuance slice opus-4-8) + §A6` (`epic-11…md:48`; `sprint-status.yaml:190`). But 11.4c is **security-adjacent** (SSO governs *which principal* gets a capability token — the authorization path) **and** carries **three** independent canned-green traps (identity-verify, at-rest-seal, SIEM-redaction) each identical to the vacuous-count class opus-4-6 shipped on 11.2a. 11.4a (F1) and 11.2b (F1) both escalated opus-4-6→opus-4-8 on exactly this evidence; John flagged after 11.4a that "opus-4-6 + §A6 on a security-adjacent story is a standing escalation trigger." A split-tier story (part 4-6, part 4-8) also risks the slice boundary being drawn loosely at review. **Resolve: whole-story opus-4-8 (recommended), or opus-4-6 with the token-issuance slice on opus-4-8 AND the full §A6 net (incl. Test-Infra + runtime-execution) MANDATORY on all three subsystems.**

### F2 — At-rest encryption vs the ratified Option-A plaintext decision. **[LEAN: additive opt-in AEAD-at-rest, OFF by default; Option-A plaintext stays the default posture]**
The requirement says "encrypted-at-rest **memory** with org KMS" (`product-scope.md:70`), but 9.4b/ADR-049 **ratified plaintext rows** region-bound by governance only — `write_entry_point.rs:4-9` states verbatim: "Under Option A memory rows are **plaintext** … no per-row encryption … no cryptographic backstop the way an AEAD-at-rest design would have (**Murat**)." Encrypting rows by default would **reverse a ratified decision** and break the byte-identical-default posture every prior enterprise story preserved. Murat's own comment anticipates the AEAD-at-rest design as the thing Option-A deliberately left room for. **LEAN: 11.4c adds an opt-in envelope-AEAD-at-rest layer** (a KMS-wrapped data key seals the loom-lite store rows and/or the TL payload via the existing `CryptoProvider::seal_for_export` AES-256-GCM primitive, `crypto.rs:78`) that is **enabled only when `MAOS_KMS_*` is configured**; with no KMS env, the write path is byte-identical Option-A (AC4 proves this). **Resolve: (a) the opt-in boundary (which stores — loom-lite rows, TL payload, or both); (b) that Option-A remains the default and is NOT reversed; (c) that this does not require a kernel-core write-path edit (the seal wraps at the adapter/store layer, not inside `write_entry_point.rs`) — confirm at Task 0, else it is a FLAG-Winston fork.**

### F3 — KMS backend: reference-only vs production matrix. **[LEAN: ship the `KeyManagementPort` + envelope mechanism + ONE reference adapter; Vault/AWS/GCP additive-per-port, deferred]**
Gap-map: "Vault/cloud-KMS secret backends" is **NOT absorbed by Epic 11** (`full-prd-gap-map…:34`); `maos-secrets` is an **empty placeholder** (`crates/maos-secrets/src/lib.rs` doc-only). Three production KMS backends in a 5-AC bundle is over-scope and each needs a `services:` block / external creds (the advisory-skip trap of the Postgres legs). **LEAN (mirror 11.4a F3):** ship the engine-agnostic `KeyManagementPort` + the envelope/data-key mechanism + **ONE in-process reference KMS** (local master-key from env/keyring — CI-runnable, so the at-rest tripwire is a **real per-commit gate**, not advisory-skipped). AWS-KMS/Vault/GCP are additive-per-port (ADR-010), named in ADR-051. `maos-secrets` (the placeholder) is the natural home for the reference adapter. **Resolve: the reference KMS shape (local-master-key) and that the port makes cloud backends additive.**

### F4 — OIDC verification: real IdP-JWKS vs offline static-JWKS. **[LEAN: real RS256/ES256 signature verification against a configured issuer JWKS; static-JWKS test vector for the CI tripwire]**
No OIDC/OAuth/JWT code or deps exist (`grep` clean; `jsonwebtoken`/`openidconnect` absent from every Cargo.toml). The identity thesis is "the assertion is **really** verified" — so the verify MUST be a real cryptographic signature check (issuer signature over the JWT, `exp`/`nbf`/`aud`/`iss` claims enforced), fail-closed on forged/expired/untrusted-issuer/wrong-audience. **LEAN:** use a vetted OIDC/JWT library (`jsonwebtoken` for verification; optionally `openidconnect` for discovery) confined to the `maos-sso` leaf crate; the CI gate runs the verify against a **static test JWKS + a signed-with-the-right-key vs signed-with-the-wrong-key** pair so the deny is a **real per-commit tripwire** (Cedar-in-process analogue — not advisory-skipped like a live-IdP leg would be). `sso-fault-inject` stubs the verifier to accept-all → the forged-token test reds. **Resolve: the library choice + license (deny.toml), the offline-JWKS CI vector, and the fail-closed claim set (`exp`/`iss`/`aud`/signature all enforced).**

### F5 — SIEM format + transport. **[LEAN: NDJSON + CEF + RFC5424-syslog framing; pluggable sink (file / TCP-socket / HTTPS); redaction via `query_with_redaction` applied BEFORE forwarding]**
NFR-Obs-4: "Transparency Log … exportable to JSONL/**SIEM** with **redaction policy applied**." The substrate exists — `maos-audit` has `AuditEntry` (`lib.rs:91`), `query`/`query_with_redaction` (9.2b), `to_ndjson`/`to_fr4_ndjson` formatters, and 30 frame `kind`s (`lib.rs:660`). No siem/syslog/cef/forward path exists (explicitly "deferred to v2.0", `maos-telemetry/src/lib.rs:30-32`). **LEAN (mirror 11.4b escape-consumer):** a new read-only TL-tail consumer (`maos-siem` + `siem_export_consumer.rs` in `maos-bin`, mirroring `escape_detector_consumer.rs`) projects each `AuditEntry` → CEF (ArcSight) / RFC5424-syslog / NDJSON and forwards to a configured sink, with `query_with_redaction` applied **before** projection. `siem-fault-inject` bypasses redaction → the redaction-leak test reds; the forwarded-record count is **derived** from the real TL tail (never a literal). **Resolve: the format set (CEF + syslog + NDJSON?), the sink transports, and that redaction is mandatory-before-forward with a redaction-completeness derive-and-reconcile.**

### F6 — Crate layout: three leaf crates vs one bundle. **[LEAN: three focused leaf adapter crates + ports in maos-domain; KMS reference folds into the maos-secrets placeholder]**
Dependency closures differ sharply (OIDC libs ≠ KMS SDK ≠ SIEM format libs), and `check-dependency-closure` keeps kernel-core/domain free of all three. **LEAN:** `IdentityAssertionPort` + `KeyManagementPort` + `SiemSinkPort` (or `SiemProjectionPort`) all in `crates/maos-domain/src/ports/`; adapters in `crates/maos-sso/` (new), `crates/maos-secrets/` (the placeholder — KMS reference), `crates/maos-siem/` (new); each depended on ONLY by `maos-bin`. Consumer/runtime wiring modules in `maos-bin` (`sso_identity_runtime.rs`, `at_rest_seal.rs`, `siem_export_consumer.rs`). **Resolve: three crates vs one `maos-enterprise` bundle (lean = three, for closure hygiene), and the port names.**

### F7 — ADR + NFR authoring. **[LEAN: author ADR-051 (lead-with-it) + new NFRs for SSO-identity and opt-in at-rest; annotate NFR-Aud-11 SIEM half as delivered]**
Next free ADR = **ADR-051** (`docs/adr/` has 050; 051 absent — verified). Mirror the 11.4a ADR-050 / 11.4b ADR-024 shape (`Status: binding-v2.0` gated on AC1-AC4 red→green; considered-and-rejected; Gate; Ratification roster). SSO-identity and at-rest have **no upstream FR/NFR ID** (prose-only in the v2.0 scope bullets), so the story authors them. NFR-Aud-11 already exists ("SIEM export at v2.0") — 11.4c **annotates** its SIEM half satisfied (OTel first half was 9.5b). **Honesty clauses (F2/F3):** the at-rest NFR must state "opt-in; Option-A plaintext remains the default" (do NOT claim universal encryption); the SSO/KMS NFRs must state "one reference protocol/backend; SAML + Vault/cloud-KMS additive-per-port, deferred" (do NOT claim the full matrix). **Resolve: ADR-051 scope + the new NFR IDs/text + the NFR-Aud-11 annotation wording + the coverage-matrix trace (Mary: primary NFR + an ADR-051 anchor, both cited "or it traces to nothing").**

### F8 — The principal→capability-token flow-through seam (the kernel-delta landmine). **[LEAN: principal populates `principal_attributes` + audit provenance ONLY; NEVER a field on the frozen token — ZERO-Δ]**
"SSO assertions flow through to capability-token issuance" is the seductive kernel-delta trap. `CapabilityToken` (`i1.rs:175`) is `#[non_exhaustive]`, kernel-core, ABI-frozen, `spirit_pid`-bound, and issued only by `cap_tokens::issue`. Adding a principal field = kernel-core delta + ABI break = **FORBIDDEN**. The already-shipped, additive `PolicyDecisionRequest.principal_attributes` (`policy_decision.rs:112`, `None` in 11.4a — shaped by 11.4a F7 *for this story*) is the sanctioned attach point: the authenticated principal (a) populates the PDP request that **authorizes** issuance (off-hot-path, `maos-domain`, ZERO kernel-core delta), and (b) is recorded in the **audit provenance** of the issuance event (the `maos-audit` channel). The "flow-through" is *governance + provenance*, not a token field. **Resolve: confirm (Task 0) that this needs no `cap_tokens::issue` / kernel-core edit; if a preflight judges a genuine issuance-time principal binding unavoidable, that is a bounded FLAG-Winston re-pin from 23081 — never silent — and the LEAN says it is NOT needed.**

---

## ⚠️ PREFLIGHT FLAGS (landmines — read first)

- **L1 — ZERO kernel-core delta is the design constraint, not an outcome.** Touch NO `.rs` under `crates/maos-kernel-core/src`. All three capabilities are `maos-domain` ports + leaf adapters + `maos-bin` wiring. `check-kernel-baseline` @ `src_lines = 23081` reds on ANY drift (it counts test + doc lines too — even a comment reflow of kernel-core reds). See F8.
- **L2 — The Option-A reversal trap (F2).** Do NOT make at-rest encryption the default. Default (no `MAOS_KMS_*`) MUST be byte-identical Option-A plaintext. AC4 proves the zero-config byte-identity — a silent default flip is a P0-class finding (reverses 9.4b/ADR-049).
- **L3 — The principal-in-token trap (F8).** Do NOT add a principal/identity field to `CapabilityToken` or edit `cap_tokens::issue`. Principal → `principal_attributes` + audit provenance only.
- **L4 — Three independent canned-green traps.** A stubbed OIDC verifier (accept-all), a passthrough "seal" (plaintext masquerading as ciphertext), and a redaction-bypass SIEM forwarder each **look identical to the real thing until falsified**. Every count/claim needs a per-item blind that reds it (F1 escalation rationale): blind the verifier → forged-token test reds; passthrough the seal → ciphertext≠plaintext test reds; blind redaction → leak test reds; blind one principal source → identity-provenance count reds.
- **L5 — The advisory-skip trap.** A live-IdP / live-cloud-KMS / live-SIEM-collector leg that needs external provisioning runs `skipped` (advisory only) — the exact caveat that weakened `check-multi-region-slo`/`check-scale-churn`. Keep the tripwire legs **in-process/offline** (static-JWKS verify, local-master-key seal, file-sink forward) so they are **real per-commit gates**; any genuinely-live leg is an ADDITIONAL advisory leg, never the tripwire (F4/F3/F5).
- **L6 — SIEM redaction is mandatory-before-forward.** Forwarding raw `AuditEntry` payloads to an external SIEM leaks exactly what NFR-Obs-4's redaction policy exists to stop. Redaction (`query_with_redaction`, 9.2b) applies BEFORE projection; the leak test is the proven-red (F5).
- **L7 — Dependency closure.** OIDC libs / KMS SDK / SIEM format libs stay in their leaf crates. `check-dependency-closure` must show `maos-kernel-core` / `maos-domain` free of all three. Confirm licenses in `deny.toml` at Task 0.
- **L8 — Export-control surface (5D002.c.1).** At-rest AEAD + IdP-TLS + SIEM-TLS are new crypto/encryption surfaces. No new *kernel* crypto (route AEAD through `CryptoProvider`); flag the surfaces to export counsel (dev-gate). See Dev-gate note.
- **L9 — NFR honesty clauses (F7).** Do NOT mark "encrypted-at-rest" universally satisfied (it is opt-in — L2/F2), and do NOT mark "org-KMS/SAML" fully satisfied (reference-only — F3/F4). Annotate the scope in ink.

---

## Decisions ledger (RESOLVED — party-mode 2026-07-06; see ✅ PREFLIGHT RESOLVED for the rulings)

| # | Decision (ratified) | Source |
|---|-----------------|--------|
| D1 | **[RESOLVED]** Whole-story **opus-4-8 + full §A6**; **AC count 5→6** (split-tier rejected; AC1→AC1+AC2 for the SSO flow-through) | F1; `epic-11…md:31,48`; 11.4a F1 |
| D2 | **[RESOLVED — scoped]** At-rest AEAD **opt-in, OFF by default**, scoped to **loom-lite Collective rows + out-of-kernel identity assertion writes**; generic kernel-authored TL payloads and kernel-core Private/Shared **deferred**; Option-A default | F2; `write_entry_point.rs:4-9`; ADR-049 |
| D3 | **[RESOLVED — labeled]** `KeyManagementPort` + envelope + **ONE reference local-master-key KMS marked dev/CI-ONLY, not production-recommended**; keyring/cloud additive-per-port | F3; `full-prd-gap-map…:34`; `maos-secrets` |
| D4 | **[RESOLVED — hardened]** OIDC verify = real signature + **explicit alg allowlist**; `alg:none`/confusion **rejected as gate negatives**; `exp`/`nbf`/`aud`/`iss`/JWKS-fail all fail-closed; offline vector = tripwire | F4/Vex; grep-clean; L5 |
| D5 | **[RESOLVED — tightened]** SIEM = read-only TL-tail consumer → **NDJSON + CEF** (RFC5424 = transport, not a 3rd format); **HTTPS sink TLS-only**; **redaction-before-forward** | F5; NFR-Obs-4; `escape_detector_consumer.rs` |
| D6 | **[RESOLVED]** Three leaf crates (`maos-sso`, `maos-secrets`, `maos-siem`) + 3 ports in `maos-domain`; wired in `maos-bin` | F6; `check-dependency-closure` |
| D7 | **[RESOLVED]** Author **ADR-051** + **NFR-Sec-18** (SSO) + **NFR-Sec-19** (at-rest) + annotate **NFR-Aud-11** SIEM half; honesty clauses IN the requirement text | F7; ADR-050/ADR-024 shape |
| D8 | **[RESOLVED — refined]** Principal → `principal_attributes` (authorization) + **NEW out-of-kernel `identity.asserted` audit kind** (provenance, authorization-layer correlation); **NEVER** a `CapabilityToken`/issuance-record field; **ZERO kernel-core delta** | F8/Grumbal; `policy_decision.rs:112`; `i1.rs:175` |
| D9 | **[RESOLVED]** `check-enterprise-identity` gate, per-leg independent (6 legs), disposition `{v1_0=advisory, v1_5=advisory, v2_0=blocking}`, absent→BLOCK@v2.0 | AC6; 11.4a/11.4b gate idiom |
| D10 | **[RESOLVED — Grumbal catch]** Fail-closed proven **PER subsystem** in AC5: SSO-down→deny, KMS-down→refuse-not-plaintext, SIEM-down→buffer-not-drop (3 distinct proven-reds) | CATCH; Vex |

---

## Acceptance Criteria (6 — RESOLVED at preflight 2026-07-06; §A5 ≤6 ceiling; AC1 split into AC1+AC2 per F1; falsifiers fold into their parent AC as proven-red, per the 11.2b pattern)

**AC1 — Enterprise OIDC assertions are really verified, fail-closed, with the algorithm allowlist enforced.**
**Given** the kernel today has no federated principal — the authorization subject is `spirit_pid: u32` (`i1.rs:175`) — and no OIDC/JWT code or deps exist (grep-clean);
**When** an OIDC assertion (JWT) is presented and the `IdentityAssertionPort` (maos-domain) → `maos-sso` adapter verifies it against the configured JWKS with an **explicit algorithm allowlist** (`Validation` pins RS256/ES256; `alg:none` rejected; HS256-signed-with-the-RS256-public-key rejected — the alg-confusion CVE class, F4/Vex), enforcing signature + `exp`/`nbf` + `aud` (= this deployment, no cross-RP replay) + `iss` (allowlist), fail-closed on JWKS-fetch-failure;
**Then** a correctly-signed, in-audience, live assertion **verifies**, and every failing class — forged signature, expired, wrong-audience, untrusted-issuer, `alg:none`, alg-confusion — is **rejected fail-closed** (no principal produced) — proven-red by the gate's `oidc-verify` leg over a static-JWKS **right-key vs wrong-key** vector PLUS the **`alg:none` + confusion negatives** (a real per-commit leg, offline — not advisory; L5);
**And** `sso-fault-inject` (stubbing the verifier to accept-all) makes the forged-token + alg-negative tests **red the leg** (L4), and `check-kernel-baseline` proves `src_lines` unchanged @ **23081** (a kernel-core `cargo fmt` reflow reds — 10.5 R3).

**AC2 — The authenticated principal governs the authorization of capability-token issuance and is provenance-recorded out-of-kernel (the opus-4-8 slice; F8).**
**Given** `PolicyDecisionRequest.principal_attributes` is `Option<HashMap<String,String>>` = `None` in 11.4a (`policy_decision.rs:112`, shaped by 11.4a F7 for this story), and `cap_tokens::issue` / the issuance audit record are **kernel-core, ABI-frozen** (`i1.rs:175`) — no `token_id` exists at assertion time (minted downstream);
**When** a verified principal (from AC1) populates `principal_attributes` on the PDP request that **governs the authorization** of a capability grant, and the `maos-sso` adapter emits a **NEW out-of-kernel `identity.asserted` audit kind** (the 11.4b `kind=8` pattern) correlating the principal to the governed authorization **at the authorization layer** ("principal X governed `spirit_pid` P's grants at decision-time T" — never "principal X owns token Y");
**Then** the principal flows into the PDP authorization and an `identity.asserted` provenance entry is written **with ZERO kernel-core delta** — no `CapabilityToken` field, no `cap_tokens::issue` edit, no mutation of the kernel's issuance audit record (F8/L3) — proven by the `principal-provenance` leg;
**And** the `identity-source-blind` reflex (§A7): blinding one issuance's principal source (stub/cache the assertion) makes the provenance-count reconcile **red** (a canned/cached principal masquerade cannot pass), and ADR-051 states the authorization-layer scope in ink (a reviewer WILL read the requirement literally — Vex/John).

**AC3 — Org-KMS envelope encryption-at-rest is real ciphertext, opt-in, scoped to provable out-of-kernel persisted rows, Option-A-default-preserving.**
**Given** disk-persisted rows are **plaintext** today ("Option A", ADR-049) — Collective rows in `maos-loom-lite` (`schema.rs:33-56`, adapter) are in scope; out-of-kernel `identity.asserted` writes are isolated in `maos-audit`; generic kernel-authored TL payloads and kernel-core Private/Shared (`write_entry_point.rs:4-9`) are **out of scope** (kernel write-path seams — F2/deferred); `maos-secrets` is an empty placeholder;
**When** org-KMS is configured (`MAOS_KMS_*`) and the `KeyManagementPort` (maos-domain) → reference adapter (`maos-secrets`) wraps a data key with the org master key and the **loom-lite adapter layer** seals rows via the existing `CryptoProvider::seal_for_export` AES-256-GCM (`crypto.rs:78`) — NOT inside kernel-core (F2/L1);
**Then** sealed in-scope rows on disk are **ciphertext ≠ plaintext**, decrypt correctly with the right unwrapped key, and **fail to decrypt with the wrong key** (real AEAD, not a key-ignoring passthrough — the exact fake to hunt, CARRIED) — proven-red by the `at-rest-seal` leg;
**And** with **no** `MAOS_KMS_*` env the write path is **byte-identical Option-A plaintext** (the default is NOT reversed — L2/F2, cross-checked by AC5), and `kms-fault-inject` (passthrough "seal") makes the ciphertext≠plaintext + wrong-key-fails tests **red the leg** (L4).

**AC4 — SIEM export forwards the Transparency Log with redaction applied before projection.**
**Given** `maos-audit` exposes `AuditEntry` (`lib.rs:91`), `query`/`query_with_redaction` (9.2b), `to_ndjson`, and 30 frame `kind`s (`lib.rs:660`), and NFR-Obs-4 requires "exportable to JSONL/SIEM **with redaction policy applied**", but no SIEM forward path exists (deferred v2.0, `maos-telemetry/src/lib.rs:30-32`);
**When** SIEM export is configured (`MAOS_SIEM_*`) and a read-only TL-tail consumer (`maos-siem` + `siem_export_consumer.rs`, mirroring `escape_detector_consumer.rs`) applies `query_with_redaction` **before** projecting each entry → **NDJSON + CEF** (two content formats) framed for **RFC5424 syslog transport**, forwarding to a configured sink where the **HTTPS/network sink is TLS-only** (plaintext-TCP/file = localhost only — F5/Vex);
**Then** forwarded records carry the redaction policy applied (secret-class fields scrubbed — no leak), the forwarded-record count is **derived** from the real TL tail (never a literal — derive-and-reconcile; empty TL → reported **N/A, not a vacuous pass**), and this delivers **NFR-Aud-11's SIEM half** (the OTel first half is 9.5b, untouched);
**And** `siem-fault-inject` (bypassing redaction) makes the **redaction-leak** test **red the leg**, and blinding one entry's redaction reds the redaction-completeness reconcile (L4/L6).

**AC5 — Additive-only, zero-config byte-identical, and fail-closed PER subsystem (Grumbal's catch, folded as a constraint).**
**Given** all three capabilities are optional enterprise integrations that must not change default v1.5 behavior;
**When** the daemon starts with **none** of `MAOS_SSO_*` / `MAOS_KMS_*` / `MAOS_SIEM_*` configured — and, separately, with each integration **configured but unavailable**;
**Then** unconfigured → **byte-identical to v1.5** (no principal required, plaintext Option-A rows, no SIEM forwarding — the `additive-byte-identical` leg, mirroring 11.4a AC1); and each **configured-but-down** integration fails **closed and loud**, proven **distinctly**: **SSO IdP unreachable → issuance DENIED** (never fall open to `spirit_pid`); **KMS unreachable → sealed write REFUSED** (never silently write plaintext under an encryption posture — the #1 real-world at-rest defeat, Vex); **SIEM sink down → buffer + operator-visible error** (never silent-drop the audit trail);
**And** ZERO kernel-core delta @ **23081** and `check-dependency-closure` shows `maos-kernel-core`/`maos-domain` free of the OIDC/KMS/SIEM client crates (L7) — the three fail-closed proofs are three distinct proven-reds in the `additive-and-failclosed` leg.

**AC6 — NEW `check-enterprise-identity` gate + ADR-051 + NFRs (NFR-Sec-18/19 + NFR-Aud-11 annotation), absent→BLOCK@v2.0.**
**Given** every new Epic-11 gate records `{v1_0, v1_5, v2_0}` disposition and absent-result flips to BLOCK at the v2.0 ship gate (`epic-11…md:30`);
**When** the `check-enterprise-identity` xtask gate is authored (copy `check_enterprise_pdp.rs`/`check_escape_detector.rs`) with **per-leg independence** — `oidc-verify` (AC1, incl. alg-negatives) · `principal-provenance` + `identity-source-blind` reflex (AC2) · `at-rest-seal` (AC3) · `siem-redaction-export` (AC4) · `additive-and-failclosed` (AC5, three distinct fail-closed reds) · `fault-inject-falsifiers` · `available-arm-integration` · `issuance-bypass-absence` · `release-graph-absence` · `kernel-abi-diff` — each reading its OWN oracle so one break reds exactly one leg, with the vacuous-green hard-fail (attempted-but-zero-tests reds), and enrolled 5 ways (`xtask/src/main.rs`; `discipline.yml`; `gate-registry.toml` flat `gates` + `[[ship_gate]]`; `check_ship_gate_completeness.rs`; `tests/coverage-matrix.yaml`);
**Then** the gate disposition is `{v1_0=advisory, v1_5=advisory, v2_0=blocking}` (D9), the three `*-fault-inject` features are `compile_error!`-guarded out of the release graph (`cargo tree -e features --release | grep` SHIP-BLOCKER), and **ADR-051** is authored (lead-with-it; `binding-v2.0` gated on AC1-AC5 observed red→green) documenting the F8 authorization-layer provenance scope + the F2 store boundary + the F3 reference-KMS-is-dev-only caveat;
**And** the NFRs carry the **honesty clauses IN the requirement text** (L9/F7): **NFR-Aud-11** SIEM half marked delivered ("OTel first-half = 9.5b"); **NFR-Sec-18** (SSO/OIDC identity) states "one reference OIDC protocol; SAML additive-per-port, **deferred**"; **NFR-Sec-19** (opt-in at-rest AEAD) states "opt-in; loom-lite Collective rows plus out-of-kernel identity assertion writes; Option-A plaintext **default**; generic kernel-authored TL payloads and kernel-core Private/Shared **deferred**; reference local-master-key KMS is **dev/CI-only, not production-recommended** — production = keyring/cloud-KMS additive-per-port" — **NONE** marked universally satisfied — and each new NFR cites **ADR-051 + its scope-bullet source** (Mary: "both cited or it traces to nothing").

---

## §A7 gate-source mapping (name each gate leg's discipline — required deliverable per `epic-11…md:71`)

| Leg (AC) | §A7 source | derive-and-reconcile numerator | real-subsystem proven-red | canned-trap avoided |
|----------|-----------|--------------------------------|---------------------------|---------------------|
| `oidc-verify` (AC1) | §A7.1 real-subsystem | accepted vs rejected assertions derived per-run from real JWKS verify (incl. alg-negatives) | real RS256/ES256 verify over static-JWKS right-vs-wrong-key + `alg:none`/confusion negatives | stubbed accept-all verifier (`sso-fault-inject`) reds |
| `principal-provenance` (AC2) | §A7 identity-provenance reflex (region-identity analogue) | each governed authorization traces to a real verified OIDC principal; `identity.asserted` count reconciled to governed authorizations | real principal → `principal_attributes` + real `identity.asserted` audit entry | canned/cached principal masquerade; `identity-source-blind` reds |
| `at-rest-seal` (AC3) | §A7.1 real-subsystem | sealed-row ciphertext≠plaintext + wrong-key-fails, derived from real AEAD | real `CryptoProvider` AES-256-GCM seal/open at loom-lite/audit adapter layer | passthrough / key-ignoring "seal" (`kms-fault-inject`) reds |
| `siem-redaction-export` (AC4) | §A7.1 derive-and-reconcile | forwarded-record count + redacted-field count derived from real TL tail (empty→N/A) | real `query_with_redaction` → NDJSON/CEF projection → sink | redaction-bypass (`siem-fault-inject`) leak reds |
| `additive-and-failclosed` (AC5) | §A7.3 feature-flag≠measurement | default-posture output byte-compared to v1.5; 3 distinct configured-but-down outcomes | real daemon: no-env byte-identical + SSO-down/KMS-down/SIEM-down live | silent default-flip / fail-OPEN (deny→allow, refuse→plaintext, buffer→drop) reds |
| `kernel-abi-diff` | §A7.4 tripwire / additive-only | `src_lines` @ 23081 + abi additive-only | in-process `check_kernel_baseline::run` | kernel-core reflow / hidden edit (F8 `identity.asserted` stays out-of-kernel) reds |

**§A7 identity-provenance reflex** (the epic's `epic-11…md:29` region-identity-reflex analogue, named for this story): a count over governed capability-issuances MUST verify each issuance's principal traces to a **real verified OIDC assertion** (real issuer signature, not a stubbed/cached/canned principal); blind one source → the count reds. Direct analogue of 11.4a's `decision-provenance` reflex and 11.4b's `escape-source-identity` reflex.

---

## Tasks / Subtasks

- [x] **Task 0 — Preflight verification (DONE — party-mode 2026-07-06; forks RESOLVED, surfaces confirmed).** (all ACs)
  - [x] Party-mode preflight (Winston · Murat · John · Amelia · Mary + Vex/security & Grumbal/adversary + Paige) ratified F1–F8 + Grumbal's fail-closed catch; ✅ PREFLIGHT RESOLVED block recorded above.
  - [x] `PolicyDecisionRequest.principal_attributes` (`policy_decision.rs:112`) confirmed `None`/unused in 11.4a → **F8: no `CapabilityToken` / `cap_tokens::issue` edit; provenance via a NEW out-of-kernel `identity.asserted` audit kind**, correlated at the authorization layer (no `token_id` at assertion time).
  - [x] `CryptoProvider::seal_for_export` (AES-256-GCM, `crypto.rs:78`) confirmed reusable; the seal wraps at the **loom-lite adapter layer**, NOT `write_entry_point.rs` and NOT generic kernel-authored TL writes — kernel-core Private/Shared and generic TL payload at-rest are **out of scope/deferred** (F2).
  - [x] `maos-audit` `query_with_redaction` + `to_ndjson` + `kind` map (`lib.rs:660`) confirmed sufficient for the SIEM projection; `append_identity_asserted` is the out-of-kernel identity provenance write; `escape_detector_consumer.rs` is the read-only-TL-tail template.
  - [x] Option-A plaintext is the ratified default and MUST remain so (L2/F2); **opt-in boundary = loom-lite Collective rows + out-of-kernel identity assertion writes only**.
  - [x] **(dev)** License/dependency-closure check for the OIDC (`jsonwebtoken`; `openidconnect` only if discovery is needed) + reference-KMS + SIEM-format crates in `deny.toml`; confirm they stay in leaf crates (L7).
  - [x] Next ADR = **ADR-051** confirmed (`docs/adr/` has 050, 051 absent). Tier set: **opus-4-8 whole-story**; §A6 pre-booked (Blind · Edge · Acceptance · Test-Infra + runtime-execution).
- [x] **Task 1 — `IdentityAssertionPort` + `maos-sso` OIDC verify (fail-closed + alg allowlist).** (AC1)
  - [x] Add `IdentityAssertionPort` to `crates/maos-domain/src/ports/` (object-safe, sync; `verify(assertion) -> Result<AuthenticatedPrincipal, IdentityError>`; fail-closed error set incl. `JwksUnavailable`, `AlgorithmRejected`, `AudienceMismatch`, `IssuerUntrusted`).
  - [x] New `crates/maos-sso/` leaf crate (`#![forbid(unsafe_code)]`): real RS256/ES256 JWKS verify with an **explicit `Validation` algorithm allowlist** (reject `alg:none`; reject HS256-with-RS256-pubkey confusion); enforce signature + `exp`/`nbf` + `aud` (this deployment) + `iss` (allowlist); fail-closed on JWKS-fetch-failure.
  - [x] `sso-fault-inject = []` feature (accept-all stub) + `compile_error!(all(feature="sso-fault-inject", not(debug_assertions)))`.
  - [x] Tests: `crates/maos-sso/tests/oidc_verify.rs` (right-key vs wrong-key static JWKS), `alg_negatives.rs` (**`alg:none` + HS256-confusion rejected**), `claims_failclosed.rs` (expired/wrong-aud/untrusted-iss/jwks-down).
- [x] **Task 2 — Principal governs authorization + `identity.asserted` out-of-kernel provenance (the opus-4-8 slice; F8).** (AC2)
  - [x] `maos-sso` populates `principal_attributes` on the PDP request that **governs** issuance authorization (off-hot-path; `maos-domain`, no kernel edit).
  - [x] Add a **NEW `identity.asserted` audit kind** to `maos-audit`'s `kind` map (next free discriminant after 29) emitted by the `maos-sso` adapter (out-of-kernel write path — the 11.4b `kind=8` pattern); correlate to the governed authorization at the **authorization layer** (principal + `spirit_pid` + decision context — NOT `token_id`).
  - [x] Tests: `crates/maos-sso/tests/principal_governs.rs` (principal reaches PDP request), `identity_provenance.rs` (`identity.asserted` entry written + reconciled to governed authorizations), `identity_source_blind.rs` (blind one principal source → provenance count reds — the §A7 reflex).
  - [x] **Guard:** confirm the new `kind` addition lives in `maos-audit` (adapter), not kernel-core → `check-kernel-baseline` stays @ 23081.
- [x] **Task 3 — `KeyManagementPort` + reference KMS in `maos-secrets` + opt-in at-rest AEAD (adapter stores).** (AC3)
  - [x] Add `KeyManagementPort` to `crates/maos-domain/src/ports/` (wrap/unwrap data key; `is_healthy`; fail-closed errors).
  - [x] Implement the **reference local-master-key** KMS in `crates/maos-secrets/` (replace the placeholder) — **marked dev/CI-only, not production-recommended** (F3); seal rows via `CryptoProvider::seal_for_export` at the **loom-lite + audit adapter layers** (F2 — NOT `write_entry_point.rs`).
  - [x] `kms-fault-inject = []` feature (passthrough / key-ignoring seal) + `compile_error!` guard.
  - [x] Tests: `crates/maos-secrets/tests/at_rest_seal.rs` (ciphertext≠plaintext, right-key-opens, **wrong-key-FAILS** — hunt the key-ignoring fake), `default_plaintext_preserved.rs` (no KMS env → Option-A byte-identical).
- [x] **Task 4 — `SiemProjectionPort` + `maos-siem` read-only TL consumer (redaction-before-forward, TLS sink).** (AC4)
  - [x] Add `SiemProjectionPort` to `crates/maos-domain/src/ports/`.
  - [x] New `crates/maos-siem/` leaf crate: apply `query_with_redaction` BEFORE projecting `AuditEntry` → **NDJSON + CEF** framed for **RFC5424 syslog transport** (L6); `siem_export_consumer.rs` in `maos-bin` tails the TL read-only (mirror `escape_detector_consumer.rs`); pluggable sink — **HTTPS/network = TLS-only**, plaintext-TCP/file = localhost only (F5/Vex).
  - [x] `siem-fault-inject = []` feature (redaction-bypass) + `compile_error!` guard.
  - [x] Tests: `crates/maos-siem/tests/redaction_before_forward.rs` (leak-red), `format_projection.rs` (NDJSON + CEF + RFC5424-framing shape), `forward_count_derive.rs` (derive-and-reconcile, empty→N/A).
- [x] **Task 5 — Composition-root wiring in `maos-bin` (env-gated, byte-identical default, fail-closed PER subsystem).** (AC5)
  - [x] Wire the three ports as `Option<Arc<dyn Port>>` at the composition root (mirror `enterprise_pdp_runtime.rs`): `MAOS_SSO_*`, `MAOS_KMS_*`, `MAOS_SIEM_*`; unconfigured → v1.5 byte-identical (Option-A plaintext, no SSO, no forward).
  - [x] **Fail-closed PER subsystem (Grumbal/D10):** SSO IdP unreachable → **issuance denied** (not `spirit_pid` fallthrough); KMS unreachable → **sealed write refused** (not silent plaintext); SIEM sink down → **buffer + operator-visible error** (not silent drop).
  - [x] Tests: `crates/maos-bin/tests/enterprise_identity_wiring.rs` (zero-config byte-identity + **three distinct configured-but-down fail-closed** outcomes).
- [x] **Task 6 — The `check-enterprise-identity` gate + 5-point enrollment.** (AC6)
  - [x] Copy `xtask/src/check_enterprise_pdp.rs`; legs: `oidc-verify` (incl. alg-negatives) · `principal-provenance` (+ `identity-source-blind`) · `at-rest-seal` · `siem-redaction-export` · `additive-and-failclosed` (3 distinct fail-closed reds) · `kernel-abi-diff` (per-leg independent; vacuous-green hard-fail; `run_kernel_abi_leg` → `check_kernel_baseline::run(false)`).
  - [x] Enroll 5 ways: `xtask/src/main.rs` (mod + `#[command]` + dispatch); `.github/workflows/discipline.yml` (v1-0 + v1-5-ship-gate `needs`/summary/fail); `xtask/gate-registry.toml` (flat `gates` + `[[ship_gate]]` disposition `{v1_0=advisory, v1_5=advisory, v2_0=blocking}`); `check_ship_gate_completeness.rs` (`EXPECTED_GATES` + match arm); `tests/coverage-matrix.yaml` (NFR row).
  - [x] The three `*-fault-inject` features: `cargo tree -e features --release | grep -q` SHIP-BLOCKER for each (absent-from-release-graph leg).
- [x] **Task 7 — ADR-051 + NFRs (author; lead-with-the-ADR).** (AC6)
  - [x] Author `docs/adr/ADR-051-enterprise-identity-at-rest-siem.md` (mirror ADR-050/ADR-024: `Status: binding-v2.0` gated on AC1-AC5 red→green; Context code-survey; **considered-and-rejected** [in-kernel identity/at-rest, **principal-in-token / mutate-issuance-audit**, default-encryption, cloud-KMS-matrix-now]; document the **F8 authorization-layer provenance scope**, the **F2 adapter-store boundary + Private/Shared deferral**, the **F3 reference-KMS-is-dev-only** caveat; Gate; Ratification roster + workflow id). Update `docs/adr/index.md`.
  - [x] Author **NFR-Sec-18** (SSO/OIDC identity) + **NFR-Sec-19** (opt-in at-rest AEAD) with **honesty clauses IN the requirement text** (L9); annotate **NFR-Aud-11** SIEM half delivered. Update `prd/non-functional-requirements.md` + `requirements-inventory.md`; coverage-matrix trace (each new NFR cites ADR-051 + its scope-bullet source).
- [x] **Task 8 — Runtime-execution check (§A6 net).** (all ACs)
  - [x] Run `cargo run -p xtask -- check-enterprise-identity --json` live; assert all 7 legs green, each `*-fault-inject` reds its leg, the alg-negatives reject, all three fail-closed outcomes hold, all three fault-inject features absent from the release graph, `check-kernel-baseline` GREEN @ **23081**, `check-dependency-closure` GREEN, `check-ship-gate-completeness` GREEN, no sibling regressions (`maos-audit`, `maos-pdp`, `maos-domain`, `maos-loom-lite`, core lib).

---

## Dev Notes

### The three substrates already exist — build ON them, do not re-implement
- **Identity attach point (11.4a F7, shaped for this story):** `crates/maos-domain/src/ports/policy_decision.rs:95-112` — `principal_attributes: Option<HashMap<String,String>>`, doc: "populated by the SSO→capability-token issuance slice; kept additive so 11.4c is non-breaking." Populate this for authorization; do NOT touch `CapabilityToken` (`i1.rs:175`, `#[non_exhaustive]`, `spirit_pid`-bound, kernel-core, ABI-frozen — F8/L3). **Provenance is a NEW out-of-kernel `identity.asserted` audit kind** emitted by `maos-sso` (the 11.4b `kind=8` pattern; next free discriminant after 29 in `maos-audit/src/lib.rs:660`), NOT a mutation of the kernel's issuance audit record (F8/Grumbal). Correlate at the authorization layer (principal + `spirit_pid` + decision context — no `token_id` exists at assertion time).
- **Crypto for at-rest:** `crates/maos-domain/src/ports/crypto.rs:78` — `CryptoProvider::seal_for_export(sealing_key, nonce, aad, plaintext)` is **AES-256-GCM AEAD**, already wired (`main.rs:1631` `RingCryptoProvider`). Reuse it for the row seal; the KMS wraps the data key. Do NOT add kernel crypto (L8).
- **Audit/SIEM read substrate:** `crates/maos-audit/src/lib.rs:91` (`AuditEntry`), `:184` (`query`), `query_with_redaction` (9.2b), `:660` (`kind` map, 0..29; e.g. 8=sandbox.block, 19=spirit.admitted), NDJSON formatters (`to_ndjson`/`to_fr4_ndjson`). Redaction is field-level payload scrubbing — apply BEFORE forward (L6).
- **Two composition-root patterns to mirror:** (1) **injected env-gated runtime** — `crates/maos-bin/src/enterprise_pdp_runtime.rs` + `main.rs:1682-1717` (the 11.4a PDP pattern → use for SSO + KMS ports); (2) **standalone read-only TL consumer** — `crates/maos-bin/src/escape_detector_consumer.rs` (the 11.4b pattern → use for SIEM export). Neither is in `api.rs` (keeps `check-composition-root-completeness` green).

### Kernel-delta discipline (§A7.4)
Baseline `src_lines = 23081` (`xtask/kernel-core-baseline.toml:205`). `check_kernel_baseline.rs` recursively counts ALL `.rs` lines (incl. test/doc) under `crates/maos-kernel-core/src` and hard-fails on ANY drift. ZERO expected — three `maos-domain` ports (domain, not kernel-core) + three leaf adapters + `maos-bin` wiring. If a preflight authorizes a genuine kernel seam (it should not — F8), pin the exact new number, disclose in the `kernel-core-baseline.toml` HISTORY ledger naming the surface + the FLAG-Winston date, and document why in ADR-051 (mirror the 11.4a +N entry). Churn outside the named surface — even a `cargo fmt` reflow (10.5 R3) — is a finding: revert or disclose.

### The gate mechanics (mirror `check_enterprise_pdp.rs` verbatim shape)
`GATE_NAME` MUST match the registry row + `#[command(name)]`; `read_disposition()`/`phase_disposition()`/`is_blocking_at()`; `LegResult{label,passed,failed,ran,attempted,green}`; per-leg `invoke_cargo_test` with its OWN filter/feature → one break reds one leg (`green = status.success() && ran && passed>=1 && failed==0`); vacuous-green hard-fail (attempted-but-zero-tests reds at every phase, `kernel-abi-diff` exempt); `oracle_green = legs.iter().all(green)`; phased tail — blocking→`Err`, advisory→WOULD-HAVE-BLOCKED banner to `$GITHUB_STEP_SUMMARY` + `Ok`. Disposition `{v1_0=advisory, v1_5=advisory, v2_0=blocking}` declared ONCE as a `[[ship_gate]]` block; stripping the banner or the `v2_0` row must red `check-ship-gate-completeness`.

### Anti-canned discipline (the reason for opus-4-8 — L4)
Every count/claim needs a per-item blind that reds it: `sso-fault-inject` (accept-all) → forged-token test reds; `kms-fault-inject` (passthrough) → ciphertext≠plaintext test reds; `siem-fault-inject` (redaction-bypass) → leak test reds; blind one principal source → identity-provenance count reds. Keep the tripwire legs **in-process/offline** (static-JWKS, local-master-key, file-sink) so they are real per-commit gates, not advisory-skipped (L5); any genuinely-live IdP/cloud-KMS/SIEM-collector leg is an ADDITIONAL advisory leg. The at-rest seal test must prove **wrong-key-fails** (a passthrough "seal" that ignores the key would pass a naive ciphertext≠plaintext-only check — hunt the exact fake, mirror 11.4a's A→B→A anti-memoize).

### Recurring operational gotchas (pre-empt — from 11.4a/11.4b retros)
- **i64→u32 truncation** on any count/pid surface is the named 11.2a P1 anti-pattern — derive counts, don't truncate.
- **Advisory-because-unprovisioned** (Postgres legs of `check-multi-region-slo`/`check-scale-churn`) — do NOT let the SIEM/KMS/SSO tripwire depend on external provisioning (L5).
- **Silent-drop on backpressure** — SIEM sink-down must buffer + surface an operator-visible error, never drop (AC4).
- **Default-flip** — the at-rest opt-in must NOT change the no-KMS default (L2); AC4's byte-identity leg is the guard.

### Project Structure Notes
- Dependency arrows (all leaf→domain, ZERO kernel-core in-edges): `maos-sso → maos-domain (IdentityAssertionPort)`; `maos-secrets → maos-domain (KeyManagementPort) + maos-domain (CryptoProvider)`; `maos-siem → maos-domain (SiemSinkPort) + maos-audit (read-only)`; all three ← `maos-bin` only. `check-dependency-closure` enforces `maos-kernel-core`/`maos-domain` free of OIDC/KMS/SIEM client crates.
- New crates: `crates/maos-sso/`, `crates/maos-siem/`; `crates/maos-secrets/` (fill the placeholder). New `maos-bin` modules: `sso_identity_runtime.rs`, `at_rest_seal.rs`, `siem_export_consumer.rs`.
- New gate source: `xtask/src/check_enterprise_identity.rs`. New ADR: `docs/adr/ADR-051-out-of-kernel-enterprise-identity-at-rest-siem.md`.

### References
- Story scope + tier + kernel-Δ + depends: [Source: _bmad-output/planning-artifacts/epics/epic-11-v20-technical-phase.md:48] · [Source: epic-11-v20-technical-phase.md:29-31] · [Source: _bmad-output/implementation-artifacts/sprint-status.yaml:190]
- NFR-Aud-11 (headline): [Source: _bmad-output/planning-artifacts/prd/non-functional-requirements.md:69] · [Source: prd/non-functional-requirements.md:221] · [Source: _bmad-output/planning-artifacts/epics/epic-9-audit-compliance-surfaces-operator-productionization-v05-v10.md:36,267-270]
- SSO/OIDC + flow-through: [Source: prd/domain-specific-requirements.md:82] · [Source: prd/product-scope.md:70] · [Source: prd/project-scoping-phased-development.md:244]
- At-rest + org-KMS + Option-A: [Source: prd/product-scope.md:70] · [Source: crates/maos-kernel-core/src/memory/write_entry_point.rs:4-9] · [Source: docs/adr/ADR-049* (region weld sign-only)] · [Source: _bmad-output/planning-artifacts/full-prd-gap-map-and-planning-plan-2026-07-06.md:34 (Vault/cloud-KMS NOT absorbed)]
- SIEM substrate + redaction: [Source: prd/non-functional-requirements.md:100-102 (NFR-Obs-2/4)] · [Source: crates/maos-audit/src/lib.rs:91,184,660] · [Source: crates/maos-telemetry/src/lib.rs:30-32 (SIEM deferred v2.0)]
- The 11.4a attach point (F8) + patterns: [Source: crates/maos-domain/src/ports/policy_decision.rs:95-112] · [Source: crates/maos-domain/src/ports/crypto.rs:50,78,92] · [Source: crates/maos-domain/src/invariants/i1.rs:60,175] · [Source: crates/maos-bin/src/enterprise_pdp_runtime.rs] · [Source: crates/maos-bin/src/escape_detector_consumer.rs]
- Gate + baseline machinery: [Source: xtask/src/check_enterprise_pdp.rs] · [Source: xtask/src/check_escape_detector.rs] · [Source: xtask/gate-registry.toml] · [Source: xtask/src/check_kernel_baseline.rs] · [Source: xtask/kernel-core-baseline.toml:205 (src_lines = 23081)] · [Source: xtask/src/check_ship_gate_completeness.rs]
- ADR precedent: [Source: docs/adr/ADR-050-enterprise-pdp-integration.md] · [Source: docs/adr/ADR-024-out-of-kernel-sandbox-escape-structural-detector.md] · [Source: _bmad-output/implementation-artifacts/11-4a-enterprise-pdp-integration.md] · [Source: _bmad-output/implementation-artifacts/11-4b-adr-024-sandbox-escape-structural-detector.md]

---

## Dev Agent Record

### Agent Model Used

<!--
§A6 NON-OPUS SAFETY NET (Epic 8 retro 2026-06-12, ratified by Lunarpulse):
Implementation by a non-Opus model is permitted ONLY with the full §A6 multi-layer review attached at the
review gate — Blind Hunter · Edge-Case Hunter · Acceptance Auditor · Test-Infra Auditor + a runtime-execution
check that runs the gate LIVE and watches each *-fault-inject falsifier red its leg. A degraded / rate-limited
§A6 review is NOT a review and hard-blocks completion. Story tier (F1 RESOLVED): opus-4-8 whole-story
(security-adjacent + three canned-green traps). Record the actual dev model below.
-->

openai-codex/gpt-5.5

### Debug Log References
- `cargo test -p maos-sso` → passed 11 tests (Task 1 OIDC verifier).
- `cargo run -p xtask -- check-dependency-closure` → passed for `maos-kernel-core` and `maos-domain` after adding `maos-sso` leaf dependencies.
- `cargo test -p maos-sso --test principal_governs --test identity_provenance --test identity_source_blind` → passed 4 tests (Task 2 governed authorization/provenance).
- `cargo test -p maos-audit --test identity_asserted_kind_test` → passed 1 test (kind 30 renders `identity.asserted`).
- `cargo run -p xtask -- check-kernel-baseline` → passed (`maos-kernel-core/src = 23081`, pinned 23081).
- `cargo test -p maos-secrets --test at_rest_seal` → passed 3 tests (ciphertext, right-key open, wrong-key fail).
- `cargo test -p maos-secrets --test default_plaintext_preserved` → passed 2 tests (no-KMS Option-A plaintext; configured KMS ciphertext).
- `cargo build --release -p maos-secrets --features kms-fault-inject` → expected failure from release-only `compile_error!` ship blocker.
- `cargo test -p maos-secrets` → passed 5 tests.
- `cargo run -p xtask -- check-dependency-closure` → passed for `maos-kernel-core` and `maos-domain` after KMS dependencies.
- `cargo test -p maos-siem --test file_sink` → passed 2 tests (localhost file sink appends RFC5424/CEF lines and surfaces I/O errors).
- `cargo test -p maos-siem --test redaction_before_forward` → passed 2 tests (query-with-redaction before projection; secret does not appear in frames).
- `cargo test -p maos-siem --test forward_count_derive` → passed 3 tests (real-row count `Some(2)`, empty TL `None`/N-A, non-empty zero-match `Some(0)`).
- `cargo test -p maos-siem` → passed 15 tests.
- `cargo build --release -p maos-siem --features siem-fault-inject` → expected failure from release-only `compile_error!` ship blocker.
- `cargo run -p xtask -- check-dependency-closure` → passed for `maos-kernel-core` and `maos-domain` after SIEM dependencies.
- `cargo test -p maos-bin --test enterprise_identity_wiring --no-default-features --features network` → passed 4 tests (zero-config byte-identical; SSO/KMS/SIEM configured-but-down fail-closed outcomes).
- `cargo test -p maos-bin --features network available_arm_tests` → passed 5 tests (KMS Available ciphertext, SSO forged/expired denial, `identity.asserted` persist, SIEM forward, sink-down buffer).
- `cargo test -p maos-bin --features network enterprise_pdp_runtime::tests::evaluate_issuance_forwards_principal_attributes` → passed 1 test (verified principal attributes reach PDP request).
- `cargo test -p maos-bin --features network --test smoke_cli_wrapper_8_12 maos_run_cli_wrapper_worker_spawns_real_subprocess` → passed 1 test after deterministic `target/debug` fixture PATH injection.
- `cargo test -p maos-bin --features network` → passed 60 tests across 17 suites.
- `cargo check -p xtask` → passed (pre-existing workspace warnings only).
- `cargo run -p xtask -- check-ship-gate-completeness` → passed; all 28 expected gates present.
- `cargo run -p xtask -- check-coverage-matrix-completeness` → passed (57 v1.0 entries, 47 advisory-until-engagement).
- `cargo run -p xtask -- check-enterprise-identity --json` → passed; 10/10 legs green (`oidc-verify=11`, `principal-provenance=5`, `at-rest-seal=5`, `siem-redaction-export=7`, `additive-and-failclosed=4`, `fault-inject-falsifiers=3`, `available-arm-integration=5`, `issuance-bypass-absence=1`, `release-graph-absence=4`, `kernel-abi-diff=1`).
- `cargo test -p maos-audit -p maos-loom-lite` → passed 237 tests (33 ignored).
- `cargo test -p maos-domain -p maos-pdp -p maos-kernel-core` → passed 836 tests (1 ignored).
- `cargo run -p xtask -- check-kernel-baseline` → passed (`maos-kernel-core/src = 23081`, pinned 23081).
- `cargo run -p xtask -- check-dependency-closure` → passed for `maos-kernel-core` and `maos-domain`.
- `cargo build --workspace` → passed (pre-existing workspace warnings only).

### Completion Notes List

- **Honesty note (NFR, L9):** do NOT mark "encrypted-at-rest" universally satisfied — it is **opt-in**, scoped to loom-lite Collective rows plus the out-of-kernel identity assertion write path; **generic kernel-authored audit TL payload encryption remains deferred** because the live write path is `maos-kernel-core::TransparencyLogAdapter::insert_frame_event`. Option-A plaintext is the **default**, and **kernel-core Private/Shared at-rest is deferred** (F2/L2). Do NOT mark "org-KMS / SAML" fully satisfied: **one reference OIDC protocol** (SAML deferred) + **one reference local-master-key KMS marked dev/CI-only** (keyring/Vault/cloud additive-per-port, deferred — F3/F4). NFR-Aud-11 SIEM half only (OTel first half = 9.5b).
- Task 1 implemented `IdentityAssertionPort`, `AuthenticatedPrincipal`, `IdentityError`, and the out-of-kernel `maos-sso::OidcVerifier` with static-JWKS RS256 verification, explicit allowlist rejection for `alg:none`/HS256-confusion, claim fail-closed mapping, and debug-only `sso-fault-inject`.
- Task 2 implemented `OidcVerifier::govern_authorization`, `GovernedAuthorization`, attested `IdentityProvenanceRecord`, and `reconcile_provenance`; provenance is out-of-kernel, authorization-layer only, and synthetic/blind records do not inflate counts. `maos-audit` now renders discriminator 30 as `identity.asserted`.
- Task 3 implemented `KeyManagementPort`, `KmsError`, dev/CI-only `LocalMasterKeyKms`, envelope `seal_at_rest`/`open_at_rest`, default-preserving `seal_at_rest_opt`, and the `kms-fault-inject` release guard. Wrong-key open fails; no-KMS default returns byte-identical plaintext.
- Task 4 implemented `SiemProjectionPort`, crate-private `maos-siem::project`, `SiemExporter`, `export_from_tl` (`query_with_redaction` before projection), `ExportReport`/`export_report_from_tl`, localhost `forward_to_file`, NDJSON/CEF/RFC5424 framing, SIEM fault-inject redaction bypass, sanitized CEF/syslog payloads, derived severity, timestamp/hostname population, and non-vacuous empty-vs-zero-match count disposition.
- Task 5 added the `maos_bin::enterprise_identity` library surface, `EnterpriseConfig`, `EnterpriseRuntime`, explicit optional `Arc<dyn IdentityAssertionPort/KeyManagementPort/SiemProjectionPort>` slots, distinct `EnterpriseFailure` variants for SSO issuance denial, KMS sealed-write refusal, and SIEM sink-down buffering, plus `main.rs` composition-root wiring for enterprise-governed capability issuance, loom-lite at-rest seal injection, and periodic SIEM forwarding.
- Task 6 added `check-enterprise-identity`, enrolled it in xtask, CI, gate registry, ship-gate completeness, and coverage matrix; live gate green proves ten independent legs, all three `*-fault-inject` inversion tests, all three release fault blockers, workspace feature-graph fault-feature absence, and direct `issue_with_mediation` bypass absence outside the enterprise wrapper.
- Task 7 added ADR-051, ADR index entry, NFR-Sec-18/19, NFR-Aud-11 SIEM-half annotation, requirements inventory entries, and coverage-matrix traces with explicit honesty clauses.
- Task 8 live verification passed: enterprise gate, dependency closure, kernel baseline, ship-gate completeness, coverage-matrix completeness, workspace build, full `maos-bin --features network`, targeted enterprise tests, and sibling package tests.

### Review Findings

### File List
- `Cargo.toml`
- `crates/maos-domain/src/ports/identity_assertion.rs`
- `crates/maos-domain/src/ports/mod.rs`
- `crates/maos-sso/Cargo.toml`
- `crates/maos-sso/src/lib.rs`
- `crates/maos-sso/tests/alg_negatives.rs`
- `crates/maos-sso/tests/claims_failclosed.rs`
- `crates/maos-sso/tests/fault_inject.rs`
- `crates/maos-sso/tests/fixtures.rs`
- `crates/maos-sso/tests/oidc_verify.rs`
- `crates/maos-audit/src/lib.rs`
- `crates/maos-audit/tests/identity_asserted_kind_test.rs`
- `crates/maos-audit/tests/identity_asserted_write.rs`
- `crates/maos-sso/tests/identity_provenance.rs`
- `crates/maos-sso/tests/identity_source_blind.rs`
- `crates/maos-sso/tests/principal_governs.rs`
- `crates/maos-domain/src/ports/key_management.rs`
- `crates/maos-secrets/Cargo.toml`
- `crates/maos-secrets/src/lib.rs`
- `crates/maos-secrets/tests/at_rest_seal.rs`
- `crates/maos-secrets/tests/default_plaintext_preserved.rs`
- `crates/maos-secrets/tests/kms_fault_inject.rs`
- `crates/maos-loom-lite/src/adapter.rs`
- `crates/maos-loom-lite/src/seal.rs`
- `crates/maos-loom-lite/src/store.rs`
- `crates/maos-domain/src/ports/siem_projection.rs`
- `crates/maos-siem/Cargo.toml`
- `crates/maos-siem/src/lib.rs`
- `crates/maos-siem/tests/file_sink.rs`
- `crates/maos-siem/tests/fault_inject.rs`
- `crates/maos-siem/tests/redaction_before_forward.rs`
- `crates/maos-siem/tests/forward_count_derive.rs`
- `crates/maos-bin/Cargo.toml`
- `crates/maos-bin/src/lib.rs`
- `crates/maos-bin/src/enterprise_identity.rs`
- `crates/maos-bin/src/enterprise_pdp_runtime.rs`
- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/tests/enterprise_identity_wiring.rs`
- `crates/maos-bin/tests/smoke_cli_wrapper_8_12.rs`
- `xtask/src/check_enterprise_identity.rs`
- `xtask/src/main.rs`
- `xtask/src/check_ship_gate_completeness.rs`
- `xtask/gate-registry.toml`
- `.github/workflows/discipline.yml`
- `tests/coverage-matrix.yaml`
- `docs/adr/ADR-051-enterprise-identity-at-rest-siem.md`
- `docs/adr/index.md`
- `_bmad-output/planning-artifacts/prd/non-functional-requirements.md`
- `_bmad-output/planning-artifacts/epics/requirements-inventory.md`
- `_bmad-output/implementation-artifacts/11-4c-enterprise-identity-at-rest-siem.md`

### Change Log

| Date | Change |
|------|--------|
| 2026-07-06 | Story created (ready-for-dev) — 5 ACs, 8 forks AS RAISED (F1–F8) + 9 landmines + decisions ledger + §A7 identity-provenance reflex. ZERO kernel-Δ @23081, opus-4-8 LEAN, next ADR-051. |
| 2026-07-06 | **PREFLIGHT RESOLVED** (party-mode — Winston · Murat · John · Amelia · Mary + Vex/security & Grumbal/adversary + Paige). All 8 forks ratified + Grumbal fail-closed catch (D10). **AC count 5→6** (F1: AC1 split into OIDC-verify + principal-governs-authorization). **F8 refined**: provenance via NEW out-of-kernel `identity.asserted` audit kind (not a kernel issuance-record mutation). **F2 scoped after implementation reality-check**: at-rest AEAD = loom-lite Collective rows + out-of-kernel identity assertion writes; generic kernel-authored TL and kernel Private/Shared deferred. **F4 hardened**: alg-allowlist + `alg:none`/confusion gate-negatives. **F5 tightened**: NDJSON+CEF, RFC5424 transport, TLS sink. **F3 labeled**: reference KMS dev/CI-only. NFR-Sec-18/19 + NFR-Aud-11 annotation. opus-4-8 whole-story confirmed. Tier + surfaces confirmed; Task 0 DONE. Ready for dev-story. |
| 2026-07-06 | Dev implementation complete — AC1–AC6 delivered; all Tasks 0–8 checked; `check-enterprise-identity` live gate GREEN (7 legs) with release fault blockers firing and kernel baseline unchanged at 23081. |
| 2026-07-07 | Review patch set applied and status promoted to done — `EnterpriseRuntime` now builds real env adapters; enterprise-governed capability issuance wraps every direct `maos-bin` `issue_with_mediation` call; SSO verifies `MAOS_SSO_ASSERTION`, forwards verified principal attributes into Enterprise PDP evaluation, and persists `identity.asserted`; loom-lite receives the KMS at-rest seal hook; SIEM forwards to a localhost file sink with buffering/error surfacing; all three fault-inject inversions run in the gate; `check-enterprise-identity` is GREEN with 10 legs including `issuance-bypass-absence`. Generic audit TL payload encryption remains deferred because the live TL write path is `maos-kernel-core::TransparencyLogAdapter::insert_frame_event`; preserving L1 zero kernel-core delta takes precedence. |

---

### Review Findings

Code review 2026-07-06 — 4 parallel adversarial layers (Blind Hunter · Edge-Case Hunter · Acceptance Auditor · Test-Infra Auditor), whole staged diff (`git diff HEAD`, 62 files / +4466). All four layers converged on the same root cause; 0 layers failed; 0 dismissed. **Consensus:** the three leaf-crate mechanisms (OIDC verify, at-rest AEAD seal, SIEM redaction-projection) are real and genuinely falsified **in isolation** — but they are **never composed into the daemon**, and the gate's green does not prove integration. AC1 + AC6 satisfied; AC2/AC3/AC4 only partially satisfied (integration absent); AC5 satisfied at AC-text level.

#### Decision resolved

- [x] [Review][Decision] **Composition root is a non-functional posture stub; the three real adapters are never wired into the daemon (root cause of the AC2/AC3/AC4 gaps)** — RESOLVED 2026-07-06 to **(A) Complete the integration now** (Lunarpulse). The full rationale/evidence is retained below as the P1 integration patch set. Converted from decision-needed → patch.

#### Patches

**P1 — daemon integration (decision A: complete the composition root so the real adapters actually run):**

- [x] [Review][Patch] **Populate `EnterpriseRuntime` port slots from env + route the `Available` arms through the real adapters** — `from_env` constructs `OidcVerifier`/`LocalMasterKeyKms`/`SiemExporter` from `MAOS_SSO_*`/`MAOS_KMS_*`/`MAOS_SIEM_*`; `issue_under_principal(Available)` calls `IdentityAssertionPort::verify`; `seal_row_at_rest(Available)` calls `seal_at_rest_opt`; `forward_audit_to_siem(Available)` tails → projects → writes through the SIEM sink. [P1, blind+edge+auditor+testinfra]
- [x] [Review][Patch] **Add `main.rs` composition-root wiring** — daemon startup constructs `EnterpriseRuntime` from env; enterprise-governed issuance wraps every direct `maos-bin` `issue_with_mediation` call; SSO verifies `MAOS_SSO_ASSERTION`; verified principal attributes feed Enterprise PDP evaluation before token minting; loom-lite receives the at-rest seal hook; SIEM forwarding runs as a composition-root consumer. Folded into `enterprise_identity.rs`/`enterprise_pdp_runtime.rs`/`main.rs` rather than separate modules. [P1, auditor]
- [x] [Review][Patch] **Persist `identity.asserted` (kind 30) to the Transparency Log** — `maos-audit::append_identity_asserted` writes kind=30 rows; `EnterpriseRuntime::issue_under_principal` persists principal + `spirit_pid` + capability key + decision time after a verified assertion. [P1, auditor]
- [x] [Review][Patch] **Apply `seal_at_rest_opt` at the loom-lite + audit adapter write layers** — loom-lite Collective writes are sealed through an injected closure when KMS is configured; `identity.asserted` is persisted through the out-of-kernel audit helper. Generic kernel-authored TL payload encryption is intentionally NOT implemented because the live TL write path is `maos-kernel-core::TransparencyLogAdapter::insert_frame_event`; sealing it would violate L1 zero kernel-core delta. [P1, auditor+blind]
- [x] [Review][Patch] **Implement `SiemProjectionPort` + a localhost file sink; wire forward + buffer** — `SiemExporter` implements the port; `forward_to_file` appends RFC5424-framed CEF lines to a local file; `forward_audit_to_siem` returns buffered counts on sink failure and never silent-drops. [P1, auditor+blind]
- [x] [Review][Patch] **Add gate legs that assert the `Available` arm end-to-end** — `available-arm-integration` proves configured KMS yields ciphertext, configured SSO denies forged/expired assertions and persists valid `identity.asserted`, and configured SIEM forwards/buffers through the real sink. [P1, blind+edge]

**P1 — falsifier layer (the §A6 non-negotiable the opus-4-8 escalation exists to deliver):**

- [x] [Review][Patch] **`siem-fault-inject` is a phantom feature — no redaction-bypass behavior exists; the AC4 "And" falsifier is fiction** — `siem-fault-inject` now routes through plain `query(...)` instead of `query_with_redaction(...)`, dropping redaction metadata; the ignored feature test is run by the gate. [P1, edge+testinfra+auditor]
- [x] [Review][Patch] **`sso-fault-inject` inversion test is never executed by the gate; its docstring falsely claims it is** — `fault-inject-falsifiers` runs `maos-sso --features sso-fault-inject -- --ignored`; inversion is now observed per gate. [P1, edge+testinfra]

**P2 — correctness / contract:**

- [x] [Review][Patch] **`kms-fault-inject` has a real passthrough branch but no test exercises it; inversion unverified** — added the ignored inversion test and gate leg under `kms-fault-inject`. [P2, edge+testinfra]
- [x] [Review][Patch] **`open_at_rest` ignores its `CryptoProvider` (`_crypto`) and hardcodes ring AES-256-GCM; asymmetric with `seal_at_rest` which routes through the trait** — removed the misleading `CryptoProvider` parameter and documented the L1-pinned ring-open boundary. [P2, blind+edge+auditor]
- [x] [Review][Patch] **`maos_siem::project` is `pub` and accepts un-redacted `AuditEntry` slices — redaction bypass reachable via the public API** — `project` is now `pub(crate)`; external callers must use `export_from_tl` or the port impl. [P2, blind]
- [x] [Review][Patch] **release-guard checks per-crate release builds with a substring matcher, not the workspace release feature graph AC6 specifies** — gate still exercises per-crate compile_error blockers and now also checks `cargo tree -e features -p maos-bin --features network` for fault-feature absence. [P2, blind]
- [x] [Review][Patch] **JWT `exp`/`nbf` rely on jsonwebtoken's unpinned default 60s leeway; `validate_exp` not set explicitly** — validation now pins `leeway = 0` and explicitly sets `validate_exp = true`. [P2, edge; NIT auditor]
- [x] [Review][Patch] **CEF/RFC5424 projection does not sanitize spaces or NUL/control chars in payload/intent** — CEF/syslog escaping now handles spaces and control bytes, including NUL. [P2, edge]
- [x] [Review][Patch] **`export_report_from_tl` conflates genuinely-empty TL with a non-empty TL whose filter matches zero rows (both → `None`)** — report now distinguishes empty TL (`None`) from non-empty zero-match (`Some(0)`). [P2, edge]

**NIT:**

- [x] [Review][Patch] **`now_ns()` silently falls back to timestamp 0 when the system clock is before UNIX_EPOCH** — SSO and composition-root timestamps now fail loud instead of stamping zero. [NIT, blind]
- [x] [Review][Patch] **RFC5424 frames carry nil timestamp/hostname and CEF severity is hardcoded to 5** — frames now populate timestamp/hostname and derive severity by audit kind. [NIT, blind]
- [x] [Review][Patch] **Stale "currently RED (compile-red)" doc in `enterprise_identity_wiring.rs`** — comment updated to describe the live wiring contract. [NIT, testinfra]
- [x] [Review][Patch] **OIDC verify/alg/claims tests are `#![cfg(target_os = "linux")]` — vacuous on non-Linux hosts** — pure-JWT tests no longer carry the Linux-only cfg. [NIT, auditor]
- [x] [Review][Patch] **Good-path OIDC assertions use an inline literal email (`"reza@maos.example"`) instead of a fixture constant** — tests use the fixture source-of-truth. [NIT, testinfra]
- [x] [Review][Patch] **Gate ships 7 legs vs story D9/AC6's 6; vacuous-green exemption includes `release-graph-absence` (spec names only `kernel-abi-diff`)** — story now discloses the 10-leg post-review gate, including `issuance-bypass-absence`, and the `release-graph-absence` exemption explicitly. [NIT, auditor]

#### Per-AC verdict (Acceptance Auditor)
- AC1 OIDC verify fail-closed + alg allowlist: **SATISFIED** (real JWKS sig verify, alg:none + HS256-confusion rejected at parse, exp/nbf/aud/iss enforced, leeway pinned).
- AC2 principal→authorization + `identity.asserted` provenance: **SATISFIED** — every direct `maos-bin` `issue_with_mediation` call now goes through `issue_enterprise_governed_capability`; when SSO is configured, `MAOS_SSO_ASSERTION` is verified fail-closed, verified principal attributes are forwarded into Enterprise PDP `PolicyDecisionRequest.principal_attributes`, PDP deny blocks token minting, and successful governed issuance persists kind=30 `identity.asserted` out-of-kernel.
- AC3 opt-in at-rest AEAD: **SATISFIED for loom-lite Collective rows and the runtime seal path** — KMS Available produces ciphertext and fail-closes on seal error; no-KMS default stays byte-identical plaintext. **Audit TL generic payload encryption remains deferred** because the live TL write path is kernel-core; implementing it would violate L1 zero kernel-core delta.
- AC4 SIEM redaction-before-forward: **SATISFIED** — redaction-before-projection, NDJSON/CEF/RFC5424 projection, localhost file sink, sink-down buffering/error surfacing, and `siem-fault-inject` inversion are all covered by the 10-leg gate.
- AC5 additive + fail-closed per subsystem: **SATISFIED** — zero-config byte-identical posture, SSO-down deny, KMS-down refuse, SIEM-down buffer/error, Available-arm paths, governed issuance, PDP deny, and no-bypass coverage are exercised.
- AC6 gate + ADR + NFRs: **SATISFIED** — enrolled 5 ways, disposition correct, ADR-051 authored, NFR honesty clauses in-text, and post-review `check-enterprise-identity` is green with 10 legs.
- Landmines L1/L2/L3/L5/L6/L7/L9: **CLEAN** (no kernel-core touch, no default-flip, no principal-in-token, offline tripwires, redaction-before-forward honored, closures in leaf crates, NFRs honest).
