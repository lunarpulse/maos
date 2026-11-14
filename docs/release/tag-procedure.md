# Signed pre-release tag procedure

> Canonical signing and key-management runbook: [`docs/runbooks/release-signing.md`](../runbooks/release-signing.md)
> The LTS rule and what a release candidate does and does not promise: [`docs/release/lts-policy.md`](lts-policy.md)

This procedure cuts a pre-release tag: `vX.Y.Z-rc.N` (the `v1.0.0-rc.N` line), and
it is the procedure that cut `v0.1.0-alpha.1` (Story 15-4; the alpha-specific
history is kept below and marked). A pre-release is not the GA tag; Holds 1–2 in
[`RELEASE-HOLDS.md`](../../RELEASE-HOLDS.md) are claim boundaries (no GA effect)
and a release candidate starts no LTS clock.

**Who runs this.** Every step is the operator's: agents never tag, sign, dispatch a
workflow, write a ledger or publish. Nothing below was exercised by the Story 20-5
landing that wrote it (hosted and operator steps are **unverified** until the
operator's first run records them). Replace `vX.Y.Z-rc.N` with the tag being cut
and keep it in `$tag`:

```bash
tag=v1.0.0-rc.1   # example; must equal "v" + [workspace.package].version in Cargo.toml
```

> **Dated note 2026-11-12 (operator ruling; the paragraph above is kept as written):**
> agents may dispatch the T3 fuzz chain (`fuzz-prerelease.yml`) when their token allows it —
> the exact command and preconditions are in
> [`decisions-2026-11-12.md`](decisions-2026-11-12.md) — but never tag, sign or publish: the
> signed tag needs the key holder.
>
> **Dated note 2026-11-12 (landing-2 review F3; the note above is kept as written):** "needs
> the key holder" is the operator's policy, not an enforced control. The commands below cut an
> ordinary `git tag`; `release.yml` holds `RELEASE_SIGNING_KEY` and signs the *artifacts*
> (`SHA256SUMS`, `SHA256SUMS-packages`) whenever an accepted tag on an `origin/main`
> ancestor is pushed. No Git tag signature is verified, and the `release` environment has no
> required reviewers, so whoever is authorised to push the tag can trigger CI-held signing
> and publication without holding the private key. Protecting the `release` environment
> (required reviewers) is the enforcement and is **UNCLAIMED**; do not describe this tag as
> a cryptographically signed Git tag.
>
> **Dated note 2026-11-12 (operator ruling 13; the notes above are kept as written):** the
> operator's free text "완료되면 태그 push" ("push the tag when it is complete";
> `_bmad-output/planning-artifacts/decisions-2026-11-12-acceleration.md` item 13) authorizes
> the orchestrator to run "Cut the tag" for `v1.0.0-rc.1` and push that one ordinary tag once
> every pre-tag checklist step is green at the candidate SHA (item 13 conditions a-e: the
> candidate is the `origin/main` tip and `main` has not moved since the T3 dispatch;
> discipline `aggregate` green for the SHA; one `fuzz-prerelease.yml` run at the SHA green
> with `t3-ledger-collect` and `check-release-precondition` green against the real ledger;
> `cargo metadata --locked --offline`, `stability-matrix --check` and
> `check-rc-readiness --tag v1.0.0-rc.1` green; the step-6b `release.yml` rehearsal green
> including `aarch64-apple-darwin`). Any red or skipped condition means no tag. That
> explicit per-tag authorization satisfies the key-holder policy for this tag only; no Git
> tag signature is created or verified, artifact signing runs in CI, and `release`
> environment protection stays **UNCLAIMED**. Force push, moving, deleting or re-cutting the
> tag, other tags and history rewrite remain per-event asks.
>
> **Dated note 2026-11-14 (rc.1 tag run; the notes above are kept as written):** the `v1.0.0-rc.1`
> tag push (at `1a57035e`) ran `release.yml` run 43544486838. Its `sign-and-publish` job failed
> in the step "Build deb and RPM for both Linux architectures without rewriting binaries" with
> `docker: cannot overwrite digest sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852`
> (exit 125); a re-run failed identically. Cause: the step's loop ran `docker run --platform
> linux/$arch` for `amd64` then `arm64` against the same pinned multi-arch **index** digest, and
> Docker 28.0.4's reference store (the runner reported `Storage Driver: overlay2`) maps one digest
> reference to exactly one image ID, so the second platform's pull collided with the first.
> Nothing was published (no GitHub Release exists for the tag). Fix: each platform now pins
> its own **child manifest** digest taken from that same index (still digest-pinned; evidence in
> `_bmad-output/implementation-artifacts/release-docker-digest-fix-evidence/`), and the job gained
> a non-publishing rehearsal (step 6b), because this step and everything after it in the job had
> never run hosted: the old dispatch rehearsal skipped the whole job. The fix is **UNVERIFIED hosted**
> until the operator dispatches the rehearsal. rc.2 plan: `v1.0.0-rc.1` stays as an unpublished tag
> and is never moved or deleted; after the fix reaches `main`, a new candidate (workspace version
> `1.0.0-rc.2`, `docs/release/v1.0.0-rc.2.md`, a T3 fuzz chain for the new SHA) runs this checklist
> again and is tagged `v1.0.0-rc.2` only after the step-6b rehearsal is green at that SHA.

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
   successful, **and a complete T3 fuzz chain for the same commit** (Story 20-5):

   First dispatch the 24 h chain at the candidate and wait for it. It is the
   operator's dispatch — the release job refuses a SHA without the chain, and a tag
   push does not start it (the `v*-rc*` push trigger was removed so a tag cannot
   start a second ~26 h chain after the tag it must precede):

   ```bash
   gh workflow run fuzz-prerelease.yml --ref main
   # select the run by commit, never by recency (as in step 6b below), then:
   gh run watch "$run_id" --exit-status   # 5 serial 5 h links + setup; `t3-ledger-collect` must be green
   ```

   The hosted wall time is stated as measured at the first dispatch (expected
   about 26 h per chain [INFERENCE: 5 links x 5 h + setup]; no chain has completed
   yet, the one dispatch so far failed). Then check both legs against the exact
   commit, with the ledger read from the `fuzz-ledger` branch:

   ```bash
   repo="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"
   gh api -H "Accept: application/vnd.github.raw+json" \
     "repos/${repo}/contents/fuzz-ledger.json?ref=fuzz-ledger" > fuzz-ledger.json
   gh api "repos/${repo}/commits/${candidate_sha}/check-runs?check_name=aggregate&filter=latest" \
     | cargo run --locked -p xtask -- check-release-precondition --fuzz-ledger fuzz-ledger.json
   ```

   The chain rule is the collector's (`fuzz-prerelease.yml` `t3-ledger-collect`):
   for each of `manifest_parser`, `frame_deser`, `fuzz_exec_deps_parse_elf`, one
   workflow run's five links, every record `tier: "T3"` at exactly `candidate_sha`,
   libFuzzer-measured seconds summing to at least 86,400 (configured time and other
   runs never count). Against the real ledger the leg **refuses** until your
   dispatch exists (0 T3 records when measured 2026-11-04).

   The perf-ledger leg (Operator ruling 2026-11-12, rc = b): `release.yml` runs
   `check-perf-verdict --json` on the `perf-ledger` branch (absent = empty ledger) and
   passes the report as `--perf-verdicts`; a release candidate adds `--perf-absent-ok`
   (ABSENT accepted, labelled in the notes), GA does not (every row PASS), and FAIL or
   INDETERMINATE are always refused. The aggregate server-side `check_name=aggregate&filter=latest` filter is mandatory
   (no `status=completed` filter: a newer run still in progress is the run judged and
   refused, never hidden behind an older green one) and matches
   the query the `Require successful main discipline aggregate` step runs in
   `.github/workflows/release.yml` `sign-and-publish`: the check-runs list defaults to
   30 entries per page while `discipline.yml` declares 161 jobs, so an
   unfiltered query can page `aggregate` out of the response, making
   `check-release-precondition` report a false
   `no aggregate check-run exists for this tagged SHA` refusal for a
   commit whose check run actually exists and is green.

3. Confirm the regenerated lockfiles and stability contract are current, and
   that the tree is a release candidate (Story 20-5; offline, reads committed
   files only, never tags):

   ```bash
   cargo metadata --locked --offline --format-version 1 >/dev/null
   cargo run --locked -p xtask -- stability-matrix --check --json
   cargo run --locked -p xtask -- check-rc-readiness --tag "$tag" --json
   ```

   `check-rc-readiness` is green only when: the workspace version is `X.Y.Z-rc.N`
   and equals `$tag` sans the `v`; STABILITY.md is in sync; every `20-*` story in
   `sprint-status.yaml` is `done` (with `--tag`, Story 20-5's own key included) — except
   `20-7-stable-runner-perf-gates`, accepted at `in-progress` and reported `WAIVED` for an
   `-rc.N` version whose notes carry the perf label (operator ruling 2026-11-12, rc = b);
   the `MAOS_RELEASE_PUBKEY`, `RELEASE_SIGNING_KEY` and `fuzz-ledger branch
   permission` rows of the provisioning checklist are `present`, and the `release
   environment protection evidence` row is `present` **or** `absent` with a labelled
   `WARN: environment protection not provisioned: claim boundary` (then the notes
   must carry the exact line `Release environment protection: NOT PROVISIONED`);
   `docs/release/v<version>.md` exists and carries the exact line `Perf verdicts: ABSENT`
   (leg `perf-verdict-label`); and every `deny.toml` ignored RUSTSEC id has a
   `deferred-work.md` row. `release.yml` runs the same command with `--tag` before
   signing when the ref is an rc tag. See
   [`decisions-2026-11-12.md`](decisions-2026-11-12.md).

4. Confirm `MAOS_RELEASE_PUBKEY` and `RELEASE_SIGNING_KEY` are both `present`
   in [`docs/runbooks/provisioning-checklist.md`](../runbooks/provisioning-checklist.md).
   The release build rejects an absent, malformed, or bundled development
   public key; signing rejects an absent private key.

5. **History (alpha only; closed).** For `v0.1.0-alpha.1` this step merged
   `recovery-lane` into `main` by merge commit. `recovery-lane` is already an
   ancestor of `main` (checked at 20-5: `git merge-base --is-ancestor 07302ace HEAD`),
   so nothing remains to merge for a later tag. Keep the rule it taught: every
   merge on `main` is a merge commit and a history rewrite would orphan the commits
   that several stories cite by SHA. The annotated tag `kernel-pin-falsifier-15-4`
   anchors one proof (`git ls-remote --tags origin kernel-pin-falsifier-15-4`);
   its name does not match `v*`, so neither `release.yml` nor `container.yml`
   fires on it.

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
   `sign-and-publish` never runs for a `pull_request` (it needs a tag push or a `dry_run`
   dispatch), so a pull-request rehearsal can never publish. This is the path that de-risks the merge itself:
   a red macOS leg is found on the PR, not on `main`. It does **not** exercise the package/sign path; step 6b does.

   ```bash
   gh pr edit "$pr_number" --add-label release-rehearsal
   gh run watch "$(gh run list --workflow release.yml --event pull_request \
     --limit 20 --json databaseId,headSha \
     --jq "[.[] | select(.headSha == \"${pr_head_sha}\")][0].databaseId")" --exit-status
   ```

   **6b — after the merge (fallback), and the only rehearsal of the package/sign path.**
   Manually dispatch `release.yml` at `main` (or at any branch ref) and require its
   `build (aarch64-apple-darwin, darwin-arm64, macos-latest)` leg **and** its
   `sign-and-publish` job to pass. Since the rc.1 failure (note above) the dispatch runs
   `sign-and-publish` end to end — main-ancestry, tag classification, rc readiness, the
   aggregate/fuzz/perf ledgers, signing, the production-key refusals, rendering, the amd64/arm64
   `deb`/`rpm` container builds, package signing, the rc notes guards and a check that every
   file the release would upload exists — up to, and excluding, `Create GitHub Release`, which is
   gated on `github.event_name == 'push'` and can never run for a dispatch. Rehearsal semantics:
   the ref is a branch, so the job classifies the tag the tree *would* carry (`v` + the workspace
   version) through the same grammar; the guards run against the dispatched commit but do not stop
   the job (so one run also exercises the packaging), and the final `Rehearsal verdict` step fails
   the run if any of them did not succeed, listing each outcome in the run summary. A rehearsal on
   a branch commit is therefore expected to be red on the ancestry and ledger guards; at the
   candidate `origin/main` tip, with the T3 chain present, it must be fully green.
   `dry_run` defaults to `true`; `dry_run=false` fails the run by design (a dispatch cannot publish).
   The step needs the `release` environment to accept the dispatched ref and the `MAOS_RELEASE_PUBKEY`
   / `RELEASE_SIGNING_KEY` secrets (as `sign-and-publish` already does).
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
   gh workflow run release.yml --ref main -f dry_run=true
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

   To rehearse a fix on a branch before merging it, dispatch that ref the same way
   (`gh workflow run release.yml --ref <branch> -f dry_run=true`) and select the run with
   `--branch <branch>` and that branch's head SHA.

7. Re-read `candidate_sha` after the dispatch. If `main` moved, restart this
   checklist for the new tip.

## Cut the tag

Do not regenerate artifacts or edit files after the checklist. Tag the checked
commit and push only this tag:

```bash
test "$(git branch --show-current)" = main
test "$(git rev-parse HEAD)" = "$candidate_sha"
git tag "$tag" "$candidate_sha"
git push origin "refs/tags/$tag"
```

(The alpha used `v0.1.0-alpha.1` in the same two commands.) A hyphenated tag
publishes as a GitHub pre-release (`release.yml` `prerelease: contains(github.ref_name, '-')`),
never as `latest`.

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
- This procedure does not cut a GA tag. Per the operator ruling of 2026-11-12,
  RELEASE-HOLDS Holds 1–2 (pen-test, ECCN) are claim boundaries, not GA
  preconditions; `v1.0.0` needs the engineering exits green plus the signed-tag
  step by whoever holds the key (an operator action AI agents cannot perform),
  `ops-ga-tag-after-holds`, and is the only tag that starts the LTS clock
  ([`lts-policy.md`](lts-policy.md)).
