# 20-10 creation — measurements

## Instrument

Prebuilt `/mnt/build/cargo-maos/debug/xtask` (`$CARGO_TARGET_DIR`, inherited; no cargo ran), invoked as
`env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=<copy>/xtask <xtask> {check-kernel-baseline,kloc-check} --json` with
cwd = a `git archive a8af3215` copy per prototype under `/tmp/proto2010/` (deleted after measurement; `git status --
crates xtask` clean before and after). Prototypes were edited by scripted exact-substring replacement, formatted with
`rustfmt 1.9.0 --edition 2021` (the diffs show only the intended hunks), never applied to the tree, never compiled.
Raw figures: `kloc-kernel-a8af3215-and-prototypes.json`; diffs `proto-*.diff`.

## Baseline (untouched copy)

`check-kernel-baseline` passed: **26841** physical, 95 files, set hash `db39bc4a…`; `kloc-check` passed: kernel
**21042** tokei, maos-bin **33498**, maos-domain 10099, xtask 44406, aggregate **191826**, `alarm = true`,
`over_budget = []`. Matches the pins (`xtask/kloc.toml:264,339,425,517`; aggregate hardfail 191827).

## Kernel prototypes (each FLAG-Winston if ruled; none recommended)

| Shape | D-row option | Files | Physical | Tokei | Diff |
|---|---|---|---:|---:|---|
| **KH** — `maos.owner=<id>` label: new `owner` field on `T3SpawnContext` and `T3Launch`, one argv push | D-20-10-01 B | `t3/spawn.rs`, `t3/argv.rs`, `cli_wrapper/runtime.rs` | +10 (26851) | +8 (21050) | `proto-kh.diff` |
| **KR** — promote the `#[doc(hidden)]` `adopt` test seam to a production `reclaim` constructor so purge runs the kernel's own teardown | D-20-10-02 B | `t3/child.rs` | +2 (26843) | −1 (21041) | `proto-kr.diff` |
| **KC** — export `CGROUP_SLICE` / `CGROUP_LEAF_PREFIX` and use them in `create_cgroup_dir` | D-20-10-05 B | `sandbox/linux.rs` | +5 (26846) | +1 (21043) | `proto-kc.diff` |
| **KS** — `Cleanup::drop` removes the empty `maos.slice` after its leaf (EBUSY while a sibling lives) | D-20-10-04 D | `sandbox/mod.rs` | +3 (26844) | +2 (21044) | `proto-ks.diff` |
| KC + KS | — | both | +8 (26849) | +3 (21045) | `proto-kc_ks.diff` |
| KH + KR | — | four | +12 (26853) | +7 (21049) | `proto-kh_kr.diff` |
| all four | — | six | +20 (26861) | +10 (21052) | (sum of the above; JSON key `all`) |

Every shape moves both kernel pins and the content hashes of the touched files. ADR-069
(`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`): the largest bundle reaches 21052 tokei, 948 under the
22000 alarm; no ENVELOPE ALARM row. Kernel interaction: 20-7's K1w + K2r is landed (it is the HEAD 26841/21042);
20-1b's R = KQ1 + P (+78/+68, computed from 26801/21011) re-bases on landing (cumulative 21110 expected, ≤ 21209).
If any 20-10 kernel option is ruled, its row is computed from HEAD 26841/21042 and the later of 20-1b / 20-10 to land
re-bases; cumulative tokei with every 20-10 shape + 20-1b = 21042 + 68 + 10 = 21120 ≤ 21209.

KR caveat (read from `child.rs:438-506`, not run): on an already-exited reclaimed CID (`started = true`, no pidfd)
the runtime `kill` fails, `kill_failed` latches, and a clean verdict still prints "CID kill failed; later cleanup
outcome is reported separately" (`:373-392,429-436`) — purge output noise the kernel shape would need more lines to
silence; the snapshot stage (`SNAPSHOT_BUDGET` 10 s) serves the exit journal row purge never writes.

## Non-kernel forecasts (not measured — [INFERENCE] from analogues)

