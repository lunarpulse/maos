# 20-2 review corrections — observed evidence and operator limits

Baseline: `88c7277c0967d2abb8bb89f5f52e4e5a7ffe46e8`, after Main committed the
separate 20-11 corrections above the initial 20-2 landing `93d39cc7`.
Dev202 honored the tracked-tree/cargo hold until Main's explicit CARGO HANDOFF.
No commit, push, tracker/status flip, protected 20-11 edit, devcontainer change,
CARGO_TARGET_DIR override or cargo clean was performed. Cargo used the existing
`target -> /mnt/build/cargo-maos` symlink; the environment variable was absent.

P1–P5 and security F1–F10/F14 are implemented. F3's repository settings and all
hosted acceptance remain unverified, not waived. F11–F13 and remaining action
refs have explicit same-line rule-10 dispositions in deferred-work.md:1443–1450.
The original acceptance.md is a historical pre-review record, not current line
references or a current hosted verdict.

## AC witness table (current name:line; before/plant; final)

`py` below means `packaging/test_release_20_2.py`; `wf` means
`xtask/tests/package_workflows_20_2.rs`. All native/operator recipe references
are to `../../ci/ci-20-2-verify.yml`, kept outside .github/workflows on main.

| AC | Current witness name:line | Before / plant | Final observed / limitation |
|---|---|---|---|
| AC1 | `ReleaseControls.test_ac1_forged_signature_refused` py:52; `review_p1_ubuntu_rpm_has_explicit_checked_dependency_mode` wf:114; `review_p2_runner_owns_output_directory_before_root_containers` wf:120 | Original deb failures remain in historical evidence. Before workflow suite exits101 on P1/P2. Real verifier signature-check removal makes the enrolled Python command fail. Operator `native-packaging`:46 authors Ubuntu RPM-database refusal and ordinary-runner PermissionError; `deb`:189 authors wrong-key build refusal/restoration. | Python signature/exact-name controls pass; P1/P2 wiring tests pass. Explicit rpm-ubuntu checks real Python Ed25519 import before --nodeps; Fedora rpm mode retains dependency checking. Actual deb/RPM native amd64/arm64 build/install/removal remains UNVERIFIED. |
| AC2 | `ac2_versions_preserve_prerelease_ordering_shape` `xtask/tests/package_recipes_20_2.rs`:16; `test_review_p5_wrong_key_control_updates_outer_hash_not_trusted_key` py:116; `review_p5_all_recipe_routes_require_wrong_key_rejection` wf:137 | New unsupported prerelease vectors fail before fix (exit101); before P5 wiring exits101. Real outer-signature-hash-update removal reds Python. Operator `recipes`:362 removes each actual verification call, requires forged-signature acceptance to be detected, restores cp/cmp/touch; then real wrong-key rejection/valid install/identity/remove. | Three renderer tests and Python crypto/hash controls pass; freshly built real four-binary release set is signed/rendered/independently verified locally, all exit0. Actual rendered Debian alpha compares before GA with dpkg (exit0). Native Arch vercmp unavailable locally; operator `version-ordering`:140 tests rc.1/alpha.1 renderer output. RPM/AUR/Brew install/audit/identity controls UNVERIFIED. |
| AC3 | `test_review_f6_sendmmsg_cannot_hide_an_outbound_attempt` py:85; `test_review_f6_io_uring_is_uninspectable_not_a_clean_witness` py:89; `review_f8_netns_artifacts_are_verified_before_execution` wf:211; `tests/air-gap-netns-corroborate.sh`:48,51 | Before Python trace controls exit1; before F8 workflow wiring exits101. Historical nonisolated replay/typed compile-shape refusal remain recorded. Operator `netns`:250 independently verifies downloaded binary bytes before any execution, then authors baseline/outbound plant/restoration/final replay. | Trace controls and verification-order wiring pass locally. sendmmsg explicitly traced; uninspectable io_uring submissions cannot produce a clean witness. Claim narrowed to no outbound attempts in the witnessed replay session, not all possible behavior. Netns/TL-growth/outbound plant remain hosted UNVERIFIED. |
| AC4 | `xtask/src/check_ship_gate_completeness.rs`:130; operator `cargo-and-symbols`:413 | Original T0 observed network symbols and selected D-02 C; historical default-subject plant is authored. No unobserved symbol scan is relabelled green. | Current ship-gate completeness exits0. Advisory symbol result is not deployment proof. Hosted subject plant UNVERIFIED; original local advisory/dirty-fixture observations retained. |
| AC5 | `ac5_login_push_and_sign_are_all_credential_gated` wf:51; `review_f2_operator_prepare_cannot_build_tag_code_with_publish_permissions` wf:155; `review_f4_critical_release_actions_are_full_commit_pinned` wf:181; `review_f5_and_f10_container_bases_and_owned_state_are_explicit` wf:200; `Dockerfile`:17 | Before F2/F4/F5-F10 wiring assertions exit101. Changed-byte crypto vector passes. Operator `container`:299 authors real binary-byte rejection and missing-owned-state default-volume init refusal, restores cp/cmp/touch, then positive init. | Current wiring tests pass. Separate contents-read preparation builds only workflow-own-ref tooling; verifies both signed denominators before handing off assets. Only separate environment-referenced publish jobs have packages/OIDC write. Both Hub credentials gate login/push/sign. Runtime state owned65532. Action/base resolution proven only, not execution. Actual Docker/build/default-volume smoke/arm64/push/cosign UNVERIFIED. |
| AC6 | `purge_dry_run_is_inert_then_removes_only_maos_owned_state` `crates/maos-bin/tests/maos_uninstall_16_4.rs`:180; `packaging/witness-recipe.sh` | Historical actual old-hint plant exit101, restored full suite23/23; not rerun merely to confirm in this correction pass. Real native package removal remains authored. | No purge implementation changed in this correction. Docs truthfully separate runtime-data purge from installer-owned binary removal; native removal UNVERIFIED. |
| AC7 | `docs/maos.dev/install.md`:75; operator `cargo-and-symbols`:413 | Historical missing Cargo route witness and old-hint plant retained. Current release native binaries genuinely rebuilt. | Correct four-package git/tag Cargo route documented. Actual temp-root Cargo install/version/uninstall hosted leg remains UNVERIFIED. |
| AC8 | `tests/integration/onb_nfr2_timing.sh`:115–181; current-size.log; measurements.json; gate logs | Historical size breach/closure retained. First review measurement honestly exits1 on exact pin drift (+3 xtask); pins updated together. First deferred-owner gate exposes five citation-grammar errors; records repaired, not a test-budget waiver. | Exact current size oracle on a copy exits0:31,936,280B <33,554,432B; glibc2.39. Kernel/kloc/ship/exit/decision/dev-record/epic/coverage/test-cost final gates exit0. No done/status flip. Hosted proof remains assigned to the existing operator lane. |

