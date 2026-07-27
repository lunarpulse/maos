---
baseline_commit: dd4a908e
depends_on: 13-5h-shared-tier-principal-partition
kernel_grant: AUTHORIZED 2026-07-26 — FLAG-Winston, pin 23234 → 23280 (+46 measured), Shape A4
---

# Story 13.5i — Private-tier filesystem residue: the production uninstall erases nothing and signs a proof saying it did

Status: **done**

**Kernel-Δ: AUTHORIZED bounded FLAG-Winston re-pin `23234 → 23280` (+46 physical / +30 tokei, measured).** Operator-ratified 2026-07-26 on the measured shape, not on the filed estimate. No KLOC ceiling re-base: `maos-kernel-core` goes 17 894 → 17 924 against the 18 248 ceiling, leaving **324**. Filed from the 13.5b code review (ADR-059 Decision 10 / Residual 8) and held for its own grant by the 13.5h panel.

> **This story is ready to implement as specified. Read the next two paragraphs before anything else — both invert the brief this story was filed under.**
>
> **It is not a Markdown residue bug.** Both production forget entry points are one-shot CLI modes, and the private store is constructed *in the same process* that immediately forgets. `in_mem` is therefore **structurally always empty at forget time**, `to_remove` is always `[]`, and the filesystem-cleanup loop iterates **zero times**. The private tier is **never erased by the production uninstall path, for any value kind**. Markdown is merely the one class that also fails in-process. A fix that special-cases Markdown does not close this.
>
> **The filed `+35..55` estimate contains the right answer (+46) for entirely wrong reasons, and that makes it a trap.** It over-priced the mechanism it named — the reverse of `namespace_to_dirname` is **4 combinator lines**, because `hex = "0.4"` is already a `maos-kernel-core` dependency (`Cargo.toml:27`) — and omitted the work that actually costs: exact-once counting (**+13**), fail-closed I/O (**+6**), symlink containment (**+3**). A team de-scoping "to the cheap version" against that range ships the **+12** variant, which was built and measured: it **fails open** (`Ok(0)` with PII on disk), **escapes `fs_root`**, and reds two existing tests. See *Trap 1*.

---

## Story

**As** the operator answering a GDPR Article 17 request on a MAOS Host,
**I want** `maos uninstall` to actually delete the principal's private-tier bytes from disk and to count what it destroyed,
**so that** the Ed25519-signed proof-of-erasure stops recording `memory_namespace: Removed { count: 0 }` over data that is still sitting in the memory root — and FR65's promise that *"substrate-uninstall is a real guarantee, not a hope"* becomes true.

---

## The defect, in code (verified 2026-07-26 against `dd4a908e`)

### D-1 — it is structural, not an edge case

Both production forget entry points are one-shot CLI modes dispatched at `main.rs:4799`:

- `main.rs:4962` — `MAOS_ONE_SHOT=forget` → `memory.forget_with_reason(…)`
- `main.rs:4905` → `run_uninstall_cascade` (`main.rs:8078`) → `run_uninstall_cascade_inner` (`:8116`) → `memory.forget_with_reason(principal_id, None)` (`:8171`)

The private store is constructed at **`main.rs:2436-2439`** — `PrivateMemoryStore::new(memory_root, 4 * 1024)` — in *that same process*, and no Spirit runs before the one-shot dispatch. `PrivateMemoryStore::new` (`private.rs:30-36`) is a pure constructor: `in_mem: RwLock::new(HashMap::new())`. No hydration, no WAL replay, no lazy index.

So `in_mem` is **always empty at forget time**. `forget_principal` (`private.rs:319-379`) derives `to_remove` **exclusively** from that map (`:323-337`), and the filesystem-cleanup loop at `:356-363` iterates **over `to_remove`**. Zero entries in, zero directories removed. **`ForgetReceipt.deleted_entries` from the private tier is structurally always `0` in production.**

Measured end-to-end with the real binary (Scout 3, probe 4): `exit 0`, `"outcome":"erased"`, `"erased_principals":1`, signed proof bundle written to disk — with an 8 KiB blob still present under the memory root.

### D-2 — the codebase already knew, and wrote an assertion that cannot fail

`crates/maos-bin/tests/erasure_uninstall_13_5b.rs:513-519`:

```rust
// NOTE: `deleted_entries` is the private tier's count and is 0 for any
// out-of-process uninstall — see `private_tier_forget_never_reaches_disk`
// below. Assert the durable, cross-process effect instead: index rows.
assert!(
    terminal["deleted_entries"].as_u64().is_some(),
    "the terminal must carry an effect count, whatever its value"
);
```

Three failures in seven lines. The comment states the defect precisely. The assertion checks only that the field **is a number** — *"whatever its value"* — so it passes over any count, forever. And `private_tier_forget_never_reaches_disk` **does not exist** anywhere in the repository; the intended referent is `private_tier_markdown_survives_the_forget_cascade` at `:798`.

### D-3 — NULL CONTROL #24: the `"private"` erasure backend is discharged by an unfalsifiable leg

`REGISTERED_ERASURE_BACKENDS` (`memory/mod.rs:35`) lists `"private"`, and `multi_backend_erasure_test.rs` signs off on it like this:

- `:118-127` plants `MemoryValue::Text(format!("{CANARY} hello"))` — **~30 bytes**.
- `:149-154` asserts `!private_store_contains(&fs_root, CANARY)` — a **pure filesystem scan** (`:42-61`).
- `:154` pushes `"private"` into `proved_erased`.

A sub-threshold `Text` **never touches the filesystem**: `write` spills only when `needs_spill` (`private.rs:177-181`), and the threshold is a strict `>` against 4 KiB (`:160`). So the assertion **cannot fail regardless of what `forget_principal` does**. It is green in a world where the private tier erases perfectly and green in a world where it erases nothing.

Same unfalsifiable shape at `gdpr_cascade_test.rs:77`, `erasure_smoke_test.rs:83,254`, `gdpr_cascade_corpus_test.rs:126`, `principal_namespace_lifecycle.rs:40-77`, and the in-crate unit test `private.rs:580-603`. **The only spilling values anywhere in the erasure suite** are `gdpr_cascade_corpus_test.rs:257` and `erasure_uninstall_13_5b.rs:641` — and both use one to force an *IO failure*, not to prove erasure.

⚠ This is structurally identical to **null control #23**, which 13.5h deleted for `"shared"`. The `"private"` twin survived because 13.5h scoped itself, correctly, to the Shared tier. **Killing #24 is this story's load-bearing deliverable — the 46 kernel lines are the easy part.**

### D-4 — partial erasure is destructive, non-erasing, and self-sealing

The in-memory removal (`:339-351`) completes **before** the filesystem loop (`:353-363`). Measured with the namespace directories made unwritable:

```
forget result = Err(Io(PermissionDenied))
in-mem 'tiny'  readable after failed forget = false   ← destroyed
spilled 'big'  readable after failed forget = true    ← intact
```

Two consequences:

1. **Destructive-and-non-erasing.** A ≤4 KiB value that existed only in memory is gone forever while the on-disk bytes remain.
2. **Self-sealing.** The map entries that *named* those subtrees are already dropped, so a retry after the operator fixes permissions finds `to_remove == []` and **can never reach the leftover subtree**. On the success path `principal_index.forget` (`mod.rs:579`) deletes the address rows, so nothing anywhere still points at it. **The filesystem walk is required for recoverability, not only for correctness.**

Also: `mod.rs:578` uses `?`, so a private-tier IO error aborts `forget_with_reason` **after** the distillate and cost-frame scrubs at `mod.rs:517-575` have run and **before** `principal_index.forget` at `:579` — a torn multi-backend state with no compensating action.

### D-5 — residue taxonomy (empirical, HEAD)

| Value | Spills? | Same process | After restart (= production) |
|---|---|---|---|
| `Markdown`, any size | always (`:159`) | count 0; **residue** unless a cached sibling shares the *exact* `(pid, principal_id, schema)` | count 0; **RESIDUE ALWAYS** |
| Text/Json/Blob **> 4096 B** | yes | count 1; cached *and* spilled → both removed | count 0; **RESIDUE ALWAYS** |
| Text/Json/Blob **≤ 4096 B** | never | count 1; nothing on disk | **value already gone** — never persisted |

Two facts to state and not "fix": ≤4 KiB private values are **RAM-only and are lost outright across a restart** (a durability property, not residue); and `read()` rehydrates the cache for spilled **non-Markdown only** (`:232-239`), so "warm the cache first" is not a viable fix — `:233` explicitly skips Markdown.

### Honest severity

