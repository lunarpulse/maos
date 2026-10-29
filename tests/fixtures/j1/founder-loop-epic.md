# Epic J1-F: the founder loop with one ambiguous story (fixture)

Story 19-4 fixture (D-19-4-04 A) for `maos run
spirits/topologies/j1-founder-loop-epic.toml --epic
tests/fixtures/j1/founder-loop-epic.md --once`, driven by `xtask demo-j1` and
`journey_j1`. The preamble above the first `###` heading is never dispatched;
each `### Story` section below is one Worker's goal, in file order. Section 2
is under-specified on purpose: its acceptance names no measurable bound, so a
competent Worker could build the wrong thing from it. The replayed judgement in
`crates/maos-journey-test/cassettes/j1/founder-loop.json` (its prompt hashes
are over these exact section bytes) decides the halt; this text is honest
about why it should.

### Story 1: Record the founder's intent

Write `intent.md` stating the one problem this epic solves, in two sentences.

**Acceptance:** `intent.md` exists and names the problem in at most two
sentences.

### Story 2: Make the import faster

The nightly import feels slow. Make it faster and keep users happy with it.

**Acceptance:** the import is fast enough and nobody complains.

### Story 3: Review the change

Using the distillate of Story 2, write `review.md` listing at most three
findings against the change, each with a severity.

**Acceptance:** `review.md` lists one to three findings, each with a severity.
