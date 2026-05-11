---
Status: binding-v0.1
Gate: invariant-lock CI gate runs on every PR touching I1–I14
Decided: 2026-04-15
Revisits: §3.2, §8.7
---

# ADR-037 — Constitutional amendment process

**Decision.** ADRs touching invariants I1–I14 require two-reviewer + invariant-test diff; CI gate `invariant-lock` enforces. ADR amendments require: (a) machine-checkable diff against the invariant set, (b) a corpus delta showing the test surface that exercises the change, (c) a phase-commitment update.

**Rationale.** The constitutional commitment (Innovation #7 in PRD Step 6) requires architectural enforcement, not founder discipline. Without ADR-037, ADRs are markdown that one human can rewrite.

**Alternatives considered.** Process-only amendment (rejected: relies on founder discipline). External governance board (rejected: scope inflation).

**What would force a revisit.** The reviewer pool becomes too small for the two-reviewer requirement.
