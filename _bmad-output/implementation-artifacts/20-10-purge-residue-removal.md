---
story_id: '20.10'
story_key: 20-10-purge-residue-removal
created: 2026-11-02
context_commit: a8af3215
baseline_commit: a8af3215
security_sensitive: true
kernel_grant: "RULED 2026-11-02 (ask s2010): kernel-Δ 0; no kernel grant. HEAD pins 26841 physical / 21042 tokei."
kloc_grant: "RULED 2026-11-02 (ask s2010; D-20-10-10 A; D02 A ×3 unmeasured). Ceilings (STOP above; pins re-fixed exactly at landing; aggregate hardfail = measured + 1): maos-bin ≤ 34548; aggregate ≤ 192876; every other crate 0. The later maos-bin lander re-bases its draw on the then-current pin."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:67-71`): security-critical — an irreversible destructive command whose ownership rule decides whether another MAOS home's Workers or a foreign container are killed (cross-home isolation), plus a concurrency boundary (a live creator racing cgroup removal). Blind and Edge on opus-5.5, Acceptance on opus-5.5, a Security reading on AC1, AC2, AC4 and AC5, Test Infrastructure on glm-5.3 (standing policy `proof-contract.md:57-71`; any session substitution recorded at review). A FLAG-Winston reading is added only if a kernel option is ruled. Creation anthropic/claude-opus-5-5."
---

# Story 20.10: Purge removes owned T3 runtime residue

Status: ready-for-dev

<!-- Created 2026-11-02 at a8af3215 (main, docs-only on top of 47631284 = `git log -1 -- crates xtask`) by bmad-create-story under the lighter measured creation contract. Operator rulings in force: D01 A, D02 A, D11 A (2026-10-30); D09 A (rows201, recorded at `20-1a-…md:162`). D-20-10-01…10 are PROPOSED, not ruled — the operator rules; a different ruling rewrites only the clauses marked with that row. No cargo ran: cites re-read at HEAD (`creation/cites-a8af3215.md`); kernel options and kloc by the prebuilt gate binary on `git archive` copies (`creation/measurements.md`, `creation/kloc-kernel-a8af3215-and-prototypes.json`, `creation/proto-*.diff`); stale E20 §20-10 cites corrected in place (C1–C3). -->

**Status rationale — ready-for-dev, not blocked.** D-20-10-09 B (RULED 2026-11-02, ask s2010) supersedes the
old "after 20-2" sequence: start when picked. D09 A assigns binaries to package managers and MAOS data/owned
runtime residue. What remains shared is one docs paragraph and the binary-hint line `purge.rs:1042`, both left to
20-2 here; the later lander re-bases the shared docs paragraph. Every real-runtime leg is hosted: rootless podman cannot run on the
dev host (`cannot clone: Operation not permitted`, `creation/measurements.md`) and it has no delegated cgroup.

> **The capability:** *after `maos purge --yes`, no container, cgroup or T3 custody record that this MAOS home created is left on the host — each one is removed and receipted, or named with the exact command when MAOS cannot prove it is this home's or cannot remove it — and nothing MAOS did not create, and nothing another MAOS home still uses, is touched.*

**Closes:**
- Epic-20 §20-10 (E20@HEAD:272-277) as amended 2026-10-30 (D01 A; PF §3.5), cites corrected in place at creation:
  epic AC1 → AC1–AC5 here, epic AC2 → AC6; AC7 = records.
- FR2 for owned runtime residue ("uninstall … without leaving orphaned state", PRD
  `functional-requirements.md:25`); routing audit `routing-audit-2026-10-22.md:24,162`; tracker S:308;
  `16-4-maos-uninstall-and-keyring.md:423-424` (containers and cgroups reported, not removed).
- 20-7's overlap note (`20-7-stable-runner-perf-gates.md:281`): what `TerminationUnconfirmed` means to purge (AC5.1).

**Does NOT close, and says so:**
- Package-owned binaries and the unconditional `cargo uninstall maos` hint (`purge.rs:1042`) — D09 A, 20-2's deb/RPM legs.
- Per-Spirit erasure (FR65, the `uninstall` one-shot, `main.rs:7360-7399`) — 20-11. Signed receipts (NFR-Aud-12): the
  receipt's `signature`/`proof` fields stay null (ADR-067:71-74).
- Containers MAOS cannot attribute to this home (another home's, pre-19-3 runs, `maos-smoke-t3-*`): named with the
  exact command, never removed (AC2). Processes inside a populated cgroup leaf: reported, never killed (AC4).
- T3 support for docker (detection offers it, the T3 argv cannot create a docker container — K10; D-20-10-06).

## Story

As **an operator uninstalling MAOS from a host that ran Workers**,
I want **`maos purge --yes` to remove the T3 containers, cgroup leaves and T3 custody records this MAOS home left
behind — proving each one is this home's first — and a dry run that lists exactly that and changes nothing**,
so that **an uninstall leaves no orphaned runtime state, and never kills or deletes what MAOS did not create or what
another MAOS home is still using.**

**Contract:**
- Epic §20-10 (E20@HEAD:272-277) as amended 2026-10-30 (D01 A; PF:128,247) — create-story tightens, does not add
  uncited scope: AC4's real cgroup set and AC5's retained T3 state are what "owned … cgroups" and the 17-7/20-7
  "state retained" residue mean at HEAD (C6, K4, K6); AC7 is the records the change makes false.
- Tracker `sprint-status.yaml:308`; E20 confidence rules 6, 9–11; proof contract `docs/testing/proof-contract.md:10-31`
  (one final-tree run + one plant per AC), `:33-71` (review, tiers); ADR-067 (purge) is amended by AC7.
- D09 A (rows201): purge removes MAOS data and owned runtime residue; binaries are the package manager's.

## Inventory at HEAD — measured, not assumed

Method: every cite re-read at `a8af3215` (`creation/cites-a8af3215.md`); host probes read-only
(`creation/measurements.md`); no cargo, no test run.

| # | Residue | Disposition at HEAD | Code | Witness / measurement | AC |
|---|---|---|---|---|---|
| 1 | Worker containers retained after a crash or an unconfirmed teardown | **report-only**, keyed on the `maos-` **name prefix** over `podman`/`docker ps -a` | `purge.rs:1969-1983`, `:2018-2049`; name ≠ authority `argv.rs:16-21` | `root_crash.rs:150-157` asserts the dry-run line (hosted `epic-17-exit`) | AC1, AC2 |
| 2 | Native (T2) cgroup leaves `maos.slice/spirit-<id>-<creator pid>-<seq>` and `maos.slice` | **unknown to purge**; purge reports `/sys/fs/cgroup/maos`, which nothing creates | `purge.rs:1952-1957`; real set `linux.rs:448-517`; `Cleanup::drop` removes the leaf only, not when the creator is SIGKILLed `sandbox/mod.rs:107-135` | none | AC4 |
| 3 | T3 custody: spawn dirs (`cid`, `pid`) and process roots under `$XDG_RUNTIME_DIR/maos-worker-state` or `/tmp/maos-worker-state-<euid>` | **outside purge by design** ("outside … ADR-067's purge inventory") | `egress.rs:240-305`; `supervision.rs:1826-1827,2029-2041`; "state retained at" `child.rs:419-427,690-707` | dev host: **8,903 empty process roots** in `/tmp/maos-worker-state-1000`, 2026-10-18 → 11-02, up to 3,185/day | AC5 |
| 4 | Worker workspaces (19-3) | removed, `podman unshare` fallback, residue line | `purge.rs:1346-1454` | `worker_workspace_19_3` (hosted `discipline.yml:3695-3698`) | AC6 (preserve) |
| 5 | Receipt | created before removal, rows updated, `complete`/`partial` | `purge.rs:579-1019` | `maos_uninstall_16_4` | AC1, AC6 |
| 6 | Dry-run inertness incl. podman storage | inert; `ps` gated on existing storage (D:1069) | `purge.rs:910-933,1994-2013` | `maos_uninstall_16_4.rs:168` | AC3 |
| 7 | Live-root refusal | `OfflineExclusive` lock before any step | `purge.rs:892-901`; roles `operator_door.rs:2630-2639` | `maos_uninstall_16_4.rs:1195` (flaky under load, D:13, owner 20-7) | AC2.3 |
| 8 | Docker leg | dead [INFERENCE]: T3 argv uses `--pidfile` (`argv.rs:75`), which `docker create` rejects | `purge.rs:1969` | measurement 4 | D-20-10-06 |

## Current-tree corrections — read before implementing

Full OLD → NEW: `creation/cites-a8af3215.md` (C1–C8, P1–P5, K1–K19).

| Premise (epic / preflight / tracker / ADR) | Fact at `a8af3215` and consequence |
|---|---|
| Report-only cite `purge.rs:1950-1981` (E20:276) | `:1950-1984`; containers `:1969-1983` by name prefix, cgroups `:1952-1957` (C1) — corrected in place. |
| Workspace removal `purge.rs:1420-1427` (E20:277) | the 19-3 path is `:1346-1454` (C2) — corrected in place. |
| "awaits D09" (E20:277) | D09 RULED A (C3) — corrected in place. |
| "owned … cgroups" = `/sys/fs/cgroup/maos` (16-4:423-424, S:308, RA:24) | never created (16-4:180-183; writer gone); the owned set is `maos.slice/spirit-*` (C6). |
| "idmaps" (E20:277, PF:128) | no repo token; read as the rootless user-namespace mapping 19-3 relies on — purge never edits it (C4, AC6.3). |
| ADR-067:87-90 "does not invoke a container runtime" | already false: `podman ps` (`purge.rs:2021`) and 19-3's `podman unshare` (`:1429`) (K15); AC7 rewrites it. |
| `egress.rs:240-241` "outside … ADR-067's purge inventory" | the retained custody accumulates (row 3); D-20-10-03. |

## The `TerminationUnconfirmed` contract (coordination with 20-7's K1w/K2r)

20-7 landing 1 (`1fd4de31`) put K1w (`snapshot_settled`, `child.rs:396-412`) and K2r (`kill_payload`, `:282-294`)
into `t3/child.rs`; HEAD is that code. `ContainerResidue::TerminationUnconfirmed` (`maos-domain/src/sandbox.rs:535-547`)
is produced at four sites and means: **MAOS could not establish, before its deadline, that the container's payload
is gone or could not finish the pre-removal evidence; it did not remove the CID, and the CID may still exist with a
live payload.** It never means "absent" and never means "alive".

| # | Producer | What is retained | Purge (this story) |
|---|---|---|---|
| P1 | a teardown joiner whose own deadline expired while another caller's teardown was in flight (`child.rs:350-355`) | whatever the in-flight teardown leaves | treated as P2/P3 below |
| P2 | started, kill sent (pidfd SIGKILL K2r `:448` and/or runtime `kill` `:449`) but neither pidfd nor `inspect` confirmed termination within `KILL_BUDGET` (`:444-456`); cleared if `rm` + `exists` later confirm absence (`:496-500`) | container (maybe running), spawn dir with `cid`/`pid` (`:384-389` removes it only on `Ok`) | container: attributed by labels + workspace mount (AC1); removal is `rm --force` (kills), then `container exists`; spawn dir reclaimed once the CID is absent (AC5) |
| P3 | no terminal snapshot for this caller, or the in-flight snapshot owner did not publish by `overall` (K1w) → returns **before `rm`** (`:458-462`) | container (usually exited), spawn dir | as P2 |
| P4 | launch custody: shutdown deadline while launch active, or ownership unknown (create committed, no CID receipt) (`:602-662`) | possibly a container with **no `cid` receipt**, spawn dir without `cid` | container found only by labels + mount (AC1 — why D-20-10-01 D, receipts only, is not enough); a spawn dir without `cid` is kept and reported (AC5) |

Also: the supervisor defaults an unresolved container to `TerminationUnconfirmed` (`supervision.rs:1161,1180`); the
egress lease's "termination_unconfirmed residue" (`egress.rs:216-230`) is in-memory and dies with the process.

Purge **consumes and never redefines** this vocabulary. Its own per-container outcome uses only the post-removal
variants `remove_failed`, `remove_timeout`, `still_exists`, `exists_failed`, `volume_cleanup_unconfirmed` (serde
snake_case) — never `termination_unconfirmed`, `kill_failed` or `runtime_client_unconfirmed`, which are facts about a
supervised payload's teardown stages that purge does not run (D-20-10-02 A). No line of `child.rs` changes; a later
story that changes a producer above updates this table.

## Kernel and kloc pricing — measured at creation

**Instrument:** prebuilt `/mnt/build/cargo-maos/debug/xtask` (`check-kernel-baseline --json`, `kloc-check --json`) on
`git archive a8af3215` copies; the untouched copy reproduces the pins exactly (26841 / 95 files / `db39bc4a…`; kernel
21042 tokei; maos-bin 33498; aggregate 191826). Prototypes rustfmt'd, never applied, never compiled.

**Kernel (no shape recommended; each is FLAG-Winston only if ruled):**

| Shape | For | Kernel physical | Kernel tokei | Diff |
|---|---|---:|---:|---|
| KH `maos.owner` label (`T3SpawnContext`/`T3Launch` field, one argv push) | D-01 B | +10 (26851) | +8 (21050) | `proto-kh.diff` |
| KR `reclaim` (the `#[doc(hidden)]` `adopt` seam made a production constructor) | D-02 B | +2 (26843) | −1 (21041) | `proto-kr.diff` |
| KC export `CGROUP_SLICE`/`CGROUP_LEAF_PREFIX` | D-05 B | +5 (26846) | +1 (21043) | `proto-kc.diff` |
| KS `Cleanup::drop` removes the empty `maos.slice` | D-04 D | +3 (26844) | +2 (21044) | `proto-ks.diff` |
| all four | — | +20 (26861) | +10 (21052) | JSON `all` |

