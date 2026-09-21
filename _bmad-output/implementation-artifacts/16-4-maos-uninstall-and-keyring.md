---
baseline_commit: "**`822070d7`** (Story 16-3 landed). Authored 2026-09-16 from five scouts — a STATIC scout over the verb table / `uninstall` occupancy / dispatcher shape, a STATIC scout over every on-disk root and resolver, a STATIC scout over the secrets + provider + env-contract surface, a RUNTIME scout that built the real binaries and drove `init`/`shell`/`run`/one-shot roots in scratch `HOME`/`MAOS_HOME`/`XDG_DATA_HOME` (with `/proc/locks` and WAL-size measurements), and a BUDGET/CI/SPEC scout that RAN `kloc-check`, `check-kernel-baseline` and `check-exit-commands`. **Nine of the epic section's ten line-anchor groups are stale**; every doctrine behind them survives. **Cites are SYMBOL-first, line second** wherever a decision rests on them (round-table 2026-09-16, §15 R2/F6) — validation round 1 found **19 of ~120 line-only cites already wrong at an unmoved HEAD**, which is a citation-form defect, not drift. ⚠ **T0 re-measures. A grant is a global, not a reservation.** ⚠ **HEAD IS CI-RED AND 16-4 DID NOT CAUSE IT** — see §0."
depends_on: "**`16-1-daemon-post-surface-and-verb-retarget` (`done`)** — `acquire_store_lock_set` / `StoreLockRole::OfflineExclusive` / `StoreLockSet::locked_paths` (`crates/maos-bin/src/operator_door.rs:1590-1837`), the typed 69/78 refusals (`main.rs:1982-2004`), `maos_domain::operator_door::maos_home` (`crates/maos-domain/src/operator_door.rs:63`) and `control.json` minting in `run_init`. **`16-2-shell-halt-registry-and-j0-scene` (`done`)** — D-16-2-F's fail-closed name resolution, whose production comment (`crates/maos-audit/src/lib.rs:1751-1755`) names `maos purge` as the erasure route for legacy rows, and the `run-maos.md` key-source paragraph this story amends. **`16-3` (`done`)** — `SCANNED_SOURCE_FILES` is 22. **`15-3`** — `verbs.rs`'s `VERBS`/`VerbName` table and `check-exit-commands`, which already OWES `purge` to this key."
blocks: "**`epic-16` exit line 7** (`maos purge --keep-log`) and therefore the epic's close — `check-exit-commands` PASSES today only because this key is `backlog`. **`epic-16-retrospective`** (four residual items, §11). **`20-2`/`20-3`** — `epic-20-ship-it-w5.md:83` AC3 still says `maos uninstall`, a verb that will never exist; corrected in §12."
spec_alignment: "⚠ **THE EPIC IS AMENDED IN THE SAME COMMIT (rule 9, Round 8 — the 16-4 section, the stories-table row, the Kloc-asks paragraph, the doorbell sentence, and `epic-20-…md:83`).** Measurement disproved premises in all three of the section's ACs. AC1's install-prefix clause is **not implementable** (nothing records a prefix and `run_init` never calls `current_exe()`); its root list is short by **two whole trees** (`$XDG_CONFIG_HOME/maos`, which holds `audit-signing.key`, and the machine-global `/tmp/maos/crl`) and three env overrides; and `--keep-log` **cannot mean one file** — on the default path the Transparency Log sqlite IS also the shared memory store and the principal namespace index (`main.rs:2016,2080,2084`), in tenant mode those are **different files**, and after a one-shot exit 189 KB of audit lives in the `-wal` while the `.sqlite` is a 4 KB empty header. AC2's inode re-check **already exists** (16-1 pre-built it, naming `maos purge` in the comment), its `FIXME(secrets)` cite is off by 415 lines and misses a second one, and its `MAOS_SECRETS_BACKEND` registration would be **vacuous** where the epic puts the reader — a defect the tree already demonstrates with `MAOS_CRL_PATH`. ⚠ **AC2/AC3 implement a capability the binding PRD does not list and NFR-Sec-19 explicitly defers** (§6); this story authors the amendment rather than drifting. The rulings are the Decisions table; the applied epic edits are §12."
split_from: "Not a split. Authored from `epics/epic-16-one-daemon-one-door-j0-w1.md` 16-4 section. **Sizing: WHOLE — RECOMMENDED, operator ratification pending (§14 Q1).** The obvious seam is `16-4b` = the keyring half (AC5/AC6). It is refused because (a) the two halves meet at a real coupling — once MAOS stores a provider key in the OS keyring, that key is MAOS state and `maos purge` must remove it, so a split ships either a purge that leaves secrets behind or a keyring nothing erases; and (b) both halves land in the same four files (`main.rs`'s composition root, `env_contract.rs`, the three `run-maos.md` copies). Measured duration **8–12 d**."
kernel_grant: "**NONE. ZERO kernel-Δ @ `822070d7`.** No file under `crates/maos-kernel-core/src` is edited; `check-kernel-baseline` must report `changed == 0, added == 0, removed == 0` over the pinned 98-file set (`xtask/kernel-core-baseline.toml:488` `src_lines = 24477`, file-set text `:493`, `[kernel_src]` `:521-525`). The one kernel-adjacent fact this story routes AROUND rather than through: `/sys/fs/cgroup/maos/spirit-<pid>/` is written by `crates/maos-kernel-core/src/scheduler/resource_ceiling.rs:103-107` — which has **zero production callers at HEAD** (17-2's charter) — so purge only REPORTS leaked subtrees, from maos-bin, and never removes one (§2, §11 row 9)."
kloc_grant: "MEASURED at `822070d7` by `cargo run -p xtask -- kloc-check --json`: `maos-bin` **21048/21048 — ZERO headroom** (`kloc.toml:475`), `maos-secrets` 161/1000 (+839, `:450`), `maos-providers` 1252/2000 (+748, `:397`), `maos-domain` 9041/9192 (**+151 only**, `:395` — and `:394` books `16-4`/`16-5` **jointly**, so 16-5 competes for the same 151), `maos-shell` 572/573 (**+1**, `:642`), `maos-audit` 7341/7341 (**ZERO**, `:386`), `xtask` 44101/44101 (ZERO, `:320`), aggregate **164216** against an alarm of 158608 (`:655`) — **the alarm is already FIRING at HEAD**. The epic's Kloc-asks figures for `maos-bin`, `maos-domain`, `maos-shell`, `maos-cli`, `maos-control`, `maos-journey-test`, `xtask` and the aggregate are **all stale** (§12). `maos-bin` **will cross** and takes a measured raise under the RECOVERY-LANE CEILING RULE (`kloc.toml:49-95`): lane OPEN (`epic-17`…`epic-21` all `backlog`), `maos-bin` is not in `ZERO_HEADROOM_CRATES` (`xtask/tests/recovery_lane_ceiling_rule.rs:46` = `[\"maos-kernel-core\"]` only), and this key is not `done` ⇒ authorized, provided the code exists and is `cargo fmt --all`-measured BEFORE the ask and the figure + driver land in the same commit. ⚠ Inline `#[cfg(test)]` is charged; integration tests under `tests/` are not."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs`) is the machine-checked set; `equiv` is prose, NOT a match token. Keep the literal `allowlist {` — it is a NEGATIVE marker: `check_dev_model_used_populated.rs:302` treats any line containing it as boilerplate and SKIPS it, so the reminder can never be mistaken for the recorded model."
review: "§A6 full-layer net BINDING (Blind + Edge Case + Acceptance + Test-Infra + non-author runtime). Not marked ◆ in the epic, but this story **ships an irreversible destructive command whose failure mode is silent under-deletion** (the defect it exists to fix is exactly a removal instruction that looked right and left the data), **decides what `--keep-log` keeps in a file that is simultaneously three stores**, and **adds a dependency tree to a workspace whose `cargo deny` is advisory by accident** (`.github/workflows/discipline.yml:38`) ⇒ **NON-DEGRADABLE RECOMMENDED (§14 Q5)**. E15-A6 binding by name — *does this test read the tree, or only what it set itself?* Obligations (a)–(r) under **Review obligations**."
---

# 16-4 — MAOS can name every byte it wrote, and remove all of it

Status: done

> **The capability:** *An operator who has run MAOS can remove it. `maos purge` enumerates every root MAOS
> actually writes — the home, the XDG data tree, the config tree that holds signing keys, the CRL directory and
> any leaked cgroup subtree — refuses while a daemon or an offline durable operation holds any of them, and
> removes them, naming each one. `--keep-log` keeps the Transparency Log intact as a unit — database, WAL and
> shared-memory index together — and says plainly what that necessarily also keeps. `maos init` stops printing a
> removal command that does not remove the data, and prints the one that does. And a provider key comes from the
> OS keyring first, with the environment as a journaled fallback.*

**Closes:** **FR2** (`prd/functional-requirements.md:25`) — **PARTIAL and already recorded as such**
(`epics/requirements-inventory.md:541`) · **NFR-Ops-1**'s v0.1 uninstall half
(`prd/non-functional-requirements.md:156`, *"install, upgrade, yank, uninstall, revoke. v0.1
(install/uninstall)"* — unclaimed by the epic and the strongest supporting NFR) · the doctrine line
*"every install verb has a documented inverse"* (`prd/developer-tool-specific-requirements.md:50`; `maos --help`
ships `install` and no inverse today) · six routed obligations (§11 has the ledger; §5 of this story lists them).

**Does NOT close, and says so:**
- **FR2's "ACP sockets"** — **they have never existed.** ACP is NDJSON over stdio
  (`crates/maos-acp/src/lib.rs:3`); `UnixListener`/`UnixStream`/`*.sock` have **zero hits** across `crates/*/src`.
  Purge removes nothing here because there is nothing. Stated, not silently dropped.
- **NFR-Aud-12** (`non-functional-requirements.md:72`, an externally-verifiable uninstall receipt, v1.0) — purge
  writes a human report, not a signed receipt. §11 row 1.
- **ADR-005 / architecture §4.3.2's "materializes the key just before the HTTPS request"** — this story resolves
  the key at provider construction, because `Provider::credential_fingerprint` (`provider.rs:28`) hashes it and
  NFR-Scale-4's per-credential rate-limit buckets depend on that value being stable. §11 row 2.
- **The `run_uninstall_cascade` private-tier COUNT defect** (D-16-2-O) — purge erases the private-tier root, which
  is the obligation routed here; making `shared_tier_principal_row_count` also count private-tier rows is a change
  to the *cascade's* emptiness proof, not to purge. §11 row 3.
- **`maosctl purge`** — removal is a local, offline, whole-host act; it has no door route by construction (D-16-4-H).

---

## 0. 🔴 HEAD IS CI-RED BEFORE THIS STORY STARTS

`cargo test -p xtask --test recovery_lane_ceiling_rule` **FAILS at `822070d7` on a clean tree** —
`kernel_core_ceiling_has_not_moved_under_the_easing` panics at `xtask/tests/recovery_lane_ceiling_rule.rs:274`
with *"maos-kernel-core ceiling moved 18935 -> 18938. The RECOVERY-LANE CEILING RULE eased the per-crate BUDGETS
only; the kernel pin and the per-line FLAG-Winston grant were explicitly NOT eased (Epic-15 retrospective,
2026-09-12). Raising this row needs its own operator decision, not the easing."* — `test result: FAILED. 9
passed; 1 failed`.

`recovery_lane_ceiling_rule.rs:268` pins `const RATIFIED_AT_EASING: i64 = 18_935`; Story 16-3's
operator-authorized review closure moved `xtask/kloc.toml:247` to `18938`. **The constant is what is stale, not
the ceiling** — the raise it refuses is one the operator already made. This is inside `workspace-test-suite`
(`discipline.yml:3558`, cmd `:3637`), enrolled in both `aggregate.needs:3937` and `v1-0-ship-gate.needs:3725`,
`blocking` at every phase (`xtask/gate-registry.toml:328-329`).

**OPERATOR-RULED 2026-09-16: fixed inside this story** (§14 Q6 — the recommendation was to reopen 16-3; the
operator ruled otherwise). T0 therefore **records the red first, with this transcript, before touching it**, then
moves `RATIFIED_AT_EASING` 18935 → 18938 in its own commit whose message states that it ratifies Story 16-3's
already-authorized kernel-core raise and changes no ceiling. D-16-4-P; §11 row 4 is closed, not routed. The
discipline that matters: **the red is attributed to 16-3 in the commit message even though 16-4 lands the fix**,
so the ledger does not record 16-4 as having caused it.

**Review round 2 (2026-09-16) — two further HEAD reds at `822070d7`, beyond the red declared above.**
(a) `xtask/tests/kernel_pin_content_hash_16_0.rs` asserted `24474` while
`kernel-core-baseline.toml:488` already read `24477` — red:
`the_three_other_readers_still_resolve_the_pin`; (b)
`crates/maos-audit/tests/fr4_classifier_16_2.rs`'s strict token pin redded against 16-3's
`TokenColumn::Optional` row. Both repairs landed inside this story's working tree and are attributed to
their originating stories — (a) to 16-0's pin lineage, (b) to 16-3's D-16-3-J — per D-16-4-P's discipline,
so the ledger does not record 16-4 as having caused either red. Both are constant-only repairs, covered by
the operator-ratified single-commit exception established above.

---

## What this story actually is

The epic describes a verb and a backend. Measurement says the verb is the *smallest* part, and that shipping it
against the epic's root list would ship a removal command that still leaves data behind — the exact defect being
fixed. At `822070d7`, in the order a dev will hit it:

1. **The removal instruction MAOS already prints is false, on the default path.** (§1)
2. **Nothing in the tree can enumerate MAOS's roots** — 30 resolvers under 8 mutually-disagreeing policies, and
   five runtime trees. (§2)
3. **`--keep-log` is not expressible as written** — the file it would keep is three stores, and the audit is in
   the WAL. (§3)
4. **Three of AC1's premises are false and one of AC2's is already built.** (§4)
5. **Six obligations were routed here by 16-1, 16-2 and `deferred-work.md`; the epic's AC text carries two.** (§5)
6. **The keyring half has no requirement, and the binding PRD defers it.** (§6)

So this is **the story that makes MAOS's own footprint knowable** — one enumeration, one true instruction, one
honest `--keep-log` — and then activates the keyring adapter ADR-051 already parked behind `KeyManagementPort`.

---

## 1. 🔴 `maos init` PRINTS A REMOVAL COMMAND THAT DOES NOT REMOVE THE DATA

`crates/maos-shell/src/lib.rs:132-141`, two adjacent statements:

```rust
let audit_path = transparency_log_path()?;
print_line(… &format!("maos: Transparency Log will be at {}", audit_path.display()))?;
print_line(… &format!("maos: to remove all data, run:  rm -rf {}", home.display()))?;
```

`home` comes from `maos_home()` (`MAOS_HOME` else `$HOME/.maos`). `audit_path` comes from
`maos_audit::default_transparency_log_path()`, which falls through `MAOS_HOME` → `MAOS_AUDIT_DB` →
`$XDG_DATA_HOME` → `$HOME/.local/share` → `/var/lib`. **With `MAOS_HOME` unset — the `cargo install maos` default
— the two lines name different trees.** Measured on rebuilt binaries in a scratch `HOME`:

```
maos: initialized <H>/.maos
maos: Transparency Log will be at <H>/.local/share/maos/audit/transparency.sqlite
maos: to remove all data, run:  rm -rf <H>/.maos                  <-- names the tree with NO data
```

After `maos init` + one `maos shell` turn, the default-path tree is:

```
<H>/.local/share/maos/audit/transparency.sqlite     106496 B   <-- the real audit log
<H>/.local/share/maos/journal/lifecycle.ndjson         135 B   <-- the real journal
<H>/.local/share/maos/{memory,erasure-proofs,spirit-archives}/
<H>/.maos/{config.toml,control.json}
<H>/.maos/audit/      0 B   <-- created by init, NEVER written
<H>/.maos/journal/    0 B   <-- created by init, NEVER written
<H>/.maos/logs/       0 B   <-- created by init, NO WRITER EXISTS ANYWHERE IN THE TREE
```

The empty `audit/` and `journal/` decoys are what make the wrong root *look* right. Following MAOS's own printed
advice deletes two empty directories and leaves every byte.

The message has **exactly one site** and **no test pins it** (`grep -rn "to remove all data" crates/ tests/ docs/`
⇒ one hit). `crates/maos-shell/tests/init_test.rs` asserts with `contains`, never on an exact line set, so
correcting the line costs no test churn — and the new truth can be pinned by an assertion that did not exist.

⚠ Also measured: **`maos shell` works with no `maos init` at all** and creates only the XDG tree. So
`config.toml` is not a sentinel for "MAOS is installed", and `$XDG_DATA_HOME/maos` — not `MAOS_HOME` — is the
mandatory root.

---

## 2. 🔴 NOTHING CAN ENUMERATE MAOS's ROOTS — 30 RESOLVERS, 8 POLICIES, 5 TREES

`crates/maos-domain/src/operator_door.rs:3-7` names **four** disagreeing resolvers. There are **eight policies**:

| policy | honours | members (file:line) |
|---|---|---|
| **A** | `MAOS_HOME` else `$HOME/.maos` | `operator_door.rs:63` (canonical) · `maos-shell/src/lib.rs:722` (wrapper) · `maos-bin/src/migration_plan.rs:184` (**partial — `MAOS_HOME` only, no `$HOME/.maos` leg; returns `None` if unset**) |
| **B** | `MAOS_HOME`→var→XDG→`~/.local/share`→`/var/lib` | `maos-audit/src/lib.rs:932` (TL) · `:1476` (journal) |
| **C** | var→XDG→…, **ignores `MAOS_HOME`** | `:1515` memory · `:1553` distillate-corpus (**DEAD**) · `:1584` isolation-corpus (**DEAD**) · `:1615` archives · `:1643` erasure-proofs |
| **D** | tenant routing over B — **four near-identical copies** | `maos-audit` `:971`/`:989`/`:1030` · `maos-bin/src/main.rs:111` · `maos-cli/src/subcommands.rs:4494` · `maos-shell/src/lib.rs:24` |
| **E** | `XDG_CONFIG_HOME`→`~/.config/maos`→`/etc/maos` | `maos-domain/src/audit_key.rs:55,88,101,108` |
| **F** | hardcoded `$HOME/.config/maos` — **ignores `XDG_CONFIG_HOME`, disagrees with E** | `maos-kernel-core/src/security/operator_config.rs:51,178` · `maos-cli/src/subcommands.rs:4872` · `maos-spirit-cli/src/signing.rs:47` |
| **G** | hardcoded `$HOME/.local/share/maos` — **ignores BOTH `XDG_DATA_HOME` and `MAOS_HOME`**; `/tmp` on unset `HOME` | `maos-skill/src/store.rs:132,236` · `maos-registry/src/storage.rs:90,435` · `maos-registry/src/yank.rs:237` |
| **H** | other hardcodes | `maos-skill/src/discovery.rs:40,53` (`~/.maos/skills` — **a `MAOS_HOME` install's skills are invisible**) · `maos-registry/src/import.rs:236` · `main.rs:3075` (CRL) · `subcommands.rs:4293` · install target ×2 |

**Measured at runtime, MAOS writes four trees** (a fifth was claimed and is withdrawn below):

| # | tree | controlled by | created by | holds |
|---|---|---|---|---|
| 1 | `$MAOS_HOME` else `$HOME/.maos` | `MAOS_HOME`, else `HOME` | `init` | `config.toml`, **`control.json` (bearer token — a secret)**, four dirs |
| 2 | `$XDG_DATA_HOME/maos` else `$HOME/.local/share/maos` | `XDG_DATA_HOME`, else `HOME` | **`shell`/`run`, not `init`** | audit (+`-wal`/`-shm`), journal, memory, erasure-proofs, spirit-archives, **registry**, **skills** |
| 3 | `$XDG_CONFIG_HOME/maos` else `$HOME/.config/maos` | `XDG_CONFIG_HOME`, else `HOME` | lazily, on key use | **`audit-signing.key`**, `operator.toml`, `spirit-signing.key` |
| 4 | `/tmp/maos/crl/` | **NOTHING — hardcoded `main.rs:3075`** | polled every interval | `latest.signed.json` |

Trees 3 and 4 appear in **no** enumeration in the tree, including 16-1's store lock set. Tree 4 is a
three-way contradiction: the code hardcodes `/tmp/maos/crl` (world-shared across accounts on a multi-user host),
`crates/maos-domain/src/revocation.rs:427` documents `~/.local/share/maos/crl/`, and **`MAOS_CRL_PATH` is
registered at `env_contract.rs:245` as `UserFacing` with ZERO readers repo-wide** — a dead contract entry, and a
worked example of the gate defect §6 describes. ⚠ **Tree 4 is machine-global**: it is shared by every concurrent
test and with the developer's own host, and `HOME`/`MAOS_HOME` cannot isolate it. AC6's `MAOS_CRL_PATH` wiring
therefore lands **before** T1's enumeration, and every purge test sets `MAOS_CRL_PATH` into its scratch tree.

⚠ **A FIFTH TREE WAS CLAIMED HERE AND IS WITHDRAWN — validation round 1.** `/sys/fs/cgroup/maos/spirit-<pid>/`
(`crates/maos-kernel-core/src/scheduler/resource_ceiling.rs:103-107`) **is never created at HEAD**:
`apply_resource_ceiling` has **zero production callers** — only `crates/maos-kernel-core/tests/
cgroup_ceiling_smoke.rs:27,49,59,77,82` and the `pub mod` line at `scheduler/mod.rs:21`. Wiring it is **17-2**'s
charter (`sprint-status.yaml`: *"`apply_resource_ceiling` wired"*). A row that cannot have been measured must not
claim it was. It is further **not removable the way a purge would remove a directory**: a cgroup v2 directory
always holds kernel-generated files that cannot be unlinked, so `remove_dir_all` fails `EPERM`; only `rmdir(2)`
works, only for a uid with write on the parent (`/sys/fs/cgroup` is `root:root` here), and it returns `EBUSY`
while any pid remains — which is why the production `Drop` is `let _ = std::fs::remove_dir(dir)` (`:48-49`), not
`remove_dir_all`. **Purge therefore REPORTS leaked `maos` cgroup subtrees and removes none**, and the report is
conditioned on 17-2 having landed. §11 row 9.

**What already exists and must be reused, not re-derived:** `acquire_store_lock_set`'s candidate list
(`crates/maos-bin/src/operator_door.rs:1689-1706`) resolves six of these through the existing resolvers —
*"none are re-implemented here"* (`:1654-1656`) — canonicalises, sorts, and dedups by `(st_dev, st_ino)` of the
open handle (`:1770-1773`). The existing offline one-shot already prints it verbatim (`main.rs:1968-1979`):

```
maos: no daemon holds <H>/.local/share/maos/audit <…>/erasure-proofs <…>/journal <…>/memory
      <…>/spirit-archives <H>/.maos; running offline
```

⚠ Two traps for a purge caller. (i) **The set CREATES every missing candidate at mode 0700 before locking**
(`:1714-1744`, comment `:1720-1724` *"The proofs dir is created here ahead of its lazy first use"*) — an `acquire` on a clean machine
materialises the tree purge is about to delete. (ii) `store_dir` (`:1675-1687`) returns a **file path's parent**,
so a redirected `MAOS_AUDIT_DB` locks an unrelated directory; purge must never blindly `remove_dir_all` over
`locked_paths()`.

---

## 3. 🔴 `--keep-log` IS NOT EXPRESSIBLE AS WRITTEN

**(a) The file is three stores — on the default path, and NOT in tenant mode.** `crates/maos-bin/src/main.rs:2016`
takes `maos_audit::default_transparency_log_path()` as `memory_db_path`, then opens `SharedMemoryStore::open`
(`:2080`) and `PrincipalNamespaceIndex::open` (`:2084`) **on that same path**. Keeping the Transparency Log
therefore necessarily keeps the shared-tier memory rows and the principal namespace index — and FR2 names
*capability tokens* among what must go, which live in that database. `--keep-log` is an operator-chosen,
documented exception to FR2, and the docs must say exactly what it retains.
⚠ **But `:2017` binds `audit_db_path = resolved_transparency_log_path()`, and the two diverge in tenant mode**:
with `MAOS_LOOM_POSTGRES`/`MAOS_LOOM_HOME_TEAM` set, `maos_audit::transparency_log_path_for_team`
(`crates/maos-audit/src/lib.rs:971-982`) returns `<parent>/teams/<team>/transparency.sqlite` while the memory and
principal stores stay on the default path. A `--keep-log` written against "one file, three stores" would keep the
team log and **delete the memory and principal database** — and the sentence the docs print would be false.
D-16-4-E rules both paths.

**(b) The audit is in the WAL.** Measured file sizes on rebuilt binaries:

| exit path | `.sqlite` | `-wal` | `-shm` |
|---|---:|---:|---:|
| `maos run` + SIGTERM (clean) | 114688 | **absent** | **absent** |
| `MAOS_ONE_SHOT=uninstall` exit | **4096 (empty header)** | **189552** | 32768 |

WAL is set at `crates/maos-iac/src/adapter/transparency_log.rs:515`. **A `--keep-log` that preserved only
`transparency.sqlite` after a one-shot would hand back an empty database and silently destroy the entire audit
log** — the same class of defect as §1, one layer down. `maos-audit/src/lib.rs:343-357` already refuses replay
when a `-wal` exists (ADR-028 D6), so "checkpoint, then keep one file" is a real option; "keep the triple" is the
other. `-shm` is **named nowhere in the tree** — handling it is new code either way.

**(c) The name is ambiguous.** `$MAOS_HOME/logs/` exists, is created by `init`, and **has no writer anywhere**.
`--keep-log` must be documented as the *Transparency Log*, and the dead `logs/` directory is removed at origin.

**(d) Sidecars a `transparency.sqlite*` glob misses.** `with_extension` **replaces**, so
`transparency.airgap-stub.log` (`main.rs:1560`) does not match. The full set: `-wal`, `-shm`, `.team`
(`maos-audit/src/lib.rs:1030-1034`), `export-seq.json` (`maos-cli/src/subcommands.rs:4293-4328`), `transparency.airgap-stub.log`,
`transparency.siem-snapshot-<pid>-<n>.sqlite` (`enterprise_identity.rs:417-421`, best-effort cleanup `:453` — a
crash leaks it), and the whole `teams/<team>/` subtree (`maos-audit/src/lib.rs:971-982`).

---

## 4. 🔴 THREE OF AC1's PREMISES ARE FALSE; ONE OF AC2's IS ALREADY BUILT

**(a) The install prefix does not exist.** AC1 removes the binary *"only when it was installed under a prefix
recorded at `maos init`"*. Nothing records a prefix: `run_init` (`maos-shell/src/lib.rs:54-144`) writes
`config.toml`, `control.json` and a skills copy, and **never calls `current_exe()`**. There is no `justfile`, no
`Makefile`, no installer script; the documented install is `cargo install --path crates/maos-cli`
(`tests/coverage-matrix.yaml:346`), so "the prefix" is `$CARGO_HOME/bin/maos` — owned by cargo. `maos init`
structurally cannot know it.

**(b) `config.toml` cannot be read to decide what to delete.** Its `[paths]` block is hardcoded and provably
false — written by a run with `MAOS_HOME` and `XDG_DATA_HOME` both set elsewhere, it still says:

```toml
[retention]
default = "persist"
# Set to "ephemeral" to remove ~/.maos/ on `cargo uninstall maos` (manual).
[paths]
home = "~/.maos"
audit = "~/.maos/audit"
journal = "~/.maos/journal"
logs = "~/.maos/logs"
```

All four are wrong on any non-default home, and the `retention = "ephemeral"` hook describes behaviour **no code
implements**.

**(c) The inode re-check already exists.** `operator_door.rs:1783-1797` already does `fstat(handle)` vs
`stat(path)` after every `flock`, with a comment naming this story's verb: *"`maos purge` may have unlinked and
re-created the directory between open and lock: a stale handle must not be trusted."* 16-1 pre-built it (V-30).

**(d) `uninstall` occupancy confirmed; every coordinate moved.** `uninstall` is `MAOS_ONE_SHOT_MODES[31]`
(**`verbs.rs:182`**, not `:193`), the cascade is **`main.rs:8883`**/`:8923` (not `:5349`), the mode test is
**`:6207`**, `LifecycleEvent::Uninstall` is **`:6225`**. `run_uninstall_cascade` **deletes no directories at all**
— it forgets principals, revokes tokens and *writes* a signed Merkle proof — so it is unusable as a root
enumeration; only its terminal/exit-code shape (`:8676-8729`, `0/3/4/5`, intents `principal.uninstall.*`, pinned
by **12** tests in `erasure_uninstall_13_5b.rs` (`:311,397,469,513,545,628,646,682,717,740,756,933`)) is reusable. `lifecycle_verb` (`subcommands.rs:31`) **no longer
exists**; 16-1 renamed it `lifecycle_door_verb` (`:160`) and `Uninstall` now routes to `dispatch_uninstall`
(`:186-212`). `purge`, `remove`, `destroy`, `nuke`, `reset` and `wipe` are all **free tokens**.

---

## 5. THE OBLIGATIONS ROUTED HERE — SIX, OF WHICH THE EPIC CARRIES TWO

| # | obligation | source (verbatim anchor) |
|---|---|---|
| 1 | **Home split-brain beyond `control.json`** — purge deletes those dirs and *"must take the same set exclusively"*; V-30 adds the inode re-check and holding the set to exit | `16-1…md:599` §11 row 2, `:1016` V-21, `:1030` V-30; `deferred-work.md:925` |
| 2 | **Provider key from the OS keyring** — and 16-4 amends the `run-maos.md` page 16-2 AC8 wrote | `16-2…md:461` §11 row 3; `16-2…md:35` |
| 3 | **Legacy Art.17 TL rows** (pre-16-2 `hello-spirit` rows with no identity row, whose `shell.turn` payloads carry what an evaluator typed) — *"erasable by 16-4's `maos purge`"* | `16-2…md:462` §11 row 4 — ⚠ **pinned in production source**, `crates/maos-audit/src/lib.rs:1751-1755` |
| 4 | **D-16-2-O private-tier residual** — *"filed to `16-4…` (its `maos purge` owns the private-tier root)"* | `16-2…md:267`, operator ruling; `sprint-status.yaml:243`. ⚠ **No `deferred-work.md` row exists**, so an owner-key grep sweep cannot see it |
| 5 | **`EnvStability` has no user-facing reader** — add the `maos --help` / `run-maos.md` environment surface | `deferred-work.md:888-892` |
| 6 | **`take_context` leaves `halt_context::<id>` in the Private tier** for the process lifetime and on disk under `retention = "persist"` | `deferred-work.md:937` |

Obligations 4 and 6 are satisfied *by construction* once purge owns the private-tier root; 3 likewise. 1 is
D-16-4-H. 2 and 5 are AC6. **16-3 routed nothing here** (`grep -c -- '16-4'` on the 16-3 story = 0).

⚠ At authoring time `sprint-status.yaml:250`'s comment still read *"NEW `maos uninstall`"* — contradicting the
operator-ratified R4 rename — and recorded none of these six, and `epic-20-ship-it-w5.md:83` AC3 carried the
**same stale verb name**, so 20-2/20-3 would have asserted a verb that will never exist. **Both are corrected in
this change** (§12 rows 7, 8, 11).

---

## 6. 🔴 THE KEYRING HALF HAS NO REQUIREMENT, AND THE BINDING PRD DEFERS IT

`prd/functional-requirements.md:3` is binding: *"This section is binding. Any capability not listed here will NOT
exist in MAOS unless explicitly added via amendment."* The words `secret`, `keyring`, `keychain`, `credential`,
`api key` and `at-rest` appear **zero times** in that file. The only NFR that names keyrings —
**NFR-Sec-19** (`prd/non-functional-requirements.md:52`) — **defers** them: *"kernel-core Private/Shared at-rest
encryption, Vault/cloud KMS, and OS keyrings are deferred additive adapters."* **ADR-051** says the same: OS
keyrings are *"deferred additive adapters behind the same port"* (`KeyManagementPort`, reference crate
`maos-secrets`).

So AC5/AC6 do not drift from the *epic* — the epic asks for them — but they do implement an unlisted, deferred
capability. **The resolution is to write the amendment, not to skip it or to ship unlisted** (D-16-4-M).
The non-superseded prose that fixes its shape is `prd/domain-specific-requirements.md:48`: *"Secrets pass through
to OS keyring (Linux secret-service / macOS Keychain / Windows Credential Manager); no kernel storage (Invariant
I9)."* (`prd/product-scope.md:18` names the same thing but is **superseded for phasing** at `:3` and must not be
planned against.)

