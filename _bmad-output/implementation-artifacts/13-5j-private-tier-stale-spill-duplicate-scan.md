---
baseline_commit: 04a6e72d
depends_on: 13-5i-private-tier-filesystem-residue
kernel_grant: AUTHORIZED 2026-07-27 — FLAG-Winston, pin 23318 → 23401 (+83 measured), all six defects
---

# Story 13.5j — The private tier's read surface disagrees with its erase surface

Status: **done**

**Kernel-Δ: AUTHORIZED bounded FLAG-Winston re-pin `23318 → 23401` (+83 physical / +34 tokei, measured).** Operator-ratified 2026-07-27 **at the measured number, after the code was written and every limb falsified** — never on an estimate. No KLOC ceiling re-base: `maos-kernel-core` goes 17 941 → 17 975 against the 18 248 ceiling, leaving **273**. Filed by the 13.5i code review as Residual 1 and held for its own grant under 13.5d's *"do not spend the grant elsewhere"* fence.

> **Read this before anything else — it corrects the brief this story was filed under, in two directions.**
>
> **The scope was under-filed.** The residual named two defects. Six exist, all on the same two functions, all of one family: **`write`, `read` and `scan` disagree with what `forget_principal` already treats as authoritative.** Story 13.5i made the erase path filesystem-authoritative, fail-closed and symlink-contained. It deliberately did not touch the read path, and the read path never caught up. Fixing only the filed two leaves `scan` able to fail an entire namespace on one junk node and leaves the read path uncontained while the erase path is contained — an asymmetry a reviewer would (correctly) call out.
>
> **The severity was over-filed.** The residual calls this *"a live data-integrity and data-exposure bug."* It is **latent at HEAD**, and saying otherwise would be exactly the failure mode this project names: a claim standing in for a measurement. **No shipped writer produces the trigger.** The sole production private writer is `halt/resolver.rs:170`, which writes `Text` under unique `halt_context::<halt_id>` keys — never superseded, because `resolve` asserts the halt was `PendingResolution` and cannot run twice. The sole production scanner is `memory_backed_digest_provider` (`maos-iac/src/adapter/decision_logger.rs:93`), which scans the `digest:` prefix — and **nothing in the workspace writes a `digest:` key outside tests.** The surface is nonetheless fully wired: `KernelCtx::memory()` (`scheduler/kernel_ctx.rs:129`) hands the whole `MemoryManagerAdapter` to Spirit-hosting code, so both halves are one Spirit write away. This is a repair made **before** the trigger ships, not after — which is the cheap time to make it.

---

## Story

**As** the kernel maintainer responsible for what a Spirit's working memory returns,
**I want** `write`, `read` and `scan` to hold to the same account of the filesystem that `forget_principal` already does,
**so that** a Spirit is never served a value it overwrote, a signed `decision.*` frame never claims the Spirit read one digest twice, and a link planted inside a Spirit's own memory area cannot read outside it.

---

## The defect, in code — six of them, all measured before a line was written

Every claim below was produced by a throwaway probe harness run against `04a6e72d`, **not** by reading. All six probes failed at HEAD; all six pass after the repair. The evidence quoted is literal probe output.

### D-1 — `write` never unlinks a superseded spill (FILED)

`write` (`private.rs:188`) spills to `fs_path_for(…, value.kind())`, whose filename carries a **per-kind extension**. Nothing removes the previous one. Write key `k` as an 8 KiB `Json`, then as an 8 KiB `Blob`:

```
P1 files on disk: ["k.bin", "k.json"]
P1 warm kind=Blob  cold kind=Json
```

Both files persist. The warm read is correct because the in-memory cache answers first (`:236`). After a restart the cache is empty, `read`'s fixed-order kind probe (`:243-248`) reaches `.json` **before** `.bin`, and the store serves the value the Spirit already overwrote.

### D-2 — `scan` unions the two sources without a shared key identity (FILED)

`write` for any non-`Markdown` value over `inline_threshold` does **both** `write_to_disk` *and* `map.insert` (`:216`). `scan` then iterates the map and the directory and pushes from each:

```
P3 scan keys = ["digest:aaa", "digest:aaa"]
```

