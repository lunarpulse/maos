# 15-4 release repair and first signed tag — historical sections archive 2026-10-27

Verbatim relocation of historical sections of `15-4-release-repair-and-first-signed-tag.md` (done story) on 2026-10-27, by operator ruling. Each section below holds the body that followed the same-named heading in the story, copied **verbatim**; the original line span at relocation time is recorded in a comment under each heading. The story keeps each heading with a one-line pointer here, and keeps all frontmatter, Acceptance Criteria, Tasks, Decisions, Dev Agent Record and Review Findings sections unchanged.

## 1. 🔴 THE RELEASE DIES TWICE AND THE SECOND DEATH IS SILENT. THE AC NAMES ONE.

<!-- original lines 60-106 -->

**Death 1 — reproduced at HEAD.** `release.yml:67-69` is the only `download-artifact` in the
repository with neither `name:` nor `merge-multiple:`:

```yaml
67:      - uses: actions/download-artifact@v4
68:        with:
69:          path: dist
```

v4 nests each artifact under `path/<artifact_name>/`, and `:50`/`:53` give the *file* and the
*artifact* the same name, so `dist/maos-linux-amd64` becomes a **directory containing a file of
the same name**. Measured against that exact layout:

```
$ cd dist && sha256sum maos-* > SHA256SUMS
sha256sum: maos-darwin-arm64: Is a directory
sha256sum: maos-linux-amd64: Is a directory
sha256sum: maos-linux-arm64: Is a directory
EXIT=1        SHA256SUMS: 0 bytes, created
```

Three in-repo precedents make `merge-multiple: true` the idiomatic fix: `multi-provider.yml:47-51`,
`fuzz-cadence.yml:117-121`, `discipline.yml:2364-2368`.

**Death 2 — NOT in the AC, and it is the dangerous one.** `release.yml:90-93`:

```yaml
90:          files: |
91:            dist/maos-*
92:            dist/SHA256SUMS
93:            dist/SHA256SUMS.sig
```

Under the nested layout `dist/maos-*` matches the three **directories**. `softprops/action-gh-release`
filters glob matches to regular files before upload — read from the action's source, not measured
here, which is precisely why the story's own control is `fail_on_unmatched_files: true` (T2) rather
than a claim about someone else's code. The published release would carry a manifest and an Ed25519
signature **over binaries that were never uploaded** — and exit 0. It is invisible today only
because `:73` kills the job four steps earlier. **A repair that patches `:73` alone (for example
`sha256sum maos-*/maos-*`) leaves `:91` intact and ships that release.** `merge-multiple: true`
is the single fix that closes both, which is why D-15-4-B takes it and then removes the globs
entirely.

**Not a third defect.** `release-verify --verify --artifacts-dir .` at `:86` resolves by
`dir_path.join(&entry.filename)` (`xtask/src/release_verify.rs:123-135`) and hard-errors
`manifest entry '{}' not found in artifacts dir` at `:127-131` in strict mode
(`verify_release(..., false)` at `:146`). It is honest and would simply fail loudly.

## 2. 🔴 `maos-*` DOES NOT MATCH `maosctl-*`. THE STORY'S HEADLINE ADDITION IS EXCLUDED BY THE GLOB THAT WAS SUPPOSED TO HASH IT.

<!-- original lines 112-139 -->

Epic 20 fixes the naming convention this story must produce (`epic-20…:14,:16,:18`):
`./dist/{maos,maosctl,maos-registry-server,maos-spirit}-linux-amd64`. Under it,
`maos-registry-server-linux-amd64` and `maos-spirit-linux-amd64` **do** match `maos-*` (literal
`maos` then `-`). `maosctl-linux-amd64` has `c` at position 5. Measured:

```
$ touch maos-{linux-amd64,linux-arm64,darwin-arm64} maosctl-{linux-amd64,linux-arm64,darwin-arm64}
$ for f in maos-*; do echo "  $f"; done
  maos-darwin-arm64
  maos-linux-amd64
  maos-linux-arm64
$ ls maos-* | wc -l   →  3          $ ls | wc -l   →  6
```

So with AC1 implemented as written, `maosctl` would be built, uploaded as a workflow artifact,
downloaded into `dist/`, and then **omitted from `SHA256SUMS` (`:73`), omitted from the signature,
and omitted from the GitHub Release (`:91`)** — three times, silently. The AC's own headline
deliverable would be absent from the release it was added to.

This matters beyond tidiness: `packaging/homebrew/maos.rb:5` says the formula *"re-verifies
SHA256 + Ed25519 (fail-closed) by running `maosctl install --verify-only --from-local
<staged-dir>` post-download"*, and `docs/runbooks/release-signing.md:13-15` already records
`RELEASE_PUBKEY` as *"Bundled in binary | **Every `maos` / `maosctl` binary**"*. **The runbook and
the brew formula both already assume a `maosctl` artifact that `release.yml:44` has never built.**

⚠ **The oracle is therefore a COUNT, not a presence check.** `SHA256SUMS` existing proves nothing;
`SHA256SUMS` having exactly as many lines as there are artifacts is the control (D-15-4-B;
denominator-asserted-first, the house rule 15-6 D-15-6-D ratified).

## 3. 🔴 AC4 IS UNIMPLEMENTABLE AS WRITTEN, AND ITS GUARDRAIL GOES GREEN IN THE SAME RUN THE RELEASE DIES.

<!-- original lines 145-234 -->

AC4 asks the `MAOS_RELEASE_PUBKEY` guardrail to *"fail loudly in release context when unset."*
Three measurements dismantle that sentence.

**(a) There is no "release context" a unit test can see.** The guardrail is
`production_pubkey_must_differ_from_dev_seed` (`crates/maos-audit/src/release_verify.rs:280`,
`option_env!` at `:287`, the *"trivially true"* admission at `:283-285`). It runs in **two** CI
places, not one:

| site | env set? | blocking? |
|---|---|---|
| `release.yml:80` — `cargo test -p maos-audit production_pubkey_must_differ_from_dev_seed` | yes (`:82`) | has **never executed** |
| `discipline.yml:3439` — `cargo test --workspace --locked --no-fail-fast` (job `workspace-test-suite`, `:3423`) | **no** | **Blocking** — `v1-0-ship-gate.needs:3491` **and** `aggregate.needs:3697` |

