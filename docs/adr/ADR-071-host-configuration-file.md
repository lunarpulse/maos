---
Status: PROPOSED — Story 22-1 (`22-1-host-config-and-standing-cast`), decisions D-22-1-01…10 taken at the recommended answers, operator not yet confirmed. The branch merges only after the `v1.0.0` GA tag.
Gate: `cargo test -p maos-bin --test host_config_22_1 --test region_reconcile_22_1`; `cargo test -p maos-domain --test data_root_22_1`; `cargo test -p xtask check_env_contract`; `cargo run -p xtask -- check-env-contract`
Decided: 2026-11-23
Accepted-in-PR: pending — Story 22-1
Revisits: ADR-067 (adds locations to the purge enumeration; the legacy leg stays); ADR-062 (the door is unchanged; the `home`/`door_bind` keys feed the same `MAOS_HOME` and `MAOS_OPERATOR_HTTP_BIND` names)
Supersedes: nothing
---

# ADR-071 — The Host configuration file

## Context

A long-lived Host was started from the command line and the environment, plus two older files
(`~/.config/maos/operator.toml` and `<home>/config.toml`). The standing set of Spirits was whatever the
installed-registry inventory and a `maos run` topology happened to name. Four stores resolved their
root four ways (memory: XDG then `$HOME/.local/share`; registry and yank cursor: `$HOME/.local/share`
only; migration plans: `MAOS_HOME` only). The env gate `check-env-contract` read only `crates/maos-bin/src`
and saw no read made through a `const` name, so `MAOS_REGION_HOME` and `MAOS_OPERATOR_HTTP_ENDPOINT`
were unregistered.

## Decision

1. **One file, one verb.** `maos serve --config <host.toml>` boots a Host from one TOML file. `serve` is a
   row of `VERBS` (network build). `maos run` is unchanged: it reads no Host file and keeps its empty
   installed list.
2. **The cast is the file's list** (D-22-1-03 A). `[[host.spirits]]` entries take the keys of
   `[[topology.spirits]]` (`manifest`, with `path` as an alias) plus `installed = "<name>"` for a
   registry-installed Spirit. A `host` key is refused (a cross-host target is not a cast member).
   Manifest members are fed to the existing topology loader; installed members to `InstalledSpirits::boot`.
   There is no restore mechanism: a restart brings the same cast back because the file names it. Installed
   Spirits the file does not list, and Spirits added with `maosctl load`, do not boot.
3. **Keys, and the names they map to** (D-22-1-04 A): `home` → `MAOS_HOME`, `door_bind` →
   `MAOS_OPERATOR_HTTP_BIND`, `region` → `MAOS_REGION_HOME`, `crypto_provider` → `MAOS_CRYPTO_PROVIDER`,
   `inference_mode` → `MAOS_INFERENCE_MODE`. The table is `host_config::HOST_SETTINGS`; every name is
   registered in `env_contract.rs`. Other variables stay env-only. A set environment name overrides its key for
   the process; a key fills only an unset name; the environment never stands in for a missing or invalid
   file.
4. **Refusals are typed and stable** (AC3): `host_config_missing`, `host_config_parse` (a TOML error or an
   unknown key, via `deny_unknown_fields`), `host_config_cast_entry`, `host_config_cast_host_key`,
   `host_config_setting`, `host_config_older_file_disagrees`, `host_config_installed_missing` and
   `host_config_installed_refused`. Each exits 78 before any store opens and before any Spirit loads. A
   cast manifest that parses but fails admission later is refused by the topology loader's admission
   gates, which tear down the members already loaded.
5. **Older files** (D-22-1-05 A). `operator.toml` `[region]` and the provider recorded in
   `<home>/config.toml` (default `ring`) are still read for keys the Host file omits. If one disagrees
   with a key the Host file sets, the boot is refused: "Disagreement is a boot error, never a warning."
6. **Region axis** (D-22-1-09 A; closes `deferred-work.md:926-935` and register row D4a). The cohort boot
   (`reconcile_transport_identity_with_manifest`, axis (e)) compares the value that stamps rows — the one
   passed to `.with_home_region` via `host_config::stamped_home_region()`, the kernel reader's result,
   which `serve` feeds from the `region` key — with the signed `TeamEntry.region` of this host's team.
   Pinning off passes; a set region with no signed team, or a mismatch, refuses
   (`cross_team_crossing::reconcile_home_region_with_manifest`). The kernel reader is untouched.
7. **`MAOS_REGION_HOME` is registered** (D-22-1-06 A) as `UserFacing` with an integrity note in
   `run-maos.md` (D-22-1-07 B). `check-env-contract` now scans every product crate and spirit under
   `crates/` and `spirits/` (xtask is harness; D-22-1-08 B), derived from `--maos-bin-dir` so the CI
   invocation is unchanged, and detects `const`/`static` `&str` names shaped `MAOS_*`.
8. **One data-root precedence** (D-22-1-10 A), `maos_domain::data_root`: an explicitly set `MAOS_HOME`,
   then `$XDG_DATA_HOME/maos`, then the legacy `$HOME/.local/share/maos` (`/var/lib/maos` when neither XDG
   nor HOME is set). A store (`memory`, `registry`, `registry/yank_cursor.json`, `upgrade-plans`) lives
   under the highest root that already holds it, else under the highest root. Data in a lower root is read in
   place; nothing is moved. Per-store overrides (`MAOS_MEMORY_ROOT`, `MAOS_REGISTRY_YANK_CURSOR_PATH`) stay
   above all roots. Both memory resolvers move together. `maos purge` enumerates every location a resolver
   can return; ADR-067's legacy leg stays.
9. **Read-back** (D-22-1-02 B). `maosctl agents` prints `<spirit_id>\t<lifecycle_state>` from
   `GET /v1/daemon`; Story 22-5 extends it.

## Consequences

- The kernel is untouched; kernel-core and its envelope do not move.
- `std::env::set_var` applies the overlay once, at the top of `serve`, before the crypto provider, stores or any
  Spirit is selected.
- An existing home whose registry sits under `$HOME/.local/share/maos` keeps working without a move. A home that
  sets `MAOS_HOME` and has no data yet now keeps the registry and memory under `$MAOS_HOME`.