One logical key, two entries. The production consumer does not deduplicate — `memory_backed_digest_provider` maps entries to refs, sorts, and hands them to `decorate_decision_frame`, so the duplicate lands in a **signed `decision.*` frame's `working_memory_digest_refs`**. `MAX_DIGEST_REFS_SCAN` is 256; duplication halves it, so a Spirit reasoning over many digests would have had *real* refs silently dropped from an audit record.

The read-through cache is the amplifier. A value written once, spilled, then merely **read**, becomes duplicated:

```
P4 warm=2  cold_before_read=1  cold_after_read=2
```

A pure read changed what a scan reports.

### D-3 — `scan` and `forget_principal` disagree about what a logical key IS

13.5i's review replaced `file_stem()` with extension-stripping in the erase path, precisely so the empty key round-trips. `scan` was never updated (`:310`, pre-fix). An empty-key `Markdown` value is the file `.md`, which `file_stem()` reads as the name `".md"` with **no** extension:

```
P5 files=[".md"]  scan_len=0
```

`forget_principal` counts it. `scan` cannot see it. Two functions, one directory, two answers.

### D-4 — one junk node fails the whole namespace scan

The private tier's Markdown area is **deliberately** operator-editable (`:203-208`), so hand-created residue is reachable by design. `scan` had no regular-file guard and called `read_from_disk` on whatever it found:

```
P6 scan result = Err(Io(Os { code: 21, kind: IsADirectory }))
```

Not one entry lost — the **entire** namespace scan fails. And the production consumer swallows it: `Err(_) => WorkingMemoryDigestRefs::default()` (`decision_logger.rs:122`). The result is a signed decision frame that silently claims the Spirit reasoned over nothing.

### D-5 / D-6 — the read path is not symlink-contained, while the erase path is

13.5i closed symlink traversal on `forget_principal` at **both** the pid and namespace level, because a followed link meant counting bytes into an Article 17 receipt that were never erased. The read path kept `ns_path.is_dir()` (`scan`) and `path.exists()` (`read`) — **both traverse symlinks**. A link planted inside a Spirit's own area therefore reads entries from outside it (I5). `exists()` also answers `true` for a directory, which is how D-4 reaches `read` as well.

---

## Why this is a kernel repair and not an adapter one

The same four grounds that settled 13.5i, re-checked against this surface:

1. **The port's own contract already promises it.** `MemoryManagerPort` documents the private tier's filesystem area as part of the store's state (`maos-domain/src/ports/memory.rs:86-89`). A store that serves a superseded value is not meeting its own published contract — this is a **conformance repair**, and `kloc.toml` states a budget rule *"must never block a correctness or compliance repair."*
2. **There is no seam above it.** `write`/`read`/`scan` are `pub(in crate::memory)`. The disagreement is *between* the map and the directory, both private to `PrivateMemoryStore`. Nothing outside the kernel can observe, let alone reconcile, them.
3. **The duplicate is sealed before any adapter sees it.** By the time `memory_backed_digest_provider` has the entries, the two-entry `Vec` is already the answer. An adapter-side dedup would paper over the cardinality without fixing the stale-value read at all.
4. **Half the fix is deletion of state, not addition.** `remove_superseded_spills` exists to make the durable state smaller and more determinate. That is not adapter work.

---

## Acceptance criteria

**AC1 — `write` leaves at most one durable file per logical key, and the pin moves only after that is proven.**
- After any `write`, the namespace directory holds **exactly one** spill for the key, or none. Kind change and shrink-below-threshold are both covered.
- The new spill is durable **before** its predecessors are unlinked, so a crash in between leaves a recoverable superset, never a gap. Do not reorder this.
- ⚠ This does **not** make sub-threshold values durable. They are process-lifetime working memory by design, so a cold read returning `None` is honest; returning the superseded 8 KiB blob is not. Assert `None`, and assert the file is gone — do not assert warm/cold equality, which would be asking for a durability guarantee this tier does not offer.
- Re-pin `xtask/kernel-core-baseline.toml` to the **measured** value with a nine-element HISTORY row. `xtask/fkcs-baseline.toml` stays byte-untouched at its frozen 23081.

**AC2 — `scan` merges by logical key, cache-first.**
- A key held in both the cache and the filesystem is **one** entry, and the cached copy wins — the precedence `read` already applies (`:236`).
- A plain `read` must not change scan cardinality.
- Key recovery goes through the same extension-stripping identity `forget_principal` uses, over a **single** `ALL_KINDS` list, so the two cannot drift. The empty key must round-trip.

