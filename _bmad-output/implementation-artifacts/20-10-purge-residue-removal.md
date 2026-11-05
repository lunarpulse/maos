---
story_id: '20.10'
story_key: 20-10-purge-residue-removal
created: 2026-11-02
context_commit: a8af3215
baseline_commit: d3e3f586cb325c16a691cd19d1fd62588b288a0d
security_sensitive: true
kernel_grant: "RULED 2026-11-02 (ask s2010): kernel-Δ 0; no kernel grant. HEAD pins 26841 physical / 21042 tokei."
kloc_grant: "RULED 2026-11-02 (ask s2010; D-20-10-10 A; D02 A ×3 unmeasured). Original pre-20-4 ceilings: maos-bin ≤ 34548; aggregate ≤ 192876; every other crate 0. CURRENT 2026-11-03 orchestrator-adjudicated, D02 A: combined required hosted corrections re-base within already authorized inherited ceilings, maos-bin ≤ 34643 and aggregate ≤ 193526; STOP above. Inherited measured base 34504/193140; final 34551/193187 (+47), hardfail193188 = measured + 1. Exact after cargo fmt --all; kernel/xtask and other grants unchanged. This is a re-base, not a widening."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:67-71`): security-critical — an irreversible destructive command whose ownership rule decides whether another MAOS home's Workers or a foreign container are killed (cross-home isolation), plus a concurrency boundary (a live creator racing cgroup removal). Blind and Edge on opus-5.5, Acceptance on opus-5.5, a Security reading on AC1, AC2, AC4 and AC5, Test Infrastructure on glm-5.3 (standing policy `proof-contract.md:57-71`; any session substitution recorded at review). A FLAG-Winston reading is added only if a kernel option is ruled. Creation anthropic/claude-opus-5-5."
---

# Story 20.10: Purge removes owned T3 runtime residue

Status: done

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

**Accepted review boundary (Main's operator-delegated adjudication):** matching
labels/name/mount is same-UID attribution, not unforgeable provenance. A
same-user rootless container creator can forge that metadata; rootless same-UID
control is not a privilege boundary. Bare `podman` intentionally uses operator
PATH; credential scrubbing does not make PATH trustworthy.

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
| **D-20-10-07** | The process-root leak at its producer (8,903 measured) | **A** diagnose and repair a demonstrated producer · **B** operator-owned historical investigation when the reproducer is harness-only · **C** purge-only cleanup without a row | **Review disposition:** accept the measured test-only strong-Arc-cycle diagnosis and Weak-latch repair; do not invent a production leak repair. Three production one-shots left zero roots; the fixture leaked one before and none across three repaired runs. The 8,903 historical roots remain unattributed and anonymous roots lack abandonment proof. | RULED A originally; Main's operator-delegated review adjudication accepts the harness repair plus **B** historical investigation, owner `20-7-stable-runner-perf-gates`, `deferred-work.md:4` (rule 10). |
| **D-20-10-08** | Receipt status when owned runtime residue survives | **A** the 19-3 convention: rows `left (<residue>)`, status `complete` unless an error note fired (`purge.rs:986-1012`, `:1237-1240`) · **B** status `partial` whenever an owned container, leaf or custody dir survives (and the 19-3 workspace residue re-labelled the same way) | **A.** One convention in one receipt; the rows carry the residue. B is more conspicuous but changes 19-3's shipped meaning. | RULED 2026-11-02: **A** (ask s2010) |
| **D-20-10-09** | Start before 20-2 | **A** wait for 20-2 `done` (E20:274) · **B** start when picked: no `packaging/` file, `purge.rs:1042` and the binary-removal sentence of the run-maos docs left to 20-2; the later lander rebases the shared docs paragraph | **B.** The ordering existed for D09, now ruled; the overlap is one docs paragraph. | RULED 2026-11-02: **B** (ask s2010) |
| **D-20-10-10** | Per-crate envelope (D02 A applied) | **A** the envelope above (×3 unmeasured / ×2 measured) · **B** exact measured grants only, every review correction re-asks · **C** allowance ×1.3 (rule 6) | **A.** Precedent: 20-3b/20-3c landed at 2–3× creation forecasts (`20-7-…md:148`); all of this story's figures are unmeasured. | RULED 2026-11-02: **A** (ask s2010) |

**Review safety amendments (operator-delegated to Main):** D-03 removes an empty
process root only with positively absent creator PID evidence encoded in its
`process-<pid>-<nonce>` name. Live/anonymous roots and shared state parents stay.
D-04 removes eligible leaves only, never shared `maos.slice`. D-08 A retains
schema 1/status vocabulary but the existing `next_step` and final CLI summary
say `complete with residue: N left`; surviving MAOS-written rows mean exit 1.
Purge refuses unset/non-absolute HOME, HOME/ancestor deletion roots, parent
components in purge/protected paths, and unsafe legacy ancestors before removal.

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
   A nonzero inspect is skipped only after a bounded `container exists` exits1;
   present or unconfirmed candidates stay fatal with inspect status/stderr.
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
   `would remove T3 state <path>` and the not-attributable list; it runs only read-only runtime verbs (`ps`, `inspect`,
   and bounded `container exists` solely to disambiguate a failed inspect, per the 2026-11-03 orchestrator instruction);
   no `rm`, `kill`, `unshare`, rmdir or write; the podman-storage guard (`purge.rs:1994-2013`, D:1069) still gates every
   runtime call.
2. **Failing-before (local):** a stub-runtime vector expecting the `would remove container` line reds at `a8af3215`.
   `purge_dry_run_is_inert_then_removes_only_maos_owned_state` (`maos_uninstall_16_4.rs:168`) stays green.
3. **Plant (local):** call the removal from the dry-run branch ⇒ the stub log shows `rm` ⇒ the vector reds.

### AC4 — owned cgroup leaves (D-04 A, D-05 A)

1. The `/sys/fs/cgroup/maos` report (`purge.rs:1952-1957`) is deleted: nothing creates that path (C6).
2. Purge walks the cgroup v2 mount without following links, finds `maos.slice` directories and their
   `spirit-<id>-<pid>-<seq>` leaves owned by this euid (`linux.rs:492,511-514`), rmdirs a leaf iff `populated 0` and no
   `/proc/<creator pid>`; shared `maos.slice` is never removed because emptiness does not exclude a creator's allocation window. Populated or live-creator leaves are reported with member pids
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
   as `egress.rs:273-288` validates (directory, owner euid, mode 0700, not a symlink): capture a valid spawn CID
   and held directory identity at inventory before AC1's removal can delete the cidfile; reclaim only with a fresh
   `container exists` absence proof and renewed ownership/mode/device/inode checks. Then remove empty
   `process-<creator pid>-<nonce>` roots only if that PID is positively absent; shared state parents are never removed.
   Keep/report live/anonymous roots, missing/invalid-at-inventory CID custody, and still-present/unknown CIDs. Receipt rows use the directory
   path. The `egress.rs:240-241` comment says so.
3. **Failing-before (local):** a fixture state root (an empty dead-creator process root; a spawn dir whose CID
   the stub reports absent; one whose CID is present; one without CID) — at the baseline all survive;
   after, the dead-creator empty root and absent-CID spawn are reclaimed, retained rows/reports are asserted.
4. **Hosted:** after the extended root-crash scene's `purge --yes`, the killed root's spawn dirs are gone.
5. **Producer review disposition (D-07):** the demonstrated stop-during-mint test seam retained its supervisor
   through an Arc cycle, bypassing Drop/`cleanup_state_root`. Its Weak-latch repair leaves the private root
   count unchanged across N runs; historical anonymous roots remain under the operator-owned deferred investigation.
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
openai-codex/gpt-6.1-sol (20-10 landing 1 implementation; no independent review claimed).

### Debug Log References

- [Exact command/exit-code ledger](20-10-purge-residue-removal-evidence/landing-1/commands.md) and [machine-readable ledger](20-10-purge-residue-removal-evidence/landing-1/commands.json).
- [Per-AC witnesses, mutations, hashes, wall times and byte-restores](20-10-purge-residue-removal-evidence/landing-1/plants.md).
- [Baseline reds](20-10-purge-residue-removal-evidence/landing-1/red-before-head.log), [restored focused suite](20-10-purge-residue-removal-evidence/landing-1/focused-restored-green.log), [serial workspace](20-10-purge-residue-removal-evidence/landing-1/workspace-final-serial.log), [default workspace](20-10-purge-residue-removal-evidence/landing-1/workspace-default.log).
- [Prescribed prepush](20-10-purge-residue-removal-evidence/landing-1/prepush.log), [fresh serial budget](20-10-purge-residue-removal-evidence/landing-1/check-test-budget.log), [final clippy ratchet](20-10-purge-residue-removal-evidence/landing-1/clippy-final-ratchet.log).
- [Unpushed hosted verification cut](20-10-purge-residue-removal-evidence/ci/ci-20-10-verify.yml), [hashed AC1 mutation](20-10-purge-residue-removal-evidence/ci/ac1-removal-noop.diff), [implementation diff digest](20-10-purge-residue-removal-evidence/landing-1/implementation.diff.sha256).


### Completion Notes List

#### 20-10 landing 1 — implementation and observed local proof

Implementation baseline is `d3e3f586cb325c16a691cd19d1fd62588b288a0d`. No commit,
push, CI-branch edit, packaging edit, kernel edit or status/tracker flip was made.
The Cargo-owned binary hint remains unchanged. The original task checkboxes are
not a claim that hosted execution occurred.

- **AC1/AC2:** read-only label-filtered CID enumeration and inspect; require both
  labels, their matching name and this home's `/workspace` source. Only parsed
  CIDs reach forced removal and absence confirmation. Unattributable labelled
  CIDs get the exact command, label-less names no authority. Store lock precedes
  enumeration. Containers precede workspace deletion.
- **AC3:** dry run uses only `ps`/`inspect`, keeps the storage guard and performs
  no runtime removal or custody/cgroup mutation. Custody lines explicitly state
  their absent-CID condition; no forbidden `exists` probe is hidden in dry run.
- **AC4:** nofollow cgroup walk over a mount-root parameter; euid ownership,
  `populated 0`, and positively absent creator PID before rmdir; empty slice
  cleanup, retained member/creator diagnostics and exact commands, never kill.
- **AC5:** private euid-owned mode-0700 custody, nofollow CID reads, absent-CID
  spawn reclamation using the existing descriptor-relative tree remover, then
  empty process/state roots. Missing/live/unknown CIDs remain. The process-root
  producer diagnosed here is **test-only**: the stop-during-mint fixture held a
  strong Arc cycle through its seam latch, so `WorkerSupervisor::drop` never
  called `cleanup_state_root`. Three production one-shots left zero roots; the
  HEAD fixture itself passed but changed its private root count 1 → 2. Replacing
  the latch with Weak and asserting release made three fixed runs keep 2 → 2.
  This does not attribute every historical host directory to that one fixture.
- **AC6:** one maos-bin podman adapter reuses the existing capped, deadline-bound
  kernel invocation and its four-key environment allowlist for every verb,
  including 19-3 unshare. No new dependency or environment variable. Runtime rows
  are all pending before removal; custody outcomes are batched to avoid N²
  receipt rewrites. Schema 1 and the 19-3 `complete`/`left (<residue>)` convention
  remain. `child.rs` and the entire kernel baseline remain byte-identical.
- **AC7:** ADR-067, English and Korean run/cookbook docs, FR2 coverage notes,
  egress custody comment, terminal-suite count 33 → 34 and its xtask reader are
  updated. `podman://<CID>`, `cgroup://<path>` and directory receipt rows are
  documented. No open deferred row is assigned to this story.

