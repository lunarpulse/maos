---
baseline_commit: "**`323ab597`** (`pre-merge macos rehearsal via label-gated pull_request`), working tree **clean** (`git status --short` empty). ⚠ **The epic file measures at `843d5365`** — its Round-4 refinement (Epic-15 retrospective, 2026-09-12) predates TWO commits, so every citation below was re-derived at `323ab597`, not inherited. Gate sweep at this baseline: **`check-kernel-baseline` PASSED (`maos-kernel-core/src` = 24474 lines, pinned 24474)** · `kloc-check` **PASSED (aggregate=158000)**, `over_budget = []` · `check-decision-register` PASSED (23 rows, 12 open, 0 findings) · `check-epic-close-coherence` PASSED (21 epics, authoritative-src-lines 24474, 0 violations) · `cargo fmt --all -- --check` **EXIT 0, zero output** · `cargo test -p xtask --test d11_xtask_ceiling_ratchet` **7 passed** · `--test recovery_lane_ceiling_rule` **10 passed**. Measured tree: `crates/maos-kernel-core/src` = **97 `.rs` files, 24474 physical lines, 97 tracked == 97 on disk**, plus **one tracked non-`.rs` file** (`security/sandbox/t3-image.lock`). `xtask` = **43797/43797, ZERO headroom** (`xtask/kloc.toml:308`); `xtask/src/check_kernel_baseline.rs` = **83 tokei code lines**. ⚠ **T0 re-measures.** A grant is a global, not a reservation — this block is a snapshot taken on 2026-09-12, not a licence."
depends_on: "**NOTHING. This story opens Epic 16.** It reads no new state, compiles against no new crate, and needs no Epic-15 artifact beyond what is already `done`. It is deliberately first: `epics/epic-16-one-daemon-one-door-j0-w1.md:145` orders **16-0 first**, because the instrument that audits a kernel re-pin must be rebuilt BEFORE the story that moves the pin."
blocks: "**`16-5-audit-drop-legal-hold-and-a2a-deny-vocabulary`** — the ◆ FLAG-Winston story that re-pins `xtask/kernel-core-baseline.toml:481` `src_lines = 24474 → N` and is scheduled LAST in Epic 16. Closing the gap after it would mean re-pinning through the instrument this story exists to replace. It is deliberately **NOT** an AC inside 16-5: *a story must not build the control that audits its own edit* (the Story 13.6e shape, `a control whose owner has already shipped`). Weakly it also blocks every later re-pin in Epics 17–21 (`epic-17` +65–130, `epic-18` conditional, `epic-19` +3–5, `epic-21` pin re-valuation) — each of those is a kernel-core commit that today passes under a bare line count."
spec_alignment: "⚠ **THE EPIC IS AMENDED IN THE SAME COMMIT (rule 9), AND SO IS EPIC 21.** Four adversarial scouts re-derived this story's section against the tree and disproved premises inside its own four ACs. The five that change what gets BUILT: **(1) the epic and the sprint row CONTRADICT each other on granularity** — epic AC1 says `a content hash over that set` (singular), `sprint-status.yaml:241` says `per-file content hash`, and **only the sprint row's version can satisfy AC2's `name security/sandbox/linux.rs`** (§1); **(2) `xtask/kernel-core-baseline.toml` IS NOT VALID TOML** — 7 orphaned prose lines at `:29-35` — so AC1's `gains both beside src_lines` is unimplementable until they are repaired (§2); **(3) AC4 is a NULL CONTROL and its stated rationale is FALSE** — `cargo fmt --all -- --check` exits 0 at HEAD so fmt cannot move anything, and `check-fmt` is a bare CI job with a different subject, not an xtask gate that could `disagree` (§4); **(4) the CI job that runs this gate uses a DEPTH-1 checkout**, so AC2's real-commit falsifier cannot resolve `830c2685` there (§5); **(5) `fs::read_dir` order is nondeterministic** — commutative for the existing SUM, FATAL for a hash (§6). OLD→NEW list in §12. The story does not merely improve on the epic: every improvement is written back. ⚠ **ROUND-TABLE 2026-09-13 (§13, Lunarpulse-ratified: *do as suggested … tag increment allowed*) changed three rulings**: the hashed set stays filesystem-derived but now for a stated THREAT MODEL (the compiler reads the filesystem, git reads the index — a `.gitignore`d `mod backdoor;` is compiled yet invisible to `git ls-files`); the no-hold-constant promise moves from a NAME GREP (theatre — rename the constant and it passes) to an **exhaustive 98-file line-neutral mutation vector** plus a `passed`-is-derived-from-findings invariant; and AC2's falsifier is anchored by an **annotated tag** `kernel-pin-falsifier-15-4` so a future squash or rebase merge of `recovery-lane` cannot delete the commit it proves against."
split_from: "Not a split, and **deliberately not split**. Authored from `epics/epic-16-one-daemon-one-door-j0-w1.md:64-94` (the `### 16-0-kernel-pin-content-hash` section) under Round 4. The story was itself PROMOTED — out of the `20-3` (W5) theme lane into Epic 16's opener — by the Epic-15 retrospective on 2026-09-12, operator-ratified, because *W5 was too late for a pin that 16-5 moves in W1* (`sprint-status.yaml:417-420` — the `- epic: 15` item whose `action:` begins `\"A5: Give `check-kernel-baseline` a file set and a content hash\"`; the literal string `E15-A5` does **not** appear in the file, so grep for the action text, status `in-progress`). ⚠ **SIZING RE-DERIVED, NOT INHERITED.** The epic prices it at `xtask +60–120`. Measurement says the instrument half is the small half: the story also owns a **file that does not parse** (§2), **three hand-rolled parsers it must not break** (§3), **a gate with ZERO tests that cannot be driven from `xtask/tests/` at all** (§7), **thirteen CI jobs downstream of one new red** (§8), and **a ratchet that is a null control for its own funding** (§10). Re-derived cost **+120…+200** raw, ×1.3 (rule 6) = **+156…+260**, and the ledger row is re-booked in this commit (§11). Measured duration **2–4 d**."
kernel_grant: "**NONE, and none is needed. ZERO kernel-Δ @24474, measured green at this baseline.** This story writes `xtask/src/`, `xtask/tests/`, `xtask/kloc.toml`, `xtask/kernel-core-baseline.toml` and `.github/workflows/discipline.yml`. It does not write one byte under `crates/maos-kernel-core/src` — and it must not, because the story's own AC2 pins that tree's per-file digests in the same commit: a kernel edit here would be the story forging its own evidence. ⚠ **`maos-kernel-core` is EXPLICITLY OUTSIDE the eased RECOVERY-LANE CEILING RULE** — `xtask/kloc.toml:52-95` keeps it at ZERO HEADROOM per D13(a), and `xtask/tests/recovery_lane_ceiling_rule.rs:46` `ZERO_HEADROOM_CRATES` + `:266-281` (`RATIFIED_AT_EASING: i64 = 18_935`) assert it against the live file. ⚠ **The PIN itself was also excluded from the easing, and this story is the named reason**: `recovery_lane_ceiling_rule.rs:24` — *Easing the pin would silently widen the very drift that Story `16-0-kernel-pin-content-hash` exists to close.* This story STRENGTHENS the pin; it never relaxes it, and `src_lines = 24474` stays at `xtask/kernel-core-baseline.toml:481` with its value and its line number unmoved (§2 proves the repair is line-count-neutral)."
kloc_grant: "**ONE measured `xtask` raise, taken AFTER the code exists, citing the OPEN key `16-0`.** `xtask` is at **43797/43797, ZERO headroom** (`xtask/kloc.toml:308`), so the FIRST production line reds a blocking gate. The eased RECOVERY-LANE CEILING RULE (architecture §15.5 clause 2(e); `xtask/kloc.toml:52-95`) permits the raise because `xtask` is not in `ZERO_HEADROOM_CRATES` and `16-0-kernel-pin-content-hash: backlog` (`sprint-status.yaml:241`) is a lawful not-`done` citation. ⚠ **FORMAT TRAP, MEASURED — the full sprint key DOES NOT TOKENIZE.** `d11_xtask_ceiling_ratchet.rs:164-179` splits on any char that is neither an ASCII digit nor `-`, then requires EXACTLY TWO all-digit parts: `16-0-kernel-pin-content-hash` yields the token `16-0-` (three parts, third empty) and **matches nothing**. The comment must carry the BARE token `16-0` followed by a non-digit, non-dash character. ⚠ **PUT EVERY TEST IN `xtask/tests/`** — measured free: the kloc invocation (`kloc_check.rs:294-318`) passes `-e tests`, and bucketing its real report shows all 43797 lines are `xtask/src/**` with zero `xtask/tests` files counted. An inline `#[cfg(test)] mod tests { … }` body in the src file IS charged. Aggregate headroom **608 to `_aggregate_alarm` (158608), 12884 to `_aggregate_hardfail` (170884)** — a +156…+260 spend needs no aggregate raise and does not trip the alarm. ⚠ **A GRANT IS A GLOBAL.** Do not cite this story's estimate as a reservation; T0 re-measures and the FORMATTED figure, not the estimate, becomes the ceiling."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs:45-48`) is the machine-checked set; `equiv` is prose, NOT a match token. Keep the literal `allowlist {` (`check_dev_model_used_populated.rs:302`)."
dev_model_used: anthropic/claude-opus-5
review: "§A6 full-layer net (Blind + Edge Case + Acceptance + Test-Infra + non-author runtime). ⚠ **E15-A6 IS BINDING ON THIS STORY BY NAME**: *does this test read the tree, or only what it set itself?* — and this gate is the worst case in the repo, because **it has no test at all today** (§7). The reviewer must: **(a)** confirm AC2's red is produced from the REAL `830c2685` blob via `git show`, not from a checked-in fixture — a fixture is explicitly refused by the epic, because the defect class is *an edit a hand-written fixture would not think to make*; **(b)** delete the `fetch-depth: 0` line added in §5 — and separately delete the local `kernel-pin-falsifier-15-4` tag — and confirm AC2's test goes **RED WITH A NAMED MESSAGE** each time, not green-by-skip; then move the tag to a different commit and confirm it reds on the SHA mismatch — a historical test that fails open is the `check_dev_record_completeness.rs:505` idiom and is the exact null control this epic must not ship; **(c)** confirm every assertion carries a DENOMINATOR (98 entries hashed, exactly 1 file named, 0 unexpected names) — `findings.is_empty()` cannot tell `everything matched` from `the walk returned nothing`; **(d)** re-run the §4 reflow vector and confirm `H0 != H1` as well as `H0 == H2` — the positive half is the one nobody writes, and without it the vector passes on a hash that ignores its input; **(e)** confirm the reflow vector runs in a TEMPDIR and that the real tree is byte-identical before and after `cargo test` — the reflow drops the count 24474 → 24471 and would red three other controls if it escaped; **(f)** confirm the **exhaustive mutation vector** actually runs 98 iterations (denominator asserted) and that inserting an exemption of ANY name — e.g. `if path == \"api.rs\" { continue }` in the comparison — reds it; a name grep cannot see that, which is why the grep is secondary (§13 F-F). §9 shows the prior art for this exact instrument is RED at HEAD behind a hardcoded fingerprint hold and has been for a year of story-time; **(g)** re-run `read_pinned` and the two hand-rolled scanners (`crates/maos-a2a-tcp/tests/t11_t12_chaos_absence.rs`, `xtask/tests/fkcs_oracle.rs`) against the EDITED baseline file and confirm all three still return 24474 (§3 — one of them `.expect()`s and would PANIC); **(h)** confirm `cargo run -p xtask -- kloc-check` is green and that the `xtask` row's comment carries the bare token `16-0`, verified by actually running `d11_xtask_ceiling_ratchet`; **(i)** confirm the three `?` call sites (`check_cohort_mesh.rs:696`, `check_multi_tenant_loom.rs:1889`, `check_epic_close_coherence.rs:124`) still emit their own reports on a new hash red rather than aborting (§8)."
---

# 16-0 — The kernel pin gains a file set and a per-file content hash

Status: **done.** (§A6 review closed 2026-09-13 — 15 patches applied in-commit; ⛔ operator still owes `git push origin refs/tags/kernel-pin-falsifier-15-4` BEFORE the landing commit reaches CI)

> **The capability:** *A change to the kernel that does not move the line count can no longer pass
> the kernel gate. The gate names the file that changed, and it proves it against the commit that
> actually got through.*

**Closes:** drift **D2** (full drift review 2026-09-04, re-homed by the Epic-15 retrospective) ·
retro action **A5** of epic 15 (`sprint-status.yaml:417-420`, `in-progress` → `done`) · the two
`deferred-work.md` rows that live inside this gate (`:637` ownerless `--json` pollution, `:524`
CWD-relative path resolution) · and, as a side effect of §2, the half of **21-4 AC1** that was
going to repair this file in W6.

**Does NOT close:** D11's unreconciled-HISTORY half — *no gate reconciles the kernel pin with its
own HISTORY* — which stays with **`21-4-one-instrument-and-env-registry`**, where
`deferred-work.md:793` already records it with that owner. This story does not build the
reconciler, and AC3 says so out loud.

---

## What this story actually is

The epic's four ACs describe adding two fields to a data file. Measurement says the two fields are
the small half. What the gate actually is at HEAD:

1. **83 lines of code, zero tests, thirteen CI jobs downstream.** `check-kernel-baseline` is not in
   `gate-registry.toml` and has no phase disposition, so nothing makes it advisory — its CI job
   (`discipline.yml:2086-2097`) has no `if:`, no `continue-on-error`, and sits in
   `aggregate.needs` (`:3725`). Fourteen call sites consume it (thirteen rows in §8; `check_epic_close_coherence` is two of them, `:54` and `:124`). **Three of them use `?`** and
   would abort before emitting their own JSON on a new failure class. And it has **no test of any
   kind** — no inline `mod tests`, no file in `xtask/tests/`. §7, §8.
2. **The file it reads does not parse.** `xtask/kernel-core-baseline.toml` fails `tomllib` at line
   29, column 10. Seven lines of a HISTORY entry lost their `#`. AC1 says the file *gains both
   beside `src_lines`* — it cannot, until those seven characters are restored. §2.
3. **Its own doc comment is false.** `check_kernel_baseline.rs:14-16` claims the a2a-tcp guard
   *reads the SAME toml (no second literal)*. There are **four** independent readers of
   `src_lines` and **four** independent line-counting implementations. §3, §7.
4. **The prior art for exactly this mechanism is RED at HEAD**, and has been since Story 13.4,
   held by a hardcoded fingerprint constant. §9.

So this is not "add two fields". It is **the story that makes the kernel pin an instrument that
can be tested at all** — and it has to do that before `16-5` moves the pin through it.

---

## 1. 🔴 THE EPIC AND THE SPRINT ROW CONTRADICT EACH OTHER, AND ONLY ONE OF THEM CAN SATISFY AC2

| artifact | says |
|---|---|
| `epics/epic-16-…md:82` (AC1) | *"(b) a **content hash** over that set"* — **singular** |
| `sprint-status.yaml:241` | *"a pinned FILE SET + **per-file** content hash over the kernel-core `src` set"* |

**AC2 requires the gate to `name security/sandbox/linux.rs`.** A single scalar over N files carries
no per-file information and structurally cannot name anything. The repo already proves this: the
prior art's mismatch message is

```rust
// xtask/src/check_fkcs.rs:563-565
format!("admission-path SHA-256 mismatch: baseline pins {expected}, worktree computes {computed}")
```

— which names no file, and cannot. Measured whole-tree hashes under that framing:

```
830c2685 (parent)  964f8699c877b2a871fbac885a2f327290bc61025a55992195f935a51ef5145e
843d5365 (15-4)    3c964883c353c00ab8d2893572c46877d855f6bd5df42d87e27134bb3f336f42
323ab597 (HEAD)    3c964883c353c00ab8d2893572c46877d855f6bd5df42d87e27134bb3f336f42
```

An aggregate would red — and then say nothing about *what*. That is the shape §9 shows going stale
for a year because nobody could cheaply adjudicate it.

**Ruled: the pinned structure is a per-file map `path → sha256`.** An aggregate `set_hash` is kept
as a cheap derived headline, computed FROM the sorted map, and is never the authoritative value.
The map also gives AC1's *"an added or deleted file is named rather than absorbed"* for free, as a
set difference — which a `files: Vec<String>` + scalar (the `AdmissionBaseline` shape) gives
neither of.

⚠ **And the file set alone cannot carry AC2.** Measured at both sides of the falsifier:
`830c2685` = **98 files (97 `.rs` + `t3-image.lock`)**, `843d5365` = **98**. The SET IS INVARIANT
ACROSS THE FALSIFIER — stated here in the denominator AC2 actually asserts.
Only the per-file hash can red it. The story must not let the file-set half stand in for the proof
— that is *a claim standing in for a control*, the signature failure mode of Epic 12.

---

## 2. 🔴 `kernel-core-baseline.toml` IS NOT VALID TOML, AND AC1 IS UNIMPLEMENTABLE UNTIL IT IS

```
$ python3 -c "import tomllib; tomllib.loads(open('xtask/kernel-core-baseline.toml').read())"
TOMLDecodeError: Expected '=' after a key in a key/value pair (at line 29, column 10)
```

The whole 481-line file has exactly **eight** non-blank, non-`#` lines: `:29-35` and `:481`. Lines
29–35 are the Story 9.3b HISTORY entry that lost its comment markers — and the blocks immediately
above (`:26-28`) and below (`:37`) are correctly `#`-prefixed, so this is a typo, not a convention:

```
:28   # amendment + FLAG-Winston), and you update it HERE — nowhere else.
:29      21472  Story 9.3b — FR62 GovernanceEvent + FR64 CostAttribution kernel delta
:30             (+136, charter-amended kernel delta, jointly re-pinned).
…
:35             AbiExtensionProposal (ADR-045 §8 dogfooding).
:37   #   21894  Story 9.4b AC-5/AC-8 — region-pinning …
```

`check_kernel_baseline.rs:73-78` already documents the consequence — *"The file is NOT valid TOML …
Comment-skipping line parsing is therefore the only correct reader"* — and
`check_epic_close_coherence.rs:41-48` documents the second-order trap (a naive regex over the raw
text returns `23081`, a `fkcs-baseline.toml` value mentioned in comments at `:304,337,386,436`,
not the assignment).

**The repair is seven characters and I ran it:**

```
$ sed '29,35s/^/#/' xtask/kernel-core-baseline.toml > /tmp/repaired.toml
$ python3 -c "import tomllib; print(tomllib.load(open('/tmp/repaired.toml','rb')))"
{'src_lines': 24474}
```

**Ruled: repair it in this story — an ORIGIN fix under rule 10(a).** Three properties make this
safe rather than scope:

- **It is line-count neutral.** Prefixing seven lines with `#` adds no lines, so `src_lines` stays
  at **`:481`** and every `kernel-core-baseline.toml:481` citation in the tree stays valid. (The
  rejected alternative — restructuring the HISTORY block — would move it and invalidate them.)
- **All three existing readers trim-then-skip `#`,** so none of them sees a difference.
- **It is the only way AC1's `gains both beside src_lines` is implementable.** The alternatives
  were considered and rejected: a **sidecar file** splits the "THE single CI-enforced source of
  truth" charter the file asserts at its own `:1-8` and contradicts AC1's wording; **hand-parsing
  97 rows** reinvents a TOML parser inside a gate.

⚠ **Rule-9 consequence: this pre-closes half of `21-4` AC1**, which currently reads *"the baseline
file's HISTORY block is converted to `#`-prefixed lines so `tomllib` parses it (invalid at line 29
today)"* (`epics/epic-21-…md:81`). Epic 21 is amended in this commit (§12). Leaving it there would
mean two stories owning one seven-line repair, five waves apart.

