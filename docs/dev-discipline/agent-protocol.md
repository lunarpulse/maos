# Agent protocol v2

Operator decisions 10 and 12, 2026-11-23 (lean-gates plan WP-12). It replaces the resume-protocol paragraphs of
`session-roadmap-2026-11-16.md`. Review is defined once, in
[`docs/testing/proof-contract.md`](../testing/proof-contract.md) ("Review by path").

## 1. Unit of work

One work-package brief is one PR. A brief is at most 40 lines: goal, files owned, change, verify commands, plant
(the planted failure that proves the check bites), rollback. No story file unless the work spans more than one PR.

## 2. Lanes

Work is split by path into lanes. One writer per lane at a time. The orchestrator schedules lanes, not stories.
A brief names the paths it owns; an agent never edits outside them and asks the orchestrator when a change outside
is needed.

## 3. Verification and hand-off

- An agent runs the scoped `cargo run -p xtask -- prepush` itself, in a per-lane target directory
  (`CARGO_TARGET_DIR=<lane dir>`), and reports commands and exit codes it observed.
- At most two cargo sessions run on the host at once: the host swaps under more.
- The orchestrator formats, commits, pushes and merges. It does not re-verify what CI verifies; CI is the gate.
- Agents never commit, push, tag, sign, dispatch release workflows or write ledgers.

## 4. Review and retries

- Review follows the path rule in the proof contract; no other PR owes a review.
- A failed package gets one narrowed re-brief. If that also fails, the orchestrator takes the package.

## 5. Operator asks

Ask only for user-visible behaviour, irreversible operations, the trust-boundary list
(`docs/testing/trust-boundary-paths.txt`) and spend. Batch them into one sheet per day; do not ask one at a time.

## 6. Done means merged

A package is done when its PR is merged. Tracker state is updated from merged PR trailers (`Story: <key>`).

## 7. Standing approval: disk hygiene

`cargo sweep --time 7` on MAOS-owned target directories is approved whenever no cargo process holds the directory
(check first with `pgrep -a cargo`). Nothing else is deleted without a per-event ask.

## 8. Infrastructure-only re-run (decision 10)

A job that failed for an infrastructure-only reason (runner loss, network, package-mirror failure, container-runtime
create or pull error) may be re-run once. Write the reason in the PR. Never re-run a test failure. Never re-run on a
candidate SHA (a release or T3 candidate): a red candidate is read and fixed. No retry-to-green anywhere else: a
second red is a failure.
