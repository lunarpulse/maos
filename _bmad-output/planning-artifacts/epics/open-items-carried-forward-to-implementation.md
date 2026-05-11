# Open Items Carried Forward to Implementation

The following items were knowingly carried into implementation rather than closed during step-03/04. None block development — each has a documented fallback path or downstream-closure plan. Dev agents picking up stories should be aware of these.

## 1. Partial crate-path retrofit on ~49 stories

**Status:** ~15 stories have full crate-path treatment in every AC (the exemplar set); ~49 stories retain some "the kernel" / generic-component references in select ACs.

**Why deferred:** Retrofitting 300+ ACs across 49 stories was diminishing-return work after the exemplar set established the pattern. Story 1a.1, 1b.5a/b/c, 3.3, 4.1, 5.2, 5.5a–e, 7.5a/b, 0.5, and 9.2 demonstrate the target conventions.

**Fallback path for dev agents:** When an AC says "the kernel" without a crate path, consult `architecture-maos-minimal-opus.md` §4.0.2 for the canonical crate-to-responsibility mapping. The 17-crate workspace is bounded — "the kernel" almost always maps to `crates/maos-kernel-core/<service>/` where `<service>` ∈ {scheduler, security, memory, iac, capability}. The full retrofit-status note lives inline at the top of the Epic List.

**Closure path:** During story execution via `/bmad-create-story`, the dev agent or PM can convert "the kernel" references to specific crate paths on a per-story basis as stories land in sprint planning. This is preferable to a speculative retrofit since the actual crate boundaries may evolve slightly during E1a's bootstrap.

## 2. v0.3 Halt corpus is provisional `synthetic-v0` N=50

**Status:** Story 4.1's halt-recall/precision floor measurement (NFR-Test-4: ≥0.7 / ≥0.85) cites a provisional corpus at `crates/maos-eval/fixtures/halt-corpus-v0/` containing 50 hand-authored synthetic scenarios.

**Why deferred:** Round-3 stress-test (Amelia + Murat) flagged that the original AC pointed to "bmad-eval standard corpus against E8 reference Spirits" — a corpus that does not exist when Story 4.1 is implemented (E8 reference Spirits don't ship until v0.3+ per their respective phase anchors). Writing tests against a future corpus is a forward-dependency leak.

**Fallback:** The synthetic-v0 corpus is sufficient to gate Story 4.1's v0.3 release. It validates the halt mechanism's measurement plumbing end-to-end. Floor numbers are real (≥35/50 recall, ≥43/50 precision); they're just measured against synthetic prompts rather than reference-Spirit production traces.

**Closure path:** At v1.0, the E8 reference-Spirit corpora (Butler 30-scenario calendar/comms from Story 8.1; Researcher distillation eval from Story 8.2; Orchestrator+Workers founder-loop scenarios from Story 8.4; Mira+Nash safety-critical N≥150 from Story 8.5) replace `synthetic-v0` as the bmad-eval gate. Story 4.1's AC4 explicitly tags the corpus `synthetic-v0` to distinguish it from the v1.0 reference corpora.

## 3. Intra-E4 ordering: Story 4.5 (HSIS 100) must close before Story 4.1's halt-receipt gate at v1.0

**Status:** Story 4.1 (halt mechanism + halt-receipt ≥99.9%) and Story 4.5 (cross-Spirit memory isolation 200-corpus + Hot-Swap I14 enforcement + HSIS Researcher+Observer 100 scenarios) are both in E4. Story 4.1's halt-receipt gate is measured against the HSIS termination corpus that Story 4.5 authors.

**Why deferred:** Round-3 (Murat's risk-rating) identified that if Story 4.5's 100 HSIS scenarios are sprinted *after* Story 4.1's gate-closure attempt, the gate has no production-grade corpus and falls back to synthetic-v0 (per item 2 above). The fix is sprint-ordering, not story-rewriting.

**Closure path:** Sprint plan must enforce Story 4.5 corpus authoring closes before Story 4.1's v1.0 halt-receipt gate runs. This is documented in:
- Story 4.1 AC4: "**intra-E4 ordering: Story 4.5 (HSIS corpus 100 scenarios) MUST close before Story 4.1 AC closes at v1.0**"
- Story 5.2: HSIS additional 200 scenarios (Butler/Orchestrator/Worker/CliWrapper classes) must also land before Story 4.1's v1.0 gate

**Composite gate at v1.0:** Story 4.1 halt-receipt ≥99.9% across the **cumulative 300-scenario HSIS corpus** = 100 from Story 4.5 (Researcher+Observer) + 200 from Story 5.2 (Butler/Orchestrator/Worker/CliWrapper). The Dependency DAG section above captures this in the v1.0 sprint invariants.

---

## Summary

These three items are known shapes of the v0.1 → v1.0 sprint plan, not defects in the epic/story breakdown. Dev agents implementing E4 should treat Story 4.1's `synthetic-v0` corpus as a v0.3-shippable measurement floor, not a permanent target. Sprint planners should sequence Story 4.5 + Story 5.2 corpus authoring **before** any v1.0 gate-closure attempt on Story 4.1. PMs running `/bmad-create-story` to extract individual story specs should consult `architecture-maos-minimal-opus.md` §4.0.2 to concretize "the kernel" references into specific crate paths at story-extraction time.