---

## 3. 🔴 A KEY NAMED `src_lines*` HIJACKS THREE HAND-ROLLED PARSERS, AND ONE OF THEM PANICS

The gate's doc comment says the a2a-tcp guard *"reads the SAME toml (no second literal)"*. Measured,
there are **four independent readers of `src_lines`**:

| reader | parse form | collides with a `src_lines`-prefixed key? |
|---|---|---|
| `xtask/src/check_kernel_baseline.rs:86` `read_pinned` (→ `check_epic_close_coherence.rs:124`) | `line.strip_prefix("src_lines")`, **first match wins** | **YES → `Err`** |
| `crates/maos-a2a-tcp/tests/t11_t12_chaos_absence.rs:196-203` | `starts_with("src_lines")` + `.rsplit('=').next()` + `.expect(…)` | **YES → PANIC** |
| `xtask/tests/fkcs_oracle.rs:170` | `strip_prefix("src_lines = ")` (exact) | no, but still first-match-wins |
| `xtask/src/check_epic_close_coherence.rs:54` | delegates to `read_pinned` | safe |

`read_pinned` does not fall through on a near-miss — it strips, fails `parse::<usize>()`, and
returns `Err`:

```rust
// :86-91
if let Some(rest) = line.strip_prefix("src_lines") {
    let val = rest.trim_start().trim_start_matches('=').trim();
    return val.parse::<usize>().map_err(|e| format!("parse src_lines `{val}`: {e}"));
}
```

A key `src_lines_sha256 = "…"` placed anywhere **above** `:481` yields `val = "_sha256 = \"…\""`
and hard-errors the gate — and `t11_t12_chaos_absence.rs` `.expect()`s on the same shape and
aborts the test process. `rsplit('=')` additionally breaks on any `=` inside a value.

⚠ **And `read_pinned` KEEPS its comment-skipping line scan, verbatim.** `toml::from_str` is used
**only** by the new `load_kernel_src` for the `[kernel_src]` table. Making `read_pinned` TOML-based
would look like a tidy-up and would quietly delete this whole section: the first-match-wins hazard
would vanish, D-16-0-C's invariants would become pointless, AC5's parser vector would become a null
control, and `check_kernel_baseline.rs:72-78`'s ruling — *"Comment-skipping line parsing is
therefore the only correct reader, and it is deliberately single-sourced here"* — would be reversed
in silence. Two readers of one file, on purpose, and the doc comment says why.

**Ruled, two invariants, both tested:** new keys are named `kernel_src` / `root` / `set_hash` /
`files` — **no `src_lines` prefix** — **AND** they land strictly **after** `src_lines = 24474`.
Either alone is sufficient; the story takes both, and AC5 pins both directions so a later editor
cannot re-open it.

There are likewise **four independent line-counting implementations**
(`check_kernel_baseline.rs:99`, `check_fkcs.rs:836`, `t11_t12_chaos_absence.rs:216`,
`fkcs_oracle.rs:180`). **This story does not unify them** — `t11_t12_chaos_absence.rs` lives in a
kernel-adjacent crate that must not grow an `xtask` dependency, and `check-service-boundary` is the
control that says so. What it DOES do is stop the gate's doc comment from lying: the "no second
literal" sentence is corrected to name all four readers, because a control whose own doc is false
is the cheapest possible place for the next reader to be misled.

---

## 4. 🔴 AC4 IS A NULL CONTROL AS WRITTEN, ITS RATIONALE IS FALSE, AND HERE IS THE VECTOR THAT RUNS

AC4: *"`cargo fmt --all` over kernel-core must not move the hash — the content hash is taken over
the same normalized bytes the count is, so `check-fmt` (E12-B4) and this gate cannot disagree.
Proven with a reflow vector."* **Three separate claims, and all three are wrong.**

**4a — it is vacuous.** Measured at this baseline:

```
$ cargo fmt --all -- --check
EXIT=0        (zero bytes of output)
```

The tree is already rustfmt's fixed point, so `cargo fmt --all` is a **no-op** and "must not move
the hash" asserts a value it set itself. That is precisely the layer E15-A6 was ratified to catch,
named against three instruments in one epic.

**4b — "the same normalized bytes the count is" describes bytes that do not exist.** The count is
`fs::read_to_string(&path)` then `content.lines().count()` (`check_kernel_baseline.rs:99-113`).
There is **no normalization step anywhere in the gate**. A developer implementing AC4 *literally* —
whitespace-normalizing the hash input — would make the hash **blind to exactly the whitespace-only
edits it exists to catch**. This is the most dangerous sentence in the epic and it is struck.

**4c — "check-fmt and this gate cannot disagree" is not a relation between them.** `check-fmt` is
not an xtask gate: it is a bare CI job (`discipline.yml:140-152`) whose entire body is
`cargo fmt --all -- --check`, over the whole workspace, blocking, and its job comment `:136-139`
records its real origin (*Story 12.2's fmt reflow moved the baseline 23081→23082 and shipped
un-caught into 12.3*). The two gates have different subjects — one asserts the tree is rustfmt's
fixed point, the other asserts the bytes equal a pinned value. A tree can be fmt-clean at any hash.

**The true rationale, which survives contact with the code:** *rustfmt is deterministic and
idempotent, so on a tree held at its fixed point the content hash is a stable function of content.
A reflow can only move the hash by moving bytes, and `check-fmt` is what holds the tree at that
fixed point. The two gates are complementary, not redundant.*

**4d — the vector, designed and ACTUALLY RUN at this baseline:**

```
H0 (pristine tempdir copy)   b2d99870a4de5a7998381797507fb67170b0f2fd186540a0c23c0217d12fc69b   lines 24474
H1 (after reflow)            ff973b88a613b8d3e4c62e0edc66aba41b2beb737deba288bb348aebead8fbfe   lines 24471
H2 (after rustfmt)           b2d99870a4de5a7998381797507fb67170b0f2fd186540a0c23c0217d12fc69b   lines 24474
H0 != H1 ? YES        H0 == H2 ? YES        rustfmt 1.9.0-stable (ac68faa20c 2026-05-25)
```

Independently reproduced on a single file (`orig 070ad3ca…` / `reflow f63e2531…` / `after-fmt
070ad3ca…`, 396 lines throughout) — **and note that middle row is a second, independent
demonstration of the very gap this story closes: the bytes moved and the line count did not.**

Four properties of the vector are load-bearing:

1. **It runs against a TEMPDIR COPY, and asserts `H0 == pinned` first.** That assertion is what
   proves the hash is **re-rootable**, which in turn forces the implementation to stamp
   **root-relative, `/`-separated** paths and take `root: &Path`. `admission_content_hash`
   resolves every path through `resolve_workspace_path` (`check_fkcs.rs:921`) and therefore cannot
   be re-rooted at all —
   which is why it is a pattern to learn from, not a function to call.
2. **The reflowed file must have no `#[rustfmt::skip]`.** There is **exactly one** in the whole
   kernel-core tree — `security/sandbox/linux.rs:25` — and it was introduced by the very commit
   AC2 uses as its falsifier. A vector aimed at `LEGACY_X86_SYSCALLS` would not be restored by
   rustfmt and would fail for the wrong reason. Use `security/sandbox/unsupported.rs`.
3. **`rustfmt --edition 2021 <file>` is enough** — the binary, not `cargo fmt` — so the tempdir
   copy needs no `Cargo.toml`.
4. **The tempdir is MANDATORY, not hygiene.** The reflow drops the count 24474 → 24471; in the real
   tree it would red `check-kernel-baseline`'s own count, red `check-fmt`, and red
   `t11_t12_chaos_absence` for any concurrent job — and a panic between steps 2 and 3 would leave
   the tracked kernel corrupted.

⚠ **CI wiring, or the vector is invisible:** the `check-kernel-baseline` job installs
`toolchain: stable` with **no `components:` line** — no rustfmt (only `discipline.yml:44` and
`:147` install it). And `discipline.yml:424` states the project's own doctrine: *"no job runs
`cargo test -p xtask` unscoped"* — a test file not named in its own step is a test nothing runs.

**4e — and the hash is over RAW BYTES.** Ruled against normalizing, on measurement:
`.gitattributes:17` pins `* text=auto eol=lf` *in the repository AND on checkout, for
EVERY platform*, with a comment naming this exact trap; `check-kernel-baseline` runs
**ubuntu-latest only**; and the tree has zero CRLF files, zero non-UTF-8 files and zero files
missing a trailing newline. Normalizing line endings inside the hash would **silently absorb an EOL
change that `.gitattributes` exists to prevent** — weakening the instrument to fix a hazard already
fixed at a better layer. `fs::read` also removes the UTF-8 error surface `read_to_string` carries.

---

## 5. 🔴 THE CI JOB CANNOT SEE THE FALSIFIER: DEPTH-1 CHECKOUT, AND NO `xtask/tests/` FILE HAS EVER READ GIT

AC2's falsifier is real. Re-derived at this baseline:

```
$ git log --oneline -- crates/maos-kernel-core/src/security/sandbox/linux.rs
843d5365 15-4-release-repair-and-first-signed-tag        ← the most recent, and the falsifier
3d751b4e 5-4-run-spirit-upgrades-and-propagate-signed-revocations
cdf98c84 1b-3-sandbox-tier-t0-t1-t2-enforcement
$ git show --numstat 843d5365 -- crates/maos-kernel-core/src
8	8	crates/maos-kernel-core/src/security/sandbox/linux.rs      ← the ONLY kernel-core file
```

- **SHA** `843d53657f2180b56cb374d4e61875f523da7662`, **parent** `830c26854fc1c2289950e58494e0a365d6f5f3df`
- Gate's own algorithm replayed over the git trees: `830c2685`, `843d5365` and `323ab597` are each
  **98 files (97 `.rs` + `security/sandbox/t3-image.lock`) / 24474 lines**. The lock file is present
  at the parent too, so the pinned SET does not move across the falsifier either
- Pin was `src_lines = 24474` at parent, at the commit **and** at HEAD. 15-1's `+2` was a different
  commit (`9f46a2df`, 2026-09-08, 24472 → 24474), four days earlier — so "PASSED throughout" is exact.
- ⚠ **`crates/maos-kernel-core/src` is byte-identical from `843d5365` to HEAD.** That is what makes
  AC2 cheap: the pin taken at the PARENT, run against the LIVE tree, reds on exactly one file.

**The blocker is the job, not the history.** `discipline.yml` has **159** `actions/checkout` steps
and exactly **four** carry `fetch-depth: 0` — `abi-diff` (`:196`), `invariant-lock` (`:464`),
`check-fkcs` (`:3338`), `check-trial-attestation` (`:3368`). `check-kernel-baseline` is not one:

```yaml
  check-kernel-baseline:            # discipline.yml:2086
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5   # :2089 — no `with:`, so a depth-1 shallow clone
```

A depth-1 clone cannot resolve `830c2685`. **`fetch-depth: 0` lands in the same commit**, with
`check-fkcs` as the shipped precedent. ⚠ `fetch-depth: 5` would "work" today — the commit is only
**2** behind HEAD — and would silently break next week. Depth 0 is the only stable answer.

