---
story_id: '20.5'
story_key: 20-5-v1-0-rc-tag-and-lts-rule
created: 2026-11-04
context_commit: 73e7afbc
baseline_commit: 73e7afbc
security_sensitive: true
kernel_grant: "NONE REQUESTED — kernel-Δ 0. The ruled `[cli_wrapper]` schema shape (D-20-5-02 A) measured at 73e7afbc leaves the kernel at 26841 physical / 21042 tokei, set hash unchanged. Priced alternatives are D-row options, never grants: B +2 / +2 (two `#[cfg(test)]` struct literals; FLAG-Winston + ABI row), C +19 / +16 (kernel window check; cumulative 21219 > shared STOP 21209). Any `crates/maos-kernel-core/src` change is STOP + a new D-row."
kloc_grant: "RULED 2026-11-04 (operator, ask d205; D-20-5-16 A; D02 A ×2 measured / ×3 unmeasured), binding at dev start: creation-priced ceilings (STOP above; pins re-fixed exactly at landing; aggregate hardfail = measured + 1): xtask ≤ 45043; maos-manifest ≤ 4656; maos-domain ≤ 10123; maos-spirit-abi ≤ 1048; maos-bin ≤ 34568; aggregate ≤ 193869; kernel 0. Each ceiling is HEAD + allowance and re-bases onto the exact pins landed before this story's dev start (20-5 is the last Epic 20 story); creation pricing is evidence, not a reservation. Every other crate 0."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:67-71`): the release precondition gates signing and publication (a rule-11 control on the release path) and AC3 adds an admission refusal on Worker manifests. Blind and Edge on opus-5.5, Acceptance on opus-5.5, Test Infrastructure on glm-5.3; a manifest change widens the focused set to every reader (`proof-contract.md:48-53`). Creation anthropic/claude-opus-5-5."
---

# Story 20.5: `v1.0.0-rc.1` readiness and the LTS rule

**Shared kernel STOP supersession — 2026-11-04 (ask `halt_grant`).** Historical internal STOP
21209 / 21,209 and the 20-9-only 21230 / 21,230 are superseded by **21250 / 21,250 for the whole
cumulative measured kernel ledger at each landing**. D-20-14-12 C's joint 21250 now equals that shared STOP,
not a separate exception. With ruled 20-11 O4, cumulative measured tokei is **21203 + 12 = 21215**
(physical **27032 + 16 = 27048**). ADR-069 alarm **22000** and envelope **23000** are unchanged.
Older arithmetic/rulings below remain historical; later kernel landers must re-measure and re-base.
For D-20-5-02 specifically, the new cumulative base gives **A 21215**, **B 21217**, **C 21231**.
Thus C is below shared STOP 21250; the frontmatter/Kernel table/cumulative paragraph/D-02 statement
“C 21219 > shared STOP 21209” is historical pricing, not the current STOP test. A remains kernel-free/recommended;
no alternative is newly granted by the STOP change.


Status: in-progress

<!-- Created 2026-11-04 at 73e7afbc (main; `git log -1 -- crates xtask` = 73e7afbc) by bmad-create-story under the lighter measured creation contract (`Create205`). Operator rulings in force: D01 A, D02 A, D11 A (2026-10-30); 20-3a Q3 (2026-10-15: the rc-tag T3 precondition is routed here); 17-1 Q8 (2026-10-07: the `[cli_wrapper]` schema version is owned here); D-18-0-03 (2026-10-24: the deny ignore re-check is owned here). D-20-5-01…16 were ruled as recommended by the operator on 2026-11-04 (ask d205); grants bind at dev start and re-base onto the then-landed exact pins. No cargo ran: cites re-read at HEAD (`creation/cites-73e7afbc.md`); kernel and kloc by the prebuilt gate binary on `git archive` copies (`creation/measurements.md`); ledger, tag, secret and run facts read-only (`creation/cites-73e7afbc.md` §O). Stale E20 §20-5 cites corrected in place (`creation/records-owed.md` §A). -->

**Status rationale — ready-for-dev, not blocked.** L1 (AC2–AC7) is hermetic and kernel-free and needs no other story.
L2 (AC1, AC8, AC9) starts only when every other Epic 20 key is `done` (D-20-5-01 B) — that is sequencing, not a
blocker on starting L1. Preflight places this story last (PF:144): "20-5 last is rc readiness, not permission to tag."

> **The capability:** *the operator runs one command on a candidate SHA and is told whether the tree is a release
> candidate — version, stability contract, Epic 20 closure, provisioning, supply-chain ignores — and the release job
> refuses to sign a tag whose SHA lacks a complete 24 h T3 fuzz chain; STABILITY.md states the LTS rule NFR-Maint-6
> actually makes, and a `[cli_wrapper]` Worker manifest carries a schema version before the LTS clock freezes it.*

**Closes:**
- Epic-20 §20-5 (E20@HEAD:211-220) as amended 2026-10-30 (PF §3.2): epic AC1 → AC1 (+ AC5); epic AC2 → AC2.
- NFR-Maint-6 (`prd/non-functional-requirements.md:137`; owner row `deferred-work.md:107`) — the v1.0 LTS rule is
  published and generated, the clock is defined to start at `v1.0.0` (AC2). The commitment itself runs from the GA tag
  (`ops-ga-tag-after-holds`).
- The `[cli_wrapper]` schema-version row `deferred-work.md:1175` (17-1 Q8, ruled 2026-10-07) (AC3).
- The rc-tag T3 precondition routed by 20-3a Q3 (tracker `sprint-status.yaml:305`; `fuzz-prerelease.yml:5`;
  routing audit `:131` NFR-Sec-5) — the release job refuses; producing the chain is the operator's dispatch (AC4).
- `deferred-work.md:1298` (D-18-0-03 deny ignore re-check) — re-checked and recorded; the row stays open while the
  upstream trigger is unmet (AC7).
- `deferred-work.md:1186` (published examples the parser refuses) and `:1209` (`Family::Variant` error routes) —
  "published docs truth before the RC" (AC6; D-20-5-11).

**Does NOT close, and says so:**
- The tag, signing and publication — `ops-rc-signed-tag` (`sprint-status.yaml:349`); the T3 chain itself — an
  operator dispatch; GA and the LTS clock — `ops-ga-tag-after-holds` after RELEASE-HOLDS Holds 1–2.
  **Dated note 2026-11-12:** GA is no longer gated on Holds 1–2 (explicit claim boundaries); the key
  `ops-ga-tag-after-holds` now means "cut `v1.0.0` once the engineering exits are green", an operator action (the
  signed tag needs the key holder).
- The perf-ledger `PASS` read in `check_release_precondition.rs` — 20-7 AC11.2 (`20-7-…md:232`), L2. This story shares
  the file and serializes with it (Overlap).
  **Dated note 2026-11-12:** L2 landing 2 (Dev205L2) adds that read here as `--perf-verdicts` (FAIL/INDETERMINATE
  always refused, ABSENT only under `--perf-absent-ok`, passed by `release.yml` for kind `rc` only; GA still needs
  every row PASS). The 20-7 L2 binding phase itself stays an unclaimed row needing a bare-metal runner.
- NFR-Maint-6's v1.5 two-year term (a v1.5 decision); the architecture's 72 CPU-hour per-target pre-GA fuzz floor
  (`5-spirit-abi.md:193`, `check_fuzz_floor.rs`) — a GA gate, not an rc gate.
- `.deb`, air-gap, Docker split — 20-2; this story verifies their presence at L2, never builds them.

## Story

As **the operator about to cut `v1.0.0-rc.1`**,
I want **one readiness command and a release job that refuse an unready SHA, and a STABILITY.md that states the real
LTS rule and the Worker manifest's schema window**,
so that **the release candidate I tag is the tree every Epic 20 story finished, fuzzed for 24 h at that exact SHA, and
the LTS clock later freezes a contract that can still evolve.**

**Contract:**
- Epic §20-5 (E20@HEAD:211-220) as amended 2026-10-30 (preflight rows `:80`, `:126`, `:144-145`, §3.2 `:194-197`) —
  create-story may tighten, not add uncited ACs: AC3 is the amendment's "Include `[cli_wrapper]`
  `manifest_schema_version` compatibility"; AC4 is "T3 24h per-target fuzz chain for exact SHA"; AC5 is "the tree is
  tag-ready"; AC6/AC7 are deferred rows owned by this key.
- Tracker `sprint-status.yaml:305`; E20 confidence rules 2, 4, 6, 9–11 (E20:59-61); proof contract
  `docs/testing/proof-contract.md:10-31` (one final-tree run + one plant per AC), `:33-71`.
- Rule 2: nothing in an AC needs a secret, a paid key, a human, a wall clock or the network; the operator-lane items
  are named in `creation/rc-contract.md` §3 and never simulated.

## Inventory at HEAD — measured, not assumed

Method: every cite read at `73e7afbc` (`creation/cites-73e7afbc.md` C1–C37, O1–O9); prototypes measured on archive
copies (`creation/measurements.md`); no cargo.

| # | Charter item | Disposition | Code / record at HEAD | Measurement | AC |
|---|---|---|---|---|---|
| 1 | Workspace version `1.0.0-rc.1` | **missing** | `Cargo.toml:67` `0.1.0-alpha.1` | C1; every in-tree `min_substrate_version` ≤ `1.0.0-rc.1` under `compare_versions` (C30, read) | AC1 |
| 2 | `release_channel` row | **missing** | `STABILITY.md:21-27`; render `stability_matrix.rs:124-257` | C7 | AC2 |
| 3 | LTS text | **contradicts NFR-Maint-6** | "v1.5 … 2-year … the v1.0 line receives security-only patches for 2 years" (`stability_matrix.rs:215-219`, `STABILITY.md:53-57`) vs PRD `:137` "1-year LTS at v1.0; 2-year at v1.5 …" | C5, C26 | AC2 (D-05) |
| 4 | LTS placeholder "Epic 10" | **stale** | `stability_matrix.rs:221,350,367`; `STABILITY.md:59-60`; tests `:474,:492` | C4–C6 | AC2 |
| 5 | rc → prerelease on GitHub | **already correct** | `release.yml:174` `contains(github.ref_name, '-')` | C8 | retained (AC1.3) |
| 6 | `[cli_wrapper]` schema version | **missing** | `CliWrapperConfig` `manifest.rs:4403-4528` (`deny_unknown_fields`); one strict parse `worker_spawn.rs:610-618`; triple only on `[class]` (`security/mod.rs:279-289,525-573`) | C21–C24, C27–C29 | AC3 (D-02…04) |
| 7 | Release precondition: T3 chain | **missing** | `check_release_precondition.rs:33-96` reads the aggregate only | 0 T3 records in `fuzz-ledger` (O2); one failed dispatch (O3) | AC4 (D-07) |
| 8 | T3 chain timing vs the tag | **conflict** | `fuzz-prerelease.yml:7-10` starts on `v*-rc*`, the same push that `release.yml:2-4,103-174` publishes on | C11, C13 | AC4 (D-07) |
| 9 | Tree-state readiness | **missing** | no command; `tag-procedure.md` is alpha-only (`:5`), step 5 is history (C31) | — | AC5 (D-08, -15) |
| 10 | Deny ignore re-check | **owed; two ignores undocumented** | `deny.toml:15-52`; no row names RUSTSEC-2021-0127 / -2025-0134 | trigger not met (O8) | AC5.1(e), AC7 (D-10) |
| 11 | Published docs truth | **still false** | topology `:38,75` (ko `:39,76`); v3 `:72` (ko `:73`); TierNotGranted `:32-34`; 42 variant routes | C35, C36 | AC6 (D-11) |
| 12 | Release notes | **auto PR list** | `release.yml:171`; alpha body = 6 PR titles | O7 | AC8 (D-13) |
| 13 | Perf `PASS` for the SHA | **20-7 L2** | `check-perf-verdict` landed (C34); no `perf-ledger`, 0 runners (O4) | — | D-09 (not this story's code) |

## Current-tree corrections — read before implementing

Full OLD → NEW: `creation/cites-73e7afbc.md`. Applied in place to E20 §20-5, the tracker row and the preflight
(`creation/records-owed.md` §A).

| Premise (epic / preflight / tracker / ledger) | Fact at `73e7afbc` and consequence |
|---|---|
| "`stability-matrix --check` (`xtask/src/main.rs:595`)"; amendment `:674-679` | `:619-625`, dispatch `:1406-1409` (C2). |
| "`Cargo.toml:64`, `0.1.0-alpha` today" | `:67`, `0.1.0-alpha.1` (C1). |
| "`release.yml` (`:88-94`) gets `prerelease: … '-rc'` (an rc publishes as a full release today)"; amendment `:142-158` | **False/closed:** `:174` already `contains(github.ref_name, '-')`; `:152-174`. Retain, do not narrow (PF:196). |
| "`container.yml` (`:12-14`)"; "`release.yml:75-77`" | `:11-14`; secrets at `:75,133-134,139,151` (C10, C16). |
| Amendment: stale wording "`:221` and `STABILITY.md:59-60`" | also the LTS sentence itself (`:215-219` / `:53-57`) contradicts NFR-Maint-6 (C5, C26) → D-20-5-05. |
| Tracker: "`EAbiTooNew` (`STABILITY.md`, `security/mod.rs:314-333`)" | `security/mod.rs:279-289,525-573` (C23). |
| Epic: "kernel-Δ 0 · xtask +20–40" | Kernel 0 holds only for D-20-5-02 A; xtask **+213 measured** for the recommended bundle (`stability_matrix.rs` alone +29); manifest +26, domain +8, spirit-abi +3, bin +2 (`creation/measurements.md`). |
| Tracker / task: 20-7 L2 operator prerequisites | **O1–O7** (`20-7-…-evidence/creation/records-owed.md:49-55`; O3, O7 are the L1 security correction) (O9). |
| `deferred-work.md:1186` "ko twins" | ko twins exist for `topology.md`, `manifest/v3.md` only; there is no ko `errors/` tree (C35). |
| `deferred-work.md:1209` "18 of 24" variant routes | **42** `Family::Variant` routes in 6 families at HEAD; `expected_distinct_pages` 135 (C36). |
| `deny.toml` comments: every ignore "tracked as deferred work" | RUSTSEC-2021-0127 and RUSTSEC-2025-0134 have no row (C20) → AC7. |
| `tag-procedure.md:34` "`release.yml:79-84`"; step 5 "merge `recovery-lane`" | `:117-122`; `recovery-lane` is already an ancestor of HEAD (C9, C31). |

## Kernel and kloc pricing — measured at creation

**Instrument:** prebuilt `/mnt/build/cargo-maos/debug/xtask` (`check-kernel-baseline --json`, `kloc-check --json`) from
a `git archive 73e7afbc` copy per prototype; the untouched copy reproduces the pins (26841 / 95 files / set hash
`db39bc4a…`; kernel 21042 tokei; xtask 44452; aggregate 193140). Diffs: `creation/prototypes/`; figures:
`creation/kloc-kernel-73e7afbc-and-prototypes.json`.

**Kernel (no grant requested):**

| `[cli_wrapper]` schema shape | Kernel physical | Kernel tokei | Other | Diff |
|---|---:|---:|---|---|
| **A** peel `manifest_schema_version` from the `[cli_wrapper]` table and check it before the strict parse; struct unchanged (**recommended**) | **0 (26841)** | **0 (21042)** | manifest +26, domain +8, spirit-abi +3, bin +2 | `cw-peel.diff` |
| B the same as a `CliWrapperConfig` field (`#[serde(default)]` = 1) | +2 (26843) | +2 (21044) | the two kernel `#[cfg(test)]` literals (`admission.rs:393`, `lifecycle.rs:66`) move the pin and content hash; 8 test literals elsewhere | `cw-field.diff` |
| C B + window check in `admit_cli_wrapper_journaled` | +19 (26860) | +16 (21058) | cumulative 21219 > shared STOP 21209 | `cw-kernel.diff` |

