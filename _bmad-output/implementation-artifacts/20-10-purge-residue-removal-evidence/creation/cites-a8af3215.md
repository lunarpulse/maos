# 20-10 creation — cites re-measured at `a8af3215` (rule 9)

Every range below was re-read at HEAD `a8af3215` (`git log -1 -- crates xtask` = `47631284`; `a8af3215` is a
docs-only commit on top). "E20" = `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md`; "S" =
`sprint-status.yaml`; "PF" = `epic-20-preflight-2026-10-30.md`; "D" = `deferred-work.md`; "RA" =
`routing-audit-2026-10-22.md`. E20 §20-10 (`:274-277`) is corrected in place by this creation (C1–C3, marked
"re-measured at 20-10 creation"); every other row is owed through `records-owed.md` §A.

## C — epic §20-10 (E20:272-277) and its stories row (E20:83)

| # | Where | OLD | NEW at `a8af3215` | Applied |
|---|---|---|---|---|
| C1 | E20:276 (AC1) | report-only residue `crates/maos-bin/src/purge.rs:1950-1981` | `print_report_only_residue` `purge.rs:1950-1984`: cgroup leg `:1952-1957` names `/sys/fs/cgroup/maos` (never created at HEAD, see C6); container leg `:1969-1983` = `maos-` **name-prefix** filter over `podman`/`docker ps -a` names (`probe_container_names` `:2018-2049`, storage guard `:1994-2013`); called from the dry run `:931` and the confirmed run `:1041` | in place |
| C2 | E20:277 (AC2) | Worker workspace removal `purge.rs:1420-1427` | 19-3 path `purge.rs:1346-1454`: `remove_worker_workspaces` `:1354-1386`, per-child fallback `:1392-1415`, `workspace_removal_command` `:1420-1422`, bounded `podman unshare rm -rf --one-file-system` `:1427-1454`; dispatch in `remove_planned_roots` `:1319-1328`; dispositions `:1237-1240`; residue lines `:1958-1968`; roots `maos_roots` `:67-72` (`<home>/workspaces`), `:81-96` (`MAOS_WORKER_WORKSPACES_ROOT`) | in place |
| C3 | E20:277 | "Package-owned binary versus runtime-data contract awaits D09" | **D09 RULED A** (operator, rows201 "as already ruled"; recorded `20-1a-registry-publish-install-exit-path.md:162`): the package manager removes binaries, `maos purge` removes MAOS data and owned runtime residue. The binary hint `purge.rs:1042` (`cargo uninstall maos`, unconditional) is unchanged at HEAD and is 20-2's (deb/RPM legs) | in place |
| C4 | E20:277 | "receipts and idmaps" | receipts = ADR-067 receipt (`docs/adr/ADR-067-…md:71-74`; `ReceiptRoot`/`ReceiptRecord` `purge.rs:579-598`; keyring rows `keyring://maos/<key>` `:356-365`). **"idmaps" has no token anywhere in the repo** except E20:277 and PF:128; read as [INFERENCE] the rootless user-namespace mapping 19-3 relies on (`/etc/subuid`/`/etc/subgid`, podman storage/config) — never edited by purge; the only user-namespace operation stays 19-3's `podman unshare rm` | §A A3 |
| C5 | E20:274, :83 | "After 20-2" | 20-2 is `backlog` (S:298). The coupling was D09; D09 is ruled A, so the remaining overlap is the docs paragraph `docs/maos.dev/run-maos.md:105-121` (+ docs-site en `:105-121`, ko `:102-113`) and `purge.rs:1042` — D-20-10-09 | D-row |
| C6 | E20:276 "removes owned … cgroups"; S:308; RA:24,162 (from `16-4-…md:423-424`) | "owned cgroups" = the reported `/sys/fs/cgroup/maos` subtree | `/sys/fs/cgroup/maos/spirit-<pid>` is **never created at HEAD** (16-4 withdrew it, `16-4-maos-uninstall-and-keyring.md:180-183`; its writer `apply_resource_ceiling` and `scheduler/resource_ceiling.rs` no longer exist at HEAD — 0 hits for `apply_resource_ceiling` under `crates/`). MAOS-owned cgroups at HEAD are the native (T2) leaves `<nearest delegated ancestor>/maos.slice/spirit-<safe_id>-<creator pid>-<seq>` (`crates/maos-kernel-core/src/security/sandbox/linux.rs:448-482` root, `:484-517` create; slice `:492`, leaf `:511-514`), removed by `Cleanup::drop` (`sandbox/mod.rs:107-135`) — which never removes `maos.slice` and does not run when the creator is SIGKILLed. T3 container cgroups (`libpod-<cid>.scope`, `maos-domain/src/sandbox.rs:443-445`) are the runtime's and go with `podman rm` | §A A1 |
| C7 | S:308; RA:24 | "16-4's `maos purge` REPORTS owned T3 containers and cgroups but does not remove them (`16-4-…md:423-424`)" | same at HEAD (`16-4-…md:423-424` re-read) — plus C1/C6 detail: the report keys on the `maos-` name, which `argv.rs:16-21` says is never a cleanup authority | §A A2 |
| C8 | E20:274 "kernel-Δ 0 planned" | 0 | 0 under every recommendation; kernel-touching options priced by reverted prototype (`kloc-kernel-a8af3215-and-prototypes.json`): KH +10/+8, KR +2/−1, KC +5/+1, KS +3/+2, all four +20/+10 | §A A1 |

