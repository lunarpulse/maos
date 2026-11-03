# 20-14 creation — measurements (2026-11-03, HEAD `8c809f9c`)

## Instrument

No cargo ran (parent rule). Prebuilt gate binary `/mnt/build/cargo-maos/debug/xtask`, invoked from each copy's root as
`env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/tmp/proto2014/<name>/xtask /mnt/build/cargo-maos/debug/xtask {check-kernel-baseline,kloc-check} --json`
on `git archive 8c809f9c` copies under `/tmp/proto2014/` (deleted after measurement; never a worktree, never a
cargo target dir). Each changed file was formatted with `rustfmt 1.9.0 --edition 2021`; every touched file is
`rustfmt --check` clean at HEAD and `diff -rq` against the untouched copy lists only the intended files. Prototypes
were never applied to the tree and never compiled ([INFERENCE] that they build; "Not measured here" 1). Every
`.diff` here passes `git apply --check` against the working tree. Raw figures:
`kloc-kernel-8c809f9c-and-prototypes.json`.

## Baseline (untouched copy) — reproduces the pins

`check-kernel-baseline` rc 0: **26841** physical, 95 files, set hash `db39bc4a…`, changed/added/removed `[]`.
`kloc-check` rc 0: kernel **21042** tokei, aggregate **192519** (hardfail 192520), `alarm=true`, `over_budget=[]`.
Touched crates: maos-domain 10099 · maos-cli 6429 (pin 6856 reserved, STOP 6666 per D-20-1-05 A) · maos-audit 7721 ·
maos-bin 34191 · maos-iac 8270 · xtask 44406. Every pin except maos-cli's is zero-headroom; the aggregate has 1 line.

## Kernel shapes (◆ FLAG-Winston; D-20-14-02, -03)

Every shape includes the same `maos-domain` change (`dom`: `CostAttributionPayload.task_ref`, `CostScope`, R3 test
replaced by a positive + legacy-read test) — measured alone **−1 tokei** (the positive test is shorter than the
negative one it replaces).

| # | Prototype (D-row) | Diff | Kernel physical | Kernel tokei | Changed kernel files | Other |
|---|---|---|---:|---:|---|---|
| KT | scope bound to the token at mint: `CapabilityRegistryAdapter::{bind_cost_scope, cost_scope}` + token-keyed map pruned on bind; emission reads it by the verified token id, session principal → `SessionPrincipal`, proxy only as fallback, `task_ref` + `schema_version: 2` (D-02 B, **recommended**) | `kt-token-bound-scope.diff` | **+35 (26876)** | **+25 (21067)** | `capability/mod.rs` (+19/+12), `inference/mod.rs` (+16/+13) | domain −1 |
| KM | SCB marker (epic literal): `SpiritControlBlock::{set_cost_scope, cost_scope}` + `ScbTable` alias; adapter `with_scbs`; same emission (D-02 A) | `km-scb-marker.diff` | +44 (26885) | +30 (21072) | `scheduler/control_block.rs`, `inference/mod.rs` | domain −1; both adapter construction sites (`main.rs:4073,4100`) must call `with_scbs` |
| KM-naive | KM without the alias (full map type at field and builder) | `km-naive-scb-marker.diff` | +59 (26900) | +47 (21089) | same | priced to show the formatting cost |
| KP | root `CostScopePort` (maos-domain trait) read at emission; adapter `with_cost_scope` (D-02 C) | `kp-root-cost-scope-port.diff` | +31 (26872) | +25 (21067) | `inference/mod.rs` | domain +6 (port +7); both construction sites must call `with_cost_scope` |
| K0 | cost row carries the token bytes (`None` → `Some(&token_bytes)`), report joins a root-written context row (D-02 D) | `k0-token-on-cost-row.diff` | 0 (26841) | 0 (21042) | `inference/mod.rs` content hash only (`check-kernel-baseline` rc 1, changed) | the task/principal never ride the cost frame |
| KR | add-on: `InferenceRequest.correlation_id` journaled as Spirit-declared `request_ref` (D-03 A), measured on KM-naive | `kr-request-correlation-id-on-km-naive.diff` | +1 | +1 | `inference/mod.rs` | domain +9 |

Recommended set (D-02 B, D-03 B): **KT, +35 physical / +25 tokei → 26876 / 21067** (combined prototype `rec`, below,
reproduces it: changed `capability/mod.rs`, `inference/mod.rs`; 95 files; no file added or removed).

### Cumulative kernel accounting (ADR-069 `docs/adr/ADR-069-kernel-core-envelope-through-v1.md`; STOP 21209)

