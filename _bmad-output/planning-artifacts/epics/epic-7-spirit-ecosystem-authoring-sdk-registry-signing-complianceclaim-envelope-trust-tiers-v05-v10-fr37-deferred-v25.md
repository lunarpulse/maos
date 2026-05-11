# Epic 7: Spirit Ecosystem — Authoring SDK, Registry, Signing, ComplianceClaim Envelope & Trust Tiers (v0.5 → v1.0; FR37 deferred v2.5)

**Goal:** A third-party Spirit author scaffolds, tests, signs, and publishes a Spirit; operator installs it across three trust tiers (`local`, `org-internal`, `public-untrusted`) with mandatory Ed25519 signature verification + ComplianceClaim envelope verification at admission + revocation propagation. NFR-Onb-1 30-Min First Spirit Validation Gate executes at v0.3 (Butler-driven) using E2 prerequisites + E8 Butler reference.

**Owns:**
- Full `cargo generate maos-spirit` per-language (Rust v0.5; TypeScript v0.5; Python v1.0; Go v1.5).
- Full `spirit-test` SDK with assertion macros (extends E2 SDK seed): covers lifecycle hooks, IAC frame I/O, halt resolution, manifest self-check, class-specific regression corpus. Coverage floor ≥80% of Spirit-author manifest-declared capabilities reachable via fixtures.
- `maos-spirit publish --tier=<tier>` CLI with Ed25519 signing; package conforms to `maos.spirit.v1` schema.
- Spirit registry full features (over MCP-Streamable-HTTP): `registry.search`, `registry.manifest`, `registry.artifact`, `registry.publish`, `registry.deprecate`.
- Three trust tiers at v1.0: `local`, `org-internal`, `public-untrusted` (PRD's `public-vetted` deferred to v2.5 via FR37).
- ComplianceClaim envelope (binding-v1.0 first-class object): Ed25519-signed, references execution-context fingerprint (manifest hash + version + trust tier + sandbox tier + capability scope set + provider-endpoint pinning + crypto-provider identity). Kernel verifies at admission with typed `EComplianceContextDrift` on drift.
- ComplianceClaim Adversarial Corpus (CCAC) v1.0: **N=600 = 200 well-formed + 400 malformed** (authored via parameterized generator: 20 well-formed templates × 10 variations = 200; 40 malformed templates × 10 variations = 400). Per-class N=30 floor ≥27/30. 100 context-drift claims 100/100 rejected. Cross-validation across ≥3 reference Spirits, agreement within ±2%.
- `maos-compliance` semantic evaluator (v0.9 binding) for ComplianceClaim envelope validation.
- Skill authoring (`maos.skill.v1` markdown + TOML frontmatter): ship in Spirit package OR write dynamically at runtime via `skill.author.self` capability scope; new skills enter operator-admission queue.
- CliWrapper `output_shape_version` fail-loud (FR40 full): kernel-builtin CliWrapperSpirit refuses to start if observed CLI shape doesn't match declared version.
- Skill-revision proposals (FR57): Spirit queries own performance telemetry within principal namespace (FR31 + FR56); emits proposal carrying target skill id + version + proposed diff + telemetry evidence; enters operator-admission queue (FR39).
- Registry yank events (FR59): publisher- and vetter-initiated yanks propagate to operators on next sync (≤5min poll cadence default); distinguishable from operator-local revocation (FR13).
- Air-gapped artifact import (FR60): signed artifacts (Spirit + skills) from offline media or mirrored registries preserve full verification chain.
- ABI Stability Triple `(kernel_version, abi_version, manifest_schema_version)` with N-1 supported / N-2 hard refusal (typed `EAbiTooOld`).
- STABILITY.md live (kernel, abi, manifest_schema) compatibility matrix + LTS branch policy + substrate-self compliance scope clause + export-control classification.
- 1-year LTS commitment at v1.0; 2-year LTS at v1.5 (deferred to E10).
- BREAKING.md grep-enforced entry for every breaking change with migration steps.
- Deprecation timeline: 2 minor releases warning + 1 major to remove.
- `min_substrate_version` manifest field enforcement; manifest schema N-1 compatibility with documented degradation paths (NFR-Maint-9).
- Skill ecosystem: filesystem-discovered at v0.5 (`~/.maos/skills/`, `_bmad/skills/`, `/usr/share/maos/skills/`); `maos.skill.v1` format intentionally close to Anthropic Skills format.

**FRs covered:** FR33 (full), FR34 (full), FR35, FR36, FR37 (DEFERRED v2.5), FR38 (envelope + admission verification v1.0 — schema frozen E1b), FR39, FR40 (full fail-loud), FR57, FR59, FR60.

**Key NFRs:** **NFR-Aud-9 (CCAC N=600 ship gate)**, NFR-Comp-2 (vetter accreditation parameters — qualification matrix, conflict-of-interest, 7-year audit retention), NFR-Test-3 (spirit-test SDK ≥80% coverage validated by 5+ third-party Spirits), **NFR-Onb-1 (30-Min First Spirit Gate execution: N=12 stratified — ≥4 no prior MAOS contribution / ≥3 never written Rust Spirit / ≥2 never written Rust at all / ≥2 non-English-native / ≥1 working offline-only; floor median ≤45min, p95 ≤90min, ≥10/12 succeed in 14 days zero-DM-support — Butler-class corpus 30-scenario calendar/comms; halt-recall ≥0.90 calendar-conflict; halt-precision ≥0.85)**, NFR-Onb-3 (three-door page `docs.maos.dev`), NFR-Onb-4 (gate iteration cadence: 3 consecutive misses escalates to v0.3 release review), NFR-Maint-3 (ABI compat 100% within major, 100% N-1), NFR-Maint-4 (STABILITY.md), NFR-Maint-5 (deprecation 2-minor + 1-major), NFR-Maint-6 (1-year LTS at v1.0), NFR-Maint-7 (BREAKING.md), NFR-Maint-9 (manifest N-1).

**Corpora authored in E7:**
- **CCAC corpus N=600** (generator-driven: 20 well-formed templates × 10 variations + 40 malformed templates × 10 variations).
- 30-Min First Spirit Gate task scripts (~24 task scripts for N=12 trial — 2 per participant).
- Three-door page content (`docs.maos.dev`).
- 5+ third-party Spirit external trials for NFR-Test-3 coverage validation.

**Acceptance demo:** External author scaffolds Spirit, signs and publishes to local registry; operator installs from `org-internal` tier with ComplianceClaim envelope verification at admission; signed Revocation List propagates within 5min; air-gapped operator imports same artifact preserving signature chain; 30-Min Gate cohort succeeds 10/12.

### Stories

## Story 7.1: Full `cargo generate` Per-Language + Full spirit-test SDK with Assertion Macros

As a Spirit author across multiple languages,
I want the full `cargo generate maos-spirit` per-language templates (Rust v0.5; TypeScript v0.5; Python v1.0; Go v1.5) AND the full spirit-test SDK with assertion macros covering lifecycle hooks + IAC frame I/O + halt resolution + manifest self-check + class-specific regression corpus,
So that the v0.5 ecosystem expansion supports non-Rust authors and the SDK coverage floor ≥80% (NFR-Test-3) is mechanically verifiable.

**Acceptance Criteria:**

**Given** the `cargo generate maos-spirit --lang <rust|typescript|python|go>` invocation
**When** the template scaffolds a Spirit
**Then** Rust + TypeScript templates work at v0.5
**And** Python template lands at v1.0
**And** Go template lands at v1.5
**And** every template ships with a working `on_idle` example, manifest, README, and passing CI

**Given** the full spirit-test SDK extending the E2 seed
**When** an author calls `spirit_test::assert!` / `spirit_test::expect_halt!` / `spirit_test::expect_frame!` macros
**Then** the macros provide compile-time-checked assertions against the Spirit ABI
**And** the macros render readable failure messages with file + line + suggested-fix

**Given** the SDK coverage floor (NFR-Test-3)
**When** measured against 5+ third-party Spirits authored by external developers
**Then** ≥80% of each Spirit-author's manifest-declared capabilities are reachable via SDK fixtures
**And** the measurement is committed to `coverage-matrix.yaml`

**Given** the Spirit-side `kernel.deprecation_warnings()` channel
**When** a Spirit uses a deprecated API
**Then** `spirit-test` surfaces the deprecation in test output
**And** the channel is consulted by the ABI compatibility matrix gate (NFR-Maint-3)

## Story 7.2: Ship End-to-End Registry — Publish, Install, Yank, and Air-Gapped Import

As an operator distributing Spirits,
I want `maos-spirit publish --tier=<tier>` with Ed25519 signing (FR35), full Spirit registry over MCP-Streamable-HTTP with all five operations (FR36), three trust tiers at v1.0 (`local`, `org-internal`, `public-untrusted`; `public-vetted` deferred to v2.5 via FR37), registry yank events propagating ≤5min (FR59), AND air-gapped artifact import preserving full verification chain (FR60),
So that the full publish → discover → install → revoke → air-gap-import loop works end-to-end at v1.0.

**Acceptance Criteria:**

**Given** the `maos-spirit publish --tier=<tier>` CLI
**When** an author publishes a Spirit
**Then** the published package conforms to `maos.spirit.v1` schema
**And** the package is Ed25519-signed
**And** the tier is one of `local` / `org-internal` / `public-untrusted` (FR37 `public-vetted` deferred v2.5)

**Given** the Spirit registry (Story 5.5 + this story extends)
**When** an operator invokes `registry.search` / `registry.manifest` / `registry.artifact` / `registry.publish` / `registry.deprecate`
**Then** all five operations succeed against the configured registry endpoint
**And** mandatory signature verification + trust-tier floor enforcement runs at admission (FR36)

**Given** a publisher- or vetter-initiated yank event
**When** the kernel polls the registry (≤5min default cadence)
**Then** running Spirit instances receive the yank notification within 5min (FR59)
**And** the yank is distinguishable in audit from operator-local revocation (FR13)
**And** operator response semantics (warn / quarantine / auto-revoke) apply per operator policy

**Given** air-gapped operator import
**When** the operator runs `maosctl import --offline <signed-bundle.tar>`
**Then** the kernel verifies the Ed25519 signing chain on the bundle (FR60)
**And** vetter attestations and ComplianceClaim envelopes in the bundle verify locally
**And** the imported Spirit is admitted equivalently to registry-served Spirits

## Story 7.3: Verify ComplianceClaim Envelopes at Admission with the CCAC N=600 Ship Gate

As a substrate compliance lead,
I want the ComplianceClaim envelope as a binding-v1.0 first-class Ed25519-signed object referencing the execution-context fingerprint (manifest hash + version + trust tier + sandbox tier + capability scope + provider-endpoint + crypto-provider) AND the CCAC corpus N=600 (NFR-Aud-9) as a v1.0 ship gate AND the `maos-compliance` semantic evaluator (v0.9 binding),
So that admission verification mechanically rejects context-drifted Spirits and the v1.0 ship-gate evidence is third-party-reproducible.

**Acceptance Criteria:**

**Given** a Spirit declared with execution-context fingerprint
**When** the operator admits the Spirit
**Then** the kernel verifies the ComplianceClaim envelope's Ed25519 signature
**And** the kernel computes the runtime execution-context fingerprint
**And** drift between declared and runtime fingerprint triggers admission rejection with typed `EComplianceContextDrift` (FR38)

**Given** the CCAC corpus N=600 (generator-driven per Murat's discipline)
**When** the corpus is authored
**Then** it comprises 200 well-formed (20 templates × 10 variations) + 400 malformed (40 templates × 10 variations) ComplianceClaim envelopes
**And** per-class N=30 minimum
**And** 100 context-drift claims are present (100/100 rejected at admission)

**Given** the CCAC v1.0 ship gate
**When** the corpus runs against ≥3 reference Spirits
**Then** per-class floor ≥27/30 passes
**And** cross-validation across the 3 Spirits shows agreement within ±2%
**And** failure is a P0 ship-blocker

**Given** the `maos-compliance` crate (v0.9 binding)
**When** ComplianceClaim envelopes flow through the semantic evaluator
**Then** the evaluator validates structural correctness + signature + execution-context match
**And** validation latency does not bottleneck admission (<10ms P99 per envelope on a typical Linux box)

## Story 7.4: Author Skills and Propose Revisions with Output-Shape Fail-Loud

As a Spirit author authoring skills,
I want to author skills as `maos.skill.v1` (markdown + TOML frontmatter) shipped in the Spirit package OR written dynamically at runtime via `skill.author.self` capability scope, AND skill-revision proposals (FR57) from Spirit's self-telemetry entering the operator-admission queue, AND CliWrapper `output_shape_version` fail-loud (FR40 full),
So that the substrate's skill ecosystem is real (filesystem-discovered at v0.5) and Spirits can propose evidence-backed improvements to their own skills.

**Acceptance Criteria:**

**Given** the `maos.skill.v1` format (markdown + TOML frontmatter)
**When** an author writes a skill
**Then** the skill validates against the schema at admission
**And** the skill can be shipped in the Spirit's package
**And** the skill can be written dynamically at runtime via `skill.author.self` capability scope, entering the operator-admission queue (FR39)

**Given** filesystem-discovered skills at v0.5
**When** the kernel scans skill paths
**Then** conventional locations are checked: `~/.maos/skills/`, `_bmad/skills/`, `/usr/share/maos/skills/`
**And** discovered skills are surfaced via `maosctl skills list`

**Given** a Spirit emits a skill-revision proposal (FR57)
**When** the Spirit queries its own performance telemetry (E4 Story 4.3 FR56)
**Then** the proposal carries (target skill id + version, proposed diff, telemetry evidence)
**And** the proposal enters the operator-admission queue (FR39 path)
**And** the proposal is subject to the same vetting and audit obligations as new skills

**Given** a CliWrapperSpirit with declared `output_shape_version`
**When** the wrapped CLI's output shape changes (e.g., `claude code` updates and breaks parsing)
**Then** the kernel refuses to start the CliWrapperSpirit with `EOutputShapeAdapterMismatch` (FR40 full fail-loud)
**And** the failure is journaled with version diff
**And** the operator must publish an updated CliWrapperSpirit configuration before resumption

**Given** the LCAS (Long-context Ambiguity Stress) corpus extension — round-3 resolution of the orphaned 140 items
**When** Story 7.4 closes its acceptance gate at v1.0
**Then** the LCAS corpus has been extended from 70 items (Story 2.4 clearly-decidable bucket) to the full N=210 — adding 70 genuinely-ambiguous items + 70 adversarially-misleading items
**And** the genuinely-ambiguous bucket exercises Spirit decisions where multiple defensible answers exist
**And** the adversarially-misleading bucket exercises A2A scenarios with planted load-bearing claims contradicting louder repeated claims (requires E6 A2A loopback from Story 6.3 to be testable; therefore authored at v0.8 after E6 ships, but acceptance lives in this Story 7.4)
**And** all 210 items are committed to `tests/corpora/lcas-v1.0-<sha>.jsonl` (SHA-256-pinned per Story 0.3)
**And** the corpus is registered in `tests/coverage-matrix.yaml` with `valid_until` 12 months out

## Story 7.5a: Publish and Enforce v1.0 ABI Stability Commitments

As a substrate maintainer at v1.0 ship,
I want the ABI Stability Triple `(kernel_version, abi_version, manifest_schema_version)` enforced at kernel load (N-1 supported / N-2 hard refusal with typed `EAbiTooOld`), STABILITY.md + BREAKING.md published with the live compatibility matrix and LTS policy, AND `min_substrate_version` rejected loudly,
So that v1.0's ABI stability promise is mechanically enforced — not a marketing claim — and the 1-year LTS clock starts on a verifiable artifact.

**Acceptance Criteria:**

**Given** the ABI Stability Triple in `crates/maos-spirit-abi/src/version.rs`
**When** a Spirit declares `min_substrate_version` in its manifest
**Then** the kernel rejects load if its own version is below the declared minimum
**And** N-1 within current major is fully supported by the load path
**And** N-2 hard-refuses with typed `EAbiTooOld`
**And** the deprecation timeline (2 minor releases warning, 1 major to remove) is enforced by `crates/maos-spirit-abi/src/deprecation.rs` via tagged-warning emission per NFR-Maint-5

**Given** STABILITY.md at the repo root
**When** v1.0 ships
**Then** the file contains the live `(kernel_version, abi_version, manifest_schema_version)` compatibility matrix generated by `xtask/stability_matrix.rs` from the workspace state
**And** the file declares the LTS branch policy (1-year at v1.0; 2-year at v1.5)
**And** the file contains the substrate-self compliance scope clause (NFR-Comp-3 — full content authored in Story 9.5)
**And** the file contains the export-control classification (NFR-Comp-1 — full content authored in Story 10.3)
**And** BREAKING.md (NFR-Maint-7) is grep-enforced via CI — every breaking change requires a dated entry with migration steps

**Given** manifest N-1 compatibility (NFR-Maint-9)
**When** kernel version V loads a manifest written for V-1
**Then** the load succeeds with documented degradation paths emitted to `tracing` at WARN level
**And** unrecognized fields are warned, not rejected
**And** an integration test in `crates/maos-spirit-abi/tests/manifest_n_minus_1_test.rs` exercises the V→V-1 path for every supported field

**Given** the 1-year LTS commitment
**When** v1.0 ships
**Then** the LTS clock starts and is published in STABILITY.md with the v1.0 commit SHA + tag
**And** security-only patches after year 1 are documented in the policy section

## Story 7.5b: Execute NFR-Onb-1 30-Minute First Spirit Validation Gate at v0.3

As the human-research lead validating MAOS's onboarding floor,
I want the NFR-Onb-1 gate executed at the v0.3 release with N=12 stratified Spirit authors (with documented recruitment, screener, support-log, and outcome-tracking artifacts) AND the three-door page live at `docs.maos.dev`,
So that the v0.3 release criterion is met via reproducible human-trial evidence — not a vibe — and we have a real signal on whether the substrate is learnable by people we haven't met.

**Prerequisites resolved per round 2 + round 3:**
- **Story 2.3** (thin cargo-generate slice + local runner) ships at v0.3-α — provides the SDK scaffold participants use
- **Story 8.1** (Butler v0.3 reference Spirit) ships at v0.3 — provides the regression corpus participants run against
- **Story 0.3** (corpus infrastructure + coverage matrix) ships at v0.1 — provides the corpus harness for measurement

**Acceptance Criteria:**

**Given** the recruitment process documented in `docs/research/nfr-onb-1-protocol.md`
**When** participants are recruited for the v0.3 trial
**Then** the cohort of N=12 meets the stratification: ≥4 with no prior MAOS contribution / ≥3 never written a Rust Spirit / ≥2 never written Rust at all / ≥2 non-English-native / ≥1 working offline-only
**And** participant credentials are verified via a screener form committed to `docs/research/nfr-onb-1-screener.md`
**And** the recruitment log lives at `_research/nfr-onb-1/v0.3/recruitment-log.jsonl` (private; not in main repo)

**Given** the 14-day trial window with zero direct-message support
**When** the trial runs
**Then** participants receive only the published documentation (cargo-generate template README + three-door page + Butler reference Spirit code)
**And** all support requests are routed to a public issue tracker (a private DM channel violates the protocol)
**And** any DM-channel breach invalidates the trial and triggers re-recruitment per NFR-Onb-4

**Given** the gate floor
**When** the trial completes
**Then** ≥10 of 12 participants produce a working signed Spirit binary that passes the Butler-class corpus (30-scenario calendar/comms from Story 8.1; halt-recall ≥0.90 on calendar-conflict subset; halt-precision ≥0.85 overall)
**And** time-to-success across the cohort: median ≤45 min, p95 ≤90 min
**And** outcome data is committed to `_research/nfr-onb-1/v0.3/outcomes.jsonl` (private)

**Given** NFR-Onb-4 iteration cadence
**When** the gate misses the floor
**Then** a fresh 6-author cohort runs within 2 weeks
**And** 3 consecutive misses escalate to v0.3 release-criterion review (PRD-author + architecture lead + research lead in the room)

**Given** the three-door page at `docs.maos.dev` (NFR-Onb-3)
**When** the gate runs
**Then** the page hosts three onboarding paths ("write a Spirit" / "run MAOS" / "understand MAOS")
**And** the Spirit-author path links to the cargo-generate template from Story 2.3
**And** the page passes WCAG AA (deferred polish; v1.0 in Story 9.5)

---