**The env-contract gate cannot enforce AC6 where the epic puts the reader.** `xtask/src/check_env_contract.rs`
syn-AST-parses `crates/maos-bin/src/**` **only** (`:118-121`) for literal `MAOS_*` reads; an unregistered read
FAILS (`:152`), but **a registered-and-unread name PASSES silently — there is no orphan detection.** Put the
`MAOS_SECRETS_BACKEND` read in `maos-secrets` and the registration is decorative. The tree already proves this
twice: `MAOS_CRL_PATH` (registered `:245`, zero readers) and `MAOS_OPENAI_API_KEY` (read at
`maos-providers/src/openai.rs:36`, **never registered**). The house precedent that works is
`MAOS_KMS_MASTER_KEY`: read at the composition root (`enterprise_identity.rs:499`), typed value passed into
`maos-secrets`, which reads **no env at all**.

Two further facts the epic's AC2 does not carry: **`FIXME(secrets)` is at `main.rs:3531`, not `:3116`, and there
is a second at `crates/maos-providers/src/anthropic.rs:29`**; and **`anthropic_key_usable` (`main.rs:3541`) is a
third, separate `env::var` read** — miss it and a keyring-only operator gets a provider that registers but is
never marked live. The injection seam already exists: `AnthropicProvider::with_api_key` (`:47`) and
`OpenAiProvider::with_api_key` (`openai.rs:51`), with exactly one production `::new` caller each (`main.rs:3544`, `:3555`).

