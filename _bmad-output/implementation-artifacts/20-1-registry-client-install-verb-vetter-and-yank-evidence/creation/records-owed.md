# 20-1 creation — records owed

Rulings: none yet for this story's rows. D01 A / D02 A / D03 A / D04 A (operator, 2026-10-30) govern; D09, D12 and
D-20-1-01…11 are PROPOSED. Line numbers are at `a4c00897` unless marked "post-edit". Shared files
(`epic-20-ship-it-w5.md`, `deferred-work.md`) are edited ONLY by the orchestrator from this list.

## Applied at creation (Create201 owns these files this session)

| # | File:line | Edit |
|---|---|---|
| A2 | `sprint-status.yaml:1` `last_updated` | **not edited** (shared line; two creators run concurrently) — owed to the orchestrator: `2026-11-01` |
| A1 | `_bmad-output/implementation-artifacts/sprint-status.yaml:296` | `20-1-registry-client-install-verb-vetter-and-yank: backlog` → `ready-for-dev` with the creation note; the prior note kept after `WAS:` |
| A0 | `20-1-registry-client-install-verb-vetter-and-yank.md`, `20-1-…-evidence/creation/**` | new |

## Owed to the orchestrator now (E20 cite corrections, rule 9; OLD→NEW detail in `cites-a4c00897.md`)

| # | E20 line | Edit |
|---|---|---|
| E1 | `:71` (stories row) | `kernel-Δ 0` → `kernel-Δ R = KQ1 + P +78 phys / +68 tokei PROPOSED (FLAG-Winston, D04 A)`; append "created 2026-11-01 @a4c00897: exit L2/L4 runnable, L5 rc 1, L7 rc 1, L8 rc 2, L9 `unknown spirit`; forecast +810…+1,686 tokei; split proposed (D-20-1-01 B)" |
| E2 | `:96` (Closes · Δ) | `kernel-Δ 0 · maos-cli +250–400 · maos-spirit-cli +230–400 · maos-registry +230–440 · maos-bin +60–120` → "kernel R +78 / +68 measured (PROPOSED); envelope in the story file (D-20-1-11)" — historical figures kept as provenance |
| E3 | `:98` | client wiring `main.rs:3268-3310` → `:3281-3320` (switch `:3251-3361`); import `subcommands.rs:1437-1455,1465-1501` → `:1470-1496,1498-1519` (summary `:1521-1542`) (C3, C6) |
| E4 | `:100` | `main.rs:276-339` → `main.rs:273-347`; `admission.rs:241-253` → `:247-253`; "Kernel route awaits D04, not a zero grant" → "D04 A RULED; priced at creation: R = KQ1 + P +78 / +68 (story §Kernel)" (C9, C10) |
| E5 | `:104` (AC1) | `:78-81` → `:77-80`; `storage.rs:89-96` → `:108-123`; `main.rs:7693-7701` → `:8980-8988`; `release.yml:44` → `:76-90`; `publish.rs:267-272` → `:266-271`; `main.rs:2918-2948` → `:3281-3320`; `cli.rs:803-870` → `:915`; `:336-363` → `cli.rs:44,427`; `subcommands.rs:1209-1210` → `:1616`; `:1073-1083` → `:1479-1489`; `admission.rs:196-208` → `:275`; `:1092-1112` → `:1498-1519`; `:1116-1129` → `:1521-1534`; `main.rs:7694-7697` → `:8981-8985` (+ eval sidecar `:821`); `maos-shell/src/lib.rs:176-200` → `:506-531` (C12–C25) |
| E6 | `:105` (AC2) | test callers `admission.rs:1219` → `:1248`; `vetting_attestation_gate.rs:176` → `:184` (C26) |
| E7 | `:106` (AC3) | `main.rs:305-339` → `:311-346`; `:316-323` → `:323-330`; `:328-338` → `:335-345`; sink `:281-300` → `:288-308`; `operator_config.rs:12-26` → `:13` (C29, C31) |
| E8 | `:107` (AC4) | `manifest.toml:8` → `:9`; `publish.rs:111-118` → `:115-120`; `main.rs:136-137` → `:142-143` (C32, C33) |
| E9 | `:29` | `main.rs:8854-8861` → `:8980-8988`; `cli.rs:893` → `:915`; `main.rs:1896-1948` → `:1896`; hello-only `:4251-4293` → `:4309-4440`; `lib.rs:509-524` → `:506-531`; `main.rs:6533-6534` → `:6607-6608` (X1–X5) |
| E10 | `:36`, `:38-40` | `main.rs:7693-7701` → `:8980-8988` (measured functional); `publish.rs:267-272` → `:266-271`; `manifest.toml:8` → `:9`; `cli.rs:803-870` → `:915`; `subcommands.rs:1209-1210` → `:1616`; `main.rs:1679`/`:1730-1737` → `:1896`; `lib.rs:164,176-200` → `:506-531`; `main.rs:7694-7697` → `:8981-8985` (X6–X9) |
| E11 | `:53` | "The 20-1 quarantine seam … need measurement" → "20-1 quarantine seam measured at creation (R +78 / +68, PROPOSED)" |

