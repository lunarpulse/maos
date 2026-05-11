# Success Criteria

Success criteria for MAOS reflect its OSS-infrastructure-standard character: the goal is **adoption as the standard**, not revenue capture. Metrics are therefore weighted toward ecosystem health, technical correctness, and felt-trust signals rather than commercial outcomes. Three categories — User, Business (Adoption/Community), and Technical — converge on what "the substrate is working" means.

## User Success

User success is felt before it is measured. The substrate's promise is that a user can run agents that act on their behalf without losing track of what those agents are doing or why. Felt-trust is the engineering target.

- **Tier 1 (solo power user) — first-30-minutes test:** A new user installs MAOS, loads at least one Spirit, and completes one useful action — measured as "agent-driven outcome the user accepts and keeps" — within their first 30 minutes. Failure to meet this bar by v1.0 indicates the install path or the first-Spirit experience is too friction-laden for OSS-style adoption.
- **Tier 2 (team peer-mesh) — Day-30 transparency-log glanceability:** On day 30 of team usage, ≥70% of team members report glancing at the transparency log during a typical work week. This is Sally's load-bearing critique made measurable: mandatory transparency that nobody reads is theatre. If users have stopped looking by day 30, legibility has failed regardless of audit completeness.
- **Tier 3 (substrate proof in OSS / research community):** A 14-site research consortium successfully deploys a 28-agent Cortex using only the public MAOS kernel — same primitives, same manifests, no architectural fork. The deployment publishes its experience publicly (paper, conference talk, or blog series). This is the binary "did the substrate generalize" test.
- **Felt-trust dashboard (per Tier, exposed via Telemetry Stream):**
  - **Surprise rate:** percentage of agent actions the user marks "I didn't expect this." Target: declining over time within a single user's deployment as the agents calibrate to the user's surprise budget.
  - **Halt acceptance rate:** percentage of `epistemic.halt` invocations the user resolves with `provided_context` or `accepted_halt` rather than `authorized_override`. Target: ≥80% by v1.0 — meaning the Spirit's halt judgment is right >4-of-5 times.
  - **Digest open rate:** percentage of "what did your Spirits do while you were gone" digests the user actually opens. Target: ≥60% by v1.0 for daily active users.
  - **Time-to-first-trust by tier:** median sessions until a user shifts a Spirit from `cautious` to `assistive` posture. Target: <10 sessions for Tier 1; <30 for Tier 2; gated by external review for Tier 3.

## Business Success (Adoption / Community Health)

For an OSS infrastructure standard, "business success" is community velocity and ecosystem indicators — not revenue. The reference class is Linux/Postgres/K8s adoption curves at equivalent post-launch phases.

- **Phase validation milestones (binary at each release):**
  - **v0.1 milestone:** The Architect reference Spirit drives a real coding task on a local repository end-to-end with approval prompts. Six ACs from the kernel implementation guide pass in CI.
  - **v0.5 milestone:** A single user runs all six default Spirits on a laptop simultaneously (Butler, Researcher, Architect, Diagnostic Engineer, Enterprise stub, Observer). Sandbox tiers T0–T3 active. Transparency log persisted.
  - **v1.0 milestone:** An 8-person team reproduces J3 (Marcus Team Nexus) end-to-end with peer A2A mesh, mTLS, per-frame consent, role queries. **A third party authors and ships a Spirit binary independently of the MAOS source tree** — the "first non-Lunarpulse Spirit in the registry" milestone.
  - **v1.5 milestone:** J4 (Elena Mira-Nash) reproducible — diagnostic-architect Spirit pair closes a prod-incident-to-deployed-fix loop in ≤90 minutes with humans gating only at architectural decisions.
  - **v2.0 milestone:** Reza Cortex (single-org cross-team) reproducible at small scale (3-region pilot, ≥10 agents minimum). WASM Spirit registry live with Ed25519 signing + four trust tiers operational.
- **OSS leading indicators (the trackable signals):**
  - **Month 6 (post-v0.1):** ≥3 Spirits in the public registry whose `Cargo.toml` author is not Lunarpulse. ≥1 "boring fork" (someone forked, modified, ran). ≥1 protocol citation (a third party's blog post / RFC / implementation references the Spirit ABI as an interface they're targeting).
  - **Month 12:** ≥10 external Spirits. ≥3 protocol citations from independent agent projects. At least one cohort project (openclaw / ironclaw / hermes / paperclip / rustain) interoperating cleanly via ACP/MCP/A2A or integrating MAOS as substrate.
  - **Month 18:** First "Spirit Jam" event held at v0.3; ≥5 community-authored Spirits emerging from it. Total external Spirit count ≥20.
