# 20-1 creation — cites re-measured at HEAD `a4c00897` (rule 9)

`a4c00897` = `77f7276a` (20-3c final landing, origin/main) + one docs commit; `git log -1 -- crates xtask` = `a4c00897`
(the 20-3c done commit touched `xtask/clippy-baseline.toml` only among gate inputs). Every line below was re-read at
HEAD with `read`/`grep`; "ok" means the cite still points at the named code. E20 = `epics/epic-20-ship-it-w5.md`,
PF = `epic-20-preflight-2026-10-30.md`, S = `sprint-status.yaml`, D = `deferred-work.md`.

**Applying these:** E20 is a shared file. The OLD→NEW rows below are applied by the orchestrator from
`records-owed.md` (A2–A6), not by this creation run.

## C — code cites in E20 §20-1 (amended paragraph E20:98, E20:100) and its ACs (E20:104-107)

| # | Where | OLD | NEW at HEAD | Note |
|---|---|---|---|---|
| C1 | E20:98 | `storage.rs:125-135` `at_path` | ok (doc `:125`, `pub fn at_path` `:126-135`) | reuse, no `with_root` |
| C2 | E20:98 | `server.rs:62-74`; signer `:77-80`; dispatch `:235-243` | ok; dispatch table `:229-261` (`registry.manifest` `:235`, `.artifact` `:240`, `.publish` `:245`, `.deprecate` `:250`, `.yanks_since` `:255`) | |
| C3 | E20:98 | client wiring `main.rs:3268-3310` | `main.rs:3251-3361` (switch); production HTTP arm `:3281-3320`; yank-poller spawn `:3321-3347` | moved +13 |
| C4 | E20:98 | `client.rs:46-50`; transport `streamable_http.rs:44-47` | ok (`new` `:46`, `with_config` `:55`); ok (struct `:44`, `new` `:56`) | |
| C5 | E20:98 | `maos-cli/Cargo.toml:18-27`; `lib.rs:38-41`; spirit-cli `Cargo.toml:30-33` | ok (`[dependencies]` `:18`, `maos-registry` `:23`, no `maos-mcp`); ok (`pub use` `:38-41`); ok (`maos-domain`/`maos-mcp`/`maos-registry`/`maos-spirit-abi` `:30-33`) | |
| C6 | E20:98 | import admission/persist/summary `subcommands.rs:1437-1455,1465-1501` | `dispatch_import` `:1348-1544`: vetting inputs `:1412-1468`, admission `:1470-1496` (`admit_spirit_with_attestation` `:1479`, `admit_spirit` `:1488`), persist `:1498-1519` (`LocalFsRegistryStorage::new()` `:1500`, `publish_with_origin` `:1510`), summary `:1521-1542` | moved +33…+41; the `--registry-uri` flag is parsed and ignored (`:1349-1354`) |
| C7 | E20:98 | `origin.rs:9-17` | ok (`enum RegistryOrigin` `:9-17`: `Published`, `Imported`) | `Remote` absent |
| C8 | E20:100 | `attestation.rs:105-128,150`; `vetting/mod.rs:197`; `admission.rs:388-396` | ok (`VettingClaim` `:105-129`, binds `manifest_hash` `:108`; `issue_attestation` `:150`); ok (`verify_attestation` `:197`, manifest-only check `:208-209`); ok (`:388`) | production `issue_attestation` callers: **0** (all callers are `#[cfg(test)]` modules — `attestation.rs:233-267`, `vetting/mod.rs:394-662`, `maos-registry/src/admission.rs:1248` (tests from `:568`) — or `tests/`) |
| C9 | E20:100 | `main.rs:276-339`; `revocation/applier.rs:160-169` | `TlYankObserver` `main.rs:273-347` (struct `:276`, sink impl `:288-308`, `on_yank` `:311-346`); applier Quarantine `:160-169` ok, drain-then-terminate task `:170-197` | |
| C10 | E20:100 | `admission.rs:241-253`; `compliance.rs:202` | `rust-inproc` refused `:247`, `wasm-component` `:250`; ok (`PublicUntrusted = 3` `:202`) | |
| C11 | E20:104 (a) | `bin/server.rs:1-7`; `Cargo.toml:36-38` | ok (stub prints "not yet wired — Story 5.5d Task 13", exits 1, `:4-6`); ok | **measured rc 1** (T1) |
| C12 | E20:104 (a) | `src/server.rs:78-81` | `:77-80` | |
| C13 | E20:104 (a) | `storage.rs:89-96` `new()` | `:108-123` | stale since PF:155 |
| C14 | E20:104 (a) | daemon arm `main.rs:7693-7701` | `main.rs:8980-8988` (port literal `127.0.0.1:6789` `:8985`, `LocalFsRegistryStorage::new()` `:8984`) | E20:29 says `:8854-8861` — also stale |
| C15 | E20:104 (a) | `release.yml:44` | `release.yml:76` (`cargo build --release -p maos-bin -p maos-cli`), package `:78-82`, uploads `:83-90`; `xtask/src/release_dry_run.rs:27-36` (`ARTIFACTS` = maos, maosctl) | |
| C16 | E20:104 (b) | `publish.rs:267-272` | `publish.rs:266-271` (`Err(anyhow!("… requires the kernel-side composition root …"))`); stub branch `:242-264` | **measured rc 1** (T1) |
| C17 | E20:104 (b) | `client.rs:38-50`; daemon wiring `main.rs:2918-2948` | ok; `main.rs:3281-3320` | |
| C18 | E20:104 (b) | `streamable_http.rs:19`; only HTTP `IoSubsystemPort` `maos-kernel-core/src/io/mod.rs` | struct `:44`; `impl IoSubsystemPort for IoSubsystemAdapter` `io/mod.rs:86` (only production impl; others are test mocks) | |
| C19 | E20:104 (b) | spirit-cli `Cargo.toml:31-32`; `server.rs:150-155`; `publish.rs:288` | ok; ok (only `POST` checked, path ignored); ok | |
| C20 | E20:104 (c) | `SpiritOp` `cli.rs:803-870`; top-level install `:336-363` | `enum SpiritOp` `cli.rs:915` (`HotSwapPrecheck` `:918`, `Upgrade` `:941`, `Inspect`); top-level `Install(InstallArgs)` `:44`, `InstallArgs` `:427` | **measured rc 2** "unrecognized subcommand 'install'" (T1) |
| C21 | E20:104 (c) | remote fetch removed `subcommands.rs:1209-1210` | `subcommands.rs:1616` | |
| C22 | E20:104 (c) | `subcommands.rs:1073-1083` / `:1092-1112` / `:1116-1129` | `:1479-1489` / `:1498-1519` / `:1521-1534` | = C6 |
| C23 | E20:104 (c) | `admission.rs:196-208` (`public_untrusted` = signature + claim) | `admission.rs:275` (`TrustTier::PublicUntrusted =>` arm of `admit_spirit` `:259`) | |
| C24 | E20:104 (d) | `main.rs:7694-7697` (only reader of the store) | registry-server arm `:8981-8985`; eval-sidecar reader `main.rs:801,821` — "only reader" false (PF:168) | |
| C25 | E20:104 (d) | `maos-shell/src/lib.rs:176-200` | `lib.rs:506-531` (`parse_at_line` `:507`, `@butler` refusal `:517-523`, `unknown spirit … known: hello-spirit` `:524-531`) | **measured**: `maos: unknown spirit 'example-spirit-ts' — known: hello-spirit`, rc 0 (T1) |
| C26 | E20:106 (AC2) | test callers `admission.rs:1219`, `tests/vetting_attestation_gate.rs:176`, `tests/end_to_end_test.rs:229` | `admission.rs:1248`; `:184`; ok | |
| C27 | E20:106 (AC2) | keyring `:157`, `:334`, `:288`, `:117` | ok (`push_attestation_revocation` `:157`, `is_revoked` `:334`, `verify_attestation_not_revoked` `:288`, `issue_event` `:117`) | |
| C28 | E20:106 (AC2) | `maos-spirit.rs:25-28` (`validate`/`inspect` stubs) | ok (`Command` `:22-29`; no `Vet`/`Yank`) | |
| C29 | E20:107 (AC3) | `main.rs:305-339`, `:316-323`, `:328-338`, sink `:281-300` | `on_yank` `:311-346`; `SpiritRevoked` row (pid 0, intent `yank-poller:remote-yank-propagated`) `:323-330`; `observe_running_spirit(… registry_yanked: true …)` `:335-345`; sink `:288-308` | |
| C30 | E20:107 (AC3) | `env_contract.rs:13` | ok (`MAOS_ENV_REGISTRY` `:13`); `MAOS_REGISTRY_YANK_POLL_INTERVAL_S` `:392`; `MAOS_YANK_POLICY` absent | |
| C31 | E20:107 (AC3) | `operator_config.rs:12-26` | `RegistrySection` `:13` | |
| C32 | E20:107 (AC4) | example `manifest.toml:8` (`trust_tier`) | `:9` (`trust_tier = "local"`); forms `:7`, artifact `:8`; template `templates/spirit-ts/manifest.toml:9` also `local` | |
| C33 | E20:107 (AC4) | `publish.rs:111-118`; `main.rs:136-137` | `publish.rs:115-120` (`TierMismatch`); `main.rs:142-143` (`parse_manifest_trust_tier` in `enforce_vetted_upgrade_precondition` `:130-202`) | |

