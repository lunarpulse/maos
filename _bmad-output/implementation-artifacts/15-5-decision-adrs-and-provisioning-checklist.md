---
baseline_commit: "**`9f46a2df`** (\"15-1-green-at-head\"), RE-BASED 2026-09-08 after 15-1 landed. The pairs collapsed: **every number in this story is now single-valued and re-measured at this commit** (R9 discharged - it was authored against `2fd492c0` plus 15-1 44 uncommitted files, so every figure had to carry a HEAD value and a tree value). Working tree clean apart from this story file, `sprint-status.yaml` and the party memlog. RE-MEASURED HERE, not inherited: `docs/adr/ADR-*.md` = **44 files**, highest **ADR-066**, `index.md` **40 rows** so the bijection gap is **ADR-027/042/043/044** and the malformed ADR-055 row is still `index.md:40`; the five new rows still insert **before `index.md:45`** (the ADR-065 row). `kloc-check: PASSED (aggregate=157231)`; ceilings `xtask` **43444** (`kloc.toml:252`), `maos-kernel-core` **18935** (`:199`), `maos-domain` **9192** (`:327`), `maos-a2a-core` **4986** (`:372`). All five Epic-15 exit-line-1 gates exit 0. Citation spot-check after 15-1 touched eight files: `security/mod.rs:266`, `control_block.rs:197`, `manifest.rs:85`, `admission.rs:179`, `env_contract.rs:255/260/265/270/470/475`, `maos-control/src/lib.rs:607/786/927`, `credential_posture_2c.rs:299` - **all still exact**. `cargo deny check advisories` still FAILS on `wasmtime 46.0.2` (unchanged; owned by AC9). **T0 still re-measures - this block is a snapshot, not a licence.**"
depends_on: "**15-1 - PRECONDITION SATISFIED 2026-09-08, VERIFIED NOT ASSUMED.** R1 required that this story not start until 15-1 was committed, because AC1 only control is a `cargo test` target whose CI runner lived in 15-1 unstaged tree. Verified at `9f46a2df`: the **`workspace-test-suite` job exists at HEAD** (`.github/workflows/discipline.yml:3423`; `cargo test --workspace --locked --no-fail-fast` at `:3439`; enrolled in **two** `needs` lists, `:3491` and `:3697`), its `worker-cli-fixture` prebuild resolves (`spirits/worker` is a tracked workspace member carrying that `[[bin]]`, and `cargo build --locked -p worker --bin worker-cli-fixture` exits 0), `docs/adr/ADR-066-*.md` is committed with its `index.md` row, and the two previously-red `t_14_2a_post_grace_*` binaries pass. **T0 still asserts the job exists** - the ruling was *asserted, never assumed*, and that survives its own precondition being met. The `index.md` merge-conflict hazard is discharged: ADR-066 row is committed, so the five new rows insert cleanly before `:45`."
blocks: "**Four epics, by name, and the epic file says so: `15-5 before Epics 16–17 open`.** `dependency-dag.md:213` carries the edges `15-5 → 16-1 (ADR-062), 17-1 (ADR-061), 17-3b (ADR-060), 21-3 (ADR-063)`. ⚠ **That row is missing ADR-064 and missing 15-6 entirely** — see §9. Named consumers verified in the tree: `epic-16…:5,30,66,79,108` · `epic-17…:9,41,51,53,79,81,86,100` · `epic-18…:5,20,22,74` · `epic-19…:15,23,81,89` · `epic-20…:34` · `epic-21…:5,72,73,93`."
spec_alignment: "⚠ **THE EPIC IS AMENDED IN THE SAME COMMIT — and so are four other planning artifacts and one architecture shard.** Operator directive in force (`feedback_pay_effort_early_deferring_drifts`, rule 9): *no drift in spec; the story and the epic say the same thing, or the epic is amended in the same commit; and never let a story be merely BETTER than its epic.* Six adversarial scouts disproved premises in this story's own section. OLD→NEW list in §12. The largest: **AC1's numbering premise is false and its frontmatter template is 2-of-44** (§1); **the story has no mechanical control at all** (§2); **AC3 is unimplementable as written because `STABILITY.md` is a generated file** (§3); **three of ADR-060's four ratified premises are unbuildable at HEAD** (§4); **ADR-061 as worded directs 17-1 to break a green test by name** (§5); **ADR-063's central deliverable is already ruled against in `RELEASE-HOLDS.md`** (§6); and **five artifacts disagree on whether this story mints four ADRs or five** (§9)."
split_from: "Not a split. Authored from `epics/epic-15-foundations-w0.md:182-190` (the `### 15-5-…` section) under Round 3. ⚠ **SIZING RE-DERIVED, NOT INHERITED:** the R2 preflight priced it 3–5 d → **4–6.5 d** at +30% (`preflight-r2/epic-15.md:183`) against three ADR analogs. That pricing assumed **four** ADRs of recording-a-ratified-decision. Measurement says otherwise: **five** ADRs, of which **ADR-060, 061 and 063 are design ADRs that must CREATE controls, reconcile four contradictory prior dispositions, and overturn a standing `RELEASE-HOLDS` ruling** — not record settled facts. Add the oracle (§2), the index-hygiene the oracle forces (§1), a `Cargo.lock` security bump (§10) and five artifacts' worth of rule-9 amendments (§12). **Re-measured: 5–8 d.** ⚠ **SPLIT PROPOSED AND REFUSED at the 2026-09-08 round-table:** Dana proposed 15-5a = {ADR-062, ADR-064} (unblocks Epic 16 and 15-6) + 15-5b = {060, 061, 063} (unblocks 17 and 21). Blocked on **sequencing, not cost** — all three hard ADRs (the ADR-002/031/040 supersession chain and the overturn of `RELEASE-HOLDS` (c.6)) sit in the back half, which would leave the reconciliation in a story with only Epic 21 behind it. **Dissent recorded (Dana): outvoted on sequencing, not on cost.** Delivered WHOLE — the ADRs are the deliverable that unblocks two epics, and a story that ships three of five leaves Epics 16 and 17 closed."
kernel_grant: "**NONE and none needed. ZERO kernel-Δ @24474, by construction.** Nothing here touches `crates/maos-kernel-core/src`. `kloc_check` measures `--types Rust` only (`xtask/src/kloc_check.rs:294-317`) so five `.md` files cost nothing on any row. Files written: `docs/adr/ADR-06{0,1,2,3,4}-*.md`, `docs/adr/index.md`, `docs/runbooks/provisioning-checklist.md`, `STABILITY.md` (inside the preserved fence only), `README.md`, `Cargo.lock`, `xtask/tests/decision_adrs_and_provisioning.rs` (**uncharged** — `kloc.toml:2` excludes `tests/`), and the planning artifacts of §12. ⚠ **The one way this story could take a kernel-adjacent Δ is by writing the WASM statement outside the `STABILITY.md` preserved fence** — that path costs charged `xtask` lines against **35** of headroom. Ruled out in F16."
kloc_grant: "**NONE ASKED, NONE NEEDED — and the margin is the reason the oracle lives where it does.** `xtask` after 15-1 = **43409/43444, 35 free** (`kloc.toml:252`). A new `xtask/src/check_*.rs` gate would cost 150–400 charged lines against those 35 and require a fresh measured grant; the same control in `xtask/tests/` costs **zero** (15-3 proved the exclusion: `kloc_check`'s `-e tests` is a depth-agnostic glob, 546 Rust reports, 0 under any `tests/` dir). Precedent for a doc-governance oracle in that directory: `xtask/tests/d19_story_file_governance.rs` (405 uncharged lines, planted-red vectors). ⚠ **A GRANT IS A GLOBAL, NOT A RESERVATION** (`feedback_grant_is_a_global`) — T0 re-measures `xtask` after 15-1 commits and does not trust this figure."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs:45-48`) is the machine-checked set; `equiv` is prose, NOT a match token. Keep the literal `allowlist {` (`check_dev_model_used_populated.rs:302`)."
review: "§A6 full-layer net (Blind + Edge Case + Acceptance + Test-Infra + non-author runtime), **non-degradable (◆)**. ⚠ **Acceptance is load-bearing and this story is why.** Five of nine ACs produce Markdown, the artifact class this repository has repeatedly shipped as a claim standing in for a control (Epic-12 retro, 11 instances). **The reviewer must: (a) run `cargo test -p xtask --test decision_adrs_and_provisioning` and confirm **every** planted-red vector actually reds — clauses (a)-(f) AND the three denominators, including the empty-workflow-glob vector; a green oracle over five real ADRs proves nothing about whether it could ever fail; (b) confirm the `workspace-test-suite` job exists at HEAD, since without it the oracle runs nowhere (R1); (c) re-read each ADR against the disproof tables in SS4-SS8 and confirm no ADR restates a premise this story measured false; (d) verify `cargo run -p xtask -- stability-matrix --check` is still green after the fence edit; (e) confirm `cargo deny check advisories` passes after the `Cargo.lock` bump.** ⚠ **Round-table 2026-09-08 (SS13) found five defects in the oracle itself, every one of them this story's own SS2 thesis** — a promise, a receipt, a shape check, two absent-pass clauses and a borrowed baseline. A reviewer who reads the ADRs for prose quality, or who runs the oracle without planting its reds, has reviewed nothing."
---

# 15-5 — Decision ADRs 060–064 and the provisioning checklist

Status: **done.**

> **The capability:** *Every open design fork in the recovery lane has exactly one written,
> ratified answer that a machine can prove is present and non-empty — and the operator can see,
> item by item, which external things engineering cannot supply are provisioned, absent, or
> deliberately waived.*

**Closes:** confidence **rule 7** (one decision per fork) for the whole lane · forks **D-A**
(operator surface), **D-B** (Worker egress + credential), **D-C** (WASM recall transport), **D-D**
(one replay selector), **D-E** (digest render path) and the revocation model · the preflight's
**§2.6** operator-provisioning gap · the two `wasmtime` RustSec advisories 15-1 filed here.

---

## What this story actually is

Rule 7 says *"one decision per fork."* Six forks were ratified by the operator in the Round-2/3
correct-course and then written down **only in planning prose**. Measured: **no machine in this
repository reads `D-A`, `D-B`, `D-C`, `D-D` or `D-E`** — `grep -rn "D-A\|D-B\|D-C\|D-D\|D-E"
xtask/src/` returns four hits and every one is an unrelated substring. The recovery lane's design
decisions currently live in files whose only reader is a human, and four epics are ordered behind
them.

So this story is not "write five ADRs." It is **the story that converts six operator rulings into
the corpus that Epics 16, 17, 19 and 21 are allowed to build against** — and, because it is the
fork-closing story, it carries more forks of its own than any other story in the lane. That is
structural, not scope creep.

Three things make the job harder than the epic's four ACs suggest, and all three were measured:

1. **The decisions as written are not all true.** Three of ADR-060's four "ratified" premises name
   mechanisms that do not exist or that a shipped validator refuses (§4). ADR-061's central verb is
   forbidden by name in a green test (§5). ADR-063's central deliverable is ruled against in
   `RELEASE-HOLDS.md` (§6). An ADR that records these verbatim ships a contradiction that **no gate
   can catch**, because nothing in this repository parses `docs/adr/` except two hardcoded ADR-040
   readers.
2. **The story has no control.** AC1's oracle is `ls`, which passes on five empty files (§2).
3. **AC3 cannot be executed as written.** `STABILITY.md` is generated byte-for-byte and guarded by
   an equality check; "gains a statement" reds three CI jobs (§3).

---

## 1. 🔴 AC1's NUMBERING PREMISE IS FALSE, ITS FRONTMATTER TEMPLATE IS 2-OF-44, AND THE INDEX IS ALREADY BROKEN

AC1 says *"numbers free: highest is `ADR-059`"*. Measured:

| | at HEAD `2fd492c0` | in the working tree |
|---|---|---|
| `ls docs/adr/ADR-*.md \| wc -l` | **44** | 45 |
| highest number | **ADR-065** | **ADR-066** |

`ADR-065` landed **in HEAD itself** (`git log --diff-filter=A docs/adr/ADR-065*` → `2fd492c0
15-3-…`); `ADR-066` is staged by 15-1. The premise was true when the R2 preflight measured it on
2026-09-05 (`preflight-r2/epic-15.md:86`) and both sibling stories invalidated it since.

**060–064 do survive — as a deliberately reserved GAP, not a tail.** `ADR-065:93-95`, verbatim:

> 4. **Numbering.** Story 15-5 mints ADR-060..064 and lands *after* this story, so the next
>    free number is **065**. Taking it here rather than waiting for 15-5 preserves the epic's
>    own dependency ordering (15-2 → 15-3 → 15-1 → {15-6, 15-5} → 15-4).

**Consequence AC1 does not state:** the five index rows must be **inserted before the ADR-065 row**
(`docs/adr/index.md:45`), not appended. The table is sorted ascending.

**The frontmatter template AC1 names is not the convention.** Key frequency across the 36 ADRs that
have YAML frontmatter (8 more use a `## Status` heading instead — e.g. `ADR-045:1-3`):

| key | count | key | count |
|---|---|---|---|
| `Status` / `Gate` / `Decided` | 36 | `Supersedes` | 11 |
| `Accepted-in-PR` | 34 | `Reuses` | **6** |
| `Revisits` | **29** | `Amends` | **3** |

The exact set AC1 names — `Status/Gate/Decided/Accepted-in-PR/Amends/Reuses` — is **ADR-058 and
ADR-059 only, 2 of 44.** The citation `ADR-059:1-8` is exact; the *norm* claim is not. **This
epic's own two ADRs both use `Revisits`/`Supersedes`** (`ADR-065:1-9`, `ADR-066:1-9`), which is also
the semantically right pair here: ADR-062 supersedes a rustdoc comment and revisits three tests;
there is no prior ADR for it to `Amend`.

**And the index is already wrong, in three ways this story's own oracle will red on:**

- **40 rows, 44 files.** Missing: **ADR-027, ADR-042, ADR-043, ADR-044**. (ADR-043 is the
  retirement-pattern precedent `ADR-065` leans on — absent from the index that is supposed to list
  it.)
- **`docs/adr/index.md:40` is a malformed table row** — the ADR-055 row has no trailing `|`.
- The trailing prose (`index.md:52-57`) still lists `027–029` as "tracked elsewhere" though
  `ADR-027` has a file.

**No gate reads any of this.** The complete set of machine readers of `docs/adr/`: `check_adr_040_accepted.rs:23`
(one hardcoded path, `Status == "accepted"`, `:86`) and `check_cross_form_equiv.rs:69` (same
hardcoded path, value reported not branched on). `invariant_lock.rs:106-140` maps `I1..I14 →
docs/invariants/*.md` only and **cannot fire on `docs/adr/**`**. `grep -rn "docs/adr" .github/workflows/`
→ empty. `docs-site/sidebars.ts` does not ingest ADRs, and `docs-site.yml:7-27`'s path filters omit
`docs/adr/**` entirely — the doc-site workflow will not even trigger. **Adding five ADRs and five
index rows can turn no currently-green gate red.** That is convenient and it is also the problem.

**Also disproved, and it is an epic-wide misattribution:** the epic cites *"the ADR-037 process"*
for raising kloc ceilings. `ADR-037` is 20 lines about invariant amendment (`:11`); KLOC ceilings are
**ADR-038**, and the I9 whitelist rule is `docs/invariants/I9.md:22-30` + `xtask/i9-whitelist.toml`.
Not this story's AC, but ADR-060..064 must not repeat it.

---

## 2. 🔴 THIS ◆ STORY HAS NO MECHANICAL CONTROL. `ls` PASSES ON FIVE EMPTY FILES.

AC1's oracle is:

```
ls docs/adr/ADR-060-*.md docs/adr/ADR-061-*.md docs/adr/ADR-062-*.md docs/adr/ADR-063-*.md docs/adr/ADR-064-*.md
```

`touch` five files and it exits 0. Measured, the story is unguarded end to end:

- **Not in the exit block.** Epic-15's hermetic exit (`epic-15…:24-28`) has three lines: 15-1/15-2's
  gates, 15-3's `check-exit-commands`, 15-4's `release-dry-run`. **No line names an ADR or the
  checklist.** The checklist appears only in the operator lane, explicitly excluded (`:36-37`).
- **Not in any CI job.** `grep -rn "docs/adr" .github/workflows/` → empty.
- **`check-exit-commands` structurally cannot help.** It resolves *verbs*, and it strips `VAR=value`
  prefixes without validating the name (`check_exit_commands.rs:421-431`, `:568-574`; only
  `MAOS_ONE_SHOT` is special-cased at `:569`) — so `MAOS_INFERENCE_MODE=replay` in five downstream
  exit blocks is silently discarded. Nothing reds if ADR-064's variable never ships.
- **`check-env-contract` fires on 15-6's commit, never on this one**, and it checks registration,
  never the ADR.
- **No machine reads `docs/runbooks/`.** The three hits in `xtask/src/check_fuzz_floor.rs:17`,
  `check_fuzz_targets.rs:11` and `discipline.yml:2652` are all prose comments.

This is precisely the failure shape the Epic-12 retrospective named — *"a claim standing in for a
control"* — on a story marked **◆ §A6 non-degradable**.

**The fix is free, and two facts make it free:**

1. `kloc_check` excludes `tests/` (`kloc.toml:2`; 15-3 measured the glob as depth-agnostic across
   546 Rust reports). A test in `xtask/tests/` costs **zero charged lines** against `xtask`'s 35.
2. **15-1 AC7 enrolls `cargo test --workspace --no-fail-fast` as a Blocking CI job in
   `aggregate.needs`** (`epic-15…:84`) — so an `xtask/tests/` target actually executes in CI once
   15-1 lands. Before 15-1, it would not have: `discipline.yml:1931` records that no job runs
   `cargo test -p xtask` unscoped.

Precedent for exactly this shape: `xtask/tests/d19_story_file_governance.rs` — 405 uncharged lines
of doc-governance assertions built around **planted-red vectors**, whose own header states the rule
this story must obey: *"Acceptance is this file, not the helper. A shared helper without a planted
red is a refactor."*

---

## 3. 🔴 AC3 IS UNIMPLEMENTABLE AS WRITTEN — `STABILITY.md` IS A GENERATED FILE UNDER A BYTE-EQUALITY GATE

AC3 says *"`STABILITY.md` gains the WASM-off-until-Hold-2 statement."* `STABILITY.md:1-4`:

> `<!-- GENERATED FILE — do not edit by hand. … Regenerate: cargo run -p xtask -- stability-matrix -->`

It is literal. `xtask/src/stability_matrix.rs::render()` (`:139-259`) emits the entire file as one
`format!` string; every heading and table row is a hard-coded literal. The gate asserts **byte
equality**: `stability_matrix.rs:52` `let in_sync = committed == rendered;` → `Err` at `:95-100`.

Measured green at HEAD:

```
$ cargo run -q -p xtask -- stability-matrix --check --json
{"passed":true,"in_sync":true,"deprecation_issues":[],"export_issue":null}
```

**A paragraph added anywhere outside the preserved fence reds three jobs:**

| job | mechanism | `continue-on-error`? |
|---|---|---|
| `check-stability-matrix` (`discipline.yml:2311`) | byte inequality at `stability_matrix.rs:52` | **no** → `aggregate` RED |
| `smoke-abi-7-5a` (`discipline.yml:2308`) | greps `'"outcome":"in_sync"'` | no |
| `check-export-control` (`discipline.yml:2553`, **blocking v1_0 + v1_5**, `gate-registry.toml:219-221`) | re-reads the fence, `check_export_control.rs:136-152` | no |

And regenerating does not help: `render()` rewrites from the template and **destroys** the added
paragraph. The alternative — adding a section to `render()`'s template — costs ~8–15 charged `xtask`
lines against **35** of headroom, on the crate 15-1 just re-based.

**There is exactly one hand-authorable region, and the statement belongs in it anyway.**
`extract_preserved_export` (`stability_matrix.rs:288-305`) round-trips the content between
`<!-- PRESERVED:export -->` (`STABILITY.md:90`) and `<!-- END PRESERVED:export -->` (`:114`)
verbatim. That block is already the export-control classification, already ends with *"pending
export-compliance counsel review before v1.0 enterprise distribution"* (`:110-113`) — i.e. it
already names Hold 2 in substance. The WASM-off statement **is** an export-control claim
(5D002.c.1). Constraints: no nested fence marker (`:320-323`), no stub phrase (`:324-329`), non-empty
(`:317-319`).

**AC3's other cites are off, and it misses two live facts:**

- `RELEASE-HOLDS.md:29-33` → the block is **`:29-35`**; `:34-35` carry the two doc references AC3
  truncates.
- `RELEASE-HOLDS.md:71` → `:71` contains no "WASM"; the clause is **`:72-73`**.
- **Missed:** `RELEASE-HOLDS.md:25`, the Hold-2 row itself, ending *"Blocks GA tag for any
  externally-published binary that includes the WASM engine."*
- **Missed:** `docs/compliance/export-counsel-precondition.md:14,:46,:50,:53,:72-73` already
  documents the feature far more fully than either cited line. AC3's *"documented for
  self-builders"* is therefore **half already done** — what is genuinely missing is a *build* doc:
  `README.md:203-206` and `:376-381` list build commands and mention **no `--features` flag at all**.
- ✅ `crates/maos-bin/Cargo.toml:30` is **exact**: `wasm-host = ["dep:maos-wasm-host"]`, off by
  default, `default = ["network"]` at `:15`.
- **No machine gate reads `RELEASE-HOLDS.md`** (one prose hit, `tests/coverage-matrix.yaml:1462`).
  AC3 moves a statement from an ungated file into a triply-gated generated one — which is why the
  fence placement is not a convenience but the whole feasibility of the AC.

---

## 4. 🔴 THREE OF ADR-060's FOUR RATIFIED PREMISES ARE UNBUILDABLE AT HEAD, AND `rust-inproc` HAS FOUR INCOMPATIBLE DISPOSITIONS IN THE CORPUS

AC1 gives ADR-060 this content: *"`rust-inproc` first-party only, never admitted from a registry,
runs unsandboxed in the daemon by design; WASM = third-party/polyglot form; `cli_wrapper` = agent
CLIs."* Measured against the tree:

| premise | verdict | evidence |
|---|---|---|
| `cli_wrapper` is a form | **FALSE — structurally impossible** | `[cli_wrapper]` is **XOR** with `[class]` (`spirits/worker/manifest.toml:1-7`; `EManifestSchemaConflict`, `xtask/error-catalog.toml:136-138`) and `forms` lives *inside* `[class]`. A `cli_wrapper` Spirit **cannot declare a form.** |
| "WASM" is a form value | **FALSE at HEAD** | `crates/maos-manifest/src/manifest.rs:392-398` accepts exactly `{rust-inproc, subprocess}`; `forms = ["wasm"]` fails manifest parse. |
| "never admitted from a registry" | **describes a control that does not exist** | `crates/maos-registry/src/admission.rs:179` `admit_spirit` never reads `class.forms` (0 occurrences); `crates/maos-kernel-core/src/security/mod.rs:266` likewise. **Nothing anywhere admits or refuses a Spirit by form.** |
| "runs unsandboxed in the daemon by design" | **TRUE**, different mechanism | `scheduler_loop.rs:243 load<T: Spirit>` is a generic over a compiled-in type — there is no process to sandbox. Corroborating: `spawn_sandboxed` has **zero** `maos-bin` callers. |

**There are also two disjoint form taxonomies, and ADR-060 must say which it governs:**

- **Taxonomy A (manifest, string):** `manifest.rs:222` `ClassSection.forms: Vec<String>` at `:228`,
  validator `:385-398`. Corpus: **10 manifests declare `rust-inproc`, 3 declare `[cli_wrapper]` with
  no `[class]`, 0 declare `subprocess`, 0 declare `wasm`.**
- **Taxonomy B (host port, enum):** `crates/maos-host/src/lib.rs:48` `SpiritForm{NativeSubprocess,
  WasmComponent}`, surface-baselined at `check_host_surface.rs:231-234`. The string `rust-inproc`
  has **no** `SpiritForm` counterpart.

**The token matters mechanically.** `epic-17…:86` (17-3b AC4) already fixes it: *"`ClassSection.forms`
accepts **`wasm-component`** … test `class_section_rejects_unknown_form` `:2589-2593` inverted for
that value, **still refusing `wasm`**."* Measured, that test's negative vector is literally the
string `"wasm"` (`manifest.rs:2590`). **Naming `wasm-component` preserves the vector; naming `wasm`
forces 17-3b to delete it.** Epic-15's AC1 says only "WASM" and never names a token.

**And the corpus already contains four incompatible dispositions of `rust-inproc`:**

| source | says |
|---|---|
| `ADR-031:54,:63,:71` (`binding-v2.0`) | rust-inproc **"remains deferred"**; the §13.1 gate stays **untripped** |
| `ADR-040:3` (frontmatter) | `Superseded-by: ADR-031 (**defer lifted**; …)` |
| `13-phased-roadmap.md:45` | rust-inproc is **"retired as a roadmap item"** |
| PRD `project-scoping-phased-development.md:18` | rust-inproc **"is the first-party form"** |

ADR-060 is the fifth and the strongest reversal. **It must explicitly `Supersedes:`/`Revisits:`
ADR-031, ADR-040 and ADR-002 and amend `13-phased-roadmap.md:45`, or it lands as an unacknowledged
contradiction with a `binding-v2.0` ADR — and no gate would catch it.**

**One wording trap to disarm explicitly.** PRD `:18` and arch `:5` say *"no subprocess Spirit host is
built"*; `ADR-031:12,:39-41,:53` describes the WASM form as *"hosted as a subprocess"*. These do not
conflict — R2 means no *Spirit-Wire-Protocol subprocess Spirit host*; ADR-031 means the wasmtime
runner process. A reader who is not told this scores ADR-060 as contradicting ADR-031.

**D-C and D-E anchors, verified.** ✅ `crates/maos-wasm-host/src/runner.rs:1-5` — *"This binary IS
`BridgeSpawnSpec.program` … speaks ADR-032 over stdio"*, stdio at `:185-188`: the runner's stdio **is**
the kernel bridge. ✅ `wit/spirit.wit:219-230` — world `spirit`, one `use`, three exports, **zero
`import` directives**. ⚠ The unix socket **does not exist**: `grep -rn "UnixListener\|UnixStream\|UnixDatagram"
crates/` → **zero hits workspace-wide**, and there is no `recall` path in `crates/maos-wasm-host/src/`.
D-C must be written as *to build*, priced by 17-3a. ✅ `Butler::morning_digest` at
`spirits/butler/src/lib.rs:675`; `butler` is already a `maos-bin` dep (`Cargo.toml:82`); **4 callers,
0 production** (`maos-bench/src/harness/j0.rs:83`, `journey_butler.rs:153`,
`butler/tests/{hallucination.rs:152,digest.rs:122}`) — the preflight's unqualified "zero" is
falsifiable; write "zero production callers."

**FR5/FR33 are amended, not restated.** FR5 (`prd/functional-requirements.md:28`) is *operator-configured
strictest-of-three*; "FR5 holds by form" replaces its mechanism. FR33 (`:82`) names `cargo generate
maos-spirit` plus per-language templates **including Go v1.5+**, which the WASM-toolchain framing
silently drops. Both FR bodies are unamended at HEAD.

---

## 5. 🔴 ADR-061 AS WORDED DIRECTS 17-1 TO BREAK A GREEN TEST, BY NAME

AC1's ADR-061 content includes *"`env_clear`"*. Measured, `crates/maos-cli/tests/credential_posture_2c.rs:299-304`:

```rust
fn env_clear_stays_absent_from_the_production_worker_spawn_path() {
    assert!(
        !RUNTIME_SRC.contains("env_clear"),
        "adding env_clear() to spawn_and_bridge breaks the paid worker path and \
         breaches the kernel baseline pin — it is a regression, not a hardening"
    );
```

and the seam documents why at `crates/maos-bin/src/worker_cli.rs:122-133`: *"**This is NOT a
credential channel, and MAOS does not have one.** … The operator exports the key into `maos`'s own
environment and the child INHERITS it."* The production spawn is `runtime.rs:461-469`, a bare
`Command::new` with `cmd.env(k, v)` and no `env_clear`. **There are zero production `env_clear` call
sites in the workspace.**

**The ADR is not wrong about the destination — it is wrong about the order.** `env_clear` becomes
correct only *after* a proxy supplies the credential. Written flat, it instructs a future story to
red a green control as a standalone "hardening."

**Four more premises measured:**

- **The two clauses of D-B live in disjoint spawn paths.** `--network=none` is real
  (`t3/argv.rs:72`, asserted `:159-160`) but it is the **T3 container** path; the vendor key lives on
  the **unsandboxed `cli_wrapper`** path (`runtime.rs:449`), which never reaches `build_runtime_argv`.
  The only production `spawn_t3` caller is a **diagnostic smoke** (`main.rs:7059-7072`,
  `SandboxSpec::new_for_test`, `echo hello-from-t3`). Separately, `t3/spawn.rs:115` passes **no `-e`
  flags**, so a T3 container already receives no host env — the leak ADR-061 worries about is a
  `cli_wrapper` property, not a T3 one.
- **The manifest has no egress field.** `SandboxConfig` is `{tier, image_pin}` (`manifest.rs:85-91`).
  And `[network]` is **already priced elsewhere**: `kloc.toml:322-323` ledgers *"Epic 17 `[network]`,
  pids_max, wasm-component and validators +80–200"* against **`maos-manifest`**.
- **No proxy exists.** Every `proxy` hit is the FKCS evaluation cohort, `AttributionSource::WriteTargetProxy`,
  or prose.
- **"kernel-minted scoped credential" has no primitive.** `CapabilityToken` (`i1.rs:175-186`) is
  four fields — `token_id`, `spirit_pid`, `expiry_ns`, `signature` — **no scope, no audience, no
  secret material**; scope is registry-side (`ports/capability.rs:29`). No production `mint_*` exists.
- ✅ TL append entry point: `crates/maos-iac/src/adapter/transparency_log.rs:596`
  `insert_frame_event`, re-exported by `kernel-core/src/iac.rs:13`. ⚠ Two constraints the ADR must
  respect: it **panics** on write failure (§7.3 I2), and `maos-a2a-tcp` **must not gain a
  `maos-kernel-core` dep** (`kloc.toml:372` — the chaos-absence barrier greps that manifest).
- **17-1's `+65–130` kernel lines have zero headroom on their own row.** `maos-kernel-core` measures
  **18935/18935** (`kloc.toml:199`); the forecast is funded **only in the aggregate** (`kloc.toml:197`).
  ADR-061 is the ADR that authorizes 17-1's shape and should say so.

---

## 6. 🔴 ADR-063's CENTRAL DELIVERABLE IS ALREADY RULED AGAINST IN `RELEASE-HOLDS.md`, ITS DICHOTOMY IS FALSE, AND EPIC-21 GIVES IT A THIRD DECISION EPIC-15 NEVER MENTIONS

**(a) "chooses exactly one of reissue-is-revocation / CRL" is a false dichotomy — both ship, on
different axes.**

| axis | model at HEAD | evidence |
|---|---|---|
| Spirit classes | **signed CRL**, end to end | `crates/maos-domain/src/revocation.rs` (`SignedRevocationList:20`, `RevocationEntry:225`, `CrlId::from_entries:345`, `LocalFileRegistryClient:439`); `RevocationApplier` (`revocation/applier.rs:28`, `apply_crl:73`) + `RevocationPoller` wired at `main.rs:2876-2906`; `maosctl revocations import\|list` |
| Peer certificates | **reissue-is-revocation**, live | `crates/maos-cohort/src/rotation.rs:22-36` — *"The authority is the signed manifest … the same frame changes which certificates every node accepts on a live handshake"*; overlap window `tofu.rs:283-289` |

There is **no X.509 CRL/OCSP consumer**; the only OCSP mention is a *simulation* (`chaos/rotation.rs:23-24`).
And the CRL axis's semantics are already pinned by a test: `crates/maos-kernel-core/tests/on_revocation_three_actions.rs`
fixes three actions, a 10 000 ms drain deadline, one `spirit.quarantine_requested` frame, and ≥2
`EpistemicHalt` receipts per pid. **ADR-063 should ratify the two-axis status quo and confine any
"choice" to the certificate axis.**

**(b) The post-grace mechanism partly SHIPPED at 14-2a — "defines" over-claims.**
`git log --oneline -3 -- crates/maos-a2a-tcp/tests/` → `ee3422d0 14-2a-production-mtls-rotation-trigger`.

| | status | citation |
|---|---|---|
| refusal detected | **EXISTS** | `transport.rs:1032-1041` |
| journaled as durable `ConsentRupture` | **EXISTS** | `transport.rs:1048-1054` → `router.rs:502-560` |
| token `cert_post_grace_reject` observable | **EXISTS — tracing only** | `router.rs:556-561`; asserted `t_14_2a_post_grace_token.rs:184-192` |
| token machine-readable / durable | **ABSENT** | `router.rs:536` hardcodes `PeerIdentityUnverified`; the frame has no detail field |
| `RuptureReason::CertPostGraceReject` | **ABSENT** | `frame.rs:384-405`, six variants |

✅ All four cited anchors verified exact — `transport.rs:1032-1041` (classifier), `:1048-1054`
(call), `router.rs:502-507` with `detail: &str` on **`:506`**. ⚠ Two corrections: `#[non_exhaustive]`
is on **`frame.rs:383`**, not `:384`; and `journal_peer_identity_refusal` **takes `detail` but drops
it for durability** — it reaches only `tracing::warn!` (`router.rs:556-561`). "Already takes
`detail: &str`" is true and does **not** mean the token is durable.
⚠ Also stale: `epic-21…:72` says `t_14_2a_post_grace_journal.rs` asserts the variant — **15-1 moved
that leg out** into `t_14_2a_post_grace_token.rs`.

**(c) The standing objection.** `RELEASE-HOLDS.md:146-157`, verbatim:

> **(c.6) The §7.2.1.a `cert_post_grace_reject` LABEL is partial and this boundary remains OPEN.**
> … `journal_peer_identity_refusal` has no detail field, and the seam cannot distinguish "retired
> generation after the grace" from "unknown leaf": the store deliberately keeps no
> retired-generation history. **A new reason token would therefore assert a distinction the system
> cannot make.** The available audit join remains the rotation timeline — a
> `cert_rotation_window_closed` row for the peer, then the refusal.

This is the **direct opposite** of `epic-21` §H-9's ruling (*"a variant, not a `detail` field"*).
The premise is mechanically true: `TofuPin` (`tofu.rs:18-30`) has six fields and no previous/retired
generation. The seam's own sentinel says so out loud (`transport.rs:1034`): *"cert_post_grace_reject
candidate (or unknown-leaf refusal — this seam cannot distinguish them)"*.
⚠ The owner recorded for closing it is **`14-2b`, which is `done`** (`14-2a…:873`) — a dangling
owner ADR-063 inherits.

**Resolving it needs one fact the objection itself supplies.** §7.2.1.a requires
`cert_post_grace_reject` only as a **log** token — *"rejection logged as `cert_post_grace_reject`"*
(`7-inter-agent-communication.md:74`) — and that is already satisfied. The durable variant is
21-3's addition, so the ADR is free to define it by an observable the store **does** have: (c.6)
names it — *"a `cert_rotation_window_closed` row for the peer, then the refusal."* Ruled in F9.

**(d) The third decision epic-15 never mentions.** `epic-21…:73` (21-3 AC4): listen-side scoping
*"is DESIGNED in ADR-063 (identity claim in the client leaf's SAN = `PeerId`, or a post-handshake
binding frame — §H-10); **built here only if ADR-063 chooses SAN** (`maos-a2a-tcp +30–80`)."*
`preflight-r2/epic-21.md:213` concurs. Today `build_server_config` passes `None` = *"listen side
learns the peer from the cert — flat TOFU lookup"* (`transport.rs:1333`), and `RELEASE-HOLDS.md:59`
row 12 records *"the listen side accepts **any** active pin."* Ruled in F10.

