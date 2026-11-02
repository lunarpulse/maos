# 20-2 creation — records owed

Rulings in force: D01 A / D02 A / D11 A (2026-10-30); D09 A confirmed and D-20-2-01…14 RULED exactly as recommended
by Lunarpulse on 2026-11-02 (ask `s202`). D-02 A remains conditional on T0 #1, C if red. The original creation
line numbers below are at `a8af3215` unless marked "post-edit"; they are provenance, not current offsets. The
shared records listed in §A/§B have now been applied or explicitly assigned to implementation landing, as
accounted below. Story and tracker statuses remain `ready-for-dev`; no implementation closure is claimed.

## Applied at creation (Create202 owns these files this session)

| # | File:line | Edit |
|---|---|---|
| X1 | `_bmad-output/implementation-artifacts/20-2-deb-airgap-docker-and-formula-corrections.md` | new story, `ready-for-dev` |
| X2 | `_bmad-output/implementation-artifacts/20-2-deb-airgap-docker-and-formula-corrections-evidence/creation/` | this directory |
| X3 | `_bmad-output/implementation-artifacts/sprint-status.yaml:298` | `backlog` → `ready-for-dev` with the creation note; the prior note kept after `WAS:` |

Applied 2026-11-02: `sprint-status.yaml` `last_updated` is `2026-11-02`.

## §A — epic corrections applied to `epic-20-ship-it-w5.md` (2026-11-02, ask s202)

| # | E20 line | Edit |
|---|---|---|
| A1 | `:7` | D09: record the rows201 ruling text (A) that `20-1a-…md:162` cites, or strike that record — the two disagree (C19) |
| A2 | `:73` (stories row) | "kernel-Δ 0 · xtask +0–30 · CI/packaging only" → append "created 2026-11-02 @a8af3215 — ready-for-dev; kernel-Δ 0 under the recommended set (D-20-2-01 B priced +38/+32 by reverted prototype); maos-bin −18, xtask +2 measured + renderer +80…+150 [INFERENCE]; D-20-2-01…14 PROPOSED" (C17) |
| A3 | `:114` | `release.yml:71-91` / "20-1 adds registry and publisher" → done by 20-1a (`release.yml:71-101,155-167`; `release_dry_run.rs:27-44`) (C1); deb rules "source anchors" → "never buildable: no changelog, `rules:29` missing separator" (C2); `proof-contract.md:170-176` → `tests/integration/onb_nfr2_timing.sh:176` (C3); size "unmeasured" → W8 (C4); `main.rs:869-882`, `:1479` → `:873-887`, `:1501-1506` (C5); netns add `-f`/`AF_UNIX` defects (C6) |
| A4 | `:116` | Docker add W3 (C7); recipes add `0.5.0` and the W5 key mismatch (C8); `main.rs:1858-1866`/`:1520-1524` → `:1881,1887-1891`/`:1518-1558,1532-1535` (C9); `main.rs:7173-7211` → `:7360-7364`, `:10454` (C10) |
| A5 | `:118` (AC1) | `release.yml:46` → `:77-78`; `check_mock_not_in_release.rs:29-31` → `:28,30`; `main.rs:1318-1342` → `:1518-1558`; `0.1.0-alpha` → `0.1.0-alpha.1`; `release_verify.rs:26-30` → `:25-28,56-59`; xtask `main.rs:767-768,:1342-1347` → C5; add closure/stub facts (C11–C14) |
| A6 | `:119` (AC2) | "today only the air-gap main has `--version`, `main.rs:1332`" → both mains answer (C15) |
| A7 | `:120` (AC3) | `verbs.rs:182` → `:164,196`; `main.rs:8883` → `:7360-7364,10454`; purge-removes-`/usr/bin/maos` → D09 A contract (C16) |
| A8 | `:47` (operator lane `ops-rc-signed-tag`) | under D-20-2-01 A: "the air-gap variant attached" → "`.deb`/`.rpm` (amd64 + arm64) with `SHA256SUMS-packages` + sig attached; the air-gap deployment witness green on the tag SHA" (C18); unchanged under D-01 B |
| A9 | `:44` | "`container.yml:40-44,:71-80` (20-2 splits build from push)" → add "measured: the only run (36094027288) failed at Docker Hub login, no image built" (W3) |

