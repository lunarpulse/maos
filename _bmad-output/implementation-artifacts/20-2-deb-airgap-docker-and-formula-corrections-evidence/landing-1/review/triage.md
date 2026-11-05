# Story 20-2 adversarial review triage

## Scope and method

Authoritative candidate: `93d39cc7ca185f513dee2d97bed2058f11554d9a`, parent `cbed4dad` (supersedes the original a118eec4 request). Read-only review of 20-2 implementation, story AC1–AC8 / ruled D rows, acceptance record, and the operator recipe at `../../ci/ci-20-2-verify.yml` (repository path: `_bmad-output/implementation-artifacts/20-2-deb-airgap-docker-and-formula-corrections-evidence/ci/ci-20-2-verify.yml`). Unrelated 20-11 content excluded.

Executed skill steps 01–04 non-interactively as requested, with independent parallel Blind Hunter, Edge Case Hunter, Acceptance Auditor and Test Infrastructure Auditor. No security layer spawned: orchestrator owns it. The tool rejects explicit model overrides for reviewer roles, so configured `reviewer` / `review-light` agents were used. No layer failed. Source was read before triage. No builds, tests, formatters, hosted runs, or mutation plants were executed. All failing-before witnesses below are requirements for the patch owner, not claimed observations. Conclusions about host/container permissions and RPM dependency resolution are static deductions, not observed runs.

Explicit assignment restrictions override the workflow's normal story/tracker writes and interactive checkpoints: only this review artifact is written; story status and sprint status are unchanged. Findings remain action items, not applied patches.

**Counts: 5 patch (2 high, 3 medium), 1 defer (medium), 3 dismiss; 0 decision-needed.** Independent security findings are not included in these counts.

## Patch

### P1 — RPM release build checks an empty Ubuntu rpmdb [high]

- Sources: blind; confirmed from the complete release call path.
- Locations: `.github/workflows/release.yml:164-168`; `packaging/build-packages.sh:22`; `packaging/rpm/maos.spec:9`.
- The tag job invokes `rpmbuild -bb` inside Ubuntu after apt installs `python3-cryptography`. The spec declares `BuildRequires: python3-cryptography`, which rpmbuild resolves through the RPM database, not apt's database. There is no RPM-installed provider. Therefore the newly authored release path refuses before `%prep`; the Fedora witness uses dnf and cannot expose this discrepancy. Packages and the release never publish. [INFERENCE from package-manager semantics; unexecuted.]
- Exact fix: keep dependency validation for the Fedora witness; give the Ubuntu release invocation an explicit, narrowly scoped `--nodeps` mode only after checking that `python3` imports `cryptography`, or build RPMs in a native RPM distribution with the required dependency installed. Do not silently disable dependency checking for every caller. Preserve the original signed-byte verification in `%prep`.
- Required failing-before witness: execute the actual Ubuntu release packaging lane on a non-root hosted runner for at least amd64, retain `Failed build dependencies: python3-cryptography` before patch, then retain successful `%prep` signature verification and package creation after patch. Run the corrected arm64 lane as part of final hosted acceptance; Fedora success alone is not this witness.

### P2 — Root-owned package directory prevents host manifest assembly [high]

- Sources: blind + edge.
- Locations: `.github/workflows/release.yml:163-179`; `packaging/build-packages.sh:6-7,17,23`.
- Containers run as root and create `dist/packages` with normal 0755 permissions. Host runner Python then calls `source.replace(target)` to move each output out of that root-owned directory. Rename requires write permission on the source directory, not merely ownership of `dist` or readability of the package. The runner lacks it, so assembly of `SHA256SUMS-packages` fails after packaging. [INFERENCE; unexecuted.]
- Exact fix: pre-create `dist/packages` as the runner before starting the root containers, or `sudo chown -R "$(id -u):$(id -g)" dist/packages` after the loop and before the Python block. Preserve root inside containers for package dependency installation.
- Required failing-before witness: run the real bind-mounted container/host handoff as an ordinary runner, record directory uid/mode and the host `PermissionError` before patch, then demonstrate the same host user moves all four package outputs, emits the four-entry manifest and reaches signature self-verification after patch. Do not run the host Python as root as the purported fix/witness.

### P3 — Arch mapping accepts prereleases that collapse or outrank GA [medium]