**AC3 — junk is skipped, never fatal, never attested.**
- A non-regular-file node in a namespace directory is skipped, exactly as `forget_principal` skips it (`:440`). One junk node must not fail the namespace.

**AC4 — the read path is symlink-contained.**
- `scan` and `read` both use `symlink_metadata`, not `is_dir()`/`exists()`.
- ⚠ The asymmetry with `forget_principal` is **deliberate and must be preserved**: erasure *errors* on a namespace symlink because a miscount would be Ed25519-signed into an Article 17 receipt; a read *reports nothing*, which is already the safe direction. Do not "make them consistent" by erroring in `scan`.

**AC5 — 13.5i's erasure count is unchanged by residue this story can no longer create.**
- A store built before this repair could leave two files for one logical key. Erasure destroys both; the receipt must still count **one**. This is a forward-compatibility guard on the upgrade path, and a regression guard on 13.5i's distinct-key identity.

**AC6 — every property is bound by an `--exact` Blocking leg, and each was falsified alone.**
- One `#[test]` per assertion surface, one `--exact` `BindingClass::Blocking` leg per test in `check_reza_production_path`, so `run_test_leg`'s `running 1 test` / `1 passed` anti-vacuity check is meaningful.
- Serialized per-limb falsification before the pin moves. A mutation must red **its own** leg and leave the others green.

---

## Traps

**Trap 1 — the filed severity is wrong and the honest version is weaker.** Say "latent, fully wired", not "live". Nothing writes `digest:` keys and the one production private writer uses unique keys. Overstating it here would be the project's signature failure mode pointed at its own remediation.

**Trap 2 — `read`'s kind-probe order looks like the bug and is not.** It probes `Markdown` first because Markdown is authoritative; `ALL_KINDS` starts with `Json`. Do **not** unify them. The order stops mattering once at most one file can exist — fix the cause in `write`, not the symptom in `read`.

**Trap 3 — `remove_superseded_spills` will delete an operator's hand-written `.md`.** That is correct: the Spirit wrote a new value for that key, and leaving the `.md` means the new value is invisible after restart. It is still a real behavioural change to the documented operator-editable area — state it, do not discover it in review.

**Trap 4 — 4 syscalls on every small write is a hot-path cost.** Short-circuit on the namespace directory's existence first, so a key that never spilled costs one `stat`.

**Trap 5 — the `break` → `continue` change in the in-memory loop is NOT separately observable.** `limit` caps the filesystem pass before a missing `cached` entry could resurrect anything, so a complete versus truncated key set produces identical output today. **Claim no control for it.** Keep it, because the comment above it would otherwise be false — but do not write a test that cannot fail.

**Trap 6 — do not copy a frame constructor from memory.** `decision_frame` in this story's control file must match `i12_real_digest_provider_8_10.rs:40` field for field. Inventing it costs a compile cycle and eleven errors (it did).

**Trap 7 — extension ambiguity is not a risk, and the comment should say why.** `.json`/`.md`/`.bin`/`.txt` are mutually non-suffixing, so `find_map` order does not affect the match. A key like `report.2026` or `notes.md` round-trips intact.

**Trap 8 — build serialization.** Every mutant run must show `Compiling maos-kernel-core`, and the source must be restored byte-identically after each. (13.5i lost a scout to a stale-artifact false GREEN.)

---

## Tasks

1. Probe first. Build a throwaway harness that reproduces each defect through `MemoryManagerAdapter` (the store methods are `pub(in crate::memory)`). Do not design until all probes are red. Delete the harness before commit.
2. Add `ALL_KINDS`; split `spill_name_parts` out of `key_from_spill_name` so `scan` gets the kind too, without duplicating an extension literal.
3. Add `remove_superseded_spills`; call it from `write` after the optional spill.
4. Rewrite `scan`'s filesystem pass: `symlink_metadata` gate, regular-file guard, `spill_name_parts` key recovery, `cached` skip, limit-check before `read_from_disk`.
5. Swap `read`'s `path.exists()` for `symlink_metadata`.
6. Write `crates/maos-kernel-core/tests/private_spill_supersession_13_5j.rs`; one test per surface, including the end-to-end proof through `memory_backed_digest_provider`.
7. Wire ten `--exact` Blocking legs into `check_reza_production_path`.
8. Falsify M1–M6, serialized, restoring after each.
9. Re-pin at the measured number with the nine-element HISTORY row; update ADR-059.
10. `cargo fmt --all`, full workspace regression, all three gates.