- **Single most diagnostic question at month 6:** "Has someone we have never met shipped something that depends on MAOS's protocol surface?" Yes/no. Binary. *No* is a falsification of the substrate-thesis; partial-yes is an early-confirmation signal.

## Technical Success

Technical success means the eight kernel guarantees and fourteen kernel invariants hold under stress, the performance budgets are met, and the substrate's claims are empirically verifiable rather than aspirational.

- **All 14 kernel invariants (I1–I14) empirically verified by v1.0** through a per-invariant property test suite. Every invariant has at least one falsifiable predicate documented and a property-based test running continuously in CI. No predicate, no claim — the invariant is downgraded from "guaranteed" to "asserted" until tested.
- **Adversarial Spirit suite passing by v1.0:** a maintained pen-test pack of malicious Spirits attempting to violate each invariant via memory residue, timing side channels, capability scope evasion, IAC log bypass attempts. Failure to break invariants becomes evidence that the kernel guarantees are real. This is non-negotiable for the "trust grounded in kernel invariants" claim.
- **Hot-path performance budgets enforced as CI gates by v1.0:**
  - `iac/send` (same-Host): <10μs P99
  - Capability token issuance (cached posture, no prompt): <5μs P99
  - Capability invocation dispatch (excluding adapter): <5μs P99
  - `memory/read` (cached): <50μs P99 / `memory/read` (uncached, SQLite): <5ms P99
  - Hot-swap (rust-inproc): <50ms P99 / Hot-swap (subprocess): <500ms P99
  - Telemetry broadcast (one event, 10 subscribers): <1μs
- **Epistemic halt empirical validation by v1.0:** Per-Spirit halt-recall and halt-precision numbers published on a public benchmark for every reference Spirit class. Floors (canonical across PRD per NFR-Test-4): halt-recall ≥0.7 (the Spirit halts on ≥70% of cases where it should), halt-precision ≥0.85 (≥85% of halts are warranted, false-halt rate ≤15%). Without these numbers, the "epistemic halt as Layer-1 capability" claim is downgraded from differentiator to mechanism-only.
- **Formal methods for invariants I5 (memory scope enforcement), I6 (hot-swap token preservation), I9 (kernel statelessness)** evaluated by v0.5; TLA+ or Alloy specs landed by v2.0 if property tests prove insufficient. Pragmatic position: property tests now via `proptest`, formal methods only when an invariant violation ships to a real user — `cargo test` carries 90% of the load at 10% of the cost.
- **OSS supply-chain hygiene from day one:** Apache 2.0 + MIT dual-license; SBOM published per release; SLSA-level attestations; reproducible *builds* of the kernel binary (`cargo build --locked`, no nightly, no `unsafe` in `maos-kernel` core) — note that LLM-stochastic *outputs* are not bit-reproducible by construction; replay determinism per ADR-028 / NFR-Aud-3 is over the trace shape, not over the payload content; signed Spirit-registry artifacts (Ed25519 publisher keypairs); `cargo deny check` in CI gating dependency drift.
- **Three-protocol coherence empirically demonstrated:** ACP, MCP, A2A all integrated via the kernel's I/O subsystem; ≥1 MAOS Host successfully launched by Zed (ACP), pulling tools from a public MCP server (MCP), and peering with another Host (A2A) — all in one session — by v1.0.

## Measurable Outcomes

The minimum bar across categories at each release:

| Phase | User signal | Adoption signal | Technical signal |
|---|---|---|---|
| v0.1 | 6 ACs pass | Public repo + first commit + CI green | Property tests for I1, I2, I10 |
| v0.5 | All six default Spirits run on a laptop | First "Spirit Jam" candidates identified | Property tests cover I1–I10; T2/T3 sandbox empirically isolating |
| v1.0 | J3 Team Nexus reproducible by 8-person team | ≥1 non-Lunarpulse Spirit in registry; ≥3 external Spirits by month 6 post-release | All 14 invariants empirically verified; halt-recall ≥0.7 / halt-precision ≥0.85 published per Spirit; performance budgets enforced as CI gates |
| v1.5 | J4 Mira-Nash reproducible (90-min diagnostic-architect loop) | ≥10 external Spirits; ≥3 protocol citations | Loom-lite operational; post-deploy IAC topic working |
| v2.0 | Reza Cortex reproducible at 3-region pilot | ≥20 external Spirits; ≥1 cohort project interoperating; "Spirit Jam" annual event | All three Spirit forms operational; signed registry; PDP integration |