## Corrections and security boundaries

- **P1/P2:** native Ubuntu RPM calls explicitly opt into apt-backed mode only
  after importing cryptography. Runner owns `dist/packages` before root builds.
  Native hosted recipe exercises both original failures and both architectures,
  then merges four real packages for signed-manifest assembly.
- **P3:** refuse numeric-leading prereleases and empty dot identifiers; retain
  rc.1/alpha.1. Removed temporary core-version Vec allocation. No kernel change.
- **P4/P5:** real Python controls are enrolled in blocking native release-dry-run
  after cryptography provisioning. Wrong-key controls update only outer signature
  download hashes for AUR/Brew; trusted keys remain unchanged. Require actual
  InvalidSignature, no installed payload, byte restore, positive identity/removal.
- **F1:** the second signed manifest authenticates all four package files plus
  verify-release.py, maos.rb, PKGBUILD, maos.spec and Debian recipe archive.
  Trusted source/local tooling bootstraps verification; downloaded code cannot
  authenticate itself. Unit package bytes are explicitly opaque fixtures, not
  claims of a native package build.
- **F2/F3:** operator inputs never become checkout/build refs. Preparation uses
  github.workflow_sha, contents-read permission, nonpersisting checkout, trusted
  built tools and both signed manifests. Publishing is a separate job because
  GitHub permissions are job-scoped. Production public key must be nonempty and
  not the documented development key. `environment: release` is referenced but
  protection configuration is UNVERIFIED: environment API returned404/exit1.
  It cannot distinguish absence from unavailable visibility. GitHub documents
  automatic unprotected creation for an unknown referenced environment. Required
  reviewers/prevent-self-review/bypass restrictions/protected tag rules remain
  operator-owned in provisioning-checklist and the deferred row, not claimed fixed.