Both are `cargo test`, i.e. the `test` profile with `debug_assertions == true`, so
`cfg!(debug_assertions)` cannot separate them. `grep -rn "CARGO_PROFILE\|env!(\"PROFILE\"" crates/ xtask/src`
is **empty** — no profile-detection mechanism exists anywhere in the tree. And the only signal the
test itself has is `option_env!`, whose `None` **is** the unset condition: the assertion cannot
distinguish *"unset because dev"* from *"unset because someone forgot the secret"*. **Making it
fail when unset reds a Blocking gate on every push and PR, for every developer.** AC4 as written
is circular.

**(b) The hazard it names is the safe state. The dangerous state is `MAOS_RELEASE_PUBKEY` SET.**
Simulated `release.yml:70-87` exactly as written, with a synthetic production keypair:

```
### :70-77  SIGN      (env :75-77 — MAOS_RELEASE_PUBKEY + RELEASE_SIGNING_KEY)
release-verify: signed 1 entries → SHA256SUMS.sig                     exit 0
### :78-82  GUARDRAIL (env :81-82 — MAOS_RELEASE_PUBKEY)
test result: ok. 1 passed; 0 failed                                   exit 0   ← AC4's control is GREEN
### :83-86  VERIFY    (NO env — the step declares none)
release verification failed: Ed25519 signature verification failed    exit 1   ← the release DIES
```

`release.yml:83-86` has no `env:` block, so cargo recompiles `maos-audit` with
`option_env!` → `None` → the **dev** key, and checks a **production** signature against it.
⚠ **And `:40-44`, the `build` job, never sets `MAOS_RELEASE_PUBKEY` either** — it is a *different
job*, and GitHub does not propagate secrets into env implicitly. So every published `maos` and
`maosctl` binary would embed the **dev** key even with both secrets provisioned. Those binaries
are the verifiers (`crates/maos-bin/src/main.rs:1604`, `crates/maos-cli/src/subcommands.rs:1201`).

⚠ **CORRECTION to `provisioning-checklist.md:56-61` (15-5's own evidence block), measured.** It
says a cached rlib can retain a stale `option_env!` value because `maos-audit` has no `build.rs`.
That is **disproved**: rustc emits `# env-dep:MY_VAR=...` into dep-info and cargo keys the
fingerprint off it — verified on a throwaway crate *and* re-verified four times on `maos-audit`
itself; every env change recompiled. `cargo:rerun-if-env-changed` is redundant with `env-dep`,
and dep-info lives under `target/`, which `Swatinem/rust-cache@v2` restores. The cache is not the
bug. **The missing `env:` blocks are.**

**(c) With the secret ABSENT, the behaviour is not a vacuous pass — it is a compile failure.**
GitHub evaluates an undefined `${{ secrets.X }}` to the empty string, so the step sets
`MAOS_RELEASE_PUBKEY=""`. Measured:

```
$ MAOS_RELEASE_PUBKEY="" cargo check -p maos-audit --lib
error[E0080]: index out of bounds: the length is 0 but the index is 0
  --> crates/maos-audit/src/release_verify.rs:56:22
note: inside `parse_hex_32`  --> :37:28
$ MAOS_RELEASE_PUBKEY="deadbeef01" cargo check -p maos-audit --lib
error[E0080]: index out of bounds: the length is 10 but the index is 10
```

Fail-closed, which is right — with a message that names nothing. So
`provisioning-checklist.md:26`'s cell *"vacuous pass: compile-time guard is omitted when unset"*
is **wrong for the shape GitHub actually produces**, and this story repairs that row.

**(d) Two real defects in the mechanism, both measured.** `parse_hex_32` (`:32-53`) loops
`while i < 32` and **never checks `s.len()`**: a 128-hex-char paste (the common `seed‖pubkey`
export shape) is **silently truncated to the first 64 chars**. And `:40`/`:46` accept `b'A'..=b'F'`
while the doc at `:22` and the panic text at `:41`/`:47` both say *"64 **lowercase** hex chars"* —
the message lies, and for the length cases it is unreachable anyway because the index panic fires
first.

**(e) The spike that killed the obvious oracle.** A byte-grep for the dev pubkey in the built
artifact looked like a free, house-consistent control (the `check-mock-not-in-release` technique).
Measured, it is unreliable:

```
$ cargo build --release -p maos-bin -p maos-cli --locked
$ python3 -c "import binascii; pub=binascii.unhexlify('bedd2ba6…469e7');
  [print(f, open(f,'rb').read().count(pub)) for f in
   ('target/release/maos','target/release/maosctl')]"
target/release/maos     size=33195488  dev_pubkey_occurrences=0
target/release/maosctl  size=10041648  dev_pubkey_occurrences=1
```

`RELEASE_PUBKEY` is a `const`, inlined per use site; with a single use site the array can be
materialised by immediate stores rather than living in `.rodata`. **Ruled out** (D-15-4-E).

**What replaces it** is in D-15-4-E: the shipped `maosctl` verifies the shipped `SHA256SUMS`.
Zero new surface, executed on the real artifact, and falsifiable by signing with a different key.

## 4. 🔴 AC1's FUNDING CLAIM IS FALSE. `xtask` HAS 35 LINES AND THIS STORY IS NOT IN ITS LEDGER.

<!-- original lines 240-249,254-275 -->

`epic-15…:179` says the new verb is *"(NEW xtask verb; charged to 15-2's xtask row)"*. Measured at
HEAD with the gate's own instrument:

```
$ cargo run -q -p xtask -- kloc-check --json | python3 -c "...per_crate..."
xtask  43409        ceiling 43444 (kloc.toml:252)        headroom +35
$ grep -n "15-4" xtask/kloc.toml
(no output)
```



The row's own ledger (`:219-222`) is explicit and does not list this story:

> Remaining ledger: 15-1 AC3/AC6; 19 +150; 15-2 AC6; 21-2 +150–300; 21-4 +150–350; 20-3 retires
> 4476–5176. **⚠ This row is OVER-SUBSCRIBED and now has ZERO headroom: every remaining ask above
> needs its own named measured grant.**

⚠ **And two rules on the same row read against each other.** 15-1's grant closes with
(`:248-250`): *"R-11: THE NUMBER IS A FORMULA — ceiling = tokei at THIS landing commit + 35 … **One
grant, one commit — never a second grant on this row.**"* Read literally that forbids what
`:219-222` mandates. **15-4 is the first story to reach the seam.** D-15-4-A rules it: R-11 binds
**15-1's own** number (it forbids 15-1 returning for more after its formula was fixed); `:219-222`
governs **subsequent named stories**, of which this is one. The ruling is put to the operator in
Open Questions because it is a ceiling amendment, not an engineering choice.

**Sizing, from the only comparable landing.** 15-3 added one verb and its wiring measured **+11**
in `main.rs` (`git show 2fd492c0 -- xtask/src/main.rs`: mod decl +1, enum variant +9, dispatch arm
+1). The module itself sits between `xtask/src/release_verify.rs` (182 lines) and
`xtask/src/check_mock_not_in_release.rs` (142 lines) in complexity, plus a three-target build
driver and an explicit artifact table: **+150…250**. Total **+171…276**, ×1.3 = **+222…359**.
⚠ **15-3 projected +95…235 for one verb and measured +1143 — 2.2× its upper bound.** The grant is
asked in **T9, after `cargo fmt --all`, on the measured number**, never on this estimate.

## 5. 🔴 AC5's GREP IS ON THE WRONG AXIS. 34 OF ITS 40 HITS MUST NOT BE EDITED, AND THE FILES THAT BREAK ARE PINNED TO `0.5.0`.

<!-- original lines 281-422 -->

`grep -rn "0\.1\.0-alpha" . --exclude-dir=target --exclude-dir=.git | wc -l` → **161 occurrences
across 67 files** (excluding this story file, which contains the literal itself), not *"25+ files"*. Of the 40 outside `Cargo.lock` and `_bmad-output/`,
**34 are `min_substrate_version`**.

**`min_substrate_version` is a floor a Spirit declares against the kernel, not the kernel's own
version.** One command disproves the coupling:

```
$ grep -n "min_substrate_version" templates/spirit-ts/manifest.toml examples/example-spirit-ts/manifest.toml
templates/spirit-ts/manifest.toml:6:min_substrate_version = "0.5.0"
examples/example-spirit-ts/manifest.toml:6:min_substrate_version = "0.5.0"
```

Two manifests in this repository already declare `0.5.0` while the workspace sits at
`0.1.0-alpha`. **AC5's framing — *"states which bind to the workspace version"* — has an answer,
and the answer is: none of the 34 do.** Editing them is a no-op at best and a self-inflicted red
at worst (below). The epic's *count* is exactly right (12 Spirit manifests + the template = 13
files); its *implication* is wrong.

