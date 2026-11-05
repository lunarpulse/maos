# First signed pre-release tag procedure

> Canonical signing and key-management runbook: [`docs/runbooks/release-signing.md`](../runbooks/release-signing.md)

This procedure cuts exactly `v0.1.0-alpha.1`. It is a pre-release, not the GA
tag governed by Holds 1–2 in [`RELEASE-HOLDS.md`](../../RELEASE-HOLDS.md).
Those holds do not block this tag.

## Pre-tag checklist

Run these checks against the exact commit that will be tagged:

1. Update local `main`, then prove the candidate is the remote `main` tip:

   ```bash
   git switch main
   git fetch origin main
   git pull --ff-only
   candidate_sha="$(git rev-parse HEAD)"
   test "$candidate_sha" = "$(git rev-parse origin/main)"
   ```

2. Require the `aggregate` check run for that commit to be completed and
   successful:

   ```bash
   repo="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"
   gh api "repos/${repo}/commits/${candidate_sha}/check-runs?check_name=aggregate&status=completed&filter=latest" \
     | cargo run --locked -p xtask -- check-release-precondition
   ```

   The server-side filter is mandatory and matches the query the
   `Require successful main discipline aggregate` step runs in
   `.github/workflows/release.yml:79-84`: the check-runs list defaults to
   30 entries per page while `discipline.yml` declares 161 jobs, so an
   unfiltered query can page `aggregate` out of the response, making
   `check-release-precondition` report a false
   `no aggregate check-run exists for this tagged SHA` refusal for a
   commit whose check run actually exists and is green.

3. Confirm the regenerated lockfiles and stability contract are current:

   ```bash
   cargo metadata --locked --offline --format-version 1 >/dev/null
   cargo run --locked -p xtask -- stability-matrix --check --json
   ```

4. Confirm `MAOS_RELEASE_PUBKEY` and `RELEASE_SIGNING_KEY` are both `present`
   in [`docs/runbooks/provisioning-checklist.md`](../runbooks/provisioning-checklist.md).
   The release build rejects an absent, malformed, or bundled development
   public key; signing rejects an absent private key.

5. Merge `recovery-lane` into `main` by **merge commit — not squash, not rebase.**

   Story 16-0's AC2 falsifier is Story 15-4's real kernel-core commit
   `843d53657f21…`, which is **not on `main`**. It is anchored by the annotated
   tag `kernel-pin-falsifier-15-4`, so it survives either merge strategy — but
   every merge already on `main` is a merge commit, and a history rewrite would
   orphan the commits that several stories cite by SHA. The tag is defence in
   depth for one proof; this rule protects all of them.

   ```bash
   git ls-remote --tags origin kernel-pin-falsifier-15-4   # must be non-empty
   ```

   The tag name deliberately does not match `v*`, so neither `release.yml` nor
   `container.yml` fires on it.