| AC | Witness definition | HEAD red-before | Green-after / mutation |
|---|---|---|---|
| AC1 | `purge_residue_20_10.rs:127` `ac1_owned_cid_removed_before_workspaces_and_receipted` | observed, rc 100 | local PASS; real root-crash/no-op plant cut, not run |
| AC2 | `purge_residue_20_10.rs:153` `ac2_other_home_missing_mount_mismatched_name_and_impostor_survive` | observed, rc 100 | PASS; name-prefix plant rc 100 |
| AC3 | `purge_residue_20_10.rs:169` `ac3_dry_run_uses_only_readonly_verbs_and_preserves_custody` | observed, rc 100 | PASS; dry-run-removal plant rc 100 |
| AC4 | `purge_runtime.rs:535` `ac4_dead_empty_live_creator_populated_and_symlink_twins`; `root_crash.rs:262` real delegated twin | hosted baseline witness cut, unexecuted | local PASS; dropped-creator-check plant rc 100 |
| AC5 | `purge_residue_20_10.rs:192` `ac5_only_absent_cid_and_empty_state_are_reclaimed` | observed, rc 100 | PASS; skipped-exists plant rc 100; producer counts above |
| AC6 | `purge_residue_20_10.rs:215` `ac6_runtime_helper_is_shared_and_receipt_convention_is_preserved` | observed, rc 100 | PASS; ambient-runner plant rc 100 |
| AC7 | `purge_residue_20_10.rs:236` `ac7_adr_documents_home_scoped_runtime_removal` | observed, rc 100 | PASS; stale-ADR plant rc 100 |

Six local plants, one per AC2–AC7, each restored by copy-back + cmp + touch
(every restore command rc 0). Their total command wall time was 55.232 s.
AC1's single hosted no-op plant is cut with its SHA-256, not locally claimed.

| Measure | Baseline | Exact landing | Ruled ceiling |
|---|---:|---:|---:|
| maos-bin tokei | 33498 | **34037 (+539)** | 34548 |
| kernel physical / tokei | 26841 / 21042 | **26841 / 21042** | unchanged |
| xtask tokei | 44406 | **44406** | unchanged |
| aggregate | 191826 | **192365 (+539)** | 192876 |
| aggregate hardfail pin | 191827 | **192366 = measured + 1** | exact |

Final pins were measured after `cargo fmt --all`, with `20-10 landing 1` HISTORY;
no other crate grew and no ceiling was exceeded. Kernel gate: 95 files,
`changed = added = removed = []`, set hash `db39bc4a1ab6a6c22575990ae934e698a519d3efd46a8dbb38e62bc376697b59`.

**Observed verification:** final all-target workspace build rc 0; restored focused
suite 142/142 PASS; final serial workspace 5562/5562 PASS, 138 configured skips;
default workspace 5562/5562 PASS, 138 configured skips. Fresh serial junit budget
passed: 594 binaries, 646.231 s summed test time, no findings. Local T3 scenes
self-skip, so none of those counts is claimed as real-podman proof.

All requested structural gates exited 0: kloc-check, check-kernel-baseline,
check-service-boundary, check-env-contract, check-serde-error-handling,
check-composition-root-completeness, check-j1-loopback-delegation,
check-dev-record-completeness, check-epic-close-coherence,
check-review-findings-resolved, check-test-cost, invariant-lock, coverage-matrix
(`--dry-run`), check-coverage-matrix-completeness, check-exit-commands,
check-decision-register, check-ff-j6 and check-perf-runner-policy. Cargo deny rc 0.
Non-denying clippy rc 0; final ratchet **1335 occurrences / 873 source-distinct**,
no warning in either new file. The only introduced warning was test-only
`format_in_format_args`, fixed before the final stream; the unchanged xtask
reader warning is `needless_borrows_for_generic_args` at line 15. The hosted
1383/897 baseline was not edited; same-toolchain base/final lint/file comparison
is included in the unpushed workflow, not claimed executed.