**What actually binds — the complete list:**

| class | sites |
|---|---|
| (a) source of truth | `Cargo.toml:64` — 44 crates inherit via `version.workspace = true`; those are exactly the 44 `Cargo.lock` entries |
| (b) independent literal, must move | `STABILITY.md:23` (generated) · `docs-site/docs/migrate/abi-stability.md:21` · `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/migrate/abi-stability.md:22` |
| (b') lockfiles | **four**, not one — §5.2 |
| (c) test assertion that breaks | **ZERO** — every version assertion is `env!("CARGO_PKG_VERSION")`-relative (`crates/maos-bin/tests/verb_table_15_3.rs:353`, `crates/maos-kernel-core/tests/abi_stability_admit.rs:129`), so 15-3's brand-new `maos --version` oracle follows the bump for free |
| (b-neg) **must NOT move** | the 34 `min_substrate_version` occurrences |
| (d) comment | `xtask/src/stability_matrix.rs:18` · `crates/maos-domain/src/revocation.rs:510` · `crates/maos-bin/src/main.rs:8159` |

⚠ **The 12 Spirit crates do not inherit the workspace version either** —
`spirits/{architect,butler,digest,hello-spirit,observer,orchestrator,researcher,reviewer,worker}/Cargo.toml`
are `version = "0.1.0"` and `spirits/{mira,nash}` are `"1.5.0"`. None move.

**5.1 — Two traps inside the 34, for a story that "greps and bumps".**

- `templates/spirit-rust/manifest.toml` and `examples/example-spirit/manifest.toml` are
  **byte-coupled**: `xtask/src/templates_regen.rs:54-66` declares the first the source and the
  second the generated target, and `discipline.yml:1008` runs `templates-regen --check --json`.
  AC5 names **one half of the pair**. Editing it alone reds a Blocking job.
- `crates/maos-manifest/tests/manifest_n_minus_1_compat.rs:31` is a *fixture string*; `:104`
  asserts it back. It proves TOML round-trip, not a version contract. Leave both or change both;
  changing one reds `cargo test -p maos-manifest --test manifest_n_minus_1_compat`
  (`discipline.yml:1837`). The epic's *"×2"* is this self-consistent pair, not two breakages.
- `spirits/worker/tests/cli_wrapper_admission.rs:248` is **inert** — it lives in
  `both_class_and_cli_wrapper_manifest_is_rejected` (`:239-263`), which is refused on the
  section-conflict leg (`EManifestSchemaConflict`) before any version comparison happens.

**5.2 — `Cargo.lock` is FOUR files, and `--locked` breaks first.** AC5's *"`Cargo.lock` stays
committed"* was already flagged vacuous by the R2 preflight (`epic-15-21-preflight-r2…:44`) — it
is tracked at HEAD. The non-vacuous half is the sequence, measured in a synthetic workspace:

```
EXIT after bump, --locked, no lock regen = 101
error: cannot update the lock file … because --locked was passed to prevent this
EXIT after a plain (unlocked) build that regenerates the lock = 0
```

**Required order: edit `Cargo.toml:64` → run an unlocked resolve (`cargo metadata --offline`) →
commit the 44-line `Cargo.lock` diff in the same commit.** And `git ls-files '*Cargo.lock'` returns
**12** tracked lockfiles, **four** of which carry the literal:

```
Cargo.lock                                                       44 entries
crates/maos-domain/fuzz/Cargo.lock                                2
crates/maos-manifest/fuzz/Cargo.lock                              4
crates/maos-wasm-host/guests/equiv-fixture/native-twin/Cargo.lock 4
```

`cargo update -w` at the root does not touch the satellites — they are separate workspaces. Their
CI build paths do not pass `--locked` (`discipline.yml:2887`, `:2666,:2668`), so they self-heal in
CI and **the committed files rot with no gate to notice**. D-15-4-H regenerates all four.

**5.3 — `STABILITY.md`: the claim is TRUE and the blast radius is exactly one line.** Measured in
an isolated scratch copy (zero repo mutation), bumping only the copied `Cargo.toml`:

```
$ diff STABILITY.md $SCRATCH/STABILITY.md
23c23
< | `kernel_version` | `0.1.0-alpha` |
---
> | `kernel_version` | `0.1.0-alpha.1` |
```

The gate is byte equality (`stability_matrix.rs:53` `committed == rendered`) and is **green at
HEAD** (`{"passed":true,"in_sync":true}`), so the story's own bump is what reds it. Three jobs
consume it: `check-stability-matrix` (`discipline.yml:2311`), `v1-0-ship-gate` (`needs` entry
`:3447`), `aggregate` (`needs` entry `:3683`).
⚠ **CORRECTION to 15-5 AC3's cite:** the export fence is `STABILITY.md:90` … `:126`, not
`:91-124`. The bump at `:23` is far outside it; `extract_preserved_export`
(`stability_matrix.rs:287`) must round-trip the fence's **35** lines (`STABILITY.md:91-125`,
between the markers at `:90` and `:126`) byte-for-byte.
⚠ **CORRECTION to AC5's cite:** `stability_matrix.rs:17` is the provenance sentence; the literal
`0.1.0-alpha` is at `:18`.