- **F4/F5:** four critical action families are immutable commit refs with version
  comments. The five base image index response hashes exactly matched registry
  digest headers, with the required platforms present. resolved-pins.json records
  provenance; this is resolution-only evidence, never a Docker execution result.
- **F6/F7/F8/F9/F10/F14:** narrow replay-only outbound claim, sendmmsg/io_uring
  handling, truthful deb/RPM build-time versus end-user verification docs,
  authenticate netns downloads before execution, exact tag/SHA/id/attempt success
  selection, owned runtime state, and Homebrew preservation explicitly
  **claimed, not verified**. Protected production environment settings are not
  silently assumed from an authored reference.
- **F11–F13/rest:** existing unrelated workflow interpolation and persisted writer
  checkout credentials remain open, not bulk-edited. perf.yml is owned by20-7 L2;
  fuzz-prerelease by20-5; ownerless items have explicit next-touch dispositions.
  Integer/SHA/status constrained sources are not misreported as proven arbitrary
  injection exploits. F15's documented development seed is informational/public.

## Exact measurements and nonrenewing rebase

| Item | Handoff exact | Current exact | Review delta | Ruled rebase ceiling |
|---|---:|---:|---:|---:|
| maos-bin |34914|34914|0|34969|
| xtask |44673|44676|+3|44920|
| aggregate |194740|194743|+3|195042|
| aggregate hardfail |194741|194744|+3|195043 bound|
| kernel physical/tokei |26841/21042|26841/21042|0|unchanged|
| iac/audit/registry |8622/8174/4412|8622/8174/4412|0|unchanged by20-2|

Original 20-2 consumed pins were bin34823, xtask44673, aggregate194099. Unrelated
20-11 drift adds bin91 and aggregate641 only. Carry those onto the original
D02 A ceilings34878/44920/194401 →34969/44920/195042; do not renew the original
x2 measured / x3 unmeasured allowance on each review pass. Current exact pins
are measured after cargo fmt --all; aggregate hardfail is exactly measured+1.
The first xtask HISTORY segment names this live story and 44673 ->44676.

## Observed commands, failures and final verification

Every recorded local command, actual exit and elapsed time is in commands.json;
separate logs retain output. Read-only action/image/environment API observations
are in resolved-pins.json. The public development signing key used by the fixture
is not a production secret.

- Before: cargo version controls exit101; separate workflow controls exit101
  (11 review assertions fail); Python sendmmsg/io_uring controls exit1.
- Real combined plant changes manifest membership, release-run binding, outer
  signature hash update and actual verifier signature checking. The enrolled
  `python3 packaging/test_release_20_2.py -v` exits1 with9 assertion/subtest
  failures. All four files are copy-back/cmp/touch restored (each exit0).
  Final Python12/12 passes (exit0), including actual crypto rejection and exact
  manifest asset hash rejection. This does not substitute for hosted recipe runs.
- `cargo test --locked -p xtask --test package_recipes_20_2 --test
  package_workflows_20_2 --test release_workflow_15_4 --test
  decision_adrs_and_provisioning`:22/22, exit0.
- `cargo nextest run --locked -p xtask --test d11_xtask_ceiling_ratchet --test
  recovery_lane_ceiling_rule`:21/21, exit0. Final provisioning contract3/3, exit0.
- `cargo build --locked -p xtask -p maos-bin -p maos-cli`:exit0.
  `cargo build --release --locked -p maos-bin -p maos-cli -p maos-spirit-cli
  -p maos-registry`:exit0 (70.756s).
- `cargo fmt --all`:exit0. `cargo clippy --locked -p xtask -p maos-bin
  -p maos-cli --all-targets --message-format=json`:exit0;690 warning events,
  zero primary diagnostics on changed renderer or changed test files. Not
  warning-free, not a hosted compiler or full pre/post warning-baseline proof.
