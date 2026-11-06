# 20-2 hosted-2 — non-cargo repair phase, 2026-11-06

Working baseline observed once at entry:
`063c13cfca5b67f8e0b9229bff2f917143d981b3`.
Main explicitly assigned cargo ownership to Dev201b. **No cargo, cargo fmt,
compiled renderer, build, Rust contract, KLOC/gate or shared-target cleanup ran
in this phase.** No commit, push, persistent Git configuration, status flip or
protected 20-11 source/evidence change was made.

Exact non-cargo commands/exits are in commands.json. Python dependency setup
used cached binary wheels (python-cryptography.log), not a Rust source build.

## Hosted result: 12 green,2 unintended reds,2 operator skips

Read the complete agent://Diag202Verify2 diagnosis. Read-only GitHub metadata
confirms [run42293312694](https://github.com/lunarpulse/maos/actions/runs/42293312694),
attempt1, push ci/20-2-verify, head
`f0c4c91796eae8147e758d5cebe24fd65b2878e8`, completed/failure.
Every job/step conclusion is preserved untruncated in hosted-job-results.jsonl.
The **per-leg table for all12 green jobs** is appended as a hosted-2 section in
[hosted-1/record.md](../hosted-1/record.md): builds on both architectures;
native deb/RPM builds and ownership controls on both; Debian negative/final
install/identity/purge/removal on both; RPM recipe controls/identity/removal;
four-package signed assembly; rendered version ordering; scoped netns replay;
amd64 container controls/init; and temp-root maos-bin Cargo/subject controls.
Those results are no longer labelled wholly unverified. They do not establish
Homebrew identity, Arch final installation, arm64 container runtime, universal
network absence, production publication/cosign, or environment protection.

Two operator jobs were excluded by the push/dispatch gate, not proven to have
skipped for missing credentials. No hosted run was retried here.

### Arch: real positive prepare failure after a correct negative lane

Job128160782826 passed the P5 bypass oracle (~332–334) and correctly refused the
wrong signature (469), with package absence at472. Its **good** makepkg prepare
then failed at523–529 (exit4):

`release verification refused: ValueError: not a regular artifact: maos-linux-amd64`

The recipe left all four makepkg source binaries as symlinks. This is a product
recipe incompatibility, not an intended red. The shared verifier's guard is
correct and remains byte-for-byte unchanged.

Changed packaging/aur/PKGBUILD:19–24: for each binary name, materialize a symlink
with `cp --remove-destination "$(readlink -f "$f")" "$f"` before the existing
one-line verifier. Already-regular files are left alone, avoiding a same-file
copy failure in repeated/direct preparation. package() continues installing the
same verified files from srcdir. The existing one-line P5 regex remains exact.

### Brew: missing process identity before install

Job128160783009 failed during P5 setup, before Homebrew installation:
Author identity unknown / empty identity at215–228; tap-new git commit exit128;
job failure at244. This is not forged-signature acceptance or rejection proof.

Changed packaging/witness-recipe.sh:23–27: after brew shellenv, export
GIT_AUTHOR_NAME/EMAIL and GIT_COMMITTER_NAME/EMAIL as maos-witness/witness@invalid.
Only this script and children inherit them; no git config or commit was run
locally. The same shared witness is called by discipline, so no duplicate
workflow identity setup was needed.

## Preempting later Brew failures without weakening audit

Changed packaging/homebrew/maos.rb:

- Ordered the actual Formula DSL metadata as desc/homepage/url/version/sha256/
  license/depends_on/skip_clean, then resources. Build dependencies are ordered
  cryptography before python@3.14.
- Replaced class RELEASES/SUFFIX constants with lexical releases/suffix/hashes
  values used while defining URLs/checksums; install computes its own identical
  suffix. Retained missing-platform refusal and all four binary hash indices.
- Preserved the same existing renderer tokens, all nounzip resource URLs and
  signature/manifest/verifier checksum tokens. Kept the exact two-line independent
  Python call, and verification still precedes every bin.install.
- **Kept brew audit --strict.** No audit exclusion, checksum bypass, verifier
  relaxation, alternative trust path or fallback was added.

Primary-source inspection resolved the resource-staging question:

- [NoUnzipCurlDownloadStrategy.stage](https://docs.brew.sh/rubydoc/NoUnzipCurlDownloadStrategy.html)
  invokes UnpackStrategy::Uncompressed.extract.
- [Current upstream uncompressed extractor](https://raw.githubusercontent.com/Homebrew/brew/main/Library/Homebrew/unpack_strategy/uncompressed.rb)
  extract_to_dir uses `FileUtils.cp path, unpack_dir/basename.sub(...), preserve: true`.
  It copies the payload rather than staging a source symlink. Existing resource
  staging is therefore retained; no speculative extra normalization remains.
- [ComponentsOrder implementation](https://raw.githubusercontent.com/Homebrew/brew/main/Library/Homebrew/rubocops/components_order.rb)
  checks DSL components against the precedence list; lexical data setup is not
  a newly invented Formula stanza. The canonical order is also documented in
  the [Homebrew API precedence list](https://docs.brew.sh/rubydoc/top-level-namespace.html).

These source reads describe current upstream behavior, not a version-locked
hosted Homebrew runtime. Ruby, gem, Homebrew and RuboCop are unavailable locally.
The formula has not been loaded or audited here; actual install/relocation and
all four installed-byte comparisons remain hosted proof.

Read the later Arch byte-identity, actual pkgver comparison, pacman -R and
four-binary absence checks. Their denominator/path/comparator/removal logic is
unchanged; no further defect was established from reading them. The standalone
rendered rc/alpha comparator leg is green in this hosted run, but that does not
promote the failed Arch install lane to a passing installed-identity witness.

## Non-cargo witnesses and observed results

All tests below are in packaging/test_release_20_2.py; none spawns cargo.

| Witness name:line | Before | Final observation |
|---|---|---|
| test_hosted_2_arch_prepare_materializes_all_symlinked_binaries:193 | Actual sourced prepare exits1 with exactly the reported not-a-regular-artifact reason | Real Ed25519-signed four-payload unit fixture: direct verifier still rejects symlinks; actual prepare makes all4 regular and verification passes; second already-regular prepare also passes |
| test_hosted_2_brew_identity_is_process_local_and_complete:230 | Isolated readonly git var exits128, Author identity unknown | Both author/committer identities resolve from actual witness exports; no global Git config/commit; strict audit invocation retained |
| test_hosted_2_brew_metadata_and_dependencies_follow_audit_order:248 | Metadata order assertion fails | Ordered DSL fields and alphabetic build dependencies pass static checks, not full Homebrew audit |
| test_hosted_2_brew_helpers_do_not_define_class_constants:257 | Old RELEASES assignment fails the structural assertion | No RELEASES/SUFFIX class constants remain; this is a preventive layout property, not a reported hosted constant offense |
| test_hosted_2_brew_resource_urls_hashes_and_verification_order_stay_bound:261 | Initial exploratory assertion assumed resource copying needed | Replaced that unsupported assumption after reading upstream copy behavior. Actual URL/hash associations and verify-before-install properties pass; no unnecessary resource workaround remains |

before-python-controls.log records20 tests with5 failing assertions. Only the
Arch/Git failures reproduce the two reported hosted failures; layout checks are
preemptive, and the initial resource-staging assumption was corrected, not
misrepresented as a product defect. All original15 controls remain present.

Final actual command:

`/tmp/maos-20-2-hosted2-dev202/venv/bin/python packaging/test_release_20_2.py -v`

**exit0,20/20**. Existing Brew and Arch P5 exact-match regressions pass after
reordering/materialization. Arch uses real signature/hash checks over clearly
labelled unit payload bytes, not native compiled binaries or makepkg execution.
The Git test uses `git -c user.useConfigOnly=true var` with isolated config:
it does not create a repository or commit and does not claim tap-new ran.

Shell syntax loop over packaging/aur/PKGBUILD and packaging/witness-recipe.sh:
**exit0**. Complete command/exits in commands.json. No Ruby syntax/runtime,
Homebrew audit, compiled renderer or Rust suite result is inferred.

## Precisely what remains for the cargo handoff

Implementation edits are complete for this non-cargo assignment. Main must
explicitly transfer the slot before the following verification phase:

1. Build trusted tools: `cargo build --locked -p xtask -p maos-cli`.
   No implementation edit is needed in xtask/src/render_packages.rs: it still
   supplies the unchanged BREW_RELEASES table and existing URL/key/hash tokens.
2. Run `cargo test --locked -p xtask --test package_recipes_20_2 --test
   package_workflows_20_2 --test release_workflow_15_4`.
   In particular xtask/tests/package_recipes_20_2.rs remains unmodified and
   unexecuted in this phase; this is pending regression verification, not a stub.
3. Invoke the actual trusted xtask render-packages CLI with a genuine signed
   artifact set and the trusted built maosctl. Refresh actual rendered AUR/Brew
   evidence and recipe/hash records under hosted-2 after that run. No manual
   substitution is presented as compiled-renderer proof; hosted-1 fixtures
   remain historical and were not overwritten.
4. With the shared Rust landing stable, the integration owner runs required
   formatter/gates and exact KLOC measurement. This pass changes no Rust source
   or token API and makes no pin/ceiling change; global pins have not been
   remeasured while the other coder is active. Re-pin only if counted source moves.

Separately, actual Brew load/strict audit/install identity and corrected Arch
makepkg install/identity/removal need a subsequent operator/hosted execution.
These are not cargo results and remain unverified, not claimed green because
static assertions pass.

Files changed: packaging/aur/PKGBUILD, packaging/witness-recipe.sh,
packaging/homebrew/maos.rb, packaging/test_release_20_2.py, story hosted-2 record/
Change Log, hosted-1/record.md append, and hosted-2 evidence. No Rust source,
shared workflow, verifier, pin, tracker or protected20-11 file changed.

## Ownership-safe cleanup

/proc scan observed one cargo and eleven rustc processes owned by the active
shared build window. **No shared target directory was inspected/pruned or
modified**; incremental cleanup is deferred to its owner, even if a later
instantaneous process scan becomes empty.

Removed own /tmp/maos-20-2-hosted2-dev202 and five import caches. Removed allocated
file-block accounting: **30,269,440B**; observed free-space delta on the scratch
filesystem: **0B** during concurrent compilation. No physical reclamation amount
is inferred from allocated blocks. Shared incremental removals0; no deps/build/
fingerprint deletion. Details are in cleanup.json.
