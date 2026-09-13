---
baseline_commit: "**`03ee0ad8`** (\"Retrospectives are refinement sessions (operator directive)\"). ⚠ The epic file measures at **`9f920180`**; `git diff --name-only 9f920180..HEAD` is **21 files, ALL under `_bmad-output/`** — zero code, zero `xtask/`, zero `.github/`. Every HEAD number in the epic's 15-2 table therefore reproduces byte-for-byte at this baseline, and I re-ran the gate to prove it rather than inferring it: `cargo run -p xtask -- kloc-check` EXIT 1, `over_budget = [\"aggregate 155498 >= 147057\", \"maos-domain 8695 > 8644\"]`. Local `tokei 14.0.0` == the CI pin (`discipline.yml:11` `TOKEI_VERSION: \"14.0.0\"`), so these measurements are CI-reproducible, not machine-local. ⚠ **Cite the CEILING RULE by QUOTE, not by line.** This story inserts lines into `xtask/kloc.toml`, and four live citations resolve into it by number — two of them in SHIPPED PRODUCTION SOURCE (`crates/maos-a2a-core/src/router.rs:1250` and `:1271`, both citing `xtask/kloc.toml:87`), plus `kloc.toml:212` (`kloc.toml:86-87`) and `kloc.toml:195` (`kloc.toml:61`). AC2's placement rule exists to keep all four valid."
depends_on: "**NOTHING.** This story opens Epic 15 and the whole recovery lane. It edits one data file, one prose block, two `eprintln!` strings and one decision register. It compiles no new logic and reads no new state."
blocks: "**Every story in Epics 15–21 that writes a line.** **Eight** crates sit at EXACTLY `loc == budget` at HEAD — `maos-a2a-core` 4856, `maos-audit` 6847, `maos-bin` 17109, `maos-cli` 5270, `maos-cohort` 5857, `maos-iac` 6960, `maos-kernel-core` 18933, `xtask` 41953 (derived by joining `kloc-check --json` `per_crate` against the `kloc.toml` keys; rule 8 — the count carries its derivation) — so the next production line written in any of them reds a BLOCKING gate. Named in the epic's own dependency line: 15-2 before 15-1, 15-3 and 15-6. **And the aggregate is red at HEAD regardless of any crate**, so `check-empty-kernel`-style work cannot reach green while `kloc-check` is in `aggregate.needs` (`discipline.yml:3516`)."
spec_alignment: "✅ **THE EPIC SAYS WHAT THIS STORY DOES — amended 2026-09-06, same session.** Operator directive: *'no drift in spec for this project; we do as per the spec.'* This story's round-tables improved on `epics/epic-15-foundations-w0.md:63-103` in **six** places, and leaving the epic stale would have reproduced the exact `rescout-r3` failure this story documents (§6: fixes filed and never applied). Confidence **rule 9** provides the vehicle — *'a refinement round may also be convened mid-epic when a story disproves a premise'* — so the epic's 15-2 section was amended in the same pass: AC1's D-F sentence (+250 → standing permission), AC2's aggregate rule (formula → 170884) and its rule-6/phantom-retirement clauses, AC5's ratification, the entire AC1 grant table (now `⌈Σupper × 1.3⌉`, 26 rows, with a provenance block), and **NEW AC6/AC7** so the epic carries 7 ACs exactly as this story does. The epic header and its Stories row record the refinement round. **Do not edit one without the other.**"
split_from: "Not a split. Authored from `epics/epic-15-foundations-w0.md:63-103` (the `### 15-2-kloc-ceiling-rebase` section) under Round 3 (`sprint-change-proposal-2026-09-05-round3.md`), which itself folded the R2 preflight's §K edits. Four adversarial scouts re-derived the whole table against the tree; §1–§7 below are what they DISPROVED."
kernel_grant: "**NONE and none needed. ZERO kernel-Δ, CONFIRMED by construction, not by assertion.** Nothing here touches `crates/maos-kernel-core/src`; the only files written are `xtask/kloc.toml` (data + comments), `xtask/src/kloc_check.rs` (two message strings, AC6), the architecture §15.5 clause-2 text and `epics/epic-14-preflight-decisions.md`. `check-kernel-baseline` is GREEN at HEAD — `actual == pinned == 24472` (`xtask/kernel-core-baseline.toml:472`; equality at `check_kernel_baseline.rs:65`, the doc comment 'Hard-fails on ANY drift' is at `:7-8`) — and this story cannot move it. ⚠⚠ **DO NOT CONFLATE THE TWO INSTRUMENTS.** The PIN counts **raw physical lines** (`content.lines().count()`, `check_kernel_baseline.rs:99-110`) over every `.rs` under `maos-kernel-core/src`, blanks/comments/in-`src` tests included → **24472**. The kloc CEILING counts **tokei `code`** with `target tests benches examples fuzz spirits` and `spill_test_faults.rs` excluded (`kloc_check.rs:173-190`) → **18933**. Difference **5539**. The architecture is explicit that they are 'each cited only in its own units, never compared' (`architecture-maos-minimal-opus/15-full-spectrum-v2-2.md:93-95`). AC5 restates the pin protocol; it never edits it."
kloc_grant: "✅ **FUNDING RATIFIED 2026-09-06 (Lunarpulse): the kloc grant increment is APPROVED as the spec needs** — the `⌈Σupper × 1.3⌉` re-derivation (rule 6, F1c), the `170884` aggregate (F1), the 7000-line phantom retirement (F1b), and `maos-kernel-core` **UNCHANGED at 18933/18933** with D-F recorded as a standing permission (§8). Operator directive issued in the same breath and binding on this story: **\"develop as close as the specs saying, try not to defer; if it is about some more effort, we pay it early as planned — deferring drifts.\"** Applied in F14: every deferral in this file was re-classified EFFORT vs SCOPE, and the six EFFORT items were pulled back in. **This story IS the grant.** It consumes nothing and authorizes everything below it. Its own footprint: `xtask/kloc.toml` is DATA (`kloc_check.rs` measures `.rs` only — a TOML edit is 0 lines to every ceiling), and AC6's two `eprintln!` string edits cost **net ≤ +2 `xtask` tokei lines**, drawn from this story's OWN `xtask` row and stated in the commit. ⚠ **A GRANT IS A GLOBAL, NOT A RESERVATION.** The 'Asks by epic' column in AC1 is a *ledger of intent*, not a set of reservations — whichever story measures first consumes the headroom. §5 below is what happens when that is forgotten: the epic's `maos-cohort` row reserves against a '14-2d T8 measured grant' **that was never asked for and does not exist**. ⚠⚠ **ROUND-TABLE 2026-09-06 turned this thesis on the story itself and it failed:** D-F's `+250` pre-authorization for five future FLAG-Winston stories **was a reservation** — the exact move this field forbids, in the one crate where the project spent three epics building a per-story discipline to prevent it. `maos-kernel-core` is therefore **UNCHANGED at 18933/18933, zero headroom**, and D-F is recorded as a *standing permission* (raises are allowed; the Epic-13 blanket refusal is overruled) rather than as capacity. Its 202-line forecast is funded in the aggregate and pre-granted nowhere. ⚠ Re-basing to `measured + grant` **destroys the existing headroom** on nine rows (a `+300` grant on a crate with 100 spare yields 300 total, not 400) — AC1's table is stated in FINAL CEILINGS so the reading cannot be ambiguous."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs:45-48`) is the machine-checked set; **`equiv` is prose, NOT a match token**. Keep the literal `allowlist {` (`check_dev_model_used_populated.rs:302`)."
review: "§A6 full-layer net (Blind + Edge Case + Acceptance + Test-Infra + non-author runtime), **non-degradable** — the epic marks this story ◆. **Test-Infra is load-bearing and this story proves why: every unit test that touches the ceilings is a null control.** `xtask/src/tests/kloc_check_tests.rs`: `kloc_check_runs_on_workspace` (`:29-36`) is `#[ignore]`d since Epic 5 — ⚠ **and my round-2 claim that no re-base could un-ignore it was WRONG**: it asserts `!alarm`, and this story re-bases `_aggregate_alarm` 16000 → 158608, so `155498 >= 158608` is false and the test passes. AC6(f) un-ignores it. The other four stay null controls: `kloc_check_produces_report_on_workspace` (`:41-54`) loads the real file but asserts only that the map contains a key; `alarm_fires_at_threshold`/`hardfail_at_aggregate_threshold`/`hardfail_at_per_crate_threshold` (`:56-79`) compare LOCAL LITERALS and never read the config at all (`assert!(20001 >= 20000)`). **A wrong number in `kloc.toml` reds NO test.** The only oracle is running the gate. The reviewer must re-run `cargo run -p xtask -- kloc-check --json` and re-derive the aggregate arithmetic by hand."
---

# 15-2 — One-time kloc ceiling re-base and CEILING RULE amendment

Status: **done.**

> **The capability:** *Epics 16–21 can write the line their measured stories need without
> convening a governance event first — because the ceilings were re-based once, in the open,
> against a measurement anyone can re-run, with the operator's authorization recorded in the
> file it authorizes.*

**Closes:** the epic's §2.2, decision **D-F**, decision **D14** (`maos-domain` −51) and decision
**D17** (`_aggregate_hardfail` −8441). Opens Epic 15 and the recovery lane.

---

## What this story actually is

`kloc-check` is **BLOCKING** (`discipline.yml:154`, no `continue-on-error`, no `if:`; in
`aggregate.needs` at `:3516`; registered `gate-registry.toml:8`; `docs/ci-baselines/README.md:83`
`required`). It is **RED at HEAD in two places at once**, and one of them is not attributable to
any crate:

```
$ cargo run -p xtask -- kloc-check          # EXIT 1, re-run at 03ee0ad8
over_budget = ["aggregate 155498 >= 147057", "maos-domain 8695 > 8644"]
```

**Eight** crates sit at **exactly** `loc == budget` — `maos-a2a-core` 4856, `maos-audit` 6847,
`maos-bin` 17109, `maos-cli` 5270, `maos-cohort` 5857, `maos-iac` 6960, `maos-kernel-core` 18933,
`xtask` 41953 (all eight, derived by joining `--json` `per_crate` against the `kloc.toml` keys —
rule 8). That is legal — the per-crate comparison is `>` (`kloc_check.rs:231`) — and it
means **the next production line written in any of them reds a blocking gate**. The aggregate
comparison is `>=` (`kloc_check.rs:225`), so the aggregate reds one line *earlier* than a crate
does. That asymmetry decides AC3's number and is not a rounding detail.

This is the state Epics 16–21 would open into. The re-base is the story that ends it.

---

## 1. 🔴 THE MACHINE READS TWO INTEGERS AND A MAP. EVERYTHING ELSE IN THIS FILE IS SOCIAL CONVENTION.

Measured, not assumed. `grep -rn "MEASURED GRANT\|CEILING RULE\|aggregate_hardfail" xtask/ .github/
--include=*.rs --include=*.yml --include=*.toml` → **19 hits**. Every `MEASURED GRANT` hit (12) and
the single `CEILING RULE` hit are `#` comments **inside `kloc.toml` itself**. The only executable
uses are `kloc_check.rs:138,:139,:225,:226`. `.github/` returns **zero**.

| Claim | Verdict |
|---|---|
| "No xtask parses grant comments" (epic AC3) | ✅ **CONFIRMED** |
| The CEILING RULE formula is enforced | ❌ **FALSE** — prose, no reader |
| "Recalculated ONLY at an epic retrospective" is enforced | ❌ **FALSE** — prose, no reader |
| `check_epic_close_coherence.rs:38` "keeps `prior:` verbatim" (epic AC3) | ❌ **FALSE** — `:38` is a doc-comment **analogy**; that gate reads `sprint-status.yaml`, `epics/index.md`, `epics/` and `kernel-core-baseline.toml` (`:57-60`) and **never opens `kloc.toml`**. Nothing enforces `prior:`. |
| The `prior:` chain is a guarantee | ❌ **FALSE** — it is a courtesy, and it is the only history that exists |

**Consequence for the dev pass, stated plainly:** the amendment AC4 writes is a **promise to
humans**, and it must be written as one. Do not imply a control. The mechanical constraints are
exactly four, and they are in §2.

---

## 2. 🔴 FOUR MECHANICAL TRAPS. EACH ONE SHIPS GREEN AND SILENT.

Every one verified by experiment against a synthetic workspace, not read off the source.

**(a) THE PLACEMENT TRAP — the ship-blocker.** `xtask/kloc.toml:512` is
`[in_progress_decomposition]`. Iteration at `kloc_check.rs:144` walks the **document root only**.
Any row appended after `:512` becomes a sub-key of that table and is **silently ignored — no
budget, no warning, no error**. AC2 adds two new rows. Appending them at end-of-file is the
natural move and it produces a ceiling that can never fire.
⇒ **Every new row goes ABOVE `:512`.** The natural slot is the Story-13.6d alphabetical block at
`:464-498` (header `:461-463`), or beside `_aggregate_hardfail` at `:501`.

**(b) A QUOTED INTEGER IS SILENTLY DROPPED.** `kloc_check.rs:148-150` takes `as_integer()` only.
`maos-egress = "600"` produces **no budget row, no warning, no error**. A negative value wraps:
`kloc_check.rs:149` casts `v as u64`, so `foo = -1` makes the crate unbounded and green.
⇒ Bare positive integers only. Re-run the gate and confirm the row appears in `--json` `per_crate`.

**(c) DUPLICATE KEYS HARD-ERROR.** `duplicate key 'x' in document root`, exit 1. The file is 516
lines with rows in **five separate blocks in creation order, not alphabetical** (`:195-202`,
`:212`, `:272-283`, `:319-344`, `:464-498`). Before adding any row, grep the whole file for the key.

**(d) THE FAILURE TABLE LIES ABOUT THE CAUSE.** `kloc_check.rs:60` does
`get_budget(...).unwrap_or(0)` and `:61` renders `❌ OVER` for any `loc > 0`. A crate with **no
row at all** prints `❌ OVER` while `over_budget` is empty and the gate is green.
⇒ **Read `over_budget` in `--json`, never the ❌ column.** And note CI runs `--json`
(`discipline.yml:189`), so the human table is a local-run artefact that no reviewer sees.

**Bonus, and it is why AC2 pre-adds two rows for crates that do not exist:** a crate present in the
tree with **no** `kloc.toml` key is **never checked** (`kloc_check.rs:229-235`, `if let Some(&budget)`)
— it silently joins the aggregate and escapes its own ceiling entirely. This has already happened
once, on the record: `docs/adr/ADR-055-multi-tenant-loom.md:183` — *"`maos-cohort` is absent from
`xtask/kloc.toml` entirely. Story 13.6a grew it ~+302 lines unmeasured."* Pre-adding rows for
`maos-egress` (17-1) and `maos-orchestrator-buffer` (21-5) closes that hole before it opens, is
**zero-cost** (a budgeted key with no directory measures 0), and is **precedented three times over**
— `maos-cap-registry = 3000` (`:197`), `maos-wire = 2000` (`:199`), `maos-journal = 2000` (`:200`)
all ship green today with no directory.

---

## 3. 🔴 THE EPIC'S OWN AC2 IS ARITHMETICALLY VACUOUS AS WRITTEN

The epic says `_aggregate_hardfail` is *"re-derived by the formula at `kloc.toml:58-59`
(`measured + max(100, ceil(0.02·measured))`, = 158608 at HEAD before grants)"*.

**158608 is correct arithmetic and the wrong number.** 155498 + max(100, ⌈0.02·155498⌉ = 3110) =
158608 ✓. But the lane is authorized to write **13774 lines** (AC3). Under 158608 it could draw
**3110 of them — 23%** — before the aggregate reds again, *with every per-crate ceiling still green*.
The re-base would deliver a table nobody can spend.

The epic anticipated this in its own escape hatch — *"the post-grant figure is recorded in the
story"* — so the story records it. ⚠ **The round-1 draft of this section proposed
`measured + Σgrants + 1 = 167866`, and the 2026-09-06 round-table OVERTURNED it**: sizing the aggregate
to the grants alone makes those grants compete with **20043 lines of headroom on rows this story does
not grant** — the reservation this story's own thesis forbids, one level up. The derivation that
survived is in **AC3**: `155498 + 13774 + 1512 + 100 = **170884**`, every term a measurement. **7000 of
that 20043 is retired outright** (three crates that do not exist, AC1 table).

**The aggregate remains a real independent instrument, and here is the proof rather than the claim:**
after this re-base the sum of per-crate ceilings is **182113** against a **170884** aggregate — a
**11229**-line gap. Distributed growth that no per-crate reserve can see still reds the aggregate
first, which is D17's stated reason for existing (`epic-14-preflight-decisions.md:83`: *"the only
instrument that catches distributed growth no per-crate reserve can see"*). D17 is discharged by
re-derivation, not erased, and the gap is a **named, derived reserve** instead of an accident.


**`_aggregate_alarm` is dead signal and this story makes it live again.** It is `16000`
(`kloc.toml:500`) against 155498 — it has fired on **every run since Epic 1** and is the sole
reason `kloc_check_runs_on_workspace` is `#[ignore]`d. Re-based to **158608** — the pre-grant
formula figure — it becomes the one statement worth making: *the workspace has begun drawing on
the Epic-15 allowance.*

---

## 4. 🔴 A RATIFIED ARCHITECTURE CLAUSE FORBIDS EXACTLY WHAT THIS STORY DOES

`architecture-maos-minimal-opus/15-full-spectrum-v2-2.md:99`, clause 2 (**ADV-055-2**, ratified
2026-07-09), verbatim:

> ceilings move only **(a) downward at any time, or (b) at epic retro, to the tight measured
> residual (+≤1% slack)** … never to round headroom, never mid-epic, never for planned growth.

This story violates all four clauses: it is **mid-epic** (it *opens* the epic), it grants **round
headroom**, the entire "Asks by epic" column **is** planned growth, and the formula is **2%**, not
≤1%. The 2026-07-25 founder CEILING RULE (`kloc.toml:49-87`) supersedes it in practice — later,
founder-ratified, and unanimous — but **clause 2's text was never amended**, so at HEAD the
architecture and the instrument say opposite things.

That the clause drifts unwatched is demonstrable from the same paragraph: clause 4 (`:102`) still
cites `_aggregate_alarm=16000/_aggregate_hardfail=103000` against a live 147057.

⇒ **AC4 amends clause 2 in the same commit.** A re-base that leaves a ratified clause forbidding
it is a claim standing in for a control.

**And the epic mis-cites its own authority for the reverse move.** The 15-2 table says *"the
ceiling is LOWERED by the same amount when 21-5 lands, **ADR-038**"*. `docs/adr/ADR-038-per-service-kloc-ceiling.md`
is 15 lines and says **nothing** about lowering, extraction, or lines moving out of a crate; `:17`
governs *raising* via the ADR-037 process. The clause that does authorise it is architecture
clause 2 (**ADV-055-2**), whose sub-clause (a) governs downward movement and whose extraction
sentence sets a new crate's initial ceiling from measured LOC at extraction — and `kloc.toml`'s
MIGRATION RULE says the **opposite**:
*"this pass only ever RAISES … Lowering a ceiling is an architectural decision for a
retrospective, never a side effect."* Three sources, two of them contradictory, one of them not
about the subject at all. AC4 resolves it to one.
⚠ **Cited by clause, not by quote or line — deliberately.** AC4(b) rewrites clause 2 in this same
commit, so the round-1 draft's quote-form citations (*"downward at any time"* and *"New crates
minted by extraction get initial ceiling = measured LOC at extraction"*) were **invalidated by the
edit that this section argues for** — the F2/R6 hazard committed in the act of closing it
(review R-P9). The post-amendment text is in the architecture file; this section names the
sub-clause and stops.

---

## 5. 🔴 THE `maos-cohort` ROW RESERVES AGAINST A GRANT THAT DOES NOT EXIST

The epic's table: *"`maos-cohort` +100 (21-3 asks +150–300; the remainder is the **14-2d T8
measured grant**, not a new row)"*.