**Reached, not merely reachable.** Unlike 13.5h's latent Shared-tier hole, this fires on **every real operator uninstall** of any principal that ever wrote a Markdown value or anything over 4 KiB. The bytes survive, `subject_access_query` correctly reports the principal gone, and the Host signs an Ed25519 proof saying `Removed { count: 0 }`. **An honest zero under a `Removed` label still reads as erased** (ADR-059 Decision 10).

---

## Ratified implementation shape — A4 (measured 2026-07-26, all variants built and reverted)

| # | Shape | physical Δ | tokei Δ | verdict |
|---|---|---:|---:|---|
| B1 | hex-**prefix** match, refined | +13 | +5 | fails open; `fs_root` escape |
| A1 | **decode** every dirname | +12 | +6 | fails open; `fs_root` escape |
| A2 | A1 + no-swallow + symlink-safe | +31 | +21 | double-counts |
| A3 | A2 + spill-aware count | +33 | +22 | double-counts |
| **A4** | **A3 + exact distinct-key union** ✅ | **+46** | **+30** | **ships — 14/14 controls green** |

**Shape A (decode) beats Shape B (hex prefix), and the "B is cheaper" hypothesis is measured false** — B is one line *worse* (+13 vs +12). Both need the same directory walk; decode is ~4 combinator lines, prefix-build ~5. B's only rationale was avoiding a decoder, and `hex` is already a dependency.

**B was rejected on failure mode, not cost.** B selects directories by `starts_with(hex("{\"Principal\":{\"principal_id\":\"<id>\""))`. That is correct *only* because `MemoryNamespace::Principal` declares `principal_id` before `schema` (`maos-domain/src/memory.rs:52-55`) with no serde attributes. Reorder those two fields — a refactor no reviewer would flag — and **every on-disk directory stops matching while the cascade still reports success**. The failure mode is *PII quietly survives erasure*. Shape A round-trips through the same serde impl as the writer, so encoder and decoder cannot drift.

**Shape C (out of kernel) — REFUTED on four grounds.** (1) `MemoryManagerPort::forget`'s own doc contract (`crates/maos-domain/src/ports/memory.rs:86-89`) *already* promises deletion from *"the in-memory map, **filesystem area**, and index table"* — so this is a **conformance repair**, and `kloc.toml`'s policy states a budget rule *"must never block a correctness or compliance repair."* (2) `namespace_to_dirname` (`:71`) and `fs_path_for` (`:91`) are private with no `fs_root` accessor; an outside sweeper must re-implement a security-critical encoding as a second, drifting copy — the exact coupling that produced this bug. (3) `deleted_entries` is computed and sealed **inside** the kernel (`mod.rs:578-596`) into the append-only `principal.forget` TL frame *and* the signed `ForgetReceipt` before `maos-bin` ever sees it; an outside sweep deletes the bytes and still signs `Removed { count: 0 }` — reproducing Decision 10 inside its own fix. (4) The trait is not on the live path: both production sites call the inherent `forget_with_reason`, so a `dyn MemoryManagerPort` decorator would never be invoked.

**Shape D (index-driven forward-derive) — rejected.** `PrincipalNamespaceIndex` does record every principal write including Markdown (`mod.rs:735-747`), so `lookup()` could reconstruct directories with no walk. But it makes Art.17 completeness depend on SQLite staying in sync with the filesystem — and Markdown is filesystem-canonical *precisely* so operators can hand-create and hand-edit files, which have no index row. **The disk walk is self-verifying; the index is a claim about the disk.**

### The change — measured diff, `crates/maos-kernel-core/src/memory/private.rs`

```rust
    pub fn forget_principal(&self, principal_id: &str) -> Result<u64, MemoryError> {
        // Identity of an erased entry, shared by both sources so a value that
        // is BOTH cached and spilled is counted exactly once:
        // (`<pid>/<ns_dirname>`, key).  A file's stem IS its key verbatim.
        let mut erased: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();

        // … `to_remove` collection unchanged (:323-337) …

        // Remove from in-memory map — now recording identity, not a counter.
        {
            let mut map = self.in_mem.write().expect("PrivateMemoryStore lock poisoned");
            for (pid, ns, key) in &to_remove {
                if map.remove(&(*pid, ns.clone(), key.clone())).is_some() {
                    let dir = format!("{pid}/{}", Self::namespace_to_dirname(ns)?);
                    erased.insert((dir, key.clone()));
                }
            }
        }

        // `Markdown` never enters the in-memory map (see `write`), so the map
        // cannot name its residue; the on-disk tree is authoritative and is a
        // strict superset of the map-derived subtrees.  Reverse
        // `namespace_to_dirname` on each directory name — hex-decode, then
        // deserialize the namespace JSON — and keep the `Principal` ones
        // naming this principal.  Undecodable names are not namespace dirs.
        // Errors are NEVER swallowed: a skipped directory is silent
        // under-deletion, which would make the Art.17 receipt claim an erasure
        // that did not happen.  `file_type` does not traverse symlinks, so a
        // link planted under `fs_root` is not a dir and the walk cannot escape.
        let fs_root = self.fs_root.clone();
        let pid_dirs = match fs::read_dir(&fs_root) {
            Ok(d) => Some(d),
            // Nothing ever spilled — not an error.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(MemoryError::Io(e)),
        };
        for pid_entry in pid_dirs.into_iter().flatten() {
            let pid_entry = pid_entry.map_err(MemoryError::Io)?;
            if !pid_entry.file_type().map_err(MemoryError::Io)?.is_dir() {
                continue;
            }
            let pid_dir = pid_entry.path();
            let pid_dir_name = pid_entry.file_name();
            let mut cleaned = false;
            for ns_entry in fs::read_dir(&pid_dir).map_err(MemoryError::Io)? {
                let ns_entry = ns_entry.map_err(MemoryError::Io)?;
                let name = ns_entry.file_name();
                let decoded = name
                    .to_str()
                    .and_then(|n| hex::decode(n).ok())
                    .and_then(|b| serde_json::from_slice::<MemoryNamespace>(&b).ok());
                if !matches!(decoded, Some(MemoryNamespace::Principal { principal_id: p, .. })
                    if p == principal_id)
                {
                    continue;
                }
                // A name that DOES decode to this principal's namespace but is
                // not a readable directory is corruption, not junk: the reads
                // below fail loud rather than skip.
                let ns_dir = ns_entry.path();
                let dir = format!(
                    "{}/{}",
                    pid_dir_name.to_string_lossy(),
                    name.to_string_lossy()
                );
                for file in fs::read_dir(&ns_dir).map_err(MemoryError::Io)? {
                    let file = file.map_err(MemoryError::Io)?.path();
                    if let Some(stem) = file.file_stem().and_then(|s| s.to_str()) {
                        erased.insert((dir.clone(), stem.to_string()));
                    }
                }
                fs::remove_dir_all(&ns_dir).map_err(MemoryError::Io)?;
                cleaned = true;
            }
            // Clean up the per-pid directory only if it is now empty.
            if cleaned
                && fs::read_dir(&pid_dir).map_err(MemoryError::Io)?.next().is_none()
            {
                let _ = fs::remove_dir(&pid_dir);
            }
        }

        Ok(erased.len() as u64)
    }
```

**Net is much smaller than gross.** The walk **replaces** the 11-line map-derived fs loop *and* deletes the 12-line `cleaned_pids` block (`:365-376`) — the on-disk tree is a strict superset of the map-derived subtree set, and the per-pid empty-check folds into the same walk via the `cleaned` flag. The old `cleaned_pids` block was itself broken: derived from `to_remove`, it is empty in the Markdown case, so the now-empty pid directory was never reaped.

---

## Acceptance Criteria (6)

**AC1 — The grant is spent exactly as authorized, and recorded.**
Binding conditions, all seven:

1. **Shape A4 only.** The authorized surface is `forget_principal` in `crates/maos-kernel-core/src/memory/private.rs`. Nothing else under `crates/maos-kernel-core/src/` may grow.
2. **The discriminating control lands WITH the fix** (AC4). The pin does not move until the rewritten control is falsified **per limb**.
3. **Code first, then the pin.** Land the change, `cargo fmt --all`, re-measure the formatted tree, re-pin to the **measured** value. 13.5d `:475`: an ahead-of-code re-pin disarms the exact-equality gate. **`23280` is the measured expectation, not a licence to skip re-measuring.**
4. **Exactly one literal moves:** `xtask/kernel-core-baseline.toml` `src_lines`. All binding surfaces read it dynamically. **`xtask/fkcs-baseline.toml:5` (`src_lines = 23081`) is the FROZEN tag — do not touch it.** It is measured against the git tag, not HEAD (`check_fkcs.rs:62-78`), so kernel growth never affects it.
5. **No ceiling re-base.** 17 894 → 17 924 against 18 248 leaves 324. Per `kloc.toml`, slack is *"operating capacity, NOT authorization."* State on the record that **no `hot_swap` extraction is owed**: the `kloc.toml:42-45` precondition is marked `[SUPERSEDED 2026-07-25]` and the policy forbids a budget rule blocking a compliance repair.
6. **ADR-059 amended in the same commit** — Decision 10 (`:88-94`), Residual 8 (`:125`), and the Consequences line at `:139` that still names the private-tier residue as an open limit. Three sites.
7. **No unrelated kernel growth rides along.** 13.5d `:473`. This explicitly excludes the stale-spill defect — see *Residuals* and successor `13-5j`.

