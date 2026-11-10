# 20-14 L1 adversarial review — f68c7de6

## Scope and result

**4 patch, 2 defer, 12 dismissed groups; 0 technical operator decisions required.** One high-severity production defect blocks acceptance of AC4.2. Review is read-only: no builds, tests, plants, formatting, or repository mutations were executed. Failing-before cases below are code-derived predictions, not observed executions.

Executed bmad-code-review steps 01–04 non-interactively. Explicit assignment overrides workflow prompts, story/status writes and automatic patching: only this review directory is writable. Four successful layers: Blind Hunter (`reviewer`), Edge-case Hunter (`reviewer`), Acceptance Auditor (`reviewer`), Test Infrastructure Auditor (`light-reviewer`). No security layer was spawned. Each layer reviewed the commit, not concurrent maos-bin working-tree changes. Layer transcripts: `history://Review2014L1.Blind2014`, `history://Review2014L1.Edge2014`, `history://Review2014L1.Accept2014`, `history://Review2014L1.Tests2014`.

## Patch

### P1 — high — Ordinary capability rows overwrite Spirit identity

- **Sources:** blind + edge + acceptance + test-infrastructure; independently confirmed during triage.
- **Location:** `crates/maos-audit/src/lib.rs:1786-1806`; consumer `crates/maos-cli/src/subcommands.rs:4898-4906,5098-5105`.
- **Violation:** AC4.2 / D-20-14-05 A.
- **Reachability:** kind 7 is CapabilityInvocation, not an identity-only kind. The query accepts all kind-7 rows except `lifecycle.journal`; every later row lacking `spirit_id` overwrites the map with its intent. Real writers include `cap.verify.ok` (`crates/maos-kernel-core/src/capability/cap_audit/writer_task.rs:49-65`) and `lifecycle.unload` (`crates/maos-kernel-core/src/scheduler/scheduler_loop.rs:547-558`). Thus ordinary operation changes the cost report's Spirit to `cap.verify.ok`, `lifecycle.unload`, or `cap.revoke`.
- **Recorded witness:** `creation/runtime/tl-rows-head.txt` contains two boots, pid 1, actual `hello-spirit` load identities followed by capability/lifecycle events and final `cap.revoke` rows. Following the committed last-wins algorithm yields `cap.revoke` for both boots [INFERENCE]. This is not a hypothetical forged database.
- **Exact fix:** accept only genuine identity shapes: kind 19 admission identities and kind 7 trusted lifecycle load/admit identities with a valid nonempty `spirit_id`; do not use arbitrary capability intents or arbitrary payload `spirit_id` values as identity authority. Preserve explicitly supported legacy identity layouts only through a positively identified legacy shape, not a catch-all fallback. Leave identities absent when no qualifying row exists so the report uses `pid:<n>`. Narrow SQL accordingly; merely keeping the first arbitrary row or making intent a fill-only fallback is insufficient for unnamed pids. Keep `(boot_nonce,pid)` isolation. Add regression rows for ordinary invocation, unload, and a non-identity payload containing a forged `spirit_id`.
- **Failing-before witness:** extend `cost_spirit_names_distinguish_boots_and_skip_echoes` after alpha's load with `(boot=7,pid=42,kind=7,ts=50,intent='cap.revoke',payload=empty blob)`. Assert name remains `alpha`; committed code returns `cap.revoke` [INFERENCE]. Also seed an unnamed pid with only `cap.verify.ok`; assert it has no name-map entry. Add a non-identity kind-7 row containing `{"spirit_id":"forged"}` and assert it cannot replace alpha.

### P2 — medium — Report tests do not independently prove boot-name separation or unnamed fallback