There is no 14-2d T8 measured grant. Measured in the file:

- `14-2d-self-identity-rotation.md:20` — `Status: **blocked.**`
- `:536-537` — T8 is an **unchecked task**: *"`cargo fmt --all`, measure, record EXACT MEASURED /
  ZERO HEADROOM, **then ask for** the `maos-cohort` + `maos-bin` grants."*
- `:7` — the frontmatter is a warning, not an authorization: *"**Expect** measured grants … **ask
  only after** `cargo fmt --all`."*

It is **a plan to request a grant, in a blocked story, with no amount stated**. It was never
opened, so it can be neither held nor consumed. And its figures are dead anyway: 14-2d measured
`maos-cohort 5678/5713 = +35` at `53845402`; HEAD is `5857/5857 = 0`. The crate grew 179 lines and
the +35 was spent by someone else in between.

> **This is the second occurrence of the same failure and it is now worse than the first.**
> `14-2c` recorded the rule after paying for it: *"A GRANT IS A GLOBAL, NOT A RESERVATION — a
> pre-authorized ceiling is consumed by whichever story measures first."* There the story reserved
> against a **real** grant someone else had spent. Here the epic reserves against a grant **nobody
> ever asked for**.

⇒ AC1 grants `maos-cohort` **+390** — `⌈300 × 1.3⌉` under the epic's rule 6 — on its own measurement (21-3 asks +150–300), and the T8
clause is struck. AC1's ask column is labelled a **ledger of intent**, never a reservation.

---

## 6. 🔴 THE TABLE WAS BUILT FROM THE R2 §E NOTES AND ROUND 3 THEN RE-MEASURED THE SAME CRATES

The epic's table header asserts *"asks from the R2 notes' §E"*. Round 3 subsequently re-priced many
of the same crates **in the epic files**, and the table was never re-derived. Where the two
disagree, the table holds the older figure while the epic it points at holds the newer one.
Re-derived here; **the AC1 table below is the corrected one.**

