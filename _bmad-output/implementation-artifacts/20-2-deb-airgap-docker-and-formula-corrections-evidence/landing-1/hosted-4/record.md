# 20-2 hosted-4 — executable mode and four Brew audit findings, 2026-11-06

Observed entry HEAD: `d84b83dd34d07b67d06c64240f451380bf0cbcef`.
Main explicitly assigned **no cargo**, with Dev201b retaining the shared slot.
No cargo, formatter, Rust build/test, gate, nextest, prepush or hosted rerun was
performed here. No commit, push, tracker flip, protected 20-11 edit or shared
incremental pruning occurred. This pass changes **no Rust source or exact pin**;
no fresh global KLOC measurement is claimed.

## Hosted observation, not a rerun

Read agent://Diag202Brew3 completely and retained the actual raw log, run metadata
and every job/step conclusion. [Run42305526387](https://github.com/lunarpulse/maos/actions/runs/42305526387),
attempt1, push ci/20-2-verify, head
`c74d49bad4bf032d7c274e8c5b5bbf0c8eadfffc`, completes failure with **13 green,
1 unintended red,2 operator skips**. Brew job128196643322 now gets past tap
creation and actual installation, but fails before installed identity:

| Retained brew-observed-failure.log lines | Observation and limit |
| --- | --- |
| 228–309; explicit acceptance at246,309 | P5 removes the verifier and detects acceptance of the forged signature. Intended negative-control red, working. |
| 318–326 | Homebrew-specific identity bridge permits tap creation. This supersedes the earlier tap-new-unverified limitation. |
| 440–460; InvalidSignature at453,460 | Actual Python/cryptography verifier rejects the wrong-key signature. This establishes actual import and verifier execution, not just a static dependency claim. |
| 461–472 | Restored good recipe installs successfully. Not yet executable/install-identity proof. |
| 556–566 | Strict audit reports the four real unintended findings listed below. The witness preserves and prints the audit failure. |
| 576–583 | brew test reaches installed maos --version, gets Permission denied and exit126. This is the first fatal step; the final saved audit-status check was not reached. |
| 602–664; exit1 at667 | Existing EXIT trap restores inputs, uninstalls and untaps. Cleanup actually ran despite the test failure. |

The other thirteen jobs' scoped conclusions and the two operator skips remain
as recorded in hosted-job-results.log. Production publication/cosign is not
proved by a push-gated skip. No failure was retried to green here.

## Applied fixes

### 1. Inverse condition (rendered19 / template13)

Use `version "@VERSION@" if Version.detect(stable.url).to_s != "@VERSION@"`.
This is the exact form requested by the observed audit; behavior is unchanged
from unless/equality. The existing version-source regression now requires this
style, rather than preserving the form the hosted cop rejected.

### 2. Public Python opt-path helper (rendered57 / template now52)

Use `system formula_opt_bin("python@3.14")/"python3.14", "verify-release.py",`.
Confirmed the exact helper, not an inferred spelling:

- [Homebrew/FormulaPathMethods](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/rubocops/formula_path_methods.rb)
  maps opt_bin to formula_opt_bin and emits the unqualified helper inside a
  Formula DSL. It does not request retaining Formula[...].opt_bin.
- [Utils::Path](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/utils/path.rb)
  implements formula_opt_bin(name) as formula_opt_prefix(name)/bin.
- [Formula](https://github.com/Homebrew/brew/blob/main/Library/Homebrew/formula.rb)
  includes Utils::Path, so this is available in install. Formula also includes
  FileUtils for the explicit chmod below.

The trusted Homebrew Python interpreter, cryptography dependency, PYTHONPATH,
verifier script, public key and four-file verification denominator are unchanged.

### 3. Expanded public-key width (rendered58 was132/118)

Bind `pubkey = "@PUBKEY@"` on its own line, then use
`"--pubkey", pubkey, *names.map { |name| "#{name}-#{suffix}" }` as the second
line of the verifier call. All eligible lines in a complete representative
substitution, including a real-length64-hex key and digest tokens, are now at
most118 characters. The previous template-only width test missed this; the
new test genuinely reports132 before the fix. This substitution is a style
fixture, not a claim that the compiled renderer was executed locally.

### 4. Executable installed files (fatal exit126 and audit finding)

Retain the literal `names.each { |name| bin.install ... }`, then add exactly
`chmod 0555, names.map { |name| bin/name }`. Keep all four explicit skip_clean
paths. The nounzip download's0644 mode no longer survives installation; this
operation changes mode only and does not strip or rewrite signed bytes.

The local mode control starts with four real0644 files, creates a genuine
Ed25519-signed four-entry manifest, and verifies it with the actual independent
Python verifier. It runs POSIX chmod using the mode extracted from the formula,
checks all four0555 modes and unchanged SHA256 digests, and verifies the signed
manifest again. This proves mode/byte behavior with real files and signatures,
**not actual Ruby FileUtils or a local brew install**. A0444 plant fails with
actual mode292 versus expected365 (octal0444 versus0555).

## P5 integration and caller migration

Updated only the branch-recipe pattern in evidence/ci/ci-20-2-verify.yml:388,
from system Formula[...] to system formula_opt_bin(...). The verifier remains
an exact two-line statement; `assert count==1` remains blocking. Public-key
binding, installation and chmod are outside that deliberate verifier removal.

Migrated both Python references to the old system call and extended the actual
pattern test: it removes exactly one call from both the template and a64-hex-key
expanded variant, removes the --pubkey argument line, and retains the binding,
bin.install and chmod. Restoring the obsolete pattern gives actual count0 and
fails the control, rather than producing a misleading hosted plant result.

Searched packaging, xtask/tests, tests, .github and the branch-recipe consumers.
No other active reference pins the old Python call. Historical evidence remains
historical. No .github/workflows copy, production workflow, renderer token API,
release trust root or installed-byte comparison was changed in this pass.

## Whole-formula recheck and version output

Reviewed the complete template before editing, the four actual strict-audit
messages and the upstream helper cop/API. Rechecked changed constructs against
[Homebrew's inherited configuration](https://github.com/Homebrew/brew/blob/main/Library/.rubocop.yml):
118-character eligible-line width, double quotes, direct public path helper,
inverted-unless preference, and zero-prefixed octal literals. The new local
pubkey is used, is not a mutable constant, and leaves the two-line call's
continuation alignment intact. `%w[...]` binary names and checksums, present-key
hash alignment, metadata/dependency order, SPDX license, nounzip URLs/resources,
explicit clean paths, class-body locals/freeze, bin.install and test-block
structure remain as they were in the hosted formula that reported only these
four problems. No additional source-supported issue was identified; this is
not a claim that a new complete Ruby/Homebrew audit ran locally.

The test stanza remains `assert_match "maos", shell_output("#{bin}/maos --version")`.
Source inspection confirms **crates/maos-bin/src/verbs.rs:325–326** writes
`maos <CARGO_PKG_VERSION>`; both main.rs:1496–1497 and1826–1829 dispatch the version
outcome through that function. A local source contract checks the literal output
and formula expectation together. No cargo or binary smoke run was needed to
confirm the source string; corrected hosted execution is still owed.

Strict audit remains blocking and its output visible. The actual installed bin
directory still must match all four signed release inputs after brew test.
There is no staged-copy fallback, tolerated checksum mismatch or relocation
opt-out. Uninstall/absence/untap and the EXIT restoration trap remain unchanged.

## Actual local witnesses

All test citations are packaging/test_release_20_2.py. These are the final lines;
source-based controls are not misrepresented as Ruby execution.

| Witness name:line | Before | Plant | Final |
| --- | --- | --- | --- |
| test_hosted_3_brew_explicit_version_only_overrides_url_version_when_needed:296 | exit1 on unless/== | reverted unless/==, exit1 | pass |
| test_hosted_4_brew_uses_public_formula_path_helper:328 | exit1 on Formula lookup | reverted Formula lookup, exit1 | pass |
| test_hosted_4_brew_expanded_tokens_fit_strict_audit_width:334 | exit1,132>118 | inline64-hex key, exit1,132>118 | pass |
| test_hosted_4_brew_sets_executable_mode_without_changing_signed_bytes:351 | exit1, no chmod | actual0444 mode instead of0555, exit1 | pass with signed before/after verification |
| test_hosted_1_brew_plant_removes_exact_actual_two_line_verifier:163 | prior form worked before callee migration | obsolete pattern, exit1,count0 | pass,count1 in template and expanded variant |
| test_hosted_4_brew_test_matches_maos_cli_version_output:381 | pass | no product behavior altered | pass |

The initial28-test suite had exactly4 failures, matching the four repairs.
Four deliberate formula regressions were installed together and the four
corresponding focused controls all failed. The obsolete workflow-regex plant
failed separately. Every production plant was restored with **cp, cmp, touch**,
all exit0. The mode test was then strengthened with real Ed25519 verification.

Final command:
`PYTHONDONTWRITEBYTECODE=1 /tmp/maos-20-2-hosted4-dev202/venv/bin/python packaging/test_release_20_2.py -v`
— **exit0,28/28** (final-signed-mode-python-controls.log).
`bash -n packaging/witness-recipe.sh` — **exit0**.
All exact subprocess commands, exit codes and durations are in commands.json.

## Remaining boundary and changed files

Changed packaging/homebrew/maos.rb, packaging/test_release_20_2.py, the story's
branch-only ci/ci-20-2-verify.yml pattern, story hosted-4/Change Log, hosted-3
supersession append and this hosted-4 evidence. No Rust source, KLOC pin,
shared witness script, verifier, protected20-11 file, deferred owner or tracker
status changed. Counted Rust source delta from this pass is0; no fresh global pin
measurement is inferred while the shared cargo slot is held elsewhere.

Actual corrected brew test, zero-offense strict audit, installed-byte identity
and restored normal-lane cleanup remain **HOSTED-UNVERIFIED**. Previous hosted
proof now specifically covers tap creation, actual Python verifier/import,
wrong-key rejection and successful good-recipe installation; it does not cover
executable mode or installed identity. Rust contracts, actual renderer refresh,
fmt/gates and any integration verification remain Main's cargo responsibility.
No pending local implementation step is hidden as a follow-up.

## Ownership-safe cleanup

/proc scan observed1 cargo,0 rustc and32 clippy-driver processes. Shared target
pruning was skipped by ownership, not made conditional on finding an idle
instant. Removed own /tmp/maos-20-2-hosted4-dev202,1132 non-symlink files:
**30,269,440 B** of allocated file blocks. Observed scratch-filesystem free-space
delta **0 B** during concurrent work; this is not inferred physical reclamation.
Shared incremental removals0; no deps/build/fingerprint deletion. See cleanup.json.