---

## 10. SIZING AND BUDGET (rule 6, ×1.3; interval arithmetic)

| crate | headroom @ HEAD | change | raw | ×1.3 upper |
|---|---|---|---|---|
| `maos-bin` | **0 (will cross)** | NEW `src/purge.rs`: root model + enumeration (extending the lock-set candidates with trees 3 and 4 and the env overrides), the pre-acquire snapshot, classification (remove / keep-for-`--keep-log` / operator-authored / absent), ordered removal, the checkpoint-then-retain TL handling, the report-only residue sweep, the `--yes`/dry-run gate with its **category disclosure counts**, the **receipt writer + its inside-a-deleted-tree refusal**, the leaf-naming report, and the legacy `$HOME/.local/share/maos` leg +290…+470 · `main.rs` two exhaustive `match verb.name` arms + handler + CRL resolver fix +40…+80 · keyring composition-root wiring (backend select, `SecretStore` construction, `anthropic_key_usable` re-derivation, both `FIXME` retirements) +40…+80 · `env_contract.rs` entries +8…+16 | +378…+646 | **+840** |
| `maos-secrets` | +839 | `SecretStore` impls: keyring backend, env backend, encrypted-file over the EXISTING `seal_at_rest`/`open_at_rest` (`lib.rs:60,126`); **no env read** | +150…+300 | **+390** |
| `maos-domain` | **+151 (tight — `kloc.toml:394` books `16-4`/`16-5` jointly)** | `ports/secret_store.rs` (siem_projection style: module doc naming story+ADR+NFR, `thiserror` enum, `/// Class:` on every method, `is_healthy()`) + `ports/mod.rs` + a CRL default resolver | +52…+82 | **+107** |
| `maos-providers` | +748 | `Option<Arc<dyn SecretStore>>` into both `::new`, resolve-then-delegate to `with_api_key`, retire `anthropic.rs:29` | +30…+60 | **+78** |
| `maos-shell` | **+1 — effectively full** | the `init` message line (a REPLACEMENT, net 0) + drop the dead `logs/` leaf (net −1). ⚠ **Also edits two EXISTING assertions** (`tests/init_test.rs:94-95`, `:231`, both asserting `logs/` is a dir) — integration tests are not charged, but §1's "no test churn" claim was wrong and is corrected | **≤ 0** | **0** |
| `maos-audit` | **0** | retire `default_distillate_corpus_root` + `default_isolation_corpus_root` (both **DEAD** — zero production callers) **and their 8 inline `#[cfg(test)]` unit tests** (`lib.rs:3004-3100`, which ARE charged) | **negative** | **≤ 0** |
| `maos-registry` · `maos-skill` | +767 / +100 | **untouched — Policy G is NOT fixed here** (D-16-4-C; a story that deletes must not also relocate). Purge reads their legacy path; neither crate changes | 0 | **0** |
| `maos-kernel-core` · `xtask` · `maos-cli` · `maos-control` · `maos-journey-test` | — | untouched | 0 | 0 |

`maos-bin` and possibly `maos-domain` cross: code first, `cargo fmt --all`, `kloc-check --json`, then the
`kloc.toml` row carries the bare token **`16-4`** followed by a non-digit non-dash character, the measured figure
and the driver, **same commit** (`kloc.toml:49-95`). ⚠ **The aggregate alarm is already FIRING at HEAD**
(164216 vs 158608, `:655`); hardfail is 170884 (`:656`), so there is +6668 of room — state the new aggregate, do
not silence the alarm. **Do not compress the enumeration to fit** — it is the capability.

Dependencies: **one new crate dependency.** `keyring` in `crates/maos-secrets/Cargo.toml` with
`default-features = false` and `linux-native`/`apple-native`/`windows-native` (MIT OR Apache-2.0, inside
`deny.toml:35-51`). Workspace convention is per-crate caret ranges — `[workspace.dependencies]`
(`Cargo.toml:70-71`) holds exactly one entry (`async-trait`), so this does **not** go there. **`Cargo.lock` must
land in the same commit** (`--locked` is used in every build job). `[bans] multiple-versions = "deny"`
(`deny.toml:55`) may need a `skip` entry in the established provenance-commented form (`:60-124`).
⚠ **`cargo deny` is ADVISORY BY ACCIDENT** — it runs at `discipline.yml:121` inside `reproducible-build`, whose
job-level `continue-on-error: true` (`:38`) swallows it. T0 **enumerates secret-service's transitive tree (zbus and below) before any code is written**, runs `cargo deny check` locally and treats a red as
binding; §11 row 5 routes the gate defect.

New file ⇒ `SCANNED_SOURCE_FILES` (`crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:881`) **22 → 23**, with an
`include_str!` entry and the `manifest_scopes == 0` negative (`:983-993`). New verb ⇒ `verb_table_15_3.rs` at
**four** sites, including `assert_eq!(verbs::VERBS.len(), 7)` → **8** (`:135`) and both sorted surface vecs;
run in **both** feature sets by the Blocking `check-exit-commands` job (`discipline.yml:448-449`).

---

## Decisions ruled in this story (rule 7 — one decision per fork)

Rulings marked **(§14)** are recommendations pending operator ratification; every other row is ruled by
measurement recorded in §1–§6.

