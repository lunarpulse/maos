---
title: Story 1a.1 — 14-ADR Landing Strategy
status: decided
decided_on: 2026-05-13
decided_in: Epic 0 retrospective (Step 9 critical-prep)
authors:
  - Charlie (System Architect — Winston persona)
approvers:
  - Lunarpulse (Project Lead)
binds: Story 1a.1 (Initialize 17-Crate Cargo Workspace + Frozen ABI Types)
supersedes: none
---

# Story 1a.1 — 14-ADR Landing Strategy

Story 1a.1 lands the **14 binding-v0.1 ADRs** simultaneously per the Epic 1a goal: ADR-001, 002, 004, 006, 010, 011, 012, 014, 022, 023, 026, 030, 032, 037. It also codifies invariants I1–I14 as types in `maos-domain`, which means it touches `docs/invariants/I*.md` registers to add the `v0.1: CI` cadence row reflecting type-codification.

This document captures the decision for *how* Story 1a.1 lands those changes through the `invariant-lock` CI gate (ADR-037) shipped in Story 0.1 and first dogfooded in Story 0.2.

## The risk

The `invariant-lock` gate enforces three tri-requirement legs per PR that touches `docs/invariants/I*.md`:

1. machine-checkable diff against the invariant set
2. corpus delta (`tests/coverage-matrix.yaml` modified in the same diff)
3. phase-commitment update (the touched `I*.md`'s enforcement-cadence table modified)

PLUS ≥2 maintainer sign-offs and a forward-only cadence-progression rule (`— → CI → runtime → fuzz`, never backwards).

Story 0.2 was the first dogfood — touched I9 only and we found bugs in `parse_cadence` during review (indentation sensitivity, eventually fixed via `line.trim()` + phase-key validation). The gate has not been exercised against a PR touching 14 invariant register files in one diff.

## Decision

**Land Story 1a.1 as a single PR with one aggregated `invariant-lock` decision at merge-gate, NOT as 14 separate PRs or 14 sub-commits.**

### Mechanics

- **One PR** for the entire 1a.1 deliverable (17 crates + 14 ADRs + I1–I14 type codification + `invariant-lock`-relevant register edits).
- **The PR description** explicitly enumerates the touched-invariant set (`invariant_ids: [I1, I2, ..., I14]`) so reviewers can audit the surface in one place.
- **One corpus-delta**: `tests/coverage-matrix.yaml` is touched once with a coherent edit (e.g., flipping rows for FR1, FR2, FR7, FR8, FR47, FR48, FR61 from `gates: []` to populated where Story 1a.1 stories ship the gate; **bulk-edit, single diff hunk per FR**).
- **One phase-commitment block**: all 14 `I*.md` files gain a `v0.1: CI` (or `v0.1: runtime` for runtime-codified invariants) row in their `enforcement_cadence:` frontmatter — additive only, no regressions; `parse_cadence` regression-check verified against the multi-invariant case before PR opens.
- **Two reviewers approve the PR once**; ADR-037's "≥2 maintainer sign-offs" is a PR-level requirement, not a per-invariant requirement. The journal entry captures all 14 `invariant_ids` in a single line.

### Pre-flight requirement

Before opening the PR, **exercise the gate against a 14-invariant test fixture**:

- Create a `xtask/tests/fixtures/clean-invariant-lock-14/` fixture with 14 synthetic `I*.md` files, each carrying an additive cadence row.
- Run `cargo run -p xtask -- invariant-lock --changed-files <fixture-list> --pr-number 0 --sha test` against it and assert it passes.
- Run a `violation-invariant-lock-14-regression/` fixture with one of the 14 files carrying a backward cadence step and assert the gate fails with a specific `<I_n>` cited in the error message (not a generic "regression detected").

If `parse_cadence` or the regression-check logic breaks on the 14-invariant case, **fix the gate first in a separate PR before opening 1a.1**. The journal append must be atomic; if the merge-gating job partially succeeds (some invariant_ids in the journal entry, others dropped), Story 0.1's AC5 contract is violated.

### Story 0.2 dogfood gap — STRUCTURAL, deeper than initially understood

Initial framing in the Epic 0 retro: "`journal.jsonl` is 0 bytes despite Story 0.2 AC5 requiring a merge-append." Investigation 2026-05-13 (Epic 0 retro Step 9 critical-prep #3) revealed this is a **structural gap, not a missed merge call**:

1. **No merge-queue job exists.** `.github/workflows/discipline.yml` defines `invariant-lock` as a per-push / per-PR job, not the merge-queue job Story 0.1 AC5 explicitly required ("the CI job that runs this is the merge queue job, not the per-push job — so the journal entry corresponds to the merged SHA").

2. **The `append_journal` function writes to ephemeral runner FS, not to the repo.** Even when the function fires, the written line vanishes when the runner shuts down. There is no `git add` + `git commit` + `git push` anywhere in the workflow to persist the change.

3. **The function only fires on `pull_request` events.** The workflow passes `pr_number` from `${{ github.event.pull_request.number }}` — empty on `push: main`. So the post-merge code path is skipped entirely; the `pull_request` path runs but its write is ephemeral.

4. **Verified empty in all history:** `git log --all --oneline -- docs/invariants/journal.jsonl` returns only the initial-creation commit (`70f45b0`); no merge has ever produced an entry.

**Tracked as DF16** in `_bmad-output/implementation-artifacts/deferred-work.md`.

### DF16 resolution (DECIDED 2026-05-13)

**Option (c) — Per-merge CI artifact** chosen over Options (a) and (b). See `docs/dev-discipline/df16-resolution-option-c.md` for the full design.

**Implementation status (2026-05-13):**

- ☑️ **Code:** xtask refactor + revert detection + 16 unit tests pass.
- ☑️ **`.github/workflows/journal-append.yml`** committed (merge_group + push:main triggered, uploads `journal-entry-<sha>` artifacts).
- ☑️ **`.github/workflows/journal-aggregate.yml`** committed (manual operator dispatch, downloads + concatenates artifacts).
- ☑️ **`discipline.yml`** per-push `invariant-lock` job updated to validate-only.
- ☐ **Operator action:** enable GitHub merge queue + add `journal-append` to required status checks (manual, in repo Settings UI).
- ☐ **End-to-end verification:** synthetic test PR confirming the artifact is uploaded on merge.

### Pre-flight gate for Story 1a.1

Before Story 1a.1's PR opens:

1. ☐ The operator must enable the GitHub merge queue on `main` and add `journal-append` to the required status-checks list. This is a Settings UI action; cannot be done from code.
2. ☐ A synthetic test PR (e.g., a typo-fix to `docs/invariants/I9.md` plus a no-op edit to `tests/coverage-matrix.yaml`) must be merged through the merge queue, and a `journal-entry-<sha>` artifact must appear in the Actions run.
3. ☐ The 14-invariant fixture pre-flight (DF17) must verify the gate handles 14 simultaneous invariants. **Status: COMPLETED 2026-05-13** — `xtask/tests/fixtures/{clean,violation}-invariant-lock-14*` committed; multi-invariant unit tests pass.

If items 1–2 do not land before 1a.1 is technically ready, the dev agent **MUST NOT open the 1a.1 PR** — the journal entry for 14 invariants is part of AC5's tri-requirement, and shipping 1a.1 without the journal mechanism running end-to-end violates the spec.

## Anti-decisions (explicitly rejected)

- **NOT 14 separate PRs.** Story 1a.1 is a coherent workspace bootstrap; splitting it artificially defeats the "land the substrate in one shape" intent and creates 14× the reviewer overhead.
- **NOT 14 sub-commits in one PR.** ADR-037's tri-requirement is a PR-level contract; per-commit invariant-lock cycles would either deadlock (each commit lacks the corpus-delta that the next commit provides) or require disabling the gate mid-merge.
- **NOT a "merge with `invariant-lock` waived" PR description.** No `--no-verify` mode exists for this gate; if the dogfood reveals a gate bug, fix the gate, do not work around it.

## Open risks (accepted)

- **Reviewer pool size.** ADR-037 requires ≥2 active maintainers; at this stage, the named pool is small. The two reviewers for the 1a.1 PR are pre-committed: Lunarpulse + one additional maintainer (TBD before 1a.1 starts).
- **PR size.** 1a.1 will be a large PR (~2–3 KLOC + 14 ADR markdown files + 14 invariant register edits + ABI types). Reviewers are expected to chunk the review: ADR set first, type codification second, workspace structure third. The PR description provides reviewer-suggested reading order.
- **`cargo-public-api` migration.** Action item D2 of the Epic 0 retro targets migrating `xtask/abi_diff.rs` before Story 1b.4's schema freeze. 1a.1 lands against the syn-based parser; if 1a.1's ABI surface exposes parser brittleness, escalate D2 to "before 1a.1 merges" and pause.

## Validation criteria

Story 1a.1 is considered "1a.1-PR-ready" when:

- [ ] The pre-flight 14-invariant fixture passes against the existing gate.
- [ ] The `journal.jsonl` append path is verified working end-to-end on a synthetic test PR.
- [ ] Two reviewers are pre-committed (named in the PR description draft).
- [ ] The PR description enumerates the 14 invariant_ids and the reviewer reading order.
- [ ] The phase-commitment block is drafted (which invariants get `v0.1: CI` vs `v0.1: runtime`) and reviewed against Architecture §3.2.1 cadence matrix.

## References

- Story 0.1 AC5 — `invariant-lock` CI gate definition
- Story 0.2 AC5 — first dogfood; lessons applied here
- ADR-037 — constitutional amendment process (committed to `docs/adr/`)
- Architecture §3.2 + §3.2.1 — invariants I1–I14 and their enforcement-cadence matrix
- Epic 0 retrospective (`_bmad-output/implementation-artifacts/epic-0-retro-2026-05-13.md`) — origin of this decision
