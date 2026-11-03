# 20-13 creation — cites re-measured at HEAD `8c809f9c` (2026-11-03)

Method: every path read from a `git archive 8c809f9c` copy (`/tmp/proto2013/base`, deleted after), never from the
working tree (the working tree carries a sibling's uncommitted 20-4 L1 edits to `main.rs`, `admission.rs` and
`maos-providers`). Line numbers are at HEAD. No cargo ran.

## C — epic / preflight / tracker cites corrected (OLD → NEW)

| # | Where (OLD) | NEW at `8c809f9c` | Consequence |
|---|---|---|---|
| C1 | E20:308, PF:257 — init selection absent `main.rs:1896-1948` | The `Init` verb arm is `crates/maos-bin/src/main.rs:1906-1913` (air-gap `:1547`, `air_gap_init` `:1575-1581`); the writer is `maos_shell::run_init` `crates/maos-shell/src/lib.rs:54-155` (`config.toml` `:59`, write `:118`, idempotent early return `:86-110`, `default_config_toml` `:784-801`). Verb row `crates/maos-bin/src/verbs.rs:90-95`. | Selection is still absent; the init-owned file is `config.toml`, which no reader consumes today (grep `config.toml` over `crates/*/src` = writer only). |
| C2 | E20:308, PF:256-257 — boot defaults ring `:2190-2191`; candidate root `:2190` | `main.rs:2218-2219` (`Arc::new(RingCryptoProvider)` + `ring-default` eprintln) | Same shape, shifted +28. |
| C3 | E20:308, PF:256 — CRL `:3211` | `main.rs:3238-3239` — a **second** `Arc<dyn CryptoProvider>` (`crypto_provider`) feeding `RevocationPoller` `:3240-3245`, the door `:3653`, `:3819` (door field `operator_door.rs:441,496`, use `:1999`) | AC1 "exactly one instance" is false at HEAD: two production instances. |
| C4 | E20:308, PF:256 — image smoke `:8202` | `main.rs:8403` inside the `smoke-t3-sandbox-5` arm (`:8374` onward) | Smoke arm, not the root. |
| C5 | E20:308, PF:256 — worker `worker_spawn.rs:888` | holds: `crates/maos-bin/src/worker_spawn.rs:888` inside `run_cli_wrapper_manifest` (`:585-599`); callers `main.rs:5504`, `:6088`, `delegation.rs:938` (host-B context `delegation.rs:846-875`, built `main.rs:11874-11930`) | Production `maos run` Worker path; a third instance per T3 launch. |
| C6 | E20:308, PF:256 — `enterprise_identity.rs:751` test module, imports `:648` | `:751` holds, inside `#[cfg(test)] mod` from `:637`; the import is `:649` | Test. |
| C7 | E20:308, PF:256 — later main `:15081/:15342/:15817/:15846` | `main.rs:15354` (`live_mcp_fixture`), `:15615` (`#[test]`), `:16090`, `:16119` — all inside `#[cfg(all(test, feature = "network"))]` modules from `:15114` and `:15711` | Test; allowed by the structural check. |
| C8 | E20:310, S:315 — "≈ 12 production sites `main.rs:1942,2946,7177,13818,14076,14551,14580`; `worker_spawn.rs:846`" | Production constructions at HEAD: **maos-bin 3** (`main.rs:2218`, `:3239`, `worker_spawn.rs:888`) + **1 smoke** (`main.rs:8403`) + **kernel 1** (`crates/maos-kernel-core/src/security/mod.rs:350`, T3 admission) — the gate simulation `gate-sim-base.txt` lists exactly these 5. Tests: maos-bin 5 (C6, C7); bench src 2 (`maos-bench/src/halt_eval.rs:625`, `harness/j4.rs:154`). | The kernel site was never in any inventory: kernel T3 admission verifies image locks with a fresh `RingCryptoProvider` whatever the root selected → AC7 (L2, FLAG-Winston). |
| C9 | E20:310, S:315 — "16 src files sign or verify with ed25519/ring directly" | **21** non-test `src` files sign, verify or seal with `ring`/`ed25519-dalek` directly besides the provider itself; 6 more use `ring` only for hashing, constant-time compare or the rustls TLS provider (`bypass-inventory-8c809f9c.md`) | Re-counted; dispositioned one by one. |
| C10 | E20:313 (AC2) — "sealed-export encryption (`seal_for_export` — `maos-secrets` at-rest **and the audit sealed export**)" | `maos-secrets` routes `seal_at_rest` through the injected provider (`crates/maos-secrets/src/lib.rs:462-499`, call `:481-483`; callers `enterprise_identity.rs:290,477`, root passes `crypto` `main.rs:2339-2340`). The audit sealed export **does not encrypt**: `maos-audit/src/sealed_export.rs:296-334` signs with `ed25519-dalek` (no AEAD in `maos-audit`; `regional_teardown.rs:48` "signed, not encrypted"); its producer is `maosctl` (`maos-cli/src/subcommands.rs:3268`) | False premise → D-20-13-10. |
| C11 | E20:313 (AC2) — capability-token signing must be routed | Already routed: `CapTokensShardRing::issue` calls `self.crypto.sign_capability_token` (`crates/maos-capability/src/cap_tokens/mod.rs:176-179`), instance from `CapabilityRegistryAdapter::new` (`crates/maos-kernel-core/src/capability/mod.rs:155-167`), root passes `crypto` (`main.rs:2261-2262`) | Only the instance's origin changes (AC1). |
| C12 | E20:313 (AC2) — "admission-time ComplianceClaim verify" | `maos_compliance::evaluator::evaluate_envelope_at` verifies with `verify_ed25519` (ring direct) `crates/maos-compliance/src/evaluator.rs:100-106,317-321`; publisher signature `maos_registry::admission::verify_publisher_sig` (ring direct) `crates/maos-registry/src/admission.rs:549-562`. Daemon caller: boot re-verification of installed Spirits `crates/maos-bin/src/installed_spirits.rs:25` → `maos_registry::install::verify_installed` `crates/maos-registry/src/install.rs:111-150` (publisher `:116`, claim `:143-148`). `maosctl` callers `install.rs:61`, `maos-cli/src/subcommands.rs:1355-1410` (maos-cli has no kernel or maos-secrets dependency, `crates/maos-cli/Cargo.toml:22-25`) | D-20-13-05. |
| C13 | E20:314 (AC3) — "visible in `maosctl` posture" | `maosctl posture` is the per-Spirit posture-shift **write** verb (`crates/maos-cli/src/cli.rs:73-74`, `subcommands.rs:1982-1992`); no daemon-level read surface exists | D-20-13-14. |
| C14 | E20:314 (AC3) — "`runtime_context.rs` already names `"ring"`" | Only in doc text (`crates/maos-compliance/src/runtime_context.rs:13-14`). Production contexts take the **manifest's** crypto id: `install.rs:135-142`; `AdmissionConfig.runtime_crypto_provider: None` ⇒ manifest-derived (`admission.rs:142-145,161-163`), callers `install.rs:57-58`, `subcommands.rs:1408-1409`, smoke `main.rs:8945,9014,14309,14565` | The FR38 drift check on crypto is vacuous (D:1379, owned here) → AC4, D-20-13-06. |
| C15 | PF:130 — "Kernel: Yes; unmeasured" | Measured by reverted prototype: KT3 **+1 physical / −1 tokei** (`measurements.md`) | D-20-13-04. |
| C16 | — (port doc) | `crates/maos-domain/src/ports/crypto.rs:5` says NFR-Sec-15 FIPS is v1.0; `:10-17` says v0.1-α has zero kernel call sites; `crates/maos-kernel-core/src/security/crypto.rs:20-27` says a FIPS swap is "one line in `main.rs`" | Stale; AC6 rewrites them (comments: 0 tokei, kernel physical counted at L2 only if touched). |
| C17 | — (identity) | The trait has no name/version method (`crypto.rs:50-97`) | D-20-13-03. |
| C18 | — (open path) | `open_at_rest` is ring-direct by design (`maos-secrets/src/lib.rs:515-540`, rationale `:516-528`: adding an open method touches the kernel impl) | D-20-13-11. |