| # | Row | Epic table says | The tree says | Effect |
|---|---|---|---|---|
| 1 | `maos-registry` | 20-1 `+50–120` (e20 §E:121) | `epic-20…:47` re-prices it **+230–440** | Σupper 590 vs a +300 grant → **1.97× short** |
| 2 | `maos-domain` | omits Epic 17 and Epic 19 | e17 §E:132 `+25–50`; e19 §E:110 `0–30` | two epics unbooked |
| 3 | `maos-domain` | `18 +50–100` | `epic-18…:34`: *"maos-domain **0** (the port does not land there)"* | pre-decision figure |
| 4 | `xtask` | omits **21-2 `+150–300`** (e21 §E:161) | `epic-21…:28`: *"over-subscribed by up to ~190 at the top"* | Σupper 800 vs +400 |
| 5 | `xtask` | `15-3 … −260` for `check_cna_registration.rs` | `wc -l` **260**, **tokei `code` 209** | see §7 — the AC-level "net ≤0" claim fails at its own top |
| 6 | `xtask` | 20-3 "~2500 retirable" | `epic-20…:94` names ten retirees summing **6406** | superseded estimate |
| 7 | `maos-bin` | "over-subscribed **~2×**" | measured **4640/3000 = 1.55×** gross, 4161 = **1.39×** net | verdict copied from the note where the grant was still +1200 |
| 8 | `maos-bin` | "gross sum ≈ +1.9k–4.2k" | Σ = **+2360–4640 gross**; 1.9k–4.2k is the figure **already net of −479** | gross/net mislabel, propagated into round-3 §6.2 |
| 9 | `maos-bin` | "18 +300–680 (**18-2** named +250)" | `epic-18…:34`: +250 is **Epic 18's whole** named draw; `epic-18…:47`: 18-2 alone is `+80–200` | attribution |
| 10 | `no change` rows | "HEAD headroom already covers the ask" | `maos-control` 480–**760** vs 697; `maos-shell` 100–**150** vs 100 | **two of the four are FALSE** |
| 11 | `maos-cli` | "was +700 … (E16 note)" | **no §E anywhere grants +700**; all five notes say `+400`. +700 is the *consolidated* §5 verdict | provenance |
| 12 | `maos-cli` | +1100 = 700 + 400 | the same cell also books `18 maosctl eval 0–120` → Σupper **1220** | 1.11× short |
| 13 | `maos-kernel-core` | "**three** FLAG-Winston stories" | five: `16-5`, `17-1`, `17-2`, `19-3`, `21-5` (+ 15-1's un-flagged +2, + 18-3's conditional Clock) | miscount, copied from the D-F rationale row |
| 14 | `maos-kernel-core` | `21-5 ≈ −1000` | `epic-21…:26,:44,:86`: **−817 tokei / −1028 physical**. `rescout-r3.md` filed this exact fix; **never applied** | wrong unit → would over-lower by ~183 |
| 15 | `xtask` | `19 +150 (demo-j1 --replay)` | D-D **retired `--replay`**; `rescout-r3.md:89` filed `(cassette env forwarding + beat re-labelling)`; **never applied**. `rescout-r3.md:103`: the last live `--replay` in the lane | stale verb |
| 16 | `maos-egress` | grant `+300–600`, ask cell `—` | ceilings are **integers**; and it is the only row with a grant and no ask | unwritable as stated |
| 17 | in-table `maos-cli` | *"20-1 draws beyond **+700**"* (`epic-15…:81`) | a 4th stale `+700` that `rescout-r3.md` D5 never enumerated | unfiled |
| 18 | `epic-20…:47` | D5's fix | landed with **the two numbers swapped** — *"note 16-1 books +1100 … of the same +700 grant"* | mis-applied; round-3 §5 claims "all 11 were fixed" |