Cumulative (shared STOP 21209; D-20-14-12 C's 21250 is 20-9 L2 + 20-14 L2 only): recorded expectation 21203 → A 21203,
B 21205, C 21219 (STOP). Not compiled ([INFERENCE]; measurement 1).

**kloc forecast** (tokei; HEAD = every pin; recommended options):

| Crate · component | Measured / forecast | Basis |
|---|---:|---|
| xtask · `stability_matrix.rs` rows, LTS text, tests (AC2) | **+29** | `sm-stability-matrix` |
| xtask · precondition T3 leg (AC4) | **+64** | `gate-fuzz-chain` |
| xtask · `check-rc-readiness` (AC5) | **+120** | `gate-fuzz-chain-plus-rc-readiness` |
| xtask · LTS clock record (D-06 B) + review corrections | +20…+55 [INFERENCE] | `stability_matrix.rs:352-368` |
| maos-manifest · `take_schema_version` (AC3) | **+26** | `cw-peel` |
| maos-domain · `ECliWrapperSchemaUnsupported` (AC3) | **+8** | `cw-peel` |
| maos-spirit-abi · three consts (AC3) | **+3** | `cw-peel` |
| maos-bin · `worker_spawn.rs` peel (+ other readers, T0 measurement 3) | **+2**; 0…+20 [INFERENCE] | `cw-peel` |
| kernel | 0 | `rec-recommended-bundle` |
| docs, notes, workflows, TOML data, tests under `*/tests` | uncharged | `xtask/src/kloc_check.rs` |
| **aggregate** | **+252 measured; +272…+327 total** | `rec-recommended-bundle` = 193392 |

**Envelope (D-20-5-16 A, RULED 2026-11-04 (operator, ask d205); binding at dev start; ×2 measured / ×3 unmeasured upper; ceilings, not reservations):**

| Crate | HEAD | Forecast | Ceiling with allowance | Operator ruling needed |
|---|---:|---:|---:|---|
| maos-kernel-core (physical / tokei) | 26841 / 21042 | 0 | **26841 / 21042** | no (any line is STOP + D-row) |
| xtask | 44452 | +213 meas, +20…+55 | **45043** (+426 + 165) | yes |
| maos-manifest | 4604 | +26 | **4656** | yes |
| maos-domain | 10107 | +8 | **10123** | yes |
| maos-spirit-abi | 1042 | +3 | **1048** | yes |
| maos-bin | 34504 | +2 meas, 0…+20 | **34568** (+4 + 60) | yes |
| aggregate | 193140 | +272…+327 | **193869** (+729); hardfail = measured + 1 | yes |

**L1 re-base at dev start — 2026-11-11 (D02 A; base `f56959c2`, exact pins read from `xtask/kloc.toml` (c8e5d035 reproduced earlier by `kloc-check --json` on a `git archive` copy): bin 37832, wasm-host 3137, xtask 44936, maos-manifest 4738, maos-domain 10203, maos-spirit-abi 1042 (pin ceiling 3000 retained), aggregate 201396, `_aggregate_hardfail` 201397, kernel 26841 / 21042).** Ceiling = base + the allowance column above: **xtask ≤ 45527** (+591), **maos-manifest ≤ 4790** (+52), **maos-domain ≤ 10219** (+16), **maos-spirit-abi ≤ 1048** (+6), **maos-bin ≤ 37896** (+64), **aggregate ≤ 202125** (+729), **kernel 0**. Landed L1 (exact formatted): xtask 45315 (+379), maos-manifest 4765 (+27), maos-domain 10211 (+8), maos-spirit-abi 1045 (+3), maos-bin 37833 (+1), aggregate 201814 (+418; `_aggregate_hardfail` 201815), kernel unchanged — every figure inside its ceiling; the xtask figure is above the creation forecast (+213 measured / +268 with the inferred bands) because the readiness gate, the clock-record reader and the T3 leg were written with their error paths (`creation/measurements.md` priced prototypes that were never compiled).

Interaction: every unlanded Epic 20 story claims room in these crates (`creation/measurements.md` "Cumulative claims":
xtask ≈ +864 in deltas plus 20-7/20-1a's 45830; aggregate ≈ +10.0k plus residues). 20-5 lands last, so each ceiling
above is the allowance column re-based onto the exact pins then landed. Pins land exact after `cargo fmt --all`;
the xtask HISTORY first segment cites `20-5-v1-0-rc-tag-and-lts-rule` with `->` (D11 A); a lower measurement lowers
the pin.

**Operator-ruling list (RULED 2026-11-04, operator, ask d205; ABI record ratification still owed at dev):** (1) D-20-5-01…16; (2) the ABI ratification row for the three
`maos-spirit-abi` consts (D-04 A; `xtask/abi-ratifications.toml`, the 17-1 precedent `:205-213`); (3) the envelope
(D-16); (4) no kernel row (D-02 A) — B or C would each need a ruled FLAG-Winston row, C also a cumulative-STOP row.

## Decisions — RULED 2026-11-04 (operator, ask d205)

The operator ruled every D-20-5-01…16 row as recommended. The envelope binds at dev start, re-based onto the exact pins then landed; kernel-Δ remains 0. The concrete ABI ratification record remains owed at development.

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-20-5-01** | Landing shape | **A** one key, one landing after every other E20 key is `done` · **B** one key, two landings: L1 now (AC2–AC7, kernel-free, needs no other story), L2 last (AC1, AC8, AC9: version bump, release notes, readiness green) · **C** split key (`20-5a` tooling / `20-5b` rc readiness) | **B.** The schema version must land before the LTS clock and does not depend on any story; the readiness gate and the fuzz leg should exist while the other stories land so the operator can dry-run them; PF:126,144 put only the *readiness* last. C adds a key and the doc-gate churn 20-7 recorded. | RULED 2026-11-04 (operator, ask d205): as recommended **B** |
| **D-20-5-02** | `[cli_wrapper]` schema-version shape (kernel) | **A** peel: `CliWrapperConfig::take_schema_version(&mut toml::Value)` removes and window-checks the key before the strict parse; struct unchanged — kernel 0 · **B** struct field (`#[serde(default)]` = 1) + pre-parse check — kernel +2/+2 (test literals), FLAG-Winston, ABI row on `CliWrapperConfig` · **C** B + kernel check in `admit_cli_wrapper_journaled` — +19/+16, cumulative 21219 > 21209 · **D** a separate root `[worker] schema_version` table — kernel 0, a second convention | **A.** The only consumer is admission (C22); a field the struct carries moves the kernel pin for two test literals (C24); `deny_unknown_fields` means any shape must read the version *before* serde or a v2 manifest fails "unknown field" instead of "too new" (A and B both do). D diverges from `[class].manifest_schema_version`. | RULED 2026-11-04 (operator, ask d205): as recommended **A** |
| **D-20-5-03** | Counter and absent semantics | **A** own counter `CLI_WRAPPER_SCHEMA_VERSION = 1`, window `1..=1`; absent ≡ 1 for the whole v1.x line; in-tree Worker manifests, templates and docs declare it explicitly · **B** own counter, required (absent refused; ~26 test manifests + 5 `spirits/worker` manifests + docs change; BREAKING.md entry) · **C** share the Spirit window (`MANIFEST_SCHEMA_VERSION`, 1..=6 today, 7 after 20-8) | **A.** Every manifest written before the field keeps loading (N-1 backward, NFR-Maint-3) and a future v2 manifest is refused typed; C couples Worker evolution to every Spirit schema bump (20-8/20-9 move it twice this epic). B breaks every Worker manifest for no extra protection. | RULED 2026-11-04 (operator, ask d205): as recommended **A** |
| **D-20-5-04** | Where the window constants live | **A** `maos-spirit-abi` beside the triple constants (`lib.rs:171`): `stability_matrix.rs:38` already reads that file; `abi-diff` sees the addition ⇒ a ratification row the operator ratifies (17-1 precedent `abi-ratifications.toml:205-213`) · **B** `maos-manifest` (no ABI row; STABILITY reads `manifest.rs`) | **A.** The v1.0 compatibility contract is one constants file and one ratified ABI record; B hides a published compatibility window from the ABI gate. | RULED 2026-11-04 (operator, ask d205): as recommended **A** |
| **D-20-5-05** | The LTS rule's text (NFR-Maint-6) | **A** PRD-literal: v1.0 carries a **1-year** LTS commitment from the `v1.0.0` clock-start, security-only patches after year 1; the 2-year term is a v1.5 decision; an rc carries none · **B** 1-year security-only, then end of support · **C** keep the generated 2-year text (needs a PRD DELTA) | **A.** PRD `:137` ("Don't write a check the v0.8 team can't cash"), inventory `:217`, coverage-matrix `:1195` and E10 `:14,28` all say 1 year at v1.0, 2 at v1.5; STABILITY.md `:55-57` alone says 2. When "after year 1" ends is the operator's to state (the doc says it is set at v1.5). | RULED 2026-11-04 (operator, ask d205): as recommended **A** |
| **D-20-5-06** | LTS clock-start source | **A** keep `lts_clock_start`'s `git rev-list v1.0.0` (`:352-368`) and prescribe a post-GA regeneration commit · **B** a committed record (e.g. `xtask/release-channel.toml` `[lts] clock_start_tag/sha`) written by `ops-ga-tag-after-holds`; render never shells out to git · **C** render the placeholder always | **B.** With A, after GA the committed file at the tagged SHA says "pending" while a clone with the tag renders the SHA, and every `git archive` copy (the measurement instrument this epic uses) renders the placeholder again — `--check` cannot be byte-for-byte for both (`stability_matrix.rs:508-509` already notes the divergence). +10…+25 xtask [INFERENCE]. | RULED 2026-11-04 (operator, ask d205): as recommended **B** |
| **D-20-5-07** | T3 chain vs the tag | **A** chain-before-tag: the operator dispatches `fuzz-prerelease` at the candidate SHA; `release.yml` `sign-and-publish` passes `fuzz-ledger.json` to `check-release-precondition --fuzz-ledger`, which refuses a SHA without a complete chain; the `v*-rc*` push trigger is removed (it would start a duplicate ≈ 26 h chain after the tag) · **B** tag-then-wait: keep the trigger; publication moves to a `workflow_run` job after `fuzz-prerelease` completes · **C** draft release on the tag; the operator promotes after the chain, the precondition runs at promotion | **A.** At HEAD the chain starts on the same push that publishes (`fuzz-prerelease.yml:7-10`, `release.yml:2-4`), so any precondition would refuse every rc; A keeps one tag event, one publish, and evidence for the exact SHA before the signature. B publishes a day after the tag from a second workflow; C splits publication across two human acts. | RULED 2026-11-04 (operator, ask d205): as recommended **A** |
| **D-20-5-08** | Readiness gate shape | **A** new `check-rc-readiness [--tag] [--json]` (tree state: version/tag, STABILITY in sync, every `20-*` key `done`, provisioning rows `present`, deny ignores documented) run by the operator before tagging and by `release.yml` before signing, plus the precondition's fuzz leg — +184 measured · **B** every leg as flags on `check-release-precondition` (one command, tracker/provisioning read at tag time) — ≈ +150 [INFERENCE] · **C** a checklist doc + the fuzz leg only — +64 | **A.** The tree half is checkable offline before any tag and by an agent; the CI-evidence half needs the GitHub API on the tag SHA — two different inputs; C leaves the tree half to a human reading a list (rule 11: a control is real or labelled). | RULED 2026-11-04 (operator, ask d205): as recommended **A** |
| **D-20-5-09** | Perf evidence for the rc | **A** as ruled in 20-7 AC11.2: the rc waits for 20-7 L2 (every binding verdict `PASS` for the tag SHA; needs `ops-stable-perf-runner`, 20-7 §O O1–O7 and ≥ 2 weeks / 20 main runs) · **B** the rc ships with perf verdicts recorded `ABSENT` and labelled in the notes; `PASS` required at GA only (amends 20-7 AC11.2 and PF:126,144) · **C** no perf leg in the release precondition | **A.** It is the ruled contract (D-20-7-05 A, AC11.2) and the preflight's (PF:126,144: "all required metric verdicts … must exist for release SHA"); B is a legitimate operator choice to cut an rc earlier, which this story cannot make. | RULED 2026-11-04 (operator, ask d205): as recommended **A** | **AMENDED 2026-11-12 (operator, ask `rc` = b): B applies to the rc: the rc ships with perf verdicts recorded `ABSENT` and labelled in the release notes; amends 20-7 AC11.2's 'the rc waits for 20-7 L2'; the 20-7 L2 binding phase stays a named UNCLAIMED row needing a bare-metal runner.** |
| **D-20-5-10** | Deny ignore re-check (D-18-0-03) | **A** the readiness leg refuses an ignored RUSTSEC id without a `deferred-work.md` row, and the dev records a dated upstream re-check of each trigger (measured 2026-11-04: not met, O8) · **B** A + a `cargo deny check advisories` with a fresh DB inside `release.yml` · **C** a checklist line only | **A.** The per-commit blocking `cargo-deny` job already runs on a fresh DB (`discipline.yml:164-189`) and the aggregate precondition covers it; what nobody checks is that each ignore stays documented — two are not (C20). B duplicates a blocking job and adds network to the release job. | RULED 2026-11-04 (operator, ask d205): as recommended **A** |
| **D-20-5-11** | Published-docs rows `:1186`, `:1209` | **A** fix both in L1: topology (en + ko) runs `maos run <manifest>`, v3 `argv_prefix` example from a real Worker, TierNotGranted names `MAOS_HOST_GRANTS`; variant pages get non-parameter slugs (e.g. `/errors/EPinMismatch--Mismatch`) with `docs_url` and the route manifest updated before the LTS clock · **B** fix the examples; re-route the collapse to a named docs story (rule 10 needs one) · **C** both labelled as known limits in the release notes | **A.** The rows were routed here as "published docs truth before the RC"; 42 variant routes now serve at most 6 pages (C36); `docs_url` is a public contract (`check_error_catalog.rs:509`) that is cheapest to change before the LTS clock. | RULED 2026-11-04 (operator, ask d205): as recommended **A** |
| **D-20-5-12** | Which operator-lane rows gate the rc | **A** rc: provisioning (release keys + fuzz-ledger write, measured present), the T3 dispatch, `ops-stable-perf-runner` via 20-7 L2 (D-09), `ops-rc-signed-tag`; GA: Holds 1–2, `ops-external-cohort-and-pen-test`, `ops-ga-tag-after-holds`; neither (named in the notes): paid live legs, brew/AUR, RPO/RTO cadence, Docker Hub push · **B** A + `ops-live-gemini-leg` and brew/AUR publication for the rc · **C** only provisioning + the tag (fuzz and perf waived to GA) | **A.** `ops-rc-signed-tag`'s own acceptance names release.yml, packages, air-gap and container skipped-or-green (`sprint-status.yaml:349`), not live keys or taps; RELEASE-HOLDS holds GA only (`:17-21`); routing audit `:72` labels live legs "mechanism-proven, live on operator lane". C contradicts 20-3a Q3. | RULED 2026-11-04 (operator, ask d205): as recommended **A** | **AMENDED 2026-11-12 (operator, asks `ga`/`rc`): Holds 1–2 and `ops-external-cohort-and-pen-test` (pen-test half) no longer gate GA: claim boundaries, no GA effect; `ops-stable-perf-runner` no longer gates the rc (perf ABSENT-labelled); agents dispatch the T3 fuzz chain and provision what the `gh` token allows; `ops-rc-signed-tag` and `ops-ga-tag-after-holds` still need the key holder (operator action AI agents cannot perform); money/account/hardware rows stay named, UNCLAIMED.** |
| **D-20-5-13** | Release notes | **A** a hand-assembled `docs/release/v1.0.0-rc.1.md` (what ships, BREAKING entries, claim boundaries from RELEASE-HOLDS, operator-lane residue, LTS status) wired as `body_path` with `generate_release_notes` kept · **B** an xtask `release-notes` generator from BREAKING.md + the tracker (+80…+150 xtask [INFERENCE]) · **C** GitHub auto-notes only | **A.** The alpha's auto body is six PR titles (O7) and most landings are direct pushes; one rc does not pay for a generator; A is reviewable text the operator signs off before tagging. | RULED 2026-11-04 (operator, ask d205): as recommended **A** |
| **D-20-5-14** | Version bump timing | **A** L2's final commit bumps `Cargo.toml:67` to `1.0.0-rc.1`, `Cargo.lock`, STABILITY.md; the fuzz chain and perf verdicts are for that SHA (or a later rc-versioned SHA) · **B** the operator bumps in a release commit under `ops-rc-signed-tag` · **C** bump in L1 now | **A.** The SHA that is fuzzed must be the SHA that is tagged, with the version it ships; B puts a code edit in the operator lane; C ships every later landing as "rc" and changes `ESubstrateTooOld` comparisons mid-epic (safe by C30, but noisy). | RULED 2026-11-04 (operator, ask d205): as recommended **A** |
| **D-20-5-15** | Where the rc procedure and the LTS rule live | **A** new `docs/release/lts-policy.md`; `docs/release/tag-procedure.md` generalized to `vX.Y.Z-rc.N` (alpha history kept); RELEASE-HOLDS pre-release sentence extended; no docs-site route · **B** A + a docs-site page (en + ko, route manifest, ko glossary gates) · **C** a separate `docs/release/rc-procedure.md` | **A.** The epic names `docs/release/` (E20:220); one procedure for every pre-release keeps the macOS rehearsal and aggregate steps in one place; B adds a route and two gates for one page. | RULED 2026-11-04 (operator, ask d205): as recommended **A** |
| **D-20-5-16** | Per-crate envelope (D02 A applied) | **A** the envelope above (×2 measured / ×3 unmeasured) · **B** exact measured grants, every correction re-asks · **C** ×1.3 (rule 6) | **A.** 20-3b/20-3c landed at 2–3× creation forecasts; 20-4 L1 re-based the same way. | RULED 2026-11-04 (operator, ask d205): as recommended **A** |
| **D-20-5-17** | Perf leg in `check-release-precondition` (rc = b) | **A** `--perf-verdicts <json> [--perf-absent-ok]` over `check-perf-verdict --json`: FAIL/INDETERMINATE always refused, ABSENT refused unless `--perf-absent-ok`, JSON must parse, carry rows and the released SHA (a tool crash is never ABSENT); `release.yml` passes `--perf-absent-ok` for kind `rc` only, fetches `perf-ledger` at its logged tip, an HTTP 404 branch = empty ledger · **B** no perf leg for the rc (notes label only) | **A.** Honest: ABSENT is accepted only because the operator ruled rc = b, GA still needs PASS; B would let a FAIL through. | APPROVED 2026-11-12 (Main, under the operator ruling `rc` = b); landing 2 (Dev205L2) |
| **D-20-5-18** | `perf-verdict-label` leg and the `release` environment | **A** `check-rc-readiness` leg 6 `perf-verdict-label`: `docs/release/v<version>.md` exists and carries the exact line `Perf verdicts: ABSENT`; the `release environment protection evidence` row `absent` passes as `WARN: environment protection not provisioned: claim boundary` only with the exact notes line `Release environment protection: NOT PROVISIONED`, `present` needs none · **B** keep the row mandatory | **A.** The environment is absent (gh read 2026-11-12) and agents cannot provision money/accounts; the boundary is stated, not hidden. | APPROVED 2026-11-12 (Main); landing 2 |
| **D-20-5-19** | The 20-7 key in leg (c) | **A** named waiver `WAIVED_KEY = 20-7-stable-runner-perf-gates` accepted at `in-progress` only, only when the workspace version is `-rc.N` and the perf label leg passes, both modes (with/without `--tag`), detail prints `WAIVED`; the key is never flipped to done · **B** flip 20-7 to done | **A.** B would be a false done (20-7 L2 needs a bare-metal runner). | APPROVED 2026-11-12 (Main); landing 2 |
| **D-20-5-20** | Ruling wording and the fuzz dispatch | **A** generator wording (`stability_matrix.rs` placeholder) says the Holds are claim boundaries, STABILITY.md regenerated; comment-only fixes in `check_export_control.rs`, `gate_common.rs`; dated ruling notes in the pen-test docs, provisioning checklist, PRD scoping/NFR; `docs/release/decisions-2026-11-12.md` records the exact `fuzz-prerelease` dispatch command, to run at the rc SHA, NOT now · **B** dispatch now | **A.** The rc SHA does not exist yet; the chain is a 24 h+ hosted job at that exact commit. | APPROVED 2026-11-12 (Main); landing 2 |

## Operator ruling 2026-11-12 (asks `ga`, `rc`, `kb`) — amends D-20-5-09 and D-20-5-12

Operator words: "we are not doing NFR-Sec-7 and NFR-Comp-1 as we do not have resources and companies to do that. same applies to ops-* work as no resources, only AI agents we have."

- `ga` = **boundary.** RELEASE-HOLDS Holds 1–2 (external pen-test NFR-Sec-7; ECCN NFR-Comp-1) are explicit claim boundaries, not GA preconditions. `v1.0.0` may be tagged without them; the release notes and RELEASE-HOLDS state plainly: no external penetration test was performed; no ECCN determination was made; assurance is AI-agent review plus the hermetic CI proof only. The release may NOT claim NFR-Sec-7 or NFR-Comp-1 is met. WASM-engine-off-by-default stays in force. The signed-tag step needs the key holder: an operator action AI agents cannot perform.
- `rc` = **b** (D-20-5-09 B): the rc ships with perf verdicts ABSENT, labelled in the release notes; agents dispatch the T3 fuzz chain and provision what the `gh` token allows; money/accounts/hardware rows stay named and UNCLAIMED (never claimed done). The 20-7 L2 binding phase remains an unclaimed row needing a bare-metal runner.
- `kb` = keep: kernel shared STOP stays 21,250 tokei; ask again before any quarantine-closure work.

**Reading of the amended rows (dated 2026-11-12; the rows above are kept as ruled).** D-20-5-09's recommendation A text
"the rc waits for 20-7 L2" and D-20-5-12 A's "GA: Holds 1–2, `ops-external-cohort-and-pen-test`, `ops-ga-tag-after-holds`"
are superseded by this ruling: the rc does not wait for 20-7 L2 (perf verdicts ABSENT, labelled); Holds 1–2 are not GA
preconditions. D-20-5-13 A stands with the two note labels recorded at AC8. Decision rows D-20-5-17…20 (above) record how.

### Landing 2 — kernel-free ruling alignment (Dev205L2, 2026-11-12; uncommitted, awaiting review)

Implemented: `check-release-precondition --perf-verdicts <json> [--perf-absent-ok]` (+ `release.yml`: rc passes
`--perf-absent-ok`, GA does not, `perf-ledger` fetched via `branches/` with 404 = empty ledger); `check-rc-readiness`
leg 6 `perf-verdict-label`, the labelled `release environment` WARN, the `-rc.N`-only 20-7 waiver (key not flipped);
`stability_matrix.rs` placeholder wording + regenerated `STABILITY.md`; comment-only fixes in `check_export_control.rs`
and `gate_common.rs`; dated ruling notes (pen-test scope + triage, provisioning checklist, PRD scoping + NFR, tag
procedure); `docs/release/decisions-2026-11-12.md` (includes the exact `fuzz-prerelease` dispatch command and
preconditions, **not dispatched**). Version bump and the real `docs/release/v1.0.0-rc.1.md` are NOT in this landing
(the final 20-5 commit, after 21-5).

**kloc (exact, formatted).** xtask **45355 → 45461 (+106)** against the 20-5 ceiling 45527 (66 left): an
**orchestrator-adjudicated draw** (Main, 2026-11-12: +101 accepted, final ≤ 45527 required; the last +5 is a clippy
`if/else` fix inside that envelope), disclosed to the operator, recorded in the xtask HISTORY by
`scripts/repin.py --apply --adjudicated xtask`. aggregate 203240 (hardfail 203241); kernel 27103 / 21256 unchanged
(this landing is kernel-free). **Live pins: xtask 45461, `_aggregate_hardfail` 203241.**
Evidence: `20-5-v1-0-rc-tag-and-lts-rule-evidence/landing-2/` (failing-before at HEAD code: 16 of 45 red; ten plants, each
reds at least one test; `evidence.md`). Unverified: the hosted release run, `ci-20-5-verify.yml` (re-cut to six legs, never run),
the real `perf-ledger` fetch inside `release.yml` (probed read-only: `branches/perf-ledger` → HTTP 404, `commits/` → 422).

## Acceptance Criteria

*(Epic §20-5 as amended 2026-10-30; clauses marked (D-…) carry the ruled recommendation (2026-11-04, operator, ask d205).
Every AC names a failing-before witness and a plant. **local** = hermetic on the dev host; **hosted** = the discipline
run. No AC needs a secret, a tag or a dispatch.)*

### L1 — kernel-free, any time

#### AC2 — the LTS rule is published and generated (epic AC2; D-20-5-05, -06, -15)

1. `xtask/src/stability_matrix.rs` renders: a `release channel` row derived from the pre-release suffix of
   `parse_workspace_version` (`:335-346`): `-rc.N` → "release candidate — pre-release; LTS clock not started", none →
   GA, any other suffix → development pre-release (epic AC1 (i); prototype `sm`); the LTS section's text per D-05 A;
   the placeholder re-pointed from Epic 10 to `ops-ga-tag-after-holds` (`:221,:350,:367`); under D-06 B the clock
   reads a committed record instead of `git rev-list` (`:352-368`), and the record is absent until GA.
2. `STABILITY.md` regenerated (`stability-matrix`), `--check` green; in-src tests `:474,:492` updated, a channel test
   added (prototype `sm`).
3. `docs/release/lts-policy.md` (D-15 A): the rc → GA rule (an rc carries no LTS; `v1.0.0` by `ops-ga-tag-after-holds`
   after Holds 1–2 starts the clock; what "security-only after year 1" means and where its end is set); how the
   clock record is written at GA (D-06). `RELEASE-HOLDS.md:17-21,72-75` name `v1.0.0-rc.N` beside the alpha as a
   pre-release unaffected by Holds 1–2.
4. `tests/coverage-matrix.yaml:1189-1195` NFR-Maint-6: names the generated LTS section and `stability-matrix --check`
   as the witness of the *published rule*; it claims no gate for the commitment itself (rule 11).
5. **Failing-before (measured at HEAD):** the render contains "2-year LTS" and "Epic 10" and no channel row
   (`STABILITY.md:21-27,53-60`). Witness: the updated in-src tests + a vector rendering `1.0.0-rc.1` → channel row.
6. **Plant (local):** restore the 2-year sentence in the template ⇒ the `1-year LTS` assertion reds.

#### AC3 — `[cli_wrapper]` carries a schema version (deferred `:1175`; D-20-5-02 A, -03 A, -04 A)

1. `maos-spirit-abi`: `CLI_WRAPPER_SCHEMA_VERSION = 1`, `MIN_/MAX_SUPPORTED_CLI_WRAPPER_SCHEMA_VERSION` beside the triple
   (`lib.rs:171`); an `xtask/abi-ratifications.toml` row is **proposed** for the operator to ratify
   (`check-abi-ratification` red until ratified — never self-ratified).
2. `CliWrapperConfig::take_schema_version(&mut toml::Value)` (`maos-manifest`, prototype `cw-peel`): absent → 1; an
   integer inside the window → that value, key removed; anything else →
   `CliWrapperAdmissionError::ECliWrapperSchemaUnsupported { declared, min_supported, max_supported }` (new variant,
   `maos-domain/src/cli_wrapper.rs`). It runs **before** `CliWrapperConfig::from_toml_str`, so a manifest declaring
   `2` with an unknown key is refused as too new, not as "unknown field".
3. Every production reader of a Worker manifest's `[cli_wrapper]` table calls it before a strict parse
   (`worker_spawn.rs:610-618`; T0 measurement 3 classifies `operator_door.rs:1193`, `main.rs:5079,6135,11907`,
   `delegation.rs:938`, xtask `demo_j1.rs:1400`); no second parse path. `struct CliWrapperConfig` is byte-unchanged
   (D-02 A) — a struct change is STOP + D-row.
4. `xtask/error-catalog.toml` entry + `docs-site/docs/errors/ECliWrapperSchemaUnsupported.md` (en; no ko `errors/`
   tree exists); `error-catalog` gates green.
5. STABILITY.md rows (prototype `sm`): the `[cli_wrapper]` window and its refusal row. `spirits/worker/manifest*.toml`
   (×5), `spirits/topologies/*` Worker sections and `docs-site/docs/manifest/v3.md` (en + ko) declare
   `manifest_schema_version = 1`; under D-03 A no BREAKING.md entry (non-breaking), under B one.
6. **Failing-before (local):** at HEAD `[cli_wrapper]` with `manifest_schema_version = 1` is refused "unknown field"
   (`deny_unknown_fields`, `manifest.rs:4415`). Witnesses (`crates/maos-manifest` unit tests +
   `crates/maos-bin/tests/cli_wrapper_schema_20_5.rs` through the real `maos run`): absent → admitted; `1` →
   admitted; `2` + unknown key → typed refusal naming `1..=1`; `"x"` / `-1` → typed refusal; refused before any
   probe, pid or token (no TL row past the refusal).
7. **Plant (local):** call `from_toml_str` before the peel ⇒ the `2` + unknown-key vector reds ("unknown field").

#### AC4 — the release job refuses a SHA without a complete T3 chain (20-3a Q3; D-20-5-07 A)

1. `check-release-precondition --fuzz-ledger <file>` (prototype `gate-fuzz-chain`): for each target
   `manifest_parser`, `frame_deser`, `fuzz_exec_deps_parse_elf` there is **one run** (`run_id` `<run>-<link>`) with five
   `tier: "T3"` records at `commit == head_sha`, Σ `cpu_seconds` ≥ 86 400 — the collector's rule
   (`fuzz-prerelease.yml:310-326`), one source for both; `schema_version != 1` and a malformed ledger are loud errors,
   never a pass. No `unwrap`/`expect` after the parse.
2. `release.yml` `sign-and-publish` (`:117-122`): fetch `fuzz-ledger` read-only and pass the file; the aggregate leg is
   unchanged. `fuzz-prerelease.yml:9-10` `v*-rc*` trigger removed (D-07 A), `:5` comment updated; the 20-7 perf leg is
   untouched (serialize, Overlap).
3. `docs/release/tag-procedure.md` (D-15 A): the operator dispatches `fuzz-prerelease` at `candidate_sha` and waits
   for `t3-ledger-collect` green before tagging; the hosted wall time is stated as measured at the first dispatch.
4. **Failing-before (local):** at HEAD `check-release-precondition` passes an aggregate-success fixture with any (or
   no) ledger — it reads none. Witnesses in `xtask/tests/release_precondition_t3_20_5.rs`: complete chain → pass;
   4 links; another SHA; `T2`; two runs each < 86 400 s summing ≥ 86 400; 86 399 s; missing target; bad
   `schema_version` → refusal naming the target.
5. **Plant (local):** sum seconds across runs instead of per run ⇒ the two-runs vector reds.
6. **Measured, not faked:** against the real `fuzz-ledger.json` at HEAD the leg refuses (0 T3 records, O2); the record
   says so. The first green is the operator's dispatch — recorded with its run id when it exists.

#### AC5 — `check-rc-readiness` (epic AC1 "tree is tag-ready"; D-20-5-08 A, -10 A, -12 A)

1. `check-rc-readiness [--tag <vX.Y.Z-rc.N>] [--json]` (prototype in `gate-fuzz-chain-plus-rc-readiness`), offline,
   committed files only, one leg per line: (a) version is `X.Y.Z-rc.N` and equals `--tag` sans `v`;
   (b) `stability-matrix --check` in sync; (c) every `20-*` story key in `sprint-status.yaml` is `done`
   (`xtask/src/sprint_status.rs:16-51`); (d) `provisioning-checklist.md` rows `MAOS_RELEASE_PUBKEY`,
   `RELEASE_SIGNING_KEY`, `fuzz-ledger branch permission` are `present`; (e) every `deny.toml` ignored RUSTSEC id
   has a `deferred-work.md` row. Without `--tag` (the dev's L2 run) this story's own key may be `in-progress` or
   `review`; with `--tag` (the operator and the release job) every `20-*` key including this one must be `done`.
   It never tags, signs, fetches or writes.
2. `release.yml` `sign-and-publish` runs it with `--tag ${{ github.ref_name }}` before signing when the ref is an rc
   tag (alpha/GA behavior unchanged); the tag procedure runs it first.
3. **Measured at HEAD (by reading; T0 runs it):** (a) red `0.1.0-alpha.1`, (b) green, (c) red (12 keys not `done`,
   `creation/rc-contract.md` §1), (d) green, (e) red (RUSTSEC-2021-0127, -2025-0134).
4. Witnesses in `xtask/tests/rc_readiness_20_5.rs` on fixture trees: all-green; one per leg red; a provenance comment
   after `done` still reads `done`; `--tag v1.0.0-rc.2` against `1.0.0-rc.1` red; this story's key at `review`
   passes without `--tag` and reds with it.
5. **Plant (local):** drop leg (c) ⇒ the not-done fixture passes and its vector reds.

#### AC6 — published docs truth (deferred `:1186`, `:1209`; D-20-5-11 A)

1. `topology.md:38,75` (en) / `:39,76` (ko) run `maos run <manifest>` with a real path; `v3.md:72` / ko `:73`
   `argv_prefix` example from a shipped Worker (`spirits/worker/manifest.toml:9`); `ECliWrapperTierNotGranted.md`
   Resolution names `MAOS_HOST_GRANTS` (`env_contract.rs:602`) and `xtask/error-catalog.toml` carries the same
   remediation (the generator is the source; `deferred-work.md:1185` warns it erases curated text).
2. Variant error pages: slugs without a `:` parameter; `docs_url` (`check_error_catalog.rs:509`) and
   `docs-site/route-manifest.json:263` `expected_distinct_pages` re-measured by the CI Playwright suite (measurement 6).
3. **Failing-before:** the topology command exits "missing manifest path" (`worker_spawn.rs:165`); the built
   `/errors/EPinMismatch::Mismatch` page shows another variant's content. **Plant:** restore one `::` slug ⇒ the
   distinct-page assertion reds.

#### AC7 — deny ignore re-check recorded (D-18-0-03; D-20-5-10 A)

1. `deferred-work.md:1298`: a dated re-check line (trigger not met 2026-11-04: `secret-service` 5.2.0 on `zbus ^5`;
   re-measured at dev). Rows for RUSTSEC-2021-0127 (serde_cbor, frozen CBOR ABI) and -2025-0134 (rustls-pemfile)
   filed with owner or "Ownerless and open" + disposition (rule 10); `deny.toml` comments cite them.

### L2 — last (after every other Epic 20 key is `done`)

#### AC1 — the command (epic AC1; D-20-5-14 A)

1. **The command (local, on the final tree):** `env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask --
   check-rc-readiness --json` exits 0 with every leg `PASS`, after `Cargo.toml:67` = `1.0.0-rc.1`, `Cargo.lock`
   committed, STABILITY.md regenerated (`release channel` = release candidate). After the `done` flip the same
   command with `--tag v1.0.0-rc.1` is the operator's first tag-procedure step (never the dev's).
2. The discipline `release-dry-run` at that HEAD lists `.deb` amd64 + arm64 and the air-gap variant beside the
   binaries (20-2's artifacts; read from the hosted run, not built here); `epic-20-exit` green on that run.
3. `release.yml:174` keeps `contains(github.ref_name, '-')` (not narrowed); `container.yml` is skipped-or-green
   without Docker Hub secrets (20-2).
4. `check-release-precondition --fuzz-ledger` at that SHA: **refuses** until the operator's chain exists; the record
   carries the refusal verbatim and hands the dispatch to the operator (never faked, never waived by the dev).
5. **Failing-before (measured):** legs (a), (c), (e) red at HEAD (AC5.3). **Plant:** revert the version line ⇒
   leg (a) reds.

#### AC8 — release notes (D-20-5-13 A)

1. `docs/release/v1.0.0-rc.1.md`: what ships (Epic 20 closures), BREAKING.md entries since the alpha, claim
   boundaries (RELEASE-HOLDS), operator-lane residue (`creation/rc-contract.md` §3), LTS status ("no LTS; clock at
   `v1.0.0`"), perf/fuzz evidence state; wired as `body_path` in `release.yml` (`fail_on_unmatched_files` kept).
   **Dated note 2026-11-12 (operator ruling; the AC text above is kept as written; D-20-5-13 A stands).** The
   notes now carry two required labels: the exact line `Perf verdicts: ABSENT` (the rc = b ruling; `check-rc-readiness`
   leg `perf-verdict-label` is red without the notes file or the line) and, while the `release` environment is
   unprovisioned, the exact line `Release environment protection: NOT PROVISIONED`; the claim-boundary text states no
   external penetration test and no ECCN determination (RELEASE-HOLDS Holds 1–2 are boundaries, not GA preconditions).
   The real notes file is the final rc commit's job (after 21-5); L2 landing 2 only pins the legs against fixtures.

#### AC9 — records

1. At `done`: `deferred-work.md:107` (NFR-Maint-6) and `:1175` closed `CLOSED <date> by 20-5 @<commit>`; `:1186`,
   `:1209` closed or re-routed per D-11; `:1298` stays open with the re-check (AC7); routing audit `:131` cites AC4.
2. `ops-rc-signed-tag` (`sprint-status.yaml:349`) comment lists the exact prerequisites from `creation/rc-contract.md`
   §2–§3; no `ops-*` row is flipped.
3. Landing: exact pins, xtask HISTORY first segment citing this key with `->`; kernel unchanged (`check-kernel-baseline`
   pinned 26841 or the then-landed pin); revert `intent-lineage-coverage-report.md` if a run modifies it.

## Tasks / Subtasks

- [x] **T0 — Preflight.** Operator rulings on D-20-5-01…16 and the ABI row recorded; tracker → `in-progress`. Run *(L1, 2026-11-11: rulings read; ceilings re-based above; measurements 1, 2, 3, 6, 7 done (landing-1/evidence.md); 4 (public-api) by `check-abi-ratification` only; tracker → `in-progress`.)*
  `creation/measurements.md` "Not measured here" 1–8 (L1 needs 1–4, 6, 7). Re-measure every cite this story touches
  (rule 9). A figure above the ruled envelope, a kernel line, or a T0 result contradicting a ruled option is STOP + D-row.
- [x] **T1 — LTS rule (AC2).** `stability_matrix.rs`, STABILITY.md, `lts-policy.md`, RELEASE-HOLDS, coverage matrix. *(L1 done.)*
- [x] **T2 — `[cli_wrapper]` schema (AC3).** Consts, peel, variant, readers, catalog + page, manifests, ABI proposal. *(L1 done; ABI row PROPOSED, never self-ratified.)*
- [x] **T3 — T3 precondition (AC4).** Leg, `release.yml`, `fuzz-prerelease.yml` trigger, procedure, vectors, plant. *(L1 done; hosted run and the operator's chain unverified.)*
- [x] **T4 — Readiness (AC5).** Command, release wiring, fixtures, plant. *(L1 done.)*
- [x] **T5 — Docs truth + deny record (AC6, AC7).** *(L1 done.)*
- [ ] **T6 — L1 landing.** Exact pins inside the ruled envelope; `check-kernel-baseline` unchanged; `kloc-check`,
  `check-abi-ratification`, `check-dev-record-completeness`, `check-epic-close-coherence`, `check-exit-commands`,
  `check-decision-register`, `prepush`; one hosted discipline run on the final tree + plants. No commit or push without
  the operator.
- [ ] **T7 — L2 (AC1, AC8, AC9)** when every other `20-*` key is `done`: re-base ceilings, version bump, lock,
  STABILITY, notes, readiness green, precondition refusal recorded, records. Hand the tag to `ops-rc-signed-tag`.

## Dev Notes

### What changes, by file

| Area | Files | AC |
|---|---|---|
| xtask | `xtask/src/stability_matrix.rs:124-257,335-368,463-493`; `xtask/src/check_release_precondition.rs:33-96`; NEW `xtask/src/check_rc_readiness.rs`; `xtask/src/main.rs:617-625,791-799,1406-1409,1461-1463`; (D-06 B) NEW `xtask/release-channel.toml`; `xtask/error-catalog.toml`; `xtask/abi-ratifications.toml` (proposal); NEW `xtask/tests/release_precondition_t3_20_5.rs`, `rc_readiness_20_5.rs`; pins `xtask/kloc.toml:339,718` + rows | AC2–AC5 |
| ABI / manifest / domain / bin | `crates/maos-spirit-abi/src/lib.rs:171`; `crates/maos-manifest/src/manifest.rs:4495-4528`; `crates/maos-domain/src/cli_wrapper.rs:66-86`; `crates/maos-bin/src/worker_spawn.rs:610-618` (+ T0 readers); NEW `crates/maos-bin/tests/cli_wrapper_schema_20_5.rs` | AC3 |
| Manifests | `spirits/worker/manifest*.toml` (×5), `spirits/topologies/*` Worker sections | AC3 |
| Workflows | `.github/workflows/release.yml:103-174`; `.github/workflows/fuzz-prerelease.yml:5-10` | AC4, AC5, AC8 |
| Docs / records | `STABILITY.md`; NEW `docs/release/lts-policy.md`, `docs/release/v1.0.0-rc.1.md`; `docs/release/tag-procedure.md`; `RELEASE-HOLDS.md:17-21,72-75`; `docs-site/docs/deploy/topology.md`, `manifest/v3.md` (+ ko), `errors/*` (variant slugs, new page); `docs-site/route-manifest.json:263`; `tests/coverage-matrix.yaml:1189-1195`; `deny.toml:15-52` comments; `deferred-work.md:107,1175,1186,1209,1298` | AC2, AC6–AC9 |
| L2 | `Cargo.toml:67`, `Cargo.lock`, STABILITY.md | AC1 |

No new `maos-bin/src` file (`SCANNED_SOURCE_FILES` in `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs` unchanged);
a new one must be enrolled. No new env var (a new one would need `env_contract.rs`).

### Overlap (serialize; re-measure)

- **`check_release_precondition.rs` / `release.yml` `sign-and-publish`** — 20-7 AC11.2 adds the perf-ledger leg in L2;
  one integration owner per landing, the later lander re-bases.
- **`release.yml`, `container.yml`, `release_dry_run.rs`** — 20-2 (packages, air-gap, Docker split) edits the same
  jobs; 20-1a added the two registry binaries.
- **`maos-manifest/src/manifest.rs`, `maos-spirit-abi`, `abi-ratifications.toml`** — 20-8 (schema 7) and 20-9
  (WIT 2.4.0) bump the Spirit window and add ABI rows; this story's counter is separate (D-03 A).
- **`worker_spawn.rs`, `main.rs` Worker paths** — 20-10 (purge), 20-9 (spawned lifecycle) touch neighbours.
- **`docs-site/docs/errors/*`, `route-manifest.json`** — any story adding an error code; re-measure the page count.
- **`deferred-work.md`** — a sibling (20-11) edits it concurrently at creation; minimal targeted edits only.

### Current state that must be preserved

- `release.yml:174` generic `'-'` prerelease rule; the aggregate leg and its `filter=latest` semantics
  (`check_release_precondition.rs:63-67`); the dev-seed refusal (`release.yml:140-147`).
- `stability-matrix --check` byte-determinism and the `PRESERVED:export` fence (`stability_matrix.rs:259-333`).
- `[class]` triple enforcement (`security/mod.rs:279-289,525-573`) and `deny_unknown_fields` on `CliWrapperConfig`/
  `CliWrapperPosture` (17-1 D-5): the peel removes one key and nothing else.
- The collector's evidence rule (`fuzz-prerelease.yml:310-326`): the readiness leg never counts configured time.
- `fuzz-cadence.yml` T2 nightly unchanged.

### Guardrails

- The dev never tags, signs, pushes a tag, publishes, dispatches a workflow, writes a ledger branch, enters a secret,
  ratifies the ABI row or flips an `ops-*` row; a leg that only the operator can turn green stays red in the record.
- No kernel line; no `CliWrapperConfig` struct change (D-02 A); any is STOP + D-row.
- No `unwrap`/`expect` after a serde/TOML parse in production code; no test spawns cargo without the expensive marker.
- One cargo process per checkout; `env -u CI -u GITHUB_ACTIONS`; inherit the target directory, never override or clean
  it; plants byte-restored in the real tree (copy-back + `cmp` + `touch`, `proof-contract.md:24-31`).

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red | Leg |
|---|---|---|---|
| LTS text (AC2) | restore 2-year sentence | `1-year LTS` assertion | local |
| Schema refusal (AC3) | strict parse before the peel | `2` + unknown-key vector | local |
| T3 precondition (AC4) | sum across runs | two-runs vector | local |
| Readiness (AC5) | drop leg (c) | not-done fixture | local |
| Variant routes (AC6) | restore one `::` slug | distinct-page assertion | hosted (docs-site) |
| Version (AC1) | revert `Cargo.toml:67` | leg (a) | local |

### Tests and commands

```bash
# local; one cargo process per checkout; inherit the target directory
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p xtask --test release_precondition_t3_20_5 --test rc_readiness_20_5
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-manifest -p maos-domain
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test cli_wrapper_schema_20_5     # + every reader (proof-contract.md:48-53)
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --test spirit_boundary_invariants
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- stability-matrix --check
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-rc-readiness --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-abi-ratification
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-kernel-baseline
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- kloc-check --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-corpus
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-dev-record-completeness
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-epic-close-coherence
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-exit-commands
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-decision-register
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated $CARGO_TARGET_DIR/debug/xtask prepush --json
```

### Review plan (`proof-contract.md:33-71`)

DIFFICULT: Blind + Edge (opus-5.5), Acceptance (opus-5.5), Test Infrastructure (glm-5.3). Focus: AC4 cannot pass a
partial or foreign chain; AC5 cannot pass an unready tree; AC3 refuses before any side effect and leaves every other
key strict. Focused suites per touched crate + every reader of the changed manifests; the reviewer reads the plant
record, never re-runs plants except for a disputed finding.

### Previous-story intelligence

- 20-14 / 20-13 / 20-9 / 20-8 (created 2026-11-03): two-landing shape, kernel last; ceilings re-base onto landed pins —
  20-5, landing last, re-bases onto all of them.
- 20-7 (in-progress): owns the perf leg in the same file (AC11.2); its L2 needs O1–O7; `ABSENT` is the honest reading.
- 20-3a: built the T3 chain and its collector; routed the rc precondition here (Q3); a measured `cpu_seconds` rule, never
  configured time.
- 18-0: owner ruling for the zbus ignores (D-18-0-03); moved `cargo deny` into its own blocking job.
- 17-1: `deny_unknown_fields` on `[cli_wrapper]` and the ratification precedent for a Worker-manifest ABI change.
- 15-4: the release precondition, dry-run layout and alpha tag procedure; `ops-first-signed-tag` cut `v0.1.0-alpha.1`.
- 20-7: a key rename broke two doc gates `prepush` does not run — this story renames no key.

### Git intelligence

`73e7afbc` (20-4 L1 review corrections; bin 34504, aggregate 193140; kernel unchanged), `ef134444` (20-10 fixture
independence; rulings for 20-8/9/13/14), `2752e750` (20-4 L1 + four stories created), `8c809f9c`, `16c35ef0` (20-10).
`stability_matrix.rs`, `check_release_precondition.rs`, `CliWrapperConfig` and `fuzz-prerelease.yml` are unchanged since
7.5a / 15-4 / 17-1 / 20-3a respectively, apart from 10.3's export fence.

### Library / framework

No new dependency: `toml` and `serde`/`serde_json` (already xtask, manifest and bin deps); `thiserror` (domain);
`clap` (xtask CLI). The `gh api` fetch in `release.yml` already exists for the aggregate.

### References

E20@HEAD:43-49,59-61,78,211-220; preflight `epic-20-preflight-2026-10-30.md:80,126,144-145,194-197`; tracker
`sprint-status.yaml:305,335-340,349`; `deferred-work.md:107,1175,1185,1186,1209,1256,1298`; routing audit `:72,131`;
requirements inventory `:129-130,214-218`; PRD `non-functional-requirements.md:38-39,137,219`; architecture
`5-spirit-abi.md:185-197`; `RELEASE-HOLDS.md`; `docs/release/tag-procedure.md`; `docs/runbooks/provisioning-checklist.md`;
20-3a `:92,340,623`; 18-0 `:61`; 17-1 `:49,96,289`; 20-7 `:232` and `records-owed.md:45-60`.

## Dev Agent Record

### Story context creation

Created 2026-11-04 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter measured contract; `Create205`) at
`73e7afbc`, without cargo (parent rule).

**Inputs:** E20 §20-5 at HEAD (amended 2026-10-30, PF §3.2) — stale cites corrected in place (`creation/records-owed.md`
§A); preflight rows; tracker `:305`; every `deferred-work.md` row naming 20-5; PRD NFR-Maint-6 / NFR-Sec-5/6; 20-3a Q3,
17-1 Q8, 18-0 D-18-0-03 rulings; 20-7 §O; direct reads of every cited range (`creation/cites-73e7afbc.md`); read-only
ledger, tag, secret-name and run facts (§O); kloc and kernel by the prebuilt gate binary on `/tmp/c205` `git archive`
copies (`creation/kloc-kernel-73e7afbc-and-prototypes.json`); prototypes `cw-peel`, `cw-field`, `cw-kernel`,
`sm-stability-matrix`, `gate-fuzz-chain`, `gate-fuzz-chain-plus-rc-readiness` and the combined
`rec-recommended-bundle` saved as `rustfmt`'d diffs, never applied; the rc contract inventory
(`creation/rc-contract.md`).

**Measurements owed:** `creation/measurements.md` "Not measured here" 1–8.

**Records owed:** `creation/records-owed.md` (§A epic, §B planning records, §C routed observations).

**Revert:** no crate source modified (prototypes only under `/tmp/c205`, deleted); no cargo/test ran;
`intent-lineage-coverage-report.md` untouched.

**Gates after the record edits** (prebuilt `/mnt/build/cargo-maos/debug/xtask`, `env -u CI -u GITHUB_ACTIONS
CARGO_MANIFEST_DIR=…/xtask`, `--json`, on a `git archive 73e7afbc` copy at `/tmp/c205/gates` with the working tree's
`_bmad-output/` overlaid and the tree's `target` symlink; `creation/gates/*.json`): `check-exit-commands` rc 0 (2 blocks,
10 tokens); `check-decision-register` rc 0 (23 rows, 6 open); `check-dev-record-completeness` rc 0 (197 done stories,
178 deferred-owner assertions, 0 violations); `check-epic-close-coherence` rc 0 (21 epics, pin 26841). The overlay
includes a sibling's uncommitted 20-11 records. One read-only `stability-matrix --check --json` ran in the working
tree (in sync). `kloc-check` / `check-kernel-baseline` ran on archive copies only.

### Agent Model Used

anthropic/claude-opus-5-5 (creation); anthropic/claude-sonnet-5-5 (`Dev205L1`, landing 1).

### Landing 1 (kernel-free: AC2–AC7), 2026-11-11, base `f56959c2`, uncommitted

Evidence: `20-5-v1-0-rc-tag-and-lts-rule-evidence/landing-1/evidence.md` (witness table, red/plant logs, kloc, gates, what was NOT run). Operator-lane steps (T3 dispatch, tag, signing, protected `release` environment, ledger access, ABI ratification) are operator-owned and unverified. L2 (AC1, AC8, AC9) is not started.

**What landed.** AC2: `stability_matrix.rs` renders a release-channel row (`-rc.N` / GA / development), the 1-year LTS text per NFR-Maint-6, the placeholder re-pointed to `ops-ga-tag-after-holds`, and reads the LTS clock from the committed `xtask/release-channel.toml` (absent until GA; malformed = error; no `git`); `docs/release/lts-policy.md`, RELEASE-HOLDS, coverage-matrix note, `tag-procedure.md` generalized to `vX.Y.Z-rc.N`. AC3: `CLI_WRAPPER_SCHEMA_VERSION` + window consts, `CliWrapperConfig::take_schema_version` (struct untouched; peel before ANY other Worker parse in `run_cli_wrapper_manifest`, the only production strict parse), `ECliWrapperSchemaUnsupported` (`declared` is the TOML literal as a bounded `String`, a deviation from the prototype's `u32` so `"x"`/`-1` are not shown as `0`), catalog entry + page, five Worker manifests and the docs examples stamped `manifest_schema_version = 1`, shipped-manifest readers in tests peel first, ABI row `20-5-cli-wrapper-schema-version` PROPOSED. AC4: `check-release-precondition --fuzz-ledger` (per run: five distinct links 1..=5 at the exact commit, `tier T3`, max-per-link, Σ ≥ 86 400; schema/malformed/missing/empty-SHA are loud errors), `release.yml` fetches the ledger and requires the chain for rc and GA tags (other pre-releases unchanged), the `v*-rc*` push trigger of `fuzz-prerelease.yml` removed. AC5: `check-rc-readiness [--tag] [--json]`, five legs, wired into `release.yml` for rc tags. AC6: topology (en + ko), v3 `argv_prefix`, TierNotGranted remediation (catalog + page), 44 variant pages served at `/errors/<Family>--<Variant>` (slugs, `docs_url`, generated catalog, troubleshoot links, `gate-routes.js`, Playwright `manifest.ts`), `expected_distinct_pages` 137 → 176 (confirmed by the local Playwright a11y run). AC7: dated re-check and rows for RUSTSEC-2021-0127 / -2025-0134, `deny.toml` comments.

**Corrections / observations.** The prototype diffs were re-cut against the current tree (manifest.rs, worker_spawn.rs and main.rs had moved; `main()` returns `()`, so no `?`). More readers than the story listed strict-parse shipped manifests (`worker_manifests_2a.rs` ×2, `cookbook_manifests_17_3b.rs`, `spirits/worker/tests/spirit_smoke.rs`): they now peel first. `maos run` prints errors through `{:?}`, so a string `declared` appears escaped (the door vector unescapes). A stale ko ABI constants page needed the new sections for the glossary-lock gate. The first AC3 plant attempt did not apply (post-`cargo fmt` text) and ran green; it was discarded and redone. After the plants the xtask binary must be rebuilt before reading evidence from it (an evidence read from a planted binary was caught and redone).

**kloc (exact).** xtask 44936 → 45315 (+379; ceiling 45527), maos-manifest 4738 → 4765 (4790), maos-domain 10203 → 10211 (10219), maos-spirit-abi 1042 → 1045 (1048; pin ceiling 3000), maos-bin 37832 → 37833 (37896), aggregate 201396 → 201814 (202125; hardfail 201815). Kernel 26841 / 21042 unchanged.

**Verification.** After a recovery (the untracked files were rebuilt from this transcript, the logs regenerated) the one workspace nextest ran: 5957 run, 5955 passed, 2 failed, both unrelated (`kloc_check_runs_on_workspace`: the operator's untracked `spikes/gpui-operator-shell`; `erasure_million_20_11` 1M-row timing, already a deferred row owned by 20-7); `verb_table_15_3` no-default 7/7; all gates, clippy and the ratchet green (evidence.md). `prepush` skipped (wraps the same run).

**Not done.** Hosted legs and every operator-lane step (unverified); L2 (AC1, AC8, AC9).

### Landing 1 review corrections (2026-11-11, `Dev205L1`, anthropic/claude-sonnet-5-5), HEAD `fba6af2a`

Two independent reviews of the candidate commit are persisted verbatim under `20-5-v1-0-rc-tag-and-lts-rule-evidence/landing-1/review/` (`gpt-review.md`: 3 P2, "incorrect"; `security.md`: GLM-5.3, 1 HIGH, 4 MEDIUM, 4 LOW, 1 INFO). Evidence, failing-before, plants and the disposition table: `landing-1/review-corrections/evidence.md`.

- **Fixed with a failing-before and a plant each:** the LTS clock record accepts only `v1.0.0` (GPT-1 / F7); the hosted recipe builds in its own mandatory step, requires the gate's own exit status and a diagnostic or validated five-leg report, and its scripts are executed against stand-in binaries (GPT-2, GPT-3); `check-rc-readiness` audits the `release environment protection evidence` row and so is red on the real tree until the operator provisions it (F1); one fail-closed tag classification step that every later control reads (F2); the tagged commit must be an ancestor of `origin/main` before any gate runs (F3); the consumed ledger tip commit is logged and the file fetched at that commit (F4, partial); the aggregate query has no `status=completed` filter (F6); deny-ignore ids match as whole tokens and the false `deny.toml` header is corrected (F5, partial).
- **Deferred with rows (rule 10):** unsigned ledger / environment protection / run-id cross-check (owner `ops-provisioning-secrets-and-accounts`, follow-up 20-5 L2); class-aware deny leg and the darwin cargo-deny target; legs that read committed markdown (F8); mutable action pins (F9, pre-existing); non-table `[cli_wrapper]` typing (F10, INFO).
- **Honest misses found on the way:** the first recipe-refusal plant stayed green (the diagnostic check alone covered the status check), so two "refusal text with a failing status" vectors were added and the plant re-run red; the first aggregate-step vector was racy (`pipefail` SIGPIPE from a stand-in `cargo` that did not drain stdin).
- **kloc:** xtask 45315 -> 45333 (+18, ceiling 45527); aggregate 202018 (hardfail 202019, ceiling 202125); others unchanged; kernel unchanged.
- **Verification:** workspace nextest 5967 run, 5965 passed, 2 unrelated failures (the operator's untracked `spikes/` in `kloc_check_runs_on_workspace`; the known 1M-row `erasure_million_20_11` timing row owned by 20-7); all gates, clippy and the ratchet green; `verb_table_15_3` no-default 7/7. Hosted and operator-lane items remain unverified.

### File List (landing 1)

xtask: `src/{check_rc_readiness (new),check_release_precondition,stability_matrix,check_error_catalog,main}.rs`, `error-catalog.toml`, `abi-ratifications.toml`, `kloc.toml`, tests `release_precondition_t3_20_5.rs`, `rc_readiness_20_5.rs` (new). Crates: `maos-spirit-abi/src/lib.rs`, `maos-domain/src/cli_wrapper.rs`, `maos-manifest/src/manifest.rs` + `tests/cli_wrapper_schema_20_5.rs`, `maos-bin/src/worker_spawn.rs` + `tests/{cli_wrapper_schema_20_5 (new),worker_manifests_2a,cookbook_manifests_17_3b}.rs`, `spirits/worker/{manifest*.toml,tests/spirit_smoke.rs}`. Workflows: `release.yml`, `fuzz-prerelease.yml`. Docs: `STABILITY.md`, `docs/release/{lts-policy (new),tag-procedure}.md`, `RELEASE-HOLDS.md`, `docs/runbooks/{fuzz-cadence,provisioning-checklist}.md`, `docs/errors/error-catalog.json`, `tests/coverage-matrix.yaml`, `deny.toml`, `deferred-work.md`, `sprint-status.yaml`, docs-site (topology, manifest v3/v5, cookbook, 44 error pages + new page + TierNotGranted, troubleshoot, ko twins, `abi/v1/constants.md` + ko, `route-manifest.json`, `scripts/gate-routes.js`, `tests/playwright/manifest.ts`). Evidence: `landing-1/`, `ci/ci-20-5-verify.yml` (unverified recipe).

### Debug Log References

### Completion Notes List

### File List

## Change Log

- 2026-11-11: **L1 review corrections** (Dev205L1): GPT 3 P2 and GLM 1 HIGH / 4 MEDIUM / 4 LOW / 1 INFO dispositioned (fixed with failing-before + plants, or deferred with rows); tag classification and main-ancestry added to `release.yml`; readiness now audits the release-environment row; recipe rewritten and executed in tests; xtask 45333, aggregate 202018.

- 2026-11-11: **L1 (kernel-free AC2–AC7) implemented at `f56959c2`, uncommitted** (Dev205L1): ceilings re-based onto the exact pins (xtask 45527 / aggregate 202125 …); landed xtask 45315, aggregate 201814; red/plant logs under `landing-1/`; workspace nextest run (2 unrelated failures), prepush skipped; hosted and operator-lane steps unverified; tracker → `in-progress`.

- 2026-11-04: Operator ruled D-20-5-01…16 as recommended (ask d205): D-01 B, D-06 B, all other rows A. Grants bind at dev start with exact-pin re-basing; kernel-Δ 0. Updated ruling records and confirmed 20-7 operator prerequisites O1–O7 from its §O record.

- 2026-11-04: Story created at `73e7afbc` (ready-for-dev; D-20-5-01…16 PROPOSED). Measured: the generated LTS text says
  2 years where NFR-Maint-6 says 1; `release.yml` already publishes an rc as a prerelease; the T3 chain starts on the
  same tag push that publishes, and the fuzz ledger holds 0 T3 records; two `deny.toml` ignores have no deferred row;
  42 variant error routes collapse. Kernel 0 for the recommended `[cli_wrapper]` peel (field +2/+2, kernel check
  +19/+16 priced by reverted prototype); non-kernel measured xtask +213, manifest +26, domain +8, spirit-abi +3, bin +2.
