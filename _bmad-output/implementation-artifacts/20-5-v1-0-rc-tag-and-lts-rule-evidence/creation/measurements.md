# 20-5 creation — kernel and kloc measurements at `73e7afbc`

## Instrument

- `git archive 73e7afbc | tar -x -C /tmp/c205/base`; one copy per prototype (`cp -a`), never a worktree, never a new
  target directory.
- Prebuilt gate binary `/mnt/build/cargo-maos/debug/xtask` (built 2026-11-03 19:21), run as
  `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=<copy>/xtask …/xtask check-kernel-baseline --json` and
  `… kloc-check --json` from each copy root.
- Touched Rust files formatted with `rustfmt --edition 2021` (rustfmt 1.9.0, not cargo). Nothing compiled; no test ran.
- Diffs: `prototypes/*.diff` (unified, `a/` = base copy). Raw figures: `kloc-kernel-73e7afbc-and-prototypes.json`.

**Base reproduces the pins exactly:** kernel 26841 physical / 95 files / set hash `db39bc4a…b59` (= pinned), kernel
tokei 21042, xtask 44452, maos-manifest 4604, maos-domain 10107, maos-bin 34504, maos-spirit-abi 1042, aggregate
193140 (hardfail 193141, no crate over budget).

## Prototypes

| Prototype | Kernel physical | Kernel tokei | Kernel files changed | Other crates (tokei) | Aggregate |
|---|---:|---:|---|---|---:|
| `cw-peel` — `[cli_wrapper] manifest_schema_version` removed and window-checked by `CliWrapperConfig::take_schema_version` before the strict parse; struct unchanged; domain variant `ECliWrapperSchemaUnsupported`; three ABI consts (D-20-5-02 A) | **26841 (0)** | **21042 (0)** | — | manifest +26, domain +8, spirit-abi +3, bin +2 | 193179 (+39) |
| `cw-field` — the same window as a `CliWrapperConfig` field (`#[serde(default)]` = 1) + pre-parse check; every struct literal gains `manifest_schema_version: 1` (D-20-5-02 B) | **26843 (+2)** | **21044 (+2)** | `lifecycle/cli_wrapper/admission.rs`, `lifecycle.rs` (the two `#[cfg(test)]` literals) | manifest +28, domain +8, spirit-abi +3, bin +2; 7 kernel `tests/` literals + 1 `spirits/worker/tests` literal uncharged | 193183 (+43) |
| `cw-kernel` — `cw-field` + `check_cli_wrapper_schema` in `admit_cli_wrapper_journaled` (D-20-5-02 C) | **26860 (+19)** | **21058 (+16)** | same two files | as `cw-field` | 193197 (+57) |
| `sm-stability-matrix` — release-channel row, `[cli_wrapper]` window rows + refusal row, NFR-Maint-6 LTS text, placeholder → `ops-ga-tag-after-holds`, test updates + one new test (D-20-5-05; epic AC1 (i)/(ii)) | 0 | 0 | — | xtask +29, spirit-abi +3 (consts it reads) | 193172 (+32) |
| `gate-fuzz-chain` — `check-release-precondition --fuzz-ledger` T3 leg (D-20-5-07 A) | 0 | 0 | — | xtask +64 | 193204 (+64) |
| `gate-fuzz-chain-plus-rc-readiness` — + `check-rc-readiness` (version/tag, STABILITY in sync, Epic 20 closure, provisioning rows, deny ignores; D-20-5-08 A, -10 A) | 0 | 0 | — | xtask +184 | 193324 (+184) |
| **`rec-recommended-bundle`** — `cw-peel` + `sm` + fuzz leg + readiness | **0** | **0** | — | **xtask +213, manifest +26, domain +8, spirit-abi +3, bin +2** | **193392 (+252)** |

D-20-5-02 D (a separate root-level `[worker]` table) is kernel-free by construction (no struct change) and was not
prototyped; its size is `cw-peel`'s ± a few lines [INFERENCE].