**5.4 — Spirit admission survives the bump, by a one-character margin in someone else's code.**
`crates/maos-kernel-core/src/security/mod.rs:282-306` computes `env!("CARGO_PKG_VERSION")` and
evaluates `semver_range_contains(kernel_version, ">={declared_min}")`, failing **closed**
(`ESubstrateTooOld`) on both `false` and unparseable. The comparator is hand-rolled
(`crates/maos-domain/src/revocation.rs:617-660`) and splits on `.` **before** handling `-`, which
is not semver §11.4. Extracted verbatim and executed:

```
compare_versions("0.1.0-alpha.1", "0.1.0-alpha") = Greater   >= satisfied = true
compare_versions("0.1.0-alpha.1", "0.1.0")       = Less      >= satisfied = false
compare_versions("0.1.0-alpha",   "0.1.0")       = Less      >= satisfied = false
```

All 13 Spirits still admit. The `>=0.1.0` declarations (`crates/maos-bin/src/main.rs:6912,:6950`)
already fail today and are unchanged — no regression. ⚠ **Do not dress this up as a near miss.**
The missing fourth segment is padded with `"0"` (`revocation.rs:622-623`), but an empty pad would
also hold: `"".parse::<u64>()` is `Err`, control falls to the `_` arm at `:644`, and
`"1".cmp("")` is `Greater` either way. The real deviation from semver §11.4 — which the function's
own doc at `:614-616` claims to follow — is that pre-release **identifiers** are compared as whole
dot-separated segments rather than per-identifier, so orderings like `alpha.2` vs `alpha.10`
compare as strings. Neither case is reachable by this bump. **T5 runs the probe anyway, because a
fail-closed admission leg is not somewhere to reason.**

**5.5 — The files that actually break are pinned to `0.5.0`, so AC5's grep cannot find them.**

```
packaging/homebrew/maos.rb:15:  version "0.5.0"
packaging/aur/PKGBUILD:12:pkgver=0.5.0
packaging/rpm/maos.spec:9:Version:        0.5.0
```

`PKGBUILD:21-26` builds every download URL from `v${pkgver}` — pointing at a tag that will never
exist — and `:27-31` / `maos.rb:25` still carry `PLACEHOLDER_SHA256_*`. All four embed the **dev**
pubkey as a literal (`packaging/deb/rules:10`, `maos.rb:20`, `PKGBUILD:36`, `maos.spec:20`),
identical to `release_verify.rs:26-30`'s `DEFAULT`. Each file's own header says it *"must be
replaced with the production key before the first tagged release"* — a comment, not a control,
and nothing reads it.
⚠ **`0.1.0-alpha.1` is not a legal `pkgver`/`Version:` string** — AUR forbids a hyphen and RPM
reads it as the Version-Release separator. ⚠ **But "cannot be expressed" is FALSE, and the
round-table struck it (2026-09-11):** the coupling lives in these files, not in the packaging
formats. `PKGBUILD:21-26` builds every URL from `v${pkgver}` **because it was written that way** —
`_tag=v0.1.0-alpha.1` with `pkgver=0.1.0.alpha.1` and `$_tag` in the `source_*` arrays decouples
them in twelve characters; `%global tag` does the same for `maos.spec:11-12`. Nothing requires
`pkgver` to equal the tag. **D-15-4-J** therefore routes a *known-shape* edit, not an open problem,
to **`ops-brew-tap-and-aur-publication`** (`epic-20…:40`, `sprint-status.yaml:283`) — a real
tracker row, ⚠ **not the section number "20-2", which is not a sprint key at all** — and records
the boundary where a reader will hit it.

## 6. 🔴 AC3's MECHANISM IS RIGHT AND ALL THREE OF ITS PRECONDITIONS ARE ABSENT.

<!-- original lines 428-460 -->

The R2 preflight's recommendation stands: a job cannot `needs:` across workflows
(`grep -rn "workflow_run\|workflow_call" .github/workflows/` → **zero**; `release.yml:2-4` is
`push: tags`, `discipline.yml:3-7` is `push/pull_request` to `main`), so a check-run query is
correct. Three things the AC does not say make it not work:

