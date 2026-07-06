# Non-Functional Requirements

> **Quality contract.** NFRs specify HOW WELL MAOS must perform — not WHAT it does (Step 9 FRs). Each NFR carries a numeric floor (or structural test) and a phase commitment. Untestable NFRs are unfalsifiable promises and are excluded. **All party-mode amendments from Step 10 round 2 are integrated** (Mary's compliance/operational gaps, Murat's corpus sizing + meta-testing + reproducibility honest revision, Winston's ADR-coverage gaps + tensions + structural-not-semantic clarifications, John's phase moves + cost/reliability/scope additions).

The 13 categories below cover **~85 NFRs** anchoring the substrate's quality contract. Several categories grew during party-mode review: a new **Compliance & Regulatory** category formalizes Mary's jurisdictional gaps, a new **Meta-Testing** category captures Murat's corpus-of-corpora discipline, and **Cost & Tenancy** entries formalize John's substrate-credibility additions.

## Performance

- **NFR-Perf-1:** IAC frame routing latency P50 < 5ms, P99 < 50ms on a typical Linux box (NVMe + 16-core tier). v0.5.
- **NFR-Perf-2:** Sustained IAC frame throughput 5,000–10,000 frames/sec single-host before log writer becomes bottleneck. Per-Spirit fairness scheduler in front of log writer (NOT FIFO). v0.5.
- **NFR-Perf-3:** Capability-token validation latency P99 < 100µs per check; 100% re-validation at use against current state, not cached state (TOCTOU correctness). v0.1.
- **NFR-Perf-4:** Posture-shift propagation P99 ≤ 2s, P99.9 ≤ 5s in 1000-shift corpus. v0.3.
- **NFR-Perf-5:** Audit query latency P99 ≤ 2s for single-Spirit queries on 30-day window; P99 ≤ 10s for global queries. v0.5 (basic), v1.0 (signed-export tier).
- **NFR-Perf-6:** Distillation step latency budget declared per Spirit class via manifest `[budget].time_cap`; soft warning at 80%; kernel emits `BudgetWarning` IAC frame. v0.5.
- **NFR-Perf-7:** Hot-swap latency P99 < 500ms (mode switch + state transfer + capability rebinding) for same-major same-additive swaps. v0.8.
- **NFR-Perf-8** [NEW, polish]: Orchestrator fan-out — sustained 50 concurrent Worker Spirits with task-dispatch latency P99 ≤500ms; 0 dropped tasks under 10 tasks/sec sustained for 1 hour. Backs FR21's fan-out floor. v0.8.

## Reliability