Write the HISTORY row in the 13.5h format (nine elements — see *Authorization record*).

**AC2 — `forget_principal` enumerates the filesystem, fail-closed and symlink-contained.**
- Implement exactly the shape above. The on-disk tree is authoritative; the map is a cache.
- ⚠ **Every `read_dir` error becomes a `MemoryError`. No `flatten()` on a fallible iterator, no `unwrap_or(false)`, no silent skip.** A swallowed error is silent under-deletion, which makes the Art.17 receipt claim an erasure that did not happen — *the very defect this story removes, one layer down.* This is also a **regression guard**: HEAD correctly propagates `Err(Io(PermissionDenied))` on an unreadable directory, and the cheap variant does not (Trap 2).
- ⚠ **Symlink containment at the pid level.** Use `pid_entry.file_type()?.is_dir()`, never `Path::is_dir()` — `file_type` does not traverse, `is_dir()` does. Measured: without this guard a symlink planted at `<fs_root>/9` makes `remove_dir_all` **delete a directory outside `fs_root`** and return `Ok(1)`.
- ⚠ **Decode before type-check** (Trap 4). A name that decodes to this principal's namespace but is not a readable directory is **corruption and must fail loud**; a name that does not decode is junk and is skipped.
- Per-namespace `remove_dir_all` remains correct: the directory name hex-encodes the **whole** namespace including `schema`, so each directory is 1:1 with exactly one `(principal_id, schema)` pair and can never hold two principals (`fs_path_for`, `:91-107`). Files are enumerated only to count them.
- Do **not** widen any enum and do **not** change `forget_principal`'s signature.

**AC3 — `deleted_entries` means distinct entries erased, counted exactly once.**
Identity is `(spirit_pid, namespace, key)` — realised as `("<pid>/<ns_dirname>", key)`, since a file's stem **is** its key verbatim (`fs_path_for` deliberately does not mangle it, `:100-102`). A value that is both cached and spilled counts **once**.
State in the ADR why the alternatives are wrong, because each is a different lie:
- *files removed* → reds `principal_namespace_lifecycle.rs:87` (`== 5`; five inline Texts spill zero files), `gdpr_cascade_test.rs:257` (`== 1`), `gdpr_cascade_corpus_test.rs:317`. Under-reports inline-only PII — today's bug in a new place.
- *namespaces removed* → reds the same three; wrong unit.
- *naive map ∪ fs union* → double-counts every spilled non-Markdown value; one >4 KiB Json reports as 2. **Over-reporting in a signed Art.17 proof is a fresh false claim, not a conservative one.**

⚠ **Newly-armed verification path.** `proof.rs:384-387` computes `claims_removal` as *any category with `count > 0`*. Today the private count is always 0, so the empty-proof-set rejection at `:388-395` is **never reached**. A correct count arms it for the first time. `build_erasure_proof` populates `subject_exclusion_proofs` per erased principal (`:216-223`), so it should hold — **prove it, do not assume it.**

**AC4 — Null control #24 is converted into a real control, and falsified per limb. This gates the pin.**
> ⚠ **The single most important line in this story.** `multi_backend_erasure_test.rs:149-154` plants a ~30-byte `Text` that never reaches the filesystem, so the `"private"` discharge passes identically whether `forget_principal` erases everything or nothing. `run_test_leg`'s anti-vacuity check (`check_reza_production_path.rs:87`) is a substring match on `"running 1 test"`/`"1 passed"` and is **structurally blind** to a semantically null assertion. The fix and its detector land together or neither lands.

Rewrite the `"private"` discharge:
- **(a)** Plant a **`Markdown`** value **and** a **>4 KiB non-Markdown** value under the principal namespace — the two classes that actually persist — then exercise the **restart** path (fresh `PrivateMemoryStore` over the seeded `fs_root`, or a subprocess) before asserting absence. Only then push `"private"` into `proved_erased`.
- **(b)** Assert on **content absence**, not on a path set. The existing pinned test compares `Vec<String>` of paths, so a mutation that truncates or zeroes the `.md` in place stays green while the message claims *"the bytes are still there."*
- **(c)** Keep an ADR-026 positive-retention assertion under its own name: a **bystander** principal and a `Default`-namespace record must **survive**. There is no over-deletion control in the suite today.
- **(d)** **Falsify per limb**, recording each transcript:

| # | mutation | must RED | must stay GREEN |
|---|---|---|---|
| M1 | delete the fs walk entirely | all residue legs | the 12 pre-existing tests |
| M2 | restrict the walk to `ext == "md"` | spill-after-restart leg | Markdown leg |
| M3 | restrict the walk to `ext != "md"` | Markdown leg | spill leg |
| M4 | revert the count to map-only | proof-count leg | file-absence legs |
| M5 | swap `file_type()` for `Path::is_dir()` | symlink-containment leg | all others |
| M6 | replace `?` with `flatten()` on `read_dir` | unreadable-directory leg | happy path |
| M7 | broaden to `remove_dir_all(&fs_root)` | bystander/over-deletion leg | target-erasure legs |
| M8 | count per file instead of per distinct key | exact-once leg | absence legs |

M2/M3 are the pair that discharges the *restart-backed* rider. **M6 and M7 are non-negotiable**: M6 because a swallowed error is this story's own defect class, M7 because a GDPR cascade that over-deletes is a different incident with the same green CI.

**AC5 — Every artifact asserting the residue is open is retired in the same commit.**
- `crates/maos-bin/tests/erasure_uninstall_13_5b.rs:798-882` — `private_tier_markdown_survives_the_forget_cascade` **will go RED** (measured). Invert it per its own docstring (`:794-796`), rename to `…_is_erased_by_the_forget_cascade`, assert content absence (AC4(b)), and correct the `namespace_category["status"]["count"] == 0` assertion at `:877-881`.
- Hoist the test-local `spilled_files` closure (`:804-822`) to a module-level `fn`, and fix its `path.is_dir()` (`:811`) — same symlink flaw as AC2.
- `xtask/src/check_reza_production_path.rs:506-522` — rename the **Blocking** leg `gdpr-private-markdown-residue-pinned`; a leg whose name asserts the defect still exists is untrue the moment it is fixed. Follow the 13.5h rename precedent exactly (rename leg → re-point test → rewrite the comment to name the new falsifier → add the anti-vacuity sibling).
- `erasure_uninstall_13_5b.rs:513-519` — replace the *"whatever its value"* assertion with a real one and fix the dangling `private_tier_forget_never_reaches_disk` reference at `:514`.
- ADR-059 Decision 10, Residual 8, Consequences `:139`.
- ⚠ **Do NOT claim these five as evidence** — the brief lists them as "tests that will flip" and **none of them flip** (measured, all green against a working fix): `erasure_smoke_test.rs:297-312`, `gdpr_cascade_corpus_test.rs:316-320`/`:333-336`, `gdpr_cascade_test.rs:239-257`, `principal_namespace_lifecycle.rs:27-88`. They all write sub-threshold `Text`. **Blast radius is one leg, not five.**
- **Verification step, not optional:** afterwards `grep -rn '13-5i' crates/ xtask/ docs/` returns zero hits outside this story file and ADR-059's historical record.

**AC6 — Real controls on the existing host; the new control must actually run.**
- **Fold onto `check-reza-production-path`** — already the v2.2 GDPR host, already runs `multi_backend_erasure_test` as the `gdpr-backend-partition` leg. No new gate.
- Place the new control in **`crates/maos-kernel-core/tests/`** — zero on **both** meters (the pin walks only `src/`; tokei excludes any path component named `tests`). `forget_principal` and `PrivateMemoryStore::new` are both `pub`, so there is no private-helper excuse for an in-`src` test.
- ⚠ **A new test file runs NOWHERE unless you name it.** `discipline.yml` contains **no `--workspace`, no `--all`**, and its single `--lib` run is another crate. All 17 `-p maos-kernel-core` invocations are `--test <name>` addressed. Register the control as a `BindingClass::Blocking` Reza leg **and** confirm the leg executes it.
- **Anti-vacuity:** exactly one `#[test]` per leg, invoked by name with `--exact`; transcript must show `running 1 test` **and** `1 passed`.
- ⚠ **Do not cite `check-fkcs` as a control.** It exits `0` with a 5-of-8 RED oracle (see *Residuals*).