The exact prescribed prepush environment (`env -u CI -u GITHUB_ACTIONS`,
`CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask`,
`MAOS_17_2_CGROUP_MODE=undelegated`) exited **0**. Its loaded-run budget gate was
advisory-red for five binaries; exact timings and same-line rule-10 citations
are recorded in `deferred-work.md:3`, owner `20-7-stable-runner-perf-gates`.
It was not retried-to-green. No timing baseline or exception was raised.

**Earlier reds repaired, not hidden:** the first focused run timed out in
`init_prints_an_executable_truthful_purge_instruction`; its separate default-home
helper still inherited the shared temp state root. All purge fixture commands
now isolate TMPDIR, and custody receipt updates are batched. The first workspace
serial run had exactly one failure: the old xtask terminal count/required-name
pin. Its reader was migrated to 34 and the new scene; the final serial run passed.

**Unverified / externally owed:** real rootless podman and delegated cgroups are
unavailable locally, and Docker is absent. The hosted T0 measurements (mount
source/filter semantics/removal outcomes/Docker `--pidfile` refusal), real
root-crash ownership/custody twins, native cgroup binding, baseline AC4 witness,
AC1 plant/restored green, the 34-scene epic-17 matrix, hosted clippy comparison,
macOS guard and independent difficult-tier review remain unexecuted here.
The workflow stops if Docker contradicts D-20-10-06 A. Host loads were recorded
before the serial run and budget check; they were not zero, so the green serial
budget is **not an idle-host certification**. No hosted workflow was pushed.
Routing-audit `witnessed @commit` completion remains at the eventual done boundary,
not fabricated in this uncommitted implementation record.


#### Hosted cold-swap oracle incident — 2026-11-02

Hosted run `41521146083`, workspace-test-suite job `125537220359`, failed
`ac1_19_5_cold_swap_admits_the_successor_at_a_new_pid` at the old line 982:
the recorded trail ends with `cap.revoke @1` **after** `spirit.upgrade @1`
and the operator completion row, while successor load, admission grant and
start are all present (`/tmp/l9-wts.log:8944-8961`). This is the same oracle
failure as 20-7 loaded run 50, not a purge production regression.

ADR-030 defines asynchronous bounded-MPSC audit persistence, not a barrier
against direct lifecycle writers. `scheduler_loop.rs:570` revokes before
unload returns; `maos-capability/src/cap_tokens/mod.rs:306-317` mutates the
token shards before enqueueing, and `cap_audit/writer_task.rs:22-25,67-81`
persists independently. The door awaits unload before successor admission
(`operator_door.rs:1436-1445`). ADR-020 adds no global persistence ordering.
Story 19-5 CR-2's per-step ordering therefore does not imply that the audit
writer must win against successor writes. The test now checks the guaranteed
causality and every required row, draining audit before the final snapshot;
the captured late-revoke order is a deterministic regression vector.
Kernel and production sources, KLOC pins, deadlines and budgets are unchanged.

**Local falsification and repetitions (all `env -u CI -u GITHUB_ACTIONS`,
inherited target configuration, no target override/clean):**

- `cargo nextest run -p maos-bin --test one_daemon_one_door_16_1 -E 'test(ac1_19_5_cold_swap)' --stress-count 200 --test-threads 1 --no-fail-fast` before repair: rc 0, 200/200 iterations, 161.847 s. No stochastic reproduction.
- The same command with exact filter `test(=ac1_19_5_cold_swap_admits_the_successor_at_a_new_pid)` under four owned `bash -c 'while :; do :; done'` CPU loops: rc 0, 200/200, 161.398 s. All four loops terminated and reaped; no stochastic reproduction.
- `cargo nextest run -p maos-bin --test one_daemon_one_door_16_1 -E 'test(=ac1_19_5_cold_swap_oracle_accepts_recorded_late_revocation)'` with the extracted **old** oracle: rc 100, intended `no lifecycle.load ... after ... revocation` panic over the hosted recorded order (0.113 s). Deterministic red, not a manufactured production race.
- `cargo fmt --all`: rc 0. Repaired `cargo nextest run -p maos-bin --test one_daemon_one_door_16_1 --no-fail-fast`: rc 0, 42 passed, one configured skip (1.493 s), including all four oracle vectors.
- Repaired first stress command under four owned CPU loops: rc 0, 200/200 iterations of all five matching tests, 247.520 s. All loops terminated and reaped before workspace verification.

Captured transcripts: `/tmp/cold-swap-{before-serial,before-loaded,oracle-red,focused,repaired-loaded}.log`.
Host load before the serial workspace sample was 27.56/15.36/17.53;
no claim of an idle host or stable-runner timing certification.

**Final incident matrix (commands prefixed `env -u CI -u GITHUB_ACTIONS`):**

| Command | Observed result |
|---|---|
| `cargo nextest run --workspace --locked --profile ci --test-threads 1 --no-fail-fast` | rc 0; 5566 passed, 138 configured skips; suite 1453.994 s |
| `cargo nextest run --workspace --locked --no-fail-fast` | rc 100; 5565 passed, one failed, 138 configured skips; 107.046 s |
| `cargo run -q --locked -p xtask -- kloc-check --json` | rc 0; maos-bin 34037, aggregate 192365, production delta zero |
| `cargo run -q --locked -p xtask -- check-kernel-baseline --json` | rc 0; 26841 physical lines/95 files, identical set hash, no changed/added/removed files |
| `cargo run -q --locked -p xtask -- check-dev-record-completeness --json` | rc 0; no violations |
| `cargo run -q --locked -p xtask -- check-epic-close-coherence --json` | rc 0; no violations |
| `cargo run -q --locked -p xtask -- check-test-budget --json --junit /tmp/cold-swap-workspace-serial-junit.xml` | rc 1; timing findings below, no stale exception |
| `cargo clippy --workspace --all-targets --locked --message-format=json` (stdout `/tmp/cold-swap-clippy.jsonl`, stderr separate) | rc 0 |
| `cargo run -q --locked -p xtask -- clippy-ratchet --input /tmp/cold-swap-clippy.jsonl` | rc 0; 1335 occurrences/873 source-distinct, unchanged 1383/897 baseline; no diagnostic on the edited test |
| `CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated /home/lunarpulse/dev_ws/target/debug/xtask prepush --json` | rc 0; 1201.444 s, 85 pass/6 skip/one timing advisory; workspace, kernel lib, doctests and tracked-tree preservation pass |

The default failure remains **open**, not erased by the separately mandated
prepush's green workspace leg: `hook_pump_17_4::one_budget_overrun_quarantines_the_subscriber:439`,
delivered=1/failed=0 instead of delivered=0/failed=1. It is the exact prior
20-1a incident already recorded in that story's hosted-exit record; no hook
or kernel edit/retry-to-green was taken. Cold-swap and all four oracle tests
passed in both workspace modes (live cold-swap 1.071 s serial, 2.153 s default).

The serial budget's seven unchanged 30 s binary limits were exceeded:
`butler_scene_18_4` 38.849 s, `hook_pump_17_4` 39.135 s,
`maos_uninstall_16_4` 30.420 s, `observer_activity_corpus_17_4` 119.653 s,
`one_daemon_one_door_16_1` 94.881 s, `participant_recall_17_8` 60.015 s,
`private_spill_supersession_13_5j` 31.276 s; suite sum 1453.852 s exceeds
1100 s. The host was shared/loaded; no recalibration or second serial sample
was taken. Prepush's independent advisory was one-daemon 36.355 s and
participant-recall 40.575 s. Rule-10 disposition: existing loaded-budget
owner `20-7-stable-runner-perf-gates` (`deferred-work.md:3`); all ceilings,
budgets and exceptions stay unchanged.