**And the test must fail LOUD, not open.** There is **zero precedent in `xtask/tests/` for reading
git** — all of it is in `xtask/src/` (`check_fkcs.rs:771` generic `git<const N>` helper,
`:741,:752` `cat-file`/`rev-list`, `:933` `commit_reachable_from_head` via
`git merge-base --is-ancestor`; `check_dev_record_completeness.rs:505,520` `git log --all --grep`
+ `git show --name-only`). 16-0's AC2 test is the **first**. The one precedent it must NOT copy is
`check_dev_record_completeness`'s `let Ok(..) else { return Vec::new() }` — that idiom turns an
unavailable commit into a silent pass, which is the null control this epic exists to remove.

---

## 6. 🔴 `read_dir` ORDER IS NONDETERMINISTIC — COMMUTATIVE FOR A SUM, FATAL FOR A HASH

`count_rs_lines` (`check_kernel_baseline.rs:99-113`) and `fs_walk::collect_rs_files`
(`fs_walk.rs:5-16`) both walk with bare `fs::read_dir` and no sort. Measured raw readdir order at
the kernel-core src root:

```
iac.rs, supervision, lib.rs, api.rs, journal, scheduler, inference, lifecycle, bin, mcp, capability, io, …
```

Not sorted, and a function of the filesystem's directory-hash state — it differs across machines,
across a fresh `git clone`, and after file churn on the same machine. Addition is commutative, so
the existing SUM is unaffected and has been correct for three epics. **SHA-256 is not.**

**Ruled:** the file list is sorted **byte-order (`LC_ALL=C`) on the root-relative path** before
hashing, and the pinned map is stored sorted. The in-repo precedent already carries the rationale:
`check_equiv_fixture_provenance.rs:45-50` sorts *"so the hash is independent of the manifest's list
order and matches the `LC_ALL=C` sort used to mint the pinned values"*.

**And the walker must fail closed.** `fs_walk::collect_rs_files` swallows every IO error
(`if let Ok(entries)`), so an unreadable subdirectory silently yields a shorter file set and a hash
over fewer files. `count_rs_lines` correctly propagates (`map_err(…)?`). **Do not reuse
`fs_walk::collect_rs_files`**, and do not reuse `admission_content_hash` (unsorted + workspace-absolute).

---

## 7. 🔴 THE GATE HAS NO TESTS, IS CWD-BOUND, AND ITS OWN DOC COMMENT IS FALSE

```
$ git ls-tree HEAD xtask/tests/ | wc -l      →  39 entries, NONE for check_kernel_baseline
$ grep -c "mod tests" xtask/src/check_kernel_baseline.rs   →  0
```

A gate that blocks thirteen CI jobs has **no null-control protection at all**. `fkcs_oracle.rs:160-174`
re-implements the parser rather than calling it, which proves nothing about the gate CI runs — the
same point `xtask/tests/decision_register_gate.rs:13-16` makes in its own doc.

**And it cannot be tested as written.** `check()` hardcodes CWD-relative consts:

```rust
const BASELINE_TOML: &str = "xtask/kernel-core-baseline.toml";   // :30
const KERNEL_SRC:   &str = "crates/maos-kernel-core/src";        // :31
pub fn check() -> Result<Report, String> {                       // :61
    let pinned_lines  = read_pinned(Path::new(BASELINE_TOML))?;  // :62
    let actual_lines  = count_rs_lines(Path::new(KERNEL_SRC))?;  // :63
```

`cargo test -p xtask` runs with CWD = `xtask/`, so calling `check()` from an integration test fails
with `read xtask/kernel-core-baseline.toml: No such file or directory`. Every other tree-bound
xtask test already works around this with a `workspace_root()` helper
(`d11_xtask_ceiling_ratchet.rs:26`, `recovery_lane_ceiling_rule.rs:211`,
`retro_action_items.rs:259`). `read_pinned` **is** already path-parameterized (`:79`) and `pub`;
`count_rs_lines` **already takes an arbitrary directory** (`:99`) and is merely private.

**The seam is therefore nearly free** — `pub fn check_at(src: &Path, baseline: &Path)` with
`check()` delegating — and it closes a real open row at the same time:
`deferred-work.md:524` records *"FKCS frozen-tag leg uses CWD-relative kernel-baseline paths …
running the gate from a workspace subdirectory fails with `No such file or directory`"*, against
`check_fkcs.rs:94-106` → `check_kernel_baseline::check()`. That row is closed here, at origin
(rule 10(a)), because the seam AC2 requires is the same seam that row asks for.

---

## 8. 🔴 ONE NEW RED REDS THIRTEEN CI JOBS, AND THREE CALLERS ABORT BEFORE THEY REPORT

`check-kernel-baseline` is **not** in `xtask/gate-registry.toml` (39 `name =` entries, no match), so
it has no phase disposition and `is_blocking_at` / `CURRENT_PHASE` never soften it. Fourteen call
sites, every one CI-blocking (`check_epic_close_coherence` is one row below but two sites, `:54`
and `:124`):

| call site | form | on a NEW `Err` class |
|---|---|---|
| `check_cohort_mesh.rs:696` | `check()?.passed` | **`?` aborts the whole gate before any leg reports** |
| `check_multi_tenant_loom.rs:1889` | `check()?` | **`?` aborts before any leg reports** |
| `check_epic_close_coherence.rs:124` | `read_pinned(..)?` | **`?` aborts the gate** |
| `check_fkcs.rs:97` | `check()?` | `?` propagates (inside `validate_live_triple`) |
| `check_fkcs.rs:651` | `.unwrap_or(false)` | leg RED + `detail` (`:662`) |
| `check_trial_attestation.rs:408` | `.map(..).unwrap_or(false)` | leg RED, **error text swallowed** |
| `check_rotation_real_timing.rs:468` | `.is_ok_and(\|r\| r.passed)` | leg RED, **error swallowed** |
| `check_cross_region_consensus.rs:151`, `check_multi_region_slo.rs:525`, `check_enterprise_identity.rs:329`, `check_enterprise_pdp.rs:320`, `check_escape_detector.rs:450`, `check_scale_churn.rs:411` | `run(false).is_ok()` | leg RED — **and each prints the FAILED message to stderr again** |

None of the twelve dependent jobs (`discipline.yml:2116` `check-epic-close-coherence`, `:2711`,
`:2947`, `:3017`, `:3084`, `:3104`, `:3266`, `:3296`, `:3314`, `:3332` `check-fkcs`, `:3362`,
`:3394`) carries an `if:` or `continue-on-error`, and `check-epic-close-coherence` is itself in
`aggregate.needs` (`:3727`). **A hash mismatch is a correct red in all thirteen** — that is the instrument working. What is NOT acceptable is the three `?` sites turning a
policy red into what looks like a crash, with no JSON emitted. AC5 pins that they still report.

**DISPROVED, and it removes a whole risk class:** nothing parses the `--json` output.
`discipline.yml:2097` is `cargo run -p xtask -- check-kernel-baseline --json` with **no `tee`, no
`jq`, no redirect**; `:3725`/`:3806`/`:3869` read the GitHub **job result**, not JSON. Grepping
`actual-lines|pinned-lines|baseline-file` across the tree returns zero consumers outside Rust field
access. **Adding fields to `Report` breaks nothing.**

That also makes the second ownerless row cheap to close. `deferred-work.md:637` —
*"`check-kernel-baseline` prints its PASSED line to stdout … **Ownerless and open** … Close when
`check-kernel-baseline --json` is machine-clean and both composite callers consume a quiet
structured result without transcript parsing."* The Report is about to get richer and six callers
already re-emit its stderr; making `--json` machine-clean now, while the file is open, is rule
10(b) — *the story that already touches that file is asked first, not last.*

---

## 9. 🔴 THE PRIOR ART FOR THIS EXACT INSTRUMENT IS RED AT HEAD, HELD BY A HARDCODED CONSTANT

`xtask/fkcs-baseline.toml` already carries `[admission_baseline] { files = [...], sha256 = "…" }`,
hashed by `check_fkcs.rs:918 admission_content_hash`. **It is RED, and has been since Story 13.4
(`148a33ee`)**:

```
computed:     9ccc1399bb42568b89885df71dd574f6f2f2552eebcad8523f3ae1a14222bd51
pinned:       dfbbf748707d8891edbbfcbdeacb5a55cf4ed83391cb614febe0f9012d2c6eb2
held-advisory constant: 9ccc1399bb42568b89885df71dd574f6f2f2552eebcad8523f3ae1a14222bd51   ← exact match
```

Held by a fingerprint-bound escape hatch at `check_fkcs.rs:254-284`, re-homed to Story 14-3 by
`14-0` AC5.3. **This is the failure this story must not repeat**: an aggregate content hash over
kernel-adjacent source went stale, nobody could cheaply adjudicate *what* had changed, and it was
parked behind a hardcoded hold for a year of story-time. 16-0 proposes the same class of instrument
over 98 files.

Two structural answers, both in the ACs: the **per-file map** (§1) means a drift is always
adjudicable — the gate names the files — and **AC3 forbids a hold constant outright — and proves the absence by behaviour, not by name**: every one of
the 98 files is mutated in turn and must red. A gate whose red can be silenced by editing a constant
in the gate is not a control, and neither is a grep that a rename defeats (§13 F-F).

What IS worth reusing from the prior art is its *framing discipline*, not its function: length-
prefixed, separator-stamped input so concatenation is unambiguous. The best example in the repo is
`evidence_ledger.rs:336-353 worktree_commit_id` — a domain separator (`b"maos-worktree-v1\0"`) plus
length-prefixed path and length-prefixed content.

---

## 10. 🔴 THE D11 RATCHET IS A NULL CONTROL FOR THIS RAISE, AND THE FULL SPRINT KEY DOES NOT TOKENIZE

The epic states that `d11_xtask_ceiling_ratchet.rs` *"reds any `xtask` ceiling raise that cites no
OPEN sprint story"*. **False as written.** `authorize_xtask_raise` (`:41-85`, with
`Refusal::{MissingStory,UnknownStory,DoneStory,Frozen}`) is called **only from synthetic fixtures**
(`:88-131`) and never sees the real `kloc.toml` row. What binds against the tree is one weaker
assertion (`:195-213`) that:

- checks only that a cited key is **present** in `development_status` — **not that it is open**
  (`DoneStory` is fixture-only); and
- scans **every comment line in the whole 659-line file**, not the `xtask` row's block (the `#`
  branch of `real_xtask_row`, `:142-159`, sits outside the `xtask` branch). Replaying its
  tokenizer against the real file: **24 citations already resolve** (`14-0 … 21-5`). The assertion
  is green no matter what 16-0 writes.

So the ratchet will not stop a bad raise — **and it will not help a good one either.** The real
hazard is the tokenizer (`:164-179`), measured:

```
'Story 16-0 MEASURED GRANT 2026-09-13: 43797 -> 43900'   ->  ['16-0']   ✅
'`16-0`'                                                  ->  ['16-0']   ✅
'16-0, driver: content hash'                              ->  ['16-0']   ✅
'16-0-kernel-pin-content-hash'                            ->  []         ❌ NO MATCH
```

Writing the **full sprint key** yields `16-0-` — three parts, third empty — and matches nothing.
**The comment must carry the bare token `16-0` followed by a non-digit, non-dash character.**

`recovery_lane_ceiling_rule.rs` has the same shape: `authorize_raise` (`:66-103`) is fixture-only;
its tree-bound tests are `the_tree_bound_inputs_exist` (`:240-260`),
`kernel_core_ceiling_has_not_moved_under_the_easing` (`:266-281`, `RATIFIED_AT_EASING = 18_935`)
and `the_lane_is_not_sealed_while_a_numbered_epic_is_open` (`:285-303`). **No conflict between the
two controls** — D11 line-scans the row, recovery-lane reads it via `toml::Table`; neither rejects
the other's format. Both are green at this baseline (7 passed / 10 passed). Six numbered epics
(16–21) are still open, so `Refusal::Frozen` / `Sealed` cannot fire during Epic 16.

⚠ **This story does not repair the ratchet.** That is a second instrument, it is 20-3a's charter
(*gate honesty — correctness of existing gates*), and building it here would make 16-0 the story
that fixes the control funding its own raise. It is FILED to `20-3a-gate-honesty` in
`deferred-work.md` with the tokenizer evidence above, not swallowed. What 16-0 owes is only that
its own comment tokenizes — and AC6 proves that by running the ratchet.

---

## 11. SIZING, BUDGET, AND THE CEILING ROW THIS STORY MUST WRITE

`xtask/src/check_kernel_baseline.rs` is **83 tokei code lines** today. Measured plan:

| change | est. |
|---|---|
| `Report` gains `file-set-entries`, `changed`, `added`, `removed` | +6 |
| `Baseline`/`KernelSrc` structs + serde derives | +12 |
| NEW `load_kernel_src` via `toml::from_str` for the `[kernel_src]` table ONLY; `read_pinned` untouched | +15 |
| sorted, fail-closed recursive walker returning `Vec<(String, Vec<u8>)>` | +25 |
| per-file sha256 map + derived `set_hash` | +15 |
| comparison producing NAMED findings (changed / added / removed) | +35 |
| `--emit-pin` printing the paste-ready toml block | +25 |
| workspace-root resolution (`check_at` + `check()` delegation) | +12 |
| the six `run(false).is_ok()` legs converted to `check()` (the `check_fkcs.rs:95-97` shape) | +8 |
| `main.rs` flag wiring (`:394`, `:1313`) | +6 |
| **raw — itemized point estimate** | **+156** |
| **×1.3 (rule 6)** on the itemized point | **+203** |
| **charged band** — ×1.3 over the raw band **+120…+200** | **+156…+260** |

⚠ **The epic books `xtask +60–120`. That figure is re-booked in this commit** (§12) — the estimate
was made before the file-set half was known to need a sorted fail-closed walker, a re-rootable
seam, an emit-pin surface and named per-file findings.

**The raise, mechanically:**

1. Write the code in `xtask/src/`. **Every test goes in `xtask/tests/`** (measured free: the kloc
   invocation passes `-e tests`, and bucketing its real report shows all 43797 lines are
   `xtask/src/**` with zero `xtask/tests` files counted). Never an inline `#[cfg(test)] mod tests`
   body in the src file.
2. `cargo fmt --all`, **then** `cargo run -p xtask -- kloc-check --json`, and read the `xtask`
   figure. **That figure — not this table — becomes the ceiling.**
3. Edit `xtask/kloc.toml:308` to `xtask = <measured>` with a `#` comment above it carrying the bare
   token **`16-0`**, the arithmetic, and the named driver.
4. Steps 1–3 land in **ONE commit**.

No aggregate raise is needed (**608** to `_aggregate_alarm`, **12884** to `_aggregate_hardfail`).
No `maos-kernel-core` row is touched — it is outside the easing and `recovery_lane_ceiling_rule.rs:268`
pins it at `<= 18935`. TOML is not counted by kloc (`--types Rust`), so
`kernel-core-baseline.toml` growing by ~110 data lines costs nothing.

---

## Decisions ratified in this story (rule 7 — one decision per fork)

