# rc.1 `sign-and-publish` docker digest collision: evidence and fix

Scope: `release.yml` run 43544486838 (event `push`, tag `v1.0.0-rc.1`, commit `1a57035e61741c2d7a49521a1466771b89659a2a`),
job `sign-and-publish`, step "Build deb and RPM for both Linux architectures without rewriting binaries".
Nothing was published (`gh release list` shows only `v0.1.0-alpha.1`). Captured 2026-11-14.

## Cause

The step looped `for arch in amd64 arm64` running
`docker run --platform linux/$arch ... ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852`.
That digest is the multi-arch **index**, not a platform image. Log lines (`rc1-failed-log-excerpt.txt`, from
`gh run view 43544486838 --log-failed`):

| line (timestamp) | text |
|---|---|
| 02:31:47.77 | `Unable to find image 'ubuntu:24.04@sha256:fe81…ef852' locally` (1st iteration, amd64) |
| 02:31:48.57 | `Status: Downloaded newer image for ubuntu@sha256:fe81…ef852` |
| 02:32:44.49 | `Unable to find image 'ubuntu:24.04@sha256:fe81…ef852' locally` (2nd iteration, arm64) |
| 02:32:46.04 | `Digest: sha256:fe81…ef852` then `docker: cannot overwrite digest sha256:fe81…ef852` |
| 02:32:46.05 | `##[error]Process completed with exit code 125.` |

The first iteration ran the whole amd64 deb+rpm build (~1 min) successfully; the failure is the second `docker run`'s pull.

Mechanism, from the moby source of the reported engine version (Docker 28.0.4, `moby-v28.0.4-reference-store-excerpt.txt`):
`reference/store.go addReference` refuses to re-point an existing **digest** reference at a different image ID
(`force only works for tags` → `cannot overwrite digest`). The reference `ubuntu@sha256:<index>` is created by the
amd64 pull pointing at the amd64 image; the arm64 pull resolves the same index digest to a different image ID, so the
store refuses it. Deterministic, which is why the re-run failed identically.

**Correction to the task brief.** The brief attributed this to the containerd image store. The runner's `docker info`
in the same log says `Storage Driver: overlay2` (classic graph driver, line 7122+ of the full log; the excerpt keeps
`Server Version: 28.0.4` and `Storage Driver: overlay2`). The error text and the source above are the graph-driver
reference store's. The fix below does not depend on which store is used.

## Resolved child digests

Source: anonymous token from `auth.docker.io` (`repository:library/ubuntu:pull`), then
`GET https://registry-1.docker.io/v2/library/ubuntu/manifests/sha256:fe81…ef852` with
`Accept: application/vnd.oci.image.index.v1+json`. Response `docker-content-digest` equals the pin
(`ubuntu-24.04-index-response-headers.txt`); `sha256sum ubuntu-24.04-index-sha256-fe815609.json` equals the pin.
`manifests[]` carries, for linux/amd64 and linux/arm64 (attestation-manifest entries with
`platform: unknown/unknown` ignored):

| platform | child manifest digest (OCI image manifest) | config digest | config `architecture` |
|---|---|---|---|
| linux/amd64 | `sha256:78d4744ead1f5b19b04d0df558ab182a5300903900755e515b2ca4d10eb2044d` | `sha256:d9b8c34d61a9b9a3c44cab883d7d2f2619d674966ab520186f799cf32e3e0377` | `amd64` |
| linux/arm64 (v8) | `sha256:19cad59657037cffa0ac9a48107019b768900ff1071140965827513ac866bd5e` | `sha256:1f1aeedbd16d617d1e09524c511cbbfb3b9748526f73ef36325366eee8347f8c` | `arm64` |

Files: `ubuntu-24.04-index-sha256-fe815609.json` (full index, raw), `ubuntu-24.04-amd64-manifest-sha256-78d4744e.json`,
`ubuntu-24.04-arm64-manifest-sha256-19cad596.json` (raw child manifests; each file's SHA-256 equals its digest, so the
pins are re-checkable offline with `sha256sum`).

Re-derive from scratch:

```bash
tok=$(curl -fsS 'https://auth.docker.io/token?service=registry.docker.io&scope=repository:library/ubuntu:pull' | python3 -c 'import sys,json;print(json.load(sys.stdin)["token"])')
curl -fsS -H "Authorization: Bearer $tok" -H 'Accept: application/vnd.oci.image.index.v1+json' \
  https://registry-1.docker.io/v2/library/ubuntu/manifests/sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852 \
  | python3 -c 'import sys,json;[print(m["platform"],m["digest"]) for m in json.load(sys.stdin)["manifests"] if m["platform"]["os"]=="linux" and m["platform"]["architecture"] in ("amd64","arm64")]'
```

