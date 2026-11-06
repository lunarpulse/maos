# 20-2 hosted-3 — Homebrew identity and strict-audit repair, 2026-11-06

Observed local entry HEAD: `96a1dce2308ef4c952ca84e3008905aff26c2645`.
Main explicitly retained cargo ownership with Dev201b. **No cargo, formatter,
Rust build/test, actual new renderer, standard gate, workspace nextest or
prepush ran in this phase.** No commit, push, persistent Git configuration,
tracker flip, protected 20-11 edit or shared-target pruning was performed.

Unlike hosted-2, this phase **does change counted Rust source** in
`xtask/src/render_packages.rs`. Exact formatted KLOC/pin updates are reserved
for the integration owner's cargo handoff; prior 44676/194743/194744 counts
are historical, not measurements of this working tree. Kernel source is
unchanged. Every subprocess command, return code and duration is preserved
in commands.json; expected red witnesses are not relabelled successful tests.

## Hosted result: 13 green, 1 unintended red, 2 operator skips

Read the complete agent://Diag202Brew diagnosis, then preserved read-only
GitHub metadata and complete Arch/Brew logs in this directory. No workflow was
re-run here. [Run 42300114383](https://github.com/lunarpulse/maos/actions/runs/42300114383),
attempt 1, push `ci/20-2-verify`, head
`100bb9f3f4aa012ef83642f212b3c88029ad52d8`, completed/failure:

- **Arch now passes**, job 128180762193. In arch-observed-success-raw.log,
  lines 336–337 show that the P5 verifier bypass was detected; 473–477 show
  InvalidSignature and package absence with the real verifier; 528 verifies
  all four regular signed binary files; 582 reports final install, exact
  installed-byte identity and package removal. This supersedes only the
  matching Arch-unverified limitation in hosted-2. It is an amd64 witness,
  not arm64 Arch or Homebrew proof.
- **Brew still stops before installation**, job 128180762283. In
  brew-observed-failure-raw.log, 258 initializes the tap, 259–271 report
  unknown author/empty identity, 272 reports git commit exit 128, and 288
  reports job exit 1. The P5 setup fails; the normal install/audit/test/
  identity/removal step is skipped. This is an unintended environment red,
  not a successful negative signature witness or a demonstrated formula bug.
- The other 12 successful jobs retain the earlier scoped proof: both default
  builds, both native-package builds, both Debian lanes, RPM, version ordering,
  netns, amd64 container, temp-root maos-bin Cargo and signed package assembly.
  Complete job/step conclusions are in hosted-job-results.log.
- Operator preparation/publication remain skipped by the push/dispatch gate.
  Missing credentials are not inferred, and publication/cosign remain unverified.

The first `gh api` log fetch exited 1 because gh refused terminal escape
sequences. The explicit `--allow-escape-sequences` fetch exited 0 and preserved
the complete log; this was a read-only retrieval correction, not a CI retry.

## Implemented corrections and source audit

### Process-local Homebrew identity

`packaging/witness-recipe.sh:27–28` now exports all four Homebrew bridge inputs:
HOMEBREW_GIT_NAME/EMAIL and HOMEBREW_GIT_COMMITTER_NAME/EMAIL. The ordinary
GIT_AUTHOR_* and GIT_COMMITTER_* exports remain. No git config or commit was
run locally. Homebrew's
[Utils::Git.set_name_email!](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/utils/git.rb)
assigns author/committer Git variables from precisely these Homebrew inputs.

The new local regression clears ordinary Git identity after the actual witness
exports and exercises the source-documented bridge with read-only `git var`.
It proves the required bridge inputs and resulting identities, **not execution
of Homebrew's own sanitizer or tap-new locally**. The old raw-Git test remains
as a separate control; it previously passed despite the hosted failure.

### Blocking audit with visible diagnostics

After restored positive installation, the witness runs `brew audit --strict`
before brew test or installed-byte comparison, captures its real status, and
prints its complete combined output. A failing audit still permits those later
checks and normal uninstall/untap to run, then fails the witness through the
saved status. Neither a nonzero audit nor its output is suppressed. On earlier
failures the existing EXIT trap still restores/cmp/touches inputs and attempts
package/tap cleanup. Positive install, brew test, strict four-binary installed
identity, uninstall, absence checks and untap are all retained.

### Concrete latent style and version issues

Reviewed the actual
[tap style root](https://github.com/Homebrew/brew/blob/main/Library/tap_rubocop_style.yml),
[inherited configuration](https://github.com/Homebrew/brew/blob/main/Library/.rubocop.yml),
[component ordering](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/rubocops/components_order.rb),
[class/test checks](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/rubocops/class.rb),
[line checks](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/rubocops/lines.rb),
[URL checks](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/rubocops/urls.rb),
[checksum checks](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/rubocops/checksum.rb),
[public API restrictions](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/rubocops/non_public_api_usage.rb),
and [runtime resource auditor](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/resource_auditor.rb).
These were source reviews of upstream main, not a locally executed strict audit.

- Historical actual renderer output has a **294-character checksum row**
  (hosted-1/rendered-dev-fixture/maos.rb:14; measured in
  historical-checksum-layout.json). The configuration's limit is 118 and its
  WordArray minimum is 4. The four checksum strings were emitted as a single
  quoted array. The renderer now emits multiline `%w[...]` with one unquoted
  64-character digest per line and aligns hash rockets to the widest **present**
  supported suffix. A one-platform Linux fixture therefore gets no fictitious
  Darwin-column padding. Existing quoted Arch checksum generation is unchanged;
  hashes and four-binary index order are unchanged. Direct string appends avoid
  adding a formatting allocation for each Brew row.
- The runtime resource auditor rejects an explicitly supplied version when it
  equals the URL-detected version. Homebrew's
  [Version parser](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/version.rb)
  recognizes GitHub `/releases/download/vX.Y.Z/` paths. The formula now supplies
  the rendered version only when `Version.detect(stable.url)` differs from it.
  This retains the explicit version needed for file URLs, architecture-number
  misdetections and prerelease URLs, without a redundant stable release version.
  The [Formula DSL](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/formula.rb)
  exposes stable.url; no additional resource URL/hash token or renderer token
  was introduced. Resource blocks are visually separated with blank lines.
- Class-body lexical releases/suffix/hashes and the local hash freeze remain.
  No constant is made mutable or introduced. The NonPublicApiUsage and
  source-only binary URL restrictions explicitly apply to official/core taps;
  they are not evidence that this third-party tap's existing class-body odie
  or binary URLs must be removed. Unsupported-platform refusal remains.
- Explicit skip_clean paths remain (never deprecated :all). SPDX any_of with
  Apache-2.0 and MIT remains. Dependencies remain alphabetic, canonical named
  cryptography and python@3.14 build inputs. Runtime compatibility of their
  installed Python modules has **not** been established locally.
- Primary and resource URLs retain nounzip and exact signed filenames/checksum
  associations. Resource-name/PyPI-only restrictions do not rename these assets.
  SoftwareSpec.owner= supplies the parent version where a resource has none;
  no speculative per-resource version was added. Homebrew's nounzip staging
  copies regular files, as recorded in hosted-2.
- The test stanza remains a real version command/assertion, not an empty/true
  block. Its command includes a space, so the strict bare-bin-path interpolation
  restriction does not apply. Desc/homepage, quote style, source whitespace,
  frozen-string-comment formula exclusions, and bin.install rename mapping
  were inspected; no other source-supported violation was identified. This is
  **not** a claim that all current/future Homebrew cops or actual formula loading
  have passed.

## Linux installed-byte identity: retained, not weakened

Source inspection distinguishes source installation from bottle relocation:

1. [FormulaInstaller](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/formula_installer.rb)
   refuses pouring with build_from_source, cleans the built keg, then calls
   fix_dynamic_linkage at finish. Placeholder/ELF relocation is invoked by pour,
   not this source-install path.
2. Generic [Keg#fix_dynamic_linkage](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/keg_relocate.rb)
   only relativizes prefix symlinks. The
   [Linux installer override](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/extend/os/linux/formula_installer.rb)
   explicitly describes that behavior and skips it when no absolute links exist.
3. [Cleaner](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/cleaner.rb)
   observes explicit skip_clean paths; the
   [Linux Cleaner override](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/extend/os/linux/cleaner.rb)
   only changes executable classification.
4. [Linux Keg relocation](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/extend/os/linux/keg_relocate.rb)
   really can patch ELF RPATH/interpreters during bottle relocation. skip_clean
   is **not** represented as an ELF relocation opt-out.

Therefore the witness still compares **the actual installed bin directory**
against all four signed inputs after `--build-from-source`. No staged-copy
substitution, identity fallback, private preservation override or tolerated
mismatch was added. Source analysis does not replace hosted proof: if the
runner's Homebrew rewrites those bytes, the strict check will fail visibly.

## Local witness table (AC2 follow-up controls)

All names below are in packaging/test_release_20_2.py. Before/plant entries are
actual exits from this phase, not hypothetical failing assertions.

| Witness name:line | Before | Deliberate plant | Final |
| --- | --- | --- | --- |
| test_hosted_3_brew_identity_survives_plain_git_environment_reset:248 | exit1, missing HOMEBREW_GIT_NAME | remove both Homebrew export lines, exit1 | pass |
| test_hosted_3_brew_audit_is_logged_blocking_and_precedes_installed_checks:272 | exit1, missing logged/status-preserving audit | replace saved-status terminal guard with colon, exit1 | pass |
| test_hosted_3_brew_strict_style_uses_multiline_checksum_word_arrays:283 | exit1, old single-line renderer | remove %w from renderer output, exit1 | pass |
| test_hosted_3_brew_explicit_version_only_overrides_url_version_when_needed:292 | exit1, unconditional version | restore unconditional version, exit1 | pass |

Each planted production file was restored with **cp, cmp, touch**, all exit0;
commands and individual plant logs are retained. The initial run had exactly
three failures among 23 tests; the additional version contract separately
failed before its change. Final Python **24/24**, exit0, and
`bash -n packaging/witness-recipe.sh`, exit0. Existing signed-byte, wrong-key,
Arch symlink, exact P5 match, manifest, trace and provenance controls remain.
The new renderer/version/audit guards are source contracts, not Ruby execution.

## Explicit integration/hosted boundary

The integration owner must build trusted xtask/maosctl, run the existing Rust
recipe/workflow/release contracts, and regenerate a genuinely signed fixture
with the **new compiled renderer** before claiming rendered output validation.
The retained old rendered fixture remains historical and was not overwritten.
After the shared Rust landing stabilizes, Main must run the prescribed formatter,
measure exact counts and update the matching pins/hardfail together, then handle
required gates/verification in its one cargo slot. No new KLOC result is claimed.

Actual Homebrew load, bridge-driven tap creation, Python dependency import,
strict audit, restored installation/test, installed identity and cleanup after
these changes remain **HOSTED-UNVERIFIED**. The branch-only recipe remains under
story evidence ci/, with push limited to ci/20-2-verify; no workflow was installed
on main. Production publication/signing, arm64 container runtime and prior proof
limits not specifically superseded above remain unverified.

## Ownership-safe cleanup

Cleanup /proc scan observed **1 cargo and 1 rustc**. Shared target/incremental
pruning was skipped because Dev201b still owned cargo. Removed only own
/tmp/maos-20-2-hosted3-dev202 (1133 non-symlink files; Python tests disabled new
bytecode writes). Allocated file-block removal **30,257,152 B**; observed scratch
filesystem free-space delta **0 B** during concurrent work. These are distinct
measurements, not inferred physical reclamation. Shared incremental removals0;
no deps/build/fingerprint deletion. See cleanup.json.

## Files changed and deferred receipt

- packaging/witness-recipe.sh: process-local Homebrew bridge and visible,
  status-preserving strict audit before other post-install checks.
- packaging/homebrew/maos.rb: nonredundant explicit version and resource spacing.
- xtask/src/render_packages.rs: multiline checksum word arrays with present-key
  hash-rocket alignment; no Arch checksum or token-API change.
- packaging/test_release_20_2.py: four additional before/plant-sensitive controls.
- Story Dev Agent Record/Change Log, hosted-2 supersession append, and this
  hosted-3 evidence directory: exact observations and remaining proof boundary.
- deferred-work.md:149: replace the stale blanket no-runtime-proof wording with
  the scoped development-key branch receipt, retaining ops-rc-signed-tag as open
  owner and requiring matching signed-tag evidence before rc acceptance. No NFR
  acceptance, universal air-gap assertion or tracker status was flipped.
