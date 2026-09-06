# Preflight R2 — Epic 15 Foundations (W0)

> Scout note, 2026-09-05. Epic file: `_bmad-output/planning-artifacts/epics/epic-15-foundations-w0.md`. Every citation pinned `@9f920180` (HEAD, tree clean at start: `git status --porcelain` empty). Method: read-only; the five exit-line gates, `coverage-matrix`, and the `t_14_2a` test were RUN, not inferred. Scratchpad copies were used for the phase experiment; `tests/` was not touched.

## Verdict block

| | |
|---|---|
| **Verdict** | **needs-rework** (not blocked: every seam is real and every red is reproducible; but three ACs name mechanisms that do not exist and two forks are still open inside ACs) |
| **Confidence** | **55 / 100** (epic asks 75) |
| **Measured duration** | **3–5 weeks** serial for one engineer + agents (epic says 2–3); 2.5–4.5 weeks if 15-5 runs in parallel |
| **False / partial premises** | 6 FALSE, 14 PARTIAL out of 61 audited claims (table §A); 41 CONFIRMED TRUE |

**Top-3 blockers**

1. **15-3 AC3 — `maos` has no `--help` to resolve against.** `crates/maos-bin` has zero `clap` uses (`grep -c "clap::" main.rs` = 0 @9f920180); argv is hand-parsed (`crates/maos-bin/src/main.rs:1653 @9f920180`, per-verb `usage:` eprintln at `:1410,:1515,:1534`). Observed: `cargo run -q -p maos-bin -- --help` prints nothing to stdout, **boots the daemon** (revocation poller loop, watchdogs) and only exits on SIGTERM after the 120 s timeout. `check-exit-commands` as written would hang or vacuously pass on every `maos` token. `maosctl --help` works (clap, 23 commands). Plus **15-3 AC2 conflates two phases**: `gate_common.rs:163-166` documents `CURRENT_PHASE = "v1_5"` as *"the GA / ship phase … governs ONLY the GA ship-gate ladder"*, while `tests/phase-config.toml:5` / `tests/coverage-matrix.yaml:2` hold the *delivered* phase `v0.1-alpha`. Measured: generating phase-config from gate_common (`v1.5` in both, scratchpad copies) makes `coverage-matrix` emit **145 violations** (advisory only because `mode: warning`, `coverage-matrix.yaml:3`, `coverage_matrix.rs:65`).
2. **15-4 AC3 — cross-workflow `needs:` does not exist.** `release.yml` runs `on: push: tags: ['v*']` (`:2-4`); `discipline.yml` runs on `push/pull_request` to `main` (`:3-7`); no `workflow_call`/`workflow_run` anywhere in `.github/workflows` (grep empty). A job cannot `needs:` a job in another workflow. And the exit block's third line (`git tag … && git push --tags`) is a network + credential action whose downstream job needs `RELEASE_SIGNING_KEY` (`release.yml:74-77`) — not hermetic under rule 2.
3. **15-1 AC2 fork + AC6 cause, and 15-6 "every Spirit".** AC2 leaves attribute-vs-whitelist to "the story" (rule 7) although the whitelist is *path*-scoped and semantically wrong (below). AC6's stated cause is contradicted by measurement: the test **passes alone** and at `--test-threads=2`; it fails only under full 3-way concurrency. 15-6: only two inference consumers exist at HEAD (Researcher; shell/hello-spirit) — the other eight Spirits declare no inference — and AC1's new default would red the **10** existing `maos run spirits/researcher/manifest.toml --once` sites.

---

## A. Citation audit

Legend: TRUE / PARTIAL (exists, claim about it wrong) / FALSE / NEITHER (does not exist and no story creates it) / NOT CHECKED.