Analogue code lines counted (non-blank, non-comment) in `purge.rs` at HEAD: `remove_worker_workspaces` 29,
`podman_unshare_remove_children` 24, `podman_unshare_remove` 28, `print_report_only_residue` 35,
`podman_storage_exists` 20, `probe_container_names` 32 (19-3's whole path 84).

| maos-bin component (recommended options) | Forecast (tokei) | Basis |
|---|---:|---|
| container enumeration (`ps --filter label`, `inspect` labels/name/mounts, serde_json) + attribution (D-01 A) | +70…+110 | `probe_container_names` 32 + parsing |
| removal (`rm --force --volumes --time=0`, `container exists`), verdict → `ContainerResidue`, receipt rows (D-02 A) | +35…+55 | `podman_unshare_remove` 28 |
| one bounded runtime helper replacing the two `try_wait` loops (`:1440-1453`, `:2029-2042`) with env hygiene | −10…+10 | net of two loops removed |
| cgroup walk, leaf attribution, rmdir, report (D-04 A, D-05 A) | +60…+90 | `remove_worker_workspaces` + children 53 |
| retained T3 state reclaim (D-03 A) | +35…+55 | `podman_unshare_remove_children` 24 + validation (`egress.rs:273-288` shape) |
| producer process-root leak repair (D-07 A) | +5…+30 | cause undiagnosed (measurement 5) |
| dry-run/report rewrite; `/sys/fs/cgroup/maos` leg and docker leg deleted (D-06 A) | +20…+40, −40 | `print_report_only_residue` 35 |
| **maos-bin total** | **+180…+350** | sum |
| maos-domain (reuses `ContainerResidue`, `parse_container_id`), xtask, kernel | 0 | — |
| tests, docs, workflows | uncharged | — |

## Host probes at creation (read-only)

- `podman ps -a --filter label=maos.spirit_id …` on the dev host (podman 5.4.2): `cannot clone: Operation not
  permitted` / `cannot re-exec process` — **rootless podman cannot run here**; every real-runtime leg is hosted
  (`epic-17-exit`). `/proc/self/cgroup` = `0::/` (no delegated subtree): the cgroup leg is hosted too.
- `/tmp/maos-worker-state-1000` (mode 0700, owner 1000): **8,903 empty process roots**, no spawn dirs, no `cid`
  receipts; oldest 2026-10-18, newest 2026-11-02 12:40; per day 2026-10-26…11-02: 354, 466, 202, 631, 1056, 1195,
  1051, 3185. `$XDG_RUNTIME_DIR/maos-worker-state` absent. The supervisor creates one process root per
  `WorkerSupervisor` (`supervision.rs:1826-1827`) and removes it only in `Drop`/root shutdown
  (`:2029-2041,2095,2858`) — so some exit path skips both ([INFERENCE]: a process exit without the drop; which
  binary — tests or `maos` one-shots — is not determined). Evidence for D-20-10-03 and D-20-10-07.

## Not measured here (owed at T0; a measured figure above the ruled envelope, or a T0 result contradicting a ruled option, is STOP + D-row)

1. `podman inspect --format '{{json .Mounts}}'` on a Worker container (podman 4.9.3 and 5.7, `ubuntu-24.04`/`26.04`):
   is `Source` the path passed to `--volume` verbatim, or canonicalised? (Decides the attribution comparison, D-01 A.)
2. `podman ps -a` with two `--filter label=…` flags: AND or OR on 4.9.3 vs 5.7 (the code must filter one label and
   check both in-process if they differ).
3. `podman rm --force --volumes --time=0` and `container exists` on an exited and on a running retained Worker
   container, rootless, both images (exit codes, stderr "volume" wording that `child.rs:479-482` keys on).
4. Whether `MAOS_T3_RUNTIME=docker` can create a T3 container at all (expected refusal: `--pidfile`, K10) —
   decides D-20-10-06 A.
5. Which exit paths leave a process root under the worker state root (run `maos` one-shots / `maos run --once`
   and the maos-bin T3 tests with a private `TMPDIR`; count roots before/after) — decides D-20-10-07's repair.
6. On the delegated hosted unit: path and ownership of `maos.slice` after a native spawn and after a SIGKILLed
   creator (`run_cgroup_ceiling_17_2.py --mode delegated`), and `cgroup.events` `populated` for an emptied leaf.
7. Compilation of the bin change and any ruled kernel shape (`cargo check`/`clippy`), and the final `kloc-check`.
