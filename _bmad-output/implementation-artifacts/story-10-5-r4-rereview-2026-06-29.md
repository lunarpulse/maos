# Story 10.5 — R4-only re-review (2026-06-29) → UNANIMOUS GO

**Verdict: GO.** Story 10.5 flips NO-GO → **done**. §A2 release hold clears. v1.5 remains
held on EXTERNAL items only (real pen-test zero-P0/P1, export-compliance counsel 5D002.c.1).

## Why this re-review
The clean 4-layer re-review of 2026-06-26 (`story-10-5-rereview-2026-06-26.md`) was **NO-GO on
R4 ONLY** — it cleared R1, R2, R3, R5, H1, H2. R4 ("the Windows sandbox's unsafe FFI enforcement
was never runtime-exercised in CI") was only partially closed: `windows-check` ran the sandbox
test but it shipped **positive controls only**, and the enforcement negative control was
circularly self-deferred. The R4 fix (commit `1bf2ac5`) then added the enforcement negative
controls; two post-push gate fixes followed (`1dda03e` surface-hash re-bless, `f8dad58`
cargo-deny `[bans]` skips). The user confirmed the full discipline workflow — **including
`windows-check`** — is GREEN. This re-review adversarially confirms that the green is **meaningful**.

## Method
Workflow `wcw24wqlm` (run `wf_8da820ce-d9a`), 4 agents (claude-opus-4-8, high effort), ~291k tokens.
3 adversarial Verify layers in parallel + an independent-reconfirm Synthesis lead. Agents run on a
Linux host (cannot read the windows-check CI log → they evaluate the **test design** for a
meaningful green, and reproduce every Linux-reproducible gate locally for observable evidence).

| Layer | Verdict | Conf. |
|---|---|---|
| R4 Enforcement Falsifier | GO | high |
| Test-Infra / post-push gate-fix integrity | GO | high |
| HEAD regression / acceptance | GO | high |
| **Synthesis (independent re-confirm)** | **GO** | **high** |

`confirmed_blockers = []`.

## R4 genuinely closed (the decisive finding)
- **Memory negative-control PAIR is a sound differential.** `windows_memory_cap_kills_overbudget_child`
  (256 MiB cap) and `windows_memory_cap_allows_underbudget_child` (4 GiB cap) run the **identical**
  ~1 GiB PowerShell hog under the **identical, unconditionally-created** low-integrity restricted
  token. The only varied input is `memory_max_mb`, which feeds **exclusively** into `ProcessMemoryLimit`
  under `JOB_OBJECT_LIMIT_PROCESS_MEMORY`. So:
  - a cap-that-doesn't-cap → 256 MiB test sees `exit 0` → `assert!(!success)` flips **RED**;
  - any *unrelated* failure (PowerShell startup, arg quoting, alloc) → the 4 GiB control also fails →
    `assert!(success)` flips **RED** rather than passing false-green.
  A simultaneously-green pair is only satisfiable if the commit cap actually bounds between 256 MiB
  and ~1.1 GiB.
- **Integrity test fail-closed.** `cmd /C "whoami /groups | findstr S-1-16-4096"`: `cmd /C` returns the
  last pipeline stage's (findstr) exit; exit 0 iff the Low Mandatory Level SID `S-1-16-4096` is present.
  A Medium token (`S-1-16-8192`) has no matching substring (exactly one mandatory-integrity SID per
  token) → `exit 1` → RED. `SetTokenInformation` failure → `.expect` panic → RED.
- **No vacuous-green path.** 6 `#[test]`, 0 `#[ignore]` (grep-confirmed), no name filter on the step
  (`cargo test ... --test sandbox_enforcement_windows -- --nocapture --test-threads=1`),
  `#![cfg(target_os = "windows")]` satisfied on windows-latest (cannot be 0-tests), `.expect` on
  spawn/wait fail-closed, and `windows-check` is in the `v1-0-ship-gate` `needs` whose aggregate fails
  on any sub-result failure/skip/cancel.

## Post-push gate fixes mask nothing (independently reproduced on Linux)
- **`1dda03e` surface re-bless** — `check-service-boundary` exits 0; independent set-compare of
  `.current_surface.items` vs the baseline = **371 == 371**, added=0, removed=0, no paths added/removed,
  no set-dedupe collapse. The commit changed exactly one baseline line (the `windows::spawn_sandboxed`
  `signature_hash` 99f8aaa8→7e620e6e, path unchanged). `canonicalize_signature` hashes `quote!(#item)`
  over the whole fn item **including its body**, so the R4 body edit legitimately re-hashes. No kernel
  symbol added/removed was hidden. *(Note: the commit message says "370==370"; the rigorous full-tuple
  count is 371 — a cosmetic off-by-one from an earlier (kind,path)-keyed dict quick-check. The
  load-bearing 1-line diff is verified correct.)*
- **`f8dad58` cargo-deny skips** — `cargo deny check` exits 0 with advisories/bans/licenses/sources all
  **independently** passing. The 9 skips are under `[bans].skip` (multiple-versions hygiene only);
  stripping them makes `cargo deny check bans` fail with exactly those 9 duplicate errors (load-bearing).
  Genuine 10.4a postgres/pgvector-vs-workspace transitive dual versions; no vulnerability/license masked.
- **H2 CI wiring** — `cargo test -p xtask --test story_10_5_proven_red` 7/7 green; wired blocking in
  `check-skill-conformance` (continue-on-error:false, in ship-gate needs); drives the real xtask binary
  over tempdir fixtures hitting the actual gate. No longer dev-pass-only.
- **Re-pin** — `check-kernel-baseline` exits 0 at `src_lines = 22964`; HISTORY discloses the +9
  windows.rs lows (FLAG-Winston) with R4 tests under `tests/` not counted.

## No regression — R1/R2/R3/R5/H1/H2 hold at HEAD
R1 (windows.rs compiles; lows are additive, no import/symbol change), R2 (i18n honestly deferred:
LOCALES.md Deferred-to-v2.0 + DO-NOT-SHIP scaffold markers, ja/index.md byte-identical to ko, zero kana,
gates report-only), R3 (22964 pin + churn disclosed), R5 (JetBrains tests wired in windows-check, pass
on Linux), H1 (JOB_OBJECT_LIMIT_PROCESS_MEMORY commit cap, not working-set), H2 (proven-red present + wired).

## AC disposition
AC1 PASS · AC2 PASS · **AC3 PASS** (former NO-GO: compiles on windows-latest, runtime enforcement bites,
R3 disclosed, H1 commit cap) · AC4 PASS · **AC5 DESCOPED** to v2.0/Epic 11 (clears R2 fabrication; not a
v1.5 release blocker) · AC6 PASS · AC7 PASS.

## Non-blocking follow-ups (logged, do not block the GO)
1. Move `cargo deny check` into its own `continue-on-error: false` job — at present it is a step inside
   the advisory `reproducible-build` job, so the "NON-NEGOTIABLE" supply-chain gate is effectively
   non-blocking (pre-existing 2026-06-12; masks nothing at HEAD since it passes).
2. Add a `>=1 test ran` vacuous-green guard to the windows-check sandbox-test step (idiom already used by
   `check-j4-latency`) — latent gap only under hypothetical future cfg/target drift.
3. Epic 11 must carry **real** ja/zh-Hans translations plus a **language-identity gate** (coverage +
   glossary-lock provably cannot detect wrong-language content).

## Remaining v1.5 holds (EXTERNAL / human — out of dev scope)
- Real external pen-test returning zero P0/P1.
- Export-compliance counsel sign-off (5D002.c.1, pre-distribution).

**Do not claim v1.5 shipped until both clear.** Story 10.5 dev work is GO/done.

---
Full re-review output: `/tmp/.../tasks/wcw24wqlm.output` (preserved). Supersedes the R4 NO-GO in
`story-10-5-rereview-2026-06-26.md`.
