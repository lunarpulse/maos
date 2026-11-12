# Release decisions — 2026-11-12 (operator ruling: `ga` = boundary, `rc` = b, `kb` = keep)

Operator words: "we are not doing NFR-Sec-7 and NFR-Comp-1 as we do not have resources and
companies to do that. same applies to ops-* work as no resources, only AI agents we have."

This note records what the ruling changed in the release machinery and what it did not. It is
dated; it never replaces the text it amends (the 20-5 story's `## Operator ruling 2026-11-12`
section and decision rows D-20-5-17…20 are the register of record).

## What the ruling means

- **Holds 1–2 are claim boundaries, not GA preconditions.** No external penetration test
  (NFR-Sec-7) and no ECCN determination (NFR-Comp-1) will be performed. `v1.0.0` may be
  tagged without them; the release notes say plainly that neither was done, that assurance is
  AI-agent review plus the hermetic CI proof only, and that neither NFR is claimed met.
  WASM-engine-off-by-default stays in force. The signed-tag step needs the key holder — an
  operator action no agent performs.

  > **Dated note 2026-11-12 (landing-2 independent review F3; the sentence above is kept as
  > written):** "needs the key holder" is the operator's **policy**, not an enforced
  > control. The procedure cuts an ordinary `git tag` (`tag-procedure.md`), and
  > `release.yml` holds `RELEASE_SIGNING_KEY` itself: pushing an accepted tag
  > (`vX.Y.Z`, `vX.Y.Z-rc.N`, `v0.x.y-alpha.N`) on a commit that is an ancestor of
  > `origin/main` triggers CI-held artifact signing and publication. Nothing verifies a Git
  > tag signature, and with the `release` environment unprotected (no required reviewers)
  > nobody approves the run: whoever may push the tag can trigger signing without holding
  > the private key. The enforcement for "only the key holder" would be protecting the
  > `release` environment (required reviewers); it is **UNCLAIMED**, as is any
  > cryptographically signed Git tag.
  >
  > **Dated note 2026-11-12 (operator ruling 13, `r3` answered; the notes above are kept as
  > written):** the operator's free text "완료되면 태그 push" ("push the tag when it is
  > complete"; `_bmad-output/planning-artifacts/decisions-2026-11-12-acceleration.md` item 13)
  > authorizes the orchestrator to cut and push exactly one ordinary tag, `v1.0.0-rc.1`, once
  > the whole `tag-procedure.md` pre-tag checklist is green at the candidate SHA (conditions
  > a-e in item 13). That explicit per-tag authorization is how the key-holder **policy** above
  > is satisfied for this tag; it is not an enforcement change. There is still no Git tag
  > signature (the tag is an ordinary `git tag`); artifact signing (`SHA256SUMS`,
  > `SHA256SUMS-packages`) runs in CI with the secret-held `RELEASE_SIGNING_KEY`; `release`
  > environment protection stays **UNCLAIMED**. Force push, moving, deleting or re-cutting the
  > tag, any other tag and history rewrite remain per-event asks. At the time of this note the
  > hosted discipline run for the candidate SHA, the `release.yml` step-6b rehearsal and the T3
  > `fuzz-prerelease` chain are **not yet run**; the final run ids and the tag result are
  > recorded in a post-tag record commit.

- **The rc ships with perf verdicts ABSENT (rc = b, D-20-5-09 B).** The 20-7 L2 binding phase
  needs a bare-metal `maos-perf` runner; it stays a named, **UNCLAIMED** row. Money, account
  and hardware rows (Docker Hub, brew/AUR, live vendor credentials, the perf runner, an
  external pen-test) stay named and `absent` in
  [`provisioning-checklist.md`](../runbooks/provisioning-checklist.md); none is claimed done.
- **Kernel shared STOP unchanged** (`kb`): ask again before any quarantine-closure work.

## What changed in the machinery (landing 2, kernel-free)

| # | Decision | Where | Behaviour |
|---|---|---|---|
| 1 | Perf leg in the release precondition | `xtask/src/check_release_precondition.rs`, `.github/workflows/release.yml` | `check-release-precondition --perf-verdicts <json> [--perf-absent-ok]` reads `check-perf-verdict --json`. FAIL / INDETERMINATE are always refused; ABSENT is refused unless `--perf-absent-ok`; otherwise every row must be PASS. The JSON must parse, name the released SHA and carry rows, so a tool crash is never read as ABSENT. `release.yml` passes `--perf-absent-ok` for kind `rc` **only** (GA still needs PASS) and fetches the `perf-ledger` branch at its logged tip; an HTTP 404 on the branch is an empty ledger (every metric ABSENT), any other failure refuses. The alpha line has no perf leg. |
| 1 | `perf-verdict-label` leg | `xtask/src/check_rc_readiness.rs` | The notes file `docs/release/v<workspace version>.md` must exist and carry the exact line `Perf verdicts: ABSENT`. No notes, another value (including `PASS`) or no line is red. The notes file itself is the final rc commit's job. |
| 2 | `release` environment leg is a labelled WARN | same | Row `release environment protection evidence` `present`: nothing more. Row `absent`: passes with detail `WARN: environment protection not provisioned: claim boundary`, but then the notes must carry the exact line `Release environment protection: NOT PROVISIONED` (else red). Any other state or a missing row stays red. |
| 3 | Leg (c) and 20-7 | same | Named waiver `WAIVED_KEY = 20-7-stable-runner-perf-gates`: accepted at `in-progress` only (never backlog / ready-for-dev / review), only when the workspace version is `X.Y.Z-rc.N` **and** the `perf-verdict-label` leg passes, in both modes (with and without `--tag`); the detail prints `WAIVED`. A GA or any non-rc version never gets it. The 20-7 key is not flipped to done. |
| 4 | Wording | `xtask/src/stability_matrix.rs` (placeholder, STABILITY.md regenerated by `xtask stability-matrix`), comments in `check_export_control.rs`, `gate_common.rs` | The Holds are described as claim boundaries; no check was weakened. |
| 5 | Docs | `docs/pen-test/scope.md`, `docs/pen-test/triage-protocol.md`, `docs/runbooks/provisioning-checklist.md`, PRD `project-scoping-phased-development.md` and `non-functional-requirements.md`, the 20-5 story, `tag-procedure.md` | Dated ruling notes; no requirement text deleted. |

