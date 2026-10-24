---
story_id: '18.0'
story_key: 18-0-green-main-ci-baseline
created: 2026-10-24
context_commit: 1fec4882
baseline_commit: 1fec4882
security_sensitive: true
kernel_grant: "NONE. kernel-Δ 0 — no file under `crates/maos-kernel-core/src` is touched (`t3_terminal_helpers_17_7.rs` is a kernel-core *test*). Pin 26361 physical / 20594 tokei unchanged (`xtask/kernel-core-baseline.toml:505`, `xtask/kloc.toml:264`). The only kernel-touching option on the table (D-18-0-12 B) is NOT recommended and would need FLAG-Winston."
kloc_grant: "Every charged crate 18-0 may touch is at ZERO headroom at 1fec4882 (maos-bin 27804, maos-shell 606, maos-a2a-core 4933, maos-cohort 5873, maos-capability 1347, xtask 46363; maos-a2a-tcp 1442 / 1621) → exact requests MEASURED AT LANDING after `cargo fmt --all` (RECOVERY-LANE CEILING RULE `xtask/kloc.toml:60-70,86-95`); `_aggregate_hardfail` = measured + 1 (181558 / 181559 today). Evidence `18-0-evidence/creation/governance-1fec4882.txt`. Test files are uncharged."
review: "bmad-code-review under `docs/testing/proof-contract.md` (focused suites + enrolled gates; workspace once in CI). Reviewer model `anthropic/claude-opus-5-5` (`proof-contract.md:54-57`). AC7 is security-relevant: a review on `anthropic/claude-fable-5-1` needs the operator's permission first."
---

# Story 18.0: Green-main CI baseline

Status: ready-for-dev

<!-- Created 2026-10-24 at 1fec4882 by bmad-create-story under the lighter creation contract (no party-mode preflight). Every open decision is D-18-0-xx, status PROPOSED, with options and a recommendation. `ready-for-dev` is set ONLY by the integrator after the operator rules; the tracker row stays `backlog` until then. Creation ran cargo as the host's only cargo process (one target dir, /mnt/build/cargo); every prototype was reverted and is recorded under 18-0-evidence/creation/. -->

> **The capability:** *`main` is green again, and stays honest: a red `aggregate` means something new broke — not one of nine known reds — because every known red is either fixed at its cause or ruled.*

**Closes:** E17-A1 (`epic-17-retro-2026-10-24.md:108`; operator rulings R1–R4, `:81-102`) — one full green `discipline` run on `main`; `deferred-work.md:1173` (WIT comment), `:1174` (recall-runner BrokenPipe), `:1178` (rupture-id collision, security), `:1184` (cargo-deny), `:1185` (`MAOS_IDLE_FAST`), `:1186` (guest prefetch — closure record only), `:1187` (hook-pump flake), `:1197` (`expensive-tests` shape), `:1198` (budget calibration + blocking), `:1199` (17-7 lease flake), `:1208` (pidfd ESRCH flake); E16-A5 (`epic-16-retro-2026-09-20.md:136,148`, partially — see D-18-0-12); E17-A2's measurement (`epic-17-retro-2026-10-24.md:109`, AC10). Epic contract: `epic-18-spirits-that-think-w3.md` §18-0 (`:100-118`, corrected in place at creation).

**Does NOT close, and says so:**
- The 5 s orphan bounds of `root_shutdown_unload_16_3` (`ac4_v4` and the cohort vectors) — calibration stays with `20-7-stable-runner-perf-gates` (`deferred-work.md:1158-1163,1209`); 18-0 only rules how AC1 treats a red one (D-18-0-01).
- Migrating the Linux secrets backend off the unmaintained zbus 5 stack — no safe upgrade exists at 1fec4882 (D-18-0-03).
- The per-poll `revocation poller: poll_once failed … No such file or directory` line on a fresh home — the poller that prints it is kernel code (`crates/maos-kernel-core/src/revocation/poller.rs:47-48`); creating the directory does not stop it (D-18-0-12).
- Build-once artifact reuse in CI — 18-0 only *prices* it (AC10); the E18 retrospective rules on a story.
- `deferred-work.md:1169-1171` (→ 21-7), `:1193,1204` (→ 20-9) (epic `:118`).

## Story

As **the team that ships Epic 18 on top of `main`**,
I want **every pre-existing red of the `discipline` workflow fixed at its cause — dependency advisories upgraded, the env registry complete, four wall-clock flakes turned into barriers, the test budget calibrated and blocking, the expensive-tests job shaped, the rupture-id collision closed — and the one remaining podman timing vector ruled**,
so that **the first green `aggregate` since 17-9 exists, and the next red one is news.**

**Contract:** epic-18 §18-0 (AC1–AC9, `_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md:100-118`, corrected in place at creation — §Current-tree corrections) and §R (`:44-81`); operator rulings R1–R4 (`epic-17-retro-2026-10-24.md:81-102`); E17-A2 (`:109`); confidence rules 9–11 (`epic-18…md:38-40`); `docs/testing/proof-contract.md` (binding).

## Current-tree corrections — read before implementing

Measured at `1fec4882` by four read-only scouts (flakes; security/init/env; CI shape; WIT re-issue), direct reads, the failed-job logs of discipline run `40080259088` (`69224639`, the last main run), cargo-deny 0.19.4 (the CI pin, `discipline.yml:30`) against advisory-db `21d909e4` (2026-10-24), and throwaway prototypes (all reverted; `18-0-evidence/creation/`).

