# Dependency DAG

Story-level dependency graph (cross-epic only — intra-epic ordering covered per-epic). Forward dependencies must be resolved by either (a) ordering the dependency before the dependent in sprint plan, or (b) a documented stub interface (`MockHaltResolver`-style pattern).

```
                    E0 Quality Substrate
                    ├──→ ALL EPICS (CI gates run on every PR)
                    └──→ Story 0.4 ComplianceClaim adversarial review BLOCKS Story 1b.4 schema freeze

E1a Workspace Bootstrap + Skeleton
├──→ E1b (workspace + ABI types must exist)
├──→ E2 (Spirit ABI types must exist)
└──→ Story 1a.1 → ALL: starter-template flag

E1b Evaluator Path + Audit Spine
├──→ E2 (manifest schema frozen)
├──→ E3 (IAC bus skeleton, Approval Decision Log, Transparency Log)
├──→ E4 (Memory Manager + Capability Registry runtime)
└──→ E5 (lifecycle hooks + sandbox infrastructure)

E2 Spirit ABI + Developer SDK
├──→ E3 (Spirit ABI lifecycle hooks)
├──→ E4 (halt-protocol Spirit-side declarations)
├──→ E7 (full SDK extends E2 seed)
└──→ Story 2.3 thin cargo-generate slice → Story 7.5b NFR-Onb-1 v0.3 gate execution

E3 Director's Surface — IAC Bus, Task Assignment, Posture
├──→ E4 Story 4.1 (halt-resolution UX surface — MockHaltResolver pattern allows unit isolation but integration gates on Story 3.3 shipping)
├──→ E6 (IAC bus skeleton)
└──→ Story 3.4 kernel log-composition primitives → Story 8.1 Butler morning-digest implementation

E4 Halt Protocol + Memory + Cognition (SINGLE HALT OWNER)
├──→ E5 Story 5.2 (halt-continuity-across-hot-swap I14 enforcement)
├──→ E5 Story 5.3 (halt-receipt production rate measurement)
├──→ E8 (cognitive primitives consumed by reference Spirits)
└──→ INTRA-E4 ORDERING: Story 4.5 (HSIS corpus 100 scenarios) MUST precede Story 4.1 AC4 (halt-recall/precision measurement)

E5 Lifecycle + Hot-Swap + Multi-Provider
├──→ E6 (lifecycle triggers + crash supervision required for A2A peers)
├──→ E7 (Spirit registry over MCP-Streamable-HTTP)
└──→ INTRA-E5 ORDERING: §13.1 measurement gate (Story 5.5e) MUST be last in E5 (go/no-go on rust-inproc)

E6 Multi-Spirit + A2A
├──→ E7 (CCAC cross-Spirit scenarios require A2A)
├──→ E8 (Orchestrator + Workers require IAC bus full features; Mira+Nash requires A2A cross-Host)
└──→ E9 (cross-Spirit memory isolation 200-corpus and GDPR cascade depend on multi-Spirit runtime)

E7 Spirit Ecosystem
├──→ E8 (reference Spirits published via registry)
├──→ E10 (CCAC corpus authored here, cross-validated in Story 10.1)
└──→ Story 7.5b (NFR-Onb-1 v0.3 gate execution) DEPENDS ON: Story 2.3 (thin cargo-generate from E2) + Story 8.1 (Butler reference Spirit from E8). Forward-resolved by slicing Story 2.3 forward to v0.3 sprint.

E8 Reference Spirits
├──→ E9 (audit queries validated against reference-Spirit production traces)
└──→ E10 (Butler/Researcher/Orchestrator+Workers/Mira+Nash gate the v1.0 + v1.5 ship)

E9 Audit + Compliance + Operator Productionization
└──→ E10 (multi-operator tenancy primitive-reservation declared here; full impl v1.5+ in Story 10.4)

E10 v1.0 Ship Gate + v1.5 Collective Tier
└──→ Coordination epic — consumes corpora authored in E4 (HSIS 100), E5 (HSIS 200), E7 (CCAC 600), E9 (red-team 80→640 generator), E0 (secret-redaction generator)
```

**Sprint-plan invariants (must hold for above DAG to be coherent):**

1. **v0.3 sprint:** Story 0.4 → Story 1a.1 → Story 1b.4 (schema freeze) → Story 2.3 (cargo-generate) → Story 3.3 (halt UX) → Story 3.4 (digest primitives) → Story 4.1 (halt mechanism) → Story 4.5 (HSIS 100 corpus) → Story 8.1 (Butler) → Story 7.5b (NFR-Onb-1 gate execution)
2. **v0.5 sprint:** Stories 5.1–5.4 → Story 5.5e (§13.1 go/no-go) → Stories 8.2, 8.3 (Researcher, Observer)
3. **v0.8/v0.9 sprint:** Story 5.2 (HSIS 200 corpus) → Stories 6.1–6.5 → Story 8.4 (Orchestrator + Workers)
4. **v1.0 sprint:** Stories 7.1–7.5a → Story 7.3 (CCAC 600) → Story 9.6 (red-team 80→640 generator) → Story 10.1 (HSIS verification + CCAC cross-validation + pen-test) → Story 10.2 (third-party trial + adversarial red-team execution) → Story 10.3 (export-control + manifest fuzz + wire fuzz + Korean docs)
5. **v1.5 sprint:** Story 10.4 (Postgres Loom-lite + Mira+Nash + SQLite→Postgres migration) → Story 10.5 (skill-format conformance + JetBrains + Windows + 2-year LTS + Japanese/CN-S i18n) → Story 8.5 (Mira+Nash safety-critical corpus 150 + κ≥0.7)