## §B — ledger and tracker records (applied/dispositioned below; no owner changes or closure)

| # | Row | Sub-bullet |
|---|---|---|
| B1 | `deferred-work.md:1042` | "Measured at 20-2 creation (2026-11-02): published alpha `maos-linux-amd64` 30,538,368 B stripped; shared-target release build 31,362,992 B stripped (tree unknown) — under the 32 MiB bound (`onb_nfr2_timing.sh:176`). Closure owed at 20-2 `done` with the final-tree figure (AC8.3–8.4)." |
| B2 | `deferred-work.md:98` | NFR-Ops-12: "20-2 created 2026-11-02: witness subject is D-20-2-01 (PROPOSED A: the shipped default build under `unshare -n`); the air-gap variant's `run` is a stub (W10)." |
| B3 | `sprint-status.yaml:341` (`ops-rc-signed-tag`) | as A8 (only if D-20-2-01 A is ruled) |
| B4 | `sprint-status.yaml:331` (`ops-brew-tap-and-aur-publication`) | "20-2 attaches rendered `maos.rb`/`PKGBUILD`/`maos.spec` to each release (D-20-2-08 A); publication copies those, never the committed templates." |
| B5 | `docs/runbooks/provisioning-checklist.md:24-25,31` | anchors move with AC5's `container.yml` split (applied by the dev at landing, listed here so nobody re-cites the old lines) |

## §O — operator prerequisites (never faked; none blocks the hermetic legs)

| # | Prerequisite | Needed by | Owner | State at creation (2026-11-02) |
|---|---|---|---|---|
| O1 | Rulings D-20-2-01…14 and D09 confirmation | T0 | Lunarpulse | RULED 2026-11-02 (ask s202); original creation state was PROPOSED |
| O2 | `DOCKERHUB_USERNAME` + `DOCKERHUB_TOKEN` | publication only (AC5.3 skips cleanly without them) | Lunarpulse (`ops-provisioning-secrets-and-accounts`) | absent (`gh secret list`, W4) |
| O3 | `lunarpulse/homebrew-maos` tap + token; AUR account | publication only | Lunarpulse (`ops-brew-tap-and-aur-publication`) | absent (`provisioning-checklist.md:33-34`) |
| O4 | GHCR package visibility (public) after the first push | only if D-20-2-04 A | Lunarpulse | not applicable under B |
| O5 | `RELEASE_SIGNING_KEY` + `MAOS_RELEASE_PUBKEY` | tag-time rendering and signing (already used) | Lunarpulse | present (W4); production pubkey `84cffd59…f40e` read from the published `maosctl` (W5) |

## Ruling-record application — 2026-11-02

| Items | Resolution |
|---|---|
| A1–A9 | Applied in the epic: D09 confirmation; creation/pricing row; corrected C1–C17 anchors and measured facts; signed package/default-build witness operator acceptance; container run W3. A8 follows the ruled D-01 A branch. |
| B1–B2 | Applied to `deferred-work.md`: provenance-labelled binary figures and default-build netns witness; owner unchanged and final-tree closure still owed to development. |
| B3–B4 | Applied to tracker operator rows: deb/RPM plus signed package manifest, tag-SHA deployment witness, publication copies rendered attachments. Epic operator rows agree. |
| B5 | Landing-only anchor refresh retained in story AC5/AC8: `container.yml` has not been split, so `docs/runbooks/provisioning-checklist.md` still correctly names the existing lines. Do not invent future offsets. |
| Story / tracker / grand plan | All 14 decision-table statuses RULED with option/date/ask; ruling paragraphs, grants and changelog retained; shared planning rows updated. |

**Accounting:** §A/§B has 14 items: 13 current recording items applied, 1 explicitly landing-only (B5); 0 outstanding current recording items. O2–O5 remain real operator prerequisites, not claimed provisioned. No source, workflow or landing pins were changed by this recording pass.