| Line | Physical | Tokei |
|---|---:|---:|
| HEAD (20-7 landed) | 26841 | 21042 |
| + ruled, unlanded: 20-1b R +78/+68, 20-4 R +13/+9, 20-11 KS +10/+6 | 26942 | **21125** (expected cumulative, as recorded) |
| + 20-14 KT measured +35/+25 | 26977 | **21150** — 59 under STOP 21209, 850 under alarm 22000 |
| + 20-14 KT at its ×2 ceiling +70/+50 (recorded rows only, at measured) | 27012 | 21175 — 34 under STOP; superseded by the rows below |
| + PROPOSED rows filed the same day at `8c809f9c`: 20-8 +33, 20-9 R +21, 20-13 KT3 −1 (their flag-winston records) | | **21203** — 6 under STOP |
| … with 20-14 KT at its ×2 ceiling | | **21228 — above STOP** → D-20-14-12 |
| … with 20-9 at its ×2 ceiling too | | 21249 |
| alternatives (on 21125): KM +30 → 21155; KM-naive +47 → 21172; KP +25 → 21150; K0 → 21125; KR adds +1 | | |
| worst case, every standalone ceiling spent (20-1b +136 from its base 21011, 20-4 +18, 20-11 +12, 20-14 +50) | | 21258 — **above STOP** |

Against the recorded rows alone the cumulative is 21150; counting every kernel row filed PROPOSED at the same HEAD
it is 21203, and this story's ×2 allowance takes it to 21228, above the cumulative STOP. That is raised as
D-20-14-12 (split / exchange / bump; recommended a joint targeted STOP of 21250 with 20-9's D-20-9-15), not
pre-ruled. 20-14 is creation-sequence step 5 (PF:142), so it is expected to land last and re-base.

## Non-kernel prototypes (measured)

| # | Prototype | Diff | Tokei |
|---|---|---|---:|
| NK | maos-cli `build_cost_report` keyed `(task, principal, attribution_source, spirit, pid, provider, model)`, row gains `task` / `attribution_source` / `spirit`, plain line prints them; maos-audit `spirit_names(db) → (boot_nonce, pid) → name` from kind-19/kind-7 rows; in-src test updated | `nk-domain-cli-audit-report.diff` | maos-cli **+25**, maos-audit **+29**, maos-domain −1 |
| BIN | shell slice on KT: `MAOS_SESSION_PRINCIPAL` env row, `session_principal_from_env()` (D-19-2-08 A rule), `ShellHost::with_session_principal`, `issue_turn_token` binds `CostScope { task_ref: "shell.turn:<ulid>", session_principal }` | inside `rec-kt-domain-cli-audit-bin-shell-slice.diff` | maos-bin **+35** |
| REC | KT + NK + BIN combined | `rec-kt-domain-cli-audit-bin-shell-slice.diff` | kernel +35/+25, domain −1, cli +25, audit +29, bin +35; aggregate **192632** (+113) |

## Non-kernel forecasts (not prototyped — [INFERENCE] from analogues)

| Crate · component | Forecast (tokei) | Analogue |
|---|---:|---|
| maos-bin · the shared seam mint binds a scope per pass (Butler/Orchestrator/Researcher, `main.rs:4179-4200`), one-shot hello (`main.rs:9601-9615`), `maos run` boot reads the session principal and threads it to the seam | +20…+60 | BIN slice (+35 for one site + env + helper) |
| maos-bin · D-20-14-04 D only: SSO subject cross-check at boot | +10…+20 | `enterprise_identity.rs:307-338` |
| maos-cli · D-20-14-06 A Worker-CLI label, `task: none` legend in plain output | +5…+15 | `format_cost_report_plain` `subcommands.rs:4986-5021` |
| maos-iac, xtask | 0 | K5 (forget body scan covers the new field); no gate changes |
| tests (`crates/*/tests`), ADR-046, docs (`run-maos.md` ×3), `tests/coverage-matrix.yaml` | uncharged | `xtask/src/kloc_check.rs` charges `src/` only |

## Not measured here (owed at T0; a figure above the ruled envelope, or a result contradicting a ruled option, is STOP + D-row)

1. Compilation and clippy of the ruled prototypes (`cargo check -p maos-kernel-core -p maos-domain -p maos-cli -p maos-audit -p maos-bin`).
2. Whether `CapTokensShardRing::get_scope` returns `None` once a token expires or is revoked (KT prunes on that).
   If it does not, prune by the stored TTL deadline (+3…+5 kernel tokei, inside the ×2 ceiling).
3. Classification of every `ProviderInfer` mint and governed-mint call site (C13; `issue_enterprise_governed_capability`
   callers `main.rs:4184,8259,9603,11543`, `recall_mediation.rs:114`, `skill_authoring.rs:112`, `spirit_session.rs:735`,
   `worker_spawn.rs:451,505,966`, `cross_team_crossing.rs:1050`): production vs smoke/test; each production
   Inference-Port site binds a scope or the record names why it cannot.
4. Whether a seam pass mints once per pass or once per call (`orchestrator_root.rs:69-90`, `researcher_root.rs:194-204`,
   `spirits/butler/src/lib.rs:1175`) — decides whether a pass is one task (D-20-14-01 A).
5. Whether both shell sessions of AC1 write a kind-19 or kind-7 identity row for hello-spirit at its real pid
   (D-20-14-05 A naming; `crates/maos-shell/src/lib.rs:202-205`).
6. AC1 witness wall time (two shell boots + one `maosctl` run) against `xtask/test-budgets.toml`.
7. `forget(P)` scrubs a session-principal cost frame with the existing body scan (K5), run once in the
   `forget_cascade_18_6.rs` harness before AC3.4 is written as a regression.