| Premise (epic / deferred row / tracker) | Fact at `1fec4882` and consequence |
|---|---|
| `wasmtime 48.0.6` carries six advisories RUSTSEC-2026-0370/0371/0372/0378/0379/0381 (`epic…md:110`; `deferred-work.md:1184`; tracker `sprint-status.yaml:276`) | **Only 0378 and 0379 are wasmtime** (fix `>=48.0.7,<49`). 0370/0371/0372 are `zvariant`/`zbus_names`/`zvariant_derive` **unmaintained** (no safe upgrade; path `keyring 4.2.0 → zbus-secret-service-keyring-store 1.0.1 → secret-service 5.2.0 → zbus 5.19.0`, `crates/maos-secrets/Cargo.toml:27-28`); 0381 is `webpki-roots 1.0.7` (fix ≥ 1.0.10). Two more were published after filing: **RUSTSEC-2026-0385** `ctutils 0.4.2` (≥ 0.4.4, via `digest 0.11`) and **RUSTSEC-2026-0386** `tokio 1.52.3` *unsound* (`>=1.51.6,<1.52` or `>=1.53.3`); `chacha20 0.10.0` is yanked (warning only). CI's log of the same job shows the identical nine errors (`ci-run-40080259088-reds.txt`). → D-18-0-02, D-18-0-03. |
| The `reproducible-build` job is AC2's witness (`epic…md:110`) | It is **ADVISORY**: `continue-on-error: true` with owner Winston and a byte-identity reason (`discipline.yml:60-75`). In run 40080259088 `aggregate`'s `needs` JSON reports it `result: success` while `cargo deny check` (`:165-166`) failed — the supply-chain gate never reaches `aggregate`, and rule 11(b) forbids citing an advisory job as proof. → D-18-0-04. |
| The wasmtime upgrade forces the four-component re-issue (`epic…md:110`; `deferred-work.md:1173,1184`) | **It does not.** The runner compiles component bytes with `Component::new` (`crates/maos-wasm-host/src/runner.rs:316`); no `.cwasm`/precompile path exists; provenance inputs are guest sources + `wit/spirit.wit` only (`tests/fixtures/wasm/equiv-fixtures.provenance.toml:25-65`). Prototype: 48.0.7 compiles `maos-wasm-host` with **zero source change**, suite **113/113** (1 skipped), `cargo check --workspace --all-targets` clean (`prototype-upgrade-reverted.txt`). The WIT comment re-issue is a separate, cheap choice: a `//` edit leaves all four binaries **byte-identical**; only the four `source_sha256` pins move (`wit-comment-reissue-probe.txt`). → D-18-0-06. |
| — (not in the epic) | The native-twin guest locks its own wasmtime **48.0.5** (`crates/maos-wasm-host/guests/equiv-fixture/native-twin/Cargo.lock:1901-1904,2137-2140`) and is built `--locked` against the host crate (`xtask/src/build_test_fixtures.rs:195-212`); raising the host's `48.0.4` floor (`crates/maos-wasm-host/Cargo.toml:48-49`) breaks that build unless its lock moves in the same commit. → D-18-0-05. |
| AC4(a): the hook-pump flake counts windows from `Instant::now()` after `start` (`epic…md:112`; `deferred-work.md:1187`) | **Root cause measured:** every `World::new` calls `init_monotonic_base()` (`crates/maos-bin/tests/hook_pump_17_4.rs:58-60`), which re-stores `MONOTONIC_BASE` while `BOOT_INSTANT` keeps the first value (`crates/maos-capability/src/cap_tokens/mod.rs:52-59`), so `monotonic_now_ns()` (`:63-74`) jumps **forward** whenever a later-starting sibling test builds its World — in one libtest process only (the loom leg runs `cargo test -p maos-bin`, `xtask/src/check_multi_tenant_loom.rs:1859-1862`; nextest is process-per-test). The watchdog reads that clock (`crates/maos-bin/src/hook_pump.rs:729,741,758`). HEAD `--test-threads=4`/`=2`: **9/10 red each**, also `fresh_inbound_frames_hold_consolidation_off` (`:755`); a one-line `Once` guard: **0/10 each** (`hook-pump-root-cause.txt`). → D-18-0-07. |
| AC4(c): `send` at `:73-75`; the racing write is `hostile` `:269-272`, test `:134` (`deferred-work.md:1174`) | `send` unwraps at `crates/maos-wasm-host/tests/recall_runner_17_3d.rs:74`; `hostile` is `:198-219`, the racing "queue another frame" write is `:211`; the latch tests are `:220-235`. The runner exits after the latching reply (`crates/maos-wasm-host/src/runner.rs:397-430`), so EPIPE on `:211` is the latch, not a harness fault. |
| AC1 note (i): `t3_terminal_helpers_17_7.rs:478` unwraps an ESRCH (`epic…md:109`) | `:478` unwraps **`pidfd_exit_status`** (the `PIDFD_GET_INFO` ioctl, `crates/maos-kernel-core/src/security/sandbox/linux.rs:582-597`), not `pidfd_open` (`:494,500`). The owner shell reaps the grandchild (`… & echo $!; wait`, `:491,499`) while the test polls — the ioctl can return ESRCH in the reap window. Production already treats any non-EINTR error as "no pidfd evidence" (`crates/maos-kernel-core/src/security/sandbox/t3/child.rs:178-183`); only the test panics. → D-18-0-08. |
| AC1 note (ii): `ac4_v4` reds `workspace-test-suite` (`epic…md:109`; `deferred-work.md:1209`) | `:348` is inside `wait_pid_gone` (`crates/maos-bin/tests/root_shutdown_unload_16_3.rs:342-354`), called through `wait_worker_stopped(…, 5 s)` at `:1452`. The test also runs release/serial in `nfr-rel-11` (`discipline.yml:1688-1689`, recount `:1703-1708`) and in the loom whole-package leg; nextest's only `serial` group does not list it (`.config/nextest.toml:27-46`). Σ for the binary jumped to 22.9 s in the red run (7.1–10.0 s otherwise; `test-budget-calibration.txt`). → D-18-0-01. |
| AC5: `check-test-budget` is advisory at v1.0 and flips to blocking (`epic…md:113`) | In CI it **already blocks**: no `continue-on-error` (`discipline.yml:566-586`), the gate returns `Err` → exit 1 (`xtask/src/check_test_budget.rs:236-284`), and it reds `aggregate` (run 40080259088). The flip is registry truth: `xtask/gate-registry.toml:435-438` (`v1_0 = "advisory"`, cite was `:436-437`), its comment `:429-434`, the job comment `discipline.yml:559-565` and `docs/testing/proof-contract.md:66-67` all say advisory. The 4 rows + 800 s pass all **five** post-20-12 junits (incl. 40080259088: Σ 579.881 s); lowering one row reds exactly one finding (`test-budget-calibration.txt`). |
| AC6: `v1-0-ship-gate.needs` at `:5331-5334`; the job must build fixtures (`epic…md:114`) | The needs list is `discipline.yml:5334-5387` (no `expensive-tests`; `aggregate` already needs it, `:5481-5615`). The three `expensive-tests` rows (`xtask/test-budgets.toml:29-45`) use no fixture (`grep -c 'fixtures::\|fixture(' xtask/tests/abi_diff_integration.rs xtask/tests/diagnostics_20_3a.rs crates/maos-spirit-sdk/tests/spirit_macro_errors.rs` = 0 each). The proven-red steps `discipline.yml:556-557` and `:585-586` have no `if:` and are skipped after a red gate step. Enrolling an unregistered job in `v1-0-ship-gate` contradicts 15-4's D-15-4-F convention (`xtask/tests/release_workflow_15_4.rs:344-349`). → D-18-0-09, D-18-0-10, D-18-0-13. |
| AC7: `alloc_id` "restarts at 0" (`epic…md:115`; `deferred-work.md:1178`) | It restarts at **1** (`AtomicU64::new(1)`, `crates/maos-a2a-core/src/router.rs:219`); mint `:431-439`. The `Duplicate` verdict is `crates/maos-cohort/src/digest.rs:364-385` (row written with `Some(frame.frame_id)`, `:366`); the deny path then answers **`CODE_INTERNAL`** instead of `CODE_INTENT_DENIED` (`router.rs:1572-1576`, `:1646-1650`) — fail-loud on the wire, but the grant-scoped withhold rows roll back with the base row (`crates/maos-iac/src/adapter/transparency_log.rs:1219-1266`). The router has no boot nonce; its TCP caller does (`crates/maos-a2a-tcp/src/transport.rs:337-340,389-391`; daemon `crates/maos-bin/src/main.rs:10966-10985`). No reader depends on the `frame_id[0..8]` prefix (grep; `rupture_id` is documented as a ULID, `crates/maos-domain/src/frame.rs:359-361`). → D-18-0-11. |
| AC8: `maos init` (`main.rs:1684`) creates the CRL dir, whose `latest.signed.json` the poller reads | `main.rs:1684-1690` only dispatches; the directories are created in `crates/maos-shell/src/lib.rs:64-67`, before the `already initialized` early return (`:76-94`). E16-A5's symptom is the poller printing **every failed poll** (`poller.rs:47-48`, 1 s cadence) — and with the directory present the file is still absent, so `LocalFileRegistryClient::fetch_signed_crl` still errs (`crates/maos-domain/src/revocation.rs:490-496`): **the directory alone does not silence it.** The existing init test sets only `MAOS_HOME` (`crates/maos-bin/tests/shell_8_14a.rs:27-61`), so a new mkdir would land in the runner's real `~/.local/share` unless the test isolates `HOME`/`XDG_DATA_HOME` (`revocation.rs:424-452`). → D-18-0-12. |
| AC3: register `MAOS_IDLE_FAST` | Holds. Template `MAOS_TEST_BOOT_NONCE` (`HarnessOnly`, `crates/maos-bin/src/env_contract.rs:129-133`). Presence — even empty or `"0"` — enables fast mode (`hook_pump.rs:649-664`). The kernel's IdleWatchdog reads the same name (`crates/maos-kernel-core/src/scheduler/idle_watchdog.rs:46-49`) outside the gate's `maos-bin/src` scope (21-4 owns the workspace surface, `epic-21…md:85`). Prototype: `check-env-contract` FAIL → PASS (98 vars). |
| AC9: guest prefetch closed by 20-12 | Holds (`deferred-work.md:1186`); run 40080259088's `workspace-test-suite` failed only on `ac4_v4` and the pidfd test (`ci-run-40080259088-reds.txt`). |

## Decisions — RULED 2026-10-24 by the operator (Lunarpulse): every row as recommended (D-18-0-01…10, 12…14 = A; D-18-0-11 = C, a fresh ULID per denial), with one owner change

