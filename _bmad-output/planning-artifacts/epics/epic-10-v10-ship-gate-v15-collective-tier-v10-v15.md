# Epic 10: v1.0 Ship Gate + v1.5 Collective Tier (v1.0 → v1.5)

**Goal:** v1.0 release certification (pen-test + CCAC + HSIS + third-party trial + cross-form equivalence) AND v1.5 collective tier maturation (Postgres+pgvector Loom-lite, SQLite→Postgres migration, Mira+Nash diagnostic-architect bilateral pair, skill-format conformance, JetBrains plugin-bridge, Windows binary, 2-year LTS commitment). Two phase sub-clusters in one epic (user's selected option).

**Sub-cluster A — v1.0 Ship Gate (coordination + final certification gates):**

- **Pen-test report zero P0/P1 (NFR-Sec-7)**: external pen-test; triage by joint panel of pen-test lead + MAOS security owner; disagreements escalate to PRD-author tiebreak; P0/P1 definitions per OWASP Risk Rating Methodology frozen at engagement start.
- **CCAC N=600 cross-validation execution** (NFR-Aud-9 gate — corpus authored E7): ≥3 reference Spirits, agreement within ±2%.
- **HSIS ≥95% verification** (NFR-Rel-3 gate — corpus authored E5): 6 class-specific corpora × 50 scenarios; zero invariant violations CVSS-7 class.
- **Third-party trial N=12 stratified execution** (NFR-Test-8): ≥4 no prior MAOS contribution / ≥3 never written Rust Spirit / ≥2 never written Rust at all / ≥2 non-English-native / ≥1 working offline-only. 14-day no-DM-support window. Floor ≥10/12 produce working signed Spirit binary that loads on fresh Host VM, runs ≥1000 frames, halt-recall ≥0.85. Wilson CI [0.552, 0.962] meaningful at N=12. Auditable via SBOM + signing chain re-loaded on clean VM by CI bot.
- **Cross-form semantic equivalence** (NFR-Test-7) — if rust-inproc form active per E5 §13.1 measurement gate: rust-inproc ↔ subprocess ≥90%; CLI-wrapper distributional via Mann-Whitney U-test p>0.05 over 30 runs.
- **STABILITY.md publication** (NFR-Maint-4): live (kernel, abi, manifest_schema) compatibility matrix + LTS branch policy + substrate-self compliance scope clause + export-control classification.
- BREAKING.md publication (NFR-Maint-7).
- 1-year LTS clock starts (NFR-Maint-6 v1.0).
- Korean localization shipped (NFR-Doc-6 v1.0).
- Export-control classification artifact (NFR-Comp-1): ECCN classification letter on file, EAR99 vs 5D002 determination published in `STABILITY.md §Export`, dual-use review for crypto primitives in kernel.
- CNA registration through MITRE (per NFR-Ops-4).
- Adversarial-Spirit red-team **80-scenario corpus authoring + gate** (NFR-Sec-10): 8 attack classes × N=10 — capability confusion / IAC frame injection / distillation poisoning / ledger tampering / cross-Spirit privilege escalation / resource exhaustion / side-channel timing / kernel-syscall abuse. Floor ≥9/10 per class, ≥72/80 aggregate, 0 unmitigated category. Authored by external pen-tester using published ABI. Phase per PRD = v1.5, but corpus authoring can begin v1.0 prep.

**Sub-cluster B — v1.5 Collective Tier:**

- Postgres+pgvector Loom-lite (collective tier of memory via MCP-Streamable-HTTP — kernel mediates access; Loom is user-space per ADR-006 / NFR-Test-9).
- **SQLite→Postgres migration test corpus** (NFR-Ops-10): forward-migration on 10⁶-row corpus; byte-identical Merkle-root preservation post-migration; rollback path tested. **v1.4 gate (gates v1.5).**
- Mira + Nash diagnostic-architect bilateral pair v1.5 (overlap with E8 Mira+Nash sub-story; coordinate ownership): pre-paired mTLS cert fingerprints; mobile push to operator; J4 latency budget <10ms P95.
- Skill-format conformance (NFR-Test-10): ≥1 third-party skill format (Anthropic Skills format OR equivalent) executes via Spirit-form adapter without kernel modification.
- JetBrains plugin-bridge for ACP (NFR-Ops adjacent).
- Windows binary v1.5.
- 2-year LTS commitment v1.5 (NFR-Maint-6).
- Japanese + Chinese-simplified localization (NFR-Doc-6 v1.5; RTL deferred to v2.5).
- mTLS cert rotation chaos test 3-host v1.5 / 10-host v2.0 (NFR-Sec-13 full).
- Manifest parser fuzz 24h `cargo-fuzz` (NFR-Sec-5) + wire protocol adversarial-input fuzz 24h (NFR-Sec-6) — v1.0 ship gates.
- ComplianceClaim envelope schema migration validation (test continuity through v1.5).

**FRs covered:** Closes any FR not closed in earlier epics; v1.5-phase implementations of FR42 (subject-access at Postgres scale), FR44 (sealed-export with Loom-lite content), FR58 v1.5 reference Spirit cohort (Mira+Nash overlap with E8).

**Key NFRs (this is the densest NFR-execution epic):**
- v1.0 gates: NFR-Sec-5 (manifest fuzz 24h zero crashes), NFR-Sec-6 (wire fuzz 24h zero crashes), NFR-Sec-7 (pen-test zero P0/P1), NFR-Sec-10 (adversarial red-team v1.5), NFR-Aud-9 (CCAC N=600 execution), NFR-Rel-3 (HSIS ≥95% execution), NFR-Test-1 (corpus discipline at v1.0), NFR-Test-5 (FKCS infrastructure v2.0 deferred; lay groundwork), NFR-Test-7 (cross-form equivalence — if rust-inproc active), NFR-Test-8 (third-party trial N=12 execution), NFR-Test-10 (skill-format conformance v1.5), NFR-Maint-6 (1-year LTS v1.0; 2-year v1.5), NFR-Comp-1 (export-control), NFR-Doc-6 (Korean v1.0; Japanese+CN-S v1.5).

**Corpora authored in E10:**
- **Adversarial red-team 80-scenario corpus** (generator: 8 attack classes × 10 canonical scenarios × 8× expansion via parameter variation = 640 effective items per Murat's generator discipline) — P0 ship-block if any false negative.
- SQLite→Postgres migration corpus 10⁶ rows.
- Skill-format conformance corpus ~50 cases.
- mTLS cert rotation chaos scenarios (3-host v1.5).
- Manifest parser fuzz inputs (24h cargo-fuzz).
- Wire protocol adversarial fuzz inputs (24h).

**Acceptance demos:**
- **v1.0 ship gate passes:** pen-test report zero P0/P1; CCAC ≥27/30 per-class with ≥3 Spirits ±2% agreement; HSIS ≥95% across 6 corpora × 50 scenarios; N=12 third-party trial ≥10/12 succeed; STABILITY.md + BREAKING.md published; 1-year LTS clock starts; Korean docs live; export-control letter on file.
- **v1.5 collective tier:** Mira on Host A + Nash on Host B coordinate over A2A; SQLite→Postgres migration on 10⁶-row corpus produces byte-identical Merkle root; skill-format adapter executes ≥1 third-party skill without kernel modification; Windows binary builds on CI; 2-year LTS commitment.

### Stories

## Story 10.1: Execute the v1.0 Release Gate — Pen-Test, CCAC Cross-Validation, HSIS Verification

As a substrate release manager certifying v1.0,
I want the v1.0 ship-gate coordination story: external pen-test report zero P0/P1 (NFR-Sec-7), CCAC N=600 cross-validation against ≥3 reference Spirits with agreement ±2% (NFR-Aud-9 — corpus authored E7), HSIS ≥95% verification against the 6×50=300 scenario corpus (NFR-Rel-3 — corpus authored E5), AND STABILITY.md + BREAKING.md publication with 1-year LTS clock start,
So that v1.0 release is gated by mechanically-verifiable evidence, not aspirational claims.

**Acceptance Criteria:**

**Given** an external pen-test engagement
**When** the pen-test concludes
**Then** the report shows zero P0/P1 findings open at v1.0 ship (NFR-Sec-7)
**And** triage is performed by a joint panel of pen-test lead + MAOS security owner
**And** disagreements escalate to PRD-author tiebreak
**And** P0/P1 definitions per OWASP Risk Rating Methodology are frozen at engagement start

**Given** the CCAC corpus N=600 (authored E7 Story 7.3)
**When** the corpus runs cross-validation against ≥3 reference Spirits
**Then** per-class floor ≥27/30 passes per Spirit
**And** cross-Spirit agreement is within ±2%
**And** 100/100 context-drift claims are rejected at admission
**And** failure on this gate is a P0 ship-block

**Given** the HSIS corpus 6×50=300 scenarios (authored E4+E5)
**When** the HSIS gate runs
**Then** ≥95% pass per Spirit class (NFR-Rel-3)
**And** zero invariant violations CVSS-7 class
**And** stratified swap-lifecycle phase distribution is verified

**Given** STABILITY.md publication (NFR-Maint-4 — content authored E7 Story 7.5)
**When** v1.0 release is cut
**Then** STABILITY.md is published with live `(kernel, abi, manifest_schema)` compatibility matrix
**And** 1-year LTS commitment clock starts (NFR-Maint-6)
**And** BREAKING.md (NFR-Maint-7) is grep-enforced — every breaking change requires an entry with migration steps

## Story 10.2: Run the Third-Party Trial N=12 and Adversarial Red-Team Gate at v1.0

As a substrate quality lead at v1.0,
I want the black-box third-party trial with N=12 stratified humans executed (NFR-Test-8), cross-form semantic equivalence verified IF rust-inproc form is active per E5 §13.1 measurement (NFR-Test-7), AND the adversarial-Spirit red-team 80-scenario corpus authored and run (NFR-Sec-10 v1.5),
So that v1.0 has externally-validated authorship + behavioral-equivalence + adversarial-resistance evidence.

**Acceptance Criteria:**

**Given** the N=12 stratified recruitment (≥4 with no prior MAOS contribution; ≥3 never written Rust Spirit; ≥2 never written Rust at all; ≥2 non-English-native; ≥1 working offline-only)
**When** the trial runs with a 14-day no-DM-support window
**Then** floor ≥10/12 produce a working signed Spirit binary
**And** the binary loads on a fresh Host VM
**And** runs ≥1000 frames
**And** achieves halt-recall ≥0.85
**And** Wilson CI [0.552, 0.962] meaningful at N=12 (meaningless at N=5 per NFR-Test-8)
**And** auditable via SBOM + signing chain re-loaded on clean VM by CI bot
**And** trials run only at major releases (v1.0, v2.0); minor releases use NFR-Onb-1 as proxy

**Given** the E5 §13.1 measurement outcome
**When** the gate evaluates rust-inproc form status
**Then** if rust-inproc form is active: cross-form semantic equivalence (NFR-Test-7) is verified — rust-inproc ↔ subprocess ≥90% / any-rust ↔ wasm ≥75%
**And** CLI-wrapper requires distributional behavioral equivalence (Mann-Whitney U-test p > 0.05 over 30 runs)
**And** if rust-inproc form is DEFERRED to v2.0+ per Story 5.5 outcome: NFR-Test-7 cross-form is removed from v1.5 scope and CLI-wrapper test runs only

**Given** the adversarial-Spirit red-team corpus (NFR-Sec-10)
**When** the corpus is authored
**Then** 8 attack classes × N=10 = 80 canonical scenarios, expanded via parameter variation to 640 effective items (Murat's generator discipline)
**And** attack classes: capability confusion / IAC frame injection / distillation poisoning / ledger tampering / cross-Spirit privilege escalation / resource exhaustion / side-channel timing / kernel-syscall abuse
**And** the corpus is authored by an external pen-tester (not MAOS team) using published ABI
**And** the corpus is pre-frozen and content-addressed

**Given** the adversarial-Spirit red-team gate
**When** the corpus runs
**Then** floor ≥9/10 per class detected/blocked by kernel
**And** floor ≥72/80 aggregate
**And** 0 unmitigated categories
**And** failure is a v1.5 ship-block (NFR-Sec-10 phase v1.5)

## Story 10.3: Close v1.0 Compliance Gates — Export-Control, Fuzz Hardening, Korean Docs, CNA Registration

As a v1.0 compliance/security lead,
I want export-control classification artifact (NFR-Comp-1) + manifest parser fuzz 24h cargo-fuzz zero crashes (NFR-Sec-5) + wire-protocol adversarial-input fuzz 24h zero crashes (NFR-Sec-6) + Korean localization shipped (NFR-Doc-6) + CNA registration through MITRE (per NFR-Ops-4),
So that the substrate is regulatory-ready, fuzz-hardened, localized, and vulnerability-pipeline ready for v1.0 enterprise distribution.

**Acceptance Criteria:**

**Given** the export-control classification artifact (NFR-Comp-1)
**When** v1.0 ships
**Then** ECCN classification letter is on file
**And** EAR99 vs 5D002 determination is published in STABILITY.md §Export
**And** dual-use review for crypto primitives in kernel is complete
**And** absence is a v1.0 ship-block

**Given** manifest parser fuzz (NFR-Sec-5)
**When** 24h `cargo-fuzz` runs
**Then** zero crashes / OOMs / infinite loops
**And** results are published as a v1.0 ship-gate artifact

**Given** wire-protocol adversarial-input fuzz (NFR-Sec-6)
**When** 24h fuzz runs
**Then** zero crashes
**And** tiered cadence per §5.2: T1 per-commit (10 min, N=4 workers) / T2 nightly (4h, N=8) / T3 pre-release (24h, N=8)
**And** per-target floor ≥72 CPU-hours per fuzz target across 90 days pre-GA
**And** aggregate floor ≥1,000 CPU-hours pre-GA

**Given** Korean localization (NFR-Doc-6 v1.0)
**When** v1.0 doc site is built
**Then** Korean translations are present for all 5 canonical doc deliverables (E9 Story 9.5)
**And** LOCALES.md glossary lock applies
**And** deep-link preservation works across language switcher

**Given** CNA registration through MITRE (per NFR-Ops-4)
**When** v1.0 ship gate runs
**Then** CNA registration is complete (moved from v0.5 to v1.0 per NFR-Ops-4)
**And** advisory-publication channel is operational
**And** disclosure pipeline is exercised with at least one synthetic advisory before v1.0 ship

## Story 10.4: Ship the v1.5 Collective Tier with Postgres+pgvector and SQLite→Postgres Migration

As a v1.5 operator deploying the diagnostic-architect bilateral 2-Host pair,
I want the Postgres+pgvector Loom-lite collective tier (kernel-mediated via MCP-Streamable-HTTP per ADR-006 / NFR-Test-9), SQLite→Postgres migration test corpus on 10⁶ rows with byte-identical Merkle-root preservation (NFR-Ops-10), AND the Mira+Nash bilateral pair coordination (overlap with E8 Story 8.5),
So that v1.5 ships the collective-memory tier and the 2-Host deployment topology as a working, audit-traced operation — not a slide.

**Acceptance Criteria:**

**Given** the Postgres+pgvector Loom-lite collective tier
**When** the substrate boots with collective tier configured
**Then** the kernel mediates collective-tier access via MCP-Streamable-HTTP (no kernel module — Loom-lite is user-space per ADR-006)
**And** the Loom-not-in-kernel grep (Story 0.2 / NFR-Test-9) continues to return ∅
**And** RPO ≤1h / RTO ≤4h backups are verified weekly via Merkle-root cross-check

**Given** the SQLite→Postgres migration test corpus (NFR-Ops-10)
**When** the migration runs against a 10⁶-row corpus
**Then** forward-migration is byte-identical Merkle-root preserving
**And** rollback path is tested
**And** v1.4 gates v1.5 — migration test must pass before v1.5 release

**Given** Mira + Nash diagnostic-architect bilateral pair (cross-ref E8 Story 8.5)
**When** Host A (prod-edge with Mira) and Host B (dev-environment with Nash) are deployed
**Then** A2A cross-Host (E6 Story 6.3) operates between them
**And** pre-paired mTLS cert fingerprints are configured (no discovery)
**And** mobile push to operator works on halt
**And** J4 latency budget met: Mira-Nash Observer colocation <10ms P95 (§13.1)

**Given** 14-institution Cortex capacity envelope (NFR-Scale-5)
**When** the v1.5 capacity test runs at scale
**Then** the capacity envelope is documented for the v1.5 release
**And** 25-host churn test passes (NFR-Scale-2 v2.0 compressed scope; full 100-host at v2.5)

## Story 10.5: Mature v1.5 — Skill-Format Conformance, JetBrains, Windows, 2-Year LTS, Japanese/CN-S i18n

As a v1.5 operator on the long-term-support promise,
I want skill-format conformance demonstrating ≥1 third-party skill format (Anthropic Skills) executes via Spirit-form adapter without kernel modification (NFR-Test-10), JetBrains plugin-bridge for ACP (v1.5), Windows binary, 2-year LTS commitment, AND Japanese + Chinese-simplified localization,
So that v1.5 is the long-term-support release with proven extensibility across editor / OS / language boundaries.

**Acceptance Criteria:**

**Given** skill-format conformance (NFR-Test-10)
**When** the test runs
**Then** ≥1 third-party skill format (Anthropic Skills format OR equivalent) executes via Spirit-form adapter without kernel modification
**And** the kernel ABI is unchanged by the adapter — verified via ABI-diff lint
**And** the conformance result is journaled as a v1.5 release artifact

**Given** JetBrains plugin-bridge for ACP (v1.5)
**When** the plugin is installed in JetBrains IDE
**Then** Spirits hosted via ACP are routable through JetBrains (NDJSON over stdio extended)
**And** the bridge does not require kernel modification

**Given** Windows binary (v1.5)
**When** the operator runs `maosctl install` on Windows
**Then** the install succeeds with the same signature verification as Linux/macOS (FR1)
**And** sandbox tier T2 uses Windows restricted-token (Story 1b.3 cross-ref)
**And** per-Spirit resource caps use Job Objects (Story 1b.3 cross-ref)

**Given** 2-year LTS commitment (NFR-Maint-6 v1.5)
**When** v1.5 ships
**Then** the LTS clock extends from 1-year (v1.0) to 2-year once support load is known
**And** STABILITY.md is updated with the new LTS span

**Given** Japanese + Chinese-simplified localization (NFR-Doc-6 v1.5)
**When** v1.5 doc site is built
**Then** Japanese and Chinese-simplified translations are present for all 5 canonical doc deliverables
**And** LOCALES.md glossary lock continues to exclude Spirit / Worker / kernel / ADR identifiers / error codes
**And** RTL layout support remains deferred to v2.5

**Given** mTLS cert rotation chaos test full execution (NFR-Sec-13)
**When** the 3-host v1.5 rotation chaos runs
**Then** zero conversation drops are observed
**And** revocation latency median ≤60s / p99 ≤5min
**And** 10-host rotation defers to v2.0

**Given** ComplianceClaim envelope schema migration validation through v1.5
**When** schema evolution is tested
**Then** any ABI-breaking change to required fields, removed fields, renames, type-changes, or enum reorderings triggers an `ABI_VERSION` bump per §8.5
**And** additive optional fields with `#[serde(default, skip_serializing_if = "Option::is_none")]` and additive enum variants with explicit `#[repr(u8)]` discriminants + `#[serde(other)]` fallback do NOT bump

---