**(e) Budget.** ✅ `maos-domain` measures **8695/9192, 497 free** (`kloc.toml:327`), and the ledger
at `:326` explicitly reserves *"21-3 +3–10"*. ⚠ `epic-21…:28`'s figure *"`:278` 8695/8644, −51"* is
**stale in both the line number and the ceiling** — D14's red was closed by 15-2.

---

## 7. 🔴 ADR-062 MUST PRESERVE WHAT IT SUPERSEDES — TWO OF THE THREE TESTS IT INVERTS ARE IDENTITY GUARDS 21-3 STILL NEEDS

✅ The READ-ONLY block is exactly `crates/maos-control/src/lib.rs:66-77` (AC1's `:68-76` is the ⚠
paragraph). It is a **rustdoc comment on the `RotationWindowSource` trait**, and **no ADR decides
the operator-surface trust path** — verified across all 44 ADRs. So ADR-062 has nothing to
`Amends:`; use `ADR-066:7`'s idiom (`Supersedes: nothing …` + a `Revisits:` naming the comment and
the tests).

**The refusal is enforced by three live tests, and they are not equivalent:**

| test | rationale |
|---|---|
| `lib.rs:607-611` | *"a mutating verb on this surface is the rejected **second trust path**"* |
| `lib.rs:786-790` | *"the write path for cohort manifest state is the **signed reissue and nothing else**"* |
| `lib.rs:927-931` | *"the surface is read-only"* — the only generic one |

