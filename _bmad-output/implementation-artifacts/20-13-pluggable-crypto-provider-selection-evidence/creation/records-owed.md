# 20-13 creation — records owed

At creation: D01 A / D02 A / D11 A (2026-10-30); DECIDE-1 split (2026-10-23); D-20-13-01…16 proposed.
Original line numbers below are at `8c809f9c`; they are provenance, not current offsets (other creators edit the same
shared files concurrently; every edit was a targeted, exactly-once substring replacement on this story's own rows).

**Rulings recorded 2026-11-03 (ask `d_batch4`):** D-20-13-01…16 as recommended. KT3 and the ceilings remain measured requests, binding at dev start; no implementation or landing claimed.

## Applied at creation (Create2013 owns these lines this session)

| # | File:line | Edit |
|---|---|---|
| X1 | `_bmad-output/implementation-artifacts/20-13-pluggable-crypto-provider-selection.md` | new story, `ready-for-dev` |
| X2 | `_bmad-output/implementation-artifacts/20-13-pluggable-crypto-provider-selection-evidence/creation/` | this directory |
| X3 | `_bmad-output/implementation-artifacts/20-13-flag-winston-decision.md` | kernel row KT3 `REQUESTED` (not granted) |
| X4 | `_bmad-output/implementation-artifacts/sprint-status.yaml:315` | `backlog` → `ready-for-dev` with the CREATED 2026-11-03 note; the prior note kept after `WAS:`; `last_updated` 2026-11-03 |
| X5 | `_bmad-output/implementation-artifacts/deferred-work.md:1379` | creation-mapping sub-bullet (AC4.3/AC4.4, D-20-13-06/07); owner and status unchanged |

## §A — epic corrections applied to `epic-20-ship-it-w5.md`

| # | E20 line | Edit |
|---|---|---|
| A1 | `:86` (stories row) | append "created 2026-11-03 @8c809f9c — ready-for-dev; kernel KT3 +1/−1 by reverted prototype (D-20-13-04 PROPOSED); measured bin +188, shell +46, compliance +22, registry +21, xtask +59; secrets/audit 0; L1 kernel-free / L2 kernel" |
| A2 | `:306` (Closes · Δ) | append the same measured figures and "maos-secrets/maos-audit measure 0 (C10, D-10)" |
| A3 | `:308` (amendment) | in place: `main.rs:1896-1948` → `:1906-1913` (+ writer `maos-shell/src/lib.rs:54-155`); `:2190-2191` → `:2218-2219`; candidate root `:2190` → `:2218`; CRL `:3211` → `:3238-3239` (second instance); image smoke `:8202` → `:8403`; imports `:648` → `:649`; later main `:15081/:15342/:15817/:15846` → `:15354/:15615/:16090/:16119` (all test) (C1–C7) |
| A4 | `:310` (Why) | append "⚠ creation @8c809f9c: 3 maos-bin production + 1 smoke + 1 kernel (`security/mod.rs:350`, T3 admission) constructions; 21 direct sign/verify/seal files besides the provider (C8, C9)" — the dated 2026-10-23 text is kept as history |
| A5 | `:313` (AC2) | "and the audit sealed export" → "⚠ the audit sealed export signs, it does not encrypt (`maos-audit/src/sealed_export.rs:296-334`, C10, D-20-13-10)"; capability tokens already routed (C11) |
| A6 | `:314` (AC3) | "`maosctl` posture" → "the boot TL frame read by `maosctl audit query` (D-20-13-14; `maosctl posture` is a write verb, C13)"; "already names `ring`" → "names it in doc only; production contexts are manifest-derived (C14, D:1379)" |

## §B — planning records (applied)

| # | File:line | Edit |
|---|---|---|
| B1 | `epic-20-preflight-2026-10-30.md:86` (envelope row) | "All figures by prototype …; no numeric grant" → creation figures + PROPOSED ceilings (D-20-13-13) |
| B2 | `epic-20-preflight-2026-10-30.md:130` (readiness row) | "**Yes; unmeasured**" → "**Yes; measured** KT3 +1/−1 (PROPOSED), L2 last" |
| B3 | `epic-20-preflight-2026-10-30.md:256-257` (§3.6) | append "⚠ creation @8c809f9c" NEW cites (C1–C9) |
| B4 | `routing-audit-2026-10-22.md:128` (FR48 row) | append the creation measurement and story path |
| B5 | `epics/requirements-inventory.md:617` (FR48 row) | append "created 2026-11-03 @8c809f9c, ready-for-dev; D-rows PROPOSED" |

## Records applied after the rulings (owed; not now)

- Story: replace "PROPOSED" with the ruled option per row; rewrite only the clauses marked with that row; change log;
  frontmatter `kernel_grant` / `kloc_grant` to the granted ceilings.
- `20-13-flag-winston-decision.md`: `REQUESTED` → `GRANTED` (or the ruled alternative / none) with the ruled ceilings.
- Tracker note, E20 `:86`, PF `:86` with the ruled ceilings.
- Under D-20-13-07 C: the D:1379 endpoint half re-labelled ownerless with its reason (rule 10).

## Records owed at landing (not now)

- `xtask/kloc.toml:517` (maos-bin), `:691` (maos-shell), `:688` (maos-registry), `:481` (maos-compliance, pin kept if inside),
  `:425` (maos-domain, if touched), `:339` (xtask), `:264` (kernel, L2), `:718` (aggregate, measured + 1) — exact after
  `cargo fmt --all` / `kloc-check --json`, HISTORY first segment citing this key with `->` (D11 A).
- `xtask/kernel-core-baseline.toml:505` + EOF HISTORY, content hash of `security/mod.rs` (L2).
- `deferred-work.md:1379` → `CLOSED <date> by 20-13 @<commit>` for the landed halves.
- `tests/coverage-matrix.yaml:740-754` (FR48), `:1733-1738` (NFR-Sec-15); operator pages (AC6).
- E20 `:86` stories row → in-progress/done notes.