`deferred-work.md` is **not** edited at creation: closures are landing records (AC13); the owner lines stay `20-1`.

## Records owed after the rulings (not now)

- Story: replace "PROPOSED" with the ruled option per row; rewrite only the clauses marked with that row; frontmatter `kernel_grant`/`kloc_grant`; change log.
- FLAG-Winston decision record `20-1-flag-winston-decision.md` for the ruled kernel bundle (precedent `20-3c-flag-winston-decision.md`), stating ADR-069's room (R reaches 21079 measured / 21147 ceiling, under 22000).
- Under **D-20-1-01 B** (split): B1 tracker — retire `20-1-…` and add `20-1a-registry-publish-install-exit-path` and `20-1b-vetting-yank-policy-and-quarantine` (`sprint-status.yaml:296`); B2 E20 — stories-table row `:71` and §20-1 heading `:94` split at the Part boundary; B3 `deferred-work.md:37,56,1093,1147,1264` owner → `20-1b-…`; B4 `docs/invariants/I5.md:4,11`, `docs/invariants/README.md:33` and `tests/coverage-matrix.yaml:371` → `20-1b`; B5 this story file splits at "Part B" (AC6–AC13, D-20-1-02/03/06–10, Part-B envelope column) into `20-1b-…md` sharing this evidence directory; 20-1a re-asks nothing kernel.
- Under D-20-1-01 A: none of B1–B5; the tracker note records the three landings.
- Under D-20-1-03 B: `deferred-work.md:1264` re-dispositioned ownerless (rule 10) with the price (+10 / +9, `p-poller-edge-trigger.diff`).
- Under D-20-1-04 B/C: E20 exit block `:23-24` reorder (init/boot before install) and AC1(c) text.
- Under D-20-1-02 D (non-kernel only): FR13 stays `asserted`; D04 would need re-ruling (it is RULED A).
- Under D09/D12 rulings: their 20-1 consequence sentences in the story; nothing else here.
- `deferred-work.md:1147` and `:1264` gain a "priced at 20-1 creation" sub-bullet (`kq1-…diff`, `p-…diff`); `:37` gains "closure shape D-20-1-09".
- Tracker `sprint-status.yaml:296`: append the ruling ask id.

## Records owed at landing (not now)

- `xtask/kloc.toml:264` (kernel, L3), `:688` (maos-registry), `:696` (maos-spirit-cli), `:489` (maos-cli), `:517` (maos-bin), `:691` (maos-shell), `:481` (maos-compliance), `:425` (maos-domain), `:339` (xtask), `:718` (aggregate, measured + 1) — exact after `cargo fmt --all` / `kloc-check --json`, HISTORY rows citing this key.
- `xtask/kernel-core-baseline.toml:505` (+ HISTORY, content hashes `revocation/applier.rs`, `revocation/poller.rs`, `capability/mod.rs`, `scheduler/scheduler_loop.rs`); `docs/ci-baselines/kernel-surface-*.json` only if `check-service-boundary` reports the new pub fns.
- `deferred-work.md:37,56,1093,1147,1264` → `CLOSED <date> by 20-1 @<commit>` with the witness, or rule-10 re-owned; `routing-audit-2026-10-22.md:43` → owned/witnessed by 20-1 @<commit>.
- `tests/coverage-matrix.yaml:366-371` (FR13), `:617-626` (FR35, FR36), `:627-632` (FR37 machinery note), `:841-846` (FR59); `docs/invariants/I5.md:4,11`; ADR-056 §1 (`:24,95-101`); `architecture-maos-minimal-opus/4-kernel-design.md:661`.
- E20 `:71` stories row → done note; `:27` "20-1 creates `epic-20-exit`" → enrolled with its run id.
