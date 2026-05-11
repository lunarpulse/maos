# Dependency Verification (12-Epic Ordering)

Strict ordering ensures no epic requires a later epic to function. Where forward dependencies exist (e.g., NFR-Onb-1 at v0.3 needs E7 ecosystem tooling), they are resolved by **slicing minimum prerequisites forward**, not by reordering.

- **E0** stands alone — cross-cutting; founding sprint with v0.1 acceptance, then maintenance.
- **E1a** depends on E0 (CI gates must be green before workspace lands).
- **E1b** depends on E0 (ComplianceClaim schema adversarially reviewed before freeze) + E1a (workspace + ABI types).
- **E2** depends on E1a (Spirit ABI types) + E1b (manifest schema frozen, hello-Spirit roundtrip). Provides NFR-Onb-1 prerequisites that E7 + E8 Butler consume at v0.3.
- **E3** depends on E1b (IAC bus skeleton + Approval Decision Log + Transparency Log) + E2 (Spirit ABI lifecycle hooks). Halt UX surface; halt mechanism dependent on E4.
- **E4** depends on E1a (halt schema types in `maos-domain`) + E2 (Spirit ABI hooks) + E3 (halt-resolution surface). **SINGLE HALT OWNER.**
- **E5** depends on E4 (halt mechanism for FR53 halt-continuity-across-hot-swap) + E1b (sandbox tier infrastructure) + E2 (Spirit ABI lifecycle hooks). §13.1 measurement gate lives here.
- **E6** depends on E1b (IAC bus skeleton) + E2 (Spirit ABI) + E3 (`task.assign` frame definition) + E4 (intent_lineage/cognition substrate types) + E5 (lifecycle triggers + crash supervision; A2A peers require Spirits to crash-survive).
- **E7** depends on E1b (ComplianceClaim schema frozen) + E2 (SDK seed) + E5 (Spirit registry over MCP) + E6 (IAC bus for spirit-test integration scenarios). Executes NFR-Onb-1 v0.3 gate using E2 prerequisites + E8 Butler.
- **E8** depends on E4 (memory + halt for Butler v0.3) + E5 (lifecycle for Butler-class), E6 (IAC + A2A for Orchestrator+Workers v0.8 and Mira+Nash v1.5), E7 (Spirit ecosystem tooling for publishing reference Spirits).
- **E9** depends on logs/frames/state from E1b–E6 to query/export/forget; on E4 principal namespace for GDPR cascade; on E7 ComplianceClaim envelope for verification queries.
- **E10** depends on **all prior epics** — coordination gates fire against work owned elsewhere (HSIS corpus authored E5, CCAC corpus authored E7, etc.). v1.5 sub-cluster work depends on E9 (multi-operator tenancy primitive-reservation) + E6 (cert rotation infrastructure) + E4 (memory tiers for Loom-lite collective).

**Forward-dependency resolution (NFR-Onb-1 v0.3 phasing tension):**
- E2 ships thin cargo-generate template + local runner + ≥1 example Spirit with passing CI **at v0.3-era completion** (before Butler reference Spirit ships in E8).
- Full spirit-test SDK with assertion macros lands in E7 (v0.5+ feature work).
- E7 RUNS the 30-Min First Spirit Gate at v0.3 using Butler from E8 + thin tooling from E2.

**Halt protocol dependency chain (resolved):**
- E1a defines halt schema types in `maos-domain` (data only).
- E4 owns halt mechanism + I14 invariant + halt-receipt 99.9% + recall/precision floors.
- E3 owns halt resolution UX surface (notification, 3-tap mobile flow) — calls into E4 primitives.
- E5 owns halt-continuity-across-hot-swap (FR53 I14 runtime check) — Hot-Swap Coordinator validates `halt_set` before swap using E4's schema.

**rust-inproc gating (resolved):**
- ADR-002 commits to subprocess form at v0.1; rust-inproc gated on §13.1 measurement.
- E5 carries the §13.1 measurement story with go/no-go gate before v0.5 ships.
- If subprocess form meets latency budgets (J1 <25ms P95 IPC; J4 <10ms P95), rust-inproc form may be deferred to v2.0+ (eliminating NFR-Test-7 cross-form equivalence from v1.5 scope).
- If gates fail, rust-inproc development unlocks within E5 with cross-form equivalence test in E10.

---
