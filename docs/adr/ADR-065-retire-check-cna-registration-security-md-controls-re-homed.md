---
Status: ACCEPTED — ratified 2026-09-07 under Story 15-3 (`15-3-single-phase-source-and-exit-command-check`), decision F1/F2. Recovery-lane Epic 15 (Foundations, W0).
Gate: removal verified by `xtask check-ship-gate-completeness` (41 expected gates, `check-cna-registration` absent from all three self-policing sites) + the live successor controls enumerated below, each with its own proven-red test
Decided: 2026-09-07
Accepted-in-PR: <PR_NUMBER>
Revisits: Story 10.3 AC-5 (NFR-Ops-4) — the CNA registration + disclosure-pipeline gate; Epic 10-3 review hardening of `has_version_table_row` against the `11.0.x` substring false-pass
Supersedes: the `check-cna-registration` xtask gate (module, subcommand, CI job, `v1-0-ship-gate.needs` entry, `gate-registry.toml` `gates` + `[[ship_gate]]` rows, and `EXPECTED_GATES` entry)
---

# ADR-065 — Retire `check-cna-registration`; its two live `SECURITY.md` controls are re-homed into `check-security-md`

**Context.** Epic 15's plan recorded `check-cna-registration` as *"absent-PASS at
`check_cna_registration.rs:54-60`; `docs/compliance/cna-registration.md` absent at HEAD →
vacuous today"*, and retired it on that basis. **That premise is inverted, and it was
measured, not read:**

```
$ git ls-files docs/compliance/cna-registration.md
docs/compliance/cna-registration.md                 # tracked; 92 lines; added 3806d9df (Epic 10-3)
$ cargo run -q -p xtask -- check-cna-registration
check-cna-registration: PASS (CNA doc + SECURITY.md valid)     # exit 0 — the LIVE arm
```

The absent-PASS arm is **not the arm taken**. The gate runs its content-validating path and
asserts three properties, two of which are live controls with **no other enforcer in the
repository**. Retiring it as originally described would therefore have dropped two controls
while reporting that nothing was lost — precisely the *"a claim standing in for a control"*
failure the Epic-12 retrospective named. This ADR ratifies the **retire** path only after
the two live controls have been re-homed and independently proven red.

## Decision — retire the gate, item by item

**Retire the gate** (delete `xtask/src/check_cna_registration.rs`, its `mod` declaration,
its `Commands` variant and dispatch arm, its `discipline.yml` job, its
`v1-0-ship-gate.needs` entry, its `gate-registry.toml` `gates` and `[[ship_gate]]` rows,
and its `check_ship_gate_completeness::EXPECTED_GATES` entry — **`EXPECTED_GATES` last**,
because deleting the const first leaves an orphan CI job invoking a removed subcommand and
clap exits 2). `docs/compliance/cna-registration.md` is **KEPT** in tree: it is NFR-Ops-4
evidence, linked from `SECURITY.md`, and its retirement is not proposed here.

Every property the gate asserted, and where it lives now:

| # | Property the retired gate asserted | Live successor | Status |
|---|---|---|---|
| (a) | `docs/compliance/cna-registration.md` exists and is non-empty | **none — deliberately not re-homed** | see below |
| (b) | `SECURITY.md` carries no `<TO-BE-PUBLISHED>` GPG-key placeholder | `check-security-md` (`GPG_PLACEHOLDER` const + the `contents.contains` branch, reported via `Report::failures`) | green at HEAD, proven red by `gpg_placeholder_control_fails_when_placeholder_present` |
| (c) | `SECURITY.md`'s supported-versions table has a real `1.0.x` **row** | `check-security-md` (`V1_TABLE_TOKEN` const + `has_version_table_row`, reported via `Report::failures`) | green at HEAD, proven red by `version_row_control_fails_without_1_0_x_row` |
| (c′) | (c) must not be satisfied by the `11.0.x` **substring** | `check-security-md` — `has_version_table_row` is copied faithfully: it requires a markdown table row whose CELL EQUALS the token after trimming whitespace and backticks, never a `contains()` | pinned by `version_row_control_rejects_11_0_x_substring_row` |
| (c″) | (c) must not be satisfied by a prose mention outside a table | `check-security-md` | pinned by `version_row_control_rejects_prose_mention_outside_table_row` |