Full evidence is `/tmp/cold-swap-workspace-{serial,default}.log`, saved serial
junit `/tmp/cold-swap-workspace-serial-junit.xml`, `/tmp/cold-swap-check-*.log`,
`/tmp/cold-swap-kloc-check.log`, `/tmp/cold-swap-clippy-ratchet.log` and
`/tmp/cold-swap-prepush.json`. Hosted rerun and real T3/delegated-cgroup
execution remain unverified; local self-skips are not substrate proof.
No commit, push, `.devcontainer` change, kernel edit or target override/clean.

#### Review correction record — 2026-11-03

Baseline `ac1fc12a7f95c4ab003f35d5ca479c181c0f22d2`; this additive record
preserves the original landing and cold-swap incident evidence above. No
story/status flip, commit, push, CI-branch change, target override/clean,
`.devcontainer` edit or kernel edit was performed.

**Adjudicated scope and fixes.** D1 retains the ruled label/name/mount
predicate, now explicitly documented as forgeable by a same-rootless-UID
creator, with trusted PATH/runtime storage as operator responsibilities:
attribution is not unforgeable provenance. D2 accepts only the measured
test-harness Arc-cycle diagnosis; the 8,903 historical anonymous roots remain
an owner-20-7 investigation (`deferred-work.md:4`), not a fabricated production
leak diagnosis. P1 names newly allocated process roots
`process-<creator-pid>-<nonce>` and requires positive creator absence before
empty-root reclamation; live/anonymous process roots and shared state parents
remain. P2 never inventories/removes shared `maos.slice` parents. P3 tests a
valid-name symlink leaf plus linked ancestor; P4 checks a workspace sentinel
inside the runtime's `rm` stub; P5 asserts retained report and receipt rows.
AC6 scans every purge source module for ambient runner spellings.

Security F1 retains receipt schema 1 and `complete`/`partial` status vocabulary:
the existing `next_step` string and final CLI line explicitly say
`complete with residue: N left`, and residue returns exit 1. F2 refuses unset
or relative HOME before any mutation and validates legacy ancestors via
nofollow directory walks plus owner/mode checks. F3 normalizes protected
paths, rejects parent traversal, and refuses purge roots equal to HOME or an
ancestor, including canonical aliases. English/Korean operator and cookbook
guidance plus ADR-067 state these boundaries.

**Witnesses.** `review-1/review-red-before.log` records five new tests failing
at the baseline (P1, P2, F1, F2, F3), exit 100. Nine distinct plants then each
failed an assertion, exit 100; every source was copied back, `cmp` returned
0 and `touch` returned 0. P3/P4/P5/AC6 are strengthened regression witnesses,
not claims that those already-correct production behaviors failed at HEAD.
Full table, exact names/lines, mutations and digests:
[`review-1/plants.md`](20-10-purge-residue-removal-evidence/landing-1/review-1/plants.md).
Final AC anchors are `purge_residue_20_10.rs:137` (AC1), `:163` plus `:311`
(AC2), `:179` plus `:362` (AC3), `purge_runtime.rs:547` plus `:633`
(AC4), `purge_residue_20_10.rs:202` plus `worker_supervision_16_3.rs:2857`
(AC5), `purge_residue_20_10.rs:247` (AC6), and `:300` (AC7).
All passed in the final focused suite; original HEAD red-before/hosted-only
AC4 evidence stays in the historical landing table.

**Exact post-fmt measurements.**

| Surface | Measured | Ceiling / exact pin |
|---|---:|---:|
| maos-bin tokei code | 34,188 | ceiling 34,548; exact pin 34,188 |
| Aggregate tokei code | 192,516 | ceiling 192,876; hardfail 192,517 |
| Kernel physical / tokei | 26,841 / 21,042 | unchanged |
| xtask tokei code | 44,406 | unchanged |

`cargo fmt --all` exited 0, then measurement and exact re-pin retained the
`20-10 landing 1` HISTORY segment. All-target workspace build exited 0.
Registered whole-package fixtures were provisioned (exit 0). Focused tests:
147/147 passed, exit 0. Full serial (`--profile ci --test-threads 1`) and
default-concurrency (`--profile ci`) workspace nextest each passed 5,571 tests,
138 configured skips, exit 0. Fresh saved serial junit budget passed, exit 0,
594 binaries, 678.553 s sum and no findings. Uptime was recorded before the
serial run (5.71 / 22.54 / 23.81 load) and budget (4.74 / 13.28 / 21.29);
this shared-host sample is not an exclusive idle-host certification.

All requested policy gates exited 0: kloc, kernel baseline, service boundary,
environment, serde, composition roots, J1 delegation, dev-record completeness,
epic coherence, review resolution, test cost, invariant lock, coverage matrix
(dry run) and completeness, exit commands, decision register, ff-j6 and
performance-runner policy. Non-denying workspace/all-target clippy exited 0;
ratchet exited 0 at **1,335 occurrences / 873 source-distinct**, versus the
unchanged hosted baseline **1,383 / 897**. Primary warnings in every changed
Rust file: **zero**, including no new `result_large_err`, `collapsible_if`
or `too_many_arguments`. Same-hosted-toolchain warning comparison remains
in the unpushed hosted cut. `cargo deny check` exited 0 (existing allowance/
skip warnings only).

Prescribed prepush, with CI/GITHUB_ACTIONS unset, absolute
`CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask` and
`MAOS_17_2_CGROUP_MODE=undelegated`, exited 0. Kernel lib tests, workspace
doctests, docs build/English-Korean coverage and workspace tests passed.
Its budget leg was **advisory**, exit 1: one-daemon 30.543 s and
participant-recall 41.594 s against unchanged 30 s budgets; not retried.
Rule-10 disposition and same-line evidence: owner `20-7-stable-runner-perf-gates`,
`deferred-work.md:3`, `review-1/prepush-review.log:660-664`, saved
`review-1/prepush-review-junit.xml`. Generated intent-lineage report matched
its saved original byte-for-byte (cmp exit 0).

**Evidence and unverified boundaries.** All exact commands, environment
overrides and exits are in
[`review-1/commands.md`](20-10-purge-residue-removal-evidence/landing-1/review-1/commands.md)
and adjacent JSON/logs/junit; review mutation digests are separate from the
historical implementation snapshot. Real Podman, Docker T0 contradictions,
delegated epic-17-exit scenes and hosted-toolchain lint comparison remain
unexecuted locally, cut in `...-evidence/ci/ci-20-10-verify.yml`, never pushed.
Local self-skips (signed T3 substrate, Python Ed25519 backend and strace)
are explicitly not proof of those legs. Historical anonymous-root producer
attribution and exclusive-idle budget certification remain owner-20-7 work.

#### Hosted incident — runs 41584515474 / 41584453971, 2026-11-03

This incident supersedes the earlier unexecuted-hosted boundary only where
actual evidence now exists. Logs, T0 measurements, classification and commands
are preserved in
[`hosted-1/hosted-findings.md`](20-10-purge-residue-removal-evidence/landing-1/hosted-1/hosted-findings.md).

The branch-only run's four scene jobs all stopped before tests: invalid
`--unit maos-20-10` violated the existing `maos-17-[27]-*` harness contract.
Head jobs 125749866755/125749866883 cite log line 4058, baseline
125749866879 line 4061, plant 125749866891 line 4059. Baseline and plant
are **not** intended assertion-red evidence. Corrected only the cut's unit
names, not the harness allowlist or assertion-message guards. The new xtask
workflow-contract witness failed before the fix (exit 100), passed afterward
(4/4, exit 0), and rejected the byte-restored invalid-unit plant (exit 100).

Both real T0 jobs passed: Podman 4.9.3 / 5.7.0 normalized mount sources,
combined spirit/nonce filters selected only the paired-label CID, forced
removal returned 0 and post-removal exists returned 1. Docker 28.0.4 / 29.4.2
rejected `--pidfile`, exit 125. No ruled D-row was contradicted; D-20-10-06 A
stands. The hosted Rust-1.99.1 warning comparison was `[]`, at 1381/896
against the unchanged 1383/897 baseline (job 125746775456 lines 2135/2144).
This proves that submitted tree, not any subsequent unexecuted revision.