### Kernel cumulative (rule: cumulative > 21209 is an operator row)

Recorded cumulative expectation 21203 tokei (20-14 creation record: 20-1b +68, 20-4 +9, 20-11 +6, 20-8 +33, 20-9 +21,
20-13 −1, 20-14 +25 over 21042; D-20-14-12 C joint STOP 21250 applies to 20-9 L2 + 20-14 L2 only). This story:
A → 21203 (0); B → 21205 (≤ 21209, but a FLAG-Winston row and both pins move for two test literals); C → 21219
(**above the shared STOP 21209** → STOP, operator row). The 20-11 sibling recorded a T0 STOP on 2026-11-04
(`deferred-work.md:1396`, uncommitted) that may change its row; re-base at dev start.

## kloc forecast (recommended options)

| Crate · component | Measured / forecast | Basis |
|---|---:|---|
| xtask · `stability_matrix.rs` rows, LTS text, tests (AC2) | **+29** | `sm` |
| xtask · precondition T3 leg + CLI flag (AC4) | **+64** | `gate-fuzz-chain` |
| xtask · `check-rc-readiness` + CLI (AC5) | **+120** | `gate-fuzz-chain-plus-rc-readiness` − `gate-fuzz-chain` |
| xtask · LTS clock-start record (D-20-5-06 B) | +10…+25 [INFERENCE] | `lts_clock_start` `:352-368` replaced by a TOML read |
| xtask · review corrections (release.yml wiring is workflow, uncharged) | +10…+30 [INFERENCE] | — |
| maos-manifest · `take_schema_version` (AC3) | **+26** | `cw-peel` |
| maos-domain · `ECliWrapperSchemaUnsupported` (AC3) | **+8** | `cw-peel` |
| maos-spirit-abi · three consts (AC3) | **+3** | `cw-peel` |
| maos-bin · `worker_spawn.rs` peel (AC3) | **+2** | `cw-peel` |
| maos-bin · other Worker-manifest readers if they must peel (`operator_door.rs:1193`, `main.rs:5079,6135`) | 0…+20 [INFERENCE] | T0 measurement 3 |
| kernel | 0 | `rec` |
| docs, release notes, LTS page, BREAKING/RELEASE-HOLDS, workflow YAML, error-catalog TOML, tests under `xtask/tests` / `crates/*/tests` | uncharged | `xtask/src/kloc_check.rs` |
| **aggregate** | **+252 measured; +272…+327 total** | `rec` = 193392 |

## Envelope (D-20-5-16 A = D02 A: ×2 measured / ×3 unmeasured upper; ceilings, not reservations)

| Crate | HEAD pin | Forecast | Allowance | Ceiling at HEAD | Operator ruling |
|---|---:|---:|---:|---:|---|
| maos-kernel-core (physical / tokei) | 26841 / 21042 | 0 | 0 | **26841 / 21042** (any kernel line is STOP + D-row) | no |
| xtask | 44452 | +213 meas, +20…+55 | +426 + 165 = **+591** | **45043** | yes |
| maos-manifest | 4604 | +26 | +52 | **4656** | yes |
| maos-domain | 10107 | +8 | +16 | **10123** | yes |
| maos-spirit-abi | 1042 | +3 | +6 | **1048** | yes |
| maos-bin | 34504 | +2 meas, 0…+20 | +4 + 60 = **+64** | **34568** | yes |
| aggregate | 193140 | +272…+327 | +504 + 225 = **+729** | **193869**; hardfail = measured + 1 | yes |

Every ceiling re-bases onto the exact pins landed before 20-5's dev start (it is the last story): ceiling = then-landed
pin + the allowance column. Each pin lands exact after `cargo fmt --all`; the xtask HISTORY first segment cites
`20-5-v1-0-rc-tag-and-lts-rule` with `->` (D11 A).

### Cumulative claims of the other unlanded stories (from their frontmatter; ceilings, not forecasts)