| # | Fork | Ruling | Rejected, and why |
|---|---|---|---|
| **D-16-0-A** | Epic AC1 says *a content hash* (singular); `sprint-status.yaml:241` says *per-file* | **PER-FILE MAP `path → sha256`.** An aggregate `set_hash` is kept as a DERIVED headline only. | *One scalar* — cannot satisfy AC2's "name `security/sandbox/linux.rs`"; §1 measures the prior art's message naming no file. *File set alone* — measured invariant across the falsifier (97 files both sides). |
| **D-16-0-B** | The baseline file is not valid TOML (`:29-35`) | **REPAIR IT HERE** — prefix seven lines with `#`. Origin fix, rule 10(a). Verified line-count-neutral, so `src_lines` stays at `:481`. | *Sidecar file* — splits the "THE single CI-enforced source of truth" charter the file asserts at its own `:1-8`, and contradicts AC1's "beside `src_lines`". *Hand-parse 97 rows* — reinvents a TOML parser inside a gate. *Leave it to 21-4* — five waves after `16-5` moves the pin. |
| **D-16-0-C** | New key names and placement | **BOTH invariants: names are `kernel_src` / `root` / `set_hash` / `files` (no `src_lines` prefix) AND the table lands strictly after `src_lines = 24474`.** Both directions pinned by AC5. | *Either one alone* — sufficient in isolation, but §3 shows three hand-rolled first-match-wins parsers, one of which `.expect()`s and PANICS; a later editor must not be able to re-open this by moving a line. |
| **D-16-0-D** | What bytes go into the hasher | **RAW bytes (`fs::read`), framed with a domain separator + length-prefixed path + length-prefixed content, over an `LC_ALL=C`-sorted root-relative path list.** | *Line-normalized* (`lines().join("\n")`) — would silently absorb an EOL change that `.gitattributes:17` already prevents repo-wide, on an ubuntu-only job, weakening the instrument to fix a solved hazard. *Unsorted* — `read_dir` order is machine-dependent (§6). *Reuse `admission_content_hash`* — unsorted, and it resolves through `resolve_workspace_path` (`check_fkcs.rs:921`), so it cannot be re-rooted. |
| **D-16-0-E** | AC4 as written is a null control with a false rationale | **REWRITTEN** around the measured tempdir reflow vector (§4d) and the corrected rationale (§4c). | *Keep it* — `cargo fmt --all -- --check` exits 0 at HEAD, so it asserts a value it set itself. *Implement it literally* — whitespace-normalizing the hash makes it blind to the edits it exists to catch. |
| **D-16-0-F** | Does the set cover `.rs` only, or everything under the root? | **EVERY file under `crates/maos-kernel-core/src`, any extension, no ignore list — 98 entries at this baseline (97 `.rs` + `security/sandbox/t3-image.lock`).** `src_lines` keeps its `.rs`-only denominator; the asymmetry is stated in AC1 and in the file's own HISTORY. | *`.rs` only* (the epic's wording) — leaves `t3-image.lock`, a signed T3 image-attestation pin read at runtime by `security/sandbox/t3/image_lock.rs:18`, as **the one unpinned security artifact inside the pinned tree**. *An ignore list* — dead code today (disk == tracked == 98) and a list that silently grows is the hole. *`git ls-files --cached --others --exclude-standard`* (measured: exactly 98, already sorted; `evidence_ledger.rs:371` idiom) — **REFUSED at round-table (§13 F-A)**: the compiler reads the filesystem and git reads the index, so a `.gitignore`d `mod backdoor;` is compiled into the kernel yet absent from the set. A stray untracked file reds, deliberately, and is **named** — a fifteen-second fix, not a mystery hash. |
| **D-16-0-G** | How is the pin re-taken? | **`--emit-pin` prints the paste-ready block to stdout**; the human commits it. Follows `check_corpus::register_corpus` (`:204-242`, `--register <name>`, `main.rs:275-276,1163-1165`), the repo's only baseline-re-pin precedent. **No in-place rewriter, and NO hold/waiver constant of any kind — proven by the exhaustive 98-file mutation vector and the `passed`-derivation invariant (AC3), with the identifier grep demoted to a secondary tripwire (§13 F-F).** | *In-place `--write`* — there is no such rewriter anywhere in xtask, and a gate that can heal itself is not a gate. *A hold constant* — §9 measures exactly that pattern holding the prior art RED for a year. |
| **D-16-0-H** | AC2's falsifier in CI, and its durability | **`fetch-depth: 0` on the `check-kernel-baseline` job** (`discipline.yml:2089`), precedent `check-fkcs:3338`; **plus an annotated tag `kernel-pin-falsifier-15-4` → `843d53657f21…`**, created and pushed before the landing commit (operator-authorized 2026-09-13, *tag increment allowed*). The test resolves the tag, asserts it is annotated and peels to the pinned SHA, and does **not** require ancestry of `HEAD`. It **fails LOUD** naming `fetch-depth` and the tag push. The name does not match `v*`, so neither `release.yml:4` nor `container.yml:12-14` fires, and nothing in the tree enumerates tags. Defence in depth: `docs/release/tag-procedure.md` gains *merge `recovery-lane` by merge commit — not squash, not rebase*, matching every merge already on `main` (`439b59fd`, `b8caae53`, `7b8ceeeb`, …). | *`fetch-depth: 5`* — works today (2 commits) and breaks silently next week. *`let Ok(..) else { … }`* (`check_dev_record_completeness.rs:505`) — turns an unavailable commit into a silent pass. *A checked-in fixture* — refused by the epic: the defect class is an edit a fixture would not think to make. *A bare SHA with no tag* — `843d5365` is **not on `main`** yet (`recovery-lane` is 9 commits ahead); one squash or rebase merge deletes it and AC2 reds on `main` forever. *Require ancestry (the FKCS shape)* — re-couples the anchor to the merge strategy the tag exists to escape. |
| **D-16-0-I** | Four readers of `src_lines`, four line counters, and a doc comment that denies it | **Do NOT unify. CORRECT the false doc comment** (`check_kernel_baseline.rs:14-16`) to name all four, and FILE the duplication. | *Unify* — `crates/maos-a2a-tcp/tests/t11_t12_chaos_absence.rs` would need an `xtask` dependency in a kernel-adjacent crate; `check-service-boundary` is the control that forbids it. |
| **D-16-0-J** | The D11 ratchet is a null control for this very raise (§10) | **Do NOT repair it here.** File to **`20-3a-gate-honesty`** (charter: correctness of existing gates) with the measured tokenizer evidence. | *Fix it here* — 16-0 would become the story that repairs the control funding its own raise; the 13.6e shape this story exists to avoid. *File to 21-4* — off-charter; rule 10 forbids the default bucket. |
| **D-16-0-K** | Should `cargo run -p xtask -- check-kernel-baseline` join Epic 16's hermetic exit block? | **NO.** The block is the **J0 operator scene** (`maos init` → `maos shell` → halt → resolve → audit → run → pause → purge); an xtask discipline gate is not a scene line. The gate is already blocking on every push (`discipline.yml:2086`, `aggregate.needs:3725`), so nothing is un-enforced. The epic's exit-block provenance names 16-0 as its owner instead. | *Add it* — `check-exit-commands` would accept the `cargo run -p xtask --` shape (`check_exit_commands.rs:1144,1290`), but it would make the epic's exit block a gate list rather than a journey, which is the drift R2 rule 2 separated the operator lane to prevent. |
| **D-16-0-M** | Does 16-0 make 21-4's re-valuation of `src_lines` to tokei units harder? | **No — it makes it possible.** Count and hash are ORTHOGONAL: `src_lines` stays a physical `.rs` line count, `[kernel_src]` is a filesystem-derived set with per-file digests, and neither is defined in terms of the other. Today the count IS the only drift tripwire, so re-denominating it means re-denominating the tripwire; after 16-0 the hash carries drift detection and 21-4 may re-value the count freely (§13 F-C). | *Align the hashed set to tokei's scope* (exclude `memory/spill_test_faults.rs`) — would couple content drift detection to a counting convention and leave a compiled, test-gated kernel file unpinned. |
| **D-16-0-L** | Three `deferred-work.md` rows touch this gate | **ADOPT two, CITE the third.** `:637` (composite `--json` not machine-clean, **Ownerless and open**) and `:524` (CWD-relative paths) are **EFFORT, not scope** — the first is six one-line call-site conversions this story's blast-radius sweep already had to enumerate (⚠ NOT a change to this gate's own output, which is already clean), the second is the seam AC2 requires — so both ship here under rule 10(b). `:793` (no gate reconciles the pin with its HISTORY) stays with **`21-4`**, which already owns it in the file; AC3 cites it rather than re-homing it. | *Defer `:637`/`:524`* — deferral is a last resort and both are inside this story's own blast radius. *Adopt `:793`* — it needs a NEW gate, it is D11's other half, and 21-4 is its recorded owner. |

---

## Acceptance Criteria (7)

**AC1 — (command) the gate reds on a line-neutral kernel-core edit, and names the file.**
`cargo run -p xtask -- check-kernel-baseline` continues to compare `src_lines` (kept, not replaced —
it is the figure every HISTORY row and every story's `kernel_grant` frontmatter cites) **and now also
compares**:
- **(a) a pinned FILE SET** — the exact root-relative, `/`-separated paths under
  `crates/maos-kernel-core/src`, **every file, any extension** (98 at this baseline: 97 `.rs` plus
  `security/sandbox/t3-image.lock`; D-16-0-F), `LC_ALL=C`-sorted. An **added** file and a **deleted**
  file are each **named** in the failure message, not absorbed into a count that happens to balance.
  ⚠ The set is **read from the filesystem, never from `git ls-files`** (§13 F-A): the compiler builds
  what is on disk, and a path listed in `.gitignore` is invisible to `--exclude-standard` while
  `mod backdoor;` still compiles it into the kernel. The set is also **not coupled to `src_lines`'s
  unit** — 21-4 may re-denominate the count to tokei without touching the hash (§13 F-C).
- **(b) a PER-FILE content hash** — `sha256` over the raw bytes of each file, framed with a domain
  separator plus length-prefixed path and length-prefixed content (`evidence_ledger.rs:336-353` is the
  in-repo shape). A **changed** file is **named**. An aggregate `set_hash` over the sorted map is
  emitted as a derived headline and is never the authoritative value.

Both land in `xtask/kernel-core-baseline.toml` **after** `src_lines = 24474` (`:481`, value and line
number unmoved), under keys `[kernel_src]` / `root` / `set_hash` / `[kernel_src.files]` — **no key
beginning with `src_lines`** (D-16-0-C, §3). The file's seven orphaned prose lines at `:29-35` are
comment-repaired in the same commit so the table is readable by `toml::from_str` (D-16-0-B, §2);
the repair is line-count-neutral and is proven so. `check_kernel_baseline.rs:14-16`'s "no second
literal" sentence is corrected to name all four readers (D-16-0-I).
⚠ The denominator is asserted: a run that hashes **0** files is a FAILURE, never a pass.

**AC2 — (proven red, against a real commit — not a fixture).**
The falsifier is **Story 15-4's actual seccomp commit**, `843d53657f21…` (parent
`830c26854fc1…`): 8 insertions / 8 deletions in
`crates/maos-kernel-core/src/security/sandbox/linux.rs`, the only kernel-core file in the commit,
with the count at **24474 on both sides** and the gate reporting `PASSED` throughout.
The commit is anchored by an **annotated tag `kernel-pin-falsifier-15-4`** (D-16-0-H). The test resolves
the tag, asserts `git cat-file -t` is `tag` (annotated — the `check_fkcs.rs:736-750` precedent) and
that it peels to the pinned full SHA `843d53657f2180b56cb374d4e61875f523da7662` (a moved tag reds),
takes the parent's bytes via `git show kernel-pin-falsifier-15-4^:crates/maos-kernel-core/src/security/sandbox/linux.rs`,
substitutes that digest into the pinned map, runs the **real** `check_at` against the **live tree**,
and asserts:
- exit is **non-zero**;
- the message **names `security/sandbox/linux.rs`**;
- it names **exactly one** file — `changed == 1`, `added == 0`, `removed == 0` (a denominator, not a
  `contains`);
- and, as the paired GREEN control, the unmodified map over the same tree **passes**, with
  `file-set-entries == 98`.

`.github/workflows/discipline.yml:2089` gains `with: fetch-depth: 0` in the same commit
(precedent `check-fkcs:3338`; a full-history checkout also fetches tags), and the new test file is
named in its own CI step — `:424` records that no job runs `cargo test -p xtask` unscoped. If the tag
or the commit cannot be resolved the test **fails with a message naming both `fetch-depth: 0` and
`git push origin refs/tags/kernel-pin-falsifier-15-4`**; it must never pass by skipping (D-16-0-H).
⚠ **The test deliberately does NOT require the commit to be an ancestor of `HEAD`** — unlike
`check_fkcs.rs:933 commit_reachable_from_head`. After a squash or rebase merge of `recovery-lane`
into `main`, `843d5365` would stop being an ancestor while the tag keeps it and its parent alive; an
ancestry check would turn a durable anchor back into a merge-strategy bet.
Added-file and deleted-file vectors are proven the same way, in a tempdir copy.

**AC3 — the pin's own HISTORY stays honest, and the gate cannot be silenced.**
The new fields carry the same HISTORY discipline as `src_lines`: a re-pin records the measured
value, the driver and the story key in the same commit. Re-pinning is performed by
**`cargo run -p xtask -- check-kernel-baseline --emit-pin`**, which prints the paste-ready
`[kernel_src]` block to stdout for a human to commit — following `check_corpus::register_corpus`
(`:204-242`), the repo's only baseline-re-pin precedent. **There is no in-place rewriter and no
hold, waiver, or known-mismatch constant of any kind**; §9 measures that exact pattern holding the
FKCS admission hash RED since Story 13.4. ⚠ **And that promise is proven by BEHAVIOUR, not by name** (§13 F-F):
- **Exhaustive line-neutral mutation vector.** In a tempdir copy, for **each of the 98 files** in
  turn: replace the first ASCII alphanumeric byte with a different ASCII alphanumeric byte (UTF-8
  stays valid and no newline moves, so the count is unchanged — every iteration IS the 15-4 defect
  class), run `check_at`, and assert `Ok(Report { passed: false, .. })` with `changed == [that path]`,
  `added == []`, `removed == []`; restore; next. The loop asserts its own denominator — **exactly 98
  iterations** — before its predicate. Any exemption of any name, whether a constant, a
  `continue`, or a `retain`, reds the iteration for the exempted file.
- **Derivation invariant.** For every `Report` the tests produce, `passed == (changed.is_empty() &&
  added.is_empty() && removed.is_empty() && actual_lines == pinned_lines)`. A pass that is not
  computed from the findings reds.
- The `HELD_|ADVISORY|KNOWN_|WAIVER` identifier grep (the `check_fkcs.rs:259-263` shape) is kept only
  as a cheap **secondary** tripwire. It is not the control: renaming the constant defeats it. D11's unreconciled-HISTORY half — *no gate reconciles the
kernel pin with its own HISTORY* — **stays with `21-4-one-instrument-and-env-registry`**, which
already owns it at `deferred-work.md:793`; this story does not build the reconciler and says so.

**AC4 — formatting does not move the hash, proven by a vector that can fail.**
The hash is over **raw bytes**, and the claim is the true one: *rustfmt is deterministic and
idempotent, so on a tree held at its fixed point the content hash is a stable function of content;
a reflow can only move the hash by moving bytes, and `check-fmt` (`discipline.yml:140-152`) is what
holds the tree at that fixed point.* Proven in a **tempdir copy** of
`crates/maos-kernel-core/src`, in three measured steps:
1. `H0` over the pristine copy — assert `H0 == pinned` (this is what proves the hasher is
   **re-rootable**, and therefore that paths are root-relative);