1. **`permissions:` must gain `checks: read`.** `release.yml:59-60` declares `contents: write`
   **only**, and GitHub sets every unlisted permission to `none`.
   `GET /repos/{owner}/{repo}/commits/{ref}/check-runs` requires the Checks read scope. The
   in-repo precedent does declare its scope: `journal-aggregate.yml:43-45` carries
   `permissions: actions: read` for its own `gh api` call.
2. **The default page is 30 and `discipline.yml` has 161 jobs.** A naive
   `gh api …/check-runs | jq 'select(.name=="aggregate")'` returns nothing on page 1 and would
   refuse **every** release — a red that proves nothing. The query must use the documented
   server-side filter `?check_name=aggregate&status=completed&filter=latest`, or `--paginate`.
3. **`aggregate` is an ambiguous check-run name in this repository.**
   `grep -rn "^  aggregate:" .github/workflows/` returns **two**: `discipline.yml:3571` and
   `journal-aggregate.yml:48`. A manual dispatch of the latter at the tagged SHA would satisfy a
   name-only filter with a check-run that proves nothing. **D-15-4-G** renames the journal job —
   verified safe: nothing outside `journal-aggregate.yml` references the job name (only the
   workflow filename appears in docs). Renaming `discipline.yml`'s `aggregate` is refused: it is
   the branch-protection check name (`docs/ci-baselines/README.md:39`).

⚠ **CORRECTION to AC3's prose.** The check-run `name` is `aggregate`; `discipline / aggregate` is
the UI display label (`workflow / job`) and never appears in the API's `name` field. The
operative clause is right, the narrative clause is the wrong string, and the wrong string is what
gets copied into code.

⚠ **And the precondition nobody wrote down.** `discipline.yml` runs only on `push`/`pull_request`
**to `main`**. HEAD is `recovery-lane`, **6 commits ahead of `main`** (`git rev-list --count
main..HEAD` = 6; `main` = `03ee0ad8`), which is this project's normal working shape. **A tag cut
on a SHA that has not reached `main` has zero discipline check-runs, and AC3's guard refuses the
release forever with a failure that looks like a bug.** The guard's refusal message must name the
cause, and `tag-procedure.md` must carry the precondition (AC3, AC5).

## 7. 🔴 AC2's CLAIM IS FALSE IN KIND, AND macOS HAS ZERO PRECEDENT IN 177 LINUX JOBS.

<!-- original lines 466-508 -->

AC2 says the two aarch64 targets are *"never compiled in any workflow"*. They are **in the
matrix**: `release.yml:18-20` and `:21-23`, built at `:44` with the cross-linker wired at `:35-43`.
The true statement — different fix, different proof — is that **`release.yml` has never
executed**: it triggers only on `push: tags: ['v*']` and the only tag reachable from this clone is
`frozen-kernel-v2.0`. ⚠ `git tag -l` is local; confirm with `git ls-remote --tags origin` in T0
before the story restates "never executed" as a finding. `release.yml:44` is the only `--target` in the entire `.github/` tree and
`:22` the only `macos` string.

Three consequences for the new leg:

- **There is no `.cargo/` directory at all.** The aarch64-linux linker exists only as step env at
  `release.yml:41-43`, so `release-dry-run` must set
  `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER` / `CC_aarch64_unknown_linux_gnu` itself. There is
  no file to inherit it from. `rust-toolchain.toml` pins `channel = "stable"` and **no targets**;
  installation is per-job via `dtolnay/rust-toolchain@v1` `targets:` (`release.yml:30`).
- **The cross-compile is genuinely untested.** `Cargo.lock` carries `ring`, `libsqlite3-sys`
  (bundled) and `zstd-sys` — three C builds. `wasm-host` is off by default
  (`crates/maos-bin/Cargo.toml:15,:30`), so `wasmtime` is not in the release closure —
  ⚠ **and a `release-dry-run` that passed `--all-features` would silently cross the
  export-control boundary `RELEASE-HOLDS.md:29-35` holds open.** It must not.
- **`check-mock-not-in-release` on a cross-compiled aarch64 ELF is an unknown.**
  `release.yml:46` runs it on `target/<triple>/release/maos`;
  `xtask/src/check_mock_not_in_release.rs:89-102` shells host `nm --demangle` and returns
  `Err("nm exited with non-zero status")` on any failure — a hard gate failure, not a skip. Whether
  the x86 runner's `nm` reads an aarch64 ELF has never been executed. `gcc-aarch64-linux-gnu`
  (installed at `:39`) ships `aarch64-linux-gnu-nm`. T4 proves or selects.

**The macOS cost line, stated factually.** `runs-on:` across all 11 workflows: **177 ×
`ubuntu-latest`**, 1 × `windows-latest` (`discipline.yml:2811`), 1 × `ubuntu-24.04`, 1 ×
`${{ matrix.os }}` (`release.yml:24`, never executed). **No job in this repository has ever run on
macOS.** GitHub's published hosted-runner multipliers are Linux ×1, Windows ×2, **macOS ×10**, and
`aggregate` is the branch-protection check — so a naive three-target matrix in `aggregate.needs`
puts the slowest, most contended runner pool on the critical path of **every PR**. **D-15-4-F**
rules the shape.

⚠ **`check-mock-not-in-release` on `maosctl` would be a NULL CONTROL.** Measured:
`check-mock-not-in-release --binary target/release/maosctl --json` →
`{"passed": true, "forbidden-symbols-found": []}` — green, and green **from birth**:
`FORBIDDEN_PRODUCTION_SYMBOLS = ["MockHaltResolver","FailingHaltResolver"]`
(`check_mock_not_in_release.rs:29`) live only in `crates/maos-acp/src/server.rs`'s `#[cfg(test)]`
module, and `crates/maos-cli/Cargo.toml` has no `maos-acp` dependency. Running it on `maosctl` and
calling that coverage is the *claim standing in for a control* shape this epic exists to remove.
D-15-4-D states the scope boundary instead of buying a green.

## 8. 🔴 THE `done` RATCHET: FLIPPING THIS STORY'S OWN SPRINT ROW ARMS A BLOCKING GATE.

<!-- original lines 514-549 -->

`check-exit-commands` at HEAD — **PASS**, and it passes *because this story is `backlog`*:

```
check-exit-commands: PASS (7 epics, 7 blocks, 31 tokens resolved, 6 owed)
  owed: `release-dry-run`      (epic-15) → 15-4-release-repair-and-first-signed-tag [backlog]
  owed: `uninstall`            (epic-16) → 16-4-maos-uninstall-and-keyring          [backlog]
  owed: `eval`                 (epic-18) → 18-2-eval-halt-verb-and-corpus-numbers   [backlog]
  owed: `release-dry-run`      (epic-20) → 15-4-release-repair-and-first-signed-tag [backlog]
  owed: `maos-registry-server` (epic-20) → 20-1-registry-client-install-verb-…      [backlog]
  owed: `install`              (epic-20) → 20-1-registry-client-install-verb-…      [backlog]
```

`check_exit_commands.rs:807` — `if status == "done" { tried.push(format!("{full} is done")); }`.
**The instant `15-4-release-repair-and-first-signed-tag: done` is written into
`sprint-status.yaml`, both `release-dry-run` tokens stop being lawfully owed** and must resolve
against real `xtask --help` output at that commit. The gate resolves by **subprocess**, not by
grep: `check_exit_commands.rs:1553-1572` runs `std::env::current_exe() --help` under
`timeout -k 0.1 0.8` and parses the clap `Commands:` block (`:1412-1440`). There is no allowlist,
no exemption file, and no registry row — **adding the clap variant is necessary and sufficient.**
The gate is Blocking (`gate-registry.toml` `v1_5 = "blocking"`; job `discipline.yml:425`;
`v1-0-ship-gate.needs:3490`; `v1-0-ship-gate` itself in `aggregate.needs:3687`). The proven-red
vector already exists: `xtask/tests/check_exit_commands_gate.rs:242-265`
`resolve_or_be_named_reds_when_the_owning_story_is_done`.

⚠ **CORRECTION, and it shrinks this story's obligation.** `epic-15…:157` and 15-3's own T13 ledger
(`15-3-…:845-850`) assign `maos-registry-server` **and** `maos-spirit` to 15-4. The live gate says
otherwise: `maos-registry-server` is owed to **20-1**, `maos-spirit` **resolves** (it is a real
`[[bin]]`, `crates/maos-spirit-cli/Cargo.toml:11`, and `--help` exits 0), and a token the table
never listed — `install` (epic-20) — is owed to 20-1. **15-4 owes exactly two tokens, both
`release-dry-run`.** Epic-20 agrees: `:26` says `maos-registry-server` and `maos-spirit` *"are
added to that set by 20-1 AC1"*. Rule-9 edit in §11.

⚠ `epic-20…:26` also asserts `grep -rn release-dry-run xtask .github` is empty. It is not, since
15-3: `xtask/src/check_exit_commands.rs:749` carries the literal in a doc comment. Use
`cargo run -p xtask -- release-dry-run` (exit 2, `unrecognized subcommand 'release-dry-run'`,
`tip: a similar subcommand exists: 'release-verify'`) as the proven-red, not that grep.

## 9. 🔴 AC6 CANNOT BE PERFORMED, AND THE FIRST TAG FIRES A SECOND WORKFLOW NOBODY COUNTED.

<!-- original lines 555-582 -->

**AC6 is a hand-off AC and the story must not claim otherwise.** `provisioning-checklist.md:26-27`
records both release secrets as **`absent`**, and `ops-provisioning-secrets-and-accounts` is
`backlog` (`sprint-status.yaml:281`); `ops-first-signed-tag` (`:285`) is conditional on it by its
own text. The epic already places the tag outside the exit block under rule 2 (`epic-15…:184`).
15-4's deliverable is **everything that must be true before the operator can cut it**, plus the
acceptance definition — not the tag.

**The uncounted workflow.** `.github/workflows/container.yml:11-14` also triggers on
`push: tags: ['v*']`, has **no `if:` guard** (job `build-and-push`, `:21`), and logs into Docker
Hub at `:40-44` with `secrets.DOCKERHUB_USERNAME` / `DOCKERHUB_TOKEN` — which the checklist records
as `absent`. Its own header (`:6-7`) says *"At v0.5 this workflow is a SCAFFOLD. Container registry
credentials and cosign signing must be configured before enabling."* **The first signed tag
produces a red workflow run by construction.** The fix is already owned: `epic-20…:38` (`ops-provisioning-secrets-and-accounts`) assigns
`container.yml:40-44,:71-80` to `ops-provisioning-secrets-and-accounts` with *"(20-2 splits build
from push)"*. **D-15-4-K** therefore amends AC6's acceptance to name it rather than papering over
it, and `tag-procedure.md` records it as an expected condition.

**The document that says not to.** `RELEASE-HOLDS.md:67` is headed **"GA-tag procedure (do not tag
until both clear)"**, `:74` says *"Only then cut the GA tag"*, and both holds are ❌ Open. The
file's own Scope (`:3-7`) is explicit that the holds gate a **GA** tag and *"Neither blocks further
development, branch merges, or internal milestones"* — so `v0.1.0-alpha.1` is lawful. But the
ledger contains **zero** occurrences of `0.1.0`, `alpha`, `first tag` or `tag-procedure`
(grep empty), and `:17` records only *"**GA tag:** ⏳ Held."* **A story titled "first signed tag"
that cuts a tag while an unamended heading reads "do not tag until both clear" is precisely the
claim-standing-in-for-a-control shape this epic attacks.** D-15-4-L adds the pre-release carve-out
in this commit. ⚠ Nothing machine-reads that file (15-6's D-15-6-L re-verified at HEAD:
`grep -rn RELEASE-HOLDS xtask/src .github/workflows` is empty), so this is prose — which is why
the *runnable* half lives in `tag-procedure.md` and AC6's acceptance.

## 9b. 🔴 THE AGGREGATE IS RED AT HEAD, RIGHT NOW, AND NOBODY HAS FILED IT.

<!-- original lines 588-626 -->

Measured at `830c2685`, with this story's own file removed from the tree to prove it is not the
cause:

```
$ cargo run -q -p xtask -- check-dev-record-completeness
dev-record-completeness: deferred-work.md:889: STALE owner `unresolved owner`
    — owner is not resolvable to a sprint-status key
check-dev-record-completeness: FAILED — 1 violations
```

