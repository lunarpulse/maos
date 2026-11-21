---
Status: binding-v0.1
Gate: invariant-lock CI gate runs on every PR touching I1–I14
Decided: 2026-04-15
Accepted-in-PR: <PR_NUMBER>
Revisits: §3.2, §8.7
---

# ADR-037 — Constitutional amendment process

**Decision.** ADRs touching invariants I1–I14 require two-reviewer + invariant-test diff; CI gate `invariant-lock` enforces. ADR amendments require: (a) machine-checkable diff against the invariant set, (b) a corpus delta showing the test surface that exercises the change, (c) a phase-commitment update.

**Amendment 2026-11-21 (D-21-10-03 A, story 21-10).** The phase-commitment requirement (c) is satisfied by a changed `enforcement_cadence` cell in a touched `I*.md`, or by a validated `claim: asserted -> guaranteed` flip of that file (the flip is itself the commitment: the v1.0 cadence cells are fixed by Architecture §3.2.1 and may not be invented). The gate reads the changed values of the file at `HEAD~1` and at the working tree; diff context lines never satisfy (c). The gate also treats every file under `crates/maos-domain/src/invariants/` as an invariant edit.

**Rationale.** The constitutional commitment (Innovation #7 in PRD Step 6) requires architectural enforcement, not founder discipline. Without ADR-037, ADRs are markdown that one human can rewrite.

**Alternatives considered.** Process-only amendment (rejected: relies on founder discipline). External governance board (rejected: scope inflation).

**What would force a revisit.** The reviewer pool becomes too small for the two-reviewer requirement.