Main run 41584453971 exposed **two new 20-10 regressions**, not known teardown
flakes. Review P1's shared root helper accidentally applied the
`process-<pid>-<nonce>` prefix to durable egress socket roots. Hosted path
shapes grew 94-107 bytes to 108-121 bytes, preventing Worker startup and
inflating startup-wait budgets. The new Linux hosted-length vector confirms
the actual bind error `path must be shorter than SUN_LEN`, before the fix
(exit 100). The production correction confines PID-bearing names to volatile
Worker-state/custody roots; durable socket roots keep short eight-hex nonces.
The custody witness retains live-PID and legacy nonce-only roots, removes only
dead-owner empty roots, and checks report/receipt rows for retained shapes.

The terminal purge fixture also set `HOME == MAOS_HOME`. Its new executable
dry-purge preflight confirms the real safety refusal before the fixture fix
(exit 100), without requiring a provisioned T3 runtime. The fixture now gives
user HOME the private scratch parent and MAOS_HOME its distinct child, while
retaining the provisioned Podman storage configuration. Production HOME
refusal remains unchanged. Root-shutdown startup failure context now captures
stdout/stderr instead of an empty string. The terminal pin stays **34**:
both hosted images actually ran 34 tests (28 passed / six failed); no new
terminal test or accounting relaxation was introduced.

**Lesson:** every path-shape change needs a hosted-length vector, including
fixed-width synthetic creator/Spirit PID cases; short tempfile homes alone
did not exercise AF_UNIX capacity. The cut now enrolls 17 socket/purge contract
tests on both head images, alongside the unchanged 34 real delegated scenes.
F1/F2 source plants failed on the intended bind/refusal assertions, then were
copy-backed, compared and touched with all restoration exits 0. See
[`hosted-1/plants.md`](20-10-purge-residue-removal-evidence/landing-1/hosted-1/plants.md).

##### Corrected-source verification and remaining boundaries

Final anchors: hosted-length bind `egress_daemon_17_1.rs:252`; fixture
preflight `worker_terminal_facts_17_7/root_crash.rs:268`; dual-shape custody
`purge_residue_20_10.rs:202`; workflow contract
`xtask/tests/epic17_exit_workflow_17_7.rs:418`; diagnostics
`root_shutdown_unload_16_3.rs:316`. The source fix is `egress.rs:88,246-258,268`.
Three distinct plants (unit namespace, durable PID prefix, HOME equality)
each failed on the intended assertion, exit 100, then were byte-restored.

| Surface | Exact measured | Ceiling / current pin |
|---|---:|---:|
| maos-bin code | 34,191 | ceiling 34,548; pin 34,191 |
| Aggregate code | 192,519 | ceiling 192,876; hardfail 192,520 |
| Kernel physical / code | 26,841 / 21,042 | unchanged |
| xtask code | 44,406 | unchanged |

Final `cargo fmt --all`, exact re-pin, all-target workspace build and focused
suite passed (exit 0); focused **164/164**. All 16 registered fixture
artefacts were provisioned. Fully provisioned serial workspace nextest
(`--profile ci --test-threads 1`) passed **5573/5573**, 138 configured skips,
exit 0. Saved fresh serial junit passed the budget gate, exit 0:
594 binaries, 644.512 s sum, no findings. Host readings were recorded:
3.58 / 4.39 / 16.52 before the final serial run and 5.42 / 4.05 / 10.01
before the budget gate. This is not an exclusive-idle-host certification.

Default-concurrency nextest ran **5573**, **5572 passed / one failed**,
138 configured skips, exit **100**. The sole failure was the explicitly known
`t3_terminal_helpers_17_7::teardown_waits_for_a_stalled_snapshot_owner_then_removes_the_container_20_7`:
`instance 16: observer never entered inspect` (10.078 s). It was **not retried**;
owner `20-7-stable-runner-perf-gates`, same-line rule-10 evidence in
`deferred-work.md:5` and `hosted-1/workspace-default-final.log:7335-7377`.
All 20-10 witnesses passed. No kernel edit or timing/assertion relaxation.

All requested structural gates exited 0: kloc/kernel/service/env/serde,
composition roots, J1 loopback delegation, dev-record completeness,
epic coherence, review resolution, test cost, invariant lock, coverage matrix
(dry run) and completeness, exit commands, decision register, ff-j6 and
performance-runner policy. Non-denying clippy exited 0 on local Rust 1.98.1;
complete-stream ratchet **1335 / 873**, exit 0, unchanged baseline **1383 / 897**.
Lint/file/message comparison against the prior local stream has **no new
warnings**, including the result-large-err/collapsible-if/too-many-arguments
families; existing warnings in touched tests were not hidden or baselined away.
`cargo deny check` exited 0.

Prescribed undelegated prepush independently exited **0**. Workspace/default
profile tests, kernel lib tests and workspace doctests passed. This does not
erase the separate default-concurrency failure. Its budget leg was advisory,
exit 1: hook-pump 30.317 s, one-daemon 34.159 s, participant-recall 44.337 s,
all against unchanged 30 s binary budgets. Owner 20-7 and same-line evidence:
`deferred-work.md:3`, `hosted-1/prepush-final.log:536-540`, adjacent saved junit.
No advisory retry or budget change. Site-doc build gates were skipped because
this incident changed ADR-067, not docs-site inputs; the earlier English/Korean
verification remains in its historical record. Intent-lineage output matched
the saved original (cmp 0).

Earlier deterministic prerequisite failures remain visible: a focused run
had 18 stale debug-fixture failures after the separate 20-4 source park; after
debug-fixture provisioning the focused run passed. The first full serial run
had exactly 19 stale release-fixture failures (four equiv-native-twin, 15
maos-wasm-runner), classified in `hosted-1/serial-failure-classification.json`.
All registered fixtures were then rebuilt before the final serial/default
runs. Freshness guards were never suppressed. An earlier combined eval timed
out during compilation; no exit code was observed for that interrupted command.
The first clippy-ratchet input was a truncated artifact view (missing
build-finished, exit 1); paging the **same** successful stream, without rerunning
clippy, restored complete evidence and ratchet exit 0.

Exact commands, exits, overrides and logs:
[`hosted-1/commands.md`](20-10-purge-residue-removal-evidence/landing-1/hosted-1/commands.md)
and adjacent JSON. Source/cut diff digests are adjacent; prior snapshots remain
unchanged. Per-AC corrected-hosted proof routes and explicitly unverified legs:
[`hosted-1/ac-hosted-map.md`](20-10-purge-residue-removal-evidence/landing-1/hosted-1/ac-hosted-map.md).
Corrected real delegated baseline/head/plant scenes and post-fix hosted-toolchain
lint require an operator rerun; local self-skips are not that proof. No commit,
push, CI-branch edit, story/status flip or kernel modification was made.

#### Hosted follow-up — runs 41610868202 / 41610893716, 2026-11-03

Baseline for this scoped correction: `2752e750c8cc7cfcb6fed2ccabbd4e4e0ed4f040`.
The remaining hosted root-crash failure was a **fixture lock-ancestry defect**:
target `<scratch>/home` and foreign `<scratch>/foreign-home` shared the scratch
user HOME/ancestor. The foreign live process correctly held a shared ancestor
guard and prevented exclusive purge. This was neither a production lock bug nor
a timing flake. Exact job/log citations and separation of passing real cgroup
legs from the blocked AC1 base/plant:
[`hosted-2/branch-run-diagnosis.md`](20-10-purge-residue-removal-evidence/landing-1/hosted-2/branch-run-diagnosis.md).