`check-dev-record-completeness` is **Blocking**: job `discipline.yml:1854`, registry entry
`xtask/gate-registry.toml:40`, and `aggregate.needs:3666`. **So `aggregate` is red at HEAD.**

The cause is one line wide. `deferred-work.md:887-892` is a 15-6 deferral whose `Owner:` cue and
its story key are split across a line break:

```
889|  `check-env-contract` nor a runtime/docs surface renders that classification. **Owner:
890|  `16-4-maos-uninstall-and-keyring`** — add the `maos --help` / …
```

The gate parses **per line** (`check_dev_record_completeness.rs:235`,
`unresolvable_owner(index + 1, "unresolved owner")`), so line 889 carries a declarative owner cue
with no key on it. Reflowing so the `Owner:` cue and the key `16-4-maos-uninstall-and-keyring` sit on the SAME
line closes it. The owner itself is correct and resolvable — this is a formatting regression, not a
governance gap. ⚠ **And the message is repaired while the line is open** (round-table 2026-09-11):
`"unresolved owner"` tells an operator who reflowed a paragraph nothing about what they did. It
becomes *"owner cue and key must be on one line"*. The line-scoped parse is **kept** —
brittle-and-loud beats lenient-and-quiet, and a bullet-scoped matcher is how a control starts
passing on anything.

⚠ **Why it belongs to this story.** 15-4 is the **last** engineering story in Epic 15, so its
landing is what `check-epic-close-green` and `check-epic-close-coherence` evaluate. Epic 15's own
header claims green-at-HEAD and 15-1 closed a seven-red inventory to get there; a red that landed
**after** 15-1 and rides `aggregate` would close the epic over exactly the four-epic decay pattern
Epic 8's retrospective named and 8.16 built a meta-gate to prevent. **Filed, not overwritten**
(house rule R-2): the regression is 15-6's, dated `830c2685`; 15-4 repairs it because 15-4 is
standing there when the epic closes. It is a T0 item, not an AC — the story cannot start from a
red aggregate and honestly claim any of its own greens.

## 12. Validation round 2026-09-11 — 14 defects in the story itself, all applied

<!-- original lines 1299-1329 -->

A fresh-context validator re-measured **68** of this story's citations against the tree: **54 PASS,
14 FAIL.** Every failure is corrected above. They are recorded here because the shape of them is
the story's own thesis — *a figure that was never re-derived is not a figure* — and because a
reviewer should know which numbers were wrong once.

| # | what was wrong | corrected to |
|---|---|---|
| 1 | `EXPECTED_GATES` stated as **42** — a naive `"…"` regex captured a string inside the `//` comment at `check_ship_gate_completeness.rs:57` | **41**, which is exactly what `epic-15…:157` predicts. The cut line inverted: nothing is owed |
| 2 | `provisioning-checklist.md:27` cited as the `MAOS_RELEASE_PUBKEY` row in four places | **`:26`**. `:27` is `RELEASE_SIGNING_KEY` and is **correct** — T7 would have rewritten a true row and left the false one |
| 3 | `epic-20…:39` cited for two different owners in five places | **`:38`** (`ops-provisioning…`, the `container.yml` hand-off) and **`:40`** (`ops-brew-tap…`, the formula/PKGBUILD). `:39` is the pen-test row and supports neither |
| 4 | edit #11's OLD text pinned to `epic-15…:166` | **`:157`**. `:166-168` is a *different and accurate* re-measured sentence; a literal OLD→NEW at `:166` finds no match |
| 5 | AC4's probe ran on all three matrix legs | the `aarch64-unknown-linux-gnu` binary **cannot exec** on an x86 runner (no binfmt/qemu on the hosted image). Guarded `if: matrix.target != …` |
| 6 | `--manifest-only`'s denominator was one-sided and its target set undefined — 2 files in the discipline leg vs 6 in `release.yml` | explicit `--targets`, and the assert runs **both ways**; `release-verify` only requires manifest→disk, never disk→manifest (`release_verify.rs:123-137`, `:146`), so a 2-of-6 manifest verifies green |
| 7 | *"the names `platform_binary_name()` already resolves"* | the `maos-` prefix is a **literal**; `maosctl-*` is a new name with **no** production reader, and `install_from_local` hashes `maos-<suffix>` whichever binary invoked it |
| 8 | `check_loom_substrate_drift.rs:19-25` cited for the `services:` rule | that is **Leg 1** (env drift); Leg 2 is **`:28-34`**. Also: **nothing in the tree enforces `timeout-minutes`** — the one citation supported only half the AC's claim |
| 9 | *"had that `unwrap_or` been `""`, the bump would have refused every Spirit"* | **false** — an empty pad falls to the `_` arm at `revocation.rs:644` and `"1".cmp("")` is still `Greater`. The real §11.4 deviation is per-segment, not per-identifier, comparison |
| 10 | export fence *"12 lines"*, `extract_preserved_export` `:288-305`, `committed == rendered` `:52` | **35 lines** (`STABILITY.md:91-125`), **`:287`**, **`:53`** |
| 11 | the R-11 quote OQ-1 turns on cited at `kloc.toml:249-251` | **`:248-250`** — `:251` is the 15-3 grant, a different grant, inside the story's own blocking operator ask |
| 12 | *"27 / 28 `dev_seed()` sites across five files"*; `trial_attestation.rs:379` called a dev-seed site | **33 occurrences across six files, 25 `dev_seed(` call sites**; `:379` is a `RELEASE_PUBKEY` consumer and the seed fallback is `producer_dev_seed()` at `:67`/`:76`/`:86` |
| 13 | T4 said *"re-point at `aarch64-linux-gnu-nm`"* as if free | `Command::new("nm")` is hardcoded (`check_mock_not_in_release.rs:94`/`:110`) with no hook — the fix is a **charged** `xtask/src` edit and now has its own §10 row |
| 14 | T10 flipped the sprint row to **`review`** | **`done`** — `review` is not a status this repository uses (only `backlog`/`ready-for-dev`/`in-progress`/`done`), and §8's whole hazard keys on the literal `"done"`, so the task never exercised the trap it describes |