## H — cites that hold at HEAD

`crates/maos-domain/src/ports/crypto.rs:50-97` (trait), `:106-119` (`CryptoError`); `crates/maos-kernel-core/src/security/crypto.rs:37-98`
(`RingCryptoProvider`), `:100-150` (`MockCryptoProvider`, `pub mod tests`); `revocation/parser.rs:19,58`; `revocation/poller.rs:17-28`;
`security/sandbox/t3/image_lock.rs:152-185`; `security/sandbox/t3/image_verify.rs:42,79,98`; `security/mod.rs:131-141` (struct,
`#[derive(Debug, Clone)]` `:130`), `:201-235` (`new`, builders), `:349-366` (T3 admission), `:479-487` (`Default`); `maos-secrets/src/lib.rs:9-10`
(`kms-fault-inject` `compile_error!` precedent); `crates/maos-bin/Cargo.toml:14-38` (features; `kms-fault-inject` `:36`); `Cargo.lock:4464-4465`
(`ring` 0.17.14); `xtask/src/check_composition_root_completeness.rs:1-14,59-131` (blocking job `.github/workflows/discipline.yml:431-443`,
`aggregate.needs` `:6534`); `xtask/src/check_mock_not_in_release.rs:29` (`FORBIDDEN_PRODUCTION_SYMBOLS`); `xtask/src/build_test_fixtures.rs:70-260`;
`crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:931` (`SCANNED_SOURCE_FILES: [_; 37]`); `tests/coverage-matrix.yaml:740-754` (FR48),
`:1733-1738` (NFR-Sec-15, `gates: []`); `deferred-work.md:110` (NFR-Sec-15 v1.5 row), `:372` (D5 empty-string drift), `:1379` (FR38 runtime drift,
owner 20-13); ADR-069 `docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-48`; operator pages `docs/maos.dev/run-maos.md:17-27`,
`docs-site/docs/run-maos.md:20-26`, ko `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md`.

## P — pins at HEAD (`kloc-check --json`, `check-kernel-baseline --json` on the archive copy)

Kernel **26841 physical / 95 files / set hash `db39bc4a…`**, tokei **21042** (`xtask/kernel-core-baseline.toml:505`, `xtask/kloc.toml:264`).
maos-bin 34191 (`:517`), maos-domain 10099 (`:425`), maos-shell 615 (`:691`), maos-registry 4248 (`:688`), maos-compliance measured 2032 / pin
2117 (`:481`), maos-secrets 536 / pin 1000 (`:480`), maos-audit 7721 (`:409`), maos-capability 1433 (`:265`), maos-cli 6429 / pin 6856 (`:489`),
xtask 44406 (`:339`); aggregate **192519**, `_aggregate_hardfail` 192520 (`:718`): one line of aggregate room.
