# What `v1.0.0-rc.1` requires — measured at `73e7afbc` (2026-11-04)

The tag itself is `ops-rc-signed-tag` (`sprint-status.yaml:349`). 20-5 makes the tree tag-ready and makes the
release job refuse an unready SHA; it never signs, tags or publishes. States below are the tracker's (authoritative)
plus read-only CI/ledger facts (`cites-73e7afbc.md` §O). Recommended options are PROPOSED until the operator rules.

## 1. Engineering closure (every Epic 20 story `done` on the tagged tree; `check-rc-readiness` leg `epic-20-stories`)

| Key | State at HEAD | What still gates the rc |
|---|---|---|
| 20-1a registry exit path | ready-for-dev (landings on main) | `done`; `epic-20-exit` lines 5–9 green |
| 20-1b vetting / yank / quarantine | backlog (starts after 20-1a done) | `done` (kernel L2 last) |
| 20-2 deb / air-gap / Docker split | ready-for-dev | `done`: `.deb` amd64 + arm64, air-gap variant, `SHA256SUMS-packages` + sig in `release.yml`; `container.yml` skipped-or-green without Docker Hub secrets |
| 20-3a / 20-3b / 20-3c | done | — |
| 20-4 Gemini + endpoint pin | ready-for-dev (L1 on main) | `done` (L2 kernel last); paid recording is `ops-live-gemini-leg`, not an rc gate (D-20-5-12 A) |
| 20-6 macOS guard | done | the pre-tag darwin rehearsal stays mandatory (`tag-procedure.md:69-86`) |
| 20-7 stable-runner perf gates | in-progress (L1 on main) | L2 `done`: every binding verdict `PASS` for the tag SHA (20-7 AC11.2; D-20-5-09 A) — needs `ops-stable-perf-runner` + 20-7 §O O1–O7 + ≥ 2 weeks / 20 main runs |
| 20-8, 20-9, 20-10, 20-11, 20-13, 20-14 | ready-for-dev | `done` |
| 20-12, 20-15, 20-16 | done | — |
| **20-5** | backlog → ready-for-dev (this creation) | L1 (kernel-free tooling, docs, schema) any time; L2 (version bump, release notes, readiness green) after every row above is `done` (D-20-5-01 B, -14 A) |

## 2. CI evidence for the exact tag SHA (release job; `check-release-precondition`)

| Evidence | State at HEAD | Owner of the check | Owner of producing it |
|---|---|---|---|
| discipline `aggregate` completed/success | checked today (`check_release_precondition.rs:48-79`) | 15-4 | CI on push to main |
| T3 24 h fuzz chain, three targets, five links each, ≥ 86 400 libFuzzer s, one run, `commit == SHA` | **absent**: 0 T3 records in `fuzz-ledger` (`c1de4004`); one dispatch (38926626626) failed | **20-5 AC4** | **operator** dispatches `fuzz-prerelease` at the candidate SHA (D-20-5-07 A; the agent token is read-only) |
| perf-ledger binding verdicts all `PASS` | **absent**: no `perf-ledger` branch, 0 runners | 20-7 AC11.2 (shares `check_release_precondition.rs`) | `ops-stable-perf-runner` + 20-7 L2 |
| `cargo deny check` (fresh DB) | blocking per commit (`discipline.yml:164-189`) | 18-0 | CI |

## 3. Operator-lane items (never in an engineering exit; agents cannot do these)