| # | Fork | Ruling | Rejected, and why |
|---|---|---|---|
| **D-16-4-A** | Sizing | **WHOLE** (frontmatter `split_from`; §14 Q1). | *Split `16-4b` = the keyring half* — once MAOS stores a key in the keyring that key is MAOS state and purge must erase it, so a split ships either a purge that leaves secrets behind or a keyring nothing erases; and both halves edit the same composition root, `env_contract.rs` and the same three `run-maos.md` copies. |
| **D-16-4-B** | The verb token | **`purge`.** Free (no verb, no one-shot mode, no `maosctl` subcommand, no xtask task), already OWED to this key by `check-exit-commands`, and already named in **two production comments** (`operator_door.rs:1788`, `maos-audit/src/lib.rs:1755`). Added as a `VERBS` row + `VerbName` variant, `builds: &[Build::AirGap, Build::Network]`. | *`uninstall`* — operator-ruled: the Spirit erasure cascade's semantics do not move. *Disambiguation by argument arity* — a typo must not change what is erased. *A synonym* — would orphan two production comments and force an epic exit-block edit. |
| **D-16-4-C** | What purge removes | **Four trees, enumerated by ONE typed function**, `purge::maos_roots() -> Vec<MaosRoot>` in the new `crates/maos-bin/src/purge.rs`, built by EXTENDING the lock-set candidate list (`fn acquire_store_lock_set`, `operator_door.rs:1689-1706`) — never by re-deriving a resolver. ⚠ **Every entry carries `written_by: Maos | Operator`, and purge NEVER removes an `Operator` entry** — this is the root model's TYPE, not a filter applied at the end, so the next directory MAOS adds inherits the rule instead of re-deciding it. Trees 1 and 2 from the existing resolvers (every `MAOS_*` override honoured automatically); **tree 3 is `fn default_audit_key_path` (`audit_key.rs:55`) ONLY** — in that tree MAOS wrote exactly one file, `audit-signing.key` (`fn generate_audit_key`, `audit_key.rs:61`, sole caller `maos-cli/src/subcommands.rs:3058`); `spirit-signing.key` is created by the OPERATOR — **our own README instructs `openssl genpkey -algorithm Ed25519 > ~/.config/maos/spirit-signing.key`** (`crates/maos-spirit-cli/README.md:15`) and `fn load_signing_seed` (`maos-spirit-cli/src/signing.rs:33`) only ever reads it — and `operator.toml` is read-only (`fn resolve_from_env_and_disk`, `operator_config.rs:51,178`). Purge removes the `Maos` file, `rmdir`s the directory **only if it is then empty**, and **reports every file it left, by name**. Tree 4 from a NEW `maos-domain` CRL resolver honouring `MAOS_CRL_PATH`. A **fifth (cgroup) tree is withdrawn** (§2). ⚠ **Policy G is NOT fixed here — reversed by the round-table 2026-09-16 (§15 R2/F3).** *A story that deletes things must not also relocate things.* Fixing the three hardcoded resolvers would silently move every existing user's `registry/` and `skills/` in the same commit that ships an irreversible destructive command, and it would NOT remove purge's obligation to know the legacy location — it would ADD a migration on top of it. Instead purge enumerates the XDG-resolved data root **and** the hardcoded `$HOME/.local/share/maos` when they differ (`fn dirs_fallback`, `maos-registry/src/storage.rs:435`, `maos-skill/src/store.rs:236`; `fn cursor_file_path`, `yank.rs:237`, whose `MAOS_REGISTRY_YANK_CURSOR_PATH` can also land outside tree 2). The legacy leg is **permanent and tested, not transitional**, and says so in its own comment. This is also the cheaper test: with `HOME` scratched, both paths land inside the sandbox, where the XDG fix would have forced every purge test to model an `XDG_DATA_HOME`-vs-`HOME` divergence. | *Delete anything MAOS did not write* — we tell operators to generate `spirit-signing.key` themselves; deleting a publisher's identity is unrecoverable and `--keep-log` protects only the recoverable thing (§15 R2/F2). *A late filter instead of a typed field* — the next directory added gets it wrong. *Fix Policy G here (the first ruling)* — couples a silent data relocation to a destructive command. *`remove_dir_all(locked_paths())`* — `store_dir` returns a file's PARENT, so a redirected `MAOS_AUDIT_DB` would delete an unrelated directory. *Reading `config.toml [paths]`* — hardcoded and false (§4b). *Enumerating the two corpus roots* — both DEAD; retired, not enumerated. |
| **D-16-4-D** | The removal instruction | **`maos init` names the verb, not a path**: `maos: to remove all data, run:  maos purge`. The enumeration is never duplicated into maos-shell (which cannot depend on maos-bin), so the message cannot go stale again. AC2's falsifier runs **exactly what `maos init` prints** and asserts no MAOS path survives. | *Print the resolved root list from `run_init`* — maos-shell cannot reach the enumeration, and a second copy is how §1 happened. *Leave the message and document the gap* — it is user-facing false text. |
| **D-16-4-E** | `--keep-log` | **Checkpoint first, then retain ONE consistent database — and retain the memory/principal DB too whenever it is a different file.** Purge opens the resolved log, runs `PRAGMA wal_checkpoint(TRUNCATE)`, and **closes the connection cleanly**, after which SQLite itself removes `-wal`/`-shm`; the retained set is `transparency.sqlite` (+ `.team` and the `teams/<team>/` subtree when present). Keeping a truncated `-wal` on disk is **refused**: `maos_audit`'s replay/export reader guards on `wal_path.exists()` (`crates/maos-audit/src/lib.rs:346-347`, ADR-028 D6), so a retained `-wal` — even a zero-byte one — leaves a log `maos audit query` reads but the export path refuses. `export-seq.json`, `transparency.airgap-stub.log` and any `transparency.siem-snapshot-*` are **removed** (derived/leaked artefacts). ⚠ **Tenant mode**: purge retains **both** `resolved_transparency_log_path()` (the team log) **and** `default_transparency_log_path()` (the memory + principal DB) when they differ, because deleting the latter under a flag named `--keep-log` would silently destroy the shared-tier memory and principal index the operator believes they kept. ⚠ **Disclosure at the decision point, not in the docs (round-table 2026-09-16, §15 R2/F4).** `--keep-log` is named for the recoverable thing but also retains what an evaluator TYPED — legacy `shell.turn` payloads are Art.17 personal data (16-2 §11 row 4) — during an operation the human believes removes the product. So **the dry run (now the default, D-16-4-G) enumerates what `--keep-log` would retain in CATEGORIES with counts QUERIED FROM THE LOG**, e.g. `retains 1204 audit rows, 37 capability tokens, 9 shell turns containing operator-typed text` — real counts, never prose, because prose is not testable. Docs state the same set — shared-tier memory rows, the principal namespace index, capability-token rows — **a documented, operator-chosen exception to FR2**, and the route by which 16-2's legacy Art.17 rows are kept rather than erased. | *Keep only `transparency.sqlite` without checkpointing* — after a one-shot that is a 4 KB empty header while 189 KB of audit sits in the `-wal` (§3b): silent total audit loss. *"Keep the triple"* — a checkpointed-and-closed DB has no `-wal`/`-shm` to keep, and keeping one breaks the export reader. *A `transparency.sqlite*` glob* — misses `transparency.airgap-stub.log` (`with_extension` replaces). *Retain only the resolved path* — deletes the memory/principal DB in tenant mode. *Interpret `--keep-log` as `$MAOS_HOME/logs/`* — that directory has no writer and is deleted. |
| **D-16-4-F** | Default retention | **Delete by default; `--keep-log` is the opt-in. OPERATOR-RULED 2026-09-16** against the stated criterion — *"choose what respects the spec closely, aligns with the other stories, and fulfils the end goals"* — so the derivation is recorded rather than the preference. **Spec:** FR2 is the binding functional requirement and says *"removing all … without leaving orphaned state"*; `user-journeys.md:295` is not in tension with it, because its climax is **`cargo uninstall maos`** — a cargo command that removes the binary and never executes MAOS code, so data necessarily survives it. That line describes a different command, and `:305` explicitly leaves the choice to the operator (*"persists **or is removed per their choice**"*) — which is what the flag is. **Alignment:** the epic's exit line 7 is `maos purge --keep-log`; if retention were the default the flag would be a no-op and `check-exit-commands` would owe a token against a meaningless line. **End goal:** the doctrine at `developer-tool-specific-requirements.md:50` is that every install verb has a documented **inverse** — an inverse that leaves the data is not one. ⚠ `user-journeys.md:295`'s **path** is still wrong independently of this story (the TL is not in `~/.maos/logs/`, and `logs/` has no writer at all) and is corrected under T6. | *Keep by default* — inverts FR2, makes the epic's exit flag a no-op. *Cite `developer-tool-specific-requirements.md:57` as contrary prose* — **withdrawn, validation round 1**: that row is the **Spirit** lifecycle table's `maosctl uninstall <spirit-id>` entry (local index, `--force`, orphaned ComplianceClaims); it says nothing about kernel-removal retention. |
| **D-16-4-G** | Confirmation | **`--yes` is REQUIRED for every destructive run; there is no interactive prompt. (REVERSED, validation round 1.)** Without `--yes`, `maos purge` performs a **dry run**: it enumerates, prints exactly what it would remove and what it would keep, and exits **0** having deleted nothing. With `--yes` it removes. This is safe in both directions that the earlier "TTY prompts, non-TTY proceeds" ruling got wrong: a script or CI leg that runs `maos purge` bare no longer irreversibly deletes a host with no confirmation, and a human at a terminal is not asked to answer a prompt no test harness can drive. ⚠ **The epic's exit line 7 (`maos purge --keep-log`, no `--yes`) therefore becomes a dry run and must gain `--yes`** — the one exit-block COMMAND edit this story makes, declared in §12 row 10; `check-exit-commands` resolves the `purge` token either way (it matches the verb, not the flags). Report and dry-run output to **stdout**, refusals to **stderr**, matching the unknown-verb split measured at HEAD. | *TTY prompts / non-TTY proceeds (the first ruling)* — a bare `maos purge` in any script deletes everything unconfirmed, and it makes the epic's own exit line interactive for the human it was written for. *Always non-interactive* — an irreversible whole-host deletion with no undo. *A prompt at all* — requires a pty in every test leg for no safety the flag does not already give. |
| **D-16-4-H** | Exclusion, and the ordering trap | **Purge acquires `acquire_store_lock_set(resolved_transparency_log_path(), OfflineExclusive)` ITSELF, first, and holds it to exit.** It must: it is a verb, and the boot lock set is acquired at `main.rs:1957-1966` — **after** the verb dispatch at `:1834-1914`, which every returning arm exits before. Contention ⇒ **exit 69 `StoreInUse`** / `OfflineOperationInProgress`, unavailable ⇒ **78**, reusing 16-1's typed errors and `main.rs:1982-2004`'s exact messages. ⚠ **The acquire MUTATES THE FILESYSTEM BEFORE IT CAN REFUSE**: it creates every missing candidate at 0700 (`operator_door.rs:1714-1744`) and only then takes the flock that produces `StoreInUse` (`:1776-1800`). So a purge that exits 69 has already created roots that did not exist — which would both falsify AC1's "found absent" report on the retry and make a naive "nothing was deleted" check pass *because of* a mutation. **Purge therefore snapshots the root set BEFORE acquiring, and AC4 asserts the post-refusal set is byte-identical to that snapshot, not merely "still present".** `StoreLockSet` exposes only `locked_paths()`, so purge cannot ask what the set created: it diffs its own pre-snapshot instead — **no `operator_door.rs` change, which is why §10 budgets none.** Purge deletes directories it holds open handles on — legal on Linux; `Drop` unlocks at exit. **Unix only**: `acquire_store_lock_set` returns `LockUnavailable` on non-unix (`:1660-1667`), so `maos purge` on Windows refuses **78** with a stated reason. | *A door route* — purge deletes the daemon's own home; there is nothing to serve it. *Acquire, then enumerate from `locked_paths()`* — the set has by then created roots that never existed, and `maos_home()` is added only `if home.exists()` (`:1697`). *Ask the lock set what it created* — no such API; adding one is an unbudgeted 16-1 change. *A `.lock` file* — forbidden: `operator_door.rs:1581-1584` (it would change the file counts `erasure_uninstall_13_5b` asserts). |
| **D-16-4-I** | FR2's "sandbox mounts" and "ACP sockets" | **Sockets: stated as non-existent** (ACP is NDJSON over stdio, `maos-acp/src/lib.rs:3`; zero `UnixListener` in the tree). **Mounts: T3 containers and cgroup subtrees — BOTH report-only.** T3 runs `--rm` with `--tmpfs=/tmp` and a read-only `--volume` (`crates/maos-kernel-core/src/security/sandbox/t3/argv.rs:69,74,78`) plus an explicit `stop`/`rm -f` teardown (`t3/child.rs:87-94`), so the only residue is a **leaked container** after a crash. The cgroup subtree is **not created at HEAD at all** (zero production callers; 17-2's charter) and could not be removed anyway (`rmdir`-only, write-on-a-root-owned-parent, `EBUSY` while populated — §2). Purge therefore **reports** both: leaked `maos-*` containers with the exact `podman`/`docker rm` line, and any `maos` cgroup subtree it can see, removing neither. Both live under a single "residue MAOS cannot remove for you" heading, so FR2's PARTIAL is visible to the operator rather than implied. | *Silently claim FR2 whole* — `requirements-inventory.md:541` already records it PARTIAL; claiming more is the "claim standing in for a control" shape Epic 12's retro named. *Run `podman rm` from purge* — purge would depend on a container runtime's presence and exit code. *`remove_dir_all` the cgroup subtree* — `EPERM`: kernel-generated files inside a cgroup v2 directory cannot be unlinked. |
| **D-16-4-J** | Where `MAOS_SECRETS_BACKEND` is read | **At the composition root** (`crates/maos-bin/src/main.rs`, beside the retired `FIXME(secrets)`), passed into `maos-secrets` as a typed `Backend` enum — the `MAOS_KMS_MASTER_KEY` precedent (`enterprise_identity.rs:499`). `maos-secrets` keeps reading **no env at all**. Registered in `env_contract.rs` **before the `];` at `:484`** (append-by-story, not alphabetical), `stability: UserFacing`. | *Read it in `maos-secrets`* — `check-env-contract` scans maos-bin `src` only, so the registration would be **decorative and unverified**, exactly as `MAOS_CRL_PATH` already is. |
| **D-16-4-K** | Backend set, default, and the Linux backend | **`keyring` (default) → env (fallback, journaled) → `encrypted-file`; the Linux backend is `sync-secret-service`, NOT `linux-native`. (CORRECTED, validation round 1.)** `linux-native` is the kernel **keyutils** backend: session-scoped, not persisted across reboot, and therefore a store an operator's key would silently vanish from — while `domain-specific-requirements.md:48`, architecture §4.3.2 and the PRD amendment all name **secret-service** as the Linux backend. The D-BUS absence that motivated `linux-native` is a CI property, and CI is already handled by pinning `MAOS_SECRETS_BACKEND=env` there, so it must not choose the production backend. **`keyring` (default) → env (fallback, journaled) → `encrypted-file`.** ⚠ **What this does and does NOT buy (Vex, round-table 2026-09-16, §15 R3) — stated in the docs, because claiming the rest is the "claim standing in for a control" failure mode:** secret-service is **session-scoped, not process-scoped** — any process at the operator's uid reads the collection over D-Bus — and `README.md:282-284` already records that until **17-1** every Worker runs at the operator uid and can read `control.json`. So the keyring is **an at-rest improvement** (the key leaves `/proc/<pid>/environ`, shell history, CI logs and `ps`) and **changes nothing about same-uid runtime exposure**; 17-1's `env_clear` + T3 buys that. The page says exactly two sentences: safer on disk and in the environment; not yet safe from another process running as you on this host. Further: **keyring entries are namespaced by the MAOS home** — unnamespaced, one purge in a scratch `HOME` deletes the developer's real key; a **locked collection blocks on an unlock prompt**, so the attempt is bounded and falls back to env with the downgrade journaled, and **that journal row carries no key material**. The encrypted-file vault is the architecture's named fallback (§4.3.2) and is built on `maos-secrets`'s EXISTING `seal_at_rest`/`open_at_rest` (`lib.rs:60,126`) under an `encrypted-file` cargo feature, as the architecture spells it. **CI sets `MAOS_SECRETS_BACKEND=env` explicitly** — hosted runners have no Secret Service, no session D-Bus and, on `windows-check` (`discipline.yml:2892`), no credential store; a keyring test that "skips when unavailable" would be the vacuous control 13.6e named. The real keyring backend is proven by unit vectors over the port plus an operator-lane leg. | *`MAOS_SECRETS_BACKEND=file` as a CI plaintext backend* — the architecture names an ENCRYPTED file vault; a plaintext one invents a new secret store. *Keyring-in-CI* — unavailable on all three runner OSes; the test would certify nothing. |
| **D-16-4-L** | Where the key is materialized | **At provider construction**, resolved by the store and handed to the existing `with_api_key` seam (`anthropic.rs:47`, `openai.rs:50`) — zero churn for the 9 existing `with_api_key` call sites, and `credential_fingerprint()` (`provider.rs:28`) stays stable, which NFR-Scale-4's per-credential rate-limit buckets require. A keyring miss AND an env miss collapse into the existing `ProviderError::Unconfigured` (`provider.rs:47`). `anthropic_key_usable` (`main.rs:3541`) is re-derived from the **resolved** key. **ADR-005 / §4.3.2's just-in-time materialization stays open and is stated** (§11 row 2). | *JIT at `provider.complete`* — `complete` is sync and per-call; a keyring round-trip per request, and `credential_fingerprint` would have to either cache (defeating JIT) or destabilise the rate-limit buckets. *A new `ProviderError` variant* — the miss mode already exists. |
| **D-16-4-M** | The unlisted capability (§6) | **Write the amendment — AUTHORED, NOT ENACTED (round-table 2026-09-16, §15 R2/F5).** The operator ruled the amendment is written in-story; the round-table bounded HOW: it lands as **its own commit, reviewable on its own**, and **AC6 asserts the amendment exists and is cited — never that a code commit redefined a binding requirement**. Paige owns the wording so it matches the voice of the FRs around it rather than reading as a patch forever. ⚠ **Scope it to what ships (Vex, §15 R3):** the line the amendment leans on, `domain-specific-requirements.md:48`, has THREE clauses — *just-in-time at the capability boundary*, *in-memory lifetime bounded*, *never logged* — and this story delivers only the last. `api_key: String` (`anthropic.rs:19`), **no `zeroize` or `secrecy` dependency anywhere in the workspace**, held for the daemon's lifetime, and D-16-4-L declines JIT. The amendment therefore covers **the key SOURCE (OS keyring, three backends) only**, and says in its own text that JIT materialization and bounded in-memory lifetime remain open (§11 row 2). **Write the amendment.** This story adds an FR row for OS-keyring secret materialization to `prd/functional-requirements.md` and revises **NFR-Sec-19** to move OS keyrings from *deferred* to *v0.1 delivered, Vault/cloud KMS still deferred*, citing `domain-specific-requirements.md:48` for the backend triple and ADR-051 for the port. The PRD's own preamble (`:3`) requires this; shipping the adapter without it makes the binding section false. | *Ship unlisted* — violates `functional-requirements.md:3`. *Drop AC5/AC6* — drifts from the epic, which the operator's standing directive forbids. *Claim NFR-Sec-19 closed* — it also covers at-rest encryption and KMS, which this story does not deliver. |
| **D-16-4-N** | ADR | **NEW `ADR-067` — "MAOS data-root layout and what `maos purge` removes."** Next free number (highest in use is ADR-066); the 17 planning-reserved gaps (003, 005, 007-009, 015-017, 019-021, 025, 029, 033-036) are **not** reusable. No ADR owns `MAOS_HOME`/filesystem layout today — ADR-062 owns only `control.json` as the discovery contract. Format matches ADR-060..066 (six frontmatter keys, `Status` beginning `ACCEPTED`, the five `##` sections, ≥40 lines, no `todo`/`tbd`/`<placeholder>`), and the strict file↔`docs/adr/index.md` bijection (`xtask/tests/decision_adrs_and_provisioning.rs:567-637`) is updated in the same commit. **No second ADR for the keyring** — ADR-051 already names OS keyrings as the deferred additive adapter behind `KeyManagementPort`, so activating it is that ADR's documented extension path. | *Extend ADR-062* — it is ACCEPTED and scoped to the operator surface. *A keyring ADR* — duplicates ADR-051. *Reuse a gap number* — `15-full-spectrum-v2-2.md:5` records a prior +2 renumbering caused by exactly that. |
| **D-16-4-Q** | The uninstall receipt (NFR-Aud-12) | **The SEAM ships now; the signature does not.** NFR-Aud-12 (`non-functional-requirements.md:72`) says *substrate-uninstall* produces a receipt *"retained independent of the substrate"* — and 16-4 builds the only substrate-uninstall path this product will have, so routing it away is *a promise about someone else's behaviour* (the disqualifier this room adopted after 15-5). Purge writes **one JSON record** to `--receipt <path>`, defaulting beside the operator's CWD and **never under any MAOS root**: schema version, timestamp, binary version, and per enumerated root its resolved path, `written_by`, and disposition (`removed` / `kept` / `absent` / `left (operator-authored)`) with a count — plus explicit `"signature": null` and `"proof": null` so v1.0 fills them without a schema break. **Typed refusal, before anything is removed, if the receipt path resolves inside a tree being deleted** — a receipt destroyed by the operation it records is worse than none. ⚠ **Security shape (Vex, §15 R3):** (a) that refusal is a PATH comparison, and a path comparison is defeated by a symlink (`--receipt /tmp/x/r.json` where `/tmp/x` → `$MAOS_HOME`), so **canonicalise both sides AND re-verify by inode after open — reuse 16-1's `fstat(handle)` vs `stat(path)` pattern (`operator_door.rs:1788-1797`), whose comment already names `maos purge`**; (b) the file is created `O_NOFOLLOW | O_CREAT | O_EXCL` at **mode 0600** and **never overwrites an existing file** — purge writes as the operator, so a pre-planted symlink at the receipt path is an arbitrary-write primitive; (c) the CWD default can itself be inside a MAOS root (`cd ~/.maos && maos purge`) and must **trip the refusal, never silently relocate**; (d) the record is a host-layout recon document (every root path, tenant team names, spirit ids, counts) and **carries no `control.json` bearer token and no key material**; (e) **two-phase write** — the plan first with `"status": "in-progress"`, then finalised in place with actual dispositions, so a purge that dies mid-way leaves *evidence of a partial purge* rather than nothing, which is the gap NFR-Aud-12 exists to close. ⚠ **Security shape (Vex, round-table 2026-09-16, §15 R3):** (a) that refusal is a PATH comparison and a path comparison is defeated by a symlink (`--receipt /tmp/x/r.json` where `/tmp/x` → `$MAOS_HOME`), so **canonicalise both sides AND re-verify by inode after open — reuse 16-1's `fstat(handle)` vs `stat(path)` pattern (`operator_door.rs:1788-1797`), do not reinvent it**; (b) the file is created `O_NOFOLLOW | O_CREAT | O_EXCL` at **mode 0600** and **never overwrites an existing file** — purge writes as the operator, so a pre-planted symlink at the receipt path is an arbitrary-write primitive; (c) the CWD default can itself be inside a MAOS root (`cd ~/.maos && maos purge`) and must **trip the refusal, never silently relocate**; (d) the record is a host-layout recon document (every root path, tenant team names, spirit ids, counts) and **carries no `control.json` bearer token and no key material**; (e) **two-phase write** — the plan is written first with `"status": "in-progress"`, then finalised in place with actual dispositions, so a purge that dies mid-way leaves *evidence of a partial purge* rather than nothing, which is exactly the gap NFR-Aud-12 exists to close. Its last line names the binary path and `cargo uninstall maos`, because purge cannot remove a binary cargo owns and FR2 says *"uninstall MAOS kernel cleanly"*. | *Cut it to §11 and route to 20-3a (the first ruling)* — no other story's charter is substrate-uninstall; the retrofit would land in the one command least able to host it. *Ship the Merkle proof now* — v1.0 scope; the nulls are the forward-compatible boundary. *Write the receipt under `MAOS_HOME`* — deleted by the operation it records. |
| **D-16-4-P** | §0's HEAD red | **Fixed in this story — OPERATOR-RULED 2026-09-16 (§14 Q6).** `RATIFIED_AT_EASING` 18935 → 18938 in a commit of its own, ahead of any 16-4 code, whose message attributes the raise to Story 16-3's operator authorization and states that no ceiling moves. T0 captures the failing transcript first, so the ledger shows the red existed and who caused it. | *Reopen 16-3 (the recommendation)* — overruled. *Absorb it silently into a 16-4 commit* — makes an un-attributable red vanish inside an unrelated diff; the commit message is what keeps this honest. *Leave it to the retrospective* — HEAD stays red and blocks every remaining Epic-16 story. |
| **D-16-4-O** | Purge and the keyring | **Purge deletes MAOS's own keyring entries BY KNOWN NAME, never by enumeration.** The `SecretStore` port gets `delete(key)` only — **no `list`** — because no backend's enumeration support is established and `keyring`'s API surface for it is backend-dependent; T0 does not get to discover this after the AC is written. MAOS writes a **closed, known set** of entries (one per provider credential, under MAOS's own service name), so purge deletes exactly that set, reports each as `removed` / `absent` / `backend-unavailable`, and **never touches an entry it did not write**. This is the coupling that keeps D-16-4-A WHOLE. | *Enumerate with a `list`/`delete` pair* — asserted with no evidence any backend supports listing; an AC that depends on an unverified API is the shape this story exists to stop. *Leave keys behind* — FR2's *"without leaving orphaned state"*, and the keys are the most sensitive state MAOS holds. *Delete the whole keyring collection* — MAOS must never remove entries it did not create. |

