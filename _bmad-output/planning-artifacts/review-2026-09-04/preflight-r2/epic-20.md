# Preflight R2 — Epic 20 Ship It (W5)

> Scout note, 2026-09-05. Epic file: `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md`. Every citation pinned `@9f920180` (HEAD; `git status --porcelain` showed only the untracked `preflight-r2/` directory, so worktree reads equal `git show 9f920180:`). Method: read-only. Things RUN, not inferred: `cargo run -p xtask -- kloc-check` (exit 1, table in §E); `cargo build -p maos-bin --no-default-features --features air-gap` (exit 0) and `target/debug/maos --version` on that binary (`maos 0.1.0-alpha`); `bash -n` on the exit block (syntax error, §B). Builds on the Epic 15–19 notes' verdict blocks and §K; nothing they established is re-derived here.

## Verdict block

| | |
|---|---|
| **Verdict** | **needs-rework** (not blocked: the registry protocol, client, admission path, vetting crypto, yank observer, air-gap feature, packaging templates and ledger workflows are all real — but the exit block is not runnable as written on three of its four commands, one story rests on two hotfix-lane rows outside any epic chain, and the biggest story is unbounded by its own text) |
| **Confidence** | **42 / 100** (epic header asserts the eight rules; it does not state a target) |
| **Measured duration** | **5–8.5 weeks** serial for one engineer + agents (epic says 5–7); 20-3 alone is 2–4 weeks as written and is the whole tail |
| **False / partial premises** | **9 FALSE/NEITHER, 21 PARTIAL** out of 49 audited claims (§A); 19 CONFIRMED TRUE |

**Top-3 blockers**