---

## Dev notes

- **`ValueKind` derives `PartialEq`, `Eq`, `Copy`** (`maos-domain/src/memory.rs:178`), so `Some(kind) == keep` compiles without a discriminant dance.
- **`hex` is already a dependency**, and `namespace_to_dirname`'s hand-rolled `format!("{byte:02x}")` loop is untouched by this story — leave it.
- **The house test idiom is `Command::new(env!("CARGO_BIN_EXE_maos"))`.** There is no `assert_cmd`, `escargot` or `predicates` anywhere in the workspace, and `cargo-deny` is live. Do not add one.
- **`cargo fmt --check` is Blocking since E12-B4.** `max_width = 100`, no `rustfmt.toml`. Run `cargo fmt --all` before measuring the pin — the pin is measured on the *formatted* tree.
- **Reuse the 13.5i fixture shape.** `Fixture::open()` returning a fresh store over the same `fs_root` is how a restart is modelled; `TransparencyLogAdapter::open_with_global_legal_holds(&db, &db, 1)` is the working constructor.

---

## Budget

| | physical | tokei |
|---|---|---|
| `maos-kernel-core/src` at `04a6e72d` | 23318 | 17941 |
| after 13.5j | **23401** | **17975** |
| delta | **+83** | **+34** |
| ceiling | — | 18248 (**273** spare) |

The physical delta is doc-heavy by design: the reasoning about *why* `read`'s probe order stopped mattering, and why the erase/read asymmetry is deliberate, is the part a future story will otherwise undo. `fkcs-baseline.toml` byte-untouched at 23081.

---

## Previous-story intelligence (13.5i)

- The 13.5i review found that Trap 3 of *that* story — "the namespace level is safe" — was right about deletion and wrong about counting. The same shape recurs here: a `read_dir` that follows a link is a different hazard from a `remove_dir_all` that does not. **Always ask what the traversal is being used *for*.**
- 13.5i's `check-fkcs` finding is still live and still ownerless: the gate computes `dev_blocks`, documents it as hard-failing a RED oracle regardless of phase, serializes it — and never consults it (`check_fkcs.rs:325`).
- Kernel-core's 49 in-`src` `#[cfg(test)]` modules remain budget-charged and CI-unexecuted. This story adds **no** in-`src` tests, for that reason.

---

## Residuals

1. **Non-atomic forget (ADR-059 Residual 9)** — untouched, still ownerless.
2. **`principal_index` WAL/free-page residue** — untouched; the asymmetry with the filesystem tier persists.
3. **Sub-threshold values are not durable.** Deliberate, now explicit in code and in the control. If working memory is ever required to survive a restart, that is a design change, not a bug fix.
4. **No gate reconciles the pin literal with its HISTORY rows** — ownerless since 13.5i; this story adds a second row that a ~10-line xtask leg would have verified.
5. **Epic-13 story count is stale.** The epic file says *"Eleven stories total"*; there are now **16**.

---

## Dev Agent Record

### Agent Model Used

`openai-codex/gpt-5.6-sol`

### Debug Log

- Probe harness `zz_probe_13_5j.rs`: 6/6 RED at `04a6e72d`, 6/6 GREEN after the repair, deleted before commit.
- Baseline measured in an isolated `git worktree` at HEAD to avoid the 13.5i dirty-tree measurement error: physical 23318, tokei 17941. Confirmed against the pin literal.
- Per-limb falsification, serialized, source restored byte-identical after each (verified with `diff -q` against a pristine snapshot):

| mutant | red | green |
|---|---|---|
| M1 `write` does not unlink | `write_unlinks_the_superseded_spill_when_the_kind_changes`, `write_unlinks_the_spill_when_the_value_shrinks_below_the_threshold` | 8 |
| M2 `scan` drops the cache-identity skip | `scan_returns_one_entry_for_a_key_held_in_cache_and_on_disk`, `a_read_does_not_change_scan_cardinality`, `digest_refs_are_not_duplicated_by_a_spilled_working_memory_entry` | 7 |
| M3 `scan` back to `file_stem()` | `scan_recovers_the_empty_key_from_its_spill_name` | 9 |
| M4 drop the regular-file guard | `scan_skips_a_directory_that_looks_like_a_spill` | 9 |
| M5 `scan` back to `is_dir()` | `scan_does_not_follow_a_namespace_directory_symlink` | 9 |
| M6 `read` back to `exists()` | `read_does_not_follow_a_spill_symlink` | 9 |