2. reflow one file **with no `#[rustfmt::skip]`** — `security/sandbox/unsupported.rs`; assert
   `H1 != H0` (**the positive half — without it the vector passes on a hasher that ignores its
   input**);
3. `rustfmt --edition 2021 <file>`; assert `H2 == H0 == pinned`.

The vector **must not touch the tracked tree**: the reflow moves the count 24474 → 24471 and would
red `check-kernel-baseline`, `check-fmt` and `t11_t12_chaos_absence`. The job running it installs
`components: rustfmt` (`check-fmt:147` is the precedent; the `check-kernel-baseline` job has no
`components:` line today).

**AC5 — determinism, the parser invariants, and the blast radius.**
- The walk is **`LC_ALL=C`-sorted** on the root-relative path before hashing, and the pinned map is
  stored sorted. Proven by a vector that hashes the same tree through two independently-ordered
  walks and asserts equality. `fs_walk::collect_rs_files` is **not** reused — it is unsorted and
  swallows IO errors (`if let Ok(entries)`); the new walker **propagates** every IO error.
- `check()` gains a path-injectable seam (`pub fn check_at(src: &Path, baseline: &Path)`, with
  `check()` delegating through workspace-root resolution). This closes `deferred-work.md:524`
  (`check_fkcs.rs:94-106` fails from a workspace subdirectory) at origin.
- **The three other readers still work.** Proven: `read_pinned` returns `24474` against the edited
  file; `crates/maos-a2a-tcp/tests/t11_t12_chaos_absence.rs` and `xtask/tests/fkcs_oracle.rs` are
  re-run green; and a vector asserts that **no non-comment line before `src_lines = ` begins with
  `src_lines`**, pinning D-16-0-C in both directions.
- **A mismatch returns `Ok(Report { passed: false, .. })`. `Err` stays reserved for IO and parse
  failure.** This is not a style note: `check_cohort_mesh.rs:696` is `if !check()?.passed { … }`,
  which already reports correctly on `Ok(passed:false)` — **only an `Err` aborts it**. A dev who
  returns `Err("hash mismatch: …")` causes exactly the failure this section warns about while
  satisfying every other word of this AC. Falsifier: plant a mismatch and assert
  `check_cohort_mesh`, `check_multi_tenant_loom` and `check_epic_close_coherence` each still emit
  their own JSON. A hash red correctly reds
  all thirteen downstream jobs — that is the instrument working — but it must look like a policy red.
- **Composite `--json` output is made machine-clean, closing the ownerless
  `deferred-work.md:637`.** ⚠ The obvious fix is not the fix: `check_kernel_baseline.rs:35-53`
  **already** prints the human `PASSED` line only in the `else` branch, so this gate's own `--json`
  is clean at HEAD. The pollution comes from the **six `run(false).is_ok()` legs**
  (`check_cross_region_consensus.rs:151`, `check_multi_region_slo.rs:525`,
  `check_enterprise_identity.rs:329`, `check_enterprise_pdp.rs:320`, `check_escape_detector.rs:450`,
  `check_scale_churn.rs:411`), each of which emits that line **inside its own `--json` payload**.
  They are converted to `check()` — the tree already documents the shape at `check_fkcs.rs:95-97`:
  *"Use `check()` (no stdout) instead of `run(false)` so `--json` stdout stays parseable."*
  **Falsifier:** one composite gate's `--json` stdout parses with `serde_json::from_str` in a test.
  Adding fields to `Report` is safe: nothing parses it (§8).

**AC6 — the raise is measured first, and its citation actually tokenizes.**
The `xtask` ceiling is raised **after** the code exists and **after** `cargo fmt --all`, to the
figure `cargo run -p xtask -- kloc-check --json` reports — not to an estimate. `xtask/kloc.toml:308`
and the code land in **one commit**, with the row's comment carrying the arithmetic, the named
driver, and the **bare token `16-0`** followed by a non-digit, non-dash character. **Proven by
running the ratchet**: `cargo test -p xtask --test d11_xtask_ceiling_ratchet` and
`--test recovery_lane_ceiling_rule` are green after the edit, and a scratch run with the full key
`16-0-kernel-pin-content-hash` in place of `16-0` is confirmed to tokenize to **nothing** (§10).
Every test this story writes lives in `xtask/tests/` (kloc-free); no inline `#[cfg(test)] mod tests`
body is added to any `xtask/src` file. `maos-kernel-core`'s row is not touched.

**AC7 — the tracker and the spec say what the tree says.**
In the same commit: the epic-15 **A5** action row (`sprint-status.yaml:417-420`) flips `in-progress` → `done` with the
control named; the epic's 16-0 section is amended per §12; **epic 21's AC1 drops the
HISTORY-comment-repair clause** (done here); architecture §15.5 clause 1's description of
`check-kernel-baseline` as *"(raw src lines, tripwire)"* is amended to name the file set and content
hash, with clause 2(e)'s "the pin is anti-drift" rationale left intact; `deferred-work.md:637` and
`:524` are closed with their citations; and the D11-ratchet null control (§10) is filed to
**`20-3a-gate-honesty`**. No ADR is minted — `decision_adrs_and_provisioning.rs:19-25`'s
`REQUIRED_ADRS` is a closed list of five and 16-0 is not in it.

---

## Declared cut lines

Filed to `deferred-work.md` in this commit, each with a named owner — never a default bucket (rule 10):

- **The D11 ratchet's tree-bound half is a null control** (§10): `authorize_xtask_raise` is
  fixture-only, the live assertion never checks that a cited story is OPEN, and it scans all 659
  lines rather than the `xtask` row's block (24 citations already resolve). → **Owner:
  `20-3a-gate-honesty`** (charter match: correctness of existing gates). D-16-0-J.
- **Four independent readers of `src_lines` and four independent line counters** (§3, §7), against a
  doc comment that claims "no second literal". The comment is corrected here; the duplication is
  not removed, because `t11_t12_chaos_absence.rs` cannot take an `xtask` dependency. → **Owner:
  `21-4-one-instrument-and-env-registry`**, which already owns the one-instrument charter.
  D-16-0-I.
- **`t11_t12_chaos_absence.rs:199-202` breaks on any `=` inside a value** (`rsplit('=')`). Not
  triggered by this story's keys (they are not `src_lines`-prefixed and land after `:481`), but it
  is a live fragility in a kernel-adjacent guard. → **Owner: `21-4-one-instrument-and-env-registry`**,
  same reader-unification charter.

**Not cut, and explicitly in scope** (EFFORT, not SCOPE — `feedback_pay_effort_early_deferring_drifts`):
the seven-line TOML repair (§2), the `--json` machine-clean fix (§8), the CWD path seam (§7), and
`fetch-depth: 0` (§5). Each is required for one of this story's own ACs to be true.

---

## Dev notes

**Reuse, do not reinvent.** Everything this story needs has a shape in the tree already:
- **Framing** — `xtask/src/evidence_ledger.rs:336-353 worktree_commit_id`: domain separator
  `b"maos-worktree-v1\0"`, length-prefixed path, length-prefixed content. Best in the repo.
- **Sorting + its rationale** — `xtask/src/check_equiv_fixture_provenance.rs:45-50 source_hash`.
- **Re-pin surface** — `xtask/src/check_corpus.rs:204-242 register_corpus` (prints, does not write).
- **Workspace-root resolution** — `xtask/tests/recovery_lane_ceiling_rule.rs:211`,
  `d11_xtask_ceiling_ratchet.rs:26`, `retro_action_items.rs:259`.
- **Git from a gate** — `xtask/src/check_fkcs.rs:771` (`git<const N>` helper with
  `current_dir(workspace_root())`), `:933 commit_reachable_from_head`.
- **`sha2 = "0.10"`, `hex = "0.4"`, `toml = "0.8"`, `walkdir = "2.5"` (`xtask/Cargo.toml:43-46`)
  and `tempfile = "3"` (`:23`, under `[dependencies]` at `:22` — a real dependency, not a dev one)
  are already `xtask` dependencies.** Add nothing.
- **The recursive walker is `walkdir::WalkDir`, not a hand-roll.** It is already a dependency, has
  two in-tree precedents (`xtask/src/check_deprecations_declared.rs`,
  `xtask/src/coverage_matrix_nfr_test_3.rs`), and yields `Result<DirEntry>` — **fail-closed by
  construction**, which is exactly what `fs_walk`'s `if let Ok(entries)` is not. ⚠ Do **not** use
  `sort_by_file_name()`: it sorts per-directory by file name, which is NOT an `LC_ALL=C` sort of the
  full root-relative path. Collect, then sort the `/`-joined root-relative paths.
- **The path-injectable seam already has an in-repo shape** — `check_epic_close_coherence::check_at`
  (`:113-124`) takes all four of its inputs as paths and lets the CLI entry point resolve them. Copy
  that shape rather than inventing one.

**Do NOT reuse:** `check_fkcs.rs:918 admission_content_hash` (unsorted, workspace-absolute paths, so
not re-rootable — and §9 shows what it is doing today), and `xtask/src/fs_walk.rs:5-16
collect_rs_files` (unsorted and error-swallowing).

**Files being modified — current state and what must be preserved:**

| file | today | this story | must not break |
|---|---|---|---|
| `xtask/src/check_kernel_baseline.rs` | 113 lines / 83 tokei code; consts `:30-31`; `check()` `:61-63`; `read_pinned` `:79` (pub, path-parameterized); `count_rs_lines` `:99` (private, dir-parameterized); no tests | file set + per-file hash, `check_at` seam, `--emit-pin`, quiet `--json`, doc-comment repair | `read_pinned`'s **signature** — `check_epic_close_coherence.rs:54,:124` imports it by name and the doc calls it deliberately single-sourced; `src_lines` semantics (raw physical lines, comments and blanks included, `.rs` only) |
| `xtask/src/check_epic_close_coherence.rs` | `fn baseline_file(dir, pin)` `:381-389` MINTS a synthetic baseline containing only one `#` comment + `src_lines = {pin}` — **no `[kernel_src]` table** — and drives `check_at(...)` in ~6 inline tests `:391-521` | not edited | **a missing `[kernel_src]` table must NOT be an error** on `read_pinned`'s path, or all six of those inline tests break. This is the concrete reason D-16-0-C's "keep `read_pinned` a line scan" ruling is not cosmetic |
| `xtask/kernel-core-baseline.toml` | 481 lines; 8 non-comment lines (`:29-35`, `:481`); invalid TOML | `#`-repair `:29-35`; append `[kernel_src]` after `:481` | `src_lines = 24474` **at line 481** — every epic doc and `check-epic-close-coherence` resolve against that value; three hand-rolled readers |
| `xtask/src/main.rs` | `:392-394` command doc + `#[command(name="check-kernel-baseline")] { json }`; dispatch `:1313` | `--emit-pin` flag | existing `--json` behaviour |
| `.github/workflows/discipline.yml` | job `:2086-2097`, checkout `:2089` (depth-1, no `with:`), in `aggregate.needs:3725` | `fetch-depth: 0`, `components: rustfmt`, a named step for the new test file | the job stays blocking and stays in `aggregate.needs`; do NOT add a `gate-registry.toml` row — the gate is not in `EXPECTED_GATES` and adding the name is what creates 15-3's trap |
| `xtask/kloc.toml` | `xtask = 43797` at `:308`; `maos-kernel-core = 18935` at `:247` | `xtask` row only, measured | `maos-kernel-core` untouched (`recovery_lane_ceiling_rule.rs:268` asserts `<= 18935`) |

**Traps measured at this baseline, in the order they will bite:**
1. `cargo test -p xtask` runs with CWD = `xtask/`. `check()` will not resolve its paths from there.
2. `read_pinned` matches `strip_prefix("src_lines")` and **returns `Err` on a near-miss** — it does
   not fall through.
3. `crates/maos-a2a-tcp/tests/t11_t12_chaos_absence.rs:199-202` `.expect()`s — a bad key **panics**
   a kernel-adjacent test, not the gate.
4. `read_dir` order differs across machines and across a fresh clone. ⚠ And do **not** "fix" that by
   switching to `git ls-files` — it is sorted, but it reads the index, not the tree the compiler builds (§13 F-A).
5. The `check-kernel-baseline` CI job is a **depth-1** checkout and has **no rustfmt**.
6. A test file not named in its own CI step is a test nothing runs (`discipline.yml:424`).
7. The `xtask` ceiling comment must carry the bare `16-0`; the full sprint key tokenizes to nothing.
8. ⚠ **Beware a poisoned `target/`.** A scratch copy of the repo built with a shared
   `CARGO_TARGET_DIR` leaves xtask test binaries whose `CARGO_MANIFEST_DIR` points outside the repo;
   tree-bound tests then report every file missing. `touch` the test file before trusting a red.

### Project Structure Notes

⚠ **`xtask` has a `[lib]` target** — `xtask/Cargo.toml:14-16` (`name = "xtask"`, `path = "src/lib.rs"`)
alongside `[[bin]]` at `:10-12`, and `src/lib.rs:14` already exports `pub mod check_kernel_baseline`.
That is what lets `xtask/tests/` call `check_at` **in-process**. Without it a dev would fall back to
shelling out to the binary and re-introduce the exact CWD trap §7 exists to close.

New files: **one** — `xtask/tests/kernel_pin_content_hash_16_0.rs` (kloc-free). No new crate, no new
xtask subcommand (extending the existing one avoids the three-site enrolment: `EXPECTED_GATES` +
`aggregate.needs` + the registry row). ⚠ `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:869`
`SCANNED_SOURCE_FILES` (**19** at HEAD) counts `crates/maos-bin/src` — **this story adds no file
there, so it does not move**. The kernel `src` tree is **read, never written**: a kernel edit in this
commit would be the story forging the evidence it pins.

### References

- [Source: `_bmad-output/planning-artifacts/epics/epic-16-one-daemon-one-door-j0-w1.md#16-0-kernel-pin-content-hash` — lines 55, 64-94, 41, 145]
- [Source: `_bmad-output/implementation-artifacts/epic-15-retro-2026-09-12.md` — §76 (the finding), §92 (action A5), §128 (drift D2 re-homing), §311]
- [Source: `_bmad-output/implementation-artifacts/sprint-status.yaml` — `:241` (the row), `:417-420` (epic-15 action A5)]
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/15-full-spectrum-v2-2.md` — §15.5 clauses 1, 2(e)]
- [Source: `_bmad-output/implementation-artifacts/15-4-release-repair-and-first-signed-tag.md` — `kernel_grant` frontmatter: the seven syscall constants, the `#[rustfmt::skip]` packing, and the 8+/8− admission]
- [Source: `_bmad-output/implementation-artifacts/deferred-work.md` — `:524`, `:637`, `:793`]
- [Source: `_bmad-output/planning-artifacts/epics/epic-21-multi-host-on-live-substrate-w6.md:81` — 21-4 AC1, the HISTORY-comment-repair clause this story pre-closes]

---

## Tasks / Subtasks

- [x] **T0 — re-measure; trust nothing in this file** (AC6)
  - [x] `git status --short` clean; record HEAD. Re-run the seven gates in `baseline_commit` and
        record real output. ⚠ A grant is a global: re-read `xtask/kloc.toml:308` and `:247` at the
        real HEAD, not from this file.
  - [x] Confirm `git rev-parse 843d5365^` = `830c26854fc1c2289950e58494e0a365d6f5f3df` and
        `git show 830c2685:crates/maos-kernel-core/src/security/sandbox/linux.rs | wc -l`.