The foreign root now owns an independent TempDir, declared before its LiveRoot
so process shutdown precedes temporary-ancestry deletion. A canonical
shared-ancestor invariant at `root_crash.rs:174-176` executes before the optional
T3 gate. Applied to HEAD's original fixture layout, the new witness failed
with exit100 on the intended shared-ancestor assertion. A nested foreign-temp
plant independently failed on the same invariant, exit100; source copy-back,
`cmp`, `touch` and final formatting each exited0. The restore intermediary was
removed. Restored focused serial suites passed **81/81**, including all **34**
terminal tests, exit0. CID removal, foreign CID survival, label-less impostor
survival, production ancestor locking and StoreInUse refusal remain unchanged.

Scoped scan: the live same-store refusal in `purge_residue_20_10.rs:321-370` is
intentional, not a success-expected foreign twin; other-home entries there are
stub metadata. `root_shutdown_unload_16_3.rs` has no such purge scene. Neither
needs relocation. The branch-only verification cut is unchanged: intended
owned-CID/dead-leaf diagnostics and byte-restoration checks remain strict.
No StoreInUse allowlist, signed-substrate bypass or test-count increase.

Formatting and KLOC gates exited0; inherited post-20-4 counts remain
**maos-bin34363 / aggregate192896 / xtask44420**, aggregate hardfail192897.
No production counted lines or pins changed. Kernel remains **26841 physical /
21042 code**, with no changed/added/removed files. Non-denying all-target
workspace clippy on Rust1.98.1 exited0; complete-stream ratchet exited0 at
**1331 occurrences /871 source-distinct**, unchanged1383/897 baseline. The sole
changed Rust file has zero primary warnings. Exact structural/record gate
commands and outcomes are recorded in the command ledger; no hosted-toolchain
lint or corrected hosted runtime result is claimed.

Cleanup scanned27 `/proc` PIDs with no active Rust/cargo tooling or candidate
reference and no inaccessible same-UID entry. After nonblocking session locks,
14 stale terminal incremental session directories were removed, preserving
current-task sessions, crate roots, lockfiles, deps and every other crate.
Their allocated-byte total was653934592; filesystem free-space increase after
Btrfs flush was12709888 bytes, not an exclusive-reclamation guarantee.
No cargo clean or broad target deletion. Witness TempDirs and restore scratch
are absent. Details: `hosted-2/cleanup-proc-scan.json` and `cleanup-result.json`.

Lesson/disposition is also recorded in the owned20-10 `deferred-work.md` row:
distinct MAOS_HOME leaves do not prove independent lock ancestry. Full witness,
mutation SHA-256, restore sequence, scan findings, proof boundaries and exact
commands:
[`hosted-2/plants.md`](20-10-purge-residue-removal-evidence/landing-1/hosted-2/plants.md)
and adjacent `commands.md` / `commands.json`. Parent scoped this follow-up to
focused serial suites, formatting, structural/record gates and clippy delta;
no new full workspace/default/prepush matrix was requested or run. A fresh
operator-owned hosted baseline/head/plant rerun remains required: the prior
StoreInUse failures are not accepted removal reds. No commit, push, story/status
flip, kernel or production-policy change.

#### Hosted AC5 cidfile-lifecycle correction — run 41803510235, 2026-11-03

Scoped baseline: `73e7afbcae81f4fb4858e9d52ded3d592bed636d`. The independent
foreign-home fixture now reaches production. Both real heads remove the owned
CIDs and preserve the foreign/impostor CIDs, then fail on spawn custody;
restored plant fails at the same point. Both real cgroup/native-binding legs
pass. Base and initial no-op plant hit their intended diagnostics. This is a
**production AC5 defect**, not StoreInUse or a timing flake. Exact hosted
job/log citations:
[`hosted-3/hosted-diagnosis.md`](20-10-purge-residue-removal-evidence/landing-1/hosted-3/hosted-diagnosis.md).

Cause: container removal preceded the first custody CID-file read, but Podman's
documented `rm` side effect deletes its cidfile. The old hosted logs did not
directly print that missing file; the historical causal link is source-supported
inference, independently reproduced by the new faithful cidfile-deleting stub.
Original production with the strengthened `purge_residue_20_10.rs:202` witness
exits100: `removed container's custody leaked after runtime deleted its cidfile`.

Inventory now captures each valid spawn CID with its held no-follow directory
before any runtime removal, validating private0700/euid ownership. Reclamation
requires a fresh bounded `container exists` exit1 proof and matching held/current
device/inode with renewed ownership/mode checks; final rmdir is bound to the
held identity. A missing/invalid CID at inventory never grants deletion. The
existing strong absent/present/unknown/live/legacy/receipt coverage is retained.
New negatives at `:265` and `:289` prove missing-at-inventory conservatism and
retention of both original/replacement directory data during an `rm` swap.

Capture-discard plant: storing `proof: None` exits100 on the intended
`absent CID custody leaked` assertion. Copy-back, `cmp`, `touch`, restore-scratch
removal and final formatting each exit0. Plant digest, exact commands and
scoped implementation/cut digests:
[`hosted-3/plants.md`](20-10-purge-residue-removal-evidence/landing-1/hosted-3/plants.md)
and adjacent `diff-sha256.json`, `commands.md` / `commands.json`. No active plant.

Custody-only local matrix, before the added race/S5 items: registered scene fixtures provisioned (5 artifacts,
exit0); **88/88** focused tests (egress5, uninstall23, purge14, shutdown12,
terminal34), exit0; library runtime safety **3/3**, exit0; executed workflow
contracts **5/5**, exit0. The focused command's25 excluded bin-unit tests do
not contain library purge tests; those are covered by the separate3-test run.
The new workflow test executes the actual restored checker: required PASS plus33
filter exclusions succeeds; explicit SKIP, wrong required name and zero run fail.
Hermetic cut count is19; terminal count remains34. Base/plant intended-red
diagnostics and copy/cmp/touch remain strict; existing removal plant still
applies. Hosted custody failures now include stdout/stderr, receipt and listing.

Formatted measurement and exact pins: **maos-bin34537 (+33)**, under the
20-10 ceiling34548; **aggregate193173 (+33)** over inherited post-20-4
baseline193140, under that inherited ceiling193526; hardfail **193174**
(measured+1). xtask44452 unchanged, no other crate grant changed. The first
measurement was blocked by a peer's evidence `.rs`; its owner renamed that
evidence, without this agent changing it. Old exact pins then correctly failed;
the measured re-pin passes. No scope exclusion or estimate was substituted.
Kernel source is untouched; final baseline/structural/record gate outcomes
are retained in the command ledger.

Non-denying all-target workspace clippy on Rust1.98.1 exits0; complete-stream
ratchet **1331/871**, exit0, baseline1383/897 unchanged. Full local comparison
against20-4's corrected stream finds no new lint/file/message multiplicities.
Touched maos-bin files have no warnings; the xtask test retains its existing
line15 `needless_borrows_for_generic_args` only. No warning suppression or
baseline change. Hosted-toolchain lint remains unverified.

Adjacent-consumer audit found only the inventory CID-file read in maos-bin;
egress and supervision use empty-root rmdir, not a post-removal CID recovery.
ADR-067 and canonical/site English/Korean run/cookbook wording now distinguish
captured CID authority from a merely missing file. No path shape, socket name,
environment variable, kernel file or producer lifetime change; existing hosted-
length vector is included in egress5. Receipt schema/vocabulary is unchanged.

After cargo finished,29 `/proc` PIDs were scanned; no active Rust tooling,
candidate references or inaccessible same-UID entries. Locked granular cleanup
removed19 stale sessions for the three affected test crates:234217472 allocated
bytes, observed free-space increase167460864 bytes after flush (filesystem-wide,
not exclusive physical-reclamation proof). Current sessions, crate roots,
lockfiles, other crates, deps and target roots remain. No cargo clean/broad
deletion; named workflow scratch and restore intermediary are absent.