---

## Acceptance Criteria (6)

1. **AC1 (command — the verb and the enumeration).** From a scratch `HOME`/`MAOS_HOME`/`XDG_DATA_HOME`/
   `XDG_CONFIG_HOME`/`MAOS_CRL_PATH`, after `maos init` and one `maos run` turn, **`maos purge --yes`** exits 0
   and its stdout names every root it removed and every root it found absent. Afterwards **every root entry marked `written_by: Maos` is
   gone and every `Operator` entry is UNTOUCHED**, asserted by re-resolving each root through its own production
   resolver (never by re-listing a hardcoded path); a directory is `rmdir`ed only if it is then empty, and every file
   left behind is named in the report. In tree 3 that means `audit-signing.key` is removed while
   **`spirit-signing.key` and `operator.toml` still exist** — MAOS never wrote them. `maos init` afterwards exits 0 on
   a clean home. Purge also enumerates the **hardcoded** `$HOME/.local/share/maos` alongside the XDG-resolved data
   root when they differ (registry, skills, yank cursor), and a **receipt** is written per D-16-4-Q. **The test first asserts
   each tree was NON-EMPTY before the purge** — an existence assertion over an already-absent tree proves
   nothing. Without `--yes` the same invocation is a dry run: it prints the same enumeration and **removes
   nothing**, asserted by a byte-identical root set (D-16-4-G). The verb is a `crates/maos-bin/src/verbs.rs`
   `VERBS` row plus a `VerbName` variant dispatched by BOTH `main()`s (`:1492` air-gap, `:1834` network); it never
   falls through to daemon boot. Leaked T3 containers and any `maos` cgroup subtree are **reported, not removed**,
   under a "residue MAOS cannot remove for you" heading (D-16-4-I).
   **Proven red twice:** (1) drop `written_by` and treat tree 3 as a directory ⇒ `spirit-signing.key` is deleted and
   AC1 reds on the untouched-Operator-entry assertion — the defect this AC exists to prevent, recorded and reverted;
   (2) remove `audit-signing.key` from the enumeration ⇒ a MAOS-written key survives and AC1 reds.
2. **AC2 (the instruction MAOS prints is true).** With `MAOS_HOME` **unset** (the default install),
   `maos init --plain` prints the line `maos: to remove all data, run:  maos purge --yes`; the test **captures
   that line and executes exactly the command it names**, and asserts no MAOS path survives in `$HOME`.
   *Mechanics, pinned so this is runnable:* `--plain` is required because `print_line`
   (`crates/maos-shell/src/lib.rs:773-776`) wraps output in `\x1b[1m…\x1b[0m` under `ColorChoice::Always`; the
   test strips the exact prefix `maos: to remove all data, run:  ` (**two** spaces), splits the remainder on
   whitespace, asserts `argv[0] == "maos"`, and **substitutes `env!("CARGO_BIN_EXE_maos")` for it** — `maos` is
   not on `PATH` in a test. It never shells out to a shell, so there is no quoting or injection path.
   **Proven red at HEAD, recorded before the repair:** the same test run against HEAD captures
   `rm -rf $HOME/.maos`, executes it, and the Transparency Log, journal, memory, erasure-proofs and
   spirit-archives all survive under `$HOME/.local/share/maos`. The two dead resolvers
   (`default_distillate_corpus_root`, `default_isolation_corpus_root`) are deleted **with their 8 inline
   `#[cfg(test)]` unit tests** (`crates/maos-audit/src/lib.rs:3004-3100`), and `$MAOS_HOME/logs/` — a directory
   with no writer — is no longer created, which **requires editing two existing assertions**
   (`crates/maos-shell/tests/init_test.rs:94-95` and `:231`, both of which assert `logs/` is a dir).
3. **AC3 (`--keep-log` keeps a usable log).** `maos purge --keep-log --yes` after a **`MAOS_ONE_SHOT` exit**
   (the WAL-heavy path, not the clean SIGTERM path) leaves a Transparency Log that still reads back, and removes
   everything else. The assertion is on **rows read back**, never on file presence:
   `maos audit query --format ndjson` is run **before and after**, counted by line, and the test asserts
   **`before > 0` AND `after == before`** — equality alone is satisfied by `0 == 0`, which is the exact failure
   mode being guarded. A **second reader** is asserted too: the ADR-028 export/replay path
   (`crates/maos-audit/src/lib.rs:346-347`, which refuses when a `-wal` exists) must also succeed, because
   `maos_audit::query` (`:186`) has no WAL guard and would go green over a log the export path rejects.
   **Proven red twice:** (1) retain only `transparency.sqlite` without checkpointing ⇒ the query returns zero
   rows against a 4 KB header; (2) retain a truncated `-wal` alongside it ⇒ `query` passes but the export path
   refuses — both recorded, reverted. In tenant mode the memory/principal database is retained as well
   (D-16-4-E), asserted by a private-tier read-back. **The dry run's disclosure is asserted, not assumed**: run
   `maos purge --keep-log` with no `--yes` against a log seeded with a known number of audit rows, capability tokens
   and operator-typed `shell.turn` rows, and assert the printed categories carry **those exact counts**. **Proven
   red:** replace a count with prose ⇒ the assertion reds, which is the whole reason the counts are queried from the
   log rather than described. The docs state what `--keep-log` necessarily also retains
   (shared-tier memory rows, the principal namespace index, capability-token rows) and that this is a documented
   exception to FR2.
4. **AC4 (refusal is typed, and the filesystem is untouched).** With a live `maos run` root holding the store
   set, `maos purge --yes` exits **69** naming the held directory, and the root set afterwards is
   **byte-identical to a snapshot taken before the invocation** — path set, and for each path its `(st_dev,
   st_ino)` and mode. *"Every root still present"* is NOT the assertion and would be satisfied by the defect:
   `acquire_store_lock_set` creates missing roots at 0700 (`operator_door.rs:1714-1744`) **before** the flock
   that refuses (`:1776-1800`), so a purge that exits 69 can leave the tree more populated than it found it.
   With the lock unavailable it exits **78**; on a non-unix build it exits 78 with a stated reason. **Proven red
   twice:** (1) acquire the set *before* snapshotting ⇒ the refusal creates roots and the byte-identity assertion
   reds (with the weaker "still present" assertion it would have passed) — recorded, reverted; (2) run the
   refusal path against a home with one root absent ⇒ that root exists afterwards unless the snapshot is taken
   first.
5. **AC5 (the keyring is the default key source).** A `SecretStore` port in `maos-domain` (siem_projection style,
   `/// Class:` on every method) is implemented by a keyring backend, an env backend and an
   `encrypted-file` backend over the existing `seal_at_rest`/`open_at_rest`; `MAOS_SECRETS_BACKEND` is read **at
   the composition root** and the typed backend passed in. Both providers resolve keyring-first with a journaled
   env fallback through the existing `with_api_key` seam; `anthropic_key_usable` is re-derived from the resolved
   key; **both** `FIXME(secrets)` comments (`main.rs:3531`, `anthropic.rs:29`) are retired. `maos purge` deletes
   MAOS's own keyring entries **by known name** (D-16-4-O — the port has `delete`, no `list`) and reports each as
   `removed` / `absent` / `backend-unavailable`. `Cargo.lock` and any `deny.toml` `skip` land in the same
   commit and `cargo deny check` is run locally and recorded (the CI job is advisory by accident).
6. **AC6 (the contract is real, and the docs are true).** `MAOS_SECRETS_BACKEND` is registered in
   `env_contract.rs` **and its reader is inside `maos-bin/src`**, so `check-env-contract` actually binds it;
   `MAOS_CRL_PATH` stops being a dead registration — the CRL directory resolves through it and defaults into the
   XDG data tree, retiring the hardcoded `/tmp/maos/crl` and making `revocation.rs:427`'s doc comment true. The
   `### Provider keys (today)` paragraph is amended in **all three** copies
   (`docs-site/docs/run-maos.md:73-77`, `docs/maos.dev/run-maos.md:73-77` — byte-identical twins — and the
   translated `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md`, heading `:72`, body
   `:74-76`), a `maos purge`
   section is added to the same pages with the `--keep-log` exception stated, `ADR-067` (D-16-4-N) lands with the `docs/adr/index.md` row, and the
   **PRD amendment (D-16-4-M) lands as its own reviewable commit** — AC6 asserts **that the amendment exists and that
   this story cites it**, never that a code commit redefined a binding requirement. **Proven red:** move the `MAOS_SECRETS_BACKEND`
   read into `maos-secrets` ⇒ `check-env-contract` still PASSES with the name registered — recorded as the
   measurement that justifies D-16-4-J, then reverted.

---

## Review obligations (§A6)

**(a)** Run AC2's captured-line test at HEAD before the repair; record the surviving paths. **(b)** Delete tree 3
from `maos_roots()` ⇒ AC1 reds on the signing key. **(c)** Confirm the cgroup sweep is **report-only and
conditioned on 17-2**: grep `apply_resource_ceiling` for production callers (expect zero at HEAD) and read the
sweep for any removal call — a sweep that can only red against state the test planted itself is the E15-A6
failure obligation (q) forbids, which is why this is a read, not a falsifier. **(d)** Run AC3 against an EMPTY
log ⇒ it must red on `before > 0`, not pass on `0 == 0`; then retain a truncated `-wal` ⇒ the export leg reds
while `query` passes. **(e)** Acquire the lock set before snapshotting ⇒ AC4's byte-identity assertion reds and
the weaker "still present" form would have passed; recorded. **(f)** Start a root, run `maos purge --yes`, assert
**69** AND `(st_dev, st_ino)`-level identity with the pre-invocation snapshot — a refusal that *created* anything
is as much a failure as one that deleted. **(g)** Move the `MAOS_SECRETS_BACKEND` read to
`maos-secrets` ⇒ `check-env-contract` passes anyway; recorded (AC6). **(h)** `check-kernel-baseline` reports
`changed == 0, added == 0, removed == 0` over 98 files at 24477. **(i)** `git diff Cargo.lock` non-empty exactly
once, for `keyring`; `cargo deny check` run locally and its output pasted. **(j)** `verb_table_15_3.rs` green in
**both** feature sets; `VERBS.len()` is 8. **(k)** `SCANNED_SOURCE_FILES` is 23 and the directory listing agrees.
**(l)** `check-exit-commands` resolves `purge` from the real `maos --help` (build the binary first) and the owed
count drops 5 → 4. **(m)** Read `purge.rs`: no resolver is re-implemented; every root comes from a production
resolver or the lock-set candidate list. **(n)** Confirm no `.lock` file is created inside any store directory
and `erasure_uninstall_13_5b` file-count assertions stay green. **(o)** Confirm purge has **no** door route and
no `maosctl` subcommand. **(p)** Diff `docs-site/docs/run-maos.md` against `docs/maos.dev/run-maos.md` — they must stay
**byte-identical**, and no gate enforces it; the `ko` copy is a translation, so check it for *content* parity and
`gate:glossary-lock` term counts, never bytes. **(q)** Every AC assertion reads state the production code wrote, not state the test set (E15-A6).
**(r)** Enumerate purge's failure interleavings — a root deleted while another is locked, a root that is a
symlink, a root that is a non-directory (`erasure_uninstall_13_5b:761` plants one), `HOME` unset (Policy G falls
back to `/tmp`), and a root outside `$HOME` via an env override — and state which are refused, which are skipped
with a named reason, and which are removed. **(u)** Point `--receipt` at a path inside a tree being deleted ⇒ purge must refuse, typed, **before removing
anything**; and confirm the default receipt path resolves under no MAOS root. **(v)** Delete `written_by` from the
root model ⇒ AC1's tree-3 vector reds on `spirit-signing.key`; confirm the field is the enumeration's type and not a
filter at the removal site. **(w)** Delete the legacy `$HOME/.local/share/maos` leg ⇒ a purge on a home whose registry
lives there leaves it behind; confirm the leg carries a comment saying it is permanent, not transitional.
**(x)** Replace one dry-run disclosure count with prose ⇒ AC3's disclosure vector reds. **(y)** Symlink the receipt path's parent into `$MAOS_HOME` ⇒ purge must refuse on the INODE check, not pass the path check; and pre-plant a symlink AT the receipt path ⇒ `O_NOFOLLOW|O_EXCL` must refuse rather than write through it. **(z)** `cd` into `$MAOS_HOME` and run purge with the default receipt path ⇒ typed refusal, no silent relocation. **(aa)** Kill purge between the `in-progress` write and the finalise ⇒ an `in-progress` receipt survives and names the partial state. **(ab)** `grep` the receipt for the `control.json` bearer token and any `sk-` material ⇒ zero hits; mode is 0600. **(ac)** Run purge against a scratch `HOME` while a keyring entry for a DIFFERENT home exists ⇒ the other entry survives (namespacing). **(ad)** Lock the secret-service collection ⇒ the daemon does not block: bounded attempt, env fallback, downgrade journaled, and the journal row contains no key. **(s)** Confirm no purge test can reach the machine-global
`/tmp/maos/crl`: every test sets `MAOS_CRL_PATH` into its own scratch tree, and AC6's wiring lands before T1's
enumeration. **(t)** Confirm the `RATIFIED_AT_EASING` fix (D-16-4-P) is its own commit, ahead of any 16-4 code,
whose message attributes the raise to Story 16-3.

---

## 11. Declared cut lines (rule 10 — each names an owner, never a bucket)