| Item | Row / owner | State measured | rc effect (D-20-5-12 A) |
|---|---|---|---|
| `RELEASE_SIGNING_KEY`, `MAOS_RELEASE_PUBKEY` | `ops-provisioning-secrets-and-accounts` (Lunarpulse) | present (repo secrets 2026-09-24; checklist `present`) | required — **signing happens in `release.yml`, never on an agent host** |
| fuzz-ledger branch + write | same | present (T2 nightly appends land; `FUZZ_LEDGER_WRITE_TOKEN` optional) | required |
| Dispatching the T3 chain (≈ 26 h, 15 hosted 5 h jobs) at the candidate SHA | operator (repository write) | never completed | required |
| `DOCKERHUB_USERNAME` / `DOCKERHUB_TOKEN` + cosign push | same | absent | not required once 20-2 makes `container.yml` skipped-or-green; required only to publish images |
| Stable perf runner (20-7 §O O1 host, O2 `perf-ledger`, O3 environment/runner-group restriction, O4 podman + T3 image lock, O5 registration + calendar, O6 hosting account, O7 fork-PR approval) | `ops-stable-perf-runner` + repo admin | O1/O2/O5 absent, O3/O7 unverified, O4 absent, O6 not inventoried | required under D-20-5-09 A; waived to GA only under B |
| Cutting and pushing `v1.0.0-rc.1` on the checked SHA | `ops-rc-signed-tag` | — | **is** the rc |
| GitHub Release publication | `release.yml` on the tag | — | automatic after the precondition; never by an agent |
| ≥ 3 / 12 external authors; pen-test firm and dates (Hold 1) | `ops-external-cohort-and-pen-test` | absent | GA only |
| ECCN letter (Hold 2) | RELEASE-HOLDS Hold 2 | open | GA only |
| `v1.0.0` + LTS clock start | `ops-ga-tag-after-holds` | backlog | GA only |
| Brew tap + AUR publication | `ops-brew-tap-and-aur-publication` | absent | not an rc gate (copies 20-2's rendered attachments after a release exists) |
| Paid live keys: `ops-live-gemini-leg`, `ops-live-key-j0-leg`, `ops-nightly-cassette-rerecord`, `ops-live-worker-under-17-1-profile`, `ops-j1-live-agent-signed-take`, `ops-j4-live-incident-recording`, `ops-google-calendar-oauth-mcp` | operator | absent | not rc gates; the release notes say "mechanism-proven, live on operator lane" (routing audit `:72`) |
| `ops-rpo-rto-cadence-proof` | operator | backlog | not an rc gate (named in release notes) |

## 4. What the 20-5 dev agent does (no operator involvement)

- LTS rule: `docs/release/lts-policy.md`; STABILITY.md LTS section and release-channel row from `stability_matrix.rs`;
  RELEASE-HOLDS pre-release sentence; tag procedure generalized to `vX.Y.Z-rc.N` (D-20-5-05, -06, -15).
- `[cli_wrapper]` `manifest_schema_version` (D-20-5-02…04): consts, pre-parse check, typed refusal, error-catalog
  entry + page, STABILITY rows, in-tree Worker manifests stamped, ABI ratification **proposal** (the operator ratifies).
- `check-release-precondition --fuzz-ledger` (T3 leg) and the `release.yml` wiring; `fuzz-prerelease.yml` trigger change
  (D-20-5-07).
- `check-rc-readiness` with a measured envelope (D-20-5-08, -10).
- Published-docs truth rows (D-20-5-11); deny re-check record (D-20-5-10).
- L2: version `1.0.0-rc.1` + `Cargo.lock` + regenerated STABILITY.md; release notes `docs/release/v1.0.0-rc.1.md`
  (D-20-5-13, -14); `check-rc-readiness --tag v1.0.0-rc.1` green except the legs only the operator can turn
  (recorded, never faked).

## 5. Explicitly NOT doable without the operator (the story never attempts, simulates or claims these)

Signing (`RELEASE_SIGNING_KEY` exists only as a repository secret), tagging and pushing a tag, publishing a GitHub
Release or an image, dispatching `fuzz-prerelease` or writing `fuzz-ledger`/`perf-ledger` by hand, provisioning or
registering a runner, entering secrets, ruling D-rows, ratifying the ABI row, flipping any `ops-*` row, or declaring
Holds 1–2 clear.