- [x] **T0a — anchor the falsifier FIRST** (AC2, D-16-0-H; operator-authorized 2026-09-13) — local half done, push OPERATOR-OWED
  - [x] `git tag -a kernel-pin-falsifier-15-4 843d53657f2180b56cb374d4e61875f523da7662 -m "Story 16-0 AC2 falsifier: 15-4's line-neutral kernel-core seccomp edit (drift D2)"`
        — created. `git cat-file -t` = `tag` (annotated); peels to `843d53657f2180b56cb374d4e61875f523da7662`.
  - [ ] ⛔ **BLOCKED — OPERATOR CREDENTIAL.** `git push origin refs/tags/kernel-pin-falsifier-15-4` fails in this
        environment with `git@github.com: Permission denied (publickey)` (exit 128); `git ls-remote --tags origin`
        fails the same way, so the confirmation cannot be run either. The tag EXISTS LOCALLY and all AC2 vectors
        pass against it, but **CI cannot resolve the anchor until the operator pushes it**, and the suite will
        then fail LOUD naming both `fetch-depth: 0` and this exact push command — by design, never green-by-skip.
        ⚠ This is the one item of the story that cannot be completed without operator credentials. Run before the
        landing commit reaches CI.
  - [x] ⚠ The name must NOT match `v*` (`release.yml:4`, `container.yml:12-14`). Re-grep workflows for
        any new tag trigger before pushing.
- [x] **T1 — repair the baseline file** (AC1, D-16-0-B)
  - [x] Prefix `xtask/kernel-core-baseline.toml:29-35` with `#`. Verify with `python3 -c "import
        tomllib; tomllib.load(open(...,'rb'))"` and verify `grep -n '^src_lines' ` still reports
        **481**.
  - [x] Re-run `read_pinned` (via the gate), `cargo test -p maos-a2a-tcp --test
        t11_t12_chaos_absence` and `cargo test -p xtask --test fkcs_oracle` — all green.
- [x] **T2 — the seam and the walker** (AC1, AC5)
  - [x] `pub fn check_at(src: &Path, baseline: &Path) -> Result<Report, String>`; `check()`
        delegates through workspace-root resolution. Keep `read_pinned`'s signature.
  - [x] Sorted (`LC_ALL=C`, root-relative, `/`-separated), **fail-closed** recursive walker over
        **every** file under the root. Do not filter by extension. Do not reuse `fs_walk`.
- [x] **T3 — hash and compare** (AC1, D-16-0-A/D)
  - [x] Per-file `sha256` over raw bytes with domain-separator + length-prefix framing; derived
        `set_hash` over the sorted map.
  - [x] Findings NAME files: `changed` / `added` / `removed`, each a sorted list. Report gains
        `file-set-entries` and the three lists. **Zero entries hashed is a FAILURE.**
- [x] **T4 — pin it** (AC1, AC3)
  - [x] `--emit-pin` prints the `[kernel_src]` block; paste it after `:481` with a HISTORY comment
        recording the measured value, the driver and `16-0`.
  - [x] Key names: `kernel_src` / `root` / `set_hash` / `files`. **No `src_lines` prefix.**
- [x] **T5 — AC2's proven red** (AC2)
  - [x] `xtask/tests/kernel_pin_content_hash_16_0.rs`: GREEN control (live tree passes,
        `file-set-entries == 98`) **before** the red.
  - [x] Resolve `kernel-pin-falsifier-15-4`: `git cat-file -t` == `tag`; peels to the pinned full SHA; **no
        ancestry-of-HEAD requirement**. Parent blob via `git show kernel-pin-falsifier-15-4^:…/linux.rs`; assert
        `Ok(Report{passed:false})`, names `security/sandbox/linux.rs`,
        `changed == 1 && added == 0 && removed == 0`.
  - [x] Unresolvable tag/commit **fails loud**, naming `fetch-depth: 0` AND the tag push. Delete the local
        tag → RED with that message. Re-point the tag at `HEAD` locally → RED on the SHA mismatch.
        Restore it. Add `with: fetch-depth: 0` at
        `discipline.yml:2089`. ⚠ That checkout has **no `with:` block at all**, so this is an
        INSERTION, not a value change; the shipped shape is `check-fkcs` at `:3336-3338`. Then
        **delete it again** and confirm the test goes RED with a message naming `fetch-depth`, not
        green-by-skip.
  - [x] Added-file and deleted-file vectors, in a tempdir copy.