ADR-069 (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`): the largest bundle is 948 under the 22000 alarm.

**kloc forecast** (tokei; HEAD = every pin, zero headroom):

| Crate · component | Forecast | Basis |
|---|---:|---|
| maos-bin · enumeration + attribution (AC1, AC2) | +70…+110 [INFERENCE] | `probe_container_names` 32 code lines |
| maos-bin · removal, verdicts, receipt rows (AC1) | +35…+55 [INFERENCE] | `podman_unshare_remove` 28 |
| maos-bin · one bounded runtime helper replacing two wait loops | −10…+10 [INFERENCE] | `purge.rs:1440-1453,2029-2042` |
| maos-bin · cgroups (AC4) | +60…+90 [INFERENCE] | 19-3 removal path 53 |
| maos-bin · T3 custody reclaim (AC5) + producer leak (D-07 A) | +40…+85 [INFERENCE] | `egress.rs:273-305` shape |
| maos-bin · dry-run/report rewrite; dead legs deleted | −20…0 [INFERENCE] | `print_report_only_residue` 35 |
| maos-domain, xtask, kernel | 0 | reuse `ContainerResidue`, `parse_container_id` |
| tests, docs, workflows | uncharged | — |
| **maos-bin / aggregate** | **+180…+350** | sum (`creation/measurements.md`) |

**Envelope (D-20-10-10 A, PROPOSED; D02 A ×3 unmeasured / ×2 measured upper; ceilings, not reservations):**

| Crate | HEAD | Forecast | Ceiling with allowance | Inherited room | Operator ruling needed |
|---|---:|---:|---:|---:|---|
| maos-bin | 33498 | +180…+350 | **34548** (+1050) | 0 | yes |
| maos-kernel-core (physical / tokei) | 26841 / 21042 | 0 | 0 (a ruled shape: its row ×2) | 0 | only if a kernel option is ruled |
| aggregate | 191826 | +180…+350 | **192876**; hardfail = measured + 1 | 0 | yes |

**Operator-ruling list (PROPOSED, not granted):** (1) D-20-10-01…10; (2) maos-bin envelope; (3) aggregate hardfail;
(4) a FLAG-Winston row only if a kernel option is chosen. Each figure lands exact with a HISTORY row citing this key.

## Decisions — RULED 2026-11-02 (ask s2010)

Preflight rows that bear on this story: D09 RULED A (rows201; scope above), D02 RULED A (envelope form), D11 RULED A
(kloc HISTORY form at landing).

**Rulings 2026-11-02 (ask s2010):** The operator ruled every D-20-10-01…10 row exactly as recommended, including D-20-10-09 B. Kernel-Δ 0 and the ceilings are recorded in frontmatter.

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-20-10-01** | What proves a container is this home's | **A** label pair `maos.spirit_id`/`maos.boot_nonce` present, name = `maos-<spirit_id>-<boot_nonce>` (`worker_spawn.rs:1061`), and its `/workspace` mount source = `<one of this purge's Worker workspaces roots>/<name>` (`argv.rs:102-105`; roots `purge.rs:67-96`); kernel 0 · **B** KH: a `maos.owner=<home digest>` label at create (+10/+8 kernel, maos-bin passes it), A's rule kept for containers created before it · **C** label pair only, every MAOS home of this user · **D** CID receipts under the worker state root | **A.** Uses identity MAOS already writes, scoped to this home by a path only this home's runs mount. C kills a second home's live Workers (the store lock covers one home only, K11). D misses P4 containers with no receipt and every container whose receipt tmpfs was cleared by logout/reboot. B is cleaner for future non-Worker T3 spawns but buys nothing for today's only production T3 spawner (K2, K3) and costs a kernel grant. | RULED 2026-11-02: **A** (ask s2010) |
| **D-20-10-02** | How purge removes an owned container | **A** maos-bin: bounded `podman rm --force --volumes --time=0 <CID>` then `podman container exists <CID>` (1 = absent); outcome in the post-removal `ContainerResidue` subset; kernel 0 · **B** KR (+2/−1): reclaim custody through a production `reclaim` constructor and run the kernel's own teardown (kill → confirm → snapshot → `rm` → `exists`) | **A.** Purge needs absence, not terminal facts: the kernel teardown's snapshot stage (up to `SNAPSHOT_BUDGET` 10 s per CID) serves the exit journal row purge never writes, and on an exited CID it prints a misleading "CID kill failed" line (`child.rs:373-392,429-436`). B keeps one teardown implementation and makes purge's verdicts the kernel's verbatim, at a FLAG-Winston grant. | RULED 2026-11-02: **A** (ask s2010) |
| **D-20-10-03** | Retained T3 custody under the worker state root | **A** reclaim: after AC1, remove each spawn dir whose `cid` names a CID `container exists` reports absent, then empty process roots, then the empty state root; keep and report spawn dirs without `cid` (P4) or naming a CID that still exists; every directory validated as `egress.rs:273-288` does (0700, owner euid, not a symlink) · **B** report-only · **C** leave (volatile `/tmp` / `XDG_RUNTIME_DIR`) | **A.** FR2 "without orphaned state"; it is exactly the residue 17-7/20-7 tell the operator about ("state retained at"), and the dev host holds 8,903 empty roots. The root is per-euid, shared by homes: removing only absent-CID and empty dirs cannot take a live home's custody. C fails FR2 where `/tmp` is persistent. | RULED 2026-11-02: **A** (ask s2010) |
| **D-20-10-04** | Owned cgroup leaves | **A** rmdir a `maos.slice/spirit-*` leaf owned by this euid iff `cgroup.events` says `populated 0` **and** its creator pid (from the name) has no `/proc/<pid>`; then `maos.slice` iff empty; populated or live-creator leaves reported with member pids and the exact commands; never `cgroup.kill` · **B** A + `cgroup.kill` populated dead-creator leaves (orphaned native Spirits) · **C** report-only with the corrected paths · **D** A + KS (+3/+2) so a normal exit stops leaking `maos.slice` | **A.** Leaves carry no home identity, so killing (B) can hit another home's live Spirit after pid reuse; the creator-alive check keeps purge from racing a live creator's empty-leaf window (`linux.rs:440-445`). D fixes a leak at source that A already cleans at uninstall, for a kernel grant. | RULED 2026-11-02: **A** (ask s2010) |
| **D-20-10-05** | Where the cgroup naming lives | **A** the two literals in maos-bin, bound by a test that spawns a native child through the kernel and asserts purge's matcher accepts `SandboxedChild::cgroup_path()` (`sandbox/mod.rs:139-145`) · **B** KC: kernel exports the constants (+5/+1) | **A.** The binding test reds on drift without a kernel line; B is the single-source answer at a FLAG-Winston grant. | RULED 2026-11-02: **A** (ask s2010) |
| **D-20-10-06** | The docker probe (`purge.rs:1969`) | **A** delete it once T0 measurement 4 confirms `MAOS_T3_RUNTIME=docker` cannot create a T3 container (`--pidfile`, `argv.rs:75`; docs.docker.com `container create`) · **B** keep docker report-only · **C** extend removal to docker | **A.** Dead weight that prints residue MAOS cannot have created; a podman-docker shim is podman and is covered by the podman leg. If measurement 4 shows docker T3 works, STOP + re-ask (C). | RULED 2026-11-02: **A** (ask s2010) |
| **D-20-10-07** | The process-root leak at its producer (8,903 measured) | **A** diagnose (measurement 5) and repair in maos-bin within this story: every exit path that ends a `WorkerSupervisor`'s process removes its empty process root · **B** a `deferred-work.md` row with an owner the operator names; 20-10 reclaims at purge only · **C** purge-time reclaim only, no row | **A.** Same crate, same residue class, small forecast (+5…+30); without it the root regrows by up to 3,185 dirs/day between purges. B if T0 shows the leak lives in test harnesses only (then it is a test-hygiene row, not FR2). | RULED 2026-11-02: **A** (ask s2010) |
| **D-20-10-08** | Receipt status when owned runtime residue survives | **A** the 19-3 convention: rows `left (<residue>)`, status `complete` unless an error note fired (`purge.rs:986-1012`, `:1237-1240`) · **B** status `partial` whenever an owned container, leaf or custody dir survives (and the 19-3 workspace residue re-labelled the same way) | **A.** One convention in one receipt; the rows carry the residue. B is more conspicuous but changes 19-3's shipped meaning. | RULED 2026-11-02: **A** (ask s2010) |
| **D-20-10-09** | Start before 20-2 | **A** wait for 20-2 `done` (E20:274) · **B** start when picked: no `packaging/` file, `purge.rs:1042` and the binary-removal sentence of the run-maos docs left to 20-2; the later lander rebases the shared docs paragraph | **B.** The ordering existed for D09, now ruled; the overlap is one docs paragraph. | RULED 2026-11-02: **B** (ask s2010) |
| **D-20-10-10** | Per-crate envelope (D02 A applied) | **A** the envelope above (×3 unmeasured / ×2 measured) · **B** exact measured grants only, every review correction re-asks · **C** allowance ×1.3 (rule 6) | **A.** Precedent: 20-3b/20-3c landed at 2–3× creation forecasts (`20-7-…md:148`); all of this story's figures are unmeasured. | RULED 2026-11-02: **A** (ask s2010) |

## Acceptance Criteria

*(Epic §20-10 as amended 2026-10-30, under the recommended options; **local** = hermetic dev host with a stub
runtime on `PATH` or a temp-dir cgroup tree; **hosted** = GitHub-hosted run with rootless podman and a delegated
unit — `epic-17-exit`, `ubuntu-24.04` and `ubuntu-26.04`.)*

### AC1 — `maos purge --yes` removes this home's owned T3 containers (epic AC1; D-01 A, D-02 A)

1. After the store lock and before the receipt, purge enumerates candidates read-only: `podman ps -a` filtered on the
   MAOS labels, then `podman inspect` per candidate (labels, name, mounts). A candidate is **owned** iff its CID passes
   `maos_domain::sandbox::parse_container_id` (`sandbox.rs:437`), its name is `maos-<maos.spirit_id>-<maos.boot_nonce>`,
   and it has one mount with destination `/workspace` whose source equals `<root>/<name>` for a `RootKind::WorkerWorkspaces`
   root of this purge (`purge.rs:67-96`), compared component-wise in the form measurement 1 establishes — runtime-supplied
   text is never canonicalised through the filesystem and never used as a path to delete.
2. Each owned CID: `podman rm --force --volumes --time=0 <CID>`, then `podman container exists <CID>` (exit 1 = absent),
   each under a deadline, through **one** bounded runtime helper that replaces the two wait loops (`purge.rs:1440-1453`,
   `:2029-2042`) and runs the client with the environment `t3/spawn.rs:58-73` allows. CIDs, never names, go to the runtime.
3. Containers go before the Worker workspaces roots, so no removed Worker still holds or writes its `/workspace`.
4. One receipt row per owned container, created `pending` with the other rows before any removal: `path` =
   `podman://<CID>` (the `keyring://` precedent, `purge.rs:360`), `written_by` = `maos`, disposition `removed` or
   `left (<residue>)` with residue ∈ {`remove_failed`, `remove_timeout`, `still_exists`, `exists_failed`,
   `volume_cleanup_unconfirmed`} (AC5.1); one output line each.
5. **Failing-before (hosted):** `root_crash_retains_stopped_container_and_purge_reports_it` (`root_crash.rs:101-161`)
   extended: after `maos purge --yes`, `podman container exists` exits 1 for the old and the new CID. At `a8af3215` both
   exit 0 (purge prints `leaked container` and removes nothing, `purge.rs:1969-1983`). Local stub vector: the stub's call
   log has `rm --force --volumes --time=0` exactly for the owned CIDs (none at HEAD).
6. **Plant (hosted, `ubuntu-24.04`):** the removal call made a no-op ⇒ the extended root-crash scene reds; restored ⇒ green.

### AC2 — ownership and refusal twins (epic AC1 "ownership/refusal twins")

1. Not removed, listed under "MAOS-labelled containers not attributable to this MAOS home" with the exact
   `podman rm --force <CID>` command: (a) label pair present, `/workspace` source under another home's root;
   (b) label pair present, no `/workspace` mount (pre-19-3 runs, `maos-smoke-t3-*`, `main.rs:8462`); (c) name and labels disagree.
2. Never listed or touched: a container without the MAOS label pair, including one **named** `maos-…` (at HEAD
   `purge.rs:1976` prints it as a leaked MAOS container).
3. A live root of this home (`RootShared`, `operator_door.rs:2630-2639`) ⇒ purge refuses before any runtime invocation
   (stub call log empty) — the existing refusal (`purge.rs:892-901`) asserted against the runtime log.
4. **Hosted twins** (same scene file): a second home's Worker container and a label-less `maos-impostor` container both
   survive `maos purge --yes` of the first home. **Local:** stub vectors for 2.1(a)(b)(c), 2.2 and 2.3.
5. **Plant (local):** attribution reduced to the `maos-` name prefix ⇒ the 2.1(a) and 2.2 vectors red.

### AC3 — the dry run stays inert (epic AC1 "no side effects on dry-run")

1. `maos purge` without `--yes` prints `would remove container <CID> (<name>)`, `would remove cgroup <path>`,
   `would remove T3 state <path>` and the not-attributable list; it runs only read-only runtime verbs (`ps`, `inspect`);
   no `rm`, `kill`, `unshare`, rmdir or write; the podman-storage guard (`purge.rs:1994-2013`, D:1069) still gates every
   runtime call.
2. **Failing-before (local):** a stub-runtime vector expecting the `would remove container` line reds at `a8af3215`.
   `purge_dry_run_is_inert_then_removes_only_maos_owned_state` (`maos_uninstall_16_4.rs:168`) stays green.
3. **Plant (local):** call the removal from the dry-run branch ⇒ the stub log shows `rm` ⇒ the vector reds.

### AC4 — owned cgroup leaves (D-04 A, D-05 A)

1. The `/sys/fs/cgroup/maos` report (`purge.rs:1952-1957`) is deleted: nothing creates that path (C6).
2. Purge walks the cgroup v2 mount without following links, finds `maos.slice` directories and their
   `spirit-<id>-<pid>-<seq>` leaves owned by this euid (`linux.rs:492,511-514`), rmdirs a leaf iff `populated 0` and no
   `/proc/<creator pid>`, then `maos.slice` iff empty; populated or live-creator leaves are reported with member pids
   (`cgroup.procs`) and the exact commands; nothing is ever written to `cgroup.kill`. Receipt rows `cgroup://<path>`.
   The walk takes the mount root as a parameter, so the logic is testable over a temp-dir tree (no environment knob).
3. **Failing-before (local):** over a temp-dir cgroup tree, a dead-creator empty leaf survives purge's removal pass at
   `a8af3215` (purge never reads `maos.slice`). Twins: a populated leaf and a live-creator empty leaf survive and are reported.
4. **Hosted:** in the delegated `epic-17-exit` unit (`run_cgroup_ceiling_17_2.py --mode delegated`,
   `discipline.yml:2345-2364`), a dead-creator leaf planted under the real delegated `maos.slice` is gone after
   `maos purge --yes`; a populated twin survives. **Binding (D-05 A):** a native child spawned through the kernel
   (`SandboxedChild::cgroup_path()`) is accepted by purge's matcher — a kernel naming change reds it.
5. **Plant (local):** drop the creator-alive check ⇒ the live-creator twin reds.

### AC5 — retained T3 custody and the `TerminationUnconfirmed` contract (D-03 A, D-07 A; coordination with 20-7)

1. Purge's per-container verdicts use only the post-removal residue variants (AC1.4); `termination_unconfirmed`,
   `kill_failed` and `runtime_client_unconfirmed` never appear in purge output or receipt; a container whose last
   kernel verdict was `TerminationUnconfirmed` (P1–P4 above) is handled as possibly live by AC1's forced removal.
   `crates/maos-kernel-core/src/security/sandbox/t3/child.rs` is byte-identical at landing.
2. Under `$XDG_RUNTIME_DIR/maos-worker-state` and `/tmp/maos-worker-state-<euid>` (`egress.rs:242-255`), each validated
   as `egress.rs:273-288` validates (directory, owner euid, mode 0700, not a symlink): remove each spawn dir whose `cid`
   names a CID `container exists` reports absent after AC1's pass, then empty process roots, then the empty state root;
   keep and report spawn dirs without `cid` and dirs naming a CID that still exists. Receipt rows with the directory
   path. The `egress.rs:240-241` comment says so.
3. **Failing-before (local):** a fixture state root (an empty process root; a spawn dir whose `cid` the stub reports
   absent; one whose CID the stub reports present; one without `cid`) — at `a8af3215` all survive `purge --yes`; after,
   only the last two survive and are reported.
4. **Hosted:** after the extended root-crash scene's `purge --yes`, the killed root's spawn dirs are gone.
5. **Producer (D-07 A):** measurement 5 names the exit path(s) that skip `cleanup_state_root` (`supervision.rs:2029-2041`);
   after the repair, N runs of that path leave the process-root count unchanged (vector over a private `TMPDIR`), red at
   `a8af3215`.
6. **Plant (local):** skip the `container exists` check before removing a spawn dir ⇒ the present-CID twin reds.

### AC6 — preserve 19-3, the receipt and the user-namespace mapping; one implementation (epic AC2)

1. Worker workspace removal keeps its behaviour, dispositions and residue lines (`purge.rs:1237-1240,1346-1454`);
   `worker_workspace_19_3`, `maos_uninstall_16_4` and the 17-7 suite stay green.
2. The receipt is still created before any removal and finalized as today (D-20-10-08 A); new rows are additive;
   `schema_version` is unchanged unless the reviewer finds a reader that breaks (then bump + reader update, same change).
3. Purge never edits `/etc/subuid`, `/etc/subgid`, podman storage or configuration, and runs no `podman system`
   verb; the stub-log verb set is ⊆ {`ps`, `inspect`, `rm`, `container exists`, `unshare`}.
4. One bounded runtime helper serves `ps`, `inspect`, `rm`, `container exists` and 19-3's `unshare` (no third wait loop).
5. `purge.rs:1042` and every `packaging/` file are untouched (D09 A → 20-2).

### AC7 — records the change makes false

1. ADR-067 `:87-90` (Consequences) states what purge now removes, the ownership rule and what stays report-only;
   `:92-97` (Verification) names the new tests. Docs: `docs/maos.dev/run-maos.md:105-121`, `docs-site/docs/run-maos.md:105-121`,
   ko `run-maos.md:102-113`, cookbook `cli-wrapper-spirit.md:63` (+ ko `:64`) — en and ko in the same change.
2. `tests/coverage-matrix.yaml:408-415` (FR2) names the new tests and the hosted job; the stale v0.1 note goes.
3. `EPIC17_TERMINAL_SUITE_TESTS` (`discipline.yml:2262`) and its required-test list match the added scenes.
4. At `done`: RA:24/:162 marked witnessed by 20-10 @<commit>; `check-dev-record-completeness` passes with no open row
   owned by this key.

## Tasks / Subtasks

- [ ] **T0 — Preflight.** Rulings on D-20-10-01…10 recorded; tracker → `in-progress`. Run `creation/measurements.md`
  "Not measured here" 1–6 (1–4 and 6 on a branch-only `ci/20-10-diag` hosted run; 5 locally with a private `TMPDIR`).
  Re-measure every cite this story touches (rule 9). A result contradicting a ruled option is STOP + D-row.
- [ ] **T1 — Bounded runtime helper (AC6.4).** Refactor the two wait loops into one helper with env hygiene; existing
  purge tests green before any behaviour change.
- [ ] **T2 — Enumeration and attribution (AC1.1, AC2).** Stub-runtime harness (stub `podman` on `PATH` logging argv,
  answering `ps`/`inspect` from fixture JSON; precedent `worker_terminal_facts_17_7/stub_scenes.rs:25-41`); local vectors + plant.
- [ ] **T3 — Removal, order, receipt rows (AC1.2–1.4).** Containers before workspaces; rows created before removal.
- [ ] **T4 — Dry run (AC3).** `would remove …` lines; read-only verbs only; vector + plant.
- [ ] **T5 — Cgroups (AC4).** Walk over a mount-root parameter; temp-dir vectors + plant; delegated hosted scene; binding test.
- [ ] **T6 — T3 custody and producer (AC5).** Reclaim with validation; contract assertions; measurement 5 → repair (D-07 A).
- [ ] **T7 — Hosted scenes (AC1.5, AC2.4, AC4.4, AC5.4).** Extend `root_crash.rs`; second-home and impostor twins;
  count pin; branch-only `ci/20-10-verify` workflow for the AC1 plant (never merged).
- [ ] **T8 — Records (AC7).** ADR-067, docs en + ko, coverage row, egress comment, docker leg (D-06 A).
- [ ] **T9 — Landing.** Exact maos-bin and aggregate pins inside the ruled envelope (kernel untouched unless a kernel
  option was ruled — then that shape alone in a final L2 landing, with its FLAG-Winston record and re-base);
  `check-kernel-baseline`, `kloc-check`, `check-env-contract`, `check-dev-record-completeness`,
  `check-epic-close-coherence`, `check-coverage-matrix-completeness`, `prepush`; one hosted discipline run on the final
  tree + the plant run. Revert `intent-lineage-coverage-report.md` if a run modifies it. No commit or push without the operator.

## Dev Notes

### What changes, by file

| Area | Files | AC |
|---|---|---|
| Bin (production) | `crates/maos-bin/src/purge.rs` (`run` `:843-1047`, dry-run branch `:910-933`, receipt preflight `:935-970`, removal `:975-984`, report `:1021-1041`, `remove_planned_roots` `:1258-1344`, 19-3 helpers `:1424-1454`, report-only `:1948-2049`); `crates/maos-bin/src/egress.rs:240-241` (comment); the producer path measurement 5 names (`supervision.rs:2029-2041` or its callers) | AC1–AC6 |
| Tests (uncharged) | `crates/maos-bin/tests/maos_uninstall_16_4.rs` or a new `purge_residue_20_10.rs` (stub-runtime vectors); `crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs` (+ twins); `purge.rs` inline tests for the cgroup walk | AC1–AC5 |
| Workflows | `.github/workflows/discipline.yml:2262` (+ required tests); branch-only `ci/20-10-diag`, `ci/20-10-verify` | AC1, AC7 |
| Docs / records | `docs/adr/ADR-067-maos-data-root-layout-and-purge.md:87-97`; run-maos ×3; cookbook ×2; `tests/coverage-matrix.yaml:408-415` | AC7 |
| Kernel | none (recommended); a ruled KH/KR/KC/KS shape touches only the files in its `proto-*.diff` | — |

### Overlap (serialize; re-measure)

- **`purge.rs` / `maos_uninstall_16_4.rs`** — 20-2 (D09 A: binary hint `:1042`, deb/RPM legs), 20-11 (erasure; the
  `uninstall` cascade is `main.rs`, not purge). `deferred-work.md:13` (owner 20-7): a load-sensitive red in this test
  file; a recurrence during this story is recorded by name and run id, never retried to green.
- **`t3/child.rs`** — 20-7 owns K1w/K2r (landed; L2 pending). This story reads it and changes nothing (AC5.1).
- **`maos-bin` pin** — 20-1a/20-1b draw on maos-bin; the later lander re-bases. **Kernel pins** only if a kernel option is ruled.
- **`run-maos.md` (en/ko)** — 20-2 (binary removal sentence) and 20-1a/b (registry); one owner per edit.
- **`supervision.rs`** (only under D-20-10-07 A) — 20-9 (spawned lifecycle) and 20-7 (drop/unload repair) edit the same file; the state-root cleanup is local to `cleanup_state_root` and its callers.

### Current state that must be preserved

- Dry run inert, including no podman storage creation (D:1069; `purge.rs:1986-2013`).
- Lock before any step; snapshot restore on every refusal (`purge.rs:892-933`).
- Receipt outside every deletion root, created before removal, `in-progress` → `complete`/`partial` (ADR-067:71-74).
- Operator-authored entries never removed (`WrittenBy::Operator`, `purge.rs:12-18,1282-1291`); `--keep-log` semantics.
- 19-3: walk first, `podman unshare` fallback, `--one-file-system`, residue command verbatim (`purge.rs:1346-1454`).
- The kernel's teardown order and residue vocabulary (`child.rs:340-506`) — consumed, not changed.

### Guardrails

- Name is never authority (`argv.rs:16-21`): a CID reaches the runtime only after AC1.1 attributes it.
- Runtime-supplied strings (names, labels, mount sources) are compared, never joined into a path purge deletes.
- No `cgroup.kill`, no signal to any pid, no write under `/sys/fs/cgroup` other than rmdir of an attributed empty leaf/slice.
- No kernel line without a ruled FLAG-Winston row; any `crates/maos-kernel-core/src` change otherwise is STOP.
- Platform gates: the cgroup walk, `/proc` checks and the custody reclaim are `cfg(target_os = "linux")`; the existing `cfg(unix)`/`cfg(not(unix))` split stays compiling. `purge.rs` is in the 20-6 macOS guard's path filter (E20:233) — a `purge.rs` change starts `macos-check`; the 2026-09-25 `st_dev` break (`purge.rs:1566-1567` comment) is the precedent. The air-gap build (`--no-default-features`) keeps `purge` (ADR-067:51-56): no network-gated dependency.
- One cargo process per checkout; `env -u CI -u GITHUB_ACTIONS`; inherit the target directory; plants byte-restored
  in the real tree (`proof-contract.md:24-31`). Local `SKIP:` lines of T3 scenes are not evidence.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red | Leg |
|---|---|---|---|
| Removal (AC1) | removal made a no-op | extended root-crash scene | hosted `ubuntu-24.04` |
| Attribution (AC2) | name-prefix attribution | other-home + impostor stub vectors | local |
| Dry run (AC3) | removal called from the dry run | stub-log vector | local |
| Cgroups (AC4) | creator-alive check dropped | live-creator twin | local |
| Custody (AC5) | `container exists` check skipped | present-CID twin | local |
| Preserve (AC6) | — (regression set: 16-4, 19-3, 17-7 suites) | — | local + hosted |

### Tests and commands

```bash
# local; one cargo process per checkout; inherit the target directory
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test maos_uninstall_16_4 --test worker_workspace_19_3
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test worker_terminal_facts_17_7   # T3 scenes SKIP locally; hosted is the evidence
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-kernel-baseline
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- kloc-check --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-env-contract
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-coverage-matrix-completeness
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-dev-record-completeness
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-epic-close-coherence
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- prepush
```

### Review plan (`proof-contract.md:33-71`)

DIFFICULT: Blind + Edge (opus-5.5), Acceptance (opus-5.5), Security reading on AC1/AC2 (cross-home and foreign
containers), AC4 (cgroup race) and AC5 (deleting under a shared `/tmp` root), Test Infrastructure (glm-5.3). Focused
suite `cargo nextest run -p maos-bin --locked`; the reviewer reads the hosted plant run, never re-runs it.

### Previous-story intelligence (16-4, 19-3, 17-7, 20-7)

- 16-4: its validation round withdrew a phantom `/sys/fs/cgroup/maos` root (`16-4-…md:180-183,720-723`) yet the
  report line shipped; the dry run once created podman storage (D:1069) — every runtime call stays behind the guard.
- 19-3: the workspace is named by the container name and mounted at `/workspace` (`worker_spawn.rs:1061-1067`) — the
  home-scoped fact AC1 attributes by; its `podman unshare` removal is the precedent for runtime calls in purge.
- 17-7: CID custody, "state retained" residue, and the root-crash scene that already proves a retained container is
  observable (`root_crash.rs:101-161`).
- 20-7: K1w/K2r narrowed when `TerminationUnconfirmed` occurs (P2/P3) without changing its meaning; its D13 A rule
  (no retry to green) applies to the 17-7 scenes this story extends.

### Git intelligence

`a8af3215` (20-7 rulings record), `47631284` (20-1a hosted fixes; last crate change), `1fd4de31` (20-7 L1: K1w + K2r
in `t3/child.rs`), `9fea84c0` (last `purge.rs` change: 19-3 review round 2 workspace removal), `bde07d32` (dry run no
longer creates podman storage).

### Library / framework

No new dependency: `serde_json` (podman `--format json`/`inspect`), `rustix` (`geteuid`, already used `purge.rs:1996`),
`maos_domain::sandbox::{ContainerResidue, parse_container_id}`. Podman: `rm --force --volumes --time=<n>`,
`container exists` (exit 1 = absent), `ps --filter label=<key>` — exact multi-filter and `Mounts[].Source` forms are
measurements 1–2. Docker's `container create` has no `--pidfile` (https://docs.docker.com/reference/cli/docker/container/create/).

### References

E20@HEAD:83,272-277; PF `epic-20-preflight-2026-10-30.md:84,128,141,183,247,293`; tracker `sprint-status.yaml:298,308`;
`deferred-work.md:13,1069`; `routing-audit-2026-10-22.md:24,162`; PRD `functional-requirements.md:25`,
`non-functional-requirements.md:72`; ADR-067, ADR-069 (`:35-46`); `16-4-maos-uninstall-and-keyring.md:180-183,423-424`;
`20-7-stable-runner-perf-gates.md:281`; `20-1a-registry-publish-install-exit-path.md:162` (D09 A).

## Dev Agent Record

### Story context creation

Created 2026-11-02 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter measured contract) at `a8af3215`,
without cargo (parent rule).

