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

5. Rehearse the macOS leg. `aarch64-apple-darwin` is the first macOS build in
   this project's history and the only artifact behind the Homebrew install
   route (`packaging/homebrew/maos.rb:24`), so it must compile once before a tag
   depends on it. There are two paths; **prefer the first.**

   **5a — before the merge (preferred).** Label the pull request
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

   **5b — after the merge (fallback).** Manually dispatch `release.yml` at `main`
   and require its
   `build (aarch64-apple-darwin, darwin-arm64, macos-latest)` leg to pass.
   Use this when the candidate is already on `main` — note that
   `workflow_dispatch` is unavailable until the workflow itself has reached the
   default branch, which is why 5a exists.
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

6. Re-read `candidate_sha` after the dispatch. If `main` moved, restart this
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
rotation. Acceptance requires `release.yml` to finish green and publish the six
binaries plus `SHA256SUMS` and `SHA256SUMS.sig`.

## Expected boundary conditions

- `container.yml` also runs on `v*` and is expected to be red until Story 20-2
  splits image build from registry publication and its Docker Hub/cosign secrets
  are provisioned. That known red run does not invalidate a green
  `release.yml` for this pre-release.
- The Homebrew, AUR, Debian, and RPM recipes remain publication scaffolds.
  Do not claim package-manager availability for `v0.1.0-alpha.1`; ownership and
  the normalized-version mechanism are recorded in `RELEASE-HOLDS.md` boundary
  19.
- This procedure does not authorize a GA tag. GA remains blocked until both
  holds in `RELEASE-HOLDS.md` are clear.