| # | Item | Measured evidence | Destination |
|---|---|---|---|
| 1 | **The uninstall receipt's SIGNATURE** (NFR-Aud-12's signed Merkle inclusion/exclusion proof, v1.0) — the record, its path and its schema ship here | `non-functional-requirements.md:72`; the cascade's signed-proof shape `main.rs:8676-8729` is the v1.0 precedent | **NOT CUT AT THE SEAM — D-16-4-Q ships the record, the path and the `signature`/`proof` nulls (round-table 2026-09-16, §15 R2/F1).** Only the signature is routed, to **`20-3a`/`20-3b`** if their charter covers NFR-Aud-12 at v1.0, else **`epic-16-retrospective`**. Note in §12. |
| 10 | **Three resolvers ignore `XDG_DATA_HOME`** (`fn dirs_fallback` `maos-registry/src/storage.rs:435`, `maos-skill/src/store.rs:236`; `fn cursor_file_path` `yank.rs:237`) | measured; purge reads the legacy path instead | **`epic-16-retrospective`** — a real defect, but fixing it relocates live user data, and rule 10 forbids coupling that to the story holding the knife (§15 R2/F3). Retro row (§12), with the evidence. |
| 2 | **Just-in-time credential materialization** — ADR-005, architecture §4.3.2, **and `prd/domain-specific-requirements.md:48`** (*"The kernel materializes secrets just-in-time at the capability boundary; in-memory lifetime bounded"*) ⚠ the same line D-16-4-M cites for the amendment, so the amendment must not claim the half this story does not deliver | `provider.rs:19` `complete` is sync; `credential_fingerprint` `:28` hashes the key; NFR-Scale-4 buckets depend on it; `api_key: String` is held for the daemon's lifetime (`anthropic.rs:19`, `openai.rs:20`) with **no `zeroize`/`secrecy` dependency anywhere in the workspace**, so `domain-specific-requirements.md:48`'s *"in-memory lifetime bounded"* is unimplemented alongside its JIT clause | **`epic-16-retrospective`** — no open story's charter covers provider-call-time materialization or bounded in-memory lifetime. Retro row (§12). |
| 3 | **`run_uninstall_cascade`'s shared-tier-only emptiness proof** (D-16-2-O's count half) | `16-2…md:267`; purge owns the private-tier ROOT, not the cascade's proof | **`16-5`** — D3's audit-truth charter (a proof that says "provably nothing to erase" over data that exists). Note in 16-5 AC1 (§12); `deferred-work.md` row at T7, since none exists. |
| 4 | **`RATIFIED_AT_EASING` is stale — HEAD is CI-red** | `xtask/tests/recovery_lane_ceiling_rule.rs:268` = 18935 vs `kloc.toml:247` = 18938; panic at `:274`; measured FAIL (§0) | **NOT CUT — FIXED HERE, operator-ruled 2026-09-16 (§14 Q6, D-16-4-P).** Own commit, ahead of any 16-4 code, message attributing the raise to Story 16-3. |
| 9 | **The cgroup subtree is unreachable at HEAD and its writer is scheduled for DELETION** | `apply_resource_ceiling` has **zero production callers** (`resource_ceiling.rs:103-107`; only `tests/cgroup_ceiling_smoke.rs`) — and `epic-17-…md:67` (17-2 AC2) already rules the `/sys/fs/cgroup/maos/spirit-<pid>` root layout and `apply_resource_ceiling` **"DELETED, not wired"**, keeping only `linux.rs`'s own-cgroup `maos.slice/spirit-<id>` (`:374`). Cgroup v2 dirs are `rmdir`-only, need write on a root-owned parent, and return `EBUSY` while populated | **NOT A TREE — withdrawn entirely.** It cannot be created by any HEAD build and its writer is deleted by **`17-2`**, which owns the surviving own-cgroup layout. Purge mentions it **only** if the directory happens to exist (pre-17-2 residue on a developer box), under the report-only residue heading, and removes nothing. Note in 17-2 AC2 (§12). |
| 5 | **`cargo deny` is advisory by accident**, and **clippy is never invoked** | `discipline.yml:38` job-level `continue-on-error` swallows `:121`; `components: clippy` at `:44` with no job running it | **`20-3b-gate-retirement-and-coverage-generator`** — charter: gate dispositions. `deferred-work.md` rows at T7. |
| 6 | **Policy F ignores `XDG_CONFIG_HOME`** (`operator_config.rs:51,178`, `subcommands.rs:4872`, `maos-spirit-cli/src/signing.rs:47`) — purge honours E, so an operator with `XDG_CONFIG_HOME` set has a config root purge finds and those readers do not | measured; four resolvers | **`epic-16-retrospective`** — no story's charter covers operator-config resolution. Retro row (§12). |
| 7 | **`migration_plan::optional_plan_path` has no `$HOME/.maos` leg** — plans are silently not persisted when `MAOS_HOME` is unset | `maos-bin/src/migration_plan.rs:183-192` | **`epic-16-retrospective`**. `deferred-work.md` row at T7. |
| 8 | **`transparency.siem-snapshot-*` leaks on a crash** (best-effort cleanup only) | `enterprise_identity.rs:417-421,453` | **Not cut — purge removes them**, and the leak itself is a retro row (§12). |

**Not cut (EFFORT, not SCOPE):** the four-tree enumeration (§2), the WAL-aware `--keep-log` (§3), the Policy-G
fix at origin, the CRL resolver, the PRD amendment and ADR-067, purge's keyring erasure. Each is required for one
of this story's own ACs to be true.

---

## Dev notes

**Reuse, do not reinvent.**
- **The root list:** `maos_bin::operator_door::acquire_store_lock_set` candidates (`operator_door.rs:1689-1706`),
  `maos_domain::operator_door::maos_home` (`:63`), `maos_audit::default_{transparency_log_path,journal_path,
  memory_root,archive_dir,erasure_proofs_dir}`, `maos_domain::audit_key::dirs_config_home` (`audit_key.rs:108`),
  `maos_bin::resolved_transparency_log_path` (`main.rs:111`).
- **Exclusion and its messages:** `StoreLockRole::OfflineExclusive`, `StoreLockError::{StoreInUse,
  OfflineOperationInProgress, LockUnavailable}`, and the exact stderr/exit shapes at `main.rs:1982-2004`.
- **Removal-with-report precedent:** `xtask/src/demo_j1.rs` `EphemeralHome` (`:363-375`) and its leaf-only report
  (`:321-327`, *"AC5 forbids absolute workstation paths in scene output"*).
- **Key injection:** `AnthropicProvider::with_api_key` (`anthropic.rs:48`), `OpenAiProvider::with_api_key`
  (`openai.rs:50`), `ProviderError::Unconfigured` (`provider.rs:47`).
- **Sealed-file vault:** `maos_secrets::{seal_at_rest, seal_at_rest_opt, open_at_rest}` (`lib.rs:60,101,126`) —
  the `Option<&dyn Port>` idiom at `:101` is the house pattern for an opt-in backend with a byte-identical default.
- **Env-at-root:** `MAOS_KMS_MASTER_KEY` (`enterprise_identity.rs:497-503`).
- **Port style:** `crates/maos-domain/src/ports/siem_projection.rs` in full.
- **Redaction needs no extension:** `maos-iac/src/adapter/redaction.rs:68-107` already covers `sk-`, `sk-ant-*`,
  `sk-proj-` (`:69-81`) and `ghp_`/`gho_`/`ghs_`/`ghu_` (`:85-97`).

**Do not.** Do not reuse the `uninstall` exit codes `0/3/4/5` (pinned by 12 tests) or the lock set's `69`/`78` for
a different meaning. Do not create a `.lock` file inside any store directory. Do not add a `maosctl purge`. Do
not let `keyring` reach `maos-domain`'s dependency closure (`check_dependency_closure.rs:65`). Do not read env
inside `maos-secrets`. Do not touch `crates/maos-kernel-core/src`.

### Project Structure Notes