6. Rehearse the macOS leg. `aarch64-apple-darwin` is the first macOS build in
   this project's history and the only artifact behind the Homebrew install
   route (`packaging/homebrew/maos.rb:24`), so it must compile once before a tag
   depends on it. There are two paths; **prefer the first.**

   **The discipline `macos-check` job does not replace this step (Story
   20-6).** Since 2026-10 a pull request or push that touches a
   platform-sensitive path also runs `cargo check --target aarch64-apple-darwin`
   on a macOS runner (`discipline.yml` `macos-scope` → `macos-check`). When the
   path filter judges a diff irrelevant, `aggregate` reports the guard as
   **NOT APPLICABLE — not a pass**, and the filter cannot see a change that
   alters what `release.yml` builds without touching a filtered path (a
   transitive dependency's own platform code, a `stable` toolchain release, or
   a portable edit — a rename, a changed signature — that breaks unchanged
   macOS-only code in another file). So the
   rehearsal below stays mandatory before every tag, regardless of the guard's
   result, and a green `macos-check` is evidence about the diff it ran on,
   never about the tagged SHA.

   **6a — before the merge (preferred).** Label the pull request
   `release-rehearsal`. `release.yml` triggers on `pull_request` into `main` and
   its `build` job runs only when that label is present
   (`github.event_name != 'pull_request' || contains(…, 'release-rehearsal')`);
   `sign-and-publish` is guarded on `github.event_name == 'push'`, so a
   rehearsal can never publish. This is the path that de-risks the merge itself:
   a red macOS leg is found on the PR, not on `main`.

   ```bash
   gh pr edit "$pr_number" --add-label release-rehearsal
   gh run watch "$(gh run list --workflow release.yml --event pull_request \
     --limit 20 --json databaseId,headSha \
     --jq "[.[] | select(.headSha == \"${pr_head_sha}\")][0].databaseId")" --exit-status
   ```

   **6b — after the merge (fallback).** Manually dispatch `release.yml` at `main`
   and require its
   `build (aarch64-apple-darwin, darwin-arm64, macos-latest)` leg to pass.
   Use this when the candidate is already on `main` — note that
   `workflow_dispatch` is unavailable until the workflow itself has reached the
   default branch, which is why 6a exists.
   The dispatch is asynchronous, so select the run by commit, not by
   recency: a recency-only `--limit 1` selection can watch an older or
   concurrent dispatch to green while the `aarch64-apple-darwin` leg —
   the first macOS build in this project's history and the only artifact
   behind the Homebrew install route (`packaging/homebrew/maos.rb:24`) —
   never runs for the commit being tagged. The selection retries for up
   to 60 seconds while the asynchronous dispatch registers:

   ```bash
   gh workflow run release.yml --ref main
   run_id=""
   for _ in 1 2 3 4 5 6; do
     sleep 10
     run_id="$(gh run list --workflow release.yml --event workflow_dispatch \
       --branch main --limit 20 --json databaseId,headSha \
       --jq "[.[] | select(.headSha == \"${candidate_sha}\")][0].databaseId // empty")"
     test -n "$run_id" && break
   done
   test -n "$run_id" # fails the checklist unless this run is $candidate_sha's
   gh run watch "$run_id" --exit-status
   ```

7. Re-read `candidate_sha` after the dispatch. If `main` moved, restart this
   checklist for the new tip.

## Cut the tag

Do not regenerate artifacts or edit files after the checklist. Tag the checked
commit and push only this tag:

```bash
test "$(git branch --show-current)" = main
test "$(git rev-parse HEAD)" = "$candidate_sha"
git tag v0.1.0-alpha.1 "$candidate_sha"
git push origin refs/tags/v0.1.0-alpha.1
```

Follow the linked canonical runbook for signing, artifact verification, and key
rotation. Acceptance requires `release.yml` to finish green and publish all four
binaries for each of the three targets, the original `SHA256SUMS`/signature,
amd64/arm64 `.deb` and `.rpm`, and the independently signed
`SHA256SUMS-packages`/signature. It also attaches rendered `maos.rb`, `PKGBUILD`,
`maos.spec` and the shared `verify-release.py`; never copy committed templates
to the tap/AUR. Packages disable stripping and carry exactly the signed bytes.

## Expected boundary conditions

- `container.yml` waits for the signed release, always builds and smokes the
  verified runtime image, then publishes/signs only with both Docker Hub
  credentials present. Otherwise it explicitly skips publication, not building.
- Before the rc tag, the operator must run the 20-2 branch-only verify recipe
  from the story's evidence `ci/ci-20-2-verify.yml` on `ci/20-2-verify`, recording
  native deb, Fedora/Arch/Homebrew, netns, container and Cargo results and
  restored negative controls. These hosted results are unverified locally.
- The first alpha release did not ship working package recipes. Only a release
  produced by the corrected pipeline can claim the rendered package routes;
  tap/AUR accounts remain `ops-brew-tap-and-aur-publication`.
- This procedure does not authorize a GA tag. GA remains blocked until both
  holds in `RELEASE-HOLDS.md` are clear.