Dated note 2026-11-12 (landing 3, review corrections F1/F2; the table above is kept as
written): in row 1 only the **branch lookup's** HTTP 404 is an absent `perf-ledger` branch;
once the branch exists, any failure fetching `ledger.jsonl` (404 included) refuses
publication ("release refused: cannot read the perf-ledger"). A new rc-only `release.yml`
step after the aggregate step refuses an rc whose notes carry the exact line
`Perf verdicts: ABSENT` while the computed verdict report has any PASS, FAIL or
INDETERMINATE row ("notes say ABSENT but the ledger has PASS/FAIL/INDETERMINATE verdicts:
update the notes"). No exact label line is that step's no-opinion case (the readiness gate
refuses a missing label). Both are exercised only as executed shell with stand-in `gh`/`cargo`;
nothing in `release.yml` has run hosted at the rc version.

## Release-notes labels the gates read (final rc commit)

`docs/release/v1.0.0-rc.1.md` must carry, each alone on its line:

```
Perf verdicts: ABSENT
Release environment protection: NOT PROVISIONED     (only while the row is still `absent`)
```

and the claim-boundary text (no external penetration test; no ECCN determination; the perf
verdicts and `release` environment protection are not claimed).

## GitHub facts read 2026-11-12 (read-only, `gh`)

The `release` environment is **absent** (a job that names `environment: release` makes GitHub
create it unprotected on first use). Secrets `CI_EVIDENCE_AUDIT_KEY`, `MAOS_RELEASE_PUBKEY`
and `RELEASE_SIGNING_KEY` exist. Branches `fuzz-ledger` and `rto-ledger` exist; `perf-ledger`
does not (`GET .../branches/perf-ledger` → HTTP 404, which `release.yml` reads as an empty
ledger). `GET .../commits/perf-ledger` returns **422** for a missing ref, which is why the
workflow queries `branches/`, not `commits/`, for the perf ledger.

## T3 fuzz chain: prepared, NOT dispatched

The chain (`fuzz-prerelease.yml`: 3 targets × 5 serial 5 h links, ≥ 86,400 libFuzzer seconds per
target, ~26 h wall [INFERENCE: 5 × 5 h + setup]) must run **at the rc commit**, which does not
exist yet (the version bump to `1.0.0-rc.1` and the real notes are the final 20-5 commit, after
21-5). Dispatching now would fuzz a SHA that is never tagged, and the release precondition
matches the chain to the exact tagged SHA. Nothing below was run.

Preconditions (all must hold before the dispatch):

1. The final 20-5 commit (version `1.0.0-rc.1`, `Cargo.lock`, STABILITY.md, notes with the
   labels above) is on the remote `main` tip: `test "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)"`.
2. `.github/workflows/fuzz-prerelease.yml` is on `main` (a `workflow_dispatch` exposes only
   workflows on the default branch) and `fuzz-ledger` exists with write access for the
   workflow's own token (`contents: write`; the branch exists, measured 2026-11-12).
3. The `gh` token can dispatch workflows (`actions: write`); the CI-log read token on the dev
   host cannot (dispatch refused, 403, by design) — dispatch needs a token that may.
4. Nothing else is pushed to `main` until the chain completes, or the chain no longer matches
   the tag SHA.

Command (at the candidate SHA, then select the run by commit, never by recency):

```bash
candidate_sha="$(git rev-parse origin/main)"
gh workflow run fuzz-prerelease.yml --ref main
run_id="$(gh run list --workflow fuzz-prerelease.yml --branch main \
  --json databaseId,headSha --jq "map(select(.headSha==\"$candidate_sha\"))[0].databaseId")"
gh run watch "$run_id" --exit-status      # `t3-ledger-collect` must be green
```

Then the precondition reads the result from the ledger branch (see
[`tag-procedure.md`](tag-procedure.md) step 2).

## Still unclaimed (named, not done)

`ops-rc-signed-tag` and `ops-ga-tag-after-holds` (the signed tag needs the key holder);
`ops-stable-perf-runner` and the 20-7 L2 binding phase (bare-metal runner); `release`
environment protection (required reviewers, tag restrictions); Docker Hub / brew / AUR
publication; live vendor credentials; the external cohort. The hosted legs of 20-5 (the release
workflow run, the recipe `ci-20-5-verify.yml`) remain unverified.
