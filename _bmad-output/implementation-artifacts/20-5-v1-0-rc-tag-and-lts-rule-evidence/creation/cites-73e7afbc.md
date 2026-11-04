# 20-5 creation — cites re-read at `73e7afbc` (confidence rule 9)

Method: every range below was read in the working tree at `73e7afbc` (clean apart from a sibling's 20-11 records,
which this story does not cite) or on the `git archive 73e7afbc` copy at `/tmp/c205/base`. No cargo ran. OLD → NEW
rows are applied in place to E20 §20-5, the tracker row and the preflight rows (`records-owed.md` §A).

## C — code and workflow cites

| # | Cite at HEAD | What it says now | OLD (epic / tracker / preflight) → NEW |
|---|---|---|---|
| C1 | `Cargo.toml:67` | `version = "0.1.0-alpha.1"` (workspace) | E20 AC1 `Cargo.toml:64`, "`0.1.0-alpha` today" → `:67`, `0.1.0-alpha.1` (the amendment already says `:67`) |
| C2 | `xtask/src/main.rs:617-625` (command), `:1406-1409` (dispatch) | `stability-matrix [--check] [--json]` | E20 AC1 `main.rs:595`, amendment `:674-679` → `:619-625`, `:1406-1409` |
| C3 | `xtask/src/stability_matrix.rs:335-346` | `parse_workspace_version` | same |
| C4 | `xtask/src/stability_matrix.rs:348-368` | `lts_clock_start`: `git rev-list -n 1 1.0.0` / `v1.0.0`; placeholder names "Epic 10" at `:367` (doc `:350`) | same; the git shell-out means a `git archive` copy and a clone render differently once `v1.0.0` exists (D-20-5-06) |
| C5 | `xtask/src/stability_matrix.rs:215-222` (template), `STABILITY.md:53-60` (rendered) | "MAOS v1.5 carries a 2-year LTS commitment … the v1.0 line receives security-only patches for 2 years"; clock comment names Epic 10 | amendment cites only `:221` / `STABILITY.md:59-60` → `:215-222` / `:53-60`: the LTS sentence itself contradicts NFR-Maint-6 (C26) |
| C6 | `xtask/src/stability_matrix.rs:463-493` | in-src tests assert `"2-year LTS"` (`:474`) and `"Epic 10"` (`:492`) | new: both assertions change with D-20-5-05 |
| C7 | `STABILITY.md:21-27` | live triple: kernel `0.1.0-alpha.1`, abi 1, schema 6, window `1..=6`, 58 crates; no release-channel row, no `[cli_wrapper]` row | new |
| C8 | `.github/workflows/release.yml:152-174` | `softprops/action-gh-release@v2`, `generate_release_notes: true` (`:171`), `prerelease: ${{ contains(github.ref_name, '-') }}` (`:174`) | E20 AC1 "`:88-94` gets `prerelease: … '-rc'` (an rc publishes as a full release today)" → **false/closed**; amendment `:142-158` → `:152-174`. Retain the generic `'-'` rule, do not narrow |
| C9 | `.github/workflows/release.yml:103-122` | `sign-and-publish` (push only) runs `gh api …check-runs?check_name=aggregate…` piped into `check-release-precondition` | new anchor; tag-procedure's `release.yml:79-84` (`docs/release/tag-procedure.md:34`) → `:117-122` |
| C10 | `.github/workflows/release.yml:75,133-134,139,151` | `MAOS_RELEASE_PUBKEY` / `RELEASE_SIGNING_KEY` secrets | E20 AC1 `release.yml:75-77` → these lines |
| C11 | `.github/workflows/release.yml:2-4` | `on: push: tags: ['v*']` | new |
| C12 | `xtask/src/check_release_precondition.rs:33-96` (judging `:48-79`) | aggregate check-run only: no fuzz-ledger, perf-ledger or deny read | amendment `:48-78` → `:48-79`, same finding |
| C13 | `.github/workflows/fuzz-prerelease.yml:1-14` | T3 chain; `on: workflow_dispatch` + `push: tags: ['v*-rc*']`; header "The rc release precondition is owned by 20-5, not implemented here" (`:5`) | new: the chain starts on the same tag push that publishes (D-20-5-07) |
| C14 | `.github/workflows/fuzz-prerelease.yml:294-326` | collector: five links per target, `tier == "T3"`, `commit == GITHUB_SHA`, Σ `cpu_seconds` ≥ 86 400 | the readiness leg mirrors this rule (C15) |
| C15 | `scripts/collect-ledger.sh:5-8,20-31`; `scripts/run-fuzz-link.sh:31-33` | ledger `fuzz-ledger.json` on branch `fuzz-ledger`, `schema_version == 1`, records `{target, tier, commit, cpu_seconds, corpus_size, run_id: "<run>-<link>", timestamp, workers}` | new |
| C16 | `.github/workflows/container.yml:11-14,40-44` | fires on `v*`; Docker Hub login with `DOCKERHUB_*`, no secrets guard | E20 AC1 `:12-14` → `:11-14`; guard is 20-2 AC1 (not landed) |
| C17 | `xtask/src/release_dry_run.rs:27-44` | `ARTIFACTS` = maos, maosctl, maos-registry-server, maos-spirit; no `.deb`, no air-gap variant | `.deb` + air-gap are 20-2's (ready-for-dev) |
| C18 | `.github/workflows/discipline.yml:2898,6594` | `epic-20-exit` job exists and is in `aggregate.needs` | new |
| C19 | `.github/workflows/discipline.yml:157-189` | blocking `cargo-deny` job: `cargo deny fetch db` (3 tries) then `cargo deny check --disable-fetch` | preflight `:192` same |
| C20 | `deny.toml:15-52` | six ignored IDs: RUSTSEC-2021-0127, -2025-0134, -2024-0449, -2026-0370/0371/0372; comments say each is "tracked in deferred-work" | **partly false**: no `deferred-work.md` row names RUSTSEC-2021-0127 or RUSTSEC-2025-0134 (grep count 0; serde_cbor is discussed at `:574` without the ID) |
| C21 | `crates/maos-manifest/src/manifest.rs:4403-4528` | `CliWrapperConfig` (`deny_unknown_fields`, `:4415`), no schema field; `from_toml_str` parses then `validate` (`:4495-4527`) | tracker cite "`security/mod.rs:314-333`" → see C23 |
| C22 | `crates/maos-bin/src/worker_spawn.rs:610-618` | the only production `CliWrapperConfig::from_toml_str` call: serializes `manifest_root["cli_wrapper"]` then strict-parses | new (the pre-parse peel lands here, D-20-5-02 A) |
| C23 | `crates/maos-kernel-core/src/security/mod.rs:279-289,525-573` | `admit_spirit` requires `[class]` (`EClassRequired`), calls `check_abi_triple` (`pub fn`, `:525`): `ESubstrateTooOld`, `EAbiTooOld`, `EAbiTooNew` on `[class].manifest_schema_version` only | tracker `security/mod.rs:314-333` → `:279-289,525-573` |
| C24 | `crates/maos-kernel-core/src/lifecycle/cli_wrapper/admission.rs:220-229,385-393`; `lifecycle.rs:58-66` | `admit_cli_wrapper_journaled` (egress, probe); the two in-src `CliWrapperConfig` literals are in `#[cfg(test)]` modules — counted in the kernel pin (physical and tokei) | new: any struct field moves the kernel pin (measurement 2) |
| C25 | `crates/maos-kernel-core/src/security/manifest.rs:9` | `pub use maos_manifest::*;` | new: a new top-level `maos-manifest` item re-exports through the kernel surface; an associated fn does not ([INFERENCE], T0 measurement 4) |
| C26 | PRD `prd/non-functional-requirements.md:137`; inventory `epics/requirements-inventory.md:217`; `tests/coverage-matrix.yaml:1189-1195` | NFR-Maint-6: "1-year LTS commitment at v1.0; 2-year LTS commitment at v1.5 once support load is known; security-only patches after year 1"; matrix `gates: []`, notes "1-year LTS at v1.0; 2-year LTS at v1.5" | contradicts C5 |
| C27 | `crates/maos-domain/src/cli_wrapper.rs:14-86` | `CliWrapperAdmissionError` (`#[non_exhaustive]`), no schema variant | new |
| C28 | `crates/maos-spirit-abi/src/lib.rs:71,122,143,171` | `ABI_VERSION`, `MANIFEST_SCHEMA_VERSION = 6`, `MIN/MAX_SUPPORTED_*` | new; `stability_matrix.rs:38` reads this file, `abi-diff` scans this crate (`xtask/src/abi_diff.rs:8`) |
| C29 | `xtask/abi-ratifications.toml:205-213` | 17-1 row: "no schema version exists on cli_wrapper"; `covered_changes = ["CliWrapperConfig", "CliWrapperPosture", "EgressDestination"]` | precedent for a ratification row (D-20-5-04) |
| C30 | `crates/maos-domain/src/revocation.rs:628-686` | `semver_range_contains` / `compare_versions`: a suffixed component sorts below its bare twin (§11.4) | read, not run: `1.0.0-rc.1` ≥ every in-tree `min_substrate_version` (40 × `0.1.0`, 17 × `0.1.0-alpha`, 2 × `0.1.0-alpha.1`) |
| C31 | `docs/release/tag-procedure.md:1-7,53-67,131-161` | alpha-only ("cuts exactly `v0.1.0-alpha.1`"); step 5 merges `recovery-lane` | step 5 is done: `recovery-lane` (`07302ace`) is an ancestor of HEAD (measured `git merge-base --is-ancestor`) |
| C32 | `RELEASE-HOLDS.md:17-21,72-83` | GA held by Holds 1–2; names only `v0.1.0-alpha.1` as the signed pre-release | new: the rc needs the same pre-release sentence |
| C33 | `docs/runbooks/provisioning-checklist.md:24-38,44` | `MAOS_RELEASE_PUBKEY`, `RELEASE_SIGNING_KEY` `present`; `DOCKERHUB_*` `absent`; fuzz-ledger branch permission `present`; "24-hour fuzz execution … rc precondition belongs to 20-5"; stable performance runner `absent` | new |
| C34 | `xtask/src/main.rs:800-820`; `xtask/src/check_perf_verdict.rs:4-21` | `check-perf-verdict`: PASS/FAIL/INDETERMINATE/ABSENT, missing ledger loud (20-7 L1 landed) | new; the release-precondition perf read is 20-7 AC11.2 (`20-7-…md:232`), L2 |
| C35 | `docs-site/docs/deploy/topology.md:38,75` (ko `:39,:76`); `docs-site/docs/manifest/v3.md:72` (ko `:73`); `docs-site/docs/errors/ECliWrapperTierNotGranted.md:32-34` | `./target/release/maos run` without a manifest (refused: `worker_spawn.rs:165`); `argv_prefix` example `["code"]`; Resolution names no `MAOS_HOST_GRANTS` (`env_contract.rs:602`) | `deferred-work.md:1186` "ko twins" → only topology/v3 have ko twins; no ko `errors/` tree exists |
| C36 | `docs-site/docs/errors/*.md` slugs; `docs-site/route-manifest.json:263`; `xtask/src/check_error_catalog.rs:509` | **42** `/errors/<Family>::<Variant>` routes in 6 families (EComplianceRejection 4, EPinMismatch 4, ESkillProposal 4, ESkillQueue 16, ESkillSchema 8, ESkillStore 6); `expected_distinct_pages` 135; `docs_url = https://docs.maos.dev/errors/{code}` | `deferred-work.md:1209` "18 of 24" → 42 variant routes at HEAD; collapse count not rebuilt ([INFERENCE] ≤ 36 wrong pages; T0 measurement 6) |
| C37 | `xtask/src/sprint_status.rs:16-51` | single-sourced tracker parser (strips comments) | reuse for the readiness leg |

