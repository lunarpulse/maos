# Story 20-5 closure note — 2026-11-12 (`Close205Records`, records only)

Story: `../../20-5-v1-0-rc-tag-and-lts-rule.md` (Status `done`). Code base: commit `23b2200b` (L2 code half). This note
cites only files and commits read for this record; nothing here was re-run except the gates in the last section.

## Claim boundary

`done` = the story's engineering scope landed, with independent review. It is **not** a claim that `v1.0.0-rc.1` is
tagged, published or hosted-verified. The tag, the hosted proofs and the operator-lane rows below are UNVERIFIED /
UNCLAIMED.

## AC by AC

| AC | Evidence (paths under `20-5-v1-0-rc-tag-and-lts-rule-evidence/` unless stated) | Boundary |
|---|---|---|
| AC1 (command, version) | `Cargo.toml` `version = "1.0.0-rc.1"` + five lock files in `23b2200b`; STABILITY.md regenerated; `landing-3/repin-check.log`; prebuilt `check-rc-readiness --json` at this tree (read 2026-11-12): `version`, `stability-matrix`, `provisioning` (WARN), `deny-ignores`, `perf-verdict-label` PASS; `epic-20-stories` red on 20-9 only. Failing-before/plant for legs a, c, e: `landing-1/ac5-3-real-tree.log`, `landing-1/plant-ac*.log`. `release.yml` keeps `prerelease: contains(github.ref_name, '-')`. Refusal of a SHA without a T3 chain, verbatim: `landing-1/ac4-6-real-ledger.log` (SHA `f56959c2`). | The candidate's own run: `closure/rc-readiness-final.json` (saved by the orchestrator; not authored here). `.deb`/air-gap lines of the hosted `release-dry-run` and `epic-20-exit` on the candidate: **not read, UNVERIFIED**. Local `check-rc-readiness --tag` green is not claimed. |
| AC2 (LTS rule) | `landing-1/evidence.md` AC2; `docs/release/lts-policy.md`; `SECURITY.md:56,60` and PRD DELTA in `23b2200b` (1 year from `v1.0.0`; 2-year term = v1.5 decision) | The commitment starts at the `v1.0.0` tag (`ops-ga-tag-after-holds`). |
| AC3 (`[cli_wrapper]` schema version) | `landing-1/evidence.md` AC3; `landing-1/review-corrections/evidence.md` | ABI ratification row `20-5-cli-wrapper-schema-version` is still `proposed` in `xtask/abi-ratifications.toml:320-328` (operator ratifies; new deferred row). |
| AC4 (T3 precondition) | `landing-1/evidence.md` AC4, `landing-1/ac4-6-real-ledger.log`; `landing-3` SHA-prefix witness in `xtask/tests/release_precondition_t3_20_5.rs`; `landing-3/final-focused.log` (76/76) | The chain itself has never run green: no `fuzz-prerelease` run at the candidate SHA (about 26 h [INFERENCE]); the only prior dispatch, run 38926626626 (2026-10-16), failed. UNVERIFIED. |
| AC5 (`check-rc-readiness`) | `landing-1/evidence.md` AC5, `landing-1/ac5-3-real-tree.log`; landing 2: `landing-2/evidence.md` (45 tests, 16 red before, ten plants p1-p10); `landing-1/review-corrections/` (release-environment row, tag classification) | No per-commit hosted run (deferred row, owner `ops-rc-signed-tag`). `ci/ci-20-5-verify.yml` never run. |
| AC6 (docs truth) | `landing-1/evidence.md` AC6; `landing-1/docs-site-*.log` (gate:routes 123 passed, gate:static PASS, a11y distinct pages with `expected_distinct_pages` 176, local Chromium) | Hosted docs-site run of the 44 `--` routes: UNVERIFIED. |
| AC7 (deny re-check) | deferred rows for RUSTSEC-0370/0371/0372 (re-checked 2026-11-11), `serde_cbor`, `rustls-pemfile`; leg `deny-ignores` PASS in the read above | Upstream triggers not met; rows stay open. |
| AC8 (notes) | `docs/release/v1.0.0-rc.1.md` (`23b2200b`): exact lines `Perf verdicts: ABSENT` (:27) and `Release environment protection: NOT PROVISIONED` (:28); `release.yml` `body_path: docs/release/${{ github.ref_name }}.md` + rc-only guard; F2 reconciliation step; `landing-3/README.md` | The notes file was not edited by this closure. `release.yml` has never run hosted at the rc version. |
| AC9 (records) | story File List/Completion Notes/Change Log; `sprint-status.yaml` 20-5 `done`, `ops-rc-signed-tag` stays `backlog` (no `ops-*` row flipped); deferred rows below; dated notes in `docs/release/decisions-2026-11-12.md` and `tag-procedure.md` | Pins at L2: `landing-3/repin-check.log` = IN SYNC; kernel unchanged. |

## Reviews (substitution disclosed)

GLM quota is exhausted until 2026-11-15, so these reviews ran on the GPT `reviewer` (a different family from the
Anthropic implementer). They are not a security-specialist review.