1. **The exit block cannot run.** `maos-registry-server` is a 7-line stub that prints "not yet wired — Story 5.5d Task 13" and exits 1 (`crates/maos-registry/src/bin/server.rs:1-7`); it has no `--bind`, is not built by `release.yml:44`, and is not in 15-4's artifact set. Line 2 is a bash syntax error (`&  &&`). `--from-local` takes a *directory* holding `SHA256SUMS`, `SHA256SUMS.sig` and the platform binary (`crates/maos-cli/src/cli.rs:355-358`, `subcommands.rs:1241-1251`), not `./maos`. `maos-spirit publish` has no positional — `--manifest` and `--artifact` are required (`crates/maos-spirit-cli/src/bin/maos-spirit.rs:39-45`). `researcher@1.0` does not exist (`spirits/researcher/manifest.toml:29` is `0.5.0`). No `maos-<target>.tar.gz` is produced by `release.yml` (bare binaries, `:17-23,:50`) or promised by 15-4 AC1, and every packaging file fetches bare binaries (`packaging/homebrew/maos.rb:24-36`, `packaging/aur/PKGBUILD:21-26`, `packaging/deb/rules:14-16`).
2. **20-4 stands on two rows outside every epic chain.** `MAOS_DEFAULT_PROVIDER` does not exist in code (grep over `crates xtask`: zero hits; only planning docs) — it is the `provider-model-env-overrides` row of the model-pin hotfix lane (`sprint-status.yaml:224`). `xtask/model-currency.toml` does not exist (`ls xtask/*.toml`) — it is created by `model-pin-currency-gate-and-retired-pin-sweep` (`sprint-status.yaml:223`). Neither row is a dependency of Epics 15–20, and AC1/AC3 cite both as if present (rule 3).
3. **20-3 is unbounded.** "23 absent-pass gates" is a count without a list; from the tree I count **19** by the strict definition plus two borderline (§A #25, §C). "Every gate carries `BindingClass`" means **59** of the 75 registered gates (`gate-registry.toml:5-162`) that do not (17 files carry it today). "≥10 gates retired … xtask net lines ≤0" names none, while 15-2 grants xtask **no** headroom ("xtask net ≤0 after retirements", `epic-15-foundations-w0.md:52`) and xtask sits at 41953/41953. AC4's generated coverage matrix must house 198 hand-authored entries with `phase`/`valid_until`/`notes`/`corpora` fields (`tests/coverage-matrix.yaml:15-33`, 1982 lines) that a per-gate `covers` field cannot express, and a generated file collides with `invariant_lock.rs:156` ("corpus delta" = the yaml must be touched).

---

## A. Citation audit

Legend: TRUE / PARTIAL (exists, claim about it wrong) / FALSE / NEITHER (does not exist and no story+AC creates it).

| # | Claim | Where | Verdict | Evidence @9f920180 |
|---|---|---|---|---|
| 1 | "5–7 weeks (measured)" | header | PARTIAL | `sprint-change-proposal-2026-09-04-round2.md:56` states 5–7 wk with no analogue table; measured range in §G is 5–8.5 wk |
| 2 | `curl -L <release>/maos-<target>.tar.gz \| tar xz` | exit L1 | FALSE | `release.yml:15-23,:47-54` packages bare `maos-linux-amd64` / `maos-linux-arm64` / `maos-darwin-arm64`; no tar step; 15-4 AC1 (`epic-15-foundations-w0.md:76`) adds `maosctl` per target, not a tarball; `maos.rb:24-36`, `PKGBUILD:21-26`, `deb/rules:14-16` all fetch bare binaries. `<release>`/`<target>` are placeholders `check-exit-commands` cannot resolve |
| 3 | `./maosctl install --from-local ./maos --verify-only` | exit L1 | PARTIAL | flags exist (`maos-cli/src/cli.rs:351-358`), but `from_local` is "a locally-staged release artifact directory. Must contain SHA256SUMS, SHA256SUMS.sig, and the binary" (`:355-356`); `install_from_local` joins `dir/SHA256SUMS`, `dir/SHA256SUMS.sig`, `dir/<platform_binary_name()>` (`subcommands.rs:1241-1251`) → `./maos` as a file exits 2 |
| 4 | `./maos-registry-server --bind 127.0.0.1:7500 &` | exit L2 | NEITHER | binary declared (`maos-registry/Cargo.toml:36-38`); `src/bin/server.rs:4-6` = `eprintln!("maos-registry-server: not yet wired — Story 5.5d Task 13"); std::process::exit(1);`. No `--bind`. Real server `SpiritRegistryServer` (`server.rs:52-90`) is reachable only as `MAOS_ONE_SHOT=registry-server maos` at `maos-bin/src/main.rs:7693-7701`, hard-coded `"127.0.0.1:6789"`, `None` org key, `LocalFsRegistryStorage::new()` (`~/.local/share/maos/registry/`, `storage.rs:89-96`). Not built by `release.yml:44` (`-p maos-bin` only). No AC in 20-1 wires it |
| 5 | `&  &&` | exit L2 | FALSE | `bash -n`: "syntax error near unexpected token `&&`" |
| 6 | `maos-spirit publish --registry-uri … --tier=local spirits/researcher` | exit L2 | PARTIAL | `--tier` (`maos-spirit.rs:36-37`), `--registry-uri` (`:57-58`) exist; `--manifest <PathBuf>` and `--artifact <PathBuf>` are required (`:39-45`); no positional. Server ignores the URL path (`server.rs:150-155` checks only `POST`), so `/mcp` is not required |
| 7 | `maosctl spirit install researcher@1.0 --registry-uri …` | exit L3 · 20-1 AC1 | PARTIAL | `maosctl spirit` exists (`cli.rs:97-99`; `SpiritOp` = `HotSwapPrecheck`/`Upgrade`/`Inspect`, `:803-870`); `install` is new — consistent with AC1. But `spirits/researcher/manifest.toml:29` `version = "0.5.0"`, `:34 trust_tier = "local"`, `:33 forms = ["rust-inproc"]` — and 17-3b AC4 refuses `rust-inproc` from any registry (`epic-17…md:70`), so the published Researcher is refused at install by the same epic's dependency |
| 8 | `MAOS_INFERENCE_MODE=replay maos shell` | exit L3 | PARTIAL | `shell` verb exists (`maos-bin/src/main.rs:1730-1737`); `MAOS_INFERENCE_MODE` is 15-6 AC1/AC4 (`epic-15…md:96,:99`), absent at HEAD (grep: epic files only) — cross-epic verb needs the edge stated |
| 9 | ops rows exist | header | TRUE | `sprint-status.yaml:281` provisioning, `:282` cohort+pen-test, `:283` brew/AUR, `:284` GA tag |
| 10 | "ZERO kernel-Δ @24472" | header | TRUE (pin) | `xtask/kernel-core-baseline.toml:472 src_lines = 24472`; per-story risk in §D |
| 11 | `publish.rs:267-272` stub-only guard | 20-1 AC1 | TRUE | `maos-spirit-cli/src/publish.rs:267-272` `Err(anyhow!("maos-spirit publish for non-`stub` registry URIs requires the kernel-side composition root; invoke through `maosctl publish` (Story 7.4+) …"))`. Note `maosctl publish` does not exist (`cli.rs`: no `Publish` variant) |
| 12 | `McpSpiritRegistryClient` "from `main.rs:2881`" | 20-1 AC1 | PARTIAL | `:2881` is a comment (`//   any HTTP(S) URI → McpSpiritRegistryClient over McpClient::StreamableHttp.`); the type is `maos-registry/src/client.rs:38`, `new(Arc<dyn McpClient>, name)` `:44-50`; wiring site `main.rs:2925,:2948`. maos-spirit-cli already depends on `maos-mcp` + `maos-registry` (`maos-spirit-cli/Cargo.toml:31-32`) so the wiring is feasible; `maos-cli` does not depend on `maos-mcp` (`maos-cli/Cargo.toml:23` = maos-registry only) and `maos-registry` re-exports the client but not `McpClient` (`lib.rs:38-41`) |
| 13 | `subcommands.rs:942-1085` import admission | 20-1 AC1 | TRUE | `dispatch_import` `:942`; `admit_spirit_with_attestation` / `admit_spirit` `:1073-1083`. `--registry-uri` on `import` is parsed and ignored (`:943-948`) |
| 14 | `issue_attestation` `attestation.rs:150`, zero callers | 20-1 AC2 | PARTIAL | `maos-compliance/src/vetting/attestation.rs:150` TRUE; callers are all test code (`maos-registry/src/admission.rs:1166-1219` inside the `#[cfg(test)]` module; `tests/vetting_attestation_gate.rs:176`; `tests/end_to_end_test.rs:229`) → "zero production callers" |
| 15 | "plus a revoke primitive and keyring enrollment" | 20-1 AC2 | PARTIAL | primitives exist: `VetterKeyring::push_attestation_revocation` (`keyring.rs:157`), `is_revoked` (`:334`), `verify_attestation_not_revoked` (`:288`), `issue_event` (`:117`, enrollment events), `SignedRevocationList`; what is missing is the CLI and a signed-revocation *issuer* |
| 16 | `TlYankObserver::on_yank` `main.rs:305-338` "journals only" | 20-1 AC3 | PARTIAL | `:305-339` TRUE; it journals `FrameKind::SpiritRevoked` (`:316-323`) **and** calls `maos_compliance::observe_running_spirit(… registry_yanked: true …)` (`:328-338`), which classifies a terminal cause and journals a `RunningSpiritObservation` through `TerminalObservationSink` (`:281-300`). No policy (grep `quarantine|auto.revoke|policy` in `yank.rs`, `vetting/terminal.rs`: none) |
| 17 | AC4 WASM installs through the same path | 20-1 AC4 | PARTIAL | `maos-registry/src` has zero `wasm` mentions; WASM admission is 17-3b AC4 (`epic-17…md:70`) — a dependency, not a mechanism |
| 18 | `release.yml` has no deb step | 20-2 AC1 | TRUE | no `dpkg` in `release.yml`; but `packaging/deb/control` + `rules` (debhelper, `rules:21-22 dh $@`) already exist from 9.4 — "via `dpkg-deb`" is a fork (§H) |
| 19 | `--no-default-features --features air-gap` | 20-2 AC1 | TRUE | `maos-bin/Cargo.toml:15 default = ["network"]`, `:29 air-gap = []`; **compiles at HEAD** (ran it, exit 0; `target/debug/maos --version` → `maos 0.1.0-alpha` via the air-gap main `main.rs:1318-1342`) |
| 20 | `check-mock-not-in-release` exists / re-run | 20-2 AC1 | TRUE | `xtask/src/check_mock_not_in_release.rs:29-31`; `release.yml:46`; `discipline.yml:294-309` |
| 21 | `Dockerfile` takes `ARG FEATURES` | 20-2 AC1 | TRUE (to-do) | `Dockerfile:27 RUN cargo build --release -p maos-bin --locked`, no ARG; `container.yml:6-7` is a SCAFFOLD needing `DOCKERHUB_*` secrets (`:43-44`) |
| 22 | `check-air-gap` is enrolled | 20-2 AC1 | TRUE (to-do) | exists (`xtask/src/check_air_gap.rs`; `main.rs:767-768,:1342-1347`) but absent from `gate-registry.toml`, `discipline.yml`, `tests/coverage-matrix.yaml` (grep: 0 hits) |
| 23 | netns script "reports SKIP as non-green" | 20-2 AC1 | PARTIAL | `tests/air-gap-netns-corroborate.sh:29-48` — four `SKIP … exit 0` branches; **no workflow runs it** (grep `.github xtask/src`: 0), so "non-green" has no consumer until the AC names one |
| 24 | `maos.rb`/`PKGBUILD` contain `SCAFFOLD`/`PLACEHOLDER_SHA256` | 20-2 AC2 | TRUE | `maos.rb:7,:25,:32,:36,:42,:47`; `PKGBUILD:6,:27-32`; also `rpm/maos.spec:4` SCAFFOLD and placeholder pubkey `maos.rb:18-20`, `PKGBUILD:34-36`, `deb/rules:5-10` — not named by the AC. `maos.rb:76` test runs `maos --version`, which only the **air-gap** main handles (`main.rs:1332`); the network main has no `--version` arm (grep: single hit) → the formula's test boots the daemon |
| 25 | "23 absent-pass gates … 7 carry the seam" | 20-3 AC1 | PARTIAL | strict count from the tree = **19**: file-absent advisory PASS (9): `check_cna_registration.rs:54-60,:132-134`, `check_pentest_gate.rs:85-87`, `check_red_team_gate.rs:133-135`, `check_skill_conformance.rs:141-153`, `check_third_party_trial.rs:208-210`, `check_cross_form_equiv.rs:189-191`, `check_fuzz_floor.rs:76-85`, `check_migration_merkle.rs:185-194`, `check_rto_gate.rs:57-72`; phase-advisory (RED or skipped leg → `Ok(())` at `CURRENT_PHASE = "v1_5"`) (6): `check_trial_attestation.rs:122,:136-139`, `check_wasm_form_equiv.rs:293`, `check_enterprise_pdp.rs:536`, `check_enterprise_identity.rs:620`, `check_escape_detector.rs:642`, `check_fkcs.rs:349`; AdvisorySubstrate ledger gates (`EvidenceState::Absent` → `Ok(())` off the `MAOS_LEDGER_ENFORCE` lane, `evidence_ledger.rs:131-140,:1528-1530,:1596-1597`) (4): `check_cross_region_consensus.rs`, `check_multi_region_slo.rs`, `check_multi_tenant_loom.rs`, `check_reza_production_path.rs`. Borderline (2): `check_j1_two_host_signed_run.rs:1301-1309` (capture absent → PASS with the claim refused) and `coverage-matrix` (`tests/coverage-matrix.yaml:3 mode: warning`). `EvidenceState` is in 7 **files** (`gate_common.rs`, `evidence_ledger.rs`, `demo_j1.rs` + the 4 gates) → **4 gates** carry the seam |
| 26 | `check-rto-gate` exits 0 as `skipped` | 20-3 AC1 | TRUE | `check_rto_gate.rs:57-72` (`Ok(())` unless measured FAIL) |
| 27 | `participants_total < 12` → FAIL | 20-3 AC2 | TRUE | `check_third_party_trial.rs:300-303` |
| 28 | "`fuzz-ledger` and `rto-ledger` branches exist" | 20-3 AC3 | PARTIAL | no such branch locally (`git branch -a`); workflows expect them (`fuzz-cadence.yml:25-29`, `rpo-rto-cadence.yml:17-18`); creating them is already in `ops-provisioning-secrets-and-accounts` (`sprint-status.yaml:281` "fuzz-/rto-ledger branch permissions") → duplicated into a hermetic AC (rule 2, §F) |
| 29 | "fuzz runs as chained ≤6 h jobs" | 20-3 AC3 | TRUE (to-do) | today 600 s × 4 workers, weekly (`fuzz-cadence.yml:35-38,:77-82`) |
| 30 | "the floor stays advisory until the ledger spans 90 days" | 20-3 AC3 | TRUE | `check_fuzz_floor.rs:13`; `gate-registry.toml:237-247`; **and** `fuzz-cadence.yml:12-17`: at weekly cadence the floor is ~8× unreachable and "must be re-derived for weekly BEFORE auto-promotion" — the AC omits this |
| 31 | `covers = [...]` per gate; matrix generated; "no proc-macro crate" | 20-3 AC4 | PARTIAL | no `covers` field exists (grep `gate-registry.toml`: 0); the matrix is keyed by **requirement** (198 top-level keys) with per-entry `gates`, `corpora`, `phase`, `valid_until`, `notes` (`coverage-matrix.yaml:15-33`); `coverage_matrix.rs:93-120` cross-checks registry keys and `phase-config.toml`; `invariant_lock.rs:54,:156` reds any invariant change that does not touch `tests/coverage-matrix.yaml` — a generated file changes that rule's meaning. Proc-macro dropped vs old 19-4 AC3: fixed |
| 32 | "`invariant-lock` re-ratified" | 20-3 AC4 | PARTIAL | `invariant_lock.rs` exists (`main.rs:134,:1106`); "re-ratified" names no mechanism (what changes: the corpus-delta rule at `:156`) |
| 33 | 14-e1 erasure attestation | 20-3 AC5 | TRUE (site) | `maos-audit/src/erasure/regional_teardown.rs:158 completed: true` (unconditional, per 13.5b); lands in **maos-audit 6847/6847** — 15-2 grants maos-audit +100 (`epic-15…md:52`) |
| 34 | "every gate carries `BindingClass`" (e12-b1, D20) | 20-3 AC5 | PARTIAL | `BindingClass` enum `gate_common.rs:210-222`; carried by 16 gate modules + `evidence_ledger.rs` (17 files, 289 refs); registry lists 75 gates (`gate-registry.toml:5-162`) → **59** without; E12-B1 is tracked `done` with "remaining reach gap … tracked forward" (`sprint-status.yaml:300`) |
| 35 | "≥10 gates retired with invariant-lock review; xtask net ≤0" | 20-3 AC6 | PARTIAL | `gate-registry.toml:2-3` "removing or renaming a gate is a breaking change that requires invariant-lock review"; six enrolment sites per Epic-15 note (`xtask/src/main.rs:96,:949,:1360`, `check_ship_gate_completeness.rs:20-60 EXPECTED_GATES` (40 rows), `tests/coverage-matrix.yaml`, `discipline.yml` aggregate `needs:` = **123** entries from `:3510`, `gate-registry.toml`); no list; 15-3 AC4 already retires `check-cna-registration` (`epic-15…md:70`) — overlap unstated |
| 36 | `MAOS_DEFAULT_PROVIDER=gemini` | 20-4 AC1 | NEITHER | zero hits in `crates xtask`; provider map built at `maos-bin/src/main.rs:3081-3100` with literal ids and `default_id` = first registered; the env var is `provider-model-env-overrides` (`sprint-status.yaml:224`, backlog, hotfix lane) |
| 37 | `provider_endpoint_pin` parse-only at `maos-manifest:2103-2171` | 20-4 AC2 | TRUE | `maos-manifest/src/manifest.rs:2103` field, `:2159-2171` 64-hex validation; no other consumer (`maos-spirit-abi/src/compliance.rs:507` is a test) |
| 38 | Gemini driver | 20-4 AC1 | TRUE (to-do) | `maos-providers/src` = anthropic/ollama/openai/fixture_replay/rate_limit (1570 lines); `xtask/fr47-vendor-sdk-denylist.toml:17 "gemini-rs"` → hand-rolled HTTP like `anthropic.rs` (307 lines) |
| 39 | `xtask/model-currency.toml` governs the Gemini IDs | 20-4 AC3 | NEITHER | file absent (`ls xtask/*.toml`: 24 files, none named so); created by `model-pin-currency-gate-and-retired-pin-sweep` (`sprint-status.yaml:223`, backlog) |
| 40 | `later-bedrock-vertex-and-kms-backends` row | 20-4 AC3 | TRUE | `sprint-status.yaml:285` |
| 41 | `git tag v1.0.0-rc.1 && git push --tags` produces signed artifacts | 20-5 AC1 | PARTIAL | `release.yml:3-4` triggers on `v*`; signing needs `RELEASE_SIGNING_KEY` + `MAOS_RELEASE_PUBKEY` (`:74-77`) → operator secrets (§F); `softprops/action-gh-release@v2` (`:88-94`) has no `prerelease:` → an rc publishes as a full release; `container.yml:12-14` also fires on `v*` with unconfigured secrets |
| 42 | "including the `.deb` and the air-gap variant" | 20-5 AC1 | PARTIAL | both are 20-2 deliverables; Dependencies line says only "20-1 before 20-5" |
| 43 | "`STABILITY.md` shows the rc row" | 20-5 AC1 | PARTIAL | no rc concept: `stability_matrix.rs:17,:137` sources `kernel_version` from `Cargo.toml:64 version = "0.1.0-alpha"` (15-4 AC5 → `0.1.0-alpha.1`); the only version-shaped rows are the triple (`:185-187`) and the LTS clock (`lts_clock_start` `:352-367`, exact `1.0.0`/`v1.0.0` refs only — an rc correctly does not start it, but the placeholder text still says "cut in Epic 10" `:367`, `STABILITY.md:59-60`). No AC bumps the workspace version to `1.0.0-rc.1` |
| 44 | "`docs/release` says so" | 20-5 AC2 | TRUE (to-do) | `docs/release/` = `v1.5-topology-support.md`, `v2.2-capacity-envelope.md`, `windows-deferral.md`; `tag-procedure.md` is 15-4 AC5 |
| 45 | `ops-ga-tag-after-holds` | 20-5 AC2 | TRUE | `sprint-status.yaml:284` |
| 46 | "Epic 19 … 20-1 before 20-5" | Dependencies | PARTIAL | see §I: eight missing edges |
| 47 | 20-2 AC3 `maos uninstall` (16-4) | 20-2 | TRUE (dep) | `epic-16…md:74` new top-level verb; today `uninstall` is a `MAOS_ONE_SHOT` lifecycle mode (`main.rs:5148-5170`) |
| 48 | `check-exit-commands` (rule 1) | header | NEITHER at HEAD | no `xtask/src/*exit*` module; it is 15-3 AC3 (`epic-15…md:68`) |
| 49 | "the stub-only guard deleted; client wired into maos-spirit-cli" | 20-1 AC1 | TRUE (feasible) | see #11, #12 |

## B. Verb-first

Fenced exit block, token by token:

| Token | Status | Evidence |
|---|---|---|
| `curl -L <release>/maos-<target>.tar.gz \| tar xz` | NEITHER (layout) + network | §A #2; a clean-VM download is a network leg and needs the 15-4 tag to exist → §F |
| `./maosctl install --from-local <dir> --verify-only` | EXISTS | `cli.rs:351-358`; argument must be a directory (§A #3) |
| `./maos-registry-server --bind` | NEITHER | stub binary, no flag (§A #4); no AC creates it |
| `maos-spirit publish --registry-uri --tier=local` | EXISTS, wrong shape | `--manifest`/`--artifact` required; guard at `publish.rs:267-272` refuses non-stub until 20-1 AC1 |
| `maosctl spirit install <name>@<ver> --registry-uri` | 20-1 AC1 (new) | `SpiritOp` has no `Install` (`cli.rs:803-870`) — consistent |
| `MAOS_INFERENCE_MODE=replay` | 15-6 AC1/AC4 (Epic 15) | cross-epic; edge implied via Epic 19 → 18 → 15 but not stated |
| `maos shell` | EXISTS | `main.rs:1730-1737` |
| `MAOS_DEFAULT_PROVIDER` (20-4 AC1) | NEITHER in any epic | `sprint-status.yaml:224` hotfix-lane row |
| `cargo run -p xtask -- check-fuzz-floor` (20-3 AC1) | EXISTS | `main.rs:961` |
| `git tag … && git push --tags` (20-5 AC1) | exists but non-hermetic | §F |

Shell: line 2 does not parse (`bash -n` → "syntax error near unexpected token `&&`"). `15-3`'s `check-exit-commands` would also fail on `<release>` and `<target>`.

## C. Foundation audit

**20-1.** Assumes: (a) a runnable registry server binary — FALSE (stub, §A #4; the real server is a `MAOS_ONE_SHOT` mode of `maos` with hard-coded port); (b) an HTTP client reachable from `maos-spirit-cli` — TRUE (`maos-mcp` + `maos-registry` deps, `McpSpiritRegistryClient::new`); (c) admission reusable from a new `maosctl` verb — TRUE (`dispatch_import` `:942-1090`), but `maos-cli` lacks `maos-mcp` for the fetch and `maos-registry` does not re-export `McpClient` (`lib.rs:38-41`) → a new dependency edge `maos-cli → maos-mcp` (already in `Cargo.lock`; `check-dependency-closure` unaffected — not checked); (d) a `SignedPackage` for Researcher to publish — `publish.rs:98-120` builds it from `--manifest` + `--artifact` with `--tier` matching the manifest's `trust_tier` (`:111-118`) → `local` matches `manifest.toml:34`; (e) 17-3b's WASM admission and its `rust-inproc` refusal — TRUE as dependency, and it refuses the very Spirit the exit publishes (§A #7); (f) vetting issue/verify/revoke primitives — TRUE (`attestation.rs:150,:167`, `keyring.rs:117,:157,:288,:334`); (g) yank observer seam — TRUE (`YankObserver` trait, `TlYankObserver` `main.rs:269-339`, poller `yank.rs:130-191`); a policy knob home — NONE; `RegistrySection` lives in **kernel-core** (`security/operator_config.rs:12-26`) → §D.

**20-2.** Assumes: `dpkg-deb` packaging (existing `packaging/deb` is debhelper — fork §H); air-gap feature compiles (TRUE, ran); `check-air-gap` runnable in CI (needs `nm`, builds with `--no-default-features --features air-gap`, `check_air_gap.rs:52-60`; dirty-fixture bite test `tests/fixtures/dirty-network-fixture`); Docker build in CI (no `docker build` in `discipline.yml`; `container.yml` SCAFFOLD) → the image is a **tag-time** artifact needing DockerHub secrets; `maos uninstall` (16-4). Formula test needs a `maos --version` in the network build (only the air-gap main has it, `main.rs:1332`) — 15-3's `maos --help` proposal must include `--version`.

**20-3.** Assumes: a list of 23 gates (none given; 19 measured); `EvidenceState` projection reusable (TRUE: `gate_common.rs:328-420 EvidenceVerdict::project`, `evidence_ledger.rs:1519-1600 finish_ledger_gate`) — but it is bound to the **ledger** posture (`MAOS_LEDGER_ENFORCE`, signed harness records, `REPORT_DIR = tests/reports`), a heavy seam for file-presence gates like pentest/red-team/CNA; an honest-N mode (new; `:299-303` is a hard floor and `[[ship_gate]]` says `blocking-when-present` at v1_0/v1_5, `gate-registry.toml:175-177`); ledger branches (operator, §F); chained jobs (workflow edit; the collector at `fuzz-cadence.yml:104-120` would need to sum per-chunk records); a matrix generator with a home for 198 entries' `phase`/`valid_until`/`notes`/`corpora` (NONE); `invariant-lock` corpus-delta rule (`:156`) compatible with a generated file (NOT as written); `BindingClass` on 59 more gates (each needs a `dev_enforced_red_blocks` call site, `gate_common.rs:229-234`); retirement list (NONE); 14-e1 site (TRUE, `regional_teardown.rs:144-158`).

**20-4.** Assumes: `MAOS_DEFAULT_PROVIDER` (NONE), `model-currency.toml` + `check-model-currency` (NONE), 15-6 replay seam supporting a Gemini cassette (15-6 AC1 generalizes `MAOS_REPLAY_CASSETTE`; per-provider cassette naming not specified), a provider trait to implement (TRUE: `maos-providers/src/provider.rs:1-48`), an adapter site for pin enforcement (`InferencePortAdapter` `main.rs:3136` per Epic-15 note; providers constructed with literal endpoints `main.rs:3085-3100`), a typed refusal variant (not checked which enum; `fr63-typed-errors.toml` governs new error types — not checked).

**20-5.** Assumes: 15-4 landed (repaired `release.yml`, `maosctl` artifact, secrets present per `ops-provisioning`), 20-2 landed (deb + air-gap artifacts), a workspace version bump and `STABILITY.md` regeneration (`stability-matrix --check` drifts otherwise, `STABILITY.md:47`), `docs/release/tag-procedure.md` (15-4 AC5).

## D. Kernel-Δ audit

Kernel set = `crates/maos-kernel-core/src` (`kernel-core-baseline.toml:1-8`, pin 24472 `:472`).

| Story | Zero possible? | Risk |
|---|---|---|
| 20-1 | Yes, **if** the yank policy knob is NOT a `RegistrySection` field — that struct is kernel-core (`security/operator_config.rs:12-26`, constructor-guarded); put the policy in `maos-registry::yank` config or a maos-bin env/config read. Everything else is maos-spirit-cli / maos-cli / maos-registry / maos-bin | ~+5–15 kernel lines if the knob lands in `RegistrySection` — the AC must say where |
| 20-2 | Yes (CI, packaging, Dockerfile, script) | none |
| 20-3 | Yes (xtask, maos-audit for 14-e1, workflows) | none |
| 20-4 | Yes: driver in maos-providers; enforcement at the maos-bin adapter / provider construction (`main.rs:3081-3100`) against `ProvidersSection` (maos-manifest). Do not route through `inference/mod.rs` | the "typed refusal" must be an out-of-kernel error type |
| 20-5 | Yes | none |

## E. Kloc headroom

`cargo run -p xtask -- kloc-check` @9f920180 → exit 1 (aggregate 155498 vs `_aggregate_hardfail = 147057`, `kloc.toml:501`; `maos-domain` 8695/8644 OVER). Rows Epic 20 touches (measured / ceiling / headroom → 15-2 grant, `epic-15…md:52`):

| Crate | @HEAD | 15-2 grant | Epic-20 draw (estimate from analogues) | Verdict |
|---|---|---|---|---|
| maos-spirit-cli | 753/853/100 | +300 | 20-1: client wiring +80–150, `vet issue\|revoke` +150–250 | tight; 230–400 vs 400 |
| maos-cli | 5270/5270/**0** | +400 | 20-1: `spirit install` +250–400 (13.4 precedent: `cli.rs` +23, `subcommands.rs` +104 for one verb; a fetch+admit verb is larger) | **over-subscribed**: 16-1 also books maos-cli +100–200 (`epic-16…md:44`) |
| maos-registry | 3515/3615/100 | +300 | 20-1: +50–120 (yank policy, install helper) | ok |
| maos-bin | 17109/17109/**0** | +1200 | 20-1 yank observer +60–120; 20-4 provider registration + pin check +40–80 | Epic-15 note §E: +1200 already claimed by 15-1/15-6; brief: over-subscribed by 16/18/19 → Epic 20 has **no** maos-bin lines unless 15-2 is re-sized |
| maos-providers | 1252/2000/748 | +400 (already there) | 20-4 gemini.rs +300–400 | ok |
| maos-audit | 6847/6847/**0** | +100 | 20-3 AC5 14-e1 +20–60 | ok if the grant lands |
| maos-compliance | 2032/2117/85 | none | 20-1 AC2 revoke issuer +30–80 if placed here (else in maos-spirit-cli) | tight |
| maos-manifest | 4214/4314/100 | none | 20-4: 0 expected | ok |
| maos-mcp | 999/1100/101 | none | 0 | ok |
| xtask | 41953/41953/**0** | **none** ("xtask net ≤0 after retirements") | 20-3: EvidenceState on 19 gates +400–800; generator +300–500; `BindingClass` on 59 gates +600–1200; 20-2: enrol `check-air-gap` +0–30 (command exists) → **+1300–2500** before retirements | only fundable by retiring ~2500 lines of gate modules; the absent-pass modules AC1 converts are the natural retirees (cna 260, pentest 133, red-team 337, skill-conformance 190, cross-form 421, migration-merkle 424 …) — the two ACs compete for the same lines |

## F. Hermetic vs operator (rule 2)

| Where | Need | Routed? |
|---|---|---|
| exit L1 `curl -L <release>` | network + a published release (15-4 tag → `ops-provisioning-secrets-and-accounts` secrets) | NOT routed; hermetic substitute: the Epic-15 note's `release-dry-run` output or `actions/download-artifact` from the discipline run |
| 20-2 AC1 Docker image | `docker build` is hermetic; push/sign needs `DOCKERHUB_*` + cosign (`container.yml:33-44,:71-80`) | build hermetic, publish → ops row (not named) |
| 20-3 AC3 "branches exist" | push permission on two branches | duplicated from `ops-provisioning…` `:281` |
| 20-3 AC3 chained fuzz | wall-clock (hours) | fine as a workflow edit; verification is YAML-level |
| 20-3 AC2 "real cohort file committed" | ≥3 humans | the AC builds the mode; the file is `ops-external-cohort-and-pen-test` `:282` — OK |
| 20-4 AC1 live Gemini | paid key | stated as operator lane — OK, but no ops row names it |
| 20-5 AC1 `git tag && git push --tags` | network, push credential, `RELEASE_SIGNING_KEY`/`MAOS_RELEASE_PUBKEY` | NOT routed — same finding the Epic-15 note made for 15-4 (§K #17 there) |

## G. Sizing from measurement (+30%)

Landed analogues (`git show --shortstat`, dates from story files/commits):

| Analogue | Size | Days |
|---|---|---|
| 7.2 registry publish/install/yank/import (`42db268c`) | 51 files, +6316/−486; `client.rs` 570, `import.rs` 374, `main.rs` 477, `subcommands.rs` 125 | 5.5d 05-24 → 7.2 05-30: ~6 |
| 5.5d registry server + admission (`6a64a97e`) | 57 files, +6505 | ~4 (05-20..24, story notes) |
| 13.4 vetting machinery (`148a33ee` + `6e04a50c`) | 36 files, +3521/−209; `vetting/*` 1757 | created and done 2026-07-23: **1** |
| 11.7 trial infra (`5b749b4d`) | 21 files, +3373; `check_trial_attestation.rs` 460 | 07-07 → 07-08: **1–2** |
| 10.2 trial N=12 + red-team gates (`fc4bdc9a`) | 23 files, +3052; 4 gate modules | ~1–2 (06-21) |
| 10.3 compliance gates + fuzz cadence (`3806d9df`) | 98 files, +8607; `fuzz-cadence.yml` 164, 4 gates | ~2–3 (06-22) |
| 13.6e evidence ledger, 4 gates (`c45df0be`, `f2090745`) | 29 files +5465/−1347, then 34 files +7612/−474 | 08-04 grant → 08-07 → 08-11 closure: **7** |
| 9.4 distribution/packaging/air-gap (`3cf93ada`) | 55 files, +4198; packaging 319, workflows 174, `check_air_gap.rs` 223 | 06-14 split → 06-15: **1–2** (scaffold quality — every file it wrote is what 20-2 must correct) |
| 9.5c / 9.5d docs+release tooling (`17f7da85`, `b674adf3`) | +2624/−1020; +2052/−1253 | 1 each |

Per story (agent-days, ×1.3):

| Story | Basis | Days |
|---|---|---|
| 20-1 | 7.2 (client+verb) minus existing client, plus vet CLI (13.4-scale ÷3), yank policy, server wiring | 5–8 → **6.5–10.5** |
| 20-2 | 9.4 corrected end-to-end, deb + air-gap leg + Docker ARG + formula | 3–5 → **4–6.5** |
| 20-3 | 13.6e (4 gates, 7 d) scaled to 19 gates + generator + 59 BindingClass + 10 retirements + 14-e1 | 10–18 → **13–23** |
| 20-4 | one driver (`anthropic.rs`-sized) + cassette + pin enforcement, 5.5b precedent | 3–5 → **4–6.5** |
| 20-5 | 9.5c-scale docs + version bump + workflow prerelease flag (+ operator latency for secrets) | 1–2 → **1.5–3** |

Sum **29–49.5 agent-days ≈ 5–8.5 weeks** serial (epic: 5–7). Consistent only if 20-3 is scoped to a named list; as written 20-3 is the tail and the epic's own "governance ≤20%" rule (old CP-2) is inverted — 20-3 is ~45% of the epic.

## H. Fork audit (rule 7)

1. **20-1 registry server**: wire the stub `maos-registry-server` binary (clap: `--bind`, storage root, `--signing-key`, ship it as a fourth release artifact) **vs.** run `MAOS_ONE_SHOT=registry-server maos` (exists, `main.rs:7693-7701`, port fixed at 6789). Recommend: wire the binary (`SpiritRegistryServer::new(storage, addr, org_pubkey)` + `with_signing_key`), add it to 15-4's artifact list, exit line uses `--bind 127.0.0.1:6789` and `--registry-uri http://127.0.0.1:6789/mcp` (the client default, `publish.rs:288`).
2. **20-1 install transport**: `maosctl` fetches over MCP itself (new `maos-cli → maos-mcp` edge) **vs.** asks the running daemon over 16-1's POST surface. Recommend: direct MCP fetch (a clean VM has no daemon yet; `dispatch_import` is already daemon-less).
3. **20-1 yank policy home**: kernel `RegistrySection` **vs.** maos-registry `YankPoller` config / maos-bin env. Recommend: `maos-registry::yank` config struct read by maos-bin from `MAOS_YANK_POLICY` (registered in `env_contract.rs`), zero kernel lines.
4. **20-1 exit Spirit**: Researcher is `rust-inproc` (`manifest.toml:33`) and 17-3b refuses it from a registry. Decide: publish/install a WASM fixture Spirit (17-3b's TS Spirit) in the exit, or exempt `--tier=local` from the refusal. Recommend: the WASM fixture — it is what "a third party without the source tree" means.
5. **20-2 deb**: `dpkg-deb` raw **vs.** existing debhelper `packaging/deb/{control,rules}`. Recommend: debhelper via `dpkg-buildpackage -us -uc -b` inside `release.yml` (the rules already verify SHA256SUMS + signature, `rules:24-56`); delete the `dpkg-deb` wording.
6. **20-3 the 23**: name them. Recommend the 19 in §A #25 (+ the two borderline explicitly excluded with a reason), and state per gate whether it is **converted** (EvidenceState) or **retired** (AC6) — the same gate cannot be both.
7. **20-3 BindingClass scope**: all 75 registered gates **vs.** the 40 `[[ship_gate]]`/`EXPECTED_GATES` rows (`check_ship_gate_completeness.rs:20-60`). Recommend: the 40 (structural lints like `check-unsafe`, `kloc-check`, `breaking-md` have no substrate and are `Blocking` by construction); record the rest as `Blocking` in the registry without touching modules.
8. **20-3 matrix generation schema**: where `phase`/`valid_until`/`notes`/`corpora` live after generation. Recommend: keep a requirements-keyed source (`tests/coverage-source.toml`) + per-gate `covers` in `gate-registry.toml`; the generator joins them; `invariant_lock.rs:156` re-pointed at the source file.
9. **20-3 retirement list**: name ≥10 with their module line counts and the six enrolment sites; net xtask ≤0 shown as arithmetic in the story.
10. **20-4 provider selection**: `MAOS_DEFAULT_PROVIDER` (hotfix-lane row) **vs.** manifest `[providers].primary` (already parsed with the pin, `manifest.rs:2103-2171`). Recommend: land `provider-model-env-overrides` first (dependency edge) **or** select by manifest — one, stated.
11. **20-5**: workspace version `1.0.0-rc.1` bump + `prerelease: true` + `container.yml` guard — must be ACs, not implied.

## I. Dependency audit

Stated: "Epic 19 (the J1 demo is what a cohort installs). 20-1 before 20-5." Chain check: 19 → {17, 18} → {15, 16} (`epic-19…md:73`, `epic-17…md:84`, `epic-18…md:74`, `epic-16…md:89`) — so Epics 15–18 are transitively upstream, but the *specific* mechanisms Epic 20 consumes are not named:

- 15-3 (`check-exit-commands`, `maos --help`; retires `check-cna-registration`) → exit block + 20-3 AC6 overlap.
- 15-4 (release repair, `maosctl` artifact, tag procedure) → 20-2 AC1, 20-5 AC1/AC2. **Layout conflict**: 15-4 ships bare per-target binaries; 20's exit expects a tarball.
- 15-6 (`MAOS_INFERENCE_MODE`) → exit L3, 20-4 AC1.
- 16-4 (`maos uninstall`) → 20-2 AC3.
- 17-3b (WASM admission; `rust-inproc` refusal) → 20-1 AC4 **and** breaks 20-1's own exit Spirit (§H 4).
- **Outside every chain**: `provider-model-env-overrides` (`MAOS_DEFAULT_PROVIDER`) and `model-pin-currency-gate-and-retired-pin-sweep` (`xtask/model-currency.toml`) → 20-4 AC1/AC3. Either add the edge or drop the env var / file from the ACs.
- Intra-epic: **20-2 before 20-5** (deb + air-gap in the rc artifacts) and **20-3 before 20-5** ("the gates say what they measured" is the epic's Makes-true) are missing; 20-1 before 20-5 holds. 20-4 is independent.
- Downstream: Epic 21 depends on 20-2 and 20-3 (`epic-21…md:83`) — consistent.

## J. Did R2 fix the prior preflight?

Old E19 = `epic-19-w4-ship-it-v10.md` @`9f920180~1`; prior findings from `epic-15-20-preflight-2026-09-04.md:17,:41-47,:50`.

| Prior false premise (old E19) | R2 status | Evidence |
|---|---|---|
| release ships no `maosctl` (formula calls it) | **fixed by dependency, renamed in form** | moved to 15-4 AC1; but Epic 20's exit now assumes a tarball 15-4 does not produce (§A #2). Side note: `maos.rb:5` only *says* it calls maosctl; `install` (`:52-73`) never does — the prior wording overstated it |
| `maosctl install <name>` fetch removed at v0.5 | **fixed** | new verb `maosctl spirit install` (20-1 AC1); `subcommands.rs:1209-1210` confirms the removal |
| `maos-spirit publish` refuses every non-stub URI | **fixed (acknowledged, cited)** | `publish.rs:267-272` in AC1 |
| committing an honest N=3 file reds a green gate | **fixed** | 20-3 AC2 honest-N; `:300-303` |
| 23 (not 12) absent-pass gates | **renamed only** | count moved 12→23 but still no list; tree count 19 (+2 borderline) |
| fuzz floor cannot promote inside an epic | **fixed** | 20-3 AC3 says advisory until 90 d; omits the weekly re-derivation (`fuzz-cadence.yml:12-17`) |
| §2.5 exit verbs must exist (`maos uninstall` …) | **fixed for the old verbs; a new absent verb introduced** | `maos uninstall` → 16-4; `maos-registry-server --bind` is NEITHER |
| §2.6 provisioning as an operator checklist | **fixed** | four `ops-*` rows `:281-284`; but 20-3 AC3 and 20-5 AC1 still pull two of them back into hermetic ACs |
| §2.7 `sign-and-publish` needs only `build` | **fixed (Epic 15)** | 15-4 AC3 |
| §2.8 20-3/20-4 `maos-persistence` contradiction | **fixed** | proposal `:82` → 21-3 owns it |
| old 19-5 live Gemini/Bedrock/Vertex/Vault | **fixed (split)** — but `MAOS_DEFAULT_PROVIDER` and `check-model-currency` still assumed to exist | `sprint-status.yaml:223-224,:285` |
| old 19-4 AC3 proc-macro `#[maos_covers]` | **fixed in form, unpriced** | `covers` field; §A #31 |
| old 19-4 AC4 "75 → ≤40 gates" | **softened, still unnamed** | "≥10 retired" |
| old 19-2 `brew install` in the exit | **fixed** | `ops-brew-tap-and-aur-publication` |
| old 19-3 cohort + pen-test in an engineering story | **fixed** | `ops-external-cohort-and-pen-test` |
| old 19-1 AC2 Python template (FR33) | **n/a under Fork C** | 17-3b WASM form |

Net: 11 fixed, 3 renamed/softened, 2 new instances of the same shape (an absent verb; two absent rows cited as foundations).

## K. Verdict, required changes, confirmed-true

**Confidence 42.** Every seam Epic 20 needs exists somewhere in the tree — MCP registry server and client, admission, vetting crypto, yank observer, air-gap feature (it compiles), packaging templates, ledger workflows — which is why this is not `blocked`. What keeps it well below the line: the exit block fails on its first three commands for six independent reasons (stub server, syntax error, directory-vs-file, required flags, non-existent version, non-existent tarball) and publishes a Spirit its own dependency refuses; 20-4 cites an env var and a TOML file that exist only as backlog rows in a different lane; and 20-3 carries three unbounded quantities (which 23, which 59, which 10) under a crate with zero headroom and no grant, plus a matrix-generation design whose data has no home. All are edit-level, but 20-3 needs a spike (rule 5) before its ACs can be numbers. **Verdict: needs-rework. Measured duration 5–8.5 weeks.**

### REQUIRED CHANGES (OLD → NEW)

1. **Exit block** — OLD lines 1–3 → NEW:
   ```
   # clean VM; artifacts from the 15-4 release run (or `release-dry-run` output) staged in ./dist
   ./dist/maosctl install --from-local ./dist --verify-only
   ./dist/maos-registry-server --bind 127.0.0.1:6789 &
   maos-spirit publish --registry-uri http://127.0.0.1:6789/mcp --tier=public_untrusted --manifest spirits/<wasm-fixture>/manifest.toml --artifact target/wasm32-wasip2/release/<fixture>.wasm
   maosctl spirit install <wasm-fixture>@<ver> --registry-uri http://127.0.0.1:6789/mcp && MAOS_INFERENCE_MODE=replay maos shell
   ```
   (no `curl`, no placeholders, no `& &&`; the Spirit is one 17-3b admits.)
2. **20-1 AC1** — OLD "`McpSpiritRegistryClient` from `main.rs:2881` wired into `maos-spirit-cli`" → NEW "`McpSpiritRegistryClient` (`maos-registry/src/client.rs:38-50`, wired at `maos-bin/src/main.rs:2918-2948`) constructed in `maos-spirit-cli` over `McpClient::StreamableHttp`; **`maos-registry-server` (`src/bin/server.rs:1-7`, exits 1 today) wired to `SpiritRegistryServer::new` with `--bind <addr>` / `--root <dir>` / `--signing-key <path>` and added to 15-4's artifact set**; `maosctl spirit install` adds `maos-mcp` to `maos-cli` (`Cargo.toml:23`), fetches `registry.manifest` + `registry.artifact` and calls the admission at `subcommands.rs:1073-1083`; kernel-Δ 0."
3. **20-1 AC3** — OLD "today `TlYankObserver::on_yank` journals only, `main.rs:305-338`" → NEW "today `on_yank` (`main.rs:305-339`) journals `SpiritRevoked` and a `RunningSpiritObservation` (`:328-338`) but applies no policy; the policy (`warn|quarantine|auto-revoke`) is a `maos-registry::yank` config read from `MAOS_YANK_POLICY` (registered in `env_contract.rs`), **not** a `RegistrySection` field (`kernel-core/src/security/operator_config.rs:12-26`); kernel-Δ 0."
4. **20-1 AC2** — add: "revocation reuses `VetterKeyring::push_attestation_revocation` (`keyring.rs:157`) / `is_revoked` (`:334`); the CLI issues the signed revocation list; `issue_attestation` has zero **production** callers (test callers at `admission.rs:1219`, `tests/vetting_attestation_gate.rs:176`)."
5. **20-1 AC4** — add the dependency edge: "requires 17-3b AC4 (WASM admission; `rust-inproc` refused)"; and note the exit Spirit accordingly (change 1).
6. **20-2 AC1** — OLD "`release.yml` produces a `.deb` via `dpkg-deb`" → NEW "`release.yml` builds the `.deb` with `dpkg-buildpackage -us -uc -b` over the existing `packaging/deb/{control,rules}` (`rules:24-56` already verify SHA256SUMS + Ed25519), for amd64 and arm64"; OLD "the netns script reports SKIP as non-green" → NEW "`tests/air-gap-netns-corroborate.sh` (`:29-48`, four `SKIP … exit 0`) is run by the air-gap leg and exits non-zero on SKIP there"; add "the Docker **build** is a discipline job (`ARG FEATURES`); **push + cosign** stay in `container.yml` behind the `ops-provisioning…` secrets."
7. **20-2 AC2** — add: "`packaging/rpm/maos.spec:4` and the placeholder pubkey at `maos.rb:18-20` / `PKGBUILD:34-36` / `deb/rules:5-10` are corrected too; the formula `test` block (`maos.rb:76`, `maos --version`) uses a verb the network build answers (15-3's `--help`/`--version`)."
8. **20-3 AC1** — OLD "the other **23** absent-pass gates … (7 carry the seam today)" → NEW "the **19** gates listed in the story (9 file-absent advisory: cna, pentest, red-team, skill-conformance, third-party-trial, cross-form-equiv, fuzz-floor, migration-merkle, rto-gate; 6 phase-advisory: trial-attestation, wasm-form-equiv, enterprise-pdp, enterprise-identity, escape-detector, fkcs; 4 ledger gates already projecting `EvidenceState::Absent`: cross-region-consensus, multi-region-slo, multi-tenant-loom, reza-production-path) report `Absent`/`Indeterminate` as non-green; `check-j1-two-host-signed-run` and `coverage-matrix` (`mode: warning`) are decided explicitly. A gate retired under AC6 is not converted."
9. **20-3 AC3** — OLD "`fuzz-ledger` and `rto-ledger` branches exist" → NEW "the collectors handle a missing branch by failing loudly (today they warn and no-op, `fuzz-cadence.yml:28-29`); branch creation stays in `ops-provisioning-secrets-and-accounts`"; add "the weekly-cadence floor re-derivation (`fuzz-cadence.yml:12-17`) is recorded before any promotion."
10. **20-3 AC4** — OLD "generated from `gate-registry.toml` plus a `covers = [...]` field per gate" → NEW "generated from `gate-registry.toml` (`covers = [...]` per gate) **joined with** a requirements-keyed source carrying `phase`/`valid_until`/`notes`/`corpora` (198 entries today, `coverage-matrix.yaml:15-33`); `invariant_lock.rs:156` re-pointed at the source; `coverage_matrix.rs:93-120` unchanged; spike first (rule 5)."
11. **20-3 AC5** — OLD "every gate carries `BindingClass`" → NEW "every `[[ship_gate]]` row (40, `check_ship_gate_completeness.rs:20-60`) carries `BindingClass` (16 do today); structural lints are recorded `Blocking` in the registry"; add "14-e1 lands in `maos-audit/src/erasure/regional_teardown.rs:144-158` under 15-2's maos-audit +100."
12. **20-3 AC6** — OLD "≥10 gates retired with invariant-lock review; xtask net lines ≤0" → NEW "the ten named in the story (with module line counts and the six enrolment sites: `xtask/src/main.rs:96,:949,:1360`, `check_ship_gate_completeness.rs:20`, `tests/coverage-matrix.yaml`, `discipline.yml` `needs:` `:3510+`, `gate-registry.toml`) retired after invariant-lock review; `check-cna-registration` is 15-3's; xtask net ≤0 shown as `retired − added` in the story."
13. **20-4 AC1** — OLD "`MAOS_DEFAULT_PROVIDER=gemini MAOS_INFERENCE_MODE=replay maos shell`" → NEW either "(a) depends on `provider-model-env-overrides` (hotfix lane) — add the edge" or "(b) `maos shell` selects the provider from the manifest `[providers].primary` (`manifest.rs:2103`)"; one of the two, stated.
14. **20-4 AC3** — OLD "`xtask/model-currency.toml` governs the Gemini IDs" → NEW "depends on `model-pin-currency-gate-and-retired-pin-sweep` (creates `xtask/model-currency.toml` + `check-model-currency`); until it lands, the Gemini model id is a literal in `maos-providers/src/gemini.rs` with a `// model-currency` marker."
15. **20-4 AC2** — add "enforced at provider construction / `InferencePortAdapter` in `maos-bin` (`main.rs:3081-3100`; endpoints are literals today) — `sha256(endpoint)` vs `provider_endpoint_pin` (`manifest.rs:2103-2171`); refusal type out-of-kernel; kernel-Δ 0."
16. **20-5 AC1** — OLD "`git tag v1.0.0-rc.1 && git push --tags` produces signed artifacts including the `.deb` and the air-gap variant; `STABILITY.md` shows the rc row" → NEW "workspace `version = \"1.0.0-rc.1\"` (`Cargo.toml:64`), `STABILITY.md` regenerated (`stability_matrix.rs:137`; the LTS placeholder text at `:367` no longer says Epic 10), `release.yml` `prerelease: true` for `-rc` tags, `container.yml` skipped unless its secrets exist; the tag itself is cut by an `ops-*` row after `ops-provisioning…` records the two secrets — hermetic proof is the discipline `release-dry-run` (15-4) containing `.deb` + air-gap artifacts."
17. **Dependencies** — OLD "Epic 19 (the J1 demo is what a cohort installs). 20-1 before 20-5." → NEW "Epic 19; specifically 15-3 (`check-exit-commands`, `maos --help/--version`), 15-4 (artifact set incl. `maos-registry-server`), 15-6 (`MAOS_INFERENCE_MODE`), 16-4 (`maos uninstall`), 17-3b (WASM admission); hotfix-lane rows `provider-model-env-overrides` and `model-pin-currency-gate-and-retired-pin-sweep` before 20-4. Order: 20-1 → 20-2 → 20-3 → 20-5; 20-4 parallel."
18. **Header** — OLD "5–7 weeks (measured)" → NEW "5–8.5 weeks (§G of the R2 note; 20-3 is 2–4 weeks and gates the rc)".

### CONFIRMED TRUE (do not re-check)

- `publish.rs:267-272` non-stub guard; `--tier`/`--registry-uri`/`--manifest`/`--artifact` flags; default URI `http://127.0.0.1:6789/mcp` (`publish.rs:288`).
- `SpiritRegistryServer` is a real std::net JSON-RPC server dispatching `registry.search|manifest|artifact|publish|deprecate|yanks_since` (`server.rs:230-260`); `LocalFsRegistryStorage` root `~/.local/share/maos/registry/`.
- `McpSpiritRegistryClient` (`client.rs:38-50`) and its `yanks_since`; `YankObserver`/`TlYankObserver` (`main.rs:269-339`); `dispatch_import` admission (`subcommands.rs:942-1090`).
- `issue_attestation` `attestation.rs:150`; keyring revoke/enroll primitives `keyring.rs:117,:157,:288,:334`; all callers test-only.
- `maosctl install --from-local <dir> --verify-only` semantics (`cli.rs:336-363`, `subcommands.rs:1235-1316`); remote fetch removed at v0.5 (`:1209-1210`).
- `maosctl spirit` = `HotSwapPrecheck|Upgrade|Inspect` (`cli.rs:803-870`); no `install`, no `publish`.
- `maos shell` (`main.rs:1730-1737`); `uninstall` is a one-shot lifecycle mode (`:5148-5170`).
- `air-gap` feature (`maos-bin/Cargo.toml:15,:29`) **compiles at HEAD**; air-gap main handles `init|run|backup|audit|install|--version` (`main.rs:1318-1342`).
- `check-air-gap` (`check_air_gap.rs:12-27,:46-60`) exists, unenrolled; `check-mock-not-in-release` enrolled (`release.yml:46`, `discipline.yml:294-309`).
- `Dockerfile:27` single-stage cargo build, no ARG; `container.yml` scaffold; packaging placeholders as cited in §A #24; `packaging/deb` is debhelper.
- `check_rto_gate.rs:57-72` skipped→0; `check_third_party_trial.rs:300-303` <12 FAIL; `check_fuzz_floor.rs:76-85` advisory; the 19-gate list in §A #25 with lines; `EvidenceState` in 4 gates; `BindingClass` in 16 gate modules; 75 registry gates; 40 `EXPECTED_GATES`; 123 aggregate `needs:`; `mode: warning` on the matrix.
- Ledger branches absent; workflows `fuzz-cadence.yml:25-29`, `rpo-rto-cadence.yml:17-18` expect them; weekly cadence note `fuzz-cadence.yml:12-17`.
- `provider_endpoint_pin` parse-only (`manifest.rs:2103-2171`); providers = anthropic/openai/ollama; `gemini-rs` denied (`fr47-vendor-sdk-denylist.toml:17`); provider literals at `main.rs:3085-3100`.
- `MAOS_DEFAULT_PROVIDER`, `xtask/model-currency.toml`, `check-exit-commands`, `MAOS_INFERENCE_MODE` all absent at HEAD.
- `stability_matrix.rs` version source and `lts_clock_start` exact-tag logic (`:352-367`); `Cargo.toml:64 0.1.0-alpha`; `docs/release/` three files; `release.yml` triggers/sign/secrets lines; ops rows `sprint-status.yaml:281-285`.
- kloc @HEAD table in §E (exit 1: aggregate + maos-domain red); 15-2 grants (`epic-15…md:52`): maos-bin +1200, maos-cli +400, maos-registry +300, maos-spirit-cli +300, maos-providers +400, maos-audit +100, kernel-core +150, xtask net ≤0.
