# Epic 9: Audit & Compliance Surfaces + Operator Productionization (v0.5 → v1.0)

**Goal:** DPO, CISO, external regulator, and operator can query, export, forget, attribute cost, and prove substrate state. Substrate as a hermes-tenant — uninstall is a real, externally-verifiable guarantee. Single epic with two internal concerns (Winston's split, John yielded).

**Internal Concern A — Audit & Compliance Rail (legal-facing):**

- `maosctl audit subject-access --principal <id>` (FR42): returns all principal-namespace entries across all Spirits with provenance (Spirit, time, derived-from observations).
- `maosctl audit posture-delta --range=<timespan>` (FR43): capability-scope changes + sandbox-tier changes + consent-policy changes with approval-chain attribution.
- `maosctl audit sealed-export <bundle-spec>` (FR44): Ed25519-signed by operator audit key; third-party-verifiable; conforms to `maos.audit-bundle.v1` schema; includes both working-memory digest refs (I12) AND distilled-output content (I11).
- `maosctl forget --principal <id> [--reason <legal-hold>]` (FR45): GDPR Article 17 right-to-be-forgotten with cross-Spirit cascade (forgetting cascades to working-memory references in other Spirits; distillates containing principal data marked redacted with re-distillation triggered); 50/50 clean removal + 50/50 redaction-marker in immutable log + 0 leakage in 100 follow-up subject-access queries.
- `journal.export(filter, redaction_policy)` (FR46, ADR-023): `maos.trajectory.v1` schema with Ed25519 signing and applied-redaction flag.
- Frame-by-frame log query (FR41): authenticated audit interface with filters by Spirit / capability / time-range / frame-kind / tag; P99 ≤2s single-Spirit on 30-day window / ≤10s global; ≥98/100 events recoverable on per-commit log-completeness corpus.
- Deterministic replay (ADR-028): over **shape of the trace** (IAC frame ordering, capability-token issuances, halt events, decision-frame emission), NOT redacted payload content; redacted slots replay as `<REDACTED:type=<class>, len=<bytes>, hash=<sha256-prefix>>` placeholders; `schemas/trace-shape.schema.json` JSON Schema draft-2020-12 validated in CI.
- Governance audit-queryable artifacts (FR62): vetter-key admission and rotation events / ABI-extension proposals and ratification status / ComplianceClaim schema versions and effective dates.
- Proof-of-erasure record on Spirit uninstall (FR65): enumerates all removed substrate state (memory namespace per ADR-026, capability tokens, pending halts, intent lineage references, scheduled invocations); externally-verifiable Merkle inclusion + exclusion proof.

**Internal Concern B — Operator Productionization (ops-facing):**

- Typed error catalog at `https://docs.maos.dev/errors/<ERR_NAME>` (FR63): CI-enforced metadata per error variant (code / severity / recovery-class / owner / kernel-or-spirit / since-version); `cargo run --bin error-metadata-check` exits non-zero if any variant missing any field; catalog covers 14+ named typed errors.
- Cost attribution per Spirit per task per principal in Transparency Log (FR64) — enterprise-readiness gate; ≥98% reconciliation against provider billing sampled monthly (NFR-Cost-1).
- Pre-built binaries (Linux amd64/arm64, macOS arm64) via GitHub Releases with SHA256 + Ed25519 verification mandatory (v0.5); Homebrew tap, AUR, deb, rpm (v1.0); container images Docker Hub / GHCR (v1.0).
- Transparency Log backup/DR (NFR-Ops-9): RPO ≤1h, RTO ≤4h, backup integrity verified weekly via Merkle-root cross-check.
- Air-gapped deployment validation (NFR-Ops-12): substrate boots/runs/produces transparency-log entries with zero outbound network calls; structural test in CI via network-namespace isolation.
- Multi-operator tenancy primitive-reservation (NFR-Ops-11 v1.0; full impl v1.5+ in E10): per-operator namespace + transparency-log shard + capability-token signing key + GDPR-erasure scope declared as primitive-reserved so v0.5 grammar lock doesn't paint into a corner.
- Region-pinning primitive (PIPL §40 / data localization — NFR-Comp-4): Transparency Log + working-memory store configurable to single jurisdictional region with cryptographic enforcement against cross-region replication.
- Spirit model-provenance manifest field (SB-1047 / Colorado AI Act adjacent — NFR-Comp-5): covered-model identifier, training-data lineage, last-eval timestamp; substrate validates field presence at admission.
- **5 canonical doc deliverables** (NFR-Doc-4) with CI-verifiable minima: manifest schema reference (≥1 example per field) at `docs.maos.dev/manifest/<version>/`; pattern cookbook (≥10 patterns) at `docs.maos.dev/cookbook/`; migration runbooks at `docs.maos.dev/migrate/`; troubleshooting guide (covers 100% of FR63 catalog) at `docs.maos.dev/troubleshoot/`; deployment topology guide at `docs.maos.dev/deploy/`.
- API reference site (NFR-Doc-3) at `docs.maos.dev/abi/<version>/`: versioned, searchable, deep-linkable, archived ≥2 minor versions.
- WCAG AA compliance for doc site (NFR-Doc-5).
- **Korean i18n v1.0** (NFR-Doc-6 — Japanese + Chinese-simplified at v1.5 in E10); LOCALES.md with glossary lock — terms NEVER translated: Spirit, Worker, kernel, ADR identifiers, error codes.
- Doc tooling (NFR-Doc-7): per-locale builds + fallback to English + language switcher with deep-link preservation + version dropdown; mdBook+i18n / Docusaurus / VitePress decision by v0.5; v1.0 in production.
- Onboarding artifacts (NFR-Ops-6): `RFC_TEMPLATE.md` v0.8, `GOVERNANCE.md` v0.5 basic + v0.8 locked, `CODE_OF_CONDUCT.md` v0.5, `LOCALES.md` v1.0, `TRADEMARK.md` v1.0, `BREAKING.md` v1.0.
- Sustainability vehicle (NFR-Ops-7): Open Collective declared-intent v0.5; legal/fiscal-sponsor work v0.8.
- Substrate-self compliance scope declaration (NFR-Comp-3): `STABILITY.md` scope-disclaimer that SOC 2 / ISO 27001 / FedRAMP scope is operator's responsibility.
- Trust-anchor framing carry-forward decision (NFR-Ops-8): published ADR by v0.3 declaring committed competitive framing (substrate-as-substrate vs substrate-as-trust-anchor).
- SIEM export (NFR-Aud-11) at v2.0; OpenTelemetry adapter at v1.0 SLO-class.

**FRs covered:** FR41, FR42, FR43, FR44, FR45, FR46, FR62, FR63, FR64, FR65, FR48 partial (FIPS readiness gate).

**Key NFRs:** NFR-Perf-5 (audit query latency), NFR-Aud-1 through NFR-Aud-6 (capability introspection / drift detection / deterministic replay / audit retention 90d / right-to-explanation / sealed-export), NFR-Aud-10 (GDPR 50-scenario corpus), NFR-Aud-11 (SIEM/OTel), NFR-Aud-12 (storage cascade erasure + externally-verifiable uninstall receipt), NFR-Aud-13 (erasure SLA 95% within 30 days, configurable 7 days enterprise), NFR-Aud-14, NFR-Comp-3, NFR-Comp-4, NFR-Comp-5, NFR-Cost-1, NFR-Doc-1 through NFR-Doc-7, NFR-Ops-1 through NFR-Ops-12, NFR-Tenancy-1.

**Corpora authored in E9:**
- GDPR Art. 17 cross-Spirit cascade 50-scenario corpus.
- Per-commit log-completeness corpus N=100 injected events.
- Trace-shape schema validation corpus.

**Acceptance demo:** DPO runs `maosctl audit subject-access --principal alice@example.org` — returns all entries across all Spirits in <2s; sealed-export bundle verifies on third-party machine; GDPR forget cascades + 0 leakage in 100 follow-up queries; cost reconciliation ≥98% against provider billing; air-gapped CI run passes; Korean-localized docs render with deep-link preservation.

### Stories

## Story 9.1: Ship `maosctl audit` Subcommands — Query, Subject-Access, Posture-Delta, Sealed-Export

As a DPO / CISO / external regulator,
I want `maosctl audit query` for frame-by-frame queries with filters (FR41), `maosctl audit subject-access --principal <id>` returning all principal-namespace entries with provenance (FR42), `maosctl audit posture-delta --range=<timespan>` for capability/sandbox/consent-policy changes with approval-chain attribution (FR43), AND `maosctl audit sealed-export <bundle-spec>` producing Ed25519-signed third-party-verifiable bundles (FR44),
So that legal-facing queries are first-class operations with audit-grade latency floors and signed export bundles.

**Acceptance Criteria:**

**Given** `maosctl audit query --spirit <id> --range <timespan> --frame-kind <kind> --tag <tag>`
**When** the query runs on a 30-day window scoped to a single Spirit
**Then** P99 latency is ≤2s (NFR-Perf-5)
**And** for global queries (no Spirit filter): P99 ≤10s
**And** the log-completeness corpus (N=100 injected events) shows ≥98/100 events recoverable (NFR-Aud-1)

**Given** `maosctl audit subject-access --principal alice@example.org` (FR42)
**When** the query runs across all Spirits
**Then** the result enumerates every entry under `principal:alice@example.org:*` across all Spirits
**And** each entry carries provenance: Spirit id, time, derived-from observations
**And** completion within the latency floor

**Given** `maosctl audit posture-delta --range=<timespan>` (FR43)
**When** the query runs
**Then** the result surfaces capability-scope changes, sandbox-tier changes, consent-policy changes
**And** each change has approval-chain attribution from the Approval Decision Log

**Given** `maosctl audit sealed-export <bundle-spec>` (FR44)
**When** the operator generates a sealed-export
**Then** the bundle is Ed25519-signed by the operator's audit key
**And** the bundle is third-party-verifiable
**And** the bundle conforms to `maos.audit-bundle.v1` schema
**And** the bundle includes both working-memory digest refs (I12) AND distilled-output content (I11)
**And** corpus tier validation: signed-export tier at v1.0 (NFR-Aud-6)

## Story 9.2: Execute GDPR Article 17 Cascade with Deterministic Replay and Proof-of-Erasure

As a regulator enforcing GDPR Article 17,
I want `maosctl forget --principal <id>` (FR45) performing cross-Spirit cascade with 50/50 + 0/100-leakage floors, `journal.export(filter, redaction_policy)` (FR46), deterministic replay (ADR-028) over trace-shape (not redacted payload), AND proof-of-erasure record on Spirit uninstall (FR65) with externally-verifiable Merkle inclusion/exclusion proof,
So that the substrate-uninstall guarantee is a real proof, not a hope, and replay determinism is anchored at v1.0 best-effort / v1.5 hard target.

**Acceptance Criteria:**

**Given** `crates/maos-cli/src/cmd/forget.rs` implementing `maosctl forget --principal <id> [--reason <legal-hold>]` (FR45)
**When** the command dispatches `crates/maos-audit/src/gdpr/cascade.rs::run_forget(principal_id, reason)`
**Then** all `principal:<principal_id>:*` entries are removed across all Spirit private tiers (Story 4.3's Memory Manager)
**And** the deletion event itself is journaled to `crates/maos-audit/src/journal.rs::write_gdpr_event` (preserving lifecycle invariant — the act of forgetting is recorded)
**And** principal data is gone from queryable surfaces
**And** cross-Spirit cascade: distillates containing principal data are marked redacted in `crates/maos-audit/src/i11_chain.rs::mark_redacted` with re-distillation triggered downstream

**Given** the 50-scenario GDPR Art. 17 cross-Spirit cascade corpus (NFR-Aud-10) at `crates/maos-audit/tests/fixtures/gdpr-cascade-v0/`
**When** `cargo test -p maos-audit -- test_gdpr_art17_cascade` runs
**Then** 50/50 scenarios show clean removal at queryable surface (verified via Story 9.1's subject-access query)
**And** 50/50 scenarios show redaction-marker present in immutable Transparency Log
**And** 0 leakage in 100 follow-up subject-access queries from a separate fixture
**And** time-to-erasure: 95% within 30 days (configurable to 7 days for enterprise tier in `config.toml`) per NFR-Aud-13
**And** audit log entry within 24h of request acceptance (timed in `crates/maos-audit/tests/erasure_sla_test.rs`)

**Given** `journal.export(filter, redaction_policy)` per ADR-023 (FR46) implemented in `crates/maos-cli/src/cmd/audit_export.rs`
**When** the operator exports a filtered trajectory
**Then** the bundle conforms to `maos.trajectory.v1` schema defined in `schemas/trajectory.schema.json`
**And** the bundle is Ed25519-signed via Story 1a.3's `CryptoProvider` with applied-redaction flag
**And** redaction policy is honored end-to-end, verified by `crates/maos-audit/tests/trajectory_redaction_test.rs`

**Given** deterministic replay (ADR-028, NFR-Aud-3) over trace-shape implemented in `crates/maos-audit/src/replay/`
**When** `crates/maos-audit/src/replay/runner.rs::replay(bundle)` executes against a sealed-export bundle
**Then** replay determinism is verified over IAC frame ordering, capability-token issuances, halt events, and decision-frame emission
**And** redacted slots replay as `<REDACTED:type=<class>, len=<bytes>, hash=<sha256-prefix>>` placeholders generated by `crates/maos-audit/src/replay/redaction_placeholder.rs`
**And** `schemas/trace-shape.schema.json` (JSON Schema draft-2020-12) validates the replay in CI via `crates/maos-audit/tests/replay_schema_test.rs`
**And** v1.0 best-effort target; v1.5 hard target

**Given** Spirit uninstall (FR65) via `crates/maos-cli/src/cmd/uninstall_spirit.rs`
**When** the operator runs `maosctl uninstall <spirit>`
**Then** `crates/maos-audit/src/erasure/proof.rs::emit_proof_of_erasure(spirit_id)` enumerates all removed substrate state (memory namespace per ADR-026, capability tokens, pending halts, intent lineage references, scheduled invocations)
**And** the record carries signed Merkle inclusion + signed Merkle exclusion proof generated by `crates/maos-audit/src/erasure/merkle.rs` (NFR-Aud-12)
**And** the proof is retained independent of the substrate at `~/.local/share/maos/erasure-proofs/<spirit_id>-<timestamp>.bundle` (third-party-verifiable via the published `tools/verify-erasure/` toolchain shipped at v1.0)
**And** 100% of registered storage backends prove erasure within bounded window — tested in `crates/maos-audit/tests/multi_backend_erasure_test.rs`
**And** the proof is retained independent of the substrate (third-party verifiable)
**And** 100% of registered storage backends prove erasure within bounded window

## Story 9.3: Publish the Typed Error Catalog + Governance Audit Artifacts and Wire Cost Attribution

As an enterprise operator,
I want the typed error catalog at `https://docs.maos.dev/errors/<ERR_NAME>` with CI-enforced metadata for every error variant (FR63), governance audit-queryable artifacts surfacing vetter-key admission / ABI-extension proposals / ComplianceClaim schema versions (FR62), AND cost attribution per Spirit per task per principal with ≥98% reconciliation against provider billing (FR64 + NFR-Cost-1),
So that v1.0 is production-grade for enterprise: errors are diagnosable, governance is auditable, costs are attributable.

**Acceptance Criteria:**

**Given** the typed error catalog (FR63)
**When** any kernel-emitted error is raised
**Then** the error carries a stable typed code from the published catalog
**And** the catalog covers all 14+ named typed errors documented in architecture-maos-minimal-opus.md
**And** each variant has 6 CI-enforced metadata fields: code / severity / recovery-class / owner / kernel-or-spirit / since-version (NFR-Doc-2)
**And** `cargo run --bin error-metadata-check` exits non-zero if any variant is missing any field

**Given** the documentation site at `docs.maos.dev/errors/<ERR_NAME>`
**When** any error code is documented
**Then** the URL renders 200 with retryability + cause-chain semantics + version-stability guarantees
**And** consistency with the LTS policy is enforced (no breaking error-code changes within an LTS cycle)

**Given** governance audit-queryable artifacts (FR62)
**When** the kernel processes governance events
**Then** vetter-key admission and rotation events are journaled and queryable
**And** ABI-extension proposals and their ratification status are journaled
**And** ComplianceClaim schema versions and their effective dates are journaled
**And** all three artifact streams are exposed via `maosctl audit query --kind governance`

**Given** cost attribution per Spirit per task per principal (FR64)
**When** the kernel records external-call costs in the Transparency Log
**Then** token-spend per provider, subprocess CPU-time, storage I/O are attributed to the originating Spirit + task + principal
**And** an enterprise operator can produce a per-tenant cost report

**Given** monthly reconciliation against provider billing (NFR-Cost-1)
**When** the operator runs `maosctl audit cost-reconcile --month <YYYY-MM>`
**Then** reconciliation accuracy is ≥98% against provider billing statements
**And** discrepancies are flagged for investigation

## Story 9.4: Productionize the Operator Surface — Distribution, Backup/DR, Air-Gap, Region-Pinning

As an enterprise operator deploying MAOS to production,
I want pre-built binaries (Linux amd64/arm64, macOS arm64) via signed GitHub Releases (v0.5) progressing to Homebrew/AUR/deb/rpm/container images (v1.0), Transparency Log backup/DR with RPO ≤1h / RTO ≤4h, air-gapped deployment validation in CI, region-pinning primitive (PIPL §40), Spirit model-provenance manifest field (SB-1047), AND multi-operator tenancy primitive-reservation (v1.0 reserved, v1.5+ implemented),
So that v1.0 ships to production-tenant operators without ad-hoc deployment glue.

**Acceptance Criteria:**

**Given** pre-built binaries at v0.5
**When** the operator runs `maosctl install` from a GitHub Releases artifact
**Then** the artifact has SHA256 + Ed25519 verification mandatory (FR1)
**And** Linux amd64/arm64 + macOS arm64 binaries are signed and published

**Given** package manager distribution at v1.0
**When** the operator installs via Homebrew tap, AUR, deb, or rpm
**Then** install succeeds with the same signature verification
**And** container images on Docker Hub / GHCR pass the same verification
**And** Windows binary lands at v1.5 (E10 Story 10.5)

**Given** Transparency Log backup/DR (NFR-Ops-9)
**When** backup runs
**Then** RPO ≤1h, RTO ≤4h
**And** backup integrity verified weekly via Merkle-root cross-check
**And** restore drill is documented and tested

**Given** air-gapped deployment validation (NFR-Ops-12)
**When** the substrate boots in network-namespace-isolated CI
**Then** the substrate boots, runs, and produces transparency-log entries with zero outbound network calls
**And** the structural test is enforced in CI
**And** documented Spirit-author guidance for air-gapped capability tokens

**Given** region-pinning primitive (NFR-Comp-4 / PIPL §40)
**When** the operator configures region pinning
**Then** Transparency Log + working-memory store are pinned to a single jurisdictional region
**And** cryptographic enforcement prevents cross-region replication
**And** any attempt to cross-region replicate fails with `ERegionViolation`

**Given** Spirit model-provenance manifest field (NFR-Comp-5 / SB-1047)
**When** a Spirit declares `[model_provenance]` with `covered_model_id`, `training_data_lineage`, `last_eval_timestamp`
**Then** the substrate validates field presence at admission
**And** missing or stale provenance is rejected

**Given** multi-operator tenancy primitive-reservation (NFR-Ops-11 v1.0)
**When** the namespace grammar reserves multi-operator primitives
**Then** per-operator namespace, per-operator transparency-log shard, per-operator capability-token signing key, per-operator GDPR-erasure scope are declared as primitive-reserved
**And** the grammar lock (NFR-Test-11) doesn't paint future implementations into a corner
**And** full implementation arrives v1.5+

## Story 9.5: Publish Five Canonical Docs with WCAG AA, Korean i18n, and Onboarding Artifacts

As a v1.0 substrate published to the world,
I want the 5 canonical doc deliverables (manifest schema reference + pattern cookbook + migration runbooks + troubleshooting + deployment topology) with WCAG AA compliance + Korean i18n + onboarding artifacts (RFC_TEMPLATE.md / GOVERNANCE.md / CODE_OF_CONDUCT.md / LOCALES.md / TRADEMARK.md / BREAKING.md) AND the trust-anchor framing carry-forward ADR published by v0.3,
So that the documentation surface is real, accessible, localized, and the competitive framing decision is locked in before v1.0.

**Acceptance Criteria:**

**Given** the 5 canonical doc deliverables (NFR-Doc-4)
**When** the doc site is built
**Then** each URL renders 200: `docs.maos.dev/manifest/<version>/` (≥1 example per field) / `docs.maos.dev/cookbook/` (≥10 patterns) / `docs.maos.dev/migrate/` / `docs.maos.dev/troubleshoot/` / `docs.maos.dev/deploy/`
**And** the troubleshooting guide covers 100% of FR63 error catalog
**And** the API reference at `docs.maos.dev/abi/<version>/` is versioned, searchable, deep-linkable, archived ≥2 minor versions back (NFR-Doc-3)

**Given** WCAG AA compliance (NFR-Doc-5)
**When** the doc site is audited
**Then** WCAG AA conformance is verified for color contrast, keyboard navigation, screen reader support
**And** automated accessibility tests run in CI

**Given** Korean i18n at v1.0 (NFR-Doc-6)
**When** the doc site is built with `--lang ko`
**Then** Korean translations render with deep-link preservation
**And** the LOCALES.md glossary lock applies — terms NEVER translated: Spirit, Worker, kernel, ADR identifiers, error codes
**And** Japanese + Chinese-simplified land at v1.5 (E10 Story 10.5)
**And** RTL layout deferred to v2.5

**Given** doc tooling (NFR-Doc-7)
**When** the operator builds docs
**Then** per-locale builds work with fallback to English
**And** language switcher preserves deep-link
**And** version dropdown switches between archived ABI versions
**And** mdBook + i18n / Docusaurus / VitePress decision is made by v0.5; in production by v1.0

**Given** onboarding artifacts (NFR-Ops-6)
**When** v1.0 ships
**Then** `RFC_TEMPLATE.md` (v0.8) / `GOVERNANCE.md` (v0.5 basic + v0.8 locked) / `CODE_OF_CONDUCT.md` (v0.5) / `LOCALES.md` (v1.0) / `TRADEMARK.md` (v1.0) / `BREAKING.md` (v1.0) all exist at the repo root
**And** each artifact is referenced from the doc site

**Given** sustainability vehicle (NFR-Ops-7)
**When** v1.0 ships
**Then** Open Collective declared-intent is published (v0.5)
**And** legal/fiscal-sponsor work is initiated (v0.8)

**Given** trust-anchor framing carry-forward ADR (NFR-Ops-8)
**When** v0.3 release approaches
**Then** a published ADR declares which competitive framing is committed (substrate-as-substrate vs substrate-as-trust-anchor)
**And** absence of this ADR is a v0.3 release-block
**And** STABILITY.md contains the substrate-self compliance scope clause (NFR-Comp-3) — SOC 2 / ISO 27001 / FedRAMP scope is operator's responsibility

**Given** OpenTelemetry adapter at v1.0 SLO-class (NFR-Aud-11)
**When** the operator configures OTel export
**Then** structured trace IDs and span linkage are exported per IAC frame, capability invocation, halt event
**And** SIEM export lands at v2.0 (NFR-Aud-11 second phase)

---
