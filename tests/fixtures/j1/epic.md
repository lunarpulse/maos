# Epic J1-E: founder-loop epic dispatch (fixture)

Story 19-1 fixture for `maos run spirits/topologies/j1-founder-loop-epic.toml
--epic tests/fixtures/j1/epic.md --once`. The preamble above the first `###`
heading is never dispatched; each `### Story` section below is one Worker's
goal, byte for byte, in file order.

### Story 1: Record the founder's intent

Write `intent.md` stating the one problem this epic solves, in two sentences.

**Acceptance:** `intent.md` exists and names the problem.

### Story 2: Draft the design note

Using the distillate of Story 1, write `design.md` with the module boundary
and the single public function it exposes.

**Acceptance:** `design.md` names exactly one public function.

### Story 3: Review the design note

Using the distillate of Story 2, write `review.md` listing at most three
findings against `design.md`, each with a severity.

**Acceptance:** `review.md` lists one to three findings.
