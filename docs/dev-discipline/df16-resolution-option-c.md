---
title: DF16 Resolution — Per-Merge CI Artifact (Option c)
status: decided
decided_on: 2026-05-13
authors:
  - Amelia (Senior Software Engineer)
approvers:
  - Lunarpulse (Project Lead)
addresses: DF16 (`docs/invariants/journal.jsonl` never receives entries on merge)
binds: Story 0.1 AC5 spec ("the CI job that runs this is the merge queue job, not the per-push job") — re-interpreted as journal-source-of-truth, not in-repo journal-as-canonical
supersedes: docs/dev-discipline/df16-journal-merge-queue-design.md
revert_semantics: paired-reverted-entry
---

# DF16 Resolution — Per-Merge CI Artifact (Option c)

## Decision

**The journal-of-record lives in the GitHub Actions artifact set, not in `docs/invariants/journal.jsonl` directly.** A merge-time workflow produces one journal-entry artifact per merging commit; an operator-triggered aggregator workflow downloads the artifact set on demand and produces a reviewable journal file for the operator to commit through normal PR review.

This option was chosen over **Option (a) — Merge-Queue + Commit-Back** because:

1. Option (a) requires a GitHub App token with `contents: write` AND branch-protection bypass — an ADR-037 sensitivity zone (the same gate that demands ≥2 reviewers gains a bypass path for its own auxiliary commits).
2. Option (c) eliminates the bypass entirely. Every change to `docs/invariants/journal.jsonl` goes through normal PR review.
3. The "in-repo journal" wording from Story 0.1 AC5 is re-interpreted: the journal-of-record is the artifact set (authoritative, append-only, 90-day retention); the in-repo file is a periodically-refreshed mirror that the operator commits.

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│  Merge time (merge_group event OR push: main)                    │
│  ──────────────────────────────────────────────                  │
│  .github/workflows/journal-append.yml                            │
│      → cargo run -p xtask -- invariant-lock \                    │
│          --changed-files <list> \                                │
│          --pr-number <N> \                                       │
│          --sha <merge-sha> \                                     │
│          --pr-body <body-file> \                                 │
│          --write-journal \                                       │
│          --journal-output /tmp/journal-output/entry.jsonl        │
│      → uploads journal-entry-<sha> artifact (90d retention)      │
└──────────────────────────────────────────────────────────────────┘
                              │
                              │  (artifact set accumulates)
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Operator-triggered (workflow_dispatch, ad-hoc cadence)          │
│  ──────────────────────────────────────────────                  │
│  .github/workflows/journal-aggregate.yml                         │
│      → lists journal-entry-* artifacts                           │
│      → downloads + concatenates in created_at order              │
│      → uploads journal-aggregated-<run_id> artifact              │
│      → operator downloads, places at                             │
│        docs/invariants/journal.jsonl, opens PR                   │
│      → PR goes through normal review (≥2 reviewers, all gates)   │
└──────────────────────────────────────────────────────────────────┘
```

## What landed in this session (2026-05-13)

### xtask refactor (`xtask/src/invariant_lock.rs` + `main.rs`)

- Added `--write-journal` flag (default false). The append code path only fires when set.
- Added `--journal-output <path>` flag (default `docs/invariants/journal.jsonl`). The merge-time workflow sets this to `/tmp/journal-output/entry.jsonl`.
- Added `--pr-body <path>` flag (optional). Points at a file containing the merging PR's body; used for revert detection.
- Added `RevertReference` struct + `detect_revert` function. Parses the GitHub revert idiom (`Reverts #N` and/or `This reverts commit <sha>.`).
- `append_journal` writes the primary entry and, if `revert_of` is populated, emits a paired "reverted" entry referencing the original PR/SHA.
- The private `invariant_lock` function no longer carries the `sha` parameter (journal-write moved to public `run`).
- 16 unit tests pass — 5 pre-existing + 4 DF17 multi-invariant + 7 DF16 revert-detection.

### Workflow files

- **`.github/workflows/journal-append.yml`** (new) — Triggered by `merge_group: checks_requested` and `push: main`. Resolves the merge SHA, parses the PR number from the merge-commit message, fetches the PR body via `gh pr view`, runs `invariant-lock` with `--write-journal`, uploads the entry as `journal-entry-<sha>` artifact (90-day retention). If no invariants are touched, no artifact is uploaded (no pollution of the artifact set).
- **`.github/workflows/journal-aggregate.yml`** (new) — Manual `workflow_dispatch`. Lists journal-entry-* artifacts via GitHub REST API, downloads each, concatenates in `created_at` order, uploads the result as `journal-aggregated-<run_id>` artifact. **Does NOT commit-back.** The operator downloads, opens a PR.

### `discipline.yml` update

- The per-push `invariant-lock` job no longer passes `--sha` or `--write-journal`. Validation continues (tri-requirement + reviewers + regression check); persistence is delegated to `journal-append.yml`. This makes the validate / persist separation explicit.