| # | Claim | Where | Verdict | Evidence @9f920180 |
|---|---|---|---|---|
| 1 | "4 red gates + 1 test red" | Closes | TRUE | Ran: `check-empty-kernel` EXIT=1 (6 violations), `check-service-boundary` EXIT=1 (19+17+4 rows), `kloc-check` EXIT=1, `check-env-contract` EXIT=1 (2 violations); `check-kernel-baseline` PASSED (24472 = 24472). `t_14_2a_post_grace_journal` FAILED 2/2 under default threads |
| 2 | 8 `CURRENT_PHASE` constants | Closes, 15-3 AC1 | TRUE | `grep -rho "const CURRENT_PHASE" xtask/src \| wc -l` = 8, at exactly the eight file:lines listed (`gate_common.rs:166` is `pub const`) |
| 3 | zero kloc headroom in 8 crates | Closes | TRUE | kloc-check table: maos-a2a-core 4856/4856, maos-audit 6847/6847, maos-bin 17109/17109, maos-cli 5270/5270, maos-cohort 5857/5857, maos-iac 6960/6960, maos-kernel-core 18933/18933, xtask 41953/41953 |
| 4 | `release.yml` first-run defect | Closes, 15-4 | TRUE | see #31-#34 |
| 5 | "no hermetic inference seam outside Researcher" | Closes | PARTIAL | Replay seam is Researcher-only (`main.rs:4623-4641`), but there is a SECOND inference consumer with no seam at all: shell mode passes `inference` to `maos_shell::run_shell` (`main.rs:3247-3250`; `crates/maos-shell/src/lib.rs:15,166`; `crates/maos-spirit-hello/src/lib.rs:79` calls `inference.complete`) |
| 6 | Exit line 1 verbs exist | Exit | TRUE | `check-empty-kernel`, `check-service-boundary`, `kloc-check`, `check-env-contract` all ran (xtask clap). `cargo test --workspace --no-fail-fast` NOT RUN (brief) |
| 7 | `check-exit-commands` | Exit line 2, 15-3 AC3 | NEITHER at HEAD | `grep -rn check-exit-commands xtask .github` empty; created by 15-3 AC3 (see §B) |
| 8 | `git tag v0.1.0-alpha.1 && git push --tags` → release.yml builds signed maos + maosctl | Exit line 3 | PARTIAL | Only tag at HEAD is `frozen-kernel-v2.0` (`git tag`), which does not match `v*` → `release.yml` has **never run**. Needs network, push credential and `RELEASE_SIGNING_KEY` (`release.yml:74-77`) — not hermetic |
| 9 | discipline `aggregate` exists | 15-1 AC1 | TRUE | `.github/workflows/discipline.yml:3510` `aggregate:` with 136 `needs:` |
| 10 | Kernel-Δ ZERO @24472 | Kernel-Δ | TRUE | `check-kernel-baseline: PASSED (maos-kernel-core/src = 24472 lines, pinned 24472)` |
| 11 | "≤2 attribute lines" for two structs | Kernel-Δ, 15-1 AC2 | PARTIAL | Achievable only with the single-line form (`#[maos_attrs::i9_exempt(reason = "…")]`, precedent `crates/maos-capability/src/cap_quota/mod.rs:37`). The kernel's own precedent is 3 lines per attribute (`security/mod.rs:120-122`, `revocation/rules.rs:16-18`) → +6 if copied. `check-fmt` is Blocking (`discipline.yml:136-139`) so a long reason string reflows |
| 12 | "ADR-037 whitelist path" | Kernel-Δ, 15-1 AC2 | PARTIAL | `docs/adr/ADR-037-constitutional-amendment-process.md` mentions no whitelist (grep `whitelist\|i9\|exempt` empty). The whitelist is `xtask/i9-whitelist.toml` (exists, 3 *path* entries; its comment: "Adding a fourth entry … requires invariant-lock review per ADR-037"). It is path-scoped: `check_empty_kernel.rs:243-244` skips the denylist for the WHOLE file, and `docs/invariants/I9.md` defines the list as "sanctioned holder paths" where *persistent state is permitted* |
| 13 | `T3ImageVerificationConfig` `security/mod.rs:124` | 15-1 AC2 | TRUE | `crates/maos-kernel-core/src/security/mod.rs:124 struct T3ImageVerificationConfig` (fields `PathBuf`, `[u8;32]` — neither denylisted) |
| 14 | `ValidatedRevocationRules` `revocation/rules.rs:19` | 15-1 AC2 | TRUE | `:19 pub(crate) struct ValidatedRevocationRules`, attribute at `:16-18`; gate: "not documented in docs/invariants/i9-exemptions.md" |
| 15 | both "documented in `docs/invariants/i9-exemptions.md`" | 15-1 AC2 | PARTIAL | File exists. `SecurityManagerAdapter` already has an entry (`i9-exemptions.md:91`). After the move, `T3ImageVerificationConfig` carries NO attribute and its fields are not denylisted → it needs **no** entry (the register "enumerates every `#[i9_exempt]` site", `:8`). Only `ValidatedRevocationRules` needs one |
| 16 | `#[i9_exempt]` at `security/mod.rs:120` moved onto `SecurityManagerAdapter` `:130`, net-0 | 15-1 AC2 | TRUE | `:120-122` attribute (reason text names "security manager adapter") sits above `:124`; `:130 pub struct SecurityManagerAdapter`; gate reds it twice (`policy: Arc<PolicyTable>`, `provider_history: Arc<Mutex<ProviderHistory>>`). Moving 3 lines = net 0 |
| 17 | `ScbRuntimeSnapshot` `control_block.rs:244` | 15-1 AC2 | TRUE | `crates/maos-kernel-core/src/scheduler/control_block.rs:244 pub struct ScbRuntimeSnapshot`; gate: field `spirit_obj: Arc<dyn AnySpiritObj>` |
| 18 | `VerifiedImageLock` `t3/image_lock.rs:27` | 15-1 AC2 | TRUE | full path `crates/maos-kernel-core/src/security/sandbox/t3/image_lock.rs:27`; gate: `attestations: Vec<T3ImageAttestation>` |
| 19 | check-empty-kernel reds on "the four" | Q1 | PARTIAL | It reds on **five** structs (the four + `SecurityManagerAdapter:130` twice) + two "not documented" rows (`:124`, `:19`) = 6 lines. After the AC2 moves all six clear |
| 20 | "**16** `other` symbols", `check_service_boundary.rs:236-241` | 15-1 AC3 | TRUE (17 rows) | 17 "has class 'other'" rows, 16 unique (`SuccessorSpiritFactory` twice). `:237-241` `classes.classes.get(&item.path)…unwrap_or_else(\|\| "other")` |
| 21 | "19 signature-changed rows" | 15-1 AC3 | PARTIAL | 19 "removed public kernel symbol" rows (17 unique; `UpgradeError`, `UpgradeOrchestrator` twice). ~9 are signature changes (reappear as "new": `StateCodecError`, `SpiritControlBlock`, `SpiritSchedulerAdapter`, `HotSwapSaga`, `HotSwapCoordinator`, `build_runtime_argv`, `spawn_t3`, `load_and_verify_lock`, + `SecurityManagerAdapter` which is classified); **~8 are genuinely absent** from the current walk (`SpawnError`, `UpgradeError`, `UpgradeOrchestrator`, `UpgradeOutcome`, `UpgradeReport`, `UpgradePolicy`, `RevocationApplier`, `get_default_image`). Regenerating the baseline for those contradicts the gate's own rule "monotonically additive within a major version" (`:227`) — an invariant-lock ratification, not "a one-line rationale" |
| 22 | `kernel-api-classes.toml` row format | Q2 | TRUE | `"maos_kernel_core::api::security::SecurityManagerAdapter" = "supervision"` (`xtask/kernel-api-classes.toml:22-28`); header says adding rows "requires invariant-lock review" (`:3`) |
| 23 | `kernel-surface-v0.1-beta.json` "is regenerated" | 15-1 AC3 | PARTIAL | File exists (75 134 B, `docs/ci-baselines/`). It is only a `--baseline` INPUT (`xtask/src/main.rs:204-208`); `check_service_boundary.rs` has no `--update`/write flag (grep empty). Regeneration mechanism: NOT PRESENT — the story must hand-edit or add one (xtask lines) |
| 24 | `maos-domain` +51 | 15-1 AC4 | TRUE | kloc-check: `maos-domain 8695 / 8644 ❌ OVER` |
| 25 | "the two env lines in maos-bin" | 15-1 AC4 | PARTIAL | Registering one var costs a 5-line `EnvVar { … }` block (`env_contract.rs:14-18`) → ~10 lines, not 2; maos-bin is 17109/17109 |
| 26 | D17 = `_aggregate_hardfail` red | 15-1 AC4 | TRUE | `kloc.toml:501` = 147057; measured aggregate 155498 (kloc-check) |
| 27 | `env_contract.rs:13` format | 15-1 AC5, 15-6 AC4 | TRUE | `:13 pub const MAOS_ENV_REGISTRY: &[EnvVar] = &[`; gate parses lines starting `name: "MAOS_` textually (`check_env_contract.rs:107`) |
| 28 | `MAOS_OPERATOR_BEARER_TOKEN` / `MAOS_OPERATOR_HTTP_BIND` unregistered | 15-1 AC5 | TRUE | gate: `main.rs:2669`, `:2670` "not in env_contract.rs registry" |
| 29 | `t_14_2a_post_grace_journal.rs:362` red, reproduced under default threads | 15-1 AC6 | TRUE | `crates/maos-a2a-tcp/tests/…:362 assert!(… "cert_post_grace_reject")`; FAILED 2/2 (`captured tracing: []`); `--test-threads=1` PASS |
| 30 | cause = "already uses `with_default` at `:267` on a `current_thread` runtime" | 15-1 AC6 | PARTIAL | `:267 tracing::subscriber::with_default(LogCapture…)`, `:268 new_current_thread()` — both TRUE as citations, **but not the cause**: the test PASSES alone under default threads (`cargo test … t_14_2a_a_promoted` → 1 passed) and PASSES at `--test-threads=2`; it fails only with all three tests concurrent. Cause = inter-test interference in the 3-test binary (no `static`/`OnceLock` in `maos-a2a-tcp/src`; candidates: tracing callsite-interest cache, timing of the handshake arm `transport.rs:1002-1040`). Root cause still unmeasured — the AC should say so |
| 31 | `check_cert_rotation_trigger.rs:613` `--test-threads=1` | 15-1 AC6 | TRUE | `:613 "--test-threads=1",` |
| 32 | `release.yml:44` builds no `maosctl` | 15-4 AC1 | TRUE | `:44 run: cargo build --release -p maos-bin --locked --target …` |
| 33 | `download-artifact@v4` without `merge-multiple` → "Is a directory" | 15-4 AC1 | TRUE | `:67-69 uses: actions/download-artifact@v4 / path: dist` (no `merge-multiple`) → each artifact lands in `dist/<name>/`; `:73 sha256sum maos-* > SHA256SUMS` |
| 34 | `sign-and-publish` `needs: build` only | 15-4 AC3 | TRUE | `:56-57` |
| 35 | aarch64 targets never compiled in CI | 15-4 AC2 | TRUE | `:18-23` matrix; `release.yml` is the only workflow mentioning aarch64; no `v*` tag has ever existed |
| 36 | `release_verify.rs:280-291` `option_env!` | 15-4 AC4 | TRUE | `crates/maos-audit/src/release_verify.rs:280 fn production_pubkey_must_differ_from_dev_seed`, `:287 if option_env!("MAOS_RELEASE_PUBKEY").is_some()`; `:283-285` comment admits the unset case is "trivially true" |
| 37 | `stability_matrix.rs:17,137` | 15-4 AC5 | TRUE | `xtask/src/stability_matrix.rs:17` (`kernel_version` sourced from `[workspace.package].version`), `:137 parse_workspace_version` |
| 38 | Workspace version → `0.1.0-alpha.1` | 15-4 AC5 | TRUE (needed) | `Cargo.toml:64 version = "0.1.0-alpha"`. Blast radius of the literal `0.1.0-alpha`: 25+ files (12 Spirit manifests, `templates/spirit-rust/manifest.toml`, `manifest_n_minus_1_compat.rs` ×2, `worker/tests/cli_wrapper_admission.rs`, docs-site) — which of these bind to the workspace version: NOT CHECKED |
| 39 | "`Cargo.lock` committed" | 15-4 AC5 | PARTIAL (vacuous) | `git ls-files Cargo.lock` → tracked at HEAD; `.gitignore` excludes only fixture locks (`:10-12`) |
| 40 | `docs/release/tag-procedure.md` written | 15-4 AC5 | TRUE (absent) | `docs/release/` exists (3 files); `docs/runbooks/release-signing.md` already covers key provenance/locations — overlap to reconcile |
| 41 | `ops-provisioning-secrets-and-accounts` row | 15-4 AC6, header | TRUE | `sprint-status.yaml:281` |
| 42 | kloc instrument | 15-2 | TRUE (tokei `code`) | `xtask/src/kloc_check.rs:163-213`: runs `tokei --types Rust`, sums `report.stats.code`; excludes `examples`, `spirits` (`:180,:184`), tests/benches (`kloc.toml:2`). Kernel pin counts **physical** lines (`check_kernel_baseline.rs:6-7`) — two instruments |
| 43 | `MEASURED GRANT 2026-09-xx (operator-authorized)` "existing format" | 15-2 AC2 | TRUE | inline: `xtask = 41953   # ⚠ 14-2a MEASURED GRANT 2026-08-30 (operator-authorized, …` (`kloc.toml:212`; also `:319,:343,:482`); comment-line: `# Story 14-2c MEASURED GRANT 2026-09-03: 41932 -> 41939 (+7),` (`:203,:316,:334,:471`). Closer precedent for a whole-file re-base: `2026-07-25 founder policy re-base (Winston 2%)` (`:275,:278,:325,:328,:344`) and `2026-08-11 Epic-13 retrospective re-base` (`:501`, commit `08ab6632`) |
| 44 | "formula at `kloc.toml:82`" | 15-2 AC2 | **FALSE** | `:82` = "# • The Epic-5 decomposition plan this posture served was never arithmetically". The formula is `:58-59`: `ceiling(crate) = measured_tokei_code + max(100, ceil(0.02 * measured))` / `_aggregate_hardfail = measured_aggregate + max(100, ceil(0.02 * measured))` |
| 45 | "retro-only rule at `kloc.toml:501`" | 15-2 AC3 | PARTIAL | `:501` is the `_aggregate_hardfail = 147057` row whose comment says "the CEILING RULE names an epic retrospective as the ONLY sanctioned recalculation point for the aggregate". The rule text is `:61-62`: "Recalculated ONLY at an epic retrospective, **or under an explicitly authorized measured grant**. NOT per story." — per-crate grants outside a retro are already permitted; the retro-only reading applies to the aggregate. Amend `:61-65` and `:501` together |
| 46 | `check_kernel_baseline.rs:7` equality | 15-2 AC4 | TRUE | `:7-8 "Hard-fails on ANY drift"` |
| 47 | AC1 headroom table crates exist | 15-2 AC1 | TRUE | all 18 exist under `crates/` (workspace has 55 members) |
| 48 | AC1 table = "re-base needed" | 15-2 AC1 | PARTIAL | Four rows already have the headroom asked: maos-control 803/1500 (697 ≈ +700), maos-secrets 161/1000 (839 > +300), maos-providers 1252/2000 (748 > +400), maos-shell 305/405 (100 = +100). See §E |
| 49 | "kernel-core +150 for the named FLAG-Winston deltas" | 15-2 AC1 | PARTIAL | Reverses the 2026-08-11 retro's explicit refusal ("maos-kernel-core (38 headroom) … stay tight because Epic 14 is declared ZERO kernel-Δ", `:501`; `:195` "ZERO HEADROOM … Slack is operating capacity, NOT authorization"). 15-1 AC2's own +2 is NOT in the named list (16-5, 17-1, 17-2, 21-5) yet needs kernel-core kloc lines |
| 50 | any gate rejects a `kloc.toml` re-base outside a retro? | Q4 | TRUE = none | No xtask parses grant comments (`grep "MEASURED GRANT" xtask/src/*.rs` empty; `check_epic_close_coherence.rs:38` only keeps `prior:` verbatim). The retro-only rule is prose |
| 51 | "~20 crates escape kloc.toml" (E13 retro flag) | Q4 | RESOLVED at HEAD | 47 keys in `kloc.toml`; 11 workspace members unlisted = 10 `spirits/*` + `examples/example-spirit`, both directories excluded by `kloc_check.rs:180,184`; 3 keys with 0 measured are forward-declared (`maos-cap-registry`, `maos-journal`, `maos-wire`) |
| 52 | `tests/phase-config.toml` sole consumer `main.rs:289`; comparison `coverage_matrix.rs:98` | 15-3 AC2 | PARTIAL | `main.rs:289` is only the `--phase-config` **default value**; the loader is `coverage_matrix.rs:81`, comparison `:98-99`, order check `:119`. `v1_5`→`v1.5` mapping is real (`gate_common.rs:161 PHASE_ORDER ["v1_0","v1_5","v2_0","v2_2"]` vs `phase-config.toml:6-18` dotted) |
| 53 | "single phase source" is one concept | 15-3 AC2 | **FALSE** | `gate_common.rs:163-166`: ship-ladder phase, "NEVER dev-time enforcement". `phase-config.toml:1-2`: "single source of truth for current phase … coverage-matrix.yaml current_phase MUST match" = delivered phase `v0.1-alpha`. Measured with scratchpad copies at `v1.5`: **145 violations** (`coverage-matrix --json`), 0 at HEAD; advisory because `mode: warning` (`coverage-matrix.yaml:3`; `coverage_matrix.rs:65` fails only in `hard`) |
| 54 | `check-exit-commands` resolves `maos` tokens "against the binaries' `--help`" | 15-3 AC3 | **FALSE** for `maos` | maos-bin has no clap (0 uses), hand-parsed argv (`main.rs:1653`); `maos --help` boots the daemon and needs SIGTERM (observed). `maosctl --help` OK (clap); xtask OK (clap, `main.rs:949` style) |
| 55 | fenced-block parser precedent in xtask | Q5 | NONE | the 5 files containing ``` use it in doc comments only (`check_rto.rs:126-131`, `check_ship_gate_completeness.rs:355-361`) — trivial to write, no precedent |
| 56 | `check-cna-registration` "file-presence only", `:51-76` | 15-3 AC4 | PARTIAL | `check_cna_registration.rs:54-60` absent-PASS (`docs/compliance/cna-registration.md` is absent at HEAD → gate vacuous today); when present it content-validates `SECURITY.md` (`:73-88`). It IS the "one absent-pass gate" rule 1 wants retired — but the AC names 2 of the 6 enrolment sites |
| 57 | retirement touches `gate-registry.toml` + `discipline.yml` | 15-3 AC4 | PARTIAL | also `xtask/src/main.rs:96,:949,:1360`; `check_ship_gate_completeness.rs:34` (hardcoded `EXPECTED_GATES`); `tests/coverage-matrix.yaml:1277` (else `coverage_matrix.rs:141` reds "gate not in registry"); `discipline.yml:2594-2605, :3401, :3453, :3490`; `gate-registry.toml:72, :215-217`. `gate-registry.toml:2-3`: removal "requires invariant-lock review" |
| 58 | "enrolled Blocking" / `BindingClass` | 15-3 AC3/AC1 | TRUE (mechanism) | Blocking in CI = job in `aggregate.needs` without `continue-on-error` (`discipline.yml:3510-…`); `BindingClass` is the xtask leg-level enum (`gate_common`; used by `check_enterprise_identity.rs:567`, `check_enterprise_pdp.rs:476`, `check_multi_region_slo.rs:53-…`). `check_escape_detector.rs` uses `CURRENT_PHASE` via `is_blocking_at` (`:578`) and no `BindingClass` (grep) — AC1's "uses BindingClass" is a TODO, correctly |
| 59 | ADR-060…063 free; `maos-control/src/lib.rs:68-76` | 15-5 AC1 | TRUE | highest is `ADR-059-operator-authority-collective-erasure.md`; frontmatter shape `Status/Gate/Decided/Accepted-in-PR/Amends/Reuses` (`ADR-059:1-8`). `lib.rs:68-76`: "READ-ONLY, and that is a design decision … would invent a SECOND trust path … `main.rs:9844-9853` warns against" |
| 60 | `RELEASE-HOLDS.md`, `STABILITY.md`, `docs/runbooks/`, `--features wasm-host` | 15-5 AC2/AC3 | TRUE / PARTIAL | all exist; `crates/maos-bin/Cargo.toml:30 wasm-host = ["dep:maos-wasm-host"]`. `RELEASE-HOLDS.md:29-33,:71` ALREADY states WASM engine off until Hold 2 → AC3 is half-vacuous (STABILITY.md has no wasm mention → new) |
| 61 | PRD `[DELTA-2026-09-04 R2]`, architecture §13.1 note | 15-5 AC4 | TRUE | `prd/project-scoping-phased-development.md:18`; `architecture-maos-minimal-opus/13-phased-roadmap.md:5` |
| 62 | `InferencePortAdapter` `main.rs:3136`; `MAOS_REPLAY_CASSETTE` `:4607-4640` | 15-6 AC1 | TRUE | `crates/maos-bin/src/main.rs:3136 let inference = InferencePortAdapter::new(`; `:4609` (record) and `:4623` (replay) inside the Researcher branch (`:4599 researcher_inference`, `:4621/:4637 with_deferred_inference_port`) |
| 63 | `maos run <manifest> --once` | 15-6 AC1 | TRUE | `main.rs:371, :1653, :3335` |
| 64 | "for **every** Spirit" | 15-6 AC1 | **FALSE (vacuous)** | Consumers at HEAD: Researcher (`spirits/researcher` 19 `InferencePort` refs) and shell/hello-spirit (`main.rs:3247-3250` → `maos_shell::run_shell`; `maos-spirit-hello/src/lib.rs:79`). Butler 0 refs (`McpClientPort` only, `spirits/butler/src/lib.rs:812-825`); Mira/Nash/Observer/Orchestrator declare "no inference" (`spirits/*/tests/spirit_smoke.rs` Decisions E/G/I); Architect/Reviewer/Digest/Worker 0 refs |
| 65 | "`live` default only when a provider is configured, otherwise typed `Unconfigured` halt, non-zero exit" | 15-6 AC1 | PARTIAL | `UnconfiguredProvider` already exists (`main.rs:3121-3124`). Today an unconfigured Researcher run takes the deterministic path and exits 0 (`main.rs:4642-4645`). **10** sites run `maos run spirits/researcher/manifest.toml --once` without `--live` (`cohort_daemon_smoke_13_5c.rs:732`, `researcher_8_14c.rs:151,182,238`, `inference_provider_polarity_8_11.rs:45`, `cross_team_crossing_13_6b.rs:2647`, …) — a non-zero default would red them |
| 66 | "Cassette format versioned" | 15-6 AC2 | PARTIAL (vacuous) | already `schema_version: "maos.journey.cassette/v1"` (`cassette_replay.rs:28-33, :185`). Provenance today = `model_id: "hand-authored-seed"` in all three cassettes (`cassettes/j-butler/on-idle-halt.json:4`, `j-researcher/survey-distill.json:4`, `j0/shell-intro.json:4`) |
| 67 | `journey-nightly.yml:94` `--live`; `tier-2-rerecord` | 15-6 AC3 | TRUE | `:94 --live` (after `--no-fail-fast`, not after `--` → nextest rejects it); job `tier-2-rerecord:` at `:65`; it sets `MAOS_JOURNEY_MODE: record` (`:87`), which `main.rs:4608` reads; `MAOS_JOURNEY_MODE` is registered (`env_contract.rs:260`) — AC must say whether it is retired or aliased |
| 68 | `MAOS_INFERENCE_MODE` | 15-6 | NEITHER at HEAD | grep empty across `crates spirits xtask .github` — created by 15-6 AC1/AC4 |
| 69 | `MAOS_REPLAY_CASSETTE` registered | 15-6 AC4 | TRUE | `env_contract.rs:255` |

## B. Verb-first

| Token | Status |
|---|---|
| `cargo run -p xtask -- check-empty-kernel` / `check-service-boundary` / `kloc-check` / `check-env-contract` | exist (ran) |
| `cargo test --workspace --no-fail-fast` | cargo built-in; suite state NOT RUN (brief). Known red: `t_14_2a` (measured). Other reds: not checked |
| `cargo run -p xtask -- check-exit-commands` | created by **15-3 AC3**; **not** at HEAD. The exit runs it → 15-3 must land before the exit is evaluated (Dependencies line does not say so) |
| `git tag v0.1.0-alpha.1 && git push --tags` | git built-ins; network + push credential; triggers `release.yml` which needs `RELEASE_SIGNING_KEY`/`MAOS_RELEASE_PUBKEY` (`:74-77,:82`) → **operator lane** (§F) |
| `maos run spirits/researcher/manifest.toml --once` (15-6 AC1) | exists (hand-parsed, `main.rs:1653`) |
| `MAOS_INFERENCE_MODE=replay\|record` | created by 15-6 |
| `ls docs/adr/ADR-06[0-3]-*.md` (15-5 AC1) | created by 15-5 |
| `grep -rho "const CURRENT_PHASE" xtask/src \| wc -l` (15-3 AC1) | works today (prints 8) |
| **Oracle gap**: `maos --help` | does not exist as a help surface (daemon boots) — 15-3 AC3's oracle for `maos` tokens is missing |

## C. Foundation audit

| Story | Assumed mechanism | Verified? |
|---|---|---|
| 15-1 | `maos_attrs::i9_exempt` proc-macro (`crates/maos-attrs/src/lib.rs:10`); `check-empty-kernel` denylist/whitelist/exemption cross-check (`check_empty_kernel.rs:102-192`); `i9-exemptions.md` name-match (`:159-169`, matches on bare struct name) | YES |
| 15-1 | a way to regenerate `kernel-surface-v0.1-beta.json` | **NO flag exists** (#23) |
| 15-1 | `check_cert_rotation_trigger.rs` invoking cargo test with `--test-threads=1` | YES (`:608-614`) |
| 15-2 | `kloc.toml` grant-comment convention; formula; retro rule | YES, but at `:58-65`, not `:82`/`:501` |
| 15-3 | `gate_common::CURRENT_PHASE` + `is_blocking_at` ladder; `BindingClass` | YES |
| 15-3 | a `--help` oracle for `maos` | **NO** (#54) |
| 15-3 | markdown-fence parser | none; trivial |
| 15-3 | gate enrolment = registry list + `[[ship_gate]]` + discipline job + aggregate `needs` + `EXPECTED_GATES` const + coverage-matrix row | YES; AC4 under-lists (#57) |
| 15-4 | cross-workflow `needs:` | **NO** (`workflow_call`/`workflow_run` absent) |
| 15-4 | `release-verify --sign/--verify` xtask verbs | exist (`release.yml:74,:86`); NOT RUN |
| 15-4 | macOS runner availability / minutes | operator/billing — NOT CHECKED |
| 15-5 | ADR index convention (`docs/adr/index.md` exists); no gate parses ADR numbering (only `check_adr_040_accepted.rs`) | YES |
| 15-6 | `CassetteRecordPort`/`CassetteReplayPort` (`crates/maos-bin/src/cassette_replay.rs:8,:153`) | YES — Researcher-wired only |
| 15-6 | one InferencePort object shared by all Spirits | **NO**: two adapters are built (`:3136` shared/shell, `:4599` Researcher-private); most Spirits take no port |
| 15-6 | `env_contract.rs` registration + gate | YES |

## D. Kernel-Δ audit

| Story | Zero-Δ possible? | Notes |
|---|---|---|
| 15-1 | **No** if attribute path: +2 (single-line) or +6 (kernel's own 3-line style); attribute move net 0. Whitelist path = 0 kernel lines but wrong mechanism (path-scoped, declares a scheduler struct a "persistent-state holder"). | Re-pin 24472→24474 (or 24478) in the same commit; **also** needs kernel-core kloc lines (18933/18933) → 15-2 must grant them and name 15-1 |
| 15-2, 15-3, 15-4, 15-5 | Yes | no kernel files |
| 15-6 | Yes | wrap at the composition root (`maos-bin`); `InferencePortAdapter` itself (`crates/maos-kernel-core/src/inference/mod.rs:38`) untouched |

Epic's Kernel-Δ line is TRUE in substance; the "or 0" branch should be dropped (fork resolution, §H).

## E. Kloc headroom (kloc-check @9f920180: measured / ceiling / headroom → 15-2 table ask)

| Crate | now | ask | Comment |
|---|---|---|---|
| maos-bin | 17109/17109/**0** | +1200 | 15-1 AC5 (~10), 15-6 (~150-300) need it → 15-2 first |
| maos-control | 803/1500/697 | +700 | ≈ already there; re-base +3 |
| maos-cli | 5270/5270/**0** | +400 | no Epic-15 story touches it |
| maos-shell | 305/405/100 | +100 | already there |
| maos-secrets | 161/1000/839 | +300 | already there |
| maos-domain | 8695/8644/**−51** | +300 | red at HEAD (D14) |
| maos-eval | 3661/3696/35 | +600 | — |
| maos-wasm-host | 1057/1129/72 | +400 | — |
| maos-registry | 3515/3615/100 | +300 | — |
| maos-spirit-cli | 753/853/100 | +300 | — |
| maos-providers | 1252/2000/748 | +400 | already there |
| maos-a2a-core | 4856/4856/**0** | +150 | D10 wall (`kloc.toml:319`) |
| maos-a2a-tcp | 1439/1500/61 | +150 | 15-1 AC6 edits a test file (uncharged) |
| maos-cohort | 5857/5857/**0** | +100 | — |
| maos-iac | 6960/6960/**0** | +100 | — |
| maos-audit | 6847/6847/**0** | +100 | — |
| maos-kernel-core | 18933/18933/**0** | +150 | 15-1 AC2 +2/+6 needs this |
| xtask | 41953/41953/**0** | net ≤0 | 15-3: +≤60 gate +~40 phase generator −260-line `check_cna_registration.rs` (tokei code fewer) −7 consts ≈ net negative **only if landed in one commit**; 15-1 AC6 −1 |
| aggregate | 155498 / 147057 | — | formula (`:59`) at HEAD = 155498 + 3110 = 158608 before any grant |

Grants of +1200/+700/+600 are NOT the ratified formula (`max(100, 2%)` → maos-bin 343) — 15-2 is a **CEILING RULE amendment**, not a re-base (§H).

## F. Hermetic vs operator

- Exit line 3 (`git tag … && git push --tags`) — network, push credential, and the job it triggers needs two secrets; move to `ops-provisioning-secrets-and-accounts` (or a new `ops-first-signed-tag` row). The hermetic substitute: a `workflow_dispatch` dry-run of `release.yml` with signing skipped, or a `release-dry-run` job in discipline that builds `maos`+`maosctl` for the three targets and runs the artifact-merge + `sha256sum` step (no secrets).
- 15-4 AC2 (aarch64-apple-darwin) needs a macOS runner — hosted minutes at 10× — engineering-runnable but a cost line; not a secret.
- 15-4 AC6 correctly operator-gated. 15-5 AC2 correctly operator-lane.
- 15-6 AC3 `tier-2-rerecord` uses `MAOS_ANTHROPIC_API_KEY` (`journey-nightly.yml:88`) — paid; correctly outside the exit.

## G. Sizing from measurement

| Story | Analogs (landed) | Measured | +30% |
|---|---|---|---|
| 15-1 | 8.16 bridge `e2bb7775` (13 files, +917/−141, 1 day 2026-06-12); 14-0 `38c52811` (32 files, +2460/−192, ready→done same day 2026-08-26) | 1.5–3 d | 2–4 d |
| 15-2 | `08ab6632` E13 re-base (1 file +10/−3, 1 d); `5bcc3c76` D13(a) grant (3 files, 1 d) | 0.5–1 d + ratification latency | 1–1.5 d |
| 15-3 | `3dc34c61` Option-C gates (12 files +592/−42, 1 d); `7b8b4d31` epic-close-coherence (7 files +603/−16, 1 d); + `maos` help-oracle gap | 2–3 d | 2.5–4 d |
| 15-4 | 9-4 `3cf93ada` (55 files +4198/−413, ~2 d) as upper analog; CI-only iteration loop (tag or dispatch per try) | 2–4 d | 2.5–5 d |
| 15-5 | ADR-053 (in 11-7 `5b749b4d`), ADR-056/057 (planning `a0958010`) ≈ 0.5–1 d each incl. round-table; 061/062 are security-design decisions (◆) | 3–5 d | 4–6.5 d |
| 15-6 | 8-15 `454dba44` (28 files +2323, 1 d) built the cassette seam; generalization + mode env + provenance + nightly fix + 10-site default preservation | 2–3 d | 2.5–4 d |
| **Total** | | 11.5–19 d | **14.5–25 d ≈ 3–5 wk** |

Epic states 2–3 weeks. The lower bound is reachable only if 15-5 runs in parallel and 15-4's CI loop converges in one or two tries.

## H. Fork audit (rule 7)

1. **15-1 AC2** "either attributed … or added to `xtask/i9-whitelist.toml` … the choice is written in the story" → **Resolve now: attribute, +2 single-line, re-pin 24474.** The whitelist is path-scoped (`check_empty_kernel.rs:136-145,:243-244`) and would declare `scheduler/control_block.rs` and `t3/image_lock.rs` sanctioned persistent-state holders (`I9.md` "Whitelist: Persistent state is permitted only inside …") — a claim standing in for a control.
2. **15-1 AC3** baseline regeneration blesses ~8 genuinely removed symbols against "monotonically additive" → decide: invariant-lock ratification of the removals (with the ABI Stability Triple note), not "a one-line rationale".
3. **15-2** allowance policy (+1200 etc.) vs ratified formula `:58-59` → decide: amend the CEILING RULE (`:49-65`) to "measured + epic-scoped operator allowance", or keep the formula and grant per-story. The AC must name which.
4. **15-3 AC2** one phase source → decide: (a) consolidate the 7 duplicate ship-ladder consts into `gate_common` (8→1) and **leave** `tests/phase-config.toml` as the delivered-phase file (rename `current_phase` → `delivered_phase` to end the confusion), or (b) move delivered phase to `v1.5` and accept/triage 145 advisory coverage violations. Recommend (a).
5. **15-3 AC3** `maos` oracle → decide: (a) add a real `--help`/verb table to maos-bin (maos-bin lines; after 15-2), or (b) resolve `maos` tokens against a checked-in verb manifest that a test asserts equals the hand-rolled dispatcher. Recommend (a) — it also serves rule 1 for every later epic.
6. **15-4 AC3** cross-workflow dependency → decide: `workflow_call` (heavy: 136 jobs on every tag), `workflow_run` (async), or a `gh api` check-run query for the tagged SHA requiring `aggregate == success` (cheap, explicit). Recommend the check-run query, named in the AC.
7. **15-5 AC1 ADR-063** "reissue-is-revocation or CRL" → fine as the ADR's own question, but the AC must say "chooses exactly one".
8. **15-6 AC1** default when `MAOS_INFERENCE_MODE` is unset → decide: unset = today's deterministic path (keeps 10 sites green), `replay` requires a cassette, `live` requires a provider else `Unconfigured` non-zero. Recommend that; "live is the default when configured" as written changes the exit code of every existing unconfigured run.
9. **15-6 AC3** `MAOS_JOURNEY_MODE=record` (registered, read at `main.rs:4608`) vs new `MAOS_INFERENCE_MODE=record` → retire or alias; say which.

## I. Dependency audit

- "None (opens the lane)" — TRUE.
- "15-2 before 15-1" — TRUE and stronger than stated: 15-1 AC2 needs kernel-core kloc lines (0 headroom) and AC5 needs maos-bin lines (0 headroom).
- **Missing**: 15-3 before the exit block's line 2 can run; 15-1 (aggregate green) before 15-4 AC3 can be true; 15-2 before 15-6 (maos-bin 0 headroom). 15-5 is independent (docs). Suggested order: 15-2 → 15-1 → 15-3 → {15-6, 15-5} → 15-4 (tag last, after the ops row).
- No story needs a later epic's mechanism. 15-5's ADR-062 (mutating operator surface) is consumed by 16-1 — correctly "15-5 before Epics 16–17 open".

## J. Did R2 fix the prior preflight (old E15 → new E15)?

| Prior finding | Status | Evidence |
|---|---|---|
| `release.yml` `download-artifact@v4` defect | fixed (15-4 AC1) | #33 |
| two secrets do not exist | fixed → ops row | `sprint-status.yaml:281` |
| aarch64 never compiled | fixed (15-4 AC2) | #35 |
| I9 fix needs kernel lines | fixed in substance, **fork left open** | #11-#12, §H-1 |
| 16 (not 9) `other` symbols | fixed | #20 (17 rows/16 unique) |
| 19 "removed public symbol" rows | renamed to "signature-changed" — **partly wrong** | #21 (~8 genuinely removed) |
| 8 `CURRENT_PHASE` constants | fixed | #2 |
| §2.2 kloc re-base in W0 | fixed (15-2), citations wrong | #44-#45 |
| §2.4 inference seam outside Researcher | added (15-6) but "every Spirit" vacuous; shell consumer missed | #5, #64 |
| §2.6 provisioning checklist | fixed (15-5 AC2) | — |
| `journey-nightly.yml:94 --live` | fixed (15-6 AC3) | #67 |
| `t_14_2a` root cause unmeasured | **still unmeasured**; new AC asserts a cause the measurement contradicts | #30 |
| `sign-and-publish` needs only `build` | addressed (15-4 AC3) with an **impossible mechanism** | §H-6 |
| §2.5 exit commands name what a machine can run | line 3 still non-hermetic | §F |

## K. Verdict, required changes, confirmed-true

**Confidence 55.** The epic's direction and every seam are real; the five gates and the test red reproduce exactly as claimed, the crate inventory and phase-constant counts are right, and the four ADR numbers are free. What keeps it below 75: two ACs rest on mechanisms that do not exist at HEAD (`maos --help`; cross-workflow `needs:`), one AC unifies two different phases and would surface 145 coverage violations, one AC asserts a root cause that a three-run measurement contradicts, "every Spirit" is true for two of ten, two forks remain inside ACs, and the exit block's third line is neither hermetic nor secret-free. All are edit-level fixes; none require a new epic. **Verdict: needs-rework. Measured duration 3–5 weeks.**

### REQUIRED CHANGES (OLD → NEW)

1. **15-1 AC2** — OLD: "`T3ImageVerificationConfig` (`security/mod.rs:124`) and `ValidatedRevocationRules` (`revocation/rules.rs:19`) documented in `docs/invariants/i9-exemptions.md`; … either attributed (+2 kernel lines, re-pin 24472→24474 in the same commit) or added to `xtask/i9-whitelist.toml` with an ADR-037 rationale — the choice is written in the story."
   NEW: "`ValidatedRevocationRules` (`revocation/rules.rs:19`) gets its entry in `docs/invariants/i9-exemptions.md` (§Entries format, `:12-16`); the 3-line `#[maos_attrs::i9_exempt]` at `security/mod.rs:120-122` moves onto `SecurityManagerAdapter` (`:130`, already documented at `i9-exemptions.md:91`, net-0; `T3ImageVerificationConfig` then carries no attribute and needs no entry); `ScbRuntimeSnapshot` (`scheduler/control_block.rs:244`) and `VerifiedImageLock` (`security/sandbox/t3/image_lock.rs:27`) get **single-line** `#[maos_attrs::i9_exempt(reason = "…")]` (precedent `maos-capability/src/cap_quota/mod.rs:37`) with entries in the register: +2 kernel lines, `kernel-core-baseline.toml` re-pinned 24472→24474 and a `maos-kernel-core` kloc line +2 in the same commit (both instruments). The whitelist path is NOT taken (path-scoped, `check_empty_kernel.rs:243-244`)."
2. **15-1 AC3** — OLD: "…regenerated for the 19 signature-changed rows, each with a one-line rationale." NEW: "…the 19 `removed public kernel symbol` rows (17 unique) are split by `check-service-boundary` output into signature-changed (re-classified in `kernel-api-classes.toml`) and genuinely removed (`SpawnError`, `UpgradeError`, `UpgradeOrchestrator`, `UpgradeOutcome`, `UpgradeReport`, `UpgradePolicy`, `RevocationApplier`, `get_default_image`); the baseline JSON is re-emitted for both under an invariant-lock review note that records the removals against NFR-Test-2's additive rule (`check_service_boundary.rs:227`). Since no regeneration flag exists (`main.rs:204-208` is input-only), the story adds `--write-baseline` to xtask or hand-edits with the diff attached."
3. **15-1 AC6** — OLD: "…its cause recorded (the test already uses `with_default` at `:267` on a `current_thread` runtime) and fixed…" NEW: "…reproduced: FAIL under default threads with the 3-test binary (2/2), PASS alone, PASS at `--test-threads=1` and `=2` — i.e. inter-test interference, not the subscriber arrangement at `:267-268`; the root cause is measured and recorded before the fix; `check-cert-rotation-trigger` drops `--test-threads=1` (`check_cert_rotation_trigger.rs:613`)…"
4. **15-1 AC4** — OLD: "the two env lines in `maos-bin`" NEW: "the two `EnvVar` blocks (~10 lines, `env_contract.rs:14-18` format) in `maos-bin`".
5. **15-2 AC2** — OLD: "`_aggregate_hardfail` re-derived by the formula at `kloc.toml:82`" NEW: "…by the formula at `kloc.toml:58-59` (`measured + max(100, ceil(0.02·measured))`, = 158608 at HEAD before grants)". Add: "The per-crate allowances in AC1 exceed that formula; AC2 therefore also amends the CEILING RULE text (`kloc.toml:49-65`) to name the epic-scoped operator allowance as a third sanctioned source, and names 15-1 AC2 (+2) among the kernel-core FLAG deltas."
6. **15-2 AC3** — OLD: "The retro-only rule at `kloc.toml:501` is amended" NEW: "The rule at `kloc.toml:61-62` ('Recalculated ONLY at an epic retrospective, or under an explicitly authorized measured grant') and the aggregate comment at `:501` are amended together: the aggregate is re-derived at epic close **or** by an operator-ratified re-base story."
7. **15-2 AC1** — add: "Rows whose HEAD headroom already covers the ask (maos-control 697, maos-secrets 839, maos-providers 748, maos-shell 100) are recorded as `no change` in the table, not re-based."
8. **15-3 AC2** — OLD: "`tests/phase-config.toml` is generated from `gate_common` with an explicit `v1_5`→`v1.5` mapping … the coverage-matrix phase move is recorded." NEW: "The seven per-gate `const CURRENT_PHASE` copies are deleted in favour of `gate_common::CURRENT_PHASE` (ship-ladder phase, `gate_common.rs:163-166`). `tests/phase-config.toml` stays the **delivered**-phase source (`v0.1-alpha`, consumed by `coverage_matrix.rs:81,:98`); its key is renamed `delivered_phase` with `coverage-matrix.yaml:2` updated in the same commit so the two phases can never be conflated again. (Measured: moving the delivered phase to `v1.5` yields 145 coverage-matrix violations.)"
9. **15-3 AC3** — OLD: "…resolves each `maos` / `maosctl` / `cargo run -p xtask --` token against the binaries' `--help`…" NEW: "…resolves `maosctl` and `xtask` tokens against `--help` (clap), and `maos` tokens against a new `maos --help` verb table added to `crates/maos-bin/src/main.rs` (hand-parsed argv today, `:1653`; `--help` currently boots the daemon — measured) whose completeness a maos-bin test asserts against the dispatcher; proven-red with a planted absent verb; enrolled Blocking (discipline job + `aggregate.needs` + `gate-registry.toml` `gates`/`[[ship_gate]]` + `check_ship_gate_completeness.rs:20` `EXPECTED_GATES` + a `tests/coverage-matrix.yaml` row)." Also add "lands after 15-2 (maos-bin 17109/17109)".
10. **15-3 AC4** — OLD: "`gate-registry.toml` and `discipline.yml` updated" NEW: "retired at all six sites: `xtask/src/main.rs:96,:949,:1360`, `gate-registry.toml:72,:215-217`, `discipline.yml:2594-2605,:3401,:3453,:3490`, `check_ship_gate_completeness.rs:34`, `tests/coverage-matrix.yaml:1277` (invariant-lock review per `gate-registry.toml:2-3`); xtask net lines ≤0 measured in the same commit that adds `check-exit-commands`."
11. **15-4 AC3** — OLD: "`sign-and-publish` `needs:` the discipline aggregate, not only `build`." NEW: "`sign-and-publish` refuses to publish unless the tagged SHA has a successful `discipline / aggregate` check-run (`gh api repos/:owner/:repo/commits/<sha>/check-runs` filtered on name `aggregate`, conclusion `success`), because a job cannot `needs:` across workflows (`release.yml:2-4` vs `discipline.yml:3-7`; no `workflow_call`/`workflow_run` exists)."
12. **15-4 AC5** — OLD: "`Cargo.lock` committed" NEW: drop (already tracked at HEAD) or "`Cargo.lock` stays committed and `--locked` builds pass on all three targets". Add: "`docs/release/tag-procedure.md` links, not duplicates, `docs/runbooks/release-signing.md`."
13. **15-5 AC3** — OLD: "`RELEASE-HOLDS.md` and `STABILITY.md` state…" NEW: "`STABILITY.md` gains the WASM-off-until-Hold-2 statement (`RELEASE-HOLDS.md:29-33,:71` already carries it)…"
14. **15-6 title/AC1** — OLD: "Inference record/replay seam for every Spirit … work for **every** Spirit through `InferencePortAdapter` (`main.rs:3136`)" NEW: "Inference record/replay seam for every inference consumer — the shared adapter at `main.rs:3136` (consumed by shell/hello-spirit via `:3247-3250`) and the Researcher-private adapter at `:4599` are both wrapped by one `MAOS_INFERENCE_MODE` selector; the eight Spirits that declare no inference (`spirits/*/tests/spirit_smoke.rs` Decisions E/G/I; Butler is MCP-only) are unaffected and inherit the seam when 18-1 wires them. Unset = today's deterministic path (the 10 existing `maos run spirits/researcher/manifest.toml --once` sites stay green); `replay` requires `MAOS_REPLAY_CASSETTE`; `live` requires a configured provider, else a typed `Unconfigured` halt with non-zero exit."
15. **15-6 AC2** — OLD: "Cassette format versioned;" NEW: "Cassette `schema_version` stays `maos.journey.cassette/v1` (`cassette_replay.rs:28-33`); a `provenance: seed|live-record` field is added and the three `model_id: "hand-authored-seed"` cassettes are re-stamped `seed`…"
16. **15-6 AC3** — add: "`MAOS_JOURNEY_MODE` (`env_contract.rs:260`, read at `main.rs:4608`) is retired in favour of `MAOS_INFERENCE_MODE=record`."
17. **Exit block** — OLD line 3: "`git tag v0.1.0-alpha.1 && git push --tags`" NEW: "`cargo run -p xtask -- release-dry-run` (new in 15-4: builds `maos` + `maosctl` for the three targets in discipline, merges artifacts, runs `sha256sum`, skips signing) — green with no secrets." Move the tag+push to `ops-provisioning-secrets-and-accounts` (or a new `ops-first-signed-tag` row) with the acceptance "release.yml green on `v0.1.0-alpha.1`". Add to Dependencies: "15-3 before the exit's `check-exit-commands` line runs; 15-2 before 15-6."
18. **Kernel-Δ line** — OLD: "≤2 attribute lines … if the ADR-037 whitelist path is not chosen." NEW: "+2 attribute lines (15-1 AC2), re-pin 24472→24474; no whitelist edit."

### CONFIRMED TRUE (do not re-check)

All four exit-line gates red at HEAD and `check-kernel-baseline` green at 24472 · 8 `CURRENT_PHASE` at the listed lines · 8 zero-headroom crates + maos-domain −51 + aggregate 155498/147057 · `i9_exempt` proc-macro, `i9-whitelist.toml` (paths), `i9-denylist.toml`, `i9-exemptions.md` (name-matched) · the four AC2 symbols at the cited lines and `SecurityManagerAdapter:130` misattribution · 17 `other` rows/16 unique; 19 removed rows · `kernel-api-classes.toml` row format · `env_contract.rs:13` + the two operator vars at `main.rs:2669-2670` · `t_14_2a:362` red reproducible, `:267/:268` citations, `check_cert_rotation_trigger.rs:613` · tokei-code instrument, all 18 named crates exist, no crate escapes kloc.toml · `check_cna_registration.rs:51-76` absent-pass; CNA doc absent at HEAD · `release.yml` lines 44/57/67-69/73; no `v*` tag ever · `release_verify.rs:280-291`, `stability_matrix.rs:17,137`, version `0.1.0-alpha`, `docs/release/` and `docs/runbooks/` exist · ADR-059 highest; `maos-control/src/lib.rs:68-76` GET-only by design; `wasm-host` feature at `maos-bin/Cargo.toml:30`; PRD/arch R2 deltas present · `InferencePortAdapter` at `main.rs:3136`, cassette sites `:4609/:4623`, `maos run --once`, `journey-nightly.yml:94 --live`, `tier-2-rerecord:65`, `MAOS_REPLAY_CASSETTE` registered `:255` · `maosctl --help` works (clap).
