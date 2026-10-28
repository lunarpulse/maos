# 19-2 creation — records owed (the creator edits no tracked file but the story and this directory)

**Status 2026-10-28:** E1–E11 and W1 applied by the parent (commit `c80964d8`). After the D-13…20 rulings (asks `w2_19_2_fr51`, `w2_19_2_d20`) the parent authorized this creator to apply T1, T2, E12–E16, R1, A1, A1′ — **APPLIED 2026-10-28** (uncommitted), with D-20 B's capped figures (≤ +93 / +58 → E19 kernel endpoint ≤ 26483 / ≤ 20686) in place of the +74/+46 / 26464/20674 shown in the rows below. Nothing 'at landing' (kloc/kernel re-pins) was applied.

Every substring below was verified present, verbatim, at `d9bd59c4`. "Now" = cite corrections (rule 9), applicable at once. "After ruling" = depends on the operator's D-19-2-xx rulings; apply only the ruled option's text.

## Now — epic `_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md` (§19-2 and the kernel paragraph that cites it)

| # | Line | OLD (substring) | NEW (substring) |
|---|---|---|---|
| E1 | 175 (AC1) | ``the continuous serving loop, `main.rs:5286-5290`; `--once` is one `on_idle` pass + drain, `:5115`)`` | ``the continuous serving loop, announced `main.rs:5469-5475` @d9bd59c4 (was `:5286-5290` @f889b3e4 — corrected at 19-2 creation); `--once` is one `on_idle` pass + drain, `:5298` (was `:5115`); ⚠ the `--epic` dispatch runs inside the topology load `for` (`:4687-5294`) in BOTH modes, before that branch)`` |
| E2 | 175 | ``(`crates/maos-cli/src/cli.rs:776-783`, `--spirit` required; its doc `:777` still says`` | ``(`crates/maos-cli/src/cli.rs:824-831` (was `:776-783`; 19-7 +48), `--spirit` required; its doc `:825` (was `:777`) still says`` |
| E3 | 175 | ``(`cli.rs:785-788`)`` | ``(`cli.rs:833-836`, was `:785-788`)`` |
| E4 | 175 | ``(`main.rs:2270-2271`; door `operator_door.rs:213,263`)`` | ``(`main.rs:2292-2293`, was `:2270-2271`; door `operator_door.rs:213,266`, was `:213,263`)`` |
| E5 | 175 | ``(`spirits/orchestrator/src/lib.rs:219-227`;`` | ``(`spirits/orchestrator/src/lib.rs:483-491`, was `:219-227` — 19-1 +264;`` |
| E6 | 176 (AC2) | ``(the doc at `spirits/orchestrator/src/lib.rs:143-147` stays true)`` | ``(the doc at `spirits/orchestrator/src/lib.rs:317-322`, was `:143-147`, stays true)`` |
| E7 | 176 | ``(`lib.rs:149-157` stays inert)`` | ``(`lib.rs:323-331`, was `:149-157`, stays inert)`` |
| E8 | 177 (AC3) | ``(`prd/functional-requirements.md:54`)`` | ``(`prd/functional-requirements.md:55`, was `:54` — FR20 is `:54`)`` |
| E9 | 178 (AC4) | `` `deferred-work.md:1277` `` | `` `deferred-work.md:1195` (was `:1277`; 2026-10-27 archival) `` |
| E10 | 178 | ``class Spirits are moved into `scheduler.load` (`main.rs:4603-4608`)`` | ``class Spirits are wrapped by `make_spirit_obj` and admitted by `admission::load_admit_start` (topology `main.rs:5163-5281`, standalone `:5696-6104`; the `main.rs:4603-4608` comment was deleted by 19-1) and a topology refuses Butler (`:5251-5256`); a retained handle already exists for a principal-bound Butler, `ButlerSeam::scene_butler` (`spirits/butler/src/lib.rs:320-336`, set `:389-391`)`` |
| E11 | 34 (kernel-Δ paragraph) | ``` `crates/maos-bin/src/main.rs:2270-2271`, through `Orchestrator::drain_next`, `spirits/orchestrator/src/lib.rs:219-227` ``` | ``` `crates/maos-bin/src/main.rs:2292-2293`, through `Orchestrator::drain_next`, `spirits/orchestrator/src/lib.rs:483-491` (corrected at 19-2 creation @d9bd59c4) ``` |
| — | 80, 81, 82, 106 (§R rows R19-13/14/15, P10) | — | **not edited** (history rows of the 2026-10-27 refinement; the 19-1/19-7 creations left §R rows as history too). Their NEW-column cites are superseded by E1–E10. |

## Now — `_bmad-output/implementation-artifacts/deferred-work.md`

| # | Line | OLD | NEW |
|---|---|---|---|
| W1 | 1195 | `` `spirits/butler/src/lib.rs:1039-1087` `handle_option_pick` `` | `` `spirits/butler/src/lib.rs:1486-1563` `handle_option_pick` (was `:1039-1087` @419f051f; corrected at 19-2 creation @d9bd59c4) `` |

## After ruling — tracker `_bmad-output/implementation-artifacts/sprint-status.yaml` (rulings 2026-10-28: D-01 B `w2_19_2_d01`; D-02 B, D-03…11 A `w2_19_2`; D-12 superseded `w2_kloc`; D-13…20 pending)

| # | Line | OLD | NEW |
|---|---|---|---|
| T1 | 288 | `  19-2-fr20-enqueue-door-and-safe-point: backlog  # daemon-side dequeue_at_safe_point caller;` | `  19-2-fr20-enqueue-door-and-safe-point: ready-for-dev  # ◆ FLAG-Winston. CREATED 2026-10-28 at d9bd59c4, AMENDED at 9ce0e47a; D-19-2-01 B (FR51(a)(d) absorbed, ask w2_19_2_d01), 02 B + 03…11 A (ask w2_19_2), 12 superseded by the E19 kloc raise (ask w2_kloc); D-13…20 PROPOSED (kernel grant request +74 physical / +46 tokei, D-20) — story: 19-2-fr20-enqueue-door-and-safe-point.md; E16-A7: measured kernel figure (#8) + CI freeze spike (#15) before FR51 kernel code \| WAS: daemon-side dequeue_at_safe_point caller;` (rest of the row kept) |
| T2 | 501 | `FR51(a)(d) widen \`19-2-fr20-enqueue-door-and-safe-point\`; verified at the E17 retro 2026-10-24. The work itself is 19-2's.` | `FR51(a)(d) widen \`19-2-fr20-enqueue-door-and-safe-point\`; verified at the E17 retro 2026-10-24. The work itself is 19-2's — OWNED by 19-2 since its creation (D-19-2-01 B, 2026-10-28, Lunarpulse, ask w2_19_2_d01): AC7 (pause fence + Worker freeze + dispatch hold, P99 ≤ 2 s measured) and AC8 (revoke: post-call re-check + Worker stop); ◆ FLAG-Winston kernel request D-19-2-20.` |
| T3 | — | (was: a new key if D-01 A) | **not owed** — D-01 B keeps the work in 19-2 |

## After ruling — epic `_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md` (line numbers at the parent's working tree, 2026-10-28; substrings verified)

| # | Line | OLD | NEW |
|---|---|---|---|
| E12 | 173 (*Closes · Δ*) | `*Closes · Δ:* FR20, FR51(c) (AC3, routed from 16-1), the director option-pick input (AC4, received from 18-4) · kernel-Δ 0 · forecast maos-bin 140, maos-control 60, maos-cli 50, maos-shell 10 against the D19-1 envelope` | `*Closes · Δ:* FR20, FR51(c) (AC3, routed from 16-1), the director option-pick input (AC4, received from 18-4), FR51(a)(d) (AC7/AC8 at story creation — routing audit 2026-10-22 widen, \`routing-audit-2026-10-22.md:40,172\`, absorbed by D-19-2-01 B, 2026-10-28) · ◆ FLAG-Winston kernel-Δ +74 physical / +46 tokei requested (D-19-2-20; FR20 half 0) · forecast maos-bin 140 (+ FR51 45–75 at creation), maos-control 60, maos-cli 50, maos-shell 10 (≈ −7 under D-19-2-09 A) against the D19-1 envelope as raised 2026-10-28` |
| E13 | 152 (Stories row) | `FR20, FR51(c) · kernel-Δ 0 · forecast maos-bin 140, maos-control 60, maos-cli 50, maos-shell 10 (D19-1)` | `FR20, FR51(a)(c)(d) (D-19-2-01 B) · ◆ FLAG-Winston kernel-Δ +74 / +46 requested (D-19-2-20) · forecast maos-bin 140 + 45–75, maos-control 60, maos-cli 50, maos-shell 10 (D19-1, raised 2026-10-28)` |
| E14 | 34 (kernel-Δ paragraph) | `19-1/19-2/19-4/19-6 ZERO` | `19-1/19-4/19-6 ZERO; 19-2 ZERO for its FR20 half and +74 physical / +46 tokei for FR51(a)(d) (D-19-2-01 B; FLAG-Winston grant D-19-2-20, measured by reverted prototype p345 at 19-2 creation)` (the rest of the sentence — the FR20 drain cites — kept) |
| E15 | 50 (derived endpoints) | `26379 → **26390** physical and 20619 → **20628** tokei` | same + ` — and, if D-19-2-20 A is ruled, → **26464** physical / **20674** tokei (19-2 FR51 fence/freeze/re-check, +74 / +46)` |
| E16 | 159 / 212 (order) | — | append to the order sentence: `19-2 and 19-3 are both FLAG-Winston (kernel); the second to land re-pins \`kernel-core-baseline.toml:505\` / \`kloc.toml:264\` on top of the first` |
| — | 38-48 (envelope) | — | **already applied by the parent** (maos-bin `31353`, aggregate `191248` present at `:40`, `:48`); no aggregate raise is owed for the kernel grant (19-2's aggregate use 315–415 incl. kernel +46 fits the 535 left — story cumulative table). |

## After ruling — `_bmad-output/planning-artifacts/epics/requirements-inventory.md`

| # | Line | OLD | NEW |
|---|---|---|---|
| R1 | 600 | `\| FR51(a)(d) bounded interruption, revocation; FR51(c) \| 16-1 \| 19-2 (with FR20) \|` | `\| FR51(a)(d) bounded interruption, revocation; FR51(c) \| 16-1 \| 19-2 (with FR20) — (c) AC3; (a)(d) AC7/AC8, absorbed 2026-10-28 (D-19-2-01 B) \|` |

## After ruling — `_bmad-output/planning-artifacts/routing-audit-2026-10-22.md` (dated record: annotate, do not rewrite)

| # | Line | OLD | NEW |
|---|---|---|---|
| A1 | 40 | `\| \`19-2\` \| FR51(a)(d) bounded director interruption and revocation (16-1 has no interrupt mechanism); FR51(c) already AC3 \| \`16-1…md:35-42\`; \`sprint-status.yaml:284\` \|` | same row, last cell + ` — ✅ absorbed at 19-2 creation 2026-10-28 (D-19-2-01 B): AC7/AC8` |
| A1′ | 172 | `\| widen \`19-2\` \|` (FR51 row, last cell) | `\| widen \`19-2\` — done 2026-10-28 (D-19-2-01 B) \|` |

## Not edited, flagged

- `epic-17-retro-2026-10-24.md:32` is a dated record; it stays as written.
- `docs/adr/ADR-062-mutating-operator-surface.md:87` (Consumers: 19-2 "queue mutation") is edited by the dev pass under AC5, not here.
- Superseded: the pre-ruling E14 (envelope +36 if D-12 B) — D-12 was superseded by the E19 kloc raise.