## Fix and design choices

1. **Per-platform child digest pins** (`release.yml`, "Build deb and RPM..."). Each loop iteration uses a different digest
   reference, so the reference store never has to overwrite one. The supply-chain pin is kept and is no weaker: a child
   manifest digest is content-addressed, and both come from the pinned index entry. `--platform` is kept (qemu/binfmt
   selection and a mismatch warning).
   Rejected: `docker rmi` between iterations or `docker pull --platform` once per arch (still two pulls of one index
   reference; relies on engine cleanup behaviour); dropping the digest for a tag (unpins the base image).
2. **In-container architecture assertion** (`test "$(dpkg --print-architecture)" = "$EXPECTED_ARCH"`): with child pins
   the platform is no longer derived from the index, so an amd64 digest filed under arm64 would otherwise warn and
   silently build the wrong packages.
3. **Rehearsal of the whole publish path.** `workflow_dispatch` gains `dry_run` (boolean, default `true`);
   `sign-and-publish` now runs for a tag push OR a dispatch with `dry_run == true`.
   * **Publication cannot be reached by a dispatch:** `Create GitHub Release` has its own `if: github.event_name == 'push'`,
     independent of any input. `dry_run=false` is refused by a separate `refuse-dispatch-publish` job (fails loudly rather
     than skipping silently). A tag push behaves as before: the same steps, the same fatal guards, publication last.
   * **Tag-ref assumptions on a dispatch.** The ref is a branch. `Classify the tag` (still fatal) derives the tag the tree
     would carry, `v` + `[workspace.package].version`, and runs the unchanged grammar; the new `tag` output feeds the rc
     readiness, perf-label and notes-file steps (they previously read `github.ref_name`). The main-ancestry, readiness,
     aggregate/fuzz/perf and notes guards are **evaluated against the dispatched commit** and are `continue-on-error`
     only when the event is not `push`; a final `Rehearsal verdict` step (always runs on a dispatch) fails the job if any of
     them did not succeed and writes the per-guard table to the run summary. So the job is green only if a tag push at that
     SHA would pass every guard; a branch rehearsal is expected red on ancestry and the T3-ledger leg, but still runs the
     docker/package/sign steps. The `continue-on-error` lines carry the `ADVISORY — owner:...; reason:...` label required by
     `workflow_honesty_20_3a`.
   * A new dispatch-only step resolves every glob in the `Create GitHub Release` file list (`fail_on_unmatched_files`
     equivalent), read from the workflow file itself, so a missing artifact is found before the first real publication.
   * Secrets: `RELEASE_SIGNING_KEY` and `MAOS_RELEASE_PUBKEY` are repository secrets (`gh secret list`), the `release`
     environment has none and (2026-11-14) no protection rules and no deployment branch policy, so a dispatch from `main` or a
     branch can start the job and read them exactly as the push path does.

## Verified (observed)

* Registry evidence above (index digest = pin; child manifests' SHA-256 = digests; config architectures amd64/arm64).
* The rc.1 log lines and the moby source for the error text.
* Local static checks of the edited workflow: it parses; every `run:` script passes `bash -n`; the rehearsal classify step
  derived `v1.0.0-rc.1` / `kind=rc` from this tree's `Cargo.toml` and refused a branch name when `REHEARSAL=false`; the
  file-list step failed on an empty `dist/` and passed on a complete one; the verdict step failed on a failed or unreported
  guard and passed on success/skipped.

## UNVERIFIED (hosted)

* That `docker run --platform linux/arm64 ubuntu:24.04@sha256:19cad…` then the amd64 equivalent pull cleanly on a hosted
  runner under Docker 28.0.4 (expected from the moby source: distinct digest references; not run here: no docker daemon was
  used).
* Everything after the docker step in `sign-and-publish` (package metadata signing, notes guard, `assemble-release-packaging.py`
  output shape) has never run hosted; the rehearsal dispatch is what exercises it.
* That the `release` environment keeps accepting dispatches once protection rules/deployment branch policies are provisioned
  (`ops-provisioning-secrets-and-accounts`).
* The updated xtask tests (`release_workflow_15_4`, `release_gates_20_5r1`, `release_precondition_t3_20_5`) were edited
  but not compiled or run here (assignment forbids cargo).

Trigger: `gh workflow run release.yml --ref <branch> -f dry_run=true`.