New files: `crates/maos-bin/src/purge.rs` (doorbell 22 → 23), `crates/maos-domain/src/ports/secret_store.rs`,
`crates/maos-bin/tests/maos_uninstall_16_4.rs`, `docs/adr/ADR-067-*.md`. One new crate dependency (`keyring`,
`maos-secrets` only) and therefore one `Cargo.lock` change. One new env var (`MAOS_SECRETS_BACKEND`); one dead
env var wired (`MAOS_CRL_PATH`). No new crate, no `xtask/src` change, no `coverage-matrix.yaml`/
`gate-registry.toml`/`EXPECTED_GATES` change (`discipline.yml:3656-3660` — 16-1's enrolment precedent).
⚠ `maos_uninstall_16_4.rs` spawns a `maos` root, so it must carry a `HOME`/`MAOS_HOME`/`env_clear` literal or
`root_spawn_home_isolation_16_1.rs` reds it locally and the CI decoy reds it in CI.

### References

- [Source: `epics/epic-16-one-daemon-one-door-j0-w1.md` — 16-4 section (Round 8 applied, §12), exit line 7, Kernel-Δ, Kloc asks, Dependencies]
- [Source: `prd/functional-requirements.md:3,25`; `prd/non-functional-requirements.md:52,72,156`; `prd/domain-specific-requirements.md:48`; `prd/developer-tool-specific-requirements.md:50,57`; `prd/user-journeys.md:295,305`]
- [Source: `architecture-maos-minimal-opus/3-vocabulary-invariants.md:36` (I9); `4-kernel-design.md:76,161,341-343`; `12-architecture-decision-records.md:126` (ADR-005)]
- [Source: `docs/adr/ADR-051`, `ADR-062`; `docs/adr/index.md`]
- [Source: `16-1-daemon-post-surface-and-verb-retarget.md` §11 row 2, V-21, V-30; `16-2-shell-halt-registry-and-j0-scene.md` §11 rows 3–4, D-16-2-F, D-16-2-O]
- [Source: `deferred-work.md:888-892,925,937`]
- [Source: `xtask/kloc.toml:49-95,247,320,386,395,450,459,475,642,655-656`; `xtask/kernel-core-baseline.toml:488` (`src_lines`), `:493` (file set), `:521-525` (`[kernel_src]`)]

---

## Tasks / Subtasks

- [x] **T0 — re-measure; trust nothing in this file** (all ACs)
  - [x] `git status --short` clean; record HEAD. Run `kloc-check --json`, `check-kernel-baseline`,
        `check-exit-commands`, `check-env-contract`, `cargo deny check`, and
        `cargo test -p xtask --test recovery_lane_ceiling_rule`. **Capture §0's failing transcript FIRST**, then
        land D-16-4-P as its own commit (`RATIFIED_AT_EASING` 18935 → 18938) whose message attributes the raise
        to Story 16-3 and states that no ceiling moves — ahead of any other 16-4 code.
  - [x] Re-derive every cite in §1–§6 and the Decisions table. Re-run the runtime probes on fresh binaries and
        record each HEAD red: init's false instruction on the default path; the XDG tree surviving `rm -rf $MAOS_HOME`;
        the 4 KB `.sqlite` + 189 KB `-wal` after a one-shot; `/proc/locks` 6 FLOCK + 2 POSIX; the unknown-verb
        stdout/stderr split.
  - [x] Measure `maos-registry`/`maos-skill` ceilings and headroom; confirm `maos-domain`'s +151 against 16-5's
        joint booking (`kloc.toml:394`).
  - [x] Confirm `keyring`'s resolved tree against `[bans] multiple-versions = "deny"` BEFORE writing code.
- [x] **T1 — the enumeration** (AC1, D-16-4-C) — **the `MAOS_CRL_PATH` wiring and the `maos-domain` CRL resolver
      land FIRST**, before any root is enumerated: until they do, tree 4 is the machine-global `/tmp/maos/crl`
      and a purge test would race the workspace suite and delete host state. Then `purge.rs` root model with
      **`written_by: Maos | Operator` as a field of the type**, `maos_roots()` (tree 3 via the already-`pub`
      `fn default_audit_key_path`, **not** the private `dirs_config_home`), the **legacy
      `$HOME/.local/share/maos` leg** with its permanent-not-transitional comment and `MAOS_REGISTRY_YANK_CURSOR_PATH`
      — **no Policy-G edit; `maos-registry` and `maos-skill` are not touched** — retire the two dead corpus resolvers
      **and their 8 inline unit tests**, and the dead `logs/` leaf **plus the two `init_test.rs` assertions that pin
      it** (`:94-95`, `:231`).
- [x] **T2 — the verb** (AC1, AC4, D-16-4-B/G/H) — `VerbName::Purge` + `VERBS` row + both `main()` arms + handler;
      lock acquisition, typed 69/78, TTY/`--yes`; `verb_table_15_3.rs` four sites; doorbell 22 → 23.
- [x] **T3 — removal, `--keep-log`, the receipt and the report** (AC1, AC3, D-16-4-E/I/Q) — ordered removal honouring
      `written_by`, `rmdir`-if-empty, the left-behind report; WAL checkpoint and the retained unit; tenant-mode dual
      retention; sidecar handling; report-only residue sweep; **the dry-run disclosure with counts queried from the
      log**; **the receipt writer, its default path outside every MAOS root, its typed inside-a-deleted-tree refusal,
      and the `cargo uninstall maos` line**.
- [x] **T4 — the true instruction** (AC2, D-16-4-D) — the `run_init` line; the captured-line test; its HEAD red
      recorded before the repair.
- [x] **T5 — the keyring** (AC5, D-16-4-J/K/L/O) — `SecretStore` port; three backends; composition-root wiring;
      provider injection; `anthropic_key_usable`; both `FIXME`s; purge's keyring erasure; `Cargo.lock` + `deny.toml`.
- [x] **T6 — contract and docs** (AC6, D-16-4-M/N) — `env_contract.rs` entries; `MAOS_CRL_PATH` wired; three
      `run-maos.md` copies; `ADR-067` + the `index.md` row; **the PRD amendment as its OWN reviewable commit**
      (authored, not enacted — AC6 asserts it exists and is cited).
- [x] **T7 — sweep** — convert the load-bearing cites to **symbol-first, line second** (~20, §15 R2/F6);
      `deferred-work.md` rows for §11 rows 3, 5, 7, 10; epic + `epic-20` + `sprint-status.yaml` edits
      verified present (§12); `cargo fmt --all -- --check`; measured `kloc.toml` raise for `maos-bin` citing `16-4`;
      all six gates; `cargo test --workspace --no-fail-fast`; every proven-red run, recorded and reverted.

---

## 12. Epic OLD→NEW edits (rule 9 — APPLIED at story creation, 2026-09-16)

Every OLD anchor was verified to exist exactly once in the file named before replacement; no row replaces a pin
value or a line number that is data.

| # | File / location | OLD (anchor) | NEW (summary) |
|---|---|---|---|
| 1 | epic-16 header | before `**Wave / duration:**` | NEW **Round 8** paragraph: 16-4 re-derived at `822070d7`; premises in all three ACs disproved; pointer to this file |
| 2 | epic-16 stories table 16-4 row | `` maos-secrets +150–300, maos-bin +100–200, maos-providers +30–60, maos-domain +30 `` | measured re-book: maos-bin ≤ +775 (ZERO headroom, measured raise), maos-secrets ≤ +390, maos-domain ≤ +107, maos-providers ≤ +78; FR2 **PARTIAL** + NFR-Ops-1 |
| 3 | epic-16 16-4 section | the three AC bullets | ⚠ R8 SUPERSEDED paragraph summarising the 6 ACs; the original AC1–AC3 kept verbatim below it |
| 4 | epic-16 Kloc asks | the 16-4 figures and the `aggregate=158000` claim | ⚠ R8: measured at `822070d7` — maos-bin/maos-domain/maos-shell/maos-cli/maos-control/maos-journey-test/xtask all stale; aggregate **164216**, alarm FIRING |
| 5 | epic-16 doorbell sentence | `` 16-3's `supervision.rs` makes it 22 `` | appends ⚠ R8: **22 at `:881`** at `822070d7`; 16-4's `purge.rs` makes it 23 |
| 6 | epic-16 16-5 AC1 | `the decision and its reason are recorded here.` | appends ⚠ R8: D-16-2-O's shared-tier-only emptiness proof joins the recorded decision (§11 row 3) |
| 7 | `epic-20-ship-it-w5.md:83` AC3 | `maos uninstall` | `maos purge` + ⚠ R8 note: the token was operator-ruled distinct at the Epic-15 retrospective |
| 8 | `sprint-status.yaml` 16-4 row | `` 16-4-maos-uninstall-and-keyring: backlog  # NEW `maos uninstall` `` | `ready-for-dev` + a summary comment naming `maos purge`, the four trees and the six inherited obligations; prior comment kept after `PRIOR:` |
| 9 | `sprint-status.yaml` epic-16-retrospective row | the existing OWES list | appends ⚠ ALSO OWES (Story 16-4): NFR-Aud-12 receipt vehicle; JIT materialization (ADR-005 + `domain-specific-requirements.md:48`); Policy-F `XDG_CONFIG_HOME`; `migration_plan` `$HOME/.maos` leg; the siem-snapshot leak. ⚠ **The `RATIFIED_AT_EASING` clause is REMOVED** — operator-ruled fixed in 16-4 (D-16-4-P) |
| 10 | epic-16 **exit block line 7** | `maos purge --keep-log` | `maos purge --keep-log --yes` — the ONE exit-block COMMAND edit this story makes, forced by D-16-4-G's reversal (without `--yes` the line would be a dry run). `check-exit-commands` resolves the `purge` token either way; `bash -n` re-run clean |
| 11 | `epic-20-ship-it-w5.md:130` Dependencies | `` 16-4 (`maos uninstall` — 20-2 AC3) `` | `` 16-4 (`maos purge` — 20-2 AC3; ⚠ R8) `` — declared here because the edit was made; validation round 1 caught its absence |
| 12 | `epic-17-workers-and-third-party-form-w2.md` 17-2 | its AC text | appends ⚠ R8 (Story 16-4 §11 row 9): 17-2 wires `apply_resource_ceiling`, so it owns whether a leaked `/sys/fs/cgroup/maos/spirit-*` is possible and who removes it — `rmdir`-only, write-on-a-root-owned-parent, `EBUSY` while populated; `maos purge` reports and removes none |
| 14 | epic-16 16-4 section (R8 paragraph) | the R8 SUPERSEDED paragraph | appends the round-table's six reversals: receipt SEAM ships (not cut); `written_by` (purge never removes an Operator-authored file); Policy G NOT fixed here; `--keep-log` disclosure with counts; PRD amendment authored-not-enacted; symbol-first cites |
| 15 | `sprint-status.yaml` epic-16-retrospective row | the OWES list appended in row 9 | adds the resolver inconsistency (three `dirs_fallback`/`cursor_file_path` sites ignore `XDG_DATA_HOME`) — §11 row 10 |
| 13 | `prd/functional-requirements.md` · `prd/non-functional-requirements.md:52` · `prd/user-journeys.md:295` | — | ⚠ **NOT §12 edits — these are T6 dev work** (D-16-4-M's amendment and the `~/.maos/logs/` path correction), listed here only so no reader mistakes them for applied planning edits |
| 16 | `xtask/kloc.toml` maos-bin pin | `maos-bin = 22657` (+1609 over 21048, already over the ≤ +775 grant) | **Review round 2 (2026-09-16) operator-ratified re-booking: `maos-bin = 22938`.** The round-table ruled the overshoot is round-1/round-2 review hardening — receipt rollback, inode-verified deletion, WAL retention, override-shadowed enumeration, teams/ retention + disclosure, post-deletion failure containment — and that safety is not un-landed to fit a number (Grumbal); the ledger entry names the driver per Yui's binding condition. Also round-2: `Cargo.lock` restored to baseline + keyring-tree-only additions (obligation (i)); two further HEAD reds attributed in §0 (kernel-pin 24474, fr4 token pin) |

---

## 14. Open questions for the operator

**None open.** Q1–Q4 and Q6 were ruled on 2026-09-16 (§15). Q5 (review net) is recorded as a recommendation in
the frontmatter and was not contested.

---

## 15. Operator ratification 2026-09-16

| # | Question | Ruling | Applied at |
|---|---|---|---|
| **Q1** | WHOLE, or split `16-4b` = the keyring half? | **WHOLE — one story** | frontmatter `split_from`; D-16-4-A |
| **Q2** | AC5/AC6 implement a capability `functional-requirements.md:3` says cannot exist without an amendment, and NFR-Sec-19 defers | **Author the amendment in-story** | D-16-4-M; AC6; T6 |
| **Q3** | Default retention — FR2 says remove everything, `user-journeys.md:295` says the log persists | **Delegated back with a criterion:** *"choose which respects the spec closely and aligns with the other stories and fulfils the end goals."* Derived: **delete by default, `--keep-log` opts in.** `user-journeys.md:295`'s climax is **`cargo uninstall maos`** — a cargo command that never executes MAOS code, so data necessarily survives it; it describes a different command, and `:305` leaves the choice to the operator explicitly. FR2 is the binding requirement; the epic's exit line 7 carries `--keep-log`, which would be a no-op under the other default; and `developer-tool-specific-requirements.md:50` requires every install verb to have an **inverse**. | D-16-4-F (with the derivation recorded, not the preference) |
| **Q4** | §0: HEAD is CI-red from 16-3's `RATIFIED_AT_EASING` | **Fix it inside 16-4** — the recommendation to reopen 16-3 was **overruled**. Recorded as such, with the mitigation that the fix is its own commit, ahead of any 16-4 code, whose message attributes the raise to Story 16-3 so the ledger does not record 16-4 as the cause. | §0; **D-16-4-P**; §11 row 4 (closed, not routed); obligation (t); §12 row 9 |
| **Q5** | Review net non-degradable? | Not contested — recorded as recommended. | frontmatter `review` |

### Round-table 2026-09-16 (second round) — six forks resolved, all applied

| # | Fork | Ruling | Applied at |
|---|---|---|---|
| **F1** | NFR-Aud-12's receipt was cut and routed | **Seam ships now, signature routed.** NFR-Aud-12 names *substrate-uninstall*; 16-4 builds the only one, so routing it whole was *a promise about someone else's behaviour* (the room's post-15-5 disqualifier) | **D-16-4-Q**; AC1; §11 row 1; obligation (u) |
| **F2** | AC1 deleted `$XDG_CONFIG_HOME/maos`, incl. `spirit-signing.key` | **Purge never removes a file MAOS did not write**, carried as `written_by` on the root model. Settled on evidence: our own README says `openssl genpkey` (`maos-spirit-cli/README.md:15`); only `audit-signing.key` is ours (`fn generate_audit_key`) | **D-16-4-C**; AC1 + its two falsifiers; obligation (v) |
| **F3** | Policy G fixed at origin | **Reversed — not fixed here.** *A story that deletes must not also relocate.* Fixing it would ADD a migration without removing purge's obligation to know the legacy path. Purge enumerates both; resolver defect → retro | **D-16-4-C**; §10 (registry/skill → untouched); §11 row 10; obligation (w) |
| **F4** | `--keep-log` silently retains Art.17 personal data | **Disclosure at the decision point** — the (now default) dry run prints categories with counts **queried from the log**, never prose | **D-16-4-E**; AC3; obligation (x) |
| **F5** | Can a dev story amend a binding PRD section? | **Authored, not enacted.** Own commit, own review; AC6 asserts the amendment exists and is cited | **D-16-4-M**; AC6; T6 |
| **F6** | ~120 line-only cites; 19 already wrong | **Symbol-first, line second** for load-bearing cites, convention stated in frontmatter per 16-3 | frontmatter; T7 |

### Round-table 2026-09-16 (third pass) — security review, Vex

| # | Finding | Ruling | Applied at |
|---|---|---|---|
| **S1** | **The keyring is not a runtime boundary at this threat model.** secret-service is session-scoped, not process-scoped; `README.md:282-284` already records that until 17-1 a Worker runs at the operator uid and reads `control.json` — it reads the collection too | **At-rest gain only, said in two sentences on the page.** Claiming isolation we do not deliver is the "claim standing in for a control" failure mode | D-16-4-K; AC6 docs |
| **S2** | The receipt's "inside a deleted tree" refusal is a **path** comparison, defeated by a symlinked parent | **Canonicalise both sides + re-verify by inode after open**, reusing 16-1's `fstat` vs `stat` (`operator_door.rs:1788-1797`) — the comment there already names `maos purge` | D-16-4-Q(a); obligation (y) |
| **S3** | `--receipt` is an operator-privileged write ⇒ a pre-planted symlink is an arbitrary-write primitive | `O_NOFOLLOW\|O_CREAT\|O_EXCL`, mode 0600, never overwrite | D-16-4-Q(b); obligations (y), (ab) |
| **S4** | The CWD default can itself be inside a MAOS root (`cd ~/.maos && maos purge`) | Trip the refusal; never silently relocate | D-16-4-Q(c); obligation (z) |
| **S5** | **The story never said WHEN the receipt is written** (Murat). After ⇒ a mid-purge crash leaves nothing, the exact NFR-Aud-12 gap; before ⇒ it records intent, a receipt that can lie | **Two-phase**: `status: "in-progress"` plan first, finalised in place | D-16-4-Q(e); obligation (aa) |
| **S6** | The receipt is a host-layout recon document | 0600; no bearer token, no key material | D-16-4-Q(d); obligation (ab) |
| **S7** | Unnamespaced keyring entries ⇒ one purge in a scratch `HOME` deletes the developer's real key | Namespace entries by the MAOS home | D-16-4-K; obligation (ac) |
| **S8** | A locked collection blocks on an unlock prompt ⇒ headless daemon hangs | Bounded attempt → env fallback, downgrade journaled, **no key in the row** | D-16-4-K; obligation (ad) |
| **S9** | The amendment leans on `domain-specific-requirements.md:48`, whose three clauses are JIT / bounded in-memory lifetime / never logged — **we deliver only the last** (`api_key: String`, no `zeroize` or `secrecy` in the workspace) | **Scope the amendment to the key SOURCE**; say JIT and bounded lifetime remain open | D-16-4-M; §11 row 2 |
| **S10** | secret-service pulls zbus and below into a substrate that advertises auditability, and `cargo deny` is advisory in CI | T0 enumerates the tree before any code; the local run **is** the control and is **recorded as weaker than a gate** | §10 |

*Vex conceded S1's framing to Sally's question — "what do we tell the user?" — and the answer became the two-sentence doc line rather than a caveat buried in a decision.*

**Validation round 1 (fresh context, adversarial) — applied in full.** It returned **5 ship-blockers, 28 must-fix
and 10 nits**. The ship-blockers were: a **phantom fifth root** (`/sys/fs/cgroup/maos/spirit-*` — zero production
callers, and `epic-17:67` already rules its writer DELETED, so a row labelled *"measured at runtime"* could not
have been measured); **cgroup dirs are unremovable** by the prescribed mechanism (`rmdir`-only, root-owned
parent, `EBUSY`); **AC3 was vacuous at `0 == 0`**; **`--keep-log` was false in tenant mode**, where the log and
the memory/principal DB are different files; and **AC4's "removes nothing" control was vacuous** because the lock
acquire creates roots at 0700 *before* the flock that refuses. It also reversed D-16-4-G (the earlier
"non-TTY proceeds" would have let any bare script call delete a host unconfirmed) and corrected D-16-4-K's Linux
backend from keyutils to secret-service. Every finding is applied; the story's claims are weaker and true rather
than stronger and wrong.

---

## Dev Agent Record

### Agent Model Used

openai-codex/gpt-5.6-sol

### Debug Log References

- Baseline: captured the dirty worktree, `HEAD`, KLOC, kernel baseline, exit-command, environment-contract, recovery-lane, dependency, and `cargo deny` results before implementation.
- Proven-red/fix loops: false `init` instruction; destructive-purge dry run; lock contention and absent-root rollback; WAL retention; unsafe receipt paths; non-repeatable default receipts; provider secret-store errors; parallel `/tmp` lock contention; stale kernel-pin and FR4 optional-token assertions.
- Focused final verification: `cargo test -p maos-bin --test maos_uninstall_16_4`; `cargo test -p maos-bin --test erasure_uninstall_13_5b`; `cargo test -p maos-secrets --all-features`; `cargo test -p maos-providers`.
- Final gates: `cargo fmt --all -- --check`; `cargo run -p xtask -- kloc-check --json`; `cargo run -p xtask -- check-kernel-baseline`; `cargo run -p xtask -- check-exit-commands`; `cargo run -p xtask -- check-env-contract`; `cargo test -p xtask --test recovery_lane_ceiling_rule`; `cargo deny check`.
- Final regression: `cargo test --workspace --no-fail-fast` — 4,445 passed, 121 ignored, 0 failed across 536 suites.
- Knowledge graph refreshed with `graphify update .`.

### Completion Notes List

- Added the first-class `maos purge` verb with typed MAOS/operator root ownership, dry-run-by-default behavior, mandatory `--yes`, offline-exclusive locking, safe root validation, no-follow deletion, and clean reinitialization.
- Added WAL-aware `--keep-log`, tenant/default database retention, retained-category disclosure, independent two-phase receipts, crash-safe atomic finalization, and report-only container/cgroup residue handling.
- Added the `SecretStore` port and environment, OS-keyring, and encrypted-file adapters; provider injection; bounded and audited fallback; known-name purge deletion; namespace isolation; and error-preserving provider construction.
- Added the CRL/environment contracts, ADR-067, PRD amendment, three operator-guide copies, deferred-work routing, exit-command ownership, and exact zero-headroom KLOC ledger.
- Adversarial review: 20 patch findings applied, 1 scope decision resolved, 0 deferred, and 4 findings dismissed where they contradicted the ratified story contract.
- Upgraded `rustls` to 0.23.45 and `rustls-webpki` to 0.103.15 so the required dependency-policy gate is green.

### File List

- `.github/workflows/discipline.yml`
- `.github/workflows/journey-nightly.yml`
- `Cargo.lock`
- `_bmad-output/implementation-artifacts/16-4-maos-uninstall-and-keyring.md`
- `_bmad-output/implementation-artifacts/deferred-work.md`
- `_bmad-output/implementation-artifacts/intent-lineage-coverage-report.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/party-mode/memories/installed/.memlog.md`
- `_bmad-output/planning-artifacts/epics/epic-16-one-daemon-one-door-j0-w1.md`
- `_bmad-output/planning-artifacts/epics/epic-17-workers-and-third-party-form-w2.md`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md`
- `_bmad-output/planning-artifacts/prd/functional-requirements.md`
- `_bmad-output/planning-artifacts/prd/non-functional-requirements.md`
- `_bmad-output/planning-artifacts/prd/user-journeys.md`
- `crates/maos-audit/src/lib.rs`
- `crates/maos-audit/tests/fr4_classifier_16_2.rs`
- `crates/maos-bin/Cargo.toml`
- `crates/maos-bin/src/enterprise_identity.rs`
- `crates/maos-bin/src/env_contract.rs`
- `crates/maos-bin/src/lib.rs`
- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/src/operator_door.rs`
- `crates/maos-bin/src/purge.rs`
- `crates/maos-bin/src/verbs.rs`
- `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs`
- `crates/maos-bin/tests/maos_uninstall_16_4.rs`
- `crates/maos-bin/tests/reports/section-13-1-smoke.json`
- `crates/maos-bin/tests/verb_table_15_3.rs`
- `crates/maos-domain/src/ports/mod.rs`
- `crates/maos-domain/src/ports/secret_store.rs`
- `crates/maos-domain/src/revocation.rs`
- `crates/maos-providers/src/anthropic.rs`
- `crates/maos-providers/src/openai.rs`
- `crates/maos-secrets/Cargo.toml`
- `crates/maos-secrets/src/lib.rs`
- `crates/maos-secrets/tests/at_rest_seal.rs`
- `crates/maos-shell/src/lib.rs`
- `crates/maos-shell/tests/init_test.rs`
- `crates/maos-wasm-host/guests/equiv-fixture/native-twin/Cargo.lock`
- `deny.toml`
- `docs-site/docs/run-maos.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md`
- `docs/adr/ADR-067-maos-data-root-layout-and-purge.md`
- `docs/adr/index.md`
- `docs/maos.dev/run-maos.md`
- `xtask/kloc.toml`
- `xtask/tests/kernel_pin_content_hash_16_0.rs`
- `xtask/tests/recovery_lane_ceiling_rule.rs`

### Change Log

- 2026-09-16: Implemented Story 16-4 end to end; applied adversarial-review hardening; repaired required baseline gates; moved to review.

### Review Findings

- Fixed: unsafe explicit-root widening, filesystem-root deletion, receipt option parsing, symlink/ancestor races, arbitrary audit-WAL retention, tenant disclosure, receipt durability and repeatability, scoped stable locks, encrypted-file replacement, keyring namespace collisions, fallback deletion/auditing, and provider error propagation.
- Dismissed against the authoritative story: non-Unix purge is explicitly typed exit 78; `MAOS_CRL_PATH` is ratified as a directory contract; the `/tmp` legacy fallback relocation is routed to the Epic 16 retrospective; registry/skill writer migration is outside this deletion story's no-relocation boundary.

#### Round 2 — bmad-code-review 2026-09-16 (4 layers: Blind + Edge Case + Acceptance + Test-Infra; non-author; every finding verified against the working tree before rating)

Decision-needed (all four RESOLVED 2026-09-16 by code-review-crew round-table — Vex, Grumbal, Boundary, Yui, Dana, with Winston/Amelia/John; operator ratified the room's consensus, including the one overturn and one amendment):

- [x] [Review][Patch] (was Decision) Tenant sibling-team Transparency Logs under `--keep-log`: **RULED — protect the whole `teams/` subtree AND enumerate+disclose.** Deletion is irreversible; disclosure alone still destroys other teams' Art.17-relevant `shell.turn` audit records under a flag spelled keep-log (Vex, Grumbal); but protected-silently is deleted-silently with the opposite sign, so dry-run and receipt must name every sibling log with row counts (Dana's condition). Overturns the operator's provisional disclose-only pick; ratified. [crates/maos-bin/src/purge.rs:854-861, 883-913]
- [x] [Review][Patch] (was Decision) Non-unix keyring: **RULED — gate the `keyring` feature out of non-unix builds AND move the non-unix default backend to `env`, with `Backend::Keyring` parsing to a typed exit 78 "not in this build"** (Boundary's amendment: gating the feature without moving the default bricks every Windows boot at main.rs:3563-3569). Aligns with the round-one ratified dismissal "non-Unix purge is explicitly typed exit 78" — a store purge can't enumerate must never be created (Vex). [main.rs:3617-3656; operator_door.rs:1661-1668; maos-bin/Cargo.toml:55]
- [x] [Review][Patch] (was Decision) Air-gap build links keyring/zbus stack: **RULED — `default-features = false, features = ["encrypted-file", "keyring"]` on maos-bin's maos-secrets dep + record in story/ADR-067.** Unanimous; air-gap doctrine is compile-out-networking and zbus pulls an async executor plus ~30 crates of D-Bus attack surface into a binary that may have no secret-service daemon. Regular build unchanged. [maos-bin/Cargo.toml:55; maos-secrets/Cargo.toml:22]
- [x] [Review][Decision] maos-bin Kloc +1609 vs ratified ≤ +775: **RULED — operator re-booking ratified 2026-09-16** (Grumbal: the overshoot is round-one review hardening — receipt rollback, inode-verified deletion, WAL retention; you don't un-fix safety to fit a number). Yui's condition binding: the §12/kloc ledger entry MUST name round-one hardening as the driver, not just "it grew". No code change; ledger recording lands with the patch batch.

Patch:

- [x] [Review][Patch] HIGH — `maos_home()` error swallowed, home root silently dropped, exit 0: `if let Ok(Some(home))` (crates/maos-bin/src/purge.rs:60) discards e.g. `MaosHomeError::Relative`; purge then deletes every other root, names no home row even in dry run, exits 0 — the story's declared failure mode. Map to typed Configuration (78) before snapshot/lock; surface Ok(None) as a named row.
- [x] [Review][Patch] HIGH — `MAOS_AUDIT_DB`/`MAOS_JOURNAL_PATH` stores silently orphaned whenever `MAOS_HOME` is set: resolver precedence MAOS_HOME→MAOS_AUDIT_DB (crates/maos-audit/src/lib.rs:934-947) means maos_roots (purge.rs:43-49) never sees the override target — not enumerated, not checkpointed, not removed, receipt reports clean. Test `purge_keeps_explicit_override_siblings_and_checkpoints_custom_audit_names` (maos_uninstall_16_4.rs:529) sets MAOS_HOME AND MAOS_AUDIT_DB, so the custom DB is shadowed and the test is vacuous. Enumerate the two overrides independently (as already done for MAOS_MEMORY_ROOT/MAOS_ARCHIVE_DIR/MAOS_ERASURE_PROOFS_DIR); rewrite the test.
- [x] [Review][Patch] HIGH — Symlinked `$XDG_CONFIG_HOME/maos` makes every confirmed purge fail forever AFTER deletion succeeded: remove_empty_audit_config_dir (purge.rs:1289-1322) tolerates only NotFound|DirectoryNotEmpty; remove_dir on a symlink → ENOTDIR → exit 1, receipt stuck in-progress, keyring credentials skipped, every retry re-enters the same branch. Guard with symlink_metadata(parent); treat NotADirectory|PermissionDenied as report-only.
- [x] [Review][Patch] HIGH — Post-deletion `?` chain (purge.rs:741-756): any receipt-update/restore/cleanup error after filesystem deletion → exit 1, in-progress receipt, no dispositions printed; cosmetic steps (remove_empty_audit_config_dir, snapshot.restore) run BEFORE delete_known_secrets, so they can abort credential erasure; no terminal `failed`/`partial` receipt status exists (handled error indistinguishable from SIGKILL). Delete secrets before cosmetic steps; finalize the receipt failed/partial on every handled post-creation error.
- [x] [Review][Patch] HIGH — Cargo.lock carries unrelated upgrades (rustls 0.23.40→0.23.45, rustls-webpki →0.103.15, chacha20 →0.10.2, socket2 0.5.10→0.6.3, itertools 0.10.5→0.14.0 + new serde/wasm-bindgen/memchr edges), defeating obligation (i) ("git diff Cargo.lock non-empty exactly once, for keyring") and S10's substitute dependency control. Revert the unrelated bumps; land any policy-driven upgrade as its own reviewed change.
- [x] [Review][Patch] MED — Audit-override sidecars unenumerated: with MAOS_AUDIT_DB outside the default data tree, `<stem>.siem-snapshot-*.sqlite` (enterprise_identity.rs:413-455 — a full VACUUM INTO copy, leaked on kill), `.team` binding, `export-seq.json`, `transparency.airgap-stub.log` survive unenumerated and unnamed; ADR-067's "purge removes leaked SIEM snapshots, export cursors, and air-gap stubs" is conditionally false. Enumerate the resolved audit parent / its MAOS-written siblings.
- [x] [Review][Patch] MED — Healthy-but-erroring keyring bricks every boot with exit 78: provider construction maps store `get` errors to Transport (anthropic.rs:36-42, openai.rs same) → main.rs:3691-3713 exit(78), where pre-change code degraded to provider-less boot; FallbackSecretStore returns the primary error when env has no value; the composition-time provisioning `Err` branch is silently discarded (main.rs:3622-3625). Map store errors to Unconfigured (journaled); journal the provisioning failure.
- [x] [Review][Patch] MED — `MAOS_OPENAI_API_KEY` unregistered in env_contract.rs (only MAOS_ANTHROPIC_API_KEY at :75) while the read moved into the scanned crate via `std::env::var(name)` indirection (main.rs:3551-3561) the check-env-contract gate cannot see — reproducing the §6 defect inside the scanned crate. Register both credential vars (or read each by registered literal).
- [x] [Review][Patch] MED — Receipt parent safety implements only half of ruling S2/obligation (y): validate_receipt_path canonicalizes (purge.rs:1366-1400) and ReceiptFile::create opens the parent O_NOFOLLOW (:493-504) but never re-verifies fstat-vs-stat after open (16-1's pattern, operator_door.rs:1788-1797), leaving the canonicalize→open TOCTOU the ruling closed.
- [x] [Review][Patch] MED — File-kind roots bypass every safety validation: validate_purge_roots skips non-directory roots (purge.rs:800-803), so TL/journal/audit-key/MAOS_REGISTRY_YANK_CURSOR_PATH (:138-146, taken verbatim from env) get no absolute/parent checks — a relative cursor kills purge mid-deletion (receipt in-progress, credentials skipped); any absolute env value is deleted as MAOS-owned unchecked. Extend validation to file roots; absolutize overrides before planning.
- [x] [Review][Patch] MED — Silent, unjournaled env→keyring promotion on every boot (main.rs:3621-3641): a healthy keyring + absent entry + env value ⇒ MAOS writes the secret into the OS keyring permanently; only failures are journaled; run-maos.md ×3 describe only read-side fallback; a later purge then deletes an entry the operator never asked MAOS to create. Journal the promotion (no material, mirroring S8) and document it.
- [x] [Review][Patch] MED — Two further HEAD-red repairs landed inside this story without the §0/D-16-4-P attribution discipline: kernel_pin_content_hash_16_0.rs 24474→24477 (proven red at 822070d7 vs kernel-core-baseline.toml:488) and fr4_classifier_16_2.rs:318-323, whose `TokenColumn::Optional => true` arm now asserts NOTHING for Optional rows (a row silently flipping to Optional escapes). Split into attributed commits (or amend §0); pin what Optional still guarantees.
- [x] [Review][Patch] MED — Registry import-bundle cache `$HOME/.cache/maos/import/<sha256>` (maos-registry/src/import.rs:236-244, ignores XDG_CACHE_HOME; MAOS_IMPORT_SCRATCH_ROOT also unenumerated) is MAOS-written state enumerated by no one and routed to no one, though the story's own §2 census lists this exact site. Add it to maos_roots as WrittenBy::Maos, or file the retro row rule-10 requires.
- [x] [Review][Patch] MED — Obligation (r) safety interleavings untested: root-is-symlink refusal (purge.rs:819-838) and root type-mismatch disposition (:993-998) have zero coverage — a regression dropping the symlink guard deletes through a symlinked MAOS_HOME with every test green; HOME-unset legacy leg also untested. Add the three fixture tests + written interleaving statement.
- [x] [Review][Patch] MED — FilesystemSnapshot::restore (the D-16-4-H acquire-mutates-then-refuses trap) never exercised with real work: tests snapshot identity only after boot creates every lock-set candidate (maos_uninstall_16_4.rs:165-235, 680-743), so restore is always a no-op and AC4's "proven red (2)" scenario is nowhere a permanent test. Add a refusal/dry-run case with one store root deleted post-boot.
- [x] [Review][Patch] MED — Pre-planted receipt-symlink case under-asserts the O_NOFOLLOW|O_EXCL refusal it pins (maos_uninstall_16_4.rs:663-675): asserts only !success + target content; a purge that unlink()ed the symlink and wrote a fresh receipt would pass. Assert exit 78 + ReceiptPathUnsafe marker + symlink_metadata still a symlink.
- [x] [Review][Patch] MED — AC1's stdout obligations never asserted: the destructive run asserts only status.success (maos_uninstall_16_4.rs:243-248); nothing greps per-root removed:/absent: lines or the left-behind report (purge.rs:758-770), so print-side regressions ship green. Assert the lines; plant one kept-tree leftover; cross-check receipt dispositions as the keep-log test does.
- [x] [Review][Patch] LOW — HOME set-but-empty or non-UTF-8 divergence: purge's legacy leg filters empty HOME → /tmp (purge.rs:127-130) while producers use env::var("HOME") with no empty filter (maos-registry/src/storage.rs:435-439) → producers root at CWD-relative `.local/share/maos`, purge enumerates /tmp — silent under-deletion. Replicate producer semantics or refuse Configuration loudly.
- [x] [Review][Patch] LOW — env_contract.rs:245-247 still documents MAOS_CRL_PATH as "Certificate Revocation List file path" while the ratified contract (story line 794) is a directory (default_crl_dir returns it verbatim, revocation.rs:424-429). Amend the purpose string.
- [x] [Review][Patch] LOW — Receipt disposition `kept` overstates partially-removed roots: planned_disposition marks a directory root "kept" when ANY protected path exists inside it (purge.rs:917-926) — under --keep-log the home root reports `kept` while config.toml/control.json/journal/skills were deleted. Record a partial disposition or per-child rows.
- [x] [Review][Patch] LOW — SecretKey::environment_variable() (secret_store.rs:25) is dead API; main.rs:3546-3553 re-hardcodes the literals — drift decouples purge's keyring names from the env source. Build the EnvSecretStore pairs from the method.
- [x] [Review][Patch] LOW — FallbackSecretStore::get (maos-secrets/src/lib.rs): whitespace-only primary reported as "absent or empty"; when both stores error only the fallback's error is returned, discarding the primary's.
- [x] [Review][Patch] LOW — print_report_only_residue (purge.rs:1482-1510) shells `podman ps`/`docker ps` with no timeout; a wedged container daemon stalls purge after the receipt is complete. Spawn piped + short deadline, kill on timeout.
- [x] [Review][Patch] LOW — AC4's LockUnavailable-78 leg untested; --keep-log retained-sidecar refusal (checkpoint_retained_databases, purge.rs:1324-1349) tested only post-hoc (no live second reader); obligations (z) cd-into-home refusal, (aa) kill-between-phases in-progress receipt, (ab) receipt grep for bearer/sk- material, (ac) cross-home keyring isolation have mechanisms but no test evidence. Add the fixtures.
- [x] [Review][Patch] LOW — Non-unix filesystem_identity stub returns an empty map (maos_uninstall_16_4.rs:776-779), making inertness/refusal identity assertions vacuous `empty == empty` there while the enclosing tests would hard-fail on Windows anyway (--receipt refused on non-unix). cfg-gate the tests; make the stub unreachable.
- [x] [Review][Patch] LOW — Tenant keep-log test's read-back proves file retention, not production retention (maos_uninstall_16_4.rs:453 plants its own `purge_retention` table; WAL/SHM removed by the test's own closes; no receipt `kept` disposition asserted, unlike the host-log test :437-439). Read back production-written rows / assert kept dispositions.
- [x] [Review][Patch] LOW — Audit-query before/after line counts filter to JSON-parseable lines (maos_uninstall_16_4.rs:372-378, 426-432); extra chatter or a swallowed error line passes. Parse each line into the entry shape.
- [x] [Review][Patch] LOW — Composition-root secret-backend selection (main.rs:3541-3670, AC5/AC6) has no test: backend=encrypted-file without MAOS_KMS_MASTER_KEY → 78, backend=env registration, and fallback journaling are undriven; AC6's falsifier was story-time measurement only. Add integration coverage or extract a testable selector.
- [x] [Review][Patch] LOW — AC1's destructive assertions re-list hardcoded fixture paths (maos_uninstall_16_4.rs:165+) instead of re-resolving through production resolvers as AC1 requires; only the AC2 child does. Reuse the isolated re-resolution child after the destructive purge.
- [x] [Review][Patch] LOW — sprint-status.yaml:253 epic-16-retrospective row still carries the RATIFIED_AT_EASING=18935-vs-18938 clause that D-16-4-P discharged in-story. Delete the stale clause.
- [x] [Review][Patch] LOW — run-maos.md ×3 state what --keep-log retains but never frame it as the documented FR2 exception AC3 requires (ADR-067 does). Add the exception sentence.
- [x] [Review][Patch] LOW — Dev Agent Record File List omits modified files: .github/workflows/discipline.yml + journey-nightly.yml, epic-17 md, enterprise_identity.rs, cohort_daemon_smoke_13_5c.rs, section-13-1-smoke.json, intent-lineage-coverage-report.md, wasm-fixture Cargo.lock, party-mode memlog.

Defer:

- [x] [Review][Defer] Legacy `/tmp/maos/crl` written by pre-16-4 builds not enumerated — deferred, pre-existing; same class as the routed `/tmp` legacy-fallback retro row; `/tmp` is volatile

Dismissed (1): ancestor-guard flock contention claim (two-level ancestor guards, operator_door.rs:1745-1766) — intentional design with in-code rationale; the claimed cross-user `/home` collision is inaccurate (guards reach two levels up from store dirs, not `/home`).

**Round-2 closure 2026-09-17:** all 35 patch findings applied (4 decision rulings converted after the code-review-crew round-table; operator ratified). One further production defect was caught by the round-2 test suite itself and fixed: secret receipt rows indexed at `filesystem_root_count` clobbered kept sibling-log rows — now indexed at a `secret_offset` captured after the sibling extend (`purge.rs:926-958`). Verification at close: `cargo fmt --all -- --check` clean; `kloc-check` PASSED (maos-bin re-booked 22657 → 22938, aggregate 166542 < 170884); `check-kernel-baseline` PASSED (zero kernel-Δ, 24477/98); `check-exit-commands` PASS (`purge` resolved, 3 owed are epic-20's); `check-env-contract` PASS (94/0, MAOS_OPENAI_API_KEY/MAOS_AUDIT_DB/MAOS_IMPORT_SCRATCH_ROOT registered); `cargo test -p maos-bin --test maos_uninstall_16_4` 23/23; `cargo test --workspace --no-fail-fast` zero failures. `cargo deny check advisories` FAILS on pre-existing RUSTSEC-2026-0285 (rustls 0.23.40) + yanked chacha20 0.10.0 — identical at baseline 822070d7, CI-advisory, routed to 20-3b in deferred-work.md (the dev-pass smuggled upgrades were reverted to keep obligation (i) intact).