- Source: edge; source and existing renderer fixtures read independently.
- Locations: `xtask/src/render_packages.rs:8-27`; `xtask/tests/package_recipes_20_2.rs:16-34`.
- `versions("1.0.0-1")` produces Arch `1.0.01`, whose final numeric component is greater than GA's `0`; `versions("1.0.0-.")` is also accepted and produces exactly GA `1.0.0`. This renderer is reused for every tag, so it must reject unsupported prerelease spellings rather than emit incorrect package ordering. The current rc.1 and alpha.1 examples do not exercise this branch. [Comparator result is unexecuted inference; returned strings follow directly from source.]
- Exact fix: keep `rc.1` / `alpha.1` supported, reject empty dot-separated prerelease identifiers, and reject a numeric-leading overall prerelease if it cannot be represented faithfully by the current Arch mapping. Test the first character of the whole prerelease, not every identifier (`rc.1` includes a valid numeric identifier). Alternatively implement a comparator-proven faithful mapping for numeric-leading SemVer; do not silently emit `core + digits`.
- Required failing-before witness: a renderer regression test must fail before patch on `1.0.0-1` and `1.0.0-.`; add `rc..1` / `rc.` refusal vectors, preserve rc.1/alpha.1 success, and use real `vercmp` on renderer-emitted supported prerelease values against their GA values. Unsupported inputs must fail rendering before writing recipes.

### P4 — Real crypto and trace regression suite has no CI enrollment [medium]

- Source: test infrastructure; no references found under `.github`, `xtask`, `tests` or `packaging`.
- Locations: `packaging/test_release_20_2.py:45-79`; `.github/workflows/discipline.yml:5845-5850` (suitable existing prerequisite/job context).
- The six real Python vectors are cited as control evidence but no normal pipeline invokes the file. Rust source/YAML tests do not execute the Python verifier or trace classifier. Their successful local run is honestly recorded, but regressions in exact-name rejection or trace-family classification are not protected by those vectors in CI.
- Exact fix: run `python3 packaging/test_release_20_2.py` in a blocking discipline job with `python3-cryptography` explicitly installed; retain nonzero status and bind the job to aggregate (or add the invocation to an existing blocking job that provisions it). Keep the real signature and trace assertions.
- Required failing-before witness: a narrowly scoped mutation that accepts a prefix-only manifest filename or misclassifies the supplied outbound vector must leave the old enrolled pipeline without this test and must make the newly enrolled Python step red; restore it and show the suite green. Merely adding a source-text assertion naming the file is insufficient.

### P5 — RPM, Arch and formula witnesses never challenge verifier wiring [medium]

- Source: test infrastructure.
- Locations: `packaging/witness-recipe.sh:8-18,21-36,38-50`; operator recipe `ci/ci-20-2-verify.yml:194-217`.
- These lanes only install a correctly signed distribution and compare installed hashes. Removing a recipe's call to `verify-release.py` leaves that entire happy-path witness green. The wrong-key deb plant proves Debian wiring, not the other three package-manager controls. Shared Python vectors alone do not prove that a recipe actually calls the verifier.
- Exact fix: for each of RPM, Arch and Homebrew, stage a wrong-key signature, update any recipe-level manifest/signature download checksum needed to let the package manager reach the independent verifier (without changing the trusted pubkey), assert rejection specifically at signature verification and no installed payload, restore all bytes, then run the valid install/identity/remove leg. Do not accept an outer download-checksum mismatch as proof of Ed25519 wiring.
- Required failing-before witness: plant removal/bypass of each recipe's verifier invocation; the new negative lane must go red because the forged signature is accepted, then green after restoring the verifier. Retain genuine wrong-key rejection messages for each manager and restored successful signed identity/removal evidence.

## Defer — pre-existing

### D1 — Default nonroot data volume is not provisioned writable [medium]

- Sources: acceptance + main reading.
- Location: `Dockerfile:12,16-17`.
- Runtime sets `MAOS_HOME=/var/lib/maos` and declares the volume, but never creates an owned directory for uid/gid 65532. A default fresh volume is therefore not demonstrated writable; `--version` / `--help` cannot detect data-write failures. The same ENV/VOLUME/nonroot arrangement exists in parent `cbed4dad:Dockerfile`, so this is not introduced by the signed-binary cutover. [Unexecuted filesystem/volume consequence.]
- Suggested follow-through: create an empty data directory in the verification stage and copy it with `--chown=65532:65532` into the runtime, then exercise `maos init` as the image's configured user with a fresh default volume. Reuse the existing container ownership workstream rather than inventing an unrelated fix here.