| Review | Result | Disposition |
|---|---|---|
| `landing-1/review/gpt-review.md` (GPT) | 3 P2 | fixed in `e322c7ad` (failing-before + plants), `landing-1/review-corrections/evidence.md` |
| `landing-1/review/security.md` (GLM-5.3, earlier) | 1 HIGH, 4 MEDIUM, 4 LOW, 1 INFO | fixed or deferred with rows (`deferred-work.md` rows owned by `ops-provisioning-secrets-and-accounts`, F5, F9, F10) |
| `landing-2/review/gpt-review.md` (GPT) | P0 0, P1 0, P2 2, P3 1 | all fixed in landing 3 (F1 branch-only 404 = empty ledger; F2 ABSENT label vs verdicts; F3 policy-not-enforcement wording); 7 failing-before, 76/76, 4 plants |
| `landing-3/review/gpt-review-final.md` (GPT) | P0 0, P1 0, P2 0, P3 1 (stale docs-site version rows) | fixed in `23b2200b` |

The final review notes that the aggregate-shell harness stubs `cargo` (not an end-to-end release test) and that
real `gh` 404 text is stubbed; refusal, not fail-open, results from a diagnostic without the token.

## Rulings used

`_bmad-output/planning-artifacts/decisions-2026-11-12-acceleration.md`: item 8 (`r1`, 20-7 rescope), item 9 (`r2`,
agents dispatch the T3 chain), item 10 (`r3` ask), item 11 (`q20`), item 12 (`p4`), item 13 (operator "완료되면 태그
push": the orchestrator pushes the ordinary tag `v1.0.0-rc.1` once the whole pre-tag checklist a-e is green at the
candidate SHA; anything else, including force push, re-cut, other tags, is a per-event ask). Story-level: `ga`, `rc` = b
(D-20-5-09 B), `kb`; D-20-5-17...20.

## Deferred rows owned by 20-5 (re-derived; before -> after)

| Line (HEAD before this closure) | Row | After |
|---|---|---|
| 197 | NFR-Maint-6 table row, `Owner: 20-5` | `Owner: ops-ga-tag-after-holds`; published rule CLOSED @23b2200b |
| 1265 | `[cli_wrapper]` schema version | CLOSED @23b2200b (local evidence); ABI ratification split into a new row (ownerless, dispositioned by D-20-5-04 A) |
| 1276 | published examples | CLOSED @23b2200b on local evidence |
| 1299 | variant error pages | CLOSED @23b2200b on local evidence; hosted run UNVERIFIED |
| 1388 | zbus ignores | ownerless and open, dispositioned by D-18-0-03 / D-20-5-10 A; upstream trigger |
| 1549 | F13 persist-credentials | `Owner: ops-provisioning-secrets-and-accounts` |
| 1587 | binary size attribution | ownerless and open, dispositioned by operator ruling `size` |
| 1724 | `check-rc-readiness` hosted run | `Owner: ops-rc-signed-tag` |
| 1727 | ledger trust anchor follow-up (cross-check) | follow-up re-owned to `ops-provisioning-secrets-and-accounts` |

Rows 1722, 1723, 1725, 1726, 1728, 1729, 1730 name 20-5 only as author or are already ownerless-dispositioned /
owned elsewhere; unchanged. Line 1346 is a historical FIXED sub-bullet (18-0 closure): only its pointer "owner 20-5" was reworded to "filed under 20-5 at the time; the zbus row carries the current disposition" because the stale-owner gate reads it.

## UNVERIFIED / not done at this record

- Hosted discipline run for `23b2200b` and for the final candidate SHA; `release-dry-run` `.deb`/air-gap lines.
- `release.yml` has never run hosted at the rc version; the step-6b `workflow_dispatch` rehearsal (incl.
  `aarch64-apple-darwin`) is pending.
- The T3 `fuzz-prerelease` chain at the candidate SHA (about 26 h [INFERENCE]).
- The tag push (authorized by ruling 13 only after the whole checklist is green).
- `release` environment protection: UNCLAIMED. No Git tag signature exists or is claimed; artifact signing runs in CI.
- Perf verdicts: ABSENT (ruling rc = b; 20-7 L2 binding phase needs a bare-metal runner, `ops-stable-perf-runner`).
- ABI ratification record: `proposed`.
- Candidate `check-rc-readiness --tag v1.0.0-rc.1 --json`: saved by the orchestrator at `closure/rc-readiness-final.json`.
- Final hosted run ids, chain run id and tag result: a post-tag record commit.

## Correction 2026-11-12: `done` was recorded over a red hosted run

20-5 was recorded `done` while the hosted discipline run on its L2 tree (`23b2200b`, run 43215484029) was red. The non-green jobs, checked with `gh run view 43215484029 --repo lunarpulse/maos --json jobs`, are `check-fmt`, `nfr-rel-11-halt-receipt-999pct` and the fan-in `aggregate`.

- `check-fmt` failed on rustfmt hunks in 20-5's own `xtask/tests/release_gates_20_5r1.rs`; commit `582f9509` fixes them and was not yet pushed when this correction was written.
- `nfr-rel-11-halt-receipt-999pct` failed on the same `ac4_v3b` failure (`root_shutdown_unload_16_3::ac4_v3b_...`) that is recorded in the 20-9 closure note; its fix is owned by agent FixAc4v3b, not by 20-5.
- Root cause of the `check-fmt` miss: the local commit routine lacked `cargo fmt --all -- --check`. It is now added (Epic 20 retrospective action E20-A1, `epic-20-retro-2026-11-12.md:206`).
- No acceptance criterion of 20-5 is changed and the `done` status is unchanged. The hosted run for the final candidate SHA remains listed under UNVERIFIED above.