**Ruling record.** The table below is the proposal as written at creation; its Status column is superseded by this paragraph. **D-18-0-03:** the `deferred-work.md` row for the three ignored zbus-stack advisories is owned by **`20-5-v1-0-rc-tag-and-lts-rule`** (the rc release precondition re-checks the `deny.toml` ignore list), not 20-13 as proposed. **D-18-0-12:** A — 18-0 creates the CRL directory only and stays kernel-Δ 0; the per-poll noise is labelled open with owner **`20-1-registry-client-install-verb-vetter-and-yank`**. Code review of this story runs on `anthropic/claude-opus-5-5` (proof-contract reviewer-model rule); a fable-5.1 security pass on AC7 happens only with the operator's permission.

Options list the recommendation first. Evidence is cited; prototypes are in `18-0-evidence/creation/`. CR-n are creation defaults the operator may overrule.

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-18-0-01** | `ac4_v4` (and the `nfr-rel-11` cohort vectors) red on AC1's landing run | **A** — AC1's re-run allowance covers exactly these 20-7-owned 5 s orphan vectors **in any job** (`nfr-rel-11-halt-receipt-999pct`, `workspace-test-suite`, the loom leg): one **full** workflow re-run, never "re-run failed jobs" (`deferred-work.md:1165`); both run ids and the failing test names recorded; the same vector red twice, or any other red, stops the close. **B** — add `binary_id(maos-bin::root_shutdown_unload_16_3)` to nextest's `serial` group (`.config/nextest.toml:30-46`). **C** — skip `ac4_v4` in `workspace-test-suite` (it still runs in `nfr-rel-11` and the loom leg). **D** — widen the 5 s bound. | **A.** The bound is 20-7's calibration (ADR-068; `deferred-work.md:1158-1163`); it failed even in a serial run (38974347270), so B buys little and hides the evidence 20-7 needs; C drops the debug-profile coverage; D is forbidden (no wall-clock widening, epic `:112`). A keeps the vector honest and bounds the cost at one re-run. | PROPOSED |
| **D-18-0-02** | Which versions clear the advisories | **A** — the measured set: patch releases take the latest of the locked minor, a minor bump takes the lowest minor that clears: `wasmtime`/`wasmtime-wasi` **48.0.7** (+31 family crates), `rustls-webpki` **0.103.16**, `webpki-roots` **1.0.11**, `ctutils` **0.4.5**, `chacha20` **0.10.2**, `tokio` **1.53.3** (`cargo update -p tokio --precise 1.53.3`). **B** — latest compatible everywhere (tokio 1.54.1). **C** — lowest clearing everywhere (webpki-roots 1.0.10, ctutils 0.4.4). | **A.** Measured: `cargo deny check` leaves only the three zbus `unmaintained` IDs; `cargo check --workspace --all-targets --locked` 0 errors; `maos-wasm-host` 113/113 on 48.0.7 (`prototype-upgrade-reverted.txt`). 48.0.7 is both the lowest and the latest 48.x (the LTS line, `crates/maos-wasm-host/Cargo.toml:45-47`; 49.0.4+/50.0.0 are majors). tokio's minor is the one bump with timing exposure in a timing-sensitive suite — take the smallest. | PROPOSED |
| **D-18-0-03** | RUSTSEC-2026-0370/0371/0372 (`zvariant`, `zbus_names`, `zvariant_derive` unmaintained; "No safe upgrade is available!") | **A** — add the three IDs to `deny.toml` `[advisories] ignore` under the file's own convention for `unmaintained`-class, no-safe-upgrade transitive crates (`deny.toml:15-41`), each with a reason naming the path and the trigger (a `secret-service`/`keyring` release on `zbus ≥ 6`), plus a `deferred-work.md` row: owner **`20-13-pluggable-crypto-provider-selection`** (the next story that opens `crates/maos-secrets`, rule 10(b)) [INFERENCE on charter fit — the operator may name another key]. **B** — move the Linux keyring backend off Secret Service (e.g. `keyring`'s kernel-keyutils store) — changes where NFR-Sec-16 credentials persist; a product change outside 18-0's charter. **C** — leave `cargo deny check` red. | **A.** R2 ("upgrade, not ignore") was ruled on the premise that all six IDs were wasmtime's with a patched line; for these three no upgrade exists — `secret-service` 5.2.0 (2026-08-29) and `keyring` 4.2.0 are the latest and still on zbus 5 while zbus 6.0.0 shipped 2026-10-16 (crates.io API, 2026-10-24). They are *unmaintained* notices, not vulnerabilities. Prototype: `advisories ok, bans ok, licenses ok, sources ok` (`prototype-upgrade-reverted.txt`). | PROPOSED |
| **D-18-0-04** | `cargo deny check` runs inside an ADVISORY job | **A** — move the `Install cargo-deny` + `cargo deny check` steps (`discipline.yml:156-166`) into a new blocking job `cargo-deny` (one image, no `continue-on-error`), listed in `aggregate.needs` only — no `gate-registry.toml` row (15-3's enrolment precedent, `sprint-status.yaml:237`), so `check-ship-gate-completeness`'s 46 expected gates are unchanged (`xtask/src/check_ship_gate_completeness.rs:553-571`); `reproducible-build` keeps its byte-identity steps and its ADVISORY label. **B** — keep it; AC2's witness is the step conclusion read from the jobs API, labelled advisory (never proof, rule 11(b)). **C** — drop `continue-on-error` from `reproducible-build` (makes byte identity blocking against its owner's reason). | **A.** Rule 11(d): 18-0 touches this control, so it comes under rule 11; (b) bars an advisory job as proof. The command is unchanged — its red is already recorded in CI (job 120832151944) and locally (`cargo-deny-0.19.4-head-1fec4882.log`), so no new plant (rule 11(c) "seen in CI with the run id recorded"). | PROPOSED |
| **D-18-0-05** | The host's wasmtime floor and the native-twin lock | **A** — raise `crates/maos-wasm-host/Cargo.toml:48-49` to `48.0.7` (comment `:45-47` gains the 2026-10-24 advisory line) **and** in the same commit `cargo update --manifest-path crates/maos-wasm-host/guests/equiv-fixture/native-twin/Cargo.toml -p wasmtime -p wasmtime-wasi` (dry run: 33 wasmtime-family packages 48.0.5 → 48.0.7, nothing else; `governance-1fec4882.txt`). **B** — lock-only: root `Cargo.lock` moves, `Cargo.toml` stays at `48.0.4`, the twin keeps 48.0.5. | **A.** The floor documents why (the file's own precedent, `:45-47`: "48.0.4 is the first fixed 48.x"); a `--locked` fixture build that silently compiles the advisory-affected 48.0.5 is the drift `cargo deny` cannot see (it graphs the root workspace only, `deny.toml:4-10`). `build-test-fixtures` then rebuilds the twin (`build_test_fixtures.rs:195-212`). | PROPOSED |
| **D-18-0-06** | `wit/spirit.wit:1` "emitter-scoped" comment and the four-component re-issue | **A** — do it in 18-0: line 1 → "lossless frames and participant-scoped log recall"; rebuild the four components with the pinned toolchain (rustc 1.98.1, wit-bindgen 0.44.0, `wasm32-wasip2`; `equiv-fixtures.provenance.toml:14-19`), confirm the four `wasm_sha256` are unchanged, re-pin the four `source_sha256` (the 17-3b manual recipe, `17-3b-wasm-form-admission-and-contract.md:89,463-465`); `check-equiv-fixture-provenance` PASSED; close `deferred-work.md:1173`. **B** — leave the comment; re-route `:1173` to the next story that edits `wit/spirit.wit` (rule 10(b)). | **A.** Measured cost is four hashes: with the corrected line the four rebuilt binaries are byte-identical to the committed blobs and the gate reds only on the four source pins (`wit-comment-reissue-probe.txt`, new hashes listed for that exact wording). R2's intent — fix the false comment once — survives the disproved "forced by wasmtime" premise. The WIT header's "needs a ratified ABI-extension entry" (`wit/spirit.wit:18-20`) is not mechanically required: `check-abi-ratification` is driven by `abi-diff`, which reads no WIT (`xtask/src/check_abi_ratification.rs:3-5`) and a `//` comment changes no WIT package. | PROPOSED |
| **D-18-0-07** | AC4(a) fix locus for the hook-pump flake | **A** — test-only: `World::new` guards the clock init with a file-level `std::sync::Once` (`hook_pump_17_4.rs:58-60`), and the window assertion becomes a barrier on the production clock (CR-1). **B** — fix at origin: `init_monotonic_base` sets the base once (its doc already promises idempotence, `crates/maos-capability/src/cap_tokens/mod.rs:50-51`), then A's assertion; production behaviour change in a capability-clock crate (1347/1347, ≈ 22 call sites in `maos-bin/src/main.rs`, 73 test files), measured request. | **A.** Measured: the guard alone takes HEAD's 9/10 red to 0/10 at `--test-threads=4` and `=2` (`hook-pump-root-cause.txt`); it is kernel- and capability-neutral. B is the better long-term clock but widens 18-0 into a token-TTL clock change; record it in the Dev Notes as an observed origin defect (no deferral owed — nothing is broken in production by it [INFERENCE: each `main.rs` arm initialises once]). | PROPOSED |
| **D-18-0-08** | AC4(d) pidfd ESRCH barrier | **A** — reap acknowledgement: each owner script reaps its grandchild and then prints a second line (`… & p=$!; echo $p; wait $p; echo reaped`), and the test reads `reaped` before ONE `pidfd_exit_status` call, asserting `Some(0x8900)` / `Some(9)` — any error, ESRCH included, is a failure. **B** — keep polling but treat ESRCH as "not yet" inside the bounded poll (mirrors production's tolerance, `t3/child.rs:178-183`). | **A.** It removes the race instead of tolerating it, and keeps ESRCH a real failure after the barrier. Linux ≥ 6.15 stores `PIDFD_INFO_EXIT` data while the task is released, before the reaper's `wait` returns [INFERENCE: kernel `pidfs_exit` from `release_task`; the dev confirms by the repeat run in AC4]. If A shows ESRCH after `reaped`, fall back to B and record why. | PROPOSED |
| **D-18-0-09** | Does `expensive-tests` build fixtures (`epic…md:114`) | **A** — no fixture build until a registered `job = "expensive-tests"` row needs one: none of today's three does (grep = 0); a fixture-needing test without a build fails closed and names `build-test-fixtures` (`docs/testing/proof-contract.md:77-80`). **B** — install `wasm32-wasip2` and run `cargo run -p xtask -- build-test-fixtures` in the job now (the `workspace-test-suite` pattern, `discipline.yml:5095-5099,5155-5158`). | **A.** B pays a fixture build on every push for zero consumers (the `wasm-host-tests` fixture step alone took 692 s in run 40080259088, `e17-a2-method-probe.txt`) while 18-0's own AC10 prices CI as compile-bound. A's failure mode is loud, not silent. | PROPOSED |
| **D-18-0-10** | Does `expensive-tests` join `v1-0-ship-gate.needs` (`epic…md:114`; `deferred-work.md:1197`) | **A** — no: it stays in `aggregate.needs` only (already there, `discipline.yml:5481-5615`) under the D-15-4-F convention that an **unregistered** job is enrolled in `aggregate.needs` and nowhere else, because `check-ship-gate-completeness` iterates `EXPECTED_GATES`, never `needs`, so a ship gate blocking on an unregistered job is unpoliced (`xtask/tests/release_workflow_15_4.rs:344-349`; `xtask/src/check_ship_gate_completeness.rs:17,202,239`); the ship gate already blocks on `check-test-cost`, which polices the expensive register (`discipline.yml:5334-5387`). **B** — register it: a `[[ship_gate]] name = "expensive-tests"` row, an `EXPECTED_GATES` entry and the count test `expected_gates_and_registry_are_one_set_of_46` → 47 (`check_ship_gate_completeness.rs:553-571`; `xtask/tests/ship_gate_binding_20_3a.rs:74-167`), then `v1-0-ship-gate.needs`. **C** — add it to `v1-0-ship-gate.needs` unregistered (the epic's literal text). | **A.** C breaks the convention 15-4 wrote down for exactly this case; B is the honest way to make it a ship gate but adds a registered gate (and its disposition debate) to a CI-baseline story. A keeps `main` green-honest; the epic clause and the deferred row's "not in `v1-0-ship-gate.needs`" item are corrected in place. If B is ruled, the dev also adds its disposition row and the 46 → 47 re-count. | PROPOSED |
| **D-18-0-11** | AC7 rupture-id mint (R4, security) | **C** — a fresh ULID per denial at the router (`router.rs:434-439` → `ulid::Ulid::new()` bytes; add `ulid = "1.1"`, already in the lock via `crates/maos-iac/Cargo.toml:21`, so no new crate): payload, emitted frame and TL row keep one id; no peer-controlled bits; ~80 random bits per millisecond. **B** — fold the boot nonce into the low half (router gains `own_boot_nonce` from `transport.rs:389-391`; hash/XOR with the counter): keeps the `frame_id[0..8]` prefix, adds wiring in `maos-a2a-tcp`, 64-bit birthday space. **A** — journal the row with `None` (TL mints its own ULID, `transparency_log.rs:881-891,1153-1189`): stored id diverges from the payload and the frame the peer receives, and the router's correlation id still collides. | **C.** Smallest blast radius (one file + one dependency line, `maos-a2a-core` at zero headroom → measured), keeps id equality end-to-end, matches the domain's own ULID contract (`frame.rs:359-361`), and removes the granter's steering of half the id (`deferred-work.md:1178`). No reader depends on the prefix (grep at creation; the dev re-greps). The second mint, peer-identity refusal `ts ‖ alloc_id()` (`router.rs:505-540`), already varies across boots by its timestamp half and stays. Plant: restore `frame_id[0..8] ‖ alloc_id()`. | PROPOSED |
| **D-18-0-12** | AC8 / E16-A5: the CRL directory vs the per-poll noise | **A** — `maos init` creates `default_crl_dir()` (the operator's documented drop location; `maos purge` already enumerates it, `crates/maos-bin/src/purge.rs:80-83`), and 18-0 **labels** the noise open: a `deferred-work.md` row with B's design, owner **`20-1-registry-client-install-verb-vetter-and-yank`** (registry-client/revocation charter, rule 10(b)) [INFERENCE: whether 20-1 opens `revocation/poller.rs` is unmeasured — the operator may name another key]. **B** — also edge-trigger the poller's failure print (first failure + recovery + count, 17-10's CR-2 pattern) in `crates/maos-kernel-core/src/revocation/poller.rs:43-57`: kernel-Δ ≈ +8–12 tokei, **FLAG-Winston**, re-pin chain; ADR-069 room exists (20594 of 23,000). **C** — maos-bin defers spawning the poller until `latest.signed.json` exists, printing one `no CRL published at <path>` line (kernel-Δ 0, but it changes a revocation control's start-up and must re-prove `nfr-rel-9-revocation-5s-p99`). | **A.** 18-0 is the CI critical path and kernel-Δ 0 by design; B is the right fix of the print but belongs in a FLAG-Winston story; C hides a misconfigured `MAOS_CRL_PATH` behind one line forever. The epic's AC8 text (dir creation) is delivered and its false premise (that the dir silences the poller) is corrected in place. | PROPOSED |
| **D-18-0-13** | Proof route (proof contract: one green matrix run + one plant per control AC, on one image) | **A** — every plant is hermetic and local, recorded once with command, output and wall-clock in `18-0-evidence/plants.md`: AC5 (lower one exception → `check-test-budget` against the recorded CI junit of run 40080259088), AC6 (a live-workflow assertion in `xtask/tests/` that both proven-red steps carry `if: ${{ !cancelled() }}`, that `expensive-tests` uploads its junit and that its enrolment matches D-18-0-10 — plant: remove the `if:`), AC7 (restore the counter mint), AC8 (remove the mkdir). AC1's landing run on `main` is the one green matrix. No branch-only workflow. **B** — additionally push a throwaway `ci/18-0-verify` branch (17-10 FW-08 precedent) whose planted budget row reds `check-test-budget` in CI and shows its proven-red step still running (needs the operator's per-story push authorization). | **A.** Each control's red is reproducible from committed inputs without a runner; GitHub's `!cancelled()` step semantics are the same ones `discipline.yml:5245-5251` already relies on. B is the literal reading of the epic's AC6 "one plant shows the proven-red step runs after a red gate step"; choose it only if the operator wants the runtime seen once. | PROPOSED |
| **D-18-0-14** | kloc | Exact measured requests **at landing** (after `cargo fmt --all`, `kloc-check --json`), returned to the operator before the landing commit, pinned with dated notes citing this key; `_aggregate_hardfail` = measured + 1. Forecast [INFERENCE, not a grant]: maos-bin +5 (AC3), maos-shell +1–3 (AC8), maos-a2a-core ±0–3 (AC7 C); xtask 0 (tests only); every other crate 0 unless an alternative option is ruled. | Contract 2026-10-23 (non-kernel crates measured at landing); `xtask/kloc.toml:60-70,86-95`; table in `governance-1fec4882.txt`. | PROPOSED |
| CR-1 | AC4(a) assertion shape | `CountsConsolidations::on_consolidate` also sends `monotonic_now_ns()` on an unbounded channel; the test awaits fires (hang guard 10 s, a guard not a bound) instead of `sleep(650)`, requires ≥ 3 fires, and asserts every adjacent gap ≥ the 100 ms window on the production clock — the "at most once per window" property itself, with no `Instant` arithmetic; the Loaded (`:684-689`) and Paused (`:710-722`) phases keep their joins. `fresh_inbound_frames_hold_consolidation_off` (`:726-768`) needs only the `Once` guard. | — |
| CR-2 | AC4(b) barrier | `supervisor_scenes.rs:745-749`: await the stub's `argv=<kill>` record (written before the stub's `sleep 30`, `stub_scenes.rs:31,37`) with a hang guard of 20 s (inside the stub's 30 s gate, so "termination remains pending" `:750-753` still observes a gated kill); the assertion message names the guard. No 3 s deadline as behaviour. | — |
| CR-3 | AC4(c) EPIPE semantics | `Session::send` returns `io::Result<()>`; every caller `.expect(…)`s it except the post-latch write in `hostile` (`recall_runner_17_3d.rs:211`), which accepts `Ok` or `ErrorKind::BrokenPipe` only. The latch is still proven by the exact report (`:213-216`), the control record, no extra records, exit 6 and `recall protocol desync` (`:90-105`). | — |
| CR-4 | AC5 rows | `seconds = 153 / 81 / 46 / 40` as ruled (R3); `40` is 32.061 × 1.248 (ceil of +25 % would be 41 — the ruled figure stands); each `reason` names the binary's real cost and the calibration source (`20-12-evidence/ci-after.json` + run 40080259088); `owner = "18-0-green-main-ci-baseline"`. | — |
| CR-5 | AC7 test home | New `crates/maos-cohort/tests/rupture_restart_18_0.rs`, harness from `rupture_grant_17_8.rs:25-116` (file-backed `TransparencyLogAdapter`, `install_recipient_resolver`, `LogRecallAdapter`) driving a real `A2ARouterCore` deny exactly as the router's own `intake_policy_deny_emits_consent_rupture` does (`router.rs:2489-2537`: `accept_allowlist` empty, `handle_intake("iac.deliver", …)`) with `CohortRuptureLogSink` installed (`digest.rs:391-405`). | — |
| CR-6 | AC8 test isolation | Extend `crates/maos-bin/tests/shell_8_14a.rs` with a test whose child env sets fresh `HOME`, `MAOS_HOME` and `XDG_DATA_HOME` and removes `MAOS_CRL_PATH`; the existing `init_creates_dot_maos_and_is_idempotent` (`:27-61`) also gains `HOME`/`XDG_DATA_HOME` isolation so no test writes the runner's real `~/.local/share/maos/crl`. | — |
| CR-7 | Deferred-work bookkeeping | Each closed row (`:1173,1174,1178,1184,1185,1187,1197,1198,1199,1208`) gets a `CLOSED by 18-0 (run <id>)` note, `:1186` gets the AC9 run id, `:1209` gets the D-18-0-01 outcome; new rows for D-18-0-03 and D-18-0-12 name their owners. Rows are never deleted. | — |

## Acceptance Criteria

*(Epic §18-0, tightened and corrected; every AC cites the code it changes. Selected options are PROPOSED; if the operator rules otherwise, the integrator rewrites the affected clauses.)*

### AC1 — command: main is green

1. **One full `discipline` run** (`.github/workflows/discipline.yml`) on `main` at 18-0's landing commit ends with `aggregate` (`:5478`; needs `:5481-5615`) **success**, and with it `v1-0-ship-gate` (`:5331`) and `v1-5-ship-gate`. The run id and every job's name/conclusion/start/end (`gh api repos/lunarpulse/maos/actions/runs/<id>/jobs --paginate`) are saved as `18-0-evidence/ci/landing-run.json`.
2. **Every creation-time red is accounted for** (run 40080259088, `18-0-evidence/creation/ci-run-40080259088-reds.txt`): `reproducible-build`/cargo deny → AC2; `check-env-contract` → AC3; `check-multi-tenant-loom` (`hook_pump_17_4.rs:700`) → AC4(a); `workspace-test-suite` (`t3_terminal_helpers_17_7.rs:478`) → AC4(d) and (`root_shutdown_unload_16_3.rs:348`) → AC1.3; `check-test-budget` → AC5; `wasm-host-tests (ubuntu-26.04-arm)` (`recall_runner_17_3d.rs:74`) → AC4(c); `v1-0-ship-gate`, `v1-5-ship-gate`, `aggregate` are fan-ins.
3. **Re-run allowance (D-18-0-01 A):** if the landing run's only reds are 20-7-owned 5 s orphan vectors (`root_shutdown_unload_16_3::ac4_v4` in any job, or `nfr-rel-11-halt-receipt-999pct`'s cohort vectors), ONE **full** workflow re-run is made (never "re-run failed jobs", `deferred-work.md:1165`); both run ids and the red test names are recorded in `landing-run.json` and on `deferred-work.md:1209`. Any other red, or the same vector red on both, stops the close.

### AC2 — `cargo deny check` green (R2)

1. **Upgrades (D-18-0-02 A):** `Cargo.lock` moves `wasmtime`/`wasmtime-wasi` 48.0.6 → 48.0.7 with their family (`:6117-6118`, `:6353-6354`; RUSTSEC-2026-0378/0379), `rustls-webpki` 0.103.15 → 0.103.16 (`:4603-4604`; 0382), `webpki-roots` 1.0.7 → 1.0.11 (`:6451-6452`; 0381), `ctutils` 0.4.2 → 0.4.5 (`:1108-1109`; 0385), `tokio` 1.52.3 → 1.53.3 (`:5290-5291`; 0386), `chacha20` 0.10.0 → 0.10.2 (`:665-666`; yanked). Floor and twin per D-18-0-05.
2. **Unmaintained zbus stack (D-18-0-03 A):** `deny.toml` `ignore` gains RUSTSEC-2026-0370/0371/0372 with the convention's comment block (`deny.toml:15-41`) and the new deferred row.
3. **The gate:** `cargo deny check` (cargo-deny 0.19.4, `discipline.yml:30`) prints `advisories ok, bans ok, licenses ok, sources ok` locally and in AC1's run, in the job D-18-0-04 selects; that job is not `continue-on-error`.
4. **No source change for wasmtime:** `cargo nextest run --locked -p maos-wasm-host` green; `wasm-host-tests` green on all four images in AC1's run; `cargo build --locked --release --manifest-path crates/maos-wasm-host/guests/equiv-fixture/native-twin/Cargo.toml` (via `build-test-fixtures`) succeeds.
5. **WIT comment (D-18-0-06 A):** `wit/spirit.wit:1` reads `… lossless frames and participant-scoped log recall.`; the four components are rebuilt with the pinned toolchain and their `wasm_sha256` stay `36ff3029…`, `0f220aef…`, `77e2bc90…`, `5369a052…` (`equiv-fixtures.provenance.toml:23,33,45,57`); the four `source_sha256` are re-pinned; `cargo run -p xtask -- check-equiv-fixture-provenance` PASSED; `check-wasm-form-equiv` (`discipline.yml:4204`) green in AC1's run. If any `wasm_sha256` moves, it is re-pinned too and the Debug Log says why.
6. **Proof:** existing gate, no new control → no plant; its red at HEAD is recorded (CI job 120832151944; `cargo-deny-0.19.4-head-1fec4882.log`).

### AC3 — env contract

`MAOS_IDLE_FAST` is an `EnvVar { stability: EnvStability::HarnessOnly }` in `crates/maos-bin/src/env_contract.rs` (beside `MAOS_TEST_BOOT_NONCE`, `:129-133`), its `purpose` stating presence semantics (any value, even empty or `"0"`: 40 ms poll, idle windows ÷100 — `hook_pump.rs:649-664`; the kernel IdleWatchdog reads the same name, `idle_watchdog.rs:46-49`). `cargo run -p xtask -- check-env-contract` → `PASS (… 0 violations)`; `check-env-contract` (`discipline.yml:409`) green in AC1's run. Existing gate → no plant (HEAD red recorded, `governance-1fec4882.txt`).

### AC4 — four flakes become barriers, not deadlines

No wall-clock bound is widened; every remaining timeout is a named hang guard, never the behaviour under test.

- **(a) Hook-pump consolidation** (`deferred-work.md:1187`; D-18-0-07, CR-1): with the fix, `cargo test --locked -p maos-bin --test hook_pump_17_4 -- --test-threads=4` and `-- --test-threads=2` pass **10 of 10** each (HEAD: 1 of 10 each, `hook-pump-root-cause.txt`); `check-multi-tenant-loom` green in AC1's run.
- **(b) 17-7 lease** (`deferred-work.md:1199`; CR-2): `binding_revokes_bearer_before_unconfirmed_stop_and_root_retains_lease` (`supervisor_scenes.rs:745-749`) waits on the stub's kill record, not a 3 s deadline; `cargo nextest run --locked -p maos-bin --test worker_terminal_facts_17_7` green.
- **(c) Recall-runner latch** (`deferred-work.md:1174`; CR-3): `recall_runner_17_3d.rs:73-75,211`; `cargo nextest run --locked -p maos-wasm-host --test recall_runner_17_3d` green; `wasm-host-tests` green on all four images in AC1's run (the arm images are where it red).
- **(d) Non-child pidfd** (`deferred-work.md:1208`; D-18-0-08): `t3_terminal_helpers_17_7.rs:469-518` opens, kills/releases and reads exit status behind explicit acknowledgements; `cargo nextest run --locked -p maos-kernel-core --test t3_terminal_helpers_17_7` green, and 20 consecutive runs of that one test green locally (`SKIP:` on a kernel < 6.15 is legal off CI, `:270-285`).
- Flake fixes are test changes, not controls (rule 11 scope) → no plants; each is shown by AC1's green run plus the repeat counts above, recorded in the Dev Agent Record.

### AC5 — test budget re-set and blocking (R3)

1. `xtask/test-budgets.toml` gains four `[[exception]]` rows (CR-4) — `maos-bin::worker_terminal_facts_17_7` 153, `maos-control::post_surface_16_1` 81, `maos-bin::worker_supervision_16_3` 46, `maos-eval::t3_network_escape_executor` 40 — and `total_seconds` 300 → 800 (`:25`).
2. `cargo run -p xtask -- check-test-budget --junit <junit> --json` passes on the five recorded CI junits (runs 39970039438, 39970126328, 39975504943, 39982797947, 40080259088; artifact `workspace-test-suite-junit`) — measured at creation, `test-budget-calibration.txt` — and on AC1's run.
3. The posture becomes true everywhere: `xtask/gate-registry.toml:437` (row `:435-438`) `v1_0 = "blocking"` with the comment `:429-434` rewritten (flip owner 18-0, ruling R3, calibration source); `discipline.yml:559-565`; `docs/testing/proof-contract.md:66-67`. Any xtask test that pins the old disposition is updated in the same commit (grep `check-test-budget` in `xtask/tests` and `xtask/src`).
4. **Proven red (1 plant):** one exception lowered (e.g. 153 → 100) → `check-test-budget` against run 40080259088's junit returns exactly one finding and exit 1 (prototype: `test-budget-calibration.txt`); restored; recorded per D-18-0-13.

### AC6 — `expensive-tests` job shape

1. `expensive-tests` (`discipline.yml:595-618`) uploads `target/nextest/expensive/junit.xml` (`.config/nextest.toml:59-64`) as `expensive-tests-junit` with `if: ${{ !cancelled() }}` (the `:5245-5251` pattern).
2. Enrolment per D-18-0-10: under **A** `expensive-tests` stays in `aggregate.needs` only (`:5481-5615`) and is **not** added to `v1-0-ship-gate.needs` (`:5334-5387`); under **B** it becomes a registered ship gate (registry row, `EXPECTED_GATES`, 46 → 47) and joins `v1-0-ship-gate.needs`.
3. The proven-red steps of `check-test-cost` (`:556-557`) and `check-test-budget` (`:585-586`) carry `if: ${{ !cancelled() }}` so they run after a red gate step.
4. Fixture build per D-18-0-09.
5. **Proven red (1 plant), D-18-0-13 A:** a live-workflow assertion in `xtask/tests/` (parse the real `.github/workflows/discipline.yml`, the `release_workflow_15_4.rs:344-365` precedent) checks 1–3 as ruled; removing one `if:` reds it.

### AC7 — rupture id unique across restarts (R4, security)

1. `crates/maos-a2a-core/src/router.rs:431-439` mints per D-18-0-11 (C: a fresh ULID); the comment at `:431-433` states the new property; the same id is the payload's `rupture_id` (`:448`) and the frame's `frame_id` (`:459`).
2. **Restart regression test** (CR-5): boot 1 (TL file, nonce N1, router R1) receives a granted same-intent frame from the peer, then denies frame F; boot 2 (same TL file, nonce N2, fresh router R2) receives another granted frame from the peer, then denies the **same** F. Asserts: both denials answer `CODE_INTENT_DENIED` (never `CODE_INTERNAL`, `router.rs:1572-1576`); two durable `ConsentRupture` base rows with distinct ids; two `transparency_log_consent_ruptures` rows; the boot-2 frame is `LogRecallError::ConsentWithheld` for the rejected recipient (the D-17-8-03 withhold, `rupture_grant_17_8.rs:134-138`).
3. **Proven red (1 plant):** restoring `frame_id[0..8] ‖ alloc_id()` reds the test at the second denial (`CODE_INTERNAL`, one row). Kernel-Δ 0. Focused set: `cargo nextest run --locked -p maos-a2a-core -p maos-cohort -p maos-a2a-tcp`.

### AC8 — `maos init` creates the CRL directory (E16-A5, partial — D-18-0-12)

1. `run_init` (`crates/maos-shell/src/lib.rs:64-67`) creates `maos_domain::revocation::default_crl_dir()` (`revocation.rs:424-452`) with the other leaves, **before** the idempotent early return (`:76-94`), so re-running `init` on an existing home also creates it.
2. Test (CR-6): fresh `HOME`/`MAOS_HOME`/`XDG_DATA_HOME`, `MAOS_CRL_PATH` removed → after `maos init`, `$XDG_DATA_HOME/maos/crl` is a directory; a second `init` succeeds and keeps it; with `MAOS_CRL_PATH=<dir>` that directory is created instead.
3. **Proven red (1 plant):** removing the mkdir reds the test. The per-poll noise is labelled open with its owner (D-18-0-12; `deferred-work.md` new row; E16-A5 row in `sprint-status.yaml` updated by the integrator).

### AC9 — guest prefetch closure record

`deferred-work.md:1186` records AC1's run id as the first fully green `workspace-test-suite` after 20-12's fix (its four post-20-12 runs are listed there). No code.

### AC10 — compile vs test wall, five largest jobs (E17-A2)

From AC1's landing run logs only (no new CI job): for `wasm-host-tests` (4 legs), `cgroup-ceiling-undelegated` (4), `cgroup-ceiling-delegated` (4), `epic-17-exit` (2) and `workspace-test-suite` (the five largest buckets in run 40080259088 and in `20-12-evidence/ci-after.json`), per leg: job wall, compile seconds (Σ cargo `Finished … in <t>`), setup seconds (checkout/toolchain/cache/install steps) and test seconds (the remainder), method as `e17-a2-method-probe.txt` (probe: one `wasm-host-tests` leg = 1762 s compile of 2221 s). Output `18-0-evidence/ci/compile-vs-test.json` plus a five-line summary in the Completion Notes for the E18 retrospective. No ruling in 18-0.

## Tasks / Subtasks

- [ ] **T0 — Preflight at the dev HEAD** (all ACs)
  - [ ] Confirm every D-18-0-01…14 row is RULED in §Decisions, the selected options are materialized in the ACs, and story + tracker read `ready-for-dev` (integrator); stop if any row is PROPOSED.
  - [ ] `git diff --stat 1fec4882 -- Cargo.lock deny.toml crates/maos-wasm-host crates/maos-a2a-core/src/router.rs crates/maos-cohort crates/maos-shell/src/lib.rs crates/maos-bin/src/env_contract.rs crates/maos-bin/tests crates/maos-kernel-core/tests/t3_terminal_helpers_17_7.rs xtask/test-budgets.toml xtask/gate-registry.toml .github/workflows/discipline.yml .config/nextest.toml wit tests/fixtures/wasm`; any drift → Debug Log, re-anchor.
  - [ ] Re-run cargo-deny 0.19.4 against the day's advisory DB; a new advisory joins AC2 under D-18-0-02's rule (record it), never an ignore without a ruling.
- [ ] **T1 — Dependencies** (AC2.1–2.4; D-18-0-02/03/05) — `cargo update -p wasmtime -p wasmtime-wasi`, `-p rustls-webpki -p webpki-roots@1.0.7 -p ctutils -p chacha20`, `-p tokio --precise 1.53.3`; `crates/maos-wasm-host/Cargo.toml:45-49`; native-twin lock; `deny.toml` ignore block; `cargo deny check` clean.
- [ ] **T2 — CI enrolment of cargo-deny** (AC2.3; D-18-0-04) — new blocking `cargo-deny` job, `aggregate.needs`; `reproducible-build` loses only the two deny steps (its ADVISORY comment stays true).
- [ ] **T3 — WIT comment re-issue** (AC2.5; D-18-0-06) — line 1; rebuild 4 guests (`cargo build --locked --release --target wasm32-wasip2 --manifest-path <guest>/Cargo.toml`, recipe `17-3b…md:89,463-465`); compare `wasm_sha256`; re-pin `source_sha256`; header `:14` dated note in the provenance file; `build-test-fixtures` again before local fixture tests.
- [ ] **T4 — Env contract** (AC3) — one `EnvVar` row.
- [ ] **T5 — Flakes** (AC4; D-18-0-07/08, CR-1…3) — `hook_pump_17_4.rs:58-60,670-768`; `supervisor_scenes.rs:745-753` (+ a small await helper beside `stub_scenes.rs:118-119` if shared); `recall_runner_17_3d.rs:73-81,198-219` and every `send` caller (`:80,139,201,207,211,240,259,279,300`); `t3_terminal_helpers_17_7.rs:286-297,469-518`. Repeat counts into the Dev Agent Record.
- [ ] **T6 — Budget** (AC5; CR-4) — four rows + 800; registry `:435-438` + comment; `discipline.yml:559-565`; `proof-contract.md:66-67`; calibration re-run on the five junits (`gh run download <id> -n workspace-test-suite-junit`); the plant.
- [ ] **T7 — `expensive-tests` shape** (AC6; D-18-0-09/10/13) — junit upload, enrolment as ruled, the two `if:`; the live-workflow assertion test; its plant.
- [ ] **T8 — Rupture id** (AC7; D-18-0-11, CR-5) — `router.rs:431-439` (+ `crates/maos-a2a-core/Cargo.toml` `ulid`); `crates/maos-cohort/tests/rupture_restart_18_0.rs`; the plant; `bans ok` still (one `ulid` version).
- [ ] **T9 — CRL directory** (AC8; D-18-0-12, CR-6) — `maos-shell/src/lib.rs:64-67`; `shell_8_14a.rs`; the plant.
- [ ] **T10 — Records** (AC9, CR-7) — `deferred-work.md` rows; `epic-18…md` §18-0 if anything moved at dev HEAD; `docs/` mentions of the advisory posture of `check-test-budget`.
- [ ] **T11 — Governance** (D-18-0-14) — `cargo fmt --all`; `kloc-check --json`; measured requests to the operator **before** landing; `check-kernel-baseline` 26361 / 95 unchanged; `check-test-cost`; `check-dev-record-completeness`; `cargo nextest run -p xtask --locked --test d11_xtask_ceiling_ratchet --test recovery_lane_ceiling_rule` before the closing commit (`proof-contract.md:40-44`).
- [ ] **T12 — Landing and AC1/AC10** (integrator pushes) — the landing run; D-18-0-01 if a 20-7 vector reds; `landing-run.json`; `compile-vs-test.json`.

### Review Findings

*(Filled by bmad-code-review. Reviewer model `anthropic/claude-opus-5-5`; a `anthropic/claude-fable-5-1` security pass over AC7 only with the operator's permission, `docs/testing/proof-contract.md:54-57`.)*

## Dev Notes

### What changes, by file

| Area | Files | Today → after |
|---|---|---|
| Lock / policy | `Cargo.lock`, `deny.toml`, `crates/maos-wasm-host/Cargo.toml:45-49`, `crates/maos-wasm-host/guests/equiv-fixture/native-twin/Cargo.lock` | 9 deny errors → `advisories ok` |
| CI | `.github/workflows/discipline.yml` (`:60-166` deny steps → new job; `:556`, `:585` `if:`; `:595-618` junit; `:5334-5387` needs; `:5481-5615` needs; `:559-565` comment) | deny advisory → blocking; proven-red steps run after a red gate |
| WIT / pins | `wit/spirit.wit:1`, `tests/fixtures/wasm/equiv-fixtures.provenance.toml` (4 × `source_sha256`) | comment true; binaries unchanged |
| Env | `crates/maos-bin/src/env_contract.rs` | `check-env-contract` 1 violation → 0 |
| Tests (flakes) | `crates/maos-bin/tests/hook_pump_17_4.rs`, `crates/maos-bin/tests/worker_terminal_facts_17_7/supervisor_scenes.rs` (± `stub_scenes.rs`), `crates/maos-wasm-host/tests/recall_runner_17_3d.rs`, `crates/maos-kernel-core/tests/t3_terminal_helpers_17_7.rs` | deadlines → barriers |
| Budget | `xtask/test-budgets.toml`, `xtask/gate-registry.toml:429-438`, `docs/testing/proof-contract.md:66-67` | 300 s / advisory → 800 s + 4 rows / blocking |
| Rupture id | `crates/maos-a2a-core/src/router.rs:431-439`, `crates/maos-a2a-core/Cargo.toml`, `crates/maos-cohort/tests/rupture_restart_18_0.rs` (new) | process-local counter → ULID |
| Init | `crates/maos-shell/src/lib.rs:64-67`, `crates/maos-bin/tests/shell_8_14a.rs` | CRL dir created |
| xtask tests | `xtask/tests/<live-workflow assertion>` (new or extended) | AC6 shape pinned |
| Records | `_bmad-output/implementation-artifacts/deferred-work.md`, `18-0-evidence/` | — |

### Guardrails

- Never touch `crates/maos-kernel-core/src` (kernel-Δ 0); the pidfd fix is test-only.
- Never widen a wall-clock bound; never turn a deadline into a longer deadline — a hang guard names itself as a guard.
- Never ignore a *vulnerability* or *unsound* advisory; only the three ruled `unmaintained` IDs, with a reason and an owner.
- Never move to wasmtime 49+/50 in this story (majors; the LTS line is 48, `crates/maos-wasm-host/Cargo.toml:45-47`).
- Never "re-run failed jobs" on the landing run — it breaks the ledger binding (`deferred-work.md:1165`).
- Never swallow an error the test exists to see: EPIPE is accepted only on the one post-latch write; ESRCH only under D-18-0-08 B, if ruled.
- The rupture id must stay equal across payload, frame and TL row; never journal it with `None` under C.
- `maos init` tests never write the runner's real home (CR-6).
- One plant per control AC (AC5, AC6, AC7, AC8), in place, restored with `git checkout -- <file>`; no second target dir; local runs `env -u CI -u GITHUB_ACTIONS` (`proof-contract.md:22-27,84-87`).
- No `cargo` inside a `#[test]` (`proof-contract.md:68-76`); the AC6 assertion reads YAML only.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red |
|---|---|---|
| Budget rows (AC5) | one `[[exception]]` lowered below its CI Σ | `check-test-budget --junit <40080259088 junit>`: 1 finding, exit 1 |
| `expensive-tests` shape (AC6) | remove the `if: ${{ !cancelled() }}` of one proven-red step | the live-workflow assertion test |
| Rupture id (AC7) | restore `frame_id[0..8] ‖ alloc_id()` | `rupture_restart_18_0`: second denial `CODE_INTERNAL`, one row |
| CRL dir (AC8) | remove the mkdir | the new `shell_8_14a` test |

AC2 and AC3 are existing gates whose HEAD reds are recorded; AC4 changes tests, not controls.

### Tests and commands

```bash
# local (agent shell exports CI=true); one cargo process; CARGO_TARGET_DIR=/mnt/build/cargo
env -u CI -u GITHUB_ACTIONS cargo deny check                       # cargo-deny 0.19.4 (release binary is fine)
env -u CI -u GITHUB_ACTIONS cargo run --locked -p xtask -- build-test-fixtures
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-wasm-host
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-bin --test hook_pump_17_4 -- --test-threads=4   # ×10, then --test-threads=2 ×10
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test hook_pump_17_4 --test worker_terminal_facts_17_7 --test shell_8_14a
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core --test t3_terminal_helpers_17_7
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-a2a-core -p maos-cohort -p maos-a2a-tcp
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p xtask --test check_test_budget_proven_red --test check_test_cost_proven_red
cargo run -p xtask -- check-env-contract
cargo run -p xtask -- check-equiv-fixture-provenance
cargo run -p xtask -- check-test-budget --junit <run>/junit.xml --json   # the five recorded junits
cargo run -p xtask -- check-multi-tenant-loom --json                      # whole-package leg = cargo test -p maos-bin
cargo run -p xtask -- check-test-cost && cargo run -p xtask -- check-kernel-baseline && cargo run -p xtask -- kloc-check --json
cargo fmt --all -- --check
```

`root_shutdown_unload_16_3` and `worker_terminal_facts_17_7` need the T3 substrate; locally they print `SKIP:` without it (`tests/harness/ci.rs`), so their proof is CI's.

### Previous-story intelligence

- **17-10 (done):** lighter-contract creation; plants in place, one per control, recorded in `<story>-evidence/plants.md`; measured kloc requests returned before landing; `interval_at` instead of an immediate first tick — the CR-5 oracle flake was caught at review.
- **17-11 (done):** a manifest/admission change widens the focused set (`proof-contract.md:45-51`) — AC7 touches neither, but the landing-run reds of 17-11 are why AC1 reads the whole run.
- **20-12 (done):** fixtures only via `build-test-fixtures`; `SKIP:` locally, fail-closed in CI; nextest is process-per-test — which is exactly why AC4(a) never reds in `workspace-test-suite` and always can in the loom leg.
- **17-8 (done):** the grant-scoped rupture rows and the D-17-8-03 withhold that AC7's collision defeats (`digest.rs:318-323`).
- **17-3b (done):** the four-component re-issue recipe and the provenance gate.

### Git intelligence

`1fec4882` CI records (69224639 run; two landing flakes routed to 18-0/20-7; reviewer-model policy in the proof contract); `57d15b16` 17-10 CI proven-red record; `69224639` 17-11 follow-up; `1bc53e63` 17-10 done + E17 retrospective (§18-0 filed). Local `main` is 2 commits ahead of `origin/main` (`69224639`). None of 18-0's files changed since the retrospective except `deferred-work.md` (`:1208-1209` added by `1fec4882`).

### Latest technical information

- **wasmtime 48.0.7** — patch on the 48 LTS line fixing GHSA-9pvw-hwfg-92h5 (component-model stream lowering panic, RUSTSEC-2026-0378) and GHSA-mghf-wjfp-p5xm (aarch64 Cranelift `br_table` jump target, RUSTSEC-2026-0379 — relevant to the two arm `wasm-host-tests` images); `>=49.0.4` is the fix on the next major. No API change observed (zero-diff build).
- **rustls-webpki 0.103.16 / webpki-roots ≥ 1.0.10** — reject the distrusted emSign intermediates (RUSTSEC-2026-0382/0381; `rustls/webpki#546`). Reached through `maos-a2a-tcp`'s direct `webpki` dependency and `rustls 0.23.46` (A2A mTLS, egress), and `ureq 2.12.1 → webpki-roots 0.26.11 → 1.0.x` (`maos-notify-push`, `maos-egress`) — `cargo-tree-reachability-1fec4882.txt`.
- **tokio 1.53.3** — RUSTSEC-2026-0386: `AsyncFd::new` and related constructors violate I/O safety (unsound); fixed in 1.51.6 and 1.53.3+.
- **cargo-deny 0.19.4** accepts plain-string `ignore` IDs (the file's current style) — keep the house style.

### Project Structure Notes

- New test files follow `<topic>_18_0.rs`; evidence under `_bmad-output/implementation-artifacts/18-0-evidence/` (`creation/`, `ci/`, `plants.md`).

### References

- Epic `epic-18-spirits-that-think-w3.md`: §18-0 `:100-118` (creation corrections in place), story table `:89`, rules 9–11 `:38-40`, Dependencies `:210`.
- `epic-17-retro-2026-10-24.md` §6 `:81-102` (R1–R4), §7 `:108-109` (E17-A1/A2), §5 `:69-79`; `epic-16-retro-2026-09-20.md:136,148` (E16-A5).
- `deferred-work.md:1158-1165,1173-1174,1178,1184-1187,1197-1199,1208-1209`.
- `docs/testing/proof-contract.md:10-18,22-27,33-57,61-80,84-87`; `20-12-evidence/ci-after.json`.

## Evidence

- `18-0-evidence/creation/ci-run-40080259088-reds.txt` — the last main run's nine non-success jobs, their failure lines, the `aggregate` needs JSON for `reproducible-build`, and all 187 job timings.
- `18-0-evidence/creation/cargo-deny-0.19.4-head-1fec4882.log` — the nine advisory errors at HEAD (advisory-db `21d909e4`).
- `18-0-evidence/creation/prototype-upgrade-reverted.txt` — lock updates, deny before/after, `maos-wasm-host` 113/113 on 48.0.7, workspace check, diff stat, revert.
- `18-0-evidence/creation/cargo-tree-reachability-1fec4882.txt` — `cargo tree -i` for every advisory crate.
- `18-0-evidence/creation/wit-comment-reissue-probe.txt` — byte-identical components, the four new source pins.
- `18-0-evidence/creation/hook-pump-root-cause.txt` — 9/10 → 0/10.
- `18-0-evidence/creation/test-budget-calibration.txt` — five junits, per-binary maxima, the plant.
- `18-0-evidence/creation/governance-1fec4882.txt` — kloc, env-contract prototype, pins.
- `18-0-evidence/creation/e17-a2-method-probe.txt` — job buckets and the compile-share method.

## Dev Agent Record

### Story context creation

Created 2026-10-24 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter creation contract) at `1fec4882`: four read-only scouts (flakes; rupture-id/init/env; CI shape; WIT re-issue), direct reads of every cited range, the failed-job logs and five junit artifacts of the post-20-12 main runs (`gh`), cargo-deny 0.19.4, `cargo update`/`cargo tree`/`cargo check`/`cargo nextest` as the host's only cargo process, and four throwaway prototypes — lock upgrade + deny ignore + budget rows + env entry; WIT comment re-issue; hook-pump `Once` guard — each reverted (`git status` clean after each). No party-mode preflight; no non-author validation pass. Epic §18-0 corrected in place (creation note `epic-18…md:104`). Status stays `backlog`; fourteen decisions PROPOSED (D-18-0-01…14).

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List

### Change Log

- 2026-10-24 — Story created (backlog; D-18-0-01…14 PROPOSED) at `1fec4882`: advisory set re-measured (only 0378/0379 are wasmtime; zbus ×3 unmaintained; 0385/0386 new; deny runs in an advisory job); wasmtime 48.0.7 proven zero-diff; WIT comment re-issue proven byte-identical; hook-pump root cause measured (clock re-init under libtest) and fixed in prototype; pidfd ESRCH is the reap window of `PIDFD_GET_INFO`; budget rows proven on five CI junits; rupture counter starts at 1 and the collision answers `CODE_INTERNAL`; CRL dir alone does not silence the poller; E17-A2 method probed; epic §18-0 corrected in place.
