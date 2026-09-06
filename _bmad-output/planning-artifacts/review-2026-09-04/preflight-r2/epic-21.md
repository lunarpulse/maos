# Preflight R2 — Epic 21 Multi-Host on Live Substrate (W6)

> Scout note, 2026-09-05. Epic file: `_bmad-output/planning-artifacts/epics/epic-21-multi-host-on-live-substrate-w6.md`. Every citation pinned `@9f920180` (HEAD verified by `git rev-parse`; `git status --porcelain` showed only the untracked `review-2026-09-04/preflight-r2/` directory, so worktree reads equal `git show 9f920180:`). Method: read-only; `kloc-check` and `check-empty-kernel` were RUN; `maos` was never executed; `tomllib` was used to parse the baseline file. Prior R2 notes (epic-15 … epic-20) were read at their verdict blocks and §K and are built on, not re-derived.

## Verdict block

| | |
|---|---|
| **Verdict** | **needs-rework** (not blocked: `demo-reza`, the loom-substrate action, `swap_serving_cert`, the `TofuPinStore` trait, `journal_peer_identity_refusal`, the 300-row κ corpus, the J4 topology file and the `orchestrator/` module are all real — but one AC observes a route that does not exist, one story's founding premise is inverted (the "ignored journeys" already run as CI gate legs), one AC is arithmetically impossible under 15-2's allowance, 21-1 stands on four undeclared mechanisms, and the Dependencies line names one vestigial edge and omits four real ones) |
| **Confidence** | **42 / 100** (proposal expects 55, `correct-course-input…confidence.md:56`) |
| **Measured duration** | **7.8–12.6 weeks** serial for 21-1..21-4 (9.1–14.6 with 21-5); **5.5–8.5 weeks** if 21-2 ∥ 21-4 ∥ 21-3 and 21-1 waits for 18-4 (epic says 5–7) |
| **False / partial premises** | **6 FALSE, 5 NEITHER, 25 PARTIAL** out of 80 audited claims (§A); 44 CONFIRMED TRUE |

**Top-3 blockers**

