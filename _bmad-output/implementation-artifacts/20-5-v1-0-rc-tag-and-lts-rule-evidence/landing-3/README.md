# 20-5 landing 3 — correction round for the landing-2 independent review (FixRel205)

Review: `../landing-2/review/gpt-review.md`. No xtask SOURCE changed (only `xtask/tests/*`); `scripts/repin.py --check` = IN SYNC.

| Finding | Fix | Witness (executed shell, `xtask/tests/release_gates_20_5r1.rs`) |
|---|---|---|
| F1 P2 | `release.yml` "Require successful main discipline aggregate": the `perf-ledger` branch lookup is judged alone (stderr of that call only); only ITS `HTTP 404` creates the empty ledger; once the branch exists any `contents/ledger.jsonl` failure (404/403/5xx) prints `release refused: cannot read the perf-ledger`, exit 1, before `check-perf-verdict`/`check-release-precondition`. | `an_absent_perf_ledger_branch_is_an_empty_ledger_and_any_other_failure_refuses` (a: branch 404 → empty ledger; c: branch 403 / rate-limit / 5xx → refuse), `a_perf_ledger_branch_that_exists_but_cannot_be_read_always_refuses` (b: contents 404; d: contents 403 / 5xx) |
| F2 P2 | New rc-only step "Require the rc notes perf label to agree with the perf verdicts (rc tags only)" after the aggregate step (python3 stdlib heredoc): notes carrying the exact line `Perf verdicts: ABSENT` (`rstrip()`-equal, as `has_line`) + any non-ABSENT verdict row → `release refused: notes say ABSENT but the ledger has PASS/FAIL/INDETERMINATE verdicts: update the notes ...`; no exact label line = no opinion; label + unreadable/empty/malformed report → refuse. | `the_label_step_is_an_rc_only_step_...`, `an_absent_label_must_match_...`, `an_absent_label_with_any_real_verdict_is_a_refused_contradiction`, `without_the_exact_label_line_this_step_has_no_opinion`, `an_absent_label_with_an_unreadable_or_empty_report_...`, `the_real_check_perf_verdict_json_is_what_the_label_step_reads` (real `check-perf-verdict --json` round trip) |
| F3 P3 | Dated notes 2026-11-12 in `docs/release/decisions-2026-11-12.md`, `docs/release/tag-procedure.md`; `docs/release/v1.0.0-rc.1.md` wording + claim-boundary row 6 + `ops-rc-signed-tag` row. Both exact label lines untouched. | docs only |
| T3 prefix | `release_precondition_t3_20_5.rs::a_full_sha_sharing_a_prefix_with_the_released_sha_is_another_commit` (shared 7/12/39 chars, full 40-hex SHAs) | — |

Logs: `failing-before.log` (7 new tests red against the unfixed workflow, incl. F1 (b)), `green-focused.log`, `final-focused.log` (76/76), `gates.log`, `repin-check.log`, `plants/` (each plant restored with copy-back + `cmp` + `touch`; the log ends with the restore line):

- `p1-f1-revert-shared-404-handler` → `a_perf_ledger_branch_that_exists_but_cannot_be_read_always_refuses` red
- `p2-f2-step-removed` → 6 label-step tests red
- `p3-f2-only-fail-indeterminate-counted` → `an_absent_label_with_any_real_verdict_is_a_refused_contradiction` red
- `p4-f1-any-contents-failure-is-empty` → `a_perf_ledger_branch_that_exists_but_cannot_be_read_always_refuses` red

UNVERIFIED: hosted behaviour of `release.yml` (nothing in it has run hosted at the rc version); real `gh` error text for 404/403/rate-limit is stubbed (the 404 test relies on `gh` printing `HTTP 404`); python3 availability on the hosted runner is assumed (ubuntu-latest).