- Gates: `check-kernel-baseline: PASSED (23401 == 23401)`; `kloc-check: PASSED (aggregate=134423)`; `check-reza-production-path: PASSED`, 53 legs, 0 blocking red, all 10 new legs `running 1 test; 1 passed`.

### Completion Notes

- Six defects filed as two; all six confirmed by running code before design, all six fixed, each falsified alone.
- Severity **corrected downward** from the filed "live" to latent-but-fully-wired, with the reachability trace recorded rather than asserted.
- No control is claimed for the `break` → `continue` change; it is not observable under `limit`.
- ADR-059 Decision 12 opened, Residual 10 CLOSED, Decision 10's forward reference updated, Consequences amended.


### Review Findings — 2026-08-01 backfill

*bmad-code-review backfill reviewed `c2e55a25` on 2026-08-01: Blind Hunter + Acceptance Auditor.*

- [x] [Review][Patch] Make overwrite persistence durable before advancing the cache; a spill failure must leave the previous durable/cache state intact [crates/maos-kernel-core/src/memory/private.rs]
- [x] [Review][Patch] Use unique create-exclusive temporary spill files and serialize concurrent writes so one writer cannot replace or delete another writer's output [crates/maos-kernel-core/src/memory/private.rs]
- [x] [Review][Patch] Deduplicate legacy spill files by logical key with latest-wins selection, returning exactly one authoritative value and safely removing obsolete duplicates [crates/maos-kernel-core/src/memory/private.rs]
- [x] [Review][Patch] Perform spill I/O and deletion directory-relatively through no-follow handles so pid, namespace, and spill symlinks cannot escape containment [crates/maos-kernel-core/src/memory/private.rs]
- [x] [Review][Patch] Surface metadata and lookup failures as `PrivateMemoryError`; never treat an unreadable directory or failed lookup as absence [crates/maos-kernel-core/src/memory/private.rs]
- [x] [Review][Patch] Fsync the completed spill file and containing directory around atomic replacement so durable visibility survives a crash [crates/maos-kernel-core/src/memory/private.rs]
### File List

- `_bmad-output/implementation-artifacts/13-5j-private-tier-stale-spill-duplicate-scan.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `crates/maos-kernel-core/Cargo.toml`
- `crates/maos-kernel-core/tests/private_forget_restart_13_5i.rs`
- `crates/maos-kernel-core/src/memory/private.rs`
- `crates/maos-kernel-core/tests/private_spill_supersession_13_5j.rs`
- `docs/adr/ADR-059-operator-authority-collective-erasure.md`
- `xtask/kernel-core-baseline.toml`
- `xtask/src/check_reza_production_path.rs`

### Change Log

| Date | Note |
|---|---|
| 2026-08-02 | CI-blocker review follow-up resolved the six 2026-08-01 findings and six additional Blind Hunter/Edge Case Hunter patches: descriptor-relative no-follow traversal, serialized hard-link-backed rollback transactions, exclusive temp files, parent/file/directory fsync, fail-closed hostile PID and equal-mtime handling, prefix-isolated scans, and explicit cleanup errors. Public-adapter integration coverage now carries the private-store contracts. Added safe `rustix` filesystem support and updated the measured kernel baseline without raising its logical-line ceiling. |
| 2026-07-27 | Story created and implemented in one pass against `04a6e72d`. Preflight was a **probe harness, not a review**: six defects reproduced against running code before any design, including four the filed residual did not name. **Scope corrected upward** (2 → 6 defects, all on `write`/`read`/`scan`, all one family: the read surface disagreed with the erase surface 13.5i made authoritative). **Severity corrected downward** (filed "live" → latent-but-fully-wired; no shipped writer produces the trigger, traced to `halt/resolver.rs:170` and `decision_logger.rs:93`). Operator-ratified 2026-07-27 at the **measured** `+83 physical / +34 tokei`, after the code existed and M1–M6 were falsified. Ten `--exact` Blocking Reza legs. ADR-059 Decision 12 + Residual 10. Ceiling untouched (273 spare); `fkcs-baseline.toml` byte-untouched. |