1. **21-3 AC1 observes a route that does not exist, and no route carries the observable.** `GET /v1/a2a/peer-versions` is not served; the operator surface has `GET /v1/cohort/peer-versions` (`crates/maos-control/src/lib.rs:347-348 @9f920180`) whose rows are `peer, declared_version, declared_hash, observed_at_secs, valid, state` (`:376-383`) — the 14-2b manifest-version convergence table, **no fingerprint**. `GET /v1/a2a/rotation-windows` (`:299`) carries `retiring/next/declared` fingerprints only for an OPEN window (`RotationWindowRow`, `:37-58`); `GET /v1/cohort/self-identity` (`:395`) reports the local leaf. Nothing reads the pin store. A durable pin therefore needs a NEW read surface (`maos-control` +40–80; the crate is at 803/1500) or a different observable. Underneath: `TcpA2ATransport.pins: Arc<InMemoryTofuPinStore>` (`crates/maos-a2a-tcp/src/transport.rs:106`) is consumed through two **inherent** methods that are not on the trait — `find_active_pin_by_fingerprint` (`crates/maos-a2a-core/src/tofu.rs:250`, called `transport.rs:1188`) and `verify_pinned_sync` (`tofu.rs:454-457`, "Additive; does NOT change the async trait surface") — so "typed to the trait" means extending `TofuPinStore` in `maos-a2a-core` (4856/4856, zero headroom, 15-2 grants +150). `maos-persistence` is an 8-line placeholder with no dependencies and no dependents (`crates/maos-persistence/src/lib.rs`, `Cargo.toml`; grep of workspace `Cargo.toml`s = only the member line `Cargo.toml:26`); its storage format is an undecided fork (§H).
2. **21-2's founding premise is inverted: the "ignored journeys" already run in CI as gate legs with a provisioned 3-DB substrate.** All four `#[ignore]` tests in `cross_team_crossing_13_6b.rs` (`:1641,:1837,:2354,:2810`) are legs of `check-multi-tenant-loom` and/or `check-loom-substrate-drift` (grep of `xtask/src/check_*.rs`), and the 3-team journey `reza_three_team_three_region_production_journey` runs in the `check-multi-tenant-loom` job that has `services:` + `provision-loom-substrate` with `maos_team_b maos_team_c` (`.github/workflows/discipline.yml:3077-3120`). All five `#[ignore]` mechanism tests in `t_12_4a_digest_read.rs` (`:298-526`) are `check-cohort-mesh` legs run `--ignored --exact` (`xtask/src/check_cohort_mesh.rs:435-505`; job `discipline.yml:3054`). Only the capture utility (`:593`, `t_12_4b_capture_j3_raw_inputs`) runs nowhere — by design: it `expect`s `MAOS_CAPTURE_J3_FIXTURE` (`:595-596`) and WRITES the committed fixture. `demo-reza` itself already runs the journey `--include-ignored` (`xtask/src/demo_reza.rs:241-245`). AC3 is likewise already true in CI: the `check-reza-production-path` job has Postgres (`discipline.yml:3130-3143`) and exports `TEAM_A/B` (`:3182-3184`), and `AdvisorySubstrate` hard-fails whenever the substrate is present (`xtask/src/gate_common.rs:216-221,229-233`; probe `check_reza_production_path.rs:25-26`). What 21-2 would add is a nightly SECOND runner of the same legs without a ledger, plus a bare `--run-ignored` (nextest requires a value; not executed), plus AC4's owner that is wrong (`journey_butler.rs:285` is `jb8_posture_shift_cognition`, "RED: Epic 9", `:284-289`; the Epic-18 note §K #12 deletes it from 18-4) and an exit line ("zero `#[ignore]` … without a named owner") that no gate implements (grep `xtask/src`, `gate-registry.toml` = 0).
3. **21-4 AC2 cannot land under 15-2's allowance, and 21-1 stands on four undeclared mechanisms.** `crates/maos-bin/src/env_contract.rs` is 507 physical / **479 tokei code** lines (87 `EnvVar {` entries); `maos-domain` is 8695/8644 (−51, `kloc-check` at HEAD), 15-2 grants it +300 (`epic-15-foundations-w0.md:57`), and 15-1 AC4 (+51), 16-4/16-5 (+36–66) and Epic 18 (+50–100) already draw on it — relocating the registry plus ~55 new entries (~+275) needs **≈ +750**. The only rationale for the `maos-domain` home ("avoid a kernel-crate-set member", `kloc.toml:278`, `sprint-change-proposal-2026-07-13.md:35`) evaporates once AC2 says "registered without a kernel edit". For 21-1: `--incidents` is not a `maos run` flag (`crates/maos-bin/src/worker_spawn.rs:53-66` accepts `--live`, `--once`, `--replay-llm`; unknown args error `:63-66`); `tests/fixtures/j4/` does not exist; no clock mechanism exists (Epic-18 note A#41-42; the only clock trait is `CohortClock`, `crates/maos-cohort/src/state.rs:25`) and 18-4 — which is to build it — is absent from the Dependencies line; and the `maos run` topology path loads Mira and Nash into ONE scheduler with **no A2A router between them** (`crates/maos-bin/src/main.rs:3797-3847`; every `LoopbackA2ARouter`/`handle_intake` site in `main.rs` is a smoke function, `:10786,:11040,:11227,:11303`), so AC3's "triggers the `handle_intake` deny path" first needs the accept path built. Mira does no inference at all (`spirits/mira/manifest.toml:16-17`; grep `Inference|provider` in `spirits/mira/src/lib.rs`, `spirits/nash/src/lib.rs` = 0), so "recorded model turns" must first be RECORDED with a paid key — the operator-lane leg the epic names but never files as an `ops-*` row (`sprint-status.yaml:281-284` has four ops rows, none for J4).

---

## A. Citation audit

| # | Claim | Where | Verdict | Evidence @9f920180 |
|---|---|---|---|---|
| 1 | Status/proposal/Fork C provenance | header :3 | TRUE | `sprint-change-proposal-2026-09-04-round2.md:57`; `sprint-status.yaml:273-279` |
| 2 | "5–7 weeks (measured)" | :5 | PARTIAL | no measurement cited; §G measures 7.8–12.6 wk serial |
| 3 | Closes NFR-Rel-6 | :7 | PARTIAL | NFR-Rel-6 = "Spirit-restart invalidates prior A2A TOFU pins; re-pin protocol with consent" (`prd/non-functional-requirements.md:25`) — already served by the in-memory store and pinned by `crates/maos-a2a/tests/restart_invalidates_pin_nfr_rel_6.rs:1-11` (6 scenarios). A durable store closes RELEASE-HOLDS row 12, not NFR-Rel-6, and must PRESERVE the invalidation semantics (AC1's "restart preserves pins" needs the boot-nonce rule stated) |
| 4 | Closes NFR-Maint-1 instrument | :7 | TRUE | `nfr…md:132` "≤ 20 KLOC excluding tests"; `kloc-check` tokei row `maos-kernel-core 18933` |
| 5 | Closes NFR-Sec-13 own-leaf | :7 | PARTIAL | NFR-Sec-13 text is the rotation chaos test + revocation latency (`nfr…md:46`); own-leaf rotation is RELEASE-HOLDS clause (c.2) (`RELEASE-HOLDS.md:76-110`) |
| 6 | "the 63 unregistered env reads" | :7, 21-4 AC2 | PARTIAL | measured: 86 registered (`env_contract.rs`); distinct `env::var/var_os("MAOS_*")` reads in `crates+spirits+xtask` src = 139, **55 unregistered**; incl. tests 148 read / **64 unregistered**; crates-only src 39; kernel-core reads 16, **15 unregistered** (`MAOS_REGISTRY_URI` is registered) |
| 7 | `cargo run -p xtask -- demo-reza` exists | exit l.1 | TRUE | `xtask/src/main.rs:1012-1013` (`--provision`, `--skip-gates`, `--journey-only`); `xtask/src/demo_reza.rs:66` |
| 8 | "already red on an absent DB" | exit l.1 | TRUE | `demo_reza.rs:71-81` → `Err("demo-reza: substrate incomplete…")` unless `--provision`; needs `psql` on PATH (`:139`) |
| 9 | "runs the 3-team journey `--include-ignored`" | exit l.1 | TRUE | `demo_reza.rs:231-245`: `cargo test -p maos-bin --test cross_team_crossing_13_6b reza_three_team_three_region_production_journey --include-ignored` |
| 10 | "the epic's first CI job" | :9 | PARTIAL | no workflow runs `demo-reza` (grep `.github/workflows` = 0) — but the journey it runs is ALREADY a `check-multi-tenant-loom` leg with a 3-DB substrate (`discipline.yml:3077-3120`) |
| 11 | "nightly: zero `#[ignore]` journey tests without a named owner" | exit l.2 | NEITHER | no gate (grep `xtask/src`, `gate-registry.toml`); the `#[ignore` hits in `xtask/src` are gates that RUN ignored tests; prose |
| 12 | operator lane "live J4 with a provider key (≥300 calls)" | :17 | PARTIAL | not an `ops-*` row (`sprint-status.yaml:281-284`: provisioning-secrets, external-cohort-pen-test, brew-tap, ga-tag) — rule 2 |
| 13 | Kernel-Δ ZERO for 21-1..21-4 | :19 | TRUE | but 21-4 AC1 REWRITES the pin value (instrument change) — a re-pin commit with zero kernel lines |
| 14 | 21-5 "about −1000 lines" | :19 | PARTIAL | −1028 physical / −817 tokei code (`tokei crates/maos-kernel-core/src/orchestrator/`) plus +2–6 port-type lines in `scheduler_loop.rs` |
| 15 | 21-3 closes RELEASE-HOLDS rows 12, 14-2(c) | table :33 | TRUE | `RELEASE-HOLDS.md:59` (row 12: `InMemoryTofuPinStore` only impl; listen side accepts ANY active pin); `:76-110` clause (c), (c.2) names `14-2d` |
| 16 | `MAOS_INFERENCE_MODE=replay` | 21-1 AC1 | NEITHER at HEAD → 15-6 | not in `crates/maos-bin/src/env_contract.rs` (only `MAOS_REPLAY_CASSETTE`, `:255`); created by 15-6 AC1/AC4 (`epic-15…:96,99`) |
| 17 | `maos run spirits/topologies/j4-mira-nash.toml` | AC1 | TRUE | file exists (290 B, `[[topology.spirits]]` mira + nash); run by `journey_j4.rs:50` |
| 18 | `--incidents <path>` | AC1 | NEITHER | `worker_spawn.rs:53-66`; no AC declares it |
| 19 | `tests/fixtures/j4/incidents-50.jsonl` | AC1 | NEITHER | `tests/fixtures/j4/` absent; no AC creates it |
| 20 | `--once` | AC1 | TRUE | `worker_spawn.rs:55` |
| 21 | "resolves 50 incidents from recorded model turns" | AC1 | NEITHER | no Mira/Nash cassette; both Spirits declare NO inference (`spirits/mira/manifest.toml:16-17`, `spirits/nash/manifest.toml:14-15`); recording is a paid act (§F) |
| 22 | "under a compressed scenario clock" | AC1 | NEITHER | no mechanism; Epic-18 note A#38,41,42 and §H-6 (place a `Clock` in `maos-spirit-sdk` under 18-4, consumed by 21-1) |
| 23 | "≥45/50 asserted" | AC1 | PARTIAL | source `prd/project-scoping-phased-development.md:226` "≥ 45/50 close in ≤ 90 min; ≥ 48/50 uphold typed-intent consent envelope"; "close" = incident-to-deployed-fix ≤ 90 min (`prd/product-scope.md:67`) — the AC states no close predicate and drops the 48/50 half |
| 24 | `spirits/mira/src/lib.rs:274` takes a signal | AC2 | TRUE | `pub fn diagnose(&self, signal: &AnomalySignal) -> Diagnosis` `:274`; `AnomalySignal` `:87-98` (subject, metric, observed, baseline, detail); fed only via `with_pending_signals` (`:241`) from `on_idle` (`:183-190`) — the daemon loads `Mira::default()` with NO signals (`main.rs:3827`) |
| 25 | "Mira and Nash gain inference seams" | AC2 | TRUE as delta | zero inference refs in both `lib.rs`; needs manifest `[capabilities.required]` (schema `deny_unknown_fields`, Epic-17 note) + daemon wiring beyond the scalar port (`main.rs:3797-3836`) |
| 26 | manifests carry `maos.mira.deterministic-diagnostic-v1` | AC2 | PARTIAL | only `spirits/mira/manifest.toml:67-68` `[model_provenance] covered_model_id`; Nash's manifest has no provenance section |
| 27 | `journey_j4.rs:117` ignored; topology triggers `handle_intake` deny path | AC3 | PARTIAL | `:117` TRUE (`j4_earned_consent_rupture_typed_oracle_deferred` `:119`); but `maos run` has no router between Mira and Nash (blocker 3) — accept path first, deny path second |
| 28 | "300-row κ corpus" separate | AC4 | TRUE | `tests/corpora/safety-critical-mira-nash-v1.5.jsonl` = 300 lines; `MANIFEST.toml:73-76` (150 Mira + 150 Nash, κ = 0.83); consumed by `crates/maos-eval/tests/safety_critical_corpus_8_5.rs` — AC4 changes nothing |
| 29 | `journey-nightly.yml` has no `services:` | 21-2 AC1 | TRUE | file `:1-127`; `services:` only at `discipline.yml:2930,2995,3077,3130`, `rpo-rto-cadence.yml:46` |
| 30 | `.github/actions/provision-loom-substrate` exists | AC1 | TRUE | `action.yml` (13.6c; composite; refuses empty list) |
| 31 | `discipline.yml:2930-2967` | AC1 | TRUE | `check-cross-region-consensus` job: services `:2930-2943`, provision `:2951-2957`, env `:2958-2966`, run `:2967-2969` |
| 32 | `-p maos-bin --test cross_team_crossing_13_6b` ignored | AC1 | TRUE | 4 ignored `:1641,:1837,:2354,:2810` ("AdvisorySubstrate: requires MAOS_TEST_POSTGRES_TEAM_A/_B[/_C]"); the 3-team journey needs `TEAM_C` (`:1331`) |
| 33 | `-p maos-a2a-tcp --test t_12_4a_digest_read` ignored | AC1 | TRUE | 6 ignored `:298,:360,:414,:471,:525,:593`; none needs Postgres (N=8 in-process mTLS mesh, `:1-2`) |
| 34 | `--run-ignored` | AC1 | PARTIAL | nextest flag with a REQUIRED value (`--run-ignored only\|all`; the nightly is nextest, `journey-nightly.yml:25-37`); bare form as written will not parse — not executed; `demo-reza` uses cargo-test `--include-ignored` |
| 35 | (implicit) the ignored journeys do not run today | AC1 | **FALSE** | blocker 2 |
| 36 | `t_12_4a:593` capture test | AC2 | TRUE | `#[ignore = "Story 12.4b fixture capture — set MAOS_CAPTURE_J3_FIXTURE…"]` `:593`, fn `:594`, `expect` `:595-596` |
| 37 | `fixtures/j3/day-30-raw.json` | AC2 | TRUE | `crates/maos-journey-test/fixtures/j3/day-30-raw.json`; consumed `journey_j3.rs:24,119,166` |
| 38 | `check_reza_production_path.rs:457` `tl-phase-b` leg | AC3 | TRUE | `:456` name `tl-phase-b-persisted-datname-vs-live-current-database`, `:457` `class: BindingClass::AdvisorySubstrate`, runs `phase_b_persisted_datname_vs_live_current_database --ignored` (test `tenant_audit_phase_a_13_5g.rs:249-253`, needs `MAOS_TEST_POSTGRES_TEAM_A`) |
| 39 | "is Blocking with the Postgres job" | AC3 | PARTIAL | already binding in CI (blocker 2); the flip only reds a laptop without Postgres |
| 40 | `journey_butler.rs:285 → 18-4` | AC4 | **FALSE** | `:285` = `jb8_posture_shift_cognition` "RED: Epic 9 — posture-shift cognition not yet wired" (`:284-289`); Epic-18 note §K #12 removes it from 18-4; owner = none |
| 41 | (implicit) count of ignored journey tests | AC4 | — | `crates/maos-journey-test/tests`: **2** (`journey_j4.rs:117`, `journey_butler.rs:285`); if "journey" includes the substrate packages: +4 (`13_6b`) +6 (`t_12_4a`) +32 (`cross_region_live`, `discipline.yml:2921-2922`) |
| 42 | `GET /v1/a2a/peer-versions` shows the rotated fingerprint | 21-3 AC1 | **FALSE** | blocker 1 |
| 43 | durable `TofuPinStore` in `maos-persistence` "as `tofu.rs:90-93` names it" | AC1 | TRUE | `crates/maos-a2a-core/src/tofu.rs:90-93` ("the persistence-backed implementation lives in `maos-persistence`"); only impl `:508`; `maos-persistence` = 8 physical / 1 tokei lines, `[dependencies]` empty |
| 44 | `TcpA2ATransport.pins` typed to the trait | AC1 | TRUE as delta | `transport.rs:106` concrete; `A2ARouterCore::try_new(…, pins.clone() as Arc<dyn TofuPinStore>)` `:387` already trait-typed; verifier needs trait-level sync methods (blocker 1) |
| 45 | sync read path for `verify_pinned_sync`, `tofu.rs:457` | AC1 | TRUE | `:454-457` |
| 46 | `swap_serving_cert` (`transport.rs:541`, in-place) | AC2 | TRUE | `:541`; doc `:538-540` "there is NO production caller"; callers = `t_14_2_rotation_scene.rs:211`, `t_10_4b_rotation_real_timing.rs:550,1252` |
| 47 | "driven by a signed self-reissue" | AC2 | PARTIAL | 14-2d AC2: port `reconcile_local_identity(&PeerCertFingerprint)` in `maos-cohort` + actuator in `crates/maos-bin/src/cert_rotation.rs` (exists) fired on `Applied`, `Confirmed` AND the boot reconcile; the trigger is the existing manifest reissue, not a new message |
| 48 | `T_grace` raised to cold+I | AC2 | TRUE | 14-2d §3b (`14-2d-self-identity-rotation.md:119-152`): `G = cold_deployment_t_grace() + confirmation_interval()` ≈ 65 s, supersedes cold + 2I |
| 49 | `cert_rotation_trigger_14_2a.rs:720-732` | AC2 | TRUE | `:720` recomputes `cold_deployment_t_grace()`, `:725` `acked_at + grace + 45 s` |
| 50 | `cert_rotation_14_2a.rs:723-742` | AC2 | TRUE | `crates/maos-cohort/tests/cert_rotation_14_2a.rs:723-742` `t_grace_matches_the_shipped_formula…` (Blocking equality leg) |
| 51 | "14-2d's design record" | AC2 | PARTIAL | 14-2d has 6 ACs + 8 tasks (`:300-360,:520-535`): port, actuator, deadline rule (NOT unanimity), plane-C setter (F3 `RwLock`, `.clone()` never a guard, `:404-410`), installed-grace accessor, `maos-control` schema delta, live mesh test, `discipline.yml` timeout. 21-3 AC2 names two. 14-2d's own status is `blocked` (`:14`) on 14-2b/14-2c, both now `done` (`sprint-status.yaml:218-219`); the tracker has NO `14-2d` row |
| 52 | `journal_peer_identity_refusal` "gains a detail field" (`router.rs:502-507`) | AC3 | PARTIAL | fn already has `detail: &str` (`crates/maos-a2a-core/src/router.rs:506`); the FRAME lacks it — `ConsentRupturePayload`/`RuptureRejection` (`crates/maos-domain/src/frame.rs:359-377`), stated by `t_14_2a_post_grace_journal.rs:241-246`; the sentinel `cert_post_grace_reject` is built at `transport.rs:1034` |
| 53 | "per ADR-063" | AC3 | NEITHER at HEAD → 15-5 | `docs/adr/` tops at ADR-059; ADR-063 = 15-5 AC1 (`epic-15…:87`, revocation model + machine-readable post-grace token); not in Epic 21's Dependencies |
| 54 | listen-side per-peer scoping (`transport.rs:1341`) | AC4 | PARTIAL | `:1341` is `.with_cert_resolver(resolver.clone())`; the flat lookup is `:1333` (`None, // listen side learns the peer from the cert — flat TOFU lookup`) in `build_server_config` `:1324`; no design for how the server knows the peer before the cert (§H) |
| 55 | one instrument (tokei code) | 21-4 AC1 | TRUE as delta | `check_kernel_baseline.rs:99-110` physical; `kloc_check.rs:163-213` tokei `-e tests -e benches -e examples -e fuzz -e spirits`. Hidden costs: `check_fkcs.rs:62-78` counts the FROZEN TAG physically via `git show` (`fkcs-baseline.toml:5 src_lines = 23081`); `check_epic_close_coherence.rs:187-197` reds every non-done epic file citing the old pin; HISTORY deltas are physical |
| 56 | baseline TOML invalid at line 29 | AC1 | TRUE | `tomllib`: "Expected '=' after a key … (at line 29, column 10)"; `read_pinned` documents it and line-parses on purpose (`check_kernel_baseline.rs:73-80`) |
| 57 | ≤25K "kernel-crate-set" ceiling + ADR-057 amendment, "or retired" | AC1 | PARTIAL | ADR-057 is "enterprise governance is a daemon posture"; the 25K figure traces to `sprint-change-proposal-2026-07-13.md:35` and `ADR-040:26`'s J1 budget "≤25,000" — **microseconds**; `xtask/kernel-crates.toml` exists (`crates = ["maos-kernel-core"]`, check-loom scan list). "or" = open fork (rule 7) |
| 58 | `check_env_contract.rs:119` scans maos-bin only | AC2 | TRUE | `:119` `join("src")`; registry path `:89`; `main.rs:1304` passes `maos_bin_dir`; detector shapes `:41-59` (`env::var`, `var_os`, `duration_ms_from_env`, `any_env_with_prefix`) |
| 59 | registry → `maos-domain` "with its 15-2 allowance" | AC2 | **FALSE** | blocker 3 (479 code lines + ~275 new entries vs +300 already drawn to −51/+36–66/+50–100) |
| 60 | "16 names in kernel-core" | AC2 | PARTIAL | 16 read, 15 unregistered (`MAOS_AUTO_REVERT_FAST, MAOS_IDLE_FAST, MAOS_REGION_HOME, MAOS_REGISTRY_ALLOW_FORCE_TIER_AT_IMPORT, …_ALLOW_UNSIGNED_LOCAL, …_ORG_SIGNING_PUBKEY, …_REQUIRE_SERVER_TIER_SIGNATURE, …_T3_FOR_PUBLIC_UNTRUSTED, …_TIER_FLOOR, MAOS_REVOCATION_FAST, MAOS_SCHEDULE_FAST, MAOS_SUPERVISION_FAST, MAOS_T3_IMAGE_LOCK_PATH, MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX, MAOS_T3_RUNTIME`) |
| 61 | `EnvStability::Secret` | AC2 | TRUE as delta | enum = `HarnessOnly \| UserFacing` (`env_contract.rs:8-11`) |
| 62 | `maos-domain/region.rs:84-324` | AC3 | PARTIAL | read at `:97` (`Region::resolve_home`), doc `:84`; `:293-324` are tests |
| 63 | `operator_config.rs:177-349` | AC3 | PARTIAL | `crates/maos-kernel-core/src/security/operator_config.rs` (KERNEL file): doc `:177`, read `:203`; `:285-349` tests. Two independent readers of the same var (maos-domain + kernel) — the AC must say which one the boot check reconciles |
| 64 | `TeamEntry.region` | AC3 | TRUE | `crates/maos-cohort/src/manifest.rs:154-159` |
| 65 | `maos-persistence` retained | AC4 | TRUE | `Cargo.toml:26`; `kloc.toml:326 = 2000` |
| 66 | three phantom `kloc.toml` budgets | AC4 | TRUE | `maos-cap-registry` (`:197`), `maos-wire` (`:199`), `maos-journal` (`:200`) — 0 LOC, no crate directory |
| 67 | "19 canonical / 10 SigningKey" dropped (17 fns, 6 keys) | AC4 | not re-derived | prior preflight §1 row 20; a dropped claim is not a deliverable |
| 68 | Closes D1 | :7, 21-4 | **FALSE** | proposal `:86` re-points **D1 → 20-3**; D1 is the out-of-ledger evidence contract (`epic-14-preflight-decisions.md:259-280`) |
| 69 | Closes D4a/b/c | 21-4 | PARTIAL | register `:282-300`: D4a = RUNTIME boot check (AC3 ✓), D4b = registration (AC2 ✓), D4c = classification + "is a slot for non-secret key-derivation input added" (AC2's `Secret` alone does not answer it) |
| 70 | Closes D11 | 21-4 | PARTIAL | D11-E1/E2/E3 (`:429-472`: `example_spirit_regen` budget with no execution path; clippy never invoked; in-`src` tests budget-charged) — no 21-4 AC names any |
| 71 | Closes D13 | 21-4 | TRUE | `:79` "(b) the instrument question → 21-4" |
| 72 | Closes D14 | 21-4 | PARTIAL | `:80` names 21-4 "with an explicit AC expansion" for the +50; but 15-1 AC4 already absorbs it ("`maos-domain` +51 … covered by 15-2", `epic-15…:49`) — D14 should re-point to 15-1/15-2 |
| 73 | `orchestrator/` 5 files, 1028 LOC | 21-5 AC1 | TRUE | buffer 220, echo_gateway 90, gateway_dispatcher 531, mod 97, registry 90 (physical); tokei code 817 |
| 74 | `scheduler_loop.rs:89,111` | AC1 | TRUE | field `:89`, ctor param `:111` (`Option<Arc<crate::orchestrator::OrchestratorBufferRegistry>>`) |
| 75 | `main.rs:1995,5696,5788,5859` | AC1 | TRUE | registry ctor; `journal_orchestrator_queue`; `journal_director_lifecycle_action`; `journal_token_revocation` |
| 76 | `check-cohort-mesh` re-runs the pin `:604` | AC1 | PARTIAL | `xtask/src/check_cohort_mesh.rs:674` (`check_kernel_baseline::check()?.passed`); `:604` is a `migration_chain` leg |
| 77 | "behind a `maos-domain` port" | AC1 | PARTIAL | only `OrchestratorBufferRegistry` is used by kernel code outside the module (`scheduler_loop.rs:89,111`); `GatewayDispatcher` + 3 `journal_*` fns are used by `maos-bin` and by kernel TESTS (`gateway_dispatcher_fr54.rs`, `gateway_uninstall_fr65_v05.rs`, `nfr_perf_4_pause_resume_latency.rs`) + `spirits/orchestrator/tests`. The 817 code lines have no home with headroom (§E) |
| 78 | "§4.0.7 naming" | :35, 21-5 | PARTIAL | §4.0.7 = "What the Kernel Does NOT Compute" (`4-kernel-design.md:180-190`: "does NOT embed an orchestration policy"); `xtask/loom-blocklist.toml:14` lists `Orchestrator` for check-loom over `kernel-crates.toml`, and check-loom is not red at HEAD → the module is not a measured violation; "naming" undefined |
| 79 | Dependencies: 20-3, 20-2 | :83 | PARTIAL | no 21-x AC mentions runbooks/packages (20-2); 20-3's evidence states are consumed by no 21-2 AC (`demo-reza` already reads the 13.6e ledger, `demo_reza.rs:60-64`) |
| 80 | "17-3b before 21-2" | :83 | **FALSE** | 21-2 touches no WASM; the DAG edge (`dependency-dag.md:217`, "J3 recall over the WASM wire") is the old E17→E20 mapping. Missing real edges: 15-2, 15-3, 15-5 (ADR-063), 15-6 (`MAOS_INFERENCE_MODE`), 18-4 (clock) |

## B. Verb-first

Exit block: `cargo run -p xtask -- demo-reza` — EXISTS (`xtask/src/main.rs:1012-1019`, clap; `check-exit-commands` can resolve it via `xtask --help`). Line 2 is a comment, not a command — NEITHER; no gate implements it (A#11).

| Token | HEAD | Created by | Verdict |
|---|---|---|---|
| `MAOS_INFERENCE_MODE=replay` | no (`env_contract.rs`: only `MAOS_REPLAY_CASSETTE` `:255`) | 15-6 AC1/AC4 | OK if 15-6 lands first (edge missing) |
| `maos run <manifest>` | yes, hand-parsed (`worker_spawn.rs:44-82`; no clap, `main.rs:1653`) | — | `check-exit-commands` cannot resolve `maos` tokens (Epic-15 blocker 1; `maos --help` boots the daemon) |
| `--incidents <path>` | no | nobody | NEITHER — must be an AC1 deliverable |
| `--once` | yes (`:55`) | — | OK; semantics = one `on_idle` pass (`worker_spawn.rs:37-38`), which cannot "resolve 50 incidents under a compressed clock" unless the scene loop lives in `maos-bin` (Epic-18 note §H-8) |
| `spirits/topologies/j4-mira-nash.toml` | yes | — | OK |
| `tests/fixtures/j4/incidents-50.jsonl` | no | nobody | NEITHER |
| `journey-nightly.yml` + `provision-loom-substrate` | yes / yes | — | OK; the action needs a `services:` block in the SAME job (`action.yml:6-11`: composite actions cannot define services) |
| `cargo nextest run … --run-ignored` | flag exists with a value | — | PARTIAL (bare form) |
| `cargo test -p maos-bin --test cross_team_crossing_13_6b` | yes | — | OK; the daemons it spawns need `target/debug/maos` (`cargo build --all-targets`, `journey-nightly.yml:30`) |
| `GET /v1/a2a/peer-versions` | no (`/v1/cohort/peer-versions`) | nobody | FALSE |
| `check-kernel-baseline`, `kloc-check`, `check-env-contract`, `check-cohort-mesh`, `check-reza-production-path` | all exist (`gate-registry.toml`) | — | OK |
| `xtask/kernel-crate-set.toml` | no (`kernel-crates.toml` exists for check-loom) | 21-4 or retired | fork |

## C. Foundation audit

**21-1.** REAL: `Mira::diagnose` + `AnomalySignal` (`lib.rs:87,274`); `Mira::with_pending_signals` (`:241`) as the only signal inlet; the J4 topology file; the daemon's Mira scalar-port wiring (`main.rs:3797-3836`); the 300-row corpus; `journey_j4.rs:117`'s deferred oracle with production types. ABSENT: `--incidents`; the fixture; a Mira/Nash inference port (no `[capabilities.required]`; `ButlerOrchestratorAdapter` wires only `EpistemicScalarPort`); recorded turns; a clock (18-4); an A2A router in the `maos run` topology path (blocker 3) — the only Mira→Nash advisory paths are `smoke-a2a-tcp-8-6` (`main.rs:10520-10689`) and the loopback smokes (`:10786-11303`) and `spirits/mira/tests/{halt_bilateral,a2a_pairing}.rs`; a "close" predicate. Assumed foundation count: 4 (flag, fixture, clock, router) + 1 operator act (recording).

**21-2.** REAL: everything cited. INVERTED: the journeys already run (blocker 2). ABSENT: an ignored-owner gate; a mechanism by which `journey_j3` (`maos-journey-test`) consumes output of a `maos-a2a-tcp` test run in the same job (path hand-off + ordering across crates); AC2 deletes the fixture-by-default design of 12.4b (the committed fixture is the point: "the only producer of the committed J3 raw-input fixture", `t_12_4a…:586-591`).

**21-3.** REAL: trait, in-memory impl, `verify_pinned_sync`, `swap_serving_cert`, `cert_rotation.rs`, `install_cert_rotation` (`main.rs:9824`, drifted from 14-2d's `:9796-9801` at baseline `53845402`), both 14-2a legs, `journal_peer_identity_refusal`, `RuptureReason::PeerIdentityUnverified` (`frame.rs:396-398`), `/v1/cohort/{peer-versions,self-identity}`, `/v1/a2a/rotation-windows`. ABSENT: a `TofuPinStore` impl in `maos-persistence` (crate has no deps — needs `maos-a2a-core` for the trait; direction is acyclic: `maos-a2a-core` → `maos-domain`, `maos-spirit-abi` only); sync trait methods; a pin read surface; the 14-2d port/actuator/setter/accessor set; a frame-level token; a listen-side identity claim.

**21-4.** REAL: both instruments, `read_pinned`, `check_fkcs`, `check_epic_close_coherence`, `check_env_contract` + `detect_env_reads` (syn-based, per-crate proven-red from 12.6), `Region::resolve_home`, `RegionSection::load`, `TeamEntry.region`, the three phantom rows. ABSENT: `EnvStability::Secret`; a workspace root for the scan (one-line change at `:119` + the per-crate proven-red 12.6 designed, `sprint-change-proposal-2026-07-13.md:61`); a boot reconciliation site in `maos-bin` (where the cohort manifest is loaded: `main.rs` cohort bootstrap around `:9765-9824`); a kernel-crate-set file or its retirement.

**21-5.** REAL: module, users. ABSENT: a `maos-domain` port trait for `OrchestratorBufferRegistry`; a home for 817 code lines + 3 kernel test files + `spirits/orchestrator/tests` fixtures that import `maos_kernel_core::orchestrator`.

## D. Kernel-Δ audit

| Story | Zero possible? | Notes |
|---|---|---|
| 21-1 | Yes | all in `spirits/`, `maos-bin`, `maos-journey-test`, fixtures; the "clock" must NOT compress `idle_watchdog.rs` (kernel; Epic-18 note D) |
| 21-2 | Yes | YAML + one `BindingClass` word + tests |
| 21-3 | Yes | `maos-persistence`, `maos-a2a-core`, `maos-a2a-tcp`, `maos-cohort`, `maos-bin`, `maos-control`, `maos-domain` (token) — none in `crates/maos-kernel-core/src` (`check_kernel_baseline.rs:31`) |
| 21-4 | Zero LINES, non-zero PIN | AC1 rewrites `src_lines` to the tokei value (18933 today, or whatever the chosen filter yields) and must update every non-done epic file's `@24472` (`check_epic_close_coherence.rs:187-197`) plus `fkcs-baseline.toml` semantics; AC3's `operator_config.rs` read stays untouched if reconciliation lives in `maos-bin` |
| 21-5 | Negative | −1028 physical (−817 tokei) +2–6 (port type at `scheduler_loop.rs:89,111`); if 21-4 lands first the re-pin is in tokei units — the epic must fix the ORDER (21-4 → 21-5) or the unit |

Gate assumptions on monotonic growth: none found — `check_kernel_baseline` is equality; `check_fkcs::validate_frozen_tag_src_lines` counts the frozen tag independently (`:62-78`); `validate_live_triple` delegates to the equality gate (`:96-108`); `t11_t12_chaos_absence` reads the same toml. The cost of a negative re-pin is documentary (7 epic files + HISTORY row), not mechanical.

## E. Kloc headroom (`kloc-check` @9f920180, RED: aggregate 155498 vs `_aggregate_hardfail` 147057; `maos-domain` OVER)

| Crate | measured / ceiling / headroom | 15-2 grant (`epic-15…:57`) | already drawn by 15–20 (their §E) | Epic 21 need | fits? |
|---|---|---|---|---|---|
| maos-a2a-core | 4856 / 4856 / **0** | +150 | 16-5 +20–40 | 21-3 sync trait methods +20–40; durable-store hooks +10–20 | fits after 15-2 |
| maos-a2a-tcp | 1439 / 1500 / 61 | +150 | — | 21-3 `Arc<dyn TofuPinStore>` re-typing across 8 sites (`transport.rs:106,375,810,977,1183,1325`) +20–60; listen-side scoping +30–80 | fits after 15-2 |
| maos-cohort | 5857 / 5857 / **0** | +100 | — | 14-2d port + deadline rule + accessor +150–300 (14-2d T8 asks for its own grant) | **exceeds +100** |
| maos-bin | 17109 / 17109 / **0** | +1200 | 15-6 +150–300; 16 +480–820; 18 +300–680; 19 +390–820 (over-subscribed, Epic-19/20 notes) | 21-1 flag/adapter/router/seams +250–500; 21-3 actuator +100–200; 21-4 boot check +30–60 | **no room unless the registry LEAVES maos-bin (−479) and 15-2 is re-cut** |
| maos-domain | 8695 / 8644 / **−51** | +300 | 15-1 +51; 16 +36–66; 18 +50–100 | 21-4 AC2 +750 (blocker 3); 21-3 token +3–10; 21-5 port +30–60 (+817 if the code lands here) | **21-4 AC2 impossible as written** |
| maos-persistence | 1 / 2000 / 1999 | — | — | 21-3 impl +300–600 | fits |
| maos-control | 803 / 1500 / 697 | +700 | 16-1 +400–600; 19-2 +40–80 | 21-3 pin read surface +40–80 | fits |
| maos-journey-test | 493 / 593 / 100 | none (16/18 notes ask +400) | 16-2, 18-1 | 21-2 journey_j3 consumer 0–50 (tests are `-e tests`) | fits |
| maos-eval | 3661 / 3696 / 35 | +600 | 18 +450–900 | 0 (κ leg unchanged) | — |
| xtask | 41953 / 41953 / **0** | net ≤0 | 19 +60–150; 20 +1300–2500 | 21-2 owner gate +150–300; 21-4 scanner root + per-crate proven-red +100–200, instrument unification +50–150 | **conflicts with "net ≤0"** |
| spirits/* | excluded (`kloc_check.rs:185-186` `-e spirits`) | — | — | 21-1 Mira/Nash seams free | OK |
| maos-kernel-core | 18933 / 18933 / 0 | +150 (FLAG-Winston only) | 16-5, 17-1, 17-2 | 21-5 −817 → headroom 817 that ADR-038 forbids spending | re-base down or it becomes free growth |

## F. Hermetic vs operator

| Item | Where | Verdict |
|---|---|---|
| Recording the 50 incidents' model turns | 21-1 AC1 ("recorded model turns") | **paid API + operator act with no `ops-*` row**; the epic's operator-lane line (:17) must become a tracker row (e.g. `ops-j4-live-incident-recording`) that PRODUCES the cassette 21-1 replays |
| "≥300 calls per run" | :17 | operator lane — OK once filed |
| Postgres service containers | 21-2, exit l.1 | hermetic (no secrets; `services:`) — but `demo-reza` needs `psql` on the runner (`demo_reza.rs:139`) |
| `T_grace` ≈ 65 s + 45 s slop | 21-3 AC2 | wall-clock inside a Blocking leg: 14-2d measured ~125 s → ~65 s and asks `discipline.yml` timeout raise (`14-2d…:438-442`); acceptable, but the AC must state the CI budget |
| Nightly `tier-2-rerecord` `--live` (invalid nextest flag, `journey-nightly.yml:94`) | 21-2 touches this file | owned by 15-6 AC3; 21-2 must not re-introduce it |
| Human-signed / two-host paid runs | — | none in Epic 21 — correct |

## G. Sizing from measurement (`git show --stat`; story-file first→last touch)

| Analogue | Commit | LOC | Calendar |
|---|---|---|---|
| 14-2a production rotation trigger | `ee3422d0` | +9391/−99, 37 files | 2026-08-30 → 08-31 dev, review to 09-04 (~1 wk) |
| 14-2b convergence observability | `4657cace` | +2045/−18 | 09-02 → 09-03 (2 d) |
| 14-1 100-host churn | `0cdcc6c0` | +8633/−1047 | 08-27 → 08-29 (3 d) |
| 14-0 preflight decisions (instrument/register work) | `38c52811` | +2460/−192 (19 xtask files) | 08-26 → 08-31 (5 d) |
| 13-6c 3-team substrate | `c571a2b9` + `4a952f81` | +1397/−84 + 260 | 07-29 → 08-04 (6 d) |
| 13-6e judge machinery | `c45df0be` | +5465/−1347 | 08-07 → 08-11 (4 d) |
| 12-6 env-contract registry | `5767cf0d` | +629/−58 | 07-13 (1 d) |
| 12-4a digest-read consent | `ab2cc512` | +2390/−120 | 07-12 (1–2 d) |
| 10-4b Mira/Nash 2-host proof | `3f65b6a8` | +2196/−26 | 06-24 → 06-27 (3 d) |

Story estimates (+30 %):

- **21-1** ≈ 10-4b-class scene + 18-1-class seam ×2 + 18-4-class clock consumer + a new in-run router + fixture + recording round: 1.5–2.5 wk → **2.0–3.3 wk**
- **21-2** ≈ 13-6c-class YAML + 12-6-class gate (owner check) + AC2 redesign: 0.6–1.2 wk → **0.8–1.6 wk** (0.3–0.6 if AC1/AC3 are re-cut to what is not already true)
- **21-3** ≈ 14-2d (the file's own T1–T8 ≈ 14-2a/14-2b tempo, 1–1.5 wk) + durable store + trait/typing (0.7–1 wk) + token + scoping design (0.5–1 wk): 2.2–3.5 wk → **2.9–4.6 wk**
- **21-4** ≈ 14-0-class instrument work (2–3 d) + 12-6-class scan widening (2–3 d) + registration/relocation (1–2 d) + boot check (1 d) + fork closures (1 d): 1.6–2.4 wk → **2.1–3.1 wk**
- **21-5** (optional) ≈ ADR-041-style port extraction + 3 test files + kloc home: 1–1.5 wk → **1.3–2.0 wk**

Serial 21-1..21-4: **7.8–12.6 wk**; with 21-5 **9.1–14.6 wk**. Parallel (21-2 ∥ 21-4 ∥ 21-3; 21-1 after 18-4): critical path 21-3 → 21-1 ≈ **5.5–8.5 wk**. Epic's 5–7 is the parallel lower bound with 21-5 excluded.

## H. Fork audit (rule 7)

1. **21-1 clock** — Butler-private / `maos-domain` port / `maos-spirit-sdk` (Epic-18 §H-6). Recommend `maos-spirit-sdk` (839/3000), built by 18-4, consumed by 21-1; add edge 18-4 → 21-1.
2. **21-1 replay selector** — `MAOS_INFERENCE_MODE` (15-6) vs `--replay <path>` (18-3) vs `MAOS_REPLAY_CASSETTE` (Epic-18 §H-1). Recommend the 15-6 pair (`MAOS_INFERENCE_MODE=replay` + `MAOS_REPLAY_CASSETTE=<cassette>`); AC1 must name the cassette.
3. **21-1 "close" predicate** — undefined. Recommend: closed ⇔ Nash emits a `proposed_fix` whose `subject` equals the incident's `subject` within N simulated minutes ≤ 90 (`prd/project-scoping…:226`); keep the 48/50 consent-envelope count as AC3's measured output.
4. **21-1 router in the run path** — `LoopbackA2ARouter` inside one daemon vs two daemons over TCP (`bilateral-2-host-mira-nash.toml` header says the pairing rides `A2APeerConfig`, not the manifest). Recommend loopback in-process for the hermetic AC (the J3 daemons already prove TCP); deny path = Nash `accept_allowlist` excluding one intent (the `disallow_router` smoke at `main.rs:10913` is the template).
5. **21-2 AC2 fixture** — regenerate-and-diff (nightly runs the capture with `MAOS_CAPTURE_J3_FIXTURE`, then `diff` against the committed file → drift red) vs live consumption. Recommend regenerate-and-diff; keep the capture `#[ignore]` (it writes a file) and keep 12.4b's committed fixture as the hermetic input.
6. **21-2 AC4 owner gate** — new xtask gate (xtask net ≤0) vs a `cargo test` in `maos-journey-test` that greps its own `tests/` for `#[ignore = "…"]` without an `owner:` token. Recommend the in-crate test (kloc-free under `-e tests`).
7. **21-3 store format** — JSON file under `XDG_DATA_HOME` / SQLite (the audit DB already uses rusqlite in `maos-audit`) / TL frames. Recommend a JSON pin file with the boot-nonce invalidation rule (NFR-Rel-6) stated; decide before backlog exit.
8. **21-3 sync path on the trait** — add `verify_pinned_sync` + `find_active_pin_by_fingerprint` to `TofuPinStore` (breaks `maos-a2a`'s `InMemoryTofuPinStore` only additively) vs a `SyncPinLookup` sub-trait. Recommend the sub-trait (no change to the async surface the 8.6 comment protects).
9. **21-3 token shape** — `RuptureReason::CertPostGraceReject` variant (`#[non_exhaustive]`, +3 `maos-domain` lines) vs a `detail` field on `RuptureRejection` (schema change on every kind-22 row; `consent_rupture_adr_034.rs`, `frame_bridge.rs` consumers). Recommend the variant; ADR-063 (15-5) must say so.
10. **21-3 listen-side scoping** — identity claim in the client cert (SAN/CN = `PeerId`) vs post-handshake binding frame. Recommend deciding in ADR-063 or cutting AC4 to a `blocked` follow-up; it is a design, not a story AC.
11. **21-4 instrument** — tokei for both (pin depends on tokei's version; `check_fkcs` frozen-tag count must move to a worktree checkout) vs physical for both (`kloc.toml` formula is tokei) vs keep two with the D13 disclosure. Recommend tokei for both with the tokei version pinned in CI and `check_fkcs` re-derived; state the new pin number in the AC.
12. **21-4 kernel-crate-set** — author vs retire. Recommend RETIRE: the 25K figure is ADR-040's J1 latency budget in μs; `kernel-crates.toml` already names the TCB crate set for check-loom.
13. **21-4 registry home** — `maos-domain` (+750, impossible) vs stay in `maos-bin` (+275 for new rows) vs a kloc-free data file (`xtask/env-contract.toml` read by the gate and by a 30-line loader). Recommend the data file; move only `EnvVar`/`EnvStability` (+15) if a shared type is wanted.
14. **21-4 `MAOS_REGION_HOME` reader** — reconcile the maos-domain `resolve_home` result or the kernel `RegionSection::load` result against `TeamEntry.region`. Recommend the maos-domain call from `maos-bin` at cohort-manifest load; mismatch = typed boot refusal.
15. **21-5 home for 817 lines** — `maos-domain` (no allowance) / `maos-bin` (none) / new leaf crate `maos-orchestrator-buffer` (+`check-workspace-count`, +kloc row). Recommend the leaf crate; price it before the story leaves backlog; order 21-4 → 21-5.

## I. Dependency audit

- **17-3b → 21-2: FALSE** (A#80). Remove.
- **20-2 → 21-\*: unjustified** — no runbook/package AC in Epic 21. Remove or add the AC that needs it.
- **20-3 → 21-2: soft** — the nightly can red on absence without 20-3's `EvidenceState`; keep only if 21-2 publishes a ledger.
- **Missing:** 15-2 (every 21-3/21-4 crate is at zero), 15-3 (`check-exit-commands` must resolve `maos` tokens — Epic-15 blocker 1 unresolved), 15-5 (ADR-063 for 21-3 AC3; `dependency-dag.md:213` has it, the epic file does not), 15-6 (21-1 AC1 env var; DAG `:214` has it), 18-4 (clock; Epic-18 §K #14 names 21-1 as consumer).
- **Intra-epic:** 21-4 before 21-5 (unit of the re-pin); 21-3 independent; 21-2 independent; 21-1 last. 21-2 and 21-4 can run in parallel with Epic 20 today (they need only 15-2/15-3).
- **14-2d:** its file is `blocked` on 14-2b/14-2c which are `done` (`sprint-status.yaml:218-219`); the tracker has no 14-2d row; 21-3 absorbs it — say so in the story and retire the file to `.archive/` or re-point its status.

## J. Did R2 fix the prior preflight (old E20 → new E21)?

| Prior finding (`epic-15-20-preflight-2026-09-04.md`) | R2 status | Evidence |
|---|---|---|
| `demo-reza --live` will not parse (§1 row 20) | **fixed** | exit is bare `demo-reza` |
| red-on-absence "already true" | **fixed** (now stated) | :12 |
| "50-scenario corpus" is the 300-line κ corpus with no reader/driver | **renamed, still present** | 21-1 AC1 now names a 50-incident fixture that does not exist, with no AC creating it; the driver into Mira (`--incidents` → `with_pending_signals`) is still unbuilt |
| "19 canonical / 10 SigningKey" wrong | **fixed** (dropped, 21-4 AC4) | — |
| moving `orchestrator/` is a kernel edit | **fixed** (FLAG-Winston, negative) | :19, 21-5 |
| 20-3/20-4 `maos-persistence` contradiction (§2.8) | **fixed** | 21-3 owns it; 21-4 AC4 retains it |
| 14-2d's stated blocker stale; real blocker = `T_grace` raise (§2.8) | **fixed** | 21-3 AC2 carries cold+I and both legs |
| `journey-nightly.yml:94` invalid `--live` (§2.8) | moved to 15-6 AC3 | not 21-2's — but 21-2 edits the same file |
| `journey_butler.rs:285` blocks "zero ignored journey tests" (§3) | **still present, wrong owner** | A#40 |
| `check-cohort-mesh` re-runs the pin on any orchestrator move (§3) | **fixed** (cited, wrong line) | A#76 |
| one 14-2a leg reds on the `T_grace` raise (§3) | **fixed** (both legs re-derived) | A#49-50 |
| §2.5 exit commands must name what a machine can run | **partially** | line 1 yes; line 2 prose; 21-1 AC1 has three absent tokens; 21-3 AC1 an absent route |

## K. Verdict, required changes, confirmed-true

**Confidence 42.** The substrate this epic stands on is unusually real — the one-command Reza scene, the composite provisioning action, four Postgres-backed gate jobs, the in-place cert swap, the pin trait with the persistence crate already named in its doc, the rupture journal, the 300-row corpus, the topology file and the orchestrator module all exist and were read, not inferred. What keeps it under 55: one AC observes a route that does not exist (21-3 AC1), one story's premise is inverted (the ignored journeys already run as Blocking/AdvisorySubstrate legs — 21-2 as written adds a second runner and a one-word class flip that CI already enforces), one AC is arithmetically impossible under the allowance it cites (21-4 AC2), 21-1 names four mechanisms nobody builds and a paid recording nobody files, three citations point at the wrong seam (`router.rs` detail, `transport.rs:1341`, `check_cohort_mesh.rs:604`), the Dependencies line carries one false edge and misses four, and fifteen forks are still inside ACs. All are edit-level; none needs a new epic. **Verdict: needs-rework. Measured duration 7.8–12.6 wk serial / 5.5–8.5 wk parallel.**

### REQUIRED CHANGES (OLD → NEW)

1. **Exit block l.2** — OLD "`# nightly: zero #[ignore] journey tests without a named owner`" → NEW "`cargo test -p maos-journey-test --test ignored_owner_audit` (NEW in 21-2 AC4: every `#[ignore = "…"]` under `crates/maos-journey-test/tests` carries `owner:<story-key>`; today 2 ignored: `journey_j4.rs:117` → 21-1 AC3, `journey_butler.rs:285` → Epic-9 residual, owner TBD)".
2. **Operator lane** — OLD "live J4 with a provider key (≥300 calls per run)" → NEW "`ops-j4-live-incident-recording` (tracker row): record the 50-incident cassette set with a provider key (≥300 calls); PRODUCES `crates/maos-journey-test/cassettes/j4/*.json` that 21-1 replays. 21-1 is blocked until the row is `done`".
3. **21-1 AC1** — OLD "`MAOS_INFERENCE_MODE=replay maos run spirits/topologies/j4-mira-nash.toml --incidents tests/fixtures/j4/incidents-50.jsonl --once` resolves 50 incidents … under a compressed scenario clock" → NEW "`MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j4 maos run spirits/topologies/j4-mira-nash.toml --incidents crates/maos-journey-test/fixtures/j4/incidents-50.jsonl --clock compressed:90m --once` — `--incidents` and the 50-row fixture are NEW (`worker_spawn.rs:53-66`; rows → `Mira::with_pending_signals`, `lib.rs:241`); `--clock` is 18-4's `maos-spirit-sdk` clock (edge 18-4 → 21-1); the scene loop lives in `maos-bin`, never in `idle_watchdog.rs`; prints `closed=N/50 consent_upheld=M/50` where closed ⇔ Nash's `proposed_fix.subject == incident.subject` within 90 simulated minutes; asserts N ≥ 45 and M ≥ 48 (`prd/project-scoping…:226`); a fixture whose Nash cassette never proposes is the proven-red".
4. **21-1 AC2** — append: "manifests gain `[capabilities.required] provider.complete` (schema bump costed, `manifest_field_coverage`); the daemon's `LoadedSpiritKind::Mira|Nash` arms (`main.rs:3797-3847`) wire the deferred inference port as 18-1 does for Butler; provenance: `spirits/mira/manifest.toml:67-68` updated, `spirits/nash/manifest.toml` gains a `[model_provenance]` section".
5. **21-1 AC3** — OLD "The J4 topology triggers the `handle_intake` deny path so `journey_j4.rs:117` is un-ignored" → NEW "The `maos run` topology path routes Mira's advisory to Nash through an in-process `LoopbackA2ARouter` (NEW; template `main.rs:10969-11040`; today `main.rs:3797-3847` loads both with no router); a second topology `j4-mira-nash-deny.toml` whose Nash `accept_allowlist` excludes the advisory intent produces the kind-22 `RuptureReason::IntentAllowlistMismatch` row; `journey_j4.rs:117` un-ignored against it".
6. **21-1 AC4** — delete (no change to the tree) or rewrite as "`safety_critical_corpus_8_5` stays green; the incident fixture cites `MANIFEST.toml` provenance for its 50 subjects".
7. **21-2 AC1** — OLD "…runs `-p maos-bin --test cross_team_crossing_13_6b` and `-p maos-a2a-tcp --test t_12_4a_digest_read` with `--run-ignored`; it reds on absence" → NEW "`journey-nightly.yml` gains a `services:` block byte-identical to `discipline.yml:2930-2943` (drift control `check-loom-substrate-drift`), `provision-loom-substrate` with `maos_team_b maos_team_c maos_shared`, and runs `cargo run -p xtask -- demo-reza --skip-gates` as the nightly scene (the four `13_6b` legs and five `t_12_4a` legs ALREADY run per commit in `check-multi-tenant-loom`/`check-loom-substrate-drift`/`check-cohort-mesh`; the nightly adds the narrated end-to-end run, not coverage); proven-red: delete the `services:` block → `demo-reza` exits non-zero".
8. **21-2 AC2** — OLD "The J3 capture test (`t_12_4a:593`) is un-ignored and `journey_j3` consumes the captured mesh output, not `fixtures/j3/day-30-raw.json`" → NEW "The nightly runs `t_12_4b_capture_j3_raw_inputs` with `MAOS_CAPTURE_J3_FIXTURE=$RUNNER_TEMP/day-30-raw.json` (stays `#[ignore]`: it writes a file) and diffs it against the committed `crates/maos-journey-test/fixtures/j3/day-30-raw.json`; drift is a nightly red with the diff attached; `journey_j3` keeps consuming the committed fixture (12.4b's design)".
9. **21-2 AC3** — OLD "…`tl-phase-b` leg is Blocking with the Postgres job" → NEW "delete (already binding: `AdvisorySubstrate` hard-fails when `MAOS_TEST_POSTGRES_TEAM_A/B` are present, `gate_common.rs:216-233`, and the job exports them, `discipline.yml:3182-3184`)" — or, if local-dev strictness is wanted, say so and cite the laptop cost.
10. **21-2 AC4** — OLD "Zero `#[ignore]` journey tests without a named owner (`journey_butler.rs:285` → 18-4)" → NEW per change 1; `journey_butler.rs:285` is `jb8_posture_shift_cognition` (Epic 9 posture-shift, NOT 18-4) — name its owner or record it as an Epic-9 residual.
11. **21-3 AC1** — OLD "`GET /v1/a2a/peer-versions` shows the rotated fingerprint after restart (durable `TofuPinStore` in `maos-persistence` … `TcpA2ATransport.pins` typed to the trait; a sync read path for `verify_pinned_sync`, `tofu.rs:457`)" → NEW "NEW `GET /v1/a2a/pins` (`maos-control`, same bearer gate as `:299`) lists `{peer, fingerprint, generation, invalidated}` from the pin store; after `SIGTERM` + restart the row for the rotated peer shows the promoted fingerprint (durable `FilePinStore: TofuPinStore` in `maos-persistence`, JSON under `XDG_DATA_HOME`, NEW dep `maos-a2a-core`; boot-nonce invalidation preserved — `restart_invalidates_pin_nfr_rel_6.rs` stays green); NEW `SyncPinLookup` sub-trait carrying `verify_pinned_sync` + `find_active_pin_by_fingerprint` (`tofu.rs:250,457`) so `TcpA2ATransport.pins` (`transport.rs:106`) and `build_{server,client}_config` (`:1324,:1343`) take `Arc<dyn …>`; proven-red: delete the pin file → 401 on the next dial".
12. **21-3 AC2** — append after "14-2d's design record": "— i.e. all six 14-2d ACs: the `reconcile_local_identity` port in `maos-cohort` (5th `install_cert_rotation` parameter, `main.rs:9824`), the actuator in `crates/maos-bin/src/cert_rotation.rs` firing on `Applied`/`Confirmed`/boot-reconcile, the deadline rule, the plane-C setter (F3, `std::sync::RwLock`, `.clone()`), the installed-grace accessor consumed by `cert_rotation_trigger_14_2a.rs:720-732`, the `discipline.yml` timeout raise; 14-2d's file moves to `.archive/` and its tracker row is 21-3".
13. **21-3 AC3** — OLD "`journal_peer_identity_refusal` gains a detail field (`router.rs:502-507`) per ADR-063" → NEW "NEW `RuptureReason::CertPostGraceReject` (`crates/maos-domain/src/frame.rs:384`, `#[non_exhaustive]`, +3 lines under 15-2's maos-domain row) emitted by `transport.rs:1046-1049` on the promoted-generation mismatch path (the fn already takes `detail: &str`, `router.rs:506`; the FRAME had no token, `t_14_2a_post_grace_journal.rs:241-246`); `t_14_2a_post_grace_journal.rs` asserts the variant on the kind-22 row; ADR-063 (15-5, edge 15-5 → 21-3) names the variant".
14. **21-3 AC4** — OLD "Listen-side per-peer scoping (`transport.rs:1341`)" → NEW "Listen-side scoping (`build_server_config`, `transport.rs:1324-1334`, `None` = flat lookup at `:1333`) is DESIGNED in ADR-063 (identity claim in the client leaf's SAN = `PeerId`, or post-handshake binding); built here only if ADR-063 chooses SAN; otherwise recorded as RELEASE-HOLDS row 12's remaining half".
15. **21-4 AC1** — OLD "…read one instrument (tokei code); `kernel-core-baseline.toml` parses as TOML…; the ≤25K "kernel-crate-set" ceiling is authored as `xtask/kernel-crate-set.toml` + ADR-057 amendment, or its citation retired" → NEW "both gates read tokei code with tokei's version pinned in CI; `src_lines` re-pinned to the tokei value (state the number; ~18933 under `kloc_check.rs:167-186`'s filters) and every non-done epic file's `@24472` updated (`check_epic_close_coherence.rs:187-197`); `check_fkcs::validate_frozen_tag_src_lines` (`:62-78`) re-derived over a `git worktree` of `fkcs-baseline.toml`'s frozen tag; the HISTORY block converted to `#`-prefixed lines so `tomllib` parses; the '≤25K kernel-crate-set' citation is RETIRED (it is ADR-040:26's J1 latency budget in μs; `xtask/kernel-crates.toml` already names the TCB set)".
16. **21-4 AC2** — OLD "…the registry moves to `maos-domain` with its 15-2 allowance; the 63 unregistered `MAOS_*` reads (16 names in kernel-core) are registered without a kernel edit" → NEW "`check_env_contract.rs:119` scans every workspace crate's `src/` with per-crate proven-red (12.6 design); the registry becomes the kloc-free data file `xtask/env-contract.toml` (479 code lines leave `maos-bin`; `EnvVar`/`EnvStability` +15 in `maos-domain`); the 55 unregistered src reads (64 incl. tests; 15 of the 16 kernel-core names) are registered as data; `Secret` classifies `ANTHROPIC_API_KEY`-class values and `MAOS_REGION_HOME` is recorded as key-derivation input (D4c)". Delete "with its 15-2 allowance".
17. **21-4 AC3** — append "the boot check lives in `maos-bin` at cohort-manifest load and compares `Region::resolve_home` (`maos-domain/region.rs:97`) with `TeamEntry.region` (`maos-cohort/src/manifest.rs:156`); mismatch = typed refusal; `operator_config.rs:203` (kernel) untouched".
18. **21-4 AC4** — keep the phantom-budget deletion (`kloc.toml:197,199,200`); delete the "19 canonical / 10 SigningKey" sentence (not a deliverable); add "D11-E1/E2/E3 (`epic-14-preflight-decisions.md:429-472`) each closed or re-pointed with a citation".
19. **Header Closes** — OLD "D1, D4a/b/c, D11, D13, D14" → NEW "D4a/b/c, D11, D13(b); D1 → 20-3 (proposal :86); D14 → 15-1/15-2 (the +51 is absorbed there)". OLD "NFR-Rel-6" → NEW "RELEASE-HOLDS row 12 (NFR-Rel-6 semantics preserved)".
20. **21-5 AC1** — append "the 817 code lines land in NEW leaf crate `maos-orchestrator-buffer` (kloc row + `check-workspace-count`), the port trait in `maos-domain` (+30–60 under 15-2); kernel tests `gateway_dispatcher_fr54.rs`, `gateway_uninstall_fr65_v05.rs`, `nfr_perf_4_pause_resume_latency.rs` move with it; the `maos-kernel-core` ceiling is LOWERED by the same amount (ADR-038); order 21-4 → 21-5; `check-cohort-mesh` re-runs the pin at `check_cohort_mesh.rs:674`". Replace "§4.0.7 naming" with "§4.0.7 'does NOT embed an orchestration policy' (`4-kernel-design.md:185`)".
21. **Dependencies** — OLD "Epic 20 (20-3 evidence states for the nightly; 20-2 packages for multi-host runbooks). 17-3b before 21-2." → NEW "15-2 (allowances for maos-a2a-core/-tcp/cohort/domain/bin), 15-3 (`check-exit-commands` resolving `maos` tokens), 15-5 (ADR-063 → 21-3), 15-6 (`MAOS_INFERENCE_MODE` → 21-1), 18-4 (clock → 21-1), `ops-j4-live-incident-recording` → 21-1. Order: 21-2 ∥ 21-4 (may run alongside Epic 20) → 21-3 → 21-5 → 21-1."
22. **Duration** — OLD "5–7 weeks (measured)" → NEW "7.8–12.6 wk serial; 5.5–8.5 wk with 21-2/21-4 parallel; 21-5 +1.3–2.0".

### CONFIRMED TRUE (do not re-check)

- `demo-reza` exists with `--provision/--skip-gates/--journey-only`, reds on an absent substrate, runs the 13.6b journey `--include-ignored`, reads the 13.6e ledger; no workflow invokes it.
- `provision-loom-substrate` composite action; four `services:` jobs in `discipline.yml` (`:2930` cross-region-consensus, `:2995` multi-region-slo, `:3077` multi-tenant-loom, `:3130` reza); `journey-nightly.yml` has none.
- Ignored inventory: `13_6b` ×4 (`:1641,:1837,:2354,:2810`), `t_12_4a` ×6 (`:298-593`), `maos-journey-test` ×2 (`j4:117`, `butler:285`); all mechanism tests are per-commit gate legs.
- `Mira::diagnose(&AnomalySignal)` `:274`; `with_pending_signals` `:241`; `on_idle` `:183`; no inference in Mira/Nash; `covered_model_id` only in Mira's manifest `:68`; J4 topology file; `journey_j4.rs` Grade-A test runs `maos run … --once` and asserts `spirit_loaded {mira,nash}` + `drain topology:true`.
- κ corpus = `tests/corpora/safety-critical-mira-nash-v1.5.jsonl`, 300 rows, `MANIFEST.toml:73-76`.
- `TofuPinStore` trait `tofu.rs:95` (maos-a2a-core), doc `:90-93` names `maos-persistence`; sole impl `InMemoryTofuPinStore` `:508`; `verify_pinned_sync` `:457`; `find_active_pin_by_fingerprint` `:250`; `maos-persistence` = placeholder (8 lines, no deps, no dependents, kloc 1/2000).
- `swap_serving_cert` `transport.rs:541` with no production caller; `cert_rotation.rs` exists; `install_cert_rotation` at `main.rs:9824`; both 14-2a legs at the cited lines; `cold_deployment_t_grace()` = 5 s; 14-2d §3b ratified `G = cold + I` ≈ 65 s and F3 `RwLock`; 14-2b and 14-2c `done`; no 14-2d tracker row.
- `journal_peer_identity_refusal(direction, peer_hint, detail)` `router.rs:502-507`; `RuptureReason` `#[non_exhaustive]` `frame.rs:384`; `PeerIdentityUnverified` exists.
- Operator routes: `/v1/a2a/rotation-windows` `:299`, `/v1/cohort/peer-versions` `:347`, `/v1/cohort/self-identity` `:395`; no pin route.
- Instruments: physical pin 24472 (equality), tokei ceiling 18933/18933; baseline TOML invalid at line 29 by design; `read_pinned` single-sourced; `check_fkcs` counts the frozen tag physically; `check_epic_close_coherence` reds stale pins in non-done epic files; `check_cohort_mesh.rs:674` re-runs the pin.
- `check-empty-kernel` RED at HEAD for I9 whitelist/exemption reasons (15-1 scope), not orchestrator naming; `check-loom` blocklist has `Orchestrator` (`loom-blocklist.toml:14`) over `kernel-crates.toml`.
- kloc @HEAD: 8 crates at zero (`maos-a2a-core, maos-audit, maos-bin, maos-cli, maos-cohort, maos-iac, maos-kernel-core, xtask`), `maos-domain` −51, aggregate 155498/147057; `spirits/` and `tests/` excluded from tokei.
- Env registry: 86 `MAOS_*` names, 87 entries, 507/479 lines, `EnvStability = {HarnessOnly, UserFacing}`; scan root `maos-bin/src` (`:119`); measured unregistered 55 src / 64 all / 15 kernel-core.
- Three phantom kloc rows (`:197,:199,:200`); `orchestrator/` 5 files 1028/817; `scheduler_loop.rs:89,111`; `main.rs:1995,5696,5788,5859`; `TeamEntry.region`; `MAOS_REGION_HOME` read at `region.rs:97` and `operator_config.rs:203`.
- ADR-063 absent (15-5 authors it); D1 → 20-3 per proposal; D13(b)/D14 name 21-4.

## Epic-specific questions — index

1. Exit block: `demo-reza` exists (`main.rs:1012`), reds on absent DB (`demo_reza.rs:71-81`), runs the journey `--include-ignored` (`:241-245`); line 2 is prose, no gate (A#7-11).
2. 21-1: `lib.rs:274` TRUE; provenance string only in Mira's manifest `:68`; `journey_j4.rs:117` TRUE; κ corpus = `tests/corpora/safety-critical-mira-nash-v1.5.jsonl` (300); topology exists; `--incidents` NEITHER; clock = ONE mechanism to build in 18-4 and consume here (Epic-18 note §H-6), currently none (A#16-28, §H-1).
3. 21-2: all citations TRUE; premise inverted — the packages already run as gate legs; `t_12_4a:593` TRUE; `day-30-raw.json` TRUE; `:457` TRUE and already binding in CI; nightly has no `services:`; 2 ignored journey tests in `maos-journey-test`, owners: j4 → 21-1, butler → none (A#29-41, blocker 2).
4. 21-3: `tofu.rs` is in `maos-a2a-core`; `:90-93`, `:457` TRUE; trait exists, in-memory only; `maos-persistence` = 8 physical / 1 tokei lines (memory's "1 line" is the tokei count); `:541` TRUE; both 14-2a legs TRUE; `router.rs:502-507` TRUE but the fn already has `detail`; `:1341` off by 8 (`:1333`); AC2 matches 14-2d's cold+I but names 2 of 6 ACs and omits F3/RwLock; no kernel files touched (A#42-54).
5. 21-4: TOML invalid at line 29 confirmed by parser; `:119` TRUE; my count 55/64 vs 63, kernel 15/16; `region.rs:97`/`operator_config.rs:203` are the reads (ranges span tests); `TeamEntry.region` TRUE; phantoms = `maos-cap-registry`, `maos-wire`, `maos-journal`; "19/10" traces to the prior preflight §1 row 20; relocation to `maos-domain` CONFLICTS with 15-2 (+750 vs +300 already drawn) (A#55-72, blocker 3).
6. 21-5: 5 files / 1028 physical / 817 tokei; `scheduler_loop.rs:89,111` TRUE; `main.rs` lines TRUE; pin re-run at `:674` not `:604`; no gate assumes monotonic growth; the 817 lines have no kloc home (A#73-79, §D, §E).
7. Dependencies: for the hermetic exit only 15-2 (+ a `services:` block) is needed; 17-3b is vestigial; 21-2 and 21-4 can run in parallel with Epic 20 (§I).
8. Duration: 7.8–12.6 wk serial / 5.5–8.5 parallel vs 5–7 stated (§G).