- Fresh genuine four-binary manifest generation/sign/render/independent signature
  verification all exit0. Real rendered Debian alpha-before-GA dpkg check exits0.
  No local vercmp binary exists; rc/alpha native comparator proof remains hosted.
- Exact size-oracle source lines115–181 executed on a copy:exit0,
  31,936,280B <33,554,432B. readelf dynamic-symbol glibc floor2.39. Full
  onb_nfr2_timing.sh not run because it invokes forbidden cargo clean.
- Final standard gates all exit0: check-kernel-baseline, kloc-check,
  check-ship-gate-completeness, check-exit-commands, check-decision-register,
  check-dev-record-completeness, check-epic-close-coherence, coverage-matrix,
  check-test-cost. First dev-record gate exit1 on five new-row citation-grammar
  errors; corrected to explicit same-line Dispositioned by, final exit0. A
  misspelled check-coverage-matrix exits2 before work; actual coverage-matrix
  exits0. Shell syntax loop over the four affected scripts exits0.

### Workspace/prepush are not green claims

No retry of the original one explicit workspace nextest or one prescribed
prepush was performed in this correction pass. Historical workspace run
24d43741-bd54-4575-943f-6c2fea571c5d exited100:5646 run/5640 pass/6 fail/138 skip.
Four original landing-caused reader/enrollment failures were repaired; the
system-Python cryptography and cold-swap failures were reported to Main, not
relabelled known flakes or retried to green here. Main's separate 20-11 work
now provisions cryptography in discipline; this correction did not conflict
with that step. The original prescribed prepush hit its2900s limit, wrapper124,
without final buffered JSON. It is incomplete, not passed or waived. Neither
historical result is represented as current-tip full-workspace validation.

## Files changed by this correction pass

- Production Rust: xtask/src/render_packages.rs; exact pins xtask/kloc.toml.
- Existing tests: xtask/tests/{package_recipes_20_2,package_workflows_20_2,
  release_workflow_15_4}.rs; packaging/test_release_20_2.py;
  tests/{air-gap-trace.py,air-gap-netns-corroborate.sh}.
- Packaging: build-packages.sh, witness-recipe.sh; new permanent witness/release
  helpers assemble-release-packaging.py, forge-recipe-signature.py,
  release-run.py, wait-release.sh. verify-release.py was only a temporary plant
  subject and was restored byte-for-byte, not a final source change.
- Delivery: Dockerfile and existing .github/workflows/{discipline,release,
  container}.yml; only the evidence ci/ci-20-2-verify.yml holds the operator
  branch recipe. No verify workflow was installed on main.
- Docs/records: docs/maos.dev/install.md, ADR-047, provisioning-checklist,
  deferred-work, story Dev Agent Record/frontmatter, historical acceptance
  label, and this review-corrections evidence directory. Protected 20-11
  sources/evidence, tracker and unrelated workflow files intentionally unchanged.

## Cleanup and remaining operator work

/proc and open-descriptor scan first: no active compiler and no open incremental
fd; two transient/unreadable proc entries recorded. Removed1257 stale s-*
snapshots, kept7863 including newest per crate and anything touched in2h. Own
/tmp/maos-20-2-review-dev202 and five Python import caches removed. No deps,
build, fingerprint or unrelated scratch was deleted.

Allocated-file-block removal: **8,378,310,656B** (own scratch124,551,168B).
Observed filesystem free-space increase: **7,757,279,232B**. Hardlinked file
blocks counted only when all links removed; compression/reflinks/concurrent
host activity mean these are distinct measurements, not equivalent claims.
See cleanup.json.

All hosted native package/netns/arm64/Docker runtime/Brew identity/Cargo route/
Docker Hub/GHCR push/keyless cosign/pre-tag rehearsal results remain UNVERIFIED.
Operator must configure/verify release environment/tag protections and run the
branch-only recipe, retaining run/attempt IDs and actual negative/final logs.
Implementation corrections are complete; overall hosted acceptance is not
claimed complete and no story status was changed.