- [x] **T6 — AC4's reflow vector** (AC4)
  - [x] Tempdir copy; `H0 == pinned`; reflow `security/sandbox/unsupported.rs` (**not** `linux.rs` —
        it carries the tree's only `#[rustfmt::skip]`); assert `H1 != H0`; `rustfmt --edition 2021`;
        assert `H2 == H0`.
  - [x] Assert the tracked tree is byte-identical before and after the test run.
  - [x] Add `components: rustfmt` to the job (`check-kernel-baseline` has no `components:` line
        today; `check-fmt:147` is the shape). Then **delete it** and confirm the reflow vector fails
        loud rather than skipping — an absent toolchain component must not be a silent pass.
- [x] **T7 — blast radius and the adopted rows** (AC5, D-16-0-L)
  - [x] Convert the six `run(false).is_ok()` legs to `check()` (`check_fkcs.rs:95-97` shape) —
        **do NOT "move the human line into the non-json branch"; it is already there**
        (`check_kernel_baseline.rs:35-53`). Prove it: `serde_json::from_str` on one composite
        gate's `--json` stdout.
  - [x] Plant a hash mismatch and confirm the three `?` callers each still emit their own report.
  - [x] Parser-invariant vector: no non-comment line before `src_lines = ` begins with `src_lines`.
  - [x] **AC3's exhaustive mutation vector** (tempdir): 98 iterations, denominator asserted first; each
        swaps one ASCII alphanumeric byte (UTF-8 valid, no newline moved) and asserts `changed ==
        [that path]`. Then plant `if path == "api.rs" { continue }` in the comparison locally and
        confirm the vector reds — and that the identifier grep does NOT (that asymmetry is why the
        grep is secondary). Remove the plant.
  - [x] `passed`-derivation invariant over every `Report` the tests produce.
  - [x] Correct `check_kernel_baseline.rs:14-16`'s "no second literal" sentence.
- [x] **T8 — wire it** (AC2, AC6)
  - [x] Name the new test file in its own `discipline.yml` step (`:424`: nothing runs
        `cargo test -p xtask` unscoped). Do **not** add a `gate-registry.toml` row.
  - [x] `cargo fmt --all`; `cargo run -p xtask -- kloc-check --json`; write the MEASURED `xtask`
        figure to `kloc.toml:308` with a comment carrying the bare `16-0`.
  - [x] `cargo test -p xtask --test d11_xtask_ceiling_ratchet --test recovery_lane_ceiling_rule` green.
- [x] **T9 — the tracker and the spec** (AC7)
  - [x] `sprint-status.yaml:417-420` (epic-15 action A5) → `done`. ⚠ The story's own row is set to **`review`**, not
        `done`: the dev-story workflow's step 9 hands a completed story to review, and dev does not self-certify `done`.
  - [x] Epic 16 §12 edits; **epic 21 AC1 clause dropped** and the D-16-0-M orthogonality sentence added;
        architecture §15.5 clause 1 amended.
  - [x] `docs/release/tag-procedure.md`: merge `recovery-lane` → `main` by **merge commit — not squash,
        not rebase** (§12 row 13), naming `kernel-pin-falsifier-15-4` as the reason.
  - [x] `deferred-work.md`: close `:637` and `:524`; file the three cut lines with their owners.
  - [x] Full sweep: `check-kernel-baseline`, `kloc-check`, `check-epic-close-coherence`,
        `check-decision-register`, `check-exit-commands` (⚠ D-16-0-K — the epic's exit block is
        deliberately NOT edited; confirm the gate is green WITHOUT a new token),
        `cargo fmt --all -- --check`,
        `cargo test --workspace --no-fail-fast`.

### Review Findings

_bmad-output §A6 net executed 2026-09-13 (Blind + Edge Case + Acceptance + Test-Infra + non-author runtime; all reviewer obligations (a)–(i) run and PASS). 23 findings raised, 5 dismissed, 3 deferred. All runtime evidence: `git status` byte-identical before/after; tag restored byte-exact (8c90b2e2 → 843d5365).

**Patch findings (unresolved):**

- [x] [Review][Patch] (FIXED in-commit 2026-09-13) **[HIGH] Symlinks (and other non-regular entries) are silently skipped — fail-open hole in the instrument** [xtask/src/check_kernel_baseline.rs:358-360] — `if !entry.file_type().is_file() { continue; }` skips symlink entries (WalkDir with `follow_links(false)` reports them as `symlink`, not `file`). A `ln -s ../evil.rs backdoor.rs` + `mod backdoor;` under the kernel root compiles into the kernel while the gate never pins or names it — exactly the `.gitignore`-backdoor hole D-16-0-F's filesystem ruling claims to close. Fix: `is_dir() → continue`, otherwise `!is_file() → Err` naming the entry (fail closed).
- [x] [Review][Patch] (FIXED in-commit 2026-09-13) **[MED] kloc grant's per-file arithmetic is false** [xtask/kloc.toml:308; sprint-status.yaml:241; story :1025,:1085,:1117] — the comment says "DRIVER, measured per file: `check_kernel_baseline.rs` +216 gross (83 -> 299)"; tokei (CI pin) measures the file at **285 code lines (+202)**. The real +216 = **+202** (gate) **+10** (`check_cohort_mesh.rs`) **+4** (`main.rs`). Ceiling 44013 itself is correct; the attribution is wrong in four records. Fix arithmetic everywhere (keep the bare `16-0` token in the kloc row comment).
- [x] [Review][Patch] (FIXED in-commit 2026-09-13) **[MED] Live epics 19 and 20 still cite the stale `:472` pin line** [epics/epic-19-founder-loop-w4.md:32; epics/epic-20-ship-it-w5.md:45] — the §11-style sweep fixed epic-16/18/21 only. Epic-19:32 carries both a stale value (`@24472`) and the stale cite; epic-20:45 has the right value (`@24474`) with the stale cite. Fix the cites deliberately (the `epic-21:26` value-trap applies); historical/archive `:472` records are true-at-their-time and stay.
- [x] [Review][Patch] (FIXED in-commit 2026-09-13) **[MED] Staged `intent-lineage-coverage-report.md` (+14) is absent from the File List** [story File List; :892-905 of the report] — three appended `## run @untracked` blocks (gate-run output) are staged for the landing commit but neither listed nor disclaimed (the footnote covers only `.memlog.md` and `epics/index.md`). Add a File List row (and optionally dedupe the three identical run blocks).
- [x] [Review][Patch] (FIXED in-commit 2026-09-13) **[MED] `--emit-pin`, AC3's named re-pin surface, is never exercised at CLI level** [xtask/src/check_kernel_baseline.rs:95-98; main.rs dispatch] — all vectors call library `pin_block()`; a flag/dispatch/print wiring regression ships green. Fix: one vector spawning `env!("CARGO_BIN_EXE_xtask") check-kernel-baseline --emit-pin`, asserting stdout parses as the pin block (and that the `--json` combination is refused — next finding).
- [x] [Review][Patch] (FIXED in-commit 2026-09-13) **[MED] AC5's order-independence vector is a null control as written** [xtask/tests/kernel_pin_content_hash_16_0.rs:503-526; check_kernel_baseline.rs:385] — `check_at`/`pin_block` normalize through `BTreeMap`, so `collect_files`' explicit sort is unobservable through every public surface: delete the sort and this vector (and every other) still passes. It also never asserts its premise (two differing raw `read_dir` orders — filesystem-dependent). Fix: assert the observable surface (emitted pin-block lines are byte-sorted) and correct the comments to name `BTreeMap` as the normalizer.
- [x] [Review][Patch] (FIXED in-commit 2026-09-13) **[MED] Six converted legs lost the named-file detail on a kernel red** [xtask/src/check_cross_region_consensus.rs:153; same shape at check_multi_region_slo.rs:525, check_enterprise_identity.rs:329, check_enterprise_pdp.rs:320, check_escape_detector.rs:450, check_scale_churn.rs:411] — `run(false)` used to `eprintln` `failure_detail` (names files); `check().is_ok_and(...)` prints nothing and the leg detail is the constant "kernel baseline re-pin FAILED". Fix: put `failure_detail(&report)` (or the `Err` string) into the red leg's `detail`.
- [x] [Review][Patch] (FIXED in-commit 2026-09-13) **[MED] `?`-propagating composites emit zero stdout on the new `Err` classes** [xtask/src/check_cohort_mesh.rs:702; check_multi_tenant_loom.rs:1889] — runtime-measured: a self-inconsistent baseline (hand-edited map) makes `check()` return `Err`; cohort-mesh then exits 1 with **zero bytes of `--json`** (named stderr only) — the exact `deferred-work.md:637` crash-shape, recurring on the new Err classes. Fix: emit a minimal JSON error object in the `Err` arm when `json`, then propagate (loom needs the same guard).
- [x] [Review][Patch] (FIXED in-commit 2026-09-13) **[LOW] `--emit-pin` silently ignores `--json`** [xtask/src/check_kernel_baseline.rs:96-98] — `run(json, emit_pin)` returns `emit_pin_block()` before the `json` branch: `--emit-pin --json` prints raw TOML to a caller asking for JSON. Fix: refuse the combination with a named error.
- [x] [Review][Patch] (FIXED in-commit 2026-09-13) **[LOW] `pin_block` emits unescaped paths** [xtask/src/check_kernel_baseline.rs:279] — a kernel path containing `"`, `\`, or a control char produces an unpasteable/invalid TOML block that then bricks `load_kernel_src`. Fix: fail closed in `collect_files` on any path outside a safe charset (`[A-Za-z0-9._/-]`), naming the file.
- [x] [Review][Patch] (FIXED in-commit 2026-09-13) **[LOW] `check_kernel_baseline.rs` silently flipped to mode 100755** [index] — `git ls-files -s` shows `100755`; no rationale anywhere. Fix: `chmod 644` + restage.
- [x] [Review][Patch] (FIXED in-commit 2026-09-13) **[LOW] `read_pinned`'s rewritten doc opens with a stale rationale** [xtask/src/check_kernel_baseline.rs:284] — "without pulling a toml dependency" is false at module level since `load_kernel_src` uses `toml`. The real reasons are documented three paragraphs down. Fix: drop/reword the clause.
- [x] [Review][Patch] (FIXED in-commit 2026-09-13) **[LOW] File List labels the story file MODIFIED; the diff adds it as a new file** [story :1101] — `new file mode 100644` at diff :43; record inaccuracy only.
- [x] [Review][Patch] (FIXED in-commit 2026-09-13) **[LOW] Escape-detector composite vector can misattribute an unrelated startup failure** [xtask/tests/kernel_pin_content_hash_16_0.rs:740-765] — `serde_json::from_str` on possibly-empty stdout panics without attribution. Fix: assert non-empty stdout/stderr first with a named message.
- [x] [Review][Patch] (FIXED in-commit 2026-09-13) **[LOW] Three vectors lack full denominators** [xtask/tests/kernel_pin_content_hash_16_0.rs] — deleted-file vector doesn't assert `file_set_entries == 97`; reflow and api.rs vectors rely on indirect emptiness soundness. Fix: add the asserts (review:(c) standard).

**Deferred findings (pre-existing or out-of-scope now):**

- [x] [Review][Defer] **[MED] No permanent regression vector for the three-`?`-caller red-path emission** [xtask/src/check_cohort_mesh.rs:702-714] — deferred, pre-existing: the planted-red falsifier was run manually (dev log + this review's runtime layer) but committing it needs a seam that doesn't mutate the tracked baseline mid-suite; see `deferred-work.md`.
- [x] [Review][Defer] **[LOW] CI wiring (`fetch-depth: 0`, named step, `components: rustfmt`) is unpinned by any local test** [.github/workflows/discipline.yml:2089] — deferred, pre-existing: deletion reds loudly in CI (proven in a faithful depth-1 simulation) but nothing catches it pre-push.
- [x] [Review][Defer] **[LOW] Reflow vector depends on a floating-`stable` rustfmt** [xtask/tests/kernel_pin_content_hash_16_0.rs:391,464] — deferred, pre-existing: a stable-roll rustfmt that formats `unsupported.rs` differently reds the kernel job with a misattributed message; toolchain pinning is an ops decision.

⚠ **Reviewer, read this first.** Per §A6 and E15-A6 (*does this test read the tree, or only what it set itself?*):
the AC2 red is produced from the REAL `830c2685` blob via `git show`, never a checked-in fixture, and the parent's
pin is minted through the gate's own `pin_block` so the test does not re-implement the framing it is testing.
The falsifications the review is owed were all RUN and are recorded in **Dev Agent Record → Debug Log References**:
delete the tag, move the tag, make the tag lightweight, remove `rustfmt` from `PATH`, and plant a renamed
`continue` exemption in the comparison (which reds the 98-file mutation vector while the identifier grep stays
green — the exact asymmetry §13 F-F predicted). ⛔ The tag **push** is still owed by the operator.

---

## 12. Epic OLD→NEW edits (rule 9 — land in this commit)

| # | file | OLD | NEW |
|---|---|---|---|
| 1 | `epic-16…md:80-82` (16-0 AC1) | *"(b) a **content hash** over that set"* | *"(b) a **per-file** content hash over that set, so the failing file is named; an aggregate `set_hash` is derived from the sorted map and is not authoritative"* — resolves the contradiction with `sprint-status.yaml:241` (§1, D-16-0-A) |
| 2 | `epic-16…md:80-81` (16-0 AC1) | *"the exact `.rs` paths"* | *"every file under the root, any extension (98 at `323ab597`: 97 `.rs` + `security/sandbox/t3-image.lock`); `src_lines` keeps its `.rs`-only denominator"* (§1, D-16-0-F) |
| 3 | `epic-16…md:82-83` (16-0 AC1) | *"`xtask/kernel-core-baseline.toml` gains both beside `src_lines = 24474` (`:481`)"* | adds: *"…after first comment-repairing its seven orphaned prose lines at `:29-35`, which make the file invalid TOML at line 29; the repair is line-count-neutral so `:481` is unmoved"* (§2) |
| 4 | `epic-16…md:92-94` (16-0 AC4) | *"`cargo fmt --all` … must not move the hash — the content hash is taken over the same normalized bytes the count is, so `check-fmt` (E12-B4) and this gate cannot disagree"* | the corrected rationale in §4c + the measured three-step tempdir vector in §4d. **The phrase "normalized bytes" is struck**: there is no normalization in the gate, and implementing it literally would blind the hash to whitespace-only edits (§4b) |
| 5 | `epic-16…md:85-88` (16-0 AC2) | *"with the pin taken at its parent, `check-kernel-baseline` must exit non-zero and name `security/sandbox/linux.rs`"* | adds the SHAs (`843d5365` / parent `830c2685`), the denominator (`changed == 1`), and the CI precondition: *"`discipline.yml:2089` is a depth-1 checkout and gains `fetch-depth: 0` in the same commit; the test fails loud, never by skipping"* (§5) |
| 6 | `epic-16…md:55` (Stories table) | *"xtask +60–120 (eased CEILING RULE)"* | *"xtask +156…+260 measured ×1.3 (rule 6); the FORMATTED figure taken after the code exists becomes the ceiling"* (§11) |
| 7 | `epic-16…md:41` (Kernel-Δ block) | *"`kloc.toml:199` keeps `maos-kernel-core = 18935`"* | **`kloc.toml:247`** — stale line cite, independently confirmed by three scouts |
| 8 | `epic-16…md:43` (Kloc asks) | *"xtask **net 0** (the epic's first CI job is a plain discipline job — `cargo test` — not an xtask gate…)"* | *"xtask **+156…+260 (16-0 only)**; 16-1..16-5 remain net 0"* — the `net 0` claim predates 16-0's promotion into this epic |
| 9 | `epic-16…md:141` (16-5 AC4) | *"the measured grant on `xtask/kloc.toml:199` (`maos-kernel-core = 18933`)"* | **`:247` (`maos-kernel-core = 18935`)** — both the line and the value were stale; 15-1 AC2 moved it |
| 10 | `epic-21…md:81` (21-4 AC1) | *"the baseline file's HISTORY block is converted to `#`-prefixed lines so `tomllib` parses it (invalid at line 29 today …)"* | *"…already done by Story 16-0 (Epic 16); 21-4 inherits a parseable file. After 16-0, drift detection is the per-file content hash, so re-valuing `src_lines` to tokei units no longer re-denominates the kernel's only tripwire"* (§2, D-16-0-B, D-16-0-M) |
| 11 | `epic-18…md:32` | cites `kernel-core-baseline.toml:472` | **`:481`** — stale since 15-1's re-pin. ⚠ `epic-21…md:26` was checked and is **NOT** in scope: its `24472` is a pin VALUE already corrected in its own clause, and a literal `:472`→`:481` replace there would turn the pin into `24481` |
| 12 | architecture `15-full-spectrum-v2-2.md:99` clause 1 | *"`check-kernel-baseline` (raw src lines, tripwire)"* | *"`check-kernel-baseline` (raw src lines **plus a pinned file set and per-file content hash**, tripwire)"* — clause 2(e)'s "the pin is anti-drift, the ceilings are anti-growth" rationale is left intact and is strengthened by this story |
| 13 | `docs/release/tag-procedure.md` (pre-tag checklist, before step 5a) | — (no merge-method rule exists) | *"Merge `recovery-lane` into `main` by **merge commit — not squash, not rebase**. Story 16-0's AC2 falsifier `kernel-pin-falsifier-15-4` is anchored by tag so it survives either, but every merge already on `main` is a merge commit and a history rewrite would orphan the commits several stories cite by SHA."* (D-16-0-H) |

---

## 13. Round-table 2026-09-13 — eight forks, three rulings changed

Convened by the operator (*"preflight check for the story, resolve the issues raised in the story
creation for the long term correctness"*); rulings ratified the same session (*"do as suggested …
tag increment allowed"*). Winston, Murat, Amelia, Mary, Paige, John, Sally; Vex and Grumbal sat in.

| Fork | Ruling | What changed in this file |
|---|---|---|
| **F-A** file-set source | **Filesystem, explicitly sorted.** Sally's question (*why read the disk when git knows the files?*) was measured — `git ls-files --cached --others --exclude-standard` returns exactly 98, already sorted — and then **inverted by Vex**: *the compiler reads the filesystem, git reads the index.* A `.gitignore`d `crates/maos-kernel-core/src/backdoor.rs` plus `mod backdoor;` compiles into the kernel and never appears in the set. John's swap-file objection was answered by the per-file map: the gate **names** the stray file. | D-16-0-F rationale; AC1 note; trap 4 |
| **F-B** `t3-image.lock` | **In**, unopposed once named — it is the T3 trust anchor inside the pinned tree. | none (already ruled) |
| **F-C** 21-4 collision | **Orthogonal, and 16-0 unblocks 21-4** (Winston): today the count is the only tripwire, so re-denominating it re-denominates the tripwire; after 16-0 it is a HISTORY figure. | NEW D-16-0-M; AC1 note; §12 row 10 |
| **F-D** 7-line TOML repair into W1 | **Pull it forward**; line-count-neutral, required for AC1. | none (already ruled) |
| **F-E** budget | **Re-book, do not cut**; the formatted measurement is the ceiling, the comment carries bare `16-0`. `walkdir` reuse likely lands it near the bottom of the band. | none (already ruled) |
| **F-F** no-hold promise | **The name grep was theatre** (Grumbal: *rename the constant and it passes*). Replaced as the binding control by an **exhaustive 98-file line-neutral mutation vector** plus a **`passed`-derivation invariant**; grep kept as a secondary tripwire. | AC3; D-16-0-G; §9; review (f); T7 |
| **F-G** 4 readers / 4 counters | **Do not unify**; correct the false doc comment; file to 21-4. The parser invariants already cover the live hazard. | none (already ruled) |
| **F-H** `fetch-depth: 0` + durability | **Yes, and anchor with an annotated tag.** Measured: `843d5365` is **not on `main`**; every merge on `main` so far is a merge commit, but one squash or rebase would orphan it and red AC2 on `main` permanently. Tag `kernel-pin-falsifier-15-4` (operator-authorized), no ancestry requirement, merge-commit rule added to `tag-procedure.md` as defence in depth. Tag name avoids `v*`; nothing in the tree enumerates tags. | AC2; D-16-0-H; NEW T0a; T5; §12 row 13; review (b) |

**Not done in this session, deliberately:** the tag is **not** created or pushed here. It is task
**T0a** of the landing, so it lands with the story that depends on it and its push is visible in the
story's own record.

## Dev Agent Record

### Agent Model Used

anthropic/claude-opus-5

(frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv} — the `model:` frontmatter policy list; on its own line because the extractor skips any line carrying the `allowlist {` boilerplate guard)

### Debug Log References

**Re-measured at the real HEAD `323ab597170e10b835ab9ba8f16e63ef316374b1` — the story's `baseline_commit` was exact.**
`843d5365^` = `830c26854fc1c2289950e58494e0a365d6f5f3df` ✓ · falsifier blob **396 lines at parent, commit AND
HEAD** with `8 8` numstat — line-neutral confirmed from the tree, not inherited · file set **98** (97 `.rs` +
`security/sandbox/t3-image.lock`) ✓ · exactly one `#[rustfmt::skip]`, at `security/sandbox/linux.rs:25` ✓ ·
`xtask/kloc.toml:308` = 43797, `:247` = 18935 ✓ (both re-read at HEAD, not from this file).

**Every control was falsified, not just asserted.** Each of these was RUN and the named red observed:

| falsification | result |
|---|---|
| delete the local tag | RED — message names **both** `fetch-depth: 0` **and** `git push origin refs/tags/kernel-pin-falsifier-15-4`; never green-by-skip |
| re-point the tag at `HEAD` | RED — *"`kernel-pin-falsifier-15-4` has MOVED"* |
| replace it with a **lightweight** tag | RED — *"must be an ANNOTATED tag"* |
| run with `rustfmt` off `PATH` | RED — *"An absent toolchain component MUST NOT be a silent pass"* |
| plant `if path == "api.rs" { continue }` in the comparison | **the asymmetry §13 F-F predicted, measured**: the 98-file mutation vector went **RED** (*"api.rs was mutated and the gate still PASSED"*) while the `HELD_/ADVISORY/KNOWN_/WAIVER` identifier grep stayed **GREEN**. The grep is theatre; the behaviour vector is the control. Plant removed, 20/20 restored. |
| plant a real hash mismatch in the pin | gate RED naming `api.rs`; all three `?` callers still emitted their own JSON |

⚠ **Two defects found by running things, that reading could not have found:**
1. **`check_cohort_mesh.rs:696` produced ZERO bytes of `--json` under a kernel red.** `Ok(passed:false)` does not
   abort at the `?` — but the bare `return Err` on the very next line returned *before* the JSON `println!`, so
   exit 1 with empty stdout. Fixed: it now emits its payload first and forwards the per-file detail (measured
   after: 79 bytes of valid JSON, `oracle_green=false`, stderr naming `api.rs`). Reading the `?` alone would have
   declared this leg fine — AC5's premise pointed one line too high.
2. **AC4's reflow could not be a blank-line collapse.** rustfmt PRESERVES author blank lines (it only caps runs),
   so `replace("\n\n","\n")` is not reversible and the vector failed at `H2 == H0` for the wrong reason. The
   correct reflow joins the multi-line `spawn_sandboxed` signature onto one 107-char line against rustfmt's
   default `max_width = 100` (no `rustfmt.toml` in this repo), which rustfmt re-splits exactly — reproducing the
   story's measured **24474 → 24471 (−3)** precisely.

**`check-exit-commands` is PRE-EXISTING RED and is NOT this story's.** Proven by `git stash -u` to pristine HEAD:
byte-identical finding, same denominators (*1 finding over 6 blocks, 16 tokens resolved* —
`[UnresolvedVerb] epic-20-ship-it-w5.md:17`, `maosctl` does not exist at HEAD). The identical count before and
after is also the positive proof for **D-16-0-K**: the epic's exit block was deliberately not edited and the gate's
verdict did not move.

### Completion Notes List

**All 7 ACs satisfied.** `check-kernel-baseline` is no longer a line count: `src_lines` (kept, unmoved at `:481`)
**plus** a pinned 98-entry file set **plus** a per-file SHA-256 map. 83 → 285 tokei lines at dev
complete (+202); the §A6 review the same day adds +24 more (symlink/non-regular-entry refusal,
non-roundtrippable-path refusal, `--emit-pin`/`--json` mutual refusal) → **309**.

- **AC1** — per-file map `path → sha256` over **raw bytes**, framed `b"maos-kernel-src-file-v1\0"` + length-prefixed
  path + length-prefixed content; **every** file under the root, any extension, read from the **filesystem** (not
  `git ls-files`); `LC_ALL=C`-sorted via `walkdir` which is **fail-closed by construction** (`fs_walk` was not
  reused — its `if let Ok(entries)` swallows IO errors). `changed`/`added`/`removed` are named, sorted lists.
  `set_hash` is derived from the sorted map and **never** authoritative — and a `set_hash` that disagrees with its
  own map is an `Err` (broken instrument), not a policy red. **Zero files hashed is a refusal.** The seven orphaned
  prose lines at `:29-35` are comment-repaired: the file now parses under `tomllib` AND `toml::from_str`, the repair
  is line-count-neutral, and `src_lines` is still at **line 481** (asserted by a test).
- **AC2** — **PROVEN RED off the real `830c2685` blob via `git show`, not a fixture.** The parent's pin is minted
  through the gate's own `pin_block` (the same code path `--emit-pin` uses — no re-implemented framing in the test)
  and the REAL `check_at` is run against the LIVE tree: `passed=false`, `changed == ["security/sandbox/linux.rs"]`,
  `added == []`, `removed == []`, `file_set_entries == 98` — **at `actual_lines == pinned_lines == 24474`**, which
  is the drift the old gate passed. No ancestry-of-HEAD requirement. Added-file and deleted-file vectors proven in
  tempdir copies (the deleted-file vector removes `t3-image.lock`, which an `.rs`-only set would not have noticed).
- **AC3** — no in-place rewriter, no hold/waiver/known-mismatch constant, **proven by behaviour**: all **98** files
  mutated in turn (one ASCII alphanumeric byte → another, so UTF-8 stays valid and no newline moves — every
  iteration IS the 15-4 defect class), denominator asserted **before** the predicate, each asserting
  `changed == [that path]` and restoring cleanly. Plus a `passed`-derivation invariant applied to **every** Report
  any vector produces. The identifier grep is kept as an explicitly secondary tripwire and was measured failing to
  catch a renamed exemption.
- **AC4** — hash is over raw bytes; the epic's *"same normalized bytes"* phrasing is struck in `§12` row 4. Vector
  runs in a tempdir copy: `H0 == pinned` (which is what proves re-rootability), reflow
  `security/sandbox/unsupported.rs` (no `#[rustfmt::skip]`) → `H1 != H0` **and** the count moves 24474 → 24471,
  `rustfmt --edition 2021` → `H2 == H0`. A paired test asserts the tracked tree is still green afterwards.
- **AC5** — determinism proven by hashing two copies built in **opposite directory-creation order** and asserting
  equal hashes. `check_at(src, baseline)` seam added, closing `deferred-work.md:524` at origin; `read_pinned`'s
  signature and line-scan are untouched, so `check_epic_close_coherence`'s six inline tests (whose synthetic
  baseline has **no** `[kernel_src]` table) still pass. All three other readers re-verified green. A mismatch is
  `Ok(Report{passed:false})`; `Err` stays reserved for IO/parse. All six `run(false).is_ok()` legs converted to
  `check()` and **all six measured emitting parseable JSON**, closing the ownerless `deferred-work.md:637`.