## P — preflight (PF) rows naming 20-10

| # | PF line | OLD | NEW |
|---|---|---|---|
| P1 | :84 | "Bin purge/T3 removal and package-owned-path contract unpriced" | priced at creation: maos-bin +180…+350 [INFERENCE], kernel 0 (options priced); package-owned path is 20-2's under D09 A |
| P2 | :128 | "add full epic section before authoring" | E20 §20-10 exists (`:272-277`, two ACs); this story expands it to AC1–AC7 without uncited scope (§Contract) |
| P3 | :183 | `purge.rs:1042` | same line at HEAD |
| P4 | :247 | `purge.rs:1950-1981`, `:1420-1427` | as C1, C2 |
| P5 | :293 (D09) | PROPOSED | RULED A (C3) |

## K — code facts the story relies on (all at `a8af3215`)

| # | Fact | Cite |
|---|---|---|
| K1 | T3 create argv: `--pidfile`/`--cidfile` into `<state_dir>`, labels `maos.spirit_id`, `maos.boot_nonce`, `--name`, workspace `--volume=<dir>:/workspace:rw,U`; the name "is never a cleanup authority" | `crates/maos-kernel-core/src/security/sandbox/t3/argv.rs:16-21,75-76,85-86,91,102-105` |
| K2 | Worker container name `maos-<worker_spirit_id>-<boot_nonce>`; workspace `<workspaces root>/<container name>` created before the state dir; `T3Launch` carries both | `crates/maos-bin/src/worker_spawn.rs:1060-1061,1066-1067,1077-1083,1117-1126`; `worker_workspace.rs:3-11,30-62,93-111` |
| K3 | Only other production T3 spawn: `maos-smoke-t3-<ns>` (no workspace; state dir removed in-process) | `crates/maos-bin/src/main.rs:8438-8521` |
| K4 | Worker state: per-euid root `$XDG_RUNTIME_DIR/maos-worker-state` else `/tmp/maos-worker-state-<euid>`, a random 0700 process root per `WorkerSupervisor`, a random 0700 spawn dir per launch holding `cid`/`pid`; "outside MAOS_HOME and ADR-067's purge inventory" | `crates/maos-bin/src/egress.rs:240-305`; supervisor create `supervision.rs:1826-1827`, per-spawn `:2218-2224`, cleanup `:2029-2041` (called `:2095` Drop, `:2858` root shutdown) |
| K5 | `ContainerResidue` (8 variants, snake_case serde) | `crates/maos-domain/src/sandbox.rs:535-547`; CID validation `parse_container_id` `:437` |
| K6 | `TerminationUnconfirmed` producers after 20-7 K1w/K2r: P1 joiner deadline while teardown in flight `child.rs:350-355`; P2 kill sent (pidfd K2r `:448` and/or runtime `:449`) but unconfirmed in `KILL_BUDGET` `:444-456` (overridden if rm+exists confirm absence `:496-500`); P3 no snapshot for this caller or in-flight owner did not publish by `overall` (K1w `snapshot_settled` `:396-412`) → return **before `rm`** `:458-462`; P4 launch custody: deadline while launch active, or ownership unknown `:602-662`. Residue line "state retained at <state_dir>" `:419-427`, `:690-707`; state dir removed only on `Ok` `:384-389` | `crates/maos-kernel-core/src/security/sandbox/t3/child.rs` |
| K7 | Supervisor defaults an unresolved container's residue to `TerminationUnconfirmed`; egress lease retained as "termination_unconfirmed residue" (in-memory) | `crates/maos-bin/src/supervision.rs:1144-1182,2022-2028`; `egress.rs:216-230` |
| K8 | `SandboxedContainerChild::adopt` is a `#[doc(hidden)]` test seam ("production owns CIDs only through `spawn_t3`"); `invoke_bounded` is `#[doc(hidden)]` | `child.rs:790-802`; `t3/spawn.rs:225-238` |
| K9 | Runtime client env hygiene: `env_clear` + PATH/HOME/XDG_RUNTIME_DIR/DBUS_SESSION_BUS_ADDRESS | `t3/spawn.rs:58-73` (purge's `podman unshare`/`ps` calls do not clear env: `purge.rs:1429-1435,2021-2027`) |
| K10 | Docker cannot be a T3 creator: the argv uses `--pidfile`, which `docker container create` does not accept (https://docs.docker.com/reference/cli/docker/container/create/); detection still offers docker | `argv.rs:75`; `t3/runtime_detect.rs:1-10` |
| K11 | Purge holds `StoreLockRole::OfflineExclusive`; every composition-root boot (`maos run`, `maos shell`, …) holds `RootShared` → no live root of this home exists while purge runs | `purge.rs:892-901`; `operator_door.rs:2630-2639`; `main.rs:2069-2071` |
| K12 | Purge receipt created before removal, rows updated as dispositions land, finalized `complete`/`partial` (partial only when an error note fired); 19-3 workspace residue rows are `left (subuid-owned residue)` under `complete` | `purge.rs:935-1019,1237-1240,1319-1328` |
| K13 | Existing real-runtime witness: `root_crash_retains_stopped_container_and_purge_reports_it` SIGKILLs a root mid-Worker and asserts the dry run prints `leaked container <name>` | `crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs:101-161` (`:150-157`); hosted in `epic-17-exit` (`.github/workflows/discipline.yml:2247-2262`, step `:2345-2364`, delegated unit via `scripts/run_cgroup_ceiling_17_2.py`, count pin `EPIC17_TERMINAL_SUITE_TESTS: 33` `:2262`) |
| K14 | Purge tests | `crates/maos-bin/tests/maos_uninstall_16_4.rs` (1975 lines; dry-run inert `:168`, live-store refusal `:1195`); `worker_workspace_19_3.rs` (hosted, `discipline.yml:3695-3698`) |
| K15 | ADR-067 "Container and cgroup residue is report-only: purge … does not invoke a container runtime" — **already false at HEAD**: purge invokes `podman ps` (`:2021`) and, since 19-3, `podman unshare` (`:1429`) | `docs/adr/ADR-067-maos-data-root-layout-and-purge.md:87-90` |
| K16 | FR2 coverage row has no gate and a v0.1 note | `tests/coverage-matrix.yaml:408-415` |
| K17 | Operator docs | `docs/maos.dev/run-maos.md:105-121`; `docs-site/docs/run-maos.md:105-121`; `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md:102-113`; cookbook `docs-site/docs/cookbook/cli-wrapper-spirit.md:63` (+ ko `:64`) |
| K18 | 20-7's own overlap note naming this story | `20-7-stable-runner-perf-gates.md:281` |
| K19 | Open flake in the purge test file, owner 20-7 | `deferred-work.md:13` (`purge_refuses_live_store_without_changing_filesystem_identity`, red under CPU load) |