**AC1's promise** — *"every crate named in Epics 16–21 shows the headroom its measured stories
need"* — is not met by the epic's grants: **13 rows are short at the upper bound and two
(`maos-cohort`, `xtask`) are short at the lower.** The corrected table covers **all 18 estimate-sized
rows at `⌈Σupper × 1.3⌉`** (the epic's own rule 6, which the original table dropped — F1c), prices
`maos-orchestrator-buffer` as a **measured move** rather than an estimate (917, F1d), and declares the
remaining three as policy rather than grants: `maos-bin` and `xtask` as **ceilings** with their
uncovered epics named, and `maos-kernel-core` as a **permission with zero capacity** (§8).

---

## 7. 15-3's "net ≤ 0" arithmetic lives in THIS story's row, and it does not hold

`epic-15…:110` (15-3 AC4) states no counts and defers: *"xtask net lines ≤0 measured in the same
commit … (**arithmetic in 15-2's xtask row**)."* So this is the only place it exists. Measured:

```
tokei code, xtask/src/check_cna_registration.rs  =  209   (wc -l = 260 — the gate counts tokei code)
15-3 gate adds (epic table)                       = +150 … +250
phase consolidation                               =    −7 consts
net at the TOP of the range   = 250 − 209 − 7 =  +34  →  NOT ≤ 0
net at the BOTTOM             = 150 − 209 − 7 =  −66  →  ≤ 0 ✓
```

`epic-15` §E prices 15-3 at `+≤60 gate + ~40 phase generator` ≈ **+100 max**, under which it holds
comfortably; the `+150–250` came from the *consolidated* §5 table, not from a §E. **The `+400`
`xtask` grant absorbs the +34 either way** (41953 + 34 = 41987 against a 42353 ceiling), so 15-3 is
not blocked — but the claim as written is false and AC1 records the corrected arithmetic rather
than repeating it.

⚠ **Doc comments are free.** tokei nests `///` and `//!` into a Markdown blob and the gate reads
only the top-level Rust `code` field (`kloc_check.rs:28,:213`). `check_epic_6_bridge.rs` is
`4592 lines / 4025 code`. Any retirement arithmetic in Epics 15–21 stated in `wc -l` is
**overstated by 10–20%**. In-`src` `#[cfg(test)]` **is** charged; `tests/` directories are not.

---

## 8. 🔴 THE STORY'S OWN THESIS CONVICTED ITS OWN KERNEL ROW (round-table, 2026-09-06)

This story argues, in its frontmatter and in §5, that **a grant is a global, not a reservation** — and
then D-F pre-authorized **250 `maos-kernel-core` lines against five stories that do not exist yet**.

That is a reservation. It is the same move the story indicts the `maos-cohort` row for, in the one
crate where the project spent three epics building a per-story discipline specifically to prevent it:

- `kloc.toml:195`, D13(a), in its own words: *"WHY ZERO HEADROOM, over the formula's 19312 … at zero
  headroom every further kernel-core line still costs its own measured FLAG-Winston grant and a
  HISTORY row, **which is the property being bought here**."*
- `check-kernel-baseline` is the control on that crate, not the ceiling. A +250 ceiling does not
  authorize a single line — every one still needs its pin and its HISTORY row — it only **removes the
  second tripwire** that would have caught a line whose pin was re-pinned without its ceiling. Which is
  precisely the failure D13 was filed for: *"one was updated, the other was not."*

**What D-F actually decides is a permission, and it is fully satisfied without any capacity.** The
Epic-13 refusal was *"stay tight because Epic 14 is declared ZERO kernel-Δ"* — a conditional whose
condition expired when Epic 14 reached `done`. Overruling it means **raises are now allowed**; it never
meant they should be pre-issued. Every FLAG-Winston story raises ceiling and pin in the same commit,
which is what `15-1` AC2 already does for its +2.

⇒ `maos-kernel-core` is **UNCHANGED at 18933/18933** by this story. The 202-line forecast is funded in
the aggregate (AC3) and granted to nobody. D13(a) survives verbatim, D-F is satisfied in writing, and
the story stops arguing against reservations while making one.

**Corollary the room could not un-see.** After this re-base `maos-bin`'s ceiling is **20109** and
`maos-kernel-core`'s is **18933**. The daemon's budget now exceeds the trusted core's by **1176 lines**,
and NFR-Maint-1 has an opinion about one of those numbers and nothing whatever to say about the other.
`maos-bin`'s 3000 against a 4640 gross ask is therefore **not a shortfall to be topped up** — it is the
only instrument in this repository that will notice the daemon becoming the second kernel. It is
written as a declared architectural constraint with its uncovered epics named, so that Epic 17 has to
come and argue for a bigger daemon instead of simply growing one.

---

## Decisions ratified in this story (rule 7 — one decision per fork)

| # | Fork | **Ruling** | Killed alternative |
|---|---|---|---|
| **F1** | Aggregate re-derivation | **`155498 measured + 13774 funded + 1512 no-change asks + 100 rule floor = 170884`** — every term measured. *(Round 1 proposed `+12367+1`; **overturned** — 20043 of UNGRANTED headroom competes for the same allowance, so a sum sized to the grants alone is the reservation this story says grants aren't.)* | the bare formula (158608, spendable to 25%); and `measured + Σgrants + 1`, whose `+1` is a fitted constant |
| **F1b** | The 20043 of ungranted headroom | **7000 of it belongs to three crates that DO NOT EXIST** (`maos-cap-registry` 3000, `maos-wire` 2000, `maos-journal` 2000). **Zeroed — not deleted.** A deleted key is the silent-pass hole of §2; a zero is a tripwire that makes line 1 of a crate that gets built cost a grant. | leaving 7000 lines of ownerless authorization inside the aggregate for six epics |
| **F1c** | Rule 6, *"size from measurement (+30%)"* — the epic's own binding rule | **15 of 20 measurement-sized rows violated it**; the table applied *"from measurement"* and dropped *"+30%"*, granting `maos-eval` 900 against a 900 estimate, `maos-manifest` 200/200, `maos-egress` 600/600. Grants re-derived at **⌈Σupper × 1.3⌉**. | shipping zero margin over numbers that are ranges in a scout note |
| **F1d** | Does +30% apply to everything? | **No — rule 6 covers ESTIMATES.** `maos-orchestrator-buffer` is 817 **measured** lines being *moved*; there is no estimation uncertainty in a move, so the CEILING RULE formula governs and **917 stands**. `maos-bin`/`xtask` are **policy ceilings** (below), also exempt. | applying +30% mechanically and inventing risk that does not exist |
| **F2** | Where lines go | **SPLIT.** The `:512` trap stands verbatim — a row after `[in_progress_decomposition]` is silently inert (and AC6's R-P7 patch now NAMES such a row instead of swallowing it). The *citation* arm is **RETIRED**: every line-number citation into `kloc.toml` is converted to **quote-form**, which costs **ZERO** tokei lines because comments are not `code` — so the amendment goes where it belongs in the rule block instead of where line numbers forced it. (R6, `14-2c`: cite the symbol, not the line.) ⚠ **Round-1 said "all four"; the review sweep found ELEVEN, five of them made stale by this story's own edit** — see AC2's corrected note. | bending the document's structure around preserving `router.rs:1250`/`:1271` |
| **F3** | `maos-cohort` | real **+390** grant on its own measurement (`⌈300 × 1.3⌉` under rule 6, F1c); T8 clause struck. *(The round-1 figure was `+350`, superseded by F1c's re-derivation before AC1 was written; corrected here at review — a stale number inside the register that ratifies it is §6's own failure mode.)* | reserving against the non-existent 14-2d T8 grant |
| **F4** | The 13 short rows | raise to **Σupper of the corrected asks** — ⚠ **SUPERSEDED by F1c**, which takes the same rows to `⌈Σupper × 1.3⌉` under the epic's own rule 6. `maos-bin`/`xtask` stay **declared ceilings** (F12); `maos-kernel-core` becomes a **permission with zero capacity** (§8). | granting the epic's figures and shipping AC1's promise false |
| **F13** | Where the ask ledger lives after landing | **In `kloc.toml` row comments** — one ledger line per re-based row: what was granted, what is booked against it, by which story, in the shape the `| prior:` chain already uses. AC1's table becomes the *derivation*; the file becomes the *record*. | leaving the only thing that stops the next story double-booking inside a story file that is archived and unread by March |
| **F14** | Operator directive 2026-09-06: *"try not to defer; if it is about some more effort, we pay it early — deferring drifts"* | **Every deferral re-classified EFFORT vs SCOPE.** Pulled back IN (effort): AC4(d)'s four stale-citation fixes · the unbudgeted-crate silent pass (ADR-055:183) · the lying ❌ column · the `unwrap_or` aggregate defaults · the quoted/negative budget silent drop · un-ignoring a test ignored since Epic 5. **Tests are free** — `xtask/src/tests/` is measured at 2697 code lines and is UNCHARGED, so no budget argument survived. Kept OUT (scope, with the reason named): the `kernel-crate-set` successor instrument; a gate that checks a cited grant exists — **impossible until grants are machine-readable, which is 21-4's instrument unification, not effort**; the `>=`/`>` asymmetry, which is a *decision* (changing it loosens the aggregate by one line for no benefit) and is now documented rather than deferred. | filing effort as scope, which is how the four `rescout-r3` fixes came to be filed-and-unapplied in the first place |
| **F5** | Architecture ADV-055-2 | **amend clause 2 in the same commit** | shipping against a ratified clause that forbids the story |
| **F6** | Authority for lowering | cite architecture `:99` 2(a); correct `21-5 ≈ −1000` → **−817 tokei / −1028 physical** | ADR-038, which is silent on lowering |
| **F7** | The gate's false messages | **fix both strings — CONDITIONED on shipping the first non-null-control test that gate has ever had**: assert the message names the *configured* threshold (proven-red today: with `_aggregate_hardfail = 10` it still prints "20 KLOC"). | an unfalsifiable edit to the instrument, inside the story that re-bases the instrument's data |
| **F7b** | `tokei` is unversioned | `kloc_check.rs:165` is `let tokei_path = "tokei";` — a bare PATH lookup with **no version assertion**, while CI pins 14.0.0 (`discipline.yml:11`). A dev on another tokei measures a different tree, writes 49 ceilings from it, and CI reds on numbers they cannot reproduce. **Assert the pinned version, fail by name.** For a story whose whole deliverable is tokei numbers this is load-bearing, and it is the gate's second real test. | "the numbers are right" when the story can only honestly claim "the numbers are checkable" |
| **F11** | The amendment has no expiry | **The epic-scoped operator allowance EXPIRES at `21-4`**, written into the clause itself. Correct for six epics (the lane cannot survive a governance event per crate per story; founder directive is functionality-first) — but an amendment with no expiry is a permanent loosening dressed as a one-time exception. | adding a third sanctioned recalculation source to the CEILING RULE forever |
| **F12** | `maos-bin` at 3000 vs a 4640 gross ask | **STAYS at 3000, and stops being an apology.** After this re-base `maos-bin`'s ceiling (20109) **exceeds `maos-kernel-core`'s (18933) by 1176**, and NFR-Maint-1 governs one of those numbers and says nothing about the other. The under-grant is not a defect in the table — it is the only instrument in this repository that will notice the daemon becoming the second kernel. Written as a **declared architectural constraint** with the epics it does not cover named, so Epic 17 has to argue for a bigger daemon rather than simply doing it. | presenting it as a short grant to be topped up later |
| **F8** | `_aggregate_alarm` | re-base `16000` → **158608** (the pre-grant formula) | leaving a warning that has fired on every run since Epic 1 |
| **F9** | D14 / D17 register rows | **re-point both to `15-2` and CLOSE them** (AC7) | letting `21-4` and `15-1` carry decisions this story discharges |
| **F10** | `maos-egress` grant | **600** (integer, upper of the range) | `+300–600`, which cannot be written to a TOML integer |

---

## Acceptance Criteria (7)

*Round-table 2026-09-06 refined every one of these, and the operator's same-day directive (**"deferring drifts"**, F14) pulled six items back OUT of the deferral list and INTO AC4(d) and AC6.*

* AC1's table is re-derived under the epic's own
rule 6, AC3's aggregate is overturned, AC4 gains an expiry, AC6 absorbs the tokei pin, and AC4(d) is
cut to a filed residual. §8 is the finding that changed the most: this story was making a reservation.*

**AC1 (command) — `cargo run -p xtask -- kloc-check` exits 0, and the headroom is *watched to work*,
not merely tabulated.**
The observable is `over_budget: []` and `kloc-check: PASSED (aggregate=155498 LOC)`.
🔴 **A green that could not have been red is not evidence, and `PASSED` prints for a ceiling that is
too loose exactly as it does for one that is right.** So the AC is demonstrated in both directions,
in four minutes, and the transcript goes in the story:
1. **Proven-green:** write `headroom` throwaway lines into the granted crate with the tightest margin
   (`maos-iac`, 100), run the gate → still green. *That* is the story's deliverable — a line landing —
   rather than a table asserting one could.
2. **Proven-red:** one line past that ceiling → `over_budget: ["maos-iac 7061 > 7060"]`, exit 1.
3. Revert both. The ceiling is proven to **bind in both directions**, which no test in this
   repository does (frontmatter `review`). The table
below is stated in **FINAL CEILINGS**, because re-basing to `measured + grant` **destroys** the
existing headroom on nine rows and the epic's "+N" column is ambiguous about it. HEAD measured =
tokei `code` @`03ee0ad8`, re-run and identical to `9f920180` (no code changed between them).
Instrument: `kloc_check.rs:163-213`; exclusions `:173-190` (⚠ the epic swaps two labels — `:180`
is **`examples`**, `:184` is **`spirits`**, and both are bare directory-name patterns matching at
**any depth**, not globs).

**D-F, in writing — and it is a PERMISSION, not capacity (§8).** The Epic-13 retro's blanket refusal
to ever grant `maos-kernel-core` headroom (`kloc.toml:501`, *"stay tight because Epic 14 is declared
ZERO kernel-Δ"*) is **overruled for the v2.2+ lane** — Epic 14 is `done` and its ZERO-Δ declaration
expired with it. But the ceiling is **NOT pre-raised**: `maos-kernel-core` stays at **18933/18933,
zero headroom**, because that property was bought deliberately by D13(a) over the formula's 19312
(`kloc.toml:195`), and pre-granting 250 lines to five stories that do not exist yet would buy it back —
a reservation, in the one crate where three epics of per-story FLAG-Winston discipline exist to
prevent one. Each of `15-1` +2 (D-H) · `16-5` +5–25 · `17-1` +65–130 · `17-2` net ≤+10 · `19-3` +3–5,
plus `18-3` AC3's conditional Clock +15–30, raises **ceiling and pin together in its own commit**.
Σ forecast **202** (5 FLAG-Winston stories, not the 3 the epic counted) is funded in the aggregate
(AC3) and pre-granted nowhere.
⚠ **NFR-Maint-1 is the one requirement this crate answers to** (`prd/non-functional-requirements.md:132`:
*"Kernel trusted core ≤ 20 KLOC excluding tests through v2.0"* — the kloc regime is the one its
"excluding tests" letter names, `architecture…/15-full-spectrum-v2-2.md:93`). At the forecast close
≈19135 the crate is **865 under the 20 KLOC letter**. The *aggregate* re-base does not touch
NFR-Maint-1 at all: the NFR bounds the trusted core and explicitly delegates adapters to *"their own
LOC budgets."*

| Crate | HEAD | old ceiling | **NEW ceiling** | headroom | Σupper | Asks by epic — *a ledger of intent, not a reservation*; **this column lives in `kloc.toml` row comments after landing (F13)** |
|---|---|---|---|---|---|---|
| `maos-bin` | 17109 | 17109 | **20109** | 3000 | 4640 | **POLICY CEILING — a declared architectural constraint, not a grant, and exempt from rule 6.** 20109 **exceeds `maos-kernel-core`'s 18933 by 1176**; NFR-Maint-1 governs the kernel and is silent on the daemon, so this row is the only thing that will notice the second kernel forming. 15-1 ~10 · 15-6 +150–300 · 16 +480–820 · 17 +440–870 · 18 +250 named · 19 +470–980 · 20 +100–200 · 21 +380–760, −479 (D-I) → **+2360–4640 gross / +1881–4161 net = 1.39× at the top**. **Uncovered: Epics 17, 19 and 21 in combination — a story that collides here argues for a bigger daemon or moves logic to ungoverned `spirits/*`.** |
| `xtask` | 41953 | 41953 | **42353** | 400 | 800 | **POLICY CEILING, exempt from rule 6, and a transient**: 20-3 nets **−4476…−5176** (`epic-20…:94`). Before it: 15-3 +34 at its top (§7) · 15-1 AC3/AC6 · 19 +150 · this story's AC6 + AC8 (≤+2 strings, ~+10 version assertion). After it: 21-2 +150–300, 21-4 +150–350. **Uncovered: 21-2 + 21-4 at the top iff 20-3 slips past them.** |
| `maos-kernel-core` | 18933 | 18933 | **18933** | 0 | 202 | ⚠ **UNCHANGED — ZERO HEADROOM, deliberately.** D-F's `+250` was a **reservation** (§8). D13(a) bought this property on purpose (`kloc.toml:195`: *"at zero headroom every further kernel-core line still costs its own measured FLAG-Winston grant and a HISTORY row"*), and pre-granting buys it back. **D-F = standing permission, not capacity**: the Epic-13 blanket refusal is overruled, and each of `15-1` +2 · `16-5` +5–25 · `17-1` +65–130 · `17-2` ≤+10 · `19-3` +3–5 (+ `18-3`'s conditional Clock) raises ceiling **and** pin **in its own commit**. Σ forecast **202** is funded in the aggregate and pre-granted nowhere. Ceiling at forecast-close ≈19135, still **865 under the NFR-Maint-1 20 KLOC letter**. |
| `maos-cli` | 5270 | 5270 | **6856** | 1586 | 1220 | ⌈1220×1.3⌉. 16-1 HTTP client for 21 verbs +400–700 · 18 `maosctl eval` 0 unless built (+60–120) · 20-1 `spirit install` +250–400 |
| `maos-eval` | 3661 | 3696 | **4831** | 1170 | 900 | ⌈900×1.3⌉ — was 900/900, **zero margin over an estimate**. 18 halt-eval driver + judge harness +450–900 |
| `maos-control` | 803 | 1500 | **1791** | 988 | 760 | ⌈760×1.3⌉. **`no change` was FALSE** — 480–760 vs 697. 16-1 +400–600 · 19-2 +40–80 · 21-3 +40–80 |
| `maos-egress` | — | — | **780** | 780 | 600 | ⌈600×1.3⌉ — was 600/600. **NEW row; crate does not exist, 17-1 mints it.** ADR-061/D-B. ⚠ Possible double-count: `epic-17` §E priced `maos-bin 17 +440–870` **before** D-B moved the proxy out; 17-1 states which side it charges. |
| `maos-registry` | 3515 | 3615 | **4282** | 767 | 590 | ⌈590×1.3⌉. 17-4 +60–150 · **20-1 +230–440** (`epic-20…:47`; the epic's +50–120 is superseded) |
| `maos-journey-test` | 493 | 593 | **1104** | 611 | 470 | ⌈470×1.3⌉. 16-2 +50–120 · 18-1 fixture MCP server +150–300 · 21-2 0–50 |
| `maos-spirit-cli` | 753 | 853 | **1273** | 520 | 400 | ⌈400×1.3⌉. 20-1 client wiring + `vet issue|revoke` +230–400 |
| `maos-wasm-host` | 1057 | 1129 | **1577** | 520 | 400 | ⌈400×1.3⌉. 17-3b +200–400 **+ an unquantified runner relay** |
| `maos-domain` | 8695 | 8644 | **9192** | 497 | 382 | ⌈382×1.3⌉. Absorbs D14's **+51 RED**. 16-4/16-5 +36–66 · **17 +25–50 (omitted by the epic)** · 18 **0** (`epic-18…:34`) · **19 0–30 (omitted)** · 21-3 +3–10 · 21-4 +15 · 21-5 +30–60 |
| `maos-cohort` | 5857 | 5857 | **6247** | 390 | 300 | ⌈300×1.3⌉. 21-3 +150–300. **§5: the "14-2d T8 grant" does not exist and is struck.** |
| `maos-manifest` | 4214 | 4314 | **4474** | 260 | 200 | ⌈200×1.3⌉ — was 200/200. 17 `[network]`, `pids_max`, `wasm-component` + validators +80–200 |
| `maos-bench` | 1531 | 1631 | **1791** | 260 | 200 | ⌈200×1.3⌉. 19-3 real-spawn bench +80–200 |
| `maos-mcp` | 999 | 1100 | **1194** | 195 | 150 | ⌈150×1.3⌉. 18-1 initialize/session/bearer +80–150 |
| `maos-shell` | 305 | 405 | **500** | 195 | 150 | ⌈150×1.3⌉. **`no change` was FALSE** — 16 +100–150 > 100 |
| `maos-a2a-tcp` | 1439 | 1500 | **1621** | 182 | 140 | ⌈140×1.3⌉. 21-3 +50–140 |
| `maos-a2a-core` | 4856 | 4856 | **4986** | 130 | 100 | ⌈100×1.3⌉. 16-5 +20–40 · 21-3 +30–60 |
| `maos-audit` | 6847 | 6847 | **6951** | 104 | 80 | ⌈80×1.3⌉. 20-3 AC5 (14-e1) +20–60 · relocated governance-categories test +20 |
| `maos-iac` | 6960 | 6960 | **7060** | 100 | 30 | floor 100 (⌈30×1.3⌉=39 < the rule's own minimum). 17 caller-scoped recall +10–30 |
| `maos-orchestrator-buffer` | — | — | **917** | 917 | 817 | **NEW row; 21-5 mints it.** `917 = 817 tokei MEASURED + max(100, ⌈2%⌉)`. **Rule 6 does NOT apply** — these lines exist in kernel-core today and are being *moved*; a move carries no estimation uncertainty (F1d). |
| `maos-cap-registry` · `maos-wire` · `maos-journal` | 0 · 0 · 0 | 3000 · 2000 · 2000 | **0 · 0 · 0** | −7000 | — | **RETIRED (F1b).** Three crates that do not exist, holding 7000 lines of ownerless authorization. **Zeroed, not deleted** — a deleted key escapes the gate entirely (`kloc_check.rs:229-235`), a zero makes line 1 cost a grant. Supersedes `epic-21…:82` (21-4 AC4), which says *delete* six epics from now; filed as an OLD→NEW (AC4(c)). ADR-041 Phase 4 mints `maos-scheduler`/`maos-memory`/`maos-hot-swap`/`maos-supervision`, **not these three**. |
| `maos-compliance` · `maos-host` · `maos-persistence` · `maos-spirit-abi` · `maos-secrets` · `maos-providers` · `maos-spirit-hello` · `maos-spirit-sdk` | 2032/2117 · 39/139 · 1/2000 · 1038/3000 · 161/1000 · 1252/2000 · 365/1000 · 839/3000 | — | **`no change`** | 85 · 100 · 1999 · 1962 · 839 · 748 · 635 · 2161 | 0 · 30 · 600 · 2 · 300 · 460 · 20 · 100 | headroom ≥ Σupper for each. Their **Σ asks = 1512** is authorization too, and is funded in the aggregate (AC3). `maos-compliance` is **0** — `epic-20…:47`: the issuer lives in `maos-spirit-cli`. |
| `_aggregate_alarm` | — | 16000 | **158608** | — | — | F8 — the pre-grant formula figure. Fires exactly when the workspace begins drawing on the Epic-15 allowance. |
| `_aggregate_hardfail` | 155498 | 147057 **(−8441 RED)** | **170884** | — | — | F1 — `155498 + 13774 + 1512 + 100`. |

**Σ funded lines = 13774** (grants 13572 + the kernel-core forecast 202, pre-granted nowhere).
**Σ per-crate ceilings after the re-base and the 7000-line phantom retirement = 182113**, which is
**11229 above** the new hardfail — so the aggregate still reds on distributed growth no per-crate
reserve can see (D17's stated purpose), and that gap is now a **named, derived reserve** rather than
an accident of arithmetic.

**AC2 — the rows are written where the parser can see them, in the house format, and the gate
proves it.**
Every edited/new row keeps the existing shape — a bare positive integer, three spaces, `#`, the
new claim first, prior history demoted behind ` | prior: ` (precedent `:275` for the clean
single-line form; `:319`, `:343`, `:482` for the chained inline form; `:203-206`, `:316-318`,
`:334-337`, `:471-474` for the comment-line-above form). Comment text: `⚠ 15-2 EPIC-15 MEASURED
RE-BASE 2026-09-xx (operator-authorized; measured at 03ee0ad8 with tokei 14.0.0, the CI pin):
OLD -> NEW (+Δ). Grant = <n>. Σupper of the Epics 16–21 asks = <m>. ZERO kernel-Δ @24472.`
🔴 **The citation arm of F2 is RETIRED, at zero cost.** The round-1 draft bent this file's structure
around preserving `kloc.toml:87` for `router.rs:1250`/`:1271`. **tokei does not count comments**
(`kloc_check.rs:28,:213`), so converting every line-number citation into `kloc.toml` — to
**quote-form costs ZERO lines**, including in
`maos-a2a-core`, which is the crate that can least afford anything. Do that instead of routing around
them (R6, `14-2c`: *a correction to a stale citation is itself a citation and goes stale the same way*).
`kloc.toml:195`'s own `kloc.toml:407` self-citation is **already stale by 94 lines** — proof the hazard
is live, not theoretical. 🔒 It is also an integrity matter: those two comments are the audit trail for
why unbudgeted code landed on a TLS path, and a justification that can silently re-point at different
text is not a justification.
⚠ **CORRECTED AT REVIEW (R-P5): there were ELEVEN, not four, and this story's own edit invalidated
five of them.** The four named above are the ones the round-tables enumerated; a sweep found seven
more inside `kloc.toml` itself. Worse, they were **accurate at HEAD and stale after this commit**:
AC4(a) inserted two lines into the rule block, so the `:60-65` range cited three times no longer
contains the *"never silently re-run to fit"* sentence it was pointing at, and the correctness-repair
clause moved from `:84-87` to `:86-89`, so the two citations that are the audit trail for the 14-2c
grants — **the same clause `router.rs:1250`/`:1271` were converted for** — went stale in the act of
converting the other two. All eleven are now quote-form. The remaining pre-existing offenders live in
`xtask/src` (`check_decision_register.rs:12` → `kloc.toml:208`, already wrong at HEAD) and are
deferred to `21-4` with the reason recorded in `deferred-work.md`.
🔴 **F13 — the ask ledger moves into the file (Sally).** AC1's "Asks by epic" column is the only thing
that stops the next story double-booking a row, and in a story file it is archived and unread by March.
**One ledger line per re-based row, in the row's own comment**: what was granted, what is booked
against it, by which story. It travels with the instrument, in the same shape the `| prior:` chain
already uses. AC1's table becomes the *derivation*; `kloc.toml` becomes the *record*.

**Placement is normative (F2):** both new rows go into the alphabetical `:464-498` block — **never
after `[in_progress_decomposition]` at `:512`**, which silently swallows them (§2a). `[in_progress_decomposition]`
is not touched (`check_epic_6_bridge.rs:1778-1786` substring-greps it for `phase_1`, and while that
job is retired at `discipline.yml:1370`, its unit tests are not). Proven by running the gate: every
new key appears in `--json` `per_crate` with the intended budget, and `over_budget` is `[]`.
🔴 **Proven-red:** write one new row after `:512` and one as a quoted string → both vanish from
`per_crate` with **exit 0 and no warning**. Record both outputs; that is the evidence for F2 and §2b.

**AC3 — the aggregate allowance equals every line this re-base authorizes anyone to write, and each
term is a measurement.**
```
_aggregate_hardfail = 155498  HEAD measured (tokei 14.0.0 @03ee0ad8)
                    + 13774  funded lines  = 13572 per-crate grants + 202 kernel-core forecast
                    +  1512  asks booked against the `no change` rows (authorization too)
                    +   100  the CEILING RULE's own floor, max(100, 2%) — the ratified number
                    = 170884
_aggregate_alarm    = 158608  the PRE-grant formula figure: fires the moment the workspace
                              begins drawing on the Epic-15 allowance (F8; was 16000, which has
                              fired on every run since Epic 1 and is why a unit test is #[ignore]d)
```
🔴 **Why not `measured + Σgrants`.** Because **20043 lines of headroom sit on rows this story does not
grant**, and every line they draw comes out of the same allowance. Sizing the aggregate to the grants
alone would make the grants compete with untracked growth — *the reservation this story's own thesis
forbids*, one level up. **7000 of that 20043 is retired outright** (the three phantom crates, AC1
table); the remainder is either funded here as a named ask or is deliberately unfunded, so a crate
drawing on unclaimed forward-declared headroom reds the aggregate and has to come back. That is D17
working, not D17 failing.
⚠ **The `>=`/`>` asymmetry is why the floor is not decoration.** The aggregate compares
`aggregate >= hardfail` (`kloc_check.rs:225`); per-crate compares `loc > budget` (`:231`). A per-crate
ceiling may equal measured — eight ship that way today and are green — but the aggregate at exactly
its ceiling reds. The 100 is the rule's floor, **not** a fitted `+1`.
The `_aggregate_hardfail` comment prepends this story's claim and **preserves the whole `| prior:`
chain verbatim** — including the `j1-crosshost-2c` and `j1-crosshost-1b` refusals and the split
`−3761 = −1835 (D17) + −1926 (2c)`, which are the record of why the red stood. **D17 is discharged by
re-derivation, not erased**, and the comment carries the proof: 182113 of per-crate ceilings against a
170884 aggregate leaves **11229** of independent anti-distributed-growth signal.


**AC4 — the CEILING RULE is amended, it expires, and every source that contradicts it is amended in
the same commit.**
(a) The rule block gains the **epic-scoped operator allowance** as a third sanctioned recalculation
source beside the epic retrospective and the per-story measured grant (amending `:61-62`), **and**
sanctions a story-driven **LOWERING** when lines physically move out of a crate (21-5) or when a
budget names a crate that does not exist (the phantom retirement), which `:67-71`'s MIGRATION RULE
currently forbids in both cases. It records — as prose, never as a claim of control — that **nothing
reads any of it** (§1).
🔴 **F11 — THE ALLOWANCE EXPIRES AT `21-4`, written into the clause itself.** Six epics of
epic-scoped allowance is right: the lane cannot survive a governance event per crate per story, and
the founder's directive is functionality-first. But `21-4` unifies the two instruments and takes the
question over, and **an amendment with no expiry is a permanent loosening dressed as a one-time
exception** — a per-story discipline silently converted into a per-epic entitlement. If the lane still
needs it after `21-4`, someone renews it in the open.
(b) `architecture-maos-minimal-opus/15-full-spectrum-v2-2.md:99` clause 2 (**ADV-055-2**) is amended to
match, with its stale clause-4 figures (`:102`, `_aggregate_hardfail=103000`) corrected (§4). A re-base
that leaves a ratified clause forbidding it is a claim standing in for a control.
(c) **OLD→NEW filed against `epic-21…:82` (21-4 AC4):** it says the three phantom budgets are
**deleted**. Deleting a key is the silent-pass hole this story's own §2 documents — an unbudgeted crate
is never checked (`kloc_check.rs:229-235`). **Zeroed here, six epics earlier; deleted only when 21-4
unifies the instrument and something else governs an unbudgeted crate.** In the same edit the lowering
authority is re-cited from **ADR-038** (which is silent on the subject, §4) to architecture `:99` 2(a),
and `epic-15…:79`'s `21-5 ≈ −1000` is corrected to **−817 tokei / −1028 physical**.
**(d) The four Round-3 residuals are FIXED HERE, not filed (F14).** They were cut to a residual on
scope grounds and the operator directive reverses that: *deferring drifts*, and these are literally
fixes that `rescout-r3.md` already filed and nobody applied — the drift is in progress. All four are
text edits: `epic-15…:78` `demo-j1 --replay` → `cassette env forwarding + beat re-labelling` (D-D
retired `--replay`; `rescout-r3.md:103` says this is the lane's last live use); `epic-15…:81`'s 4th
stale `+700` → the ratified `maos-cli` figure; `epic-18…:32`'s stale kernel-core `+150` → the D-F
permission (§8); and `epic-20…:47`, where D5's fix landed with **its two numbers swapped**, producing a
sentence that asserts 16-1 books +1100 *and* preserves the stale +700. Round 3's §5 claims *"all 11
were fixed"*; two were not and one landed backwards, and that claim is corrected in the same pass.


**AC5 — the kernel pin protocol is restated and untouched, with its expiry named; the ratification is
recorded in the file it authorizes.**
✅ `FUNDING RATIFIED 2026-09-06 (Lunarpulse)` — the grant increment approved *as the spec needs*, with
the standing directive *"try not to defer; if it is about some more effort, we pay it early — deferring
drifts"* (F14). Recorded here **and** in each `kloc.toml` row comment, per the T0 precedent that a grant
lands with the lines it authorizes. AC1's per-epic column is the **ledger**, never a reservation.
`check-kernel-baseline` is equality (`check_kernel_baseline.rs:65`, `actual == pinned`), so drift in
**either** direction reds; the pin is resolved from `xtask/kernel-core-baseline.toml`, never
restated as a literal. Each FLAG-Winston story re-pins **in its own commit** with its own figure and
a HISTORY row; a kloc grant is capacity, never a pre-approved delta. The two instruments measure
different things (frontmatter `kernel_grant`) and are never compared. ⚠ **"Unchanged" holds for
15-2 … 21-3 only**: `epic-21…:78` (21-4 AC1) re-pins `src_lines` from the physical 24472/24474 to
the tokei value, collapsing the two instruments into one. Documented, not done here.
⚠ **Hand-off to 15-1:** the moment 15-1 re-pins 24472→24474, `check-epic-close-coherence` check 5
reds against **every not-yet-`done` epic file still citing 24472** — `epic-21…:26` is one at HEAD.
That is 15-1's to sweep; it is recorded here because this story lands first.

**AC6 — the gate stops failing open, and every one of its silent passes becomes a named failure.**
Six defects, one class, one commit. **Every test below lands in `xtask/src/tests/`, which is
UNCHARGED** — measured: `tokei` on that directory is **2697 code lines** and `xtask` still reports
**41953**, i.e. the ceiling cannot see them. *There is no budget argument for deferring any of this.*

| | Defect at HEAD | Fix |
|---|---|---|
| **(a)** | `:53`/`:65` print `16 KLOC` / `20 KLOC` from **hardcoded literals** describing the `unwrap_or` defaults, not the config — proven: with `_aggregate_hardfail = 10` it still says "20 KLOC". And NFR-Maint-1 bounds the **kernel core**, not a 155k workspace aggregate | name the workspace aggregate, interpolate the configured threshold |
| **(b)** | `:165` `let tokei_path = "tokei";` — bare PATH lookup, **no version assertion**, against a CI pin of `14.0.0` (`discipline.yml:11`). A dev on another tokei writes 49 ceilings CI cannot reproduce | read `tokei --version`, compare to the pin, fail by name with both versions |
| **(c)** | `:229-235` — a crate **in the tree with no `kloc.toml` key is never checked**. It joins the aggregate and escapes its own ceiling. This is not hypothetical: `docs/adr/ADR-055-multi-tenant-loom.md:183` — *"`maos-cohort` is absent from `xtask/kloc.toml` entirely. Story 13.6a grew it ~+302 lines unmeasured."* | a measured crate with no budget is a **hard fail**, named. Green at HEAD (0 unbudgeted), so it reds only on a real regression |
| **(d)** | `:137`/`:141` — a missing or non-integer `_aggregate_alarm`/`_aggregate_hardfail` **silently substitutes a default** (16000/20000). Deleting the aggregate key makes the gate pass | absent or non-integer aggregate key = **hard fail**, never a default |
| **(e)** | `:148-150`/`:149` — `maos-egress = "600"` yields **no row, no warning, no error**; `= -1` casts to `u64::MAX` and makes the crate unbounded | a non-integer or negative budget is a **hard fail**, named with the key |
| **(f)** | `kloc_check_runs_on_workspace` (`tests/kloc_check_tests.rs:29-36`) has been `#[ignore]`d **since Epic 5** for a reason that expired — *"maos-kernel-core overshoot (21k LOC vs 6k ceiling)"* | **UN-IGNORE it.** After this re-base `passed` is true and `alarm` is false (`155498 >= 158608` = false), so `assert!(report.passed && !report.alarm)` holds. ⚠ My round-2 claim that no re-base could un-ignore it was **wrong** — the story changes the very threshold it depends on |

🔴 **Each of (a)–(e) ships with a proven-red test**, because every existing test on this gate is a null
control (frontmatter `review`) and an unfalsifiable edit to the instrument — inside the story that
re-bases the instrument's data — is the exact defect class this story exists to close. (f) is itself
the proof that the ceilings are now honestly meetable.
Cost: `xtask` **+95 lines MEASURED** (41953 → 42048 after `cargo fmt --all`; the dev pass landed +48
and the review patches added +47 for the honest failure headline, the injectable `tokei` seam, the
threshold-inversion guard and the nested-row rejection), drawn from this story's own `+400` row —
15-3 takes +34 at its top and 19 takes +150 → **42232 against a 42353 ceiling**. ⚠ The pre-dev
estimate in this AC was `≈+32`; it was **50% low** even before review, and the measured figure is
what `kloc.toml`'s `xtask` ledger records (review R-P12: an estimate left standing beside a
measurement is the drift this story exists to stop). **ZERO** semantic change to
what `passed` means for a crate that has a budget.
⚠ CI runs `--json` (`discipline.yml:189`) and never prints (a)'s strings; that audience is the operator
running AC1 locally, which is exactly who ratifies this story.


**AC7 — the decision register is re-pointed, not left holding decisions this story discharges.**
`check-decision-register` is a **real reader** (`xtask/src/check_decision_register.rs`; 23 rows, 15
open, green at HEAD) that asserts every Target-story cell resolves to a live `sprint-status.yaml`
key and reds an OPEN row whose mechanical deadline has passed. Two rows are mis-homed:
- **D14** (`epic-14-preflight-decisions.md:80`) targets `21-4-one-instrument-and-env-registry`,
  deadline *"Before 21-4 leaves `backlog`"*. Its own text demands *"an EXPLICIT AC expansion at
  preflight, never silent inheritance"* — **AC1's `maos-domain` row is that expansion.** Re-point
  to `15-2-kloc-ceiling-rebase` and mark **CLOSED** when the row lands. `21-4` keeps **D13(b)**,
  the instrument question, which this story does not answer.
- **D17** (`:83`) targets `15-1-green-at-head`, deadline *"Before 15-1 reaches `done`"* — but
  `epic-15…:71` (15-1 AC4) defers the re-derivation *to this story*. Re-point to
  `15-2-kloc-ceiling-rebase` and mark **CLOSED**; D17's load-bearing claim was already withdrawn on
  measurement at `epic-14-preflight-decisions.md:314-326`.
🔴 **Proven-green:** `cargo run -p xtask -- check-decision-register` exits 0 after the edit, with
the open-row count down by two. Fix the re-homing map at `:91` in the same pass — it says
`14-7`→`20-5` while D14's actual cell says `21-4`.

---

## Declared cut lines

**Kept out because it is SCOPE, with the reason named (F14) — not because it is effort:**

- **The `kernel-crate-set` successor instrument.** `architecture…:102` clause 4 hands NFR-Maint-1's
  post-v2.0 instrument to a `xtask/kernel-crate-set.toml` that does not exist; it needs a membership
  file, an ADR, a new aggregate key and its own xtask leg. **D13(b), owned by `21-4`.**
- **A gate asserting that a grant a story cites actually exists** (the class behind §5's phantom
  `14-2d T8` reservation). This is **not deferrable effort — it is currently impossible**: grants are
  prose (`grep "MEASURED GRANT" xtask/src` is empty, §1), so there is nothing to check against.
  Machine-readable grants are `21-4`'s instrument unification. Filed there with that reasoning.
- **The `>=` / `>` asymmetry** (`kloc_check.rs:225` vs `:231`). A *decision*, not a debt: making the
  aggregate `>` would loosen it by one line for no benefit. **Documented in AC3 and deliberately
  unchanged**, with the rule's own 100-line floor absorbing the boundary.
- **ADR-041 Phase 3/4 decomposition.** Real architectural work; `[in_progress_decomposition]` is not
  touched (`check_epic_6_bridge.rs:1778-1786` substring-greps it). This story does not make
  `maos-kernel-core` smaller **and does not make it bigger** (§8) — the crate is byte-unchanged, and
  T13 asserts it in the diff.
- **`spirits/*` and `examples/*` governance.** Excluded by design (`kloc_check.rs:180,:184`), and the
  `maos-bin` policy ceiling explicitly relies on that escape valve.
- **Any grant for Epic 22+.** The ledger stops at Epic 21.

**Explicitly NOT cut, having been re-classified as effort under F14:** the four Round-3 stale-citation
fixes (AC4(d)), and all six fail-open defects in the instrument (AC6) — including un-ignoring the
`kloc_check_runs_on_workspace` test, which this re-base is what finally makes true.


## Dev notes

**The loop.** There is **no `--write`, `--fix` or regenerate mode anywhere** — repo-wide grep is
empty, and `kloc-check` takes only `--config` and `--json` (`main.rs:161-166`). The re-base is a
hand edit. The loop is: `cargo fmt --all` → `cargo run -p xtask -- kloc-check --json` from the
**repo root** (the workspace root is derived lexically as the grandparent of the config path,
`kloc_check.rs:154-161`, so a wrong CWD measures a different tree) → transcribe `per_crate` →
re-run. Every existing grant comment insists on fmt-before-measure because *"fmt is what CI
measures"* (`:273`, `:319`, `:343`).

**Order of operations inside the commit.** Rows and aggregates first, gate green, *then* the prose.
Prose edits shift line numbers; measuring after them makes the diff unreadable.

**Repair the stale self-citation you will otherwise duplicate.** `kloc.toml:195` (the kernel-core
row) cites *"`kloc.toml:407` keeps this crate tight"*. At `5bcc3c76` the file was 422 lines and
`_aggregate_hardfail` sat at 407; the file is now 516 and that row is at **501**. The citation is
stale by 94 lines. This story moves the file again — so repair it to `:501`+delta **or**, better
and consistent with F2, re-cite it by **quote**. `kloc.toml:273` (`maos-audit`) already shows the
robust form.

**What a past re-base looked like.** `08ab6632` ("kloc: Epic-13 retrospective ceiling re-base —
grant 3, refuse 3") is the only dedicated re-base commit in the file's history: **10 insertions, 3
deletions, 3 of 49 rows touched.** It prepended the new claim and chained ` | prior: `; it recorded
its **refusals** inside `_aggregate_hardfail`'s comment rather than as row edits; and it **did not
touch the rule text**. This story is larger and it *does* touch the rule text — deliberately, and
that is the first time. Say so in the commit message rather than letting a reviewer discover it.

**Do not read the ❌ column.** See §2d. The oracle is `over_budget` in `--json`.

**`(unknown:spikes)` / `(unknown:templates)`** are real rows (`:464`, `:465`, budgets 244/111),
keyed with parentheses and quotes verbatim, produced by the fallback at `kloc_check.rs:268-276`
from `spikes/story-11-0-wasm-host/*.rs` (144) and `templates/spirit-rust/src/lib.rs` (11). They
ride the aggregate and need no change.

**Prose that will drift if only `kloc.toml` is amended** (not in scope, but list it in the commit
so the next reader is not misled): `README.md:68,:169-170,:367,:417`; `docs/adr/ADR-038:3,11`;
`docs/adr/index.md:26`; `docs/adr/ADR-041:55`; `docs/adr/ADR-047:34`; `docs/adr/ADR-048:37,65`
(guards `kloc.toml:510` `_docs_site_isolation` — **do not disturb that key**);
`docs/ci-baselines/README.md:83`; `tests/coverage-matrix.yaml:27` (files `kloc-check` under
**ADR-011**, not ADR-038) and `:1092-1096` (NFR-Maint-1 registered with `gates: []`, never flagged
because `check_coverage_matrix_completeness.rs:52-55` inspects `phase == "v1.0"` only).

**Story-file gates.** Flipping this key to `ready-for-dev` puts the file under
`gate_common::governed_story_keys` (`gate_common.rs:54`). At `ready-for-dev` the dev-record and
review-findings gates do not apply (`check_dev_record_completeness.rs:36`,
`check_review_findings_resolved.rs:31` — terminal `done` only) and the model gate exempts
pre-development statuses (`check_dev_model_used_populated.rs:22`). One live constraint: `check-bare-review-findings`
substring-greps every governed story file for its `PLACEHOLDER` const
(`check_bare_review_findings.rs:13`) — read it there and never let that exact string
appear in this file, not even as a quoted example (this paragraph is the reason it is
described rather than shown).

---

## Tasks / Subtasks

> **Funding is ratified (frontmatter).** The open question is no longer *may we* — it is *did the
> numbers survive re-measurement*. T0 is therefore still first and still blocking.

- [x] **T0 — Re-measure before writing a single number.** `git log --oneline -1`; if HEAD moved off
      `03ee0ad8`, `cargo fmt --all`, then `cargo run -p xtask -- kloc-check --json` and **re-derive the
      whole AC1 table**, every `⌈Σupper × 1.3⌉` and `_aggregate_hardfail` included. A grant is a global;
      the ratified table is a measurement, and measurements expire (§5, §8, `14-2c` R1).
- [x] T1 — **AC6(b) FIRST: the `tokei` version assertion + proven-red test.** Nothing below this line is
      trustworthy until the tool producing every number is pinned at the point of use.
- [x] T2 — AC6(c,d,e): the three remaining fail-open holes — unbudgeted crate, absent/non-integer
      aggregate key, quoted/negative budget. One proven-red test each. All tests land in
      `xtask/src/tests/`, which is **uncharged**.
- [x] T3 — AC2: the 20 existing rows to their AC1 ceilings, in their five blocks, house comment format,
      **each carrying its F13 ledger line**. Grep every key first — duplicates hard-error.
- [x] T4 — AC1/AC4(c): zero `maos-cap-registry`, `maos-wire`, `maos-journal`. **Zero, never delete.**
- [x] T5 — AC2: add `maos-egress = 780` and `maos-orchestrator-buffer = 917` into the `:464-498`
      alphabetical block, **above `:512`**. Run both proven-red vectors (a row after `:512`; a quoted
      integer) — note T2(e) converts the second from a silent pass into a named failure, so record the
      before/after.
- [x] T6 — AC3: `_aggregate_hardfail = 170884`, `_aggregate_alarm = 158608`, derivations in their
      comments, the entire `| prior:` chain preserved verbatim.
- [x] T7 — **Gate green here, before any prose.** `kloc-check --json` → `over_budget: []`, exit 0. Save
      the JSON. Prose edits move line numbers; measuring after them is unreadable.
- [x] T8 — **AC6(f): un-ignore `kloc_check_runs_on_workspace`** and delete its expired Epic-5 ignore
      reason. `cargo test -p xtask` green. This is the story's cheapest proof that the ceilings are now
      honestly meetable.
- [x] T9 — AC1 observables: plant `maos-iac` +100 → green; +101 → `over_budget: ["maos-iac 7061 > 7060"]`,
      exit 1; revert both. **Paste both transcripts into the story.**
- [x] T10 — AC2/F2: convert all four `kloc.toml` line-number citations to quote-form — `router.rs:1250`,
      `:1271`, `kloc.toml:212`, `kloc.toml:195` — and repair `:195`'s stale `kloc.toml:407`. Comments are
      free; re-measure to prove the delta is zero.
- [x] T11 — AC4(a): the rule amendment **with its `21-4` expiry clause**, plus the lowering sanction in
      both directions (lines moving out; a budget naming a crate that does not exist).
- [x] T12 — AC4(b): architecture §15.5 clause 2 + the stale clause-4 figures. AC4(c): the OLD→NEW against
      `epic-21…:82`, and `21-5 ≈ −1000` → `−817 tokei / −1028 physical`.
- [x] T13 — **AC4(d): fix all four Round-3 residuals** (`epic-15…:78`, `epic-15…:81`, `epic-18…:32`,
      `epic-20…:47`) and correct Round 3 §5's *"all 11 were fixed"*. Filed-and-unapplied IS the drift.
- [x] T14 — AC6(a): the two message strings + proven-red test. `cargo fmt --all`, then measure the total
      `xtask` delta from T1+T2+T14 together (≈+32) and state it against this story's own row.
- [x] T15 — AC7: re-point and CLOSE D14 and D17; fix the `:91` re-homing map;
      `check-decision-register` exits 0 with the open count down by two.
- [x] T16 — AC5: the pin-protocol restatement, the `21-4` expiry, the `15-1`
      `check-epic-close-coherence` hand-off. **Assert `maos-kernel-core` is byte-unchanged in the diff.**
- [x] T17 — File with the reason named (F14, scope not effort): the `kernel-crate-set` successor and the
      cited-grant-existence gate → `21-4`/D13(b), the latter noting it is impossible until grants are
      machine-readable.
- [x] T18 — Full sweep: `kloc-check`, `check-kernel-baseline`, `check-decision-register`,
      `check-epic-close-coherence`, `cargo test -p xtask`. Record the ratification date in this file.

### Review Findings

> §A6 non-degradable net, run 2026-09-06 against the working tree at baseline `0773ec6e`
> (branch `recovery-lane`, `git diff HEAD`, 12 files, +310/−93). Five layers: Blind Hunter ·
> Edge Case Hunter · Acceptance Auditor · Test Infrastructure Auditor · non-author runtime.
> **Runtime re-verified independently:** `kloc-check` exit 0 `over_budget: []` aggregate 155546 ·
> `check-kernel-baseline` 24472 == 24472 · `check-decision-register` 23 rows / 13 open (−2) ·
> `check-epic-close-coherence` 21 epics PASSED · `cargo test -p xtask` 823 passed, 1 ignored
> (unrelated). T9 reproduced exactly: +100 → 7060/155646 green, +101 → `over_budget:
> ["maos-iac 7061 > 7060"]` exit 1, clean revert. AC1/AC3 arithmetic re-derived from the shipped
> file and reproduces to the digit: Σ per-crate ceilings **182113**, gap **11229**,
> `_aggregate_hardfail` **170884**, `_aggregate_alarm` **158608**, Σ granted headroom **13572**,
> Σ ungranted headroom **13043** (= 20043 − 7000 retired). AC1, AC2, AC3, AC5, AC7 verified met.

> **Both `decision-needed` findings were resolved by round-table on 2026-09-06** (Winston · Amelia ·
> Murat · Mary · Sally · Dana · Grumbal · Paige · John, with Boundary, Vex and Yui summoned),
> under the operator's criterion *per spec and long-term correctness*. Both dissolved into patches:
> neither was a genuine fork. Rulings recorded as R-D1 and R-D2 below.

- [x] [Review][Patch] **R-D1 — the un-ignored test swears on `158608`, the integer F1 already disqualified** [xtask/src/tests/kloc_check_tests.rs:28-34] — `kloc_check_runs_on_workspace` asserts `report.passed && !report.alarm` against the live repo `kloc.toml`. F8 re-based `_aggregate_alarm` to 158608 *so that it fires* "when the workspace begins drawing on the Epic-15 allowance" (`kloc.toml:549`); measured is 155546, so the test is not *at risk* of failing — it is **scheduled** to fail after 3062 of the 13774 funded lines, i.e. **22%** of the authorized draw, in the `cargo test -p xtask` leg, with a name that says nothing about ceilings. `passed` is unaffected (`kloc_check.rs:288`: `passed = over_budget.is_empty()`), so `kloc-check` still exits 0 — the gate warns and the *unit test* blocks. **The collision the room could not un-see:** F1 killed 158608 as a hardfail for exactly this arithmetic ("spendable to only 25% of the grant … a table nobody can spend"), and AC6(f) re-adopted the same integer as a test oracle in the same file. **Ruling — serve both intents, trade neither:** keep `assert!(report.passed)` (which is what AC6(f) actually promises — "the cheapest proof that the ceilings are now honestly meetable", and what the deleted Epic-5 ignore reason was about), remove `!report.alarm`, and give the alarm its **first real control** in a deterministic temp fixture — one line under the configured alarm → `alarm` false, one line over → `alarm` true with `passed` still true. Not spec drift: AC6(f) states the assertion as a *derivation* of why the un-ignore is now possible ("`passed` is true and `alarm` is false, **so** `assert!(…)` holds"), never as the deliverable. Prerequisite finding: the alarm has **no** proven-red control anywhere today — `alarm_fires…` builds a `Report` by hand and asserts a field it set itself, so the parser never runs, and this workspace test had been standing in for that control on a timer.
- [x] [Review][Patch] **R-D2 — clause 2(a) restores the permission; the two new cases become clarification, not substitution** [architecture-maos-minimal-opus/15-full-spectrum-v2-2.md:100] — AC4(a) authorized *adding* the epic-scoped allowance and *sanctioning* two lowerings the MIGRATION RULE forbade. The delivered edit also *replaced* the unconditional "(a) downward at any time" with a two-case conditional. **Decisive asymmetry:** F11's sunset attaches to **(d) only** — "by an operator-ratified epic re-base story until Story 21-4 lands, when this temporary allowance expires" — so the allowance that motivated the amendment is temporary while the narrowing it carried in is **permanent**. Lowering is monotone: it can only make the gate stricter, and no threat model exists for "someone lowered a ceiling for an unlisted reason", so the conditional buys nothing while the *unsafe* direction now has four sanctioned sources. A list of allowed reasons for a harmless operation also goes stale — the next legitimate lowering off the list would need an architecture amendment to do something free, which is the governance-event-per-story cost F11 says the lane cannot survive. **Ruling:** 2(a) reads *"downward at any time — including when measured lines physically move out of a crate, or when a budget names a crate that does not exist (both previously forbidden by the MIGRATION RULE)."* Both current citations still resolve (`epic-21…` 21-4 AC4 and 21-5). **Rider:** 2(b)'s change from *"the tight measured residual (+≤1% slack)"* to *"the CEILING RULE formula"* IS authorized (AC4(b) says amend *to match*; §4 named the ≤1%-vs-2% conflict), but it is a substantive loosening of a ratified constraint that the amended clause does not disclose — add the parenthetical *"(the ≤1% retro slack is superseded by the CEILING RULE's max(100, 2%))"*. Comments are uncharged.
- [x] [Review][Patch] `run()` labels every failure a workspace-aggregate hard fail, so a per-crate-only breach prints a false comparison [xtask/src/kloc_check.rs:63-73] — reproduced: with `maos-iac` one line over, the gate prints `workspace aggregate KLOC hard fail: configured threshold=170884, current=155647` while the aggregate is 15237 lines *under* its threshold. AC6(a) exists to stop the instrument lying about the cause (§2d); the fix made the headline lie too, and more specifically than the string it replaced. Condition the aggregate label on `aggregate >= aggregate_hardfail`; emit a per-crate heading otherwise.
- [x] [Review][Patch] AC6(a) has no proven-red control — the test calls the formatter, never `run()` [xtask/src/tests/kloc_check_tests.rs:83-92] — `aggregate_operator_message_uses_configured_threshold` invokes `format_aggregate_notice("hard fail", …, 11)` with a hand-written `current` and never captures `run()`'s stderr. Reverting either call site at `kloc_check.rs:58-61` / `:70-73` to the old hardcoded text, or wiring the alarm branch to `aggregate_hardfail`, leaves it green. `assert!(!message.contains("20 KLOC"))` is trivially true for any output of that function.
- [x] [Review][Patch] AC6(b) has no proven-red control at the call site [xtask/src/tests/kloc_check_tests.rs:74-80] — `tokei_version_mismatch_is_named` exercises only `validate_tokei_version` with a supplied string. Deleting the whole subprocess probe at `kloc_check.rs:199-208` restores the unpinned behaviour with the suite still green, which is the regression F7b was filed to prevent. Cover it with a temp dir prepended to `PATH` holding a fake `tokei` that prints a mismatched version — no production seam needed.
- [x] [Review][Patch] `_aggregate_hardfail` threshold validation has no proven-red control [xtask/src/tests/kloc_check_tests.rs:129-145] — both fixtures in `missing_or_non_integer_aggregate_threshold_is_named` keep `_aggregate_hardfail = 200` valid and vary only `_aggregate_alarm`. Restoring the old `unwrap_or(20000)` for the *blocking* key alone leaves the test green. Add the two mirrored fixtures.
- [x] [Review][Patch] Five `kloc.toml` line-number citations were invalidated by this diff, and F2 claims there were only four [xtask/kloc.toml:222, :281, :363, :370, :504] — the "must never block a correctness or compliance repair" clause moved from `:84-87` to `:86-89`, so `:363` and `:504` (the audit trail for the two 14-2c correctness-repair grants — the *same* clause `router.rs:1250`/`:1271` were converted for) now cite different text. AC4(a) inserted two lines into the rule block, so the "never silently re-run to fit" sentence moved from `:65` to `:67` and now falls *outside* the `kloc.toml:60-65` range cited at `:222`, `:281` (`maos-iac`) and `:370` (`maos-bin`). The file holds eleven such citations, not four. Convert these five to quote-form — comments are not tokei `code`, so the cost is zero, which is F2's own argument.
- [x] [Review][Patch] `_aggregate_alarm > _aggregate_hardfail` is accepted silently, so an alarm that can never fire passes [xtask/src/kloc_check.rs:158-160, :273] — verified: with `_aggregate_alarm = 999999` the gate exits 0 with `alarm=false, passed=true`. That is the dead-signal state F8 exists to end, reachable by a one-character typo. Reject the inversion where the two thresholds are read.
- [x] [Review][Patch] Budget-shaped keys nested in a TOML table bypass the new integer and non-negative validation [xtask/src/kloc_check.rs:174-177] — `value.is_table()` skips `[in_progress_decomposition]` wholesale, so `maos-egress = "780"` or `= -1` appended after `:561` reaches neither check; for a crate that does not yet exist the AC6(c) measured-crate check has nothing to catch either, and the gate stays green with the budget absent. The table holds only `phase_N = { … }` inline tables, so rejecting integer/quoted-integer keys inside it is additive and does not disturb the text `check_epic_6_bridge.rs:1778-1786` greps.
- [x] [Review][Patch] The `tokei --version` probe does not run from `workspace_root` while the measurement does [xtask/src/kloc_check.rs:199-202] — `:235` sets `.current_dir(workspace_root)` for the measuring call; the version probe has no `current_dir`, so a cwd-sensitive `PATH` entry can approve 14.0.0 from one binary and measure with another. The comment at `:196-197` already claims the probe runs from the workspace root. One-line fix.
- [x] [Review][Patch] §4 and F6 cite architecture clause 2(a) by two quotes this same commit deleted [15-2-kloc-ceiling-rebase.md §4, F6] — both cite *"downward at any time"* and *"New crates minted by extraction get initial ceiling = measured LOC at extraction"*; neither phrase survives the AC4(b) amendment. Re-quote against the amended clause. This is the precise hazard F2/R6 was invoked to close, committed in the act of closing it.
- [x] [Review][Patch] F3 states the `maos-cohort` grant as **+350**; §5, the AC1 table and the shipped row are **+390** [15-2-kloc-ceiling-rebase.md F3] — `⌈300 × 1.3⌉ = 390`, and `kloc.toml` ships 5857 → 6247. The ratified decision register carries a stale figure for the decision it ratifies, which is §6's own failure mode. Correct F3 to +390.
- [x] [Review][Patch] Two claims this story disproved are still live in `epic-15-foundations-w0.md` [epic-15-foundations-w0.md:69, :71] — `:69` still records `maos-control` (697) and `maos-shell` (100) as "`no change` … not re-based", which §6 row 10 marks FALSE and which the epic's *own* refined table at `:106-107` contradicts ("**Was `no change` — FALSE**"), and `kloc.toml` re-bases both (1500→1791, 405→500). `:71` still asserts `check_epic_close_coherence.rs:38` "keeps `prior:` verbatim", which §1's table marks ❌ FALSE (a doc-comment analogy in a gate that never opens `kloc.toml`). AC4(d)'s four *named* residuals are genuinely fixed in `0773ec6e`; these two are adjacent text the amendment pass left behind.
- [x] [Review][Patch] Dev-record baseline and AC6 cost figures do not match the tree [15-2-kloc-ceiling-rebase.md Debug Log, AC6] — the Debug Log records HEAD `03ee0ad8`, but `git log --oneline -1` is `0773ec6e`; `git diff --name-only 03ee0ad8..0773ec6e` is documentation-only so no arithmetic moves, yet T0 exists precisely to record the re-measurement HEAD. Separately AC6 states the instrument repair as "≈+32 lines … 42169 against a 42353 ceiling" while the measured cost is **+48** (41953 → 42001, independently reproduced) — 42185 at the top of the row's booked draws. `kloc.toml` already records +48 honestly; reconcile AC6 and the story's `xtask` row to it.
- [x] [Review][Patch] The ❌-column path keeps the pre-repair lenient parsing that F14 booked as in-scope effort [xtask/src/kloc_check.rs:66, :334-338] — F14 lists "the lying ❌ column" among the six items pulled back IN, but no AC covers it and no edit landed: `get_budget(...).unwrap_or(0)` at `:66` and `as_integer().map(|v| v as u64)` at `:337` still bypass `required_nonnegative_integer`, and `get_budget` re-reads and re-parses `kloc.toml` once per crate. The lie is now *unreachable* — AC6(c) returns `Err` before `run()` can render a budget-less crate — so the residual is stale, inconsistent code, not a live defect. Route the column through the validated `budgets` map already carried in `Report`.
- [x] [Review][Patch] `maos-manifest`'s ledger line states `Grant = 260` against its own `(+160)` with no reconciliation [xtask/kloc.toml] — every sibling row whose Δ differs from its grant carries the qualifier (`maos-bench`: "Grant = 260 over measured 1531"; `maos-mcp`: "Grant = 195 over measured 999"). The number is correct (260 over measured 4214); the phrasing is the one a future story copies forward wrong.
- [x] [Review][Defer] Pre-existing stale citations into `kloc.toml`, not introduced by this diff [xtask/src/check_decision_register.rs:12, xtask/kloc.toml:232] — deferred, pre-existing. `check_decision_register.rs:12` cites `xtask/kloc.toml:208` as one of two `epic-14-preflight-decisions` mentions, but those sit at `:195`/`:251`/`:268` at HEAD (now `:199`/`:258`/`:275`) — the citation was already wrong before this story. `kloc.toml:232` cites `` `:551` ``, which did not exist when written (the file was 516 lines). Both are outside this story's declared four-citation scope; they belong with `21-4`'s instrument unification, which is where machine-readable grants land.


## Dev Agent Record

### Agent Model Used
`openai-codex/gpt-5.6-sol`

### Debug Log References
- 2026-09-06 baseline: **HEAD `0773ec6e`** ("15-2-kloc-ceiling-rebase: spec landed, and the epic amended to match"). ⚠ **CORRECTED AT REVIEW (R-P12): this line originally recorded `03ee0ad8`, which was HEAD when the story was authored, not when it was implemented.** `git diff --name-only 03ee0ad8..0773ec6e` is documentation-only (5 files, all under `_bmad-output/`), so every measured number reproduces byte-for-byte at either commit and no arithmetic moves — but T0 exists to record the re-measurement HEAD, and recording the wrong one is the defect T0 was written to prevent. tokei `14.0.0`; `kloc-check --json` reproduced aggregate 155498 and the pre-story breaches.
- Fail-closed red/green vectors: version mismatch, missing budget, missing/non-integer aggregate thresholds, quoted/negative budgets, and configured operator-message threshold.
- Placement vectors: a row after `[in_progress_decomposition]` remained invisible; a quoted budget failed by key after validation landed.
- T9 `maos-iac` +100 transcript (relevant JSON fields, exit 0):
  ```json
  {"passed":true,"alarm":false,"aggregate":155646,"per_crate":{"maos-iac":7060},"over_budget":[]}
  ```
- T9 `maos-iac` +101 transcript (relevant JSON fields, exit 1), followed by probe removal:
  ```json
  {"passed":false,"alarm":false,"aggregate":155647,"per_crate":{"maos-iac":7061},"over_budget":["maos-iac 7061 > 7060"]}
  ```
- Operator smoke vector with `_aggregate_hardfail = 10`: `workspace aggregate KLOC hard fail: configured threshold=10, current=11`; exit 1.
- Final sweep: KLOC aggregate 155546 PASS; kernel 24472 == 24472 PASS; decision register 23 rows / 13 open; epic coherence 21 epics PASS; `cargo test -p xtask` 823 passed, 1 ignored.

**§A6 review closure — 2026-09-06 (non-author, five layers).** 16 patch findings applied, 1 deferred to `21-4`, 2 dismissed; both `decision-needed` items resolved by round-table under *spec fidelity + long-term correctness* (R-D1, R-D2). Independently re-verified after the patches:
- `kloc-check --json` exit 0, `over_budget: []`, aggregate 155593, `xtask` 42048/42353.
- T9 re-run by the reviewer, not copied: `+100` → `maos-iac` 7060, aggregate 155646, exit 0; `+101` → `over_budget: ["maos-iac 7061 > 7060"]`, aggregate 155647, exit 1; probe removed, tree clean.
- **Every new control proven-red by reverting its fix and observing the named test fail:** R-P1 (aggregate-label condition), R-P2 (alarm names its own threshold), R-P3 (version pin at the call site), R-P4 (`_aggregate_hardfail` validation), R-P6 (inversion guard), R-P7 (nested budget row), R-D1 (alarm boundary). Seven for seven; none is a null control.
- Residual fail-open vectors re-probed after patching: quoted budget → named; negative budget → named; deleted `_aggregate_hardfail` → named; `_aggregate_alarm` above `_aggregate_hardfail` → named (was silently accepted); budget row below `[in_progress_decomposition]` → named (was silently inert).
- `check-kernel-baseline` 24472 == 24472 PASS; `check-decision-register` 23 rows / 13 open (−2 vs baseline 15); `check-epic-close-coherence` 21 epics PASS.

### Completion Notes List
- Rebased 20 existing crate ceilings, retired three phantom budgets to zero, added two future crate rows, and re-derived aggregate alarm/hard-fail thresholds from measured funding.
- Closed all six KLOC fail-open paths: pinned tokei version, required typed non-negative thresholds and budgets, missing-budget hard failure, configured operator messages, and the unignored workspace gate.
- Converted shipped KLOC policy citations to stable quote form; amended CEILING RULE lowering/re-base authority with Story 21-4 expiry; aligned architecture and successor epics.
- Closed D14 and D17 under Story 15-2; D13(b) retains the kernel-crate-set and machine-readable cited-grant successor work in 21-4.
- `xtask` measured 41953 -> 42001 (+48) at the dev pass, then **-> 42048 (+95 total)** after the review patches, all after `cargo fmt --all`, within the 42353 ceiling. `maos-kernel-core/src` remained byte-unchanged.
- Funding ratification date: 2026-09-06 (Lunarpulse).

### File List
- `_bmad-output/implementation-artifacts/15-2-kloc-ceiling-rebase.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/15-full-spectrum-v2-2.md`
- `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md`
- `_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md`
- `_bmad-output/planning-artifacts/epics/epic-21-multi-host-on-live-substrate-w6.md`
- `_bmad-output/planning-artifacts/sprint-change-proposal-2026-09-05-round3.md`
- `crates/maos-a2a-core/src/router.rs`
- `xtask/kloc.toml`
- `xtask/src/kloc_check.rs`
- `xtask/src/tests/kloc_check_tests.rs`

### Change Log
- 2026-09-06 — Implemented the operator-ratified Epic-15 KLOC re-base, fail-closed gate repairs, policy/source alignment, decision closures, and complete validation sweep.
