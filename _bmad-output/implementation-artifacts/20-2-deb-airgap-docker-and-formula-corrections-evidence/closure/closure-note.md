# Story 20-2 closure note (2026-11-12; record-only, zero kernel, no cargo)

Hosted green is read from `gh run view --json conclusion,jobs` (read-only), not from reading every log; tag-only paths were never executed.

- Branch-only run **42308080980**, workflow `ci-20-2-verify` @ `21431e39` (branch `ci/20-2-verify`): conclusion `success`; 16 jobs = 14 success + 2 skipped (`operator-prepare`, `operator-publication`, intended on a non-dispatch run). Brew job 128204570469: `==> Installing maos from maos/verify`, `==> Testing maos/verify/maos`, four `identity:` lines, wrong-key `InvalidSignature` refusal and "PASS: brew independently rejected wrong-key signature, then installed exact signed bytes and removed them". No `brew audit` offense line appears in the log (the step, if clean, is silent); the audit result is therefore asserted only by the job's success.
- Main discipline run **43150757804** @ `d00802aa`: `success`, 199/199 jobs success.

## Claim boundary (UNCLAIMED / tag-time / ops)

Homebrew tap, AUR, Docker Hub image push/cosign signature, and the production-key release-attached .deb/.rpm were NOT published or exercised; the corrected recipes were witnessed only against the development-signed dist on hosted runners (ci-20-2-verify run 42308080980). Air-gap (NFR-Ops-12) is witnessed as no outbound attempts in a replay session on the development-key build, not on a signed tag.

| Item | Status | Evidence / owner |
|---|---|---|
| AC1.1, AC1.2, AC1.4, AC1.5 | LANDED (93d39cc7, hosted green) | `deb (ubuntu-24.04[-arm])`, wrong-key plant step |
| AC1.3 release attach of packages + `SHA256SUMS-packages` + sig | CLAIM-BOUNDARY | assembly proven by `packaging-assembly`; attach step is push-tag-only (`release.yml`), never executed; `ops-rc-signed-tag` |
| AC2.1, 2.2, 2.4, 2.6, 2.7 | LANDED | `recipes (rpm|arch|brew)` green; local plants |
| AC2.3 `maos.rb` | LANDED; brew install/test/identity previously HOSTED-UNVERIFIED (hosted-4), now green in run 42308080980 | job 128204570469 |
| AC2.5 release attaches rpm + recipes | CLAIM-BOUNDARY | tag-only; `ops-rc-signed-tag`, `ops-brew-tap-and-aur-publication` |
| AC3.1–3.4, 3.6, 3.7 | LANDED, hosted green on dev-key build | `netns`, discipline `air-gap-netns-witness` |
| AC3.5 hosted failing-before run id | CLAIM-BOUNDARY | no hosted base-tree run id exists; creation-time reading only; superseded by the green witness + plant |
| AC3.8 | NOT TRIGGERED | netns green |
| AC4.1–4.3 | LANDED (deviation: `check-air-gap` advisory per D-20-2-02 C) | verify `cargo-and-symbols` plant |
| AC5.1, 5.2, 5.4, 5.5 | LANDED, hosted green | `container-build`, verify `container` |
| AC5.3 `container.yml` split / Docker Hub push + cosign | CLAIM-BOUNDARY | tag-only, no `DOCKERHUB_*` secrets; `ops-provisioning-secrets-and-accounts` |
| AC6.x, AC7.1, AC8.1 | LANDED | |
| AC7.2 cargo route | PARTIAL by design | temp-root `maos-bin` only, per hosted-1 |
| AC8.2 coverage rows | LANDED 2026-11-12 | `tests/coverage-matrix.yaml` notes corrected |
| AC8.3 size | LANDED | 31,936,280 B < 33,554,432 B |
| AC8.4 deferred rows | LANDED 2026-11-12 | `deferred-work.md` rows 1144/1146/1147 closed; row 202 re-cited |
| T5 `release-rehearsal` dispatch | NOT-DONE (optional, owed to Main) | `gh workflow run release.yml --ref main` |
| T10 `prepush` / workspace nextest | NOT-DONE here | prepush timed out (wrapper 124) and 6-failure nextest run recorded in the Dev Agent Record; not retried |

Not verified: drift of the tree since `21431e39`; hosted logs beyond the brew job; the `release` environment state.