The owned deferred/lesson row records the production cause and disposition.
Fresh corrected real delegated/Podman head/restored-plant green and hosted lint
remain operator-owned and unverified; local gated returns are not that proof.
Full workspace/default/prepush and docs-site builds were not requested/run in
this scoped follow-up. No commit, push, CI-branch edit or story/status flip.

#### Combined hosted correction — enumeration race and S5, 2026-11-03

Main added two further diagnoses from main run41803503598 to this same pass.
Workspace job126484784599 exposed a real shared-Podman enumeration race:
a listed candidate can disappear before inspect. `inspect` now skips only
nonzero status followed by a bounded `container exists` exit1. Present,
error or deadline stays fatal; inspect status/stderr are printed. The new
dry-run vector (`purge_residue_20_10.rs:164`) is red100 before the fix and when
the skip is planted away. The negative at `:200` covers exists0/125/timeout,
requiring refusal78, diagnostic evidence and no mutation/receipt. The conditional
read-only probe is explicitly allowed by Main's instruction and now reflected
in current AC1/AC3 and English/Korean/ADR wording; it is not a retry or a hidden
relaxation of the storage/mutation guard. The original AC5 fix remains intact.

Loom job126484785052's S5 failure is **test-only**, not another production
regression: two rows sampled2.422539097s and2.622919372s are200.380275ms apart,
matching warning-to-timeout separation. Frame kinds/reset trace were absent
from that log; warning/exceeded and clock-offset attribution remain source-
supported inference. The old ratio>=1 oracle deterministically counts2 for
those warning/exceeded payloads. `unload_scenes.rs:21` proves old-predicate
red100 and restored-old-filter plant red100. Rows retain `FrameKind`; the
live assertion counts exactly one BudgetExceeded and still checks the injected
hook/cap payload. Both World constructors share one `std::sync::Once` at the
terminal binary root. No production clock/budget change, count relaxation,
serialization workaround or retry-to-green. A same-line rule-10 note is appended
to the existing20-7-owned `deferred-work.md:4`, citing the200ms pair and18-0
clock-reinit precedent. Other timing families remain open.

**Budget re-base — 2026-11-03, orchestrator-adjudicated, D02 A.** Main permits
this combined required delta within already authorized inherited caps:
maos-bin<=34643, aggregate<=193526; STOP above, no ceiling widening. The old
34548 absolute cap was set against pre-20-4 bin34191; the inherited current
base is34504/193140. Final formatted measurement is **34551 /193187 (+47)**:
custody+33, enumeration+14; S5/test-file changes uncharged by the existing gate.
Exact pins are bin34551 and aggregate hardfail**193188** (measured+1), with
live20-10 key and `->` in first HISTORY segments. xtask44452 and kernel
26841/21042 are unchanged; no other grant changed.

After all three byte-restored mutations, final fmt exits0. Registered scene
fixtures are provisioned (5 artifacts,0). Combined focused serial nextest:
**91/91,0 skips, exit0** (egress5, uninstall23, purge16, shutdown12,
terminal35); library runtime safety**3/3,0**; workflow contracts**5/5,0**.
Discipline/cut terminal pin and required names are35; hermetic cut is21.
Restored cut requires exactly1 named PASS and permits34 filter exclusions,
while explicit SKIP/wrong name/zero run fail. Base/plant intended-red diagnostics
and byte-restoration remain strict; existing removal no-op patch still applies.
Expanded custody diagnostics print stdout/stderr, receipt and directory listing.

Final complete non-denying clippy exits0 on Rust1.98.1; ratchet**1331/871,0**
against unchanged1383/897 baseline. No new lint/file/message multiplicities
against20-4's corrected local stream; existing xtask1/supervisor2 warnings remain.
Structural/record gate outcomes and every command/exit are in the final ledger.
No hosted-toolchain warning result is inferred from this local comparison.

Initial custody-only phase evidence above is historical, not the combined final
tree. Final witness names/lines, three mutation hashes, implementation/cut
SHA-256, exact limits, proof boundaries and cleanup:
[`hosted-3/combined-plants.md`](20-10-purge-residue-removal-evidence/landing-1/hosted-3/combined-plants.md).
The granular19-session prune accounts for234217472 allocated bytes and observed
167460864-byte free-space increase after flush, with no broad target deletion.
Final27-PID scan has no active Rust tooling; all restore/named scratch is absent.
Corrected real hosted head/restored-plant, concurrent enumeration and S5 clock
behavior remain operator verification; local gated returns are not that proof.
No new full workspace/default/prepush or docs-site matrix was requested/run.
No commit, push, CI-branch or story/status flip; unrelated work is preserved.

**Done — landing record (2026-11-04):** GitHub-verified landings `75538bf7`, `16c35ef0`, `8c809f9c` (regression fixes), `ef134444` (independent foreign-live TempDir), and `76efc19d` (pre-removal CID custody capture, vanished-candidate inspect disambiguation, typed S5 BudgetExceeded counting).