---

## Authorization record — FLAG-Winston, 2026-07-26

**Granted by the operator on the measured shape.** Authorized delta: `src_lines` **23234 → 23280 (+46 physical)**, `maos-kernel-core` tokei **17 894 → 17 924 (+30)**. Ceiling unchanged at **18 248** (324 reserve after the spend). All six candidate shapes were built, compiled, `cargo fmt`-ed, tested, measured, and reverted before the grant was issued; the working tree is verified byte-identical to `dd4a908e` (`check-kernel-baseline: PASSED, 23234 == 23234`).

**Why the grant is 7.7× 13.5h's.** 13.5h was a guard — six lines that refuse an input. This is an enumeration repair with three independent hardening requirements (fail-closed I/O, symlink containment, exact-once identity), and the hardening is **34 of the 46 lines**. The cheap 12-line version was measured and is not shippable.

**The estimate is not the authorization.** The filed `+35..55` (ADR-059 Residual 8, `sprint-status.yaml:229`) contains +46 by coincidence. It over-priced the reverse decoder and omitted the correctness work — see the headline. Record this in the HISTORY row so the next story does not inherit the reasoning.

**HISTORY row — the nine required elements** (copy the 13.5h row's structure verbatim):
1. `#   23280  Story 13.5i — private-tier filesystem residue (+46, AUTHORIZED kernel-core delta).`
2. `FLAG-Winston:` operator, date, and the ratification record.
3. The defect closed, with file:line.
4. Explicit statement that the prior control was a **null control** (#24), measured.
5. **SHAPE**, measured, including the rejected variants and *why* (B rejected on failure mode; the +12 variant rejected as fail-open).
6. **LANDED AND RE-MEASURED** on the formatted tree, both units, plus *"the estimate is not the authorization."*
7. **BINDING CONDITION DISCHARGED BEFORE THIS NUMBER MOVED** — the per-limb M1–M8 falsification transcript.
8. The `fkcs-baseline.toml` non-touch statement.
9. `All additive; no public symbols removed.`

---

## Traps

**Trap 1 — the filed estimate is itself the trap.** `+35..55` contains the answer for the wrong reasons. Anyone "de-scoping to the cheap version" builds the **+12** variant: measured to return `Ok(0)` on an unreadable directory with PII still on disk, to delete outside `fs_root` through a pid-level symlink, and to red two existing tests. **The range is not a menu.**

**Trap 2 — `read_dir(..).into_iter().flatten()` swallows the `Err`.** It is the idiomatic-looking line and it is the bug. A `chmod 000` pid directory is skipped and the cascade reports success with zero deletions. **Regression vs HEAD**, which propagates the error today.

**Trap 3 — `Path::is_dir()` and `Path::exists()` follow symlinks; `DirEntry::file_type()` does not.** Measured on rustc 1.96: `remove_dir_all` on a path that *is* a symlink-to-dir only unlinks the link (so the **ns level is safe**), but `read_dir` **follows** the pid-level link, making `ns_dir` a real directory outside the tree. There is no `SQLITE_OPEN_NOFOLLOW` equivalent in use anywhere in `private.rs`. Guard at the pid level with `file_type()`.

**Trap 4 — decode BEFORE type-check, or you flip a `failed` terminal to `erased`.** Two existing tests plant a regular **file** at a principal namespace path and require `Err`: `erasure_uninstall_13_5b.rs:659` and `gdpr_cascade_corpus_test.rs:343` (corpus scenario `gdpr-cascade-040`, `expected_outcome: "failed"`). The obvious hardening `if !ns_dir.is_dir() { continue; }` turns **both RED** and also reds the Blocking leg `uninstall_forwards_erased_held_not_found_and_failed_codes`. Traps 2 and 4 pull in opposite directions; the rule is **unreadable ⇒ error; decodes-but-not-a-directory ⇒ error; does-not-decode ⇒ skip; genuinely absent ⇒ skip.**

**Trap 5 — double-count.** Write `Markdown` at key `k` (spills to `k.md`, not cached), overwrite `k` with a small `Text` (cached, no spill): `write` never unlinks the stale `.md`, so a naive union counts the key twice. This is why AC3 requires a distinct-key `HashSet`, not two counters. (The underlying stale-spill defect is successor `13-5j` — do not fix it here.)

**Trap 6 — the five "tests that will flip" do not flip.** Measured green against a working fix. Do not cite them; do not "fix" them.

**Trap 7 — the pinned test compares paths, not bytes.** On inversion, assert content absence — otherwise a truncate-in-place mutation stays green under a message that says the bytes survive.

**Trap 8 — build serialization.** A scout got a **false GREEN** on the pinned test from a stale `target/debug/maos` racing a concurrent build; the clean re-run reported FAILED. **All proven-red evidence for this story must come from a serialized build.** Force the rebuild and quote the `Compiling` lines in the Debug Log.

**Trap 9 — `MemoryNamespace::principal()` validates almost nothing.** `maos-domain/src/memory.rs:63-97` rejects only `:`, NUL and ASCII control chars. It **accepts** `"`, `\`, `/`, space and non-ASCII — and `MemoryNamespace::Principal { .. }` is a public variant with public fields, so struct-literal construction bypasses validation entirely. Shape A is immune (it round-trips serde). Any string-built matching is not.

**Trap 10 — kernel-core in-`src` `#[cfg(test)]` costs both budgets and runs in no CI job.** 49 such modules exist. Put the control in `tests/` and name it in a leg.

---

## Tasks / Subtasks

- [x] **Task 0 — establish the clean baseline.** (AC: 1)
  - [x] Confirm `git status --short` is clean and pin == actual == **23234** (`find crates/maos-kernel-core/src -name '*.rs' -exec cat {} + | wc -l`).
  - [x] Record pre-story `kloc-check` numbers: kernel-core **17894**, aggregate **134062**. Note `xtask/fkcs-baseline.toml` FROZEN tag **23081**.
- [x] **Task 1 — the fix (Shape A4).** (AC: 2, 3)
  - [x] Rewrite `forget_principal` per the measured diff: distinct-key `HashSet`, authoritative fs walk, fail-closed `read_dir`, pid-level `file_type()` guard, decode-before-type-check, folded per-pid cleanup replacing the deleted `cleaned_pids` block.
  - [x] `cargo fmt --all`; re-measure **both** budgets. Expect **+46 physical / +30 tokei** — verify, do not assume.
- [x] **Task 2 — kill null control #24 (gates the pin).** (AC: 4)
  - [x] Rewrite the `"private"` discharge in `multi_backend_erasure_test.rs`: Markdown + >4 KiB spill, restart path, content-absence assertion, bystander survival.
  - [x] Add the restart-backed control in `crates/maos-kernel-core/tests/`.
  - [x] **Falsify M1–M8**, each at its own assertion; restore; record every transcript in the Debug Log. Serialized builds only (Trap 8).
- [x] **Task 3 — move the pin, once Tasks 1-2 are green.** (AC: 1)
  - [x] Re-pin `src_lines` to the **measured** value; touch nothing else. Verify `fkcs-baseline.toml` byte-untouched by `git diff`.
  - [x] Write the nine-element HISTORY row.
  - [x] Confirm `check-kernel-baseline`, `kloc-check`, and the a2a-tcp guard `t12b_kernel_core_byte_identical_line_count` all pass at the new number.
- [x] **Task 4 — retire every "residue is open" claim.** (AC: 5)
  - [x] Invert and rename the pinned test; hoist and fix `spilled_files`; replace the *"whatever its value"* assertion; fix the dangling reference.
  - [x] Rename and re-point the Blocking leg; add the anti-vacuity sibling.
  - [x] Zero-hit `grep -rn '13-5i' crates/ xtask/ docs/`.
- [x] **Task 5 — gate and CI.** (AC: 6)
  - [x] Register the new control as a `Blocking` `--exact` leg on `check-reza-production-path`; **confirm the leg actually executes it** (Trap 10).
  - [x] Verify `claims_removal`'s newly-armed rejection path holds (AC3).
- [x] **Task 6 — ADR-059 amendment (same commit).** (AC: 1, 5)
  - [x] Decision 10 → fixed; Residual 8 → CLOSED with the measured number; Consequences `:139` updated. Record the `deleted_entries` semantics decision and the Shape B/C/D rejections. Open **`13-5j`** for the stale-spill defect. Do **not** open a new ADR.

### Review Findings

*Code review 2026-07-27 against `dd4a908e`. Four parallel layers (Blind Hunter, Edge Case Hunter, Acceptance Auditor, Test Infrastructure Auditor). 3 decision-needed, 8 patch, 3 deferred, 3 dismissed.*

**Verified satisfied:** AC1 (pin `23280 == 23280` measured on the formatted tree; tokei `17924`; `fkcs-baseline.toml` byte-untouched; nine-element HISTORY row present; only `private.rs` grew under `crates/maos-kernel-core/src/`), AC5 (case-sensitive `grep -rn '13-5i' crates/ xtask/ docs/` returns only ADR-059's CLOSED row; pinned test inverted+renamed; `spilled_files` hoisted and `file_type`-based; the *"whatever its value"* assertion replaced; ADR-059 amended at all three sites), AC6 (control lives in `crates/maos-kernel-core/tests/`, seven `#[test]`s each bound to its own `--exact` `BindingClass::Blocking` leg). M1–M5, M7 are genuinely detected; see finding 4 for M8 and finding 10 for M6.

- [x] [Review][Decision] **A namespace-directory symlink produces a signed erasure receipt over bytes that survive** — `forget_principal` type-checks only the *pid* entry (`private.rs:376`). After a namespace name decodes to the target principal it calls `fs::read_dir(&ns_dir)` (`private.rs:399`) with no `file_type()` check, so a symlink at `<fs_root>/<pid>/<hex-ns>` is **followed**: every external file stem is inserted into `erased` and counted, then `fs::remove_dir_all(&ns_dir)` (`private.rs:409`) merely unlinks the link. Measured on rustc 1.96.0: `read_dir(symlink)` → `Ok(1)`, `remove_dir_all(symlink)` → `Ok(())`, target contents survive. Net result is `Ok(n)`, `n > 0`, `Removed { count: n }` in an Ed25519-signed Art.17 proof over PII still on disk — this story's own defect class, one level down. AC2 mandated containment only at the pid level, so this is conformant to the letter of the AC and a live hole in its intent. `forget_does_not_follow_pid_directory_symlink` (`private_forget_restart_13_5i.rs:253-277`) cannot see it. Fix is unambiguous (require `ns_entry.file_type()?.is_dir()` after the decode match and return `Err` otherwise — fail-closed, consistent with Trap 4, and it keeps `erasure_uninstall_13_5b.rs:659` / `gdpr-cascade-040` green) but costs ~3 kernel lines, which breaks the exact-equality pin at `23280` and needs a fresh FLAG-Winston delta. **Decide: re-authorize a bounded `+3..5` repair, or record as an ADR-059 residual with a successor story.**
- [x] [Review][Decision] **An empty key is counted twice in the signed receipt** — `sanitize_key` (`private.rs:42-64`) rejects `/`, `\`, NUL, ASCII control and `..` components but **not** the empty string, and no caller validates it (`memory/mod.rs:693-737` passes `key` through untouched). A `>4 KiB` value written at key `""` spills to `<ns>/.bin`; `Path::new(".bin").file_stem()` returns `Some(".bin")`, while the cache side inserts the real key `""` (`private.rs:352`, `:406`). One logical entry, two `HashSet` identities, `deleted_entries == 2`. AC3: *"Over-reporting in a signed Art.17 proof is a fresh false claim, not a conservative one."* Two fixes, both kernel lines and both a behavior change: reject the empty key in `sanitize_key`, or derive the identity by stripping a known `file_ext_for_kind` extension rather than `file_stem()`. **Decide which, or defer with the pin.**
- [x] [Review][Decision] **Namespace children are attested as logical keys with no type or extension check** — `private.rs:403-407` inserts `file_stem()` for *every* `DirEntry` under a matched namespace directory. A hand-created sub-directory `archive/` counts as one key while its `remove_dir_all`-deleted contents count as none; a non-UTF-8 filename fails `to_str()` and is deleted **without being counted** (silent under-report); a FIFO or unrelated file over-counts. The private tier's Markdown area is *deliberately* operator-hand-editable (`private.rs:183-188`, story `:99`, ADR-026), so these are reachable, not theoretical. Same fix site and same pin cost as the finding above. **Decide: count only regular files with a recognized `file_ext_for_kind` extension and fail closed on anything else, or accept and document the divergence between the signed count and disk reality.**
- [x] [Review][Patch] M8 as specified (per-file counting) has no CI-executed detector; the Debug Log records a different mutant [crates/maos-kernel-core/tests/private_forget_restart_13_5i.rs:195-212] — `forget_counts_cached_and_spilled_value_exactly_once` writes ONE large blob: one cache entry, one spill file. A per-file implementation also returns `1`, so the leg stays green. `restart_forget_reports_distinct_persisted_entry_count` (`:165-192`) has two files and expects `2` — also green under per-file. AC4(d) row M8 requires *"count per file instead of per distinct key"*; the Debug Log at story `:481` records *"map cardinality added to the distinct set"* instead. The only assertion in the repo that reds under per-file counting is `principal_namespace_lifecycle.rs:87` (`deleted_entries == 5` from five inline Texts that spill zero files) — and **no CI leg names that test** (`grep principal_namespace_lifecycle xtask .github` → no matches; story Residual 3). Fix: add a mixed inline-only + spilled control to `private_forget_restart_13_5i.rs` and bind it as an `--exact` Blocking leg. Test-only, zero pin impact.
- [x] [Review][Patch] AC3's newly-armed `claims_removal` rejection path is asserted, not proven [crates/maos-audit/src/erasure/proof.rs:383-395] — AC3 states ⚠ *"prove it, do not assume it."* No test calls `verify_erasure_proof` on a bundle carrying a non-zero private count: the subprocess test only reads the serialized category (`erasure_uninstall_13_5b.rs:853-866`) and the backend test only inspects the in-memory `ForgetReceipt` (`multi_backend_erasure_test.rs:187-209`). The production path writes the bundle without verifying it (`main.rs:8348-8364`). Fix: verify the emitted bundle in the subprocess test. Test-only.
- [x] [Review][Patch] Content-absence controls fail open on I/O error, undercutting AC4(b) [crates/maos-audit/tests/multi_backend_erasure_test.rs:46-65; crates/maos-bin/tests/erasure_uninstall_13_5b.rs:241-246] — `private_store_contains` returns `false` when `read_dir` fails and drops per-entry errors with `flatten()`; `private_tree_contains` maps a failed `fs::read` to `false` via `unwrap_or(false)`. `!contains(...)` then passes over *unreadable surviving residue*. An observation failure is being treated as proof of absence, in a control whose entire purpose is content absence — and in direct tension with the fail-closed doctrine AC2 imposes on production. Fix: `.expect()` both, matching the new `spilled_files` walker (`erasure_uninstall_13_5b.rs:222-239`). Test-only.
- [x] [Review][Patch] Torn-state-on-failure residual is unrecorded in ADR-059 [crates/maos-kernel-core/src/memory/private.rs:343-355] — the in-memory removal still completes *before* the first fallible filesystem operation. Shape A4 closes D-4 consequence 2 (self-sealing: the map-independent walk can now reach the leftover subtree on retry) but leaves consequence 1 intact — a ≤4 KiB value that existed only in RAM is destroyed even when the forget returns `Err` — and **adds** a new one: a retry after the operator fixes permissions no longer sees those already-purged cache entries, so its signed `deleted_entries` under-reports what the failed attempt actually destroyed. Neither is a spec violation (AC2 did not require fixing them) but ADR-059 Decision 10 now reads as if the private tier's erasure story is complete. Fix: add a residual row. Doc-only.
- [x] [Review][Patch] `forget_fails_closed_when_pid_directory_is_unreadable` fails deterministically when tests run as root [crates/maos-kernel-core/tests/private_forget_restart_13_5i.rs:291-301] — root ignores mode bits, so `read_dir` on the `0o000` pid directory succeeds, `forget_principal` returns `Ok(1)`, and `matches!(result, Err(MemoryError::Io(_)))` fails. This is a *Blocking* leg, so a rootful container blocks the gate on a false red. GitHub-hosted `ubuntu-latest` runners are non-root, so CI is currently safe; local Docker/devcontainer development is not. Fix: probe readability after the `chmod` and return early (or `eprintln!` + skip) when the process can still enumerate. Test-only.
- [x] [Review][Patch] Two Blocking legs are structurally vacuous on any non-Unix target [crates/maos-kernel-core/tests/private_forget_restart_13_5i.rs:253-277,280-303] — `#[test]` is unconditional but the entire body sits inside an inner `#[cfg(unix)]` block, so on a non-Unix target `gdpr-private-symlink-containment` and `gdpr-private-fail-closed-io` compile to empty functions that report `running 1 test` / `1 passed`. `run_test_leg`'s anti-vacuity substring check (`check_reza_production_path.rs:87`) is blind to this — the same blindness that let null control #24 live. No non-Unix target exists in CI today (`release.yml` builds linux + darwin only), so the consequence is latent. Fix: hoist the gate to `#[cfg_attr(not(unix), ignore = "unix-only containment control")]` on the function so a non-Unix run reports `0 passed; 1 ignored` instead of a false green. Test-only.
- [x] [Review][Patch] The exact-once control never establishes its spilled precondition [crates/maos-kernel-core/tests/private_forget_restart_13_5i.rs:195-212] — unlike its siblings at `:133` and `:155`, it never asserts the blob reached disk before forgetting. If `should_spill_to_disk` (`private.rs:154-162`) ever regresses, the blob stays cached, map-only deletion returns `1`, the post-forget canary scan is vacuously true, and a test named `cached_and_spilled` passes while exercising only the cache. Fix: add `assert!(tree_contains(&fixture.fs_root, SPILL_CANARY));` before the forget. Test-only.
- [x] [Review][Patch] A late `read_dir` iterator error is swallowed in the pid-directory reap [crates/maos-kernel-core/src/memory/private.rs:414-420] — only the `read_dir` *constructor* error is mapped; `.next()` yielding `Some(Err(_))` makes the emptiness condition simply `false`. AC2's literal rule is *"every `read_dir` error becomes a `MemoryError`."* Consequence is bounded — the worst case is an already-emptied pid directory left on disk, holding no PII — so this is a conformance nit, not a residue path. Fix: match on `.next()` and propagate `Some(Err(e))`. Costs kernel lines; fold into whichever decision above is authorized, or leave.
- [x] [Review][Defer] TOCTOU: the checked pid directory can be swapped for a symlink before `read_dir` [crates/maos-kernel-core/src/memory/private.rs:374-382] — deferred, requires a concurrent local writer inside the operator-owned memory root during an uninstall; a real fix needs descriptor-anchored `openat(O_NOFOLLOW)` traversal, a new dependency and a materially larger kernel delta than any bounded repair here.
- [x] [Review][Defer] A write concurrent with `forget_principal` can survive it [crates/maos-kernel-core/src/memory/private.rs:326-355,368-410] — deferred, pre-existing. There is no operation-wide exclusion: a target write landing after the `to_remove` snapshot stays in `in_mem`, and a spill after `remove_dir_all` recreates the namespace. Unchanged in shape by 13.5i, and production forget is a one-shot CLI process with no Spirit running (D-1).
- [x] [Review][Defer] Directory entries are unlinked while their parent's `ReadDir` is still being iterated [crates/maos-kernel-core/src/memory/private.rs:399-421] — deferred. `remove_dir_all(&ns_dir)` runs inside `for ns_entry in fs::read_dir(&pid_dir)`, and `remove_dir(&pid_dir)` inside `for pid_entry in read_dir(&fs_root)`. POSIX leaves subsequent `readdir` results unspecified once the directory is modified after `opendir`; on ext4/tmpfs glibc's buffering makes this safe in practice, but on other backing stores an entry can be skipped — silent under-deletion under a success receipt. [INFERENCE] — not reproduced here. Fix if ever revisited: collect the entries into a `Vec` before deleting.

**Dismissed (3):** non-pid-named directories under `fs_root` are walked (only the kernel writes there, and it emits `u32::to_string()` exclusively); uppercase-hex / alternate-JSON namespace-directory aliases double-count (a hand-crafted directory with a non-canonical encoding holds no entry `read()` can return); HISTORY row element 3 names `memory/private.rs` without line numbers.

---

## Dev notes

- **The 46 lines are the easy part.** The load-bearing deliverable is AC4 — converting an unfalsifiable claim into a control. 13.5h shipped 6 kernel lines and its value was making an existing control tell the truth; this is the same story with a bigger diff.
- **The map is a cache; the disk is the record.** Every design that reasons from `in_mem` inherits the bug, because in production `in_mem` is empty by construction.
- **Reuse, do not invent.** `hex` is already a dependency (`Cargo.toml:27`, already used at `memory/mod.rs:573`). `CategoryStatus::{Removed, VerifiedEmpty, CoverageGap}` (`proof.rs:20-26`) is the vocabulary. The subprocess fixture idiom (`erasure_uninstall_13_5b.rs:60-82`, `Command::new(env!("CARGO_BIN_EXE_maos"))`) is the house pattern — **do not add `assert_cmd`**; it would be the workspace's first and `cargo-deny` is live.
- **`subject_access_query` is correct and must not be touched.** `crates/maos-audit/src/lib.rs:1344-1378` reads only `principal_index`, which *is* durably erased (`mod.rs:579`). It is a faithful reporter. The dishonest artifact is the **proof bundle**, not the Article 15 answer — the brief's "asymmetry that hides its own residue" framing overstates its role.
- **`principal_index` has its own out-of-scope residue class**: it opens with `PRAGMA journal_mode=WAL` (`principal.rs:42-43`), so bytes live in `-wal`/`-shm` sidecars and deleted rows persist as free pages until VACUUM. Declared out of scope at `gdpr_cascade_corpus_test.rs:186-193`. After this story the private tier's on-disk story is **strictly stronger** than the index's — say so, rather than letting a reader infer parity.
- **No corpus regeneration is required for the fix itself.** `MANIFEST.toml` pins SHA-256 over the committed JSONL, which carries only scenario *inputs* — no `deleted_entries` field. If you add a Markdown/spill stratum (recommended, and the only way the corpus covers this defect), follow the 13.5b precedent exactly: edit `gen_gdpr_cascade.rs` `stratum_counts`, regenerate, then update **three** MANIFEST fields in the same commit — `sha256`, `description`, and `item_count`.
- **`ErasureProof::schema_version` does not bump.** It is the literal `"maos.erasure-proof.v1"` (`proof.rs:236`); no field is added or removed.
- **The collective and shared tiers have no analogous residue.** `shared_memory` is a plain SQLite table (`shared.rs:18-28`); `maos-loom-lite/src/store.rs` contains zero `fs::`/`PathBuf` references. Verified, not assumed.

## Test scaffolding you will reuse — do not rebuild any of this

`crates/maos-bin/tests/erasure_uninstall_13_5b.rs` (882 lines, 13 tests) already contains every fixture this story needs:

| Item | Line | What it gives you |
|---|---|---|
| `const PRINCIPAL` | `:17` | `"held-uninstall@example.org"` |
| `Fixture::new()` | `:26-47` | TempDir root; creates `data/`, `memory/`, writes a hex 32-byte audit key at `0o600` |
| `Fixture::command()` | `:60-72` | the subprocess builder — `Command::new(env!("CARGO_BIN_EXE_maos"))` + 8 env vars |
| `Fixture::run_uninstall(Option<&str>)` | `:74-82` | sets/clears `MAOS_REGION_HOME`, returns `Output` |
| `Fixture::seed_named_principal(principal, held)` | `:84-124` | **the in-process store builder** — copy this shape for a >4 KiB seed |
| `Fixture::seed_markdown_principal()` | `:130-161` | already writes `MemoryValue::Markdown` at key `"dossier"` |
| `Fixture::plant_pre_partition_shared_row()` | `:179-204` | raw `rusqlite` INSERT precedent (13.5h) |
| `terminal(&Output)` | `:218-226` | parses stdout JSON, panics with stderr on failure |
| `private_forget_reports_filesystem_removal_failure` | `:611-660` | the **in-process** precedent — holds an `Arc::clone(&private)` and calls `forget_principal` directly |

⚠ `PrivateMemoryStore::write` is `pub(in crate::memory)` (`private.rs:168`), so an out-of-crate test **cannot** call it — seed through `MemoryManagerAdapter::write`. `forget_principal` **is** `pub` (`:319`), so it can be called directly.
⚠ There is **no `assert_cmd`, `escargot` or `predicates` anywhere in the workspace** — 43 test files use the `CARGO_BIN_EXE_` idiom. Do not add a new dev-dependency; `cargo-deny` is live.
⚠ The `spilled_files` walker is a **closure local to the pinned test** (`:804-822`). Hoist it to module scope (AC5) rather than writing a second one.

**Namespace serialization, for the decode you are writing** — `MemoryNamespace` is externally tagged with no serde attributes; field order is declaration order (`maos-domain/src/memory.rs:40-56`):

```
MemoryNamespace::principal("alice@example.org", "calendar")
  → {"Principal":{"principal_id":"alice@example.org","schema":"calendar"}}
```

Unit variants serialize as bare strings (`"Default"`, `"Coordination"`, `"Forgotten"`), so they simply fail the `Principal` match and are skipped.

## Budget — measured 2026-07-26 at `dd4a908e`

| Crate | Measured | Ceiling | Headroom | This story |
|---|---:|---:|---:|---|
| **`maos-kernel-core`** | **17 894** | **18 248** | 354 | **+30 → 17 924 (324 spare)** |
| `maos-audit` | 6 147 | 6 240 | **93** | AC4 test rewrite (tests excluded from kloc) |
| `maos-bin` | 14 175 | 14 433 | 258 | AC5 test inversion (excluded) |
| `xtask` | 30 925 | 31 530 | 605 | AC6 leg |
| `_aggregate_hardfail` | 134 062 | 136 669 | 2 607 | → 134 092 |

Physical baseline pin: `xtask/kernel-core-baseline.toml` = **23 234** → authorized **23 280**. Separate number from the KLOC table (physical vs tokei).

⚠ **`maos-audit` has only 93 lines of reserve** and has drifted +30 past its 2026-07-25 re-base measurement. If any AC lands production code there, it is the crate that breaches first. Tests do not count.

## Previous-story intelligence

- **13.5h (immediate predecessor, commit `dd4a908e`)** — killed null control #23 for `"shared"` by the exact method AC4 requires: refusal proven at the tier's own entry point, a namespace-filtered scan, the positive-retention canary split into its own assertion, and per-leg falsification before the pin moved. Its `VerifiedEmpty`-is-**earned**-never-asserted decision (`shared_tier_principal_row_count`) is the precedent for AC3's count. **This story is its twin on the private tier.**
- **13.5b** — established that a control *naming* a store is not a control *covering* it, and that an artifact must describe what actually happened (ADR-059 Decisions 8 and 9). It found this defect while writing its own proven-red test, and recorded it honestly rather than fixing it out of scope.
- **13.5d** — the "code first, then the pin" fence (`:475`) and "do not spend the grant elsewhere" (`:473`). Both bind here.
- **13.3b** — a Blocking leg passed because an appended invalid marker, not the property under test, satisfied its `assert_ne`. Found only when the author neutered the guard. AC4(d) is that discipline.

## Testing standards

- **Proven-red is the bar,** and per AC4 it gates the pin. Falsify **per limb**, not once.
- **Serialized builds only** for proven-red evidence (Trap 8). Quote the `Compiling` lines.
- **Test placement matters to the pin.** `crates/maos-kernel-core/tests/` costs zero on both meters; in-`src` `#[cfg(test)]` costs both. Prefer `tests/` — and then **name it in a CI leg**, or it never runs.
- **Restart-backed** means either a fresh `PrivateMemoryStore` over a pre-seeded `fs_root` (faithful — `new` is a pure constructor) or a subprocess. Use **both altitudes**: the kernel-core unit companion localizes a regression to `private.rs`; the `maos-bin` subprocess test is the only one that sees the composition-root wiring, the terminal exit code and the signed bundle.
- **Hermetic → `Blocking`.** Nothing here needs Postgres.
- **Anti-vacuity:** one `#[test]` per leg, `--exact`, transcript shows `running 1 test` and `1 passed`. Remember this check cannot see a null assertion — the falsification matrix is the real control.

## Gate discipline (§A7 reflex)

Legs land on **`check-reza-production-path`**. It is a control for this story only because (a) deleting any limb of the walk reds the rewritten AC4 leg at its own assertion, (b) the inverted pin leg reds if the residue returns, and (c) the bystander leg reds on over-deletion. Forbidden shortcuts, all previously used in this epic: leaving a string in `ABSENT_SUCCESSORS` in place of a leg; an `available_arm_tests`-only leg; citing `abi-diff` or `check-empty-kernel` as sensors (both blind here); **and citing `check-fkcs`, which exits 0 with a red oracle.**

## Residuals

1. **⚠ Stale-spill / duplicate-scan defect — OPEN, successor `13-5j`, filed 2026-07-26.** `PrivateMemoryStore::write` never unlinks a superseded on-disk file. Write `Markdown` to key `k` (spills to `k.md`, not cached), overwrite with a small `Text` (cached, no spill): `k.md` survives indefinitely. `read` returns the map value so it is invisible, but **`scan` (`:250-315`) merges map and filesystem without dedup and returns two entries for one key** — exposing the superseded value to the Spirit. A live data-integrity and data-exposure bug **independent of erasure**, on a different surface (`write`/`scan`, not `forget_principal`). Operator-ratified 2026-07-26 as a separate story per 13.5d's "do not spend the grant elsewhere" fence. Record in ADR-059 with a named owner.
2. **⚠ `check-fkcs` is a null control — OPEN, ownerless, Epic-13 retro.** `check_fkcs.rs` computes `dev_blocks` (`:260-264`), documents it as *"the Blocking binding class hard-fails a RED oracle at HEAD regardless of `CURRENT_PHASE`"*, serializes it into JSON — and the return path at `:325` never consults it, keying on `blocking_now` (phase) instead. **Re-measured on a clean tree 2026-07-26: `check-fkcs: PASS (advisory — oracle RED …) `, `admission-path-unmodified=red`, process exit `0`.** One leg red, not five — the preflight's first measurement was taken against a working tree dirtied by a concurrent scout and over-reported. **The null control is confirmed either way**: a red oracle leg exits 0 and GitHub Actions records success. Mutation that should red it but would not: any change to `crates/maos-registry/src/admission.rs`, including deleting the `OffFrozenSurface` rejection arm that ADR-052 makes the FKCS negative control depend on. This is the E12-B1 `gate_binding_decay` finding **still live at HEAD, in the one gate that imported the fix**. Four further gates never imported `BindingClass` at all: `check_cohort_mesh`, `check_cross_region_consensus`, `check_escape_detector`, `check_multi_region_slo`.
3. **⚠ Kernel-core in-`src` tests are budget-charged and CI-unexecuted — OPEN, ownerless, Epic-13 retro.** No `--workspace`, no `--all`, and the repo's single `--lib` run is another crate. All **49** `#[cfg(test)]` modules in `maos-kernel-core` never execute in CI, including the 12.5 proven-red tests that the `23141` HISTORY row cites to justify its +59 lines. Either add a `cargo test -p maos-kernel-core --lib` job or stop charging the pin for tests that never run. Deliberately **not** absorbed here: those modules have never run, so an unknown number may be red at HEAD.
4. **No gate reconciles the pin literal with its HISTORY rows — OPEN, ownerless.** 13.5d's re-pin landed with no row and was repaired retroactively a story later. A ~10-line xtask leg (last HISTORY row value == `src_lines`) would have caught it.
5. **Ceiling comments drift silently from measured — Epic-13 retro.** `maos-audit` +30 past its 2026-07-25 measurement (93 left), `maos-bin` +25, `xtask` +14, `maos-kernel-core` +4. The policy comments quote stale reserves.
6. **`principal_index` WAL/free-page residue — declared out of scope, not closed.** `gdpr_cascade_corpus_test.rs:186-193`. Unchanged by this story; note the asymmetry rather than implying parity.
7. **Epic-13 story count is stale.** The epic file says *"Eleven stories total"*; there are now 15 (13.5f–13.5i added post-hoc). Epic-13 retro bookkeeping.

## References

**Requirements this story serves**
- **FR65** — *"Operator can uninstall a Spirit; kernel emits a proof-of-erasure record enumerating all removed substrate state (memory namespace per ADR-026, …)"*, annotated **"Defends the v1.0 hermes-tenant positioning claim that substrate-uninstall is a real guarantee, not a hope."** [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md:107`] — the sentence a `Removed { count: 0 }` over surviving bytes falsifies.
- **FR45** — GDPR Article 17 via `maosctl forget --principal <id>`; *"kernel removes all principal-namespace entries."* Floor: *"0 leakage in 100 follow-up subject-access queries."* [Source: same, `:101`] ⚠ That floor is currently met **vacuously**: `subject_access_query` reads `principal_index`, which *is* erased, while the bytes remain on disk.
- **ADR-026** — *"Writes to this namespace … **inherit three kernel-mediated operations**: subject-access query, right-to-be-forgotten, redaction-on-export."* [Source: `docs/adr/ADR-026-principal-memory-namespace.md:12`] — filesystem-only residue means right-to-be-forgotten does **not** in fact cover all writes to the namespace. **This is an ADR-026 conformance violation, not only an FR gap.**
- **ADR-059** — Decision 10 (`:88-94`, *"The private tier's filesystem residue is an open hole … recorded, not fixed"*), Residual 8 (`:125`, owner `Story 13-5i`), Consequences (`:139`). All three amended by AC1(6).
- **Port contract** — `MemoryManagerPort::forget` doc, `crates/maos-domain/src/ports/memory.rs:86-89`: deletes from *"the in-memory map, **filesystem area**, and index table."* The kernel is out of conformance with its own published contract.

**Governance**
- Pin: `xtask/kernel-core-baseline.toml` (`src_lines`, exact equality, `check_kernel_baseline.rs:94-107`). FROZEN tag: `xtask/fkcs-baseline.toml:5` = 23081 — do not touch.
- Ceiling policy: `xtask/kloc.toml` — `ceiling = measured + max(100, ceil(0.02 × measured))`, founder-ratified 2026-07-25; *"must never block a correctness or compliance repair."*
- Gate host: `xtask/src/check_reza_production_path.rs`; `BindingClass` at `xtask/src/gate_common.rs:78-103`.

## Dev Agent Record

### Agent Model Used

### Debug Log References

- Model: `openai-codex/gpt-5.6-sol`.
- Baseline: `HEAD=dd4a908e94f48494d39e074273d65dbae8168747`; no pre-existing code modifications. `git status --short` contained only the untracked story artifact plus the workflow-owned sprint-status change. `check-kernel-baseline` reported `23234 == 23234`; `tokei` reported kernel-core `17894`; `kloc-check` reported aggregate `134062`; frozen FKCS tag remained `23081`.
- RED reproduction: serialized `cargo test -p maos-bin --test erasure_uninstall_13_5b private_tier_markdown_survives_the_forget_cascade -- --exact` compiled `maos-kernel-core` and `maos-bin`, then failed `0 passed; 1 failed` because the repaired walk removed the pinned Markdown residue.
- Shape A4 landed in `PrivateMemoryStore::forget_principal`: authoritative namespace decode walk, fail-closed directory I/O, pid-level `DirEntry::file_type` symlink containment, and exact-once `(pid/namespace, key)` counting.
- Formatted measurement before pin movement: physical `23234 → 23280` (+46); tokei `17894 → 17924` (+30). `xtask/fkcs-baseline.toml` remained byte-untouched.
- Serialized mutation evidence:
  - M1 walk deleted: restart residue controls RED (`0 passed; 3 failed`); 13 pre-existing private-store tests GREEN.
  - M2 Markdown-only walk: spill limb RED; Markdown limb GREEN.
  - M3 non-Markdown-only walk: Markdown limb RED; spill limb GREEN.
  - M4 map-only count: proof-count limb RED; file-absence limb GREEN.
  - M5 `Path::is_dir` symlink-following: containment limb RED; ordinary erasure GREEN.
  - M6 swallowed `read_dir` error: unreadable-directory limb RED; happy path GREEN.
  - M7 all-decodable-namespace deletion: bystander-retention limb RED; target erasure GREEN.
  - M8 map cardinality added to the distinct set: exact-once limb RED; absence limb GREEN.
  - Every mutant run showed `Compiling maos-kernel-core`; the source was restored after each serialized run. Restored suite: 7/7 GREEN.
- Focused GREEN: restart controls 7/7; backend partition 1/1; uninstall integration 13/13.
- Full regression: `cargo test --workspace --no-fail-fast` — 3511 passed, 0 failed, 92 ignored across 450 suites.
- Quality/gates: `cargo fmt --all -- --check` PASS; `check-kernel-baseline` PASS (`23280 == 23280`); `kloc-check` PASS (aggregate 134190); a2a-tcp line-count guard 1/1; `check-reza-production-path --json` passed every Blocking leg with `running 1 test; 1 passed`.
- Environment note: three pre-existing live Postgres Reza legs were not attempted because the two-datname substrate was absent; they remained correctly reported as `AdvisorySubstrate`. All hermetic story legs were attempted and GREEN.
- Reference retirement: case-sensitive `13-5i` search under `crates/`, `xtask/`, and `docs/` returned only ADR-059's CLOSED historical row.

### Implementation Plan

### Completion Notes

- Production one-shot uninstall now discovers and removes private-tier Markdown and post-restart spills from the filesystem instead of relying on an empty cache.
- `deleted_entries` now records distinct logical entries exactly once across cached and spilled representations; the signed proof carries the real non-zero effect.
- Null control #24 was replaced with restart-backed Markdown, large-spill, content-absence, exact-count, symlink, fail-closed-I/O, and bystander-retention controls. M1–M8 were independently falsified before the pin moved.
- Kernel grant spent exactly as authorized: +46 physical / +30 tokei; source pin `23234 → 23280`; KLOC ceiling and frozen FKCS tag unchanged.
- ADR-059 Decision 10 and Residual 8 are CLOSED. Separate stale-spill/duplicate-scan defect remains owned by successor `13-5j`.

### File List

- `_bmad-output/implementation-artifacts/13-5i-private-tier-filesystem-residue.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `crates/maos-audit/tests/multi_backend_erasure_test.rs`
- `crates/maos-bin/tests/erasure_uninstall_13_5b.rs`
- `crates/maos-kernel-core/src/memory/private.rs`
- `crates/maos-kernel-core/tests/private_forget_restart_13_5i.rs` (new)
- `docs/adr/ADR-059-operator-authority-collective-erasure.md`
- `xtask/kernel-core-baseline.toml`
- `xtask/src/check_reza_production_path.rs`

### Change Log

| Date | Change |
|---|---|
| 2026-07-26 | Story created. 4-scout + validator adversarial preflight against `dd4a908e`. **Reframed:** the defect is not Markdown residue but that the production uninstall path erases the private tier **not at all, for any value kind** — `in_mem` is structurally empty at forget time (D-1), confirmed end-to-end with the real binary. **Null control #24 found** — the `"private"` erasure discharge plants a sub-threshold `Text` that never reaches disk, so it is green in both worlds (D-3); structurally identical to the #23 that 13.5h deleted for `"shared"`. **Cost measured, not estimated:** six shapes built and reverted; the cheap +12 variant fails open and escapes `fs_root`; Shape A4 = **+46 physical / +30 tokei**. Shape B rejected on failure mode (a serde field reorder silently makes PII survive), C refuted four ways, D rejected (the index is a claim about the disk; the walk is self-verifying). Three operator ratifications: grant approved at the measured +46; stale-spill defect split to successor `13-5j`; control wired as a Blocking Reza leg with the systemic `--lib` gap deferred to the retro. |
| 2026-07-26 | Implemented Shape A4 private-tier filesystem erasure; replaced null control #24 with restart-backed Blocking controls; independently falsified M1–M8; re-pinned kernel-core `23234 → 23280`; closed ADR-059 Decision 10 / Residual 8; full workspace regression and discipline gates passed. |
| 2026-07-27 | **Code review** (4 parallel layers). 3 decision + 8 patch findings resolved, 3 deferred, 3 dismissed. Landed an operator-authorized hardening on the same surface: namespace-level symlink containment (a symlink at `<fs_root>/<pid>/<hex-ns>` was followed by `read_dir`, its external files counted, and then only unlinked by `remove_dir_all` — a signed `Removed { count: n }` over surviving bytes, this story's own defect one level down); logical-key identity via `file_ext_for_kind` stripping instead of `file_stem()` (the empty key double-counted, and hand-created sub-directories / editor backups were attested as entries); and a fail-closed pid reap. New controls: `forget_counts_inline_only_entries_that_never_spill` (M8 **as specified** — per-file counting had no CI-executed detector before), `forget_does_not_follow_namespace_directory_symlink` (M9), `forget_counts_logical_keys_not_filesystem_nodes`, all three bound as `--exact` Blocking Reza legs. Test-only repairs: `verify_erasure_proof` now run on the emitted bundle (AC3's newly-armed `claims_removal` path, proven not assumed); both content-absence scanners fail loud instead of reading an I/O error as absence; the fail-closed leg skips instead of false-redding under root; the two unix-only legs are `#[cfg_attr(not(unix), ignore)]` instead of silently empty; the exact-once control asserts its spill precondition. Re-pinned `23280 → 23318` (+38 physical / +17 tokei; total story spend +84 / +47 against `dd4a908e`, ceiling 18248 unchanged with 307 spare). ADR-059 Decision 10 amended and Residual 9 opened for the non-atomic forget. Evidence: 4 serialized mutants each RED at their own limb and GREEN elsewhere; kernel-core control file 10/10, `erasure_uninstall_13_5b` 13/13, `cargo test --workspace --no-fail-fast` zero failures, `cargo fmt --all -- --check` PASS, `check-kernel-baseline` `23318 == 23318`, `kloc-check` PASS (aggregate 134249), `check-reza-production-path` 43 legs, 0 blocking failures, all 11 private legs `running 1 test; 1 passed`. |