- **Sources:** test-infrastructure; triage confirmed at `crates/maos-cli/src/subcommands.rs:5950-6014`.
- **Location:** `crates/maos-cli/src/subcommands.rs:5984-6006`; fallback `:4898-4901`.
- **Violation:** AC4.2/AC4.4 and review's recycled-pid/unnamed-row coverage requirement.
- **Detail:** boot 2 is also the only entry without a task. Its row would remain distinct even if the Spirit dimension stopped participating in grouping. All entries in the new vector have names; the old empty-map vector checks tokens but not the resulting Spirit string. The recorded all-axes-at-once mutant does not establish independent boot-name protection.
- **Exact fix:** add two cost frames with identical task, principal, source, pid, provider and model but different boots mapping to alpha/beta; assert exactly two rows, correct names and nonzero token totals per row. Add an identity-less `(boot,pid)` and assert `spirit == "pid:<pid>"`. Include repeated same-key frames to prove aggregation remains additive. Prefer feeding names obtained from a SQLite fixture through the report when feasible; do not expose a new public production API solely for testing.
- **Failing-before witness:** hold the Spirit component of the grouping key constant while retaining the display name; the new equal-other-axes vector must fail with one row instead of two. Replace the fallback with `"unknown"`; the new unnamed assertion must fail. These are predicted mutation witnesses, not executed plants.

### P3 — low — ADR amendment removes unrelated outstanding SR-1 debt

- **Sources:** edge; original/current ADR read during triage.
- **Location:** `docs/adr/ADR-046-cost-attribution-and-reconciliation.md:96-98`, related `:58`.
- **Violation:** AC5's truthful in-place ADR amendment; root session binding does not close write-path authorization.
- **Detail:** the parent Consequences bullet explicitly retains both SR-1 write-path authorization and SR-5/R3 binding. Its replacement retains only SR-5/R3. Line 58 still refers readers to a tracked write-path follow-up in §10, which no longer contains it. This is a debt-record deletion, not a request to implement authorization in L1.
- **Exact fix:** retain a separate explicit SR-1 write-path-authorization follow-up sentence in Consequences/§10, with its existing disposition and a valid pointer. Keep SR-5/R3 pending L2 and the proxy tagged fallback. Do not declare this debt closed or introduce an unrelated source-code fix.
- **Failing-before witness:** a focused ADR content assertion that the retained follow-up section names both `SR-1` and `validate_namespace_write` fails on the changed Consequences/§10 text [INFERENCE]; scope the assertion to that section, not the entire ADR where historical mentions remain.

### P4 — low — Decision date and execution status do not describe the recorded landing

- **Sources:** blind + edge + acceptance.
- **Location:** `_bmad-output/implementation-artifacts/20-14-complete-cost-attribution.md:24,181,292`; `sprint-status.yaml:388-389`.
- **Detail:** D-13 records a 2026-11-12 ruling in a commit dated 2026-11-10. The story/tracker still say ready-for-dev although the L1 implementation exists and T0 explicitly requires in-progress. Neither is evidence that the ruling itself is absent: the operator's supplied D-13 A adjudication is authoritative.
- **Exact fix:** remove the unsupported calendar date from D-13 while retaining operator ask `d1408`, or replace it with the date from the authoritative ask record; do not invent that date from the commit clock. Set story and tracker to `in-progress`, never `done`; keep T0/T4 and all L2 work open until their evidence exists. Do not tick tasks solely from this review.
- **Failing-before witness:** inspect the decision row against the commit timestamp and the story's own T0 status transition: the current date is future-relative and both execution-status fields remain ready-for-dev. This is a record comparison, not a runnable production failure.

## Defer — pre-existing, not L1 regressions

### D1 — medium — Per-Spirit coverage summary still pools recycled pids

- **Sources:** blind + edge + test-infrastructure.
- **Location:** `crates/maos-cli/src/subcommands.rs:4794-4797,4921,4936-4956,5033-5036`.
- The row key gains names, but `per_spirit_attributable_fraction` is still keyed only by pid. Different Spirits across boots can share one summary fraction. This predates the commit and AC4 explicitly enumerates a row key, not a summary schema migration.
- **Future fix/witness:** define a name+pid or incarnation summary identity and migrate its wire/plain consumers explicitly. Two differently named Spirits sharing pid 7, one attributed and one unattributed with equal priced spend, currently yield one 0.5 summary instead of separate 1.0/0.0 summaries [INFERENCE]. No L1 operator decision is required unless scope is expanded to migrate this pre-existing schema.

### D2 — low — R6 terminology overstates unknown-dimension tolerance