**Inputs:** E20 §20-10 and stories row at HEAD (amended 2026-10-30, PF §3.5); preflight rows 20-10 and D09; tracker
`:308`; 16-4, 19-3, 17-7 and 20-7 records; direct reads of every cited range (`creation/cites-a8af3215.md`); kernel and
kloc by the prebuilt gate binary on `/tmp/proto2010` `git archive` copies (`creation/kloc-kernel-a8af3215-and-prototypes.json`);
prototypes KH, KR, KC, KS and bundles saved as diffs, `rustfmt`'d, never applied; read-only host probes (rootless
podman unavailable; 8,903 empty process roots).

**Measurements owed:** `creation/measurements.md` "Not measured here" 1–7.

**Records owed:** `creation/records-owed.md` (§A epic/preflight/routing/20-7 notes for the orchestrator; rulings; landing).

**Revert:** no crate source modified (prototypes only under `/tmp/proto2010`, deleted); no cargo/test ran;
`intent-lineage-coverage-report.md` untouched.

**Gates after the record edits** (prebuilt `/mnt/build/cargo-maos/debug/xtask`, `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=…/xtask`, `--json`, in-tree; `creation/gates/*.json`): `check-dev-record-completeness` rc 0 (197 done stories, 166 deferred-owner assertions, 0 violations); `check-epic-close-coherence` rc 0; `check-decision-register` rc 0; `check-exit-commands` rc 0; `check-kernel-baseline` rc 0 (26841 / 95 files, no drift). In-tree `kloc-check` rc 1 (`maos-wasm-host 2420 > 2412`, aggregate 191834) from a concurrent uncommitted `crates/maos-wasm-host/src/runner.rs` edit (+8, mtime 14:37:31) this creator did not make and left alone; the HEAD-archive `kloc-check` used for every figure above passed (aggregate 191826, `over_budget = []`).

### Agent Model Used

anthropic/claude-opus-5-5 (creation).

### Debug Log References

### Completion Notes List

### File List

## Change Log

- 2026-11-02: Story created at `a8af3215` (ready-for-dev; D-20-10-01…10 PROPOSED). E20 §20-10 cites corrected in place
  (C1–C3). Measured: purge reports containers by name prefix and a cgroup path nothing creates; the real owned cgroups
  (`maos.slice/spirit-*`) and the T3 custody under the worker state root (8,903 empty process roots on the dev host) are
  outside purge. Kernel 0 recommended; KH/KR/KC/KS priced +10/+8, +2/−1, +5/+1, +3/+2.
- 2026-11-02: Operator rulings recorded (ask s2010): D-20-10-01…10 as recommended; kernel-Δ 0 and ceilings per frontmatter. Status remains ready-for-dev.