**Property (a) is dropped on purpose, and it is the only thing that changes.** The CNA
document is *static evidence*, not a moving control surface: a gate asserting that a tracked
92-line file is non-empty catches a class of failure (`git rm` plus a passing CI run) that
`check-ship-gate-completeness` and review already cover, and it was the one property whose
subject is the evidence artifact rather than the enforced policy. It is recorded here rather
than left implicit.

**Why `check-security-md` is the right home and not merely a convenient one.** It already
reads the same file, so no new I/O, no new module, and no second reader of `SECURITY.md` is
created. Critically, it could NOT have absorbed these controls implicitly: its own doc
comment records that it is *"intentionally header-text-based (not regex-rich) so that prose
evolution within sections does not break CI"*, and its `REQUIRED_SECTIONS` asserts **only
four H2 headers**. A `SECURITY.md` with all four headers, a `<TO-BE-PUBLISHED>` placeholder
and no `1.0.x` row passed `check-security-md` before this ADR. The controls are therefore
carried in a NEW `Report::failures` vector rather than overloaded into `missing_sections`,
so the section taxonomy and the content controls stay independently diagnosable, and
`passed == missing_sections.is_empty() && failures.is_empty()` — a tripped content control
changes the command's exit status, because a ported control that cannot fail the command is
not a control.

The retired gate's own evidence document names the regression these two controls exist to
catch, at `docs/compliance/cna-registration.md`: *"`SECURITY.md` regresses (placeholder
returns, version table drops `1.0.x`)."* That sentence is now enforced by
`check-security-md` rather than by `check-cna-registration`.

## Rationale

1. **Retire ≠ drop a control.** Each retired property maps to a live successor with its own
   proven-red test, or is explicitly declared dropped with its reason — the ADR-043 pattern
   this ADR copies deliberately (ratified ADR, item-by-item successor table, an explicit
   *"no control goes dark"* claim, and a `# RETIRED <date> by Story 15-3 per ADR-065`
   comment at each removal site).
2. **A green that could not have been red is not evidence.** The re-home was verified by
   reverting each control against a fixture and watching the NAMED test fail, not by
   observing that the suite still passed. Six tests were added (5 → 11 in the module);
   every red-arm fixture carries all four required H2 sections and asserts
   `missing_sections.is_empty()`, so a failure is attributable to the content control alone.
3. **`invariant-lock` never fires on `gate-registry.toml`.** The review sentence at the top
   of that file (*"invariant-lock review"*) is a **prose convention**, not a machine check:
   `xtask/invariants/lock.toml` maps `I1..In → docs/invariants/*.md` only. ADR-043 (Story
   8.16) is the one discharging precedent for that convention, so this ADR discharges it the
   same way rather than treating the comment as satisfied by prose.
4. **Numbering.** Story 15-5 mints ADR-060..064 and lands *after* this story, so the next
   free number is **065**. Taking it here rather than waiting for 15-5 preserves the epic's
   own dependency ordering (15-2 → 15-3 → 15-1 → {15-6, 15-5} → 15-4).

## Consequences

- `check-cna-registration` is absent from all three self-policing directions
  (`EXPECTED_GATES`, `v1-0-ship-gate.needs`, `gate-registry.toml`), so none of them reds and
  no orphan CI job invokes a removed subcommand.
- `check-security-md` is now a content gate as well as a taxonomy gate. Anyone editing
  `SECURITY.md` must keep the supported-versions table's `1.0.x` row and must not
  re-introduce the GPG placeholder. Its two pre-existing pass-arm test fixtures were
  extended with the now-required `1.0.x` row; their section-taxonomy assertions are
  unchanged.
- `docs/compliance/cna-registration.md` remains as NFR-Ops-4 evidence and is no longer
  gate-validated. Its `SECURITY.md`-regression sentence now points at `check-security-md`.
- `tests/coverage-matrix.yaml`'s `NFR-Ops-4` row lists `check-security-md` alone, and its
  notes no longer claim a CNA gate. Removing the `gates` array entry while leaving a
  coverage-matrix row referencing a retired gate fails **open** today (`mode: warning`) and
  becomes a red when Epic 20 AC5 flips the mode to `error` — so both were done in the same
  commit.
- **No control goes dark.** Properties (b) and (c) are enforced at HEAD by
  `check-security-md`, each with a proven-red test; property (a) is the single declared
  reduction and is named above.