## X — exit-block amendment and provenance (E20:16-24, 27, 29, 32-40)

| # | Where | OLD | NEW at HEAD |
|---|---|---|---|
| X1 | E20:29 | daemon arm `main.rs:8854-8861` | `main.rs:8980-8988` |
| X2 | E20:29 | `SpiritOp::Install` absent `cli.rs:893` | `cli.rs:915` |
| X3 | E20:29 | init/shell `main.rs:1896-1948`; hello-only loading `:4251-4293` | `Init` `main.rs:1896`; hello-only shell load `:4309-4440` (`bind_storage_identity(hello_pid, "hello-spirit")` `:4435`) |
| X4 | E20:29 | shell does not load installed Spirits `lib.rs:509-524` | `lib.rs:506-531` |
| X5 | E20:29 | `WasmEngineOff` run path `main.rs:6533-6534` | `main.rs:6607-6608`; gate `crates/maos-bin/src/admission.rs:700-703` |
| X6 | E20:36 | "real server is `MAOS_ONE_SHOT=registry-server maos` at `main.rs:7693-7701`" | `:8980-8988`; **measured functional** (T1): ready in 3 polls, `registry.search` 200 `{"items":[]}` |
| X7 | E20:38 | `publish.rs:267-272`; `manifest.toml:8` local tier; `publish.rs:111-118` | `:266-271`; `:9`; `:115-120` |
| X8 | E20:39 | `SpiritOp` `cli.rs:803-870`; `subcommands.rs:1209-1210` | `cli.rs:915`; `subcommands.rs:1616` |
| X9 | E20:40 | `main.rs:1679`, `:1730-1737`; `lib.rs:164,176-200`; `main.rs:7694-7697` | `main.rs:1896`; `lib.rs:506-531`; `main.rs:8981-8985` (+ eval sidecar `:821`) |
| X10 | E20:27 | `release_dry_run.rs:27-36`; `xtask/src/main.rs:816-843`; `release_verify.rs:25-28,56-59,253-261`; `subcommands.rs:1589-1595,1608` | ok (ARTIFACTS `:27-36`); lines 2 and 4 **ran green** on a dev-signed two-binary dist staged from the prebuilt binaries (T1) |

