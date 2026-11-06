# 20-2 hosted-1 fix pass — 2026-11-06

Baseline main HEAD `ebf2ab03ac64de5b828d995f72ae8b24199fa9a0`.
Assignment: fix the first hosted package/witness reds, inspect the previously
unreached later steps, and apply the parent-authorized model-currency calibration
only after trying a cheap honest helper reduction. This is not a new review
round. No commit, push, tracker/status flip, protected 20-11 edit, target-dir
override, cargo clean, workspace/prepush rerun or hosted rerun was performed.

Exact local commands, actual exits and elapsed time are in commands.json;
neighboring logs preserve their output. CARGO_TARGET_DIR was absent; cargo used
the repository's existing target -> /mnt/build/cargo-maos symlink.

## Hosted diagnosis and provenance

Read the complete read-only diagnosis at agent://Diag202Verify. Its log citations
below are 1-based lines from each job's gh api actions/jobs/<id>/logs response,
not source lines. User/parent-reported failures were treated as ground truth,
not rerun to confirm. Separate read-only run-metadata API calls observed:

- [verify42224184744](https://github.com/lunarpulse/maos/actions/runs/42224184744):
  attempt1, push ci/20-2-verify, head6fda747d9d9733e6b310e201cba6eb5ac367fd5a,
  completed/failure; **7 failed,6 green,3 skipped** in the diagnosis.
- [discipline42224167600](https://github.com/lunarpulse/maos/actions/runs/42224167600):
  attempt1, push main, head ebf2ab03ac64de5b828d995f72ae8b24199fa9a0,
  now completed/failure. The diagnosis originally inspected it while still running;
  no partial snapshot is presented as its final overall verdict.

### Seven verify failures: all unintended

| Group | Job id / log lines | Actual first cause | Intended control never reached |
|---|---|---|---|
| A | native amd64 127923778461:1405–1407; native arm64 127923778425:1403–1405; RPM recipe127923778506:426–428 | PRODUCT: RPM rejects %global suffix as a built-in macro | Ubuntu dependency refusal / forged-signature acceptance detection |
| B | deb amd64 127923778427:366–369; deb arm64 127923778433:363–366 | Provisioning: build-essential:native absent | Wrong-key InvalidSignature; later install/identity/purge/removal |
| C | Arch recipe127923778508:356–369 | PRODUCT: SHA256 checks pass, then makepkg mistakes raw SHA256SUMS.sig for OpenPGP and fails GPG verification | prepare's independent Ed25519 check and its bypass control |
| D | Brew recipe127923778571:186–188,206–209 | Witness: plant regex matches zero calls, assertion ('brew',0) | Brew execution, signature rejection or acceptance, restoration |

Deb baseline missing-changelog refusal **was** correctly observed:
amd64 job127923778427:336–337 and arm64 job127923778433:333–334.
That does not make the next prerequisite failure an intended signature red.

Discipline independently reproduced missing build-essential in release-dry-run:
amd64 job127913349470:4932–4935; arm64 job127913349160:4872–4875.
Its package-recipes, air-gap-netns-witness and container-build were then blocked
through needs:release-dry-run, not intentionally excluded operator legs.

### Six verify green jobs: exactly what they established

| Job | Observed log lines | Proof and limit |
|---|---|---|
| build amd64 |2334,2983,3871| Four real default release artifacts, manifest signing and authenticated rendering; not native package execution |
| build arm64 |2328,2977,3862| Same on arm64; not package install/removal |
| version-ordering |1934–1938| Actual rendered rc.1 and alpha.1 compare before GA with Arch vercmp and Debian dpkg; not RPM/package-install proof |
| netns |252;329–332;383–385| Four downloaded binaries independently authenticated; outbound plant detected; restored replay response/journal growth and zero outbound attempts in that session. Replay-session corroboration only, not universal network absence |
| container |473–474;564–565;632–680| Changed binary rejected; missing-owned-state init refused for permissions; restored amd64 image runs version/help and initializes /var/lib/maos. No arm64 runtime, registry push or cosign proof |
| cargo-and-symbols |451–453;2933–2938| Actual temp-root maos-bin install/version/uninstall and forbidden-mock scan; advisory network-symbol scan explicitly remains exit1. Not a clean symbol scan or four-package Cargo install proof |

operator-prepare was skipped by its dispatch/nonempty-tag gate on a push;
operator-publication was skipped through that dependency. These are designed
operator exclusions, **not demonstrated missing-credential skips**. Four-package
packaging-assembly was skipped because native-packaging failed: an evidence
blocker, not an intentional acceptance exclusion.

Main also reported model_currency_gate's hosted budget failure (below). The
separate IAC perf-count instruction +4.1% versus2% tolerance finding at
job127913331230:2676–2678 is not attributed to packaging and is untouched.

## Changes and local red-before/final witnesses

| Fix | Source | Local failing-before witness | Final observed |
|---|---|---|---|
| A RPM macro | packaging/rpm/maos.spec:13,15,17–20,30,36 | hosted_1_rpm_artifact_macro_does_not_redefine_reserved_suffix, xtask/tests/package_workflows_20_2.rs:227, cargo exit101 on reserved %global suffix | Renamed both definitions and all9 references to maos_artifact_suffix; regression and fresh actual renderer pass |
| B all Debian builders | discipline.yml:5849; verify recipe:68,207; release.yml:169 | hosted_1_every_debian_builder_provisions_build_essential, package_workflows_20_2.rs:238, cargo exit101 on missing provisioning | build-essential added to both reported sites AND previously unreached native/production Ubuntu containers. Test covers all4 builder locations; no dpkg-buildpackage -d |
| C raw signature transport | packaging/aur/PKGBUILD:12,16 | test_hosted_1_arch_raw_ed25519_signature_is_not_openpgp, packaging/test_release_20_2.py:151, Python exit1 on .sig source filename | Alias SHA256SUMS.ed25519::.../SHA256SUMS.sig; prepare copies to expected name before Python check. Original download hash and independent verification retained |
| D real Brew mutation | verify recipe:388 | test_hosted_1_brew_plant_removes_exact_actual_two_line_verifier, Python:161, exit1 because actual pattern matched0 | Pattern matches same-line system+verifier and following --pubkey continuation; exactly1 assertion retained; regression passes |
| Extra latent Arch plant | verify recipe:389 | test_hosted_1_arch_verifier_bypass_reaches_package_instead_of_false_exit, Python:174, exit1; actual sourced prepare returns1 | Deleting final verifier left false x86_64 [[ CARCH=aarch64 ]] selection as function result. Deliberate bypass now ends successfully, allowing makepkg to reach the intended accepted-forged-signature oracle; regression executes actual mutated prepare and passes |

The first cargo witness invocation hit its600s compilation deadline before any
test result. Its process group was terminated. Resumed compilation then ran both
new tests and produced the two actual failing assertions (exit101). No timeout
was substituted for a red-before assertion, and no failed test was retried to
green without the corresponding source fix.

### Real rendered Arch prepare control (not makepkg execution)

Trusted existing native release binaries were copied into isolated scratch,
freshly manifested/signed with the documented public development seed and
rendered through the actual built xtask/maosctl. Actual sourced rendered prepare
ran with CARCH=x86_64 and Python cryptography:

1. Valid Ed25519 alias: exit0; all four exact binary hashes verified.
2. Real wrong-key signature planted in SHA256SUMS.ed25519: exit1 with
   **release verification refused: InvalidSignature** (arch-real-prepare-wrong-key.log:1).
3. Both signature names restored with copy-back/cmp/touch: exit0.
4. Actual restored prepare: exit0.

The alias still receives the signature's original SHA256 download check in the
recipe. Local direct prepare testing does not claim makepkg's transport/install
path ran. rpmspec, rpmbuild and makepkg are absent. dpkg-buildpackage exists;
dpkg-query reports no python3-cryptography provider (and a blank debhelper
version), so a complete native Debian build is not claimed. No native dependency
checks were bypassed or host packages installed.

rendered-dev-fixture/ preserves the actually generated recipes and original
binary manifest; measurements.json records their hashes. These are DEVELOPMENT
witness bytes, not production release recipes. Their file URLs refer to removed
scratch; payload binaries are intentionally not committed as evidence.

### Later-step inspection

Read native Ubuntu build/ownership commands, rpm-ubuntu/Fedora dependency paths,
Debian rules and rootless build declaration, all RPM/Arch/Brew identity/removal
steps, installed-byte oracle and four-package signed-manifest assembly. Fixed
the two additional Debian provisioning sites and the latent Arch bypass return
status above. Source ownership keeps runner-owned directories before root writes;
assembler consumes exactly4 package files and signs recipe/verifier/archive
bytes through Rust verification (native '~' filenames are accepted there).
No new defect is claimed without evidence, no oracle is weakened, and the next
hosted run is not promised green. Native package installation/identity and
second-denominator assembly remain pending actual hosted execution after fixes.

## Parent-authorized model-currency cost calibration

Observed hosted run42224167600: **Σ32.983s across26 tests >30s default**.
Green baseline cb0abc06 was29.82s: only0.18s headroom. Parent assigned ownership
to20-4, whose constructor/default scan increased the cost; no20-2 production
model-currency code changed.

First tried a cheap honest helper change: share one COPY of the actual swept
tree across all11 retired/unknown-pin plant vectors, restoring each surface
between vectors. No vector or production audit was dropped or cached away.
Matched focused test: **5.92s before vs5.96s trial**, no material reduction.
All26 trial cases passed, local Σ21.989s; that local value is not evidence of a
hosted improvement. Reverted the ineffective helper with cp/cmp/touch, leaving
model_currency_gate.rs byte-identical to baseline.

Applied exactly ONE exception in xtask/test-budgets.toml:79–83:

- binary xtask::model_currency_gate;
- seconds42 = ceil(32.983×1.25), per existing R3 calibration policy;
- owner20-4-gemini-driver-and-endpoint-pin;
- reason contains hosted/baseline observations and the reverted trial;
- default30 and suite total1100 unchanged.

hosted_1_model_currency_calibration_changes_only_its_binary_budget at
xtask/tests/check_test_budget_proven_red.rs:275 replays the observed aggregate
as a clearly labelled unit input through the actual budget audit: default30
refuses, committed42 accepts, only that binary is excepted. It does not fabricate
26 individual hosted timings. All26 restored tests pass locally; final JUnit
Σ**12.520s**, distinct from trialΣ21.989s. Host load differences/local speed are
not used to erase or relabel the authoritative hosted32.983s observation.

## Final local verification and unchanged exact pins

- Python controls15/15, exit0.
- cargo test --locked -p xtask --test package_recipes_20_2 --test
  package_workflows_20_2 --test release_workflow_15_4 --test
  check_test_budget_proven_red:35/35 (3+16+2+14), exit0.
- cargo nextest run --locked -p xtask --test model_currency_gate:
  26/26, exit0, no dropped vectors; JUnit copies retained for trial/final.
- cargo fmt --all:exit0.
- Real manifest/sign/render, rendered PKGBUILD bash syntax and actual
  independent prepare control: results above, exact commands in commands.json.
- Scoped gates: kernel baseline, kloc-check, check-test-cost and
  check-dev-record-completeness all exit0.
- **No counted Rust source moved:** xtask44676, aggregate194743,
  hardfail194744, maos-bin34914, kernel26841 physical/21042 tokei unchanged.
  No re-pin, ceiling or headroom change was necessary; existing rebased ceilings
  stand. model_currency_gate.rs was restored, not a final changed file.

Files changed: RPM and AUR templates; packaging/test_release_20_2.py;
.github/workflows/{discipline,release}.yml; evidence ci/ci-20-2-verify.yml;
xtask/tests/{package_workflows_20_2,check_test_budget_proven_red}.rs;
xtask/test-budgets.toml; story hosted-1/Change Log/File List and this evidence.
Homebrew formula, production Rust, kloc pins, tracker, protected20-11 sources
and unrelated workflow files intentionally unchanged.

## Cleanup and remaining proof

/proc/open-descriptor scan first found no active compiler or open incremental
file descriptor (two transient/unreadable entries recorded). Removed190 stale
incremental s-* snapshots; kept7769, including newest per crate and anything
touched within2h. Removed own /tmp/maos-20-2-hosted1-dev202 and five import caches.
No deps/build/fingerprint or unrelated scratch was deleted.

Allocated-file-block removal: **2,768,224,256B** (own scratch154,988,544B).
Observed filesystem free-space increase: **2,442,948,608B**. Count hardlinked
blocks only if all links were removed; compression/reflinks/concurrent activity
mean these measurements are not interchangeable. See cleanup.json.

The next hosted run must exercise the corrected native/package-manager paths
and unblocked assembly. Existing scoped netns/container/version/Cargo proofs
are recorded above, not discarded; unexecuted native install/identity/assembly,
arm64 container runtime, production registry publication/signing and environment
protection settings are not claimed verified. This fix pass performs no push,
status flip or retry-to-green of the original hosted/workspace/prepush runs.

## hosted-2 (2026-11-06; non-cargo repair phase)

Run [42293312694](https://github.com/lunarpulse/maos/actions/runs/42293312694),
attempt1, push `ci/20-2-verify`, head
`f0c4c91796eae8147e758d5cebe24fd65b2878e8`: completed/failure with
**12 green,2 unintended reds,2 operator skips**. Read-only job/step metadata
and the full `Diag202Verify2` diagnosis were read; no hosted check was rerun.

| Green leg | Job id | What this run proved, with scope |
|---|---:|---|
| build amd64 |128152034757| Four real native default artifacts, development manifest signing and verified rendering; not production-key publication |
| build arm64 |128152035192| Same native artifact/sign/render path on arm64 |
| native-packaging amd64 |128160782831| Correct Ubuntu RPM dependency-refusal reason, real deb/RPM builds, root-owned-directory failure and runner-owned-output renames |
| native-packaging arm64 |128160782712| Same native package/ownership path on arm64 |
| deb amd64 |128160783669| Baseline missing-changelog refusal, wrong-key build rejection/restoration, signed installed-byte identity, init/purge and package removal |
| deb arm64 |128160782853| Same Debian negative/final install/identity/removal path on arm64 |
| recipes rpm |128160782906| Bypass acceptance oracle, independent wrong-key rejection, restored valid RPM installation/identity/removal and comparator check |
| packaging-assembly |128163020684| Four real package outputs assembled with verifier/recipes/archive; second denominator signed and self-verified using development fixture key |
| version-ordering |128160782788| Actual renderer outputs for rc.1/alpha.1 compare below GA with Arch vercmp and Debian dpkg |
| netns |128160782813| Download verification, outbound-attempt plant detection, restored replay/TL growth and no outbound attempts in that session; not universal network absence |
| container |128160782932| amd64 changed-byte and owned-state controls plus restored version/help/init; not arm64 runtime or registry/cosign proof |
| cargo-and-symbols |128152035092| Temporary-root maos-bin install/version/removal and subject/mock controls; network-symbol scan remains an advisory, not a clean-network proof |

operator-prepare128152036861 and operator-publication128152037237 were
excluded by the push/dispatch gate. These are not demonstrated credential skips.

**Arch red:** job128160782826 passed P5 acceptance detection and then correctly
rejected the wrong key (log469). Its good prepare failed at log523–529:
`not a regular artifact: maos-linux-amd64` (exit4). The source files staged as
symlinks had not been materialized. `PKGBUILD` now copies each of the four
binary links to a regular file before verification; already-regular files are
left intact. The verifier's symlink guard remains unchanged.

**Brew red:** job128160783009 failed before install at log215–228/244:
`Author identity unknown`, tap-new git commit exit128. The common witness now
exports all four GIT_AUTHOR/GIT_COMMITTER name/email variables after shellenv,
covering both verify and discipline without persistent git configuration.

Preempted the formula's metadata/dependency ordering issue while keeping
`brew audit --strict`: url/version/sha256 precede license/dependencies/skip_clean;
checksums/platform are lexical locals rather than class constants, and install
computes its own platform suffix. Resource URL/hash associations and the exact
two-line verifier call are preserved. Current upstream nounzip extraction uses
`FileUtils.cp`, not symlinks, so original resource staging is retained rather
than adding an unsupported copy workaround. See the cited upstream sources and
limits in [hosted-2/record.md](../hosted-2/record.md).

Non-cargo regressions: actual Arch signed symlink fixture failed before the fix,
then passed with all four regular files and also on a second regular-file prepare;
Git identity failed128 before the export and now passes readonly `git var`
checks. Layout/class-constant checks red before restructuring. Final Python
controls20/20 and shell syntax pass. Both P5 regexes still match exactly one
verifier call. Ruby, Homebrew and RuboCop are absent locally: actual formula load,
strict audit, staging/install/relocation and installed identity remain unverified.

**Cargo remains explicitly deferred to Main's handoff:** no cargo, formatter,
compiled renderer, Rust contract, KLOC gate, shared-target cleanup or rendered
evidence regeneration ran while Dev201b owns the slot. No Rust source/token
interface changed: `render_packages.rs` still supplies the existing BREW_RELEASES
map. After handoff, run trusted renderer/build + package Rust contracts and
refresh rendered/hash evidence; remeasure/re-pin only if counted Rust moves.
Precise pending commands/steps are in hosted-2/record.md. No commit/push/flip.

