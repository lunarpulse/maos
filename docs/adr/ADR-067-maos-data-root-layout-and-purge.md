---
Status: ACCEPTED — ratified 2026-09-16 under Story 16-4, decisions D-16-4-C through D-16-4-I
Gate: `cargo test -p xtask --test decision_adrs_and_provisioning` and `cargo test -p maos-bin --test maos_uninstall_16_4`
Decided: 2026-09-16
Accepted-in-PR: pending — Story 16-4
Revisits: ADR-062 only for the shared `MAOS_HOME` discovery contract; it does not change the operator door
Supersedes: no ADR — replaces the false removal instruction formerly printed by `maos init`
---

# ADR-067 — MAOS data-root layout and what `maos purge` removes

## Context

MAOS state is not confined to `MAOS_HOME`. Production resolvers place the
Transparency Log, Lifecycle Journal, memory, erasure proofs, Spirit archives,
registry data, skills, a registry yank cursor, an audit signing key, and
certificate revocation lists under several environment-selectable roots.

The old `maos init` instruction printed `rm -rf <MAOS_HOME>`. With the default
resolver configuration that command removed configuration scaffolding but left
the durable data tree under `$XDG_DATA_HOME/maos` or
`$HOME/.local/share/maos`.

Some files under the discovered directories are operator-authored. In
particular, the publisher signing key and `operator.toml` are not MAOS output.
A correct inverse must distinguish authorship rather than deleting a directory
because MAOS wrote one sibling inside it.

## Decision

`maos_bin::purge::maos_roots` is the single typed enumeration of filesystem
state MAOS may write. Every entry records its path, whether it is a file or
directory, and whether it was written by MAOS or by the operator. Purge never
removes an operator-authored entry.

The enumeration delegates to production resolvers for `MAOS_HOME`, XDG data,
XDG configuration, the audit path, journal, memory, erasure proofs, Spirit
archives, CRLs, and the registry yank cursor. It also carries the permanent
`$HOME/.local/share/maos` legacy leg because the registry and skill stores still
use that location independently of `XDG_DATA_HOME`.

Provider credentials are removed by their closed `SecretKey` names through the
`SecretStore` port. Purge does not enumerate a user's credential collection and
cannot touch credentials belonging to a different MAOS-home namespace.

A destructive run requires `--yes`. Without it, `maos purge` reports the exact
plan and changes no user state. Purge acquires the existing offline-exclusive
store lock set before deletion and returns the established typed refusal when a
live daemon or another offline durable operation owns a store.

The air-gap build compiles the keyring stack out: `maos-bin` depends on
`maos-secrets` with `default-features = false` — encrypted-file everywhere, the
OS-keyring backend only on `cfg(unix)` — because the air-gap doctrine is
compile-out networking and the secret-service/zbus tree violates it; the
regular build is unchanged. On non-unix the default backend is `env` and
`Backend::Keyring` parses to a typed exit-78 refusal.

## Retention and receipts

Deletion is the default. `--keep-log` checkpoints SQLite first and retains the
resolved Transparency Log plus the default database when tenant mode separates
them. This necessarily preserves shared-memory rows, the principal namespace
index, capability-token rows, and legacy operator-authored `shell.turn` text;
the dry run reports those categories using counts read from the database.

Derived audit sidecars are not retained. Purge removes leaked SIEM snapshots,
export cursors, and air-gap stubs while preserving the checkpointed database and
its team binding. A surviving WAL is a refusal because the export reader cannot
safely consume that state.

Before removing state, purge creates an exclusive mode-0600 JSON receipt outside
every deletion root. It records `in-progress` first and finalizes `complete`
after removals. Signature and externally verifiable proof fields are present but
remain null until the v1.0 NFR-Aud-12 signing work lands.

Runtime rows use the same schema and convention: `podman://<CID>` for owned
containers, `cgroup://<path>` for cgroups, and directory paths for T3 custody.
Rows start `pending` before any removal and finish `removed` or `left (<residue>)`.
Like 19-3 workspace residue, surviving runtime residue alone does not change
`complete` to `partial`; an error note does. Read the rows, not only status.

## Consequences

Adding a new MAOS-written durable root now requires extending one typed function
and its root-enumeration test. Adding an operator-authored sibling requires an
explicit `WrittenBy::Operator` entry so removal behavior is reviewable at the
same boundary.

`maos init` names `maos purge` rather than reconstructing or printing paths.
Documentation describes the two-stage removal: purge MAOS-owned state, then run
`cargo uninstall maos` for the Cargo-owned executable.

ACP sockets have no cleanup entry because MAOS uses NDJSON over stdio. Purge
enumerates podman containers with `maos.spirit_id`, then checks both that label
and `maos.boot_nonce`, the exact `maos-<spirit_id>-<boot_nonce>` name, and the
`/workspace` mount source against this home's Worker workspace roots,
component-wise without filesystem canonicalization of runtime text. Only owned
64-hex CIDs reach bounded `rm --force --volumes --time=0` followed by
`container exists`; containers precede workspace deletion. MAOS-labelled
containers without that authority stay report-only with the exact removal
command; label-less containers are ignored even when named `maos-*`.

On Linux, purge rmdirs euid-owned `maos.slice/spirit-<id>-<creator pid>-<seq>`
leaves only when `cgroup.events` reports `populated 0` and the creator has no
`/proc/<pid>`, then the empty slice. It never kills members or writes
`cgroup.kill`. Private mode-0700, euid-owned T3 custody directories under the
runtime/temp state roots are reclaimed only after their CID is confirmed
absent; unknown/present-CID custody stays, followed by empty process/root
directory cleanup. Directory traversal and control-file reads do not follow
links. A dry run invokes only `ps`/`inspect` and changes no state, including
podman storage; spawn removal is reported as conditional on CID absence.
Purge never edits user-namespace mappings, podman configuration or storage.

## Verification

`maos_uninstall_16_4` exercises root discovery under isolated `HOME`,
`MAOS_HOME`, `XDG_DATA_HOME`, and `XDG_CONFIG_HOME`; dry-run safety; confirmed
removal; operator-file preservation; lock refusal; receipt placement; and
`--keep-log` checkpoint behavior.

`purge_residue_20_10` binds container authority/refusal twins, dry-run verbs,
receipt ordering, custody, and the post-removal residue vocabulary. Unit
`ac4_dead_empty_live_creator_populated_and_symlink_twins` binds the temp cgroup
walk. The hosted `epic-17-exit` root-crash scene removes real retained CIDs and
custody while preserving another home's Worker and an impostor;
`purge_cgroup_native_binding_and_dead_creator_twins` binds kernel naming and
real delegated empty/populated leaves.

The verb-table tests bind `purge` in network and air-gap builds. The environment
contract gate binds `MAOS_SECRETS_BACKEND` and `MAOS_CRL_PATH` at the composition
root. The kernel baseline must remain byte-identical because enumeration,
locking orchestration, and deletion live outside `maos-kernel-core`.
