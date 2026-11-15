# RC.2 release record (2026-11-15)

Status: record of the `v1.0.0-rc.2` tag act and its evidence. Documents only. This file claims no gate result. Plan: `_bmad-output/planning-artifacts/ci-release-ready-program-2026-11-14.md`. Rulings: `_bmad-output/planning-artifacts/decisions-2026-11-14-release-ci.md`.

## 1. Candidate and tag
- Candidate: `650efcff89c378c5007f10ede2614bac06d306b9` on `main`, reached through PR #14 (`land-2026-11-14`), merged by fast-forward.
- Tag: `v1.0.0-rc.2`, lightweight, pushed 2026-11-15T09:41:53Z under operator ruling 13 (only that ref was pushed).

## 2. Timeline (UTC)
- 2026-11-14 07:37 - fuzz-prerelease run 43571938618 dispatched at 650efcff.
- 2026-11-14 08:35:59 - main push discipline run 43571926114: aggregate success.
- 2026-11-14 - PR #14 discipline attempt 2: aggregate success (attempt 1 red only on check-multi-region-slo and fan-ins; whole-run re-run disclosed, see section 7).
- 2026-11-14 - rc.2 rehearsal 43571940685 (workflow_dispatch, dry_run): red ONLY on the `discipline aggregate and ledgers` guard, because no T3 chain existed yet. Ancestry, classify, readiness, perf label, notes, deb/rpm for both archs, package signing and file patterns were green.
- 2026-11-15 ~09:40 - fuzz-prerelease run 43571938618 success (T3 chain complete).
- 2026-11-15 09:41 - pre-tag checks at 650efcff: check-release-precondition PASS x3; `cargo metadata --locked --offline` rc=0; `stability-matrix --check` rc=0; `check-rc-readiness --tag v1.0.0-rc.2` rc=0.
- 2026-11-15 09:41:53 - tag `v1.0.0-rc.2` pushed.
- 2026-11-15 10:03:23 - GitHub Release v1.0.0-rc.2 published (prerelease=true, draft=false). First run of the real `Create GitHub Release` step.
- 2026-11-15 10:03:42 - release.yml run 43700484361: success (3 builds + sign-and-publish).

## 3. Evidence
- T3 chain: fuzz-prerelease 43571938618, 5 links x 3 targets, t3-ledger-collect success. Correction: the figure 72004 cited earlier was ONE link, not the chain total.
- fuzz-ledger: 5 T3 records per target at 650efcff; 360020 cpu_seconds per target (floor 86400).
- Release v1.0.0-rc.2 assets (25): 12 binaries (maos, maosctl, maos-spirit, maos-registry-server for linux-amd64, linux-arm64, darwin-arm64); maos_1.0.0.rc.2-1_amd64.deb and _arm64.deb; maos-1.0.0.rc.2-1.x86_64.rpm and .aarch64.rpm; PKGBUILD, maos.rb, maos.spec, debian-recipes.tar.gz; SHA256SUMS, SHA256SUMS.sig, SHA256SUMS-packages, SHA256SUMS-packages.sig, verify-release.py.

## 4. Verified, and by whom
- Orchestrator: `sha256sum -c` of maosctl-linux-amd64 OK; `maosctl --version` = 1.0.0-rc.2.
- Orchestrator: embedded release pubkey is 84cffd596a2b556793615e01df6f6a431d8caa13527af352b33ecd2ecbddf40e (not the dev key bedd2ba6...).
- Orchestrator: the SHIPPED verify-release.py (python venv with cryptography) verified the signature and exact hashes of all 12 binaries.
- Release job only: package signatures were self-verified inside sign-and-publish.

## 5. Not verified
- deb/rpm via SHA256SUMS-packages were NOT verified by the orchestrator (known issue 1).
- Package signatures: self-verified inside the release job only; no independent check recorded.
- Live Anthropic re-record leg (journey-nightly tier-2-rerecord): cannot run. It reads `secrets.MAOS_ANTHROPIC_API_KEY`, which is absent from repository secrets (present: CI_EVIDENCE_AUDIT_KEY, MAOS_RELEASE_PUBKEY, RELEASE_SIGNING_KEY). The operator must add it before B4.
- Perf verdicts: ABSENT for this rc (perf-ledger branch absent, HTTP 404 = empty ledger; accepted by check-release-precondition). No timing claim is made.
- No external penetration test and no ECCN determination were performed (claim boundary, release notes).

## 6. Known issues (non-blocking for the release)
1. container.yml run 43700484370 failed: `release-verify --verify SHA256SUMS-packages` reports entry `maos-1.0.0~rc.2-1.aarch64.rpm` not found, because SHA256SUMS-packages lists tilde names and GitHub stores the assets with `~` replaced by `.`. Pre-release only; GA 1.0.0 has no tilde (unverified). Docker Hub publication is skipped (DOCKERHUB_* absent). Owner: `ops-ci-release-ready`.
2. docs-site.yml run 43700484344 failed: Playwright does not support chromium on ubuntu26.04-x64 (`ubuntu-latest` moved to 26.04). Not release-related. Owner: `ops-ci-release-ready`.
3. rc.1 (`v1.0.0-rc.1`, 1a57035e61741c2d7a49521a1466771b89659a2a, pushed 2026-11-14T02:07:07Z): release.yml run 43544486838 failed at step 17 with docker "cannot overwrite digest" (exit 125); attempt 2 failed identically; NOTHING was published. The failure was deterministic. rc.1 stays an unpublished tag, never moved or deleted. Fixed by pinning per-platform child digests (PR #14); proven hosted by dry-run 43558157808 and by release run 43700484361.

## 7. Corrections and disclosures
- Correction: 72004 was one link, not the T3 chain total (section 3).
- Correction: the rc.1 failure cause is the deterministic docker digest collision at step 17 (section 6.3).
- Disclosure: the local commit routine did NOT run `xtask prepush` before the candidate pushes, against proof-contract.md:147.
- Disclosure: GLM quota was exhausted, so the GPT reviewer performed the security reviews.
- Disclosure: one whole-run re-run of PR #14 discipline attempt 2 (attempt 1 red only on check-multi-region-slo and fan-ins). Recorded in decisions-2026-11-14-release-ci.md section 1.
- Evidence directories stay out of git (ruling B7a) at /home/lunarpulse/maos-evidence-archive/2026-11-14/ (local only, no backup).

## 8. Open
- PR #15 (parity-2026-11-14, local/hosted parity fixes): hosted proof pending.
- rc.2 is the second tag act on `ops-rc-signed-tag`, which is now `done` in sprint-status.yaml. Follow-ups are owned by `ops-ci-release-ready`.