Fifteen enhancements were also applied: `aggregate` enrolment is **three** sites not one (`needs:`
+ the results-table `$GITHUB_OUTPUT` line + the JS report row); `pattern: 'maos*'`,
`fail_on_unmatched_files: true` and `fail-fast: false`; the two `docs-site` release-signing twins
carry the same stale flow as the runbook; `air_gap_platform_binary_name()`
(`crates/maos-bin/src/main.rs:1646`) is a **second** copy of the naming contract; two further
`provisioning-checklist.md` rows go stale on this commit; the `0.1.0-alpha` census is **161**, not
162, once the story file itself is excluded; `RELEASE-HOLDS.md:17` not `:18`;
`check_env_contract.rs:67` not `:69`; and the `abi-stability.md` `manifest_schema_version` row is
**two** rows below, not one.

## 13. Round-table 2026-09-11 — four rulings, one of them inverting a decision the story had already made

<!-- original lines 1335-1347 -->

The room re-opened the story's four operator questions against the tree. **All four closed**, and
one existing ruling was overturned.

| ruling | outcome |
|---|---|
| **OQ-1 · the eighth `xtask` re-base** | **Authorized** (*"inevitable to increase the upper limit of kloc"*). The room's fight was not whether, but *what stops the ninth*. `kloc.toml:305-311` has carried *"the growth rate is NOT ratified here … must not be treated as settled by this grant"* since the Epic-13 retrospective, and D11 stayed `OPEN` through three more stories **because the clause was a promise**. A `grants_remaining` field in `kloc_check.rs` was proposed and rejected as absurd — a **charged** `xtask/src` edit funded by the grant it polices. The resolution came from the exclusion list: **`xtask/tests/` is kloc-free**, so the ratchet costs nothing. `xtask/tests/d11_xtask_ceiling_ratchet.rs` carries it. ⚠ **Round two then re-aimed that ratchet — see below: it binds on CAUSE, not count.** |
| **OQ-2 · macOS** | **Overruled** (*"macos jobs are dead. linux first. macos ci can be commented out."*). D-15-4-F's `if: github.event_name != 'pull_request'` is superseded by a Linux-only matrix with the darwin leg commented out in place. ⚠ The room surfaced the price before it was paid: **darwin's first compile becomes the tag**, with the whole Homebrew route on it. Mitigated by a pre-tag `workflow_dispatch` line in `tag-procedure.md`, at zero cost. |
| **OQ-3 · packaging** | **The premise was wrong.** The story claimed `0.1.0-alpha.1` *"cannot be expressed"* in AUR/RPM. It can: `PKGBUILD:21-26` couples URL to `pkgver` **because it was written that way**, and `_tag=` / `%global tag` decouple them in twelve characters. The word is struck, the mechanism is named for the owner, and the defer is re-pointed from the section number *"20-2"* — **not a sprint key** — to `ops-brew-tap-and-aur-publication`. |
| **OQ-4 · the bundled dev seed** | **Consensus, approved.** The release currently signs itself with a key whose private half is committed (`release_verify.rs:254`) and whose public half is hardcoded in all four packaging recipes. The default stays; **`release.yml` refuses to publish a binary that embeds it.** This is what makes AC4's two arms reachable one env var apart instead of leaving the meaningful one gated on a secret that does not exist. |

Also folded in: §9b's gate message becomes actionable (*"owner cue and key must be on one line"*)
while the line is open, with the line-scoped parse deliberately **kept** — brittle-and-loud beats
lenient-and-quiet, and a bullet-scoped matcher is how a control starts passing on anything.

## Round two, same day — one ruling inverted, two controls found to be aimed wrong

<!-- original lines 1351-1355 -->

| # | finding | ruling |
|---|---|---|
| 1 | **The D11 ratchet was measuring the wrong axis.** Lunarpulse: *"feature needs to be implemented then kloc increase; after all features in then we lock."* A ratchet that refuses the **ninth** re-base refuses **feature work** — the exact inversion of the founder functionality-first directive, and the same *"advisory until the right time"* shape that left seven v2.0 gates un-binding (`project_gate_binding_decay`). | **Re-shaped: cause, not count.** (i) An `xtask` raise must cite a `sprint-status.yaml` key that is **not `done`** — a raise with no open feature behind it is drift and reds. (ii) The ceiling **freezes once every `epic-N` is `done`** — the lock is *derived from the tree*, so nobody has to remember to throw it. The ninth, tenth and nth all pass **while a feature is behind them**, which is the rule as stated. Four falsifiers replace the one. |
| 2 | **AC3's proven-red was a null control.** The guard is a jq filter in YAML; the story's vectors were Rust. **A test of a re-implementation tests the copy, and the copy always passes** — on a control that guards *publication*. | **One predicate, two callers.** It moves into `cargo run -p xtask -- check-release-precondition`, reading the `gh api` response on stdin or `--check-runs <path>`. `release.yml` pipes the live API into it; the vectors drive the **same binary** with fixture JSON. ⚠ And `conclusion` must be `success` **explicitly** — `skipped` is also `completed`, so a skipped `aggregate` would otherwise wave the release through. Fifth vector added for it. ⚠ Boundary stated, not patched: whoever can push a `v*` tag can edit `release.yml` in the same commit — this stops an accident, not a determined actor. |
| 3 | **The manifest denominator's disk→manifest leg would red two ratified downstream plans.** `epic-20…:108` puts `.deb` (amd64 + arm64) and the air-gap variant **into this same `dist/`** via `20-2-deb-airgap-docker-and-formula-corrections`, and `epic-20…:16` fills it with `ln -sf` symlinks that `is_file()` follows through to a real file. *"Any regular file not in the table reds"* breaks both. | **Scope (b) to the DECLARED artifact set, and skip symlinks (`symlink_metadata`).** The defect being caught is a **missing** artifact, never a surplus one — the manifest is generated *from* the table, so a file outside the table was never going to be signed anyway. §2's `maosctl` omission is exactly the missing case. A third vector is planted for forward-compatibility: a `.deb` and a symlink beside the binaries must go **green**; if that one reds, the scope is wrong. |