## T — runtime measurements (prebuilt binaries, no cargo; `runtime/exit-probe-a4c00897.txt`)

| # | Exit line | Result at HEAD |
|---|---|---|
| T1 | 2 `release-verify --sign` (dev seed) | rc 0, "signed 2 entries" |
|  | 4 `maosctl install --from-local --verify-only` | rc 0, "verification passed for 2 file(s)" — the verb is runnable; the four-binary set is not (line 1) |
|  | 5 `maos-registry-server --bind --root` | rc 1, "not yet wired — Story 5.5d Task 13" |
|  | 5-alt `MAOS_ONE_SHOT=registry-server maos` | functional: listens on fixed `127.0.0.1:6789`, `registry.search` → `{"items":[]}`; **`registry.deprecate` with no credential yanks any `spirit_id`** (`yank-80`, persisted to `yanks.json`) |
|  | 7 `maos-spirit publish --tier=public_untrusted` | rc 1, the non-stub guard (fires before the tier check; `--tier=local` gives the same) |
|  | 8 `maosctl spirit install …` | rc 2, clap "unrecognized subcommand 'install'" |
|  | 9a `maos init` | rc 0, records an operator endpoint |
|  | 9b replay `maos shell`, `@example-spirit-ts hello` | rc 0, `maos: unknown spirit 'example-spirit-ts' — known: hello-spirit` (not the typed engine-off refusal) |
|  | 1 `release-dry-run` | not run (builds with cargo; parent rule); source: two artifacts only |