- **Sources:** edge + test-infrastructure; triage confirmed at `crates/maos-domain/src/cost.rs:279-297` and ADR `:36,105`.
- Unknown dimensions reject the entire frame and the report skips it with a warning (`subcommands.rs:4862-4870`); this is unchanged. The new wrapper regression correctly preserves that behavior and AC4.1 requires the existing R6 tests unchanged. ADR calls this forward-read tolerance, which can be misread as preserving known quantities from a future-dimension frame.
- **Future fix/witness:** clarify the documented rejection/not-panic contract, or separately authorize a forward-compatible dimension representation. A frame containing tokens plus `cpu_micros` is rejected today, not partially counted. Do not silently change this behavior in the wrapper patch.

## Dismissed — 12 normalized groups

1. Missing AC1–AC3/root writer/full AC6: expressly L2 under D-08 A, not claimed by L1.
2. T0 wrapper kernel delta, complete mint inventory, seam identity and AC1 timing not all measured: disclosed outstanding; no measured/closed claim to contradict.
3. Domain 10203 exceeds original 10200 allowance: adjudicated 10215 ceiling governs; 10203 is inside it. CLI 6529 likewise inside adjudicated 6545.
4. Moving counted prior tests to evade pins: no prior vectors moved; old no-task test replaced in place and new serde-only vectors live in tests/.
5. Flatten legacy/duplicate/u64/unknown-top-level compatibility defect: explicit vectors cover these; no new concrete failing input identified. Unknown dimensions are D2, not a wrapper regression.
6. New panic from serde unwrap/expect: production report parsing uses a Result match; new serialization unwraps are test fixture/assertion code, not input-driven production panics.
7. No final-tree green/prepush/hosted/macOS/Windows/no-default-features proof: evidence expressly discloses unrun assertion, timed-out prepush, failed workspace tests and unverified platforms. They remain landing proof prerequisites, not fabricated passes or proven portability bugs. Review did not rerun failures.
8. Mandatory new real-command integration test/error-branch test in L1: real AC1 command is L2; P1/P2 carry concrete L1 regressions rather than requiring a new public reporting API.
9. Dropped request/correlation-id negative assertion: D-03 B remains authoritative; no request field or request-fed task was introduced. Optional tripwire restoration is not a demonstrated implementation defect.
10. SR-3 manually seeded partial frame and pending-string docs assertion: SR-3 is explicitly a T0 body-scan measurement, not a claim of schema-2 emission; docs correctly assert pending L2 and should change there. Neither is a fake L1 closure.
11. Task/Spirit sentinel collisions, timestamp ties, erased identities and numeric nonce casts: no reachable new defect demonstrated for root-minted task formats; boot/pid casts match the established audit reader. Harden only with a concrete production witness.
12. Context reworded rather than byte-preserved, `task: none` prose vs `task=none` plain syntax, and stale old grouping help: supersession and L1/L2 posture remain explicit; `task: none` is story terminology rather than a promised literal output line. Help can be updated with the next CLI documentation pass; not a blocker.

## Acceptance and accounting disposition

- Wrapper uses flatten and leaves the exhaustive kernel payload unchanged; L1 adds no kernel src changes. Kernel tests are outside the pinned src surface. No invariant files changed in this commit.
- Recorded exact production pins: domain 10203 (+5), CLI 6529 (+100), audit 8209 (+30), bin unchanged; aggregate 198914 + 135 = 199049, hardfail 199050. These are inside disclosed adjudicated ceilings. No new measurement was run by review.
- Coverage-matrix notes name actual L1 witnesses and leave root-bound emission, real command and closure to L2. Operator guides disclose the proxy honesty boundary, schema-1/2 meanings, Worker exclusion, v1.1 dimensions and operational invoice reconciliation. ADR retains pending-writer language.
- **Operator decisions:** retain D-08 A two landings, D-09 A rebased ceilings, D-13 A flattened wrapper and adjudicated domain 10215/CLI 6545. D-12 C/shared STOP governs only later kernel landing; review grants no kernel lines. No new technical design decision is needed for P1–P3. P4's actual ruling date may only come from the authoritative operator record; removing the unsupported date avoids guessing.
- **After patches, Main/coder owns verification:** focused maos-audit spirit_resolution, maos-cli cost_report and cost_docs_20_14, maos-domain cost_frame_20_14 if wire code changes; refresh exact kloc pins and required documentation/record gates. The previously unrun schema-1 assertion requires final-tree evidence. Complete the existing prepush/authoritative CI and platform obligations without claiming this read-only review executed them. Re-measure grants if fixes exceed existing ceilings; do not silently borrow kernel room.