## O — operator-lane and CI facts (measured 2026-11-04, read-only)

| # | Fact | Method |
|---|---|---|
| O1 | Tags: `v0.1.0-alpha.1` (`0c661995`), `frozen-kernel-v2.0`, `kernel-pin-falsifier-15-4`; no `v1.0.0*` | `git ls-remote --tags origin` |
| O2 | `fuzz-ledger` tip `c1de4004` (2026-11-03T14:39Z): 21 records, all `T2` (7 per target, 7 commits); **0 `T3` records**; local `origin/fuzz-ledger` is stale (`60181e4d`, 9 records) | `git clone --depth 1 --branch fuzz-ledger` into `/tmp` |
| O3 | `fuzz-prerelease` has run once: 38926626626 (workflow_dispatch @ `c36857c8`, 2026-10-16), link 1 `manifest_parser` failed after ~1 min, links 2–5 skipped, collector failed | `gh run list/view` |
| O4 | No `perf-ledger` branch; self-hosted runners: 0 | `git ls-remote origin`; `gh api …/actions/runners` |
| O5 | Repository secrets: `CI_EVIDENCE_AUDIT_KEY`, `MAOS_RELEASE_PUBKEY`, `RELEASE_SIGNING_KEY` (2026-09-24); no `DOCKERHUB_*`, no `FUZZ_LEDGER_WRITE_TOKEN`; no environments, no variables (org/environment secrets not visible to this token) | `gh secret list`, `gh api …/environments`, `gh variable list` |
| O6 | Repository is PUBLIC (hosted minutes for the 15 × 5 h T3 jobs are not billed on standard runners [INFERENCE from GitHub's public-repo policy]) | `gh repo view --json visibility` |
| O7 | `v0.1.0-alpha.1` release body = GitHub auto-notes: a list of PR titles (#1–#6); most landings since are direct pushes | `gh release view` |
| O8 | Upstream re-check of the zbus trigger (D-18-0-03): `secret-service` 5.2.0 depends on `zbus ^5`; `zbus` 6.2.1 exists; `keyring` 4.2.0, `zbus-secret-service-keyring-store` 1.0.1 unchanged → **trigger not met** | crates.io API |
| O9 | 20-7 L2 operator prerequisites are **O1–O7**, not O1–O6 (`20-7-…-evidence/creation/records-owed.md:49-55`; O3 and O7 added by the L1 security correction, `:60`) | read |

## T — tracker states for Epic 20 at HEAD (`sprint-status.yaml:295-322`)

done: 20-3a, 20-3b, 20-3c, 20-6, 20-12, 20-15, 20-16 · in-progress: 20-7 (L1 landed; L2 needs `ops-stable-perf-runner`) ·
ready-for-dev: 20-1a, 20-2, 20-4, 20-8, 20-9, 20-10, 20-11, 20-13, 20-14 (20-1a, 20-4, 20-10 have landings on main but
are not `done`) · backlog: 20-1b, 20-5 (this story; → ready-for-dev at creation). `epic-20-exit` is enrolled (C18);
its green on the final tree is 20-1a's/20-2's contract, not re-run here.