| Crate | Other stories' ceilings (their bases) | Remaining room they claim over today's pin [INFERENCE arithmetic] |
|---|---|---|
| xtask (44452) | 20-2 ≤ 44860 (44406), 20-4 ≤ 44466 (drawn to 44452), 20-8 ≤ 44459 (44406), 20-9 ≤ 44451 (44406), 20-13 ≤ 44644 (44406), 20-1b ≤ 42920 (42860), 20-7 / 20-1a ≤ 45830 | +864 as deltas (454 + 14 + 53 + 45 + 238 + 60) + 20-7 / 20-1a room to 45830 (+1378); with 20-5's +591 → ≈ 47.3k if every allowance is drawn |
| aggregate (193140) | 20-2 +490, 20-4 +386 left, 20-8 +1699, 20-9 +3181, 20-10 +511 left, 20-11 +2232, 20-13 +1045, 20-14 +455; 20-1a/1b (≤ 193768) and 20-7 (≤ 192568) residues unknown | ≈ +10.0k + residues; with 20-5's +729 → ≈ 203.9k if all drawn (20-9's record: ≈ +10.8k at 192519) |
| maos-manifest (4604) | 20-4 ≤ 4610, 20-8 ≤ 4812 | +214 |
| maos-domain (10107) | 20-4 ≤ 10115, 20-8 ≤ 10133, 20-9 ≤ 10117, 20-13 ≤ 10114, 20-14 ≤ 10101 | per-story deltas; 20-5 re-bases last |
| maos-bin (34504) | 20-4, 20-8 ≤ 34767, 20-9 ≤ 36379, 20-10 ≤ 34548, 20-11 ≤ 34698, 20-13 ≤ 34687, 20-14 ≤ 34441 | per-story deltas; 20-5 re-bases last |

## Other measurements

- `stability-matrix --check --json` → `{"passed":true,"in_sync":true,…}` on the archive copy and (read-only) in the
  working tree.
- `git merge-base --is-ancestor 07302ace HEAD` → true (`recovery-lane` merged; tag-procedure step 5 is history).
- crates.io, 2026-11-04: `secret-service` 5.2.0 → `zbus ^5`; `zbus` 6.2.1; `keyring` 4.2.0;
  `zbus-secret-service-keyring-store` 1.0.1.

## Not measured here (T0 owes; a result outside the ruled envelope or contradicting a ruled option is STOP + D-row)

1. Compile, clippy (hosted lints: `result_large_err`, `collapsible_if`, `too_many_arguments`, `unused_mut`,
   `trim_split_whitespace`) and tests of every prototype — none was compiled.
2. The kernel pin under the ruled AC3 shape on the then-landed tree (`check-kernel-baseline`; set hash unchanged
   under D-20-5-02 A).
3. Every reader of a Worker manifest's `[cli_wrapper]` table (`worker_spawn.rs:610-618`, `operator_door.rs:1193`,
   `main.rs:5079,6135,11907`, `delegation.rs:938`, xtask `demo_j1.rs:1400`) — which strict-parse it and must peel.
4. Kernel API surface / `cargo public-api` diff: the three `maos-spirit-abi` consts (abi-diff ⇒ ratification row)
   and whether an associated fn on `CliWrapperConfig` moves the kernel surface through `security/manifest.rs:9`.
5. Version bump `1.0.0-rc.1`: `Cargo.lock` regeneration, every test or doc that prints the kernel version
   (`env!("CARGO_PKG_VERSION")`, 17 sites), `ESubstrateTooOld` vectors.
6. Docs-site build + the CI Playwright suite for the `Family::Variant` collapse (42 routes) and the new
   `expected_distinct_pages`.
7. The fuzz-chain leg against the real `fuzz-ledger.json` at the candidate SHA (red today: 0 T3 records) and a
   fixture chain (green); hosted wall time of a dispatched chain (5 serial links × 5 h + setup ≈ 26 h [INFERENCE]).
8. 20-2's `.deb` (amd64 + arm64) and air-gap artifacts in the discipline `release-dry-run` layout at the final tree.