- **NFR-Rel-1:** Spirit-process crash detection ≤ 2s; `task.orphaned` IAC frame ≤ 5s. Floor: ≥99/100 detected within 2s on SIGKILL crash corpus. v0.8.
- **NFR-Rel-2:** Hung-Spirit detection (no-progress IAC for >30s) → `task.stalled` event within 60s. Floor: ≥48/50 reclassified within 60s on hang corpus. v0.8.
- **NFR-Rel-3:** HSIS (Hot-Swap Invariant Suite) ≥ 95% pass per Spirit class; **zero invariant violations** (CVSS-7 class). 6 class-specific corpora at 50 scenarios each; stratified swap-lifecycle phase distribution. v1.0.
- **NFR-Rel-4:** Silent-failure detection. Kernel emits `silent_failure_suspect` event when Spirit emits no progress IAC frames for >30s despite healthy heartbeats. Floor: ≥45/50 detected on adversarial zombie-heartbeat corpus. v1.0.
- **NFR-Rel-5:** Hot-swap rollback within 30s if successor health-check fails. Kernel auto-reverts to predecessor and emits `HotSwapAborted` IAC frame. v1.0.
- **NFR-Rel-6:** Spirit-restart invalidates prior A2A TOFU pins; re-pin protocol with consent confirmation. v1.0.
- **NFR-Rel-7:** A2A trust establishment under churn — 100-host Cortex (or compressed 30-host scale per Murat's cost-compression), 10–20% host turnover/week for 4 weeks, 3 planted adversarial hosts. Floor: detection latency ≤ 1h median, blast radius ≤ 5 peers, recovery ≤ 24h. v2.0 (compressed) / v2.5 (full 100-host). [PHASE-MOVE per John]
- **NFR-Rel-8:** Lifecycle journal durability — fsync per state transition; ring-buffer flush latency < 1ms. v0.1.
- **NFR-Rel-9** [NEW, Mary]: Revocation propagation latency ≤ 5s p99 under 10⁴ concurrent capability-token validations. Closes Winston's "A2A trust establishment under churn" production-risk gap and the weakest leg of the hermes-tenant positioning sentence. v0.8.
- **NFR-Rel-10** [NEW, John]: Kernel cold-restart ≤ 30s with no data loss on graceful shutdown; ≤ 1 in-flight message loss on hard kill. v0.8.
- **NFR-Rel-11** [NEW, Winston]: Halt-receipt production rate ≥ 99.9%. Every Spirit termination, planned or unplanned, produces a halt receipt before process exit. Closes I14 directly (separate from HSIS aggregate). v0.8.

## Security

- **NFR-Sec-1:** Sandbox tier enforced per Spirit; strictest-of-(manifest, trust-tier, operator-policy) floor. v0.1 (T0/T1/T2); v0.5 (T3); v2.0 (T4 WASM).
- **NFR-Sec-2:** Capability-token TTL ≤ 60s for high-privilege operations; bound to Spirit-PID + boot-nonce; audit-logged at every use with origin-Spirit-ID. v1.5 (ADR-023).
- **NFR-Sec-3:** Sandbox-escape **structural** anomaly detection (syscall pattern divergence from manifest declaration, fd-table growth, unexpected outbound IAC connections). **The kernel raises a structural alarm; the *interpretation* of whether the alarm constitutes malice is Spirit-side or operator-side. The kernel does not classify intent.** v2.0 (ADR-024). [STRUCTURAL-NOT-SEMANTIC clarification per Winston] **Scope @v2.0 (Story 11.4b): signal 1 of 3 delivered — seccomp exit-classification-vs-manifest correlation via the out-of-kernel `maos-escape-detector` (`check-escape-detector` gate). fd-table-growth + outbound-IAC deferred to a bounded follow-up (each needs a new raw-fact seam with no live proven-red substrate today). NOT marked fully satisfied — the honesty clause.**
- **NFR-Sec-4:** Pre-write secret-redaction filter at Transparency Log boundary. Floor: **0 secrets across the bounded test populations** — 10⁴-case corpus per-commit (0/10⁴), 10⁵-case quarterly audit (0/10⁵), and production canary system (1000 unique synthetic secrets/month with cryptographic markers; 0 leak per month). Production canary is the falsifiable surrogate for "in any logged frame, ever" — a leak detected via canary halts the distillation pipeline until root-caused. Discovery latency ≤ 24h p95. Any false negative is P0 ship-blocker. v0.5. [TWO-TIER per Murat; sampling bound clarified per polish review]
- **NFR-Sec-5:** Manifest parser fuzz: 24h `cargo-fuzz`, zero crashes/OOMs/infinite loops. v1.0 ship gate.
- **NFR-Sec-6:** Wire protocol adversarial-input fuzz: 24h, zero crashes. v1.0.
- **NFR-Sec-7:** External pen-test report with zero P0/P1 findings open at v1.0 ship. **Triage by joint panel of pen-test lead + MAOS security owner; disagreements escalate to PRD-author tiebreak. P0/P1 definitions per OWASP Risk Rating Methodology, frozen at engagement start.** Pen-tester engagement scheduled 6–8 weeks before v1.0 ship as critical-path dependency. [TIGHTENED per Murat + John]
- **NFR-Sec-8:** Negative-capability assertion via manifest `forbidden_capabilities`; kernel enforces never holding tokens for forbidden surfaces, even transitively via A2A. v1.0.
- **NFR-Sec-9:** Zero `unsafe` blocks in kernel capability-validation path (Rust). v0.1 ship gate.
- **NFR-Sec-10** [AMENDED, Murat]: Adversarial-Spirit red-team **80-scenario** corpus across **8 attack classes** (capability confusion, IAC frame injection, distillation poisoning, ledger tampering, cross-Spirit privilege escalation, resource exhaustion, side-channel timing, kernel-syscall abuse), **N=10 per class**. Floor: **≥9/10 per class** detected/blocked by kernel; ≥72/80 aggregate; **0 unmitigated category** (no class scores 0). Authored by external pen-tester (not MAOS team) using published ABI; pre-frozen corpus, content-addressed. v1.5 [PHASE-MOVE per John, paired with pen-test budget].
- **NFR-Sec-11:** mTLS handshake replay-attack test: 1000 captured handshakes replayed, 0 succeed. v0.5 (loopback) / v1.0 (cross-host).
- **NFR-Sec-12:** TOFU pin-mismatch on second connection: 100% detected, blocked, alerted. v0.5.
- **NFR-Sec-13:** mTLS cert rotation chaos test: 3-host at v1.5; 10-host at v2.0; rotation under load with zero conversation drops; revocation latency median ≤ 60s, p99 ≤ 5min. [PHASE-SPLIT per John]
- **NFR-Sec-14** [NEW, Mary + Winston merged]: **Cross-Spirit memory isolation corpus** — 200-scenario adversarial corpus where Spirit-A actively attempts to enumerate, read, side-channel, or timing-attack Spirit-B's substrate state. Categories: namespace enumeration, working-memory read-across, decision-frame observation, halt-signal observation, transparency-log cross-read, working-memory-digest cross-read, capability-token forgery cross-Spirit, sandbox-escape lateral. Floor: **200/200 isolation maintained**; any leak = P0 ship-block. Defends the v1.0 hermes-tenant positioning sentence. v0.8 (must be in place before the positioning sentence is allowed in marketing).
- **NFR-Sec-15** [NEW, Mary]: Crypto-module pluggability with FIPS 140-3-validated default option. Kernel-internal cryptographic operations (signature verification, sealed-export encryption, capability-token signing) route through a provider trait permitting substitution of FIPS-validated, hardware-backed, or post-quantum implementations without recompilation of Spirits. v1.0.
- **NFR-Sec-16** [NEW, Winston]: Manifest-evolution lint forcing binary `secret`/`non-secret` annotation on every new manifest field — no default. Mitigates structural-vs-semantic redaction tension (Tension A) by shifting cost from runtime detection (forbidden by §4.0.7) to authoring time. v0.5.
- **NFR-Sec-17** [NEW, John]: **Enterprise PDP integration** — capability-authorization decisions sourced from a real external Policy Decision Point behind an out-of-kernel `PolicyDecisionPort` (maos-domain), evaluated by the PDP's real engine (Cedar in-process reference in `maos-pdp`); a deny rule is proven to actually block a capability (policy-swap-flips-verdict + `pdp-fault-inject` falsifier that stubs the engine and reds the deny); fail-closed on PDP unavailability (a configured-but-broken PDP denies all governed capabilities, never relaxes to permissive); absent→BLOCK@v2.0. A security **floor** (not a compliance feature): the org's policy governs capability grants while the kernel stays the small, dumb mediator (ADR-006 / ADR-050). Cross-ref: v2.0 enterprise-deployment roadmap. v2.0.

## Auditability & Compliance

- **NFR-Aud-1:** Capability-contract introspection via `maosctl capability inspect <spirit>`. Returns machine-readable list of declared capabilities, observed capabilities used in last 30d, capability-token issuance count per type. **Log-completeness corpus with N=100 injected events; floor ≥98/100 events recoverable from logs.** v1.0. [TIGHTENED per Murat]
- **NFR-Aud-2:** Drift detection — kernel compares Spirit's **set-membership and frequency-distribution** (capabilities used, tags written, halts emitted) against manifest declarations. **Set-membership and frequency-distribution comparison only — no semantic interpretation. Per §4.0.7, the kernel does not classify whether observed behavior is "suspicious" or "malicious"; it surfaces structural divergence and the operator (or Spirit-side cognition) interprets.** v1.0. [STRUCTURAL-NOT-SEMANTIC clarification per Winston]
- **NFR-Aud-3:** Deterministic replay anchored by ADR-028. Replay determinism is over the **shape of the trace** (IAC frame ordering, capability-token issuances, halt events, decision-frame emission), NOT over redacted payload content. Redacted slots replay as `<REDACTED:type=<class>, len=<bytes>, hash=<sha256-prefix>>` placeholders. v1.0 best-effort; v1.5 hard target.
- **NFR-Aud-4:** Audit retention ≥ 90 days private tier (default); configurable per-deployment; Merkle-root anchoring optional for tamper-evidence. v0.5.
- **NFR-Aud-5:** Right-to-explanation via I12 — 100% of `decision.*` frames carry `working_memory_digest_refs` for explainability replay. EU AI Act adjacent compliance. v0.8.
- **NFR-Aud-6:** Sealed-export Ed25519-signed by operator audit key; third-party-verifiable; conforms to `maos.audit-bundle.v1` schema. **Bundle includes both working-memory digest refs (I12) AND distilled-output content (I11).** v1.0.
- **NFR-Aud-7:** Five-metric distillation gate per distillation-shipping Spirit:
  - Digest-recall ≥ 0.90
  - Digest-faithfulness ≥ 0.98 unflagged contradictions
  - Digest-hedge-preservation ≥ 0.95
  - Digest-traceability = 100% (kernel-enforced via I11)
  - Digest-secret-leakage = 0% (zero-tolerance)
- **NFR-Aud-8** [AMENDED, Murat]: **Two-tier corpus**: N=100 calibration per-commit (CI width 0.124, fine for trend detection) + **N=500 quarterly audit** (CI width ≤0.05 at p=0.90 for digest-recall; tight statistical confidence). Plus 10⁵-case secret-leakage corpus + production canary system per NFR-Sec-4. v0.5 (per-commit), v1.0 (quarterly).
- **NFR-Aud-9** [AMENDED, Murat]: ComplianceClaim Adversarial Corpus (CCAC) v1.0 — N=600 (200 well-formed + 400 malformed). **Per-class N=30, floor ≥ 27/30** (Wilson CI tightened from N=20 vagueness; detects 95% → 70% degradation reliably). 100 context-drift claims (100/100 rejected). Cross-validation across ≥3 reference Spirits, agreement within ±2%. v1.0 ship gate.
- **NFR-Aud-10:** GDPR Article 17 right-to-be-forgotten — 50-scenario corpus with cross-Spirit cascade. Floor: 50/50 clean removal at queryable surface; 50/50 redaction-marker present in immutable log; 0 leakage in 100 follow-up subject-access queries. v1.0.
- **NFR-Aud-11:** SIEM export at v2.0. OpenTelemetry adapter at v1.0.
- **NFR-Aud-12** [NEW, Mary + Winston merged]: **Storage cascade erasure completeness + externally-verifiable uninstall receipt.** Substrate-uninstall produces a portable, externally-verifiable erasure receipt (signed Merkle inclusion + signed Merkle exclusion proof, retained independent of the substrate). 100% of registered storage backends prove erasure within bounded window for any given principal. **Closes the weakest leg of the hermes-tenant positioning sentence.** v1.0.
- **NFR-Aud-13** [NEW, Mary]: Time-to-erasure SLA. Floor: 95% of right-to-be-forgotten requests complete within 30 days (configurable to 7 for enterprise tier); audit log entry within 24h of request acceptance. v1.0.
- **NFR-Aud-14** [NEW, Winston]: Intent-lineage propagation completeness — 100% of cross-Spirit IAC frames carry unbroken lineage chain back to originating principal intent. Closes ADR-018/I13 NFR coverage gap. v0.8.

## Testability

- **NFR-Test-1** [AMENDED, Murat — honest revision]: All ship-gate test corpora are **static artifacts content-addressed in the repo** (SHA-256 of JSONL); generation provenance is documented but not required to be reproducible. Pinned model versions, temperature=0 for judge calls, top_p=1.0, seed where supported, prompt-version hash committed alongside, retry budget=1, quarterly re-baseline with ≥98% agreement on golden snapshot. v1.0.
- **NFR-Test-2:** **Kernel-API surface invariant test (per-commit gate).** Build-time reflection enumerates every kernel API exported to Spirits via `kernel::api::*`; classifies each function by computational class (universal-arithmetic / data-movement / supervision / **other**); **floor: 0 functions in class "other"**; new function entering class "other" is build-break. Static analyzer on Rust `syn` walking allowlist-based predicate definitions; **decidable for permitted subset (no theorem prover)**. **Kernel-utility crate (`kernel::util::*`) has separate looser invariant: no I/O except via injected trait, no global state. The allowlist is the contract; PR-amendment process (not flag) for changes; sign-off from PRD author + tech lead.** Per the §4.0.7 founder principle. v0.1 build gate (surface-diff only); v0.5 adds static analyzer for predicates [PHASE-SPLIT per John].
- **NFR-Test-3:** spirit-test SDK harness coverage ≥ 80% of Spirit author's manifest-declared capabilities reachable via fixtures; validated by external-author trial in 5+ third-party Spirits. v1.0.
- **NFR-Test-4:** Halt-recall ≥ 0.7 / halt-precision ≥ 0.85 per Spirit class on `bmad-eval` standard corpus. v0.5.
- **NFR-Test-5** [PHASE-SPLIT per John]: FKCS (Frozen-Kernel Conformance Suite). **FKCS-infrastructure (diff oracle, test harness, kernel-frozen-vN.0 commit-tagging) at v2.0**; **FKCS-populated (3 future Spirits implemented by external authors) at v2.5** (requires ecosystem of three external authors). Floor: ≥27/30 per Spirit, ≥85/90 aggregate; diff oracle confirms zero kernel changes; negative-control "fourth Spirit" deliberately uses undocumented kernel internal and MUST fail.
- **NFR-Test-6** [AMENDED, Murat]: LCAS (Long-context Ambiguity Stress) corpus — **N=210 scenarios** in 3 buckets (clearly-decidable n=70 / genuinely-ambiguous n=70 / **adversarially-misleading n=70** — raised from 60 for statistical power; Mann-Whitney U at p<0.01 needs ~64 per group at power=0.84). Adversarial trajectories contain a planted load-bearing claim contradicting a louder repeated claim. v0.5 ship gate.
- **NFR-Test-7** [PHASE-MOVE per John]: Cross-form Semantic equivalence (rust-inproc ↔ subprocess) ≥ 90%; (any-rust ↔ wasm-component) ≥ 75%. CLI-wrapper requires distributional behavioral equivalence (Mann-Whitney U-test p > 0.05 over 30 runs). **v1.5** (rust↔subprocess; cohort interop at v1.0 is rust-rust); **v2.0** (any-rust↔wasm).
- **NFR-Test-8** [AMENDED, Murat]: **Black-box third-party trial v1.0 — N=12 stratified** (≥4 with no prior MAOS contribution; ≥3 who've never written Rust Spirit; ≥2 who've never written Rust at all; ≥2 non-English-native; ≥1 working offline-only). 14-day no-DM-support window. Floor: **≥10/12 produce working signed Spirit binary** that loads on fresh Host VM, runs ≥1000 frames, halt-recall ≥0.85. Wilson CI [0.552, 0.953] meaningful at N=12; meaningless at N=5. Auditable via SBOM + signing chain re-loaded on clean VM by CI bot. **Run only at major releases (v1.0, v2.0); minor releases use NFR-Onb-1 (12-author onboarding) as proxy** [COST-COMPRESSION per Murat].
- **NFR-Test-9** [NEW, Winston]: Loom-not-in-kernel structural test. `grep` of kernel crate for orchestration/planning symbols returns ∅. Per-commit gate. Covers ADR-006's negative commitment (Loom is user-space). v0.5.
- **NFR-Test-10** [NEW, Winston]: Skill-format conformance — at least one third-party skill format (Anthropic Skills format OR equivalent) executes via Spirit-form adapter without kernel modification. Covers ADR-027's external-standard interop assertion empirically. v1.5.
- **NFR-Test-11** [NEW, Mary]: Namespace grammar lock test. Grammar `.lark` (or equivalent) hash pinned in CI; any change requires architecture-lock review process, not regular PR. v0.5.
- **NFR-Test-12** [NEW, Mary]: **v0.3 architecture lock script as per-commit gate.** `scripts/check_v0_3_lock.sh` runs four mechanical checks: (1) `LICENSE` matches ADR-decided license string; (2) consortium-target ADR exists with status `accepted` and ≥2 maintainer sign-offs; (3) `ROADMAP.md` has trust-anchor decision section with status `decided` linking to ADR; (4) failure-semantics doc exists with at least one fully-specified route. **No v0.3 tag without script in green.** v0.3.
- **NFR-Test-13:** Manifest field test coverage ≥ 3 cases per field (well-formed, malformed-rejected, edge-case); CI-enforced. v0.1.
- **NFR-Test-14:** Wire protocol cross-language byte-equal golden corpus per frame variant per SDK (Rust + TS v0.5 + Python v1.0 + Go v1.5+). v1.0.

## Meta-Testing [NEW CATEGORY, Murat]

- **NFR-Meta-1:** **Corpus-quality audit.** Each ship-gate corpus reviewed by independent assessor (not corpus author) on a 10-point rubric (representativeness, edge-case coverage, label correctness, distribution match to production). Floor: ≥8/10 per corpus. Cadence: at corpus creation + every 12 months. v1.0.
- **NFR-Meta-2:** **Corpus-staleness.** Every corpus carries a `valid_until` date in metadata. CI fails if any active gate references an expired corpus. Default validity: 12 months. Extension requires explicit "no-update justification" PR with assessor sign-off. v1.0.
- **NFR-Meta-3:** **Coverage matrix.** Single source-of-truth file `tests/coverage-matrix.yaml` mapping {FR, NFR} → {corpora, gates}. CI fails if any FR/NFR with phase-status `delivered ≤ current-phase` has zero corpus coverage. Generated report surfaces gaps automatically and explicitly labels deferred FR/NFRs (e.g., FR37 deferred-to-v2.5; FR60 v1.5; v2.0+ NFRs) as out-of-scope for the current phase's gate. Floor at v1.0: 100% coverage of FRs/NFRs delivered ≤ v1.0 (≈ FR1–FR36, FR38–FR53, FR55–FR59, FR61–FR65 + ~70 of ~85 NFRs); deferred items tracked but not gated. v1.0.

## Observability

- **NFR-Obs-1:** Author-observability contract — Spirit author can read same diagnostic surface as operator for their own Spirit, redacted of cross-Spirit data. **Metric M is queryable in <500ms with cardinality ≤10⁴.** v1.0. [TIGHTENED per Murat]
- **NFR-Obs-2:** OpenTelemetry export per IAC frame, capability invocation, halt event. v0.5 basic; v1.0 SLO-class.
- **NFR-Obs-3:** Per-Spirit telemetry stream with topic-based broadcast + filtered subscription. v0.3 (Butler narrow); v0.5 (Observer broad).
- **NFR-Obs-4:** Transparency Log per-Host SQLite (append-only), exportable to JSONL/SIEM with redaction policy applied. v0.5.
- **NFR-Obs-5:** Approval Decision Log distinct from Transparency Log; full intent + decision + reasoning chain per Invariant I4. v0.3.

## Documentation Quality

- **NFR-Doc-1:** Every public ABI method has ≥ 1 doctested example; CI broken-link blocking on doc site **at v0.5 (when doc site lands)**; doctest CI gate at v0.1 [PHASE-SPLIT per John].
- **NFR-Doc-2:** Typed error catalog at `https://docs.maos.dev/errors/<ERR_NAME>` covering all 14+ named typed errors. **CI-enforced metadata: each variant has 6 fields (code, severity, recovery-class, owner, kernel-or-spirit, since-version). CI runs `cargo run --bin error-metadata-check` which exits non-zero if any variant is missing any field; each field has its own assertion.** v1.0. [TIGHTENED per Murat]
- **NFR-Doc-3:** API reference site at `https://docs.maos.dev/abi/<version>/`; versioned, searchable, deep-linkable, archived ≥ 2 minor versions back. v1.0.
- **NFR-Doc-4:** Five canonical doc deliverables published with concrete URL paths and CI-verifiable minima:
  - **Manifest schema reference** at `https://docs.maos.dev/manifest/<version>/` — one entry per declared field rendered from the JSON schema; **floor: ≥1 example per field**.
  - **Pattern cookbook** at `https://docs.maos.dev/cookbook/` covering lifecycle / IAC / capability / halt / hot-swap / cross-Spirit / distillation / memory / ecosystem / audit; **floor: ≥10 patterns**.
  - **Migration runbooks** at `https://docs.maos.dev/migrate/` — Path A (in-major) + Path B (cross-major); each runbook has a rollback section.
  - **Troubleshooting guide** at `https://docs.maos.dev/troubleshoot/` keyed by typed error code (per FR63 catalog); **floor: every typed error in the FR63 catalog has a troubleshooting entry**.
  - **Deployment topology guide** at `https://docs.maos.dev/deploy/` covering single-host / team-mesh / cross-host / Cortex topologies.
  - **CI gate:** each URL renders 200 (broken-link blocker per NFR-Doc-1); pattern-cookbook entry count ≥10; troubleshooting entries cover 100% of FR63 catalog. v1.0.
- **NFR-Doc-5:** WCAG AA compliance for doc site. v1.0.
- **NFR-Doc-6** [PHASE-MOVE per John]: Localization v1.0 = **Korean only** (shipped); Japanese + Chinese-simplified at v1.5. `LOCALES.md` with glossary lock — terms NEVER translated: Spirit, Worker, kernel, ADR identifiers, error codes.
- **NFR-Doc-7** [PHASE-MOVE per John]: Doc tooling supports per-locale builds + fallback to English + language switcher with deep-link preservation + version dropdown. **RTL layout support deferred to v2.5** (no RTL locale targeted before v2.5). Pick mdBook + i18n / Docusaurus / VitePress by v0.5; v1.0 in production.

## Onboarding

- **NFR-Onb-1:** **30-Min First Spirit Validation Gate.** N=12 stratified external Spirit authors (≥4 with no prior MAOS contribution; ≥3 who've never written Rust Spirit; ≥2 who've never written Rust at all; ≥2 non-English-native; ≥1 working offline-only). Floor: median ≤ 45 min, p95 ≤ 90 min, AND **≥ 10/12 succeed where "succeed" = author produces Spirit binary that (a) compiles against published ABI, (b) passes the v0.3-grade Butler-class regression corpus (30-scenario calendar/comms; halt-recall ≥0.90 on calendar-conflict subset; halt-precision ≥0.85 overall — same corpus the reference Butler is gated on), (c) does so within 14 calendar days from kit handoff with zero direct-message support; forum/docs questions allowed and logged.** v0.3 release criterion. **NOTE:** the FKCS conformance suite (NFR-Test-5) is a *kernel-freeze validation harness* shipping at v2.0/v2.5 — distinct artifact. NFR-Onb-1 measures Spirit-author velocity against the v0.3-grade reference corpus; rerun against richer corpora at v0.5 (Researcher) and v1.0 (any reference Spirit class). [TIGHTENED per Murat]
- **NFR-Onb-2:** First-time installer J0 evaluator path — install + first useful Spirit response within 5 minutes. v0.1.
- **NFR-Onb-3:** Three-door page at `docs.maos.dev` ("write a Spirit" / "run MAOS" / "understand MAOS"). v0.5.
- **NFR-Onb-4** [NEW, Mary]: 30-Min Gate iteration cadence. If floor missed, run fresh 6-author cohort within 2 weeks; three consecutive misses escalate to v0.3 release-criterion review. Operational commitment, not one-shot gate. v0.3.

## Maintainability

- **NFR-Maint-1:** **Kernel trusted core ≤ 20 KLOC excluding tests through v2.0** (core scheduler + IAC bus + capability check + journal). Integration adapters in separate crates with their own LOC budgets. v2.0.
- **NFR-Maint-2** [PHASE-SPLIT per John]: Capability-registry fuzz coverage **≥60% line at v0.1; ≥80% line / ≥60% branch at v0.5** on 1M-iteration libFuzzer run; zero crashes.
- **NFR-Maint-3:** ABI compatibility matrix 100% within current major; 100% N-1 boundary including negative typed-error cases. v0.1 (within-major); v1.0 (N-1).
- **NFR-Maint-4:** STABILITY.md publishes live (kernel_version, abi_version, manifest_schema_version) compatibility matrix. v1.0.
- **NFR-Maint-5:** Deprecation timeline: 2 minor releases of warning, 1 major release to remove. v1.0.
- **NFR-Maint-6** [PHASE-SPLIT per John]: **1-year LTS commitment at v1.0**; 2-year LTS commitment at v1.5 once support load is known; security-only patches after year 1. Don't write a check the v0.8 team can't cash.
- **NFR-Maint-7** [PHASE-MOVE per John]: BREAKING.md required entry for every breaking change with migration steps; CI grep-enforced. **v1.0** (you don't break things until you have stable surface to break).
- **NFR-Maint-8:** Capability-token TOCTOU test: 100% re-validation at use against current state. v1.0.
- **NFR-Maint-9** [NEW, Winston]: Manifest schema N-1 compatibility — kernel version V can load manifests written for V-1 with documented degradation paths. Closes ADR-025 NFR-coverage gap. v1.0.

## Scalability

- **NFR-Scale-1:** Cortex 3-region pilot at v2.0 with ≥ 10 agents minimum; sustained operation for 30 days; zero substrate-invariant violations.
- **NFR-Scale-1-SLO** [NEW, Story 11.2b, John — bifurcated]: **Cross-region propagation latency SLO, split into two independently-bound portions.**
  - **(a) In-tree / CI-bound (BINDING at v2.0 via `check-multi-region-slo`):** the cross-region round-trip *machinery + convergence + regression floor* — a **single-clock A→B→A round-trip** ("cross-region round-trip (network + remote-service)", NOT "network RTT") measured on the J4 percentile engine against `MULTI_REGION_SLO_P95_US = 30_000µs` (loopback-calibrated regression floor, **NOT a geo-SLO** — CI Postgres is co-located, so a real geo-RTT is physically unobservable). This portion is mechanically-falsifiable and ship-blocking at v2.0; its teeth come from the `slo-fault-inject` falsifier (injects 15ms into the measured span → p95 crosses the floor).
  - **(b) Geo-operational / release-gate (DEFERRED — "validated in pilot", NOT CI-bound):** the **absolute geo-SLO** (a wall-clock cross-region latency figure measured across genuinely geo-distributed regions) + **sustained live operation** + the **30-day soak** are release-gate pilot artifacts, validated in the separately-tracked live geo pilot — they MUST NOT be claimed as "validated in CI" (the gate cannot observe San-Francisco↔Frankfurt latency on a co-located runner). Absent/unmeasured → BLOCK@v2.0.
- **NFR-Scale-1-Read** [NEW, Story 11.2b, Decision-11 §9]: Fail-closed region-identity on the LIVE collective read path (NFR-Comp-4 "no transparent replication" enforced on READ as well as write) — a foreign-region row that was not validly re-attested is **refused, never served**, via a store-internal `LoomLiteStore::region_guard` below the `CollectiveMemoryPort` (ZERO kernel-Δ). BINDING at v2.0 via `check-multi-region-slo` (live-read-region-identity leg).
- **NFR-Scale-2** [PHASE-SPLIT per John + cost-compression per Murat]: **25-host churn test at v2.0; 100-host churn at v2.5** (cost compression: 100→30 hosts at v2.0 same churn-events-per-week, full 100-host moves to v2.5).
- **NFR-Scale-3:** Per-Spirit fairness scheduler in front of log writer (NOT FIFO). **Algorithm: Deficit Round Robin (DRR) with per-Spirit weight=1 by default; operator-configurable weights via `[scheduler.weights]` in the operator policy file.** **Floor:** under uneven load (1 noisy Spirit at 10× the median write rate alongside ≥4 normal Spirits sustained for 60s), the max-min P99 latency ratio across Spirits ≤ 3.0 (no single healthy Spirit's writes blocked more than 3× the slowest healthy Spirit's P99). v0.5.
- **NFR-Scale-4:** Provider rate-limit isolation — per-(provider, credential) token bucket; typed `RateLimited` IAC frame. v0.5.
- **NFR-Scale-5:** Multi-host A2A peer mesh scales to 14-institution Cortex; v2.0 target with documented capacity envelope.

## Operational

- **NFR-Ops-1:** Substrate operations checklist fully delivered: install, upgrade, yank, uninstall, revoke. v0.1 (install/uninstall) → v0.5 (upgrade/yank) → v1.0 (revoke).
- **NFR-Ops-2:** Signed Revocation List (CRL) artifact; registry-pushed (kernel polls every 5min) + offline-import path. v1.0.
- **NFR-Ops-3:** Telemetry opt-in default; `PRIVACY.md` with retention, jurisdiction, deletion path; per-field redaction layer. v1.0.
- **NFR-Ops-4:** **`SECURITY.md`** with disclosure address (`security@maos.dev`), GPG key, embargo window (90-day default), advisory-publication channel, supported-versions matrix. **v0.1 ship gate.** **CNA registration through MITRE moves to v0.5** (6–12 weeks elapsed paperwork; v0.1 just needs disclosure pipeline to exist). [PHASE-SPLIT per John]
- **NFR-Ops-5:** maosctl `--plain` flag + `NO_COLOR` + `TERM=dumb` accessibility. v0.1.
- **NFR-Ops-6** [PHASE-MOVES per John]: Onboarding artifacts — `RFC_TEMPLATE.md` at **v0.8** (was v0.5), `GOVERNANCE.md` at v0.5 (basic) + v0.8 (locked), `CODE_OF_CONDUCT.md` at v0.5, `LOCALES.md` at **v1.0** (was v0.5), `TRADEMARK.md` at **v1.0** (was v0.5), `BREAKING.md` at **v1.0** (was v0.5; matches NFR-Maint-7).
- **NFR-Ops-7** [PHASE-MOVE per John]: Sustainability vehicle — declared-intent at v0.5 (Open Collective open, accepting $0 expected); **legal/fiscal-sponsor work at v0.8**.
- **NFR-Ops-8** [NEW, Mary]: Trust-anchor framing carry-forward decision. Published ADR by v0.3 declaring which competitive framing is committed (substrate-as-substrate vs substrate-as-trust-anchor); absence = v0.3 release-block. v0.3.
- **NFR-Ops-9** [NEW, Mary]: Transparency Log backup/DR. RPO ≤ 1h, RTO ≤ 4h, backup integrity verified weekly via Merkle-root cross-check. v1.0.
- **NFR-Ops-10** [NEW, Mary]: Database migration test corpus. SQLite→Postgres at v1.5 (committed in roadmap). Floor: forward-migration test on 10⁶-row corpus, byte-identical Merkle-root preservation post-migration, rollback path tested. v1.4 (gates v1.5).
- **NFR-Ops-11** [NEW, Mary]: Multi-operator tenancy isolation — primitive-reservation only at v1.0 (declared as primitive-reserved in namespace grammar so v0.5 grammar lock doesn't paint us into a corner; full implementation v1.5+). Per-operator namespace, per-operator transparency-log shard, per-operator capability-token signing key, per-operator GDPR-erasure scope. v1.0 (reserved); v1.5+ (implemented).
- **NFR-Ops-12** [NEW, Mary]: Air-gapped deployment validation. Substrate boots, runs, produces transparency-log entries with zero outbound network calls; structural test in CI via network-namespace isolation; documented Spirit-author guidance for air-gapped capability tokens. v1.0.

## Compliance & Regulatory [NEW CATEGORY, Mary]

- **NFR-Comp-1:** Export-control classification artifact. ECCN classification letter on file, EAR99 vs 5D002 determination published in `STABILITY.md §Export`, dual-use review for crypto primitives in kernel. v0.8 (before any v1.0 enterprise-distribution conversation).
- **NFR-Comp-2:** Vetter accreditation parameters — published vetter qualification matrix (cryptography review credential OR 5+ years agentic-security review OR equivalent), conflict-of-interest disclosure required, vetter rotation policy (no single vetter on >40% of Spirit-class promotions in any 12-month window), vetter audit-trail retained 7 years. **v1.0** (aligned with vetter trust model documentation; FR37 vetting issuance is v2.5 per phase-defer). [PHASE-MOVED from v0.8 per polish review — documenting accreditation 4 phases before any accredited issuer is dead documentation.]
- **NFR-Comp-3:** Substrate-self compliance scope declaration. `STABILITY.md` contains scope-disclaimer paragraph explicitly stating SOC 2 / ISO 27001 / FedRAMP scope is the *operator's* responsibility, not the substrate's, with kernel-as-service boundary drawn. **Structural test that the four named regimes appear with disclaimer; failure = ship-block.** v0.5.
- **NFR-Comp-4:** Region-pinning primitive (PIPL §40 / data localization). Transparency Log + working-memory store configurable to single jurisdictional region with cryptographic enforcement against cross-region replication. Without primitive, enterprise distribution cannot configure for PIPL. v1.0.
- **NFR-Comp-5:** Spirit model-provenance manifest field (SB-1047 / Colorado AI Act adjacent). Manifest declares covered-model identifier, training-data lineage, last-eval timestamp; substrate validates field presence at admission. v1.0.

## Cost & Tenancy [NEW CATEGORY, John]

- **NFR-Cost-1:** Cost-attribution accuracy ≥ 98% reconciliation against provider billing, sampled monthly. Per-Spirit per-task per-principal attribution. **Without this NFR, FR64 (cost accounting) is theater.** v1.0.
- **NFR-Tenancy-1:** Explicit single-tenant per kernel instance commitment through v2.0; multi-tenant primitive-reserved at v1.0 per NFR-Ops-11; full multi-tenant out of scope before v2.5. **Make the boundary loud** (avoids hidden-multi-tenancy assumptions in design reviews). v0.1 (declared); v2.0 (single-tenant guaranteed).

---

## NFR-to-architecture traceability (updated; ~85 NFRs across 28 ADRs and 14 invariants)

| Category | Anchors | Phase distribution |
|---|---|---|
| Performance (8) | ADR-001/010/011; I2 | v0.1, v0.3, v0.5, v0.8 |
| Reliability (11) | I6/I10; ADR-017/019/020/022 | v0.1, v0.8 (heavy), v1.0, v2.0/v2.5 |
| Security (16) | I1/I9; ADR-009/012/023/024 | v0.1, v0.5 (heavy), v1.0 (heavy), v1.5, v2.0 |
| Auditability & Compliance (14) | I2/I11/I12/I13; ADR-013/015/023/026/028 | v0.5, v0.8, v1.0 (heavy) |
| Testability (14) | All ADRs; §4.0.7 | v0.1, v0.3, v0.5, v1.0 (heavy), v1.5, v2.0/v2.5 |
| Meta-Testing (3) | Cross-cutting | v1.0 |
| Observability (5) | I7/I4; ADR-013 | v0.3, v0.5, v1.0 |
| Documentation Quality (7) | Step 7 commitments | v0.1, v0.5, v1.0, v1.5, v2.5 |
| Onboarding (4) | Step 7+8 commitments | v0.1, v0.3, v0.5 |
| Maintainability (9) | ABI triple §14 #4; STABILITY.md | v0.1, v0.5, v1.0, v1.5 |
| Scalability (5) | ADR-006; A2A peer mesh | v0.5, v2.0, v2.5 |
| Operational (12) | Step 7 substrate operations | **v0.1 (NFR-Ops-4 SECURITY.md, NFR-Ops-5 accessibility)**, v0.3, v0.5, v0.8, v1.0 |
| Compliance & Regulatory (5) | EU AI Act, NIS2, PIPL, SB-1047 | v0.5, v0.8, v1.0 |
| Cost & Tenancy (2) | FR64 anchor | v1.0 |

**~85 NFRs total.** Each carries a numeric floor (or structural test) and a phase commitment. Each will need a CI gate, test corpus, or operational artifact during implementation.

---

## NFR ship-gate consolidation by phase (post-Tier-3)

The most contested ship gates by phase, after John's rebalancing:

**v0.1 (foundational, ~6–8 weeks for one founder):** SECURITY.md basic (NFR-Ops-4, **CNA registration deferred to v0.5**), kernel-API surface invariant test surface-diff-only (NFR-Test-2, **static analyzer deferred to v0.5**), ABI matrix within-major (NFR-Maint-3), capability-registry fuzz **≥60% line** at v0.1 (NFR-Maint-2, **≥80% deferred to v0.5**), zero `unsafe` in capability-validation (NFR-Sec-9), manifest field test coverage (NFR-Test-13), accessibility flags (NFR-Ops-5), doctest CI gate (NFR-Doc-1, **broken-link blocker deferred to v0.5**), J0 evaluator path (NFR-Onb-2), lifecycle journal durability (NFR-Rel-8), capability-token TOCTOU (NFR-Perf-3 + NFR-Maint-8), explicit single-tenant commitment (NFR-Tenancy-1).

**v0.3 (Butler):** 30-Min First Spirit Validation Gate (NFR-Onb-1) + iteration cadence (NFR-Onb-4) — v0.3 release criterion. Posture-shift latency (NFR-Perf-4). v0.3 architecture lock script per-commit gate (NFR-Test-12). Trust-anchor framing carry-forward (NFR-Ops-8).

**v0.5 (Researcher + Observer):** LCAS test corpus N=210 (NFR-Test-6). Five-metric distillation gate baseline (NFR-Aud-7..8). Pre-write secret redaction with two-tier corpus (NFR-Sec-4). IAC routing latency (NFR-Perf-1, NFR-Perf-2). Halt-recall/precision per Spirit class (NFR-Test-4). Substrate-self compliance scope clause (NFR-Comp-3). Manifest-evolution lint (NFR-Sec-16). Loom-not-in-kernel structural test (NFR-Test-9). Namespace grammar lock (NFR-Test-11). Static-analyzer-for-predicates upgrade (NFR-Test-2 follow-on). 

**v0.8 (Founder Loop wedge demo):** Crash detection + hung-Spirit detection (NFR-Rel-1, NFR-Rel-2). Hot-swap latency (NFR-Perf-7). Right-to-explanation via I12 (NFR-Aud-5). **Cross-Spirit isolation corpus (NFR-Sec-14) — must be in place before positioning sentence allowed in marketing.** Revocation propagation latency (NFR-Rel-9). Kernel cold-restart (NFR-Rel-10). Halt-receipt production rate (NFR-Rel-11). Intent-lineage propagation (NFR-Aud-14). Export-control ECCN (NFR-Comp-1). Vetter accreditation parameters (NFR-Comp-2).

**v1.0 (Team-ready):** HSIS (NFR-Rel-3). CCAC N=600 (NFR-Aud-9). Black-box third-party trial N=12 (NFR-Test-8). Manifest fuzz + wire fuzz (NFR-Sec-5, NFR-Sec-6). External pen-test (NFR-Sec-7). Typed error catalog (NFR-Doc-2). **1-year LTS announcement** (NFR-Maint-6, **2-year deferred to v1.5**). GDPR right-to-be-forgotten (NFR-Aud-10). Cascade erasure receipt (NFR-Aud-12). Time-to-erasure SLA (NFR-Aud-13). Storage cascade completeness (NFR-Aud-12). Cost-attribution accuracy (NFR-Cost-1). Region-pinning primitive (NFR-Comp-4). Spirit model-provenance manifest (NFR-Comp-5). Crypto-module pluggability (NFR-Sec-15). TX log backup/DR (NFR-Ops-9). Air-gapped deployment validation (NFR-Ops-12). Multi-operator primitive-reservation (NFR-Ops-11). Three Meta-NFRs (corpus quality, staleness, coverage matrix). Manifest schema N-1 compat (NFR-Maint-9).

**v1.5 (Mira-Nash):** Capability-token TTL + bind-to-PID (NFR-Sec-2). 3-host mTLS cert rotation chaos test (NFR-Sec-13). Adversarial-Spirit red-team (NFR-Sec-10) — paired with pen-test budget. Cross-form rust↔subprocess equivalence (NFR-Test-7). Skill-format conformance (NFR-Test-10). Deterministic replay hard target (NFR-Aud-3). 2-year LTS commitment (NFR-Maint-6 follow-on). Localization JA + ZH (NFR-Doc-6 follow-on). DB migration test (NFR-Ops-10).

**v2.0 (technical):** FKCS-infrastructure (NFR-Test-5 first half). 25-host Cortex churn test (NFR-Scale-2 first half). 10-host mTLS chaos (NFR-Sec-13 follow-on). Sandbox-escape detection (NFR-Sec-3). SIEM export (NFR-Aud-11). Cross-form any-rust↔wasm (NFR-Test-7 follow-on).

**v2.5 (ecosystem):** FKCS-populated (NFR-Test-5 second half). 100-host Cortex churn (NFR-Scale-2 second half). RTL layout support (NFR-Doc-7 follow-on). Vetter ecosystem maturity. Multi-operator full implementation (NFR-Ops-11 follow-on).

These gates are the substrate's quality contract. They are non-negotiable at the named phase or the phase doesn't ship.