## Dismissed findings (3)

1. **Advisory symbol scan must fail on baseline symbols / compare a frozen symbol allowlist.** D-02 C explicitly selects advisory observation after measured T0 red. The job prints the actual output/status and is not a ship badge. Converting it into a new blocking symbol-baseline mechanism is not authorized by that ruling. The blocking default-build netns job remains enrolled.
2. **Literal comparator examples alone constitute a separate broken mapping.** The tests also assert exact renderer mappings for rc.1/alpha.1, and the literals check those format semantics; no separate current-release break was demonstrated. The concrete accepted-input ordering defect is retained as P3, with actual emitted-value comparator coverage required there.
3. **The YAML presence test must prove every hosted step cannot become `run: true`.** Its header explicitly separates wiring from hosted execution; it does not claim source checks replace hosted proof. Missing assertion strength alone is not a demonstrated production defect. Concrete unenrolled Python vectors and absent recipe-wiring challenges remain P4/P5.

## Focus checks and evidence boundaries

- `container.yml` is tag-push-only; publication flag is job env derived from both Hub credentials. Unconditional build/smoke precedes registry login. Login, multi-platform push, cosign installation and signing are all credential-gated. `contents: read`, `actions: read`, `packages: write`, `id-token: write` are present. The operator lane is explicit-dispatch-only for publication. Security-layer review owns token exposure, mutable action/base-image references, unauthenticated verifier distribution and signer trust policy; none is silently waived here.
- Release PR rehearsal cannot reach `sign-and-publish` because that job requires push; build receives `MAOS_RELEASE_PUBKEY` (public verification material), not the signing private key. Existing PR trigger semantics and default permission hardening are not reclassified as newly introduced defects without evidence.
- Docker context allowlist includes only Dockerfile, the shared verifier and the two named release binaries/manifests; only verified `maos` is copied into the distroless runtime. Base image references are mutable tags, not digests; no digest pinning/provenance proof is claimed by this review. Runtime nonroot derives from the `:nonroot` base, not an explicit USER line.
- Shared verifier verifies Ed25519(SHA256(manifest)), rejects malformed/duplicate entries, matches exact filenames, and rejects symlink/nonregular requested payloads before hashing. Formula includes SHA-256 for payloads, manifest, signature and verifier; no placeholder/hash omission was found by source reading. Its Linux package-manager execution remains unverified, including Python/Homebrew dependency resolution and byte preservation.
- Netns prerequisite failure exits 1 under declared CI, 77 locally; no successful green-skip branch remains. Canary is mandatory, real replay expected response and TL row growth are asserted, tracer follows children and observes the AC's named connect/sendto/sendmsg operations. No trace-parser evasion finding was substantiated; no expansion beyond the AC to every conceivable networking syscall is inferred. The actual no-outbound result is still unknown until hosted execution, and AC3.8 STOP must be honored if it finds an attempt.
- Compile-shape `run` explicitly refuses with `BuildShapeUnsupported`, exit 2, and no longer writes the fake TL stub. Purge hints name the real four-package Cargo removal set and all package managers. The exact reported pin changes agree with source/evidence: maos-bin 34823, xtask 44673, aggregate 194099/hardfail 194100; no independent measurement was executed.
- `landing-1/acceptance.md` and story explicitly mark native lifecycle, recipe execution, netns, Docker build/push/sign, arm64 and hosted plants **UNVERIFIED**. They also report prepush timeout 124 and the non-green workspace execution rather than claiming success. Those labels are honest limitations, not completion evidence. The story remains not done. Authoring a hosted lane does not satisfy the hosted ACs, and P1/P2 show why execution is indispensable.

## Operator handoff

No new design ruling is necessary for P1–P5. Apply patches and record required failing-before/restored witnesses through the orchestrator. Existing operator-controlled hosted recipe/rehearsal runs, signing/registry credentials, final-tree discipline and a completed required prepush remain outstanding; this review does not authorize a done flip or publication. Merge this triage with the orchestrator's independent security report before landing.