Binary provenance: `/mnt/build/cargo-maos/debug/{maos,maosctl,maos-registry-server,maos-spirit,xtask}`, mtimes
2026-11-01 13:46–13:51Z, before the `a4c00897` commit (13:52Z) whose crate/xtask sources match [INFERENCE: mtime, not a
build-id comparison].

## F — facts at HEAD the epic does not state

| # | Fact | Evidence | Bears on |
|---|---|---|---|
| F1 | The 5-min yank poller **is wired** in the daemon when `MAOS_REGISTRY_URI` is `http(s)://` (`main.rs:3321-3347`; `yank.rs:150-210`, default 300 s, clamp 30–3600 s) — FR59's cadence exists; policy, origin and authentication do not | `yank.rs:62-128,150-210` | AC7, AC8 |
| F2 | Yank timestamps are **server-process-monotonic** (`storage.rs:365-394` stamps `monotonic_now_ns()` `:499-503`, reset per process) and compared with a client cursor (`yank.rs:81-102`): after a server restart new yanks carry small values below the cursor and are never fetched. Measured: a fresh server stamped `yanked_at_ns = 2885` | T1 5-alt; `storage.rs:396-407` | AC7 (D-20-1-07) |
| F3 | The cursor persistence (`yank.rs:212-306` `save_cursor`/`load_cursor`) has **no production caller**: `grep load_cursor\|save_cursor crates/maos-bin/src` is empty, so every daemon boot replays every historical yank into the TL | grep | AC7 |
| F4 | `registry.deprecate` is unauthenticated; `registry.publish` stores any package without verifying its signature and **overwrites** an existing `name/version` from any key (`handlers/publish.rs:9-22`; `storage.rs:211-250` writes then `retain(|i| i.version != version)`) | T1; code | AC7, AC10 |
| F5 | `YankEntry` has no origin (`maos-domain/src/ports/registry.rs:331-340`: `spirit_id`, `version`, `yanked_at_ns`, `reason`); `RevocationOrigin::{Operator,Publisher,RegistryYank}` exists for CRLs (`maos-domain/src/revocation.rs:296-310`) | code | AC7 |
| F6 | `observe_running_spirit` has one production caller (the yank observer, `main.rs:335`); `vetting_revoked`/`attestation_expired` are set only in tests (`tests/vetting_attestation_gate.rs:329,333`) — the D9 lapse/revocation path has no producer | grep | AC9 |
| F7 | The Story 19-2 capability fence is a **pub, non-kernel-callable** isolation API: `CapabilityRegistryAdapter::set_paused` (`capability/mod.rs:332-340`) makes `issue_with_mediation` (`:203`), `verify_and_audit` (`:319`) and `recheck` (`:345`) fail closed; mediated sites span inference, MCP tool calls, memory, worker spawn and the schedule watchdog (`issue_with_mediation`/`verify_and_audit` callers: `inference/mod.rs` ×3, `mcp/mod.rs` ×3, `memory/mod.rs` ×4, `schedule_watchdog.rs`, `maos-bin` `main.rs` ×2, `shell_host.rs`, `skill_authoring.rs`, `worker_spawn.rs`). It is lifted by `resume` (`scheduler_loop.rs:488`) and `unload` (`:572`) | code | AC11 (D-20-1-02) |
| F8 | The CRL poller calls the applier from inside the kernel (`revocation/poller.rs:65`); the door's apply path (`operator_door.rs:2010`) and `main.rs:8247` are the other callers — a non-kernel route cannot change what the applier's Quarantine arm does on a polled CRL | code | D04 B comparison |
| F9 | Any edit to `revocation/applier.rs` that removes the >100-column `json!` line (`:163`) lets `rustfmt` format the whole `apply_crl` body: **+40 physical / +40 tokei of formatting alone** (control prototype `fmt0-…diff`) — rustfmt currently skips that statement silently | prototype | kernel pricing |
| F10 | `bind_storage_identity` takes an opaque identity string (`memory/private.rs:81-100`; directory `id-<hex(identity)>`, `MAX_KEY_LEN` 1024 `:39`); maos-bin passes the bare `spirit_id` (`admission.rs:893`, `main.rs:4435`) — a publisher-keyed identity needs no kernel change, but the hex doubling meets the 255-byte file-name limit for long names | code | AC10 (D-20-1-09) |
| F11 | Exit-job enrollment: `epic-20-exit` has 0 hits in `.github/workflows/discipline.yml` and `xtask/gate-registry.toml` | grep | AC5 |
| F12 | Coverage rows: FR13 notes "asserted, owner 20-1" (`tests/coverage-matrix.yaml:366-371`); FR35 `:617`, FR36 `:622`, FR59 `:841` have `gates: []`; FR37 is `phase: v2.0+` `:627-632`; invariant I5 owner `20-1`, `FR13: asserted, owner 20-1` (`docs/invariants/I5.md:4,11`) | read | AC13 |

## L — ledger and tracker anchors

| # | Anchor | Content |
|---|---|---|
| L1 | S:296 | 20-1 `backlog`; D9 routing (vetter lapse/revocation → vetter yank with policy, `enforce_vetted_upgrade_precondition` `main.rs:130-202` stays the only refusal point); R10 FR13 quarantine; D8 binding; R8 hyphen tier |
| L2 | D:37 | same-name principal-storage squatting — Owner `20-1` (20-3b row 06 A) |
| L3 | D:56 | FR59 coverage row — Owner `20-1` |
| L4 | D:783 | D9 CLOSED by decision; lapse consequence routed to 20-1 (closed row; no owner assertion) |
| L5 | D:1093 | attestation binds manifest only — Owner `20-1` |
| L6 | D:1147 | FR13 quarantine has no isolation semantics — Owner `20-1` |
| L7 | D:1264 | revocation poller prints every failed poll — Owner `20-1` (D-18-0-12 A; design B edge-trigger) |
| L8 | routing-audit `:43`, `:124-127`, `:143`, `:169` | FR36 authenticated install, FR37 lapse/revocation, FR59 ≤5 min + vetter origin, FR13 isolation; J6 independence unproven (ops) |