## Revert semantics (paired-reverted-entry)

When a merging PR is itself a revert (its body matches the GitHub revert idiom), the journal entry includes a `revert_of` payload, and the `append_journal` function emits **two** entries in the artifact:

```json
{"ts": 1234567890, "invariant_ids": ["I7"], "pr_number": 5678, "reviewers": 2, "sha": "<merging-sha>"}
{"ts": 1234567890, "reverted_by_sha": "<merging-sha>", "reverted_by_pr": 5678, "pr_number": <original-pr>, "sha": "<original-sha>"}
```

The pair is human-auditable: the primary entry records the revert PR's claims; the paired entry annotates that the original PR's claims are now reverted. The journal remains append-only and self-consistent on revert.

## What the operator must do

For the system to work end-to-end, the operator (Project Lead or designated maintainer) must:

1. **Enable GitHub merge queue** for the repo (Settings → Branches → main → Require merge queue). The `merge_group` event fires only when this is on.
2. **Add `journal-append` to the required-status-checks list** for `main`'s branch protection rule, so the journal entry is computed before merge completes.
3. **Run `journal-aggregate` periodically** (suggested cadence: quarterly, aligning with NFR-Test-1 corpus rebaseline). Trigger via Actions → journal-aggregate → Run workflow. Download the resulting artifact and open a PR placing it at `docs/invariants/journal.jsonl`. The PR goes through normal review.

The artifact set is the authoritative log between aggregations. Any consumer who needs the up-to-date journal can either (a) read the latest committed `journal.jsonl` (stale up to the aggregation cadence) or (b) download the artifacts directly via `gh run download` (always current).

## Acceptance criteria for DF16-closed

DF16 is closed when:

1. ☑️ xtask refactor lands (`--write-journal`, `--journal-output`, `--pr-body`, `detect_revert`) — DONE 2026-05-13.
2. ☑️ `.github/workflows/journal-append.yml` committed — DONE 2026-05-13.
3. ☑️ `.github/workflows/journal-aggregate.yml` committed — DONE 2026-05-13.
4. ☑️ `discipline.yml`'s per-push `invariant-lock` job updated to validate-only — DONE 2026-05-13.
5. ☑️ All 16 invariant_lock unit tests pass — DONE 2026-05-13.
6. ☐ **Operator enables GitHub merge queue + adds `journal-append` to required status checks.** Manual action; cannot be automated from this side.
7. ☐ **Synthetic test PR validates end-to-end.** Open a no-op PR that touches `docs/invariants/I9.md` (e.g., a typo fix in the body) plus `tests/coverage-matrix.yaml` (any no-op edit), merge via the merge queue, and verify a `journal-entry-<sha>` artifact appears in the Actions run.

Items 6 and 7 are the remaining steps to fully close DF16. Items 1–5 (the code work) are complete.

## Failure modes and mitigations

| Failure | Mitigation |
|---|---|
| `merge_group` event doesn't fire (merge queue disabled) | The `push: main` belt-and-suspenders trigger still runs; the artifact is produced post-merge. Mitigation note in the workflow comments. |
| `gh pr view` fails to fetch the PR body | Workflow falls through with an empty body; revert detection silently returns None. The primary entry is still uploaded. |
| Artifact upload races / GitHub Actions outage | The journal entry for that merge is lost. Mitigation: rerun the workflow via "Re-run jobs" on the run page; the journal-append workflow is idempotent (computes the entry from git state, not from any stateful side-effect). |
| Aggregator hits the 1000-artifact cap | Trigger again with `since-iso-date` set to bracket the next 1000. The cap is a safeguard against runaway runs, not a hard limit. |
| Multiple revert idioms in one PR body | `detect_revert` captures both `pr_number` and `sha`; the paired entry includes both. Multiple `Reverts #N` lines take the last match (consistent with the parser's `for line in body.lines()` pattern). |

## Open questions deferred to Story 1a.1 or later

- **Naming convention for the aggregation commit.** When the operator commits the aggregated journal, the commit message should probably link the run_id of the aggregator job. Decide at first aggregation.
- **Retention beyond 90 days.** GitHub Actions caps free-tier artifact retention at 90 days. If quarterly aggregation falls behind, entries could expire. Mitigation options: bump retention to 400 days (paid plan); aggregate more frequently; or accept that journal-of-record requires active operator engagement.

## References

- `df16-journal-merge-queue-design.md` (superseded) — Option (a) design, retained for history
- `1a1-adr-landing.md` — the Story 1a.1 strategy doc, now updated to reflect Option (c) implementation status
- Story 0.1 AC5 — the spec language
- Epic 0 retrospective — DF16 origin
- `xtask/src/invariant_lock.rs` — implementation
- `.github/workflows/journal-append.yml`, `journal-aggregate.yml` — implementation