`epic-16…:66` (16-1 AC2) says all three *"are inverted under ADR-062."* But `epic-21…:71` (21-3 AC2)
requires own-leaf rotation stay *"driven by the existing signed manifest reissue (14-2a), **not a new
message**."* **Inverting the first two deletes the guards 21-3 depends on and reintroduces the exact
second trust path the comment exists to prevent.** Ruled in F8.

**Five more corrections the ADR must carry:**

- **The comment's own cross-reference is dead.** It cites *"`main.rs:9844-9853` warns against"*;
  measured, `main.rs:9838-9860` is A2A intake-channel wiring plus a 14-2a rotation-plane note. **Do
  not propagate the pointer.**
- **`maos-cli` already has an HTTP client** — `subcommands.rs:1930-1975`, a hand-rolled loopback
  `TcpStream` GET with bearer, reading `MAOS_OPERATOR_HTTP_ENDPOINT`/`MAOS_OPERATOR_BEARER_TOKEN`
  (`:1938-1941`). The delta is **POST + a body parser + typed errors**, not "gets an HTTP client."
  (`epic-16…:66` already words this correctly; epic-15's AC1 does not.)
- **"may not link `maos-control`" is an existing enforced test**, not a new ruling:
  `crates/maos-cli/tests/dep_kernel_core_free_test.rs:8-29` shells `cargo tree` and asserts no
  `maos-kernel-core`. The edge it blocks is real: `crates/maos-control/Cargo.toml:12`.
- **`maos init` has TWO arms** — `main.rs:1699` (`VerbName::Init`) and `main.rs:1352`
  (`air_gap_init()`). AC1's `:1679` is inside the `Outcome::Help` arm. The implementation
  `maos_shell::run_init` (`maos-shell/src/lib.rs:46`) writes **`config.toml`** (`:48-49`, template
  `:313-329`); **`control.json` has zero source occurrences workspace-wide.**
- **The server has no 405 and never reads a body.** Four routes, all GET (`lib.rs:299,347,392,428`),
  catch-all 404 (`:434-440`), `respond`'s phrase table is `{200,401,404,503}` with `_ => "Internal
  Server Error"` (`:452-461`), and `handle_connection` stops at `\r\n\r\n` (`:274-283`). Binding is
  env-gated (`main.rs:2702-2730`), loopback-enforced (`:213-218`), constant-time bearer (`:288-292`),
  sync `std::thread` accept loop (`:224`) with no tokio dep.

⚠ **Filed, not fixed by this story:** `MAOS_OPERATOR_HTTP_ENDPOINT` is **read** at
`subcommands.rs:1938` and **not registered** in `env_contract.rs` (which has only
`MAOS_OPERATOR_BEARER_TOKEN:470` and `MAOS_OPERATOR_HTTP_BIND:475` from 15-1 AC5). It escapes
`check-env-contract` only because that gate scans `maos-bin/src` alone — and **21-4 AC2 widens the
scan to every crate**, at which point it becomes a red with no owner. ADR-062 names this variable as
a kept override, so the story files it to 21-4.

---

## 8. 🔴 ADR-064 IS FIVE-SIXTHS DUPLICATED INTO 15-6, AND NOTHING ORDERS THE ADR FIRST

**Duplication measured.** Epic-15's ADR-064 clause (`:186`) and 15-6's AC1/AC4 (`:195`, `:198`)
state the same six things — the env pair, the `--replay-llm` retirement with the *same*
`worker_spawn.rs:56-59` cite, "no `--replay` flag", "unset = today's deterministic path". **Five of
six clauses are verbatim in both.** Only the consumer list is 15-5's alone.

**Nothing orders them.** `dependency-dag.md:213` lists 15-5's edges as `16-1, 17-1, 17-3b, 21-3` —
**no `15-5 → 15-6` edge exists anywhere**, and `epic-15…:202` says only `{15-6, 15-5 in parallel}`.
And no gate can catch it (§2). So 15-6 can close D-D in code before the ADR exists, discharging rule
7 *after* the fork was closed.

**"Four spellings today" is wrong in both directions.** Measured:

| spelling | site | real? |
|---|---|---|
| `--replay-llm` | `worker_spawn.rs:59` | **accepted no-op** |
| `--live` | `worker_spawn.rs:54` | real (the inverse selector) |
| `MAOS_REPLAY_CASSETTE` | `env_contract.rs:255`; read `main.rs:4643`, `:4657` | real — **presence is the selector** |
| `MAOS_JOURNEY_MODE` | `env_contract.rs:260`; read `main.rs:4642` | real (`== "record"`) |
| `MAOS_REPLAY_STRICT` | `env_contract.rs:265`; read `main.rs:4658` | real — **a modifier, not a mode; epic-18's exit block uses it 3×, and neither story mentions it** |
| `--deterministic` | — | **HAS NEVER EXISTED** (0 hits) |
| `--replay` | — | **HAS NEVER EXISTED** (0 hits) |

**Every `main.rs` cite in the 15-6 section is +34 lines stale** — the same drift class 15-1 §9 filed.
True values: shared adapter **`:3170`** (not 3136), shell consumption **`:3281-3288`** (not
3247-3250), Researcher-private **`:4633`** (not 4599), record **`:4642`**, replay **`:4657`**,
unset-deterministic **`:4676-4680`**, `UnconfiguredProvider` site **`:3155-3159`**. ✅
`inference/mod.rs:38` is exact.

**Two structural facts neither story states:**

- **The adapters are asymmetric.** The Researcher branch is `if --live … else if
  MAOS_REPLAY_CASSETTE … else deterministic`. **The shared/shell adapter (`:3170`) has no cassette
  branch at all** — `MAOS_REPLAY_CASSETTE` is inert for `maos shell`. Yet epic-16 exit line 1 and
  epic-20 exit line 9 both replay `maos shell` from `cassettes/j0/shell-intro.json`. That is
  **net-new wiring**, not a rename.
- **`UnconfiguredProvider` is a fallback that succeeds.** `main.rs:3155-3159` installs it when no
  provider is configured, prints to stderr and returns `Err(Unconfigured)` **per call** — there is
  **no non-zero exit**, and its presence *masks* the one path that does exit non-zero
  (`main.rs:4624-4629`), because `default_id()` then returns `Some("anthropic")`. AC1's *"else a
  typed `Unconfigured` halt with non-zero exit"* is **false at HEAD**.

✅ `journey-nightly.yml:65`/`:87`/`:94` all exact, and the `--live` defect is worse than filed: with
no `--` separator it is consumed by `cargo nextest run` itself, and **no test in
`crates/maos-journey-test/tests/` parses `--live` as a test-binary argument** — so the paid leg has
**never** re-recorded anything.

---

## 9. 🔴 FIVE ARTIFACTS DISAGREE ON WHETHER THIS STORY MINTS FOUR ADRs OR FIVE

| artifact | says |
|---|---|
| `epic-15-foundations-w0.md:58` + AC1 (`:186`) | **five** — 060, 061, 062, 063, **064** |
| this story's key + title | **five** |
| `sprint-status.yaml:237` | **four** — "060 … 061 … 062 … 063 revocation" |
| `epics/index.md:153` | **four** — "15.5 ADR-060..063" |
| `epics/dependency-dag.md:213` | **four** — `16-1 (062), 17-1 (061), 17-3b (060), 21-3 (063)` |
| `architecture-…/13-phased-roadmap.md:5` | **four** — *"Four ADRs (060–063) are authored in 15-5"* |

**AC4 therefore fails on its own citation**: it asks that the architecture note be *"cross-checked
against the ADR text"*, and that note contradicts the story's scope. Amended in §12.

**AC4's cross-check surface is also incomplete.** It names PRD `:18` and arch `:5` but **omits
`13-phased-roadmap.md:100`**, the `[DELTA-2026-09-05 R3]` block that constrains **ADR-060, 061, 062
and 064** (*"one daemon process owns the loopback control surface … WASM recall = maos-bin
side-channel over a unix socket, kernel bridge (runner stdio) unchanged … one replay selector"*).

✅ Both cited deltas are exact (`prd/project-scoping-phased-development.md:18`,
`13-phased-roadmap.md:5`), but two PRD statements the delta does **not** amend still contradict it:
`:31` lists v0.1 key skill *"single-Spirit subprocess"* and `:43` still says the milestone loads
`hello-spirit` in **subprocess form** — the exact form `:18` says was never built. And AC4 mislabels
its own cite: `:5` sits under `# 13. Phased Roadmap`; **§13.1 begins at `:43`**.

⚠ **Citation hygiene for the ADRs:** the epic cites *"architecture §15.5 Decision clause 2(a)"* for
kloc lowering. Located at `15-full-spectrum-v2-2.md:100`, and it is **ADR-057's** clause, **amended
by Story 15-2 on 2026-09-06**, and its own last sentence reads *"These comments are governance
prose; no gate parses them."* Any ADR citing it must carry all three qualifiers.

---

## 10. 🔴 THE `wasmtime` ADVISORY FILED HERE IS A ONE-COMMAND FIX, AND THE EXCEPTION PATH WOULD FALSIFY `deny.toml`'s OWN HEADER

`sprint-status.yaml:237` files to this story: *"cargo deny advisories RED inside the
reproducible-build job (continue-on-error: true, cannot red the aggregate): wasmtime 46.0.2 carries
RUSTSEC-2026-0268 … and RUSTSEC-2026-0269 … — upgrade or documented-exception decision belongs to
the ADR lane."*

Measured, all four claims verified plus three the filing does not state:

- ✅ `Cargo.lock:5640-5643` → `wasmtime 46.0.2`. `cargo deny check advisories` **FAILS**, both
  `error[vulnerability]`.
- ✅ `discipline.yml:38` job-level `continue-on-error: true`; `cargo deny check` at `:121`; the
  aggregate's only failure test is `contains(needs.*.result, 'failure')` at `:3756-3757`, which a
  `continue-on-error` job reports as `success`. **Structurally invisible.**
- **The advisories do NOT reach shipped binaries.** Measured: `cargo tree -p maos-bin --edges all |
  grep -ci wasmtime` → **0**; with `--features wasm-host` → **39**. They enter the deny graph only
  via workspace member `maos-wasm-host` (`Cargo.toml:54` + `crates/maos-wasm-host/Cargo.toml:40-41`),
  which `deny.toml:9`'s `all-features = false` does not exclude. **This strengthens AC3's WASM-off
  statement** — it gives it a second, security-hygiene justification beyond export control.
- **The fix needs no manifest edit.** Both advisories report `Solution: Upgrade to >=46.0.3,
  <47.0.0`, and `crates/maos-wasm-host/Cargo.toml:40` already declares `version = "46"`. A bare
  `cargo update -p wasmtime` satisfies both.
- **The exception path falsifies its own list.** `deny.toml:16-19` asserts *"All entries below are
  `unmaintained` advisories (NOT `vulnerability` class) … none masks a security vulnerability."*
  Both new RUSTSECs are `vulnerability`. Ignoring them requires amending that header in the same
  commit — a self-invalidating control comment of exactly the class this project's retros flag.

Ruled in F17.

---

## 11. 🔴 AC2's PROVISIONING CHECKLIST: §2.6 IS WRONG ABOUT TWO ITEMS, UNDER-LISTS THE SECRETS, AND ITS REPORTING EDGE IS A TRAP

**Ground truth, from the tree and the live repo API.** `GET actions/secrets` → `{"total_count":0}`;
`GET actions/variables` → `{"total_count":0}`; `GET actions/runners` → `{"total_count":0}`. **No
`vars.` reference, no `environment:` key and no `runs-on: self-hosted` exists in any of the 11
workflow files.** CI references **eight** `secrets.*`, of which `GITHUB_TOKEN` is auto-provided —
so **seven** are operator-provisioned, where §2.6 names **two**.

| §2.6 item | measured verdict |
|---|---|
| `RELEASE_SIGNING_KEY` | absent; **hard fail** unset (`release_verify.rs:171-173`) |
| `MAOS_RELEASE_PUBKEY` | absent; **vacuous pass** — see below |
| macOS/aarch64 build | **never run** — 0 remote tags, 0 releases; the single `release.yml` run ever (`27854958328`) is a 0-second startup failure with `jobs: []` |
| `lunarpulse/homebrew-maos` | **repo does not exist** (API 404). Formula is a scaffold: dev pubkey `maos.rb:20`, 5× `PLACEHOLDER_SHA256_*` |
| AUR account | not submitted; `PKGBUILD` has 6× `PLACEHOLDER_SHA256_*`, stale `pkgver=0.5.0`, dev pubkey `:36` |
| `rto-ledger` branch | **✅ EXISTS AND WORKS** — 8 real weekly appends since 2026-07-19; `check-rto-gate` executes for real using the default `GITHUB_TOKEN` |
| `fuzz-ledger` branch | absent — **and NOT a provisioning item** (see below) |
| self-hosted runner / 24-h fuzz | no runner; **no 24-h run was ever configured** — `fuzz-cadence.yml:81-82` is 600 s × 4 workers, weekly, no `timeout-minutes` |
| ≥3 external authors | absent; `check_third_party_trial.rs:325` **fails at `< 12`**, so an honest N=3 reds a green gate |
| pen-test firm + dates | absent — **the one item `RELEASE-HOLDS.md` already owns** (Hold 1, `:24`) |
| Calendar OAuth / Gemini-Bedrock-Vertex | absent; **no name exists in the tree** for either |

**Two §2.6 claims are disproved outright:**

1. **The ledger write tokens are not provisioning items.** `rto-ledger` has taken 8 real commits with
   the default `GITHUB_TOKEN`, which proves push permission. `FUZZ_LEDGER_WRITE_TOKEN` /
   `RTO_LEDGER_WRITE_TOKEN` are both `|| secrets.GITHUB_TOKEN` fallbacks
   (`fuzz-cadence.yml:124`, `rpo-rto-cadence.yml:113`).
2. **`fuzz-ledger` is blocked by a code bug, not by provisioning.** `fuzz-cadence.yml:146` is
   `jq --slurpfile rec "$f" '.records += $rec[0]'` — `$rec[0]` is an **object**, `.records` is an
   **array**, and jq refuses `array + object`. Nothing is staged, `git diff --cached --quiet` is
   true, `"no new records to append"` prints, the push at `:154` is never reached and the branch is
   never created. The job exits **`success`** (the failing `jq` sits left of an `&&`, exempt from
   `set -euo pipefail`). `check-fuzz-floor` has therefore **never executed**. *Creating the branch
   and granting a PAT would change nothing.* Filed to 20-3 (§12); the checklist records it as
   `not-a-provisioning-item`.

**Two more facts the checklist must carry, because they make "provision the secret" insufficient:**

- **The guardrail's vacuity is compile-time.** `release_verify.rs:280-293` wraps its assertion in
  `if option_env!("MAOS_RELEASE_PUBKEY").is_some()`. Unset → the body never compiles in → an empty
  green test. And `maos-audit` has **no `build.rs`** and no `cargo:rerun-if-env-changed`, while
  `release.yml:66` restores a `rust-cache` — **so a cached rlib can satisfy the guardrail even after
  the secret is provisioned.**
- **Provisioning cannot produce a signature at HEAD.** `release.yml:67-69` uses
  `download-artifact@v4` with no `merge-multiple: true`, so `:72-73`'s `sha256sum maos-*` globs
  directories and the step dies **before** `RELEASE_SIGNING_KEY` is read. (15-4 owns the fix.) Also
  only `maos` is packaged — there is no `maosctl` artifact, which `ops-first-signed-tag`'s acceptance
  assumes.

**The reporting edge is a trap.** AC2 says the row *"reports against"* the checklist. `sprint-status.yaml`
is machine-read by four gates, but none reads a doc path from a comment and **none reads
`docs/runbooks/` at all** — so the edge is human-only. Worse: `ops-provisioning-secrets-and-accounts`
is a `development_status` key and therefore enters `governed_story_keys`. `gate_common.rs:96` exempts
it only while `backlog` or `done`:

```rust
!matches!(status, "backlog" | "done") && !stories_dir.join(format!("{key}.md")).exists()
```

**Flipping the row to any active status to "report" hard-fails every D19-governed gate**, demanding
`_bmad-output/implementation-artifacts/ops-provisioning-secrets-and-accounts.md`, which does not and
should not exist. Ruled in F19.

**Scope mismatch:** §2.6's eleven items are owned by **five** operator rows
(`sprint-status.yaml:281,282,283,289,293`), not one — and §2.6 names Bedrock/Vertex credentials,
which have **no** row at all, while the sprint row names `DOCKERHUB_*` + cosign, which §2.6 omits.

**Conventions, measured:** 7 runbooks, **no frontmatter in any**, 4 of 7 carry a bolded
key-value block after the H1, **3 of 7 use an `xx-n-` prefix and 4 of 7 are unprefixed** — there is
**no `ops-` prefix precedent in the tree**. **No runbook has a per-item state column**, and
`grep -rn "waived\|waiver"` outside `_bmad-output` returns only ADR prose. The nearest shape in the
repo is `RELEASE-HOLDS.md:22-25`'s `Status` column with `❌ Open`.

---

## Decisions ratified in this story (rule 7 — one decision per fork)

| # | Fork | **Ruling** | Killed alternative |
|---|---|---|---|
| **F1** | ADR numbering + frontmatter template | Mint **060–064** into the reserved gap; frontmatter = the **`ADR-065`/`ADR-066` shape** (`Status/Gate/Decided/Accepted-in-PR/Revisits/Supersedes`), `Status: ACCEPTED — ratified 2026-09-… under Story 15-5 …`; rows **inserted at `index.md:45`**, before ADR-065 | Copying `ADR-059`'s `Amends/Reuses` — 2 of 44, and ADR-062 has nothing to amend |
| **F2** | The story's oracle | Replace `ls` with **`xtask/tests/decision_adrs_and_provisioning.rs`** — uncharged, run by 15-1 AC7's workspace job, built on the `d19_story_file_governance.rs` planted-red pattern | `ls` (passes on empty files); a new `xtask/src/check_*.rs` gate (150–400 charged lines against 35 headroom) |
| **F3** | ADR-060's WASM form token | **`wasm-component`** — matches `SpiritForm::WasmComponent` and 17-3b AC4, and **preserves** the `"wasm"` negative vector at `manifest.rs:2589-2593` | `wasm` — forces 17-3b to delete a shipped negative test |
| **F4** | ADR-060 vs four prior dispositions | ADR-060 **`Supersedes:` ADR-031's deferral clauses (§54/§63/§71)** and **`Revisits:` ADR-040, ADR-002 and arch §13.1 `:45`**, and disambiguates "no subprocess Spirit host" (SWP host) from ADR-031's wasmtime runner process | Authoring it silently — a `binding-v2.0` contradiction no gate can catch |
| **F5** | `cli_wrapper`'s place | It is an alternative **manifest shape** (XOR with `[class]`), **not a form value**; ADR-060 governs `[class].forms` and names `[cli_wrapper]` as the third hosting shape with its own T3 control (`ECliWrapperRequiresT3`) | Listing it as a form — contradicts a shipped admission error |
| **F6** | ADR-061 vs `credential_posture_2c.rs:299` | The ADR states the **ORDER**: the proxy lands first, and `env_clear` + the inversion of that test happen **in 17-1's single commit**, never before | Writing `env_clear` flat as a hardening — breaks the paid worker path by name |
| **F7** | Which spawn path ADR-061 governs | The **`cli_wrapper` Worker path** (`runtime.rs:449`). T3's `--network=none` is cited as the **model**, not the mechanism — the only production `spawn_t3` caller is a diagnostic smoke | "T3 egress allowlist for Workers" — a union of two disjoint paths |
| **F8** | ADR-062's supersession scope | Supersedes the **general** read-only posture; **PRESERVES the certificate-identity write-path exclusion**. The three read routes answer **405**, a route-scoped refusal that survives (`respond` gains the phrase — 16-1's line, not this story's) | Inverting all three `POST → 404` tests — deletes two identity guards 21-3 AC2 depends on |
| **F9** | ADR-063's post-grace variant vs `RELEASE-HOLDS` (c.6) | **Mint the variant, and define it by the JOIN, not the leaf.** `CertPostGraceReject` is emitted **only** where the emitter observes a closed rotation window for that peer within the grace (the audit join (c.6) itself names); otherwise the existing `PeerIdentityUnverified` stands. The §7.2.1.a **log** token is unchanged. This satisfies §H-9 (a variant), satisfies (c.6) (asserts no distinction the system cannot make), keeps the `+3 maos-domain` ledger, and closes 14-2b's dangling ownership | **(A)** upholding (c.6) with no variant — leaves §H-9 and the ledger reservation stranded; **(B)** adding retired-generation history to `TofuPin` — real, but grows `maos-a2a-core` and 21-3's scope for a distinction the join already supplies |
| **F10** | ADR-063 §H-10 listen-side scoping | **SAN** — the identity claim rides the client leaf's SAN (`PeerId`), scoping the listen-side pin lookup; crypto still refuses a forgery. Follows ADR-055's shipped pattern (verify-key derived from the **claimed** identity, refused at `verify`). ⚠ **Consequence stated for the operator: this ADDS `maos-a2a-tcp +30–80` to 21-3 AC4** rather than deferring it to `RELEASE-HOLDS` row 12 | A post-handshake binding frame — adds a round trip, a new frame kind, and moves the trust decision *after* the handshake completes |
| **F11** | ADR-064 vs 15-6 duplication | **ADR-064 holds the normative clauses; 15-6's AC1/AC4 are amended to CITE it**, and a **`15-5 → 15-6` edge** is added to `dependency-dag.md:213` | Leaving both — guaranteed drift; rule 9 would demand another mid-epic refinement round |
| **F12** | Replay precedence lattice | **Unset = today's behaviour EXACTLY** (cassette-presence stays a selector, `--live` still wins) so the 10 existing call sites stay green. **When set, the mode is authoritative**, and `--live` alongside `MAOS_INFERENCE_MODE=replay` is a **typed error**, not a silent precedence | Making the mode authoritative unconditionally — changes the exit code of every existing unconfigured run |
| **F13** | `MAOS_REPLAY_STRICT` | **Kept, orthogonal** — it is a modifier, not a mode. ADR-064 says so explicitly because epic-18's exit block uses it 3× | Folding it into a `replay:strict` token — silently breaks three exit lines |
| **F14** | Stability class | `MAOS_INFERENCE_MODE` = **`UserFacing`**, and `MAOS_REPLAY_CASSETTE` is **promoted to `UserFacing` with it**; `MAOS_REPLAY_STRICT` stays `HarnessOnly` | A `UserFacing` mode whose required argument is `HarnessOnly` — incoherent, and 15-6 AC4 as written produces exactly that |
| **F15** | What `live` means with no provider | `MAOS_INFERENCE_MODE=live` checks for a **real** provider and exits non-zero typed when only `UnconfiguredProvider` is installed. The fallback is **flagged, not deleted**, so the unset path is unchanged | Deleting `UnconfiguredProvider` (changes today's boot); or trusting `default_id().is_some()`, which the fallback makes always true |
| **F16** | AC3's placement in `STABILITY.md` | **Inside the `<!-- PRESERVED:export -->` fence** (`:91-113`) — zero xtask lines, zero gate risk, topically correct. Self-builder `--features wasm-host` docs go to **`README.md`** | A new `## WASM` section (reds 3 jobs incl. `aggregate`); amending `render()` (charged lines against 35 headroom) |
| **F17** | The `wasmtime` advisories | **Upgrade** — but ⚠ **the one-command form is a NO-OP and was DISPROVED by running it** (2026-09-08): `cargo update -p wasmtime` reports `Locking 0 packages`, and `--precise 46.0.3` **fails to resolve** — `wasmtime-wasi 46.0.2` pins `wiggle "=46.0.2"` which pins `wasmtime-environ "=46.0.2"`, while `wasmtime 46.0.3` demands `"=46.0.3"`. The family must move together: **`cargo update -p wasmtime -p wasmtime-wasi --precise 46.0.3`**, measured to move **39 packages** 46.0.2 → 46.0.3, and **still no manifest edit** (both requirements are `"46"` = `^46`, `crates/maos-wasm-host/Cargo.toml:40-41`). Oracle: `cargo deny check advisories` green | The documented-exception path — falsifies `deny.toml:16-19`'s own header, and both advisories are `vulnerability` class |
| **F18** | Checklist home + state vocabulary | `docs/runbooks/provisioning-checklist.md` (unprefixed, the 4-of-7 majority). States: **`present` / `absent` / `waived-by <name> <date>` / `not-a-provisioning-item (<reason>)`** — the fourth is required because measurement disproved two §2.6 items | `ops-1-` prefix (invents a new family); a three-state vocabulary that would force `fuzz-ledger` into a false `absent` |
| **F19** | The AC2 reporting edge | The operator reports **by editing the checklist**, and `ops-provisioning-secrets-and-accounts` **stays `backlog` until done**. The checklist says so in its own header, citing `gate_common.rs:96` | "Flip the row to in-progress to report" — hard-fails every D19-governed gate demanding a story file for an ops row |
| **F20** | Four ADRs or five | **Five.** The epic and the story key are authoritative; `sprint-status.yaml:237`, `epics/index.md:153`, `dependency-dag.md:213` and `13-phased-roadmap.md:5` are amended (§12) | Dropping ADR-064 to match the majority — it is the one ADR with five consuming epics |


⚠ **F1–F20 were ruled at story creation. R1–R9 (§13) were ruled at the 2026-09-08 round-table and AMEND them** — chiefly F2, whose oracle the room found carried five defects of its own.

---

## Acceptance Criteria (9)

- **AC1 (command).** `cargo test -p xtask --test decision_adrs_and_provisioning` is green, and it is
  a **real** control, not a presence check. The target is **new**, lives in `xtask/tests/`
  (**uncharged** — `kloc.toml:2`), and is executed in CI by 15-1 AC7's `workspace-test-suite` job
  (`cargo test --workspace --locked --no-fail-fast`; `xtask` is workspace member #1, `Cargo.toml:4`,
  so `--workspace` runs `xtask/tests/*`). ⚠ **T0 asserts that job exists at HEAD before a line is
  written** (R1) — it is unstaged in 15-1's tree today. Fixture-planting uses the
  `d19_story_file_governance.rs` tempdir + `current_dir` idiom (`:19-23`); the real-corpus clauses
  read the real tree.
  **Denominator first (R6 — this is what makes the rest non-vacuous).** Before any mapping is
  asserted the test **fails loudly** unless it found: ≥ 11 files under `.github/workflows/`, ≥ 7
  distinct non-`GITHUB_TOKEN` `secrets.<NAME>` references, and ≥ 44 files matching
  `docs/adr/ADR-*.md`. An empty glob must red, never satisfy a universal quantifier vacuously —
  the absent-pass shape this story's own §2 indicts.
  Then it asserts:
  **(a)** the five files `docs/adr/ADR-06{0,1,2,3,4}-*.md` **each exist** — *existence, not
  exclusivity* (R6/Yui: asserting "exactly five match `ADR-06[0-4]-*`" reds on a future superseding
  `ADR-060b` authored by someone who did nothing wrong);
  **(b)** each has YAML frontmatter carrying the F1 key set, with `Status` beginning `ACCEPTED`;
  **(c)** each body is non-vacuous **and anchored to the tree** — shape (≥ 40 lines, a `## Context`
  and a `## Decision` heading, none of `TODO`/`TBD`/`<PLACEHOLDER`/"to be decided") **plus two
  anti-rot anchors per ADR** (R3), each keyed to a claim that ADR's own `## Context` makes, so the
  ADR reds when the code moves under it. Shape alone is prose wearing a test's clothes. The ten
  anchors are named in **Dev notes**;
  **(d)** consumer naming is **bidirectional** (R2) — each ADR names ≥ 1 consuming story key that
  exists in `sprint-status.yaml`, **and that story's own epic section cites the ADR number back**.
  A one-way string match is the `resolve-or-be-named` receipt this room already killed once in 15-3
  (*"what stops me writing `maos summon-dragon`?"*); the back-reference already exists for four of
  five (e.g. `epic-17…:53` "Design in force (D-B, ADR-061 authored by 15-5)"), so the assertion is
  satisfiable today and breaks only on invention;
  **(e)** `docs/adr/index.md` and `ls docs/adr/ADR-*.md` are a **bijection** — every file has a row
  and every row has a file (this forces the ADR-027/042/043/044 repair of §1), rows are ascending,
  and every row parses as a 4-cell table row (this forces the `index.md:40` repair);
  **(f)** every `secrets.<NAME>` in `.github/workflows/*.yml` other than `GITHUB_TOKEN` has a row in
  `docs/runbooks/provisioning-checklist.md`, and every checklist row carries one of the four F18
  states. ⚠ **This clause is a cross-story tripwire and its failure message must be actionable**
  (R4/Dana): it names the workflow, the secret, the checklist path and the four legal states — a red
  that lands on a stranger's PR without telling them the fix is a trap, not a control. Same rule for
  every denominator failure.
  **Proven-red, one planted vector per clause (a)–(f) plus one per denominator**, each asserting the
  test **fails** when the defect is planted — an absent ADR, a missing frontmatter key, a shape-only
  body whose anchor no longer matches the tree, a one-way consumer reference, a file with no index
  row, a workflow secret with no checklist row, and an empty workflow glob. Per the `d19` header
  rule: *acceptance is the planted red, not the helper.*

- **AC2 — ADR-060, Spirit forms by trust tier (Fork C; carries D-C and D-E).** `docs/adr/ADR-060-*.md`
  is ACCEPTED and states, as decided content: `rust-inproc` is the **first-party** form — kernel-
  distribution code, never admitted from a registry, in-process by construction (`scheduler_loop.rs:243`
  `load<T: Spirit>`, so there is no process to sandbox; `spawn_sandboxed` has zero `maos-bin` callers);
  the **`wasm-component`** token (F3) is the third-party and polyglot form; and **`[cli_wrapper]` is a
  manifest shape, not a form** (F5), XOR with `[class]` per `EManifestSchemaConflict`
  (`xtask/error-catalog.toml:136-138`), tier-controlled by `ECliWrapperRequiresT3`. It states
  explicitly that **admission-by-form does not exist today and this ADR creates it** — neither
  `maos-registry::admit_spirit` (`admission.rs:179`) nor `SecurityPolicy::admit`
  (`kernel-core/src/security/mod.rs:266`) reads `class.forms` — and that accepting `wasm-component`
  is a **manifest-validator change** (`manifest.rs:392-398`) with a `RawClassSection`
  `deny_unknown_fields` (`:290`) and schema-version consequence, owned by 17-3b. It names **both**
  taxonomies (`manifest.rs:222` strings vs `maos-host/src/lib.rs:48` `SpiritForm`) and says which it
  governs. Per F4 it carries `Supersedes: ADR-031 §§54/63/71` and `Revisits: ADR-040, ADR-002,
  architecture §13.1 (13-phased-roadmap.md:45)`, and disambiguates "no subprocess Spirit host" from
  ADR-031's wasmtime runner process. **D-C:** the WASM recall service is a maos-bin side-channel on a
  **unix socket** whose path maos-bin hands the runner; the runner's stdio **is** the ADR-032 bridge
  (`runner.rs:1-5,:185-188`) and stays byte-identical; the WIT world imports nothing
  (`wit/spirit.wit:219-230`); kernel-Δ 0; **written as to-build** — `UnixListener`/`UnixStream` have
  zero occurrences workspace-wide — and priced by 17-3a. **D-E:** the daemon renders the FR17 digest
  by calling `Butler::morning_digest` (`spirits/butler/src/lib.rs:675`; `butler` is already a
  `maos-bin` dep, `Cargo.toml:82`; **zero production callers**, 4 test/bench callers listed) without
  Butler joining the topology. It records that **FR5's mechanism and FR33's Go v1.5+ line are
  amended, not restated** (`prd/functional-requirements.md:28`, `:82`).

- **AC3 — ADR-061, Worker egress and scoped credential (D-B).** `docs/adr/ADR-061-*.md` is ACCEPTED
  and states: the Worker container keeps `--network=none` semantics toward the internet; its only
  reachable endpoint is a **daemon-side egress proxy on host loopback** holding the vendor key,
  enforcing a manifest allowlist and journalling every refusal out of port. Per **F7** it names the
  **`cli_wrapper` spawn path** (`runtime.rs:449-472`) as the path it governs and cites T3's flag set
  (`t3/argv.rs:69-76`) as the model, recording that the only production `spawn_t3` caller is a
  diagnostic smoke (`main.rs:7059-7072`) and that a T3 container already receives no host env
  (`t3/spawn.rs:115`, no `-e` flags). Per **F6** it states the **order**: the proxy lands first, and
  `env_clear` — together with the inversion of
  `crates/maos-cli/tests/credential_posture_2c.rs:299-304` — happens in **17-1's single commit**,
  never as a standalone hardening. It records the three things that do not exist and must be
  designed: the manifest `[network]`/egress schema (priced against **`maos-manifest`** under Epic 17,
  `kloc.toml:322-323` — **not** `maos-domain`), the host-side proxy's crate home (respecting the
  `maos-a2a-tcp ↛ maos-kernel-core` barrier, `kloc.toml:372`), and what a "scoped credential" **is**,
  given `CapabilityToken` (`i1.rs:175-186`) carries no scope, audience or secret and scope is
  registry-side (`ports/capability.rs:29`). It names the TL append (`transparency_log.rs:596`) and
  its panic-on-write contract, and records that 17-1's `+65–130` kernel lines sit on a row at
  **18935/18935, zero headroom**, funded only in the aggregate (`kloc.toml:197,:199`).

- **AC4 — ADR-062, the mutating operator surface (D-A).** `docs/adr/ADR-062-*.md` is ACCEPTED and
  states: the `maos run` / `maos shell` daemon binds a **loopback HTTP POST door with a bearer**;
  `maos init` mints endpoint + bearer into `MAOS_HOME/control.json`; `maosctl` discovers both from
  that file; **one trust path**. Per **F8** it supersedes the *general* READ-ONLY posture at
  `crates/maos-control/src/lib.rs:66-77` while **preserving the certificate-identity write-path
  exclusion** — the write path for cohort/rotation state remains the signed manifest reissue and
  nothing else, as `21-3 AC2` requires — and it rules that the three read routes answer **405**
  (`respond`'s phrase table, `lib.rs:452-461`, gains it in 16-1), **not** that
  `lib.rs:607-611`/`:786-790`/`:927-931` are inverted to accept mutation. It records the surface as
  it is (4 GET routes `:299,347,392,428`; catch-all 404 `:434-440`; header-only read loop `:274-283`;
  constant-time bearer `:288-292`; loopback enforced `:213-218`; sync accept loop `:224`), states the
  delta as **POST + body parser + typed errors** (not "gets an HTTP client" — `maos-cli` has one at
  `subcommands.rs:1930-1975`), cites `dep_kernel_core_free_test.rs:8-29` as the **existing**
  enforcement of the no-`maos-control`-link rule, names **both** `maos init` arms (`main.rs:1699`
  and `:1352`) and that `run_init` writes `config.toml` today (`maos-shell/src/lib.rs:48-49`) while
  `control.json` has zero source occurrences. ⚠ **The 405 body must not echo the request line** (R5/Vex): `respond` takes a `&[u8]`
  literal at every call site today (`lib.rs:452-470`) and the ADR requires it stay that way — the
  moment a request is formatted into a response, an operator's own log becomes an injection surface.
  Vex cleared the 405 itself by tracing the order: the bearer check at `lib.rs:288-292` returns 401
  **before** any route match, so a 405 is reachable only post-auth and leaks no route existence to an
  anonymous scanner. Frontmatter per F1: `Supersedes: nothing — supersedes a
  rustdoc decision, not an ADR` + `Revisits:` the comment and the three tests. **The stale
  `main.rs:9844-9853` pointer inside the superseded comment is not propagated.**

- **AC5 — ADR-063, the revocation model.** `docs/adr/ADR-063-*.md` is ACCEPTED and **ratifies the
  two-axis status quo rather than choosing one model**: spirit-class revocation is a **signed CRL**
  (`maos-domain/src/revocation.rs`; `applier.rs:73`; wired `main.rs:2876-2906`; semantics pinned by
  `on_revocation_three_actions.rs` — three actions, 10 000 ms drain deadline, one
  `spirit.quarantine_requested` frame, ≥2 halt receipts per pid), and peer-certificate revocation is
  **reissue-is-revocation** (`maos-cohort/src/rotation.rs:22-36`), with **no OCSP/CRL cert client**
  to be built. It resolves the (c.6)-vs-§H-9 collision per **F9**: `RuptureReason::CertPostGraceReject`
  is minted and **defined by the rotation-window join**, not by a leaf classification the seam cannot
  make (`transport.rs:1034` says so in its own sentinel; `TofuPin` `tofu.rs:18-30` keeps no retired
  generation); the §7.2.1.a **log** token
  (`7-inter-agent-communication.md:74`) is unchanged; `RELEASE-HOLDS.md` (c.6) is **explicitly
  overturned with this reason**, and 14-2b's dangling ownership (`14-2a…:873`) is closed. It decides
  **§H-10 per F10 — SAN** — and states the consequence that 21-3 AC4 builds it (`maos-a2a-tcp
  +30–80`) rather than recording `RELEASE-HOLDS` row 12's remaining half. ⚠ **The migration is named,
  not discovered** (R7): SAN-in-the-client-leaf means every peer's leaf must be **reissued** carrying
  the `PeerId`, which is not thirty lines — it is a fleet operation. The ADR states that the reissue
  **rides 21-3 AC2's own rotation path** (`swap_serving_cert` gaining its first production caller,
  driven by the signed manifest reissue, under `G ≈ 65 s`), so the story that needs the migration is
  the story that builds its vehicle. It also records **why the binding frame lost**: not cost, but
  **when the trust decision happens** — a post-handshake frame moves it after the handshake has
  already completed, which is the weaker posture. It records
  `G = cold_deployment_t_grace() + confirmation_interval() ≈ 65 s` (14-2d `:141-142`) as **ratified
  but unbuilt**, owned by 21-3. Cites use `frame.rs:383` for `#[non_exhaustive]` and `:384` for the
  enum, and state that `journal_peer_identity_refusal` takes `detail` (`router.rs:506`) but **drops
  it for durability** (`:556-561`).

- **AC6 — ADR-064, one replay selector (D-D).** `docs/adr/ADR-064-*.md` is ACCEPTED and states the
  **full precedence lattice** (F12): `MAOS_INFERENCE_MODE={record,replay,live}` with
  `MAOS_REPLAY_CASSETTE=<path>`; **unset = today's behaviour exactly** (cassette-presence remains a
  selector, `--live` still wins) so the ten existing `maos run … --once` sites stay green; when set,
  the mode is authoritative and `--live` with `MAOS_INFERENCE_MODE=replay` is a **typed error**. It
  rules **F13** (`MAOS_REPLAY_STRICT` kept, orthogonal — epic-18's exit block uses it 3×), **F14**
  (mode `UserFacing`, `MAOS_REPLAY_CASSETTE` promoted with it, `MAOS_REPLAY_STRICT` stays
  `HarnessOnly`) and **F15** (`live` checks for a real provider; `UnconfiguredProvider` is flagged,
  not deleted — it is a fallback that **succeeds**, `main.rs:3155-3159`, and today masks the one
  non-zero-exit path at `:4624-4629`). It retires `--replay-llm` (`worker_spawn.rs:59`, plus the doc
  at `:34`, the comment `:56-58` and the usage strings `:65,:78`) and `MAOS_JOURNEY_MODE`
  (`env_contract.rs:260`), states that **`--replay` and `--deterministic` have never existed** (so
  neither is "retired"), and records that the **shell adapter has no cassette branch at all**
  (`main.rs:3170` vs the Researcher chain at `:4633-4680`), so honouring a cassette in `maos shell`
  is **net-new wiring** owned by 15-6. All `main.rs` cites are the measured `+34` values. Per **F11**
  the ADR is normative and 15-6 cites it. It also records **why D-D earns an ADR when D-F, D-H and
  D-I did not** (F20/R-Wildcard): those three have one consuming epic each and were folded into the
  epic file; D-D has **five** consumers and defines a user-facing environment contract that outlives
  every one of them. The line is not *"is it architecture"* but **"must the decision survive the epic
  that consumed it."** Written into the ADR because the next author will ask.

- **AC7 — the provisioning checklist.** `docs/runbooks/provisioning-checklist.md` (absent at HEAD)
  exists and is **derived from the tree, not from §2.6's prose** (confidence rule 8). It lists every
  operator-provisioned `secrets.<NAME>` in `.github/workflows/*.yml` — **seven**, excluding the
  auto-provided `GITHUB_TOKEN` — plus the non-secret external items, each row carrying: the real name
  in the tree, the consuming `workflow:line`, **what happens when it is unset** (hard fail / vacuous
  pass / skip-by-design / invalid-ref), the owning `ops-*` sprint row, and one of the four F18 states
  `present` / `absent` / `waived-by <name> <date>` / `not-a-provisioning-item (<reason>)`. It records
  the measured disproofs of §11: `rto-ledger` is **`present`** (8 real appends since 2026-07-19; the
  default `GITHUB_TOKEN` suffices, so both `*_LEDGER_WRITE_TOKEN` secrets are
  `not-a-provisioning-item`); `fuzz-ledger` is `not-a-provisioning-item (blocked by the jq
  array+object bug at fuzz-cadence.yml:146 — filed to 20-3)`; **no self-hosted runner is registered
  and no 24-h fuzz was ever configured**. It records the two reasons provisioning alone is
  insufficient — the `option_env!` compile-time vacuity plus `rust-cache`
  (`release_verify.rs:280-293`, no `build.rs`), and the `download-artifact` merge defect that fails
  **before** the key is read (`release.yml:67-73`, 15-4's fix). It **cross-references** `RELEASE-HOLDS.md`
  Hold 1 (`:24`) for the pen-test item rather than restating its status, and names the five owning
  ops rows for the items that are not this row's. Its header carries the **F19** rule: report by
  editing this file; `ops-provisioning-secrets-and-accounts` stays `backlog` until done, because
  `gate_common.rs:96` would otherwise demand a story file for an ops row.

- **AC8 — `STABILITY.md` and the self-builder docs.** The WASM-off-until-Hold-2 statement lands
  **inside the `<!-- PRESERVED:export -->` fence** (`STABILITY.md:91-113`, per **F16**), containing
  no fence marker and not the stub phrase, and
  `cargo run -p xtask -- stability-matrix --check --json` still reports
  `{"passed":true,"in_sync":true,…}` — with `check-export-control` and `smoke-abi-7-5a` also green.
  The statement cites the two live facts: `wasm-host` is off by default
  (`crates/maos-bin/Cargo.toml:30`; `default = ["network"]` `:15`) as the **5D002.c.1 export-control
  precondition**, and — measured — the engine is **absent from the shipped default closure**
  (`cargo tree -p maos-bin --edges all` → 0 wasmtime lines; the coordinated 46.0.3 update moved 39 lockfile entries), which
  is also why the two RustSec advisories of AC9 never reached a published binary. `README.md` gains
  `--features wasm-host` in its build documentation (`:203-206` or `:376-381`, which list build
  commands and mention no feature flag today), pointing at
  `docs/compliance/export-counsel-precondition.md` for the full treatment. **`RELEASE-HOLDS.md` is
  not edited** — it already carries the statement at `:25`, `:29-35` and `:72-73`.

- **AC9 — cross-document reconciliation and the `wasmtime` decision.** `cargo deny check advisories`
  exits 0: `Cargo.lock` carries the `wasmtime` family at **46.0.3** via
  **`cargo update -p wasmtime -p wasmtime-wasi --precise 46.0.3`** (**F17 as corrected** — the
  single-package form is a measured no-op and `--precise` on `wasmtime` alone fails to resolve against
  `wiggle`'s exact `wasmtime-environ` pin; the coordinated form moves **39 packages** and needs **no
  manifest and no `deny.toml` edit**, both requirements being `"46"` at
  `crates/maos-wasm-host/Cargo.toml:40-41`), and the story records the measured consequences: `Cargo.lock` churn
  invalidates `hashFiles('**/Cargo.lock')` cache keys and re-runs `reproducible-build`'s byte-identity
  legs, and any new `multiple-versions` finding is **recorded, not suppressed** (`deny.toml:55` is a
  separate check class from advisories). Separately, the PRD delta
  (`prd/project-scoping-phased-development.md:18`), the architecture note
  (`13-phased-roadmap.md:5`) **and the R3 delta at `13-phased-roadmap.md:100`** — which AC4 as written
  omits and which constrains ADR-060/061/062/064 — are cross-checked against the five ADRs, and every
  contradiction found is **amended, not merely read**: the "Four ADRs (060–063)" count
  (`13-phased-roadmap.md:5`), and the two PRD statements the delta never amended (`:31` "single-Spirit
  subprocess", `:43` "subprocess form"). Full OLD→NEW list in §12.

---

## Declared cut lines

**Kept out because it is SCOPE, with the reason named — not because it is effort:**

- **Building any of the five decisions.** ADR-060's admission-by-form and validator change (17-3b),
  ADR-061's proxy and manifest `[network]` (17-1), ADR-062's POST door and 405 (16-1), ADR-063's
  variant, join and SAN (21-3), ADR-064's env plumbing (15-6). This story decides; the named stories
  build. Writing any of it here would put kernel-adjacent code in a documentation story and break
  the ZERO-Δ declaration.
- **The `fuzz-cadence.yml:146` jq fix.** A one-character-class bug (`+= $rec[0]` → `+= [$rec[0]]`)
  that has kept `check-fuzz-floor` from ever executing. It is a **workflow correctness repair**, not
  a decision or a provisioning item; filed to **20-3** (gate-honesty pass) in §12 with the measured
  evidence, and recorded on the checklist as `not-a-provisioning-item` so the operator does not
  provision against a code bug.
- **`release.yml`'s `download-artifact` merge defect and the missing `maosctl` artifact.** Owned by
  **15-4**; the checklist records them as the reason provisioning alone cannot produce a signature.
- **`MAOS_OPERATOR_HTTP_ENDPOINT` registration.** Read at `subcommands.rs:1938`, unregistered, and
  invisible only because `check_env_contract.rs` scans `maos-bin/src` alone. **21-4 AC2** widens that
  scan and owns it; filed in §12.
- **`check-adr040-accepted`'s two defects** — it is in `gate-registry.toml:34` but in **no workflow**
  and in no `EXPECTED_GATES`, and the registry name (`check-adr040-accepted`) does not match the CLI
  subcommand (`check-adr-040-accepted`, `main.rs:471-472`). Pre-existing; filed to **20-3**, whose
  charter is gate honesty. This story must not change `ADR-040`'s `Status:` line (it would break that
  gate locally) — it amends `Superseded-by:` only.
- **Retiring `MAOS_JOURNEY_MODE` and `--replay-llm` in code.** ADR-064 *decides* the retirement;
  15-6 AC3/AC1 executes it.

**Not cut — effort, and it ships here:** the oracle (AC1), the ADR-index bijection repair the oracle
forces (§1), the `Cargo.lock` bump (AC9), and five artifacts' worth of rule-9 amendments (§12).
Under the operator directive, effort ships now.

---

## Dev notes

- **T0 before a line, and its FIRST assertion is not a number** (R1): `git log --oneline -1 -- `.github/workflows/discipline.yml`` shows 15-1 landed **and** `grep -n 'workspace-test-suite' .github/workflows/discipline.yml` returns a job. If either fails, stop — AC1 has no runner.
- **Every number carries its pair** (R9): value at HEAD and value in tree, or an explicit statement of which single tree it belongs to. Three occurrences in five weeks, three different mechanisms; prose disclosure is not a control.
- **The ten anti-rot anchors** (R3), two per ADR, each keyed to a claim that ADR's own `## Context` makes: **060** — `manifest.rs` validator accepts exactly `{rust-inproc, subprocess}`, and `admit_spirit` contains zero occurrences of `forms`; **061** — `credential_posture_2c.rs` still asserts `!RUNTIME_SRC.contains("env_clear")`, and `argv.rs` still emits `--network=none`; **062** — `maos-control` serves four routes and `respond`'s phrase table has no 405, and `dep_kernel_core_free_test.rs` still bans the kernel-core edge; **063** — `RuptureReason` has no `CertPostGraceReject` variant, and `build_server_config` still passes `None`; **064** — `MAOS_INFERENCE_MODE` is absent from `env_contract.rs`, and `worker_spawn.rs` still accepts `--replay-llm`. Each anchor reds when the code moves without the ADR moving — that is the whole point; the dev **updates the ADR and the anchor together**, never the anchor alone.
- **T0 also re-measures the tree.** The tree is dirty with 15-1's 44 uncommitted files. Re-measure after 15-1
  commits: `ls docs/adr/ | tail`, `git status --short docs/adr/`, `cargo run -p xtask -- kloc-check
  --json` (the `xtask` row), `cargo run -q -p xtask -- stability-matrix --check --json`, `cargo deny
  check advisories`. **Do not inherit a number from the frontmatter block.**
- **⚠ PATHS: an `xtask` integration test does NOT run at the workspace root.** Cargo sets CWD to the package dir (`xtask/`), so a bare `.github/workflows/*.yml` or `docs/adr/` glob resolves under `xtask/` and comes back **empty** — the exact vacuous pass R6 exists to catch. Use the shipped idiom: `Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join(rel)` (`xtask/tests/coverage_matrix_completeness_tests.rs:7-8`). If a denominator assertion fires, **the bug is the path, not the threshold** — never loosen the denominator to make it pass.
- **Write the oracle first, and watch it red.** Author `xtask/tests/decision_adrs_and_provisioning.rs`
  before the ADRs. It must fail on clause (a) immediately, then on (e) the moment the first ADR
  lands without an index row. A test written after the artifacts it checks is a test nobody has seen
  fail.
- **The index bijection will red on four pre-existing omissions** (ADR-027/042/043/044) and the
  malformed row at `index.md:40`. That is the oracle working, not a bug — repair them in the same
  commit.
- **Every ADR paragraph must be traceable.** Each of the five carries a `## Context` that states what
  exists at HEAD *with citations*, and a `## Decision` that says what is created. The failure mode
  this story exists to prevent is an ADR that reads like a record of settled fact when it is in fact
  creating a control — §4, §5 and §6 name the exact sentences at risk.
- **Do not touch `STABILITY.md` outside `:91-113`.** Verify with `git diff STABILITY.md` that no line
  outside the fence moved, then run the three gates.
- **`cargo update -p wasmtime -p wasmtime-wasi --precise 46.0.3` — the two-package form, measured.**
  The single-package form is a **no-op** (`Locking 0 packages`) and `--precise` on `wasmtime` alone
  **fails to resolve**: `wasmtime-wasi 46.0.2` → `wiggle "=46.0.2"` → `wasmtime-environ "=46.0.2"`,
  against `wasmtime 46.0.3`'s `"=46.0.3"`. The coordinated form moves 39 packages and needs no
  manifest edit. A bare `cargo update` would churn the whole lock and put `reproducible-build` and
  every cache key in play for no reason.
- **Order:** T0 → oracle (red) → index repair → ADR-064 (least contested) → 062 → 061 → 060 →
  063 (most contested; F9/F10 carry the two rulings a reviewer is most likely to challenge) →
  checklist → `STABILITY.md`/README → `Cargo.lock` → §12 amendments → oracle green.

---

## Tasks / Subtasks

- [x] **T0 — Assert the runner, then re-measure.** (R1) First: 15-1 is committed and the
      `workspace-test-suite` job exists at HEAD — stop if not. Then, after 15-1 commits: ADR corpus count and highest
      number; `xtask` kloc row; `stability-matrix --check`; `cargo deny check advisories`;
      `git status --short docs/adr/`. Record all five in the Dev Agent Record. (AC: all)
- [x] **T1 — Author the oracle and prove it red.** `xtask/tests/decision_adrs_and_provisioning.rs`:
      **denominator assertions first** (R6), then clauses (a)–(f), with **one planted-red vector per
      clause AND per denominator** — including an empty-workflow-glob vector. Bidirectional consumer
      check (R2), ten anti-rot anchors (R3), actionable failure messages naming the fix (R4),
      existence-not-exclusivity on (a). Tempdir-isolated per the `d19` idiom. Confirm every vector
      reds. (AC: 1)
- [x] **T2 — Repair `docs/adr/index.md`.** Add rows for ADR-027, 042, 043, 044; fix the malformed
      ADR-055 row at `:40`; correct the stale "027–029 tracked elsewhere" prose at `:52-57`. (AC: 1e)
- [x] **T3 — ADR-064** (one replay selector). F11–F15. Insert its index row before ADR-065. (AC: 6)
- [x] **T4 — ADR-062** (mutating operator surface). F8; `Supersedes: nothing` + `Revisits:`. (AC: 4)
- [x] **T5 — ADR-061** (Worker egress + credential). F6, F7. (AC: 3)
- [x] **T6 — ADR-060** (forms by trust tier, carrying D-C and D-E). F3, F4, F5. (AC: 2)
- [x] **T7 — ADR-063** (revocation). F9, F10; overturn (c.6) in writing with its reason. (AC: 5)
- [x] **T8 — `docs/runbooks/provisioning-checklist.md`.** Derive rows from
      `grep -rhoE "secrets\.[A-Z_0-9]+" .github/workflows/*.yml`; F18 states; F19 header rule;
      cross-reference Hold 1. (AC: 7)
- [x] **T9 — `STABILITY.md` + self-builder docs.** Use the preserved export fence per F16. (AC: 8)
- [x] **T10 — `cargo update -p wasmtime`.** F17. Confirm `cargo deny check advisories` exits 0;
      record any `multiple-versions` finding rather than suppressing it. (AC: 9)
- [x] **T11 — §12 amendments** across the epic, `sprint-status.yaml`, `epics/index.md`,
      `dependency-dag.md`, and the two architecture/PRD shards. (AC: 9)
- [x] **T12 — Oracle green + full local gate sweep.** `cargo test -p xtask --test
      decision_adrs_and_provisioning`; `kloc-check`; `stability-matrix --check`; `check-env-contract`;
      `check-exit-commands`; `check-epic-close-coherence`; `cargo deny check`. (AC: all)

### Review Findings

- [x] [Review][Patch] HIGH (resolved from Decision D1 — Lunarpulse: extend now): oracle guards none of the non-ADR artifacts this story ships — add clauses + planted reds for the `STABILITY.md` preserved-fence WASM statement, the `README.md` `--features wasm-host` doc, the `Cargo.lock` wasmtime `46.0.3` pin, and a non-secret checklist-row denominator [xtask/tests/decision_adrs_and_provisioning.rs] — uncharged (`kloc.toml:2`), keeps the ◆ net intact.
- [x] [Review][Patch] (resolved from Decision D2 — Lunarpulse): fill Dev Agent Record `Agent Model Used` with `glm-5.3` [15-5-decision-adrs-and-provisioning-checklist.md:1053].
- [x] [Review][Patch] HIGH: §12 rule-9 inversion — T11 "removed stale first-party rust-inproc" inverted the ratified ruling in four surfaces [`13-phased-roadmap.md:5,:45`; `prd/project-scoping-phased-development.md:18,:31,:43`; `epics/epic-15-foundations-w0.md:186`] — they now say "native subprocess is the first-party form … rust-inproc remains speculative," contradicting ADR-060 Decision §1 (`rust-inproc` is the first-party form), story AC2, F4, and §12 items 14/15 (which contract "first-party form" and authorize only the Five-ADR count fix at `:5`); `roadmap:45` attributes the inverted ruling to ADR-060 by name. Restore the §12-contracted text.
- [x] [Review][Patch] MEDIUM: ADR-063 missing four AC5-mandated contents [docs/adr/ADR-063-two-axis-revocation-and-post-grace-identity.md] — (1) R7 migration sentence: the reissue rides 21-3 AC2's `swap_serving_cert` (first production caller, driven by signed manifest reissue, under `G ≈ 65 s`); (2) `G = cold_deployment_t_grace() + confirmation_interval() ≈ 65 s` (14-2d `:141-142`) recorded as ratified-but-unbuilt, owned by 21-3; (3) 14-2b dangling-ownership closure (`14-2a…:873`); (4) paired cites `frame.rs:383` (`#[non_exhaustive]`) / `:384` (enum) — only `:384` present.
- [x] [Review][Patch] MEDIUM: epic-16:66 drift vs ADR-062 [epics/epic-16-one-daemon-one-door-j0-w1.md:66] — ADR-062 `:66-69` rules the three `POST → 404` tests "are not inverted into acceptance tests" and the generic assertion is "narrowed"; the unamended named consumer says they "are inverted under ADR-062". Amend to ADR-062's terms (rule 9).
- [x] [Review][Patch] HIGH: oracle `require_absent` vacuous pass + overclaiming coverage [xtask/tests/decision_adrs_and_provisioning.rs:210,:777,:5] — (a) `read_to_string(...).unwrap_or_default()` makes all four `require_absent` anti-rot anchors silently pass when the anchor file is deleted/renamed (module move/crate split); unreadable anchor must be a red finding; (b) no planted red exists for the `require_absent` regression direction; (c) module doc "every defect class also has an isolated fixture" and test name overclaim — ~15 helper finding classes have no planted red; state per-clause (a)–(f)+denominator coverage precisely (d19 rule: acceptance is the planted red).
- [x] [Review][Patch] MEDIUM: frozen false measurement — `STABILITY.md:118-119` "39-package engine family" is not reproducible [STABILITY.md:118] — measured: `--edges all` closure adds 514 distinct packages; engine-family names = 79 (`--edges normal`); 39 is the F17/T10 lockfile-update count (38 version bumps + 1 windows-core ref). Byte-frozen by `stability-matrix`; reword to the reproducible fact (and fix the story's AC8 parenthetical).
- [x] [Review][Patch] LOW: secret/workflow scanner blind spots [xtask/tests/decision_adrs_and_provisioning.rs:69-70; docs/runbooks/provisioning-checklist.md:23-25] — only `.yml` scanned and `[A-Z_0-9]+` secret names; GitHub permits `.yaml` workflows and lowercase names — future secrets ungoverned while denominators stay green; the checklist's "*.yml" claim needs the same fix.
- [x] [Review][Patch] LOW: epic-15 AC3 cites the fence as `:91-113` but this commit moves the interior to `:91-124` [epics/epic-15-foundations-w0.md:188] — cite the PRESERVED:export markers or the post-edit range (R9's own rule).
- [x] [Review][Patch] LOW: ADR-062:22 "`respond` maps only 200, 401, 404, and 503" misses the 500 catch-all (`crates/maos-control/src/lib.rs:459-465`) [docs/adr/ADR-062-mutating-operator-surface.md:22].
- [x] [Review][Patch] LOW: ADR-063:44 "`close_rotation_window` returns a fingerprint that its caller drops" — it has no production caller (production uses `close_rotation_window_if`, `rotation.rs:1055`); reword to the seam fact [docs/adr/ADR-063-two-axis-revocation-and-post-grace-identity.md:44].
- [x] [Review][Patch] LOW: ADR-064 "Its `default_id()` therefore returns `Some`" — the `Some` comes from router wiring (`main.rs:3157`), not the provider type; fix the antecedent [docs/adr/ADR-064-one-inference-mode-selector.md].
- [x] [Review][Patch] LOW: three identical zero-information `## run @untracked` appends [_bmad-output/implementation-artifacts/intent-lineage-coverage-report.md] — keep one.
- [x] [Review][Patch] LOW: oracle parser hardening [xtask/tests/decision_adrs_and_provisioning.rs:497,:531,:370] — checklist row-skip guards fail-open on `' Item '`/`'---'` substrings anywhere in a row; duplicate checklist items last-win (index reds duplicates, the checklist does not); epic backreference is substring-based (`ADR-060` matches inside `ADR-0600`).
- [x] [Review][Defer] Frontmatter/index parser edges — duplicate frontmatter keys last-win; UTF-8 BOM reds well-formed frontmatter with a misleading message; `## Context` binds to first occurrence anywhere in the body; forbidden-marker scan is plain substring (matches "Todorov"); indented index rows misdiagnosed; waiver dates unvalidated [xtask/tests/decision_adrs_and_provisioning.rs:107-127,:156-163,:414] — deferred, dormant/fail-closed
- [x] [Review][Defer] Corpus regex tolerance — `ADR-060b`-suffixed, subdirectory, and case-variant ADR files are invisible to the corpus/bijection [xtask/tests/decision_adrs_and_provisioning.rs:91-103] — deferred; R6 policy deliberately tolerates the superseding-suffix case; revisit when superseding ADRs appear
- [x] [Review][Defer] No checklist row→secret reverse bijection — retired secrets' rows accumulate silently [xtask/tests/decision_adrs_and_provisioning.rs:586-593] — deferred
- [x] [Review][Defer] Fixture checklist header shape (7-cell) diverges from the real file's 5-cell shape, contra d19's mirror-the-real-file methodology [xtask/tests/decision_adrs_and_provisioning.rs:734] — deferred, nit

---

## 12. Epic OLD→NEW edits (rule 9 — land in this commit)

| # | File | OLD | NEW |
|---|---|---|---|
| 1 | `epics/epic-15-foundations-w0.md:186` (AC1) | `lists five accepted ADRs (numbers free: highest is \`ADR-059\`; frontmatter shape \`Status/Gate/Decided/Accepted-in-PR/Amends/Reuses\`, \`ADR-059:1-8\`…)` | `AC1's oracle becomes \`cargo test -p xtask --test decision_adrs_and_provisioning\` (F2). Numbering: **060–064 are a reserved GAP — the corpus tops at ADR-065 (committed at \`2fd492c0\`) and ADR-066 (15-1); \`ADR-065:93-95\` reserves them by name**; rows are INSERTED at \`index.md:45\`. Frontmatter follows **ADR-065/066** (\`Status/Gate/Decided/Accepted-in-PR/Revisits/Supersedes\`), not ADR-059's \`Amends/Reuses\` (2 of 44). |
| 2 | same, ADR-060 clause | `WASM = third-party/polyglot form; \`cli_wrapper\` = agent CLIs` | `the **\`wasm-component\`** token (preserves the \`"wasm"\` negative vector, \`manifest.rs:2589-2593\`) is the third-party/polyglot form; **\`[cli_wrapper]\` is a manifest shape, not a form** (XOR with \`[class]\`, \`EManifestSchemaConflict\`). Admission-by-form **does not exist and this ADR creates it**. Supersedes ADR-031 §§54/63/71; revisits ADR-040, ADR-002, §13.1. |
| 3 | same, ADR-061 clause | `\`env_clear\`, kernel-minted scoped credential` | `the ORDER is stated: proxy first, then \`env_clear\` **and** the inversion of \`credential_posture_2c.rs:299\` in 17-1's single commit. Governs the **\`cli_wrapper\`** path (\`runtime.rs:449\`); T3 is the model, not the mechanism. \`[network]\` is priced against **maos-manifest** (\`kloc.toml:322-323\`). |
| 4 | same, ADR-062 clause | `supersedes the READ-ONLY decision at \`maos-control/src/lib.rs:68-76\`` | `supersedes the **general** posture at \`lib.rs:66-77\` while **preserving the certificate-identity write-path exclusion** 21-3 AC2 depends on; the three read routes answer **405**, not an inversion of \`:607-611\`/\`:786-790\`/\`:927-931\`. \`maos-cli\` already HAS an HTTP client (\`subcommands.rs:1930-1975\`); the delta is POST. Two \`maos init\` arms (\`main.rs:1699\`, \`:1352\`). |
| 5 | same, ADR-063 clause | `chooses exactly one of reissue-is-revocation / CRL, and defines the machine-readable post-grace token` | `**ratifies the two-axis status quo** (CRL for spirit classes, reissue-is-revocation for peer certs — both ship); the variant is **defined by the rotation-window join, not the leaf** (F9), explicitly overturning \`RELEASE-HOLDS.md\` (c.6) with its reason and closing 14-2b's dangling ownership; **and decides §H-10 = SAN** (F10), which epic-15 never listed. \`#[non_exhaustive]\` is \`frame.rs:383\`. |
| 6 | same, ADR-064 clause | `unset = today's deterministic path; consumed by 15-6, …` | `+ the full precedence lattice (F12), \`MAOS_REPLAY_STRICT\` kept orthogonal (F13), stability classes (F14), \`live\`'s predicate (F15). **\`--replay\` and \`--deterministic\` have never existed.** ADR-064 is normative; **15-6 cites it rather than restating it** (F11). |
| 7 | same, AC2 (`:187`) | `lists every item of preflight §2.6 with a state \`present\` / \`waived-by <name> <date>\`` | `is **derived from the workflows** (rule 8): seven operator-provisioned secrets, not two; four states incl. \`not-a-provisioning-item\`; \`rto-ledger\` is **present** and \`fuzz-ledger\` is a **code bug** (\`fuzz-cadence.yml:146\`), not provisioning; the F19 rule keeps the ops row at \`backlog\` (\`gate_common.rs:96\`). |
| 8 | same, AC3 (`:188`) | `\`STABILITY.md\` gains the WASM-off-until-Hold-2 statement (\`RELEASE-HOLDS.md:29-33,:71\`…)` | `the statement lands **inside the \`<!-- PRESERVED:export -->\` fence** (\`STABILITY.md:91-113\`) — the file is GENERATED under a byte-equality gate and any edit outside the fence reds three jobs incl. \`aggregate\`. Cites corrected to \`RELEASE-HOLDS.md:25,:29-35,:72-73\`. Self-builder docs go to **README.md**. |
| 9 | same, AC4 (`:189`) | `PRD \`[DELTA-2026-09-04 R2]\` … and architecture §13.1 note … cross-checked` | `+ **\`13-phased-roadmap.md:100\`** (the R3 delta, which constrains ADR-060/061/062/064); contradictions are **amended, not merely read** — the "Four ADRs (060–063)" count and PRD \`:31\`/\`:43\`. \`:5\` sits under \`# 13\`; §13.1 begins at \`:43\`. |
| 10 | same, story table `:58` | `Decision ADRs 060–064 and the provisioning checklist ◆ \| rule 7, D-A..D-E · kernel-Δ 0` | `… · **9 ACs (was 4); refined 2026-09-08 by a mid-epic refinement round (rule 9) — this section was amended to match** · 20 forks ruled` |
| 11 | `sprint-status.yaml:237` | `◆ ADR-060 … / 061 … / 062 … / 063 revocation; provisioning checklist runbook` | add **`064 one replay selector`**; record the F17 ruling (upgrade, not exception) against the filed advisories |
| 12 | `epics/index.md:153` | `15.5 ADR-060..063 + provisioning checklist` | `15.5 ADR-060..**064** + provisioning checklist` |
| 13 | `epics/dependency-dag.md:213` | `15-5 → 16-1 (ADR-062), 17-1 (ADR-061), 17-3b (ADR-060), 21-3 (ADR-063)` | `15-5 → **15-6 (ADR-064)**, 16-1 (ADR-062), 17-1 (ADR-061), 17-3b (ADR-060), 21-3 (ADR-063)` — F11 |
| 14 | `architecture-…/13-phased-roadmap.md:5` | `Four ADRs (060–063) are authored in 15-5.` | `**Five** ADRs (060–06**4**) are authored in 15-5.` |
| 15 | `architecture-…/13-phased-roadmap.md:45` | §13.1 disposition: rust-inproc `retired as a roadmap item` | amended to record ADR-060's ruling (first-party form) and the supersession chain, per F4 |
| 16 | `prd/project-scoping-phased-development.md:31,:43` | v0.1 key skill `single-Spirit subprocess`; milestone loads `hello-spirit` (**subprocess form**…) | amended to match `:18`'s own delta, which says that form was never built |
| 17 | `epic-15-foundations-w0.md` §15-6 (`:195`,`:198`) | 15-6 AC1/AC4 restate the D-D decision | rewritten to **cite ADR-064**; all `main.rs` cites corrected by **+34**; `MAOS_REPLAY_STRICT` and the shell adapter's missing cassette branch named — F11 |
| 18 | `epic-20-ship-it-w5.md` §20-3 | — | **filed:** the `fuzz-cadence.yml:146` jq `array + object` bug (`check-fuzz-floor` has never executed; the job exits `success`) and `check-adr040-accepted`'s no-workflow + name-skew defects |
| 19 | `epic-21-…w6.md:28,:72,:73` | `maos-domain +48–78 (\`:278\` 8695/8644, −51)`; AC3's `t_14_2a_post_grace_journal.rs` pointer; AC4 conditional | ceiling is **9192** at `kloc.toml:327`, **+497 free**, D14 closed; the leg moved to `t_14_2a_post_grace_token.rs` (15-1); AC4 is **unconditional — ADR-063 chose SAN** (F10) |
| 20 | `epic-21-…w6.md` §21-4 | — | **filed:** `MAOS_OPERATOR_HTTP_ENDPOINT` is read at `subcommands.rs:1938` and unregistered; 21-4 AC2's workspace-wide scan makes it a red with no owner |

---

## 13. Round-table 2026-09-08 — nine rulings folded in, and a split refused

The room convicted the story with the story's own diagnosis. Every finding is §2's thesis — *a claim
standing in for a control* — committed by the document that names it.

| # | Finding | Ruling |
|---|---|---|
| **R1** | **The control's runner does not exist at HEAD.** AC1 cited 15-1 AC7's workspace job as fact; `workspace-test-suite` is an **addition in 15-1's forty-four uncommitted files**. A control whose runner lives in an unstaged tree is a promise about someone else's work — and 15-1 has already been amended *after* being marked `done` (the 226→316 breach). | `depends_on` becomes a **hard precondition**: 15-5 does not start until 15-1 is committed, and **T0's first assertion is that the job exists at HEAD**. Asserted, never assumed. |
| **R2** | **AC1(d) was a receipt.** "Names a consuming story key that exists in `sprint-status.yaml`" checks that a *string* exists in a YAML file — the `maos summon-dragon` shape this room killed in 15-3. | **Bidirectional**: the ADR names the consumer **and** that consumer's epic section cites the ADR back. Satisfiable today (4 of 5 back-references already exist, e.g. `epic-17…:53`); breaks only on invention. |
| **R3** | **AC1(c) was prose wearing a test's clothes.** ≥40 lines + a `## Decision` heading + no `TODO` is satisfiable with forty lines of nothing — a shape check inside a story whose thesis is that documents are not controls. | Shape **plus two anti-rot anchors per ADR**, each keyed to a claim in that ADR's own `## Context`, so the ADR reds when the code moves under it. Ten greps, uncharged. Precedent: 9.5c's semantic-projection anti-rot gate. Dana held it at two, not five. |
| **R4** | **AC1(f) is a cross-story tripwire.** The next person to add a secret to a workflow reds a test they have never heard of. | Correct behaviour, **hostile without an actionable message**: the failure names the workflow, the secret, the checklist path and the four legal states. Same rule for every denominator failure. |
| **R5** | **The 405 is safe, with one condition.** The bearer check (`lib.rs:288-292`) precedes route matching, so a 405 is post-auth only and leaks no route existence. | ADR-062 records that **the 405 body must not echo the request line** — `respond` takes `&[u8]` literals today and stays that way, or an operator's log becomes an injection surface. A category, not an exploit. |
| **R6** | **Two clauses could pass on an empty set.** Zero secrets found ⇒ "every secret has a row" is vacuously true; zero ADR files ⇒ a perfect bijection. An absent-pass gate inside the story that indicts absent-pass gates. Separately, "exactly five match `ADR-06[0-4]-*`" reds on a future superseding `ADR-060b`. | **Denominator asserted first** (≥11 workflow files, ≥7 non-`GITHUB_TOKEN` secrets, ≥44 ADR files), with its own planted red; and clause (a) asserts **existence, not exclusivity**. |
| **R7** | **F10's SAN ruling had an unpriced migration.** SAN-in-the-client-leaf requires every peer's leaf to be **reissued** — a fleet operation, not thirty lines. | Ruling **stands, strengthened**: the reissue rides **21-3 AC2's own rotation path** (`swap_serving_cert`'s first production caller under `G ≈ 65 s`) — the story that needs the migration builds its vehicle. Named in the ADR so 21-3 does not discover it in week three. The binding frame lost on **when the trust decision happens**, not on cost. |
| **R8** | **Five ADRs or four, decided rather than assumed.** D-F, D-H and D-I were folded into the epic with no ADR; D-D gets one. | **Five.** The principle — written into ADR-064 — is not *"is it architecture"* but **"must the decision survive the epic that consumed it."** D-F/D-H/D-I have one consuming epic each; D-D has five and defines a user-facing env contract. |
| **R9** | **Half the story's load-bearing numbers measure a dirty tree, not `2fd492c0`.** Third occurrence in five weeks, third different mechanism (concurrent agent → rebase → uncommitted sibling). | **Every number carries its pair** — value at HEAD, value in tree — or states which single tree it belongs to. Prose disclosure is not a control. |

**Split refused; dissent recorded.** Dana proposed 15-5a = {ADR-062, ADR-064} (unblocks Epic 16 and
15-6) and 15-5b = {060, 061, 063} (unblocks 17 and 21). John blocked it on sequencing, not cost:
**all three hard ADRs are in the back half** — the ADR-002/031/040 supersession chain and the
overturn of a standing `RELEASE-HOLDS` ruling — leaving the reconciliation in a story with only
Epic 21 queued behind it. *That is the decay curve this epic exists to reverse.* Dana's dissent
stands: outvoted on sequencing, not on cost. She carried R4 instead.

**Grumbal's closing, recorded because it is the review instruction:** *"Its control was a promise.
Its consumer check was a receipt. Its shape check was prose wearing a test's clothes. Its
denominators were absent-pass. And its baseline was borrowed. Five for five — which means the
diagnosis in §2 was right and the author didn't apply it to himself."*


## Dev Agent Record

### Agent Model Used

`glm-5.3` (zai/glm-5.3)

### Debug Log References
- 2026-09-08 T0: verified `.github/workflows/discipline.yml` last changed by `9f46a2df`; `workspace-test-suite` exists and runs `cargo test --workspace --locked --no-fail-fast`, with memberships in both required `needs` lists. Re-measured: 44 ADR files, highest ADR-066, clean `docs/adr/` status, `xtask` 43409/43444, aggregate 157231, stability matrix in sync, and the expected pre-fix RustSec failures for wasmtime 46.0.2 (RUSTSEC-2026-0268 and RUSTSEC-2026-0269).
- Implementation plan: follow the mandated sequence—write and prove the governance oracle red; repair index hygiene; author ADR-064, 062, 061, 060, then 063; derive the provisioning checklist; update stability/build docs; upgrade the coordinated wasmtime family; reconcile the six planning/spec surfaces; finish with the oracle and full gate sweep.
- 2026-09-08 T1 RED: `every_clause_and_denominator_has_a_planted_red` passed all nine isolated vectors (clauses a–f plus workflow, secret, and ADR denominators). The full target failed exactly on the five absent ADRs, four pre-existing index omissions, absent checklist, and seven unrecorded workflow secrets; the complete synthetic fixture remained green.
- 2026-09-08 T2: repaired the index/file bijection by adding ADR-027/042/043/044 in numeric order, fixed ADR-055's missing trailing delimiter, and removed landed ADR-027/028 from the “tracked elsewhere” list. The real-corpus oracle now reports no index findings.
- 2026-09-08 T3: authored ADR-064 with the authoritative/set versus compatibility/unset precedence lattice, typed live/replay conflict, orthogonal strictness, environment stability classes, real-provider predicate, shell wiring obligation, and the five-consumer durability rationale; inserted its row before ADR-065.
- 2026-09-08 T4: authored ADR-062 with one daemon-owned POST trust path, `control.json` discovery, preserved signed-manifest identity writes, post-auth static-body 405 behavior, existing CLI dependency boundary, and both current init arms. The real-corpus oracle reports no ADR-062 finding.
- 2026-09-08 T5: authored ADR-061 around the actual production `cli_wrapper` spawn, treating T3 `--network=none` as the target model rather than the current mechanism. It defines proxy-first ordering, a distinct scoped bearer, durable refusal audit, dependency constraints, and same-commit `env_clear` inversion. The oracle reports no ADR-061 finding.
- 2026-09-08 T6: authored ADR-060 with separate manifest and host taxonomies, an explicit ADR-002/031/040 supersession chain, registry `rust-inproc` refusal assigned to Story 17-3b, preserved `wasm` negative vector, and to-build D-C/D-E decisions grounded in the absent WIT/socket and daemon caller seams. The oracle reports no ADR-060 finding.
- 2026-09-08 T7: authored ADR-063 as a two-axis model (signed Spirit CRL plus certificate reissue), overturned only the no-variant conclusion of RELEASE-HOLDS (c.6), defined `CertPostGraceReject` by the durable peer-scoped rotation/refusal join, and chose client-leaf SAN for listen-side scope. The oracle reports no ADR-063 finding.
- 2026-09-08 T8: created the unprefixed provisioning checklist from the seven live non-`GITHUB_TOKEN` workflow references plus external tree anchors. It records legal per-item states, unset behavior, real owners, the working RTO fallback, broken fuzz jq path, release compile/cache and artifact-merge blockers, and delegates pen-test status to Hold 1. The complete oracle is green.
- 2026-09-08 T9: added the WASM-off/Hold-2 statement inside `STABILITY.md`'s preserved export fence and the explicit `--features wasm-host` self-builder command to `README.md`. `stability-matrix --check --json` reports passed/in-sync, `check-export-control` passes, and the actual CI smoke invocation (`MAOS_ONE_SHOT=smoke-abi-7-5a cargo run -q -p maos-bin`) emitted five JSON steps and exited 0; `smoke-abi-7-5a` is a workflow job/one-shot token, not an xtask subcommand.
- 2026-09-08 T10: ran the corrected coordinated update `cargo update -p wasmtime -p wasmtime-wasi --precise 46.0.3`; 39 packages moved, including the complete wasmtime/wiggle family, with no manifest edit. `cargo deny check advisories` exits 0 (`advisories ok`; unrelated yanked `chacha20 0.10.0` warning remains) and `cargo deny check bans` reports `bans ok`, so no new multiple-version finding exists to record or suppress.
- 2026-09-08 T11: applied all 20 OLD→NEW amendments across the Epic-15 story/table, sprint descriptor, epic index, dependency DAG, PRD, roadmap, and Epic-20/21 consumer contracts. Removed stale first-party rust-inproc, four-ADR, conditional SAN, old replay-citation, and provisioning-as-code-fix claims; filed the fuzz/ADR-040/env-registry defects with their owning stories. The decision/provisioning oracle remains 3/3 green.
- 2026-09-08 T12: full workspace regression passed (499 suites, 4184 passed, 0 failed, 118 ignored). Final gates: decision/provisioning oracle 3/3; `kloc-check` aggregate 157231; stability matrix in sync; env contract 88 registered/0 violations; exit commands 7 epics/7 blocks/31 resolved/6 owed; epic-close coherence 21 epics; `cargo deny check` advisories/bans/licenses/sources all OK with the pre-existing yanked `chacha20 0.10.0` warning; `cargo fmt --all -- --check` clean. `graphify update .` rebuilt 13,529 nodes and 49,159 edges.

### Completion Notes List
- T0 complete: the CI runner prerequisite is committed and executable; all five baseline measurements were captured before implementation.
- T1 complete: added the uncharged integration oracle with denominator-first validation, bidirectional consumer checks, ten source anti-rot anchors, index bijection enforcement, actionable secret inventory failures, and isolated planted-red coverage for every required failure class.
- T2 complete: `docs/adr/index.md` now bijects with the pre-story 44-file ADR corpus and every parsed row is ordered and four-celled.
- T3 complete: ADR-064 records F11–F15 as accepted decisions without claiming nonexistent `--replay` or `--deterministic` compatibility.
- T4 complete: ADR-062 supersedes only the general read-only posture and preserves the two certificate-identity guards required by Story 21-3.
- T5 complete: ADR-061 prevents a standalone `env_clear` regression and assigns the full credential/egress cutover to Story 17-1.
- T6 complete: ADR-060 makes trust tier—not execution syntax—the form discriminator and does not misstate the current validator, registry, WASM recall, or Butler wiring.
- T7 complete: ADR-063 preserves the current non-distinguishability fact while making the observable post-rotation context typed and assigning SAN implementation cost to Story 21-3.
- T8 complete: the checklist is the operator reporting surface; its owning sprint row remains `backlog` until provisioning is complete.
- T9 complete: default builds remain engine-free and opt-in builders are pointed to the live export precondition.
- T10 complete: `Cargo.lock` now carries wasmtime 46.0.3 and the two 46.0.2 RustSec advisories are cleared.
- T11 complete: downstream planning now cites ADR-060–064 as the binding decisions and assigns every implementation or repair to an existing consumer story.
- T12 complete: every story gate and the full locked workspace suite pass after formatting; no acceptance criterion remains open.

### File List
- `Cargo.lock`
- `README.md`
- `STABILITY.md`
- `_bmad-output/implementation-artifacts/15-5-decision-adrs-and-provisioning-checklist.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/13-phased-roadmap.md`
- `_bmad-output/planning-artifacts/epics/dependency-dag.md`
- `_bmad-output/planning-artifacts/epics/epic-15-foundations-w0.md`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md`
- `_bmad-output/planning-artifacts/epics/epic-21-multi-host-on-live-substrate-w6.md`
- `_bmad-output/planning-artifacts/epics/index.md`
- `_bmad-output/planning-artifacts/prd/project-scoping-phased-development.md`
- `docs/adr/ADR-060-spirit-forms-by-trust-tier.md`
- `docs/adr/ADR-061-worker-egress-and-scoped-credential.md`
- `docs/adr/ADR-062-mutating-operator-surface.md`
- `docs/adr/ADR-063-two-axis-revocation-and-post-grace-identity.md`
- `docs/adr/ADR-064-one-inference-mode-selector.md`
- `docs/adr/index.md`
- `docs/runbooks/provisioning-checklist.md`
- `xtask/tests/decision_adrs_and_provisioning.rs`

### Change Log

| Date | Change |
|---|---|
| 2026-09-08 | Implementation complete: authored ADR-060–064 and the workflow-derived provisioning checklist; added the fail-closed decision/provisioning oracle and index repairs; documented the WASM export hold and self-builder path; upgraded the coordinated wasmtime family to 46.0.3; applied all 20 downstream planning amendments; full workspace and gate sweep green. |
| 2026-09-08 | **AC9/F17 CORRECTED BY RUNNING IT.** The ruling (upgrade, not documented exception) stands; **the command was wrong**. `cargo update -p wasmtime` is a measured **no-op** (`Locking 0 packages`), and `--precise 46.0.3` on `wasmtime` alone **fails to resolve** — `wasmtime-wasi 46.0.2` pins `wiggle "=46.0.2"` which pins `wasmtime-environ "=46.0.2"`, against `wasmtime 46.0.3`'s `"=46.0.3"`. Correct form, dry-run verified: **`cargo update -p wasmtime -p wasmtime-wasi --precise 46.0.3`**, moving **39 packages**, still with **no manifest and no `deny.toml` edit**. Also confirmed the runner: `cargo test --workspace --locked --no-fail-fast` **exit 0 — 498 suites, 4181 passed / 0 failed / 118 ignored** (the epic measured `EXIT 101, 4168 passed / 4 FAILED` at `2fd492c0`). |
| 2026-09-08 | **RE-BASED `2fd492c0` → `9f46a2df` — 15-1 landed, and R1's precondition is SATISFIED, verified rather than assumed.** `workspace-test-suite` exists at HEAD (`discipline.yml:3423`/`:3439`, in two `needs` lists), its `worker-cli-fixture` prebuild resolves (`spirits/worker`, `cargo build --locked` exit 0), ADR-066 + its index row are committed, and both previously-red `t_14_2a_post_grace_*` binaries pass. All five exit-line-1 gates exit 0; `kloc-check: PASSED (aggregate=157231)`. **Every number re-measured and now single-valued** (R9 discharged): 44 ADR files / 40 index rows, bijection gap unchanged (027/042/043/044), malformed row still `index.md:40`, insertion point still before `:45`. Thirteen citations across the eight files 15-1 touched re-verified exact. `cargo deny check advisories` still red on `wasmtime 46.0.2` — unchanged, owned by AC9. Also closed: **an `xtask` integration test does not run at the workspace root** — CWD is `xtask/`, so a bare `.github/workflows/*.yml` glob returns empty, which is precisely R6's vacuous pass; the `CARGO_MANIFEST_DIR).parent()` idiom (`coverage_matrix_completeness_tests.rs:7-8`) is now named in Dev notes with the rule that a firing denominator means the path is wrong, never the threshold. |
| 2026-09-08 | **Round-table (R1–R9): the room convicted the story with its own diagnosis — five defects, each one §2's thesis committed by the document that names it.** The control's runner (`workspace-test-suite`) exists only in 15-1's **uncommitted** tree → hard precondition + T0 assertion; AC1(d) was a one-way string match (the 15-3 `summon-dragon` receipt) → **bidirectional**; AC1(c) was a shape check → shape **+ ten anti-rot anchors**; two clauses could **pass on an empty set** → denominators asserted first, with their own planted reds, and (a) asserts existence not exclusivity; AC1(f)'s cross-story red must name its fix; the 405 is post-auth-only (bearer precedes dispatch) but **must not echo the request line**; F10's SAN ruling stands with its **fleet leaf reissue named and routed through 21-3 AC2's own rotation path**; five ADRs confirmed on the principle *"must the decision survive the epic that consumed it"*. Split to 15-5a/b **refused** — all three hard ADRs sit in the back half; Dana's dissent recorded. |
| 2026-09-08 | Story created. Six adversarial scouts measured at `2fd492c0`; 4 ACs → 9; 20 forks ruled. Disproved, among others: the ADR-numbering premise (corpus tops at 065/066, not 059) and its frontmatter template (2 of 44); **the story had no mechanical control at all**; AC3 unimplementable because `STABILITY.md` is generated under a byte-equality gate; three of ADR-060's four premises unbuildable (`cli_wrapper` cannot be a form, `wasm` is refused by the validator, nothing admits by form) plus four incompatible `rust-inproc` dispositions in the corpus; ADR-061's `env_clear` forbidden by name in a green test; **ADR-063's deliverable already ruled against by `RELEASE-HOLDS.md` (c.6)** and given a third decision (§H-10) epic-15 never listed; ADR-064 five-sixths duplicated into 15-6 with no ordering edge; five artifacts disagreeing on four ADRs vs five; and two §2.6 provisioning items disproved by measurement (`rto-ledger` works; `fuzz-ledger` is a jq bug). |
