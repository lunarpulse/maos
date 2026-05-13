---
title: DF16 Resolution Design — Merge-Queue + Commit-Back Journal (superseded)
status: superseded
superseded_by: docs/dev-discipline/df16-resolution-option-c.md
superseded_on: 2026-05-13
authors:
  - Amelia (Senior Software Engineer)
proposes_decision: 2026-05-13 (post Epic 0 retrospective)
addresses: DF16 (`docs/invariants/journal.jsonl` never receives entries on merge)
binds: Story 0.1 AC5 spec ("the CI job that runs this is the merge queue job, not the per-push job")
blocks: Story 1a.1 PR-ready
---

> **Superseded.** Project Lead chose **Option (c) — per-merge CI artifact**
> instead of Option (a) — Merge-Queue + Commit-Back. Kept here for the design
> history; current resolution is in `df16-resolution-option-c.md`.

# DF16 Resolution — Merge-Queue + Commit-Back Journal (SUPERSEDED)

## Decision sought

Story 1a.1 cannot open its PR until the `invariant-lock` gate produces a verifiable journal entry on merge. The retrospective enumerated three options:

- **(a) Merge-queue + commit-back** (this document's recommendation)
- **(b) Out-of-repo append store** (S3 / GitHub Releases / separate repo)
- **(c) Per-merge CI artifact, indexed by SHA**

This document concretizes Option (a): exact workflow, exact token scope, exact failure modes. The architect's approval (Charlie / Winston persona) is the precondition for committing the workflow file.

## Design summary

Add a **merge-queue-triggered workflow** that runs after a PR is queued for merge, computes the journal entry from the merging PR's metadata, commits the entry to `docs/invariants/journal.jsonl`, and pushes the commit back to `main` via a narrow-scope GitHub App token. The merge-queue blocks merge until the journal-append job succeeds.

The existing per-push `invariant-lock` job continues to enforce the tri-requirement (diff + corpus delta + phase-commitment) as a **precondition** for merge. The new merge-queue job is the **commit-back** half — it does not re-validate the gate; it only persists the journal entry for invariants the per-push job already approved.

## Why a GitHub App and not a PAT

Branch protection on `main` requires ≥2 reviewer approvals. The merge-queue's commit-back step writes one trivial commit to `main` AFTER the PR's content has already been approved and merged. To bypass the "≥2 reviewer" rule for the journal-only commit, the pushing identity needs branch-protection bypass.

| Option | Bypass scope | Security surface |
|---|---|---|
| Personal access token (PAT) | Token-holder's entire write surface across all repos | Wide; PAT leakage = total compromise |
| GitHub App scoped to this repo | `contents: write` on this repo only | Bounded |
| GitHub App scoped to one path | `contents: write` only on `docs/invariants/journal.jsonl` | **Minimal** |

GitHub Apps do not natively support per-path scoping. The minimal-feasible option is therefore: **a custom GitHub App with `contents: write` permission scoped to this repo only**, with a webhook secret pinned, and the app's installation token used by the merge-queue workflow via `actions/create-github-app-token@v1` (or equivalent first-party action).

If the architect prefers stricter bounding, the workflow can additionally:

- Verify the only changed file is `docs/invariants/journal.jsonl` before pushing (xtask sanity check + workflow assertion)
- Verify the diff is append-only (no in-place line modification, no line removal)
- Reject any push attempt that includes content beyond the expected one-line append

These checks make the bypass surface effectively per-path, even though GitHub's permission model is per-repo.

## Workflow draft

This file would land at `.github/workflows/journal-append.yml`. **Not committed yet** — pending architect approval.

```yaml
name: journal-append

# Triggered by merge_group (GitHub merge queue). Runs after the PR is queued
# for merge but before the merge commit lands on main. If this job fails,
# the merge is abandoned.
on:
  merge_group:
    types: [checks_requested]

permissions:
  contents: read           # default; we use the App token for write

jobs:
  append-journal:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          ref: ${{ github.event.merge_group.head_sha }}
          fetch-depth: 2

      - uses: dtolnay/rust-toolchain@v1
        with:
          toolchain: stable

      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ hashFiles('**/Cargo.lock') }}

      # 1. Re-run invariant-lock against the merge-group head; if no invariants
      #    are touched, no journal entry is needed and we exit clean.
      - name: Detect touched invariants
        id: lock
        run: |
          git diff --name-only HEAD~1 > /tmp/changed-files.txt
          cargo run -p xtask -- invariant-lock \
            --changed-files /tmp/changed-files.txt \
            --pr-number ${{ github.event.merge_group.head_commit.message_pr_number || 0 }} \
            --sha ${{ github.event.merge_group.head_sha }} \
            --json > /tmp/lock-report.json

          # If no invariants touched, skip the commit-back.
          touched_count=$(jq '.touched_invariants | length' /tmp/lock-report.json)
          echo "touched_count=$touched_count" >> $GITHUB_OUTPUT

      - name: Generate GitHub App token
        if: steps.lock.outputs.touched_count != '0'
        id: app-token
        uses: actions/create-github-app-token@v1
        with:
          app-id: ${{ secrets.MAOS_JOURNAL_APP_ID }}
          private-key: ${{ secrets.MAOS_JOURNAL_APP_PRIVATE_KEY }}

      - name: Pre-flight — ensure journal is append-only
        if: steps.lock.outputs.touched_count != '0'
        run: |
          # The xtask should have appended exactly one line to journal.jsonl
          # during the invariant-lock run above. Verify:
          #   1. journal.jsonl is the only changed file
          #   2. The diff is purely an append (no in-place edits)
          changed=$(git status --porcelain | awk '{print $2}')
          if [ "$changed" != "docs/invariants/journal.jsonl" ]; then
            echo "REFUSING TO COMMIT: unexpected files changed: $changed"
            exit 1
          fi
          # Append-only check: the diff should have only + lines, no - lines
          # (and no + lines that are followed by a modification of the prior line).
          if git diff docs/invariants/journal.jsonl | grep -q '^-[^-]'; then
            echo "REFUSING TO COMMIT: journal.jsonl diff contains removals"
            exit 1
          fi

      - name: Commit and push journal entry
        if: steps.lock.outputs.touched_count != '0'
        env:
          GH_TOKEN: ${{ steps.app-token.outputs.token }}
        run: |
          git config user.name "maos-journal-bot"
          git config user.email "journal-bot@noreply.maos.dev"
          git add docs/invariants/journal.jsonl
          git commit -m "journal: append entry for ${{ github.event.merge_group.head_sha }}

[skip ci]

invariants: $(jq -r '.touched_invariants | join(\",\")' /tmp/lock-report.json)
PR: ${{ github.event.merge_group.head_commit.message_pr_number }}
"
          git push https://x-access-token:${GH_TOKEN}@github.com/${{ github.repository }} HEAD:${{ github.event.merge_group.head_ref }}
```

## Workflow semantics

1. **Trigger:** `merge_group: checks_requested` fires when a PR enters the merge queue. The merge queue blocks the actual merge to `main` until all required `merge_group`-triggered checks pass.

2. **Idempotency:** If two PRs are queued back-to-back, each gets its own `merge_group` event with a distinct `head_sha`. The journal entries are append-only and naturally ordered by merge time.

3. **No-op short-circuit:** PRs that touch no invariants (the common case) exit before generating the App token, so the bypass surface is exercised only when actually needed.

4. **Atomicity:** The `git add` + `git commit` + `git push` sequence is wrapped in pre-flight checks that refuse to commit if more than `journal.jsonl` was changed or if the diff contains removals.

5. **Failure modes:**
   - **App token unavailable:** Job fails; merge is abandoned. The architect sees a missing-secret error and re-runs after rotating.
   - **`journal.jsonl` write succeeds but push fails:** Job fails; merge abandoned. No half-state in `main`.
   - **Push race condition** (two merges race on the file): The second push fails with a non-fast-forward error; the merge queue retries the failed PR against the new `main`.

## Refactor required in `xtask/src/invariant_lock.rs`

The existing `append_journal` function (line 303) writes to the runner's working directory, which the workflow can then `git add` + `git commit`. **One refactor is required:** the function currently fires only when both `pr_number` AND `sha` are `Some`. The merge-queue event delivers both reliably, but the per-push job (which the new workflow ALSO calls for the touched-invariant detection in step 1) does NOT — and the per-push journal-write is now redundant.

**Recommended change:** add a `--write-journal` flag to the xtask CLI. The merge-queue workflow passes `--write-journal`; the per-push `discipline.yml` does not. This separates "validate" from "persist" cleanly. The pre-existing per-push behavior of running the journal-append in pull_request mode (which currently writes ephemerally and loses the data) gets deleted.

Concrete change:

```rust
// xtask/src/main.rs (Commands::InvariantLock variant)
InvariantLock {
    changed_files: Option<String>,
    pr_number: Option<u64>,
    sha: Option<String>,
    write_journal: bool,    // NEW: only the merge-queue job sets this
    json: bool,
},

// xtask/src/invariant_lock.rs::run signature gains write_journal: bool
// invariant_lock() at line 150 changes:
- if passed && sha.is_some() && pr_number.is_some() {
+ if passed && write_journal && sha.is_some() && pr_number.is_some() {
      append_journal(...)
  }
```

`.github/workflows/discipline.yml` — remove the journal-append side-effect from the per-push `invariant-lock` job (does not pass `--write-journal`). The merge-queue workflow is the sole journal writer.

## Acceptance criteria for DF16-closed

DF16 is considered closed when:

1. ☐ Architect (Charlie / Winston persona) approves this design (or proposes Option b / c with rationale).
2. ☐ GitHub App `maos-journal-bot` is created, installed on the repo, and its App ID + private key are committed as repository secrets (`MAOS_JOURNAL_APP_ID`, `MAOS_JOURNAL_APP_PRIVATE_KEY`).
3. ☐ `.github/workflows/journal-append.yml` is committed.
4. ☐ `xtask/src/invariant_lock.rs` is refactored to take a `--write-journal` flag and the per-push behavior is removed from `discipline.yml`.
5. ☐ A synthetic test PR (e.g., a no-op `docs/invariants/I9.md` cadence touch + corresponding `tests/coverage-matrix.yaml` no-op) is opened, merged via the merge queue, and the merge produces exactly one new line in `journal.jsonl` containing `invariant_ids: ["I9"]`, the correct PR number, and the merge SHA. The new line is verified as readable by `cargo run -p xtask -- invariant-lock` (the xtask can parse its own output).
6. ☐ Branch protection on `main` is updated to require the `journal-append` merge-queue check.

## Open questions for the architect

1. **Custom GitHub App vs first-party bot identity.** A custom App needs creation + secret rotation discipline. Is there a simpler first-party identity (e.g., a maintainer's PAT scoped via fine-grained tokens) that meets the ADR-037 sensitivity bar?

2. **`[skip ci]` directive.** The journal commit carries `[skip ci]` to avoid an infinite re-run loop. Is that acceptable, or should the journal commit be exempted from CI via a workflow `paths-ignore` rule instead?

3. **What happens on revert.** If a merged PR is later reverted, the journal entry for the original merge remains. Is that the intended semantic (journal is append-only, history-of-claims, not history-of-truths) or should reverts append a paired "reverted" entry?

4. **Multi-region runner concern.** GitHub's `merge_group` runs in their cloud only. If the project later self-hosts CI, the merge-queue trigger semantics may not port. Not blocking now; flag for v0.5+ consideration.

## References

- DF16 entry — `_bmad-output/implementation-artifacts/deferred-work.md`
- `1a1-adr-landing.md` — names this as a Story 1a.1 pre-flight blocker
- GitHub `merge_group` event reference — https://docs.github.com/en/webhooks/webhook-events-and-payloads#merge_group
- `actions/create-github-app-token@v1` — https://github.com/actions/create-github-app-token
- Story 0.1 AC5 — `_bmad-output/planning-artifacts/epics/epic-0-...md` (the spec that called for "merge queue job, not per-push job")
- ADR-037 — `docs/adr/ADR-037-constitutional-amendment-process.md`
