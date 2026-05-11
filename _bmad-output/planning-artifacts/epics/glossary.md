# Glossary

A dev agent picking up any story should be able to resolve every term below without leaving this document. Definitions cross-reference the PRD, architecture-maos-minimal-opus.md, and the first story where the term becomes load-bearing.

| Term | Expansion / One-line definition | First load-bearing in |
|---|---|---|
| **A2A** | Agent-to-Agent peer mesh — loopback at v0.8 (127.0.0.1 mTLS+TOFU); cross-Host at v1.0 (operator-managed PKI + ADR-012 typed-intent consent) | Story 6.3 |
| **ABI Stability Triple** | `(kernel_version, abi_version, manifest_schema_version)` — three independently-evolving versions governing kernel/Spirit compatibility; N-1 supported / N-2 hard refusal | Story 7.5 |
| **ACP** | Agent Control Protocol — NDJSON over stdio for editor-hosted Spirits (Zed + VSCode at v1.0; JetBrains plugin-bridge at v1.5) | Story 5.5c |
| **bmad-eval** | The standard halt-recall/halt-precision evaluation corpus. v0.3 = synthetic 50-scenario corpus in `crates/maos-eval/fixtures/halt-corpus-v0/`; v1.0 = E8-authored reference-Spirit corpora replace it | Story 4.1 |
| **BREAKING.md** | Grep-enforced file at repo root; every breaking change requires a dated entry with migration steps (NFR-Maint-7) | Story 7.5a |
| **CCAC** | ComplianceClaim Adversarial Corpus — v1.0 ship-gate corpus N=600 (200 well-formed + 400 malformed via parameterized generator); per-class floor ≥27/30; cross-Spirit agreement ±2% (NFR-Aud-9) | Story 7.3 |
| **ComplianceClaim envelope** | Ed25519-signed first-class object binding (manifest hash + version + trust tier + sandbox tier + capability scope + provider-endpoint + crypto-provider) to a compliance attestation; kernel verifies at admission with `EComplianceContextDrift` | Story 7.3 |
| **ComplianceClaim schema** | The wire-stable type definitions in `crates/maos-spirit-abi/src/compliance.rs`; FROZEN at Story 1b.4 after E0 adversarial review (Story 0.4); ABI-break required to change | Story 0.4 |
| **CRL** | Signed Revocation List artifact; registry-pushed (kernel polls every 5min) + offline-import path (FR13) | Story 5.4 |
| **DRR** | Deficit Round Robin fairness scheduler in front of the log writer; per-Spirit weight=1 default; operator-configurable `[scheduler.weights]`; max-min P99 latency ratio ≤3.0 under uneven load (NFR-Scale-3) | Story 6.1 |
| **Director** | The human end-user of MAOS — assigns tasks, resolves halts (3-pathway), shifts posture, instant pause/resume/revoke. Stakeholder for E3 stories | Story 3.1 |
| **Director's-surface metaphor** | Theater/actor/director framing — Spirits are actors, kernel is the stage, Director controls the production with bounded-time intervention (FR51) | Story 3.4 |
| **Evaluator** | First-time MAOS user; the 5-minute install-to-hello-Spirit-response stakeholder (NFR-Onb-2). Distinct from operator/director/Spirit author | Story 1b.5a |
| **HSIS** | Hot-Swap Invariant Suite — 6 Spirit-class-specific corpora × 50 scenarios = 300 scenarios; ≥95% pass per class; zero invariant violations CVSS-7 class (NFR-Rel-3) | Story 4.5 |
| **I1–I14** | The 14 architectural invariants codified in `crates/maos-domain/src/invariants/`. I1 = total capability mediation; I2 = log-before-deliver; I4 = Approval Decision Log distinct from Transparency Log; I5 = memory tier namespace isolation; I9 = empty-kernel; I10 = lifecycle journal durability; I11 = digest audit chain; I12 = working-memory digest refs; I13 = intent-lineage propagation; I14 = halt continuity across hot-swap | Story 1a.1 |
| **IAC bus** | Inter-Actor Communication bus — kernel-internal mailbox + broadcast + `retract` channels routing frames by SpiritId; log-before-deliver guarantee I2 | Story 3.1 |
| **J0 / J1 / J4 / J6** | Per-journey latency budgets from architecture §13.1. J0 = Butler conversational <400ms P95 / IPC <60ms; J1 = founder-loop CliWrapper IPC <25ms P95; J4 = Mira-Nash Observer colocation <10ms P95; J6 = Diego cold-start <500ms | Story 1b.5a |
| **KLOC budget** | Kernel trusted core ≤20 KLOC excluding tests through v2.0; `tokei` alarm at 16 (NFR-Maint-1); enforced per-merge via E0 | Story 0.1 |
| **LCAS** | Long-context Ambiguity Stress corpus — N=210 in 3 buckets (70 clearly-decidable v0.3 in Story 2.4 / 70 genuinely-ambiguous post-A2A in Story 7.4 / 70 adversarially-misleading post-A2A in Story 7.4); planted load-bearing claim contradicting louder repeated claim | Story 2.4 |
| **MCP** | Model Context Protocol — kernel-hosted tool-server interface with three transports (stdio / SSE / Streamable HTTP); Streamable HTTP default at v0.5 | Story 5.5c |
| **NFR-Onb-1** | 30-Min First Spirit Validation Gate — N=12 stratified authors, ≥10/12 succeed in 14 days zero-DM-support against Butler-class corpus; **v0.3 release criterion** despite gate-execution living in E7 | Story 7.5b |
| **NFR-Sec-7** | External pen-test report with zero P0/P1 findings at v1.0 ship; OWASP Risk Rating Methodology frozen at engagement start | Story 10.1 |
| **NFR-Aud-9** | CCAC ship gate at v1.0 — see CCAC entry | Story 7.3 |
| **NFR-Rel-3** | HSIS ship gate at v1.0 — see HSIS entry | Story 5.2 |
| **Operator** | Substrate deployment owner — configures providers, sandbox tiers, audit policy, GDPR forget cascades, distribution channels. Distinct from Director (end-user) and Spirit Author (developer) | Story 5.5a |
| **Principal namespace** | Typed namespace `principal:<principal_id>:<schema>` within Spirit's private memory tier (ADR-026); auto-eligible for subject-access, GDPR Art. 17 cascade, redaction-on-export | Story 4.3 |
| **§13.1 measurement** | Subprocess vs rust-inproc latency measurement story in E5; go/no-go gate before v0.5 ships — if subprocess meets J1+J4 budgets, rust-inproc form deferred to v2.0+; else rust-inproc unlocks with NFR-Test-7 cross-form equivalence at v1.5 | Story 5.5e |
| **scalar.tap** | Dedicated read-only stream from Capability Registry's tagged-scalar slot (ADR-035 binding-v0.5); every `working_memory.set_scalar(tag, value, derived_from)` write emits `(spirit_id, tag, value, timestamp)`; Observer Spirits subscribe to detect pre-halt drift | Story 4.2 |
| **Spirit** | LLM-backed agent hosted by the MAOS kernel; subprocess form at v0.1 (ADR-002); rust-inproc form gated on §13.1 | Story 1a.1 |
| **Spirit Author** | Developer writing Spirit code; primary stakeholder for E2 + E7 ecosystem stories | Story 2.1 |
| **Spirit class** | Typed category of Spirit (e.g., Butler / Researcher / Observer / Orchestrator / Worker / CliWrapper / Architect / Reviewer / Mira / Nash); halt-recall/precision floors measured per class | Story 4.1 |
| **Spirit form** | Spirit execution model — `subprocess` (v0.1; OS-process-isolated, LSP-style wire protocol over stdio) vs `rust-inproc` (gated on §13.1; same-process Rust dynamic library) | Story 1b.5a |
| **T0 / T1 / T2 / T3 / T4** | Sandbox tier ladder (ADR-004): T0 = no sandbox (trusted local); T1 = process isolation + UID separation; T2 = Linux Landlock+seccomp / macOS Seatbelt / Windows restricted-token; T3 = T2 + container (Docker/Podman, v0.5); T4 = WASM (v2.0 deferred). Strictest-of-(manifest, trust-tier, operator-policy) floor applies | Story 1b.3 |
| **three-door page** | `docs.maos.dev` landing page with three onboarding paths ("write a Spirit" / "run MAOS" / "understand MAOS"); NFR-Onb-3 | Story 7.5b |
| **TOFU pinning** | Trust-on-First-Use mTLS certificate pinning for A2A loopback at v0.8; second-connection pin-mismatch triggers 100% detect/block/alert (NFR-Sec-12); restart invalidates prior pins with re-pin consent confirmation (NFR-Rel-6) | Story 6.3 |
| **Transparency Log** | Per-Host SQLite append-only log of every IAC frame, approval, capability use, retract event; the personal audit trail. Distinct from `tracing` (debug) and the Telemetry Stream (typed broadcast events) | Story 1b.1 |