- Hosted verify [41815301963](https://github.com/lunarpulse/maos/actions/runs/41815301963) on `ci/20-10-verify` is **fully green, 7/7 jobs**: lint, two T0 probes, head on ubuntu-24.04 and ubuntu-26.04, base and plant controls. Both heads used real rootless Podman and delegated cgroups (`Delegate=yes`). Base failed on both intended diagnostics, `owned CID … survived purge` and `dead-creator empty leaf survived`; the removal no-op plant failed on the owned-CID assertion and its restored run exited 0. Green control jobs mean the expected reds were observed, not that the unpatched base passed.
- Hosted T0 logs prove Podman **4.9.3 / 5.7.0**, real mount representation/normalization probes, paired `maos.spirit_id` / `maos.boot_nonce` filters, successful `rm` and absent `container exists` exit 1; Docker rejects `--pidfile` (exit 125). These are measured runtime contracts, not a claim of Docker T3 support.
- Main discipline [41910498869](https://github.com/lunarpulse/maos/actions/runs/41910498869) @ `cb0abc06` is **not fully green**: 188 jobs succeeded; `workspace-test-suite` failed on the known 20-7-owned snapshot-owner test `teardown_waits_for_a_stalled_snapshot_owner_then_removes_the_container_20_7` at `t3_terminal_helpers_17_7.rs:564` (**instance 11: observer never entered inspect** in the verified log, not supplied instance 16), 5619/5620 passed. `v1-0-ship-gate`, `v1-5-ship-gate`, and `aggregate` cascade from that one failure.
- **Closing metrics:** maos-bin **34,191 at the initial corrected landing** (`8c809f9c`); kernel unchanged **26,841 physical / 21,042 tokei**. Combined later correction at `76efc19d` measured bin **34,551**, aggregate **193,187**, hardfail **193,188**, as recorded above; do not mislabel 34,191 as the later cumulative tree.
- **Residual boundary:** unattributed historical anonymous custody roots remain retained and owned by `20-7-stable-runner-perf-gates` (`deferred-work.md`, historical T3 process-roots row); no creator-liveness evidence or authorization for historical cleanup is inferred. Proof-retention policy (purge deleting erasure-proofs versus NFR-Aud-12) remains ownerless and open, explicitly dispositioned under rule 10 in `deferred-work.md`; no keep/export/delete ruling or repair is claimed. Package-manager binary removal remains 20-2. Operator publication and historical-root investigation are not hosted acceptance claims.
- The three fixed 20-10-owned incident rows in `deferred-work.md` are closed with their landing commits and hosted witness; prior unverified bullets above remain historical. No gates were rerun for this documentation close.

### File List

- Done flip (2026-11-04): this story, `_bmad-output/implementation-artifacts/sprint-status.yaml`, `_bmad-output/implementation-artifacts/deferred-work.md` (three incident closures and one open rule-10 disposition), and `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md` story-table closure mirror.

- [Purge orchestration](../../crates/maos-bin/src/purge.rs), [new runtime reclamation module](../../crates/maos-bin/src/purge_runtime.rs), [creator-PID custody roots](../../crates/maos-bin/src/egress.rs).
- [New hermetic witnesses](../../crates/maos-bin/tests/purge_residue_20_10.rs), [purge fixture isolation](../../crates/maos-bin/tests/maos_uninstall_16_4.rs), [producer harness-cycle fix](../../crates/maos-bin/tests/worker_supervision_16_3.rs), [real hosted scenes](../../crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs), [source scanner enrollment](../../crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs).
- [Terminal job pin](../../.github/workflows/discipline.yml), [pin reader](../../xtask/tests/epic17_exit_workflow_17_7.rs), [exact KLOC pins](../../xtask/kloc.toml).
- [ADR-067](../../docs/adr/ADR-067-maos-data-root-layout-and-purge.md), [operator guide](../../docs/maos.dev/run-maos.md), [site guide](../../docs-site/docs/run-maos.md), [Korean guide](../../docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md), [cookbook](../../docs-site/docs/cookbook/cli-wrapper-spirit.md), [Korean cookbook](../../docs-site/i18n/ko/docusaurus-plugin-content-docs/current/cookbook/cli-wrapper-spirit.md).
- [FR2 coverage row](../../tests/coverage-matrix.yaml), [loaded-budget disposition](deferred-work.md), this Dev Agent Record, and [landing/hosted-cut evidence](20-10-purge-residue-removal-evidence/landing-1/plants.md).
- Cold-swap oracle incident: `crates/maos-bin/tests/one_daemon_one_door_16_1.rs`; the resolved cold-swap row in `deferred-work.md`; this record and the 20-1a Dev Agent Record. The earlier implementation-diff digest is not regenerated or expanded.
- Hosted incident: [AF_UNIX hosted-length witness](../../crates/maos-bin/tests/egress_daemon_17_1.rs), [startup failure context](../../crates/maos-bin/tests/root_shutdown_unload_16_3.rs), the existing terminal/custody witnesses, [workflow contract](../../xtask/tests/epic17_exit_workflow_17_7.rs), [corrected branch-only cut](20-10-purge-residue-removal-evidence/ci/ci-20-10-verify.yml), ADR-067, exact pins and [hosted incident evidence](20-10-purge-residue-removal-evidence/landing-1/hosted-1/hosted-findings.md).
- Hosted shared-ancestry follow-up: [independent foreign-root fixture and pre-gate invariant](../../crates/maos-bin/tests/worker_terminal_facts_17_7/root_crash.rs), the owned lesson row, this record, and [focused follow-up evidence](20-10-purge-residue-removal-evidence/landing-1/hosted-2/plants.md). No workflow, production source or pin change.
- Hosted AC5 lifecycle correction: validated pre-removal custody capture in `purge_runtime.rs`, faithful runtime stub plus missing-CID/inode-swap negatives, expanded real-scene diagnostics, restored-log checker/workflow regression, ADR/operator English/Korean wording, exact maos-bin/aggregate pins, owned lesson/record, and [hosted-3 evidence](20-10-purge-residue-removal-evidence/landing-1/hosted-3/plants.md). Other work remains untouched.
- Combined enumeration/S5 addition: `purge_runtime.rs` failed-inspect disambiguation and diagnostics; new purge race vectors; shared terminal clock initializer plus typed unload rows/oracle; supervisor caller; discipline/cut count35 and required-name pins; workflow regression/count21; current dry-run contract wording; exact combined pins, owned production lesson and same-line existing20-7 flake note; [combined final proof](20-10-purge-residue-removal-evidence/landing-1/hosted-3/combined-plants.md).


## Change Log

- 2026-11-02: Story created at `a8af3215` (ready-for-dev; D-20-10-01…10 PROPOSED). E20 §20-10 cites corrected in place
  (C1–C3). Measured: purge reports containers by name prefix and a cgroup path nothing creates; the real owned cgroups
  (`maos.slice/spirit-*`) and the T3 custody under the worker state root (8,903 empty process roots on the dev host) are
  outside purge. Kernel 0 recommended; KH/KR/KC/KS priced +10/+8, +2/−1, +5/+1, +3/+2.
- 2026-11-02: Operator rulings recorded (ask s2010): D-20-10-01…10 as recommended; kernel-Δ 0 and ceilings per frontmatter. Status remains ready-for-dev.
- 2026-11-02: 20-10 landing 1 implemented with kernel-Δ 0, exact maos-bin 34037 /
  aggregate 192365 (hardfail 192366), six locally proven-red byte-restored plants,
  final serial/default workspace suites and prescribed prepush rc 0. Hosted-only
  witnesses and the AC1 mutation cut but not pushed/run; status intentionally unchanged.
- 2026-11-02: Hosted run `41521146083` cold-swap failure diagnosed as an over-strict cross-writer journal oracle, with a deterministic recorded-order red-before vector and causal/audit-drained replacement. No purge, kernel, production or budget change.
- 2026-11-03: Review corrections retain live/anonymous T3 roots and shared parents, validate HOME/legacy ancestry/protected paths, make residue an explicit nonzero outcome, document the same-UID attribution boundary, and strengthen all identified witnesses. Exact maos-bin 34188 / aggregate 192516 (hardfail 192517), kernel unchanged; five baseline-red vectors and nine byte-restored assertion-red plants; focused, serial/default, gates, clippy, deny and prepush observed green except separately recorded prepush budget advisories. No status flip or push.
- 2026-11-03: Hosted incidents 41584515474 / 41584453971 corrected at cause: use the existing delegated-unit namespace; confine creator-PID naming to volatile custody so durable AF_UNIX socket paths stay short; separate terminal fixture HOME/MAOS_HOME and expose startup stderr. Three byte-restored assertion-red plants, focused164/164, serial5573/5573 and prepush0; default's known snapshot-owner failure remains recorded/unretried under owner20-7. Exact bin34191 / aggregate192519 (hardfail192520), kernel unchanged; corrected hosted cut remains unpushed.
- 2026-11-03: Hosted runs41610868202 /41610893716 classified as a shared-ancestor fixture defect; foreign LiveRoot moved to independent TempDir with a pre-T3 canonical ancestry invariant. Original-layout and nested-layout reds100; byte-restored focused serial81/81, terminal count34 unchanged; local clippy1331/871 with no changed-file warnings. Production lock policy and intended hosted red diagnostics unchanged; operator rerun still required. Inherited post-20-4 counts unchanged.
- 2026-11-03: Run41803510235 exposes real AC5 late-cidfile-read defect after the repaired fixture reaches removal. Capture validated CID/held identity before Podman rm, require fresh absence and identity on reclaim; missing-at-inventory stays conservative. Faithful-stub red100 and capture plant red100; byte-restored focused88/88, library3/3 and workflow5/5 green. Hermetic cut19, terminal34; restored checker permits filter exclusions only with exact required PASS. Formatted bin34537 / aggregate193173 (hardfail193174), xtask/kernel unchanged; local lint1331/871 with no new warnings. Corrected hosted proof remains operator-owned.
- 2026-11-03: Orchestrator-adjudicated, D02 A combined re-base within inherited bin34643/aggregate193526 caps. Add conservative vanished-candidate disambiguation with inspect status/stderr and read-only fallback; repair S5 typed-kind oracle/shared clock Once without production/budget change. Both new red-before vectors and byte-restored plants100; combined serial91/91, library3/3, workflow5/5 and lint1331/871 green. Exact bin34551/aggregate193187/hardfail193188, xtask/kernel unchanged; terminal35/hermetic21. Corrected hosted proof remains unverified; no push/status flip.
- 2026-11-04: Done; GitHub-verified landings through 76efc19d and hosted verify 41815301963 (7/7, intended base/plant reds and restored/head greens) recorded. Main discipline 41910498869 retains the known 20-7 snapshot-owner flake and cascading ship gates. Three incident rows closed; historical roots remain 20-7-owned; proof-retention policy remains explicitly ownerless/open under rule 10.