- **AC6** — ceiling raised **after** the code existed and **after** `cargo fmt --all`, to the measured figure:
  `xtask 43797 → 44013 (+216)`, inside the story's own +156…+260 band. `maos-kernel-core` untouched at 18935.
  Both ratchets green (7 + 10). §10's tokenizer trap replayed: the shipped comment yields `['15-4','16-0']`, while
  the full key `16-0-kernel-pin-content-hash` yields **`[]`** — confirmed, which is why the bare token is used.
- **AC7** — epic-15 action **A5 flipped `in-progress` → `done`**; epic-16 §12 rows 1–9 applied; epic-21's 21-4 AC1
  HISTORY-repair clause dropped and D-16-0-M orthogonality added; epic-18's stale `:472` → `:481` (exactly one
  occurrence, a line cite — the `epic-21:26` pin VALUE trap avoided); architecture §15.5 clause 1 amended;
  `tag-procedure.md` gains the merge-commit rule (steps renumbered 5→6/6a/6b, 6→7; only self-references existed);
  `deferred-work.md` closes `:524` and `:637` and files the three cut lines to `20-3a-gate-honesty` (×1) and
  `21-4-one-instrument-and-env-registry` (×2). No ADR minted.

**⛔ ONE ITEM IS OPERATOR-OWED AND CANNOT BE DONE HERE.** `git push origin refs/tags/kernel-pin-falsifier-15-4`
fails in this environment with `git@github.com: Permission denied (publickey)` (exit 128). The annotated tag exists
**locally** and every AC2 vector passes against it, but **CI cannot resolve the anchor until it is pushed**. That is
not a silent hazard: the suite fails LOUD naming both `fetch-depth: 0` and that exact push command. **Push before
the landing commit reaches CI.**

**Deliberately NOT done** (each filed, none swallowed): the D11 ratchet is not repaired here (→ `20-3a-gate-honesty`
— repairing the control that funds this story's own raise is the 13.6e shape); the four readers / four line-counters
are not unified (→ `21-4`, because `t11_t12_chaos_absence` cannot take an `xtask` dependency) though the false
*"no second literal"* doc comment IS corrected; D11's unreconciled-HISTORY half stays with `21-4`. No
`gate-registry.toml` row was added, no new subcommand, and **not one byte** was written under
`crates/maos-kernel-core/src` — a kernel edit in this commit would be the story forging the evidence it pins.

### File List

| file | change |
|---|---|
| `xtask/src/check_kernel_baseline.rs` | MODIFIED — file set + per-file hash, `check_at` seam, `pin_block`/`--emit-pin`, `failure_detail`, corrected four-reader doc comment; §A6 review: symlink/non-regular-entry + non-roundtrippable-path refusals, `--emit-pin`/`--json` mutual refusal (83 → 309 tokei: 285 dev + 24 review) |
| `xtask/src/check_cohort_mesh.rs` | MODIFIED — emit `--json` before propagating a kernel red; forward the per-file detail; §A6 review: the `Err` arm emits JSON too (the :637 crash-shape on the new Err classes) |
| `xtask/src/check_cross_region_consensus.rs` | MODIFIED — `run(false).is_ok()` → `check()`; corrected the false stdout comment; §A6 review: red leg detail carries `failure_detail` |
| `xtask/src/check_multi_region_slo.rs` | MODIFIED — `run(false).is_ok()` → `check()`; §A6 review: stderr diagnosis on a kernel red |
| `xtask/src/check_enterprise_identity.rs` | MODIFIED — `run(false).is_ok()` → `check()`; §A6 review: stderr diagnosis on a kernel red |
| `xtask/src/check_enterprise_pdp.rs` | MODIFIED — `run(false).is_ok()` → `check()`; §A6 review: stderr diagnosis on a kernel red |
| `xtask/src/check_escape_detector.rs` | MODIFIED — `run(false).is_ok()` → `check()`; §A6 review: stderr diagnosis on a kernel red |
| `xtask/src/check_scale_churn.rs` | MODIFIED — `run(false).is_ok()` → `check()`; §A6 review: stderr diagnosis on a kernel red |
| `xtask/src/main.rs` | MODIFIED — `--emit-pin` flag + dispatch + command doc |
| `xtask/tests/kernel_pin_content_hash_16_0.rs` | **NEW** — 20 dev vectors + 4 §A6-review vectors (symlink refusal, non-roundtrippable path, CLI `--emit-pin`, byte-sorted emission) = 24 (kloc-free) |
| `xtask/kernel-core-baseline.toml` | MODIFIED — `:29-35` comment-repaired (line-neutral); `[kernel_src]` + 98-entry map appended after `:481` |
| `xtask/kloc.toml` | MODIFIED — `xtask` 43797 → 44096 measured (+216 dev, +83 §A6 review), with the bare `16-0` citation and corrected per-file arithmetic |
| `.github/workflows/discipline.yml` | MODIFIED — `fetch-depth: 0`, `components: rustfmt`, named test step |
| `docs/release/tag-procedure.md` | MODIFIED — merge-commit rule (new step 5); steps renumbered |
| `_bmad-output/implementation-artifacts/sprint-status.yaml` | MODIFIED — story row; epic-15 action A5 → `done`; `last_updated` |
| `_bmad-output/implementation-artifacts/deferred-work.md` | MODIFIED — `:524` and `:637` closed; three cut lines filed with owners |
| `_bmad-output/implementation-artifacts/16-0-kernel-pin-content-hash.md` | **NEW** in this commit (created 2026-09-12, developed and reviewed before first push) — tasks, Dev Agent Record, Review Findings, File List, Change Log, Status |
| `_bmad-output/planning-artifacts/epics/epic-16-one-daemon-one-door-j0-w1.md` | MODIFIED — §12 rows 1–9 |
| `_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md` | MODIFIED — §12 row 11 (`:472` → `:481`) |
| `_bmad-output/planning-artifacts/epics/epic-21-multi-host-on-live-substrate-w6.md` | MODIFIED — §12 row 10 |
| `_bmad-output/implementation-artifacts/intent-lineage-coverage-report.md` | MODIFIED — three `## run @untracked` sections appended by this story's gate runs (disclosed by the §A6 review; content is generated output, not hand-edited) |
| `_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md` | MODIFIED — §A6 review: Kernel-Δ block stale pin cite/value corrected (`:472`/24472 → `:481`/24474; live-epic sweep completion) |
| `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md` | MODIFIED — §A6 review: Kernel-Δ block stale pin cite corrected (`:472` → `:481`) |
| `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/15-full-spectrum-v2-2.md` | MODIFIED — §12 row 12 |

*(`_bmad-output/party-mode/memories/installed/.memlog.md` and `_bmad-output/planning-artifacts/epics/index.md` were
already modified in the working tree before this story started and are NOT this story's changes.)*

### Change Log

| Date | Change |
|---|---|
| 2026-09-12 | Story created from `epic-16…md:64-94` at baseline `323ab597`. Four adversarial scouts; **11 ship-blockers** found, five of them premise-level inside the story's own ACs. 12 rule-9 spec edits filed. |
| 2026-09-12 | **VALIDATION ROUND, same day.** A fresh-context validator re-measured **all 92** `file:line` citations and every measured number. **13 citations were WRONG**, including three of this story's own §12 epic-edit targets — row 9 pointed at 16-4 AC1 (`:130`) instead of 16-5 AC4 (`:141`); row 8 named a blank line and an OLD text that exists nowhere in the epic; row 11 would have rewritten a pin VALUE (`24472`) as a line number in `epic-21…md:26`, turning the pin into `24481`. It also found **four structural defects**: AC5's `--json` remedy was a **null fix** (the human line is already in the `else` branch — the pollution is in the six `run(false)` legs); §11 booked a `read_pinned` rewrite that would have silently deleted §3, D-16-0-C and AC5's own vector; the blast radius was **thirteen** CI jobs, not eleven; and `check_epic_close_coherence.rs:381-389` mints a synthetic baseline in six inline tests that nothing in the story protected. All 25 findings applied. |
| 2026-09-13 | **ROUND-TABLE (§13), operator-ratified.** Eight forks resolved; three rulings changed: F-A rationale (filesystem, because the compiler reads it — `git ls-files` refused on a `.gitignore`d-module threat), F-F control (name grep → exhaustive 98-file line-neutral mutation vector + `passed`-derivation invariant), F-H durability (annotated tag `kernel-pin-falsifier-15-4` + merge-commit rule; no ancestry check). NEW D-16-0-M (count/hash orthogonal; 16-0 unblocks 21-4), NEW task T0a, NEW §12 row 13. All three operator questions closed. Status `ready-for-dev`. |
| 2026-09-13 | **DEV COMPLETE (opus-5). All 7 ACs satisfied; 45 of 46 task boxes checked.** `check-kernel-baseline` rebuilt 83 → 285 tokei lines (+202; the grant's +216 = +202 here +10 `check_cohort_mesh.rs` +4 `main.rs` — arithmetic corrected by the §A6 review, the dev-pass comment had said "83 → 299/+216 gross" for this one file, both false): `src_lines` (kept, `:481` unmoved) + a pinned **98**-entry file set (every file, any extension — 97 `.rs` + `security/sandbox/t3-image.lock`) + a **per-file** SHA-256 map over raw bytes, domain-separator + length-prefix framed, `LC_ALL=C`-sorted, fail-closed `walkdir`; `changed`/`added`/`removed` NAMED; `set_hash` derived-never-authoritative and `Err` if it disagrees with its own map; **0 files hashed is a refusal**. Added `check_at(src, baseline)` (closes `deferred-work.md:524` at origin) and `--emit-pin`/`pin_block`. Baseline TOML comment-repaired at `:29-35` — line-count-neutral, now parses under `tomllib` AND `toml::from_str`. **AC2 proven RED off the REAL `830c2685` blob via `git show` (never a fixture)**, anchored by annotated tag `kernel-pin-falsifier-15-4`: `changed == 1` naming `security/sandbox/linux.rs`, `added == 0`, `removed == 0`, **at 24474 lines on both sides**. 20 vectors in kloc-free `xtask/tests/kernel_pin_content_hash_16_0.rs` (20/20), named in its own CI step with `fetch-depth: 0` + `components: rustfmt`. **Every control falsified, not asserted**: tag deleted / moved / made lightweight, `rustfmt` removed from `PATH`, and a planted `continue` exemption — the last reproducing §13 F-F's predicted **asymmetry exactly** (mutation vector RED, identifier grep GREEN). **TWO defects found only by running things:** (1) `check_cohort_mesh.rs:696` emitted **zero bytes** of `--json` under a kernel red — the `return Err` sat one line *below* the `?` AC5 blamed — now fixed and forwarding the per-file detail; (2) AC4's reflow could not be a blank-line collapse (rustfmt preserves author blank lines), so it joins the `spawn_sandboxed` signature to 107 chars against `max_width = 100`, reproducing the story's measured −3 exactly. Six `run(false).is_ok()` legs → `check()`, all six measured emitting parseable JSON (closes ownerless `deferred-work.md:637`). Ceiling raised AFTER the code and AFTER `cargo fmt --all`: **`xtask` 43797 → 44013 (+216)**, inside the +156…+260 band; `maos-kernel-core` untouched at 18935; both ratchets green (7+10); §10's tokenizer trap replayed — the shipped comment yields `['15-4','16-0']`, the full key yields `[]`. Tracker/spec: epic-15 **A5 → `done`**, epic-16 §12 rows 1–9, epic-21 21-4 AC1 clause dropped + D-16-0-M, epic-18 `:472`→`:481`, architecture §15.5 clause 1, `tag-procedure.md` merge-commit rule, 3 cut lines filed by charter. **ZERO bytes written under `crates/maos-kernel-core/src`.** `check-exit-commands` is **PRE-EXISTING RED** (proven byte-identical at pristine HEAD via `git stash -u`) — and that unchanged verdict is the positive proof for D-16-0-K. ⛔ **ONE OPERATOR-OWED ITEM:** `git push origin refs/tags/kernel-pin-falsifier-15-4` is blocked here by `Permission denied (publickey)`; the tag exists locally, CI cannot resolve it until pushed, and the suite fails LOUD naming that exact command. Status → `review`. |
| 2026-09-13 | **§A6 CODE REVIEW (Review Findings above; all 15 patches applied in-commit).** Blind + Edge Case + Acceptance + Test-Infra + non-author runtime; obligations (a)–(i) all PASS (tag falsifications re-run, reflow vector H0≠H1/H2==H0, 98-file mutation vector, three legacy readers, kloc + both ratchets, three `?`-callers emit their own reports; tree byte-identical before/after). **1 HIGH**: symlinked/non-regular entries were silently skipped by the walk — the fail-open hole D-16-0-F's filesystem ruling claims to close; now refused, named, vector-proven. 14 more patches: corrected kloc arithmetic (ceiling re-measured 44013 → **44096**, +83 review, same `16-0` grant, story still `review`), `--emit-pin` CLI vector + `--json` mutual refusal, byte-sorted-emission vector (the order-independence vector was BTreeMap-normalized — vacuous as written), six legs' named-file diagnosis restored (leg detail / stderr), cohort-mesh + loom Err-arm JSON (the `:637` crash-shape on the new Err classes), non-roundtrippable-path refusal, mode bit 100755 → 100644, epic-19/20 stale pin cites (live-epic sweep completion), File List disclosures (intent-lineage report; story file is NEW). Suite 24/24 after patches; fmt, kloc-check, both ratchets, gate, and three legacy readers green. 3 findings deferred to `deferred-work.md` (§ code review of 16-0, 2026-09-13); 5 dismissed. |

---

## Open questions for the operator

**None open.** All three were closed at the 2026-09-13 round-table (§13): the file-set widening to
`t3-image.lock` is confirmed (F-B), the seven-line TOML repair is pulled into W1 (F-D), and the
`xtask` ledger row is re-booked rather than cut (F-E). The tag push in **T0a** is operator-authorized
and is performed at landing, not before.
